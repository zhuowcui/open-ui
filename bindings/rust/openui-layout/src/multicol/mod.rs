//! CSS Multi-column Layout Module Level 1 — column container and fragmentainer
//! integration with the block layout and fragmentation system.
//!
//! Ported from Blink's `ColumnLayoutAlgorithm` (`column_layout_algorithm.cc`).
//!
//! Multi-column layout (`column-count`, `column-width`) creates multiple columns
//! within a block container. Content flows from one column to the next using the
//! fragmentation system (break tokens, fragmentainer space).
//!
//! Key concepts:
//! - **`ColumnLayoutAlgorithm`**: Main entry point. Resolves column count/width,
//!   dispatches content into fragmentainer columns, and optionally balances heights.
//! - **`resolve_column_count_and_width()`**: Per CSS Multicol §3-4, resolves the
//!   used column count and width from the specified values and available space.
//! - **`layout_columns()`**: Lays out content across columns using break tokens.
//! - **`balance_columns()`**: Binary search for the minimum balanced column height.

use openui_geometry::{LayoutUnit, PhysicalOffset, PhysicalSize};
use openui_dom::NodeId;
use openui_style::{BorderStyle, Color, ColumnFill, ComputedStyle};

use crate::fragment::Fragment;
use crate::fragmentation::FragmentainerSpace;

// ── Column Rule ─────────────────────────────────────────────────────────

/// Visual rule (divider line) drawn between adjacent columns.
///
/// Corresponds to `column-rule-width`, `column-rule-style`, `column-rule-color`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ColumnRule {
    pub width: LayoutUnit,
    pub style: BorderStyle,
    pub color: Color,
}

// ── Column Layout Algorithm ─────────────────────────────────────────────

/// Main entry point for multi-column layout.
///
/// Mirrors Blink's `ColumnLayoutAlgorithm`. Holds the resolved multicol
/// properties and drives the column fragmentation loop.
#[derive(Debug, Clone)]
pub struct ColumnLayoutAlgorithm {
    pub column_count: u32,
    pub column_width: Option<LayoutUnit>,
    pub column_gap: LayoutUnit,
    pub column_fill: ColumnFill,
    pub column_rule: Option<ColumnRule>,
}

/// Result of resolving column count and width from CSS properties and
/// available inline space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedColumns {
    /// The used column count.
    pub count: u32,
    /// The used column width.
    pub width: LayoutUnit,
}

/// Result of laying out content across columns.
#[derive(Debug)]
pub struct ColumnLayoutResult {
    /// The container fragment holding all column child fragments.
    pub fragment: Fragment,
    /// Column fragments positioned inside the container.
    pub column_fragments: Vec<Fragment>,
    /// Total block size (height) of the column container.
    pub block_size: LayoutUnit,
}

// ── Resolution: column-count and column-width (CSS Multicol §3-4) ───────

/// Resolve the used column count and column width from specified CSS values
/// and the available inline size.
///
/// Implements the algorithm from CSS Multicol §3-4:
/// - If both `column-count` and `column-width` are specified:
///   `actual_count = max(1, min(count, floor((available - gap) / (width + gap))))`
/// - If only `column-count`:
///   `width = (available - (count - 1) * gap) / count`
/// - If only `column-width`:
///   `count = max(1, floor((available + gap) / (width + gap)))`
/// - If neither: defaults to 1 column filling available space.
pub fn resolve_column_count_and_width(
    specified_count: Option<u32>,
    specified_width: Option<LayoutUnit>,
    available_inline_size: LayoutUnit,
    column_gap: LayoutUnit,
) -> ResolvedColumns {
    // Clamp available size to at least zero.
    let available = if available_inline_size.raw() > 0 {
        available_inline_size
    } else {
        LayoutUnit::zero()
    };

    match (specified_count, specified_width) {
        // Both count and width specified.
        (Some(count), Some(width)) => {
            let count = count.max(1);
            let width = clamp_column_width(width);

            // How many columns of this width fit?
            let width_plus_gap = width + column_gap;
            let fitting = if width_plus_gap.raw() > 0 {
                let avail_plus_gap = available + column_gap;
                let raw_fit = avail_plus_gap.raw() / width_plus_gap.raw();
                (raw_fit as u32).max(1)
            } else {
                count
            };

            let used_count = count.min(fitting);
            let used_width = compute_column_width(available, used_count, column_gap);
            ResolvedColumns { count: used_count, width: used_width }
        }

        // Only count specified; derive width.
        (Some(count), None) => {
            let count = count.max(1);
            let width = compute_column_width(available, count, column_gap);
            ResolvedColumns { count, width }
        }

        // Only width specified; derive count.
        (None, Some(width)) => {
            let width = clamp_column_width(width);
            let width_plus_gap = width + column_gap;
            let count = if width_plus_gap.raw() > 0 {
                let avail_plus_gap = available + column_gap;
                let raw_fit = avail_plus_gap.raw() / width_plus_gap.raw();
                (raw_fit as u32).max(1)
            } else {
                1
            };
            // Redistribute to fill available space.
            let used_width = compute_column_width(available, count, column_gap);
            ResolvedColumns { count, width: used_width }
        }

        // Neither specified: default to 1 column.
        (None, None) => {
            ResolvedColumns {
                count: 1,
                width: available,
            }
        }
    }
}

