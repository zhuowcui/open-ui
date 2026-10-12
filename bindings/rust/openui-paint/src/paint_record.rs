//! Paint operations retained with a content picture for analytic tile decisions.
//!
//! Unsupported operations keep ordinary picture raster. No output pixels are
//! inspected to choose a color. Eligible single-draw alpha groups are folded
//! before picture recording using Chromium's float-alpha operation rule.

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
    static OPACITY_SCOPES: RefCell<Vec<OpacityState>> = const { RefCell::new(Vec::new()) };
}

struct PendingOpacity {
    bounds: Option<Rect>,
    alpha: f32,
    base_save_count: usize,
    draw: Option<(Rect, Paint)>,
}

struct OpacityState {
    canvas: usize,
    pending: Option<PendingOpacity>,
}

/// A pending operation must not outlive the canvas. Nested fragment traversal
/// on the same canvas shares its outer scope; another recording owns its own.
pub(crate) struct OpacityScope<'a> {
    canvas: &'a SkiaCanvas,
    owner: bool,
}

impl<'a> OpacityScope<'a> {
    fn new(canvas: &'a SkiaCanvas) -> Self {
        let id = canvas_id(canvas);
        let owner = OPACITY_SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            if scopes.iter().any(|scope| scope.canvas == id) {
                false
            } else {
                scopes.push(OpacityState {
                    canvas: id,
                    pending: None,
                });
                true
            }
        });
        Self { canvas, owner }
    }
}

impl Drop for OpacityScope<'_> {
    fn drop(&mut self) {
        if self.owner {
            // An unfinished group keeps its original saveLayer and save count.
            // This also applies during unwinding. Never retain a canvas pointer
            // for later dereferencing after this borrowed lifetime ends.
            materialize_opacity(self.canvas);
            OPACITY_SCOPES.with(|scopes| {
                let mut scopes = scopes.borrow_mut();
                let index = scopes
                    .iter()
                    .position(|scope| scope.canvas == canvas_id(self.canvas))
                    .expect("owned opacity scope");
                scopes.remove(index);
            });
        }
    }
}

fn canvas_id(canvas: &SkiaCanvas) -> usize {
    canvas as *const SkiaCanvas as usize
}

fn take_pending_opacity(canvas: &SkiaCanvas) -> Option<PendingOpacity> {
    OPACITY_SCOPES.with(|scopes| {
        scopes
            .borrow_mut()
            .iter_mut()
            .find(|scope| scope.canvas == canvas_id(canvas))
            .and_then(|scope| scope.pending.take())
    })
}

fn materialize_opacity(canvas: &SkiaCanvas) {
    if let Some(pending) = take_pending_opacity(canvas) {
        // No state operation has occurred since the provisional Save. Replace
        // it with the original ordinary-format layer before the first operation
        // that disqualifies SaveLayerAlpha -> Draw -> Restore folding.
        debug_assert_eq!(canvas.save_count(), pending.base_save_count + 1);
        canvas.restore();
        canvas.save_layer_alpha_f(pending.bounds, pending.alpha);
        if let Some((rect, paint)) = pending.draw {
            canvas.draw_rect(rect, &paint);
        }
    }
}

fn restore_pending_opacity(canvas: &SkiaCanvas) -> bool {
    let Some(pending) = take_pending_opacity(canvas) else {
        return false;
    };
    debug_assert_eq!(canvas.save_count(), pending.base_save_count + 1);
    if let Some((rect, mut paint)) = pending.draw {
        // cc::ScopedRasterFlags multiplies the original float alpha. Do this
        // before recording the picture, avoiding SkRecord's packed-alpha fold.
        paint.set_alpha_f(paint.alpha_f() * pending.alpha);
        canvas.draw_rect(rect, &paint);
    }
    canvas.restore();
    true
}

