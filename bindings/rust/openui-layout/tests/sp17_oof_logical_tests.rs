//! SP17 W1G — logical out-of-flow geometry regressions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{
    BorderStyle, ContentAlignment, ContentPosition, Direction, Display, ItemAlignment,
    ItemPosition, Position, WritingMode,
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

fn add_positioned_container(
    doc: &mut Document,
    mode: WritingMode,
    direction: Direction,
    width: i32,
    height: i32,
) -> NodeId {
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| {
        style.display = Display::Block;
        style.position = Position::Relative;
        style.writing_mode = mode;
        style.direction = direction;
        style.width = Length::px(width as f32);
        style.height = Length::px(height as f32);
    });
    doc.append_child(doc.root(), container);
    container
}

fn add_abspos(
    doc: &mut Document,
    parent: NodeId,
    mode: WritingMode,
    direction: Direction,
) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| {
        style.display = Display::Block;
        style.position = Position::Absolute;
        style.writing_mode = mode;
        style.direction = direction;
    });
    doc.append_child(parent, child);
    child
}

fn fragment_for<'a>(parent: &'a Fragment, node_id: NodeId) -> &'a Fragment {
    parent
        .children
        .iter()
        .find(|fragment| fragment.node_id == node_id)
        .unwrap_or_else(|| panic!("missing fragment for {node_id:?}"))
}

#[test]
fn physical_overconstraint_polarity_covers_three_modes_and_both_directions() {
    let cases = [
        (WritingMode::HorizontalTb, Direction::Ltr, 10, 10),
        (WritingMode::HorizontalTb, Direction::Rtl, 180, 10),
        (WritingMode::VerticalLr, Direction::Ltr, 10, 10),
        (WritingMode::VerticalLr, Direction::Rtl, 10, 130),
        (WritingMode::VerticalRl, Direction::Ltr, 180, 10),
        (WritingMode::VerticalRl, Direction::Rtl, 180, 130),
    ];

    for (mode, direction, expected_left, expected_top) in cases {
        let mut doc = Document::new();
        let container = add_positioned_container(&mut doc, mode, direction, 300, 200);
        let abspos = add_abspos(&mut doc, container, mode, direction);
        doc.update_resolved_style(abspos, |style| {
            style.left = Length::px(10.0);
            style.right = Length::px(20.0);
            style.top = Length::px(10.0);
            style.bottom = Length::px(20.0);
            style.width = Length::px(100.0);
            style.height = Length::px(50.0);
        });

        let root = layout_root(&doc);
        let container_fragment = fragment_for(&root, container);
        let positioned = fragment_for(container_fragment, abspos);
        assert_eq!(
            positioned.offset.left,
            lu(expected_left),
            "left for {mode:?}/{direction:?}"
        );
        assert_eq!(
            positioned.offset.top,
            lu(expected_top),
            "top for {mode:?}/{direction:?}"
        );
        assert_eq!(positioned.size.width, lu(100));
        assert_eq!(positioned.size.height, lu(50));
    }
}

#[test]
fn flex_abspos_static_position_projects_asymmetric_edges_once() {
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let mut doc = Document::new();
            let container = add_positioned_container(&mut doc, mode, direction, 120, 80);
            doc.update_resolved_style(container, |style| {
                style.update_derived(|computed| computed.display = Display::Flex);
                style.update_derived(|computed| {
                    computed.justify_content = ContentAlignment::new(ContentPosition::Center)
                });
                style.update_derived(|computed| {
                    computed.align_items = ItemAlignment::new(ItemPosition::Center)
                });
                style.update_derived(|computed| computed.border_left_width = 7);
                style.update_derived(|computed| computed.border_right_width = 11);
                style.update_derived(|computed| computed.border_top_width = 13);
                style.update_derived(|computed| computed.border_bottom_width = 17);
                style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.padding_left = Length::px(19.0));
                style.update_derived(|computed| computed.padding_right = Length::px(23.0));
                style.update_derived(|computed| computed.padding_top = Length::px(29.0));
                style.update_derived(|computed| computed.padding_bottom = Length::px(31.0));
            });
            let abspos = add_abspos(&mut doc, container, mode, direction);
            doc.update_resolved_style(abspos, |style| style.width = Length::px(20.0));
            doc.update_resolved_style(abspos, |style| style.height = Length::px(10.0));

            let root = layout_root(&doc);
            let container_fragment = fragment_for(&root, container);
            let positioned = fragment_for(container_fragment, abspos);
            assert_eq!(
                positioned.offset.left,
                lu(76),
                "left for {mode:?}/{direction:?}"
            );
            assert_eq!(
                positioned.offset.top,
                lu(77),
                "top for {mode:?}/{direction:?}"
            );
            assert_eq!(positioned.size.width, lu(20));
            assert_eq!(positioned.size.height, lu(10));
        }
    }
}

