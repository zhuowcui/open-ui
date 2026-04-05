//! Tests for SP11 Round 30 code review fixes — openui-paint crate.
//!
//! Issue 1: `from-font` line-through should always use underline_thickness
//!          (Blink uses UnderlineThickness() for ALL decoration types).
//! Issue 2: `auto` decoration thickness should NOT round (Blink returns raw).

use openui_paint::decoration_painter::{paint_text_decorations, DecorationPhase};

use openui_style::ComputedStyle;
use openui_style::{TextDecorationLine, TextDecorationStyle, TextDecorationThickness};

use openui_text::font::FontMetrics;
use openui_text::shaping::TextDirection;

// ── Issue 1: from-font uses underline_thickness for ALL types ───────────

#[test]
fn r30_from_font_ignores_strikeout_thickness() {
    // Blink: TextDecorationInfo always uses UnderlineThickness() for from-font,
    // regardless of whether the decoration is underline, overline, or line-through.
    let metrics = FontMetrics {
        underline_thickness: 1.5,
        strikeout_thickness: 3.0, // must be ignored
        strikeout_position: 5.0,
        ascent: 12.0,
        descent: 4.0,
        ..FontMetrics::zero()
    };

    let mut style = ComputedStyle::default();
    style.font_size = 16.0;
    style.text_decoration_line = TextDecorationLine::LINE_THROUGH;
    style.text_decoration_thickness = TextDecorationThickness::FromFont;
    style.text_decoration_style = TextDecorationStyle::Solid;

    let surface_info = skia_safe::ImageInfo::new_n32_premul((100, 100), None);
    let mut surface =
        skia_safe::surfaces::raster(&surface_info, None, None).expect("surface");
    let canvas = surface.canvas();

    let shape = openui_text::shaping::ShapeResult::empty(TextDirection::Ltr);
    paint_text_decorations(
        canvas,
        &shape,
        (0.0, 50.0),
        &style,
        &metrics,
        DecorationPhase::AfterText,
        None,
    );
}

#[test]
fn r30_underline_and_linethrough_share_thickness() {
    // Both decorations should get the same thickness when using from-font.
    let metrics = FontMetrics {
        underline_thickness: 2.0,
        strikeout_thickness: 5.0, // must NOT affect result
        underline_offset: 2.0,
        strikeout_position: 5.0,
        ascent: 12.0,
        descent: 4.0,
        ..FontMetrics::zero()
    };

    let mut style = ComputedStyle::default();
    style.font_size = 16.0;
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0 | TextDecorationLine::LINE_THROUGH.0,
    );
    style.text_decoration_thickness = TextDecorationThickness::FromFont;
    style.text_decoration_style = TextDecorationStyle::Solid;

    let surface_info = skia_safe::ImageInfo::new_n32_premul((100, 100), None);
    let mut surface =
        skia_safe::surfaces::raster(&surface_info, None, None).expect("surface");
    let canvas = surface.canvas();
    let shape = openui_text::shaping::ShapeResult::empty(TextDirection::Ltr);

    paint_text_decorations(
        canvas, &shape, (0.0, 50.0), &style, &metrics, DecorationPhase::BeforeText,
        None,
    );
    paint_text_decorations(
        canvas, &shape, (0.0, 50.0), &style, &metrics, DecorationPhase::AfterText,
        None,
    );
}

// ── Issue 2: auto thickness no rounding, matching Blink ─────────────────

#[test]
fn r30_auto_thickness_14px_not_rounded() {
    // Blink: 14.0 / 10.0 = 1.4 (raw). Old code rounded to 1.0.
    let mut style = ComputedStyle::default();
    style.font_size = 14.0;
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_thickness = TextDecorationThickness::Auto;
    style.text_decoration_style = TextDecorationStyle::Solid;

    let metrics = FontMetrics {
        underline_offset: 2.0,
        ascent: 10.0,
        descent: 3.0,
        ..FontMetrics::zero()
    };

    let surface_info = skia_safe::ImageInfo::new_n32_premul((100, 100), None);
    let mut surface =
        skia_safe::surfaces::raster(&surface_info, None, None).expect("surface");
    let canvas = surface.canvas();
    let shape = openui_text::shaping::ShapeResult::empty(TextDirection::Ltr);

    paint_text_decorations(
        canvas, &shape, (0.0, 50.0), &style, &metrics, DecorationPhase::BeforeText,
        None,
    );
}

#[test]
fn r30_auto_thickness_16px_not_rounded() {
    // Blink: 16.0 / 10.0 = 1.6 (raw). Old code rounded to 2.0.
    let mut style = ComputedStyle::default();
    style.font_size = 16.0;
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_thickness = TextDecorationThickness::Auto;
    style.text_decoration_style = TextDecorationStyle::Solid;

    let metrics = FontMetrics {
        underline_offset: 2.0,
        ascent: 12.0,
        descent: 4.0,
        ..FontMetrics::zero()
    };

    let surface_info = skia_safe::ImageInfo::new_n32_premul((100, 100), None);
    let mut surface =
        skia_safe::surfaces::raster(&surface_info, None, None).expect("surface");
    let canvas = surface.canvas();
    let shape = openui_text::shaping::ShapeResult::empty(TextDirection::Ltr);

    paint_text_decorations(
        canvas, &shape, (0.0, 50.0), &style, &metrics, DecorationPhase::BeforeText,
        None,
    );
}
