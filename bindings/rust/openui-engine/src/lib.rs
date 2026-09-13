//! Retained, thread-affine Open UI document engine.

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
struct HitEntry {
    node: NodeId,
    rect: SceneRect,
}

#[derive(Debug, Clone, Copy, Default)]
struct DirtyState {
    layout: bool,
    paint: bool,
    compositing: bool,
    accessibility: bool,
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
    animation_time_ms: f64,
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
            },
            dirty_generations: DirtyGenerations::default(),
            latest_fragment: None,
            latest_scene: None,
            hit_test: Vec::new(),
            focused: None,
            pointer_capture: HashMap::new(),
            animation_time_ms: 0.0,
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
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(self.handle_for_slot(index))
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
        if self
            .focused
            .is_some_and(|focused| removed_handles.contains(&focused))
        {
            self.focused = None;
        }
        self.pointer_capture
            .retain(|_, captured| !removed_handles.contains(captured));
        for node in descendants {
            if let Some(index) = self.node_slots.remove(&node) {
                let slot = &mut self.slots[index as usize];
                slot.node = None;
                slot.authored.clear();
                slot.generation = slot.generation.wrapping_add(1).max(1);
                self.free_slots.push(index);
            }
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
        let name = name.into();
        let value = value.into();
        if self.document.node(node).attributes.get(&name) == Some(&value) {
            return Ok(());
        }
        self.document.set_attribute(node, name, value);
        self.mark_dirty(InvalidationClass::Accessibility);
        Ok(())
    }

    pub fn attribute(&self, handle: NodeHandle, name: &str) -> Result<Option<&str>, EngineError> {
        let node = self.resolve(handle)?;
        Ok(self
            .document
            .node(node)
            .attributes
            .get(name)
            .map(String::as_str))
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
            .remove(name)
            .is_some();
        if removed {
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
            .insert(property as u16, value);
        self.mark_dirty(property.metadata().invalidation);
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
        let data = self.document.node_mut(node);
        if (data.scroll_left, data.scroll_top) == (x, y) {
            return Ok(());
        }
        data.scroll_left = x;
        data.scroll_top = y;
        self.mark_dirty(InvalidationClass::Composite);
        Ok(())
    }

    pub fn focus(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        self.resolve(handle)?;
        if self.focused != Some(handle) {
            self.focused = Some(handle);
            self.mark_dirty(InvalidationClass::Accessibility);
        }
        Ok(())
    }

    pub fn blur(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        self.resolve(handle)?;
        if self.focused == Some(handle) {
            self.focused = None;
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
        Ok(())
    }

    pub fn animation_time(&self) -> f64 {
        self.animation_time_ms
    }

    pub fn update(&mut self) -> Result<&SceneSnapshot, EngineError> {
        if !self.dirty.visual() {
            self.dirty.accessibility = false;
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
        self.dirty = DirtyState::default();
        Ok(self.latest_scene.as_ref().expect("scene assigned"))
    }

    pub fn scene(&mut self) -> Result<SceneSnapshot, EngineError> {
        Ok(self.update()?.clone())
    }

    pub fn hit_test(&mut self, x: f32, y: f32) -> Result<Option<NodeHandle>, EngineError> {
        self.update()?;
        for entry in self.hit_test.iter().rev() {
            if x >= entry.rect.x
                && y >= entry.rect.y
                && x < entry.rect.x + entry.rect.width
                && y < entry.rect.y + entry.rect.height
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
            .map(|entry| entry.rect))
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
    }
    fn rebuild_hit_test(&mut self, fragment: &Fragment) {
        fn walk(fragment: &Fragment, x: f32, y: f32, out: &mut Vec<HitEntry>) {
            let x = x + fragment.offset.left.to_f32();
            let y = y + fragment.offset.top.to_f32();
            if !fragment.node_id.is_none() {
                out.push(HitEntry {
                    node: fragment.node_id,
                    rect: SceneRect {
                        x,
                        y,
                        width: fragment.size.width.to_f32(),
                        height: fragment.size.height.to_f32(),
                    },
                });
            }
            for child in &fragment.children {
                walk(child, x, y, out);
            }
        }
        let mut entries = Vec::new();
        walk(fragment, 0.0, 0.0, &mut entries);
        self.hit_test = entries;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_compositor::SoftwareCompositor;
    use openui_style::{Color, Display, LengthValue};

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
}
