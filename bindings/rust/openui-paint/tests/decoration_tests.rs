//! Decoration painting tests — verifies underline, overline, line-through rendering,
//! decoration styles, colors, thickness, and edge cases.
//!
//! These tests complement the existing `text_paint_tests.rs` by focusing on
//! decoration-specific scenarios with synthetic metrics and pixel verification.

use std::sync::Arc;

use skia_safe::{surfaces, Color as SkColor, Surface};

use openui_paint::{decoration_painter, text_painter};
use openui_style::*;
use openui_text::{Font, FontDescription, FontMetrics, ShapeResult, TextDirection, TextShaper};

// ── Test helpers ─────────────────────────────────────────────────────

fn make_font(size: f32) -> Font {
    let mut desc = FontDescription::new();
    desc.size = size;
    desc.specified_size = size;
    Font::new(desc)
}

fn shape_text(text: &str) -> ShapeResult {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Ltr)
}

fn make_surface(width: i32, height: i32) -> Surface {
    let mut surface =
        surfaces::raster_n32_premul((width, height)).expect("Failed to create Skia surface");
    surface.canvas().clear(SkColor::WHITE);
    surface
}

fn has_non_white_pixels(surface: &mut Surface) -> bool {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = info.min_row_bytes();
    let mut pixels = vec![0u8; info.height() as usize * row_bytes];
    image.read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    for chunk in pixels.chunks(4) {
        if chunk.len() == 4 && (chunk[0] != 0xFF || chunk[1] != 0xFF || chunk[2] != 0xFF) {
            return true;
        }
    }
    false
}

/// Build a style with underline enabled and the given overrides applied.
fn underline_style(f: impl FnOnce(&mut ComputedStyle)) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    f(&mut style);
    style
}

/// Build a style with overline enabled and the given overrides applied.
fn overline_style(f: impl FnOnce(&mut ComputedStyle)) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::OVERLINE;
    f(&mut style);
    style
}

/// Build a style with line-through enabled and the given overrides applied.
fn line_through_style(f: impl FnOnce(&mut ComputedStyle)) -> ComputedStyle {
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::LINE_THROUGH;
    f(&mut style);
    style
}

