//! Comprehensive tests for the reactive runtime.

use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::reset_runtime;
use crate::{
    batch, create_effect, create_memo, create_scope, create_signal, dispose_scope, on_cleanup,
};

/// Helper — run each test with a fresh runtime.
fn with_fresh_runtime(f: impl FnOnce()) {
    reset_runtime();
    f();
    reset_runtime();
}

// ═══════════════════════════════════════════════════════════════════════
// Signal basics
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn signal_create_and_get() {
    with_fresh_runtime(|| {
        let s = create_signal(42);
        assert_eq!(s.get(), 42);
    });
}

#[test]
fn signal_set_and_get() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        s.set(7);
        assert_eq!(s.get(), 7);
    });
}

#[test]
fn signal_multiple_independent() {
    with_fresh_runtime(|| {
        let a = create_signal(1);
        let b = create_signal(2);
        let c = create_signal(3);
        assert_eq!(a.get(), 1);
        assert_eq!(b.get(), 2);
        assert_eq!(c.get(), 3);
        a.set(10);
        assert_eq!(a.get(), 10);
        assert_eq!(b.get(), 2);
    });
}

#[test]
fn signal_with_string() {
    with_fresh_runtime(|| {
        let s = create_signal(String::from("hello"));
        assert_eq!(s.get(), "hello");
        s.set(String::from("world"));
        assert_eq!(s.get(), "world");
    });
}

#[test]
fn signal_with_vec() {
    with_fresh_runtime(|| {
        let s = create_signal(vec![1, 2, 3]);
        assert_eq!(s.get(), vec![1, 2, 3]);
        s.set(vec![4, 5]);
        assert_eq!(s.get(), vec![4, 5]);
    });
}

#[test]
fn signal_with_struct() {
    with_fresh_runtime(|| {
        #[derive(Clone, Debug, PartialEq)]
        struct Point {
            x: f64,
            y: f64,
        }
        let s = create_signal(Point { x: 1.0, y: 2.0 });
        assert_eq!(s.get(), Point { x: 1.0, y: 2.0 });
        s.set(Point { x: 3.0, y: 4.0 });
        assert_eq!(s.get(), Point { x: 3.0, y: 4.0 });
    });
}

#[test]
fn signal_update_in_place() {
    with_fresh_runtime(|| {
        let s = create_signal(vec![1, 2]);
        s.update(|v| v.push(3));
        assert_eq!(s.get(), vec![1, 2, 3]);
    });
}

#[test]
fn signal_update_numeric() {
    with_fresh_runtime(|| {
        let s = create_signal(10);
        s.update(|v| *v += 5);
        assert_eq!(s.get(), 15);
    });
}

#[test]
fn signal_set_many_times() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        for i in 1..=100 {
            s.set(i);
        }
        assert_eq!(s.get(), 100);
    });
}

#[test]
fn signal_bool() {
    with_fresh_runtime(|| {
        let s = create_signal(false);
        assert!(!s.get());
        s.set(true);
        assert!(s.get());
    });
}

#[test]
fn signal_option() {
    with_fresh_runtime(|| {
        let s = create_signal::<Option<i32>>(None);
        assert_eq!(s.get(), None);
        s.set(Some(42));
        assert_eq!(s.get(), Some(42));
    });
}

// ═══════════════════════════════════════════════════════════════════════
// Effect tracking
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn effect_runs_immediately() {
    with_fresh_runtime(|| {
        let ran = Rc::new(Cell::new(false));
        let ran_c = ran.clone();
        create_effect(move || {
            ran_c.set(true);
        });
        assert!(ran.get());
    });
}

#[test]
fn effect_reruns_on_dependency_change() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        create_effect(move || {
            let _ = s.get();
            count_c.set(count_c.get() + 1);
        });
        assert_eq!(count.get(), 1); // initial run
        s.set(1);
        assert_eq!(count.get(), 2);
        s.set(2);
        assert_eq!(count.get(), 3);
    });
}

