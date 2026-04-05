//! Block/Flex/Grid/Inline layout algorithms extracted from Blink's NG layout.
//!
//! SP9: block layout. SP10: flexbox layout. SP12: full block layout + floats.

mod constraint_space;
mod fragment;
pub(crate) mod length_resolver;
pub mod block;
pub mod relative;
pub mod out_of_flow;
pub mod flex;
pub mod inline;
pub mod ruby;
pub mod exclusions;
pub mod layout_result;
pub mod inflow_position;
pub mod bfc_resolution;
pub mod new_formatting_context;
pub mod fragmentation;
pub mod sticky;
pub mod multicol;
pub mod margin_collapsing;

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
pub use out_of_flow::{OutOfFlowCandidate, layout_out_of_flow_children};
pub use bfc_resolution::{BfcBlockOffsetState, PendingFloats};
pub use new_formatting_context::{creates_new_formatting_context, layout_new_formatting_context};
pub use fragmentation::{BlockBreakToken, BreakToken, BreakAppeal, FragmentainerSpace};
pub use sticky::{apply_sticky_offset, StickyPositionData, compute_sticky_offset};
pub use multicol::{layout_columns, resolve_column_count_and_width};
