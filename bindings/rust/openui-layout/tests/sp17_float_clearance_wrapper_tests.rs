//! SP17 W1M — clearance-only anonymous inline wrapper regressions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use openui_style::{Clear, Display, Float, LineHeight};

fn lu(value: i32) -> LayoutUnit {
    LayoutUnit::from_i32(value)
}

fn add_float(
    doc: &mut Document,
    parent: NodeId,
    side: Float,
    height: i32,
    margin_top: i32,
    margin_bottom: i32,
) -> NodeId {
    let child = doc.create_node(ElementTag::Div);
    {
        let style = &mut doc.node_mut(child).style;
        style.display = Display::Block;
        style.float = side;
        style.width = Length::px(20.0);
        style.height = Length::px(height as f32);
        style.margin_top = Length::px(margin_top as f32);
        style.margin_bottom = Length::px(margin_bottom as f32);
    }
    doc.append_child(parent, child);
    child
}

fn add_clearing_break(doc: &mut Document, parent: NodeId, clear: Clear) {
    let child = doc.create_node(ElementTag::Break);
    doc.node_mut(child).style.clear = clear;
    doc.append_child(parent, child);
}

fn add_collapsible_whitespace(doc: &mut Document, parent: NodeId) {
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("\n    ".to_string());
    doc.append_child(parent, text);
}

fn add_marker(doc: &mut Document, parent: NodeId) -> NodeId {
    let marker = doc.create_node(ElementTag::Div);
    {
        let style = &mut doc.node_mut(marker).style;
        style.display = Display::Block;
        style.width = Length::px(1.0);
        style.height = Length::px(1.0);
    }
    doc.append_child(parent, marker);
    marker
}

fn layout_container(doc: &Document, container: NodeId) -> Fragment {
    let mut root = block_layout(
        doc,
        doc.root(),
        &ConstraintSpace::for_root(lu(100), lu(100)),
    );
    let index = root
        .children
        .iter()
        .position(|fragment| fragment.node_id == container)
        .unwrap_or_else(|| panic!("container fragment; root={root:#?}"));
    root.children.remove(index)
}

#[test]
fn zero_height_clearing_breaks_separate_left_right_and_both_float_rows() {
    for (clear, first, second, expected_marker_top) in [
        (
            Clear::Left,
            vec![(Float::Left, 10)],
            vec![(Float::Left, 6)],
            16,
        ),
        (
            Clear::Right,
            vec![(Float::Right, 9)],
            vec![(Float::Right, 7)],
            16,
        ),
        (
            Clear::Both,
            vec![(Float::Left, 10), (Float::Right, 14)],
            vec![(Float::Left, 5), (Float::Right, 7)],
            21,
        ),
    ] {
        let mut doc = Document::new();
        let container = doc.create_node(ElementTag::Div);
        doc.node_mut(container).style.display = Display::Block;
        doc.node_mut(container).style.width = Length::px(100.0);
        doc.node_mut(container).style.line_height = LineHeight::Length(1.0);
        doc.append_child(doc.root(), container);

        for (side, height) in first {
            add_float(&mut doc, container, side, height, 0, 0);
        }
        add_clearing_break(&mut doc, container, clear);
        for (side, height) in second {
            add_float(&mut doc, container, side, height, 0, 0);
        }
        add_clearing_break(&mut doc, container, clear);
        let marker = add_marker(&mut doc, container);

        let fragment = layout_container(&doc, container);
        let marker_fragment = fragment
            .children
            .iter()
            .find(|child| child.node_id == marker)
            .expect("marker fragment");
        assert_eq!(marker_fragment.offset.top, lu(expected_marker_top));
        assert_eq!(fragment.size.height, lu(expected_marker_top + 1));
        assert_eq!(
            fragment
                .children
                .iter()
                .filter(|child| child.node_id.is_none() && child.size.height == lu(0))
                .count(),
            2,
            "clearing breaks must not gain a visible line-box strut",
        );
    }
}

#[test]
fn margin_box_clearance_advances_anonymous_wrapper_and_parent_once() {
    let mut doc = Document::new();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(100.0);
    doc.node_mut(container).style.line_height = LineHeight::Length(1.0);
    doc.append_child(doc.root(), container);

    add_float(&mut doc, container, Float::Left, 10, 2, 3);
    add_clearing_break(&mut doc, container, Clear::Left);
    let second = add_float(&mut doc, container, Float::Left, 6, 1, 2);
    add_clearing_break(&mut doc, container, Clear::Left);
    let marker = add_marker(&mut doc, container);

    let fragment = layout_container(&doc, container);
    let second_fragment = fragment
        .children
        .iter()
        .find(|child| child.node_id == second)
        .expect("second float fragment");
    let marker_fragment = fragment
        .children
        .iter()
        .find(|child| child.node_id == marker)
        .expect("marker fragment");
    assert_eq!(second_fragment.offset.top, lu(16));
    assert_eq!(marker_fragment.offset.top, lu(24));
    assert_eq!(fragment.size.height, lu(25));
}

#[test]
fn only_a_break_that_advances_clearance_suppresses_its_line_strut() {
    let mut doc = Document::new();
    let container = doc.create_node(ElementTag::Div);
    {
        let style = &mut doc.node_mut(container).style;
        style.display = Display::Block;
        style.width = Length::px(100.0);
        style.line_height = LineHeight::Length(10.0);
    }
    doc.append_child(doc.root(), container);

    add_float(&mut doc, container, Float::Left, 5, 0, 0);
    add_collapsible_whitespace(&mut doc, container);
    add_clearing_break(&mut doc, container, Clear::Both);
    add_collapsible_whitespace(&mut doc, container);
    add_clearing_break(&mut doc, container, Clear::Both);
    let marker = add_marker(&mut doc, container);

    let fragment = layout_container(&doc, container);
    let marker_fragment = fragment
        .children
        .iter()
        .find(|child| child.node_id == marker)
        .expect("marker fragment");
    assert_eq!(marker_fragment.offset.top, lu(20));
    assert_eq!(fragment.size.height, lu(21));
    assert_eq!(
        fragment
            .children
            .iter()
            .filter(|child| child.node_id.is_none() && child.size.height == lu(0))
            .count(),
        1,
        "the advancing clear contributes extent without a visible strut",
    );
}
