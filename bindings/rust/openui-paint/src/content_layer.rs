//! Immutable document content and the metadata used before layer raster.

use openui_dom::NodeId;
use openui_style::Color;
use skia_safe::{BlendMode, Canvas, ClipOp, Contains, Picture, Rect};

/// A retained scrolling content recording, independent of viewport controls.
#[derive(Clone)]
pub struct RecordedContentLayer {
    pub node_id: NodeId,
    pub bounds: Rect,
    pub background_color: Color,
    pub rect_known_to_be_opaque: Rect,
    pub contents_opaque: bool,
    pub scroll_translation: (f32, f32),
    pub picture: Picture,
    pub(crate) paint_record: crate::paint_record::LayerPaintRecord,
    // A caller may clone a content layer independently of its parent scene.
    pub(crate) _font_cache_lifetime: openui_text::font::FontCacheLifetime,
}

impl RecordedContentLayer {
    pub(crate) fn replay(&self, canvas: &Canvas, scale: f64) {
        // cc::RasterSource clears the boundary of opaque recordings before
        // display-list playback. At non-unit raster scales a boundary texel
        // can receive partial coverage despite the layer's opaque interior.
        // Direct picture composition has no sampled texel outside the quad;
        // the external texture guard must not be painted into the frame.
        if self.contents_opaque {
            let scale = scale as f32;
            let physical = Rect::from_ltrb(
                self.bounds.left * scale,
                self.bounds.top * scale,
                self.bounds.right * scale,
                self.bounds.bottom * scale,
            );
            let left_partial = physical.left.fract() != 0.0;
            let top_partial = physical.top.fract() != 0.0;
            let trailing_partial = scale != 1.0;
            let trailing_guard = if trailing_partial { 1.0 } else { 0.0 };
            if left_partial || top_partial || trailing_partial {
                let outer = Rect::from_ltrb(
                    physical.left.floor(),
                    physical.top.floor(),
                    physical.right.ceil(),
                    physical.bottom.ceil(),
                );
                let inner = Rect::from_ltrb(
                    outer.left + if left_partial { 1.0 } else { 0.0 },
                    outer.top + if top_partial { 1.0 } else { 0.0 },
                    physical.right.ceil() - trailing_guard,
                    physical.bottom.ceil() - trailing_guard,
                );
                let logical = |rect: Rect| {
                    Rect::from_ltrb(
                        rect.left / scale,
                        rect.top / scale,
                        rect.right / scale,
                        rect.bottom / scale,
                    )
                };
                // A picture recorder's logical clip query can discard a
                // subpixel band which is visible after physical scaling.
                // Select visible tiles from the parent clip before recording
                // that band; raster playback owns its physical coverage.
                let visible = canvas.local_clip_bounds();
                canvas.save();
                canvas.clip_rect(logical(outer), ClipOp::Intersect, false);
                if !inner.is_empty() {
                    canvas.clip_rect(logical(inner), ClipOp::Difference, false);
                }
                self.clear_boundary_tiles(canvas, scale, physical, inner, visible);
                canvas.restore();
            }
        }
        canvas.draw_picture(&self.picture, None, None);
    }

    fn clear_boundary_tiles(
        &self,
        canvas: &Canvas,
        scale: f32,
        physical: Rect,
        inner: Rect,
        visible: Option<Rect>,
    ) {
        let background = skia_safe::Color4f::new(
            self.background_color.r,
            self.background_color.g,
            self.background_color.b,
            self.background_color.a,
        );
        if !self.paint_record.supports_analysis() {
            canvas.draw_color(background, BlendMode::Src);
            return;
        }
        let Some(visible) = visible else {
            canvas.draw_color(background, BlendMode::Src);
            return;
        };
        // Pinned Linux CPU tilings use 256 texels with one border texel.
        // cc::TilingData partitions geometry at 255, 509, ... while each
        // tile's analysis includes its border. Solid-color tiles bypass
        // normal raster playback and therefore its background-hint clear.
        let axis = |length: f32, visible_lo: f32, visible_hi: f32| {
            let size = (f64::from(length.ceil()).min(f64::from(i32::MAX))) as i64;
            let count = (1 + (size - 3) / 254).max(1);
            let first = (((visible_lo - 1.0) / 254.0).floor() as i64).clamp(0, count - 1);
            let last = ((visible_hi / 254.0).ceil() as i64).clamp(first, count - 1);
            (first..=last)
                .map(|index| {
                    let lo = 254 * index + i64::from(index != 0);
                    let hi = (254 * (index + 1) + 1 + i64::from(index + 1 == count)).min(size);
                    (
                        lo as f32,
                        hi as f32,
                        (254 * index) as f32,
                        (254 * index + 256).min(size) as f32,
                    )
                })
                .collect::<Vec<_>>()
        };
        let columns = axis(
            self.bounds.width() * scale,
            visible.left * scale - physical.left,
            visible.right * scale - physical.left,
        );
        let rows = axis(
            self.bounds.height() * scale,
            visible.top * scale - physical.top,
            visible.bottom * scale - physical.top,
        );
        for &(top, bottom, analysis_top, analysis_bottom) in &rows {
            for &(left, right, analysis_left, analysis_right) in &columns {
                let tile = Rect::from_ltrb(
                    physical.left + left,
                    physical.top + top,
                    physical.left + right,
                    physical.top + bottom,
                );
                if !inner.is_empty() && inner.contains(tile) {
                    continue;
                }
                let query = Rect::from_ltrb(
                    (self.bounds.left + analysis_left / scale)
                        .floor()
                        .max(self.bounds.left),
                    (self.bounds.top + analysis_top / scale)
                        .floor()
                        .max(self.bounds.top),
                    (self.bounds.left + analysis_right / scale)
                        .ceil()
                        .min(self.bounds.right),
                    (self.bounds.top + analysis_bottom / scale)
                        .ceil()
                        .min(self.bounds.bottom),
                );
                let color = self
                    .paint_record
                    .solid_color(query)
                    .filter(|color| color.a == 1.0)
                    .unwrap_or(background);
                canvas.save();
                canvas.clip_rect(
                    Rect::from_ltrb(
                        tile.left / scale,
                        tile.top / scale,
                        tile.right / scale,
                        tile.bottom / scale,
                    ),
                    ClipOp::Intersect,
                    false,
                );
                canvas.draw_color(color, BlendMode::Src);
                canvas.restore();
            }
        }
    }
}
