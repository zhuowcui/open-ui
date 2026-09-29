//! Arena-based element tree with parent/child relationships.
//!
//! Uses a simple Vec<Node> arena indexed by `NodeId`. This is similar to
//! how Blink stores nodes — a flat arena with pointer-like indices for
//! parent, first_child, last_child, next_sibling, prev_sibling.

use std::collections::{BTreeMap, HashMap};

use openui_geometry::RasterConfiguration;
use openui_style::{
    apply_to_computed, Color, ComputedStyle, ContainerCondition, Containment, CounterStyle,
    Display, FontFamily, GeneratedContentItem, GenericFontFamily, ImageResourceId, Overflow,
    OverflowClipBox, PropertyTypeError, QuotePair, RendererInternalStyleValue, ScrollMarkerGroup,
    Style, StyleProperty, StyleValue,
};

/// Encoded raster or static-SVG bytes owned by a document.
///
/// The registry deliberately stores only transport metadata and bytes. Image
/// decoding and backend caches belong to paint, while style/layout retain a
/// stable [`ImageResourceId`].
#[derive(Debug, Clone)]
pub struct EncodedImageResource {
    pub source: String,
    pub mime_type: String,
    pub sha256: String,
    pub bytes: Vec<u8>,
}

/// Deterministic replaced-resource role. Bytes remain in the document image
/// registry; this metadata supplies intrinsic dimensions and element behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplacedResourceKind {
    Image(ImageResourceId),
    StaticSvg(ImageResourceId),
    TransparentCanvas,
    /// A poster image or hash-pinned decoded first frame for a media element.
    MediaPoster(ImageResourceId),
    PackagedDocument(ImageResourceId),
}

#[derive(Debug, Clone, Copy)]
pub struct ReplacedContent {
    pub resource: ReplacedResourceKind,
    pub intrinsic_width: Option<f32>,
    pub intrinsic_height: Option<f32>,
    pub intrinsic_ratio: Option<(f32, f32)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormControlRole {
    Button,
    TextInput,
    ColorInput,
    DateInput,
    FileInput,
    Checkbox,
    Radio,
    TextArea,
    Select,
    Option,
    OptGroup,
    Range,
    Meter,
    Progress,
    Fieldset,
    Legend,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ScrollButtonDirection {
    Up,
    Right,
    Down,
    Left,
    BlockStart,
    BlockEnd,
    InlineStart,
    InlineEnd,
}

/// Properties a static container-query result may replace after phase one.
/// The matched style is a complete typed value; this list limits copying to
/// declarations actually present in the query rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerQueryProperty {
    Display,
    Width,
    Height,
    MinWidth,
    MinHeight,
    MaxWidth,
    MaxHeight,
    Margin,
    Padding,
    Background,
    Color,
    Columns,
    GridTemplateColumns,
    GridTemplateRows,
}

#[derive(Debug, Clone)]
pub struct ContainerQueryRule {
    pub container_name: Option<String>,
    pub condition: ContainerCondition,
    pub matched_style: ComputedStyle,
    pub properties: Vec<ContainerQueryProperty>,
}

/// Opaque handle into the node arena.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) u32);

impl NodeId {
    pub const NONE: Self = Self(u32::MAX);

    #[inline]
    pub fn is_none(self) -> bool {
        self.0 == u32::MAX
    }

    #[inline]
    pub fn index(self) -> usize {
        self.0 as usize
    }
}

/// What kind of element this node represents.
/// We don't need a full tag enum — the layout algorithm only cares about
/// display type (from ComputedStyle) and whether this is text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ElementTag {
    /// A generic container element (like HTML `<div>`).
    #[default]
    Div,
    /// An inline container (like HTML `<span>`).
    Span,
    /// A text run (leaf node, no children).
    Text,
    /// A semantic forced line break (`<br>`).
    Break,
    /// A semantic soft line-break opportunity (`<wbr>`).
    WordBreak,
    /// An HTML ruby container. Ruby base and annotation content form one
    /// atomic inline-level formatting object.
    Ruby,
    /// HTML ruby annotation content (`<rt>`).
    RubyText,
    Table,
    TableCaption,
    TableColumnGroup,
    TableColumn,
    TableHead,
    TableBody,
    TableFoot,
    TableRow,
    TableCell,
    TableHeaderCell,
    Image,
    Canvas,
    Svg,
    IFrame,
    Object,
    Audio,
    Video,
    Input,
    Button,
    Meter,
    Progress,
    Fieldset,
    Legend,
    Details,
    Summary,
    TextArea,
    Select,
    Option,
    OptGroup,
    Form,
    Embed,
    /// The document element (`<html>`).
    Html,
    /// The document body (`<body>`).
    Body,
    /// An author style element whose text became renderable through CSS.
    Style,
    /// The root viewport element.
    Viewport,
}

/// Generated pseudo-element identity stored on an ordinary arena node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PseudoElementKind {
    Before,
    After,
    Marker,
    ScrollMarker,
    ScrollMarkerGroup,
    ScrollButton(ScrollButtonDirection),
    Column,
    ColumnScrollMarker,
    DetailsContent,
}

/// Data stored for each node in the tree.
#[derive(Clone)]
pub struct NodeData {
    pub tag: ElementTag,
    pub style: ComputedStyle,

    /// Identity and originating element for generated boxes. Pseudo boxes
    /// otherwise participate in layout and paint exactly like authored nodes.
    pub pseudo_kind: Option<PseudoElementKind>,
    pub pseudo_origin: NodeId,

    /// HTML attributes used by generated `attr()` values.
    pub attributes: BTreeMap<String, String>,

    /// Current scroll position in CSS pixels. Layout uses it for sticky
    /// constraints and paint translates scrollable descendants by it.
    pub scroll_left: f32,
    pub scroll_top: f32,

    /// HTML table span metadata. Values are normalized to at least one by the
    /// porter; row span zero remains representable for HTML's "to row-group
    /// end" behavior.
    pub table_col_span: u32,
    pub table_row_span: u32,

    /// Optional deterministic intrinsic/replaced element state.
    pub replaced: Option<ReplacedContent>,
    /// Original encoded HTML for a statically lowered embedded document.
    /// Its generated child viewport supplies layout and paint without a live
    /// browsing context or script runtime.
    pub embedded_document: Option<ImageResourceId>,
    /// Propagated canvas color of a statically lowered nested document. This
    /// paints inside the iframe content viewport, independently of the host
    /// element's own CSS background layers.
    pub embedded_canvas_color: Option<Color>,
    pub form_control: Option<FormControlRole>,
    /// Whether the platform-native form-control appearance remains enabled
    /// after the authored `appearance` cascade.
    pub form_control_native_appearance: bool,
    /// Whether the HTML control is disabled. Native appearance and text
    /// colors consume this state without a script/event runtime.
    pub form_control_disabled: bool,

