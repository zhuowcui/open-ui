//! SP12 E3 — CSS Sizing Level 3 integration tests.
//!
//! Tests for sizing keywords, aspect-ratio, definite size detection,
//! automatic sizing, and preferred size resolution.

use openui_geometry::{LayoutUnit, Length, MinMaxSizes, INDEFINITE_SIZE};
use openui_layout::css_sizing::{
    SizingKeyword, apply_aspect_ratio, apply_aspect_ratio_with_auto,
    compute_automatic_size, compute_definite_size, resolve_preferred_size,
    resolve_sizing_keyword,
};
use openui_layout::ConstraintSpace;
use openui_style::AspectRatio;

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

// ── SizingKeyword enum construction ──────────────────────────────────

#[test]
fn sizing_keyword_enum_construction() {
    let auto = SizingKeyword::Auto;
    let min_c = SizingKeyword::MinContent;
    let max_c = SizingKeyword::MaxContent;
    let fit = SizingKeyword::FitContent(lu(100));
    let stretch = SizingKeyword::Stretch;

    // Verify they are distinct values via Debug.
    assert_ne!(format!("{:?}", auto), format!("{:?}", min_c));
    assert_ne!(format!("{:?}", max_c), format!("{:?}", fit));
    assert_ne!(format!("{:?}", stretch), format!("{:?}", auto));

    // FitContent carries a value.
    if let SizingKeyword::FitContent(v) = fit {
        assert_eq!(v, lu(100));
    } else {
        panic!("Expected FitContent");
    }
}

// ── MinContent keyword resolution ────────────────────────────────────

#[test]
fn min_content_keyword_resolution() {
    let intrinsic = MinMaxSizes::new(lu(60), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::MinContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(60));
}

// ── MaxContent keyword resolution ────────────────────────────────────

#[test]
fn max_content_keyword_resolution() {
    let intrinsic = MinMaxSizes::new(lu(60), lu(300));
    let result = resolve_sizing_keyword(SizingKeyword::MaxContent, &intrinsic, lu(800), lu(0));
    assert_eq!(result, lu(300));
}

// ── FitContent clamping ──────────────────────────────────────────────

#[test]
fn fit_content_clamping_below_min() {
    // fit-content(20) with min=50, max=200 → clamp up to 50
    let intrinsic = MinMaxSizes::new(lu(50), lu(200));
    let result = resolve_sizing_keyword(
        SizingKeyword::FitContent(lu(20)), &intrinsic, lu(500), lu(0),
    );
    assert_eq!(result, lu(50));
}

#[test]
fn fit_content_clamping_above_max() {
    // fit-content(500) with min=50, max=200 → clamp down to 200
    let intrinsic = MinMaxSizes::new(lu(50), lu(200));
    let result = resolve_sizing_keyword(
        SizingKeyword::FitContent(lu(500)), &intrinsic, lu(800), lu(0),
    );
    assert_eq!(result, lu(200));
}

#[test]
fn fit_content_clamping_between_min_and_max() {
    // fit-content(150) with min=50, max=200 → 150 (within range)
    let intrinsic = MinMaxSizes::new(lu(50), lu(200));
    let result = resolve_sizing_keyword(
        SizingKeyword::FitContent(lu(150)), &intrinsic, lu(800), lu(0),
    );
    assert_eq!(result, lu(150));
}

// ── Stretch resolution ───────────────────────────────────────────────

#[test]
fn stretch_resolution_fills_available() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(600), lu(0));
    assert_eq!(result, lu(600));
}

#[test]
fn stretch_with_margins_subtracted() {
    let intrinsic = MinMaxSizes::new(lu(0), lu(0));
    // available=600, margins=50 → 550
    let result = resolve_sizing_keyword(SizingKeyword::Stretch, &intrinsic, lu(600), lu(50));
    assert_eq!(result, lu(550));
}

// ── Aspect ratio: width → height ─────────────────────────────────────

