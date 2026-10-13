//! Owning-thread native selection and scroll notification phases.
use crate::{Document, Error, Event, Modifiers};
use openui_engine::NodeHandle;
use std::cell::{Cell, RefCell};
use std::collections::{HashSet, VecDeque};

#[derive(Default)]
pub(crate) struct SelectionTasks {
    pending: RefCell<Pending>,
    dispatching: Cell<bool>,
    active_select_remaining: Cell<usize>,
}

#[derive(Default)]
struct Pending {
    changes: VecDeque<NodeHandle>,
    change_targets: HashSet<NodeHandle>,
    selects: VecDeque<NodeHandle>,
    scrolls: VecDeque<NodeHandle>,
    scroll_targets: HashSet<NodeHandle>,
    active_scrolls: VecDeque<NodeHandle>,
}

struct DispatchGuard<'a>(&'a Cell<bool>);
impl Drop for DispatchGuard<'_> {
    fn drop(&mut self) {
        self.0.set(false);
    }
}

impl SelectionTasks {
    pub(crate) fn schedule(
        &self,
        target: NodeHandle,
        value_changed: bool,
        selection_changed: bool,
        request_select: bool,
    ) -> Result<(), Error> {
        if !value_changed && !selection_changed {
            return Ok(());
        }
        let mut pending = self
            .pending
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?;
        if pending.change_targets.insert(target) {
            pending.changes.push_back(target);
        }
        if request_select && selection_changed {
            pending.selects.push_back(target);
        }
        Ok(())
    }

    pub(crate) fn is_pending(&self) -> Result<bool, Error> {
        let pending = self
            .pending
            .try_borrow()
            .map_err(|_| Error::ReentrantMutation)?;
        Ok(!pending.changes.is_empty()
            || !pending.selects.is_empty()
            || !pending.scrolls.is_empty()
            || !pending.active_scrolls.is_empty())
    }

    pub(crate) fn dispatch(&self, document: &Document) -> Result<(), Error> {
        if self.dispatching.replace(true) {
            return Ok(());
        }
        let _guard = DispatchGuard(&self.dispatching);
        // Reject an outstanding engine borrow before consuming any tasks.
        let scrolls = document.with_engine_mut(|engine| Ok(engine.drain_scroll_events()))?;
        self.schedule_scroll(scrolls)?;
        if !self
            .pending
            .try_borrow()
            .map_err(|_| Error::ReentrantMutation)?
            .active_scrolls
            .is_empty()
        {
            return self.dispatch_scroll_batch(document);
        }
        // Resume an interrupted frame batch before newly queued change tasks.
        // Decrement before callbacks: a partially delivered notification is
        // never duplicated if an application callback unwinds.
        if self.active_select_remaining.get() == 0 {
            let initial_changes = self
                .pending
                .try_borrow()
                .map_err(|_| Error::ReentrantMutation)?
                .changes
                .len();
            for _ in 0..initial_changes {
                let target = {
                    let mut pending = self
                        .pending
                        .try_borrow_mut()
                        .map_err(|_| Error::ReentrantMutation)?;
                    let Some(target) = pending.changes.pop_front() else {
                        break;
                    };
                    // Coalescing lasts until this target's actual delivery.
                    pending.change_targets.remove(&target);
                    target
                };
                if document.target_is_live(target)? {
                    document.dispatch_to(
                        target,
                        &Event::keyboard("selectionchange", 0, None, Modifiers::NONE),
                    )?;
                }
            }
            let pending = self
                .pending
                .try_borrow()
                .map_err(|_| Error::ReentrantMutation)?;
            // Callback changes get a later native task turn, before select.
            // One dispatch is bounded even when a listener keeps mutating.
            if !pending.changes.is_empty() {
                drop(pending);
                return self.capture_and_dispatch_scroll_batch(document);
            }
            self.active_select_remaining.set(pending.selects.len());
        }
        while self.active_select_remaining.get() > 0 {
            let target = self
                .pending
                .try_borrow_mut()
                .map_err(|_| Error::ReentrantMutation)?
                .selects
                .pop_front();
            let Some(target) = target else {
                self.active_select_remaining.set(0);
                break;
            };
            self.active_select_remaining
                .set(self.active_select_remaining.get() - 1);
            if document.target_is_live(target)? {
                document
                    .dispatch_to(target, &Event::keyboard("select", 0, None, Modifiers::NONE))?;
            }
            // Selection changes made by these callbacks wait for the entire
            // captured frame batch. New select tasks remain in the next batch.
        }
        self.capture_and_dispatch_scroll_batch(document)
    }

    fn schedule_scroll(&self, targets: Vec<NodeHandle>) -> Result<(), Error> {
        if targets.is_empty() {
            return Ok(());
        }
        let mut pending = self
            .pending
            .try_borrow_mut()
            .map_err(|_| Error::ReentrantMutation)?;
        for target in targets {
            if pending.scroll_targets.insert(target) {
                pending.scrolls.push_back(target);
            }
        }
        Ok(())
    }

    fn capture_and_dispatch_scroll_batch(&self, document: &Document) -> Result<(), Error> {
        {
            let mut pending = self
                .pending
                .try_borrow_mut()
                .map_err(|_| Error::ReentrantMutation)?;
            debug_assert!(pending.active_scrolls.is_empty());
            pending.active_scrolls = std::mem::take(&mut pending.scrolls);
            // Pinned .50 clears the per-frame coalescing map at capture. A
            // callback may queue the same target in a distinct later batch,
            // including one that is still undelivered in this captured batch.
            pending.scroll_targets.clear();
        }
        self.dispatch_scroll_batch(document)
    }

    fn dispatch_scroll_batch(&self, document: &Document) -> Result<(), Error> {
        loop {
            let target = self
                .pending
                .try_borrow_mut()
                .map_err(|_| Error::ReentrantMutation)?
                .active_scrolls
                .pop_front();
            let Some(target) = target else {
                break;
            };
            // Pop before callbacks so unwinding cannot duplicate delivery.
            // Newly queued targets remain separate from this captured batch.
            if document.target_is_live(target)? {
                // Pinned .50 bubbles document scroll only. Element scroll is
                // nonbubbling and noncancelable; capture still follows its route.
                let root = document.with_engine(|engine| engine.root() == target)?;
                document.dispatch_to(target, &Event::new("scroll", root, false))?;
            }
        }
        Ok(())
    }
}
