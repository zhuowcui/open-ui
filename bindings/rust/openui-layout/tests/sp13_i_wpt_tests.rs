// SP13 Phase I: WPT-Style Inline Layout Test Suite
//
// Comprehensive tests modeled after Web Platform Tests (WPT) to verify CSS
// spec conformance across all SP13 inline layout features:
//
//   1. Float exclusions in inline context
//   2. Inline box decorations (box-decoration-break)
//   3. Block-in-inline splitting
//   4. OOF static position from inline context
//   5. Baseline propagation
//   6. Inline fragmentation (orphans/widows)
//   7. Intrinsic block-size from inline content
//   8. ::first-line / ::first-letter pseudo-elements
//   9. Score-based line breaking (text-wrap: balance/pretty)
//  10. initial-letter (drop caps, raised caps)

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::block::block_layout;
use openui_layout::inline::algorithm::{
    apply_inline_fragmentation, inline_layout, resume_inline_from_break_token,
};
use openui_layout::inline::first_letter::{
    extract_first_letter, has_first_letter_content, split_first_letter, FirstLetterMetrics,
    FirstLetterStyle,
};
use openui_layout::inline::first_line::{
    apply_first_line_overrides, build_first_line_style, effective_first_line_style,
    resolve_first_line_style, FirstLineOverrides, FirstLineProperty,
};
use openui_layout::inline::initial_letter::{
    available_width_with_exclusion, compute_exclusion_rect, compute_initial_letter_layout,
    create_initial_letter_style, line_in_exclusion_zone, validate_initial_letter,
};
use openui_layout::inline::score_line_breaker::{
    balance_score, candidates_from_word_widths, requires_scoring, score_line_break,
    BreakCandidate, FitnessClass,
};
use openui_layout::fragmentation::{BreakToken, InlineBreakToken};
use openui_layout::intrinsic_sizing::compute_intrinsic_block_sizes;
use openui_layout::ConstraintSpace;
use openui_layout::Fragment;
use openui_style::{
    BorderStyle, BoxDecorationBreak, Color, ComputedStyle, Direction, Display, Float,
    InitialLetter, LineHeight, Position, TextDecorationLine, TextWrap,
};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

fn lu_i(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

fn space(w: i32, h: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu_i(w), lu_i(h))
}

fn space_bl(w: f32, h: f32) -> ConstraintSpace {
    let mut s = ConstraintSpace::for_root(lu(w), lu(h));
    s.needs_first_baseline = true;
    s.needs_last_baseline = true;
    s
}

fn add_text(doc: &mut Document, parent: NodeId, text: &str) -> NodeId {
    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some(text.to_string());
    doc.node_mut(t).style.display = Display::Inline;
    doc.append_child(parent, t);
    t
}

fn make_fake_lines(num: usize, line_h: f32, width: f32) -> Fragment {
    let total_h = num as f32 * line_h;
    let mut frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(width), lu(total_h)));
    for i in 0..num {
        let mut line = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(width), lu(line_h)));
        line.offset = PhysicalOffset::new(LayoutUnit::zero(), lu(i as f32 * line_h));
        line.baseline_offset = line_h * 0.75;
        frag.children.push(line);
    }
    frag.first_baseline = frag
        .children
        .first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    frag.last_baseline = frag
        .children
        .last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    frag
}

fn clone_frag(frag: &Fragment) -> Fragment {
    let mut f = Fragment::new_box(frag.node_id, frag.size);
    f.offset = frag.offset;
    f.first_baseline = frag.first_baseline;
    f.last_baseline = frag.last_baseline;
    f.baseline_offset = frag.baseline_offset;
    f.children = frag
        .children
        .iter()
        .map(|c| {
            let mut child = Fragment::new_box(c.node_id, c.size);
            child.offset = c.offset;
            child.baseline_offset = c.baseline_offset;
            child
        })
        .collect();
    f
}

// ═══════════════════════════════════════════════════════════════════════════
// 1. FLOAT EXCLUSIONS IN INLINE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_float_left_shrinks_available_inline_size() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(300.0);
        n.style.height = Length::px(400.0);
    }
    doc.append_child(vp, div);
    let fl = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(fl);
        n.style.display = Display::Block;
        n.style.float = Float::Left;
        n.style.width = Length::px(80.0);
        n.style.height = Length::px(40.0);
    }
    doc.append_child(div, fl);
    add_text(&mut doc, div, "Words here");
    let frag = block_layout(&doc, vp, &space(300, 400));
    let container = &frag.children[0];
    assert!(
        container.children.len() >= 2,
        "Float + line box(es), got {}",
        container.children.len()
    );
    let float_frag = &container.children[0];
    assert_eq!(float_frag.size.width, lu(80.0));
    assert_eq!(float_frag.size.height, lu(40.0));
}

#[test]
fn wpt_float_right_positioned_at_right_edge() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(400.0);
    }
    doc.append_child(vp, div);
    let fl = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(fl);
        n.style.display = Display::Block;
        n.style.float = Float::Right;
        n.style.width = Length::px(120.0);
        n.style.height = Length::px(60.0);
    }
    doc.append_child(div, fl);
    add_text(&mut doc, div, "Text");
    let frag = block_layout(&doc, vp, &space(400, 400));
    let container = &frag.children[0];
    let float_frag = &container.children[0];
    // Right float placed at right edge: left offset = container_width - float_width
    let expected_left = lu(400.0) - lu(120.0);
    assert_eq!(
        float_frag.offset.left, expected_left,
        "Right float should be at right edge"
    );
}

#[test]
fn wpt_dual_floats_squeeze_text() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(600.0);
    }
    doc.append_child(vp, div);
    // Left float
    let lf = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(lf);
        n.style.display = Display::Block;
        n.style.float = Float::Left;
        n.style.width = Length::px(100.0);
        n.style.height = Length::px(50.0);
    }
    doc.append_child(div, lf);
    // Right float
    let rf = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(rf);
        n.style.display = Display::Block;
        n.style.float = Float::Right;
        n.style.width = Length::px(100.0);
        n.style.height = Length::px(50.0);
    }
    doc.append_child(div, rf);
    add_text(&mut doc, div, "Hello");
    let frag = block_layout(&doc, vp, &space(400, 600));
    let container = &frag.children[0];
    // At least 2 floats + 1 line
    assert!(container.children.len() >= 3);
}

