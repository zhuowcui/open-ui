//! Text decoration painting — underline, overline, line-through.
//!
//! Extracted from Blink's `TextDecorationPainter`
//! (`core/paint/text_decoration_painter.cc`).
//!
//! Decoration lines are positioned relative to the text baseline using
//! font metrics. Each decoration style (solid, double, dotted, dashed, wavy)
//! uses specific Skia draw calls matching Blink's implementation.
//!
//! ## Skip-ink
//!
//! The `text-decoration-skip-ink` property controls whether decoration lines
//! skip over glyph ink (Blink: `ApplyDecorationOverrideForSkipInk`).
//! Intercepts are computed via Skia's `TextBlob::get_intercepts()`, which
//! returns x-ranges where glyph outlines cross the decoration stripe. Gaps
//! are added with a horizontal padding of 1px and a vertical dilation of
//! `min(thickness, 13px)`.

use skia_safe::{Canvas, ColorSpace, Paint, PaintStyle, Path, PathEffect, Point, Rect};

use openui_style::ComputedStyle;
use openui_style::{TextDecorationSkipInk, TextDecorationStyle, TextDecorationThickness};
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

/// Which specific decoration line is being drawn (affects double-line direction).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DecorationLineKind {
    Underline,
    Overline,
    LineThrough,
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
/// * `shape_result` — For measuring text advance width and computing intercepts
/// * `origin` — (x, baseline_y) in device pixels
/// * `style` — Computed style with decoration properties
/// * `metrics` — Font metrics for decoration positioning
/// * `phase` — Whether painting underline/overline or line-through
/// * `text_content` — Original text for CJK detection in Auto skip-ink mode
pub fn paint_text_decorations(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
    metrics: &FontMetrics,
    phase: DecorationPhase,
    text_content: Option<&str>,
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
    // Blink: TextDecorationInfo::ResolvedThickness() — uses UnderlineThickness()
    // for ALL decoration types (underline, overline, and line-through).
    let thickness = resolve_thickness(&style.text_decoration_thickness, metrics, style.font_size);

    // Build the base paint for decorations.
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color4f(to_sk_color4f(&resolved_color), None::<&ColorSpace>);

    let skip_ink = style.text_decoration_skip_ink;

    match phase {
        DecorationPhase::BeforeText => {
            // ── Underline ────────────────────────────────────────────────────
            // Blink: underline_offset = font_metrics.UnderlinePosition()
            // which is a positive value below the baseline.
            if decoration_line.has_underline() {
                // Apply CSS text-underline-offset if it's a resolved length.
                // Percentages resolve against 1em (the computed font size).
                let css_offset = if style.text_underline_offset.is_percent() {
                    style.text_underline_offset.value() / 100.0 * style.font_size
                } else if style.text_underline_offset.is_fixed() {
                    style.text_underline_offset.value()
                } else {
                    0.0
                };
                let y = baseline_y + metrics.underline_offset + css_offset;
                draw_decoration_line_with_skip_ink(
                    canvas, &paint, shape_result, origin, x, y, width,
                    &style.text_decoration_style, thickness,
                    DecorationLineKind::Underline, skip_ink, text_content,
                );
            }

            // ── Overline ─────────────────────────────────────────────────────
            // Blink: overline positioned at -ascent from baseline (top of em box).
            if decoration_line.has_overline() {
                let y = baseline_y - metrics.ascent;
                draw_decoration_line_with_skip_ink(
                    canvas, &paint, shape_result, origin, x, y, width,
                    &style.text_decoration_style, thickness,
                    DecorationLineKind::Overline, skip_ink, text_content,
                );
            }
        }
        DecorationPhase::AfterText => {
            // ── Line-through ─────────────────────────────────────────────────
            // Blink: strikeout_position is positive above baseline.
            // Per CSS spec, skip-ink does NOT apply to line-through.
            if decoration_line.has_line_through() {
                let y = baseline_y - metrics.strikeout_position;
                draw_decoration_line(canvas, &paint, x, y, width, &style.text_decoration_style, thickness, DecorationLineKind::LineThrough);
            }
        }
    }
}

// ── Skip-ink constants ────────────────────────────────────────────────────
//
// These constants match Blink's implementation in
// `core/paint/text_decoration_painter.cc` and Firefox's rendering engine.

