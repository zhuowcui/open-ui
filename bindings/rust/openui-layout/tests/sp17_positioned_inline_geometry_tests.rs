//! SP17 W1I — positioned-inline static geometry regressions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{BoxStrut, LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace, Fragment, FragmentKind};
use openui_style::{BorderStyle, ColumnFill, Direction, Display, Position, WritingMode};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn add_text(
    doc: &mut Document,
    parent: NodeId,
    text: &str,
    mode: WritingMode,
    direction: Direction,
) -> NodeId {
    let node = doc.create_node(ElementTag::Text);
    doc.update_resolved_style(node, |style| {
        style.writing_mode = mode;
        style.direction = direction;
        style.font_size = 10.0;
    });
    doc.node_mut(node).text = Some(text.to_string());
    doc.append_child(parent, node);
    node
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

fn layout(doc: &Document, mode: WritingMode, direction: Direction) -> Fragment {
    block_layout(
        doc,
        doc.root(),
        &ConstraintSpace::for_root_with_writing_direction(
            lu(800),
            lu(600),
            direction.writing_direction(mode),
        ),
    )
}

#[test]
fn direct_positioned_inline_geometry_covers_three_modes_and_both_directions() {
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let mut doc = Document::new();
            let root = doc.root();
            doc.update_resolved_style(root, |style| {
                style.update_derived(|computed| computed.display = Display::Block);
                style.update_derived(|computed| computed.writing_mode = mode);
                style.update_derived(|computed| computed.direction = direction);
                style.update_derived(|computed| computed.width = Length::px(240.0));
                style.update_derived(|computed| computed.height = Length::px(180.0));
                style.update_derived(|computed| computed.text_indent = Length::px(7.0));
                style.update_derived(|computed| computed.border_top_width = 2);
                style.update_derived(|computed| computed.border_right_width = 3);
                style.update_derived(|computed| computed.border_bottom_width = 5);
                style.update_derived(|computed| computed.border_left_width = 7);
                style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.padding_top = Length::px(11.0));
                style.update_derived(|computed| computed.padding_right = Length::px(13.0));
                style.update_derived(|computed| computed.padding_bottom = Length::px(17.0));
                style.update_derived(|computed| computed.padding_left = Length::px(19.0));
            });

            add_text(&mut doc, root, "AA ", mode, direction);
            let containing_inline = doc.create_node(ElementTag::Span);
            doc.update_resolved_style(containing_inline, |style| {
                style.update_derived(|computed| computed.display = Display::Inline);
                style.update_derived(|computed| computed.position = Position::Relative);
                style.update_derived(|computed| computed.writing_mode = mode);
                style.update_derived(|computed| computed.direction = direction);
                style.update_derived(|computed| computed.left = Length::px(3.0));
                style.update_derived(|computed| computed.top = Length::px(4.0));
                style.update_derived(|computed| computed.margin_left = Length::px(2.0));
                style.update_derived(|computed| computed.margin_right = Length::px(5.0));
                style.update_derived(|computed| computed.padding_left = Length::px(1.0));
                style.update_derived(|computed| computed.padding_right = Length::px(4.0));
                style.update_derived(|computed| computed.border_left_width = 1);
                style.update_derived(|computed| computed.border_right_width = 2);
                style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
                style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
            });
            doc.append_child(root, containing_inline);
            add_text(
                &mut doc,
                containing_inline,
                "BBBB BBBB BBBB ",
                mode,
                direction,
            );

            let inline_hypothetical = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(inline_hypothetical, |style| {
                style.update_derived(|computed| computed.display = Display::Inline);
                style.update_derived(|computed| computed.position = Position::Absolute);
                style.update_derived(|computed| computed.writing_mode = mode);
                style.update_derived(|computed| computed.direction = direction);
                style.update_derived(|computed| computed.width = Length::px(12.0));
                style.update_derived(|computed| computed.height = Length::px(18.0));
                style.update_derived(|computed| computed.margin_top = Length::px(2.0));
                style.update_derived(|computed| computed.margin_right = Length::px(3.0));
                style.update_derived(|computed| computed.margin_bottom = Length::px(5.0));
                style.update_derived(|computed| computed.margin_left = Length::px(7.0));
            });
            doc.append_child(containing_inline, inline_hypothetical);
            add_text(
                &mut doc,
                containing_inline,
                " CCCC CCCC CCCC ",
                mode,
                direction,
            );

            let block_hypothetical = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(block_hypothetical, |style| {
                style.update_derived(|computed| computed.display = Display::Block);
                style.update_derived(|computed| computed.position = Position::Absolute);
                style.update_derived(|computed| computed.writing_mode = mode);
                style.update_derived(|computed| computed.direction = direction);
                style.update_derived(|computed| computed.width = Length::px(20.0));
                style.update_derived(|computed| computed.height = Length::px(10.0));
            });
            doc.append_child(containing_inline, block_hypothetical);
            add_text(&mut doc, containing_inline, " DD", mode, direction);

            let fragment = layout(&doc, mode, direction);
            let inline_fragments = fragments_for(&fragment, inline_hypothetical);
            let block_fragments = fragments_for(&fragment, block_hypothetical);
            assert_eq!(inline_fragments.len(), 1, "inline {mode:?}/{direction:?}");
            assert_eq!(block_fragments.len(), 1, "block {mode:?}/{direction:?}");

            let inline_fragment = inline_fragments[0];
            let block_fragment = block_fragments[0];
            assert_eq!(inline_fragment.size.width, lu(12));
            assert_eq!(inline_fragment.size.height, lu(18));
            assert_eq!(block_fragment.size.width, lu(20));
            assert_eq!(block_fragment.size.height, lu(10));

            for positioned in [inline_fragment, block_fragment] {
                let geometry = positioned
                    .positioned_fragmentation
                    .expect("retained positioned-inline geometry");
                assert_eq!(
                    geometry.inline_containing_block_node,
                    Some(containing_inline),
                    "inline CB {mode:?}/{direction:?}"
                );
                assert!(geometry.has_inline_containing_block);
                assert!(geometry.containing_block_size.width >= LayoutUnit::zero());
                assert!(geometry.containing_block_size.height >= LayoutUnit::zero());
                assert_eq!(positioned.border, BoxStrut::zero());
            }
            assert_ne!(
                inline_fragment.positioned_fragmentation.unwrap().static_position,
                block_fragment.positioned_fragmentation.unwrap().static_position,
                "inline- and block-level hypothetical boxes must keep distinct anchors for {mode:?}/{direction:?}"
            );
        }
    }
}

