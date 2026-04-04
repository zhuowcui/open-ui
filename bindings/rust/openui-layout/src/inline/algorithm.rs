//! Inline layout algorithm — entry point for inline formatting context.
//!
//! Mirrors Blink's `InlineLayoutAlgorithm` from
//! `third_party/blink/renderer/core/layout/inline/inline_layout_algorithm.cc`.
//!
//! Takes a block node that has inline children and produces a Fragment
//! containing positioned line box fragments with text fragments inside.
//!
//! The algorithm follows CSS 2.2 §10.6.1 (inline formatting context),
//! §10.8 (line height calculations), and §16.2 (text alignment).

use openui_dom::{Document, NodeId};
use openui_geometry::{LayoutUnit, PhysicalOffset, PhysicalSize};
use openui_style::{ComputedStyle, Direction, LineHeight, TextAlign, VerticalAlign};
use openui_text::Font;

use crate::block::{resolve_border, resolve_padding};
use crate::constraint_space::ConstraintSpace;
use crate::fragment::{Fragment, FragmentKind};
use crate::length_resolver::resolve_margin_or_padding;

use super::items::{InlineItemResult, InlineItemType};
use super::items_builder::{style_to_font_description, InlineItemsData, InlineItemsBuilder};
use super::line_breaker::LineBreaker;
use super::line_info::LineInfo;

// ── Line height metrics (CSS 2.2 §10.8.1 half-leading model) ────────────

/// Vertical extent above/below baseline for a single inline element,
/// after applying line-height (half-leading distribution).
#[derive(Debug, Clone, Copy)]
struct LineHeightMetrics {
    /// Distance above baseline (positive upward).
    ascent: f32,
    /// Distance below baseline (positive downward).
    descent: f32,
}

/// Compute line height metrics using the CSS 2.2 §10.8.1 half-leading model.
///
/// The computed line-height determines total height, and extra space (leading)
/// is distributed equally above and below the font's ascent/descent.
///
/// Blink puts floor on ascent side, ceil on descent side, so the total
/// exactly equals the computed line-height.
fn compute_line_height_metrics(
    font_ascent: f32,
    font_descent: f32,
    line_height: &LineHeight,
    font_size: f32,
    line_spacing: f32,
) -> LineHeightMetrics {
    let computed_line_height = match line_height {
        LineHeight::Normal => line_spacing,
        LineHeight::Number(n) => font_size * n,
        LineHeight::Length(px) => *px,
        LineHeight::Percentage(pct) => font_size * pct / 100.0,
    };

    let leading = computed_line_height - (font_ascent + font_descent);
    let half_leading_floor = (leading / 2.0).floor();
    let half_leading_ceil = leading - half_leading_floor;

    LineHeightMetrics {
        ascent: font_ascent + half_leading_floor,
        descent: font_descent + half_leading_ceil,
    }
}

// ── Vertical alignment (CSS 2.2 §10.8) ──────────────────────────────────

/// Compute baseline shift for vertical-align.
///
/// Returns a float offset where positive = downward shift from parent baseline.
/// Blink: `InlineBoxState::ComputeTextMetrics` and related code in
/// `inline_box_state.cc`.
fn compute_baseline_shift(
    vertical_align: &VerticalAlign,
    font_size: f32,
    parent_ascent: f32,
    parent_descent: f32,
    parent_x_height: f32,
    item_ascent: f32,
    item_descent: f32,
    parent_line_spacing: f32,
) -> f32 {
    match vertical_align {
        VerticalAlign::Baseline => 0.0,
        VerticalAlign::Sub => font_size / 5.0 + 1.0,
        VerticalAlign::Super => -(font_size / 3.0 + 1.0),
        VerticalAlign::Middle => {
            (item_ascent - item_descent) / 2.0 - parent_x_height / 2.0
        }
        VerticalAlign::TextTop => {
            item_ascent - parent_ascent
        }
        VerticalAlign::TextBottom => {
            parent_descent - item_descent
        }
        VerticalAlign::Length(px) => -px,
        VerticalAlign::Percentage(pct) => {
            -(parent_line_spacing * pct / 100.0)
        }
        // Top/Bottom need deferred resolution after full line height is known.
        // Return 0.0 here; resolved in a second pass.
        VerticalAlign::Top | VerticalAlign::Bottom => 0.0,
    }
}

