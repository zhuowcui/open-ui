//! Authored box identity across retained coordinate-carrier copies.
//!
//! A complete direct-spanner source can remain in several ancestor slices for
//! overflow and paint-phase accounting. Those carriers are not additional
//! authored layout boxes. Provenance is recorded before rebasing; publication
//! is admitted only after the selected producer path has finished.

use crate::layout_context::LayoutDocument;
use crate::{Fragment, FragmentKind};
use openui_dom::{Document, NodeId};
use openui_geometry::{
    LayoutUnit, LogicalOffset, LogicalRect, LogicalSize, PhysicalOffset, PhysicalRect,
    PhysicalSize, WritingDirectionMode, WritingModeConverter,
};
use openui_style::{BoxDecorationBreak, ColumnSpan, Display, Float, Overflow, Position};
use std::fmt;
use std::sync::Arc;

#[derive(Clone)]
pub struct AuthoredProjection {
    record: Arc<Record>,
}

#[derive(Clone)]
enum Record {
    Source {
        complete_builder: bool,
        parent: NodeId,
        direction: WritingDirectionMode,
        published_domain: bool,
    },
    Copy {
        source: Arc<Source>,
        member: Arc<Member>,
        window: Option<Arc<Window>>,
        admitted: bool,
        published_domain: bool,
    },
    Boundary {
        sources: Vec<Arc<Source>>,
        window: Option<Arc<Window>>,
        admitted: bool,
    },
}

struct Source {
    producer: NodeId,
    parent: NodeId,
    point: LayoutUnit,
    extent: LayoutUnit,
    direction: WritingDirectionMode,
    members: Vec<Arc<Member>>,
}

struct Member {
    root: bool,
    node: NodeId,
    kind: FragmentKind,
    size: PhysicalSize,
    border_box: PhysicalRect,
    children: usize,
}

struct Window {
    start: LayoutUnit,
    end: LayoutUnit,
    source_end: LayoutUnit,
    terminal: bool,
    coordinate_carrier: bool,
}

impl fmt::Debug for AuthoredProjection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.record.as_ref() {
            Record::Source {
                complete_builder, ..
            } => f
                .debug_struct("AuthoredProjectionSource")
                .field("complete_builder", complete_builder)
                .finish(),
            Record::Copy {
                source,
                member,
                window,
                admitted,
                ..
            } => f
                .debug_struct("AuthoredProjectionCopy")
                .field("producer", &source.producer)
                .field("parent", &source.parent)
                .field("source_point", &source.point)
                .field("source_extent", &source.extent)
                .field("writing_direction", &source.direction)
                .field("node", &member.node)
                .field("root", &member.root)
                .field("window", &window.as_ref().map(|w| (w.start, w.end)))
                .field("admitted", admitted)
                .field("owns_projection", &self.is_owner())
                .finish(),
            Record::Boundary {
                sources,
                window,
                admitted,
            } => f
                .debug_struct("AuthoredProjectionBoundary")
                .field("source_count", &sources.len())
                .field("window", &window.as_ref().map(|w| (w.start, w.end)))
                .field("admitted", admitted)
                .finish(),
        }
    }
}

impl AuthoredProjection {
    /// Raw carrier ownership is unchanged; only admitted source-copy records
    /// distinguish an authored projection from retained source accounting.
    pub fn is_owner(&self) -> bool {
        match self.record.as_ref() {
            Record::Copy {
                source,
                window: Some(window),
                admitted: true,
                ..
            } => owns_point(source.point, window),
            _ => true,
        }
    }

    pub fn is_materialized_copy(&self) -> bool {
        matches!(
            self.record.as_ref(),
            Record::Copy {
                window: Some(_),
                admitted: true,
                ..
            }
        )
    }

    pub fn source_window(&self) -> Option<(LayoutUnit, LayoutUnit)> {
        match self.record.as_ref() {
            Record::Copy {
                window: Some(window),
                ..
            }
            | Record::Boundary {
                window: Some(window),
                ..
            } => Some((window.start, window.end)),
            _ => None,
        }
    }

    pub fn source_point(&self) -> Option<LayoutUnit> {
        match self.record.as_ref() {
            Record::Copy { source, .. } => Some(source.point),
            _ => None,
        }
    }

