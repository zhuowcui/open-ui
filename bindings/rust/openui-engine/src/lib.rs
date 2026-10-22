//! Retained, thread-affine Open UI document engine.

mod accessibility;
mod animation;
mod animation_timeline;
mod authored_source;
mod client_geometry;
mod compositor_scene;
mod control_geometry;
mod element_dimensions;
mod forms;
mod hit_geometry;
mod interaction;
mod scroll_button;
mod scroll_into_view;
mod scroll_notifications;
mod scroll_overflow;
mod table_spans;

pub use scroll_button::{
    ScrollButtonAvailability, ScrollButtonDirection, ScrollButtonState, ScrollSnapStrictness,
};

pub use scroll_into_view::{ScrollAlignment, ScrollIntoViewContainer, ScrollIntoViewOptions};

pub use accessibility::{
    AccessibilityAction, AccessibilityActionData, AccessibilityActionRequest, AccessibilityLive,
    AccessibilityNode, AccessibilityNodeId, AccessibilityPlatformAction, AccessibilityRelation,
    AccessibilityRole, AccessibilityScrollDirection, AccessibilityToggled, AccessibilityTreeUpdate,
};

pub use animation_timeline::{
    AnimationAttachmentSnapshot, AnimationResolvedTimelineRanges, AnimationTimelineSnapshot,
};

pub use animation::{
    AnimationEvent, AnimationEventKind, AnimationId, AnimationState, AnimationTimeline,
    AnimationTimelineSource, ScrollAnimationId,
};

pub use interaction::{
    ActivationResult, CheckableActivation, ControlAdjustment, ControlState, EditCommand,
    EventPhase, EventRoute, FocusOrigin, PointerEventKind, PointerUpdate, RangeSelectionMode,
    RouteStep, SelectionDirection, TextDirection, TextUnit,
};

use authored_source::NativeStyleSource;
use hit_geometry::WorldTransform;
use openui_compositor::{SceneGeneration, SceneRect, SceneSnapshot};
use openui_dom::{
    Document as NativeDocument, ElementTag, FormControlRole, NodeId, PseudoElementKind,
    ReplacedContent, ReplacedResourceKind,
};
pub use openui_geometry::{
    RasterBackend, RasterConfiguration, RasterPixelGeometry, TextEdging, TextHinting,
    TextRasterConfiguration, ViewportAuthority, ViewportMetrics, ViewportMetricsError,
};
use openui_layout::Fragment;
use openui_paint::{record_fragment, record_fragment_with_program};
use openui_style::{
    apply_to_computed, AuthoredStyleContribution, AuthoredStyleDeclarations, CanonicalStyleField,
    Color, ComputedStyle, Display, ImageResourceId, InvalidationClass, PseudoStyleTarget,
    RendererInternalStyleValue, RendererStyleValue, Style, StyleProperty, StyleValue,
};
pub use openui_text::{
    FontAxisRange, FontCollection, FontCollectionError, FontCollectionStats, FontContainerFormat,
    FontFaceDescriptor, FontFaceHandle, FontFaceInfo, FontFeatureDefault, FontMetricOverrides,
    FontPaletteBase, FontPaletteEntryOverride, FontPaletteHandle, FontPaletteValuesDescriptor,
    FontStyleRange, FontUnicodeRange, HyphenationDictionaryHandle, HyphenationRegistry,
    HyphenationRegistryError,
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    AllocationFailure,
}

impl std::fmt::Display for EngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AllocationFailure => f.write_str("unable to allocate owned Engine data"),
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

impl From<openui_style::AnimationError> for EngineError {
    fn from(value: openui_style::AnimationError) -> Self {
        match value {
            openui_style::AnimationError::AllocationFailure => Self::AllocationFailure,
            error => Self::Render(error.to_string()),
        }
    }
}

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
    native_source: NativeStyleSource,
    authored: AuthoredStyleDeclarations,
    authored_pseudo: [Option<Style>; 4],
    // The current unanimated computed source is distinct from presentation.
    // It survives inactive/forwards-filled effects until the final owner leaves.
    animation_base_style: Option<ComputedStyle>,
    animation_properties: BTreeSet<u16>,
}

#[derive(Debug, Clone)]
struct HitClip {
    world_to_local: WorldTransform,
    width: f32,
    height: f32,
    clip_x: bool,
    clip_y: bool,
    radii: [(f32, f32); 4],
    structural_projection: Option<openui_layout::AuthoredProjection>,
}

impl HitClip {
    fn contains(&self, x: f32, y: f32) -> bool {
        let (x, y) = self.world_to_local.project_point(x, y);
        if self.clip_x && self.clip_y {
            rounded_rect_contains(x, y, self.width, self.height, self.radii)
        } else {
            (!self.clip_x || (x >= 0.0 && x < self.width))
                && (!self.clip_y || (y >= 0.0 && y < self.height))
        }
    }
}

