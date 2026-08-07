//! SP14 regressions for forced breaks emitted by deterministic text ports.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::ConstraintSpace;
use openui_style::{Display, WhiteSpace};

#[test]
fn lone_forced_break_establishes_a_nonzero_line_strut() {
    let mut doc = Document::new();
    let vp = doc.root();

    let break_container = doc.create_node(ElementTag::Div);
    doc.node_mut(break_container).style.display = Display::Block;
    doc.append_child(vp, break_container);

    let forced_break = doc.create_node(ElementTag::Text);
    doc.node_mut(forced_break).style.white_space = WhiteSpace::PreLine;
    doc.node_mut(forced_break).text = Some("\n".to_string());
    doc.append_child(break_container, forced_break);

    let following = doc.create_node(ElementTag::Div);
    doc.node_mut(following).style.display = Display::Block;
    doc.node_mut(following).style.height = Length::px(10.0);
    doc.append_child(vp, following);

    let space = ConstraintSpace::for_root(LayoutUnit::from_f32(200.0), LayoutUnit::from_f32(200.0));
    let fragment = block_layout(&doc, vp, &space);
    let break_fragment = &fragment.children[0];
    let following_fragment = &fragment.children[1];

    assert!(
        break_fragment.size.height > LayoutUnit::zero(),
        "a forced break must establish a line strut"
    );
    assert_eq!(following_fragment.offset.top, break_fragment.size.height);
}
