//! Text decoration painting — underline, overline, line-through.
//!
//! Extracted from Blink's `TextDecorationPainter`
//! (`core/paint/text_decoration_painter.cc`).
//!
//! Decoration lines are positioned relative to the text baseline using
//! font metrics. Each decoration style (solid, double, dotted, dashed, wavy)
//! uses specific Skia draw calls matching Blink's implementation.

use skia_safe::{Canvas, ColorSpace, Paint, PaintStyle, Path, PathEffect, Point, Rect};

use openui_style::ComputedStyle;
use openui_style::{TextDecorationStyle, TextDecorationThickness};
use openui_text::font::FontMetrics;
use openui_text::shaping::ShapeResult;

use crate::text_painter::to_sk_color4f;

/// Phase of decoration painting relative to text glyphs.
///
/// CSS spec requires underline/overline to be painted behind text glyphs,
/// while line-through must be painted in front of text glyphs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationPhase {
    /// Underline + overline (painted behind text).
    BeforeText,
    /// Line-through (painted in front of text).
    AfterText,
}

/// Paint text decorations (underline, overline, line-through) for a text fragment.
///
/// Mirrors Blink's `TextDecorationPainter::Paint()`.
///
/// Decorations are drawn relative to the baseline position. The inline layout
/// algorithm positions text fragments such that `origin.1` is the baseline Y.
///
/// # Arguments
/// * `canvas` — Skia raster canvas
/// * `shape_result` — For measuring text advance width
/// * `origin` — (x, baseline_y) in device pixels
/// * `style` — Computed style with decoration properties
/// * `metrics` — Font metrics for decoration positioning
pub fn paint_text_decorations(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
    metrics: &FontMetrics,
    phase: DecorationPhase,
) {
    let decoration_line = style.text_decoration_line;
    if decoration_line.is_none() {
        return;
    }

    let width = shape_result.width();
    if width <= 0.0 {
        return;
    }

    let x = origin.0;
    let baseline_y = origin.1;

    // Resolve decoration color.
    // Blink: TextDecorationInfo::ResolvedColor()
    let resolved_color = style.text_decoration_color.resolve(&style.color);

    // Resolve decoration thickness.
    // Blink: TextDecorationInfo::ResolvedThickness()
    let thickness = resolve_thickness(&style.text_decoration_thickness, metrics);

    // Build the base paint for decorations.
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color4f(to_sk_color4f(&resolved_color), None::<&ColorSpace>);

    match phase {
        DecorationPhase::BeforeText => {
            // ── Underline ────────────────────────────────────────────────────
            // Blink: underline_offset = font_metrics.UnderlinePosition()
            // which is a positive value below the baseline.
            if decoration_line.has_underline() {
                let y = baseline_y + metrics.underline_offset;
                draw_decoration_line(canvas, &paint, x, y, width, &style.text_decoration_style, thickness);
            }

            // ── Overline ─────────────────────────────────────────────────────
            // Blink: overline positioned at -ascent from baseline (top of em box).
            if decoration_line.has_overline() {
                let y = baseline_y - metrics.ascent;
                draw_decoration_line(canvas, &paint, x, y, width, &style.text_decoration_style, thickness);
            }
        }
        DecorationPhase::AfterText => {
            // ── Line-through ─────────────────────────────────────────────────
            // Blink: strikeout_position is positive above baseline.
            if decoration_line.has_line_through() {
                let y = baseline_y - metrics.strikeout_position;
                draw_decoration_line(canvas, &paint, x, y, width, &style.text_decoration_style, thickness);
            }
        }
    }
}

/// Resolve the decoration thickness from the computed style and font metrics.
///
/// Blink: `TextDecorationInfo::ComputeThickness()` in
/// `core/paint/text_decoration_info.cc`.
fn resolve_thickness(thickness: &TextDecorationThickness, metrics: &FontMetrics) -> f32 {
    let t = match thickness {
        TextDecorationThickness::Auto => {
            // Blink uses the font's underline thickness, clamped to at least 1 CSS pixel.
            metrics.underline_thickness.max(1.0)
        }
        TextDecorationThickness::FromFont => {
            // Blink: from-font uses the font's preferred thickness,
            // clamped to at least 1 CSS pixel to avoid zero-width decorations.
            metrics.underline_thickness.max(1.0)
        }
        TextDecorationThickness::Length(px) => *px,
    };
    // Clamp to a positive minimum to prevent infinite loops in draw routines
    // (dotted/wavy loops rely on positive spacing derived from thickness).
    t.max(0.5)
}

