//! SP17 W1J — vertical multicol positioned-fragmentation regressions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length, PhysicalSize};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{
    BorderStyle, BoxSizing, ColumnFill, ComputedStyle, Direction, Display, Position, WritingMode,
};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn descendants_for<'a>(fragment: &'a Fragment, node: NodeId, out: &mut Vec<&'a Fragment>) {
    if fragment.node_id == node {
        out.push(fragment);
    }
    for child in &fragment.children {
        descendants_for(child, node, out);
    }
}

fn fragments_for(fragment: &Fragment, node: NodeId) -> Vec<&Fragment> {
    let mut found = Vec::new();
    descendants_for(fragment, node, &mut found);
    found
}

fn set_vertical_logical_size(style: &mut ComputedStyle, inline: Length, block: Length) {
    style.height = inline;
    style.width = block;
}

fn set_logical_starts(
    style: &mut ComputedStyle,
    mode: WritingMode,
    direction: Direction,
    inline: f32,
    block: f32,
) {
    if direction == Direction::Ltr {
        style.top = Length::px(inline);
    } else {
        style.bottom = Length::px(inline);
    }
    if mode == WritingMode::VerticalLr {
        style.left = Length::px(block);
    } else {
        style.right = Length::px(block);
    }
}

fn vertical_multicol_document(mode: WritingMode, direction: Direction) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let multicol = doc.create_node(ElementTag::Div);
    let style = &mut doc.node_mut(multicol).style;
    style.display = Display::Block;
    style.writing_mode = mode;
    style.direction = direction;
    style.width = Length::px(100.0);
    style.height = Length::px(100.0);
    style.column_count = Some(2);
    style.column_fill = ColumnFill::Auto;
    style.column_gap = Some(Length::px(0.0));
    doc.append_child(root, multicol);
    (doc, multicol)
}

fn layout(doc: &Document) -> Fragment {
    block_layout(
        doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(800), lu(600)),
    )
}

#[test]
fn fixed_box_after_block_in_inline_has_two_source_ordered_continuations() {
    for mode in [WritingMode::VerticalLr, WritingMode::VerticalRl] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            for translated in [false, true] {
                let (mut doc, multicol) = vertical_multicol_document(mode, direction);

                let wrapper = doc.create_node(ElementTag::Div);
                doc.node_mut(wrapper).style.display = Display::Block;
                doc.node_mut(wrapper).style.writing_mode = mode;
                doc.node_mut(wrapper).style.direction = direction;
                doc.append_child(multicol, wrapper);

                let containing_inline = doc.create_node(ElementTag::Span);
                doc.node_mut(containing_inline).style.display = Display::Inline;
                doc.node_mut(containing_inline).style.position = Position::Relative;
                doc.node_mut(containing_inline).style.writing_mode = mode;
                doc.node_mut(containing_inline).style.direction = direction;
                if translated {
                    set_logical_starts(
                        &mut doc.node_mut(containing_inline).style,
                        mode,
                        direction,
                        5.0,
                        10.0,
                    );
                }
                doc.append_child(wrapper, containing_inline);

                let interruption = doc.create_node(ElementTag::Div);
                doc.node_mut(interruption).style.display = Display::Block;
                doc.node_mut(interruption).style.writing_mode = mode;
                doc.node_mut(interruption).style.direction = direction;
                set_vertical_logical_size(
                    &mut doc.node_mut(interruption).style,
                    Length::px(50.0),
                    Length::px(200.0),
                );
                doc.append_child(containing_inline, interruption);

                let positioned = doc.create_node(ElementTag::Div);
                doc.node_mut(positioned).style.display = Display::Block;
                doc.node_mut(positioned).style.position = Position::Absolute;
                doc.node_mut(positioned).style.writing_mode = mode;
                doc.node_mut(positioned).style.direction = direction;
                set_vertical_logical_size(
                    &mut doc.node_mut(positioned).style,
                    Length::px(50.0),
                    Length::px(200.0),
                );
                set_logical_starts(
                    &mut doc.node_mut(positioned).style,
                    mode,
                    direction,
                    0.0,
                    0.0,
                );
                doc.append_child(containing_inline, positioned);

                let result = layout(&doc);
                let fragments = fragments_for(&result, positioned);
                assert_eq!(fragments.len(), 2, "{mode:?}/{direction:?}/{translated}");
                assert_eq!(
                    fragments
                        .iter()
                        .map(|fragment| fragment.size)
                        .collect::<Vec<_>>(),
                    vec![PhysicalSize::new(lu(100), lu(50)); 2],
                    "{mode:?}/{direction:?}/{translated}"
                );
                assert_eq!(
                    fragments
                        .iter()
                        .map(|fragment| fragment.decoration_slice.unwrap().source_block_offset)
                        .collect::<Vec<_>>(),
                    vec![lu(0), lu(100)],
                    "{mode:?}/{direction:?}/{translated}"
                );
                assert_eq!(
                    fragments
                        .iter()
                        .map(|fragment| {
                            fragment
                                .positioned_fragmentation
                                .unwrap()
                                .fragmentainer_index
                        })
                        .collect::<Vec<_>>(),
                    vec![Some(0), Some(1)],
                    "{mode:?}/{direction:?}/{translated}"
                );
                assert!(fragments[0].is_first_for_node);
                assert!(!fragments[0].is_last_for_node);
                assert!(!fragments[1].is_first_for_node);
                assert!(fragments[1].is_last_for_node);
                assert!(fragments.iter().all(|fragment| {
                    fragment.has_overflow_clip
                        && fragment.block_axis_clip_only
                        && fragment.children.is_empty()
                }));

                let block_delta = if translated {
                    if mode == WritingMode::VerticalLr {
                        lu(10)
                    } else {
                        lu(-10)
                    }
                } else {
                    lu(0)
                };
                let inline_delta = if translated {
                    if direction == Direction::Ltr {
                        lu(5)
                    } else {
                        lu(-5)
                    }
                } else {
                    lu(0)
                };
                let expected_tops = if direction == Direction::Ltr {
                    [lu(0) + inline_delta, lu(50) + inline_delta]
                } else {
                    [lu(50) + inline_delta, lu(0) + inline_delta]
                };
                assert_eq!(fragments[0].offset.left, block_delta);
                assert_eq!(fragments[1].offset.left, block_delta);
                assert_eq!(fragments[0].offset.top, expected_tops[0]);
                assert_eq!(fragments[1].offset.top, expected_tops[1]);
            }
        }
    }
}

