//! Advanced inline layout tests — covering edge cases for line-height,
//! vertical-align, text-align, multi-line, mixed content, text-indent,
//! and other corner cases not exercised by inline_layout_tests.rs.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{
    ComputedStyle, Direction, Display, LineHeight, TextAlign, VerticalAlign, WhiteSpace,
};

// ═══════════════════════════════════════════════════════════════════════
// HELPERS (copied from inline_layout_tests.rs)
// ═══════════════════════════════════════════════════════════════════════

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn space(width: i32, height: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu_i(width), lu_i(height))
}

fn layout_text(texts: &[&str], width: i32) -> Fragment {
    let (doc, block) = make_text_block(&texts, width);
    let sp = ConstraintSpace::for_block_child(
        lu_i(width),
        lu_i(600),
        lu_i(width),
        lu_i(600),
        false,
    );
    inline_layout(&doc, block, &sp)
}

fn block_layout_text(texts: &[&str], width: i32) -> Fragment {
    let mut doc = Document::new();
    let vp = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(vp, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);
    }

    let sp = space(width, 600);
    block_layout(&doc, vp, &sp)
}

fn make_text_block(texts: &[&str], _width: i32) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);
    }
    (doc, block)
}

fn make_span_block(
    span_texts: &[&str],
    span_style_fn: impl Fn(&mut ComputedStyle),
) -> (Document, NodeId) {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    span_style_fn(&mut doc.node_mut(span).style);
    doc.append_child(block, span);

    for text in span_texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        let span_style = doc.node(span).style.clone();
        doc.node_mut(t).style.font_size = span_style.font_size;
        doc.node_mut(t).style.line_height = span_style.line_height;
        doc.node_mut(t).style.vertical_align = span_style.vertical_align;
        doc.append_child(span, t);
    }
    (doc, block)
}

#[allow(dead_code)]
fn count_text_fragments(fragment: &Fragment) -> usize {
    let mut count = 0;
    if fragment.kind == FragmentKind::Text {
        count += 1;
    }
    for child in &fragment.children {
        count += count_text_fragments(child);
    }
    count
}

fn count_line_boxes(fragment: &Fragment) -> usize {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .count()
}

fn collect_text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    if fragment.kind == FragmentKind::Text {
        result.push(fragment);
    }
    for child in &fragment.children {
        result.extend(collect_text_fragments(child));
    }
    result
}

// Helper: build inline layout with a custom block style setter.
fn layout_text_with_block_style(
    texts: &[&str],
    width: i32,
    block_style_fn: impl Fn(&mut ComputedStyle),
) -> Fragment {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    block_style_fn(&mut doc.node_mut(block).style);
    doc.append_child(root, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.append_child(block, t);
    }

    let sp = ConstraintSpace::for_block_child(
        lu_i(width),
        lu_i(600),
        lu_i(width),
        lu_i(600),
        false,
    );
    inline_layout(&doc, block, &sp)
}

// Helper: build inline layout with custom per-text style.
fn layout_text_with_text_style(
    texts: &[&str],
    width: i32,
    text_style_fn: impl Fn(&mut ComputedStyle),
) -> Fragment {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        text_style_fn(&mut doc.node_mut(t).style);
        doc.append_child(block, t);
    }

    let sp = ConstraintSpace::for_block_child(
        lu_i(width),
        lu_i(600),
        lu_i(width),
        lu_i(600),
        false,
    );
    inline_layout(&doc, block, &sp)
}

// ═══════════════════════════════════════════════════════════════════════
// 1. LINE HEIGHT EDGE CASES (6 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_line_height_number_zero_produces_minimal_height() {
    // line-height: Number(0.0) — computed line-height = 0 * font-size = 0px.
    // The line box may still have non-zero height from font metrics, but it
    // should be noticeably shorter than the normal default.
    let frag_normal = layout_text(&["Hello"], 800);
    let frag_zero = layout_text_with_text_style(&["Hello"], 800, |s| {
        s.line_height = LineHeight::Number(0.0);
    });
    // Zero line-height should be <= normal line-height
    assert!(
        frag_zero.size.height <= frag_normal.size.height,
        "line-height: 0 ({:?}) should not exceed normal ({:?})",
        frag_zero.size.height,
        frag_normal.size.height
    );
}

