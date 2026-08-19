//! Render pipeline — layout a Document and paint to PNG or Skia surface.
//!
//! This is the main public API for headless rendering: build a Document,
//! set styles, call `render_to_png()`, and get a pixel-perfect PNG.

use openui_dom::Document;
use openui_geometry::LayoutUnit;
use openui_layout::{block_layout, ConstraintSpace};
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    surfaces, Color as SkColor, EncodedImageFormat, FilterMode, ImageInfo, Paint, PictureRecorder,
    PixelGeometry, Rect, SamplingOptions, Surface, SurfaceProps, SurfacePropsFlags,
};

use crate::painter::paint_fragment;

/// Render a Document tree to a PNG file.
///
/// 1. Performs block layout starting from the viewport root.
/// 2. Creates a Skia raster surface at the given dimensions.
/// 3. Paints the fragment tree to the surface.
/// 4. Encodes the surface to PNG and writes to the given path.
pub fn render_to_png(doc: &Document, width: i32, height: i32, path: &str) -> Result<(), String> {
    let mut surface = render_to_surface(doc, width, height)?;

    // Encode to PNG
    let image = surface.image_snapshot();
    let data = image
        .encode(None, EncodedImageFormat::PNG, None)
        .ok_or_else(|| "Failed to encode PNG".to_string())?;

    std::fs::write(path, data.as_bytes()).map_err(|e| format!("Failed to write PNG: {}", e))?;

    Ok(())
}

/// Render a Document tree to a Skia surface (for testing / compositing).
///
/// Returns the surface with the rendered content. The surface uses
/// raster (CPU) backend — same pixels as Blink's software renderer.
pub fn render_to_surface(doc: &Document, width: i32, height: i32) -> Result<Surface, String> {
    // Chromium's Linux LCD path uses horizontal RGB subpixels. This is scoped
    // by the comparison runner to SP16 IDs so historical Ahem and box-only
    // snapshots retain the default unknown pixel geometry.
    let real_font_raster = std::env::var("OPENUI_REAL_FONT_RASTER").ok().as_deref() == Some("1");
    let mut surface = create_raster_surface(width, height, real_font_raster)
        .ok_or_else(|| "Failed to create Skia surface".to_string())?;

    // Clear to the propagated root/body canvas background. Transparent
    // documents retain the browser's white default canvas.
    let canvas_color = doc
        .canvas_background_source()
        .map(|source| doc.node(source).style.background_color)
        .map(|color| {
            SkColor::from_argb(
                (color.a * 255.0).round() as u8,
                (color.r * 255.0).round() as u8,
                (color.g * 255.0).round() as u8,
                (color.b * 255.0).round() as u8,
            )
        })
        .unwrap_or(SkColor::WHITE);
    // Layout
    let space =
        ConstraintSpace::for_root(LayoutUnit::from_i32(width), LayoutUnit::from_i32(height));
    let fragment = block_layout(doc, doc.root(), &space);

    // Chromium's software compositor rasterizes paint records into overlapping
    // 256px tiles and composites each tile's 255px interior. Replaying the
    // display list with the same tile origin is observable for antialiased
    // geometry that crosses a tile boundary, so keep rasterization and final
    // composition as separate, general stages here too.
    let bounds = Rect::from_xywh(0.0, 0.0, width as f32, height as f32);
    let mut recorder = PictureRecorder::new();
    let recording_canvas = recorder.begin_recording(bounds, false);
    recording_canvas.clear(canvas_color);
    paint_fragment(
        recording_canvas,
        &fragment,
        doc,
        openui_geometry::PhysicalOffset::zero(),
    );
    let picture = recorder
        .finish_recording_as_picture(None)
        .ok_or_else(|| "Failed to record paint commands".to_string())?;

    surface.canvas().clear(canvas_color);
    const TILE_SIZE: i32 = 256;
    const TILE_STEP: i32 = TILE_SIZE - 2;
    for tile_y in (0..height).step_by(TILE_STEP as usize) {
        for tile_x in (0..width).step_by(TILE_STEP as usize) {
            let mut tile = create_raster_surface(TILE_SIZE, TILE_SIZE, real_font_raster)
                .ok_or_else(|| "Failed to create raster tile".to_string())?;
            tile.canvas().clear(canvas_color);
            tile.canvas().translate((-tile_x as f32, -tile_y as f32));
            tile.canvas().draw_picture(&picture, None, None);

            let image = tile.image_snapshot();
            let crop_left = if tile_x == 0 { 0 } else { 1 };
            let crop_top = if tile_y == 0 { 0 } else { 1 };
            let destination_x = tile_x + crop_left;
            let destination_y = tile_y + crop_top;
            let visible_right = (tile_x + TILE_SIZE - 1).min(width);
            let visible_bottom = (tile_y + TILE_SIZE - 1).min(height);
            let visible_width = visible_right - destination_x;
            let visible_height = visible_bottom - destination_y;
            if visible_width <= 0 || visible_height <= 0 {
                continue;
            }
            let source = Rect::from_xywh(
                crop_left as f32,
                crop_top as f32,
                visible_width as f32,
                visible_height as f32,
            );
            let destination = Rect::from_xywh(
                destination_x as f32,
                destination_y as f32,
                visible_width as f32,
                visible_height as f32,
            );
            surface.canvas().draw_image_rect_with_sampling_options(
                image,
                Some((&source, SrcRectConstraint::Strict)),
                destination,
                SamplingOptions::from(FilterMode::Linear),
                &Paint::default(),
            );
        }
    }

    Ok(surface)
}

