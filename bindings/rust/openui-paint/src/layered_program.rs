//! Owned intrinsic draws partitioned without changing the actual state spine.
//! This is diagnostic static replay. Sources are not independently composited
//! surfaces and no transform/effect/scroll track is admitted by this capture.

use crate::paint_program::{replay_operations, OwnedDraw, ProgramOperation};
use crate::{
    PaintCaptureStatus, PaintLayoutScopeRole, PaintOwnerPhase, PaintProgramChunk,
    PaintProgramOwner, RecordedPaintProgram,
};
use openui_dom::NodeId;
use skia_safe::Canvas;
use std::{collections::HashMap, sync::Arc};

/// Nonreused source key supplied by the publishing Engine's role registry.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaintLayerSourceId(pub u64);

/// Copied generation identity. It carries no DOM or callback reference.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PaintLayerNodeIdentity {
    pub document: u64,
    pub index: u32,
    pub generation: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintLayerSourceRole {
    AuthoredBox,
    GeneratedBox,
    DocumentContent,
    ViewportOverlay,
}

/// Binding of an actual paint owner to a retained Engine identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintLayerBinding {
    pub id: PaintLayerSourceId,
    pub owner: Option<PaintLayerNodeIdentity>,
    pub role: PaintLayerSourceRole,
}

impl PaintLayerBinding {
    fn valid(self) -> bool {
        self.id.0 != 0
            && match self.role {
                PaintLayerSourceRole::AuthoredBox | PaintLayerSourceRole::GeneratedBox => self
                    .owner
                    .is_some_and(|owner| owner.document != 0 && owner.generation != 0),
                PaintLayerSourceRole::DocumentContent | PaintLayerSourceRole::ViewportOverlay => {
                    self.owner.is_none()
                }
            }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayeredPaintError {
    InvalidBinding,
    ConflictingIdentity(PaintLayerSourceId),
    UnboundOwner(NodeId),
    UnbalancedOwnerScopes,
    InvalidDrawReference,
    IncompleteCapture(PaintCaptureStatus),
}

impl std::fmt::Display for LayeredPaintError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "layered paint capture: {self:?}")
    }
}
impl std::error::Error for LayeredPaintError {}

/// One uninterrupted sequence of global draws from this source. State changes
/// remain in the global spine; this interval is not a standalone replay program.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintLayerRun {
    pub first_operation: usize,
    pub end_operation: usize,
    pub first_source_draw: usize,
    pub draw_count: usize,
}

/// Provenance from the actual painter entry/contents scope, including borrowed
/// helpers. Neither these markers nor copied scene styles apply effects again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintLayerScope {
    pub owner: PaintProgramOwner,
    pub phase: PaintOwnerPhase,
    pub layout_role: PaintLayoutScopeRole,
    pub source: PaintLayerSourceId,
    pub first_operation: usize,
    pub end_operation: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintLayerDrawReference {
    pub operation: usize,
    pub source: PaintLayerSourceId,
    pub source_draw: usize,
    pub paint_owner: Option<PaintProgramOwner>,
}

/// State is not reconstructed from DOM ancestry or copied style metadata.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintLayerStateKind {
    Save,
    OpacityBarrier,
    SaveLayerAlpha,
    SaveLayer,
    Restore,
    RestoreTo,
    Translate,
    Scale,
    Rotate,
    Concat,
    Concat44,
    SetMatrix,
    ClipRect,
    ClipRRect,
    ClipPath,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaintLayerStateReference {
    pub operation: usize,
    pub kind: PaintLayerStateKind,
}

/// Coverage of nested sources is separate from complete static operation capture.
/// Image operands are owned but this outer capture does not classify whether
/// they hide a previously rasterized descendant subtree.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaintLayerCoverage {
    pub opaque_picture_draws: usize,
    pub unclassified_image_draws: usize,
    pub fixed_canvas_queries: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LayeredPaintQualification {
    UnqualifiedStaticReplay,
}