/// Compute column width given available space, count, and gap.
/// `width = (available - (count - 1) * gap) / count`
fn compute_column_width(available: LayoutUnit, count: u32, gap: LayoutUnit) -> LayoutUnit {
    if count <= 1 {
        return available;
    }
    let total_gaps = gap * LayoutUnit::from_i32((count - 1) as i32);
    let remaining = available - total_gaps;
    if remaining.raw() <= 0 {
        return LayoutUnit::zero();
    }
    LayoutUnit::from_raw(remaining.raw() / count as i32)
}

/// Clamp column-width to at least zero (negative values are invalid per spec).
fn clamp_column_width(width: LayoutUnit) -> LayoutUnit {
    if width.raw() < 0 {
        LayoutUnit::zero()
    } else {
        width
    }
}

// ── Column position computation ─────────────────────────────────────────

/// Computed inline position for each column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ColumnPosition {
    /// Inline offset of this column from the container's content edge.
    pub inline_offset: LayoutUnit,
    /// Width of this column.
    pub width: LayoutUnit,
}

/// Calculate the inline position of each column within the container.
///
/// When `center` is false (the normal case), the remaining space
/// (`available_inline_size − total_gaps`) is distributed across columns
/// using cumulative fractions so that every sub-pixel remainder is
/// accounted for.  Column *i* gets width
/// `⌊remaining*(i+1)/count⌋ − ⌊remaining*i/count⌋`.
///
/// When `center` is true, uniform `column_width` is used and the group
/// is centered within the available space.
pub fn compute_column_positions(
    column_count: u32,
    column_width: LayoutUnit,
    column_gap: LayoutUnit,
    available_inline_size: LayoutUnit,
    center: bool,
) -> Vec<ColumnPosition> {
    if column_count == 0 {
        return Vec::new();
    }

    if center {
        // Centering mode: uniform column_width, centered within available.
        let total_width = column_width * LayoutUnit::from_i32(column_count as i32)
            + column_gap * LayoutUnit::from_i32(column_count.saturating_sub(1) as i32);
        let start_offset = if total_width < available_inline_size {
            LayoutUnit::from_raw((available_inline_size - total_width).raw() / 2)
        } else {
            LayoutUnit::zero()
        };
        let stride = column_width + column_gap;
        return (0..column_count)
            .map(|i| ColumnPosition {
                inline_offset: start_offset + stride * LayoutUnit::from_i32(i as i32),
                width: column_width,
            })
            .collect();
    }

    // Non-centered: distribute remaining space using cumulative fractions
    // so that the column widths sum to exactly (available − total_gaps).
    let total_gaps = column_gap * LayoutUnit::from_i32(column_count.saturating_sub(1) as i32);
    let remaining_raw = (available_inline_size - total_gaps).raw().max(0) as i64;
    let n = column_count as i64;

    (0..column_count)
        .map(|i| {
            let i64_i = i as i64;
            let left_raw = (remaining_raw * i64_i / n) as i32;
            let right_raw = (remaining_raw * (i64_i + 1) / n) as i32;
            let w = right_raw - left_raw;
            let gap_offset = column_gap * LayoutUnit::from_i32(i as i32);
            ColumnPosition {
                inline_offset: LayoutUnit::from_raw(left_raw) + gap_offset,
                width: LayoutUnit::from_raw(w),
            }
        })
        .collect()
}

/// Compute the inline positions of column rules (dividers between columns).
///
/// Rules are centered in the gap between adjacent columns.
/// Returns the inline center offset for each rule (count - 1 rules).
pub fn compute_column_rule_positions(
    column_count: u32,
    column_width: LayoutUnit,
    column_gap: LayoutUnit,
) -> Vec<LayoutUnit> {
    if column_count <= 1 {
        return Vec::new();
    }
    let stride = column_width + column_gap;
    let half_gap = LayoutUnit::from_raw(column_gap.raw() / 2);
    (0..column_count - 1)
        .map(|i| {
            column_width + stride * LayoutUnit::from_i32(i as i32) + half_gap
        })
        .collect()
}

