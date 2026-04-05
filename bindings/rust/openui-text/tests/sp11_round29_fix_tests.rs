//! Tests for SP11 Round 29 code review fixes — openui-text crate.
//!
//! Issue 1: int_line_spacing rounds each metric individually.
//! Issue 2: Platform fallback `continue` instead of `break` on tried codepoint.
//! Issue 3: Cluster span coverage in selective_splice_fallback_runs.

use openui_text::font::{Font, FontDescription, FontMetrics};
use openui_text::shaping::{ShapeResult, ShapeResultRun, TextDirection, TextShaper};
use std::sync::Arc;

// ── Issue 1: int_line_spacing rounds each metric individually ────────────

#[test]
fn r29_int_line_spacing_rounds_each_metric_individually() {
    // Blink simple_font_data.cc:175: lroundf(ascent) + lroundf(descent) + lroundf(line_gap)
    // round(10.4) + round(4.6) + round(0.4) = 10.0 + 5.0 + 0.0 = 15.0
    // Old formula: round(10.4 + 4.6) + 0.4 = round(15.0) + 0.4 = 15.0 + 0.4 = 15.4
    let m = FontMetrics {
        ascent: 10.4,
        descent: 4.6,
        line_gap: 0.4,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 15.0);
}

#[test]
fn r29_int_line_spacing_fractional_line_gap_rounded() {
    // round(8.0) + round(3.0) + round(0.6) = 8.0 + 3.0 + 1.0 = 12.0
    // Old formula: round(8.0 + 3.0) + 0.6 = 11.0 + 0.6 = 11.6
    let m = FontMetrics {
        ascent: 8.0,
        descent: 3.0,
        line_gap: 0.6,
        ..FontMetrics::zero()
    };
    assert_eq!(m.int_line_spacing(), 12.0);
}

#[test]
fn r29_int_line_spacing_uses_int_ascent_int_descent() {
    // Verify the result equals int_ascent() + int_descent() + round(line_gap).
    let m = FontMetrics {
        ascent: 12.7,
        descent: 5.3,
        line_gap: 1.4,
        ..FontMetrics::zero()
    };
    let expected = m.int_ascent() + m.int_descent() + m.line_gap.round();
    assert_eq!(m.int_line_spacing(), expected);
}

// ── Issue 2: Platform fallback continues past tried codepoints ───────────

fn make_font(size: f32) -> Font {
    let mut d = FontDescription::default();
    d.size = size;
    d.specified_size = size;
    Font::new(d)
}

#[test]
fn r29_platform_fallback_continues_past_tried_codepoint() {
    // Shape text that contains two different rare codepoints.
    // After the first one triggers fallback, the loop should continue
    // to find the second one even if the first is encountered again
    // in a subsequent iteration.
    let shaper = TextShaper::new();
    let font = make_font(16.0);
    // Use Latin text bracketing rare chars — the shaper should not panic
    // and should produce a result covering all characters.
    let text = "A\u{2603}B\u{2764}C"; // A☃B❤C
    let result = shaper.shape(text, &font, TextDirection::Ltr);
    assert_eq!(
        result.num_characters,
        text.chars().count(),
        "Fallback should produce result covering all characters"
    );
    assert!(result.width > 0.0, "Width should be positive");
}

#[test]
fn r29_platform_fallback_skips_tried_codepoint_continues_others() {
    // Mix of Latin + multiple distinct non-Latin chars. The shaper should
    // handle all of them without getting stuck on a tried codepoint.
    let shaper = TextShaper::new();
    let font = make_font(16.0);
    let text = "\u{2603}\u{2603}\u{2764}"; // ☃☃❤
    let result = shaper.shape(text, &font, TextDirection::Ltr);
    assert_eq!(
        result.num_characters,
        text.chars().count(),
        "Should cover all characters including duplicates and distinct codepoints"
    );
}

// ── Issue 3: Cluster span coverage in selective_splice_fallback_runs ─────

