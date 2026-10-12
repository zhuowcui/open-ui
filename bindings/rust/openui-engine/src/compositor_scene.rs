//! Thread-affine source registry; only owned immutable values are published.
//! IDs follow live handle generations and explicit roles, never bounds, path
//! positions, computed style values, or Skia picture addresses.

use crate::{Engine, EngineError, NodeHandle, Slot};
use openui_compositor::{
    RetainedScenePlan, SceneBoxRole, SceneClipId, SceneClipProjection, SceneClipSnapshot,
    SceneEffectId, SceneEffectSnapshot, SceneFragmentProjection, SceneLayerId, SceneLayerRole,
    SceneLayerSnapshot, SceneNodeIdentity, SceneOverflowClipBox, SceneRecordingIdentity,
    SceneRecordingRole, SceneScrollId, SceneScrollProjection, SceneScrollSnapshot,
};
use openui_layout::{AnonymousLayoutBox, Fragment};
use openui_paint::RecordedPicture;
use openui_style::{ComputedStyle, Overflow};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// Global allocation prevents identities colliding across documents, including
// saved scenes surviving their originating Engine. Exhaustion never wraps.
static NEXT_SCENE_ID: AtomicU64 = AtomicU64::new(1);

fn allocate_ids(count: u64) -> Result<u64, EngineError> {
    NEXT_SCENE_ID
        .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |next| {
            next.checked_add(count)
        })
        .map_err(|_| EngineError::Render("retained scene identity exhausted".into()))
}

fn owner(handle: NodeHandle) -> SceneNodeIdentity {
    SceneNodeIdentity {
        document: handle.document_id(),
        index: handle.index(),
        generation: handle.generation(),
    }
}

#[derive(Clone, Copy)]
struct NodeSceneIds {
    layer: SceneLayerId,
    effect: SceneEffectId,
    clip: SceneClipId,
    scroll: SceneScrollId,
    content_clip: SceneClipId,
    content_scroll: SceneScrollId,
}

#[derive(Default)]
pub(crate) struct SceneIdentityRegistry {
    nodes: HashMap<NodeHandle, NodeSceneIds>,
    recordings: HashMap<SceneRecordingRole, SceneLayerId>,
}

impl SceneIdentityRegistry {
    pub(crate) fn retire(&mut self, released: &[NodeHandle]) {
        for handle in released {
            self.nodes.remove(handle);
        }
    }

    fn retain_live(&mut self, document: u64, slots: &[Slot]) {
        // Hidden or detached live nodes retain their IDs, allowing later
        // projection without identity churn. Removed generations are retired.
        self.nodes.retain(|handle, _| {
            handle.document == document
                && slots
                    .get(handle.index as usize)
                    .is_some_and(|slot| slot.node.is_some() && slot.generation == handle.generation)
        });
    }

    fn node_ids(&mut self, handle: NodeHandle) -> Result<NodeSceneIds, EngineError> {
        if let Some(ids) = self.nodes.get(&handle) {
            return Ok(*ids);
        }
        self.nodes
            .try_reserve(1)
            .map_err(|_| EngineError::AllocationFailure)?;
        let first = allocate_ids(6)?;
        let ids = NodeSceneIds {
            layer: SceneLayerId(first),
            effect: SceneEffectId(first + 1),
            clip: SceneClipId(first + 2),
            scroll: SceneScrollId(first + 3),
            content_clip: SceneClipId(first + 4),
            content_scroll: SceneScrollId(first + 5),
        };
        self.nodes.insert(handle, ids);
        Ok(ids)
    }

    fn recording_id(&mut self, role: SceneRecordingRole) -> Result<SceneLayerId, EngineError> {
        if let Some(id) = self.recordings.get(&role) {
            return Ok(*id);
        }
        self.recordings
            .try_reserve(1)
            .map_err(|_| EngineError::AllocationFailure)?;
        let id = SceneLayerId(allocate_ids(1)?);
        self.recordings.insert(role, id);
        Ok(id)
    }
}

struct LayerBuilder {
    ids: NodeSceneIds,
    owner: SceneNodeIdentity,
    role: SceneLayerRole,
    effect: SceneEffectSnapshot,
    projections: Vec<SceneFragmentProjection>,
    clips: Vec<SceneClipProjection>,
    scrolls: Vec<SceneScrollProjection>,
    content_clips: Vec<SceneClipProjection>,
    content_scrolls: Vec<SceneScrollProjection>,
}

