//! SP17 W2A — authoritative rotated vertical and sideways text regressions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{
    block_layout, flex_layout, ConstraintSpace, Fragment, FragmentKind, TextRunOrientation,
};
use openui_style::{
    BorderStyle, Direction, Display, FlexDirection, FlexWrap, Float, FontFamilyList,
    TextOrientation, WritingMode,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    fn collect<'a>(fragment: &'a Fragment, output: &mut Vec<&'a Fragment>) {
        if fragment.kind == FragmentKind::Text {
            output.push(fragment);
        }
        for child in &fragment.children {
            collect(child, output);
        }
    }
    let mut output = Vec::new();
    collect(fragment, &mut output);
    output
}

fn wpt_like_text_item(
    writing_mode: WritingMode,
    direction: Direction,
    font_size: f32,
) -> (Document, NodeId, NodeId) {
    let mut doc =
        Document::new_with_font_collection(openui_text::FontCollection::deterministic_test());
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.width = Length::px(500.0));
        style.update_derived(|computed| computed.height = Length::px(150.0));
    });
    doc.append_child(doc.root(), container);

    let item = doc.create_node(ElementTag::Span);
    doc.update_resolved_style(item, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        style.update_derived(|computed| computed.direction = direction);
        style.update_derived(|computed| computed.text_orientation = TextOrientation::Mixed);
        style.update_derived(|computed| computed.height = Length::px(6.0)); // inline-size: 6px in vertical/sideways modes
        style.update_derived(|computed| computed.font_size = font_size);
        style.update_derived(|computed| computed.font_family = FontFamilyList::single("Ahem"));
        style.update_derived(|computed| computed.margin_top = Length::px(11.0));
        style.update_derived(|computed| computed.margin_right = Length::px(13.0));
        style.update_derived(|computed| computed.margin_bottom = Length::px(17.0));
        style.update_derived(|computed| computed.margin_left = Length::px(7.0));
        style.update_derived(|computed| computed.border_top_width = 2);
        style.update_derived(|computed| computed.border_right_width = 2);
        style.update_derived(|computed| computed.border_bottom_width = 2);
        style.update_derived(|computed| computed.border_left_width = 2);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    });
    doc.append_child(container, item);

    let text = doc.create_node(ElementTag::Text);
    doc.update_resolved_style(text, |style| {
        style.update_derived(|computed| computed.display = Display::Inline);
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        style.update_derived(|computed| computed.direction = direction);
        style.update_derived(|computed| computed.text_orientation = TextOrientation::Mixed);
        style.update_derived(|computed| computed.font_size = font_size);
        style.update_derived(|computed| computed.font_family = FontFamilyList::single("Ahem"));
    });
    doc.node_mut(text).text = Some(if font_size == 12.0 {
        "p b c".to_string()
    } else {
        "p e".to_string()
    });
    doc.append_child(item, text);
    (doc, container, item)
}

#[test]
fn ahem_advances_margins_inline_size_and_baselines_survive_rotation() {
    for writing_mode in [
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
        WritingMode::SidewaysRl,
        WritingMode::SidewaysLr,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            for (font_size, expected_width, expected_baseline) in
                [(12.0, 40, 9.609_375), (20.0, 44, 16.0)]
            {
                let (doc, container, item) = wpt_like_text_item(writing_mode, direction, font_size);
                let fragment = flex_layout(
                    &doc,
                    container,
                    &ConstraintSpace::for_root(lu(800), lu(600)),
                );
                let item = fragment
                    .children
                    .iter()
                    .find(|fragment| fragment.node_id == item)
                    .expect("text flex item");
                assert_eq!(
                    (item.width(), item.height()),
                    (lu(expected_width), lu(10)),
                    "{writing_mode:?}/{direction:?}/{font_size}"
                );
                assert_eq!(
                    (
                        item.margin.top,
                        item.margin.right,
                        item.margin.bottom,
                        item.margin.left
                    ),
                    (lu(11), lu(13), lu(17), lu(7))
                );

                let runs = text_fragments(item);
                assert_eq!(runs.len(), if font_size == 12.0 { 3 } else { 2 });
                let expected_orientation = if writing_mode == WritingMode::SidewaysLr {
                    TextRunOrientation::CounterClockwise
                } else {
                    TextRunOrientation::Clockwise
                };
                for run in runs {
                    assert_eq!(run.text_run_orientation, expected_orientation);
                    assert!((run.baseline_offset - expected_baseline).abs() < 0.001);
                    let advance = run.shape_result.as_ref().expect("shape result").width();
                    assert!(
                        (advance - font_size).abs() < 0.05,
                        "horizontal advance {advance} for {font_size}px Ahem"
                    );
                    assert!((run.width().to_f32() - font_size).abs() < 0.05);
                    assert!((run.height().to_f32() - font_size).abs() < 0.05);
                }
            }
        }
    }
}

