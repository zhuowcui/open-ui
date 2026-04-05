//! Text-combine-upright (tate-chū-yoko) layout.
//!
//! Extracted from Blink's `LayoutTextCombine` and related code in
//! `third_party/blink/renderer/core/layout/layout_text_combine.cc`.
//!
//! In vertical writing modes, `text-combine-upright: all` causes a run of
//! characters to be laid out horizontally and compressed (if necessary) to
//! fit within a single advance width of the surrounding vertical text. The
//! canonical example is two-digit numbers (e.g. "12") in a vertical column
//! for dates like "12月25日".
//!
//! ## Spec References
//!
//! - CSS Writing Modes Level 4 §9.1
//!   <https://www.w3.org/TR/css-writing-modes-4/#text-combine-upright>
//! - CSS Writing Modes Level 3 §9.1
//!   <https://www.w3.org/TR/css-writing-modes-3/#text-combine-upright>
//!
//! ## Algorithm
//!
//! 1. Measure the combined advance width of the horizontal text.
//! 2. If the combined width exceeds one em (the target width), compute a
//!    horizontal scale factor `scale_x = target / combined`.
//! 3. When painting, apply `canvas.scale(scale_x, 1.0)` to compress the
//!    text horizontally, then center the result within the character cell.
//! 4. The combined run occupies exactly one em in the block (vertical)
//!    direction, matching the advance of a single ideographic character.

use openui_style::{TextCombineUpright, WritingMode};

/// Layout data for a text-combine-upright (tate-chū-yoko) run.
///
/// Produced by [`compute_text_combine`] and consumed by the paint system
/// to apply the correct horizontal scaling and centering transforms.
///
/// Blink: `LayoutTextCombine` stores `scale_x_` and `compressed_font_`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextCombineLayout {
    /// The natural combined width (sum of individual glyph advances).
    pub combined_width: f32,
    /// The target width — one em of the current font size.
    pub target_width: f32,
    /// Horizontal scale factor: `target_width / combined_width` when
    /// compression is needed, otherwise 1.0.
    pub scale_x: f32,
    /// Whether the combined text needs horizontal compression.
    pub needs_compression: bool,
    /// Vertical offset to center the combined text within the line.
    /// In standard tate-chū-yoko the combined text is centered on the
    /// vertical midpoint, so this is typically 0.0.
    pub vertical_offset: f32,
    /// The horizontal offset to center the (possibly compressed) text
    /// within the target cell. Applied after scaling.
    pub centering_offset: f32,
}

/// Compute layout parameters for a text-combine-upright run.
///
/// Mirrors Blink's `LayoutTextCombine::ComputeTextCombine()` which
/// calculates whether compression is needed and the resulting scale.
///
/// # Arguments
///
/// * `text` — The text content being combined (used for length/emptiness checks).
/// * `combined_advance` — The natural horizontal advance of the shaped text.
/// * `font_size` — The current font size in pixels (defines one em).
/// * `writing_mode` — The writing mode of the containing block.
///
/// # Returns
///
/// A [`TextCombineLayout`] with the computed scale factor and offsets.
/// Returns a no-op layout (scale 1.0, no compression) when the writing
/// mode is horizontal or the text is empty.
pub fn compute_text_combine(
    text: &str,
    combined_advance: f32,
    font_size: f32,
    writing_mode: WritingMode,
) -> TextCombineLayout {
    // Tate-chū-yoko only applies in vertical writing modes.
    // In horizontal-tb, the property has no effect.
    if writing_mode.is_horizontal() || text.is_empty() || font_size <= 0.0 {
        return TextCombineLayout {
            combined_width: combined_advance,
            target_width: font_size,
            scale_x: 1.0,
            needs_compression: false,
            vertical_offset: 0.0,
            centering_offset: 0.0,
        };
    }

    // The target width is one em — a single character cell in the
    // vertical advance direction.
    let target_width = font_size;

    let needs_compression = combined_advance > target_width;
    let scale_x = if needs_compression {
        target_width / combined_advance
    } else {
        1.0
    };

    // When the (possibly scaled) text is narrower than the target, center
    // it horizontally within the cell.
    let actual_width = if needs_compression {
        target_width
    } else {
        combined_advance
    };
    let centering_offset = (target_width - actual_width) / 2.0;

    TextCombineLayout {
        combined_width: combined_advance,
        target_width,
        scale_x,
        needs_compression,
        vertical_offset: 0.0,
        centering_offset,
    }
}

/// Check whether text-combine-upright is active for the given property
/// value and writing mode.
///
/// Returns `true` only when the value is `All` *and* the writing mode is
/// vertical. In horizontal writing modes the property has no effect per
/// CSS Writing Modes Level 4 §9.1.
#[inline]
pub fn is_text_combine_active(
    text_combine: TextCombineUpright,
    writing_mode: WritingMode,
) -> bool {
    text_combine == TextCombineUpright::All && writing_mode.is_vertical()
}

