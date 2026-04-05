//! Logical offset — extracted from Blink's `LogicalOffset`.
//!
//! Logical coordinates are writing-mode-aware: inline vs block dimension.
//! For horizontal-tb, inline = left, block = top.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/logical_offset.h

use crate::LayoutUnit;

/// An offset in logical coordinates (inline-offset, block-offset).
///
/// In horizontal-tb the inline-offset maps to physical left and block-offset
/// maps to physical top.  In vertical writing modes the axes swap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogicalOffset {
    pub inline_offset: LayoutUnit,
    pub block_offset: LayoutUnit,
}

impl LogicalOffset {
    #[inline]
    pub const fn new(inline_offset: LayoutUnit, block_offset: LayoutUnit) -> Self {
        Self { inline_offset, block_offset }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            inline_offset: LayoutUnit::zero(),
            block_offset: LayoutUnit::zero(),
        }
    }
}

impl std::ops::Add for LogicalOffset {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            inline_offset: self.inline_offset + rhs.inline_offset,
            block_offset: self.block_offset + rhs.block_offset,
        }
    }
}

impl std::ops::AddAssign for LogicalOffset {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.inline_offset += rhs.inline_offset;
        self.block_offset += rhs.block_offset;
    }
}

impl std::ops::Sub for LogicalOffset {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            inline_offset: self.inline_offset - rhs.inline_offset,
            block_offset: self.block_offset - rhs.block_offset,
        }
    }
}

impl std::ops::Neg for LogicalOffset {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            inline_offset: -self.inline_offset,
            block_offset: -self.block_offset,
        }
    }
}
