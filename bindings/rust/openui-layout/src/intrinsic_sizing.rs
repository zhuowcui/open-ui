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

use openui_geometry::{LayoutUnit, LengthType, MinMaxSizes};
use openui_style::ComputedStyle;
use openui_style::BoxSizing;
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

    // Flex containers have their own intrinsic sizing algorithm.
    // CSS Flexbox §9.9: Flex container intrinsic sizes.
    if style.display.is_flex() {
        return compute_flex_intrinsic_sizes(doc, node_id, style);
    }

    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let bp_inline = border.inline_sum() + padding.inline_sum();
    let bp_block = border.block_sum() + padding.block_sum();

    let mut min_inline = LayoutUnit::zero();
    // CSS Sizing 3 §4.1: Max-content inline size must accommodate all floats
    // side-by-side (summed), plus the widest non-float child (maxed).
    let mut non_float_max_inline = LayoutUnit::zero();
    // For inline-only containers: max-content is the SUM of inline children
    // (they all sit on one line in max-content mode).
    let mut inline_children_max_sum = LayoutUnit::zero();
    let mut float_max_inline_sum = LayoutUnit::zero();
    // Track min-content and max-content block sizes separately.
    // CSS Sizing 3 §5: min-content uses each child's min-content contribution,
    // max-content uses each child's max-content contribution.
    let mut min_content_block = LayoutUnit::zero();
    let mut max_content_block = LayoutUnit::zero();
    // CSS Sizing 3 §5: Float block-size contributions differ between modes.
    let mut float_block_sum = LayoutUnit::zero();  // for min-content
    let mut float_block_max = LayoutUnit::zero();  // for max-content
    let is_bfc = style.creates_new_formatting_context();

    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;

        // Skip absolutely positioned and display:none children.
        if child_style.display == openui_style::Display::None
            || child_style.position.is_absolutely_positioned()
        {
            continue;
        }

        let child_sizes = compute_child_intrinsic_contribution(doc, child_id);
        let child_is_inline = is_inline_level(child_style)
            || doc.node(child_id).tag == ElementTag::Text;

        if child_style.float != openui_style::Float::None {
            min_inline = min_inline.max_of(child_sizes.min_content_inline_size);
            float_max_inline_sum = float_max_inline_sum + child_sizes.max_content_inline_size;
            float_block_sum = float_block_sum + child_sizes.min_content_block_size;
            float_block_max = float_block_max.max_of(child_sizes.max_content_block_size);
        } else if child_is_inline {
            // CSS Sizing 3 §4.1: Inline-level children share a line.
            // min-content: widest individual inline item.
            // max-content: all inline items on one line (sum widths).
            min_inline = min_inline.max_of(child_sizes.min_content_inline_size);
            inline_children_max_sum = inline_children_max_sum + child_sizes.max_content_inline_size;
            // Block-size for inline children is computed via inline layout below
            // (not by summing individual contributions, which would double-count).
        } else {
            // CSS Sizing 3 §4.1: non-float block children contribute via max.
            min_inline = min_inline.max_of(child_sizes.min_content_inline_size);
            non_float_max_inline = non_float_max_inline.max_of(child_sizes.max_content_inline_size);
            min_content_block = min_content_block + child_sizes.min_content_block_size;
            max_content_block = max_content_block + child_sizes.max_content_block_size;
        }
    }

    // Inline children on one line contribute their sum as max-content.
    non_float_max_inline = non_float_max_inline.max_of(inline_children_max_sum);

    // Max-content inline: container must be wide enough for all floats
    // side-by-side OR the widest non-float child, whichever is larger.
    let max_inline = non_float_max_inline.max_of(float_max_inline_sum);

    // BFC roots include float bottom margin edge in auto height (§10.6.7).
    if is_bfc {
        min_content_block = min_content_block.max_of(float_block_sum);
        max_content_block = max_content_block.max_of(float_block_max);
    }

    // CSS 2.1 §10.6.3 / CSS Sizing 3 §5: Inline content contributes to
    // the block-size by running inline layout at the given width and
    // measuring the resulting height. Individual inline items report zero
    // block-size; the container must compute it from line layout.
    //
    // Blink: BlockNode::ComputeMinMaxSizes → runs inline layout to
    // determine block-size contribution from inline formatting contexts.
    let has_inline_children = crate::inline::algorithm::has_inline_children(doc, node_id);
    let has_block_children = crate::block::has_block_children(doc, node_id);

    if has_inline_children && !has_block_children {
        let content_inline_min = min_inline;
        let content_inline_max = max_inline;

        // Run inline layout at min-content width to get the block-size when
        // text wraps as tightly as possible.
        let min_space = crate::ConstraintSpace::for_block_child(
            content_inline_min,
            LayoutUnit::max(),
            content_inline_min,
            LayoutUnit::zero(),
            false,
        );
        let min_frag = crate::inline::algorithm::inline_layout(doc, node_id, &min_space);
        min_content_block = min_content_block + min_frag.size.height;

        // Run inline layout at max-content width to get the block-size when
        // text is laid out as wide as possible (single line if fits).
        let max_space = crate::ConstraintSpace::for_block_child(
            content_inline_max,
            LayoutUnit::max(),
            content_inline_max,
            LayoutUnit::zero(),
            false,
        );
        let max_frag = crate::inline::algorithm::inline_layout(doc, node_id, &max_space);
        max_content_block = max_content_block + max_frag.size.height;
    }

    // Add container border + padding.
    IntrinsicSizes {
        min_content_inline_size: min_inline + bp_inline,
        max_content_inline_size: max_inline + bp_inline,
        min_content_block_size: min_content_block + bp_block,
        max_content_block_size: max_content_block + bp_block,
    }
}