fn supports_folding_alpha(paint: &Paint) -> bool {
    // Exact cc::PaintFlags::SupportsFoldingAlpha predicate. SkPaint has no
    // PaintFlags looper; that operation is untracked and cannot reach this path.
    paint.as_blend_mode() == Some(BlendMode::SrcOver)
        && paint.color_filter().is_none()
        && paint.image_filter().is_none()
}

fn defer_opacity_rect(canvas: &SkiaCanvas, rect: Rect, paint: &Paint) -> bool {
    OPACITY_SCOPES.with(|scopes| {
        let mut scopes = scopes.borrow_mut();
        let pending = scopes
            .iter_mut()
            .find(|scope| scope.canvas == canvas_id(canvas))
            .and_then(|scope| scope.pending.as_mut());
        if let Some(pending) = pending {
            if pending.draw.is_none() && supports_folding_alpha(paint) {
                pending.draw = Some((rect, paint.clone()));
                return true;
            }
        }
        false
    })
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
    /// Used only by the tracking facade. Escaping to Skia must flush first.
    fn raw_tracking_target(&self) -> &SkiaCanvas;
    fn raw_untracked(&self) -> &SkiaCanvas;
}

impl PaintCanvas for SkiaCanvas {
    fn raw_tracking_target(&self) -> &SkiaCanvas {
        self
    }
    fn raw_untracked(&self) -> &SkiaCanvas {
        materialize_opacity(self);
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
    fn query_target(&self) -> &SkiaCanvas {
        let canvas = self.raw_tracking_target();
        let bounded_layer = OPACITY_SCOPES.with(|scopes| {
            scopes.borrow().iter().any(|scope| {
                scope.canvas == canvas_id(canvas)
                    && scope
                        .pending
                        .as_ref()
                        .is_some_and(|pending| pending.bounds.is_some())
            })
        });
        // A bounded Skia layer may narrow its device bounds. Preserve its
        // original answers if a draw algorithm asks for those bounds.
        if bounded_layer {
            materialize_opacity(canvas);
        }
        canvas
    }
    pub(crate) fn opacity_scope(&self) -> OpacityScope<'_> {
        OpacityScope::new(self.raw_tracking_target())
    }
    pub(crate) fn save_layer_alpha_f(&self, bounds: impl Into<Option<Rect>>, alpha: f32) -> usize {
        let c = self.raw_tracking_target();
        materialize_opacity(c);
        let bounds = bounds.into();
        // The original record still contains a layer: CompositeIterator's
        // solid-color analyzer rejects it even if raster playback can fold it.
        unsupported(c);
        // SkCanvas handles alpha >= 1 before multiplying a draw's alpha, and
        // SkPaint pins negative/NaN alpha. Preserve those original semantics
        // for inputs outside Chromium's normalized opacity operation contract.
        if !alpha.is_finite() || !(0.0..=1.0).contains(&alpha) {
            return c.save_layer_alpha_f(bounds, alpha);
        }
        let deferred = OPACITY_SCOPES.with(|scopes| {
            let mut scopes = scopes.borrow_mut();
            if let Some(scope) = scopes.iter_mut().find(|scope| scope.canvas == canvas_id(c)) {
                let base_save_count = c.save();
                scope.pending = Some(PendingOpacity {
                    bounds,
                    alpha,
                    base_save_count,
                    draw: None,
                });
                Some(base_save_count)
            } else {
                None
            }
        });
        deferred.unwrap_or_else(|| c.save_layer_alpha_f(bounds, alpha))
    }
    pub(crate) fn save(&self) -> usize {
        let c = self.raw_untracked();
        capture(c, || Operation::Save);
        c.save()
    }
    pub(crate) fn restore(&self) -> &Self {
        let c = self.raw_tracking_target();
        if restore_pending_opacity(c) {
            return self;
        }
        capture(c, || Operation::Restore);
        c.restore();
        self
    }
    pub(crate) fn restore_to_count(&self, count: usize) -> &Self {
        let c = self.raw_tracking_target();
        if count < c.save_count() {
            restore_pending_opacity(c);
        }
        capture(c, || Operation::RestoreTo(count));
        c.restore_to_count(count);
        self
    }
    pub(crate) fn save_count(&self) -> usize {
        self.raw_tracking_target().save_count()
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
        let c = self.raw_tracking_target();
        if defer_opacity_rect(c, rect, paint) {
            return self;
        }
        materialize_opacity(c);
        capture(c, || Operation::Rect(rect, paint.clone()));
        c.draw_rect(rect, paint);
        self
    }
    pub(crate) fn local_clip_bounds(&self) -> Option<Rect> {
        self.query_target().local_clip_bounds()
    }
    pub(crate) fn local_to_device_as_3x3(&self) -> Matrix {
        self.query_target().local_to_device_as_3x3()
    }
    pub(crate) fn is_clip_rect(&self) -> bool {
        self.query_target().is_clip_rect()
    }
    pub(crate) fn device_clip_bounds(&self) -> Option<skia_safe::IRect> {
        self.query_target().device_clip_bounds()
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

#[cfg(test)]
mod opacity_tests {
    use super::*;
    use skia_safe::{Color, ImageInfo};

    fn red(alpha: f32) -> Paint {
        let mut paint = Paint::default();
        paint.set_color4f(Color4f::new(1.0, 0.0, 0.0, alpha), None);
        paint
    }

    fn render(draw: impl FnOnce(&SkiaCanvas)) -> Vec<u8> {
        let mut surface = surfaces::raster_n32_premul((8, 8)).unwrap();
        surface.canvas().clear(Color::from_rgb(0, 128, 0));
        draw(surface.canvas());
        surface
            .image_snapshot()
            .peek_pixels()
            .unwrap()
            .bytes()
            .unwrap()
            .to_vec()
    }

    fn tracked(draw: impl FnOnce(&dyn PaintCanvas)) -> Vec<u8> {
        render(|canvas| {
            let tracked: &dyn PaintCanvas = canvas;
            let _scope = tracked.opacity_scope();
            draw(tracked);
        })
    }

    #[test]
    fn one_src_over_rect_multiplies_float_alpha_before_raster() {
        let rect = Rect::from_xywh(0.0, 0.0, 8.0, 8.0);
        for (paint_alpha, layer_alpha) in [(1.0, 0.1), (0.7, 0.3), (0.0, 0.2)] {
            let expected = render(|canvas| {
                canvas.draw_rect(rect, &red(paint_alpha * layer_alpha));
            });
            let actual = tracked(|canvas| {
                let count = canvas.save_layer_alpha_f(None, layer_alpha);
                assert_eq!(canvas.save_count(), count + 1);
                canvas.draw_rect(rect, &red(paint_alpha));
                // State queries do not insert a paint operation.
                assert!(canvas.local_clip_bounds().is_some());
                assert_eq!(canvas.local_to_device_as_3x3(), Matrix::new_identity());
                canvas.restore();
                assert_eq!(canvas.save_count(), count);
            });
            assert_eq!(actual, expected);
            if paint_alpha == 1.0 {
                // Native N32 bytes are BGRA: this is the direct-paint value
                // observed in every required Chromium rectangle profile.
                assert_eq!(&actual[..4], &[0, 115, 26, 255]);
            }
        }
    }

    #[test]
    fn non_normalized_alpha_uses_original_skia_clamp_and_layer_semantics() {
        let rect = Rect::from_xywh(0.0, 0.0, 8.0, 8.0);
        for alpha in [-1.0, 2.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let expected = render(|canvas| {
                canvas.save_layer_alpha_f(None, alpha);
                canvas.draw_rect(rect, &red(0.5));
                canvas.restore();
            });
            let actual = tracked(|canvas| {
                let count = canvas.save_layer_alpha_f(None, alpha);
                assert_eq!(canvas.save_count(), count + 1);
                canvas.draw_rect(rect, &red(0.5));
                canvas.restore();
                assert_eq!(canvas.save_count(), count);
            });
            assert_eq!(actual, expected, "ordinary alpha fallback: {alpha}");
        }
    }

    #[test]
    fn second_draw_and_ineligible_blend_keep_the_original_layer() {
        let rect = Rect::from_xywh(0.0, 0.0, 8.0, 8.0);
        for mode in [BlendMode::SrcOver, BlendMode::Src, BlendMode::Multiply] {
            let mut second = red(0.6);
            second.set_blend_mode(mode);
            let expected = render(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                canvas.draw_rect(rect, &red(1.0));
                canvas.draw_rect(Rect::from_xywh(2.0, 1.0, 4.0, 5.0), &second);
                canvas.restore();
            });
            let actual = tracked(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                canvas.draw_rect(rect, &red(1.0));
                canvas.draw_rect(Rect::from_xywh(2.0, 1.0, 4.0, 5.0), &second);
                canvas.restore();
            });
            assert_eq!(actual, expected);
        }
        let mut source = red(1.0);
        source.set_blend_mode(BlendMode::Src);
        assert_eq!(
            tracked(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                canvas.draw_rect(rect, &source);
                canvas.restore();
            }),
            render(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                canvas.draw_rect(rect, &source);
                canvas.restore();
            })
        );
    }

    #[test]
    fn state_operations_and_unknown_draws_materialize_before_observation() {
        let rect = Rect::from_xywh(0.0, 0.0, 8.0, 8.0);
        for operation in 0..4 {
            let expected = render(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                canvas.draw_rect(rect, &red(1.0));
                match operation {
                    0 => {
                        canvas.save();
                        canvas.restore();
                    }
                    1 => {
                        canvas.clip_rect(
                            Rect::from_xywh(0.0, 0.0, 4.0, 8.0),
                            ClipOp::Intersect,
                            true,
                        );
                    }
                    2 => {
                        canvas.translate((1.0, 0.5));
                    }
                    _ => {
                        canvas.draw_oval(Rect::from_xywh(1.0, 1.0, 4.0, 4.0), &red(0.5));
                    }
                }
                canvas.restore();
            });
            assert_eq!(
                tracked(|canvas| {
                    canvas.save_layer_alpha_f(None, 0.2);
                    canvas.draw_rect(rect, &red(1.0));
                    match operation {
                        0 => {
                            canvas.save();
                            canvas.restore();
                        }
                        1 => {
                            canvas.clip_rect(
                                Rect::from_xywh(0.0, 0.0, 4.0, 8.0),
                                ClipOp::Intersect,
                                true,
                            );
                        }
                        2 => {
                            canvas.translate((1.0, 0.5));
                        }
                        _ => {
                            canvas.draw_oval(Rect::from_xywh(1.0, 1.0, 4.0, 4.0), &red(0.5));
                        }
                    }
                    canvas.restore();
                }),
                expected
            );
        }
        let info = ImageInfo::new_n32_premul((8, 8), None);
        let mut expected_readback = vec![0; 8 * 8 * 4];
        let expected = render(|canvas| {
            canvas.save_layer_alpha_f(None, 0.2);
            canvas.draw_rect(rect, &red(1.0));
            assert!(canvas.read_pixels(&info, &mut expected_readback, 8 * 4, (0, 0)));
            canvas.restore();
        });
        let mut actual_readback = vec![0; 8 * 8 * 4];
        let actual = tracked(|canvas| {
            canvas.save_layer_alpha_f(None, 0.2);
            canvas.draw_rect(rect, &red(1.0));
            // Deref flushes the original group before exposing a Skia readback.
            assert!(canvas.read_pixels(&info, &mut actual_readback, 8 * 4, (0, 0)));
            canvas.restore();
        });
        assert_eq!(actual_readback, expected_readback);
        assert_eq!(actual, expected);
    }

    #[test]
    fn nested_scopes_restore_to_count_and_empty_groups_preserve_stack_state() {
        assert_eq!(
            tracked(|canvas| {
                let base = canvas.save_count();
                let count = canvas.save_layer_alpha_f(None, 0.1);
                canvas.restore_to_count(count + 1); // No-op does not consume a draw.
                {
                    let _nested = canvas.opacity_scope();
                    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 8.0, 8.0), &red(1.0));
                }
                assert_eq!(canvas.save_count(), base + 1);
                canvas.restore_to_count(base);
                canvas.save_layer_alpha_f(None, 0.3);
                canvas.restore_to_count(0); // Skia clamps the target to the root save.
                assert_eq!(canvas.save_count(), base);
            }),
            render(|canvas| {
                canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 8.0, 8.0), &red(0.1));
            })
        );
    }

    #[test]
    fn unfinished_scope_flushes_without_closing_the_callers_layer() {
        assert_eq!(
            render(|canvas| {
                let base = canvas.save_count();
                {
                    let facade: &dyn PaintCanvas = canvas;
                    let _scope = facade.opacity_scope();
                    facade.save_layer_alpha_f(None, 0.2);
                    facade.draw_rect(Rect::from_xywh(0.0, 0.0, 8.0, 8.0), &red(1.0));
                }
                assert_eq!(canvas.save_count(), base + 1);
                canvas.restore();
            }),
            render(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 8.0, 8.0), &red(1.0));
                canvas.restore();
            })
        );
    }

    #[test]
    fn another_canvas_and_unwinding_do_not_steal_or_leak_pending_operations() {
        let rect = Rect::from_xywh(0.0, 0.0, 8.0, 8.0);
        let expected = render(|canvas| {
            canvas.draw_rect(rect, &red(0.1));
        });
        assert_eq!(
            tracked(|canvas| {
                canvas.save_layer_alpha_f(None, 0.1);
                canvas.draw_rect(rect, &red(1.0));
                let inner = tracked(|other| {
                    other.save_layer_alpha_f(None, 0.2);
                    other.draw_rect(rect, &red(1.0));
                    other.restore();
                });
                assert_eq!(
                    inner,
                    render(|other| {
                        other.draw_rect(rect, &red(0.2));
                    })
                );
                canvas.restore();
            }),
            expected
        );

        assert_eq!(
            render(|canvas| {
                let base = canvas.save_count();
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let facade: &dyn PaintCanvas = canvas;
                    let _scope = facade.opacity_scope();
                    facade.save_layer_alpha_f(None, 0.2);
                    facade.draw_rect(rect, &red(1.0));
                    panic!("unwind with an unfinished opacity group");
                }));
                assert!(result.is_err());
                assert_eq!(canvas.save_count(), base + 1);
                canvas.restore();
                // A new scope on this canvas must find no leaked predecessor.
                let facade: &dyn PaintCanvas = canvas;
                let _scope = facade.opacity_scope();
                assert!(OPACITY_SCOPES.with(|scopes| scopes
                    .borrow()
                    .iter()
                    .find(|scope| scope.canvas == canvas_id(canvas))
                    .unwrap()
                    .pending
                    .is_none()));
            }),
            render(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                canvas.draw_rect(rect, &red(1.0));
                canvas.restore();
            })
        );
    }

    #[test]
    fn raster_folding_does_not_admit_a_layer_to_solid_color_analysis() {
        render(|canvas| {
            let facade: &dyn PaintCanvas = canvas;
            let _scope = facade.opacity_scope();
            let recording = RecordingScope::new(canvas);
            facade.save_layer_alpha_f(None, 0.2);
            facade.draw_rect(Rect::from_xywh(0.0, 0.0, 8.0, 8.0), &red(1.0));
            facade.restore();
            assert!(!recording.snapshot().supports_analysis());
        });
    }
}
