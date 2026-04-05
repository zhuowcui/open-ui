//! Tests for SP11 Round 20 fixes.
//!
//! Issue 1: Half-leading uses rounded integer metrics (Blink FixedAscent/FixedDescent).
//! Issue 2: Justification excludes trailing spaces from ShapeResult expansion.
//! Issue 3: Tab expansion uses actual glyph advance, not space_advance for all chars.
//! Issue 4: BiDi reorder early-returns when no odd levels exist.
//! Issue 5: capitalize treats digits as word-internal (no spurious "1St").
//! Issue 6: to_titlecase handles ß → Ss and Armenian ligature.
//! Issue 7: Justification accumulator saves old value before mutation.
//! Issue 8: Cross-node whitespace checks prior space collapsibility.

use openui_dom::{Document, ElementTag};
use openui_layout::inline::items_builder::{expand_tabs, InlineItemsBuilder};
use openui_style::{Display, TabSize, TextTransform, WhiteSpace};
use openui_text::{apply_text_transform, FontMetrics};

// ── Helpers ─────────────────────────────────────────────────────────────

// ── Issue 1: Half-leading rounded metrics ───────────────────────────────

#[test]
fn int_ascent_rounds_to_nearest() {
    let m = FontMetrics {
        ascent: 10.4,
        descent: 3.6,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_ascent(), 10.0);
    assert_eq!(m.int_descent(), 4.0);
}

#[test]
fn int_ascent_rounds_half_up() {
    let m = FontMetrics {
        ascent: 10.5,
        descent: 3.5,
        ..FontMetrics::zero()
    };
    // f32::round() rounds half-away-from-zero
    assert_eq!(m.int_ascent(), 11.0);
    assert_eq!(m.int_descent(), 4.0);
}

#[test]
fn int_line_spacing_rounds_sum() {
    let m = FontMetrics {
        ascent: 10.3,
        descent: 3.3,
        line_gap: 2.1,
        ..FontMetrics::zero()
    };
    // 10.3 + 3.3 + 2.1 = 15.7, round = 16.0
    assert_eq!(m.int_line_spacing(), 16.0);
}

#[test]
fn int_line_spacing_zero_gap() {
    let m = FontMetrics {
        ascent: 10.0,
        descent: 4.0,
        line_gap: 0.0,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 14.0);
}

// ── Issue 2: Justification excludes trailing spaces ─────────────────────

#[test]
fn justify_exclude_trailing_spaces() {
    use openui_text::shaping::{TextDirection, TextShaper};
    use openui_text::font::{Font, FontDescription};

    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::new());
    let text = "a b  "; // 2 internal spaces, 2 trailing
    let mut sr = shaper.shape(text, &font, TextDirection::Ltr);
    let original_width = sr.width();

    // Exclude 2 trailing spaces — only 1 internal space should expand.
    sr.apply_justification(10.0, text, 2);
    let expected_width = original_width + 10.0;
    assert!(
        (sr.width() - expected_width).abs() < 0.1,
        "Width should increase by 10.0 (1 expandable space * 10.0), got delta {}",
        sr.width() - original_width,
    );
}

#[test]
fn justify_exclude_all_trailing() {
    use openui_text::shaping::{TextDirection, TextShaper};
    use openui_text::font::{Font, FontDescription};

    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::new());
    let text = "   "; // all spaces are trailing
    let mut sr = shaper.shape(text, &font, TextDirection::Ltr);
    let original_width = sr.width();

    sr.apply_justification(10.0, text, 3);
    assert!(
        (sr.width() - original_width).abs() < 0.01,
        "No expansion expected when all spaces are trailing, delta={}",
        sr.width() - original_width,
    );
}

// ── Issue 3: Tab expansion with actual char widths ──────────────────────

#[test]
fn tab_expansion_proportional_narrow_chars() {
    // Narrow chars (width 5.0 each): "iii" = 15.0, tab-size 4, space_advance=10.0
    // tab_interval = 40.0, current_advance after "iii" = 15.0
    // next_stop = 40.0, tab_width = 25.0, num_spaces = round(25/10) = 3
    let result = expand_tabs("iii\tx", &TabSize::Spaces(4), 10.0, |_| 5.0);
    assert_eq!(result, "iii   x");
}

#[test]
fn tab_expansion_proportional_wide_chars() {
    // Wide chars (width 15.0 each): "WWW" = 45.0, tab-size 4, space_advance=10.0
    // tab_interval = 40.0, current_advance = 45.0
    // next_stop = 80.0, tab_width = 35.0, num_spaces = round(35/10) = 4
    let _result = expand_tabs("WWW\tx", &TabSize::Spaces(4), 10.0, |_| 15.0);
    // With narrow and wide chars: tab position differs
    assert_ne!(
        expand_tabs("iii\tx", &TabSize::Spaces(4), 10.0, |_| 5.0),
        expand_tabs("WWW\tx", &TabSize::Spaces(4), 10.0, |_| 15.0),
        "Proportional chars should produce different tab expansion"
    );
}

#[test]
fn tab_expansion_uniform_width_matches_old_behavior() {
    // When all chars have the same width as space_advance, behavior
    // should match the old space_advance-for-all approach.
    let result_new = expand_tabs("ab\tx", &TabSize::Spaces(4), 10.0, |_| 10.0);
    assert_eq!(result_new, "ab  x");
}

// ── Issue 4: BiDi reorder no-odd early return ───────────────────────────

#[test]
fn bidi_reorder_all_even_levels_preserves_order() {
    // Construct a document with only LTR text; items should not be reordered.
    let mut doc = Document::new();
    let vp = doc.root();
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.append_child(vp, container);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("Hello ".to_string());
    doc.append_child(container, t1);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("World".to_string());
    doc.append_child(container, t2);

    let data = InlineItemsBuilder::collect(&doc, container);
    // All items should have bidi_level 0 (even) → no reordering.
    for item in &data.items {
        assert_eq!(item.bidi_level, 0, "All LTR items should have level 0");
    }
}

