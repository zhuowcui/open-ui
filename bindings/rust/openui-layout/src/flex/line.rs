//! FlexLine — per-line data during flex layout.
//!
//! Extracted from Blink's `FlexLine` (core/layout/flex/flex_line.h:64-112).
//! Stores the items on a line, the resolved cross size, baselines, and offsets.

use openui_geometry::LayoutUnit;

/// A single flex line containing one or more flex items.
///
/// Mirrors Blink's `FlexLine` struct. During layout, items are broken into
/// lines by the line breaker, then each line is flexed independently.
#[derive(Debug, Clone)]
pub struct FlexLine {
    /// Indices into the parent `flex_items` vector for items on this line.
    /// Blink: `item_indices` (Vector<wtf_size_t>).
    pub item_indices: Vec<usize>,

    /// Remaining free space on the main axis after flexing.
    /// Positive = extra space for justify-content distribution.
    /// Negative = overflow.
    /// Blink: `main_axis_free_space`.
    pub main_axis_free_space: LayoutUnit,

    /// Cross-axis size of this line (max of all item cross-margin-box sizes,
    /// or container cross size for single-line).
    /// Blink: `line_cross_size`.
    pub line_cross_size: LayoutUnit,

    /// Maximum ascent among major-baseline-aligned items.
    /// Blink: `major_baseline`.
    pub major_baseline: LayoutUnit,

    /// Maximum ascent among minor-baseline-aligned items.
    /// Blink: `minor_baseline`.
    pub minor_baseline: LayoutUnit,

    /// Total number of auto margins on the main axis across all items.
    /// Blink: `main_axis_auto_margin_count`.
    pub main_axis_auto_margin_count: u32,

    /// Used main-axis size of this line (sum of all items' flexed margin-box sizes).
    /// Computed after flexing. Used for column flex intrinsic block size.
    pub main_axis_used_size: LayoutUnit,

    /// Cross-axis offset of this line within the flex container.
    /// Set during align-content resolution.
    /// Blink: `cross_axis_offset`.
    pub cross_axis_offset: LayoutUnit,
}

impl FlexLine {
    /// Create a new flex line with the given item indices.
    pub fn new(item_indices: Vec<usize>) -> Self {
        Self {
            item_indices,
            main_axis_free_space: LayoutUnit::zero(),
            line_cross_size: LayoutUnit::zero(),
            major_baseline: LayoutUnit::zero(),
            minor_baseline: LayoutUnit::zero(),
            main_axis_auto_margin_count: 0,
            main_axis_used_size: LayoutUnit::zero(),
            cross_axis_offset: LayoutUnit::zero(),
        }
    }

    /// Number of items on this line.
    #[inline]
    pub fn item_count(&self) -> usize {
        self.item_indices.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_line() {
        let line = FlexLine::new(vec![0, 1, 2]);
        assert_eq!(line.item_count(), 3);
        assert_eq!(line.main_axis_free_space, LayoutUnit::zero());
        assert_eq!(line.line_cross_size, LayoutUnit::zero());
    }
}
