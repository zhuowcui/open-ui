//! Tests for SP11 Round 23 code review fixes — openui-text crate.
//!
//! Issue 1: Ligature/duplicate-cluster handling in shape_result methods.
//!   - glyph_range_for_char_range groups duplicate clusters correctly
//!   - apply_inter_character_justification handles duplicate clusters
//!   - char_advance_for sums advances of duplicate-cluster glyphs

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

fn make_shape_result_with_clusters(
    advances: Vec<f32>,
    clusters: Vec<usize>,
    num_characters: usize,
) -> ShapeResult {
    let font_data = get_test_font_data();
    let num_glyphs = advances.len();
    let total_width: f32 = advances.iter().sum();

    // Build character_data using cluster-aware advance computation.
    // Group glyphs by unique cluster, sum their advances, divide by chars covered.
    let mut unique_clusters: Vec<usize> = clusters.clone();
    unique_clusters.sort();
    unique_clusters.dedup();

    let mut x = 0.0f32;
    let character_data: Vec<ShapeResultCharacterData> = (0..num_characters)
        .map(|i| {
            let cd = ShapeResultCharacterData {
                x_position: x,
                is_cluster_base: true,
                safe_to_break_before: i == 0,
            };
            // Find which cluster group this character belongs to.
            let uc_idx = unique_clusters.iter().rposition(|&c| c <= i);
            if let Some(uc_idx) = uc_idx {
                let uc = unique_clusters[uc_idx];
                let next_uc = if uc_idx + 1 < unique_clusters.len() {
                    unique_clusters[uc_idx + 1]
                } else {
                    num_characters
                };
                let chars_in_cluster = next_uc - uc;
                // Sum advances of glyphs with this cluster value.
                let cluster_adv: f32 = clusters
                    .iter()
                    .enumerate()
                    .filter(|(_, &c)| c == uc)
                    .map(|(gi, _)| advances[gi])
                    .sum();
                x += cluster_adv / chars_in_cluster as f32;
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

// ── Issue 1a: glyph_range_for_char_range with duplicate clusters ────────

#[test]
fn glyph_range_duplicate_clusters_includes_all_glyphs() {
    // Two glyphs with cluster=0 (combining mark scenario), one glyph with cluster=1.
    // Glyphs: [g0 cluster=0, g1 cluster=0, g2 cluster=1]
    // Characters: [char0, char1]
    // Requesting char range [0, 1) should include g0 and g1 (both cluster=0).
    let font_data = get_test_font_data();
    let run = ShapeResultRun {
        font_data,
        glyphs: vec![100, 101, 200],
        advances: vec![8.0, 4.0, 10.0],
        offsets: vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        clusters: vec![0, 0, 1], // g0 and g1 share cluster 0
        start_index: 0,
        num_characters: 2,
        num_glyphs: 3,
        direction: TextDirection::Ltr,
    };

    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 0, 1);
    assert_eq!(gs, 0, "Should start at glyph 0");
    assert_eq!(ge, 2, "Should include both glyphs in cluster 0 (g0 and g1)");
}

#[test]
fn glyph_range_duplicate_clusters_second_cluster() {
    // Three glyphs: [g0 cluster=0, g1 cluster=1, g2 cluster=1]
    // Requesting char range [1, 2) should include g1 and g2.
    let font_data = get_test_font_data();
    let run = ShapeResultRun {
        font_data,
        glyphs: vec![100, 200, 201],
        advances: vec![10.0, 6.0, 4.0],
        offsets: vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        clusters: vec![0, 1, 1], // g1 and g2 share cluster 1
        start_index: 0,
        num_characters: 2,
        num_glyphs: 3,
        direction: TextDirection::Ltr,
    };

    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 1, 2);
    assert_eq!(gs, 1, "Should start at glyph 1");
    assert_eq!(ge, 3, "Should include both glyphs in cluster 1 (g1 and g2)");
}

#[test]
fn glyph_range_duplicate_clusters_not_zero_width() {
    // Previously, duplicate clusters created zero-width intervals [0,0)
    // which dropped the glyph. Verify no glyphs are dropped.
    let font_data = get_test_font_data();
    let run = ShapeResultRun {
        font_data,
        glyphs: vec![10, 11, 20],
        advances: vec![5.0, 5.0, 10.0],
        offsets: vec![(0.0, 0.0), (0.0, 0.0), (0.0, 0.0)],
        clusters: vec![0, 0, 1],
        start_index: 0,
        num_characters: 2,
        num_glyphs: 3,
        direction: TextDirection::Ltr,
    };

    // Full range should include all 3 glyphs.
    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 0, 2);
    assert_eq!(gs, 0);
    assert_eq!(ge, 3, "All 3 glyphs should be included for full range");
}

// ── Issue 1b: apply_inter_character_justification with duplicate clusters ─

#[test]
fn inter_char_justification_duplicate_clusters_no_double_expansion() {
    // 3 characters, 4 glyphs: [g0 cluster=0, g1 cluster=0, g2 cluster=1, g3 cluster=2]
    // Duplicate-cluster glyphs (g0, g1) should not both get extra spacing.
    // 2 gaps (between char 0-1 and char 1-2). extra_per_gap = 5.0.
    let mut sr = make_shape_result_with_clusters(
        vec![8.0, 4.0, 10.0, 10.0],  // g0=8, g1=4, g2=10, g3=10
        vec![0, 0, 1, 2],             // g0,g1 share cluster 0
        3,
    );
    let original_width = sr.width; // 32.0
    assert!((original_width - 32.0).abs() < 0.01);

    sr.apply_inter_character_justification(5.0);

    // 2 gaps × 5.0 = 10.0 extra total.
    let expected_width = 32.0 + 10.0;
    assert!(
        (sr.width - expected_width).abs() < 0.01,
        "Width should be {expected_width}, got {}. Duplicate clusters should not cause double expansion.",
        sr.width
    );
}

#[test]
fn inter_char_justification_ligature_covering_multiple_chars() {
    // Ligature: 1 glyph covers 3 characters (cluster=0 for all 3 chars).
    // Then 1 glyph for char 3 (cluster=3).
    // Total chars = 4, total gaps = 3.
    // The ligature glyph should get extra_per_gap * 2 (for 2 internal gaps)
    // plus 1 boundary gap to the next cluster = extra_per_gap * 3 total? No —
    // the ligature covers chars [0,3), so internal gaps = 2, plus boundary
    // gap after cluster (char 3 exists) = 1. Total = 3 gaps from the ligature glyph.
    // The last glyph (char 3) gets 0 gaps (it's the last char).
    let mut sr = make_shape_result_with_clusters(
        vec![30.0, 10.0],
        vec![0, 3],  // glyph 0 covers chars [0,3), glyph 1 covers char 3
        4,
    );
    let original_width = sr.width; // 40.0
    sr.apply_inter_character_justification(2.0);

    // 3 gaps × 2.0 = 6.0 extra.
    let expected_width = original_width + 6.0;
    assert!(
        (sr.width - expected_width).abs() < 0.01,
        "Ligature should get correct expansion: expected {expected_width}, got {}",
        sr.width
    );
    // The ligature glyph (g0) should have the extra, last glyph (g1) should not.
    assert!(
        (sr.runs[0].advances[0] - (30.0 + 6.0)).abs() < 0.01,
        "Ligature glyph should get all 6.0 extra, got {}",
        sr.runs[0].advances[0]
    );
    assert!(
        (sr.runs[0].advances[1] - 10.0).abs() < 0.01,
        "Last glyph should remain unchanged, got {}",
        sr.runs[0].advances[1]
    );
}

// ── Issue 1c: char_advance_for sums duplicate-cluster glyphs ────────────

#[test]
fn char_advance_sums_duplicate_cluster_glyphs() {
    // 2 glyphs with cluster=0 (advances 8 and 4), 1 glyph with cluster=1 (advance 10).
    // char_advance_for(0) should be (8+4)/1 = 12 (cluster 0 covers 1 char).
    let sr = make_shape_result_with_clusters(
        vec![8.0, 4.0, 10.0],
        vec![0, 0, 1],
        2,
    );
    // Verify via width_for_range (which uses character_data built from char_advance_for).
    // char 0 should have advance 12 (8+4 from cluster 0, covering 1 char).
    let w0 = sr.width_for_range(0, 1);
    assert!(
        (w0 - 12.0).abs() < 0.01,
        "Char 0 advance should be 12.0 (sum of duplicate cluster glyphs), got {w0}"
    );
    let w1 = sr.width_for_range(1, 2);
    assert!(
        (w1 - 10.0).abs() < 0.01,
        "Char 1 advance should be 10.0, got {w1}"
    );
}

#[test]
fn char_advance_sums_three_glyphs_same_cluster() {
    // 3 glyphs all with cluster=0 (decomposed character scenario),
    // 1 glyph with cluster=1.
    // char_advance_for(0) should be (5+3+2)/1 = 10.
    let sr = make_shape_result_with_clusters(
        vec![5.0, 3.0, 2.0, 8.0],
        vec![0, 0, 0, 1],
        2,
    );
    let w0 = sr.width_for_range(0, 1);
    assert!(
        (w0 - 10.0).abs() < 0.01,
        "Char 0 advance should be 10.0 (sum of 3 glyphs in cluster 0), got {w0}"
    );
    let w1 = sr.width_for_range(1, 2);
    assert!(
        (w1 - 8.0).abs() < 0.01,
        "Char 1 advance should be 8.0, got {w1}"
    );
}
