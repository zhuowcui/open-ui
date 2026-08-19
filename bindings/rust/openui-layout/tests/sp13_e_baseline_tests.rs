// SP13 Phase E: Baseline propagation tests
//
// CSS Inline 3 §3: First/last baseline sets for block containers with
// inline content. The first baseline is the baseline of the first line box;
// the last baseline is the baseline of the last line box. Block containers
// propagate baselines from child fragments to parent containers.
//
// Blink: PhysicalBoxFragment::FirstBaseline(), PhysicalBoxFragment::LastBaseline()

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::ConstraintSpace;
use openui_layout::Fragment;
use openui_style::Display;

fn lu(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

fn space_with_baselines(width: f32, height: f32) -> ConstraintSpace {
    let mut space = ConstraintSpace::for_root(lu(width), lu(height));
    space.needs_first_baseline = true;
    space.needs_last_baseline = true;
    space
}

fn layout_with_baselines(doc: &Document, node_id: openui_dom::NodeId) -> Fragment {
    let space = space_with_baselines(400.0, 800.0);
    block_layout(doc, node_id, &space)
}

#[test]
fn single_line_first_and_last_baseline_are_equal() {
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
    }
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello baseline".to_string());
    doc.append_child(div, text);

    let frag = layout_with_baselines(&doc, vp);
    let div_frag = &frag.children[0];

    assert!(
        div_frag.first_baseline.is_some(),
        "first_baseline should be set"
    );
    assert!(
        div_frag.last_baseline.is_some(),
        "last_baseline should be set"
    );
    assert_eq!(
        div_frag.first_baseline, div_frag.last_baseline,
        "Single line: first and last baseline should be equal"
    );

    let bl = div_frag.first_baseline.unwrap();
    assert!(
        bl.to_f32() > 0.0,
        "Baseline should be positive, got {}",
        bl.to_f32()
    );
}

#[test]
fn multi_line_first_baseline_before_last() {
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(60.0);
    }
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("First line Second line Third line".to_string());
    doc.append_child(div, text);

    let space = space_with_baselines(60.0, 800.0);
    let frag = block_layout(&doc, vp, &space);
    let div_frag = &frag.children[0];

    let first = div_frag.first_baseline.unwrap();
    let last = div_frag.last_baseline.unwrap();

    assert!(
        first.to_f32() < last.to_f32(),
        "First baseline ({}) should be before (less than) last baseline ({})",
        first.to_f32(),
        last.to_f32()
    );
}

#[test]
fn baseline_includes_container_offset() {
    let mut doc = Document::new();
    let vp = doc.root();

    let outer = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(outer);
        node.style.display = Display::Block;
        node.style.padding_top = Length::px(20.0);
        node.style.font_size = 16.0;
    }
    doc.append_child(vp, outer);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Padded".to_string());
    doc.append_child(outer, text);

    let frag = layout_with_baselines(&doc, vp);
    let outer_frag = &frag.children[0];

    let bl = outer_frag.first_baseline.unwrap();
    assert!(
        bl.to_f32() >= 20.0,
        "Baseline should account for padding-top. Got {}",
        bl.to_f32()
    );
}

#[test]
fn baselines_always_computed() {
    // Baselines are always computed by block_layout, regardless of the
    // needs_first_baseline flag. This ensures parent containers can always
    // access baselines without re-layout.
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
    }
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Baselines always present".to_string());
    doc.append_child(div, text);

    // Default space doesn't explicitly request baselines
    let space = ConstraintSpace::for_root(lu(400.0), lu(800.0));
    let frag = block_layout(&doc, vp, &space);
    let div_frag = &frag.children[0];

    // Baselines should still be computed
    assert!(
        div_frag.first_baseline.is_some(),
        "first_baseline should always be computed for blocks with inline content"
    );
}

#[test]
fn nested_block_propagates_child_baseline() {
    let mut doc = Document::new();
    let vp = doc.root();

    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.append_child(vp, outer);

    let inner = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(inner);
        node.style.display = Display::Block;
        node.style.font_size = 20.0;
    }
    doc.append_child(outer, inner);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Nested".to_string());
    doc.append_child(inner, text);

    let frag = layout_with_baselines(&doc, vp);
    let outer_frag = &frag.children[0];

    assert!(
        outer_frag.first_baseline.is_some(),
        "Outer block should have first_baseline from nested child"
    );
}

#[test]
fn baseline_from_first_child_not_second() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.append_child(vp, container);

    let child1 = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(child1);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
    }
    doc.append_child(container, child1);
    let text1 = doc.create_node(ElementTag::Text);
    doc.node_mut(text1).text = Some("First".to_string());
    doc.append_child(child1, text1);

    let child2 = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(child2);
        node.style.display = Display::Block;
        node.style.font_size = 32.0;
    }
    doc.append_child(container, child2);
    let text2 = doc.create_node(ElementTag::Text);
    doc.node_mut(text2).text = Some("Second".to_string());
    doc.append_child(child2, text2);

    let frag = layout_with_baselines(&doc, vp);
    let container_frag = &frag.children[0];

    let first_bl = container_frag.first_baseline.unwrap();
    let last_bl = container_frag.last_baseline.unwrap();

    assert!(
        first_bl.to_f32() < last_bl.to_f32(),
        "first_baseline ({}) should be before last_baseline ({})",
        first_bl.to_f32(),
        last_bl.to_f32()
    );
}

#[test]
fn inline_layout_sets_baselines_directly() {
    use openui_layout::inline::algorithm::inline_layout;

    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
    }
    doc.append_child(vp, div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Direct inline layout".to_string());
    doc.append_child(div, text);

    let space = ConstraintSpace::for_root(lu(400.0), lu(800.0));
    let inline_frag = inline_layout(&doc, div, &space);

    assert!(
        inline_frag.first_baseline.is_some(),
        "inline_layout should always set first_baseline"
    );
    assert!(
        inline_frag.last_baseline.is_some(),
        "inline_layout should always set last_baseline"
    );
}

#[test]
fn empty_block_has_no_baselines() {
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(div);
        node.style.display = Display::Block;
        node.style.height = Length::px(50.0);
    }
    doc.append_child(vp, div);

    let frag = layout_with_baselines(&doc, vp);
    let div_frag = &frag.children[0];

    assert!(
        div_frag.first_baseline.is_none(),
        "Empty block should have no first_baseline"
    );
    assert!(
        div_frag.last_baseline.is_none(),
        "Empty block should have no last_baseline"
    );
}