#[test]
fn effect_tracks_multiple_dependencies() {
    with_fresh_runtime(|| {
        let a = create_signal(1);
        let b = create_signal(2);
        let sum = Rc::new(Cell::new(0));
        let sum_c = sum.clone();
        create_effect(move || {
            sum_c.set(a.get() + b.get());
        });
        assert_eq!(sum.get(), 3);
        a.set(10);
        assert_eq!(sum.get(), 12);
        b.set(20);
        assert_eq!(sum.get(), 30);
    });
}

#[test]
fn effect_dynamic_dependencies() {
    with_fresh_runtime(|| {
        let toggle = create_signal(true);
        let a = create_signal(1);
        let b = create_signal(2);
        let result = Rc::new(Cell::new(0));
        let result_c = result.clone();
        let run_count = Rc::new(Cell::new(0));
        let run_count_c = run_count.clone();

        create_effect(move || {
            run_count_c.set(run_count_c.get() + 1);
            if toggle.get() {
                result_c.set(a.get());
            } else {
                result_c.set(b.get());
            }
        });
        assert_eq!(result.get(), 1);
        assert_eq!(run_count.get(), 1);

        // Changing `a` triggers (it's a dep)
        a.set(10);
        assert_eq!(result.get(), 10);
        assert_eq!(run_count.get(), 2);

        // Changing `b` does NOT trigger (not currently subscribed)
        b.set(20);
        assert_eq!(run_count.get(), 2);

        // Switch to reading `b`
        toggle.set(false);
        assert_eq!(result.get(), 20);

        // Now `a` changes should NOT trigger
        let before = run_count.get();
        a.set(100);
        assert_eq!(run_count.get(), before);

        // But `b` changes should trigger
        b.set(30);
        assert_eq!(result.get(), 30);
    });
}

#[test]
fn effect_does_not_run_for_non_dependencies() {
    with_fresh_runtime(|| {
        let a = create_signal(1);
        let b = create_signal(2);
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        create_effect(move || {
            let _ = a.get();
            count_c.set(count_c.get() + 1);
        });
        assert_eq!(count.get(), 1);
        b.set(99);
        assert_eq!(count.get(), 1); // should NOT have re-run
    });
}

#[test]
fn effect_with_no_dependencies_runs_once() {
    with_fresh_runtime(|| {
        let count = Rc::new(Cell::new(0));
        let count_c = count.clone();
        create_effect(move || {
            count_c.set(count_c.get() + 1);
        });
        assert_eq!(count.get(), 1);
        // Nothing to trigger a rerun
    });
}

#[test]
fn effect_nested_signal_access() {
    with_fresh_runtime(|| {
        let outer = create_signal(1);
        let inner = create_signal(10);
        let result = Rc::new(Cell::new(0));
        let result_c = result.clone();

        create_effect(move || {
            let o = outer.get();
            let i = inner.get();
            result_c.set(o + i);
        });
        assert_eq!(result.get(), 11);
        inner.set(20);
        assert_eq!(result.get(), 21);
        outer.set(2);
        assert_eq!(result.get(), 22);
    });
}

#[test]
fn effect_updates_signal_chain() {
    with_fresh_runtime(|| {
        let source = create_signal(1);
        let derived = create_signal(0);
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let log_c = log.clone();

        // Effect 1: source → derived
        create_effect(move || {
            derived.set(source.get() * 2);
        });

        // Effect 2: derived → log
        create_effect(move || {
            log_c.borrow_mut().push(derived.get());
        });

        assert_eq!(derived.get(), 2);
        source.set(5);
        assert_eq!(derived.get(), 10);
        // The log should contain the initial and updated values
        let log = log.borrow();
        assert!(log.contains(&2));
        assert!(log.contains(&10));
    });
}

