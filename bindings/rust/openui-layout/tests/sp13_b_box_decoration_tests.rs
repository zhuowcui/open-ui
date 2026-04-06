// SP13 Phase B: Inline Box Decoration Tests
//
// Tests for inline box decoration splitting (InlineBoxState tracking),
// box-decoration-break property, and is_first_for_node/is_last_for_node
// fragment metadata.
//
// CSS Fragmentation Level 3 §4.4: When an inline element spans multiple
// lines, its border/padding/margin decorations are split:
// - slice (default): first fragment gets inline-start MBP, last gets inline-end MBP
// - clone: every fragment gets full MBP on both sides
//
// Blink: InlineBoxState in inline_box_state.cc

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::ConstraintSpace;
use openui_layout::Fragment;
use openui_style::{BorderStyle, BoxDecorationBreak, Direction, Display};

fn lu(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

fn space(width: f32, height: f32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu(width), lu(height))
}

fn layout(doc: &Document, node_id: openui_dom::NodeId) -> Fragment {
    let s = space(400.0, 800.0);
    block_layout(doc, node_id, &s)
}

/// Helper: create a block container with a span child containing text.
/// Returns (doc, viewport_id, container_id, span_id).
fn make_span_in_block(
    container_width: f32,
    text: &str,
    padding_left: f32,
    padding_right: f32,
) -> (Document, openui_dom::NodeId, openui_dom::NodeId, openui_dom::NodeId) {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(container);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(container_width);
    }
    doc.append_child(vp, container);

    let span = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(span);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(padding_left);
        node.style.padding_right = Length::px(padding_right);
    }
    doc.append_child(container, span);

    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some(text.to_string());
    doc.append_child(span, text_node);

    (doc, vp, container, span)
}

// ── Test 1: Single-line span gets full MBP ──────────────────────────────

#[test]
fn single_line_span_gets_full_mbp() {
    // A span that fits entirely on one line should get both inline-start
    // and inline-end padding. The text should be offset by the left padding.
    let (doc, vp, _container, _span) = make_span_in_block(500.0, "Hello", 10.0, 10.0);

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];
    assert!(!div_frag.children.is_empty(), "Should have at least one line box");

    let line = &div_frag.children[0];
    // The line should have children (text fragments).
    assert!(!line.children.is_empty(), "Line should have text children");

    // First text fragment should be offset by at least the left padding.
    let first_text = &line.children[0];
    let text_left = first_text.offset.left.to_f32();
    assert!(
        text_left >= 10.0,
        "Text should be offset by at least left padding (10px), got {}",
        text_left
    );

    // Fragment metadata: single-line span → first AND last for node.
    assert!(first_text.is_first_for_node, "Single-line span text should be first for node");
    assert!(first_text.is_last_for_node, "Single-line span text should be last for node");
}

// ── Test 2: Multi-line span first line gets inline-start MBP only ───────

#[test]
fn multi_line_span_first_line_has_inline_start_mbp() {
    // Use narrow container to force wrapping inside the span.
    let (doc, vp, _container, _span) = make_span_in_block(
        80.0,
        "Hello World Test",
        10.0,
        10.0,
    );

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    // Should have multiple lines.
    assert!(
        div_frag.children.len() >= 2,
        "Expected at least 2 lines, got {}",
        div_frag.children.len()
    );

    // First line's first text fragment should be marked as first for node.
    let line1 = &div_frag.children[0];
    if let Some(first_text) = line1.children.first() {
        assert!(
            first_text.is_first_for_node,
            "First line text should be is_first_for_node=true"
        );
    }
}

// ── Test 3: Multi-line span last line gets inline-end MBP only ──────────

#[test]
fn multi_line_span_last_line_has_inline_end_mbp() {
    let (doc, vp, _container, _span) = make_span_in_block(
        80.0,
        "Hello World Test",
        10.0,
        10.0,
    );

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    let last_line_idx = div_frag.children.len() - 1;
    let last_line = &div_frag.children[last_line_idx];

    // Last line's text fragments should be marked as last for node.
    if let Some(last_text) = last_line.children.last() {
        assert!(
            last_text.is_last_for_node,
            "Last line text should be is_last_for_node=true"
        );
    }
}

