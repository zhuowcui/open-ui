//! Explicit, unqualified transparent-source diagnostics over actual paint scopes.
//! The release renderer and canonical recording remain separate.

use crate::{PaintLayerBinding, PaintLayerSourceId, PaintLayoutScopeRole};
use openui_dom::NodeId;

/// Property role emitted at an actual painter decision, not inferred from tags.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PaintPropertyRole {
    Transform,
    OverflowClip,
    ClipPath,
    Filter,
    Mask,
    Opacity,
}

/// A property shares its real retained source generation. Multiple fragments
/// carry projections of this identity; operation positions are never lifetimes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintPropertyIdentity {
    pub source: PaintLayerSourceId,
    pub role: PaintPropertyRole,
    pub layout_role: PaintLayoutScopeRole,
}

/// Exact decision operands from the canonical painter. An effect reason is
/// recorded separately from any future surface/presentation admission.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PaintPropertyDecision {
    Transform,
    OverflowClip,
    ClipPath,
    Filter,
    Mask,
    Opacity {
        alpha: f32,
        will_change_transform: bool,
    },
}

impl PaintPropertyDecision {
    pub fn role(self) -> PaintPropertyRole {
        match self {
            Self::Transform => PaintPropertyRole::Transform,
            Self::OverflowClip => PaintPropertyRole::OverflowClip,
            Self::ClipPath => PaintPropertyRole::ClipPath,
            Self::Filter => PaintPropertyRole::Filter,
            Self::Mask => PaintPropertyRole::Mask,
            Self::Opacity { .. } => PaintPropertyRole::Opacity,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PropertyMarker {
    pub node: NodeId,
    pub owns_element_box: bool,
    pub layout_role: PaintLayoutScopeRole,
    pub decision: PaintPropertyDecision,
}

/// A projection into the exact ordered recording stream. It does not apply
/// another effect and contains no DOM reference or callback.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintPropertyProjection {
    /// None preserves an actual elided owner whose retained layer ID is not
    /// available; it never borrows another node's lifetime identity.
    pub identity: Option<PaintPropertyIdentity>,
    pub binding: Option<PaintLayerBinding>,
    pub owner: crate::PaintProgramOwner,
    pub layout_role: PaintLayoutScopeRole,
    pub decision: PaintPropertyDecision,
    pub operation: usize,
}

/// Cull/query/object-class and compositor interval qualification is separate
/// from possessing an owned static source picture.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransparentSurfaceQualification {
    UnqualifiedStaticDiagnostic,
}

use crate::paint_program::{
    replay_operations, replay_operations_with, OwnedDraw, ProgramOperation,
};
use crate::{LayeredPaintProgram, PaintCaptureStatus};
use skia_safe::{
    surfaces, Canvas, Color, Image, Paint, Picture, PictureRecorder, Rect, SamplingOptions, M44,
};
use std::sync::Arc;

/// An actual canonical opacity interval. `selected` records the explicit
/// painter reason, not a claim that Chromium/Viz admitted this exact surface.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PaintOpacityInterval {
    pub property: PaintPropertyProjection,
    pub first_operation: usize,
    pub end_operation: usize,
    pub alpha: f32,
    pub selected: bool,
    /// A RestoreTo may close additional ancestor scopes at the same position.
    pub restore_to: Option<usize>,
    pub has_ancestor_filter: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TransparentSurfaceError {
    InvalidBounds,
    IncompleteCapture(PaintCaptureStatus),
    UnbalancedState,
    UnboundEffect,
    FilterDependency,
    OpaquePictureDependency,
    UnclassifiedShaderDependency,
    AllocationFailure,
    PictureRecordingFailure,
}
impl std::fmt::Display for TransparentSurfaceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "transparent source diagnostic: {self:?}")
    }
}
impl std::error::Error for TransparentSurfaceError {}

