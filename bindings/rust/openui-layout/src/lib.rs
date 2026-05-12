//! Block/Flex/Grid/Inline layout algorithms extracted from Blink's NG layout.
//!
//! SP9: block layout. SP10: flexbox layout. SP12: full block layout + floats.

pub mod bfc_resolution;
pub mod block;
mod constraint_space;
pub mod css_sizing;
pub mod exclusions;
pub mod flex;
mod fragment;
pub mod fragmentation;
pub mod inflow_position;
pub mod inline;
pub mod intrinsic_sizing;
pub mod layout_result;
pub(crate) mod length_resolver;
pub mod margin_collapsing;
pub mod multicol;
pub mod new_formatting_context;
pub mod out_of_flow;
pub mod relative;
pub mod ruby;
pub mod size_constraints;
pub mod sticky;

pub use crate::inline::algorithm::inline_layout;
pub use crate::inline::algorithm::{apply_inline_fragmentation, resume_inline_from_break_token};
pub use bfc_resolution::{BfcBlockOffsetState, PendingFloats};
pub use block::block_layout;
pub use block::establishes_new_fc;
pub use constraint_space::{ConstraintSpace, ConstraintSpaceBuilder};
pub use css_sizing::{
    apply_aspect_ratio, apply_aspect_ratio_with_auto, compute_automatic_size,
    compute_definite_size, resolve_preferred_size, resolve_sizing_keyword, SizingKeyword,
};
pub use exclusions::ExclusionSpace;
pub use flex::flex_layout;
pub use fragment::{Fragment, FragmentKind};
pub use fragmentation::{
    BlockBreakToken, BreakAppeal, BreakToken, FragmentainerSpace, InlineBreakToken,
};
pub use inflow_position::{InflowChildData, PreviousInflowPosition};
pub use intrinsic_sizing::{
    compute_block_size_from_content, compute_intrinsic_block_sizes, compute_intrinsic_inline_sizes,
    compute_replaced_intrinsic_sizes, shrink_to_fit_inline_size, IntrinsicSizes,
};
pub use layout_result::{AdjoiningObjectTypes, BreakBetween, LayoutResult, LayoutStatus};
pub use length_resolver::resolve_length;
pub use multicol::{layout_columns, resolve_column_count_and_width};
pub use new_formatting_context::{creates_new_formatting_context, layout_new_formatting_context};
pub use out_of_flow::{layout_out_of_flow_children, OutOfFlowCandidate};
pub use relative::apply_relative_offset;
pub use ruby::{clamp_overhang, compute_ruby_layout, max_ruby_overhang, RubyInfo, RubyLayout};
pub use size_constraints::{
    apply_box_sizing_adjustment, constrain_block_size, constrain_inline_size, resolve_block_size,
    resolve_inline_size, resolve_size_constraints, SizeConstraint,
};
pub use sticky::{apply_sticky_offset, compute_sticky_offset, StickyPositionData};
