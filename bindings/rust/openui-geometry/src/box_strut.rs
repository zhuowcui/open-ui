//! Box strut — margin/padding/border edge values.
//!
//! Extracted from Blink's `BoxStrut` (core/layout/geometry/box_strut.h).
//! Stores four `LayoutUnit` values for the four physical edges.

use crate::{LayoutUnit, WritingDirectionMode};

/// Four-sided box strut in logical coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LogicalBoxStrut {
    pub inline_start: LayoutUnit,
    pub inline_end: LayoutUnit,
    pub block_start: LayoutUnit,
    pub block_end: LayoutUnit,
}

impl LogicalBoxStrut {
    #[inline]
    pub const fn new(
        inline_start: LayoutUnit,
        inline_end: LayoutUnit,
        block_start: LayoutUnit,
        block_end: LayoutUnit,
    ) -> Self {
        Self {
            inline_start,
            inline_end,
            block_start,
            block_end,
        }
    }

    #[inline]
    pub const fn zero() -> Self {
        Self::new(
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
            LayoutUnit::zero(),
        )
    }

    #[inline]
    pub fn inline_sum(self) -> LayoutUnit {
        self.inline_start + self.inline_end
    }

    #[inline]
    pub fn block_sum(self) -> LayoutUnit {
        self.block_start + self.block_end
    }

    /// Convert logical edges to the physical edge storage used by fragments.
    pub fn to_physical(self, writing_direction: WritingDirectionMode) -> BoxStrut {
        if writing_direction.is_horizontal() {
            let (left, right) = if writing_direction.is_rtl() {
                (self.inline_end, self.inline_start)
            } else {
                (self.inline_start, self.inline_end)
            };
            BoxStrut::new(self.block_start, right, self.block_end, left)
        } else {
            let inline_reversed = writing_direction.is_flipped_lines() ^ writing_direction.is_rtl();
            let (top, bottom) = if inline_reversed {
                (self.inline_end, self.inline_start)
            } else {
                (self.inline_start, self.inline_end)
            };
            let (left, right) = if writing_direction.is_flipped_blocks() {
                (self.block_end, self.block_start)
            } else {
                (self.block_start, self.block_end)
            };
            BoxStrut::new(top, right, bottom, left)
        }
    }
}

/// Four-sided box strut (top, right, bottom, left) in physical coordinates.
///
/// Fragments and paint keep this representation. Layout projects it through
/// `LogicalBoxStrut` while making writing-mode-dependent decisions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BoxStrut {
    pub top: LayoutUnit,
    pub right: LayoutUnit,
    pub bottom: LayoutUnit,
    pub left: LayoutUnit,
}

impl BoxStrut {
    #[inline]
    pub const fn new(
        top: LayoutUnit,
        right: LayoutUnit,
        bottom: LayoutUnit,
        left: LayoutUnit,
    ) -> Self {
        Self {
            top,
            right,
            bottom,
            left,
        }
    }

    #[inline]
    pub const fn all(value: LayoutUnit) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
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

    /// Convert physical fragment/style edges to logical layout edges.
    pub fn to_logical(self, writing_direction: WritingDirectionMode) -> LogicalBoxStrut {
        if writing_direction.is_horizontal() {
            let (inline_start, inline_end) = if writing_direction.is_rtl() {
                (self.right, self.left)
            } else {
                (self.left, self.right)
            };
            LogicalBoxStrut::new(inline_start, inline_end, self.top, self.bottom)
        } else {
            let inline_reversed = writing_direction.is_flipped_lines() ^ writing_direction.is_rtl();
            let (inline_start, inline_end) = if inline_reversed {
                (self.bottom, self.top)
            } else {
                (self.top, self.bottom)
            };
            let (block_start, block_end) = if writing_direction.is_flipped_blocks() {
                (self.right, self.left)
            } else {
                (self.left, self.right)
            };
            LogicalBoxStrut::new(inline_start, inline_end, block_start, block_end)
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(value: i32) -> LayoutUnit {
        LayoutUnit::from_i32(value)
    }

    fn physical_edges() -> BoxStrut {
        BoxStrut::new(lu(1), lu(2), lu(3), lu(4))
    }

    #[test]
    fn horizontal_ltr_and_rtl_map_inline_edges() {
        let ltr = WritingDirectionMode::horizontal_ltr();
        assert_eq!(
            physical_edges().to_logical(ltr),
            LogicalBoxStrut::new(lu(4), lu(2), lu(1), lu(3))
        );

        let rtl = WritingDirectionMode::new(true, false, false, true);
        assert_eq!(
            physical_edges().to_logical(rtl),
            LogicalBoxStrut::new(lu(2), lu(4), lu(1), lu(3))
        );
    }

    #[test]
    fn vertical_and_sideways_modes_map_all_edges() {
        let vertical_rl_ltr = WritingDirectionMode::new(false, true, false, false);
        assert_eq!(
            physical_edges().to_logical(vertical_rl_ltr),
            LogicalBoxStrut::new(lu(1), lu(3), lu(2), lu(4))
        );

        let vertical_lr_rtl = WritingDirectionMode::new(false, false, false, true);
        assert_eq!(
            physical_edges().to_logical(vertical_lr_rtl),
            LogicalBoxStrut::new(lu(3), lu(1), lu(4), lu(2))
        );

        let sideways_lr_ltr = WritingDirectionMode::new(false, false, true, false);
        assert_eq!(
            physical_edges().to_logical(sideways_lr_ltr),
            LogicalBoxStrut::new(lu(3), lu(1), lu(4), lu(2))
        );
    }

    #[test]
    fn physical_logical_round_trip_covers_all_direction_flags() {
        for writing_direction in [
            WritingDirectionMode::horizontal_ltr(),
            WritingDirectionMode::new(true, false, false, true),
            WritingDirectionMode::new(false, true, false, false),
            WritingDirectionMode::new(false, true, false, true),
            WritingDirectionMode::new(false, false, false, false),
            WritingDirectionMode::new(false, false, false, true),
            WritingDirectionMode::new(false, false, true, false),
            WritingDirectionMode::new(false, false, true, true),
        ] {
            assert_eq!(
                physical_edges()
                    .to_logical(writing_direction)
                    .to_physical(writing_direction),
                physical_edges()
            );
        }
    }
}
