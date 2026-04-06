//! SP12 H8 — CSS Fragmentation and Multi-column Layout comprehensive tests.
//!
//! Tests for break tokens, break appeal ranking, fragmentainer space,
//! break-point selection, multi-column layout, column spanning, nested
//! fragmentation, and edge cases per CSS Fragmentation Module Level 3
//! and CSS Multi-column Layout Module Level 1.

use openui_geometry::LayoutUnit;
use openui_dom::NodeId;
use openui_style::{BreakValue, BreakInside, ColumnFill, ColumnSpan, ComputedStyle};

use openui_layout::fragmentation::{
    BlockBreakToken, BreakAppeal, BreakPoint, BreakToken, ChildBreakInfo,
    FragmentainerSpace, find_best_break_point, should_break_before, should_break_after,
    break_before_appeal, break_after_appeal, join_break_between,
};
use openui_layout::multicol::{
    ColumnLayoutAlgorithm, ColumnRule,
    resolve_column_count_and_width, compute_column_positions,
    compute_column_rule_positions, balance_columns, layout_columns,
};
use openui_layout::BreakBetween;

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: i32) -> LayoutUnit { LayoutUnit::from_i32(px) }
fn node() -> NodeId { NodeId::NONE }

fn child(size: i32) -> ChildBreakInfo {
    ChildBreakInfo {
        break_before: BreakValue::Auto,
        break_after: BreakValue::Auto,
        break_inside: BreakInside::Auto,
        block_size: lu(size),
    }
}

fn algo(count: u32, gap: i32, fill: ColumnFill) -> ColumnLayoutAlgorithm {
    ColumnLayoutAlgorithm {
        column_count: count, column_width: None,
        column_gap: lu(gap), column_fill: fill, column_rule: None,
    }
}

fn algo_with_width(count: u32, width: Option<i32>, gap: i32, fill: ColumnFill) -> ColumnLayoutAlgorithm {
    ColumnLayoutAlgorithm {
        column_count: count, column_width: width.map(lu),
        column_gap: lu(gap), column_fill: fill, column_rule: None,
    }
}

// =========================================================================
// 1. BASIC FRAGMENTATION
// =========================================================================

#[test] fn basic_frag_content_fits_single_fragmentainer() {
    let r = find_best_break_point(&[child(100), child(100)], &FragmentainerSpace::new(lu(300)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Default);
}
#[test] fn basic_frag_single_child_fits() {
    assert_eq!(find_best_break_point(&[child(50)], &FragmentainerSpace::new(lu(100))).child_index, 1);
}
#[test] fn basic_frag_exactly_fills() {
    let r = find_best_break_point(&[child(200), child(300)], &FragmentainerSpace::new(lu(500)));
    assert_eq!(r.child_index, 2);
}
#[test] fn basic_frag_exceeds_fragmentainer() {
    let r = find_best_break_point(&[child(200), child(200), child(200)], &FragmentainerSpace::new(lu(500)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Default);
}
#[test] fn basic_frag_break_between_siblings() {
    assert_eq!(find_best_break_point(&[child(300), child(300)], &FragmentainerSpace::new(lu(400))).child_index, 1);
}
#[test] fn basic_frag_token_child_index() {
    assert_eq!(BlockBreakToken::new(3, lu(250)).child_index, 3);
}
#[test] fn basic_frag_token_consumed() {
    assert_eq!(BlockBreakToken::new(1, lu(400)).consumed_block_size, lu(400));
}
#[test] fn basic_frag_multiple_fragmentainers() {
    assert_eq!(find_best_break_point(&vec![child(100); 5], &FragmentainerSpace::new(lu(200))).child_index, 2);
}
#[test] fn basic_frag_empty_no_break() {
    let r = find_best_break_point(&[], &FragmentainerSpace::new(lu(500)));
    assert_eq!(r.child_index, 0); assert_eq!(r.appeal, BreakAppeal::Default);
}
#[test] fn basic_frag_single_overflows() {
    assert_eq!(find_best_break_point(&[child(1000)], &FragmentainerSpace::new(lu(200))).appeal, BreakAppeal::LastResort);
}
#[test] fn basic_frag_all_zero_height() {
    assert_eq!(find_best_break_point(&[child(0), child(0), child(0)], &FragmentainerSpace::new(lu(100))).child_index, 3);
}
#[test] fn basic_frag_first_fills_exactly() {
    assert_eq!(find_best_break_point(&[child(500), child(100)], &FragmentainerSpace::new(lu(500))).child_index, 1);
}
#[test] fn basic_frag_three_two_fit() {
    assert_eq!(find_best_break_point(&[child(100), child(100), child(100)], &FragmentainerSpace::new(lu(250))).child_index, 2);
}
#[test] fn basic_frag_many_small() {
    let c: Vec<_> = (0..20).map(|_| child(10)).collect();
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(100))).child_index, 10);
}
#[test] fn basic_frag_token_no_children() {
    let t = BlockBreakToken::new(0, lu(0));
    assert!(!t.has_child_break_tokens()); assert!(t.child_break_tokens.is_empty());
}
#[test] fn basic_frag_token_with_children() {
    let mut p = BlockBreakToken::new(1, lu(100));
    p.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(50))));
    assert!(p.has_child_break_tokens()); assert_eq!(p.child_break_tokens.len(), 1);
}
#[test] fn basic_frag_token_multiple_children() {
    let mut p = BlockBreakToken::new(2, lu(200));
    for i in 0..5 { p.add_child_token(BreakToken::Block(BlockBreakToken::new(i, lu(i as i32 * 10)))); }
    assert_eq!(p.child_break_tokens.len(), 5);
}
#[test] fn basic_frag_break_before_flag() { assert!(BlockBreakToken::break_before(0, lu(0)).is_break_before); }
#[test] fn basic_frag_not_break_before() { assert!(!BlockBreakToken::new(0, lu(0)).is_break_before); }
#[test] fn basic_frag_token_preserves() {
    let t = BlockBreakToken::new(5, lu(999));
    assert_eq!(t.child_index, 5); assert_eq!(t.consumed_block_size, lu(999));
}
#[test] fn basic_frag_first_overflows() {
    assert_eq!(find_best_break_point(&[child(600), child(100)], &FragmentainerSpace::new(lu(500))).child_index, 1);
}
#[test] fn basic_frag_large_space_all_fit() {
    assert_eq!(find_best_break_point(&[child(50), child(50), child(50)], &FragmentainerSpace::new(lu(10000))).child_index, 3);
}
#[test] fn basic_frag_unequal_break() {
    // 50+100=150, 150+200=350>300
    assert_eq!(find_best_break_point(&[child(50), child(100), child(200), child(50)], &FragmentainerSpace::new(lu(300))).child_index, 2);
}
#[test] fn basic_frag_partial_offset() {
    let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(200));
    assert_eq!(find_best_break_point(&[child(200), child(200)], &s).child_index, 1);
}
#[test] fn basic_frag_no_remaining() {
    let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(500));
    assert_eq!(find_best_break_point(&[child(100)], &s).appeal, BreakAppeal::LastResort);
}
#[test] fn basic_frag_chaining_deep() {
    let mut root = BlockBreakToken::new(0, lu(100));
    let mut mid = BlockBreakToken::new(0, lu(50));
    mid.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(25))));
    root.add_child_token(BreakToken::Block(mid));
    assert_eq!(root.child_break_tokens.len(), 1);
    match &root.child_break_tokens[0] { BreakToken::Block(i) => assert_eq!(i.child_break_tokens.len(), 1) }
}
#[test] fn basic_frag_zero_consumed() { assert_eq!(BlockBreakToken::new(0, lu(0)).consumed_block_size, lu(0)); }
#[test] fn basic_frag_five_equal_small() {
    assert_eq!(find_best_break_point(&vec![child(100); 5], &FragmentainerSpace::new(lu(150))).child_index, 1);
}
#[test] fn basic_frag_two_exactly_fit() {
    assert_eq!(find_best_break_point(&[child(250), child(250)], &FragmentainerSpace::new(lu(500))).child_index, 2);
}
#[test] fn basic_frag_tiny_fragmentainer() {
    assert!(find_best_break_point(&[child(100), child(100), child(100)], &FragmentainerSpace::new(lu(50))).child_index <= 1);
}
#[test] fn basic_frag_gradual_increase() {
    // 10+20+30=60 fits, 60+40=100>65
    assert_eq!(find_best_break_point(&[child(10), child(20), child(30), child(40), child(50)], &FragmentainerSpace::new(lu(65))).child_index, 3);
}
#[test] fn basic_frag_break_before_consumed() {
    let t = BlockBreakToken::break_before(3, lu(300));
    assert!(t.is_break_before); assert_eq!(t.child_index, 3); assert_eq!(t.consumed_block_size, lu(300));
}
#[test] fn basic_frag_one_per_fragmentainer() {
    assert_eq!(find_best_break_point(&[child(100), child(100), child(100)], &FragmentainerSpace::new(lu(100))).child_index, 1);
}
#[test] fn basic_frag_100_children() {
    let c: Vec<_> = (0..100).map(|_| child(1)).collect();
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(50))).child_index, 50);
}
#[test] fn basic_frag_alternating() {
    // 50+150=200, 200+50=250, 250+150=400>250
    assert_eq!(find_best_break_point(&[child(50), child(150), child(50), child(150)], &FragmentainerSpace::new(lu(250))).child_index, 3);
}
#[test] fn basic_frag_token_clone() {
    let t = BlockBreakToken::new(2, lu(150)).clone();
    assert_eq!(t.child_index, 2); assert_eq!(t.consumed_block_size, lu(150));
}
#[test] fn basic_frag_token_debug() { assert!(format!("{:?}", BlockBreakToken::new(1, lu(100))).contains("BlockBreakToken")); }
#[test] fn basic_frag_enum_block() {
    match BreakToken::Block(BlockBreakToken::new(0, lu(0))) { BreakToken::Block(b) => assert_eq!(b.child_index, 0) }
}
#[test] fn basic_frag_large_consumed() { assert_eq!(BlockBreakToken::new(0, lu(100_000)).consumed_block_size, lu(100_000)); }
#[test] fn basic_frag_large_index() { assert_eq!(BlockBreakToken::new(9999, lu(0)).child_index, 9999); }
#[test] fn basic_frag_partial_fit() {
    // 100+150=250<280, 250+80=330>280
    assert_eq!(find_best_break_point(&[child(100), child(150), child(80)], &FragmentainerSpace::new(lu(280))).child_index, 2);
}
#[test] fn basic_frag_new_at_start() {
    let s = FragmentainerSpace::new(lu(500));
    assert!(s.is_at_block_start); assert_eq!(s.block_offset, lu(0));
}
#[test] fn basic_frag_consumed_results() {
    let mut s = FragmentainerSpace::new(lu(1000)); s.consume(lu(100));
    assert_eq!(find_best_break_point(&[child(500), child(500)], &s).child_index, 1);
}
#[test] fn basic_frag_second_boundary() {
    // 100*3=300<350, 300+100=400>350
    assert_eq!(find_best_break_point(&vec![child(100); 4], &FragmentainerSpace::new(lu(350))).child_index, 3);
}
#[test] fn basic_frag_decreasing() {
    // 200+150=350<400, 350+100=450>400
    assert_eq!(find_best_break_point(&[child(200), child(150), child(100), child(50)], &FragmentainerSpace::new(lu(400))).child_index, 2);
}
#[test] fn basic_frag_exact_boundary_sum() {
    // 100+200=300 exact
    assert_eq!(find_best_break_point(&[child(100), child(200), child(200)], &FragmentainerSpace::new(lu(300))).child_index, 2);
}

