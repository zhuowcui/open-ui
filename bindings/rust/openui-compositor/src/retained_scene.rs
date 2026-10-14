//! Owned scene identities and source metadata. These plans do not change raster
//! replay or select a Chromium surface/quad route. Arbitrary element pictures
//! still require partitioned paint capture before presentation can sample them.

use openui_geometry::{BoxStrut, Length, PhysicalOffset, PhysicalRect, WritingDirectionMode};
use openui_layout::{ResolvedFragmentTransform, ScrollArea, StickyTimelineGeometry};
use openui_paint::{RecordedContentLayer, RecordedPicture};
use skia_safe::Picture;
use std::sync::Arc;

macro_rules! identity {
    ($name:ident) => {
        /// A nonreused identity allocated by the publishing document engine.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub struct $name(pub u64);
    };
}
identity!(SceneLayerId);
identity!(SceneEffectId);
identity!(SceneClipId);
identity!(SceneScrollId);

/// A copied live-node identity, without an Engine reference or DOM borrow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SceneNodeIdentity {
    pub document: u64,
    pub index: u32,
    pub generation: u32,
}

/// Actual existing recordings. Canonical overlaps the other roles and is an
/// alternative complete replay, not another independently composed layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SceneRecordingRole {
    Canonical,
    DocumentContent,
    ViewportScrollingContent,
    ViewportOverlay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SceneRecordingIdentity {
    pub id: SceneLayerId,
    pub role: SceneRecordingRole,
}

/// Keeps the complete resource/cache owner alive as well as the selected
/// immutable picture. Cloning this value remains safe after the scene and
/// originating Engine are dropped.
#[derive(Clone)]
pub struct SceneLayerRecording {
    identity: SceneRecordingIdentity,
    owner: Arc<RecordedPicture>,
}

impl SceneLayerRecording {
    pub fn id(&self) -> SceneLayerId {
        self.identity.id
    }
    pub fn role(&self) -> SceneRecordingRole {
        self.identity.role
    }
    /// Raw picture access; the role's existing tiling/clear/composition policy
    /// is still required. This accessor does not implement independent replay.
    pub fn picture(&self) -> &Picture {
        match self.role() {
            SceneRecordingRole::Canonical => self.owner.canonical_picture(),
            SceneRecordingRole::DocumentContent => self.owner.document_content_picture(),
            SceneRecordingRole::ViewportScrollingContent => {
                &self
                    .owner
                    .viewport_content_layer()
                    .expect("validated recording")
                    .picture
            }
            SceneRecordingRole::ViewportOverlay => self
                .owner
                .viewport_overlay_picture()
                .expect("validated recording"),
        }
    }
    pub fn viewport_content_layer(&self) -> Option<&RecordedContentLayer> {
        (self.role() == SceneRecordingRole::ViewportScrollingContent)
            .then(|| self.owner.viewport_content_layer())
            .flatten()
    }
    pub fn retained_font_face_count(&self) -> usize {
        self.owner.retained_font_face_count()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneLayerRole {
    AuthoredBox,
    GeneratedBox,
}

/// Physical projections share one authored identity. Fragment paths and local
/// geometry describe this immutable frame, never serve as lifetime keys.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneFragmentProjection {
    pub fragment_path: Arc<[usize]>,
    pub layout_parent: Option<SceneLayerId>,
    pub offset: PhysicalOffset,
    pub border_box: PhysicalRect,
    pub transform: Option<ResolvedFragmentTransform>,
}

/// Source layer plan. No arbitrary subtree picture is claimed by this type.
/// Effects and scroll offsets remain baked into the canonical recording.
#[derive(Debug, Clone)]
pub struct SceneLayerSnapshot {
    pub id: SceneLayerId,
    pub owner: SceneNodeIdentity,
    pub role: SceneLayerRole,
    pub effect: SceneEffectId,
    pub clip: SceneClipId,
    /// Separate clip for a delegated formatting/content scope. The authored
    /// wrapper and its legend are not inside this anonymous subtree.
    pub content_clip: Option<SceneClipId>,
    pub scroll: Option<SceneScrollId>,
    pub projections: Arc<[SceneFragmentProjection]>,
}

/// Effect source retained independently of the live computed style. These are
/// canvas-baked effects, not evidence of final compositor surface ownership.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneEffectSnapshot {
    pub id: SceneEffectId,
    pub owner: SceneNodeIdentity,
    pub source_parent: Option<SceneNodeIdentity>,
    pub opacity: f32,
    pub blur: f32,
    pub grayscale: f32,
    pub paint_containment: bool,
    pub will_change_transform: bool,
}