// ── Column layout (fragmentainer dispatch) ──────────────────────────────

/// Lay out content across multiple columns using the fragmentation system.
///
/// Creates a `FragmentainerSpace` for each column, simulates laying out
/// content by consuming block size from each column, and advances to the
/// next column when a break is needed.
///
/// `child_block_sizes` represents the block sizes of content blocks to be
/// distributed across columns. In a full implementation this would call
/// `block_layout()` with fragmentation constraints; here we simulate the
/// fragmentation loop.
///
/// Returns the container fragment with column children.
pub fn layout_columns(
    algo: &ColumnLayoutAlgorithm,
    node_id: NodeId,
    available_inline_size: LayoutUnit,
    available_block_size: LayoutUnit,
    child_block_sizes: &[LayoutUnit],
) -> ColumnLayoutResult {
    let resolved = resolve_column_count_and_width(
        if algo.column_count > 0 { Some(algo.column_count) } else { None },
        algo.column_width,
        available_inline_size,
        algo.column_gap,
    );

    let column_height = match algo.column_fill {
        ColumnFill::Balance | ColumnFill::BalanceAll => {
            balance_columns(child_block_sizes, &vec![false; child_block_sizes.len()], resolved.count, available_block_size, &vec![false; child_block_sizes.len()])
        }
        ColumnFill::Auto => {
            if available_block_size.raw() > 0 && available_block_size.raw() < i32::MAX / 2 {
                available_block_size
            } else {
                // Fallback: sum of all content (single tall column).
                let total: i32 = child_block_sizes.iter().map(|s| s.raw()).sum();
                LayoutUnit::from_raw(total)
            }
        }
    };

    // Distribute children across columns using fragmentainer simulation.
    let positions = compute_column_positions(
        resolved.count,
        resolved.width,
        algo.column_gap,
        available_inline_size,
        false,
    );

    let mut column_fragments: Vec<Fragment> = Vec::new();
    let mut col_idx: usize = 0;
    let mut space = FragmentainerSpace::new(column_height);
    let mut child_iter = child_block_sizes.iter().peekable();
    let mut current_col_children: Vec<LayoutUnit> = Vec::new();

    while let Some(&&child_size) = child_iter.peek() {
        if col_idx >= positions.len() {
            // Overflow: more content than columns. Place remaining in last column.
            current_col_children.push(child_size);
            child_iter.next();
            continue;
        }

        if space.remaining().raw() >= child_size.raw() {
            // Child fits in the current column.
            space.consume(child_size);
            current_col_children.push(child_size);
            child_iter.next();
        } else {
            // Column is full — finalize this column and move to next.
            let col_frag = make_column_fragment(
                node_id,
                &positions[col_idx],
                column_height,
                &current_col_children,
            );
            column_fragments.push(col_frag);
            current_col_children.clear();
            col_idx += 1;
            space = FragmentainerSpace::new(column_height);
        }
    }

    // Finalize the last (or only) column.
    if !current_col_children.is_empty() || column_fragments.is_empty() {
        let pos_idx = col_idx.min(positions.len().saturating_sub(1));
        let pos = positions.get(pos_idx).copied().unwrap_or(ColumnPosition {
            inline_offset: LayoutUnit::zero(),
            width: resolved.width,
        });
        let col_frag = make_column_fragment(node_id, &pos, column_height, &current_col_children);
        column_fragments.push(col_frag);
    }

    // Build the container fragment.
    let total_inline = available_inline_size;
    let container_size = PhysicalSize::new(total_inline, column_height);
    let mut container = Fragment::new_box(node_id, container_size);

    // Re-create column fragments as children of the container.
    for cf in &column_fragments {
        let mut child = Fragment::new_box(cf.node_id, cf.size);
        child.offset = cf.offset;
        container.children.push(child);
    }

    let result_fragments: Vec<Fragment> = column_fragments;

    ColumnLayoutResult {
        fragment: container,
        column_fragments: result_fragments,
        block_size: column_height,
    }
}

/// Create a fragment representing a single column.
fn make_column_fragment(
    node_id: NodeId,
    position: &ColumnPosition,
    height: LayoutUnit,
    _child_sizes: &[LayoutUnit],
) -> Fragment {
    let size = PhysicalSize::new(position.width, height);
    let mut frag = Fragment::new_box(node_id, size);
    frag.offset = PhysicalOffset::new(position.inline_offset, LayoutUnit::zero());
    frag
}

// ── Column balancing ────────────────────────────────────────────────────

