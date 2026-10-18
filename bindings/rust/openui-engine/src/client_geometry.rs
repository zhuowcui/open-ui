//! Source-coordinate client quads for authored boxes in block fragmentation.
//!
//! Chromium147 LayoutBox::QuadsInAncestorInternal maps each physical part's
//! root-fragmentation offset through the first source box's container transform.
//! PaintPropertyTreeBuilder instead resolves a transform for each actual part.
//! Keep those operations separate: a reported client rectangle can be away from
//! painted ink and its center need not hit the element.

use crate::hit_geometry::WorldTransform;
use openui_compositor::SceneRect;
use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{PhysicalOffset, PhysicalRect};
use openui_layout::{resolved_transform_for_reference_box, Fragment, PromotedTransformAncestor};
use openui_style::AppliedTransform;
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy)]
struct SourceBox {
    /// First owned CSS border anchor before authored transforms, including
    /// layout/scroll and late visual movement in physical viewport coordinates.
    anchor: (f64, f64),
    parent: Option<NodeId>,
    local_transform: AppliedTransform,
    transformed_continuation: bool,
    promoted_ancestry: bool,
    retained_box: bool,
}

#[derive(Clone, Copy)]
struct SourceWorld {
    transform: WorldTransform,
    source_domain: bool,
}

fn owns_box(document: &Document, fragment: &Fragment) -> bool {
    fragment.owns_element_box() && document.node(fragment.node_id).tag != ElementTag::Text
}

fn translated(offset: (f64, f64), physical: PhysicalOffset) -> (f64, f64) {
    (
        offset.0 + physical.left.to_f64(),
        offset.1 + physical.top.to_f64(),
    )
}

fn children_offset(document: &Document, fragment: &Fragment, raw: (f64, f64)) -> (f64, f64) {
    if fragment.owns_scroll_box() {
        let node = document.node(fragment.node_id);
        (
            raw.0 - node.scroll_left as f64,
            raw.1 - node.scroll_top as f64,
        )
    } else {
        raw
    }
}

fn removed_source(
    document: &Document,
    ancestor: &PromotedTransformAncestor,
    parent_raw: (f64, f64),
    parent: Option<NodeId>,
) -> SourceBox {
    // Removed ancestors retain the owning multicol's source coordinates.
    // Their current fragment offset and zero-block missing-fragment origin
    // belong to painting; source client quads retain the authored source box.
    let raw = translated(
        translated(parent_raw, ancestor.source_offset),
        ancestor.visual_offset,
    );
    let reference = ancestor.source_reference_box;
    SourceBox {
        anchor: translated(raw, reference.offset),
        parent,
        local_transform: resolved_transform_for_reference_box(
            &document.node(ancestor.node_id).style,
            PhysicalRect::new(PhysicalOffset::zero(), reference.size),
        )
        .local_applied,
        transformed_continuation: document.node(ancestor.node_id).style.has_transform(),
        promoted_ancestry: false,
        retained_box: false,
    }
}

fn collect_sources(
    document: &Document,
    fragment: &Fragment,
    parent_raw: (f64, f64),
    parent: Option<NodeId>,
    sources: &mut HashMap<NodeId, SourceBox>,
) {
    let raw = translated(parent_raw, fragment.offset);
    let mut source_parent = parent;
    for ancestor in &fragment.promoted_transform_ancestors {
        // A real retained box supplies the primary source geometry. An ancestor
        // removed from every physical subtree keeps an explicit owned descriptor.
        sources
            .entry(ancestor.node_id)
            .or_insert_with(|| removed_source(document, ancestor, parent_raw, source_parent));
        source_parent = Some(ancestor.node_id);
    }
    if owns_box(document, fragment) {
        // Authored continuations remain separate boxes. Only the first source
        // anchor is shared, matching LayoutBox's ordered physical fragments.
        if sources
            .get(&fragment.node_id)
            .is_none_or(|source| !source.retained_box)
        {
            let border_box = fragment.border_box_rect();
            // The full source wrapper and its principal border box are separate
            // layout-owned values. Keep that original principal anchor even when
            // a physical continuation's decoration slice changes its local box.
            let (source_offset, size) = fragment
                .box_geometry
                .map_or((border_box.offset, border_box.size), |data| {
                    (data.source_border_box_rect.offset, data.stitched_size)
                });
            let reference = PhysicalRect::new(PhysicalOffset::zero(), size);
            let style = &document.node(fragment.node_id).style;
            sources.insert(
                fragment.node_id,
                SourceBox {
                    anchor: translated(raw, source_offset),
                    parent: source_parent,
                    local_transform: resolved_transform_for_reference_box(style, reference)
                        .local_applied,
                    transformed_continuation: style.has_transform()
                        && fragment
                            .box_geometry
                            .is_some_and(|data| data.continuation_record),
                    promoted_ancestry: !fragment.promoted_transform_ancestors.is_empty(),
                    retained_box: true,
                },
            );
        }
        source_parent = Some(fragment.node_id);
    }
    let child_raw = children_offset(document, fragment, raw);
    for child in &fragment.children {
        collect_sources(document, child, child_raw, source_parent, sources);
    }
}

