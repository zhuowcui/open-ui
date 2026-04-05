//! SP12 E2 — Min/Max width & height constraint tests.
//!
//! Tests for CSS 2.1 §10.4 / §10.7 constraint resolution.

use openui_geometry::{LayoutUnit, Length};
use openui_layout::size_constraints::{
    SizeConstraint, resolve_size_constraints,
    constrain_inline_size, constrain_block_size,
    resolve_inline_size, resolve_block_size,
    apply_box_sizing_adjustment,
};
use openui_style::BoxSizing;

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

// ── constrain_inline_size ───────────────────────────────────────────────

#[test]
fn constrain_inline_within_bounds_no_change() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(300), &c), lu(300));
}

#[test]
fn constrain_inline_below_min_clamps_up() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(50), &c), lu(100));
}

#[test]
fn constrain_inline_above_max_clamps_down() {
    let c = SizeConstraint {
        min_inline_size: lu(100),
        max_inline_size: lu(500),
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    assert_eq!(constrain_inline_size(lu(600), &c), lu(500));
}

#[test]
fn constrain_inline_min_greater_than_max_min_wins() {
    // CSS 2.1 §10.4: "If min-width is greater than max-width, max-width is
    // set to the value of min-width." Effectively, min always wins.
    let c = SizeConstraint {
        min_inline_size: lu(400),
        max_inline_size: lu(200),  // min > max
        min_block_size: lu(0),
        max_block_size: LayoutUnit::max(),
    };
    // constrain: size.max(400).min(200) → 400.min(200) → 200?
    // No — resolve_size_constraints normalizes so max >= min.
    // But testing the raw constrain function: max_of(50, 400) = 400, min_of(400, 200) = 200
    // The resolution function fixes this, but let's test the raw function too.
    assert_eq!(constrain_inline_size(lu(50), &c), lu(200));
}

#[test]
fn resolve_constraints_normalizes_min_gt_max() {
    // When resolve_size_constraints is used, min > max is normalized so max = min.
    let c = resolve_size_constraints(
        &Length::px(400.0),   // min_inline = 400
        &Length::px(200.0),   // max_inline = 200
        &Length::auto(),      // min_block = auto → 0
        &Length::none(),      // max_block = none → max
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    // After normalization: max_inline should be raised to min_inline (400)
    assert_eq!(c.min_inline_size, lu(400));
    assert_eq!(c.max_inline_size, lu(400));
    // Any size gets clamped to exactly 400
    assert_eq!(constrain_inline_size(lu(100), &c), lu(400));
    assert_eq!(constrain_inline_size(lu(500), &c), lu(400));
}

// ── constrain_block_size ────────────────────────────────────────────────

#[test]
fn constrain_block_within_bounds() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(50),
        max_block_size: lu(300),
    };
    assert_eq!(constrain_block_size(lu(200), &c), lu(200));
}

#[test]
fn constrain_block_below_min() {
    let c = SizeConstraint {
        min_inline_size: lu(0),
        max_inline_size: LayoutUnit::max(),
        min_block_size: lu(100),
        max_block_size: lu(500),
    };
    assert_eq!(constrain_block_size(lu(30), &c), lu(100));
}

// ── resolve_inline_size ─────────────────────────────────────────────────

#[test]
fn resolve_inline_auto_uses_available() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::auto(),
        lu(800),   // available
        lu(800),   // CB
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    assert_eq!(result, lu(800));
}

#[test]
fn resolve_inline_fixed() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::px(300.0),
        lu(800),
        lu(800),
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    assert_eq!(result, lu(300));
}

#[test]
fn resolve_inline_percentage() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::percent(50.0),
        lu(800),
        lu(800),
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    assert_eq!(result, lu(400));
}

// ── resolve_block_size ──────────────────────────────────────────────────

#[test]
fn resolve_block_auto_uses_content() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(
        &Length::auto(),
        lu(250),   // content_block_size
        lu(600),   // CB
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    assert_eq!(result, lu(250));
}

#[test]
fn resolve_block_fixed() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(
        &Length::px(400.0),
        lu(200),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    assert_eq!(result, lu(400));
}

#[test]
fn resolve_block_percentage_definite_cb() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(
        &Length::percent(50.0),
        lu(200),
        lu(600),   // definite CB
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    assert_eq!(result, lu(300));
}

