//! Intrinsic block sizing — min-content, max-content, shrink-to-fit.
//!
//! CSS Intrinsic & Extrinsic Sizing Module Level 3.
//!
//! This module computes the natural width/height of block-level elements
//! based on their content, ignoring available space from the parent.
//! Used for:
//! - Auto-width determination
//! - Shrink-to-fit contexts (floats, abs pos, inline-blocks)
//! - `min-content` / `max-content` CSS values
//! - Table cell sizing
//!
//! Source: CSS Sizing 3 §4-5, CSS 2.1 §10.3.5-7, §10.6.7.

use openui_geometry::{LayoutUnit, MinMaxSizes};
use openui_style::ComputedStyle;
use openui_dom::{Document, ElementTag, NodeId};

use crate::block::{resolve_border, resolve_padding, resolve_margins};
use crate::length_resolver::resolve_length;

/// Check if a style represents an inline-level element.
fn is_inline_level(style: &ComputedStyle) -> bool {
    style.display.is_inline_level()
}

// ── Result struct ────────────────────────────────────────────────────────

/// Intrinsic sizes for an element in both axes.
///
/// CSS Sizing 3 §4: every box has a min-content and max-content size in
/// both the inline and block dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IntrinsicSizes {
    /// The narrowest the element can be without overflow (inline axis).
    pub min_content_inline_size: LayoutUnit,
    /// The widest the element would be given infinite available space (inline axis).
    pub max_content_inline_size: LayoutUnit,
    /// Min-content contribution in the block axis.
    pub min_content_block_size: LayoutUnit,
    /// Max-content contribution in the block axis.
    pub max_content_block_size: LayoutUnit,
}

impl IntrinsicSizes {
    pub fn zero() -> Self {
        Self {
            min_content_inline_size: LayoutUnit::zero(),
            max_content_inline_size: LayoutUnit::zero(),
            min_content_block_size: LayoutUnit::zero(),
            max_content_block_size: LayoutUnit::zero(),
        }
    }
}

impl Default for IntrinsicSizes {
    fn default() -> Self {
        Self::zero()
    }
}

// ── Block container intrinsic sizing ─────────────────────────────────────

/// Compute intrinsic inline sizes for a block container.
///
/// CSS Sizing 3 §5.1: For a block container, the min-content inline size
/// is the maximum of all children's min-content inline-size contributions.
/// The max-content inline size is the maximum of all children's max-content
/// inline-size contributions.
///
/// Border and padding of the container are added to the result.
pub fn compute_intrinsic_block_sizes(doc: &Document, node_id: NodeId) -> IntrinsicSizes {
    let style = &doc.node(node_id).style;
    let tag = doc.node(node_id).tag;

    // Replaced elements use their own intrinsic dimensions.
    if is_replaced_element(tag) {
        return compute_replaced_intrinsic_sizes(style);
    }

    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let bp_inline = border.inline_sum() + padding.inline_sum();
    let bp_block = border.block_sum() + padding.block_sum();

    let mut min_inline = LayoutUnit::zero();
    let mut max_inline = LayoutUnit::zero();
    let mut content_block = LayoutUnit::zero();

    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;

        // Skip out-of-flow and display:none children.
        if child_style.display == openui_style::Display::None || child_style.is_out_of_flow() {
            continue;
        }

        let child_sizes = compute_child_intrinsic_contribution(doc, child_id);

        // Block container: each child contribution is independent.
        // Min-content inline = max of all children's min-content.
        min_inline = min_inline.max_of(child_sizes.min_content_inline_size);
        // Max-content inline = max of all children's max-content.
        max_inline = max_inline.max_of(child_sizes.max_content_inline_size);

        // Block size: sum of children's max-content block contributions.
        content_block = content_block + child_sizes.max_content_block_size;
    }

    // Add container border + padding.
    IntrinsicSizes {
        min_content_inline_size: min_inline + bp_inline,
        max_content_inline_size: max_inline + bp_inline,
        min_content_block_size: content_block + bp_block,
        max_content_block_size: content_block + bp_block,
    }
}

