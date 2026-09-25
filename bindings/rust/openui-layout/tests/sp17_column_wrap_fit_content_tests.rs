//! SP17 W1N — fit-content cross sizing for wrapped column flex items.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length, INDEFINITE_SIZE};
use openui_layout::{flex_layout, ConstraintSpace, Fragment};
use openui_style::{
    BorderStyle, BoxSizing, ContentAlignment, ContentPosition, Direction, Display, FlexDirection,
    FlexWrap, ItemAlignment, ItemPosition, WritingMode,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn fragment_for(parent: &Fragment, node_id: NodeId) -> &Fragment {
    parent
        .children
        .iter()
        .find(|fragment| fragment.node_id == node_id)
        .unwrap_or_else(|| panic!("missing fragment for {node_id:?}"))
}

fn make_column_flex(
    doc: &mut Document,
    physical_width: i32,
    physical_height: i32,
    writing_mode: WritingMode,
    direction: Direction,
) -> NodeId {
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.width = Length::px(physical_width as f32));
        style.update_derived(|computed| computed.height = Length::px(physical_height as f32));
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.flex_wrap = FlexWrap::Wrap);
        style.update_derived(|computed| {
            computed.align_items = ItemAlignment::new(ItemPosition::FlexStart)
        });
        style.update_derived(|computed| {
            computed.align_content = ContentAlignment::new(ContentPosition::FlexStart)
        });
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        style.update_derived(|computed| computed.direction = direction);
    });
    doc.append_child(doc.root(), container);
    container
}

fn add_intrinsic_item(
    doc: &mut Document,
    container: NodeId,
    writing_mode: WritingMode,
    direction: Direction,
    physical_main_size: i32,
    atom_inline_size: i32,
    atom_count: usize,
) -> NodeId {
    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        style.update_derived(|computed| computed.direction = direction);
        style.update_derived(|computed| computed.flex_grow = 0.0);
        style.update_derived(|computed| computed.flex_shrink = 0.0);
        if writing_mode == WritingMode::HorizontalTb {
            style
                .update_derived(|computed| computed.height = Length::px(physical_main_size as f32));
            style.update_derived(|computed| computed.min_height = Length::zero());
        } else {
            style.update_derived(|computed| computed.width = Length::px(physical_main_size as f32));
            style.update_derived(|computed| computed.min_width = Length::zero());
        }
    });
    doc.append_child(container, item);

    for _ in 0..atom_count {
        let atom = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(atom, |style| {
            style.update_derived(|computed| computed.display = Display::InlineBlock);
            style.update_derived(|computed| computed.writing_mode = writing_mode);
            style.update_derived(|computed| computed.direction = direction);
            if writing_mode == WritingMode::HorizontalTb {
                style.update_derived(|computed| {
                    computed.width = Length::px(atom_inline_size as f32)
                });
                style.update_derived(|computed| computed.height = Length::zero());
            } else {
                style.update_derived(|computed| computed.width = Length::zero());
                style.update_derived(|computed| {
                    computed.height = Length::px(atom_inline_size as f32)
                });
            }
        });
        doc.append_child(item, atom);
    }
    item
}

fn layout(doc: &Document, writing_mode: WritingMode, direction: Direction) -> Fragment {
    let writing_direction = direction.writing_direction(writing_mode);
    let space =
        ConstraintSpace::for_root_with_writing_direction(lu(800), lu(600), writing_direction);
    flex_layout(doc, doc.children(doc.root()).next().unwrap(), &space)
}

#[test]
fn auto_cross_size_clamps_available_space_to_intrinsic_bounds() {
    for (available, expected) in [(25, 50), (100, 100), (300, 250)] {
        let mut doc = Document::new();
        let container = make_column_flex(
            &mut doc,
            available,
            100,
            WritingMode::HorizontalTb,
            Direction::Ltr,
        );
        let item = add_intrinsic_item(
            &mut doc,
            container,
            WritingMode::HorizontalTb,
            Direction::Ltr,
            100,
            50,
            5,
        );

        let fragment = layout(&doc, WritingMode::HorizontalTb, Direction::Ltr);
        assert_eq!(
            fragment_for(&fragment, item).width(),
            lu(expected),
            "fit-content width for {available}px of available cross space",
        );
    }
}