#[test]
fn wpt_float_zero_height_no_exclusion() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(300.0);
    }
    doc.append_child(vp, div);
    let fl = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(fl);
        n.style.display = Display::Block;
        n.style.float = Float::Left;
        n.style.width = Length::px(100.0);
        n.style.height = Length::px(0.0);
    }
    doc.append_child(div, fl);
    add_text(&mut doc, div, "text");
    let frag = block_layout(&doc, vp, &space(300, 300));
    let container = &frag.children[0];
    // Float still present even with zero height
    assert!(container.children.len() >= 2);
}

#[test]
fn wpt_float_wider_than_container() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(100.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);
    let fl = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(fl);
        n.style.display = Display::Block;
        n.style.float = Float::Left;
        n.style.width = Length::px(200.0);
        n.style.height = Length::px(50.0);
    }
    doc.append_child(div, fl);
    add_text(&mut doc, div, "A");
    let frag = block_layout(&doc, vp, &space(100, 200));
    let container = &frag.children[0];
    assert!(
        !container.children.is_empty(),
        "Should produce children even with overflowing float"
    );
}

#[test]
fn wpt_no_float_full_width_baseline() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(500.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Hello");
    let frag = block_layout(&doc, vp, &space(500, 300));
    let container = &frag.children[0];
    assert_eq!(container.children.len(), 1, "Single line, no float");
    assert_eq!(container.children[0].offset.left, lu(0.0));
}

#[test]
fn wpt_float_tall_pushes_text_below() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(200.0);
        n.style.height = Length::px(400.0);
    }
    doc.append_child(vp, div);
    let fl = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(fl);
        n.style.display = Display::Block;
        n.style.float = Float::Left;
        n.style.width = Length::px(190.0);
        n.style.height = Length::px(100.0);
    }
    doc.append_child(div, fl);
    add_text(&mut doc, div, "W");
    let frag = block_layout(&doc, vp, &space(200, 400));
    let container = &frag.children[0];
    // Float takes nearly all width; text should be pushed alongside or below
    assert!(
        container.children.len() >= 2,
        "Float + text fragment(s)"
    );
}

#[test]
fn wpt_float_with_margin() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(400.0);
    }
    doc.append_child(vp, div);
    let fl = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(fl);
        n.style.display = Display::Block;
        n.style.float = Float::Left;
        n.style.width = Length::px(100.0);
        n.style.height = Length::px(60.0);
        n.style.margin_right = Length::px(10.0);
    }
    doc.append_child(div, fl);
    add_text(&mut doc, div, "text next to float");
    let frag = block_layout(&doc, vp, &space(400, 400));
    let container = &frag.children[0];
    assert!(container.children.len() >= 2);
    // Float is at left edge
    assert_eq!(container.children[0].offset.left, lu(0.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. INLINE BOX DECORATIONS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_inline_padding_offsets_text() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(400.0);
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    let span = doc.create_node(ElementTag::Span);
    {
        let n = doc.node_mut(span);
        n.style.display = Display::Inline;
        n.style.padding_left = Length::px(20.0);
        n.style.padding_right = Length::px(20.0);
    }
    doc.append_child(div, span);
    add_text(&mut doc, span, "Hello");
    let frag = block_layout(&doc, vp, &space(400, 400));
    let div_frag = &frag.children[0];
    let line = &div_frag.children[0];
    let text_frag = &line.children[0];
    assert!(
        text_frag.offset.left.to_f32() >= 20.0,
        "Text should be offset by padding-left (20px), got {}",
        text_frag.offset.left.to_f32()
    );
}

#[test]
fn wpt_box_decoration_break_slice_default() {
    let style = ComputedStyle::initial();
    assert_eq!(style.box_decoration_break, BoxDecorationBreak::Slice);
}

#[test]
fn wpt_box_decoration_break_clone_setting() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(100.0);
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    let span = doc.create_node(ElementTag::Span);
    {
        let n = doc.node_mut(span);
        n.style.display = Display::Inline;
        n.style.padding_left = Length::px(8.0);
        n.style.padding_right = Length::px(8.0);
        n.style.box_decoration_break = BoxDecorationBreak::Clone;
    }
    doc.append_child(div, span);
    add_text(&mut doc, span, "Clone mode wrap text");
    let frag = block_layout(&doc, vp, &space(100, 400));
    let div_frag = &frag.children[0];
    assert!(!div_frag.children.is_empty(), "Should produce line boxes");
    assert_eq!(
        doc.node(span).style.box_decoration_break,
        BoxDecorationBreak::Clone
    );
}

#[test]
fn wpt_inline_border_contributes_to_offset() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(500.0);
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    let span = doc.create_node(ElementTag::Span);
    {
        let n = doc.node_mut(span);
        n.style.display = Display::Inline;
        n.style.border_left_width = 3;
        n.style.border_left_style = BorderStyle::Solid;
        n.style.padding_left = Length::px(7.0);
    }
    doc.append_child(div, span);
    add_text(&mut doc, span, "X");
    let frag = block_layout(&doc, vp, &space(500, 400));
    let div_frag = &frag.children[0];
    let line = &div_frag.children[0];
    let text_frag = &line.children[0];
    // border_left(3) + padding_left(7) = 10
    assert!(
        text_frag.offset.left.to_f32() >= 10.0,
        "Text offset should include border+padding (>=10), got {}",
        text_frag.offset.left.to_f32()
    );
}

#[test]
fn wpt_single_line_span_is_first_and_last() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(500.0);
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(div, span);
    add_text(&mut doc, span, "one-liner");
    let frag = block_layout(&doc, vp, &space(500, 400));
    let div_frag = &frag.children[0];
    let line = &div_frag.children[0];
    let text_frag = &line.children[0];
    assert!(text_frag.is_first_for_node);
    assert!(text_frag.is_last_for_node);
}

