//! # Open UI Reactive Runtime
//!
//! A Leptos-style fine-grained reactive system in pure Rust.
//!
//! ## Core primitives
//!
//! | Primitive | Purpose |
//! |-----------|---------|
//! | [`Signal`] | Readable/writable reactive value |
//! | [`Memo`] | Cached derived value |
//! | [`create_effect`] | Side-effect that re-runs on dependency changes |
//! | [`batch`] | Defer effect execution across multiple updates |
//! | [`create_scope`] | Group effects for batch disposal |
//! | [`dispose_scope`] | Tear down a scope and its children |
//! | [`on_cleanup`] | Register a cleanup callback in the current scope |

pub mod effect;
pub mod runtime;
pub mod scope;
pub mod signal;

// Re-export the public API at crate root.
pub use effect::{batch, create_effect};
pub use runtime::{EffectId, ScopeId, SignalId};
pub use scope::{create_scope, dispose_scope, on_cleanup};
pub use signal::{create_memo, create_signal, Memo, Signal};

#[cfg(test)]
mod tests;
