//! Owned operations observed during the existing fragment paint walk.
//!
//! The canonical canvas remains the query and output target. This stream keeps
//! original operands, including unmodulated float opacity, and never infers a
//! clip from canvas bounds. It is a static capture, not a partitioned scene or
//! a presentation-thread animation plan.

use std::{
    cell::RefCell,
    sync::{Arc, Mutex},
};

use openui_dom::NodeId;
use openui_layout::Fragment;
use skia_safe::canvas::{SaveLayerFlags, SaveLayerRec, SrcRectConstraint};
use skia_safe::{
    BlendMode, Canvas, ClipOp, Color4f, FilterMode, Font, IRect, Image, Matrix, Paint, PaintStyle,
    Path, Picture, PictureRecorder, Point, RRect, Rect, SamplingOptions, TextBlob, Vector, M44,
};

use crate::paint_record::PaintCanvas;

/// Why an observed stream cannot be replayed as a complete recording.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintCaptureIncompleteReason {
    /// A drawing/state operation escaped the audited canvas facade.
    RawCanvasAccess,
    /// Pixel readback has dependencies beyond an immutable operation stream.
    PixelReadback,
    /// A restore reached outside the captured canvas's initial save stack.
    RestoreBeforeCapture,
    /// The source canvas finished at a different save depth.
    UnbalancedSaveStack,
}

/// Capture completeness describes operation coverage, not pixel qualification.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaintCaptureStatus {
    Complete,
    Incomplete(Vec<PaintCaptureIncompleteReason>),
}

/// Source ownership from the actual paint walk. This is not a compositor ID.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintProgramOwner {
    pub node_id: NodeId,
    pub owns_element_box: bool,
}

/// Skia's safe Path is Send but not Sync because it lazily updates caches.
/// Share only an owned clone and serialize its use; never retain the caller's
/// path or publish an unsafe Sync implementation for it.
#[derive(Clone)]
pub(crate) struct OwnedPath(Arc<Mutex<Path>>);

impl OwnedPath {
    pub(crate) fn new(path: &Path) -> Self {
        let mut owned = path.clone();
        owned.update_bounds_cache();
        Self(Arc::new(Mutex::new(owned)))
    }

    fn with_path<T>(&self, draw: impl FnOnce(&Path) -> T) -> T {
        let path = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        draw(&path)
    }
}

#[derive(Clone)]
pub(crate) enum OwnedDraw {
    Clear(Color4f),
    Color(Color4f, BlendMode),
    Rect(Rect, Paint),
    Line(Point, Point, Paint),
    Oval(Rect, Paint),
    Circle(Point, f32, Paint),
    RRect(RRect, Paint),
    RoundRect(Rect, f32, f32, Paint),
    DRRect(RRect, RRect, Paint),
    Path(OwnedPath, Paint),
    Image(
        Image,
        Option<(Rect, SrcRectConstraint)>,
        Rect,
        SamplingOptions,
        Paint,
    ),
    ImageNine(Image, IRect, Rect, FilterMode, Option<Paint>),
    TextBlob(TextBlob, Point, Paint),
    String(String, Point, Font, Paint),
    Picture(Picture, Option<Matrix>, Option<Paint>),
}

impl OwnedDraw {
    pub(crate) fn replay(&self, canvas: &dyn PaintCanvas) {
        match self {
            Self::Clear(color) => {
                canvas.clear(*color);
            }
            Self::Color(color, mode) => {
                canvas.draw_color(*color, *mode);
            }
            Self::Rect(rect, paint) => {
                canvas.draw_rect(rect, paint);
            }
            Self::Line(a, b, paint) => {
                canvas.draw_line(*a, *b, paint);
            }
            Self::Oval(rect, paint) => {
                canvas.draw_oval(rect, paint);
            }
            Self::Circle(point, radius, paint) => {
                canvas.draw_circle(*point, *radius, paint);
            }
            Self::RRect(rect, paint) => {
                canvas.draw_rrect(rect, paint);
            }
            Self::RoundRect(rect, rx, ry, paint) => {
                canvas.draw_round_rect(rect, *rx, *ry, paint);
            }
            Self::DRRect(outer, inner, paint) => {
                canvas.draw_drrect(outer, inner, paint);
            }
            Self::Path(path, paint) => {
                path.with_path(|path| canvas.draw_path(path, paint));
            }
            Self::Image(image, src, dst, sampling, paint) => {
                canvas.draw_image_rect_with_sampling_options(
                    image,
                    src.as_ref().map(|(rect, constraint)| (rect, *constraint)),
                    dst,
                    *sampling,
                    paint,
                );
            }
            Self::ImageNine(image, center, dst, filter, paint) => {
                canvas.draw_image_nine(image, center, dst, *filter, paint.as_ref());
            }
            Self::TextBlob(blob, origin, paint) => {
                canvas.draw_text_blob(blob, *origin, paint);
            }
            Self::String(text, origin, font, paint) => {
                canvas.draw_str(text, *origin, font, paint);
            }
            Self::Picture(picture, matrix, paint) => {
                canvas.draw_picture(picture, matrix.as_ref(), paint.as_ref());
            }
        }
    }