    /// Whether this node is an SVG `foreignObject` graphics element. Its CSS
    /// box participates in block layout, while SVG viewport clipping remains
    /// observable during decoration paint.
    pub is_svg_foreign_object: bool,

    /// Base marker color retained when a generated column marker's single
    /// pseudo node is expanded into multiple virtual marker boxes. The first
    /// box may use `:target-current`; later boxes paint this inactive color.
    pub scroll_marker_inactive_background: Option<openui_style::Color>,

    /// Static size-container queries evaluated after the first layout phase.
    pub container_query_rules: Vec<ContainerQueryRule>,

    // Tree pointers (arena indices)
    pub parent: NodeId,
    pub first_child: NodeId,
    pub last_child: NodeId,
    pub next_sibling: NodeId,
    pub prev_sibling: NodeId,

    /// For text nodes: the text content.
    pub text: Option<String>,

    /// User-defined debug label (optional, for test output).
    pub label: Option<String>,
}

impl NodeData {
    fn new(tag: ElementTag) -> Self {
        let mut style = ComputedStyle::initial();
        if matches!(
            tag,
            ElementTag::Image
                | ElementTag::Canvas
                | ElementTag::Video
                | ElementTag::IFrame
                | ElementTag::Embed
                | ElementTag::Object
        ) {
            // Blink's UA stylesheet clips replaced hosts at their content
            // box. Apply the same default for native Rust and C elements;
            // generated WPT builders also record it explicitly.
            style.update_derived(|fields| {
                fields.overflow_x = Overflow::Clip;
                fields.overflow_y = Overflow::Clip;
                fields.overflow_clip_box = OverflowClipBox::ContentBox;
            });
        }
        Self {
            tag,
            style,
            pseudo_kind: None,
            pseudo_origin: NodeId::NONE,
            attributes: BTreeMap::new(),
            scroll_left: 0.0,
            scroll_top: 0.0,
            table_col_span: 1,
            table_row_span: 1,
            replaced: None,
            embedded_document: None,
            embedded_canvas_color: None,
            form_control: None,
            form_control_native_appearance: true,
            form_control_disabled: false,
            is_svg_foreign_object: false,
            scroll_marker_inactive_background: None,
            container_query_rules: Vec::new(),
            parent: NodeId::NONE,
            first_child: NodeId::NONE,
            last_child: NodeId::NONE,
            next_sibling: NodeId::NONE,
            prev_sibling: NodeId::NONE,
            text: None,
            label: None,
        }
    }
}

/// The document tree — an arena of nodes.
///
/// This is the native equivalent of Blink's `Document` + DOM tree, but
/// without any parsing, events, or script execution. It's just a tree.
#[derive(Clone)]
pub struct Document {
    nodes: Vec<NodeData>,
    free_nodes: Vec<u32>,
    root: NodeId,
    image_resources: Vec<EncodedImageResource>,
    legacy_canvas_body: Option<NodeId>,
    font_collection: std::sync::Arc<openui_text::FontCollection>,
    raster_configuration: RasterConfiguration,
    device_scale_factor: f64,
}

impl Document {
    /// Create a new document with a root viewport element.
    pub fn new() -> Self {
        Self::new_with_font_collection(openui_text::FontCollection::system())
    }

    /// Create a document with an explicit application/system font registry.
    pub fn new_with_font_collection(
        font_collection: std::sync::Arc<openui_text::FontCollection>,
    ) -> Self {
        Self::new_with_raster_configuration(font_collection, RasterConfiguration::default(), 1.0)
    }

    /// Create a document with explicit immutable raster context.
    pub fn new_with_raster_configuration(
        font_collection: std::sync::Arc<openui_text::FontCollection>,
        raster_configuration: RasterConfiguration,
        device_scale_factor: f64,
    ) -> Self {
        let mut doc = Self {
            nodes: Vec::new(),
            free_nodes: Vec::new(),
            root: NodeId::NONE,
            image_resources: Vec::new(),
            legacy_canvas_body: None,
            font_collection,
            raster_configuration,
            device_scale_factor,
        };
        let root_id = doc.create_node(ElementTag::Viewport);
        doc.root = root_id;
        // The viewport is a block-level element.
        doc.nodes[root_id.index()].style = ComputedStyle::for_viewport();
        doc.nodes[root_id.index()]
            .style
            .set_raster_context(raster_configuration, device_scale_factor);
        doc
    }

    pub fn raster_configuration(&self) -> RasterConfiguration {
        self.raster_configuration
    }

    pub fn device_scale_factor(&self) -> f64 {
        self.device_scale_factor
    }

    /// Update the engine-owned raster context after a viewport scale change.
    #[doc(hidden)]
    pub fn set_raster_context(
        &mut self,
        raster_configuration: RasterConfiguration,
        device_scale_factor: f64,
    ) {
        self.raster_configuration = raster_configuration;
        self.device_scale_factor = device_scale_factor;
        for node in &mut self.nodes {
            node.style
                .set_raster_context(raster_configuration, device_scale_factor);
        }
    }

    /// Apply one schema-validated author declaration at the DOM/style
    /// boundary. No mutable computed-style reference is returned.
    pub fn apply_style_property(
        &mut self,
        node: NodeId,
        property: StyleProperty,
        value: &StyleValue,
        viewport: (f32, f32),
    ) -> Result<openui_style::InvalidationClass, PropertyTypeError> {
        apply_to_computed(
            &mut self.nodes[node.index()].style,
            property,
            value,
            viewport,
        )
    }

    /// Apply an ordered typed declaration list through the same boundary.
    pub fn apply_style(
        &mut self,
        node: NodeId,
        declarations: &Style,
        viewport: (f32, f32),
    ) -> Result<(), PropertyTypeError> {
        for declaration in declarations.declarations() {
            self.apply_style_property(node, declaration.property, &declaration.value, viewport)?;
        }
        Ok(())
    }

    /// Install a style snapshot produced for an anonymous layout box. Author
    /// declarations must use `apply_style` or `apply_style_property`.
    #[doc(hidden)]
    pub fn install_resolved_style(&mut self, node: NodeId, mut style: ComputedStyle) {
        style.set_raster_context(self.raster_configuration, self.device_scale_factor);
        self.nodes[node.index()].style = style;
    }