// =========================================================================
// 2. BREAK PROPERTIES
// =========================================================================

#[test] fn break_before_page() {
    let mut c = vec![child(50), child(50)]; c[1].break_before = BreakValue::Page;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn break_before_column() {
    let mut c = vec![child(50), child(50)]; c[1].break_before = BreakValue::Column;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).appeal, BreakAppeal::Perfect);
}
#[test] fn break_before_always() {
    let mut c = vec![child(50), child(50), child(50)]; c[1].break_before = BreakValue::Always;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn break_before_avoid_demotes() {
    let mut c = vec![child(200), child(200), child(200)]; c[1].break_before = BreakValue::Avoid;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(500)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Default);
}
#[test] fn break_before_left() {
    let mut c = vec![child(50), child(50)]; c[1].break_before = BreakValue::Left;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).appeal, BreakAppeal::Perfect);
}
#[test] fn break_before_right() {
    let mut c = vec![child(50), child(50)]; c[1].break_before = BreakValue::Right;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).appeal, BreakAppeal::Perfect);
}
#[test] fn break_after_page() {
    let mut c = vec![child(50), child(50), child(50)]; c[0].break_after = BreakValue::Page;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn break_after_always() {
    let mut c = vec![child(50), child(50), child(50)]; c[0].break_after = BreakValue::Always;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).child_index, 1);
}
#[test] fn break_after_column() {
    let mut c = vec![child(50), child(50), child(50)]; c[0].break_after = BreakValue::Column;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).appeal, BreakAppeal::Perfect);
}
#[test] fn break_after_left() {
    let mut c = vec![child(50), child(50)]; c[0].break_after = BreakValue::Left;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).appeal, BreakAppeal::Perfect);
}
#[test] fn break_after_right() {
    let mut c = vec![child(50), child(50)]; c[0].break_after = BreakValue::Right;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).appeal, BreakAppeal::Perfect);
}
#[test] fn break_after_avoid_demotes() {
    let mut c = vec![child(200), child(200), child(200)]; c[0].break_after = BreakValue::Avoid;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(500))).child_index, 2);
}
#[test] fn break_inside_avoid() { assert!(BreakInside::Avoid.is_avoid()); }
#[test] fn break_inside_avoid_page() { assert!(BreakInside::AvoidPage.is_avoid()); }
#[test] fn break_inside_avoid_column() { assert!(BreakInside::AvoidColumn.is_avoid()); }
#[test] fn break_inside_auto_not_avoid() { assert!(!BreakInside::Auto.is_avoid()); }
#[test] fn legacy_always() { let v = BreakValue::from_legacy_page_break("always"); assert_eq!(v, BreakValue::Page); assert!(should_break_before(v)); }
#[test] fn legacy_avoid() { let v = BreakValue::from_legacy_page_break("avoid"); assert_eq!(v, BreakValue::Avoid); assert!(!should_break_before(v)); }
#[test] fn legacy_auto() { assert_eq!(BreakValue::from_legacy_page_break("auto"), BreakValue::Auto); }
#[test] fn legacy_left() { assert_eq!(BreakValue::from_legacy_page_break("left"), BreakValue::Left); }
#[test] fn legacy_right() { assert_eq!(BreakValue::from_legacy_page_break("right"), BreakValue::Right); }
#[test] fn legacy_unknown() { assert_eq!(BreakValue::from_legacy_page_break("x"), BreakValue::Auto); }
#[test] fn legacy_inside_avoid() { let v = BreakInside::from_legacy_page_break_inside("avoid"); assert_eq!(v, BreakInside::Avoid); assert!(v.is_avoid()); }
#[test] fn legacy_inside_auto() { let v = BreakInside::from_legacy_page_break_inside("auto"); assert_eq!(v, BreakInside::Auto); assert!(!v.is_avoid()); }
#[test] fn legacy_inside_unknown() { assert_eq!(BreakInside::from_legacy_page_break_inside("x"), BreakInside::Auto); }
#[test] fn break_before_first_child_no_effect() {
    let mut c = vec![child(50), child(50)]; c[0].break_before = BreakValue::Always;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).child_index, 2);
}
#[test] fn break_after_last_child_no_effect() {
    let mut c = vec![child(50), child(50)]; c[1].break_after = BreakValue::Always;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).child_index, 2);
}
#[test] fn should_break_before_all_forced() {
    for v in [BreakValue::Always, BreakValue::Page, BreakValue::Column, BreakValue::Left, BreakValue::Right] { assert!(should_break_before(v)); }
}
#[test] fn should_not_break_before_non_forced() {
    for v in [BreakValue::Auto, BreakValue::Avoid, BreakValue::AvoidPage, BreakValue::AvoidColumn] { assert!(!should_break_before(v)); }
}
#[test] fn should_break_after_all_forced() {
    for v in [BreakValue::Always, BreakValue::Page, BreakValue::Column, BreakValue::Left, BreakValue::Right] { assert!(should_break_after(v)); }
}
#[test] fn should_not_break_after_non_forced() {
    for v in [BreakValue::Auto, BreakValue::Avoid] { assert!(!should_break_after(v)); }
}
#[test] fn break_avoid_page_prop() {
    let mut c = vec![child(200), child(200), child(200)]; c[1].break_before = BreakValue::AvoidPage;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(500))).child_index, 2);
}
#[test] fn break_avoid_column_prop() {
    let mut c = vec![child(200), child(200), child(200)]; c[1].break_before = BreakValue::AvoidColumn;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(500))).child_index, 2);
}
#[test] fn forced_overrides_natural() {
    let mut c = vec![child(50), child(50), child(50)]; c[2].break_before = BreakValue::Always;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn forced_break_after_second() {
    let mut c = vec![child(50), child(50), child(50)]; c[1].break_after = BreakValue::Always;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn avoid_before_and_after_both() {
    let mut c = vec![child(200), child(200), child(200)];
    c[0].break_after = BreakValue::Avoid; c[1].break_before = BreakValue::Avoid;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(500))).child_index, 2);
}
#[test] fn break_value_is_forced_true() {
    for v in [BreakValue::Always, BreakValue::Page, BreakValue::Column, BreakValue::Left, BreakValue::Right] { assert!(v.is_forced()); }
}
#[test] fn break_value_is_forced_false() {
    for v in [BreakValue::Auto, BreakValue::Avoid, BreakValue::AvoidPage, BreakValue::AvoidColumn] { assert!(!v.is_forced()); }
}
#[test] fn break_value_is_avoid_true() {
    for v in [BreakValue::Avoid, BreakValue::AvoidPage, BreakValue::AvoidColumn] { assert!(v.is_avoid()); }
}
#[test] fn break_value_is_avoid_false() {
    for v in [BreakValue::Auto, BreakValue::Always, BreakValue::Page] { assert!(!v.is_avoid()); }
}
#[test] fn break_inside_is_avoid_all() {
    assert!(BreakInside::Avoid.is_avoid()); assert!(BreakInside::AvoidPage.is_avoid());
    assert!(BreakInside::AvoidColumn.is_avoid()); assert!(!BreakInside::Auto.is_avoid());
}
#[test] fn computed_style_frag_defaults() {
    let s = ComputedStyle::initial();
    assert_eq!(s.break_before, BreakValue::Auto); assert_eq!(s.break_after, BreakValue::Auto);
    assert_eq!(s.break_inside, BreakInside::Auto);
}
#[test] fn combined_before_after_forces() {
    let mut c = vec![child(50), child(50), child(50)];
    c[0].break_after = BreakValue::Page; c[1].break_before = BreakValue::Column;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn multiple_forced_first_wins() {
    let mut c = vec![child(50); 4]; c[1].break_before = BreakValue::Always; c[2].break_before = BreakValue::Always;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).child_index, 1);
}
#[test] fn avoid_all_boundaries_last_resort() {
    let mut c = vec![child(200); 3]; c[1].break_before = BreakValue::Avoid; c[2].break_before = BreakValue::Avoid;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(500)));
    assert_eq!(r.appeal, BreakAppeal::LastResort); assert_eq!(r.child_index, 2);
}

// =========================================================================
// 3. BREAK APPEAL
// =========================================================================

