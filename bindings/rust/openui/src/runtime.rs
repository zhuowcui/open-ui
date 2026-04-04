//! Core reactive runtime — thread-local generational arena for signals, effects, and scopes.
//!
//! All public mutation flows through standalone functions that borrow/release
//! the `RefCell<Runtime>` in small, non-overlapping windows so that effect
//! closures can re-borrow the runtime for signal reads and writes.

use std::any::Any;
use std::cell::RefCell;

/// A generational index into the signal arena. `Copy` for ergonomic use.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct SignalId {
    pub(crate) index: u32,
    pub(crate) generation: u32,
}

/// Internal storage slot for a single signal.
pub(crate) struct SignalSlot {
    pub(crate) value: Box<dyn Any>,
    pub(crate) generation: u32,
    pub(crate) subscribers: Vec<EffectId>,
}

/// A handle to an effect in the runtime.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct EffectId(pub(crate) u32);

/// Internal storage slot for a single effect.
#[allow(dead_code)] // `scope` retained for future introspection
pub(crate) struct EffectSlot {
    pub(crate) f: Box<dyn Fn()>,
    pub(crate) dependencies: Vec<SignalId>,
    pub(crate) scope: Option<ScopeId>,
    pub(crate) active: bool,
}

/// A handle to a reactive scope.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ScopeId(pub(crate) u32);

/// Internal storage slot for a scope.
#[allow(dead_code)] // `parent` and `active` retained for future introspection
pub(crate) struct ScopeSlot {
    pub(crate) effects: Vec<EffectId>,
    pub(crate) children: Vec<ScopeId>,
    pub(crate) parent: Option<ScopeId>,
    pub(crate) cleanups: Vec<Box<dyn FnOnce()>>,
    pub(crate) active: bool,
}

/// The thread-local reactive runtime.
pub struct Runtime {
    pub(crate) signals: Vec<Option<SignalSlot>>,
    pub(crate) free_signals: Vec<u32>,
    pub(crate) effects: Vec<Option<EffectSlot>>,
    pub(crate) free_effects: Vec<u32>,
    pub(crate) scopes: Vec<Option<ScopeSlot>>,
    pub(crate) free_scopes: Vec<u32>,
    pub(crate) current_effect: Option<EffectId>,
    pub(crate) current_scope: Option<ScopeId>,
    pub(crate) batch_depth: u32,
    pub(crate) pending_effects: Vec<EffectId>,
}

thread_local! {
    pub(crate) static RUNTIME: RefCell<Runtime> = RefCell::new(Runtime::new());
}

impl Runtime {
    /// Create a fresh, empty runtime.
    pub fn new() -> Self {
        Self {
            signals: Vec::new(),
            free_signals: Vec::new(),
            effects: Vec::new(),
            free_effects: Vec::new(),
            scopes: Vec::new(),
            free_scopes: Vec::new(),
            current_effect: None,
            current_scope: None,
            batch_depth: 0,
            pending_effects: Vec::new(),
        }
    }

    // ── Signals ──────────────────────────────────────────────────────

    /// Allocate a new signal slot, return its generational id.
    pub(crate) fn create_signal_raw(&mut self, value: Box<dyn Any>) -> SignalId {
        if let Some(index) = self.free_signals.pop() {
            let idx = index as usize;
            let gen = match &self.signals[idx] {
                Some(slot) => slot.generation + 1,
                None => 1,
            };
            self.signals[idx] = Some(SignalSlot {
                value,
                generation: gen,
                subscribers: Vec::new(),
            });
            SignalId { index, generation: gen }
        } else {
            let index = self.signals.len() as u32;
            self.signals.push(Some(SignalSlot {
                value,
                generation: 0,
                subscribers: Vec::new(),
            }));
            SignalId { index, generation: 0 }
        }
    }

    /// Track the read (if inside an effect) and clone the typed value.
    pub(crate) fn get_signal_value_cloned<T: Clone + 'static>(&mut self, id: SignalId) -> Option<T> {
        let slot = self.signals.get(id.index as usize)?.as_ref()?;
        if slot.generation != id.generation {
            return None;
        }

        // Dependency tracking
        if let Some(eff_id) = self.current_effect {
            let slot = self.signals[id.index as usize].as_mut().unwrap();
            if !slot.subscribers.contains(&eff_id) {
                slot.subscribers.push(eff_id);
            }
            if let Some(eff_slot) = self.effects[eff_id.0 as usize].as_mut() {
                if !eff_slot.dependencies.contains(&id) {
                    eff_slot.dependencies.push(id);
                }
            }
        }