    // These are experimental pictures, not admitted compositor operands.
    // Skia may discard draws outside a recorder's cull rectangle; geometry
    // bounds alone do not prove conservative transformed antialias coverage.
    // Typed replay retains the original operands and never uses this picture.
    fn intrinsic_picture(&self) -> Option<Picture> {
        let (bounds, paint) = match self {
            Self::Rect(rect, paint)
            | Self::Oval(rect, paint)
            | Self::RoundRect(rect, _, _, paint) => (*rect, paint),
            Self::RRect(rect, paint) => (*rect.rect(), paint),
            Self::DRRect(outer, _, paint) => (*outer.rect(), paint),
            _ => return None,
        };
        if paint.style() != PaintStyle::Fill
            || paint.image_filter().is_some()
            || paint.mask_filter().is_some()
            || paint.path_effect().is_some()
            || bounds.is_empty()
            || ![bounds.left, bounds.top, bounds.right, bounds.bottom]
                .iter()
                .all(|v| v.is_finite())
        {
            return None;
        }
        let mut recorder = PictureRecorder::new();
        let canvas: &dyn PaintCanvas = recorder.begin_recording(bounds, false);
        self.replay(canvas);
        recorder.finish_recording_as_picture(None)
    }
}

#[derive(Clone)]
pub(crate) enum ProgramOperation {
    Save,
    OpacityBarrier,
    SaveLayerAlpha(Option<Rect>, f32),
    SaveLayer {
        bounds: Option<Rect>,
        paint: Option<Paint>,
        flags: SaveLayerFlags,
    },
    Restore,
    RestoreTo(usize),
    Translate(Vector),
    Scale((f32, f32)),
    Rotate(f32, Option<Point>),
    Concat(Matrix),
    Concat44(M44),
    SetMatrix(M44),
    ClipRect(Rect, ClipOp, bool),
    ClipRRect(RRect, ClipOp, bool),
    ClipPath(OwnedPath, ClipOp, bool),
    Draw(usize),
    EnterOwner(PaintProgramOwner),
    ExitOwner,
}

/// Bounds admission for an experimental intrinsic picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntrinsicPictureStatus {
    NotRecorded,
    /// Geometry cull bounds have not been proven conservative for arbitrary
    /// transforms, raster phases, antialias coverage and surrounding layers.
    UnqualifiedCullBounds,
}

/// One intrinsic draw with owned resources and an optional experimental picture.
/// State/effect scopes live in the ordered program, never in this chunk.
#[derive(Clone)]
pub struct PaintProgramChunk {
    draw: OwnedDraw,
    picture: Option<Picture>,
    owner: Option<PaintProgramOwner>,
}

impl PaintProgramChunk {
    /// Experimental source picture. This is not admitted for compositor replay;
    /// inspect `intrinsic_picture_status()` before assigning any future role.
    /// Exact static program replay always uses the original owned draw instead.
    pub fn intrinsic_picture(&self) -> Option<&Picture> {
        self.picture.as_ref()
    }
    pub fn intrinsic_picture_status(&self) -> IntrinsicPictureStatus {
        if self.picture.is_some() {
            IntrinsicPictureStatus::UnqualifiedCullBounds
        } else {
            IntrinsicPictureStatus::NotRecorded
        }
    }
    pub fn owner(&self) -> Option<PaintProgramOwner> {
        self.owner
    }
}