#[test] fn appeal_perfect_gt_default() { assert!(BreakAppeal::Perfect > BreakAppeal::Default); }
#[test] fn appeal_default_gt_violating() { assert!(BreakAppeal::Default > BreakAppeal::Violating); }
#[test] fn appeal_violating_gt_last() { assert!(BreakAppeal::Violating > BreakAppeal::LastResort); }
#[test] fn appeal_perfect_gt_last() { assert!(BreakAppeal::Perfect > BreakAppeal::LastResort); }
#[test] fn appeal_perfect_gt_violating() { assert!(BreakAppeal::Perfect > BreakAppeal::Violating); }
#[test] fn appeal_default_gt_last() { assert!(BreakAppeal::Default > BreakAppeal::LastResort); }
#[test] fn appeal_chain() {
    let a = [BreakAppeal::LastResort, BreakAppeal::Violating, BreakAppeal::Default, BreakAppeal::Perfect];
    for w in a.windows(2) { assert!(w[0] < w[1]); }
}
#[test] fn appeal_eq() {
    assert_eq!(BreakAppeal::Perfect, BreakAppeal::Perfect);
    assert_eq!(BreakAppeal::Default, BreakAppeal::Default);
    assert_eq!(BreakAppeal::Violating, BreakAppeal::Violating);
    assert_eq!(BreakAppeal::LastResort, BreakAppeal::LastResort);
}
#[test] fn appeal_is_forced() {
    assert!(BreakAppeal::Perfect.is_forced()); assert!(!BreakAppeal::Default.is_forced());
    assert!(!BreakAppeal::Violating.is_forced()); assert!(!BreakAppeal::LastResort.is_forced());
}
#[test] fn appeal_is_acceptable() {
    assert!(BreakAppeal::Perfect.is_acceptable()); assert!(BreakAppeal::Default.is_acceptable());
    assert!(!BreakAppeal::Violating.is_acceptable()); assert!(!BreakAppeal::LastResort.is_acceptable());
}
#[test] fn appeal_avoid_picks_nonavoid() {
    let mut c = vec![child(200); 3]; c[1].break_before = BreakValue::Avoid;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(500)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Default);
}
#[test] fn appeal_forced() {
    let mut c = vec![child(50), child(50)]; c[1].break_before = BreakValue::Always;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(1000))).appeal, BreakAppeal::Perfect);
}
#[test] fn appeal_no_valid_last_resort() {
    let mut c = vec![child(200), child(200)]; c[1].break_before = BreakValue::Avoid;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(300))).appeal, BreakAppeal::LastResort);
}
#[test] fn appeal_later_index_wins() {
    assert_eq!(find_best_break_point(&vec![child(100); 4], &FragmentainerSpace::new(lu(350))).child_index, 3);
}
#[test] fn appeal_before_forced_vals() {
    for v in [BreakValue::Always, BreakValue::Page, BreakValue::Column, BreakValue::Left, BreakValue::Right] {
        assert_eq!(break_before_appeal(v), BreakAppeal::Perfect);
    }
}
#[test] fn appeal_before_avoid_vals() {
    for v in [BreakValue::Avoid, BreakValue::AvoidPage, BreakValue::AvoidColumn] {
        assert_eq!(break_before_appeal(v), BreakAppeal::LastResort);
    }
}
#[test] fn appeal_before_auto() { assert_eq!(break_before_appeal(BreakValue::Auto), BreakAppeal::Default); }
#[test] fn appeal_after_forced_vals() {
    for v in [BreakValue::Always, BreakValue::Page, BreakValue::Column] { assert_eq!(break_after_appeal(v), BreakAppeal::Perfect); }
}
#[test] fn appeal_after_avoid_vals() {
    for v in [BreakValue::Avoid, BreakValue::AvoidPage, BreakValue::AvoidColumn] { assert_eq!(break_after_appeal(v), BreakAppeal::LastResort); }
}
#[test] fn appeal_after_auto() { assert_eq!(break_after_appeal(BreakValue::Auto), BreakAppeal::Default); }
#[test] fn appeal_copy() { let a = BreakAppeal::Perfect; let b = a; let c = a.clone(); assert_eq!(a, b); assert_eq!(a, c); }
#[test] fn appeal_hash() {
    use std::collections::HashSet;
    let mut s = HashSet::new();
    s.insert(BreakAppeal::Perfect); s.insert(BreakAppeal::Default);
    s.insert(BreakAppeal::Violating); s.insert(BreakAppeal::LastResort);
    assert_eq!(s.len(), 4);
}
#[test] fn appeal_debug() { assert!(format!("{:?}", BreakAppeal::Default).contains("Default")); }
#[test] fn appeal_point_eq() {
    assert_eq!(BreakPoint { child_index: 1, appeal: BreakAppeal::Default }, BreakPoint { child_index: 1, appeal: BreakAppeal::Default });
}
#[test] fn appeal_point_ne() {
    assert_ne!(BreakPoint { child_index: 1, appeal: BreakAppeal::Default }, BreakPoint { child_index: 2, appeal: BreakAppeal::Default });
}
#[test] fn appeal_forced_middle() {
    let mut c = vec![child(50); 4]; c[2].break_before = BreakValue::Page;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn appeal_avoid_later() {
    let mut c = vec![child(150); 3]; c[1].break_before = BreakValue::Avoid;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(400)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Default);
}
#[test] fn appeal_multi_avoid() {
    let mut c = vec![child(100); 4]; c[1].break_before = BreakValue::Avoid;
    c[2].break_before = BreakValue::Avoid; c[3].break_before = BreakValue::Avoid;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(350))).appeal, BreakAppeal::LastResort);
}
#[test] fn appeal_perfect_wins() {
    let mut c = vec![child(100); 3]; c[1].break_before = BreakValue::Always; c[2].break_before = BreakValue::Avoid;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn appeal_equal_prefer_later() {
    assert_eq!(find_best_break_point(&vec![child(100); 3], &FragmentainerSpace::new(lu(250))).child_index, 2);
}

// =========================================================================
// 4. MULTI-COLUMN LAYOUT
// =========================================================================

#[test] fn mc_count_2() { let r = resolve_column_count_and_width(Some(2), None, lu(600), lu(20)); assert_eq!(r.count, 2); assert_eq!(r.width, lu(290)); }
#[test] fn mc_count_3_gap() { let r = resolve_column_count_and_width(Some(3), None, lu(900), lu(20)); assert_eq!(r.count, 3); assert_eq!(r.width, LayoutUnit::from_raw(lu(860).raw()/3)); }
#[test] fn mc_width_only() { assert_eq!(resolve_column_count_and_width(None, Some(lu(200)), lu(900), lu(20)).count, 4); }
#[test] fn mc_both_count_wins() { assert_eq!(resolve_column_count_and_width(Some(2), Some(lu(300)), lu(900), lu(20)).count, 2); }
#[test] fn mc_both_width_wins() { assert_eq!(resolve_column_count_and_width(Some(5), Some(lu(400)), lu(900), lu(20)).count, 2); }
#[test] fn mc_exact_width() { let r = resolve_column_count_and_width(Some(4), None, lu(800), lu(0)); assert_eq!(r.count, 4); assert_eq!(r.width, lu(200)); }
#[test] fn mc_gap_affects() { assert!(resolve_column_count_and_width(Some(3), None, lu(900), lu(0)).width > resolve_column_count_and_width(Some(3), None, lu(900), lu(30)).width); }
#[test] fn mc_centering() { assert_eq!(compute_column_positions(2, lu(100), lu(20), lu(400), true)[0].inline_offset, lu(90)); }
#[test] fn mc_fill_balance() { assert_eq!(balance_columns(&[lu(100), lu(100), lu(100)], 3, lu(1000)), lu(100)); }
#[test] fn mc_fill_auto() { assert_eq!(layout_columns(&algo(3, 10, ColumnFill::Auto), node(), lu(640), lu(500), &[lu(200), lu(300)]).block_size, lu(500)); }
#[test] fn mc_balance_uneven() { assert_eq!(balance_columns(&[lu(50), lu(80), lu(60), lu(40)], 2, lu(1000)), lu(130)); }
#[test] fn mc_balance_convergence() { assert_eq!(balance_columns(&[lu(100); 6], 3, lu(1000)), lu(200)); }
#[test] fn mc_auto_auto() { let r = resolve_column_count_and_width(None, None, lu(600), lu(10)); assert_eq!(r.count, 1); assert_eq!(r.width, lu(600)); }
#[test] fn mc_single_pass() { let r = resolve_column_count_and_width(Some(1), None, lu(800), lu(20)); assert_eq!(r.count, 1); assert_eq!(r.width, lu(800)); }
#[test] fn mc_positions_3() {
    let p = compute_column_positions(3, lu(200), lu(20), lu(660), false);
    assert_eq!(p.len(), 3); assert_eq!(p[0].inline_offset, lu(0));
    assert_eq!(p[1].inline_offset, lu(220)); assert_eq!(p[2].inline_offset, lu(440));
}
#[test] fn mc_positions_width() { for p in compute_column_positions(3, lu(200), lu(20), lu(660), false) { assert_eq!(p.width, lu(200)); } }
#[test] fn mc_positions_single() { let p = compute_column_positions(1, lu(600), lu(0), lu(600), false); assert_eq!(p.len(), 1); assert_eq!(p[0].inline_offset, lu(0)); }
#[test] fn mc_positions_zero() { assert!(compute_column_positions(0, lu(100), lu(20), lu(400), false).is_empty()); }
#[test] fn mc_positions_large_gap() { assert_eq!(compute_column_positions(2, lu(100), lu(200), lu(400), false)[1].inline_offset, lu(300)); }
#[test] fn mc_rules_3() { assert_eq!(compute_column_rule_positions(3, lu(200), lu(20)).len(), 2); }
#[test] fn mc_rules_1() { assert!(compute_column_rule_positions(1, lu(200), lu(20)).is_empty()); }
#[test] fn mc_rules_between() { assert_eq!(compute_column_rule_positions(2, lu(300), lu(50))[0], lu(300) + LayoutUnit::from_raw(lu(50).raw()/2)); }
#[test] fn mc_layout_dist() { let r = layout_columns(&algo(3, 10, ColumnFill::Balance), node(), lu(640), lu(500), &[lu(100); 3]); assert_eq!(r.block_size, lu(100)); }
#[test] fn mc_layout_empty() { assert_eq!(layout_columns(&algo(3, 10, ColumnFill::Balance), node(), lu(640), lu(500), &[]).block_size, lu(0)); }
#[test] fn mc_layout_single() { assert_eq!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(500), lu(500), &[lu(200)]).block_size, lu(200)); }
#[test] fn mc_layout_overflow() { assert_eq!(layout_columns(&algo(2, 10, ColumnFill::Auto), node(), lu(500), lu(200), &[lu(100); 4]).block_size, lu(200)); }
#[test] fn mc_balance_single_col() { assert_eq!(balance_columns(&[lu(100), lu(200)], 1, lu(1000)), lu(300)); }
#[test] fn mc_balance_empty() { assert_eq!(balance_columns(&[], 3, lu(1000)), lu(0)); }
#[test] fn mc_balance_large_child() { assert_eq!(balance_columns(&[lu(500), lu(50), lu(50)], 2, lu(1000)), lu(500)); }
#[test] fn mc_balance_all_same() { assert_eq!(balance_columns(&[lu(100); 6], 3, lu(1000)), lu(200)); }
#[test] fn mc_balance_2c3i() { assert_eq!(balance_columns(&[lu(100); 3], 2, lu(1000)), lu(200)); }
#[test] fn mc_4_zero_gap() { let r = resolve_column_count_and_width(Some(4), None, lu(800), lu(0)); assert_eq!(r.count, 4); assert_eq!(r.width, lu(200)); }
#[test] fn mc_w200g10() { assert_eq!(resolve_column_count_and_width(None, Some(lu(200)), lu(650), lu(10)).count, 3); }
#[test] fn mc_2g50() { let r = resolve_column_count_and_width(Some(2), None, lu(600), lu(50)); assert_eq!(r.count, 2); assert_eq!(r.width, lu(275)); }
#[test] fn mc_neg_width() { let r = resolve_column_count_and_width(None, Some(lu(-100)), lu(600), lu(10)); assert!(r.count >= 1); }
#[test] fn mc_large_narrow() { assert_eq!(resolve_column_count_and_width(Some(100), None, lu(50), lu(0)).count, 100); }
#[test] fn mc_both_zero_gap() { let r = resolve_column_count_and_width(Some(3), Some(lu(150)), lu(600), lu(0)); assert_eq!(r.count, 3); assert_eq!(r.width, lu(200)); }
#[test] fn mc_5g10() { assert_eq!(resolve_column_count_and_width(Some(5), None, lu(1000), lu(10)).width, LayoutUnit::from_raw(lu(960).raw()/5)); }
#[test] fn mc_centered_3() {
    let p = compute_column_positions(3, lu(100), lu(10), lu(500), true);
    assert_eq!(p[0].inline_offset, lu(90)); assert_eq!(p[1].inline_offset, lu(200)); assert_eq!(p[2].inline_offset, lu(310));
}
#[test] fn mc_not_centered() { assert_eq!(compute_column_positions(2, lu(200), lu(20), lu(600), false)[0].inline_offset, lu(0)); }
#[test] fn mc_bal_7i3c() { let h = balance_columns(&[lu(10), lu(20), lu(30), lu(40), lu(50), lu(60), lu(70)], 3, lu(1000)); assert!(h.raw() > 0 && h <= lu(1000)); }
#[test] fn mc_bal_constrained() { let h = balance_columns(&[lu(100); 3], 2, lu(120)); assert!(h.raw() > 0); }
#[test] fn mc_rules_4() { assert_eq!(compute_column_rule_positions(4, lu(100), lu(20)).len(), 3); }
#[test] fn mc_rules_zero_gap() { let r = compute_column_rule_positions(3, lu(200), lu(0)); assert_eq!(r.len(), 2); assert_eq!(r[0], lu(200)); }
#[test] fn mc_bal_2c() { assert_eq!(layout_columns(&algo(2, 0, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(100); 2]).block_size, lu(100)); }
#[test] fn mc_auto_height() { assert_eq!(layout_columns(&algo(2, 10, ColumnFill::Auto), node(), lu(400), lu(300), &[lu(100), lu(200)]).block_size, lu(300)); }
#[test] fn mc_frag_children() { let r = layout_columns(&algo(3, 10, ColumnFill::Balance), node(), lu(640), lu(500), &[lu(100); 3]); assert_eq!(r.fragment.children.len(), r.column_fragments.len()); }
#[test] fn mc_1_full_width() { assert_eq!(resolve_column_count_and_width(Some(1), None, lu(400), lu(20)).width, lu(400)); }
#[test] fn mc_exact_divide() { assert_eq!(resolve_column_count_and_width(None, Some(lu(100)), lu(320), lu(20)).count, 2); }
#[test] fn mc_4_zero_gap_pos() { let p = compute_column_positions(4, lu(100), lu(0), lu(400), false); for (i, c) in p.iter().enumerate() { assert_eq!(c.inline_offset, lu(i as i32 * 100)); } }
#[test] fn mc_single_item_2c() { assert_eq!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(50)]).block_size, lu(50)); }
#[test] fn mc_10_narrow() { assert_eq!(resolve_column_count_and_width(Some(10), None, lu(100), lu(0)).count, 10); }
#[test] fn mc_nonzero_block() { assert!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(200); 2]).block_size.raw() > 0); }
#[test] fn mc_bal_identical() { assert_eq!(balance_columns(&[lu(50); 10], 5, lu(1000)), lu(100)); }
#[test] fn mc_bal_one_large() { let mut c = vec![lu(10); 9]; c.push(lu(500)); assert_eq!(balance_columns(&c, 2, lu(1000)), lu(500)); }
#[test] fn mc_center_exceeds() { assert_eq!(compute_column_positions(2, lu(300), lu(20), lu(400), true)[0].inline_offset, lu(0)); }
#[test] fn mc_w300g0a900() { let r = resolve_column_count_and_width(None, Some(lu(300)), lu(900), lu(0)); assert_eq!(r.count, 3); assert_eq!(r.width, lu(300)); }
#[test] fn mc_from_style_none() { assert!(ColumnLayoutAlgorithm::from_style(&ComputedStyle::initial()).is_none()); }
#[test] fn mc_from_style_count() { let mut s = ComputedStyle::initial(); s.column_count = Some(3); assert_eq!(ColumnLayoutAlgorithm::from_style(&s).unwrap().column_count, 3); }
#[test] fn mc_style_defaults() { let s = ComputedStyle::initial(); assert!(s.column_count.is_none()); assert!(s.column_width.is_none()); assert_eq!(s.column_fill, ColumnFill::Balance); }

