//! Read-only layout boxes over the application's retained native document.
//!
//! Anonymous formatting boxes have layout style and child membership without
//! creating, cloning or mutating semantic DOM nodes. Public algorithm entries
//! retain their Document signatures; internal algorithms share this context.
use openui_dom::{Document, ElementTag, NodeData, NodeId};
use openui_geometry::RasterConfiguration;
use openui_style::ComputedStyle;
use openui_text::{Font, FontCollection, FontDescription};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayoutBoxRole {
    Authored,
    FieldsetContent,
}

pub(crate) struct LayoutChildren<'a> {
    document: &'a Document,
    next: NodeId,
    excluded: &'a [NodeId],
}
impl Iterator for LayoutChildren<'_> {
    type Item = NodeId;
    fn next(&mut self) -> Option<NodeId> {
        while !self.next.is_none() {
            let node = self.next;
            self.next = self.document.node(node).next_sibling;
            if !self.excluded.contains(&node) {
                return Some(node);
            }
        }
        None
    }
}

pub(crate) trait LayoutDocument {
    fn source_document(&self) -> &Document;
    fn node(&self, id: NodeId) -> &NodeData {
        self.source_document().node(id)
    }
    fn excluded_children(&self) -> &[NodeId] {
        &[]
    }
    fn children(&self, parent: NodeId) -> LayoutChildren<'_> {
        let source = self.source_document();
        LayoutChildren {
            document: source,
            next: source.node(parent).first_child,
            excluded: self.excluded_children(),
        }
    }
    fn layout_box_role(&self, _node: NodeId) -> LayoutBoxRole {
        LayoutBoxRole::Authored
    }
    fn anonymous_absolute_container(&self, node: NodeId) -> bool {
        self.layout_box_role(node) == LayoutBoxRole::FieldsetContent && {
            let style = &self.source_document().node(node).style;
            style.position.is_positioned()
                || style.has_transform_containing_block()
                || style.has_layout_containment()
        }
    }
    fn anonymous_fixed_container(&self, node: NodeId) -> bool {
        self.layout_box_role(node) == LayoutBoxRole::FieldsetContent && {
            let style = &self.source_document().node(node).style;
            style.has_transform_containing_block() || style.has_layout_containment()
        }
    }
    fn root(&self) -> NodeId {
        self.source_document().root()
    }
    fn attribute(&self, node: NodeId, name: &str) -> Option<&str> {
        self.source_document().attribute(node, name)
    }
    fn document_element(&self) -> Option<NodeId> {
        self.source_document().document_element()
    }
    fn body_element(&self) -> Option<NodeId> {
        self.source_document().body_element()
    }
    fn body_overflow_is_propagated(&self) -> bool {
        self.source_document().body_overflow_is_propagated()
    }
    fn fieldset_rendered_legend(&self, node: NodeId) -> Option<NodeId> {
        if self.layout_box_role(node) == LayoutBoxRole::FieldsetContent {
            None
        } else {
            self.source_document().fieldset_rendered_legend(node)
        }
    }
    fn font_collection(&self) -> &Arc<FontCollection> {
        self.source_document().font_collection()
    }
    fn resolve_font(&self, description: FontDescription) -> Font {
        self.source_document().resolve_font(description)
    }
    fn raster_configuration(&self) -> RasterConfiguration {
        self.source_document().raster_configuration()
    }
    fn device_scale_factor(&self) -> f64 {
        self.source_document().device_scale_factor()
    }
    fn find_anchor_named(&self, name: &str) -> Option<NodeId> {
        self.source_document().find_anchor_named(name)
    }
}
impl LayoutDocument for Document {
    fn source_document(&self) -> &Document {
        self
    }
}