fn flex_positions(
    writing_mode: WritingMode,
    direction: Direction,
    flex_direction: FlexDirection,
    flex_wrap: FlexWrap,
) -> Vec<(LayoutUnit, LayoutUnit)> {
    let mut doc = Document::new();
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        style.update_derived(|computed| computed.direction = direction);
        style.update_derived(|computed| computed.flex_direction = flex_direction);
        style.update_derived(|computed| computed.flex_wrap = flex_wrap);
        style.update_derived(|computed| computed.width = Length::px(40.0));
        style.update_derived(|computed| computed.height = Length::px(40.0));
    });
    doc.append_child(doc.root(), container);
    let mut ids = Vec::new();
    for _ in 0..4 {
        let item = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(item, |style| {
            style.display = Display::Block;
            style.width = Length::px(10.0);
            style.height = Length::px(20.0);
            style.flex_shrink = 0.0;
        });
        doc.append_child(container, item);
        ids.push(item);
    }
    let fragment = flex_layout(&doc, container, &ConstraintSpace::for_root(lu(80), lu(80)));
    ids.into_iter()
        .map(|id| {
            let item = fragment
                .children
                .iter()
                .find(|fragment| fragment.node_id == id)
                .expect("flow item");
            (item.offset.left, item.offset.top)
        })
        .collect()
}

#[test]
fn sideways_lr_rtl_flipping_is_independent_of_reverse_flow_and_wrapping() {
    for flex_direction in [
        FlexDirection::Row,
        FlexDirection::RowReverse,
        FlexDirection::Column,
        FlexDirection::ColumnReverse,
    ] {
        for flex_wrap in [FlexWrap::Wrap, FlexWrap::WrapReverse] {
            let positions = flex_positions(
                WritingMode::SidewaysLr,
                Direction::Rtl,
                flex_direction,
                flex_wrap,
            );
            let mut unique = positions.clone();
            unique.sort();
            unique.dedup();
            assert_eq!(unique.len(), 4, "{flex_direction:?}/{flex_wrap:?}");
            assert!(positions.iter().all(|(left, top)| {
                *left >= LayoutUnit::zero()
                    && *top >= LayoutUnit::zero()
                    && *left < lu(40)
                    && *top < lu(40)
            }));
        }
    }

    let row = flex_positions(
        WritingMode::SidewaysLr,
        Direction::Rtl,
        FlexDirection::Row,
        FlexWrap::Wrap,
    );
    let row_reverse = flex_positions(
        WritingMode::SidewaysLr,
        Direction::Rtl,
        FlexDirection::RowReverse,
        FlexWrap::Wrap,
    );
    assert_eq!(
        row_reverse,
        vec![row[1], row[0], row[3], row[2]],
        "row-reverse must reverse each wrapped inline line exactly once"
    );
}

#[test]
fn auto_sized_flex_padding_projects_once_across_writing_modes() {
    for writing_mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
        WritingMode::SidewaysLr,
        WritingMode::SidewaysRl,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            for (side, amount, expected) in [
                ("top", 3, (16, 19)),
                ("right", 4, (20, 16)),
                ("bottom", 5, (16, 21)),
                ("left", 6, (22, 16)),
            ] {
                let mut doc = Document::new();
                let container = doc.create_node(ElementTag::Div);
                doc.update_resolved_style(container, |style| {
                    style.update_derived(|computed| computed.display = Display::Flex);
                    style.update_derived(|computed| computed.float = Float::Left);
                    style.update_derived(|computed| computed.writing_mode = writing_mode);
                    style.update_derived(|computed| computed.direction = direction);
                    style.update_derived(|computed| computed.flex_direction = FlexDirection::Row);
                    style.update_derived(|computed| computed.border_top_width = 2);
                    style.update_derived(|computed| computed.border_right_width = 2);
                    style.update_derived(|computed| computed.border_bottom_width = 2);
                    style.update_derived(|computed| computed.border_left_width = 2);
                    style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
                    style.update_derived(|computed| {
                        computed.border_right_style = BorderStyle::Solid
                    });
                    style.update_derived(|computed| {
                        computed.border_bottom_style = BorderStyle::Solid
                    });
                    style
                        .update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
                });
                doc.append_child(doc.root(), container);
                let item = doc.create_node(ElementTag::Div);
                doc.update_resolved_style(item, |style| {
                    style.update_derived(|computed| computed.display = Display::Block);
                    style.update_derived(|computed| computed.width = Length::px(10.0));
                    style.update_derived(|computed| computed.height = Length::px(10.0));
                    style.update_derived(|computed| computed.border_top_width = 1);
                    style.update_derived(|computed| computed.border_right_width = 1);
                    style.update_derived(|computed| computed.border_bottom_width = 1);
                    style.update_derived(|computed| computed.border_left_width = 1);
                    style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
                    style.update_derived(|computed| {
                        computed.border_right_style = BorderStyle::Solid
                    });
                    style.update_derived(|computed| {
                        computed.border_bottom_style = BorderStyle::Solid
                    });
                    style
                        .update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
                    match side {
                        "top" => style.update_derived(|computed| {
                            computed.padding_top = Length::px(amount as f32)
                        }),
                        "right" => style.update_derived(|computed| {
                            computed.padding_right = Length::px(amount as f32)
                        }),
                        "bottom" => style.update_derived(|computed| {
                            computed.padding_bottom = Length::px(amount as f32)
                        }),
                        "left" => style.update_derived(|computed| {
                            computed.padding_left = Length::px(amount as f32)
                        }),
                        _ => unreachable!(),
                    }
                });
                doc.append_child(container, item);
                let root = block_layout(
                    &doc,
                    doc.root(),
                    &ConstraintSpace::for_root(lu(100), lu(100)),
                );
                let fragment = root
                    .children
                    .iter()
                    .find(|fragment| fragment.node_id == container)
                    .expect("shrink-to-fit flex float");
                assert_eq!(
                    (fragment.width(), fragment.height()),
                    (lu(expected.0), lu(expected.1)),
                    "{writing_mode:?}/{direction:?}/{side}"
                );
            }
        }
    }
}
