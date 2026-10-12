//! Scroll geometry in final physical fragment coordinates.
//!
//! Chromium's ScrollableOverflowCalculator starts with the padding rectangle,
//! adds child overflow, then includes padding around in-flow bounds. The
//! border box used for painting is not the native scroll content size.

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

    /// Layout-owned used borders of the principal scrolling box. A captioned
    /// table places decoration on a grid helper while its wrapper owns the
    /// scrolling box; that separation must not discard the table's used edges.
    pub fn used_border(fragment: &Fragment) -> BoxStrut {
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

    /// Actual CSS padding delegated to a captioned table's grid decoration.
    /// The wrapper retains ownership of the layout client/content rectangles.
    pub fn used_padding(fragment: &Fragment) -> BoxStrut {
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

    /// Native scroll offsets are relative to the client origin, including
    /// negative offsets in reversed physical directions.
    pub fn limits(self) -> ((f64, f64), (f64, f64)) {
        let axis = |start: LayoutUnit,
                    end: LayoutUnit,
                    client_start: LayoutUnit,
                    client_end: LayoutUnit,
                    negative: bool,
                    overflow: Overflow| {
            if matches!(overflow, Overflow::Visible | Overflow::Clip) {
                (0.0, 0.0)
            } else if negative {
                ((start - client_start).to_f64().min(0.0), 0.0)
            } else {
                (0.0, (end - client_end).to_f64().max(0.0))
            }
        };
        (
            axis(
                self.content_rect.x(),
                self.content_rect.right(),
                self.client_rect.x(),
                self.client_rect.right(),
                self.negative_x,
                self.overflow_x,
            ),
            axis(
                self.content_rect.y(),
                self.content_rect.bottom(),
                self.client_rect.y(),
                self.client_rect.bottom(),
                self.negative_y,
                self.overflow_y,
            ),
        )
    }
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
pub(crate) fn is_replaced_layout_box(doc: &Document, node: NodeId) -> bool {
    let item = doc.node(node);
    item.replaced.is_some()
        || matches!(
            item.tag,
            ElementTag::Video | ElementTag::Audio | ElementTag::Canvas
        )
}

pub(crate) fn viewport_defining_element(doc: &Document) -> Option<NodeId> {
    if doc.body_overflow_is_propagated() {
        doc.body_element()
    } else {
        doc.document_element()
    }
}

/// Used overflow belongs to the retained layout box, not the specified style.
/// LayoutBlock ignores local overflow on the viewport-defining HTML/body box;
/// LayoutReplaced clips non-visible overflow without creating a scrollport.
pub fn used_overflow_pair(doc: &Document, node: NodeId) -> (Overflow, Overflow) {
    let item = doc.node(node);
    if Some(node) == viewport_defining_element(doc)
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

pub(crate) fn is_scroll_container(doc: &Document, node: NodeId) -> bool {
    is_scrollable_pair(used_overflow_pair(doc, node))
}

pub(crate) fn has_overflow_clip(doc: &Document, node: NodeId) -> bool {
    let (x, y) = used_overflow_pair(doc, node);
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

fn propagated(doc: &Document, child: &Fragment) -> PhysicalRect {
    let border = child.border_box_rect();
    let Some(area) = child.scroll_area else {
        let mut rect = child.scrollable_overflow();
        if child.node_id.is_none() {
            // Anonymous line wrappers retain the available inline size, even
            // when nowrap children extend beyond it. They have no principal
            // node or scroll area; propagate their descendants in the same
            // physical coordinates instead of treating that size as a clip.
            for descendant in &child.children {
                let mut overflow = propagated(doc, descendant);
                overflow.offset.left = overflow.offset.left + descendant.offset.left;
                overflow.offset.top = overflow.offset.top + descendant.offset.top;
                rect = union(rect, overflow);
            }
        }
        return rect;
    };
    let style = &doc.node(child.node_id).style;
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
        rect = crate::transform::transformed_overflow_rect(used.local_matrix(), rect);
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

pub(crate) fn populate(doc: &Document, fragment: &mut Fragment) {
    for child in &mut fragment.children {
        populate(doc, child);
    }
    if let Some(viewport) = fragment.viewport_scrollport {
        fragment.scroll_area = Some(ScrollArea {
            client_rect: viewport.client_rect,
            content_rect: viewport.content_rect,
            overflow_x: viewport.overflow_x,
            overflow_y: viewport.overflow_y,
            negative_x: viewport.negative_x,
            negative_y: viewport.negative_y,
            integer_offsets: false,
        });
        return;
    }
    if !fragment.owns_element_box()
        || fragment.kind != FragmentKind::Box
        || matches!(fragment.table_part, Some(FragmentTablePart::GridDecoration))
    {
        fragment.scroll_area = None;
        return;
    }
    let style = &doc.node(fragment.node_id).style;
    let (overflow_x, overflow_y) = used_overflow_pair(doc, fragment.node_id);
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
        child_rect.offset.left = child_rect.offset.left + child.offset.left;
        child_rect.offset.top = child_rect.offset.top + child.offset.top;
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
            let bounds = PhysicalRect::new(child.offset, child.size);
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
        overflow_x,
        overflow_y,
        negative_x,
        negative_y,
        integer_offsets: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FragmentBoxOwnership;

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
}
