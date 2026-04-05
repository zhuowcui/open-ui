//! Tests for SP11 Round 12 fixes.
//!
//! Issue 1: pre-wrap force-on-line should consume newline (no extra blank line).
//! Issue 2: text-indent + text-align interaction should not overflow on first line.
//! Issue 3: Inline element margin/border/padding should be accounted for during
//!          line breaking.
//! Issue 6: Cross-node whitespace should be collapsed between adjacent text items.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::inline::items_builder::InlineItemsBuilder;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{Display, TextAlign, WhiteSpace};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn make_constraint_width(w: i32) -> ConstraintSpace {
    ConstraintSpace::for_block_child(lu_i(w), lu_i(600), lu_i(w), lu_i(600), false)
}

fn collect_line_boxes(fragment: &Fragment) -> Vec<&Fragment> {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .collect()
}

// ── Issue 1: pre-wrap force-on-line consumes newline ─────────────────────

#[test]
fn prewrap_forced_overlong_before_newline_no_extra_blank_line() {
    // A single text node "LONGWORD\nafter" in pre-wrap mode with a narrow
    // container.  "LONGWORD" is wider than the available width so it gets
    // force-placed.  The newline should be consumed on that same line so the
    // next line starts with "after" — NOT with an empty forced-break line.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("LONGWORD\nafter".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).style.white_space = WhiteSpace::PreWrap;
    doc.append_child(block, text);

    // Use a very narrow width (20px) so "LONGWORD" can't fit and must be forced
    let constraint = make_constraint_width(20);
    let frag = inline_layout(&doc, block, &constraint);

    let lines = collect_line_boxes(&frag);
    // Should be exactly 2 lines: "LONGWORD" (forced + newline consumed) and "after"
    // NOT 3 lines where the middle one is a blank forced-break line
    assert_eq!(
        lines.len(),
        2,
        "Expected 2 lines (forced word + after), got {}. \
         The newline should be consumed when force-placing the overlong word.",
        lines.len()
    );
}

// ── Issue 2: text-indent + text-align: right doesn't overflow ────────────

#[test]
fn text_indent_with_text_align_right_no_overflow() {
    // Container 100px wide.  text-indent: 20px, text-align: right.
    // A short word that occupies ~30px of text.
    // The text should end at x=100 (right edge), meaning offset starts at
    // (100 - 20 - text_width) + 20 = 100 - text_width.
    // It must NOT start at 70 + 20 = 90 which would overflow.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Right;
    doc.node_mut(block).style.text_indent = Length::px(20.0);
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hi".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.append_child(block, text);

    let constraint = make_constraint_width(100);
    let frag = inline_layout(&doc, block, &constraint);

    let lines = collect_line_boxes(&frag);
    assert!(!lines.is_empty(), "Should produce at least one line");

    let line_box = lines[0];
    // Find the text fragment
    for child in &line_box.children {
        if child.kind == FragmentKind::Text {
            let right_edge = child.offset.left + child.size.width;
            // The text right edge should be at or near 100px (container width)
            // and the left edge should be >= 20px (text-indent minimum)
            assert!(
                right_edge <= lu(100.0) + lu(1.0),
                "Text right edge {:?} should not exceed container width 100px \
                 (text-indent + text-align: right overflow bug)",
                right_edge,
            );
        }
    }
}

#[test]
fn text_indent_with_text_align_center_content_centered_in_remaining() {
    // text-indent: 40px, text-align: center, container 200px.
    // Effective available for alignment = 200 - 40 = 160px.
    // A short text (~30px) should be centered in 160px, then shifted by +40.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.text_align = TextAlign::Center;
    doc.node_mut(block).style.text_indent = Length::px(40.0);
    doc.append_child(root, block);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hi".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.append_child(block, text);

    let constraint = make_constraint_width(200);
    let frag = inline_layout(&doc, block, &constraint);

    let lines = collect_line_boxes(&frag);
    assert!(!lines.is_empty());
    let line_box = lines[0];
    for child in &line_box.children {
        if child.kind == FragmentKind::Text {
            // With indent 40 and center alignment in 160px remaining,
            // the text start should be >= 40px (indent is always applied)
            assert!(
                child.offset.left >= lu(40.0),
                "Text offset {:?} should be >= 40px due to text-indent",
                child.offset.left,
            );
            let right_edge = child.offset.left + child.size.width;
            assert!(
                right_edge <= lu(200.0) + lu(1.0),
                "Text right edge {:?} should not exceed 200px container",
                right_edge,
            );
        }
    }
}

// ── Issue 3: Inline span MBP triggers line wrap ─────────────────────────

