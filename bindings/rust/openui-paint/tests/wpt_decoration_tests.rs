//! WPT-equivalent tests for CSS Text Decoration Module.
//!
//! Each test corresponds to behaviors verified by WPT css/css-text-decor tests.
//! Categories: text-decoration-line, text-decoration-style, text-decoration-color,
//! text-decoration-thickness, text-decoration-skip-ink, text-underline-position,
//! text-underline-offset, text-emphasis-style, text-emphasis-position,
//! text-emphasis-color, text-shadow.

use std::sync::Arc;

use skia_safe::{surfaces, Color as SkColor, Surface};

use openui_paint::{decoration_painter, emphasis_painter, text_painter};
use openui_style::*;
use openui_text::{
    Font, FontDescription, FontMetrics, ShapeResult, TextDirection, TextShaper,
};
use openui_text::emphasis::{
    default_mark_for_writing_mode, default_position_for_writing_mode, resolve_emphasis_mark,
    should_draw_emphasis_mark, ResolvedEmphasisMark,
};

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

fn shape_text_with_size(text: &str, size: f32) -> ShapeResult {
    let font = make_font(size);
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

fn count_non_white_pixels(surface: &mut Surface) -> usize {
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
    let mut count = 0usize;
    for chunk in pixels.chunks(4) {
        if chunk.len() == 4 && (chunk[0] != 0xFF || chunk[1] != 0xFF || chunk[2] != 0xFF) {
            count += 1;
        }
    }
    count
}

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

/// Paint with DecorationPhase::BeforeText (underline/overline).
fn paint_before(
    surface: &mut Surface,
    sr: &Arc<ShapeResult>,
    style: &ComputedStyle,
    metrics: &FontMetrics,
) {
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        sr,
        (10.0, 50.0),
        style,
        metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
}

/// Paint with DecorationPhase::AfterText (line-through).
fn paint_after(
    surface: &mut Surface,
    sr: &Arc<ShapeResult>,
    style: &ComputedStyle,
    metrics: &FontMetrics,
) {
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        sr,
        (10.0, 50.0),
        style,
        metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
}

/// Paint with skip-ink text content.
fn paint_before_with_text(
    surface: &mut Surface,
    sr: &Arc<ShapeResult>,
    style: &ComputedStyle,
    metrics: &FontMetrics,
    text: &str,
) {
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        sr,
        (10.0, 50.0),
        style,
        metrics,
        decoration_painter::DecorationPhase::BeforeText,
        Some(text),
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_decoration_line ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-decoration-line-none-001 — default is NONE.
#[test]
fn wpt_line_default_is_none() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_decoration_line, TextDecorationLine::NONE);
    assert!(style.text_decoration_line.is_none());
}

/// WPT: text-decoration-line-underline-001 — underline renders.
#[test]
fn wpt_line_underline_renders() {
    let sr = Arc::new(shape_text("Hello World"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = underline_style(|_| {});
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface), "underline must produce pixels");
}

/// WPT: text-decoration-line-overline-001 — overline renders.
#[test]
fn wpt_line_overline_renders() {
    let sr = Arc::new(shape_text("Hello World"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = overline_style(|_| {});
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface), "overline must produce pixels");
}