/// Binary search for the minimum column height that fits all content in the
/// given number of columns.
///
/// Algorithm:
/// 1. Start with `total_height / column_count` as initial guess.
/// 2. Iterate: if content overflows (needs more columns), increase height.
///    If it fits with room to spare, decrease height.
/// 3. Max 10 iterations for convergence guarantee.
///
/// `forced_break_before` indicates children that must start in a new column
/// due to `break-before: column` or the previous child's `break-after: column`.
/// Per CSS Fragmentation §3.4, a forced break on the very first child is
/// ignored.
///
/// Returns the balanced column height.
pub fn balance_columns(
    child_block_sizes: &[LayoutUnit],
    avoid_break_inside: &[bool],
    column_count: u32,
    max_height: LayoutUnit,
    forced_break_before: &[bool],
) -> LayoutUnit {
    if column_count <= 1 || child_block_sizes.is_empty() {
        let total: i32 = child_block_sizes.iter().map(|s| s.raw()).sum();
        return LayoutUnit::from_raw(total);
    }

    let total_raw: i64 = child_block_sizes.iter().map(|s| s.raw() as i64).sum();

    // The minimum column height must accommodate any unsplittable child.
    let max_unsplittable = child_block_sizes.iter().zip(avoid_break_inside.iter())
        .filter(|(_, &avoid)| avoid)
        .map(|(s, _)| s.raw() as i64)
        .max()
        .unwrap_or(0);

    // Count how many forced breaks exist (excluding the first child).
    // Each forced break consumes one column boundary, reducing the number
    // of columns available for content distribution.
    let forced_count = forced_break_before.iter().enumerate()
        .filter(|&(i, &f)| f && i > 0)
        .count() as u32;
    // The effective column count for content distribution is reduced by
    // forced breaks (each forced break uses a column boundary), but at
    // least 1 column per forced-break segment.
    let effective_columns = column_count.max(forced_count + 1);

    // Compute the tallest forced-break segment: content between consecutive
    // forced breaks that must all fit in a single column group.
    let mut max_segment: i64 = 0;
    let mut cur_segment: i64 = 0;
    for (i, &size) in child_block_sizes.iter().enumerate() {
        let has_forced = forced_break_before.get(i).copied().unwrap_or(false) && i > 0;
        if has_forced {
            max_segment = max_segment.max(cur_segment);
            cur_segment = 0;
        }
        cur_segment += size.raw() as i64;
    }
    max_segment = max_segment.max(cur_segment);

    // With fragmentation, children can be split at column boundaries.
    // The minimum possible column height is ceil(total / count),
    // but also at least as tall as the tallest unsplittable child,
    // and at least ceil(max_segment / columns_in_that_segment).
    let min_raw = ((total_raw + column_count as i64 - 1) / column_count as i64)
        .max(max_unsplittable)
        .max((max_segment + effective_columns as i64 - 1) / effective_columns as i64) as i32;

    let mut lo = min_raw;
    let mut hi = if max_height.raw() > 0 && max_height.raw() < i32::MAX / 2 {
        total_raw.min(max_height.raw() as i64) as i32
    } else {
        total_raw.min(i32::MAX as i64 / 2) as i32
    };

    if lo > hi {
        hi = lo;
    }

    const MAX_ITERATIONS: u32 = 32;

    for _ in 0..MAX_ITERATIONS {
        if lo >= hi {
            break;
        }
        let mid = lo + (hi - lo) / 2;
        let needed = columns_needed_for_height(child_block_sizes, avoid_break_inside, LayoutUnit::from_raw(mid), forced_break_before);
        if needed <= column_count {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }

    LayoutUnit::from_raw(lo)
}

/// Count how many columns are needed to fit all children at the given height.
/// Children with break-inside: avoid are treated as unsplittable units.
/// Children with forced_break_before always start a new column (except the first).
fn columns_needed_for_height(
    child_block_sizes: &[LayoutUnit],
    avoid_break_inside: &[bool],
    height: LayoutUnit,
    forced_break_before: &[bool],
) -> u32 {
    if height.raw() <= 0 {
        return u32::MAX;
    }
    let mut columns = 1u32;
    let mut remaining = height;

    for (i, &child_size) in child_block_sizes.iter().enumerate() {
        // CSS Fragmentation §3.1: forced break-before always starts a new
        // column (ignored on the very first child per §3.4).
        let has_forced = forced_break_before.get(i).copied().unwrap_or(false) && i > 0;
        if has_forced {
            columns += 1;
            remaining = height;
        }

        let avoid = avoid_break_inside.get(i).copied().unwrap_or(false);
        if avoid {
            // Child cannot be split. If it doesn't fit in remaining space
            // (and there's already content), move to next column.
            if child_size.raw() > remaining.raw() && remaining < height {
                columns += 1;
                remaining = height;
            }
            // Place the whole child (even if it overflows the column)
            remaining = remaining - child_size;
            if remaining.raw() < 0 {
                remaining = LayoutUnit::zero();
            }
        } else {
            let mut left = child_size;
            while left.raw() > remaining.raw() {
                left = left - remaining;
                columns += 1;
                remaining = height;
            }
            remaining = remaining - left;
        }
    }

    columns
}

// ── Convenience: construct from ComputedStyle ───────────────────────────

impl ColumnLayoutAlgorithm {
    /// Build a `ColumnLayoutAlgorithm` from a `ComputedStyle`.
    pub fn from_style(style: &ComputedStyle) -> Option<Self> {
        // A multicol container must have column-count or column-width set.
        // CSS Multicol §3: column-count values less than 1 are invalid;
        // treat Some(0) the same as None (auto).
        let effective_count = style.column_count.filter(|&c| c >= 1);
        if effective_count.is_none() && style.column_width.is_none() {
            return None;
        }

        let column_gap = match style.column_gap {
            Some(ref len) if len.is_fixed() => LayoutUnit::from_f32(len.value()),
            Some(ref len) if len.is_percent() => {
                // Percentage column-gap resolved later against available width
                // For now store the percentage value; resolve_columns handles it.
                // Actually, percentage is against the content box of the multicol.
                // We don't have that here yet, so use 0 as placeholder.
                LayoutUnit::zero()
            }
            _ => LayoutUnit::from_f32(style.font_size), // CSS default `normal` = 1em
        };

        let column_rule = if style.column_rule_style.has_visible_border() {
            Some(ColumnRule {
                width: LayoutUnit::from_i32(style.column_rule_width),
                style: style.column_rule_style,
                color: match style.column_rule_color {
                    openui_style::StyleColor::Resolved(c) => c,
                    _ => style.color,
                },
            })
        } else {
            None
        };

        Some(Self {
            column_count: effective_count.unwrap_or(0),
            column_width: style.column_width.as_ref().and_then(|l| {
                if l.is_fixed() { Some(LayoutUnit::from_f32(l.value())) } else { None }
            }),
            column_gap,
            column_fill: style.column_fill,
            column_rule,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_only_count() {
        let r = resolve_column_count_and_width(
            Some(3), None,
            LayoutUnit::from_i32(900),
            LayoutUnit::from_i32(20),
        );
        assert_eq!(r.count, 3);
        // width = (900 - 2*20) / 3 = 860 / 3 ≈ 286
        let expected = LayoutUnit::from_raw(
            (LayoutUnit::from_i32(900) - LayoutUnit::from_i32(40)).raw() / 3,
        );
        assert_eq!(r.width, expected);
    }

    #[test]
    fn resolve_only_width() {
        let r = resolve_column_count_and_width(
            None, Some(LayoutUnit::from_i32(200)),
            LayoutUnit::from_i32(900),
            LayoutUnit::from_i32(20),
        );
        // count = floor((900+20)/(200+20)) = floor(920/220) = floor(4.18) = 4
        assert_eq!(r.count, 4);
    }

    #[test]
    fn resolve_auto_auto() {
        let r = resolve_column_count_and_width(
            None, None,
            LayoutUnit::from_i32(600),
            LayoutUnit::from_i32(10),
        );
        assert_eq!(r.count, 1);
        assert_eq!(r.width, LayoutUnit::from_i32(600));
    }

    #[test]
    fn balance_equal_content() {
        let children = vec![
            LayoutUnit::from_i32(100),
            LayoutUnit::from_i32(100),
            LayoutUnit::from_i32(100),
        ];
        let h = balance_columns(&children, &vec![false; 3], 3, LayoutUnit::from_i32(1000), &vec![false; 3]);
        assert_eq!(h, LayoutUnit::from_i32(100));
    }

    #[test]
    fn balance_uneven_content() {
        let children = vec![
            LayoutUnit::from_i32(50),
            LayoutUnit::from_i32(80),
            LayoutUnit::from_i32(60),
            LayoutUnit::from_i32(40),
        ];
        let h = balance_columns(&children, &vec![false; 4], 2, LayoutUnit::from_i32(1000), &vec![false; 4]);
        // Total = 230, 2 cols. With fragmentation:
        // col1 = 50 + 65 (first part of 80) = 115
        // col2 = 15 (remainder of 80) + 60 + 40 = 115
        assert_eq!(h, LayoutUnit::from_i32(115));
    }
}
