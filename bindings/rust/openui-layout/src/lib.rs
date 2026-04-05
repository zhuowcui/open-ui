//! Block/Flex/Grid/Inline layout algorithms extracted from Blink's NG layout.
//!
//! SP9: block layout. SP10: flexbox layout.

mod constraint_space;
mod fragment;
pub(crate) mod length_resolver;
pub mod block;
pub mod flex;
pub mod inline;
pub mod ruby;

pub use constraint_space::ConstraintSpace;
pub use fragment::{Fragment, FragmentKind};
pub use length_resolver::resolve_length;
pub use block::block_layout;
pub use flex::flex_layout;
pub use crate::inline::algorithm::inline_layout;
pub use ruby::{compute_ruby_layout, max_ruby_overhang, clamp_overhang, RubyInfo, RubyLayout};
