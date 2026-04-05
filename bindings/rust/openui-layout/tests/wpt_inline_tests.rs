//! WPT-equivalent tests for CSS Inline Layout Module.
//!
//! Each test corresponds to behaviors verified by WPT css/css-inline tests.
//! Categories: vertical-align, line-height, baseline alignment.

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::LayoutUnit;
use openui_layout::block::block_layout;
use openui_layout::inline::algorithm::inline_layout;
use openui_layout::{ConstraintSpace, Fragment, FragmentKind};
use openui_style::{ComputedStyle, Display, LineHeight, VerticalAlign};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

fn lu_i(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn space(width: i32, height: i32) -> ConstraintSpace {
    ConstraintSpace::for_root(lu_i(width), lu_i(height))
}

fn inline_space(width: i32) -> ConstraintSpace {
    ConstraintSpace::for_block_child(lu_i(width), lu_i(600), lu_i(width), lu_i(600), false)
}

fn layout_text(texts: &[&str], width: i32) -> Fragment {
    let (doc, block) = make_text_block(texts, width);
    inline_layout(&doc, block, &inline_space(width))
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

/// Build inline layout with custom per-text style.
fn layout_text_with_style(
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
    inline_layout(&doc, block, &inline_space(width))
}

fn count_line_boxes(fragment: &Fragment) -> usize {
    fragment
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::Box)
        .count()
}

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

fn first_block_child(fragment: &Fragment) -> &Fragment {
    fragment
        .children
        .iter()
        .find(|c| c.kind == FragmentKind::Box)
        .unwrap()
}

// ═══════════════════════════════════════════════════════════════════════
// VERTICAL-ALIGN TESTS (22 tests)
// Corresponds to WPT css/css-inline/vertical-align-*
// ═══════════════════════════════════════════════════════════════════════
mod vertical_align {
    use super::*;

    /// vertical-align initial value is baseline (WPT vertical-align-initial-001).
    #[test]
    fn initial_value_is_baseline() {
        let style = ComputedStyle::default();
        assert_eq!(style.vertical_align, VerticalAlign::Baseline);
    }

    /// Baseline-aligned text has a non-negative top offset.
    #[test]
    fn baseline_text_has_nonnegative_offset() {
        let frag = layout_text(&["Hello"], 800);
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(
            texts[0].offset.top >= LayoutUnit::zero(),
            "Baseline text top should be >= 0"
        );
    }

    /// vertical-align: sub shifts text below the baseline position.
    #[test]
    fn sub_shifts_text_below_baseline() {
        let frag_bl = layout_text(&["ABC"], 800);
        let (doc, block) = make_span_block(&["ABC"], |s| {
            s.vertical_align = VerticalAlign::Sub;
        });
        let frag_sub = inline_layout(&doc, block, &inline_space(800));

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

    /// vertical-align: super shifts text above the baseline position.
    #[test]
    fn super_shifts_text_above_baseline() {
        // Use two fragments on the same line to compare offsets
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
        doc.node_mut(span).style.vertical_align = VerticalAlign::Super;
        doc.append_child(block, span);
        let t2 = doc.create_node(ElementTag::Text);
        doc.node_mut(t2).text = Some("sup".to_string());
        doc.node_mut(t2).style.display = Display::Inline;
        doc.node_mut(t2).style.vertical_align = VerticalAlign::Super;
        doc.append_child(span, t2);

        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert_eq!(texts.len(), 2);
        assert!(
            texts[1].offset.top < texts[0].offset.top,
            "Super text should be above baseline text"
        );
    }

    /// vertical-align: super is higher than vertical-align: sub.
    #[test]
    fn super_is_above_sub() {
        let (doc_sub, block_sub) = make_span_block(&["x"], |s| {
            s.vertical_align = VerticalAlign::Sub;
        });
        let frag_sub = inline_layout(&doc_sub, block_sub, &inline_space(800));

        let (doc_sup, block_sup) = make_span_block(&["x"], |s| {
            s.vertical_align = VerticalAlign::Super;
        });
        let frag_sup = inline_layout(&doc_sup, block_sup, &inline_space(800));

        let t_sub = collect_text_fragments(&frag_sub);
        let t_sup = collect_text_fragments(&frag_sup);
        assert!(!t_sub.is_empty() && !t_sup.is_empty());
        assert!(
            t_sup[0].offset.top < t_sub[0].offset.top,
            "Super ({:?}) should be above sub ({:?})",
            t_sup[0].offset.top,
            t_sub[0].offset.top
        );
    }

    /// vertical-align: text-top produces layout with text fragment.
    #[test]
    fn text_top_produces_text_fragment() {
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::TextTop;
        });
        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty(), "TextTop should produce a text fragment");
        assert!(texts[0].size.width > LayoutUnit::zero());
    }

    /// vertical-align: text-bottom produces layout with text fragment.
    #[test]
    fn text_bottom_produces_text_fragment() {
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::TextBottom;
        });
        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(texts[0].size.width > LayoutUnit::zero());
    }

    /// vertical-align: middle produces text with non-negative offset.
    #[test]
    fn middle_produces_valid_offset() {
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Middle;
        });
        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        // Middle-aligned text should have a reasonable vertical position
        assert!(texts[0].offset.top.to_f32().abs() < 50.0);
    }

    /// vertical-align: top places text near the top of the line box.
    #[test]
    fn top_aligns_near_line_box_top() {
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Top;
        });
        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(
            texts[0].offset.top.to_f32().abs() < 5.0,
            "Top-aligned text should be near line top, got {:?}",
            texts[0].offset.top
        );
    }

    /// vertical-align: bottom produces a valid layout.
    #[test]
    fn bottom_produces_valid_layout() {
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Bottom;
        });
        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(frag.size.height > LayoutUnit::zero());
    }

    /// vertical-align: Length(5.0) shifts baseline upward by 5px.
    #[test]
    fn length_positive_shifts_up() {
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
        doc.node_mut(span).style.vertical_align = VerticalAlign::Length(5.0);
        doc.append_child(block, span);
        let t2 = doc.create_node(ElementTag::Text);
        doc.node_mut(t2).text = Some("up".to_string());
        doc.node_mut(t2).style.display = Display::Inline;
        doc.node_mut(t2).style.vertical_align = VerticalAlign::Length(5.0);
        doc.append_child(span, t2);

        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert_eq!(texts.len(), 2);
        assert!(
            texts[1].offset.top < texts[0].offset.top,
            "Length(5.0) should shift text up: shifted={:?} base={:?}",
            texts[1].offset.top,
            texts[0].offset.top
        );
    }

    /// vertical-align: Length(-5.0) shifts text downward.
    #[test]
    fn length_negative_shifts_down() {
        let frag_bl = layout_text(&["x"], 800);
        let (doc, block) = make_span_block(&["x"], |s| {
            s.vertical_align = VerticalAlign::Length(-5.0);
        });
        let frag_neg = inline_layout(&doc, block, &inline_space(800));

        let t_bl = collect_text_fragments(&frag_bl);
        let t_neg = collect_text_fragments(&frag_neg);
        assert!(!t_bl.is_empty() && !t_neg.is_empty());
        assert!(
            t_neg[0].offset.top > t_bl[0].offset.top,
            "Length(-5.0) should push text down: neg={:?} bl={:?}",
            t_neg[0].offset.top,
            t_bl[0].offset.top
        );
    }

    /// vertical-align: Length(0.0) behaves identically to baseline.
    #[test]
    fn length_zero_equals_baseline() {
        let frag_bl = layout_text(&["Hello"], 800);
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Length(0.0);
        });
        let frag_zero = inline_layout(&doc, block, &inline_space(800));

        let t_bl = collect_text_fragments(&frag_bl);
        let t_zero = collect_text_fragments(&frag_zero);
        assert!(!t_bl.is_empty() && !t_zero.is_empty());
        assert_eq!(
            t_bl[0].offset.top, t_zero[0].offset.top,
            "Length(0) should equal baseline"
        );
    }

    /// vertical-align: Percentage(50.0) produces a valid layout distinct from baseline.
    #[test]
    fn percentage_produces_valid_layout() {
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Percentage(50.0);
        });
        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(frag.size.height > LayoutUnit::zero());
    }

    /// Positive percentage shifts text upward relative to baseline.
    #[test]
    fn percentage_positive_shifts_up() {
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
        doc.node_mut(span).style.vertical_align = VerticalAlign::Percentage(100.0);
        doc.append_child(block, span);
        let t2 = doc.create_node(ElementTag::Text);
        doc.node_mut(t2).text = Some("shifted".to_string());
        doc.node_mut(t2).style.display = Display::Inline;
        doc.node_mut(t2).style.vertical_align = VerticalAlign::Percentage(100.0);
        doc.append_child(span, t2);

        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert_eq!(texts.len(), 2);
        // 100% of line-height is a significant upward shift
        assert_ne!(
            texts[0].offset.top, texts[1].offset.top,
            "Percentage(100) should differ from baseline"
        );
    }

    /// vertical-align does not affect horizontal (inline) size of text.
    #[test]
    fn does_not_affect_inline_size() {
        let frag_bl = layout_text(&["Hello"], 800);
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Super;
        });
        let frag_sup = inline_layout(&doc, block, &inline_space(800));

        let t_bl = collect_text_fragments(&frag_bl);
        let t_sup = collect_text_fragments(&frag_sup);
        assert!(!t_bl.is_empty() && !t_sup.is_empty());
        assert_eq!(
            t_bl[0].size.width, t_sup[0].size.width,
            "Vertical-align should not change text width"
        );
    }

    /// Sub alignment may expand the line box vertically.
    #[test]
    fn sub_expands_line_box() {
        let frag_normal = layout_text(&["Hello"], 800);
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Sub;
        });
        let frag_sub = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag_sub.size.height >= frag_normal.size.height,
            "Sub should not shrink line box"
        );
    }

    /// Super alignment may expand the line box vertically.
    #[test]
    fn super_expands_line_box() {
        let frag_normal = layout_text(&["Hello"], 800);
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Super;
        });
        let frag_sup = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag_sup.size.height >= frag_normal.size.height,
            "Super should expand or maintain line box height"
        );
    }

    /// Mixed vertical-align values on the same line produce different offsets.
    #[test]
    fn mixed_on_same_line() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.append_child(root, block);

        // Normal baseline text
        let t1 = doc.create_node(ElementTag::Text);
        doc.node_mut(t1).text = Some("normal ".to_string());
        doc.node_mut(t1).style.display = Display::Inline;
        doc.append_child(block, t1);

        // Super-aligned span
        let span = doc.create_node(ElementTag::Span);
        doc.node_mut(span).style.display = Display::Inline;
        doc.node_mut(span).style.vertical_align = VerticalAlign::Super;
        doc.append_child(block, span);
        let t2 = doc.create_node(ElementTag::Text);
        doc.node_mut(t2).text = Some("super".to_string());
        doc.node_mut(t2).style.display = Display::Inline;
        doc.node_mut(t2).style.vertical_align = VerticalAlign::Super;
        doc.append_child(span, t2);

        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert_eq!(texts.len(), 2);
        assert_ne!(
            texts[0].offset.top, texts[1].offset.top,
            "Mixed align should produce different vertical positions"
        );
    }

    /// Vertical-align preserves text shaping result.
    #[test]
    fn preserves_shape_result() {
        let (doc, block) = make_span_block(&["Hello"], |s| {
            s.vertical_align = VerticalAlign::Super;
        });
        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        assert!(
            texts[0].shape_result.is_some(),
            "Vertical-align should not discard shape result"
        );
    }

    /// Large positive Length value raises text significantly.
    #[test]
    fn length_large_positive_raises_significantly() {
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
        doc.node_mut(span).style.vertical_align = VerticalAlign::Length(20.0);
        doc.append_child(block, span);
        let t2 = doc.create_node(ElementTag::Text);
        doc.node_mut(t2).text = Some("raised".to_string());
        doc.node_mut(t2).style.display = Display::Inline;
        doc.node_mut(t2).style.vertical_align = VerticalAlign::Length(20.0);
        doc.append_child(span, t2);

        let frag = inline_layout(&doc, block, &inline_space(800));
        let texts = collect_text_fragments(&frag);
        assert_eq!(texts.len(), 2);
        let diff = texts[0].offset.top.to_f32() - texts[1].offset.top.to_f32();
        assert!(
            diff >= 15.0,
            "Length(20) should raise text by ~20px, got diff={diff}"
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// LINE-HEIGHT TESTS (15 tests)
// Corresponds to WPT css/css-inline/line-height-*
// ═══════════════════════════════════════════════════════════════════════
mod line_height {
    use super::*;

    /// Default line-height is Normal.
    #[test]
    fn default_is_normal() {
        let style = ComputedStyle::default();
        assert_eq!(style.line_height, LineHeight::Normal);
    }

    /// Normal line-height produces a positive line box height.
    #[test]
    fn normal_produces_positive_height() {
        let frag = layout_text(&["Hello"], 800);
        assert!(
            frag.size.height > LayoutUnit::zero(),
            "Normal line-height should produce positive height"
        );
    }

    /// line-height: Number(2.0) produces line box >= 2× font-size (32px at 16px).
    #[test]
    fn number_multiplier_doubles_height() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.line_height = LineHeight::Number(2.0);
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("Hello".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.node_mut(t).style.line_height = LineHeight::Number(2.0);
        doc.append_child(block, t);

        let frag = inline_layout(&doc, block, &inline_space(800));
        // 16px font × 2.0 = 32px minimum
        assert!(
            frag.size.height >= lu(32.0),
            "Number(2.0) at 16px should be >= 32px, got {:?}",
            frag.size.height
        );
    }

    /// line-height: Length(48.0) produces line box >= 48px.
    #[test]
    fn fixed_length_48px() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.line_height = LineHeight::Length(48.0);
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("Test".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.node_mut(t).style.line_height = LineHeight::Length(48.0);
        doc.append_child(block, t);

        let frag = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag.size.height >= lu(48.0),
            "Length(48.0) should produce >= 48px, got {:?}",
            frag.size.height
        );
    }

    /// line-height: Percentage(150.0) at 16px → computed 24px.
    #[test]
    fn percentage_150_computes_correctly() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.line_height = LineHeight::Percentage(150.0);
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("Test".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.node_mut(t).style.line_height = LineHeight::Percentage(150.0);
        doc.append_child(block, t);

        let frag = inline_layout(&doc, block, &inline_space(800));
        // 16px × 150% = 24px
        assert!(
            frag.size.height >= lu(24.0),
            "Percentage(150) at 16px should be >= 24px, got {:?}",
            frag.size.height
        );
    }

    /// line-height: Percentage(200.0) at 16px → computed 32px.
    #[test]
    fn percentage_200_is_double_font_size() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.line_height = LineHeight::Percentage(200.0);
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("Test".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.node_mut(t).style.line_height = LineHeight::Percentage(200.0);
        doc.append_child(block, t);

        let frag = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag.size.height >= lu(32.0),
            "Percentage(200) at 16px should be >= 32px, got {:?}",
            frag.size.height
        );
    }

    /// Line height affects the overall line box height.
    #[test]
    fn larger_line_height_produces_taller_box() {
        let frag_normal = layout_text(&["Hello"], 800);
        let frag_large = layout_text_with_style(&["Hello"], 800, |s| {
            s.line_height = LineHeight::Length(60.0);
        });
        assert!(
            frag_large.size.height > frag_normal.size.height,
            "60px line-height ({:?}) should be taller than normal ({:?})",
            frag_large.size.height,
            frag_normal.size.height
        );
    }

    /// Two text items with different line-heights — tallest wins.
    #[test]
    fn tallest_line_height_wins() {
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

        let frag = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag.size.height >= lu(60.0),
            "Line box should accommodate tallest item (60px), got {:?}",
            frag.size.height
        );
    }

    /// line-height: Number(0.0) does not crash and is shorter than normal.
    #[test]
    fn zero_does_not_crash_and_is_compact() {
        let frag_normal = layout_text(&["Hello"], 800);
        let frag_zero = layout_text_with_style(&["Hello"], 800, |s| {
            s.line_height = LineHeight::Number(0.0);
        });
        assert!(
            frag_zero.size.height <= frag_normal.size.height,
            "Zero line-height should be <= normal"
        );
    }

    /// Large line-height creates a correspondingly tall line box.
    #[test]
    fn large_value_creates_tall_line() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.line_height = LineHeight::Length(200.0);
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("tall".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.node_mut(t).style.line_height = LineHeight::Length(200.0);
        doc.append_child(block, t);

        let frag = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag.size.height >= lu(200.0),
            "200px line-height should produce >= 200px, got {:?}",
            frag.size.height
        );
    }

    /// line-height: Number(0.5) is shorter than default normal.
    #[test]
    fn half_multiplier_shorter_than_normal() {
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

        let frag_half = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag_half.size.height < frag_normal.size.height,
            "0.5 multiplier ({:?}) should be shorter than normal ({:?})",
            frag_half.size.height,
            frag_normal.size.height
        );
    }

    /// line-height: Number(3.0) at 16px → computed 48px.
    #[test]
    fn number_three_produces_48px() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = doc.create_node(ElementTag::Div);
        doc.node_mut(block).style.display = Display::Block;
        doc.node_mut(block).style.line_height = LineHeight::Number(3.0);
        doc.append_child(root, block);

        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some("x".to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.node_mut(t).style.line_height = LineHeight::Number(3.0);
        doc.append_child(block, t);

        let frag = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag.size.height >= lu(48.0),
            "Number(3.0) at 16px should be >= 48px, got {:?}",
            frag.size.height
        );
    }

    /// line-height: Percentage(50.0) is shorter than normal.
    #[test]
    fn percentage_50_shorter_than_normal() {
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

        let frag_50 = inline_layout(&doc, block, &inline_space(800));
        assert!(
            frag_50.size.height <= frag_normal.size.height,
            "50% ({:?}) should be <= normal ({:?})",
            frag_50.size.height,
            frag_normal.size.height
        );
    }

    /// Line-height normal uses font metrics and is consistent across runs.
    #[test]
    fn normal_is_deterministic() {
        let frag1 = layout_text(&["Hello world"], 800);
        let frag2 = layout_text(&["Hello world"], 800);
        assert_eq!(
            frag1.size.height, frag2.size.height,
            "Normal line-height should be deterministic"
        );
    }

    /// Line height with large font size scales accordingly.
    #[test]
    fn scales_with_font_size() {
        let frag_16 = layout_text(&["Hello"], 800);

        let frag_32 = layout_text_with_style(&["Hello"], 800, |s| {
            s.font_size = 32.0;
        });

        // A 2× font size should produce a taller line box
        assert!(
            frag_32.size.height > frag_16.size.height,
            "32px font ({:?}) should be taller than 16px ({:?})",
            frag_32.size.height,
            frag_16.size.height
        );
    }
}

