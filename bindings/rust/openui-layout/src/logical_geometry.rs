//! Shared computed-style projection into logical layout coordinates.
//!
//! `ComputedStyle` intentionally keeps CSS physical properties and fragments
//! intentionally keep physical geometry. Layout algorithms consume this view
//! between those boundaries so writing-mode conversion happens once and is not
//! reimplemented by block, flex, fragmentation, and positioned layout.

use openui_geometry::{LayoutUnit, Length, LogicalBoxStrut, LogicalSize, PhysicalSize};
use openui_style::ComputedStyle;

/// Four computed length values addressed by logical edge.
#[derive(Debug, Clone, Copy)]
pub struct LogicalLengthSides<'a> {
    pub inline_start: &'a Length,
    pub inline_end: &'a Length,
    pub block_start: &'a Length,
    pub block_end: &'a Length,
}

/// Computed min/preferred/max sizes addressed by logical axis.
#[derive(Debug, Clone, Copy)]
pub struct LogicalSizeLengths<'a> {
    pub inline_size: &'a Length,
    pub block_size: &'a Length,
    pub min_inline_size: &'a Length,
    pub min_block_size: &'a Length,
    pub max_inline_size: &'a Length,
    pub max_block_size: &'a Length,
}

/// One authoritative logical view of the physical fields in `ComputedStyle`.
#[derive(Debug, Clone, Copy)]
pub struct ResolvedLogicalBox<'a> {
    pub writing_direction: openui_geometry::WritingDirectionMode,
    pub sizes: LogicalSizeLengths<'a>,
    pub margins: LogicalLengthSides<'a>,
    pub padding: LogicalLengthSides<'a>,
    pub insets: LogicalLengthSides<'a>,
    pub border: LogicalBoxStrut,
}

impl<'a> ResolvedLogicalBox<'a> {
    pub fn from_style(style: &'a ComputedStyle) -> Self {
        let writing_direction = style.direction.writing_direction(style.writing_mode);
        let sizes = if writing_direction.is_horizontal() {
            LogicalSizeLengths {
                inline_size: &style.width,
                block_size: &style.height,
                min_inline_size: &style.min_width,
                min_block_size: &style.min_height,
                max_inline_size: &style.max_width,
                max_block_size: &style.max_height,
            }
        } else {
            LogicalSizeLengths {
                inline_size: &style.height,
                block_size: &style.width,
                min_inline_size: &style.min_height,
                min_block_size: &style.min_width,
                max_inline_size: &style.max_height,
                max_block_size: &style.max_width,
            }
        };
        Self {
            writing_direction,
            sizes,
            margins: logical_length_sides(
                &style.margin_top,
                &style.margin_right,
                &style.margin_bottom,
                &style.margin_left,
                writing_direction,
            ),
            padding: logical_length_sides(
                &style.padding_top,
                &style.padding_right,
                &style.padding_bottom,
                &style.padding_left,
                writing_direction,
            ),
            insets: logical_length_sides(
                &style.top,
                &style.right,
                &style.bottom,
                &style.left,
                writing_direction,
            ),
            border: openui_geometry::BoxStrut::new(
                LayoutUnit::from_i32(style.effective_border_top()),
                LayoutUnit::from_i32(style.effective_border_right()),
                LayoutUnit::from_i32(style.effective_border_bottom()),
                LayoutUnit::from_i32(style.effective_border_left()),
            )
            .to_logical(writing_direction),
        }
    }

    /// Convert a completed logical border-box size to fragment storage.
    #[inline]
    pub fn physical_size(self, size: LogicalSize) -> PhysicalSize {
        if self.writing_direction.is_horizontal() {
            PhysicalSize::new(size.inline_size, size.block_size)
        } else {
            PhysicalSize::new(size.block_size, size.inline_size)
        }
    }
}

