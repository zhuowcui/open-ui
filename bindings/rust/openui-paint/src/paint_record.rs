//! Paint operations retained with a content picture for analytic tile decisions.
//!
//! Unsupported operations keep ordinary picture raster. No output pixels are
//! inspected to choose a color, and all calls reach the original Skia canvas.

use skia_safe::{
    surfaces, BlendMode, Canvas as SkiaCanvas, ClipOp, Color4f, Contains, Matrix, Paint,
    PaintStyle, Rect, Vector,
};
use std::{cell::RefCell, ops::Deref};

#[derive(Clone)]
enum Operation {
    Save,
    Restore,
    RestoreTo(usize),
    Translate(Vector),
    Scale((f32, f32)),
    Concat(Matrix),
    Clip(Rect, ClipOp, bool),
    Rect(Rect, Paint),
}

#[derive(Clone, Default)]
pub(crate) struct LayerPaintRecord {
    operations: Vec<Operation>,
    complete: bool,
}

struct Recording {
    canvas: usize,
    record: LayerPaintRecord,
}

thread_local! {
    static RECORDING: RefCell<Option<Recording>> = const { RefCell::new(None) };
}

pub(crate) struct RecordingScope {
    previous: Option<Recording>,
}

impl RecordingScope {
    pub(crate) fn new(canvas: &SkiaCanvas) -> Self {
        let previous = RECORDING.with(|state| {
            state.replace(Some(Recording {
                canvas: canvas as *const SkiaCanvas as usize,
                record: LayerPaintRecord {
                    operations: Vec::new(),
                    complete: true,
                },
            }))
        });
        Self { previous }
    }

    pub(crate) fn snapshot(&self) -> LayerPaintRecord {
        RECORDING.with(|state| {
            state
                .borrow()
                .as_ref()
                .expect("active paint recording")
                .record
                .clone()
        })
    }
}

impl Drop for RecordingScope {
    fn drop(&mut self) {
        RECORDING.with(|state| {
            state.replace(self.previous.take());
        });
    }
}

fn capture(canvas: &SkiaCanvas, operation: impl FnOnce() -> Operation) {
    RECORDING.with(|state| {
        if let Some(recording) = state.borrow_mut().as_mut() {
            if recording.canvas == canvas as *const SkiaCanvas as usize && recording.record.complete
            {
                recording.record.operations.push(operation());
            }
        }
    });
}

fn unsupported(canvas: &SkiaCanvas) {
    RECORDING.with(|state| {
        if let Some(recording) = state.borrow_mut().as_mut() {
            if recording.canvas == canvas as *const SkiaCanvas as usize {
                recording.record.complete = false;
            }
        }
    });
}

/// Internal canvas facade. Untracked Skia operations conservatively disable
/// analytic color selection for this recording rather than guessing their effect.
pub(crate) trait PaintCanvas {
    fn raw_untracked(&self) -> &SkiaCanvas;
}

impl PaintCanvas for SkiaCanvas {
    fn raw_untracked(&self) -> &SkiaCanvas {
        self
    }
}

impl Deref for dyn PaintCanvas + '_ {
    type Target = SkiaCanvas;
    fn deref(&self) -> &SkiaCanvas {
        let canvas = self.raw_untracked();
        unsupported(canvas);
        canvas
    }
}

impl dyn PaintCanvas + '_ {
    pub(crate) fn save(&self) -> usize {
        let c = self.raw_untracked();
        capture(c, || Operation::Save);
        c.save()
    }
    pub(crate) fn restore(&self) -> &Self {
        let c = self.raw_untracked();
        capture(c, || Operation::Restore);
        c.restore();
        self
    }
    pub(crate) fn restore_to_count(&self, count: usize) -> &Self {
        let c = self.raw_untracked();
        capture(c, || Operation::RestoreTo(count));
        c.restore_to_count(count);
        self
    }
    pub(crate) fn save_count(&self) -> usize {
        self.raw_untracked().save_count()
    }
    pub(crate) fn translate(&self, offset: impl Into<Vector>) -> &Self {
        let offset = offset.into();
        let c = self.raw_untracked();
        capture(c, || Operation::Translate(offset));
        c.translate(offset);
        self
    }
    pub(crate) fn scale(&self, scale: (f32, f32)) -> &Self {
        let c = self.raw_untracked();
        capture(c, || Operation::Scale(scale));
        c.scale(scale);
        self
    }
    pub(crate) fn concat(&self, matrix: &Matrix) -> &Self {
        let c = self.raw_untracked();
        capture(c, || Operation::Concat(*matrix));
        c.concat(matrix);
        self
    }
    pub(crate) fn clip_rect(
        &self,
        rect: impl AsRef<Rect>,
        op: impl Into<Option<ClipOp>>,
        aa: impl Into<Option<bool>>,
    ) -> &Self {
        let rect = *rect.as_ref();
        let op = op.into().unwrap_or(ClipOp::Intersect);
        let aa = aa.into().unwrap_or(false);
        let c = self.raw_untracked();
        capture(c, || Operation::Clip(rect, op, aa));
        c.clip_rect(rect, op, aa);
        self
    }
    pub(crate) fn draw_rect(&self, rect: impl AsRef<Rect>, paint: &Paint) -> &Self {
        let rect = *rect.as_ref();
        let c = self.raw_untracked();
        capture(c, || Operation::Rect(rect, paint.clone()));
        c.draw_rect(rect, paint);
        self
    }
    pub(crate) fn local_clip_bounds(&self) -> Option<Rect> {
        self.raw_untracked().local_clip_bounds()
    }
    pub(crate) fn local_to_device_as_3x3(&self) -> Matrix {
        self.raw_untracked().local_to_device_as_3x3()
    }
    pub(crate) fn is_clip_rect(&self) -> bool {
        self.raw_untracked().is_clip_rect()
    }
    pub(crate) fn device_clip_bounds(&self) -> Option<skia_safe::IRect> {
        self.raw_untracked().device_clip_bounds()
    }
}

