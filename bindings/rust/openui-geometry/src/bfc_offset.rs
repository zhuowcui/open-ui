//! BFC (Block Formatting Context) geometry types — extracted from Blink.
//!
//! Source: core/layout/geometry/bfc_offset.h, core/layout/geometry/bfc_rect.h
//!
//! BFC coordinates use `line_offset` (always left-to-right, regardless of text
//! direction) and `block_offset` (top-to-bottom in horizontal writing modes).
//! This differs from logical coordinates which flip for RTL.

use crate::LayoutUnit;
use std::fmt;

// ---------------------------------------------------------------------------
// BfcDelta
// ---------------------------------------------------------------------------

/// A delta (displacement) in BFC coordinates.
///
/// Source: `BfcDelta` in `geometry/bfc_offset.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BfcDelta {
    pub line_offset_delta: LayoutUnit,
    pub block_offset_delta: LayoutUnit,
}

impl BfcDelta {
    #[inline]
    pub fn new(line_offset_delta: LayoutUnit, block_offset_delta: LayoutUnit) -> Self {
        Self {
            line_offset_delta,
            block_offset_delta,
        }
    }
}

// ---------------------------------------------------------------------------
// BfcOffset
// ---------------------------------------------------------------------------

/// Position of a rect relative to a block formatting context.
///
/// BFCs are agnostic to text direction and use `line_offset` instead of
/// `inline_offset`. Care must be taken when converting to `LogicalOffset` to
/// respect text direction.
///
/// Source: `BfcOffset` in `geometry/bfc_offset.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BfcOffset {
    pub line_offset: LayoutUnit,
    pub block_offset: LayoutUnit,
}

impl BfcOffset {
    #[inline]
    pub const fn new(line_offset: LayoutUnit, block_offset: LayoutUnit) -> Self {
        Self {
            line_offset,
            block_offset,
        }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self {
            line_offset: LayoutUnit::zero(),
            block_offset: LayoutUnit::zero(),
        }
    }
}

impl std::ops::Add<BfcDelta> for BfcOffset {
    type Output = BfcOffset;

    #[inline]
    fn add(self, delta: BfcDelta) -> BfcOffset {
        BfcOffset {
            line_offset: self.line_offset + delta.line_offset_delta,
            block_offset: self.block_offset + delta.block_offset_delta,
        }
    }
}

impl std::ops::AddAssign<BfcDelta> for BfcOffset {
    #[inline]
    fn add_assign(&mut self, delta: BfcDelta) {
        *self = *self + delta;
    }
}

impl fmt::Display for BfcOffset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BfcOffset({}, {})",
            self.line_offset.to_f32(),
            self.block_offset.to_f32()
        )
    }
}

// ---------------------------------------------------------------------------
// BfcRect
// ---------------------------------------------------------------------------

/// Position and size of a rect relative to a block formatting context.
///
/// Defined by a start offset (top-left in LTR horizontal-tb) and an end offset
/// (bottom-right). Invariant: end >= start on both axes.
///
/// Source: `BfcRect` in `geometry/bfc_rect.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BfcRect {
    pub start_offset: BfcOffset,
    pub end_offset: BfcOffset,
}

impl BfcRect {
    /// Create a new BfcRect. Panics in debug mode if end < start.
    #[inline]
    pub fn new(start_offset: BfcOffset, end_offset: BfcOffset) -> Self {
        debug_assert!(
            end_offset.line_offset >= start_offset.line_offset,
            "BfcRect: end.line_offset ({}) < start.line_offset ({})",
            end_offset.line_offset.to_f32(),
            start_offset.line_offset.to_f32()
        );
        debug_assert!(
            end_offset.block_offset >= start_offset.block_offset,
            "BfcRect: end.block_offset ({}) < start.block_offset ({})",
            end_offset.block_offset.to_f32(),
            start_offset.block_offset.to_f32()
        );
        Self {
            start_offset,
            end_offset,
        }
    }

    #[inline]
    pub fn line_start_offset(&self) -> LayoutUnit {
        self.start_offset.line_offset
    }

    #[inline]
    pub fn line_end_offset(&self) -> LayoutUnit {
        self.end_offset.line_offset
    }

    #[inline]
    pub fn block_start_offset(&self) -> LayoutUnit {
        self.start_offset.block_offset
    }

