//! Text emphasis mark resolution — CSS Text Decoration Module Level 3 §3.
//!
//! Resolves `text-emphasis-style` + `text-emphasis-fill` into the actual
//! Unicode character to render above or below each base character.
//!
//! Source: Blink `text_decoration_info.cc`, `emphasis_mark_info.cc`.
//! Spec: <https://www.w3.org/TR/css-text-decor-3/#text-emphasis-style-property>

use openui_style::{TextEmphasisFill, TextEmphasisMark, TextEmphasisPosition, WritingMode};

/// Resolved emphasis mark: the character to paint and its position.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedEmphasisMark {
    /// The Unicode character to render as the emphasis mark.
    pub character: char,
    /// Whether to place the mark above (`true`) or below (`false`) in
    /// horizontal writing mode, or right/left in vertical.
    pub over: bool,
}

/// Resolves the emphasis mark shape, fill, and position into a concrete character.
///
/// Returns `None` if `mark` is `TextEmphasisMark::None`.
///
/// Per CSS Text Decoration Level 3 §3.4, when no explicit shape is given
/// (i.e., only `filled` or `open` is specified), the default shape depends
/// on the writing mode:
/// - Horizontal text → dot
/// - Vertical text → sesame
///
/// This function handles that default via `default_mark_for_writing_mode`.
pub fn resolve_emphasis_mark(
    mark: TextEmphasisMark,
    fill: TextEmphasisFill,
    position: TextEmphasisPosition,
) -> Option<ResolvedEmphasisMark> {
    let ch = mark.character(fill)?;
    Some(ResolvedEmphasisMark {
        character: ch,
        over: position.over,
    })
}

/// Returns the default emphasis mark shape for a writing mode.
///
/// Per CSS Text Decoration Level 3 §3.4:
/// - Horizontal text → `Dot` (filled dot '•')
/// - Vertical text → `Sesame` (filled sesame '﹅')
pub fn default_mark_for_writing_mode(writing_mode: WritingMode) -> TextEmphasisMark {
    if writing_mode.is_horizontal() {
        TextEmphasisMark::Dot
    } else {
        TextEmphasisMark::Sesame
    }
}

/// Returns the default emphasis position for a writing mode.
///
/// Per CSS Text Decoration Level 3 §3.5:
/// - Horizontal text → `over right`
/// - Vertical text → `over right` (right side in vertical-rl)
pub fn default_position_for_writing_mode(_writing_mode: WritingMode) -> TextEmphasisPosition {
    TextEmphasisPosition::INITIAL // over right for all modes
}

/// Checks whether emphasis marks should be applied to a character.
///
/// Per CSS Text Decoration Level 3 §3.4, emphasis marks are drawn for
/// every typographic character unit except:
/// - Whitespace (Unicode general category Zs, plus U+0009 tab)
/// - Control characters (Cc)
/// - Format characters (Cf), except for soft hyphen (U+00AD)
/// - Line and paragraph separators (Zl, Zp)
///
/// Blink: `ShouldDrawEmphasisMark()` in `text_decoration_info.cc`.
pub fn should_draw_emphasis_mark(ch: char) -> bool {
    if ch == '\u{00AD}' {
        // Soft hyphen: draw emphasis mark (Blink special-case).
        return true;
    }

    match unicode_general_category(ch) {
        // Skip whitespace separators (Zs), control (Cc), format (Cf),
        // line sep (Zl), paragraph sep (Zp).
        GeneralCategory::SpaceSeparator
        | GeneralCategory::Control
        | GeneralCategory::Format
        | GeneralCategory::LineSeparator
        | GeneralCategory::ParagraphSeparator => false,
        _ => true,
    }
}

/// Minimal Unicode General Category classification for emphasis mark filtering.
///
/// We only need to distinguish the categories relevant to `should_draw_emphasis_mark`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GeneralCategory {
    SpaceSeparator,
    LineSeparator,
    ParagraphSeparator,
    Control,
    Format,
    Other,
}