#[test]
fn adv_line_height_large_number_multiplier() {
    // line-height: Number(3.0) at 16px → computed = 48px
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Number(3.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Number(3.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(
        frag.size.height >= lu(48.0),
        "line-height: 3.0 at 16px should produce >= 48px, got {:?}",
        frag.size.height
    );
}

#[test]
fn adv_line_height_length_100px() {
    // line-height: Length(100.0) → line box >= 100px
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Length(100.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("x".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Length(100.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(
        frag.size.height >= lu(100.0),
        "line-height: 100px should produce >= 100px, got {:?}",
        frag.size.height
    );
}

#[test]
fn adv_line_height_percentage_50_smaller_than_normal() {
    // line-height: Percentage(50.0) at 16px → computed = 8px.
    // Should be noticeably shorter than normal (~19px).
    let frag_normal = layout_text(&["Hello"], 800);

    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Percentage(50.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Percentage(50.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_50 = inline_layout(&doc, block, &sp);

    assert!(
        frag_50.size.height <= frag_normal.size.height,
        "line-height: 50% ({:?}) should be <= normal ({:?})",
        frag_50.size.height,
        frag_normal.size.height
    );
}

#[test]
fn adv_line_height_two_nodes_tallest_wins() {
    // Two text nodes on the same line with different line-heights.
    // The taller line-height determines the line box height.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("A ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.node_mut(t1).style.line_height = LineHeight::Length(20.0);
    doc.append_child(block, t1);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("B".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.line_height = LineHeight::Length(60.0);
    doc.append_child(block, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);

    // Line box must accommodate the taller item (60px)
    assert!(
        frag.size.height >= lu(60.0),
        "Tallest line-height (60px) should win, got {:?}",
        frag.size.height
    );
}

#[test]
fn adv_line_height_number_half_smaller_than_default() {
    // line-height: Number(0.5) at 16px → computed = 8px.
    let frag_normal = layout_text(&["Hello"], 800);

    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Number(0.5);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Number(0.5);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_half = inline_layout(&doc, block, &sp);

    assert!(
        frag_half.size.height < frag_normal.size.height,
        "line-height: 0.5 ({:?}) should be shorter than normal ({:?})",
        frag_half.size.height,
        frag_normal.size.height
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2. VERTICAL ALIGNMENT (6 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_vertical_align_super_offset_differs_from_baseline() {
    // Place both baseline and super text on the same line and verify
    // their vertical offsets differ (super is higher than baseline).
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("baseline ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(block, t1);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.vertical_align = VerticalAlign::Super;
    doc.append_child(block, span);
    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("super".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.vertical_align = VerticalAlign::Super;
    doc.append_child(span, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 2);
    assert_ne!(
        texts[0].offset.top, texts[1].offset.top,
        "Super text should be at a different offset than baseline text"
    );
}

#[test]
fn adv_vertical_align_sub_offset_below_baseline() {
    // Sub-aligned text should have a larger top offset (pushed downward)
    // compared to baseline-aligned text.
    let frag_bl = layout_text(&["ABC"], 800);
    let (doc, block) = make_span_block(&["ABC"], |s| {
        s.vertical_align = VerticalAlign::Sub;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_sub = inline_layout(&doc, block, &sp);

    let t_bl = collect_text_fragments(&frag_bl);
    let t_sub = collect_text_fragments(&frag_sub);
    assert!(!t_bl.is_empty() && !t_sub.is_empty());
    assert!(
        t_sub[0].offset.top > t_bl[0].offset.top,
        "Sub text ({:?}) should be below baseline text ({:?})",
        t_sub[0].offset.top,
        t_bl[0].offset.top
    );
}

#[test]
fn adv_vertical_align_length_positive_shifts_up() {
    // Place baseline and Length(10.0) text on the same line.
    // The Length(10.0) text should be shifted upward relative to baseline text.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("base ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(block, t1);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.vertical_align = VerticalAlign::Length(10.0);
    doc.append_child(block, span);
    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("up".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.vertical_align = VerticalAlign::Length(10.0);
    doc.append_child(span, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 2);
    // Length(10.0) shifts up → smaller top offset than baseline
    assert!(
        texts[1].offset.top < texts[0].offset.top,
        "Length(10.0) text ({:?}) should be above baseline text ({:?})",
        texts[1].offset.top,
        texts[0].offset.top
    );
}

#[test]
fn adv_vertical_align_length_negative_shifts_down() {
    // vertical-align: Length(-5.0) should push text downward.
    let frag_bl = layout_text(&["x"], 800);
    let (doc, block) = make_span_block(&["x"], |s| {
        s.vertical_align = VerticalAlign::Length(-5.0);
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag_down = inline_layout(&doc, block, &sp);

    let t_bl = collect_text_fragments(&frag_bl);
    let t_down = collect_text_fragments(&frag_down);
    assert!(!t_bl.is_empty() && !t_down.is_empty());
    // Negative length pushes text down
    assert!(
        t_down[0].offset.top > t_bl[0].offset.top,
        "Length(-5.0) should shift text down: shifted={:?} baseline={:?}",
        t_down[0].offset.top,
        t_bl[0].offset.top
    );
}

#[test]
fn adv_vertical_align_top_near_line_box_top() {
    // vertical-align: Top should place text at (or very near) the top of the line box.
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Top;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    // Top-aligned: the text's top offset should be close to 0
    assert!(
        texts[0].offset.top.to_f32().abs() < 5.0,
        "Top-aligned text should be near top of line, got {:?}",
        texts[0].offset.top
    );
}

#[test]
fn adv_vertical_align_bottom_near_line_box_bottom() {
    // vertical-align: Bottom should place text so its bottom aligns with line bottom.
    let (doc, block) = make_span_block(&["Hello"], |s| {
        s.vertical_align = VerticalAlign::Bottom;
    });
    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(!frag.children.is_empty());
    let line = &frag.children[0];
    let text_bottom = texts[0].offset.top + texts[0].size.height;
    // Text bottom should be close to line box bottom
    let diff = (line.size.height - text_bottom).to_f32().abs();
    assert!(
        diff < 5.0,
        "Bottom-aligned text bottom ({:?}) should be near line bottom ({:?})",
        text_bottom,
        line.size.height
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 3. TEXT ALIGNMENT (6 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_text_align_left_short_text_at_zero() {
    // TextAlign::Left with short text: offset.left should be 0
    let frag = layout_text_with_block_style(&["Hi"], 800, |s| {
        s.text_align = TextAlign::Left;
    });
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert_eq!(
        texts[0].offset.left,
        LayoutUnit::zero(),
        "Left-aligned text should start at x=0"
    );
}

#[test]
fn adv_text_align_right_short_text_offset_positive() {
    // TextAlign::Right with short text in 800px container: offset.left > 0
    let frag = layout_text_with_block_style(&["Hi"], 800, |s| {
        s.text_align = TextAlign::Right;
    });
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(
        texts[0].offset.left > LayoutUnit::zero(),
        "Right-aligned text should have positive offset, got {:?}",
        texts[0].offset.left
    );
}

#[test]
fn adv_text_align_center_approximately_centered() {
    // TextAlign::Center: text offset.left ≈ (container_width - text_width) / 2
    let frag = layout_text_with_block_style(&["Hi"], 800, |s| {
        s.text_align = TextAlign::Center;
    });
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    let text_width = texts[0].size.width.to_f32();
    let expected_offset = (800.0 - text_width) / 2.0;
    let actual_offset = texts[0].offset.left.to_f32();
    let delta = (actual_offset - expected_offset).abs();
    assert!(
        delta < 2.0,
        "Centered text offset ({:.1}) should be close to ({:.1}), delta={:.1}",
        actual_offset,
        expected_offset,
        delta
    );
}

#[test]
fn adv_text_align_justify_two_words_wider_spacing() {
    // TextAlign::Justify with exactly 2 words on a non-last line should
    // produce a wider layout than Start-aligned equivalent.
    let text = "Hello World this is a longer sentence that must wrap across lines";

    let frag_start = layout_text_with_block_style(&[text], 200, |s| {
        s.text_align = TextAlign::Start;
    });
    let frag_justify = layout_text_with_block_style(&[text], 200, |s| {
        s.text_align = TextAlign::Justify;
    });

    // Both should produce multiple lines
    assert!(count_line_boxes(&frag_start) > 1);
    assert!(count_line_boxes(&frag_justify) > 1);

    // The justified first line text should extend closer to the right edge
    let just_line = &frag_justify.children[0];
    let just_texts = collect_text_fragments(just_line);
    if !just_texts.is_empty() {
        let last = just_texts.last().unwrap();
        let right_edge = last.offset.left + last.size.width;
        // Justified text should fill close to the available width
        assert!(
            right_edge > lu(150.0),
            "Justified first line should extend towards right edge, got {:?}",
            right_edge
        );
    }
}

#[test]
fn adv_text_align_justify_single_line_not_stretched() {
    // A single-line justified paragraph (= last line) should NOT be stretched,
    // per text-align-last: auto (which defaults to start for justify).
    let frag = layout_text_with_block_style(&["Hi there"], 800, |s| {
        s.text_align = TextAlign::Justify;
    });
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    // Single line = last line → start-aligned → offset.left == 0
    assert_eq!(
        texts[0].offset.left,
        LayoutUnit::zero(),
        "Justify single-line (last line) should be start-aligned"
    );
}

#[test]
fn adv_text_align_start_with_rtl_direction() {
    // TextAlign::Start + Direction::Rtl → text should be right-aligned
    let frag = layout_text_with_block_style(&["Hello"], 800, |s| {
        s.text_align = TextAlign::Start;
        s.direction = Direction::Rtl;
    });
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    assert!(
        texts[0].offset.left > LayoutUnit::zero(),
        "Start + RTL should right-align text, got offset {:?}",
        texts[0].offset.left
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 4. MULTIPLE LINES (6 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_multi_line_three_lines_y_increases() {
    // With 3+ lines, each line box's y (offset.top) should strictly increase.
    let frag = layout_text(
        &["Hello world this is a longer sentence that wraps to three lines"],
        80,
    );
    let lines: Vec<_> = frag
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .collect();
    assert!(
        lines.len() >= 3,
        "Expected >= 3 lines, got {}",
        lines.len()
    );
    for i in 1..lines.len() {
        assert!(
            lines[i].offset.top > lines[i - 1].offset.top,
            "Line {} y ({:?}) should be > line {} y ({:?})",
            i,
            lines[i].offset.top,
            i - 1,
            lines[i - 1].offset.top
        );
    }
}

#[test]
fn adv_multi_line_adjacent_lines() {
    // line2.offset.top ≈ line1.offset.top + line1.size.height
    let frag = layout_text(&["Hello world this is text that wraps"], 80);
    let lines: Vec<_> = frag
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .collect();
    assert!(lines.len() >= 2);
    for i in 1..lines.len() {
        let expected_top = lines[i - 1].offset.top + lines[i - 1].size.height;
        let delta = (lines[i].offset.top - expected_top).to_f32().abs();
        assert!(
            delta < 1.0,
            "Line {} top ({:?}) should equal line {} bottom ({:?}), delta={:.2}",
            i,
            lines[i].offset.top,
            i - 1,
            expected_top,
            delta
        );
    }
}

#[test]
fn adv_multi_line_same_height_same_font() {
    // All lines with the same font should have identical heights.
    let frag = layout_text(
        &["Hello world this text wraps to many lines nicely"],
        60,
    );
    let heights: Vec<_> = frag
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .map(|l| l.size.height)
        .collect();
    assert!(heights.len() >= 2, "Need at least 2 lines");
    for h in &heights[1..] {
        assert_eq!(
            *h, heights[0],
            "All same-font lines should have identical height"
        );
    }
}

#[test]
fn adv_multi_line_text_fragments_on_different_lines() {
    // Each line should contain at least one text fragment.
    let frag = layout_text(&["Hello world this text wraps"], 60);
    let lines: Vec<_> = frag
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .collect();
    assert!(lines.len() >= 2);
    for (i, line) in lines.iter().enumerate() {
        let texts = collect_text_fragments(line);
        assert!(
            !texts.is_empty(),
            "Line {} should have at least one text fragment",
            i
        );
    }
}

#[test]
fn adv_multi_line_count_matches_expected() {
    // Long text in a narrow container should produce multiple lines.
    let frag = layout_text(
        &["Hello world this is a long sentence with many words"],
        80,
    );
    let lines = count_line_boxes(&frag);
    assert!(
        lines >= 3,
        "Long text in 80px should produce >= 3 lines, got {}",
        lines
    );
}

#[test]
fn adv_multi_line_height_grows_with_more_lines() {
    // More wrapping lines → taller fragment. Use text that reliably wraps.
    let text = &["The quick brown fox jumps over the lazy dog and keeps running"];
    let frag_wide = layout_text(text, 800);
    let frag_narrow = layout_text(text, 100);

    let wide_lines = count_line_boxes(&frag_wide);
    let narrow_lines = count_line_boxes(&frag_narrow);
    assert!(
        narrow_lines >= wide_lines,
        "Narrow ({} lines) should have >= wide ({} lines)",
        narrow_lines,
        wide_lines
    );
    assert!(
        frag_narrow.size.height >= frag_wide.size.height,
        "Narrower ({:?}) should be >= wide ({:?})",
        frag_narrow.size.height,
        frag_wide.size.height
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 5. MIXED CONTENT (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_mixed_text_empty_span_text() {
    // Text node + empty span + text node: layout should succeed.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("before ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(block, t1);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(block, span);
    // Empty span — no children

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("after".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.append_child(block, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 2, "Should have two text fragments");
    assert!(frag.size.height > LayoutUnit::zero());
}

#[test]
fn adv_mixed_spans_different_font_sizes() {
    // Multiple spans with different font sizes on one line: the tallest
    // font's metrics should determine the line box height.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.font_size = 10.0;
    doc.append_child(root, block);

    // Small span
    let span1 = doc.create_node(ElementTag::Span);
    doc.node_mut(span1).style.display = Display::Inline;
    doc.node_mut(span1).style.font_size = 10.0;
    doc.append_child(block, span1);
    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("small ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.node_mut(t1).style.font_size = 10.0;
    doc.append_child(span1, t1);

    // Big span
    let span2 = doc.create_node(ElementTag::Span);
    doc.node_mut(span2).style.display = Display::Inline;
    doc.node_mut(span2).style.font_size = 40.0;
    doc.append_child(block, span2);
    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("BIG".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.font_size = 40.0;
    doc.append_child(span2, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(
        frag.size.height > lu(30.0),
        "Line box should accommodate 40px font, got {:?}",
        frag.size.height
    );
}

#[test]
fn adv_mixed_nested_spans_render_text() {
    // Nested spans: outer > inner > text — text still renders correctly.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let outer = doc.create_node(ElementTag::Span);
    doc.node_mut(outer).style.display = Display::Inline;
    doc.append_child(block, outer);

    let inner = doc.create_node(ElementTag::Span);
    doc.node_mut(inner).style.display = Display::Inline;
    doc.append_child(outer, inner);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Nested text".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(inner, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].size.width > LayoutUnit::zero());
    assert!(texts[0].shape_result.is_some());
}

#[test]
fn adv_mixed_text_before_and_after_span() {
    // Plain text, span with text, more plain text — all on one line.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("A ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(block, t1);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(block, span);
    let ts = doc.create_node(ElementTag::Text);
    doc.node_mut(ts).text = Some("B ".to_string());
    doc.node_mut(ts).style.display = Display::Inline;
    doc.append_child(span, ts);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("C".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.append_child(block, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 3, "Should have 3 text fragments: A, B, C");
    assert_eq!(count_line_boxes(&frag), 1, "Should all fit on one line");
}

#[test]
fn adv_mixed_multiple_text_nodes_concatenated() {
    // Multiple adjacent text nodes produce ordered text fragments.
    let frag = layout_text(&["aa", "bb", "cc", "dd"], 800);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 4);
    // Each successive text should be to the right of the previous one
    for i in 1..texts.len() {
        assert!(
            texts[i].offset.left >= texts[i - 1].offset.left,
            "Text {} should be at or right of text {}",
            i,
            i - 1
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// 6. TEXT INDENT (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_text_indent_positive_shifts_first_line_right() {
    // text-indent: 40px → first line text shifted right compared to no indent.
    let frag_no_indent = layout_text(&["Hello world"], 800);
    let frag_indent = layout_text_with_block_style(&["Hello world"], 800, |s| {
        s.text_indent = Length::px(40.0);
    });

    let t_no = collect_text_fragments(&frag_no_indent);
    let t_yes = collect_text_fragments(&frag_indent);
    assert!(!t_no.is_empty() && !t_yes.is_empty());
    assert!(
        t_yes[0].offset.left > t_no[0].offset.left,
        "Indented text ({:?}) should be further right than non-indented ({:?})",
        t_yes[0].offset.left,
        t_no[0].offset.left
    );
}

#[test]
fn adv_text_indent_negative_shifts_first_line_left() {
    // text-indent: -20px → first line text offset may be negative or at 0.
    // It should be less than or equal to the default (0).
    let frag = layout_text_with_block_style(&["Hello world"], 800, |s| {
        s.text_indent = Length::px(-20.0);
    });
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    // Negative indent: text starts at negative offset or is clamped to 0
    assert!(
        texts[0].offset.left <= LayoutUnit::zero(),
        "Negative text-indent should place text at or before x=0, got {:?}",
        texts[0].offset.left
    );
}

#[test]
fn adv_text_indent_only_affects_first_line() {
    // text-indent should only shift the first line; subsequent lines start at 0.
    let frag = layout_text_with_block_style(
        &["Hello world this is a sentence that wraps to multiple lines"],
        100,
        |s| {
            s.text_indent = Length::px(30.0);
        },
    );
    let lines: Vec<_> = frag
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .collect();
    assert!(lines.len() >= 2, "Need at least 2 lines for this test");

    // First line text should be indented
    let first_texts = collect_text_fragments(lines[0]);
    assert!(!first_texts.is_empty());
    assert!(
        first_texts[0].offset.left > LayoutUnit::zero(),
        "First line should be indented"
    );

    // Second line text should NOT be indented
    let second_texts = collect_text_fragments(lines[1]);
    assert!(!second_texts.is_empty());
    assert_eq!(
        second_texts[0].offset.left,
        LayoutUnit::zero(),
        "Second line should not be indented"
    );
}

#[test]
fn adv_text_indent_large_value_may_push_text_off() {
    // text-indent larger than available width: text starts far right,
    // possibly overflowing. Should not panic.
    let frag = layout_text_with_block_style(&["Hi"], 100, |s| {
        s.text_indent = Length::px(200.0);
    });
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
    // Text offset should be large (>= 100px indent or clamped)
    assert!(
        texts[0].offset.left >= lu(100.0),
        "Large indent should push text far right, got {:?}",
        texts[0].offset.left
    );
}

#[test]
fn adv_text_indent_with_center_alignment() {
    // text-indent combined with center alignment: the indented first line
    // should have a different offset than centered-without-indent.
    let frag_center = layout_text_with_block_style(&["Hello"], 800, |s| {
        s.text_align = TextAlign::Center;
    });
    let frag_center_indent = layout_text_with_block_style(&["Hello"], 800, |s| {
        s.text_align = TextAlign::Center;
        s.text_indent = Length::px(50.0);
    });

    let t1 = collect_text_fragments(&frag_center);
    let t2 = collect_text_fragments(&frag_center_indent);
    assert!(!t1.is_empty() && !t2.is_empty());
    // Center + indent should place text differently than center alone
    assert_ne!(
        t1[0].offset.left, t2[0].offset.left,
        "Center+indent ({:?}) should differ from center-only ({:?})",
        t2[0].offset.left,
        t1[0].offset.left
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 7. EDGE CASES (6 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_edge_empty_string_produces_zero_height() {
    // Empty string should produce a fragment with zero height and no line boxes.
    let frag = layout_text(&[""], 800);
    assert_eq!(frag.size.height, LayoutUnit::zero());
    assert_eq!(count_line_boxes(&frag), 0);
}

#[test]
fn adv_edge_single_character_correct_line_box() {
    // A single character should produce exactly one line box with positive dimensions.
    let frag = layout_text(&["A"], 800);
    assert_eq!(count_line_boxes(&frag), 1);
    let line = &frag.children[0];
    assert!(line.size.height > LayoutUnit::zero());
    assert!(line.size.width > LayoutUnit::zero());
    let texts = collect_text_fragments(line);
    assert_eq!(texts.len(), 1);
    assert!(texts[0].size.width > LayoutUnit::zero());
}

#[test]
fn adv_edge_very_wide_container_all_on_one_line() {
    // Very wide container (10000px) should keep everything on one line.
    let frag = layout_text(
        &["Hello world this is a sentence with many words that fits easily"],
        10000,
    );
    assert_eq!(
        count_line_boxes(&frag),
        1,
        "All text should fit on one line in 10000px container"
    );
}

#[test]
fn adv_edge_very_narrow_container_multiple_lines() {
    // Narrow container with many words should produce multiple lines.
    let frag = layout_text(
        &["Hello world this is a long sentence that wraps in a narrow container"],
        60,
    );
    assert!(
        count_line_boxes(&frag) >= 2,
        "60px container with many words should force wrapping, got {} lines",
        count_line_boxes(&frag)
    );
}

#[test]
fn adv_edge_only_whitespace_minimal_layout() {
    // Only whitespace: may collapse to nothing or at most one line.
    let frag = layout_text(&["     "], 800);
    assert!(count_line_boxes(&frag) <= 1);
    // Height should be zero (whitespace collapsed) or a single line
    if count_line_boxes(&frag) == 0 {
        assert_eq!(frag.size.height, LayoutUnit::zero());
    }
}

#[test]
fn adv_edge_block_layout_fragment_width_matches_container() {
    // Block layout: the block fragment width should match the container width.
    let frag = block_layout_text(&["Hello world"], 500);
    let block = &frag.children[0];
    assert_eq!(
        block.size.width.to_i32(),
        500,
        "Block fragment width should match container"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 8. BONUS TESTS (additional coverage)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn adv_bonus_nowrap_all_on_one_line() {
    // white-space: nowrap should keep all text on a single line, even in a narrow container.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello world this should not wrap".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::Nowrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(50), lu_i(600), lu_i(50), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert_eq!(
        count_line_boxes(&frag),
        1,
        "Nowrap should produce exactly 1 line"
    );
}

#[test]
fn adv_bonus_pre_wrap_preserves_spaces() {
    // white-space: pre-wrap should preserve spaces and still wrap.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.white_space = WhiteSpace::PreWrap;
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("Hello   World".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.white_space = WhiteSpace::PreWrap;
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    assert!(count_line_boxes(&frag) >= 1);
    let texts = collect_text_fragments(&frag);
    assert!(!texts.is_empty());
}

#[test]
fn adv_bonus_vertical_align_percentage_differs_from_baseline() {
    // Place baseline and Percentage(50.0) text on the same line.
    // The percentage-aligned text should be shifted relative to baseline.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("base ".to_string());
    doc.node_mut(t1).style.display = Display::Inline;
    doc.append_child(block, t1);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.node_mut(span).style.vertical_align = VerticalAlign::Percentage(50.0);
    doc.node_mut(span).style.line_height = LineHeight::Length(40.0);
    doc.append_child(block, span);
    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("shifted".to_string());
    doc.node_mut(t2).style.display = Display::Inline;
    doc.node_mut(t2).style.vertical_align = VerticalAlign::Percentage(50.0);
    doc.node_mut(t2).style.line_height = LineHeight::Length(40.0);
    doc.append_child(span, t2);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    let texts = collect_text_fragments(&frag);
    assert_eq!(texts.len(), 2);
    // Percentage(50%) of line-height(40px) = 20px shift should differ from baseline
    assert_ne!(
        texts[0].offset.top, texts[1].offset.top,
        "Percentage vertical-align should produce different offset from baseline text"
    );
}

#[test]
fn adv_bonus_line_height_length_1px_very_compact() {
    // line-height: 1px → extremely compact, should not panic.
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.line_height = LineHeight::Length(1.0);
    doc.append_child(root, block);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("compact".to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.node_mut(t).style.line_height = LineHeight::Length(1.0);
    doc.append_child(block, t);

    let sp = ConstraintSpace::for_block_child(lu_i(800), lu_i(600), lu_i(800), lu_i(600), false);
    let frag = inline_layout(&doc, block, &sp);
    // Should not panic and should produce a fragment
    assert!(count_line_boxes(&frag) >= 1);
}

#[test]
fn adv_bonus_text_indent_zero_is_default() {
    // text-indent: 0px should behave identically to no indent.
    let frag_default = layout_text(&["Hello"], 800);
    let frag_zero = layout_text_with_block_style(&["Hello"], 800, |s| {
        s.text_indent = Length::px(0.0);
    });

    let t1 = collect_text_fragments(&frag_default);
    let t2 = collect_text_fragments(&frag_zero);
    assert!(!t1.is_empty() && !t2.is_empty());
    assert_eq!(t1[0].offset.left, t2[0].offset.left);
    assert_eq!(frag_default.size.height, frag_zero.size.height);
}

#[test]
fn adv_bonus_block_layout_wrapping_produces_taller_block() {
    // Block with wrapping text should be taller than block with non-wrapping text.
    let frag_short = block_layout_text(&["Hi"], 800);
    let frag_wrap = block_layout_text(&["Hello world this text wraps"], 50);

    let short_h = frag_short.children[0].size.height;
    let wrap_h = frag_wrap.children[0].size.height;
    assert!(
        wrap_h > short_h,
        "Wrapping block ({:?}) should be taller than single-line ({:?})",
        wrap_h,
        short_h
    );
}