#[test]
fn indefinite_cross_space_uses_max_content() {
    let mut doc = Document::new();
    let container = make_column_flex(&mut doc, 0, 100, WritingMode::HorizontalTb, Direction::Ltr);
    doc.update_resolved_style(container, |style| style.width = Length::auto());
    let item = add_intrinsic_item(
        &mut doc,
        container,
        WritingMode::HorizontalTb,
        Direction::Ltr,
        100,
        50,
        5,
    );
    let space = ConstraintSpace::for_root(INDEFINITE_SIZE, lu(600));
    let fragment = flex_layout(&doc, container, &space);

    assert_eq!(fragment_for(&fragment, item).width(), lu(250));
}

#[test]
fn wrapped_lines_and_centered_line_group_keep_fit_content_widths() {
    let mut two_line_doc = Document::new();
    let container = make_column_flex(
        &mut two_line_doc,
        50,
        100,
        WritingMode::HorizontalTb,
        Direction::Ltr,
    );
    let first = add_intrinsic_item(
        &mut two_line_doc,
        container,
        WritingMode::HorizontalTb,
        Direction::Ltr,
        100,
        25,
        5,
    );
    let second = add_intrinsic_item(
        &mut two_line_doc,
        container,
        WritingMode::HorizontalTb,
        Direction::Ltr,
        100,
        25,
        5,
    );
    let fragment = layout(&two_line_doc, WritingMode::HorizontalTb, Direction::Ltr);
    assert_eq!(fragment_for(&fragment, first).width(), lu(50));
    assert_eq!(fragment_for(&fragment, second).offset.left, lu(50));

    let mut one_line_doc = Document::new();
    let container = make_column_flex(
        &mut one_line_doc,
        100,
        100,
        WritingMode::HorizontalTb,
        Direction::Ltr,
    );
    let item = add_intrinsic_item(
        &mut one_line_doc,
        container,
        WritingMode::HorizontalTb,
        Direction::Ltr,
        100,
        50,
        5,
    );
    let fragment = layout(&one_line_doc, WritingMode::HorizontalTb, Direction::Ltr);
    assert_eq!(fragment_for(&fragment, item).width(), lu(100));

    let mut centered_doc = Document::new();
    let container = make_column_flex(
        &mut centered_doc,
        100,
        100,
        WritingMode::HorizontalTb,
        Direction::Ltr,
    );
    centered_doc.update_resolved_style(container, |style| {
        style.align_content = ContentAlignment::new(ContentPosition::Center)
    });
    let narrow = add_intrinsic_item(
        &mut centered_doc,
        container,
        WritingMode::HorizontalTb,
        Direction::Ltr,
        100,
        20,
        1,
    );
    let wide = add_intrinsic_item(
        &mut centered_doc,
        container,
        WritingMode::HorizontalTb,
        Direction::Ltr,
        100,
        60,
        1,
    );
    let fragment = layout(&centered_doc, WritingMode::HorizontalTb, Direction::Ltr);
    assert_eq!(fragment_for(&fragment, narrow).offset.left, lu(10));
    assert_eq!(fragment_for(&fragment, wide).offset.left, lu(30));
}

#[test]
fn specified_cross_margins_reduce_fit_space_and_auto_margins_are_zero() {
    for (auto_margin, expected_width) in [(false, 100), (true, 125)] {
        let mut doc = Document::new();
        let container = make_column_flex(
            &mut doc,
            125,
            100,
            WritingMode::HorizontalTb,
            Direction::Ltr,
        );
        let item = add_intrinsic_item(
            &mut doc,
            container,
            WritingMode::HorizontalTb,
            Direction::Ltr,
            100,
            50,
            5,
        );
        doc.update_resolved_style(item, |style| {
            style.margin_right = if auto_margin {
                Length::auto()
            } else {
                Length::px(25.0)
            }
        });

        let fragment = layout(&doc, WritingMode::HorizontalTb, Direction::Ltr);
        assert_eq!(fragment_for(&fragment, item).width(), lu(expected_width),);
    }
}