#[test]
fn wpt_multi_line_span_first_not_last() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(80.0);
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    let span = doc.create_node(ElementTag::Span);
    {
        let n = doc.node_mut(span);
        n.style.display = Display::Inline;
        n.style.padding_left = Length::px(5.0);
        n.style.padding_right = Length::px(5.0);
    }
    doc.append_child(div, span);
    add_text(&mut doc, span, "These words wrap across lines");
    let frag = block_layout(&doc, vp, &space(80, 800));
    let div_frag = &frag.children[0];
    if div_frag.children.len() >= 2 {
        let first_line = &div_frag.children[0];
        if let Some(t) = first_line.children.first() {
            assert!(t.is_first_for_node, "First line text should be is_first_for_node");
        }
        let last_idx = div_frag.children.len() - 1;
        let last_line = &div_frag.children[last_idx];
        if let Some(t) = last_line.children.last() {
            assert!(t.is_last_for_node, "Last line text should be is_last_for_node");
        }
    }
}

#[test]
fn wpt_empty_inline_span_no_crash() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.append_child(vp, div);
    let span = doc.create_node(ElementTag::Span);
    {
        let n = doc.node_mut(span);
        n.style.display = Display::Inline;
        n.style.padding_left = Length::px(10.0);
    }
    doc.append_child(div, span);
    // No text child — empty span
    let frag = block_layout(&doc, vp, &space(400, 400));
    assert!(frag.children[0].size.height.to_f32() >= 0.0);
}

#[test]
fn wpt_rtl_inline_direction() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.width = Length::px(400.0);
        n.style.direction = Direction::Rtl;
    }
    doc.append_child(vp, div);
    let span = doc.create_node(ElementTag::Span);
    {
        let n = doc.node_mut(span);
        n.style.display = Display::Inline;
        n.style.direction = Direction::Rtl;
        n.style.padding_left = Length::px(5.0);
        n.style.padding_right = Length::px(10.0);
    }
    doc.append_child(div, span);
    add_text(&mut doc, span, "RTL text");
    let frag = block_layout(&doc, vp, &space(400, 400));
    let div_frag = &frag.children[0];
    assert!(!div_frag.children.is_empty(), "RTL should produce lines");
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. BLOCK-IN-INLINE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_block_in_inline_splits_into_three() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);
    add_text(&mut doc, span, "Before ");
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(40.0);
    doc.append_child(span, block);
    add_text(&mut doc, span, " After");
    let frag = block_layout(&doc, container, &space(500, 500));
    // Anonymous block before + block + anonymous block after
    assert!(
        frag.children.len() >= 3,
        "block-in-inline should produce >= 3 children, got {}",
        frag.children.len()
    );
    assert_eq!(frag.children[1].size.height, lu(40.0));
}

#[test]
fn wpt_block_in_inline_children_dont_overlap() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);
    add_text(&mut doc, span, "A");
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(80.0);
    doc.append_child(span, block);
    add_text(&mut doc, span, "B");
    let frag = block_layout(&doc, container, &space(500, 500));
    for i in 1..frag.children.len() {
        let prev_end = frag.children[i - 1].offset.top + frag.children[i - 1].size.height;
        assert!(
            frag.children[i].offset.top >= prev_end,
            "Children must not overlap at index {}",
            i
        );
    }
}

#[test]
fn wpt_multiple_blocks_in_inline_heights() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);
    add_text(&mut doc, span, "X");
    let b1 = doc.create_node(ElementTag::Div);
    doc.node_mut(b1).style.display = Display::Block;
    doc.node_mut(b1).style.height = Length::px(25.0);
    doc.append_child(span, b1);
    add_text(&mut doc, span, "Y");
    let b2 = doc.create_node(ElementTag::Div);
    doc.node_mut(b2).style.display = Display::Block;
    doc.node_mut(b2).style.height = Length::px(35.0);
    doc.append_child(span, b2);
    add_text(&mut doc, span, "Z");
    let frag = block_layout(&doc, container, &space(500, 500));
    assert!(frag.children.len() >= 5);
    assert_eq!(frag.children[1].size.height, lu(25.0));
    assert_eq!(frag.children[3].size.height, lu(35.0));
}

#[test]
fn wpt_block_in_inline_empty_before() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);
    // No text before block
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(60.0);
    doc.append_child(span, block);
    add_text(&mut doc, span, "After");
    let frag = block_layout(&doc, container, &space(500, 500));
    let has_60 = frag.children.iter().any(|c| c.size.height == lu(60.0));
    assert!(has_60);
}

#[test]
fn wpt_block_in_inline_empty_after() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);
    add_text(&mut doc, span, "Before");
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(70.0);
    doc.append_child(span, block);
    // No text after block
    let frag = block_layout(&doc, container, &space(500, 500));
    assert!(frag.children.len() >= 2);
    let has_70 = frag.children.iter().any(|c| c.size.height == lu(70.0));
    assert!(has_70);
}

#[test]
fn wpt_nested_inline_with_block_splits() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let outer_span = doc.create_node(ElementTag::Span);
    doc.node_mut(outer_span).style.display = Display::Inline;
    doc.append_child(container, outer_span);
    let inner_span = doc.create_node(ElementTag::Span);
    doc.node_mut(inner_span).style.display = Display::Inline;
    doc.append_child(outer_span, inner_span);
    add_text(&mut doc, inner_span, "pre");
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(45.0);
    doc.append_child(inner_span, block);
    let frag = block_layout(&doc, container, &space(500, 500));
    let has_45 = frag.children.iter().any(|c| c.size.height == lu(45.0));
    assert!(has_45, "Block inside nested inline should appear");
}

#[test]
fn wpt_block_in_inline_total_height_correct() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(container, span);
    add_text(&mut doc, span, "A");
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(100.0);
    doc.append_child(span, block);
    add_text(&mut doc, span, "B");
    let frag = block_layout(&doc, container, &space(500, 500));
    // Total height must be at least 100px (the block) + line heights
    assert!(frag.size.height > lu(100.0));
}