    pub fn producer(&self) -> Option<NodeId> {
        match self.record.as_ref() {
            Record::Copy {
                source,
                admitted: true,
                ..
            } => Some(source.producer),
            _ => None,
        }
    }

    pub fn is_structural_boundary(&self) -> bool {
        matches!(
            self.record.as_ref(),
            Record::Boundary { admitted: true, .. }
        )
    }

    /// Permission is tied to the recorded source lineage, not a matching node
    /// or style. Real authored clips must be kept separately by the consumer.
    pub fn crosses_structural_boundary(&self, boundary: &Self) -> bool {
        match (self.record.as_ref(), boundary.record.as_ref()) {
            (
                Record::Copy {
                    source,
                    admitted: true,
                    ..
                },
                Record::Boundary {
                    sources,
                    admitted: true,
                    ..
                },
            ) if self.is_owner() => sources
                .iter()
                .any(|candidate| Arc::ptr_eq(candidate, source)),
            _ => false,
        }
    }

    pub fn source_border_box(&self) -> Option<PhysicalRect> {
        match self.record.as_ref() {
            Record::Copy { member, .. } => Some(member.border_box),
            _ => None,
        }
    }

    /// A relevant source-policy transition needs the actual producer again.
    /// Color changes and idle reads do not invent new layout work.
    pub fn source_eligibility_changed(&self, document: &Document, fragment: &Fragment) -> bool {
        match self.record.as_ref() {
            Record::Source {
                complete_builder: true,
                parent,
                direction,
                published_domain,
            } => {
                let current = eligible_parent(document, *parent, *direction)
                    && eligible_subtree(fragment, document, true, *direction);
                current != *published_domain
            }
            Record::Copy {
                source,
                member,
                published_domain,
                ..
            } if member.root => {
                let current = eligible_parent(document, source.parent, source.direction)
                    && eligible_subtree(fragment, document, true, source.direction);
                current != *published_domain
            }
            _ => false,
        }
    }
}

fn owns_point(point: LayoutUnit, window: &Window) -> bool {
    point >= window.start
        && (point < window.end
            || (window.terminal && point == window.end && point == window.source_end))
}

pub(crate) fn record_direct_spanner_source(
    fragment: &mut Fragment,
    parent: NodeId,
    direction: WritingDirectionMode,
) {
    let complete_builder = fragment.break_token.is_none()
        && fragment
            .box_geometry
            .is_none_or(|g| !g.continuation_record && !g.is_repeated);
    fragment.authored_projection = Some(AuthoredProjection {
        record: Arc::new(Record::Source {
            complete_builder,
            parent,
            direction,
            published_domain: false,
        }),
    });
}

fn eligible_parent(
    doc: &dyn LayoutDocument,
    parent: NodeId,
    direction: WritingDirectionMode,
) -> bool {
    let style = &doc.node(parent).style;
    crate::multicol::ColumnLayoutAlgorithm::from_style(style).is_some()
        && !style.height.is_auto()
        && style.position == Position::Static
        && style.float == Float::None
        && style.direction.writing_direction(style.writing_mode) == direction
        && style.box_decoration_break != BoxDecorationBreak::Clone
        && !style.has_transform()
        && style.opacity >= 1.0
        && style.filter_blur <= 0.0
        && style.filter_grayscale <= 0.0
        && style.clip_path_inset.is_none()
        && style.mask_layers.is_empty()
        && !style.has_paint_containment()
        && !style.has_layout_containment()
        && !style.has_block_size_containment()
        && style.box_shadow.is_empty()
        && style.border_image.is_none()
        && !style.has_outline()
        && crate::used_overflow_pair(doc.source_document(), parent)
            == (Overflow::Visible, Overflow::Visible)
}