#[test]
fn inline_span_padding_causes_earlier_line_break() {
    // Container 100px.  A span with 40px padding-left wraps a short word.
    // The padding eats 40px, so only 60px is left for text.
    // "Hello World" should break because the padding reduces available space.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.padding_left = Length::px(40.0);
    doc.append_child(block, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello World".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.append_child(span, text);

    let constraint = make_constraint_width(100);
    let frag = inline_layout(&doc, block, &constraint);

    let lines = collect_line_boxes(&frag);
    // With 40px padding, "Hello World" (~80-90px) should need wrapping
    // that would not occur without the padding being counted.
    // This asserts that we get more than one line (padding caused the break).
    assert!(
        lines.len() >= 2,
        "Expected at least 2 lines because 40px inline padding should reduce \
         available width for breaking. Got {} line(s).",
        lines.len()
    );
}

#[test]
fn inline_span_border_contributes_to_used_width() {
    // Container 60px.  Span with 20px border-left + 20px border-right = 40px.
    // Even a tiny word should see reduced available space.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.border_left_width = 20;
    doc.node_mut(span).style.border_left_style = openui_style::BorderStyle::Solid;
    doc.node_mut(span).style.border_right_width = 20;
    doc.node_mut(span).style.border_right_style = openui_style::BorderStyle::Solid;
    doc.append_child(block, span);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("Hello World".to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.append_child(span, text);

    let constraint = make_constraint_width(60);
    let frag = inline_layout(&doc, block, &constraint);

    let lines = collect_line_boxes(&frag);
    // 40px of border + text width > 60px → should wrap
    assert!(
        lines.len() >= 2,
        "Expected wrapping due to 40px inline border. Got {} line(s).",
        lines.len()
    );
}

// ── Issue 6: Cross-node whitespace collapsing ────────────────────────────

#[test]
fn cross_node_trailing_leading_spaces_collapse_to_single() {
    // <span>foo </span><span> bar</span> → collected text should be "foo bar"
    // not "foo  bar" (double space).
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span1 = doc.create_node(ElementTag::Span);
    doc.node_mut(span1).style.display = Display::Inline;
    doc.append_child(block, span1);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("foo ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(span1, t1);

    let span2 = doc.create_node(ElementTag::Span);
    doc.node_mut(span2).style.display = Display::Inline;
    doc.append_child(block, span2);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some(" bar".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.append_child(span2, t2);

    let data = InlineItemsBuilder::collect(&doc, block);

    // The collected text must not have a double space
    assert!(
        !data.text.contains("  "),
        "Cross-node spaces should collapse to single space. Got: {:?}",
        data.text,
    );
    assert!(
        data.text.contains("foo bar"),
        "Expected 'foo bar' (single space), got: {:?}",
        data.text,
    );
}

#[test]
fn cross_node_space_collapse_preserves_non_space_boundary() {
    // <span>foo</span><span> bar</span> → "foo bar" (single space, not collapsed)
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span1 = doc.create_node(ElementTag::Span);
    doc.node_mut(span1).style.display = Display::Inline;
    doc.append_child(block, span1);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("foo".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(span1, t1);

    let span2 = doc.create_node(ElementTag::Span);
    doc.node_mut(span2).style.display = Display::Inline;
    doc.append_child(block, span2);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some(" bar".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.append_child(span2, t2);

    let data = InlineItemsBuilder::collect(&doc, block);

    assert!(
        data.text.contains("foo bar"),
        "Non-collapsible boundary should keep single space: {:?}",
        data.text,
    );
}

#[test]
fn cross_node_prewrap_does_not_collapse_spaces() {
    // In pre-wrap mode, spaces are preserved — no cross-node collapsing.
    let mut doc = Document::new();
    let root = doc.root();

    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span1 = doc.create_node(ElementTag::Span);
    doc.node_mut(span1).style.display = Display::Inline;
    doc.append_child(block, span1);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("foo ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.node_mut(t1).style.white_space = WhiteSpace::PreWrap;
    doc.append_child(span1, t1);

    let span2 = doc.create_node(ElementTag::Span);
    doc.node_mut(span2).style.display = Display::Inline;
    doc.append_child(block, span2);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some(" bar".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.white_space = WhiteSpace::PreWrap;
    doc.append_child(span2, t2);

    let data = InlineItemsBuilder::collect(&doc, block);

    // pre-wrap preserves all spaces — double space is expected
    assert!(
        data.text.contains("foo  bar"),
        "pre-wrap should preserve both spaces: {:?}",
        data.text,
    );
}
