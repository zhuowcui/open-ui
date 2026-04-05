//! Tests for SP11 Round 24 code review fixes — openui-text crate.
//!
//! Issue 1: char_advance_from_runs fails to sum duplicate-cluster glyph advances.
//! The fix applies the same dedup + sum pattern as char_advance_for in shape_result.rs.

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
            let uc_idx = unique_clusters.iter().rposition(|&c| c <= i);
            if let Some(uc_idx) = uc_idx {
                let uc = unique_clusters[uc_idx];
                let next_uc = if uc_idx + 1 < unique_clusters.len() {
                    unique_clusters[uc_idx + 1]
                } else {
                    num_characters
                };
                let chars_in_cluster = next_uc - uc;
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

    let total_width: f32 = advances.iter().sum();

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

// ── Issue 1: char_advance_from_runs sums duplicate-cluster glyphs ───────

#[test]
fn r24_char_advance_from_runs_duplicate_clusters_sum_correctly() {
    // Two glyphs with cluster=0 (advances 8 and 4), one glyph with cluster=1 (advance 10).
    // After the fix, char 0 should get (8+4)=12 total, char 1 gets 10.
    let sr = make_shape_result_with_clusters(vec![8.0, 4.0, 10.0], vec![0, 0, 1], 2);

    let w0 = sr.width_for_range(0, 1);
    assert!(
        (w0 - 12.0).abs() < 0.01,
        "Char 0 advance should be 12.0 (sum of cluster 0 glyphs), got {w0}"
    );
    let w1 = sr.width_for_range(1, 2);
    assert!(
        (w1 - 10.0).abs() < 0.01,
        "Char 1 advance should be 10.0, got {w1}"
    );
}

#[test]
fn r24_char_advance_from_runs_three_glyphs_one_cluster() {
    // 3 glyphs all with cluster=0, 1 glyph with cluster=1.
    // Cluster 0 covers chars [0,1), sum=5+3+2=10, per-char=10.
    let sr = make_shape_result_with_clusters(vec![5.0, 3.0, 2.0, 8.0], vec![0, 0, 0, 1], 2);

    let w0 = sr.width_for_range(0, 1);
    assert!(
        (w0 - 10.0).abs() < 0.01,
        "Char 0 advance should be 10.0 (sum of 3 cluster-0 glyphs), got {w0}"
    );
    let w1 = sr.width_for_range(1, 2);
    assert!(
        (w1 - 8.0).abs() < 0.01,
        "Char 1 advance should be 8.0, got {w1}"
    );
}

#[test]
fn r24_char_advance_total_width_preserved_with_duplicates() {
    // Total width should be sum of all advances regardless of cluster mapping.
    let sr = make_shape_result_with_clusters(vec![6.0, 4.0, 3.0, 7.0], vec![0, 0, 1, 1], 2);

    // Total width = 6+4+3+7 = 20
    assert!(
        (sr.width - 20.0).abs() < 0.01,
        "Total width should be 20.0, got {}",
        sr.width
    );
    // Char 0 covers cluster 0: sum=10, char 1 covers cluster 1: sum=10
    let w0 = sr.width_for_range(0, 1);
    let w1 = sr.width_for_range(1, 2);
    assert!(
        (w0 + w1 - 20.0).abs() < 0.01,
        "Sum of per-char widths should equal total: {w0} + {w1} != 20"
    );
}