/// Classifies a character into the relevant Unicode General Category.
///
/// This is a simplified classifier covering the ranges needed for emphasis
/// mark filtering. Full Unicode data tables are not required here.
fn unicode_general_category(ch: char) -> GeneralCategory {
    let cp = ch as u32;

    // Cc: C0 controls (U+0000–U+001F) and C1 controls (U+007F–U+009F)
    if cp <= 0x001F || (0x007F..=0x009F).contains(&cp) {
        return GeneralCategory::Control;
    }

    // Zl: Line Separator (U+2028)
    if cp == 0x2028 {
        return GeneralCategory::LineSeparator;
    }

    // Zp: Paragraph Separator (U+2029)
    if cp == 0x2029 {
        return GeneralCategory::ParagraphSeparator;
    }

    // Zs: Space Separator
    // Covers ASCII space, no-break space, and Unicode space characters.
    if is_space_separator(cp) {
        return GeneralCategory::SpaceSeparator;
    }

    // Cf: Format characters
    if is_format_char(cp) {
        return GeneralCategory::Format;
    }

    GeneralCategory::Other
}

/// Returns `true` if the code point is in Unicode General Category Zs.
fn is_space_separator(cp: u32) -> bool {
    matches!(
        cp,
        0x0020  // SPACE
        | 0x00A0  // NO-BREAK SPACE
        | 0x1680  // OGHAM SPACE MARK
        | 0x2000..=0x200A  // EN QUAD through HAIR SPACE
        | 0x202F  // NARROW NO-BREAK SPACE
        | 0x205F  // MEDIUM MATHEMATICAL SPACE
        | 0x3000  // IDEOGRAPHIC SPACE
    )
}

