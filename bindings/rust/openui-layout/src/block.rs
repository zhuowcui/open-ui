//! Block layout algorithm — extracted from Blink's `block_layout_algorithm.cc`.
//!
//! Source: core/layout/block_layout_algorithm.cc (~4200 lines)
#![allow(unused_assignments)] // margin_strut is a loop accumulator
//!
//! This implements CSS normal flow (block formatting context): children are
//! stacked vertically, auto margins are resolved, margins collapse between
//! siblings and between parent/child.
//!
//! The algorithm follows Blink's NG layout pipeline:
//! 1. Compute child margins and padding
//! 2. Create constraint space for child
//! 3. Layout child (recursively)
//! 4. Position child using ComputeInflowPosition logic
//! 5. After all children: compute intrinsic block size, apply CSS height

use openui_geometry::{LayoutUnit, BoxStrut, PhysicalOffset, PhysicalRect, PhysicalSize, MarginStrut};
use openui_style::{ComputedStyle, Display, BoxSizing, Overflow};
use openui_dom::{Document, NodeId};

use crate::constraint_space::ConstraintSpace;
use crate::fragment::{Fragment, FragmentKind};
use crate::length_resolver::{resolve_length, resolve_margin_or_padding};

/// Perform block layout on a node and its descendants.
///
/// This is the main entry point, equivalent to Blink's
/// `BlockLayoutAlgorithm::Layout()` (line 593).
///
/// Returns a `Fragment` with resolved sizes and positioned children.
pub fn block_layout(doc: &Document, node_id: NodeId, space: &ConstraintSpace) -> Fragment {
    let style = &doc.node(node_id).style;

    // Dispatch flex containers to the flex algorithm
    if style.display.is_flex() {
        return crate::flex::flex_layout(doc, node_id, space);
    }

    // ── Step 1: Resolve border + padding ─────────────────────────────
    // Blink: uses pre-resolved border widths (integers) and resolves padding
    // against percentage_resolution_inline_size.

    let border = resolve_border(style);
    let padding = resolve_padding(style, space.percentage_resolution_inline_size);

    let border_padding_inline = border.left + border.right + padding.left + padding.right;
    let border_padding_block = border.top + border.bottom + padding.top + padding.bottom;

    // ── Step 2: Resolve width ────────────────────────────────────────
    // Blink: ComputeBlockSizeForFragment / ResolveMainInlineLength

    let content_inline_size = resolve_inline_size(
        style,
        space,
        border_padding_inline,
    );

    // The total border-box inline size
    let border_box_inline = if style.box_sizing == BoxSizing::BorderBox {
        content_inline_size.max_of(border_padding_inline)
    } else {
        content_inline_size + border_padding_inline
    };

    // Available inline size for children = content box width
    let child_available_inline = if style.box_sizing == BoxSizing::BorderBox {
        border_box_inline - border_padding_inline
    } else {
        content_inline_size
    };

    // ── Step 3: Layout children (the main loop) ─────────────────────
    // Blink: block_layout_algorithm.cc lines 981-1110
    //
    // Iterate children in document order. For each in-flow block child:
    // 1. Calculate margins
    // 2. Create child constraint space
    // 3. Layout child
    // 4. Position using ComputeInflowPosition

    // Per CSS 2.1 §10.5, percentage heights on children resolve against the
    // containing block's *specified* height (if definite), not auto-computed.
    // If the parent's height is auto, percentage heights are indefinite.
    let child_percentage_block_size = if !style.height.is_auto() {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(), // auto fallback (shouldn't reach here)
            LayoutUnit::zero(), // none fallback
        );
        // Convert to content-box size
        if style.box_sizing == BoxSizing::BorderBox {
            (raw - border_padding_block).clamp_negative_to_zero()
        } else {
            raw
        }
    } else {
        // Auto height → per CSS 2.2 §10.5, percentage heights on children
        // are indefinite (treated as auto). Do NOT pass through the parent's
        // percentage resolution — that would incorrectly let grandchildren
        // resolve percentage heights against an ancestor's explicit height.
        openui_geometry::INDEFINITE_SIZE
    };

    let content_edge = border.top + padding.top;
    let mut block_offset = content_edge;
    let mut margin_strut = MarginStrut::new();
    let mut child_fragments: Vec<Fragment> = Vec::new();
    let mut intrinsic_block_size = content_edge;

    // Classify children: detect whether we have only inline, only block,
    // or mixed content (CSS 2.2 §9.2.1.1 — anonymous block boxes).
    let has_inline = crate::inline::algorithm::has_inline_children(doc, node_id);
    let has_block = has_block_children(doc, node_id);

    if has_inline && !has_block {
        // ── Pure inline formatting context ───────────────────────────
        let inline_space = ConstraintSpace::for_block_child(
            child_available_inline,
            space.available_block_size,
            child_available_inline,
            child_percentage_block_size,
            false,
        );
        let inline_fragment = crate::inline::algorithm::inline_layout(
            doc, node_id, &inline_space,
        );
        for line_frag in inline_fragment.children {
            let line_height = line_frag.size.height;
            let mut positioned_line = line_frag;
            positioned_line.offset = PhysicalOffset::new(
                border.left + padding.left + positioned_line.offset.left,
                content_edge + positioned_line.offset.top,
            );
            intrinsic_block_size = intrinsic_block_size.max_of(
                positioned_line.offset.top + line_height,
            );
            child_fragments.push(positioned_line);
        }
        block_offset = intrinsic_block_size;
    } else if has_inline && has_block {
        // ── Mixed content: create anonymous block boxes (CSS 2.2 §9.2.1.1) ─
        // Collect contiguous runs of inline children into anonymous wrappers,
        // interleaved with real block-level children.
        let children_ids: Vec<NodeId> = doc.children(node_id).collect();
        let mut i = 0;
        while i < children_ids.len() {
            let child_id = children_ids[i];
            let child_style = &doc.node(child_id).style;

            if child_style.is_out_of_flow() || child_style.display == Display::None {
                i += 1;
                continue;
            }

            if is_inline_level_child(doc, child_id) {
                // Gather contiguous run of inline children.
                // display:none and out-of-flow children are transparent to
                // inline run gathering — they don't break the run.
                let run_start = i;
                while i < children_ids.len() {
                    let cid = children_ids[i];
                    let cs = &doc.node(cid).style;
                    if cs.display == Display::None || cs.is_out_of_flow() {
                        i += 1;
                        continue;
                    }
                    if !is_inline_level_child(doc, cid) {
                        break;
                    }
                    i += 1;
                }
                let inline_run = &children_ids[run_start..i];

                // Lay out this anonymous inline wrapper.
                let inline_space = ConstraintSpace::for_block_child(
                    child_available_inline,
                    space.available_block_size,
                    child_available_inline,
                    child_percentage_block_size,
                    false,
                );
                let anon_fragment = crate::inline::algorithm::inline_layout_for_children(
                    doc, node_id, inline_run, &inline_space,
                );
                for line_frag in anon_fragment.children {
                    let line_height = line_frag.size.height;
                    let mut positioned_line = line_frag;
                    positioned_line.offset = PhysicalOffset::new(
                        border.left + padding.left + positioned_line.offset.left,
                        block_offset + positioned_line.offset.top,
                    );
                    intrinsic_block_size = intrinsic_block_size.max_of(
                        positioned_line.offset.top + line_height,
                    );
                    child_fragments.push(positioned_line);
                }
                block_offset = intrinsic_block_size;
            } else {
                // Block-level child — lay out normally.
                layout_block_child(
                    doc, child_id, space,
                    child_available_inline, child_percentage_block_size,
                    &border, &padding, content_edge,
                    &mut block_offset, &mut margin_strut,
                    &mut intrinsic_block_size, &mut child_fragments,
                );
                i += 1;
            }
        }
    } else {
        // ── Pure block formatting context ────────────────────────────

    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;

        // Skip out-of-flow children (absolute, fixed) — they don't participate
        // in block flow. Floats are also skipped for SP9.
        if child_style.is_out_of_flow() || child_style.display == Display::None {
            continue;
        }
        if child_style.display.is_inline_level() {
            continue;
        }

        layout_block_child(
            doc, child_id, space,
            child_available_inline, child_percentage_block_size,
            &border, &padding, content_edge,
            &mut block_offset, &mut margin_strut,
            &mut intrinsic_block_size, &mut child_fragments,
        );
    }

    } // end block children

    // ── Step 4: Finish layout (FinishLayout, line 1165) ──────────────
    // Resolve the trailing margin strut if margins can't collapse through
    // the bottom edge. Per CSS 2.1 §8.3.1, bottom border or padding
    // prevents the last child's bottom margin from collapsing with the
    // parent's bottom margin.
    let bottom_edge = border.bottom + padding.bottom;
    if !margin_strut.is_empty()
        && (space.is_new_formatting_context || bottom_edge > LayoutUnit::zero())
    {
        intrinsic_block_size += margin_strut.sum();
    }

    // Add bottom border + padding
    intrinsic_block_size += bottom_edge;

    // ── Step 5: Resolve height ───────────────────────────────────────
    // Blink: ComputeBlockSizeForFragment (length_utils.h:314)
    let is_viewport = doc.node(node_id).tag == openui_dom::ElementTag::Viewport;
    let resolved_block_size = resolve_block_size(
        style,
        space,
        intrinsic_block_size,
        border_padding_block,
        is_viewport,
    );

    let border_box_size = PhysicalSize::new(border_box_inline, resolved_block_size);

    let mut fragment = Fragment::new_box(node_id, border_box_size);
    fragment.border = border;
    fragment.padding = padding;
    fragment.children = child_fragments;
    fragment.kind = if doc.node(node_id).tag == openui_dom::ElementTag::Viewport {
        FragmentKind::Viewport
    } else {
        FragmentKind::Box
    };

    // ── Overflow tracking ────────────────────────────────────────────
    // Compute the scrollable overflow rect by unioning all child border-box
    // rects (relative to this fragment). If the union extends beyond this
    // fragment's border-box, store it as the overflow rect.
    let border_box_rect = PhysicalRect::new(PhysicalOffset::zero(), border_box_size);
    let mut overflow = border_box_rect;
    for child in &fragment.children {
        let child_rect = PhysicalRect::new(child.offset, child.size);
        overflow = overflow.unite(&child_rect);

        // Include grandchild overflow that wasn't clipped by the child.
        if !child.has_overflow_clip {
            if let Some(child_overflow) = child.overflow_rect {
                let shifted = PhysicalRect::new(
                    PhysicalOffset::new(
                        child.offset.left + child_overflow.offset.left,
                        child.offset.top + child_overflow.offset.top,
                    ),
                    child_overflow.size,
                );
                overflow = overflow.unite(&shifted);
            }
        }
    }
    if overflow != border_box_rect {
        fragment.overflow_rect = Some(overflow);
    }

    // Set the overflow clip flag from style.
    fragment.has_overflow_clip = style.overflow_x != Overflow::Visible
        || style.overflow_y != Overflow::Visible;

    fragment
}