/// Intrinsic draw operands retain their images, paths, fonts and font-cache
/// lease. They intentionally expose no independently bounded replay API.
#[derive(Clone)]
pub struct RetainedPaintLayerSource {
    binding: PaintLayerBinding,
    chunks: Arc<[PaintProgramChunk]>,
    runs: Arc<[PaintLayerRun]>,
    retained_font_bytes: Arc<[Arc<[u8]>]>,
    _font_cache_lifetime: Option<openui_text::font::FontCacheLifetime>,
}

impl RetainedPaintLayerSource {
    pub fn binding(&self) -> PaintLayerBinding {
        self.binding
    }
    pub fn chunks(&self) -> &[PaintProgramChunk] {
        &self.chunks
    }
    pub fn runs(&self) -> &[PaintLayerRun] {
        &self.runs
    }
    pub fn retained_font_face_count(&self) -> usize {
        self.retained_font_bytes.len()
    }
}

#[derive(Clone, Copy)]
struct DrawLocation {
    source: usize,
    draw: usize,
}

/// Immutable layered lookup over the exact captured sequence. Only Draw
/// references are redirected; every state/clip/effect operation is retained
/// once, in its original order. Sources can have several interleaved runs.
#[derive(Clone)]
pub struct LayeredPaintProgram {
    operations: Arc<[ProgramOperation]>,
    draws: Arc<[DrawLocation]>,
    sources: Arc<[RetainedPaintLayerSource]>,
    draw_references: Arc<[PaintLayerDrawReference]>,
    state_references: Arc<[PaintLayerStateReference]>,
    scopes: Arc<[PaintLayerScope]>,
    status: PaintCaptureStatus,
    coverage: PaintLayerCoverage,
}

struct SourceBuilder {
    binding: PaintLayerBinding,
    chunks: Vec<PaintProgramChunk>,
    runs: Vec<PaintLayerRun>,
}

