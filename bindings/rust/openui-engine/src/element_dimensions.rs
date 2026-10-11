//! Native integer dimensions reflected from retained, untransformed layout.
//!
//! Chromium147 Element::clientWidth/clientHeight/scrollWidth/scrollHeight
//! round LayoutUnit values. LayoutBox::ComputeSize and
//! SetScrollableOverflowFromLayoutResults stitch continuations in source
//! coordinates; visual bounds across columns are a different measurement.

use crate::{Engine, EngineError, NodeHandle};
use openui_dom::NodeId;
use openui_geometry::{BoxStrut, LayoutUnit, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_layout::{fragmentation::BreakToken, Fragment, ScrollArea};
use openui_style::{Display, WritingMode};

fn union(a: PhysicalRect, b: PhysicalRect) -> PhysicalRect {
    let left = a.x().min_of(b.x());
    let top = a.y().min_of(b.y());
    let right = a.right().max_of(b.right());
    let bottom = a.bottom().max_of(b.bottom());
    PhysicalRect::from_xywh(left, top, right - left, bottom - top)
}

fn collect_boxes<'a>(fragment: &'a Fragment, node: NodeId, out: &mut Vec<&'a Fragment>) {
    // A non-atomic inline's per-line boxes are LayoutInline geometry. They
    // remain visible to client_rects, but are not CSSOM LayoutBox dimensions.
    if fragment.node_id == node && fragment.owns_element_box() && !fragment.is_inline_box_fragment {
        out.push(fragment);
    }
    for child in &fragment.children {
        collect_boxes(child, node, out);
    }
}

fn block_size(size: PhysicalSize, horizontal: bool) -> LayoutUnit {
    if horizontal {
        size.height
    } else {
        size.width
    }
}

fn source_parts<'a>(boxes: &[&'a Fragment], horizontal: bool) -> Vec<(&'a Fragment, LayoutUnit)> {
    let mut parts = Vec::new();
    let mut consumed = LayoutUnit::zero();
    for &fragment in boxes {
        // Repeated table headers and complete boxes replayed in another
        // fragmentainer do not become additional portions of the source box.
        if !parts.is_empty() && fragment.is_first_for_node {
            break;
        }
        let offset = fragment
            .decoration_slice
            .map_or(consumed, |slice| slice.source_block_offset);
        parts.push((fragment, offset));
        consumed = match fragment.break_token.as_ref() {
            Some(BreakToken::Block(token)) => token.consumed_block_size,
            Some(BreakToken::Inline(token)) => token.consumed_block_size,
            None => offset + block_size(fragment.size, horizontal),
        };
    }
    parts
}

fn stitched_size(parts: &[(&Fragment, LayoutUnit)], mode: WritingMode) -> PhysicalSize {
    let first = parts[0].0.border_box_rect().size;
    if parts.len() == 1 {
        return first;
    }
    let horizontal = mode == WritingMode::HorizontalTb;
    let flipped = matches!(mode, WritingMode::VerticalRl | WritingMode::SidewaysRl);
    let mut end = LayoutUnit::zero();
    for (fragment, consumed) in parts {
        let owned = fragment.border_box_rect();
        let extent = block_size(owned.size, horizontal);
        // A zero-sized owned box can still carry visible child overflow into
        // later columns. That flow must not enlarge its own client dimension.
        if extent <= LayoutUnit::zero() {
            if fragment.is_last_for_node {
                break;
            }
            continue;
        }
        let start = if horizontal {
            owned.y()
        } else if flipped {
            fragment.size.width - owned.right()
        } else {
            owned.x()
        };
        end = end.max_of(*consumed + start + extent);
        if fragment.is_last_for_node {
            break;
        }
    }
    if horizontal {
        PhysicalSize::new(first.width, end)
    } else {
        PhysicalSize::new(end, first.height)
    }
}

fn stitched_overflow(
    parts: &[(&Fragment, LayoutUnit)],
    mode: WritingMode,
    fallback: PhysicalRect,
) -> PhysicalRect {
    let mut combined = fallback;
    // Blink keeps overflow continuations after the owned source box ends.
    // Flipped block flow is first accumulated from right to left, then made
    // relative to the left edge by the sum of every retained fragment width.
    let fragment_width_sum = parts.iter().fold(LayoutUnit::zero(), |sum, (fragment, _)| {
        sum + fragment.size.width
    });
    for (fragment, consumed) in parts {
        let Some(area) = fragment.scroll_area else {
            continue;
        };
        let mut rect = area.content_rect;
        match mode {
            WritingMode::HorizontalTb => rect.offset.top = rect.offset.top + *consumed,
            WritingMode::VerticalRl | WritingMode::SidewaysRl => {
                rect.offset.left =
                    rect.offset.left + fragment_width_sum - *consumed - fragment.size.width;
            }
            WritingMode::VerticalLr | WritingMode::SidewaysLr => {
                rect.offset.left = rect.offset.left + *consumed;
            }
        }
        combined = union(combined, rect);
    }
    combined
}