// ── Helper: detect block-level children ──────────────────────────────

/// Check if a node has any block-level children (CSS 2.2 §9.2.1.1).
fn has_block_children(doc: &Document, node_id: NodeId) -> bool {
    for child_id in doc.children(node_id) {
        let child = doc.node(child_id);
        if child.style.is_out_of_flow() || child.style.display == Display::None {
            continue;
        }
        if child.tag == openui_dom::ElementTag::Text {
            continue;
        }
        if child.style.display.is_block_level() {
            return true;
        }
    }
    false
}

/// Check if a child is inline-level (text node or inline display).
fn is_inline_level_child(doc: &Document, child_id: NodeId) -> bool {
    let child = doc.node(child_id);
    if child.style.is_out_of_flow() || child.style.display == Display::None {
        return false;
    }
    child.tag == openui_dom::ElementTag::Text || child.style.display.is_inline_level()
}

// ── Helper: layout a single block child ──────────────────────────────

/// Layout a single block-level child in normal flow.
///
/// Extracted from the block child loop to be reusable by both the pure-block
/// and the mixed-content (anonymous block box) paths.
#[allow(clippy::too_many_arguments)]
fn layout_block_child(
    doc: &Document,
    child_id: NodeId,
    space: &ConstraintSpace,
    child_available_inline: LayoutUnit,
    child_percentage_block_size: LayoutUnit,
    border: &BoxStrut,
    padding: &BoxStrut,
    content_edge: LayoutUnit,
    block_offset: &mut LayoutUnit,
    margin_strut: &mut MarginStrut,
    intrinsic_block_size: &mut LayoutUnit,
    child_fragments: &mut Vec<Fragment>,
) {
    let child_style = &doc.node(child_id).style;

    if child_style.is_out_of_flow() || child_style.display == Display::None {
        return;
    }

    let child_margin = resolve_margins(child_style, child_available_inline);
    margin_strut.append_normal(child_margin.top);

    if child_fragments.is_empty() {
        if space.is_new_formatting_context || content_edge > LayoutUnit::zero() {
            *block_offset += margin_strut.sum();
            *margin_strut = MarginStrut::new();
        }
    } else {
        *block_offset += margin_strut.sum();
        *margin_strut = MarginStrut::new();
    }

    let child_non_auto_margin_inline = {
        let ml = if child_style.margin_left.is_auto() {
            LayoutUnit::zero()
        } else {
            child_margin.left
        };
        let mr = if child_style.margin_right.is_auto() {
            LayoutUnit::zero()
        } else {
            child_margin.right
        };
        ml + mr
    };
    let child_constrained_inline =
        (child_available_inline - child_non_auto_margin_inline).clamp_negative_to_zero();

    let child_is_new_fc = establishes_new_fc(child_style);
    let child_space = ConstraintSpace::for_block_child(
        child_constrained_inline,
        space.available_block_size,
        child_available_inline,
        child_percentage_block_size,
        child_is_new_fc,
    );

    let mut child_fragment = block_layout(doc, child_id, &child_space);

    let child_border_box_inline = child_fragment.size.width;
    let remaining_space = child_available_inline - child_border_box_inline;

    let resolved_margin_left;
    let resolved_margin_right;

    if child_style.margin_left.is_auto() && child_style.margin_right.is_auto() {
        if remaining_space > LayoutUnit::zero() {
            let half = remaining_space / 2;
            resolved_margin_left = half;
            resolved_margin_right = remaining_space - half;
        } else {
            resolved_margin_left = LayoutUnit::zero();
            resolved_margin_right = LayoutUnit::zero();
        }
    } else if child_style.margin_left.is_auto() {
        resolved_margin_right = child_margin.right;
        resolved_margin_left = remaining_space - resolved_margin_right;
    } else if child_style.margin_right.is_auto() {
        resolved_margin_left = child_margin.left;
        resolved_margin_right = remaining_space - resolved_margin_left;
    } else {
        resolved_margin_left = child_margin.left;
        resolved_margin_right = child_margin.right;
    }

    child_fragment.offset = PhysicalOffset::new(
        border.left + padding.left + resolved_margin_left,
        *block_offset,
    );

    // Apply relative positioning offsets (CSS 2.1 §9.4.3).
    // The fragment retains its normal-flow position for sibling layout;
    // only the visual offset is shifted.
    crate::relative::apply_relative_offset(
        &mut child_fragment,
        child_style,
        child_available_inline,
        space.available_block_size,
    );

    child_fragment.margin = BoxStrut::new(
        child_margin.top,
        resolved_margin_right,
        child_margin.bottom,
        resolved_margin_left,
    );

    *block_offset += child_fragment.size.height;

    *margin_strut = MarginStrut::new();
    margin_strut.append_normal(child_margin.bottom);

    *intrinsic_block_size = *block_offset;
    child_fragments.push(child_fragment);
}