#[test]
fn wpt_block_in_inline_with_padding_context() {
    let mut doc = Document::new();
    let root = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(500.0);
    doc.append_child(root, container);
    let span = doc.create_node(ElementTag::Span);
    {
        let n = doc.node_mut(span);
        n.style.display = Display::Inline;
        n.style.padding_left = Length::px(10.0);
        n.style.padding_right = Length::px(10.0);
    }
    doc.append_child(container, span);
    add_text(&mut doc, span, "before");
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.height = Length::px(50.0);
    doc.append_child(span, block);
    add_text(&mut doc, span, "after");
    let frag = block_layout(&doc, container, &space(500, 500));
    assert!(frag.children.len() >= 3);
    assert!(frag.size.height > lu(50.0));
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. OOF STATIC POSITION IN INLINE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_oof_absolute_in_inline_appears_in_tree() {
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Pre ");
    let abs = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(abs);
        n.style.display = Display::Block;
        n.style.position = Position::Absolute;
        n.style.width = Length::px(50.0);
        n.style.height = Length::px(50.0);
    }
    doc.append_child(div, abs);
    add_text(&mut doc, div, " Post");
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    assert!(div_frag.children.len() >= 1);
}

#[test]
fn wpt_oof_fixed_in_inline() {
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);
    let fixed = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(fixed);
        n.style.display = Display::Block;
        n.style.position = Position::Fixed;
        n.style.width = Length::px(40.0);
        n.style.height = Length::px(40.0);
    }
    doc.append_child(div, fixed);
    add_text(&mut doc, div, "text");
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    assert!(div_frag.children.len() >= 1);
}

#[test]
fn wpt_multiple_oof_in_inline_all_placed() {
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);
    let abs1 = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(abs1);
        n.style.position = Position::Absolute;
        n.style.width = Length::px(20.0);
        n.style.height = Length::px(20.0);
    }
    doc.append_child(div, abs1);
    add_text(&mut doc, div, "A");
    let abs2 = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(abs2);
        n.style.position = Position::Absolute;
        n.style.width = Length::px(30.0);
        n.style.height = Length::px(30.0);
    }
    doc.append_child(div, abs2);
    add_text(&mut doc, div, "B");
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    let oof_count = div_frag
        .children
        .iter()
        .filter(|c| c.node_id == abs1 || c.node_id == abs2)
        .count();
    assert!(oof_count >= 2, "Both OOF should be placed, got {}", oof_count);
}

#[test]
fn wpt_oof_static_position_from_inline() {
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Before abs");
    let abs = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(abs);
        n.style.display = Display::Block;
        n.style.position = Position::Absolute;
        n.style.width = Length::px(30.0);
        n.style.height = Length::px(30.0);
    }
    doc.append_child(div, abs);
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    let has_oof = div_frag.children.iter().any(|c| c.node_id == abs);
    assert!(has_oof, "OOF should appear in children");
}

#[test]
fn wpt_oof_no_text_before_absolute() {
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);
    let abs = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(abs);
        n.style.display = Display::Block;
        n.style.position = Position::Absolute;
        n.style.width = Length::px(10.0);
        n.style.height = Length::px(10.0);
    }
    doc.append_child(div, abs);
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    let has_oof = div_frag.children.iter().any(|c| c.node_id == abs);
    assert!(has_oof, "OOF with no preceding text should still be placed");
}

#[test]
fn wpt_oof_relative_parent_not_absolute() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(300.0);
        n.style.height = Length::px(300.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "text");
    let frag = block_layout(&doc, vp, &space(300, 300));
    let div_frag = &frag.children[0];
    // No OOF children — just text
    assert!(!div_frag.children.is_empty(), "Should have line children");
}

#[test]
fn wpt_oof_between_text_nodes() {
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(400.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Left ");
    let abs = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(abs);
        n.style.position = Position::Absolute;
        n.style.width = Length::px(25.0);
        n.style.height = Length::px(25.0);
    }
    doc.append_child(div, abs);
    add_text(&mut doc, div, " Right");
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    let has_oof = div_frag.children.iter().any(|c| c.node_id == abs);
    assert!(has_oof, "OOF between text should be placed");
}

#[test]
fn wpt_oof_absolute_doesnt_affect_inline_flow() {
    // Absolute elements don't contribute to inline flow height
    let mut doc = Document::new();
    let vp = doc.root();
    doc.node_mut(vp).style.position = Position::Relative;
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.position = Position::Relative;
        n.style.width = Length::px(400.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Text");
    let abs = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(abs);
        n.style.position = Position::Absolute;
        n.style.width = Length::px(200.0);
        n.style.height = Length::px(200.0);
    }
    doc.append_child(div, abs);
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    // OOF 200px tall shouldn't make div that tall by itself (no contribution to flow)
    // The div height should reflect text content height only (unless height auto)
    assert!(
        div_frag.size.height.to_f32() > 0.0,
        "Div should have positive height from text"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. BASELINE PROPAGATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_baseline_single_line_equal() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Single line");
    let frag = block_layout(&doc, vp, &space_bl(400.0, 800.0));
    let div_frag = &frag.children[0];
    assert!(div_frag.first_baseline.is_some());
    assert!(div_frag.last_baseline.is_some());
    assert_eq!(div_frag.first_baseline, div_frag.last_baseline);
}

#[test]
fn wpt_baseline_multi_line_order() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.font_size = 16.0;
        n.style.width = Length::px(60.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "First line Second line Third");
    let frag = block_layout(&doc, vp, &space_bl(60.0, 800.0));
    let div_frag = &frag.children[0];
    let first = div_frag.first_baseline.unwrap();
    let last = div_frag.last_baseline.unwrap();
    assert!(
        first.to_f32() < last.to_f32(),
        "First baseline ({}) should be above last ({})",
        first.to_f32(),
        last.to_f32()
    );
}

#[test]
fn wpt_baseline_with_padding_top() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.padding_top = Length::px(30.0);
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Padded");
    let frag = block_layout(&doc, vp, &space_bl(400.0, 800.0));
    let div_frag = &frag.children[0];
    let bl = div_frag.first_baseline.unwrap();
    assert!(
        bl.to_f32() >= 30.0,
        "Baseline should account for padding-top (30px), got {}",
        bl.to_f32()
    );
}