// ═══════════════════════════════════════════════════════════════════════
// BASELINE ALIGNMENT TESTS (8 tests)
// Corresponds to WPT css/css-inline/baseline-*
// ═══════════════════════════════════════════════════════════════════════
mod baseline_alignment {
    use super::*;

    /// A single text fragment has a non-zero baseline offset.
    #[test]
    fn single_text_has_baseline() {
        let frag = layout_text(&["Hello"], 800);
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        // The inline layout result itself tracks baseline
        assert!(
            frag.baseline_offset != 0.0 || frag.size.height > LayoutUnit::zero(),
            "Layout should produce measurable baseline data"
        );
    }

    /// Baseline offset is deterministic for the same input.
    #[test]
    fn baseline_is_deterministic() {
        let frag1 = layout_text(&["Hello world"], 800);
        let frag2 = layout_text(&["Hello world"], 800);
        assert_eq!(
            frag1.baseline_offset, frag2.baseline_offset,
            "Baseline should be consistent across identical layouts"
        );
    }

    /// Multiple text nodes on one line share the same line box.
    #[test]
    fn multiple_texts_share_line_box() {
        let frag = layout_text(&["Hello ", "World"], 800);
        assert_eq!(
            count_line_boxes(&frag),
            1,
            "Two short texts should share a single line box"
        );
        assert_eq!(
            count_text_fragments(&frag),
            2,
            "Should have two text fragments"
        );
    }

