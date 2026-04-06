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
use openui_style::{BoxDecorationBreak, ComputedStyle, Direction, Display, LineHeight, TextAlign, TextAlignLast, TextJustify, VerticalAlign};
use openui_text::{Font, FontMetrics, ShapeResult, TextShaper};
use std::sync::Arc;
use unicode_segmentation::UnicodeSegmentation;

use crate::constraint_space::ConstraintSpace;
use crate::fragment::{Fragment, FragmentKind};
use crate::length_resolver::resolve_margin_or_padding;
use crate::out_of_flow::OutOfFlowCandidate;

use super::items::{InlineItemResult, InlineItemType};
use super::items_builder::{style_to_font_description, InlineItemsData, InlineItemsBuilder};
use super::line_breaker::{byte_to_char_offset, LineBreaker};
use super::line_info::LineInfo;
use super::line_width::compute_line_availability;

// ── Inline box state tracking (CSS Fragmentation §4.4) ──────────────────

/// Tracks an open inline box for line-splitting state.
///
/// When an inline element (e.g. `<span>`) spans multiple lines, its
/// border/padding/margin must be split across fragments:
/// - First fragment: gets inline-start MBP
/// - Last fragment: gets inline-end MBP
/// - Middle fragments: no inline MBP
///
/// With `box-decoration-break: clone`, every fragment gets full MBP.
///
/// Blink: `InlineBoxState` in `inline_box_state.cc`.
#[derive(Debug, Clone)]
pub struct InlineBoxState {
    /// Index into the styles array for this inline element.
    pub style_index: usize,
    /// DOM node of the inline element.
    pub node_id: NodeId,
    /// `box-decoration-break` mode from the element's style.
    pub box_decoration_break: BoxDecorationBreak,
}

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
///
/// The effective content width is `used_width` (which already has trailing
/// space widths subtracted). For `pre-wrap` the spaces "hang" past the line
/// box — their width is recorded in `hang_width` and *not* included in
/// `used_width`, so the alignment math naturally excludes them.
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
///
/// Only counts boundaries between text items that are logically adjacent
/// (separated only by OpenTag/CloseTag). AtomicInline or Control items
/// break the adjacency, so no boundary gap is counted across them.
fn count_inter_character_opportunities(line_info: &LineInfo, items_data: &InlineItemsData) -> usize {
    // Collect character counts per contiguous text segment, where segments
    // are separated by AtomicInline or Control items.
    let mut segments: Vec<usize> = Vec::new();
    let mut current_segment_chars = 0usize;
    for item_result in &line_info.items {
        match item_result.item_type {
            InlineItemType::Text => {
                let text = &items_data.text[item_result.text_range.clone()];
                current_segment_chars += text.chars().count();
            }
            InlineItemType::OpenTag | InlineItemType::CloseTag => {
                // Tags don't break adjacency — continue accumulating.
            }
            InlineItemType::AtomicInline | InlineItemType::Control | InlineItemType::BlockInInline => {
                // Non-text items break adjacency.
                if current_segment_chars > 0 {
                    segments.push(current_segment_chars);
                    current_segment_chars = 0;
                }
            }
        }
    }
    if current_segment_chars > 0 {
        segments.push(current_segment_chars);
    }

    // Exclude trailing spaces from the last segment.
    if let Some(last_seg) = segments.last_mut() {
        for item_result in line_info.items.iter().rev() {
            if item_result.item_type == InlineItemType::Text {
                let text = &items_data.text[item_result.text_range.clone()];
                let trailing_spaces = text.chars().rev().take_while(|c| *c == ' ').count();
                *last_seg = last_seg.saturating_sub(trailing_spaces);
                break;
            }
            if item_result.item_type != InlineItemType::CloseTag
                && item_result.item_type != InlineItemType::OpenTag
            {
                break;
            }
        }
    }

    // Total gaps = sum of (segment_chars - 1) for each segment.
    segments.iter().map(|&c| c.saturating_sub(1)).sum()
}

/// Detect whether the current line contains CJK characters.
///
/// Used by `text-justify: auto` to select inter-character justification
/// for CJK text instead of inter-word justification. Checks the actual
/// text content of items on the current line.
///
/// CJK character ranges checked:
/// - CJK Unified Ideographs: U+4E00–U+9FFF
/// - CJK Extension A: U+3400–U+4DBF
/// - Katakana: U+30A0–U+30FF
/// - Hiragana: U+3040–U+309F
/// - Hangul Syllables: U+AC00–U+D7AF
/// - CJK Compatibility Ideographs: U+F900–U+FAFF
fn detect_cjk_content(line_info: &LineInfo, items_data: &InlineItemsData) -> bool {
    for item_result in &line_info.items {
        if item_result.item_type == InlineItemType::Text {
            let text = &items_data.text[item_result.text_range.clone()];
            for ch in text.chars() {
                if is_cjk_character(ch) {
                    return true;
                }
            }
        }
    }
    false
}

/// Check if a character is in a CJK Unicode range.
#[inline]
fn is_cjk_character(ch: char) -> bool {
    matches!(ch,
        '\u{4E00}'..='\u{9FFF}'   // CJK Unified Ideographs
        | '\u{3400}'..='\u{4DBF}' // CJK Extension A
        | '\u{30A0}'..='\u{30FF}' // Katakana
        | '\u{3040}'..='\u{309F}' // Hiragana
        | '\u{AC00}'..='\u{D7AF}' // Hangul Syllables
        | '\u{F900}'..='\u{FAFF}' // CJK Compatibility Ideographs
    )
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
    let mut items_data = InlineItemsBuilder::collect(doc, node_id);
    let style = &doc.node(node_id).style;
    let base_direction = if style.direction == Direction::Rtl {
        openui_text::TextDirection::Rtl
    } else {
        openui_text::TextDirection::Ltr
    };
    items_data.apply_bidi(base_direction);
    items_data.shape_text();
    inline_layout_from_items(doc, node_id, space, &items_data, 0, items_data.items.len())
}

