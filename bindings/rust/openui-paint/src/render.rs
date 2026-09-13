//! Render pipeline — layout a Document and paint to PNG or Skia surface.
//!
//! This is the main public API for headless rendering: build a Document,
//! set styles, call `render_to_png()`, and get a pixel-perfect PNG.

use openui_dom::Document;
use openui_geometry::{LayoutUnit, ViewportMetrics};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    surfaces, Color as SkColor, ColorSpace, EncodedImageFormat, FilterMode, ImageInfo, Paint,
    Picture, PictureRecorder, PixelGeometry, Rect, SamplingOptions, Surface, SurfaceProps,
    SurfacePropsFlags,
};

use crate::painter::paint_fragment;

fn has_promoted_non_axis_transform(fragment: &Fragment, doc: &Document) -> bool {
    let is_promoted = !fragment.node_id.is_none() && {
        let style = &doc.node(fragment.node_id).style;
        style.will_change_transform
            && style.transform != openui_style::Transform2D::IDENTITY
            && (style.transform.b != 0.0 || style.transform.c != 0.0)
    };
    is_promoted
        || fragment
            .children
            .iter()
            .any(|child| has_promoted_non_axis_transform(child, doc))
}

fn root_constraint_space(doc: &Document, width: f64, height: f64) -> ConstraintSpace {
    let root_style = &doc.node(doc.root()).style;
    let writing_direction = root_style
        .direction
        .writing_direction(root_style.writing_mode);
    ConstraintSpace::for_root_with_writing_direction(
        LayoutUnit::from_f64(width),
        LayoutUnit::from_f64(height),
        writing_direction,
    )
}

/// Immutable paint recording consumed by both headless and window compositors.
#[derive(Clone)]
pub struct RecordedPicture {
    pub picture: Picture,
    pub viewport: ViewportMetrics,
    pub(crate) lcd_surface: bool,
    pub(crate) direct_replay: bool,
}

/// Lay out and record a document without rasterizing it.
pub fn record_document(
    doc: &Document,
    viewport: ViewportMetrics,
) -> Result<(Fragment, RecordedPicture), String> {
    let width = logical_dimension(viewport.logical_width())?;
    let height = logical_dimension(viewport.logical_height())?;
    let space = root_constraint_space(doc, width, height);
    let fragment = block_layout(doc, doc.root(), &space);
    let picture = record_fragment(doc, &fragment, viewport)?;
    Ok((fragment, picture))
}

/// Record an already-computed fragment tree into an immutable Skia picture.
pub fn record_fragment(
    doc: &Document,
    fragment: &Fragment,
    viewport: ViewportMetrics,
) -> Result<RecordedPicture, String> {
    let width = logical_dimension(viewport.logical_width())?;
    let height = logical_dimension(viewport.logical_height())?;
    let real_font_raster = std::env::var("OPENUI_REAL_FONT_RASTER").ok().as_deref() == Some("1");
    let lcd_surface = real_font_raster
        || doc.uses_native_control_text()
        || (std::env::var("OPENUI_EDGING").ok().as_deref() == Some("alias")
            && doc.uses_lcd_author_text());
    let bounds = Rect::from_xywh(0.0, 0.0, width as f32, height as f32);
    let mut recorder = PictureRecorder::new();
    let recording_canvas = recorder.begin_recording(bounds, false);
    recording_canvas.clear(SkColor::WHITE);
    crate::painter::paint_canvas_background(
        recording_canvas,
        doc,
        fragment,
        width as f32,
        height as f32,
    );
    paint_fragment(
        recording_canvas,
        fragment,
        doc,
        openui_geometry::PhysicalOffset::zero(),
    );
    let picture = recorder
        .finish_recording_as_picture(None)
        .ok_or_else(|| "Failed to record paint commands".to_string())?;
    Ok(RecordedPicture {
        picture,
        viewport,
        lcd_surface,
        direct_replay: has_promoted_non_axis_transform(fragment, doc),
    })
}

