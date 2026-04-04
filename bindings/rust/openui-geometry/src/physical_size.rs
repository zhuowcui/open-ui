//! Physical size — extracted from Blink's `PhysicalSize`.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/physical_size.h

use crate::LayoutUnit;

/// A 2D size in physical coordinates (width, height).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalSize {
    pub width: LayoutUnit,
    pub height: LayoutUnit,
}

impl PhysicalSize {
    #[inline]
    pub const fn new(width: LayoutUnit, height: LayoutUnit) -> Self {
        Self { width, height }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            width: LayoutUnit::zero(),
            height: LayoutUnit::zero(),
        }
    }

    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.width.raw() <= 0 || self.height.raw() <= 0
    }
}
