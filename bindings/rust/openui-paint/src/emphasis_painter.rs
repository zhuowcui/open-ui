//! Emphasis mark painting — CSS Text Decoration Module Level 3 §3.
//!
//! Source: Blink `text_painter.cc`, `text_decoration_info.cc`.
//! Spec: <https://www.w3.org/TR/css-text-decor-3/#text-emphasis-style-property>
//!
//! Emphasis marks are small symbols (dots, circles, sesames, etc.) drawn
//! above or below each grapheme cluster to indicate emphasis. The mark
//! character is painted at 50% of the text font size, centered horizontally
//! over each character.
//!
//! ## Paint order
//!
//! Emphasis marks are painted after text glyphs and before line-through
//! decorations, matching Blink's `TextFragmentPainter::Paint()` ordering.
//!
//! ## Placement algorithm
//!
//! In horizontal writing mode:
//! - **over**: marks are placed above the ascender line with a proportional gap
//! - **under**: marks are placed below the descender line with a proportional gap
//!
//! In vertical writing mode:
//! - **right**: marks are placed to the right of the text
//! - **left**: marks are placed to the left of the text
//!
//! Characters that are whitespace, control, or format characters (except
//! soft hyphen) do not receive emphasis marks, per the CSS spec and
//! Blink's `ShouldDrawEmphasisMark()`.

use skia_safe::{Canvas, ColorSpace, Font as SkFont, Paint, PaintStyle, Point};

use openui_style::{Color, ComputedStyle, StyleColor, TextEmphasisMark, WritingMode};
use openui_text::emphasis::should_draw_emphasis_mark;
use openui_text::shaping::ShapeResult;

use crate::text_painter::to_sk_color4f;

/// Emphasis font size as a fraction of the text font size.
///
/// Blink renders emphasis marks at 50% of the text font size.
/// Reference: `TextDecorationInfo::SetEmphasisMarkInfo()` in
/// `core/paint/text_decoration_info.cc`.
pub const EMPHASIS_FONT_SIZE_RATIO: f32 = 0.5;

/// Gap between emphasis mark and text as a fraction of the emphasis font size.
///
/// Blink uses approximately 15% of the emphasis font size as spacing between
/// the text's ascender/descender and the emphasis mark. This prevents marks
/// from overlapping with the text glyphs.
pub const EMPHASIS_GAP_RATIO: f32 = 0.15;

/// Paint emphasis marks for each grapheme cluster in a text run.
///
/// Mirrors Blink's emphasis mark painting from `TextPainter::PaintEmphasisMark()`
/// (`core/paint/text_painter.cc`).
///
/// # Arguments
/// * `canvas` — Skia raster canvas
/// * `shape_result` — Shaped text for glyph positions and advances
/// * `origin` — (x, baseline_y) in device pixels
/// * `style` — Computed style with emphasis properties
/// * `text_content` — Original text for character iteration and filtering
pub fn paint_emphasis_marks(
    canvas: &Canvas,
    shape_result: &ShapeResult,
    origin: (f32, f32),
    style: &ComputedStyle,
    text_content: Option<&str>,
) {
    if style.text_emphasis_mark == TextEmphasisMark::None {
        return;
    }

    let text = match text_content {
        Some(t) if !t.is_empty() => t,
        _ => return,
    };

    let emphasis_char = match style.text_emphasis_mark.character(style.text_emphasis_fill) {
        Some(ch) => ch,
        None => return,
    };

    let emphasis_str = emphasis_char.to_string();

    let text_font_size = style.font_size;
    let emphasis_font_size = text_font_size * EMPHASIS_FONT_SIZE_RATIO;
    if emphasis_font_size <= 0.0 {
        return;
    }

    // Create a Skia font at the emphasis size using the same typeface as the text.
    let emphasis_sk_font = match create_emphasis_font(shape_result, emphasis_font_size) {
        Some(f) => f,
        None => return,
    };

    // Measure the emphasis mark width for horizontal centering over each character.
    let (mark_width, _) = emphasis_sk_font.measure_str(&emphasis_str, None);

    let color = resolve_emphasis_color(&style.text_emphasis_color, &style.color);

    let mut paint = Paint::default();
    paint.set_anti_alias(true);
    paint.set_style(PaintStyle::Fill);
    paint.set_color4f(to_sk_color4f(&color), None::<&ColorSpace>);

    let mark_offset_y = compute_emphasis_offset(
        style.text_emphasis_position,
        text_font_size,
        emphasis_font_size,
        style.writing_mode,
    );

    let (base_x, baseline_y) = origin;

    for (char_idx, ch) in text.chars().enumerate() {
        if !should_draw_emphasis_mark(ch) {
            continue;
        }

        if char_idx >= shape_result.num_characters {
            break;
        }

        let char_x = shape_result.x_position_for_offset(char_idx);
        let char_advance = char_advance_for_emphasis(shape_result, char_idx);

        // Center the emphasis mark horizontally over the character.
        let mark_x = base_x + char_x + (char_advance - mark_width) / 2.0;
        let mark_y = baseline_y + mark_offset_y;

        canvas.draw_str(
            &emphasis_str,
            Point::new(mark_x, mark_y),
            &emphasis_sk_font,
            &paint,
        );
    }
}

