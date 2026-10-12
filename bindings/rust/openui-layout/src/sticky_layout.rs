//! Resolve retained sticky constraints in the final physical layout tree.
//!
//! Chromium147 uses authored stitched boxes, the real block's padding box,
//! and two distinct ancestor channels. Anonymous line/column/table helpers
//! retain positioning/traversal but never become another authored block.
use crate::layout_context::LayoutDocument;
use crate::{Fragment, ScrollArea, StickyTimelineGeometry};
#[cfg(test)]
use openui_dom::Document;
use openui_dom::NodeId;
use openui_geometry::{BoxStrut, LayoutUnit, Length, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_style::{Display, Position};
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct BoxGeometry {
    rect: PhysicalRect,
    padding_rect: PhysicalRect,
    padding: BoxStrut,
    parent: NodeId,
    block: bool,
    inline: bool,
    consumed: LayoutUnit,
    content_container: Option<BoxContentContainer>,
}
#[derive(Clone, Copy)]
struct BoxContentContainer {
    owner: NodeId,
    rect: PhysicalRect,
    padding_rect: PhysicalRect,
    padding: BoxStrut,
}

#[derive(Clone, Copy)]
struct Resolved {
    delta: PhysicalOffset,
    box_total: PhysicalOffset,
    containing_total: PhysicalOffset,
    geometry: StickyTimelineGeometry,
}
fn add(a: PhysicalOffset, b: PhysicalOffset) -> PhysicalOffset {
    PhysicalOffset::new(a.left + b.left, a.top + b.top)
}
fn moved(mut rect: PhysicalRect, offset: PhysicalOffset) -> PhysicalRect {
    rect.offset = add(rect.offset, offset);
    rect
}
fn content(rect: PhysicalRect, padding: BoxStrut) -> PhysicalRect {
    rect.shrink(padding.top, padding.right, padding.bottom, padding.left)
}
/// MinimumValueForLength's pure-percent operation order differs from the
/// general layout percentage resolver. Auto margins contribute zero here.
fn minimum(length: &Length, basis: LayoutUnit) -> LayoutUnit {
    crate::sticky::resolve_sticky_length(length, basis)
}

fn is_block(doc: &dyn LayoutDocument, fragment: &Fragment) -> bool {
    !fragment.is_inline_box_fragment
        && !crate::scroll_area::is_replaced_layout_box(doc, fragment.node_id)
        && matches!(
            doc.node(fragment.node_id).style.display,
            Display::Block
                | Display::InlineBlock
                | Display::FlowRoot
                | Display::ListItem
                | Display::Flex
                | Display::InlineFlex
                | Display::Grid
                | Display::InlineGrid
                | Display::Table
                | Display::InlineTable
                | Display::TableCell
                | Display::TableCaption
        )
}
fn normal_offset(fragment: &Fragment, source: NodeId) -> PhysicalOffset {
    fragment
        .sticky_timeline_geometry
        .filter(|data| data.scroll_container == source)
        .map_or(fragment.offset, |data| data.parent_relative_normal_offset)
}
fn collect(
    doc: &dyn LayoutDocument,
    fragment: &Fragment,
    source: NodeId,
    parent_offset: PhysicalOffset,
    parent_owner: NodeId,
    boxes: &mut HashMap<NodeId, BoxGeometry>,
    order: &mut Vec<NodeId>,
    anonymous_container: Option<BoxContentContainer>,
) {
    let origin = add(parent_offset, normal_offset(fragment, source));
    let mut next_owner = parent_owner;
    let mut next_anonymous = anonymous_container;
    if let Some(crate::AnonymousLayoutBox::FieldsetContent { owner }) =
        fragment.anonymous_layout_box
    {
        let local = fragment.border_box_rect();
        let size = fragment
            .box_geometry
            .map_or(local.size, |data| data.stitched_size);
        let padding_rect = fragment.scroll_area.map_or(local, |area| area.client_rect);
        next_anonymous = Some(BoxContentContainer {
            owner,
            rect: PhysicalRect::new(add(origin, local.offset), size),
            padding_rect: moved(padding_rect, origin),
            padding: ScrollArea::used_padding(fragment),
        });
        if fragment
            .scroll_area
            .is_some_and(|area| area.is_scroll_container())
        {
            return;
        }
    }
    if fragment.owns_element_box() {
        let local = fragment.border_box_rect();
        let size = if fragment.is_inline_box_fragment {
            local.size
        } else {
            fragment
                .box_geometry
                .map_or(local.size, |data| data.stitched_size)
        };
        let rect = PhysicalRect::new(add(origin, local.offset), size);
        let gutters = fragment
            .element_scrollbars
            .map_or(BoxStrut::zero(), |bars| bars.insets);
        let padding_rect = if let Some(data) = fragment.box_geometry {
            // A sliced first part can omit the ending border/padding. The
            // principal padding rectangle uses its unsliced used edges.
            rect.shrink(
                data.source_border.top + gutters.top,
                data.source_border.right + gutters.right,
                data.source_border.bottom + gutters.bottom,
                data.source_border.left + gutters.left,
            )
        } else {
            let local_padding = fragment.scroll_area.map_or_else(
                || {
                    local.shrink(
                        fragment.border.top,
                        fragment.border.right,
                        fragment.border.bottom,
                        fragment.border.left,
                    )
                },
                |area| area.client_rect,
            );
            moved(local_padding, origin)
        };
        let entry = BoxGeometry {
            rect,
            padding_rect,
            padding: fragment.box_geometry.map_or_else(
                || ScrollArea::used_padding(fragment),
                |data| data.source_padding,
            ),
            parent: parent_owner,
            block: is_block(doc, fragment),
            inline: fragment.is_inline_box_fragment,
            consumed: fragment
                .box_geometry
                .map_or(LayoutUnit::zero(), |data| data.consumed_block_size),
            content_container: anonymous_container,
        };
        if let Some(previous) = boxes.get_mut(&fragment.node_id) {
            if entry.inline && previous.inline {
                previous.rect = previous.rect.unite(&entry.rect);
                previous.padding_rect = previous.padding_rect.unite(&entry.padding_rect);
            } else if entry.consumed < previous.consumed {
                *previous = entry;
            }
        } else {
            boxes.insert(fragment.node_id, entry);
            order.push(fragment.node_id);
        }
        if fragment.node_id != parent_owner {
            next_owner = fragment.node_id;
        }
        if fragment
            .scroll_area
            .is_some_and(|area| area.is_scroll_container())
            || fragment.viewport_scrollport.is_some()
        {
            return;
        }
    }
    for child in &fragment.children {
        collect(
            doc,
            child,
            source,
            origin,
            next_owner,
            boxes,
            order,
            next_anonymous,
        );
    }
}
fn parent(doc: &dyn LayoutDocument, node: NodeId, boxes: &HashMap<NodeId, BoxGeometry>) -> NodeId {
    // Inline wrappers can be siblings in the physical line tree. Use retained
    // authored ancestors when present, then the layout owner for promoted boxes.
    let mut candidate = doc.node(node).parent;
    while !candidate.is_none() {
        if boxes.contains_key(&candidate) {
            return candidate;
        }
        candidate = doc.node(candidate).parent;
    }
    boxes.get(&node).map_or(NodeId::NONE, |data| data.parent)
}
fn containing_block(
    doc: &dyn LayoutDocument,
    node: NodeId,
    source: NodeId,
    boxes: &HashMap<NodeId, BoxGeometry>,
) -> NodeId {
    let mut ancestor = parent(doc, node, boxes);
    while !ancestor.is_none() && ancestor != source {
        if boxes.get(&ancestor).is_some_and(|data| data.block) {
            return ancestor;
        }
        ancestor = parent(doc, ancestor, boxes);
    }
    source
}
fn nearest_sticky(
    doc: &dyn LayoutDocument,
    mut node: NodeId,
    end: NodeId,
    boxes: &HashMap<NodeId, BoxGeometry>,
) -> Option<NodeId> {
    while !node.is_none() && node != end {
        if doc.node(node).style.position == Position::Sticky {
            return Some(node);
        }
        node = parent(doc, node, boxes);
    }
    None
}
fn fixed_to_view(doc: &dyn LayoutDocument, node: NodeId, source: NodeId) -> bool {
    let mut ancestor = node;
    while !ancestor.is_none() && ancestor != source {
        let style = &doc.node(ancestor).style;
        if !matches!(style.display, Display::None | Display::Contents)
            && style.position == Position::Fixed
        {
            let mut container = doc.node(ancestor).parent;
            while !container.is_none() {
                let style = &doc.node(container).style;
                if !matches!(style.display, Display::None | Display::Contents)
                    && (style.has_transform_containing_block() || style.has_layout_containment())
                {
                    return false;
                }
                container = doc.node(container).parent;
            }
            return true;
        }
        ancestor = doc.node(ancestor).parent;
    }
    false
}
fn adjust_pair(
    start: &mut Option<LayoutUnit>,
    end: &mut Option<LayoutUnit>,
    available: LayoutUnit,
    extent: LayoutUnit,
    flipped: bool,
) {
    if let (Some(a), Some(b)) = (start.as_mut(), end.as_mut()) {
        let free = available - extent - *a - *b;
        if free < LayoutUnit::zero() {
            if flipped {
                *a = *a + free;
            } else {
                *b = *b + free;
            }
        }
    }
}

pub(crate) fn populate(doc: &dyn LayoutDocument, scroller: &mut Fragment) {
    let source = scroller.node_id;
    // Direct layout consumers can hand this stage a completed principal box
    // before the root finalizer has retained its scroll geometry. Use the same
    // authoritative geometry collector, rather than a child-size approximation.
    if scroller.scroll_area.is_none() {
        crate::scroll_area::populate(doc, scroller);
    }
    let Some(area) = scroller.scroll_area else {
        return;
    };
    let padding = ScrollArea::used_padding(scroller);
    let border = ScrollArea::used_border(scroller);
    let principal = scroller.border_box_rect();
    let mut boxes = HashMap::new();
    boxes.insert(
        source,
        BoxGeometry {
            rect: principal,
            padding_rect: area.client_rect,
            padding,
            parent: NodeId::NONE,
            block: true,
            inline: false,
            consumed: LayoutUnit::zero(),
            content_container: None,
        },
    );
    let mut order = Vec::new();
    for child in &scroller.children {
        collect(
            doc,
            child,
            source,
            PhysicalOffset::zero(),
            source,
            &mut boxes,
            &mut order,
            None,
        );
    }
    // PaintLayerScrollableArea::UpdateScrollOrigin floors the negated overflow
    // origin relative to the border, then adds integer left/top scrollbar gutters.
    let gutters = scroller
        .element_scrollbars
        .map_or(BoxStrut::zero(), |bars| bars.insets);
    let origin = PhysicalOffset::new(
        (border.left - area.content_rect.x()).floor() + LayoutUnit::from_i32(gutters.left.to_i32()),
        (border.top - area.content_rect.y()).floor() + LayoutUnit::from_i32(gutters.top.to_i32()),
    );
    let node = doc.node(source);
    let scroll_position = PhysicalOffset::new(
        LayoutUnit::from_f32_floor(node.scroll_left + origin.left.to_f32()),
        LayoutUnit::from_f32_floor(node.scroll_top + origin.top.to_f32()),
    );
    let constraining = moved(
        content(area.client_rect, padding),
        PhysicalOffset::new(-origin.left, -origin.top),
    );
    let mut resolved = HashMap::<NodeId, Resolved>::new();
    for node in order {
        let style = &doc.node(node).style;
        if style.position != Position::Sticky {
            continue;
        }
        let normal = boxes[&node];
        let block_node = containing_block(doc, node, source, &boxes);
        let mut block = boxes[&block_node];
        if let Some(content) = normal
            .content_container
            .filter(|content| content.owner == block_node)
        {
            block.rect = content.rect;
            block.padding_rect = content.padding_rect;
            block.padding = content.padding;
        }
        let mut containing = if block_node == source {
            content(area.content_rect, padding)
        } else {
            content(block.padding_rect, block.padding)
        };
        let block_style = &doc.node(block_node).style;
        let direction = block_style
            .direction
            .writing_direction(block_style.writing_mode);
        let logical_width = if direction.is_horizontal() {
            content(block.padding_rect, block.padding).width()
        } else {
            content(block.padding_rect, block.padding).height()
        };
        containing = containing.shrink(
            minimum(&style.margin_top, logical_width),
            minimum(&style.margin_right, logical_width),
            minimum(&style.margin_bottom, logical_width),
            minimum(&style.margin_left, logical_width),
        );
        let nearest_box = nearest_sticky(doc, parent(doc, node, &boxes), block_node, &boxes);
        let nearest_block = nearest_sticky(doc, block_node, source, &boxes);
        let a = nearest_box
            .and_then(|node| resolved.get(&node))
            .map_or(PhysicalOffset::zero(), |data| data.box_total);
        let b = nearest_block
            .and_then(|node| resolved.get(&node))
            .map_or(PhysicalOffset::zero(), |data| data.containing_total);
        let sticky_box = moved(normal.rect, add(a, b));
        let shifted_containing = moved(containing, b);
        let fixed = fixed_to_view(doc, node, source);
        let viewport = if fixed {
            constraining
        } else {
            moved(constraining, scroll_position)
        };
        let mut insets = crate::sticky::StickyConstraintRect {
            top: (!style.top.is_auto()).then(|| minimum(&style.top, constraining.height())),
            bottom: (!style.bottom.is_auto())
                .then(|| minimum(&style.bottom, constraining.height())),
            left: (!style.left.is_auto()).then(|| minimum(&style.left, constraining.width())),
            right: (!style.right.is_auto()).then(|| minimum(&style.right, constraining.width())),
        };
        let flip_x = if direction.is_horizontal() {
            direction.is_rtl()
        } else {
            direction.is_flipped_blocks()
        };
        let flip_y =
            !direction.is_horizontal() && (direction.is_rtl() != direction.is_flipped_lines());
        adjust_pair(
            &mut insets.left,
            &mut insets.right,
            constraining.width(),
            normal.rect.width(),
            flip_x,
        );
        adjust_pair(
            &mut insets.top,
            &mut insets.bottom,
            constraining.height(),
            normal.rect.height(),
            flip_y,
        );
        let delta = crate::sticky::compute_sticky_offset(
            sticky_box.offset,
            PhysicalOffset::zero(),
            viewport,
            &insets,
            sticky_box.size,
            shifted_containing,
        );
        let geometry = StickyTimelineGeometry {
            scroll_container: source,
            parent_relative_normal_offset: PhysicalOffset::zero(),
            scroll_container_relative_normal_offset: normal.rect.offset,
            containing_block_rect: containing,
            insets,
            sticky_box_rect: normal.rect,
            containing_block: block_node,
            nearest_sticky_box: nearest_box,
            nearest_sticky_containing_block: nearest_block,
            total_sticky_box_offset: add(a, delta),
            total_containing_block_offset: add(add(a, b), delta),
            is_fixed_to_view: fixed,
        };
        resolved.insert(
            node,
            Resolved {
                delta,
                box_total: add(a, delta),
                containing_total: add(add(a, b), delta),
                geometry,
            },
        );
    }
    fn apply(fragment: &mut Fragment, source: NodeId, resolved: &HashMap<NodeId, Resolved>) {
        let normal = normal_offset(fragment, source);
        if fragment
            .sticky_timeline_geometry
            .is_some_and(|data| data.scroll_container == source)
        {
            fragment.offset = normal;
            fragment.sticky_timeline_geometry = None;
        }
        if fragment.owns_element_box() {
            if let Some(data) = resolved.get(&fragment.node_id) {
                let mut geometry = data.geometry;
                geometry.parent_relative_normal_offset = normal;
                fragment.sticky_timeline_geometry = Some(geometry);
                fragment.offset = add(normal, data.delta);
            }
            if fragment.viewport_scrollport.is_some()
                || fragment
                    .scroll_area
                    .is_some_and(|area| area.is_scroll_container())
            {
                return;
            }
        }
        if fragment.anonymous_layout_box.is_some()
            && fragment
                .scroll_area
                .is_some_and(|area| area.is_scroll_container())
        {
            return;
        }
        for child in &mut fragment.children {
            apply(child, source, resolved);
        }
    }
    for child in &mut scroller.children {
        apply(child, source, &resolved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FragmentBoxGeometry, FragmentBoxOwnership};
    use openui_dom::ElementTag;
    use openui_geometry::WritingDirectionMode;
    use openui_style::Overflow;
    fn lu(value: i32) -> LayoutUnit {
        LayoutUnit::from_i32(value)
    }
    fn node(doc: &mut Document, parent: NodeId, sticky: bool, inline: bool) -> NodeId {
        let node = doc.create_node(ElementTag::Div);
        doc.append_child(parent, node);
        doc.update_resolved_style(node, |style| {
            style.display = if inline {
                Display::Inline
            } else {
                Display::Block
            };
            if sticky {
                style.position = Position::Sticky;
                style.top = Length::px(0.0);
            }
        });
        node
    }
    fn box_(node: NodeId, y: i32, height: i32) -> Fragment {
        let mut fragment = Fragment::new_box(node, PhysicalSize::new(lu(100), lu(height)));
        fragment.offset.top = lu(y);
        fragment
    }
    fn source(doc: &mut Document, y: f32) -> Fragment {
        let root = doc.root();
        doc.update_resolved_style(root, |style| {
            style.display = Display::Block;
            style.overflow_x = Overflow::Hidden;
            style.overflow_y = Overflow::Hidden;
        });
        doc.node_mut(root).scroll_top = y;
        let mut fragment = box_(root, 0, 80);
        fragment.scroll_area = Some(ScrollArea {
            client_rect: PhysicalRect::from_xywh(lu(0), lu(0), lu(100), lu(80)),
            content_rect: PhysicalRect::from_xywh(lu(0), lu(0), lu(100), lu(500)),
            overflow_x: Overflow::Hidden,
            overflow_y: Overflow::Hidden,
            negative_x: false,
            negative_y: false,
            integer_offsets: true,
        });
        fragment
    }
    #[test]
    fn authored_padding_block_survives_a_same_source_anonymous_helper() {
        let mut doc = Document::new();
        let root = doc.root();
        let parent = node(&mut doc, root, false, false);
        let subject = node(&mut doc, parent, true, false);
        let mut source = source(&mut doc, 20.0);
        source.border = BoxStrut::new(lu(10), lu(10), lu(10), lu(10));
        source.padding = BoxStrut::new(lu(5), lu(5), lu(5), lu(5));
        source.scroll_area.as_mut().unwrap().client_rect =
            PhysicalRect::from_xywh(lu(10), lu(10), lu(110), lu(90));
        source.scroll_area.as_mut().unwrap().content_rect =
            PhysicalRect::from_xywh(lu(10), lu(10), lu(110), lu(310));
        let mut owner = box_(parent, 15, 60);
        let mut helper = box_(parent, 0, 5);
        helper.box_ownership = FragmentBoxOwnership::LayoutHelper;
        helper.children.push(box_(subject, 0, 20));
        owner.children.push(helper);
        source.children.push(owner);
        populate(&doc, &mut source);
        let subject = &source.children[0].children[0].children[0];
        assert_eq!(subject.offset.top, lu(20));
        let data = subject.sticky_timeline_geometry.unwrap();
        assert_eq!(data.containing_block, parent);
        assert_eq!(
            data.containing_block_rect,
            PhysicalRect::from_xywh(lu(0), lu(15), lu(100), lu(60))
        );
        assert_eq!(data.scroll_container_relative_normal_offset.top, lu(15));
    }
    #[test]
    fn box_only_and_containing_block_ancestor_channels_are_independent() {
        let mut doc = Document::new();
        let root = doc.root();
        let block = node(&mut doc, root, true, false);
        let inline = node(&mut doc, block, true, true);
        let child = node(&mut doc, inline, true, false);
        doc.update_resolved_style(inline, |style| style.top = Length::px(50.0));
        doc.update_resolved_style(child, |style| style.top = Length::px(70.0));
        let mut source = source(&mut doc, 100.0);
        let mut block_box = box_(block, 40, 100);
        let mut inline_box = box_(inline, 20, 40);
        inline_box.is_inline_box_fragment = true;
        inline_box.children.push(box_(child, 10, 20));
        block_box.children.push(inline_box);
        source.children.push(block_box);
        populate(&doc, &mut source);
        let block = &source.children[0];
        let inline = &block.children[0];
        let child = &inline.children[0];
        assert_eq!(block.offset.top, lu(100));
        assert_eq!(inline.offset.top, lu(50));
        assert_eq!(child.offset.top, lu(20));
        let data = child.sticky_timeline_geometry.unwrap();
        assert_eq!(data.nearest_sticky_box, Some(inline.node_id));
        assert_eq!(data.nearest_sticky_containing_block, Some(block.node_id));
        assert_eq!(data.total_sticky_box_offset.top, lu(40));
        assert_eq!(data.total_containing_block_offset.top, lu(100));
    }
    #[test]
    fn continued_sticky_owner_uses_stitched_extent_and_repeated_pass_keeps_normal_parts() {
        let mut doc = Document::new();
        let root = doc.root();
        let subject = node(&mut doc, root, true, false);
        let mut source = source(&mut doc, 350.0);
        for (x, consumed, height) in [(0, 0, 100), (100, 100, 100), (200, 200, 60)] {
            let mut fragment = box_(subject, 0, height);
            fragment.offset.left = lu(x);
            fragment.box_geometry = Some(FragmentBoxGeometry {
                source_size: PhysicalSize::new(lu(100), lu(260)),
                source_border: BoxStrut::zero(),
                source_padding: BoxStrut::zero(),
                stitched_size: PhysicalSize::new(lu(100), lu(260)),
                writing_direction: WritingDirectionMode::horizontal_ltr(),
                inline_size: lu(100),
                block_size: lu(height),
                consumed_block_size: lu(consumed),
                has_next: consumed < 200,
                is_repeated: false,
                continuation_record: true,
            });
            source.children.push(fragment);
        }
        populate(&doc, &mut source);
        for part in &source.children {
            assert_eq!(part.offset.top, lu(240));
            assert_eq!(
                part.sticky_timeline_geometry
                    .unwrap()
                    .sticky_box_rect
                    .height(),
                lu(260)
            );
        }
        doc.node_mut(root).scroll_top = 0.0;
        populate(&doc, &mut source);
        for part in &source.children {
            assert_eq!(part.offset.top, lu(0));
        }
    }
    #[test]
    fn negative_fractional_scroll_position_floors_after_the_integer_scroll_origin() {
        let mut doc = Document::new();
        let root = doc.root();
        let subject = node(&mut doc, root, true, false);
        let mut source = source(&mut doc, -0.01);
        let area = source.scroll_area.as_mut().unwrap();
        area.negative_y = true;
        area.content_rect = PhysicalRect::from_xywh(lu(0), lu(-100), lu(100), lu(600));
        source.children.push(box_(subject, -20, 20));
        populate(&doc, &mut source);
        assert_eq!(source.children[0].offset.top, LayoutUnit::from_raw(-1));
    }

    #[test]
    fn continued_containing_block_uses_unsliced_border_and_padding_edges() {
        let mut doc = Document::new();
        let root = doc.root();
        let parent = node(&mut doc, root, false, false);
        let subject = node(&mut doc, parent, true, false);
        let mut source = source(&mut doc, 200.0);
        let mut block = box_(parent, 0, 100);
        block.border = BoxStrut::new(lu(2), lu(0), lu(0), lu(0));
        block.padding = BoxStrut::new(lu(3), lu(0), lu(0), lu(0));
        block.box_geometry = Some(FragmentBoxGeometry {
            source_size: PhysicalSize::new(lu(100), lu(200)),
            source_border: BoxStrut::new(lu(2), lu(0), lu(4), lu(0)),
            source_padding: BoxStrut::new(lu(3), lu(0), lu(7), lu(0)),
            stitched_size: PhysicalSize::new(lu(100), lu(200)),
            writing_direction: WritingDirectionMode::horizontal_ltr(),
            inline_size: lu(100),
            block_size: lu(100),
            consumed_block_size: lu(0),
            has_next: true,
            is_repeated: false,
            continuation_record: true,
        });
        block.children.push(box_(subject, 5, 20));
        source.children.push(block);
        populate(&doc, &mut source);
        let child = &source.children[0].children[0];
        let geometry = child.sticky_timeline_geometry.unwrap();
        assert_eq!(
            geometry.containing_block_rect,
            PhysicalRect::from_xywh(lu(0), lu(5), lu(100), lu(184))
        );
        assert_eq!(child.offset.top, lu(169));
    }

    #[test]
    fn viewport_fixed_ancestor_ignores_scroll_until_a_real_layout_block_captures_it() {
        let mut doc = Document::new();
        let root = doc.root();
        let capture = node(&mut doc, root, false, false);
        let fixed = node(&mut doc, capture, false, false);
        let subject = node(&mut doc, fixed, true, false);
        doc.update_resolved_style(fixed, |style| style.position = Position::Fixed);
        doc.update_resolved_style(subject, |style| style.top = Length::px(10.0));
        let mut source = source(&mut doc, 100.0);
        let mut captured = box_(capture, 0, 300);
        let mut fixed_box = box_(fixed, 0, 100);
        fixed_box.children.push(box_(subject, 0, 20));
        captured.children.push(fixed_box);
        source.children.push(captured);
        populate(&doc, &mut source);
        let child = &source.children[0].children[0].children[0];
        assert_eq!(child.offset.top, lu(10));
        assert!(child.sticky_timeline_geometry.unwrap().is_fixed_to_view);
        doc.update_resolved_style(capture, |style| {
            style.contain = openui_style::Containment::LAYOUT
        });
        populate(&doc, &mut source);
        let child = &source.children[0].children[0].children[0];
        assert_eq!(child.offset.top, lu(80));
        assert!(!child.sticky_timeline_geometry.unwrap().is_fixed_to_view);
    }
}