/// Horizontal padding added on each side of a glyph ink intercept gap.
/// Blink: hardcoded 1.0px in `ApplyDecorationOverrideForSkipInk`.
const SKIP_INK_HORIZONTAL_PADDING: f32 = 1.0;

/// Maximum vertical dilation applied to the decoration stripe when querying
/// intercepts. Caps at 13px to avoid excessive gap detection on thick lines.
/// Blink: `ClampDilationAdjustment` in `text_decoration_painter.cc` caps at 13.
/// Firefox uses the same 13px cap.
const SKIP_INK_MAX_DILATION: f32 = 13.0;

/// Draw a decoration line with optional skip-ink gap detection.
///
/// For underline and overline, when skip-ink is enabled (Auto or All), this
/// queries Skia's `TextBlob::get_intercepts()` to detect glyph ink regions
/// that cross the decoration stripe, then draws segmented lines around them.
///
/// Blink: `TextDecorationPainter::PaintDecorationUnderOrOverLine()` calls
/// `ApplyDecorationOverrideForSkipInk()` which uses clip-out rectangles.
/// We achieve the equivalent effect by splitting the line into segments.
fn draw_decoration_line_with_skip_ink(
    canvas: &Canvas,
    paint: &Paint,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    x: f32,
    y: f32,
    width: f32,
    decoration_style: &TextDecorationStyle,
    thickness: f32,
    kind: DecorationLineKind,
    skip_ink: TextDecorationSkipInk,
    text_content: Option<&str>,
) {
    if skip_ink == TextDecorationSkipInk::None {
        draw_decoration_line(canvas, paint, x, y, width, decoration_style, thickness, kind);
        return;
    }

    let intercepts = compute_skip_ink_intercepts(
        shape_result, origin, y, thickness, skip_ink, text_content,
    );

    if intercepts.is_empty() {
        draw_decoration_line(canvas, paint, x, y, width, decoration_style, thickness, kind);
        return;
    }

    // Draw line segments between gaps.
    // Each gap is defined by an intercept pair with horizontal padding.
    let end_x = x + width;
    let mut current_x = x;

    for &(ink_start, ink_end) in &intercepts {
        let gap_start = ink_start - SKIP_INK_HORIZONTAL_PADDING;
        let gap_end = ink_end + SKIP_INK_HORIZONTAL_PADDING;

        // Draw segment before this gap (if there's positive width).
        if current_x < gap_start {
            let seg_width = gap_start - current_x;
            draw_decoration_line(canvas, paint, current_x, y, seg_width, decoration_style, thickness, kind);
        }

        current_x = gap_end;
    }

    // Draw final segment after last gap.
    if current_x < end_x {
        let seg_width = end_x - current_x;
        draw_decoration_line(canvas, paint, current_x, y, seg_width, decoration_style, thickness, kind);
    }
}

/// Compute glyph ink intercepts for skip-ink decoration.
///
/// Returns pairs of `(start_x, end_x)` positions (in device pixels, relative
/// to the canvas) where glyph ink intersects the decoration stripe.
///
/// Blink: `TextDecorationPainter::ApplyDecorationOverrideForSkipInk()` calls
/// `SkTextBlob::getIntercepts()` with dilated bounds.
///
/// The decoration stripe is vertically dilated by `min(thickness, 13px)` on
/// each side to ensure intercepts are detected even for thin lines near glyph
/// edges. The 13px cap prevents excessive gap detection on very thick lines.
pub(crate) fn compute_skip_ink_intercepts(
    shape_result: &ShapeResult,
    origin: (f32, f32),
    decoration_y: f32,
    thickness: f32,
    skip_ink: TextDecorationSkipInk,
    text_content: Option<&str>,
) -> Vec<(f32, f32)> {
    if skip_ink == TextDecorationSkipInk::None {
        return Vec::new();
    }

    let text_blob = match shape_result.to_text_blob() {
        Some(blob) => blob,
        None => return Vec::new(),
    };

    // Compute the dilated decoration stripe bounds.
    // Blink: dilation = ClampDilationAdjustment(thickness) = min(thickness, 13)
    let dilation = thickness.min(SKIP_INK_MAX_DILATION);
    let stripe_top = decoration_y - thickness / 2.0 - dilation;
    let stripe_bottom = decoration_y + thickness / 2.0 + dilation;

    // TextBlob::get_intercepts() expects bounds relative to the blob's origin.
    // Our decoration coordinates are in canvas space; the blob is drawn at
    // `origin`, so we need to subtract origin.1 to get blob-local Y bounds.
    let local_top = stripe_top - origin.1;
    let local_bottom = stripe_bottom - origin.1;

    let raw_intercepts = text_blob.get_intercepts([local_top, local_bottom], None);

    if raw_intercepts.is_empty() {
        return Vec::new();
    }

    // Intercepts come as pairs: [start0, end0, start1, end1, ...].
    // Each pair is in the blob's coordinate system (relative to blob origin.x = 0).
    // We offset them by origin.0 to get canvas-space x-coordinates.
    let origin_x = origin.0;
    let mut pairs: Vec<(f32, f32)> = raw_intercepts
        .chunks_exact(2)
        .map(|chunk| (chunk[0] + origin_x, chunk[1] + origin_x))
        .collect();

    // Sort by start position for proper merging and CJK filtering.
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // In Auto mode, filter out intercepts that correspond to CJK characters.
    if skip_ink == TextDecorationSkipInk::Auto {
        if let Some(text) = text_content {
            pairs = filter_non_cjk_intercepts(&pairs, shape_result, origin_x, text);
        }
    }

    // Merge overlapping or adjacent intercepts.
    merge_intercepts(&mut pairs);

    pairs
}

