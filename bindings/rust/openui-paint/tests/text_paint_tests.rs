//! Text painting tests — verifies text glyph rendering, decoration painting,
//! and integration with the fragment painting pipeline.
//!
//! These tests use real system fonts (via Skia's `SkFontMgr`) to produce
//! pixel-level verification of the paint output.

use std::sync::Arc;

use skia_safe::{surfaces, Color as SkColor, Surface};

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::{Fragment, FragmentKind};
use openui_paint::{paint_fragment, text_painter, decoration_painter};
use openui_style::*;
use openui_text::{
    Font, FontDescription, ShapeResult, TextDirection, TextShaper,
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
    let mut surface = surfaces::raster_n32_premul((width, height))
        .expect("Failed to create Skia surface");
    surface.canvas().clear(SkColor::WHITE);
    surface
}

/// Check if any pixels in the surface differ from the background (white).
fn has_non_white_pixels(surface: &mut Surface) -> bool {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = info.min_row_bytes();
    let mut pixels = vec![0u8; info.height() as usize * row_bytes];
    image.read_pixels(&info, &mut pixels, row_bytes, (0, 0), skia_safe::image::CachingHint::Allow);

    for chunk in pixels.chunks(4) {
        // BGRA or RGBA — check if any pixel isn't white (0xFF)
        if chunk.len() == 4 && (chunk[0] != 0xFF || chunk[1] != 0xFF || chunk[2] != 0xFF) {
            return true;
        }
    }
    false
}

/// Create a default style with text decorations disabled.
fn default_style() -> ComputedStyle {
    ComputedStyle::default()
}

/// Create a text fragment with a shape result for testing.
fn make_text_fragment(
    node_id: openui_dom::NodeId,
    shape_result: &Arc<ShapeResult>,
    metrics: &openui_text::FontMetrics,
) -> Fragment {
    let width = shape_result.width();
    let height = metrics.ascent + metrics.descent;
    let mut frag = Fragment::new_text(
        node_id,
        PhysicalSize::new(
            LayoutUnit::from_f32(width),
            LayoutUnit::from_f32(height),
        ),
        Arc::clone(shape_result),
        String::new(),
    );
    frag.baseline_offset = metrics.ascent;
    frag
}

/// Create a minimal Document with one text node for testing.
fn make_doc_with_text_style(style_fn: impl FnOnce(&mut ComputedStyle)) -> (Document, openui_dom::NodeId) {
    let mut doc = Document::new();
    let vp = doc.root();
    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some("Hello".to_string());
    style_fn(&mut doc.node_mut(text_node).style);
    doc.append_child(vp, text_node);
    (doc, text_node)
}

// ═══════════════════════════════════════════════════════════════════════
// ── TEXT RENDERING TESTS ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn paint_text_produces_non_white_pixels() {
    let sr = Arc::new(shape_text("Hello World"));
    let _metrics = text_painter::metrics_from_shape_result(&sr);

    let mut surface = make_surface(400, 100);
    let style = default_style();
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &style);

    assert!(has_non_white_pixels(&mut surface), "Text should produce visible pixels");
}

#[test]
fn paint_text_empty_string_does_not_crash() {
    let sr = Arc::new(shape_text(""));
    let mut surface = make_surface(100, 100);
    let style = default_style();
    // Should not panic or produce visible output
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &style);
}