#[test]
fn multiple_effects_same_signal() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count_a = Rc::new(Cell::new(0));
        let count_b = Rc::new(Cell::new(0));
        let ca = count_a.clone();
        let cb = count_b.clone();

        create_effect(move || {
            let _ = s.get();
            ca.set(ca.get() + 1);
        });
        create_effect(move || {
            let _ = s.get();
            cb.set(cb.get() + 1);
        });

        assert_eq!(count_a.get(), 1);
        assert_eq!(count_b.get(), 1);
        s.set(1);
        assert_eq!(count_a.get(), 2);
        assert_eq!(count_b.get(), 2);
    });
}

#[test]
fn effect_rerun_sees_latest_value() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let seen = Rc::new(RefCell::new(Vec::new()));
        let seen_c = seen.clone();
        create_effect(move || {
            seen_c.borrow_mut().push(s.get());
        });
        s.set(1);
        s.set(2);
        s.set(3);
        assert_eq!(*seen.borrow(), vec![0, 1, 2, 3]);
    });
}

// ═══════════════════════════════════════════════════════════════════════
// Memo
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn memo_computes_on_creation() {
    with_fresh_runtime(|| {
        let s = create_signal(3);
        let m = create_memo(move || s.get() * 2);
        assert_eq!(m.get(), 6);
    });
}

#[test]
fn memo_returns_cached_value() {
    with_fresh_runtime(|| {
        let compute_count = Rc::new(Cell::new(0));
        let cc = compute_count.clone();
        let s = create_signal(5);
        let m = create_memo(move || {
            cc.set(cc.get() + 1);
            s.get() + 1
        });
        assert_eq!(m.get(), 6);
        assert_eq!(m.get(), 6); // second read — no recompute
        // The memo computes once eagerly and once in the effect's initial run
        // (needed to establish dependency tracking). Subsequent reads don't recompute.
        assert_eq!(compute_count.get(), 2);
    });
}

#[test]
fn memo_recomputes_on_dependency_change() {
    with_fresh_runtime(|| {
        let s = create_signal(2);
        let m = create_memo(move || s.get() * 10);
        assert_eq!(m.get(), 20);
        s.set(3);
        assert_eq!(m.get(), 30);
    });
}

#[test]
fn memo_skips_update_when_value_unchanged() {
    with_fresh_runtime(|| {
        let s = create_signal(4);
        let downstream_count = Rc::new(Cell::new(0));
        let dc = downstream_count.clone();

        // Memo that clamps to max 10
        let m = create_memo(move || {
            let v = s.get();
            if v > 10 { 10 } else { v }
        });

        create_effect(move || {
            let _ = m.get();
            dc.set(dc.get() + 1);
        });

        assert_eq!(m.get(), 4);
        let _initial_count = downstream_count.get();

        // Set to same clamped value — memo doesn't propagate
        s.set(5);
        assert_eq!(m.get(), 5);

        // Set above clamp threshold twice — memo value stays 10
        s.set(100);
        assert_eq!(m.get(), 10);
        let count_after_first_clamp = downstream_count.get();

        s.set(200); // still clamps to 10 — no change
        assert_eq!(m.get(), 10);
        // Downstream should not have run again since value didn't change
        assert_eq!(downstream_count.get(), count_after_first_clamp);
    });
}

#[test]
fn memo_chained() {
    with_fresh_runtime(|| {
        let s = create_signal(2);
        let doubled = create_memo(move || s.get() * 2);
        let quadrupled = create_memo(move || doubled.get() * 2);
        assert_eq!(quadrupled.get(), 8);
        s.set(3);
        assert_eq!(doubled.get(), 6);
        assert_eq!(quadrupled.get(), 12);
    });
}

#[test]
fn memo_with_string() {
    with_fresh_runtime(|| {
        let name = create_signal(String::from("world"));
        let greeting = create_memo(move || format!("Hello, {}!", name.get()));
        assert_eq!(greeting.get(), "Hello, world!");
        name.set(String::from("Rust"));
        assert_eq!(greeting.get(), "Hello, Rust!");
    });
}

