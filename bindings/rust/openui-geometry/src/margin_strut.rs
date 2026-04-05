//! Margin strut for margin collapsing — extracted from Blink's `MarginStrut`.
//!
//! Source: third_party/blink/renderer/core/layout/geometry/margin_strut.h
//!
//! Blink's margin collapsing tracks the largest positive and smallest (most
//! negative) margin independently, then combines them. This is the exact
//! algorithm from CSS 2.1 §8.3.1.
//!
//! Extended in SP12 with quirky margin support, discard_margins, and
//! trim_leading_margins — matching Blink's full MarginStrut.

use crate::LayoutUnit;

/// Tracks collapsing margins through a block formatting context.
///
/// Blink's `MarginStrut` stores:
/// - `positive_margin`: largest positive margin seen
/// - `negative_margin`: most negative margin seen (stored as negative value)
/// - `quirky_positive_margin`: largest positive quirky margin (always ≥ 0)
///
/// The collapsed margin = max(quirky_positive, positive) + negative
/// (since negative_margin is negative).
///
/// Source: `MarginStrut` in `geometry/margin_strut.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MarginStrut {
    pub positive_margin: LayoutUnit,
    pub negative_margin: LayoutUnit,

    /// Quirky margins are always default margins (always positive). Quirky
    /// containers need to ignore quirky end margins.
    pub quirky_positive_margin: LayoutUnit,

    /// If set, only non-quirky margins are appended. Set when a quirky
    /// container encounters its start edge.
    pub is_quirky_container_start: bool,

    /// If set, all adjoining margins are discarded (collapsed to zero).
    pub discard_margins: bool,

    /// Set by containers with `margin-trim: block-start`. Margins adjacent
    /// to the block-start content edge are truncated.
    pub trim_leading_margins: bool,
}

impl MarginStrut {
    #[inline]
    pub const fn new() -> Self {
        Self {
            positive_margin: LayoutUnit::zero(),
            negative_margin: LayoutUnit::zero(),
            quirky_positive_margin: LayoutUnit::zero(),
            is_quirky_container_start: false,
            discard_margins: false,
            trim_leading_margins: false,
        }
    }

    /// Append a margin value. Positive values accumulate into the positive
    /// bucket, negative values into the negative bucket. Only the largest
    /// positive and most-negative survive — this is the core of CSS margin
    /// collapsing.
    ///
    /// If `is_quirky` is true, the value is accumulated into
    /// `quirky_positive_margin` instead of `positive_margin` (quirky margins
    /// are always positive). When `is_quirky_container_start` is set, quirky
    /// margins are ignored entirely.
    ///
    /// Source: `MarginStrut::Append` in Blink.
    #[inline]
    pub fn append(&mut self, value: LayoutUnit, is_quirky: bool) {
        if self.discard_margins {
            return;
        }

        if is_quirky && self.is_quirky_container_start {
            // Quirky containers ignore quirky margins at start.
            return;
        }

        if value.raw() > 0 {
            if is_quirky {
                if value > self.quirky_positive_margin {
                    self.quirky_positive_margin = value;
                }
            } else if value > self.positive_margin {
                self.positive_margin = value;
            }
        } else if value.raw() < 0 {
            if value < self.negative_margin {
                self.negative_margin = value;
            }
        }
    }

    /// Convenience: append a non-quirky margin.
    #[inline]
    pub fn append_normal(&mut self, value: LayoutUnit) {
        self.append(value, false);
    }

    /// The collapsed margin value = max(quirky_positive, positive) + negative.
    ///
    /// Returns zero if `discard_margins` is set.
    #[inline]
    pub fn sum(&self) -> LayoutUnit {
        if self.discard_margins {
            return LayoutUnit::zero();
        }
        let positive = if self.quirky_positive_margin > self.positive_margin {
            self.quirky_positive_margin
        } else {
            self.positive_margin
        };
        positive + self.negative_margin
    }

    /// Sum up non-quirky margins only. Used by quirky containers to compute
    /// the last margin (ignoring quirky contributions).
    #[inline]
    pub fn quirky_container_sum(&self) -> LayoutUnit {
        if self.discard_margins {
            return LayoutUnit::zero();
        }
        self.positive_margin + self.negative_margin
    }

