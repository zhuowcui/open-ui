//! Arena-based element tree with parent/child relationships.
//!
//! Uses a simple Vec<Node> arena indexed by `NodeId`. This is similar to
//! how Blink stores nodes — a flat arena with pointer-like indices for
//! parent, first_child, last_child, next_sibling, prev_sibling.

use std::collections::{BTreeMap, HashMap};

use openui_style::{
    ComputedStyle, ContainerCondition, Containment, CounterStyle, Display, GeneratedContentItem,
    ImageResourceId, Overflow, QuotePair, ScrollMarkerGroup,
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
    Range,
    Meter,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementTag {
    /// A generic container element (like HTML `<div>`).
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
    Fieldset,
    Legend,
    /// The document element (`<html>`).
    Html,
    /// The document body (`<body>`).
    Body,
    /// An author style element whose text became renderable through CSS.
    Style,
    /// The root viewport element.
    Viewport,
}

impl Default for ElementTag {
    fn default() -> Self {
        Self::Div
    }
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
}

/// Data stored for each node in the tree.
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
    pub form_control: Option<FormControlRole>,
    /// Whether the platform-native form-control appearance remains enabled
    /// after the authored `appearance` cascade.
    pub form_control_native_appearance: bool,

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
        Self {
            tag,
            style: ComputedStyle::initial(),
            pseudo_kind: None,
            pseudo_origin: NodeId::NONE,
            attributes: BTreeMap::new(),
            scroll_left: 0.0,
            scroll_top: 0.0,
            table_col_span: 1,
            table_row_span: 1,
            replaced: None,
            form_control: None,
            form_control_native_appearance: true,
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

    /// Mutable access to the style (convenience for tests and builder patterns).
    #[inline]
    pub fn style_mut(&mut self) -> &mut ComputedStyle {
        &mut self.style
    }
}

/// The document tree — an arena of nodes.
///
/// This is the native equivalent of Blink's `Document` + DOM tree, but
/// without any parsing, events, or script execution. It's just a tree.
pub struct Document {
    nodes: Vec<NodeData>,
    root: NodeId,
    image_resources: Vec<EncodedImageResource>,
    legacy_canvas_body: Option<NodeId>,
}

impl Document {
    /// Create a new document with a root viewport element.
    pub fn new() -> Self {
        let mut doc = Self {
            nodes: Vec::new(),
            root: NodeId::NONE,
            image_resources: Vec::new(),
            legacy_canvas_body: None,
        };
        let root_id = doc.create_node(ElementTag::Viewport);
        doc.root = root_id;
        // The viewport is a block-level element.
        doc.nodes[root_id.index()].style.display = openui_style::Display::Block;
        doc
    }

    /// The root viewport node.
    #[inline]
    pub fn root(&self) -> NodeId {
        self.root
    }

    /// Create a new detached node (not yet in the tree).
    pub fn create_node(&mut self, tag: ElementTag) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(NodeData::new(tag));
        id
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
    fn insert_before_sibling(&mut self, sibling: NodeId, child: NodeId) {
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
                self.insert_before_sibling(origin, pseudo)
            }
            PseudoElementKind::ScrollMarkerGroup
                if !self.nodes[origin.index()].parent.is_none() =>
            {
                self.insert_after_sibling(origin, pseudo)
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
            | PseudoElementKind::ScrollMarkerGroup => self.append_child(origin, pseudo),
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

    /// Count of all nodes in the document.
    pub fn node_count(&self) -> usize {
        self.nodes.len()
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
        doc.node_mut(html).style.display = Display::Block;
        doc.node_mut(body).style.display = Display::Block;
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
    fn pseudo_nodes_use_css_tree_order_and_materialize_attr_text() {
        let mut doc = Document::new();
        let origin = doc.create_node(ElementTag::Div);
        let authored = doc.create_node(ElementTag::Text);
        doc.node_mut(authored).text = Some("body".into());
        doc.append_child(doc.root(), origin);
        doc.append_child(origin, authored);
        doc.set_attribute(origin, "data-label", "value");

        let after = doc.insert_pseudo_element(origin, PseudoElementKind::After);
        doc.node_mut(after).style.content = Some(vec![GeneratedContentItem::String("A".into())]);
        let before = doc.insert_pseudo_element(origin, PseudoElementKind::Before);
        doc.node_mut(before).style.content = Some(vec![
            GeneratedContentItem::Attribute("data-label".into()),
            GeneratedContentItem::String(":".into()),
        ]);
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
        doc.node_mut(origin).style.counter_reset = vec![openui_style::CounterOperation {
            name: "section".into(),
            value: 3,
        }];
        doc.append_child(doc.root(), origin);
        let before = doc.insert_pseudo_element(origin, PseudoElementKind::Before);
        doc.node_mut(before).style.counter_increment = vec![openui_style::CounterOperation {
            name: "section".into(),
            value: 1,
        }];
        doc.node_mut(before).style.content = Some(vec![
            GeneratedContentItem::OpenQuote,
            GeneratedContentItem::Counter {
                name: "section".into(),
                style: CounterStyle::UpperRoman,
            },
            GeneratedContentItem::CloseQuote,
        ]);
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
        doc.node_mut(node).style.display = openui_style::Display::Block;
        doc.node_mut(node).style.width = openui_geometry::Length::px(100.0);
        doc.node_mut(node).style.background_color = openui_style::Color::RED;

        assert_eq!(doc.node(node).style.display, openui_style::Display::Block);
        assert_eq!(
            doc.node(node).style.width,
            openui_geometry::Length::px(100.0)
        );
    }

    #[test]
    fn root_background_precedes_body_canvas_propagation() {
        let (mut doc, html, body) = root_aware_document();
        doc.node_mut(html).style.background_color = openui_style::Color::RED;
        doc.node_mut(body).style.background_color = openui_style::Color::BLUE;
        assert_eq!(doc.canvas_background_source(), Some(html));

        doc.node_mut(html).style.background_color = openui_style::Color::TRANSPARENT;
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
        doc.node_mut(html)
            .style
            .background_layers
            .push(openui_style::BackgroundLayer::new(image.clone()));
        assert_eq!(doc.canvas_background_source(), Some(html));

        doc.node_mut(html).style.background_layers.clear();
        doc.node_mut(body)
            .style
            .background_layers
            .push(openui_style::BackgroundLayer::new(image));
        assert_eq!(doc.canvas_background_source(), Some(body));
    }

    #[test]
    fn explicitly_registered_legacy_body_propagates_canvas_background() {
        let mut doc = Document::new();
        let body = doc.create_node(ElementTag::Div);
        doc.node_mut(body).style.display = Display::Block;
        doc.node_mut(body).style.background_color = openui_style::Color::BLUE;
        doc.append_child(doc.root(), body);
        doc.set_legacy_canvas_body(body);
        assert_eq!(doc.canvas_background_source(), Some(body));

        let mut ordinary = Document::new();
        let div = ordinary.create_node(ElementTag::Div);
        ordinary.node_mut(div).style.display = Display::Block;
        ordinary.node_mut(div).style.background_color = openui_style::Color::BLUE;
        ordinary.append_child(ordinary.root(), div);
        assert_eq!(ordinary.canvas_background_source(), None);
    }

    #[test]
    fn authored_legacy_root_background_precedes_registered_body() {
        let mut doc = Document::new();
        let body = doc.create_node(ElementTag::Div);
        doc.node_mut(body).style.display = Display::Block;
        doc.node_mut(body).style.background_color = openui_style::Color::BLACK;
        doc.append_child(doc.root(), body);
        doc.set_legacy_canvas_body(body);
        doc.node_mut(doc.root()).style.background_color = openui_style::Color::WHITE;

        assert_eq!(doc.canvas_background_source(), Some(doc.root()));
    }

    #[test]
    fn hidden_or_unboxed_root_body_cannot_supply_canvas_background() {
        let (mut doc, html, body) = root_aware_document();
        doc.node_mut(body).style.background_color = openui_style::Color::RED;
        doc.node_mut(body).style.display = Display::Contents;
        assert_eq!(doc.canvas_background_source(), None);

        doc.node_mut(body).style.display = Display::Block;
        doc.node_mut(html).style.display = Display::None;
        assert_eq!(doc.canvas_background_source(), None);
    }

    #[test]
    fn root_overflow_prevents_body_overflow_propagation() {
        let (mut doc, html, body) = root_aware_document();
        doc.node_mut(body).style.overflow_y = Overflow::Hidden;
        assert!(doc.body_overflow_is_propagated());

        doc.node_mut(html).style.overflow_y = Overflow::Hidden;
        assert!(!doc.body_overflow_is_propagated());
    }
}
