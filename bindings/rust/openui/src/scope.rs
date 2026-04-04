//! Reactive scopes — group effects for batch disposal (e.g., component unmount).

use crate::runtime::{ScopeId, RUNTIME};

/// Create a new reactive scope. Effects created while `f` executes are
/// automatically associated with this scope and will be disposed when the
/// scope is disposed.
///
/// Returns the [`ScopeId`] so the caller can dispose it later.
pub fn create_scope(f: impl FnOnce()) -> ScopeId {
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let id = rt.create_scope_raw();
        id
    });

    // Set as current scope, run f, restore previous scope
    let prev_scope = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let prev = rt.current_scope;
        rt.current_scope = Some(id);
        prev
    });

    f();

    RUNTIME.with(|rt| {
        rt.borrow_mut().current_scope = prev_scope;
    });

    id
}

/// Dispose a scope and all its effects, child scopes, and cleanups.
pub fn dispose_scope(id: ScopeId) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().dispose_scope_raw(id);
    });
}

/// Register a cleanup function in the current scope. The function will be
/// called when the scope is disposed.
///
/// If there is no current scope, the closure is intentionally leaked to keep
/// its captures (e.g., `TextNode` handles) alive for the application lifetime.
pub fn on_cleanup(f: impl FnOnce() + 'static) {
    let boxed: Box<dyn FnOnce()> = Box::new(f);
    RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        if let Some(scope_id) = rt.current_scope {
            if let Some(Some(scope_slot)) = rt.scopes.get_mut(scope_id.0 as usize) {
                scope_slot.cleanups.push(boxed);
                return;
            }
        }
        // No active scope — leak the closure so its captures (e.g., TextNode
        // handles) stay alive for the application lifetime. DOM cleanup
        // relies on oui_element_remove_all_child_nodes().
        std::mem::forget(boxed);
    });
}