/// Compute intrinsic sizes for flex containers.
///
/// CSS Flexbox §9.9: Flex container intrinsic main size is determined by
/// item flex-base sizes; cross size by item cross sizes.
///
/// For row flex (is_column = false):
///   - min-content inline: largest item min-content contribution (single-line)
///     or sum if no wrapping possible
///   - max-content inline: sum of all item max-content contributions + gaps
///   - min/max-content block: max of item cross sizes
///
/// For column flex (is_column = true): swap axes.
fn compute_flex_intrinsic_sizes(
    doc: &Document,
    node_id: NodeId,
    style: &ComputedStyle,
) -> IntrinsicSizes {
    let border = resolve_border(style);
    let padding = resolve_padding(style, LayoutUnit::zero());
    let bp_inline = border.inline_sum() + padding.inline_sum();
    let bp_block = border.block_sum() + padding.block_sum();

    let is_column = style.flex_direction == openui_style::FlexDirection::Column
        || style.flex_direction == openui_style::FlexDirection::ColumnReverse;
    let is_wrap = style.flex_wrap != openui_style::FlexWrap::Nowrap;

    // Resolve gaps
    let main_gap = if is_column {
        style.row_gap.as_ref().map(|g| resolve_length(g, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero())).unwrap_or(LayoutUnit::zero())
    } else {
        style.column_gap.as_ref().map(|g| resolve_length(g, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero())).unwrap_or(LayoutUnit::zero())
    };
    let cross_gap = if is_column {
        style.column_gap.as_ref().map(|g| resolve_length(g, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero())).unwrap_or(LayoutUnit::zero())
    } else {
        style.row_gap.as_ref().map(|g| resolve_length(g, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero())).unwrap_or(LayoutUnit::zero())
    };

    // Per-item data needed for column-wrap wrapping simulation.
    struct ItemData {
        main_min: LayoutUnit,
        main_max: LayoutUnit,
        cross_min: LayoutUnit,
        cross_max: LayoutUnit,
    }

    let mut items: Vec<ItemData> = Vec::new();
    let mut sum_main_min = LayoutUnit::zero();
    let mut sum_main_max = LayoutUnit::zero();
    let mut max_main_min = LayoutUnit::zero();
    let mut max_cross_min = LayoutUnit::zero();
    let mut max_cross_max = LayoutUnit::zero();

    // Resolve the container's definite cross size (if any) for aspect-ratio children.
    // Row flex: cross = block (height); Column flex: cross = inline (width).
    let container_definite_cross = {
        let cross_prop = if is_column { &style.width } else { &style.height };
        if !cross_prop.is_auto() && cross_prop.is_fixed() {
            let val = resolve_length(cross_prop, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero());
            let content_val = if style.box_sizing == BoxSizing::BorderBox {
                if is_column {
                    (val - bp_inline).clamp_negative_to_zero()
                } else {
                    (val - bp_block).clamp_negative_to_zero()
                }
            } else { val };
            content_val
        } else {
            LayoutUnit::zero()
        }
    };

    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;

        if child_style.display == openui_style::Display::None
            || child_style.position.is_absolutely_positioned()
        {
            continue;
        }

        let child_sizes = compute_child_intrinsic_contribution(doc, child_id);

        // Determine main-axis and cross-axis contributions
        let (mut main_min, mut main_max, cross_min, cross_max) = if is_column {
            (child_sizes.min_content_block_size, child_sizes.max_content_block_size,
             child_sizes.min_content_inline_size, child_sizes.max_content_inline_size)
        } else {
            (child_sizes.min_content_inline_size, child_sizes.max_content_inline_size,
             child_sizes.min_content_block_size, child_sizes.max_content_block_size)
        };

        // CSS Flexbox §9.9.1: When the flex container has a definite cross size
        // and a child has aspect-ratio with auto main size, the child's main-axis
        // contribution should be derived from the definite cross size via AR.
        // This handles cases like `inline-flex; height:100px` with child `aspect-ratio:1/1`.
        if let Some(ref ar) = child_style.aspect_ratio {
            if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
                let (cross_size_prop, main_size_prop) = if is_column {
                    (&child_style.width, &child_style.height)
                } else {
                    (&child_style.height, &child_style.width)
                };
                // Only apply when cross size is definite (from container or child)
                // and main size is auto
                if main_size_prop.is_auto() {
                    let definite_cross = if !cross_size_prop.is_auto() && cross_size_prop.is_fixed() {
                        Some(resolve_length(cross_size_prop, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero()))
                    } else if container_definite_cross > LayoutUnit::zero() {
                        // Container's definite cross size (stretch)
                        Some(container_definite_cross)
                    } else {
                        None
                    };
                    if let Some(cross_val) = definite_cross {
                        let child_bp = {
                            let b = resolve_border(child_style);
                            let p = resolve_padding(child_style, LayoutUnit::zero());
                            if is_column {
                                (b.left + b.right + p.left + p.right,
                                 b.top + b.bottom + p.top + p.bottom)
                            } else {
                                (b.top + b.bottom + p.top + p.bottom,
                                 b.left + b.right + p.left + p.right)
                            }
                        };
                        let content_cross = (cross_val - child_bp.0).clamp_negative_to_zero();
                        let transferred = if is_column {
                            // Column: cross=inline(width), main=block(height)
                            LayoutUnit::from_f32(content_cross.to_f32() * ar.ratio.1 / ar.ratio.0)
                        } else {
                            // Row: cross=block(height), main=inline(width)
                            LayoutUnit::from_f32(content_cross.to_f32() * ar.ratio.0 / ar.ratio.1)
                        };
                        let ar_main = transferred + child_bp.1;
                        main_min = main_min.max_of(ar_main);
                        main_max = main_max.max_of(ar_main);
                    }
                }
            }
        }

        // Check for explicit flex-basis
        let flex_basis = &child_style.flex_basis;
        let main_contribution_min;
        let main_contribution_max;

        if flex_basis.is_fixed() {
            let basis = resolve_length(flex_basis, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero());
            main_contribution_min = basis.max_of(main_min);
            main_contribution_max = basis.max_of(main_max);
        } else {
            main_contribution_min = main_min;
            main_contribution_max = main_max;
        }

        // CSS Flexbox §9.9.1: Clamp item contributions by main-axis min/max constraints.
        let (min_main_prop, max_main_prop) = if is_column {
            (&child_style.min_height, &child_style.max_height)
        } else {
            (&child_style.min_width, &child_style.max_width)
        };
        let clamped_min_val = if !min_main_prop.is_auto() && min_main_prop.is_fixed() {
            resolve_length(min_main_prop, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero())
        } else {
            LayoutUnit::zero()
        };
        let clamped_max_val = if !max_main_prop.is_none() && max_main_prop.is_fixed() {
            resolve_length(max_main_prop, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero())
        } else {
            LayoutUnit::from_i32(33554431) // Max
        };
        let main_contribution_min = main_contribution_min.max_of(clamped_min_val).min_of(clamped_max_val);
        let main_contribution_max = main_contribution_max.max_of(clamped_min_val).min_of(clamped_max_val);

        sum_main_min = sum_main_min + main_contribution_min;
        sum_main_max = sum_main_max + main_contribution_max;
        max_main_min = max_main_min.max_of(main_contribution_min);
        max_cross_min = max_cross_min.max_of(cross_min);
        max_cross_max = max_cross_max.max_of(cross_max);

        items.push(ItemData {
            main_min: main_contribution_min,
            main_max: main_contribution_max,
            cross_min,
            cross_max,
        });
    }

    let item_count = items.len();

    // Add gaps between items
    let gap_count = if item_count > 1 { item_count - 1 } else { 0 };
    let total_main_gap = main_gap * gap_count as i32;

    // CSS Flexbox §9.9.1:
    // For single-line: min-content = sum of clamped flex-base sizes
    // For multi-line (wrap): min-content = largest item min-content contribution
    let (min_main, max_main) = if is_wrap {
        (max_main_min, sum_main_max + total_main_gap)
    } else {
        (sum_main_min + total_main_gap, sum_main_max + total_main_gap)
    };

    // For column+wrap, the cross-axis (inline) size depends on wrapping.
    // When the container has a definite main-axis constraint (height/max-height),
    // simulate wrapping to determine the sum of column widths.
    let (min_cross_total, max_cross_total) = if is_column && is_wrap && !items.is_empty() {
        let main_constraint = {
            let mut c = LayoutUnit::from_i32(33554431);
            if style.height.is_fixed() {
                let h = resolve_length(&style.height, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero());
                let ch = if style.box_sizing == BoxSizing::BorderBox {
                    (h - bp_block).clamp_negative_to_zero()
                } else { h };
                c = c.min_of(ch);
            }
            if !style.max_height.is_none() && style.max_height.is_fixed() {
                let mh = resolve_length(&style.max_height, LayoutUnit::zero(), LayoutUnit::zero(), LayoutUnit::zero());
                let cmh = if style.box_sizing == BoxSizing::BorderBox {
                    (mh - bp_block).clamp_negative_to_zero()
                } else { mh };
                c = c.min_of(cmh);
            }
            c
        };

        if main_constraint < LayoutUnit::from_i32(33554431) {
            // Simulate wrapping to compute cross-axis totals
            let simulate = |use_max: bool| -> LayoutUnit {
                let mut total_cross = LayoutUnit::zero();
                let mut line_main = LayoutUnit::zero();
                let mut line_cross = LayoutUnit::zero();
                let mut line_count: usize = 0;
                let mut items_in_line: usize = 0;

                for item in &items {
                    let im = if use_max { item.main_max } else { item.main_min };
                    let ic = if use_max { item.cross_max } else { item.cross_min };
                    let new_main = if items_in_line > 0 {
                        line_main + main_gap + im
                    } else { im };

                    if items_in_line > 0 && new_main > main_constraint {
                        total_cross = total_cross + line_cross;
                        line_count += 1;
                        line_main = im;
                        line_cross = ic;
                        items_in_line = 1;
                    } else {
                        line_main = new_main;
                        line_cross = line_cross.max_of(ic);
                        items_in_line += 1;
                    }
                }
                if items_in_line > 0 {
                    total_cross = total_cross + line_cross;
                    line_count += 1;
                }
                if line_count > 1 {
                    total_cross = total_cross + cross_gap * (line_count - 1) as i32;
                }
                total_cross
            };
            (simulate(false), simulate(true))
        } else {
            (max_cross_min, max_cross_max)
        }
    } else {
        (max_cross_min, max_cross_max)
    };

    // Convert back to inline/block coordinates
    let (min_inline, max_inline, min_block, max_block) = if is_column {
        (min_cross_total, max_cross_total, min_main, max_main)
    } else {
        (min_main, max_main, max_cross_min, max_cross_max)
    };

    IntrinsicSizes {
        min_content_inline_size: min_inline + bp_inline,
        max_content_inline_size: max_inline + bp_inline,
        min_content_block_size: min_block + bp_block,
        max_content_block_size: max_block + bp_block,
    }
}