    /// Update an engine-derived snapshot in place without returning a mutable
    /// computed-style reference. This is for anonymous boxes and low-level
    /// layout construction; authored declarations must use the schema APIs.
    #[doc(hidden)]
    pub fn update_resolved_style(
        &mut self,
        node: NodeId,
        update: impl FnOnce(&mut openui_style::ComputedStyleFields),
    ) {
        self.nodes[node.index()].style.update_derived(update);
    }

    /// Apply a generated engine-owned style value at the DOM/style boundary.
    /// These values are never author declarations and therefore have no
    /// public property ID, but their mutation is still kept inside the DOM.
    #[doc(hidden)]
    pub fn apply_internal_style(&mut self, node: NodeId, value: RendererInternalStyleValue) {
        value.apply_to(&mut self.nodes[node.index()].style);
    }

    pub fn font_collection(&self) -> &std::sync::Arc<openui_text::FontCollection> {
        &self.font_collection
    }

    pub fn resolve_font(&self, description: openui_text::FontDescription) -> openui_text::Font {
        openui_text::Font::new_in_collection(
            description,
            std::sync::Arc::clone(&self.font_collection),
        )
    }

    /// Replace the collection before layout; existing fragments remain tied
    /// to the typefaces with which they were shaped.
    pub fn set_font_collection(
        &mut self,
        font_collection: std::sync::Arc<openui_text::FontCollection>,
    ) {
        self.font_collection = font_collection;
    }

    /// The root viewport node.
    #[inline]
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Create a new detached node (not yet in the tree).
    pub fn create_node(&mut self, tag: ElementTag) -> NodeId {
        let node = if let Some(index) = self.free_nodes.pop() {
            self.nodes[index as usize] = NodeData::new(tag);
            NodeId(index)
        } else {
            let id = NodeId(self.nodes.len() as u32);
            self.nodes.push(NodeData::new(tag));
            id
        };
        self.nodes[node.index()]
            .style
            .set_raster_context(self.raster_configuration, self.device_scale_factor);
        node
    }

    /// Append `child` as the last child of `parent`.
    ///
    /// # Panics
    /// Panics if `child` already has a parent, if `child == parent`,
    /// or if `parent` is a descendant of `child` (would create a cycle).
    pub fn append_child(&mut self, parent: NodeId, child: NodeId) {
        assert!(
            self.nodes[child.index()].parent.is_none(),
            "append_child: node already has a parent — detach it first"
        );
        assert!(
            child != parent,
            "append_child: cannot append a node to itself"
        );
        // Walk ancestors of parent to ensure child is not among them.
        // This prevents ancestor→descendant cycles.
        {
            let mut ancestor = parent;
            while !ancestor.is_none() {
                assert!(
                    ancestor != child,
                    "append_child: parent is a descendant of child — would create cycle"
                );
                ancestor = self.nodes[ancestor.index()].parent;
            }
        }

        self.nodes[child.index()].parent = parent;
        self.nodes[child.index()].next_sibling = NodeId::NONE;

        let last = self.nodes[parent.index()].last_child;
        if last.is_none() {
            // First child
            self.nodes[parent.index()].first_child = child;
            self.nodes[child.index()].prev_sibling = NodeId::NONE;
        } else {
            // Append after last
            self.nodes[last.index()].next_sibling = child;
            self.nodes[child.index()].prev_sibling = last;
        }
        self.nodes[parent.index()].last_child = child;
    }

    /// Detach a node from its parent while retaining its arena allocation.
    ///
    /// Arena entries are intentionally never reused here. Higher-level owners
    /// may layer generation-checked handles over the stable `NodeId` storage.
    pub fn detach(&mut self, child: NodeId) -> bool {
        if child == self.root || child.index() >= self.nodes.len() {
            return false;
        }
        let parent = self.nodes[child.index()].parent;
        if parent.is_none() {
            return false;
        }
        let previous = self.nodes[child.index()].prev_sibling;
        let next = self.nodes[child.index()].next_sibling;
        if previous.is_none() {
            self.nodes[parent.index()].first_child = next;
        } else {
            self.nodes[previous.index()].next_sibling = next;
        }
        if next.is_none() {
            self.nodes[parent.index()].last_child = previous;
        } else {
            self.nodes[next.index()].prev_sibling = previous;
        }
        self.nodes[child.index()].parent = NodeId::NONE;
        self.nodes[child.index()].prev_sibling = NodeId::NONE;
        self.nodes[child.index()].next_sibling = NodeId::NONE;
        true
    }

    /// Return a detached leaf node's arena entry to the reusable pool.
    ///
    /// This is intentionally explicit: legacy layout builders may retain
    /// stable `NodeId` values and never call it. Generation-checked owners
    /// such as `openui-engine` can release an already-invalidated subtree in
    /// postorder and safely reuse its storage.
    pub fn release_detached_node(&mut self, node: NodeId) -> bool {
        if node == self.root || node.index() >= self.nodes.len() {
            return false;
        }
        let data = &self.nodes[node.index()];
        if !data.parent.is_none() || !data.first_child.is_none() || !data.last_child.is_none() {
            return false;
        }
        if self.free_nodes.contains(&node.0) {
            return false;
        }
        self.nodes[node.index()] = NodeData::new(ElementTag::Div);
        self.free_nodes.push(node.0);
        true
    }

    /// Insert `child` before the current first child of `parent`.
    pub fn prepend_child(&mut self, parent: NodeId, child: NodeId) {
        assert!(self.nodes[child.index()].parent.is_none());
        assert_ne!(parent, child);
        let first = self.nodes[parent.index()].first_child;
        self.nodes[child.index()].parent = parent;
        self.nodes[child.index()].prev_sibling = NodeId::NONE;
        self.nodes[child.index()].next_sibling = first;
        if first.is_none() {
            self.nodes[parent.index()].last_child = child;
        } else {
            self.nodes[first.index()].prev_sibling = child;
        }
        self.nodes[parent.index()].first_child = child;
    }

    /// Insert a detached node immediately before an attached sibling.
    pub fn insert_before(&mut self, sibling: NodeId, child: NodeId) {
        assert!(self.nodes[child.index()].parent.is_none());
        let parent = self.nodes[sibling.index()].parent;
        assert!(!parent.is_none());
        let previous = self.nodes[sibling.index()].prev_sibling;
        self.nodes[child.index()].parent = parent;
        self.nodes[child.index()].prev_sibling = previous;
        self.nodes[child.index()].next_sibling = sibling;
        self.nodes[sibling.index()].prev_sibling = child;
        if previous.is_none() {
            self.nodes[parent.index()].first_child = child;
        } else {
            self.nodes[previous.index()].next_sibling = child;
        }
    }

