//! Logical rect — extracted from Blink's `LogicalRect`.
//!
//! Combines a logical offset with a logical size. Represents a rectangle
//! in writing-mode-aware coordinates.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/logical_rect.h

use crate::{LayoutUnit, LogicalOffset, LogicalSize};

/// A rectangle in logical coordinates (offset + size).
///
/// `offset` is the inline-start / block-start corner.
/// `size` holds inline-size and block-size.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogicalRect {
    pub offset: LogicalOffset,
    pub size: LogicalSize,
}

impl LogicalRect {
    #[inline]
    pub const fn new(offset: LogicalOffset, size: LogicalSize) -> Self {
        Self { offset, size }
    }

    #[inline]
    pub fn from_position_and_size(
        inline_offset: LayoutUnit,
        block_offset: LayoutUnit,
        inline_size: LayoutUnit,
        block_size: LayoutUnit,
    ) -> Self {
        Self {
            offset: LogicalOffset::new(inline_offset, block_offset),
            size: LogicalSize::new(inline_size, block_size),
        }
    }

    #[inline]
    pub const fn inline_offset(&self) -> LayoutUnit { self.offset.inline_offset }
    #[inline]
    pub const fn block_offset(&self) -> LayoutUnit { self.offset.block_offset }
    #[inline]
    pub const fn inline_size(&self) -> LayoutUnit { self.size.inline_size }
    #[inline]
    pub const fn block_size(&self) -> LayoutUnit { self.size.block_size }

    /// End position in the inline direction.
    #[inline]
    pub fn inline_end(&self) -> LayoutUnit {
        self.offset.inline_offset + self.size.inline_size
    }

    /// End position in the block direction.
    #[inline]
    pub fn block_end(&self) -> LayoutUnit {
        self.offset.block_offset + self.size.block_size
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.size.inline_size.raw() <= 0 || self.size.block_size.raw() <= 0
    }
}
