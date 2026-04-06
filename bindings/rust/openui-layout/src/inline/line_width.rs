//! Per-line available width computation with float exclusion awareness.
//!
//! Mirrors Blink's `LineWidth` from
//! `third_party/blink/renderer/core/layout/inline/line_width.cc`.
//!
//! In CSS 2.1 §9.5.1, content flows alongside floats. Each line box in an
//! inline formatting context may have a different available width depending
//! on which floats overlap at that line's block offset. This module queries
//! the `ExclusionSpace` to compute the available inline size for each line.

use openui_geometry::{BfcOffset, LayoutUnit};

use crate::exclusions::ExclusionSpace;

/// Per-line available width result from an exclusion space query.
///
/// Contains the inline-start offset (where content begins, shifted right
/// by left floats) and the available inline size (narrowed by both left
/// and right floats).
#[derive(Debug, Clone, Copy)]
pub struct LineAvailability {
    /// Inline-start offset relative to the content edge of the container.
    /// Non-zero when left floats intrude into the content area.
    pub inline_start: LayoutUnit,

    /// Available inline size for content on this line.
    /// May be less than the container's full inline size when floats intrude.
    pub available_inline_size: LayoutUnit,
}

/// Compute the available inline size for a line at the given block offset.
///
/// Queries the `ExclusionSpace` to find the first layout opportunity at
/// `line_block_offset` (in content-edge-relative coordinates) with at least
/// `min_inline_size` of space.
///
/// If no exclusion space is provided (no floats), returns the full
/// `container_inline_size` with zero inline-start offset.
///
/// # Parameters
///
/// - `exclusion_space`: The float exclusion state, or `None` if no floats exist.
/// - `line_block_offset`: Block offset of this line relative to the content
///   edge of the inline formatting context's containing block.
/// - `container_inline_size`: Full inline size of the containing block's
///   content area.
/// - `min_inline_size`: Minimum inline space required (typically zero for
///   text, or the atomic inline's width).
///
/// # Returns
///
/// A `LineAvailability` with the inline-start offset and available width.
pub fn compute_line_availability(
    exclusion_space: Option<&ExclusionSpace>,
    line_block_offset: LayoutUnit,
    container_inline_size: LayoutUnit,
    min_inline_size: LayoutUnit,
) -> LineAvailability {
    let es = match exclusion_space {
        Some(es) if es.has_floats() => es,
        _ => {
            return LineAvailability {
                inline_start: LayoutUnit::zero(),
                available_inline_size: container_inline_size,
            };
        }
    };

    let opportunity = es.find_layout_opportunity(
        &BfcOffset::new(LayoutUnit::zero(), line_block_offset),
        container_inline_size,
        min_inline_size,
    );

    let inline_start = opportunity.rect.line_start_offset();
    let available = opportunity.inline_size();

    LineAvailability {
        inline_start,
        available_inline_size: available,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exclusions::{ExclusionArea, ExclusionType};
    use openui_geometry::BfcRect;

    #[test]
    fn no_exclusion_space_returns_full_width() {
        let result = compute_line_availability(
            None,
            LayoutUnit::zero(),
            LayoutUnit::from_f32(400.0),
            LayoutUnit::zero(),
        );
        assert_eq!(result.inline_start, LayoutUnit::zero());
        assert_eq!(result.available_inline_size, LayoutUnit::from_f32(400.0));
    }

    #[test]
    fn empty_exclusion_space_returns_full_width() {
        let es = ExclusionSpace::new();
        let result = compute_line_availability(
            Some(&es),
            LayoutUnit::zero(),
            LayoutUnit::from_f32(400.0),
            LayoutUnit::zero(),
        );
        assert_eq!(result.inline_start, LayoutUnit::zero());
        assert_eq!(result.available_inline_size, LayoutUnit::from_f32(400.0));
    }

    #[test]
    fn left_float_narrows_from_left() {
        let mut es = ExclusionSpace::new();
        // Left float: 100px wide, 50px tall, starting at (0, 0).
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(LayoutUnit::zero(), LayoutUnit::zero()),
                BfcOffset::new(LayoutUnit::from_f32(100.0), LayoutUnit::from_f32(50.0)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        // Line at y=10 (within float range): shifted right, narrower.
        let result = compute_line_availability(
            Some(&es),
            LayoutUnit::from_f32(10.0),
            LayoutUnit::from_f32(400.0),
            LayoutUnit::zero(),
        );
        assert_eq!(result.inline_start, LayoutUnit::from_f32(100.0));
        assert_eq!(result.available_inline_size, LayoutUnit::from_f32(300.0));

        // Line at y=60 (below float): full width.
        let result = compute_line_availability(
            Some(&es),
            LayoutUnit::from_f32(60.0),
            LayoutUnit::from_f32(400.0),
            LayoutUnit::zero(),
        );
        assert_eq!(result.inline_start, LayoutUnit::zero());
        assert_eq!(result.available_inline_size, LayoutUnit::from_f32(400.0));
    }

    #[test]
    fn right_float_narrows_from_right() {
        let mut es = ExclusionSpace::new();
        // Right float: starts at x=300, 100px wide, 50px tall.
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(LayoutUnit::from_f32(300.0), LayoutUnit::zero()),
                BfcOffset::new(LayoutUnit::from_f32(400.0), LayoutUnit::from_f32(50.0)),
            ),
            exclusion_type: ExclusionType::Right,
        });

        // Line at y=10: narrowed from the right.
        let result = compute_line_availability(
            Some(&es),
            LayoutUnit::from_f32(10.0),
            LayoutUnit::from_f32(400.0),
            LayoutUnit::zero(),
        );
        assert_eq!(result.inline_start, LayoutUnit::zero());
        assert_eq!(result.available_inline_size, LayoutUnit::from_f32(300.0));
    }

    #[test]
    fn both_floats_narrow_from_both_sides() {
        let mut es = ExclusionSpace::new();
        // Left float: 80px wide, 40px tall.
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(LayoutUnit::zero(), LayoutUnit::zero()),
                BfcOffset::new(LayoutUnit::from_f32(80.0), LayoutUnit::from_f32(40.0)),
            ),
            exclusion_type: ExclusionType::Left,
        });
        // Right float: 60px wide, 30px tall.
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(LayoutUnit::from_f32(340.0), LayoutUnit::zero()),
                BfcOffset::new(LayoutUnit::from_f32(400.0), LayoutUnit::from_f32(30.0)),
            ),
            exclusion_type: ExclusionType::Right,
        });

        // Line at y=10 (both floats active): narrowed from both sides.
        let result = compute_line_availability(
            Some(&es),
            LayoutUnit::from_f32(10.0),
            LayoutUnit::from_f32(400.0),
            LayoutUnit::zero(),
        );
        assert_eq!(result.inline_start, LayoutUnit::from_f32(80.0));
        assert_eq!(result.available_inline_size, LayoutUnit::from_f32(260.0));

        // Line at y=35 (only left float active): narrowed from left only.
        let result = compute_line_availability(
            Some(&es),
            LayoutUnit::from_f32(35.0),
            LayoutUnit::from_f32(400.0),
            LayoutUnit::zero(),
        );
        assert_eq!(result.inline_start, LayoutUnit::from_f32(80.0));
        assert_eq!(result.available_inline_size, LayoutUnit::from_f32(320.0));
    }

    #[test]
    fn float_taller_than_content_narrows_all_lines() {
        let mut es = ExclusionSpace::new();
        // Left float: 150px wide, 1000px tall.
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(LayoutUnit::zero(), LayoutUnit::zero()),
                BfcOffset::new(LayoutUnit::from_f32(150.0), LayoutUnit::from_f32(1000.0)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        // Lines at various offsets all see the float.
        for y in [0.0, 50.0, 200.0, 500.0, 999.0] {
            let result = compute_line_availability(
                Some(&es),
                LayoutUnit::from_f32(y),
                LayoutUnit::from_f32(400.0),
                LayoutUnit::zero(),
            );
            assert_eq!(result.inline_start, LayoutUnit::from_f32(150.0));
            assert_eq!(result.available_inline_size, LayoutUnit::from_f32(250.0));
        }
    }
}
