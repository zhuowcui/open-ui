//! Logical size — extracted from Blink's `LogicalSize`.
//!
//! Logical coordinates are writing-mode-aware: inline vs block dimension.
//! For horizontal-tb (the default), inline = width, block = height.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/logical_size.h

use crate::LayoutUnit;

/// A size in logical coordinates (inline-size × block-size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogicalSize {
    pub inline_size: LayoutUnit,
    pub block_size: LayoutUnit,
}

impl LogicalSize {
    #[inline]
    pub const fn new(inline_size: LayoutUnit, block_size: LayoutUnit) -> Self {
        Self { inline_size, block_size }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            inline_size: LayoutUnit::zero(),
            block_size: LayoutUnit::zero(),
        }
    }
}
