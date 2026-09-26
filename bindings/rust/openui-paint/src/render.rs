//! Render pipeline — layout a Document and paint to PNG or Skia surface.
//!
//! This is the main public API for headless rendering: build a Document,
//! set styles, call `render_to_png()`, and get a pixel-perfect PNG.

use openui_dom::Document;
use openui_geometry::{
    LayoutUnit, RasterConfiguration, RasterPixelGeometry, TextEdging, ViewportMetrics,
};
use openui_layout::{block_layout, ConstraintSpace, Fragment};
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    surfaces, Color as SkColor, ColorSpace, EncodedImageFormat, FilterMode, ImageInfo, Paint,
    Picture, PictureRecorder, PixelGeometry, Rect, SamplingOptions, Surface, SurfaceProps,
    SurfacePropsFlags,
};
use std::sync::Arc;

use crate::painter::paint_fragment;

// Chromium 147's cc::LayerTreeSettings::max_untiled_layer_size.
const MAX_UNTILED_LAYER_SIZE: i32 = 512;

fn should_replay_untiled(direct_replay: bool, width: i32, height: i32) -> bool {
    direct_replay || (width <= MAX_UNTILED_LAYER_SIZE && height <= MAX_UNTILED_LAYER_SIZE)
}

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
    pub raster_configuration: RasterConfiguration,
    pub(crate) lcd_surface: bool,
    pub(crate) direct_replay: bool,
    /// Retains immutable registered bytes for every face referenced by a
    /// scene, even if the live document unregisters that face immediately.
    pub(crate) retained_font_bytes: Arc<[Arc<[u8]>]>,
}

impl RecordedPicture {
    pub fn retained_font_face_count(&self) -> usize {
        self.retained_font_bytes.len()
    }

    pub fn uses_lcd_surface(&self) -> bool {
        self.lcd_surface
    }
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
    crate::painter::reset_picture_paint_state();
    let width = logical_dimension(viewport.logical_width())?;
    let height = logical_dimension(viewport.logical_height())?;
    let raster_configuration = doc.raster_configuration();
    let lcd_surface = raster_configuration.author_text.edging == TextEdging::SubpixelAntiAlias
        || doc.uses_native_control_text()
        || (raster_configuration.author_text.edging == TextEdging::Alias
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
        raster_configuration,
        lcd_surface,
        direct_replay: has_promoted_non_axis_transform(fragment, doc),
        retained_font_bytes: doc.font_collection().retained_face_bytes(),
    })
}

