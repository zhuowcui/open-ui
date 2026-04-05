//! Blink-identical geometry primitives for Open UI's native rendering engine.
//!
//! This crate provides the exact same fixed-point arithmetic, length types,
//! and geometric primitives that Blink uses internally. Every operator, every
//! rounding rule, every overflow behavior is extracted character-by-character
//! from Blink's source code.

mod layout_unit;
mod length;
mod physical_offset;
mod physical_rect;
mod physical_size;
mod logical_size;
mod box_strut;
mod margin_strut;
mod min_max_sizes;

pub use layout_unit::{LayoutUnit, INDEFINITE_SIZE};
pub use length::{Length, LengthType};
pub use physical_offset::PhysicalOffset;
pub use physical_rect::PhysicalRect;
pub use physical_size::PhysicalSize;
pub use logical_size::LogicalSize;
pub use box_strut::BoxStrut;
pub use margin_strut::MarginStrut;
pub use min_max_sizes::MinMaxSizes;