#[test]
fn vertical_abspos_intrinsic_block_keywords_and_auto_margins_are_physical() {
    for intrinsic in [
        Length::min_content(),
        Length::max_content(),
        Length::fit_content(),
    ] {
        let mut doc = Document::new();
        let container = add_positioned_container(
            &mut doc,
            WritingMode::HorizontalTb,
            Direction::Ltr,
            500,
            300,
        );
        let abspos = add_abspos(&mut doc, container, WritingMode::VerticalRl, Direction::Ltr);
        doc.update_resolved_style(abspos, |style| {
            style.update_derived(|computed| computed.left = Length::px(0.0));
            style.update_derived(|computed| computed.right = Length::px(0.0));
            style.update_derived(|computed| computed.top = Length::px(0.0));
            style.update_derived(|computed| computed.bottom = Length::px(0.0));
            style.update_derived(|computed| computed.width = intrinsic);
            style.update_derived(|computed| computed.height = Length::px(100.0));
            style.update_derived(|computed| computed.margin_left = Length::auto());
            style.update_derived(|computed| computed.margin_right = Length::auto());
            style.update_derived(|computed| computed.margin_top = Length::auto());
            style.update_derived(|computed| computed.margin_bottom = Length::auto());
            style.update_derived(|computed| computed.border_left_width = 5);
            style.update_derived(|computed| computed.border_right_width = 5);
            style.update_derived(|computed| computed.border_top_width = 5);
            style.update_derived(|computed| computed.border_bottom_width = 5);
            style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
            style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
            style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
            style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        });
        let inner = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(inner, |style| style.display = Display::Block);
        doc.update_resolved_style(inner, |style| style.writing_mode = WritingMode::VerticalRl);
        doc.update_resolved_style(inner, |style| style.width = Length::px(200.0));
        doc.append_child(abspos, inner);

        let root = layout_root(&doc);
        let container_fragment = fragment_for(&root, container);
        let positioned = fragment_for(container_fragment, abspos);
        assert_eq!(positioned.size.width, lu(210));
        assert_eq!(positioned.size.height, lu(110));
        assert_eq!(positioned.offset.left, lu(145));
        assert_eq!(positioned.offset.top, lu(95));
    }
}

#[test]
fn vertical_aspect_ratio_transfer_is_definite_for_percentage_descendants() {
    let mut doc = Document::new();
    let container = add_positioned_container(
        &mut doc,
        WritingMode::HorizontalTb,
        Direction::Ltr,
        300,
        200,
    );
    let abspos = add_abspos(&mut doc, container, WritingMode::VerticalLr, Direction::Ltr);
    doc.update_resolved_style(abspos, |style| {
        style.update_derived(|computed| computed.left = Length::px(0.0));
        style.update_derived(|computed| computed.right = Length::px(0.0));
        style.update_derived(|computed| computed.top = Length::px(0.0));
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| {
            computed.aspect_ratio = Some(openui_style::AspectRatio {
                ratio: (1.0, 1.0),
                auto_flag: false,
            })
        });
    });
    let percentage_child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(percentage_child, |style| style.display = Display::Block);
    doc.update_resolved_style(percentage_child, |style| {
        style.writing_mode = WritingMode::VerticalLr
    });
    doc.update_resolved_style(percentage_child, |style| style.width = Length::px(10.0));
    doc.update_resolved_style(percentage_child, |style| {
        style.height = Length::percent(50.0)
    });
    doc.append_child(abspos, percentage_child);

    let root = layout_root(&doc);
    let container_fragment = fragment_for(&root, container);
    let positioned = fragment_for(container_fragment, abspos);
    let percentage_fragment = fragment_for(positioned, percentage_child);
    assert_eq!(positioned.size.width, lu(100));
    assert_eq!(positioned.size.height, lu(100));
    assert_eq!(percentage_fragment.size.width, lu(10));
    assert_eq!(percentage_fragment.size.height, lu(50));
}