#[test]
fn bidi_visual_reorder_even_levels_no_reversal() {
    use openui_text::bidi::BidiParagraph;
    use openui_text::shaping::TextDirection;

    let text = "Hello World";
    let bidi = BidiParagraph::new(text, Some(TextDirection::Ltr));
    let runs = bidi.runs();
    let visual = bidi.visual_runs();
    // Pure LTR: visual order should equal logical order.
    assert_eq!(runs.len(), visual.len());
    for (r, v) in runs.iter().zip(visual.iter()) {
        assert_eq!(r.start, v.start);
        assert_eq!(r.end, v.end);
    }
}

// ── Issue 5: capitalize digits word-internal ────────────────────────────

#[test]
fn capitalize_1st_no_spurious_cap() {
    assert_eq!(
        apply_text_transform("1st", TextTransform::Capitalize),
        "1st",
    );
}

#[test]
fn capitalize_digit_then_space_then_word() {
    // "1st place" → "1st Place" (space is a word boundary)
    assert_eq!(
        apply_text_transform("1st place", TextTransform::Capitalize),
        "1st Place",
    );
}

#[test]
fn capitalize_leading_digits_in_word() {
    assert_eq!(
        apply_text_transform("3d model", TextTransform::Capitalize),
        "3d Model",
    );
}

// ── Issue 6: to_titlecase ß → Ss, Armenian ligature, identity forms ─────

#[test]
fn capitalize_eszett_to_ss() {
    // ß should titlecase to Ss, not SS.
    assert_eq!(
        apply_text_transform("straße", TextTransform::Capitalize),
        "Straße", // Only first letter of the word is capitalized
    );
    // When ß is the first letter:
    assert_eq!(
        apply_text_transform("ßtraße", TextTransform::Capitalize),
        "Sstraße",
    );
}

#[test]
fn capitalize_titlecase_digraph_identity() {
    // Already-titlecase forms ǲ/ǅ/ǈ/ǋ should map to themselves.
    assert_eq!(
        apply_text_transform("\u{01F2}bc", TextTransform::Capitalize),
        "\u{01F2}bc",
    );
}

#[test]
fn capitalize_armenian_ligature() {
    // և (U+0587) should titlecase to Եւ.
    assert_eq!(
        apply_text_transform("\u{0587}", TextTransform::Capitalize),
        "\u{0535}\u{0582}",
    );
}

// ── Issue 7: Justification accumulator precision ────────────────────────

#[test]
fn justify_accumulator_precision_no_drift() {
    use openui_text::shaping::{TextDirection, TextShaper};
    use openui_text::font::{Font, FontDescription};

    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::new());
    // Use a value that causes IEEE 754 precision issues: 0.1
    let text = "a b c d e f g h i j k";
    let mut sr = shaper.shape(text, &font, TextDirection::Ltr);
    let original_width = sr.width();

    // 10 spaces * 0.1 per space = 1.0 total
    sr.apply_justification(0.1, text, 0);
    let delta = sr.width() - original_width;
    assert!(
        (delta - 1.0).abs() < 0.01,
        "Total justification should be ~1.0, got {}",
        delta,
    );
}

// ── Issue 8: Cross-node whitespace collapsibility ───────────────────────

#[test]
fn cross_node_pre_space_does_not_collapse_next() {
    // <pre>text </pre><span>word</span>
    // The trailing space from `pre` is preserved. The next node's text
    // " word" (after collapsing) should NOT have its leading space stripped
    // because the previous space was NOT collapsible.
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.append_child(vp, container);

    // First child: pre-formatted text ending with a space
    let pre_span = doc.create_node(ElementTag::Span);
    doc.node_mut(pre_span).style.display = Display::Inline;
    doc.node_mut(pre_span).style.white_space = WhiteSpace::Pre;
    doc.append_child(container, pre_span);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("text ".to_string());
    doc.node_mut(t1).style.white_space = WhiteSpace::Pre;
    doc.append_child(pre_span, t1);

    // Second child: normal text starting with a space
    let normal_span = doc.create_node(ElementTag::Span);
    doc.node_mut(normal_span).style.display = Display::Inline;
    doc.append_child(container, normal_span);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some(" word".to_string());
    doc.append_child(normal_span, t2);

    let data = InlineItemsBuilder::collect(&doc, container);
    // The concatenated text should have TWO spaces between "text" and "word":
    // one preserved from `pre` and one from the normal text (which was NOT
    // collapsed because the prior space was not collapsible).
    assert!(
        data.text.contains("text  word"),
        "Pre space + normal space should both be present: got {:?}",
        data.text,
    );
}

#[test]
fn cross_node_normal_spaces_still_collapse() {
    // <span>text </span><span> word</span>  (both normal mode)
    // Both spaces are collapsible → only one space between "text" and "word".
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.append_child(vp, container);

    let s1 = doc.create_node(ElementTag::Span);
    doc.node_mut(s1).style.display = Display::Inline;
    doc.append_child(container, s1);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("text ".to_string());
    doc.append_child(s1, t1);

    let s2 = doc.create_node(ElementTag::Span);
    doc.node_mut(s2).style.display = Display::Inline;
    doc.append_child(container, s2);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some(" word".to_string());
    doc.append_child(s2, t2);

    let data = InlineItemsBuilder::collect(&doc, container);
    assert!(
        data.text.contains("text word") && !data.text.contains("text  word"),
        "Normal + normal spaces should collapse to one: got {:?}",
        data.text,
    );
}