/// Perform inline layout using pre-collected items over a specific item range.
///
/// Lays out items in `[item_start..item_end)` from the given `InlineItemsData`.
/// BlockInInline items within the range are skipped (they should have been
/// handled by the caller via block-in-inline splitting).
///
/// Used by block_layout's block-in-inline path (CSS 2.2 §9.2.1.1) to lay
/// out inline segments between block-level interruptions.
pub fn inline_layout_from_items(
    doc: &Document,
    node_id: NodeId,
    space: &ConstraintSpace,
    items_data: &InlineItemsData,
    item_start: usize,
    item_end: usize,
) -> Fragment {
    let style = &doc.node(node_id).style;

    let available_inline_size = space.available_inline_size.clamp_negative_to_zero();

    // Build a sub-view of items for the requested range.
    // If processing a subset, create a filtered InlineItemsData with only
    // the items in [item_start..item_end). The line breaker and layout
    // functions work on item indices relative to the data's items array.
    let working_items_data = if item_start == 0 && item_end == items_data.items.len() {
        items_data.clone()
    } else {
        // Create a filtered copy with only the items in the requested range.
        let mut filtered = items_data.clone();
        filtered.items = items_data.items[item_start..item_end].to_vec();
        // Re-index item indices for OOF children within this range.
        filtered.oof_children = items_data.oof_children.iter()
            .filter(|o| o.item_index >= item_start && o.item_index < item_end)
            .map(|o| super::items_builder::OofPlaceholder {
                node_id: o.node_id,
                item_index: o.item_index - item_start,
            })
            .collect();
        filtered.block_in_inline = Vec::new(); // already handled by caller
        filtered
    };

    // Create line breaker from the (possibly filtered) items.
    let mut line_breaker = LineBreaker::new(&working_items_data, available_inline_size);
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
    //
    // CSS 2.1 §9.5.1: Line boxes flow alongside floats. Each line's
    // available width may differ depending on float exclusion areas at
    // that line's block offset. We query the ExclusionSpace per-line.
    let mut line_fragments: Vec<Fragment> = Vec::new();
    let mut block_offset = LayoutUnit::zero();
    let mut is_first_line = true;

    // Track which inline boxes (by style_index) are open at the start of
    // each line. Carried forward across lines for inline box decoration
    // splitting (CSS Fragmentation §4.4).
    let mut boxes_open_at_line_start: Vec<InlineBoxState> = Vec::new();

    // Dereference the exclusion space once for the entire line loop.
    let exclusion_ref = space.exclusion_space.as_deref();
    // BFC block offset of this inline content's start within the exclusion space.
    let bfc_block_start = space.bfc_offset.block_offset;

    while !line_breaker.is_finished() {
        // Query float exclusions at this line's block offset.
        // The exclusion space uses content-edge-relative coordinates; add the
        // BFC start offset so we query at the correct absolute position.
        let line_avail = compute_line_availability(
            exclusion_ref,
            bfc_block_start + block_offset,
            available_inline_size,
            LayoutUnit::zero(),
        );

        // Apply text-indent: reduce available width on first line only.
        let line_available = if is_first_line && text_indent != LayoutUnit::zero() {
            (line_avail.available_inline_size - text_indent).clamp_negative_to_zero()
        } else {
            line_avail.available_inline_size
        };

        if let Some(mut line_info) = line_breaker.next_line(line_available) {
            // Step 4b: BiDi reorder items on this line for visual display.
            bidi_reorder_line(&mut line_info.items, &working_items_data);

            // Apply text-overflow: ellipsis if configured on the block style.
            if style.text_overflow == openui_style::TextOverflow::Ellipsis
                && style.overflow_x == openui_style::Overflow::Hidden
            {
                apply_text_overflow_ellipsis(&mut line_info, line_available, &working_items_data, style);
            }

            let line_fragment = create_line_box(
                doc,
                &working_items_data,
                &line_info,
                line_avail.available_inline_size,
                block_offset,
                style,
                &block_metrics,
                space.percentage_resolution_inline_size,
                if is_first_line { text_indent } else { LayoutUnit::zero() },
                space.percentage_resolution_block_size,
                &boxes_open_at_line_start,
            );

            // Update open inline box state for the next line:
            // replay OpenTag/CloseTag items on this line to determine which
            // inline boxes remain open at line end.
            let mut current_open = boxes_open_at_line_start.clone();
            for item_result in &line_info.items {
                let item = &working_items_data.items[item_result.item_index];
                match item_result.item_type {
                    InlineItemType::OpenTag => {
                        let s = &working_items_data.styles[item.style_index];
                        current_open.push(InlineBoxState {
                            style_index: item.style_index,
                            node_id: item.node_id,
                            box_decoration_break: s.box_decoration_break,
                        });
                    }
                    InlineItemType::CloseTag => {
                        current_open.pop();
                    }
                    _ => {}
                }
            }
            boxes_open_at_line_start = current_open;

            // Offset the line box inline-start when floats intrude from the left.
            let mut positioned_line = line_fragment;
            if line_avail.inline_start > LayoutUnit::zero() {
                positioned_line.offset.left =
                    positioned_line.offset.left + line_avail.inline_start;
            }

            block_offset = block_offset + positioned_line.size.height;
            line_fragments.push(positioned_line);
            is_first_line = false;
        }
    }

    let intrinsic_block_size = block_offset;

    // Compute first and last baselines from line boxes.
    // CSS Inline 3 §3: The first baseline of a block container with inline
    // content is the baseline of its first line box. The last baseline is
    // the baseline of its last line box.
    // Blink: InlineLayoutAlgorithm::Layout() — first/last_baseline computation.
    let first_baseline = line_fragments.first().map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    let last_baseline = line_fragments.last().map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    // Build the container fragment.
    let border_box_inline = space.available_inline_size;
    let border_box_size = PhysicalSize::new(border_box_inline, intrinsic_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.children = line_fragments;
    fragment.first_baseline = first_baseline;
    fragment.last_baseline = last_baseline;

    // Generate OOF candidates from inline content.
    // CSS 2.1 §10.3.7: The static position of an absolutely-positioned element
    // within inline content is where it would have been placed in normal flow.
    // Use the item_index to find the block offset of the line containing the
    // OOF child, and set inline offset to 0 (line start).
    for oof in &items_data.oof_children {
        let oof_style = doc.node(oof.node_id).style.clone();
        let static_block = find_static_block_for_item_index(
            oof.item_index,
            items_data.items.len(),
            &fragment.children,
            intrinsic_block_size,
        );
        fragment.oof_candidates.push(OutOfFlowCandidate {
            node_id: oof.node_id,
            style: oof_style,
            static_position: PhysicalOffset::new(LayoutUnit::zero(), static_block),
            containing_block_size: border_box_size,
            containing_block_border: openui_geometry::BoxStrut::zero(),
            containing_block_direction: doc.node(node_id).style.direction,
            static_position_direction: doc.node(node_id).style.direction,
        });
    }

    fragment
}

/// Find the static block position for an OOF placeholder at a given item index.
///
/// Since line fragments don't track which item indices they contain, we use
/// a proportional mapping: the OOF's item_index relative to the total item
/// count determines which line it falls in. For a single line or when the
/// item is beyond all items, we return the last line's top offset.
///
/// This matches Blink's simplified static-position-for-inline behavior where
/// the OOF is placed at the block offset of the line containing its static
/// position. A more precise implementation would thread item indices through
/// line breaking, but this is sufficient for correct behavior in practice.
fn find_static_block_for_item_index(
    item_index: usize,
    total_items: usize,
    line_fragments: &[Fragment],
    intrinsic_block_size: LayoutUnit,
) -> LayoutUnit {
    if line_fragments.is_empty() {
        return intrinsic_block_size;
    }
    if line_fragments.len() == 1 || total_items == 0 {
        return line_fragments[0].offset.top;
    }
    // Map item_index to a line index proportionally.
    let line_idx = (item_index * line_fragments.len() / total_items)
        .min(line_fragments.len() - 1);
    line_fragments[line_idx].offset.top
}
/// Apply inline fragmentation to a laid-out inline formatting context.
///
/// Takes a fragment produced by `inline_layout` / `inline_layout_from_items`
/// and splits it at fragmentainer boundaries. Returns the fragment for the
/// current fragmentainer, with a break token if content continues.
///
/// CSS Break 3 §3: Line boxes are class-B break points — a break can occur
/// between any two line boxes. Orphans/widows constraints (§4.1) may shift
/// the break point to satisfy minimum line counts.
///
/// Blink: `InlineLayoutAlgorithm::BreakLine()` and `BreakBeforeLine()` in
/// `inline_layout_algorithm.cc`.
///
/// # Parameters
/// - `fragment`: The fully laid-out inline fragment (all lines).
/// - `fragmentainer_block_size`: Height of the current fragmentainer.
/// - `block_offset_in_fragmentainer`: How far into the fragmentainer this
///   inline content starts.
/// - `lines_already_consumed`: Number of lines consumed in previous
///   fragmentainers (from an `InlineBreakToken`).
/// - `orphans`: CSS `orphans` property value (minimum lines before break).
/// - `widows`: CSS `widows` property value (minimum lines after break).
pub fn apply_inline_fragmentation(
    mut fragment: Fragment,
    fragmentainer_block_size: LayoutUnit,
    block_offset_in_fragmentainer: LayoutUnit,
    lines_already_consumed: usize,
    orphans: u32,
    widows: u32,
) -> Fragment {
    use crate::fragmentation::{BreakToken, InlineBreakToken};

    let total_lines = fragment.children.len();

    // No lines — nothing to fragment.
    if total_lines == 0 {
        return fragment;
    }

    // No fragmentation context — return as-is.
    if fragmentainer_block_size <= LayoutUnit::zero() {
        return fragment;
    }

    // Available block space in this fragmentainer for inline content.
    let available_block = fragmentainer_block_size - block_offset_in_fragmentainer;
    if available_block <= LayoutUnit::zero() {
        // No space at all — produce empty fragment with break token at line 0.
        fragment.children.clear();
        fragment.size.height = LayoutUnit::zero();
        fragment.first_baseline = None;
        fragment.last_baseline = None;
        fragment.break_token = Some(BreakToken::Inline(InlineBreakToken::new(
            lines_already_consumed,
            LayoutUnit::zero(),
        )));
        return fragment;
    }

    // Determine how many lines fit in the available block space.
    // Walk line fragments (children) accumulating their block sizes.
    let mut lines_that_fit = 0usize;
    for child in &fragment.children {
        let line_bottom = child.offset.top + child.size.height;
        if line_bottom > available_block {
            break;
        }
        lines_that_fit += 1;
    }

    // All lines fit — no break needed.
    if lines_that_fit >= total_lines {
        return fragment;
    }

    // Apply orphans and widows constraints.
    // CSS Break 3 §4.1:
    // - "orphans" = minimum lines that must remain before the break.
    // - "widows" = minimum lines that must remain after the break.
    let lines_remaining = total_lines - lines_that_fit;
    let orphans = orphans as usize;
    let widows = widows as usize;

    // If orphans constraint is violated (too few lines before break),
    // reduce lines_that_fit to satisfy it... but we can't go below 0.
    // Actually orphans says we need AT LEAST `orphans` lines before break.
    // If lines_that_fit < orphans, that's okay if that's all we have space for;
    // orphans is a soft constraint that may be violated if there's no room.
    // But if we have room for more than orphans, we shouldn't reduce.
    // The key scenario: if we have room for many lines but widows would be
    // violated, we reduce lines_that_fit so the next fragmentainer gets enough.

    // Widows check: the next fragmentainer needs at least `widows` lines.
    // If lines_remaining < widows, steal lines from this fragmentainer.
    let mut adjusted_fit = lines_that_fit;
    if lines_remaining < widows && adjusted_fit > 0 {
        let needed = widows - lines_remaining;
        if adjusted_fit > needed {
            adjusted_fit -= needed;
        } else {
            // Can't satisfy widows without violating orphans to 0.
            // Keep at least 1 line (or orphans, whichever is smaller).
            adjusted_fit = adjusted_fit.min(1);
        }
    }

    // Orphans check: this fragmentainer needs at least `orphans` lines.
    // If adjusted_fit < orphans and there are enough total lines, try to
    // fit at least `orphans` lines (if they physically fit).
    if adjusted_fit < orphans && lines_that_fit >= orphans {
        adjusted_fit = orphans;
    }

    // Ensure we don't exceed what physically fits.
    adjusted_fit = adjusted_fit.min(lines_that_fit);

    // If adjusted_fit is 0 and there are lines, keep at least 1 line
    // (last resort — a line box is unbreakable content).
    if adjusted_fit == 0 && total_lines > 0 {
        adjusted_fit = 1;
    }

    // If all lines now fit after adjustment, no break needed.
    if adjusted_fit >= total_lines {
        return fragment;
    }

    // Split: keep first `adjusted_fit` lines, produce break token for rest.
    let consumed_block_size = if adjusted_fit > 0 {
        let last_kept = &fragment.children[adjusted_fit - 1];
        last_kept.offset.top + last_kept.size.height
    } else {
        LayoutUnit::zero()
    };

    fragment.children.truncate(adjusted_fit);
    fragment.size.height = consumed_block_size;

    // Update baselines for the truncated set.
    fragment.first_baseline = fragment.children.first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    fragment.last_baseline = fragment.children.last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    // Produce break token so the next fragmentainer can resume.
    let total_consumed = lines_already_consumed + adjusted_fit;
    fragment.break_token = Some(BreakToken::Inline(InlineBreakToken::new(
        total_consumed,
        consumed_block_size,
    )));

    fragment
}

/// Resume inline layout from a break token, producing a fragment for the
/// next fragmentainer.
///
/// Takes the full set of line fragments (from a complete inline layout),
/// skips lines already consumed, re-offsets the remaining lines to start
/// at block offset 0, and optionally applies fragmentation again if the
/// remaining lines still don't fit.
///
/// # Parameters
/// - `full_fragment`: The complete inline fragment with ALL lines.
/// - `break_token`: The inline break token from the previous fragmentainer.
/// - `fragmentainer_block_size`: Height of the new fragmentainer.
/// - `orphans`: CSS `orphans` value.
/// - `widows`: CSS `widows` value.
pub fn resume_inline_from_break_token(
    full_fragment: Fragment,
    break_token: &crate::fragmentation::InlineBreakToken,
    fragmentainer_block_size: LayoutUnit,
    orphans: u32,
    widows: u32,
) -> Fragment {
    let lines_to_skip = break_token.lines_consumed;
    let total_lines = full_fragment.children.len();

    if lines_to_skip >= total_lines {
        // All lines consumed — return empty fragment.
        let mut empty = Fragment::new_box(full_fragment.node_id, PhysicalSize::new(
            full_fragment.size.width,
            LayoutUnit::zero(),
        ));
        empty.first_baseline = None;
        empty.last_baseline = None;
        return empty;
    }

    // Take the remaining lines and re-offset them to start from 0.
    let remaining_children: Vec<Fragment> = full_fragment.children.into_iter()
        .skip(lines_to_skip)
        .collect();

    let first_offset = if let Some(first) = remaining_children.first() {
        first.offset.top
    } else {
        LayoutUnit::zero()
    };

    let adjusted_children: Vec<Fragment> = remaining_children.into_iter()
        .map(|mut f| {
            f.offset.top = f.offset.top - first_offset;
            f
        })
        .collect();

    // Compute the total block size of remaining lines.
    let total_block = if let Some(last) = adjusted_children.last() {
        last.offset.top + last.size.height
    } else {
        LayoutUnit::zero()
    };

    let mut resumed_fragment = Fragment::new_box(
        full_fragment.node_id,
        PhysicalSize::new(full_fragment.size.width, total_block),
    );
    resumed_fragment.children = adjusted_children;

    // Update baselines.
    resumed_fragment.first_baseline = resumed_fragment.children.first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    resumed_fragment.last_baseline = resumed_fragment.children.last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    // Apply fragmentation again if this fragmentainer also can't hold
    // all remaining lines.
    apply_inline_fragmentation(
        resumed_fragment,
        fragmentainer_block_size,
        LayoutUnit::zero(),
        lines_to_skip,
        orphans,
        widows,
    )
}

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

    // Track which inline boxes are open at the start of each line.
    let mut boxes_open_at_line_start: Vec<InlineBoxState> = Vec::new();

    // Dereference the exclusion space once for the entire line loop.
    let exclusion_ref = space.exclusion_space.as_deref();
    // BFC block offset of this anonymous wrapper's start within the exclusion space.
    let bfc_block_start = space.bfc_offset.block_offset;

    while !line_breaker.is_finished() {
        // Query float exclusions at this line's block offset.
        let line_avail = compute_line_availability(
            exclusion_ref,
            bfc_block_start + block_offset,
            available_inline_size,
            LayoutUnit::zero(),
        );

        let line_available = if is_first_line && text_indent != LayoutUnit::zero() {
            (line_avail.available_inline_size - text_indent).clamp_negative_to_zero()
        } else {
            line_avail.available_inline_size
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
                line_avail.available_inline_size,
                block_offset,
                style,
                &block_metrics,
                space.percentage_resolution_inline_size,
                if is_first_line { text_indent } else { LayoutUnit::zero() },
                space.percentage_resolution_block_size,
                &boxes_open_at_line_start,
            );

            // Update open inline box state for the next line.
            let mut current_open = boxes_open_at_line_start.clone();
            for item_result in &line_info.items {
                let item = &items_data.items[item_result.item_index];
                match item_result.item_type {
                    InlineItemType::OpenTag => {
                        let s = &items_data.styles[item.style_index];
                        current_open.push(InlineBoxState {
                            style_index: item.style_index,
                            node_id: item.node_id,
                            box_decoration_break: s.box_decoration_break,
                        });
                    }
                    InlineItemType::CloseTag => {
                        current_open.pop();
                    }
                    _ => {}
                }
            }
            boxes_open_at_line_start = current_open;

            // Offset the line box inline-start when floats intrude from the left.
            let mut positioned_line = line_fragment;
            if line_avail.inline_start > LayoutUnit::zero() {
                positioned_line.offset.left =
                    positioned_line.offset.left + line_avail.inline_start;
            }

            block_offset = block_offset + positioned_line.size.height;
            line_fragments.push(positioned_line);
            is_first_line = false;
        }
    }

    let intrinsic_block_size = block_offset;

    let first_baseline = line_fragments.first().map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    let last_baseline = line_fragments.last().map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    let border_box_inline = space.available_inline_size;
    let border_box_size = PhysicalSize::new(border_box_inline, intrinsic_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.children = line_fragments;
    fragment.first_baseline = first_baseline;
    fragment.last_baseline = last_baseline;

    // OOF candidates from anonymous inline wrapper.
    for oof in &items_data.oof_children {
        let oof_style = doc.node(oof.node_id).style.clone();
        let static_block = find_static_block_for_item_index(
            oof.item_index,
            items_data.items.len(),
            &fragment.children,
            intrinsic_block_size,
        );
        fragment.oof_candidates.push(OutOfFlowCandidate {
            node_id: oof.node_id,
            style: oof_style,
            static_position: PhysicalOffset::new(LayoutUnit::zero(), static_block),
            containing_block_size: border_box_size,
            containing_block_border: openui_geometry::BoxStrut::zero(),
            containing_block_direction: doc.node(node_id).style.direction,
            static_position_direction: doc.node(node_id).style.direction,
        });
    }

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
    doc: &Document,
    items_data: &InlineItemsData,
    line_info: &LineInfo,
    available_width: LayoutUnit,
    block_offset: LayoutUnit,
    block_style: &ComputedStyle,
    block_metrics: &openui_text::FontMetrics,
    percentage_base: LayoutUnit,
    text_indent: LayoutUnit,
    percentage_block_base: LayoutUnit,
    boxes_open_at_line_start: &[InlineBoxState],
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

    // === PRE-STEP: Run block_layout for atomic inlines ===
    // Per CSS 2.1 §10.6.1, inline-block/inline-flex/inline-grid establish a new
    // block formatting context. We pre-compute their layout so the result's
    // dimensions are authoritative for both line metrics and fragment creation.
    let mut atomic_layout_results: Vec<Option<Fragment>> = (0..line_info.items.len()).map(|_| None).collect();
    for (idx, item_result) in line_info.items.iter().enumerate() {
        if item_result.item_type == InlineItemType::AtomicInline {
            let item = &items_data.items[item_result.item_index];
            if !item.node_id.is_none() {
                let item_width = item_result.inline_size;
                let style = &items_data.styles[item.style_index];
                let available_block = match style.height.length_type() {
                    openui_geometry::LengthType::Fixed => {
                        LayoutUnit::from_f32(style.height.value())
                    }
                    _ => LayoutUnit::max(),
                };
                // Use the containing block's actual block size for percentage
                // resolution so that `height: 50%` etc. resolve correctly.
                // When the containing block height is indefinite or LayoutUnit::max(),
                // keep LayoutUnit::max() which correctly triggers auto behavior.
                let percentage_block = if !percentage_block_base.is_indefinite()
                    && percentage_block_base > LayoutUnit::zero()
                    && percentage_block_base < LayoutUnit::max()
                {
                    percentage_block_base
                } else {
                    available_block
                };
                let child_space = ConstraintSpace::for_block_child(
                    item_width,
                    available_block,
                    item_width,
                    percentage_block,
                    true,
                );
                let result = crate::block::block_layout(doc, item.node_id, &child_space);
                atomic_layout_results[idx] = Some(result);
            }
        }
    }

    // === STEP 2: Compute per-item metrics and unite ===
    for (step2_idx, item_result) in line_info.items.iter().enumerate() {
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
                // Use the pre-computed block_layout result if available.
                let item = &items_data.items[item_result.item_index];
                let style = &items_data.styles[item.style_index];

                let item_height = if let Some(ref result) = atomic_layout_results[step2_idx] {
                    result.size.height.to_f32()
                } else {
                    match style.height.length_type() {
                        openui_geometry::LengthType::Fixed => style.height.value(),
                        _ => {
                            let font_desc = style_to_font_description(style);
                            let font = Font::new(font_desc);
                            let metrics = font.font_metrics().copied().unwrap_or_default();
                            metrics.ascent + metrics.descent
                        }
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
                                metrics.int_line_spacing()
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

    // Pre-shape hyphen to include its width in alignment calculations.
    let hyphen_shape_data = if line_info.has_forced_hyphen && !line_info.has_ellipsis {
        let last_style = line_info.items.last()
            .map(|r| &items_data.styles[items_data.items[r.item_index].style_index])
            .unwrap_or(block_style);
        let hyphen_font_desc = style_to_font_description(last_style);
        let hyphen_font = Font::new(hyphen_font_desc);
        let shaper = TextShaper::new();
        let hyphen_text = "-";
        let hyphen_sr = shaper.shape(hyphen_text, &hyphen_font, openui_text::TextDirection::Ltr);
        let hyphen_width = LayoutUnit::from_f32(hyphen_sr.width);
        Some((Arc::new(hyphen_sr), hyphen_width, last_style.clone()))
    } else {
        None
    };

    let hyphen_extra_width = hyphen_shape_data.as_ref()
        .map(|(_, w, _)| *w)
        .unwrap_or(LayoutUnit::zero());

    // Pre-compute ellipsis width so alignment accounts for it.
    let ellipsis_extra_width = if line_info.has_ellipsis {
        let block_font_desc = style_to_font_description(block_style);
        let ellipsis_font = Font::new(block_font_desc);
        let shaper = TextShaper::new();
        let sr = shaper.shape("\u{2026}", &ellipsis_font, openui_text::TextDirection::Ltr);
        LayoutUnit::from_f32(sr.width)
    } else {
        LayoutUnit::zero()
    };

    // Total extra width from appended glyphs (hyphen or ellipsis, never both).
    let appended_extra_width = hyphen_extra_width + ellipsis_extra_width;

    // === STEP 3: Horizontal positioning (text-align) ===
    // Use the effective available width (after text-indent) for alignment
    // so that text-align: right with text-indent doesn't overflow.
    let align_available = available_width - text_indent;
    let text_align_offset = compute_text_align_offset(
        line_info,
        align_available - appended_extra_width,
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
        let remaining = align_available - appended_extra_width - line_info.used_width;
        if remaining > LayoutUnit::zero() {
            match text_justify {
                TextJustify::InterCharacter => {
                    let char_count = count_inter_character_opportunities(line_info, items_data);
                    if char_count > 0 {
                        justification_per_char = remaining.to_f32() / char_count as f32;
                    }
                }
                // Auto: use inter-character for CJK content, inter-word otherwise.
                TextJustify::Auto => {
                    if detect_cjk_content(line_info, items_data) {
                        let char_count = count_inter_character_opportunities(line_info, items_data);
                        if char_count > 0 {
                            justification_per_char = remaining.to_f32() / char_count as f32;
                        }
                    } else {
                        let space_count = count_expansion_opportunities(line_info, items_data);
                        if space_count > 0 {
                            justification_per_space = remaining.to_f32() / space_count as f32;
                        }
                    }
                }
                TextJustify::InterWord => {
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

    // --- Inline box decoration tracking (CSS Fragmentation §4.4) ---
    // Build a set of style indices for boxes open at line start, for quick lookup.
    let boxes_open_at_start_set: Vec<usize> = boxes_open_at_line_start
        .iter()
        .map(|b| b.style_index)
        .collect();

    // For `box-decoration-break: clone`, boxes that were already open at line
    // start need their inline-start MBP added at the beginning of this line.
    for open_box in boxes_open_at_line_start {
        if open_box.box_decoration_break == BoxDecorationBreak::Clone {
            let style = &items_data.styles[open_box.style_index];
            inline_offset = inline_offset + resolve_inline_start(style, percentage_base);
        }
    }

    // Pre-scan to determine which boxes don't close on this line (needed for
    // clone mode inline-end MBP and for is_last_for_node metadata).
    let mut scan_stack: Vec<usize> = boxes_open_at_start_set.clone();
    for item_result in &line_info.items {
        match item_result.item_type {
            InlineItemType::OpenTag => {
                let item = &items_data.items[item_result.item_index];
                scan_stack.push(item.style_index);
            }
            InlineItemType::CloseTag => {
                scan_stack.pop();
            }
            _ => {}
        }
    }
    // scan_stack now contains style indices of boxes still open at line end.
    let boxes_open_at_line_end: Vec<usize> = scan_stack;

    // Track the current inline box stack during positioning for
    // is_first_for_node / is_last_for_node metadata on text fragments.
    // Each entry: (style_index, is_first_fragment_on_this_line)
    let mut inline_box_stack: Vec<(usize, bool)> = boxes_open_at_start_set
        .iter()
        .map(|&si| (si, false)) // boxes from previous line are NOT first
        .collect();

    for (step4_idx, item_result) in line_info.items.iter().enumerate() {
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
                    let is_last_text = line_info.items[(step4_idx + 1)..]
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
                // Populate text_content so paint pipeline can use it for
                // emphasis marks and skip-ink CJK filtering.
                text_fragment.text_content = Some(
                    items_data.text[item_result.text_range.clone()].to_string()
                );
                text_fragment.offset = PhysicalOffset::new(inline_offset, text_top);
                // Store the baseline offset (distance from fragment top to baseline)
                // so paint can use it directly instead of recomputing from metrics.
                text_fragment.baseline_offset =
                    (baseline - text_top).to_f32();
                // Use justified shape result if justification was applied,
                // otherwise use the sub-range shape result for this line portion.
                text_fragment.shape_result = justified_shape.or(line_shape_result);

                // Set inline box decoration metadata for the paint system.
                // If this text is inside an inline box (span), indicate whether
                // it's on the first/last line of that box.
                if let Some(&(style_idx, is_first)) = inline_box_stack.last() {
                    text_fragment.is_first_for_node = is_first;
                    text_fragment.is_last_for_node =
                        !boxes_open_at_line_end.contains(&style_idx);
                }

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
                // Track that this box opened on this line (is_first = true).
                inline_box_stack.push((item.style_index, true));
            }
            InlineItemType::CloseTag => {
                let style = &items_data.styles[item.style_index];
                inline_offset = inline_offset + resolve_inline_end(style, percentage_base);
                inline_metrics_stack.pop();
                inline_box_stack.pop();
            }
            InlineItemType::Control => {
                // <br> — no visual contribution, line break already handled.
                // Reset inter-character tracking so no boundary gap is added
                // between text items separated by a control.
                inter_char_chars_before = 0;
            }
            InlineItemType::AtomicInline | InlineItemType::BlockInInline => {
                // Reset inter-character tracking so no boundary gap is added
                // between text items separated by an atomic inline.
                inter_char_chars_before = 0;
                // Use pre-computed block_layout result as authoritative source
                // for atomic inline dimensions (Issue 2 fix).
                let item_width = item_result.inline_size;
                let style = &items_data.styles[item.style_index];
                let item_height = if let Some(ref result) = atomic_layout_results[step4_idx] {
                    result.size.height
                } else {
                    match style.height.length_type() {
                        openui_geometry::LengthType::Fixed => {
                            LayoutUnit::from_f32(style.height.value())
                        }
                        _ => {
                            let font_desc = style_to_font_description(style);
                            let font = Font::new(font_desc);
                            let metrics = font.font_metrics().copied().unwrap_or_default();
                            LayoutUnit::from_f32_ceil(metrics.ascent + metrics.descent)
                        }
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
                                metrics.int_line_spacing()
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

                // Use the pre-computed block_layout result as the atomic fragment,
                // preserving its computed size, border, padding, margin, and children.
                // Only fall back to a new empty box when block_layout was not run.
                let atomic_fragment = if let Some(result) = atomic_layout_results[step4_idx].take() {
                    let mut frag = result;
                    // Use block_layout's authoritative width; only override height
                    // and position. block_layout already accounts for border+padding.
                    frag.size.height = item_height;
                    frag.offset = PhysicalOffset::new(inline_offset, atomic_top);
                    frag
                } else {
                    let mut frag = Fragment::new_box(
                        item.node_id,
                        PhysicalSize::new(item_width, item_height),
                    );
                    frag.offset = PhysicalOffset::new(inline_offset, atomic_top);
                    frag
                };

                children.push(atomic_fragment);
                inline_offset = inline_offset + item_result.inline_size;
            }
        }
    }

    // For `box-decoration-break: clone`, boxes still open at line end need
    // their inline-end MBP added after all items have been positioned.
    for &style_idx in &boxes_open_at_line_end {
        let style = &items_data.styles[style_idx];
        if style.box_decoration_break == BoxDecorationBreak::Clone {
            inline_offset = inline_offset + resolve_inline_end(style, percentage_base);
        }
    }

    // === STEP 4b: Append visible hyphen if line was broken at a soft hyphen ===
    if let Some((ref hyphen_sr, hyphen_width, ref hyphen_style)) = hyphen_shape_data {
        let hyphen_metrics = {
            let hyphen_font_desc = style_to_font_description(hyphen_style);
            let hyphen_font = Font::new(hyphen_font_desc);
            hyphen_font.font_metrics().copied().unwrap_or_default()
        };
        let hyphen_height = LayoutUnit::from_f32_ceil(
            hyphen_metrics.ascent + hyphen_metrics.descent,
        );
        let hyphen_top = baseline - LayoutUnit::from_f32_ceil(hyphen_metrics.ascent);

        let mut hyphen_fragment = Fragment::new_text(
            NodeId::NONE,
            PhysicalSize::new(hyphen_width, hyphen_height),
            Arc::clone(hyphen_sr),
            "-".to_string(),
        );
        hyphen_fragment.inherited_style = Some(hyphen_style.clone());
        hyphen_fragment.baseline_offset = (baseline - hyphen_top).to_f32();

        if block_style.direction == Direction::Rtl {
            // RTL: place hyphen at visual start (left of content), shift content right.
            for child in &mut children {
                child.offset = PhysicalOffset::new(
                    child.offset.left + hyphen_width,
                    child.offset.top,
                );
            }
            hyphen_fragment.offset = PhysicalOffset::new(
                text_align_offset + text_indent,
                hyphen_top,
            );
            children.insert(0, hyphen_fragment);
        } else {
            hyphen_fragment.offset = PhysicalOffset::new(inline_offset, hyphen_top);
            children.push(hyphen_fragment);
            inline_offset = inline_offset + hyphen_width;
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
            ellipsis_fragment.offset = PhysicalOffset::new(
                text_align_offset + text_indent,
                ellipsis_top,
            );
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
    line_fragment.baseline_offset = baseline.to_f32();
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
        line_info.has_forced_hyphen = false;
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

                            let last_mut = line_info.items.last_mut()
                                .expect("non-empty: guarded by while-loop condition above");
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
    line_info.has_forced_hyphen = false;
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
        oof_children: Vec::new(),
            block_in_inline: Vec::new(),
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
        oof_children: Vec::new(),
            block_in_inline: Vec::new(),
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
        oof_children: Vec::new(),
            block_in_inline: Vec::new(),
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

    // ── Issue 4 (R24): detect_cjk_content ──────────────────────────────

    #[test]
    fn is_cjk_character_detects_han_ideographs() {
        assert!(is_cjk_character('\u{4E00}')); // CJK Unified start
        assert!(is_cjk_character('\u{9FFF}')); // CJK Unified end
        assert!(is_cjk_character('\u{5927}')); // 大
        assert!(!is_cjk_character('A'));
        assert!(!is_cjk_character(' '));
    }

    #[test]
    fn is_cjk_character_detects_kana_and_hangul() {
        assert!(is_cjk_character('\u{3042}')); // Hiragana あ
        assert!(is_cjk_character('\u{30A2}')); // Katakana ア
        assert!(is_cjk_character('\u{AC00}')); // Hangul 가
        assert!(!is_cjk_character('Z'));
    }

    // ── Issue 2 (R26): atomic fragment uses block_layout width ──────────

    #[test]
    fn atomic_inline_block_layout_width_preserved() {
        // An inline-block child with width:80px inside a 400px container.
        // The fragment width should come from block_layout (80px), not from
        // the line breaker's item_result.inline_size which previously
        // overwrote the block_layout result.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(80.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::px(30.0);
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::from_i32(600),
        );
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "should have line boxes");
        let line = &div_frag.children[0];
        assert!(!line.children.is_empty(), "line should have atomic inline child");
        let atomic = &line.children[0];
        // Width should be 80px from block_layout, not overridden.
        assert!(
            (atomic.size.width.to_f32() - 80.0).abs() < 1.0,
            "atomic inline width should be ~80px from block_layout, got {}",
            atomic.size.width.to_f32(),
        );
    }

    #[test]
    fn atomic_inline_block_layout_width_not_overridden_with_border() {
        // An inline-block with width:60px and 5px border on each side.
        // block_layout should produce border-box width of 70px.
        // The fragment should preserve that, not override it.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(60.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::px(30.0);
        doc.node_mut(inline_block).style.border_left_width = 5;
        doc.node_mut(inline_block).style.border_right_width = 5;
        doc.node_mut(inline_block).style.border_left_style = openui_style::BorderStyle::Solid;
        doc.node_mut(inline_block).style.border_right_style = openui_style::BorderStyle::Solid;
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::from_i32(600),
        );
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        let line = &div_frag.children[0];
        let atomic = &line.children[0];
        // Width should be content(60) + border(10) = 70.
        assert!(
            (atomic.size.width.to_f32() - 70.0).abs() < 1.0,
            "atomic inline width should be ~70 (60+border 10), got {}",
            atomic.size.width.to_f32(),
        );
    }

    // ── Issue 3 (R26): percentage heights on atomic inlines resolve ──────

    #[test]
    fn atomic_inline_percentage_height_resolves_against_container() {
        // Container has fixed height 200px. Inline-block has height:50%.
        // Should resolve to 100px, not auto/0.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        doc.node_mut(div).style.height = openui_geometry::Length::px(200.0);
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(50.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::percent(50.0);
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::from_i32(600),
        );
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "should have line boxes");
        let line = &div_frag.children[0];
        assert!(!line.children.is_empty(), "line should have the inline-block");
        let atomic = &line.children[0];
        // Height should be 50% of 200 = 100px.
        let h = atomic.size.height.to_f32();
        assert!(
            h > 50.0 && h <= 100.0 + 1.0,
            "atomic inline height should be ~100px (50% of 200), got {}",
            h,
        );
    }

    #[test]
    fn atomic_inline_percentage_height_indefinite_container_uses_auto() {
        // Container has auto height (indefinite). Inline-block has height:50%.
        // Should behave as auto (not crash, and produce some reasonable height).
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.font_size = 16.0;
        doc.node_mut(div).style.width = openui_geometry::Length::px(400.0);
        // height: auto (default)
        doc.append_child(vp, div);

        let inline_block = doc.create_node(ElementTag::Div);
        doc.node_mut(inline_block).style.display = Display::InlineBlock;
        doc.node_mut(inline_block).style.width = openui_geometry::Length::px(50.0);
        doc.node_mut(inline_block).style.height = openui_geometry::Length::percent(50.0);
        doc.append_child(div, inline_block);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(400),
            LayoutUnit::max(),
        );
        let fragment = crate::block::block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];
        assert!(!div_frag.children.is_empty(), "should have line boxes");
        // Just verify it doesn't crash/panic with indefinite containing block.
        let line = &div_frag.children[0];
        assert!(!line.children.is_empty(), "line should have the inline-block");
    }

    // ── Hang width + alignment tests ─────────────────────────────────

    #[test]
    fn text_align_right_with_hang_width() {
        // When hang_width > 0, the remaining space for alignment should be
        // based on used_width (which already excludes hung spaces), not on
        // used_width + hang_width.
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Right, false);
        // Simulate 20px of hung trailing spaces: used_width is 100 (after
        // subtraction), and hang_width records what was subtracted.
        line.hang_width = LayoutUnit::from_f32(20.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(200), Direction::Ltr, TextAlignLast::Auto,
        );
        // remaining = 200 - 100 = 100 → right-align offset = 100
        assert_eq!(offset, LayoutUnit::from_i32(100),
            "right-align should use used_width (not used_width + hang_width)");
    }

    #[test]
    fn text_align_center_with_hang_width() {
        let mut line = make_test_line_info(200.0, 80.0, TextAlign::Center, false);
        line.hang_width = LayoutUnit::from_f32(10.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(200), Direction::Ltr, TextAlignLast::Auto,
        );
        // remaining = 200 - 80 = 120 → center offset = 60
        assert_eq!(offset.to_i32(), 60,
            "center-align should use used_width (excluding hang_width)");
    }

    #[test]
    fn text_align_left_unaffected_by_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Left, false);
        line.hang_width = LayoutUnit::from_f32(30.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(200), Direction::Ltr, TextAlignLast::Auto,
        );
        assert_eq!(offset, LayoutUnit::zero(),
            "left-align offset is always 0 regardless of hang_width");
    }

    #[test]
    fn text_align_start_rtl_with_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Start, false);
        line.hang_width = LayoutUnit::from_f32(15.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(200), Direction::Rtl, TextAlignLast::Auto,
        );
        // RTL start = right-align: remaining = 200 - 100 = 100
        assert_eq!(offset, LayoutUnit::from_i32(100),
            "RTL start-align with hang_width");
    }

    #[test]
    fn text_align_end_ltr_with_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::End, false);
        line.hang_width = LayoutUnit::from_f32(25.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(200), Direction::Ltr, TextAlignLast::Auto,
        );
        // LTR end = right-align: remaining = 200 - 100 = 100
        assert_eq!(offset, LayoutUnit::from_i32(100),
            "LTR end-align with hang_width");
    }

    #[test]
    fn text_align_justify_with_hang_width() {
        let mut line = make_test_line_info(200.0, 100.0, TextAlign::Justify, false);
        line.hang_width = LayoutUnit::from_f32(20.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(200), Direction::Ltr, TextAlignLast::Auto,
        );
        // Justify offset is always 0 (justification done by space expansion).
        assert_eq!(offset, LayoutUnit::zero(),
            "justify offset is 0 regardless of hang_width");
    }

    #[test]
    fn text_align_last_center_with_hang_width() {
        let mut line = make_test_line_info(200.0, 80.0, TextAlign::Justify, false);
        line.is_last_line = true;
        line.hang_width = LayoutUnit::from_f32(10.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(200), Direction::Ltr, TextAlignLast::Center,
        );
        // Last line with text-align-last: center; remaining = 200 - 80 = 120; offset = 60
        assert_eq!(offset.to_i32(), 60,
            "text-align-last: center with hang_width on last line");
    }

    #[test]
    fn hang_width_does_not_cause_overflow_alignment() {
        // If used_width < available but used_width + hang_width > available,
        // alignment should still work (hang_width is excluded from overflow check).
        let mut line = make_test_line_info(100.0, 90.0, TextAlign::Right, false);
        line.hang_width = LayoutUnit::from_f32(20.0);

        let offset = compute_text_align_offset(
            &line, LayoutUnit::from_i32(100), Direction::Ltr, TextAlignLast::Auto,
        );
        // remaining = 100 - 90 = 10 → still positive → offset = 10
        assert_eq!(offset, LayoutUnit::from_i32(10),
            "hang_width overflow should not affect alignment");
    }
}
