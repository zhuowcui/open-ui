//! Render pipeline — layout a Document and paint to PNG or Skia surface.
//!
//! This is the main public API for headless rendering: build a Document,
//! set styles, call `render_to_png()`, and get a pixel-perfect PNG.

use openui_dom::Document;
use openui_geometry::LayoutUnit;
use openui_layout::{block_layout, ConstraintSpace};
use skia_safe::{
    surfaces, Color as SkColor, EncodedImageFormat, ImageInfo, PixelGeometry, Surface,
    SurfaceProps, SurfacePropsFlags,
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
    surface.canvas().clear(canvas_color);

    // Layout
    let space =
        ConstraintSpace::for_root(LayoutUnit::from_i32(width), LayoutUnit::from_i32(height));
    let fragment = block_layout(doc, doc.root(), &space);

    // Paint
    let zero_offset = openui_geometry::PhysicalOffset::zero();
    paint_fragment(surface.canvas(), &fragment, doc, zero_offset);

    Ok(surface)
}

fn create_raster_surface(width: i32, height: i32, real_font_raster: bool) -> Option<Surface> {
    if real_font_raster {
        // Chromium's Linux Skia build pins these in //skia/BUILD.gn. Passing
        // them explicitly avoids inheriting the independently-built skia-safe
        // defaults (0.5 contrast and sRGB gamma).
        let contrast = raster_parameter("OPENUI_TEXT_CONTRAST", 0.2);
        let gamma = raster_parameter("OPENUI_TEXT_GAMMA", 1.2);
        let props = SurfaceProps::new_with_text_properties(
            SurfacePropsFlags::default(),
            PixelGeometry::RGBH,
            contrast,
            gamma,
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

fn raster_parameter(name: &str, default: f32) -> f32 {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<f32>().ok())
        .unwrap_or(default)
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