/// Independent owned, world-baked source of one effect interval. Ancestor
/// transforms remain in the program; ancestor clips/effects are composed by the
/// parent exactly once. Cull, arbitrary host movement and tracks are unqualified.
#[derive(Clone)]
pub struct RetainedTransparentSource {
    interval: PaintOpacityInterval,
    picture: Picture,
    operations: Arc<[ProgramOperation]>,
    original_operations: Arc<[Option<usize>]>,
    owner: Arc<LayeredPaintProgram>,
    bounds: Rect,
}
impl RetainedTransparentSource {
    pub fn interval(&self) -> PaintOpacityInterval {
        self.interval
    }
    pub fn picture(&self) -> &Picture {
        &self.picture
    }
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    pub fn qualification(&self) -> TransparentSurfaceQualification {
        TransparentSurfaceQualification::UnqualifiedStaticDiagnostic
    }
    /// Replay the owned original source, without applying its owning alpha.
    /// Ancestor clip/effect and raster context remain explicit caller inputs.
    pub fn replay_original_source(&self, canvas: &Canvas) {
        replay_operations(&self.operations, canvas, |i| self.owner.owned_draw(i));
    }
}

/// Static diagnostic only. Original ordinary intervals stay on the original
/// state spine; selected intervals compose one transparent N32 image with the
/// owning alpha. The release recording and its renderer selection are unchanged.
#[derive(Clone)]
pub struct TransparentSurfaceProgram {
    owner: Arc<LayeredPaintProgram>,
    intervals: Arc<[PaintOpacityInterval]>,
    sources: Arc<[RetainedTransparentSource]>,
    bounds: Rect,
}

impl LayeredPaintProgram {
    /// Standalone full-viewport bounds are conservative only for this explicit
    /// fixed output. Filter/outside-target and dynamic culling remain unqualified.
    pub fn transparent_surface_program_for_viewport(
        &self,
        viewport: openui_geometry::ViewportMetrics,
    ) -> Result<TransparentSurfaceProgram, TransparentSurfaceError> {
        self.transparent_surface_program(Rect::from_xywh(
            0.0,
            0.0,
            viewport.logical_width() as f32,
            viewport.logical_height() as f32,
        ))
    }
    pub fn transparent_surface_program(
        &self,
        conservative_bounds: Rect,
    ) -> Result<TransparentSurfaceProgram, TransparentSurfaceError> {
        TransparentSurfaceProgram::new(self.clone(), conservative_bounds)
    }
}