fn create_raster_surface(width: i32, height: i32, real_font_raster: bool) -> Option<Surface> {
    if real_font_raster {
        // Chromium's Linux Skia build pins these in //skia/BUILD.gn. Passing
        // them explicitly avoids inheriting the independently-built skia-safe
        // defaults (0.5 contrast and sRGB gamma).
        let props = SurfaceProps::new_with_text_properties(
            SurfacePropsFlags::default(),
            PixelGeometry::RGBH,
            0.2,
            1.2,
        );
        surfaces::raster(
            &ImageInfo::new_n32_premul((width, height), None),
            None,
            Some(&props),
        )
    } else {
        surfaces::raster_n32_premul((width, height))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::ElementTag;
    use openui_geometry::Length;
    use openui_style::*;

    #[test]
    fn render_simple_red_box() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(100.0);
        doc.node_mut(div).style.height = Length::px(100.0);
        doc.node_mut(div).style.background_color = Color::RED;
        doc.append_child(vp, div);

        // Should not panic
        let mut surface = render_to_surface(&doc, 200, 200).unwrap();
        let image = surface.image_snapshot();
        assert_eq!(image.width(), 200);
        assert_eq!(image.height(), 200);
    }

    #[test]
    fn render_nested_boxes() {
        let mut doc = Document::new();
        let vp = doc.root();

        // Outer: blue background, 10px padding
        let outer = doc.create_node(ElementTag::Div);
        doc.node_mut(outer).style.display = Display::Block;
        doc.node_mut(outer).style.width = Length::px(200.0);
        doc.node_mut(outer).style.padding_top = Length::px(10.0);
        doc.node_mut(outer).style.padding_left = Length::px(10.0);
        doc.node_mut(outer).style.padding_right = Length::px(10.0);
        doc.node_mut(outer).style.padding_bottom = Length::px(10.0);
        doc.node_mut(outer).style.background_color = Color::BLUE;
        doc.append_child(vp, outer);

        // Inner: red box
        let inner = doc.create_node(ElementTag::Div);
        doc.node_mut(inner).style.display = Display::Block;
        doc.node_mut(inner).style.height = Length::px(50.0);
        doc.node_mut(inner).style.background_color = Color::RED;
        doc.append_child(outer, inner);

        let mut surface = render_to_surface(&doc, 400, 300).unwrap();
        let image = surface.image_snapshot();
        assert_eq!(image.width(), 400);
    }

    #[test]
    fn render_with_borders() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(150.0);
        doc.node_mut(div).style.height = Length::px(100.0);
        doc.node_mut(div).style.background_color = Color::from_rgba8(200, 200, 200, 255);
        doc.node_mut(div).style.border_top_width = 3;
        doc.node_mut(div).style.border_right_width = 3;
        doc.node_mut(div).style.border_bottom_width = 3;
        doc.node_mut(div).style.border_left_width = 3;
        doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
        doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
        doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
        doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
        doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
        doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
        doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
        doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
        doc.append_child(vp, div);

        let mut surface = render_to_surface(&doc, 400, 300).unwrap();
        let image = surface.image_snapshot();
        assert_eq!(image.width(), 400);
    }

    #[test]
    fn render_to_png_file() {
        let mut doc = Document::new();
        let vp = doc.root();

        let div = doc.create_node(ElementTag::Div);
        doc.node_mut(div).style.display = Display::Block;
        doc.node_mut(div).style.width = Length::px(100.0);
        doc.node_mut(div).style.height = Length::px(100.0);
        doc.node_mut(div).style.background_color = Color::from_rgba8(50, 150, 50, 255);
        doc.append_child(vp, div);

        let path = "/tmp/openui_test_render.png";
        render_to_png(&doc, 200, 200, path).unwrap();
        assert!(std::path::Path::new(path).exists());
        // Cleanup
        std::fs::remove_file(path).ok();
    }

    #[test]
    fn real_font_surface_uses_horizontal_rgb_geometry() {
        let surface = create_raster_surface(16, 16, true).unwrap();
        let props = surface.props();
        assert_eq!(props.pixel_geometry(), PixelGeometry::RGBH);
        assert_eq!(props.text_contrast(), 0.2);
        assert_eq!(props.text_gamma(), 1.2);
    }

    #[test]
    fn legacy_surface_keeps_unknown_pixel_geometry() {
        let surface = create_raster_surface(16, 16, false).unwrap();
        assert_eq!(surface.props().pixel_geometry(), PixelGeometry::Unknown);
    }
}