/// Compute the intrinsic size contribution of a single child.
///
/// This accounts for the child's own intrinsic sizes plus its margin box.
/// For inline-level children (text, inline), uses inline intrinsic sizing.
pub fn compute_child_intrinsic_contribution(doc: &Document, child_id: NodeId) -> IntrinsicSizes {
    let child_style = &doc.node(child_id).style;
    let child_tag = doc.node(child_id).tag;

    // Resolve child margins (percentages resolve to zero for intrinsic sizing).
    let margin = resolve_margins(child_style, LayoutUnit::zero());
    let margin_inline = margin.inline_sum();
    let margin_block = margin.block_sum();

    // For text nodes and inline-level elements, use inline intrinsic sizing.
    // Block-size contribution requires running inline layout at the given
    // width to determine how many lines wrap.
    let child_intrinsic = if child_tag == ElementTag::Text || is_inline_level(child_style) {
        let inline_sizes = compute_intrinsic_inline_sizes(doc, child_id);
        IntrinsicSizes {
            min_content_inline_size: inline_sizes.min,
            max_content_inline_size: inline_sizes.max,
            // Block-size is zero for individual inline items at this level;
            // the container's block-size is computed from inline layout
            // in compute_intrinsic_block_sizes.
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
    // Intrinsic keywords in min/max-width (e.g. `max-width: max-content`) must
    // resolve against the CONTENT-BASED intrinsic sizes, not the specified width.
    // CSS Sizing 3 §4: intrinsic sizes are determined by the content.
    // However, for AR-derived widths (auto width + aspect-ratio + fixed height),
    // the transferred size IS the intrinsic size per CSS Sizing 4 §5.1.
    let intrinsic_for_keywords = if child_style.width.length_type() == openui_geometry::LengthType::Fixed {
        // Explicit width: intrinsic keywords resolve against content-based sizes.
        (child_intrinsic.min_content_inline_size, child_intrinsic.max_content_inline_size)
    } else {
        // Auto/intrinsic width (possibly AR-derived): use post-override values.
        (min_inline, max_inline)
    };
    let min_inline = apply_min_max_inline(child_style, min_inline, intrinsic_for_keywords);
    let max_inline = apply_min_max_inline(child_style, max_inline, intrinsic_for_keywords);

    // CSS Sizing 4 §5.1: For elements with AR and min-width:auto, the
    // automatic minimum in the ratio-dependent axis is the content-based
    // min-content. apply_size_override_inline may have replaced the
    // content-based intrinsic with the AR-derived size (which can be
    // smaller). Ensure the content min-content acts as a floor.
    let min_inline = if child_style.min_width.is_auto()
        && child_style.aspect_ratio.is_some()
        && (child_style.width.is_auto() || child_style.width.is_content_or_intrinsic())
    {
        min_inline.max_of(child_intrinsic.min_content_inline_size)
    } else {
        min_inline
    };
    let max_inline = if child_style.min_width.is_auto()
        && child_style.aspect_ratio.is_some()
        && (child_style.width.is_auto() || child_style.width.is_content_or_intrinsic())
    {
        max_inline.max_of(child_intrinsic.min_content_inline_size)
    } else {
        max_inline
    };

    // Apply explicit height if set. Compute separately for min-content
    // and max-content modes: at min-content width, wrapping content may
    // be taller than at max-content width (CSS Sizing 3 §5).
    let min_block_size = apply_size_override_block(child_style, child_intrinsic.min_content_block_size);
    let min_block_size = apply_min_max_block(child_style, min_block_size);
    let max_block_size = apply_size_override_block(child_style, child_intrinsic.max_content_block_size);
    let max_block_size = apply_min_max_block(child_style, max_block_size);

    // CSS Sizing 4 §5.1: When height is auto and the element has aspect-ratio,
    // the intrinsic block size is the transferred size from the resolved inline
    // size. apply_size_override_block only handles Fixed width; when width is
    // auto/intrinsic, derive block sizes from the already-resolved inline sizes.
    let (min_block_size, max_block_size) = if child_style.height.is_auto()
        && !child_style.width.is_fixed()
    {
        if let Some(ref ar) = child_style.aspect_ratio {
            if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
                let b = resolve_border(child_style);
                let p = resolve_padding(child_style, LayoutUnit::zero());
                let bp_inline = b.left + b.right + p.left + p.right;
                let bp_block = b.top + b.bottom + p.top + p.bottom;

                // Transfer inline → block through AR.
                // CSS Sizing 4: bare ratio respects box-sizing; auto ratio
                // maps through content-box.
                let ar_uses_border_box = !ar.auto_flag
                    && child_style.box_sizing == BoxSizing::BorderBox;

                let (transferred_min, transferred_max) = if ar_uses_border_box {
                    // AR maps border-box inline → border-box block
                    (
                        LayoutUnit::from_f32(min_inline.to_f32() * ar.ratio.1 / ar.ratio.0),
                        LayoutUnit::from_f32(max_inline.to_f32() * ar.ratio.1 / ar.ratio.0),
                    )
                } else {
                    // AR maps content-box: content_w → content_h, then add bp
                    let content_min_w = (min_inline - bp_inline).clamp_negative_to_zero();
                    let content_max_w = (max_inline - bp_inline).clamp_negative_to_zero();
                    (
                        LayoutUnit::from_f32(content_min_w.to_f32() * ar.ratio.1 / ar.ratio.0) + bp_block,
                        LayoutUnit::from_f32(content_max_w.to_f32() * ar.ratio.1 / ar.ratio.0) + bp_block,
                    )
                };

                // Transferred size replaces content-based size, then clamp.
                (
                    apply_min_max_block(child_style, transferred_min),
                    apply_min_max_block(child_style, transferred_max),
                )
            } else {
                (min_block_size, max_block_size)
            }
        } else {
            (min_block_size, max_block_size)
        }
    } else {
        (min_block_size, max_block_size)
    };

    // CSS Sizing 4 §5.1: Reverse AR transfer — when min-height (or explicit height)
    // inflates the block size beyond what was derived from inline, transfer back
    // through AR to inflate inline size.
    // The transferred values are still subject to the element's explicit max-width.
    let (min_inline, max_inline) = if let Some(ref ar) = child_style.aspect_ratio {
        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 && child_style.width.is_auto() {
            let b = resolve_border(child_style);
            let p = resolve_padding(child_style, LayoutUnit::zero());
            let bp_inline = b.left + b.right + p.left + p.right;
            let bp_block = b.top + b.bottom + p.top + p.bottom;

            let ar_uses_border_box = !ar.auto_flag
                && child_style.box_sizing == BoxSizing::BorderBox;

            let reverse_transfer = |block_bb: LayoutUnit| -> LayoutUnit {
                if ar_uses_border_box {
                    // AR maps border-box block → border-box inline
                    LayoutUnit::from_f32(block_bb.to_f32() * ar.ratio.0 / ar.ratio.1)
                } else {
                    let content_h = (block_bb - bp_block).clamp_negative_to_zero();
                    LayoutUnit::from_f32(content_h.to_f32() * ar.ratio.0 / ar.ratio.1) + bp_inline
                }
            };

            let transferred_min = reverse_transfer(min_block_size);
            let transferred_max = reverse_transfer(max_block_size);

            // Clamp transferred sizes by a fixed max-width constraint (not intrinsic
            // keywords — those are already handled by apply_min_max_inline using the
            // post-AR intrinsic sizes).
            let max_w_cap = if !child_style.max_width.is_none()
                && !child_style.max_width.is_auto()
                && !child_style.max_width.is_content_or_intrinsic()
            {
                let max_w_raw = resolve_length(
                    &child_style.max_width,
                    openui_geometry::INDEFINITE_SIZE,
                    LayoutUnit::max(),
                    LayoutUnit::max(),
                );
                if max_w_raw < LayoutUnit::max() {
                    if child_style.box_sizing == BoxSizing::ContentBox {
                        max_w_raw + bp_inline
                    } else {
                        max_w_raw.max_of(bp_inline)
                    }
                } else {
                    LayoutUnit::max()
                }
            } else {
                LayoutUnit::max()
            };

            let clamped_min = min_inline.max_of(transferred_min).min_of(max_w_cap);
            let clamped_max = max_inline.max_of(transferred_max).min_of(max_w_cap);

            (clamped_min, clamped_max)
        } else {
            (min_inline, max_inline)
        }
    } else {
        (min_inline, max_inline)
    };

    IntrinsicSizes {
        min_content_inline_size: min_inline + margin_inline,
        max_content_inline_size: max_inline + margin_inline,
        min_content_block_size: min_block_size + margin_block,
        max_content_block_size: max_block_size + margin_block,
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
    // Only consider in-flow children (skip display:none and absolutely positioned).
    let children: Vec<NodeId> = doc.children(node_id)
        .filter(|&id| {
            let s = &doc.node(id).style;
            s.display != openui_style::Display::None
                && !s.position.is_absolutely_positioned()
                && s.float == openui_style::Float::None
        })
        .collect();
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

    // Determine the effective aspect ratio for deriving the missing dimension.
    // CSS Sizing 4: If a CSS `aspect-ratio` is specified (without `auto`), it
    // overrides the natural ratio. With `auto <ratio>`, the natural ratio
    // (from the element's intrinsic dimensions) takes priority.
    let (ratio_w, ratio_h) = if let Some(ref ar) = style.aspect_ratio {
        if ar.auto_flag {
            // `auto <ratio>`: prefer the natural ratio (default 2:1).
            (default_width, default_height)
        } else if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
            // Bare `<ratio>`: override natural ratio with specified one.
            (LayoutUnit::from_f32(ar.ratio.0), LayoutUnit::from_f32(ar.ratio.1))
        } else {
            (default_width, default_height)
        }
    } else {
        (default_width, default_height)
    };

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
            // Derive height from aspect ratio.
            let h = apply_aspect_ratio(w, ratio_w, ratio_h);
            (w, h)
        }
        (false, true) => {
            let h = LayoutUnit::from_f32(style.height.value());
            // Derive width from aspect ratio.
            let w = apply_aspect_ratio_inverse(h, ratio_w, ratio_h);
            (w, h)
        }
        (false, false) => {
            // No explicit dimensions. Use natural width with the effective
            // ratio to derive height, ensuring they're consistent.
            let h = apply_aspect_ratio(default_width, ratio_w, ratio_h);
            (default_width, h)
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
/// If the element has an explicit fixed width, use it (as border-box);
/// otherwise return the intrinsic value (already border-box).
///
/// The returned value must be border-box because the intrinsic sizes from
/// `compute_intrinsic_block_sizes` include border+padding. Converting the
/// explicit width to border-box ensures consistent units throughout the
/// intrinsic sizing pipeline.
pub fn apply_size_override_inline(style: &ComputedStyle, intrinsic: LayoutUnit) -> LayoutUnit {
    if style.width.length_type() == openui_geometry::LengthType::Fixed {
        let raw = LayoutUnit::from_f32(style.width.value());
        let bp_val = {
            let b = resolve_border(style);
            let p = resolve_padding(style, LayoutUnit::zero());
            b.left + b.right + p.left + p.right
        };
        if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(bp_val)
        } else {
            raw + bp_val
        }
    } else if style.width.is_auto() || style.width.is_content_or_intrinsic() {
        // CSS Sizing 4 §5.1: When width is auto (or an intrinsic keyword like
        // min-content/max-content) and the element has aspect-ratio + definite
        // height, compute width from height × ratio. For intrinsic keywords,
        // the transferred size replaces the content-based intrinsic size.
        if let Some(ref ar) = style.aspect_ratio {
            if style.height.length_type() == openui_geometry::LengthType::Fixed
                && ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0
            {
                let b = resolve_border(style);
                let p = resolve_padding(style, LayoutUnit::zero());
                let bp_inline = b.left + b.right + p.left + p.right;
                let bp_block = b.top + b.bottom + p.top + p.bottom;

                // CSS Sizing 4: when `auto <ratio>`, AR maps through
                // content-box. Bare `<ratio>` respects box-sizing.
                let ar_uses_border_box = !ar.auto_flag
                    && style.box_sizing == BoxSizing::BorderBox;

                // Get raw height and clamp by min-height / max-height.
                let h_raw = LayoutUnit::from_f32(style.height.value());
                let indefinite = openui_geometry::INDEFINITE_SIZE;
                let min_h = resolve_length(
                    &style.min_height, indefinite,
                    LayoutUnit::zero(), LayoutUnit::zero(),
                );
                let max_h = resolve_length(
                    &style.max_height, indefinite,
                    LayoutUnit::max(), LayoutUnit::max(),
                );
                let h_clamped = if style.box_sizing == BoxSizing::BorderBox {
                    let h_bb = h_raw.max_of(bp_block);
                    let min_h_bb = if min_h > LayoutUnit::zero() { min_h.max_of(bp_block) } else { LayoutUnit::zero() };
                    let max_h_bb = if max_h < LayoutUnit::max() { max_h.max_of(bp_block) } else { LayoutUnit::max() };
                    h_bb.clamp(min_h_bb, max_h_bb)
                } else {
                    h_raw.clamp(min_h, max_h)
                };

                if ar_uses_border_box {
                    // AR applies to border-box: w_bb = h_bb × ratio
                    LayoutUnit::from_f32(h_clamped.to_f32() * ar.ratio.0 / ar.ratio.1)
                } else {
                    // AR applies to content-box: content_w = content_h × ratio
                    let content_h = if style.box_sizing == BoxSizing::BorderBox {
                        (h_clamped - bp_block).clamp_negative_to_zero()
                    } else {
                        h_clamped
                    };
                    LayoutUnit::from_f32(content_h.to_f32() * ar.ratio.0 / ar.ratio.1) + bp_inline
                }
            } else {
                intrinsic
            }
        } else {
            intrinsic
        }
    } else {
        intrinsic
    }
}

/// If the element has an explicit fixed height, use it (as border-box);
/// otherwise return the intrinsic value (already border-box).
fn apply_size_override_block(style: &ComputedStyle, intrinsic: LayoutUnit) -> LayoutUnit {
    if style.height.length_type() == openui_geometry::LengthType::Fixed {
        let raw = LayoutUnit::from_f32(style.height.value());
        let bp_val = {
            let b = resolve_border(style);
            let p = resolve_padding(style, LayoutUnit::zero());
            b.top + b.bottom + p.top + p.bottom
        };
        if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(bp_val)
        } else {
            raw + bp_val
        }
    } else if style.height.is_auto() {
        // CSS Sizing 4 §5.1: When height is auto and the element has
        // aspect-ratio + definite width, compute height from width × ratio.
        if let Some(ref ar) = style.aspect_ratio {
            if style.width.length_type() == openui_geometry::LengthType::Fixed
                && ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0
            {
                let b = resolve_border(style);
                let p = resolve_padding(style, LayoutUnit::zero());
                let bp_inline = b.left + b.right + p.left + p.right;
                let bp_block = b.top + b.bottom + p.top + p.bottom;

                let w_raw = LayoutUnit::from_f32(style.width.value());
                let content_w = if style.box_sizing == BoxSizing::BorderBox {
                    (w_raw - bp_inline).clamp_negative_to_zero()
                } else {
                    w_raw
                };
                let content_h = LayoutUnit::from_f32(
                    content_w.to_f32() * ar.ratio.1 / ar.ratio.0,
                );
                content_h + bp_block
            } else {
                intrinsic
            }
        } else {
            intrinsic
        }
    } else {
        intrinsic
    }
}