/// Immutable static paint stream. Complete capture is not release qualification.
#[derive(Clone)]
pub struct RecordedPaintProgram {
    operations: Vec<ProgramOperation>,
    chunks: Vec<PaintProgramChunk>,
    reasons: Vec<PaintCaptureIncompleteReason>,
    canvas_queries: usize,
    retained_font_bytes: Arc<[Arc<[u8]>]>,
    // Drop the font-cache lease after all retained draws/pictures/font bytes.
    font_cache_lifetime: Option<openui_text::font::FontCacheLifetime>,
}

impl RecordedPaintProgram {
    pub(crate) fn retain_resources(&mut self, recording: &crate::RecordedPicture) {
        self.retained_font_bytes = recording.retained_font_bytes.clone();
        self.font_cache_lifetime = Some(recording._font_cache_lifetime.clone());
    }
    pub fn retained_font_face_count(&self) -> usize {
        self.retained_font_bytes.len()
    }

    pub fn status(&self) -> PaintCaptureStatus {
        if self.reasons.is_empty() {
            PaintCaptureStatus::Complete
        } else {
            PaintCaptureStatus::Incomplete(self.reasons.clone())
        }
    }
    pub fn chunks(&self) -> &[PaintProgramChunk] {
        &self.chunks
    }
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
    /// Query-dependent decisions are captured as fixed operands. They have not
    /// been proven movable under a future compositor-owned transform or effect.
    pub fn canvas_query_count(&self) -> usize {
        self.canvas_queries
    }
    pub fn authored_layers_are_partitioned(&self) -> bool {
        false
    }

    /// Replay original operations and preserve the host canvas state. The caller
    /// owns raster scale, tile phase, resource/thread admission and qualification.
    pub fn replay(&self, canvas: &Canvas) -> Result<(), PaintCaptureStatus> {
        let status = self.status();
        if status != PaintCaptureStatus::Complete {
            return Err(status);
        }
        let tracked: &dyn PaintCanvas = canvas;
        tracked.legacy_helper_boundary();
        let host_count = canvas.save_count();
        struct RestoreHost<'a> {
            canvas: &'a Canvas,
            count: usize,
        }
        impl Drop for RestoreHost<'_> {
            fn drop(&mut self) {
                self.canvas.restore_to_count(self.count);
            }
        }
        let _restore_host = RestoreHost {
            canvas,
            count: host_count,
        };
        canvas.save();
        let base = canvas.save_count();
        let base_matrix = canvas.local_to_device();
        let _opacity_scope = tracked.opacity_scope();
        for operation in &self.operations {
            match operation {
                ProgramOperation::Save => {
                    tracked.save();
                }
                ProgramOperation::OpacityBarrier => {
                    tracked.legacy_helper_boundary();
                }
                ProgramOperation::SaveLayerAlpha(bounds, alpha) => {
                    tracked.save_layer_alpha_f(*bounds, *alpha);
                }
                ProgramOperation::SaveLayer {
                    bounds,
                    paint,
                    flags,
                } => {
                    tracked.save_layer_owned(*bounds, paint.as_ref(), *flags);
                }
                ProgramOperation::Restore => {
                    tracked.restore();
                }
                ProgramOperation::RestoreTo(count) => {
                    tracked.restore_to_count(base + count);
                }
                ProgramOperation::Translate(offset) => {
                    tracked.translate(*offset);
                }
                ProgramOperation::Scale(scale) => {
                    tracked.scale(*scale);
                }
                ProgramOperation::Rotate(angle, point) => {
                    tracked.rotate(*angle, *point);
                }
                ProgramOperation::Concat(matrix) => {
                    tracked.concat(matrix);
                }
                ProgramOperation::Concat44(matrix) => {
                    tracked.concat_44(matrix);
                }
                ProgramOperation::SetMatrix(matrix) => {
                    tracked.set_matrix(&M44::concat(&base_matrix, matrix));
                }
                ProgramOperation::ClipRect(rect, op, aa) => {
                    tracked.clip_rect(rect, *op, *aa);
                }
                ProgramOperation::ClipRRect(rect, op, aa) => {
                    tracked.clip_rrect(rect, *op, *aa);
                }
                ProgramOperation::ClipPath(path, op, aa) => {
                    path.with_path(|path| tracked.clip_path(path, *op, *aa));
                }
                ProgramOperation::Draw(index) => {
                    self.chunks[*index].draw.replay(tracked);
                }
                ProgramOperation::EnterOwner(_) | ProgramOperation::ExitOwner => {}
            }
        }
        tracked.restore_to_count(host_count);
        Ok(())
    }
}

