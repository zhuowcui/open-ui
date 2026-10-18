//! Scroll geometry in final physical fragment coordinates.
//!
//! Chromium's ScrollableOverflowCalculator starts with the padding rectangle,
//! adds child overflow, then includes padding around in-flow bounds. The
//! border box used for painting is not the native scroll content size.

use crate::layout_context::LayoutDocument;
use crate::{Fragment, FragmentKind, FragmentTablePart};
use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_style::{
    BorderCollapse, ComputedStyle, ContainerType, Containment, Display, FlexDirection, FlexWrap,
    Overflow, OverflowClipBox, WritingMode,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScrollArea {
    pub client_rect: PhysicalRect,
    pub content_rect: PhysicalRect,
    /// Layout-box ClientLeft/ClientTop, excluding the principal box origin.
    /// Chromium snaps content size at this phase before deriving legal offsets.
    pub contents_snap_origin: PhysicalOffset,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub negative_x: bool,
    pub negative_y: bool,
    /// The pinned Chromium element scrollport rounds logical offsets. The
    /// root-frame viewport retains precision for its subviewports.
    pub integer_offsets: bool,
}

impl ScrollArea {
    /// Whether this retained box owns element scrolling. Viewport propagation,
    /// table adjustment, and replaced-box clipping are resolved during layout.
    pub fn is_scroll_container(self) -> bool {
        is_scrollable_pair((self.overflow_x, self.overflow_y))
    }

    /// Layout-owned used borders of the physical continuation. A captioned
    /// table delegates its source edges to the grid decoration; excluded slice
    /// sides are removed from that source for the retained scrolling box.
    pub fn used_border(fragment: &Fragment) -> BoxStrut {
        truncate_fragment_edges(fragment, Self::source_border(fragment))
    }

    /// Unsliced source borders, before continuation sides are excluded.
    /// Layout's source-geometry capture must retain these independently from
    /// the current physical fragment's client/content edges.
    pub(crate) fn source_border(fragment: &Fragment) -> BoxStrut {
        if fragment.owns_element_box() {
            if let Some(data) = &fragment.collapsed_table_border_data {
                return data
                    .borders
                    .used_table_border()
                    .to_physical(data.writing_direction);
            }
            if let Some(grid) = fragment
                .children
                .iter()
                .find(|child| matches!(child.table_part, Some(FragmentTablePart::GridDecoration)))
            {
                return grid.border;
            }
        }
        fragment.border
    }

    /// Actual CSS padding of this physical continuation, with excluded slice
    /// sides removed independently from the source padding.
    pub fn used_padding(fragment: &Fragment) -> BoxStrut {
        truncate_fragment_edges(fragment, Self::source_padding(fragment))
    }

    pub(crate) fn source_padding(fragment: &Fragment) -> BoxStrut {
        if fragment.owns_element_box() {
            if let Some(grid) = fragment
                .children
                .iter()
                .find(|child| matches!(child.table_part, Some(FragmentTablePart::GridDecoration)))
            {
                return grid.padding;
            }
        }
        fragment.padding
    }

    pub fn clamp_offset(self, x: f64, y: f64) -> (f32, f32) {
        let (x_range, y_range) = self.limits();
        let x = x.clamp(x_range.0, x_range.1);
        let y = y.clamp(y_range.0, y_range.1);
        if self.integer_offsets {
            (x.round() as f32, y.round() as f32)
        } else {
            (x as f32, y as f32)
        }
    }

    /// Legal logical offsets derived from the retained scroll origin and the
    /// independently snapped content/client sizes. These are physical axes,
    /// including negative minima and occasionally positive RTL maxima.
    /// Raw physical rectangles and ScrollMetrics retain their fractional sizes.
    pub fn limits(self) -> ((f64, f64), (f64, f64)) {
        fn snap_size(size: LayoutUnit, location: LayoutUnit) -> i32 {
            // Blink147 SnapSizeToPixel: preserve the location's signed phase.
            let fraction = location.fraction();
            let result = (fraction + size).round_i32() - fraction.round_i32();
            if result == 0 && (size.raw() > 4 || size.raw() < -4) {
                if size > LayoutUnit::zero() {
                    1
                } else {
                    -1
                }
            } else {
                result
            }
        }
        let axis = |content_start: LayoutUnit,
                    content_size: LayoutUnit,
                    client_start: LayoutUnit,
                    client_size: LayoutUnit,
                    snap_origin: LayoutUnit,
                    overflow: Overflow| {
            if matches!(overflow, Overflow::Visible | Overflow::Clip) {
                return (0.0, 0.0);
            }
            // UpdateScrollOrigin floors the distance before integer scrollbar
            // origin adjustment. client_start already includes that adjustment.
            let origin = (client_start - content_start).floor_i32();
            let minimum = origin.saturating_neg();
            let visible = client_size.round_i32().max(0);
            let content = snap_size(content_size, snap_origin).max(visible);
            let maximum = minimum.saturating_add(content.saturating_sub(visible));
            (minimum as f64, maximum as f64)
        };
        (
            axis(
                self.content_rect.x(),
                self.content_rect.width(),
                self.client_rect.x(),
                self.client_rect.width(),
                self.contents_snap_origin.left,
                self.overflow_x,
            ),
            axis(
                self.content_rect.y(),
                self.content_rect.height(),
                self.client_rect.y(),
                self.client_rect.height(),
                self.contents_snap_origin.top,
                self.overflow_y,
            ),
        )
    }
}

/// Blink147 BoxFragmentBuilder::ApplicableBorders/ApplicablePadding removes
/// excluded logical block sides before storing physical fragment struts. Native
/// Fragment keeps the source struts for decoration/source geometry, so project
/// that same exclusion only at the used-edge read. Clone pieces keep both
/// first/last decoration flags; inline layout already stores used physical sides.
fn truncate_fragment_edges(fragment: &Fragment, mut edges: BoxStrut) -> BoxStrut {
    if !fragment.owns_layout_box() || fragment.is_inline_box_fragment {
        return edges;
    }
    match fragment.fragmentation_writing_direction {
        Some(direction) if !direction.is_horizontal() && direction.is_flipped_blocks() => {
            if !fragment.is_first_for_node {
                edges.right = LayoutUnit::zero();
            }
            if !fragment.is_last_for_node {
                edges.left = LayoutUnit::zero();
            }
        }
        Some(direction) if !direction.is_horizontal() => {
            if !fragment.is_first_for_node {
                edges.left = LayoutUnit::zero();
            }
            if !fragment.is_last_for_node {
                edges.right = LayoutUnit::zero();
            }
        }
        _ => {
            if !fragment.is_first_for_node {
                edges.top = LayoutUnit::zero();
            }
            if !fragment.is_last_for_node {
                edges.bottom = LayoutUnit::zero();
            }
        }
    }
    edges
}

fn union(a: PhysicalRect, b: PhysicalRect) -> PhysicalRect {
    // Empty border boxes still contribute their edge coordinates.
    let left = a.x().min_of(b.x());
    let top = a.y().min_of(b.y());
    let right = a.right().max_of(b.right());
    let bottom = a.bottom().max_of(b.bottom());
    PhysicalRect::from_xywh(left, top, right - left, bottom - top)
}

fn directions(style: &ComputedStyle) -> (bool, bool) {
    let direction = style.direction.writing_direction(style.writing_mode);
    let mut x = if direction.is_horizontal() {
        direction.is_rtl()
    } else {
        direction.is_flipped_blocks()
    };
    let mut y = !direction.is_horizontal()
        && (direction.is_rtl() != (style.writing_mode == WritingMode::SidewaysLr));
    if matches!(style.display, Display::Flex | Display::InlineFlex) {
        let column = matches!(
            style.flex_direction,
            FlexDirection::Column | FlexDirection::ColumnReverse
        );
        let reverse = matches!(
            style.flex_direction,
            FlexDirection::RowReverse | FlexDirection::ColumnReverse
        );
        let inline_reverse = if column {
            style.flex_wrap == FlexWrap::WrapReverse
        } else {
            reverse
        };
        let block_reverse = if column {
            reverse
        } else {
            style.flex_wrap == FlexWrap::WrapReverse
        };
        if direction.is_horizontal() {
            x ^= inline_reverse;
            y ^= block_reverse;
        } else {
            x ^= block_reverse;
            y ^= inline_reverse;
        }
    }
    (x, y)
}

pub fn overflow_pair(style: &ComputedStyle) -> (Overflow, Overflow) {
    let mut x = style.overflow_x;
    let mut y = style.overflow_y;
    if !matches!(x, Overflow::Visible | Overflow::Clip)
        || !matches!(y, Overflow::Visible | Overflow::Clip)
    {
        // Blink StyleAdjuster::AdjustOverflow: table boxes retain only a
        // matching hidden pair; other non-visible/non-clip pairs become visible.
        // Internal cells/captions remain block containers and use normal rules.
        if style.display.is_table_wrapper() {
            return if x == Overflow::Hidden && y == Overflow::Hidden {
                (Overflow::Hidden, Overflow::Hidden)
            } else {
                (Overflow::Visible, Overflow::Visible)
            };
        }
        let computed = |value| match value {
            Overflow::Visible => Overflow::Auto,
            Overflow::Clip => Overflow::Hidden,
            other => other,
        };
        x = computed(x);
        y = computed(y);
    }
    (x, y)
}

fn is_scrollable_pair((x, y): (Overflow, Overflow)) -> bool {
    [x, y]
        .iter()
        .any(|value| matches!(value, Overflow::Hidden | Overflow::Auto | Overflow::Scroll))
}

/// Resource metadata marks decoded images and embedded resource boxes. Native
/// passive media and canvas retain their replaced layout class even before a
/// resource is installed. A failed image instead uses its block-flow fallback
/// class, as HTMLImageElement::CreateLayoutObject does in the pinned source.
pub(crate) fn is_replaced_layout_box(doc: &dyn LayoutDocument, node: NodeId) -> bool {
    let item = doc.node(node);
    item.replaced.is_some()
        || matches!(
            item.tag,
            ElementTag::Video | ElementTag::Audio | ElementTag::Canvas
        )
}

pub(crate) fn viewport_defining_element(doc: &dyn LayoutDocument) -> Option<NodeId> {
    if doc.body_overflow_is_propagated() {
        doc.body_element()
    } else {
        doc.document_element()
    }
}

/// Used overflow belongs to the retained layout box, not the specified style.
/// LayoutBlock ignores local overflow on the viewport-defining HTML/body box;
/// LayoutReplaced clips non-visible overflow without creating a scrollport.
pub(crate) fn used_overflow_pair_with_layout(
    doc: &dyn LayoutDocument,
    node: NodeId,
) -> (Overflow, Overflow) {
    let item = doc.node(node);
    if (item.tag == ElementTag::Fieldset
        && doc.layout_box_role(node) == crate::layout_context::LayoutBoxRole::Authored)
        || Some(node) == viewport_defining_element(doc)
        || matches!(
            item.style.display,
            Display::Inline
                | Display::TableRow
                | Display::TableRowGroup
                | Display::TableHeaderGroup
                | Display::TableFooterGroup
                | Display::TableColumn
                | Display::TableColumnGroup
        ) && !is_replaced_layout_box(doc, node)
    {
        return (Overflow::Visible, Overflow::Visible);
    }
    let pair = overflow_pair(&item.style);
    if is_replaced_layout_box(doc, node) {
        let clip = |value| {
            if value == Overflow::Visible {
                value
            } else {
                Overflow::Clip
            }
        };
        (clip(pair.0), clip(pair.1))
    } else {
        pair
    }
}

pub(crate) fn is_scroll_container(doc: &dyn LayoutDocument, node: NodeId) -> bool {
    is_scrollable_pair(used_overflow_pair_with_layout(doc, node))
}

pub(crate) fn has_overflow_clip(doc: &dyn LayoutDocument, node: NodeId) -> bool {
    let (x, y) = used_overflow_pair_with_layout(doc, node);
    x != Overflow::Visible || y != Overflow::Visible
}

fn reachable(rect: PhysicalRect, client: PhysicalRect, x: bool, y: bool) -> PhysicalRect {
    let left = if x {
        rect.x().min_of(client.right())
    } else {
        rect.x().max_of(client.x())
    };
    let right = if x {
        rect.right().min_of(client.right())
    } else {
        rect.right().max_of(client.x())
    };
    let top = if y {
        rect.y().min_of(client.bottom())
    } else {
        rect.y().max_of(client.y())
    };
    let bottom = if y {
        rect.bottom().min_of(client.bottom())
    } else {
        rect.bottom().max_of(client.y())
    };
    PhysicalRect::from_xywh(left, top, right - left, bottom - top)
}

fn scrollable_child_offset(child: &Fragment) -> PhysicalOffset {
    // Sticky is a paint/scroll adjustment. Its retained normal offset owns
    // scrollable overflow, just like Blink's unchanged fragment child offset.
    child
        .sticky_timeline_geometry
        .map_or(child.offset, |geometry| {
            geometry.parent_relative_normal_offset
        })
}

fn propagated(doc: &dyn LayoutDocument, child: &Fragment) -> PhysicalRect {
    let border = child.border_box_rect();
    let Some(area) = child.scroll_area else {
        let mut rect = child.scrollable_overflow();
        if child.node_id.is_none() || !child.owns_element_box() {
            // Anonymous line wrappers and borrowed layout helpers retain the available inline size, even
            // when nowrap children extend beyond it. They have no principal
            // node or scroll area; propagate their descendants in the same
            // physical coordinates instead of treating that size as a clip.
            for descendant in &child.children {
                let mut overflow = propagated(doc, descendant);
                let offset = scrollable_child_offset(descendant);
                overflow.offset.left = overflow.offset.left + offset.left;
                overflow.offset.top = overflow.offset.top + offset.top;
                rect = union(rect, overflow);
            }
        }
        return rect;
    };
    let style = child.layout_style(&doc.node(child.node_id).style);
    let mut rect = border;
    // Paint containment establishes a formatting context, but its clipped
    // descendants still contribute to ancestor scrolling through clip margins.
    // Only actual layout containment suppresses their overflow propagation.
    let owned = child.owns_element_box();
    let paint_contained = owned && style.has_paint_containment();
    let layout_contained = owned
        && (style.contain.contains(Containment::LAYOUT)
            || style.container_type != ContainerType::Normal);
    if !layout_contained && style.display != Display::Inline {
        let mut content = area.content_rect;
        let applies_clip_margin = owned
            && ((area.overflow_x == Overflow::Clip && area.overflow_y == Overflow::Clip)
                || (paint_contained && !style.is_scroll_container()));
        if applies_clip_margin {
            // A clip margin exposes child overflow to enclosing scrollports.
            // Preserve the actual child extent, intersect it with the authored
            // visual box plus margin, then retain the child's own border box.
            // Blink: ScrollableOverflowCalculator::ScrollableOverflowForChild.
            content = union(content, child.scrollable_overflow());
            let margin = LayoutUnit::from_f32(style.overflow_clip_margin.max(0.0));
            let insets = match style.overflow_clip_box {
                OverflowClipBox::BorderBox => BoxStrut::zero(),
                OverflowClipBox::PaddingBox => child.border,
                OverflowClipBox::ContentBox => BoxStrut::new(
                    child.border.top + child.padding.top,
                    child.border.right + child.padding.right,
                    child.border.bottom + child.padding.bottom,
                    child.border.left + child.padding.left,
                ),
            };
            let mut outsets = BoxStrut::new(
                margin - insets.top,
                margin - insets.right,
                margin - insets.bottom,
                margin - insets.left,
            );
            if let Some(direction) = child.fragmentation_writing_direction {
                if direction.is_horizontal() {
                    if !child.is_first_for_node {
                        outsets.top = LayoutUnit::zero();
                    }
                    if !child.is_last_for_node {
                        outsets.bottom = LayoutUnit::zero();
                    }
                } else if direction.is_flipped_blocks() {
                    if !child.is_first_for_node {
                        outsets.right = LayoutUnit::zero();
                    }
                    if !child.is_last_for_node {
                        outsets.left = LayoutUnit::zero();
                    }
                } else {
                    if !child.is_first_for_node {
                        outsets.left = LayoutUnit::zero();
                    }
                    if !child.is_last_for_node {
                        outsets.right = LayoutUnit::zero();
                    }
                }
            }
            let left = content.x().max_of(-outsets.left);
            let top = content.y().max_of(-outsets.top);
            let right = content.right().min_of(child.size.width + outsets.right);
            let bottom = content.bottom().min_of(child.size.height + outsets.bottom);
            content = PhysicalRect::from_xywh(
                left,
                top,
                (right - left).clamp_negative_to_zero(),
                (bottom - top).clamp_negative_to_zero(),
            );
        } else if area.overflow_x != Overflow::Visible || paint_contained {
            content.offset.left = LayoutUnit::zero();
            content.size.width = child.size.width;
        }
        if (area.overflow_y != Overflow::Visible || paint_contained) && !applies_clip_margin {
            content.offset.top = LayoutUnit::zero();
            content.size.height = child.size.height;
        }
        rect = union(rect, content);
    }
    if owned && style.has_transform() {
        let used = crate::resolved_fragment_transform(child, style);
        rect = crate::transform::transformed_overflow_rect(used.local_applied, rect);
    }
    rect
}

/// One physical client rectangle for retained scrolling and public scrollbar
/// geometry, including captioned-table used borders and owned continuations.
pub(crate) fn client_rect(fragment: &Fragment, gutters: BoxStrut) -> PhysicalRect {
    let border_box = fragment.border_box_rect();
    let border = ScrollArea::used_border(fragment);
    PhysicalRect::new(
        PhysicalOffset::new(
            border_box.x() + border.left + gutters.left,
            border_box.y() + border.top + gutters.top,
        ),
        PhysicalSize::new(
            (border_box.width() - border.inline_sum() - gutters.inline_sum())
                .clamp_negative_to_zero(),
            (border_box.height() - border.block_sum() - gutters.block_sum())
                .clamp_negative_to_zero(),
        ),
    )
}

pub(crate) fn populate(doc: &dyn LayoutDocument, fragment: &mut Fragment) {
    for child in &mut fragment.children {
        populate(doc, child);
    }
    if let Some(mut viewport) = fragment.viewport_scrollport {
        viewport.content_rect = viewport_content_rect(
            doc,
            fragment,
            viewport.layout_box_rect,
            viewport.client_rect,
            viewport.negative_x,
            viewport.negative_y,
        );
        fragment.viewport_scrollport = Some(viewport);
        fragment.scroll_area = Some(ScrollArea {
            client_rect: viewport.client_rect,
            content_rect: viewport.content_rect,
            contents_snap_origin: viewport.client_rect.offset,
            overflow_x: viewport.overflow_x,
            overflow_y: viewport.overflow_y,
            negative_x: viewport.negative_x,
            negative_y: viewport.negative_y,
            integer_offsets: false,
        });
        return;
    }
    if !fragment.owns_layout_box()
        || fragment.kind != FragmentKind::Box
        || matches!(fragment.table_part, Some(FragmentTablePart::GridDecoration))
    {
        fragment.scroll_area = None;
        return;
    }
    let style = fragment.layout_style(&doc.node(fragment.node_id).style);
    let (overflow_x, overflow_y) = if fragment.anonymous_layout_box.is_some() {
        overflow_pair(style)
    } else {
        used_overflow_pair_with_layout(doc, fragment.node_id)
    };
    let (negative_x, negative_y) = directions(style);
    let border_box = fragment.border_box_rect();
    let gutters = fragment
        .element_scrollbars
        .map_or(BoxStrut::zero(), |bars| bars.insets);
    let client = client_rect(fragment, gutters);
    let mut content = client;
    let mut inflow: Option<PhysicalRect> = None;
    for child in fragment
        .children
        .iter()
        .filter(|_| !is_replaced_layout_box(doc, fragment.node_id))
    {
        // This empty native paint helper has no counterpart among Chromium's
        // CSS table children. The wrapper, captions, rows and cells own the
        // real extents; the grid border contributes separately when collapsed.
        if matches!(child.table_part, Some(FragmentTablePart::GridDecoration)) {
            continue;
        }
        let mut child_rect = propagated(doc, child);
        let offset = scrollable_child_offset(child);
        child_rect.offset.left = child_rect.offset.left + offset.left;
        child_rect.offset.top = child_rect.offset.top + offset.top;
        content = union(
            content,
            reachable(child_rect, client, negative_x, negative_y),
        );
        if child.node_id.is_none()
            || !doc
                .node(child.node_id)
                .style
                .position
                .is_absolutely_positioned()
        {
            let bounds = PhysicalRect::new(offset, child.size);
            inflow = Some(inflow.map_or(bounds, |current| union(current, bounds)));
        }
    }
    if is_scrollable_pair((overflow_x, overflow_y)) {
        if let Some(inflow) = inflow {
            let padding = PhysicalRect::from_xywh(
                inflow.x() - fragment.padding.left,
                inflow.y() - fragment.padding.top,
                inflow.width() + fragment.padding.left + fragment.padding.right,
                inflow.height() + fragment.padding.top + fragment.padding.bottom,
            );
            content = union(content, reachable(padding, client, negative_x, negative_y));
        }
    }
    // ScrollableOverflowCalculator::AddTableSelfRect includes the table's
    // own box when its borders are collapsed. Separated borders are excluded.
    if fragment.owns_element_box()
        && matches!(style.display, Display::Table | Display::InlineTable)
        && style.border_collapse == BorderCollapse::Collapse
    {
        content = union(
            content,
            reachable(border_box, client, negative_x, negative_y),
        );
    }
    fragment.scroll_area = Some(ScrollArea {
        client_rect: client,
        content_rect: content,
        contents_snap_origin: PhysicalOffset::new(
            client.x() - border_box.x(),
            client.y() - border_box.y(),
        ),
        overflow_x,
        overflow_y,
        negative_x,
        negative_y,
        integer_offsets: true,
    });
}

/// Recompute viewport overflow from its actual principal layout rectangle and
/// current descendants. The later restored outside size includes gutters and
/// is not a substitute for that original own size/padding contribution.
pub(crate) fn viewport_content_rect(
    doc: &dyn LayoutDocument,
    fragment: &Fragment,
    layout_box_rect: PhysicalRect,
    client: PhysicalRect,
    negative_x: bool,
    negative_y: bool,
) -> PhysicalRect {
    let mut content = union(
        client,
        reachable(layout_box_rect, client, negative_x, negative_y),
    );
    let root = doc.root();
    let root_establishes_transform_containing_block =
        doc.node(root).style.has_transform_containing_block();
    for child in &fragment.children {
        // Chromium ScrollableOverflowCalculator::AddChild excludes a fixed
        // direct child of the view. Resolve that ownership from the actual
        // positioned metadata; a fixed box in a transformed CB still contributes
        // through its ordinary ancestor, and unresolved/helper boxes remain.
        if child.owns_element_box() {
            let style = child.layout_style(&doc.node(child.node_id).style);
            if child.is_fixed_to_viewport(style, root, root_establishes_transform_containing_block)
            {
                continue;
            }
        }
        let mut rect = propagated(doc, child);
        let offset = scrollable_child_offset(child);
        rect.offset.left = rect.offset.left + offset.left;
        rect.offset.top = rect.offset.top + offset.top;
        content = union(content, reachable(rect, client, negative_x, negative_y));
    }
    content
}

/// Dependencies discovered while refreshing retained transform overflow.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetainedOverflowUpdate {
    pub transforms_changed: bool,
    pub ranges_changed: bool,
    /// Actual scrollbar space or conservative projective geometry requires
    /// layout. Ordinary two-dimensional transform updates retain box sizing.
    pub requires_layout: bool,
}