// ═══════════════════════════════════════════════════════════════════════
// Batch
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn batch_defers_effects() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();
        create_effect(move || {
            let _ = s.get();
            cc.set(cc.get() + 1);
        });
        assert_eq!(count.get(), 1);

        batch(|| {
            s.set(1);
            s.set(2);
            s.set(3);
            // Effect should NOT have run yet
            assert_eq!(count.get(), 1);
        });
        // Now the effect should have run exactly once more
        assert_eq!(count.get(), 2);
        assert_eq!(s.get(), 3);
    });
}

#[test]
fn batch_multiple_signals() {
    with_fresh_runtime(|| {
        let a = create_signal(0);
        let b = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();
        create_effect(move || {
            let _ = a.get() + b.get();
            cc.set(cc.get() + 1);
        });
        assert_eq!(count.get(), 1);

        batch(|| {
            a.set(1);
            b.set(2);
        });
        // Effect ran once for the batch, not twice
        assert_eq!(count.get(), 2);
    });
}

#[test]
fn batch_nested() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();
        create_effect(move || {
            let _ = s.get();
            cc.set(cc.get() + 1);
        });
        assert_eq!(count.get(), 1);

        batch(|| {
            s.set(1);
            batch(|| {
                s.set(2);
                s.set(3);
            });
            // Inner batch ended but outer hasn't — still deferred
            assert_eq!(count.get(), 1);
            s.set(4);
        });
        // All updates flushed once
        assert_eq!(count.get(), 2);
        assert_eq!(s.get(), 4);
    });
}

#[test]
fn batch_no_changes() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();
        create_effect(move || {
            let _ = s.get();
            cc.set(cc.get() + 1);
        });
        assert_eq!(count.get(), 1);

        batch(|| {
            // No changes
        });
        assert_eq!(count.get(), 1); // no extra runs
    });
}

#[test]
fn batch_with_memo() {
    with_fresh_runtime(|| {
        let s = create_signal(1);
        let doubled = create_memo(move || s.get() * 2);
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let log_c = log.clone();
        create_effect(move || {
            log_c.borrow_mut().push(doubled.get());
        });

        batch(|| {
            s.set(2);
            s.set(3);
        });

        let log = log.borrow();
        // Initial value, then final batch value
        assert_eq!(*log.first().unwrap(), 2);
        assert_eq!(*log.last().unwrap(), 6);
    });
}

// ═══════════════════════════════════════════════════════════════════════
// Scope
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn scope_collects_effects() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();

        let scope_id = create_scope(|| {
            create_effect(move || {
                let _ = s.get();
                cc.set(cc.get() + 1);
            });
        });

        assert_eq!(count.get(), 1);
        s.set(1);
        assert_eq!(count.get(), 2);

        dispose_scope(scope_id);
        s.set(2);
        assert_eq!(count.get(), 2); // effect no longer runs
    });
}

#[test]
fn scope_nested_dispose() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let outer_count = Rc::new(Cell::new(0));
        let inner_count = Rc::new(Cell::new(0));
        let oc = outer_count.clone();
        let ic = inner_count.clone();

        let outer_scope = create_scope(|| {
            create_effect(move || {
                let _ = s.get();
                oc.set(oc.get() + 1);
            });
            let _inner_scope = create_scope(|| {
                create_effect(move || {
                    let _ = s.get();
                    ic.set(ic.get() + 1);
                });
            });
        });

        assert_eq!(outer_count.get(), 1);
        assert_eq!(inner_count.get(), 1);
        s.set(1);
        assert_eq!(outer_count.get(), 2);
        assert_eq!(inner_count.get(), 2);

        // Disposing outer should also dispose inner
        dispose_scope(outer_scope);
        s.set(2);
        assert_eq!(outer_count.get(), 2);
        assert_eq!(inner_count.get(), 2);
    });
}

