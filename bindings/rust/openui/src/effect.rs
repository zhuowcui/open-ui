//! Side-effect creation and batching.

use crate::runtime::{flush_pending, run_effect_standalone, EffectId, RUNTIME};

/// Create a side effect that re-runs whenever any signal it reads changes.
///
/// The function executes **immediately once** upon creation, and then
/// re-executes every time a dependency is updated.
///
/// # Example
/// ```ignore
/// let count = create_signal(0);
/// create_effect(move || {
///     println!("count = {}", count.get());
/// });
/// count.set(1); // prints "count = 1"
/// ```
pub fn create_effect(f: impl Fn() + 'static) {
    create_effect_raw(f);
}

/// Internal: allocate an effect slot and run it once, returning its id.
pub(crate) fn create_effect_raw(f: impl Fn() + 'static) -> EffectId {
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        let scope = rt.current_scope;
        let id = if let Some(index) = rt.free_effects.pop() {
            rt.effects[index as usize] = Some(crate::runtime::EffectSlot {
                f: Box::new(f),
                dependencies: Vec::new(),
                scope,
                active: true,
            });
            EffectId(index)
        } else {
            let index = rt.effects.len() as u32;
            rt.effects.push(Some(crate::runtime::EffectSlot {
                f: Box::new(f),
                dependencies: Vec::new(),
                scope,
                active: true,
            }));
            EffectId(index)
        };

        // Register in current scope
        if let Some(scope_id) = scope {
            if let Some(Some(scope_slot)) = rt.scopes.get_mut(scope_id.0 as usize) {
                scope_slot.effects.push(id);
            }
        }
        id
    });

    // Run the effect once immediately (outside the borrow)
    run_effect_standalone(id);
    id
}

/// Run a batch of updates — effects are deferred until the batch ends.
///
/// This is useful when making multiple signal updates that should only
/// trigger effects once (with the final state), rather than once per update.
///
/// # Example
/// ```ignore
/// let a = create_signal(0);
/// let b = create_signal(0);
/// create_effect(move || { /* reads a and b */ });
///
/// batch(|| {
///     a.set(1);
///     b.set(2);
/// }); // effect runs once here, not twice
/// ```
pub fn batch(f: impl FnOnce()) {
    RUNTIME.with(|rt| {
        rt.borrow_mut().batch_depth += 1;
    });
    f();
    RUNTIME.with(|rt| {
        rt.borrow_mut().batch_depth -= 1;
    });
    flush_pending();
}