struct Capture {
    canvas: usize,
    base_save_count: usize,
    program: RecordedPaintProgram,
    owners: Vec<PaintProgramOwner>,
}

thread_local! {
    static CAPTURES: RefCell<Vec<Capture>> = const { RefCell::new(Vec::new()) };
}

fn matching(canvas: &Canvas, action: impl FnOnce(&mut Capture)) {
    CAPTURES.with(|state| {
        if let Some(capture) = state
            .borrow_mut()
            .iter_mut()
            .rev()
            .find(|c| c.canvas == canvas as *const Canvas as usize)
        {
            action(capture);
        }
    });
}

pub(crate) fn is_capturing(canvas: &Canvas) -> bool {
    CAPTURES.with(|state| {
        state
            .borrow()
            .iter()
            .any(|c| c.canvas == canvas as *const Canvas as usize)
    })
}

pub(crate) fn record_operation(canvas: &Canvas, operation: ProgramOperation) {
    matching(canvas, |capture| capture.program.operations.push(operation));
}

pub(crate) fn record_draw(canvas: &Canvas, make_draw: impl FnOnce() -> OwnedDraw) {
    if !is_capturing(canvas) {
        return;
    }
    let draw = make_draw();
    // Recording the side picture uses a different canvas and does not mutate
    // the canonical state, its save depth or the pending opacity operation.
    let picture = draw.intrinsic_picture();
    matching(canvas, |capture| {
        let index = capture.program.chunks.len();
        capture.program.chunks.push(PaintProgramChunk {
            draw,
            picture,
            owner: capture.owners.last().copied(),
        });
        capture
            .program
            .operations
            .push(ProgramOperation::Draw(index));
    });
}

pub(crate) fn mark_incomplete(canvas: &Canvas, reason: PaintCaptureIncompleteReason) {
    matching(canvas, |capture| {
        if !capture.program.reasons.contains(&reason) {
            capture.program.reasons.push(reason);
        }
    });
}

pub(crate) fn record_restore(canvas: &Canvas) {
    matching(canvas, |capture| {
        if canvas.save_count() > capture.base_save_count {
            capture.program.operations.push(ProgramOperation::Restore);
        } else if capture.base_save_count > 1 {
            if !capture
                .program
                .reasons
                .contains(&PaintCaptureIncompleteReason::RestoreBeforeCapture)
            {
                capture
                    .program
                    .reasons
                    .push(PaintCaptureIncompleteReason::RestoreBeforeCapture);
            }
        }
    });
}

pub(crate) fn record_restore_to(canvas: &Canvas, count: usize) {
    matching(canvas, |capture| {
        if let Some(relative) = count.checked_sub(capture.base_save_count) {
            capture
                .program
                .operations
                .push(ProgramOperation::RestoreTo(relative));
        } else if !capture
            .program
            .reasons
            .contains(&PaintCaptureIncompleteReason::RestoreBeforeCapture)
        {
            capture
                .program
                .reasons
                .push(PaintCaptureIncompleteReason::RestoreBeforeCapture);
        }
    });
}

pub(crate) fn record_query(canvas: &Canvas) {
    matching(canvas, |capture| capture.program.canvas_queries += 1);
}

/// Bound to the canonical recording canvas, with no retained pointer after drop.
pub(crate) struct PaintProgramScope<'a> {
    canvas: &'a Canvas,
}

