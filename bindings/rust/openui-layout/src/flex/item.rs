//! FlexItem — per-item data during flex layout.
//!
//! Extracted from Blink's `FlexItem` (core/layout/flex/flex_item.h:20-136).
//! Stores resolved flex properties, base sizes, and mutable state for the
//! LineFlexer grow/shrink algorithm.

use openui_dom::NodeId;
use openui_geometry::{BoxStrut, LayoutUnit, MinMaxSizes};
use openui_style::ItemPosition;

/// State of a flex item during the resolve-flexible-lengths algorithm.
/// Blink: `FlexerState` used internally in `LineFlexer`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlexerState {
    /// Not yet processed.
    None,
    /// Clamped to minimum — positive violation.
    MinViolation,
    /// Clamped to maximum — negative violation.
    MaxViolation,
    /// Frozen — won't change in subsequent rounds.
    Frozen,
}

impl Default for FlexerState {
    fn default() -> Self { Self::None }
}

/// Per-item data collected during `ConstructAndAppendFlexItems`.
///
/// Mirrors Blink's `FlexItem` struct. All sizes are content-box values
/// (excluding border+padding) unless noted otherwise.
#[derive(Debug, Clone)]
pub struct FlexItem {
    /// The DOM node this item corresponds to.
    pub node_id: NodeId,

    /// Index in the `flex_items` vector (for FlexLine references).
    pub item_index: usize,

    // ── From style ───────────────────────────────────────────────────

    /// Resolved flex-grow value. Default: 0.0.
    pub flex_grow: f32,

    /// Resolved flex-shrink value. Default: 1.0.
    pub flex_shrink: f32,

    // ── Resolved sizes (content-box, excludes border/padding) ────────

    /// Flex base size minus border+padding (content-box).
    /// Blink: `base_content_size`.
    pub base_content_size: LayoutUnit,

    /// `clamp(base_content_size, min, max)` — the hypothetical main size.
    /// Blink: `hypothetical_content_size`.
    pub hypothetical_content_size: LayoutUnit,

    /// Min and max constraints in the main axis (content-box).
    /// Blink: `main_axis_min_max_sizes`.
    pub main_axis_min_max: MinMaxSizes,

    /// Sum of border + padding in the main axis direction.
    /// Blink: `main_axis_border_padding`.
    pub main_axis_border_padding: LayoutUnit,

    // ── Margins ──────────────────────────────────────────────────────

    /// Resolved physical margins (before auto-margin resolution).
    /// Blink: `initial_margins` (PhysicalBoxStrut).
    pub margin: BoxStrut,

    /// Number of `auto` margins on the main axis (0, 1, or 2).
    /// Blink: `main_axis_auto_margin_count`.
    pub main_axis_auto_margin_count: u8,

    // ── Alignment ────────────────────────────────────────────────────

    /// Resolved alignment for this item (after auto → parent's align-items,
    /// normal → stretch, and writing-mode coercion).
    /// Blink: `alignment` field.
    pub alignment: ItemPosition,

    // ── Mutable state (used during LineFlexer) ───────────────────────

    /// Final main-axis content size after grow/shrink.
    /// Blink: `flexed_content_size`.
    pub flexed_content_size: LayoutUnit,

    /// Current state in the resolve-flexible-lengths algorithm.
    /// Blink: `state` field.
    pub state: FlexerState,

    /// Fraction of free space this item receives during distribution.
    /// Blink: `free_space_fraction` (double).
    pub free_space_fraction: f64,

    // ── Flags ────────────────────────────────────────────────────────

    /// True if flex-basis resolved to a content-based value.
    /// Blink: `is_used_flex_basis_indefinite`.
    pub is_used_flex_basis_indefinite: bool,

    /// True if the main axis is horizontal (for axis mapping).
    /// Blink: `is_horizontal_flow`.
    pub is_horizontal_flow: bool,
}

impl FlexItem {
    /// Hypothetical main axis margin-box size.
    /// Blink: `HypotheticalMainAxisMarginBoxSize()`.
    #[inline]
    pub fn hypothetical_main_axis_margin_box_size(&self) -> LayoutUnit {
        self.hypothetical_content_size
            + self.main_axis_border_padding
            + self.main_axis_margin_extent()
    }

    /// Flex base margin-box size.
    /// Blink: `FlexBaseMarginBoxSize()`.
    #[inline]
    pub fn flex_base_margin_box_size(&self) -> LayoutUnit {
        self.base_content_size
            + self.main_axis_border_padding
            + self.main_axis_margin_extent()
    }

    /// Flexed border-box size (after grow/shrink).
    /// Blink: `FlexedBorderBoxSize()`.
    #[inline]
    pub fn flexed_border_box_size(&self) -> LayoutUnit {
        self.flexed_content_size + self.main_axis_border_padding
    }

    /// Flexed margin-box size (after grow/shrink).
    /// Blink: `FlexedMarginBoxSize()`.
    #[inline]
    pub fn flexed_margin_box_size(&self) -> LayoutUnit {
        self.flexed_content_size
            + self.main_axis_border_padding
            + self.main_axis_margin_extent()
    }

    /// Sum of margins on the main axis.
    /// Blink: `MainAxisMarginExtent()`.
    #[inline]
    pub fn main_axis_margin_extent(&self) -> LayoutUnit {
        if self.is_horizontal_flow {
            self.margin.inline_sum()
        } else {
            self.margin.block_sum()
        }
    }

    /// Sum of margins on the cross axis.
    /// Blink: `CrossAxisMarginExtent()`.
    #[inline]
    pub fn cross_axis_margin_extent(&self) -> LayoutUnit {
        if self.is_horizontal_flow {
            self.margin.block_sum()
        } else {
            self.margin.inline_sum()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_geometry::LayoutUnit;

    fn make_item(base: i32, hyp: i32, bp: i32) -> FlexItem {
        FlexItem {
            node_id: NodeId::NONE,
            item_index: 0,
            flex_grow: 0.0,
            flex_shrink: 1.0,
            base_content_size: LayoutUnit::from_i32(base),
            hypothetical_content_size: LayoutUnit::from_i32(hyp),
            main_axis_min_max: MinMaxSizes::zero(),
            main_axis_border_padding: LayoutUnit::from_i32(bp),
            margin: BoxStrut::new(
                LayoutUnit::from_i32(5), LayoutUnit::from_i32(10),
                LayoutUnit::from_i32(5), LayoutUnit::from_i32(10),
            ),
            main_axis_auto_margin_count: 0,
            alignment: ItemPosition::Stretch,
            flexed_content_size: LayoutUnit::from_i32(hyp),
            state: FlexerState::None,
            free_space_fraction: 0.0,
            is_used_flex_basis_indefinite: false,
            is_horizontal_flow: true,
        }
    }

    #[test]
    fn margin_box_sizes() {
        let item = make_item(100, 100, 20);
        // horizontal flow: main = inline, margins = left(10) + right(10) = 20
        assert_eq!(item.main_axis_margin_extent(), LayoutUnit::from_i32(20));
        assert_eq!(item.cross_axis_margin_extent(), LayoutUnit::from_i32(10));
        // hypothetical_main_margin_box = 100 + 20 + 20 = 140
        assert_eq!(item.hypothetical_main_axis_margin_box_size(), LayoutUnit::from_i32(140));
        // flexed_border_box = 100 + 20 = 120
        assert_eq!(item.flexed_border_box_size(), LayoutUnit::from_i32(120));
        // flexed_margin_box = 100 + 20 + 20 = 140
        assert_eq!(item.flexed_margin_box_size(), LayoutUnit::from_i32(140));
    }
}