/// Rasterize an immutable recording using the exact headless tiling policy.
pub fn rasterize_picture(recording: &RecordedPicture) -> Result<Surface, String> {
    let width = i32::try_from(recording.viewport.physical_width())
        .map_err(|_| "physical viewport width exceeds Skia limit".to_string())?;
    let height = i32::try_from(recording.viewport.physical_height())
        .map_err(|_| "physical viewport height exceeds Skia limit".to_string())?;
    let scale = recording.viewport.device_scale_factor() as f32;
    let mut surface = create_raster_surface(width, height, recording.lcd_surface)
        .ok_or_else(|| "Failed to create Skia surface".to_string())?;
    let canvas_color = SkColor::WHITE;
    surface.canvas().clear(canvas_color);
    if recording.direct_replay {
        surface.canvas().scale((scale, scale));
        surface
            .canvas()
            .draw_picture(&recording.picture, None, None);
        return Ok(surface);
    }
    const TILE_SIZE: i32 = 256;
    const TILE_STEP: i32 = TILE_SIZE - 2;
    for tile_y in (0..height).step_by(TILE_STEP as usize) {
        for tile_x in (0..width).step_by(TILE_STEP as usize) {
            let mut tile = create_raster_surface(TILE_SIZE, TILE_SIZE, recording.lcd_surface)
                .ok_or_else(|| "Failed to create raster tile".to_string())?;
            tile.canvas().clear(canvas_color);
            tile.canvas().translate((-tile_x as f32, -tile_y as f32));
            tile.canvas().scale((scale, scale));
            tile.canvas().draw_picture(&recording.picture, None, None);

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

/// Render a Document tree to a PNG file.
///
/// 1. Performs block layout starting from the viewport root.
/// 2. Creates a Skia raster surface at the given dimensions.
/// 3. Paints the fragment tree to the surface.
/// 4. Encodes the surface to PNG and writes to the given path.
pub fn render_to_png(doc: &Document, viewport: ViewportMetrics, path: &str) -> Result<(), String> {
    let mut surface = render_to_surface(doc, viewport)?;

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
pub fn render_to_surface(doc: &Document, viewport: ViewportMetrics) -> Result<Surface, String> {
    let (_, recording) = record_document(doc, viewport)?;
    rasterize_picture(&recording)
}

fn logical_dimension(value: f64) -> Result<f64, String> {
    if !value.is_finite() || value <= 0.0 || value > 33_554_431.0 {
        return Err("logical viewport dimensions exceed the layout limit".to_string());
    }
    Ok(value)
}

fn create_raster_surface(width: i32, height: i32, real_font_raster: bool) -> Option<Surface> {
    // Chromium composites CSS colors and color-managed replaced images into an
    // sRGB destination. An untagged Skia surface skips conversion for images
    // carrying an embedded ICC profile, leaving their encoded source samples
    // in the output bitmap.
    let image_info = ImageInfo::new_n32_premul((width, height), Some(ColorSpace::new_srgb()));
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
        surfaces::raster(&image_info, None, Some(&props))
    } else {
        surfaces::raster(&image_info, None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::ElementTag;
    use openui_geometry::Length;
    use openui_style::*;

    #[test]
    fn root_constraint_uses_computed_writing_direction() {
        let mut doc = Document::new();
        let root = doc.root();
        doc.node_mut(root).style.writing_mode = WritingMode::VerticalRl;
        doc.node_mut(root).style.direction = Direction::Rtl;

        let space = root_constraint_space(&doc, 800.0, 600.0);
        assert!(!space.writing_direction.is_horizontal());
        assert!(space.writing_direction.is_flipped_blocks());
        assert!(space.writing_direction.is_rtl());
        assert_eq!(space.available_inline_size, LayoutUnit::from_i32(600));
        assert_eq!(space.available_block_size, LayoutUnit::from_i32(800));
    }

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
        let mut surface = render_to_surface(
            &doc,
            openui_geometry::ViewportMetrics::from_logical_size(200 as f64, 200 as f64, 1.0)
                .unwrap(),
        )
        .unwrap();
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

        let mut surface = render_to_surface(
            &doc,
            openui_geometry::ViewportMetrics::from_logical_size(400 as f64, 300 as f64, 1.0)
                .unwrap(),
        )
        .unwrap();
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

        let mut surface = render_to_surface(
            &doc,
            openui_geometry::ViewportMetrics::from_logical_size(400 as f64, 300 as f64, 1.0)
                .unwrap(),
        )
        .unwrap();
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
        render_to_png(
            &doc,
            openui_geometry::ViewportMetrics::from_logical_size(200 as f64, 200 as f64, 1.0)
                .unwrap(),
            path,
        )
        .unwrap();
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
