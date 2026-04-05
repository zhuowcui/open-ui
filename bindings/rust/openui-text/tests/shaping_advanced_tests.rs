//! Advanced shaping tests covering kerning, multi-script text, emoji,
//! whitespace variants, ShapeResult operations, edge cases, direction, and
//! script segmenter edge cases.

use openui_text::font::{Font, FontDescription};
use openui_text::shaping::{
    RunSegmenter, Script, ShapeResult, TextDirection, TextShaper,
};

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

fn make_font(size: f32) -> Font {
    let mut d = FontDescription::new();
    d.size = size;
    d.specified_size = size;
    Font::new(d)
}

fn shape_text(text: &str) -> ShapeResult {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Ltr)
}

fn shape_rtl(text: &str) -> ShapeResult {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Rtl)
}

fn shape_with_size(text: &str, size: f32) -> ShapeResult {
    let font = make_font(size);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Ltr)
}

// ═══════════════════════════════════════════════════════════════════════
// 1. Kerning (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn kern_av_pair_not_wider_than_sum() {
    let av = shape_text("AV");
    let a = shape_text("A");
    let v = shape_text("V");
    // With kerning, "AV" should be narrower or equal to individual widths.
    assert!(
        av.width() <= a.width() + v.width() + 0.01,
        "AV ({}) should be <= A ({}) + V ({})",
        av.width(),
        a.width(),
        v.width()
    );
}

#[test]
fn kern_to_pair_not_wider_than_sum() {
    let to = shape_text("To");
    let t = shape_text("T");
    let o = shape_text("o");
    assert!(
        to.width() <= t.width() + o.width() + 0.01,
        "To ({}) should be <= T ({}) + o ({})",
        to.width(),
        t.width(),
        o.width()
    );
}

#[test]
fn kern_different_pairs_may_differ() {
    // Different kern pairs produce different total widths even when the
    // individual characters have similar widths. We just verify that
    // "AV" and "VA" both shape to positive widths and may differ.
    let av = shape_text("AV");
    let va = shape_text("VA");
    assert!(av.width() > 0.0);
    assert!(va.width() > 0.0);
    // Both contain the same characters, so widths should be close but
    // kerning may make them slightly different.
    let diff = (av.width() - va.width()).abs();
    assert!(
        diff < av.width() * 0.5,
        "AV ({}) and VA ({}) should have similar widths, diff={}",
        av.width(),
        va.width(),
        diff
    );
}

#[test]
fn kern_non_kerning_pair_width_close_to_sum() {
    let ab = shape_text("ab");
    let a = shape_text("a");
    let b = shape_text("b");
    let sum = a.width() + b.width();
    // "ab" is not a strong kern pair; width should be very close to sum.
    let tolerance = sum * 0.1;
    assert!(
        (ab.width() - sum).abs() < tolerance,
        "ab ({}) should be close to a ({}) + b ({}), tolerance={}",
        ab.width(),
        a.width(),
        b.width(),
        tolerance
    );
}

