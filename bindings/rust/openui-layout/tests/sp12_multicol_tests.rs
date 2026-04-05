//! SP12 G2 — Multi-column layout integration tests.
//!
//! Tests for `resolve_column_count_and_width()`, `compute_column_positions()`,
//! `balance_columns()`, `layout_columns()`, column rule positioning, and
//! edge cases per CSS Multi-column Layout Module Level 1.

use openui_geometry::LayoutUnit;
use openui_dom::NodeId;
use openui_style::ColumnFill;

use openui_layout::multicol::{
    ColumnLayoutAlgorithm,
    resolve_column_count_and_width, compute_column_positions,
    compute_column_rule_positions, balance_columns, layout_columns,
};

// ── Helper ──────────────────────────────────────────────────────────────

fn lu(px: i32) -> LayoutUnit {
    LayoutUnit::from_i32(px)
}

fn node() -> NodeId {
    NodeId::NONE
}

fn algo(count: u32, gap: i32, fill: ColumnFill) -> ColumnLayoutAlgorithm {
    ColumnLayoutAlgorithm {
        column_count: count,
        column_width: None,
        column_gap: lu(gap),
        column_fill: fill,
        column_rule: None,
    }
}

// ── Resolve: only column-count specified ────────────────────────────────

#[test]
fn resolve_only_count_3_columns() {
    let r = resolve_column_count_and_width(Some(3), None, lu(900), lu(20));
    assert_eq!(r.count, 3);
    // width = (900 - 2*20) / 3 = 860 / 3 ≈ 286 (in LayoutUnit raw)
    let expected_width = LayoutUnit::from_raw((lu(900) - lu(40)).raw() / 3);
    assert_eq!(r.width, expected_width);
}

#[test]
fn resolve_only_count_1_column() {
    let r = resolve_column_count_and_width(Some(1), None, lu(600), lu(10));
    assert_eq!(r.count, 1);
    assert_eq!(r.width, lu(600));
}

// ── Resolve: only column-width specified ────────────────────────────────

#[test]
fn resolve_only_width_200px() {
    let r = resolve_column_count_and_width(None, Some(lu(200)), lu(900), lu(20));
    // count = floor((900+20)/(200+20)) = floor(920/220) = 4
    assert_eq!(r.count, 4);
    // Width redistributed to fill available space.
    let expected_width = LayoutUnit::from_raw((lu(900) - lu(60)).raw() / 4);
    assert_eq!(r.width, expected_width);
}

// ── Resolve: both specified (count wins when narrower) ──────────────────

#[test]
fn resolve_both_count_wins() {
    // column-count: 2, column-width: 300px, available: 900px, gap: 20px
    // Fitting by width: floor((900+20)/(300+20)) = floor(920/320) = 2
    // min(2, 2) = 2 → count wins
    let r = resolve_column_count_and_width(Some(2), Some(lu(300)), lu(900), lu(20));
    assert_eq!(r.count, 2);
}

// ── Resolve: both specified (width wins when wider) ─────────────────────

#[test]
fn resolve_both_width_wins() {
    // column-count: 5, column-width: 400px, available: 900px, gap: 20px
    // Fitting by width: floor((900+20)/(400+20)) = floor(920/420) = 2
    // min(5, 2) = 2 → width constraint dominates
    let r = resolve_column_count_and_width(Some(5), Some(lu(400)), lu(900), lu(20));
    assert_eq!(r.count, 2);
}

// ── Resolve: auto/auto defaults ─────────────────────────────────────────

#[test]
fn resolve_auto_auto_defaults_to_1_column() {
    let r = resolve_column_count_and_width(None, None, lu(600), lu(10));
    assert_eq!(r.count, 1);
    assert_eq!(r.width, lu(600));
}

// ── Column gap affects width calculation ────────────────────────────────

#[test]
fn gap_affects_width() {
    let r_no_gap = resolve_column_count_and_width(Some(3), None, lu(900), lu(0));
    let r_with_gap = resolve_column_count_and_width(Some(3), None, lu(900), lu(30));
    // Without gap: 900/3 = 300. With gap: (900 - 60)/3 = 280.
    assert_eq!(r_no_gap.width, lu(300));
    let expected = LayoutUnit::from_raw((lu(900) - lu(60)).raw() / 3);
    assert_eq!(r_with_gap.width, expected);
}

// ── Column positions with 3 columns ────────────────────────────────────

#[test]
fn column_positions_3_columns() {
    let positions = compute_column_positions(3, lu(200), lu(20), lu(660), false);
    assert_eq!(positions.len(), 3);
    assert_eq!(positions[0].inline_offset, lu(0));
    assert_eq!(positions[0].width, lu(200));
    assert_eq!(positions[1].inline_offset, lu(220)); // 200 + 20
    assert_eq!(positions[2].inline_offset, lu(440)); // 200 + 20 + 200 + 20
}

// ── Column positions with gap ───────────────────────────────────────────

#[test]
fn column_positions_with_gap() {
    let positions = compute_column_positions(2, lu(300), lu(50), lu(650), false);
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0].inline_offset, lu(0));
    assert_eq!(positions[1].inline_offset, lu(350)); // 300 + 50
}

// ── Balance: equal content divides evenly ───────────────────────────────

#[test]
fn balance_equal_content() {
    let children = vec![lu(100), lu(100), lu(100)];
    let h = balance_columns(&children, 3, lu(1000));
    assert_eq!(h, lu(100));
}

// ── Balance: uneven content finds minimum height ────────────────────────

