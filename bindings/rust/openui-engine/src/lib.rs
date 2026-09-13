//! Retained, thread-affine Open UI document engine.

mod accessibility;
mod animation;
mod interaction;

pub use accessibility::{
    AccessibilityAction, AccessibilityActionData, AccessibilityActionRequest, AccessibilityLive,
    AccessibilityNode, AccessibilityNodeId, AccessibilityPlatformAction, AccessibilityRelation,
    AccessibilityRole, AccessibilityTreeUpdate,
};

pub use animation::{
    AnimationEvent, AnimationEventKind, AnimationId, AnimationState, AnimationTimeline,
    ScrollAnimationId,
};

pub use interaction::{
    ActivationResult, ControlAdjustment, ControlState, EditCommand, EventPhase, EventRoute,
    FocusOrigin, PointerEventKind, PointerUpdate, RouteStep, TextDirection, TextUnit,
};

use openui_compositor::{SceneGeneration, SceneRect, SceneSnapshot};
use openui_dom::{
    Document as NativeDocument, ElementTag, NodeId, ReplacedContent, ReplacedResourceKind,
};
use openui_layout::Fragment;
use openui_paint::record_fragment;
use openui_style::{
    apply_to_computed, ImageResourceId, InvalidationClass, StyleProperty, StyleValue,
};
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: u32,
    pub height: u32,
    pub scale_factor: f64,
}

