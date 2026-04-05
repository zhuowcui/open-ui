//! Exclusion space and float positioning — extracted from Blink's exclusions/.
//!
//! Source: core/layout/exclusions/exclusion_space.h/cc
//!
//! This module tracks float exclusion rectangles within a BFC and provides
//! queries for finding layout opportunities (available space between floats).

mod exclusion_space;

pub use exclusion_space::ExclusionSpace;