#[test]
fn on_cleanup_runs_on_dispose() {
    with_fresh_runtime(|| {
        let cleaned = Rc::new(Cell::new(false));
        let cc = cleaned.clone();

        let scope_id = create_scope(|| {
            on_cleanup(move || {
                cc.set(true);
            });
        });

        assert!(!cleaned.get());
        dispose_scope(scope_id);
        assert!(cleaned.get());
    });
}

#[test]
fn on_cleanup_multiple() {
    with_fresh_runtime(|| {
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let l1 = log.clone();
        let l2 = log.clone();

        let scope_id = create_scope(|| {
            on_cleanup(move || l1.borrow_mut().push(1));
            on_cleanup(move || l2.borrow_mut().push(2));
        });

        dispose_scope(scope_id);
        let log = log.borrow();
        assert!(log.contains(&1));
        assert!(log.contains(&2));
    });
}

#[test]
fn disposed_scope_effects_dont_run() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();

        let scope_id = create_scope(|| {
            create_effect(move || {
                let _ = s.get();
                cc.set(cc.get() + 1);
            });
        });

        dispose_scope(scope_id);

        // Setting signal should NOT trigger the disposed effect
        s.set(1);
        s.set(2);
        s.set(3);
        assert_eq!(count.get(), 1); // only the initial run
    });
}

#[test]
fn scope_without_effects() {
    with_fresh_runtime(|| {
        let cleaned = Rc::new(Cell::new(false));
        let cc = cleaned.clone();
        let scope_id = create_scope(|| {
            on_cleanup(move || cc.set(true));
        });
        dispose_scope(scope_id);
        assert!(cleaned.get());
    });
}

// ═══════════════════════════════════════════════════════════════════════
// Integration
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn integration_signal_effect_signal_chain() {
    with_fresh_runtime(|| {
        let source = create_signal(5);
        let derived = create_signal(0);
        create_effect(move || {
            derived.set(source.get() + 100);
        });
        assert_eq!(derived.get(), 105);
        source.set(10);
        assert_eq!(derived.get(), 110);
    });
}

#[test]
fn integration_diamond_dependency() {
    with_fresh_runtime(|| {
        let a = create_signal(1);
        let b = create_memo(move || a.get() + 1);
        let c = create_memo(move || a.get() * 2);
        let d_val = Rc::new(Cell::new(0));
        let dv = d_val.clone();
        create_effect(move || {
            dv.set(b.get() + c.get());
        });

        // a=1 → b=2, c=2 → d=4
        assert_eq!(d_val.get(), 4);
        a.set(3);
        // a=3 → b=4, c=6 → d=10
        assert_eq!(d_val.get(), 10);
    });
}

#[test]
fn integration_rapid_updates() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();
        create_effect(move || {
            let _ = s.get();
            cc.set(cc.get() + 1);
        });

        for i in 1..=1000 {
            s.set(i);
        }
        // 1 initial + 1000 updates = 1001
        assert_eq!(count.get(), 1001);
        assert_eq!(s.get(), 1000);
    });
}

#[test]
fn integration_rapid_updates_batched() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();
        create_effect(move || {
            let _ = s.get();
            cc.set(cc.get() + 1);
        });

        batch(|| {
            for i in 1..=1000 {
                s.set(i);
            }
        });
        // 1 initial + 1 batch flush = 2
        assert_eq!(count.get(), 2);
        assert_eq!(s.get(), 1000);
    });
}

#[test]
fn integration_scope_cleanup_frees_resources() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count = Rc::new(Cell::new(0));
        let cc = count.clone();

        for _ in 0..100 {
            let cc2 = cc.clone();
            let scope_id = create_scope(|| {
                create_effect(move || {
                    let _ = s.get();
                    cc2.set(cc2.get() + 1);
                });
            });
            dispose_scope(scope_id);
        }

        let count_after_scopes = count.get();
        s.set(1);
        // None of the disposed effects should trigger
        assert_eq!(count.get(), count_after_scopes);
    });
}