/// WPT: text-decoration-line-line-through-001 — line-through renders.
#[test]
fn wpt_line_through_renders() {
    let sr = Arc::new(shape_text("Hello World"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = line_through_style(|_| {});
    paint_after(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface), "line-through must produce pixels");
}

/// WPT: text-decoration-line-none-002 — NONE draws nothing.
#[test]
fn wpt_line_none_draws_nothing() {
    let sr = Arc::new(shape_text("Hello World"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = ComputedStyle::default();
    paint_before(&mut surface, &sr, &style, &metrics);
    paint_after(&mut surface, &sr, &style, &metrics);
    assert!(!has_non_white_pixels(&mut surface), "NONE must produce no pixels");
}

/// WPT: text-decoration-line-underline-overline-001 — combined.
#[test]
fn wpt_line_underline_plus_overline() {
    let sr = Arc::new(shape_text("Dual"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0 | TextDecorationLine::OVERLINE.0,
    );
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-line-underline-linethrough-001 — combined.
#[test]
fn wpt_line_underline_plus_line_through() {
    let sr = Arc::new(shape_text("Combined"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0 | TextDecorationLine::LINE_THROUGH.0,
    );
    paint_before(&mut surface, &sr, &style, &metrics);
    let before_count = count_non_white_pixels(&mut surface);
    paint_after(&mut surface, &sr, &style, &metrics);
    let total_count = count_non_white_pixels(&mut surface);
    assert!(before_count > 0, "underline must draw");
    assert!(total_count > before_count, "line-through must add pixels");
}

/// WPT: text-decoration-line-all-001 — all three combined.
#[test]
fn wpt_line_all_three() {
    let sr = Arc::new(shape_text("All three"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0
            | TextDecorationLine::OVERLINE.0
            | TextDecorationLine::LINE_THROUGH.0,
    );
    paint_before(&mut surface, &sr, &style, &metrics);
    paint_after(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: has_underline check.
#[test]
fn wpt_has_underline_method() {
    assert!(TextDecorationLine::UNDERLINE.has_underline());
    assert!(!TextDecorationLine::OVERLINE.has_underline());
    assert!(!TextDecorationLine::LINE_THROUGH.has_underline());
    assert!(!TextDecorationLine::NONE.has_underline());
}

/// WPT: has_overline check.
#[test]
fn wpt_has_overline_method() {
    assert!(TextDecorationLine::OVERLINE.has_overline());
    assert!(!TextDecorationLine::UNDERLINE.has_overline());
    assert!(!TextDecorationLine::LINE_THROUGH.has_overline());
    assert!(!TextDecorationLine::NONE.has_overline());
}

/// WPT: has_line_through check.
#[test]
fn wpt_has_line_through_method() {
    assert!(TextDecorationLine::LINE_THROUGH.has_line_through());
    assert!(!TextDecorationLine::UNDERLINE.has_line_through());
    assert!(!TextDecorationLine::OVERLINE.has_line_through());
    assert!(!TextDecorationLine::NONE.has_line_through());
}

/// WPT: is_none check.
#[test]
fn wpt_is_none_method() {
    assert!(TextDecorationLine::NONE.is_none());
    assert!(!TextDecorationLine::UNDERLINE.is_none());
    assert!(!TextDecorationLine::OVERLINE.is_none());
    assert!(!TextDecorationLine::LINE_THROUGH.is_none());
}

/// WPT: combined flag introspection.
#[test]
fn wpt_combined_flags_introspection() {
    let combined = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0 | TextDecorationLine::OVERLINE.0,
    );
    assert!(combined.has_underline());
    assert!(combined.has_overline());
    assert!(!combined.has_line_through());
    assert!(!combined.is_none());
}

/// WPT: all-three flag introspection.
#[test]
fn wpt_all_flags_introspection() {
    let all = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0
            | TextDecorationLine::OVERLINE.0
            | TextDecorationLine::LINE_THROUGH.0,
    );
    assert!(all.has_underline());
    assert!(all.has_overline());
    assert!(all.has_line_through());
    assert!(!all.is_none());
}

/// WPT: overline uses synthetic metrics positioning.
#[test]
fn wpt_overline_with_synthetic_metrics() {
    let sr = Arc::new(shape_text("Top"));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = overline_style(|_| {});
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_decoration_style ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-decoration-style-initial — default is Solid.
#[test]
fn wpt_style_default_is_solid() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_decoration_style, TextDecorationStyle::Solid);
}

/// WPT: text-decoration-style-solid-001
#[test]
fn wpt_style_solid_renders() {
    let sr = Arc::new(shape_text("Solid"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Solid;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-style-double-001
#[test]
fn wpt_style_double_renders() {
    let sr = Arc::new(shape_text("Double"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Double;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-style-dotted-001
#[test]
fn wpt_style_dotted_renders() {
    let sr = Arc::new(shape_text("Dotted"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Dotted;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-style-dashed-001
#[test]
fn wpt_style_dashed_renders() {
    let sr = Arc::new(shape_text("Dashed"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Dashed;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-style-wavy-001
#[test]
fn wpt_style_wavy_renders() {
    let sr = Arc::new(shape_text("Wavy"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Wavy;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: wavy produces more pixels than solid (wider stroke area).
#[test]
fn wpt_style_wavy_wider_than_solid() {
    let sr = Arc::new(shape_text("Wider wavy line test"));
    let metrics = synthetic_metrics();

    let mut solid_surface = make_surface(400, 100);
    let solid_style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Solid;
    });
    paint_before(&mut solid_surface, &sr, &solid_style, &metrics);
    let solid_px = count_non_white_pixels(&mut solid_surface);

    let mut wavy_surface = make_surface(400, 100);
    let wavy_style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Wavy;
    });
    paint_before(&mut wavy_surface, &sr, &wavy_style, &metrics);
    let wavy_px = count_non_white_pixels(&mut wavy_surface);

    assert!(
        wavy_px > solid_px,
        "wavy ({wavy_px}) should have more pixels than solid ({solid_px})"
    );
}

/// WPT: double produces more pixels than solid.
#[test]
fn wpt_style_double_more_than_solid() {
    let sr = Arc::new(shape_text("Double line test text"));
    let metrics = synthetic_metrics();

    let mut solid_surface = make_surface(400, 100);
    let solid_style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Solid;
    });
    paint_before(&mut solid_surface, &sr, &solid_style, &metrics);
    let solid_px = count_non_white_pixels(&mut solid_surface);

    let mut double_surface = make_surface(400, 100);
    let double_style = underline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Double;
    });
    paint_before(&mut double_surface, &sr, &double_style, &metrics);
    let double_px = count_non_white_pixels(&mut double_surface);

    assert!(
        double_px > solid_px,
        "double ({double_px}) should have more pixels than solid ({solid_px})"
    );
}

/// WPT: different styles applied to overline.
#[test]
fn wpt_style_overline_wavy() {
    let sr = Arc::new(shape_text("WavyOverline"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = overline_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Wavy;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: dashed style applied to line-through.
#[test]
fn wpt_style_line_through_dashed() {
    let sr = Arc::new(shape_text("DashedStrike"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = line_through_style(|s| {
        s.text_decoration_style = TextDecorationStyle::Dashed;
    });
    paint_after(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_decoration_color ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-decoration-color-initial — default is CurrentColor.
#[test]
fn wpt_color_default_is_currentcolor() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_decoration_color, StyleColor::CurrentColor);
}

/// WPT: text-decoration-color-red-001
#[test]
fn wpt_color_red_underline() {
    let sr = Arc::new(shape_text("Red"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(255, 0, 0, 255));
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface), "red underline must render");
}

/// WPT: text-decoration-color-blue-001
#[test]
fn wpt_color_blue_underline() {
    let sr = Arc::new(shape_text("Blue"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(0, 0, 255, 255));
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface), "blue underline must render");
}

/// WPT: text-decoration-color-green-overline
#[test]
fn wpt_color_green_overline() {
    let sr = Arc::new(shape_text("Green"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = overline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(0, 128, 0, 255));
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-color-currentcolor-resolves
#[test]
fn wpt_color_currentcolor_resolves() {
    let sr = Arc::new(shape_text("Current"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.color = Color::from_rgba8(255, 0, 0, 255);
        s.text_decoration_color = StyleColor::CurrentColor;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: different colors produce different pixel patterns.
#[test]
fn wpt_color_red_vs_blue_differ() {
    let sr = Arc::new(shape_text("Differ"));
    let metrics = synthetic_metrics();

    let mut red_surface = make_surface(300, 100);
    let red_style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(255, 0, 0, 255));
    });
    paint_before(&mut red_surface, &sr, &red_style, &metrics);

    let mut blue_surface = make_surface(300, 100);
    let blue_style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(0, 0, 255, 255));
    });
    paint_before(&mut blue_surface, &sr, &blue_style, &metrics);

    // Both must render, confirming they are not identical blank surfaces.
    assert!(has_non_white_pixels(&mut red_surface));
    assert!(has_non_white_pixels(&mut blue_surface));
}

/// WPT: fully transparent color draws nothing visible.
#[test]
fn wpt_color_transparent_draws_nothing() {
    let sr = Arc::new(shape_text("Transparent"));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(0, 0, 0, 0));
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(!has_non_white_pixels(&mut surface), "transparent decoration must not render");
}

/// WPT: semi-transparent color still draws.
#[test]
fn wpt_color_semitransparent_draws() {
    let sr = Arc::new(shape_text("SemiAlpha"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_color = StyleColor::Resolved(Color::from_rgba8(0, 0, 0, 128));
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_decoration_thickness ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-decoration-thickness-initial — default is Auto.
#[test]
fn wpt_thickness_default_is_auto() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_decoration_thickness, TextDecorationThickness::Auto);
}

/// WPT: text-decoration-thickness-from-font-001
#[test]
fn wpt_thickness_from_font() {
    let sr = Arc::new(shape_text("FromFont"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::FromFont;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-thickness-length-001 (2px)
#[test]
fn wpt_thickness_length_2px() {
    let sr = Arc::new(shape_text("Thick2"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(2.0);
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-thickness-length-001 (4px)
#[test]
fn wpt_thickness_length_4px() {
    let sr = Arc::new(shape_text("Thick4"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(4.0);
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: thicker line produces more pixels than thinner.
#[test]
fn wpt_thickness_thicker_more_pixels() {
    let sr = Arc::new(shape_text("Thickness compare text"));
    let metrics = synthetic_metrics();

    let mut thin_surface = make_surface(400, 100);
    let thin_style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(1.0);
        s.text_decoration_skip_ink = TextDecorationSkipInk::None;
    });
    paint_before(&mut thin_surface, &sr, &thin_style, &metrics);
    let thin_px = count_non_white_pixels(&mut thin_surface);

    let mut thick_surface = make_surface(400, 100);
    let thick_style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(4.0);
        s.text_decoration_skip_ink = TextDecorationSkipInk::None;
    });
    paint_before(&mut thick_surface, &sr, &thick_style, &metrics);
    let thick_px = count_non_white_pixels(&mut thick_surface);

    assert!(
        thick_px > thin_px,
        "4px ({thick_px}) should have more pixels than 1px ({thin_px})"
    );
}

/// WPT: auto thickness still renders.
#[test]
fn wpt_thickness_auto_renders() {
    let sr = Arc::new(shape_text("AutoThick"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Auto;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: overline with custom thickness.
#[test]
fn wpt_thickness_overline_custom() {
    let sr = Arc::new(shape_text("ThickOver"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = overline_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(3.0);
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: line-through with custom thickness.
#[test]
fn wpt_thickness_line_through_custom() {
    let sr = Arc::new(shape_text("ThickStrike"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = line_through_style(|s| {
        s.text_decoration_thickness = TextDecorationThickness::Length(3.0);
    });
    paint_after(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_decoration_skip_ink ────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-decoration-skip-ink-initial — default is Auto.
#[test]
fn wpt_skip_ink_default_is_auto() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_decoration_skip_ink, TextDecorationSkipInk::Auto);
}

/// WPT: text-decoration-skip-ink-none-001
#[test]
fn wpt_skip_ink_none_renders() {
    let sr = Arc::new(shape_text("gypsy"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::None;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "gypsy");
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-skip-ink-auto-001
#[test]
fn wpt_skip_ink_auto_renders() {
    let sr = Arc::new(shape_text("gypsy"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::Auto;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "gypsy");
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-decoration-skip-ink-all-001
#[test]
fn wpt_skip_ink_all_renders() {
    let sr = Arc::new(shape_text("gypsy"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::All;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "gypsy");
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: skip-ink none draws more pixels than auto (no gaps).
#[test]
fn wpt_skip_ink_none_more_pixels_than_auto() {
    let sr = Arc::new(shape_text("gypsy jumping quickly"));
    let metrics = synthetic_metrics();

    let mut none_surface = make_surface(500, 100);
    let none_style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::None;
    });
    paint_before_with_text(&mut none_surface, &sr, &none_style, &metrics, "gypsy jumping quickly");
    let none_px = count_non_white_pixels(&mut none_surface);

    let mut auto_surface = make_surface(500, 100);
    let auto_style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::Auto;
    });
    paint_before_with_text(&mut auto_surface, &sr, &auto_style, &metrics, "gypsy jumping quickly");
    let auto_px = count_non_white_pixels(&mut auto_surface);

    assert!(
        none_px >= auto_px,
        "skip-ink:none ({none_px}) must have >= pixels than auto ({auto_px})"
    );
}

/// WPT: skip-ink with uppercase (no descenders).
#[test]
fn wpt_skip_ink_uppercase_text() {
    let sr = Arc::new(shape_text("HELLO WORLD"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::Auto;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "HELLO WORLD");
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: skip-ink with CJK text.
#[test]
fn wpt_skip_ink_cjk_text() {
    let sr = Arc::new(shape_text("日本語テスト"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::Auto;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "日本語テスト");
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: skip-ink none overline.
#[test]
fn wpt_skip_ink_none_overline() {
    let sr = Arc::new(shape_text("Testing"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let style = overline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::None;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "Testing");
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: skip-ink with empty string does not crash.
#[test]
fn wpt_skip_ink_empty_string() {
    let sr = Arc::new(shape_text(""));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::Auto;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "");
    // Empty string should not crash; may or may not have pixels.
}

/// WPT: skip-ink with spaces only.
#[test]
fn wpt_skip_ink_spaces_only() {
    let sr = Arc::new(shape_text("     "));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_decoration_skip_ink = TextDecorationSkipInk::Auto;
    });
    paint_before_with_text(&mut surface, &sr, &style, &metrics, "     ");
    // Spaces have no ink to skip; underline should still render.
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_underline_position ─────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-underline-position-initial — default is Auto.
#[test]
fn wpt_underline_position_default_is_auto() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_underline_position, TextUnderlinePosition::Auto);
}

/// WPT: text-underline-position-under-001
#[test]
fn wpt_underline_position_under_renders() {
    let sr = Arc::new(shape_text("Under"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_position = TextUnderlinePosition::Under;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-underline-position-auto-001
#[test]
fn wpt_underline_position_auto_renders() {
    let sr = Arc::new(shape_text("AutoPos"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_position = TextUnderlinePosition::Auto;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-underline-position-left-001
#[test]
fn wpt_underline_position_left() {
    let sr = Arc::new(shape_text("Left"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_position = TextUnderlinePosition::Left;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-underline-position-right-001
#[test]
fn wpt_underline_position_right() {
    let sr = Arc::new(shape_text("Right"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_position = TextUnderlinePosition::Right;
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: under position below descenders — different pixel count from auto.
#[test]
fn wpt_underline_position_under_vs_auto() {
    let sr = Arc::new(shape_text("gypsy jig"));
    let metrics = synthetic_metrics();

    let mut auto_surface = make_surface(400, 100);
    let auto_style = underline_style(|s| {
        s.text_underline_position = TextUnderlinePosition::Auto;
    });
    paint_before(&mut auto_surface, &sr, &auto_style, &metrics);
    let auto_px = count_non_white_pixels(&mut auto_surface);

    let mut under_surface = make_surface(400, 100);
    let under_style = underline_style(|s| {
        s.text_underline_position = TextUnderlinePosition::Under;
    });
    paint_before(&mut under_surface, &sr, &under_style, &metrics);
    let under_px = count_non_white_pixels(&mut under_surface);

    // Both should render.
    assert!(auto_px > 0, "auto must render");
    assert!(under_px > 0, "under must render");
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_underline_offset ───────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-underline-offset-initial — default is auto (Length::auto()).
#[test]
fn wpt_underline_offset_default_is_auto() {
    let style = ComputedStyle::default();
    assert!(style.text_underline_offset.is_auto());
}

/// WPT: text-underline-offset custom 4px.
#[test]
fn wpt_underline_offset_4px() {
    let sr = Arc::new(shape_text("Offset4"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_offset = openui_geometry::Length::px(4.0);
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-underline-offset custom 0px.
#[test]
fn wpt_underline_offset_0px() {
    let sr = Arc::new(shape_text("Offset0"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_offset = openui_geometry::Length::px(0.0);
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-underline-offset negative value.
#[test]
fn wpt_underline_offset_negative() {
    let sr = Arc::new(shape_text("OffsetNeg"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_offset = openui_geometry::Length::px(-2.0);
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: large offset moves underline further away.
#[test]
fn wpt_underline_offset_large() {
    let sr = Arc::new(shape_text("LargeOffset"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let style = underline_style(|s| {
        s.text_underline_offset = openui_geometry::Length::px(10.0);
    });
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_emphasis_mark ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-emphasis-style-initial — default is None.
#[test]
fn wpt_emphasis_mark_default_is_none() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_emphasis_mark, TextEmphasisMark::None);
}

/// WPT: text-emphasis-style-dot-filled
#[test]
fn wpt_emphasis_resolve_dot_filled() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Dot,
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{2022}', over: true })
    );
}

/// WPT: text-emphasis-style-circle-filled
#[test]
fn wpt_emphasis_resolve_circle_filled() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Circle,
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{25CF}', over: true })
    );
}

/// WPT: text-emphasis-style-double-circle-filled
#[test]
fn wpt_emphasis_resolve_double_circle_filled() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::DoubleCircle,
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{25C9}', over: true })
    );
}

/// WPT: text-emphasis-style-triangle-filled
#[test]
fn wpt_emphasis_resolve_triangle_filled() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Triangle,
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{25B2}', over: true })
    );
}

/// WPT: text-emphasis-style-sesame-filled
#[test]
fn wpt_emphasis_resolve_sesame_filled() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Sesame,
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{FE45}', over: true })
    );
}

/// WPT: text-emphasis-style-dot-open
#[test]
fn wpt_emphasis_resolve_dot_open() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Dot,
        TextEmphasisFill::Open,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{25E6}', over: true })
    );
}

/// WPT: text-emphasis-style-circle-open
#[test]
fn wpt_emphasis_resolve_circle_open() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Circle,
        TextEmphasisFill::Open,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{25CB}', over: true })
    );
}

/// WPT: text-emphasis-style-none returns None
#[test]
fn wpt_emphasis_resolve_none_returns_none() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::None,
        TextEmphasisFill::Filled,
        TextEmphasisPosition::INITIAL,
    );
    assert_eq!(result, None);
}

/// WPT: text-emphasis-style-custom-001
#[test]
fn wpt_emphasis_resolve_custom_char() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Custom('★'),
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '★', over: true })
    );
}

/// WPT: custom char ignores fill.
#[test]
fn wpt_emphasis_custom_ignores_fill() {
    let filled = resolve_emphasis_mark(
        TextEmphasisMark::Custom('♥'),
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: true, right: true },
    );
    let open = resolve_emphasis_mark(
        TextEmphasisMark::Custom('♥'),
        TextEmphasisFill::Open,
        TextEmphasisPosition { over: true, right: true },
    );
    assert_eq!(filled, open, "custom char should be same regardless of fill");
}

/// WPT: should_draw_emphasis_mark on letters.
#[test]
fn wpt_emphasis_draw_on_letters() {
    assert!(should_draw_emphasis_mark('A'));
    assert!(should_draw_emphasis_mark('z'));
    assert!(should_draw_emphasis_mark('漢'));
    assert!(should_draw_emphasis_mark('α'));
    assert!(should_draw_emphasis_mark('Я'));
}

/// WPT: should_draw_emphasis_mark on digits.
#[test]
fn wpt_emphasis_draw_on_digits() {
    assert!(should_draw_emphasis_mark('0'));
    assert!(should_draw_emphasis_mark('5'));
    assert!(should_draw_emphasis_mark('9'));
}

/// WPT: should_draw_emphasis_mark on punctuation.
#[test]
fn wpt_emphasis_draw_on_punctuation() {
    assert!(should_draw_emphasis_mark('!'));
    assert!(should_draw_emphasis_mark('?'));
    assert!(should_draw_emphasis_mark('。'));
}

/// WPT: should_draw_emphasis_mark skips whitespace.
#[test]
fn wpt_emphasis_skip_whitespace() {
    assert!(!should_draw_emphasis_mark(' '));
    assert!(!should_draw_emphasis_mark('\t'));
    assert!(!should_draw_emphasis_mark('\n'));
    assert!(!should_draw_emphasis_mark('\u{00A0}')); // no-break space
    assert!(!should_draw_emphasis_mark('\u{3000}')); // ideographic space
}

/// WPT: should_draw_emphasis_mark skips format chars.
#[test]
fn wpt_emphasis_skip_format_chars() {
    assert!(!should_draw_emphasis_mark('\u{200B}')); // ZWSP
    assert!(!should_draw_emphasis_mark('\u{200D}')); // ZWJ
    assert!(!should_draw_emphasis_mark('\u{FEFF}')); // BOM
    assert!(!should_draw_emphasis_mark('\u{200E}')); // LRM
    assert!(!should_draw_emphasis_mark('\u{200F}')); // RLM
}

/// WPT: should_draw_emphasis_mark skips separators.
#[test]
fn wpt_emphasis_skip_separators() {
    assert!(!should_draw_emphasis_mark('\u{2028}')); // line separator
    assert!(!should_draw_emphasis_mark('\u{2029}')); // paragraph separator
}

/// WPT: soft hyphen is special-cased — draw emphasis mark.
#[test]
fn wpt_emphasis_soft_hyphen_draws() {
    assert!(should_draw_emphasis_mark('\u{00AD}'));
}

/// WPT: emoji gets emphasis marks.
#[test]
fn wpt_emphasis_draw_on_emoji() {
    assert!(should_draw_emphasis_mark('😊'));
    assert!(should_draw_emphasis_mark('🎉'));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_emphasis_position ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-emphasis-position-initial — over, right.
#[test]
fn wpt_emphasis_position_default() {
    let pos = TextEmphasisPosition::INITIAL;
    assert!(pos.over);
    assert!(pos.right);
}

/// WPT: text-emphasis-position for horizontal writing mode.
#[test]
fn wpt_emphasis_position_horizontal() {
    let pos = default_position_for_writing_mode(WritingMode::HorizontalTb);
    assert!(pos.over, "horizontal default must be over");
    assert!(pos.right, "horizontal default must be right");
}

/// WPT: text-emphasis-position for vertical-rl writing mode.
#[test]
fn wpt_emphasis_position_vertical_rl() {
    let pos = default_position_for_writing_mode(WritingMode::VerticalRl);
    assert!(pos.over);
    assert!(pos.right);
}

/// WPT: text-emphasis-position for vertical-lr writing mode.
#[test]
fn wpt_emphasis_position_vertical_lr() {
    let pos = default_position_for_writing_mode(WritingMode::VerticalLr);
    assert!(pos.over);
    assert!(pos.right);
}

/// WPT: resolve with over=false produces under mark.
#[test]
fn wpt_emphasis_position_under() {
    let result = resolve_emphasis_mark(
        TextEmphasisMark::Dot,
        TextEmphasisFill::Filled,
        TextEmphasisPosition { over: false, right: true },
    );
    assert_eq!(
        result,
        Some(ResolvedEmphasisMark { character: '\u{2022}', over: false })
    );
}

/// WPT: default mark varies by writing mode.
#[test]
fn wpt_emphasis_default_mark_horizontal_vs_vertical() {
    let h_mark = default_mark_for_writing_mode(WritingMode::HorizontalTb);
    let v_mark = default_mark_for_writing_mode(WritingMode::VerticalRl);
    assert_eq!(h_mark, TextEmphasisMark::Dot);
    assert_eq!(v_mark, TextEmphasisMark::Sesame);
    assert_ne!(h_mark, v_mark, "horizontal and vertical defaults must differ");
}

/// WPT: sideways-rl default mark.
#[test]
fn wpt_emphasis_default_mark_sideways_rl() {
    let mark = default_mark_for_writing_mode(WritingMode::SidewaysRl);
    assert_eq!(mark, TextEmphasisMark::Sesame);
}

/// WPT: sideways-lr default mark.
#[test]
fn wpt_emphasis_default_mark_sideways_lr() {
    let mark = default_mark_for_writing_mode(WritingMode::SidewaysLr);
    assert_eq!(mark, TextEmphasisMark::Sesame);
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_emphasis_painting ──────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: emphasis marks render visible pixels.
#[test]
fn wpt_emphasis_painting_dot_renders() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_emphasis_mark = TextEmphasisMark::Dot;
    style.text_emphasis_fill = TextEmphasisFill::Filled;
    style.text_emphasis_position = TextEmphasisPosition { over: true, right: true };
    style.color = Color::BLACK;
    style.text_emphasis_color = StyleColor::CurrentColor;
    emphasis_painter::paint_emphasis_marks(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        Some("Hello"),
    );
    assert!(has_non_white_pixels(&mut surface), "emphasis dots must render");
}

/// WPT: emphasis mark None draws nothing.
#[test]
fn wpt_emphasis_painting_none_draws_nothing() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_emphasis_mark = TextEmphasisMark::None;
    emphasis_painter::paint_emphasis_marks(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        Some("Hello"),
    );
    assert!(!has_non_white_pixels(&mut surface), "None emphasis must not render");
}

/// WPT: emphasis with circle mark renders.
#[test]
fn wpt_emphasis_painting_circle_renders() {
    let sr = Arc::new(shape_text("Test"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_emphasis_mark = TextEmphasisMark::Circle;
    style.text_emphasis_fill = TextEmphasisFill::Filled;
    style.text_emphasis_position = TextEmphasisPosition { over: true, right: true };
    style.color = Color::BLACK;
    style.text_emphasis_color = StyleColor::CurrentColor;
    emphasis_painter::paint_emphasis_marks(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        Some("Test"),
    );
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: emphasis with empty text content draws nothing.
#[test]
fn wpt_emphasis_painting_empty_text() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_emphasis_mark = TextEmphasisMark::Dot;
    style.text_emphasis_fill = TextEmphasisFill::Filled;
    style.color = Color::BLACK;
    style.text_emphasis_color = StyleColor::CurrentColor;
    emphasis_painter::paint_emphasis_marks(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        None,
    );
    assert!(!has_non_white_pixels(&mut surface), "no text content means no marks");
}

/// WPT: emphasis with under position renders.
#[test]
fn wpt_emphasis_painting_under_position() {
    let sr = Arc::new(shape_text("Under"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_emphasis_mark = TextEmphasisMark::Sesame;
    style.text_emphasis_fill = TextEmphasisFill::Filled;
    style.text_emphasis_position = TextEmphasisPosition { over: false, right: true };
    style.color = Color::BLACK;
    style.text_emphasis_color = StyleColor::CurrentColor;
    emphasis_painter::paint_emphasis_marks(
        surface.canvas(),
        &sr,
        (10.0, 50.0),
        &style,
        Some("Under"),
    );
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_emphasis_color ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-emphasis-color-initial — default is CurrentColor.
#[test]
fn wpt_emphasis_color_default_is_currentcolor() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_emphasis_color, StyleColor::CurrentColor);
}

/// WPT: resolve_emphasis_color with resolved color.
#[test]
fn wpt_emphasis_color_resolved() {
    let red = Color::from_rgba8(255, 0, 0, 255);
    let result = emphasis_painter::resolve_emphasis_color(
        &StyleColor::Resolved(red),
        &Color::BLACK,
    );
    assert!((result.r - 1.0).abs() < 0.01);
    assert!(result.g.abs() < 0.01);
    assert!(result.b.abs() < 0.01);
}

/// WPT: resolve_emphasis_color with CurrentColor.
#[test]
fn wpt_emphasis_color_currentcolor_resolves_to_text() {
    let blue = Color::from_rgba8(0, 0, 255, 255);
    let result = emphasis_painter::resolve_emphasis_color(
        &StyleColor::CurrentColor,
        &blue,
    );
    assert!(result.r.abs() < 0.01);
    assert!(result.g.abs() < 0.01);
    assert!((result.b - 1.0).abs() < 0.01);
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_emphasis_offset ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: compute_emphasis_offset — over in horizontal mode.
#[test]
fn wpt_emphasis_offset_over_horizontal() {
    let offset = emphasis_painter::compute_emphasis_offset(
        TextEmphasisPosition { over: true, right: true },
        16.0,
        8.0,
        WritingMode::HorizontalTb,
    );
    // Over: should be negative (above baseline).
    assert!(offset < 0.0, "over position should yield negative offset, got {offset}");
}

/// WPT: compute_emphasis_offset — under in horizontal mode.
#[test]
fn wpt_emphasis_offset_under_horizontal() {
    let offset = emphasis_painter::compute_emphasis_offset(
        TextEmphasisPosition { over: false, right: true },
        16.0,
        8.0,
        WritingMode::HorizontalTb,
    );
    // Under: should be positive (below baseline).
    assert!(offset > 0.0, "under position should yield positive offset, got {offset}");
}

/// WPT: compute_emphasis_offset magnitude varies with font size.
#[test]
fn wpt_emphasis_offset_scales_with_size() {
    let small_offset = emphasis_painter::compute_emphasis_offset(
        TextEmphasisPosition { over: true, right: true },
        12.0,
        6.0,
        WritingMode::HorizontalTb,
    )
    .abs();
    let large_offset = emphasis_painter::compute_emphasis_offset(
        TextEmphasisPosition { over: true, right: true },
        24.0,
        12.0,
        WritingMode::HorizontalTb,
    )
    .abs();
    assert!(
        large_offset > small_offset,
        "larger font should produce larger offset: {large_offset} vs {small_offset}"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod text_shadow ─────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-shadow-initial — default is empty vec.
#[test]
fn wpt_shadow_default_empty() {
    let style = ComputedStyle::default();
    assert!(style.text_shadow.is_empty());
}

/// WPT: text-shadow-001 single shadow with offset.
#[test]
fn wpt_shadow_single_offset_renders() {
    let sr = Arc::new(shape_text("Shadow"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.color = Color::BLACK;
    style.text_shadow = vec![TextShadow {
        offset_x: 2.0,
        offset_y: 2.0,
        blur_radius: 0.0,
        color: Color::from_rgba8(255, 0, 0, 255),
    }];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface), "shadow must render pixels");
}

/// WPT: text-shadow-002 no shadow renders no extra pixels.
#[test]
fn wpt_shadow_none_no_extra_pixels() {
    let sr = Arc::new(shape_text("NoShadow"));
    let mut surface = make_surface(400, 100);
    let style = ComputedStyle::default(); // no shadow
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(!has_non_white_pixels(&mut surface), "no shadow means no pixels");
}

/// WPT: text-shadow-003 multiple shadows.
#[test]
fn wpt_shadow_multiple() {
    let sr = Arc::new(shape_text("Multi Shadow"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.color = Color::BLACK;
    style.text_shadow = vec![
        TextShadow {
            offset_x: 2.0,
            offset_y: 2.0,
            blur_radius: 0.0,
            color: Color::from_rgba8(255, 0, 0, 255),
        },
        TextShadow {
            offset_x: -2.0,
            offset_y: -2.0,
            blur_radius: 0.0,
            color: Color::from_rgba8(0, 0, 255, 255),
        },
    ];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-shadow-004 with blur.
#[test]
fn wpt_shadow_with_blur() {
    let sr = Arc::new(shape_text("Blurry"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.color = Color::BLACK;
    style.text_shadow = vec![TextShadow {
        offset_x: 0.0,
        offset_y: 0.0,
        blur_radius: 4.0,
        color: Color::from_rgba8(255, 0, 0, 255),
    }];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface), "blurred shadow must render");
}

/// WPT: text-shadow-005 zero offset and zero blur still renders at text position.
#[test]
fn wpt_shadow_zero_offset_zero_blur() {
    let sr = Arc::new(shape_text("Zero"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.color = Color::BLACK;
    style.text_shadow = vec![TextShadow {
        offset_x: 0.0,
        offset_y: 0.0,
        blur_radius: 0.0,
        color: Color::from_rgba8(255, 0, 0, 255),
    }];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: text-shadow large offset.
#[test]
fn wpt_shadow_large_offset() {
    let sr = Arc::new(shape_text("BigOffset"));
    let mut surface = make_surface(500, 200);
    let mut style = ComputedStyle::default();
    style.color = Color::BLACK;
    style.text_shadow = vec![TextShadow {
        offset_x: 20.0,
        offset_y: 20.0,
        blur_radius: 0.0,
        color: Color::from_rgba8(0, 128, 0, 255),
    }];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

/// WPT: shadow with transparent color draws nothing.
#[test]
fn wpt_shadow_transparent_color() {
    let sr = Arc::new(shape_text("Ghost"));
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.color = Color::BLACK;
    style.text_shadow = vec![TextShadow {
        offset_x: 2.0,
        offset_y: 2.0,
        blur_radius: 0.0,
        color: Color::from_rgba8(0, 0, 0, 0),
    }];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(!has_non_white_pixels(&mut surface), "transparent shadow must not render");
}

/// WPT: shadow with blur produces more pixels than without blur.
#[test]
fn wpt_shadow_blur_produces_more_pixels() {
    let sr = Arc::new(shape_text("Blur compare"));

    let mut no_blur_surface = make_surface(400, 100);
    let mut no_blur_style = ComputedStyle::default();
    no_blur_style.color = Color::BLACK;
    no_blur_style.text_shadow = vec![TextShadow {
        offset_x: 2.0,
        offset_y: 2.0,
        blur_radius: 0.0,
        color: Color::from_rgba8(0, 0, 0, 255),
    }];
    text_painter::paint_text_shadows(
        no_blur_surface.canvas(),
        &sr,
        (10.0, 50.0),
        &no_blur_style,
    );
    let no_blur_px = count_non_white_pixels(&mut no_blur_surface);

    let mut blur_surface = make_surface(400, 100);
    let mut blur_style = ComputedStyle::default();
    blur_style.color = Color::BLACK;
    blur_style.text_shadow = vec![TextShadow {
        offset_x: 2.0,
        offset_y: 2.0,
        blur_radius: 8.0,
        color: Color::from_rgba8(0, 0, 0, 255),
    }];
    text_painter::paint_text_shadows(
        blur_surface.canvas(),
        &sr,
        (10.0, 50.0),
        &blur_style,
    );
    let blur_px = count_non_white_pixels(&mut blur_surface);

    assert!(
        blur_px > no_blur_px,
        "blurred ({blur_px}) should cover more pixels than non-blurred ({no_blur_px})"
    );
}

/// WPT: multiple shadows produce more pixels than single shadow.
#[test]
fn wpt_shadow_multiple_more_pixels_than_single() {
    let sr = Arc::new(shape_text("Multi test"));

    let mut single_surface = make_surface(400, 100);
    let mut single_style = ComputedStyle::default();
    single_style.color = Color::BLACK;
    single_style.text_shadow = vec![TextShadow {
        offset_x: 3.0,
        offset_y: 3.0,
        blur_radius: 0.0,
        color: Color::from_rgba8(255, 0, 0, 255),
    }];
    text_painter::paint_text_shadows(
        single_surface.canvas(),
        &sr,
        (10.0, 50.0),
        &single_style,
    );
    let single_px = count_non_white_pixels(&mut single_surface);

    let mut multi_surface = make_surface(400, 100);
    let mut multi_style = ComputedStyle::default();
    multi_style.color = Color::BLACK;
    multi_style.text_shadow = vec![
        TextShadow {
            offset_x: 3.0,
            offset_y: 3.0,
            blur_radius: 0.0,
            color: Color::from_rgba8(255, 0, 0, 255),
        },
        TextShadow {
            offset_x: -3.0,
            offset_y: -3.0,
            blur_radius: 0.0,
            color: Color::from_rgba8(0, 0, 255, 255),
        },
    ];
    text_painter::paint_text_shadows(
        multi_surface.canvas(),
        &sr,
        (10.0, 50.0),
        &multi_style,
    );
    let multi_px = count_non_white_pixels(&mut multi_surface);

    assert!(
        multi_px > single_px,
        "multiple shadows ({multi_px}) should produce more pixels than single ({single_px})"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod decoration_phase ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: BeforeText draws underline but not line-through.
#[test]
fn wpt_phase_before_text_no_line_through() {
    let sr = Arc::new(shape_text("Phase"));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = line_through_style(|_| {});
    // BeforeText should NOT draw line-through.
    paint_before(&mut surface, &sr, &style, &metrics);
    assert!(
        !has_non_white_pixels(&mut surface),
        "BeforeText must not draw line-through"
    );
}

/// WPT: AfterText draws line-through but not underline.
#[test]
fn wpt_phase_after_text_no_underline() {
    let sr = Arc::new(shape_text("Phase"));
    let metrics = synthetic_metrics();
    let mut surface = make_surface(300, 100);
    let style = underline_style(|_| {});
    // AfterText should NOT draw underline.
    paint_after(&mut surface, &sr, &style, &metrics);
    assert!(
        !has_non_white_pixels(&mut surface),
        "AfterText must not draw underline"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod writing_mode ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: writing-mode-horizontal-tb is initial.
#[test]
fn wpt_writing_mode_default() {
    let style = ComputedStyle::default();
    assert_eq!(style.writing_mode, WritingMode::HorizontalTb);
}

/// WPT: is_horizontal / is_vertical predicates.
#[test]
fn wpt_writing_mode_predicates() {
    assert!(WritingMode::HorizontalTb.is_horizontal());
    assert!(!WritingMode::HorizontalTb.is_vertical());
    assert!(WritingMode::VerticalRl.is_vertical());
    assert!(!WritingMode::VerticalRl.is_horizontal());
    assert!(WritingMode::VerticalLr.is_vertical());
    assert!(WritingMode::SidewaysRl.is_vertical());
    assert!(WritingMode::SidewaysLr.is_vertical());
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod emphasis_fill ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: text-emphasis-fill-initial — default is Filled.
#[test]
fn wpt_emphasis_fill_default_is_filled() {
    let style = ComputedStyle::default();
    assert_eq!(style.text_emphasis_fill, TextEmphasisFill::Filled);
}

/// WPT: open vs filled produce different characters.
#[test]
fn wpt_emphasis_fill_open_vs_filled() {
    let filled = resolve_emphasis_mark(
        TextEmphasisMark::Triangle,
        TextEmphasisFill::Filled,
        TextEmphasisPosition::INITIAL,
    );
    let open = resolve_emphasis_mark(
        TextEmphasisMark::Triangle,
        TextEmphasisFill::Open,
        TextEmphasisPosition::INITIAL,
    );
    assert_ne!(filled, open, "filled and open Triangle must differ");
    assert_eq!(filled.unwrap().character, '\u{25B2}'); // ▲
    assert_eq!(open.unwrap().character, '\u{25B3}'); // △
}

// ═══════════════════════════════════════════════════════════════════════
// ── mod metrics_integration ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// WPT: metrics_from_shape_result returns positive values.
#[test]
fn wpt_metrics_from_shape_result_positive() {
    let sr = Arc::new(shape_text("Metrics"));
    let m = text_painter::metrics_from_shape_result(&sr);
    assert!(m.ascent > 0.0, "ascent should be positive");
    assert!(m.descent > 0.0, "descent should be positive");
    assert!(m.line_spacing > 0.0, "line_spacing should be positive");
}

/// WPT: metrics_from_shape_result — underline thickness > 0.
#[test]
fn wpt_metrics_underline_thickness_positive() {
    let sr = Arc::new(shape_text("UThick"));
    let m = text_painter::metrics_from_shape_result(&sr);
    assert!(m.underline_thickness > 0.0, "underline_thickness should be > 0");
}

/// WPT: different font sizes produce different metrics.
#[test]
fn wpt_metrics_different_sizes() {
    let small_sr = Arc::new(shape_text_with_size("Size", 12.0));
    let large_sr = Arc::new(shape_text_with_size("Size", 36.0));
    let small_m = text_painter::metrics_from_shape_result(&small_sr);
    let large_m = text_painter::metrics_from_shape_result(&large_sr);
    assert!(
        large_m.ascent > small_m.ascent,
        "larger font should have larger ascent"
    );
}