/// Refresh the owned transform and scroll geometry without measuring boxes.
/// Blink147 RecalcScrollableOverflowNG uses the existing fragment tree; an auto
/// scrollbar existence change schedules layout before publishing dimensions.
pub fn refresh_fragment_overflow(
    document: &Document,
    fragment: &mut Fragment,
) -> RetainedOverflowUpdate {
    fn transforms(document: &Document, fragment: &Fragment, update: &mut RetainedOverflowUpdate) {
        if fragment.owns_element_box() {
            let style = &document.node(fragment.node_id).style;
            let resolved = crate::resolved_fragment_transform(fragment, style);
            if fragment.resolved_transform != Some(resolved) {
                update.transforms_changed = true;
                if !resolved.local_applied.is_2d()
                    || fragment
                        .resolved_transform
                        .is_some_and(|old| !old.local_applied.is_2d())
                    || style
                        .native_transform
                        .as_ref()
                        .is_some_and(openui_style::transform_contains_3d)
                {
                    update.requires_layout = true;
                }
            }
        }
        for child in &fragment.children {
            transforms(document, child, update);
        }
    }
    fn old_areas(fragment: &Fragment, values: &mut Vec<Option<ScrollArea>>) {
        values.push(fragment.scroll_area);
        for child in &fragment.children {
            old_areas(child, values);
        }
    }
    fn check(
        document: &Document,
        fragment: &Fragment,
        previous: &mut impl Iterator<Item = Option<ScrollArea>>,
        update: &mut RetainedOverflowUpdate,
    ) {
        update.ranges_changed |=
            previous.next().expect("same retained fragment tree") != fragment.scroll_area;
        update.requires_layout |= crate::scrollbars::retained_requires_layout(document, fragment);
        if let Some(viewport) = fragment.viewport_scrollport {
            let enabled = viewport.scrollbar_thickness > LayoutUnit::zero();
            let horizontal = enabled
                && (viewport.overflow_x == Overflow::Scroll
                    || viewport.overflow_x == Overflow::Auto
                        && (viewport.content_rect.x() < viewport.client_rect.x()
                            || viewport.content_rect.right() > viewport.client_rect.right()));
            let vertical = enabled
                && (viewport.overflow_y == Overflow::Scroll
                    || viewport.overflow_y == Overflow::Auto
                        && (viewport.content_rect.y() < viewport.client_rect.y()
                            || viewport.content_rect.bottom() > viewport.client_rect.bottom()));
            update.requires_layout |= (horizontal, vertical)
                != (viewport.horizontal_scrollbar, viewport.vertical_scrollbar);
        }
        for child in &fragment.children {
            check(document, child, previous, update);
        }
    }
    let mut update = RetainedOverflowUpdate::default();
    transforms(document, fragment, &mut update);
    let mut previous = Vec::new();
    old_areas(fragment, &mut previous);
    crate::finalize_fragment_transforms(document, fragment);
    populate(document, fragment);
    check(document, fragment, &mut previous.into_iter(), &mut update);
    update
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FragmentBoxOwnership;

    #[test]
    fn retained_refresh_traverses_borrowed_helpers_without_duplicate_transforms() {
        let mut doc = Document::new();
        let owner = doc.create_node(ElementTag::Div);
        let child = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(owner, |style| {
            style.display = Display::Block;
            style.overflow_x = Overflow::Hidden;
            style.overflow_y = Overflow::Hidden;
        });
        doc.update_resolved_style(child, |style| style.display = Display::Block);
        let lu = LayoutUnit::from_i32;
        let mut fragment = Fragment::new_box(owner, PhysicalSize::new(lu(200), lu(150)));
        let mut helper = Fragment::new_box(owner, PhysicalSize::new(lu(100), lu(50)));
        helper.box_ownership = FragmentBoxOwnership::LayoutHelper;
        helper
            .children
            .push(Fragment::new_box(child, PhysicalSize::new(lu(100), lu(50))));
        fragment.children.push(helper);
        populate(&doc, &mut fragment);
        crate::finalize_fragment_transforms(&doc, &mut fragment);
        doc.update_resolved_style(child, |style| {
            style.native_transform = Some(openui_style::TransformList(vec![
                openui_style::TransformOperation::Scale(4.0, 4.0),
            ]));
        });
        let update = refresh_fragment_overflow(&doc, &mut fragment);
        assert!(update.transforms_changed && update.ranges_changed);
        assert!(!update.requires_layout);
        assert_eq!(fragment.scroll_area.unwrap().content_rect.width(), lu(250));
        assert!(fragment.children[0].scroll_area.is_none());
        assert_eq!(
            refresh_fragment_overflow(&doc, &mut fragment),
            RetainedOverflowUpdate::default()
        );
    }

    #[test]
    fn legal_bounds_snap_each_size_and_retain_the_fractional_rtl_origin() {
        let lu = LayoutUnit::from_f32;
        let rect = |x, width| PhysicalRect::from_xywh(lu(x), lu(0.0), lu(width), lu(80.75));
        for (client, content, expected_ltr, expected_rtl) in [
            (100.75, 180.25, (0.0, 79.0), (-79.0, 0.0)),
            (100.25, 180.75, (0.0, 81.0), (-80.0, 1.0)),
        ] {
            let mut area = ScrollArea {
                client_rect: rect(0.0, client),
                content_rect: PhysicalRect::from_xywh(lu(0.0), lu(0.0), lu(content), lu(160.25)),
                contents_snap_origin: PhysicalOffset::zero(),
                overflow_x: Overflow::Auto,
                overflow_y: Overflow::Auto,
                negative_x: false,
                negative_y: false,
                integer_offsets: true,
            };
            assert_eq!(area.limits(), (expected_ltr, (0.0, 79.0)));
            assert_eq!(area.clamp_offset(1e6, 1e6), (expected_ltr.1 as f32, 79.0));
            area.negative_x = true;
            area.content_rect.offset.left = lu(client - content);
            assert_eq!(area.limits(), (expected_rtl, (0.0, 79.0)));
            assert_eq!(area.clamp_offset(-1e6, -1e6), (expected_rtl.0 as f32, 0.0));
            assert_eq!(area.clamp_offset(1e6, 1e6), (expected_rtl.1 as f32, 79.0));
            area.overflow_x = Overflow::Clip;
            assert_eq!(area.limits().0, (0.0, 0.0));
        }
    }

    #[test]
    fn content_snapping_phase_excludes_the_principal_box_origin() {
        let lu = LayoutUnit::from_f32;
        let mut area = ScrollArea {
            client_rect: PhysicalRect::from_xywh(lu(23.5), lu(11.0), lu(100.25), lu(80.0)),
            content_rect: PhysicalRect::from_xywh(lu(23.5), lu(11.0), lu(180.75), lu(160.0)),
            contents_snap_origin: PhysicalOffset::new(lu(0.5), lu(0.0)),
            overflow_x: Overflow::Auto,
            overflow_y: Overflow::Auto,
            negative_x: false,
            negative_y: false,
            integer_offsets: true,
        };
        // Actual ClientLeft=.5 snaps180.75 to180; the caption/box position23
        // is not an additional content-size phase.
        assert_eq!(area.limits(), ((0.0, 80.0), (0.0, 80.0)));
        area.client_rect.offset.left = lu(49.5);
        area.content_rect.offset.left = lu(49.5);
        assert_eq!(area.limits(), ((0.0, 80.0), (0.0, 80.0)));
        area.contents_snap_origin.left = lu(0.0);
        assert_eq!(area.limits().0, (0.0, 81.0));
    }

    #[test]
    fn borrowed_style_helper_does_not_hide_native_ancestor_scroll_extent() {
        let mut doc = Document::new();
        let owner = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(owner, |style| {
            style.display = Display::Block;
            style.overflow_x = Overflow::Hidden;
            style.overflow_y = Overflow::Hidden;
        });
        let mut parent = Fragment::new_box(
            owner,
            PhysicalSize::new(LayoutUnit::from_i32(30), LayoutUnit::from_i32(10)),
        );
        let mut helper = Fragment::new_box(
            owner,
            PhysicalSize::new(LayoutUnit::from_i32(20), LayoutUnit::from_i32(10)),
        );
        helper.box_ownership = FragmentBoxOwnership::LayoutHelper;
        helper.children.push(Fragment::new_box(
            NodeId::NONE,
            PhysicalSize::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(10)),
        ));
        parent.children.push(helper);
        populate(&doc, &mut parent);
        assert!(parent.children[0].scroll_area.is_none());
        let area = parent.scroll_area.unwrap();
        assert_eq!(area.content_rect.width(), LayoutUnit::from_i32(100));
        assert_eq!(area.limits(), ((0.0, 70.0), (0.0, 0.0)));
    }
    #[test]
    fn continuation_used_edges_exclude_sides_without_mutating_source_or_clone_edges() {
        let mut doc = Document::new();
        let node = doc.create_node(ElementTag::Div);
        let lu = LayoutUnit::from_i32;
        let border = BoxStrut::new(lu(11), lu(13), lu(17), lu(19));
        let padding = BoxStrut::new(lu(3), lu(5), lu(7), lu(9));
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            for (first, last) in [(true, false), (false, false), (false, true), (true, true)] {
                let mut part = Fragment::new_box(node, PhysicalSize::new(lu(100), lu(80)));
                part.border = border;
                part.padding = padding;
                part.is_first_for_node = first;
                part.is_last_for_node = last;
                part.fragmentation_writing_direction =
                    Some(doc.node(node).style.direction.writing_direction(mode));
                let expected = |source: BoxStrut| match mode {
                    WritingMode::HorizontalTb => BoxStrut::new(
                        if first { source.top } else { lu(0) },
                        source.right,
                        if last { source.bottom } else { lu(0) },
                        source.left,
                    ),
                    WritingMode::VerticalRl => BoxStrut::new(
                        source.top,
                        if first { source.right } else { lu(0) },
                        source.bottom,
                        if last { source.left } else { lu(0) },
                    ),
                    WritingMode::VerticalLr => BoxStrut::new(
                        source.top,
                        if last { source.right } else { lu(0) },
                        source.bottom,
                        if first { source.left } else { lu(0) },
                    ),
                    _ => unreachable!(),
                };
                assert_eq!(ScrollArea::used_border(&part), expected(border));
                assert_eq!(ScrollArea::used_padding(&part), expected(padding));
                assert_eq!(ScrollArea::source_border(&part), border);
                assert_eq!(ScrollArea::source_padding(&part), padding);
                assert_eq!(part.border, border);
                assert_eq!(part.padding, padding);
                // Borrowed helpers do not acquire the source element's slice
                // policy; their stored structural edges remain authoritative.
                part.box_ownership = FragmentBoxOwnership::LayoutHelper;
                assert_eq!(ScrollArea::used_border(&part), border);
                assert_eq!(ScrollArea::used_padding(&part), padding);
                part.box_ownership = FragmentBoxOwnership::AuthoredElement;
                part.is_inline_box_fragment = true;
                assert_eq!(ScrollArea::used_border(&part), border);
                assert_eq!(ScrollArea::used_padding(&part), padding);
            }
        }
    }
}

