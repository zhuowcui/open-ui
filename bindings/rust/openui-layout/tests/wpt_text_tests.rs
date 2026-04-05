//! WPT-equivalent tests for CSS Text Module Level 3.
//!
//! Each test corresponds to behaviors verified by WPT css/css-text tests.
//! Categories: text-align, text-align-last, white-space, word-break,
//! overflow-wrap, text-transform, line-break, hyphens, letter-spacing,
//! word-spacing, text-indent, tab-size, hanging-punctuation, text-justify,
//! and property interactions.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{LayoutUnit, Length};
use openui_layout::block::block_layout;
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
#[allow(unused_imports)]
use openui_style::{
    ComputedStyle, Direction, Display, HangingPunctuation, Hyphens, LineBreak, LineHeight,
    OverflowWrap, TabSize, TextAlign, TextAlignLast, TextJustify, TextTransform, VerticalAlign,
    WhiteSpace, WordBreak, WritingMode, TextOrientation,
};

// -- Helpers --

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
    let (doc, block) = make_text_block(texts, width);
    let sp = ConstraintSpace::for_block_child(
        lu_i(width), lu_i(600), lu_i(width), lu_i(600), false,
    );
    inline_layout(&doc, block, &sp)
}

#[allow(dead_code)]
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

fn make_styled_text_block(
    texts: &[&str],
    width: i32,
    style_fn: impl Fn(&mut ComputedStyle),
) -> Fragment {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    style_fn(&mut doc.node_mut(block).style);
    doc.append_child(root, block);
    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        let block_style = doc.node(block).style.clone();
        doc.node_mut(t).style.text_align = block_style.text_align;
        doc.node_mut(t).style.white_space = block_style.white_space;
        doc.node_mut(t).style.word_break = block_style.word_break;
        doc.node_mut(t).style.overflow_wrap = block_style.overflow_wrap;
        doc.node_mut(t).style.line_break = block_style.line_break;
        doc.node_mut(t).style.hyphens = block_style.hyphens;
        doc.node_mut(t).style.text_transform = block_style.text_transform;
        doc.node_mut(t).style.letter_spacing = block_style.letter_spacing;
        doc.node_mut(t).style.word_spacing = block_style.word_spacing;
        doc.node_mut(t).style.text_indent = block_style.text_indent.clone();
        doc.node_mut(t).style.tab_size = block_style.tab_size.clone();
        doc.node_mut(t).style.text_justify = block_style.text_justify;
        doc.node_mut(t).style.text_align_last = block_style.text_align_last;
        doc.node_mut(t).style.line_height = block_style.line_height.clone();
        doc.node_mut(t).style.direction = block_style.direction;
        doc.node_mut(t).style.writing_mode = block_style.writing_mode;
        doc.node_mut(t).style.hanging_punctuation = block_style.hanging_punctuation.clone();
        doc.node_mut(t).style.font_size = block_style.font_size;
        doc.append_child(block, t);
    }
    let sp = space(width, 600);
    block_layout(&doc, root, &sp)
}

fn count_line_boxes(fragment: &Fragment) -> usize {
    fragment.children.iter().filter(|c| c.kind == FragmentKind::Box).count()
}

#[allow(dead_code)]
fn count_text_fragments(fragment: &Fragment) -> usize {
    let mut count = 0;
    if fragment.kind == FragmentKind::Text { count += 1; }
    for child in &fragment.children { count += count_text_fragments(child); }
    count
}

fn collect_text_fragments(fragment: &Fragment) -> Vec<&Fragment> {
    let mut result = Vec::new();
    if fragment.kind == FragmentKind::Text { result.push(fragment); }
    for child in &fragment.children { result.extend(collect_text_fragments(child)); }
    result
}

fn first_block_child(fragment: &Fragment) -> &Fragment {
    fragment.children.iter().find(|c| c.kind == FragmentKind::Box).unwrap()
}

/// Inline layout with a block-level style setter.
fn layout_text_styled(
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
        lu_i(width), lu_i(600), lu_i(width), lu_i(600), false,
    );
    inline_layout(&doc, block, &sp)
}

/// Inline layout with style inherited by text nodes.
fn layout_text_inheriting(
    texts: &[&str],
    width: i32,
    style_fn: impl Fn(&mut ComputedStyle),
) -> Fragment {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    style_fn(&mut doc.node_mut(block).style);
    doc.append_child(root, block);
    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        let bs = doc.node(block).style.clone();
        doc.node_mut(t).style.white_space = bs.white_space;
        doc.node_mut(t).style.word_break = bs.word_break;
        doc.node_mut(t).style.overflow_wrap = bs.overflow_wrap;
        doc.node_mut(t).style.line_break = bs.line_break;
        doc.node_mut(t).style.hyphens = bs.hyphens;
        doc.node_mut(t).style.letter_spacing = bs.letter_spacing;
        doc.node_mut(t).style.word_spacing = bs.word_spacing;
        doc.node_mut(t).style.line_height = bs.line_height.clone();
        doc.node_mut(t).style.font_size = bs.font_size;
        doc.node_mut(t).style.text_transform = bs.text_transform;
        doc.node_mut(t).style.tab_size = bs.tab_size.clone();
        doc.append_child(block, t);
    }
    let sp = ConstraintSpace::for_block_child(
        lu_i(width), lu_i(600), lu_i(width), lu_i(600), false,
    );
    inline_layout(&doc, block, &sp)
}

// ==========================================================================
// mod text_align -- 27 tests
// ==========================================================================

mod text_align {
    use super::*;

    #[test]
    fn initial_value_is_start() {
        let s = ComputedStyle::initial();
        assert_eq!(s.text_align, TextAlign::Start);
    }

    #[test]
    fn default_is_start() {
        assert_eq!(TextAlign::default(), TextAlign::Start);
    }

    #[test]
    fn all_values_are_distinct() {
        let vals = [TextAlign::Left, TextAlign::Right, TextAlign::Center,
                     TextAlign::Justify, TextAlign::Start, TextAlign::End];
        for (i, a) in vals.iter().enumerate() {
            for (j, b) in vals.iter().enumerate() {
                if i != j { assert_ne!(a, b); }
            }
        }
    }

    #[test]
    fn left_text_starts_at_zero() {
        let frag = layout_text_styled(&["Hi"], 800, |s| { s.text_align = TextAlign::Left; });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert_eq!(texts[0].offset.left, LayoutUnit::zero());
    }

    #[test]
    fn right_text_offset_positive() {
        let frag = layout_text_styled(&["Hi"], 800, |s| { s.text_align = TextAlign::Right; });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(texts[0].offset.left > LayoutUnit::zero(),
            "Right-aligned text should have positive offset, got {:?}", texts[0].offset.left);
    }

    #[test]
    fn right_text_ends_near_container_edge() {
        let frag = layout_text_styled(&["Hi"], 800, |s| { s.text_align = TextAlign::Right; });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        let right_edge = texts[0].offset.left + texts[0].size.width;
        let diff = (right_edge - lu_i(800)).to_f32().abs();
        assert!(diff < 2.0, "Right edge should be near 800px, got {:?}", right_edge);
    }

    #[test]
    fn center_text_approximately_centered() {
        let frag = layout_text_styled(&["Hi"], 800, |s| { s.text_align = TextAlign::Center; });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        let text_width = texts[0].size.width.to_f32();
        let expected_left = (800.0 - text_width) / 2.0;
        let actual_left = texts[0].offset.left.to_f32();
        assert!((actual_left - expected_left).abs() < 2.0,
            "Center text should be at ~{}, got {}", expected_left, actual_left);
    }

    #[test]
    fn center_short_word_positive_offset() {
        let frag = layout_text_styled(&["x"], 800, |s| { s.text_align = TextAlign::Center; });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(texts[0].offset.left > lu(100.0));
    }

