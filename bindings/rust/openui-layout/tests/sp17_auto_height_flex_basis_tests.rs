//! SP17 W1O — auto-height percentage flex bases and semantic break struts.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, flex_layout, ConstraintSpace, Fragment};
use openui_style::{BorderStyle, Direction, Display, FlexDirection, LineHeight, WritingMode};

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

fn flex_basis_case(
    writing_mode: WritingMode,
    direction: FlexDirection,
    definite_main_size: bool,
    basis: Length,
) -> (LayoutUnit, LayoutUnit) {
    let mut doc = Document::new();
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = direction);
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        if writing_mode == WritingMode::HorizontalTb {
            style.update_derived(|computed| computed.width = Length::px(100.0));
            style.update_derived(|computed| {
                computed.height = if definite_main_size {
                    Length::px(100.0)
                } else {
                    Length::auto()
                }
            });
        } else {
            style.update_derived(|computed| {
                computed.width = if definite_main_size {
                    Length::px(100.0)
                } else {
                    Length::auto()
                }
            });
            style.update_derived(|computed| computed.height = Length::px(100.0));
        }
    });
    doc.append_child(doc.root(), container);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.flex_grow = 0.0);
        style.update_derived(|computed| computed.flex_shrink = 0.0);
        style.update_derived(|computed| computed.flex_basis = basis);
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        if writing_mode == WritingMode::HorizontalTb {
            style.update_derived(|computed| computed.height = Length::px(200.0));
            style.update_derived(|computed| computed.min_height = Length::zero());
        } else {
            style.update_derived(|computed| computed.width = Length::px(200.0));
            style.update_derived(|computed| computed.min_width = Length::zero());
        }
    });
    doc.append_child(container, item);

    let content = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(content, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.writing_mode = writing_mode);
        if writing_mode == WritingMode::HorizontalTb {
            style.update_derived(|computed| computed.width = Length::px(10.0));
            style.update_derived(|computed| computed.height = Length::px(40.0));
        } else {
            style.update_derived(|computed| computed.width = Length::px(40.0));
            style.update_derived(|computed| computed.height = Length::px(10.0));
        }
    });
    doc.append_child(item, content);

    let writing_direction = Direction::Ltr.writing_direction(writing_mode);
    let space =
        ConstraintSpace::for_root_with_writing_direction(lu(300), lu(300), writing_direction);
    let fragment = flex_layout(&doc, container, &space);
    let item_fragment = fragment_for(&fragment, item);
    if writing_mode == WritingMode::HorizontalTb {
        (fragment.height(), item_fragment.height())
    } else {
        (fragment.width(), item_fragment.width())
    }
}

#[test]
fn zero_percent_falls_back_to_content_only_when_main_size_is_indefinite() {
    for writing_mode in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalLr,
        WritingMode::VerticalRl,
    ] {
        for direction in [FlexDirection::Column, FlexDirection::ColumnReverse] {
            assert_eq!(
                flex_basis_case(writing_mode, direction, false, Length::percent(0.0),),
                (lu(40), lu(40)),
                "indefinite 0% must use content for {writing_mode:?}/{direction:?}",
            );
            assert_eq!(
                flex_basis_case(writing_mode, direction, false, Length::px(0.0)),
                (lu(0), lu(0)),
                "fixed zero must stay definite for {writing_mode:?}/{direction:?}",
            );
            assert_eq!(
                flex_basis_case(writing_mode, direction, true, Length::percent(0.0),),
                (lu(100), lu(0)),
                "definite 0% must resolve to zero for {writing_mode:?}/{direction:?}",
            );
            assert_eq!(
                flex_basis_case(writing_mode, direction, true, Length::px(0.0)),
                (lu(100), lu(0)),
                "fixed zero changed under a definite main size for {writing_mode:?}/{direction:?}",
            );
        }
    }
}

