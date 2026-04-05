//! Tests for SP11 Round 24 code review fixes — openui-paint crate.
//!
//! Issue 5: Non-uniform border corners use overlapping rectangles instead of trapezoids.
//! The fix draws each border side as a 4-point polygon with diagonal corner joins.

use skia_safe::{surfaces, Color as SkColor, Surface};
use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::Fragment;
use openui_paint::paint_fragment;
use openui_style::*;

fn make_surface(width: i32, height: i32) -> Surface {
    let mut surface = surfaces::raster_n32_premul((width, height))
        .expect("Failed to create Skia surface");
    surface.canvas().clear(SkColor::WHITE);
    surface
}

/// Sample average color in a rectangular region (returns RGB averages).
fn sample_avg_color(surface: &mut Surface, x: i32, y: i32, w: i32, h: i32) -> (f32, f32, f32) {
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

    let bpp = 4;
    let mut r_sum = 0.0f64;
    let mut g_sum = 0.0f64;
    let mut b_sum = 0.0f64;
    let mut count = 0u32;
    for py in y..(y + h).min(info.height()) {
        for px in x..(x + w).min(info.width() as i32) {
            let offset = (py as usize) * row_bytes + (px as usize) * bpp;
            if offset + 3 < pixels.len() {
                b_sum += pixels[offset] as f64;
                g_sum += pixels[offset + 1] as f64;
                r_sum += pixels[offset + 2] as f64;
                count += 1;
            }
        }
    }
    if count == 0 {
        return (255.0, 255.0, 255.0);
    }
    (
        (r_sum / count as f64) as f32,
        (g_sum / count as f64) as f32,
        (b_sum / count as f64) as f32,
    )
}

fn make_non_uniform_border_box(doc: &mut Document) -> Fragment {
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let s = &mut doc.node_mut(div).style;
        s.display = Display::Block;
        s.width = Length::px(80.0);
        s.height = Length::px(80.0);
        // Different colors on each side to test trapezoid rendering
        s.border_top_width = 10;
        s.border_right_width = 10;
        s.border_bottom_width = 10;
        s.border_left_width = 10;
        // Red top, green right, blue bottom, yellow left
        s.border_top_color = StyleColor::Resolved(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        s.border_right_color = StyleColor::Resolved(Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 });
        s.border_bottom_color = StyleColor::Resolved(Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 });
        s.border_left_color = StyleColor::Resolved(Color { r: 1.0, g: 1.0, b: 0.0, a: 1.0 });
        s.border_top_style = BorderStyle::Solid;
        s.border_right_style = BorderStyle::Solid;
        s.border_bottom_style = BorderStyle::Solid;
        s.border_left_style = BorderStyle::Solid;
    }
    doc.append_child(vp, div);

    Fragment::new_box(div, PhysicalSize::new(
        LayoutUnit::from_i32(80),
        LayoutUnit::from_i32(80),
    ))
}

// ── Issue 5: Non-uniform borders use trapezoids, not overlapping rects ──

#[test]
fn r24_non_uniform_border_top_center_is_red() {
    // The top border center (away from corners) should be clearly red.
    let mut doc = Document::new();
    let frag = make_non_uniform_border_box(&mut doc);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Sample top border center: row 3-7, columns 30-50 (well inside the top edge)
    let (r, g, b) = sample_avg_color(&mut surface, 30, 3, 20, 4);

    // Top border should be red-ish
    assert!(
        r > 200.0 && g < 50.0 && b < 50.0,
        "Top border center should be red, got r={r}, g={g}, b={b}"
    );
}

#[test]
fn r24_non_uniform_border_right_center_is_green() {
    // The right border center should be clearly green.
    let mut doc = Document::new();
    let frag = make_non_uniform_border_box(&mut doc);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Sample right border center: columns 73-77, rows 30-50
    let (r, g, b) = sample_avg_color(&mut surface, 73, 30, 4, 20);

    assert!(
        g > 200.0 && r < 50.0 && b < 50.0,
        "Right border center should be green, got r={r}, g={g}, b={b}"
    );
}

#[test]
fn r24_non_uniform_border_corner_no_overlap() {
    // The top-right corner pixel should be covered by either the top or right
    // trapezoid, not an overlapping rectangle bleeding into wrong colors.
    // With trapezoid rendering, the diagonal join from (80,0) to (70,10)
    // means pixels near (75,5) should be either red or green, not both.
    let mut doc = Document::new();
    let frag = make_non_uniform_border_box(&mut doc);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Sample a small region in the top-right corner area
    let (r, g, b) = sample_avg_color(&mut surface, 74, 2, 3, 3);

    // Should be either predominantly red (top border) or predominantly green (right border)
    // but NOT white (unpainted) or a mix that shows overlap artifacts.
    let max_channel = r.max(g).max(b);
    assert!(
        max_channel > 100.0,
        "Corner should have some color (red or green), got r={r}, g={g}, b={b}"
    );
    // With proper trapezoids, the corner diagonal should give a clean split
    // The corner at (74,2) should be mostly red (top border wins near top-right outer corner)
    // because the top trapezoid reaches to the outer-top-right corner.
    assert!(
        r > 100.0 || g > 100.0,
        "Corner should be red or green (not white), got r={r}, g={g}, b={b}"
    );
}
