//! Scroll geometry in final physical fragment coordinates.
//!
//! Chromium's ScrollableOverflowCalculator starts with the padding rectangle,
//! adds child overflow, then includes padding around in-flow bounds. The
//! border box used for painting is not the native scroll content size.

use crate::{Fragment, FragmentKind};
use openui_dom::Document;
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_style::{
    ComputedStyle, ContainerType, Containment, Display, FlexDirection, FlexWrap, Overflow,
    OverflowClipBox, WritingMode,
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

pub(crate) fn overflow_pair(style: &ComputedStyle) -> (Overflow, Overflow) {
    let mut x = style.overflow_x;
    let mut y = style.overflow_y;
    if !matches!(x, Overflow::Visible | Overflow::Clip)
        || !matches!(y, Overflow::Visible | Overflow::Clip)
    {
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
    let border = PhysicalRect::new(PhysicalOffset::zero(), child.size);
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
    let layout_contained = style.contain.contains(Containment::LAYOUT)
        || style.container_type != ContainerType::Normal;
    if !layout_contained && style.display != Display::Inline {
        let mut content = area.content_rect;
        let applies_clip_margin = (area.overflow_x == Overflow::Clip
            && area.overflow_y == Overflow::Clip)
            || (style.has_paint_containment() && !style.is_scroll_container());
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
        } else if area.overflow_x != Overflow::Visible || style.has_paint_containment() {
            content.offset.left = LayoutUnit::zero();
            content.size.width = child.size.width;
        }
        if (area.overflow_y != Overflow::Visible || style.has_paint_containment())
            && !applies_clip_margin
        {
            content.offset.top = LayoutUnit::zero();
            content.size.height = child.size.height;
        }
        rect = union(rect, content);
    }
    let transform = style.transform;
    if transform != openui_style::Transform2D::IDENTITY {
        let origin_x = crate::resolve_length(
            &style.transform_origin.0,
            child.size.width,
            child.size.width / 2,
            child.size.width / 2,
        )
        .to_f32();
        let origin_y = crate::resolve_length(
            &style.transform_origin.1,
            child.size.height,
            child.size.height / 2,
            child.size.height / 2,
        )
        .to_f32();
        let points = [
            (rect.x(), rect.y()),
            (rect.right(), rect.y()),
            (rect.x(), rect.bottom()),
            (rect.right(), rect.bottom()),
        ];
        let mut bounds = (
            f32::INFINITY,
            f32::INFINITY,
            f32::NEG_INFINITY,
            f32::NEG_INFINITY,
        );
        for (x, y) in points {
            let x = x.to_f32() - origin_x;
            let y = y.to_f32() - origin_y;
            let next_x = transform.a * x + transform.c * y + transform.e + origin_x;
            let next_y = transform.b * x + transform.d * y + transform.f + origin_y;
            bounds.0 = bounds.0.min(next_x);
            bounds.1 = bounds.1.min(next_y);
            bounds.2 = bounds.2.max(next_x);
            bounds.3 = bounds.3.max(next_y);
        }
        rect = PhysicalRect::from_xywh(
            LayoutUnit::from_f32(bounds.0),
            LayoutUnit::from_f32(bounds.1),
            LayoutUnit::from_f32(bounds.2 - bounds.0),
            LayoutUnit::from_f32(bounds.3 - bounds.1),
        );
    }
    rect
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
    if fragment.node_id.is_none()
        || fragment.kind != FragmentKind::Box
        || fragment.skip_box_decoration
    {
        return;
    }
    let style = &doc.node(fragment.node_id).style;
    let (overflow_x, overflow_y) = overflow_pair(style);
    let (negative_x, negative_y) = directions(style);
    let client = fragment.element_scrollbars.map_or_else(
        || {
            PhysicalRect::new(
                PhysicalOffset::new(fragment.border.left, fragment.border.top),
                PhysicalSize::new(
                    fragment.padding_box_size().width.clamp_negative_to_zero(),
                    fragment.padding_box_size().height.clamp_negative_to_zero(),
                ),
            )
        },
        |scrollbars| scrollbars.client_rect(fragment),
    );
    let mut content = client;
    let mut inflow: Option<PhysicalRect> = None;
    for child in &fragment.children {
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
    if style.is_scroll_container() {
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