fn inline_break_height(sequence: &[ElementTag]) -> LayoutUnit {
    let mut doc = Document::new();
    let block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(block, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(200.0));
        style.update_derived(|computed| computed.font_size = 10.0);
        style.update_derived(|computed| computed.line_height = LineHeight::Length(10.0));
    });
    doc.append_child(doc.root(), block);

    for tag in sequence {
        let child = doc.create_node(*tag);
        {
            let node = doc.node_mut(child);
            node.style
                .update_derived(|computed| computed.display = Display::Inline);
            node.style.update_derived(|computed| {
                computed.font_size = if *tag == ElementTag::Break {
                    40.0
                } else {
                    10.0
                }
            });
            node.style.update_derived(|computed| {
                computed.line_height = if *tag == ElementTag::Break {
                    LineHeight::Length(40.0)
                } else {
                    LineHeight::Length(10.0)
                }
            });
            if *tag == ElementTag::Text {
                node.text = Some("X".to_string());
            }
        }
        doc.append_child(block, child);
    }

    block_layout(
        &doc,
        block,
        &ConstraintSpace::for_block_child(lu(200), lu(300), lu(200), lu(300), false),
    )
    .height()
}

#[test]
fn semantic_break_sequences_use_the_break_strut_without_a_phantom_line() {
    assert_eq!(
        inline_break_height(&[ElementTag::Text, ElementTag::Break]),
        lu(40),
        "text followed by <br> must occupy one break-terminated line",
    );
    assert_eq!(inline_break_height(&[ElementTag::Break]), lu(40));
    assert_eq!(
        inline_break_height(&[ElementTag::Break, ElementTag::Break]),
        lu(80),
        "each repeated break establishes exactly one line",
    );
    assert_eq!(
        inline_break_height(&[ElementTag::Break, ElementTag::Text]),
        lu(50),
        "content after a break must start one additional line",
    );
}

#[test]
fn auto_height_flex_item_includes_border_padding_and_one_break_line() {
    let mut doc = Document::new();
    let container = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(container, |style| {
        style.update_derived(|computed| computed.display = Display::Flex);
        style.update_derived(|computed| computed.flex_direction = FlexDirection::Column);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::auto());
        style.update_derived(|computed| computed.border_top_width = 1);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_width = 1);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.padding_top = Length::px(5.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(5.0));
    });
    doc.append_child(doc.root(), container);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.flex_grow = 1.0);
        style.update_derived(|computed| computed.flex_shrink = 1.0);
        style.update_derived(|computed| computed.flex_basis = Length::percent(0.0));
        style.update_derived(|computed| computed.border_top_width = 2);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_width = 2);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.padding_top = Length::px(3.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(3.0));
        style.update_derived(|computed| computed.font_size = 20.0);
        style.update_derived(|computed| computed.line_height = LineHeight::Length(20.0));
    });
    doc.append_child(container, item);

    let text = doc.create_node(ElementTag::Text);
    doc.update_resolved_style(text, |style| style.font_size = 20.0);
    doc.update_resolved_style(text, |style| style.line_height = LineHeight::Length(20.0));
    // The words wrap at min-content width but fit the item's actual cross
    // size. A semantic trailing break must not make the automatic main-axis
    // minimum reserve that synthetic wrapped measurement.
    doc.node_mut(text).text = Some("XX XX".to_string());
    doc.append_child(item, text);
    let line_break = doc.create_node(ElementTag::Break);
    doc.update_resolved_style(line_break, |style| style.display = Display::Inline);
    doc.update_resolved_style(line_break, |style| style.font_size = 20.0);
    doc.update_resolved_style(line_break, |style| {
        style.line_height = LineHeight::Length(20.0)
    });
    doc.append_child(item, line_break);

    let fragment = flex_layout(
        &doc,
        container,
        &ConstraintSpace::for_root(lu(300), lu(300)),
    );
    let item_fragment = fragment_for(&fragment, item);
    assert_eq!(item_fragment.height(), lu(30));
    assert_eq!(fragment.height(), lu(42));
}