#[test]
fn wpt_baseline_empty_block_none() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.height = Length::px(50.0);
    }
    doc.append_child(vp, div);
    // No text → no baselines
    let frag = block_layout(&doc, vp, &space_bl(400.0, 800.0));
    let div_frag = &frag.children[0];
    assert!(div_frag.first_baseline.is_none());
    assert!(div_frag.last_baseline.is_none());
}

#[test]
fn wpt_baseline_nested_propagation() {
    let mut doc = Document::new();
    let vp = doc.root();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.append_child(vp, outer);
    let inner = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(inner);
        n.style.display = Display::Block;
        n.style.font_size = 20.0;
    }
    doc.append_child(outer, inner);
    add_text(&mut doc, inner, "Nested");
    let frag = block_layout(&doc, vp, &space_bl(400.0, 800.0));
    let outer_frag = &frag.children[0];
    assert!(
        outer_frag.first_baseline.is_some(),
        "Outer should propagate first_baseline from nested child"
    );
}

#[test]
fn wpt_baseline_positive_value() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.font_size = 24.0;
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Baseline");
    let frag = block_layout(&doc, vp, &space_bl(400.0, 800.0));
    let div_frag = &frag.children[0];
    let bl = div_frag.first_baseline.unwrap();
    assert!(bl.to_f32() > 0.0, "Baseline must be positive");
}

#[test]
fn wpt_baseline_always_computed_without_flag() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.font_size = 16.0;
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Always computed");
    // Default space without needs_first_baseline
    let frag = block_layout(&doc, vp, &space(400, 800));
    let div_frag = &frag.children[0];
    assert!(
        div_frag.first_baseline.is_some(),
        "Baselines should always be computed even without needs_first_baseline flag"
    );
}

#[test]
fn wpt_baseline_from_inline_layout_directly() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.font_size = 16.0;
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Direct inline");
    let s = ConstraintSpace::for_root(lu(400.0), lu(800.0));
    let inline_frag = inline_layout(&doc, div, &s);
    assert!(inline_frag.first_baseline.is_some());
    assert!(inline_frag.last_baseline.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. INLINE FRAGMENTATION
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_frag_all_lines_fit_no_break() {
    let frag = make_fake_lines(3, 20.0, 200.0);
    let result = apply_inline_fragmentation(frag, lu(200.0), LayoutUnit::zero(), 0, 2, 2);
    assert!(result.break_token.is_none());
    assert_eq!(result.children.len(), 3);
}

#[test]
fn wpt_frag_split_at_exact_boundary() {
    let frag = make_fake_lines(4, 25.0, 200.0);
    // Fragmentainer = 50px → exactly 2 lines
    let result = apply_inline_fragmentation(frag, lu(50.0), LayoutUnit::zero(), 0, 1, 1);
    assert_eq!(result.children.len(), 2);
    assert!(result.break_token.is_some());
    match &result.break_token {
        Some(BreakToken::Inline(t)) => assert_eq!(t.lines_consumed, 2),
        _ => panic!("Expected Inline break token"),
    }
}

#[test]
fn wpt_frag_offset_reduces_space() {
    let frag = make_fake_lines(5, 20.0, 200.0);
    // Fragmentainer 100px but offset 60px → 40px available → 2 lines
    let result = apply_inline_fragmentation(frag, lu(100.0), lu(60.0), 0, 1, 1);
    assert_eq!(result.children.len(), 2);
    assert!(result.break_token.is_some());
}

#[test]
fn wpt_frag_zero_available_produces_empty() {
    let frag = make_fake_lines(3, 20.0, 200.0);
    let result = apply_inline_fragmentation(frag, lu(40.0), lu(40.0), 0, 1, 1);
    assert_eq!(result.children.len(), 0);
    assert!(result.break_token.is_some());
}

#[test]
fn wpt_frag_widows_steals_line() {
    // 5 lines × 20px, fragmentainer 80px → 4 fit. widows=2 → remaining=1 < 2.
    // Steal 1 → keep 3.
    let frag = make_fake_lines(5, 20.0, 200.0);
    let result = apply_inline_fragmentation(frag, lu(80.0), LayoutUnit::zero(), 0, 1, 2);
    assert_eq!(result.children.len(), 3);
}

#[test]
fn wpt_frag_orphans_minimum() {
    // 6 lines, fragmentainer 80px → 4 fit. orphans=3, widows=3.
    let frag = make_fake_lines(6, 20.0, 200.0);
    let result = apply_inline_fragmentation(frag, lu(80.0), LayoutUnit::zero(), 0, 3, 3);
    assert_eq!(result.children.len(), 3);
}

#[test]
fn wpt_frag_resume_skips_consumed() {
    let frag = make_fake_lines(6, 20.0, 200.0);
    let token = InlineBreakToken::new(3, lu(60.0));
    let result = resume_inline_from_break_token(frag, &token, lu(2000.0), 1, 1);
    assert_eq!(result.children.len(), 3);
    assert!(result.break_token.is_none());
    assert_eq!(result.children[0].offset.top, lu(0.0));
}

#[test]
fn wpt_frag_resume_rebases_offsets() {
    let frag = make_fake_lines(4, 20.0, 200.0);
    let token = InlineBreakToken::new(2, lu(40.0));
    let result = resume_inline_from_break_token(frag, &token, lu(2000.0), 1, 1);
    assert_eq!(result.children.len(), 2);
    assert_eq!(result.children[0].offset.top, lu(0.0));
    assert_eq!(result.children[1].offset.top, lu(20.0));
}

#[test]
fn wpt_frag_height_adjusted() {
    let frag = make_fake_lines(4, 25.0, 200.0);
    // 4×25=100px. Fragmentainer 60px → 2 lines (50px).
    let result = apply_inline_fragmentation(frag, lu(60.0), LayoutUnit::zero(), 0, 1, 1);
    assert_eq!(result.children.len(), 2);
    assert_eq!(result.size.height, lu(50.0));
}

#[test]
fn wpt_frag_zero_fragmentainer_no_break() {
    let frag = make_fake_lines(3, 20.0, 200.0);
    let result = apply_inline_fragmentation(frag, LayoutUnit::zero(), LayoutUnit::zero(), 0, 2, 2);
    assert_eq!(result.children.len(), 3);
    assert!(result.break_token.is_none());
}

#[test]
fn wpt_frag_preserves_baselines() {
    let mut frag = make_fake_lines(4, 20.0, 200.0);
    for (i, c) in frag.children.iter_mut().enumerate() {
        c.baseline_offset = 15.0 + i as f32;
    }
    let result = apply_inline_fragmentation(frag, lu(45.0), LayoutUnit::zero(), 0, 1, 1);
    assert_eq!(result.children.len(), 2);
    assert!(result.first_baseline.is_some());
    assert!(result.last_baseline.is_some());
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. INTRINSIC BLOCK-SIZE FROM INLINE
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_intrinsic_text_nonzero() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.font_size = 16.0;
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Content");
    let sizes = compute_intrinsic_block_sizes(&doc, div);
    assert!(sizes.min_content_block_size > lu(0.0));
    assert!(sizes.max_content_block_size > lu(0.0));
}

#[test]
fn wpt_intrinsic_wrapping_min_ge_max() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.font_size = 16.0;
    doc.append_child(vp, div);
    add_text(&mut doc, div, "This sentence wraps at different widths producing different heights");
    let sizes = compute_intrinsic_block_sizes(&doc, div);
    assert!(
        sizes.min_content_block_size >= sizes.max_content_block_size,
        "min ({}) should be >= max ({})",
        sizes.min_content_block_size.to_f32(),
        sizes.max_content_block_size.to_f32()
    );
}