fn logical_length_sides<'a>(
    top: &'a Length,
    right: &'a Length,
    bottom: &'a Length,
    left: &'a Length,
    writing_direction: openui_geometry::WritingDirectionMode,
) -> LogicalLengthSides<'a> {
    if writing_direction.is_horizontal() {
        let (inline_start, inline_end) = if writing_direction.is_rtl() {
            (right, left)
        } else {
            (left, right)
        };
        LogicalLengthSides {
            inline_start,
            inline_end,
            block_start: top,
            block_end: bottom,
        }
    } else {
        let inline_reversed = writing_direction.is_flipped_lines() ^ writing_direction.is_rtl();
        let (inline_start, inline_end) = if inline_reversed {
            (bottom, top)
        } else {
            (top, bottom)
        };
        let (block_start, block_end) = if writing_direction.is_flipped_blocks() {
            (right, left)
        } else {
            (left, right)
        };
        LogicalLengthSides {
            inline_start,
            inline_end,
            block_start,
            block_end,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::Length;
    use openui_style::{BorderStyle, Direction, WritingMode};

    fn px(length: &Length) -> f32 {
        length.value()
    }

    fn distinctive_style(writing_mode: WritingMode, direction: Direction) -> ComputedStyle {
        let mut style = ComputedStyle::default();
        style.writing_mode = writing_mode;
        style.direction = direction;
        style.width = Length::px(10.0);
        style.height = Length::px(20.0);
        style.min_width = Length::px(30.0);
        style.min_height = Length::px(40.0);
        style.max_width = Length::px(50.0);
        style.max_height = Length::px(60.0);
        style.margin_top = Length::px(1.0);
        style.margin_right = Length::px(2.0);
        style.margin_bottom = Length::px(3.0);
        style.margin_left = Length::px(4.0);
        style.border_top_width = 5;
        style.border_right_width = 6;
        style.border_bottom_width = 7;
        style.border_left_width = 8;
        style.border_top_style = BorderStyle::Solid;
        style.border_right_style = BorderStyle::Solid;
        style.border_bottom_style = BorderStyle::Solid;
        style.border_left_style = BorderStyle::Solid;
        style
    }

    #[test]
    fn horizontal_projection_is_a_strict_no_op_for_sizes() {
        let style = distinctive_style(WritingMode::HorizontalTb, Direction::Ltr);
        let logical = ResolvedLogicalBox::from_style(&style);
        assert_eq!(px(logical.sizes.inline_size), 10.0);
        assert_eq!(px(logical.sizes.block_size), 20.0);
        assert_eq!(px(logical.sizes.min_inline_size), 30.0);
        assert_eq!(px(logical.sizes.min_block_size), 40.0);
        assert_eq!(px(logical.sizes.max_inline_size), 50.0);
        assert_eq!(px(logical.sizes.max_block_size), 60.0);
        assert_eq!(px(logical.margins.inline_start), 4.0);
        assert_eq!(px(logical.margins.block_start), 1.0);
    }

    #[test]
    fn vertical_rl_projection_swaps_axes_and_maps_edges() {
        let style = distinctive_style(WritingMode::VerticalRl, Direction::Ltr);
        let logical = ResolvedLogicalBox::from_style(&style);
        assert_eq!(px(logical.sizes.inline_size), 20.0);
        assert_eq!(px(logical.sizes.block_size), 10.0);
        assert_eq!(px(logical.margins.inline_start), 1.0);
        assert_eq!(px(logical.margins.inline_end), 3.0);
        assert_eq!(px(logical.margins.block_start), 2.0);
        assert_eq!(px(logical.margins.block_end), 4.0);
        assert_eq!(logical.border.inline_start, LayoutUnit::from_i32(5));
        assert_eq!(logical.border.block_start, LayoutUnit::from_i32(6));
    }

    #[test]
    fn sideways_lr_rtl_combines_line_flip_and_direction() {
        let style = distinctive_style(WritingMode::SidewaysLr, Direction::Rtl);
        let logical = ResolvedLogicalBox::from_style(&style);
        assert_eq!(px(logical.margins.inline_start), 1.0);
        assert_eq!(px(logical.margins.inline_end), 3.0);
        assert_eq!(px(logical.margins.block_start), 4.0);
        assert_eq!(px(logical.margins.block_end), 2.0);
    }

    #[test]
    fn completed_logical_size_converts_to_physical_fragment_size() {
        let style = distinctive_style(WritingMode::VerticalLr, Direction::Ltr);
        let logical = ResolvedLogicalBox::from_style(&style);
        let physical = logical.physical_size(LogicalSize::new(
            LayoutUnit::from_i32(120),
            LayoutUnit::from_i32(80),
        ));
        assert_eq!(physical.width, LayoutUnit::from_i32(80));
        assert_eq!(physical.height, LayoutUnit::from_i32(120));
    }
}