#[test]
fn paint_text_single_character() {
    let sr = Arc::new(shape_text("A"));
    let mut surface = make_surface(100, 100);
    let style = default_style();
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn different_font_sizes_produce_different_results() {
    // Shape the same text at two different sizes
    let sr_small = Arc::new(shape_text_with_size("Test", 10.0));
    let sr_large = Arc::new(shape_text_with_size("Test", 48.0));

    // Widths should differ significantly
    assert!(sr_large.width() > sr_small.width() * 1.5,
        "Larger font should produce wider text: large={}, small={}",
        sr_large.width(), sr_small.width());
}

#[test]
fn text_color_is_applied() {
    let sr = Arc::new(shape_text("X"));

    // Red text
    let mut surface_red = make_surface(100, 100);
    let mut style_red = default_style();
    style_red.color = Color::RED;
    text_painter::paint_text(surface_red.canvas(), &sr, (10.0, 50.0), &style_red);

    // Blue text
    let mut surface_blue = make_surface(100, 100);
    let mut style_blue = default_style();
    style_blue.color = Color::BLUE;
    text_painter::paint_text(surface_blue.canvas(), &sr, (10.0, 50.0), &style_blue);

    // Both should produce visible output
    assert!(has_non_white_pixels(&mut surface_red), "Red text should be visible");
    assert!(has_non_white_pixels(&mut surface_blue), "Blue text should be visible");
}

#[test]
fn transparent_text_produces_no_visible_output() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(200, 100);
    let mut style = default_style();
    style.color = Color::TRANSPARENT;
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(!has_non_white_pixels(&mut surface), "Transparent text should not be visible");
}

#[test]
fn white_text_on_white_background_is_invisible() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(200, 100);
    let mut style = default_style();
    style.color = Color::WHITE;
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(!has_non_white_pixels(&mut surface), "White text on white should not be visible");
}

#[test]
fn paint_text_with_default_style() {
    // Default style has black text — should be visible
    let sr = Arc::new(shape_text("Test"));
    let mut surface = make_surface(200, 100);
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &default_style());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn long_text_string_renders() {
    let sr = Arc::new(shape_text("The quick brown fox jumps over the lazy dog"));
    let mut surface = make_surface(800, 100);
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &default_style());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_at_different_positions() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(400, 400);
    let style = default_style();

    // Paint at multiple positions — should not panic
    text_painter::paint_text(surface.canvas(), &sr, (0.0, 50.0), &style);
    text_painter::paint_text(surface.canvas(), &sr, (100.0, 100.0), &style);
    text_painter::paint_text(surface.canvas(), &sr, (200.0, 200.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_shape_result_width_is_positive() {
    let sr = shape_text("Hello");
    assert!(sr.width() > 0.0, "Shaped text should have positive width: {}", sr.width());
}

#[test]
fn shape_result_to_text_blob_succeeds() {
    let sr = shape_text("Hello");
    assert!(sr.to_text_blob().is_some(), "to_text_blob should produce a blob");
}

#[test]
fn empty_shape_result_to_text_blob_returns_none() {
    let sr = ShapeResult::empty(TextDirection::Ltr);
    assert!(sr.to_text_blob().is_none(), "Empty shape result should not produce a blob");
}

#[test]
fn metrics_from_shape_result_returns_valid_metrics() {
    let sr = shape_text("Hello");
    let metrics = text_painter::metrics_from_shape_result(&sr);
    assert!(metrics.ascent > 0.0, "Ascent should be positive: {}", metrics.ascent);
    assert!(metrics.descent > 0.0, "Descent should be positive: {}", metrics.descent);
}

#[test]
fn metrics_from_empty_shape_result_returns_zero() {
    let sr = ShapeResult::empty(TextDirection::Ltr);
    let metrics = text_painter::metrics_from_shape_result(&sr);
    assert_eq!(metrics.ascent, 0.0);
    assert_eq!(metrics.descent, 0.0);
}

#[test]
fn paint_text_rtl_does_not_crash() {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    let sr = Arc::new(shaper.shape("Hello", &font, TextDirection::Rtl));
    let mut surface = make_surface(200, 100);
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &default_style());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_with_custom_rgba_color() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(200, 100);
    let mut style = default_style();
    style.color = Color::from_rgba8(128, 64, 32, 255);
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_with_partial_opacity_color() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(200, 100);
    let mut style = default_style();
    style.color = Color::from_rgba8(0, 0, 0, 128); // semi-transparent black
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn larger_font_produces_taller_metrics() {
    let sr_small = shape_text_with_size("X", 12.0);
    let sr_large = shape_text_with_size("X", 48.0);
    let m_small = text_painter::metrics_from_shape_result(&sr_small);
    let m_large = text_painter::metrics_from_shape_result(&sr_large);
    assert!(m_large.ascent > m_small.ascent,
        "Larger font should have greater ascent: large={}, small={}",
        m_large.ascent, m_small.ascent);
}

#[test]
fn paint_text_digits() {
    let sr = Arc::new(shape_text("0123456789"));
    let mut surface = make_surface(400, 100);
    text_painter::paint_text(surface.canvas(), &sr, (10.0, 50.0), &default_style());
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── TEXT SHADOW TESTS ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn paint_text_shadow_produces_output() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.color = Color::WHITE; // text invisible, only shadow visible
    style.text_shadow = vec![TextShadow {
        offset_x: 2.0,
        offset_y: 2.0,
        blur_radius: 0.0,
        color: Color::BLACK,
    }];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface), "Shadow should be visible");
}

