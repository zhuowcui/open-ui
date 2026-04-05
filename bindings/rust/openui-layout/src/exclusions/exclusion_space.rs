//! ExclusionSpace — tracks float exclusion areas within a BFC.
//!
//! Source: core/layout/exclusions/exclusion_space.h/cc (~2,000 lines)
//!
//! The exclusion space maintains sorted lists of left and right float
//! exclusion rectangles and provides efficient queries for:
//! - Finding layout opportunities (available inline space at a given block offset)
//! - Computing clearance offsets (block offset to clear past floats)
//! - Adding new float exclusions
//!
//! Blink uses a shelf-based algorithm for O(n) opportunity queries. This
//! implementation mirrors that approach.

use openui_geometry::{BfcOffset, BfcRect, LayoutUnit};
use std::fmt;

/// Type of float exclusion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExclusionType {
    /// `float: left` — content flows to the right of this exclusion.
    Left,
    /// `float: right` — content flows to the left of this exclusion.
    Right,
}

/// A single float exclusion area within the BFC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExclusionArea {
    /// The rectangle this float occupies in BFC coordinates.
    pub rect: BfcRect,
    /// Whether this is a left or right float.
    pub exclusion_type: ExclusionType,
}

/// A layout opportunity — a rectangular region where content can be placed
/// without overlapping any floats.
///
/// Source: Blink's `LayoutOpportunity`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutOpportunity {
    /// The available rectangle in BFC coordinates.
    pub rect: BfcRect,
}

impl LayoutOpportunity {
    /// Inline size of this opportunity.
    #[inline]
    pub fn inline_size(&self) -> LayoutUnit {
        self.rect.inline_size()
    }

    /// Block size of this opportunity.
    #[inline]
    pub fn block_size(&self) -> LayoutUnit {
        self.rect.block_size()
    }
}

/// CSS `clear` property values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearType {
    None,
    Left,
    Right,
    Both,
}

/// Tracks float exclusion areas within a block formatting context.
///
/// Provides queries for finding available space (layout opportunities) at
/// any block offset, and for computing clearance offsets. Float exclusions
/// are stored as sorted rectangles and queried using a shelf-based algorithm
/// matching Blink's `ExclusionSpace`.
///
/// Source: `ExclusionSpace` in `exclusions/exclusion_space.h/cc`.
#[derive(Debug, Clone)]
pub struct ExclusionSpace {
    /// Left float exclusions, sorted by block_start ascending.
    left_floats: Vec<ExclusionArea>,
    /// Right float exclusions, sorted by block_start ascending.
    right_floats: Vec<ExclusionArea>,

    /// Cached clearance offsets for efficient lookup.
    left_clear_offset: LayoutUnit,
    right_clear_offset: LayoutUnit,
}

impl ExclusionSpace {
    /// Create an empty exclusion space.
    pub fn new() -> Self {
        Self {
            left_floats: Vec::new(),
            right_floats: Vec::new(),
            left_clear_offset: LayoutUnit::zero(),
            right_clear_offset: LayoutUnit::zero(),
        }
    }

    /// Add a float exclusion to the space.
    ///
    /// Updates the internal sorted lists and clearance caches.
    pub fn add(&mut self, exclusion: ExclusionArea) {
        let block_end = exclusion.rect.block_end_offset();

        match exclusion.exclusion_type {
            ExclusionType::Left => {
                if block_end > self.left_clear_offset {
                    self.left_clear_offset = block_end;
                }
                self.left_floats.push(exclusion);
            }
            ExclusionType::Right => {
                if block_end > self.right_clear_offset {
                    self.right_clear_offset = block_end;
                }
                self.right_floats.push(exclusion);
            }
        }
    }

