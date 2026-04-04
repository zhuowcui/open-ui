//! LineFlexer — resolve flexible lengths per CSS Flexbox §9.7.
//!
//! Extracted character-by-character from Blink's `LineFlexer`
//! (core/layout/flex/line_flexer.cc, 182 lines).
//!
//! This is the heart of flexbox: given a set of items on a line and the
//! available main-axis space, distribute extra space (grow) or absorb
//! overflow (shrink) among the items.

use openui_geometry::LayoutUnit;
use super::item::{FlexItem, FlexerState};

/// Mode of operation for the flexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FlexMode {
    Grow,
    Shrink,
}

/// Resolves flexible lengths for a single flex line.
///
/// Implements CSS Flexbox §9.7 "Resolving Flexible Lengths".
/// Blink: `LineFlexer` class in `line_flexer.cc`.
pub struct LineFlexer<'a> {
    items: &'a mut [FlexItem],
    line_indices: &'a [usize],
    mode: FlexMode,
    initial_free_space: LayoutUnit,
    total_flex_factor: f64,
    free_space: LayoutUnit,
    gap_total: LayoutUnit,
    main_axis_inner_size: LayoutUnit,
}

impl<'a> LineFlexer<'a> {
    /// Create a new LineFlexer for the given line.
    ///
    /// Blink constructor: `LineFlexer::LineFlexer(...)` at line_flexer.cc:9.
    ///
    /// - `items`: mutable slice of all flex items
    /// - `line_indices`: indices of items on this line
    /// - `main_axis_inner_size`: container main axis content-box size
    /// - `sum_hypothetical_main_sizes`: sum of all items' hypothetical margin-box sizes
    /// - `gap_between_items`: resolved gap between items
    pub fn new(
        items: &'a mut [FlexItem],
        line_indices: &'a [usize],
        main_axis_inner_size: LayoutUnit,
        sum_hypothetical_main_sizes: LayoutUnit,
        gap_between_items: LayoutUnit,
    ) -> Self {
        let num_gaps = if line_indices.len() > 1 {
            line_indices.len() as i32 - 1
        } else {
            0
        };
        let gap_total = gap_between_items * num_gaps;

        // Step 1: Determine mode (Blink line_flexer.cc:14)
        let mode = if sum_hypothetical_main_sizes + gap_total < main_axis_inner_size {
            FlexMode::Grow
        } else {
            FlexMode::Shrink
        };

        // Step 2: Initial freeze (Blink line_flexer.cc:20-41)
        // Freeze items that cannot flex in the current mode.
        for &idx in line_indices {
            let item = &mut items[idx];
            let flex_factor = match mode {
                FlexMode::Grow => item.flex_grow,
                FlexMode::Shrink => item.flex_shrink,
            };

            let should_freeze = flex_factor == 0.0
                || (mode == FlexMode::Grow && item.base_content_size > item.hypothetical_content_size)
                || (mode == FlexMode::Shrink && item.base_content_size < item.hypothetical_content_size);

            if should_freeze {
                item.state = FlexerState::Frozen;
                item.flexed_content_size = item.hypothetical_content_size;
            } else {
                item.state = FlexerState::None;
            }
        }

        let mut flexer = Self {
            items,
            line_indices,
            mode,
            initial_free_space: LayoutUnit::zero(),
            total_flex_factor: 0.0,
            free_space: LayoutUnit::zero(),
            gap_total,
            main_axis_inner_size,
        };

        flexer.freeze_items();
        flexer.initial_free_space = flexer.free_space;
        flexer
    }

    /// Run the resolve-flexible-lengths loop until convergence.
    /// Blink: called from `PlaceFlexItems` after constructing LineFlexer.
    pub fn run(&mut self) {
        // Loop until no more violations (Blink line_flexer.cc:88)
        while self.resolve_flexible_lengths() {}
    }

