//! Physical offset in 2D space — extracted from Blink's `PhysicalOffset`.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/physical_offset.h

use crate::LayoutUnit;

/// A 2D offset in physical coordinates (left, top).
///
/// Blink's `PhysicalOffset` stores two `LayoutUnit` values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalOffset {
    pub left: LayoutUnit,
    pub top: LayoutUnit,
}

impl PhysicalOffset {
    #[inline]
    pub const fn new(left: LayoutUnit, top: LayoutUnit) -> Self {
        Self { left, top }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            left: LayoutUnit::zero(),
            top: LayoutUnit::zero(),
        }
    }
}

impl std::ops::Add for PhysicalOffset {
    type Output = Self;
    #[inline]
    fn add(self, rhs: Self) -> Self {
        Self {
            left: self.left + rhs.left,
            top: self.top + rhs.top,
        }
    }
}

impl std::ops::AddAssign for PhysicalOffset {
    #[inline]
    fn add_assign(&mut self, rhs: Self) {
        self.left += rhs.left;
        self.top += rhs.top;
    }
}

impl std::ops::Sub for PhysicalOffset {
    type Output = Self;
    #[inline]
    fn sub(self, rhs: Self) -> Self {
        Self {
            left: self.left - rhs.left,
            top: self.top - rhs.top,
        }
    }
}

impl std::ops::Neg for PhysicalOffset {
    type Output = Self;
    #[inline]
    fn neg(self) -> Self {
        Self {
            left: -self.left,
            top: -self.top,
        }
    }
}
