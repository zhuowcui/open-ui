//! Tests for SP11 Round 8 — overflow clipping at padding box, not border box.
//!
//! CSS specifies that overflow:hidden clips at the padding box (inner edge of
//! border). Verifies that content inside the border area is clipped, while
//! content inside the padding area remains visible.

use std::sync::Arc;

use skia_safe::{surfaces, Color as SkColor, Surface};

use openui_dom::{Document, ElementTag};
use openui_geometry::{BoxStrut, LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::Fragment;
use openui_paint::{paint_fragment, text_painter};
use openui_style::*;
use openui_text::{Font, FontDescription, TextDirection, TextShaper};

fn make_font(size: f32) -> Font {
    let mut desc = FontDescription::new();
    desc.size = size;
    desc.specified_size = size;
    Font::new(desc)
}

fn shape_text(text: &str) -> openui_text::ShapeResult {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Ltr)
}

fn make_surface(width: i32, height: i32) -> Surface {
    let mut surface = surfaces::raster_n32_premul((width, height))
        .expect("Failed to create Skia surface");
    surface.canvas().clear(SkColor::WHITE);
    surface
}

/// Check if any pixels in the given rectangular region are non-white.
fn has_non_white_pixels_in_region(
    surface: &mut Surface,
    x: i32, y: i32, w: i32, h: i32,
) -> bool {
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

    let bpp = 4; // BGRA
    let img_w = info.width() as i32;
    for py in y..(y + h).min(info.height()) {
        for px in x..(x + w).min(img_w) {
            let offset = (py as usize) * row_bytes + (px as usize) * bpp;
            if offset + 3 < pixels.len() {
                let (b, g, r) = (pixels[offset], pixels[offset + 1], pixels[offset + 2]);
                if r != 0xFF || g != 0xFF || b != 0xFF {
                    return true;
                }
            }
        }
    }
    false
}

#[test]
fn overflow_hidden_clips_at_padding_box_not_border_box() {
    // A box with 20px borders on all sides, 100px total width/height,
    // overflow:hidden. Content that extends into the border area should be
    // clipped. The visible area should be 60×60 (100 - 20 - 20).
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(100.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.overflow_x = Overflow::Hidden;
    doc.node_mut(div).style.overflow_y = Overflow::Hidden;
    // Border widths set on the style for consistency, but border_style left
    // as default (None) so no visible border is painted — we only care about
    // the clip inset from fragment.border.
    doc.append_child(vp, div);

    // Wide text that extends beyond the padding box
    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some("XXXXXXXXXXXXXXXXXXXXX".to_string());
    doc.node_mut(text_node).style.color = Color::BLACK;
    doc.append_child(div, text_node);

    let sr = Arc::new(shape_text("XXXXXXXXXXXXXXXXXXXXX"));
    let metrics = text_painter::metrics_from_shape_result(&sr);

    let text_width = sr.width;
    let text_height = metrics.ascent + metrics.descent;
    let mut text_frag = Fragment::new_text(
        text_node,
        PhysicalSize::new(
            LayoutUnit::from_f32(text_width),
            LayoutUnit::from_f32(text_height),
        ),
        Arc::clone(&sr),
        "XXXXXXXXXXXXXXXXXXXXX".to_string(),
    );
    // Position text at the padding box origin (after border)
    text_frag.offset = PhysicalOffset::new(
        LayoutUnit::from_f32(20.0),
        LayoutUnit::from_f32(20.0),
    );

    let mut box_frag = Fragment::new_box(
        div,
        PhysicalSize::new(LayoutUnit::from_f32(100.0), LayoutUnit::from_f32(100.0)),
    );
    box_frag.border = BoxStrut::new(
        LayoutUnit::from_f32(20.0),
        LayoutUnit::from_f32(20.0),
        LayoutUnit::from_f32(20.0),
        LayoutUnit::from_f32(20.0),
    );
    box_frag.children.push(text_frag);

    let mut surface = make_surface(150, 120);
    paint_fragment(surface.canvas(), &box_frag, &doc, PhysicalOffset::zero());

    // Region beyond the padding box right edge (x >= 80 = 20 border + 60 content)
    // should be clipped (no text pixels).
    let leaked_right = has_non_white_pixels_in_region(&mut surface, 85, 20, 60, 60);
    assert!(
        !leaked_right,
        "Text should be clipped at the padding box right edge (x=80), not the border box edge (x=100)"
    );

    // But the region inside the padding box should have content.
    let has_content = has_non_white_pixels_in_region(&mut surface, 20, 20, 60, 30);
    assert!(
        has_content,
        "Inside the padding box should have visible text content"
    );
}

#[test]
fn overflow_hidden_no_border_still_clips() {
    // Without borders, the padding box equals the border box —
    // clipping should still work as before.
    let mut doc = Document::new();
    let vp = doc.root();

    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(50.0);
    doc.node_mut(div).style.height = Length::px(30.0);
    doc.node_mut(div).style.overflow_x = Overflow::Hidden;
    doc.node_mut(div).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, div);

    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some("XXXXXXXXXXXXX".to_string());
    doc.node_mut(text_node).style.color = Color::BLACK;
    doc.append_child(div, text_node);

    let sr = Arc::new(shape_text("XXXXXXXXXXXXX"));
    let metrics = text_painter::metrics_from_shape_result(&sr);

    let text_width = sr.width;
    let text_height = metrics.ascent + metrics.descent;
    let mut text_frag = Fragment::new_text(
        text_node,
        PhysicalSize::new(
            LayoutUnit::from_f32(text_width),
            LayoutUnit::from_f32(text_height),
        ),
        Arc::clone(&sr),
        "XXXXXXXXXXXXX".to_string(),
    );
    text_frag.offset = PhysicalOffset::new(LayoutUnit::zero(), LayoutUnit::zero());

    let mut box_frag = Fragment::new_box(
        div,
        PhysicalSize::new(LayoutUnit::from_f32(50.0), LayoutUnit::from_f32(30.0)),
    );
    box_frag.children.push(text_frag);

    let mut surface = make_surface(200, 50);
    paint_fragment(surface.canvas(), &box_frag, &doc, PhysicalOffset::zero());

    let has_leaked = has_non_white_pixels_in_region(&mut surface, 60, 0, 140, 50);
    assert!(
        !has_leaked,
        "Pixels beyond the box (x>50) should be clipped by overflow:hidden"
    );

    let has_content = has_non_white_pixels_in_region(&mut surface, 0, 0, 50, 30);
    assert!(
        has_content,
        "Inside the box should have visible text content"
    );
}
