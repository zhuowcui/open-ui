//! Length resolution — converting CSS Length values to LayoutUnit.
//!
//! Extracted from Blink's `length_utils.h` / `length_utils.cc`.
//! This is where percentage, auto, and fixed lengths get resolved against
//! a containing block dimension.

use openui_geometry::{LayoutUnit, Length, LengthType};

/// Resolve a CSS Length to a concrete LayoutUnit value.
///
/// Matches Blink's `ResolveInlineLength()` / `ResolveBlockLength()` logic.
///
/// - `Fixed` → the pixel value directly.
/// - `Percent` → `(percentage / 100) * containing_block_size`.
/// - `Auto` → returns `auto_value` (caller decides what auto means).
/// - `None` → returns `none_value` (for max-width/max-height, typically unconstrained).
pub fn resolve_length(
    length: &Length,
    containing_block_size: LayoutUnit,
    auto_value: LayoutUnit,
    none_value: LayoutUnit,
) -> LayoutUnit {
    match length.length_type() {
        LengthType::Fixed => LayoutUnit::from_f32(length.value()),
        LengthType::Percent => {
            if containing_block_size.is_indefinite() {
                // Percentage against indefinite containing block → auto
                auto_value
            } else {
                // Blink: (percentage / 100) * containing_block_size
                LayoutUnit::from_f32(length.value() / 100.0 * containing_block_size.to_f32())
            }
        }
        LengthType::Auto => auto_value,
        LengthType::None => none_value,
        // Intrinsic sizes and calc() will be implemented in later SPs.
        _ => auto_value,
    }
}

/// Resolve a padding/margin length. Same as `resolve_length` but auto is
/// resolved against the inline size (for margins) or treated as 0 (for padding).
pub fn resolve_margin_or_padding(
    length: &Length,
    containing_inline_size: LayoutUnit,
) -> LayoutUnit {
    match length.length_type() {
        LengthType::Fixed => LayoutUnit::from_f32(length.value()),
        LengthType::Percent => {
            if containing_inline_size.is_indefinite() {
                LayoutUnit::zero()
            } else {
                LayoutUnit::from_f32(length.value() / 100.0 * containing_inline_size.to_f32())
            }
        }
        // Auto margins are resolved later during layout (centering, etc.)
        // For now return 0 — the block layout algorithm handles auto margins.
        LengthType::Auto => LayoutUnit::zero(),
        _ => LayoutUnit::zero(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_fixed() {
        let l = Length::px(100.0);
        let result = resolve_length(&l, LayoutUnit::from_i32(500), LayoutUnit::zero(), LayoutUnit::max());
        assert_eq!(result.to_i32(), 100);
    }

    #[test]
    fn resolve_percent() {
        let l = Length::percent(50.0);
        let result = resolve_length(&l, LayoutUnit::from_i32(400), LayoutUnit::zero(), LayoutUnit::max());
        assert_eq!(result.to_i32(), 200);
    }

    #[test]
    fn resolve_auto() {
        let l = Length::auto();
        let result = resolve_length(&l, LayoutUnit::from_i32(400), LayoutUnit::from_i32(999), LayoutUnit::max());
        assert_eq!(result.to_i32(), 999);
    }

    #[test]
    fn resolve_none() {
        let l = Length::none();
        let result = resolve_length(&l, LayoutUnit::from_i32(400), LayoutUnit::zero(), LayoutUnit::max());
        assert_eq!(result, LayoutUnit::max());
    }

    #[test]
    fn resolve_percent_against_indefinite() {
        let l = Length::percent(50.0);
        let indef = LayoutUnit::from_raw(-64); // kIndefiniteSize
        let result = resolve_length(&l, indef, LayoutUnit::from_i32(42), LayoutUnit::max());
        // Percentage against indefinite → auto value
        assert_eq!(result.to_i32(), 42);
    }
}
