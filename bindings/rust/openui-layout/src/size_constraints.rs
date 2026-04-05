//! Min/Max width & height constraint resolution — CSS 2.1 §10.4, §10.7.
//!
//! Implements the constraint resolution algorithm that resolves CSS min-width,
//! max-width, min-height, max-height properties to concrete LayoutUnit values,
//! then applies them to computed sizes.
//!
//! References:
//! - CSS 2.1 §10.4: Minimum and maximum widths
//! - CSS 2.1 §10.7: Minimum and maximum heights
//! - Blink: `length_utils.h` / `length_utils.cc`

use openui_geometry::{LayoutUnit, Length, LengthType};
use openui_style::BoxSizing;

use crate::length_resolver::resolve_length;

/// Resolved min/max size constraints for an element.
///
/// All values are in the element's writing mode (inline/block).
/// `max_inline_size` / `max_block_size` of `LayoutUnit::max()` means "none"
/// (no upper constraint).
#[derive(Debug, Clone, Copy)]
pub struct SizeConstraint {
    pub min_inline_size: LayoutUnit,
    pub max_inline_size: LayoutUnit,
    pub min_block_size: LayoutUnit,
    pub max_block_size: LayoutUnit,
}

impl SizeConstraint {
    /// Unconstrained: min = 0, max = LayoutUnit::max().
    pub fn unconstrained() -> Self {
        Self {
            min_inline_size: LayoutUnit::zero(),
            max_inline_size: LayoutUnit::max(),
            min_block_size: LayoutUnit::zero(),
            max_block_size: LayoutUnit::max(),
        }
    }
}

/// Resolve CSS min/max sizing properties to concrete constraint values.
///
/// Per CSS 2.1 §10.4 / §10.7:
/// - `min-width`/`min-height`: percentages resolve against CB, `auto` → 0
/// - `max-width`/`max-height`: percentages resolve against CB, `none` → unconstrained
/// - Intrinsic keywords (min-content, max-content, fit-content) use the
///   available size as a fallback for now.
/// - `box-sizing: border-box` values have padding+border subtracted so the
///   returned constraints are in the content-box coordinate space.
pub fn resolve_size_constraints(
    style_min_inline: &Length,
    style_max_inline: &Length,
    style_min_block: &Length,
    style_max_block: &Length,
    cb_inline_size: LayoutUnit,
    cb_block_size: LayoutUnit,
    box_sizing: BoxSizing,
    padding_border_inline: LayoutUnit,
    padding_border_block: LayoutUnit,
) -> SizeConstraint {
    let mut min_inline = resolve_min_length(style_min_inline, cb_inline_size);
    let mut max_inline = resolve_max_length(style_max_inline, cb_inline_size);
    let mut min_block = resolve_min_length(style_min_block, cb_block_size);
    let mut max_block = resolve_max_length(style_max_block, cb_block_size);

    // box-sizing: border-box means the specified min/max include padding+border.
    // Convert to content-box space by subtracting padding+border.
    if box_sizing == BoxSizing::BorderBox {
        min_inline = apply_box_sizing_adjustment(min_inline, BoxSizing::BorderBox, padding_border_inline);
        if max_inline != LayoutUnit::max() {
            max_inline = apply_box_sizing_adjustment(max_inline, BoxSizing::BorderBox, padding_border_inline);
        }
        min_block = apply_box_sizing_adjustment(min_block, BoxSizing::BorderBox, padding_border_block);
        if max_block != LayoutUnit::max() {
            max_block = apply_box_sizing_adjustment(max_block, BoxSizing::BorderBox, padding_border_block);
        }
    }

    // CSS 2.1 §10.4: if min > max, min wins.
    if max_inline < min_inline {
        max_inline = min_inline;
    }
    if max_block < min_block {
        max_block = min_block;
    }

    SizeConstraint {
        min_inline_size: min_inline,
        max_inline_size: max_inline,
        min_block_size: min_block,
        max_block_size: max_block,
    }
}