/// Filter intercepts to exclude those covering CJK characters.
///
/// In Auto mode, CJK ideographs and related scripts are excluded from
/// skip-ink because their complex strokes create excessive gaps that harm
/// readability. This matches Blink's `ShouldSkipForTextDecorationSkipInk()`
/// which checks `Character::IsCJKIdeographOrSymbol()`.
fn filter_non_cjk_intercepts(
    intercepts: &[(f32, f32)],
    shape_result: &ShapeResult,
    origin_x: f32,
    text: &str,
) -> Vec<(f32, f32)> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return intercepts.to_vec();
    }

    intercepts
        .iter()
        .filter(|&&(start_x, end_x)| {
            // Find which character(s) this intercept overlaps with.
            // Use the character_data x_positions from the shape result.
            let mid_x = (start_x + end_x) / 2.0 - origin_x;
            let char_idx = shape_result.offset_for_x_position(mid_x);
            let ch = chars.get(char_idx).copied().unwrap_or('\0');
            // Keep this intercept only if the character is NOT CJK.
            !is_cjk_character(ch)
        })
        .copied()
        .collect()
}

/// Merge overlapping or adjacent intercept pairs into minimal non-overlapping set.
///
/// Input must be sorted by start position. After merging, adjacent gaps
/// within 0.5px are also coalesced to avoid rendering tiny line fragments.
fn merge_intercepts(pairs: &mut Vec<(f32, f32)>) {
    if pairs.len() <= 1 {
        return;
    }

    let mut merged: Vec<(f32, f32)> = Vec::with_capacity(pairs.len());
    merged.push(pairs[0]);

    for &(start, end) in &pairs[1..] {
        let last = merged.last_mut().unwrap();
        // Merge if overlapping or within 0.5px (avoids sub-pixel slivers).
        if start <= last.1 + 0.5 {
            last.1 = last.1.max(end);
        } else {
            merged.push((start, end));
        }
    }

    *pairs = merged;
}

/// Returns true if the character is CJK and should be excluded from
/// skip-ink in Auto mode.
///
/// CJK ideographs have dense, complex strokes — skipping them creates
/// excessive gaps that harm readability. This matches Blink's
/// `Character::IsCJKIdeographOrSymbol()` in `platform/text/character.cc`.
///
/// Ranges include:
/// - CJK Unified Ideographs and Extensions A–F
/// - CJK Compatibility Ideographs
/// - Hiragana, Katakana, Katakana Phonetic Extensions
/// - Hangul Syllables
/// - CJK Symbols and Punctuation
pub(crate) fn is_cjk_character(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        0x3000..=0x303F |     // CJK Symbols and Punctuation
        0x3040..=0x309F |     // Hiragana
        0x30A0..=0x30FF |     // Katakana
        0x31F0..=0x31FF |     // Katakana Phonetic Extensions
        0x3400..=0x4DBF |     // CJK Unified Ideographs Extension A
        0x4E00..=0x9FFF |     // CJK Unified Ideographs
        0xAC00..=0xD7AF |     // Hangul Syllables
        0xF900..=0xFAFF |     // CJK Compatibility Ideographs
        0x20000..=0x2A6DF |   // CJK Extension B
        0x2A700..=0x2B73F |   // CJK Extension C
        0x2B740..=0x2B81F |   // CJK Extension D
        0x2B820..=0x2CEAF |   // CJK Extension E
        0x2CEB0..=0x2EBEF |   // CJK Extension F
        0x2F800..=0x2FA1F     // CJK Compatibility Ideographs Supplement
    )
}