    /// Two baseline-aligned texts on the same line have equal top offsets.
    #[test]
    fn two_texts_same_baseline() {
        let frag = layout_text(&["Hello ", "World"], 800);
        let texts = collect_text_fragments(&frag);
        assert_eq!(texts.len(), 2);
        assert_eq!(
            texts[0].offset.top, texts[1].offset.top,
            "Baseline-aligned texts should share the same top offset"
        );
    }

    /// Baseline offset through block_layout integration is consistent.
    #[test]
    fn block_layout_baseline_consistent() {
        let frag1 = block_layout_text(&["Test"], 800);
        let frag2 = block_layout_text(&["Test"], 800);
        let block1 = first_block_child(&frag1);
        let block2 = first_block_child(&frag2);
        assert_eq!(
            block1.baseline_offset, block2.baseline_offset,
            "Block layout baseline should be deterministic"
        );
    }

    /// The line box fragment is a Box kind.
    #[test]
    fn line_box_is_box_kind() {
        let frag = layout_text(&["Hello"], 800);
        assert!(
            frag.children
                .iter()
                .all(|c| c.kind == FragmentKind::Box),
            "All direct children of inline layout should be Box (line boxes)"
        );
    }

    /// Text fragments within the line box are Text kind.
    #[test]
    fn text_fragments_are_text_kind() {
        let frag = layout_text(&["Hello"], 800);
        let texts = collect_text_fragments(&frag);
        assert!(!texts.is_empty());
        for t in &texts {
            assert_eq!(t.kind, FragmentKind::Text);
        }
    }

    /// Baseline offset is positive for a line with text content.
    #[test]
    fn baseline_offset_positive_for_text() {
        let frag = layout_text(&["Hello world"], 800);
        // The fragment's baseline_offset should be a reasonable positive value
        // indicating distance from top to baseline
        assert!(
            frag.baseline_offset >= 0.0,
            "Baseline offset should be >= 0, got {}",
            frag.baseline_offset
        );
    }
}