// ── Text alignment (CSS 2.2 §16.2) ──────────────────────────────────────

/// Compute the inline-start offset for text-align.
///
/// Blink: `InlineLayoutAlgorithm::ApplyTextAlign`.
fn compute_text_align_offset(
    line_info: &LineInfo,
    available_width: LayoutUnit,
    direction: Direction,
) -> LayoutUnit {
    let remaining = available_width - line_info.used_width;
    if remaining <= LayoutUnit::zero() {
        return LayoutUnit::zero();
    }

    // On the last line, justify falls back to start alignment.
    let effective_align = if line_info.is_last_line || line_info.has_forced_break {
        match line_info.text_align {
            TextAlign::Justify => TextAlign::Start,
            other => other,
        }
    } else {
        line_info.text_align
    };

    match effective_align {
        TextAlign::Left => LayoutUnit::zero(),
        TextAlign::Right => remaining,
        TextAlign::Center => LayoutUnit::from_raw(remaining.raw() / 2),
        TextAlign::Justify => {
            // Justification is handled by expanding spaces; offset is 0.
            LayoutUnit::zero()
        }
        TextAlign::Start => {
            if direction == Direction::Rtl {
                remaining
            } else {
                LayoutUnit::zero()
            }
        }
        TextAlign::End => {
            if direction == Direction::Rtl {
                LayoutUnit::zero()
            } else {
                remaining
            }
        }
    }
}

/// Count expansion opportunities (spaces between words) for justification.
fn count_expansion_opportunities(line_info: &LineInfo, items_data: &InlineItemsData) -> usize {
    let mut count = 0;
    for item_result in &line_info.items {
        if item_result.item_type == InlineItemType::Text {
            let text = &items_data.text[item_result.text_range.clone()];
            count += text.chars().filter(|c| *c == ' ').count();
        }
    }
    count
}

// ── Inline start/end resolution for open/close tag items ─────────────────

/// Resolve inline-start contribution of an OpenTag item (margin-left + border-left + padding-left).
fn resolve_inline_start(style: &ComputedStyle, percentage_base: LayoutUnit) -> LayoutUnit {
    let margin = resolve_margin_or_padding(&style.margin_left, percentage_base);
    let border = LayoutUnit::from_i32(style.effective_border_left());
    let padding = resolve_margin_or_padding(&style.padding_left, percentage_base);
    margin + border + padding
}

/// Resolve inline-end contribution of a CloseTag item (padding-right + border-right + margin-right).
fn resolve_inline_end(style: &ComputedStyle, percentage_base: LayoutUnit) -> LayoutUnit {
    let padding = resolve_margin_or_padding(&style.padding_right, percentage_base);
    let border = LayoutUnit::from_i32(style.effective_border_right());
    let margin = resolve_margin_or_padding(&style.margin_right, percentage_base);
    padding + border + margin
}

// ── Main entry point ─────────────────────────────────────────────────────