impl Engine {
    pub(crate) fn retained_scene_plan(
        &mut self,
        fragment: &Fragment,
        recording: &RecordedPicture,
    ) -> Result<RetainedScenePlan, EngineError> {
        self.scene_registry.retain_live(self.id, &self.slots);
        let mut plan = RetainedScenePlan {
            recordings: Vec::new(),
            layers: Vec::new(),
            effects: Vec::new(),
            clips: Vec::new(),
            scrolls: Vec::new(),
            unidentified_owned_box_paths: Vec::new(),
        };
        plan.recordings
            .try_reserve(4)
            .map_err(|_| EngineError::AllocationFailure)?;
        for (role, present) in [
            (SceneRecordingRole::Canonical, true),
            (SceneRecordingRole::DocumentContent, true),
            (
                SceneRecordingRole::ViewportScrollingContent,
                recording.viewport_content_layer().is_some(),
            ),
            (
                SceneRecordingRole::ViewportOverlay,
                recording.viewport_overlay_picture().is_some(),
            ),
        ] {
            if present {
                plan.recordings.push(SceneRecordingIdentity {
                    id: self.scene_registry.recording_id(role)?,
                    role,
                });
            }
        }
        let mut builders = Vec::new();
        let mut indices = HashMap::new();
        let mut path = Vec::new();
        self.collect_scene_layers(
            fragment,
            None,
            &mut path,
            &mut builders,
            &mut indices,
            &mut plan.unidentified_owned_box_paths,
        )?;
        let count = builders.len();
        plan.layers
            .try_reserve(count)
            .map_err(|_| EngineError::AllocationFailure)?;
        plan.effects
            .try_reserve(count)
            .map_err(|_| EngineError::AllocationFailure)?;
        let box_count = count.checked_mul(2).ok_or(EngineError::AllocationFailure)?;
        plan.clips
            .try_reserve(box_count)
            .map_err(|_| EngineError::AllocationFailure)?;
        plan.scrolls
            .try_reserve(box_count)
            .map_err(|_| EngineError::AllocationFailure)?;
        for builder in builders {
            let content_clip =
                (!builder.content_clips.is_empty()).then_some(builder.ids.content_clip);
            let scroll = if !builder.content_scrolls.is_empty() {
                Some(builder.ids.content_scroll)
            } else {
                (!builder.scrolls.is_empty()).then_some(builder.ids.scroll)
            };
            plan.layers.push(SceneLayerSnapshot {
                id: builder.ids.layer,
                owner: builder.owner,
                role: builder.role,
                effect: builder.ids.effect,
                clip: builder.ids.clip,
                content_clip,
                scroll,
                projections: builder.projections.into(),
            });
            plan.effects.push(builder.effect);
            plan.clips.push(SceneClipSnapshot {
                id: builder.ids.clip,
                owner: builder.owner,
                role: SceneBoxRole::Principal,
                projections: builder.clips.into(),
            });
            if !builder.scrolls.is_empty() {
                plan.scrolls.push(SceneScrollSnapshot {
                    id: builder.ids.scroll,
                    owner: builder.owner,
                    role: SceneBoxRole::Principal,
                    projections: builder.scrolls.into(),
                });
            }
            if let Some(id) = content_clip {
                plan.clips.push(SceneClipSnapshot {
                    id,
                    owner: builder.owner,
                    role: SceneBoxRole::FieldsetContent,
                    projections: builder.content_clips.into(),
                });
            }
            if !builder.content_scrolls.is_empty() {
                plan.scrolls.push(SceneScrollSnapshot {
                    id: builder.ids.content_scroll,
                    owner: builder.owner,
                    role: SceneBoxRole::FieldsetContent,
                    projections: builder.content_scrolls.into(),
                });
            }
        }
        Ok(plan)
    }