#[derive(Debug, Clone)]
struct HitEntry {
    node: NodeId,
    world_to_local: WorldTransform,
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
    font_collection_generation: u64,
    document: NativeDocument,
    slots: Vec<Slot>,
    free_slots: Vec<u32>,
    node_slots: HashMap<NodeId, u32>,
    dirty: DirtyState,
    dirty_generations: DirtyGenerations,
    latest_fragment: Option<Arc<Fragment>>,
    latest_scene: Option<SceneSnapshot>,
    scene_registry: compositor_scene::SceneIdentityRegistry,
    layered_paint_capture_enabled: bool,
    hit_test: Vec<HitEntry>,
    fragment_rects: HashMap<NodeId, Vec<SceneRect>>,
    fragment_box_worlds: HashMap<NodeId, WorldTransform>,
    focused: Option<NodeHandle>,
    pointer_capture: HashMap<u64, NodeHandle>,
    hover_paths: HashMap<u64, Vec<NodeHandle>>,
    active_pointers: HashMap<u64, NodeHandle>,
    controls: HashMap<u32, ControlState>,
    scroll_buttons: scroll_button::ScrollButtons,
    radio_associations: HashMap<u32, forms::RadioAssociation>,
    semantics: HashMap<u32, accessibility::SemanticProperties>,
    accessibility_nodes: HashMap<AccessibilityNodeId, AccessibilityNode>,
    accessibility_initialized: bool,
    focus_visible: bool,
    modal_root: Option<NodeHandle>,
    focus_before_modal: Option<NodeHandle>,
    animation_time_ms: f64,
    animations: BTreeMap<AnimationId, animation::AnimationInstance>,
    animation_events: Vec<AnimationEvent>,
    scroll_notifications: scroll_notifications::ScrollNotifications,
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
        let font_collection_generation = document.font_collection().generation();
        let root_slot = Slot {
            generation: 1,
            node: Some(root_node),
            native_source: NativeStyleSource::Defaults(document.node(root_node).style.clone()),
            authored: AuthoredStyleDeclarations::default(),
            authored_pseudo: Default::default(),
            animation_base_style: None,
            animation_properties: BTreeSet::new(),
        };
        Ok(Self {
            id: NEXT_DOCUMENT_ID.fetch_add(1, Ordering::Relaxed),
            viewport,
            raster_configuration: options.raster_configuration,
            font_collection_generation,
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
            scene_registry: compositor_scene::SceneIdentityRegistry::default(),
            layered_paint_capture_enabled: false,
            hit_test: Vec::new(),
            fragment_rects: HashMap::new(),
            fragment_box_worlds: HashMap::new(),
            focused: None,
            pointer_capture: HashMap::new(),
            hover_paths: HashMap::new(),
            active_pointers: HashMap::new(),
            controls: HashMap::new(),
            scroll_buttons: scroll_button::ScrollButtons::default(),
            radio_associations: HashMap::new(),
            semantics: HashMap::new(),
            accessibility_nodes: HashMap::new(),
            accessibility_initialized: false,
            focus_visible: false,
            modal_root: None,
            focus_before_modal: None,
            animation_time_ms: 0.0,
            animations: BTreeMap::new(),
            animation_events: Vec::new(),
            scroll_notifications: Default::default(),
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
        self.synchronize_font_collection()?;
        Ok(handle)
    }

    pub fn unregister_font_face(&mut self, handle: FontFaceHandle) -> Result<(), EngineError> {
        self.document.font_collection().unregister(handle)?;
        self.synchronize_font_collection()?;
        Ok(())
    }

    /// Refresh retained declarations when a shared application font collection
    /// changes. Unchanged generations perform no style, layout or paint work.
    pub fn synchronize_font_collection(&mut self) -> Result<(), EngineError> {
        let generation = self.font_collection().generation();
        if generation != self.font_collection_generation {
            self.recompute_authored_styles()?;
            self.font_collection_generation = generation;
            self.mark_dirty(InvalidationClass::Intrinsic);
        }
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
        if logical_changed || scale_changed {
            self.recompute_authored_styles()?;
        }
        if logical_changed {
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
        self.create_native_element_with_display_default(tag, display)
    }

    /// Create a native alias with its constructor-owned display default.
    #[doc(hidden)]
    pub fn create_native_element_with_display_default(
        &mut self,
        tag: ElementTag,
        display: Display,
    ) -> Result<NodeHandle, EngineError> {
        let handle = self.create_element(tag)?;
        self.set_native_display_default(handle, display)?;
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
        self.slots[handle.index as usize].native_source =
            NativeStyleSource::Defaults(data.style.clone());
        Ok(handle)
    }

    fn register_native_node(&mut self, node: NodeId) -> NodeHandle {
        let index = if let Some(index) = self.free_slots.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            slot.native_source =
                NativeStyleSource::Defaults(self.document.node(node).style.clone());
            slot.authored = AuthoredStyleDeclarations::default();
            slot.authored_pseudo = Default::default();
            slot.animation_base_style = None;
            slot.animation_properties.clear();
            index
        } else {
            let index = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 1,
                node: Some(node),
                native_source: NativeStyleSource::Defaults(self.document.node(node).style.clone()),
                authored: AuthoredStyleDeclarations::default(),
                authored_pseudo: Default::default(),
                animation_base_style: None,
                animation_properties: BTreeSet::new(),
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
        self.validate_owned_scroll_button_move(child)?;
        let parent_node = self.resolve(parent)?;
        let child_node = self.resolve(child)?;
        if !self.document.node(child_node).parent.is_none() {
            return Err(EngineError::AlreadyAttached);
        }
        self.validate_child_placement(parent, child)?;
        self.document.append_child(parent_node, child_node);
        self.relocate_owned_scroll_buttons()?;
        self.refresh_inherited_styles(child_node)?;
        self.reconcile_radio_subtree(child_node)?;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    /// Validate a move without changing the tree or focus.
    pub fn validate_append_or_move_child(
        &self,
        parent: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), EngineError> {
        self.validate_owned_scroll_button_move(child)?;
        self.validate_child_placement(parent, child)
    }

    /// Validate both presentation and origin-owned ancestry before any mutation.
    /// A scroll button can be a sibling box, or be detached while its origin is
    /// hidden; it remains an owned semantic descendant of that origin. Moving
    /// the origin beneath that button would make subsequent relocation cyclic.
    fn validate_child_placement(
        &self,
        parent: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), EngineError> {
        self.resolve(child)?;
        let mut pending = Vec::new();
        pending
            .try_reserve(1)
            .map_err(|_| EngineError::AllocationFailure)?;
        pending.push((parent, false));
        // True means the node is on the current DFS path; false means its two
        // ancestor routes have completed. Shared ancestry is a valid diamond.
        let mut visited: HashMap<NodeHandle, bool> = HashMap::new();
        while let Some((ancestor, leaving)) = pending.pop() {
            if leaving {
                *visited.get_mut(&ancestor).expect("entered ancestor") = false;
                continue;
            }
            let node = self.resolve(ancestor)?;
            if ancestor == child {
                return Err(EngineError::Cycle);
            }
            match visited.get(&ancestor) {
                Some(true) => return Err(EngineError::Cycle),
                Some(false) => continue,
                None => {}
            }
            visited
                .try_reserve(1)
                .map_err(|_| EngineError::AllocationFailure)?;
            pending
                .try_reserve(3)
                .map_err(|_| EngineError::AllocationFailure)?;
            visited.insert(ancestor, true);
            pending.push((ancestor, true));

            let physical_parent = self.document.node(node).parent;
            if !physical_parent.is_none() {
                let index = *self
                    .node_slots
                    .get(&physical_parent)
                    .ok_or(EngineError::InvalidInput("ancestor node is not registered"))?;
                let slot = self
                    .slots
                    .get(index as usize)
                    .ok_or(EngineError::StaleHandle)?;
                let parent_handle = NodeHandle {
                    document: self.id,
                    index,
                    generation: slot.generation,
                };
                if self.resolve(parent_handle)? != physical_parent {
                    return Err(EngineError::InvalidInput(
                        "ancestor registration is inconsistent",
                    ));
                }
                pending.push((parent_handle, false));
            }
            if let Some(binding) = self.scroll_buttons.bindings.get(&ancestor) {
                if binding.button != ancestor {
                    return Err(EngineError::InvalidInput(
                        "scroll-button ownership is inconsistent",
                    ));
                }
                self.resolve(binding.origin)?;
                if self
                    .scroll_buttons
                    .directions
                    .get(&(binding.origin, binding.direction))
                    != Some(&ancestor)
                {
                    return Err(EngineError::InvalidInput(
                        "scroll-button direction owner is inconsistent",
                    ));
                }
                pending.push((binding.origin, false));
            }
        }
        Ok(())
    }

    /// Validate an insertion before callbacks can change its operands.
    pub fn validate_insert_before(
        &self,
        parent: NodeHandle,
        child: NodeHandle,
        before: NodeHandle,
    ) -> Result<(), EngineError> {
        self.validate_append_or_move_child(parent, child)?;
        let parent = self.resolve(parent)?;
        let before = self.resolve(before)?;
        if self.document.node(before).parent != parent {
            return Err(EngineError::InvalidSibling);
        }
        Ok(())
    }

    /// Validate a detach without clearing focus or presentation state.
    pub fn validate_detach(&self, handle: NodeHandle) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        if node == self.document.root() {
            return Err(EngineError::RootRemoval);
        }
        Ok(())
    }

    /// Append a node, moving it from its existing parent when necessary.
    pub fn append_or_move_child(
        &mut self,
        parent: NodeHandle,
        child: NodeHandle,
    ) -> Result<(), EngineError> {
        self.validate_append_or_move_child(parent, child)?;
        let parent_node = self.resolve(parent)?;
        let child_node = self.resolve(child)?;
        self.document.detach(child_node);
        self.remove_radio_subtree(child_node)?;
        self.document.append_child(parent_node, child_node);
        self.relocate_owned_scroll_buttons()?;
        self.refresh_inherited_styles(child_node)?;
        self.reconcile_radio_subtree(child_node)?;
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
        self.validate_insert_before(parent, child, before)?;
        let parent_node = self.resolve(parent)?;
        let child_node = self.resolve(child)?;
        let before_node = self.resolve(before)?;
        // Inserting a node before itself still removes and reinserts it.
        // Its former next sibling determines the insertion position.
        let insertion_point = if child_node == before_node {
            self.document.node(child_node).next_sibling
        } else {
            before_node
        };
        self.document.detach(child_node);
        self.remove_radio_subtree(child_node)?;
        if insertion_point.is_none() {
            self.document.append_child(parent_node, child_node);
        } else {
            self.document.insert_before(insertion_point, child_node);
        }
        self.relocate_owned_scroll_buttons()?;
        self.refresh_inherited_styles(child_node)?;
        self.reconcile_radio_subtree(child_node)?;
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
            .filter(|handle| !self.scroll_buttons.bindings.contains_key(handle))
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
            if let Some(base) = &self.slots[original.index as usize].animation_base_style {
                data.style = base.clone();
            }
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
            // Snap mode follows the copied native authored style. Target IDs,
            // controls and offsets remain fresh on the cloned document nodes;
            // reverting to static child translation would lose native offsets.
            data.attributes.remove("data-oui-focused");
            data.attributes.remove("data-oui-composition-start");
            data.attributes.remove("data-oui-composition-end");
            *self.document.node_mut(duplicate_node) = data;
            self.slots[duplicate.index as usize].native_source =
                self.slots[original.index as usize].native_source.clone();
            self.slots[duplicate.index as usize].authored = authored;
            self.slots[duplicate.index as usize].authored_pseudo =
                self.slots[original.index as usize].authored_pseudo.clone();
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
        self.reconcile_radio_subtree(self.resolve(root)?)?;
        Ok(root)
    }

    /// Find the first attached element with this ID in document order.
    pub fn element_by_id(&self, id: &str) -> Option<NodeHandle> {
        if id.is_empty() {
            return None;
        }
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
        self.validate_owned_scroll_button_move(handle)?;
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
        self.relocate_owned_scroll_buttons()?;
        let detached_handles: Vec<_> = descendants
            .iter()
            .filter_map(|node| self.node_slots.get(node))
            .map(|index| self.handle_for_slot(*index))
            .collect();
        self.clear_subtree_presentation_state(&detached_handles);
        self.refresh_inherited_styles(node)?;
        self.remove_radio_subtree(node)?;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    fn clear_subtree_presentation_state(&mut self, handles: &[NodeHandle]) {
        self.cancel_animations_for_handles(handles);
        if self
            .focused
            .is_some_and(|focused| handles.contains(&focused))
        {
            if let Some(focused) = self.focused {
                if let Ok(node) = self.resolve(focused) {
                    self.document
                        .node_mut(node)
                        .attributes
                        .remove("data-oui-focused");
                }
            }
            self.focused = None;
            self.focus_visible = false;
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
        self.forget_scroll_button_handles(&removed_handles);
        self.remove_radio_subtree(node)?;
        for node in &descendants {
            if let Some(index) = self.node_slots.remove(node) {
                self.controls.remove(&index);
                self.radio_associations.remove(&index);
                self.semantics.remove(&index);
                let slot = &mut self.slots[index as usize];
                slot.node = None;
                slot.native_source = NativeStyleSource::Defaults(ComputedStyle::initial());
                slot.authored = AuthoredStyleDeclarations::default();
                slot.authored_pseudo = Default::default();
                slot.animation_base_style = None;
                slot.animation_properties.clear();
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
        self.scene_registry.retire(&removed_handles);
        self.scroll_notifications.retire(&removed_handles);
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
        let previous_id = (name == "id")
            .then(|| self.document.attribute(node, "id").map(str::to_owned))
            .flatten();
        self.document
            .set_attribute(node, name.clone(), value.clone());
        if name == "id" {
            self.radio_id_targets_changed(&[previous_id.as_deref().unwrap_or_default(), &value])?;
        }
        if name == "lang" {
            self.set_property(
                handle,
                StyleProperty::Locale,
                StyleValue::Renderer(RendererStyleValue::Locale(Some(value.clone()))),
            )?;
        }
        self.sync_control_attribute(handle, &name, &value);
        let table_span = self.sync_table_span_attribute(node, &name, Some(&value));
        self.mark_dirty(
            if name == "open" && self.document.node(node).tag == ElementTag::Details {
                InvalidationClass::Subtree
            } else if name == "disabled" {
                InvalidationClass::Paint
            } else if table_span || self.control_attribute_changes_geometry(node, &name) {
                InvalidationClass::Intrinsic
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
        let previous = self
            .document
            .node_mut(node)
            .attributes
            .remove(&name.to_ascii_lowercase());
        let removed = previous.is_some();
        if removed {
            if name.eq_ignore_ascii_case("id") {
                self.radio_id_targets_changed(&[previous.as_deref().unwrap_or_default()])?;
            }
            if name.eq_ignore_ascii_case("lang") {
                self.set_property(
                    handle,
                    StyleProperty::Locale,
                    StyleValue::Renderer(RendererStyleValue::Locale(None)),
                )?;
            }
            self.remove_control_attribute(handle, &name.to_ascii_lowercase());
            let table_span = self.sync_table_span_attribute(node, &name.to_ascii_lowercase(), None);
            self.mark_dirty(
                if name.eq_ignore_ascii_case("open")
                    && self.document.node(node).tag == ElementTag::Details
                {
                    InvalidationClass::Subtree
                } else if name.eq_ignore_ascii_case("disabled") {
                    InvalidationClass::Paint
                } else if table_span
                    || self.control_attribute_changes_geometry(node, &name.to_ascii_lowercase())
                {
                    InvalidationClass::Intrinsic
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
        let old_transform = (property == StyleProperty::Transform)
            .then(|| openui_style::value_from_computed(&self.document.node(node).style, property));
        let Some(candidate) = self.staged_authored_property(handle, property, value)? else {
            return Ok(());
        };
        let previous =
            std::mem::replace(&mut self.slots[handle.index as usize].authored, candidate);
        if let Err(error) = self.refresh_inherited_styles(node) {
            self.slots[handle.index as usize].authored = previous;
            return Err(error);
        }
        if old_transform.as_ref().is_some_and(|old| {
            openui_style::value_from_computed(&self.document.node(node).style, property) == *old
        }) {
            return Ok(());
        }
        self.dirty.hit_test = true;
        self.mark_dirty(property.metadata().invalidation);
        Ok(())
    }

    /// Remove the authored canonical longhands represented by `property`.
    /// Constructor defaults and explicit snapshot source inputs remain. A
    /// removed part of an older shorthand does not revive on recomposition.
    /// Returns false without work if no live declaration owns these fields.
    pub fn remove_property(
        &mut self,
        handle: NodeHandle,
        property: StyleProperty,
    ) -> Result<bool, EngineError> {
        let node = self.resolve(handle)?;
        let mut candidate = self.slots[handle.index as usize].authored.clone();
        if !candidate.remove(property) {
            return Ok(false);
        }
        let old_transform = (property == StyleProperty::Transform)
            .then(|| openui_style::value_from_computed(&self.document.node(node).style, property));
        let previous =
            std::mem::replace(&mut self.slots[handle.index as usize].authored, candidate);
        if let Err(error) = self.refresh_inherited_styles(node) {
            self.slots[handle.index as usize].authored = previous;
            return Err(error);
        }
        if !old_transform.as_ref().is_some_and(|old| {
            openui_style::value_from_computed(&self.document.node(node).style, property) == *old
        }) {
            self.dirty.hit_test = true;
            self.mark_dirty(property.metadata().invalidation);
        }
        Ok(true)
    }

    /// Apply a schema-generated computed longhand to a resolved snapshot.
    /// Omitted fields retain their computed defaults; they must not inherit
    /// again. This input freezes the current author declarations into computed
    /// state. Scroll snapping still uses the retained Engine layout barrier,
    /// offsets and target identities. Native applications use `set_property` or
    /// framework setters to
    /// retain declarations that respond to parent and viewport changes.
    #[doc(hidden)]
    pub fn set_renderer_style(
        &mut self,
        handle: NodeHandle,
        value: RendererStyleValue,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        let property = value.property();
        // The old renderer affine setter shares the retained source route.
        // An identity legacy matrix means none; an authored identity operation
        // supplied through the native TransformList remains a specified box.
        let value = match value {
            RendererStyleValue::Transform(matrix) => StyleValue::Transform(
                openui_style::TransformList(if matrix == openui_style::Transform2D::IDENTITY {
                    Vec::new()
                } else {
                    vec![openui_style::TransformOperation::Matrix(matrix)]
                }),
            ),
            value => StyleValue::Renderer(value),
        };
        let old_transform = (property == StyleProperty::Transform).then(|| {
            (
                self.document
                    .node(node)
                    .style
                    .has_transform_containing_block(),
                openui_style::value_from_computed(&self.document.node(node).style, property),
            )
        });
        let slot = &self.slots[handle.index as usize];
        if slot.native_source.is_snapshot()
            && slot.authored.contributions().is_empty()
            && slot.authored_pseudo.iter().all(Option::is_none)
            && !self
                .animations
                .values()
                .any(|animation| animation.target == handle)
            // Derived presentation can equal an explicit renderer input even
            // while its permanent source differs (for example text-area Auto
            // overflow adjusted from Visible). Retain that input before a later
            // metadata/context change removes the derived adjustment.
            && openui_style::value_from_computed(slot.native_source.style(), property) == value
            && openui_style::value_from_computed(&self.document.node(node).style, property) == value
            && self.intrinsic_capture_width_matches(handle.index, &value)
        {
            return Ok(());
        }
        let viewport = (
            self.viewport.logical_width() as f32,
            self.viewport.logical_height() as f32,
        );
        let mut source = self.unanimated_style(node, handle.index)?;
        apply_to_computed(&mut source, property, &value, viewport)
            .map_err(|_| EngineError::PropertyType { property })?;
        let slot = &mut self.slots[handle.index as usize];
        let previous_source =
            std::mem::replace(&mut slot.native_source, NativeStyleSource::Snapshot(source));
        let previous_authored = std::mem::take(&mut slot.authored);
        if let Err(error) = self.refresh_inherited_styles(node) {
            let slot = &mut self.slots[handle.index as usize];
            slot.native_source = previous_source;
            slot.authored = previous_authored;
            return Err(error);
        }
        self.commit_intrinsic_capture_width(handle.index, &value);
        if let Some((old_policy, old_value)) = old_transform {
            let style = self.document.node(node).style.clone();
            self.invalidate_transform_containing_block_change(node, old_policy, &style);
            if openui_style::value_from_computed(&style, property) == old_value {
                return Ok(());
            }
        }
        self.dirty.hit_test = true;
        self.mark_dirty(property.metadata().invalidation);
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
        value
            .clone()
            .apply_to(self.slots[handle.index as usize].native_source.style_mut());
        if let Some(base) = &mut self.slots[handle.index as usize].animation_base_style {
            value.clone().apply_to(base);
        }
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
        let source_scrollbar_width = style.scrollbar_width;
        let slot = &mut self.slots[handle.index as usize];
        let previous_source =
            std::mem::replace(&mut slot.native_source, NativeStyleSource::Snapshot(style));
        let previous_authored = std::mem::take(&mut slot.authored);
        let previous_pseudo = std::mem::take(&mut slot.authored_pseudo);
        if let Err(error) = self.refresh_inherited_styles(node) {
            let slot = &mut self.slots[handle.index as usize];
            slot.native_source = previous_source;
            slot.authored = previous_authored;
            slot.authored_pseudo = previous_pseudo;
            return Err(error);
        }
        self.commit_intrinsic_capture_width(
            handle.index,
            &StyleValue::Renderer(RendererStyleValue::ScrollbarWidth(source_scrollbar_width)),
        );
        self.dirty.hit_test = true;
        self.mark_dirty(InvalidationClass::Subtree);
        Ok(())
    }

    /// Resolve inheritance before owned queries, layout and paint. Anonymous
    /// and pseudo snapshots are already resolved and retain their values.
    fn refresh_inherited_styles(&mut self, root: NodeId) -> Result<(), EngineError> {
        self.publish_native_cascade(root)
    }

    /// A specified identity transform owns positioned descendants just like
    /// a nonidentity matrix. Replacing that policy requires layout only when
    /// a real retained descendant can change its geometric containing block.
    /// Borrowed helper style IDs do not create independent element ownership.
    fn invalidate_transform_containing_block_change(
        &mut self,
        owner: NodeId,
        previous_policy: bool,
        style: &ComputedStyle,
    ) {
        if self.dirty.layout
            || previous_policy == style.has_transform_containing_block()
            || owner == self.document.root()
        {
            return;
        }
        fn collect_owned(fragment: &Fragment, nodes: &mut std::collections::HashSet<NodeId>) {
            if fragment.owns_element_box() {
                nodes.insert(fragment.node_id);
            }
            for child in &fragment.children {
                collect_owned(child, nodes);
            }
        }
        let mut owned = std::collections::HashSet::new();
        if let Some(fragment) = &self.latest_fragment {
            collect_owned(fragment, &mut owned);
        } else {
            // There is no retained layout before the first frame. The first
            // layout is normally already dirty; keep this fallback explicit.
            owned.extend(self.node_slots.keys().copied());
        }
        if !owned.contains(&owner) {
            return;
        }
        let affects = owned.iter().copied().any(|descendant| {
            if descendant == owner {
                return false;
            }
            let position = self.document.node(descendant).style.position;
            let fixed = position == openui_style::Position::Fixed;
            if !fixed && position != openui_style::Position::Absolute {
                return false;
            }
            // An existing position/containment owner keeps absolute geometry
            // stable. Fixed geometry is independent of position:relative.
            if style.has_layout_containment() || (!fixed && style.position.is_positioned()) {
                return false;
            }
            let mut current = self.document.node(descendant).parent;
            while !current.is_none() {
                if current == owner {
                    return true;
                }
                if owned.contains(&current) {
                    let ancestor = &self.document.node(current).style;
                    if ancestor.has_transform_containing_block()
                        || ancestor.has_layout_containment()
                        || (!fixed && ancestor.position.is_positioned())
                    {
                        return false;
                    }
                }
                current = self.document.node(current).parent;
            }
            false
        });
        if affects {
            self.mark_dirty(InvalidationClass::Layout);
        }
    }

    /// Compute native declarations once for ordinary and pseudo styles.
    fn resolve_native_declarations(
        &self,
        mut style: ComputedStyle,
        declarations: &[AuthoredStyleContribution],
        parent: &ComputedStyle,
        root_font: f32,
        viewport: (f32, f32),
        is_root: bool,
    ) -> Result<ComputedStyle, EngineError> {
        let parent_font = parent.font_size;
        let parent_metrics = declarations
            .iter()
            .any(|contribution| {
                (contribution
                    .fields()
                    .contains(CanonicalStyleField::FontSize)
                    || contribution
                        .fields()
                        .contains(CanonicalStyleField::LineHeight))
                    && contribution.value().depends_on_font_metrics()
            })
            .then(|| self.native_font_length_metrics(parent));
        // Font size is computed against the parent, and all other
        // em lengths use the final cascaded size of this element.
        let mut font_style = style.clone();
        for contribution in declarations {
            let property = &contribution.property();
            let value = contribution.value();
            if contribution
                .fields()
                .contains(CanonicalStyleField::FontSize)
            {
                let value = Self::resolve_native_lengths(
                    *property,
                    value,
                    parent_font,
                    root_font,
                    viewport,
                    parent_metrics,
                );
                contribution
                    .project_resolved(&mut font_style, &value, viewport)
                    .map_err(|_| EngineError::PropertyType {
                        property: *property,
                    })?;
            }
        }
        let font_size = font_style.font_size;
        style.update_derived(|fields| fields.font_size = font_size);
        // Metric-dependent lengths see every final font/line-height property,
        // even when width or padding was declared before the font family.
        // Ordinary declarations retain their existing font-free path.
        let metrics = if declarations
            .iter()
            .any(|contribution| contribution.value().depends_on_font_metrics())
        {
            let mut metric_style = font_style.clone();
            let mut authored_line_height = false;
            for contribution in declarations {
                let property = &contribution.property();
                let value = contribution.value();
                if value.depends_on_font_metrics() {
                    continue;
                }
                let value = value.capture_own_current_color(*property, parent.color);
                let value = Self::resolve_native_lengths(
                    *property,
                    &value,
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
                    None,
                );
                contribution
                    .project_resolved(&mut metric_style, &value, viewport)
                    .map_err(|_| EngineError::PropertyType {
                        property: *property,
                    })?;
                metric_style.update_derived(|fields| fields.font_size = font_size);
                authored_line_height |= contribution
                    .fields()
                    .contains(CanonicalStyleField::LineHeight);
            }
            if authored_line_height {
                metric_style.update_derived(|fields| {
                    if let openui_style::LineHeight::Percentage(percent) = fields.line_height {
                        fields.line_height = openui_style::LineHeight::Length(
                            font_size * (percent as i32) as f32 / 100.0,
                        );
                    }
                });
            }
            // A font shorthand can override a preceding line-height. Resolve
            // only the winning metric-dependent line-height after all font
            // longhands, then capture the lh basis for other declarations.
            if let Some(contribution) = declarations.iter().rev().find(|contribution| {
                contribution
                    .fields()
                    .contains(CanonicalStyleField::LineHeight)
            }) {
                let value = contribution.value();
                if contribution.property() == StyleProperty::LineHeight
                    && value.depends_on_font_metrics()
                {
                    let mut line_metrics = self.native_font_length_metrics(&metric_style);
                    line_metrics.lh = parent_metrics.expect("line-height parent metrics").lh;
                    let resolved = Self::resolve_native_lengths(
                        StyleProperty::LineHeight,
                        value,
                        font_size,
                        if is_root { font_size } else { root_font },
                        viewport,
                        Some(line_metrics),
                    );
                    contribution
                        .project_resolved(&mut metric_style, &resolved, viewport)
                        .map_err(|_| EngineError::PropertyType {
                            property: StyleProperty::LineHeight,
                        })?;
                }
            }
            Some(self.native_font_length_metrics(&metric_style))
        } else {
            None
        };
        let mut authored_line_height = false;
        for contribution in declarations {
            let property = &contribution.property();
            let value = contribution.value();
            let value = value.capture_own_current_color(*property, parent.color);
            let value = Self::resolve_native_lengths(
                *property,
                &value,
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
                if *property == StyleProperty::FontSize {
                    parent_metrics
                } else if *property == StyleProperty::LineHeight {
                    metrics.map(|mut metrics| {
                        if let Some(parent) = parent_metrics {
                            metrics.lh = parent.lh;
                        }
                        metrics
                    })
                } else {
                    metrics
                },
            );
            contribution
                .project_resolved(&mut style, &value, viewport)
                .map_err(|_| EngineError::PropertyType {
                    property: *property,
                })?;
            style.update_derived(|fields| fields.font_size = font_size);
            if contribution
                .fields()
                .contains(CanonicalStyleField::LineHeight)
            {
                authored_line_height = true;
            }
        }
        if authored_line_height {
            style.update_derived(|fields| {
                if let openui_style::LineHeight::Percentage(percent) = fields.line_height {
                    fields.line_height = openui_style::LineHeight::Length(
                        font_size * (percent as i32) as f32 / 100.0,
                    );
                }
            });
        }
        style.resolve_current_color_dependencies();
        Ok(style)
    }

    fn resolve_authored_pseudo_styles(
        &self,
        index: u32,
        mut origin: ComputedStyle,
        root_font: f32,
        viewport: (f32, f32),
    ) -> Result<ComputedStyle, EngineError> {
        for (target, declarations) in self.slots[index as usize]
            .authored_pseudo
            .iter()
            .enumerate()
        {
            if let Some(declarations) = declarations {
                let canonical = Self::canonical_declarations(declarations)?;
                let mut pseudo = ComputedStyle::for_pseudo(&origin);
                pseudo.inherit_properties_from(&origin);
                let pseudo = self.resolve_native_declarations(
                    pseudo,
                    canonical.contributions(),
                    &origin,
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
        Ok(origin)
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
        metrics: Option<openui_style::FontRelativeLengthMetrics>,
    ) -> StyleValue {
        use openui_style::{Edges, Gap, LengthValue};
        let resolve = |value: LengthValue| {
            let length = if let Some(metrics) = metrics {
                value.resolve_with_font_metrics(viewport, font, root_font, metrics)
            } else {
                value.resolve(viewport, font, root_font)
            };
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
            StyleValue::Length(value)
                if property == StyleProperty::LineHeight && !value.is_valid_for_line_height() =>
            {
                StyleValue::Length(*value)
            }
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
            StyleValue::CornerRadii(value) => {
                StyleValue::CornerRadii(openui_style::CornerRadii(Edges {
                    top: resolve(value.0.top),
                    right: resolve(value.0.right),
                    bottom: resolve(value.0.bottom),
                    left: resolve(value.0.left),
                }))
            }
            StyleValue::Transform(value) => {
                StyleValue::Transform(openui_style::map_transform_lengths(value, &resolve))
            }
            _ => value.clone(),
        }
    }

    fn native_font_length_metrics(
        &self,
        style: &ComputedStyle,
    ) -> openui_style::FontRelativeLengthMetrics {
        use openui_text::{FontRelativeLengthResolver, FontRelativeUnit};
        let mut style = style.clone();
        style.update_derived(|fields| {
            fields.raster_configuration = self.raster_configuration;
            fields.device_scale_factor = self.viewport.device_scale_factor();
        });
        let units = FontRelativeLengthResolver::from_style_in_collection(
            &style,
            self.font_collection().clone(),
        );
        openui_style::FontRelativeLengthMetrics {
            ch: units.resolve(1.0, FontRelativeUnit::Ch),
            ex: units.resolve(1.0, FontRelativeUnit::Ex),
            lh: units.resolve(1.0, FontRelativeUnit::Lh),
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
                if let Some(control) = self.controls.get_mut(&handle.index) {
                    control.native_intrinsic_sizing = false;
                    control.intrinsic_capture_scrollbar_width = None;
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
                if let Some(control) = self.controls.get_mut(&handle.index) {
                    if value != Some(control.role) {
                        control.intrinsic_capture_scrollbar_width = None;
                    }
                }
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
                if self.document.node(node).scroll_left != value {
                    self.queue_snap_end_axis(handle, 0);
                    self.enqueue_scroll_notification(node);
                }
                self.document.node_mut(node).scroll_left = value;
            }
            RendererNodeState::ScrollTop(value) => {
                if !value.is_finite() {
                    return Err(EngineError::Render("scroll offset must be finite".into()));
                }
                if self.document.node(node).scroll_top != value {
                    self.queue_snap_end_axis(handle, 1);
                    self.enqueue_scroll_notification(node);
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
        let declarations_to_resolve = Self::canonical_declarations(declarations)?;
        let pseudo = self.resolve_native_declarations(
            pseudo,
            declarations_to_resolve.contributions(),
            &origin,
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
        if let Some(control) = self.controls.get_mut(&handle.index) {
            control.native_intrinsic_sizing = false;
            control.intrinsic_capture_scrollbar_width = None;
        }
        self.document.node_mut(node).replaced = Some(ReplacedContent {
            resource: ReplacedResourceKind::Image(resource),
            intrinsic_width,
            intrinsic_height,
            intrinsic_ratio,
        });
        self.mark_dirty(InvalidationClass::Intrinsic);
        Ok(())
    }

    /// Reflect one independent native radius axis from the current computed
    /// source. This does not reduce overlaps or substitute a layout-box size.
    pub fn radius_axis_length(
        &self,
        handle: NodeHandle,
        property: StyleProperty,
    ) -> Result<openui_style::LengthValue, EngineError> {
        let node = self.resolve(handle)?;
        self.document
            .node(node)
            .style
            .border_radius_axis_length(property)
            .ok_or(EngineError::InvalidInput(
                "property is not a native radius axis",
            ))
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
            if let Some(cursor) = self.slots[node.index as usize]
                .authored
                .contributions()
                .iter()
                .find_map(|contribution| {
                    if contribution
                        .fields()
                        .contains(openui_style::CanonicalStyleField::Cursor)
                    {
                        if let StyleValue::Cursor(cursor) = contribution.value() {
                            return Some(*cursor);
                        }
                    }
                    None
                })
            {
                return Ok(cursor);
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
        self.resolve(handle)?;
        let (x, y) = self.clamp_scroll_offset(handle, x, y)?;
        let (x, y) = self.resolve_native_scroll_endpoint(handle, x, y)?;
        self.scroll_to_resolved(handle, x, y)
    }

    pub(crate) fn scroll_to_resolved(
        &mut self,
        handle: NodeHandle,
        x: f32,
        y: f32,
    ) -> Result<(), EngineError> {
        let node = self.resolve(handle)?;
        self.scroll_buttons.pending_end_axes.remove(&handle);
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
        // Programmatic scrolling samples the same retained timeline without
        // advancing the document clock or calling application callbacks.
        self.sample_progress_animations_after_layout()?;
        Ok(())
    }

    fn invalidate_scroll(&mut self, node: NodeId) {
        self.enqueue_scroll_notification(node);
        // Sticky fragment positions currently resolve during layout. Invalidate
        // that dependency only when this scrollport owns a sticky descendant;
        // ordinary scrolling retains its existing compositor-only path.
        fn contains_sticky(fragment: &Fragment, document: &NativeDocument) -> bool {
            for child in &fragment.children {
                if child.owns_element_box()
                    && document.node(child.node_id).style.position == openui_style::Position::Sticky
                {
                    return true;
                }
                if child.owns_scroll_box()
                    && child
                        .scroll_area
                        .is_some_and(|area| area.is_scroll_container())
                {
                    continue;
                }
                if contains_sticky(child, document) {
                    return true;
                }
            }
            false
        }
        let sticky = self
            .latest_fragment
            .as_ref()
            .and_then(|root| root.find_scroll_box(node))
            .is_some_and(|box_| contains_sticky(box_, &self.document));
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
        let (x, y) = self.resolve_native_scroll_displacement(handle, dx, dy)?;
        self.scroll_to_resolved(handle, x, y)
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

    /// Style of the real retained formatting/scroll box. Anonymous content
    /// inherits only the source-defined properties, rather than every authored
    /// effect or scroll property of its semantic owner.
    fn retained_scroll_style(&self, node: NodeId) -> &ComputedStyle {
        let authored = &self.document.node(node).style;
        self.latest_fragment
            .as_ref()
            .and_then(|root| root.find_scroll_box(node))
            .map_or(authored, |box_| box_.layout_style(authored))
    }

    fn layout_scroll_area(&self, node: NodeId) -> Option<openui_layout::ScrollArea> {
        self.latest_fragment
            .as_ref()?
            .find_scroll_box(node)?
            .scroll_area
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

    /// Default wheel scrolling from a hit element through its retained box
    /// ancestry. Direct `scroll_wheel` remains an explicit operation on one
    /// element; this operation respects anonymous content membership.
    pub fn scroll_wheel_from_target(
        &mut self,
        target: NodeHandle,
        delta_x: f64,
        delta_y: f64,
    ) -> Result<bool, EngineError> {
        if !delta_x.is_finite() || !delta_y.is_finite() {
            return Err(EngineError::InvalidInput("wheel deltas must be finite"));
        }
        let target_node = self.resolve(target)?;
        self.update()?;
        let mut current = Some(target);
        while let Some(handle) = current {
            let owner = self.resolve(handle)?;
            let in_scroll_box = self
                .latest_fragment
                .as_ref()
                .is_some_and(|root| root.scroll_box_contains_element(owner, target_node));
            if in_scroll_box && self.scroll_wheel(handle, delta_x, delta_y)? {
                return Ok(true);
            }
            current = self.parent(handle)?;
        }
        Ok(false)
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
        self.sample_scroll_animations()?;
        if self
            .animations
            .values()
            .any(|instance| !instance.timeline.is_document())
        {
            self.update_once()?;
            self.sample_progress_animations_after_layout()?;
        }
        Ok(())
    }

    pub fn animation_time(&self) -> f64 {
        self.animation_time_ms
    }

    pub fn update(&mut self) -> Result<&SceneSnapshot, EngineError> {
        // Freeze one post-layout progress-timeline snapshot per native barrier.
        // A layout-affecting sample gets a second layout without recursively
        // re-snapshotting a feedback-dependent view timeline in this barrier.
        let snapshot_needed = self.dirty.visual() || self.latest_fragment.is_none();
        self.update_once()?;
        if snapshot_needed {
            self.sample_progress_animations_after_layout()?;
        }
        if self.dirty.visual() {
            self.update_once()?;
        }
        Ok(self.latest_scene.as_ref().expect("scene assigned"))
    }

    fn update_once(&mut self) -> Result<&SceneSnapshot, EngineError> {
        self.synchronize_font_collection()?;
        if !self.dirty.layout && self.dirty.visual() {
            fn source_policy_changed(document: &NativeDocument, fragment: &Fragment) -> bool {
                fragment
                    .authored_projection
                    .as_ref()
                    .is_some_and(|record| record.source_eligibility_changed(document, fragment))
                    || fragment
                        .children
                        .iter()
                        .any(|child| source_policy_changed(document, child))
            }
            if self
                .latest_fragment
                .as_ref()
                .is_some_and(|fragment| source_policy_changed(&self.document, fragment))
            {
                // A source-policy transition returns to the actual producer.
                // This check does not run for idle scenes or geometry reads.
                self.mark_dirty(InvalidationClass::Layout);
            } else {
                self.refresh_retained_scroll_overflow();
            }
        }
        if !self.dirty.layout && self.latest_fragment.is_some() {
            self.refresh_scroll_button_states();
        }
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
            self.relocate_owned_scroll_buttons()?;
            self.refresh_native_control_geometry();
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
                if let Some(area) = fragment.scroll_area.filter(|_| fragment.owns_scroll_box()) {
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
                    let data = self.document.node_mut(node);
                    data.scroll_left = x;
                    data.scroll_top = y;
                    self.enqueue_scroll_notification(node);
                }
                // Sticky descendants also consume the retained offsets.
                fragment =
                    openui_layout::block_layout(&self.document, self.document.root(), &space);
            }
            openui_layout::finalize_fragment_transforms(&self.document, &mut fragment);
            if self.settle_native_snap_after_layout(&fragment) {
                // Retained snap offsets and sticky descendants are recorded
                // from the same final-state layout, without shifting fixtures.
                fragment =
                    openui_layout::block_layout(&self.document, self.document.root(), &space);
            }
            openui_layout::finalize_fragment_transforms(&self.document, &mut fragment);
            let fragment = Arc::new(fragment);
            self.stats.layouts += 1;
            self.rebuild_hit_test(&fragment);
            self.latest_fragment = Some(fragment);
            self.scroll_buttons.dirty = true;
        }
        self.refresh_scroll_button_states();
        let fragment = self
            .latest_fragment
            .as_ref()
            .expect("visual dirtiness always establishes a fragment")
            .clone();
        let recording = if self.layered_paint_capture_enabled {
            record_fragment_with_program(&self.document, &fragment, self.viewport)
                .map(|capture| (capture.canonical.clone(), Some(capture)))
        } else {
            record_fragment(&self.document, &fragment, self.viewport)
                .map(|recording| (recording, None))
        };
        self.restore_closed_details_content(hidden_details_content);
        let (recording, capture) = recording.map_err(EngineError::Render)?;
        self.stats.paints += 1;
        let generation = SceneGeneration(self.stats.scenes + 1);
        let damage = vec![SceneRect {
            x: 0.0,
            y: 0.0,
            width: width as f32,
            height: height as f32,
        }];
        let retained = self.retained_scene_plan(&fragment, &recording)?;
        let scene = if let Some(capture) = capture {
            let capture = self.bind_layered_paint_capture(capture, &retained)?;
            SceneSnapshot::new_with_layered_capture(generation, capture, fragment, damage)
        } else {
            SceneSnapshot::new(generation, recording, fragment, damage)
        }
        .with_retained_scene(retained)
        .map_err(|error| match error {
            openui_compositor::RetainedSceneError::AllocationFailure => {
                EngineError::AllocationFailure
            }
            other => EngineError::Render(other.to_string()),
        })?;
        self.latest_scene = Some(scene);
        self.stats.scenes += 1;
        let accessibility = self.dirty.accessibility;
        self.dirty = DirtyState {
            accessibility,
            ..DirtyState::default()
        };
        Ok(self.latest_scene.as_ref().expect("scene assigned"))
    }

    /// Opt in to owned diagnostic paint partitions on the next visual update.
    /// This leaves ordinary canonical presentation selected. Layered replay and
    /// independent animation admission still require their exact runtime gates.
    pub fn set_layered_paint_capture_enabled(&mut self, enabled: bool) {
        if self.layered_paint_capture_enabled == enabled {
            return;
        }
        self.layered_paint_capture_enabled = enabled;
        self.mark_dirty(InvalidationClass::Paint);
    }

    pub fn layered_paint_capture_enabled(&self) -> bool {
        self.layered_paint_capture_enabled
    }

    pub fn scene(&mut self) -> Result<SceneSnapshot, EngineError> {
        Ok(self.update()?.clone())
    }

    pub fn hit_test(&mut self, x: f32, y: f32) -> Result<Option<NodeHandle>, EngineError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(EngineError::InvalidInput("hit coordinates must be finite"));
        }
        self.update()?;
        for entry in self.hit_test.iter().rev() {
            let (local_x, local_y) = entry.world_to_local.project_point(x, y);
            if rounded_rect_contains(local_x, local_y, entry.width, entry.height, [(0.0, 0.0); 4])
                && entry.clips.iter().all(|clip| clip.contains(x, y))
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

    /// Test connectivity without layout work or focus side effects.
    pub fn is_connected(&self, handle: NodeHandle) -> Result<bool, EngineError> {
        Ok(self.node_is_connected(self.resolve(handle)?))
    }

    fn node_is_connected(&self, mut node: NodeId) -> bool {
        // Origin-owned buttons can be absent from the presentation tree while
        // hidden. Their logical connection follows the live origin, so the
        // first geometry read after showing it still reaches the layout barrier.
        // Ordinary detached subtrees continue to skip unrelated layout work.
        let mut remaining = self.node_slots.len();
        while !node.is_none() {
            // Ownership and parent links must not turn malformed retained
            // state into an unbounded walk. No allocation is needed here.
            if remaining == 0 {
                return false;
            }
            remaining -= 1;
            let Some(index) = self.node_slots.get(&node).copied() else {
                return false;
            };
            if self.slots.get(index as usize).and_then(|slot| slot.node) != Some(node) {
                return false;
            }
            let handle = self.handle_for_slot(index);
            if let Some(binding) = self.scroll_buttons.bindings.get(&handle) {
                if binding.button != handle {
                    return false;
                }
                let Ok(origin) = self.resolve(binding.origin) else {
                    return false;
                };
                node = origin;
                continue;
            }
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
        fn walk(
            engine: &Engine,
            node: NodeId,
            out: &mut Vec<NodeId>,
            seen: &mut std::collections::HashSet<NodeId>,
        ) {
            if !seen.insert(node) {
                return;
            }
            out.push(node);
            for child in engine.document.children(node) {
                walk(engine, child, out, seen);
            }
            for button in engine.owned_buttons(node) {
                walk(engine, button, out, seen);
            }
        }
        walk(self, node, out, &mut std::collections::HashSet::new());
    }
    fn mark_dirty(&mut self, class: InvalidationClass) {
        self.scroll_buttons.dirty = true;
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
        fn walk(
            document: &NativeDocument,
            fragment: &Fragment,
            parent_world: WorldTransform,
            parent_paint_offset: openui_geometry::PhysicalOffset,
            inherited_hittable: bool,
            inherited_clips: &[HitClip],
            out: &mut Vec<HitEntry>,
            rects: &mut HashMap<NodeId, Vec<SceneRect>>,
            box_worlds: &mut HashMap<NodeId, WorldTransform>,
        ) {
            // CSS paint placement is snapped before authored matrix/scroll
            // application. It must not be derived from transformed world bounds.
            let paint_offset = openui_geometry::PhysicalOffset::new(
                parent_paint_offset.left + fragment.offset.left,
                parent_paint_offset.top + fragment.offset.top,
            );
            let width = fragment.size.width.to_f32();
            let height = fragment.size.height.to_f32();
            let border_box = fragment.border_box_rect();
            let box_width = border_box.size.width.to_f32();
            let box_height = border_box.size.height.to_f32();
            // Out-of-flow continuation records retain transforms whose real
            // ancestors are absent from this physical fragment subtree. Use
            // the same owned reference-box operation as painting.
            let mut promoted_world = parent_world;
            let mut ancestry_hittable = inherited_hittable;
            for ancestor in &fragment.promoted_transform_ancestors {
                promoted_world = promoted_world.then(WorldTransform(
                    openui_layout::promoted_ancestor_applied_transform(document, ancestor),
                ));
                ancestry_hittable &= promoted_world.is_invertible();
                // These are actual removed element boundaries. The current
                // flat style ends their inherited depth before the next box;
                // the inverse eligibility latch survives that flattening.
                promoted_world = promoted_world.flattened();
            }
            let translated = promoted_world.then(WorldTransform::translate(
                fragment.offset.left.to_f64(),
                fragment.offset.top.to_f64(),
            ));
            // Helpers retain a style source for decoration and structural
            // clips, but authored effects belong to the element's own box.
            let node_style =
                (!fragment.node_id.is_none()).then(|| &document.node(fragment.node_id).style);
            let owns_box = fragment.owns_authored_projection_box()
                && document.node(fragment.node_id).tag != ElementTag::Text;
            let world =
                if let Some(style) = node_style.filter(|style| owns_box && style.has_transform()) {
                    let resolved = fragment.resolved_transform.unwrap_or_else(|| {
                        openui_layout::resolved_fragment_transform(fragment, style)
                    });
                    translated.then(WorldTransform(resolved.local_applied))
                } else {
                    translated
                };
            let box_world = world.then(WorldTransform::translate(
                border_box.offset.left.to_f64(),
                border_box.offset.top.to_f64(),
            ));
            if owns_box {
                rects
                    .entry(fragment.node_id)
                    .or_default()
                    .push(box_world.map_rect(box_width, box_height));
                box_worlds.entry(fragment.node_id).or_insert(box_world);
            }
            // Blink rejects the complete CSS transform before plane
            // projection. A flat child must not revive a singular ancestor.
            // Geometry queries still retain all of the layout rectangles.
            let subtree_hittable = ancestry_hittable && world.is_invertible();
            let world_to_local = subtree_hittable.then(|| box_world.inverse()).flatten();
            let mut own_clips: Vec<_> = inherited_clips
                .iter()
                .filter(|clip| {
                    let may_cross = owns_box
                        && node_style.is_some_and(|style| style.has_transform())
                        && fragment
                            .authored_projection
                            .as_ref()
                            .is_some_and(|projection| {
                                clip.structural_projection.as_ref().is_some_and(|boundary| {
                                    projection.crosses_structural_boundary(boundary)
                                })
                            });
                    !may_cross
                })
                .cloned()
                .collect();
            if let Some(style) = node_style {
                if let Some(inset) = style.clip_path_inset.filter(|_| owns_box) {
                    let resolve = |length: &openui_geometry::Length, size: f32| {
                        if length.is_fixed() || length.is_percent() || length.is_calculated() {
                            // BasicShapeInset uses FloatValueForLength. Its
                            // calculated-length arithmetic retains the source
                            // kind and differs from a pure percent operation.
                            openui_style::transform_length_for_reference_box(*length, size)
                        } else {
                            0.0
                        }
                    };
                    let top = resolve(&inset[0], box_height);
                    let right = resolve(&inset[1], box_width);
                    let bottom = resolve(&inset[2], box_height);
                    let left = resolve(&inset[3], box_width);
                    let clipped_world =
                        box_world.then(WorldTransform::translate(left as f64, top as f64));
                    if let Some(inverse) = clipped_world.inverse() {
                        own_clips.push(HitClip {
                            world_to_local: inverse,
                            width: (box_width - left - right).max(0.0),
                            height: (box_height - top - bottom).max(0.0),
                            clip_x: true,
                            clip_y: true,
                            radii: [(0.0, 0.0); 4],
                            structural_projection: None,
                        });
                    }
                }
                let pointer_eligible = subtree_hittable
                    && owns_box
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
                // CSS overflow uses the final layout policy, including table,
                // replaced and viewport-propagated boxes. A visible axis must
                // continue accepting the descendant ink painted on that axis.
                let effective = fragment.layout_style(style);
                let geometry = openui_paint::compute_overflow_clip_geometry(
                    fragment,
                    effective,
                    document,
                    paint_offset,
                    openui_paint::OverflowClipContourPurpose::DirectFragmentHit,
                );
                let structural_projection = fragment
                    .authored_projection
                    .as_ref()
                    .filter(|projection| {
                        projection.is_structural_boundary()
                            && fragment.owns_element_box()
                            && openui_layout::used_overflow_pair(document, fragment.node_id)
                                == (
                                    openui_style::Overflow::Visible,
                                    openui_style::Overflow::Visible,
                                )
                            && !effective.has_paint_containment()
                    })
                    .cloned();
                let push_contour = |rect: (f32, f32, f32, f32),
                                    axes: (bool, bool),
                                    radii: [(f32, f32); 4],
                                    clips: &mut Vec<HitClip>| {
                    let clip_world = world.then(WorldTransform::translate(
                        rect.0 as f64 - paint_offset.left.to_f64(),
                        rect.1 as f64 - paint_offset.top.to_f64(),
                    ));
                    if let Some(world_to_local) = clip_world.inverse().filter(|_| axes.0 || axes.1)
                    {
                        clips.push(HitClip {
                            world_to_local,
                            width: rect.2,
                            height: rect.3,
                            clip_x: axes.0,
                            clip_y: axes.1,
                            radii,
                            structural_projection: structural_projection.clone(),
                        });
                    }
                };
                // The rectangular overflow bound includes its real axes/margin.
                // The pinned direct rounded inner check is a separate operation
                // with physical fragment size and all computed border sides.
                push_contour(
                    geometry.rect,
                    (geometry.clips_x, geometry.clips_y),
                    [(0.0, 0.0); 4],
                    &mut own_clips,
                );
                if let Some(rounded) = geometry.rounded {
                    push_contour(rounded.rect, (true, true), rounded.radii, &mut own_clips);
                }
            }
            // Authored effects keep their ordinary flattening boundary even
            // when scroll state belongs to an anonymous content descendant.
            let child_world = if owns_box { world.flattened() } else { world };
            let child_world = if fragment.owns_scroll_box() {
                let node = document.node(fragment.node_id);
                child_world.then(WorldTransform::translate(
                    -node.scroll_left as f64,
                    -node.scroll_top as f64,
                ))
            } else {
                child_world
            };
            let mut children: Vec<_> = fragment.children.iter().enumerate().collect();
            children.sort_by_key(|(order, child)| {
                let z = if !child.owns_element_box() {
                    0
                } else {
                    document.node(child.node_id).style.z_index.unwrap_or(0)
                };
                (z, *order)
            });
            let child_entries_start = out.len();
            let mut paint_ranges = Vec::with_capacity(children.len());
            for (order, child) in children {
                let (z, phase) = if !child.owns_element_box() {
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
                    paint_offset,
                    subtree_hittable,
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
            WorldTransform::IDENTITY,
            openui_geometry::PhysicalOffset::zero(),
            true,
            &[],
            &mut entries,
            &mut rects,
            &mut box_worlds,
        );
        self.hit_test = entries;
        self.fragment_rects = rects;
        // Client quads use Chromium's source-box/container mapping, while
        // physical hit entries and their clips keep the actual fragment matrix.
        self.fragment_rects
            .extend(client_geometry::fragment_client_rects(
                &self.document,
                fragment,
            ));
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
mod semantic_insertion_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use openui_compositor::SoftwareCompositor;
    use openui_style::{
        Color, CornerRadii, Display, Edges, LengthValue, Overflow, TransformList,
        TransformOperation,
    };

    fn font_unit_engine() -> Engine {
        let mut engine = Engine::new_with_font_collection_and_options(
            ViewportMetrics::from_logical_size(640.0, 480.0, 1.0).unwrap(),
            FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration: RasterConfiguration::deterministic_aliased(false),
            },
        )
        .unwrap();
        engine
            .set_property(
                engine.root(),
                StyleProperty::FontFamily,
                openui_style::FontFamilyList {
                    families: vec![openui_style::FontFamily::Named("Ahem".into())],
                }
                .into(),
            )
            .unwrap();
        engine
            .set_property(
                engine.root(),
                StyleProperty::FontSize,
                LengthValue::px(20.0).into(),
            )
            .unwrap();
        engine
    }

    #[test]
    fn native_font_units_retain_nested_lengths_and_final_font_context() {
        use openui_style::{FontFamily, FontFamilyList, Gap, LineHeight};
        let mut engine = font_unit_engine();
        let node = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::Ch(2.0).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::Lh(2.0).into())
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::Padding,
                Edges::all(LengthValue::Ex(1.0)).into(),
            )
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::Gap,
                Gap {
                    row: LengthValue::Lh(1.0),
                    column: LengthValue::Ch(1.0),
                }
                .into(),
            )
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::BorderRadius,
                CornerRadii(Edges::all(LengthValue::Ch(0.5))).into(),
            )
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::Transform,
                TransformList(vec![TransformOperation::Translate(
                    LengthValue::Ch(1.0),
                    LengthValue::Ex(1.0),
                )])
                .into(),
            )
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::FontFamily,
                FontFamilyList {
                    families: vec![FontFamily::Named("Ahem".into())],
                }
                .into(),
            )
            .unwrap();
        engine
            .set_property(
                node,
                StyleProperty::LineHeight,
                LineHeight::Number(1.5).into(),
            )
            .unwrap();
        engine
            .set_property(node, StyleProperty::FontSize, LengthValue::px(30.0).into())
            .unwrap();
        let style = engine.computed_style(node).unwrap();
        assert_eq!(style.width.value(), 60.0);
        assert_eq!(style.height.value(), 90.0);
        assert_eq!(style.padding_top.value(), 24.0);
        assert_eq!(style.row_gap.unwrap().value(), 45.0);
        assert_eq!(style.column_gap.unwrap().value(), 30.0);
        assert_eq!(style.border_top_left_radius, (15.0, 15.0));
        assert_eq!(style.transform.e, 30.0);
        assert_eq!(style.transform.f, 24.0);
    }

    #[test]
    fn native_font_units_font_size_and_pseudos_use_parent_metrics() {
        let mut engine = font_unit_engine();
        let node = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::FontSize, LengthValue::Ex(2.0).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::Ch(2.0).into())
            .unwrap();
        engine
            .set_pseudo_style(
                node,
                PseudoStyleTarget::FirstLine,
                &Style::default()
                    .font_size(LengthValue::Ch(2.0))
                    .width(LengthValue::Ch(3.0)),
            )
            .unwrap();
        assert_eq!(engine.computed_style(node).unwrap().font_size, 32.0);
        assert_eq!(engine.computed_style(node).unwrap().width.value(), 64.0);
        assert_eq!(
            engine
                .computed_style(node)
                .unwrap()
                .first_line_style
                .as_ref()
                .unwrap()
                .font_size,
            64.0
        );
        engine
            .set_property(
                engine.root(),
                StyleProperty::FontSize,
                LengthValue::px(30.0).into(),
            )
            .unwrap();
        let style = engine.computed_style(node).unwrap();
        assert_eq!(style.font_size, 48.0);
        assert_eq!(style.width.value(), 96.0);
        assert_eq!(style.first_line_style.as_ref().unwrap().font_size, 96.0);
        assert_eq!(
            style.first_line_style.as_ref().unwrap().width.value(),
            288.0
        );
    }

    #[test]
    fn native_font_unit_animation_reacts_to_ancestor_font_changes() {
        use openui_style::{AnimationOptions, FillMode, Keyframes, PropertyKeyframes};
        let mut engine = font_unit_engine();
        let node = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Width, LengthValue::Ch(1.0).into())
            .unwrap();
        let animation = engine
            .animate(
                node,
                PropertyKeyframes::typed(
                    StyleProperty::Width,
                    Keyframes::from_values(LengthValue::Ch(2.0), LengthValue::Ch(4.0)),
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
        engine.set_animation_time(50.0).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().width.value(), 60.0);
        engine
            .set_property(
                engine.root(),
                StyleProperty::FontSize,
                LengthValue::px(30.0).into(),
            )
            .unwrap();
        assert_eq!(engine.computed_style(node).unwrap().width.value(), 90.0);
        engine.cancel_animation(animation).unwrap();
        assert_eq!(engine.computed_style(node).unwrap().width.value(), 30.0);
    }

    #[test]
    fn native_font_unit_repeated_setter_preserves_unchanged_frame_work() {
        let mut engine = font_unit_engine();
        let node = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), node).unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::Lh(2.0).into())
            .unwrap();
        engine.scene().unwrap();
        let stats = engine.stats();
        let generations = engine.dirty_generations();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::Lh(2.0).into())
            .unwrap();
        engine.scene().unwrap();
        assert_eq!(engine.stats(), stats);
        assert_eq!(engine.dirty_generations(), generations);
    }

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
    fn native_control_attribute_no_op_does_no_work_and_explicit_metadata_survives() {
        let mut engine = Engine::new_with_font_collection(
            ViewportMetrics::from_logical_size(640.0, 360.0, 1.0).unwrap(),
            FontCollection::deterministic_test(),
        )
        .unwrap();
        let input = engine.create_native_element(ElementTag::Input).unwrap();
        engine.append_child(engine.root(), input).unwrap();
        engine.set_attribute(input, "size", "7").unwrap();
        engine.scene().unwrap();
        let stats = engine.stats();
        engine.set_attribute(input, "size", "7").unwrap();
        engine.scene().unwrap();
        assert_eq!(stats, engine.stats());

        engine
            .set_renderer_node_state(
                input,
                RendererNodeState::Replaced(Some(ReplacedContent {
                    resource: ReplacedResourceKind::TransparentCanvas,
                    intrinsic_width: Some(80.0),
                    intrinsic_height: Some(30.0),
                    intrinsic_ratio: None,
                })),
            )
            .unwrap();
        engine.set_attribute(input, "size", "3").unwrap();
        let bounds = engine.bounds(input).unwrap().unwrap();
        assert_eq!((bounds.width, bounds.height), (80.0, 30.0));
        let clone = engine.clone_subtree(input).unwrap();
        engine.append_child(engine.root(), clone).unwrap();
        engine.set_attribute(clone, "size", "9").unwrap();
        let bounds = engine.bounds(clone).unwrap().unwrap();
        assert_eq!((bounds.width, bounds.height), (80.0, 30.0));
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

    #[test]
    fn owned_button_connectivity_rejects_cycles_and_invalid_origin_handles() {
        let mut engine =
            Engine::new(ViewportMetrics::from_logical_size(80.0, 60.0, 1.0).unwrap()).unwrap();
        let origin = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), origin).unwrap();
        let button = engine
            .create_scroll_button(origin, ScrollButtonDirection::Right)
            .unwrap();
        let child = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(button, child).unwrap();
        engine.update().unwrap();
        let valid_binding = engine.scroll_buttons.bindings[&button];
        let baseline = engine.stats();
        // The child-to-button parent edge makes this a two-node ownership cycle.
        for invalid_origin in [
            button,
            child,
            NodeHandle {
                generation: origin.generation.wrapping_add(1),
                ..origin
            },
            NodeHandle {
                document: engine.document_id().wrapping_add(1),
                ..origin
            },
        ] {
            engine
                .scroll_buttons
                .bindings
                .get_mut(&button)
                .unwrap()
                .origin = invalid_origin;
            assert!(!engine.is_connected(button).unwrap());
            assert!(!engine.is_connected(child).unwrap());
            assert!(engine.bounds(button).unwrap().is_none());
            assert!(engine.client_rects(child).unwrap().is_empty());
            assert_eq!(engine.stats(), baseline);
        }
        engine.scroll_buttons.bindings.insert(button, valid_binding);
        assert!(engine.is_connected(button).unwrap());

        // Reusing a removed slot must not validate the old origin generation.
        let removed = engine.create_native_element(ElementTag::Div).unwrap();
        engine.remove(removed).unwrap();
        let replacement = engine.create_native_element(ElementTag::Div).unwrap();
        assert_eq!(replacement.index(), removed.index());
        assert_ne!(replacement.generation(), removed.generation());
        engine.append_child(engine.root(), replacement).unwrap();
        engine
            .scroll_buttons
            .bindings
            .get_mut(&button)
            .unwrap()
            .origin = removed;
        let baseline = engine.stats();
        assert!(!engine.is_connected(button).unwrap());
        assert!(engine.bounds(child).unwrap().is_none());
        assert_eq!(engine.stats(), baseline);
        engine.scroll_buttons.bindings.insert(button, valid_binding);

        engine
            .scroll_buttons
            .bindings
            .get_mut(&button)
            .unwrap()
            .button = origin;
        assert!(!engine.is_connected(button).unwrap());
        engine.scroll_buttons.bindings.insert(button, valid_binding);
        engine.remove(origin).unwrap();
        assert!(engine.is_connected(button).is_err());
        assert!(engine.is_connected(child).is_err());
    }
}
