//! SP17 W1L — edge-biased flex abspos static-position regressions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{
    BorderStyle, ContentAlignment, ContentPosition, Direction, Display, FlexDirection,
    ItemAlignment, ItemPosition, Position, WritingMode,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn layout_root(doc: &Document) -> Fragment {
    block_layout(
        doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(800), lu(600)),
    )
}

fn fragment_for(parent: &Fragment, node_id: NodeId) -> &Fragment {
    parent
        .children
        .iter()
        .find(|fragment| fragment.node_id == node_id)
        .unwrap_or_else(|| panic!("missing fragment for {node_id:?}"))
}

fn absolute_offsets_for(
    fragment: &Fragment,
    node_id: NodeId,
    parent_offset: openui_geometry::PhysicalOffset,
    offsets: &mut Vec<openui_geometry::PhysicalOffset>,
) {
    let absolute = openui_geometry::PhysicalOffset::new(
        parent_offset.left + fragment.offset.left,
        parent_offset.top + fragment.offset.top,
    );
    if fragment.node_id == node_id {
        offsets.push(absolute);
    }
    for child in &fragment.children {
        absolute_offsets_for(child, node_id, absolute, offsets);
    }
}

fn bordered(style: &mut openui_style::ComputedStyleFields, left: i32, top: i32) {
    style.update_derived(|computed| computed.border_left_width = left);
    style.update_derived(|computed| computed.border_top_width = top);
    style.update_derived(|computed| computed.border_right_width = 3);
    style.update_derived(|computed| computed.border_bottom_width = 4);
    style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
    style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
    style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
}

fn add_outer(doc: &mut Document) -> NodeId {
    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        bordered(style, 7, 5);
        style.update_derived(|computed| computed.padding_left = Length::px(11.0));
        style.update_derived(|computed| computed.padding_right = Length::px(13.0));
        style.update_derived(|computed| computed.padding_top = Length::px(9.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(15.0));
    });
    doc.append_child(doc.root(), outer);
    outer
}

fn add_centering_flex(doc: &mut Document, outer: NodeId, direction: FlexDirection) -> NodeId {
    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = direction);
        style.update_derived(|computed| computed.width = Length::px(40.0));
        style.update_derived(|computed| computed.height = Length::px(40.0));
        style.update_derived(|computed| computed.margin_left = Length::px(10.0));
        style.update_derived(|computed| computed.margin_top = Length::px(10.0));
        style.update_derived(|computed| {
            computed.justify_content = ContentAlignment::new(ContentPosition::Center)
        });
        style.update_derived(|computed| {
            computed.align_items = ItemAlignment::new(ItemPosition::Center)
        });
    });
    doc.append_child(outer, flex);
    flex
}

fn add_abspos(doc: &mut Document, parent: NodeId) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Absolute);
    });
    doc.append_child(parent, child);
    child
}

#[test]
fn bubbled_horizontal_center_uses_asymmetric_interval_for_known_and_auto_widths() {
    for known_width in [true, false] {
        let mut doc = Document::new();
        let outer = add_outer(&mut doc);
        let flex = add_centering_flex(&mut doc, outer, FlexDirection::Row);
        let abspos = add_abspos(&mut doc, flex);
        doc.update_resolved_style(abspos, |style| {
            style.update_derived(|computed| computed.top = Length::px(0.0));
            style.update_derived(|computed| computed.height = Length::px(10.0));
            if known_width {
                style.update_derived(|computed| computed.width = Length::px(20.0));
                style.update_derived(|computed| computed.margin_left = Length::px(4.0));
                style.update_derived(|computed| computed.margin_right = Length::px(6.0));
            }
        });
        if !known_width {
            for _ in 0..2 {
                let intrinsic = doc.create_node(ElementTag::Div);
                doc.update_resolved_style(intrinsic, |style| style.display = Display::InlineBlock);
                doc.update_resolved_style(intrinsic, |style| style.width = Length::px(50.0));
                doc.update_resolved_style(intrinsic, |style| style.height = Length::px(10.0));
                doc.append_child(abspos, intrinsic);
            }
        }

        let root = layout_root(&doc);
        let outer_fragment = fragment_for(&root, outer);
        let positioned = fragment_for(outer_fragment, abspos);
        if known_width {
            // Padding-coordinate anchor = 11 + 10 + 20 = 41. The centered
            // interval is [0,82]; the 30px margin box centers at 41 and the
            // outer 7px border is added once.
            assert_eq!(positioned.offset.left, lu(37));
            assert_eq!(positioned.size.width, lu(20));
            assert_eq!(positioned.margin.left, lu(4));
            assert_eq!(positioned.margin.right, lu(6));
        } else {
            assert_eq!(positioned.offset.left, lu(7));
            assert_eq!(positioned.size.width, lu(82));
        }
        assert_eq!(positioned.offset.top, lu(5));
    }
}