    /// Recalculate totals after freezing items.
    /// Blink: `LineFlexer::FreezeItems()` at line_flexer.cc:44.
    fn freeze_items(&mut self) {
        self.total_flex_factor = 0.0;
        self.free_space = self.main_axis_inner_size - self.gap_total;

        // Compute total weighted flex shrink factor for shrink mode
        let mut total_weighted_flex_shrink: f64 = 0.0;

        for &idx in self.line_indices {
            let item = &self.items[idx];
            if item.state == FlexerState::Frozen {
                // Frozen items: subtract their flexed margin-box size from free space
                self.free_space = self.free_space - item.flexed_margin_box_size();
            } else {
                // Unfrozen items: subtract their base margin-box size from free space
                self.free_space = self.free_space - item.flex_base_margin_box_size();

                match self.mode {
                    FlexMode::Grow => {
                        self.total_flex_factor += item.flex_grow as f64;
                    }
                    FlexMode::Shrink => {
                        self.total_flex_factor += item.flex_shrink as f64;
                        // Scaled flex shrink factor = flex_shrink × base_content_size
                        // Blink line_flexer.cc:64
                        total_weighted_flex_shrink +=
                            item.flex_shrink as f64 * item.base_content_size.to_f64();
                    }
                }
            }
        }

        // Compute free_space_fraction for each unfrozen item (Blink line_flexer.cc:68-82)
        for &idx in self.line_indices {
            let item = &mut self.items[idx];
            if item.state == FlexerState::Frozen {
                continue;
            }
            item.free_space_fraction = match self.mode {
                FlexMode::Grow => {
                    if self.total_flex_factor > 0.0 {
                        item.flex_grow as f64 / self.total_flex_factor
                    } else {
                        0.0
                    }
                }
                FlexMode::Shrink => {
                    if total_weighted_flex_shrink > 0.0 {
                        (item.flex_shrink as f64 * item.base_content_size.to_f64())
                            / total_weighted_flex_shrink
                    } else {
                        0.0
                    }
                }
            };
        }
    }