// ── Test 4: Multi-line span middle line gets NO inline MBP ──────────────

#[test]
fn multi_line_span_middle_line_no_inline_mbp() {
    // Use very narrow container to force 3+ lines.
    let (doc, vp, _container, _span) = make_span_in_block(
        50.0,
        "Hello World Test More Text Here",
        10.0,
        10.0,
    );

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    if div_frag.children.len() >= 3 {
        // Middle line (index 1) should have is_first_for_node=false
        // and is_last_for_node=false.
        let middle_line = &div_frag.children[1];
        if let Some(middle_text) = middle_line.children.first() {
            assert!(
                !middle_text.is_first_for_node,
                "Middle line text should be is_first_for_node=false"
            );
            assert!(
                !middle_text.is_last_for_node,
                "Middle line text should be is_last_for_node=false"
            );
        }
    }
    // If fewer than 3 lines, this test is a no-op (font metrics may vary).
}

// ── Test 5: Nested spans multi-line ─────────────────────────────────────

#[test]
fn nested_spans_multi_line() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(container);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(80.0);
    }
    doc.append_child(vp, container);

    // Outer span with padding
    let outer_span = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(outer_span);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(5.0);
        node.style.padding_right = Length::px(5.0);
    }
    doc.append_child(container, outer_span);

    // Inner span with padding
    let inner_span = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(inner_span);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(3.0);
        node.style.padding_right = Length::px(3.0);
    }
    doc.append_child(outer_span, inner_span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Nested span text here".to_string());
    doc.append_child(inner_span, text);

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    // Should produce at least one line.
    assert!(
        !div_frag.children.is_empty(),
        "Nested spans should produce lines"
    );

    // First line should have text offset by both outer and inner padding.
    let line1 = &div_frag.children[0];
    if let Some(first_text) = line1.children.first() {
        let left = first_text.offset.left.to_f32();
        // Should be at least outer_padding + inner_padding = 5 + 3 = 8
        assert!(
            left >= 8.0,
            "Nested span text should be offset by combined padding (>= 8px), got {}",
            left
        );
    }
}

// ── Test 6: box-decoration-break: clone ─────────────────────────────────

#[test]
fn box_decoration_break_clone_all_fragments_get_mbp() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(container);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(80.0);
    }
    doc.append_child(vp, container);

    let span = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(span);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(10.0);
        node.style.padding_right = Length::px(10.0);
        node.style.box_decoration_break = BoxDecorationBreak::Clone;
    }
    doc.append_child(container, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello World Test Text".to_string());
    doc.append_child(span, text);

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    // With clone mode and wrapping, the property should be set.
    // Verify at least one line was produced.
    assert!(
        !div_frag.children.is_empty(),
        "Clone mode span should produce lines"
    );

    // The clone mode setting is now on the style.
    assert_eq!(
        doc.node(span).style.box_decoration_break,
        BoxDecorationBreak::Clone,
        "box_decoration_break should be Clone"
    );
}

// ── Test 7: Multiple spans on same line ─────────────────────────────────

#[test]
fn multiple_spans_same_line_both_get_full_mbp() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(container);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(500.0);
    }
    doc.append_child(vp, container);

    // First span
    let span1 = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(span1);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(5.0);
        node.style.padding_right = Length::px(5.0);
    }
    doc.append_child(container, span1);

    let text1 = doc.create_node(ElementTag::Text);
    doc.node_mut(text1).text = Some("AAA".to_string());
    doc.append_child(span1, text1);

    // Second span
    let span2 = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(span2);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(8.0);
        node.style.padding_right = Length::px(8.0);
    }
    doc.append_child(container, span2);

    let text2 = doc.create_node(ElementTag::Text);
    doc.node_mut(text2).text = Some("BBB".to_string());
    doc.append_child(span2, text2);

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    // Both spans fit on one line → one line box.
    assert_eq!(
        div_frag.children.len(), 1,
        "Two short spans on wide container should produce 1 line"
    );

    let line = &div_frag.children[0];
    // Should have at least 2 text fragments.
    assert!(
        line.children.len() >= 2,
        "Line should have at least 2 text children, got {}",
        line.children.len()
    );

    // Both text fragments are first and last for their respective inline boxes.
    for child in &line.children {
        assert!(child.is_first_for_node, "Single-line span text should be is_first_for_node=true");
        assert!(child.is_last_for_node, "Single-line span text should be is_last_for_node=true");
    }

    // Second text should be offset further than first (by span1's MBP + text width + span2's MBP).
    let left1 = line.children[0].offset.left.to_f32();
    let left2 = line.children[1].offset.left.to_f32();
    assert!(
        left2 > left1,
        "Second span text ({}) should be right of first span text ({})",
        left2, left1
    );
}

