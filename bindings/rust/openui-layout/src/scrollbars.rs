//! Physical scrollbar space, separate from authored border and padding.

use crate::layout_context::LayoutDocument;
use crate::{block::block_layout_contents, ConstraintSpace, Fragment};
#[cfg(test)]
use openui_dom::Document;
use openui_dom::NodeId;
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalRect};
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
        crate::scroll_area::client_rect(fragment, self.insets)
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
pub(crate) fn intrinsic_insets(doc: &dyn LayoutDocument, node: NodeId) -> BoxStrut {
    let item = doc.node(node);
    let style = &item.style;
    if crate::scroll_area::is_replaced_layout_box(doc, node)
        || style.display.is_table_wrapper()
        || !crate::scroll_area::is_scroll_container(doc, node)
    {
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
    doc: &dyn LayoutDocument,
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

pub(crate) fn layout(doc: &dyn LayoutDocument, node: NodeId, space: &ConstraintSpace) -> Fragment {
    let item = doc.node(node);
    let style = &item.style;
    let viewport_source = crate::scroll_area::viewport_defining_element(doc);
    let replaced = crate::scroll_area::is_replaced_layout_box(doc, node);
    // Stable gutters are available on replaced boxes too, although those
    // boxes never own actual scrollbars. Table boxes use adjusted overflow;
    // their track sizing and fragment placement consume different edge struts.
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
            | Display::Table
            | Display::InlineTable
            | Display::TableCell
            | Display::TableCaption
    ) && !replaced
        || Some(node) == viewport_source
        || (!crate::scroll_area::is_scroll_container(doc, node) && !replaced)
    {
        let mut fragment = block_layout_contents(doc, node, space);
        crate::layout_context::retain_layout_box(doc, node, &mut fragment);
        return fragment;
    }
    let (overflow_x, overflow_y) = crate::scroll_area::overflow_pair(style);
    let enabled = style.scrollbar_width != ScrollbarWidth::None && !replaced;
    let mut horizontal = enabled && overflow_x == Overflow::Scroll;
    let mut vertical = enabled && overflow_y == Overflow::Scroll;
    let mut scrollbars = geometry(style, horizontal, vertical);
    loop {
        let mut current = space.clone();
        current.element_scrollbars = Some((node, scrollbars));
        let mut fragment = block_layout_contents(doc, node, &current);
        crate::layout_context::retain_layout_box(doc, node, &mut fragment);
        fragment.element_scrollbars = Some(scrollbars);
        // LayoutBox clamps a left gutter to the physical content width. The
        // right gutter remains its theme thickness, even in a narrow box.
        let left = scrollbars.insets.left.min_of(
            (crate::scroll_area::client_rect(&fragment, BoxStrut::zero()).width()
                - crate::ScrollArea::used_padding(&fragment).inline_sum())
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

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::ElementTag;
    use openui_geometry::{Length, PhysicalSize};
    use openui_style::{
        FlexDirection, GridTrackBreadth, GridTrackComponent, GridTrackList, GridTrackSize,
    };

    // Public algorithm consumers must get the same actual percentage basis as
    // the retained-document entry point, rather than bypassing gutter layout.
    #[test]
    fn public_flex_and_grid_entries_reserve_classic_gutters_once() {
        for display in [Display::Flex, Display::Grid] {
            let mut doc = Document::new();
            let container = doc.create_node(ElementTag::Div);
            let child = doc.create_node(ElementTag::Div);
            let track = GridTrackList::Tracks(vec![GridTrackComponent::Track(
                GridTrackSize::Breadth(GridTrackBreadth::Length(Length::percent(100.0))),
            )]);
            doc.node_mut(container).style.update_derived(|style| {
                style.display = display;
                style.width = Length::px(100.0);
                style.height = Length::px(80.0);
                style.overflow_x = Overflow::Scroll;
                style.overflow_y = Overflow::Scroll;
                style.flex_direction = FlexDirection::Column;
                style.grid_template_columns = track.clone();
                style.grid_template_rows = track.clone();
            });
            doc.node_mut(child).style.update_derived(|style| {
                style.display = Display::Block;
                style.width = Length::percent(100.0);
                style.height = Length::percent(100.0);
                style.min_width = Length::px(0.0);
                style.min_height = Length::px(0.0);
                style.flex_shrink = 0.0;
            });
            doc.append_child(container, child);
            let space =
                ConstraintSpace::for_root(LayoutUnit::from_i32(320), LayoutUnit::from_i32(240));
            let direct = if display == Display::Flex {
                crate::flex::flex_layout(&doc, container, &space)
            } else {
                crate::grid::grid_layout_with_layout(&doc, container, &space)
            };
            let retained = crate::block::block_layout_with_layout(&doc, container, &space);
            for fragment in [&direct, &retained] {
                assert_eq!(
                    fragment.size,
                    PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(80))
                );
                assert_eq!(
                    fragment.content_size(),
                    PhysicalSize::new(LayoutUnit::from_i32(85), LayoutUnit::from_i32(65))
                );
                let actual = fragment
                    .children
                    .iter()
                    .find(|fragment| fragment.node_id == child)
                    .unwrap();
                assert_eq!(actual.size, fragment.content_size());
                assert_eq!(fragment.padding, BoxStrut::zero());
            }
        }
    }
}
