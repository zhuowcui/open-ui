//! Retained overflow dependencies of composite-only transform updates.
use crate::{Engine, InvalidationClass};
use openui_dom::NodeId;
use openui_layout::Fragment;
use std::collections::HashSet;
use std::sync::Arc;

impl Engine {
    pub(crate) fn refresh_retained_scroll_overflow(&mut self) {
        let Some(fragment) = self.latest_fragment.as_mut() else {
            return;
        };
        let update =
            openui_layout::refresh_fragment_overflow(&self.document, Arc::make_mut(fragment));
        if update.requires_layout {
            self.mark_dirty(InvalidationClass::Layout);
            return;
        }
        if !update.transforms_changed && !update.ranges_changed {
            return;
        }
        let fragment = self
            .latest_fragment
            .as_ref()
            .expect("retained fragment")
            .clone();
        fn owned_offsets(
            fragment: &Fragment,
            seen: &mut HashSet<NodeId>,
            out: &mut Vec<(NodeId, openui_layout::ScrollArea)>,
        ) {
            if fragment.owns_scroll_box() && seen.insert(fragment.node_id) {
                if let Some(area) = fragment.scroll_area {
                    out.push((fragment.node_id, area));
                }
            }
            for child in &fragment.children {
                owned_offsets(child, seen, out);
            }
        }
        let mut areas = Vec::new();
        owned_offsets(&fragment, &mut HashSet::new(), &mut areas);
        for (node, area) in &areas {
            let data = self.document.node(*node);
            let next = area.clamp_offset(data.scroll_left as f64, data.scroll_top as f64);
            if next != (data.scroll_left, data.scroll_top) {
                let data = self.document.node_mut(*node);
                data.scroll_left = next.0;
                data.scroll_top = next.1;
                self.invalidate_scroll(*node);
            }
        }
        if self.dirty.layout {
            // A sticky owner must first rebuild its physical descendants from
            // the clamped offset. Layout will settle snap targets afterward.
            return;
        }
        // A changed transform can move a snap area without changing the range.
        // Reuse the shared retained target selection, including partial axes.
        let before: Vec<_> = areas
            .iter()
            .map(|(node, _)| {
                let data = self.document.node(*node);
                (*node, data.scroll_left, data.scroll_top)
            })
            .collect();
        if self.settle_native_snap_after_layout(&fragment) {
            for (node, x, y) in before {
                let data = self.document.node(node);
                if (x, y) != (data.scroll_left, data.scroll_top) {
                    self.invalidate_scroll(node);
                }
            }
        }
    }
}
