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

use crate::exclusions::{ExclusionSpace, ExclusionType};

/// Per-line available width result from an exclusion space query.
///
/// Contains the inline-start offset (where content begins, shifted right
/// by left floats) and the available inline size (narrowed by both left
/// and right floats).
#[derive(Debug, Clone, Copy)]
pub struct LineAvailability {
    /// Block offset where this opportunity begins. It can be below the
    /// requested shelf when an oversized float leaves no inline space.
    pub block_offset: LayoutUnit,

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
                block_offset: line_block_offset,
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
        block_offset: opportunity.rect.block_start_offset(),
        inline_start,
        available_inline_size: available,
    }
}

/// Compute an opportunity that remains usable across the full line-box
/// height. A float may begin below the line's block-start while still
/// intersecting the line box; sampling only the first shelf would then place
/// atomic inline content through that later float.
pub fn compute_line_availability_for_block_size(
    exclusion_space: Option<&ExclusionSpace>,
    line_block_offset: LayoutUnit,
    container_inline_size: LayoutUnit,
    min_inline_size: LayoutUnit,
    line_block_size: LayoutUnit,
) -> LineAvailability {
    let es = match exclusion_space {
        Some(es) if es.has_floats() && line_block_size > LayoutUnit::zero() => es,
        _ => {
            return compute_line_availability(
                exclusion_space,
                line_block_offset,
                container_inline_size,
                min_inline_size,
            );
        }
    };

    let opportunity = es.find_opportunity_for_bfc(
        &BfcOffset::new(LayoutUnit::zero(), line_block_offset),
        container_inline_size,
        min_inline_size,
        line_block_size,
    );

    // A zero-height float has no positive-area BFC overlap, but CSS line-box
    // construction still wraps a line whose block interval crosses the
    // float's block coordinate. Apply that point exclusion only here; normal
    // BFC opportunity queries deliberately retain their area semantics.
    let opportunity_start = opportunity.rect.block_start_offset();
    let opportunity_end = opportunity_start + line_block_size;
    let mut line_start = opportunity.rect.line_start_offset();
    let mut line_end = opportunity.rect.line_end_offset();
    for exclusion in es.all_exclusions() {
        let float_start = exclusion.rect.block_start_offset();
        if exclusion.rect.block_size() != LayoutUnit::zero()
            || float_start < opportunity_start
            || float_start >= opportunity_end
        {
            continue;
        }
        match exclusion.exclusion_type {
            ExclusionType::Left => {
                line_start = line_start.max_of(exclusion.rect.line_end_offset());
            }
            ExclusionType::Right => {
                line_end = line_end.min_of(exclusion.rect.line_start_offset());
            }
        }
    }

    LineAvailability {
        block_offset: opportunity.rect.block_start_offset(),
        inline_start: line_start,
        available_inline_size: (line_end - line_start).clamp_negative_to_zero(),
    }
}

/// CSS 2.1 §9.5.1 (rule for unfittable shortened line boxes): find the block
/// offset of the nearest float bottom strictly below `block_offset`.
///
/// When a line box shortened by floats is too small to contain its content,
/// the line box is shifted downward until either the content fits or there
/// are no more floats present. Shifting to successive float bottoms is
/// sufficient: available width only changes at float edges.
pub fn next_float_bottom(
    exclusion_space: Option<&ExclusionSpace>,
    block_offset: LayoutUnit,
) -> Option<LayoutUnit> {
    let es = exclusion_space?;
    if !es.has_floats() {
        return None;
    }
    let mut next: Option<LayoutUnit> = None;
    for ex in es.all_exclusions() {
        let bottom = ex.rect.end_offset.block_offset;
        if bottom > block_offset {
            next = Some(match next {
                Some(n) if n <= bottom => n,
                _ => bottom,
            });
        }
    }
    next
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exclusions::{ExclusionArea, ExclusionType};
    use openui_geometry::BfcRect;

    fn lu(value: i32) -> LayoutUnit {
        LayoutUnit::from_i32(value)
    }

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
    fn full_height_line_uses_narrowest_future_shelf() {
        let mut es = ExclusionSpace::new();
        es.add(ExclusionArea {
            rect: BfcRect::new(BfcOffset::new(lu(0), lu(0)), BfcOffset::new(lu(50), lu(75))),
            exclusion_type: ExclusionType::Left,
        });
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(0), lu(75)),
                BfcOffset::new(lu(100), lu(150)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        let result =
            compute_line_availability_for_block_size(Some(&es), lu(50), lu(400), lu(200), lu(50));
        assert_eq!(result.block_offset, lu(50));
        assert_eq!(result.inline_start, lu(100));
        assert_eq!(result.available_inline_size, lu(300));
    }

    #[test]
    fn full_height_line_advances_when_shelves_have_no_common_space() {
        let mut es = ExclusionSpace::new();
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(0), lu(0)),
                BfcOffset::new(lu(250), lu(75)),
            ),
            exclusion_type: ExclusionType::Left,
        });
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(150), lu(75)),
                BfcOffset::new(lu(400), lu(150)),
            ),
            exclusion_type: ExclusionType::Right,
        });

        let result =
            compute_line_availability_for_block_size(Some(&es), lu(50), lu(400), lu(100), lu(50));
        assert_eq!(result.block_offset, lu(75));
        assert_eq!(result.inline_start, lu(0));
        assert_eq!(result.available_inline_size, lu(150));
    }

    #[test]
    fn full_height_line_wraps_at_zero_height_float_coordinate() {
        let mut es = ExclusionSpace::new();
        es.add(ExclusionArea {
            rect: BfcRect::new(BfcOffset::new(lu(0), lu(0)), BfcOffset::new(lu(10), lu(30))),
            exclusion_type: ExclusionType::Left,
        });
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(lu(0), lu(30)),
                BfcOffset::new(lu(100), lu(30)),
            ),
            exclusion_type: ExclusionType::Left,
        });

        let result =
            compute_line_availability_for_block_size(Some(&es), lu(20), lu(500), lu(300), lu(20));
        assert_eq!(result.block_offset, lu(20));
        assert_eq!(result.inline_start, lu(100));
        assert_eq!(result.available_inline_size, lu(400));
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

    #[test]
    fn next_float_bottom_without_floats_is_none() {
        assert_eq!(next_float_bottom(None, LayoutUnit::zero()), None);
        assert_eq!(
            next_float_bottom(Some(&ExclusionSpace::new()), LayoutUnit::zero()),
            None
        );
    }

    #[test]
    fn next_float_bottom_visits_left_and_right_edges_in_order() {
        let mut es = ExclusionSpace::new();
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(LayoutUnit::zero(), LayoutUnit::zero()),
                BfcOffset::new(LayoutUnit::from_f32(80.0), LayoutUnit::from_f32(60.0)),
            ),
            exclusion_type: ExclusionType::Left,
        });
        es.add(ExclusionArea {
            rect: BfcRect::new(
                BfcOffset::new(LayoutUnit::from_f32(300.0), LayoutUnit::zero()),
                BfcOffset::new(LayoutUnit::from_f32(400.0), LayoutUnit::from_f32(35.0)),
            ),
            exclusion_type: ExclusionType::Right,
        });

        assert_eq!(
            next_float_bottom(Some(&es), LayoutUnit::zero()),
            Some(LayoutUnit::from_f32(35.0))
        );
        assert_eq!(
            next_float_bottom(Some(&es), LayoutUnit::from_f32(35.0)),
            Some(LayoutUnit::from_f32(60.0))
        );
        assert_eq!(
            next_float_bottom(Some(&es), LayoutUnit::from_f32(60.0)),
            None
        );
    }
}
