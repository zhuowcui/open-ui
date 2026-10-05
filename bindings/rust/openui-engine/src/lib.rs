//! Retained, thread-affine Open UI document engine.

mod accessibility;
mod animation;
mod interaction;
mod scroll_into_view;

pub use scroll_into_view::{ScrollAlignment, ScrollIntoViewContainer, ScrollIntoViewOptions};

pub use accessibility::{
    AccessibilityAction, AccessibilityActionData, AccessibilityActionRequest, AccessibilityLive,
    AccessibilityNode, AccessibilityNodeId, AccessibilityPlatformAction, AccessibilityRelation,
    AccessibilityRole, AccessibilityToggled, AccessibilityTreeUpdate,
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
    Document as NativeDocument, ElementTag, FormControlRole, NodeId, PseudoElementKind,
    ReplacedContent, ReplacedResourceKind,
};
pub use openui_geometry::{
    RasterConfiguration, ViewportAuthority, ViewportMetrics, ViewportMetricsError,
};
use openui_layout::Fragment;
use openui_paint::record_fragment;
use openui_style::{
    apply_to_computed, Color, ComputedStyle, Display, ImageResourceId, InvalidationClass,
    PseudoStyleTarget, RendererInternalStyleValue, RendererStyleValue, Style, StyleProperty,
    StyleValue,
};
pub use openui_text::{
    FontAxisRange, FontCollection, FontCollectionError, FontCollectionStats, FontContainerFormat,
    FontFaceDescriptor, FontFaceHandle, FontFaceInfo, FontFeatureDefault, FontMetricOverrides,
    FontPaletteBase, FontPaletteEntryOverride, FontPaletteHandle, FontPaletteValuesDescriptor,
    FontStyleRange, FontUnicodeRange, HyphenationDictionaryHandle, HyphenationRegistry,
    HyphenationRegistryError,
};
use std::collections::{BTreeMap, HashMap};
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

static NEXT_DOCUMENT_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeHandle {
    document: u64,
    index: u32,
    generation: u32,
}

/// Owned, resolved dimensions of an element's scrolling area, in logical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ScrollMetrics {
    pub client_width: f64,
    pub client_height: f64,
    pub scroll_width: f64,
    pub scroll_height: f64,
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

/// Non-author DOM state used by deterministic renderer fixtures. Style
/// declarations are intentionally excluded and must use `set_property` or
/// `set_renderer_style`.
#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum RendererNodeState {
    Text(Option<String>),
    Replaced(Option<ReplacedContent>),
    TableColumnSpan(u32),
    TableRowSpan(u32),
    FormControl(Option<FormControlRole>),
    FormControlDisabled(bool),
    FormControlNativeAppearance(bool),
    EmbeddedDocument(Option<ImageResourceId>),
    EmbeddedCanvasColor(Option<Color>),
    ScrollLeft(f32),
    ScrollTop(f32),
    ScrollMarkerInactiveBackground(Option<Color>),
    SvgForeignObject(bool),
}

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
    Font(FontCollectionError),
    Hyphenation(HyphenationRegistryError),
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
            Self::Font(value) => write!(f, "font registration failed: {value}"),
            Self::Hyphenation(value) => write!(f, "hyphenation dictionary failed: {value}"),
        }
    }
}

impl std::error::Error for EngineError {}

impl From<ViewportMetricsError> for EngineError {
    fn from(_: ViewportMetricsError) -> Self {
        Self::InvalidViewport
    }
}

impl From<FontCollectionError> for EngineError {
    fn from(value: FontCollectionError) -> Self {
        Self::Font(value)
    }
}