/// Perform inline layout for a block node that has inline children.
///
/// This is the inline formatting context (IFC) layout algorithm.
/// Returns a Fragment containing line box fragments as children.
///
/// Blink: `InlineLayoutAlgorithm::Layout()` in `inline_layout_algorithm.cc`.
pub fn inline_layout(
    doc: &Document,
    node_id: NodeId,
    space: &ConstraintSpace,
) -> Fragment {
    let style = &doc.node(node_id).style;

    // Resolve border + padding for the containing block.
    let border = resolve_border(style);
    let padding = resolve_padding(style, space.percentage_resolution_inline_size);
    let border_padding = border + padding;

    let content_inline_size = space.available_inline_size
        - border_padding.left
        - border_padding.right;
    let available_inline_size = content_inline_size.clamp_negative_to_zero();

    // Step 1: Collect inline items from DOM children.
    let mut items_data = InlineItemsBuilder::collect(doc, node_id);

    // Step 1b: Apply bidi analysis.
    let base_direction = if style.direction == Direction::Rtl {
        openui_text::TextDirection::Rtl
    } else {
        openui_text::TextDirection::Ltr
    };
    items_data.apply_bidi(base_direction);

    // Step 2: Shape all text items.
    items_data.shape_text();

    // Step 3: Create line breaker.
    let mut line_breaker = LineBreaker::new(&items_data);
    line_breaker.set_text_align(style.text_align);

    // Step 3b: Resolve text-indent for the first line.
    let text_indent = crate::length_resolver::resolve_length(
        &style.text_indent,
        available_inline_size,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
    );

    // Get block's font metrics for the strut.
    let block_font_desc = style_to_font_description(style);
    let block_font = Font::new(block_font_desc);
    let block_metrics = block_font
        .font_metrics()
        .copied()
        .unwrap_or_default();

    // Step 4: Layout each line.
    let mut line_fragments: Vec<Fragment> = Vec::new();
    let mut block_offset = border_padding.top;
    let mut is_first_line = true;

    while !line_breaker.is_finished() {
        // Apply text-indent: reduce available width on first line only.
        let line_available = if is_first_line && text_indent != LayoutUnit::zero() {
            (available_inline_size - text_indent).clamp_negative_to_zero()
        } else {
            available_inline_size
        };

        if let Some(mut line_info) = line_breaker.next_line(line_available) {
            // Step 4b: BiDi reorder items on this line for visual display.
            bidi_reorder_line(&mut line_info.items, &items_data);

            // Apply text-overflow: ellipsis if configured on the block style.
            if style.text_overflow == openui_style::TextOverflow::Ellipsis
                && style.overflow_x == openui_style::Overflow::Hidden
            {
                apply_text_overflow_ellipsis(&mut line_info, line_available);
            }

            let line_fragment = create_line_box(
                doc,
                &items_data,
                &line_info,
                available_inline_size,
                block_offset,
                style,
                &block_metrics,
                space.percentage_resolution_inline_size,
                if is_first_line { text_indent } else { LayoutUnit::zero() },
            );
            block_offset = block_offset + line_fragment.size.height;
            line_fragments.push(line_fragment);
            is_first_line = false;
        }
    }

    // If no lines were produced but the block has inline content (e.g., empty
    // text), ensure at least the strut height.
    let intrinsic_block_size = block_offset + border_padding.bottom;

    // Build the container fragment.
    let border_box_inline = space.available_inline_size;
    let border_box_size = PhysicalSize::new(border_box_inline, intrinsic_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.border = border;
    fragment.padding = padding;
    fragment.children = line_fragments;
    fragment
}

// ── Line box creation ────────────────────────────────────────────────────

/// Create a positioned line box fragment from a LineInfo.
///
/// This is the core of the inline layout algorithm:
/// 1. Compute line height using the half-leading model
/// 2. Apply vertical alignment
/// 3. Apply text alignment (horizontal offset)
/// 4. Position each item within the line box
///
/// Blink: `InlineLayoutAlgorithm::CreateLine()`.
fn create_line_box(
    _doc: &Document,
    items_data: &InlineItemsData,
    line_info: &LineInfo,
    available_width: LayoutUnit,
    block_offset: LayoutUnit,
    block_style: &ComputedStyle,
    block_metrics: &openui_text::FontMetrics,
    percentage_base: LayoutUnit,
    text_indent: LayoutUnit,
) -> Fragment {
    // === STEP 1: Compute strut (minimum line height from block's font) ===
    let strut = compute_line_height_metrics(
        block_metrics.ascent,
        block_metrics.descent,
        &block_style.line_height,
        block_style.font_size,
        block_metrics.line_spacing,
    );

    let mut line_ascent = strut.ascent;
    let mut line_descent = strut.descent;

    // Track items that need deferred vertical-align resolution (top/bottom).
    struct DeferredItem {
        item_ascent: f32,
        item_descent: f32,
        is_top: bool,
    }
    let mut deferred_items: Vec<DeferredItem> = Vec::new();

    // === STEP 2: Compute per-item metrics and unite ===
    for item_result in &line_info.items {
        match item_result.item_type {
            InlineItemType::Text => {
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();
                let item_lh = compute_line_height_metrics(
                    metrics.ascent,
                    metrics.descent,
                    &style.line_height,
                    style.font_size,
                    metrics.line_spacing,
                );

                let baseline_shift = compute_baseline_shift(
                    &style.vertical_align,
                    style.font_size,
                    block_metrics.ascent,
                    block_metrics.descent,
                    block_metrics.x_height,
                    metrics.ascent,
                    metrics.descent,
                    block_metrics.line_spacing,
                );

                match style.vertical_align {
                    VerticalAlign::Top => {
                        deferred_items.push(DeferredItem {
                            item_ascent: item_lh.ascent,
                            item_descent: item_lh.descent,
                            is_top: true,
                        });
                    }
                    VerticalAlign::Bottom => {
                        deferred_items.push(DeferredItem {
                            item_ascent: item_lh.ascent,
                            item_descent: item_lh.descent,
                            is_top: false,
                        });
                    }
                    _ => {
                        line_ascent = line_ascent.max(item_lh.ascent - baseline_shift);
                        line_descent = line_descent.max(item_lh.descent + baseline_shift);
                    }
                }
            }
            InlineItemType::AtomicInline => {
                // Atomic inline contributes its margin box height.
                // For now, treat as baseline-aligned with its margin box.
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();
                line_ascent = line_ascent.max(metrics.ascent);
                line_descent = line_descent.max(metrics.descent);
            }
            _ => {}
        }
    }

    // === STEP 2b: Resolve deferred top/bottom items ===
    // Top/bottom aligned items may expand the line box but use the already-
    // computed line height from other items.
    for deferred in &deferred_items {
        let item_total = deferred.item_ascent + deferred.item_descent;
        let line_total = line_ascent + line_descent;
        if item_total > line_total {
            // Expand the line to fit this item.
            let extra = item_total - line_total;
            if deferred.is_top {
                // Top-aligned: extra goes to descent side.
                line_descent += extra;
            } else {
                // Bottom-aligned: extra goes to ascent side.
                line_ascent += extra;
            }
        }
    }

    let line_height = LayoutUnit::from_f32_ceil(line_ascent + line_descent);
    let baseline = LayoutUnit::from_f32_ceil(line_ascent);

    // === STEP 3: Horizontal positioning (text-align) ===
    let text_align_offset = compute_text_align_offset(
        line_info,
        available_width,
        block_style.direction,
    );

    // === STEP 3b: Justification ===
    // Distribute extra space among word gaps if text-align: justify.
    let mut justification_per_space = 0.0f32;
    let should_justify = line_info.text_align == TextAlign::Justify
        && !line_info.is_last_line
        && !line_info.has_forced_break;
    if should_justify {
        let remaining = available_width - line_info.used_width;
        if remaining > LayoutUnit::zero() {
            let space_count = count_expansion_opportunities(line_info, items_data);
            if space_count > 0 {
                justification_per_space = remaining.to_f32() / space_count as f32;
            }
        }
    }

    // === STEP 4: Position each item ===
    let mut children: Vec<Fragment> = Vec::new();
    let mut inline_offset = text_align_offset + text_indent;
    let mut justification_accumulator = 0.0f32;

    for (_i, item_result) in line_info.items.iter().enumerate() {
        let item = &items_data.items[item_result.item_index];
        match item_result.item_type {
            InlineItemType::Text => {
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();

                let baseline_shift = compute_baseline_shift(
                    &style.vertical_align,
                    style.font_size,
                    block_metrics.ascent,
                    block_metrics.descent,
                    block_metrics.x_height,
                    metrics.ascent,
                    metrics.descent,
                    block_metrics.line_spacing,
                );

                // Compute vertical offset for top/bottom aligned items.
                let effective_shift = match style.vertical_align {
                    VerticalAlign::Top => {
                        // Align top of item with top of line box.
                        let item_lh = compute_line_height_metrics(
                            metrics.ascent,
                            metrics.descent,
                            &style.line_height,
                            style.font_size,
                            metrics.line_spacing,
                        );
                        -(line_ascent - item_lh.ascent)
                    }
                    VerticalAlign::Bottom => {
                        // Align bottom of item with bottom of line box.
                        let item_lh = compute_line_height_metrics(
                            metrics.ascent,
                            metrics.descent,
                            &style.line_height,
                            style.font_size,
                            metrics.line_spacing,
                        );
                        line_descent - item_lh.descent
                    }
                    _ => baseline_shift,
                };

                // Text top = baseline position - font ascent, adjusted for shift.
                let text_top = baseline
                    - LayoutUnit::from_f32_ceil(metrics.ascent - effective_shift);

                // Compute item width, adding justification if applicable.
                let mut item_width = item_result.inline_size;
                if should_justify && justification_per_space > 0.0 {
                    let text = &items_data.text[item_result.text_range.clone()];
                    let space_count = text.chars().filter(|c| *c == ' ').count();
                    if space_count > 0 {
                        let extra = justification_per_space * space_count as f32;
                        justification_accumulator += extra;
                        let extra_lu = LayoutUnit::from_f32(justification_accumulator)
                            - LayoutUnit::from_f32(
                                justification_accumulator - extra,
                            );
                        item_width = item_width + extra_lu;
                    }
                }

                let text_height = LayoutUnit::from_f32_ceil(metrics.ascent + metrics.descent);

                let mut text_fragment = Fragment::new_box(item.node_id, PhysicalSize::new(
                    item_width,
                    text_height,
                ));
                text_fragment.kind = FragmentKind::Text;
                text_fragment.offset = PhysicalOffset::new(inline_offset, text_top);
                text_fragment.shape_result = item_result.shape_result.clone();

                children.push(text_fragment);
                inline_offset = inline_offset + item_width;
            }
            InlineItemType::OpenTag => {
                let style = &items_data.styles[item.style_index];
                inline_offset = inline_offset + resolve_inline_start(style, percentage_base);
            }
            InlineItemType::CloseTag => {
                let style = &items_data.styles[item.style_index];
                inline_offset = inline_offset + resolve_inline_end(style, percentage_base);
            }
            InlineItemType::Control => {
                // <br> — no visual contribution, line break already handled.
            }
            InlineItemType::AtomicInline | InlineItemType::BlockInInline => {
                // AtomicInline uses its measured inline_size from the line breaker.
                inline_offset = inline_offset + item_result.inline_size;
            }
        }
    }

    // Build the line box fragment.
    let mut line_fragment = Fragment::new_box(NodeId::NONE, PhysicalSize::new(
        available_width,
        line_height,
    ));
    line_fragment.offset = PhysicalOffset::new(LayoutUnit::zero(), block_offset);
    line_fragment.children = children;
    line_fragment
}

// ── BiDi visual reordering ──────────────────────────────────────────────

/// Reorder items within a line for visual display per UAX#9 L2.
///
/// After the line breaker produces a line in logical order, this function
/// reorders items so RTL runs are visually reversed.
///
/// Blink: `InlineLayoutAlgorithm::BidiReorder` / `ReorderInlineItems`.
fn bidi_reorder_line(items: &mut Vec<InlineItemResult>, items_data: &InlineItemsData) {
    if items.is_empty() {
        return;
    }

    let max_level = items
        .iter()
        .map(|ir| items_data.items[ir.item_index].bidi_level)
        .max()
        .unwrap_or(0);

    if max_level == 0 {
        return; // All LTR, no reordering needed
    }

    let min_odd = items
        .iter()
        .map(|ir| items_data.items[ir.item_index].bidi_level)
        .filter(|l| l % 2 == 1)
        .min()
        .unwrap_or(max_level);

    // UAX#9 L2: for each level from max down to min odd level,
    // reverse every maximal contiguous run of items at that level or higher.
    for level in (min_odd..=max_level).rev() {
        let mut i = 0;
        while i < items.len() {
            let item_level = items_data.items[items[i].item_index].bidi_level;
            if item_level >= level {
                let start = i;
                while i < items.len() {
                    let l = items_data.items[items[i].item_index].bidi_level;
                    if l >= level {
                        i += 1;
                    } else {
                        break;
                    }
                }
                items[start..i].reverse();
            } else {
                i += 1;
            }
        }
    }
}

// ── Text-overflow: ellipsis ─────────────────────────────────────────────

/// Apply text-overflow: ellipsis to a line that overflows.
///
/// Removes trailing items until the line fits within available width
/// minus the ellipsis width. The caller is responsible for actually
/// painting the ellipsis character.
///
/// Blink: `NGLineInfo::SetHasEllipsis` / `NGLineTruncator`.
fn apply_text_overflow_ellipsis(line_info: &mut LineInfo, available_width: LayoutUnit) {
    if line_info.used_width <= available_width {
        return;
    }

    // Approximate ellipsis width as ~3 dots. We use a conservative estimate.
    // In a full implementation this would be measured from the font.
    let ellipsis_width = LayoutUnit::from_f32(12.0);
    let target_width = available_width - ellipsis_width;

    if target_width <= LayoutUnit::zero() {
        return;
    }

    // Remove items from the end until we have room for the ellipsis.
    while line_info.used_width > target_width && !line_info.items.is_empty() {
        let last = line_info.items.last().unwrap();
        let last_size = last.inline_size;
        if last_size <= LayoutUnit::zero()
            && last.item_type != InlineItemType::Text
        {
            // Non-content items (open/close tags) — just remove
            line_info.items.pop();
            continue;
        }
        line_info.used_width = line_info.used_width - last_size;
        line_info.items.pop();
    }

    line_info.has_ellipsis = true;
}

/// Check if a block node has any inline children (text or inline-level elements).
///
/// Used by block_layout to detect when to dispatch to inline layout.
pub fn has_inline_children(doc: &Document, node_id: NodeId) -> bool {
    for child_id in doc.children(node_id) {
        let child = doc.node(child_id);
        if child.tag == openui_dom::ElementTag::Text {
            return true;
        }
        if child.style.display.is_inline_level() {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::ElementTag;
    use openui_style::Display;

    #[test]
    fn line_height_metrics_normal() {
        // Normal line height uses line_spacing from font metrics
        let m = compute_line_height_metrics(
            10.0,  // ascent
            4.0,   // descent
            &LineHeight::Normal,
            16.0,  // font_size
            16.0,  // line_spacing (ascent + descent + gap = 10 + 4 + 2)
        );
        // leading = 16 - 14 = 2, half_leading_floor = 1, half_leading_ceil = 1
        assert_eq!(m.ascent, 11.0);
        assert_eq!(m.descent, 5.0);
    }

    #[test]
    fn line_height_metrics_number() {
        // line-height: 2.0 doubles line height
        let m = compute_line_height_metrics(
            10.0, 4.0,
            &LineHeight::Number(2.0),
            16.0,  // font_size
            16.0,  // line_spacing
        );
        // computed = 16 * 2 = 32, leading = 32 - 14 = 18
        // half_leading_floor = 9, half_leading_ceil = 9
        assert_eq!(m.ascent, 19.0);
        assert_eq!(m.descent, 13.0);
    }

    #[test]
    fn line_height_metrics_length() {
        let m = compute_line_height_metrics(
            10.0, 4.0,
            &LineHeight::Length(24.0),
            16.0, 16.0,
        );
        // leading = 24 - 14 = 10, floor(5) = 5, ceil = 5
        assert_eq!(m.ascent, 15.0);
        assert_eq!(m.descent, 9.0);
    }

    #[test]
    fn line_height_metrics_percentage() {
        let m = compute_line_height_metrics(
            10.0, 4.0,
            &LineHeight::Percentage(150.0),
            16.0, 16.0,
        );
        // computed = 16 * 150 / 100 = 24, leading = 10
        assert_eq!(m.ascent, 15.0);
        assert_eq!(m.descent, 9.0);
    }

    #[test]
    fn line_height_half_leading_odd() {
        // Odd leading: floor on ascent, ceil on descent
        let m = compute_line_height_metrics(
            10.0, 4.0,
            &LineHeight::Length(25.0),
            16.0, 16.0,
        );
        // leading = 25 - 14 = 11, floor(5.5) = 5, ceil = 6
        assert_eq!(m.ascent, 15.0);
        assert_eq!(m.descent, 10.0);
    }

    #[test]
    fn baseline_shift_baseline() {
        let shift = compute_baseline_shift(
            &VerticalAlign::Baseline,
            16.0, 10.0, 4.0, 8.0, 10.0, 4.0, 16.0,
        );
        assert_eq!(shift, 0.0);
    }

    #[test]
    fn baseline_shift_sub() {
        let shift = compute_baseline_shift(
            &VerticalAlign::Sub,
            16.0, 10.0, 4.0, 8.0, 10.0, 4.0, 16.0,
        );
        assert_eq!(shift, 16.0 / 5.0 + 1.0);
    }

    #[test]
    fn baseline_shift_super() {
        let shift = compute_baseline_shift(
            &VerticalAlign::Super,
            16.0, 10.0, 4.0, 8.0, 10.0, 4.0, 16.0,
        );
        assert_eq!(shift, -(16.0 / 3.0 + 1.0));
    }

    #[test]
    fn text_align_left() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Left, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_right() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Right, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr);
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_center() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Center, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr);
        assert_eq!(offset.to_i32(), 20);
    }

    #[test]
    fn text_align_start_ltr() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Start, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_start_rtl() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Start, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Rtl);
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_end_ltr() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::End, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr);
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_end_rtl() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::End, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Rtl);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_justify_last_line_falls_back() {
        // Justify on the last line falls back to start alignment.
        let mut line = make_test_line_info(100.0, 60.0, TextAlign::Justify, false);
        line.is_last_line = true;
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_overflow_no_offset() {
        // When content overflows, offset should be 0.
        let line = make_test_line_info(100.0, 150.0, TextAlign::Right, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn has_inline_children_text_node() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("hello".to_string());
        doc.append_child(block, text);

        assert!(has_inline_children(&doc, block));
    }

    #[test]
    fn has_inline_children_span() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.append_child(block, span);

        assert!(has_inline_children(&doc, block));
    }

    #[test]
    fn has_inline_children_block_only() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.append_child(block, child);

        assert!(!has_inline_children(&doc, block));
    }

    // ── Helper ───────────────────────────────────────────────────────────

    fn make_test_line_info(
        available: f32,
        used: f32,
        align: TextAlign,
        is_last: bool,
    ) -> LineInfo {
        let mut info = LineInfo::new(LayoutUnit::from_f32(available));
        info.used_width = LayoutUnit::from_f32(used);
        info.text_align = align;
        info.is_last_line = is_last;
        info
    }
}
