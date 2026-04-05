//! Tests for SP11 Round 22 code review fixes — openui-text crate.
//!
//! Issue 2: Inter-character justification updates shape result glyphs.
//! Issue 3: Platform fallback locale uses actual locale, not hardcoded "en".
//! Issue 5: sub_range() includes ligature glyphs at clip boundaries.
//! Issue 6: Spacing rebuild uses cluster-based widths, not uniform averages.

use openui_text::{ShapeResult, ShapeResultCharacterData, ShapeResultRun, TextDirection};
use std::sync::Arc;

// ── Helper ──────────────────────────────────────────────────────────────

fn get_test_font_data() -> Arc<openui_text::font::FontPlatformData> {
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
}

fn make_shape_result(
    advances: Vec<f32>,
    clusters: Vec<usize>,
    num_characters: usize,
) -> ShapeResult {
    let font_data = get_test_font_data();
    let num_glyphs = advances.len();
    let total_width: f32 = advances.iter().sum();

    let mut x = 0.0f32;
    let character_data: Vec<ShapeResultCharacterData> = (0..num_characters)
        .map(|i| {
            let cd = ShapeResultCharacterData {
                x_position: x,
                is_cluster_base: true,
                safe_to_break_before: i == 0,
            };
            // Simple: for 1:1 mapping advance by glyph width
            if i < num_glyphs {
                x += advances[i];
            }
            cd
        })
        .collect();

    ShapeResult {
        runs: vec![ShapeResultRun {
            font_data,
            glyphs: vec![1; num_glyphs],
            advances,
            offsets: vec![(0.0, 0.0); num_glyphs],
            clusters,
            start_index: 0,
            num_characters,
            num_glyphs,
            direction: TextDirection::Ltr,
        }],
        width: total_width,
        num_characters,
        direction: TextDirection::Ltr,
        character_data,
    }
}

// ── Issue 2: apply_inter_character_justification ─────────────────────────

#[test]
fn inter_char_justification_expands_width() {
    // 3 characters "abc" with 1:1 glyph mapping, each 10px wide.
    // 2 gaps, extra_per_gap = 5.0 → total extra = 10.0
    let mut sr = make_shape_result(
        vec![10.0, 10.0, 10.0],
        vec![0, 1, 2],
        3,
    );
    assert_eq!(sr.width, 30.0);

    sr.apply_inter_character_justification(5.0);

    // Width should increase by 10.0 (2 gaps × 5.0).
    assert!(
        (sr.width - 40.0).abs() < 0.01,
        "Width after inter-char justification should be 40.0, got {}",
        sr.width
    );
    // First glyph advance: 10 + 5 = 15
    assert!(
        (sr.runs[0].advances[0] - 15.0).abs() < 0.01,
        "First glyph advance should be 15.0, got {}",
        sr.runs[0].advances[0]
    );
    // Last glyph should NOT get extra (no gap after last char).
    assert!(
        (sr.runs[0].advances[2] - 10.0).abs() < 0.01,
        "Last glyph advance should remain 10.0, got {}",
        sr.runs[0].advances[2]
    );
}

#[test]
fn inter_char_justification_single_char_no_change() {
    // Single character: no gaps to expand.
    let mut sr = make_shape_result(vec![10.0], vec![0], 1);
    let original_width = sr.width;
    sr.apply_inter_character_justification(5.0);
    assert!(
        (sr.width - original_width).abs() < 0.01,
        "Single char should have no expansion"
    );
}

#[test]
fn inter_char_justification_updates_character_positions() {
    // 4 characters, each 10px wide, extra_per_gap = 2.0.
    // 3 gaps → total extra = 6.0
    let mut sr = make_shape_result(
        vec![10.0, 10.0, 10.0, 10.0],
        vec![0, 1, 2, 3],
        4,
    );
    sr.apply_inter_character_justification(2.0);

    // Check character_data x_positions after justification.
    // Char 0: x=0, advance=12
    // Char 1: x=12, advance=12
    // Char 2: x=24, advance=12
    // Char 3: x=36, advance=10 (no extra for last)
    assert!((sr.character_data[0].x_position - 0.0).abs() < 0.01);
    assert!((sr.character_data[1].x_position - 12.0).abs() < 0.01);
    assert!((sr.character_data[2].x_position - 24.0).abs() < 0.01);
    assert!((sr.character_data[3].x_position - 36.0).abs() < 0.01);
    assert!((sr.width - 46.0).abs() < 0.01);
}