/// Compute the intrinsic size contribution of a single child.
///
/// This accounts for the child's own intrinsic sizes plus its margin box.
/// For inline-level children (text, inline), uses inline intrinsic sizing.
fn compute_child_intrinsic_contribution(doc: &Document, child_id: NodeId) -> IntrinsicSizes {
    let child_style = &doc.node(child_id).style;
    let child_tag = doc.node(child_id).tag;

    // Resolve child margins (percentages resolve to zero for intrinsic sizing).
    let margin = resolve_margins(child_style, LayoutUnit::zero());
    let margin_inline = margin.inline_sum();
    let margin_block = margin.block_sum();

    // For text nodes and inline-level elements, use inline intrinsic sizing.
    let child_intrinsic = if child_tag == ElementTag::Text || is_inline_level(child_style) {
        let inline_sizes = compute_intrinsic_inline_sizes(doc, child_id);
        IntrinsicSizes {
            min_content_inline_size: inline_sizes.min,
            max_content_inline_size: inline_sizes.max,
            min_content_block_size: LayoutUnit::zero(),
            max_content_block_size: LayoutUnit::zero(),
        }
    } else {
        // Block-level children: recursive block intrinsic sizing.
        compute_intrinsic_block_sizes(doc, child_id)
    };

    // Apply explicit width if set (CSS Sizing 3 §5.1 — definite sizes override).
    let min_inline = apply_size_override_inline(child_style, child_intrinsic.min_content_inline_size);
    let max_inline = apply_size_override_inline(child_style, child_intrinsic.max_content_inline_size);

    // Apply min-width / max-width clamping.
    let min_inline = apply_min_max_inline(child_style, min_inline);
    let max_inline = apply_min_max_inline(child_style, max_inline);

    // Apply explicit height if set.
    let block_size = apply_size_override_block(child_style, child_intrinsic.max_content_block_size);
    let block_size = apply_min_max_block(child_style, block_size);

    IntrinsicSizes {
        min_content_inline_size: min_inline + margin_inline,
        max_content_inline_size: max_inline + margin_inline,
        min_content_block_size: block_size + margin_block,
        max_content_block_size: block_size + margin_block,
    }
}

// ── Inline-level intrinsic sizing ────────────────────────────────────────

/// Compute intrinsic inline sizes for inline-level content.
///
/// - Text: min-content = widest word, max-content = entire text line width.
/// - Replaced: intrinsic width from the element.
/// - Inline-block: recursive intrinsic sizing.
pub fn compute_intrinsic_inline_sizes(doc: &Document, node_id: NodeId) -> MinMaxSizes {
    let node = doc.node(node_id);
    let tag = node.tag;

    match tag {
        ElementTag::Text => {
            // For text nodes, approximate word-based sizing.
            if let Some(ref text) = node.text {
                compute_text_intrinsic_sizes(text)
            } else {
                MinMaxSizes::zero()
            }
        }
        _ if is_replaced_element(tag) => {
            let sizes = compute_replaced_intrinsic_sizes(&node.style);
            MinMaxSizes::new(
                sizes.min_content_inline_size,
                sizes.max_content_inline_size,
            )
        }
        _ => {
            // Inline-block or other: recursive sizing.
            let sizes = compute_intrinsic_block_sizes(doc, node_id);
            MinMaxSizes::new(
                sizes.min_content_inline_size,
                sizes.max_content_inline_size,
            )
        }
    }
}

/// Compute text intrinsic sizes.
///
/// min-content = widest word (based on character count × average char width).
/// max-content = full text width.
///
/// Uses `chars().count()` for correct Unicode handling (multi-byte characters).
/// The average character width is an approximation; real text shaping is handled
/// by the inline layout module when content is actually rendered.
fn compute_text_intrinsic_sizes(text: &str) -> MinMaxSizes {
    const APPROX_CHAR_WIDTH: f32 = 8.0;

    if text.is_empty() {
        return MinMaxSizes::zero();
    }

    // Max-content: entire text on one line.
    let max_content = LayoutUnit::from_f32(text.chars().count() as f32 * APPROX_CHAR_WIDTH);

    // Min-content: widest single word.
    let min_content = text
        .split_whitespace()
        .map(|word| LayoutUnit::from_f32(word.chars().count() as f32 * APPROX_CHAR_WIDTH))
        .fold(LayoutUnit::zero(), |acc, w| acc.max_of(w));

    MinMaxSizes::new(min_content, max_content)
}