impl TransparentSurfaceProgram {
    fn new(owner: LayeredPaintProgram, bounds: Rect) -> Result<Self, TransparentSurfaceError> {
        if bounds.is_empty()
            || ![bounds.left, bounds.top, bounds.right, bounds.bottom]
                .into_iter()
                .all(f32::is_finite)
        {
            return Err(TransparentSurfaceError::InvalidBounds);
        }
        if owner.capture_status() != &PaintCaptureStatus::Complete {
            return Err(TransparentSurfaceError::IncompleteCapture(
                owner.capture_status().clone(),
            ));
        }
        let owner = Arc::new(owner);
        let mut stack: Vec<(Option<usize>, bool)> = Vec::new();
        let mut intervals = Vec::new();
        for (i, operation) in owner.operations().iter().enumerate() {
            match operation {
                ProgramOperation::Save => stack.push((None, false)),
                ProgramOperation::SaveLayer { paint, .. } => stack.push((
                    None,
                    paint
                        .as_ref()
                        .is_some_and(|p| p.image_filter().is_some() || p.mask_filter().is_some()),
                )),
                ProgramOperation::SaveLayerAlpha(_, alpha) => {
                    let property = i
                        .checked_sub(1)
                        .and_then(|marker| {
                            owner.properties().iter().find(|p| p.operation == marker)
                        })
                        .copied();
                    let interval = if let Some(property) = property {
                        if let PaintPropertyDecision::Opacity {
                            alpha: recorded_alpha,
                            will_change_transform,
                        } = property.decision
                        {
                            if recorded_alpha.to_bits() != alpha.to_bits() || !alpha.is_finite() {
                                return Err(TransparentSurfaceError::UnbalancedState);
                            }
                            if will_change_transform && property.binding.is_none() {
                                return Err(TransparentSurfaceError::UnboundEffect);
                            }
                            let index = intervals.len();
                            intervals.push(PaintOpacityInterval {
                                property,
                                first_operation: i,
                                end_operation: 0,
                                alpha: *alpha,
                                selected: will_change_transform,
                                restore_to: None,
                                has_ancestor_filter: stack.iter().any(|(_, filter)| *filter),
                            });
                            Some(index)
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                    stack.push((interval, false));
                }
                ProgramOperation::Restore => {
                    let (entry, _) = stack
                        .pop()
                        .ok_or(TransparentSurfaceError::UnbalancedState)?;
                    if let Some(index) = entry {
                        intervals[index].end_operation = i + 1;
                    }
                }
                ProgramOperation::RestoreTo(count) => {
                    // SkCanvas restore-to above the current depth is a no-op.
                    // The complete recording retains that command unchanged.
                    while stack.len() > *count {
                        if let (Some(index), _) = stack.pop().expect("checked state depth") {
                            intervals[index].end_operation = i + 1;
                            intervals[index].restore_to = Some(*count);
                        }
                    }
                }
                _ => {}
            }
        }
        if !stack.is_empty() {
            return Err(TransparentSurfaceError::UnbalancedState);
        }
        let mut sources = Vec::new();
        for interval in intervals.iter().filter(|i| i.selected) {
            if interval.has_ancestor_filter {
                return Err(TransparentSurfaceError::FilterDependency);
            }
            // Retaining a Picture's pixels is not proof of the independent child
            // capture it hides (notably SVG raw-canvas access). Fail closed.
            for operation in
                &owner.operations()[interval.first_operation + 1..interval.end_operation]
            {
                match operation {
                    ProgramOperation::SaveLayer {
                        paint: Some(paint), ..
                    } if paint.image_filter().is_some() || paint.mask_filter().is_some() => {
                        return Err(TransparentSurfaceError::FilterDependency);
                    }
                    ProgramOperation::Draw(index) => {
                        let draw = owner.owned_draw(*index);
                        if matches!(draw, OwnedDraw::Picture(..)) {
                            return Err(TransparentSurfaceError::OpaquePictureDependency);
                        }
                        if draw.has_filter_dependency() {
                            return Err(TransparentSurfaceError::FilterDependency);
                        }
                        if draw.shader_dependency() == crate::PaintShaderDependency::Unclassified {
                            return Err(TransparentSurfaceError::UnclassifiedShaderDependency);
                        }
                    }
                    _ => {}
                }
            }
            // The prefix supplies only the actual matrix/save-depth history.
            // Every ancestor clip and alpha stays on the parent state spine.
            let mut operations = Vec::new();
            let mut mapping = Vec::new();
            for operation in &owner.operations()[..interval.first_operation] {
                let operation = match operation {
                    ProgramOperation::Draw(_)
                    | ProgramOperation::ClipRect(..)
                    | ProgramOperation::ClipRRect(..)
                    | ProgramOperation::ClipPath(..)
                    | ProgramOperation::EnterOwner(..)
                    | ProgramOperation::ExitOwner
                    | ProgramOperation::Property(..) => continue,
                    ProgramOperation::SaveLayerAlpha(..) | ProgramOperation::SaveLayer { .. } => {
                        ProgramOperation::Save
                    }
                    other => other.clone(),
                };
                operations.push(operation);
                mapping.push(None);
            }
            // Keep the original relative depth while omitting only owning alpha.
            operations.push(ProgramOperation::Save);
            mapping.push(None);
            for (i, operation) in owner
                .operations()
                .iter()
                .enumerate()
                .take(interval.end_operation)
                .skip(interval.first_operation + 1)
            {
                operations.push(operation.clone());
                mapping.push(Some(i));
            }
            operations.push(ProgramOperation::RestoreTo(0));
            mapping.push(None);
            let mut recorder = PictureRecorder::new();
            let canvas = recorder.begin_recording(bounds, false);
            replay_operations(&operations, canvas, |i| owner.owned_draw(i));
            let picture = recorder
                .finish_recording_as_picture(None)
                .ok_or(TransparentSurfaceError::PictureRecordingFailure)?;
            sources.push(RetainedTransparentSource {
                interval: *interval,
                picture,
                operations: operations.into(),
                original_operations: mapping.into(),
                owner: Arc::clone(&owner),
                bounds,
            });
        }
        Ok(Self {
            owner,
            intervals: intervals.into(),
            sources: sources.into(),
            bounds,
        })
    }
    pub fn intervals(&self) -> &[PaintOpacityInterval] {
        &self.intervals
    }
    pub fn sources(&self) -> &[RetainedTransparentSource] {
        &self.sources
    }
    /// Full conservative damage only. Independent tighter culling is not admitted.
    pub fn damage(&self) -> Rect {
        self.bounds
    }
    pub fn qualification(&self) -> TransparentSurfaceQualification {
        TransparentSurfaceQualification::UnqualifiedStaticDiagnostic
    }
    pub fn dynamic_tracks_are_admitted(&self) -> bool {
        false
    }
    pub fn canvas_query_dependency_count(&self) -> usize {
        self.owner.coverage().fixed_canvas_queries
    }

    /// Explicit standalone CPU target, using white clear and the supplied scale.
    /// This is not the tiled document/viewport compositor or release selection.
    pub fn rasterize_diagnostic(
        &self,
        viewport: openui_geometry::ViewportMetrics,
    ) -> Result<skia_safe::Surface, TransparentSurfaceError> {
        let width = i32::try_from(viewport.physical_width())
            .map_err(|_| TransparentSurfaceError::InvalidBounds)?;
        let height = i32::try_from(viewport.physical_height())
            .map_err(|_| TransparentSurfaceError::InvalidBounds)?;
        let mut surface = surfaces::raster_n32_premul((width, height))
            .ok_or(TransparentSurfaceError::AllocationFailure)?;
        surface.canvas().clear(Color::WHITE);
        let scale = viewport.device_scale_factor() as f32;
        surface.canvas().scale((scale, scale));
        self.replay_diagnostic(
            surface.canvas(),
            (viewport.physical_width(), viewport.physical_height()),
        )?;
        Ok(surface)
    }

    /// A physical target is an explicit raster input. Sources remain world-baked;
    /// no target size/phase/tile context is inferred from DOM or historical rows.
    /// Allocate every selected source before touching the output canvas.
    pub fn replay_diagnostic(
        &self,
        canvas: &Canvas,
        physical_size: (u32, u32),
    ) -> Result<(), TransparentSurfaceError> {
        let size = (
            i32::try_from(physical_size.0).map_err(|_| TransparentSurfaceError::InvalidBounds)?,
            i32::try_from(physical_size.1).map_err(|_| TransparentSurfaceError::InvalidBounds)?,
        );
        if size.0 <= 0 || size.1 <= 0 {
            return Err(TransparentSurfaceError::InvalidBounds);
        }
        let root_matrix = canvas.local_to_device();
        let mut images: Vec<Option<Image>> = vec![None; self.sources.len()];
        // Inner intervals occur after their owning outer interval. Render them
        // first so recursive composition uses owned immutable images only.
        for index in (0..self.sources.len()).rev() {
            let source = &self.sources[index];
            let mut surface = surfaces::raster_n32_premul(size)
                .ok_or(TransparentSurfaceError::AllocationFailure)?;
            surface.canvas().clear(Color::TRANSPARENT);
            surface.canvas().set_matrix(&root_matrix);
            self.replay_with_images(
                &source.operations,
                Some(&source.original_operations),
                surface.canvas(),
                &images,
                size,
            )?;
            images[index] = Some(surface.image_snapshot());
        }
        self.replay_with_images(self.owner.operations(), None, canvas, &images, size)
    }
    fn replay_with_images(
        &self,
        operations: &[ProgramOperation],
        mapping: Option<&[Option<usize>]>,
        canvas: &Canvas,
        images: &[Option<Image>],
        size: (i32, i32),
    ) -> Result<(), TransparentSurfaceError> {
        replay_operations_with(
            operations,
            canvas,
            |i| self.owner.owned_draw(i),
            |local, tracked, _, base| {
                let global = match mapping {
                    Some(m) => m[local],
                    None => Some(local),
                };
                let Some(global) = global else {
                    return Ok(None);
                };
                let Some((index, source)) = self
                    .sources
                    .iter()
                    .enumerate()
                    .find(|(_, s)| s.interval.first_operation == global)
                else {
                    return Ok(None);
                };
                let image = images[index]
                    .as_ref()
                    .ok_or(TransparentSurfaceError::AllocationFailure)?;
                tracked.legacy_helper_boundary();
                tracked.save();
                tracked.set_matrix(&M44::new_identity());
                let mut paint = Paint::default();
                paint.set_alpha_f(source.interval.alpha);
                tracked.draw_image_rect_with_sampling_options(
                    image,
                    None,
                    Rect::from_xywh(0.0, 0.0, size.0 as f32, size.1 as f32),
                    SamplingOptions::default(),
                    &paint,
                );
                tracked.restore();
                if let Some(count) = source.interval.restore_to {
                    tracked.restore_to_count(base + count);
                }
                let next = match mapping {
                    Some(m) => {
                        m.iter()
                            .position(|i| *i == Some(source.interval.end_operation - 1))
                            .ok_or(TransparentSurfaceError::UnbalancedState)?
                            + 1
                    }
                    None => source.interval.end_operation,
                };
                Ok(Some(next))
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint_program::{record_fragment_property, PaintOwnerScope, PaintProgramScope};
    use crate::paint_record::PaintCanvas;
    use crate::{PaintLayerNodeIdentity, PaintLayerSourceRole};
    use openui_dom::{Document, ElementTag};
    use openui_geometry::{LayoutUnit, PhysicalSize};
    use openui_layout::Fragment;
    use skia_safe::{AlphaType, ClipOp, ImageInfo, RRect, Surface};

    const BOUNDS: Rect = Rect {
        left: 0.0,
        top: 0.0,
        right: 96.0,
        bottom: 72.0,
    };
    fn binding(id: u64, owned: bool) -> PaintLayerBinding {
        PaintLayerBinding {
            id: PaintLayerSourceId(id),
            owner: owned.then_some(PaintLayerNodeIdentity {
                document: 1,
                index: id as u32,
                generation: 1,
            }),
            role: if owned {
                PaintLayerSourceRole::AuthoredBox
            } else {
                PaintLayerSourceRole::DocumentContent
            },
        }
    }
    fn capture(selected: bool, nested: bool) -> (Picture, LayeredPaintProgram) {
        let mut doc = Document::new();
        let node = doc.create_node(ElementTag::Div);
        let child = doc.create_node(ElementTag::Div);
        let fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(40), LayoutUnit::from_i32(30)),
        );
        let child_fragment = Fragment::new_box(child, fragment.size);
        let mut recorder = PictureRecorder::new();
        let raw = recorder.begin_recording(BOUNDS, false);
        let capture = PaintProgramScope::new(raw);
        let c: &dyn PaintCanvas = raw;
        {
            let _opacity = c.opacity_scope();
            c.draw_color(Color::WHITE, skia_safe::BlendMode::Src);
            c.save();
            c.translate((10.25, 7.75));
            c.clip_rrect(
                RRect::new_rect_xy(Rect::from_xywh(0.0, 0.0, 60.0, 48.0), 5.0, 7.0),
                ClipOp::Intersect,
                true,
            );
            {
                let _owner = PaintOwnerScope::new(raw, &fragment);
                record_fragment_property(
                    raw,
                    &fragment,
                    PaintPropertyDecision::Opacity {
                        alpha: 0.2,
                        will_change_transform: selected,
                    },
                );
                c.save_layer_alpha_f(None, 0.2);
                let mut paint = Paint::default();
                paint.set_color(Color::GREEN);
                c.draw_rect(Rect::from_xywh(1.0, 1.0, 32.0, 24.0), &paint);
                if nested {
                    let _child_owner = PaintOwnerScope::new(raw, &child_fragment);
                    record_fragment_property(
                        raw,
                        &child_fragment,
                        PaintPropertyDecision::Opacity {
                            alpha: 0.5,
                            will_change_transform: selected,
                        },
                    );
                    c.save_layer_alpha_f(None, 0.5);
                    paint.set_color(Color::RED);
                    c.draw_rect(Rect::from_xywh(15.0, 8.0, 24.0, 20.0), &paint);
                    c.restore();
                }
                // An ordinary second draw prevents single-operation alpha folding.
                paint.set_color(Color::BLUE);
                c.draw_line((2.0, 29.0), (38.0, 29.0), &paint);
                c.restore();
            }
            c.restore();
        }
        let program = capture.snapshot();
        drop(capture);
        let picture = recorder.finish_recording_as_picture(None).unwrap();
        let layered = program
            .partition_layers(binding(1, false), |n| {
                Some(binding(if n == node { 2 } else { 3 }, true))
            })
            .unwrap();
        (picture, layered)
    }
    fn bytes(surface: &mut Surface, size: (i32, i32)) -> Vec<u8> {
        let info = ImageInfo::new_n32(size, AlphaType::Premul, None);
        let stride = size.0 as usize * 4;
        let mut b = vec![0; stride * size.1 as usize];
        assert!(surface.read_pixels(&info, &mut b, stride, (0, 0)));
        b
    }
    #[test]
    fn ordinary_intervals_keep_original_pixels_and_selected_sources_are_owned() {
        fn send_sync<T: Send + Sync>() {}
        send_sync::<TransparentSurfaceProgram>();
        send_sync::<RetainedTransparentSource>();
        for scale in [1.0_f32, 1.25, 1.5, 1.75, 2.0] {
            let size = ((96.0 * scale) as i32, (72.0 * scale) as i32);
            let (original, ordinary) = capture(false, true);
            let plan = ordinary.transparent_surface_program(BOUNDS).unwrap();
            assert_eq!(plan.intervals().len(), 2);
            assert!(plan.sources().is_empty());
            let mut expected = surfaces::raster_n32_premul(size).unwrap();
            expected.canvas().scale((scale, scale));
            expected.canvas().draw_picture(&original, None, None);
            let mut actual = surfaces::raster_n32_premul(size).unwrap();
            actual.canvas().scale((scale, scale));
            plan.replay_diagnostic(actual.canvas(), (size.0 as u32, size.1 as u32))
                .unwrap();
            assert_eq!(bytes(&mut actual, size), bytes(&mut expected, size));
            let (_, selected) = capture(true, true);
            let selected = selected.transparent_surface_program(BOUNDS).unwrap();
            assert_eq!(selected.sources().len(), 2);
            assert_eq!(selected.damage(), BOUNDS);
            assert!(!selected.dynamic_tracks_are_admitted());
            assert_eq!(
                selected.qualification(),
                TransparentSurfaceQualification::UnqualifiedStaticDiagnostic
            );
            for source in selected.sources() {
                assert!(source.interval().property.identity.is_some());
                assert_eq!(
                    source.qualification(),
                    TransparentSurfaceQualification::UnqualifiedStaticDiagnostic
                );
            }
            // The selected outer source excludes its owning alpha and its
            // ancestor clip. A strict interior pixel is the original opaque
            // GREEN operand, not a twice-multiplied alpha result.
            let mut source_surface = surfaces::raster_n32_premul(size).unwrap();
            source_surface.canvas().scale((scale, scale));
            selected.sources()[0].replay_original_source(source_surface.canvas());
            let rgba_info = ImageInfo::new(
                size,
                skia_safe::ColorType::RGBA8888,
                AlphaType::Unpremul,
                None,
            );
            let stride = size.0 as usize * 4;
            let mut rgba = vec![0; stride * size.1 as usize];
            assert!(source_surface.read_pixels(&rgba_info, &mut rgba, stride, (0, 0)));
            let pixel = ((12.0 * scale) as usize * size.0 as usize + (14.0 * scale) as usize) * 4;
            assert_eq!(&rgba[pixel..pixel + 4], &[0, 128, 0, 255]);
            // Sources own the complete draw/resources/program, independent of
            // the original Document, fragment, recording and publishing thread.
            let selected = std::thread::spawn(move || selected).join().unwrap();
            let mut target = surfaces::raster_n32_premul(size).unwrap();
            target.canvas().scale((scale, scale));
            target.canvas().save();
            target.canvas().translate((0.25, 0.5));
            target.canvas().clip_rect(
                Rect::from_xywh(0.0, 0.0, 90.0, 65.0),
                ClipOp::Intersect,
                false,
            );
            let count = target.canvas().save_count();
            let matrix = target.canvas().local_to_device();
            let clip = target.canvas().device_clip_bounds();
            selected
                .replay_diagnostic(target.canvas(), (size.0 as u32, size.1 as u32))
                .unwrap();
            assert_eq!(target.canvas().save_count(), count);
            assert_eq!(target.canvas().local_to_device(), matrix);
            assert_eq!(target.canvas().device_clip_bounds(), clip);
            let first = bytes(&mut target, size);
            selected
                .replay_diagnostic(target.canvas(), (size.0 as u32, size.1 as u32))
                .unwrap();
            assert_eq!(bytes(&mut target, size), first);
            target.canvas().restore();
        }
    }

    fn capture_restore_to(
        above_current: bool,
        close_ancestors: bool,
    ) -> (Picture, LayeredPaintProgram) {
        let mut document = Document::new();
        let node = document.create_node(ElementTag::Div);
        let fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(40), LayoutUnit::from_i32(30)),
        );
        let mut recorder = PictureRecorder::new();
        let raw = recorder.begin_recording(BOUNDS, false);
        let capture = PaintProgramScope::new(raw);
        let canvas: &dyn PaintCanvas = raw;
        {
            let _opacity = canvas.opacity_scope();
            canvas.draw_color(Color::WHITE, skia_safe::BlendMode::Src);
            let outer_base = canvas.save();
            canvas.translate((6.0, 8.0));
            canvas.clip_rect(
                Rect::from_xywh(0.0, 0.0, 60.0, 48.0),
                ClipOp::Intersect,
                false,
            );
            {
                let _owner = PaintOwnerScope::new(raw, &fragment);
                record_fragment_property(
                    raw,
                    &fragment,
                    PaintPropertyDecision::Opacity {
                        alpha: 0.5,
                        will_change_transform: true,
                    },
                );
                canvas.save_layer_alpha_f(None, 0.5);
                if above_current {
                    let depth = canvas.save_count();
                    canvas.restore_to_count(depth + 1);
                    assert_eq!(canvas.save_count(), depth);
                }
                let mut paint = Paint::default();
                paint.set_color(Color::GREEN);
                canvas.draw_rect(Rect::from_xywh(1.0, 1.0, 32.0, 24.0), &paint);
                paint.set_color(Color::RED);
                canvas.draw_rect(Rect::from_xywh(15.0, 8.0, 24.0, 20.0), &paint);
                if close_ancestors {
                    canvas.restore_to_count(outer_base);
                } else {
                    canvas.restore();
                }
            }
            if !close_ancestors {
                canvas.restore();
            }
            // This sibling draw must retain its original transform/clip/alpha.
            let mut paint = Paint::default();
            paint.set_color(Color::BLUE);
            canvas.draw_rect(Rect::from_xywh(68.0, 52.0, 12.0, 10.0), &paint);
        }
        let program = capture.snapshot();
        assert_eq!(program.status(), PaintCaptureStatus::Complete);
        drop(capture);
        let picture = recorder.finish_recording_as_picture(None).unwrap();
        let layered = program
            .partition_layers(binding(1, false), |_| Some(binding(2, true)))
            .unwrap();
        (picture, layered)
    }

    #[test]
    fn restore_to_noop_and_ancestor_closure_preserve_intervals_pixels_and_host_state() {
        for above_current in [false, true] {
            for close_ancestors in [false, true] {
                for scale in [1.0_f32, 1.25, 1.5, 1.75, 2.0] {
                    let size = ((96.0 * scale) as i32, (72.0 * scale) as i32);
                    let (picture, layered) = capture_restore_to(above_current, close_ancestors);
                    let plan = layered.transparent_surface_program(BOUNDS).unwrap();
                    assert_eq!(plan.intervals().len(), 1);
                    assert_eq!(plan.sources().len(), 1);
                    assert_eq!(plan.intervals()[0].restore_to, close_ancestors.then_some(0));
                    let mut expected = surfaces::raster_n32_premul(size).unwrap();
                    expected.canvas().scale((scale, scale));
                    expected.canvas().draw_picture(&picture, None, None);
                    let mut actual = surfaces::raster_n32_premul(size).unwrap();
                    actual.canvas().scale((scale, scale));
                    actual.canvas().save();
                    let count = actual.canvas().save_count();
                    let matrix = actual.canvas().local_to_device();
                    let clip = actual.canvas().device_clip_bounds();
                    plan.replay_diagnostic(actual.canvas(), (size.0 as u32, size.1 as u32))
                        .unwrap();
                    assert_eq!(actual.canvas().save_count(), count);
                    assert_eq!(actual.canvas().local_to_device(), matrix);
                    assert_eq!(actual.canvas().device_clip_bounds(), clip);
                    assert_eq!(
                        bytes(&mut actual, size),
                        bytes(&mut expected, size),
                        "above={above_current} close={close_ancestors} scale={scale}"
                    );
                    actual.canvas().translate((0.25, 0.5));
                    actual.canvas().clip_rect(
                        Rect::from_xywh(0.0, 0.0, 90.0, 65.0),
                        ClipOp::Intersect,
                        false,
                    );
                    let count = actual.canvas().save_count();
                    let matrix = actual.canvas().local_to_device();
                    let clip = actual.canvas().device_clip_bounds();
                    plan.replay_diagnostic(actual.canvas(), (size.0 as u32, size.1 as u32))
                        .unwrap();
                    assert_eq!(actual.canvas().save_count(), count);
                    assert_eq!(actual.canvas().local_to_device(), matrix);
                    assert_eq!(actual.canvas().device_clip_bounds(), clip);
                    actual.canvas().restore();
                }
            }
        }
    }
}
