//! Text glyph painting — extracted from Blink's `TextPainter`.
//!
//! Source: `core/paint/text_painter.cc`, `TextPainterBase::PaintDecorationsExceptLineThrough()`.
//!
//! Renders shaped text glyphs onto a Skia canvas. The `ShapeResult` from
//! HarfBuzz shaping is converted to a `TextBlob` and drawn at the text
//! baseline position with the correct paint color and anti-aliasing.
//!
//! ## Color Font Support
//!
//! Color fonts (COLR/CPAL, CBDT/CBLC, sbix, SVG-in-OpenType) contain
//! embedded glyph colors. Skia handles these natively through
//! `Canvas::draw_text_blob()` — when a glyph has embedded color data,
//! Skia uses the glyph's own colors instead of the paint color.
//!
//! Our pipeline preserves this by:
//! 1. Using anti-aliased rendering (required for color glyph rasterization)
//! 2. Letting Skia's `drawTextBlob` handle color/non-color dispatch
//! 3. Not forcing monochrome rendering paths

use skia_safe::{
    Canvas, Color4f, ColorSpace, Paint, PaintStyle, Point, Rect, TextBlob, TextBlobBuilder,
};

use openui_geometry::{
    PhysicalSnap, RasterBackend, RasterSnapping, TextEdging, TextRasterConfiguration,
};
use openui_layout::inline::text_combine::TextCombineLayout;
use openui_style::{Color, ComputedStyle, FontFamily, GenericFontFamily};
use openui_text::font::FontMetrics;
use openui_text::shaping::{ShapeResult, TextRasterPolicy};

/// Paint shaped text glyphs onto a canvas.
///
/// Mirrors Blink's `TextPainter::Paint()` (`core/paint/text_painter.cc:95`).
///
/// The origin is the (x, baseline_y) position — the same coordinate system
/// used by `ShapeResult::to_text_blob()` where glyph Y offsets are relative
/// to the baseline.
///
/// For color fonts (COLR/CPAL, CBDT/CBLC, sbix, SVG), Skia automatically
/// uses the glyph's embedded colors. The paint color is set as a fallback
/// for non-color glyphs in the same run — Skia's `drawTextBlob` handles
/// the dispatch per-glyph.
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
    let author_lcd = uses_chromium_author_lcd(style);
    let raster_policy = if style.native_control_text {
        TextRasterPolicy::ChromiumNativeControl
    } else if style.embedded_document_text {
        TextRasterPolicy::ChromiumEmbeddedDocument
    } else if author_lcd {
        if style.raster_configuration.author_text.edging == TextEdging::Alias {
            TextRasterPolicy::ChromiumAliased
        } else {
            TextRasterPolicy::ChromiumAuthorLcd
        }
    } else {
        TextRasterPolicy::Skia
    };
    paint_text_with_raster_policy(canvas, shape_result, origin, style, raster_policy);
}