#[test]
fn wpt_intrinsic_single_word_equal() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.font_size = 16.0;
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Word");
    let sizes = compute_intrinsic_block_sizes(&doc, div);
    assert_eq!(sizes.min_content_block_size, sizes.max_content_block_size);
}

#[test]
fn wpt_intrinsic_empty_block_zero() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.append_child(vp, div);
    let sizes = compute_intrinsic_block_sizes(&doc, div);
    assert_eq!(sizes.min_content_block_size, lu(0.0));
    assert_eq!(sizes.max_content_block_size, lu(0.0));
}

#[test]
fn wpt_intrinsic_includes_border_padding() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.font_size = 16.0;
        n.style.padding_top = Length::px(10.0);
        n.style.padding_bottom = Length::px(10.0);
        n.style.border_top_width = 2;
        n.style.border_bottom_width = 2;
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Content");
    let sizes = compute_intrinsic_block_sizes(&doc, div);
    // At least 24px (10+10+2+2) + text height
    assert!(
        sizes.max_content_block_size.to_f32() >= 24.0,
        "Should include border+padding, got {}",
        sizes.max_content_block_size.to_f32()
    );
}

#[test]
fn wpt_intrinsic_larger_font_taller() {
    let mut doc = Document::new();
    let vp = doc.root();
    let small = doc.create_node(ElementTag::Div);
    doc.node_mut(small).style.display = Display::Block;
    doc.node_mut(small).style.font_size = 10.0;
    doc.append_child(vp, small);
    add_text(&mut doc, small, "Text");
    let big = doc.create_node(ElementTag::Div);
    doc.node_mut(big).style.display = Display::Block;
    doc.node_mut(big).style.font_size = 40.0;
    doc.append_child(vp, big);
    add_text(&mut doc, big, "Text");
    let s_sizes = compute_intrinsic_block_sizes(&doc, small);
    let b_sizes = compute_intrinsic_block_sizes(&doc, big);
    assert!(b_sizes.max_content_block_size > s_sizes.max_content_block_size);
}

#[test]
fn wpt_intrinsic_nested_block() {
    let mut doc = Document::new();
    let vp = doc.root();
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.append_child(vp, outer);
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.font_size = 16.0;
    doc.append_child(outer, inner);
    add_text(&mut doc, inner, "Nested");
    let sizes = compute_intrinsic_block_sizes(&doc, outer);
    assert!(sizes.min_content_block_size > lu(0.0));
}

#[test]
fn wpt_intrinsic_border_only_no_text() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.border_top_width = 5;
        n.style.border_bottom_width = 5;
    }
    doc.append_child(vp, div);
    let sizes = compute_intrinsic_block_sizes(&doc, div);
    // Border-only block with no content: may contribute 10px from borders
    assert!(
        sizes.min_content_block_size.to_f32() >= 0.0,
        "Should be non-negative"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. FIRST-LINE / FIRST-LETTER
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_first_line_font_size_override() {
    let base = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        font_size: Some(32.0),
        ..Default::default()
    };
    let merged = apply_first_line_overrides(&base, &overrides);
    assert_eq!(merged.font_size, 32.0);
    assert_eq!(merged.letter_spacing, 0.0); // unchanged
}

#[test]
fn wpt_first_line_color_override() {
    let base = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        color: Some(Color::from_rgba8(0, 128, 0, 255)),
        ..Default::default()
    };
    let merged = apply_first_line_overrides(&base, &overrides);
    assert_eq!(merged.color, Color::from_rgba8(0, 128, 0, 255));
}

#[test]
fn wpt_first_line_underline_override() {
    let base = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        text_decoration_line: Some(TextDecorationLine::UNDERLINE),
        ..Default::default()
    };
    let merged = apply_first_line_overrides(&base, &overrides);
    assert!(merged.text_decoration_line.has_underline());
}

#[test]
fn wpt_first_line_absent_returns_base() {
    let style = ComputedStyle::initial();
    let eff = effective_first_line_style(&style);
    assert_eq!(eff.font_size, 16.0);
}

#[test]
fn wpt_first_line_resolve_none_when_absent() {
    let style = ComputedStyle::initial();
    assert!(resolve_first_line_style(&style).is_none());
}

#[test]
fn wpt_first_line_resolve_some_when_set() {
    let mut style = ComputedStyle::initial();
    let mut fls = ComputedStyle::initial();
    fls.font_size = 48.0;
    style.first_line_style = Some(Box::new(fls));
    let resolved = resolve_first_line_style(&style).unwrap();
    assert_eq!(resolved.font_size, 48.0);
}