/// Compute the advance width for a character at `char_idx` for emphasis mark centering.
///
/// Uses the difference between consecutive character x-positions from the
/// shape result's character data. For the last character, the total width
/// is used as the end boundary.
pub fn char_advance_for_emphasis(shape_result: &ShapeResult, char_idx: usize) -> f32 {
    let x_start = shape_result.x_position_for_offset(char_idx);
    let x_end = if char_idx + 1 < shape_result.num_characters {
        shape_result.x_position_for_offset(char_idx + 1)
    } else {
        shape_result.width
    };
    (x_end - x_start).abs()
}

/// Create a Skia font for emphasis marks at the given size.
///
/// Clones the typeface from the first shaping run and creates a new `SkFont`
/// at the reduced emphasis size. The font is configured with subpixel
/// positioning and slight hinting to match the text font configuration.
fn create_emphasis_font(shape_result: &ShapeResult, size: f32) -> Option<SkFont> {
    let first_run = shape_result.runs.first()?;
    let typeface = first_run.font_data.typeface().clone();
    let mut font = SkFont::from_typeface(typeface, size);
    font.set_subpixel(true);
    font.set_hinting(skia_safe::FontHinting::Slight);
    Some(font)
}

/// Compute vertical offset for emphasis mark placement relative to the text baseline.
///
/// Returns the Y offset (horizontal mode) or X offset (vertical mode) to
/// add to the baseline position to get the emphasis mark drawing position.
///
/// ## Horizontal writing mode
///
/// **Over** (above text): The mark is placed above the text's ascender line.
/// The offset is negative (upward from baseline). The ascender height is
/// approximated as 80% of the font size, which matches typical Latin/CJK
/// fonts. A proportional gap separates the ascender from the mark.
///
/// **Under** (below text): The mark is placed below the text's descender line.
/// The offset is positive (downward from baseline). The descender depth is
/// approximated as 20% of the font size.
///
/// ## Vertical writing mode
///
/// **Right**: The mark is placed to the right of the text centerline.
/// **Left**: The mark is placed to the left of the text centerline.
/// The character width is approximated as 60% of the font size for the
/// offset from the text center.
///
/// Blink reference: `TextDecorationInfo::SetEmphasisMarkTextPosition()`.
pub fn compute_emphasis_offset(
    position: openui_style::TextEmphasisPosition,
    text_font_size: f32,
    emphasis_font_size: f32,
    writing_mode: WritingMode,
) -> f32 {
    let gap = emphasis_font_size * EMPHASIS_GAP_RATIO;

    if writing_mode.is_horizontal() {
        if position.over {
            -(text_font_size * 0.8 + gap + emphasis_font_size * 0.5)
        } else {
            text_font_size * 0.2 + gap + emphasis_font_size * 0.5
        }
    } else {
        if position.right {
            text_font_size * 0.6 + gap
        } else {
            -(text_font_size * 0.6 + gap + emphasis_font_size)
        }
    }
}