/// Exact retained inputs to clipping for one physical projection. The authored
/// inset lengths are source inputs, not resolved clip bounds; paint's shared
/// actual-box evaluator remains authoritative. Structural clips can belong to
/// intervening anonymous fragments, retained by SceneSnapshot::fragments().
#[derive(Debug, Clone, PartialEq)]
pub struct SceneClipProjection {
    pub fragment_path: Arc<[usize]>,
    pub offset: PhysicalOffset,
    pub client_rect: PhysicalRect,
    pub overflow_clips_x: bool,
    pub overflow_clips_y: bool,
    pub paint_containment: bool,
    pub structural_clip: bool,
    pub block_axis_clip_only: bool,
    pub inline_axis_clip_only: bool,
    pub fragmentation_writing_direction: Option<WritingDirectionMode>,
    pub border_radii: [(f32, f32); 4],
    pub border_radius_percent: [(bool, bool); 4],
    /// Specified length kinds survive independently of compatibility fields.
    pub border_radius_lengths: [Option<(Length, Length)>; 4],
    /// Unsnapped CSS paint-coordinate source bases. Paint stitched/source and
    /// direct physical-fragment hit contours intentionally have distinct inputs.
    pub paint_border_radius_reference_box: (f32, f32, f32, f32),
    pub hit_border_radius_reference_box: (f32, f32, f32, f32),
    pub paint_clip_geometry: openui_paint::OverflowClipGeometry,
    pub direct_hit_clip_geometry: openui_paint::OverflowClipGeometry,
    pub clip_path_insets: Option<[Length; 4]>,
    pub overflow_clip_box: SceneOverflowClipBox,
    pub overflow_clip_margin: f32,
    pub used_border: BoxStrut,
    pub used_padding: BoxStrut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneOverflowClipBox {
    Border,
    Padding,
    Content,
}

/// Explicit box ownership role within a live semantic element. A delegated
/// fieldset content box owns scroll/clip scope, but no authored effect/layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneBoxRole {
    Principal,
    FieldsetContent,
}

#[derive(Debug, Clone)]
pub struct SceneClipSnapshot {
    pub id: SceneClipId,
    pub owner: SceneNodeIdentity,
    pub role: SceneBoxRole,
    pub projections: Arc<[SceneClipProjection]>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SceneScrollProjection {
    pub fragment_path: Arc<[usize]>,
    pub box_offset: PhysicalOffset,
    pub area: ScrollArea,
    pub offset: (f32, f32),
    pub viewport: bool,
    pub sticky: Option<StickyTimelineGeometry>,
}

#[derive(Debug, Clone)]
pub struct SceneScrollSnapshot {
    pub id: SceneScrollId,
    pub owner: SceneNodeIdentity,
    pub role: SceneBoxRole,
    pub projections: Arc<[SceneScrollProjection]>,
}

/// Additive source-publication input. No callbacks, resolvers, DOM references,
/// user data, or mutable track clocks are carried into a retained scene.
#[doc(hidden)]
pub struct RetainedScenePlan {
    pub recordings: Vec<SceneRecordingIdentity>,
    pub layers: Vec<SceneLayerSnapshot>,
    pub effects: Vec<SceneEffectSnapshot>,
    pub clips: Vec<SceneClipSnapshot>,
    pub scrolls: Vec<SceneScrollSnapshot>,
    /// Explicit coverage gap rather than invented live identities for boxes
    /// which the publishing Engine cannot associate with a live handle.
    pub unidentified_owned_box_paths: Vec<Arc<[usize]>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetainedSceneError {
    MissingRecording(SceneRecordingRole),
    AllocationFailure,
}

impl std::fmt::Display for RetainedSceneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingRecording(role) => write!(f, "missing retained {role:?} recording"),
            Self::AllocationFailure => f.write_str("retained scene allocation failed"),
        }
    }
}
impl std::error::Error for RetainedSceneError {}