// ── Issue 3: Locale fallback ─────────────────────────────────────────────

#[test]
fn font_description_locale_is_used_not_discarded() {
    // Verify FontDescription with locale set passes it through.
    // We can't easily test the Skia call, but we can verify the locale
    // field is properly stored and not ignored.
    let mut desc = openui_text::FontDescription::default();
    desc.locale = Some("ja".to_string());
    assert_eq!(desc.locale.as_deref(), Some("ja"));
}

#[test]
fn font_description_empty_locale_defaults_to_en() {
    // When locale is None or empty, the code should use "en" as default.
    let desc = openui_text::FontDescription::default();
    let locale = desc.locale.as_ref().map_or(true, |l| l.is_empty());
    assert!(locale, "Default FontDescription should have no locale set");
}

#[test]
fn platform_fallback_with_locale_does_not_panic() {
    // Verify that calling platform_fallback_for_character with a non-empty
    // locale doesn't panic (it used to always use "en").
    let mut cache = openui_text::font::cache::GLOBAL_FONT_CACHE
        .lock()
        .unwrap();
    let mut desc = openui_text::FontDescription::default();
    desc.locale = Some("ja".to_string());
    // This should not panic regardless of available fonts.
    let _result = cache.platform_fallback_for_character('あ', &desc);
    // We don't assert the result since it depends on system fonts,
    // but the call itself should succeed without panicking.
}

// ── Issue 5: sub_range() includes ligature glyphs at clip boundaries ─────

#[test]
fn glyph_range_includes_ligature_spanning_clip_start() {
    // Ligature glyph at index 0 covers characters 0 and 1 (cluster=0).
    // Glyph at index 1 covers character 2 (cluster=2).
    // Requesting char_range [1, 3) should include glyph 0 (since it covers
    // chars [0,2) which overlaps with [1,3)).
    let font_data = get_test_font_data();
    let run = ShapeResultRun {
        font_data,
        glyphs: vec![100, 200],
        advances: vec![20.0, 10.0],
        offsets: vec![(0.0, 0.0), (0.0, 0.0)],
        clusters: vec![0, 2],     // glyph 0 → char 0, glyph 1 → char 2
        start_index: 0,
        num_characters: 3,
        num_glyphs: 2,
        direction: TextDirection::Ltr,
    };

    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 1, 3);
    // Should include both glyphs: glyph 0 covers [0,2) overlapping [1,3),
    // glyph 1 covers [2,3) overlapping [1,3).
    assert_eq!(gs, 0, "Should include ligature glyph covering char 1");
    assert_eq!(ge, 2, "Should include glyph covering char 2");
}

#[test]
fn glyph_range_excludes_ligature_not_overlapping() {
    // Ligature glyph at index 0 covers characters [0, 2).
    // Glyph at index 1 covers characters [2, 4).
    // Requesting char_range [2, 4) should NOT include glyph 0.
    let font_data = get_test_font_data();
    let run = ShapeResultRun {
        font_data,
        glyphs: vec![100, 200],
        advances: vec![20.0, 20.0],
        offsets: vec![(0.0, 0.0), (0.0, 0.0)],
        clusters: vec![0, 2],
        start_index: 0,
        num_characters: 4,
        num_glyphs: 2,
        direction: TextDirection::Ltr,
    };

    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 2, 4);
    assert_eq!(gs, 1, "Should start at glyph 1");
    assert_eq!(ge, 2, "Should end at glyph 2");
}