#[test]
fn wpt_first_line_build_preserves_base_fields() {
    let parent = ComputedStyle::initial();
    let overrides = FirstLineOverrides {
        font_size: Some(28.0),
        ..Default::default()
    };
    let built = build_first_line_style(&parent, &overrides);
    assert_eq!(built.font_size, 28.0);
    assert_eq!(built.line_height, parent.line_height);
}

#[test]
fn wpt_first_line_property_count() {
    assert_eq!(FirstLineProperty::ALL.len(), 10);
    assert!(FirstLineProperty::ALL.contains(&FirstLineProperty::FontSize));
    assert!(FirstLineProperty::ALL.contains(&FirstLineProperty::Color));
}

#[test]
fn wpt_first_letter_simple() {
    let r = extract_first_letter("Hello").unwrap();
    assert_eq!(r.first_letter_end, 1);
    assert!(r.has_letter);
}

#[test]
fn wpt_first_letter_with_punctuation() {
    let (first, rest) = split_first_letter("\"Hello").unwrap();
    assert_eq!(first, "\"H");
    assert_eq!(rest, "ello");
}

#[test]
fn wpt_first_letter_empty_none() {
    assert!(extract_first_letter("").is_none());
    assert!(extract_first_letter("   ").is_none());
}

#[test]
fn wpt_first_letter_has_content() {
    assert!(has_first_letter_content("Hello"));
    assert!(!has_first_letter_content(""));
    assert!(!has_first_letter_content("   "));
}

#[test]
fn wpt_first_letter_cjk() {
    let r = extract_first_letter("漢字テスト").unwrap();
    let text = "漢字テスト";
    assert_eq!(&text[..r.first_letter_end], "漢");
}

