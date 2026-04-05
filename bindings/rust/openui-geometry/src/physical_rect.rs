//! Physical rect — extracted from Blink's `PhysicalRect`.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/physical_rect.h

use crate::{LayoutUnit, PhysicalOffset, PhysicalSize};

/// A rectangle in physical coordinates (offset + size).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PhysicalRect {
    pub offset: PhysicalOffset,
    pub size: PhysicalSize,
}

impl PhysicalRect {
    #[inline]
    pub const fn new(offset: PhysicalOffset, size: PhysicalSize) -> Self {
        Self { offset, size }
    }

    #[inline]
    pub fn from_xywh(x: LayoutUnit, y: LayoutUnit, w: LayoutUnit, h: LayoutUnit) -> Self {
        Self {
            offset: PhysicalOffset::new(x, y),
            size: PhysicalSize::new(w, h),
        }
    }

    #[inline]
    pub const fn x(&self) -> LayoutUnit { self.offset.left }
    #[inline]
    pub const fn y(&self) -> LayoutUnit { self.offset.top }
    #[inline]
    pub const fn width(&self) -> LayoutUnit { self.size.width }
    #[inline]
    pub const fn height(&self) -> LayoutUnit { self.size.height }

    #[inline]
    pub fn right(&self) -> LayoutUnit { self.offset.left + self.size.width }
    #[inline]
    pub fn bottom(&self) -> LayoutUnit { self.offset.top + self.size.height }

    #[inline]
    pub const fn is_empty(&self) -> bool { self.size.is_empty() }

    /// Convert to f32 rect for Skia. Uses `to_f32()` which matches Blink's
    /// `ToFloat()` — exact conversion from fixed-point to float.
    #[inline]
    pub fn to_f32_rect(&self) -> (f32, f32, f32, f32) {
        (
            self.offset.left.to_f32(),
            self.offset.top.to_f32(),
            self.size.width.to_f32(),
            self.size.height.to_f32(),
        )
    }

    /// Pixel-snap this rect to integer device pixels.
    /// Matches Blink's `ToPixelSnappedRect()`.
    pub fn to_pixel_snapped(&self) -> (i32, i32, i32, i32) {
        let x = self.offset.left.floor().to_i32();
        let y = self.offset.top.floor().to_i32();
        let right = self.right().ceil().to_i32();
        let bottom = self.bottom().ceil().to_i32();
        (x, y, right - x, bottom - y)
    }

    /// Shrink this rect inward by the given strut amounts.
    pub fn shrink(&self, top: LayoutUnit, right: LayoutUnit, bottom: LayoutUnit, left: LayoutUnit) -> Self {
        Self {
            offset: PhysicalOffset::new(self.offset.left + left, self.offset.top + top),
            size: PhysicalSize::new(
                self.size.width - left - right,
                self.size.height - top - bottom,
            ),
        }
    }

    /// Compute the smallest rect that contains both `self` and `other`.
    ///
    /// Mirrors Blink's `PhysicalRect::Unite()`.
    pub fn unite(&self, other: &Self) -> Self {
        if self.is_empty() {
            return *other;
        }
        if other.is_empty() {
            return *self;
        }
        let min_x = self.x().min_of(other.x());
        let min_y = self.y().min_of(other.y());
        let max_x = self.right().max_of(other.right());
        let max_y = self.bottom().max_of(other.bottom());
        Self::from_xywh(min_x, min_y, max_x - min_x, max_y - min_y)
    }
}
