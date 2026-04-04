//! Box strut — margin/padding/border edge values.
//!
//! Extracted from Blink's `BoxStrut` (core/layout/geometry/box_strut.h).
//! Stores four `LayoutUnit` values for the four physical edges.

use crate::LayoutUnit;

/// Four-sided box strut (top, right, bottom, left) in physical coordinates.
///
/// Used for margins, padding, and border widths. Blink calls this `BoxStrut`
/// in logical coordinates and `PhysicalBoxStrut` in physical. We use physical
/// since we target horizontal-tb writing mode for SP9.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoxStrut {
    pub top: LayoutUnit,
    pub right: LayoutUnit,
    pub bottom: LayoutUnit,
    pub left: LayoutUnit,
}

impl BoxStrut {
    #[inline]
    pub const fn new(top: LayoutUnit, right: LayoutUnit, bottom: LayoutUnit, left: LayoutUnit) -> Self {
        Self { top, right, bottom, left }
    }

    #[inline]
    pub const fn all(value: LayoutUnit) -> Self {
        Self { top: value, right: value, bottom: value, left: value }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            top: LayoutUnit::zero(),
            right: LayoutUnit::zero(),
            bottom: LayoutUnit::zero(),
            left: LayoutUnit::zero(),
        }
    }

    /// Total horizontal extent (left + right).
    #[inline]
    pub fn inline_sum(&self) -> LayoutUnit {
        self.left + self.right
    }

    /// Total vertical extent (top + bottom).
    #[inline]
    pub fn block_sum(&self) -> LayoutUnit {
        self.top + self.bottom
    }
}

impl std::ops::Add for BoxStrut {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            top: self.top + rhs.top,
            right: self.right + rhs.right,
            bottom: self.bottom + rhs.bottom,
            left: self.left + rhs.left,
        }
    }
}
