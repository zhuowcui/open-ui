//! Flexbox layout algorithm — extracted from Blink's `flex_layout_algorithm.cc`.
//!
//! This module implements the complete CSS Flexbox Level 1 specification,
//! faithfully reimplemented from Blink's source character by character.
//!
//! Source files studied:
//! - `core/layout/flex/flex_layout_algorithm.cc` (3,221 lines)
//! - `core/layout/flex/flex_item.h` (142 lines)
//! - `core/layout/flex/flex_line.h` (119 lines)
//! - `core/layout/flex/line_flexer.cc` (182 lines)
//! - `core/layout/flex/flex_line_breaker.cc` (417 lines)
//! - `core/layout/flex/flex_child_iterator.cc` (36 lines)

mod item;
mod line;
mod line_flexer;
mod line_breaker;
mod alignment;
mod algorithm;
#[allow(dead_code)]
mod intrinsic;

pub use item::{FlexItem, FlexerState};
pub use line::FlexLine;
pub use algorithm::flex_layout;