impl Viewport {
    pub fn new(width: u32, height: u32) -> Result<Self, EngineError> {
        if width == 0 || height == 0 {
            return Err(EngineError::InvalidViewport);
        }
        Ok(Self {
            width,
            height,
            scale_factor: 1.0,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle {
    document: u64,
    index: u32,
    generation: u32,
}

impl NodeHandle {
    pub fn document_id(self) -> u64 {
        self.document
    }
    pub fn index(self) -> u32 {
        self.index
    }
    pub fn generation(self) -> u32 {
        self.generation
    }
    pub fn downgrade(self) -> WeakNode {
        WeakNode(self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WeakNode(NodeHandle);

impl WeakNode {
    pub fn upgrade(self, engine: &Engine) -> Result<NodeHandle, EngineError> {
        engine.resolve(self.0)?;
        Ok(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EngineError {
    InvalidViewport,
    WrongDocument,
    StaleHandle,
    RootRemoval,
    Cycle,
    AlreadyAttached,
    DetachedParent,
    InvalidSibling,
    PropertyType { property: StyleProperty },
    UnknownResource,
    InvalidInput(&'static str),
    InvalidSelection,
    NotAControl,
    NotEditable,
    NotFocusable,
    Render(String),
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidViewport => f.write_str("viewport dimensions must be positive"),
            Self::WrongDocument => f.write_str("node belongs to another document"),
            Self::StaleHandle => f.write_str("node handle is stale"),
            Self::RootRemoval => f.write_str("the document root cannot be removed"),
            Self::Cycle => f.write_str("tree mutation would create a cycle"),
            Self::AlreadyAttached => f.write_str("child is already attached"),
            Self::DetachedParent => f.write_str("parent is detached"),
            Self::InvalidSibling => f.write_str("reference node is not a child of the parent"),
            Self::PropertyType { property } => {
                write!(f, "wrong value type for `{}`", property.metadata().css_name)
            }
            Self::UnknownResource => f.write_str("resource does not belong to this document"),
            Self::InvalidInput(message) => f.write_str(message),
            Self::InvalidSelection => f.write_str("selection is outside the control value"),
            Self::NotAControl => f.write_str("node is not a form control"),
            Self::NotEditable => f.write_str("control is not editable"),
            Self::NotFocusable => f.write_str("node is not focusable"),
            Self::Render(value) => write!(f, "render failed: {value}"),
        }
    }
}

impl std::error::Error for EngineError {}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LifecycleStats {
    pub layouts: u64,
    pub paints: u64,
    pub scenes: u64,
}

/// Counts of document-owned objects used by leak and soak qualification.
///
/// `handle_slots` is retained capacity and may include reusable vacant slots;
/// `live_nodes` is the number of currently valid node handles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EngineObjectCounts {
    pub live_nodes: usize,
    pub arena_nodes: usize,
    pub reusable_arena_nodes: usize,
    pub handle_slots: usize,
    pub vacant_handle_slots: usize,
    pub controls: usize,
    pub semantic_overrides: usize,
    pub animations: usize,
    pub scroll_animations: usize,
    pub image_resources: usize,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DirtyGenerations {
    pub tree: u64,
    pub intrinsic: u64,
    pub layout: u64,
    pub paint: u64,
    pub compositing: u64,
    pub accessibility: u64,
}

#[derive(Clone)]
struct Slot {
    generation: u32,
    node: Option<NodeId>,
    authored: BTreeMap<u16, StyleValue>,
}

#[derive(Debug, Clone, Copy)]
struct Affine {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Affine {
    const IDENTITY: Self = Self {
        a: 1.0,
        b: 0.0,
        c: 0.0,
        d: 1.0,
        e: 0.0,
        f: 0.0,
    };

    fn translate(x: f32, y: f32) -> Self {
        Self {
            e: x,
            f: y,
            ..Self::IDENTITY
        }
    }

    fn then(self, rhs: Self) -> Self {
        Self {
            a: self.a * rhs.a + self.c * rhs.b,
            b: self.b * rhs.a + self.d * rhs.b,
            c: self.a * rhs.c + self.c * rhs.d,
            d: self.b * rhs.c + self.d * rhs.d,
            e: self.a * rhs.e + self.c * rhs.f + self.e,
            f: self.b * rhs.e + self.d * rhs.f + self.f,
        }
    }

    fn inverse(self) -> Option<Self> {
        let determinant = self.a * self.d - self.b * self.c;
        if !determinant.is_finite() || determinant.abs() < f32::EPSILON {
            return None;
        }
        let inverse = determinant.recip();
        Some(Self {
            a: self.d * inverse,
            b: -self.b * inverse,
            c: -self.c * inverse,
            d: self.a * inverse,
            e: (self.c * self.f - self.d * self.e) * inverse,
            f: (self.b * self.e - self.a * self.f) * inverse,
        })
    }

    fn map(self, x: f32, y: f32) -> (f32, f32) {
        (
            self.a * x + self.c * y + self.e,
            self.b * x + self.d * y + self.f,
        )
    }
}

#[derive(Debug, Clone)]
struct HitClip {
    world_to_local: Affine,
    width: f32,
    height: f32,
    radii: [(f32, f32); 4],
}

#[derive(Debug, Clone)]
struct HitEntry {
    node: NodeId,
    local_to_world: Affine,
    world_to_local: Affine,
    width: f32,
    height: f32,
    clips: Vec<HitClip>,
}

#[derive(Debug, Clone, Copy, Default)]
struct DirtyState {
    layout: bool,
    paint: bool,
    compositing: bool,
    accessibility: bool,
    hit_test: bool,
}

impl DirtyState {
    fn visual(self) -> bool {
        self.layout || self.paint || self.compositing
    }
}

/// Owns all mutable state for one UI document. `Engine` is deliberately
/// `!Send` and `!Sync`; immutable `SceneSnapshot`s may cross threads.
pub struct Engine {
    id: u64,
    viewport: Viewport,
    document: NativeDocument,
    slots: Vec<Slot>,
    free_slots: Vec<u32>,
    node_slots: HashMap<NodeId, u32>,
    dirty: DirtyState,
    dirty_generations: DirtyGenerations,
    latest_fragment: Option<Arc<Fragment>>,
    latest_scene: Option<SceneSnapshot>,
    hit_test: Vec<HitEntry>,
    focused: Option<NodeHandle>,
    pointer_capture: HashMap<u64, NodeHandle>,
    hover_paths: HashMap<u64, Vec<NodeHandle>>,
    active_pointers: HashMap<u64, NodeHandle>,
    controls: HashMap<u32, ControlState>,
    semantics: HashMap<u32, accessibility::SemanticProperties>,
    accessibility_nodes: HashMap<AccessibilityNodeId, AccessibilityNode>,
    accessibility_initialized: bool,
    focus_visible: bool,
    modal_root: Option<NodeHandle>,
    focus_before_modal: Option<NodeHandle>,
    animation_time_ms: f64,
    animations: BTreeMap<AnimationId, animation::AnimationInstance>,
    animation_events: Vec<AnimationEvent>,
    next_animation_id: u64,
    scroll_animations: BTreeMap<ScrollAnimationId, animation::ScrollAnimationInstance>,
    next_scroll_animation_id: u64,
    reduced_motion: bool,
    stats: LifecycleStats,
    _thread_affine: PhantomData<Rc<()>>,
}

impl Engine {
    pub fn new(viewport: Viewport) -> Result<Self, EngineError> {
        if viewport.width == 0
            || viewport.height == 0
            || !viewport.scale_factor.is_finite()
            || viewport.scale_factor <= 0.0
        {
            return Err(EngineError::InvalidViewport);
        }
        let document = NativeDocument::new();
        let root_node = document.root();
        let root_slot = Slot {
            generation: 1,
            node: Some(root_node),
            authored: BTreeMap::new(),
        };
        Ok(Self {
            id: NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed),
            viewport,
            document,
            slots: vec![root_slot],
            free_slots: Vec::new(),
            node_slots: HashMap::from([(root_node, 0)]),
            dirty: DirtyState {
                layout: true,
                paint: true,
                compositing: true,
                accessibility: true,
                hit_test: true,
            },
            dirty_generations: DirtyGenerations::default(),
            latest_fragment: None,
            latest_scene: None,
            hit_test: Vec::new(),
            focused: None,
            pointer_capture: HashMap::new(),
            hover_paths: HashMap::new(),
            active_pointers: HashMap::new(),
            controls: HashMap::new(),
            semantics: HashMap::new(),
            accessibility_nodes: HashMap::new(),
            accessibility_initialized: false,
            focus_visible: false,
            modal_root: None,
            focus_before_modal: None,
            animation_time_ms: 0.0,
            animations: BTreeMap::new(),
            animation_events: Vec::new(),
            next_animation_id: 1,
            scroll_animations: BTreeMap::new(),
            next_scroll_animation_id: 1,
            reduced_motion: false,
            stats: LifecycleStats::default(),
            _thread_affine: PhantomData,
        })
    }

    pub fn document_id(&self) -> u64 {
        self.id
    }
    pub fn viewport(&self) -> Viewport {
        self.viewport
    }
    pub fn root(&self) -> NodeHandle {
        self.handle_for_slot(0)
    }
    pub fn stats(&self) -> LifecycleStats {
        self.stats
    }
    pub fn object_counts(&self) -> EngineObjectCounts {
        EngineObjectCounts {
            live_nodes: self.node_slots.len(),
            arena_nodes: self.document.node_count(),
            reusable_arena_nodes: self.document.reusable_node_count(),
            handle_slots: self.slots.len(),
            vacant_handle_slots: self.free_slots.len(),
            controls: self.controls.len(),
            semantic_overrides: self.semantics.len(),
            animations: self.animations.len(),
            scroll_animations: self.scroll_animations.len(),
            image_resources: self.document.image_resource_count(),
        }
    }
    pub fn dirty_generations(&self) -> DirtyGenerations {
        self.dirty_generations
    }
    pub fn native_document(&self) -> &NativeDocument {
        &self.document
    }

    pub fn set_viewport(&mut self, viewport: Viewport) -> Result<(), EngineError> {
        if viewport.width == 0
            || viewport.height == 0
            || !viewport.scale_factor.is_finite()
            || viewport.scale_factor <= 0.0
        {
            return Err(EngineError::InvalidViewport);
        }
        if self.viewport != viewport {
            self.viewport = viewport;
            self.mark_dirty(InvalidationClass::Intrinsic);
        }
        Ok(())
    }

    pub fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        operation(self)
    }

    pub fn create_element(&mut self, tag: ElementTag) -> Result<NodeHandle, EngineError> {
        let node = self.document.create_node(tag);
        let index = if let Some(index) = self.free_slots.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            slot.authored.clear();
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 1,
                node: Some(node),
                authored: BTreeMap::new(),
            });
            index
        };
        self.node_slots.insert(node, index);
        let handle = self.handle_for_slot(index);
        self.initialize_control(handle, tag);
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(handle)
    }

    pub fn create_text(&mut self, text: impl Into<String>) -> Result<NodeHandle, EngineError> {
        let handle = self.create_element(ElementTag::Text)?;
        let node = self.resolve(handle)?;
        self.document.node_mut(node).text = Some(text.into());
        Ok(handle)
    }

    pub fn append_child(
        &mut self,
        parent: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), EngineError> {
        let parent = self.resolve(parent)?;
        let child = self.resolve(child)?;
        if !self.document.node(child).parent.is_none() {
            return Err(EngineError::AlreadyAttached);
        }
        let mut ancestor = parent;
        while !ancestor.is_none() {
            if ancestor == child {
                return Err(EngineError::Cycle);
            }
            ancestor = self.document.node(ancestor).parent;
        }
        self.document.append_child(parent, child);
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    /// Append a node, moving it from its existing parent when necessary.
    pub fn append_or_move_child(
        &mut self,
        parent: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), EngineError> {
        let parent_node = self.resolve(parent)?;
        let child_node = self.resolve(child)?;
        let mut ancestor = parent_node;
        while !ancestor.is_none() {
            if ancestor == child_node {
                return Err(EngineError::Cycle);
            }
            ancestor = self.document.node(ancestor).parent;
        }
        self.document.detach(child_node);
        self.document.append_child(parent_node, child_node);
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    /// Insert or move `child` immediately before `before` under `parent`.
    pub fn insert_before(
        &mut self,
        parent: NodeHandle,
        child: NodeHandle,
        before: NodeHandle,
    ) -> Result<(), EngineError> {
        let parent_node = self.resolve(parent)?;
        let child_node = self.resolve(child)?;
        let before_node = self.resolve(before)?;
        if child_node == before_node {
            return Ok(());
        }
        if self.document.node(before_node).parent != parent_node {
            return Err(EngineError::InvalidSibling);
        }
        let mut ancestor = parent_node;
        while !ancestor.is_none() {
            if ancestor == child_node {
                return Err(EngineError::Cycle);
            }
            ancestor = self.document.node(ancestor).parent;
        }
        self.document.detach(child_node);
        self.document.insert_before(before_node, child_node);
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    pub fn parent(&self, handle: NodeHandle) -> Result<Option<NodeHandle>, EngineError> {
        let node = self.resolve(handle)?;
        let parent = self.document.node(node).parent;
        Ok(self
            .node_slots
            .get(&parent)
            .map(|index| self.handle_for_slot(*index)))
    }

    pub fn children(&self, handle: NodeHandle) -> Result<Vec<NodeHandle>, EngineError> {
        let node = self.resolve(handle)?;
        Ok(self
            .document
            .children(node)
            .filter_map(|child| self.node_slots.get(&child))
            .map(|index| self.handle_for_slot(*index))
            .collect())
    }

    pub fn remove_children(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        let children = self.children(handle)?;
        for child in children {
            self.remove(child)?;
        }
        Ok(())
    }

    pub fn remove(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        if node == self.document.root() {
            return Err(EngineError::RootRemoval);
        }
        let mut descendants = Vec::new();
        self.collect_subtree(node, &mut descendants);
        self.document.detach(node);
        let removed_handles: Vec<_> = descendants
            .iter()
            .filter_map(|node| self.node_slots.get(node))
            .map(|index| self.handle_for_slot(*index))
            .collect();
        self.cancel_animations_for_handles(&removed_handles);
        if self
            .focused
            .is_some_and(|focused| removed_handles.contains(&focused))
        {
            self.focused = None;
        }
        self.pointer_capture
            .retain(|_, captured| !removed_handles.contains(captured));
        self.active_pointers
            .retain(|_, active| !removed_handles.contains(active));
        self.hover_paths.retain(|_, path| {
            path.retain(|node| !removed_handles.contains(node));
            !path.is_empty()
        });
        if self
            .modal_root
            .is_some_and(|modal| removed_handles.contains(&modal))
        {
            self.modal_root = None;
        }
        for node in &descendants {
            if let Some(index) = self.node_slots.remove(node) {
                self.controls.remove(&index);
                self.semantics.remove(&index);
                let slot = &mut self.slots[index as usize];
                slot.node = None;
                slot.authored.clear();
                slot.generation = slot.generation.wrapping_add(1).max(1);
                self.free_slots.push(index);
            }
        }
        for node in descendants.into_iter().rev() {
            if !self.document.node(node).parent.is_none() {
                self.document.detach(node);
            }
            debug_assert!(self.document.release_detached_node(node));
        }
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    pub fn set_text(
        &mut self,
        handle: NodeHandle,
        text: impl Into<String>,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let text = text.into();
        if self.document.node(node).text.as_deref() == Some(text.as_str()) {
            return Ok(());
        }
        self.document.node_mut(node).text = Some(text);
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(())
    }

    pub fn set_attribute(
        &mut self,
        handle: NodeHandle,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let name = name.into().to_ascii_lowercase();
        let value = value.into();
        if self.document.node(node).attributes.get(&name) == Some(&value) {
            return Ok(());
        }
        self.document
            .set_attribute(node, name.clone(), value.clone());
        self.sync_control_attribute(handle, &name, &value);
        self.mark_dirty(InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn attribute(&self, handle: NodeHandle, name: &str) -> Result<Option<&str>, EngineError> {
        let node = self.resolve(handle)?;
        Ok(self.document.attribute(node, name))
    }

    pub fn remove_attribute(
        &mut self,
        handle: NodeHandle,
        name: &str,
    ) -> Result<bool, EngineError> {
        let node = self.resolve(handle)?;
        let removed = self
            .document
            .node_mut(node)
            .attributes
            .remove(&name.to_ascii_lowercase())
            .is_some();
        if removed {
            self.remove_control_attribute(handle, &name.to_ascii_lowercase());
            self.mark_dirty(InvalidationClass::Accessibility);
        }
        Ok(removed)
    }

    pub fn set_property(
        &mut self,
        handle: NodeHandle,
        property: StyleProperty,
        value: StyleValue,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        if self.slots[handle.index as usize]
            .authored
            .get(&(property as u16))
            == Some(&value)
        {
            return Ok(());
        }
        apply_to_computed(
            &mut self.document.node_mut(node).style,
            property,
            &value,
            (self.viewport.width as f32, self.viewport.height as f32),
        )
        .map_err(|_| EngineError::PropertyType { property })?;
        self.slots[handle.index as usize]
            .authored
            .insert(property as u16, value.clone());
        let mut has_animation = false;
        for animation in self.animations.values_mut().filter(|animation| {
            animation.target == handle && animation.keyframes.property() == property
        }) {
            animation.underlying = value.clone();
            animation.last_applied = None;
            has_animation = true;
        }
        self.dirty.hit_test = true;
        self.mark_dirty(property.metadata().invalidation);
        if has_animation {
            self.sample_animations()?;
        }
        Ok(())
    }

    pub fn register_image_resource(
        &mut self,
        source: impl Into<String>,
        mime_type: impl Into<String>,
        sha256: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> ImageResourceId {
        self.document
            .register_image_resource(source, mime_type, sha256, bytes)
    }

    pub fn set_image_resource(
        &mut self,
        handle: NodeHandle,
        resource: ImageResourceId,
        intrinsic_size: Option<(f32, f32)>,
    ) -> Result<(), EngineError> {
        if self.document.image_resource(resource).is_none() {
            return Err(EngineError::UnknownResource);
        }
        let node = self.resolve(handle)?;
        let (intrinsic_width, intrinsic_height) = intrinsic_size
            .map(|(width, height)| (Some(width), Some(height)))
            .unwrap_or((None, None));
        let intrinsic_ratio = intrinsic_size
            .and_then(|(width, height)| (width > 0.0 && height > 0.0).then_some((width, height)));
        self.document.node_mut(node).replaced = Some(ReplacedContent {
            resource: ReplacedResourceKind::Image(resource),
            intrinsic_width,
            intrinsic_height,
            intrinsic_ratio,
        });
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(())
    }

    pub fn computed_style(
        &self,
        handle: NodeHandle,
    ) -> Result<&openui_style::ComputedStyle, EngineError> {
        Ok(&self.document.node(self.resolve(handle)?).style)
    }

    pub fn cursor(&self, handle: NodeHandle) -> Result<openui_style::Cursor, EngineError> {
        let mut current = Some(handle);
        while let Some(node) = current {
            self.resolve(node)?;
            if let Some(StyleValue::Cursor(cursor)) = self.slots[node.index as usize]
                .authored
                .get(&(StyleProperty::Cursor as u16))
            {
                return Ok(*cursor);
            }
            current = self.parent(node)?;
        }
        Ok(openui_style::Cursor::Auto)
    }

    pub fn element_tag(&self, handle: NodeHandle) -> Result<ElementTag, EngineError> {
        Ok(self.document.node(self.resolve(handle)?).tag)
    }

    pub fn scroll_offset(&self, handle: NodeHandle) -> Result<(f64, f64), EngineError> {
        let node = self.resolve(handle)?;
        let node = self.document.node(node);
        Ok((node.scroll_left as f64, node.scroll_top as f64))
    }

    pub fn scroll_to(&mut self, handle: NodeHandle, x: f64, y: f64) -> Result<(), EngineError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::Render("scroll offsets must be finite".into()));
        }
        let node = self.resolve(handle)?;
        let (x, y) = (x.max(0.0) as f32, y.max(0.0) as f32);
        if (
            self.document.node(node).scroll_left,
            self.document.node(node).scroll_top,
        ) == (x, y)
        {
            return Ok(());
        }
        self.scroll_animations
            .retain(|_, animation| animation.target != handle);
        let data = self.document.node_mut(node);
        data.scroll_left = x;
        data.scroll_top = y;
        self.dirty.hit_test = true;
        self.mark_dirty(InvalidationClass::Composite);
        Ok(())
    }

    pub fn focus(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        self.focus_with_origin(handle, FocusOrigin::Script)
            .map(|_| ())
    }

    pub fn blur(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        self.resolve(handle)?;
        if self.focused == Some(handle) {
            self.focused = None;
            self.focus_visible = false;
            if let Ok(node) = self.resolve(handle) {
                self.document
                    .node_mut(node)
                    .attributes
                    .remove("data-oui-focused");
            }
            self.dirty.paint = true;
            self.mark_dirty(InvalidationClass::Accessibility);
        }
        Ok(())
    }

    pub fn focused(&self) -> Option<NodeHandle> {
        self.focused
    }

    pub fn set_pointer_capture(
        &mut self,
        pointer_id: u64,
        handle: NodeHandle,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        self.pointer_capture.insert(pointer_id, handle);
        Ok(())
    }

    pub fn release_pointer_capture(
        &mut self,
        pointer_id: u64,
        handle: NodeHandle,
    ) -> Result<(), EngineError> {
        self.resolve(handle)?;
        if self.pointer_capture.get(&pointer_id) == Some(&handle) {
            self.pointer_capture.remove(&pointer_id);
        }
        Ok(())
    }

    pub fn pointer_capture(&self, pointer_id: u64) -> Option<NodeHandle> {
        self.pointer_capture.get(&pointer_id).copied()
    }

    pub fn set_animation_time(&mut self, time_ms: f64) -> Result<(), EngineError> {
        if !time_ms.is_finite() || time_ms < 0.0 {
            return Err(EngineError::Render(
                "animation time must be finite and non-negative".into(),
            ));
        }
        self.animation_time_ms = time_ms;
        self.sample_animations()?;
        self.sample_scroll_animations()
    }

    pub fn animation_time(&self) -> f64 {
        self.animation_time_ms
    }

    pub fn update(&mut self) -> Result<&SceneSnapshot, EngineError> {
        if self.dirty.hit_test && !self.dirty.layout {
            if let Some(fragment) = self.latest_fragment.clone() {
                self.rebuild_hit_test(&fragment);
            }
            self.dirty.hit_test = false;
        }
        if !self.dirty.visual() {
            return self
                .latest_scene
                .as_ref()
                .ok_or_else(|| EngineError::Render("missing initial scene".into()));
        }
        let width = self.viewport.width as i32;
        let height = self.viewport.height as i32;
        if self.dirty.layout || self.latest_fragment.is_none() {
            let root_style = &self.document.node(self.document.root()).style;
            let direction = root_style
                .direction
                .writing_direction(root_style.writing_mode);
            let space = openui_layout::ConstraintSpace::for_root_with_writing_direction(
                openui_geometry::LayoutUnit::from_i32(width),
                openui_geometry::LayoutUnit::from_i32(height),
                direction,
            );
            let fragment = Arc::new(openui_layout::block_layout(
                &self.document,
                self.document.root(),
                &space,
            ));
            self.stats.layouts += 1;
            self.rebuild_hit_test(&fragment);
            self.latest_fragment = Some(fragment);
        }
        let fragment = self
            .latest_fragment
            .as_ref()
            .expect("visual dirtiness always establishes a fragment")
            .clone();
        let recording = record_fragment(&self.document, &fragment, width, height)
            .map_err(EngineError::Render)?;
        self.stats.paints += 1;
        let generation = SceneGeneration(self.stats.scenes + 1);
        let damage = vec![SceneRect {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
        }];
        self.latest_scene = Some(SceneSnapshot::new(generation, recording, fragment, damage));
        self.stats.scenes += 1;
        let accessibility = self.dirty.accessibility;
        self.dirty = DirtyState {
            accessibility,
            ..DirtyState::default()
        };
        Ok(self.latest_scene.as_ref().expect("scene assigned"))
    }

    pub fn scene(&mut self) -> Result<SceneSnapshot, EngineError> {
        Ok(self.update()?.clone())
    }

    pub fn hit_test(&mut self, x: f32, y: f32) -> Result<Option<NodeHandle>, EngineError> {
        self.update()?;
        for entry in self.hit_test.iter().rev() {
            let (local_x, local_y) = entry.world_to_local.map(x, y);
            if rounded_rect_contains(local_x, local_y, entry.width, entry.height, [(0.0, 0.0); 4])
                && entry.clips.iter().all(|clip| {
                    let (clip_x, clip_y) = clip.world_to_local.map(x, y);
                    rounded_rect_contains(clip_x, clip_y, clip.width, clip.height, clip.radii)
                })
            {
                if let Some(index) = self.node_slots.get(&entry.node) {
                    return Ok(Some(self.handle_for_slot(*index)));
                }
            }
        }
        Ok(None)
    }

    pub fn bounds(&mut self, handle: NodeHandle) -> Result<Option<SceneRect>, EngineError> {
        let node = self.resolve(handle)?;
        self.update()?;
        Ok(self
            .hit_test
            .iter()
            .rev()
            .find(|entry| entry.node == node)
            .map(|entry| {
                let corners = [
                    entry.local_to_world.map(0.0, 0.0),
                    entry.local_to_world.map(entry.width, 0.0),
                    entry.local_to_world.map(0.0, entry.height),
                    entry.local_to_world.map(entry.width, entry.height),
                ];
                let min_x = corners
                    .iter()
                    .map(|point| point.0)
                    .fold(f32::INFINITY, f32::min);
                let max_x = corners
                    .iter()
                    .map(|point| point.0)
                    .fold(f32::NEG_INFINITY, f32::max);
                let min_y = corners
                    .iter()
                    .map(|point| point.1)
                    .fold(f32::INFINITY, f32::min);
                let max_y = corners
                    .iter()
                    .map(|point| point.1)
                    .fold(f32::NEG_INFINITY, f32::max);
                SceneRect {
                    x: min_x,
                    y: min_y,
                    width: max_x - min_x,
                    height: max_y - min_y,
                }
            }))
    }

    fn handle_for_slot(&self, index: u32) -> NodeHandle {
        NodeHandle {
            document: self.id,
            index,
            generation: self.slots[index as usize].generation,
        }
    }
    fn resolve(&self, handle: NodeHandle) -> Result<NodeId, EngineError> {
        if handle.document != self.id {
            return Err(EngineError::WrongDocument);
        }
        let Some(slot) = self.slots.get(handle.index as usize) else {
            return Err(EngineError::StaleHandle);
        };
        if slot.generation != handle.generation {
            return Err(EngineError::StaleHandle);
        }
        slot.node.ok_or(EngineError::StaleHandle)
    }
    fn collect_subtree(&self, node: NodeId, out: &mut Vec<NodeId>) {
        out.push(node);
        for child in self.document.children(node) {
            self.collect_subtree(child, out);
        }
    }
    fn mark_dirty(&mut self, class: InvalidationClass) {
        match class {
            InvalidationClass::Composite => {
                self.dirty.compositing = true;
                self.dirty_generations.compositing += 1;
            }
            InvalidationClass::Paint => {
                self.dirty.paint = true;
                self.dirty_generations.paint += 1;
            }
            InvalidationClass::Layout => {
                self.dirty.layout = true;
                self.dirty_generations.layout += 1;
            }
            InvalidationClass::Intrinsic => {
                self.dirty.layout = true;
                self.dirty_generations.intrinsic += 1;
            }
            InvalidationClass::Subtree => {
                self.dirty.layout = true;
                self.dirty_generations.tree += 1;
            }
            InvalidationClass::Accessibility => {
                self.dirty.accessibility = true;
                self.dirty_generations.accessibility += 1;
            }
        }
        if class != InvalidationClass::Accessibility {
            self.dirty.accessibility = true;
            self.dirty_generations.accessibility += 1;
        }
    }
    fn rebuild_hit_test(&mut self, fragment: &Fragment) {
        fn resolve_origin(length: &openui_geometry::Length, size: f32) -> f32 {
            if length.is_fixed() {
                length.value()
            } else if length.is_percent() {
                length.value() * size / 100.0
            } else if length.is_calculated() {
                length.calc_offset() + length.value() * size / 100.0
            } else {
                size * 0.5
            }
        }
        fn walk(
            document: &NativeDocument,
            fragment: &Fragment,
            parent_world: Affine,
            inherited_clips: &[HitClip],
            out: &mut Vec<HitEntry>,
        ) {
            let width = fragment.size.width.to_f32();
            let height = fragment.size.height.to_f32();
            let translated = parent_world.then(Affine::translate(
                fragment.offset.left.to_f32(),
                fragment.offset.top.to_f32(),
            ));
            let (world, node_style) = if fragment.node_id.is_none() {
                (translated, None)
            } else {
                let style = &document.node(fragment.node_id).style;
                let transform = style.transform;
                let origin_x = resolve_origin(&style.transform_origin.0, width);
                let origin_y = resolve_origin(&style.transform_origin.1, height);
                let transform = Affine {
                    a: transform.a,
                    b: transform.b,
                    c: transform.c,
                    d: transform.d,
                    e: transform.e,
                    f: transform.f,
                };
                (
                    translated
                        .then(Affine::translate(origin_x, origin_y))
                        .then(transform)
                        .then(Affine::translate(-origin_x, -origin_y)),
                    Some(style),
                )
            };
            let Some(world_to_local) = world.inverse() else {
                return;
            };
            let mut own_clips = inherited_clips.to_vec();
            if let Some(style) = node_style {
                if let Some(inset) = style.clip_path_inset {
                    let resolve = |length: &openui_geometry::Length, size: f32| {
                        if length.is_fixed() {
                            length.value()
                        } else if length.is_percent() {
                            length.value() * size / 100.0
                        } else {
                            0.0
                        }
                    };
                    let top = resolve(&inset[0], height);
                    let right = resolve(&inset[1], width);
                    let bottom = resolve(&inset[2], height);
                    let left = resolve(&inset[3], width);
                    let clipped_world = world.then(Affine::translate(left, top));
                    if let Some(inverse) = clipped_world.inverse() {
                        own_clips.push(HitClip {
                            world_to_local: inverse,
                            width: (width - left - right).max(0.0),
                            height: (height - top - bottom).max(0.0),
                            radii: [(0.0, 0.0); 4],
                        });
                    }
                }
                let node = document.node(fragment.node_id);
                let pointer_eligible = node.tag != ElementTag::Text
                    && style.visibility == openui_style::Visibility::Visible
                    && style.pointer_events == openui_style::PointerEvents::Auto;
                if pointer_eligible && width > 0.0 && height > 0.0 {
                    out.push(HitEntry {
                        node: fragment.node_id,
                        local_to_world: world,
                        world_to_local,
                        width,
                        height,
                        clips: own_clips.clone(),
                    });
                }
                if fragment.has_overflow_clip {
                    own_clips.push(HitClip {
                        world_to_local,
                        width,
                        height,
                        radii: if fragment.ignore_border_radius {
                            [(0.0, 0.0); 4]
                        } else {
                            [
                                style.border_top_left_radius,
                                style.border_top_right_radius,
                                style.border_bottom_right_radius,
                                style.border_bottom_left_radius,
                            ]
                        },
                    });
                }
            }
            let child_world = if fragment.node_id.is_none() {
                world
            } else {
                let node = document.node(fragment.node_id);
                world.then(Affine::translate(-node.scroll_left, -node.scroll_top))
            };
            let mut children: Vec<_> = fragment.children.iter().enumerate().collect();
            children.sort_by_key(|(order, child)| {
                let z = if child.node_id.is_none() {
                    0
                } else {
                    document.node(child.node_id).style.z_index.unwrap_or(0)
                };
                (z, *order)
            });
            for (_, child) in children {
                walk(document, child, child_world, &own_clips, out);
            }
        }
        let mut entries = Vec::new();
        walk(
            &self.document,
            fragment,
            Affine::IDENTITY,
            &[],
            &mut entries,
        );
        self.hit_test = entries;
    }
}

fn rounded_rect_contains(x: f32, y: f32, width: f32, height: f32, radii: [(f32, f32); 4]) -> bool {
    if x < 0.0 || y < 0.0 || x >= width || y >= height {
        return false;
    }
    let corners = [
        (radii[0], radii[0].0, radii[0].1),
        (radii[1], width - radii[1].0, radii[1].1),
        (radii[2], width - radii[2].0, height - radii[2].1),
        (radii[3], radii[3].0, height - radii[3].1),
    ];
    for (index, ((radius_x, radius_y), center_x, center_y)) in corners.into_iter().enumerate() {
        let in_corner = match index {
            0 => x < center_x && y < center_y,
            1 => x > center_x && y < center_y,
            2 => x > center_x && y > center_y,
            _ => x < center_x && y > center_y,
        };
        if in_corner && radius_x > 0.0 && radius_y > 0.0 {
            let dx = (x - center_x) / radius_x;
            let dy = (y - center_y) / radius_y;
            return dx * dx + dy * dy <= 1.0;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_compositor::SoftwareCompositor;
    use openui_style::{
        Color, CornerRadii, Display, Edges, LengthValue, Overflow, TransformList,
        TransformOperation,
    };

    #[test]
    fn stale_and_cross_document_handles_are_rejected() {
        let mut a = Engine::new(Viewport::new(100, 100).unwrap()).unwrap();
        let mut b = Engine::new(Viewport::new(100, 100).unwrap()).unwrap();
        let child = a.create_element(ElementTag::Div).unwrap();
        a.append_child(a.root(), child).unwrap();
        a.remove(child).unwrap();
        assert_eq!(a.set_text(child, "stale"), Err(EngineError::StaleHandle));
        assert_eq!(
            b.set_text(a.root(), "foreign"),
            Err(EngineError::WrongDocument)
        );
    }

    #[test]
    fn retained_tree_supports_safe_reordering() {
        let mut engine = Engine::new(Viewport::new(100, 100).unwrap()).unwrap();
        let root = engine.root();
        let a = engine.create_element(ElementTag::Div).unwrap();
        let b = engine.create_element(ElementTag::Div).unwrap();
        let c = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(root, a).unwrap();
        engine.append_child(root, b).unwrap();
        engine.append_child(root, c).unwrap();
        engine.insert_before(root, c, a).unwrap();
        assert_eq!(engine.children(root).unwrap(), vec![c, a, b]);
        engine.append_or_move_child(root, a).unwrap();
        assert_eq!(engine.children(root).unwrap(), vec![c, b, a]);
        assert_eq!(engine.parent(a).unwrap(), Some(root));
    }

    #[test]
    fn unchanged_scene_performs_no_lifecycle_work() {
        let mut engine = Engine::new(Viewport::new(64, 64).unwrap()).unwrap();
        let first = engine.scene().unwrap();
        let stats = engine.stats();
        let second = engine.scene().unwrap();
        assert_eq!(first.generation(), second.generation());
        assert_eq!(stats, engine.stats());
    }

    #[test]
    fn invalidation_runs_only_the_required_visual_stages() {
        let mut engine = Engine::new(Viewport::new(64, 64).unwrap()).unwrap();
        let div = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), div).unwrap();
        engine.scene().unwrap();

        let before_paint = engine.stats();
        engine
            .set_property(div, StyleProperty::BackgroundColor, Color::RED.into())
            .unwrap();
        engine.scene().unwrap();
        assert_eq!(engine.stats().layouts, before_paint.layouts);
        assert_eq!(engine.stats().paints, before_paint.paints + 1);

        let before_accessibility = engine.stats();
        engine.set_attribute(div, "aria-label", "sample").unwrap();
        let generation = engine.scene().unwrap().generation();
        assert_eq!(engine.stats(), before_accessibility);
        assert_eq!(generation.0, before_accessibility.scenes);
    }

    #[test]
    fn typed_tree_renders_through_scene_compositor() {
        let mut engine = Engine::new(Viewport::new(64, 64).unwrap()).unwrap();
        let div = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), div).unwrap();
        engine
            .set_property(div, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(div, StyleProperty::Width, LengthValue::px(32.0).into())
            .unwrap();
        engine
            .set_property(div, StyleProperty::Height, LengthValue::px(32.0).into())
            .unwrap();
        engine
            .set_property(div, StyleProperty::BackgroundColor, Color::RED.into())
            .unwrap();
        let scene = engine.scene().unwrap();
        let frame = SoftwareCompositor::default().render(&scene).unwrap();
        assert_eq!((frame.width, frame.height), (64, 64));
        assert!(frame.pixels.iter().any(|v| *v != 255));
    }

    #[test]
    fn hit_testing_respects_transforms_pointer_eligibility_and_rounded_clips() {
        let mut engine = Engine::new(Viewport::new(100, 100).unwrap()).unwrap();
        let root = engine.root();
        let transformed = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(root, transformed).unwrap();
        for (property, value) in [
            (StyleProperty::Display, Display::Block.into()),
            (StyleProperty::Width, LengthValue::px(20.0).into()),
            (StyleProperty::Height, LengthValue::px(20.0).into()),
        ] {
            engine.set_property(transformed, property, value).unwrap();
        }
        engine
            .set_property(
                transformed,
                StyleProperty::Transform,
                TransformList(vec![TransformOperation::Translate(
                    LengthValue::px(40.0),
                    LengthValue::px(0.0),
                )])
                .into(),
            )
            .unwrap();
        assert_eq!(engine.hit_test(45.0, 5.0).unwrap(), Some(transformed));

        engine
            .set_property(
                transformed,
                StyleProperty::PointerEvents,
                openui_style::PointerEvents::None.into(),
            )
            .unwrap();
        assert_ne!(engine.hit_test(45.0, 5.0).unwrap(), Some(transformed));

        let clip = engine.create_element(ElementTag::Div).unwrap();
        let child = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(root, clip).unwrap();
        engine.append_child(clip, child).unwrap();
        for node in [clip, child] {
            engine
                .set_property(node, StyleProperty::Display, Display::Block.into())
                .unwrap();
            engine
                .set_property(node, StyleProperty::Width, LengthValue::px(40.0).into())
                .unwrap();
            engine
                .set_property(node, StyleProperty::Height, LengthValue::px(40.0).into())
                .unwrap();
        }
        engine
            .set_property(clip, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        engine
            .set_property(
                clip,
                StyleProperty::BorderRadius,
                CornerRadii(Edges::all(LengthValue::px(20.0))).into(),
            )
            .unwrap();
        let bounds = engine.bounds(clip).unwrap().unwrap();
        assert_eq!(
            engine.hit_test(bounds.x + 1.0, bounds.y + 1.0).unwrap(),
            Some(clip)
        );
    }

    #[test]
    fn resources_are_document_owned() {
        let mut engine = Engine::new(Viewport::new(64, 64).unwrap()).unwrap();
        let image = engine.create_element(ElementTag::Image).unwrap();
        let resource = engine.register_image_resource(
            "memory:pixel",
            "image/png",
            "known-hash",
            Vec::<u8>::new(),
        );
        engine
            .set_image_resource(image, resource, Some((1.0, 1.0)))
            .unwrap();
        assert_eq!(
            engine.set_image_resource(image, ImageResourceId::new(99), None),
            Err(EngineError::UnknownResource)
        );
    }

    #[test]
    fn long_running_mutations_reuse_owned_storage() {
        let mut engine = Engine::new(Viewport::new(64, 64).unwrap()).unwrap();
        let root = engine.root();
        for iteration in 0..10_000 {
            let node = engine.create_element(ElementTag::Div).unwrap();
            engine.append_child(root, node).unwrap();
            engine.set_text(node, iteration.to_string()).unwrap();
            engine.remove(node).unwrap();
        }
        let counts = engine.object_counts();
        assert_eq!(counts.live_nodes, 1);
        assert_eq!(counts.handle_slots, 2);
        assert_eq!(counts.vacant_handle_slots, 1);
        assert_eq!(counts.arena_nodes, 2);
        assert_eq!(counts.reusable_arena_nodes, 1);
        assert_eq!(counts.controls, 0);
        assert_eq!(counts.semantic_overrides, 0);
        assert_eq!(counts.animations, 0);
        assert_eq!(counts.scroll_animations, 0);
    }
}
