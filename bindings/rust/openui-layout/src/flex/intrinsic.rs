//! Intrinsic sizing for flex containers — min-content and max-content.
//!
//! Extracted from Blink's `FlexLayoutAlgorithm::ComputeMinMaxSizes()`
//! (flex_layout_algorithm.cc:2915).
//!
//! Different formulas for each combination of:
//! - Row vs Column direction
//! - Single-line vs Multi-line

use openui_geometry::{LayoutUnit, MinMaxSizes};
use super::item::FlexItem;

/// Compute min-content and max-content sizes for a flex container.
///
/// Blink: `FlexLayoutAlgorithm::ComputeMinMaxSizes()` at line 2915.
///
/// - `items`: all flex items with resolved sizes
/// - `is_column`: true if flex-direction is column/column-reverse
/// - `is_multi_line`: true if flex-wrap is wrap/wrap-reverse
/// - `gap_between_items`: resolved gap between items on main axis
pub fn compute_flex_min_max_sizes(
    items: &[FlexItem],
    is_column: bool,
    is_multi_line: bool,
    gap_between_items: LayoutUnit,
) -> MinMaxSizes {
    if items.is_empty() {
        return MinMaxSizes::zero();
    }

    if is_column {
        compute_column_min_max(items, is_multi_line, gap_between_items)
    } else {
        compute_row_min_max(items, is_multi_line, gap_between_items)
    }
}

/// Row flex container intrinsic sizing.
///
/// Single-line: min = sum(item.min), max = sum(item.max) + gaps
/// Multi-line:  min = max(item.min), max = sum(item.max) + gaps
fn compute_row_min_max(
    items: &[FlexItem],
    is_multi_line: bool,
    gap_between_items: LayoutUnit,
) -> MinMaxSizes {
    let num_gaps = if items.len() > 1 { items.len() as i32 - 1 } else { 0 };
    let total_gap = gap_between_items * num_gaps;

    let mut min_content = LayoutUnit::zero();
    let mut max_content = LayoutUnit::zero();

    for item in items {
        let item_min = item.main_axis_min_max.min + item.main_axis_border_padding
            + item.main_axis_margin_extent();
        let item_max = item.hypothetical_main_axis_margin_box_size();

        if is_multi_line {
            // Multi-line row: min = max of all item mins (longest item determines width)
            min_content = min_content.max_of(item_min);
        } else {
            // Single-line row: min = sum of all item mins
            min_content = min_content + item_min;
        }
        max_content = max_content + item_max;
    }

    if !is_multi_line {
        min_content = min_content + total_gap;
    }
    max_content = max_content + total_gap;

    MinMaxSizes::new(min_content, max_content)
}

/// Column flex container intrinsic sizing.
///
/// Single-line: min = max(item.min_cross), max = max(item.max_cross)
/// Multi-line:  Full layout simulation (not yet implemented, approximated)
fn compute_column_min_max(
    items: &[FlexItem],
    _is_multi_line: bool,
    _gap_between_items: LayoutUnit,
) -> MinMaxSizes {
    // For column flex, the intrinsic inline size depends on cross-axis sizes.
    // The cross axis for column flex is the inline axis (width).
    // Since FlexItem only stores main-axis (block/height) sizes,
    // we approximate using the item's cross-axis margin extent.
    // Full accuracy requires a dedicated cross-axis min/max field on FlexItem,
    // which will be added when this module is wired into the layout path.
    //
    // Column single-line: inline size = max of all items' cross-axis sizes
    // Column multi-line: inline size = sum of line cross sizes (needs full layout)

    let mut min_content = LayoutUnit::zero();
    let mut max_content = LayoutUnit::zero();

    for item in items {
        // Cross-axis contribution = cross margin extent only (no content data available).
        // This is a known limitation; the caller must provide cross-axis content sizes
        // for accurate results.
        let item_cross = item.cross_axis_margin_extent();

        min_content = min_content.max_of(item_cross);
        max_content = max_content.max_of(item_cross);
    }

    MinMaxSizes::new(min_content, max_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::BoxStrut;
    use openui_dom::NodeId;
    use openui_style::ItemPosition;
    use super::super::item::FlexerState;

    fn make_item(base: i32, min: i32, max: i32) -> FlexItem {
        FlexItem {
            node_id: NodeId::NONE,
            item_index: 0,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            base_content_size: LayoutUnit::from_i32(base),
            hypothetical_content_size: LayoutUnit::from_i32(base),
            main_axis_min_max: MinMaxSizes::new(
                LayoutUnit::from_i32(min),
                LayoutUnit::from_i32(max),
            ),
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
    fn row_single_line_min_is_sum() {
        let items = vec![make_item(100, 50, 200), make_item(100, 30, 150)];
        let result = compute_flex_min_max_sizes(&items, false, false, LayoutUnit::zero());
        // min = 50 + 30 = 80
        assert_eq!(result.min, LayoutUnit::from_i32(80));
        // max = 100 + 100 = 200
        assert_eq!(result.max, LayoutUnit::from_i32(200));
    }

    #[test]
    fn row_multi_line_min_is_max() {
        let items = vec![make_item(100, 50, 200), make_item(100, 80, 150)];
        let result = compute_flex_min_max_sizes(&items, false, true, LayoutUnit::zero());
        // min = max(50, 80) = 80
        assert_eq!(result.min, LayoutUnit::from_i32(80));
        // max = 100 + 100 = 200
        assert_eq!(result.max, LayoutUnit::from_i32(200));
    }

    #[test]
    fn row_with_gaps() {
        let items = vec![make_item(100, 50, 200), make_item(100, 30, 150)];
        let gap = LayoutUnit::from_i32(10);
        let result = compute_flex_min_max_sizes(&items, false, false, gap);
        // min = 50 + 30 + 10 = 90
        assert_eq!(result.min, LayoutUnit::from_i32(90));
        // max = 100 + 100 + 10 = 210
        assert_eq!(result.max, LayoutUnit::from_i32(210));
    }
}