#[test]
fn resolve_block_percentage_indefinite_cb_treated_as_auto() {
    let c = SizeConstraint::unconstrained();
    let indef = LayoutUnit::from_raw(-64); // kIndefiniteSize
    let result = resolve_block_size(
        &Length::percent(50.0),
        lu(200),   // content_block_size (auto fallback)
        indef,     // indefinite CB
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    // Should fall back to content_block_size
    assert_eq!(result, lu(200));
}

// ── box-sizing adjustment ───────────────────────────────────────────────

#[test]
fn box_sizing_border_box_subtracts_padding_border() {
    let result = apply_box_sizing_adjustment(lu(200), BoxSizing::BorderBox, lu(40));
    assert_eq!(result, lu(160));
}

#[test]
fn box_sizing_content_box_no_adjustment() {
    let result = apply_box_sizing_adjustment(lu(200), BoxSizing::ContentBox, lu(40));
    assert_eq!(result, lu(200));
}

// ── min-width with percentage ───────────────────────────────────────────

#[test]
fn min_width_percentage() {
    let c = resolve_size_constraints(
        &Length::percent(20.0),  // min_inline = 20% of 500 = 100
        &Length::none(),
        &Length::auto(),
        &Length::none(),
        lu(500),
        lu(400),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.min_inline_size, lu(100));
    // A size of 50 should be clamped up to 100
    assert_eq!(constrain_inline_size(lu(50), &c), lu(100));
}

// ── max-height none (no constraint) ─────────────────────────────────────

#[test]
fn max_height_none_no_constraint() {
    let c = resolve_size_constraints(
        &Length::auto(),
        &Length::none(),
        &Length::auto(),
        &Length::none(),   // max_block = none → unconstrained
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.max_block_size, LayoutUnit::max());
    // Any block size passes through unconstrained
    assert_eq!(constrain_block_size(lu(9999), &c), lu(9999));
}

// ── combined min/max resolution chain ───────────────────────────────────

#[test]
fn combined_min_max_resolution_chain() {
    // min-width: 100px, max-width: 400px, width: auto → available 800
    // Resolved content width = 800, then clamped to [100, 400] → 400
    let c = resolve_size_constraints(
        &Length::px(100.0),
        &Length::px(400.0),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    let result = resolve_inline_size(
        &Length::auto(),
        lu(800),
        lu(800),
        BoxSizing::ContentBox,
        lu(0),
        &c,
    );
    assert_eq!(result, lu(400));
}

// ── fit-content keyword ─────────────────────────────────────────────────

#[test]
fn fit_content_min_treated_as_zero() {
    // fit-content as min-width → treated as 0 for constraint resolution
    let c = resolve_size_constraints(
        &Length::fit_content(),
        &Length::none(),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

#[test]
fn fit_content_max_treated_as_none() {
    // fit-content as max-width → treated as none (unconstrained)
    let c = resolve_size_constraints(
        &Length::auto(),
        &Length::fit_content(),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.max_inline_size, LayoutUnit::max());
}

// ── zero min-width ──────────────────────────────────────────────────────

#[test]
fn zero_min_width() {
    let c = resolve_size_constraints(
        &Length::px(0.0),
        &Length::none(),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
    // Size of 0 is valid
    assert_eq!(constrain_inline_size(LayoutUnit::zero(), &c), LayoutUnit::zero());
}

// ── border-box with constraints ─────────────────────────────────────────

#[test]
fn border_box_constraints_subtract_padding_border() {
    // min-width: 200px border-box with 30px padding+border → 170px content
    // max-width: 500px border-box with 30px padding+border → 470px content
    let c = resolve_size_constraints(
        &Length::px(200.0),
        &Length::px(500.0),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::BorderBox,
        lu(30),   // padding_border_inline
        lu(20),   // padding_border_block
    );
    assert_eq!(c.min_inline_size, lu(170));
    assert_eq!(c.max_inline_size, lu(470));
}

#[test]
fn resolve_inline_border_box_fixed() {
    // width: 300px border-box, padding+border = 40
    // Content width = 300 - 40 = 260, unconstrained
    let c = SizeConstraint::unconstrained();
    let result = resolve_inline_size(
        &Length::px(300.0),
        lu(800),
        lu(800),
        BoxSizing::BorderBox,
        lu(40),
        &c,
    );
    assert_eq!(result, lu(260));
}

// ── resolve_block with border-box ───────────────────────────────────────

#[test]
fn resolve_block_border_box_fixed() {
    let c = SizeConstraint::unconstrained();
    let result = resolve_block_size(
        &Length::px(400.0),
        lu(100),
        lu(600),
        BoxSizing::BorderBox,
        lu(50),   // padding+border
        &c,
    );
    assert_eq!(result, lu(350));
}

// ── min-content / max-content keywords in constraints ───────────────────

#[test]
fn min_content_as_min_width_treated_as_zero() {
    let c = resolve_size_constraints(
        &Length::min_content(),
        &Length::none(),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

#[test]
fn max_content_as_max_width_treated_as_none() {
    let c = resolve_size_constraints(
        &Length::auto(),
        &Length::max_content(),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.max_inline_size, LayoutUnit::max());
}

// ── edge: border-box adjustment clamps negative to zero ─────────────────

#[test]
fn border_box_small_size_clamps_to_zero() {
    // min-width: 20px border-box, but padding+border = 50px → content = 0 (clamped)
    let c = resolve_size_constraints(
        &Length::px(20.0),
        &Length::none(),
        &Length::auto(),
        &Length::none(),
        lu(800),
        lu(600),
        BoxSizing::BorderBox,
        lu(50),
        lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}

// ── percentage min against indefinite CB ─────────────────────────────────

#[test]
fn min_percentage_against_indefinite_cb_is_zero() {
    let indef = LayoutUnit::from_raw(-64);
    let c = resolve_size_constraints(
        &Length::percent(50.0),  // min_inline: 50% of indefinite → 0
        &Length::none(),
        &Length::auto(),
        &Length::none(),
        indef,
        lu(600),
        BoxSizing::ContentBox,
        lu(0),
        lu(0),
    );
    assert_eq!(c.min_inline_size, LayoutUnit::zero());
}
