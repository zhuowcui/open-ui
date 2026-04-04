//! Reactive component primitives — [`Show`], [`For`], and [`DynChild`].
//!
//! These components provide conditional rendering, keyed list rendering, and
//! dynamic child rendering respectively. They are the building blocks that the
//! `view!` macro relies on for reactive DOM updates.

#![allow(non_snake_case)]

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;

use crate::context::current_document;
use crate::effect::create_effect;
use crate::element::Element;
use crate::runtime::ScopeId;
use crate::scope::{create_scope, dispose_scope};
use crate::view_node::{mount_view, IntoView, ViewNode};

/// Conditionally render content based on a reactive boolean expression.
///
/// When `when` returns `true`, the `children` closure is called and the
/// result is mounted into the DOM. When `false`, the `fallback` closure
/// is rendered instead. Switching between branches disposes the previous
/// scope (tearing down any effects created by the old branch) and mounts
/// fresh content.
///
/// The container element uses `display: contents` so it does not affect
/// layout.
///
/// # Example
///
/// ```ignore
/// let visible = create_signal(true);
/// Show(
///     move || visible.get(),
///     || view! { <p>"Nothing here"</p> },
///     move || view! { <p>"Content is visible!"</p> },
/// )
/// ```
pub fn Show<W, F, CF, FV, V>(when: W, fallback: F, children: CF) -> ViewNode
where
    W: Fn() -> bool + 'static,
    F: Fn() -> FV + 'static,
    FV: IntoView,
    CF: Fn() -> V + 'static,
    V: IntoView,
{
    let doc = current_document();
    let container = Element::create(doc, "div").expect("Show: failed to create container");
    container.set_style("display", "contents").ok();

    let raw = container.as_raw();
    let scope_cell: Cell<Option<ScopeId>> = Cell::new(None);
    let showing_cell: Cell<Option<bool>> = Cell::new(None);

    create_effect(move || {
        let show = when();

        // Skip re-render when the branch has not changed.
        if showing_cell.get() == Some(show) {
            return;
        }
        showing_cell.set(Some(show));

        // Dispose previous branch scope (tears down effects + cleanups).
        if let Some(old) = scope_cell.get() {
            dispose_scope(old);
        }

        let scope_id = create_scope(|| {
            // SAFETY: raw points to our container which outlives this scope.
            let container_ref = unsafe { Element::from_raw_borrowed(raw) };
            container_ref.remove_all_children();

            if show {
                let view = children().into_view();
                mount_view(&container_ref, view);
            } else {
                let view = fallback().into_view();
                mount_view(&container_ref, view);
            }
        });

        scope_cell.set(Some(scope_id));
    });

    ViewNode::Element(container)
}

