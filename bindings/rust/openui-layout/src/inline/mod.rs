//! Inline formatting context — inline item collection and line breaking.
//!
//! Extracted from Blink's inline layout engine:
//! `third_party/blink/renderer/core/layout/inline/`
//!
//! This module implements:
//! - Inline item collection (flattening the DOM into a linear sequence)
//! - White-space processing (CSS Text Module Level 3 §4)
//! - Text shaping integration
//! - Line breaking (UAX#14 + CSS word-break/overflow-wrap/line-break)

pub mod algorithm;
pub mod first_letter;
pub mod first_line;
pub mod initial_letter;
pub mod items;
pub mod items_builder;
pub mod line_breaker;
pub mod line_info;
pub mod line_width;
pub mod score_line_breaker;
pub mod text_combine;