#[test]
fn paint_text_shadow_with_blur() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_shadow = vec![TextShadow {
        offset_x: 3.0,
        offset_y: 3.0,
        blur_radius: 4.0,
        color: Color::BLACK,
    }];
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn no_shadow_when_empty_shadow_list() {
    let sr = Arc::new(shape_text("Hello"));
    let mut surface = make_surface(200, 100);
    let style = default_style(); // empty text_shadow
    text_painter::paint_text_shadows(surface.canvas(), &sr, (10.0, 50.0), &style);
    assert!(!has_non_white_pixels(&mut surface), "No shadow should be drawn");
}

// ═══════════════════════════════════════════════════════════════════════
// ── DECORATION TESTS ─────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn underline_draws_visible_line() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface), "Underline should be visible");
}

#[test]
fn overline_draws_visible_line() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::OVERLINE;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface), "Overline should be visible");
}

#[test]
fn line_through_draws_visible_line() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::LINE_THROUGH;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface), "Line-through should be visible");
}

#[test]
fn no_decoration_when_none() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::NONE;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(!has_non_white_pixels(&mut surface), "No decoration should be drawn when NONE");
}

#[test]
fn decoration_color_matches_current_color() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.color = Color::RED;
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_color = StyleColor::CurrentColor;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_color_explicit() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_color = StyleColor::Resolved(Color::BLUE);
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_style_solid() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_style = TextDecorationStyle::Solid;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_style_double() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_style = TextDecorationStyle::Double;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_style_dotted() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_style = TextDecorationStyle::Dotted;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_style_dashed() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_style = TextDecorationStyle::Dashed;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_style_wavy() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_style = TextDecorationStyle::Wavy;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_thickness_auto() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_thickness = TextDecorationThickness::Auto;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_thickness_from_font() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_thickness = TextDecorationThickness::FromFont;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_thickness_explicit_length() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_thickness = TextDecorationThickness::Length(3.0);
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn multiple_decorations_underline_and_line_through() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    // Combine underline + line-through via bitwise OR
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0 | TextDecorationLine::LINE_THROUGH.0,
    );
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn all_three_decorations_combined() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine(
        TextDecorationLine::UNDERLINE.0
            | TextDecorationLine::OVERLINE.0
            | TextDecorationLine::LINE_THROUGH.0,
    );
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_transparent_color_not_visible() {
    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_color = StyleColor::Resolved(Color::TRANSPARENT);
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(!has_non_white_pixels(&mut surface),
        "Transparent decoration should not be visible");
}

