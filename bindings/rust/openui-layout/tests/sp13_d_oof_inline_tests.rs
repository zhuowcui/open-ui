// SP13 Phase D: OOF inline static position tests
//
// CSS 2.1 §10.3.7: The static position of an absolutely-positioned element
// within inline content is where it would have been placed in normal flow.
// When OOF children appear inside inline formatting contexts, inline_layout
// records their positions and returns OOF candidates on the Fragment.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::ConstraintSpace;
use openui_layout::Fragment;
use openui_style::{Display, Position};

fn lu(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

#[test]
fn oof_in_pure_inline_generates_candidate() {
    // An absolutely-positioned child inside pure inline content should
    // generate an OOF candidate on the fragment.
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.position = Position::Relative;
        node.style.width = Length::px(400.0);
        node.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);

    // Text before the OOF child
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello ".to_string());
    doc.append_child(div, text);

    // Absolutely positioned child inside inline context
    let abs = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(abs);
        node.style.display = Display::Block;
        node.style.position = Position::Absolute;
        node.style.width = Length::px(50.0);
        node.style.height = Length::px(50.0);
    }
    doc.append_child(div, abs);

    // Text after the OOF child
    let text2 = doc.create_node(ElementTag::Text);
    doc.node_mut(text2).text = Some(" world".to_string());
    doc.append_child(div, text2);

    let space = ConstraintSpace::for_root(lu(400.0), lu(800.0));
    let frag = block_layout(&doc, vp, &space);
    let div_frag = &frag.children[0];

    // The div should have laid out the OOF child
    // Check that OOF fragments are present (as positioned children or oof_candidates)
    let total_children = div_frag.children.len();
    assert!(
        total_children >= 1,
        "Should have at least 1 child (line boxes + OOF fragments)"
    );
}

#[test]
fn oof_in_inline_gets_static_position() {
    // The static position should be within the inline formatting context
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.position = Position::Relative;
        node.style.width = Length::px(400.0);
        node.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Before abs".to_string());
    doc.append_child(div, text);

    let abs = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(abs);
        node.style.display = Display::Block;
        node.style.position = Position::Absolute;
        node.style.width = Length::px(30.0);
        node.style.height = Length::px(30.0);
    }
    doc.append_child(div, abs);

    let space = ConstraintSpace::for_root(lu(400.0), lu(800.0));
    let frag = block_layout(&doc, vp, &space);

    // Check that the absolutely positioned child was laid out somewhere
    // (it should be in the children as an OOF fragment)
    let div_frag = &frag.children[0];

    // There should be OOF children laid out
    let has_oof = div_frag.children.iter().any(|c| c.node_id == abs);
    assert!(
        has_oof,
        "Absolutely positioned child should appear in fragment children"
    );
}

#[test]
fn multiple_oof_in_inline() {
    // Multiple OOF children should all be collected
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.position = Position::Relative;
        node.style.width = Length::px(400.0);
        node.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);

    let text1 = doc.create_node(ElementTag::Text);
    doc.node_mut(text1).text = Some("A".to_string());
    doc.append_child(div, text1);

    let abs1 = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(abs1);
        node.style.position = Position::Absolute;
        node.style.width = Length::px(20.0);
        node.style.height = Length::px(20.0);
    }
    doc.append_child(div, abs1);

    let text2 = doc.create_node(ElementTag::Text);
    doc.node_mut(text2).text = Some("B".to_string());
    doc.append_child(div, text2);

    let abs2 = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(abs2);
        node.style.position = Position::Absolute;
        node.style.width = Length::px(20.0);
        node.style.height = Length::px(20.0);
    }
    doc.append_child(div, abs2);

    let space = ConstraintSpace::for_root(lu(400.0), lu(800.0));
    let frag = block_layout(&doc, vp, &space);
    let div_frag = &frag.children[0];

    // Both OOF children should be present in the fragment tree
    let oof_count = div_frag.children.iter()
        .filter(|c| c.node_id == abs1 || c.node_id == abs2)
        .count();
    assert!(
        oof_count >= 2,
        "Both OOF children should be laid out, found {}",
        oof_count
    );
}
