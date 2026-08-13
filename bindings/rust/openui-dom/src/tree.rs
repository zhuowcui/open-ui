//! Arena-based element tree with parent/child relationships.
//!
//! Uses a simple Vec<Node> arena indexed by `NodeId`. This is similar to
//! how Blink stores nodes — a flat arena with pointer-like indices for
//! parent, first_child, last_child, next_sibling, prev_sibling.

use openui_style::{ComputedStyle, Display, Overflow};

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
    /// An HTML ruby container. Ruby base and annotation content form one
    /// atomic inline-level formatting object.
    Ruby,
    /// HTML ruby annotation content (`<rt>`).
    RubyText,
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

/// Data stored for each node in the tree.
pub struct NodeData {
    pub tag: ElementTag,
    pub style: ComputedStyle,

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
}

impl Document {
    /// Create a new document with a root viewport element.
    pub fn new() -> Self {
        let mut doc = Self {
            nodes: Vec::new(),
            root: NodeId::NONE,
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

    /// Element whose solid background is propagated to the document canvas.
    pub fn canvas_background_source(&self) -> Option<NodeId> {
        let html = self.document_element()?;
        let html_style = &self.node(html).style;
        if matches!(html_style.display, Display::None | Display::Contents) {
            return None;
        }
        if !html_style.background_color.is_transparent() {
            return Some(html);
        }
        let body = self.body_element()?;
        let body_style = &self.node(body).style;
        if matches!(body_style.display, Display::None | Display::Contents)
            || body_style.background_color.is_transparent()
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
            && (body_style.overflow_x != Overflow::Visible
                || body_style.overflow_y != Overflow::Visible)
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
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
