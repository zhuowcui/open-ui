//! Flex alignment — justify-content, align-content, align-self resolution.
//!
//! Extracted from Blink's `FlexLayoutAlgorithm`:
//! - `InitialContentPositionOffset()` (line 1606)
//! - `ContentDistributionSpace()` (line 1661)
//! - Align-self resolution (line 1919)
//! - Auto margin resolution (line 1891)

use openui_geometry::LayoutUnit;
use openui_style::{
    ContentAlignment, ContentDistribution, ContentPosition, ItemPosition,
    OverflowAlignment,
};

/// Result of resolving content alignment (justify-content or align-content).
/// Contains the initial offset before the first item and the spacing between items.
#[derive(Debug, Clone, Copy)]
pub struct ContentAlignmentResult {
    /// Offset before the first item/line.
    pub initial_offset: LayoutUnit,
    /// Space between each pair of items/lines.
    pub between_space: LayoutUnit,
}

/// Resolve content alignment for the main axis (justify-content) or
/// cross axis (align-content).
///
/// Blink: `InitialContentPositionOffset()` + `ContentDistributionSpace()`.
///
/// - `alignment`: the resolved ContentAlignment value
/// - `free_space`: remaining space after items placed
/// - `item_count`: number of items or lines
/// - `is_reverse`: true if flex-direction is reversed
pub fn resolve_content_alignment(
    alignment: &ContentAlignment,
    free_space: LayoutUnit,
    item_count: usize,
    is_reverse: bool,
) -> ContentAlignmentResult {
    // Handle distribution types first (space-between, space-around, space-evenly)
    if alignment.distribution != ContentDistribution::Default {
        return resolve_distribution(alignment, free_space, item_count, is_reverse);
    }

    // Position-only alignment
    let initial_offset = resolve_position_offset(
        alignment.position,
        alignment.overflow,
        free_space,
        is_reverse,
    );

    ContentAlignmentResult {
        initial_offset,
        between_space: LayoutUnit::zero(),
    }
}

/// Resolve distribution-based alignment (space-between, space-around, space-evenly, stretch).
/// Blink: combination of `InitialContentPositionOffset` + `ContentDistributionSpace`.
fn resolve_distribution(
    alignment: &ContentAlignment,
    free_space: LayoutUnit,
    item_count: usize,
    is_reverse: bool,
) -> ContentAlignmentResult {
    let zero = LayoutUnit::zero();

    match alignment.distribution {
        ContentDistribution::SpaceBetween => {
            // Fallback: if free_space <= 0 or only 1 item, behave like flex-start.
            // For reverse, flex-start is at the end, so offset = free_space (may be negative).
            if free_space <= zero || item_count <= 1 {
                let offset = if is_reverse { free_space } else { zero };
                return ContentAlignmentResult {
                    initial_offset: offset,
                    between_space: zero,
                };
            }
            let between = distribute_evenly(free_space, item_count - 1);
            ContentAlignmentResult {
                initial_offset: zero,
                between_space: between,
            }
        }
        ContentDistribution::SpaceAround => {
            // Fallback: if free_space <= 0, safe center
            if free_space <= zero || item_count == 0 {
                let offset = safe_center_offset(free_space);
                return ContentAlignmentResult {
                    initial_offset: offset,
                    between_space: zero,
                };
            }
            let per_item = distribute_evenly(free_space, item_count);
            // Half-space before first — use floor division to match Blink
            let half = LayoutUnit::from_raw(per_item.raw() / 2);
            ContentAlignmentResult {
                initial_offset: half,
                between_space: per_item,
            }
        }
        ContentDistribution::SpaceEvenly => {
            // Fallback: if free_space <= 0, safe center
            if free_space <= zero || item_count == 0 {
                let offset = safe_center_offset(free_space);
                return ContentAlignmentResult {
                    initial_offset: offset,
                    between_space: zero,
                };
            }
            let per_slot = distribute_evenly(free_space, item_count + 1);
            ContentAlignmentResult {
                initial_offset: per_slot,
                between_space: per_slot,
            }
        }
        ContentDistribution::Stretch => {
            // Stretch behaves like flex-start for positioning
            // (actual stretching is handled separately for lines)
            let offset = if is_reverse { free_space } else { zero };
            ContentAlignmentResult {
                initial_offset: offset,
                between_space: zero,
            }
        }
        ContentDistribution::Default => unreachable!(),
    }
}