/// Immutable metadata associated with one exact canonical picture. It owns
/// actual recording resources; authored plans are explicitly unpartitioned.
#[derive(Clone)]
pub struct RetainedSceneSnapshot {
    recordings: Arc<[SceneLayerRecording]>,
    layers: Arc<[SceneLayerSnapshot]>,
    effects: Arc<[SceneEffectSnapshot]>,
    clips: Arc<[SceneClipSnapshot]>,
    scrolls: Arc<[SceneScrollSnapshot]>,
    unidentified_owned_box_paths: Arc<[Arc<[usize]>]>,
    source_layer_paint: Option<Arc<openui_paint::SourceLayerPaintCapture>>,
}

impl RetainedSceneSnapshot {
    pub(crate) fn bind(
        plan: RetainedScenePlan,
        owner: Arc<RecordedPicture>,
    ) -> Result<Self, RetainedSceneError> {
        let mut recordings = Vec::new();
        recordings
            .try_reserve(plan.recordings.len())
            .map_err(|_| RetainedSceneError::AllocationFailure)?;
        for identity in plan.recordings {
            let present = match identity.role {
                SceneRecordingRole::Canonical | SceneRecordingRole::DocumentContent => true,
                SceneRecordingRole::ViewportScrollingContent => {
                    owner.viewport_content_layer().is_some()
                }
                SceneRecordingRole::ViewportOverlay => owner.viewport_overlay_picture().is_some(),
            };
            if !present {
                return Err(RetainedSceneError::MissingRecording(identity.role));
            }
            recordings.push(SceneLayerRecording {
                identity,
                owner: Arc::clone(&owner),
            });
        }
        Ok(Self {
            recordings: recordings.into(),
            layers: plan.layers.into(),
            effects: plan.effects.into(),
            clips: plan.clips.into(),
            scrolls: plan.scrolls.into(),
            unidentified_owned_box_paths: plan.unidentified_owned_box_paths.into(),
            source_layer_paint: None,
        })
    }
    pub fn recordings(&self) -> &[SceneLayerRecording] {
        &self.recordings
    }
    pub(crate) fn attach_source_layer_diagnostic(
        &mut self,
        capture: Arc<openui_paint::SourceLayerPaintCapture>,
    ) {
        self.source_layer_paint = Some(capture);
    }
    pub fn source_layer_paint_capture(
        &self,
    ) -> Option<&Arc<openui_paint::SourceLayerPaintCapture>> {
        self.source_layer_paint.as_ref()
    }
    pub fn layers(&self) -> &[SceneLayerSnapshot] {
        &self.layers
    }
    pub fn effects(&self) -> &[SceneEffectSnapshot] {
        &self.effects
    }
    pub fn clips(&self) -> &[SceneClipSnapshot] {
        &self.clips
    }
    pub fn scrolls(&self) -> &[SceneScrollSnapshot] {
        &self.scrolls
    }
    pub fn unidentified_owned_box_paths(&self) -> &[Arc<[usize]>] {
        &self.unidentified_owned_box_paths
    }
    /// Per-element pictures have not been partitioned from canonical paint.
    pub fn authored_layers_are_partitioned(&self) -> bool {
        false
    }
}