impl LayerPaintRecord {
    pub(crate) fn supports_analysis(&self) -> bool {
        self.complete
    }

    pub(crate) fn solid_color(&self, rect: Rect) -> Option<Color4f> {
        if !self.complete || rect.is_empty() {
            return None;
        }
        // Chromium's SolidColorAnalyzer evaluates paint records with a
        // SkNoDrawCanvas over the logical, layer-bounded tile query.
        let mut surface = surfaces::null((rect.width() as i32, rect.height() as i32))?;
        let canvas = surface.canvas();
        canvas.translate((-rect.left, -rect.top));
        canvas.clip_rect(rect, ClipOp::Intersect, false);
        let mut color = Color4f::new(0.0, 0.0, 0.0, 0.0);
        let mut solid = true;
        let mut draws = 0;
        for operation in &self.operations {
            match operation {
                Operation::Save => {
                    canvas.save();
                }
                Operation::Restore => {
                    canvas.restore();
                }
                Operation::RestoreTo(count) => {
                    canvas.restore_to_count(*count);
                }
                Operation::Translate(offset) => {
                    canvas.translate(*offset);
                }
                Operation::Scale(scale) => {
                    canvas.scale(*scale);
                }
                Operation::Concat(matrix) => {
                    canvas.concat(matrix);
                }
                Operation::Clip(rect, op, aa) => {
                    if *op == ClipOp::Difference {
                        return None;
                    }
                    canvas.clip_rect(rect, *op, *aa);
                }
                Operation::Rect(shape, paint) => {
                    if paint.nothing_to_draw() {
                        continue;
                    }
                    // These effects can move ink beyond the rectangle, so its
                    // unexpanded bounds are insufficient for tile rejection.
                    if paint.style() != PaintStyle::Fill
                        || paint.image_filter().is_some()
                        || paint.mask_filter().is_some()
                        || paint.path_effect().is_some()
                    {
                        return None;
                    }
                    let matrix = canvas.local_to_device_as_3x3();
                    let device_shape = matrix.map_rect(shape).0;
                    let Some(clip) = canvas.device_clip_bounds() else {
                        continue;
                    };
                    let clip = Rect::from(clip);
                    if device_shape.right <= clip.left
                        || device_shape.left >= clip.right
                        || device_shape.bottom <= clip.top
                        || device_shape.top >= clip.bottom
                    {
                        continue;
                    }
                    draws += 1;
                    // cc::TileManager uses five analyzed draw operations.
                    if draws > 5 {
                        return None;
                    }
                    let full_clip = canvas.is_clip_rect()
                        && clip.contains(Rect::from_xywh(0.0, 0.0, rect.width(), rect.height()));
                    let covers = matrix.rect_stays_rect()
                        && matrix
                            .invert()
                            .is_some_and(|inverse| shape.contains(inverse.map_rect(clip).0));
                    let Some(mode) = paint.as_blend_mode() else {
                        return None;
                    };
                    let plain = paint.style() == PaintStyle::Fill
                        && paint.shader().is_none()
                        && paint.color_filter().is_none()
                        && paint.image_filter().is_none()
                        && paint.mask_filter().is_none()
                        && paint.path_effect().is_none();
                    if !full_clip
                        || !covers
                        || !plain
                        || !matches!(mode, BlendMode::Src | BlendMode::SrcOver)
                    {
                        solid = false;
                        continue;
                    }
                    let source = paint.color4f();
                    if mode == BlendMode::Src || source.a == 1.0 {
                        color = source;
                        solid = true;
                    } else if solid {
                        let alpha = source.a + color.a * (1.0 - source.a);
                        if alpha == 0.0 {
                            color = Color4f::new(0.0, 0.0, 0.0, 0.0);
                        } else {
                            let sw = source.a / alpha;
                            let dw = color.a * (1.0 - source.a) / alpha;
                            color = Color4f::new(
                                source.r * sw + color.r * dw,
                                source.g * sw + color.g * dw,
                                source.b * sw + color.b * dw,
                                alpha,
                            );
                        }
                    }
                }
            }
        }
        solid.then_some(color)
    }
}