impl RecordedPaintProgram {
    /// Bind actual authored paint scopes on the UI thread. The resolver must
    /// return the publishing Engine's live generation/role identity. Borrowed
    /// text/table/fieldset scopes inherit the nearest authored scope; they do
    /// not introduce another authored effect. Missing authored identities fail.
    pub fn partition_layers(
        &self,
        fallback: PaintLayerBinding,
        mut resolve: impl FnMut(NodeId) -> Option<PaintLayerBinding>,
    ) -> Result<LayeredPaintProgram, LayeredPaintError> {
        if !fallback.valid() || fallback.owner.is_some() {
            return Err(LayeredPaintError::InvalidBinding);
        }
        let mut sources = vec![SourceBuilder {
            binding: fallback,
            chunks: Vec::new(),
            runs: Vec::new(),
        }];
        let mut indices: HashMap<PaintLayerSourceId, usize> = HashMap::from([(fallback.id, 0)]);
        let mut scopes: Vec<PaintLayerScope> = Vec::new();
        let mut stack: Vec<(PaintProgramOwner, PaintLayerBinding, usize)> = Vec::new();
        let mut locations = vec![None; self.chunks.len()];
        let mut references = Vec::new();
        let mut states = Vec::new();
        let mut previous_source = None;
        let mut coverage = PaintLayerCoverage {
            fixed_canvas_queries: self.canvas_queries,
            ..Default::default()
        };
        for (operation_index, operation) in self.operations.iter().enumerate() {
            match operation {
                ProgramOperation::EnterOwner(owner, phase, layout_role) => {
                    let inherited = stack
                        .last()
                        .map(|(_, binding, _)| *binding)
                        .unwrap_or(fallback);
                    let binding = if owner.owns_element_box && !owner.node_id.is_none() {
                        resolve(owner.node_id)
                            .ok_or(LayeredPaintError::UnboundOwner(owner.node_id))?
                    } else {
                        inherited
                    };
                    if !binding.valid() {
                        return Err(LayeredPaintError::InvalidBinding);
                    }
                    if let Some(index) = indices.get(&binding.id).copied() {
                        if sources[index].binding != binding {
                            return Err(LayeredPaintError::ConflictingIdentity(binding.id));
                        }
                    } else {
                        indices.insert(binding.id, sources.len());
                        sources.push(SourceBuilder {
                            binding,
                            chunks: Vec::new(),
                            runs: Vec::new(),
                        });
                    }
                    let index = scopes.len();
                    scopes.push(PaintLayerScope {
                        owner: *owner,
                        phase: *phase,
                        layout_role: *layout_role,
                        source: binding.id,
                        first_operation: operation_index,
                        end_operation: 0,
                    });
                    stack.push((*owner, binding, index));
                }
                ProgramOperation::ExitOwner => {
                    let (_, _, index) = stack
                        .pop()
                        .ok_or(LayeredPaintError::UnbalancedOwnerScopes)?;
                    scopes[index].end_operation = operation_index + 1;
                }
                ProgramOperation::Draw(draw_index) => {
                    let chunk = self
                        .chunks
                        .get(*draw_index)
                        .ok_or(LayeredPaintError::InvalidDrawReference)?;
                    if locations[*draw_index].is_some()
                        || chunk.owner() != stack.last().map(|(owner, _, _)| *owner)
                    {
                        return Err(LayeredPaintError::InvalidDrawReference);
                    }
                    let binding = stack
                        .last()
                        .map(|(_, binding, _)| *binding)
                        .unwrap_or(fallback);
                    let source = if let Some(index) = indices.get(&binding.id).copied() {
                        if sources[index].binding != binding {
                            return Err(LayeredPaintError::ConflictingIdentity(binding.id));
                        }
                        index
                    } else {
                        let index = sources.len();
                        indices.insert(binding.id, index);
                        sources.push(SourceBuilder {
                            binding,
                            chunks: Vec::new(),
                            runs: Vec::new(),
                        });
                        index
                    };
                    let builder = &mut sources[source];
                    let draw = builder.chunks.len();
                    builder.chunks.push(chunk.clone());
                    if previous_source == Some(source) {
                        let run = builder.runs.last_mut().expect("preceding source run");
                        run.end_operation = operation_index + 1;
                        run.draw_count += 1;
                    } else {
                        builder.runs.push(PaintLayerRun {
                            first_operation: operation_index,
                            end_operation: operation_index + 1,
                            first_source_draw: draw,
                            draw_count: 1,
                        });
                    }
                    previous_source = Some(source);
                    locations[*draw_index] = Some(DrawLocation { source, draw });
                    references.push(PaintLayerDrawReference {
                        operation: operation_index,
                        source: binding.id,
                        source_draw: draw,
                        paint_owner: chunk.owner(),
                    });
                    match &chunk.draw {
                        OwnedDraw::Picture(..) => coverage.opaque_picture_draws += 1,
                        OwnedDraw::Image(..) | OwnedDraw::ImageNine(..) => {
                            coverage.unclassified_image_draws += 1
                        }
                        _ => {}
                    }
                }
                other => states.push(PaintLayerStateReference {
                    operation: operation_index,
                    kind: state_kind(other),
                }),
            }
        }
        if !stack.is_empty() {
            return Err(LayeredPaintError::UnbalancedOwnerScopes);
        }
        let draws = locations
            .into_iter()
            .collect::<Option<Vec<_>>>()
            .ok_or(LayeredPaintError::InvalidDrawReference)?;
        let sources = sources
            .into_iter()
            .map(|source| RetainedPaintLayerSource {
                binding: source.binding,
                chunks: source.chunks.into(),
                runs: source.runs.into(),
                retained_font_bytes: self.retained_font_bytes.clone(),
                _font_cache_lifetime: self.font_cache_lifetime.clone(),
            })
            .collect::<Vec<_>>();
        Ok(LayeredPaintProgram {
            operations: self.operations.clone().into(),
            draws: draws.into(),
            sources: sources.into(),
            draw_references: references.into(),
            state_references: states.into(),
            scopes: scopes.into(),
            status: self.status(),
            coverage,
        })
    }
}

