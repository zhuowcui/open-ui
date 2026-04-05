//! Text glyph painting — extracted from Blink's `TextPainter`.
//!
//! Source: `core/paint/text_painter.cc`, `TextPainterBase::PaintDecorationsExceptLineThrough()`.
//!
//! Renders shaped text glyphs onto a Skia canvas. The `ShapeResult` from
//! HarfBuzz shaping is converted to a `TextBlob` and drawn at the text
//! baseline position with the correct paint color and anti-aliasing.

use skia_safe::{Canvas, Color4f, ColorSpace, Paint, PaintStyle, Point};

use openui_style::{Color, ComputedStyle};
use openui_text::shaping::ShapeResult;
use openui_text::font::FontMetrics;

/// Paint shaped text glyphs onto a canvas.
///
/// Mirrors Blink's `TextPainter::Paint()` (`core/paint/text_painter.cc:95`).
///
/// The origin is the (x, baseline_y) position — the same coordinate system
/// used by `ShapeResult::to_text_blob()` where glyph Y offsets are relative
/// to the baseline.
///
/// # Arguments
/// * `canvas` — Skia raster canvas
/// * `shape_result` — Shaped text containing glyph runs
/// * `origin` — (x, baseline_y) in device pixels
/// * `style` — Computed style for text color
pub fn paint_text(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
) {
    // Build a Skia TextBlob from the shaped glyph runs.
    // Blink: TextPainter::Paint → DrawBlob → canvas->drawTextBlob()
    if let Some(text_blob) = shape_result.to_text_blob() {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);

        // Text color from the computed `color` property.
        // Blink: TextPainterBase::UpdatePaint sets the fill color.
        let c = &style.color;
        paint.set_color4f(Color4f::new(c.r, c.g, c.b, c.a), None::<&ColorSpace>);

        canvas.draw_text_blob(&text_blob, Point::new(origin.0, origin.1), &paint);
    }
}

/// Paint text shadows behind text glyphs.
///
/// Mirrors Blink's `TextPainterBase::PaintShadow()` which draws the text
/// blob multiple times, once per shadow layer, offset and blurred.
///
/// Blink applies shadows in reverse order (last declared = painted first,
/// i.e. closest to the text is the first in the list).
pub fn paint_text_shadows(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
) {
    if style.text_shadow.is_empty() {
        return;
    }

    let text_blob = match shape_result.to_text_blob() {
        Some(blob) => blob,
        None => return,
    };

    // Paint shadows in reverse order — last declared shadow is closest to
    // the text (painted first), matching CSS painting order.
    for shadow in style.text_shadow.iter().rev() {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);

        let sc = &shadow.color;
        paint.set_color4f(Color4f::new(sc.r, sc.g, sc.b, sc.a), None::<&ColorSpace>);

        // Apply blur via Skia's MaskFilter.
        // Blink: ApplyShadowBlurToFlags → SkMaskFilter::MakeBlur(kNormal_SkBlurStyle, sigma)
        if shadow.blur_radius > 0.0 {
            // Convert CSS blur radius to Skia sigma: sigma = blur_radius / 2.0
            // Blink: style/filter_operations.h uses this conversion.
            let sigma = shadow.blur_radius / 2.0;
            if let Some(filter) =
                skia_safe::MaskFilter::blur(skia_safe::BlurStyle::Normal, sigma, false)
            {
                paint.set_mask_filter(filter);
            }
        }

        let shadow_x = origin.0 + shadow.offset_x;
        let shadow_y = origin.1 + shadow.offset_y;
        canvas.draw_text_blob(&text_blob, Point::new(shadow_x, shadow_y), &paint);
    }
}

/// Resolve the font metrics for the primary font of a shape result.
///
/// The first run's font data provides the metrics. Falls back to zero
/// metrics if no runs are present (empty text).
pub fn metrics_from_shape_result(shape_result: &ShapeResult) -> FontMetrics {
    shape_result
        .runs
        .first()
        .map(|run| *run.font_data.metrics())
        .unwrap_or_default()
}

/// Convert an `openui_style::Color` to a `skia_safe::Color4f`.
#[inline]
pub(crate) fn to_sk_color4f(color: &Color) -> Color4f {
    Color4f::new(color.r, color.g, color.b, color.a)
}

/// Convert a `ComputedStyle` to a `FontDescription` for resolving primary
/// font metrics. Mirrors `openui_layout::inline::items_builder::style_to_font_description`.
pub fn style_to_font_description(style: &ComputedStyle) -> openui_text::FontDescription {
    openui_text::FontDescription {
        family: style.font_family.clone(),
        size: style.font_size,
        specified_size: style.font_size,
        weight: style.font_weight,
        stretch: style.font_stretch,
        style: style.font_style,
        variant_caps: style.font_variant_caps,
        letter_spacing: style.letter_spacing,
        word_spacing: style.word_spacing,
        locale: style.locale.clone(),
        font_smoothing: style.font_smoothing,
        text_rendering: style.text_rendering,
        feature_settings: style.font_feature_settings.clone(),
        variation_settings: style.font_variation_settings.clone(),
        font_synthesis_weight: style.font_synthesis_weight,
        font_synthesis_style: style.font_synthesis_style,
        font_optical_sizing: style.font_optical_sizing,
        variant_ligatures: style.font_variant_ligatures,
        variant_numeric: style.font_variant_numeric,
        variant_east_asian: style.font_variant_east_asian,
        variant_position: style.font_variant_position,
        variant_alternates: style.font_variant_alternates,
        orientation: openui_style::font_orientation(style.writing_mode, style.text_orientation),
    }
}
