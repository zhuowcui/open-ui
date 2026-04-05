//! Block/Flex/Grid/Inline layout algorithms extracted from Blink's NG layout.
//!
//! SP9: block layout. SP10: flexbox layout. SP12: full block layout + floats.

mod constraint_space;
mod fragment;
pub(crate) mod length_resolver;
pub mod block;
pub mod relative;
pub mod flex;
pub mod inline;
pub mod ruby;
pub mod exclusions;
pub mod layout_result;
pub mod inflow_position;

pub use constraint_space::{ConstraintSpace, ConstraintSpaceBuilder};
pub use fragment::{Fragment, FragmentKind};
pub use length_resolver::resolve_length;
pub use block::block_layout;
pub use block::establishes_new_fc;
pub use relative::apply_relative_offset;
pub use flex::flex_layout;
pub use crate::inline::algorithm::inline_layout;
pub use ruby::{compute_ruby_layout, max_ruby_overhang, clamp_overhang, RubyInfo, RubyLayout};
pub use layout_result::{LayoutResult, LayoutStatus, BreakBetween, AdjoiningObjectTypes};
pub use inflow_position::{PreviousInflowPosition, InflowChildData};
pub use exclusions::{ExclusionSpace};