fn state_kind(operation: &ProgramOperation) -> PaintLayerStateKind {
    use PaintLayerStateKind as K;
    match operation {
        ProgramOperation::Save => K::Save,
        ProgramOperation::OpacityBarrier => K::OpacityBarrier,
        ProgramOperation::SaveLayerAlpha(..) => K::SaveLayerAlpha,
        ProgramOperation::SaveLayer { .. } => K::SaveLayer,
        ProgramOperation::Restore => K::Restore,
        ProgramOperation::RestoreTo(..) => K::RestoreTo,
        ProgramOperation::Translate(..) => K::Translate,
        ProgramOperation::Scale(..) => K::Scale,
        ProgramOperation::Rotate(..) => K::Rotate,
        ProgramOperation::Concat(..) => K::Concat,
        ProgramOperation::Concat44(..) => K::Concat44,
        ProgramOperation::SetMatrix(..) => K::SetMatrix,
        ProgramOperation::ClipRect(..) => K::ClipRect,
        ProgramOperation::ClipRRect(..) => K::ClipRRect,
        ProgramOperation::ClipPath(..) => K::ClipPath,
        ProgramOperation::Draw(..)
        | ProgramOperation::EnterOwner(..)
        | ProgramOperation::ExitOwner => unreachable!("non-state operation"),
    }
}