/// Resolve position-only alignment offset.
/// Blink: `InitialContentPositionOffset()` (line 1606).
fn resolve_position_offset(
    position: ContentPosition,
    overflow: OverflowAlignment,
    free_space: LayoutUnit,
    is_reverse: bool,
) -> LayoutUnit {
    let zero = LayoutUnit::zero();

    // Safe overflow: clamp negative to zero
    let space = if overflow == OverflowAlignment::Safe {
        free_space.clamp_negative_to_zero()
    } else {
        free_space
    };

    match position {
        ContentPosition::FlexStart | ContentPosition::Normal => {
            if is_reverse { space } else { zero }
        }
        ContentPosition::FlexEnd => {
            if is_reverse { zero } else { space }
        }
        ContentPosition::Center => {
            // Blink: free_space / 2
            LayoutUnit::from_raw(space.raw() / 2)
        }
        ContentPosition::Start => zero,
        ContentPosition::End => space,
        ContentPosition::Left => zero,  // LTR assumption
        ContentPosition::Right => space,
        ContentPosition::Baseline | ContentPosition::LastBaseline => {
            // Baseline alignment for content is complex (SP11+).
            // For now, treat as flex-start.
            if is_reverse { space } else { zero }
        }
    }
}

/// Safe center: if free_space is negative, clamp to 0.
/// Blink: `InitialContentPositionOffset` fallback for space-around/space-evenly.
#[inline]
fn safe_center_offset(free_space: LayoutUnit) -> LayoutUnit {
    if free_space <= LayoutUnit::zero() {
        LayoutUnit::zero()
    } else {
        LayoutUnit::from_raw(free_space.raw() / 2)
    }
}

/// Distribute `total` evenly across `n` slots.
/// Uses integer division matching Blink's `LayoutUnitDiffuser`.
#[inline]
fn distribute_evenly(total: LayoutUnit, n: usize) -> LayoutUnit {
    if n == 0 {
        return LayoutUnit::zero();
    }
    LayoutUnit::from_raw(total.raw() / n as i32)
}

/// Resolve the cross-axis offset for a single item based on align-self.
///
/// - `alignment`: resolved ItemPosition for this item
/// - `cross_space`: available cross-axis space (line_cross_size - item_cross_margin_box)
/// - `overflow`: overflow alignment modifier
/// - `is_wrap_reverse`: whether flex-wrap is wrap-reverse (flips flex-start/flex-end)
///
/// Returns the cross-axis offset for the item within its line.
pub fn resolve_align_self(
    alignment: ItemPosition,
    cross_space: LayoutUnit,
    overflow: OverflowAlignment,
    is_wrap_reverse: bool,
) -> LayoutUnit {
    let zero = LayoutUnit::zero();

    // Safe overflow: clamp negative to zero
    let space = if overflow == OverflowAlignment::Safe {
        cross_space.clamp_negative_to_zero()
    } else {
        cross_space
    };

    // Wrap-reverse flips flex-start and flex-end (Blink line 1919)
    let effective_alignment = if is_wrap_reverse {
        match alignment {
            ItemPosition::FlexStart => ItemPosition::FlexEnd,
            ItemPosition::FlexEnd => ItemPosition::FlexStart,
            other => other,
        }
    } else {
        alignment
    };

    match effective_alignment {
        ItemPosition::FlexStart | ItemPosition::Start | ItemPosition::SelfStart => zero,
        ItemPosition::FlexEnd | ItemPosition::End | ItemPosition::SelfEnd => space,
        ItemPosition::Center => LayoutUnit::from_raw(space.raw() / 2),
        ItemPosition::Stretch => {
            // Stretch: item at start, size expanded (handled elsewhere)
            if is_wrap_reverse { space } else { zero }
        }
        ItemPosition::Baseline | ItemPosition::LastBaseline => {
            // Baseline alignment offset computed separately from baseline tracking.
            // For now, treat as flex-start.
            zero
        }
        // Auto should have been resolved before reaching here
        ItemPosition::Auto | ItemPosition::Normal => zero,
        ItemPosition::Legacy | ItemPosition::Left | ItemPosition::Right => zero,
    }
}

