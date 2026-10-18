//! Owned source-layer diagnostics. Possessing a source or a paint item does not
//! establish an independently composited layer or qualify a release backend.

use crate::paint_program::{
    replay_operations, replay_operations_with, OwnedDraw, ProgramOperation,
};
use crate::{LayeredPaintProgram, PaintCaptureStatus, PaintShaderDependency};
use crate::{PaintLayerBinding, PaintLayoutScopeRole, PaintProgramOwner};
use openui_dom::NodeId;
use skia_safe::canvas::SrcRectConstraint;
use skia_safe::{
    BlendMode, ClipOp, Color4f, Contains, IRect, Matrix, Paint, Picture, PictureRecorder,
    SamplingOptions, M44,
};
use skia_safe::{Canvas, Rect};
use std::sync::Arc;

/// A drawing phase recorded where the painter actually starts the item. The
/// owner scope is separate: one owner can contribute several drawing items.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaintDisplayItemRole {
    BoxDecorationBackground,
    DocumentBackground,
    Foreground,
    ReplacedContent,
    Outline,
}

/// Containing-block identity resolved by layout, without a live document borrow.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePositioningReference {
    InFlow,
    Viewport { root: PaintLayerBinding },
    ContainingBox { container: PaintLayerBinding },
    Unresolved,
}

/// This identity follows the retained source generation and structural role.
/// Physical fragments are projections of it, not new lifetime identities.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePaintContextIdentity {
    pub source: crate::PaintLayerSourceId,
    pub layout_role: PaintLayoutScopeRole,
}

/// Copied operands from an actual painter entry. A viewport reference is not
/// inferred from a tag, the size of a box or the spelling of a fixture.
#[derive(Clone, Debug, PartialEq)]
pub struct SourcePaintContextProjection {
    pub identity: SourcePaintContextIdentity,
    pub binding: PaintLayerBinding,
    pub parent: Option<SourcePaintContextIdentity>,
    pub owns_element_box: bool,
    pub border_box: Rect,
    pub rounded_paint_offset: (f32, f32),
    pub paint_offset_residual: (f32, f32),
    pub positioning: SourcePositioningReference,
    pub scroll_owner: Option<PaintLayerBinding>,
    pub is_viewport: bool,
    pub will_change_transform: bool,
    pub owns_transform_containing_block: bool,
    pub paint_isolation: bool,
    pub first_operation: usize,
    pub end_operation: usize,
    /// Exact state at entry. The owned operands are retained on the state spine.
    pub transform: Option<SourcePaintPropertyIdentity>,
    pub clip: Option<SourcePaintPropertyIdentity>,
    pub effect: Option<SourcePaintPropertyIdentity>,
    pub scroll_translation: Option<SourcePaintPropertyIdentity>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLayerStrategy {
    SolidColor,
    OpaquePicture,
    TransparentPicture,
}

/// Static source/geometry/ownership admission is separate from exact Chromium
/// pixel qualification, final Viz material attribution and presentation tracks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLayerQualification {
    UnqualifiedStaticDiagnostic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawPositioningReference {
    InFlow,
    Viewport(NodeId),
    ContainingBox(NodeId),
    Unresolved,
}

#[derive(Clone, Debug)]
pub(crate) struct PaintSourceContextInput {
    pub owner: PaintProgramOwner,
    pub layout_role: PaintLayoutScopeRole,
    pub border_box: Rect,
    pub rounded_paint_offset: (f32, f32),
    pub paint_offset_residual: (f32, f32),
    pub positioning: RawPositioningReference,
    pub scroll_owner: Option<NodeId>,
    pub is_viewport: bool,
    pub will_change_transform: bool,
    pub owns_transform_containing_block: bool,
    pub paint_isolation: bool,
}

/// Capture-only node references are bound to live generations before publication.
/// Operation positions locate the exact state spine; they are never layer IDs.
#[derive(Clone)]
pub(crate) enum PaintSourceEventKind {
    EnterContext(PaintSourceContextInput),
    ExitContext,
    EnterItem {
        owner: PaintProgramOwner,
        role: PaintDisplayItemRole,
    },
    ExitItem,
    CanvasQuery,
    ScrollTranslation {
        node: NodeId,
    },
    NestedViewport {
        node: NodeId,
        bounds: Rect,
        scroll_translation: (f32, f32),
    },
}

#[derive(Clone)]
pub(crate) struct PaintSourceEvent {
    pub operation: usize,
    pub save_depth: usize,
    pub kind: PaintSourceEventKind,
}

pub(crate) fn event_nodes(kind: &PaintSourceEventKind) -> Vec<NodeId> {
    match kind {
        PaintSourceEventKind::EnterContext(input) => {
            let mut result = vec![input.owner.node_id];
            match input.positioning {
                RawPositioningReference::Viewport(node)
                | RawPositioningReference::ContainingBox(node) => result.push(node),
                RawPositioningReference::InFlow | RawPositioningReference::Unresolved => {}
            }
            result.extend(input.scroll_owner);
            result
        }
        PaintSourceEventKind::EnterItem { owner, .. } => vec![owner.node_id],
        PaintSourceEventKind::NestedViewport { node, .. } => vec![*node],
        PaintSourceEventKind::ScrollTranslation { node } => vec![*node],
        PaintSourceEventKind::ExitContext
        | PaintSourceEventKind::ExitItem
        | PaintSourceEventKind::CanvasQuery => Vec::new(),
    }
}

/// Annotation scopes touch no canonical canvas state and retain no document.
pub(crate) struct PaintSourceContextScope<'a> {
    canvas: &'a Canvas,
}

impl<'a> PaintSourceContextScope<'a> {
    pub(crate) fn new(canvas: &'a Canvas, input: PaintSourceContextInput) -> Self {
        crate::paint_program::record_source_event(
            canvas,
            PaintSourceEventKind::EnterContext(input),
        );
        Self { canvas }
    }
}

impl Drop for PaintSourceContextScope<'_> {
    fn drop(&mut self) {
        crate::paint_program::record_source_event(self.canvas, PaintSourceEventKind::ExitContext);
    }
}

pub(crate) struct PaintDisplayItemScope<'a> {
    canvas: &'a Canvas,
}

impl<'a> PaintDisplayItemScope<'a> {
    pub(crate) fn new(
        canvas: &'a Canvas,
        owner: PaintProgramOwner,
        role: PaintDisplayItemRole,
    ) -> Self {
        crate::paint_program::record_source_event(
            canvas,
            PaintSourceEventKind::EnterItem { owner, role },
        );
        Self { canvas }
    }
}

impl Drop for PaintDisplayItemScope<'_> {
    fn drop(&mut self) {
        crate::paint_program::record_source_event(self.canvas, PaintSourceEventKind::ExitItem);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourcePaintPropertyKind {
    Transform,
    Clip,
    Effect,
    ScrollTranslation,
}

/// Stable source identity plus a real property occurrence in its paint role.
/// This is a diagnostic key, not a browser property-tree object identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourcePaintPropertyIdentity {
    pub source: crate::PaintLayerSourceId,
    pub kind: SourcePaintPropertyKind,
    pub occurrence: u32,
}

#[derive(Clone, Debug)]
pub struct SourcePaintPropertyProjection {
    pub identity: SourcePaintPropertyIdentity,
    pub parent: Option<SourcePaintPropertyIdentity>,
    pub context: Option<SourcePaintContextIdentity>,
    pub operation: usize,
    pub end_operation: usize,
    /// Full matrix, including perspective. Extraction accepts finite affine2D only.
    pub matrix: [f32; 16],
    pub clip_bounds: Option<Rect>,
    pub alpha: Option<f32>,
}