impl LayeredPaintProgram {
    pub fn sources(&self) -> &[RetainedPaintLayerSource] {
        &self.sources
    }
    pub fn draw_references(&self) -> &[PaintLayerDrawReference] {
        &self.draw_references
    }
    pub fn state_references(&self) -> &[PaintLayerStateReference] {
        &self.state_references
    }
    pub fn scopes(&self) -> &[PaintLayerScope] {
        &self.scopes
    }
    pub fn operation_count(&self) -> usize {
        self.operations.len()
    }
    pub fn capture_status(&self) -> &PaintCaptureStatus {
        &self.status
    }
    pub fn coverage(&self) -> PaintLayerCoverage {
        self.coverage
    }
    pub fn qualification(&self) -> LayeredPaintQualification {
        LayeredPaintQualification::UnqualifiedStaticReplay
    }
    pub fn dynamic_tracks_are_admitted(&self) -> bool {
        false
    }
    /// Draw operands are partitioned; this does not claim independently bounded
    /// pictures or recursive ownership of children hidden in opaque operands.
    pub fn intrinsic_draws_are_partitioned(&self) -> bool {
        true
    }
    pub fn replay(&self, canvas: &Canvas) -> Result<(), LayeredPaintError> {
        if self.status != PaintCaptureStatus::Complete {
            return Err(LayeredPaintError::IncompleteCapture(self.status.clone()));
        }
        replay_operations(&self.operations, canvas, |index| {
            let location = self.draws[index];
            &self.sources[location.source].chunks[location.draw].draw
        });
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paint_program::{PaintOwnerScope, PaintProgramScope};
    use crate::paint_record::PaintCanvas;
    use openui_dom::{Document, ElementTag};
    use openui_geometry::{LayoutUnit, PhysicalSize};
    use openui_layout::{Fragment, FragmentBoxOwnership};
    use skia_safe::{surfaces, Color, Paint, Picture, PictureRecorder, Rect};

    fn fallback() -> PaintLayerBinding {
        PaintLayerBinding {
            id: PaintLayerSourceId(1),
            owner: None,
            role: PaintLayerSourceRole::DocumentContent,
        }
    }
    fn binding(id: u64, index: u32) -> PaintLayerBinding {
        PaintLayerBinding {
            id: PaintLayerSourceId(id),
            owner: Some(PaintLayerNodeIdentity {
                document: 1,
                index,
                generation: 1,
            }),
            role: PaintLayerSourceRole::AuthoredBox,
        }
    }
    fn captured() -> (Picture, RecordedPaintProgram, NodeId, NodeId) {
        let mut doc = Document::new();
        let a = doc.create_node(ElementTag::Div);
        let b = doc.create_node(ElementTag::Div);
        let size = PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20));
        let box_a = Fragment::new_box(a, size);
        let box_b = Fragment::new_box(b, size);
        let mut helper = box_a.clone();
        helper.box_ownership = FragmentBoxOwnership::LayoutHelper;
        let mut recorder = PictureRecorder::new();
        let raw = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 96.0, 72.0), false);
        let scope = PaintProgramScope::new(raw);
        let canvas: &dyn PaintCanvas = raw;
        {
            let _opacity = canvas.opacity_scope();
            canvas.clear(Color::WHITE);
            canvas.save();
            canvas.translate((0.25, 0.75));
            canvas.save_layer_alpha_f(None, 0.5);
            {
                let _owner_a = PaintOwnerScope::new(raw, &box_a);
                let mut paint = Paint::default();
                paint.set_color(Color::RED);
                paint.set_anti_alias(true);
                canvas.draw_rect(Rect::from_xywh(4.0, 5.0, 20.0, 20.0), &paint);
                {
                    let _owner_b = PaintOwnerScope::new(raw, &box_b);
                    canvas.save();
                    canvas.clip_rect(
                        Rect::from_xywh(0.0, 0.0, 70.0, 60.0),
                        skia_safe::ClipOp::Intersect,
                        true,
                    );
                    paint.set_color(Color::BLUE);
                    canvas.draw_rect(Rect::from_xywh(14.0, 15.0, 20.0, 20.0), &paint);
                    canvas.restore();
                }
                {
                    let _helper = PaintOwnerScope::contents(raw, &helper);
                    paint.set_color(Color::GREEN);
                    canvas.draw_rect(Rect::from_xywh(24.0, 25.0, 20.0, 20.0), &paint);
                }
            }
            canvas.restore();
            canvas.restore();
        }
        let program = scope.snapshot();
        drop(scope);
        (
            recorder.finish_recording_as_picture(None).unwrap(),
            program,
            a,
            b,
        )
    }
    fn image_bytes(surface: &mut skia_safe::Surface) -> Vec<u8> {
        let image = surface.image_snapshot();
        let info = image
            .image_info()
            .with_color_type(skia_safe::ColorType::RGBA8888);
        let mut bytes = vec![0; info.compute_min_byte_size()];
        assert!(image.read_pixels(
            &info,
            &mut bytes,
            info.min_row_bytes(),
            (0, 0),
            skia_safe::image::CachingHint::Allow
        ));
        bytes
    }
    #[test]
    fn intrinsic_runs_preserve_interleaved_owners_and_borrowed_scope_provenance() {
        let (_, program, a, b) = captured();
        let layered = program
            .partition_layers(fallback(), |node| {
                if node == a {
                    Some(binding(10, 1))
                } else if node == b {
                    Some(binding(20, 2))
                } else {
                    None
                }
            })
            .unwrap();
        assert_eq!(
            layered
                .draw_references()
                .iter()
                .map(|reference| reference.source.0)
                .collect::<Vec<_>>(),
            vec![1, 10, 20, 10]
        );
        let source = layered
            .sources()
            .iter()
            .find(|s| s.binding().id.0 == 10)
            .unwrap();
        assert_eq!(source.runs().len(), 2);
        assert_eq!(source.chunks().len(), 2);
        assert!(!source.chunks()[1].owner().unwrap().owns_element_box);
        assert!(layered
            .scopes()
            .iter()
            .any(|scope| scope.phase == PaintOwnerPhase::Contents
                && !scope.owner.owns_element_box
                && scope.source.0 == 10));
        assert_eq!(layered.operation_count(), program.operation_count());
        assert_eq!(
            layered.qualification(),
            LayeredPaintQualification::UnqualifiedStaticReplay
        );
        assert!(!layered.dynamic_tracks_are_admitted());
        assert!(layered.intrinsic_draws_are_partitioned());
    }
    #[test]
    fn layered_state_spine_preserves_host_matrix_clip_stack_and_repeated_pixels() {
        let (_, program, a, b) = captured();
        let layered = program
            .partition_layers(fallback(), |node| {
                Some(if node == a {
                    binding(10, 1)
                } else {
                    assert_eq!(node, b);
                    binding(20, 2)
                })
            })
            .unwrap();
        for scale in [1.0_f32, 1.25, 1.5, 1.75, 2.0] {
            for phase in [0.0_f32, 0.25, 0.75] {
                let render = |partitioned: bool| {
                    let mut surface = surfaces::raster_n32_premul((240, 180)).unwrap();
                    let canvas = surface.canvas();
                    canvas.clear(Color::WHITE);
                    canvas.save();
                    canvas.translate((11.0 + phase, 7.0));
                    canvas.scale((scale, scale));
                    canvas.clip_rect(
                        Rect::from_xywh(0.0, 0.0, 90.0, 70.0),
                        skia_safe::ClipOp::Intersect,
                        true,
                    );
                    let matrix = canvas.local_to_device();
                    let count = canvas.save_count();
                    for _ in 0..2 {
                        if partitioned {
                            layered.replay(canvas).unwrap();
                        } else {
                            program.replay(canvas).unwrap();
                        }
                        assert_eq!(canvas.local_to_device(), matrix);
                        assert_eq!(canvas.save_count(), count);
                    }
                    let mut paint = Paint::default();
                    paint.set_color(Color::BLUE);
                    canvas.draw_rect(Rect::from_xywh(60.0, 50.0, 5.0, 5.0), &paint);
                    canvas.restore();
                    image_bytes(&mut surface)
                };
                assert_eq!(render(true), render(false), "scale {scale} phase {phase}");
            }
        }
    }
    #[test]
    fn invalid_binding_scopes_and_draws_fail_before_replay_or_host_mutation() {
        let (_, program, a, b) = captured();
        assert!(matches!(
            program.partition_layers(fallback(), |_| None),
            Err(LayeredPaintError::UnboundOwner(_))
        ));
        assert!(matches!(
            program.partition_layers(fallback(), |node| Some(binding(
                10,
                if node == a { 1 } else { 2 }
            ))),
            Err(LayeredPaintError::ConflictingIdentity(_))
        ));
        let resolve = |node| {
            Some(if node == a {
                binding(10, 1)
            } else {
                assert_eq!(node, b);
                binding(20, 2)
            })
        };
        let mut broken = program.clone();
        broken.operations.push(ProgramOperation::ExitOwner);
        assert!(matches!(
            broken.partition_layers(fallback(), resolve),
            Err(LayeredPaintError::UnbalancedOwnerScopes)
        ));
        let mut broken = program.clone();
        broken.operations.push(ProgramOperation::Draw(usize::MAX));
        assert!(matches!(
            broken.partition_layers(fallback(), resolve),
            Err(LayeredPaintError::InvalidDrawReference)
        ));
        let mut broken = program.clone();
        broken
            .reasons
            .push(crate::PaintCaptureIncompleteReason::RawCanvasAccess);
        let layered = broken.partition_layers(fallback(), resolve).unwrap();
        let mut surface = surfaces::raster_n32_premul((20, 20)).unwrap();
        surface.canvas().clear(Color::RED);
        let before = image_bytes(&mut surface);
        let count = surface.canvas().save_count();
        assert!(matches!(
            layered.replay(surface.canvas()),
            Err(LayeredPaintError::IncompleteCapture(_))
        ));
        assert_eq!(surface.canvas().save_count(), count);
        assert_eq!(image_bytes(&mut surface), before);
    }
    #[test]
    fn immutable_layer_sources_are_send_and_sync() {
        fn require<T: Send + Sync>() {}
        require::<LayeredPaintProgram>();
        require::<RetainedPaintLayerSource>();
    }
}