impl From<HyphenationRegistryError> for EngineError {
    fn from(value: HyphenationRegistryError) -> Self {
        Self::Hyphenation(value)
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LifecycleStats {
    pub layouts: u64,
    pub paints: u64,
    pub scenes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EngineOptions {
    pub raster_configuration: RasterConfiguration,
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
    pub font_faces: usize,
    pub cached_font_instances: usize,
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
    authored_order: Vec<u16>,
    authored_pseudo: [Option<Style>; 4],
    resolved_style_snapshot: bool,
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

    fn map_rect(self, width: f32, height: f32) -> SceneRect {
        let corners = [
            self.map(0.0, 0.0),
            self.map(width, 0.0),
            self.map(0.0, height),
            self.map(width, height),
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
    viewport: ViewportMetrics,
    raster_configuration: RasterConfiguration,
    document: NativeDocument,
    slots: Vec<Slot>,
    free_slots: Vec<u32>,
    node_slots: HashMap<NodeId, u32>,
    dirty: DirtyState,
    dirty_generations: DirtyGenerations,
    latest_fragment: Option<Arc<Fragment>>,
    latest_scene: Option<SceneSnapshot>,
    hit_test: Vec<HitEntry>,
    fragment_rects: HashMap<NodeId, Vec<SceneRect>>,
    fragment_box_worlds: HashMap<NodeId, Affine>,
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
    pub fn new(viewport: ViewportMetrics) -> Result<Self, EngineError> {
        Self::new_with_options(viewport, EngineOptions::default())
    }

    pub fn new_with_options(
        viewport: ViewportMetrics,
        options: EngineOptions,
    ) -> Result<Self, EngineError> {
        Self::new_with_font_collection_and_options(viewport, FontCollection::system(), options)
    }

    pub fn new_with_font_collection(
        viewport: ViewportMetrics,
        font_collection: Arc<FontCollection>,
    ) -> Result<Self, EngineError> {
        Self::new_with_font_collection_and_options(
            viewport,
            font_collection,
            EngineOptions::default(),
        )
    }

    pub fn new_with_font_collection_and_options(
        viewport: ViewportMetrics,
        font_collection: Arc<FontCollection>,
        options: EngineOptions,
    ) -> Result<Self, EngineError> {
        let document = NativeDocument::new_with_raster_configuration(
            font_collection,
            options.raster_configuration,
            viewport.device_scale_factor(),
        );
        let root_node = document.root();
        let root_slot = Slot {
            generation: 1,
            node: Some(root_node),
            authored: BTreeMap::new(),
            authored_order: Vec::new(),
            authored_pseudo: Default::default(),
            resolved_style_snapshot: false,
        };
        Ok(Self {
            id: NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed),
            viewport,
            raster_configuration: options.raster_configuration,
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
            fragment_rects: HashMap::new(),
            fragment_box_worlds: HashMap::new(),
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
    pub fn viewport(&self) -> ViewportMetrics {
        self.viewport
    }
    pub fn raster_configuration(&self) -> RasterConfiguration {
        self.raster_configuration
    }
    pub fn font_collection(&self) -> &Arc<FontCollection> {
        self.document.font_collection()
    }
    pub fn hyphenation_registry(&self) -> &Arc<HyphenationRegistry> {
        self.document.font_collection().hyphenation_registry()
    }
    pub fn root(&self) -> NodeHandle {
        self.handle_for_slot(0)
    }
    pub fn stats(&self) -> LifecycleStats {
        self.stats
    }
    pub fn object_counts(&self) -> EngineObjectCounts {
        let font_stats = self.document.font_collection().stats();
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
            font_faces: font_stats.registered_faces,
            cached_font_instances: font_stats.cached_instances,
        }
    }

    pub fn register_font_face(
        &mut self,
        bytes: Arc<[u8]>,
        descriptor: FontFaceDescriptor,
    ) -> Result<FontFaceHandle, EngineError> {
        let handle = self
            .document
            .font_collection()
            .register(bytes, descriptor)?;
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(handle)
    }

    pub fn unregister_font_face(&mut self, handle: FontFaceHandle) -> Result<(), EngineError> {
        self.document.font_collection().unregister(handle)?;
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(())
    }

    pub fn font_face_info(&self, handle: FontFaceHandle) -> Result<FontFaceInfo, EngineError> {
        Ok(self.document.font_collection().query(handle)?)
    }
    pub fn register_font_palette_values(
        &mut self,
        descriptor: FontPaletteValuesDescriptor,
    ) -> Result<FontPaletteHandle, EngineError> {
        let handle = self
            .document
            .font_collection()
            .register_palette_values(descriptor)?;
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(handle)
    }
    pub fn unregister_font_palette_values(
        &mut self,
        handle: FontPaletteHandle,
    ) -> Result<(), EngineError> {
        self.document
            .font_collection()
            .unregister_palette_values(handle)?;
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(())
    }
    pub fn register_hyphenation_dictionary(
        &mut self,
        locale: &str,
        bytes: Arc<[u8]>,
    ) -> Result<HyphenationDictionaryHandle, EngineError> {
        let handle = self.hyphenation_registry().register(locale, bytes)?;
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(handle)
    }
    pub fn unregister_hyphenation_dictionary(
        &mut self,
        handle: HyphenationDictionaryHandle,
    ) -> Result<(), EngineError> {
        self.hyphenation_registry().unregister(handle)?;
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(())
    }
    pub fn dirty_generations(&self) -> DirtyGenerations {
        self.dirty_generations
    }
    pub fn native_document(&self) -> &NativeDocument {
        &self.document
    }

    pub fn set_viewport(&mut self, viewport: ViewportMetrics) -> Result<(), EngineError> {
        if self.viewport == viewport {
            return Ok(());
        }
        let logical_changed = self.viewport.logical_size() != viewport.logical_size();
        let scale_changed = self.viewport.device_scale_factor() != viewport.device_scale_factor();
        let physical_changed = self.viewport.physical_size() != viewport.physical_size();
        self.viewport = viewport;
        self.document
            .set_raster_context(self.raster_configuration, viewport.device_scale_factor());
        if logical_changed {
            self.recompute_authored_styles()?;
            self.mark_dirty(InvalidationClass::Intrinsic);
        }
        if scale_changed {
            // Until oracle evidence proves a narrower dependency, scale is a
            // font-metric, intrinsic, layout, paint, and raster invalidation.
            self.dirty.layout = true;
            self.dirty.paint = true;
            self.dirty.compositing = true;
            self.dirty.hit_test = true;
            self.dirty_generations.intrinsic += 1;
            self.dirty_generations.layout += 1;
            self.dirty_generations.paint += 1;
            self.dirty_generations.compositing += 1;
        } else if physical_changed && !logical_changed {
            self.dirty.paint = true;
            self.dirty.compositing = true;
            self.dirty_generations.paint += 1;
            self.dirty_generations.compositing += 1;
        }
        Ok(())
    }

    fn recompute_authored_styles(&mut self) -> Result<(), EngineError> {
        let roots: Vec<_> = self
            .slots
            .iter()
            .filter_map(|slot| slot.node)
            .filter(|node| self.document.node(*node).parent.is_none())
            .collect();
        for root in roots {
            self.refresh_inherited_styles(root)?;
        }
        Ok(())
    }

    // A closed <details> keeps only its first direct <summary> in the visual
    // tree. Apply this to the layout/paint view of the retained document, then
    // restore authored styles so native tree inspection and later mutations
    // still see the original elements and their display values.
    fn hide_closed_details_content(&mut self) -> Vec<(NodeId, Display)> {
        let details_nodes: Vec<_> = self
            .slots
            .iter()
            .filter_map(|slot| slot.node)
            .filter(|node| {
                let data = self.document.node(*node);
                data.tag == ElementTag::Details
                    && !data.parent.is_none()
                    && self.document.attribute(*node, "open").is_none()
            })
            .collect();
        let mut hidden = Vec::new();
        for details in details_nodes {
            let mut first_summary_seen = false;
            let children: Vec<_> = self.document.children(details).collect();
            for child in children {
                if self.document.node(child).tag == ElementTag::Summary && !first_summary_seen {
                    first_summary_seen = true;
                    continue;
                }
                let display = self.document.node(child).style.display;
                if display != Display::None {
                    hidden.push((child, display));
                    self.document
                        .update_resolved_style(child, |style| style.display = Display::None);
                }
            }
        }
        hidden
    }

    fn restore_closed_details_content(&mut self, hidden: Vec<(NodeId, Display)>) {
        for (node, display) in hidden {
            self.document
                .update_resolved_style(node, |style| style.display = display);
        }
    }

    pub fn transaction<T>(
        &mut self,
        operation: impl FnOnce(&mut Self) -> Result<T, EngineError>,
    ) -> Result<T, EngineError> {
        operation(self)
    }

    pub fn create_element(&mut self, tag: ElementTag) -> Result<NodeHandle, EngineError> {
        let node = self.document.create_node(tag);
        let handle = self.register_native_node(node);
        self.initialize_control(handle, tag);
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(handle)
    }

    /// Create an application element with the native kind's display default.
    /// Rust and C application bindings use this same constructor. The raw
    /// `create_element` entry point retains its initial-style behavior for
    /// Engine callers that supply a complete resolved style.
    pub fn create_native_element(&mut self, tag: ElementTag) -> Result<NodeHandle, EngineError> {
        let display = match tag {
            ElementTag::Div
            | ElementTag::Fieldset
            | ElementTag::Legend
            | ElementTag::Details
            | ElementTag::Summary
            | ElementTag::Option
            | ElementTag::OptGroup
            | ElementTag::Form
            | ElementTag::Html
            | ElementTag::Body
            | ElementTag::Viewport => Display::Block,
            ElementTag::Table => Display::Table,
            ElementTag::TableCaption => Display::TableCaption,
            ElementTag::TableColumnGroup => Display::TableColumnGroup,
            ElementTag::TableColumn => Display::TableColumn,
            ElementTag::TableHead => Display::TableHeaderGroup,
            ElementTag::TableBody => Display::TableRowGroup,
            ElementTag::TableFoot => Display::TableFooterGroup,
            ElementTag::TableRow => Display::TableRow,
            ElementTag::TableCell | ElementTag::TableHeaderCell => Display::TableCell,
            ElementTag::Image
            | ElementTag::Canvas
            | ElementTag::Svg
            | ElementTag::IFrame
            | ElementTag::Object
            | ElementTag::Audio
            | ElementTag::Video
            | ElementTag::Input
            | ElementTag::Button
            | ElementTag::Meter
            | ElementTag::Progress
            | ElementTag::TextArea
            | ElementTag::Select
            | ElementTag::Embed => Display::InlineBlock,
            ElementTag::Span
            | ElementTag::Text
            | ElementTag::Break
            | ElementTag::WordBreak
            | ElementTag::Ruby
            | ElementTag::RubyText
            | ElementTag::Style => Display::Inline,
        };
        let handle = self.create_element(tag)?;
        self.set_property(handle, StyleProperty::Display, display.into())?;
        Ok(handle)
    }

    /// Create a native SVG foreignObject viewport for retained UI children.
    /// Width and height describe the viewport, including the fragment bounds;
    /// border and padding do not enlarge it. Authored children, style, events,
    /// and lifecycle use the same engine as ordinary elements. Its native
    /// container kind is [`ElementTag::Div`].
    pub fn create_svg_foreign_object(&mut self) -> Result<NodeHandle, EngineError> {
        let handle = self.create_element(ElementTag::Div)?;
        let node = self.resolve(handle)?;
        let data = self.document.node_mut(node);
        data.is_svg_foreign_object = true;
        data.style.update_derived(|style| {
            style.display = Display::Block;
            style.overflow_x = openui_style::Overflow::Hidden;
            style.overflow_y = openui_style::Overflow::Hidden;
        });
        Ok(handle)
    }

    fn register_native_node(&mut self, node: NodeId) -> NodeHandle {
        let index = if let Some(index) = self.free_slots.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            slot.authored.clear();
            slot.authored_order.clear();
            slot.authored_pseudo = Default::default();
            slot.resolved_style_snapshot = false;
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 1,
                node: Some(node),
                authored: BTreeMap::new(),
                authored_order: Vec::new(),
                authored_pseudo: Default::default(),
                resolved_style_snapshot: false,
            });
            index
        };
        self.node_slots.insert(node, index);
        self.handle_for_slot(index)
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
        self.refresh_inherited_styles(child)?;
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
        self.refresh_inherited_styles(child_node)?;
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
        self.refresh_inherited_styles(child_node)?;
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

    /// Duplicate an authored node and its descendants as a detached tree.
    /// Runtime focus, scroll position, animations, and generated pseudo nodes
    /// belong to the original presentation and are not copied.
    pub fn clone_subtree(&mut self, source: NodeHandle) -> Result<NodeHandle, EngineError> {
        let source_node = self.resolve(source)?;
        if self.document.node(source_node).pseudo_kind.is_some() {
            return Err(EngineError::InvalidInput("cannot clone a generated node"));
        }
        let mut pending = vec![(source, None)];
        let mut root = None;
        let mut clones = HashMap::new();
        while let Some((original, parent)) = pending.pop() {
            let node = self.resolve(original)?;
            let mut data = self.document.node(node).clone();
            if data.pseudo_kind.is_some() {
                continue;
            }
            let children = self.children(original)?;
            let authored = self.slots[original.index as usize].authored.clone();
            let control = self.controls.get(&original.index).cloned();
            let semantics = self.semantics.get(&original.index).cloned();
            let duplicate = self.create_element(data.tag)?;
            clones.insert(original, duplicate);
            let duplicate_node = self.resolve(duplicate)?;
            data.parent = NodeId::NONE;
            data.first_child = NodeId::NONE;
            data.last_child = NodeId::NONE;
            data.next_sibling = NodeId::NONE;
            data.prev_sibling = NodeId::NONE;
            data.pseudo_origin = NodeId::NONE;
            data.scroll_left = 0.0;
            data.scroll_top = 0.0;
            data.attributes.remove("data-oui-focused");
            data.attributes.remove("data-oui-composition-start");
            data.attributes.remove("data-oui-composition-end");
            *self.document.node_mut(duplicate_node) = data;
            self.slots[duplicate.index as usize].authored = authored;
            self.slots[duplicate.index as usize].authored_order =
                self.slots[original.index as usize].authored_order.clone();
            self.slots[duplicate.index as usize].authored_pseudo =
                self.slots[original.index as usize].authored_pseudo.clone();
            self.slots[duplicate.index as usize].resolved_style_snapshot =
                self.slots[original.index as usize].resolved_style_snapshot;
            if let Some(mut control) = control {
                control.clear_composition();
                self.controls.insert(duplicate.index, control);
            }
            if let Some(semantics) = semantics {
                self.semantics.insert(duplicate.index, semantics);
            }
            if let Some(parent) = parent {
                self.append_child(parent, duplicate)?;
            } else {
                root = Some(duplicate);
            }
            pending.extend(
                children
                    .into_iter()
                    .rev()
                    .map(|child| (child, Some(duplicate))),
            );
        }
        for duplicate in clones.values() {
            if let Some(semantics) = self.semantics.get_mut(&duplicate.index) {
                semantics.remap_cloned_relations(&clones);
            }
        }
        let root = root.expect("validated source produces a root clone");
        self.refresh_inherited_styles(self.resolve(root)?)?;
        Ok(root)
    }

    /// Find the first attached element with this ID in document order.
    pub fn element_by_id(&self, id: &str) -> Option<NodeHandle> {
        let mut stack = vec![self.document.root()];
        while let Some(node) = stack.pop() {
            let data = self.document.node(node);
            if data.attributes.get("id").is_some_and(|value| value == id) {
                if let Some(index) = self.node_slots.get(&node) {
                    return Some(self.handle_for_slot(*index));
                }
            }
            let children: Vec<_> = self.document.children(node).collect();
            stack.extend(children.into_iter().rev());
        }
        None
    }

    pub fn remove_children(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        let children = self.children(handle)?;
        for child in children {
            self.remove(child)?;
        }
        Ok(())
    }

    /// Detach an authored subtree without invalidating its handles or losing
    /// its state. The caller may edit and reattach it later.
    pub fn detach(&mut self, handle: NodeHandle) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        if node == self.document.root() {
            return Err(EngineError::RootRemoval);
        }
        if self.document.node(node).parent.is_none() {
            return Ok(());
        }
        let mut descendants = Vec::new();
        self.collect_subtree(node, &mut descendants);
        self.document.detach(node);
        let detached_handles: Vec<_> = descendants
            .iter()
            .filter_map(|node| self.node_slots.get(node))
            .map(|index| self.handle_for_slot(*index))
            .collect();
        self.clear_subtree_presentation_state(&detached_handles);
        self.refresh_inherited_styles(node)?;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    fn clear_subtree_presentation_state(&mut self, handles: &[NodeHandle]) {
        self.cancel_animations_for_handles(handles);
        if self
            .focused
            .is_some_and(|focused| handles.contains(&focused))
        {
            self.focused = None;
        }
        self.pointer_capture
            .retain(|_, captured| !handles.contains(captured));
        self.active_pointers
            .retain(|_, active| !handles.contains(active));
        self.hover_paths.retain(|_, path| {
            path.retain(|node| !handles.contains(node));
            !path.is_empty()
        });
        if self
            .modal_root
            .is_some_and(|modal| handles.contains(&modal))
        {
            self.modal_root = None;
        }
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
        self.clear_subtree_presentation_state(&removed_handles);
        for node in &descendants {
            if let Some(index) = self.node_slots.remove(node) {
                self.controls.remove(&index);
                self.semantics.remove(&index);
                let slot = &mut self.slots[index as usize];
                slot.node = None;
                slot.authored.clear();
                slot.authored_order.clear();
                slot.authored_pseudo = Default::default();
                slot.resolved_style_snapshot = false;
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

    /// Update a node's own text data. Native element text replacement uses
    /// `set_text_content` so its text participates in ordinary child layout.
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

    /// Replace native element children with one authored text node.
    ///
    /// Rust and C element setters use this operation. Replaced child handles
    /// become stale, as with `remove_children`; this does not detach and retain
    /// an old subtree. A text-node handle instead updates its existing data.
    /// Empty element text retains an empty text child, preserving the native
    /// Rust setter's existing behavior.
    pub fn set_text_content(
        &mut self,
        handle: NodeHandle,
        text: impl Into<String>,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        if self.document.node(node).tag == ElementTag::Text {
            return self.set_text(handle, text);
        }
        self.remove_children(handle)?;
        // Container-local text data is not laid out as an authored text child.
        // Remove any data previously supplied through the low-level operation.
        self.document.node_mut(node).text = None;
        let child = self.create_text(text)?;
        self.append_child(handle, child)
    }

    /// Return the data of an authored text node.
    pub fn text_data(&self, handle: NodeHandle) -> Result<&str, EngineError> {
        let node = self.document.node(self.resolve(handle)?);
        if node.tag != ElementTag::Text || node.pseudo_kind.is_some() {
            return Err(EngineError::InvalidInput("not an authored text node"));
        }
        Ok(node.text.as_deref().unwrap_or(""))
    }

    /// Concatenate authored text in tree order, excluding generated content.
    pub fn text_content(&self, handle: NodeHandle) -> Result<String, EngineError> {
        let mut content = String::new();
        let mut pending = vec![handle];
        while let Some(current) = pending.pop() {
            let node = self.document.node(self.resolve(current)?);
            if node.pseudo_kind.is_some() {
                continue;
            }
            if let Some(text) = node.text.as_deref() {
                content.push_str(text);
            }
            pending.extend(self.children(current)?.into_iter().rev());
        }
        Ok(content)
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
        if name == "lang" {
            self.set_property(
                handle,
                StyleProperty::Locale,
                StyleValue::Renderer(RendererStyleValue::Locale(Some(value.clone()))),
            )?;
        }
        self.sync_control_attribute(handle, &name, &value);
        self.mark_dirty(
            if name == "open" && self.document.node(node).tag == ElementTag::Details {
                InvalidationClass::Subtree
            } else {
                InvalidationClass::Accessibility
            },
        );
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
            if name.eq_ignore_ascii_case("lang") {
                self.set_property(
                    handle,
                    StyleProperty::Locale,
                    StyleValue::Renderer(RendererStyleValue::Locale(None)),
                )?;
            }
            self.remove_control_attribute(handle, &name.to_ascii_lowercase());
            self.mark_dirty(
                if name.eq_ignore_ascii_case("open")
                    && self.document.node(node).tag == ElementTag::Details
                {
                    InvalidationClass::Subtree
                } else {
                    InvalidationClass::Accessibility
                },
            );
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
        let slot = &self.slots[handle.index as usize];
        if slot.authored.get(&(property as u16)) == Some(&value)
            && slot
                .authored_order
                .iter()
                .rev()
                .take_while(|id| **id != property as u16)
                .all(|id| {
                    !StyleProperty::from_u16(*id)
                        .expect("authored property")
                        .affects_same_fields_as(property)
                })
        {
            return Ok(());
        }
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        let parent = self.document.node(node).parent;
        let font = if property == StyleProperty::FontSize {
            if parent.is_none() {
                ComputedStyle::initial().font_size
            } else {
                self.document.node(parent).style.font_size
            }
        } else {
            self.document.node(node).style.font_size
        };
        let root_font = if node == self.document.root() && property == StyleProperty::FontSize {
            ComputedStyle::initial().font_size
        } else {
            self.document.node(self.document.root()).style.font_size
        };
        let resolved = Self::resolve_native_lengths(property, &value, font, root_font, viewport);
        self.document
            .apply_style_property(node, property, &resolved, viewport)
            .map_err(|_| EngineError::PropertyType { property })?;
        self.slots[handle.index as usize]
            .authored
            .insert(property as u16, value.clone());
        let order = &mut self.slots[handle.index as usize].authored_order;
        order.retain(|id| *id != property as u16);
        order.push(property as u16);
        if property.metadata().inherited {
            self.refresh_inherited_styles(node)?;
        }
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

    /// Apply a schema-generated computed longhand to a resolved snapshot.
    /// Omitted fields retain their computed defaults; they must not inherit
    /// again. This input freezes the current author declarations into computed
    /// state. Native applications use `set_property` or framework setters to
    /// retain declarations that respond to parent and viewport changes.
    #[doc(hidden)]
    pub fn set_renderer_style(
        &mut self,
        handle: NodeHandle,
        value: RendererStyleValue,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let property = value.property();
        let value = StyleValue::Renderer(value);
        let slot = &self.slots[handle.index as usize];
        if slot.resolved_style_snapshot
            && slot.authored.is_empty()
            && slot.authored_pseudo.iter().all(Option::is_none)
            && !self
                .animations
                .values()
                .any(|animation| animation.target == handle)
            && openui_style::value_from_computed(&self.document.node(node).style, property) == value
        {
            return Ok(());
        }
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        self.document
            .apply_style_property(node, property, &value, viewport)
            .map_err(|_| EngineError::PropertyType { property })?;
        let slot = &mut self.slots[handle.index as usize];
        slot.authored.clear();
        slot.authored_order.clear();
        slot.resolved_style_snapshot = true;
        self.refresh_inherited_styles(node)?;
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

    /// Apply engine-owned derived style state. These values are explicitly
    /// excluded from the author-property inventory.
    #[doc(hidden)]
    pub fn set_internal_style(
        &mut self,
        handle: NodeHandle,
        value: RendererInternalStyleValue,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        self.document.apply_internal_style(node, value);
        self.dirty.hit_test = true;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    /// Install a detached style produced for an anonymous or pseudo box.
    #[doc(hidden)]
    pub fn install_derived_style(
        &mut self,
        handle: NodeHandle,
        style: ComputedStyle,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        self.document.install_resolved_style(node, style);
        let slot = &mut self.slots[handle.index as usize];
        slot.authored.clear();
        slot.authored_order.clear();
        slot.authored_pseudo = std::array::from_fn(|_| None);
        slot.resolved_style_snapshot = true;
        self.refresh_inherited_descendants(node)?;
        self.dirty.hit_test = true;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    /// Resolve inheritance before owned queries, layout and paint. Anonymous
    /// and pseudo snapshots are already resolved and retain their values.
    fn refresh_inherited_styles(&mut self, root: NodeId) -> Result<(), EngineError> {
        let initial = ComputedStyle::initial();
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if let Some(index) = self.node_slots.get(&node).copied() {
                let slot = &self.slots[index as usize];
                if !slot.resolved_style_snapshot
                    || !slot.authored.is_empty()
                    || self.animations.values().any(|animation| {
                        animation.target.index == index && animation.last_applied.is_some()
                    })
                {
                    let declarations: Vec<_> = slot
                        .authored_order
                        .iter()
                        .map(|id| {
                            (
                                StyleProperty::from_u16(*id).expect("authored property"),
                                slot.authored[id].clone(),
                            )
                        })
                        .collect();
                    let mut animated: Vec<_> = self
                        .animations
                        .iter()
                        .filter_map(|(id, animation)| {
                            (animation.target.index == index)
                                .then(|| {
                                    animation.last_applied.as_ref().map(|value| {
                                        (*id, animation.keyframes.property(), value.clone())
                                    })
                                })
                                .flatten()
                        })
                        .collect();
                    animated.sort_by_key(|(id, _, _)| *id);
                    let mut declarations = declarations;
                    declarations.extend(
                        animated
                            .into_iter()
                            .map(|(_, property, value)| (property, value)),
                    );
                    let parent_node = self.document.node(node).parent;
                    let parent = if parent_node.is_none() {
                        initial.clone()
                    } else {
                        self.document.node(parent_node).style.clone()
                    };
                    let root_font = if node == self.document.root() {
                        initial.font_size
                    } else {
                        self.document.node(self.document.root()).style.font_size
                    };
                    let mut style = self.document.node(node).style.clone();
                    if !slot.resolved_style_snapshot {
                        style.inherit_properties_from(&parent);
                    }
                    let style = Self::resolve_native_declarations(
                        style,
                        &declarations,
                        parent.font_size,
                        root_font,
                        viewport,
                        node == self.document.root(),
                    )?;
                    self.document.install_resolved_style(node, style);
                }
                self.refresh_authored_pseudo_styles(node, index)?;
            }
            let children: Vec<_> = self.document.children(node).collect();
            pending.extend(children.into_iter().rev());
        }
        Ok(())
    }

    /// Compute native declarations once for ordinary and pseudo styles.
    fn resolve_native_declarations(
        mut style: ComputedStyle,
        declarations: &[(StyleProperty, StyleValue)],
        parent_font: f32,
        root_font: f32,
        viewport: (f32, f32),
        is_root: bool,
    ) -> Result<ComputedStyle, EngineError> {
        // Font size is computed against the parent, and all other
        // em lengths use the final cascaded size of this element.
        let mut font_style = style.clone();
        for (property, value) in declarations {
            if matches!(property, StyleProperty::Font | StyleProperty::FontSize) {
                let value = Self::resolve_native_lengths(
                    *property,
                    value,
                    parent_font,
                    root_font,
                    viewport,
                );
                apply_to_computed(&mut font_style, *property, &value, viewport).map_err(|_| {
                    EngineError::PropertyType {
                        property: *property,
                    }
                })?;
            }
        }
        let font_size = font_style.font_size;
        style.update_derived(|fields| fields.font_size = font_size);
        let mut authored_line_height = false;
        for (property, value) in declarations {
            let value = Self::resolve_native_lengths(
                *property,
                value,
                if *property == StyleProperty::FontSize {
                    parent_font
                } else {
                    font_size
                },
                if is_root && *property != StyleProperty::FontSize {
                    font_size
                } else {
                    root_font
                },
                viewport,
            );
            apply_to_computed(&mut style, *property, &value, viewport).map_err(|_| {
                EngineError::PropertyType {
                    property: *property,
                }
            })?;
            style.update_derived(|fields| fields.font_size = font_size);
            if matches!(property, StyleProperty::Font | StyleProperty::LineHeight) {
                authored_line_height = true;
            }
        }
        if authored_line_height {
            style.update_derived(|fields| {
                if let openui_style::LineHeight::Percentage(percent) = fields.line_height {
                    fields.line_height =
                        openui_style::LineHeight::Length(percent * font_size / 100.0);
                }
            });
        }
        Ok(style)
    }

    fn refresh_authored_pseudo_styles(
        &mut self,
        node: NodeId,
        index: u32,
    ) -> Result<(), EngineError> {
        let authored = self.slots[index as usize].authored_pseudo.clone();
        if authored.iter().all(Option::is_none) {
            return Ok(());
        }
        let mut origin = self.document.node(node).style.clone();
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        let root_font = self.document.node(self.document.root()).style.font_size;
        for (target, declarations) in authored.iter().enumerate() {
            if let Some(declarations) = declarations {
                let mut pseudo = ComputedStyle::for_pseudo(&origin);
                pseudo.inherit_properties_from(&origin);
                let declarations: Vec<_> = declarations
                    .declarations()
                    .iter()
                    .map(|declaration| (declaration.property, declaration.value.clone()))
                    .collect();
                let pseudo = Self::resolve_native_declarations(
                    pseudo,
                    &declarations,
                    origin.font_size,
                    root_font,
                    viewport,
                    false,
                )?;
                origin.update_derived(|fields| {
                    let destination = match target {
                        0 => &mut fields.first_line_style,
                        1 => &mut fields.first_letter_style,
                        2 => &mut fields.marker_style,
                        3 => &mut fields.placeholder_style,
                        _ => unreachable!("native pseudo target"),
                    };
                    *destination = Some(Box::new(pseudo));
                });
            }
        }
        self.document.install_resolved_style(node, origin);
        Ok(())
    }

    fn refresh_inherited_descendants(&mut self, parent: NodeId) -> Result<(), EngineError> {
        let children: Vec<_> = self.document.children(parent).collect();
        for child in children {
            self.refresh_inherited_styles(child)?;
        }
        Ok(())
    }

    fn resolve_native_lengths(
        property: StyleProperty,
        value: &StyleValue,
        font: f32,
        root_font: f32,
        viewport: (f32, f32),
    ) -> StyleValue {
        use openui_style::{Edges, Gap, LengthValue};
        let resolve = |value: LengthValue| {
            let length = value.resolve(viewport, font, root_font);
            LengthValue::Computed(
                if property == StyleProperty::FontSize
                    && (length.is_percent() || length.is_calculated())
                {
                    openui_geometry::Length::px(
                        (font * length.value() / 100.0 + length.calc_offset()).max(0.0),
                    )
                } else {
                    length
                },
            )
        };
        match value {
            StyleValue::Length(value) => StyleValue::Length(resolve(*value)),
            StyleValue::Edges(value) => StyleValue::Edges(Edges {
                top: resolve(value.top),
                right: resolve(value.right),
                bottom: resolve(value.bottom),
                left: resolve(value.left),
            }),
            StyleValue::Gap(value) => StyleValue::Gap(Gap {
                row: resolve(value.row),
                column: resolve(value.column),
            }),
            _ => value.clone(),
        }
    }

    /// Mutate validated non-style state needed by static renderer fixtures.
    #[doc(hidden)]
    pub fn set_renderer_node_state(
        &mut self,
        handle: NodeHandle,
        state: RendererNodeState,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        match state {
            RendererNodeState::Text(value) => self.document.node_mut(node).text = value,
            RendererNodeState::Replaced(value) => {
                if let Some(content) = value {
                    let resource = match content.resource {
                        ReplacedResourceKind::Image(id)
                        | ReplacedResourceKind::StaticSvg(id)
                        | ReplacedResourceKind::MediaPoster(id)
                        | ReplacedResourceKind::PackagedDocument(id) => Some(id),
                        ReplacedResourceKind::TransparentCanvas => None,
                    };
                    if resource.is_some_and(|id| self.document.image_resource(id).is_none()) {
                        return Err(EngineError::UnknownResource);
                    }
                }
                self.document.node_mut(node).replaced = value;
            }
            RendererNodeState::TableColumnSpan(value) => {
                if value == 0 {
                    return Err(EngineError::Render(
                        "table column span must be positive".into(),
                    ));
                }
                self.document.node_mut(node).table_col_span = value;
            }
            RendererNodeState::TableRowSpan(value) => {
                self.document.node_mut(node).table_row_span = value;
            }
            RendererNodeState::FormControl(value) => {
                self.document.node_mut(node).form_control = value;
            }
            RendererNodeState::FormControlDisabled(value) => {
                self.document.node_mut(node).form_control_disabled = value;
            }
            RendererNodeState::FormControlNativeAppearance(value) => {
                self.document.node_mut(node).form_control_native_appearance = value;
            }
            RendererNodeState::EmbeddedDocument(value) => {
                if value.is_some_and(|id| self.document.image_resource(id).is_none()) {
                    return Err(EngineError::UnknownResource);
                }
                self.document.node_mut(node).embedded_document = value;
            }
            RendererNodeState::EmbeddedCanvasColor(value) => {
                self.document.node_mut(node).embedded_canvas_color = value;
            }
            RendererNodeState::ScrollLeft(value) => {
                if !value.is_finite() {
                    return Err(EngineError::Render("scroll offset must be finite".into()));
                }
                self.document.node_mut(node).scroll_left = value;
            }
            RendererNodeState::ScrollTop(value) => {
                if !value.is_finite() {
                    return Err(EngineError::Render("scroll offset must be finite".into()));
                }
                self.document.node_mut(node).scroll_top = value;
            }
            RendererNodeState::ScrollMarkerInactiveBackground(value) => {
                self.document
                    .node_mut(node)
                    .scroll_marker_inactive_background = value;
            }
            RendererNodeState::SvgForeignObject(value) => {
                self.document.node_mut(node).is_svg_foreign_object = value;
            }
        }
        self.dirty.hit_test = true;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    #[doc(hidden)]
    pub fn insert_renderer_pseudo(
        &mut self,
        origin: NodeHandle,
        kind: PseudoElementKind,
    ) -> Result<NodeHandle, EngineError> {
        let origin = self.resolve(origin)?;
        let node = self.document.insert_pseudo_element(origin, kind);
        let handle = self.register_native_node(node);
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(handle)
    }

    #[doc(hidden)]
    pub fn materialize_generated_content(&mut self) -> Result<(), EngineError> {
        let first_new = self.document.node_count();
        self.document.materialize_generated_content();
        for index in first_new..self.document.node_count() {
            let node = self
                .document
                .node_id_at(index)
                .ok_or_else(|| EngineError::Render("generated node disappeared".into()))?;
            self.register_native_node(node);
        }
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    #[doc(hidden)]
    pub fn set_legacy_canvas_body(&mut self, body: NodeHandle) -> Result<(), EngineError> {
        let body = self.resolve(body)?;
        self.document.set_legacy_canvas_body(body);
        self.mark_dirty(InvalidationClass::Paint);
        Ok(())
    }

    /// Atomically replace the declarations for a supported pseudo-element.
    pub fn set_pseudo_style(
        &mut self,
        handle: NodeHandle,
        target: PseudoStyleTarget,
        declarations: &Style,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let origin = self.document.node(node).style.clone();
        let mut pseudo = ComputedStyle::for_pseudo(&origin);
        pseudo.inherit_properties_from(&origin);
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        let declarations_to_resolve: Vec<_> = declarations
            .declarations()
            .iter()
            .map(|declaration| (declaration.property, declaration.value.clone()))
            .collect();
        let pseudo = Self::resolve_native_declarations(
            pseudo,
            &declarations_to_resolve,
            origin.font_size,
            self.document.node(self.document.root()).style.font_size,
            viewport,
            false,
        )?;
        let resolved = origin.derive(|style| {
            let destination = match target {
                PseudoStyleTarget::FirstLine => &mut style.first_line_style,
                PseudoStyleTarget::FirstLetter => &mut style.first_letter_style,
                PseudoStyleTarget::Marker => &mut style.marker_style,
                PseudoStyleTarget::Placeholder => &mut style.placeholder_style,
            };
            *destination = Some(Box::new(pseudo));
        });
        self.slots[handle.index as usize].authored_pseudo[target as usize] =
            Some(declarations.clone());
        self.document.install_resolved_style(node, resolved);
        self.dirty.hit_test = true;
        self.mark_dirty(InvalidationClass::Subtree);
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

    /// Whether this handle identifies an authored element rather than text or
    /// a generated pseudo-element.
    pub fn is_authored_element(&self, handle: NodeHandle) -> Result<bool, EngineError> {
        let node = self.document.node(self.resolve(handle)?);
        Ok(node.tag != ElementTag::Text && node.pseudo_kind.is_none())
    }

    pub fn scroll_offset(&self, handle: NodeHandle) -> Result<(f64, f64), EngineError> {
        let node = self.resolve(handle)?;
        let node = self.document.node(node);
        Ok((node.scroll_left as f64, node.scroll_top as f64))
    }

    /// Resolve pending layout and return an owned scroll-area snapshot.
    /// Elements without a layout box have no metrics.
    pub fn scroll_metrics(
        &mut self,
        handle: NodeHandle,
    ) -> Result<Option<ScrollMetrics>, EngineError> {
        let node = self.resolve(handle)?;
        if !self.node_is_connected(node) {
            return Ok(None);
        }
        self.update()?;
        let Some(area) = self.layout_scroll_area(node) else {
            return Ok(None);
        };
        let client = area.client_rect.size;
        let content = area.content_rect.size;
        Ok(Some(ScrollMetrics {
            client_width: client.width.to_f64(),
            client_height: client.height.to_f64(),
            scroll_width: content.width.to_f64(),
            scroll_height: content.height.to_f64(),
        }))
    }

    pub fn scroll_to(&mut self, handle: NodeHandle, x: f64, y: f64) -> Result<(), EngineError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::Render("scroll offsets must be finite".into()));
        }
        let node = self.resolve(handle)?;
        let (x, y) = self.clamp_scroll_offset(handle, x, y)?;
        self.scroll_animations
            .retain(|_, animation| animation.target != handle);
        if (
            self.document.node(node).scroll_left,
            self.document.node(node).scroll_top,
        ) == (x, y)
        {
            return Ok(());
        }
        let data = self.document.node_mut(node);
        data.scroll_left = x;
        data.scroll_top = y;
        self.invalidate_scroll(node);
        Ok(())
    }

    fn invalidate_scroll(&mut self, node: NodeId) {
        // Sticky fragment positions currently resolve during layout. Invalidate
        // that dependency only when this scrollport owns a sticky descendant;
        // ordinary scrolling retains its existing compositor-only path.
        let mut pending = self.document.children(node).collect::<Vec<_>>();
        let mut sticky = false;
        while let Some(descendant) = pending.pop() {
            let style = &self.document.node(descendant).style;
            if style.display == Display::None {
                continue;
            }
            if style.position == openui_style::Position::Sticky {
                sticky = true;
                break;
            }
            if !style.is_scroll_container() {
                pending.extend(self.document.children(descendant));
            }
        }
        self.dirty.hit_test = true;
        self.mark_dirty(if sticky {
            InvalidationClass::Layout
        } else {
            InvalidationClass::Composite
        });
    }

    pub fn scroll_by(&mut self, handle: NodeHandle, dx: f64, dy: f64) -> Result<(), EngineError> {
        if !dx.is_finite() || !dy.is_finite() {
            return Err(EngineError::InvalidInput("scroll deltas must be finite"));
        }
        self.resolve(handle)?;
        self.update()?;
        let (x, y) = self.scroll_offset(handle)?;
        self.scroll_to(handle, x + dx, y + dy)
    }

    fn clamp_scroll_offset(
        &mut self,
        handle: NodeHandle,
        x: f64,
        y: f64,
    ) -> Result<(f32, f32), EngineError> {
        let node = self.resolve(handle)?;
        self.update()?;
        Ok(self
            .layout_scroll_area(node)
            .map_or((0.0, 0.0), |area| area.clamp_offset(x, y)))
    }

    fn layout_scroll_area(&self, node: NodeId) -> Option<openui_layout::ScrollArea> {
        fn find(fragment: &Fragment, node: NodeId) -> Option<openui_layout::ScrollArea> {
            if fragment.node_id == node {
                if let Some(area) = fragment.scroll_area {
                    return Some(area);
                }
            }
            fragment.children.iter().find_map(|child| find(child, node))
        }
        self.latest_fragment
            .as_ref()
            .and_then(|root| find(root, node))
    }

    /// Apply user wheel input through the document-owned viewport geometry.
    /// A hidden viewport remains available to programmatic scrolling only.
    pub fn scroll_wheel(
        &mut self,
        handle: NodeHandle,
        delta_x: f64,
        delta_y: f64,
    ) -> Result<bool, EngineError> {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return Err(EngineError::InvalidInput("wheel deltas must be finite"));
        }
        let node = self.resolve(handle)?;
        self.update()?;
        let (user_x, user_y) = self
            .layout_scroll_area(node)
            .map_or((false, false), |area| {
                (
                    area.overflow_x.is_scrollable(),
                    area.overflow_y.is_scrollable(),
                )
            });
        let before = self.scroll_offset(handle)?;
        self.scroll_to(
            handle,
            before.0 + if user_x { delta_x } else { 0.0 },
            before.1 + if user_y { delta_y } else { 0.0 },
        )?;
        Ok(self.scroll_offset(handle)? != before)
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
        let hidden_details_content = self.hide_closed_details_content();
        let width = self.viewport.logical_width();
        let height = self.viewport.logical_height();
        if self.dirty.layout || self.latest_fragment.is_none() {
            let root_style = &self.document.node(self.document.root()).style;
            let direction = root_style
                .direction
                .writing_direction(root_style.writing_mode);
            let space = openui_layout::ConstraintSpace::for_root_with_writing_direction(
                openui_geometry::LayoutUnit::from_f64(width),
                openui_geometry::LayoutUnit::from_f64(height),
                direction,
            );
            let mut fragment =
                openui_layout::block_layout(&self.document, self.document.root(), &space);
            fn collect_clamped_offsets(
                fragment: &Fragment,
                document: &NativeDocument,
                seen: &mut std::collections::HashSet<NodeId>,
                changes: &mut Vec<(NodeId, f32, f32)>,
            ) {
                if let Some(area) = fragment.scroll_area {
                    if seen.insert(fragment.node_id) {
                        let node = document.node(fragment.node_id);
                        let (x, y) =
                            area.clamp_offset(node.scroll_left as f64, node.scroll_top as f64);
                        if (node.scroll_left, node.scroll_top) != (x, y) {
                            changes.push((fragment.node_id, x, y));
                        }
                    }
                }
                for child in &fragment.children {
                    collect_clamped_offsets(child, document, seen, changes);
                }
            }
            let mut changes = Vec::new();
            collect_clamped_offsets(
                &fragment,
                &self.document,
                &mut std::collections::HashSet::new(),
                &mut changes,
            );
            if !changes.is_empty() {
                for (node, x, y) in changes {
                    let node = self.document.node_mut(node);
                    node.scroll_left = x;
                    node.scroll_top = y;
                }
                // Sticky descendants also consume the retained offsets.
                fragment =
                    openui_layout::block_layout(&self.document, self.document.root(), &space);
            }
            let fragment = Arc::new(fragment);
            self.stats.layouts += 1;
            self.rebuild_hit_test(&fragment);
            self.latest_fragment = Some(fragment);
        }
        let fragment = self
            .latest_fragment
            .as_ref()
            .expect("visual dirtiness always establishes a fragment")
            .clone();
        let recording = record_fragment(&self.document, &fragment, self.viewport);
        self.restore_closed_details_content(hidden_details_content);
        let recording = recording.map_err(EngineError::Render)?;
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

    /// Owned border-box rectangles in layout order, in logical viewport
    /// coordinates after scrolling and transforms. Input eligibility,
    /// visibility, and clipping do not remove layout boxes from this query.
    pub fn client_rects(&mut self, handle: NodeHandle) -> Result<Vec<SceneRect>, EngineError> {
        let node = self.resolve(handle)?;
        if !self.node_is_connected(node) {
            return Ok(Vec::new());
        }
        self.update()?;
        Ok(self.fragment_rects.get(&node).cloned().unwrap_or_default())
    }

    /// Bounds of all nonempty border-box fragments. If every fragment is
    /// empty, return the final rectangle, matching Chromium's ordered union;
    /// if there is no layout box, return `None`.
    pub fn bounds(&mut self, handle: NodeHandle) -> Result<Option<SceneRect>, EngineError> {
        let node = self.resolve(handle)?;
        if !self.node_is_connected(node) {
            return Ok(None);
        }
        self.update()?;
        Ok(self.node_bounds(node))
    }

    fn node_is_connected(&self, mut node: NodeId) -> bool {
        // Chromium's UpdateStyleAndLayoutForNode skips detached nodes. A
        // geometry read on a detached subtree must not flush unrelated
        // attached layout or clamp its pending scroll positions.
        while !node.is_none() {
            if node == self.document.root() {
                return true;
            }
            node = self.document.node(node).parent;
        }
        false
    }

    fn node_bounds(&self, node: NodeId) -> Option<SceneRect> {
        let rects = self.fragment_rects.get(&node)?;
        let bounds = rects
            .iter()
            .copied()
            .filter(|rect| rect.width > 0.0 && rect.height > 0.0)
            .reduce(|a, b| {
                let x = a.x.min(b.x);
                let y = a.y.min(b.y);
                SceneRect {
                    x,
                    y,
                    width: (a.x + a.width).max(b.x + b.width) - x,
                    height: (a.y + a.height).max(b.y + b.height) - y,
                }
            });
        // Chromium unions rectangles in layout order. An empty accumulator
        // is replaced even by an empty next rectangle, so an entirely empty
        // list retains its final rectangle. Once nonempty, empty boxes do
        // not enlarge the bounds.
        bounds.or_else(|| rects.last().copied())
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
            } else if length.length_type() == openui_geometry::LengthType::Percent {
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
            rects: &mut HashMap<NodeId, Vec<SceneRect>>,
            box_worlds: &mut HashMap<NodeId, Affine>,
        ) {
            let width = fragment.size.width.to_f32();
            let height = fragment.size.height.to_f32();
            let border_box = fragment.border_box_rect();
            let box_width = border_box.size.width.to_f32();
            let box_height = border_box.size.height.to_f32();
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
            let box_world = world.then(Affine::translate(
                border_box.offset.left.to_f32(),
                border_box.offset.top.to_f32(),
            ));
            if node_style.is_some()
                && document.node(fragment.node_id).tag != ElementTag::Text
                && matches!(
                    fragment.kind,
                    openui_layout::FragmentKind::Box | openui_layout::FragmentKind::Viewport
                )
            {
                rects
                    .entry(fragment.node_id)
                    .or_default()
                    .push(box_world.map_rect(box_width, box_height));
                box_worlds.entry(fragment.node_id).or_insert(box_world);
            }
            let world_to_local = box_world.inverse();
            let mut own_clips = inherited_clips.to_vec();
            if let Some(style) = node_style {
                if let Some(inset) = style.clip_path_inset {
                    let resolve = |length: &openui_geometry::Length, size: f32| {
                        if length.is_fixed() {
                            length.value()
                        } else if length.length_type() == openui_geometry::LengthType::Percent {
                            length.value() * size / 100.0
                        } else {
                            0.0
                        }
                    };
                    let top = resolve(&inset[0], box_height);
                    let right = resolve(&inset[1], box_width);
                    let bottom = resolve(&inset[2], box_height);
                    let left = resolve(&inset[3], box_width);
                    let clipped_world = box_world.then(Affine::translate(left, top));
                    if let Some(inverse) = clipped_world.inverse() {
                        own_clips.push(HitClip {
                            world_to_local: inverse,
                            width: (box_width - left - right).max(0.0),
                            height: (box_height - top - bottom).max(0.0),
                            radii: [(0.0, 0.0); 4],
                        });
                    }
                }
                let node = document.node(fragment.node_id);
                let pointer_eligible = node.tag != ElementTag::Text
                    && style.visibility == openui_style::Visibility::Visible
                    && style.pointer_events == openui_style::PointerEvents::Auto;
                if let Some(world_to_local) = world_to_local
                    .filter(|_| pointer_eligible && box_width > 0.0 && box_height > 0.0)
                {
                    out.push(HitEntry {
                        node: fragment.node_id,
                        world_to_local,
                        width: box_width,
                        height: box_height,
                        clips: own_clips.clone(),
                    });
                }
                // An authored overflow clip belongs to the element's border
                // box. A synthetic continuation clip still bounds the larger
                // flow extent carrying its visible descendants.
                let clips_authored_box = style.overflow_x != openui_style::Overflow::Visible
                    || style.overflow_y != openui_style::Overflow::Visible;
                let (clip_world, clip_width, clip_height) =
                    if let Some(scrollport) = fragment.viewport_scrollport {
                        (
                            world,
                            scrollport.client_rect.width().to_f32(),
                            scrollport.client_rect.height().to_f32(),
                        )
                    } else if let (Some(_), Some(area)) =
                        (fragment.element_scrollbars, fragment.scroll_area)
                    {
                        (
                            box_world.then(Affine::translate(
                                area.client_rect.x().to_f32(),
                                area.client_rect.y().to_f32(),
                            )),
                            area.client_rect.width().to_f32(),
                            area.client_rect.height().to_f32(),
                        )
                    } else if clips_authored_box {
                        (box_world, box_width, box_height)
                    } else {
                        (world, width, height)
                    };
                if let Some(world_to_local) =
                    clip_world.inverse().filter(|_| fragment.has_overflow_clip)
                {
                    own_clips.push(HitClip {
                        world_to_local,
                        width: clip_width,
                        height: clip_height,
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
            let child_entries_start = out.len();
            let mut paint_ranges = Vec::with_capacity(children.len());
            for (order, child) in children {
                let (z, phase) = if child.node_id.is_none() {
                    (
                        0,
                        openui_paint::paint_order::InFlowPaintPhase::BlockBackground,
                    )
                } else {
                    let style = &document.node(child.node_id).style;
                    (
                        style.z_index.unwrap_or(0),
                        openui_paint::paint_order::in_flow_paint_phase(style),
                    )
                };
                let start = out.len();
                walk(
                    document,
                    child,
                    child_world,
                    &own_clips,
                    out,
                    rects,
                    box_worlds,
                );
                paint_ranges.push(((z, phase, order), start, out.len()));
            }
            if paint_ranges.windows(2).any(|pair| pair[0].0 > pair[1].0) {
                // Layout rectangle collection retains its original order.
                // Only input entries move into their shared paint phases.
                // Atomic flex/grid content is above later ordinary block
                // backgrounds, while positioned/effect groups remain above it.
                let mut entries: Vec<_> = out
                    .split_off(child_entries_start)
                    .into_iter()
                    .map(Some)
                    .collect();
                paint_ranges.sort_by_key(|range| range.0);
                for (_, start, end) in paint_ranges {
                    out.extend(
                        entries[start - child_entries_start..end - child_entries_start]
                            .iter_mut()
                            .filter_map(Option::take),
                    );
                }
            }
        }
        let mut entries = Vec::new();
        let mut rects = HashMap::new();
        let mut box_worlds = HashMap::new();
        walk(
            &self.document,
            fragment,
            Affine::IDENTITY,
            &[],
            &mut entries,
            &mut rects,
            &mut box_worlds,
        );
        self.hit_test = entries;
        self.fragment_rects = rects;
        self.fragment_box_worlds = box_worlds;
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
    fn native_resolved_computed_defaults_preserve_snapshots_and_native_inheritance() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let parent = engine.root();
        engine
            .set_property(
                parent,
                StyleProperty::TextIndent,
                LengthValue::px(20.0).into(),
            )
            .unwrap();
        engine
            .set_property(parent, StyleProperty::Color, Color::RED.into())
            .unwrap();
        let computed = engine.create_native_element(ElementTag::Span).unwrap();
        engine
            .set_renderer_style(computed, RendererStyleValue::FontSize(16.0))
            .unwrap();
        engine.append_child(parent, computed).unwrap();
        let initial = engine.computed_style(computed).unwrap().clone();
        assert_eq!(initial.text_indent.value(), 0.0);
        assert_eq!(initial.color, Color::BLACK);
        let native = engine.create_native_element(ElementTag::Span).unwrap();
        engine
            .set_property(native, StyleProperty::Display, Display::Inline.into())
            .unwrap();
        engine.append_child(parent, native).unwrap();
        assert_eq!(
            engine.computed_style(native).unwrap().text_indent.value(),
            20.0
        );
        assert_eq!(engine.computed_style(native).unwrap().color, Color::RED);
        let descendant = engine.create_native_element(ElementTag::Span).unwrap();
        engine.append_child(computed, descendant).unwrap();
        engine
            .set_renderer_style(computed, RendererStyleValue::Color(Color::GREEN))
            .unwrap();
        assert_eq!(
            engine.computed_style(descendant).unwrap().color,
            Color::GREEN
        );
        engine
            .set_property(
                parent,
                StyleProperty::TextIndent,
                LengthValue::px(28.0).into(),
            )
            .unwrap();
        assert_eq!(
            engine.computed_style(native).unwrap().text_indent.value(),
            28.0
        );
        assert_eq!(
            engine.computed_style(computed).unwrap().text_indent.value(),
            0.0
        );
        let clone = engine.clone_subtree(computed).unwrap();
        engine.append_child(parent, clone).unwrap();
        assert_eq!(
            engine.computed_style(clone).unwrap().text_indent.value(),
            0.0
        );
        assert_eq!(engine.computed_style(clone).unwrap().color, Color::GREEN);
        assert_eq!(initial.color, Color::BLACK);
        assert_eq!(initial.text_indent.value(), 0.0);
    }

    #[test]
    fn native_resolved_style_animation_refreshes_target_and_native_dependents() {
        use openui_style::{AnimationOptions, FillMode, Keyframes, PropertyKeyframes};
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let target = engine.create_native_element(ElementTag::Div).unwrap();
        let mut style = ComputedStyle::initial();
        style.update_derived(|fields| fields.font_size = 20.0);
        engine.install_derived_style(target, style).unwrap();
        engine.append_child(engine.root(), target).unwrap();
        engine
            .set_property(target, StyleProperty::Width, LengthValue::Em(3.0).into())
            .unwrap();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(target, child).unwrap();
        engine
            .set_property(child, StyleProperty::Width, LengthValue::Em(2.0).into())
            .unwrap();
        let before = engine.computed_style(target).unwrap().clone();
        let animation = engine
            .animate(
                target,
                PropertyKeyframes::typed(
                    StyleProperty::FontSize,
                    Keyframes::from_values(LengthValue::Em(1.0), LengthValue::Em(2.0)),
                )
                .unwrap(),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
                AnimationTimeline::Document,
            )
            .unwrap();
        assert_eq!(engine.computed_style(target).unwrap().font_size, 16.0);
        assert_eq!(engine.computed_style(target).unwrap().width.value(), 48.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 32.0);
        engine.set_animation_time(50.0).unwrap();
        assert_eq!(engine.computed_style(target).unwrap().font_size, 24.0);
        assert_eq!(engine.computed_style(target).unwrap().width.value(), 72.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 48.0);
        engine.cancel_animation(animation).unwrap();
        assert_eq!(engine.computed_style(target).unwrap().font_size, 20.0);
        assert_eq!(engine.computed_style(target).unwrap().width.value(), 60.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 40.0);
        assert_eq!(before.font_size, 20.0);
        assert_eq!(before.width.value(), 60.0);
    }

    #[test]
    fn native_resolved_style_replacement_drops_previous_native_declarations() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let target = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .set_property(target, StyleProperty::Color, Color::RED.into())
            .unwrap();
        let mut style = ComputedStyle::initial();
        style.update_derived(|fields| fields.color = Color::BLUE);
        engine.install_derived_style(target, style).unwrap();
        assert_eq!(engine.computed_style(target).unwrap().color, Color::BLUE);
        engine
            .set_property(target, StyleProperty::Color, Color::RED.into())
            .unwrap();
        assert_eq!(engine.computed_style(target).unwrap().color, Color::RED);
    }

    #[test]
    fn native_resolved_repeated_computed_value_preserves_unchanged_work() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let target = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .set_renderer_style(target, RendererStyleValue::Color(Color::GREEN))
            .unwrap();
        engine.append_child(engine.root(), target).unwrap();
        engine.scene().unwrap();
        let before = engine.dirty_generations();
        engine
            .set_renderer_style(target, RendererStyleValue::Color(Color::GREEN))
            .unwrap();
        assert_eq!(engine.dirty_generations(), before);
        engine.scene().unwrap();
        assert_eq!(engine.dirty_generations(), before);
    }

    #[test]
    fn native_inheritance_computes_renderer_line_height_percentages() {
        use openui_style::LineHeight;
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(24.0).into())
            .unwrap();
        engine
            .set_property(
                root,
                StyleProperty::LineHeight,
                LineHeight::Percentage(150.0).into(),
            )
            .unwrap();
        let child = engine.create_native_element(ElementTag::Span).unwrap();
        engine
            .set_property(child, StyleProperty::FontSize, LengthValue::px(40.0).into())
            .unwrap();
        engine.append_child(root, child).unwrap();
        assert_eq!(
            engine.computed_style(root).unwrap().line_height,
            LineHeight::Length(36.0)
        );
        assert_eq!(
            engine.computed_style(child).unwrap().line_height,
            LineHeight::Length(36.0)
        );
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
            .unwrap();
        assert_eq!(
            engine.computed_style(child).unwrap().line_height,
            LineHeight::Length(30.0)
        );
        engine
            .set_property(
                root,
                StyleProperty::LineHeight,
                StyleValue::Renderer(RendererStyleValue::LineHeight(LineHeight::Percentage(
                    150.0,
                ))),
            )
            .unwrap();
        assert_eq!(
            engine.computed_style(root).unwrap().line_height,
            LineHeight::Length(30.0)
        );
        assert_eq!(
            engine.computed_style(child).unwrap().line_height,
            LineHeight::Length(30.0)
        );
    }

    #[test]
    fn native_inherited_styles_follow_parent_mutations_and_tree_changes() {
        use openui_style::{Direction, LengthValue, Visibility};
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let parent = engine.create_native_element(ElementTag::Div).unwrap();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        let text = engine.create_text("native text").unwrap();
        engine.append_child(child, text).unwrap();
        engine.append_child(engine.root(), parent).unwrap();
        for (property, value) in [
            (StyleProperty::Color, Color::RED.into()),
            (StyleProperty::FontSize, LengthValue::px(20.0).into()),
            (StyleProperty::Direction, Direction::Rtl.into()),
            (
                StyleProperty::Visibility,
                StyleValue::Renderer(RendererStyleValue::Visibility(Visibility::Hidden)),
            ),
        ] {
            engine.set_property(parent, property, value).unwrap();
        }
        engine.append_child(parent, child).unwrap();
        let initial = engine.computed_style(text).unwrap().clone();
        assert_eq!(initial.color, Color::RED);
        assert_eq!(initial.font_size, 20.0);
        assert_eq!(initial.direction, Direction::Rtl);
        assert_eq!(initial.visibility, Visibility::Hidden);
        engine
            .set_property(child, StyleProperty::Color, Color::BLACK.into())
            .unwrap();
        engine
            .set_property(parent, StyleProperty::Color, Color::BLUE.into())
            .unwrap();
        engine
            .set_property(
                parent,
                StyleProperty::FontSize,
                LengthValue::px(24.0).into(),
            )
            .unwrap();
        assert_eq!(engine.computed_style(text).unwrap().color, Color::BLACK);
        assert_eq!(engine.computed_style(text).unwrap().font_size, 24.0);
        engine.detach(child).unwrap();
        assert_eq!(engine.computed_style(text).unwrap().font_size, 16.0);
        assert_eq!(
            engine.computed_style(text).unwrap().direction,
            Direction::Ltr
        );
        assert_eq!(
            engine.computed_style(text).unwrap().visibility,
            Visibility::Visible
        );
        engine.append_child(parent, child).unwrap();
        let duplicate = engine.clone_subtree(child).unwrap();
        assert_eq!(engine.computed_style(duplicate).unwrap().font_size, 16.0);
        assert_eq!(
            engine.computed_style(duplicate).unwrap().color,
            Color::BLACK
        );
        engine.append_child(parent, duplicate).unwrap();
        assert_eq!(engine.computed_style(duplicate).unwrap().font_size, 24.0);
        let resolved = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .install_derived_style(resolved, ComputedStyle::initial())
            .unwrap();
        engine.append_child(parent, resolved).unwrap();
        assert_eq!(engine.computed_style(resolved).unwrap().color, Color::BLACK);
        assert_eq!(engine.computed_style(resolved).unwrap().font_size, 16.0);
        engine.remove(parent).unwrap();
        assert_eq!(initial.color, Color::RED);
        assert_eq!(initial.font_size, 20.0);
    }

    #[test]
    fn native_inheritance_recomputes_relative_lengths_and_preserves_declaration_order() {
        use openui_style::{
            LengthValue, TextWrapMode, WhiteSpace, WhiteSpaceCollapse, WhiteSpaceShorthand,
        };
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let parent = engine.create_native_element(ElementTag::Div).unwrap();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), parent).unwrap();
        engine
            .set_property(
                parent,
                StyleProperty::FontSize,
                LengthValue::px(20.0).into(),
            )
            .unwrap();
        engine
            .set_property(child, StyleProperty::FontSize, LengthValue::Em(2.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Width, LengthValue::Em(3.0).into())
            .unwrap();
        engine
            .set_property(
                child,
                StyleProperty::WhiteSpace,
                WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::Preserve,
                    wrap: TextWrapMode::Nowrap,
                }
                .into(),
            )
            .unwrap();
        engine
            .set_property(
                child,
                StyleProperty::TextWrapMode,
                TextWrapMode::Wrap.into(),
            )
            .unwrap();
        engine.append_child(parent, child).unwrap();
        assert_eq!(engine.computed_style(child).unwrap().font_size, 40.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 120.0);
        assert_eq!(
            engine.computed_style(child).unwrap().white_space,
            WhiteSpace::PreWrap
        );
        engine
            .set_property(
                parent,
                StyleProperty::FontSize,
                LengthValue::px(24.0).into(),
            )
            .unwrap();
        assert_eq!(engine.computed_style(child).unwrap().font_size, 48.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 144.0);
        assert_eq!(
            engine.computed_style(child).unwrap().white_space,
            WhiteSpace::PreWrap
        );
        engine
            .set_property(
                child,
                StyleProperty::WhiteSpace,
                WhiteSpaceShorthand {
                    collapse: WhiteSpaceCollapse::Preserve,
                    wrap: TextWrapMode::Nowrap,
                }
                .into(),
            )
            .unwrap();
        engine
            .set_property(parent, StyleProperty::Color, Color::BLUE.into())
            .unwrap();
        assert_eq!(
            engine.computed_style(child).unwrap().white_space,
            WhiteSpace::Pre
        );
    }

    #[test]
    fn native_inheritance_computes_calculated_font_sizes_against_parent() {
        use openui_geometry::Length;
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(root, child).unwrap();
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
            .unwrap();
        engine
            .set_property(
                child,
                StyleProperty::FontSize,
                LengthValue::Computed(Length::calc_percent_px(150.0, -2.0)).into(),
            )
            .unwrap();
        engine
            .set_property(child, StyleProperty::Width, LengthValue::Em(3.0).into())
            .unwrap();
        let before = engine.computed_style(child).unwrap().clone();
        assert_eq!(before.font_size, 28.0);
        assert_eq!(before.width.value(), 84.0);
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(24.0).into())
            .unwrap();
        assert_eq!(engine.computed_style(child).unwrap().font_size, 34.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 102.0);
        assert_eq!(before.font_size, 28.0);
    }

    #[test]
    fn native_inheritance_distinguishes_percentage_and_unitless_line_heights() {
        use openui_style::LineHeight;
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(root, child).unwrap();
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(24.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::FontSize, LengthValue::px(40.0).into())
            .unwrap();
        engine
            .set_property(
                root,
                StyleProperty::LineHeight,
                LineHeight::Percentage(150.0).into(),
            )
            .unwrap();
        let before = engine.computed_style(child).unwrap().clone();
        assert_eq!(before.line_height, LineHeight::Length(36.0));
        engine
            .set_property(
                root,
                StyleProperty::LineHeight,
                LineHeight::Number(1.5).into(),
            )
            .unwrap();
        assert_eq!(
            engine.computed_style(child).unwrap().line_height,
            LineHeight::Number(1.5)
        );
        assert_eq!(before.line_height, LineHeight::Length(36.0));
    }

    #[test]
    fn native_inheritance_refreshes_authored_pseudo_styles_after_origin_mutation() {
        use openui_style::{FontWeight, PseudoStyleTarget};
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(root, child).unwrap();
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
            .unwrap();
        engine
            .set_property(root, StyleProperty::Color, Color::RED.into())
            .unwrap();
        engine
            .set_pseudo_style(
                child,
                PseudoStyleTarget::FirstLine,
                &Style::default()
                    .font_size(LengthValue::Em(2.0))
                    .font_weight(FontWeight::BOLD),
            )
            .unwrap();
        let before = engine
            .computed_style(child)
            .unwrap()
            .first_line_style
            .as_ref()
            .unwrap()
            .as_ref()
            .clone();
        assert_eq!(before.font_size, 40.0);
        assert_eq!(before.color, Color::RED);
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(24.0).into())
            .unwrap();
        engine
            .set_property(root, StyleProperty::Color, Color::BLUE.into())
            .unwrap();
        let after = engine
            .computed_style(child)
            .unwrap()
            .first_line_style
            .as_ref()
            .unwrap();
        assert_eq!(after.font_size, 48.0);
        assert_eq!(after.color, Color::BLUE);
        assert_eq!(after.font_weight, FontWeight::BOLD);
        let clone = engine.clone_subtree(child).unwrap();
        engine.append_child(root, clone).unwrap();
        let cloned = engine
            .computed_style(clone)
            .unwrap()
            .first_line_style
            .as_ref()
            .unwrap();
        assert_eq!(cloned.font_size, 48.0);
        assert_eq!(cloned.color, Color::BLUE);
        engine.detach(child).unwrap();
        let detached = engine
            .computed_style(child)
            .unwrap()
            .first_line_style
            .as_ref()
            .unwrap();
        assert_eq!(detached.font_size, 32.0);
        assert_eq!(detached.color, Color::BLACK);
        assert_eq!(before.font_size, 40.0);
        assert_eq!(before.color, Color::RED);
    }

    #[test]
    fn native_inheritance_animated_font_resolves_relative_to_parent() {
        use openui_style::{AnimationOptions, FillMode, Keyframes, PropertyKeyframes};
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(root, child).unwrap();
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Width, LengthValue::Em(3.0).into())
            .unwrap();
        let animation = engine
            .animate(
                root,
                PropertyKeyframes::typed(
                    StyleProperty::FontSize,
                    Keyframes::from_values(LengthValue::Em(1.0), LengthValue::Em(2.0)),
                )
                .unwrap(),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
                AnimationTimeline::Document,
            )
            .unwrap();
        assert_eq!(engine.computed_style(root).unwrap().font_size, 16.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 48.0);
        engine.set_animation_time(50.0).unwrap();
        assert_eq!(engine.computed_style(root).unwrap().font_size, 24.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 72.0);
        engine.cancel_animation(animation).unwrap();
        assert_eq!(engine.computed_style(root).unwrap().font_size, 20.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 60.0);
    }

    #[test]
    fn native_inheritance_transition_retains_a_previously_unauthored_target() {
        use openui_style::{AnimationOptions, FillMode};
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(root, child).unwrap();
        engine
            .set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Width, LengthValue::Em(3.0).into())
            .unwrap();
        let animation = engine
            .transition(
                child,
                StyleProperty::FontSize,
                LengthValue::px(40.0),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
            )
            .unwrap();
        engine.cancel_animation(animation).unwrap();
        assert_eq!(engine.computed_style(child).unwrap().font_size, 40.0);
        engine
            .set_property(root, StyleProperty::Color, Color::BLUE.into())
            .unwrap();
        assert_eq!(engine.computed_style(child).unwrap().font_size, 40.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 120.0);
        engine
            .set_viewport(ViewportMetrics::from_logical_size(640.0, 240.0, 1.0).unwrap())
            .unwrap();
        assert_eq!(engine.computed_style(child).unwrap().font_size, 40.0);
        assert_eq!(engine.computed_style(child).unwrap().width.value(), 120.0);
    }

    #[test]
    fn stale_and_cross_document_handles_are_rejected() {
        let mut a =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
        let mut b =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
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
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
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
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
        let first = engine.scene().unwrap();
        let stats = engine.stats();
        let second = engine.scene().unwrap();
        assert_eq!(first.generation(), second.generation());
        assert_eq!(stats, engine.stats());
    }

    #[test]
    fn invalidation_runs_only_the_required_visual_stages() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
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
    fn compositor_distinguishes_owned_recordings_with_equal_document_generations() {
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let mut scenes = Vec::new();
            for color in [Color::RED, Color::BLUE] {
                let mut engine =
                    Engine::new(ViewportMetrics::from_logical_size(64.0, 48.0, scale).unwrap())
                        .unwrap();
                let element = engine.create_element(ElementTag::Div).unwrap();
                engine.append_child(engine.root(), element).unwrap();
                engine
                    .set_property(element, StyleProperty::Display, Display::Block.into())
                    .unwrap();
                engine
                    .set_property(element, StyleProperty::Width, LengthValue::px(32.0).into())
                    .unwrap();
                engine
                    .set_property(element, StyleProperty::Height, LengthValue::px(24.0).into())
                    .unwrap();
                engine
                    .set_property(element, StyleProperty::BackgroundColor, color.into())
                    .unwrap();
                scenes.push(engine.scene().unwrap());
                // Rendering below uses owned scenes after both documents die.
            }
            assert_eq!(scenes[0].generation(), scenes[1].generation());
            let first = SoftwareCompositor::default().render(&scenes[0]).unwrap();
            let second = SoftwareCompositor::default().render(&scenes[1]).unwrap();
            assert_ne!(first.pixels, second.pixels);

            let mut compositor = SoftwareCompositor::default();
            assert_eq!(compositor.render(&scenes[0]).unwrap(), first);
            assert_eq!(compositor.render(&scenes[0].clone()).unwrap(), first);
            assert_eq!(compositor.stats().rasterized, 1);
            assert_eq!(compositor.stats().reused, 1);
            assert_eq!(
                compositor.render(&scenes[1]).unwrap(),
                second,
                "scale={scale}"
            );
            assert_eq!(compositor.stats().rasterized, 2);
            assert_eq!(compositor.stats().reused, 1);
            assert_eq!(compositor.render(&scenes[1].clone()).unwrap(), second);
            assert_eq!(compositor.stats().rasterized, 2);
            assert_eq!(compositor.stats().reused, 2);
            assert_eq!(compositor.render(&scenes[0]).unwrap(), first);
            assert_eq!(compositor.stats().rasterized, 3);
        }
    }

    #[test]
    fn typed_tree_renders_through_scene_compositor() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
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
    fn logical_layout_rasterizes_at_the_authoritative_physical_size() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(64.0, 48.0, 2.0).unwrap()).unwrap();
        let scene = engine.scene().unwrap();
        assert_eq!(scene.viewport().logical_size(), (64.0, 48.0));
        assert_eq!(scene.viewport().physical_size(), (128, 96));
        assert_eq!(
            scene.physical_damage(),
            &[openui_compositor::PhysicalSceneRect {
                x: 0,
                y: 0,
                width: 128,
                height: 96,
            }]
        );

        let frame = SoftwareCompositor::default().render(&scene).unwrap();
        assert_eq!((frame.width, frame.height), (128, 96));
        assert_eq!(frame.viewport, scene.viewport());
        assert_eq!(frame.raster_configuration, scene.raster_configuration());
    }

    #[test]
    fn viewport_units_recompute_and_scale_invalidates_all_visual_stages() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 80.0, 1.0).unwrap()).unwrap();
        let div = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), div).unwrap();
        engine
            .set_property(div, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(
                div,
                StyleProperty::Width,
                LengthValue::ViewportWidth(50.0).into(),
            )
            .unwrap();
        engine
            .set_property(div, StyleProperty::Height, LengthValue::px(10.0).into())
            .unwrap();
        assert_eq!(engine.bounds(div).unwrap().unwrap().width, 50.0);

        engine
            .set_viewport(ViewportMetrics::from_logical_size(200.0, 80.0, 1.0).unwrap())
            .unwrap();
        assert_eq!(engine.bounds(div).unwrap().unwrap().width, 100.0);

        let before = engine.dirty_generations();
        engine
            .set_viewport(ViewportMetrics::from_logical_size(200.0, 80.0, 2.0).unwrap())
            .unwrap();
        let after = engine.dirty_generations();
        assert_eq!(after.intrinsic, before.intrinsic + 1);
        assert_eq!(after.layout, before.layout + 1);
        assert_eq!(after.paint, before.paint + 1);
        assert_eq!(after.compositing, before.compositing + 1);
        assert_eq!(engine.computed_style(div).unwrap().device_scale_factor, 2.0);
        assert_eq!(engine.bounds(div).unwrap().unwrap().width, 100.0);
        assert_eq!(
            engine.scene().unwrap().viewport().physical_size(),
            (400, 160)
        );
    }

    #[test]
    fn hit_testing_respects_transforms_pointer_eligibility_and_rounded_clips() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(100.0, 100.0, 1.0).unwrap()).unwrap();
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
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
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
    fn application_fonts_are_document_owned_invalidate_and_survive_in_scenes() {
        let viewport = ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap();
        let mut first = Engine::new(viewport).unwrap();
        let second = Engine::new(viewport).unwrap();
        let before = first.dirty_generations();
        let handle = first
            .register_font_face(
                Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice()),
                FontFaceDescriptor::new("Engine-owned Ahem"),
            )
            .unwrap();
        assert_eq!(first.object_counts().font_faces, 1);
        assert_eq!(second.object_counts().font_faces, 0);
        assert!(first.dirty_generations().intrinsic > before.intrinsic);

        let scene = first.scene().unwrap();
        assert_eq!(scene.retained_font_face_count(), 1);
        first.unregister_font_face(handle).unwrap();
        assert_eq!(first.object_counts().font_faces, 0);
        assert_eq!(scene.retained_font_face_count(), 1);
        assert!(matches!(
            first.font_face_info(handle),
            Err(EngineError::Font(FontCollectionError::UnknownFace))
        ));
    }

    #[test]
    fn native_text_content_replaces_children_and_reuses_owned_storage() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
        let root = engine.root();
        let parent = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(root, parent).unwrap();
        let replaced = engine.create_text("old").unwrap();
        engine.append_child(parent, replaced).unwrap();
        let old_weak = replaced.downgrade();
        engine.set_text_content(parent, "new").unwrap();
        assert!(old_weak.upgrade(&engine).is_err());
        assert_eq!(engine.text_content(parent).unwrap(), "new");
        let text = engine.children(parent).unwrap()[0];
        engine.set_text_content(text, "data").unwrap();
        assert_eq!(engine.text_data(text).unwrap(), "data");
        assert_eq!(engine.parent(text).unwrap(), Some(parent));
        for iteration in 0..10_000 {
            engine
                .set_text_content(parent, iteration.to_string())
                .unwrap();
        }
        let counts = engine.object_counts();
        assert_eq!(counts.live_nodes, 3);
        assert_eq!(counts.handle_slots, 3);
        assert_eq!(counts.vacant_handle_slots, 0);
        assert_eq!(counts.arena_nodes, 3);
        assert_eq!(counts.reusable_arena_nodes, 0);
        engine.set_text_content(parent, "").unwrap();
        assert_eq!(engine.text_content(parent).unwrap(), "");
        assert_eq!(engine.children(parent).unwrap().len(), 1);
    }

    #[test]
    fn long_running_mutations_reuse_owned_storage() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
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

    #[test]
    fn pseudo_styles_and_language_use_validated_public_paths() {
        use openui_style::{FontWeight, LanguageTag, PseudoStyleTarget, TextDecorationLine};

        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(80.0, 60.0, 1.0).unwrap()).unwrap();
        let node = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        let pseudo = Style::default()
            .font_weight(FontWeight::BOLD)
            .text_decoration_line(TextDecorationLine::UNDERLINE);
        engine
            .set_pseudo_style(node, PseudoStyleTarget::FirstLine, &pseudo)
            .unwrap();
        let id = engine.resolve(node).unwrap();
        let first_line = engine
            .document
            .node(id)
            .style
            .first_line_style
            .as_ref()
            .unwrap();
        assert_eq!(first_line.font_weight, FontWeight::BOLD);
        assert!(first_line.text_decoration_line.has_underline());

        let language = LanguageTag::parse("ar-EG").unwrap();
        engine
            .set_attribute(node, "lang", language.as_str())
            .unwrap();
        assert_eq!(
            engine.document.node(id).style.locale.as_deref(),
            Some("ar-EG")
        );
        assert!(LanguageTag::parse("not_a_tag").is_none());
    }
}