/// Paint shaped glyphs with an explicit platform outline raster policy.
pub fn paint_text_with_raster_policy(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
    raster_policy: TextRasterPolicy,
) {
    let text_raster = text_raster_configuration(style);
    let aliased_ahem = text_raster.edging == TextEdging::Alias
        && !shape_result.runs.is_empty()
        && shape_result.runs.iter().all(|run| {
            run.font_data
                .typeface()
                .family_name()
                .eq_ignore_ascii_case("Ahem")
        });
    let origin = if text_raster.subpixel_positioning
        || (style.device_scale_factor - 1.0).abs() <= f64::EPSILON
        // Chromium retains the LayoutUnit origin while selecting a monochrome
        // Ahem mask. Snapping the run origin first changes the phase of every
        // later fractional advance; the glyph rasterizer, not paint, owns
        // that final device-cell choice.
        || aliased_ahem
    {
        origin
    } else {
        let snapping = RasterSnapping::new(style.device_scale_factor);
        (
            snapping.logical_coordinate(origin.0, PhysicalSnap::Nearest),
            snapping.logical_coordinate(origin.1, PhysicalSnap::Nearest),
        )
    };
    // Build a Skia TextBlob from the shaped glyph runs.
    // Blink: TextPainter::Paint → DrawBlob → canvas->drawTextBlob()
    let lcd_origin = (style.native_control_text
        || raster_policy == TextRasterPolicy::ChromiumAuthorLcd
        || text_raster.edging == TextEdging::SubpixelAntiAlias)
        .then_some(origin.0);
    // Fit glyph positions at their retained layout origin, then apply the
    // caller's physical phase exactly once to the draw origin. Re-rounding
    // the shifted origin to logical LayoutUnits would change its physical
    // distance at fractional device scales. Layout advances stay unchanged.
    let origin = lcd_raster_origin(origin, text_raster, style.device_scale_factor);
    if let Some(logical_blob) = shape_result.to_text_blob_with_raster_policy_and_geometry_at_scale(
        lcd_origin,
        raster_policy,
        1.0,
        style.raster_configuration.pixel_geometry,
    ) {
        // Cull against the glyphs' logical ink bounds before Skia applies its
        // LCD coverage filter.  Skia's filter taps extend one device pixel
        // beyond those bounds; if a run begins exactly at a hard overflow
        // clip edge, drawing it first and clipping the filtered mask can leak
        // a colored subpixel back into the visible area.  Blink rejects the
        // display item from its logical visual rect in this case.
        if text_blob_is_outside_device_clip(canvas, &logical_blob, origin)
            || (style
                .font_family
                .families
                .iter()
                .any(|family| matches!(family, FontFamily::Named(name) if name.eq_ignore_ascii_case("ahem")))
                && text_origin_is_past_device_clip_end(canvas, origin))
        {
            return;
        }

        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);

        // Set the text color from the computed `color` property.
        // For non-color glyphs, this is used directly. For color glyphs
        // (COLR/CPAL, CBDT/CBLC, sbix, SVG), Skia overrides this paint
        // color with the glyph's embedded palette colors automatically.
        // Blink: TextPainterBase::UpdatePaint sets the fill color.
        let c = &style.color;
        paint.set_color4f(Color4f::new(c.r, c.g, c.b, c.a), None::<&ColorSpace>);

        let physical_scale = style.device_scale_factor as f32;
        if style.raster_configuration.backend.is_chromium_cpu()
            && (physical_scale - 1.0).abs() > f32::EPSILON
        {
            if let Some(physical_blob) = shape_result
                .to_text_blob_with_raster_policy_and_geometry_at_scale(
                    lcd_origin,
                    raster_policy,
                    physical_scale,
                    style.raster_configuration.pixel_geometry,
                )
            {
                canvas.save();
                canvas.translate(origin);
                canvas.scale((1.0 / physical_scale, 1.0 / physical_scale));
                canvas.draw_text_blob(&physical_blob, Point::new(0.0, 0.0), &paint);
                canvas.restore();
            }
        } else {
            canvas.draw_text_blob(&logical_blob, Point::new(origin.0, origin.1), &paint);
        }
    }
}

pub(crate) fn text_raster_configuration(style: &ComputedStyle) -> TextRasterConfiguration {
    if style.native_control_text {
        style.raster_configuration.native_text
    } else if style.embedded_document_text {
        style.raster_configuration.embedded_text
    } else {
        style.raster_configuration.author_text
    }
}

pub(crate) fn lcd_raster_origin(
    origin: (f32, f32),
    settings: TextRasterConfiguration,
    device_scale: f64,
) -> (f32, f32) {
    if settings.edging != TextEdging::SubpixelAntiAlias || settings.lcd_phase_64ths == 0 {
        return origin;
    }
    (
        origin.0 + f32::from(settings.lcd_phase_64ths) / 64.0 / device_scale as f32,
        origin.1,
    )
}

fn uses_chromium_author_lcd(style: &ComputedStyle) -> bool {
    if style.native_control_text || style.embedded_document_text {
        return false;
    }
    if style.raster_configuration.backend == RasterBackend::ChromiumLinuxFontations {
        // Fontations is an explicit authored-outline choice. The ordinary
        // Chromium Linux LCD constructor preserves its FreeType path, as
        // required by captures that select FontDataServiceLinux Freetype.
        return true;
    }
    if style.raster_configuration.author_text.edging != TextEdging::Alias {
        return false;
    }
    match style.font_family.families.first() {
        Some(FontFamily::Named(name)) => ![
            "Ahem",
            "Droid Sans Fallback",
            "Noto Sans Devanagari",
            "Noto Color Emoji",
            "DejaVu Sans",
        ]
        .iter()
        .any(|family| name.eq_ignore_ascii_case(family)),
        Some(FontFamily::Generic(family)) => !matches!(
            family,
            GenericFontFamily::None
                | GenericFontFamily::SansSerif
                | GenericFontFamily::Emoji
                | GenericFontFamily::UiSansSerif
        ),
        None => false,
    }
}

