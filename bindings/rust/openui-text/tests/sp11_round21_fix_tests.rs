//! Tests for SP11 Round 21 code review fixes — openui-text crate.
//!
//! Covers Issues 1, 2, 4, 5 from the review.

use openui_text::{FontMetrics, ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection};
use openui_style::TextTransform;

// ── Issue 1: int_line_spacing() rounds sum-of-rounds ─────────────────────

#[test]
fn int_line_spacing_sum_of_rounds_blink_example() {
    // Blink: lroundf(ascent) + lroundf(descent) + lroundf(line_gap) — simple_font_data.cc:175
    // round(10.4) + round(4.4) + round(0.3) = 10.0 + 4.0 + 0.0 = 14.0
    let m = FontMetrics {
        ascent: 10.4,
        descent: 4.4,
        line_gap: 0.3,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 14.0);
}

#[test]
fn int_line_spacing_sum_of_rounds_half_values() {
    // Blink: round(10.5) + round(4.5) + round(0.5) = 11.0 + 5.0 + 1.0 = 17.0
    let m = FontMetrics {
        ascent: 10.5,
        descent: 4.5,
        line_gap: 0.5,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 17.0);
}

#[test]
fn int_line_spacing_already_integer() {
    // No rounding needed → result is same either way.
    let m = FontMetrics {
        ascent: 10.0,
        descent: 4.0,
        line_gap: 2.0,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 16.0);
}

// ── Issue 2: RTL apply_justification trailing space exclusion ────────────

/// Helper to build a minimal ShapeResult for justification testing.
fn make_justification_test_result(
    text: &str,
    direction: TextDirection,
    advances: Vec<f32>,
) -> ShapeResult {
    let chars: Vec<char> = text.chars().collect();
    let num_chars = chars.len();
    let num_glyphs = advances.len();

    // Build clusters: for RTL, glyphs are in reverse order
    let clusters: Vec<usize> = if direction == TextDirection::Rtl {
        (0..num_glyphs).rev().collect()
    } else {
        (0..num_glyphs).collect()
    };

    let font_data = {
        // Use the global font cache to get a valid FontPlatformData
        let mut cache = openui_text::font::cache::GLOBAL_FONT_CACHE
            .lock()
            .unwrap();
        let desc = openui_text::FontDescription::default();
        cache
            .get_font_platform_data("sans-serif", &desc)
            .unwrap_or_else(|| {
                cache
                    .get_font_platform_data("serif", &desc)
                    .expect("need at least one system font")
            })
    };

    let total_width: f32 = advances.iter().sum();

    let mut x = 0.0;
    let character_data: Vec<ShapeResultCharacterData> = (0..num_chars)
        .map(|i| {
            let cd = ShapeResultCharacterData {
                x_position: x,
                is_cluster_base: true,
                safe_to_break_before: i == 0,
            };
            if i < advances.len() {
                x += advances[if direction == TextDirection::Rtl { num_glyphs - 1 - i } else { i }];
            }
            cd
        })
        .collect();

    ShapeResult {
        runs: vec![ShapeResultRun {
            font_data,
            glyphs: vec![1; num_glyphs], // non-zero = real glyphs
            advances,
            offsets: vec![(0.0, 0.0); num_glyphs],
            clusters,
            start_index: 0,
            num_characters: num_chars,
            num_glyphs,
            direction,
        }],
        width: total_width,
        num_characters: num_chars,
        direction,
        character_data,
    }
}

#[test]
fn rtl_justification_trailing_spaces_excluded_correctly() {
    // Text: "a b c " (trailing space at the end logically)
    // RTL: glyphs are stored in reverse order
    // Trailing 1 space (the last ' ' in logical order) should be excluded.
    // There are 2 spaces total; only the first (between 'a' and 'b') should expand.
    let text = "a b c ";
    let advances = vec![10.0, 5.0, 10.0, 5.0, 10.0, 5.0]; // a,sp,b,sp,c,sp
    let mut result = make_justification_test_result(text, TextDirection::Rtl, advances);

    let old_width = result.width;
    result.apply_justification(4.0, text, 1);

    // One trailing space excluded → only 1 expandable space ("a b" has the non-trailing space)
    // Wait: "a b c " has spaces at indices 1, 3, 5. Trailing 1 → index 5 excluded.
    // Expandable spaces at indices 1 and 3 → 2 spaces × 4.0 = 8.0 extra
    assert_eq!(result.width, old_width + 8.0);
}

#[test]
fn ltr_justification_trailing_spaces_excluded_correctly() {
    // Same text but LTR — should behave the same for trailing exclusion.
    let text = "a b c ";
    let advances = vec![10.0, 5.0, 10.0, 5.0, 10.0, 5.0];
    let mut result = make_justification_test_result(text, TextDirection::Ltr, advances);

    let old_width = result.width;
    result.apply_justification(4.0, text, 1);

    // Trailing 1 space excluded → 2 expandable spaces → 8.0 extra
    assert_eq!(result.width, old_width + 8.0);
}

#[test]
fn rtl_justification_exclude_two_trailing() {
    let text = "a b  ";
    let advances = vec![10.0, 5.0, 10.0, 5.0, 5.0]; // a,sp,b,sp,sp
    let mut result = make_justification_test_result(text, TextDirection::Rtl, advances);

    let old_width = result.width;
    result.apply_justification(3.0, text, 2);

    // 3 total spaces (indices 1,3,4). Exclude trailing 2 (indices 4,3).
    // Only space at index 1 expanded: 1 × 3.0 = 3.0
    assert_eq!(result.width, old_width + 3.0);
}

// ── Issue 5: Capitalize ligature characters ──────────────────────────────

#[test]
fn capitalize_ligature_ff() {
    // U+FB00 ﬀ should titlecase to "Ff", not "FF"
    let result = openui_text::apply_text_transform("\u{FB00}oo", TextTransform::Capitalize, None);
    assert_eq!(result, "Ffoo");
}

#[test]
fn capitalize_ligature_fi() {
    let result = openui_text::apply_text_transform("\u{FB01}nd", TextTransform::Capitalize, None);
    assert_eq!(result, "Find");
}

#[test]
fn capitalize_ligature_fl() {
    let result = openui_text::apply_text_transform("\u{FB02}ow", TextTransform::Capitalize, None);
    assert_eq!(result, "Flow");
}

#[test]
fn capitalize_ligature_ffi() {
    let result = openui_text::apply_text_transform("\u{FB03}ce", TextTransform::Capitalize, None);
    assert_eq!(result, "Ffice");
}

#[test]
fn capitalize_ligature_ffl() {
    let result = openui_text::apply_text_transform("\u{FB04}at", TextTransform::Capitalize, None);
    assert_eq!(result, "Fflat");
}

#[test]
fn capitalize_ligature_long_st() {
    let result = openui_text::apply_text_transform("\u{FB05}ar", TextTransform::Capitalize, None);
    assert_eq!(result, "Star");
}

#[test]
fn capitalize_ligature_st() {
    let result = openui_text::apply_text_transform("\u{FB06}ep", TextTransform::Capitalize, None);
    assert_eq!(result, "Step");
}

#[test]
fn capitalize_dutch_ij_digraph() {
    let result = openui_text::apply_text_transform("\u{0133}ssel", TextTransform::Capitalize, None);
    assert_eq!(result, "\u{0132}ssel");
}
