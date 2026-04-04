//! Margin strut for margin collapsing — extracted from Blink's `MarginStrut`.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/margin_strut.h
//!
//! Blink's margin collapsing tracks the largest positive and smallest (most
//! negative) margin independently, then combines them. This is the exact
//! algorithm from CSS 2.1 §8.3.1.

use crate::LayoutUnit;

/// Tracks collapsing margins through a block formatting context.
///
/// Blink's `MarginStrut` stores:
/// - `positive_margin`: largest positive margin seen
/// - `negative_margin`: most negative margin seen (stored as negative value)
///
/// The collapsed margin = positive_margin + negative_margin (since negative_margin is negative).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginStrut {
    pub positive_margin: LayoutUnit,
    pub negative_margin: LayoutUnit,
}

impl MarginStrut {
    #[inline]
    pub const fn new() -> Self {
        Self {
            positive_margin: LayoutUnit::zero(),
            negative_margin: LayoutUnit::zero(),
        }
    }

    /// Append a margin value. Positive values accumulate into `positive_margin`,
    /// negative values into `negative_margin`. This is the core of CSS margin
    /// collapsing — only the largest positive and most-negative survive.
    #[inline]
    pub fn append(&mut self, value: LayoutUnit) {
        if value.raw() > 0 {
            if value > self.positive_margin {
                self.positive_margin = value;
            }
        } else if value.raw() < 0 {
            if value < self.negative_margin {
                self.negative_margin = value;
            }
        }
    }

    /// The collapsed margin value = max(positive) + min(negative).
    #[inline]
    pub fn sum(&self) -> LayoutUnit {
        self.positive_margin + self.negative_margin
    }

    /// True if no margins have been accumulated.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positive_margin.raw() == 0 && self.negative_margin.raw() == 0
    }
}

impl Default for MarginStrut {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_strut_is_zero() {
        let strut = MarginStrut::new();
        assert_eq!(strut.sum(), LayoutUnit::zero());
        assert!(strut.is_empty());
    }

    #[test]
    fn positive_margins_take_largest() {
        let mut strut = MarginStrut::new();
        strut.append(LayoutUnit::from_i32(10));
        strut.append(LayoutUnit::from_i32(20));
        strut.append(LayoutUnit::from_i32(5));
        assert_eq!(strut.sum(), LayoutUnit::from_i32(20));
    }

    #[test]
    fn negative_margins_take_most_negative() {
        let mut strut = MarginStrut::new();
        strut.append(LayoutUnit::from_i32(-10));
        strut.append(LayoutUnit::from_i32(-20));
        strut.append(LayoutUnit::from_i32(-5));
        assert_eq!(strut.sum(), LayoutUnit::from_i32(-20));
    }

    #[test]
    fn mixed_margins_collapse() {
        let mut strut = MarginStrut::new();
        strut.append(LayoutUnit::from_i32(30));
        strut.append(LayoutUnit::from_i32(-10));
        // Collapsed = 30 + (-10) = 20
        assert_eq!(strut.sum(), LayoutUnit::from_i32(20));
    }
}