// ── Auto block size from content ─────────────────────────────────────────

/// Compute the auto block size of an element from its children.
///
/// CSS 2.1 §10.6.3: the height of a block-level element with `height: auto`
/// is the distance between the top content edge and:
/// - the bottom edge of the last in-flow child's margin box, or
/// - the bottom edge of the last line box, or
/// - zero if there are no children.
///
/// Margins between children collapse per CSS 2.1 §8.3.1.
///
/// The result is clamped by min-height / max-height.
pub fn compute_block_size_from_content(
    doc: &Document,
    node_id: NodeId,
    child_margin_boxes: &[LayoutUnit],
) -> LayoutUnit {
    let style = &doc.node(node_id).style;

    // Sum of all children's margin-box block sizes.
    let mut content_height = LayoutUnit::zero();
    for &child_block in child_margin_boxes {
        content_height = content_height + child_block;
    }

    // Apply simple margin collapsing between adjacent siblings.
    // For intrinsic sizing purposes, we apply a simplified version:
    // adjacent positive margins collapse (take the larger).
    content_height = collapse_adjacent_margins(doc, node_id, content_height);

    // Clamp by min-height / max-height.
    let min_height = resolve_length(
        &style.min_height,
        LayoutUnit::zero(), // percentage resolves to 0 when containing block is auto
        LayoutUnit::zero(), // auto min-height = 0
        LayoutUnit::zero(), // none = 0
    );
    let max_height = resolve_length(
        &style.max_height,
        LayoutUnit::zero(),
        LayoutUnit::max(),   // auto = unconstrained
        LayoutUnit::max(),   // none = unconstrained
    );

    content_height.clamp(min_height, max_height)
}

/// Simplified margin collapsing for block size computation.
///
/// Looks at adjacent children's margins and collapses them per CSS 2.1 §8.3.1.
/// Returns the adjusted total block size.
fn collapse_adjacent_margins(
    doc: &Document,
    node_id: NodeId,
    raw_sum: LayoutUnit,
) -> LayoutUnit {
    let children: Vec<NodeId> = doc.children(node_id).collect();
    if children.len() < 2 {
        return raw_sum;
    }

    let mut collapsed_reduction = LayoutUnit::zero();

    for i in 0..children.len() - 1 {
        let current_style = &doc.node(children[i]).style;
        let next_style = &doc.node(children[i + 1]).style;

        let current_bottom = resolve_length(
            &current_style.margin_bottom,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );
        let next_top = resolve_length(
            &next_style.margin_top,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        );

        // Only collapse when both are non-negative (simple case).
        if current_bottom.raw() >= 0 && next_top.raw() >= 0 {
            let smaller = current_bottom.min_of(next_top);
            collapsed_reduction = collapsed_reduction + smaller;
        } else if current_bottom.raw() < 0 && next_top.raw() < 0 {
            // Both negative: keep the more negative, remove the other.
            let less_negative = current_bottom.max_of(next_top);
            collapsed_reduction = collapsed_reduction + less_negative;
        }
        // Mixed positive/negative: they sum (no collapsing reduction).
    }

    raw_sum - collapsed_reduction
}

// ── Shrink-to-fit ────────────────────────────────────────────────────────

/// CSS 2.1 §10.3.5: shrink-to-fit inline size.
///
/// `min(max(min_content, available), max_content)`
///
/// Used for floats, absolutely positioned elements with `width: auto`,
/// inline-block elements, and table cells.
#[inline]
pub fn shrink_to_fit_inline_size(
    min_content: LayoutUnit,
    max_content: LayoutUnit,
    available: LayoutUnit,
) -> LayoutUnit {
    // preferred minimum width = min_content
    // preferred width = max_content
    // available width = available
    // Result = min(max(preferred minimum, available), preferred)
    let lower = min_content.max_of(available);
    lower.min_of(max_content)
}