#[derive(Clone, Debug)]
pub struct SourceDisplayItemProjection {
    pub binding: PaintLayerBinding,
    pub context: Option<SourcePaintContextIdentity>,
    pub role: PaintDisplayItemRole,
    pub first_operation: usize,
    pub end_operation: usize,
    /// Counts actual record state and drawing operations, excluding ownership annotations.
    pub record_operation_count: usize,
    pub draw_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLayerAdmission {
    ViewportContainingBlock,
    TransformHint,
    PaintIsolation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLayerUnavailable {
    OrdinaryPaint,
    BorrowedLayoutBox,
    UnresolvedBinding,
    IncompleteCapture,
    CanvasQuery,
    OpaquePictureDependency,
    ShaderGraph,
    FilterDependency,
    UnsupportedBounds,
    UnsupportedTransform,
    UnbalancedState,
    NestedLayer,
    BackdropBlendDependency,
}

#[derive(Clone, Debug)]
pub struct SourceLayerCandidateProjection {
    pub context: SourcePaintContextIdentity,
    pub first_operation: usize,
    pub end_operation: usize,
    pub admission: Option<SourceLayerAdmission>,
    pub unavailable: Option<SourceLayerUnavailable>,
    pub source_index: Option<usize>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceLayerOrderDecision {
    DirectPropertyBoundary,
    OverlapBarrier,
    ContiguousSameSourceMerge,
    InterveningOperationBarrier,
    UpcastRevokesSolid,
}
#[derive(Clone, Debug)]
pub struct SourceLayerOrderProjection {
    pub home: SourcePaintContextIdentity,
    pub guest: SourcePaintContextIdentity,
    pub decision: SourceLayerOrderDecision,
}

#[derive(Clone)]
pub struct RetainedSourcePaintLayer {
    binding: PaintLayerBinding,
    context: SourcePaintContextIdentity,
    strategy: SourceLayerStrategy,
    bounds: Rect,
    known_opaque: Rect,
    background_hint: Color4f,
    solid_color: Option<Color4f>,
    solid_blend: BlendMode,
    picture: Picture,
    first_operation: usize,
    end_operation: usize,
    entry_matrix: Matrix,
    owner_translation: Option<(SourcePaintPropertyIdentity, Matrix)>,
    // Own both the exact operands and all font/image/path leases.
    _program: Arc<LayeredPaintProgram>,
}

impl RetainedSourcePaintLayer {
    pub fn binding(&self) -> PaintLayerBinding {
        self.binding
    }
    pub fn context(&self) -> SourcePaintContextIdentity {
        self.context
    }
    pub fn strategy(&self) -> SourceLayerStrategy {
        self.strategy
    }
    /// Drawing coordinates before the retained owner translation, separate from
    /// the ancestor state. The drawing rectangle is not a transformed AABB.
    pub fn bounds(&self) -> Rect {
        self.bounds
    }
    /// An owned property retained outside the single drawing record. Other
    /// transforms remain in the source or on the original ancestor spine.
    pub fn owner_translation(&self) -> Option<(SourcePaintPropertyIdentity, Matrix)> {
        self.owner_translation
    }
    fn entry_bounds(&self) -> Rect {
        self.owner_translation
            .map_or(self.bounds, |(_, matrix)| matrix.map_rect(self.bounds).0)
    }
    pub fn rect_known_to_be_opaque(&self) -> Rect {
        self.known_opaque
    }
    pub fn background_hint(&self) -> Color4f {
        self.background_hint
    }
    pub fn qualification(&self) -> SourceLayerQualification {
        SourceLayerQualification::UnqualifiedStaticDiagnostic
    }
    pub fn dynamic_tracks_are_admitted(&self) -> bool {
        false
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SourceLayerError {
    InvalidBounds,
    InvalidHierarchy,
    InvalidBinding,
    IncompleteCapture(PaintCaptureStatus),
    AllocationFailure,
    RecordingFailure,
}
impl std::fmt::Display for SourceLayerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SourceLayerError {}

#[derive(Clone)]
struct State {
    matrix: Matrix,
    transform: Option<SourcePaintPropertyIdentity>,
    clip: Option<SourcePaintPropertyIdentity>,
    effect: Option<SourcePaintPropertyIdentity>,
    scroll: Option<SourcePaintPropertyIdentity>,
    clip_ops: Vec<usize>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            matrix: Matrix::new_identity(),
            transform: None,
            clip: None,
            effect: None,
            scroll: None,
            clip_ops: Vec::new(),
        }
    }
}

#[derive(Clone)]
pub struct SourceLayerProgram {
    program: Arc<LayeredPaintProgram>,
    contexts: Arc<[SourcePaintContextProjection]>,
    items: Arc<[SourceDisplayItemProjection]>,
    properties: Arc<[SourcePaintPropertyProjection]>,
    candidates: Arc<[SourceLayerCandidateProjection]>,
    sources: Arc<[RetainedSourcePaintLayer]>,
    nested: Arc<[(usize, SourceLayerProgram)]>,
    nested_matrices: Arc<[Matrix]>,
    ordering: Arc<[SourceLayerOrderProjection]>,
}

fn finite_rect(r: Rect) -> bool {
    [r.left, r.top, r.right, r.bottom]
        .into_iter()
        .all(f32::is_finite)
        && !r.is_empty()
}
fn join(a: Rect, b: Rect) -> Rect {
    if a.is_empty() {
        b
    } else if b.is_empty() {
        a
    } else {
        Rect::from_ltrb(
            a.left.min(b.left),
            a.top.min(b.top),
            a.right.max(b.right),
            a.bottom.max(b.bottom),
        )
    }
}
fn intersection(a: Rect, b: Rect) -> Rect {
    let r = Rect::from_ltrb(
        a.left.max(b.left),
        a.top.max(b.top),
        a.right.min(b.right),
        a.bottom.min(b.bottom),
    );
    if r.is_empty() {
        Rect::default()
    } else {
        r
    }
}
fn covered_union(a: Rect, b: Rect) -> Rect {
    if a.is_empty() || b.contains(a) {
        return b;
    }
    if b.is_empty() || a.contains(b) {
        return a;
    }
    if (a.left == b.left && a.right == b.right && a.top <= b.bottom && b.top <= a.bottom)
        || (a.top == b.top && a.bottom == b.bottom && a.left <= b.right && b.left <= a.right)
    {
        return join(a, b);
    }
    if a.width() * a.height() >= b.width() * b.height() {
        a
    } else {
        b
    }
}
fn affine(m: &M44) -> Option<Matrix> {
    if (0..4).any(|r| (0..4).any(|c| !m.rc(r, c).is_finite()))
        || m.rc(3, 0) != 0.0
        || m.rc(3, 1) != 0.0
        || m.rc(3, 3) != 1.0
        || m.rc(0, 2) != 0.0
        || m.rc(1, 2) != 0.0
        || m.rc(2, 0) != 0.0
        || m.rc(2, 1) != 0.0
        || m.rc(2, 3) != 0.0
        || m.rc(3, 2) != 0.0
    {
        return None;
    }
    Some(m.to_m33())
}
fn matrix_array(m: &Matrix) -> [f32; 16] {
    let m = M44::from(m);
    std::array::from_fn(|i| m.rc(i / 4, i % 4))
}
fn bound_draw(draw: &OwnedDraw) -> Option<Rect> {
    let (rect, paint) = match draw {
        OwnedDraw::Rect(r, p) | OwnedDraw::Oval(r, p) | OwnedDraw::RoundRect(r, _, _, p) => {
            (*r, Some(p))
        }
        OwnedDraw::RRect(r, p) | OwnedDraw::DRRect(r, _, p) => (r.rect().to_owned(), Some(p)),
        OwnedDraw::Circle(c, r, p) => {
            (Rect::from_xywh(c.x - r, c.y - r, 2.0 * r, 2.0 * r), Some(p))
        }
        OwnedDraw::Line(a, b, p) => (
            Rect::from_ltrb(a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)),
            Some(p),
        ),
        // Contour bounds do not bound inverse fill. Keep the original owned
        // draw unless its effective clip can be bounded by a future extractor.
        OwnedDraw::Path(p, paint) => (
            p.with_path(|p| (!p.is_inverse_fill_type()).then(|| *p.bounds()))?,
            Some(paint),
        ),
        OwnedDraw::Image(_, _, r, _, p) => (*r, Some(p)),
        OwnedDraw::ImageNine(_, _, r, _, p) => (*r, p.as_ref()),
        _ => return None,
    };
    let expand = paint
        .filter(|p| p.style() != skia_safe::paint::Style::Fill)
        .map_or(0.0, |p| {
            p.stroke_width().max(1.0) * p.stroke_miter().max(1.0) * 0.5
        });
    let r = Rect::from_ltrb(
        rect.left - expand,
        rect.top - expand,
        rect.right + expand,
        rect.bottom + expand,
    );
    finite_rect(r).then_some(r)
}
fn solid_flags(p: &Paint) -> bool {
    p.style() == skia_safe::paint::Style::Fill
        && p.shader().is_none()
        && p.color_filter().is_none()
        && p.image_filter().is_none()
        && p.mask_filter().is_none()
        && p.path_effect().is_none()
        && matches!(p.blend_mode(), BlendMode::Src | BlendMode::SrcOver)
}
fn annotation(op: &ProgramOperation) -> bool {
    matches!(
        op,
        ProgramOperation::EnterOwner(..)
            | ProgramOperation::ExitOwner
            | ProgramOperation::Property(..)
            | ProgramOperation::OpacityBarrier
    )
}

impl LayeredPaintProgram {
    /// Build from actual item/context annotations and the owned operation spine.
    pub fn source_layer_program(&self) -> Result<SourceLayerProgram, SourceLayerError> {
        SourceLayerProgram::new(self.clone(), None)
    }
}

impl SourceLayerProgram {
    pub(crate) fn new(
        program: LayeredPaintProgram,
        child: Option<SourceLayerProgram>,
    ) -> Result<Self, SourceLayerError> {
        let program = Arc::new(program);
        let binding = |n: NodeId| {
            program
                .source_bindings
                .iter()
                .find(|(node, _)| *node == n)
                .map(|(_, b)| *b)
        };
        let mut contexts: Vec<SourcePaintContextProjection> = Vec::new();
        let mut items: Vec<SourceDisplayItemProjection> = Vec::new();
        let mut context_stack = Vec::new();
        let mut item_stack = Vec::new();
        let mut depths = Vec::new();
        let mut queries = Vec::new();
        let mut nested = Vec::new();
        let mut scroll_events = Vec::new();
        for event in program.source_events.iter() {
            if event.operation > program.operation_count() {
                return Err(SourceLayerError::InvalidHierarchy);
            }
            match &event.kind {
                PaintSourceEventKind::EnterContext(input) => {
                    if ![
                        input.border_box.left,
                        input.border_box.top,
                        input.border_box.right,
                        input.border_box.bottom,
                        input.rounded_paint_offset.0,
                        input.rounded_paint_offset.1,
                        input.paint_offset_residual.0,
                        input.paint_offset_residual.1,
                    ]
                    .into_iter()
                    .all(f32::is_finite)
                    {
                        return Err(SourceLayerError::InvalidBounds);
                    }
                    let b = binding(input.owner.node_id)
                        .or_else(|| {
                            (!input.owner.owns_element_box).then(|| {
                                context_stack
                                    .last()
                                    .map(|i: &usize| contexts[*i].binding)
                                    .unwrap_or_else(|| program.fallback_binding())
                            })
                        })
                        .ok_or(SourceLayerError::InvalidBinding)?;
                    let position = match input.positioning {
                        RawPositioningReference::InFlow => SourcePositioningReference::InFlow,
                        RawPositioningReference::Unresolved => {
                            SourcePositioningReference::Unresolved
                        }
                        RawPositioningReference::Viewport(n) => {
                            SourcePositioningReference::Viewport {
                                root: binding(n).ok_or(SourceLayerError::InvalidBinding)?,
                            }
                        }
                        RawPositioningReference::ContainingBox(n) => {
                            SourcePositioningReference::ContainingBox {
                                container: binding(n).ok_or(SourceLayerError::InvalidBinding)?,
                            }
                        }
                    };
                    let parent = context_stack.last().map(|i: &usize| contexts[*i].identity);
                    let index = contexts.len();
                    contexts.push(SourcePaintContextProjection {
                        identity: SourcePaintContextIdentity {
                            source: b.id,
                            layout_role: input.layout_role,
                        },
                        binding: b,
                        parent,
                        owns_element_box: input.owner.owns_element_box,
                        border_box: input.border_box,
                        rounded_paint_offset: input.rounded_paint_offset,
                        paint_offset_residual: input.paint_offset_residual,
                        positioning: position,
                        scroll_owner: input.scroll_owner.map(binding).transpose_option()?,
                        is_viewport: input.is_viewport,
                        will_change_transform: input.will_change_transform,
                        owns_transform_containing_block: input.owns_transform_containing_block,
                        paint_isolation: input.paint_isolation,
                        first_operation: event.operation,
                        end_operation: 0,
                        transform: None,
                        clip: None,
                        effect: None,
                        scroll_translation: None,
                    });
                    depths.push(event.save_depth);
                    context_stack.push(index);
                }
                PaintSourceEventKind::ExitContext => {
                    let i = context_stack
                        .pop()
                        .ok_or(SourceLayerError::InvalidHierarchy)?;
                    if event.save_depth != depths[i] {
                        return Err(SourceLayerError::InvalidHierarchy);
                    }
                    contexts[i].end_operation = event.operation;
                }
                PaintSourceEventKind::EnterItem { owner, role } => {
                    let i = items.len();
                    let item_binding = binding(owner.node_id)
                        .or_else(|| {
                            (!owner.owns_element_box).then(|| {
                                context_stack
                                    .last()
                                    .map(|i: &usize| contexts[*i].binding)
                                    .unwrap_or_else(|| program.fallback_binding())
                            })
                        })
                        .ok_or(SourceLayerError::InvalidBinding)?;
                    items.push(SourceDisplayItemProjection {
                        binding: item_binding,
                        context: context_stack.last().map(|i| contexts[*i].identity),
                        role: *role,
                        first_operation: event.operation,
                        end_operation: 0,
                        record_operation_count: 0,
                        draw_count: 0,
                    });
                    item_stack.push(i);
                }
                PaintSourceEventKind::ExitItem => {
                    let i = item_stack.pop().ok_or(SourceLayerError::InvalidHierarchy)?;
                    items[i].end_operation = event.operation;
                    for op in &program.operations()[items[i].first_operation..event.operation] {
                        items[i].record_operation_count += usize::from(!annotation(op));
                        items[i].draw_count += usize::from(matches!(op, ProgramOperation::Draw(_)));
                    }
                }
                PaintSourceEventKind::CanvasQuery => queries.push(event.operation),
                PaintSourceEventKind::ScrollTranslation { node } => scroll_events.push((
                    event.operation,
                    binding(*node).ok_or(SourceLayerError::InvalidBinding)?,
                )),
                PaintSourceEventKind::NestedViewport { .. } => {
                    if let Some(child) = child.as_ref() {
                        if !matches!(program.operations().get(event.operation),Some(ProgramOperation::Draw(i)) if matches!(program.owned_draw(*i),OwnedDraw::Picture(..)))
                        {
                            return Err(SourceLayerError::InvalidHierarchy);
                        }
                        nested.push((event.operation, child.clone()));
                    }
                }
            }
        }
        if !context_stack.is_empty() || !item_stack.is_empty() {
            return Err(SourceLayerError::InvalidHierarchy);
        }
        let mut state = State::default();
        let mut stack = Vec::new();
        let mut states = Vec::new();
        let mut properties = Vec::new();
        for (i, op) in program.operations().iter().enumerate() {
            states.push(state.clone());
            let current = contexts
                .iter()
                .enumerate()
                .filter(|(_, c)| c.first_operation <= i && i < c.end_operation)
                .max_by_key(|(_, c)| c.first_operation)
                .map(|(j, _)| j);
            let kind = match op {
                ProgramOperation::Translate(_) if scroll_events.iter().any(|(op, _)| *op == i) => {
                    Some(SourcePaintPropertyKind::ScrollTranslation)
                }
                ProgramOperation::Translate(_)
                | ProgramOperation::Scale(_)
                | ProgramOperation::Rotate(..)
                | ProgramOperation::Concat(_)
                | ProgramOperation::Concat44(_)
                | ProgramOperation::SetMatrix(_) => Some(SourcePaintPropertyKind::Transform),
                ProgramOperation::ClipRect(..)
                | ProgramOperation::ClipRRect(..)
                | ProgramOperation::ClipPath(..) => Some(SourcePaintPropertyKind::Clip),
                ProgramOperation::SaveLayerAlpha(..) | ProgramOperation::SaveLayer { .. } => {
                    Some(SourcePaintPropertyKind::Effect)
                }
                _ => None,
            };
            match op {
                ProgramOperation::Save
                | ProgramOperation::SaveLayerAlpha(..)
                | ProgramOperation::SaveLayer { .. } => stack.push(state.clone()),
                ProgramOperation::Restore => {
                    state = stack.pop().ok_or(SourceLayerError::InvalidHierarchy)?
                }
                ProgramOperation::RestoreTo(count) => {
                    while stack.len() > *count {
                        state = stack.pop().expect("checked depth");
                    }
                }
                ProgramOperation::Translate(v) => {
                    state.matrix.pre_translate(*v);
                }
                ProgramOperation::Scale(v) => {
                    state.matrix.pre_scale(*v, None);
                }
                ProgramOperation::Rotate(angle, pivot) => {
                    state.matrix.pre_rotate(*angle, *pivot);
                }
                ProgramOperation::Concat(m) => {
                    state.matrix.pre_concat(m);
                }
                ProgramOperation::Concat44(m) => {
                    state.matrix.pre_concat(&affine(m).unwrap_or_else(|| {
                        Matrix::new_all(f32::NAN, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
                    }));
                }
                ProgramOperation::SetMatrix(m) => {
                    state.matrix = affine(m).unwrap_or_else(|| {
                        Matrix::new_all(f32::NAN, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0)
                    });
                }
                _ => {}
            }
            if let Some(kind) = kind {
                let source = scroll_events
                    .iter()
                    .find(|(op, _)| *op == i)
                    .map(|(_, b)| b.id)
                    .or_else(|| current.map(|j| contexts[j].binding.id))
                    .unwrap_or_else(|| program.fallback_binding().id);
                let occurrence = properties
                    .iter()
                    .filter(|p: &&SourcePaintPropertyProjection| {
                        p.identity.source == source && p.identity.kind == kind
                    })
                    .count() as u32;
                let id = SourcePaintPropertyIdentity {
                    source,
                    kind,
                    occurrence,
                };
                let parent = match kind {
                    SourcePaintPropertyKind::Transform => state.transform,
                    SourcePaintPropertyKind::Clip => state.clip,
                    SourcePaintPropertyKind::Effect => state.effect,
                    SourcePaintPropertyKind::ScrollTranslation => state.scroll,
                };
                let clip_bounds = match op {
                    ProgramOperation::ClipRect(r, ..) => Some(*r),
                    ProgramOperation::ClipRRect(r, ..) => Some(*r.rect()),
                    ProgramOperation::ClipPath(p, ..) => Some(p.with_path(|p| *p.bounds())),
                    _ => None,
                };
                let alpha = if let ProgramOperation::SaveLayerAlpha(_, alpha) = op {
                    Some(*alpha)
                } else {
                    None
                };
                properties.push(SourcePaintPropertyProjection {
                    identity: id,
                    parent,
                    context: current.map(|j| contexts[j].identity),
                    operation: i,
                    end_operation: program.operation_count(),
                    matrix: matrix_array(&state.matrix),
                    clip_bounds,
                    alpha,
                });
                match kind {
                    SourcePaintPropertyKind::Transform => state.transform = Some(id),
                    SourcePaintPropertyKind::Clip => {
                        state.clip = Some(id);
                        state.clip_ops.push(i);
                    }
                    SourcePaintPropertyKind::Effect => state.effect = Some(id),
                    SourcePaintPropertyKind::ScrollTranslation => state.scroll = Some(id),
                }
            }
        }
        states.push(state);
        if !stack.is_empty() {
            return Err(SourceLayerError::InvalidHierarchy);
        }
        for j in 0..properties.len() {
            let id = properties[j].identity;
            let start = properties[j].operation + 1;
            let live = |s: &State| {
                let mut current = match id.kind {
                    SourcePaintPropertyKind::Transform => s.transform,
                    SourcePaintPropertyKind::Clip => s.clip,
                    SourcePaintPropertyKind::Effect => s.effect,
                    SourcePaintPropertyKind::ScrollTranslation => s.scroll,
                };
                while let Some(key) = current {
                    if key == id {
                        return true;
                    }
                    current = properties
                        .iter()
                        .find(|p| p.identity == key)
                        .and_then(|p| p.parent);
                }
                false
            };
            let end = (start..states.len())
                .find(|i| !live(&states[*i]))
                .unwrap_or(program.operation_count());
            properties[j].end_operation = end;
        }
        for c in &mut contexts {
            let s = &states[c.first_operation];
            c.transform = s.transform;
            c.clip = s.clip;
            c.effect = s.effect;
            c.scroll_translation = s.scroll;
        }
        let mut sources = Vec::new();
        let mut candidates = Vec::new();
        let mut ordering = Vec::new();
        for (j, c) in contexts.iter().enumerate() {
            let admission = if matches!(c.positioning, SourcePositioningReference::Viewport { .. })
            {
                Some(SourceLayerAdmission::ViewportContainingBlock)
            } else if c.will_change_transform {
                Some(SourceLayerAdmission::TransformHint)
            } else if c.paint_isolation {
                Some(SourceLayerAdmission::PaintIsolation)
            } else {
                None
            };
            let mut candidate = SourceLayerCandidateProjection {
                context: c.identity,
                first_operation: c.first_operation,
                end_operation: c.end_operation,
                admission,
                unavailable: None,
                source_index: None,
            };
            let unavailable = if !c.owns_element_box {
                Some(SourceLayerUnavailable::BorrowedLayoutBox)
            } else if matches!(c.positioning, SourcePositioningReference::Unresolved) {
                Some(SourceLayerUnavailable::UnresolvedBinding)
            } else if admission.is_none() {
                Some(SourceLayerUnavailable::OrdinaryPaint)
            } else if program.capture_status() != &PaintCaptureStatus::Complete {
                Some(SourceLayerUnavailable::IncompleteCapture)
            } else if queries
                .iter()
                .any(|q| c.first_operation <= *q && *q < c.end_operation)
            {
                Some(SourceLayerUnavailable::CanvasQuery)
            } else if sources.iter().any(|s: &RetainedSourcePaintLayer| {
                s.first_operation <= c.first_operation && c.end_operation <= s.end_operation
            }) {
                Some(SourceLayerUnavailable::NestedLayer)
            } else {
                None
            };
            if let Some(reason) = unavailable {
                candidate.unavailable = Some(reason);
            } else {
                match extract(&program, c, &items, depths[j], &states, &properties) {
                    Ok(source) => {
                        if source.strategy != SourceLayerStrategy::SolidColor
                            && program.operations()[c.first_operation..c.end_operation]
                                .iter()
                                .any(|op| {
                                    matches!(
                                        op,
                                        ProgramOperation::Concat(..)
                                            | ProgramOperation::Concat44(..)
                                            | ProgramOperation::Rotate(..)
                                            | ProgramOperation::Scale(..)
                                            | ProgramOperation::Translate(..)
                                    )
                                })
                        {
                            ordering.push(SourceLayerOrderProjection {
                                home: c.identity,
                                guest: c.identity,
                                decision: SourceLayerOrderDecision::UpcastRevokesSolid,
                            });
                        }
                        let mut merged = false;
                        if let Some(home) = sources.last() {
                            let same = home.owner_translation.is_none()
                                && source.owner_translation.is_none()
                                && home.context == c.identity
                                && contexts
                                    .iter()
                                    .enumerate()
                                    .find(|(_, p)| p.first_operation == home.first_operation)
                                    .is_some_and(|(k, p)| {
                                        depths[k] == depths[j]
                                            && p.transform == c.transform
                                            && p.clip == c.clip
                                            && p.effect == c.effect
                                            && p.scroll_translation == c.scroll_translation
                                            && p.positioning == c.positioning
                                    });
                            let no_ops = program.operations()
                                [home.end_operation..c.first_operation]
                                .iter()
                                .all(annotation);
                            let overlaps =
                                !intersection(home.entry_bounds(), source.entry_bounds())
                                    .is_empty();
                            let decision = if same && no_ops {
                                SourceLayerOrderDecision::ContiguousSameSourceMerge
                            } else if overlaps {
                                SourceLayerOrderDecision::OverlapBarrier
                            } else if !same {
                                SourceLayerOrderDecision::DirectPropertyBoundary
                            } else {
                                SourceLayerOrderDecision::InterveningOperationBarrier
                            };
                            ordering.push(SourceLayerOrderProjection {
                                home: home.context,
                                guest: c.identity,
                                decision,
                            });
                            if same && no_ops {
                                let mut combined = c.clone();
                                combined.first_operation = home.first_operation;
                                if let Ok(mut combined_source) = extract(
                                    &program,
                                    &combined,
                                    &items,
                                    depths[j],
                                    &states,
                                    &properties,
                                ) {
                                    // A real multi-item merged record cannot remain a single solid item.
                                    combined_source.solid_color = None;
                                    if combined_source.strategy == SourceLayerStrategy::SolidColor {
                                        combined_source.strategy =
                                            SourceLayerStrategy::OpaquePicture;
                                    }
                                    let index = sources.len() - 1;
                                    sources[index] = combined_source;
                                    candidate.source_index = Some(index);
                                    merged = true;
                                }
                            }
                        }
                        if !merged {
                            candidate.source_index = Some(sources.len());
                            sources.push(source);
                        }
                    }
                    Err(reason) => candidate.unavailable = Some(reason),
                }
            }
            candidates.push(candidate);
        }
        let nested_matrices = nested
            .iter()
            .map(|(op, _)| states[*op].matrix)
            .collect::<Vec<_>>();
        Ok(Self {
            program,
            contexts: contexts.into(),
            items: items.into(),
            properties: properties.into(),
            candidates: candidates.into(),
            sources: sources.into(),
            nested: nested.into(),
            nested_matrices: nested_matrices.into(),
            ordering: ordering.into(),
        })
    }
    pub fn contexts(&self) -> &[SourcePaintContextProjection] {
        &self.contexts
    }
    pub fn display_items(&self) -> &[SourceDisplayItemProjection] {
        &self.items
    }
    pub fn properties(&self) -> &[SourcePaintPropertyProjection] {
        &self.properties
    }
    pub fn candidates(&self) -> &[SourceLayerCandidateProjection] {
        &self.candidates
    }
    pub fn ordering(&self) -> &[SourceLayerOrderProjection] {
        &self.ordering
    }
    pub fn sources(&self) -> &[RetainedSourcePaintLayer] {
        &self.sources
    }
    pub fn qualification(&self) -> SourceLayerQualification {
        SourceLayerQualification::UnqualifiedStaticDiagnostic
    }
    pub fn dynamic_tracks_are_admitted(&self) -> bool {
        false
    }
    pub fn capture_status(&self) -> &PaintCaptureStatus {
        self.program.capture_status()
    }
    /// Validate every retained source allocation before mutating the caller.
    pub fn replay_diagnostic(&self, canvas: &Canvas) -> Result<(), SourceLayerError> {
        if self.program.capture_status() != &PaintCaptureStatus::Complete {
            return Err(SourceLayerError::IncompleteCapture(
                self.program.capture_status().clone(),
            ));
        }
        let prepared = self.prepare(canvas.local_to_device_as_3x3())?;
        self.replay_prepared(canvas, &prepared)
    }
    fn prepare(&self, initial: Matrix) -> Result<PreparedProgram, SourceLayerError> {
        if !initial.is_scale_translate()
            || !initial.is_finite()
            || initial.scale_x() <= 0.0
            || initial.scale_y() <= 0.0
        {
            return Err(SourceLayerError::InvalidBounds);
        }
        let mut images = Vec::new();
        for source in self.sources.iter() {
            images.push(if source.strategy == SourceLayerStrategy::SolidColor {
                None
            } else {
                Some(raster_source(
                    source,
                    Matrix::concat(&initial, &source.entry_matrix),
                )?)
            });
        }
        let nested = self
            .nested
            .iter()
            .zip(self.nested_matrices.iter())
            .map(|((_, p), m)| p.prepare(Matrix::concat(&initial, m)))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(PreparedProgram { images, nested })
    }
    fn replay_prepared(
        &self,
        canvas: &Canvas,
        prepared: &PreparedProgram,
    ) -> Result<(), SourceLayerError> {
        replay_operations_with(
            self.program.operations(),
            canvas,
            |i| self.program.owned_draw(i),
            |i, tracked, _, _| {
                if let Some((index, source)) = self
                    .sources
                    .iter()
                    .enumerate()
                    .find(|(_, s)| s.first_operation == i)
                {
                    tracked.legacy_helper_boundary();
                    tracked.save();
                    if let Some((_, matrix)) = source.owner_translation {
                        tracked.concat(&matrix);
                    }
                    if let Some(color) = source.solid_color {
                        let mut paint = Paint::default();
                        paint.set_color4f(color, None);
                        paint.set_blend_mode(source.solid_blend);
                        // SoftwareRenderer::IsScaleAndIntegerTranslate accepts any
                        // scale, but integer translation. Keep external clips/alpha.
                        let device = Matrix::concat(
                            &tracked.raw_tracking_target().local_to_device_as_3x3(),
                            &Matrix::translate((source.bounds.left, source.bounds.top)),
                        );
                        paint.set_anti_alias(
                            !device.is_scale_translate()
                                || device.translate_x().fract() != 0.0
                                || device.translate_y().fract() != 0.0,
                        );
                        tracked.draw_rect(source.bounds, &paint);
                    } else {
                        let raster = prepared.images[index]
                            .as_ref()
                            .ok_or(SourceLayerError::AllocationFailure)?;
                        let device = Matrix::concat(
                            &tracked.raw_tracking_target().local_to_device_as_3x3(),
                            &Matrix::translate((source.bounds.left, source.bounds.top)),
                        );
                        let mut paint = Paint::default();
                        paint.set_anti_alias(
                            !device.is_scale_translate()
                                || device.translate_x().fract() != 0.0
                                || device.translate_y().fract() != 0.0,
                        );
                        tracked.draw_image_rect_with_sampling_options(
                            &raster.image,
                            Some((&raster.source_rect, SrcRectConstraint::Strict)),
                            source.bounds,
                            SamplingOptions::default(),
                            &paint,
                        );
                    }
                    tracked.restore();
                    return Ok(Some(source.end_operation));
                }
                if let Some((n, (_, child))) =
                    self.nested.iter().enumerate().find(|(_, (op, _))| *op == i)
                {
                    child.replay_prepared(tracked.raw_tracking_target(), &prepared.nested[n])?;
                    return Ok(Some(i + 1));
                }
                Ok(None)
            },
        )
    }
}

// Keep optional ownership resolution distinct from an absent scroll owner.
trait TransposeBinding {
    fn transpose_option(self) -> Result<Option<PaintLayerBinding>, SourceLayerError>;
}
impl TransposeBinding for Option<Option<PaintLayerBinding>> {
    fn transpose_option(self) -> Result<Option<PaintLayerBinding>, SourceLayerError> {
        match self {
            None => Ok(None),
            Some(Some(b)) => Ok(Some(b)),
            Some(None) => Err(SourceLayerError::InvalidBinding),
        }
    }
}
struct RasterSourceImage {
    image: skia_safe::Image,
    source_rect: Rect,
}
struct PreparedProgram {
    images: Vec<Option<RasterSourceImage>>,
    nested: Vec<PreparedProgram>,
}

/// Retain an actual owner property separately from its one-operation drawing
/// record. This is not a transform upcast: no draw geometry or paint flags are
/// rewritten, and a mixed state interval keeps the ordinary picture route.
fn extract_owner_translation(
    program: &Arc<LayeredPaintProgram>,
    context: &SourcePaintContextProjection,
    items: &[SourceDisplayItemProjection],
    states: &[State],
    properties: &[SourcePaintPropertyProjection],
) -> Option<RetainedSourcePaintLayer> {
    let active = items
        .iter()
        .filter(|item| {
            context.first_operation <= item.first_operation
                && item.end_operation <= context.end_operation
                && item.draw_count > 0
        })
        .collect::<Vec<_>>();
    if active.len() != 1 {
        return None;
    }
    let item = active[0];
    if item.binding != context.binding
        || item.context != Some(context.identity)
        || item.role != PaintDisplayItemRole::BoxDecorationBackground
        || item.record_operation_count != 1
        || item.draw_count != 1
    {
        return None;
    }
    let entry = &states[context.first_operation];
    let mut depth = 0usize;
    let mut draw = None;
    let mut translation = None;
    for (operation, op) in program
        .operations()
        .iter()
        .enumerate()
        .take(context.end_operation)
        .skip(context.first_operation)
    {
        match op {
            ProgramOperation::Save if draw.is_none() => depth += 1,
            ProgramOperation::Translate(offset) if draw.is_none() && depth > 0 => {
                if !offset.x.is_finite() || !offset.y.is_finite() {
                    return None;
                }
                let property = properties.iter().find(|property| {
                    property.operation == operation
                        && property.context == Some(context.identity)
                        && property.identity.source == context.binding.id
                        && property.identity.kind == SourcePaintPropertyKind::Transform
                })?;
                translation = Some(property.identity);
            }
            ProgramOperation::Draw(index)
                if draw.is_none()
                    && depth > 0
                    && item.first_operation <= operation
                    && operation < item.end_operation =>
            {
                let OwnedDraw::Rect(rect, paint) = program.owned_draw(*index) else {
                    return None;
                };
                if !solid_flags(paint) || !finite_rect(*rect) {
                    return None;
                }
                let state = &states[operation];
                if state.clip != entry.clip
                    || state.effect != entry.effect
                    || state.scroll != entry.scroll
                    || state.transform != translation
                {
                    return None;
                }
                draw = Some((*index, *rect, state.matrix));
            }
            ProgramOperation::Restore if draw.is_some() => {
                depth = depth.checked_sub(1)?;
            }
            ProgramOperation::EnterOwner(..)
            | ProgramOperation::ExitOwner
            | ProgramOperation::Property(..) => {}
            // Real clips, effects, scale, rotation, skew, later translations,
            // restore-to commands and second drawing operations stay baked.
            _ => return None,
        }
    }
    if depth != 0 {
        return None;
    }
    let property = translation?;
    let (index, rect, draw_matrix) = draw?;
    let restored = &states[context.end_operation];
    if restored.matrix != entry.matrix
        || restored.transform != entry.transform
        || restored.clip != entry.clip
        || restored.effect != entry.effect
        || restored.scroll != entry.scroll
    {
        return None;
    }
    let matrix = Matrix::concat(&entry.matrix.invert()?, &draw_matrix);
    if !matrix.is_finite()
        || !matrix.is_scale_translate()
        || matrix.scale_x() != 1.0
        || matrix.scale_y() != 1.0
    {
        return None;
    }
    let OwnedDraw::Rect(_, paint) = program.owned_draw(index) else {
        return None;
    };
    let color = paint.color4f();
    if ![color.r, color.g, color.b, color.a]
        .into_iter()
        .all(f32::is_finite)
    {
        return None;
    }
    let local = Rect::from_xywh(0.0, 0.0, rect.width(), rect.height());
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(local, false);
    canvas.translate((-rect.left, -rect.top));
    canvas.draw_rect(rect, paint);
    let picture = recorder.finish_recording_as_picture(None)?;
    Some(RetainedSourcePaintLayer {
        binding: context.binding,
        context: context.identity,
        strategy: SourceLayerStrategy::SolidColor,
        bounds: rect,
        known_opaque: if color.a == 1.0 {
            rect
        } else {
            Rect::default()
        },
        background_hint: color,
        solid_color: Some(color),
        solid_blend: paint.blend_mode(),
        picture,
        first_operation: context.first_operation,
        end_operation: context.end_operation,
        entry_matrix: entry.matrix,
        owner_translation: Some((property, matrix)),
        _program: Arc::clone(program),
    })
}

fn extract(
    program: &Arc<LayeredPaintProgram>,
    context: &SourcePaintContextProjection,
    items: &[SourceDisplayItemProjection],
    depth: usize,
    states: &[State],
    properties: &[SourcePaintPropertyProjection],
) -> Result<RetainedSourcePaintLayer, SourceLayerUnavailable> {
    if let Some(source) = extract_owner_translation(program, context, items, states, properties) {
        return Ok(source);
    }
    let entry = &states[context.first_operation];
    let inverse = entry
        .matrix
        .invert()
        .ok_or(SourceLayerUnavailable::UnsupportedTransform)?;
    let mut bounds = Rect::default();
    let mut opaque = Rect::default();
    let mut hint = Color4f::new(0.0, 0.0, 0.0, 0.0);
    let mut operations = Vec::new();
    let mut internal_depth = 0usize;
    let mut has_internal_properties = false;
    let mut only_rect = None;
    for (i, op) in program
        .operations()
        .iter()
        .enumerate()
        .take(context.end_operation)
        .skip(context.first_operation)
    {
        match op {
            ProgramOperation::Save => internal_depth += 1,
            ProgramOperation::SaveLayerAlpha(_, alpha) => {
                if !alpha.is_finite() {
                    return Err(SourceLayerUnavailable::FilterDependency);
                }
                internal_depth += 1;
                has_internal_properties = true;
            }
            ProgramOperation::SaveLayer { paint, .. } => {
                internal_depth += 1;
                has_internal_properties = true;
                if paint.as_ref().is_some_and(|p| {
                    p.image_filter().is_some()
                        || p.mask_filter().is_some()
                        || p.shader().is_some()
                        || p.color_filter().is_some()
                }) {
                    return Err(SourceLayerUnavailable::FilterDependency);
                }
            }
            ProgramOperation::Restore => {
                internal_depth = internal_depth
                    .checked_sub(1)
                    .ok_or(SourceLayerUnavailable::UnbalancedState)?;
            }
            ProgramOperation::RestoreTo(count) => {
                if *count < depth {
                    return Err(SourceLayerUnavailable::UnbalancedState);
                }
                internal_depth = internal_depth.min(*count - depth);
                operations.push(ProgramOperation::RestoreTo(*count - depth));
                continue;
            }
            ProgramOperation::SetMatrix(_) => {
                return Err(SourceLayerUnavailable::UnsupportedTransform)
            }
            ProgramOperation::ClipRect(..)
            | ProgramOperation::ClipRRect(..)
            | ProgramOperation::ClipPath(..)
            | ProgramOperation::Translate(_)
            | ProgramOperation::Scale(_)
            | ProgramOperation::Rotate(..)
            | ProgramOperation::Concat(_)
            | ProgramOperation::Concat44(_) => has_internal_properties = true,
            ProgramOperation::Draw(index) => {
                let draw = program.owned_draw(*index);
                if matches!(draw, OwnedDraw::Picture(..)) {
                    return Err(SourceLayerUnavailable::OpaquePictureDependency);
                }
                if draw.has_filter_dependency() {
                    return Err(SourceLayerUnavailable::FilterDependency);
                }
                if draw.shader_dependency() == PaintShaderDependency::Unclassified {
                    return Err(SourceLayerUnavailable::ShaderGraph);
                }
                let rect = bound_draw(draw).ok_or(SourceLayerUnavailable::UnsupportedBounds)?;
                let relative = Matrix::concat(&inverse, &states[i].matrix);
                if !relative.is_finite() {
                    return Err(SourceLayerUnavailable::UnsupportedTransform);
                }
                let visual = relative.map_rect(rect).0;
                if !finite_rect(visual) {
                    return Err(SourceLayerUnavailable::UnsupportedBounds);
                }
                bounds = join(bounds, visual);
                // Destructive/non-source-over blends can punch a hole in an
                // earlier opaque background. Do not retain that coverage hint.
                let paint = match draw {
                    OwnedDraw::Rect(_, p)
                    | OwnedDraw::Oval(_, p)
                    | OwnedDraw::RRect(_, p)
                    | OwnedDraw::RoundRect(_, _, _, p)
                    | OwnedDraw::DRRect(_, _, p)
                    | OwnedDraw::Circle(_, _, p)
                    | OwnedDraw::Line(_, _, p)
                    | OwnedDraw::Path(_, p)
                    | OwnedDraw::Image(_, _, _, _, p) => Some(p),
                    OwnedDraw::ImageNine(_, _, _, _, p) => p.as_ref(),
                    _ => None,
                };
                if paint.is_some_and(|p| {
                    p.blend_mode() != BlendMode::SrcOver
                        && !(p.blend_mode() == BlendMode::Src
                            && p.color4f().a == 1.0
                            && solid_flags(p))
                }) {
                    return Err(SourceLayerUnavailable::BackdropBlendDependency);
                }
                if let OwnedDraw::Rect(r, p) = draw {
                    if solid_flags(p) && relative.is_scale_translate() {
                        let mut coverage = relative.map_rect(r).0;
                        for clip in &states[i].clip_ops {
                            if *clip < context.first_operation {
                                continue;
                            }
                            match &program.operations()[*clip] {
                                ProgramOperation::ClipRect(r, ClipOp::Intersect, _) => {
                                    let m = Matrix::concat(&inverse, &states[*clip].matrix);
                                    if m.is_scale_translate() {
                                        coverage = intersection(coverage, m.map_rect(r).0);
                                    } else {
                                        coverage = Rect::default();
                                    }
                                }
                                _ => coverage = Rect::default(),
                            }
                        }
                        if p.color4f().a == 1.0 && states[i].effect == entry.effect {
                            opaque = covered_union(opaque, coverage);
                        }
                        if hint.a == 0.0 {
                            hint = p.color4f();
                        }
                        if only_rect.is_none() {
                            only_rect = Some((*r, p.color4f(), p.blend_mode()));
                        }
                    }
                }
            }
            _ => {}
        }
        operations.push(op.clone());
    }
    if internal_depth != 0 {
        return Err(SourceLayerUnavailable::UnbalancedState);
    }
    if !finite_rect(bounds) {
        return Err(SourceLayerUnavailable::UnsupportedBounds);
    }
    // A background hint is not opaque coverage. Require a single actual record
    // operation for solid; multiple items, clips or upcast transforms revoke it.
    let active: Vec<_> = items
        .iter()
        .filter(|item| {
            context.first_operation <= item.first_operation
                && item.end_operation <= context.end_operation
                && item.draw_count > 0
        })
        .collect();
    let solid = active.len() == 1
        && active[0].role == PaintDisplayItemRole::BoxDecorationBackground
        && active[0].record_operation_count == 1
        && !has_internal_properties
        && operations
            .iter()
            .filter(|op| matches!(op, ProgramOperation::Draw(_)))
            .count()
            == 1;
    let color = only_rect.and_then(|(r, color, _)| (solid && r.contains(bounds)).then_some(color));
    let solid_blend = only_rect.map_or(BlendMode::SrcOver, |(_, _, mode)| mode);
    let strategy = if color.is_some() {
        SourceLayerStrategy::SolidColor
    } else if hint.a == 1.0 && opaque.contains(bounds) {
        SourceLayerStrategy::OpaquePicture
    } else {
        SourceLayerStrategy::TransparentPicture
    };
    let local = Rect::from_xywh(0.0, 0.0, bounds.width(), bounds.height());
    let mut recorder = PictureRecorder::new();
    let canvas = recorder.begin_recording(local, false);
    canvas.translate((-bounds.left, -bounds.top));
    replay_operations(&operations, canvas, |i| program.owned_draw(i));
    let picture = recorder
        .finish_recording_as_picture(None)
        .ok_or(SourceLayerUnavailable::UnsupportedBounds)?;
    Ok(RetainedSourcePaintLayer {
        binding: context.binding,
        context: context.identity,
        strategy,
        bounds,
        known_opaque: opaque,
        background_hint: hint,
        solid_color: color,
        solid_blend,
        picture,
        first_operation: context.first_operation,
        end_operation: context.end_operation,
        entry_matrix: entry.matrix,
        owner_translation: None,
        _program: Arc::clone(program),
    })
}

#[cfg(test)]
mod source_tests {
    use super::*;
    use crate::paint_program::{PaintOwnerScope, PaintProgramScope};
    use crate::paint_record::PaintCanvas;
    use crate::{PaintLayerNodeIdentity, PaintLayerSourceId, PaintLayerSourceRole};
    use openui_dom::{Document, ElementTag};
    use openui_geometry::{LayoutUnit, PhysicalSize};
    use openui_layout::Fragment;
    use skia_safe::{AlphaType, Color, ImageInfo, RRect};
    fn captured(mode: u32) -> (Picture, SourceLayerProgram) {
        let mut doc = Document::new();
        let node = doc.create_node(ElementTag::Div);
        let fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20)),
        );
        let mut recorder = PictureRecorder::new();
        let raw = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 32.0, 32.0), false);
        let scope = PaintProgramScope::new(raw);
        let canvas: &dyn PaintCanvas = raw;
        canvas.clear(Color::WHITE);
        {
            let _owner = PaintOwnerScope::new(raw, &fragment);
            let _context = PaintSourceContextScope::new(
                raw,
                PaintSourceContextInput {
                    owner: PaintProgramOwner {
                        node_id: node,
                        owns_element_box: true,
                    },
                    layout_role: PaintLayoutScopeRole::AuthoredBox,
                    border_box: Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
                    rounded_paint_offset: (0.0, 0.0),
                    paint_offset_residual: (0.0, 0.0),
                    positioning: RawPositioningReference::InFlow,
                    scroll_owner: None,
                    is_viewport: false,
                    will_change_transform: true,
                    owns_transform_containing_block: false,
                    paint_isolation: false,
                },
            );
            if mode == 3 {
                canvas.save_layer_alpha_f(None, 0.5);
            }
            if mode == 4 {
                canvas.save();
                canvas.clip_rect(
                    Rect::from_xywh(0.0, 0.0, 10.0, 20.0),
                    ClipOp::Intersect,
                    false,
                );
            }
            if mode == 5 {
                canvas.save();
                canvas.translate((2.0, 1.0));
            }
            if [7, 8, 9].contains(&mode) {
                canvas.save();
                canvas.clip_rect(
                    Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
                    ClipOp::Intersect,
                    false,
                );
            }
            if mode == 6 {
                assert!(canvas.local_clip_bounds().is_some());
            }
            {
                let _item = PaintDisplayItemScope::new(
                    raw,
                    PaintProgramOwner {
                        node_id: node,
                        owns_element_box: true,
                    },
                    PaintDisplayItemRole::BoxDecorationBackground,
                );
                let mut p = Paint::default();
                p.set_color(Color::BLUE);
                p.set_anti_alias(true);
                if [7, 8, 9].contains(&mode) {
                    let mut path = skia_safe::PathBuilder::new();
                    path.set_fill_type(match mode {
                        8 => skia_safe::PathFillType::InverseWinding,
                        9 => skia_safe::PathFillType::InverseEvenOdd,
                        _ => skia_safe::PathFillType::Winding,
                    });
                    path.add_rect(Rect::from_xywh(4.0, 4.0, 12.0, 12.0), None, None);
                    canvas.draw_path(&path.detach(), &p);
                } else if mode == 1 {
                    canvas.draw_rrect(
                        RRect::new_rect_xy(Rect::from_xywh(0.0, 0.0, 20.0, 20.0), 3.0, 3.0),
                        &p,
                    );
                } else {
                    canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 20.0, 20.0), &p);
                }
            }
            if mode == 2 {
                let _item = PaintDisplayItemScope::new(
                    raw,
                    PaintProgramOwner {
                        node_id: node,
                        owns_element_box: true,
                    },
                    PaintDisplayItemRole::Foreground,
                );
                let mut p = Paint::default();
                p.set_color(Color::BLACK);
                canvas.draw_rect(Rect::from_xywh(2.0, 2.0, 5.0, 5.0), &p);
            }
            if [3, 4, 5, 7, 8, 9].contains(&mode) {
                canvas.restore();
            }
        }
        let program = scope.snapshot();
        drop(scope);
        let picture = recorder.finish_recording_as_picture(None).unwrap();
        let fallback = PaintLayerBinding {
            id: PaintLayerSourceId(1),
            owner: None,
            role: PaintLayerSourceRole::DocumentContent,
        };
        let program = program
            .partition_layers(fallback, |n| {
                Some(PaintLayerBinding {
                    id: PaintLayerSourceId(n.index() as u64 + 2),
                    owner: Some(PaintLayerNodeIdentity {
                        document: 1,
                        index: u32::try_from(n.index()).unwrap(),
                        generation: 1,
                    }),
                    role: PaintLayerSourceRole::AuthoredBox,
                })
            })
            .unwrap();
        (picture, program.source_layer_program().unwrap())
    }
    fn pixels(picture: &Picture, program: Option<&SourceLayerProgram>, scale: f32) -> Vec<u8> {
        let size = ((32.0 * scale) as i32, (32.0 * scale) as i32);
        let mut surface = skia_safe::surfaces::raster_n32_premul(size).unwrap();
        surface.canvas().scale((scale, scale));
        if let Some(program) = program {
            program.replay_diagnostic(surface.canvas()).unwrap();
        } else {
            surface.canvas().draw_picture(picture, None, None);
        }
        let mut pixels = vec![0; size.0 as usize * size.1 as usize * 4];
        assert!(surface.read_pixels(
            &ImageInfo::new_n32(size, AlphaType::Premul, None),
            &mut pixels,
            size.0 as usize * 4,
            (0, 0)
        ));
        pixels
    }
    // Full-frame oracle from two independent pinned Chromium 147.0.7727.50 captures.
    // This promoted rounded primitive differs from direct canonical raster at 1.75/2.0.
    // N32 bytes are explicitly BGRA on the qualification platform; swap channels only.
    fn assert_promoted_rounded_chromium_frame(pixels: &[u8], scale: f32) {
        let (side, expected): (i32, &[u8]) = match scale {
            1.0 => (
                32,
                include_bytes!(
                    "../tests/fixtures/rounded-source-layer-chromium-oracle/scale-1_0.rgba"
                ),
            ),
            1.25 => (
                40,
                include_bytes!(
                    "../tests/fixtures/rounded-source-layer-chromium-oracle/scale-1_25.rgba"
                ),
            ),
            1.5 => (
                48,
                include_bytes!(
                    "../tests/fixtures/rounded-source-layer-chromium-oracle/scale-1_5.rgba"
                ),
            ),
            1.75 => (
                56,
                include_bytes!(
                    "../tests/fixtures/rounded-source-layer-chromium-oracle/scale-1_75.rgba"
                ),
            ),
            2.0 => (
                64,
                include_bytes!(
                    "../tests/fixtures/rounded-source-layer-chromium-oracle/scale-2_0.rgba"
                ),
            ),
            _ => panic!("unmeasured rounded-source oracle scale {scale}"),
        };
        assert_eq!((32.0 * scale) as i32, side);
        assert_eq!(
            ImageInfo::new_n32((side, side), AlphaType::Premul, None).color_type(),
            skia_safe::ColorType::BGRA8888
        );
        assert_eq!(pixels.len(), side as usize * side as usize * 4);
        assert_eq!(expected.len(), pixels.len());
        for (index, (bgra, oracle)) in pixels
            .chunks_exact(4)
            .zip(expected.chunks_exact(4))
            .enumerate()
        {
            let rgba = [bgra[2], bgra[1], bgra[0], bgra[3]];
            assert_eq!(
                rgba.as_slice(),
                oracle,
                "promoted rounded at {scale} physical({}, {})",
                index % side as usize,
                index / side as usize
            );
        }
    }
    #[test]
    fn record_size_solid_opaque_and_alpha_are_independent() {
        for (mode, strategy) in [
            (0, SourceLayerStrategy::SolidColor),
            (1, SourceLayerStrategy::TransparentPicture),
            (2, SourceLayerStrategy::OpaquePicture),
            (3, SourceLayerStrategy::TransparentPicture),
            (4, SourceLayerStrategy::TransparentPicture),
            (5, SourceLayerStrategy::SolidColor),
        ] {
            let (picture, program) = captured(mode);
            assert_eq!(program.sources().len(), 1, "mode {mode}");
            assert_eq!(program.sources()[0].strategy(), strategy, "mode {mode}");
            if mode == 5 {
                let source = &program.sources()[0];
                assert_eq!(source.bounds(), Rect::from_xywh(0.0, 0.0, 20.0, 20.0));
                let (property, matrix) = source.owner_translation().unwrap();
                assert_eq!(property.source, source.binding().id);
                assert_eq!(property.kind, SourcePaintPropertyKind::Transform);
                assert_eq!(matrix, Matrix::translate((2.0, 1.0)));
                assert!(program.properties().iter().any(|retained| {
                    retained.identity == property
                        && retained.context == Some(source.context())
                        && retained.matrix == matrix_array(&matrix)
                }));
            }
            for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
                if mode == 1 {
                    assert_promoted_rounded_chromium_frame(
                        &pixels(&picture, Some(&program), scale),
                        scale,
                    );
                } else {
                    assert_eq!(
                        pixels(&picture, Some(&program), scale),
                        pixels(&picture, None, scale),
                        "mode {mode} at {scale}"
                    );
                }
            }
            if mode == 2 {
                assert_eq!(
                    program
                        .display_items()
                        .iter()
                        .filter(|i| i.draw_count > 0)
                        .count(),
                    2
                );
            }
            if mode == 3 {
                assert_eq!(
                    program
                        .properties()
                        .iter()
                        .filter_map(|p| p.alpha)
                        .collect::<Vec<_>>(),
                    vec![0.5]
                );
            }
        }
    }
    #[test]
    fn query_dependency_and_host_state_fail_closed_or_restore() {
        let (picture, program) = captured(6);
        assert!(program.sources().is_empty());
        assert!(program
            .candidates()
            .iter()
            .any(|p| p.unavailable == Some(SourceLayerUnavailable::CanvasQuery)));
        let mut surface = skia_safe::surfaces::raster_n32_premul((64, 64)).unwrap();
        surface.canvas().translate((1.0, 2.0));
        surface.canvas().clip_rect(
            Rect::from_xywh(0.0, 0.0, 25.0, 25.0),
            ClipOp::Intersect,
            false,
        );
        let matrix = surface.canvas().local_to_device_as_3x3();
        let clip = surface.canvas().device_clip_bounds();
        let count = surface.canvas().save_count();
        program.replay_diagnostic(surface.canvas()).unwrap();
        assert_eq!(surface.canvas().local_to_device_as_3x3(), matrix);
        assert_eq!(surface.canvas().device_clip_bounds(), clip);
        assert_eq!(surface.canvas().save_count(), count);
        surface.canvas().rotate(30.0, None);
        let matrix = surface.canvas().local_to_device_as_3x3();
        let count = surface.canvas().save_count();
        assert_eq!(
            program.replay_diagnostic(surface.canvas()),
            Err(SourceLayerError::InvalidBounds)
        );
        assert_eq!(surface.canvas().local_to_device_as_3x3(), matrix);
        assert_eq!(surface.canvas().save_count(), count);
        drop(picture);
    }
    #[test]
    fn source_plan_is_owned_send_sync() {
        fn check<T: Send + Sync>() {}
        check::<SourceLayerProgram>();
        check::<crate::SourceLayerPaintCapture>();
    }

    #[test]
    fn inverse_path_contour_bounds_do_not_admit_a_source_raster() {
        for mode in [7, 8, 9] {
            let (picture, program) = captured(mode);
            if mode == 7 {
                assert_eq!(program.sources().len(), 1);
                assert_eq!(
                    program.sources()[0].strategy(),
                    SourceLayerStrategy::TransparentPicture
                );
                assert_eq!(
                    program.sources()[0].bounds(),
                    Rect::from_xywh(4.0, 4.0, 12.0, 12.0)
                );
            } else {
                assert!(program.sources().is_empty());
                assert!(program.candidates().iter().any(|candidate| {
                    candidate.unavailable == Some(SourceLayerUnavailable::UnsupportedBounds)
                }));
            }
            for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
                let canonical = pixels(&picture, None, scale);
                assert_eq!(
                    pixels(&picture, Some(&program), scale),
                    canonical,
                    "mode {mode} at {scale}"
                );
                let width = (32.0 * scale) as usize;
                let at = |x: f32, y: f32| {
                    let index = ((y * scale) as usize * width + (x * scale) as usize) * 4;
                    &canonical[index..index + 4]
                };
                if mode == 7 {
                    assert_eq!(at(2.0, 2.0), &[255; 4]);
                    assert_ne!(at(8.0, 8.0), &[255; 4]);
                } else {
                    assert_ne!(at(2.0, 2.0), &[255; 4]);
                    assert_eq!(at(8.0, 8.0), &[255; 4]);
                }
            }
        }
    }

    fn test_binding(node: NodeId) -> PaintLayerBinding {
        PaintLayerBinding {
            id: PaintLayerSourceId(node.index() as u64 + 2),
            owner: Some(PaintLayerNodeIdentity {
                document: 1,
                index: u32::try_from(node.index()).unwrap(),
                generation: 1,
            }),
            role: PaintLayerSourceRole::AuthoredBox,
        }
    }

    fn test_context(
        owner: PaintProgramOwner,
        role: PaintLayoutScopeRole,
    ) -> PaintSourceContextInput {
        PaintSourceContextInput {
            owner,
            layout_role: role,
            border_box: Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
            rounded_paint_offset: (0.0, 0.0),
            paint_offset_residual: (0.0, 0.0),
            positioning: RawPositioningReference::InFlow,
            scroll_owner: None,
            is_viewport: false,
            will_change_transform: false,
            owns_transform_containing_block: false,
            paint_isolation: false,
        }
    }

    #[test]
    fn borrowed_text_and_anonymous_items_inherit_the_actual_context_or_document() {
        for with_parent in [false, true] {
            let mut doc = Document::new();
            let parent = doc.create_node(ElementTag::Div);
            let text = doc.create_node(ElementTag::Text);
            let size = PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20));
            let parent_box = Fragment::new_box(parent, size);
            let fallback = PaintLayerBinding {
                id: PaintLayerSourceId(1),
                owner: None,
                role: PaintLayerSourceRole::DocumentContent,
            };
            let expected = if with_parent {
                test_binding(parent)
            } else {
                fallback
            };
            let mut recorder = PictureRecorder::new();
            let raw = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 32.0, 32.0), false);
            let scope = PaintProgramScope::new(raw);
            let canvas: &dyn PaintCanvas = raw;
            canvas.clear(Color::WHITE);
            {
                let _parent_owner = with_parent.then(|| PaintOwnerScope::new(raw, &parent_box));
                let _parent_context = with_parent.then(|| {
                    PaintSourceContextScope::new(
                        raw,
                        test_context(
                            PaintProgramOwner {
                                node_id: parent,
                                owns_element_box: true,
                            },
                            PaintLayoutScopeRole::AuthoredBox,
                        ),
                    )
                });
                for (index, node) in [text, NodeId::NONE].into_iter().enumerate() {
                    let mut fragment = Fragment::new_box(node, size);
                    fragment.kind = openui_layout::FragmentKind::Text;
                    fragment.box_ownership = openui_layout::FragmentBoxOwnership::LayoutHelper;
                    let owner = PaintProgramOwner {
                        node_id: node,
                        owns_element_box: false,
                    };
                    let _owner = PaintOwnerScope::new(raw, &fragment);
                    let _context = PaintSourceContextScope::new(
                        raw,
                        test_context(owner, PaintLayoutScopeRole::TextRun),
                    );
                    let _item =
                        PaintDisplayItemScope::new(raw, owner, PaintDisplayItemRole::Foreground);
                    let mut paint = Paint::default();
                    paint.set_color(Color::BLACK);
                    canvas.draw_rect(
                        Rect::from_xywh(2.0 + index as f32 * 6.0, 3.0, 4.0, 5.0),
                        &paint,
                    );
                }
            }
            let recorded = scope.snapshot();
            drop(scope);
            let picture = recorder.finish_recording_as_picture(None).unwrap();
            let layered = recorded
                .partition_layers(fallback, |node| {
                    (node == parent).then(|| test_binding(parent))
                })
                .unwrap();
            assert_eq!(layered.fallback_binding(), fallback);
            assert!(layered
                .draw_references()
                .iter()
                .skip(1)
                .all(|draw| draw.source == expected.id));
            let program = layered.source_layer_program().unwrap();
            let text_contexts = program
                .contexts()
                .iter()
                .filter(|context| context.identity.layout_role == PaintLayoutScopeRole::TextRun)
                .collect::<Vec<_>>();
            assert_eq!(text_contexts.len(), 2);
            assert!(text_contexts.iter().all(|context| {
                context.binding == expected
                    && !context.owns_element_box
                    && context.parent.map(|parent| parent.source)
                        == with_parent.then_some(expected.id)
            }));
            assert_eq!(program.display_items().len(), 2);
            assert!(program
                .display_items()
                .iter()
                .all(|item| item.binding == expected && item.draw_count == 1));
            assert!(program.properties().is_empty());
            assert!(program.sources().is_empty());
            assert!(program
                .candidates()
                .iter()
                .filter(|candidate| {
                    candidate.context.layout_role == PaintLayoutScopeRole::TextRun
                })
                .all(|candidate| candidate.unavailable
                    == Some(SourceLayerUnavailable::BorrowedLayoutBox)));
            drop(doc);
            for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
                assert_eq!(
                    pixels(&picture, Some(&program), scale),
                    pixels(&picture, None, scale)
                );
            }
        }
    }

    #[test]
    fn fallback_is_retained_without_draws_and_before_the_first_authored_draw() {
        for mode in 0..4 {
            let mut doc = Document::new();
            let node = doc.create_node(ElementTag::Div);
            let size = PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20));
            let fragment = Fragment::new_box(node, size);
            let fallback = PaintLayerBinding {
                id: PaintLayerSourceId(1),
                owner: None,
                role: PaintLayerSourceRole::DocumentContent,
            };
            let mut recorder = PictureRecorder::new();
            let raw = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 32.0, 32.0), false);
            let scope = PaintProgramScope::new(raw);
            let canvas: &dyn PaintCanvas = raw;
            canvas.save();
            canvas.translate((1.0, 2.0));
            if mode == 1 || mode == 3 {
                let _context = PaintSourceContextScope::new(
                    raw,
                    test_context(
                        PaintProgramOwner {
                            node_id: if mode == 3 { node } else { NodeId::NONE },
                            owns_element_box: mode == 3,
                        },
                        if mode == 3 {
                            PaintLayoutScopeRole::AuthoredBox
                        } else {
                            PaintLayoutScopeRole::LayoutHelper
                        },
                    ),
                );
            }
            if mode == 2 {
                let _owner = PaintOwnerScope::new(raw, &fragment);
                let mut paint = Paint::default();
                paint.set_color(Color::BLUE);
                canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 20.0, 20.0), &paint);
            }
            canvas.restore();
            let recorded = scope.snapshot();
            drop(scope);
            let picture = recorder.finish_recording_as_picture(None).unwrap();
            let layered = recorded
                .partition_layers(fallback, |n| {
                    (mode == 2 && n == node).then(|| test_binding(node))
                })
                .unwrap();
            assert_eq!(layered.fallback_binding(), fallback);
            assert_eq!(layered.capture_status(), &PaintCaptureStatus::Complete);
            if mode == 2 {
                assert_eq!(layered.draw_references().len(), 1);
                assert_eq!(layered.draw_references()[0].source, test_binding(node).id);
                assert!(layered.sources()[0].chunks().is_empty());
            } else {
                assert!(layered.draw_references().is_empty());
            }
            let result = layered.source_layer_program();
            if mode == 3 {
                assert!(matches!(result, Err(SourceLayerError::InvalidBinding)));
                continue;
            }
            let program = result.unwrap();
            assert_eq!(program.properties().len(), 1);
            assert_eq!(program.properties()[0].identity.source, fallback.id);
            assert!(program
                .contexts()
                .iter()
                .all(|context| context.binding == fallback));
            assert!(program.display_items().is_empty());
            assert!(program.sources().is_empty());
            drop(doc);
            for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
                assert_eq!(
                    pixels(&picture, Some(&program), scale),
                    pixels(&picture, None, scale)
                );
            }
        }
    }
    fn translated_owner_capture(
        translations: &[(f32, f32)],
        neighbor: u32,
        repeat_context: bool,
    ) -> (Picture, SourceLayerProgram) {
        let mut doc = Document::new();
        let node = doc.create_node(ElementTag::Div);
        let fragment = Fragment::new_box(
            node,
            PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(20)),
        );
        let mut recorder = PictureRecorder::new();
        let raw = recorder.begin_recording(Rect::from_xywh(0.0, 0.0, 32.0, 32.0), false);
        let scope = PaintProgramScope::new(raw);
        let canvas: &dyn PaintCanvas = raw;
        canvas.clear(Color::WHITE);
        for pass in 0..if repeat_context { 2 } else { 1 } {
            let _owner = PaintOwnerScope::new(raw, &fragment);
            let _context = PaintSourceContextScope::new(
                raw,
                PaintSourceContextInput {
                    owner: PaintProgramOwner {
                        node_id: node,
                        owns_element_box: neighbor != 7,
                    },
                    layout_role: PaintLayoutScopeRole::AuthoredBox,
                    border_box: Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
                    rounded_paint_offset: (0.0, 0.0),
                    paint_offset_residual: (0.0, 0.0),
                    positioning: RawPositioningReference::InFlow,
                    scroll_owner: None,
                    is_viewport: false,
                    will_change_transform: true,
                    owns_transform_containing_block: false,
                    paint_isolation: false,
                },
            );
            canvas.save();
            for offset in translations {
                canvas.translate(*offset);
            }
            if pass == 1 {
                canvas.translate((20.0, 0.0));
            }
            match neighbor {
                1 => {
                    canvas.scale((2.0, 1.0));
                }
                2 => {
                    canvas.clip_rect(
                        Rect::from_xywh(0.0, 0.0, 10.0, 20.0),
                        ClipOp::Intersect,
                        false,
                    );
                }
                3 => {
                    canvas.save_layer_alpha_f(None, 0.5);
                }
                5 => {
                    canvas.rotate(30.0, None);
                }
                6 => {
                    canvas.concat(&Matrix::new_all(
                        1.0, 0.25, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0,
                    ));
                }
                _ => {}
            }
            {
                let _item = PaintDisplayItemScope::new(
                    raw,
                    PaintProgramOwner {
                        node_id: node,
                        owns_element_box: true,
                    },
                    PaintDisplayItemRole::BoxDecorationBackground,
                );
                let mut paint = Paint::default();
                paint.set_color(Color::BLUE);
                paint.set_anti_alias(true);
                canvas.draw_rect(Rect::from_xywh(0.0, 0.0, 20.0, 20.0), &paint);
                if neighbor == 4 {
                    paint.set_color(Color::BLACK);
                    canvas.draw_rect(Rect::from_xywh(2.0, 2.0, 5.0, 5.0), &paint);
                }
            }
            if neighbor == 3 {
                canvas.restore();
            }
            if neighbor == 8 {
                // A later transform is not an enclosing owner property.
                canvas.translate((1.0, 0.0));
            }
            canvas.restore();
        }
        let captured = scope.snapshot();
        drop(scope);
        let picture = recorder.finish_recording_as_picture(None).unwrap();
        let layered = captured
            .partition_layers(
                PaintLayerBinding {
                    id: PaintLayerSourceId(1),
                    owner: None,
                    role: PaintLayerSourceRole::DocumentContent,
                },
                |n| {
                    Some(PaintLayerBinding {
                        id: PaintLayerSourceId(n.index() as u64 + 2),
                        owner: Some(PaintLayerNodeIdentity {
                            document: 1,
                            index: u32::try_from(n.index()).unwrap(),
                            generation: 1,
                        }),
                        role: PaintLayerSourceRole::AuthoredBox,
                    })
                },
            )
            .unwrap();
        let program = layered.source_layer_program().unwrap();
        drop(doc);
        (picture, program)
    }

    fn translated_owner_spine_pixels(program: &SourceLayerProgram, scale: f32) -> Vec<u8> {
        let size = ((32.0 * scale) as i32, (32.0 * scale) as i32);
        let mut surface = skia_safe::surfaces::raster_n32_premul(size).unwrap();
        surface.canvas().scale((scale, scale));
        program.program.replay(surface.canvas()).unwrap();
        let mut bytes = vec![0; size.0 as usize * size.1 as usize * 4];
        assert!(surface.read_pixels(
            &ImageInfo::new_n32(size, AlphaType::Premul, None),
            &mut bytes,
            size.0 as usize * 4,
            (0, 0),
        ));
        bytes
    }

    #[test]
    fn retained_owner_translation_preserves_original_rect_and_exact_fractional_phase() {
        for translations in [
            vec![(2.0, 1.0)],
            vec![(-1.25, -0.75)],
            vec![(1.25, 2.25)],
            vec![(1.0, 0.5), (1.0, 0.5)],
        ] {
            let (picture, program) = translated_owner_capture(&translations, 0, false);
            assert_eq!(program.sources().len(), 1);
            let source = &program.sources()[0];
            assert_eq!(source.strategy(), SourceLayerStrategy::SolidColor);
            assert_eq!(source.bounds(), Rect::from_xywh(0.0, 0.0, 20.0, 20.0));
            assert_eq!(source.rect_known_to_be_opaque(), source.bounds());
            let (property, matrix) = source.owner_translation().unwrap();
            assert_eq!(property.source, source.binding().id);
            assert_eq!(property.kind, SourcePaintPropertyKind::Transform);
            let offset = translations
                .iter()
                .fold((0.0, 0.0), |sum, next| (sum.0 + next.0, sum.1 + next.1));
            assert_eq!(matrix, Matrix::translate(offset));
            assert_eq!(program.display_items()[0].record_operation_count, 1);
            assert_eq!(program.display_items()[0].draw_count, 1);
            for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
                assert_eq!(
                    pixels(&picture, Some(&program), scale),
                    pixels(&picture, None, scale)
                );
                assert_eq!(
                    translated_owner_spine_pixels(&program, scale),
                    pixels(&picture, None, scale)
                );
                let mut surface = skia_safe::surfaces::raster_n32_premul((
                    (32.0 * scale) as i32,
                    (32.0 * scale) as i32,
                ))
                .unwrap();
                surface.canvas().scale((scale, scale));
                surface.canvas().save();
                surface.canvas().translate((3.0, 4.0));
                surface.canvas().clip_rect(
                    Rect::from_xywh(0.0, 0.0, 20.0, 20.0),
                    ClipOp::Intersect,
                    false,
                );
                let count = surface.canvas().save_count();
                let before = surface.canvas().local_to_device_as_3x3();
                let clip = surface.canvas().device_clip_bounds();
                program.replay_diagnostic(surface.canvas()).unwrap();
                assert_eq!(surface.canvas().save_count(), count);
                assert_eq!(surface.canvas().local_to_device_as_3x3(), before);
                assert_eq!(surface.canvas().device_clip_bounds(), clip);
                surface.canvas().restore();
            }
        }
    }

    #[test]
    fn retained_owner_translation_rejects_mixed_state_and_extra_drawing_records() {
        for neighbor in 1..=8 {
            let (_, program) = translated_owner_capture(&[(2.0, 1.0)], neighbor, false);
            assert!(program
                .sources()
                .iter()
                .all(|source| source.owner_translation().is_none()));
            assert!(program
                .sources()
                .iter()
                .all(|source| source.strategy() != SourceLayerStrategy::SolidColor));
            if neighbor == 7 {
                assert!(program
                    .candidates()
                    .iter()
                    .any(|candidate| candidate.unavailable
                        == Some(SourceLayerUnavailable::BorrowedLayoutBox)));
            }
        }
        let (_, invalid) = translated_owner_capture(&[(f32::INFINITY, 0.0)], 0, false);
        assert!(invalid.sources().is_empty());
        assert!(invalid
            .candidates()
            .iter()
            .all(|candidate| candidate.source_index.is_none()));
    }

    #[test]
    fn retained_owner_translation_is_a_direct_property_boundary_between_same_source_runs() {
        let (picture, program) = translated_owner_capture(&[(2.0, 1.0)], 0, true);
        assert_eq!(program.sources().len(), 2);
        assert_eq!(
            program.sources()[0].binding(),
            program.sources()[1].binding()
        );
        assert_ne!(
            program.sources()[0].owner_translation(),
            program.sources()[1].owner_translation()
        );
        assert!(program
            .ordering()
            .iter()
            .all(|order| order.decision != SourceLayerOrderDecision::ContiguousSameSourceMerge));
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            assert_eq!(
                pixels(&picture, Some(&program), scale),
                pixels(&picture, None, scale)
            );
            assert_eq!(
                translated_owner_spine_pixels(&program, scale),
                pixels(&picture, None, scale)
            );
        }
    }
}

