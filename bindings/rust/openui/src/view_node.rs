//! View node types and the [`IntoView`] trait.
//!
//! These types form the intermediate representation between the `view!`
//! macro output and the live DOM.

use crate::element::Element;

/// A renderable node in the UI tree.
pub enum ViewNode {
    /// A DOM element (possibly with children already appended).
    Element(Element),
    /// A static text string.
    Text(String),
    /// Multiple sibling nodes.
    Fragment(Vec<ViewNode>),
    /// Nothing — used for conditional rendering.
    Empty,
}

/// Trait for anything that can be converted into a [`ViewNode`].
pub trait IntoView {
    /// Convert this value into a renderable view node.
    fn into_view(self) -> ViewNode;
}

impl IntoView for ViewNode {
    fn into_view(self) -> ViewNode {
        self
    }
}

impl IntoView for &str {
    fn into_view(self) -> ViewNode {
        ViewNode::Text(self.to_string())
    }
}

impl IntoView for String {
    fn into_view(self) -> ViewNode {
        ViewNode::Text(self)
    }
}

impl IntoView for Element {
    fn into_view(self) -> ViewNode {
        ViewNode::Element(self)
    }
}

impl<T: IntoView> IntoView for Vec<T> {
    fn into_view(self) -> ViewNode {
        ViewNode::Fragment(self.into_iter().map(IntoView::into_view).collect())
    }
}

impl<T: IntoView> IntoView for Option<T> {
    fn into_view(self) -> ViewNode {
        match self {
            Some(v) => v.into_view(),
            None => ViewNode::Empty,
        }
    }
}

/// Mount a [`ViewNode`] as a child of `parent`.
///
/// Elements are appended and then "forgotten" (ownership transfers to the
/// DOM tree). Text nodes are wrapped in a `<span>` element.
pub fn mount_view(parent: &Element, view: ViewNode) {
    match view {
        ViewNode::Element(el) => {
            parent.append_child(&el);
            std::mem::forget(el);
        }
        ViewNode::Text(text) => {
            let doc = crate::current_document();
            let el = Element::create(doc, "span").expect("failed to create text element");
            el.set_text(&text).expect("set text");
            parent.append_child(&el);
            std::mem::forget(el);
        }
        ViewNode::Fragment(nodes) => {
            for node in nodes {
                mount_view(parent, node);
            }
        }
        ViewNode::Empty => {}
    }
}
