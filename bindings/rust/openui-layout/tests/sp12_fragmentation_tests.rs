//! SP12 G1 — Block Fragmentation integration tests.
//!
//! Tests for break tokens, break appeal ranking, fragmentainer space tracking,
//! break-point selection, and break-between join/merge logic.

use openui_geometry::LayoutUnit;
use openui_layout::fragmentation::{
    BlockBreakToken, BreakAppeal, BreakPoint, BreakToken, ChildBreakInfo,
    FragmentainerSpace, find_best_break_point, should_break_before, should_break_after,
    break_before_appeal, break_after_appeal, join_break_between,
};
use openui_layout::BreakBetween;
use openui_style::{BreakValue, BreakInside};

// ── FragmentainerSpace ──────────────────────────────────────────────────

#[test]
fn fragmentainer_remaining_calculation() {
    let space = FragmentainerSpace {
        block_size: LayoutUnit::from_i32(800),
        block_offset: LayoutUnit::from_i32(350),
        is_at_block_start: false,
    };
    assert_eq!(space.remaining(), LayoutUnit::from_i32(450));
}

#[test]
fn fragmentainer_exhausted_when_full() {
    let space = FragmentainerSpace {
        block_size: LayoutUnit::from_i32(500),
        block_offset: LayoutUnit::from_i32(500),
        is_at_block_start: false,
    };
    assert!(space.is_exhausted());
}

#[test]
fn fragmentainer_exhausted_when_overflowed() {
    let space = FragmentainerSpace {
        block_size: LayoutUnit::from_i32(500),
        block_offset: LayoutUnit::from_i32(600),
        is_at_block_start: false,
    };
    assert!(space.is_exhausted());
}

#[test]
fn fragmentainer_not_exhausted_with_space() {
    let space = FragmentainerSpace::new(LayoutUnit::from_i32(1000));
    assert!(!space.is_exhausted());
    assert_eq!(space.remaining(), LayoutUnit::from_i32(1000));
}

#[test]
fn fragmentainer_consume_updates_state() {
    let mut space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
    assert!(space.is_at_block_start);
    space.consume(LayoutUnit::from_i32(200));
    assert_eq!(space.remaining(), LayoutUnit::from_i32(300));
    assert!(!space.is_at_block_start);
    space.consume(LayoutUnit::from_i32(300));
    assert!(space.is_exhausted());
}

// ── BreakAppeal ordering ────────────────────────────────────────────────

#[test]
fn break_appeal_ordering_perfect_gt_default() {
    assert!(BreakAppeal::Perfect > BreakAppeal::Default);
}

#[test]
fn break_appeal_ordering_default_gt_violating() {
    assert!(BreakAppeal::Default > BreakAppeal::Violating);
}

#[test]
fn break_appeal_ordering_violating_gt_last_resort() {
    assert!(BreakAppeal::Violating > BreakAppeal::LastResort);
}

#[test]
fn break_appeal_full_chain() {
    let appeals = [
        BreakAppeal::LastResort,
        BreakAppeal::Violating,
        BreakAppeal::Default,
        BreakAppeal::Perfect,
    ];
    for window in appeals.windows(2) {
        assert!(window[0] < window[1], "{:?} should be < {:?}", window[0], window[1]);
    }
}

// ── BlockBreakToken ─────────────────────────────────────────────────────

#[test]
fn block_break_token_new() {
    let token = BlockBreakToken::new(2, LayoutUnit::from_i32(150));
    assert_eq!(token.child_index, 2);
    assert_eq!(token.consumed_block_size, LayoutUnit::from_i32(150));
    assert!(!token.is_break_before);
    assert!(token.child_break_tokens.is_empty());
}

#[test]
fn block_break_token_break_before_flag() {
    let token = BlockBreakToken::break_before(5, LayoutUnit::from_i32(400));
    assert!(token.is_break_before);
    assert_eq!(token.child_index, 5);
    assert_eq!(token.consumed_block_size, LayoutUnit::from_i32(400));
}

#[test]
fn break_token_chaining_parent_and_child() {
    let mut parent = BlockBreakToken::new(1, LayoutUnit::from_i32(300));
    let child = BlockBreakToken::new(0, LayoutUnit::from_i32(100));
    parent.add_child_token(BreakToken::Block(child));

    assert!(parent.has_child_break_tokens());
    assert_eq!(parent.child_break_tokens.len(), 1);

    match &parent.child_break_tokens[0] {
        BreakToken::Block(inner) => {
            assert_eq!(inner.child_index, 0);
            assert_eq!(inner.consumed_block_size, LayoutUnit::from_i32(100));
        }
    }
}