// =========================================================================
// 5. COLUMN SPANNING
// =========================================================================

#[test] fn span_default_none() { assert_eq!(ComputedStyle::initial().column_span, ColumnSpan::None); }
#[test] fn span_all_eq() { assert_eq!(ColumnSpan::All, ColumnSpan::All); assert_ne!(ColumnSpan::All, ColumnSpan::None); }
#[test] fn span_none_eq() { assert_eq!(ColumnSpan::None, ColumnSpan::None); }
#[test] fn span_set_all() { let mut s = ComputedStyle::initial(); s.column_span = ColumnSpan::All; assert_eq!(s.column_span, ColumnSpan::All); }
#[test] fn span_set_none() { let mut s = ComputedStyle::initial(); s.column_span = ColumnSpan::None; assert_eq!(s.column_span, ColumnSpan::None); }
#[test] fn span_ne() { assert_ne!(ColumnSpan::All, ColumnSpan::None); }
#[test] fn span_copy() { let a = ColumnSpan::All; let b = a; assert_eq!(a, b); }
#[test] fn span_clone() { assert_eq!(ColumnSpan::All.clone(), ColumnSpan::All); }
#[test] fn span_debug() { assert!(format!("{:?}", ColumnSpan::All).contains("All")); }
#[test] fn span_hash() { use std::collections::HashSet; let mut s = HashSet::new(); s.insert(ColumnSpan::All); s.insert(ColumnSpan::None); assert_eq!(s.len(), 2); }
#[test] fn span_before() { assert!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(100); 2]).block_size.raw() > 0); }
#[test] fn span_after() { assert!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(50); 2]).block_size.raw() > 0); }
#[test] fn span_mutation() { let mut s = ComputedStyle::initial(); s.column_span = ColumnSpan::All; assert_eq!(s.column_span, ColumnSpan::All); s.column_span = ColumnSpan::None; assert_eq!(s.column_span, ColumnSpan::None); }
#[test] fn span_multi_styles() { let (mut a, mut b) = (ComputedStyle::initial(), ComputedStyle::initial()); a.column_span = ColumnSpan::All; b.column_span = ColumnSpan::None; assert_ne!(a.column_span, b.column_span); }
#[test] fn span_initial() { assert_eq!(ColumnSpan::INITIAL, ColumnSpan::None); }
#[test] fn span_default() { assert_eq!(ColumnSpan::default(), ColumnSpan::None); }
#[test] fn span_unaffected() { assert_eq!(layout_columns(&algo(3, 10, ColumnFill::Balance), node(), lu(600), lu(500), &[lu(100); 3]).block_size, lu(100)); }
#[test] fn span_2c_normal() { assert_eq!(layout_columns(&algo(2, 0, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(50); 4]).block_size, lu(100)); }
#[test] fn span_auto() { assert_eq!(layout_columns(&algo(2, 10, ColumnFill::Auto), node(), lu(400), lu(300), &[lu(100); 2]).block_size, lu(300)); }
#[test] fn span_all_repr() { assert_eq!(ColumnSpan::All as u8, 1); }
#[test] fn span_none_repr() { assert_eq!(ColumnSpan::None as u8, 0); }
#[test] fn span_fill_variants() {
    assert!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(100); 2]).block_size.raw() > 0);
    assert!(layout_columns(&algo(2, 10, ColumnFill::Auto), node(), lu(400), lu(500), &[lu(100); 2]).block_size.raw() > 0);
}
#[test] fn span_3c6i() { assert_eq!(layout_columns(&algo(3, 10, ColumnFill::Balance), node(), lu(600), lu(500), &[lu(50); 6]).block_size, lu(100)); }
#[test] fn span_many_2c() { assert_eq!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(10); 20]).block_size, lu(100)); }

// =========================================================================
// 6. NESTED FRAGMENTATION
// =========================================================================