#[test]
fn balance_uneven_content() {
    // Total = 230. 2 columns. Greedy: col1 = 50+80=130, col2 = 60+40=100 → 130
    let children = vec![lu(50), lu(80), lu(60), lu(40)];
    let h = balance_columns(&children, 2, lu(1000));
    assert_eq!(h, lu(130));
}

// ── Auto fill: columns use fragmentainer height ─────────────────────────

#[test]
fn auto_fill_uses_available_height() {
    let a = ColumnLayoutAlgorithm {
        column_count: 3,
        column_width: None,
        column_gap: lu(10),
        column_fill: ColumnFill::Auto,
        column_rule: None,
    };
    let result = layout_columns(&a, node(), lu(640), lu(500), &[lu(200), lu(300), lu(150)]);
    // Auto fill: should use 500 as column height.
    assert_eq!(result.block_size, lu(500));
}

// ── Column span all (placeholder) ───────────────────────────────────────

#[test]
fn column_span_all_placeholder() {
    // Column spanning is a style property; verify the enum exists and defaults.
    let style = openui_style::ComputedStyle::initial();
    assert_eq!(style.column_span, openui_style::ColumnSpan::None);
}

// ── Column count = 1 (no columns, pass-through) ────────────────────────

#[test]
fn single_column_passthrough() {
    let r = resolve_column_count_and_width(Some(1), None, lu(800), lu(20));
    assert_eq!(r.count, 1);
    assert_eq!(r.width, lu(800));
}

// ── Negative column-width clamped to 0 ─────────────────────────────────

#[test]
fn negative_column_width_clamped() {
    let r = resolve_column_count_and_width(None, Some(lu(-100)), lu(600), lu(10));
    // Negative width → clamped to 0. count = floor((600+10)/(0+10)) = 61.
    // Width redistributed.
    assert!(r.count >= 1);
    assert!(r.width.raw() >= 0);
}

// ── Zero column-gap ─────────────────────────────────────────────────────

#[test]
fn zero_column_gap() {
    let r = resolve_column_count_and_width(Some(4), None, lu(800), lu(0));
    assert_eq!(r.count, 4);
    assert_eq!(r.width, lu(200)); // 800/4 = 200
}

// ── Large column-count with narrow container ────────────────────────────

#[test]
fn large_count_narrow_container() {
    // column-count: 100, available: 50px, gap: 0
    let r = resolve_column_count_and_width(Some(100), None, lu(50), lu(0));
    assert_eq!(r.count, 100);
    // Width: 50/100 = 0.5 → in LayoutUnit from_raw(50*64/100) = from_raw(32)
    // which is 0.5 px in LayoutUnit.
    assert!(r.width.raw() > 0 || r.count > 0); // doesn't crash
}

// ── Column rule positioning between columns ─────────────────────────────

#[test]
fn column_rule_positions() {
    let rules = compute_column_rule_positions(3, lu(200), lu(20));
    assert_eq!(rules.len(), 2);
    // Rule between col 0 and col 1: at 200 + 10 = 210
    assert_eq!(rules[0], lu(200) + LayoutUnit::from_raw(lu(20).raw() / 2));
    // Rule between col 1 and col 2: at 200 + 20 + 200 + 10 = 430
    let stride = lu(200) + lu(20);
    let half_gap = LayoutUnit::from_raw(lu(20).raw() / 2);
    assert_eq!(rules[1], lu(200) + stride + half_gap);
}

// ── Additional tests ────────────────────────────────────────────────────

#[test]
fn balance_single_column() {
    // column_count=1: should return total height.
    let children = vec![lu(100), lu(200)];
    let h = balance_columns(&children, 1, lu(1000));
    assert_eq!(h, lu(300));
}

#[test]
fn balance_empty_content() {
    let h = balance_columns(&[], 3, lu(1000));
    assert_eq!(h, lu(0));
}

#[test]
fn column_positions_centered() {
    // 2 columns of width 100, gap 20 → total 220. Available 400.
    // Centering offset = (400 - 220) / 2 = 90.
    let positions = compute_column_positions(2, lu(100), lu(20), lu(400), true);
    assert_eq!(positions.len(), 2);
    assert_eq!(positions[0].inline_offset, lu(90));
    assert_eq!(positions[1].inline_offset, lu(90) + lu(120)); // 90 + 100 + 20
}

#[test]
fn column_positions_zero_count() {
    let positions = compute_column_positions(0, lu(100), lu(20), lu(400), false);
    assert!(positions.is_empty());
}

#[test]
fn layout_columns_distributes_across_columns() {
    let a = algo(3, 10, ColumnFill::Balance);
    let children = vec![lu(100), lu(100), lu(100)];
    let result = layout_columns(&a, node(), lu(640), lu(500), &children);
    // 3 equal blocks into 3 columns → balanced at 100.
    assert_eq!(result.block_size, lu(100));
    assert!(!result.column_fragments.is_empty());
}

#[test]
fn column_rule_no_rules_for_single_column() {
    let rules = compute_column_rule_positions(1, lu(200), lu(20));
    assert!(rules.is_empty());
}

#[test]
fn resolve_both_with_zero_gap() {
    let r = resolve_column_count_and_width(Some(3), Some(lu(150)), lu(600), lu(0));
    // Fitting: floor((600+0)/(150+0)) = 4. min(3, 4) = 3.
    assert_eq!(r.count, 3);
    assert_eq!(r.width, lu(200)); // 600/3 = 200
}

#[test]
fn style_defaults_no_multicol() {
    let style = openui_style::ComputedStyle::initial();
    assert!(style.column_count.is_none());
    assert!(style.column_width.is_none());
    assert_eq!(style.column_fill, openui_style::ColumnFill::Balance);
    assert_eq!(style.column_span, openui_style::ColumnSpan::None);
}