/// Returns `true` if the code point is in Unicode General Category Cf.
fn is_format_char(cp: u32) -> bool {
    matches!(
        cp,
        0x00AD  // SOFT HYPHEN
        | 0x0600..=0x0605  // Arabic number sign, etc.
        | 0x061C  // ARABIC LETTER MARK
        | 0x06DD  // ARABIC END OF AYAH
        | 0x070F  // SYRIAC ABBREVIATION MARK
        | 0x0890..=0x0891  // Arabic pound/piastre marks
        | 0x08E2  // ARABIC DISPUTED END OF AYAH
        | 0x180E  // MONGOLIAN VOWEL SEPARATOR
        | 0x200B..=0x200F  // ZWSP, ZWNJ, ZWJ, LRM, RLM
        | 0x202A..=0x202E  // Bidi overrides
        | 0x2060..=0x2064  // Word joiner, invisible operators
        | 0x2066..=0x206F  // Bidi isolates + deprecated formatting
        | 0xFEFF  // BOM / ZWNBSP
        | 0xFFF9..=0xFFFB  // Interlinear annotation anchors
        | 0x110BD | 0x110CD  // Kaithi number signs
        | 0x13430..=0x1343F  // Egyptian hieroglyph formatting
        | 0x1BCA0..=0x1BCA3  // Shorthand format controls
        | 0x1D173..=0x1D17A  // Musical symbol formatting
        | 0xE0001  // LANGUAGE TAG
        | 0xE0020..=0xE007F  // TAG characters
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::{TextEmphasisFill, TextEmphasisMark, TextEmphasisPosition, WritingMode};

    // ── Character resolution tests ─────────────────────────────────

    #[test]
    fn filled_dot() {
        assert_eq!(TextEmphasisMark::Dot.character(TextEmphasisFill::Filled), Some('\u{2022}'));
    }

    #[test]
    fn open_dot() {
        assert_eq!(TextEmphasisMark::Dot.character(TextEmphasisFill::Open), Some('\u{25E6}'));
    }

    #[test]
    fn filled_circle() {
        assert_eq!(TextEmphasisMark::Circle.character(TextEmphasisFill::Filled), Some('\u{25CF}'));
    }

    #[test]
    fn open_circle() {
        assert_eq!(TextEmphasisMark::Circle.character(TextEmphasisFill::Open), Some('\u{25CB}'));
    }

    #[test]
    fn filled_double_circle() {
        assert_eq!(
            TextEmphasisMark::DoubleCircle.character(TextEmphasisFill::Filled),
            Some('\u{25C9}')
        );
    }

    #[test]
    fn open_double_circle() {
        assert_eq!(
            TextEmphasisMark::DoubleCircle.character(TextEmphasisFill::Open),
            Some('\u{25CE}')
        );
    }

    #[test]
    fn filled_triangle() {
        assert_eq!(
            TextEmphasisMark::Triangle.character(TextEmphasisFill::Filled),
            Some('\u{25B2}')
        );
    }

    #[test]
    fn open_triangle() {
        assert_eq!(
            TextEmphasisMark::Triangle.character(TextEmphasisFill::Open),
            Some('\u{25B3}')
        );
    }

    #[test]
    fn filled_sesame() {
        assert_eq!(
            TextEmphasisMark::Sesame.character(TextEmphasisFill::Filled),
            Some('\u{FE45}')
        );
    }

    #[test]
    fn open_sesame() {
        assert_eq!(
            TextEmphasisMark::Sesame.character(TextEmphasisFill::Open),
            Some('\u{FE46}')
        );
    }

    #[test]
    fn none_returns_none() {
        assert_eq!(TextEmphasisMark::None.character(TextEmphasisFill::Filled), None);
        assert_eq!(TextEmphasisMark::None.character(TextEmphasisFill::Open), None);
    }

    #[test]
    fn custom_char_ignores_fill() {
        let mark = TextEmphasisMark::Custom('★');
        assert_eq!(mark.character(TextEmphasisFill::Filled), Some('★'));
        assert_eq!(mark.character(TextEmphasisFill::Open), Some('★'));
    }

    #[test]
    fn custom_char_unicode() {
        let mark = TextEmphasisMark::Custom('♥');
        assert_eq!(mark.character(TextEmphasisFill::Filled), Some('♥'));
    }

    // ── Resolve emphasis mark tests ────────────────────────────────

    #[test]
    fn resolve_filled_dot_over() {
        let result = resolve_emphasis_mark(
            TextEmphasisMark::Dot,
            TextEmphasisFill::Filled,
            TextEmphasisPosition { over: true, right: true },
        );
        assert_eq!(result, Some(ResolvedEmphasisMark { character: '\u{2022}', over: true }));
    }

    #[test]
    fn resolve_open_circle_under() {
        let result = resolve_emphasis_mark(
            TextEmphasisMark::Circle,
            TextEmphasisFill::Open,
            TextEmphasisPosition { over: false, right: true },
        );
        assert_eq!(result, Some(ResolvedEmphasisMark { character: '\u{25CB}', over: false }));
    }

    #[test]
    fn resolve_none_returns_none() {
        let result = resolve_emphasis_mark(
            TextEmphasisMark::None,
            TextEmphasisFill::Filled,
            TextEmphasisPosition::INITIAL,
        );
        assert_eq!(result, None);
    }

    // ── Default mark for writing mode ──────────────────────────────

    #[test]
    fn default_mark_horizontal() {
        assert_eq!(
            default_mark_for_writing_mode(WritingMode::HorizontalTb),
            TextEmphasisMark::Dot
        );
    }

    #[test]
    fn default_mark_vertical_rl() {
        assert_eq!(
            default_mark_for_writing_mode(WritingMode::VerticalRl),
            TextEmphasisMark::Sesame
        );
    }

    #[test]
    fn default_mark_vertical_lr() {
        assert_eq!(
            default_mark_for_writing_mode(WritingMode::VerticalLr),
            TextEmphasisMark::Sesame
        );
    }

    // ── Default position ───────────────────────────────────────────

    #[test]
    fn default_position_is_over_right() {
        let pos = default_position_for_writing_mode(WritingMode::HorizontalTb);
        assert!(pos.over);
        assert!(pos.right);
    }

    // ── should_draw_emphasis_mark ──────────────────────────────────

    #[test]
    fn draw_on_letters() {
        assert!(should_draw_emphasis_mark('A'));
        assert!(should_draw_emphasis_mark('漢'));
        assert!(should_draw_emphasis_mark('α'));
        assert!(should_draw_emphasis_mark('Я'));
    }

    #[test]
    fn draw_on_digits() {
        assert!(should_draw_emphasis_mark('0'));
        assert!(should_draw_emphasis_mark('9'));
    }

    #[test]
    fn draw_on_punctuation() {
        assert!(should_draw_emphasis_mark('!'));
        assert!(should_draw_emphasis_mark('。'));
    }

    #[test]
    fn skip_ascii_space() {
        assert!(!should_draw_emphasis_mark(' '));
    }

    #[test]
    fn skip_tab() {
        assert!(!should_draw_emphasis_mark('\t'));
    }

    #[test]
    fn skip_newline() {
        assert!(!should_draw_emphasis_mark('\n'));
    }

    #[test]
    fn skip_no_break_space() {
        assert!(!should_draw_emphasis_mark('\u{00A0}'));
    }

    #[test]
    fn skip_ideographic_space() {
        assert!(!should_draw_emphasis_mark('\u{3000}'));
    }

    #[test]
    fn skip_zero_width_space() {
        assert!(!should_draw_emphasis_mark('\u{200B}'));
    }

    #[test]
    fn skip_zero_width_joiner() {
        assert!(!should_draw_emphasis_mark('\u{200D}'));
    }

    #[test]
    fn skip_bom() {
        assert!(!should_draw_emphasis_mark('\u{FEFF}'));
    }

    #[test]
    fn skip_line_separator() {
        assert!(!should_draw_emphasis_mark('\u{2028}'));
    }

    #[test]
    fn skip_paragraph_separator() {
        assert!(!should_draw_emphasis_mark('\u{2029}'));
    }

    #[test]
    fn soft_hyphen_draws() {
        // Special case: soft hyphen is Cf but should draw emphasis mark.
        assert!(should_draw_emphasis_mark('\u{00AD}'));
    }

    #[test]
    fn skip_left_to_right_mark() {
        assert!(!should_draw_emphasis_mark('\u{200E}'));
    }

    #[test]
    fn skip_right_to_left_mark() {
        assert!(!should_draw_emphasis_mark('\u{200F}'));
    }

    #[test]
    fn draw_on_emoji() {
        assert!(should_draw_emphasis_mark('😊'));
    }

    // ── Enum defaults ──────────────────────────────────────────────

    #[test]
    fn mark_default() {
        assert_eq!(TextEmphasisMark::default(), TextEmphasisMark::None);
    }

    #[test]
    fn fill_default() {
        assert_eq!(TextEmphasisFill::default(), TextEmphasisFill::Filled);
    }

    #[test]
    fn position_default() {
        let p = TextEmphasisPosition::default();
        assert!(p.over);
        assert!(p.right);
    }

    // ── All shapes × fills exhaustive ──────────────────────────────

    #[test]
    fn all_shapes_filled() {
        let fills = [
            (TextEmphasisMark::Dot, '\u{2022}'),
            (TextEmphasisMark::Circle, '\u{25CF}'),
            (TextEmphasisMark::DoubleCircle, '\u{25C9}'),
            (TextEmphasisMark::Triangle, '\u{25B2}'),
            (TextEmphasisMark::Sesame, '\u{FE45}'),
        ];
        for (mark, expected) in fills {
            assert_eq!(mark.character(TextEmphasisFill::Filled), Some(expected),
                "filled {:?} should be {:?}", mark, expected);
        }
    }

    #[test]
    fn all_shapes_open() {
        let opens = [
            (TextEmphasisMark::Dot, '\u{25E6}'),
            (TextEmphasisMark::Circle, '\u{25CB}'),
            (TextEmphasisMark::DoubleCircle, '\u{25CE}'),
            (TextEmphasisMark::Triangle, '\u{25B3}'),
            (TextEmphasisMark::Sesame, '\u{FE46}'),
        ];
        for (mark, expected) in opens {
            assert_eq!(mark.character(TextEmphasisFill::Open), Some(expected),
                "open {:?} should be {:?}", mark, expected);
        }
    }
}