#[test] fn nest_parent_child() {
    let mut p = BlockBreakToken::new(1, lu(300));
    p.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(100))));
    assert!(p.has_child_break_tokens()); assert_eq!(p.child_break_tokens.len(), 1);
}
#[test] fn nest_3_levels() {
    let mut l1 = BlockBreakToken::new(0, lu(300));
    let mut l2 = BlockBreakToken::new(1, lu(200));
    l2.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(50))));
    l1.add_child_token(BreakToken::Block(l2));
    match &l1.child_break_tokens[0] { BreakToken::Block(i) => { assert_eq!(i.child_break_tokens.len(), 1); match &i.child_break_tokens[0] { BreakToken::Block(l) => assert_eq!(l.consumed_block_size, lu(50)) } } }
}
#[test] fn nest_multicol_in_multicol() {
    assert!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(600), lu(500), &[lu(200); 2]).block_size.raw() > 0);
    assert!(layout_columns(&algo(3, 5, ColumnFill::Balance), node(), lu(295), lu(200), &[lu(50); 3]).block_size.raw() > 0);
}
#[test] fn nest_content_in_col() { assert!(layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(100); 4]).column_fragments.len() >= 2); }
#[test] fn nest_block_in_frag() {
    let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(200));
    assert_eq!(find_best_break_point(&[child(100), child(100), child(200)], &s).child_index, 2);
}
#[test] fn nest_multi_child_tokens() {
    let mut p = BlockBreakToken::new(3, lu(500));
    for i in 0..3 { p.add_child_token(BreakToken::Block(BlockBreakToken::new(i, lu(i as i32 * 50)))); }
    assert_eq!(p.child_break_tokens.len(), 3);
}
#[test] fn nest_preserves_data() {
    let mut p = BlockBreakToken::new(0, lu(0));
    p.add_child_token(BreakToken::Block(BlockBreakToken::break_before(5, lu(999))));
    match &p.child_break_tokens[0] { BreakToken::Block(i) => { assert!(i.is_break_before); assert_eq!(i.child_index, 5); assert_eq!(i.consumed_block_size, lu(999)); } }
}
#[test] fn nest_frag_in_col() { assert_eq!(find_best_break_point(&[child(150), child(100)], &FragmentainerSpace::new(lu(200))).child_index, 1); }
#[test] fn nest_4_levels() {
    let mut l1 = BlockBreakToken::new(0, lu(400));
    let mut l2 = BlockBreakToken::new(0, lu(300));
    let mut l3 = BlockBreakToken::new(0, lu(200));
    l3.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(100))));
    l2.add_child_token(BreakToken::Block(l3));
    l1.add_child_token(BreakToken::Block(l2));
    match &l1.child_break_tokens[0] { BreakToken::Block(b2) => match &b2.child_break_tokens[0] { BreakToken::Block(b3) => match &b3.child_break_tokens[0] { BreakToken::Block(b4) => { assert_eq!(b4.consumed_block_size, lu(100)); assert!(!b4.has_child_break_tokens()); } } } }
}
#[test] fn nest_break_before_nested() {
    let mut p = BlockBreakToken::new(2, lu(200));
    p.add_child_token(BreakToken::Block(BlockBreakToken::break_before(0, lu(0))));
    match &p.child_break_tokens[0] { BreakToken::Block(i) => assert!(i.is_break_before) }
}
#[test] fn nest_sibling_tokens() {
    let mut p = BlockBreakToken::new(0, lu(500));
    p.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(100))));
    p.add_child_token(BreakToken::Block(BlockBreakToken::new(1, lu(200))));
    assert_eq!(p.child_break_tokens.len(), 2);
}
#[test] fn nest_empty_children() { assert!(!BlockBreakToken::new(0, lu(0)).has_child_break_tokens()); }
#[test] fn nest_col_multi_frag() { assert_eq!(layout_columns(&algo(3, 10, ColumnFill::Balance), node(), lu(640), lu(500), &[lu(50); 9]).block_size, lu(150)); }
#[test] fn nest_balance_within() { assert_eq!(balance_columns(&[lu(30); 6], 2, lu(500)), lu(90)); }
#[test] fn nest_consumed_accum() { assert_eq!(BlockBreakToken::new(0, lu(100)).consumed_block_size + BlockBreakToken::new(1, lu(200)).consumed_block_size, lu(300)); }
#[test] fn nest_2x2() { assert_eq!(layout_columns(&algo(2, 0, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(100); 4]).block_size, lu(200)); }
#[test] fn nest_break_nested_ctx() {
    let mut s = FragmentainerSpace::new(lu(100)); s.consume(lu(50));
    assert_eq!(find_best_break_point(&[child(30), child(30)], &s).child_index, 1);
}
#[test] fn nest_index_varies() { for i in 0..10usize { assert_eq!(BlockBreakToken::new(i, lu(i as i32 * 100)).child_index, i); } }
#[test] fn nest_clone_children() {
    let mut p = BlockBreakToken::new(0, lu(100));
    p.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(50))));
    assert_eq!(p.clone().child_break_tokens.len(), 1);
}
#[test] fn nest_child_lt_parent() {
    let mut p = BlockBreakToken::new(2, lu(500));
    p.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(200))));
    match &p.child_break_tokens[0] { BreakToken::Block(c) => assert!(c.consumed_block_size < p.consumed_block_size) }
}
#[test] fn nest_positioned() { let t = BlockBreakToken::new(0, lu(0)); assert_eq!(t.child_index, 0); assert!(!t.is_break_before); }
#[test] fn nest_float_in_frag() {
    let mut s = FragmentainerSpace::new(lu(300)); s.consume(lu(100));
    assert_eq!(find_best_break_point(&[child(100), child(150)], &s).child_index, 1);
}
#[test] fn nest_5_levels() {
    let mut tokens: Vec<BlockBreakToken> = (0..5usize).map(|i| BlockBreakToken::new(i, lu(i as i32 * 10))).collect();
    for _ in 0..4 { let inner = tokens.pop().unwrap(); tokens.last_mut().unwrap().add_child_token(BreakToken::Block(inner)); }
    assert!(tokens.pop().unwrap().has_child_break_tokens());
}
#[test] fn nest_3c9i() { assert_eq!(layout_columns(&algo(3, 0, ColumnFill::Balance), node(), lu(600), lu(500), &[lu(30); 9]).block_size, lu(90)); }
#[test] fn nest_prior_consumed() {
    let s = FragmentainerSpace { block_size: lu(500), block_offset: lu(300), is_at_block_start: false };
    assert_eq!(find_best_break_point(&[child(100), child(150)], &s).child_index, 1);
}

// =========================================================================
// 7. FRAGMENTAINER SPACE
// =========================================================================

#[test] fn fs_remaining_full() { assert_eq!(FragmentainerSpace::new(lu(1000)).remaining(), lu(1000)); }
#[test] fn fs_remaining_partial() { assert_eq!((FragmentainerSpace { block_size: lu(800), block_offset: lu(350), is_at_block_start: false }).remaining(), lu(450)); }
#[test] fn fs_remaining_zero() { assert_eq!((FragmentainerSpace { block_size: lu(500), block_offset: lu(500), is_at_block_start: false }).remaining(), lu(0)); }
#[test] fn fs_remaining_neg() { assert!((FragmentainerSpace { block_size: lu(500), block_offset: lu(600), is_at_block_start: false }).remaining() < lu(0)); }
#[test] fn fs_exhausted_exact() { assert!((FragmentainerSpace { block_size: lu(500), block_offset: lu(500), is_at_block_start: false }).is_exhausted()); }
#[test] fn fs_exhausted_over() { assert!((FragmentainerSpace { block_size: lu(500), block_offset: lu(600), is_at_block_start: false }).is_exhausted()); }
#[test] fn fs_not_exhausted() { assert!(!FragmentainerSpace::new(lu(500)).is_exhausted()); }
#[test] fn fs_consume_offset() { let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(200)); assert_eq!(s.block_offset, lu(200)); assert_eq!(s.remaining(), lu(300)); }
#[test] fn fs_consume_clears_start() { let mut s = FragmentainerSpace::new(lu(500)); assert!(s.is_at_block_start); s.consume(lu(1)); assert!(!s.is_at_block_start); }
#[test] fn fs_consume_exact() { let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(500)); assert!(s.is_exhausted()); }
#[test] fn fs_consume_beyond() { let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(600)); assert!(s.is_exhausted()); }
#[test] fn fs_multi_consume() { let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(100)); s.consume(lu(100)); s.consume(lu(100)); assert_eq!(s.remaining(), lu(200)); }
#[test] fn fs_new_start() { let s = FragmentainerSpace::new(lu(800)); assert!(s.is_at_block_start); assert_eq!(s.block_offset, lu(0)); assert_eq!(s.block_size, lu(800)); }
#[test] fn fs_zero_size() { let s = FragmentainerSpace::new(lu(0)); assert!(s.is_exhausted()); assert_eq!(s.remaining(), lu(0)); }
#[test] fn fs_consume_zero() { let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(0)); assert_eq!(s.remaining(), lu(500)); assert!(!s.is_at_block_start); }
#[test] fn fs_large() { assert_eq!(FragmentainerSpace::new(lu(100_000)).remaining(), lu(100_000)); }
#[test] fn fs_copy() { let a = FragmentainerSpace { block_size: lu(500), block_offset: lu(200), is_at_block_start: false }; let b = a; assert_eq!(b.remaining(), lu(300)); }
#[test] fn fs_debug() { assert!(format!("{:?}", FragmentainerSpace::new(lu(500))).contains("FragmentainerSpace")); }
#[test] fn fs_seq_exhaust() { let mut s = FragmentainerSpace::new(lu(300)); for _ in 0..6 { s.consume(lu(50)); } assert!(s.is_exhausted()); }
#[test] fn fs_one_left() { let mut s = FragmentainerSpace::new(lu(1000)); s.consume(lu(999)); assert_eq!(s.remaining(), lu(1)); assert!(!s.is_exhausted()); }
#[test] fn fs_exactly_one() { let mut s = FragmentainerSpace::new(lu(100)); s.consume(lu(99)); assert_eq!(s.remaining(), lu(1)); }
#[test] fn fs_all_at_once() { let mut s = FragmentainerSpace::new(lu(750)); s.consume(lu(750)); assert!(s.is_exhausted()); }
#[test] fn fs_size_unchanged() { let mut s = FragmentainerSpace::new(lu(500)); s.consume(lu(200)); assert_eq!(s.block_size, lu(500)); }
#[test] fn fs_at_start() { assert!(FragmentainerSpace::new(lu(100)).is_at_block_start); }
#[test] fn fs_not_start() { assert!(!(FragmentainerSpace { block_size: lu(500), block_offset: lu(100), is_at_block_start: false }).is_at_block_start); }
#[test] fn fs_copy_sem() { let a = FragmentainerSpace::new(lu(500)); let b = a; assert_eq!(a.remaining(), b.remaining()); }
#[test] fn fs_half() { let mut s = FragmentainerSpace::new(lu(400)); s.consume(lu(200)); assert_eq!(s.remaining(), lu(200)); assert!(!s.is_exhausted()); }
#[test] fn fs_increments() { let mut s = FragmentainerSpace::new(lu(100)); for i in 0..10 { assert_eq!(s.block_offset, lu(i*10)); s.consume(lu(10)); } assert!(s.is_exhausted()); }
#[test] fn fs_small() { assert_eq!(FragmentainerSpace::new(lu(1)).remaining(), lu(1)); }
#[test] fn fs_tiny() { let mut s = FragmentainerSpace::new(lu(1000)); s.consume(lu(1)); assert_eq!(s.remaining(), lu(999)); }

// =========================================================================
// 8. EDGE CASES
// =========================================================================