// Public entry points retain the original native Document signatures.
pub fn used_overflow_pair(doc: &Document, node: NodeId) -> (Overflow, Overflow) {
    used_overflow_pair_with_layout(doc, node)
}

#[cfg(test)]
mod viewport_fixed_metadata_guards {
    use super::*;

    #[test]
    fn missing_positioned_metadata_is_not_a_viewport_fixed_proof() {
        let mut doc = Document::new();
        let root = doc.root();
        let fixed = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(fixed, |style| {
            style.display = Display::Block;
            style.position = openui_style::Position::Fixed;
        });
        let lu = LayoutUnit::from_i32;
        let mut fragment = Fragment::new_box(root, PhysicalSize::new(lu(785), lu(600)));
        let child = Fragment::new_box(fixed, PhysicalSize::new(lu(800), lu(50)));
        assert!(child.owns_element_box());
        assert!(child.positioned_fragmentation.is_none());
        assert!(!child.is_fixed_to_viewport(&doc.node(fixed).style, root, false));
        fragment.children.push(child);
        let client = PhysicalRect::new(PhysicalOffset::zero(), fragment.size);
        let content = viewport_content_rect(&doc, &fragment, client, client, false, false);
        assert_eq!(content.size, PhysicalSize::new(lu(800), lu(600)));
        // An ordinary helper may borrow the same Fixed style. It also keeps
        // its retained extent without acquiring authored viewport ownership.
        fragment.children[0].box_ownership = crate::FragmentBoxOwnership::LayoutHelper;
        let content = viewport_content_rect(&doc, &fragment, client, client, false, false);
        assert_eq!(content.size, PhysicalSize::new(lu(800), lu(600)));
    }
}