#[test]
fn atomic_and_block_in_inline_descendants_keep_one_inline_containing_block_owner() {
    for mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let mut doc = Document::new();
            let root = doc.root();
            doc.update_resolved_style(root, |style| style.display = Display::Block);
            doc.update_resolved_style(root, |style| style.writing_mode = mode);
            doc.update_resolved_style(root, |style| style.direction = direction);
            doc.update_resolved_style(root, |style| style.width = Length::px(160.0));
            doc.update_resolved_style(root, |style| style.height = Length::px(120.0));

            let containing_inline = doc.create_node(ElementTag::Span);
            doc.update_resolved_style(containing_inline, |style| style.display = Display::Inline);
            doc.update_resolved_style(containing_inline, |style| {
                style.position = Position::Relative
            });
            doc.update_resolved_style(containing_inline, |style| style.writing_mode = mode);
            doc.update_resolved_style(containing_inline, |style| style.direction = direction);
            doc.append_child(root, containing_inline);
            add_text(&mut doc, containing_inline, "AA ", mode, direction);

            let atomic = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(atomic, |style| style.display = Display::InlineBlock);
            doc.update_resolved_style(atomic, |style| style.writing_mode = mode);
            doc.update_resolved_style(atomic, |style| style.direction = direction);
            doc.update_resolved_style(atomic, |style| style.width = Length::px(30.0));
            doc.update_resolved_style(atomic, |style| style.height = Length::px(20.0));
            doc.append_child(containing_inline, atomic);
            let atomic_abs = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(atomic_abs, |style| style.display = Display::Inline);
            doc.update_resolved_style(atomic_abs, |style| style.position = Position::Absolute);
            doc.update_resolved_style(atomic_abs, |style| style.writing_mode = mode);
            doc.update_resolved_style(atomic_abs, |style| style.direction = direction);
            doc.update_resolved_style(atomic_abs, |style| style.width = Length::px(7.0));
            doc.update_resolved_style(atomic_abs, |style| style.height = Length::px(9.0));
            doc.append_child(atomic, atomic_abs);

            let interruption = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(interruption, |style| style.display = Display::Block);
            doc.update_resolved_style(interruption, |style| style.writing_mode = mode);
            doc.update_resolved_style(interruption, |style| style.direction = direction);
            doc.update_resolved_style(interruption, |style| style.width = Length::px(40.0));
            doc.update_resolved_style(interruption, |style| style.height = Length::px(20.0));
            doc.append_child(containing_inline, interruption);
            let interrupted_abs = doc.create_node(ElementTag::Div);
            doc.update_resolved_style(interrupted_abs, |style| style.display = Display::Block);
            doc.update_resolved_style(interrupted_abs, |style| style.position = Position::Absolute);
            doc.update_resolved_style(interrupted_abs, |style| style.writing_mode = mode);
            doc.update_resolved_style(interrupted_abs, |style| style.direction = direction);
            doc.update_resolved_style(interrupted_abs, |style| style.width = Length::px(11.0));
            doc.update_resolved_style(interrupted_abs, |style| style.height = Length::px(13.0));
            doc.append_child(interruption, interrupted_abs);
            add_text(&mut doc, containing_inline, " BB", mode, direction);

            let fragment = layout(&doc, mode, direction);
            for positioned_node in [atomic_abs, interrupted_abs] {
                let positioned = fragments_for(&fragment, positioned_node);
                assert_eq!(
                    positioned.len(),
                    1,
                    "single ownership for {positioned_node:?} in {mode:?}/{direction:?}"
                );
                assert_eq!(
                    positioned[0]
                        .positioned_fragmentation
                        .expect("positioned geometry")
                        .inline_containing_block_node,
                    Some(containing_inline),
                    "inline CB for {positioned_node:?} in {mode:?}/{direction:?}",
                );
            }
        }
    }
}

