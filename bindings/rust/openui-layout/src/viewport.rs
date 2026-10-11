//! Layout of the initial containing block and its native scrollport.

use crate::{block::block_layout_contents, ConstraintSpace, Fragment, ViewportScrollport};
use openui_dom::{Document, NodeId};
use openui_geometry::{LayoutUnit, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_style::{Overflow, ScrollbarWidth, WritingMode};

pub(crate) fn layout(doc: &Document, root: NodeId, space: &ConstraintSpace) -> Fragment {
    let source = if doc.body_overflow_is_propagated() {
        doc.body_element().unwrap_or(root)
    } else {
        doc.document_element().unwrap_or(root)
    };
    let style = &doc.node(source).style;
    // CSS Overflow: visible on the viewport is auto, and clip is hidden.
    // In particular, a clipped viewport can still be scrolled by a native
    // application, though it exposes no user-operated scrollbar.
    let viewport_overflow = |overflow| match overflow {
        Overflow::Visible => Overflow::Auto,
        Overflow::Clip => Overflow::Hidden,
        other => other,
    };
    let overflow_x = viewport_overflow(style.overflow_x);
    let overflow_y = viewport_overflow(style.overflow_y);
    let direction = style.direction.writing_direction(style.writing_mode);
    let negative_x = if direction.is_horizontal() {
        direction.is_rtl()
    } else {
        direction.is_flipped_blocks()
    };
    let negative_y = !direction.is_horizontal()
        && (direction.is_rtl() != (style.writing_mode == WritingMode::SidewaysLr));
    let thickness = LayoutUnit::from_i32(match style.scrollbar_width {
        ScrollbarWidth::Auto => 15,
        ScrollbarWidth::Thin => 10,
        ScrollbarWidth::None => 0,
    });
    let outer_size = if space.writing_direction.is_horizontal() {
        PhysicalSize::new(space.available_inline_size, space.available_block_size)
    } else {
        PhysicalSize::new(space.available_block_size, space.available_inline_size)
    };
    let mut show_x = overflow_x == Overflow::Scroll && thickness > LayoutUnit::zero();
    let mut show_y = overflow_y == Overflow::Scroll && thickness > LayoutUnit::zero();
    // Reserving one scrollbar can make the other axis overflow. Re-layout
    // against the reduced initial containing block until both agree.
    loop {
        let client = PhysicalSize::new(
            (outer_size.width
                - if show_y {
                    thickness
                } else {
                    LayoutUnit::zero()
                })
            .clamp_negative_to_zero(),
            (outer_size.height
                - if show_x {
                    thickness
                } else {
                    LayoutUnit::zero()
                })
            .clamp_negative_to_zero(),
        );
        let mut client_space = space.clone();
        let (inline_size, block_size) = if space.writing_direction.is_horizontal() {
            (client.width, client.height)
        } else {
            (client.height, client.width)
        };
        client_space.available_inline_size = inline_size;
        client_space.available_block_size = block_size;
        client_space.percentage_resolution_inline_size = inline_size;
        client_space.percentage_resolution_block_size = block_size;
        let mut fragment = block_layout_contents(doc, root, &client_space);
        let raw_content = fragment.scrollable_overflow();
        // Overflow before the scroll origin is unreachable in a positive
        // axis; overflow after it is unreachable in a negative axis. Ink in
        // that region must not create a scrollbar or expand the scroll range.
        let left = if negative_x {
            raw_content.x().min_of(LayoutUnit::zero())
        } else {
            LayoutUnit::zero()
        };
        let top = if negative_y {
            raw_content.y().min_of(LayoutUnit::zero())
        } else {
            LayoutUnit::zero()
        };
        let right = if negative_x {
            client.width
        } else {
            raw_content.right().max_of(client.width)
        };
        let bottom = if negative_y {
            client.height
        } else {
            raw_content.bottom().max_of(client.height)
        };
        let content = PhysicalRect::new(
            PhysicalOffset::new(left, top),
            PhysicalSize::new(right - left, bottom - top),
        );
        let next_x = show_x
            || (overflow_x == Overflow::Auto
                && thickness > LayoutUnit::zero()
                && (content.x() < LayoutUnit::zero() || content.right() > client.width));
        let next_y = show_y
            || (overflow_y == Overflow::Auto
                && thickness > LayoutUnit::zero()
                && (content.y() < LayoutUnit::zero() || content.bottom() > client.height));
        if (next_x, next_y) != (show_x, show_y) {
            show_x = next_x;
            show_y = next_y;
            continue;
        }
        fragment.viewport_scrollport = Some(ViewportScrollport {
            client_rect: PhysicalRect::new(PhysicalOffset::zero(), client),
            content_rect: content,
            horizontal_scrollbar: show_x,
            vertical_scrollbar: show_y,
            scrollbar_thickness: thickness,
            overflow_x,
            overflow_y,
            negative_x,
            negative_y,
        });
        // A viewport box that fills the client describes the outside window
        // after its gutter is restored. A content-sized formatting context
        // keeps its computed size, even when the authored dimension is auto.
        // Gutters remain independent of authored borders and padding.
        let root_style = &doc.node(root).style;
        if root_style.width.is_auto() && fragment.size.width == client.width {
            fragment.size.width = outer_size.width;
        }
        if root_style.height.is_auto() && fragment.size.height == client.height {
            fragment.size.height = outer_size.height;
        }
        fragment.has_overflow_clip = true;
        return fragment;
    }
}
