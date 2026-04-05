//! Tests for SP11 Round 27 code review fixes — openui-text crate.
//!
//! Covers Issues 1, 2, 3, 4, 6 from the review.

use openui_text::font::FontMetrics;
use openui_text::shaping::{TextDirection, TextShaper};
use openui_text::font::{Font, FontDescription};

// ── Issue 1: RTL word-spacing applied to wrong glyphs ─────────────────────

#[test]
fn rtl_word_spacing_applies_to_space_character() {
    // Shape RTL text with word-spacing. The space glyph (not the first visual
    // glyph) should receive the extra advance.
    let shaper = TextShaper::new();
    let mut desc = FontDescription::default();
    desc.word_spacing = 20.0;
    desc.letter_spacing = 0.0;
    let font = Font::new(desc);

    // Hebrew text with space: "שלום עולם" (shalom olam)
    let text = "\u{05E9}\u{05DC}\u{05D5}\u{05DD} \u{05E2}\u{05D5}\u{05DC}\u{05DD}";
    let result_rtl = shaper.shape(text, &font, TextDirection::Rtl);

    // Shape the same text without word-spacing to compare.
    let font_no_ws = Font::new(FontDescription::default());
    let result_no_ws = shaper.shape(text, &font_no_ws, TextDirection::Rtl);

    // The total width difference should be exactly the word-spacing (20.0),
    // applied once for the single space character.
    let width_diff = result_rtl.width - result_no_ws.width;
    assert!(
        (width_diff - 20.0).abs() < 0.5,
        "RTL word-spacing should add exactly 20.0 to total width: diff was {}",
        width_diff
    );
}

#[test]
fn rtl_word_spacing_multiple_spaces() {
    // Multiple spaces in RTL text — each should get word-spacing.
    let shaper = TextShaper::new();
    let mut desc = FontDescription::default();
    desc.word_spacing = 10.0;
    desc.letter_spacing = 0.0;
    let font = Font::new(desc);

    let text = "a b c"; // 2 spaces
    let result_ws = shaper.shape(text, &font, TextDirection::Rtl);

    let font_no_ws = Font::new(FontDescription::default());
    let result_no_ws = shaper.shape(text, &font_no_ws, TextDirection::Rtl);

    // 2 spaces × 10.0 = 20.0 extra width.
    let width_diff = result_ws.width - result_no_ws.width;
    assert!(
        (width_diff - 20.0).abs() < 0.5,
        "Two spaces in RTL should add 20.0 total word-spacing: diff was {}",
        width_diff
    );
}

#[test]
fn ltr_word_spacing_still_correct() {
    // Regression: LTR word-spacing should still work correctly.
    let shaper = TextShaper::new();
    let mut desc = FontDescription::default();
    desc.word_spacing = 15.0;
    desc.letter_spacing = 0.0;
    let font = Font::new(desc);

    let text = "a b";
    let result_ws = shaper.shape(text, &font, TextDirection::Ltr);

    let font_no_ws = Font::new(FontDescription::default());
    let result_no_ws = shaper.shape(text, &font_no_ws, TextDirection::Ltr);

    let width_diff = result_ws.width - result_no_ws.width;
    assert!(
        (width_diff - 15.0).abs() < 0.5,
        "LTR word-spacing should add 15.0 for one space: diff was {}",
        width_diff
    );
}

// ── Issue 2: int_line_spacing rounds sum not each metric ──────────────────

#[test]
fn int_line_spacing_blink_round27_example() {
    // Issue example: ascent=9.408, descent=3.136, line_gap=0
    // Blink: lroundf(9.408) + lroundf(3.136) + lroundf(0) = 9.0 + 3.0 + 0.0 = 12.0
    let m = FontMetrics {
        ascent: 9.408,
        descent: 3.136,
        line_gap: 0.0,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 12.0);
}

#[test]
fn int_line_spacing_line_gap_not_independently_rounded() {
    // Each metric rounded independently:
    // round(10.0) + round(4.0) + round(0.7) = 10.0 + 4.0 + 1.0 = 15.0
    let m = FontMetrics {
        ascent: 10.0,
        descent: 4.0,
        line_gap: 0.7,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 15.0);
}