    fn collect_scene_layers(
        &mut self,
        fragment: &Fragment,
        layout_parent: Option<SceneLayerId>,
        path: &mut Vec<usize>,
        builders: &mut Vec<LayerBuilder>,
        indices: &mut HashMap<NodeHandle, usize>,
        unidentified: &mut Vec<Arc<[usize]>>,
    ) -> Result<(), EngineError> {
        let mut child_parent = layout_parent;
        // Ordinary borrowed helpers do not create authored effects or scroll
        // owners. Explicit delegated content boxes are handled separately.
        if fragment.owns_element_box() {
            let handle = self
                .node_slots
                .get(&fragment.node_id)
                .copied()
                .map(|index| self.handle_for_slot(index));
            if let Some(handle) = handle {
                let ids = self.scene_registry.node_ids(handle)?;
                child_parent = Some(ids.layer);
                let index = if let Some(index) = indices.get(&handle) {
                    *index
                } else {
                    builders
                        .try_reserve(1)
                        .map_err(|_| EngineError::AllocationFailure)?;
                    indices
                        .try_reserve(1)
                        .map_err(|_| EngineError::AllocationFailure)?;
                    let node = self.document.node(fragment.node_id);
                    let style = &node.style;
                    let index = builders.len();
                    builders.push(LayerBuilder {
                        ids,
                        owner: owner(handle),
                        role: if node.pseudo_kind.is_some() {
                            SceneLayerRole::GeneratedBox
                        } else {
                            SceneLayerRole::AuthoredBox
                        },
                        effect: SceneEffectSnapshot {
                            id: ids.effect,
                            owner: owner(handle),
                            source_parent: self
                                .node_slots
                                .get(&node.parent)
                                .map(|index| owner(self.handle_for_slot(*index))),
                            opacity: style.opacity,
                            blur: style.filter_blur,
                            grayscale: style.filter_grayscale,
                            paint_containment: style.has_paint_containment(),
                            will_change_transform: style.will_change_transform,
                        },
                        projections: Vec::new(),
                        clips: Vec::new(),
                        scrolls: Vec::new(),
                        content_clips: Vec::new(),
                        content_scrolls: Vec::new(),
                    });
                    indices.insert(handle, index);
                    index
                };
                let builder = &mut builders[index];
                builder
                    .projections
                    .try_reserve(1)
                    .map_err(|_| EngineError::AllocationFailure)?;
                builder
                    .clips
                    .try_reserve(1)
                    .map_err(|_| EngineError::AllocationFailure)?;
                let mut owned_path = Vec::new();
                owned_path
                    .try_reserve(path.len())
                    .map_err(|_| EngineError::AllocationFailure)?;
                owned_path.extend_from_slice(path);
                let owned_path: Arc<[usize]> = owned_path.into();
                let node = self.document.node(fragment.node_id);
                let style = &node.style;
                let overflow = if !fragment.owns_scroll_box() {
                    (Overflow::Visible, Overflow::Visible)
                } else {
                    fragment
                        .scroll_area
                        .map(|area| (area.overflow_x, area.overflow_y))
                        .unwrap_or_else(|| {
                            openui_layout::used_overflow_pair(&self.document, fragment.node_id)
                        })
                };
                builder.projections.push(SceneFragmentProjection {
                    fragment_path: Arc::clone(&owned_path),
                    layout_parent,
                    offset: fragment.offset,
                    border_box: fragment.border_box_rect(),
                    transform: fragment.resolved_transform,
                });
                builder.clips.push(clip_projection(
                    fragment,
                    Arc::clone(&owned_path),
                    style,
                    overflow,
                    true,
                ));
                if let Some(area) = fragment.scroll_area.filter(|area| {
                    fragment.owns_scroll_box()
                        && (area.is_scroll_container() || fragment.viewport_scrollport.is_some())
                }) {
                    builder
                        .scrolls
                        .try_reserve(1)
                        .map_err(|_| EngineError::AllocationFailure)?;
                    builder.scrolls.push(SceneScrollProjection {
                        fragment_path: owned_path,
                        box_offset: fragment.offset,
                        area,
                        offset: (node.scroll_left, node.scroll_top),
                        viewport: fragment.viewport_scrollport.is_some(),
                        sticky: fragment.sticky_timeline_geometry,
                    });
                }
            } else {
                unidentified
                    .try_reserve(1)
                    .map_err(|_| EngineError::AllocationFailure)?;
                let mut owned_path = Vec::new();
                owned_path
                    .try_reserve(path.len())
                    .map_err(|_| EngineError::AllocationFailure)?;
                owned_path.extend_from_slice(path);
                unidentified.push(owned_path.into());
            }
        }
        if let Some(AnonymousLayoutBox::FieldsetContent {
            owner: content_owner,
        }) = fragment.anonymous_layout_box
        {
            // A real anonymous formatting box delegates scrolling under the
            // authored owner's generation. It never publishes another layer,
            // effect, transform, or semantic element projection.
            let handle = self
                .node_slots
                .get(&content_owner)
                .copied()
                .map(|index| self.handle_for_slot(index));
            let index = handle.and_then(|handle| indices.get(&handle).copied());
            if let Some(index) = index.filter(|_| fragment.owns_scroll_box()) {
                let mut owned_path = Vec::new();
                owned_path
                    .try_reserve(path.len())
                    .map_err(|_| EngineError::AllocationFailure)?;
                owned_path.extend_from_slice(path);
                let owned_path: Arc<[usize]> = owned_path.into();
                let builder = &mut builders[index];
                let node = self.document.node(content_owner);
                let effective = fragment.layout_style(&node.style);
                // Used overflow must come from this effective retained box.
                // Missing area is an explicit metadata coverage gap.
                if let Some(area) = fragment.scroll_area {
                    builder
                        .content_clips
                        .try_reserve(1)
                        .map_err(|_| EngineError::AllocationFailure)?;
                    builder.content_clips.push(clip_projection(
                        fragment,
                        Arc::clone(&owned_path),
                        effective,
                        (area.overflow_x, area.overflow_y),
                        false,
                    ));
                    if area.is_scroll_container() {
                        builder
                            .content_scrolls
                            .try_reserve(1)
                            .map_err(|_| EngineError::AllocationFailure)?;
                        builder.content_scrolls.push(SceneScrollProjection {
                            fragment_path: owned_path,
                            box_offset: fragment.offset,
                            area,
                            offset: (node.scroll_left, node.scroll_top),
                            viewport: false,
                            sticky: fragment.sticky_timeline_geometry,
                        });
                    }
                } else {
                    unidentified
                        .try_reserve(1)
                        .map_err(|_| EngineError::AllocationFailure)?;
                    unidentified.push(owned_path);
                }
            } else {
                unidentified
                    .try_reserve(1)
                    .map_err(|_| EngineError::AllocationFailure)?;
                let mut owned_path = Vec::new();
                owned_path
                    .try_reserve(path.len())
                    .map_err(|_| EngineError::AllocationFailure)?;
                owned_path.extend_from_slice(path);
                unidentified.push(owned_path.into());
            }
        }
        if !fragment.children.is_empty() {
            path.try_reserve(1)
                .map_err(|_| EngineError::AllocationFailure)?;
        }
        for (index, child) in fragment.children.iter().enumerate() {
            path.push(index);
            self.collect_scene_layers(child, child_parent, path, builders, indices, unidentified)?;
            path.pop();
        }
        Ok(())
    }
}