#[test]
fn break_token_multiple_children() {
    let mut parent = BlockBreakToken::new(3, LayoutUnit::from_i32(500));
    parent.add_child_token(BreakToken::Block(
        BlockBreakToken::new(0, LayoutUnit::from_i32(50)),
    ));
    parent.add_child_token(BreakToken::Block(
        BlockBreakToken::new(1, LayoutUnit::from_i32(100)),
    ));
    assert_eq!(parent.child_break_tokens.len(), 2);
}

// ── find_best_break_point ───────────────────────────────────────────────

fn child(size: i32) -> ChildBreakInfo {
    ChildBreakInfo {
        break_before: BreakValue::Auto,
        break_after: BreakValue::Auto,
        break_inside: BreakInside::Auto,
        block_size: LayoutUnit::from_i32(size),
    }
}

#[test]
fn no_break_when_all_content_fits() {
    let children = vec![child(100), child(100), child(100)];
    let space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
    let result = find_best_break_point(&children, &space);

    // All fit — index == children.len()
    assert_eq!(result.child_index, children.len());
    assert_eq!(result.appeal, BreakAppeal::Default);
}

#[test]
fn break_at_fragmentainer_boundary() {
    // 3 children of 200px each, fragmentainer = 500px
    // Children 0+1 = 400 fit. Child 2 would overflow at 600.
    let children = vec![child(200), child(200), child(200)];
    let space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
    let result = find_best_break_point(&children, &space);

    assert_eq!(result.child_index, 2); // break before child 2
    assert_eq!(result.appeal, BreakAppeal::Default);
}

#[test]
fn forced_break_before_always() {
    let mut children = vec![child(50), child(50), child(50)];
    children[1].break_before = BreakValue::Always;

    let space = FragmentainerSpace::new(LayoutUnit::from_i32(1000));
    let result = find_best_break_point(&children, &space);

    // Forced break before child 1, even though content fits
    assert_eq!(result.child_index, 1);
    assert_eq!(result.appeal, BreakAppeal::Perfect);
}

#[test]
fn forced_break_before_page() {
    let mut children = vec![child(50), child(50)];
    children[1].break_before = BreakValue::Page;

    let space = FragmentainerSpace::new(LayoutUnit::from_i32(1000));
    let result = find_best_break_point(&children, &space);

    assert_eq!(result.child_index, 1);
    assert_eq!(result.appeal, BreakAppeal::Perfect);
}

#[test]
fn forced_break_after_always() {
    let mut children = vec![child(50), child(50), child(50)];
    children[0].break_after = BreakValue::Always;

    let space = FragmentainerSpace::new(LayoutUnit::from_i32(1000));
    let result = find_best_break_point(&children, &space);

    assert_eq!(result.child_index, 1);
    assert_eq!(result.appeal, BreakAppeal::Perfect);
}

#[test]
fn break_avoid_demotes_to_last_resort() {
    // Child 1 has break-before: avoid. If we must break there, appeal is LastResort.
    let mut children = vec![child(200), child(200), child(200)];
    children[1].break_before = BreakValue::Avoid;
    // Child 2 has normal break-before (auto) → Default

    let space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
    let result = find_best_break_point(&children, &space);

    // Should prefer breaking before child 2 (Default) over child 1 (LastResort)
    assert_eq!(result.child_index, 2);
    assert_eq!(result.appeal, BreakAppeal::Default);
}

#[test]
fn break_avoid_all_candidates_last_resort() {
    // Both boundaries have avoid — must pick one as LastResort.
    let mut children = vec![child(200), child(200), child(200)];
    children[1].break_before = BreakValue::Avoid;
    children[2].break_before = BreakValue::Avoid;

    let space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
    let result = find_best_break_point(&children, &space);

    assert_eq!(result.appeal, BreakAppeal::LastResort);
    // Should break before child 2 (later index preferred)
    assert_eq!(result.child_index, 2);
}

#[test]
fn empty_children_returns_default() {
    let children: Vec<ChildBreakInfo> = vec![];
    let space = FragmentainerSpace::new(LayoutUnit::from_i32(500));
    let result = find_best_break_point(&children, &space);
    assert_eq!(result.child_index, 0);
    assert_eq!(result.appeal, BreakAppeal::Default);
}

// ── should_break_before / should_break_after ────────────────────────────

#[test]
fn should_break_before_forced_values() {
    assert!(should_break_before(BreakValue::Always));
    assert!(should_break_before(BreakValue::Page));
    assert!(should_break_before(BreakValue::Column));
    assert!(should_break_before(BreakValue::Left));
    assert!(should_break_before(BreakValue::Right));
}

#[test]
fn should_break_before_non_forced_values() {
    assert!(!should_break_before(BreakValue::Auto));
    assert!(!should_break_before(BreakValue::Avoid));
    assert!(!should_break_before(BreakValue::AvoidPage));
    assert!(!should_break_before(BreakValue::AvoidColumn));
}