impl<'a> PaintProgramScope<'a> {
    pub(crate) fn new(canvas: &'a Canvas) -> Self {
        CAPTURES.with(|state| {
            state.borrow_mut().push(Capture {
                canvas: canvas as *const Canvas as usize,
                base_save_count: canvas.save_count(),
                program: RecordedPaintProgram {
                    operations: Vec::new(),
                    chunks: Vec::new(),
                    reasons: Vec::new(),
                    canvas_queries: 0,
                    retained_font_bytes: Arc::from([]),
                    font_cache_lifetime: None,
                },
                owners: Vec::new(),
            })
        });
        Self { canvas }
    }
    pub(crate) fn snapshot(&self) -> RecordedPaintProgram {
        let mut result = None;
        matching(self.canvas, |capture| {
            if self.canvas.save_count() != capture.base_save_count
                && !capture
                    .program
                    .reasons
                    .contains(&PaintCaptureIncompleteReason::UnbalancedSaveStack)
            {
                capture
                    .program
                    .reasons
                    .push(PaintCaptureIncompleteReason::UnbalancedSaveStack);
            }
            result = Some(capture.program.clone());
        });
        result.expect("active owned paint-program capture")
    }
}

impl Drop for PaintProgramScope<'_> {
    fn drop(&mut self) {
        CAPTURES.with(|state| {
            let mut state = state.borrow_mut();
            let index = state
                .iter()
                .rposition(|c| c.canvas == self.canvas as *const Canvas as usize)
                .expect("owned paint-program capture");
            state.remove(index);
        });
    }
}

pub(crate) struct PaintOwnerScope<'a> {
    canvas: &'a Canvas,
}
impl<'a> PaintOwnerScope<'a> {
    pub(crate) fn new(canvas: &'a Canvas, fragment: &Fragment) -> Self {
        let owner = PaintProgramOwner {
            node_id: fragment.node_id,
            owns_element_box: fragment.owns_element_box(),
        };
        matching(canvas, |capture| {
            capture.owners.push(owner);
            capture
                .program
                .operations
                .push(ProgramOperation::EnterOwner(owner));
        });
        Self { canvas }
    }
}
impl Drop for PaintOwnerScope<'_> {
    fn drop(&mut self) {
        matching(self.canvas, |capture| {
            capture.owners.pop();
            capture.program.operations.push(ProgramOperation::ExitOwner);
        });
    }
}

pub(crate) fn save_layer(
    canvas: &Canvas,
    bounds: Option<Rect>,
    paint: Option<&Paint>,
    flags: SaveLayerFlags,
) -> usize {
    let mut rec = SaveLayerRec::default().flags(flags);
    if let Some(bounds) = &bounds {
        rec = rec.bounds(bounds);
    }
    if let Some(paint) = paint {
        rec = rec.paint(paint);
    }
    canvas.save_layer(&rec)
}

#[cfg(test)]
mod tests {
    use super::*;
    use skia_safe::{surfaces, AlphaType, Color, ColorSpace, ImageInfo, PathBuilder};

    const BOUNDS: Rect = Rect {
        left: 0.0,
        top: 0.0,
        right: 96.0,
        bottom: 72.0,
    };

    fn color_paint(color: Color4f) -> Paint {
        let mut paint = Paint::default();
        paint.set_color4f(color, None::<&ColorSpace>);
        paint.set_anti_alias(true);
        paint
    }

    fn capture(draw: impl FnOnce(&dyn PaintCanvas)) -> (Picture, RecordedPaintProgram) {
        let mut recorder = PictureRecorder::new();
        let raw = recorder.begin_recording(BOUNDS, false);
        let scope = PaintProgramScope::new(raw);
        let canvas: &dyn PaintCanvas = raw;
        {
            let _opacity = canvas.opacity_scope();
            canvas.clear(Color::WHITE);
            draw(canvas);
        }
        let program = scope.snapshot();
        drop(scope);
        let picture = recorder.finish_recording_as_picture(None).unwrap();
        (picture, program)
    }

    fn pixels(picture: &Picture, scale: f32) -> Vec<u8> {
        let size = ((96.0 * scale) as i32, (72.0 * scale) as i32);
        let mut surface = surfaces::raster_n32_premul(size).unwrap();
        surface.canvas().scale((scale, scale));
        surface.canvas().draw_picture(picture, None, None);
        let info = ImageInfo::new_n32(size, AlphaType::Premul, None);
        let stride = size.0 as usize * 4;
        let mut bytes = vec![0; stride * size.1 as usize];
        assert!(surface.read_pixels(&info, &mut bytes, stride, (0, 0)));
        bytes
    }

