//! Layout-owned principal-box size across fragmentainers.
//!
//! Chromium147 LayoutBox::ComputeSize uses the first inline size and each
//! continuation's preceding consumed block size. Repeated and overflow-only
//! continuations stop at the generating box's block end; decoration policy
//! does not supply this geometry.

use crate::{Fragment, FragmentBoxGeometry};
use openui_dom::NodeId;
use openui_geometry::{
    BoxStrut, LayoutUnit, LogicalBoxStrut, LogicalSize, PhysicalSize, WritingDirectionMode,
    WritingModeConverter,
};
use std::collections::HashMap;

/// Retain the completed source before a multicol owner changes its size or
/// projects descendants into continuation-local coordinates. The source tree
/// here is in the fragmenting context's logical `(inline, block)` storage.
pub(crate) fn capture_source(fragment: &mut Fragment, direction: WritingDirectionMode) {
    if fragment.owns_layout_box()
        && !fragment.is_inline_box_fragment
        && fragment.box_geometry.is_none()
    {
        let rect = fragment.border_box_rect();
        let source_size = WritingModeConverter::new(direction, PhysicalSize::zero())
            .to_physical_size(LogicalSize::new(rect.width(), rect.height()));
        let physical_edges = |edges: BoxStrut| {
            LogicalBoxStrut::new(edges.left, edges.right, edges.top, edges.bottom)
                .to_physical(direction)
        };
        // Collapsed borders are already retained in their real physical table
        // writing direction; ordinary block/multicol working edges are logical.
        let source_border = if fragment.collapsed_table_border_data.is_some() {
            crate::ScrollArea::used_border(fragment)
        } else {
            physical_edges(crate::ScrollArea::used_border(fragment))
        };
        let source_padding = physical_edges(crate::ScrollArea::used_padding(fragment));
        fragment.box_geometry = Some(FragmentBoxGeometry {
            source_size,
            source_border,
            source_padding,
            stitched_size: source_size,
            writing_direction: direction,
            inline_size: rect.width(),
            block_size: rect.height(),
            consumed_block_size: LayoutUnit::zero(),
            has_next: false,
            is_repeated: false,
            continuation_record: false,
        });
    }
    for child in &mut fragment.children {
        // Promoted/positioned descendants retain physical geometry and are
        // fragmented by their separate layout owner.
        if child.positioned_fragmentation.is_none() {
            capture_source(child, direction);
        }
    }
}

/// Record the real principal-box part without consulting paint-side first/last
/// flags. Clone decoration can retain both borders on every fragment.
pub(crate) fn record_piece(
    fragment: &mut Fragment,
    direction: WritingDirectionMode,
    consumed: LayoutUnit,
    block_size: LayoutUnit,
    has_next: bool,
) {
    if !fragment.owns_layout_box() || fragment.is_inline_box_fragment {
        return;
    }
    capture_source(fragment, direction);
    let rect = fragment.border_box_rect();
    if let Some(geometry) = fragment.box_geometry.as_mut() {
        geometry.writing_direction = direction;
        geometry.inline_size = rect.width();
        geometry.block_size = block_size;
        geometry.consumed_block_size = consumed;
        geometry.has_next = has_next;
        geometry.continuation_record = true;
    }
}

/// A descendant materialized from an already-owned continuation advances in
/// that node's retained source coordinates, rather than the parent's paint
/// translation or a decoration slice's coordinate system.
pub(crate) fn record_subslice(fragment: &mut Fragment, consumed: LayoutUnit, has_next: bool) {
    if let Some(previous) = fragment.box_geometry {
        let before = if previous.continuation_record {
            previous.consumed_block_size
        } else {
            LayoutUnit::zero()
        };
        let size = fragment.border_box_rect().height();
        record_piece(
            fragment,
            previous.writing_direction,
            before + consumed,
            size,
            has_next,
        );
    }
}

/// Finalize once after physical placement. This changes retained metadata only;
/// it does not resize, translate, clip, decorate or reorder a fragment.
pub(crate) fn populate(root: &mut Fragment) {
    fn collect(fragment: &Fragment, boxes: &mut HashMap<(NodeId, bool), Vec<FragmentBoxGeometry>>) {
        if fragment.owns_layout_box() && !fragment.is_inline_box_fragment {
            if let Some(geometry) = fragment.box_geometry {
                boxes
                    .entry((fragment.node_id, fragment.anonymous_layout_box.is_some()))
                    .or_default()
                    .push(geometry);
            }
        }
        for child in &fragment.children {
            collect(child, boxes);
        }
    }
    let mut boxes = HashMap::new();
    collect(root, &mut boxes);
    let sizes: HashMap<_, _> = boxes
        .into_iter()
        .map(|(node, mut records)| {
            if records.iter().any(|record| record.continuation_record) {
                records.retain(|record| record.continuation_record);
            }
            records.sort_by_key(|record| record.consumed_block_size.raw());
            let first = records[0];
            let mut block = first.block_size;
            for record in &records {
                block = record.consumed_block_size + record.block_size;
                if !record.has_next || record.is_repeated {
                    break;
                }
            }
            let size = WritingModeConverter::new(first.writing_direction, PhysicalSize::zero())
                .to_physical_size(LogicalSize::new(first.inline_size, block));
            (node, size)
        })
        .collect();
    fn publish(fragment: &mut Fragment, sizes: &HashMap<(NodeId, bool), PhysicalSize>) {
        if fragment.owns_layout_box() {
            if let Some(geometry) = fragment.box_geometry.as_mut() {
                if let Some(size) =
                    sizes.get(&(fragment.node_id, fragment.anonymous_layout_box.is_some()))
                {
                    geometry.stitched_size = *size;
                }
            }
        }
        for child in &mut fragment.children {
            publish(child, sizes);
        }
    }
    publish(root, &sizes);
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_dom::{Document, ElementTag};
    #[test]
    fn unsliced_edges_keep_their_source_writing_direction_after_decoration_removal() {
        let mut doc = Document::new();
        let node = doc.create_node(ElementTag::Div);
        let lu = LayoutUnit::from_i32;
        let mut fragment = Fragment::new_box(node, PhysicalSize::new(lu(100), lu(200)));
        fragment.border = BoxStrut::new(lu(2), lu(7), lu(4), lu(3));
        fragment.padding = BoxStrut::new(lu(6), lu(9), lu(8), lu(5));
        capture_source(
            &mut fragment,
            WritingDirectionMode::new(false, false, false, false),
        );
        fragment.border.bottom = lu(0);
        fragment.padding.bottom = lu(0);
        let data = fragment.box_geometry.unwrap();
        assert_eq!(data.source_size, PhysicalSize::new(lu(200), lu(100)));
        assert_eq!(
            data.source_border,
            BoxStrut::new(lu(3), lu(4), lu(7), lu(2))
        );
        assert_eq!(
            data.source_padding,
            BoxStrut::new(lu(5), lu(8), lu(9), lu(6))
        );
    }
}