    /// One iteration of the resolve-flexible-lengths algorithm.
    /// Returns true if the loop should continue, false if converged.
    /// Blink: `LineFlexer::ResolveFlexibleLengths()` at line_flexer.cc:88.
    fn resolve_flexible_lengths(&mut self) -> bool {
        // Step a: Fractional flex factors (Blink line_flexer.cc:97-107)
        // If total_flex_factor < 1.0, limit distribution.
        let mut used_free_space = self.free_space;
        if self.total_flex_factor > 0.0 && self.total_flex_factor < 1.0 {
            let limited = LayoutUnit::from_f64(
                self.initial_free_space.to_f64() * self.total_flex_factor
            );
            match self.mode {
                FlexMode::Grow => {
                    if limited < self.free_space {
                        used_free_space = limited;
                    }
                }
                FlexMode::Shrink => {
                    if limited > self.free_space {
                        used_free_space = limited;
                    }
                }
            }
        }

        // Step b: Early exit if no space to distribute (Blink line_flexer.cc:109-115)
        if self.mode == FlexMode::Grow && used_free_space <= LayoutUnit::zero() {
            for &idx in self.line_indices {
                if self.items[idx].state != FlexerState::Frozen {
                    self.items[idx].state = FlexerState::Frozen;
                    self.items[idx].flexed_content_size = self.items[idx].base_content_size;
                }
            }
            return false;
        }
        if self.mode == FlexMode::Shrink && used_free_space >= LayoutUnit::zero() {
            for &idx in self.line_indices {
                if self.items[idx].state != FlexerState::Frozen {
                    self.items[idx].state = FlexerState::Frozen;
                    self.items[idx].flexed_content_size = self.items[idx].base_content_size;
                }
            }
            return false;
        }

        // Step c: Distribute free space (Blink line_flexer.cc:139-168)
        // Iterate in reverse to avoid rounding drift (cumulative fraction approach).
        let mut total_violation = LayoutUnit::zero();
        let mut cumulative_fraction: f64 = 0.0;

        // Collect unfrozen indices in reverse order
        let unfrozen_reversed: Vec<usize> = self.line_indices.iter()
            .rev()
            .copied()
            .filter(|&idx| self.items[idx].state != FlexerState::Frozen)
            .collect();

        let free_space_f64 = used_free_space.to_f64();

        for (_i, &idx) in unfrozen_reversed.iter().enumerate() {
            let item = &mut self.items[idx];

            cumulative_fraction += item.free_space_fraction;

            // Cumulative fraction approach: each item's share is the difference
            // between the cumulative chunk including this item and the previous.
            // This naturally distributes rounding errors to the first-in-forward
            // (last-in-reverse) item since its cumulative_fraction == 1.0.
            let prev_cumulative = cumulative_fraction - item.free_space_fraction;
            let current_chunk = LayoutUnit::from_f64(free_space_f64 * cumulative_fraction);
            let prev_chunk = LayoutUnit::from_f64(free_space_f64 * prev_cumulative);
            let extra_size = current_chunk - prev_chunk;

            let unclamped = item.base_content_size + extra_size;
            let clamped = item.main_axis_min_max.clamp(unclamped);

            let violation = clamped - unclamped;
            total_violation = total_violation + violation;

            item.flexed_content_size = clamped;

            if violation < LayoutUnit::zero() {
                item.state = FlexerState::MaxViolation;
            } else if violation > LayoutUnit::zero() {
                item.state = FlexerState::MinViolation;
            }
        }

        // Step d: Freeze violators (Blink line_flexer.cc:171-181)
        if total_violation == LayoutUnit::zero() {
            // No violations — converged. Freeze all remaining unfrozen items.
            for &idx in self.line_indices {
                if self.items[idx].state != FlexerState::Frozen {
                    self.items[idx].state = FlexerState::Frozen;
                }
            }
            return false;
        }

        // Positive total → freeze min-violations
        // Negative total → freeze max-violations
        let freeze_state = if total_violation > LayoutUnit::zero() {
            FlexerState::MinViolation
        } else {
            FlexerState::MaxViolation
        };

        let mut any_frozen = false;
        for &idx in self.line_indices {
            let item = &mut self.items[idx];
            if item.state == freeze_state {
                item.state = FlexerState::Frozen;
                any_frozen = true;
            } else if item.state != FlexerState::Frozen {
                // Reset non-frozen, non-matching violations for next round
                item.state = FlexerState::None;
            }
        }

        if any_frozen {
            self.freeze_items();
        }

        any_frozen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::MinMaxSizes;
    use openui_dom::NodeId;
    use openui_style::ItemPosition;

    fn make_test_item(index: usize, base: i32, grow: f32, shrink: f32) -> FlexItem {
        FlexItem {
            node_id: NodeId::NONE,
            item_index: index,
            flex_grow: grow,
            flex_shrink: shrink,
            base_content_size: LayoutUnit::from_i32(base),
            hypothetical_content_size: LayoutUnit::from_i32(base),
            main_axis_min_max: MinMaxSizes::new(LayoutUnit::zero(), LayoutUnit::from_i32(10000)),
            main_axis_border_padding: LayoutUnit::zero(),
            margin: openui_geometry::BoxStrut::zero(),
            main_axis_auto_margin_count: 0,
            alignment: ItemPosition::Stretch,
            flexed_content_size: LayoutUnit::zero(),
            state: FlexerState::None,
            free_space_fraction: 0.0,
            is_used_flex_basis_indefinite: false,
            is_horizontal_flow: true,
        }
    }

    #[test]
    fn equal_grow_distributes_evenly() {
        let mut items = vec![
            make_test_item(0, 100, 1.0, 1.0),
            make_test_item(1, 100, 1.0, 1.0),
        ];
        let indices = vec![0, 1];
        let container_size = LayoutUnit::from_i32(400); // 200 extra space
        let sum_hyp = LayoutUnit::from_i32(200);

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, LayoutUnit::zero());
        flexer.run();

        // Each gets 100 extra → 200 each
        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(200));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(200));
    }

    #[test]
    fn weighted_grow() {
        let mut items = vec![
            make_test_item(0, 0, 1.0, 1.0),
            make_test_item(1, 0, 2.0, 1.0),
            make_test_item(2, 0, 1.0, 1.0),
        ];
        let indices = vec![0, 1, 2];
        let container_size = LayoutUnit::from_i32(400);
        let sum_hyp = LayoutUnit::zero();

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, LayoutUnit::zero());
        flexer.run();

        // Proportions: 1:2:1 of 400 = 100:200:100
        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(100));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(200));
        assert_eq!(items[2].flexed_content_size, LayoutUnit::from_i32(100));
    }

    #[test]
    fn equal_shrink() {
        let mut items = vec![
            make_test_item(0, 200, 0.0, 1.0),
            make_test_item(1, 200, 0.0, 1.0),
        ];
        let indices = vec![0, 1];
        let container_size = LayoutUnit::from_i32(300); // 100 overflow
        let sum_hyp = LayoutUnit::from_i32(400);

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, LayoutUnit::zero());
        flexer.run();

        // Equal base sizes → equal shrink: each loses 50 → 150 each
        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(150));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(150));
    }

    #[test]
    fn shrink_weighted_by_base_size() {
        // Item 0: base 300, shrink 1.0 → weighted = 300
        // Item 1: base 100, shrink 1.0 → weighted = 100
        // Total weighted = 400, overflow = 100
        // Item 0 loses: 100 * 300/400 = 75 → 225
        // Item 1 loses: 100 * 100/400 = 25 → 75
        let mut items = vec![
            make_test_item(0, 300, 0.0, 1.0),
            make_test_item(1, 100, 0.0, 1.0),
        ];
        let indices = vec![0, 1];
        let container_size = LayoutUnit::from_i32(300);
        let sum_hyp = LayoutUnit::from_i32(400);

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, LayoutUnit::zero());
        flexer.run();

        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(225));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(75));
    }

    #[test]
    fn min_constraint_freezes_item() {
        // Item 0: base 200, grow 1, min=180
        // Item 1: base 200, grow 1, min=0
        // Container: 300 → need to shrink by 100
        // First round: both shrink by 50 → 150 each
        // Item 0 hits min 180 → clamped, violation = 30
        // Item 0 frozen at 180, item 1 gets remaining: 300 - 180 = 120
        let mut items = vec![
            make_test_item(0, 200, 0.0, 1.0),
            make_test_item(1, 200, 0.0, 1.0),
        ];
        items[0].main_axis_min_max.min = LayoutUnit::from_i32(180);
        let indices = vec![0, 1];
        let container_size = LayoutUnit::from_i32(300);
        let sum_hyp = LayoutUnit::from_i32(400);

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, LayoutUnit::zero());
        flexer.run();

        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(180));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(120));
    }

    #[test]
    fn no_flex_items_keep_hypothetical() {
        let mut items = vec![
            make_test_item(0, 100, 0.0, 0.0),
            make_test_item(1, 100, 0.0, 0.0),
        ];
        let indices = vec![0, 1];
        let container_size = LayoutUnit::from_i32(400);
        let sum_hyp = LayoutUnit::from_i32(200);

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, LayoutUnit::zero());
        flexer.run();

        // No grow/shrink → keep hypothetical sizes
        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(100));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(100));
    }

    #[test]
    fn gap_reduces_free_space() {
        let mut items = vec![
            make_test_item(0, 100, 1.0, 1.0),
            make_test_item(1, 100, 1.0, 1.0),
        ];
        let indices = vec![0, 1];
        let container_size = LayoutUnit::from_i32(400);
        let sum_hyp = LayoutUnit::from_i32(200);
        let gap = LayoutUnit::from_i32(20); // 1 gap of 20px

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, gap);
        flexer.run();

        // Free space = 400 - 200 - 20 = 180, each gets 90
        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(190));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(190));
    }

    #[test]
    fn fractional_flex_factor_less_than_one() {
        // Blink: when total_flex_factor < 1.0, limit to initial_free_space * total
        let mut items = vec![
            make_test_item(0, 100, 0.25, 1.0),
            make_test_item(1, 100, 0.25, 1.0),
        ];
        let indices = vec![0, 1];
        let container_size = LayoutUnit::from_i32(400); // 200 extra
        let sum_hyp = LayoutUnit::from_i32(200);

        let mut flexer = LineFlexer::new(&mut items, &indices, container_size, sum_hyp, LayoutUnit::zero());
        flexer.run();

        // total_flex_factor = 0.5, limit = 200 * 0.5 = 100
        // Each gets 50 → 150 each
        assert_eq!(items[0].flexed_content_size, LayoutUnit::from_i32(150));
        assert_eq!(items[1].flexed_content_size, LayoutUnit::from_i32(150));
    }
}