#[test]
fn int_line_spacing_fractional_ascent_descent_sum() {
    // round(7.3 + 2.8) + 1.0 = round(10.1) + 1.0 = 10.0 + 1.0 = 11.0
    let m = FontMetrics {
        ascent: 7.3,
        descent: 2.8,
        line_gap: 1.0,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 11.0);
}

// ── Issue 3: Platform fallback checks all missing chars ──────────────────

#[test]
fn platform_fallback_handles_mixed_missing_segment() {
    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::default());

    // Latin text with a few non-Latin chars that should trigger fallback.
    // Use common symbols that most systems have fonts for.
    let text = "Hello\u{00E9}\u{00F1}World"; // Helloéñ World
    let result = shaper.shape(text, &font, TextDirection::Ltr);

    assert_eq!(result.num_characters, text.chars().count());
    assert!(result.width > 0.0, "Mixed text should have positive width");
    assert!(!result.runs.is_empty(), "Should have at least one run");
}

#[test]
fn platform_fallback_iterates_for_distinct_missing_chars() {
    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::default());

    // Use common symbol characters that are widely available.
    let text = "x\u{00A9}\u{00AE}y"; // x©®y (copyright + registered)
    let result = shaper.shape(text, &font, TextDirection::Ltr);

    assert_eq!(result.num_characters, text.chars().count());
    assert!(result.width > 0.0);
    for run in &result.runs {
        assert!(run.num_glyphs > 0, "Each run should have glyphs");
    }
}

// ── Issue 4: Synthetic oblique preserved for platform fallback ───────────

#[test]
fn platform_fallback_preserves_oblique_angle() {
    let mut desc = FontDescription::default();
    desc.style = openui_style::FontStyleEnum::Oblique(14.0);

    let mut cache = openui_text::font::cache::GLOBAL_FONT_CACHE.lock().unwrap();
    if let Some(fb_data) = cache.platform_fallback_for_character('\u{4E16}', &desc) {
        assert_eq!(
            fb_data.synthetic_oblique_angle(),
            14.0,
            "Platform fallback font should carry the oblique angle from the description"
        );
    }
}

#[test]
fn platform_fallback_normal_style_has_zero_oblique() {
    let desc = FontDescription::default();
    let mut cache = openui_text::font::cache::GLOBAL_FONT_CACHE.lock().unwrap();
    if let Some(fb_data) = cache.platform_fallback_for_character('A', &desc) {
        assert_eq!(
            fb_data.synthetic_oblique_angle(),
            0.0,
            "Normal-style fallback font should have 0° oblique"
        );
    }
}

// ── Issue 6: Bidi sanitization includes U+2028 ──────────────────────────

#[test]
fn bidi_sanitizes_line_separator_u2028() {
    use openui_text::bidi::BidiParagraph;

    let text = "Hello\u{2028}World";
    let bidi = BidiParagraph::new(text, Some(TextDirection::Ltr));

    let char_count = text.chars().count();
    assert_eq!(
        bidi.levels().len(),
        char_count,
        "All {} characters should have bidi levels (U+2028 should be sanitized)",
        char_count
    );
}

#[test]
fn bidi_u2028_does_not_split_rtl_analysis() {
    use openui_text::bidi::BidiParagraph;

    let text = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}\u{2028}\u{05E2}\u{05D5}\u{05DC}\u{05DD}";
    let bidi = BidiParagraph::new(text, Some(TextDirection::Rtl));

    let char_count = text.chars().count();
    assert_eq!(
        bidi.levels().len(),
        char_count,
        "All characters including those after U+2028 should have bidi levels"
    );

    for i in 5..char_count {
        assert!(
            bidi.levels()[i].is_rtl(),
            "Hebrew char at index {} (after U+2028) should be RTL",
            i
        );
    }
}

#[test]
fn bidi_preserves_original_text_with_u2028() {
    use openui_text::bidi::BidiParagraph;

    let text = "A\u{2028}B";
    let bidi = BidiParagraph::new(text, None);
    assert_eq!(bidi.text(), text, "Original text with U+2028 should be preserved");
    assert!(bidi.text().contains('\u{2028}'));
}
