//! A rounded translucent border over a background image that cannot render.
//!
//! Blink still selects `kBackgroundBleedClipLayer` for the declared image: it
//! clips to the outer rounded border and opens a layer. The layer then holds
//! only the border's fast-path `DrawPath`, so cc's playback folds
//! SaveLayerAlpha -> DrawPath -> Restore into that one draw. The border is
//! painted straight through the antialiased clip with no intermediate layer.

use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::Fragment;
use openui_paint::paint_fragment;
use openui_style::*;
use skia_safe::{
    surfaces, ClipOp, Color as SkColor, Paint, PathBuilder, PathFillType, RRect, Rect, Surface,
    Vector,
};

const EMPTY_SHA256: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

fn white_surface() -> Surface {
    let mut surface = surfaces::raster_n32_premul((140, 100)).expect("surface");
    surface.canvas().clear(SkColor::WHITE);
    surface
}

fn pixels(surface: &mut Surface) -> Vec<u8> {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = info.min_row_bytes();
    let mut bytes = vec![0u8; info.height() as usize * row_bytes];
    assert!(image.read_pixels(
        &info,
        &mut bytes,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    ));
    bytes
}

fn translucent_rounded_border_box(doc: &mut Document) -> Fragment {
    let unavailable = doc.register_image_resource(
        "unavailable:missing.png",
        "image/png",
        EMPTY_SHA256,
        Vec::new(),
    );
    let root = doc.root();
    let div = doc.create_node(ElementTag::Div);
    let border = StyleColor::Resolved(Color::from_rgba8(60, 150, 255, 102));
    doc.update_resolved_style(div, |s| {
        s.update_derived(|c| c.display = Display::Block);
        s.update_derived(|c| c.width = Length::px(100.0));
        s.update_derived(|c| c.height = Length::px(60.0));
        s.update_derived(|c| {
            c.border_top_width = 8;
            c.border_right_width = 8;
            c.border_bottom_width = 8;
            c.border_left_width = 8;
            c.border_top_style = BorderStyle::Solid;
            c.border_right_style = BorderStyle::Solid;
            c.border_bottom_style = BorderStyle::Solid;
            c.border_left_style = BorderStyle::Solid;
            c.border_top_color = border;
            c.border_right_color = border;
            c.border_bottom_color = border;
            c.border_left_color = border;
        });
        s.update_derived(|c| {
            let radius = (20.0_f32, 20.0_f32);
            c.border_top_left_radius = radius;
            c.border_top_right_radius = radius;
            c.border_bottom_right_radius = radius;
            c.border_bottom_left_radius = radius;
        });
        s.update_derived(|c| {
            c.background_layers = vec![BackgroundLayer::new(CssImage::Raster(unavailable))]
        });
    });
    doc.append_child(root, div);
    Fragment::new_box(
        div,
        PhysicalSize::new(LayoutUnit::from_i32(116), LayoutUnit::from_i32(76)),
    )
}

#[test]
fn border_over_unrenderable_background_image_paints_without_a_layer() {
    let mut doc = Document::new();
    let fragment = translucent_rounded_border_box(&mut doc);
    let mut actual = white_surface();
    paint_fragment(
        actual.canvas(),
        &fragment,
        &doc,
        PhysicalOffset::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(10)),
    );

    // Blink: ClipContouredRect(outer), then the border's inverse-winding
    // DrawPath of the inner contour, folded out of its layer.
    let mut expected = white_surface();
    let canvas = expected.canvas();
    canvas.save();
    canvas.clip_rrect(
        RRect::new_rect_radii(
            Rect::from_xywh(10.0, 10.0, 116.0, 76.0),
            &[Vector::new(20.0, 20.0); 4],
        ),
        ClipOp::Intersect,
        true,
    );
    let mut path = PathBuilder::new();
    path.set_fill_type(PathFillType::InverseWinding);
    path.add_rrect(
        RRect::new_rect_radii(
            Rect::from_xywh(18.0, 18.0, 100.0, 60.0),
            &[Vector::new(12.0, 12.0); 4],
        ),
        None,
        None,
    );
    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_color(SkColor::from_argb(102, 60, 150, 255));
    canvas.draw_path(&path.detach(), &paint);
    canvas.restore();

    let (actual, expected) = (pixels(&mut actual), pixels(&mut expected));
    let differing = actual
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .filter(|(a, e)| a != e)
        .count();
    assert_eq!(differing, 0, "border pixels differ from the folded draw");
}