#[test]
fn integration_effect_writing_back_to_signal() {
    with_fresh_runtime(|| {
        let input = create_signal(5);
        let clamped = create_signal(0);

        create_effect(move || {
            let v = input.get();
            clamped.set(v.clamp(0, 10));
        });

        assert_eq!(clamped.get(), 5);
        input.set(15);
        assert_eq!(clamped.get(), 10);
        input.set(-3);
        assert_eq!(clamped.get(), 0);
    });
}

#[test]
fn integration_memo_in_effect() {
    with_fresh_runtime(|| {
        let count = create_signal(1);
        let doubled = create_memo(move || count.get() * 2);
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let log_c = log.clone();

        create_effect(move || {
            log_c.borrow_mut().push(doubled.get());
        });

        count.set(2);
        count.set(3);

        let log = log.borrow();
        assert_eq!(log[0], 2);
        assert!(log.contains(&4));
        assert!(log.contains(&6));
    });
}

#[test]
fn integration_scope_with_memo_and_effect() {
    with_fresh_runtime(|| {
        let s = create_signal(1);
        let log = Rc::new(RefCell::new(Vec::<i32>::new()));
        let log_c = log.clone();

        let scope_id = create_scope(|| {
            let m = create_memo(move || s.get() * 3);
            create_effect(move || {
                log_c.borrow_mut().push(m.get());
            });
        });

        s.set(2);
        assert_eq!(*log.borrow().last().unwrap(), 6);

        dispose_scope(scope_id);
        s.set(3);
        // After dispose, no new entries
        assert_eq!(*log.borrow().last().unwrap(), 6);
    });
}

#[test]
fn signal_is_copy() {
    with_fresh_runtime(|| {
        let s = create_signal(42);
        let s2 = s; // copy
        assert_eq!(s.get(), 42);
        assert_eq!(s2.get(), 42);
        s.set(100);
        assert_eq!(s2.get(), 100); // same underlying signal
    });
}

#[test]
fn memo_is_copy() {
    with_fresh_runtime(|| {
        let s = create_signal(1);
        let m = create_memo(move || s.get() + 1);
        let m2 = m;
        assert_eq!(m.get(), 2);
        assert_eq!(m2.get(), 2);
    });
}

#[test]
fn effect_with_conditional_panic_guard() {
    // Ensure effects don't break the runtime on repeated runs
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let result = Rc::new(Cell::new(0));
        let rc = result.clone();
        create_effect(move || {
            let v = s.get();
            rc.set(v * v);
        });
        assert_eq!(result.get(), 0);
        s.set(5);
        assert_eq!(result.get(), 25);
        s.set(12);
        assert_eq!(result.get(), 144);
    });
}

#[test]
fn multiple_scopes_independent() {
    with_fresh_runtime(|| {
        let s = create_signal(0);
        let count_a = Rc::new(Cell::new(0));
        let count_b = Rc::new(Cell::new(0));
        let ca = count_a.clone();
        let cb = count_b.clone();

        let scope_a = create_scope(|| {
            create_effect(move || {
                let _ = s.get();
                ca.set(ca.get() + 1);
            });
        });

        let scope_b = create_scope(|| {
            create_effect(move || {
                let _ = s.get();
                cb.set(cb.get() + 1);
            });
        });

        s.set(1);
        assert_eq!(count_a.get(), 2);
        assert_eq!(count_b.get(), 2);

        // Dispose only scope_a
        dispose_scope(scope_a);
        s.set(2);
        assert_eq!(count_a.get(), 2); // stopped
        assert_eq!(count_b.get(), 3); // still running

        dispose_scope(scope_b);
        s.set(3);
        assert_eq!(count_b.get(), 3); // now stopped too
    });
}
