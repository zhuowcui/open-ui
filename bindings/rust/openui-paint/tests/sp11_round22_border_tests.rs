//! Tests for SP11 Round 22 code review fixes — openui-paint crate.
//!
//! Issue 7: Non-solid border styles are now painted instead of silently skipped.

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

fn make_bordered_box(doc: &mut Document, style_fn: impl FnOnce(&mut ComputedStyle)) -> Fragment {
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let s = &mut doc.node_mut(div).style;
        s.display = Display::Block;
        s.width = Length::px(60.0);
        s.height = Length::px(60.0);
        // 4px red border on all sides.
        s.border_top_width = 4;
        s.border_right_width = 4;
        s.border_bottom_width = 4;
        s.border_left_width = 4;
        s.border_top_color = StyleColor::Resolved(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        s.border_right_color = StyleColor::Resolved(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        s.border_bottom_color = StyleColor::Resolved(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        s.border_left_color = StyleColor::Resolved(Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 });
        style_fn(s);
    }
    doc.append_child(vp, div);

    Fragment::new_box(
        div,
        PhysicalSize::new(LayoutUnit::from_f32(60.0), LayoutUnit::from_f32(60.0)),
    )
}

// ── Issue 7: Border style tests ──────────────────────────────────────────

#[test]
fn dashed_border_paints_something() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Dashed;
        s.border_right_style = BorderStyle::Dashed;
        s.border_bottom_style = BorderStyle::Dashed;
        s.border_left_style = BorderStyle::Dashed;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // The top border region (y=0..4, x=0..60) should have non-white pixels
    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Dashed border should paint visible pixels"
    );
}

#[test]
fn dotted_border_paints_something() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Dotted;
        s.border_right_style = BorderStyle::Dotted;
        s.border_bottom_style = BorderStyle::Dotted;
        s.border_left_style = BorderStyle::Dotted;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Dotted border should paint visible pixels"
    );
}

#[test]
fn double_border_paints_something() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Double;
        s.border_right_style = BorderStyle::Double;
        s.border_bottom_style = BorderStyle::Double;
        s.border_left_style = BorderStyle::Double;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Double border should paint visible pixels"
    );
}

#[test]
fn groove_border_paints_something() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Groove;
        s.border_right_style = BorderStyle::Groove;
        s.border_bottom_style = BorderStyle::Groove;
        s.border_left_style = BorderStyle::Groove;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Groove border should paint visible pixels"
    );
}

#[test]
fn ridge_border_paints_something() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Ridge;
        s.border_right_style = BorderStyle::Ridge;
        s.border_bottom_style = BorderStyle::Ridge;
        s.border_left_style = BorderStyle::Ridge;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Ridge border should paint visible pixels"
    );
}

#[test]
fn inset_border_paints_something() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Inset;
        s.border_right_style = BorderStyle::Inset;
        s.border_bottom_style = BorderStyle::Inset;
        s.border_left_style = BorderStyle::Inset;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Inset border should paint visible pixels"
    );
}

#[test]
fn outset_border_paints_something() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Outset;
        s.border_right_style = BorderStyle::Outset;
        s.border_bottom_style = BorderStyle::Outset;
        s.border_left_style = BorderStyle::Outset;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Outset border should paint visible pixels"
    );
}

#[test]
fn none_border_paints_nothing() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::None;
        s.border_right_style = BorderStyle::None;
        s.border_bottom_style = BorderStyle::None;
        s.border_left_style = BorderStyle::None;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        !has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "None border should not paint anything"
    );
}

#[test]
fn solid_border_still_works_after_refactor() {
    let mut doc = Document::new();
    let frag = make_bordered_box(&mut doc, |s| {
        s.border_top_style = BorderStyle::Solid;
        s.border_right_style = BorderStyle::Solid;
        s.border_bottom_style = BorderStyle::Solid;
        s.border_left_style = BorderStyle::Solid;
    });

    let mut surface = make_surface(80, 80);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        has_non_white_pixels_in_region(&mut surface, 0, 0, 60, 5),
        "Solid border should still paint visible pixels"
    );
}