// ── Replaced element intrinsic sizes ─────────────────────────────────────

/// Compute intrinsic sizes for replaced elements (img, video, canvas, etc.).
///
/// CSS 2.1 §10.3.2, CSS Sizing 3 §5.2:
/// - Use intrinsic width/height if specified (CSS `width`/`height` on the element).
/// - If only one dimension is specified and the element has an aspect ratio,
///   derive the other from the ratio.
/// - Default to 300×150 for objects with no intrinsic size (CSS 2.1 §10.3.2).
pub fn compute_replaced_intrinsic_sizes(style: &ComputedStyle) -> IntrinsicSizes {
    // Default replaced element size (CSS 2.1 §10.3.2).
    let default_width = LayoutUnit::from_i32(300);
    let default_height = LayoutUnit::from_i32(150);

    let has_width = style.width.length_type() == openui_geometry::LengthType::Fixed;
    let has_height = style.height.length_type() == openui_geometry::LengthType::Fixed;

    let (width, height) = match (has_width, has_height) {
        (true, true) => {
            let w = LayoutUnit::from_f32(style.width.value());
            let h = LayoutUnit::from_f32(style.height.value());
            (w, h)
        }
        (true, false) => {
            let w = LayoutUnit::from_f32(style.width.value());
            // Derive height from aspect ratio (default 2:1 → 300:150).
            let h = apply_aspect_ratio(w, default_width, default_height);
            (w, h)
        }
        (false, true) => {
            let h = LayoutUnit::from_f32(style.height.value());
            // Derive width from aspect ratio.
            let w = apply_aspect_ratio_inverse(h, default_width, default_height);
            (w, h)
        }
        (false, false) => {
            (default_width, default_height)
        }
    };

    // Add border + padding.
    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let bp_inline = border.inline_sum() + padding.inline_sum();
    let bp_block = border.block_sum() + padding.block_sum();

    let total_inline = width + bp_inline;
    let total_block = height + bp_block;

    IntrinsicSizes {
        min_content_inline_size: total_inline,
        max_content_inline_size: total_inline,
        min_content_block_size: total_block,
        max_content_block_size: total_block,
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// True if this element tag represents a replaced element.
///
/// In the current DOM model, only `Viewport` and `Text` are special;
/// all others are generic containers. We treat none as replaced for now,
/// but this function provides the extension point. The caller can mark
/// elements as replaced through the style (e.g., explicit width+height
/// on an img-like element).
fn is_replaced_element(_tag: ElementTag) -> bool {
    // In the current DOM model there are no dedicated replaced element tags.
    // Replaced sizing is triggered via `compute_replaced_intrinsic_sizes`
    // when called explicitly by the layout algorithm for known replaced
    // elements. For intrinsic block sizing, we always recurse into children.
    false
}

/// Derive height from width using the default aspect ratio.
///
/// `height = width * (intrinsic_height / intrinsic_width)`
fn apply_aspect_ratio(
    known: LayoutUnit,
    intrinsic_width: LayoutUnit,
    intrinsic_height: LayoutUnit,
) -> LayoutUnit {
    if intrinsic_width.raw() == 0 {
        return intrinsic_height;
    }
    known.mul_div(intrinsic_height, intrinsic_width)
}

/// Derive width from height using the default aspect ratio.
///
/// `width = height * (intrinsic_width / intrinsic_height)`
fn apply_aspect_ratio_inverse(
    known: LayoutUnit,
    intrinsic_width: LayoutUnit,
    intrinsic_height: LayoutUnit,
) -> LayoutUnit {
    if intrinsic_height.raw() == 0 {
        return intrinsic_width;
    }
    known.mul_div(intrinsic_width, intrinsic_height)
}

/// If the element has an explicit fixed width, use it (content-box);
/// otherwise return the intrinsic size.
fn apply_size_override_inline(style: &ComputedStyle, intrinsic: LayoutUnit) -> LayoutUnit {
    if style.width.length_type() == openui_geometry::LengthType::Fixed {
        LayoutUnit::from_f32(style.width.value())
    } else {
        intrinsic
    }
}

/// If the element has an explicit fixed height, use it; otherwise return intrinsic.
fn apply_size_override_block(style: &ComputedStyle, intrinsic: LayoutUnit) -> LayoutUnit {
    if style.height.length_type() == openui_geometry::LengthType::Fixed {
        LayoutUnit::from_f32(style.height.value())
    } else {
        intrinsic
    }
}

/// Clamp a resolved inline size by min-width / max-width.
fn apply_min_max_inline(style: &ComputedStyle, size: LayoutUnit) -> LayoutUnit {
    let min = resolve_length(
        &style.min_width,
        LayoutUnit::zero(),
        LayoutUnit::zero(), // auto min-width = 0
        LayoutUnit::zero(),
    );
    let max = resolve_length(
        &style.max_width,
        LayoutUnit::zero(),
        LayoutUnit::max(), // auto = unconstrained
        LayoutUnit::max(), // none = unconstrained
    );
    size.clamp(min, max)
}

/// Clamp a resolved block size by min-height / max-height.
fn apply_min_max_block(style: &ComputedStyle, size: LayoutUnit) -> LayoutUnit {
    let min = resolve_length(
        &style.min_height,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        LayoutUnit::zero(),
    );
    let max = resolve_length(
        &style.max_height,
        LayoutUnit::zero(),
        LayoutUnit::max(),
        LayoutUnit::max(),
    );
    size.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intrinsic_sizes_zero_default() {
        let sizes = IntrinsicSizes::zero();
        assert_eq!(sizes.min_content_inline_size, LayoutUnit::zero());
        assert_eq!(sizes.max_content_inline_size, LayoutUnit::zero());
        assert_eq!(sizes.min_content_block_size, LayoutUnit::zero());
        assert_eq!(sizes.max_content_block_size, LayoutUnit::zero());
    }

    #[test]
    fn shrink_to_fit_uses_max_when_available_exceeds() {
        let min = LayoutUnit::from_i32(50);
        let max = LayoutUnit::from_i32(200);
        let available = LayoutUnit::from_i32(300);
        // min(max(50, 300), 200) = min(300, 200) = 200
        assert_eq!(shrink_to_fit_inline_size(min, max, available), max);
    }

    #[test]
    fn shrink_to_fit_uses_available_in_between() {
        let min = LayoutUnit::from_i32(50);
        let max = LayoutUnit::from_i32(200);
        let available = LayoutUnit::from_i32(150);
        // min(max(50, 150), 200) = min(150, 200) = 150
        assert_eq!(
            shrink_to_fit_inline_size(min, max, available),
            LayoutUnit::from_i32(150)
        );
    }

    #[test]
    fn shrink_to_fit_uses_min_when_available_too_small() {
        let min = LayoutUnit::from_i32(100);
        let max = LayoutUnit::from_i32(200);
        let available = LayoutUnit::from_i32(50);
        // min(max(100, 50), 200) = min(100, 200) = 100
        assert_eq!(shrink_to_fit_inline_size(min, max, available), min);
    }

    #[test]
    fn text_min_content_widest_word() {
        let sizes = compute_text_intrinsic_sizes("hello world");
        // "hello" = 5 chars, "world" = 5 chars → min = 5 * 8 = 40
        assert_eq!(sizes.min, LayoutUnit::from_f32(40.0));
        // "hello world" = 11 chars → max = 11 * 8 = 88
        assert_eq!(sizes.max, LayoutUnit::from_f32(88.0));
    }

    #[test]
    fn text_single_word_min_equals_max() {
        let sizes = compute_text_intrinsic_sizes("indivisible");
        // Both min and max are the full word
        assert_eq!(sizes.min, sizes.max);
    }
}