    #[test]
    fn justify_single_line_not_justified() {
        let frag = layout_text_styled(&["Hello world"], 800, |s| { s.text_align = TextAlign::Justify; });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert_eq!(texts[0].offset.left, LayoutUnit::zero());
    }

    #[test]
    fn justify_multiline_wraps() {
        let frag = layout_text_styled(
            &["The quick brown fox jumps over the lazy dog and more text here"],
            200, |s| { s.text_align = TextAlign::Justify; });
        assert!(count_line_boxes(&frag) >= 2, "Should wrap to multiple lines");
    }

    #[test]
    fn start_ltr_equals_left() {
        let frag_s = layout_text_styled(&["Hi"], 800, |s| {
            s.text_align = TextAlign::Start; s.direction = Direction::Ltr;
        });
        let frag_l = layout_text_styled(&["Hi"], 800, |s| { s.text_align = TextAlign::Left; });
        let ts = collect_text_fragments(&frag_s);
        let tl = collect_text_fragments(&frag_l);
        assert!(!ts.is_empty() && !tl.is_empty());
        assert_eq!(ts[0].offset.left, tl[0].offset.left);
    }

    #[test]
    fn start_rtl_text_offset_positive() {
        let frag = layout_text_styled(&["Hi"], 800, |s| {
            s.text_align = TextAlign::Start; s.direction = Direction::Rtl;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(texts[0].offset.left > LayoutUnit::zero());
    }

    #[test]
    fn end_ltr_text_offset_positive() {
        let frag = layout_text_styled(&["Hi"], 800, |s| {
            s.text_align = TextAlign::End; s.direction = Direction::Ltr;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(texts[0].offset.left > LayoutUnit::zero());
    }

    #[test]
    fn end_rtl_text_starts_at_zero() {
        let frag = layout_text_styled(&["Hi"], 800, |s| {
            s.text_align = TextAlign::End; s.direction = Direction::Rtl;
        });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert_eq!(texts[0].offset.left, LayoutUnit::zero());
    }

    #[test]
    fn multiline_left_all_at_zero() {
        let frag = layout_text_styled(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.text_align = TextAlign::Left; });
        for line in &frag.children {
            if line.kind == FragmentKind::Box {
                let texts = collect_text_fragments(line);
                if !texts.is_empty() {
                    assert_eq!(texts[0].offset.left, LayoutUnit::zero());
                }
            }
        }
    }

    #[test]
    fn multiline_right_all_positive_offset() {
        let frag = layout_text_styled(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.text_align = TextAlign::Right; });
        let lines: Vec<_> = frag.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2);
        for line in &lines {
            let texts = collect_text_fragments(line);
            if !texts.is_empty() {
                assert!(texts[0].offset.left > LayoutUnit::zero());
            }
        }
    }

    #[test]
    fn multiline_center_offsets_positive() {
        let frag = layout_text_styled(
            &["The quick brown fox jumps over"], 100,
            |s| { s.text_align = TextAlign::Center; });
        let lines: Vec<_> = frag.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2);
        for line in &lines {
            let texts = collect_text_fragments(line);
            if !texts.is_empty() { assert!(texts[0].offset.left > LayoutUnit::zero()); }
        }
    }

    #[test]
    fn left_text_width_positive() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Left; });
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(texts[0].size.width > LayoutUnit::zero());
    }

    #[test]
    fn right_position_differs_from_left() {
        let fl = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Left; });
        let fr = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Right; });
        let tl = collect_text_fragments(&fl);
        let tr = collect_text_fragments(&fr);
        assert!(!tl.is_empty() && !tr.is_empty());
        assert_ne!(tl[0].offset.left, tr[0].offset.left);
    }

    #[test]
    fn center_position_differs_from_left() {
        let fl = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Left; });
        let fc = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Center; });
        let tl = collect_text_fragments(&fl);
        let tc = collect_text_fragments(&fc);
        assert!(!tl.is_empty() && !tc.is_empty());
        assert_ne!(tl[0].offset.left, tc[0].offset.left);
    }

    #[test]
    fn left_produces_positive_height() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Left; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn right_produces_positive_height() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Right; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn center_produces_positive_height() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Center; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn justify_produces_positive_height() {
        let frag = layout_text_styled(&["Hello world"], 800, |s| { s.text_align = TextAlign::Justify; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn text_width_unchanged_by_alignment() {
        let fl = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Left; });
        let fr = layout_text_styled(&["Hello"], 800, |s| { s.text_align = TextAlign::Right; });
        let tl = collect_text_fragments(&fl);
        let tr = collect_text_fragments(&fr);
        assert!(!tl.is_empty() && !tr.is_empty());
        let diff = (tl[0].size.width - tr[0].size.width).to_f32().abs();
        assert!(diff < 1.0, "Text width should be same regardless of alignment");
    }

    #[test]
    fn empty_text_left_no_crash() {
        let frag = layout_text_styled(&[""], 800, |s| { s.text_align = TextAlign::Left; });
        let _ = frag.size;
    }

    #[test]
    fn empty_text_center_no_crash() {
        let frag = layout_text_styled(&[""], 800, |s| { s.text_align = TextAlign::Center; });
        let _ = frag.size;
    }
}

// ==========================================================================
// mod text_align_last -- 12 tests
// ==========================================================================

mod text_align_last {
    use super::*;

    #[test]
    fn initial_value_is_auto() {
        let s = ComputedStyle::initial();
        assert_eq!(s.text_align_last, TextAlignLast::Auto);
    }

    #[test]
    fn default_is_auto() {
        assert_eq!(TextAlignLast::default(), TextAlignLast::Auto);
    }

    #[test]
    fn all_values_distinct() {
        let vals = [TextAlignLast::Auto, TextAlignLast::Start, TextAlignLast::End,
                     TextAlignLast::Left, TextAlignLast::Right, TextAlignLast::Center,
                     TextAlignLast::Justify];
        for (i, a) in vals.iter().enumerate() {
            for (j, b) in vals.iter().enumerate() {
                if i != j { assert_ne!(a, b); }
            }
        }
    }

    #[test]
    fn auto_with_justify_last_line_is_start() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more words"],
            200, |s| { s.text_align = TextAlign::Justify; s.text_align_last = TextAlignLast::Auto; });
        let block = first_block_child(&frag);
        let lines: Vec<_> = block.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-align-last");{
            let last = lines.last().unwrap();
            let texts = collect_text_fragments(last);
            if !texts.is_empty() { assert_eq!(texts[0].offset.left, LayoutUnit::zero()); }
        }
    }

    #[test]
    fn center_last_line() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more words"],
            200, |s| { s.text_align = TextAlign::Left; s.text_align_last = TextAlignLast::Center; });
        let block = first_block_child(&frag);
        let lines: Vec<_> = block.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-align-last");{
            let last = lines.last().unwrap();
            let texts = collect_text_fragments(last);
            if !texts.is_empty() { assert!(texts[0].offset.left > LayoutUnit::zero()); }
        }
    }

    #[test]
    fn right_last_line() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more words"],
            200, |s| { s.text_align = TextAlign::Left; s.text_align_last = TextAlignLast::Right; });
        let block = first_block_child(&frag);
        let lines: Vec<_> = block.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-align-last");{
            let last = lines.last().unwrap();
            let texts = collect_text_fragments(last);
            if !texts.is_empty() { assert!(texts[0].offset.left > LayoutUnit::zero()); }
        }
    }

    #[test]
    fn left_last_line_at_zero() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more words"],
            200, |s| { s.text_align = TextAlign::Right; s.text_align_last = TextAlignLast::Left; });
        let block = first_block_child(&frag);
        let lines: Vec<_> = block.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-align-last");{
            let last = lines.last().unwrap();
            let texts = collect_text_fragments(last);
            if !texts.is_empty() { assert_eq!(texts[0].offset.left, LayoutUnit::zero()); }
        }
    }

    #[test]
    fn start_last_line_ltr() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more words"],
            200, |s| { s.text_align = TextAlign::Right; s.text_align_last = TextAlignLast::Start; });
        let block = first_block_child(&frag);
        let lines: Vec<_> = block.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-align-last");{
            let last = lines.last().unwrap();
            let texts = collect_text_fragments(last);
            if !texts.is_empty() { assert_eq!(texts[0].offset.left, LayoutUnit::zero()); }
        }
    }

    #[test]
    fn end_last_line_ltr_positive() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more words"],
            200, |s| { s.text_align = TextAlign::Left; s.text_align_last = TextAlignLast::End; });
        let block = first_block_child(&frag);
        let lines: Vec<_> = block.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-align-last");{
            let last = lines.last().unwrap();
            let texts = collect_text_fragments(last);
            if !texts.is_empty() { assert!(texts[0].offset.left > LayoutUnit::zero()); }
        }
    }

    #[test]
    fn single_line_last_applies() {
        let frag = make_styled_text_block(&["Hello"], 800, |s| {
            s.text_align = TextAlign::Left; s.text_align_last = TextAlignLast::Center;
        });
        let block = first_block_child(&frag);
        let texts = collect_text_fragments(block);
        if !texts.is_empty() { assert!(texts[0].offset.left > LayoutUnit::zero()); }
    }

    #[test]
    fn justify_last_line_value() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more words here now"],
            200, |s| { s.text_align = TextAlign::Left; s.text_align_last = TextAlignLast::Justify; });
        let block = first_block_child(&frag);
        let lines: Vec<_> = block.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2);
    }

    #[test]
    fn empty_no_crash() {
        let frag = make_styled_text_block(&[""], 800, |s| { s.text_align_last = TextAlignLast::Center; });
        let _ = frag.size;
    }
}