/// Pinned ClearForOpaqueRaster inputs, including bitmap origin. External guard
/// texels are returned separately from the frame-visible playback rectangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OpaqueRasterClearRects {
    pub outer: Rect,
    pub inner: Rect,
    pub visible: Rect,
    pub clear_required: bool,
}
pub fn opaque_raster_clear_rects(
    size: (f32, f32),
    scale: (f32, f32),
    translation: (f32, f32),
    playback: IRect,
    bitmap_origin: (i32, i32),
) -> Result<OpaqueRasterClearRects, SourceLayerError> {
    if ![
        size.0,
        size.1,
        scale.0,
        scale.1,
        translation.0,
        translation.1,
    ]
    .into_iter()
    .all(f32::is_finite)
        || size.0 <= 0.0
        || size.1 <= 0.0
        || scale.0 <= 0.0
        || scale.1 <= 0.0
        || !(0.0..1.0).contains(&translation.0)
        || !(0.0..1.0).contains(&translation.1)
    {
        return Err(SourceLayerError::InvalidBounds);
    }
    if size.0.fract() != 0.0 || size.1.fract() != 0.0 || playback.is_empty() {
        return Err(SourceLayerError::InvalidBounds);
    }
    let edge = |present: bool| if present { 1.0 } else { 0.0 };
    let left = translation.0 != 0.0;
    let top = translation.1 != 0.0;
    let right = scale.0 != 1.0;
    let bottom = scale.1 != 1.0;
    let outer = Rect::from_ltrb(0.0, 0.0, size.0 + edge(right), size.1 + edge(bottom));
    let inner = Rect::from_ltrb(
        edge(left),
        edge(top),
        size.0 - edge(right),
        size.1 - edge(bottom),
    );
    let adjusted = Rect::from_ltrb(
        playback.left as f32 - edge(left && playback.left == 0),
        playback.top as f32 - edge(top && playback.top == 0),
        playback.right as f32 + edge(right && playback.right as f32 == size.0),
        playback.bottom as f32 + edge(bottom && playback.bottom as f32 == size.1),
    );
    let mut outer = intersection(outer, adjusted);
    let mut inner = intersection(inner, adjusted);
    let clear_required = !(inner.contains(adjusted) || outer.is_empty() || inner == outer);
    let mut visible = intersection(outer, Rect::from_xywh(0.0, 0.0, size.0, size.1));
    outer.offset((-bitmap_origin.0 as f32, -bitmap_origin.1 as f32));
    inner.offset((-bitmap_origin.0 as f32, -bitmap_origin.1 as f32));
    visible.offset((-bitmap_origin.0 as f32, -bitmap_origin.1 as f32));
    Ok(OpaqueRasterClearRects {
        outer,
        inner,
        visible,
        clear_required,
    })
}
fn raster_source(
    source: &RetainedSourcePaintLayer,
    device: Matrix,
) -> Result<RasterSourceImage, SourceLayerError> {
    if !device.is_scale_translate()
        || !device.is_finite()
        || device.scale_x() <= 0.0
        || device.scale_y() <= 0.0
    {
        return Err(SourceLayerError::InvalidBounds);
    }
    let scale = (device.scale_x(), device.scale_y());
    let origin = device.map_xy(source.bounds.left, source.bounds.top);
    let phase = (origin.x - origin.x.floor(), origin.y - origin.y.floor());
    let width = (source.bounds.width() * scale.0).ceil();
    let height = (source.bounds.height() * scale.1).ceil();
    if !width.is_finite()
        || !height.is_finite()
        || width < 1.0
        || height < 1.0
        || width >= i32::MAX as f32
        || height >= i32::MAX as f32
    {
        return Err(SourceLayerError::InvalidBounds);
    }
    // Keep one external sampling texel, and expose only the original quad rect.
    let mut surface = skia_safe::surfaces::raster_n32_premul((width as i32 + 1, height as i32 + 1))
        .ok_or(SourceLayerError::AllocationFailure)?;
    surface.canvas().clear(skia_safe::Color::TRANSPARENT);
    if source.strategy == SourceLayerStrategy::OpaquePicture {
        let clear = opaque_raster_clear_rects(
            (width, height),
            scale,
            phase,
            IRect::from_wh(width as i32, height as i32),
            (0, 0),
        )?;
        surface.canvas().save();
        surface
            .canvas()
            .clip_rect(clear.outer, ClipOp::Intersect, false);
        if !clear.inner.is_empty() {
            surface
                .canvas()
                .clip_rect(clear.inner, ClipOp::Difference, false);
        }
        surface
            .canvas()
            .draw_color(source.background_hint, BlendMode::Src);
        surface.canvas().restore();
    }
    surface.canvas().translate(phase);
    surface.canvas().scale(scale);
    surface.canvas().draw_picture(&source.picture, None, None);
    let source_rect = Rect::from_xywh(
        phase.0,
        phase.1,
        source.bounds.width() * scale.0,
        source.bounds.height() * scale.1,
    );
    Ok(RasterSourceImage {
        image: surface.image_snapshot(),
        source_rect,
    })
}