/// Clamp a resolved border-box inline size by min-width / max-width.
///
/// The `size` parameter is in border-box units (from `apply_size_override_inline`
/// or from `compute_intrinsic_block_sizes` which includes border+padding).
/// Min/max values must be converted to border-box before clamping when
/// box-sizing is content-box.
fn apply_min_max_inline(
    style: &ComputedStyle,
    size: LayoutUnit,
    base_intrinsic: (LayoutUnit, LayoutUnit),  // (min-content, max-content) in border-box
) -> LayoutUnit {
    let zero = LayoutUnit::zero();

    // Percentage min/max resolve against the containing block's inline size.
    // In intrinsic sizing there is no containing block, so use INDEFINITE_SIZE
    // to trigger the auto fallback in resolve_length (CSS Sizing 3 §5.1:
    // percentage sizes against indefinite bases are treated as auto).
    let indefinite = openui_geometry::INDEFINITE_SIZE;

    // Resolve min-width, handling intrinsic keywords.
    let min_raw = if style.min_width.is_content_or_intrinsic() {
        match style.min_width.length_type() {
            LengthType::MinContent => base_intrinsic.0,
            LengthType::MaxContent => base_intrinsic.1,
            _ => base_intrinsic.0, // fit-content → min-content for min sizing
        }
    } else {
        resolve_length(
            &style.min_width, indefinite,
            zero, // auto min-width = 0
            zero,
        )
    };
    // Resolve max-width, handling intrinsic keywords.
    let max_raw = if style.max_width.is_content_or_intrinsic() {
        match style.max_width.length_type() {
            LengthType::MinContent => base_intrinsic.0,
            LengthType::MaxContent => base_intrinsic.1,
            _ => base_intrinsic.1, // fit-content → max-content for max sizing
        }
    } else {
        resolve_length(
            &style.max_width, indefinite,
            LayoutUnit::max(), // auto = unconstrained
            LayoutUnit::max(), // none = unconstrained
        )
    };

    // Compute border+padding for conversion and floor.
    let bp_val = {
        let b = resolve_border(style);
        let p = resolve_padding(style, zero);
        b.left + b.right + p.left + p.right
    };

    // Convert to border-box units to match the size being clamped.
    // For content-box: add bp. For border-box: floor at bp.
    let min_bb = if min_raw > zero {
        if style.box_sizing == BoxSizing::ContentBox {
            min_raw + bp_val
        } else {
            min_raw.max_of(bp_val)
        }
    } else {
        zero
    };
    let max_bb = if max_raw == LayoutUnit::max() {
        max_raw
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + bp_val
    } else {
        max_raw.max_of(bp_val)
    };

    // CSS Sizing 4 §5.2: transferred min/max through aspect-ratio.
    // If min-height or max-height is definite and AR is present, transfer to inline axis.
    let (mut min_bb, mut max_bb) = (min_bb, max_bb);
    if let Some(ref ar) = style.aspect_ratio {
        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
            let bp_block = {
                let b = resolve_border(style);
                let p = resolve_padding(style, zero);
                b.top + b.bottom + p.top + p.bottom
            };

            // CSS Sizing 4: bare ratio respects box-sizing; auto ratio
            // always maps through content-box.
            let ar_uses_border_box = !ar.auto_flag
                && style.box_sizing == BoxSizing::BorderBox;

            // Transfer min-height → min-width (only if min-width is auto/0)
            let min_h_raw = resolve_length(
                &style.min_height, indefinite, zero, zero,
            );
            if min_h_raw > zero && min_bb == zero {
                if ar_uses_border_box {
                    min_bb = LayoutUnit::from_f32(
                        min_h_raw.to_f32() * ar.ratio.0 / ar.ratio.1,
                    );
                } else {
                    let content_min_h = if style.box_sizing == BoxSizing::BorderBox {
                        (min_h_raw - bp_block).clamp_negative_to_zero()
                    } else {
                        min_h_raw
                    };
                    min_bb = LayoutUnit::from_f32(
                        content_min_h.to_f32() * ar.ratio.0 / ar.ratio.1,
                    ) + bp_val;
                }
            }

            // Transfer max-height → max-width (only if max-width is unconstrained)
            let max_h_raw = resolve_length(
                &style.max_height, indefinite, LayoutUnit::max(), LayoutUnit::max(),
            );
            if max_h_raw < LayoutUnit::max() && max_bb == LayoutUnit::max() {
                if ar_uses_border_box {
                    max_bb = LayoutUnit::from_f32(
                        max_h_raw.to_f32() * ar.ratio.0 / ar.ratio.1,
                    );
                } else {
                    let content_max_h = if style.box_sizing == BoxSizing::BorderBox {
                        (max_h_raw - bp_block).clamp_negative_to_zero()
                    } else {
                        max_h_raw
                    };
                    max_bb = LayoutUnit::from_f32(
                        content_max_h.to_f32() * ar.ratio.0 / ar.ratio.1,
                    ) + bp_val;
                }
            }
        }
    }

    size.clamp(min_bb, max_bb)
}