        let slot = self.signals[id.index as usize].as_ref().unwrap();
        Some(slot.value.downcast_ref::<T>().expect("signal type mismatch").clone())
    }

    /// Overwrite value. Returns the list of subscriber effect ids to schedule.
    pub(crate) fn set_signal_value(&mut self, id: SignalId, value: Box<dyn Any>) -> Vec<EffectId> {
        let slot = match self.signals.get_mut(id.index as usize) {
            Some(Some(s)) if s.generation == id.generation => s,
            _ => return Vec::new(),
        };
        slot.value = value;
        slot.subscribers.clone()
    }

    /// Enqueue effects without flushing (flushing is done outside the borrow).
    pub(crate) fn enqueue_effects(&mut self, ids: Vec<EffectId>) {
        for eid in ids {
            if !self.pending_effects.contains(&eid) {
                self.pending_effects.push(eid);
            }
        }
    }

    // ── Scopes ───────────────────────────────────────────────────────

    /// Create a new scope, optionally nested under the current scope.
    pub(crate) fn create_scope_raw(&mut self) -> ScopeId {
        let parent = self.current_scope;
        let id = if let Some(index) = self.free_scopes.pop() {
            self.scopes[index as usize] = Some(ScopeSlot {
                effects: Vec::new(),
                children: Vec::new(),
                parent,
                cleanups: Vec::new(),
                active: true,
            });
            ScopeId(index)
        } else {
            let index = self.scopes.len() as u32;
            self.scopes.push(Some(ScopeSlot {
                effects: Vec::new(),
                children: Vec::new(),
                parent,
                cleanups: Vec::new(),
                active: true,
            }));
            ScopeId(index)
        };

        if let Some(parent_id) = parent {
            if let Some(Some(parent_slot)) = self.scopes.get_mut(parent_id.0 as usize) {
                parent_slot.children.push(id);
            }
        }
        id
    }

    /// Dispose a scope: deactivate effects, run cleanups, recurse into children.
    pub(crate) fn dispose_scope_raw(&mut self, id: ScopeId) {
        let children: Vec<ScopeId> = self
            .scopes.get(id.0 as usize)
            .and_then(|s| s.as_ref())
            .map(|s| s.children.clone())
            .unwrap_or_default();

        for child in children {
            self.dispose_scope_raw(child);
        }

        let effect_ids: Vec<EffectId> = self
            .scopes.get(id.0 as usize)
            .and_then(|s| s.as_ref())
            .map(|s| s.effects.clone())
            .unwrap_or_default();

        for eid in &effect_ids {
            if let Some(Some(eff)) = self.effects.get_mut(eid.0 as usize) {
                eff.active = false;
                let deps = std::mem::take(&mut eff.dependencies);
                for sig_id in deps {
                    if let Some(Some(sig)) = self.signals.get_mut(sig_id.index as usize) {
                        if sig.generation == sig_id.generation {
                            sig.subscribers.retain(|e| e != eid);
                        }
                    }
                }
            }
            if let Some(slot) = self.effects.get_mut(eid.0 as usize) {
                *slot = None;
                self.free_effects.push(eid.0);
            }
        }

        self.pending_effects.retain(|e| !effect_ids.contains(e));

        let cleanups: Vec<Box<dyn FnOnce()>> = self
            .scopes.get_mut(id.0 as usize)
            .and_then(|s| s.as_mut())
            .map(|s| std::mem::take(&mut s.cleanups))
            .unwrap_or_default();
        for cleanup in cleanups {
            cleanup();
        }

        if let Some(slot) = self.scopes.get_mut(id.0 as usize) {
            *slot = None;
            self.free_scopes.push(id.0);
        }
    }
}

// ─── Standalone helpers (borrow-safe) ────────────────────────────────

/// Flush all pending effects. Each effect is run outside any `RefCell` borrow
/// so its closure can freely read/write signals.
pub(crate) fn flush_pending() {
    loop {
        let next = RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            if rt.batch_depth > 0 {
                return None; // still inside a batch
            }
            if rt.pending_effects.is_empty() {
                return None;
            }
            // Take the first pending effect
            Some(rt.pending_effects.remove(0))
        });

        match next {
            Some(eid) => run_effect_standalone(eid),
            None => break,
        }
    }
}

/// Run a single effect outside any `&mut Runtime` borrow.
pub(crate) fn run_effect_standalone(id: EffectId) {
    // 1. Check active & clear old deps
    let should_run = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let active = rt.effects.get(id.0 as usize)
            .and_then(|s| s.as_ref())
            .map(|s| s.active)
            .unwrap_or(false);
        if !active {
            return false;
        }

        // Clear old dependencies
        let old_deps: Vec<SignalId> = rt.effects[id.0 as usize]
            .as_ref()
            .map(|e| e.dependencies.clone())
            .unwrap_or_default();

        for sig_id in &old_deps {
            if let Some(Some(slot)) = rt.signals.get_mut(sig_id.index as usize) {
                if slot.generation == sig_id.generation {
                    slot.subscribers.retain(|e| *e != id);
                }
            }
        }
        if let Some(Some(eff)) = rt.effects.get_mut(id.0 as usize) {
            eff.dependencies.clear();
        }
        true
    });

    if !should_run {
        return;
    }

    // 2. Set current_effect, save previous
    let prev_effect = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let prev = rt.current_effect;
        rt.current_effect = Some(id);
        prev
    });

    // 3. Extract function pointer and call it.
    let f_ptr: *const dyn Fn() = RUNTIME.with(|rt| {
        let rt = rt.borrow();
        let eff = rt.effects[id.0 as usize].as_ref().unwrap();
        &*eff.f as *const dyn Fn()
    });

    // SAFETY: Single-threaded (thread-local), function object lives in the
    // arena and is not freed during execution, RefCell borrow is released.
    unsafe { (*f_ptr)(); }

    // 4. Restore previous effect
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.current_effect = prev_effect;
    });
}

/// Reset the entire runtime (useful for test isolation).
#[cfg(test)]
pub(crate) fn reset_runtime() {
    RUNTIME.with(|rt| {
        *rt.borrow_mut() = Runtime::new();
    });
}
