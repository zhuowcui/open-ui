//! Core ruby layout algorithm — computing column widths, offsets, and positions.
//!
//! A ruby "column" is a base–annotation pair. The column width is the wider of
//! the base and annotation, and the narrower element is offset according to the
//! `ruby-align` property.
//!
//! Reference: CSS Ruby Annotation Layout Module Level 1
//! <https://drafts.csswg.org/css-ruby-1/>
//!
//! Blink: `NGRubyUtils::ComputeRubyColumnInlineSize` and related helpers in
//! `ng_ruby_utils.cc`.

use openui_style::{RubyAlign, RubyPosition, WritingMode};

/// Layout data for a ruby annotation pair (base + annotation).
///
/// After layout, the base text is placed at `base_offset` from the start edge
/// of the column, and the annotation text at `annotation_offset`. The column
/// is `column_width` wide in the inline direction.
///
/// Blink: corresponds to the per-column metrics computed in
/// `LayoutRubyRun::Layout` and `NGRubyUtils`.
#[derive(Debug, Clone, PartialEq)]
pub struct RubyLayout {
    /// Width of the base text in the inline direction.
    pub base_width: f32,
    /// Width of the annotation text in the inline direction.
    pub annotation_width: f32,
    /// The column width: `max(base_width, annotation_width)`.
    /// This is the space reserved in the inline flow for this ruby pair.
    pub column_width: f32,
    /// Inline offset of the base text from the column start edge.
    pub base_offset: f32,
    /// Inline offset of the annotation text from the column start edge.
    pub annotation_offset: f32,
    /// Block size of the annotation (height for horizontal, width for vertical).
    /// Computed from the annotation font's approximate line height.
    pub annotation_size: f32,
    /// Block offset of the annotation relative to the base's start edge.
    /// Negative values mean the annotation is above/before the base (Over),
    /// zero or positive means below/after the base (Under).
    pub annotation_block_offset: f32,
}

/// Ruby annotation data attached to inline items.
///
/// Tracks the annotation text, font size, and CSS ruby properties for an
/// inline element that participates in ruby layout.
///
/// Blink: corresponds to the annotation metadata gathered by
/// `LayoutRubyRun::RubyBase` and `LayoutRubyRun::RubyText`.
#[derive(Debug, Clone, PartialEq)]
pub struct RubyInfo {
    /// The annotation text content (e.g., furigana kana).
    pub annotation_text: String,
    /// Font size of the annotation text in pixels.
    /// Typically half the base font size per CSS Ruby Level 1 §2.
    pub annotation_font_size: f32,
    /// Where the annotation is placed relative to the base.
    pub position: RubyPosition,
    /// How the annotation content is aligned within the column.
    pub align: RubyAlign,
}

/// Line-height multiplier for annotation text.
///
/// Blink uses the font metrics to compute exact line height, but for the
/// layout algorithm we approximate with a 1.2× multiplier on font size,
/// which is the CSS `normal` line-height value for most CJK fonts.
const ANNOTATION_LINE_HEIGHT_FACTOR: f32 = 1.2;

/// Compute the layout geometry for a single ruby base–annotation pair.
///
/// This is the core algorithm corresponding to Blink's
/// `NGRubyUtils::ComputeRubyColumnInlineSize` and the per-column layout in
/// `LayoutRubyRun::Layout`.
///
/// # Parameters
///
/// - `base_width`: measured inline size of the base text.
/// - `annotation_width`: measured inline size of the annotation text.
/// - `annotation_font_size`: the annotation text's `font-size` in pixels.
/// - `ruby_align`: the computed `ruby-align` value.
/// - `ruby_position`: the computed `ruby-position` value.
/// - `_writing_mode`: the computed `writing-mode` (reserved for future
///   vertical-specific adjustments).
///
/// # CSS Ruby Level 1 §4 — Alignment
///
/// - `space-around`: extra space is distributed equally before and after each
///   character in the narrower element, resulting in centering. For a single
///   run, this is equivalent to centering the narrower element.
/// - `center`: the narrower element is centered within the column.
/// - `space-between`: characters are spread to fill the column width with no
///   leading/trailing space; the element itself starts at offset 0.
/// - `start`: both elements are start-aligned (offset 0 for the narrower).
pub fn compute_ruby_layout(
    base_width: f32,
    annotation_width: f32,
    annotation_font_size: f32,
    ruby_align: RubyAlign,
    ruby_position: RubyPosition,
    _writing_mode: WritingMode,
) -> RubyLayout {
    // Sanitize negative inputs — widths and font sizes cannot be negative.
    let base_width = base_width.max(0.0);
    let annotation_width = annotation_width.max(0.0);
    let annotation_font_size = annotation_font_size.max(0.0);

    // The column occupies the wider of base and annotation.
    // CSS Ruby Level 1 §4: "The ruby container's inline size is at least as
    // large as the widest ruby base or ruby annotation box."
    let column_width = base_width.max(annotation_width);

    // Compute inline offsets based on ruby-align.
    let (base_offset, annotation_offset) = compute_alignment_offsets(
        base_width,
        annotation_width,
        column_width,
        ruby_align,
    );

    // Annotation block size ≈ annotation line height.
    let annotation_size = annotation_font_size * ANNOTATION_LINE_HEIGHT_FACTOR;

    // Block offset: how far the annotation sits from the base's start edge.
    // Over → annotation sits above the base (negative offset in block direction).
    // Under → annotation sits below the base (at offset 0, meaning flush with
    //         the base's after edge — the caller adds the base's block size).
    let annotation_block_offset = match ruby_position {
        RubyPosition::Over => -annotation_size,
        RubyPosition::Under => 0.0,
    };

    RubyLayout {
        base_width,
        annotation_width,
        column_width,
        base_offset,
        annotation_offset,
        annotation_size,
        annotation_block_offset,
    }
}