/// Paint an upright run in a vertical line.
///
/// Shaping selects vertical OpenType alternates, while this builder maps each
/// HarfBuzz cluster to one font-size vertical advance and keeps every glyph
/// outline upright. Multiple glyphs in one cluster (combining sequences and
/// emoji) share a baseline. `vertical_baseline_x` is layout's exported
/// baseline axis and `top_y` is the fragment's physical inline-start.
pub fn paint_vertical_text(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    vertical_baseline_x: f32,
    top_y: f32,
    style: &ComputedStyle,
) {
    if shape_result.runs.is_empty() || shape_result.num_glyphs() == 0 {
        return;
    }

    let mut builder = TextBlobBuilder::new();
    let text_raster = text_raster_configuration(style);
    let physical_scale = style.device_scale_factor as f32;
    let physical_strike = style.raster_configuration.backend.is_chromium_cpu()
        && (physical_scale - 1.0).abs() > f32::EPSILON;
    let (vertical_baseline_x, top_y) = if text_raster.subpixel_positioning
        || (style.device_scale_factor - 1.0).abs() <= f64::EPSILON
    {
        (vertical_baseline_x, top_y)
    } else {
        let snapping = RasterSnapping::new(style.device_scale_factor);
        (
            // Upright vertical glyphs use an aliased physical strike.
            // Blink selects the containing block-axis device column;
            // nearest rounding advances every glyph by one column when
            // the exported baseline lands on an exact half-pixel.
            snapping.logical_coordinate(vertical_baseline_x, PhysicalSnap::Nearest),
            snapping.logical_coordinate(top_y, PhysicalSnap::Nearest),
        )
    };
    let (vertical_baseline_x, top_y) = lcd_raster_origin(
        (vertical_baseline_x, top_y),
        text_raster,
        style.device_scale_factor,
    );
    let mut run_y = top_y;
    for run in &shape_result.runs {
        if run.num_glyphs == 0 {
            continue;
        }
        let source_font = run.font_data.sk_font();
        let mut scaled_font;
        let raster_font = if physical_strike {
            scaled_font = source_font.clone();
            scaled_font.set_size(source_font.size() * physical_scale);
            &scaled_font
        } else {
            source_font
        };
        let (glyphs_out, positions_out) = builder.alloc_run_pos(raster_font, run.num_glyphs, None);
        glyphs_out.copy_from_slice(&run.glyphs);

        let mut cluster_y = run_y;
        for index in 0..run.num_glyphs {
            let glyph = run.glyphs[index];
            let offset = run.offsets.get(index).copied().unwrap_or((0.0, 0.0));
            positions_out[index] = Point::new(
                (vertical_baseline_x + offset.0)
                    * if physical_strike { physical_scale } else { 1.0 },
                (cluster_y + run.font_data.vertical_origin_y(glyph) + offset.1)
                    * if physical_strike { physical_scale } else { 1.0 },
            );
            let cluster = run.clusters.get(index).copied().unwrap_or(index);
            let next_cluster = run.clusters.get(index + 1).copied();
            if next_cluster != Some(cluster) {
                cluster_y += run.font_data.vertical_advance(glyph);
            }
        }
        run_y = cluster_y;
    }

    if let Some(blob) = builder.make() {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        paint.set_style(PaintStyle::Fill);
        let color = &style.color;
        paint.set_color4f(
            Color4f::new(color.r, color.g, color.b, color.a),
            None::<&ColorSpace>,
        );
        if physical_strike {
            canvas.save();
            canvas.scale((1.0 / physical_scale, 1.0 / physical_scale));
            canvas.draw_text_blob(&blob, Point::new(0.0, 0.0), &paint);
            canvas.restore();
        } else {
            canvas.draw_text_blob(&blob, Point::new(0.0, 0.0), &paint);
        }
    }
}