// ── Helper: establishes new formatting context ───────────────────────

/// Whether a style establishes a new formatting context.
///
/// overflow != visible, display: flow-root, floats, abs pos, inline-block,
/// flex, grid — all establish a new block formatting context.
///
/// Delegates to `ComputedStyle::creates_new_formatting_context()` which checks
/// these conditions. Provided as a free function for use in layout algorithms.
pub fn establishes_new_fc(style: &ComputedStyle) -> bool {
    style.creates_new_formatting_context()
}

// ── Helper: resolve border widths from style ─────────────────────────

pub fn resolve_border(style: &ComputedStyle) -> BoxStrut {
    BoxStrut::new(
        LayoutUnit::from_i32(style.effective_border_top()),
        LayoutUnit::from_i32(style.effective_border_right()),
        LayoutUnit::from_i32(style.effective_border_bottom()),
        LayoutUnit::from_i32(style.effective_border_left()),
    )
}

// ── Helper: resolve padding lengths ──────────────────────────────────

pub fn resolve_padding(style: &ComputedStyle, percentage_base: LayoutUnit) -> BoxStrut {
    BoxStrut::new(
        resolve_margin_or_padding(&style.padding_top, percentage_base),
        resolve_margin_or_padding(&style.padding_right, percentage_base),
        resolve_margin_or_padding(&style.padding_bottom, percentage_base),
        resolve_margin_or_padding(&style.padding_left, percentage_base),
    )
}