#[test] fn edge_zero_frag() { assert_eq!(find_best_break_point(&[child(100)], &FragmentainerSpace::new(lu(0))).appeal, BreakAppeal::LastResort); }
#[test] fn edge_exact_fit() { assert_eq!(find_best_break_point(&[child(100), child(200), child(200)], &FragmentainerSpace::new(lu(500))).child_index, 3); }
#[test] fn edge_single_larger() { assert_eq!(find_best_break_point(&[child(1000)], &FragmentainerSpace::new(lu(100))).appeal, BreakAppeal::LastResort); }
#[test] fn edge_empty_mc() { assert_eq!(layout_columns(&algo(3, 10, ColumnFill::Balance), node(), lu(600), lu(500), &[]).block_size, lu(0)); }
#[test] fn edge_c1_pass() { let r = resolve_column_count_and_width(Some(1), None, lu(800), lu(20)); assert_eq!(r.count, 1); assert_eq!(r.width, lu(800)); }
#[test] fn edge_large_count() { assert_eq!(resolve_column_count_and_width(Some(1000), None, lu(100), lu(0)).count, 1000); }
#[test] fn edge_zero_gap() { let r = resolve_column_count_and_width(Some(4), None, lu(800), lu(0)); assert_eq!(r.count, 4); assert_eq!(r.width, lu(200)); }
#[test] fn edge_bb_auto_auto() { assert_eq!(join_break_between(BreakBetween::Auto, BreakBetween::Auto), BreakBetween::Auto); }
#[test] fn edge_bb_auto_col() { assert_eq!(join_break_between(BreakBetween::Auto, BreakBetween::Column), BreakBetween::Column); }
#[test] fn edge_bb_avoid_auto() { assert_eq!(join_break_between(BreakBetween::Avoid, BreakBetween::Auto), BreakBetween::Avoid); }
#[test] fn edge_bb_page_avoid() { assert_eq!(join_break_between(BreakBetween::Page, BreakBetween::Avoid), BreakBetween::Page); }
#[test] fn edge_bb_left_page() { assert_eq!(join_break_between(BreakBetween::Left, BreakBetween::Page), BreakBetween::Left); }
#[test] fn edge_bb_col_right() { assert_eq!(join_break_between(BreakBetween::Column, BreakBetween::Right), BreakBetween::Right); }
#[test] fn edge_bb_avoid_apage() { assert_eq!(join_break_between(BreakBetween::Avoid, BreakBetween::AvoidPage), BreakBetween::AvoidPage); }
#[test] fn edge_bb_acol_avoid() { assert_eq!(join_break_between(BreakBetween::AvoidColumn, BreakBetween::Avoid), BreakBetween::AvoidColumn); }
#[test] fn edge_bb_page_col() { assert_eq!(join_break_between(BreakBetween::Page, BreakBetween::Column), BreakBetween::Page); }
#[test] fn edge_bb_recto_page() { assert_eq!(join_break_between(BreakBetween::Recto, BreakBetween::Page), BreakBetween::Recto); }
#[test] fn edge_bb_verso_col() { assert_eq!(join_break_between(BreakBetween::Verso, BreakBetween::Column), BreakBetween::Verso); }
#[test] fn edge_bb_right_left() { assert_eq!(join_break_between(BreakBetween::Right, BreakBetween::Left), BreakBetween::Right); }
#[test] fn edge_bb_auto_sym() {
    for v in [BreakBetween::Page, BreakBetween::Column, BreakBetween::Avoid, BreakBetween::Left] {
        assert_eq!(join_break_between(BreakBetween::Auto, v), v);
        assert_eq!(join_break_between(v, BreakBetween::Auto), v);
    }
}
#[test] fn edge_margin_scenario() {
    let c = ChildBreakInfo { break_before: BreakValue::Auto, break_after: BreakValue::Auto, break_inside: BreakInside::Auto, block_size: lu(120) };
    assert_eq!(find_best_break_point(&[c.clone(), c.clone(), c], &FragmentainerSpace::new(lu(300))).child_index, 2);
}
#[test] fn edge_all_zero() { assert_eq!(find_best_break_point(&vec![child(0); 10], &FragmentainerSpace::new(lu(0))).child_index, 10); }
#[test] fn edge_1px() { assert_eq!(find_best_break_point(&[child(1), child(1)], &FragmentainerSpace::new(lu(1))).child_index, 1); }
#[test] fn edge_large_small() { assert_eq!(find_best_break_point(&[child(10000)], &FragmentainerSpace::new(lu(1))).appeal, BreakAppeal::LastResort); }
#[test] fn edge_bal_single_large() { assert_eq!(balance_columns(&[lu(1000)], 3, lu(2000)), lu(1000)); }
#[test] fn edge_bal_zero_h() { assert_eq!(balance_columns(&[lu(0); 3], 2, lu(500)), lu(0)); }
#[test] fn edge_fill_balance_all() { assert_eq!(layout_columns(&ColumnLayoutAlgorithm { column_count: 2, column_width: None, column_gap: lu(10), column_fill: ColumnFill::BalanceAll, column_rule: None }, node(), lu(400), lu(500), &[lu(100); 2]).block_size, lu(100)); }
#[test] fn edge_large_avail() { let r = resolve_column_count_and_width(Some(3), None, lu(100_000), lu(10)); assert_eq!(r.count, 3); assert!(r.width.raw() > 0); }
#[test] fn edge_bb_default() { assert_eq!(BreakBetween::default(), BreakBetween::Auto); }
#[test] fn edge_bb_all_variants() { assert_eq!([BreakBetween::Auto, BreakBetween::Avoid, BreakBetween::AvoidPage, BreakBetween::AvoidColumn, BreakBetween::Page, BreakBetween::Left, BreakBetween::Right, BreakBetween::Recto, BreakBetween::Verso, BreakBetween::Column].len(), 10); }
#[test] fn edge_bv_all_variants() { assert_eq!([BreakValue::Auto, BreakValue::Avoid, BreakValue::AvoidPage, BreakValue::AvoidColumn, BreakValue::Page, BreakValue::Column, BreakValue::Left, BreakValue::Right, BreakValue::Always].len(), 9); }
#[test] fn edge_bi_all_variants() { assert_eq!([BreakInside::Auto, BreakInside::Avoid, BreakInside::AvoidPage, BreakInside::AvoidColumn].len(), 4); }
#[test] fn edge_cf_all_variants() { assert_eq!([ColumnFill::Balance, ColumnFill::BalanceAll, ColumnFill::Auto].len(), 3); }
#[test] fn edge_cf_initial() { assert_eq!(ColumnFill::INITIAL, ColumnFill::Balance); assert_eq!(ColumnFill::default(), ColumnFill::Balance); }
#[test] fn edge_bv_initial() { assert_eq!(BreakValue::INITIAL, BreakValue::Auto); assert_eq!(BreakValue::default(), BreakValue::Auto); }
#[test] fn edge_bi_initial() { assert_eq!(BreakInside::INITIAL, BreakInside::Auto); assert_eq!(BreakInside::default(), BreakInside::Auto); }
#[test] fn edge_many_forced() {
    let mut c: Vec<_> = (0..10).map(|_| child(10)).collect();
    c[1].break_before = BreakValue::Always; c[3].break_before = BreakValue::Page;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(10000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn edge_forced_after_first() {
    let mut c = vec![child(50); 4]; c[0].break_after = BreakValue::Column;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(10000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn edge_rule_with_algo() {
    use openui_style::BorderStyle;
    let a = ColumnLayoutAlgorithm { column_count: 3, column_width: None, column_gap: lu(20), column_fill: ColumnFill::Balance,
        column_rule: Some(ColumnRule { width: lu(2), style: BorderStyle::Solid, color: openui_style::Color::BLACK }) };
    assert!(layout_columns(&a, node(), lu(600), lu(500), &[lu(100); 3]).block_size.raw() > 0);
}
#[test] fn edge_avoid_both() {
    let mut c = vec![child(200); 2]; c[0].break_after = BreakValue::Avoid; c[1].break_before = BreakValue::Avoid;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(300))).appeal, BreakAppeal::LastResort);
}
#[test] fn edge_cbi_debug() { assert!(format!("{:?}", child(100)).contains("ChildBreakInfo")); }
#[test] fn edge_cbi_clone() { let c = child(100); assert_eq!(c.clone().block_size, c.block_size); }
#[test] fn edge_bal_max_small() { assert!(balance_columns(&[lu(100), lu(200), lu(50)], 3, lu(50)) >= lu(200)); }
#[test] fn edge_resolve_1col() { let r = resolve_column_count_and_width(None, Some(lu(1000)), lu(500), lu(0)); assert_eq!(r.count, 1); assert_eq!(r.width, lu(500)); }
#[test] fn edge_pos_5c() { assert_eq!(compute_column_positions(5, lu(100), lu(10), lu(540), false)[4].inline_offset, lu(440)); }
#[test] fn edge_with_width() { assert!(layout_columns(&algo_with_width(0, Some(200), 10, ColumnFill::Balance), node(), lu(640), lu(500), &[lu(100)]).block_size.raw() > 0); }
#[test] fn edge_exact_single() { assert_eq!(find_best_break_point(&[child(500)], &FragmentainerSpace::new(lu(500))).child_index, 1); }
#[test] fn edge_second_zero() { assert_eq!(find_best_break_point(&[child(400), child(0)], &FragmentainerSpace::new(lu(400))).child_index, 2); }
#[test] fn edge_all_forced_brk() {
    let mut c = vec![child(10); 3]; c[1].break_before = BreakValue::Always; c[2].break_before = BreakValue::Always;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 1); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn edge_mixed() {
    let mut c = vec![child(100); 3]; c[1].break_before = BreakValue::Avoid; c[2].break_before = BreakValue::Always;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(1000)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn edge_result_fields() {
    let r = layout_columns(&algo(2, 10, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(100); 2]);
    assert!(r.block_size.raw() > 0); assert!(!r.column_fragments.is_empty());
    assert_eq!(r.fragment.children.len(), r.column_fragments.len());
}
#[test] fn edge_bal_10i4c() { let h = balance_columns(&[lu(25); 10], 4, lu(1000)); assert!(h >= lu(50) && h <= lu(100)); }
#[test] fn edge_bb_debug() { assert!(format!("{:?}", BreakBetween::Page).contains("Page")); }
#[test] fn edge_bb_clone() { let a = BreakBetween::AvoidPage; let b = a; assert_eq!(a, b); }

// =========================================================================
// 9. ADDITIONAL COMPREHENSIVE TESTS (to reach 400+)
// =========================================================================

// -- More basic fragmentation --
#[test] fn extra_frag_two_then_one() {
    // 150+150=300 fits, 300+100=400>350
    assert_eq!(find_best_break_point(&[child(150), child(150), child(100)], &FragmentainerSpace::new(lu(350))).child_index, 2);
}
#[test] fn extra_frag_staircase() {
    // 10+20=30, 30+30=60, 60+40=100, 100+50=150>120
    assert_eq!(find_best_break_point(&[child(10), child(20), child(30), child(40), child(50)], &FragmentainerSpace::new(lu(120))).child_index, 4);
}
#[test] fn extra_frag_binary_split() {
    let c = vec![child(250), child(250)];
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(300))).child_index, 1);
}
#[test] fn extra_frag_all_one_px() {
    let c: Vec<_> = (0..50).map(|_| child(1)).collect();
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(25))).child_index, 25);
}
#[test] fn extra_frag_triple_boundary() {
    // 80+80=160<200, 160+80=240>200
    assert_eq!(find_best_break_point(&[child(80), child(80), child(80), child(80)], &FragmentainerSpace::new(lu(200))).child_index, 2);
}
#[test] fn extra_frag_four_fit() {
    assert_eq!(find_best_break_point(&[child(50), child(50), child(50), child(50)], &FragmentainerSpace::new(lu(200))).child_index, 4);
}
#[test] fn extra_frag_near_boundary() {
    // 199+199=398<400, 398+199>400
    assert_eq!(find_best_break_point(&[child(199), child(199), child(199)], &FragmentainerSpace::new(lu(400))).child_index, 2);
}
#[test] fn extra_frag_space_after_consume_3() {
    let mut s = FragmentainerSpace::new(lu(600)); s.consume(lu(100)); s.consume(lu(100)); s.consume(lu(100));
    assert_eq!(s.remaining(), lu(300)); assert_eq!(find_best_break_point(&[child(200), child(200)], &s).child_index, 1);
}
#[test] fn extra_frag_token_at_various_indices() {
    for i in [0, 1, 5, 10, 50, 100] { let t = BlockBreakToken::new(i, lu(0)); assert_eq!(t.child_index, i); }
}
#[test] fn extra_frag_token_various_consumed() {
    for v in [0, 1, 100, 1000, 50000] { let t = BlockBreakToken::new(0, lu(v)); assert_eq!(t.consumed_block_size, lu(v)); }
}