/// Clamp a resolved border-box block size by min-height / max-height.
fn apply_min_max_block(style: &ComputedStyle, size: LayoutUnit) -> LayoutUnit {
    let zero = LayoutUnit::zero();

    // Same as apply_min_max_inline: use INDEFINITE_SIZE for percentage base
    // so percentage min/max-height resolves to auto values (CSS Sizing 3 §5.1).
    let indefinite = openui_geometry::INDEFINITE_SIZE;

    let min_raw = resolve_length(
        &style.min_height, indefinite,
        zero,
        zero,
    );
    let max_raw = resolve_length(
        &style.max_height, indefinite,
        LayoutUnit::max(),
        LayoutUnit::max(),
    );

    let bp_val = {
        let b = resolve_border(style);
        let p = resolve_padding(style, zero);
        b.top + b.bottom + p.top + p.bottom
    };

    let min_bb = if min_raw > zero {
        if style.box_sizing == BoxSizing::ContentBox {
            min_raw + bp_val
        } else {
            min_raw.max_of(bp_val)
        }
    } else {
        zero
    };
    let max_bb = if max_raw == LayoutUnit::max() {
        max_raw
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + bp_val
    } else {
        max_raw.max_of(bp_val)
    };

    // CSS Sizing 4 §5.2: transferred min/max through aspect-ratio.
    // If min-width or max-width is definite and AR is present, transfer to block axis.
    let (mut min_bb, mut max_bb) = (min_bb, max_bb);
    if let Some(ref ar) = style.aspect_ratio {
        if ar.ratio.0 != 0.0 && ar.ratio.1 != 0.0 {
            let bp_inline = {
                let b = resolve_border(style);
                let p = resolve_padding(style, zero);
                b.left + b.right + p.left + p.right
            };

            // Transfer min-width → min-height (only if min-height is auto/0)
            let min_w_raw = resolve_length(
                &style.min_width, indefinite, zero, zero,
            );
            if min_w_raw > zero && min_bb == zero {
                if style.box_sizing == BoxSizing::BorderBox {
                    // AR applies to border-box: border_box_h = border_box_w * h/w
                    min_bb = LayoutUnit::from_f32(
                        min_w_raw.to_f32() * ar.ratio.1 / ar.ratio.0,
                    );
                } else {
                    let transferred_min_h = LayoutUnit::from_f32(
                        min_w_raw.to_f32() * ar.ratio.1 / ar.ratio.0,
                    );
                    min_bb = transferred_min_h + bp_val;
                }
            }

            // Transfer max-width → max-height (only if max-height is unconstrained)
            let max_w_raw = resolve_length(
                &style.max_width, indefinite, LayoutUnit::max(), LayoutUnit::max(),
            );
            if max_w_raw < LayoutUnit::max() && max_bb == LayoutUnit::max() {
                if style.box_sizing == BoxSizing::BorderBox {
                    max_bb = LayoutUnit::from_f32(
                        max_w_raw.to_f32() * ar.ratio.1 / ar.ratio.0,
                    );
                } else {
                    let transferred_max_h = LayoutUnit::from_f32(
                        max_w_raw.to_f32() * ar.ratio.1 / ar.ratio.0,
                    );
                    max_bb = transferred_max_h + bp_val;
                }
            }
        }
    }

    size.clamp(min_bb, max_bb)
}
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