    fn assert_static_replay(picture: &Picture, program: &RecordedPaintProgram) {
        assert_eq!(program.status(), PaintCaptureStatus::Complete);
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let mut recorder = PictureRecorder::new();
            let canvas = recorder.begin_recording(BOUNDS, false);
            program.replay(canvas).unwrap();
            let replay = recorder.finish_recording_as_picture(None).unwrap();
            assert_eq!(
                pixels(picture, scale),
                pixels(&replay, scale),
                "scale {scale}"
            );
        }
        assert!(!program.authored_layers_are_partitioned());
    }

    #[test]
    fn published_program_resources_are_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<RecordedPaintProgram>();
        assert_send_sync::<PaintProgramChunk>();
    }

    #[test]
    fn experimental_fill_pictures_match_direct_draws_in_transformed_aa_scopes() {
        let paint = color_paint(Color4f::new(0.2, 0.6, 0.8, 0.7));
        let bounds = Rect::from_xywh(18.0, 16.0, 28.0, 22.0);
        let outer = RRect::new_rect_xy(bounds, 7.0, 5.0);
        let inner = RRect::new_rect_xy(Rect::from_xywh(23.0, 20.0, 18.0, 14.0), 3.0, 2.0);
        let draws = [
            OwnedDraw::Rect(bounds, paint.clone()),
            OwnedDraw::Oval(bounds, paint.clone()),
            OwnedDraw::RRect(outer, paint.clone()),
            OwnedDraw::RoundRect(bounds, 7.0, 5.0, paint.clone()),
            OwnedDraw::DRRect(outer, inner, paint),
        ];
        for (shape, draw) in draws.iter().enumerate() {
            let picture = draw.intrinsic_picture().unwrap();
            let chunk = PaintProgramChunk {
                draw: draw.clone(),
                picture: Some(picture.clone()),
                owner: None,
            };
            assert_eq!(
                chunk.intrinsic_picture_status(),
                IntrinsicPictureStatus::UnqualifiedCullBounds
            );
            for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
                for phase in [0.0, 0.25, 0.5] {
                    for rotation in [0.0, 17.0] {
                        for clip_aa in [false, true] {
                            for nested_layer in [false, true] {
                                let render = |use_picture| {
                                    let size = ((96.0 * scale) as i32, (72.0 * scale) as i32);
                                    let mut surface = surfaces::raster_n32_premul(size).unwrap();
                                    let canvas = surface.canvas();
                                    canvas.clear(Color::WHITE);
                                    canvas.scale((scale, scale));
                                    canvas.translate((phase, phase * 0.5));
                                    canvas.rotate(rotation, Some(Point::new(32.0, 27.0)));
                                    // The first clip contains the complete draw; the
                                    // second crosses its ink, exercising AA clipping.
                                    canvas.clip_rrect(
                                        RRect::new_rect_xy(
                                            Rect::from_xywh(10.0, 8.0, 50.0, 45.0),
                                            6.0,
                                            8.0,
                                        ),
                                        ClipOp::Intersect,
                                        clip_aa,
                                    );
                                    canvas.clip_rect(
                                        Rect::from_xywh(17.25, 14.5, 30.0, 25.0),
                                        ClipOp::Intersect,
                                        clip_aa,
                                    );
                                    if nested_layer {
                                        canvas.save_layer_alpha_f(None, 0.8);
                                        canvas.save_layer_alpha_f(None, 0.6);
                                    }
                                    if use_picture {
                                        canvas.draw_picture(&picture, None, None);
                                    } else {
                                        draw.replay(canvas as &dyn PaintCanvas);
                                    }
                                    if nested_layer {
                                        canvas.restore();
                                        canvas.restore();
                                    }
                                    let info = ImageInfo::new_n32(size, AlphaType::Premul, None);
                                    let stride = size.0 as usize * 4;
                                    let mut bytes = vec![0; stride * size.1 as usize];
                                    assert!(surface.read_pixels(&info, &mut bytes, stride, (0, 0)));
                                    bytes
                                };
                                let direct = render(false);
                                for repeat in 0..2 {
                                    assert_eq!(
                                        direct,
                                        render(true),
                                        "shape {shape}, scale {scale}, phase {phase}, rotation {rotation}, AA clip {clip_aa}, nested layer {nested_layer}, repeat {repeat}"
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }
        // Even passing this bounded guard does not establish conservative
        // picture bounds for arbitrary scopes or admit pictures for promotion.
    }

    #[test]
    fn exact_state_stream_preserves_interleaved_effect_and_complex_clips() {
        let (picture, program) = capture(|canvas| {
            let base = canvas.save();
            canvas.translate((3.25, 2.5));
            canvas.concat(&Matrix::scale((1.1, 0.9)));
            canvas.rotate(4.0, Some(Point::new(25.0, 20.0)));
            canvas.clip_rrect(
                RRect::new_rect_xy(Rect::from_xywh(1.0, 1.0, 70.0, 50.0), 9.0, 6.0),
                ClipOp::Intersect,
                true,
            );
            let mut path = PathBuilder::new();
            path.move_to((18.0, 15.0));
            path.line_to((34.0, 15.0));
            path.line_to((26.0, 31.0));
            path.close();
            canvas.clip_path(&path.detach(), ClipOp::Difference, true);
            canvas.save_layer_alpha_f(None, 0.2);
            let red = color_paint(Color4f::new(1.0, 0.0, 0.0, 1.0));
            let green = color_paint(Color4f::new(0.0, 1.0, 0.0, 1.0));
            canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 40.0, 40.0), &red);
            canvas.draw_rrect(
                RRect::new_rect_xy(Rect::from_xywh(20.0, 5.0, 45.0, 35.0), 8.0, 4.0),
                &green,
            );
            canvas.restore();
            canvas.restore_to_count(base);
        });
        assert_static_replay(&picture, &program);
        assert!(program
            .chunks()
            .iter()
            .any(|chunk| chunk.intrinsic_picture().is_some()));
    }

    #[test]
    fn float_opacity_original_operand_and_empty_helper_barrier_are_retained() {
        for barrier in [false, true] {
            let (picture, program) = capture(|canvas| {
                canvas.save_layer_alpha_f(None, 0.2);
                if barrier {
                    canvas.legacy_helper_boundary();
                }
                canvas.draw_rect(
                    Rect::from_xywh(0.0, 0.0, 50.0, 50.0),
                    &color_paint(Color4f::new(0.0, 0.5, 0.0, 0.7)),
                );
                canvas.restore();
            });
            assert_static_replay(&picture, &program);
            let rect_paint = program
                .chunks
                .iter()
                .find_map(|chunk| {
                    if let OwnedDraw::Rect(_, paint) = &chunk.draw {
                        Some(paint)
                    } else {
                        None
                    }
                })
                .unwrap();
            assert_eq!(rect_paint.alpha_f().to_bits(), 0.7_f32.to_bits());
        }
    }

    #[test]
    fn owned_image_text_and_path_operands_survive_source_drop() {
        let (picture, program) = capture(|canvas| {
            let mut source = surfaces::raster_n32_premul((12, 12)).unwrap();
            source.canvas().clear(Color::BLUE);
            let image = source.image_snapshot();
            let paint = color_paint(Color4f::new(0.0, 0.0, 0.0, 1.0));
            let src = Rect::from_xywh(1.0, 1.0, 10.0, 10.0);
            canvas.draw_image_rect_with_sampling_options(
                &image,
                Some((&src, SrcRectConstraint::Strict)),
                Rect::from_xywh(4.5, 5.25, 24.0, 18.0),
                SamplingOptions::from(FilterMode::Linear),
                &paint,
            );
            canvas.draw_image_nine(
                &image,
                IRect::new(3, 3, 9, 9),
                Rect::from_xywh(34.0, 4.0, 22.0, 20.0),
                FilterMode::Nearest,
                None,
            );
            let font = Font::default();
            canvas.draw_str("native", (4.0, 40.0), &font, &paint);
            if let Some(blob) = TextBlob::from_str("owned", &font) {
                canvas.draw_text_blob(&blob, (4.0, 58.0), &paint);
            }
            let mut path = PathBuilder::new();
            path.move_to((60.0, 8.0));
            path.line_to((80.0, 8.0));
            path.line_to((74.0, 26.0));
            path.close();
            canvas.draw_path(&path.detach(), &paint);
        });
        assert_static_replay(&picture, &program);
        // Unknown/unbounded intrinsic picture bounds retain original typed
        // operands instead of silently inventing an independently safe chunk.
        assert!(program
            .chunks
            .iter()
            .any(|chunk| matches!(chunk.draw, OwnedDraw::Path(..)) && chunk.picture.is_none()));
    }

    #[test]
    fn original_layer_flags_and_mask_blend_span_all_ordered_chunks() {
        let (picture, program) = capture(|canvas| {
            canvas.save_layer_owned(Some(BOUNDS), None, SaveLayerFlags::F16_COLOR_TYPE);
            canvas.draw_oval(
                Rect::from_xywh(4.0, 4.0, 45.0, 35.0),
                &color_paint(Color4f::new(1.0, 0.0, 0.0, 1.0)),
            );
            let mut mask = Paint::default();
            mask.set_blend_mode(BlendMode::DstIn);
            canvas.save_layer_owned(None, Some(&mask), SaveLayerFlags::empty());
            canvas.draw_round_rect(
                Rect::from_xywh(20.0, 5.0, 55.0, 30.0),
                7.0,
                7.0,
                &color_paint(Color4f::new(0.0, 0.0, 1.0, 0.6)),
            );
            canvas.restore();
            canvas.restore();
        });
        assert_static_replay(&picture, &program);
    }

    #[test]
    fn restore_depth_and_set_matrix_preserve_the_host_canvas() {
        let (_, program) = capture(|canvas| {
            let saved = canvas.save();
            canvas.concat_44(&M44::from(Matrix::translate((7.0, 5.0))));
            canvas.set_matrix(&M44::from(Matrix::translate((11.0, 9.0))));
            canvas.draw_rect(
                Rect::from_xywh(0.0, 0.0, 4.0, 4.0),
                &color_paint(Color4f::new(0.0, 1.0, 0.0, 1.0)),
            );
            canvas.restore_to_count(saved);
        });
        let mut surface = surfaces::raster_n32_premul((96, 72)).unwrap();
        let canvas = surface.canvas();
        canvas.save();
        canvas.translate((10.0, 3.0));
        canvas.scale((1.25, 1.25));
        let before_count = canvas.save_count();
        let before_matrix = canvas.local_to_device();
        for _ in 0..2 {
            program.replay(canvas).unwrap();
            assert_eq!(canvas.save_count(), before_count);
            assert_eq!(canvas.local_to_device(), before_matrix);
        }
    }

    #[test]
    fn raw_escape_and_unbalanced_state_reject_replay_before_drawing() {
        let (_, escaped) = capture(|canvas| {
            canvas.raw_untracked().draw_rect(BOUNDS, &Paint::default());
        });
        assert!(
            matches!(escaped.status(), PaintCaptureStatus::Incomplete(ref reasons) if reasons.contains(&PaintCaptureIncompleteReason::RawCanvasAccess))
        );
        let (_, unbalanced) = capture(|canvas| {
            canvas.save();
        });
        assert!(
            matches!(unbalanced.status(), PaintCaptureStatus::Incomplete(ref reasons) if reasons.contains(&PaintCaptureIncompleteReason::UnbalancedSaveStack))
        );
        let mut surface = surfaces::raster_n32_premul((96, 72)).unwrap();
        let before = surface.canvas().save_count();
        assert!(escaped.replay(surface.canvas()).is_err());
        assert_eq!(surface.canvas().save_count(), before);
    }

    #[test]
    fn nested_recording_and_queries_keep_the_correct_canvas_owner() {
        let (_, outer) = capture(|canvas| {
            assert!(canvas.local_clip_bounds().is_some());
            let (_, inner) = capture(|other| {
                other.draw_circle((20.0, 20.0), 5.0, &Paint::default());
            });
            assert_eq!(inner.canvas_query_count(), 0);
            canvas.draw_rect(Rect::from_xywh(10.0, 10.0, 8.0, 8.0), &Paint::default());
        });
        assert_eq!(outer.status(), PaintCaptureStatus::Complete);
        assert_eq!(outer.canvas_query_count(), 1);
        assert_eq!(outer.chunks.len(), 2); // clear plus the outer rectangle
    }
}