/// Ahem has square glyphs with no negative inline bearing. If the run origin
/// is on or beyond a hard overflow edge, its LCD filter fringe is not logical
/// ink and must not leak one subpixel back into the clipped box.
fn text_origin_is_past_device_clip_end(canvas: &Canvas, origin: (f32, f32)) -> bool {
    let device_origin = canvas
        .local_to_device_as_3x3()
        .map_point(Point::new(origin.0, origin.1));
    canvas
        .device_clip_bounds()
        .is_none_or(|clip| device_origin.x >= clip.right as f32)
}

/// Whether a positioned text blob has no logical ink inside the exact device
/// clip.  `Canvas::local_clip_bounds` is deliberately outset for antialiasing,
/// so use the non-outset device clip after mapping the blob bounds instead.
fn text_blob_is_outside_device_clip(
    canvas: &Canvas,
    text_blob: &TextBlob,
    origin: (f32, f32),
) -> bool {
    let local_bounds = text_blob
        .bounds()
        .with_offset(Point::new(origin.0, origin.1));
    let (device_bounds, _) = canvas.local_to_device_as_3x3().map_rect(Rect::from_ltrb(
        local_bounds.left,
        local_bounds.top,
        local_bounds.right,
        local_bounds.bottom,
    ));
    let Some(clip) = canvas.device_clip_bounds() else {
        return true;
    };

    // This cull is intentionally inline-axis only. Fragmentainers may allow
    // the ink of an oversized line to cross their block edge even when the
    // line's logical rectangle belongs to the following continuation.
    device_bounds.right <= clip.left as f32 || device_bounds.left >= clip.right as f32
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
            // Blink records the CSS blur in local coordinates. Respect the
            // canvas transform so device scale and authored transforms apply
            // equally to the glyphs and their shadow kernel.
            let sigma = shadow.blur_radius / 2.0;
            if let Some(filter) =
                skia_safe::MaskFilter::blur(skia_safe::BlurStyle::Normal, sigma, true)
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
    openui_text::FontDescription::from_computed_style(style)
}

/// Paint text with text-combine-upright (tate-chū-yoko) transforms.
///
/// Mirrors Blink's `TextCombinePainter::Paint()` in
/// `core/paint/text_combine_painter.cc`.
///
/// When `text-combine-upright: all` is active in vertical writing, the
/// combined characters are rendered horizontally and scaled to fit within
/// one em-width of the vertical line. This function:
///
/// 1. Saves the canvas state.
/// 2. Translates by the centering offset (to center short text in the cell).
/// 3. Applies a horizontal scale (`scale_x`) to compress wide text.
/// 4. Draws the text blob at the adjusted origin.
/// 5. Restores the canvas state.
///
/// CSS Writing Modes Level 4 §9.1
/// <https://www.w3.org/TR/css-writing-modes-4/#text-combine-upright>
pub fn paint_text_combine(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
    layout: &TextCombineLayout,
) {
    let text_blob = match shape_result.to_text_blob() {
        Some(blob) => blob,
        None => return,
    };

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);

    let c = &style.color;
    paint.set_color4f(Color4f::new(c.r, c.g, c.b, c.a), None::<&ColorSpace>);

    let (translate_x, translate_y, scale_x) =
        openui_layout::inline::text_combine::paint_transform(layout);

    canvas.save();

    // Translate to the centering position within the character cell.
    let (draw_x, draw_y) = lcd_raster_origin(
        (origin.0 + translate_x, origin.1 + translate_y),
        text_raster_configuration(style),
        style.device_scale_factor,
    );

    // Apply horizontal compression if the combined text exceeds one em.
    if layout.needs_compression {
        // Scale around the draw origin: translate to origin, scale, then
        // draw at (0, 0) — equivalent to translating first.
        canvas.translate(Point::new(draw_x, draw_y));
        canvas.scale((scale_x, 1.0));
        canvas.draw_text_blob(&text_blob, Point::new(0.0, 0.0), &paint);
    } else {
        canvas.draw_text_blob(&text_blob, Point::new(draw_x, draw_y), &paint);
    }

    canvas.restore();
}