/// Resolve the emphasis mark color from the `text-emphasis-color` property.
///
/// Per CSS Text Decoration Level 3 §3.6, the initial value of
/// `text-emphasis-color` is `currentColor`, which resolves to the
/// element's computed `color` property value.
pub fn resolve_emphasis_color(emphasis_color: &StyleColor, text_color: &Color) -> Color {
    match emphasis_color {
        StyleColor::Resolved(c) => *c,
        StyleColor::CurrentColor => *text_color,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::{
        Color, StyleColor, TextEmphasisFill, TextEmphasisMark, TextEmphasisPosition, WritingMode,
    };
    use openui_text::shaping::{ShapeResult, ShapeResultCharacterData, TextDirection};

    // ── Helper: build a minimal ShapeResult with uniform character advances ──

    fn make_shape_result(num_chars: usize, char_advance: f32) -> ShapeResult {
        let mut character_data = Vec::with_capacity(num_chars);
        for i in 0..num_chars {
            character_data.push(ShapeResultCharacterData {
                x_position: i as f32 * char_advance,
                is_cluster_base: true,
                safe_to_break_before: i == 0,
            });
        }
        ShapeResult {
            runs: Vec::new(),
            width: num_chars as f32 * char_advance,
            num_characters: num_chars,
            direction: TextDirection::Ltr,
            character_data,
        }
    }

    fn make_shape_result_variable(advances: &[f32]) -> ShapeResult {
        let num_chars = advances.len();
        let mut character_data = Vec::with_capacity(num_chars);
        let mut x = 0.0_f32;
        for (i, &adv) in advances.iter().enumerate() {
            character_data.push(ShapeResultCharacterData {
                x_position: x,
                is_cluster_base: true,
                safe_to_break_before: i == 0,
            });
            x += adv;
        }
        ShapeResult {
            runs: Vec::new(),
            width: x,
            num_characters: num_chars,
            direction: TextDirection::Ltr,
            character_data,
        }
    }

    // ── Constants ──────────────────────────────────────────────────────

    #[test]
    fn emphasis_font_size_ratio_is_half() {
        assert_eq!(EMPHASIS_FONT_SIZE_RATIO, 0.5);
    }

    #[test]
    fn emphasis_gap_ratio_value() {
        assert!((EMPHASIS_GAP_RATIO - 0.15).abs() < f32::EPSILON);
    }

    // ── compute_emphasis_offset: horizontal over ──────────────────────

    #[test]
    fn horizontal_over_offset_is_negative() {
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::HorizontalTb,
        );
        assert!(offset < 0.0, "over offset should be negative (above baseline)");
    }

    #[test]
    fn horizontal_over_offset_value() {
        // font_size=20, emphasis=10, gap=10*0.15=1.5
        // offset = -(20*0.8 + 1.5 + 10*0.5) = -(16 + 1.5 + 5) = -22.5
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            20.0,
            10.0,
            WritingMode::HorizontalTb,
        );
        assert!((offset - (-22.5)).abs() < 0.001);
    }

    // ── compute_emphasis_offset: horizontal under ─────────────────────

    #[test]
    fn horizontal_under_offset_is_positive() {
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: false, right: true },
            16.0,
            8.0,
            WritingMode::HorizontalTb,
        );
        assert!(offset > 0.0, "under offset should be positive (below baseline)");
    }

    #[test]
    fn horizontal_under_offset_value() {
        // font_size=20, emphasis=10, gap=10*0.15=1.5
        // offset = 20*0.2 + 1.5 + 10*0.5 = 4 + 1.5 + 5 = 10.5
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: false, right: true },
            20.0,
            10.0,
            WritingMode::HorizontalTb,
        );
        assert!((offset - 10.5).abs() < 0.001);
    }

    // ── compute_emphasis_offset: vertical right/left ──────────────────

    #[test]
    fn vertical_right_offset_is_positive() {
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::VerticalRl,
        );
        assert!(offset > 0.0, "right offset should be positive");
    }

    #[test]
    fn vertical_right_offset_value() {
        // font_size=20, emphasis=10, gap=10*0.15=1.5
        // offset = 20*0.6 + 1.5 = 12 + 1.5 = 13.5
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            20.0,
            10.0,
            WritingMode::VerticalRl,
        );
        assert!((offset - 13.5).abs() < 0.001);
    }

    #[test]
    fn vertical_left_offset_is_negative() {
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: false, right: false },
            16.0,
            8.0,
            WritingMode::VerticalRl,
        );
        assert!(offset < 0.0, "left offset should be negative");
    }

    #[test]
    fn vertical_left_offset_value() {
        // font_size=20, emphasis=10, gap=10*0.15=1.5
        // offset = -(20*0.6 + 1.5 + 10) = -(12 + 1.5 + 10) = -23.5
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: false, right: false },
            20.0,
            10.0,
            WritingMode::VerticalRl,
        );
        assert!((offset - (-23.5)).abs() < 0.001);
    }

    // ── compute_emphasis_offset: different writing modes ──────────────

    #[test]
    fn vertical_lr_uses_vertical_offsets() {
        let offset_vrl = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::VerticalRl,
        );
        let offset_vlr = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::VerticalLr,
        );
        assert_eq!(offset_vrl, offset_vlr, "both vertical modes use vertical offset logic");
    }

    #[test]
    fn sideways_rl_uses_vertical_offsets() {
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::SidewaysRl,
        );
        assert!(offset > 0.0, "sideways-rl is vertical, right should be positive");
    }

    #[test]
    fn sideways_lr_uses_vertical_offsets() {
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::SidewaysLr,
        );
        assert!(offset > 0.0, "sideways-lr is vertical, right should be positive");
    }

    // ── compute_emphasis_offset: scaling ──────────────────────────────

    #[test]
    fn larger_font_produces_larger_over_offset() {
        let small = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            12.0,
            6.0,
            WritingMode::HorizontalTb,
        );
        let large = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            24.0,
            12.0,
            WritingMode::HorizontalTb,
        );
        assert!(
            large.abs() > small.abs(),
            "larger font should produce larger offset magnitude"
        );
    }

    #[test]
    fn zero_font_size_produces_zero_offset() {
        let offset = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            0.0,
            0.0,
            WritingMode::HorizontalTb,
        );
        assert_eq!(offset, 0.0);
    }

    #[test]
    fn over_and_under_offsets_differ_in_sign() {
        let over = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::HorizontalTb,
        );
        let under = compute_emphasis_offset(
            TextEmphasisPosition { over: false, right: true },
            16.0,
            8.0,
            WritingMode::HorizontalTb,
        );
        assert!(over < 0.0);
        assert!(under > 0.0);
    }

    #[test]
    fn right_and_left_offsets_differ_in_sign() {
        let right = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::VerticalRl,
        );
        let left = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: false },
            16.0,
            8.0,
            WritingMode::VerticalRl,
        );
        assert!(right > 0.0);
        assert!(left < 0.0);
    }

    // ── resolve_emphasis_color ────────────────────────────────────────

    #[test]
    fn current_color_resolves_to_text_color() {
        let text_color = Color::RED;
        let result = resolve_emphasis_color(&StyleColor::CurrentColor, &text_color);
        assert_eq!(result, Color::RED);
    }

    #[test]
    fn resolved_color_uses_explicit_value() {
        let text_color = Color::BLACK;
        let blue = Color::BLUE;
        let result = resolve_emphasis_color(&StyleColor::Resolved(blue), &text_color);
        assert_eq!(result, Color::BLUE);
    }

    #[test]
    fn current_color_with_custom_text_color() {
        let custom = Color::from_rgba8(128, 64, 32, 255);
        let result = resolve_emphasis_color(&StyleColor::CurrentColor, &custom);
        assert_eq!(result.r, custom.r);
        assert_eq!(result.g, custom.g);
        assert_eq!(result.b, custom.b);
        assert_eq!(result.a, custom.a);
    }

    #[test]
    fn resolved_transparent_color() {
        let result =
            resolve_emphasis_color(&StyleColor::Resolved(Color::TRANSPARENT), &Color::BLACK);
        assert_eq!(result, Color::TRANSPARENT);
    }

    // ── char_advance_for_emphasis ─────────────────────────────────────

    #[test]
    fn advance_first_character_uniform() {
        let sr = make_shape_result(5, 10.0);
        let adv = char_advance_for_emphasis(&sr, 0);
        assert!((adv - 10.0).abs() < 0.001);
    }

    #[test]
    fn advance_middle_character_uniform() {
        let sr = make_shape_result(5, 10.0);
        let adv = char_advance_for_emphasis(&sr, 2);
        assert!((adv - 10.0).abs() < 0.001);
    }

    #[test]
    fn advance_last_character_uses_width() {
        let sr = make_shape_result(5, 10.0);
        let adv = char_advance_for_emphasis(&sr, 4);
        // last char: width(50) - x_position(40) = 10
        assert!((adv - 10.0).abs() < 0.001);
    }

    #[test]
    fn advance_single_character() {
        let sr = make_shape_result(1, 12.5);
        let adv = char_advance_for_emphasis(&sr, 0);
        assert!((adv - 12.5).abs() < 0.001);
    }

    #[test]
    fn advance_variable_widths() {
        let sr = make_shape_result_variable(&[8.0, 12.0, 6.0, 14.0]);
        assert!((char_advance_for_emphasis(&sr, 0) - 8.0).abs() < 0.001);
        assert!((char_advance_for_emphasis(&sr, 1) - 12.0).abs() < 0.001);
        assert!((char_advance_for_emphasis(&sr, 2) - 6.0).abs() < 0.001);
        assert!((char_advance_for_emphasis(&sr, 3) - 14.0).abs() < 0.001);
    }

    #[test]
    fn advance_empty_shape_result() {
        let sr = make_shape_result(0, 0.0);
        // x_position_for_offset returns 0.0 for empty, width is 0.0
        let adv = char_advance_for_emphasis(&sr, 0);
        assert_eq!(adv, 0.0);
    }

    // ── Emphasis font size computation ────────────────────────────────

    #[test]
    fn emphasis_size_is_half_of_text_size() {
        let text_size = 32.0;
        let emphasis_size = text_size * EMPHASIS_FONT_SIZE_RATIO;
        assert!((emphasis_size - 16.0).abs() < f32::EPSILON);
    }

    #[test]
    fn emphasis_size_for_small_font() {
        let text_size = 8.0;
        let emphasis_size = text_size * EMPHASIS_FONT_SIZE_RATIO;
        assert!((emphasis_size - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn emphasis_size_for_zero_font() {
        let text_size = 0.0;
        let emphasis_size = text_size * EMPHASIS_FONT_SIZE_RATIO;
        assert_eq!(emphasis_size, 0.0);
    }

    // ── Mark centering computation ────────────────────────────────────

    #[test]
    fn mark_centered_when_narrower_than_character() {
        let char_advance: f32 = 20.0;
        let mark_width: f32 = 10.0;
        let offset = (char_advance - mark_width) / 2.0;
        assert!((offset - 5.0).abs() < f32::EPSILON);
    }

    #[test]
    fn mark_centered_when_same_width_as_character() {
        let char_advance = 10.0;
        let mark_width = 10.0;
        let offset = (char_advance - mark_width) / 2.0;
        assert_eq!(offset, 0.0);
    }

    #[test]
    fn mark_offset_negative_when_wider_than_character() {
        let char_advance = 8.0;
        let mark_width = 12.0;
        let offset = (char_advance - mark_width) / 2.0;
        assert!(offset < 0.0, "mark wider than char should have negative centering offset");
    }

    // ── should_draw_emphasis_mark filtering (re-exported, verify integration) ──

    #[test]
    fn draw_on_cjk_character() {
        assert!(should_draw_emphasis_mark('漢'));
    }

    #[test]
    fn draw_on_latin_letter() {
        assert!(should_draw_emphasis_mark('A'));
    }

    #[test]
    fn skip_space_character() {
        assert!(!should_draw_emphasis_mark(' '));
    }

    #[test]
    fn skip_tab_character() {
        assert!(!should_draw_emphasis_mark('\t'));
    }

    #[test]
    fn skip_newline_character() {
        assert!(!should_draw_emphasis_mark('\n'));
    }

    #[test]
    fn skip_zero_width_space() {
        assert!(!should_draw_emphasis_mark('\u{200B}'));
    }

    #[test]
    fn draw_on_punctuation_character() {
        assert!(should_draw_emphasis_mark('!'));
    }

    #[test]
    fn soft_hyphen_draws_mark() {
        assert!(should_draw_emphasis_mark('\u{00AD}'));
    }

    // ── Emphasis mark character resolution (via style enums) ──────────

    #[test]
    fn filled_dot_character() {
        assert_eq!(
            TextEmphasisMark::Dot.character(TextEmphasisFill::Filled),
            Some('\u{2022}')
        );
    }

    #[test]
    fn open_sesame_character() {
        assert_eq!(
            TextEmphasisMark::Sesame.character(TextEmphasisFill::Open),
            Some('\u{FE46}')
        );
    }

    #[test]
    fn none_mark_returns_no_character() {
        assert_eq!(TextEmphasisMark::None.character(TextEmphasisFill::Filled), None);
    }

    #[test]
    fn custom_mark_ignores_fill() {
        let mark = TextEmphasisMark::Custom('★');
        assert_eq!(mark.character(TextEmphasisFill::Filled), Some('★'));
        assert_eq!(mark.character(TextEmphasisFill::Open), Some('★'));
    }

    // ── Gap proportionality ──────────────────────────────────────────

    #[test]
    fn gap_scales_with_emphasis_size() {
        let gap_small = 6.0 * EMPHASIS_GAP_RATIO;
        let gap_large = 12.0 * EMPHASIS_GAP_RATIO;
        assert!((gap_large - gap_small * 2.0).abs() < f32::EPSILON);
    }

    // ── Offset symmetry properties ───────────────────────────────────

    #[test]
    fn horizontal_over_magnitude_exceeds_under() {
        // Over offset must clear the ascender (~80% of font), while under
        // only needs to clear the descender (~20% of font).
        let over_mag = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::HorizontalTb,
        )
        .abs();
        let under_mag = compute_emphasis_offset(
            TextEmphasisPosition { over: false, right: true },
            16.0,
            8.0,
            WritingMode::HorizontalTb,
        )
        .abs();
        assert!(
            over_mag > under_mag,
            "over offset magnitude ({over_mag}) should exceed under ({under_mag})"
        );
    }

    #[test]
    fn vertical_left_magnitude_exceeds_right() {
        // Left offset includes the emphasis mark width; right does not.
        let right_mag = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: true },
            16.0,
            8.0,
            WritingMode::VerticalRl,
        )
        .abs();
        let left_mag = compute_emphasis_offset(
            TextEmphasisPosition { over: true, right: false },
            16.0,
            8.0,
            WritingMode::VerticalRl,
        )
        .abs();
        assert!(
            left_mag > right_mag,
            "left offset magnitude ({left_mag}) should exceed right ({right_mag})"
        );
    }
}