#[test]
fn stretched_box_uses_fragmented_block_and_flex_containing_blocks() {
    for mode in [WritingMode::VerticalLr, WritingMode::VerticalRl] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            for display in [Display::Block, Display::Flex] {
                let (mut doc, multicol) = vertical_multicol_document(mode, direction);

                let containing_block = doc.create_node(ElementTag::Div);
                {
                    let style = &mut doc.node_mut(containing_block).style;
                    style.display = display;
                    style.position = Position::Relative;
                    style.writing_mode = mode;
                    style.direction = direction;
                    style.box_sizing = BoxSizing::ContentBox;
                    set_vertical_logical_size(style, Length::px(50.0), Length::px(160.0));
                    if mode == WritingMode::VerticalLr {
                        style.border_left_width = 20;
                        style.border_right_width = 10;
                    } else {
                        style.border_right_width = 20;
                        style.border_left_width = 10;
                    }
                    style.border_left_style = BorderStyle::Solid;
                    style.border_right_style = BorderStyle::Solid;
                }
                doc.append_child(multicol, containing_block);

                let positioned = doc.create_node(ElementTag::Div);
                {
                    let style = &mut doc.node_mut(positioned).style;
                    style.display = Display::Block;
                    style.position = Position::Absolute;
                    style.writing_mode = mode;
                    style.direction = direction;
                    set_vertical_logical_size(style, Length::percent(100.0), Length::auto());
                    style.left = Length::px(0.0);
                    style.right = Length::px(0.0);
                    if direction == Direction::Ltr {
                        style.top = Length::px(0.0);
                    } else {
                        style.bottom = Length::px(0.0);
                    }
                }
                doc.append_child(containing_block, positioned);

                let result = layout(&doc);
                let fragments = fragments_for(&result, positioned);
                assert_eq!(fragments.len(), 2, "{mode:?}/{direction:?}/{display:?}");
                assert_eq!(
                    fragments
                        .iter()
                        .map(|fragment| fragment.decoration_slice.unwrap().source_block_offset)
                        .collect::<Vec<_>>(),
                    vec![lu(0), lu(80)],
                    "{mode:?}/{direction:?}/{display:?}"
                );
                assert!(fragments.iter().all(|fragment| {
                    fragment.size == PhysicalSize::new(lu(80), lu(50))
                        && fragment.has_overflow_clip
                        && fragment.block_axis_clip_only
                        && fragment
                            .decoration_slice
                            .is_some_and(|slice| slice.source_block_size == lu(160))
                }));
                assert_eq!(
                    fragments
                        .iter()
                        .map(|fragment| {
                            fragment
                                .positioned_fragmentation
                                .unwrap()
                                .fragmentainer_index
                        })
                        .collect::<Vec<_>>(),
                    vec![Some(0), Some(1)],
                    "{mode:?}/{direction:?}/{display:?}"
                );
                assert!(fragments[0].is_first_for_node);
                assert!(!fragments[0].is_last_for_node);
                assert!(!fragments[1].is_first_for_node);
                assert!(fragments[1].is_last_for_node);
            }
        }
    }
}