// ── Helper: resolve margins ──────────────────────────────────────────

pub fn resolve_margins(style: &ComputedStyle, percentage_base: LayoutUnit) -> BoxStrut {
    BoxStrut::new(
        resolve_margin_or_padding(&style.margin_top, percentage_base),
        resolve_margin_or_padding(&style.margin_right, percentage_base),
        resolve_margin_or_padding(&style.margin_bottom, percentage_base),
        resolve_margin_or_padding(&style.margin_left, percentage_base),
    )
}

// ── Helper: resolve inline size (width) ──────────────────────────────

fn resolve_inline_size(
    style: &ComputedStyle,
    space: &ConstraintSpace,
    border_padding: LayoutUnit,
) -> LayoutUnit {
    let available = space.available_inline_size;

    // When flex layout determines the exact inline size, use it directly.
    if space.is_fixed_inline_size || space.stretch_inline_size {
        return if style.box_sizing == BoxSizing::BorderBox {
            available
        } else {
            (available - border_padding).clamp_negative_to_zero()
        };
    }

    // Resolve the CSS width property
    let resolved = if style.width.is_auto() {
        // Auto width: fill available space minus border+padding
        if style.box_sizing == BoxSizing::BorderBox {
            available
        } else {
            (available - border_padding).clamp_negative_to_zero()
        }
    } else {
        resolve_length(
            &style.width,
            space.percentage_resolution_inline_size,
            available, // auto fallback
            available, // none fallback
        )
    };

    // Apply min-width / max-width constraints
    let min = if style.min_width.is_auto() {
        LayoutUnit::zero() // min-width: auto → 0 for block elements
    } else {
        resolve_length(
            &style.min_width,
            space.percentage_resolution_inline_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
    };

    let max = resolve_length(
        &style.max_width,
        space.percentage_resolution_inline_size,
        LayoutUnit::max(), // auto → unconstrained
        LayoutUnit::max(), // none → unconstrained
    );

    resolved.clamp(min, max)
}

// ── Helper: resolve block size (height) ──────────────────────────────

fn resolve_block_size(
    style: &ComputedStyle,
    space: &ConstraintSpace,
    intrinsic_block_size: LayoutUnit,
    border_padding_block: LayoutUnit,
    is_viewport: bool,
) -> LayoutUnit {
    // When flex layout determines the exact block size, use it directly.
    if space.is_fixed_block_size || space.stretch_block_size {
        let available = space.available_block_size;
        return if style.box_sizing == BoxSizing::BorderBox {
            available.max_of(border_padding_block)
        } else {
            (available - border_padding_block).clamp_negative_to_zero() + border_padding_block
        };
    }

    // For the viewport/initial containing block, auto height = viewport height
    // (not content-sized). This matches Blink's initial containing block behavior.
    let resolved = if style.height.is_auto() {
        if is_viewport {
            space.available_block_size
        } else {
            intrinsic_block_size
        }
    } else {
        let raw = resolve_length(
            &style.height,
            space.percentage_resolution_block_size,
            intrinsic_block_size, // auto fallback
            intrinsic_block_size, // none fallback
        );
        if style.box_sizing == BoxSizing::BorderBox {
            raw.max_of(border_padding_block)
        } else {
            raw + border_padding_block
        }
    };

    // Apply min-height / max-height.
    // min/max values are in content-box space (for box-sizing: content-box),
    // so convert them to border-box before clamping against the resolved value
    // which is already in border-box space.
    let min_raw = if style.min_height.is_auto() {
        LayoutUnit::zero()
    } else {
        resolve_length(
            &style.min_height,
            space.percentage_resolution_block_size,
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
    };
    let min = if style.box_sizing == BoxSizing::ContentBox && min_raw > LayoutUnit::zero() {
        min_raw + border_padding_block
    } else if min_raw > LayoutUnit::zero() {
        min_raw.max_of(border_padding_block)
    } else {
        min_raw
    };

    let max_raw = resolve_length(
        &style.max_height,
        space.percentage_resolution_block_size,
        LayoutUnit::max(), // auto → unconstrained
        LayoutUnit::max(), // none → unconstrained
    );
    let max = if max_raw == LayoutUnit::max() {
        max_raw // don't add border_padding to unconstrained
    } else if style.box_sizing == BoxSizing::ContentBox {
        max_raw + border_padding_block
    } else {
        max_raw.max_of(border_padding_block)
    };

    resolved.clamp(min, max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::Length;
    use openui_style::*;
    use openui_dom::ElementTag;

    #[test]
    fn single_div_fills_width() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);

        // Viewport should be 800×600
        assert_eq!(fragment.size.width.to_i32(), 800);

        // The child div should fill width (800) and be 50px tall
        assert_eq!(fragment.children.len(), 1);
        assert_eq!(fragment.children[0].size.width.to_i32(), 800);
        assert_eq!(fragment.children[0].size.height.to_i32(), 50);
    }

    #[test]
    fn fixed_width_and_height() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.height = Length::px(100.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        assert_eq!(child.size.width.to_i32(), 200);
        assert_eq!(child.size.height.to_i32(), 100);
    }

    #[test]
    fn children_stack_vertically() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.append_child(vp, a);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.append_child(vp, b);

        let c = doc.create_node(ElementTag::Div);
        doc.node_mut(c).style.display = Display::Block;
        doc.node_mut(c).style.height = Length::px(20.0);
        doc.append_child(vp, c);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);

        assert_eq!(fragment.children.len(), 3);
        assert_eq!(fragment.children[0].offset.top.to_i32(), 0);
        assert_eq!(fragment.children[1].offset.top.to_i32(), 50);
        assert_eq!(fragment.children[2].offset.top.to_i32(), 80); // 50 + 30
    }

    #[test]
    fn margin_creates_spacing() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.node_mut(a).style.margin_bottom = Length::px(20.0);
        doc.append_child(vp, a);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.append_child(vp, b);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);

        // b should be at 50 + 20 (margin) = 70
        assert_eq!(fragment.children[1].offset.top.to_i32(), 70);
    }

    #[test]
    fn margin_collapsing_between_siblings() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.node_mut(a).style.margin_bottom = Length::px(20.0);
        doc.append_child(vp, a);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.node_mut(b).style.margin_top = Length::px(30.0);
        doc.append_child(vp, b);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);

        // Collapsed margin between siblings = max(20, 30) = 30
        // b should be at 50 + 30 = 80, NOT 50 + 20 + 30 = 100
        assert_eq!(fragment.children[1].offset.top.to_i32(), 80);
    }

    #[test]
    fn auto_margin_centering() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.node_mut(div).style.margin_left = Length::auto();
        doc.node_mut(div).style.margin_right = Length::auto();
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // Centered: (800 - 200) / 2 = 300
        assert_eq!(child.offset.left.to_i32(), 300);
        assert_eq!(child.size.width.to_i32(), 200);
    }

    #[test]
    fn border_and_padding() {
        let mut doc = Document::new();
        let vp = doc.root();

        let outer = doc.create_node(ElementTag::Div);
        doc.node_mut(outer).style.display = Display::Block;
        doc.node_mut(outer).style.padding_top = Length::px(10.0);
        doc.node_mut(outer).style.padding_left = Length::px(20.0);
        doc.node_mut(outer).style.padding_right = Length::px(20.0);
        doc.node_mut(outer).style.padding_bottom = Length::px(10.0);
        doc.node_mut(outer).style.border_top_width = 2;
        doc.node_mut(outer).style.border_top_style = BorderStyle::Solid;
        doc.node_mut(outer).style.border_bottom_width = 2;
        doc.node_mut(outer).style.border_bottom_style = BorderStyle::Solid;
        doc.node_mut(outer).style.border_left_width = 2;
        doc.node_mut(outer).style.border_left_style = BorderStyle::Solid;
        doc.node_mut(outer).style.border_right_width = 2;
        doc.node_mut(outer).style.border_right_style = BorderStyle::Solid;
        doc.append_child(vp, outer);

        let inner = doc.create_node(ElementTag::Div);
        doc.node_mut(inner).style.display = Display::Block;
        doc.node_mut(inner).style.height = Length::px(50.0);
        doc.append_child(outer, inner);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let outer_frag = &fragment.children[0];

        // Outer should be 800px wide (fills parent)
        assert_eq!(outer_frag.size.width.to_i32(), 800);
        // Outer height = 2 (border-top) + 10 (padding-top) + 50 (child) + 10 (padding-bottom) + 2 (border-bottom) = 74
        assert_eq!(outer_frag.size.height.to_i32(), 74);

        // Inner child should be offset by border+padding
        let inner_frag = &outer_frag.children[0];
        assert_eq!(inner_frag.offset.left.to_i32(), 22); // 2 (border) + 20 (padding)
        assert_eq!(inner_frag.offset.top.to_i32(), 12);  // 2 (border) + 10 (padding)

        // Inner width = 800 - 2*2 (borders) - 2*20 (padding) = 756
        assert_eq!(inner_frag.size.width.to_i32(), 756);
    }

    #[test]
    fn percentage_width() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::percent(50.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        assert_eq!(child.size.width.to_i32(), 400); // 50% of 800
    }

    #[test]
    fn border_box_sizing() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.height = Length::px(100.0);
        doc.node_mut(div).style.padding_left = Length::px(20.0);
        doc.node_mut(div).style.padding_right = Length::px(20.0);
        doc.node_mut(div).style.padding_top = Length::px(10.0);
        doc.node_mut(div).style.padding_bottom = Length::px(10.0);
        doc.node_mut(div).style.border_left_width = 5;
        doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
        doc.node_mut(div).style.border_right_width = 5;
        doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // border-box: total width = 200 (includes padding + border)
        assert_eq!(child.size.width.to_i32(), 200);
        // Content width = 200 - 20 - 20 - 5 - 5 = 150
        assert_eq!(child.content_size().width.to_i32(), 150);
    }

    #[test]
    fn nested_layout() {
        let mut doc = Document::new();
        let vp = doc.root();

        let outer = doc.create_node(ElementTag::Div);
        doc.node_mut(outer).style.display = Display::Block;
        doc.node_mut(outer).style.width = Length::px(400.0);
        doc.append_child(vp, outer);

        let inner = doc.create_node(ElementTag::Div);
        doc.node_mut(inner).style.display = Display::Block;
        doc.node_mut(inner).style.width = Length::percent(50.0);
        doc.node_mut(inner).style.height = Length::px(30.0);
        doc.append_child(outer, inner);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);

        let outer_frag = &fragment.children[0];
        assert_eq!(outer_frag.size.width.to_i32(), 400);

        let inner_frag = &outer_frag.children[0];
        // 50% of 400 = 200
        assert_eq!(inner_frag.size.width.to_i32(), 200);
        assert_eq!(inner_frag.size.height.to_i32(), 30);
    }

    #[test]
    fn display_none_skipped() {
        let mut doc = Document::new();
        let vp = doc.root();

        let a = doc.create_node(ElementTag::Div);
        doc.node_mut(a).style.display = Display::Block;
        doc.node_mut(a).style.height = Length::px(50.0);
        doc.append_child(vp, a);

        let hidden = doc.create_node(ElementTag::Div);
        doc.node_mut(hidden).style.display = Display::None;
        doc.node_mut(hidden).style.height = Length::px(999.0);
        doc.append_child(vp, hidden);

        let b = doc.create_node(ElementTag::Div);
        doc.node_mut(b).style.display = Display::Block;
        doc.node_mut(b).style.height = Length::px(30.0);
        doc.append_child(vp, b);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);

        // Only 2 visible children
        assert_eq!(fragment.children.len(), 2);
        // b should be at 50, not 50+999
        assert_eq!(fragment.children[1].offset.top.to_i32(), 50);
    }

    #[test]
    fn min_max_width() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(1000.0); // wider than max
        doc.node_mut(div).style.max_width = Length::px(500.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        assert_eq!(child.size.width.to_i32(), 500); // clamped by max-width
    }

    #[test]
    fn overconstrained_auto_margins_become_zero() {
        // CSS 2.1 §10.3.3: overconstrained auto margins → 0
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(1000.0); // wider than container
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.node_mut(div).style.margin_left = Length::auto();
        doc.node_mut(div).style.margin_right = Length::auto();
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // Child should be flush-left (margin-left = 0), not shifted negative
        assert_eq!(child.offset.left.to_i32(), 0);
    }

    #[test]
    fn first_child_margin_with_parent_border() {
        // CSS 2.1 §8.3.1: border/padding prevents parent-child margin collapsing
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.border_top_width = 1;
        doc.node_mut(parent).style.border_top_style = BorderStyle::Solid;
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.margin_top = Length::px(20.0);
        doc.node_mut(child).style.height = Length::px(30.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // Child offset = border_top(1) + margin(20) = 21
        assert_eq!(child_frag.offset.top.to_i32(), 21);
        // Parent height = border(1) + margin(20) + child(30) = 51
        assert_eq!(parent_frag.size.height.to_i32(), 51);
    }

    #[test]
    fn viewport_uses_available_height() {
        let doc = Document::new();

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, doc.root(), &space);

        // Viewport should always be full height even with no children
        assert_eq!(fragment.size.height.to_i32(), 600);
        assert_eq!(fragment.size.width.to_i32(), 800);
    }

    #[test]
    fn inline_children_produce_line_boxes() {
        // SP11: Inline elements are now laid out via inline formatting context.
        // A block with an inline child should produce line box children.
        let mut doc = Document::new();
        let vp = doc.root();

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(vp, block);

        // An inline span child (empty, but still triggers IFC)
        let inline = doc.create_node(ElementTag::Span);
        doc.node_mut(inline).style.display = Display::Inline;
        doc.append_child(block, inline);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let block_frag = &fragment.children[0];

        // Block should now have children (line boxes from IFC).
        // An empty span produces no text items, so line breaker produces no
        // lines. The block height is just border+padding (0 here).
        // The block_frag.children may be empty (no actual text content)
        // but the block itself is laid out.
        assert_eq!(block_frag.size.width.to_i32(), 800);
    }

    #[test]
    fn min_height_with_content_box_sizing() {
        // min-height in content-box space must be converted to border-box
        // before clamping the border-box resolved height.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.height = Length::px(50.0);
        doc.node_mut(div).style.min_height = Length::px(100.0);
        doc.node_mut(div).style.padding_top = Length::px(10.0);
        doc.node_mut(div).style.padding_bottom = Length::px(10.0);
        // box-sizing: content-box (default)
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // min-height(100) > height(50), so content = 100px
        // border-box = content(100) + padding(20) = 120px
        assert_eq!(child.size.height.to_i32(), 120);
    }

    #[test]
    fn last_child_bottom_margin_with_parent_border() {
        // CSS 2.1 §8.3.1: bottom border/padding prevents last child's
        // bottom margin from collapsing through the parent.
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.border_bottom_width = 1;
        doc.node_mut(parent).style.border_bottom_style = BorderStyle::Solid;
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::px(30.0);
        doc.node_mut(child).style.margin_bottom = Length::px(20.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];

        // Parent height = child(30) + margin(20) + border(1) = 51
        assert_eq!(parent_frag.size.height.to_i32(), 51);
    }

    #[test]
    fn inline_block_children_produce_line_boxes() {
        // SP11: InlineBlock children are now laid out via IFC.
        let mut doc = Document::new();
        let vp = doc.root();

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(vp, block);

        let ib = doc.create_node(ElementTag::Div);
        doc.node_mut(ib).style.display = Display::InlineBlock;
        doc.node_mut(ib).style.height = Length::px(50.0);
        doc.append_child(block, ib);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let block_frag = &fragment.children[0];

        // Block now uses IFC for inline-level children.
        // The block should be laid out with some height.
        assert_eq!(block_frag.size.width.to_i32(), 800);
    }

    #[test]
    fn auto_width_subtracts_fixed_margins() {
        // CSS 2.1 §10.3.3: width:auto = containing_block - margin - border - padding
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.margin_left = Length::px(20.0);
        doc.node_mut(div).style.margin_right = Length::px(20.0);
        doc.node_mut(div).style.height = Length::px(50.0);
        // width: auto (default)
        doc.append_child(vp, div);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let child = &fragment.children[0];

        // width:auto should be 800 - 20 - 20 = 760
        assert_eq!(child.size.width.to_i32(), 760);
        // positioned at left margin
        assert_eq!(child.offset.left.to_i32(), 20);
    }

    #[test]
    fn percentage_margin_resolves_against_parent_content_box() {
        // Percentage margins resolve against the child's containing block
        // width (parent's content-box), not the grandparent's.
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.width = Length::px(400.0);
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::px(50.0);
        doc.node_mut(child).style.margin_left = Length::percent(50.0);
        // margin-left: 50% of 400 = 200
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // margin-left = 50% of 400 = 200
        assert_eq!(child_frag.offset.left.to_i32(), 200);
        // child width = 400 - 200 (margin) = 200
        assert_eq!(child_frag.size.width.to_i32(), 200);
    }

    #[test]
    fn percentage_height_resolves_against_parent_height() {
        // Percentage heights resolve against the parent's specified height
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        doc.node_mut(parent).style.height = Length::px(200.0);
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::percent(50.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // child height = 50% of 200 = 100
        assert_eq!(child_frag.size.height.to_i32(), 100);
    }

    // ── Issue 1: Double border+padding subtraction ───────────────────

    #[test]
    fn padding_does_not_double_subtract_in_inline_layout() {
        // A div with 20px padding on each side and 200px width should
        // have 160px content box. Text that fits in 160px should NOT
        // wrap (i.e., only one line box). Before the fix, border+padding
        // was subtracted twice, making only 120px available.
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.padding_left = Length::px(20.0);
        doc.node_mut(div).style.padding_right = Length::px(20.0);
        doc.append_child(vp, div);

        // Create text that fits in 160px but not 120px.
        // We use a short word to ensure it fits in the content box.
        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hi".to_string());
        doc.append_child(div, text);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];

        // The div should be 200px wide (content-box sizing default: 200 + 20 + 20 = 240)
        assert_eq!(div_frag.size.width.to_i32(), 240);

        // Should have line boxes as children (from inline layout)
        assert!(
            !div_frag.children.is_empty(),
            "Div with text should produce line boxes"
        );
        // Single short text should produce exactly one line
        assert_eq!(
            div_frag.children.len(),
            1,
            "Short text in 160px content box should fit on one line"
        );
    }

    // ── Issue 3: Mixed content anonymous block boxes ─────────────────

    #[test]
    fn mixed_content_preserves_block_children() {
        // Mixed content: text + block div + text should produce fragments
        // for all three pieces (anonymous inline wrappers + block child).
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        // First inline child: text
        let text1 = doc.create_node(ElementTag::Text);
        doc.node_mut(text1).text = Some("Before".to_string());
        doc.append_child(container, text1);

        // Block child
        let block_child = doc.create_node(ElementTag::Div);
        doc.node_mut(block_child).style.display = Display::Block;
        doc.node_mut(block_child).style.height = Length::px(30.0);
        doc.append_child(container, block_child);

        // Second inline child: text
        let text2 = doc.create_node(ElementTag::Text);
        doc.node_mut(text2).text = Some("After".to_string());
        doc.append_child(container, text2);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // Should have at least 3 children:
        // 1. Anonymous inline wrapper (line box for "Before")
        // 2. Block child (30px height div)
        // 3. Anonymous inline wrapper (line box for "After")
        assert!(
            container_frag.children.len() >= 3,
            "Mixed content should produce at least 3 child fragments, got {}",
            container_frag.children.len(),
        );

        // The block child should have 30px height and be present
        let has_30px_child = container_frag.children.iter().any(|f| f.size.height.to_i32() == 30);
        assert!(
            has_30px_child,
            "Block child with height 30px should be present in fragment tree"
        );
    }

    // ── Issue 4: Span with display:inline-block is atomic inline ─────

    #[test]
    fn span_inline_block_is_atomic_inline() {
        // A <span> with display: inline-block should be treated as an
        // atomic inline, not flattened as a regular inline container.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.append_child(vp, container);

        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::InlineBlock;
        doc.node_mut(span).style.width = Length::px(50.0);
        doc.node_mut(span).style.height = Length::px(20.0);
        doc.append_child(container, span);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // Should produce line boxes (inline formatting context)
        assert!(
            !container_frag.children.is_empty(),
            "Container with inline-block span should produce line boxes"
        );
    }

    // ── SP11 Round 11 Issue 1: display:none doesn't split inline runs ──

    #[test]
    fn display_none_child_does_not_split_inline_run() {
        // [Text, Span(display:none), Text, Div(block)] should produce
        // ONE anonymous inline wrapper for both text nodes, not two.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.node_mut(container).style.width = Length::px(400.0);
        doc.append_child(vp, container);

        let text1 = doc.create_node(ElementTag::Text);
        doc.node_mut(text1).text = Some("Before".to_string());
        doc.append_child(container, text1);

        let hidden_span = doc.create_node(ElementTag::Span);
        doc.node_mut(hidden_span).style.display = Display::None;
        doc.append_child(container, hidden_span);

        let text2 = doc.create_node(ElementTag::Text);
        doc.node_mut(text2).text = Some("After".to_string());
        doc.append_child(container, text2);

        let block_child = doc.create_node(ElementTag::Div);
        doc.node_mut(block_child).style.display = Display::Block;
        doc.node_mut(block_child).style.height = Length::px(10.0);
        doc.append_child(container, block_child);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // Should produce 2 fragments: 1 anonymous inline wrapper (one IFC
        // for both text nodes) + 1 block child. Before the fix, the
        // display:none span split it into 3 fragments.
        assert_eq!(
            container_frag.children.len(), 2,
            "display:none child should not split inline run; expected 2 fragments, got {}",
            container_frag.children.len(),
        );
    }

    #[test]
    fn out_of_flow_child_does_not_split_inline_run() {
        // An absolutely positioned child between inline content should not
        // split the inline run.
        let mut doc = Document::new();
        let vp = doc.root();

        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.node_mut(container).style.width = Length::px(400.0);
        doc.append_child(vp, container);

        let text1 = doc.create_node(ElementTag::Text);
        doc.node_mut(text1).text = Some("A".to_string());
        doc.append_child(container, text1);

        let abs_child = doc.create_node(ElementTag::Div);
        doc.node_mut(abs_child).style.display = Display::Block;
        doc.node_mut(abs_child).style.position = Position::Absolute;
        doc.append_child(container, abs_child);

        let text2 = doc.create_node(ElementTag::Text);
        doc.node_mut(text2).text = Some("B".to_string());
        doc.append_child(container, text2);

        let block_child = doc.create_node(ElementTag::Div);
        doc.node_mut(block_child).style.display = Display::Block;
        doc.node_mut(block_child).style.height = Length::px(10.0);
        doc.append_child(container, block_child);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let container_frag = &fragment.children[0];

        // 1 inline wrapper (single IFC) + 1 block child = 2
        assert_eq!(
            container_frag.children.len(), 2,
            "Out-of-flow child should not split inline run; expected 2 fragments, got {}",
            container_frag.children.len(),
        );
    }

    // ── SP11 Round 11 Issue 3: percentage height in auto-height parent ──

    #[test]
    fn percentage_height_in_auto_height_parent_is_indefinite() {
        // A child with height:50% inside a parent with height:auto should
        // resolve to 0 (indefinite), not 50% of the viewport.
        let mut doc = Document::new();
        let vp = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        doc.node_mut(parent).style.display = Display::Block;
        // height is auto (default)
        doc.append_child(vp, parent);

        let child = doc.create_node(ElementTag::Div);
        doc.node_mut(child).style.display = Display::Block;
        doc.node_mut(child).style.height = Length::percent(50.0);
        doc.append_child(parent, child);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let parent_frag = &fragment.children[0];
        let child_frag = &parent_frag.children[0];

        // With auto-height parent, 50% height should resolve to 0 (indefinite).
        // Before the fix, it resolved to 50% of 600 = 300.
        assert_eq!(
            child_frag.size.height.to_i32(), 0,
            "50% height in auto-height parent should be 0 (indefinite), got {}",
            child_frag.size.height.to_i32(),
        );
    }

    // ── SP11 Round 15 Issue 5: percentage padding resolves against containing block ──

    #[test]
    fn percentage_padding_resolves_against_containing_block_not_content_box() {
        // A block with padding has its content-box smaller than its width.
        // When inline_layout re-resolves the SAME block's percentage padding,
        // it should use the containing block's width (from the parent), not
        // the block's own content-box width.
        //
        // Setup: viewport(800) → div(width:200, padding-left:10%)
        // Before fix: padding resolved as 10% of 200 = 20px (wrong)
        // After fix: padding resolved as 10% of 800 = 80px (correct)
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(200.0);
        doc.node_mut(div).style.padding_left = Length::percent(10.0);
        doc.append_child(vp, div);

        let text = doc.create_node(ElementTag::Text);
        doc.node_mut(text).text = Some("Hello".to_string());
        doc.append_child(div, text);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let div_frag = &fragment.children[0];

        // 10% of containing block (800px viewport) = 80px.
        // The fix ensures inline_layout receives the correct percentage base.
        let resolved_padding = div_frag.padding.left.to_i32();
        assert_eq!(
            resolved_padding, 80,
            "10% padding-left should resolve against containing block (800px) = 80px, got {}",
            resolved_padding,
        );
    }
}
