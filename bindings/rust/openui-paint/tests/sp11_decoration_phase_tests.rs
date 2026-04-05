//! Tests for SP11 Round 2 re-review fix 6: line-through paint order.
//!
//! CSS spec requires line-through to be painted AFTER text glyphs (in front),
//! while underline and overline are painted BEFORE text glyphs (behind).

use std::sync::Arc;

use openui_paint::decoration_painter::{paint_text_decorations, DecorationPhase};
use openui_paint::text_painter::metrics_from_shape_result;
use openui_style::*;
use openui_text::{Font, FontDescription, TextDirection, TextShaper};

fn make_surface(width: i32, height: i32) -> skia_safe::Surface {
    let mut surface = skia_safe::surfaces::raster_n32_premul((width, height)).unwrap();
    surface.canvas().clear(skia_safe::Color::WHITE);
    surface
}

fn has_non_white_pixels(surface: &mut skia_safe::Surface) -> bool {
    let img = surface.image_snapshot();
    let pm = img.peek_pixels().unwrap();
    let data = pm.bytes().unwrap();
    data.chunks(4).any(|px| px[0] != 255 || px[1] != 255 || px[2] != 255)
}

fn shape_text(text: &str) -> openui_text::ShapeResult {
    let font = Font::new(FontDescription::new());
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Ltr)
}

// ═══════════════════════════════════════════════════════════════════════
// ── DECORATION PHASE TESTS ─────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn before_text_phase_paints_underline() {
    let sr = Arc::new(shape_text("Test"));
    let metrics = metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;

    paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        DecorationPhase::BeforeText,
        None,
    );

    assert!(
        has_non_white_pixels(&mut surface),
        "BeforeText phase should paint underline"
    );
}

#[test]
fn after_text_phase_paints_line_through() {
    let sr = Arc::new(shape_text("Test"));
    let metrics = metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::LINE_THROUGH;

    paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        DecorationPhase::AfterText,
        None,
    );

    assert!(
        has_non_white_pixels(&mut surface),
        "AfterText phase should paint line-through"
    );
}

#[test]
fn before_text_phase_does_not_paint_line_through() {
    let sr = Arc::new(shape_text("Test"));
    let metrics = metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::LINE_THROUGH;

    paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        DecorationPhase::BeforeText,
        None,
    );

    assert!(
        !has_non_white_pixels(&mut surface),
        "BeforeText phase should NOT paint line-through"
    );
}
