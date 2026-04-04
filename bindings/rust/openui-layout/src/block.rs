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

use openui_geometry::{LayoutUnit, BoxStrut, PhysicalOffset, PhysicalSize, MarginStrut};
use openui_style::{ComputedStyle, Display, BoxSizing};
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

    let content_edge = border.top + padding.top;
    let mut block_offset = content_edge;
    let mut margin_strut = MarginStrut::new();
    let mut child_fragments: Vec<Fragment> = Vec::new();
    let mut intrinsic_block_size = content_edge;

    for child_id in doc.children(node_id) {
        let child_style = &doc.node(child_id).style;

        // Skip out-of-flow children (absolute, fixed) — they don't participate
        // in block flow. Floats are also skipped for SP9.
        // Skip inline-level elements — inline formatting context is SP11.
        if child_style.is_out_of_flow() || child_style.display == Display::None {
            continue;
        }
        if child_style.display == Display::Inline {
            // Inline elements require an inline formatting context (SP11).
            // Skip for now to avoid laying them out as blocks.
            continue;
        }

        // ── Calculate child margins ──────────────────────────────────
        // Blink: CalculateMargins() (line 3329)
        let child_margin = resolve_margins(child_style, space.percentage_resolution_inline_size);

        // ── Margin collapsing ────────────────────────────────────────
        // Blink: ComputeChildData / ComputeInflowPosition
        //
        // Append child's margin-top to the current margin strut.
        // The strut tracks the largest positive and most-negative margins
        // and collapses them per CSS 2.1 §8.3.1.
        margin_strut.append(child_margin.top);

        // Resolve the margin strut in these cases:
        // 1. First child in a new BFC — border/padding prevents collapse-through
        // 2. First child when parent has top border or padding (CSS 2.1 §8.3.1:
        //    "The top margin of an in-flow block-level element collapses with
        //    its first in-flow block-level child's top margin if the element
        //    has no top border, no top padding...")
        // 3. Between siblings — collapse previous bottom + current top
        if child_fragments.is_empty() {
            if space.is_new_formatting_context || content_edge > LayoutUnit::zero() {
                block_offset += margin_strut.sum();
                margin_strut = MarginStrut::new();
            }
        } else {
            // Between siblings: resolve the collapsed margin between
            // previous sibling's margin-bottom and this child's margin-top.
            block_offset += margin_strut.sum();
            margin_strut = MarginStrut::new();
        }

        // ── Create child constraint space ────────────────────────────
        // Blink: CreateConstraintSpaceForChild() (line 3408)
        let child_is_new_fc = child_style.creates_new_formatting_context();
        let child_space = ConstraintSpace::for_block_child(
            child_available_inline,
            space.available_block_size, // pass through for now
            child_available_inline,     // percentage resolution
            space.percentage_resolution_block_size,
            child_is_new_fc,
        );

        // ── Layout child ─────────────────────────────────────────────
        let mut child_fragment = if child_style.display == Display::Block
            || child_style.display == Display::FlowRoot
            || child_style.display == Display::ListItem
        {
            block_layout(doc, child_id, &child_space)
        } else {
            // For non-block display types in block flow (inline-block, etc.),
            // treat as block for now. Flex/Grid/Inline will be added later.
            block_layout(doc, child_id, &child_space)
        };

        // ── Auto margin resolution (horizontal centering) ───────────
        // Blink: CalculateMargins + auto margin resolution
        //
        // If both margin-left and margin-right are auto, center the child.
        // If only one is auto, it absorbs the remaining space.
        let child_border_box_inline = child_fragment.size.width;
        let remaining_space = child_available_inline - child_border_box_inline;

        let resolved_margin_left;
        let resolved_margin_right;

        if child_style.margin_left.is_auto() && child_style.margin_right.is_auto() {
            // Center: split remaining space equally.
            // Per CSS 2.1 §10.3.3: if overconstrained, auto margins → 0.
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

        // ── Position child ───────────────────────────────────────────
        // Blink: ComputeInflowPosition() (line 2881)
        child_fragment.offset = PhysicalOffset::new(
            border.left + padding.left + resolved_margin_left,
            block_offset,
        );
        child_fragment.margin = BoxStrut::new(
            child_margin.top,
            resolved_margin_right,
            child_margin.bottom,
            resolved_margin_left,
        );

        // Advance block offset past the child
        block_offset += child_fragment.size.height;

        // Start new margin strut with child's bottom margin
        margin_strut = MarginStrut::new();
        margin_strut.append(child_margin.bottom);

        intrinsic_block_size = block_offset;
        child_fragments.push(child_fragment);
    }

    // ── Step 4: Finish layout (FinishLayout, line 1165) ──────────────
    // Resolve the trailing margin strut if we're in a new formatting context.
    if space.is_new_formatting_context && !margin_strut.is_empty() {
        intrinsic_block_size += margin_strut.sum();
    }

    // Add bottom border + padding
    intrinsic_block_size += border.bottom + padding.bottom;

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

    fragment
}

// ── Helper: resolve border widths from style ─────────────────────────

fn resolve_border(style: &ComputedStyle) -> BoxStrut {
    BoxStrut::new(
        LayoutUnit::from_i32(style.effective_border_top()),
        LayoutUnit::from_i32(style.effective_border_right()),
        LayoutUnit::from_i32(style.effective_border_bottom()),
        LayoutUnit::from_i32(style.effective_border_left()),
    )
}

// ── Helper: resolve padding lengths ──────────────────────────────────

fn resolve_padding(style: &ComputedStyle, percentage_base: LayoutUnit) -> BoxStrut {
    BoxStrut::new(
        resolve_margin_or_padding(&style.padding_top, percentage_base),
        resolve_margin_or_padding(&style.padding_right, percentage_base),
        resolve_margin_or_padding(&style.padding_bottom, percentage_base),
        resolve_margin_or_padding(&style.padding_left, percentage_base),
    )
}

// ── Helper: resolve margins ──────────────────────────────────────────

fn resolve_margins(style: &ComputedStyle, percentage_base: LayoutUnit) -> BoxStrut {
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
    fn inline_children_are_skipped() {
        // Inline elements are not yet supported (SP11). They should be
        // skipped in block layout, not laid out as blocks.
        let mut doc = Document::new();
        let vp = doc.root();

        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(vp, block);

        // Default display is Inline — this child should be skipped
        let inline = doc.create_node(ElementTag::Span);
        doc.node_mut(inline).style.height = Length::px(50.0);
        doc.append_child(block, inline);

        let space = ConstraintSpace::for_root(
            LayoutUnit::from_i32(800),
            LayoutUnit::from_i32(600),
        );
        let fragment = block_layout(&doc, vp, &space);
        let block_frag = &fragment.children[0];

        // Block should have no children (inline was skipped)
        assert!(block_frag.children.is_empty());
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
}