fn source_world(
    node: NodeId,
    sources: &HashMap<NodeId, SourceBox>,
    worlds: &mut HashMap<NodeId, SourceWorld>,
    visiting: &mut HashSet<NodeId>,
) -> Option<SourceWorld> {
    if let Some(world) = worlds.get(&node) {
        return Some(*world);
    }
    if !visiting.insert(node) {
        return None;
    }
    let resolved = (|| {
        let source = *sources.get(&node)?;
        let (parent, offset) = if let Some(parent_node) = source.parent {
            let parent_source = sources.get(&parent_node)?;
            let parent_world = source_world(parent_node, sources, worlds, visiting)?;
            (
                parent_world,
                (
                    source.anchor.0 - parent_source.anchor.0,
                    source.anchor.1 - parent_source.anchor.1,
                ),
            )
        } else {
            (
                SourceWorld {
                    transform: WorldTransform::IDENTITY,
                    source_domain: false,
                },
                source.anchor,
            )
        };
        // MapLocalToAncestor applies the box's source transform, then its offset
        // into the source container. This preserves the source operation order;
        // column placement enters each local quad before either transformation.
        let world = SourceWorld {
            transform: parent
                .transform
                .flattened()
                .then(WorldTransform::translate(offset.0, offset.1))
                .then(WorldTransform(source.local_transform)),
            source_domain: parent.source_domain
                || source.transformed_continuation
                || source.promoted_ancestry,
        };
        Some(world)
    })();
    visiting.remove(&node);
    if let Some(world) = resolved {
        worlds.insert(node, world);
    }
    resolved
}

fn project_fragments(
    document: &Document,
    fragment: &Fragment,
    parent_raw: (f64, f64),
    sources: &HashMap<NodeId, SourceBox>,
    worlds: &mut HashMap<NodeId, SourceWorld>,
    rects: &mut HashMap<NodeId, Vec<SceneRect>>,
) {
    let raw = translated(parent_raw, fragment.offset);
    if owns_box(document, fragment) {
        if let Some(world) = source_world(fragment.node_id, sources, worlds, &mut HashSet::new()) {
            if world.source_domain {
                let source = sources[&fragment.node_id];
                let rect = fragment.border_box_rect();
                let anchor = translated(raw, rect.offset);
                let delta = (anchor.0 - source.anchor.0, anchor.1 - source.anchor.1);
                rects.entry(fragment.node_id).or_default().push(
                    world
                        .transform
                        .then(WorldTransform::translate(delta.0, delta.1))
                        .map_rect(rect.width().to_f32(), rect.height().to_f32()),
                );
            }
        }
    }
    let child_raw = children_offset(document, fragment, raw);
    for child in &fragment.children {
        project_fragments(document, child, child_raw, sources, worlds, rects);
    }
}

/// Override only authored boxes affected by source-coordinate transforms in
/// real continuation metadata. Ordinary geometry keeps its existing floating
/// operation order, and no paint/hit/clip state is modified by this collection.
pub(crate) fn fragment_client_rects(
    document: &Document,
    fragment: &Fragment,
) -> HashMap<NodeId, Vec<SceneRect>> {
    let mut sources = HashMap::new();
    collect_sources(document, fragment, (0.0, 0.0), None, &mut sources);
    let mut rects = HashMap::new();
    project_fragments(
        document,
        fragment,
        (0.0, 0.0),
        &sources,
        &mut HashMap::new(),
        &mut rects,
    );
    rects
}
