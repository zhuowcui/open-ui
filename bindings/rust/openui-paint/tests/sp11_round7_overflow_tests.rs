//! Tests for SP11 Round 7 — overflow hidden clipping in painter.
//!
//! Verifies that paint_fragment clips children when overflow: hidden is set.

use std::sync::Arc;

use skia_safe::{surfaces, Color as SkColor, Surface};

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalSize};
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
fn overflow_hidden_clips_text_beyond_box() {
    // A box with overflow:hidden should clip children that extend
    // beyond its boundaries.
    let mut doc = Document::new();
    let vp = doc.root();

    // Create a narrow box (50px wide) with overflow:hidden
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(50.0);
    doc.node_mut(div).style.height = Length::px(30.0);
    doc.node_mut(div).style.overflow_x = Overflow::Hidden;
    doc.node_mut(div).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, div);

    // Text child that extends beyond the box
    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some("XXXXXXXXXXXXX".to_string());
    doc.node_mut(text_node).style.color = Color::BLACK;
    doc.append_child(div, text_node);

    let sr = Arc::new(shape_text("XXXXXXXXXXXXX"));
    let metrics = text_painter::metrics_from_shape_result(&sr);

    // Text is wider than 50px — it extends beyond the box
    assert!(
        sr.width > 50.0,
        "Text should be wider than 50px to test clipping, got {}",
        sr.width
    );

    // Build fragment tree: box → text child (positioned to extend beyond)
    let mut box_frag = Fragment::new_box(
        div,
        PhysicalSize::new(LayoutUnit::from_f32(50.0), LayoutUnit::from_f32(30.0)),
    );

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
    text_frag.baseline_offset = metrics.ascent;
    box_frag.children.push(text_frag);

    // Surface wider than the box to check if pixels leak beyond
    let mut surface = make_surface(200, 50);
    paint_fragment(surface.canvas(), &box_frag, &doc, PhysicalOffset::zero());

    // The region beyond x=50 should be empty (clipped)
    let has_leaked = has_non_white_pixels_in_region(&mut surface, 60, 0, 140, 50);
    assert!(
        !has_leaked,
        "Pixels beyond the box (x>50) should be clipped by overflow:hidden"
    );

    // But the region inside the box should have content
    let has_content = has_non_white_pixels_in_region(&mut surface, 0, 0, 50, 30);
    assert!(
        has_content,
        "Inside the box should have visible text content"
    );
}