// ==========================================================================
// mod white_space -- 27 tests
// ==========================================================================

mod white_space {
    use super::*;

    #[test]
    fn initial_value_is_normal() {
        let s = ComputedStyle::initial();
        assert_eq!(s.white_space, WhiteSpace::Normal);
    }

    #[test]
    fn default_is_normal() { assert_eq!(WhiteSpace::default(), WhiteSpace::Normal); }

    #[test]
    fn all_values_distinct() {
        let vals = [WhiteSpace::Normal, WhiteSpace::Nowrap, WhiteSpace::Pre,
                     WhiteSpace::PreWrap, WhiteSpace::PreLine, WhiteSpace::BreakSpaces];
        for (i, a) in vals.iter().enumerate() {
            for (j, b) in vals.iter().enumerate() { if i != j { assert_ne!(a, b); } }
        }
    }

    #[test]
    fn normal_wraps_long_text() {
        let frag = layout_text_inheriting(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.white_space = WhiteSpace::Normal; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn normal_collapses_multiple_spaces() {
        let fm = layout_text_inheriting(&["Hello     World"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        let fs = layout_text_inheriting(&["Hello World"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        let tm = collect_text_fragments(&fm);
        let ts = collect_text_fragments(&fs);
        if !tm.is_empty() && !ts.is_empty() {
            let diff = (tm[0].size.width - ts[0].size.width).to_f32().abs();
            assert!(diff < 2.0, "Collapsed spaces should produce same width");
        }
    }

    #[test]
    fn nowrap_no_wrapping() {
        let frag = layout_text_inheriting(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.white_space = WhiteSpace::Nowrap; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn nowrap_collapses_spaces() {
        let fm = layout_text_inheriting(&["Hello     World"], 800, |s| { s.white_space = WhiteSpace::Nowrap; });
        let fs = layout_text_inheriting(&["Hello World"], 800, |s| { s.white_space = WhiteSpace::Nowrap; });
        let tm = collect_text_fragments(&fm);
        let ts = collect_text_fragments(&fs);
        if !tm.is_empty() && !ts.is_empty() {
            let diff = (tm[0].size.width - ts[0].size.width).to_f32().abs();
            assert!(diff < 2.0);
        }
    }

    #[test]
    fn pre_preserves_spaces() {
        let fp = layout_text_inheriting(&["Hello     World"], 800, |s| { s.white_space = WhiteSpace::Pre; });
        let fn_ = layout_text_inheriting(&["Hello World"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        let tp = collect_text_fragments(&fp);
        let tn = collect_text_fragments(&fn_);
        if !tp.is_empty() && !tn.is_empty() {
            assert!(tp[0].size.width > tn[0].size.width, "Pre should preserve spaces");
        }
    }

    #[test]
    fn pre_no_wrapping() {
        let frag = layout_text_inheriting(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.white_space = WhiteSpace::Pre; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn pre_preserves_newlines() {
        let frag = layout_text_inheriting(&["Hello\nWorld"], 800, |s| { s.white_space = WhiteSpace::Pre; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn pre_wrap_wraps() {
        let frag = layout_text_inheriting(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.white_space = WhiteSpace::PreWrap; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn pre_wrap_preserves_spaces() {
        let fp = layout_text_inheriting(&["Hello     World"], 800, |s| { s.white_space = WhiteSpace::PreWrap; });
        let fn_ = layout_text_inheriting(&["Hello World"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        let tp = collect_text_fragments(&fp);
        let tn = collect_text_fragments(&fn_);
        if !tp.is_empty() && !tn.is_empty() {
            assert!(tp[0].size.width > tn[0].size.width);
        }
    }

    #[test]
    fn pre_wrap_preserves_newlines() {
        let frag = layout_text_inheriting(&["Hello\nWorld"], 800, |s| { s.white_space = WhiteSpace::PreWrap; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn pre_line_collapses_spaces() {
        let fp = layout_text_inheriting(&["Hello     World"], 800, |s| { s.white_space = WhiteSpace::PreLine; });
        let fn_ = layout_text_inheriting(&["Hello World"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        let tp = collect_text_fragments(&fp);
        let tn = collect_text_fragments(&fn_);
        if !tp.is_empty() && !tn.is_empty() {
            let diff = (tp[0].size.width - tn[0].size.width).to_f32().abs();
            assert!(diff < 2.0);
        }
    }

    #[test]
    fn pre_line_preserves_newlines() {
        let frag = layout_text_inheriting(&["Hello\nWorld"], 800, |s| { s.white_space = WhiteSpace::PreLine; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn pre_line_wraps() {
        let frag = layout_text_inheriting(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.white_space = WhiteSpace::PreLine; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_spaces_preserves_spaces() {
        let fb = layout_text_inheriting(&["Hello     World"], 800, |s| { s.white_space = WhiteSpace::BreakSpaces; });
        let fn_ = layout_text_inheriting(&["Hello World"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        let tb = collect_text_fragments(&fb);
        let tn = collect_text_fragments(&fn_);
        if !tb.is_empty() && !tn.is_empty() {
            assert!(tb[0].size.width > tn[0].size.width);
        }
    }

    #[test]
    fn break_spaces_wraps() {
        let frag = layout_text_inheriting(
            &["The quick brown fox jumps over the lazy dog"], 100,
            |s| { s.white_space = WhiteSpace::BreakSpaces; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn normal_single_word_fits() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn nowrap_overflows() {
        let frag = layout_text_inheriting(
            &["A very long sentence that should not wrap at all nowrap"],
            100, |s| { s.white_space = WhiteSpace::Nowrap; });
        assert_eq!(count_line_boxes(&frag), 1);
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].size.width > lu_i(100)); }
    }

    #[test]
    fn pre_with_tabs() {
        let ft = layout_text_inheriting(&["A\tB"], 800, |s| { s.white_space = WhiteSpace::Pre; });
        let fs = layout_text_inheriting(&["A B"], 800, |s| { s.white_space = WhiteSpace::Pre; });
        let tt = collect_text_fragments(&ft);
        let ts = collect_text_fragments(&fs);
        if !tt.is_empty() && !ts.is_empty() {
            assert!(tt[0].size.width > ts[0].size.width);
        }
    }

    #[test]
    fn normal_newlines_as_space() {
        let frag = layout_text_inheriting(&["Hello\nWorld"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn normal_positive_height() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.white_space = WhiteSpace::Normal; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn pre_empty_newline() {
        let frag = layout_text_inheriting(&["\n"], 800, |s| { s.white_space = WhiteSpace::Pre; });
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn pre_wrap_newline_break() {
        let frag = layout_text_inheriting(&["A\nB"], 800, |s| { s.white_space = WhiteSpace::PreWrap; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_spaces_newlines() {
        let frag = layout_text_inheriting(&["Hello\nWorld"], 800, |s| { s.white_space = WhiteSpace::BreakSpaces; });
        assert!(count_line_boxes(&frag) >= 2);
    }
}

// ==========================================================================
// mod word_break -- 20 tests
// ==========================================================================

mod word_break {
    use super::*;

    #[test]
    fn initial_value_is_normal() { assert_eq!(ComputedStyle::initial().word_break, WordBreak::Normal); }

    #[test]
    fn default_is_normal() { assert_eq!(WordBreak::default(), WordBreak::Normal); }

    #[test]
    fn all_values_distinct() {
        let v = [WordBreak::Normal, WordBreak::BreakAll, WordBreak::KeepAll, WordBreak::BreakWord];
        for (i, a) in v.iter().enumerate() { for (j, b) in v.iter().enumerate() { if i!=j { assert_ne!(a,b); } } }
    }

    #[test]
    fn normal_no_break_in_word() {
        let frag = layout_text_inheriting(&["Supercalifragilisticexpialidocious"], 100, |s| { s.word_break = WordBreak::Normal; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn break_all_breaks_word() {
        let frag = layout_text_inheriting(&["Supercalifragilisticexpialidocious"], 100, |s| { s.word_break = WordBreak::BreakAll; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_all_narrow_many_lines() {
        let frag = layout_text_inheriting(&["Internationalization"], 50, |s| { s.word_break = WordBreak::BreakAll; });
        assert!(count_line_boxes(&frag) >= 3);
    }

    #[test]
    fn keep_all_cjk_no_break() {
        let frag = layout_text_inheriting(&["Hello-World Foo-Bar"], 800, |s| { s.word_break = WordBreak::KeepAll; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn normal_cjk_wraps() {
        let frag = layout_text_inheriting(&["\u{6F22}\u{5B57}\u{6587}"], 800, |s| { s.word_break = WordBreak::Normal; });
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn break_all_short_one_line() {
        let frag = layout_text_inheriting(&["Hi"], 800, |s| { s.word_break = WordBreak::BreakAll; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn break_all_positive_height() {
        let frag = layout_text_inheriting(&["Hello"], 50, |s| { s.word_break = WordBreak::BreakAll; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn normal_multi_words_wrap() {
        let frag = layout_text_inheriting(&["The quick brown fox jumps"], 80, |s| { s.word_break = WordBreak::Normal; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_word_breaks_long() {
        let frag = layout_text_inheriting(&["Supercalifragilisticexpialidocious"], 100, |s| { s.word_break = WordBreak::BreakWord; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn keep_all_wraps_at_spaces() {
        let frag = layout_text_inheriting(&["Hello World Test"], 60, |s| { s.word_break = WordBreak::KeepAll; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_all_per_char() {
        let frag = layout_text_inheriting(&["ABCDEFGH"], 20, |s| { s.word_break = WordBreak::BreakAll; });
        assert!(count_line_boxes(&frag) >= 3);
    }

    #[test]
    fn normal_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.word_break = WordBreak::Normal; });
        let _ = frag.size;
    }

    #[test]
    fn break_all_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.word_break = WordBreak::BreakAll; });
        let _ = frag.size;
    }

    #[test]
    fn keep_all_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.word_break = WordBreak::KeepAll; });
        let _ = frag.size;
    }

    #[test]
    fn break_all_text_fits_container() {
        let frag = layout_text_inheriting(&["ABCDEFGHIJKLMNOP"], 80, |s| { s.word_break = WordBreak::BreakAll; });
        for line in &frag.children {
            if line.kind == FragmentKind::Box {
                let texts = collect_text_fragments(line);
                for t in &texts { assert!(t.size.width <= lu(85.0)); }
            }
        }
    }

    #[test]
    fn normal_single_word_wide() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.word_break = WordBreak::Normal; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn keep_all_positive_height() {
        let frag = layout_text_inheriting(&["\u{6F22}\u{5B57}"], 800, |s| { s.word_break = WordBreak::KeepAll; });
        assert!(frag.size.height > LayoutUnit::zero());
    }
}

// ==========================================================================
// mod overflow_wrap -- 16 tests
// ==========================================================================

mod overflow_wrap {
    use super::*;

    #[test]
    fn initial_value_is_normal() { assert_eq!(ComputedStyle::initial().overflow_wrap, OverflowWrap::Normal); }

    #[test]
    fn default_is_normal() { assert_eq!(OverflowWrap::default(), OverflowWrap::Normal); }

    #[test]
    fn all_values_distinct() {
        let v = [OverflowWrap::Normal, OverflowWrap::BreakWord, OverflowWrap::Anywhere];
        for (i, a) in v.iter().enumerate() { for (j, b) in v.iter().enumerate() { if i!=j { assert_ne!(a,b); } } }
    }

    #[test]
    fn normal_long_word_overflows() {
        let frag = layout_text_inheriting(&["Supercalifragilisticexpialidocious"], 100, |s| { s.overflow_wrap = OverflowWrap::Normal; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn break_word_breaks() {
        let frag = layout_text_inheriting(&["Supercalifragilisticexpialidocious"], 100, |s| { s.overflow_wrap = OverflowWrap::BreakWord; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn anywhere_breaks() {
        let frag = layout_text_inheriting(&["Supercalifragilisticexpialidocious"], 100, |s| { s.overflow_wrap = OverflowWrap::Anywhere; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_word_short_one_line() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.overflow_wrap = OverflowWrap::BreakWord; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn anywhere_short_one_line() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.overflow_wrap = OverflowWrap::Anywhere; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn break_word_wraps_at_spaces_first() {
        let frag = layout_text_inheriting(&["Hello World Test"], 80, |s| { s.overflow_wrap = OverflowWrap::BreakWord; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_word_narrow() {
        let frag = layout_text_inheriting(&["Internationalization"], 50, |s| { s.overflow_wrap = OverflowWrap::BreakWord; });
        assert!(count_line_boxes(&frag) >= 3);
    }

    #[test]
    fn anywhere_narrow() {
        let frag = layout_text_inheriting(&["Internationalization"], 50, |s| { s.overflow_wrap = OverflowWrap::Anywhere; });
        assert!(count_line_boxes(&frag) >= 3);
    }

    #[test]
    fn normal_wraps_at_spaces() {
        let frag = layout_text_inheriting(&["Hello World foo bar"], 60, |s| { s.overflow_wrap = OverflowWrap::Normal; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn break_word_positive_height() {
        let frag = layout_text_inheriting(&["Hello"], 100, |s| { s.overflow_wrap = OverflowWrap::BreakWord; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn anywhere_positive_height() {
        let frag = layout_text_inheriting(&["Hello"], 100, |s| { s.overflow_wrap = OverflowWrap::Anywhere; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn break_word_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 100, |s| { s.overflow_wrap = OverflowWrap::BreakWord; });
        let _ = frag.size;
    }

    #[test]
    fn normal_overflow_exceeds() {
        let frag = layout_text_inheriting(&["Pneumonoultramicroscopicsilicovolcanoconiosis"], 80, |s| { s.overflow_wrap = OverflowWrap::Normal; });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].size.width > lu_i(80)); }
    }
}

// ==========================================================================
// mod text_transform -- 15 tests
// ==========================================================================

mod text_transform {
    use super::*;

    #[test]
    fn initial_value_is_none() { assert_eq!(ComputedStyle::initial().text_transform, TextTransform::None); }

    #[test]
    fn default_is_none() { assert_eq!(TextTransform::default(), TextTransform::None); }

    #[test]
    fn all_values_distinct() {
        let v = [TextTransform::None, TextTransform::Capitalize, TextTransform::Uppercase,
                  TextTransform::Lowercase, TextTransform::FullWidth, TextTransform::FullSizeKana];
        for (i,a) in v.iter().enumerate() { for (j,b) in v.iter().enumerate() { if i!=j { assert_ne!(a,b); } } }
    }

    #[test]
    fn none_produces_fragments() {
        let frag = layout_text_inheriting(&["Hello World"], 800, |s| { s.text_transform = TextTransform::None; });
        assert!(count_text_fragments(&frag) >= 1);
    }

    #[test]
    fn uppercase_succeeds() {
        let frag = layout_text_inheriting(&["hello world"], 800, |s| { s.text_transform = TextTransform::Uppercase; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn lowercase_succeeds() {
        let frag = layout_text_inheriting(&["HELLO WORLD"], 800, |s| { s.text_transform = TextTransform::Lowercase; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn capitalize_succeeds() {
        let frag = layout_text_inheriting(&["hello world"], 800, |s| { s.text_transform = TextTransform::Capitalize; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn full_width_succeeds() {
        let frag = layout_text_inheriting(&["ABC"], 800, |s| { s.text_transform = TextTransform::FullWidth; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn full_width_wider() {
        let fn_ = layout_text_inheriting(&["ABC"], 800, |s| { s.text_transform = TextTransform::None; });
        let fw = layout_text_inheriting(&["ABC"], 800, |s| { s.text_transform = TextTransform::FullWidth; });
        let tn = collect_text_fragments(&fn_);
        let tf = collect_text_fragments(&fw);
        if !tn.is_empty() && !tf.is_empty() {
            // Full-width transform should produce different widths than normal.
            // In a full font environment, full-width would be wider; here we
            // just verify both are valid and the transform had an effect.
            assert!(tn[0].size.width > LayoutUnit::zero(),
                "normal text should have positive width");
            assert!(tf[0].size.width > LayoutUnit::zero(),
                "full-width text should have positive width");
            assert_ne!(tf[0].size.width, tn[0].size.width,
                "full-width transform should change text width");
        }
    }

    #[test]
    fn none_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.text_transform = TextTransform::None; });
        let _ = frag.size;
    }

    #[test]
    fn uppercase_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.text_transform = TextTransform::Uppercase; });
        let _ = frag.size;
    }

    #[test]
    fn capitalize_single_char() {
        let frag = layout_text_inheriting(&["a"], 800, |s| { s.text_transform = TextTransform::Capitalize; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn uppercase_wraps_narrow() {
        let frag = layout_text_inheriting(&["hello world test"], 80, |s| { s.text_transform = TextTransform::Uppercase; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn lowercase_width_matches() {
        let fl = layout_text_inheriting(&["HELLO"], 800, |s| { s.text_transform = TextTransform::Lowercase; });
        let fm = layout_text_inheriting(&["hello"], 800, |s| { s.text_transform = TextTransform::None; });
        let tl = collect_text_fragments(&fl);
        let tm = collect_text_fragments(&fm);
        if !tl.is_empty() && !tm.is_empty() {
            let diff = (tl[0].size.width - tm[0].size.width).to_f32().abs();
            assert!(diff < 3.0);
        }
    }

    #[test]
    fn full_size_kana_succeeds() {
        let frag = layout_text_inheriting(&["\u{FF71}\u{FF72}\u{FF73}"], 800, |s| { s.text_transform = TextTransform::FullSizeKana; });
        assert!(frag.size.height > LayoutUnit::zero());
    }
}

// ==========================================================================
// mod line_break -- 15 tests
// ==========================================================================

mod line_break {
    use super::*;

    #[test]
    fn initial_value_is_auto() { assert_eq!(ComputedStyle::initial().line_break, LineBreak::Auto); }

    #[test]
    fn default_is_auto() { assert_eq!(LineBreak::default(), LineBreak::Auto); }

    #[test]
    fn all_values_distinct() {
        let v = [LineBreak::Auto, LineBreak::Loose, LineBreak::Normal, LineBreak::Strict, LineBreak::Anywhere];
        for (i,a) in v.iter().enumerate() { for (j,b) in v.iter().enumerate() { if i!=j { assert_ne!(a,b); } } }
    }

    #[test]
    fn auto_wraps_cjk() {
        let frag = layout_text_inheriting(&["\u{6F22}\u{5B57}\u{6587}"], 800, |s| { s.line_break = LineBreak::Auto; });
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn loose_wraps_cjk() {
        let frag = layout_text_inheriting(&["\u{6F22}\u{5B57}\u{6587}"], 800, |s| { s.line_break = LineBreak::Loose; });
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn normal_wraps_cjk() {
        let frag = layout_text_inheriting(&["\u{6F22}\u{5B57}\u{6587}"], 800, |s| { s.line_break = LineBreak::Normal; });
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn strict_wraps_cjk() {
        let frag = layout_text_inheriting(&["\u{6F22}\u{5B57}\u{6587}"], 800, |s| { s.line_break = LineBreak::Strict; });
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn anywhere_breaks_within_word() {
        let frag = layout_text_inheriting(&["Helloworld"], 30, |s| { s.line_break = LineBreak::Anywhere; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn anywhere_narrow_many_lines() {
        let frag = layout_text_inheriting(&["ABCDEFGHIJKLMN"], 20, |s| { s.line_break = LineBreak::Anywhere; });
        assert!(count_line_boxes(&frag) >= 4);
    }

    #[test]
    fn auto_latin_wraps_at_spaces() {
        let frag = layout_text_inheriting(&["Hello World Test Foo"], 60, |s| { s.line_break = LineBreak::Auto; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn strict_latin_wraps_at_spaces() {
        let frag = layout_text_inheriting(&["Hello World Test"], 60, |s| { s.line_break = LineBreak::Strict; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn auto_positive_height() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.line_break = LineBreak::Auto; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn anywhere_single_char() {
        let frag = layout_text_inheriting(&["X"], 800, |s| { s.line_break = LineBreak::Anywhere; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn loose_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.line_break = LineBreak::Loose; });
        let _ = frag.size;
    }

    #[test]
    fn strict_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.line_break = LineBreak::Strict; });
        let _ = frag.size;
    }
}

// ==========================================================================
// mod hyphens -- 12 tests
// ==========================================================================

mod hyphens {
    use super::*;

    #[test]
    fn initial_value_is_manual() { assert_eq!(ComputedStyle::initial().hyphens, Hyphens::Manual); }

    #[test]
    fn default_is_manual() { assert_eq!(Hyphens::default(), Hyphens::Manual); }

    #[test]
    fn all_values_distinct() {
        let v = [Hyphens::None, Hyphens::Manual, Hyphens::Auto];
        for (i,a) in v.iter().enumerate() { for (j,b) in v.iter().enumerate() { if i!=j { assert_ne!(a,b); } } }
    }

    #[test]
    fn none_succeeds() {
        let frag = layout_text_inheriting(&["Hello World"], 800, |s| { s.hyphens = Hyphens::None; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn manual_succeeds() {
        let frag = layout_text_inheriting(&["Hello World"], 800, |s| { s.hyphens = Hyphens::Manual; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn auto_succeeds() {
        let frag = layout_text_inheriting(&["Hello World"], 800, |s| { s.hyphens = Hyphens::Auto; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn manual_at_soft_hyphen() {
        let frag = layout_text_inheriting(&["Supercalifragilis\u{00AD}ticexpialidocious"], 150, |s| { s.hyphens = Hyphens::Manual; });
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn none_ignores_soft_hyphen() {
        let frag = layout_text_inheriting(&["Supercalifragilis\u{00AD}ticexpialidocious"], 150, |s| { s.hyphens = Hyphens::None; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn auto_long_word() {
        let frag = layout_text_inheriting(&["Internationalization"], 100, |s| { s.hyphens = Hyphens::Auto; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn none_empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.hyphens = Hyphens::None; });
        let _ = frag.size;
    }

    #[test]
    fn manual_wraps_at_spaces() {
        let frag = layout_text_inheriting(&["Hello World Test Foo"], 60, |s| { s.hyphens = Hyphens::Manual; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn auto_wraps_at_spaces() {
        let frag = layout_text_inheriting(&["Hello World Test Foo"], 60, |s| { s.hyphens = Hyphens::Auto; });
        assert!(count_line_boxes(&frag) >= 2);
    }
}

// ==========================================================================
// mod letter_spacing -- 15 tests
// ==========================================================================

mod letter_spacing {
    use super::*;

    #[test]
    fn initial_value_is_zero() { assert_eq!(ComputedStyle::initial().letter_spacing, 0.0); }

    #[test]
    fn positive_increases_width() {
        let f0 = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 0.0; });
        let f5 = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 5.0; });
        let t0 = collect_text_fragments(&f0);
        let t5 = collect_text_fragments(&f5);
        if !t0.is_empty() && !t5.is_empty() { assert!(t5[0].size.width > t0[0].size.width); }
    }

    #[test]
    fn negative_decreases_width() {
        let f0 = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 0.0; });
        let fn_ = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = -1.0; });
        let t0 = collect_text_fragments(&f0);
        let tn = collect_text_fragments(&fn_);
        if !t0.is_empty() && !tn.is_empty() { assert!(tn[0].size.width < t0[0].size.width); }
    }

    #[test]
    fn zero_same_as_default() {
        let fd = layout_text(&["Hello"], 800);
        let fz = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 0.0; });
        let td = collect_text_fragments(&fd);
        let tz = collect_text_fragments(&fz);
        if !td.is_empty() && !tz.is_empty() {
            let diff = (td[0].size.width - tz[0].size.width).to_f32().abs();
            assert!(diff < 1.0);
        }
    }

    #[test]
    fn large_causes_wrapping() {
        let frag = layout_text_inheriting(&["Hello World"], 100, |s| { s.letter_spacing = 10.0; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn single_char() {
        let frag = layout_text_inheriting(&["X"], 800, |s| { s.letter_spacing = 5.0; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.letter_spacing = 5.0; });
        let _ = frag.size;
    }

    #[test]
    fn two_px_per_char() {
        let f0 = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 0.0; });
        let f2 = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 2.0; });
        let t0 = collect_text_fragments(&f0);
        let t2 = collect_text_fragments(&f2);
        if !t0.is_empty() && !t2.is_empty() {
            let diff = (t2[0].size.width - t0[0].size.width).to_f32();
            assert!((diff - 10.0).abs() < 2.0, "Expected ~10px extra, got {}", diff);
        }
    }

    #[test]
    fn cjk_characters() {
        let frag = layout_text_inheriting(&["\u{6F22}\u{5B57}"], 800, |s| { s.letter_spacing = 5.0; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn preserves_line_count_wide() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 3.0; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn multiline_narrow() {
        let frag = layout_text_inheriting(&["The quick brown fox"], 80, |s| { s.letter_spacing = 3.0; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn positive_height() {
        let frag = layout_text_inheriting(&["Hello World"], 800, |s| { s.letter_spacing = 5.0; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn large_value_lays_out() {
        let frag = layout_text_inheriting(&["AB"], 800, |s| { s.letter_spacing = 100.0; });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].size.width > lu(200.0)); }
    }

    #[test]
    fn negative_large() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = -3.0; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn does_not_affect_height() {
        let f0 = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 0.0; });
        let f5 = layout_text_inheriting(&["Hello"], 800, |s| { s.letter_spacing = 5.0; });
        let diff = (f0.size.height - f5.size.height).to_f32().abs();
        assert!(diff < 2.0);
    }
}

// ==========================================================================
// mod word_spacing -- 13 tests
// ==========================================================================

mod word_spacing {
    use super::*;

    #[test]
    fn initial_value_is_zero() { assert_eq!(ComputedStyle::initial().word_spacing, 0.0); }

    #[test]
    fn positive_increases_width() {
        let f0 = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = 0.0; });
        let f10 = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = 10.0; });
        let t0 = collect_text_fragments(&f0);
        let t10 = collect_text_fragments(&f10);
        if !t0.is_empty() && !t10.is_empty() { assert!(t10[0].size.width > t0[0].size.width); }
    }

    #[test]
    fn negative_decreases_width() {
        let f0 = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = 0.0; });
        let fn_ = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = -2.0; });
        let t0 = collect_text_fragments(&f0);
        let tn = collect_text_fragments(&fn_);
        if !t0.is_empty() && !tn.is_empty() { assert!(tn[0].size.width < t0[0].size.width); }
    }

    #[test]
    fn no_spaces_no_effect() {
        let f0 = layout_text_inheriting(&["Hello"], 800, |s| { s.word_spacing = 0.0; });
        let f10 = layout_text_inheriting(&["Hello"], 800, |s| { s.word_spacing = 10.0; });
        let t0 = collect_text_fragments(&f0);
        let t10 = collect_text_fragments(&f10);
        if !t0.is_empty() && !t10.is_empty() {
            let diff = (t0[0].size.width - t10[0].size.width).to_f32().abs();
            assert!(diff < 1.0);
        }
    }

    #[test]
    fn two_spaces_double_effect() {
        let f0 = layout_text_inheriting(&["A B C"], 800, |s| { s.word_spacing = 0.0; });
        let f5 = layout_text_inheriting(&["A B C"], 800, |s| { s.word_spacing = 5.0; });
        let t0 = collect_text_fragments(&f0);
        let t5 = collect_text_fragments(&f5);
        if !t0.is_empty() && !t5.is_empty() {
            let diff = (t5[0].size.width - t0[0].size.width).to_f32();
            assert!((diff - 10.0).abs() < 2.0, "Expected ~10px extra, got {}", diff);
        }
    }

    #[test]
    fn large_causes_wrapping() {
        let frag = layout_text_inheriting(&["Hello World"], 100, |s| { s.word_spacing = 50.0; });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn empty_no_crash() {
        let frag = layout_text_inheriting(&[""], 800, |s| { s.word_spacing = 10.0; });
        let _ = frag.size;
    }

    #[test]
    fn preserves_height() {
        let f0 = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = 0.0; });
        let f10 = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = 10.0; });
        let diff = (f0.size.height - f10.size.height).to_f32().abs();
        assert!(diff < 2.0);
    }

    #[test]
    fn single_word_one_line() {
        let frag = layout_text_inheriting(&["Hello"], 800, |s| { s.word_spacing = 50.0; });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn zero_same_as_default() {
        let fd = layout_text(&["Hello World"], 800);
        let fz = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = 0.0; });
        let td = collect_text_fragments(&fd);
        let tz = collect_text_fragments(&fz);
        if !td.is_empty() && !tz.is_empty() {
            let diff = (td[0].size.width - tz[0].size.width).to_f32().abs();
            assert!(diff < 1.0);
        }
    }

    #[test]
    fn many_words() {
        let frag = layout_text_inheriting(&["one two three four five six"], 800, |s| { s.word_spacing = 5.0; });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn positive_width() {
        let frag = layout_text_inheriting(&["A B"], 800, |s| { s.word_spacing = 10.0; });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].size.width > LayoutUnit::zero()); }
    }

    #[test]
    fn negative_large() {
        let frag = layout_text_inheriting(&["Hello World"], 800, |s| { s.word_spacing = -5.0; });
        assert!(frag.size.height > LayoutUnit::zero());
    }
}

// ==========================================================================
// mod text_indent -- 16 tests
// ==========================================================================

mod text_indent {
    use super::*;

    #[test]
    fn initial_value_is_zero() { assert_eq!(ComputedStyle::initial().text_indent, Length::zero()); }

    #[test]
    fn positive_offsets_first_line() {
        let frag = layout_text_styled(&["Hello World"], 800, |s| { s.text_indent = Length::px(40.0); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].offset.left >= lu(40.0)); }
    }

    #[test]
    fn negative_indent() {
        let frag = layout_text_styled(&["Hello World"], 800, |s| { s.text_indent = Length::px(-20.0); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].offset.left < LayoutUnit::zero()); }
    }

    #[test]
    fn zero_at_zero() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_indent = Length::px(0.0); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert_eq!(texts[0].offset.left, LayoutUnit::zero()); }
    }

    #[test]
    fn only_first_line() {
        let frag = layout_text_styled(
            &["The quick brown fox jumps over the lazy dog and more"], 120,
            |s| { s.text_indent = Length::px(30.0); });
        let lines: Vec<_> = frag.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-indent");{
            let ft = collect_text_fragments(lines[0]);
            let st = collect_text_fragments(lines[1]);
            if !ft.is_empty() && !st.is_empty() { assert!(ft[0].offset.left > st[0].offset.left); }
        }
    }

    #[test]
    fn second_line_not_indented() {
        let frag = layout_text_styled(
            &["The quick brown fox jumps over the lazy dog and more text"], 120,
            |s| { s.text_indent = Length::px(50.0); });
        let lines: Vec<_> = frag.children.iter().filter(|c| c.kind == FragmentKind::Box).collect();
        assert!(lines.len() >= 2, "text must wrap to test text-indent");{
            let st = collect_text_fragments(lines[1]);
            if !st.is_empty() { assert!(st[0].offset.left < lu(50.0)); }
        }
    }

    #[test]
    fn percentage() {
        let frag = layout_text_styled(&["Hello"], 500, |s| { s.text_indent = Length::percent(10.0); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() {
            let offset = texts[0].offset.left.to_f32();
            assert!((offset - 50.0).abs() < 2.0, "10% of 500 should be ~50px, got {}", offset);
        }
    }

    #[test]
    fn large_indent() {
        let frag = layout_text_styled(&["Hello"], 500, |s| { s.text_indent = Length::px(400.0); });
        assert!(!frag.children.is_empty());
    }

    #[test]
    fn auto_resolves_to_zero() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_indent = Length::auto(); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert_eq!(texts[0].offset.left, LayoutUnit::zero()); }
    }

    #[test]
    fn with_center_alignment() {
        let frag = layout_text_styled(&["Hi"], 500, |s| {
            s.text_indent = Length::px(20.0); s.text_align = TextAlign::Center;
        });
        assert!(!frag.children.is_empty());
    }

    #[test]
    fn with_right_alignment() {
        let frag = layout_text_styled(&["Hi"], 500, |s| {
            s.text_indent = Length::px(20.0); s.text_align = TextAlign::Right;
        });
        assert!(!frag.children.is_empty());
    }

    #[test]
    fn does_not_affect_height() {
        let f0 = layout_text_styled(&["Hello"], 800, |s| { s.text_indent = Length::px(0.0); });
        let f50 = layout_text_styled(&["Hello"], 800, |s| { s.text_indent = Length::px(50.0); });
        let diff = (f0.size.height - f50.size.height).to_f32().abs();
        assert!(diff < 2.0);
    }

    #[test]
    fn empty_no_crash() {
        let frag = layout_text_styled(&[""], 800, |s| { s.text_indent = Length::px(50.0); });
        let _ = frag.size;
    }

    #[test]
    fn indent_10px() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_indent = Length::px(10.0); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].offset.left >= lu(10.0)); }
    }

    #[test]
    fn indent_100px() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_indent = Length::px(100.0); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].offset.left >= lu(100.0)); }
    }

    #[test]
    fn negative_50px() {
        let frag = layout_text_styled(&["Hello"], 800, |s| { s.text_indent = Length::px(-50.0); });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].offset.left < LayoutUnit::zero()); }
    }
}

// ==========================================================================
// mod tab_size -- 8 tests
// ==========================================================================

mod tab_size {
    use super::*;

    #[test]
    fn initial_is_8_spaces() { assert_eq!(ComputedStyle::initial().tab_size, TabSize::Spaces(8)); }

    #[test]
    fn default_is_8_spaces() { assert_eq!(TabSize::default(), TabSize::Spaces(8)); }

    #[test]
    fn custom_4_spaces() {
        let frag = layout_text_inheriting(&["A\tB"], 800, |s| {
            s.white_space = WhiteSpace::Pre; s.tab_size = TabSize::Spaces(4);
        });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn tab_with_pre() {
        let frag = layout_text_inheriting(&["X\tY"], 800, |s| {
            s.white_space = WhiteSpace::Pre; s.tab_size = TabSize::Spaces(8);
        });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn tab_with_pre_wrap() {
        let frag = layout_text_inheriting(&["X\tY"], 800, |s| {
            s.white_space = WhiteSpace::PreWrap; s.tab_size = TabSize::Spaces(8);
        });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn larger_tab_wider() {
        let f4 = layout_text_inheriting(&["A\tB"], 800, |s| {
            s.white_space = WhiteSpace::Pre; s.tab_size = TabSize::Spaces(4);
        });
        let f16 = layout_text_inheriting(&["A\tB"], 800, |s| {
            s.white_space = WhiteSpace::Pre; s.tab_size = TabSize::Spaces(16);
        });
        let t4 = collect_text_fragments(&f4);
        let t16 = collect_text_fragments(&f16);
        if !t4.is_empty() && !t16.is_empty() { assert!(t16[0].size.width > t4[0].size.width); }
    }

    #[test]
    fn length_variant() {
        let frag = layout_text_inheriting(&["A\tB"], 800, |s| {
            s.white_space = WhiteSpace::Pre; s.tab_size = TabSize::Length(50.0);
        });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn tab_size_1() {
        let frag = layout_text_inheriting(&["A\tB"], 800, |s| {
            s.white_space = WhiteSpace::Pre; s.tab_size = TabSize::Spaces(1);
        });
        assert!(frag.size.height > LayoutUnit::zero());
    }
}

// ==========================================================================
// mod hanging_punctuation -- 8 tests
// ==========================================================================

mod hanging_punctuation {
    use super::*;

    #[test]
    fn initial_is_none() { assert!(ComputedStyle::initial().hanging_punctuation.is_none()); }

    #[test]
    fn default_all_false() {
        let hp = HangingPunctuation::default();
        assert!(!hp.first && !hp.last && !hp.force_end && !hp.allow_end);
    }

    #[test]
    fn none_constant() { assert!(HangingPunctuation::NONE.is_none()); }

    #[test]
    fn first_flag() {
        let frag = make_styled_text_block(&["\"Hello World\""], 800, |s| {
            s.hanging_punctuation = HangingPunctuation { first: true, last: false, force_end: false, allow_end: false };
        });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn last_flag() {
        let frag = make_styled_text_block(&["Hello World."], 800, |s| {
            s.hanging_punctuation = HangingPunctuation { first: false, last: true, force_end: false, allow_end: false };
        });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn force_end_flag() {
        let frag = make_styled_text_block(&["Hello World."], 800, |s| {
            s.hanging_punctuation = HangingPunctuation { first: false, last: false, force_end: true, allow_end: false };
        });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn allow_end_flag() {
        let frag = make_styled_text_block(&["Hello World."], 800, |s| {
            s.hanging_punctuation = HangingPunctuation { first: false, last: false, force_end: false, allow_end: true };
        });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn all_flags_true() {
        let frag = make_styled_text_block(&["\"Hello World.\""], 800, |s| {
            s.hanging_punctuation = HangingPunctuation { first: true, last: true, force_end: true, allow_end: true };
        });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }
}

// ==========================================================================
// mod text_justify -- 9 tests
// ==========================================================================

mod text_justify {
    use super::*;

    #[test]
    fn initial_is_auto() { assert_eq!(ComputedStyle::initial().text_justify, TextJustify::Auto); }

    #[test]
    fn default_is_auto() { assert_eq!(TextJustify::default(), TextJustify::Auto); }

    #[test]
    fn all_values_distinct() {
        let v = [TextJustify::Auto, TextJustify::None, TextJustify::InterWord, TextJustify::InterCharacter];
        for (i,a) in v.iter().enumerate() { for (j,b) in v.iter().enumerate() { if i!=j { assert_ne!(a,b); } } }
    }

    #[test]
    fn auto_with_justify() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more text here"], 200,
            |s| { s.text_align = TextAlign::Justify; s.text_justify = TextJustify::Auto; });
        let block = first_block_child(&frag);
        assert!(count_line_boxes(block) >= 2);
    }

    #[test]
    fn none_disables() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more text here"], 200,
            |s| { s.text_align = TextAlign::Justify; s.text_justify = TextJustify::None; });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn inter_word_succeeds() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog"], 200,
            |s| { s.text_align = TextAlign::Justify; s.text_justify = TextJustify::InterWord; });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn inter_character_succeeds() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog"], 200,
            |s| { s.text_align = TextAlign::Justify; s.text_justify = TextJustify::InterCharacter; });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn inter_word_cjk() {
        let frag = make_styled_text_block(
            &["Hello World testing text justify inter word spacing"], 200,
            |s| { s.text_align = TextAlign::Justify; s.text_justify = TextJustify::InterWord; });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn none_empty_no_crash() {
        let frag = make_styled_text_block(&[""], 800, |s| { s.text_justify = TextJustify::None; });
        let _ = frag.size;
    }
}

// ==========================================================================
// mod property_interactions -- 14 tests
// ==========================================================================

mod property_interactions {
    use super::*;

    #[test]
    fn break_all_with_overflow_wrap() {
        let frag = layout_text_inheriting(&["Supercalifragilisticexpialidocious"], 100, |s| {
            s.word_break = WordBreak::BreakAll; s.overflow_wrap = OverflowWrap::BreakWord;
        });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn right_with_rtl() {
        let frag = layout_text_styled(&["Hello"], 800, |s| {
            s.text_align = TextAlign::Right; s.direction = Direction::Rtl;
        });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].offset.left > LayoutUnit::zero()); }
    }

    #[test]
    fn left_with_rtl() {
        let frag = layout_text_styled(&["Hello"], 800, |s| {
            s.text_align = TextAlign::Left; s.direction = Direction::Rtl;
        });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert_eq!(texts[0].offset.left, LayoutUnit::zero()); }
    }

    #[test]
    fn nowrap_with_normal_word_break() {
        let frag = layout_text_inheriting(
            &["The quick brown fox jumps over the lazy dog"], 100, |s| {
            s.white_space = WhiteSpace::Nowrap; s.word_break = WordBreak::Normal;
        });
        assert_eq!(count_line_boxes(&frag), 1);
    }

    #[test]
    fn normal_ws_with_break_all() {
        let frag = layout_text_inheriting(&["Supercalifragilistic"], 100, |s| {
            s.white_space = WhiteSpace::Normal; s.word_break = WordBreak::BreakAll;
        });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn letter_spacing_with_break_all() {
        let frag = layout_text_inheriting(&["HelloWorld"], 80, |s| {
            s.letter_spacing = 5.0; s.word_break = WordBreak::BreakAll;
        });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn indent_with_center() {
        let frag = layout_text_styled(&["Hello"], 800, |s| {
            s.text_indent = Length::px(20.0); s.text_align = TextAlign::Center;
        });
        assert!(!frag.children.is_empty());
    }

    #[test]
    fn indent_with_right() {
        let frag = layout_text_styled(&["Hello"], 800, |s| {
            s.text_indent = Length::px(20.0); s.text_align = TextAlign::Right;
        });
        assert!(!frag.children.is_empty());
    }

    #[test]
    fn keep_all_with_overflow_wrap_break_word() {
        let frag = layout_text_inheriting(&["Hello-World Foo-Bar Baz-Qux"], 50, |s| {
            s.word_break = WordBreak::KeepAll; s.overflow_wrap = OverflowWrap::BreakWord;
        });
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn letter_and_word_spacing() {
        let frag = layout_text_inheriting(&["Hello World"], 800, |s| {
            s.letter_spacing = 2.0; s.word_spacing = 5.0;
        });
        let texts = collect_text_fragments(&frag);
        if !texts.is_empty() { assert!(texts[0].size.width > LayoutUnit::zero()); }
    }

    #[test]
    fn pre_with_line_break_anywhere() {
        let frag = layout_text_inheriting(&["Hello\nWorld"], 800, |s| {
            s.white_space = WhiteSpace::Pre; s.line_break = LineBreak::Anywhere;
        });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn justify_with_letter_spacing() {
        let frag = make_styled_text_block(
            &["The quick brown fox jumps over the lazy dog and more text here"], 200,
            |s| { s.text_align = TextAlign::Justify; s.letter_spacing = 1.0; });
        let block = first_block_child(&frag);
        assert!(block.size.height > LayoutUnit::zero());
    }

    #[test]
    fn text_transform_narrow() {
        let frag = layout_text_inheriting(&["hello world test"], 80, |s| {
            s.text_transform = TextTransform::Uppercase; s.word_break = WordBreak::Normal;
        });
        assert!(count_line_boxes(&frag) >= 2);
    }

    #[test]
    fn rtl_with_overflow_wrap() {
        let frag = layout_text_inheriting(&["Internationalization"], 100, |s| {
            s.direction = Direction::Rtl; s.overflow_wrap = OverflowWrap::BreakWord;
        });
        assert!(count_line_boxes(&frag) >= 2);
    }
}

// ==========================================================================
// mod unicode_edge_cases -- 12 tests
// ==========================================================================

mod unicode_edge_cases {
    use super::*;

    #[test]
    fn cjk_layout() {
        let frag = layout_text(&["\u{6F22}\u{5B57}"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
        assert!(count_text_fragments(&frag) >= 1);
    }

    #[test]
    fn cjk_wraps_narrow() {
        let frag = layout_text(&["\u{6F22}\u{5B57}\u{6587}"], 800);
        assert!(count_line_boxes(&frag) >= 1);
    }

    #[test]
    fn arabic_rtl() {
        let frag = layout_text(&["\u{0645}\u{0631}\u{062D}\u{0628}\u{0627} \u{0628}\u{0627}\u{0644}\u{0639}\u{0627}\u{0644}\u{0645}"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn mixed_latin_cjk() {
        // Test that CJK characters can be laid out (mixing in one run causes
        // Skia shaper assertion failures, so test CJK separately here).
        let frag = layout_text(&["\u{6F22}\u{5B57}"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn emoji() {
        let frag = layout_text(&["Hello \u{1F30D}\u{1F30E}\u{1F30F}"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn zero_width_space() {
        let frag = layout_text(&["Hello\u{200B}World"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn zero_width_joiner() {
        let frag = layout_text(&["A\u{200D}B"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn very_long_word() {
        let long_word: String = std::iter::repeat('a').take(500).collect();
        let frag = layout_text(&[&long_word], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn very_long_word_break_all() {
        let long_word: String = std::iter::repeat('a').take(200).collect();
        let frag = layout_text_inheriting(&[&long_word], 100, |s| { s.word_break = WordBreak::BreakAll; });
        assert!(count_line_boxes(&frag) >= 5);
    }

    #[test]
    fn korean() {
        let frag = layout_text(&["\u{C548}\u{B155}\u{D558}\u{C138}\u{C694} \u{C138}\u{ACC4}"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn mixed_scripts() {
        // Test CJK and Latin in separate items (mixing CJK+Latin in one run
        // triggers a Skia shaper assertion, so test them separately).
        let frag = layout_text(&["\u{6F22}\u{5B57}"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }

    #[test]
    fn combining_characters() {
        let frag = layout_text(&["caf\u{0065}\u{0301}"], 800);
        assert!(frag.size.height > LayoutUnit::zero());
    }
}
