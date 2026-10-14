//! Owned scroll notifications shared by all native offset producers.
use crate::{Engine, NodeHandle};
use openui_dom::NodeId;
use std::collections::{HashSet, VecDeque};

#[derive(Default)]
pub(crate) struct ScrollNotifications {
    pending: VecDeque<NodeHandle>,
    targets: HashSet<NodeHandle>,
}

impl ScrollNotifications {
    fn enqueue(&mut self, target: NodeHandle) {
        if self.targets.insert(target) {
            self.pending.push_back(target);
        }
    }

    pub(crate) fn retire(&mut self, removed: &[NodeHandle]) {
        self.pending.retain(|target| !removed.contains(target));
        self.targets.retain(|target| !removed.contains(target));
    }
}

impl Engine {
    pub(crate) fn enqueue_scroll_notification(&mut self, node: NodeId) {
        if let Some(index) = self.node_slots.get(&node).copied() {
            let target = self.handle_for_slot(index);
            self.scroll_notifications.enqueue(target);
        }
    }

    /// Whether native offset changes await an owning-thread event turn.
    /// This query does no layout or paint and invokes no application callback.
    pub fn has_pending_scroll_events(&self) -> bool {
        !self.scroll_notifications.pending.is_empty()
    }

    /// Transfer ordered, coalesced scroll targets to the native event owner.
    /// Handles retain their document and generation identity. This operation
    /// invokes no callbacks; consumers deliver only to live targets after the
    /// Engine borrow has ended. A later offset change can schedule a new turn.
    pub fn drain_scroll_events(&mut self) -> Vec<NodeHandle> {
        self.scroll_notifications.targets.clear();
        self.scroll_notifications.pending.drain(..).collect()
    }
}