#[test]
fn bubbled_vertical_center_repositions_auto_physical_height_after_vertical_layout() {
    for known_height in [true, false] {
        let mut doc = Document::new();
        let outer = add_outer(&mut doc);
        let flex = add_centering_flex(&mut doc, outer, FlexDirection::Column);
        let abspos = add_abspos(&mut doc, flex);
        doc.update_resolved_style(abspos, |style| {
            style.update_derived(|computed| computed.left = Length::px(0.0));
            style.update_derived(|computed| computed.width = Length::px(10.0));
            if known_height {
                style.update_derived(|computed| computed.height = Length::px(20.0));
                style.update_derived(|computed| computed.margin_top = Length::px(4.0));
                style.update_derived(|computed| computed.margin_bottom = Length::px(6.0));
            } else {
                style.update_derived(|computed| computed.writing_mode = WritingMode::VerticalRl);
                style.update_derived(|computed| computed.direction = Direction::Ltr);
            }
        });
        if !known_height {
            for _ in 0..2 {
                let inline = doc.create_node(ElementTag::Div);
                doc.update_resolved_style(inline, |style| {
                    style.display = Display::InlineBlock;
                    style.writing_mode = WritingMode::VerticalRl;
                    style.width = Length::px(10.0);
                    style.height = Length::px(50.0);
                });
                doc.append_child(abspos, inline);
            }
        }

        let root = layout_root(&doc);
        let outer_fragment = fragment_for(&root, outer);
        let positioned = fragment_for(outer_fragment, abspos);
        if known_height {
            // Padding-coordinate anchor = 9 + 10 + 20 = 39. The centered
            // interval is [0,78], and the complete 30px margin box is centered.
            assert_eq!(positioned.offset.top, lu(33));
            assert_eq!(positioned.size.height, lu(20));
        } else {
            assert_eq!(positioned.offset.top, lu(5));
            assert_eq!(positioned.size.height, lu(78));
        }
        assert_eq!(positioned.offset.left, lu(7));
    }
}

#[test]
fn center_anchor_is_independent_of_hypothetical_size_across_flow_reversal() {
    for flex_direction in [
        FlexDirection::Row,
        FlexDirection::RowReverse,
        FlexDirection::Column,
        FlexDirection::ColumnReverse,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let mut doc = Document::new();
            let outer = add_outer(&mut doc);
            let flex = add_centering_flex(&mut doc, outer, flex_direction);
            doc.update_resolved_style(flex, |style| style.direction = direction);
            let abspos = add_abspos(&mut doc, flex);
            doc.update_resolved_style(abspos, |style| {
                style.update_derived(|computed| computed.width = Length::px(20.0));
                style.update_derived(|computed| computed.height = Length::px(20.0));
            });

            let root = layout_root(&doc);
            let outer_fragment = fragment_for(&root, outer);
            let positioned = fragment_for(outer_fragment, abspos);
            assert_eq!(
                positioned.offset.left,
                lu(38),
                "x for {direction:?}/{flex_direction:?}",
            );
            assert_eq!(
                positioned.offset.top,
                lu(34),
                "y for {direction:?}/{flex_direction:?}",
            );
        }
    }
}

#[test]
fn ordinary_block_static_start_edge_behavior_is_unchanged() {
    for direction in [Direction::Ltr, Direction::Rtl] {
        let mut doc = Document::new();
        let outer = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(outer, |style| {
            style.update_derived(|computed| computed.display = Display::Block);
            style.update_derived(|computed| computed.position = Position::Relative);
            style.update_derived(|computed| computed.direction = direction);
            style.update_derived(|computed| computed.width = Length::px(100.0));
            style.update_derived(|computed| computed.height = Length::px(80.0));
        });
        doc.append_child(doc.root(), outer);

        let inflow = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(inflow, |style| style.height = Length::px(20.0));
        doc.append_child(outer, inflow);

        let abspos = add_abspos(&mut doc, outer);
        doc.update_resolved_style(abspos, |style| style.width = Length::px(10.0));
        doc.update_resolved_style(abspos, |style| style.height = Length::px(10.0));

        let root = layout_root(&doc);
        let positioned = fragment_for(fragment_for(&root, outer), abspos);
        assert_eq!(positioned.offset.top, lu(0));
        assert_eq!(
            positioned.offset.left,
            if direction == Direction::Ltr {
                lu(0)
            } else {
                lu(90)
            },
        );
    }
}

#[test]
fn fragmented_column_flex_materializes_center_and_end_edges_before_column_mapping() {
    for (position, expected_left, expected_top) in [
        (ContentPosition::Center, 25, 50),
        (ContentPosition::FlexEnd, 75, 0),
    ] {
        let mut doc = Document::new();
        let multicol = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(multicol, |style| {
            style.update_derived(|computed| computed.display = Display::Block);
            style.update_derived(|computed| computed.position = Position::Relative);
            style.update_derived(|computed| computed.width = Length::px(100.0));
            style.update_derived(|computed| computed.height = Length::px(100.0));
            style.update_derived(|computed| computed.column_count = Some(4));
            style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        });
        doc.append_child(doc.root(), multicol);

        let flex = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(flex, |style| {
            style.update_derived(|computed| computed.display = Display::Flex);
            style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
            style.update_derived(|computed| {
                computed.justify_content = ContentAlignment::new(position)
            });
            style.update_derived(|computed| computed.width = Length::px(50.0));
            style.update_derived(|computed| computed.height = Length::px(400.0));
        });
        doc.append_child(multicol, flex);

        let abspos = add_abspos(&mut doc, flex);
        doc.update_resolved_style(abspos, |style| style.width = Length::px(25.0));
        doc.update_resolved_style(abspos, |style| style.height = Length::px(100.0));
        for _ in 0..4 {
            let child = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(child, |style| style.width = Length::px(25.0));
            doc.update_resolved_style(child, |style| style.height = Length::px(100.0));
            doc.append_child(flex, child);
        }

        let root = layout_root(&doc);
        let mut offsets = Vec::new();
        absolute_offsets_for(
            &root,
            abspos,
            openui_geometry::PhysicalOffset::zero(),
            &mut offsets,
        );
        assert_eq!(
            offsets,
            vec![openui_geometry::PhysicalOffset::new(
                lu(expected_left),
                lu(expected_top),
            )],
            "fragmented static position for {position:?}",
        );
    }
}