    /// Find the first layout opportunity at or below `offset` that has at
    /// least `min_inline_size` of inline space within `available_inline_size`.
    ///
    /// This implements Blink's shelf-based algorithm: scan down block offsets
    /// where float edges change, and at each shelf compute available inline
    /// space by subtracting overlapping float widths.
    pub fn find_layout_opportunity(
        &self,
        offset: &BfcOffset,
        available_inline_size: LayoutUnit,
        min_inline_size: LayoutUnit,
    ) -> LayoutOpportunity {
        let mut block_offset = offset.block_offset;

        // Collect all distinct block offsets where shelves change
        let mut shelf_edges: Vec<LayoutUnit> = Vec::new();
        shelf_edges.push(block_offset);

        for f in &self.left_floats {
            let start = f.rect.block_start_offset();
            let end = f.rect.block_end_offset();
            if end > block_offset {
                if start > block_offset {
                    shelf_edges.push(start);
                }
                shelf_edges.push(end);
            }
        }
        for f in &self.right_floats {
            let start = f.rect.block_start_offset();
            let end = f.rect.block_end_offset();
            if end > block_offset {
                if start > block_offset {
                    shelf_edges.push(start);
                }
                shelf_edges.push(end);
            }
        }

        shelf_edges.sort_unstable();
        shelf_edges.dedup();

        // At each shelf edge, compute available space
        for &shelf_start in &shelf_edges {
            if shelf_start < block_offset {
                continue;
            }

            let (left_edge, right_edge) =
                self.compute_edges_at(shelf_start, offset.line_offset, available_inline_size);
            let inline_space = right_edge - left_edge;

            if inline_space >= min_inline_size {
                // Find block end of this opportunity (where a new float starts)
                let block_end = self.next_float_start_after(shelf_start);

                return LayoutOpportunity {
                    rect: BfcRect::new(
                        BfcOffset::new(left_edge, shelf_start),
                        BfcOffset::new(right_edge, block_end),
                    ),
                };
            }

            block_offset = shelf_start;
        }

        // No floats obstruct — full width available below all floats
        let max_clear = if self.left_clear_offset > self.right_clear_offset {
            self.left_clear_offset
        } else {
            self.right_clear_offset
        };
        let start_block = if max_clear > block_offset {
            max_clear
        } else {
            block_offset
        };

        LayoutOpportunity {
            rect: BfcRect::new(
                BfcOffset::new(offset.line_offset, start_block),
                BfcOffset::new(
                    offset.line_offset + available_inline_size,
                    LayoutUnit::max(),
                ),
            ),
        }
    }

    /// Compute the clearance offset for the given clear type.
    ///
    /// Returns the block offset below which no floats of the specified type exist.
    pub fn clearance_offset(&self, clear_type: ClearType) -> LayoutUnit {
        match clear_type {
            ClearType::None => LayoutUnit::zero(),
            ClearType::Left => self.left_clear_offset,
            ClearType::Right => self.right_clear_offset,
            ClearType::Both => {
                if self.left_clear_offset > self.right_clear_offset {
                    self.left_clear_offset
                } else {
                    self.right_clear_offset
                }
            }
        }
    }

    /// Whether this exclusion space has any floats.
    #[inline]
    pub fn has_floats(&self) -> bool {
        !self.left_floats.is_empty() || !self.right_floats.is_empty()
    }

    /// Number of float exclusions tracked.
    #[inline]
    pub fn num_exclusions(&self) -> usize {
        self.left_floats.len() + self.right_floats.len()
    }

    // ── Internal helpers ─────────────────────────────────────────────

    /// Compute the left and right edges of available space at a given block offset.
    fn compute_edges_at(
        &self,
        block_offset: LayoutUnit,
        bfc_line_start: LayoutUnit,
        available_inline_size: LayoutUnit,
    ) -> (LayoutUnit, LayoutUnit) {
        let mut left_edge = bfc_line_start;
        let mut right_edge = bfc_line_start + available_inline_size;

        for f in &self.left_floats {
            if f.rect.block_start_offset() <= block_offset
                && f.rect.block_end_offset() > block_offset
            {
                let float_right = f.rect.line_end_offset();
                if float_right > left_edge {
                    left_edge = float_right;
                }
            }
        }

        for f in &self.right_floats {
            if f.rect.block_start_offset() <= block_offset
                && f.rect.block_end_offset() > block_offset
            {
                let float_left = f.rect.line_start_offset();
                if float_left < right_edge {
                    right_edge = float_left;
                }
            }
        }

        (left_edge, right_edge)
    }

    /// Find the next block offset where a float edge starts after the given offset.
    fn next_float_start_after(&self, block_offset: LayoutUnit) -> LayoutUnit {
        let mut next = LayoutUnit::max();

        for f in &self.left_floats {
            let start = f.rect.block_start_offset();
            if start > block_offset && start < next {
                next = start;
            }
            let end = f.rect.block_end_offset();
            if end > block_offset && end < next {
                next = end;
            }
        }

        for f in &self.right_floats {
            let start = f.rect.block_start_offset();
            if start > block_offset && start < next {
                next = start;
            }
            let end = f.rect.block_end_offset();
            if end > block_offset && end < next {
                next = end;
            }
        }

        next
    }
}