fn clip_projection(
    fragment: &Fragment,
    path: Arc<[usize]>,
    effective: &ComputedStyle,
    overflow: (Overflow, Overflow),
    owns_effects: bool,
) -> SceneClipProjection {
    SceneClipProjection {
        fragment_path: path,
        offset: fragment.offset,
        client_rect: fragment
            .viewport_scrollport
            .map(|viewport| viewport.client_rect)
            .or_else(|| fragment.scroll_area.map(|area| area.client_rect))
            .unwrap_or_else(|| fragment.border_box_rect()),
        overflow_clips_x: overflow.0 != Overflow::Visible,
        overflow_clips_y: overflow.1 != Overflow::Visible,
        paint_containment: owns_effects && effective.has_paint_containment(),
        structural_clip: fragment.has_overflow_clip,
        block_axis_clip_only: fragment.block_axis_clip_only,
        inline_axis_clip_only: fragment.inline_axis_clip_only,
        fragmentation_writing_direction: fragment.fragmentation_writing_direction,
        border_radii: if fragment.ignore_border_radius {
            [(0.0, 0.0); 4]
        } else {
            [
                effective.border_top_left_radius,
                effective.border_top_right_radius,
                effective.border_bottom_right_radius,
                effective.border_bottom_left_radius,
            ]
        },
        border_radius_percent: if fragment.ignore_border_radius {
            [(false, false); 4]
        } else {
            effective.border_radius_percent
        },
        clip_path_insets: if owns_effects {
            effective.clip_path_inset
        } else {
            None
        },
        overflow_clip_box: match effective.overflow_clip_box {
            openui_style::OverflowClipBox::BorderBox => SceneOverflowClipBox::Border,
            openui_style::OverflowClipBox::PaddingBox => SceneOverflowClipBox::Padding,
            openui_style::OverflowClipBox::ContentBox => SceneOverflowClipBox::Content,
        },
        overflow_clip_margin: effective.overflow_clip_margin,
        used_border: openui_layout::ScrollArea::used_border(fragment),
        used_padding: openui_layout::ScrollArea::used_padding(fragment),
    }
}