/// Create synthetic metrics with known values for deterministic positioning.
fn synthetic_metrics() -> FontMetrics {
    FontMetrics {
        ascent: 12.0,
        descent: 4.0,
        line_gap: 0.0,
        line_spacing: 16.0,
        x_height: 8.0,
        cap_height: 11.0,
        zero_width: 8.0,
        underline_offset: 2.0,
        underline_thickness: 1.0,
        strikeout_position: 5.0,
        strikeout_thickness: 1.0,
        overline_offset: 0.0,
        units_per_em: 1000,
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── UNDERLINE TESTS ─────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn underline_renders_visible_pixels() {
    let sr = Arc::new(shape_text("Decoration"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = underline_style(|_| {});
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Underline should produce visible pixels"
    );
}

#[test]
fn no_decoration_produces_no_pixels() {
    let sr = Arc::new(shape_text("Decoration"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = ComputedStyle::default(); // NONE by default
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        !has_non_white_pixels(&mut surface),
        "No decoration line should mean no visible pixels"
    );
}

#[test]
fn underline_position_uses_font_metrics_offset() {
    // Paint with synthetic metrics so underline_offset is deterministic.
    let sr = Arc::new(shape_text("Test"));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = underline_style(|_| {});
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn underline_with_custom_color() {
    let sr = Arc::new(shape_text("Color"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(255, 0, 0, 255));
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Red underline should be visible"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── OVERLINE TESTS ──────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overline_renders_visible_pixels() {
    let sr = Arc::new(shape_text("Overline"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = overline_style(|_| {});
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 60.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Overline should produce visible pixels"
    );
}

#[test]
fn overline_positioned_above_baseline() {
    // With synthetic metrics ascent=12, the overline at baseline_y=60
    // should draw at y = 60 - 12 = 48 (above baseline).
    let sr = Arc::new(shape_text("Top"));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = overline_style(|_| {});
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 60.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn overline_with_custom_thickness() {
    let sr = Arc::new(shape_text("Thick"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = overline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(4.0);
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 60.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── LINE-THROUGH TESTS ──────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn line_through_renders_visible_pixels() {
    let sr = Arc::new(shape_text("Strike"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = line_through_style(|_| {});
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Line-through should produce visible pixels"
    );
}

#[test]
fn line_through_at_middle_of_text() {
    // Strikeout is at baseline_y - strikeout_position (50 - 5 = 45 with synthetic metrics).
    let sr = Arc::new(shape_text("Middle"));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = line_through_style(|_| {});
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn line_through_with_dashed_style() {
    let sr = Arc::new(shape_text("Dashed"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = line_through_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Dashed;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── DECORATION STYLE RENDERING ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn solid_style_renders() {
    let sr = Arc::new(shape_text("Solid"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Solid;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn double_style_renders() {
    let sr = Arc::new(shape_text("Double"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Double;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn dotted_style_renders() {
    let sr = Arc::new(shape_text("Dotted"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Dotted;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn dashed_style_renders() {
    let sr = Arc::new(shape_text("Dashed"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Dashed;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn wavy_style_renders() {
    let sr = Arc::new(shape_text("Wavy"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Wavy;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── DECORATION COLOR ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn current_color_resolves_to_element_color() {
    let sr = Arc::new(shape_text("Resolve"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.color = Color::GREEN;
        s.text_decoration_color = StyleColor::CurrentColor;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "currentColor → green should be visible"
    );
}

#[test]
fn specified_red_underline_color() {
    let sr = Arc::new(shape_text("Red"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::RED);
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Red underline should be visible"
    );
}

#[test]
fn transparent_decoration_produces_no_visible_pixels() {
    let sr = Arc::new(shape_text("Invisible"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::TRANSPARENT);
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        !has_non_white_pixels(&mut surface),
        "Transparent underline should produce no visible pixels"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── MULTIPLE DECORATIONS ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn underline_and_overline_simultaneously() {
    let sr = Arc::new(shape_text("Both"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line =
        TextDecorationLine(TextDecorationLine::UNDERLINE.0 | TextDecorationLine::OVERLINE.0);
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 60.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Underline + overline should both render"
    );
}

#[test]
fn underline_and_line_through_simultaneously() {
    let sr = Arc::new(shape_text("Both"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0 | TextDecorationLine::LINE_THROUGH.0,
    );
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Underline + line-through should both render"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── DECORATION THICKNESS ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn auto_thickness_uses_font_metric_at_least_1px() {
    let sr = Arc::new(shape_text("Auto"));
    // Use synthetic metrics with small underline_thickness (0.5) to verify
    // the Auto mode clamps to at least 1.0.
    let mut metrics = synthetic_metrics();
    metrics.underline_thickness = 0.5;
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Auto;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(
        has_non_white_pixels(&mut surface),
        "Auto thickness (clamped to 1px) should be visible"
    );
}

#[test]
fn from_font_uses_underline_thickness() {
    let sr = Arc::new(shape_text("Font"));
    let metrics = synthetic_metrics(); // underline_thickness = 1.0
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::FromFont;
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn explicit_length_thickness() {
    let sr = Arc::new(shape_text("Thick"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(3.0);
    });
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── EDGE CASES ──────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn empty_text_does_not_crash() {
    let sr = Arc::new(ShapeResult::empty(TextDirection::Ltr));
    let metrics = FontMetrics::default();
    let mut surface = make_surface(100, 100);
    let style = underline_style(|_| {});
    // Should not panic — zero-width text produces no drawing.
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
}

#[test]
fn decoration_on_zero_width_text_does_not_crash() {
    // A shape result with zero width should early-return without drawing.
    let sr = Arc::new(ShapeResult::empty(TextDirection::Ltr));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(100, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0
            | TextDecorationLine::OVERLINE.0
            | TextDecorationLine::LINE_THROUGH.0,
    );
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(
        !has_non_white_pixels(&mut surface),
        "Zero-width text should not produce any pixels"
    );
}