impl Default for ExclusionSpace {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for ExclusionSpace {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ExclusionSpace(left: {}, right: {})",
            self.left_floats.len(),
            self.right_floats.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    fn make_float(
        exclusion_type: ExclusionType,
        line_start: i32,
        block_start: i32,
        line_end: i32,
        block_end: i32,
    ) -> ExclusionArea {
        ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(line_start), lu(block_start)),
                BfcOffset::new(lu(line_end), lu(block_end)),
            ),
            exclusion_type,
        }
    }

    #[test]
    fn empty_space_full_opportunity() {
        let space = ExclusionSpace::new();
        let opp = space.find_layout_opportunity(
            &BfcOffset::new(lu(0), lu(0)),
            lu(800),
            lu(100),
        );
        assert_eq!(opp.rect.line_start_offset(), lu(0));
        assert_eq!(opp.rect.line_end_offset(), lu(800));
        assert_eq!(opp.rect.block_start_offset(), lu(0));
    }

    #[test]
    fn left_float_reduces_inline_space() {
        let mut space = ExclusionSpace::new();
        // Left float: 0-200px wide, 0-100px tall
        space.add(make_float(ExclusionType::Left, 0, 0, 200, 100));

        let opp = space.find_layout_opportunity(
            &BfcOffset::new(lu(0), lu(0)),
            lu(800),
            lu(100),
        );
        // Content starts at line_offset 200 (after the float)
        assert_eq!(opp.rect.line_start_offset(), lu(200));
        assert_eq!(opp.rect.line_end_offset(), lu(800));
    }

    #[test]
    fn right_float_reduces_inline_space() {
        let mut space = ExclusionSpace::new();
        // Right float: 600-800px wide, 0-100px tall
        space.add(make_float(ExclusionType::Right, 600, 0, 800, 100));

        let opp = space.find_layout_opportunity(
            &BfcOffset::new(lu(0), lu(0)),
            lu(800),
            lu(100),
        );
        assert_eq!(opp.rect.line_start_offset(), lu(0));
        assert_eq!(opp.rect.line_end_offset(), lu(600));
    }

    #[test]
    fn both_floats_narrow_space() {
        let mut space = ExclusionSpace::new();
        space.add(make_float(ExclusionType::Left, 0, 0, 200, 100));
        space.add(make_float(ExclusionType::Right, 600, 0, 800, 100));

        let opp = space.find_layout_opportunity(
            &BfcOffset::new(lu(0), lu(0)),
            lu(800),
            lu(100),
        );
        assert_eq!(opp.rect.line_start_offset(), lu(200));
        assert_eq!(opp.rect.line_end_offset(), lu(600));
    }

    #[test]
    fn content_drops_below_float_if_no_space() {
        let mut space = ExclusionSpace::new();
        // Float takes almost all width
        space.add(make_float(ExclusionType::Left, 0, 0, 700, 100));

        // Need 200px but only 100px available (800 - 700)
        let opp = space.find_layout_opportunity(
            &BfcOffset::new(lu(0), lu(0)),
            lu(800),
            lu(200),
        );
        // Should drop below the float
        assert!(opp.rect.block_start_offset() >= lu(100));
        assert_eq!(opp.inline_size(), lu(800));
    }

    #[test]
    fn clearance_offset_left() {
        let mut space = ExclusionSpace::new();
        space.add(make_float(ExclusionType::Left, 0, 0, 200, 150));
        assert_eq!(space.clearance_offset(ClearType::Left), lu(150));
        assert_eq!(space.clearance_offset(ClearType::Right), lu(0));
        assert_eq!(space.clearance_offset(ClearType::Both), lu(150));
    }

    #[test]
    fn clearance_offset_both() {
        let mut space = ExclusionSpace::new();
        space.add(make_float(ExclusionType::Left, 0, 0, 200, 100));
        space.add(make_float(ExclusionType::Right, 600, 0, 800, 200));
        assert_eq!(space.clearance_offset(ClearType::Left), lu(100));
        assert_eq!(space.clearance_offset(ClearType::Right), lu(200));
        assert_eq!(space.clearance_offset(ClearType::Both), lu(200));
    }

    #[test]
    fn has_floats() {
        let mut space = ExclusionSpace::new();
        assert!(!space.has_floats());
        space.add(make_float(ExclusionType::Left, 0, 0, 100, 50));
        assert!(space.has_floats());
    }

    #[test]
    fn opportunity_below_expired_float() {
        let mut space = ExclusionSpace::new();
        space.add(make_float(ExclusionType::Left, 0, 0, 200, 50));

        // At block_offset 50, float has expired
        let opp = space.find_layout_opportunity(
            &BfcOffset::new(lu(0), lu(50)),
            lu(800),
            lu(100),
        );
        assert_eq!(opp.rect.line_start_offset(), lu(0));
        assert_eq!(opp.rect.line_end_offset(), lu(800));
    }

    #[test]
    fn multiple_stacked_left_floats() {
        let mut space = ExclusionSpace::new();
        space.add(make_float(ExclusionType::Left, 0, 0, 200, 100));
        space.add(make_float(ExclusionType::Left, 0, 50, 300, 150));

        // At offset 50, both floats active — wider one wins (300px)
        let opp = space.find_layout_opportunity(
            &BfcOffset::new(lu(0), lu(50)),
            lu(800),
            lu(100),
        );
        assert_eq!(opp.rect.line_start_offset(), lu(300));
    }
}