fn eligible_subtree(
    fragment: &Fragment,
    doc: &dyn LayoutDocument,
    root: bool,
    direction: WritingDirectionMode,
) -> bool {
    if fragment.kind != FragmentKind::Box
        || !fragment.owns_element_box()
        || fragment.anonymous_layout_box.is_some()
        || fragment.positioned_fragmentation.is_some()
        || !fragment.promoted_transform_ancestors.is_empty()
        || fragment.is_inline_box_fragment
        || !fragment.is_first_for_node
        || !fragment.is_last_for_node
        || fragment.break_token.is_some()
        || fragment.decoration_slice.is_some()
        || fragment.table_part.is_some()
        || fragment.fragmentation_visual_offset != PhysicalOffset::zero()
        || fragment
            .box_geometry
            .is_some_and(|g| g.continuation_record || g.is_repeated)
        || (!root && fragment.multicol_fragmentation.is_some())
    {
        return false;
    }
    if !fragment.node_id.is_none() {
        let node = doc.node(fragment.node_id);
        if node.replaced.is_some()
            || node.form_control.is_some()
            || node.embedded_document.is_some()
            || node.embedded_canvas_color.is_some()
            || node.is_svg_foreign_object
            || node.pseudo_kind.is_some()
        {
            return false;
        }
        let style = &node.style;
        if style.position != Position::Static
            || style.float != Float::None
            || style.display.is_flex()
            || style.display == Display::Grid
            || style.display.is_table_wrapper()
            || style.display == Display::InlineBlock
            || style.display == Display::FlowRoot
            || style.direction.writing_direction(style.writing_mode) != direction
            || style.box_decoration_break == BoxDecorationBreak::Clone
            || style.opacity < 1.0
            || style.filter_blur > 0.0
            || style.filter_grayscale > 0.0
            || style.clip_path_inset.is_some()
            || !style.mask_layers.is_empty()
            || style.has_paint_containment()
            || style.has_layout_containment()
            || style.has_block_size_containment()
            || !style.box_shadow.is_empty()
            || style.border_image.is_some()
            || style.has_outline()
            || style.break_before.is_forced()
            || style.break_after.is_forced()
            || style.break_inside.is_avoid()
            || (style.has_transform() && (root || !fragment.children.is_empty()))
            || style.overflow_x != Overflow::Visible
            || style.overflow_y != Overflow::Visible
        {
            return false;
        }
    }
    fragment
        .children
        .iter()
        .all(|child| eligible_subtree(child, doc, false, direction))
}

/// Seed the actual unsplit source tree. A seed has no consumer effect until
/// the final materialization transaction has verified every carrier.
pub(crate) fn seed_coordinate_sources(
    fragment: &mut Fragment,
    doc: &dyn LayoutDocument,
    producer: NodeId,
    direction: WritingDirectionMode,
) {
    let parent = fragment.node_id;
    if !eligible_parent(doc, parent, direction) {
        return;
    }
    for child in &mut fragment.children {
        let complete_source = child.authored_projection.as_ref().is_some_and(|p| {
            matches!(
                p.record.as_ref(),
                Record::Source {
                    complete_builder: true,
                    ..
                }
            )
        });
        if !complete_source
            || !child.owns_element_box()
            || child.kind != FragmentKind::Box
            || doc.node(child.node_id).style.column_span != ColumnSpan::All
            || doc
                .node(child.node_id)
                .style
                .direction
                .writing_direction(doc.node(child.node_id).style.writing_mode)
                != direction
            || !eligible_subtree(child, doc, true, direction)
        {
            continue;
        }
        fn collect_members(
            fragment: &Fragment,
            direction: WritingDirectionMode,
            root: bool,
            members: &mut Vec<Arc<Member>>,
        ) {
            let rect = fragment.border_box_rect();
            let size = WritingModeConverter::new(direction, PhysicalSize::zero())
                .to_physical_size(LogicalSize::new(fragment.size.width, fragment.size.height));
            let border_box =
                WritingModeConverter::new(direction, size).to_physical_rect(LogicalRect::new(
                    LogicalOffset::new(rect.x(), rect.y()),
                    LogicalSize::new(rect.width(), rect.height()),
                ));
            members.push(Arc::new(Member {
                root,
                node: fragment.node_id,
                kind: fragment.kind,
                size,
                border_box,
                children: fragment.children.len(),
            }));
            for child in &fragment.children {
                collect_members(child, direction, false, members);
            }
        }
        let mut members = Vec::new();
        collect_members(child, direction, true, &mut members);
        let source = Arc::new(Source {
            producer,
            parent,
            point: child.offset.top,
            extent: child.size.height,
            direction,
            members,
        });
        fn assign(fragment: &mut Fragment, source: &Arc<Source>, index: &mut usize) {
            let member = source.members[*index].clone();
            *index += 1;
            fragment.authored_projection = Some(AuthoredProjection {
                record: Arc::new(Record::Copy {
                    source: source.clone(),
                    member,
                    window: None,
                    admitted: false,
                    published_domain: true,
                }),
            });
            for child in &mut fragment.children {
                assign(child, source, index);
            }
        }
        let mut index = 0;
        assign(child, &source, &mut index);
    }
}