/// Rasterize an immutable recording using the exact headless tiling policy.
pub fn rasterize_picture(recording: &RecordedPicture) -> Result<Surface, String> {
    let width = i32::try_from(recording.viewport.physical_width())
        .map_err(|_| "physical viewport width exceeds Skia limit".to_string())?;
    let height = i32::try_from(recording.viewport.physical_height())
        .map_err(|_| "physical viewport height exceeds Skia limit".to_string())?;
    let scale = recording.viewport.device_scale_factor() as f32;
    let mut surface = create_raster_surface(
        width,
        height,
        recording.raster_configuration,
        recording.lcd_surface,
    )
    .ok_or_else(|| "Failed to create Skia surface".to_string())?;
    let canvas_color = SkColor::WHITE;
    surface.canvas().clear(canvas_color);
    // Chromium 147 keeps layers no larger than 512x512 physical pixels
    // untiled (`LayerTreeSettings::max_untiled_layer_size`). Replaying those
    // pictures through a synthetic 256px tile changes analytic rrect coverage
    // near the tile clip even though the assembled pixels are copied exactly.
    if should_replay_untiled(recording.direct_replay, width, height) {
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
            let mut tile = create_raster_surface(
                TILE_SIZE,
                TILE_SIZE,
                recording.raster_configuration,
                recording.lcd_surface,
            )
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
                // Source and destination are identical integral pixel spans.
                // Nearest is a bit-preserving copy; linear filtering here
                // changes edge coverage at every compositor tile boundary.
                SamplingOptions::from(FilterMode::Nearest),
                &Paint::default(),
            );
        }
    }
    let exact_physical_height = recording.viewport.logical_height() as f32 * scale;
    if height as f32 > exact_physical_height + f32::EPSILON {
        // CDP/Chromium closes a rounded-up physical viewport by repeating the
        // first canvas scanline into the fractional terminal scanline. This
        // is observable when the first row contains edge-to-edge ink. Apply
        // the same deterministic closure after tiled replay.
        let image = surface.image_snapshot();
        surface.canvas().draw_image_rect_with_sampling_options(
            image,
            Some((
                &Rect::from_xywh(0.0, 0.0, width as f32, 1.0),
                SrcRectConstraint::Strict,
            )),
            Rect::from_xywh(0.0, height as f32 - 1.0, width as f32, 1.0),
            SamplingOptions::from(FilterMode::Nearest),
            &Paint::default(),
        );
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

pub fn raster_surface_properties(
    configuration: RasterConfiguration,
    lcd_surface: bool,
) -> Option<SurfaceProps> {
    if !lcd_surface {
        return None;
    }
    Some(SurfaceProps::new_with_text_properties(
        SurfacePropsFlags::default(),
        match configuration.pixel_geometry {
            RasterPixelGeometry::Unknown => PixelGeometry::Unknown,
            RasterPixelGeometry::RgbHorizontal => PixelGeometry::RGBH,
            RasterPixelGeometry::BgrHorizontal => PixelGeometry::BGRH,
            RasterPixelGeometry::RgbVertical => PixelGeometry::RGBV,
            RasterPixelGeometry::BgrVertical => PixelGeometry::BGRV,
        },
        configuration.contrast(),
        configuration.gamma(),
    ))
}

fn create_raster_surface(
    width: i32,
    height: i32,
    configuration: RasterConfiguration,
    lcd_surface: bool,
) -> Option<Surface> {
    // Chromium composites CSS colors and color-managed replaced images into an
    // sRGB destination. An untagged Skia surface skips conversion for images
    // carrying an embedded ICC profile, leaving their encoded source samples
    // in the output bitmap.
    let image_info = ImageInfo::new_n32_premul((width, height), Some(ColorSpace::new_srgb()));
    if let Some(props) = raster_surface_properties(configuration, lcd_surface) {
        // Chromium's Linux Skia build pins these in //skia/BUILD.gn. Passing
        // them explicitly avoids inheriting the independently-built skia-safe
        // defaults (0.5 contrast and sRGB gamma).
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
    fn chromium_sized_small_layers_replay_untiled() {
        assert!(should_replay_untiled(false, 512, 512));
        assert!(!should_replay_untiled(false, 513, 512));
        assert!(!should_replay_untiled(false, 512, 513));
        assert!(should_replay_untiled(true, 4096, 4096));
    }

    #[test]
    fn root_constraint_uses_computed_writing_direction() {
        let mut doc = Document::new();
        let root = doc.root();
        doc.update_resolved_style(root, |style| style.writing_mode = WritingMode::VerticalRl);
        doc.update_resolved_style(root, |style| style.direction = Direction::Rtl);

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
        doc.update_resolved_style(div, |style| style.display = Display::Block);
        doc.update_resolved_style(div, |style| style.width = Length::px(100.0));
        doc.update_resolved_style(div, |style| style.height = Length::px(100.0));
        doc.update_resolved_style(div, |style| style.background_color = Color::RED);
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
        doc.update_resolved_style(outer, |style| style.display = Display::Block);
        doc.update_resolved_style(outer, |style| style.width = Length::px(200.0));
        doc.update_resolved_style(outer, |style| style.padding_top = Length::px(10.0));
        doc.update_resolved_style(outer, |style| style.padding_left = Length::px(10.0));
        doc.update_resolved_style(outer, |style| style.padding_right = Length::px(10.0));
        doc.update_resolved_style(outer, |style| style.padding_bottom = Length::px(10.0));
        doc.update_resolved_style(outer, |style| style.background_color = Color::BLUE);
        doc.append_child(vp, outer);

        // Inner: red box
        let inner = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(inner, |style| style.display = Display::Block);
        doc.update_resolved_style(inner, |style| style.height = Length::px(50.0));
        doc.update_resolved_style(inner, |style| style.background_color = Color::RED);
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
        doc.update_resolved_style(div, |style| style.display = Display::Block);
        doc.update_resolved_style(div, |style| style.width = Length::px(150.0));
        doc.update_resolved_style(div, |style| style.height = Length::px(100.0));
        doc.update_resolved_style(div, |style| {
            style.background_color = Color::from_rgba8(200, 200, 200, 255)
        });
        doc.update_resolved_style(div, |style| style.border_top_width = 3);
        doc.update_resolved_style(div, |style| style.border_right_width = 3);
        doc.update_resolved_style(div, |style| style.border_bottom_width = 3);
        doc.update_resolved_style(div, |style| style.border_left_width = 3);
        doc.update_resolved_style(div, |style| style.border_top_style = BorderStyle::Solid);
        doc.update_resolved_style(div, |style| style.border_right_style = BorderStyle::Solid);
        doc.update_resolved_style(div, |style| style.border_bottom_style = BorderStyle::Solid);
        doc.update_resolved_style(div, |style| style.border_left_style = BorderStyle::Solid);
        doc.update_resolved_style(div, |style| {
            style.border_top_color = StyleColor::Resolved(Color::BLACK)
        });
        doc.update_resolved_style(div, |style| {
            style.border_right_color = StyleColor::Resolved(Color::BLACK)
        });
        doc.update_resolved_style(div, |style| {
            style.border_bottom_color = StyleColor::Resolved(Color::BLACK)
        });
        doc.update_resolved_style(div, |style| {
            style.border_left_color = StyleColor::Resolved(Color::BLACK)
        });
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
        doc.update_resolved_style(div, |style| style.display = Display::Block);
        doc.update_resolved_style(div, |style| style.width = Length::px(100.0));
        doc.update_resolved_style(div, |style| style.height = Length::px(100.0));
        doc.update_resolved_style(div, |style| {
            style.background_color = Color::from_rgba8(50, 150, 50, 255)
        });
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
        let surface =
            create_raster_surface(16, 16, RasterConfiguration::chromium_linux_lcd(), true).unwrap();
        let props = surface.props();
        assert_eq!(props.pixel_geometry(), PixelGeometry::RGBH);
        assert_eq!(props.text_contrast(), 0.2);
        assert_eq!(props.text_gamma(), 1.2);
    }

    #[test]
    fn legacy_surface_keeps_unknown_pixel_geometry() {
        let surface = create_raster_surface(16, 16, RasterConfiguration::default(), false).unwrap();
        assert_eq!(surface.props().pixel_geometry(), PixelGeometry::Unknown);
    }
}