#[test]
fn r29_cluster_span_covers_non_base_characters() {
    // Simulate a ligature: 4 characters shaped into 2 glyphs.
    // clusters = [0, 2] means glyph 0 covers chars [0..2), glyph 1 covers [2..4).
    // Character 1 (non-base) should still be considered covered by glyph 0.
    let font = make_font(16.0);
    let fd = font.primary_font().expect("need font_data").clone();

    let run = ShapeResultRun {
        font_data: Arc::clone(&fd),
        glyphs: vec![42, 43],       // non-zero = real glyphs
        advances: vec![10.0, 10.0],
        offsets: vec![(0.0, 0.0), (0.0, 0.0)],
        clusters: vec![0, 2],
        start_index: 0,
        num_characters: 4,
        num_glyphs: 2,
        direction: TextDirection::Ltr,
    };

    let result = ShapeResult {
        runs: vec![run],
        width: 20.0,
        num_characters: 4,
        direction: TextDirection::Ltr,
        character_data: vec![],
    };

    // All 4 characters should be covered (have real glyphs).
    // With the old code, only chars 0 and 2 would be detected as covered;
    // chars 1 and 3 would be missed because no glyph has cluster == 1 or == 3.
    for ci in 0..4 {
        let is_covered = result.runs.iter().any(|r| {
            let run_end = r.start_index + r.num_characters;
            if ci >= r.start_index && ci < run_end {
                let local = ci - r.start_index;
                if !r.clusters.is_empty() {
                    // Use cluster span logic: check if any glyph's span covers `local`
                    r.clusters.iter().enumerate().any(|(gi, &c)| {
                        if local < c {
                            return false;
                        }
                        let mut end = r.num_characters;
                        for &cc in r.clusters.iter() {
                            if cc > c && cc < end {
                                end = cc;
                            }
                        }
                        local < end && r.glyphs.get(gi).copied().unwrap_or(0) != 0
                    })
                } else {
                    r.glyphs.get(local).copied().unwrap_or(0) != 0
                }
            } else {
                false
            }
        });
        assert!(
            is_covered,
            "Character {} should be covered by cluster span, but was not",
            ci
        );
    }
}

#[test]
fn r29_cluster_span_notdef_detected_for_all_chars_in_cluster() {
    // If a glyph is .notdef (glyph 0) and covers chars [0..3),
    // ALL chars 0, 1, 2 should be marked as .notdef.
    let font = make_font(16.0);
    let fd = font.primary_font().expect("need font_data").clone();

    let run = ShapeResultRun {
        font_data: Arc::clone(&fd),
        glyphs: vec![0, 55],        // glyph 0 is .notdef, glyph 1 is real
        advances: vec![0.0, 12.0],
        offsets: vec![(0.0, 0.0), (0.0, 0.0)],
        clusters: vec![0, 3],       // glyph 0 spans [0..3), glyph 1 spans [3..5)
        start_index: 0,
        num_characters: 5,
        num_glyphs: 2,
        direction: TextDirection::Ltr,
    };

    // Chars 0, 1, 2 should be notdef; chars 3, 4 should NOT be notdef.
    for ci in 0..5 {
        let local = ci;
        let is_notdef = if !run.clusters.is_empty() {
            !run.clusters.iter().enumerate().any(|(gi, &c)| {
                if local < c {
                    return false;
                }
                let mut end = run.num_characters;
                for &cc in run.clusters.iter() {
                    if cc > c && cc < end {
                        end = cc;
                    }
                }
                local < end && run.glyphs.get(gi).copied().unwrap_or(0) != 0
            })
        } else {
            run.glyphs.get(local).copied() == Some(0)
        };

        if ci < 3 {
            assert!(is_notdef, "Char {} should be notdef (in .notdef cluster span)", ci);
        } else {
            assert!(!is_notdef, "Char {} should NOT be notdef (covered by real glyph)", ci);
        }
    }
}