fn sources_in(fragment: &Fragment, out: &mut Vec<Arc<Source>>) {
    if let Some(p) = &fragment.authored_projection {
        if let Record::Copy { source, .. } = p.record.as_ref() {
            if !out.iter().any(|s| Arc::ptr_eq(s, source)) {
                out.push(source.clone());
            }
        }
    }
    for child in &fragment.children {
        sources_in(child, out);
    }
}

pub(crate) fn record_coordinate_window(
    fragment: &mut Fragment,
    start: LayoutUnit,
    end: LayoutUnit,
    source_end: LayoutUnit,
    coordinate_carrier: bool,
) {
    let mut sources = Vec::new();
    sources_in(fragment, &mut sources);
    if sources.is_empty() {
        return;
    }
    let window = Arc::new(Window {
        start,
        end,
        source_end,
        terminal: end == source_end,
        coordinate_carrier,
    });
    fn assign(fragment: &mut Fragment, window: &Arc<Window>) {
        if let Some(p) = &fragment.authored_projection {
            if let Record::Copy { source, member, .. } = p.record.as_ref() {
                fragment.authored_projection = Some(AuthoredProjection {
                    record: Arc::new(Record::Copy {
                        source: source.clone(),
                        member: member.clone(),
                        window: Some(window.clone()),
                        admitted: false,
                        published_domain: true,
                    }),
                });
            }
        }
        for child in &mut fragment.children {
            assign(child, window);
        }
    }
    assign(fragment, &window);
    fragment.authored_projection = Some(AuthoredProjection {
        record: Arc::new(Record::Boundary {
            sources,
            window: Some(window),
            admitted: false,
        }),
    });
}

pub(crate) fn record_column_boundary(fragment: &mut Fragment) {
    let mut sources = Vec::new();
    sources_in(fragment, &mut sources);
    if !sources.is_empty() {
        fragment.authored_projection = Some(AuthoredProjection {
            record: Arc::new(Record::Boundary {
                sources,
                window: None,
                admitted: false,
            }),
        });
    }
}

