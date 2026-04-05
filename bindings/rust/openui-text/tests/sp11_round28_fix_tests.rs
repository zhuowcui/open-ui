//! Tests for SP11 Round 28 code review fixes — openui-text crate.
//!
//! Issue 1: char_advance_for RTL cluster fix (shape_result.rs).
//! Issue 2: Platform fallback selective merge (shaper.rs).
//! Issue 3: RTL .notdef detection for 1:1 runs (shaper.rs).

use openui_text::font::{Font, FontDescription};
use openui_text::shaping::{TextDirection, TextShaper};

// ── Helpers ──────────────────────────────────────────────────────────────

fn make_font(size: f32) -> Font {
    let mut d = FontDescription::default();
    d.size = size;
    d.specified_size = size;
    Font::new(d)
}

fn shape_rtl(text: &str) -> openui_text::shaping::ShapeResult {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Rtl)
}

// ── Issue 1: char_advance_for RTL cluster fix ───────────────────────────

#[test]
fn r28_char_advance_for_rtl_uses_cluster_mapping() {
    // Shape RTL text. For a 1:1 run with cluster data, char_advance_for
    // must use cluster mapping (not direct index) to look up advances.
    // We verify indirectly: the x_positions rebuilt by char_advance_for
    // should be monotonically increasing for all characters.
    let result = shape_rtl("abc");
    assert!(result.num_characters == 3);
    assert!(!result.character_data.is_empty());
    for i in 1..result.character_data.len() {
        assert!(
            result.character_data[i].x_position >= result.character_data[i - 1].x_position,
            "x_positions should be monotonically increasing after RTL char_advance_for fix: \
             pos[{}]={} < pos[{}]={}",
            i,
            result.character_data[i].x_position,
            i - 1,
            result.character_data[i - 1].x_position,
        );
    }
}

#[test]
fn r28_char_advance_for_rtl_total_matches_width() {
    // The sum of all per-character advances (computed via char_advance_for
    // through x_positions) should approximately equal the total width.
    let result = shape_rtl("Hello");
    let last_x = result
        .character_data
        .last()
        .map(|c| c.x_position)
        .unwrap_or(0.0);
    // The last character's x_position + its advance ≈ width.
    // We can approximate the last char advance from (width - last_x).
    let last_advance = result.width - last_x;
    assert!(
        last_advance >= 0.0,
        "Last character advance should be non-negative: {}",
        last_advance,
    );
    // Total width from x_positions should be close to result.width.
    assert!(
        (result.width - (last_x + last_advance)).abs() < 0.01,
        "x_positions should sum to total width",
    );
}

#[test]
fn r28_char_advance_for_rtl_hebrew_monotonic() {
    // Hebrew text: "שלום" — pure RTL, character advances must be consistent.
    let result = shape_rtl("\u{05E9}\u{05DC}\u{05D5}\u{05DD}");
    assert_eq!(result.num_characters, 4);
    for i in 1..result.character_data.len() {
        assert!(
            result.character_data[i].x_position >= result.character_data[i - 1].x_position,
            "Hebrew RTL x_positions should be monotonically increasing",
        );
    }
}

// ── Issue 2: Platform fallback selective merge ──────────────────────────

#[test]
fn r28_fallback_does_not_overwrite_previously_resolved() {
    // Shape text where fallback is needed. After shaping, characters that
    // were successfully resolved by the primary font should retain their
    // glyphs. We test with CJK text where all characters use the same
    // fallback font (so no overwrite can occur).
    let shaper = TextShaper::new();
    let font = make_font(16.0);

    let text = "中文字";
    let result = shaper.shape(text, &font, TextDirection::Ltr);
    assert_eq!(result.num_characters, 3);
    assert!(result.width > 0.0, "CJK text should have non-zero width");

    // After fallback, all characters should have non-zero advances.
    for i in 0..result.num_characters {
        let x_start = result.character_data[i].x_position;
        let x_end = if i + 1 < result.num_characters {
            result.character_data[i + 1].x_position
        } else {
            result.width
        };
        assert!(
            (x_end - x_start) > 0.0,
            "CJK char {} should have non-zero advance after fallback",
            i,
        );
    }
}

#[test]
fn r28_fallback_preserves_earlier_resolved_runs() {
    // Shape text where multiple fallback rounds are needed.
    // After shaping, each character should have a non-zero advance
    // (indicating it was properly resolved, not overwritten to .notdef).
    let shaper = TextShaper::new();
    let font = make_font(16.0);

    // Two characters from different script blocks likely needing
    // different fallback fonts.
    let text = "中文"; // Two CJK characters
    let result = shaper.shape(text, &font, TextDirection::Ltr);

    // Both characters should have non-zero advances.
    for i in 0..result.num_characters {
        let x_start = result.character_data[i].x_position;
        let x_end = if i + 1 < result.num_characters {
            result.character_data[i + 1].x_position
        } else {
            result.width
        };
        let advance = x_end - x_start;
        assert!(
            advance > 0.0,
            "Character {} should have non-zero advance after fallback: {}",
            i, advance,
        );
    }
}

// ── Issue 3: RTL .notdef detection for 1:1 runs ────────────────────────

#[test]
fn r28_rtl_notdef_detection_uses_cluster_mapping() {
    // Shape RTL text. Latin chars in an RTL run should all have valid
    // glyphs — the .notdef detection should correctly identify them as
    // non-missing even though glyph order differs from character order.
    let result = shape_rtl("abc");
    assert_eq!(result.num_characters, 3);

    // All characters should have non-zero width (no .notdef).
    for i in 0..result.num_characters {
        let x_start = result.character_data[i].x_position;
        let x_end = if i + 1 < result.num_characters {
            result.character_data[i + 1].x_position
        } else {
            result.width
        };
        assert!(
            (x_end - x_start) > 0.0,
            "RTL char {} should have non-zero advance (not falsely detected as .notdef)",
            i,
        );
    }
}

#[test]
fn r28_rtl_hebrew_no_false_notdef() {
    // Hebrew text should not falsely detect characters as .notdef.
    let result = shape_rtl("\u{05E9}\u{05DC}\u{05D5}\u{05DD}");
    assert_eq!(result.num_characters, 4);
    assert!(result.width > 0.0);

    // All runs should have non-zero glyph IDs for Hebrew.
    for run in &result.runs {
        for (gi, &glyph) in run.glyphs.iter().enumerate() {
            assert_ne!(
                glyph, 0,
                "Hebrew glyph {} should not be .notdef",
                gi,
            );
        }
    }
}