#[test]
fn should_break_after_forced_values() {
    assert!(should_break_after(BreakValue::Always));
    assert!(should_break_after(BreakValue::Page));
    assert!(should_break_after(BreakValue::Column));
    assert!(should_break_after(BreakValue::Left));
    assert!(should_break_after(BreakValue::Right));
}

#[test]
fn should_break_after_non_forced_values() {
    assert!(!should_break_after(BreakValue::Auto));
    assert!(!should_break_after(BreakValue::Avoid));
}

// ── break appeal helpers ────────────────────────────────────────────────

#[test]
fn break_before_appeal_forced() {
    assert_eq!(break_before_appeal(BreakValue::Always), BreakAppeal::Perfect);
    assert_eq!(break_before_appeal(BreakValue::Page), BreakAppeal::Perfect);
}

#[test]
fn break_before_appeal_avoid() {
    assert_eq!(break_before_appeal(BreakValue::Avoid), BreakAppeal::LastResort);
    assert_eq!(break_before_appeal(BreakValue::AvoidPage), BreakAppeal::LastResort);
}

#[test]
fn break_before_appeal_auto() {
    assert_eq!(break_before_appeal(BreakValue::Auto), BreakAppeal::Default);
}

// ── Legacy page-break-* property handling ───────────────────────────────

#[test]
fn legacy_page_break_before_always() {
    let value = BreakValue::from_legacy_page_break("always");
    assert_eq!(value, BreakValue::Page);
    assert!(should_break_before(value));
}

#[test]
fn legacy_page_break_before_avoid() {
    let value = BreakValue::from_legacy_page_break("avoid");
    assert_eq!(value, BreakValue::Avoid);
    assert!(!should_break_before(value));
}

#[test]
fn legacy_page_break_before_auto() {
    let value = BreakValue::from_legacy_page_break("auto");
    assert_eq!(value, BreakValue::Auto);
}

#[test]
fn legacy_page_break_left_right() {
    assert_eq!(BreakValue::from_legacy_page_break("left"), BreakValue::Left);
    assert_eq!(BreakValue::from_legacy_page_break("right"), BreakValue::Right);
}

#[test]
fn legacy_page_break_inside_avoid() {
    let value = BreakInside::from_legacy_page_break_inside("avoid");
    assert_eq!(value, BreakInside::Avoid);
    assert!(value.is_avoid());
}

#[test]
fn legacy_page_break_inside_auto() {
    let value = BreakInside::from_legacy_page_break_inside("auto");
    assert_eq!(value, BreakInside::Auto);
    assert!(!value.is_avoid());
}

// ── BreakBetween join/merge ─────────────────────────────────────────────

#[test]
fn join_break_between_auto_yields_other() {
    assert_eq!(
        join_break_between(BreakBetween::Auto, BreakBetween::Column),
        BreakBetween::Column,
    );
    assert_eq!(
        join_break_between(BreakBetween::Avoid, BreakBetween::Auto),
        BreakBetween::Avoid,
    );
    assert_eq!(
        join_break_between(BreakBetween::Auto, BreakBetween::Auto),
        BreakBetween::Auto,
    );
}

#[test]
fn join_break_between_forced_wins_over_avoid() {
    assert_eq!(
        join_break_between(BreakBetween::Page, BreakBetween::Avoid),
        BreakBetween::Page,
    );
    assert_eq!(
        join_break_between(BreakBetween::AvoidPage, BreakBetween::Column),
        BreakBetween::Column,
    );
}

#[test]
fn join_break_between_stronger_forced_wins() {
    assert_eq!(
        join_break_between(BreakBetween::Left, BreakBetween::Page),
        BreakBetween::Left,
    );
    assert_eq!(
        join_break_between(BreakBetween::Column, BreakBetween::Right),
        BreakBetween::Right,
    );
}

#[test]
fn join_break_between_avoid_specificity() {
    assert_eq!(
        join_break_between(BreakBetween::Avoid, BreakBetween::AvoidPage),
        BreakBetween::AvoidPage,
    );
    assert_eq!(
        join_break_between(BreakBetween::AvoidColumn, BreakBetween::Avoid),
        BreakBetween::AvoidColumn,
    );
}

// ── ComputedStyle fragmentation properties ──────────────────────────────

#[test]
fn computed_style_fragmentation_defaults() {
    let style = openui_style::ComputedStyle::initial();
    assert_eq!(style.break_before, BreakValue::Auto);
    assert_eq!(style.break_after, BreakValue::Auto);
    assert_eq!(style.break_inside, BreakInside::Auto);
}