// -- More break properties --
#[test] fn extra_break_avoid_page_after() {
    let mut c = vec![child(200), child(200), child(200)]; c[0].break_after = BreakValue::AvoidPage;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(500))).child_index, 2);
}
#[test] fn extra_break_avoid_column_after() {
    let mut c = vec![child(200), child(200), child(200)]; c[0].break_after = BreakValue::AvoidColumn;
    assert_eq!(find_best_break_point(&c, &FragmentainerSpace::new(lu(500))).child_index, 2);
}
#[test] fn extra_break_forced_mid_4children() {
    let mut c = vec![child(100), child(100), child(100), child(100)];
    c[2].break_before = BreakValue::Page;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(10000)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn extra_break_column_after_mid() {
    let mut c = vec![child(100), child(100), child(100), child(100)];
    c[1].break_after = BreakValue::Column;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(10000)));
    assert_eq!(r.child_index, 2); assert_eq!(r.appeal, BreakAppeal::Perfect);
}
#[test] fn extra_break_avoid_both_boundaries() {
    let mut c = vec![child(100), child(100), child(100)];
    c[0].break_after = BreakValue::Avoid; c[1].break_after = BreakValue::Avoid;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(250)));
    assert_eq!(r.appeal, BreakAppeal::LastResort);
}
#[test] fn extra_should_break_left_right_after() {
    assert!(should_break_after(BreakValue::Left)); assert!(should_break_after(BreakValue::Right));
}

// -- More appeal --
#[test] fn extra_appeal_ne_variants() {
    assert_ne!(BreakAppeal::Perfect, BreakAppeal::Default);
    assert_ne!(BreakAppeal::Default, BreakAppeal::Violating);
    assert_ne!(BreakAppeal::Violating, BreakAppeal::LastResort);
}
#[test] fn extra_appeal_repr_values() {
    assert_eq!(BreakAppeal::LastResort as u8, 0);
    assert_eq!(BreakAppeal::Violating as u8, 1);
    assert_eq!(BreakAppeal::Default as u8, 2);
    assert_eq!(BreakAppeal::Perfect as u8, 3);
}
#[test] fn extra_appeal_point_debug() {
    let p = BreakPoint { child_index: 5, appeal: BreakAppeal::Perfect };
    let s = format!("{:?}", p); assert!(s.contains("BreakPoint"));
}
#[test] fn extra_appeal_ne_diff_appeal() {
    assert_ne!(BreakPoint { child_index: 1, appeal: BreakAppeal::Default }, BreakPoint { child_index: 1, appeal: BreakAppeal::Perfect });
}

// -- More multicol --
#[test] fn extra_mc_balance_4c8i() {
    assert_eq!(balance_columns(&[lu(50); 8], 4, lu(1000)), lu(100));
}
#[test] fn extra_mc_balance_5c10i() {
    assert_eq!(balance_columns(&[lu(20); 10], 5, lu(1000)), lu(40));
}
#[test] fn extra_mc_balance_2c_desc() {
    // 200, 100, 50 → total 350. 2 cols: col1=200+100=300, col2=50. Height=200.
    let h = balance_columns(&[lu(200), lu(100), lu(50)], 2, lu(1000));
    assert_eq!(h, lu(200));
}
#[test] fn extra_mc_count_1_gap() {
    let r = resolve_column_count_and_width(Some(1), None, lu(500), lu(100));
    assert_eq!(r.count, 1); assert_eq!(r.width, lu(500));
}
#[test] fn extra_mc_positions_2c_centered() {
    let p = compute_column_positions(2, lu(150), lu(0), lu(400), true);
    // total = 300. offset = 50
    assert_eq!(p[0].inline_offset, lu(50)); assert_eq!(p[1].inline_offset, lu(200));
}
#[test] fn extra_mc_layout_6i_3c() {
    let r = layout_columns(&algo(3, 0, ColumnFill::Balance), node(), lu(600), lu(500), &[lu(50); 6]);
    assert_eq!(r.block_size, lu(100));
}
#[test] fn extra_mc_layout_auto_no_block_size() {
    let r = layout_columns(&algo(2, 10, ColumnFill::Auto), node(), lu(400), lu(0), &[lu(100), lu(100)]);
    // Fallback: total = 200
    assert_eq!(r.block_size, lu(200));
}
#[test] fn extra_mc_width_plus_gap() {
    // width=150, gap=50: (150+50)=200. avail+gap=920/200=4
    let r = resolve_column_count_and_width(None, Some(lu(150)), lu(870), lu(50));
    assert_eq!(r.count, 4);
}
#[test] fn extra_mc_resolve_count_0() {
    // column_count=0 → clamped to 1
    let r = resolve_column_count_and_width(Some(0), None, lu(600), lu(10));
    assert_eq!(r.count, 1);
}
#[test] fn extra_mc_balance_single_item_2c() {
    assert_eq!(balance_columns(&[lu(200)], 2, lu(1000)), lu(200));
}
#[test] fn extra_mc_layout_result_size() {
    let r = layout_columns(&algo(2, 0, ColumnFill::Balance), node(), lu(400), lu(500), &[lu(100); 4]);
    assert_eq!(r.fragment.size.width, lu(400));
    assert_eq!(r.fragment.size.height, lu(200));
}
#[test] fn extra_mc_from_style_with_width() {
    use openui_geometry::Length;
    let mut s = ComputedStyle::initial(); s.column_width = Some(Length::px(200.0));
    assert!(ColumnLayoutAlgorithm::from_style(&s).is_some());
}
#[test] fn extra_mc_from_style_count_and_width() {
    use openui_geometry::Length;
    let mut s = ComputedStyle::initial(); s.column_count = Some(3); s.column_width = Some(Length::px(150.0));
    let a = ColumnLayoutAlgorithm::from_style(&s).unwrap();
    assert_eq!(a.column_count, 3);
}

// -- More nested fragmentation --
#[test] fn extra_nest_token_clone_deep() {
    let mut root = BlockBreakToken::new(0, lu(500));
    let mut mid = BlockBreakToken::new(0, lu(250));
    mid.add_child_token(BreakToken::Block(BlockBreakToken::new(0, lu(125))));
    root.add_child_token(BreakToken::Block(mid));
    let cloned = root.clone();
    assert_eq!(cloned.child_break_tokens.len(), 1);
    match &cloned.child_break_tokens[0] { BreakToken::Block(i) => assert_eq!(i.child_break_tokens.len(), 1) }
}
#[test] fn extra_nest_multicol_auto_inner() {
    let inner = algo(2, 5, ColumnFill::Auto);
    let r = layout_columns(&inner, node(), lu(200), lu(100), &[lu(80), lu(80)]);
    assert_eq!(r.block_size, lu(100));
}
#[test] fn extra_nest_break_with_children_5() {
    let mut p = BlockBreakToken::new(0, lu(1000));
    for i in 0..5 { p.add_child_token(BreakToken::Block(BlockBreakToken::new(i, lu((i+1) as i32 * 50)))); }
    assert_eq!(p.child_break_tokens.len(), 5);
    match &p.child_break_tokens[4] { BreakToken::Block(b) => assert_eq!(b.consumed_block_size, lu(250)) }
}

// -- More fragmentainer space --
#[test] fn extra_fs_consume_tiny_increments() {
    let mut s = FragmentainerSpace::new(lu(10));
    for _ in 0..10 { s.consume(lu(1)); }
    assert!(s.is_exhausted());
}
#[test] fn extra_fs_remaining_after_each_consume() {
    let mut s = FragmentainerSpace::new(lu(50));
    for i in 1..=5 { s.consume(lu(10)); assert_eq!(s.remaining(), lu(50 - i * 10)); }
}
#[test] fn extra_fs_block_offset_tracks() {
    let mut s = FragmentainerSpace::new(lu(200));
    s.consume(lu(75)); assert_eq!(s.block_offset, lu(75));
    s.consume(lu(25)); assert_eq!(s.block_offset, lu(100));
}
#[test] fn extra_fs_not_start_after_zero() {
    let mut s = FragmentainerSpace::new(lu(100));
    s.consume(lu(0));
    assert!(!s.is_at_block_start);
}