/// Compute the inline offsets for base and annotation within a ruby column.
///
/// CSS Ruby Level 1 §4.1 defines four alignment modes. For the column-level
/// offset calculation, `space-around` and `center` both center the narrower
/// element because the extra space distribution happens at the character level
/// during text layout. `space-between` and `start` both place the narrower
/// element at offset 0 because the stretching (or lack thereof) is handled
/// during text justification.
fn compute_alignment_offsets(
    base_width: f32,
    annotation_width: f32,
    column_width: f32,
    ruby_align: RubyAlign,
) -> (f32, f32) {
    match ruby_align {
        // Center or SpaceAround: the narrower element is centered.
        RubyAlign::Center | RubyAlign::SpaceAround => {
            let base_offset = (column_width - base_width) / 2.0;
            let annotation_offset = (column_width - annotation_width) / 2.0;
            (base_offset, annotation_offset)
        }
        // SpaceBetween: characters justified, element starts at 0.
        RubyAlign::SpaceBetween => (0.0, 0.0),
        // Start: both start-aligned.
        RubyAlign::Start => (0.0, 0.0),
    }
}

/// Maximum overhang per side for a ruby annotation.
///
/// When the annotation is wider than the base, it may overhang adjacent
/// characters by up to half the annotation font size on each side.
///
/// CSS Ruby Level 1 §4.3: "A ruby annotation may overhang adjacent content
/// on the line by up to half the advance width of one annotation character."
/// Blink approximates this as half the annotation font size.
///
/// Blink: `NGRubyUtils::MaxRubyOverhang` in `ng_ruby_utils.cc`.
///
/// # Returns
///
/// The maximum overhang in pixels per side (left and right, or before and
/// after in the inline direction).
pub fn max_ruby_overhang(annotation_font_size: f32) -> f32 {
    (annotation_font_size.max(0.0)) / 2.0
}