    /// True if no margins have been accumulated.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.positive_margin.raw() == 0
            && self.negative_margin.raw() == 0
            && self.quirky_positive_margin.raw() == 0
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
        strut.append_normal(LayoutUnit::from_i32(10));
        strut.append_normal(LayoutUnit::from_i32(20));
        strut.append_normal(LayoutUnit::from_i32(5));
        assert_eq!(strut.sum(), LayoutUnit::from_i32(20));
    }

    #[test]
    fn negative_margins_take_most_negative() {
        let mut strut = MarginStrut::new();
        strut.append_normal(LayoutUnit::from_i32(-10));
        strut.append_normal(LayoutUnit::from_i32(-20));
        strut.append_normal(LayoutUnit::from_i32(-5));
        assert_eq!(strut.sum(), LayoutUnit::from_i32(-20));
    }

    #[test]
    fn mixed_margins_collapse() {
        let mut strut = MarginStrut::new();
        strut.append_normal(LayoutUnit::from_i32(30));
        strut.append_normal(LayoutUnit::from_i32(-10));
        // Collapsed = 30 + (-10) = 20
        assert_eq!(strut.sum(), LayoutUnit::from_i32(20));
    }

    #[test]
    fn quirky_margin_takes_largest() {
        let mut strut = MarginStrut::new();
        strut.append(LayoutUnit::from_i32(10), true); // quirky
        strut.append(LayoutUnit::from_i32(20), true); // quirky, larger
        assert_eq!(strut.quirky_positive_margin, LayoutUnit::from_i32(20));
        // Sum should use max(quirky, positive)
        assert_eq!(strut.sum(), LayoutUnit::from_i32(20));
    }

    #[test]
    fn quirky_vs_normal_takes_larger() {
        let mut strut = MarginStrut::new();
        strut.append(LayoutUnit::from_i32(15), true);  // quirky = 15
        strut.append_normal(LayoutUnit::from_i32(25));  // normal = 25
        // Sum = max(15, 25) + 0 = 25
        assert_eq!(strut.sum(), LayoutUnit::from_i32(25));

        let mut strut2 = MarginStrut::new();
        strut2.append(LayoutUnit::from_i32(30), true);  // quirky = 30
        strut2.append_normal(LayoutUnit::from_i32(10));  // normal = 10
        // Sum = max(30, 10) + 0 = 30
        assert_eq!(strut2.sum(), LayoutUnit::from_i32(30));
    }

    #[test]
    fn quirky_container_start_ignores_quirky() {
        let mut strut = MarginStrut::new();
        strut.is_quirky_container_start = true;
        strut.append(LayoutUnit::from_i32(20), true); // ignored
        strut.append_normal(LayoutUnit::from_i32(10)); // kept
        assert_eq!(strut.quirky_positive_margin, LayoutUnit::zero());
        assert_eq!(strut.sum(), LayoutUnit::from_i32(10));
    }

    #[test]
    fn quirky_container_sum_ignores_quirky() {
        let mut strut = MarginStrut::new();
        strut.append(LayoutUnit::from_i32(20), true);  // quirky
        strut.append_normal(LayoutUnit::from_i32(10));  // normal
        assert_eq!(strut.quirky_container_sum(), LayoutUnit::from_i32(10));
    }

    #[test]
    fn discard_margins_returns_zero() {
        let mut strut = MarginStrut::new();
        strut.append_normal(LayoutUnit::from_i32(30));
        strut.discard_margins = true;
        assert_eq!(strut.sum(), LayoutUnit::zero());
    }

    #[test]
    fn discard_margins_prevents_append() {
        let mut strut = MarginStrut::new();
        strut.discard_margins = true;
        strut.append_normal(LayoutUnit::from_i32(30));
        assert!(strut.is_empty());
    }

    #[test]
    fn trim_leading_flag_preserved() {
        let mut strut = MarginStrut::new();
        strut.trim_leading_margins = true;
        assert!(strut.trim_leading_margins);
    }
}