    /// Insert a detached node immediately after an attached sibling.
    fn insert_after_sibling(&mut self, sibling: NodeId, child: NodeId) {
        assert!(self.nodes[child.index()].parent.is_none());
        let parent = self.nodes[sibling.index()].parent;
        assert!(!parent.is_none());
        let next = self.nodes[sibling.index()].next_sibling;
        self.nodes[child.index()].parent = parent;
        self.nodes[child.index()].prev_sibling = sibling;
        self.nodes[child.index()].next_sibling = next;
        self.nodes[sibling.index()].next_sibling = child;
        if next.is_none() {
            self.nodes[parent.index()].last_child = child;
        } else {
            self.nodes[next.index()].prev_sibling = child;
        }
    }

    /// Create and attach an ordinary arena node for `::before` or `::after`.
    /// CSS tree order is guaranteed even when both pseudo boxes are present.
    pub fn insert_pseudo_element(&mut self, origin: NodeId, kind: PseudoElementKind) -> NodeId {
        let pseudo = self.create_node(ElementTag::Span);
        self.nodes[pseudo.index()].pseudo_kind = Some(kind);
        self.nodes[pseudo.index()].pseudo_origin = origin;
        match kind {
            PseudoElementKind::Before | PseudoElementKind::Marker => {
                self.prepend_child(origin, pseudo)
            }
            // A scroll-marker group is an external box immediately before or
            // after the originating scroll container.  Keep its origin link
            // for virtual marker collection, but put it in the parent's flow
            // so it contributes size and is not clipped by the scrollport.
            PseudoElementKind::ScrollMarkerGroup
                if !self.nodes[origin.index()].parent.is_none()
                    && self.nodes[origin.index()].style.scroll_marker_group
                        == ScrollMarkerGroup::Before =>
            {
                self.insert_before(origin, pseudo)
            }
            PseudoElementKind::ScrollMarkerGroup
                if self.nodes[origin.index()].parent.is_none()
                    && self.nodes[origin.index()].style.scroll_marker_group
                        == ScrollMarkerGroup::Before =>
            {
                self.prepend_child(origin, pseudo)
            }
            PseudoElementKind::ScrollMarkerGroup
                if !self.nodes[origin.index()].parent.is_none() =>
            {
                // In the external pseudo tree, scroll buttons precede an
                // `after` marker group. Generation may discover the group
                // after its buttons, so insert beyond the complete button
                // run rather than immediately after the principal box.
                let mut sibling = origin;
                let mut next = self.nodes[sibling.index()].next_sibling;
                while !next.is_none()
                    && matches!(
                        self.nodes[next.index()].pseudo_kind,
                        Some(PseudoElementKind::ScrollButton(_))
                    )
                    && self.nodes[next.index()].pseudo_origin == origin
                {
                    sibling = next;
                    next = self.nodes[sibling.index()].next_sibling;
                }
                self.insert_after_sibling(sibling, pseudo)
            }
            // Scroll buttons are siblings of the scroll container's
            // principal box. Size containment and overflow clipping on the
            // scrollport therefore cannot suppress the passive control.
            PseudoElementKind::ScrollButton(_) if !self.nodes[origin.index()].parent.is_none() => {
                self.insert_after_sibling(origin, pseudo)
            }
            PseudoElementKind::After
            | PseudoElementKind::ScrollMarker
            | PseudoElementKind::ScrollButton(_)
            | PseudoElementKind::Column
            | PseudoElementKind::ColumnScrollMarker
            | PseudoElementKind::ScrollMarkerGroup
            | PseudoElementKind::DetailsContent => self.append_child(origin, pseudo),
        }
        pseudo
    }

    pub fn set_attribute(
        &mut self,
        node: NodeId,
        name: impl Into<String>,
        value: impl Into<String>,
    ) {
        self.nodes[node.index()]
            .attributes
            .insert(name.into().to_ascii_lowercase(), value.into());
    }

    pub fn attribute(&self, node: NodeId, name: &str) -> Option<&str> {
        self.nodes[node.index()]
            .attributes
            .get(&name.to_ascii_lowercase())
            .map(String::as_str)
    }

    /// Resolve counters, quotes and `attr()` and materialize generated text as
    /// normal Text children. Call after the generated pseudo styles are set.
    pub fn materialize_generated_content(&mut self) {
        let mut counters: HashMap<String, Vec<i32>> = HashMap::new();
        let mut quote_depth = 0usize;
        self.materialize_subtree(self.root, &mut counters, &mut quote_depth);
    }

    fn materialize_subtree(
        &mut self,
        node: NodeId,
        counters: &mut HashMap<String, Vec<i32>>,
        quote_depth: &mut usize,
    ) {
        let style = self.nodes[node.index()].style.clone();
        let mut reset_names = Vec::new();
        for operation in &style.counter_reset {
            counters
                .entry(operation.name.clone())
                .or_default()
                .push(operation.value);
            reset_names.push(operation.name.clone());
        }
        for operation in &style.counter_set {
            let values = counters
                .entry(operation.name.clone())
                .or_insert_with(|| vec![0]);
            if let Some(value) = values.last_mut() {
                *value = operation.value;
            }
        }
        for operation in &style.counter_increment {
            let values = counters
                .entry(operation.name.clone())
                .or_insert_with(|| vec![0]);
            if let Some(value) = values.last_mut() {
                *value += operation.value;
            }
        }

        if self.nodes[node.index()].pseudo_kind.is_some() {
            if let Some(items) = style.content.as_deref() {
                let origin = self.nodes[node.index()].pseudo_origin;
                let generated = resolve_generated_items(
                    items,
                    origin,
                    &self.nodes,
                    counters,
                    quote_depth,
                    &style.quotes,
                );
                if !generated.is_empty() {
                    let text = self.create_node(ElementTag::Text);
                    // Generated text participates as an ordinary text child
                    // of the pseudo box. It inherits text properties, but it
                    // must not copy the pseudo box's display, sizing, margin,
                    // float, or positioning declarations (notably when the
                    // pseudo itself is a flex container).
                    self.nodes[text.index()].style = ComputedStyle::for_pseudo(&style);
                    self.nodes[text.index()].text = Some(generated);
                    self.append_child(node, text);
                }
            }
        }

        // Snapshot because generated text can extend the arena while walking.
        let children: Vec<_> = self.children(node).collect();
        for child in children {
            self.materialize_subtree(child, counters, quote_depth);
        }
        for name in reset_names.into_iter().rev() {
            if let Some(values) = counters.get_mut(&name) {
                values.pop();
                if values.is_empty() {
                    counters.remove(&name);
                }
            }
        }
    }