#[test]
fn border_padding_and_cross_min_max_are_applied_once() {
    for (available, expected_border_box) in [(50, 134), (200, 174)] {
        let mut doc = Document::new();
        let container = make_column_flex(
            &mut doc,
            available,
            100,
            WritingMode::HorizontalTb,
            Direction::Ltr,
        );
        let item = add_intrinsic_item(
            &mut doc,
            container,
            WritingMode::HorizontalTb,
            Direction::Ltr,
            100,
            50,
            5,
        );
        doc.update_resolved_style(item, |style| {
            style.update_derived(|computed| computed.box_sizing = BoxSizing::ContentBox);
            style.update_derived(|computed| computed.min_width = Length::px(120.0));
            style.update_derived(|computed| computed.max_width = Length::px(160.0));
            style.update_derived(|computed| computed.padding_left = Length::px(5.0));
            style.update_derived(|computed| computed.padding_right = Length::px(5.0));
            style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
            style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
            style.update_derived(|computed| computed.border_left_width = 2);
            style.update_derived(|computed| computed.border_right_width = 2);
        });

        let fragment = layout(&doc, WritingMode::HorizontalTb, Direction::Ltr);
        assert_eq!(
            fragment_for(&fragment, item).width(),
            lu(expected_border_box),
            "content-box min/max must each receive the 14px outer edge exactly once",
        );
    }
}

#[test]
fn logical_cross_axis_is_preserved_across_flow_reversals_and_writing_modes() {
    let modes = [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ];
    let directions = [Direction::Ltr, Direction::Rtl];
    let flex_directions = [FlexDirection::Column, FlexDirection::ColumnReverse];
    let wraps = [FlexWrap::Wrap, FlexWrap::WrapReverse];

    for writing_mode in modes {
        for direction in directions {
            for flex_direction in flex_directions {
                for flex_wrap in wraps {
                    let (container_width, container_height, expected_width, expected_height) =
                        if writing_mode == WritingMode::HorizontalTb {
                            (80, 60, 80, 60)
                        } else {
                            (60, 80, 60, 80)
                        };
                    let mut doc = Document::new();
                    let container = make_column_flex(
                        &mut doc,
                        container_width,
                        container_height,
                        writing_mode,
                        direction,
                    );
                    doc.update_resolved_style(container, |style| {
                        style.update_derived(|computed| computed.flex_direction = flex_direction);
                        style.update_derived(|computed| computed.flex_wrap = flex_wrap);
                    });
                    let first =
                        add_intrinsic_item(&mut doc, container, writing_mode, direction, 60, 25, 5);
                    let second =
                        add_intrinsic_item(&mut doc, container, writing_mode, direction, 60, 25, 5);

                    let fragment = layout(&doc, writing_mode, direction);
                    let first = fragment_for(&fragment, first);
                    let second = fragment_for(&fragment, second);
                    for item in [first, second] {
                        assert_eq!(
                            (item.width(), item.height()),
                            (lu(expected_width), lu(expected_height)),
                            "wrong physical projection for {writing_mode:?}/{direction:?}/{flex_direction:?}/{flex_wrap:?}",
                        );
                    }
                    let cross_distance = if writing_mode == WritingMode::HorizontalTb {
                        (first.offset.left - second.offset.left).abs()
                    } else {
                        (first.offset.top - second.offset.top).abs()
                    };
                    assert_eq!(
                        cross_distance,
                        lu(80),
                        "wrapped lines lost the logical inline axis for {writing_mode:?}/{direction:?}/{flex_direction:?}/{flex_wrap:?}",
                    );
                }
            }
        }
    }
}
