//! Public [`Signal`] and [`Memo`] types — lightweight `Copy` handles into the runtime.

use std::marker::PhantomData;

use crate::runtime::{flush_pending, SignalId, RUNTIME};

/// A reactive signal holding a value of type `T`.
///
/// `Signal<T>` is [`Copy`] — it is just a generational index into the
/// thread-local runtime arena. Reading inside an effect automatically
/// subscribes to future changes.
pub struct Signal<T: 'static> {
    pub(crate) id: SignalId,
    _marker: PhantomData<T>,
}

impl<T: 'static> Copy for Signal<T> {}
impl<T: 'static> Clone for Signal<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> std::fmt::Debug for Signal<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Signal").field("id", &self.id).finish()
    }
}

impl<T: Clone + 'static> Signal<T> {
    /// Read the current value, cloning it out.
    ///
    /// If called inside an effect, the effect is subscribed to future changes
    /// of this signal.
    pub fn get(&self) -> T {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            rt.get_signal_value_cloned::<T>(self.id)
                .expect("signal has been disposed or type mismatch")
        })
    }
}

impl<T: 'static> Signal<T> {
    /// Overwrite the value and notify all subscribers.
    pub fn set(&self, value: T) {
        // Borrow, set value, enqueue subscribers, release borrow.
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let subs = rt.set_signal_value(self.id, Box::new(value));
            rt.enqueue_effects(subs);
        });
        // Flush outside the borrow so effect closures can re-borrow.
        flush_pending();
    }

    /// Mutate the value in place via a closure and notify subscribers.
    pub fn update(&self, f: impl FnOnce(&mut T)) {
        RUNTIME.with(|rt| {
            let mut rt = rt.borrow_mut();
            let slot = rt.signals.get_mut(self.id.index as usize)
                .and_then(|s| s.as_mut())
                .expect("signal has been disposed");
            if slot.generation != self.id.generation {
                panic!("signal generation mismatch");
            }
            let val = slot.value.downcast_mut::<T>().expect("signal type mismatch");
            f(val);
            let subs = slot.subscribers.clone();
            rt.enqueue_effects(subs);
        });
        flush_pending();
    }

    /// Get the underlying [`SignalId`] for advanced use.
    pub fn id(&self) -> SignalId {
        self.id
    }
}

/// Create a new signal with the given initial value.
///
/// # Example
/// ```ignore
/// let count = create_signal(0);
/// assert_eq!(count.get(), 0);
/// count.set(1);
/// assert_eq!(count.get(), 1);
/// ```
pub fn create_signal<T: 'static>(initial: T) -> Signal<T> {
    let id = RUNTIME.with(|rt| {
        let mut rt = rt.borrow_mut();
        rt.create_signal_raw(Box::new(initial))
    });
    Signal {
        id,
        _marker: PhantomData,
    }
}

// ── Memo ─────────────────────────────────────────────────────────────

/// A derived/computed signal that caches its result.
///
/// `Memo<T>` re-computes only when one of its dependencies changes, and
/// only propagates if the new value differs from the cached one
/// (requires [`PartialEq`]).
pub struct Memo<T: 'static> {
    signal: Signal<T>,
}

impl<T: 'static> Copy for Memo<T> {}
impl<T: 'static> Clone for Memo<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: 'static> std::fmt::Debug for Memo<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Memo")
            .field("signal", &self.signal)
            .finish()
    }
}

impl<T: Clone + 'static> Memo<T> {
    /// Read the cached value. Subscribes the calling effect, if any.
    pub fn get(&self) -> T {
        self.signal.get()
    }
}

/// Create a memo — a derived signal that recomputes when its dependencies change.
///
/// The computation only stores a new value (and notifies downstream) when the
/// result differs from the previously cached value.
///
/// # Example
/// ```ignore
/// let a = create_signal(2);
/// let doubled = create_memo(move || a.get() * 2);
/// assert_eq!(doubled.get(), 4);
/// ```
pub fn create_memo<T: Clone + PartialEq + 'static>(f: impl Fn() -> T + 'static) -> Memo<T> {
    let initial = f();
    let signal = create_signal(initial);

    // The effect keeps the memo signal up to date.
    crate::effect::create_effect_raw(move || {
        let new_val = f();
        // Only update if value actually changed to avoid needless downstream triggers.
        let old_val: T = signal.get();
        if new_val != old_val {
            signal.set(new_val);
        }
    });

    Memo { signal }
}