#[test]
fn aspect_ratio_width_to_height() {
    // 16:9 ratio, width=320, height=indefinite → height = 320 * 9/16 = 180
    let (w, h) = apply_aspect_ratio(lu(320), INDEFINITE_SIZE, (16.0, 9.0));
    assert_eq!(w, lu(320));
    assert_eq!(h, lu(180));
}

// ── Aspect ratio: height → width ─────────────────────────────────────

#[test]
fn aspect_ratio_height_to_width() {
    // 16:9 ratio, height=180, width=indefinite → width = 180 * 16/9 = 320
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(180), (16.0, 9.0));
    assert_eq!(w, lu(320));
    assert_eq!(h, lu(180));
}

// ── Aspect ratio with auto flag ──────────────────────────────────────

#[test]
fn aspect_ratio_with_auto_flag_prefers_intrinsic() {
    let ar = AspectRatio { ratio: (16.0, 9.0), auto_flag: true };
    let intrinsic = Some((4.0, 3.0));
    // auto_flag + intrinsic exists → use 4:3 instead of 16:9
    let (w, h) = apply_aspect_ratio_with_auto(INDEFINITE_SIZE, lu(300), &ar, intrinsic);
    // width = 300 * 4/3 = 400
    assert_eq!(w, lu(400));
    assert_eq!(h, lu(300));
}

// ── Aspect ratio both definite (ignored) ─────────────────────────────

#[test]
fn aspect_ratio_both_definite_ignored() {
    let (w, h) = apply_aspect_ratio(lu(200), lu(100), (16.0, 9.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, lu(100));
}

// ── Definite size detection: length ──────────────────────────────────

#[test]
fn definite_size_detection_length() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::px(200.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(200)));
}

// ── Definite size detection: percentage with definite CB ─────────────

#[test]
fn definite_size_detection_percentage_with_definite_cb() {
    let space = ConstraintSpace::for_root(lu(800), lu(600));
    let result = compute_definite_size(&Length::percent(50.0), lu(800), &space, true);
    assert_eq!(result, Some(lu(400)));
}

// ── Indefinite: percentage with indefinite CB ────────────────────────

#[test]
fn indefinite_percentage_with_indefinite_cb() {
    let space = ConstraintSpace::for_root(lu(800), INDEFINITE_SIZE);
    let result = compute_definite_size(&Length::percent(50.0), INDEFINITE_SIZE, &space, false);
    assert_eq!(result, None);
}

// ── Automatic inline size (stretch) ──────────────────────────────────

#[test]
fn automatic_inline_size_stretch() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(300));
    // Inline axis → stretch: available - margins
    let result = compute_automatic_size(true, &intrinsic, lu(600), lu(40));
    assert_eq!(result, lu(560));
}

// ── Automatic block size (fit-content) ───────────────────────────────

#[test]
fn automatic_block_size_fit_content() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(300));
    // Block axis → fit-content: clamp max-content to available
    let result = compute_automatic_size(false, &intrinsic, lu(600), lu(0));
    // max-content=300, available=600, min-content=100 → min(300, 600) = 300
    assert_eq!(result, lu(300));
}

#[test]
fn automatic_block_size_fit_content_small_available() {
    let intrinsic = MinMaxSizes::new(lu(100), lu(500));
    // Block axis: available 200, max-content 500 → min(500, 200) → 200, max(200, 100) → 200
    let result = compute_automatic_size(false, &intrinsic, lu(200), lu(0));
    assert_eq!(result, lu(200));
}

// ── Preferred size with keyword + min/max ────────────────────────────

#[test]
fn preferred_size_with_keyword_and_min_max() {
    let intrinsic = MinMaxSizes::new(lu(80), lu(250));
    // preferred=max-content (250), min-width=100px, max-width=200px
    // → clamp(100, 250, 200) → 200
    let result = resolve_preferred_size(
        &Length::max_content(),
        &Length::px(100.0),
        &Length::px(200.0),
        lu(800),
        &intrinsic,
        lu(800),
        lu(0),
        INDEFINITE_SIZE,
        None,
        true,
    );
    assert_eq!(result, lu(200));
}

// ── Aspect ratio + min-width constraint ──────────────────────────────