#[test]
fn decoration_on_empty_text_does_not_crash() {
    let sr = Arc::new(ShapeResult::empty(TextDirection::Ltr));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(100, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    // Should not crash, and nothing drawn (zero width)
}

#[test]
fn decoration_wavy_overline() {
    let sr = Arc::new(shape_text("Hello World"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::OVERLINE;
    style.text_decoration_style = TextDecorationStyle::Wavy;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 60.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_dashed_line_through() {
    let sr = Arc::new(shape_text("Hello World"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(400, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::LINE_THROUGH;
    style.text_decoration_style = TextDecorationStyle::Dashed;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_dotted_overline() {
    let sr = Arc::new(shape_text("Test"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::OVERLINE;
    style.text_decoration_style = TextDecorationStyle::Dotted;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 60.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_double_line_through() {
    let sr = Arc::new(shape_text("Test"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::LINE_THROUGH;
    style.text_decoration_style = TextDecorationStyle::Double;
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::AfterText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn decoration_thick_underline() {
    let sr = Arc::new(shape_text("Thick"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut surface = make_surface(300, 100);
    let mut style = default_style();
    style.text_decoration_line = TextDecorationLine::UNDERLINE;
    style.text_decoration_thickness = TextDecorationThickness::Length(5.0);
    decoration_painter::paint_text_decorations(
        surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
        decoration_painter::DecorationPhase::BeforeText,
        None,
    );
    assert!(has_non_white_pixels(&mut surface));
}

// ═══════════════════════════════════════════════════════════════════════
// ── INTEGRATION TESTS — FULL FRAGMENT PIPELINE ──────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn paint_text_fragment_full_pipeline() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
    });

    let sr = Arc::new(shape_text("Hello World"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let frag = make_text_fragment(text_node, &sr, &metrics);

    let mut surface = make_surface(400, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface), "Text fragment should produce visible pixels");
}

#[test]
fn paint_text_fragment_with_offset() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
    });

    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let mut frag = make_text_fragment(text_node, &sr, &metrics);
    frag.offset = PhysicalOffset::new(
        LayoutUnit::from_f32(50.0),
        LayoutUnit::from_f32(20.0),
    );

    let mut surface = make_surface(400, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_fragment_with_opacity() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
        s.opacity = 0.5;
    });

    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let frag = make_text_fragment(text_node, &sr, &metrics);

    let mut surface = make_surface(300, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface), "Semi-transparent text should be visible");
}

#[test]
fn paint_text_fragment_hidden_visibility() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
        s.visibility = Visibility::Hidden;
    });

    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let frag = make_text_fragment(text_node, &sr, &metrics);

    let mut surface = make_surface(300, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(!has_non_white_pixels(&mut surface),
        "Hidden text should not be visible");
}

#[test]
fn paint_mixed_box_and_text_fragment_tree() {
    let mut doc = Document::new();
    let vp = doc.root();

    // Create a div with a red background
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, div);

    // Create a text node
    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some("Hello".to_string());
    doc.node_mut(text_node).style.color = Color::BLACK;
    doc.append_child(div, text_node);

    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);

    // Build fragment tree: box → text child
    let mut box_frag = Fragment::new_box(
        div,
        PhysicalSize::new(LayoutUnit::from_f32(300.0), LayoutUnit::from_f32(50.0)),
    );

    let text_frag = make_text_fragment(text_node, &sr, &metrics);
    box_frag.children.push(text_frag);

    let mut surface = make_surface(400, 100);
    paint_fragment(surface.canvas(), &box_frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_fragment_with_decorations() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
        s.text_decoration_line = TextDecorationLine::UNDERLINE;
        s.text_decoration_color = StyleColor::CurrentColor;
    });

    let sr = Arc::new(shape_text("Decorated"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let frag = make_text_fragment(text_node, &sr, &metrics);

    let mut surface = make_surface(400, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_fragment_no_shape_result() {
    // A text fragment with no shape_result should not crash or paint
    let (doc, text_node) = make_doc_with_text_style(|_| {});

    let frag = Fragment {
        node_id: text_node,
        kind: FragmentKind::Text,
        offset: PhysicalOffset::zero(),
        size: PhysicalSize::new(LayoutUnit::from_f32(100.0), LayoutUnit::from_f32(20.0)),
        padding: openui_geometry::BoxStrut::zero(),
        border: openui_geometry::BoxStrut::zero(),
        margin: openui_geometry::BoxStrut::zero(),
        children: Vec::new(),
        shape_result: None,
        text_content: None,
        inherited_style: None,
        baseline_offset: 0.0,
        text_combine: None,
        overflow_rect: None,
        has_overflow_clip: false,
    };

    let mut surface = make_surface(200, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(!has_non_white_pixels(&mut surface),
        "Text without shape_result should produce no output");
}

#[test]
fn paint_nested_boxes_with_text_child() {
    let mut doc = Document::new();
    let vp = doc.root();

    // Outer box with padding
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(400.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.node_mut(outer).style.padding_top = Length::px(10.0);
    doc.node_mut(outer).style.padding_left = Length::px(10.0);
    doc.node_mut(outer).style.background_color = Color::from_rgba8(240, 240, 240, 255);
    doc.append_child(vp, outer);

    // Inner box
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.width = Length::px(380.0);
    doc.node_mut(inner).style.height = Length::px(30.0);
    doc.append_child(outer, inner);

    // Text inside inner
    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some("Nested text".to_string());
    doc.node_mut(text_node).style.color = Color::BLACK;
    doc.append_child(inner, text_node);

    let sr = Arc::new(shape_text("Nested text"));
    let metrics = text_painter::metrics_from_shape_result(&sr);

    let text_frag = make_text_fragment(text_node, &sr, &metrics);

    let mut inner_frag = Fragment::new_box(
        inner,
        PhysicalSize::new(LayoutUnit::from_f32(380.0), LayoutUnit::from_f32(30.0)),
    );
    inner_frag.children.push(text_frag);

    let mut outer_frag = Fragment::new_box(
        outer,
        PhysicalSize::new(LayoutUnit::from_f32(400.0), LayoutUnit::from_f32(100.0)),
    );
    outer_frag.offset = PhysicalOffset::zero();
    outer_frag.children.push(inner_frag);

    let mut surface = make_surface(500, 200);
    paint_fragment(surface.canvas(), &outer_frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_multiple_text_fragments_in_sequence() {
    let mut doc = Document::new();
    let vp = doc.root();

    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(600.0);
    doc.node_mut(container).style.height = Length::px(50.0);
    doc.append_child(vp, container);

    let t1 = doc.create_node(ElementTag::Text);
    doc.node_mut(t1).text = Some("Hello ".to_string());
    doc.node_mut(t1).style.color = Color::BLACK;
    doc.append_child(container, t1);

    let t2 = doc.create_node(ElementTag::Text);
    doc.node_mut(t2).text = Some("World".to_string());
    doc.node_mut(t2).style.color = Color::RED;
    doc.append_child(container, t2);

    let sr1 = Arc::new(shape_text("Hello "));
    let sr2 = Arc::new(shape_text("World"));
    let m1 = text_painter::metrics_from_shape_result(&sr1);
    let m2 = text_painter::metrics_from_shape_result(&sr2);

    let frag1 = make_text_fragment(t1, &sr1, &m1);
    let mut frag2 = make_text_fragment(t2, &sr2, &m2);

    // Position second fragment after first
    frag2.offset = PhysicalOffset::new(
        LayoutUnit::from_f32(sr1.width()),
        LayoutUnit::from_f32(0.0),
    );

    let mut container_frag = Fragment::new_box(
        container,
        PhysicalSize::new(LayoutUnit::from_f32(600.0), LayoutUnit::from_f32(50.0)),
    );
    container_frag.children.push(frag1);
    container_frag.children.push(frag2);

    let mut surface = make_surface(600, 100);
    paint_fragment(surface.canvas(), &container_frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_text_fragment_zero_opacity() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
        s.opacity = 0.0;
    });

    let sr = Arc::new(shape_text("Hello"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let frag = make_text_fragment(text_node, &sr, &metrics);

    let mut surface = make_surface(300, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(!has_non_white_pixels(&mut surface),
        "Zero opacity text should not be visible");
}

#[test]
fn paint_text_fragment_with_text_shadow() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
        s.text_shadow = vec![TextShadow {
            offset_x: 2.0,
            offset_y: 2.0,
            blur_radius: 1.0,
            color: Color::from_rgba8(128, 128, 128, 255),
        }];
    });

    let sr = Arc::new(shape_text("Shadow"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let frag = make_text_fragment(text_node, &sr, &metrics);

    let mut surface = make_surface(400, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn new_text_fragment_constructor_sets_kind() {
    let sr = Arc::new(shape_text("Test"));
    let (_doc, text_node) = make_doc_with_text_style(|_| {});
    let frag = Fragment::new_text(
        text_node,
        PhysicalSize::new(LayoutUnit::from_f32(50.0), LayoutUnit::from_f32(16.0)),
        sr,
        "Test".to_string(),
    );
    assert_eq!(frag.kind, FragmentKind::Text);
    assert!(frag.shape_result.is_some());
    assert_eq!(frag.text_content.as_deref(), Some("Test"));
}

#[test]
fn new_box_fragment_has_no_shape_result() {
    let (_doc, text_node) = make_doc_with_text_style(|_| {});
    let frag = Fragment::new_box(
        text_node,
        PhysicalSize::new(LayoutUnit::from_f32(100.0), LayoutUnit::from_f32(100.0)),
    );
    assert_eq!(frag.kind, FragmentKind::Box);
    assert!(frag.shape_result.is_none());
    assert!(frag.text_content.is_none());
}

#[test]
fn paint_text_with_all_decoration_styles_does_not_crash() {
    let sr = Arc::new(shape_text("All styles"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let styles = [
        TextDecorationStyle::Solid,
        TextDecorationStyle::Double,
        TextDecorationStyle::Dotted,
        TextDecorationStyle::Dashed,
        TextDecorationStyle::Wavy,
    ];
    let lines = [
        TextDecorationLine::UNDERLINE,
        TextDecorationLine::OVERLINE,
        TextDecorationLine::LINE_THROUGH,
    ];

    for &deco_style in &styles {
        for &deco_line in &lines {
            let mut surface = make_surface(300, 100);
            let mut style = default_style();
            style.text_decoration_line = deco_line;
            style.text_decoration_style = deco_style;
            decoration_painter::paint_text_decorations(
                surface.canvas(), &sr, (10.0, 50.0), &style, &metrics,
                decoration_painter::DecorationPhase::AfterText,
                None,
            );
            // Should not panic for any combination
        }
    }
}

#[test]
fn paint_text_fragment_large_font() {
    let (doc, text_node) = make_doc_with_text_style(|s| {
        s.color = Color::BLACK;
        s.font_size = 48.0;
    });

    let sr = Arc::new(shape_text_with_size("Big", 48.0));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let frag = make_text_fragment(text_node, &sr, &metrics);

    let mut surface = make_surface(400, 200);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(has_non_white_pixels(&mut surface));
}

#[test]
fn paint_ellipsis_hidden_visibility_no_output() {
    // An anonymous text fragment (ellipsis) with visibility:hidden should NOT paint.
    let doc = Document::new();

    let sr = Arc::new(shape_text("\u{2026}"));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let width = sr.width();
    let height = metrics.ascent + metrics.descent;

    let frag = Fragment {
        node_id: openui_dom::NodeId::NONE,
        kind: FragmentKind::Text,
        offset: PhysicalOffset::new(LayoutUnit::from_f32(10.0), LayoutUnit::from_f32(20.0)),
        size: PhysicalSize::new(LayoutUnit::from_f32(width), LayoutUnit::from_f32(height)),
        padding: openui_geometry::BoxStrut::zero(),
        border: openui_geometry::BoxStrut::zero(),
        margin: openui_geometry::BoxStrut::zero(),
        children: Vec::new(),
        shape_result: Some(Arc::clone(&sr)),
        text_content: Some("\u{2026}".to_string()),
        inherited_style: Some({
            let mut s = ComputedStyle::default();
            s.color = Color::BLACK;
            s.visibility = Visibility::Hidden;
            s
        }),
        baseline_offset: metrics.ascent,
        text_combine: None,
        overflow_rect: None,
        has_overflow_clip: false,
    };

    let mut surface = make_surface(200, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());
    assert!(
        !has_non_white_pixels(&mut surface),
        "Ellipsis with visibility:hidden should not produce visible pixels"
    );
}