impl Engine {
    /// Integer layout client width, preserving fractional raw ScrollMetrics.
    pub fn client_width(&mut self, handle: NodeHandle) -> Result<i32, EngineError> {
        Ok(self.element_dimensions(handle)?[0])
    }

    /// Integer layout client height, independent of visual transforms.
    pub fn client_height(&mut self, handle: NodeHandle) -> Result<i32, EngineError> {
        Ok(self.element_dimensions(handle)?[1])
    }

    /// Integer width of the element's retained layout overflow.
    pub fn scroll_width(&mut self, handle: NodeHandle) -> Result<i32, EngineError> {
        Ok(self.element_dimensions(handle)?[2])
    }

    /// Integer height of the element's retained layout overflow.
    pub fn scroll_height(&mut self, handle: NodeHandle) -> Result<i32, EngineError> {
        Ok(self.element_dimensions(handle)?[3])
    }

    fn element_dimensions(&mut self, handle: NodeHandle) -> Result<[i32; 4], EngineError> {
        let node = self.resolve(handle)?;
        if !self.node_is_connected(node) {
            return Ok([0; 4]);
        }
        self.update()?;
        // Native documents expose a viewport root. Browser-shaped snapshots
        // expose their standards document element through the same viewport.
        let viewport_element =
            node == self.document.root() || Some(node) == self.document.document_element();
        let metric_node = if viewport_element {
            self.document.root()
        } else {
            node
        };
        let Some(root) = self.latest_fragment.as_ref() else {
            return Ok([0; 4]);
        };
        let mut boxes = Vec::new();
        collect_boxes(root, metric_node, &mut boxes);
        let Some(&first) = boxes.first() else {
            return Ok([0; 4]);
        };
        if viewport_element {
            let Some(area) = first.scroll_area else {
                return Ok([0; 4]);
            };
            return Ok([
                area.client_rect.width().round_i32(),
                area.client_rect.height().round_i32(),
                area.content_rect.width().round_i32(),
                area.content_rect.height().round_i32(),
            ]);
        }
        let style = &self.document.node(node).style;
        let mode = style.writing_mode;
        let parts = source_parts(&boxes, mode == WritingMode::HorizontalTb);
        let size = stitched_size(&parts, mode);
        let mut border = ScrollArea::used_border(first);
        // Native continuation flags can suppress the last source edge in an
        // early fragment. The stitched box retains each physical used edge.
        for (fragment, _) in &parts {
            let next = ScrollArea::used_border(fragment);
            border.top = border.top.max_of(next.top);
            border.right = border.right.max_of(next.right);
            border.bottom = border.bottom.max_of(next.bottom);
            border.left = border.left.max_of(next.left);
        }
        let gutters = first
            .element_scrollbars
            .map_or(BoxStrut::zero(), |bars| bars.insets);
        let client = PhysicalSize::new(
            (size.width - border.inline_sum() - gutters.inline_sum()).clamp_negative_to_zero(),
            (size.height - border.block_sum() - gutters.block_sum()).clamp_negative_to_zero(),
        );
        let origin = first.border_box_rect().offset;
        let padding_rect = PhysicalRect::new(
            PhysicalOffset::new(
                origin.left + border.left + gutters.left,
                origin.top + border.top + gutters.top,
            ),
            client,
        );
        let content = stitched_overflow(&parts, mode, padding_rect);
        let scroll = if style.is_scroll_container() {
            PhysicalSize::new(
                content.width().max_of(client.width),
                content.height().max_of(client.height),
            )
        } else {
            // LayoutBox::ScrollWidth accounts for reversed physical X. Its
            // non-container ScrollHeight deliberately uses physical bottom.
            let reversed = style.direction.writing_direction(mode);
            let flipped_x = if reversed.is_horizontal() {
                reversed.is_rtl()
            } else {
                reversed.is_flipped_blocks()
            };
            let width = if flipped_x {
                client.width - (content.x() - origin.left - border.left).min_of(LayoutUnit::zero())
            } else {
                client
                    .width
                    .max_of(content.right() - origin.left - border.left)
            };
            PhysicalSize::new(
                width,
                client
                    .height
                    .max_of(content.bottom() - origin.top - border.top),
            )
        };
        let table = matches!(style.display, Display::Table | Display::InlineTable);
        Ok([
            (client.width
                + if table {
                    border.inline_sum()
                } else {
                    LayoutUnit::zero()
                })
            .round_i32(),
            (client.height
                + if table {
                    border.block_sum()
                } else {
                    LayoutUnit::zero()
                })
            .round_i32(),
            scroll.width.clamp_negative_to_zero().round_i32(),
            scroll.height.clamp_negative_to_zero().round_i32(),
        ])
    }
}