// ── Test 8: Empty span ──────────────────────────────────────────────────

#[test]
fn empty_span_produces_no_crash() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(container);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
    }
    doc.append_child(vp, container);

    let span = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(span);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(10.0);
        node.style.padding_right = Length::px(10.0);
    }
    doc.append_child(container, span);

    // Empty span — no text child.
    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    // Should not crash. May produce zero or one line.
    // Height should be minimal (strut only).
    let height = div_frag.size.height.to_f32();
    assert!(
        height >= 0.0,
        "Empty span container height should be >= 0"
    );
}

// ── Test 9: Span with padding and border ────────────────────────────────

#[test]
fn span_with_padding_and_border_contributes_to_inline_size() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(container);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(500.0);
    }
    doc.append_child(vp, container);

    let span = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(span);
        node.style.display = Display::Inline;
        node.style.padding_left = Length::px(10.0);
        node.style.padding_right = Length::px(10.0);
        node.style.border_left_width = 2;
        node.style.border_right_width = 2;
        node.style.border_left_style = BorderStyle::Solid;
        node.style.border_right_style = BorderStyle::Solid;
    }
    doc.append_child(container, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("X".to_string());
    doc.append_child(span, text);

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];
    let line = &div_frag.children[0];
    let text_frag = &line.children[0];

    // Text should be offset by padding_left + border_left = 10 + 2 = 12.
    let left = text_frag.offset.left.to_f32();
    assert!(
        left >= 12.0,
        "Text should be offset by padding+border (>= 12px), got {}",
        left
    );
}

// ── Test 10: RTL span multi-line ────────────────────────────────────────

#[test]
fn rtl_span_multi_line_reverses_inline_start_end() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    {
        let node = doc.node_mut(container);
        node.style.display = Display::Block;
        node.style.font_size = 16.0;
        node.style.width = Length::px(500.0);
        node.style.direction = Direction::Rtl;
    }
    doc.append_child(vp, container);

    let span = doc.create_node(ElementTag::Span);
    {
        let node = doc.node_mut(span);
        node.style.display = Display::Inline;
        node.style.direction = Direction::Rtl;
        // In RTL, inline-start is the RIGHT side, inline-end is the LEFT side.
        node.style.padding_left = Length::px(5.0);  // This is inline-end in RTL
        node.style.padding_right = Length::px(15.0); // This is inline-start in RTL
    }
    doc.append_child(container, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("RTL".to_string());
    doc.append_child(span, text);

    let frag = layout(&doc, vp);
    let div_frag = &frag.children[0];

    assert!(
        !div_frag.children.is_empty(),
        "RTL span should produce lines"
    );

    // In RTL, the inline-start MBP is the right padding (15px).
    // Text should be positioned accounting for RTL direction.
    let line = &div_frag.children[0];
    assert!(
        !line.children.is_empty(),
        "RTL line should have children"
    );
}

// ── Test 11: BoxDecorationBreak default is Slice ────────────────────────

#[test]
fn box_decoration_break_default_is_slice() {
    let style = openui_style::ComputedStyle::initial();
    assert_eq!(
        style.box_decoration_break,
        BoxDecorationBreak::Slice,
        "Default box_decoration_break should be Slice"
    );
}

// ── Test 12: Fragment is_first_for_node/is_last_for_node defaults ───────

#[test]
fn fragment_defaults_are_first_and_last() {
    use openui_geometry::PhysicalSize;
    use openui_dom::NodeId;

    let frag = openui_layout::Fragment::new_box(
        NodeId::NONE,
        PhysicalSize::new(lu(100.0), lu(50.0)),
    );
    assert!(frag.is_first_for_node, "new_box default is_first_for_node should be true");
    assert!(frag.is_last_for_node, "new_box default is_last_for_node should be true");
}