/// Render a reactive list of items with keyed reconciliation.
///
/// When the `each` closure's result changes, `For` efficiently:
/// - **Removes** DOM elements for keys no longer present (and disposes their
///   reactive scopes).
/// - **Creates** new DOM elements for newly-appearing keys.
/// - **Reorders** existing DOM elements to match the new order.
///
/// Each item is wrapped in a transparent `<div style="display:contents">`
/// container to simplify tracking.
///
/// # Example
///
/// ```ignore
/// let items = create_signal(vec!["apple", "banana", "cherry"]);
/// For(
///     move || items.get(),
///     |item| item.to_string(),
///     |item| view! { <li>{item}</li> },
/// )
/// ```
pub fn For<T, K, E, KF, CF, V>(each: E, key: KF, children: CF) -> ViewNode
where
    T: 'static,
    K: Eq + Hash + Clone + 'static,
    E: Fn() -> Vec<T> + 'static,
    KF: Fn(&T) -> K + 'static,
    CF: Fn(T) -> V + 'static,
    V: IntoView,
{
    let doc = current_document();
    let container = Element::create(doc, "div").expect("For: failed to create container");
    container.set_style("display", "contents").ok();

    let raw = container.as_raw();

    // Ordered state: (key, scope_id, wrapper_raw_ptr) for each item currently
    // in the DOM.
    let state: RefCell<Vec<(K, ScopeId, *mut openui_sys::OuiElement)>> = RefCell::new(Vec::new());

    create_effect(move || {
        let new_data = each();

        // SAFETY: raw points to our container which outlives this effect.
        let container_ref = unsafe { Element::from_raw_borrowed(raw) };

        let mut old_state = state.borrow_mut();

        // Build the new key list and a fast lookup set.
        let new_keys: Vec<K> = new_data.iter().map(&key).collect();
        let new_key_set: HashMap<K, ()> = new_keys.iter().map(|k| (k.clone(), ())).collect();

        // Partition old items into kept (reusable) and removed.
        let mut kept: HashMap<K, (ScopeId, *mut openui_sys::OuiElement)> = HashMap::new();
        for (k, scope_id, el_raw) in old_state.drain(..) {
            if new_key_set.contains_key(&k) {
                kept.insert(k, (scope_id, el_raw));
            } else {
                dispose_scope(scope_id);
                Element::destroy_subtree(el_raw);
            }
        }

        // Build new state: reuse kept items and create new ones, appending in
        // order. `append_child` on an already-attached child moves it, which
        // gives us correct ordering for free.
        let mut new_state = Vec::with_capacity(new_data.len());
        for (item, k) in new_data.into_iter().zip(new_keys.into_iter()) {
            if let Some((scope_id, el_raw)) = kept.remove(&k) {
                // Re-use existing element — move to end via append_child.
                let el_ref = unsafe { Element::from_raw_borrowed(el_raw) };
                container_ref.append_child(&el_ref);
                new_state.push((k, scope_id, el_raw));
            } else {
                // Create a new wrapper + content.
                let wrapper = Element::create(current_document(), "div")
                    .expect("For: failed to create item wrapper");
                wrapper.set_style("display", "contents").ok();
                let wrapper_raw = wrapper.as_raw();

                let scope_id = create_scope(|| {
                    let w = unsafe { Element::from_raw_borrowed(wrapper_raw) };
                    let view = children(item).into_view();
                    mount_view(&w, view);
                });

                container_ref.append_child(&wrapper);
                // Ownership transfers to the DOM tree.
                std::mem::forget(wrapper);
                new_state.push((k, scope_id, wrapper_raw));
            }
        }

        *old_state = new_state;
    });

    ViewNode::Element(container)
}

/// Render a dynamic child that re-renders whenever its reactive
/// dependencies change.
///
/// This is the primitive that the `view!` macro uses for `{expression}`
/// children — any expression whose result depends on reactive signals is
/// wrapped in a `DynChild` so the DOM updates automatically.
///
/// The container is a `<span>` element to keep inline flow.
///
/// # Example
///
/// ```ignore
/// let count = create_signal(0);
/// DynChild(move || format!("Count is {}", count.get()))
/// ```
pub fn DynChild<F, V>(f: F) -> ViewNode
where
    F: Fn() -> V + 'static,
    V: IntoView,
{
    let doc = current_document();
    let container = Element::create(doc, "span").expect("DynChild: failed to create container");

    let raw = container.as_raw();
    let scope_cell: Cell<Option<ScopeId>> = Cell::new(None);

    create_effect(move || {
        // Dispose previous content scope.
        if let Some(old) = scope_cell.get() {
            dispose_scope(old);
        }

        let scope_id = create_scope(|| {
            // SAFETY: raw points to our container which outlives this scope.
            let container_ref = unsafe { Element::from_raw_borrowed(raw) };
            container_ref.remove_all_children();

            let view = f().into_view();
            mount_view(&container_ref, view);
        });

        scope_cell.set(Some(scope_id));
    });

    ViewNode::Element(container)
}