#[test]
fn aspect_ratio_plus_min_width_constraint() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(400));
    let ar = AspectRatio { ratio: (2.0, 1.0), auto_flag: false };
    // preferred=auto, other_axis (height)=30 → width = 30 * 2/1 = 60
    // min-width=100px → clamped up to 100
    let result = resolve_preferred_size(
        &Length::auto(),
        &Length::px(100.0),
        &Length::none(),
        lu(800),
        &intrinsic,
        lu(800),
        lu(0),
        lu(30),
        Some(&ar),
        true,
    );
    assert_eq!(result, lu(100));
}

// ── Zero aspect ratio handling ───────────────────────────────────────

#[test]
fn zero_aspect_ratio_handling() {
    // Zero ratio.0 → returns both unchanged.
    let (w, h) = apply_aspect_ratio(lu(200), INDEFINITE_SIZE, (0.0, 9.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, INDEFINITE_SIZE);

    // Zero ratio.1 → returns both unchanged.
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, lu(100), (16.0, 0.0));
    assert_eq!(w, INDEFINITE_SIZE);
    assert_eq!(h, lu(100));
}

// ── Additional tests for coverage ────────────────────────────────────

#[test]
fn definite_size_auto_with_fixed_space() {
    let mut space = ConstraintSpace::for_root(lu(500), lu(400));
    space.is_fixed_inline_size = true;
    let result = compute_definite_size(&Length::auto(), lu(500), &space, true);
    assert_eq!(result, Some(lu(500)));
}

#[test]
fn definite_size_auto_without_fixed_space() {
    let space = ConstraintSpace::for_root(lu(500), lu(400));
    let result = compute_definite_size(&Length::auto(), lu(500), &space, true);
    assert_eq!(result, None);
}

#[test]
fn definite_size_min_content_is_indefinite() {
    let space = ConstraintSpace::for_root(lu(500), lu(400));
    let result = compute_definite_size(&Length::min_content(), lu(500), &space, true);
    assert_eq!(result, None);
}

#[test]
fn preferred_size_fixed_value() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    // preferred=150px, min=auto(0), max=none(max) → 150
    let result = resolve_preferred_size(
        &Length::px(150.0),
        &Length::auto(),
        &Length::none(),
        lu(800),
        &intrinsic,
        lu(800),
        lu(0),
        INDEFINITE_SIZE,
        None,
        true,
    );
    assert_eq!(result, lu(150));
}

#[test]
fn preferred_size_percentage() {
    let intrinsic = MinMaxSizes::new(lu(50), lu(300));
    // preferred=50%, containing-block=400 → 200
    let result = resolve_preferred_size(
        &Length::percent(50.0),
        &Length::auto(),
        &Length::none(),
        lu(400),
        &intrinsic,
        lu(400),
        lu(0),
        INDEFINITE_SIZE,
        None,
        true,
    );
    assert_eq!(result, lu(200));
}

#[test]
fn stretch_definite_in_flex_context() {
    let mut space = ConstraintSpace::for_root(lu(600), lu(400));
    space.stretch_inline_size = true;
    let result = compute_definite_size(&Length::stretch(), lu(600), &space, true);
    assert_eq!(result, Some(lu(600)));
}

#[test]
fn stretch_indefinite_without_flex() {
    let space = ConstraintSpace::for_root(lu(600), lu(400));
    let result = compute_definite_size(&Length::stretch(), lu(600), &space, true);
    assert_eq!(result, None);
}

#[test]
fn aspect_ratio_1_to_1() {
    // Square aspect ratio.
    let (w, h) = apply_aspect_ratio(lu(200), INDEFINITE_SIZE, (1.0, 1.0));
    assert_eq!(w, lu(200));
    assert_eq!(h, lu(200));
}

#[test]
fn aspect_ratio_both_indefinite_no_resolution() {
    let (w, h) = apply_aspect_ratio(INDEFINITE_SIZE, INDEFINITE_SIZE, (16.0, 9.0));
    assert_eq!(w, INDEFINITE_SIZE);
    assert_eq!(h, INDEFINITE_SIZE);
}