#[test]
fn vertical_multicol_maps_inline_endpoints_once_for_outer_and_inner_direction() {
    for mode in [WritingMode::VerticalLr, WritingMode::VerticalRl] {
        for outer_direction in [Direction::Ltr, Direction::Rtl] {
            for inner_direction in [Direction::Ltr, Direction::Rtl] {
                for inline_containing_block in [false, true] {
                    let mut doc = Document::new();
                    let root = doc.root();
                    doc.update_resolved_style(root, |style| {
                        style.update_derived(|computed| computed.display = Display::Block);
                        style.update_derived(|computed| computed.writing_mode = mode);
                        style.update_derived(|computed| computed.direction = Direction::Ltr);
                        style.update_derived(|computed| computed.width = Length::px(60.0));
                        style.update_derived(|computed| computed.height = Length::px(160.0));
                        style.update_derived(|computed| computed.column_count = Some(2));
                        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
                        style
                            .update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
                    });

                    let container = doc.create_node(ElementTag::Div);
                    doc.update_resolved_style(container, |style| {
                        style.update_derived(|computed| computed.display = Display::Block);
                        style.update_derived(|computed| computed.position = Position::Relative);
                        style.update_derived(|computed| computed.writing_mode = mode);
                        style.update_derived(|computed| computed.direction = outer_direction);
                        style.update_derived(|computed| computed.width = Length::px(120.0));
                        style.update_derived(|computed| computed.height = Length::px(80.0));
                        style.update_derived(|computed| computed.font_size = 10.0);
                    });
                    doc.append_child(root, container);

                    let inline = doc.create_node(ElementTag::Span);
                    doc.update_resolved_style(inline, |style| style.display = Display::Inline);
                    doc.update_resolved_style(inline, |style| style.writing_mode = mode);
                    doc.update_resolved_style(inline, |style| style.direction = inner_direction);
                    if inline_containing_block {
                        doc.update_resolved_style(inline, |style| {
                            style.position = Position::Relative
                        });
                    }
                    doc.append_child(container, inline);
                    add_text(
                        &mut doc,
                        inline,
                        "AA AA AA AA AA AA AA AA ",
                        mode,
                        inner_direction,
                    );

                    let inset_start = doc.create_node(ElementTag::Div);
                    doc.update_resolved_style(inset_start, |style| {
                        style.update_derived(|computed| computed.display = Display::Inline);
                        style.update_derived(|computed| computed.position = Position::Absolute);
                        style.update_derived(|computed| computed.writing_mode = mode);
                        style.update_derived(|computed| computed.direction = inner_direction);
                        style.update_derived(|computed| computed.width = Length::px(20.0));
                        style.update_derived(|computed| computed.height = Length::px(20.0));
                        style.update_derived(|computed| computed.left = Length::px(0.0));
                        style.update_derived(|computed| computed.top = Length::px(0.0));
                    });
                    doc.append_child(inline, inset_start);
                    add_text(
                        &mut doc,
                        inline,
                        "BB BB BB BB BB BB BB BB ",
                        mode,
                        inner_direction,
                    );

                    let inset_end = doc.create_node(ElementTag::Div);
                    doc.update_resolved_style(inset_end, |style| {
                        style.update_derived(|computed| computed.display = Display::Block);
                        style.update_derived(|computed| computed.position = Position::Absolute);
                        style.update_derived(|computed| computed.writing_mode = mode);
                        style.update_derived(|computed| computed.direction = inner_direction);
                        style.update_derived(|computed| computed.width = Length::px(20.0));
                        style.update_derived(|computed| computed.height = Length::px(20.0));
                        style.update_derived(|computed| computed.right = Length::px(0.0));
                        style.update_derived(|computed| computed.bottom = Length::px(0.0));
                    });
                    doc.append_child(inline, inset_end);
                    add_text(&mut doc, inline, " CC", mode, inner_direction);

                    let fragment = layout(&doc, mode, Direction::Ltr);
                    for positioned_node in [inset_start, inset_end] {
                        let positioned = fragments_for(&fragment, positioned_node);
                        assert_eq!(
                            positioned.len(),
                            1,
                            "one multicol owner for {mode:?}/{outer_direction:?}/{inner_direction:?}/inline_cb={inline_containing_block}"
                        );
                        assert!(positioned[0].size.width > LayoutUnit::zero());
                        assert!(positioned[0].size.height > LayoutUnit::zero());
                        let geometry = positioned[0]
                            .positioned_fragmentation
                            .expect("fragmentation geometry");
                        assert_eq!(
                            geometry.inline_containing_block_node,
                            inline_containing_block.then_some(inline),
                        );
                    }
                    assert!(fragment.children.iter().all(|child| {
                        child.kind != FragmentKind::ColumnBox
                            || fragments_for(child, inset_start).is_empty()
                                && fragments_for(child, inset_end).is_empty()
                    }));
                }
            }
        }
    }
}