/// Clamp an overhang request to the available space and the maximum allowed.
///
/// In practice, the overhang on each side is limited to:
/// 1. Half the annotation font size (`max_ruby_overhang`).
/// 2. The amount of extra annotation width beyond the base.
/// 3. The available space from adjacent content.
///
/// Blink: `NGRubyUtils::ClampRubyOverhang`.
///
/// # Parameters
///
/// - `annotation_width`: inline size of the annotation text.
/// - `base_width`: inline size of the base text.
/// - `annotation_font_size`: annotation `font-size` for computing the max.
/// - `available_before`: available space before (start side) from adjacent content.
/// - `available_after`: available space after (end side) from adjacent content.
///
/// # Returns
///
/// `(overhang_before, overhang_after)` — the actual overhang to apply on
/// each side, clamped to all constraints.
pub fn clamp_overhang(
    annotation_width: f32,
    base_width: f32,
    annotation_font_size: f32,
    available_before: f32,
    available_after: f32,
) -> (f32, f32) {
    let excess = (annotation_width - base_width).max(0.0);
    if excess == 0.0 {
        return (0.0, 0.0);
    }

    let max_per_side = max_ruby_overhang(annotation_font_size);

    // Distribute the excess equally to both sides, then clamp.
    let half_excess = excess / 2.0;

    let overhang_before = half_excess.min(max_per_side).min(available_before.max(0.0));
    let overhang_after = half_excess.min(max_per_side).min(available_after.max(0.0));

    (overhang_before, overhang_after)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Column width ─────────────────────────────────────────────────

    #[test]
    fn column_width_is_max_of_base_and_annotation() {
        let r = compute_ruby_layout(50.0, 30.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 50.0);
    }

    #[test]
    fn column_width_annotation_wider() {
        let r = compute_ruby_layout(30.0, 50.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 50.0);
    }

    #[test]
    fn column_width_equal_widths() {
        let r = compute_ruby_layout(40.0, 40.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 40.0);
    }

    // ── Center alignment ─────────────────────────────────────────────

    #[test]
    fn center_short_annotation() {
        let r = compute_ruby_layout(60.0, 30.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 15.0); // (60-30)/2
    }

    #[test]
    fn center_wide_annotation() {
        let r = compute_ruby_layout(30.0, 60.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 15.0); // (60-30)/2
        assert_eq!(r.annotation_offset, 0.0);
    }

    #[test]
    fn center_equal_widths_zero_offsets() {
        let r = compute_ruby_layout(50.0, 50.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    // ── SpaceAround alignment ────────────────────────────────────────

    #[test]
    fn space_around_short_annotation() {
        let r = compute_ruby_layout(60.0, 30.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 15.0);
    }

    #[test]
    fn space_around_wide_annotation() {
        let r = compute_ruby_layout(30.0, 60.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 15.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    #[test]
    fn space_around_equal_widths() {
        let r = compute_ruby_layout(50.0, 50.0, 10.0, RubyAlign::SpaceAround, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    // ── SpaceBetween alignment ───────────────────────────────────────

    #[test]
    fn space_between_offsets_are_zero() {
        let r = compute_ruby_layout(60.0, 30.0, 10.0, RubyAlign::SpaceBetween, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    #[test]
    fn space_between_wide_annotation() {
        let r = compute_ruby_layout(30.0, 60.0, 10.0, RubyAlign::SpaceBetween, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    // ── Start alignment ──────────────────────────────────────────────

    #[test]
    fn start_offsets_are_zero() {
        let r = compute_ruby_layout(60.0, 30.0, 10.0, RubyAlign::Start, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    #[test]
    fn start_wide_annotation() {
        let r = compute_ruby_layout(30.0, 60.0, 10.0, RubyAlign::Start, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    // ── Over vs Under positioning ────────────────────────────────────

    #[test]
    fn over_position_negative_block_offset() {
        let r = compute_ruby_layout(50.0, 30.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert!(r.annotation_block_offset < 0.0);
        assert_eq!(r.annotation_block_offset, -12.0); // -(10 * 1.2)
    }

    #[test]
    fn under_position_zero_block_offset() {
        let r = compute_ruby_layout(50.0, 30.0, 10.0, RubyAlign::Center, RubyPosition::Under, WritingMode::HorizontalTb);
        assert_eq!(r.annotation_block_offset, 0.0);
    }

    #[test]
    fn over_position_with_large_font() {
        let r = compute_ruby_layout(50.0, 30.0, 24.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.annotation_block_offset, -(24.0 * 1.2));
        assert_eq!(r.annotation_size, 24.0 * 1.2);
    }

    #[test]
    fn under_position_annotation_size() {
        let r = compute_ruby_layout(50.0, 30.0, 16.0, RubyAlign::Center, RubyPosition::Under, WritingMode::HorizontalTb);
        assert_eq!(r.annotation_size, 16.0 * 1.2);
        assert_eq!(r.annotation_block_offset, 0.0);
    }

    // ── Annotation size ──────────────────────────────────────────────

    #[test]
    fn annotation_size_is_font_times_line_height() {
        let r = compute_ruby_layout(50.0, 30.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.annotation_size, 12.0);
    }

    #[test]
    fn annotation_size_zero_font() {
        let r = compute_ruby_layout(50.0, 30.0, 0.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.annotation_size, 0.0);
        assert_eq!(r.annotation_block_offset, 0.0); // -0.0 == 0.0
    }

    // ── Overhang calculation ─────────────────────────────────────────

    #[test]
    fn max_overhang_half_font_size() {
        assert_eq!(max_ruby_overhang(10.0), 5.0);
        assert_eq!(max_ruby_overhang(24.0), 12.0);
        assert_eq!(max_ruby_overhang(0.0), 0.0);
    }

    #[test]
    fn max_overhang_negative_font_clamped() {
        assert_eq!(max_ruby_overhang(-5.0), 0.0);
    }

    #[test]
    fn clamp_overhang_no_excess() {
        // Annotation narrower than base — no overhang needed.
        let (before, after) = clamp_overhang(30.0, 60.0, 10.0, 20.0, 20.0);
        assert_eq!(before, 0.0);
        assert_eq!(after, 0.0);
    }

    #[test]
    fn clamp_overhang_equal_widths() {
        let (before, after) = clamp_overhang(50.0, 50.0, 10.0, 20.0, 20.0);
        assert_eq!(before, 0.0);
        assert_eq!(after, 0.0);
    }

    #[test]
    fn clamp_overhang_basic_excess() {
        // Annotation 20px wider, excess = 20, half = 10, max = 5 (font 10/2).
        let (before, after) = clamp_overhang(60.0, 40.0, 10.0, 20.0, 20.0);
        assert_eq!(before, 5.0);
        assert_eq!(after, 5.0);
    }

    #[test]
    fn clamp_overhang_limited_by_available_before() {
        // half_excess = 10, max_per_side = 5, but only 3px available before.
        let (before, after) = clamp_overhang(60.0, 40.0, 10.0, 3.0, 20.0);
        assert_eq!(before, 3.0);
        assert_eq!(after, 5.0);
    }

    #[test]
    fn clamp_overhang_limited_by_available_after() {
        let (before, after) = clamp_overhang(60.0, 40.0, 10.0, 20.0, 2.0);
        assert_eq!(before, 5.0);
        assert_eq!(after, 2.0);
    }

    #[test]
    fn clamp_overhang_limited_by_both_sides() {
        let (before, after) = clamp_overhang(60.0, 40.0, 10.0, 1.0, 1.0);
        assert_eq!(before, 1.0);
        assert_eq!(after, 1.0);
    }

    #[test]
    fn clamp_overhang_zero_available() {
        let (before, after) = clamp_overhang(60.0, 40.0, 10.0, 0.0, 0.0);
        assert_eq!(before, 0.0);
        assert_eq!(after, 0.0);
    }

    #[test]
    fn clamp_overhang_large_font_limits() {
        // Excess = 20, half = 10, max per side = 12 (font 24 / 2).
        // Max doesn't limit, half_excess does.
        let (before, after) = clamp_overhang(60.0, 40.0, 24.0, 20.0, 20.0);
        assert_eq!(before, 10.0);
        assert_eq!(after, 10.0);
    }

    #[test]
    fn clamp_overhang_negative_available_treated_as_zero() {
        let (before, after) = clamp_overhang(60.0, 40.0, 10.0, -5.0, -5.0);
        assert_eq!(before, 0.0);
        assert_eq!(after, 0.0);
    }

    // ── Vertical writing mode ────────────────────────────────────────

    #[test]
    fn vertical_rl_produces_valid_layout() {
        let r = compute_ruby_layout(50.0, 30.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::VerticalRl);
        assert_eq!(r.column_width, 50.0);
        assert_eq!(r.annotation_offset, 10.0);
    }

    #[test]
    fn vertical_lr_produces_valid_layout() {
        let r = compute_ruby_layout(30.0, 50.0, 10.0, RubyAlign::Center, RubyPosition::Under, WritingMode::VerticalLr);
        assert_eq!(r.column_width, 50.0);
        assert_eq!(r.base_offset, 10.0);
        assert_eq!(r.annotation_block_offset, 0.0);
    }

    #[test]
    fn sideways_rl_produces_valid_layout() {
        let r = compute_ruby_layout(40.0, 40.0, 12.0, RubyAlign::SpaceAround, RubyPosition::Over, WritingMode::SidewaysRl);
        assert_eq!(r.column_width, 40.0);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    #[test]
    fn sideways_lr_produces_valid_layout() {
        let r = compute_ruby_layout(40.0, 60.0, 8.0, RubyAlign::Start, RubyPosition::Under, WritingMode::SidewaysLr);
        assert_eq!(r.column_width, 60.0);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    // ── Edge cases ───────────────────────────────────────────────────

    #[test]
    fn zero_width_base() {
        let r = compute_ruby_layout(0.0, 30.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 30.0);
        assert_eq!(r.base_offset, 15.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    #[test]
    fn zero_width_annotation() {
        let r = compute_ruby_layout(30.0, 0.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 30.0);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 15.0);
    }

    #[test]
    fn both_zero_width() {
        let r = compute_ruby_layout(0.0, 0.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 0.0);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
    }

    #[test]
    fn negative_widths_clamped_to_zero() {
        let r = compute_ruby_layout(-10.0, -20.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_width, 0.0);
        assert_eq!(r.annotation_width, 0.0);
        assert_eq!(r.column_width, 0.0);
    }

    #[test]
    fn negative_font_size_clamped() {
        let r = compute_ruby_layout(50.0, 30.0, -5.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.annotation_size, 0.0);
    }

    #[test]
    fn very_large_widths() {
        let r = compute_ruby_layout(10000.0, 5000.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 10000.0);
        assert_eq!(r.annotation_offset, 2500.0);
    }

    #[test]
    fn fractional_widths() {
        let r = compute_ruby_layout(33.3, 66.7, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 66.7);
        let expected_base_offset = (66.7 - 33.3) / 2.0;
        assert!((r.base_offset - expected_base_offset).abs() < 1e-5);
    }

    // ── RubyLayout field preservation ────────────────────────────────

    #[test]
    fn base_width_preserved() {
        let r = compute_ruby_layout(42.5, 30.0, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_width, 42.5);
    }

    #[test]
    fn annotation_width_preserved() {
        let r = compute_ruby_layout(30.0, 55.5, 10.0, RubyAlign::Center, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.annotation_width, 55.5);
    }

    // ── RubyInfo ─────────────────────────────────────────────────────

    #[test]
    fn ruby_info_construction() {
        let info = RubyInfo {
            annotation_text: "きょう".to_string(),
            annotation_font_size: 7.0,
            position: RubyPosition::Over,
            align: RubyAlign::SpaceAround,
        };
        assert_eq!(info.annotation_text, "きょう");
        assert_eq!(info.annotation_font_size, 7.0);
        assert_eq!(info.position, RubyPosition::Over);
        assert_eq!(info.align, RubyAlign::SpaceAround);
    }

    #[test]
    fn ruby_info_under_position() {
        let info = RubyInfo {
            annotation_text: "ㄊㄞˊ".to_string(),
            annotation_font_size: 6.0,
            position: RubyPosition::Under,
            align: RubyAlign::Center,
        };
        assert!(info.position.is_under());
        assert!(!info.position.is_over());
    }

    #[test]
    fn ruby_info_clone_eq() {
        let info = RubyInfo {
            annotation_text: "abc".to_string(),
            annotation_font_size: 8.0,
            position: RubyPosition::Over,
            align: RubyAlign::Start,
        };
        let cloned = info.clone();
        assert_eq!(info, cloned);
    }

    // ── Alignment combined with position ─────────────────────────────

    #[test]
    fn start_align_over_position() {
        let r = compute_ruby_layout(60.0, 30.0, 10.0, RubyAlign::Start, RubyPosition::Over, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
        assert!(r.annotation_block_offset < 0.0);
    }

    #[test]
    fn start_align_under_position() {
        let r = compute_ruby_layout(60.0, 30.0, 10.0, RubyAlign::Start, RubyPosition::Under, WritingMode::HorizontalTb);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
        assert_eq!(r.annotation_block_offset, 0.0);
    }

    #[test]
    fn space_between_under_position() {
        let r = compute_ruby_layout(40.0, 80.0, 12.0, RubyAlign::SpaceBetween, RubyPosition::Under, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 80.0);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 0.0);
        assert_eq!(r.annotation_block_offset, 0.0);
        assert_eq!(r.annotation_size, 12.0 * 1.2);
    }

    #[test]
    fn center_under_wide_base() {
        let r = compute_ruby_layout(100.0, 20.0, 8.0, RubyAlign::Center, RubyPosition::Under, WritingMode::HorizontalTb);
        assert_eq!(r.column_width, 100.0);
        assert_eq!(r.base_offset, 0.0);
        assert_eq!(r.annotation_offset, 40.0); // (100-20)/2
        assert_eq!(r.annotation_block_offset, 0.0);
    }

    // ── Overhang combined scenarios ──────────────────────────────────

    #[test]
    fn overhang_with_small_excess() {
        // Annotation 2px wider, excess=2, half=1, max=5 → clamp to 1.
        let (before, after) = clamp_overhang(42.0, 40.0, 10.0, 20.0, 20.0);
        assert_eq!(before, 1.0);
        assert_eq!(after, 1.0);
    }

    #[test]
    fn overhang_asymmetric_available() {
        // excess=40, half=20, max=10 (font 20 / 2).
        // before limited to 5 by available, after limited to max 10.
        let (before, after) = clamp_overhang(80.0, 40.0, 20.0, 5.0, 20.0);
        assert_eq!(before, 5.0);
        assert_eq!(after, 10.0);
    }

    #[test]
    fn overhang_zero_font_size() {
        // max overhang = 0, so no overhang even with excess.
        let (before, after) = clamp_overhang(60.0, 40.0, 0.0, 20.0, 20.0);
        assert_eq!(before, 0.0);
        assert_eq!(after, 0.0);
    }
}