/// Resolve auto margins on the main axis for a single item.
///
/// Blink: auto margins consume free space before justify-content applies.
/// Returns the (start_margin, end_margin) values for the main axis.
///
/// - `auto_margin_count`: number of auto margins on main axis (0, 1, or 2)
/// - `free_space`: remaining free space (must be > 0 for auto margins to activate)
/// - `is_start_auto`: true if the start margin is auto
/// - `is_end_auto`: true if the end margin is auto
pub fn resolve_main_auto_margins(
    free_space: LayoutUnit,
    is_start_auto: bool,
    is_end_auto: bool,
) -> (LayoutUnit, LayoutUnit) {
    let zero = LayoutUnit::zero();

    // Auto margins only distribute positive free space
    if free_space <= zero {
        return (zero, zero);
    }

    match (is_start_auto, is_end_auto) {
        (true, true) => {
            let half = LayoutUnit::from_raw(free_space.raw() / 2);
            (half, free_space - half)
        }
        (true, false) => (free_space, zero),
        (false, true) => (zero, free_space),
        (false, false) => (zero, zero),
    }
}

/// Resolve auto margins on the cross axis for a single item.
///
/// Blink: cross-axis auto margins override align-self.
/// Returns the (start_margin, end_margin) values for the cross axis.
///
/// - `cross_space`: available cross-axis space (clamped to >= 0)
/// - `is_start_auto`: true if the cross-start margin is auto
/// - `is_end_auto`: true if the cross-end margin is auto
pub fn resolve_cross_auto_margins(
    cross_space: LayoutUnit,
    is_start_auto: bool,
    is_end_auto: bool,
) -> (LayoutUnit, LayoutUnit) {
    let zero = LayoutUnit::zero();
    let space = cross_space.clamp_negative_to_zero();

    match (is_start_auto, is_end_auto) {
        (true, true) => {
            let half = LayoutUnit::from_raw(space.raw() / 2);
            (half, space - half)
        }
        (true, false) => (space, zero),
        (false, true) => (zero, space),
        (false, false) => (zero, zero),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn justify_content_flex_start() {
        let align = ContentAlignment::new(ContentPosition::FlexStart);
        let result = resolve_content_alignment(&align, LayoutUnit::from_i32(100), 3, false);
        assert_eq!(result.initial_offset, LayoutUnit::zero());
        assert_eq!(result.between_space, LayoutUnit::zero());
    }

    #[test]
    fn justify_content_flex_end() {
        let align = ContentAlignment::new(ContentPosition::FlexEnd);
        let result = resolve_content_alignment(&align, LayoutUnit::from_i32(100), 3, false);
        assert_eq!(result.initial_offset, LayoutUnit::from_i32(100));
        assert_eq!(result.between_space, LayoutUnit::zero());
    }

    #[test]
    fn justify_content_center() {
        let align = ContentAlignment::new(ContentPosition::Center);
        let result = resolve_content_alignment(&align, LayoutUnit::from_i32(100), 3, false);
        assert_eq!(result.initial_offset, LayoutUnit::from_i32(50));
        assert_eq!(result.between_space, LayoutUnit::zero());
    }

    #[test]
    fn justify_content_space_between() {
        let align = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
        let result = resolve_content_alignment(&align, LayoutUnit::from_i32(100), 3, false);
        assert_eq!(result.initial_offset, LayoutUnit::zero());
        assert_eq!(result.between_space, LayoutUnit::from_i32(50));
    }

    #[test]
    fn justify_content_space_around() {
        let align = ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
        // 120 free space, 3 items → per_item = 40, half = 20
        let result = resolve_content_alignment(&align, LayoutUnit::from_i32(120), 3, false);
        assert_eq!(result.initial_offset, LayoutUnit::from_i32(20));
        assert_eq!(result.between_space, LayoutUnit::from_i32(40));
    }

    #[test]
    fn justify_content_space_evenly() {
        let align = ContentAlignment::with_distribution(ContentDistribution::SpaceEvenly);
        // 120 free space, 3 items → 4 slots → 30 each
        let result = resolve_content_alignment(&align, LayoutUnit::from_i32(120), 3, false);
        assert_eq!(result.initial_offset, LayoutUnit::from_i32(30));
        assert_eq!(result.between_space, LayoutUnit::from_i32(30));
    }

    #[test]
    fn space_between_negative_fallback() {
        let align = ContentAlignment::with_distribution(ContentDistribution::SpaceBetween);
        let neg = LayoutUnit::from_i32(-50);
        let result = resolve_content_alignment(&align, neg, 3, false);
        // Fallback to flex-start: offset = 0 (not reversed)
        assert_eq!(result.initial_offset, LayoutUnit::zero());
    }

    #[test]
    fn space_around_negative_fallback() {
        let align = ContentAlignment::with_distribution(ContentDistribution::SpaceAround);
        let neg = LayoutUnit::from_i32(-50);
        let result = resolve_content_alignment(&align, neg, 3, false);
        // Fallback to safe center: clamp to 0
        assert_eq!(result.initial_offset, LayoutUnit::zero());
    }

    #[test]
    fn align_self_center() {
        let offset = resolve_align_self(
            ItemPosition::Center,
            LayoutUnit::from_i32(100),
            OverflowAlignment::Default,
            false,
        );
        assert_eq!(offset, LayoutUnit::from_i32(50));
    }

    #[test]
    fn align_self_wrap_reverse_flips() {
        let offset = resolve_align_self(
            ItemPosition::FlexStart,
            LayoutUnit::from_i32(100),
            OverflowAlignment::Default,
            true, // wrap-reverse
        );
        // FlexStart with wrap-reverse → FlexEnd → space
        assert_eq!(offset, LayoutUnit::from_i32(100));
    }

    #[test]
    fn auto_margins_main_both() {
        let (start, end) = resolve_main_auto_margins(LayoutUnit::from_i32(100), true, true);
        assert_eq!(start, LayoutUnit::from_i32(50));
        assert_eq!(end, LayoutUnit::from_i32(50));
    }

    #[test]
    fn auto_margins_main_start_only() {
        let (start, end) = resolve_main_auto_margins(LayoutUnit::from_i32(100), true, false);
        assert_eq!(start, LayoutUnit::from_i32(100));
        assert_eq!(end, LayoutUnit::zero());
    }

    #[test]
    fn auto_margins_no_positive_space() {
        let (start, end) = resolve_main_auto_margins(LayoutUnit::from_i32(-50), true, true);
        assert_eq!(start, LayoutUnit::zero());
        assert_eq!(end, LayoutUnit::zero());
    }

    #[test]
    fn flex_start_reversed() {
        let align = ContentAlignment::new(ContentPosition::FlexStart);
        let result = resolve_content_alignment(&align, LayoutUnit::from_i32(100), 3, true);
        assert_eq!(result.initial_offset, LayoutUnit::from_i32(100));
    }
}
