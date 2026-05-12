//! Blink-identical geometry primitives for Open UI's native rendering engine.
//!
//! This crate provides the exact same fixed-point arithmetic, length types,
//! and geometric primitives that Blink uses internally. Every operator, every
//! rounding rule, every overflow behavior is extracted character-by-character
//! from Blink's source code.

mod bfc_offset;
mod box_strut;
mod layout_unit;
mod length;
mod logical_offset;
mod logical_rect;
mod logical_size;
mod margin_strut;
mod min_max_sizes;
mod physical_offset;
mod physical_rect;
mod physical_size;
mod writing_mode;

pub use bfc_offset::{BfcDelta, BfcOffset, BfcRect};
pub use box_strut::BoxStrut;
pub use layout_unit::{LayoutUnit, INDEFINITE_SIZE};
pub use length::{Length, LengthType};
pub use logical_offset::LogicalOffset;
pub use logical_rect::LogicalRect;
pub use logical_size::LogicalSize;
pub use margin_strut::MarginStrut;
pub use min_max_sizes::MinMaxSizes;
pub use physical_offset::PhysicalOffset;
pub use physical_rect::PhysicalRect;
pub use physical_size::PhysicalSize;
pub use writing_mode::{WritingDirectionMode, WritingModeConverter};