    /// Access a node immutably.
    #[inline]
    pub fn node(&self, id: NodeId) -> &NodeData {
        &self.nodes[id.index()]
    }

    /// Access a node mutably (for setting style, text, etc.).
    #[inline]
    pub fn node_mut(&mut self, id: NodeId) -> &mut NodeData {
        &mut self.nodes[id.index()]
    }

    /// Iterate over child node IDs of `parent`.
    pub fn children(&self, parent: NodeId) -> ChildIter<'_> {
        ChildIter {
            doc: self,
            current: self.nodes[parent.index()].first_child,
        }
    }

    /// Return the legend that supplies an HTML fieldset's rendered legend box.
    ///
    /// `display: contents` removes its principal box, so legends below an
    /// unbroken chain of such elements participate as fieldset children in the
    /// box tree. Hidden, unboxed, and out-of-flow legends do not establish the
    /// fieldset's border area.
    pub fn fieldset_rendered_legend(&self, fieldset: NodeId) -> Option<NodeId> {
        if self.node(fieldset).tag != ElementTag::Fieldset {
            return None;
        }

        fn find(doc: &Document, parent: NodeId) -> Option<NodeId> {
            for child in doc.children(parent) {
                let node = doc.node(child);
                if node.style.display == Display::None || node.style.is_out_of_flow() {
                    continue;
                }
                if node.style.display == Display::Contents {
                    if let Some(legend) = find(doc, child) {
                        return Some(legend);
                    }
                } else if node.tag == ElementTag::Legend {
                    return Some(child);
                }
            }
            None
        }

        find(self, fieldset)
    }

    /// Count of all nodes in the document.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Resolve an arena index while an engine adopts nodes created by DOM
    /// materialization. This does not expose mutable tree storage.
    #[doc(hidden)]
    pub fn node_id_at(&self, index: usize) -> Option<NodeId> {
        (index < self.nodes.len()).then_some(NodeId(index as u32))
    }

    /// Number of arena entries currently available for reuse.
    pub fn reusable_node_count(&self) -> usize {
        self.free_nodes.len()
    }

    /// Whether any attached/generated node requires the platform native text
    /// raster surface. This remains a document-level surface capability;
    /// individual fonts still select aliased versus LCD glyph masks.
    pub fn uses_native_control_text(&self) -> bool {
        self.nodes.iter().any(|node| node.style.native_control_text)
    }

    /// Whether an author-selected face escapes the deterministic aliased
    /// Fontconfig family set and therefore needs an LCD-capable raster surface.
    /// Pseudo styles live on their originating element rather than as attached
    /// arena nodes, so include `::first-line` explicitly.
    pub fn uses_lcd_author_text(&self) -> bool {
        fn style_uses_lcd(style: &ComputedStyle) -> bool {
            if style.native_control_text || style.embedded_document_text {
                return false;
            }
            let Some(primary) = style.font_family.families.first() else {
                return false;
            };
            match primary {
                FontFamily::Named(name) => ![
                    "Ahem",
                    "Droid Sans Fallback",
                    "Noto Sans Devanagari",
                    "Noto Color Emoji",
                    "DejaVu Sans",
                ]
                .iter()
                .any(|family| name.eq_ignore_ascii_case(family)),
                FontFamily::Generic(family) => !matches!(
                    family,
                    GenericFontFamily::None
                        | GenericFontFamily::SansSerif
                        | GenericFontFamily::Emoji
                        | GenericFontFamily::UiSansSerif
                ),
            }
        }

        self.nodes.iter().any(|node| {
            style_uses_lcd(&node.style)
                || node
                    .style
                    .first_line_style
                    .as_deref()
                    .is_some_and(style_uses_lcd)
        })
    }

    /// Find the first element exposing a given CSS anchor name.
    ///
    /// Generated documents retain computed anchor names directly in style;
    /// the arena order is document order, which supplies deterministic tie
    /// breaking for the compact anchor-positioning model.
    pub fn find_anchor_named(&self, name: &str) -> Option<NodeId> {
        self.nodes
            .iter()
            .position(|node| node.style.anchor_name.as_deref() == Some(name))
            .map(|index| NodeId(index as u32))
    }

    /// Register deterministic encoded image bytes and return their stable
    /// document-local identifier. Re-registering the same source/hash pair
    /// reuses the first entry.
    pub fn register_image_resource(
        &mut self,
        source: impl Into<String>,
        mime_type: impl Into<String>,
        sha256: impl Into<String>,
        bytes: impl Into<Vec<u8>>,
    ) -> ImageResourceId {
        let source = source.into();
        let sha256 = sha256.into();
        if let Some((index, _)) = self
            .image_resources
            .iter()
            .enumerate()
            .find(|(_, resource)| resource.source == source && resource.sha256 == sha256)
        {
            return ImageResourceId::new(index as u32);
        }
        let id = ImageResourceId::new(self.image_resources.len() as u32);
        self.image_resources.push(EncodedImageResource {
            source,
            mime_type: mime_type.into(),
            sha256,
            bytes: bytes.into(),
        });
        id
    }

    #[inline]
    pub fn image_resource(&self, id: ImageResourceId) -> Option<&EncodedImageResource> {
        self.image_resources.get(id.index())
    }

    #[inline]
    pub fn image_resource_count(&self) -> usize {
        self.image_resources.len()
    }

    /// The direct `<html>` child of the viewport, for root-aware documents.
    pub fn document_element(&self) -> Option<NodeId> {
        self.children(self.root)
            .find(|&id| self.node(id).tag == ElementTag::Html)
    }

    /// The direct `<body>` child of the document element.
    pub fn body_element(&self) -> Option<NodeId> {
        let html = self.document_element()?;
        self.children(html)
            .find(|&id| self.node(id).tag == ElementTag::Body)
    }

    /// Identify the synthetic body used by legacy two-node documents.
    ///
    /// Browser-shaped documents should use explicit `Html` and `Body` nodes.
    /// This compatibility hook keeps older generated builders independent of
    /// element-tag semantics while still allowing body background propagation.
    pub fn set_legacy_canvas_body(&mut self, body: NodeId) {
        assert_eq!(
            self.node(body).parent,
            self.root,
            "legacy canvas body must be a direct child of the viewport"
        );
        self.legacy_canvas_body = Some(body);
    }

    /// Element whose complete background is propagated to the document canvas.
    pub fn canvas_background_source(&self) -> Option<NodeId> {
        let Some(html) = self.document_element() else {
            // Historical generated documents omit an explicit html element,
            // so an authored root style is represented on the viewport and
            // their synthetic body is registered explicitly. An authored
            // root background wins over body propagation just like `<html>`.
            let root_style = &self.node(self.root).style;
            if !matches!(root_style.display, Display::None | Display::Contents)
                && (!root_style.background_color.is_transparent()
                    || !root_style.background_layers.is_empty()
                    || root_style.background_linear_gradient.is_some())
            {
                return Some(self.root);
            }
            // Do not infer a body from an arbitrary first child: ordinary
            // fragment/unit-test documents contain direct div children that
            // must retain normal box-scoped backgrounds.
            let body = self.legacy_canvas_body?;
            let style = &self.node(body).style;
            return (!matches!(style.display, Display::None | Display::Contents)
                && (!style.background_color.is_transparent()
                    || !style.background_layers.is_empty()
                    || style.background_linear_gradient.is_some()))
            .then_some(body);
        };
        let html_style = &self.node(html).style;
        if matches!(html_style.display, Display::None | Display::Contents) {
            return None;
        }
        if !html_style.background_color.is_transparent()
            || !html_style.background_layers.is_empty()
            || html_style.background_linear_gradient.is_some()
        {
            return Some(html);
        }
        if html_style.contain.contains(Containment::PAINT) {
            return None;
        }
        let body = self.body_element()?;
        let body_style = &self.node(body).style;
        if matches!(body_style.display, Display::None | Display::Contents)
            || body_style.contain.contains(Containment::PAINT)
            || (body_style.background_color.is_transparent()
                && body_style.background_layers.is_empty()
                && body_style.background_linear_gradient.is_none())
        {
            None
        } else {
            Some(body)
        }
    }

    /// Whether body overflow is transferred to the viewport and reset to
    /// visible on the body box.
    pub fn body_overflow_is_propagated(&self) -> bool {
        let Some(html) = self.document_element() else {
            return false;
        };
        let Some(body) = self.body_element() else {
            return false;
        };
        let html_style = &self.node(html).style;
        let body_style = &self.node(body).style;
        !matches!(html_style.display, Display::None | Display::Contents)
            && !matches!(body_style.display, Display::None | Display::Contents)
            && html_style.overflow_x == Overflow::Visible
            && html_style.overflow_y == Overflow::Visible
            && !html_style.contain.contains(Containment::PAINT)
            && !body_style.contain.contains(Containment::PAINT)
            && (body_style.overflow_x != Overflow::Visible
                || body_style.overflow_y != Overflow::Visible)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_generated_items(
    items: &[GeneratedContentItem],
    origin: NodeId,
    nodes: &[NodeData],
    counters: &HashMap<String, Vec<i32>>,
    quote_depth: &mut usize,
    authored_quotes: &[QuotePair],
) -> String {
    let default_quotes = [
        QuotePair {
            open: "\u{201c}".to_string(),
            close: "\u{201d}".to_string(),
        },
        QuotePair {
            open: "\u{2018}".to_string(),
            close: "\u{2019}".to_string(),
        },
    ];
    let quotes = if authored_quotes.is_empty() {
        &default_quotes[..]
    } else {
        authored_quotes
    };
    let quote_at = |depth: usize| &quotes[depth.min(quotes.len() - 1)];
    let mut output = String::new();
    for item in items {
        match item {
            GeneratedContentItem::String(value) => output.push_str(value),
            GeneratedContentItem::Attribute(name) => {
                if !origin.is_none() {
                    if let Some(value) = nodes[origin.index()]
                        .attributes
                        .get(&name.to_ascii_lowercase())
                    {
                        output.push_str(value);
                    }
                }
            }
            GeneratedContentItem::Counter { name, style } => {
                let value = counters
                    .get(name)
                    .and_then(|values| values.last())
                    .copied()
                    .unwrap_or(0);
                output.push_str(&format_counter(value, *style));
            }
            GeneratedContentItem::Counters {
                name,
                separator,
                style,
            } => {
                if let Some(values) = counters.get(name) {
                    let mut first = true;
                    for value in values {
                        if !first {
                            output.push_str(separator);
                        }
                        first = false;
                        output.push_str(&format_counter(*value, *style));
                    }
                } else {
                    output.push('0');
                }
            }
            GeneratedContentItem::OpenQuote => {
                output.push_str(&quote_at(*quote_depth).open);
                *quote_depth += 1;
            }
            GeneratedContentItem::CloseQuote => {
                *quote_depth = quote_depth.saturating_sub(1);
                output.push_str(&quote_at(*quote_depth).close);
            }
            GeneratedContentItem::NoOpenQuote => *quote_depth += 1,
            GeneratedContentItem::NoCloseQuote => {
                *quote_depth = quote_depth.saturating_sub(1);
            }
        }
    }
    output
}

fn format_counter(value: i32, style: CounterStyle) -> String {
    match style {
        CounterStyle::Decimal => value.to_string(),
        CounterStyle::DecimalLeadingZero => {
            if (-9..=9).contains(&value) {
                if value < 0 {
                    format!("-0{}", value.unsigned_abs())
                } else {
                    format!("0{value}")
                }
            } else {
                value.to_string()
            }
        }
        CounterStyle::LowerAlpha => format_alpha(value, false),
        CounterStyle::UpperAlpha => format_alpha(value, true),
        CounterStyle::LowerRoman => format_roman(value, false),
        CounterStyle::UpperRoman => format_roman(value, true),
    }
}

fn format_alpha(mut value: i32, upper: bool) -> String {
    if value <= 0 {
        return value.to_string();
    }
    let mut bytes = Vec::new();
    while value > 0 {
        value -= 1;
        bytes.push((if upper { b'A' } else { b'a' }) + (value % 26) as u8);
        value /= 26;
    }
    bytes.reverse();
    String::from_utf8(bytes).expect("ASCII alphabetic counter")
}

fn format_roman(value: i32, upper: bool) -> String {
    if !(1..=3999).contains(&value) {
        return value.to_string();
    }
    const ROMAN: &[(i32, &str)] = &[
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut remaining = value;
    let mut output = String::new();
    for &(unit, token) in ROMAN {
        while remaining >= unit {
            remaining -= unit;
            output.push_str(token);
        }
    }
    if upper {
        output
    } else {
        output.to_ascii_lowercase()
    }
}

/// Iterator over children of a node.
pub struct ChildIter<'a> {
    doc: &'a Document,
    current: NodeId,
}

impl<'a> Iterator for ChildIter<'a> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        if self.current.is_none() {
            return None;
        }
        let id = self.current;
        self.current = self.doc.nodes[id.index()].next_sibling;
        Some(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_aware_document() -> (Document, NodeId, NodeId) {
        let mut doc = Document::new();
        let html = doc.create_node(ElementTag::Html);
        let body = doc.create_node(ElementTag::Body);
        doc.update_resolved_style(html, |style| style.display = Display::Block);
        doc.update_resolved_style(body, |style| style.display = Display::Block);
        doc.append_child(doc.root(), html);
        doc.append_child(html, body);
        (doc, html, body)
    }

    #[test]
    fn create_document() {
        let doc = Document::new();
        assert!(!doc.root().is_none());
        assert_eq!(doc.node(doc.root()).tag, ElementTag::Viewport);
        assert_eq!(doc.node_count(), 1);
    }

    #[test]
    fn append_children() {
        let mut doc = Document::new();
        let root = doc.root();

        let a = doc.create_node(ElementTag::Div);
        let b = doc.create_node(ElementTag::Div);
        let c = doc.create_node(ElementTag::Div);

        doc.append_child(root, a);
        doc.append_child(root, b);
        doc.append_child(root, c);

        let children: Vec<NodeId> = doc.children(root).collect();
        assert_eq!(children.len(), 3);
        assert_eq!(children[0], a);
        assert_eq!(children[1], b);
        assert_eq!(children[2], c);

        // Verify parent pointers
        assert_eq!(doc.node(a).parent, root);
        assert_eq!(doc.node(b).parent, root);
        assert_eq!(doc.node(c).parent, root);

        // Verify sibling pointers
        assert_eq!(doc.node(a).next_sibling, b);
        assert_eq!(doc.node(b).prev_sibling, a);
        assert_eq!(doc.node(b).next_sibling, c);
        assert_eq!(doc.node(c).prev_sibling, b);
    }

    #[test]
    fn detach_repairs_sibling_links() {
        let mut doc = Document::new();
        let root = doc.root();
        let a = doc.create_node(ElementTag::Div);
        let b = doc.create_node(ElementTag::Div);
        let c = doc.create_node(ElementTag::Div);
        doc.append_child(root, a);
        doc.append_child(root, b);
        doc.append_child(root, c);
        assert!(doc.detach(b));
        assert_eq!(doc.children(root).collect::<Vec<_>>(), vec![a, c]);
        assert!(doc.node(b).parent.is_none());
        assert!(!doc.detach(b));
    }

    #[test]
    fn released_detached_nodes_reuse_arena_storage() {
        let mut document = Document::new();
        let node = document.create_node(ElementTag::Div);
        assert!(document.release_detached_node(node));
        assert_eq!(document.reusable_node_count(), 1);
        let replacement = document.create_node(ElementTag::Span);
        assert_eq!(replacement, node);
        assert_eq!(document.reusable_node_count(), 0);
        assert_eq!(document.node_count(), 2);
        assert_eq!(document.node(replacement).tag, ElementTag::Span);
    }

    #[test]
    fn pseudo_nodes_use_css_tree_order_and_materialize_attr_text() {
        let mut doc = Document::new();
        let origin = doc.create_node(ElementTag::Div);
        let authored = doc.create_node(ElementTag::Text);
        doc.node_mut(authored).text = Some("body".into());
        doc.append_child(doc.root(), origin);
        doc.append_child(origin, authored);
        doc.set_attribute(origin, "data-label", "value");

        let after = doc.insert_pseudo_element(origin, PseudoElementKind::After);
        doc.update_resolved_style(after, |style| {
            style.content = Some(vec![GeneratedContentItem::String("A".into())])
        });
        let before = doc.insert_pseudo_element(origin, PseudoElementKind::Before);
        doc.update_resolved_style(before, |style| {
            style.content = Some(vec![
                GeneratedContentItem::Attribute("data-label".into()),
                GeneratedContentItem::String(":".into()),
            ])
        });
        doc.materialize_generated_content();

        let children: Vec<_> = doc.children(origin).collect();
        assert_eq!(children, vec![before, authored, after]);
        assert_eq!(doc.node(before).pseudo_origin, origin);
        let before_text = doc.children(before).next().unwrap();
        let after_text = doc.children(after).next().unwrap();
        assert_eq!(doc.node(before_text).text.as_deref(), Some("value:"));
        assert_eq!(doc.node(after_text).text.as_deref(), Some("A"));
    }

    #[test]
    fn generated_counters_scope_format_and_quotes() {
        let mut doc = Document::new();
        let origin = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(origin, |style| {
            style.counter_reset = vec![openui_style::CounterOperation {
                name: "section".into(),
                value: 3,
            }]
        });
        doc.append_child(doc.root(), origin);
        let before = doc.insert_pseudo_element(origin, PseudoElementKind::Before);
        doc.update_resolved_style(before, |style| {
            style.counter_increment = vec![openui_style::CounterOperation {
                name: "section".into(),
                value: 1,
            }]
        });
        doc.update_resolved_style(before, |style| {
            style.content = Some(vec![
                GeneratedContentItem::OpenQuote,
                GeneratedContentItem::Counter {
                    name: "section".into(),
                    style: CounterStyle::UpperRoman,
                },
                GeneratedContentItem::CloseQuote,
            ])
        });
        doc.materialize_generated_content();
        let text = doc.children(before).next().unwrap();
        assert_eq!(doc.node(text).text.as_deref(), Some("“IV”"));
    }

    #[test]
    fn nested_children() {
        let mut doc = Document::new();
        let root = doc.root();

        let parent = doc.create_node(ElementTag::Div);
        let child = doc.create_node(ElementTag::Div);

        doc.append_child(root, parent);
        doc.append_child(parent, child);

        assert_eq!(doc.node(child).parent, parent);
        let root_children: Vec<_> = doc.children(root).collect();
        assert_eq!(root_children.len(), 1);

        let parent_children: Vec<_> = doc.children(parent).collect();
        assert_eq!(parent_children.len(), 1);
        assert_eq!(parent_children[0], child);
    }

    #[test]
    fn style_mutation() {
        let mut doc = Document::new();
        let node = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(node, |style| style.display = openui_style::Display::Block);
        doc.update_resolved_style(node, |style| {
            style.width = openui_geometry::Length::px(100.0)
        });
        doc.update_resolved_style(node, |style| {
            style.background_color = openui_style::Color::RED
        });

        assert_eq!(doc.node(node).style.display, openui_style::Display::Block);
        assert_eq!(
            doc.node(node).style.width,
            openui_geometry::Length::px(100.0)
        );
    }

    #[test]
    fn fieldset_rendered_legend_follows_display_contents_box_children() {
        let mut doc = Document::new();
        let fieldset = doc.create_node(ElementTag::Fieldset);
        doc.append_child(doc.root(), fieldset);

        let ordinary = doc.create_node(ElementTag::Div);
        doc.append_child(fieldset, ordinary);
        let nested_ordinary_legend = doc.create_node(ElementTag::Legend);
        doc.append_child(ordinary, nested_ordinary_legend);

        let contents = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(contents, |style| style.display = Display::Contents);
        doc.append_child(fieldset, contents);
        let flattened_legend = doc.create_node(ElementTag::Legend);
        doc.append_child(contents, flattened_legend);

        let later_direct_legend = doc.create_node(ElementTag::Legend);
        doc.append_child(fieldset, later_direct_legend);

        assert_eq!(
            doc.fieldset_rendered_legend(fieldset),
            Some(flattened_legend)
        );

        doc.update_resolved_style(flattened_legend, |style| style.display = Display::Contents);
        assert_eq!(
            doc.fieldset_rendered_legend(fieldset),
            Some(later_direct_legend)
        );

        doc.update_resolved_style(later_direct_legend, |style| style.display = Display::None);
        assert_eq!(doc.fieldset_rendered_legend(fieldset), None);
    }

    #[test]
    fn installed_resolved_style_retains_document_raster_context() {
        let raster_configuration = RasterConfiguration::legacy_deterministic_aliased(true);
        let mut doc = Document::new_with_raster_configuration(
            openui_text::FontCollection::system(),
            raster_configuration,
            2.0,
        );
        let node = doc.create_node(ElementTag::Div);

        doc.install_resolved_style(node, ComputedStyle::initial());

        assert_eq!(
            doc.node(node).style.raster_configuration,
            raster_configuration
        );
        assert_eq!(doc.node(node).style.device_scale_factor, 2.0);
    }

    #[test]
    fn root_background_precedes_body_canvas_propagation() {
        let (mut doc, html, body) = root_aware_document();
        doc.update_resolved_style(html, |style| {
            style.background_color = openui_style::Color::RED
        });
        doc.update_resolved_style(body, |style| {
            style.background_color = openui_style::Color::BLUE
        });
        assert_eq!(doc.canvas_background_source(), Some(html));

        doc.update_resolved_style(html, |style| {
            style.background_color = openui_style::Color::TRANSPARENT
        });
        assert_eq!(doc.canvas_background_source(), Some(body));
    }

    #[test]
    fn image_layers_participate_in_complete_canvas_background_selection() {
        let (mut doc, html, body) = root_aware_document();
        let image = openui_style::CssImage::LinearGradient(openui_style::CssLinearGradient {
            angle_degrees: 90.0,
            corner_direction: None,
            repeating: false,
            color_space: openui_style::GradientColorSpace::Srgb,
            stops: vec![
                openui_style::GradientStop {
                    color: openui_style::StyleColor::Resolved(openui_style::Color::RED),
                    position: openui_style::GradientStopPosition::Percent(0.0),
                },
                openui_style::GradientStop {
                    color: openui_style::StyleColor::Resolved(openui_style::Color::BLUE),
                    position: openui_style::GradientStopPosition::Percent(100.0),
                },
            ],
        });
        doc.update_resolved_style(html, |style| {
            style
                .background_layers
                .push(openui_style::BackgroundLayer::new(image.clone()));
        });
        assert_eq!(doc.canvas_background_source(), Some(html));

        doc.update_resolved_style(html, |style| style.background_layers.clear());
        doc.update_resolved_style(body, |style| {
            style
                .background_layers
                .push(openui_style::BackgroundLayer::new(image));
        });
        assert_eq!(doc.canvas_background_source(), Some(body));
    }

    #[test]
    fn explicitly_registered_legacy_body_propagates_canvas_background() {
        let mut doc = Document::new();
        let body = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(body, |style| style.display = Display::Block);
        doc.update_resolved_style(body, |style| {
            style.background_color = openui_style::Color::BLUE
        });
        doc.append_child(doc.root(), body);
        doc.set_legacy_canvas_body(body);
        assert_eq!(doc.canvas_background_source(), Some(body));

        let mut ordinary = Document::new();
        let div = ordinary.create_node(ElementTag::Div);
        ordinary.update_resolved_style(div, |style| style.display = Display::Block);
        ordinary.update_resolved_style(div, |style| {
            style.background_color = openui_style::Color::BLUE
        });
        ordinary.append_child(ordinary.root(), div);
        assert_eq!(ordinary.canvas_background_source(), None);
    }

    #[test]
    fn authored_legacy_root_background_precedes_registered_body() {
        let mut doc = Document::new();
        let body = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(body, |style| style.display = Display::Block);
        doc.update_resolved_style(body, |style| {
            style.background_color = openui_style::Color::BLACK
        });
        doc.append_child(doc.root(), body);
        doc.set_legacy_canvas_body(body);
        doc.update_resolved_style(doc.root(), |style| {
            style.background_color = openui_style::Color::WHITE
        });

        assert_eq!(doc.canvas_background_source(), Some(doc.root()));
    }

    #[test]
    fn hidden_or_unboxed_root_body_cannot_supply_canvas_background() {
        let (mut doc, html, body) = root_aware_document();
        doc.update_resolved_style(body, |style| {
            style.background_color = openui_style::Color::RED
        });
        doc.update_resolved_style(body, |style| style.display = Display::Contents);
        assert_eq!(doc.canvas_background_source(), None);

        doc.update_resolved_style(body, |style| style.display = Display::Block);
        doc.update_resolved_style(html, |style| style.display = Display::None);
        assert_eq!(doc.canvas_background_source(), None);
    }

    #[test]
    fn root_overflow_prevents_body_overflow_propagation() {
        let (mut doc, html, body) = root_aware_document();
        doc.update_resolved_style(body, |style| style.overflow_y = Overflow::Hidden);
        assert!(doc.body_overflow_is_propagated());

        doc.update_resolved_style(html, |style| style.overflow_y = Overflow::Hidden);
        assert!(!doc.body_overflow_is_propagated());
    }
}