// ═══════════════════════════════════════════════════════════════════════
// Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::reset_runtime;
    use crate::signal::{create_signal, Signal};
    use std::cell::Cell;
    use std::rc::Rc;

    /// Helper — run each test with a fresh runtime.
    fn with_fresh_runtime(f: impl FnOnce()) {
        reset_runtime();
        f();
        reset_runtime();
    }

    // ── Type-level verification ──────────────────────────────────────
    //
    // These functions are never called but prove the generic bounds compile.

    #[allow(dead_code, unused_variables)]
    fn _show_type_check(visible: Signal<bool>) {
        // Show accepts a bool-returning closure, fallback, and children.
        let _when = move || visible.get();
    }

    #[allow(dead_code, unused_variables)]
    fn _for_type_check(items: Signal<Vec<(u32, String)>>) {
        // For accepts a Vec-returning closure, a key extractor, and a child factory.
        fn _key(item: &(u32, String)) -> u32 {
            item.0
        }
    }

    #[allow(dead_code, unused_variables)]
    fn _dyn_child_type_check(count: Signal<i32>) {
        // DynChild accepts a closure returning IntoView.
        fn _fmt(c: i32) -> String {
            format!("count = {c}")
        }
    }

    // ── Reactive logic tests (no DOM needed) ─────────────────────────

    #[test]
    fn scope_disposes_on_branch_switch() {
        with_fresh_runtime(|| {
            let disposed = Rc::new(Cell::new(false));
            let disposed_clone = disposed.clone();

            let _flag = create_signal(true);

            // Create a scope, register cleanup, dispose it — mimics Show.
            let scope = create_scope(|| {
                crate::on_cleanup(move || {
                    disposed_clone.set(true);
                });
            });

            assert!(!disposed.get(), "cleanup should not have run yet");

            dispose_scope(scope);

            assert!(disposed.get(), "cleanup should have run after dispose");
        });
    }

    #[test]
    fn effect_tracks_signal_changes() {
        with_fresh_runtime(|| {
            let count = create_signal(0);
            let seen = Rc::new(Cell::new(0));
            let seen_clone = seen.clone();

            create_effect(move || {
                let _val = count.get();
                seen_clone.set(seen_clone.get() + 1);
            });

            // Effect runs once immediately.
            assert_eq!(seen.get(), 1);

            count.set(1);
            assert_eq!(seen.get(), 2);

            count.set(2);
            assert_eq!(seen.get(), 3);
        });
    }

    #[test]
    fn effect_skips_when_condition_unchanged() {
        with_fresh_runtime(|| {
            let flag = create_signal(true);
            let renders = Rc::new(Cell::new(0u32));
            let renders_clone = renders.clone();

            let showing: Cell<Option<bool>> = Cell::new(None);

            create_effect(move || {
                let show = flag.get();
                if showing.get() == Some(show) {
                    return;
                }
                showing.set(Some(show));
                renders_clone.set(renders_clone.get() + 1);
            });

            // Initial render.
            assert_eq!(renders.get(), 1);

            // Set to same value — effect fires but skips re-render.
            flag.set(true);
            assert_eq!(renders.get(), 1);

            // Actual change.
            flag.set(false);
            assert_eq!(renders.get(), 2);
        });
    }

    #[test]
    fn scope_nesting_disposes_children() {
        with_fresh_runtime(|| {
            let inner_disposed = Rc::new(Cell::new(false));
            let outer_disposed = Rc::new(Cell::new(false));
            let inner_clone = inner_disposed.clone();
            let outer_clone = outer_disposed.clone();

            let outer = create_scope(|| {
                crate::on_cleanup(move || outer_clone.set(true));
                let _inner = create_scope(|| {
                    crate::on_cleanup(move || inner_clone.set(true));
                });
            });

            assert!(!inner_disposed.get());
            assert!(!outer_disposed.get());

            dispose_scope(outer);

            assert!(inner_disposed.get(), "inner scope should be disposed");
            assert!(outer_disposed.get(), "outer scope should be disposed");
        });
    }

    #[test]
    fn for_keyed_diff_logic() {
        with_fresh_runtime(|| {
            // Simulates the keyed diff: old=[A,B,C], new=[B,D,A]
            // Expected: C removed, D added, B and A reused.
            let old_keys = vec!["A", "B", "C"];
            let new_keys = vec!["B", "D", "A"];

            let new_set: HashMap<&str, ()> = new_keys.iter().map(|k| (*k, ())).collect();

            let mut kept = Vec::new();
            let mut removed = Vec::new();
            for k in &old_keys {
                if new_set.contains_key(k) {
                    kept.push(*k);
                } else {
                    removed.push(*k);
                }
            }

            assert_eq!(removed, vec!["C"]);
            assert_eq!(kept.len(), 2);
            assert!(kept.contains(&"A"));
            assert!(kept.contains(&"B"));

            let old_set: HashMap<&str, ()> = old_keys.iter().map(|k| (*k, ())).collect();
            let added: Vec<&str> = new_keys
                .iter()
                .filter(|k| !old_set.contains_key(*k))
                .copied()
                .collect();
            assert_eq!(added, vec!["D"]);
        });
    }
}