/// Resolve a min-width / min-height CSS length.
///
/// `auto` → 0 (CSS 2.1 initial value for min-width/min-height on non-flex items).
/// Intrinsic keywords are treated as 0 for constraint resolution; the layout
/// algorithm handles them separately.
fn resolve_min_length(length: &Length, containing_block_size: LayoutUnit) -> LayoutUnit {
    match length.length_type() {
        LengthType::Fixed => LayoutUnit::from_f32(length.value()).clamp_negative_to_zero(),
        LengthType::Percent => {
            if containing_block_size.is_indefinite() {
                LayoutUnit::zero()
            } else {
                LayoutUnit::from_f32(length.value() / 100.0 * containing_block_size.to_f32())
                    .clamp_negative_to_zero()
            }
        }
        LengthType::Auto | LengthType::None => LayoutUnit::zero(),
        // min-content, max-content, fit-content — treated as 0 for constraint
        // clamping; the intrinsic sizing path uses them directly.
        _ => LayoutUnit::zero(),
    }
}

/// Resolve a max-width / max-height CSS length.
///
/// `none` → LayoutUnit::max() (no upper constraint).
/// `auto` is treated the same as `none` for max-* properties.
fn resolve_max_length(length: &Length, containing_block_size: LayoutUnit) -> LayoutUnit {
    match length.length_type() {
        LengthType::Fixed => LayoutUnit::from_f32(length.value()).clamp_negative_to_zero(),
        LengthType::Percent => {
            if containing_block_size.is_indefinite() {
                LayoutUnit::max()
            } else {
                LayoutUnit::from_f32(length.value() / 100.0 * containing_block_size.to_f32())
                    .clamp_negative_to_zero()
            }
        }
        LengthType::None | LengthType::Auto => LayoutUnit::max(),
        // Intrinsic keywords in max-* properties: treat as none (unconstrained).
        _ => LayoutUnit::max(),
    }
}

/// Apply min/max inline-size constraints to a computed size.
///
/// CSS 2.1 §10.4: `min-width ≤ width ≤ max-width`.
/// When `min > max`, min wins (per spec).
#[inline]
pub fn constrain_inline_size(size: LayoutUnit, constraints: &SizeConstraint) -> LayoutUnit {
    size.max_of(constraints.min_inline_size)
        .min_of(constraints.max_inline_size)
}

/// Apply min/max block-size constraints to a computed size.
///
/// CSS 2.1 §10.7: `min-height ≤ height ≤ max-height`.
/// When `min > max`, min wins (per spec).
#[inline]
pub fn constrain_block_size(size: LayoutUnit, constraints: &SizeConstraint) -> LayoutUnit {
    size.max_of(constraints.min_block_size)
        .min_of(constraints.max_block_size)
}

/// Full inline size resolution: resolve `width`, apply box-sizing, then
/// clamp with min/max constraints.
///
/// CSS 2.1 §10.3 + §10.4:
/// - `auto` → use `available_inline_size` (block-level) or shrink-to-fit
/// - length/percentage → resolve against containing block
/// - Apply `box-sizing` adjustment
/// - Clamp with min/max constraints
pub fn resolve_inline_size(
    style_width: &Length,
    available_inline_size: LayoutUnit,
    cb_inline_size: LayoutUnit,
    box_sizing: BoxSizing,
    padding_border_inline: LayoutUnit,
    constraints: &SizeConstraint,
) -> LayoutUnit {
    let raw = resolve_length(
        style_width,
        cb_inline_size,
        available_inline_size,   // auto → fill available
        available_inline_size,   // none → fill available (shouldn't occur for width)
    );

    // Convert from border-box to content-box if needed.
    let content_size = if box_sizing == BoxSizing::BorderBox {
        apply_box_sizing_adjustment(raw, BoxSizing::BorderBox, padding_border_inline)
    } else {
        raw
    };

    constrain_inline_size(content_size, constraints)
}