/// A single layout-style substitution; semantic tree storage stays borrowed.
/// Flex measurement and anonymous content use the same read-only operation.
pub(crate) struct NodeStyleContext<'a> {
    source: &'a dyn LayoutDocument,
    node: NodeId,
    data: NodeData,
}
impl<'a> NodeStyleContext<'a> {
    pub(crate) fn new(source: &'a dyn LayoutDocument, node: NodeId, style: ComputedStyle) -> Self {
        let mut data = source.node(node).clone();
        data.style = style;
        Self { source, node, data }
    }
}
impl LayoutDocument for NodeStyleContext<'_> {
    fn source_document(&self) -> &Document {
        self.source.source_document()
    }
    fn node(&self, node: NodeId) -> &NodeData {
        if node == self.node {
            &self.data
        } else {
            self.source.node(node)
        }
    }
    fn excluded_children(&self) -> &[NodeId] {
        self.source.excluded_children()
    }
    fn layout_box_role(&self, node: NodeId) -> LayoutBoxRole {
        self.source.layout_box_role(node)
    }
}

/// The anonymous fieldset content is a layout box, not another element.
/// Its source NodeId identifies authored style/state only; ownership is retained
/// explicitly on the resulting fragment. The rendered legend is a sibling.
pub(crate) struct FieldsetContentContext<'a> {
    source: &'a dyn LayoutDocument,
    owner: NodeId,
    data: NodeData,
    excluded: Vec<NodeId>,
}
impl<'a> FieldsetContentContext<'a> {
    pub(crate) fn new(
        source: &'a dyn LayoutDocument,
        owner: NodeId,
        style: ComputedStyle,
        legend: Option<NodeId>,
    ) -> Self {
        let mut data = source.node(owner).clone();
        data.style = style;
        // A generic formatting box uses the shared block/flex/grid algorithms,
        // while semantic operations continue to use source_document().
        data.tag = ElementTag::Div;
        data.form_control = None;
        data.replaced = None;
        let mut excluded = source.excluded_children().to_vec();
        if let Some(legend) = legend {
            excluded.push(legend);
        }
        Self {
            source,
            owner,
            data,
            excluded,
        }
    }
}
impl LayoutDocument for FieldsetContentContext<'_> {
    fn source_document(&self) -> &Document {
        self.source.source_document()
    }
    fn node(&self, node: NodeId) -> &NodeData {
        if node == self.owner {
            &self.data
        } else {
            self.source.node(node)
        }
    }
    fn excluded_children(&self) -> &[NodeId] {
        &self.excluded
    }
    fn layout_box_role(&self, node: NodeId) -> LayoutBoxRole {
        if node == self.owner {
            LayoutBoxRole::FieldsetContent
        } else {
            self.source.layout_box_role(node)
        }
    }
}

/// Record the context's anonymous box identity before scrollbar measurement or
/// final root geometry collection, preserving authored descendants unchanged.
pub(crate) fn retain_layout_box(
    doc: &dyn LayoutDocument,
    node: NodeId,
    fragment: &mut crate::Fragment,
) {
    if doc.layout_box_role(node) == LayoutBoxRole::FieldsetContent {
        fragment.box_ownership = crate::FragmentBoxOwnership::LayoutHelper;
        fragment.anonymous_layout_box =
            Some(crate::AnonymousLayoutBox::FieldsetContent { owner: node });
        fragment.inherited_style = Some(doc.node(node).style.clone());
        fragment.skip_box_decoration = true;
        if fragment.box_geometry.is_none() {
            let rect = fragment.border_box_rect();
            let direction = doc
                .node(node)
                .style
                .direction
                .writing_direction(doc.node(node).style.writing_mode);
            let logical = openui_geometry::WritingModeConverter::new(
                direction,
                openui_geometry::PhysicalSize::zero(),
            )
            .to_logical_size(rect.size);
            fragment.box_geometry = Some(crate::FragmentBoxGeometry {
                source_size: rect.size,
                source_fragment_size: fragment.size,
                source_border_box_rect: rect,
                source_border: fragment.border,
                source_padding: fragment.padding,
                stitched_size: rect.size,
                writing_direction: direction,
                inline_size: logical.inline_size,
                block_size: logical.block_size,
                consumed_block_size: openui_geometry::LayoutUnit::zero(),
                has_next: false,
                is_repeated: false,
                continuation_record: false,
            });
        }
    }
}