// -- More edge cases --
#[test] fn extra_edge_bb_page_page() {
    assert_eq!(join_break_between(BreakBetween::Page, BreakBetween::Page), BreakBetween::Page);
}
#[test] fn extra_edge_bb_col_col() {
    assert_eq!(join_break_between(BreakBetween::Column, BreakBetween::Column), BreakBetween::Column);
}
#[test] fn extra_edge_bb_avoid_avoid() {
    assert_eq!(join_break_between(BreakBetween::Avoid, BreakBetween::Avoid), BreakBetween::Avoid);
}
#[test] fn extra_edge_balance_2c_single_item() {
    assert_eq!(balance_columns(&[lu(300)], 2, lu(500)), lu(300));
}
#[test] fn extra_edge_bal_max_capped() {
    // With a small max_height, the binary search still converges.
    let h = balance_columns(&[lu(100); 4], 2, lu(100));
    // 4 items of 100 in 2 cols needs height 200. With max 100, needs 4 cols.
    // lo starts at 100 (max child), hi = min(400, 100) = 100. So h=100.
    assert_eq!(h, lu(100));
}
#[test] fn extra_edge_layout_single_col_count() {
    // column-count: 1 is a valid passthrough
    let r = layout_columns(&algo(1, 0, ColumnFill::Balance), node(), lu(600), lu(500), &[lu(100), lu(200)]);
    assert_eq!(r.block_size, lu(300));
}
#[test] fn extra_edge_bb_recto_recto() {
    assert_eq!(join_break_between(BreakBetween::Recto, BreakBetween::Recto), BreakBetween::Recto);
}
#[test] fn extra_edge_bb_verso_verso() {
    assert_eq!(join_break_between(BreakBetween::Verso, BreakBetween::Verso), BreakBetween::Verso);
}
#[test] fn extra_edge_bb_left_left() {
    assert_eq!(join_break_between(BreakBetween::Left, BreakBetween::Left), BreakBetween::Left);
}
#[test] fn extra_edge_bb_right_right() {
    assert_eq!(join_break_between(BreakBetween::Right, BreakBetween::Right), BreakBetween::Right);
}
#[test] fn extra_edge_break_default() {
    assert_eq!(BreakValue::default(), BreakValue::Auto);
    assert_eq!(BreakInside::default(), BreakInside::Auto);
    assert_eq!(ColumnFill::default(), ColumnFill::Balance);
    assert_eq!(ColumnSpan::default(), ColumnSpan::None);
    assert_eq!(BreakBetween::default(), BreakBetween::Auto);
}
#[test] fn extra_edge_mc_positions_stride() {
    let p = compute_column_positions(5, lu(80), lu(20), lu(480), false);
    for (i, col) in p.iter().enumerate() {
        assert_eq!(col.inline_offset, lu(i as i32 * 100));
        assert_eq!(col.width, lu(80));
    }
}
#[test] fn extra_edge_break_appeal_partial_ord() {
    assert!(BreakAppeal::Perfect >= BreakAppeal::Perfect);
    assert!(BreakAppeal::Default <= BreakAppeal::Perfect);
    assert!(BreakAppeal::LastResort <= BreakAppeal::Violating);
}
#[test] fn extra_edge_mc_rule_count_5() {
    assert_eq!(compute_column_rule_positions(5, lu(100), lu(20)).len(), 4);
}
#[test] fn extra_edge_mc_rule_count_2() {
    assert_eq!(compute_column_rule_positions(2, lu(200), lu(40)).len(), 1);
}
#[test] fn extra_edge_mc_layout_10i_5c() {
    let r = layout_columns(&algo(5, 0, ColumnFill::Balance), node(), lu(500), lu(500), &[lu(40); 10]);
    assert_eq!(r.block_size, lu(80));
}
#[test] fn extra_edge_break_value_repr() {
    assert_eq!(BreakValue::Auto as u8, 0);
    assert_eq!(BreakValue::Always as u8, 8);
}
#[test] fn extra_edge_break_inside_repr() {
    assert_eq!(BreakInside::Auto as u8, 0);
    assert_eq!(BreakInside::Avoid as u8, 1);
    assert_eq!(BreakInside::AvoidPage as u8, 2);
    assert_eq!(BreakInside::AvoidColumn as u8, 3);
}
#[test] fn extra_edge_column_fill_repr() {
    assert_eq!(ColumnFill::Balance as u8, 0);
    assert_eq!(ColumnFill::BalanceAll as u8, 1);
    assert_eq!(ColumnFill::Auto as u8, 2);
}
#[test] fn extra_edge_resolved_cols_debug() {
    let r = resolve_column_count_and_width(Some(2), None, lu(400), lu(20));
    let s = format!("{:?}", r); assert!(s.contains("ResolvedColumns"));
}
#[test] fn extra_edge_resolved_cols_eq() {
    let a = resolve_column_count_and_width(Some(2), None, lu(400), lu(20));
    let b = resolve_column_count_and_width(Some(2), None, lu(400), lu(20));
    assert_eq!(a, b);
}
#[test] fn extra_edge_layout_result_debug() {
    let r = layout_columns(&algo(2, 0, ColumnFill::Balance), node(), lu(200), lu(200), &[lu(50), lu(50)]);
    let s = format!("{:?}", r); assert!(s.contains("ColumnLayoutResult"));
}
#[test] fn extra_mc_layout_many_cols() {
    let r = layout_columns(&algo(10, 0, ColumnFill::Balance), node(), lu(1000), lu(1000), &[lu(50); 10]);
    assert_eq!(r.block_size, lu(50));
}
#[test] fn extra_mc_balance_all_ones() {
    assert_eq!(balance_columns(&[lu(1); 100], 10, lu(1000)), lu(10));
}
#[test] fn extra_frag_avoid_after_second() {
    let mut c = vec![child(100), child(100), child(100)];
    c[1].break_after = BreakValue::Avoid;
    let r = find_best_break_point(&c, &FragmentainerSpace::new(lu(250)));
    assert_eq!(r.child_index, 1); // Break before child 1 is Default
}
#[test] fn extra_frag_mixed_sizes() {
    // 25+75=100, 100+125=225<250, 225+175=400>250
    assert_eq!(find_best_break_point(&[child(25), child(75), child(125), child(175)], &FragmentainerSpace::new(lu(250))).child_index, 3);
}
#[test] fn extra_frag_fragspace_half_then_break() {
    let mut s = FragmentainerSpace::new(lu(600));
    s.consume(lu(300)); // half
    assert_eq!(find_best_break_point(&[child(200), child(200)], &s).child_index, 1);
}
#[test] fn extra_nest_6_child_tokens() {
    let mut p = BlockBreakToken::new(0, lu(600));
    for i in 0..6 { p.add_child_token(BreakToken::Block(BlockBreakToken::new(i, lu(100)))); }
    assert_eq!(p.child_break_tokens.len(), 6);
}
#[test] fn extra_mc_col_fill_balance_all_same() {
    let a = ColumnLayoutAlgorithm { column_count: 3, column_width: None, column_gap: lu(0), column_fill: ColumnFill::BalanceAll, column_rule: None };
    let r = layout_columns(&a, node(), lu(300), lu(500), &[lu(60); 6]);
    assert_eq!(r.block_size, lu(120));
}
#[test] fn extra_mc_wide_gaps() {
    let r = resolve_column_count_and_width(Some(2), None, lu(600), lu(200));
    assert_eq!(r.count, 2); assert_eq!(r.width, lu(200));
}
#[test] fn extra_fs_new_various_sizes() {
    for size in [1, 10, 100, 1000, 10000] {
        let s = FragmentainerSpace::new(lu(size));
        assert_eq!(s.remaining(), lu(size)); assert!(s.is_at_block_start);
    }
}
#[test] fn extra_break_after_avoid_page() {
    assert!(!should_break_after(BreakValue::AvoidPage));
    assert!(!should_break_after(BreakValue::AvoidColumn));
}
#[test] fn extra_break_before_after_appeal_symmetry() {
    for v in [BreakValue::Auto, BreakValue::Avoid, BreakValue::Always, BreakValue::Page, BreakValue::Column] {
        assert_eq!(break_before_appeal(v), break_after_appeal(v));
    }
}
#[test] fn extra_mc_layout_preserves_node() {
    let r = layout_columns(&algo(2, 0, ColumnFill::Balance), node(), lu(200), lu(200), &[lu(50)]);
    assert_eq!(r.fragment.node_id, node());
}
#[test] fn extra_break_value_is_not_forced_auto() {
    assert!(!BreakValue::Auto.is_forced());
    assert!(!BreakValue::Auto.is_avoid());
}
#[test] fn extra_mc_count_0_resolves() {
    // When column_count is 0 it gets clamped to 1
    let a = algo(0, 0, ColumnFill::Balance);
    let r = layout_columns(&a, node(), lu(400), lu(200), &[lu(100)]);
    assert!(r.block_size.raw() > 0);
}
#[test] fn extra_frag_break_token_eq_consumed() {
    let a = BlockBreakToken::new(0, lu(100));
    let b = BlockBreakToken::new(0, lu(100));
    assert_eq!(a.child_index, b.child_index);
    assert_eq!(a.consumed_block_size, b.consumed_block_size);
}
#[test] fn extra_mc_bal_3c_asymmetric() {
    // 300, 100, 100 → total 500, 3 cols.
    let h = balance_columns(&[lu(300), lu(100), lu(100)], 3, lu(1000));
    assert_eq!(h, lu(300));
}
#[test] fn extra_mc_layout_auto_with_many_items() {
    let a = algo(3, 10, ColumnFill::Auto);
    let r = layout_columns(&a, node(), lu(640), lu(200), &[lu(50); 12]);
    assert_eq!(r.block_size, lu(200));
}
#[test] fn extra_edge_bb_avoid_page_page() {
    // forced > avoid
    assert_eq!(join_break_between(BreakBetween::AvoidPage, BreakBetween::Page), BreakBetween::Page);
}
#[test] fn extra_edge_cbi_field_access() {
    let c = child(42);
    assert_eq!(c.block_size, lu(42));
    assert_eq!(c.break_before, BreakValue::Auto);
    assert_eq!(c.break_after, BreakValue::Auto);
    assert_eq!(c.break_inside, BreakInside::Auto);
}
#[test] fn extra_mc_positions_equal_spacing() {
    let p = compute_column_positions(3, lu(100), lu(50), lu(400), false);
    assert_eq!(p[1].inline_offset - p[0].inline_offset, lu(150));
    assert_eq!(p[2].inline_offset - p[1].inline_offset, lu(150));
}
#[test] fn extra_frag_space_exhausted_then_new() {
    let mut s = FragmentainerSpace::new(lu(100));
    s.consume(lu(100)); assert!(s.is_exhausted());
    let s2 = FragmentainerSpace::new(lu(100)); assert!(!s2.is_exhausted());
}
#[test] fn extra_frag_child_debug_clone() {
    let c = child(77);
    let c2 = c.clone();
    let d = format!("{:?}", c2);
    assert!(d.contains("77") || d.contains("ChildBreakInfo"));
}
#[test] fn extra_mc_bal_2c2i_equal() {
    assert_eq!(balance_columns(&[lu(150), lu(150)], 2, lu(1000)), lu(150));
}
#[test] fn extra_edge_bb_auto_self() {
    assert_eq!(join_break_between(BreakBetween::Auto, BreakBetween::Auto), BreakBetween::Auto);
}
#[test] fn extra_edge_rule_zero_width() {
    let rules = compute_column_rule_positions(3, lu(100), lu(0));
    assert_eq!(rules.len(), 2);
}