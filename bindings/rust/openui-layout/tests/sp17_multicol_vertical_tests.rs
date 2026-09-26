//! SP17 W1H — logical multicol and vertical fragmentation regressions.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    BorderStyle, Color, ColumnFill, Direction, Display, Float, Overflow, Position, WritingMode,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn layout_multicol(mode: WritingMode, direction: Direction) -> Fragment {
    let mut doc = Document::new();
    let root = doc.root();
    doc.update_resolved_style(root, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.writing_mode = mode);
        style.update_derived(|computed| computed.direction = direction);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.border_top_width = 1);
        style.update_derived(|computed| computed.border_right_width = 2);
        style.update_derived(|computed| computed.border_bottom_width = 3);
        style.update_derived(|computed| computed.border_left_width = 4);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.padding_top = Length::px(5.0));
        style.update_derived(|computed| computed.padding_right = Length::px(6.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(7.0));
        style.update_derived(|computed| computed.padding_left = Length::px(8.0));
    });

    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.writing_mode = mode);
        style.update_derived(|computed| computed.direction = direction);
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(0, 128, 0, 255)
        });
        if mode == WritingMode::HorizontalTb {
            style.update_derived(|computed| computed.height = Length::px(200.0));
        } else {
            style.update_derived(|computed| computed.width = Length::px(200.0));
        }
    });
    doc.append_child(root, child);

    let writing_direction = direction.writing_direction(mode);
    block_layout(
        &doc,
        root,
        &ConstraintSpace::for_root_with_writing_direction(lu(800), lu(600), writing_direction),
    )
}

#[test]
fn column_projection_covers_three_modes_and_both_directions() {
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let fragment = layout_multicol(mode, direction);
            let columns: Vec<_> = fragment
                .children
                .iter()
                .filter(|child| child.kind == FragmentKind::ColumnBox)
                .collect();
            assert_eq!(columns.len(), 2, "{mode:?}/{direction:?}");

            if mode == WritingMode::HorizontalTb {
                assert_eq!(columns[0].size.width, lu(50));
                assert_eq!(columns[0].size.height, lu(100));
                let expected = if direction == Direction::Ltr {
                    [lu(12), lu(62)]
                } else {
                    [lu(62), lu(12)]
                };
                assert_eq!(
                    [columns[0].offset.left, columns[1].offset.left],
                    expected,
                    "{mode:?}/{direction:?}"
                );
            } else {
                assert_eq!(columns[0].size.width, lu(100));
                assert_eq!(columns[0].size.height, lu(50));
                let expected = if direction == Direction::Ltr {
                    [lu(6), lu(56)]
                } else {
                    [lu(56), lu(6)]
                };
                assert_eq!(
                    [columns[0].offset.top, columns[1].offset.top],
                    expected,
                    "{mode:?}/{direction:?}"
                );
            }
            assert!(columns.iter().all(|column| {
                column.fragmentation_writing_direction == Some(direction.writing_direction(mode))
            }));

            assert_eq!(fragment.border.top, lu(1));
            assert_eq!(fragment.border.right, lu(2));
            assert_eq!(fragment.border.bottom, lu(3));
            assert_eq!(fragment.border.left, lu(4));
            assert_eq!(fragment.padding.top, lu(5));
            assert_eq!(fragment.padding.right, lu(6));
            assert_eq!(fragment.padding.bottom, lu(7));
            assert_eq!(fragment.padding.left, lu(8));
        }
    }
}

#[test]
fn vertical_balance_uses_visible_logical_block_overflow() {
    let mut doc = Document::new();
    let root = doc.root();
    doc.update_resolved_style(root, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.writing_mode = WritingMode::VerticalRl);
        style.update_derived(|computed| computed.width = Length::auto());
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
    });
    let wrapper = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(wrapper, |style| style.display = Display::Block);
    doc.update_resolved_style(wrapper, |style| {
        style.writing_mode = WritingMode::VerticalRl
    });
    doc.update_resolved_style(wrapper, |style| style.width = Length::px(20.0));
    doc.append_child(root, wrapper);
    let overflow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(overflow, |style| style.display = Display::Block);
    doc.update_resolved_style(overflow, |style| {
        style.writing_mode = WritingMode::VerticalRl
    });
    doc.update_resolved_style(overflow, |style| style.width = Length::px(200.0));
    doc.append_child(wrapper, overflow);

    let direction = Direction::Ltr.writing_direction(WritingMode::VerticalRl);
    let fragment = block_layout(
        &doc,
        root,
        &ConstraintSpace::for_root_with_writing_direction(lu(800), lu(600), direction),
    );
    assert_eq!(fragment.size.width, lu(100));
    assert_eq!(fragment.size.height, lu(100));
    assert_eq!(
        fragment
            .children
            .iter()
            .filter(|child| child.kind == FragmentKind::ColumnBox)
            .count(),
        2
    );
}

