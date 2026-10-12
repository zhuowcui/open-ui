//! Physical scrollbar space, separate from authored border and padding.

use crate::{block::block_layout_contents, ConstraintSpace, Fragment};
use openui_dom::{Document, NodeId};
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_style::{ComputedStyle, Display, Overflow, ScrollbarWidth};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ElementScrollbars {
    pub insets: BoxStrut,
    pub horizontal: bool,
    pub vertical: bool,
    pub thickness: LayoutUnit,
}

impl ElementScrollbars {
    pub fn client_rect(self, fragment: &Fragment) -> PhysicalRect {
        PhysicalRect::new(
            PhysicalOffset::new(
                fragment.border.left + self.insets.left,
                fragment.border.top + self.insets.top,
            ),
            PhysicalSize::new(
                (fragment.padding_box_size().width - self.insets.inline_sum())
                    .clamp_negative_to_zero(),
                (fragment.padding_box_size().height - self.insets.block_sum())
                    .clamp_negative_to_zero(),
            ),
        )
    }
}

fn geometry(style: &ComputedStyle, horizontal: bool, vertical: bool) -> ElementScrollbars {
    let thickness = LayoutUnit::from_i32(match style.scrollbar_width {
        ScrollbarWidth::Auto => 15,
        ScrollbarWidth::Thin => 10,
        ScrollbarWidth::None => 0,
    });
    let stable = style.scrollbar_gutter.is_stable();
    let direction = style.direction.writing_direction(style.writing_mode);
    let (overflow_x, overflow_y) = crate::scroll_area::overflow_pair(style);
    let reserves = |overflow| {
        matches!(
            overflow,
            Overflow::Hidden | Overflow::Auto | Overflow::Scroll
        )
    };
    let stable_vertical = stable && direction.is_horizontal() && reserves(overflow_y);
    let stable_horizontal = stable && !direction.is_horizontal() && reserves(overflow_x);
    let mut insets = BoxStrut::zero();
    if vertical || stable_vertical {
        if direction.is_horizontal() && direction.is_rtl() {
            insets.left = thickness;
        } else {
            insets.right = thickness;
        }
        if stable_vertical && style.scrollbar_gutter.both_edges() {
            insets.left = thickness;
            insets.right = thickness;
        }
    }
    if horizontal || stable_horizontal {
        insets.bottom = thickness;
        if stable_horizontal && style.scrollbar_gutter.both_edges() {
            insets.top = thickness;
        }
    }
    ElementScrollbars {
        insets,
        horizontal,
        vertical,
        thickness,
    }
}

/// Reserved space belongs to this element, not its descendants. A parent's
/// layout can carry the same constraint space through several algorithms.
pub(crate) fn for_node(space: &ConstraintSpace, node: NodeId) -> Option<ElementScrollbars> {
    space
        .element_scrollbars
        .filter(|(owner, _)| *owner == node)
        .map(|(_, scrollbars)| scrollbars)
}

/// The fixed/stable contribution available before measuring the contents.
/// Auto scrollbars are discovered by the shared layout loop below.
pub(crate) fn intrinsic_insets(doc: &Document, node: NodeId) -> BoxStrut {
    let item = doc.node(node);
    let style = &item.style;
    let viewport_source = if doc.body_overflow_is_propagated() {
        doc.body_element()
    } else {
        doc.document_element()
    };
    if item.replaced.is_some() || !style.is_scroll_container() || Some(node) == viewport_source {
        return BoxStrut::zero();
    }
    let (overflow_x, overflow_y) = crate::scroll_area::overflow_pair(style);
    let enabled = style.scrollbar_width != ScrollbarWidth::None;
    geometry(
        style,
        enabled && overflow_x == Overflow::Scroll,
        enabled && overflow_y == Overflow::Scroll,
    )
    .insets
}

/// Intrinsic sizing includes fixed/stable gutters. A later auto-scrollbar
/// pass replaces that contribution with its actual reserved space.
pub(crate) fn intrinsic_sizes_for_layout(
    doc: &Document,
    node: NodeId,
    space: &ConstraintSpace,
    mut sizes: crate::intrinsic_sizing::IntrinsicSizes,
) -> crate::intrinsic_sizing::IntrinsicSizes {
    let Some(scrollbars) = for_node(space, node) else {
        return sizes;
    };
    let direction = doc
        .node(node)
        .style
        .direction
        .writing_direction(doc.node(node).style.writing_mode);
    let initial = intrinsic_insets(doc, node).to_logical(direction);
    let actual = scrollbars.insets.to_logical(direction);
    let inline_delta = actual.inline_sum() - initial.inline_sum();
    let block_delta = actual.block_sum() - initial.block_sum();
    sizes.min_content_inline_size = sizes.min_content_inline_size + inline_delta;
    sizes.max_content_inline_size = sizes.max_content_inline_size + inline_delta;
    sizes.min_content_block_size = sizes.min_content_block_size + block_delta;
    sizes.max_content_block_size = sizes.max_content_block_size + block_delta;
    sizes
}

pub(crate) fn layout(doc: &Document, node: NodeId, space: &ConstraintSpace) -> Fragment {
    let item = doc.node(node);
    let style = &item.style;
    let viewport_source = if doc.body_overflow_is_propagated() {
        doc.body_element()
    } else {
        doc.document_element()
    };
    // Each admitted formatting context consumes independent scrollbar space.
    // Table overflow has class-specific used values; replaced elements do not
    // establish scroll containers. Their stable-gutter paths are separate.
    if !matches!(
        style.display,
        Display::Block
            | Display::InlineBlock
            | Display::FlowRoot
            | Display::ListItem
            | Display::Flex
            | Display::InlineFlex
            | Display::Grid
            | Display::InlineGrid
    ) || !style.is_scroll_container()
        || item.replaced.is_some()
        || Some(node) == viewport_source
    {
        return block_layout_contents(doc, node, space);
    }
    let (overflow_x, overflow_y) = crate::scroll_area::overflow_pair(style);
    let enabled = style.scrollbar_width != ScrollbarWidth::None;
    let mut horizontal = enabled && overflow_x == Overflow::Scroll;
    let mut vertical = enabled && overflow_y == Overflow::Scroll;
    let mut scrollbars = geometry(style, horizontal, vertical);
    loop {
        let mut current = space.clone();
        current.element_scrollbars = Some((node, scrollbars));
        let mut fragment = block_layout_contents(doc, node, &current);
        fragment.element_scrollbars = Some(scrollbars);
        // LayoutBox clamps a left gutter to the physical content width. The
        // right gutter remains its theme thickness, even in a narrow box.
        let left = scrollbars.insets.left.min_of(
            (fragment.padding_box_size().width - fragment.padding.inline_sum())
                .clamp_negative_to_zero(),
        );
        if left != scrollbars.insets.left {
            scrollbars.insets.left = left;
            continue;
        }
        crate::scroll_area::populate(doc, &mut fragment);
        let area = fragment.scroll_area.expect("element scroll geometry");
        let (x, y) = area.limits();
        let next_horizontal = horizontal || enabled && overflow_x == Overflow::Auto && x.0 != x.1;
        let next_vertical = vertical || enabled && overflow_y == Overflow::Auto && y.0 != y.1;
        if (horizontal, vertical) == (next_horizontal, next_vertical) {
            return fragment;
        }
        horizontal = next_horizontal;
        vertical = next_vertical;
        scrollbars = geometry(style, horizontal, vertical);
    }
}