    #[inline]
    pub fn block_end_offset(&self) -> LayoutUnit {
        self.end_offset.block_offset
    }

    /// Block size of this rect. Returns `LayoutUnit::max_value()` if the end
    /// offset is max (unbounded).
    #[inline]
    pub fn block_size(&self) -> LayoutUnit {
        if self.end_offset.block_offset == LayoutUnit::max() {
            return LayoutUnit::max();
        }
        self.end_offset.block_offset - self.start_offset.block_offset
    }

    /// Inline size of this rect. Handles `LayoutUnit::max()` edge cases
    /// identically to Blink.
    #[inline]
    pub fn inline_size(&self) -> LayoutUnit {
        if self.end_offset.line_offset == LayoutUnit::max() {
            if self.start_offset.line_offset == LayoutUnit::max() {
                return LayoutUnit::zero();
            }
            return LayoutUnit::max();
        }
        self.end_offset.line_offset - self.start_offset.line_offset
    }
}

impl fmt::Display for BfcRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BfcRect({} -> {}, {} -> {})",
            self.start_offset.line_offset.to_f32(),
            self.end_offset.line_offset.to_f32(),
            self.start_offset.block_offset.to_f32(),
            self.end_offset.block_offset.to_f32()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    #[test]
    fn bfc_offset_default_is_zero() {
        let o = BfcOffset::default();
        assert_eq!(o.line_offset, LayoutUnit::zero());
        assert_eq!(o.block_offset, LayoutUnit::zero());
    }

    #[test]
    fn bfc_offset_add_delta() {
        let o = BfcOffset::new(lu(10), lu(20));
        let d = BfcDelta::new(lu(5), lu(-3));
        let r = o + d;
        assert_eq!(r.line_offset, lu(15));
        assert_eq!(r.block_offset, lu(17));
    }

    #[test]
    fn bfc_offset_add_assign() {
        let mut o = BfcOffset::new(lu(10), lu(20));
        o += BfcDelta::new(lu(5), lu(5));
        assert_eq!(o, BfcOffset::new(lu(15), lu(25)));
    }

    #[test]
    fn bfc_rect_basic() {
        let r = BfcRect::new(
            BfcOffset::new(lu(10), lu(20)),
            BfcOffset::new(lu(110), lu(80)),
        );
        assert_eq!(r.line_start_offset(), lu(10));
        assert_eq!(r.line_end_offset(), lu(110));
        assert_eq!(r.block_start_offset(), lu(20));
        assert_eq!(r.block_end_offset(), lu(80));
        assert_eq!(r.inline_size(), lu(100));
        assert_eq!(r.block_size(), lu(60));
    }

    #[test]
    fn bfc_rect_max_block_size() {
        let r = BfcRect::new(
            BfcOffset::new(lu(0), lu(0)),
            BfcOffset::new(lu(100), LayoutUnit::max()),
        );
        assert_eq!(r.block_size(), LayoutUnit::max());
    }

    #[test]
    fn bfc_rect_max_inline_size() {
        let r = BfcRect::new(
            BfcOffset::new(lu(0), lu(0)),
            BfcOffset::new(LayoutUnit::max(), lu(100)),
        );
        assert_eq!(r.inline_size(), LayoutUnit::max());
    }

    #[test]
    fn bfc_rect_both_max_inline_is_zero() {
        let r = BfcRect::new(
            BfcOffset::new(LayoutUnit::max(), lu(0)),
            BfcOffset::new(LayoutUnit::max(), lu(100)),
        );
        assert_eq!(r.inline_size(), LayoutUnit::zero());
    }

    #[test]
    fn bfc_offset_equality() {
        let a = BfcOffset::new(lu(10), lu(20));
        let b = BfcOffset::new(lu(10), lu(20));
        let c = BfcOffset::new(lu(10), lu(21));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn bfc_rect_equality() {
        let a = BfcRect::new(BfcOffset::new(lu(0), lu(0)), BfcOffset::new(lu(100), lu(50)));
        let b = BfcRect::new(BfcOffset::new(lu(0), lu(0)), BfcOffset::new(lu(100), lu(50)));
        let c = BfcRect::new(BfcOffset::new(lu(0), lu(0)), BfcOffset::new(lu(100), lu(51)));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }
}