#[test]
fn orthogonal_spanner_shrink_wraps_its_own_inline_axis() {
    let mut doc = Document::new();
    let root = doc.root();
    doc.update_resolved_style(root, |style| style.display = Display::Block);
    doc.update_resolved_style(root, |style| style.width = Length::px(100.0));
    doc.update_resolved_style(root, |style| style.column_count = Some(4));
    doc.update_resolved_style(root, |style| style.column_gap = Some(Length::px(0.0)));

    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| style.display = Display::Block);
    doc.update_resolved_style(spanner, |style| {
        style.writing_mode = WritingMode::VerticalRl
    });
    doc.update_resolved_style(spanner, |style| {
        style.column_span = openui_style::ColumnSpan::All
    });
    doc.append_child(root, spanner);
    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| style.display = Display::Block);
    doc.update_resolved_style(child, |style| style.writing_mode = WritingMode::VerticalRl);
    doc.update_resolved_style(child, |style| style.width = Length::px(100.0));
    doc.update_resolved_style(child, |style| style.height = Length::px(50.0));
    doc.append_child(spanner, child);

    let fragment = block_layout(&doc, root, &ConstraintSpace::for_root(lu(800), lu(600)));
    let spanner_fragment = fragment
        .children
        .iter()
        .find(|fragment| fragment.node_id == spanner)
        .expect("spanner fragment");
    assert_eq!(spanner_fragment.size.width, lu(100));
    assert_eq!(spanner_fragment.size.height, lu(50));
}

#[test]
fn bubbled_horizontal_rtl_oof_uses_the_physical_content_left_edge() {
    let mut doc = Document::new();
    let root = doc.root();
    doc.update_resolved_style(root, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });

    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.direction = Direction::Rtl);
        style.update_derived(|computed| computed.width = Length::px(32.0));
        style.update_derived(|computed| computed.height = Length::px(180.0));
        style.update_derived(|computed| computed.border_left_width = 10);
        style.update_derived(|computed| computed.border_right_width = 8);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
    });
    doc.append_child(root, child);

    let containing_block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(containing_block, |style| style.display = Display::Block);
    doc.update_resolved_style(containing_block, |style| {
        style.position = Position::Relative
    });
    doc.append_child(child, containing_block);

    let abspos = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(abspos, |style| style.display = Display::Block);
    doc.update_resolved_style(abspos, |style| style.position = Position::Absolute);
    doc.update_resolved_style(abspos, |style| style.width = Length::px(22.0));
    doc.update_resolved_style(abspos, |style| style.height = Length::px(180.0));
    doc.append_child(containing_block, abspos);

    let fragment = block_layout(&doc, root, &ConstraintSpace::for_root(lu(800), lu(600)));
    let positioned: Vec<_> = fragment
        .children
        .iter()
        .filter(|fragment| fragment.node_id == abspos)
        .collect();
    assert_eq!(positioned.len(), 2);
    assert_eq!(positioned[0].offset.left, lu(10));
    assert_eq!(positioned[1].offset.left, lu(60));
}

#[test]
fn horizontal_rtl_keeps_physical_left_floats_on_the_left() {
    let mut doc = Document::new();
    let root = doc.root();
    doc.update_resolved_style(root, |style| style.display = Display::Block);
    doc.update_resolved_style(root, |style| style.direction = Direction::Rtl);
    doc.update_resolved_style(root, |style| style.width = Length::px(100.0));

    let floated = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(floated, |style| style.display = Display::Block);
    doc.update_resolved_style(floated, |style| style.float = Float::Left);
    doc.update_resolved_style(floated, |style| style.width = Length::px(20.0));
    doc.update_resolved_style(floated, |style| style.height = Length::px(20.0));
    doc.append_child(root, floated);

    let direction = Direction::Rtl.writing_direction(WritingMode::HorizontalTb);
    let fragment = block_layout(
        &doc,
        root,
        &ConstraintSpace::for_root_with_writing_direction(lu(800), lu(600), direction),
    );
    let floated_fragment = fragment
        .children
        .iter()
        .find(|fragment| fragment.node_id == floated)
        .expect("float fragment");
    assert_eq!(floated_fragment.offset.left, LayoutUnit::zero());
}

#[test]
fn horizontal_rtl_keeps_float_avoidance_offsets_physical() {
    let mut doc = Document::new();
    let root = doc.root();
    doc.update_resolved_style(root, |style| style.display = Display::Block);
    doc.update_resolved_style(root, |style| style.direction = Direction::Rtl);
    doc.update_resolved_style(root, |style| style.width = Length::px(100.0));
    doc.update_resolved_style(root, |style| style.height = Length::px(100.0));

    let floated = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(floated, |style| style.display = Display::Block);
    doc.update_resolved_style(floated, |style| style.float = Float::Left);
    doc.update_resolved_style(floated, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(floated, |style| style.height = Length::px(100.0));
    doc.append_child(root, floated);

    let formatting_context = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(formatting_context, |style| style.display = Display::Block);
    doc.update_resolved_style(formatting_context, |style| {
        style.overflow_x = Overflow::Hidden
    });
    doc.update_resolved_style(formatting_context, |style| {
        style.overflow_y = Overflow::Hidden
    });
    doc.update_resolved_style(formatting_context, |style| {
        style.margin_left = Length::px(-20.0)
    });
    doc.update_resolved_style(formatting_context, |style| style.height = Length::px(100.0));
    doc.append_child(root, formatting_context);

    let direction = Direction::Rtl.writing_direction(WritingMode::HorizontalTb);
    let fragment = block_layout(
        &doc,
        root,
        &ConstraintSpace::for_root_with_writing_direction(lu(800), lu(600), direction),
    );
    let formatting_context_fragment = fragment
        .children
        .iter()
        .find(|fragment| fragment.node_id == formatting_context)
        .expect("formatting-context fragment");
    assert_eq!(formatting_context_fragment.offset.left, lu(50));
    assert_eq!(formatting_context_fragment.size.width, lu(50));
}
