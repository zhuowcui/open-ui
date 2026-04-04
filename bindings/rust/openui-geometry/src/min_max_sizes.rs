//! MinMaxSizes — min-content and max-content size pair.
//!
//! Extracted from Blink's `MinMaxSizes` (core/layout/geometry/min_max_sizes.h).
//! Used for intrinsic sizing and flex item min/max constraints.

use crate::LayoutUnit;

/// A pair of min-content and max-content sizes.
///
/// In Blink, this is `MinMaxSizes` with `min_size` and `max_size` fields.
/// Used for intrinsic sizing queries and flex item min/max clamping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MinMaxSizes {
    pub min: LayoutUnit,
    pub max: LayoutUnit,
}

impl MinMaxSizes {
    pub fn new(min: LayoutUnit, max: LayoutUnit) -> Self {
        Self { min, max }
    }

    pub fn zero() -> Self {
        Self {
            min: LayoutUnit::zero(),
            max: LayoutUnit::zero(),
        }
    }

    /// Clamp a value to [min, max]. Matches Blink's `Encompass` + `ShrinkTo` pattern.
    #[inline]
    pub fn clamp(&self, value: LayoutUnit) -> LayoutUnit {
        value.clamp(self.min, self.max)
    }

    /// Ensure both min and max are at least `lower_bound`.
    #[inline]
    pub fn encompass(&mut self, lower_bound: LayoutUnit) {
        self.min = self.min.max_of(lower_bound);
        self.max = self.max.max_of(lower_bound);
    }

    /// Ensure max is at most `upper_bound`, then ensure min <= max.
    #[inline]
    pub fn shrink_to(&mut self, upper_bound: LayoutUnit) {
        self.max = self.max.min_of(upper_bound);
        self.min = self.min.min_of(self.max);
    }

    /// Add a fixed amount to both min and max.
    #[inline]
    pub fn add(&mut self, amount: LayoutUnit) {
        self.min = self.min + amount;
        self.max = self.max + amount;
    }
}

impl Default for MinMaxSizes {
    fn default() -> Self { Self::zero() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_within_range() {
        let mm = MinMaxSizes::new(LayoutUnit::from_i32(50), LayoutUnit::from_i32(200));
        assert_eq!(mm.clamp(LayoutUnit::from_i32(100)), LayoutUnit::from_i32(100));
        assert_eq!(mm.clamp(LayoutUnit::from_i32(30)), LayoutUnit::from_i32(50));
        assert_eq!(mm.clamp(LayoutUnit::from_i32(300)), LayoutUnit::from_i32(200));
    }

    #[test]
    fn encompass_raises_both() {
        let mut mm = MinMaxSizes::new(LayoutUnit::from_i32(10), LayoutUnit::from_i32(50));
        mm.encompass(LayoutUnit::from_i32(30));
        assert_eq!(mm.min, LayoutUnit::from_i32(30));
        assert_eq!(mm.max, LayoutUnit::from_i32(50));
    }

    #[test]
    fn shrink_to_lowers_both() {
        let mut mm = MinMaxSizes::new(LayoutUnit::from_i32(100), LayoutUnit::from_i32(200));
        mm.shrink_to(LayoutUnit::from_i32(150));
        assert_eq!(mm.min, LayoutUnit::from_i32(100));
        assert_eq!(mm.max, LayoutUnit::from_i32(150));

        mm.shrink_to(LayoutUnit::from_i32(80));
        assert_eq!(mm.min, LayoutUnit::from_i32(80));
        assert_eq!(mm.max, LayoutUnit::from_i32(80));
    }

    #[test]
    fn add_to_both() {
        let mut mm = MinMaxSizes::new(LayoutUnit::from_i32(50), LayoutUnit::from_i32(100));
        mm.add(LayoutUnit::from_i32(20));
        assert_eq!(mm.min, LayoutUnit::from_i32(70));
        assert_eq!(mm.max, LayoutUnit::from_i32(120));
    }
}
