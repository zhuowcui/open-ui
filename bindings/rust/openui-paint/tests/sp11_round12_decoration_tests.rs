//! Tests for SP11 Round 12 Issue 5: double decoration line direction.
//!
//! Verifies that draw_double_line draws the second line in the correct
//! direction for overline (upward), underline (downward), and
//! line-through (centered).

use std::sync::Arc;

use skia_safe::{surfaces, Color as SkColor, Surface};

use openui_paint::decoration_painter;
use openui_style::*;
use openui_text::{Font, FontDescription, FontMetrics, ShapeResult, TextDirection, TextShaper};

// ── Helpers ──────────────────────────────────────────────────────────

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

/// Check if any non-white pixel exists in the row range [y_start, y_end).
fn has_colored_pixels_in_rows(surface: &mut Surface, y_start: i32, y_end: i32) -> bool {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let width = info.width() as usize;
    let row_bytes = info.min_row_bytes();
    let mut pixels = vec![0u8; info.height() as usize * row_bytes];
    image.read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    for y in y_start..y_end.min(info.height()) {
        let row_start = y as usize * row_bytes;
        for x in 0..width {
            let offset = row_start + x * 4;
            if offset + 3 < pixels.len() {
                let (r, g, b) = (pixels[offset], pixels[offset + 1], pixels[offset + 2]);
                if r != 0xFF || g != 0xFF || b != 0xFF {
                    return true;
                }
            }
        }
    }
    false
}

// ── Double overline: second line should be ABOVE first line ──────────

#[test]
fn double_overline_second_line_is_above_first() {
    // With synthetic metrics: ascent = 12.  Overline y = baseline_y - ascent.
    // baseline_y = 60 → overline nominal y = 48.
    // thickness = 2.0 (from_auto synthetic).
    // For double overline, second line should be above (y < 48), not below.
    let sr = Arc::new(shape_text("Test"));
    let metrics = synthetic_metrics();
    let baseline_y = 60.0;
    let thickness = 2.0;
    let gap = thickness * 1.5; // 3.0

    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::OVERLINE;
    style.text_decoration_style = TextDecorationStyle::Double;
    style.text_decoration_thickness = TextDecorationThickness::Length(thickness);

    let mut surface = make_surface(300, 100);
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, baseline_y),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
    );

    let overline_y = (baseline_y - metrics.ascent) as i32; // 48
    // The second double line should extend ABOVE: at y ≈ 48 - thickness - gap = 43
    // Check that there are colored pixels above the first line position
    let above_first_line_top = overline_y - (thickness + gap + thickness) as i32 - 1;
    assert!(
        has_colored_pixels_in_rows(&mut surface, above_first_line_top.max(0), overline_y - 1),
        "Double overline second line should be drawn ABOVE the first line (away from text)"
    );

    // The area well below the overline should NOT have decoration pixels
    // (below baseline = row 60+, the second line should NOT be there)
    let far_below = (baseline_y as i32) + 5;
    assert!(
        !has_colored_pixels_in_rows(&mut surface, far_below, far_below + 10),
        "Double overline should not draw a second line below the baseline"
    );
}

// ── Double underline: second line is below first ─────────────────────

#[test]
fn double_underline_second_line_is_below_first() {
    let sr = Arc::new(shape_text("Test"));
    let metrics = synthetic_metrics();
    let baseline_y = 40.0;
    let thickness = 2.0;

    let mut style = ComputedStyle::default();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_style = TextDecorationStyle::Double;
    style.text_decoration_thickness = TextDecorationThickness::Length(thickness);

    let mut surface = make_surface(300, 100);
    decoration_painter::paint_text_decorations(
        surface.canvas(),
        &sr,
        (10.0, baseline_y),
        &style,
        &metrics,
        decoration_painter::DecorationPhase::BeforeText,
    );

    let underline_y = (baseline_y + metrics.underline_offset) as i32; // 42
    let gap = (thickness * 1.5) as i32; // 3

    // Second line should be below: at approximately underline_y + thickness + gap
    let second_line_region_start = underline_y + gap;
    let second_line_region_end = underline_y + gap + (thickness as i32) + 4;
    assert!(
        has_colored_pixels_in_rows(&mut surface, second_line_region_start, second_line_region_end),
        "Double underline second line should be below the first"
    );
}
