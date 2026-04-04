//! Line breaker — break flex items into lines.
//!
//! Extracted from Blink's `FlexLineBreaker` (core/layout/flex/flex_line_breaker.cc).
//! Implements greedy line breaking for `flex-wrap: wrap` and `wrap-reverse`.

use openui_geometry::LayoutUnit;
use super::item::FlexItem;
use super::line::FlexLine;

/// Break flex items into lines using the greedy algorithm.
///
/// Blink: `GreedyBreakFlexItemsIntoLines()` at flex_line_breaker.cc:405.
///
/// - `items`: all flex items (already order-sorted)
/// - `main_axis_inner_size`: container main axis content-box size
/// - `gap_between_items`: resolved gap between items
/// - `is_multi_line`: true if `flex-wrap` is wrap or wrap-reverse
///
/// Returns a vector of `FlexLine`, each containing indices into `items`.
pub fn break_into_lines(
    items: &[FlexItem],
    main_axis_inner_size: LayoutUnit,
    gap_between_items: LayoutUnit,
    is_multi_line: bool,
) -> Vec<FlexLine> {
    if items.is_empty() {
        return vec![FlexLine::new(vec![])];
    }

    let mut lines: Vec<FlexLine> = Vec::new();
    let mut current_indices: Vec<usize> = Vec::new();
    let mut current_line_size = LayoutUnit::zero();

    for (i, item) in items.iter().enumerate() {
        let item_main_size = item.hypothetical_main_axis_margin_box_size();

        // Add gap if not the first item on the line
        let gap = if current_indices.is_empty() {
            LayoutUnit::zero()
        } else {
            gap_between_items
        };

        let new_line_size = current_line_size + item_main_size + gap;

        // Break condition: multi-line AND line not empty AND would overflow
        // When main axis is indefinite, never break (all items on one line)
        let should_break = is_multi_line
            && !current_indices.is_empty()
            && !main_axis_inner_size.is_indefinite()
            && new_line_size > main_axis_inner_size;

        if should_break {
            // Finalize current line, start new one
            lines.push(FlexLine::new(current_indices));
            current_indices = Vec::new();
            current_line_size = item_main_size;
        } else {
            current_line_size = new_line_size;
        }

        current_indices.push(i);
    }

    // Push the last line
    if !current_indices.is_empty() {
        lines.push(FlexLine::new(current_indices));
    }

    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::{BoxStrut, MinMaxSizes};
    use openui_dom::NodeId;
    use openui_style::ItemPosition;
    use super::super::item::FlexerState;

    fn make_item(index: usize, size: i32) -> FlexItem {
        FlexItem {
            node_id: NodeId::NONE,
            item_index: index,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            base_content_size: LayoutUnit::from_i32(size),
            hypothetical_content_size: LayoutUnit::from_i32(size),
            main_axis_min_max: MinMaxSizes::new(LayoutUnit::zero(), LayoutUnit::from_i32(10000)),
            main_axis_border_padding: LayoutUnit::zero(),
            margin: BoxStrut::zero(),
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
    fn single_line_nowrap() {
        let items = vec![make_item(0, 100), make_item(1, 100), make_item(2, 100)];
        let lines = break_into_lines(
            &items,
            LayoutUnit::from_i32(200), // container smaller than total
            LayoutUnit::zero(),
            false, // nowrap
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].item_indices, vec![0, 1, 2]);
    }

    #[test]
    fn multi_line_wrap() {
        let items = vec![
            make_item(0, 100),
            make_item(1, 100),
            make_item(2, 100),
        ];
        let lines = break_into_lines(
            &items,
            LayoutUnit::from_i32(250),
            LayoutUnit::zero(),
            true, // wrap
        );
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].item_indices, vec![0, 1]);
        assert_eq!(lines[1].item_indices, vec![2]);
    }

    #[test]
    fn wrap_with_gap() {
        let items = vec![
            make_item(0, 100),
            make_item(1, 100),
            make_item(2, 100),
        ];
        // 100 + 10 + 100 = 210 > 200, so second item wraps
        let lines = break_into_lines(
            &items,
            LayoutUnit::from_i32(200),
            LayoutUnit::from_i32(10),
            true,
        );
        assert_eq!(lines.len(), 3); // each on its own line: 100 fits, 100+10+100 > 200
        assert_eq!(lines[0].item_indices, vec![0]);
        assert_eq!(lines[1].item_indices, vec![1]);
        assert_eq!(lines[2].item_indices, vec![2]);
    }

    #[test]
    fn empty_items() {
        let items: Vec<FlexItem> = vec![];
        let lines = break_into_lines(
            &items,
            LayoutUnit::from_i32(400),
            LayoutUnit::zero(),
            true,
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].item_count(), 0);
    }

    #[test]
    fn first_item_always_on_first_line() {
        // Even if a single item exceeds the container, it goes on the first line
        let items = vec![make_item(0, 500)];
        let lines = break_into_lines(
            &items,
            LayoutUnit::from_i32(100),
            LayoutUnit::zero(),
            true,
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].item_indices, vec![0]);
    }
}