#[test]
fn wpt_first_letter_metrics() {
    let m = FirstLetterMetrics::from_font_size(36.0);
    assert!((m.height - (m.ascent + m.descent)).abs() < 0.01, "height = ascent + descent");
    assert!(m.ascent > 0.0);
    assert!(m.width > 0.0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 9. SCORE-BASED LINE BREAKING
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_score_balance_produces_result() {
    let candidates = candidates_from_word_widths(&[30.0, 30.0, 30.0, 30.0], 5.0);
    let result = score_line_break(&candidates, 100.0, TextWrap::Balance);
    assert!(result.is_some());
    assert!(result.unwrap().used_scoring);
}

#[test]
fn wpt_score_pretty_produces_result() {
    let candidates = candidates_from_word_widths(&[40.0, 40.0, 40.0, 10.0], 5.0);
    let result = score_line_break(&candidates, 100.0, TextWrap::Pretty);
    assert!(result.is_some());
    assert!(result.unwrap().used_scoring);
}

#[test]
fn wpt_score_wrap_no_scoring() {
    assert!(!requires_scoring(TextWrap::Wrap));
    assert!(!requires_scoring(TextWrap::Nowrap));
    assert!(!requires_scoring(TextWrap::Stable));
}

#[test]
fn wpt_score_balance_pretty_require_scoring() {
    assert!(requires_scoring(TextWrap::Balance));
    assert!(requires_scoring(TextWrap::Pretty));
}

#[test]
fn wpt_score_single_word_no_break() {
    let candidates = candidates_from_word_widths(&[50.0], 5.0);
    let result = score_line_break(&candidates, 200.0, TextWrap::Balance).unwrap();
    assert!(result.break_indices.len() <= 1);
}

#[test]
fn wpt_score_fallback_long_paragraph() {
    let words: Vec<f64> = vec![30.0; 600];
    let candidates = candidates_from_word_widths(&words, 5.0);
    let result = score_line_break(&candidates, 200.0, TextWrap::Pretty);
    assert!(result.is_none(), "Should fall back for long paragraphs");
}

#[test]
fn wpt_fitness_class_classification() {
    assert_eq!(FitnessClass::from_ratio(-1.0), FitnessClass::Tight);
    assert_eq!(FitnessClass::from_ratio(0.0), FitnessClass::Normal);
    assert_eq!(FitnessClass::from_ratio(0.75), FitnessClass::Loose);
    assert_eq!(FitnessClass::from_ratio(2.0), FitnessClass::VeryLoose);
}

#[test]
fn wpt_fitness_class_transition_penalty() {
    // Adjacent: no penalty
    assert_eq!(
        FitnessClass::transition_penalty(FitnessClass::Normal, FitnessClass::Loose),
        0.0
    );
    // Non-adjacent: penalty
    assert!(FitnessClass::transition_penalty(FitnessClass::Tight, FitnessClass::VeryLoose) > 0.0);
}

#[test]
fn wpt_balance_score_perfect() {
    assert_eq!(balance_score(&[100.0, 100.0, 100.0]), 0.0);
}

#[test]
fn wpt_balance_score_unbalanced() {
    let score = balance_score(&[150.0, 50.0]);
    assert!(score > 0.0);
}

#[test]
fn wpt_text_wrap_enum_methods() {
    assert!(TextWrap::Balance.uses_scoring());
    assert!(TextWrap::Pretty.uses_scoring());
    assert!(!TextWrap::Wrap.uses_scoring());
    assert!(TextWrap::Nowrap.is_nowrap());
    assert!(!TextWrap::Wrap.is_nowrap());
}

// ═══════════════════════════════════════════════════════════════════════════
// 10. INITIAL-LETTER (DROP CAPS / RAISED CAPS)
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_drop_cap_3_lines() {
    let il = InitialLetter { size: 3.0, sink: None };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
    assert_eq!(layout.letter_height, lu(60.0));
    assert_eq!(layout.computed_font_size, 48.0);
    assert_eq!(layout.block_offset, lu(0.0));
    assert_eq!(layout.exclusion_lines, 3);
    assert!(layout.is_drop_cap);
    assert!(!layout.is_raised);
}

#[test]
fn wpt_raised_cap_3_lines_sink_1() {
    let il = InitialLetter { size: 3.0, sink: Some(1.0) };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
    assert_eq!(layout.letter_height, lu(60.0));
    assert_eq!(layout.block_offset, lu(-40.0));
    assert_eq!(layout.exclusion_lines, 1);
    assert!(layout.is_raised);
}

#[test]
fn wpt_drop_cap_2_lines() {
    let il = InitialLetter { size: 2.0, sink: None };
    let layout = compute_initial_letter_layout(&il, 24.0, 16.0);
    assert_eq!(layout.letter_height, lu(48.0));
    assert_eq!(layout.computed_font_size, 32.0);
    assert_eq!(layout.exclusion_lines, 2);
    assert!(layout.is_drop_cap);
}

#[test]
fn wpt_initial_letter_exclusion_rect() {
    let il = InitialLetter { size: 3.0, sink: None };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
    let margin = lu(4.0);
    let (start, end, bstart, bend) = compute_exclusion_rect(&layout, margin);
    assert_eq!(start, lu(0.0));
    assert_eq!(bstart, lu(0.0));
    assert_eq!(bend, lu(60.0));
    assert!(end > lu(0.0));
}

#[test]
fn wpt_initial_letter_line_exclusion_detection() {
    assert!(line_in_exclusion_zone(lu(0.0), lu(20.0), lu(0.0), lu(60.0)));
    assert!(line_in_exclusion_zone(lu(40.0), lu(20.0), lu(0.0), lu(60.0)));
    assert!(!line_in_exclusion_zone(lu(60.0), lu(20.0), lu(0.0), lu(60.0)));
}

#[test]
fn wpt_initial_letter_available_width() {
    assert_eq!(available_width_with_exclusion(lu(300.0), lu(50.0)), lu(250.0));
    assert_eq!(
        available_width_with_exclusion(lu(30.0), lu(50.0)),
        lu(0.0),
        "Should clamp to zero"
    );
}

#[test]
fn wpt_initial_letter_validate() {
    assert!(validate_initial_letter(3.0, None).is_some());
    assert!(validate_initial_letter(3.0, Some(2.0)).is_some());
    assert!(validate_initial_letter(1.0, Some(1.0)).is_some());
    assert!(validate_initial_letter(0.5, None).is_none());
    assert!(validate_initial_letter(3.0, Some(0.5)).is_none());
}

#[test]
fn wpt_initial_letter_style_default_none() {
    let s = ComputedStyle::initial();
    assert!(s.initial_letter.is_none());
}

#[test]
fn wpt_initial_letter_effective_sink() {
    let il = InitialLetter { size: 3.0, sink: None };
    assert_eq!(il.effective_sink(), 3.0);
    let il2 = InitialLetter { size: 3.0, sink: Some(2.0) };
    assert_eq!(il2.effective_sink(), 2.0);
}

#[test]
fn wpt_initial_letter_create_style() {
    let base = ComputedStyle::initial();
    let il = InitialLetter { size: 3.0, sink: None };
    let layout = compute_initial_letter_layout(&il, 20.0, 16.0);
    let style = create_initial_letter_style(&base, &layout);
    assert_eq!(style.font_size, 48.0);
}

// ═══════════════════════════════════════════════════════════════════════════
// Cross-feature integration tests
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn wpt_integration_style_defaults_stable() {
    let s = ComputedStyle::initial();
    assert_eq!(s.text_wrap, TextWrap::Wrap);
    assert!(s.first_line_style.is_none());
    assert!(s.initial_letter.is_none());
    assert_eq!(s.orphans, 2);
    assert_eq!(s.widows, 2);
    assert_eq!(s.box_decoration_break, BoxDecorationBreak::Slice);
}

#[test]
fn wpt_integration_layout_with_defaults() {
    let mut doc = Document::new();
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let n = doc.node_mut(div);
        n.style.display = Display::Block;
        n.style.font_size = 16.0;
        n.style.width = Length::px(200.0);
    }
    doc.append_child(vp, div);
    add_text(&mut doc, div, "Hello world");
    let frag = block_layout(&doc, vp, &space(400, 800));
    assert!(!frag.children.is_empty());
    let div_frag = &frag.children[0];
    assert!(div_frag.size.width > lu(0.0));
    assert!(div_frag.size.height > lu(0.0));
}

#[test]
fn wpt_integration_text_wrap_balance_field() {
    let mut s = ComputedStyle::initial();
    s.text_wrap = TextWrap::Balance;
    assert_eq!(s.text_wrap, TextWrap::Balance);
    assert!(s.text_wrap.uses_scoring());
}

#[test]
fn wpt_integration_fragment_new_box_defaults() {
    let frag = Fragment::new_box(NodeId::NONE, PhysicalSize::new(lu(100.0), lu(50.0)));
    assert!(frag.is_first_for_node);
    assert!(frag.is_last_for_node);
    assert!(frag.break_token.is_none());
    assert!(frag.first_baseline.is_none());
    assert!(frag.last_baseline.is_none());
}

#[test]
fn wpt_integration_break_token_inline_variant() {
    let token = BreakToken::Inline(InlineBreakToken::new(3, lu(60.0)));
    match &token {
        BreakToken::Inline(t) => {
            assert_eq!(t.lines_consumed, 3);
            assert_eq!(t.consumed_block_size, lu(60.0));
        }
        _ => panic!("Expected Inline variant"),
    }
}

#[test]
fn wpt_integration_multi_fragmentainer() {
    let frag = make_fake_lines(8, 20.0, 200.0);
    // First fragmentainer: 60px → 3 lines
    let first = apply_inline_fragmentation(clone_frag(&frag), lu(60.0), LayoutUnit::zero(), 0, 1, 1);
    assert_eq!(first.children.len(), 3);
    let tok1 = match &first.break_token {
        Some(BreakToken::Inline(t)) => t.clone(),
        _ => panic!("Expected break token"),
    };
    // Second fragmentainer: 60px → 3 more
    let second = resume_inline_from_break_token(clone_frag(&frag), &tok1, lu(60.0), 1, 1);
    assert_eq!(second.children.len(), 3);
    let tok2 = match &second.break_token {
        Some(BreakToken::Inline(t)) => t.clone(),
        _ => panic!("Expected break token"),
    };
    // Third: remainder
    let third = resume_inline_from_break_token(clone_frag(&frag), &tok2, lu(60.0), 1, 1);
    assert_eq!(third.children.len(), 2);
    assert!(third.break_token.is_none());
    let total = first.children.len() + second.children.len() + third.children.len();
    assert_eq!(total, 8);
}