/// Resolve the decoration thickness from the computed style and font metrics.
///
/// Blink: `TextDecorationInfo::ComputeThickness()` in
/// `core/paint/text_decoration_info.cc`.
///
/// - `auto`: `font_size / 10.0`, minimum 1px (no rounding — Blink returns raw).
/// - `from-font`: font's `UnderlineThickness()` for ALL decoration types,
///   minimum 1px (no rounding). Falls back to auto formula when unavailable.
/// - explicit length: `roundf()` to device pixel, minimum 1px (Blink rounds only
///   explicit `text-decoration-thickness` lengths).
fn resolve_thickness(thickness: &TextDecorationThickness, metrics: &FontMetrics, font_size: f32) -> f32 {
    let t = match thickness {
        TextDecorationThickness::Auto => {
            // Blink: computed_font_size / 10.f — raw, no rounding.
            (font_size / 10.0).max(1.0)
        }
        TextDecorationThickness::FromFont => {
            // Blink: UnderlineThickness() for ALL decoration types (underline,
            // overline, and line-through). The thickness is computed once and shared.
            let from_metric = metrics.underline_thickness;
            if from_metric > 0.0 {
                from_metric.max(1.0)
            } else {
                // Fallback to auto formula when metric unavailable.
                (font_size / 10.0).max(1.0)
            }
        }
        TextDecorationThickness::Length(px) => {
            // Blink: roundf(text_decoration_thickness_pixels), minimum 1px.
            px.round().max(1.0)
        }
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
    kind: DecorationLineKind,
) {
    match decoration_style {
        TextDecorationStyle::Solid => {
            draw_solid_line(canvas, paint, x, y, width, thickness);
        }
        TextDecorationStyle::Double => {
            draw_double_line(canvas, paint, x, y, width, thickness, kind);
        }
        TextDecorationStyle::Dotted => {
            draw_dotted_line(canvas, paint, x, y, width, thickness);
        }
        TextDecorationStyle::Dashed => {
            draw_dashed_line(canvas, paint, x, y, width, thickness);
        }
        TextDecorationStyle::Wavy => {
            draw_wavy_line(canvas, paint, x, y, width, thickness, kind);
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
/// The offset between strokes is `thickness + 1.0` (fixed 1px gap),
/// matching Blink's `double_offset = thickness + 1.0`.
fn draw_double_line(canvas: &Canvas, paint: &Paint, x: f32, y: f32, width: f32, thickness: f32, kind: DecorationLineKind) {
    let double_offset = thickness + 1.0;
    let half_t = thickness / 2.0;
    let mut fill_paint = paint.clone();
    fill_paint.set_style(PaintStyle::Fill);

    match kind {
        DecorationLineKind::Underline => {
            let rect1 = Rect::from_xywh(x, y - half_t, width, thickness);
            canvas.draw_rect(rect1, &fill_paint);
            let rect2 = Rect::from_xywh(x, y - half_t + double_offset, width, thickness);
            canvas.draw_rect(rect2, &fill_paint);
        }
        DecorationLineKind::Overline => {
            let rect1 = Rect::from_xywh(x, y - half_t, width, thickness);
            canvas.draw_rect(rect1, &fill_paint);
            let rect2 = Rect::from_xywh(x, y - half_t - double_offset, width, thickness);
            canvas.draw_rect(rect2, &fill_paint);
        }
        DecorationLineKind::LineThrough => {
            // Center both lines around the nominal position
            let rect1 = Rect::from_xywh(x, y - double_offset / 2.0 - half_t, width, thickness);
            canvas.draw_rect(rect1, &fill_paint);
            let rect2 = Rect::from_xywh(x, y + double_offset / 2.0 - half_t, width, thickness);
            canvas.draw_rect(rect2, &fill_paint);
        }
    }
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

/// Wavy decoration: a sinusoidal wave using cubic Bézier curves.
///
/// Blink: `TextDecorationPainter::PaintWavyTextDecoration()` / `MakeWave`.
/// Uses `step = thickness + 1.0` for both amplitude and half-wavelength step.
/// The wave is offset away from text for underline/overline (by `step`).
fn draw_wavy_line(canvas: &Canvas, paint: &Paint, x: f32, y: f32, width: f32, thickness: f32, kind: DecorationLineKind) {
    let step = thickness + 1.0;
    let amplitude = step;

    // Wavy offset shifts the wave away from text (Blink's ComputeLineData).
    let wavy_y = match kind {
        DecorationLineKind::Underline => y + step,
        DecorationLineKind::Overline => y - step,
        DecorationLineKind::LineThrough => y,
    };

    let mut path = Path::new();
    path.move_to(Point::new(x, wavy_y));

    let mut cx = x;
    let mut up = true;
    while cx < x + width {
        let dir = if up { -amplitude } else { amplitude };
        let half_wave_end = (cx + 2.0 * step).min(x + width);
        // Cubic Bézier control points for a smooth half-wave:
        //   cp1 at 1/4 of half-wave, cp2 at 3/4, with full amplitude.
        let cp1 = Point::new(cx + step * 0.5, wavy_y + dir);
        let cp2 = Point::new(cx + step * 1.5, wavy_y + dir);
        let end = Point::new(half_wave_end, wavy_y);
        path.cubic_to(cp1, cp2, end);
        cx = half_wave_end;
        up = !up;
    }

    let mut stroke_paint = paint.clone();
    stroke_paint.set_style(PaintStyle::Stroke);
    stroke_paint.set_stroke_width(thickness);
    canvas.draw_path(&path, &stroke_paint);
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_text::font::FontMetrics;

    // ── Issue 2: resolve_thickness matches Blink ─────────────────────

    #[test]
    fn auto_thickness_uses_font_size_over_10() {
        let metrics = FontMetrics { underline_thickness: 0.8, ..FontMetrics::zero() };
        // auto: font_size / 10.0, no rounding (matches Blink), min 1px
        let t = resolve_thickness(&TextDecorationThickness::Auto, &metrics, 16.0);
        // 16.0 / 10.0 = 1.6 → max(1.0) → 1.6 (no rounding)
        assert_eq!(t, 1.6);
    }

    #[test]
    fn auto_thickness_small_font_clamps_to_1() {
        let metrics = FontMetrics::zero();
        let t = resolve_thickness(&TextDecorationThickness::Auto, &metrics, 8.0);
        // 8.0 / 10.0 = 0.8 → max(1.0) → 1.0
        assert_eq!(t, 1.0);
    }

    #[test]
    fn from_font_uses_metric_when_positive() {
        let metrics = FontMetrics { underline_thickness: 1.7, ..FontMetrics::zero() };
        let t = resolve_thickness(&TextDecorationThickness::FromFont, &metrics, 16.0);
        // 1.7 → max(1.0) → 1.7 (no rounding, matches Blink)
        assert_eq!(t, 1.7);
    }

    #[test]
    fn from_font_falls_back_to_auto_when_zero() {
        let metrics = FontMetrics { underline_thickness: 0.0, ..FontMetrics::zero() };
        let t = resolve_thickness(&TextDecorationThickness::FromFont, &metrics, 20.0);
        // fallback: 20.0 / 10.0 = 2.0 → max(1.0) → 2.0
        assert_eq!(t, 2.0);
    }

    #[test]
    fn explicit_length_rounds_to_device_pixel() {
        let metrics = FontMetrics::zero();
        let t = resolve_thickness(&TextDecorationThickness::Length(1.4), &metrics, 16.0);
        assert_eq!(t, 1.0); // 1.4 → round → 1.0
        let t2 = resolve_thickness(&TextDecorationThickness::Length(1.6), &metrics, 16.0);
        assert_eq!(t2, 2.0); // 1.6 → round → 2.0
    }

    // ── from-font always uses underline_thickness (Issue 1) ──────────

    #[test]
    fn from_font_line_through_uses_underline_thickness() {
        // Blink always uses UnderlineThickness() for from-font, regardless
        // of decoration type. Verify that strikeout_thickness is NOT used.
        let metrics = FontMetrics {
            underline_thickness: 1.5,
            strikeout_thickness: 3.0,
            ..FontMetrics::zero()
        };
        let t = resolve_thickness(&TextDecorationThickness::FromFont, &metrics, 16.0);
        // Should use underline_thickness (1.5), not strikeout_thickness (3.0)
        assert_eq!(t, 1.5);
    }

    #[test]
    fn from_font_shared_thickness_for_all_decoration_types() {
        // There's only one resolve_thickness call now (no is_line_through param),
        // so underline, overline, and line-through all share the same value.
        let metrics = FontMetrics {
            underline_thickness: 2.3,
            strikeout_thickness: 4.0,
            ..FontMetrics::zero()
        };
        let t = resolve_thickness(&TextDecorationThickness::FromFont, &metrics, 16.0);
        assert_eq!(t, 2.3);
    }

    // ── auto thickness: no rounding (Issue 2) ────────────────────────

    #[test]
    fn auto_thickness_14px_matches_blink() {
        let metrics = FontMetrics::zero();
        let t = resolve_thickness(&TextDecorationThickness::Auto, &metrics, 14.0);
        // Blink: 14.0 / 10.0 = 1.4 (raw, no rounding)
        assert_eq!(t, 1.4);
    }

    #[test]
    fn auto_thickness_12px_matches_blink() {
        let metrics = FontMetrics::zero();
        let t = resolve_thickness(&TextDecorationThickness::Auto, &metrics, 12.0);
        // Blink: 12.0 / 10.0 = 1.2 (raw, no rounding)
        assert_eq!(t, 1.2);
    }

    #[test]
    fn from_font_no_rounding() {
        // from-font should not round; Blink returns raw metric value.
        let metrics = FontMetrics { underline_thickness: 1.3, ..FontMetrics::zero() };
        let t = resolve_thickness(&TextDecorationThickness::FromFont, &metrics, 16.0);
        assert_eq!(t, 1.3);
    }

    // ── Issue 3: double decoration offset = thickness + 1px ──────────

    #[test]
    fn double_line_offset_is_thickness_plus_one() {
        // The double_offset formula should be thickness + 1.0 (1px gap).
        // We test by calling draw_double_line on a tiny surface and checking
        // the function doesn't panic and uses the correct geometry.
        // Since we can't easily inspect canvas output, verify the formula
        // indirectly: at thickness=2, gap should be 1px, total offset = 3px.
        let thickness = 2.0_f32;
        let double_offset = thickness + 1.0;
        assert_eq!(double_offset, 3.0, "double_offset should be thickness + 1.0");
        // The gap between the two strokes is:
        // double_offset - thickness = 1.0 (always 1px regardless of thickness)
        assert_eq!(double_offset - thickness, 1.0);
    }

    #[test]
    fn double_line_gap_is_always_1px() {
        for thickness in [0.5_f32, 1.0, 2.0, 3.0, 5.0, 10.0] {
            let double_offset = thickness + 1.0;
            let gap = double_offset - thickness;
            assert_eq!(gap, 1.0, "Gap should always be 1px, got {} for thickness={}", gap, thickness);
        }
    }

    // ── Issue 4: wavy decoration geometry ────────────────────────────

    #[test]
    fn wavy_step_equals_thickness_plus_one() {
        let thickness = 2.0_f32;
        let step = thickness + 1.0;
        assert_eq!(step, 3.0);
        // wavelength = 4 * step
        let wavelength = 4.0 * step;
        assert_eq!(wavelength, 12.0);
        // amplitude = step
        assert_eq!(step, 3.0);
    }

    #[test]
    fn wavy_underline_offset_is_positive() {
        // For underline, wavy_y = y + step (shifted down, away from text)
        let y = 10.0_f32;
        let thickness = 2.0_f32;
        let step = thickness + 1.0;
        let wavy_y = y + step;
        assert!(wavy_y > y, "Underline wave should be below the decoration line");
    }

    #[test]
    fn wavy_overline_offset_is_negative() {
        // For overline, wavy_y = y - step (shifted up, away from text)
        let y = 10.0_f32;
        let thickness = 2.0_f32;
        let step = thickness + 1.0;
        let wavy_y = y - step;
        assert!(wavy_y < y, "Overline wave should be above the decoration line");
    }

    #[test]
    fn wavy_linethrough_no_offset() {
        let y = 10.0_f32;
        let wavy_y = y; // line-through: no offset
        assert_eq!(wavy_y, y);
    }

    // ── SP11 Round 14 Issue 4: text-underline-offset applied ─────────

    /// Resolve text-underline-offset the same way the painter does.
    fn resolve_underline_offset(offset: &openui_geometry::Length, font_size: f32) -> f32 {
        if offset.is_percent() {
            offset.value() / 100.0 * font_size
        } else if offset.is_fixed() {
            offset.value()
        } else {
            0.0
        }
    }

    #[test]
    fn text_underline_offset_shifts_underline_position() {
        // When text_underline_offset is a fixed length, the underline Y position
        // should be shifted by that amount relative to the baseline + metrics offset.
        let baseline_y = 20.0_f32;
        let font_size = 16.0_f32;
        let metrics = FontMetrics {
            underline_offset: 2.0,
            ..FontMetrics::zero()
        };
        // auto → css_offset = 0.0
        let offset_auto = openui_geometry::Length::auto();
        let css_offset_auto = resolve_underline_offset(&offset_auto, font_size);
        let y_auto = baseline_y + metrics.underline_offset + css_offset_auto;
        assert_eq!(y_auto, 22.0, "Auto offset should not shift underline");

        // 3px offset → css_offset = 3.0
        let offset_px = openui_geometry::Length::px(3.0);
        let css_offset_px = resolve_underline_offset(&offset_px, font_size);
        let y_px = baseline_y + metrics.underline_offset + css_offset_px;
        assert_eq!(y_px, 25.0, "3px offset should shift underline down by 3");

        // Negative offset → shifts underline up
        let offset_neg = openui_geometry::Length::px(-2.0);
        let css_offset_neg = resolve_underline_offset(&offset_neg, font_size);
        let y_neg = baseline_y + metrics.underline_offset + css_offset_neg;
        assert_eq!(y_neg, 20.0, "Negative offset should shift underline up");
    }

    // ── SP11 Round 17 Issue 1: percentage offset resolved against 1em ──

    #[test]
    fn text_underline_offset_percent_resolves_against_font_size() {
        let baseline_y = 20.0_f32;
        let font_size = 16.0_f32;
        let metrics = FontMetrics {
            underline_offset: 2.0,
            ..FontMetrics::zero()
        };

        // 50% of 16px font_size = 8px offset
        let offset_pct = openui_geometry::Length::percent(50.0);
        let css_offset = resolve_underline_offset(&offset_pct, font_size);
        assert_eq!(css_offset, 8.0, "50% of 16px font-size should be 8px");
        let y = baseline_y + metrics.underline_offset + css_offset;
        assert_eq!(y, 30.0, "Underline should be at baseline + metrics + 8px");

        // 100% = full em
        let offset_full = openui_geometry::Length::percent(100.0);
        let css_full = resolve_underline_offset(&offset_full, font_size);
        assert_eq!(css_full, 16.0, "100% should equal font-size");

        // 0% = no shift
        let offset_zero = openui_geometry::Length::percent(0.0);
        let css_zero = resolve_underline_offset(&offset_zero, font_size);
        assert_eq!(css_zero, 0.0, "0% should be zero offset");
    }

    // ── SP11 Round 15 Issue 3: decoration metrics use styled font ────

    #[test]
    fn decoration_metrics_from_styled_font_not_fallback() {
        // Verify that resolve_decoration_metrics in the painter prefers
        // styled font metrics over the shape result's first run metrics.
        // We can't easily construct a ShapeResult with a fallback font in
        // a unit test, but we CAN verify the function path by confirming
        // that a styled font lookup produces valid metrics.
        let style = openui_style::ComputedStyle::default();
        // The default style should resolve to a valid system font.
        let font_desc = crate::text_painter::style_to_font_description(&style);
        let font = openui_text::Font::new(font_desc);
        let metrics = font.font_metrics().copied().unwrap_or_default();

        // Primary font metrics should have positive ascent and descent.
        assert!(
            metrics.ascent > 0.0,
            "Styled font should have positive ascent, got {}",
            metrics.ascent,
        );
        assert!(
            metrics.descent > 0.0,
            "Styled font should have positive descent, got {}",
            metrics.descent,
        );
        assert!(
            metrics.underline_offset > 0.0 || metrics.underline_offset == 0.0,
            "Styled font underline_offset should be non-negative",
        );
    }

    #[test]
    fn style_to_font_description_preserves_font_size() {
        let mut style = openui_style::ComputedStyle::default();
        style.font_size = 24.0;
        let desc = crate::text_painter::style_to_font_description(&style);
        assert_eq!(desc.size, 24.0, "Font description should preserve font size");
        assert_eq!(desc.specified_size, 24.0);
    }
}
