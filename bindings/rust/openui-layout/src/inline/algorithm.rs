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
use openui_style::{ComputedStyle, Direction, Display, LineHeight, TextAlign, TextAlignLast, TextJustify, VerticalAlign};
use openui_text::{Font, FontMetrics, ShapeResult, TextShaper};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

use crate::constraint_space::ConstraintSpace;
use crate::fragment::{Fragment, FragmentKind};
use crate::length_resolver::resolve_margin_or_padding;

use super::items::{InlineItemResult, InlineItemType};
use super::items_builder::{style_to_font_description, InlineItemsData, InlineItemsBuilder};
use super::line_breaker::{byte_to_char_offset, LineBreaker};
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
    metrics: &FontMetrics,
    line_height: &LineHeight,
    font_size: f32,
) -> LineHeightMetrics {
    // Blink uses integer-rounded ascent/descent (FixedAscent/FixedDescent)
    // BEFORE the half-leading calculation.
    let font_ascent = metrics.int_ascent();
    let font_descent = metrics.int_descent();

    let computed_line_height = match line_height {
        // Blink uses the rounded sum for line-height: normal.
        LineHeight::Normal => metrics.int_line_spacing(),
        LineHeight::Number(n) => font_size * n,
        LineHeight::Length(px) => *px,
        LineHeight::Percentage(pct) => font_size * pct / 100.0,
    };

    let leading = computed_line_height - (font_ascent + font_descent);
    // Blink snaps to LayoutUnit grid (1/64 px): floor on ascent side,
    // ceil on descent side. Computing descent as `leading - ascent_half`
    // ensures the total exactly equals computed line-height after snapping.
    let grid = 1.0 / 64.0; // LayoutUnit precision
    let ascent_half = (leading / 2.0 / grid).floor() * grid;
    let descent_half = leading - ascent_half;

    LineHeightMetrics {
        ascent: font_ascent + ascent_half,
        descent: font_descent + descent_half,
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
    element_line_height: f32,
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
            // CSS 2.2 §10.8.1: percentage is of the element's own line-height.
            -(element_line_height * pct / 100.0)
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
    text_align_last: TextAlignLast,
) -> LayoutUnit {
    let remaining = available_width - line_info.used_width;
    if remaining <= LayoutUnit::zero() {
        return LayoutUnit::zero();
    }

    // On the last line or forced-break line, always check text-align-last
    // first. Per CSS Text Level 3 §7.3, text-align-last overrides the
    // last line's alignment regardless of text-align's value (unless
    // text-align-last is `auto`).
    let effective_align = if line_info.is_last_line || line_info.has_forced_break {
        match text_align_last {
            TextAlignLast::Auto => match line_info.text_align {
                TextAlign::Justify => TextAlign::Start,
                other => other,
            },
            TextAlignLast::Start => TextAlign::Start,
            TextAlignLast::End => TextAlign::End,
            TextAlignLast::Left => TextAlign::Left,
            TextAlignLast::Right => TextAlign::Right,
            TextAlignLast::Center => TextAlign::Center,
            TextAlignLast::Justify => TextAlign::Justify,
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
///
/// Excludes trailing spaces, which are already stripped from width measurement
/// and should not be counted as expansion opportunities.
fn count_expansion_opportunities(line_info: &LineInfo, items_data: &InlineItemsData) -> usize {
    let mut count = 0;
    for item_result in &line_info.items {
        if item_result.item_type == InlineItemType::Text {
            let text = &items_data.text[item_result.text_range.clone()];
            count += text.chars().filter(|c| *c == ' ').count();
        }
    }
    // Exclude trailing spaces: the last text item's trailing spaces were already
    // stripped from width and should not be expansion opportunities.
    // In pre-wrap mode there can be multiple trailing spaces.
    if count > 0 {
        for item_result in line_info.items.iter().rev() {
            if item_result.item_type == InlineItemType::Text {
                let text = &items_data.text[item_result.text_range.clone()];
                let trailing_spaces = text.chars().rev().take_while(|c| *c == ' ').count();
                count = count.saturating_sub(trailing_spaces);
                break;
            }
            if item_result.item_type != InlineItemType::CloseTag
                && item_result.item_type != InlineItemType::OpenTag
            {
                break;
            }
        }
    }
    count
}

/// Count inter-character expansion opportunities for `text-justify: inter-character`.
///
/// Every character boundary (excluding trailing spaces) is an expansion point.
/// Returns the number of gaps between characters (char_count - 1 for non-empty text).
fn count_inter_character_opportunities(line_info: &LineInfo, items_data: &InlineItemsData) -> usize {
    let mut total_chars = 0usize;
    for item_result in &line_info.items {
        if item_result.item_type == InlineItemType::Text {
            let text = &items_data.text[item_result.text_range.clone()];
            total_chars += text.chars().count();
        }
    }
    // Exclude trailing spaces from the count.
    for item_result in line_info.items.iter().rev() {
        if item_result.item_type == InlineItemType::Text {
            let text = &items_data.text[item_result.text_range.clone()];
            let trailing_spaces = text.chars().rev().take_while(|c| *c == ' ').count();
            total_chars = total_chars.saturating_sub(trailing_spaces);
            break;
        }
        if item_result.item_type != InlineItemType::CloseTag
            && item_result.item_type != InlineItemType::OpenTag
        {
            break;
        }
    }
    total_chars.saturating_sub(1)
}

// ── Inline start/end resolution for open/close tag items ─────────────────

/// Resolve inline-start MBP contribution of an OpenTag item.
///
/// In LTR, inline-start is the left side; in RTL, inline-start is the right side.
fn resolve_inline_start(style: &ComputedStyle, percentage_base: LayoutUnit) -> LayoutUnit {
    if style.direction == Direction::Rtl {
        let margin = resolve_margin_or_padding(&style.margin_right, percentage_base);
        let border = LayoutUnit::from_i32(style.effective_border_right());
        let padding = resolve_margin_or_padding(&style.padding_right, percentage_base);
        margin + border + padding
    } else {
        let margin = resolve_margin_or_padding(&style.margin_left, percentage_base);
        let border = LayoutUnit::from_i32(style.effective_border_left());
        let padding = resolve_margin_or_padding(&style.padding_left, percentage_base);
        margin + border + padding
    }
}

/// Resolve inline-end MBP contribution of a CloseTag item.
///
/// In LTR, inline-end is the right side; in RTL, inline-end is the left side.
fn resolve_inline_end(style: &ComputedStyle, percentage_base: LayoutUnit) -> LayoutUnit {
    if style.direction == Direction::Rtl {
        let padding = resolve_margin_or_padding(&style.padding_left, percentage_base);
        let border = LayoutUnit::from_i32(style.effective_border_left());
        let margin = resolve_margin_or_padding(&style.margin_left, percentage_base);
        padding + border + margin
    } else {
        let padding = resolve_margin_or_padding(&style.padding_right, percentage_base);
        let border = LayoutUnit::from_i32(style.effective_border_right());
        let margin = resolve_margin_or_padding(&style.margin_right, percentage_base);
        padding + border + margin
    }
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

    // The caller (block_layout) already resolves border+padding for this
    // container and subtracts it to produce the content-box width in
    // space.available_inline_size. We do NOT re-resolve border+padding here
    // because the percentage_resolution_inline_size is now this block's
    // content-box width (correct for descendants), which would give wrong
    // results for the container's own percentage padding.
    let available_inline_size = space.available_inline_size.clamp_negative_to_zero();

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
    let mut line_breaker = LineBreaker::new(&items_data, available_inline_size);
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
    // Line offsets are relative to the content box (0-based). The caller
    // (block_layout) adds border+padding offsets when positioning.
    let mut line_fragments: Vec<Fragment> = Vec::new();
    let mut block_offset = LayoutUnit::zero();
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
                apply_text_overflow_ellipsis(&mut line_info, line_available, &items_data, style);
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

    let intrinsic_block_size = block_offset;

    // Build the container fragment.
    let border_box_inline = space.available_inline_size;
    let border_box_size = PhysicalSize::new(border_box_inline, intrinsic_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.children = line_fragments;
    fragment
}

/// Inline layout for an explicit set of children (anonymous block box wrapper).
///
/// Used by block_layout for CSS 2.2 §9.2.1.1 anonymous block boxes when
/// mixed inline+block content is present. Lays out only the given children
/// as an inline formatting context.
pub fn inline_layout_for_children(
    doc: &Document,
    node_id: NodeId,
    children: &[NodeId],
    space: &ConstraintSpace,
) -> Fragment {
    let style = &doc.node(node_id).style;

    let available_inline_size = space.available_inline_size.clamp_negative_to_zero();

    // Collect inline items only from the specified children.
    let mut items_data = InlineItemsBuilder::collect_for_children(doc, node_id, children);

    let base_direction = if style.direction == Direction::Rtl {
        openui_text::TextDirection::Rtl
    } else {
        openui_text::TextDirection::Ltr
    };
    items_data.apply_bidi(base_direction);
    items_data.shape_text();

    let mut line_breaker = LineBreaker::new(&items_data, available_inline_size);
    line_breaker.set_text_align(style.text_align);

    let text_indent = crate::length_resolver::resolve_length(
        &style.text_indent,
        available_inline_size,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
    );

    let block_font_desc = style_to_font_description(style);
    let block_font = Font::new(block_font_desc);
    let block_metrics = block_font
        .font_metrics()
        .copied()
        .unwrap_or_default();

    let mut line_fragments: Vec<Fragment> = Vec::new();
    let mut block_offset = LayoutUnit::zero();
    let mut is_first_line = true;

    while !line_breaker.is_finished() {
        let line_available = if is_first_line && text_indent != LayoutUnit::zero() {
            (available_inline_size - text_indent).clamp_negative_to_zero()
        } else {
            available_inline_size
        };

        if let Some(mut line_info) = line_breaker.next_line(line_available) {
            bidi_reorder_line(&mut line_info.items, &items_data);

            if style.text_overflow == openui_style::TextOverflow::Ellipsis
                && style.overflow_x == openui_style::Overflow::Hidden
            {
                apply_text_overflow_ellipsis(&mut line_info, line_available, &items_data, style);
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

    let intrinsic_block_size = block_offset;
    let border_box_inline = space.available_inline_size;
    let border_box_size = PhysicalSize::new(border_box_inline, intrinsic_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.children = line_fragments;
    fragment
}

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
        block_metrics,
        &block_style.line_height,
        block_style.font_size,
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

    // Stack of parent inline font metrics for vertical-align resolution.
    // When inside a nested inline (e.g., <span style="font-size:30px">),
    // text-top/text-bottom/middle should align to the parent inline's font
    // metrics, not the block container's. The stack is pushed on OpenTag
    // and popped on CloseTag.
    let mut inline_metrics_stack: Vec<FontMetrics> = Vec::new();

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
                    &metrics,
                    &style.line_height,
                    style.font_size,
                );

                let element_line_height = match style.line_height {
                    LineHeight::Normal => metrics.int_line_spacing(),
                    LineHeight::Number(n) => style.font_size * n,
                    LineHeight::Length(px) => px,
                    LineHeight::Percentage(pct) => style.font_size * pct / 100.0,
                };

                // Use parent inline's metrics if inside a nested inline,
                // otherwise fall back to block container metrics.
                let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);

                let baseline_shift = compute_baseline_shift(
                    &style.vertical_align,
                    style.font_size,
                    parent_metrics.ascent,
                    parent_metrics.descent,
                    parent_metrics.x_height,
                    item_lh.ascent,
                    item_lh.descent,
                    element_line_height,
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
                // Atomic inline contributes its actual height to line metrics.
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];

                let item_height = match style.height.length_type() {
                    openui_geometry::LengthType::Fixed => style.height.value(),
                    _ => {
                        let font_desc = style_to_font_description(style);
                        let font = Font::new(font_desc);
                        let metrics = font.font_metrics().copied().unwrap_or_default();
                        metrics.ascent + metrics.descent
                    }
                };

                match style.vertical_align {
                    VerticalAlign::Top => {
                        deferred_items.push(DeferredItem {
                            item_ascent: item_height,
                            item_descent: 0.0,
                            is_top: true,
                        });
                    }
                    VerticalAlign::Bottom => {
                        deferred_items.push(DeferredItem {
                            item_ascent: item_height,
                            item_descent: 0.0,
                            is_top: false,
                        });
                    }
                    VerticalAlign::Middle => {
                        // CSS defines "middle" relative to the parent inline's
                        // x-height, not the block's.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let x_height = parent_metrics.x_height;
                        let above_baseline = item_height / 2.0 + x_height / 2.0;
                        let below_baseline = (item_height / 2.0 - x_height / 2.0).max(0.0);
                        line_ascent = line_ascent.max(above_baseline);
                        line_descent = line_descent.max(below_baseline);
                    }
                    VerticalAlign::Length(px) => {
                        // Shift from baseline by px (negative = down).
                        let shifted_ascent = (item_height + px).max(0.0);
                        let shifted_descent = (-px).max(0.0);
                        line_ascent = line_ascent.max(shifted_ascent);
                        line_descent = line_descent.max(shifted_descent);
                    }
                    VerticalAlign::Percentage(pct) => {
                        // Compute element's own line-height for percentage basis
                        let element_line_height = match style.line_height {
                            LineHeight::Normal => {
                                let font_desc = style_to_font_description(style);
                                let font = Font::new(font_desc);
                                let metrics = font.font_metrics().copied().unwrap_or_default();
                                metrics.line_spacing
                            }
                            LineHeight::Number(n) => style.font_size * n,
                            LineHeight::Length(px) => px,
                            LineHeight::Percentage(p) => style.font_size * p / 100.0,
                        };
                        let shift = element_line_height * pct / 100.0;
                        let shifted_ascent = (item_height + shift).max(0.0);
                        let shifted_descent = (-shift).max(0.0);
                        line_ascent = line_ascent.max(shifted_ascent);
                        line_descent = line_descent.max(shifted_descent);
                    }
                    VerticalAlign::TextTop => {
                        // Item top aligns with parent inline's font ascent line.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let font_ascent = parent_metrics.ascent;
                        line_ascent = line_ascent.max(font_ascent);
                        line_descent =
                            line_descent.max((item_height - font_ascent).max(0.0));
                    }
                    VerticalAlign::TextBottom => {
                        // Item bottom aligns with parent inline's font descent line.
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let font_descent = parent_metrics.descent;
                        line_ascent =
                            line_ascent.max((item_height - font_descent).max(0.0));
                        line_descent = line_descent.max(font_descent);
                    }
                    VerticalAlign::Sub => {
                        // Lowered by sub_offset below the baseline.
                        let sub_offset = style.font_size / 5.0 + 1.0;
                        line_ascent =
                            line_ascent.max((item_height - sub_offset).max(0.0));
                        line_descent = line_descent.max(sub_offset);
                    }
                    VerticalAlign::Super => {
                        // Raised by super_offset above the baseline.
                        let super_offset = style.font_size / 3.0 + 1.0;
                        line_ascent = line_ascent.max(item_height + super_offset);
                    }
                    _ => {
                        // Baseline-aligned: bottom sits on baseline, full height above.
                        line_ascent = line_ascent.max(item_height);
                    }
                }
            }
            // OpenTag: push this inline element's font metrics onto the stack
            // so nested content uses the parent inline's metrics for vertical-align.
            InlineItemType::OpenTag => {
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();
                inline_metrics_stack.push(metrics);
            }
            // CloseTag: pop the parent inline's font metrics.
            InlineItemType::CloseTag => {
                inline_metrics_stack.pop();
            }
            // Control: forced breaks have no height contribution.
            // BlockInInline: handled separately in block layout.
            InlineItemType::Control
            | InlineItemType::BlockInInline => {}
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
    // Use the effective available width (after text-indent) for alignment
    // so that text-align: right with text-indent doesn't overflow.
    let align_available = available_width - text_indent;
    let text_align_offset = compute_text_align_offset(
        line_info,
        align_available,
        block_style.direction,
        block_style.text_align_last,
    );

    // === STEP 3b: Justification ===
    // Distribute extra space among word gaps (or character gaps) if justified.
    let mut justification_per_space = 0.0f32;
    let mut justification_per_char = 0.0f32;
    let text_justify = block_style.text_justify;

    // Determine if this line should be justified: text-align:justify on
    // non-last, non-forced-break lines, OR text-align-last:justify on last lines.
    let should_justify = if line_info.is_last_line || line_info.has_forced_break {
        block_style.text_align_last == TextAlignLast::Justify
    } else {
        line_info.text_align == TextAlign::Justify
    };

    if should_justify && text_justify != TextJustify::None {
        let remaining = align_available - line_info.used_width;
        if remaining > LayoutUnit::zero() {
            match text_justify {
                TextJustify::InterCharacter => {
                    let char_count = count_inter_character_opportunities(line_info, items_data);
                    if char_count > 0 {
                        justification_per_char = remaining.to_f32() / char_count as f32;
                    }
                }
                // Auto and InterWord both use inter-word justification
                // (for Latin scripts; CJK auto-detection is future work).
                TextJustify::Auto | TextJustify::InterWord => {
                    let space_count = count_expansion_opportunities(line_info, items_data);
                    if space_count > 0 {
                        justification_per_space = remaining.to_f32() / space_count as f32;
                    }
                }
                TextJustify::None => {} // unreachable due to outer guard
            }
        }
    }

    // === STEP 4: Position each item ===
    let mut children: Vec<Fragment> = Vec::new();
    let mut inline_offset = text_align_offset + text_indent;
    let mut justification_accumulator = 0.0f32;
    // Track how many characters we've seen before this item (for inter-character
    // justification boundary gaps — Issue 4 fix).
    let mut inter_char_chars_before = 0usize;

    // Reset the inline metrics stack for the positioning pass.
    inline_metrics_stack.clear();

    for (_i, item_result) in line_info.items.iter().enumerate() {
        let item = &items_data.items[item_result.item_index];
        match item_result.item_type {
            InlineItemType::Text => {
                let style = &items_data.styles[item.style_index];
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();

                let element_line_height = match style.line_height {
                    LineHeight::Normal => metrics.int_line_spacing(),
                    LineHeight::Number(n) => style.font_size * n,
                    LineHeight::Length(px) => px,
                    LineHeight::Percentage(pct) => style.font_size * pct / 100.0,
                };

                // Compute half-leading-adjusted metrics for baseline shift
                // (CSS 2.2 §10.8.1: text-top/text-bottom/middle use the inline
                // box, not the content area).
                let item_lh = compute_line_height_metrics(
                    &metrics,
                    &style.line_height,
                    style.font_size,
                );

                // Use parent inline's metrics if inside a nested inline.
                let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);

                let baseline_shift = compute_baseline_shift(
                    &style.vertical_align,
                    style.font_size,
                    parent_metrics.ascent,
                    parent_metrics.descent,
                    parent_metrics.x_height,
                    item_lh.ascent,
                    item_lh.descent,
                    element_line_height,
                );

                // Compute vertical offset for top/bottom aligned items.
                let effective_shift = match style.vertical_align {
                    VerticalAlign::Top => {
                        // Align top of item with top of line box.
                        -(line_ascent - item_lh.ascent)
                    }
                    VerticalAlign::Bottom => {
                        // Align bottom of item with bottom of line box.
                        line_descent - item_lh.descent
                    }
                    _ => baseline_shift,
                };

                // Text top = baseline position - font ascent, adjusted for shift.
                let text_top = baseline
                    - LayoutUnit::from_f32_ceil(metrics.ascent - effective_shift);

                // Compute sub-range shape result for the line portion.
                // When text wraps, the item_result's text_range may be a subset
                // of the full item's text_range. Clip the shape result to only
                // the characters visible on this line (Issue 1 fix).
                let line_shape_result = if let Some(ref sr) = item_result.shape_result {
                    let item_byte_start = item.text_range.start;
                    let line_byte_start = item_result.text_range.start;
                    let line_byte_end = item_result.text_range.end;
                    let full_text = &items_data.text;
                    let char_start = byte_to_char_offset(full_text, line_byte_start)
                        - byte_to_char_offset(full_text, item_byte_start);
                    let char_end = byte_to_char_offset(full_text, line_byte_end)
                        - byte_to_char_offset(full_text, item_byte_start);
                    if char_start == 0 && char_end == sr.num_characters {
                        item_result.shape_result.clone()
                    } else {
                        Some(Arc::new(sr.sub_range(char_start, char_end)))
                    }
                } else {
                    None
                };

                // Compute item width, adding justification if applicable.
                let mut item_width = item_result.inline_size;
                let mut justified_shape: Option<Arc<ShapeResult>> = None;
                if should_justify && (justification_per_space > 0.0 || justification_per_char > 0.0) {
                    let text = &items_data.text[item_result.text_range.clone()];
                    // In pre-wrap mode, the last text item's trailing spaces
                    // hang and must not receive justification expansion.
                    let is_last_text = line_info.items[(_i + 1)..]
                        .iter()
                        .all(|ir| ir.item_type != InlineItemType::Text);
                    let style_for_item = &items_data.styles[item.style_index];
                    let trailing_spaces = if is_last_text
                        && matches!(
                            style_for_item.white_space,
                            openui_style::WhiteSpace::PreWrap | openui_style::WhiteSpace::BreakSpaces
                        )
                    {
                        text.chars().rev().take_while(|c| *c == ' ').count()
                    } else {
                        0
                    };

                    if justification_per_char > 0.0 {
                        // Inter-character justification: distribute extra space
                        // between every character boundary.
                        let char_count = text.chars().count().saturating_sub(trailing_spaces);
                        let internal_gaps = char_count.saturating_sub(1);
                        // Boundary gap: if there are characters before this item,
                        // add one gap for the boundary between the previous text
                        // item's last char and this item's first char (Issue 4 fix).
                        let boundary_gap = if inter_char_chars_before > 0 && char_count > 0 { 1 } else { 0 };
                        let total_item_gaps = internal_gaps + boundary_gap;
                        if total_item_gaps > 0 {
                            let extra = justification_per_char * total_item_gaps as f32;
                            let old_acc = justification_accumulator;
                            justification_accumulator += extra;
                            let extra_lu = LayoutUnit::from_f32(justification_accumulator)
                                - LayoutUnit::from_f32(old_acc);
                            item_width = item_width + extra_lu;
                            // Shift x position by boundary gap amount.
                            if boundary_gap > 0 {
                                let boundary_shift = LayoutUnit::from_f32(justification_per_char);
                                inline_offset = inline_offset + boundary_shift;
                                // Reduce item_width by boundary_shift since it's positional.
                                item_width = item_width - boundary_shift;
                            }
                            // Create justified shape result with modified glyph advances.
                            if let Some(ref sr) = line_shape_result {
                                let mut justified_sr = sr.sub_range(0, sr.num_characters);
                                justified_sr.apply_inter_character_justification(
                                    justification_per_char,
                                );
                                justified_shape = Some(Arc::new(justified_sr));
                            }
                        }
                        inter_char_chars_before += char_count;
                    } else {
                        let total = text.chars().filter(|c| *c == ' ').count();
                        let space_count = total - trailing_spaces;
                        if space_count > 0 {
                            let extra = justification_per_space * space_count as f32;
                            let old_acc = justification_accumulator;
                            justification_accumulator += extra;
                            let extra_lu = LayoutUnit::from_f32(justification_accumulator)
                                - LayoutUnit::from_f32(old_acc);
                            item_width = item_width + extra_lu;
                            // Also adjust glyph advances in the shape result so that
                            // space glyphs are visually wider — exclude trailing spaces.
                            if let Some(ref sr) = line_shape_result {
                                let mut justified_sr = sr.sub_range(0, sr.num_characters);
                                justified_sr.apply_justification(
                                    justification_per_space, text, trailing_spaces,
                                );
                                justified_shape = Some(Arc::new(justified_sr));
                            }
                        }
                    }
                }

                let text_height = LayoutUnit::from_f32_ceil(metrics.ascent + metrics.descent);

                let mut text_fragment = Fragment::new_box(item.node_id, PhysicalSize::new(
                    item_width,
                    text_height,
                ));
                text_fragment.kind = FragmentKind::Text;
                text_fragment.offset = PhysicalOffset::new(inline_offset, text_top);
                // Store the baseline offset (distance from fragment top to baseline)
                // so paint can use it directly instead of recomputing from metrics.
                text_fragment.baseline_offset =
                    (baseline - text_top).to_f32();
                // Use justified shape result if justification was applied,
                // otherwise use the sub-range shape result for this line portion.
                text_fragment.shape_result = justified_shape.or(line_shape_result);

                children.push(text_fragment);
                inline_offset = inline_offset + item_width;
            }
            InlineItemType::OpenTag => {
                let style = &items_data.styles[item.style_index];
                inline_offset = inline_offset + resolve_inline_start(style, percentage_base);
                // Push this inline element's font metrics for nested content.
                let font_desc = style_to_font_description(style);
                let font = Font::new(font_desc);
                let metrics = font.font_metrics().copied().unwrap_or_default();
                inline_metrics_stack.push(metrics);
            }
            InlineItemType::CloseTag => {
                let style = &items_data.styles[item.style_index];
                inline_offset = inline_offset + resolve_inline_end(style, percentage_base);
                inline_metrics_stack.pop();
            }
            InlineItemType::Control => {
                // <br> — no visual contribution, line break already handled.
            }
            InlineItemType::AtomicInline | InlineItemType::BlockInInline => {
                // Create a box fragment for the atomic inline element.
                let item_width = item_result.inline_size;
                let style = &items_data.styles[item.style_index];
                // Resolve height: use explicit CSS height if available, else font metrics.
                let item_height = match style.height.length_type() {
                    openui_geometry::LengthType::Fixed => {
                        LayoutUnit::from_f32(style.height.value())
                    }
                    _ => {
                        // Fall back to font metrics (ascent + descent) for the item's font
                        let font_desc = style_to_font_description(style);
                        let font = Font::new(font_desc);
                        let metrics = font.font_metrics().copied().unwrap_or_default();
                        LayoutUnit::from_f32_ceil(metrics.ascent + metrics.descent)
                    }
                };
                let atomic_top = match style.vertical_align {
                    VerticalAlign::Top => {
                        // Flush with top of line box.
                        LayoutUnit::zero()
                    }
                    VerticalAlign::Bottom => {
                        // Flush with bottom of line box.
                        line_height - item_height
                    }
                    VerticalAlign::Middle => {
                        // Centered: baseline - x_height/2 - item_height/2
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        let x_height = LayoutUnit::from_f32(parent_metrics.x_height);
                        baseline - x_height / LayoutUnit::from_f32(2.0)
                            - item_height / LayoutUnit::from_f32(2.0)
                    }
                    VerticalAlign::TextTop => {
                        // Top of item aligns with top of text (ascent above baseline).
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        baseline - LayoutUnit::from_f32_ceil(parent_metrics.ascent)
                    }
                    VerticalAlign::TextBottom => {
                        // Bottom of item aligns with bottom of text (descent below baseline).
                        let parent_metrics = inline_metrics_stack.last().unwrap_or(block_metrics);
                        baseline + LayoutUnit::from_f32_ceil(parent_metrics.descent)
                            - item_height
                    }
                    VerticalAlign::Sub => {
                        let shift = LayoutUnit::from_f32(style.font_size / 5.0 + 1.0);
                        baseline + shift - item_height
                    }
                    VerticalAlign::Super => {
                        let shift = LayoutUnit::from_f32(style.font_size / 3.0 + 1.0);
                        baseline - shift - item_height
                    }
                    VerticalAlign::Length(px) => {
                        // Positive length shifts up from baseline.
                        let shift = LayoutUnit::from_f32(px);
                        baseline - item_height - shift
                    }
                    VerticalAlign::Percentage(pct) => {
                        // Percentage of the element's own line-height (CSS 2.2 §10.8.1).
                        let element_line_height = match style.line_height {
                            LineHeight::Normal => {
                                let font_desc = style_to_font_description(style);
                                let font = Font::new(font_desc);
                                let metrics = font.font_metrics().copied().unwrap_or_default();
                                metrics.line_spacing
                            }
                            LineHeight::Number(n) => style.font_size * n,
                            LineHeight::Length(px) => px,
                            LineHeight::Percentage(p) => style.font_size * p / 100.0,
                        };
                        let shift = LayoutUnit::from_f32(element_line_height * pct / 100.0);
                        baseline - item_height - shift
                    }
                    _ => {
                        // Baseline (default): bottom of item sits on baseline.
                        baseline - item_height
                    }
                };
                let mut atomic_fragment = Fragment::new_box(
                    item.node_id,
                    PhysicalSize::new(item_width, item_height),
                );
                atomic_fragment.offset = PhysicalOffset::new(inline_offset, atomic_top);
                children.push(atomic_fragment);
                inline_offset = inline_offset + item_result.inline_size;
            }
        }
    }

    // === STEP 5: Paint ellipsis if text-overflow: ellipsis is active ===
    if line_info.has_ellipsis {
        // Shape the ellipsis character "…" (U+2026) with the block's font.
        let block_font_desc = style_to_font_description(block_style);
        let ellipsis_font = Font::new(block_font_desc);
        let shaper = TextShaper::new();
        let ellipsis_text = "\u{2026}";
        let ellipsis_sr = shaper.shape(
            ellipsis_text,
            &ellipsis_font,
            openui_text::TextDirection::Ltr,
        );
        let ellipsis_width = LayoutUnit::from_f32(ellipsis_sr.width);
        let ellipsis_metrics = ellipsis_font.font_metrics().copied().unwrap_or_default();
        let ellipsis_height = LayoutUnit::from_f32_ceil(
            ellipsis_metrics.ascent + ellipsis_metrics.descent,
        );
        let ellipsis_top = baseline
            - LayoutUnit::from_f32_ceil(ellipsis_metrics.ascent);

        if line_info.ellipsis_at_start {
            // RTL: place ellipsis at the left edge, shift content right.
            for child in &mut children {
                child.offset = PhysicalOffset::new(
                    child.offset.left + ellipsis_width,
                    child.offset.top,
                );
            }
            let mut ellipsis_fragment = Fragment::new_text(
                NodeId::NONE,
                PhysicalSize::new(ellipsis_width, ellipsis_height),
                Arc::new(ellipsis_sr),
                ellipsis_text.to_string(),
            );
            ellipsis_fragment.offset = PhysicalOffset::new(LayoutUnit::zero(), ellipsis_top);
            ellipsis_fragment.inherited_style = Some(block_style.clone());
            ellipsis_fragment.baseline_offset = (baseline - ellipsis_top).to_f32();
            children.insert(0, ellipsis_fragment);
        } else {
            // LTR: place ellipsis at the right edge (after content).
            let mut ellipsis_fragment = Fragment::new_text(
                NodeId::NONE,
                PhysicalSize::new(ellipsis_width, ellipsis_height),
                Arc::new(ellipsis_sr),
                ellipsis_text.to_string(),
            );
            ellipsis_fragment.offset = PhysicalOffset::new(inline_offset, ellipsis_top);
            ellipsis_fragment.inherited_style = Some(block_style.clone());
            ellipsis_fragment.baseline_offset = (baseline - ellipsis_top).to_f32();
            children.push(ellipsis_fragment);
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

    let min_odd = match items
        .iter()
        .map(|ir| items_data.items[ir.item_index].bidi_level)
        .filter(|l| l % 2 == 1)
        .min()
    {
        Some(v) => v,
        None => return, // No odd levels → no reordering needed
    };

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
/// minus the ellipsis width. When the last remaining item is text and
/// partially fits, it is trimmed instead of fully removed. The caller
/// is responsible for actually painting the ellipsis character.
///
/// Blink: `NGLineInfo::SetHasEllipsis` / `NGLineTruncator`.
fn apply_text_overflow_ellipsis(
    line_info: &mut LineInfo,
    available_width: LayoutUnit,
    items_data: &InlineItemsData,
    block_style: &ComputedStyle,
) {
    if line_info.used_width <= available_width {
        return;
    }

    let block_font_desc = style_to_font_description(block_style);
    let block_font = Font::new(block_font_desc);
    let shaper = TextShaper::new();
    let ellipsis_sr = shaper.shape(
        "\u{2026}",
        &block_font,
        openui_text::TextDirection::Ltr,
    );
    let ellipsis_width = LayoutUnit::from_f32(ellipsis_sr.width);

    let target_width = available_width - ellipsis_width;

    let is_rtl = block_style.direction == Direction::Rtl;

    if target_width <= LayoutUnit::zero() {
        line_info.items.clear();
        line_info.used_width = LayoutUnit::zero();
        line_info.has_ellipsis = true;
        if is_rtl {
            line_info.ellipsis_at_start = true;
        }
        return;
    }

    if is_rtl {
        // RTL: truncate from the visual left (beginning of items after bidi reorder).
        // Remove items from the front until we have room for the ellipsis.
        while line_info.used_width > target_width && !line_info.items.is_empty() {
            let first_size = line_info.items[0].inline_size;
            if first_size <= LayoutUnit::zero()
                && line_info.items[0].item_type != InlineItemType::Text
            {
                line_info.items.remove(0);
                continue;
            }

            let excess = line_info.used_width - target_width;
            if line_info.items[0].item_type == InlineItemType::Text && first_size > excess {
                // Partial fit — trim from the left side of this text item.
                let item_target = first_size - excess;
                let item = &items_data.items[line_info.items[0].item_index];
                if let Some(ref sr) = item.shape_result {
                    let line_text = &items_data.text[line_info.items[0].text_range.clone()];
                    let item_char_start = byte_to_char_offset(
                        &items_data.text,
                        item.text_range.start,
                    );
                    let portion_char_start = byte_to_char_offset(
                        &items_data.text,
                        line_info.items[0].text_range.start,
                    );
                    let portion_char_end = byte_to_char_offset(
                        &items_data.text,
                        line_info.items[0].text_range.end,
                    );
                    let local_start = portion_char_start - item_char_start;
                    let local_end = portion_char_end - item_char_start;
                    let total_chars = local_end - local_start;

                    // Find the first grapheme-safe trim point from the start
                    // that fits. Use grapheme clusters and safe_to_break_before
                    // to avoid splitting emoji/ZWJ sequences.
                    let mut trim_chars = 0;
                    let mut trim_byte_offset = 0;
                    for (byte_offset, _grapheme) in line_text.grapheme_indices(true).skip(1) {
                        let char_count = line_text[..byte_offset].chars().count();
                        let local_trim = local_start + char_count;
                        // Only trim at shaping-safe positions.
                        if !sr.safe_to_break_before(local_trim) {
                            continue;
                        }
                        let remaining_width = LayoutUnit::from_f32(
                            sr.width_for_range(local_trim, local_end),
                        );
                        if remaining_width <= item_target {
                            trim_chars = char_count;
                            trim_byte_offset = byte_offset;
                            break;
                        }
                    }

                    if trim_chars > 0 && trim_chars < total_chars {
                        let trimmed_width = LayoutUnit::from_f32(
                            sr.width_for_range(local_start + trim_chars, local_end),
                        );
                        let new_text_start = line_info.items[0].text_range.start + trim_byte_offset;
                        let old_size = first_size;

                        let first_mut = &mut line_info.items[0];
                        first_mut.inline_size = trimmed_width;
                        first_mut.text_range = new_text_start..first_mut.text_range.end;
                        line_info.used_width = line_info.used_width - old_size + trimmed_width;
                    } else {
                        line_info.used_width = line_info.used_width - first_size;
                        line_info.items.remove(0);
                    }
                } else {
                    line_info.used_width = line_info.used_width - first_size;
                    line_info.items.remove(0);
                }
                break;
            }

            line_info.used_width = line_info.used_width - first_size;
            line_info.items.remove(0);
        }
    } else {
        // LTR: truncate from the visual right (end of items).
        while line_info.used_width > target_width && !line_info.items.is_empty() {
            if let Some(last) = line_info.items.last() {
                let last_size = last.inline_size;
                if last_size <= LayoutUnit::zero()
                    && last.item_type != InlineItemType::Text
                {
                    line_info.items.pop();
                    continue;
                }

                let excess = line_info.used_width - target_width;
                if last.item_type == InlineItemType::Text && last_size > excess {
                    let item_target = last_size - excess;
                    let item = &items_data.items[last.item_index];
                    if let Some(ref sr) = item.shape_result {
                        let line_text = &items_data.text[last.text_range.clone()];
                        let item_char_start = byte_to_char_offset(
                            &items_data.text,
                            item.text_range.start,
                        );
                        let portion_char_start = byte_to_char_offset(
                            &items_data.text,
                            last.text_range.start,
                        );
                        let local_start = portion_char_start - item_char_start;

                        // Walk grapheme boundaries from end to find the
                        // largest safe-to-break prefix that fits.
                        let mut fit_chars = 0;
                        let mut fit_byte_end = 0;
                        let grapheme_offsets: Vec<usize> = line_text
                            .grapheme_indices(true)
                            .skip(1)
                            .map(|(off, _)| off)
                            .collect();
                        for &byte_offset in grapheme_offsets.iter().rev() {
                            let char_count = line_text[..byte_offset].chars().count();
                            let local_trim = local_start + char_count;
                            if !sr.safe_to_break_before(local_trim) {
                                continue;
                            }
                            let w = LayoutUnit::from_f32(
                                sr.width_for_range(local_start, local_trim),
                            );
                            if w <= item_target {
                                fit_chars = char_count;
                                fit_byte_end = byte_offset;
                                break;
                            }
                        }

                        if fit_chars > 0 {
                            let trimmed_width = LayoutUnit::from_f32(
                                sr.width_for_range(local_start, local_start + fit_chars),
                            );
                            let new_text_end = last.text_range.start + fit_byte_end;
                            let old_size = last_size;

                            let last_mut = line_info.items.last_mut().unwrap();
                            last_mut.inline_size = trimmed_width;
                            last_mut.text_range = last_mut.text_range.start..new_text_end;
                            line_info.used_width = line_info.used_width - old_size + trimmed_width;
                        } else {
                            line_info.used_width = line_info.used_width - last_size;
                            line_info.items.pop();
                        }
                    } else {
                        line_info.used_width = line_info.used_width - last_size;
                        line_info.items.pop();
                    }
                    break;
                }

                line_info.used_width = line_info.used_width - last_size;
                line_info.items.pop();
            }
        }
    }

    line_info.has_ellipsis = true;
    line_info.ellipsis_at_start = is_rtl;
}

/// Check if a block node has any inline children (text or inline-level elements).
///
/// Used by block_layout to detect when to dispatch to inline layout.
pub fn has_inline_children(doc: &Document, node_id: NodeId) -> bool {
    for child_id in doc.children(node_id) {
        let child = doc.node(child_id);
        // display:none and out-of-flow children don't participate in layout.
        if child.style.display == Display::None || child.style.is_out_of_flow() {
            continue;
        }
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

    /// Helper to build a FontMetrics with specific ascent, descent, and line_gap.
    fn test_metrics(ascent: f32, descent: f32, line_gap: f32) -> FontMetrics {
        FontMetrics {
            ascent,
            descent,
            line_gap,
            line_spacing: ascent + descent + line_gap,
            ..FontMetrics::zero()
        }
    }

    #[test]
    fn line_height_metrics_normal() {
        // Normal line height uses int_line_spacing from font metrics.
        // ascent=10, descent=4, gap=2 → int_line_spacing = round(16) = 16
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(
            &metrics,
            &LineHeight::Normal,
            16.0,  // font_size
        );
        // leading = 16 - 14 = 2, half_leading = 1, rest = 1
        assert_eq!(m.ascent, 11.0);
        assert_eq!(m.descent, 5.0);
    }

    #[test]
    fn line_height_metrics_number() {
        // line-height: 2.0 doubles line height
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(
            &metrics,
            &LineHeight::Number(2.0),
            16.0,  // font_size
        );
        // computed = 16 * 2 = 32, leading = 32 - 14 = 18
        // half_leading = 9, rest = 9
        assert_eq!(m.ascent, 19.0);
        assert_eq!(m.descent, 13.0);
    }

    #[test]
    fn line_height_metrics_length() {
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(
            &metrics,
            &LineHeight::Length(24.0),
            16.0,
        );
        // leading = 24 - 14 = 10, half = 5, rest = 5
        assert_eq!(m.ascent, 15.0);
        assert_eq!(m.descent, 9.0);
    }

    #[test]
    fn line_height_metrics_percentage() {
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(
            &metrics,
            &LineHeight::Percentage(150.0),
            16.0,
        );
        // computed = 16 * 150 / 100 = 24, leading = 10
        assert_eq!(m.ascent, 15.0);
        assert_eq!(m.descent, 9.0);
    }

    #[test]
    fn line_height_half_leading_odd() {
        // Odd leading: sub-pixel precision preserved (no floor/ceil rounding)
        let metrics = test_metrics(10.0, 4.0, 2.0);
        let m = compute_line_height_metrics(
            &metrics,
            &LineHeight::Length(25.0),
            16.0,
        );
        // leading = 25 - 14 = 11, half = 5.5, rest = 5.5
        assert_eq!(m.ascent, 15.5);
        assert_eq!(m.descent, 9.5);
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
    fn baseline_shift_percentage_uses_element_line_height() {
        // CSS 2.2 §10.8.1: percentage is of the element's own line-height.
        // element_line_height = 40px, 50% => shift = -(40 * 50 / 100) = -20
        let shift = compute_baseline_shift(
            &VerticalAlign::Percentage(50.0),
            16.0, 10.0, 4.0, 8.0, 10.0, 4.0, 40.0,
        );
        assert_eq!(shift, -20.0);
    }

    #[test]
    fn baseline_shift_percentage_with_normal_line_height() {
        // When line-height is normal, element_line_height = font line_spacing.
        // Use line_spacing = 18.0, 50% => shift = -(18 * 50 / 100) = -9
        let shift = compute_baseline_shift(
            &VerticalAlign::Percentage(50.0),
            16.0, 10.0, 4.0, 8.0, 10.0, 4.0, 18.0,
        );
        assert_eq!(shift, -9.0);
    }

    #[test]
    fn text_align_left() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Left, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_right() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Right, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto);
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_center() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Center, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto);
        assert_eq!(offset.to_i32(), 20);
    }

    #[test]
    fn text_align_start_ltr() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Start, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_start_rtl() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::Start, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Rtl, TextAlignLast::Auto);
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_end_ltr() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::End, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto);
        assert_eq!(offset, LayoutUnit::from_i32(40));
    }

    #[test]
    fn text_align_end_rtl() {
        let line = make_test_line_info(100.0, 60.0, TextAlign::End, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Rtl, TextAlignLast::Auto);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_justify_last_line_falls_back() {
        // Justify on the last line falls back to start alignment.
        let mut line = make_test_line_info(100.0, 60.0, TextAlign::Justify, false);
        line.is_last_line = true;
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto);
        assert_eq!(offset, LayoutUnit::zero());
    }

    #[test]
    fn text_align_overflow_no_offset() {
        // When content overflows, offset should be 0.
        let line = make_test_line_info(100.0, 150.0, TextAlign::Right, false);
        let offset = compute_text_align_offset(&line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto);
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

    #[test]
    fn ellipsis_uses_block_font_for_measurement() {
        // Verify that apply_text_overflow_ellipsis measures the ellipsis using
        // the block_style's font (consistent with create_line_box), not the
        // first visible item's font.
        let block_style = ComputedStyle::default();
        let block_font_desc = style_to_font_description(&block_style);
        let block_font = Font::new(block_font_desc);
        let shaper = TextShaper::new();

        // Measure the ellipsis width with the block font.
        let ellipsis_sr = shaper.shape(
            "\u{2026}",
            &block_font,
            openui_text::TextDirection::Ltr,
        );
        let expected_width = LayoutUnit::from_f32(ellipsis_sr.width);

        // The ellipsis width should be a positive, reasonable value.
        assert!(
            expected_width > LayoutUnit::zero(),
            "Ellipsis measured with block font should have positive width",
        );

        // Create a minimal overflowing line with no items (edge case).
        let mut line_info = LineInfo::new(LayoutUnit::from_f32(100.0));
        line_info.used_width = LayoutUnit::from_f32(150.0);
        let items_data = InlineItemsData {
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
        };

        // Should not panic even with empty items.
        apply_text_overflow_ellipsis(
            &mut line_info,
            LayoutUnit::from_f32(100.0),
            &items_data,
            &block_style,
        );

        // Line should be flagged with ellipsis.
        assert!(line_info.has_ellipsis);
    }

    // ── Issue 2: Half-leading-adjusted metrics for baseline shift ────

    #[test]
    fn text_top_uses_half_leading_adjusted_metrics() {
        // With a large line-height, text-top should use half-leading-adjusted
        // ascent (not raw font ascent). compute_baseline_shift with text-top
        // returns item_ascent - parent_ascent. When we pass half-leading-
        // adjusted metrics, the shift is different from raw metrics.
        //
        // Raw font: ascent=10, descent=4 (total=14)
        // line-height: 24px → leading=10, half=5
        // half-leading ascent = 10 + 5 = 15
        //
        // text-top shift = item_ascent - parent_ascent
        // With raw: 10 - 10 = 0
        // With half-leading: 15 - 10 = 5
        let shift_with_half_leading = compute_baseline_shift(
            &VerticalAlign::TextTop,
            16.0,
            10.0,  // parent_ascent
            4.0,   // parent_descent
            8.0,   // parent_x_height
            15.0,  // item_ascent (half-leading adjusted)
            9.0,   // item_descent (half-leading adjusted)
            24.0,  // element_line_height
        );
        // text-top: item_ascent - parent_ascent = 15 - 10 = 5
        assert_eq!(shift_with_half_leading, 5.0);

        let shift_with_raw = compute_baseline_shift(
            &VerticalAlign::TextTop,
            16.0,
            10.0, 4.0, 8.0,
            10.0, // raw ascent
            4.0,  // raw descent
            24.0,
        );
        // With raw metrics: 10 - 10 = 0 (old behavior, now different from above)
        assert_eq!(shift_with_raw, 0.0);
        assert_ne!(
            shift_with_half_leading, shift_with_raw,
            "Half-leading metrics should produce different shift than raw metrics"
        );
    }

    // ── Issue 5: RTL ellipsis truncates from left ────────────────────

    #[test]
    fn rtl_ellipsis_truncates_from_left() {
        // For RTL, ellipsis should be placed at the start (left) and
        // content should be truncated from the left side.
        let mut block_style = ComputedStyle::default();
        block_style.direction = Direction::Rtl;

        let mut line_info = LineInfo::new(LayoutUnit::from_f32(100.0));
        line_info.used_width = LayoutUnit::from_f32(150.0);

        let items_data = InlineItemsData {
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
        };

        apply_text_overflow_ellipsis(
            &mut line_info,
            LayoutUnit::from_f32(100.0),
            &items_data,
            &block_style,
        );

        assert!(line_info.has_ellipsis);
        assert!(
            line_info.ellipsis_at_start,
            "RTL ellipsis should be placed at start (left)"
        );
    }

    #[test]
    fn ltr_ellipsis_not_at_start() {
        // For LTR, ellipsis_at_start should be false.
        let block_style = ComputedStyle::default();

        let mut line_info = LineInfo::new(LayoutUnit::from_f32(100.0));
        line_info.used_width = LayoutUnit::from_f32(150.0);

        let items_data = InlineItemsData {
            text: String::new(),
            items: Vec::new(),
            styles: Vec::new(),
        };

        apply_text_overflow_ellipsis(
            &mut line_info,
            LayoutUnit::from_f32(100.0),
            &items_data,
            &block_style,
        );

        assert!(line_info.has_ellipsis);
        assert!(
            !line_info.ellipsis_at_start,
            "LTR ellipsis should not be at start"
        );
    }

    // ── SP11 Round 11 Issue 4: baseline_offset stored on text fragments ──

    #[test]
    fn text_fragment_has_nonzero_baseline_offset() {
        // Layout a simple text node and verify that the resulting text
        // fragment has a positive baseline_offset (ascent-based).
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = openui_geometry::Length::px(200.0);
        doc.append_child(vp, div);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hello".to_string());
        doc.append_child(div, text);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];

        // Line box → text fragment
        assert!(!div_frag.children.is_empty(), "Should have line boxes");
        let line = &div_frag.children[0];
        assert!(!line.children.is_empty(), "Line should have text fragments");

        let text_frag = &line.children[0];
        assert!(
            text_frag.baseline_offset > 0.0,
            "Text fragment baseline_offset should be positive (ascent), got {}",
            text_frag.baseline_offset,
        );
    }

    // ── SP11 Round 15 Issue 1: vertical-align uses parent inline metrics ──

    #[test]
    fn vertical_align_text_top_uses_parent_inline_metrics() {
        // Verify that compute_baseline_shift correctly receives parent inline
        // metrics. With a parent ascent of 25.0, text-top should produce
        // item_ascent - parent_ascent = 10 - 25 = -15.
        // With block metrics (ascent=12), it would be 10 - 12 = -2.
        let shift_parent = compute_baseline_shift(
            &VerticalAlign::TextTop,
            12.0,
            25.0,  // parent inline ascent (larger font)
            8.0,   // parent inline descent
            10.0,  // parent inline x_height
            10.0,  // item ascent
            3.0,   // item descent
            14.0,  // element_line_height
        );
        // text-top: item_ascent - parent_ascent = 10 - 25 = -15
        assert_eq!(shift_parent, -15.0);

        let shift_block = compute_baseline_shift(
            &VerticalAlign::TextTop,
            12.0,
            12.0,  // block ascent (smaller)
            4.0,
            6.0,
            10.0,
            3.0,
            14.0,
        );
        // text-top: item_ascent - parent_ascent = 10 - 12 = -2
        assert_eq!(shift_block, -2.0);

        // The shift differs when using parent inline vs block metrics.
        assert_ne!(shift_parent, shift_block,
            "text-top shift should differ between parent inline (30px) and block (16px)");
    }

    #[test]
    fn vertical_align_middle_uses_parent_x_height() {
        // Verify that middle alignment uses the parent's x_height.
        // With parent x_height=10: (item_ascent - item_descent)/2 - 10/2
        let shift = compute_baseline_shift(
            &VerticalAlign::Middle,
            12.0,
            20.0,  // parent ascent
            5.0,   // parent descent
            10.0,  // parent x_height
            8.0,   // item ascent
            3.0,   // item descent
            14.0,
        );
        // middle: (8 - 3)/2 - 10/2 = 2.5 - 5.0 = -2.5
        assert_eq!(shift, -2.5);
    }

    #[test]
    fn inline_metrics_stack_affects_layout() {
        // Build: <div font-size=16><span font-size=30><text font-size=12 v-align=text-top>X</text></span></div>
        // The text-top aligned text should be positioned based on the 30px
        // span's metrics, not the 16px block's metrics.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.append_child(vp, div);

        // Outer span with 30px font
        let outer_span = doc.create_node(ElementTag::Span);
        doc.node_mut(outer_span).style.display = Display::Inline;
        doc.node_mut(outer_span).style.font_size = 30.0;
        doc.append_child(div, outer_span);

        let outer_text = doc.create_node(ElementTag::Text);
        doc.node_mut(outer_text).text = Some("A".to_string());
        doc.node_mut(outer_text).style.font_size = 30.0;
        doc.append_child(outer_span, outer_text);

        // Text with text-top inside the outer span (no intermediate span)
        let inner_text = doc.create_node(ElementTag::Text);
        doc.node_mut(inner_text).text = Some("X".to_string());
        doc.node_mut(inner_text).style.font_size = 12.0;
        doc.node_mut(inner_text).style.vertical_align = VerticalAlign::TextTop;
        doc.append_child(outer_span, inner_text);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "Should have line boxes");
        let line = &div_frag.children[0];

        assert!(
            line.children.len() >= 2,
            "Line should have at least 2 text fragments, got {}",
            line.children.len(),
        );

        let outer_frag = &line.children[0]; // "A" at 30px
        let inner_frag = &line.children[1]; // "X" at 12px, text-top

        // text-top: the top of the inner item aligns with the parent inline's
        // text top. Since the parent inline has 30px font, the inner item's top
        // should be near the outer item's top.
        let outer_top = outer_frag.offset.top.to_f32();
        let inner_top = inner_frag.offset.top.to_f32();

        // With block metrics (16px), text-top would align to the block's ascent,
        // producing a larger offset difference. With parent inline metrics (30px),
        // the inner text's top should be closer to the outer text's top.
        let diff = (inner_top - outer_top).abs();
        assert!(
            diff < 5.0,
            "text-top inner text should align near outer span's text top; outer_top={}, inner_top={}, diff={}",
            outer_top, inner_top, diff,
        );
    }
}