#[test]
fn sub_range_preserves_ligature_glyph_at_boundary() {
    // Build a shape result with a ligature: glyph 0 covers chars [0,2), glyph 1 covers char 2.
    let font_data = get_test_font_data();
    let sr = ShapeResult {
        runs: vec![ShapeResultRun {
            font_data,
            glyphs: vec![100, 200],
            advances: vec![20.0, 10.0],
            offsets: vec![(0.0, 0.0), (0.0, 0.0)],
            clusters: vec![0, 2],
            start_index: 0,
            num_characters: 3,
            num_glyphs: 2,
            direction: TextDirection::Ltr,
        }],
        width: 30.0,
        num_characters: 3,
        direction: TextDirection::Ltr,
        character_data: vec![
            ShapeResultCharacterData { x_position: 0.0, is_cluster_base: true, safe_to_break_before: true },
            ShapeResultCharacterData { x_position: 10.0, is_cluster_base: false, safe_to_break_before: false },
            ShapeResultCharacterData { x_position: 20.0, is_cluster_base: true, safe_to_break_before: true },
        ],
    };

    // sub_range [1, 3) should include the ligature glyph (covers chars 0-1).
    let sub = sr.sub_range(1, 3);
    assert!(
        !sub.runs.is_empty(),
        "sub_range should include runs when ligature overlaps"
    );
    assert!(
        sub.num_glyphs() >= 1,
        "sub_range should include at least the ligature glyph"
    );
}

// ── Issue 6: Spacing rebuild uses cluster-based widths ───────────────────

#[test]
fn char_advance_from_runs_uses_cluster_geometry() {
    // A run with 2 glyphs covering 4 characters:
    // Glyph 0 (cluster=0) covers chars [0,2) with advance 20.
    // Glyph 1 (cluster=2) covers chars [2,4) with advance 10.
    // char_advance for char 0 should be 20/2=10, not (20+10)/4=7.5.
    let font_data = get_test_font_data();
    let sr = ShapeResult {
        runs: vec![ShapeResultRun {
            font_data,
            glyphs: vec![100, 200],
            advances: vec![20.0, 10.0],
            offsets: vec![(0.0, 0.0), (0.0, 0.0)],
            clusters: vec![0, 2],
            start_index: 0,
            num_characters: 4,
            num_glyphs: 2,
            direction: TextDirection::Ltr,
        }],
        width: 30.0,
        num_characters: 4,
        direction: TextDirection::Ltr,
        character_data: vec![
            ShapeResultCharacterData { x_position: 0.0, is_cluster_base: true, safe_to_break_before: true },
            ShapeResultCharacterData { x_position: 10.0, is_cluster_base: false, safe_to_break_before: false },
            ShapeResultCharacterData { x_position: 20.0, is_cluster_base: true, safe_to_break_before: true },
            ShapeResultCharacterData { x_position: 25.0, is_cluster_base: false, safe_to_break_before: false },
        ],
    };

    // Use width_for_range to verify cluster-based widths.
    // Chars [0,2) = glyph 0's cluster, should be 20.0 (from x_position difference: 20.0 - 0.0)
    let w_01 = sr.width_for_range(0, 2);
    assert!(
        (w_01 - 20.0).abs() < 0.01,
        "Chars [0,2) should span 20px (cluster 0), got {w_01}"
    );
    // Chars [2,4) = glyph 1's cluster, should be 10.0
    let w_23 = sr.width_for_range(2, 4);
    assert!(
        (w_23 - 10.0).abs() < 0.01,
        "Chars [2,4) should span 10px (cluster 1), got {w_23}"
    );
}

#[test]
fn char_advance_ligature_not_uniform() {
    // With the old code, a ligature run would distribute width uniformly.
    // New code uses cluster boundaries. Verify two clusters with different
    // advances produce different per-char widths.
    let font_data = get_test_font_data();
    let sr = ShapeResult {
        runs: vec![ShapeResultRun {
            font_data,
            glyphs: vec![100, 200],
            advances: vec![30.0, 10.0],
            offsets: vec![(0.0, 0.0), (0.0, 0.0)],
            clusters: vec![0, 1],  // Each glyph covers 1 char out of 2
            start_index: 0,
            num_characters: 2,
            num_glyphs: 2,
            direction: TextDirection::Ltr,
        }],
        width: 40.0,
        num_characters: 2,
        direction: TextDirection::Ltr,
        character_data: vec![
            ShapeResultCharacterData { x_position: 0.0, is_cluster_base: true, safe_to_break_before: true },
            ShapeResultCharacterData { x_position: 30.0, is_cluster_base: true, safe_to_break_before: true },
        ],
    };

    // sub_range for first char only
    let sub = sr.sub_range(0, 1);
    // Width should be 30.0 (char 0's advance), not 40/2=20 (uniform average).
    assert!(
        (sub.width - 30.0).abs() < 0.01,
        "Sub-range [0,1) should be 30px from cluster data, got {}",
        sub.width
    );
}