/// Draw a single decoration line with the given style.
///
/// Blink: `TextDecorationPainter::PaintDecorationUnderOrOverLine()` and
/// `PaintDecorationLineThrough()` dispatch to style-specific drawing.
fn draw_decoration_line(
    canvas: &Canvas,
    paint: &Paint,
    x: f32,
    y: f32,
    width: f32,
    decoration_style: &TextDecorationStyle,
    thickness: f32,
) {
    match decoration_style {
        TextDecorationStyle::Solid => {
            draw_solid_line(canvas, paint, x, y, width, thickness);
        }
        TextDecorationStyle::Double => {
            draw_double_line(canvas, paint, x, y, width, thickness);
        }
        TextDecorationStyle::Dotted => {
            draw_dotted_line(canvas, paint, x, y, width, thickness);
        }
        TextDecorationStyle::Dashed => {
            draw_dashed_line(canvas, paint, x, y, width, thickness);
        }
        TextDecorationStyle::Wavy => {
            draw_wavy_line(canvas, paint, x, y, width, thickness);
        }
    }
}

/// Solid decoration: a filled rectangle.
///
/// Blink: Draws a filled rect at (x, y - thickness/2) with the full width.
/// The rect is centered on the decoration line position.
fn draw_solid_line(canvas: &Canvas, paint: &Paint, x: f32, y: f32, width: f32, thickness: f32) {
    let mut fill_paint = paint.clone();
    fill_paint.set_style(PaintStyle::Fill);
    let rect = Rect::from_xywh(x, y - thickness / 2.0, width, thickness);
    canvas.draw_rect(rect, &fill_paint);
}

/// Double decoration: two parallel solid lines separated by a gap.
///
/// Blink: `TextDecorationInfo::PaintDoubleDecorationLine()`.
/// The gap between lines equals the line thickness × 1.5 (matching Gecko).
fn draw_double_line(canvas: &Canvas, paint: &Paint, x: f32, y: f32, width: f32, thickness: f32) {
    let gap = thickness * 1.5;
    let half_t = thickness / 2.0;
    let mut fill_paint = paint.clone();
    fill_paint.set_style(PaintStyle::Fill);

    // First line
    let rect1 = Rect::from_xywh(x, y - half_t, width, thickness);
    canvas.draw_rect(rect1, &fill_paint);

    // Second line below
    let rect2 = Rect::from_xywh(x, y - half_t + thickness + gap, width, thickness);
    canvas.draw_rect(rect2, &fill_paint);
}

/// Dotted decoration: a series of circular dots.
///
/// Blink: Uses a round cap with dash path effect for dots.
/// We draw individual circles for pixel-perfect matching.
fn draw_dotted_line(canvas: &Canvas, paint: &Paint, x: f32, y: f32, width: f32, thickness: f32) {
    let mut dot_paint = paint.clone();
    dot_paint.set_style(PaintStyle::Fill);

    let dot_diameter = thickness;
    let dot_radius = dot_diameter / 2.0;
    // Blink: dot spacing = diameter * 2 (center-to-center)
    let spacing = dot_diameter * 2.0;

    let mut cx = x + dot_radius;
    while cx < x + width {
        canvas.draw_circle(Point::new(cx, y), dot_radius, &dot_paint);
        cx += spacing;
    }
}

/// Dashed decoration: a line with a dash path effect.
///
/// Blink: `TextDecorationPainter` uses `SkDashPathEffect` with
/// dash_length = 3 × thickness and gap = 2 × thickness.
fn draw_dashed_line(canvas: &Canvas, paint: &Paint, x: f32, y: f32, width: f32, thickness: f32) {
    let dash_length = thickness * 3.0;
    let gap_length = thickness * 2.0;

    let mut stroke_paint = paint.clone();
    stroke_paint.set_style(PaintStyle::Stroke);
    stroke_paint.set_stroke_width(thickness);

    if let Some(effect) = PathEffect::dash(&[dash_length, gap_length], 0.0) {
        stroke_paint.set_path_effect(effect);
    }

    canvas.draw_line(Point::new(x, y), Point::new(x + width, y), &stroke_paint);
}

/// Wavy decoration: a sinusoidal wave.
///
/// Blink: `TextDecorationPainter::PaintWavyTextDecoration()`.
/// Implemented as a series of quadratic Bézier curves forming a wave
/// with amplitude = thickness and wavelength = 4 × thickness.
fn draw_wavy_line(canvas: &Canvas, paint: &Paint, x: f32, y: f32, width: f32, thickness: f32) {
    let amplitude = thickness;
    let half_wavelength = thickness * 2.0;

    let mut path = Path::new();
    path.move_to(Point::new(x, y));

    let mut cx = x;
    let mut up = true;
    while cx < x + width {
        let ctrl_y = if up { y - amplitude } else { y + amplitude };
        let end_x = (cx + half_wavelength).min(x + width);
        let ctrl_x = cx + half_wavelength / 2.0;
        path.quad_to(Point::new(ctrl_x, ctrl_y), Point::new(end_x, y));
        cx = end_x;
        up = !up;
    }

    let mut stroke_paint = paint.clone();
    stroke_paint.set_style(PaintStyle::Stroke);
    stroke_paint.set_stroke_width(thickness);
    canvas.draw_path(&path, &stroke_paint);
}