/// Full block size resolution: resolve `height`, apply box-sizing, then
/// clamp with min/max constraints.
///
/// CSS 2.1 §10.5 + §10.7:
/// - `auto` → determined by content (caller supplies `content_block_size`)
/// - length → resolve directly
/// - percentage → resolve against containing block only if CB has definite height;
///   otherwise treated as auto
/// - Apply `box-sizing` adjustment
/// - Clamp with min/max constraints
pub fn resolve_block_size(
    style_height: &Length,
    content_block_size: LayoutUnit,
    cb_block_size: LayoutUnit,
    box_sizing: BoxSizing,
    padding_border_block: LayoutUnit,
    constraints: &SizeConstraint,
) -> LayoutUnit {
    let raw = match style_height.length_type() {
        LengthType::Auto => content_block_size,
        LengthType::Fixed => LayoutUnit::from_f32(style_height.value()),
        LengthType::Percent => {
            if cb_block_size.is_indefinite() {
                // CSS 2.1 §10.5: percentage height with indefinite CB → auto
                content_block_size
            } else {
                LayoutUnit::from_f32(
                    style_height.value() / 100.0 * cb_block_size.to_f32(),
                )
            }
        }
        // None / intrinsic keywords → content-based
        _ => content_block_size,
    };

    let content_size = if box_sizing == BoxSizing::BorderBox {
        apply_box_sizing_adjustment(raw, BoxSizing::BorderBox, padding_border_block)
    } else {
        raw
    };

    constrain_block_size(content_size, constraints)
}

/// Convert a size between content-box and border-box coordinate spaces.
///
/// - `BorderBox` → subtract `padding_and_border` (the specified value includes
///   padding+border, so we remove it to get content size).
/// - `ContentBox` → no adjustment (the value is already content-box).
///
/// The result is clamped to zero to prevent negative content sizes.
#[inline]
pub fn apply_box_sizing_adjustment(
    size: LayoutUnit,
    box_sizing: BoxSizing,
    padding_and_border: LayoutUnit,
) -> LayoutUnit {
    match box_sizing {
        BoxSizing::BorderBox => (size - padding_and_border).clamp_negative_to_zero(),
        BoxSizing::ContentBox => size,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lu(v: i32) -> LayoutUnit {
        LayoutUnit::from_i32(v)
    }

    #[test]
    fn unconstrained_defaults() {
        let c = SizeConstraint::unconstrained();
        assert_eq!(c.min_inline_size, LayoutUnit::zero());
        assert_eq!(c.max_inline_size, LayoutUnit::max());
        assert_eq!(c.min_block_size, LayoutUnit::zero());
        assert_eq!(c.max_block_size, LayoutUnit::max());
    }

    #[test]
    fn resolve_min_auto_is_zero() {
        let result = resolve_min_length(&Length::auto(), lu(500));
        assert_eq!(result, LayoutUnit::zero());
    }

    #[test]
    fn resolve_max_none_is_max() {
        let result = resolve_max_length(&Length::none(), lu(500));
        assert_eq!(result, LayoutUnit::max());
    }

    #[test]
    fn resolve_min_percent() {
        let result = resolve_min_length(&Length::percent(25.0), lu(400));
        assert_eq!(result.to_i32(), 100);
    }

    #[test]
    fn resolve_max_percent() {
        let result = resolve_max_length(&Length::percent(75.0), lu(400));
        assert_eq!(result.to_i32(), 300);
    }

    #[test]
    fn box_sizing_border_box_subtracts() {
        let result = apply_box_sizing_adjustment(lu(200), BoxSizing::BorderBox, lu(30));
        assert_eq!(result, lu(170));
    }

    #[test]
    fn box_sizing_content_box_no_change() {
        let result = apply_box_sizing_adjustment(lu(200), BoxSizing::ContentBox, lu(30));
        assert_eq!(result, lu(200));
    }

    #[test]
    fn box_sizing_border_box_clamps_negative() {
        let result = apply_box_sizing_adjustment(lu(10), BoxSizing::BorderBox, lu(30));
        assert_eq!(result, LayoutUnit::zero());
    }
}