#[test]
fn kern_effect_at_reasonable_font_size() {
    // Kerning effects should be measurable at typical font sizes.
    let av_16 = shape_with_size("AV", 16.0);
    let a_16 = shape_with_size("A", 16.0);
    let v_16 = shape_with_size("V", 16.0);
    let pair_reduction = (a_16.width() + v_16.width()) - av_16.width();
    // Kerning reduction should be non-negative (pair can't be wider than parts).
    assert!(
        pair_reduction >= -0.01,
        "Kerning should not make pair wider: reduction={}",
        pair_reduction
    );
    // At larger sizes, kerning should still apply.
    let av_48 = shape_with_size("AV", 48.0);
    let a_48 = shape_with_size("A", 48.0);
    let v_48 = shape_with_size("V", 48.0);
    let pair_reduction_48 = (a_48.width() + v_48.width()) - av_48.width();
    assert!(
        pair_reduction_48 >= -0.01,
        "Kerning at 48px should not make pair wider: reduction={}",
        pair_reduction_48
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Multi-script text (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn multi_script_arabic_only_shapes_with_positive_width() {
    // Pure Arabic text shaped LTR should still produce output.
    let result = shape_text("مرحبا");
    assert!(result.width() > 0.0, "Arabic text should have positive width");
    assert!(result.num_characters > 0);
}

#[test]
fn multi_script_cjk_only_positive_width() {
    let result = shape_text("你好世界");
    assert!(result.width() > 0.0, "CJK text should have positive width");
    assert_eq!(result.num_characters, 4);
}

#[test]
fn segmenter_latin_arabic_multiple_segments() {
    let segments = RunSegmenter::segment("Hello مرحبا");
    assert!(
        segments.len() >= 2,
        "Latin+Arabic should produce >= 2 segments, got {}",
        segments.len()
    );
    // First segment should be Latin.
    assert_eq!(segments[0].script, Script::Latin);
    // Last segment should be Arabic.
    let last = segments.last().unwrap();
    assert_eq!(last.script, Script::Arabic);
}

#[test]
fn segmenter_latin_cjk_multiple_segments() {
    let segments = RunSegmenter::segment("Hello 你好");
    assert!(
        segments.len() >= 2,
        "Latin+CJK should produce >= 2 segments, got {}",
        segments.len()
    );
    // Verify at least one segment is Han script.
    let has_han = segments.iter().any(|s| s.script == Script::Han);
    assert!(has_han, "Should have a Han script segment");
}

#[test]
fn segmenter_pure_digits_common_script() {
    let segments = RunSegmenter::segment("12345");
    assert_eq!(segments.len(), 1, "Pure digits should be 1 segment");
    assert_eq!(segments[0].script, Script::Common);
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Emoji shaping (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn emoji_single_codepoint_produces_glyphs() {
    let result = shape_text("\u{1F600}"); // 😀
    assert!(
        result.num_glyphs() >= 1,
        "Emoji should produce at least 1 glyph, got {}",
        result.num_glyphs()
    );
}

#[test]
fn emoji_has_positive_width() {
    let result = shape_text("\u{1F600}"); // 😀
    assert!(
        result.width() > 0.0,
        "Emoji should have positive width: {}",
        result.width()
    );
}

#[test]
fn emoji_flag_does_not_crash() {
    // Flag emoji: U+1F1FA U+1F1F8 (🇺🇸). May or may not be supported as a
    // single glyph; the test just verifies no crash and some output.
    let result = shape_text("\u{1F1FA}\u{1F1F8}");
    assert!(result.width() > 0.0, "Flag emoji should have positive width");
    assert!(result.num_characters > 0);
}

#[test]
fn emoji_zwj_sequence_does_not_crash() {
    // Family emoji: 👨‍👩‍👧 (U+1F468 ZWJ U+1F469 ZWJ U+1F467)
    let result = shape_text("\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}");
    assert!(result.width() > 0.0, "ZWJ sequence should have positive width");
    assert!(result.num_glyphs() >= 1, "ZWJ sequence should produce glyphs");
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Whitespace shaping (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn whitespace_space_has_positive_advance() {
    let result = shape_text(" ");
    assert!(result.width() > 0.0, "Space should have positive width");
    // Check advances in the run.
    for run in &result.runs {
        for &adv in &run.advances {
            assert!(adv > 0.0, "Space advance should be positive: {}", adv);
        }
    }
}

#[test]
fn whitespace_tab_has_advance() {
    let result = shape_text("\t");
    // Tab should have non-negative width (may be zero in some shapers).
    assert!(result.width() >= 0.0, "Tab width should be non-negative: {}", result.width());
    assert_eq!(result.num_characters, 1);
}

#[test]
fn whitespace_multiple_spaces_proportional() {
    let one = shape_text(" ");
    let three = shape_text("   ");
    let five = shape_text("     ");
    // 3 spaces should be roughly 3x one space.
    let ratio_3 = three.width() / one.width();
    assert!(
        (ratio_3 - 3.0).abs() < 0.5,
        "3 spaces should be ~3x one space, ratio={}",
        ratio_3
    );
    // 5 spaces should be roughly 5x one space.
    let ratio_5 = five.width() / one.width();
    assert!(
        (ratio_5 - 5.0).abs() < 0.5,
        "5 spaces should be ~5x one space, ratio={}",
        ratio_5
    );
}

#[test]
fn whitespace_em_space_wider_than_normal() {
    let normal = shape_text(" "); // U+0020
    let em = shape_text("\u{2003}"); // Em space
    assert!(
        em.width() >= normal.width(),
        "Em space ({}) should be >= normal space ({})",
        em.width(),
        normal.width()
    );
}

#[test]
fn whitespace_nbsp_similar_to_normal_space() {
    let normal = shape_text(" "); // U+0020
    let nbsp = shape_text("\u{00A0}"); // Non-breaking space
    let diff = (normal.width() - nbsp.width()).abs();
    assert!(
        diff < normal.width() * 0.5,
        "NBSP ({}) should be similar width to space ({}), diff={}",
        nbsp.width(),
        normal.width(),
        diff
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 5. ShapeResult operations (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn shape_result_num_glyphs_matches_run_sum() {
    let result = shape_text("Hello World");
    let run_sum: usize = result.runs.iter().map(|r| r.num_glyphs).sum();
    assert_eq!(
        result.num_glyphs(),
        run_sum,
        "num_glyphs() ({}) should match sum of run glyphs ({})",
        result.num_glyphs(),
        run_sum
    );
}

#[test]
fn shape_result_character_data_length_matches_num_characters() {
    let result = shape_text("The quick brown fox");
    assert_eq!(
        result.character_data.len(),
        result.num_characters,
        "character_data.len() ({}) should match num_characters ({})",
        result.character_data.len(),
        result.num_characters
    );
}

#[test]
fn shape_result_advances_sum_near_total_width() {
    let result = shape_text("Hello World");
    let sum: f32 = result.runs.iter().flat_map(|r| &r.advances).sum();
    let diff = (sum - result.width()).abs();
    assert!(
        diff < 0.5,
        "Sum of advances ({}) should be near width ({}), diff={}",
        sum,
        result.width(),
        diff
    );
}

#[test]
fn shape_result_to_text_blob_some_for_nonempty() {
    let result = shape_text("Test blob");
    let blob = result.to_text_blob();
    assert!(blob.is_some(), "Non-empty ShapeResult should produce a TextBlob");
}

// ═══════════════════════════════════════════════════════════════════════
// 6. Empty/edge case shaping (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn edge_empty_string_zeros() {
    let result = shape_text("");
    assert_eq!(result.width(), 0.0);
    assert_eq!(result.num_glyphs(), 0);
    assert_eq!(result.num_characters, 0);
    assert!(result.runs.is_empty());
    assert!(result.character_data.is_empty());
}

#[test]
fn edge_single_char_one_glyph_one_character() {
    let result = shape_text("X");
    assert_eq!(result.num_characters, 1);
    assert!(
        result.num_glyphs() >= 1,
        "Single char should produce >= 1 glyph"
    );
    assert_eq!(result.character_data.len(), 1);
}

#[test]
fn edge_very_long_string_completes() {
    let long = "a".repeat(5000);
    let result = shape_text(&long);
    assert!(result.width() > 0.0, "5000-char string should have positive width");
    assert_eq!(result.num_characters, 5000);
    assert!(result.num_glyphs() >= 5000);
}

#[test]
fn edge_repeated_chars_proportional_width() {
    let one = shape_text("m");
    let ten = shape_text("mmmmmmmmmm");
    let ratio = ten.width() / one.width();
    assert!(
        (ratio - 10.0).abs() < 0.5,
        "10x 'm' width ratio should be ~10.0, got {}",
        ratio
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Direction-specific shaping (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn direction_rtl_result_is_rtl() {
    let result = shape_rtl("Hello");
    assert_eq!(result.direction, TextDirection::Rtl);
}

#[test]
fn direction_ltr_result_is_ltr() {
    let result = shape_text("Hello");
    assert_eq!(result.direction, TextDirection::Ltr);
}

#[test]
fn direction_arabic_rtl_positive_width() {
    let result = shape_rtl("مرحبا");
    assert!(
        result.width() > 0.0,
        "Arabic RTL should have positive width: {}",
        result.width()
    );
    assert_eq!(result.direction, TextDirection::Rtl);
}

#[test]
fn direction_same_text_both_directions_positive() {
    let ltr = shape_text("Hello");
    let rtl = shape_rtl("Hello");
    assert!(ltr.width() > 0.0);
    assert!(rtl.width() > 0.0);
    // Widths may differ slightly due to different shaping paths.
    let diff = (ltr.width() - rtl.width()).abs();
    assert!(
        diff < ltr.width() * 0.5,
        "LTR ({}) and RTL ({}) widths should be in the same ballpark, diff={}",
        ltr.width(),
        rtl.width(),
        diff
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Script segmenter edge cases (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn segmenter_latin_with_digits_single_segment() {
    let segments = RunSegmenter::segment("abc123def");
    // Digits are Common script and should merge with surrounding Latin.
    assert_eq!(
        segments.len(),
        1,
        "Latin + digits should be 1 segment, got {}",
        segments.len()
    );
    assert_eq!(segments[0].script, Script::Latin);
}

#[test]
fn segmenter_punctuation_between_latin_single_segment() {
    let segments = RunSegmenter::segment("Hello, World!");
    assert_eq!(
        segments.len(),
        1,
        "Latin + punctuation should be 1 segment, got {}",
        segments.len()
    );
    assert_eq!(segments[0].script, Script::Latin);
}

#[test]
fn segmenter_empty_string_no_segments() {
    let segments = RunSegmenter::segment("");
    assert!(segments.is_empty(), "Empty string should produce no segments");
}

#[test]
fn segmenter_all_common_chars_single_segment() {
    // Digits and spaces are all Script::Common.
    let segments = RunSegmenter::segment("123 456 789");
    assert_eq!(
        segments.len(),
        1,
        "All-Common chars should be 1 segment, got {}",
        segments.len()
    );
    assert_eq!(segments[0].script, Script::Common);
}

// ═══════════════════════════════════════════════════════════════════════
// 9. Additional coverage (bonus tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn empty_shape_result_factory() {
    let empty = ShapeResult::empty(TextDirection::Ltr);
    assert_eq!(empty.width(), 0.0);
    assert_eq!(empty.num_glyphs(), 0);
    assert_eq!(empty.num_characters, 0);
    assert_eq!(empty.direction, TextDirection::Ltr);
    assert!(empty.runs.is_empty());
    assert!(empty.character_data.is_empty());
}

#[test]
fn empty_shape_result_rtl_direction() {
    let empty = ShapeResult::empty(TextDirection::Rtl);
    assert_eq!(empty.direction, TextDirection::Rtl);
    assert_eq!(empty.width(), 0.0);
}

#[test]
fn text_direction_is_ltr_helper() {
    assert!(TextDirection::Ltr.is_ltr());
    assert!(!TextDirection::Ltr.is_rtl());
}

#[test]
fn text_direction_is_rtl_helper() {
    assert!(TextDirection::Rtl.is_rtl());
    assert!(!TextDirection::Rtl.is_ltr());
}

#[test]
fn shape_result_empty_to_text_blob_is_none() {
    let empty = ShapeResult::empty(TextDirection::Ltr);
    assert!(empty.to_text_blob().is_none());
}