/// Commit whole source groups after all actual slicing/rewrite decisions.
/// Genuine or ambiguous multiple-piece sources keep the old consumer path.
pub(crate) fn finalize(doc: &dyn LayoutDocument, root: &mut Fragment) {
    struct Group {
        source: Arc<Source>,
        windows: Vec<Arc<Window>>,
        roots: usize,
        members: Vec<(Arc<Member>, Arc<Window>)>,
        valid: bool,
    }
    fn collect(doc: &dyn LayoutDocument, fragment: &Fragment, groups: &mut Vec<Group>) {
        if let Some(p) = &fragment.authored_projection {
            match p.record.as_ref() {
                Record::Copy {
                    source,
                    member,
                    window,
                    ..
                } => {
                    let index = match groups.iter().position(|g| Arc::ptr_eq(&g.source, source)) {
                        Some(i) => i,
                        None => {
                            groups.push(Group {
                                source: source.clone(),
                                windows: Vec::new(),
                                roots: 0,
                                members: Vec::new(),
                                valid: true,
                            });
                            groups.len() - 1
                        }
                    };
                    let group = &mut groups[index];
                    if let Some(window) = window {
                        group.members.push((member.clone(), window.clone()));
                    } else {
                        group.valid = false;
                    }
                    group.valid &= fragment.node_id == member.node
                        && fragment.kind == member.kind
                        && fragment.size == member.size
                        && fragment.border_box_rect() == member.border_box
                        && fragment.children.len() == member.children
                        && eligible_subtree(fragment, doc, member.root, source.direction);
                    if member.root {
                        group.roots += 1;
                        if let Some(window) = window {
                            group.windows.push(window.clone());
                        } else {
                            group.valid = false;
                        }
                    }
                }
                _ => {}
            }
        }
        for child in &fragment.children {
            collect(doc, child, groups);
        }
    }
    let mut groups = Vec::new();
    collect(doc, root, &mut groups);
    for group in &mut groups {
        group.windows.sort_by_key(|w| w.start);
        group.valid &= !group.windows.is_empty()
            && group.roots == group.windows.len()
            && group.members.len() == group.roots * group.source.members.len()
            && group.members.iter().all(|(_, window)| {
                group
                    .windows
                    .iter()
                    .any(|root_window| Arc::ptr_eq(root_window, window))
            })
            && group.windows.iter().all(|window| {
                let receipts = group
                    .members
                    .iter()
                    .filter(|(_, member_window)| Arc::ptr_eq(member_window, window))
                    .collect::<Vec<_>>();
                receipts.len() == group.source.members.len()
                    && group.source.members.iter().all(|original_member| {
                        receipts
                            .iter()
                            .filter(|(member, _)| Arc::ptr_eq(member, original_member))
                            .count()
                            == 1
                    })
            })
            && group.windows.iter().all(|w| {
                w.coordinate_carrier
                    && w.end > w.start
                    && w.source_end == group.windows[0].source_end
            })
            && group.windows[0].start == LayoutUnit::zero()
            && group.windows.last().is_some_and(|w| w.terminal)
            && group.windows.windows(2).all(|w| w[0].end == w[1].start)
            && group.source.point >= LayoutUnit::zero()
            && group.source.point <= group.windows[0].source_end;
        let point_owners = group
            .windows
            .iter()
            .filter(|w| owns_point(group.source.point, w))
            .count();
        // Positive real source-piece topology and zero points are independent.
        // No device-space size, transformed AABB, or visible-cell fit is used.
        let positive_pieces = group
            .windows
            .iter()
            .filter(|w| {
                (group.source.point + group.source.extent).min_of(w.end)
                    > group.source.point.max_of(w.start)
            })
            .count();
        group.valid &= point_owners == 1
            && (group.source.extent == LayoutUnit::zero() || positive_pieces == 1);
    }
    fn apply(doc: &dyn LayoutDocument, fragment: &mut Fragment, groups: &[Group]) {
        if let Some(p) = &fragment.authored_projection {
            let record = match p.record.as_ref() {
                Record::Copy {
                    source,
                    member,
                    window,
                    ..
                } => {
                    let admitted = groups
                        .iter()
                        .any(|g| g.valid && Arc::ptr_eq(&g.source, source));
                    Some(Record::Copy {
                        source: source.clone(),
                        member: member.clone(),
                        window: window.clone(),
                        admitted,
                        published_domain: eligible_parent(doc, source.parent, source.direction)
                            && eligible_subtree(fragment, doc, true, source.direction),
                    })
                }
                Record::Boundary {
                    sources, window, ..
                } => {
                    let kept = sources
                        .iter()
                        .filter(|s| groups.iter().any(|g| g.valid && Arc::ptr_eq(&g.source, s)))
                        .cloned()
                        .collect::<Vec<_>>();
                    (!kept.is_empty()).then(|| Record::Boundary {
                        sources: kept,
                        window: window.clone(),
                        admitted: true,
                    })
                }
                Record::Source {
                    complete_builder,
                    parent,
                    direction,
                    ..
                } => Some(Record::Source {
                    complete_builder: *complete_builder,
                    parent: *parent,
                    direction: *direction,
                    published_domain: eligible_parent(doc, *parent, *direction)
                        && eligible_subtree(fragment, doc, true, *direction),
                }),
            };
            fragment.authored_projection = record.map(|record| AuthoredProjection {
                record: Arc::new(record),
            });
        }
        for child in &mut fragment.children {
            apply(doc, child, groups);
        }
    }
    apply(doc, root, &groups);
}
