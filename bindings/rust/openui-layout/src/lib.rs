//! Block/Flex/Grid/Inline layout algorithms extracted from Blink's NG layout.
//!
//! SP9 implements block layout only. Flex (SP10), Grid (SP13), and Inline (SP11)
//! will be added in later phases.

mod constraint_space;
mod fragment;
mod length_resolver;
mod block;

pub use constraint_space::ConstraintSpace;
pub use fragment::{Fragment, FragmentKind};
pub use length_resolver::resolve_length;
pub use block::block_layout;