/// Compute the inline advance that a combined run contributes to the
/// vertical line.
///
/// A tate-chū-yoko run always occupies exactly one em regardless of the
/// number of characters, matching the width of a single ideographic glyph.
///
/// Blink: `LayoutTextCombine::InlineAdvance()`.
#[inline]
pub fn combined_inline_advance(font_size: f32) -> f32 {
    font_size
}

/// Compute the paint transform parameters for rendering a combined run.
///
/// Returns `(translate_x, translate_y, scale_x)` to be applied to the
/// canvas before drawing the text blob.
///
/// The transform sequence (applied in order):
/// 1. Translate to the cell origin plus centering offset.
/// 2. Scale horizontally by `scale_x` to compress the text.
///
/// Blink: `TextCombinePainter::Paint()` in `text_combine_painter.cc`.
pub fn paint_transform(layout: &TextCombineLayout) -> (f32, f32, f32) {
    let translate_x = layout.centering_offset;
    let translate_y = layout.vertical_offset;
    let scale_x = layout.scale_x;
    (translate_x, translate_y, scale_x)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Basic computation ───────────────────────────────────────────

    #[test]
    fn two_digit_compression() {
        // "12" at 16px font, natural width 18px → compressed
        let layout = compute_text_combine("12", 18.0, 16.0, WritingMode::VerticalRl);
        assert!(layout.needs_compression);
        assert!((layout.scale_x - 16.0 / 18.0).abs() < 1e-6);
        assert_eq!(layout.target_width, 16.0);
    }

    #[test]
    fn no_compression_single_char() {
        // Single narrow char "1" at 16px, natural width 8px → fits
        let layout = compute_text_combine("1", 8.0, 16.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
        assert!((layout.centering_offset - 4.0).abs() < 1e-6); // (16-8)/2
    }

    #[test]
    fn exact_fit_no_compression() {
        // Text exactly fits one em
        let layout = compute_text_combine("AB", 16.0, 16.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
        assert_eq!(layout.centering_offset, 0.0);
    }

    #[test]
    fn target_width_equals_font_size() {
        let layout = compute_text_combine("99", 20.0, 24.0, WritingMode::VerticalRl);
        assert_eq!(layout.target_width, 24.0);
    }

    #[test]
    fn scale_factor_for_wide_text() {
        // "2024" at 16px, natural width 40px
        let layout = compute_text_combine("2024", 40.0, 16.0, WritingMode::VerticalRl);
        assert!(layout.needs_compression);
        assert!((layout.scale_x - 0.4).abs() < 1e-6);
    }

    #[test]
    fn combined_width_preserved() {
        let layout = compute_text_combine("AB", 22.5, 16.0, WritingMode::VerticalLr);
        assert_eq!(layout.combined_width, 22.5);
    }

    // ── Writing mode behaviour ──────────────────────────────────────

    #[test]
    fn horizontal_mode_no_effect() {
        let layout = compute_text_combine("12", 30.0, 16.0, WritingMode::HorizontalTb);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
    }

    #[test]
    fn vertical_rl_activates() {
        let layout = compute_text_combine("12", 30.0, 16.0, WritingMode::VerticalRl);
        assert!(layout.needs_compression);
    }

    #[test]
    fn vertical_lr_activates() {
        let layout = compute_text_combine("12", 30.0, 16.0, WritingMode::VerticalLr);
        assert!(layout.needs_compression);
    }

    #[test]
    fn sideways_rl_activates() {
        let layout = compute_text_combine("12", 30.0, 16.0, WritingMode::SidewaysRl);
        assert!(layout.needs_compression);
    }

    #[test]
    fn sideways_lr_activates() {
        let layout = compute_text_combine("12", 30.0, 16.0, WritingMode::SidewaysLr);
        assert!(layout.needs_compression);
    }

    // ── Edge cases ──────────────────────────────────────────────────

    #[test]
    fn empty_text() {
        let layout = compute_text_combine("", 0.0, 16.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
    }

    #[test]
    fn zero_font_size() {
        let layout = compute_text_combine("12", 10.0, 0.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
    }

    #[test]
    fn negative_font_size() {
        let layout = compute_text_combine("12", 10.0, -5.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
    }

    #[test]
    fn zero_advance() {
        let layout = compute_text_combine("X", 0.0, 16.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert!((layout.centering_offset - 8.0).abs() < 1e-6);
    }

    #[test]
    fn very_long_text_extreme_compression() {
        // 100px of text into 16px
        let layout = compute_text_combine("ABCDEFGHIJ", 100.0, 16.0, WritingMode::VerticalRl);
        assert!(layout.needs_compression);
        assert!((layout.scale_x - 0.16).abs() < 1e-6);
    }

    #[test]
    fn tiny_advance_large_centering() {
        let layout = compute_text_combine(".", 1.0, 16.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert!((layout.centering_offset - 7.5).abs() < 1e-6);
    }

    // ── is_text_combine_active ──────────────────────────────────────

    #[test]
    fn active_when_all_and_vertical() {
        assert!(is_text_combine_active(TextCombineUpright::All, WritingMode::VerticalRl));
    }

    #[test]
    fn inactive_when_none() {
        assert!(!is_text_combine_active(TextCombineUpright::None, WritingMode::VerticalRl));
    }

    #[test]
    fn inactive_when_horizontal() {
        assert!(!is_text_combine_active(TextCombineUpright::All, WritingMode::HorizontalTb));
    }

    #[test]
    fn inactive_when_none_and_horizontal() {
        assert!(!is_text_combine_active(TextCombineUpright::None, WritingMode::HorizontalTb));
    }

    // ── combined_inline_advance ─────────────────────────────────────

    #[test]
    fn inline_advance_equals_font_size() {
        assert_eq!(combined_inline_advance(16.0), 16.0);
        assert_eq!(combined_inline_advance(24.0), 24.0);
        assert_eq!(combined_inline_advance(10.5), 10.5);
    }

    // ── paint_transform ─────────────────────────────────────────────

    #[test]
    fn paint_transform_compressed() {
        let layout = compute_text_combine("12", 20.0, 16.0, WritingMode::VerticalRl);
        let (tx, ty, sx) = paint_transform(&layout);
        assert_eq!(tx, 0.0); // Fully compressed → no centering
        assert_eq!(ty, 0.0);
        assert!((sx - 0.8).abs() < 1e-6);
    }

    #[test]
    fn paint_transform_no_compression() {
        let layout = compute_text_combine("1", 8.0, 16.0, WritingMode::VerticalRl);
        let (tx, ty, sx) = paint_transform(&layout);
        assert!((tx - 4.0).abs() < 1e-6); // centered
        assert_eq!(ty, 0.0);
        assert_eq!(sx, 1.0);
    }

    #[test]
    fn paint_transform_horizontal_noop() {
        let layout = compute_text_combine("12", 20.0, 16.0, WritingMode::HorizontalTb);
        let (tx, _ty, sx) = paint_transform(&layout);
        // In horizontal mode, no compression, text may be centered
        assert_eq!(sx, 1.0);
        assert!(tx >= 0.0); // centering offset non-negative
    }

    // ── Centering offset ────────────────────────────────────────────

    #[test]
    fn centering_offset_when_narrower() {
        // 10px text in 16px cell → 3px on each side
        let layout = compute_text_combine("A", 10.0, 16.0, WritingMode::VerticalRl);
        assert!((layout.centering_offset - 3.0).abs() < 1e-6);
    }

    #[test]
    fn centering_offset_zero_when_exact() {
        let layout = compute_text_combine("AB", 16.0, 16.0, WritingMode::VerticalRl);
        assert_eq!(layout.centering_offset, 0.0);
    }

    #[test]
    fn centering_offset_zero_when_compressed() {
        // Compressed text fills the entire target width
        let layout = compute_text_combine("12", 20.0, 16.0, WritingMode::VerticalRl);
        assert_eq!(layout.centering_offset, 0.0);
    }

    // ── Vertical offset ─────────────────────────────────────────────

    #[test]
    fn vertical_offset_default_zero() {
        let layout = compute_text_combine("12", 20.0, 16.0, WritingMode::VerticalRl);
        assert_eq!(layout.vertical_offset, 0.0);
    }

    // ── TextCombineLayout fields ────────────────────────────────────

    #[test]
    fn layout_clone_and_copy() {
        let layout = compute_text_combine("12", 20.0, 16.0, WritingMode::VerticalRl);
        let cloned = layout;
        assert_eq!(layout, cloned);
    }

    #[test]
    fn layout_debug_format() {
        let layout = compute_text_combine("12", 20.0, 16.0, WritingMode::VerticalRl);
        let debug_str = format!("{:?}", layout);
        assert!(debug_str.contains("TextCombineLayout"));
    }

    // ── Realistic scenarios ─────────────────────────────────────────

    #[test]
    fn date_month_two_digits() {
        // "12" (two digits for month) at typical 16px body text
        let layout = compute_text_combine("12", 17.78, 16.0, WritingMode::VerticalRl);
        assert!(layout.needs_compression);
        assert!(layout.scale_x > 0.0 && layout.scale_x < 1.0);
    }

    #[test]
    fn date_day_single_digit() {
        // "5" (single digit) at 16px — fits without compression
        let layout = compute_text_combine("5", 8.89, 16.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
    }

    #[test]
    fn year_four_digits() {
        // "2024" at 16px — needs heavy compression
        let layout = compute_text_combine("2024", 35.56, 16.0, WritingMode::VerticalRl);
        assert!(layout.needs_compression);
        assert!(layout.scale_x < 0.5);
    }

    #[test]
    fn large_font_size_less_compression() {
        // Same text at 48px font — may not need compression
        let layout = compute_text_combine("12", 17.78, 48.0, WritingMode::VerticalRl);
        assert!(!layout.needs_compression);
        assert_eq!(layout.scale_x, 1.0);
    }

    #[test]
    fn fractional_font_size() {
        let layout = compute_text_combine("AB", 20.0, 13.5, WritingMode::VerticalRl);
        assert!(layout.needs_compression);
        assert!((layout.scale_x - 13.5 / 20.0).abs() < 1e-6);
        assert_eq!(layout.target_width, 13.5);
    }
}
