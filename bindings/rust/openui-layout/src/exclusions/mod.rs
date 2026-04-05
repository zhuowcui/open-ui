//! Exclusion space and float positioning — extracted from Blink's exclusions/.
//!
//! Source: core/layout/exclusions/exclusion_space.h/cc
//!
//! This module tracks float exclusion rectangles within a BFC and provides
//! queries for finding layout opportunities (available space between floats).

mod exclusion_space;
pub mod float_utils;

pub use exclusion_space::ExclusionSpace;
pub use exclusion_space::{ExclusionType, ExclusionArea, LayoutOpportunity, ClearType};
pub use float_utils::{UnpositionedFloat, PositionedFloat, position_float, compute_margin_box_inline_size};
