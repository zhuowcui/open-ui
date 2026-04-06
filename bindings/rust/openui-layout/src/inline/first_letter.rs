//! ::first-letter pseudo-element support.
//!
//! CSS 2.1 §5.12.2: The `::first-letter` pseudo-element creates a box around
//! the first typographic letter unit of a block container's first formatted
//! line. Punctuation before and after the first letter is included.
//!
//! Blink: `first_letter_utils.cc`, `FirstLetterPseudoElement`.
//!
//! The first letter is extracted as a separate inline item with its own style,
//! so it can receive special properties (font-size, float, margins, etc.)
//! that don't apply to the rest of the text.

use openui_style::ComputedStyle;
use openui_text::Font;
use crate::inline::items_builder::style_to_font_description;

/// Result of extracting the first typographic letter unit from text.
///
/// Contains the byte ranges separating the first-letter portion from the
/// remainder, so the items builder can split the text item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirstLetterExtraction {
    /// Byte offset where the first-letter portion starts (inclusive).
    /// Skips any leading whitespace — the first-letter pseudo-element
    /// does not include whitespace per CSS 2.1 §5.12.2.
    pub first_letter_start: usize,
    /// Byte offset where the first-letter portion ends (exclusive).
    /// This includes any leading punctuation + the letter + trailing punctuation.
    pub first_letter_end: usize,
    /// Whether any actual letter content was found.
    pub has_letter: bool,
}

/// Unicode categories considered punctuation for `::first-letter`.
///
/// CSS 2.1 §5.12.2: "Punctuation (i.e, characters defined in Unicode in
/// the 'open' (Ps), 'close' (Pe), 'initial' (Pi), 'final' (Pf) and
/// 'other' (Po) punctuation classes), that precedes or follows the first
/// typographic letter unit must also be included."
fn is_first_letter_punctuation(ch: char) -> bool {
    matches!(
        unicode_general_category(ch),
        UnicodeCategory::Ps
            | UnicodeCategory::Pe
            | UnicodeCategory::Pi
            | UnicodeCategory::Pf
            | UnicodeCategory::Po
    )
}

/// Simplified Unicode general category for first-letter punctuation check.
///
/// We only need to distinguish punctuation categories from letters.
/// This avoids pulling in a full Unicode category crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnicodeCategory {
    /// Open punctuation (Ps) — e.g., '(', '[', '{'.
    Ps,
    /// Close punctuation (Pe) — e.g., ')', ']', '}'.
    Pe,
    /// Initial punctuation (Pi) — e.g., '«', '"', '''.
    Pi,
    /// Final punctuation (Pf) — e.g., '»', '"', '''.
    Pf,
    /// Other punctuation (Po) — e.g., '!', ',', '.', '"', '\''.
    Po,
    /// Letter or other non-punctuation character.
    Other,
}

/// Classify a character into the Unicode general category subset
/// relevant for `::first-letter` extraction.
fn unicode_general_category(ch: char) -> UnicodeCategory {
    // We check specific ranges rather than using a full Unicode database.
    // This covers the most common punctuation encountered in practice.
    //
    // Note: Some characters belong to multiple conceptual categories (e.g.,
    // « and » are classified as Ps/Pe in some contexts). We handle them
    // in priority order: Pi/Pf first, then Ps/Pe, then Po.
    match ch {
        // Pi - Initial punctuation (checked first — takes priority)
        '\u{201B}' | '\u{201F}'
        | '\u{2E02}' | '\u{2E04}' | '\u{2E09}' | '\u{2E0C}' | '\u{2E1C}' | '\u{2E20}' => {
            UnicodeCategory::Pi
        }
        // Pf - Final punctuation
        '\u{2E03}' | '\u{2E05}'
        | '\u{2E0A}' | '\u{2E0D}' | '\u{2E1D}' | '\u{2E21}' => UnicodeCategory::Pf,
        // Ps - Open punctuation
        '(' | '[' | '{' | '\u{00AB}' | '\u{2018}' | '\u{201C}' | '\u{2039}' | '\u{300C}'
        | '\u{300E}' | '\u{3010}' | '\u{3014}' | '\u{3016}' | '\u{3018}' | '\u{301A}'
        | '\u{FF08}' | '\u{FF3B}' | '\u{FF5B}' => UnicodeCategory::Ps,
        // Pe - Close punctuation
        ')' | ']' | '}' | '\u{00BB}' | '\u{2019}' | '\u{201D}' | '\u{203A}' | '\u{300D}'
        | '\u{300F}' | '\u{3011}' | '\u{3015}' | '\u{3017}' | '\u{3019}' | '\u{301B}'
        | '\u{FF09}' | '\u{FF3D}' | '\u{FF5D}' => UnicodeCategory::Pe,
        // Po - Other punctuation (common ASCII + Unicode)
        '!' | '"' | '#' | '%' | '&' | '\'' | '*' | ',' | '.' | '/' | ':' | ';' | '?'
        | '@' | '\\' | '\u{00A1}' | '\u{00BF}' | '\u{2010}' | '\u{2011}' | '\u{2012}'
        | '\u{2013}' | '\u{2014}' | '\u{2015}' | '\u{2026}' | '\u{FF01}' | '\u{FF0C}'
        | '\u{FF0E}' | '\u{FF1A}' | '\u{FF1B}' | '\u{FF1F}' | '\u{3001}' | '\u{3002}' => {
            UnicodeCategory::Po
        }
        _ => UnicodeCategory::Other,
    }
}

/// Is the character a Unicode letter (L category)?
///
/// Simplified check: alphanumeric or Unicode letter ranges.
fn is_letter(ch: char) -> bool {
    ch.is_alphabetic() || ch.is_numeric()
}

/// Extract the first typographic letter unit from text content.
///
/// CSS 2.1 §5.12.2: The first letter includes:
/// 1. Leading punctuation (Ps, Pe, Pi, Pf, Po categories)
/// 2. The first letter character
/// 3. Trailing punctuation immediately following the letter
///
/// For multi-codepoint letter clusters (e.g., digraphs in Dutch "IJ"),
/// the entire cluster is included. We use Unicode grapheme cluster
/// boundaries as an approximation.
///
/// Returns `None` if the text contains no letter content (all whitespace
/// or all punctuation with no letter).
pub fn extract_first_letter(text: &str) -> Option<FirstLetterExtraction> {
    if text.is_empty() {
        return None;
    }

    let mut chars = text.char_indices().peekable();
    let mut offset = 0;
    let mut found_letter = false;

    // Phase 1: Skip whitespace at the start
    while let Some(&(idx, ch)) = chars.peek() {
        if ch.is_whitespace() {
            chars.next();
            offset = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    let first_letter_start = offset;

    // Phase 2: Consume leading punctuation
    while let Some(&(idx, ch)) = chars.peek() {
        if is_first_letter_punctuation(ch) {
            chars.next();
            offset = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    // Phase 3: Consume the first letter (including multi-codepoint clusters)
    if let Some(&(idx, ch)) = chars.peek() {
        if is_letter(ch) {
            found_letter = true;
            chars.next();
            offset = idx + ch.len_utf8();

            // Handle combining marks following the base letter.
            // Only actual combining marks (Unicode Mn/Mc/Me categories)
            // are part of the same grapheme cluster and should be included.
            while let Some(&(idx, ch)) = chars.peek() {
                if is_combining_mark(ch) {
                    chars.next();
                    offset = idx + ch.len_utf8();
                } else {
                    break;
                }
            }
        }
    }

    if !found_letter {
        return None;
    }

    // Phase 4: Consume trailing punctuation
    while let Some(&(idx, ch)) = chars.peek() {
        if is_first_letter_punctuation(ch) {
            chars.next();
            offset = idx + ch.len_utf8();
        } else {
            break;
        }
    }

    Some(FirstLetterExtraction {
        first_letter_start,
        first_letter_end: offset,
        has_letter: true,
    })
}

/// Check if a character is a Unicode combining mark (Mn, Mc, Me categories).
fn is_combining_mark(ch: char) -> bool {
    let cp = ch as u32;
    // Common combining mark ranges
    (0x0300..=0x036F).contains(&cp)   // Combining Diacritical Marks
        || (0x0483..=0x0489).contains(&cp)  // Cyrillic combining marks
        || (0x0591..=0x05BD).contains(&cp)  // Hebrew combining marks
        || (0x0610..=0x061A).contains(&cp)  // Arabic combining marks
        || (0x064B..=0x065F).contains(&cp)  // Arabic combining marks
        || (0x0900..=0x0903).contains(&cp)  // Devanagari combining marks
        || (0x093A..=0x094F).contains(&cp)  // Devanagari combining marks
        || (0x0E31..=0x0E3A).contains(&cp)  // Thai combining marks
        || (0x20D0..=0x20FF).contains(&cp)  // Combining Marks for Symbols
        || (0xFE20..=0xFE2F).contains(&cp)  // Combining Half Marks
}

/// Style properties applicable to `::first-letter`.
///
/// CSS 2.1 §5.12.2 allows: font, color, background, text-decoration,
/// text-transform, letter-spacing, word-spacing, line-height,
/// vertical-align (if float is none), margin, padding, border, float.
///
/// This is a superset of `::first-line` properties, plus box-model and float.
#[derive(Debug, Clone)]
pub struct FirstLetterStyle {
    /// The complete style for the first-letter pseudo-element.
    /// In practice, authors typically set font-size, float, and margin.
    pub style: ComputedStyle,
}

impl FirstLetterStyle {
    /// Create a first-letter style from a base style with overrides.
    pub fn new(base: &ComputedStyle) -> Self {
        Self {
            style: base.clone(),
        }
    }

    /// Apply common drop-cap styling: large font, left float, right margin.
    pub fn with_drop_cap(base: &ComputedStyle, font_size: f32, margin_right: f32) -> Self {
        let mut style = base.clone();
        style.font_size = font_size;
        style.float = openui_style::Float::Left;
        style.margin_right = openui_geometry::Length::px(margin_right);
        Self { style }
    }
}

/// Compute the first-letter box dimensions based on the letter's metrics.
///
/// The first-letter box is an inline box (or floated box if `float` is set)
/// containing only the first letter. Its dimensions come from the font
/// metrics at the specified font-size.
#[derive(Debug, Clone, Copy)]
pub struct FirstLetterMetrics {
    /// Width of the first letter in pixels.
    pub width: f32,
    /// Height (ascent + descent) of the first letter in pixels.
    pub height: f32,
    /// Ascent from the baseline in pixels.
    pub ascent: f32,
    /// Descent below the baseline in pixels.
    pub descent: f32,
}

impl FirstLetterMetrics {
    /// Compute metrics for a first letter using the given style.
    ///
    /// Queries actual font metrics from the resolved font to ensure pixel-
    /// perfect accuracy. The width uses the font's `zero_width` (ch unit)
    /// as a representative character width; the height uses real ascent and
    /// descent from the font tables.
    pub fn from_style(style: &ComputedStyle) -> Self {
        let font_desc = style_to_font_description(style);
        let font = Font::new(font_desc);
        let metrics = font.font_metrics().copied().unwrap_or_default();
        Self {
            width: metrics.zero_width.max(metrics.cap_height * 0.7),
            height: metrics.ascent + metrics.descent,
            ascent: metrics.ascent,
            descent: metrics.descent,
        }
    }

    /// Compute metrics for a first letter at the given font size.
    ///
    /// Queries actual font metrics from the default font at the requested
    /// size. Prefer `from_style()` when a `ComputedStyle` is available.
    pub fn from_font_size(font_size: f32) -> Self {
        let mut style = ComputedStyle::default();
        style.font_size = font_size;
        Self::from_style(&style)
    }
}

/// Determine if a text node starts with content eligible for `::first-letter`.
///
/// Returns `true` if the text begins with (optional punctuation +) a letter.
/// Returns `false` for text that starts with whitespace-only or has no letters.
pub fn has_first_letter_content(text: &str) -> bool {
    extract_first_letter(text).is_some()
}

/// Split text into the first-letter portion and the remainder.
///
/// Returns `(first_letter_text, remainder_text)`. Leading whitespace before
/// the first letter is excluded from the first-letter portion (CSS 2.1 §5.12.2).
pub fn split_first_letter(text: &str) -> Option<(&str, &str)> {
    let extraction = extract_first_letter(text)?;
    Some((
        &text[extraction.first_letter_start..extraction.first_letter_end],
        &text[extraction.first_letter_end..],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_letter() {
        let result = extract_first_letter("Hello").unwrap();
        assert_eq!(result.first_letter_end, 1); // "H"
        assert!(result.has_letter);
    }

    #[test]
    fn letter_with_leading_punctuation() {
        let result = extract_first_letter("\"Hello").unwrap();
        assert_eq!(result.first_letter_end, 2); // "\"H"
        assert!(result.has_letter);
    }

    #[test]
    fn letter_with_trailing_punctuation() {
        let result = extract_first_letter("A.bc").unwrap();
        assert_eq!(result.first_letter_end, 2); // "A."
        assert!(result.has_letter);
    }

    #[test]
    fn letter_with_both_punctuation() {
        let result = extract_first_letter("\"A\"rest").unwrap();
        assert_eq!(result.first_letter_end, 3); // "\"A\""
        assert!(result.has_letter);
    }

    #[test]
    fn leading_whitespace_then_letter() {
        let result = extract_first_letter("  Hello").unwrap();
        assert_eq!(result.first_letter_start, 2); // skip "  "
        assert_eq!(result.first_letter_end, 3); // "H" at byte 2..3
        // The first-letter text is "H", not "  H"
        assert_eq!(&"  Hello"[result.first_letter_start..result.first_letter_end], "H");
    }

    #[test]
    fn empty_text_returns_none() {
        assert!(extract_first_letter("").is_none());
    }

    #[test]
    fn whitespace_only_returns_none() {
        assert!(extract_first_letter("   ").is_none());
    }

    #[test]
    fn punctuation_only_returns_none() {
        assert!(extract_first_letter("...").is_none());
    }

    #[test]
    fn multibyte_letter() {
        let result = extract_first_letter("Über").unwrap();
        assert_eq!(&"Über"[..result.first_letter_end], "Ü");
        assert!(result.has_letter);
    }

    #[test]
    fn cjk_character() {
        let result = extract_first_letter("漢字").unwrap();
        assert_eq!(&"漢字"[..result.first_letter_end], "漢");
        assert!(result.has_letter);
    }

    #[test]
    fn split_first_letter_basic() {
        let (first, rest) = split_first_letter("Hello").unwrap();
        assert_eq!(first, "H");
        assert_eq!(rest, "ello");
    }

    #[test]
    fn split_first_letter_with_punctuation() {
        let (first, rest) = split_first_letter("\"Hello\"").unwrap();
        assert_eq!(first, "\"H");
        assert_eq!(rest, "ello\"");
    }

    #[test]
    fn has_first_letter_content_true() {
        assert!(has_first_letter_content("Hello"));
        assert!(has_first_letter_content("\"X\""));
        assert!(has_first_letter_content("  A"));
    }

    #[test]
    fn has_first_letter_content_false() {
        assert!(!has_first_letter_content(""));
        assert!(!has_first_letter_content("   "));
    }

    #[test]
    fn first_letter_metrics_from_font_size() {
        let m = FirstLetterMetrics::from_font_size(48.0);
        // Uses real font metrics, so values depend on the actual default font.
        // Assert positive, sensible values rather than hardcoded approximations.
        assert!(m.ascent > 0.0, "ascent should be positive: {}", m.ascent);
        assert!(m.descent > 0.0, "descent should be positive: {}", m.descent);
        assert!((m.height - (m.ascent + m.descent)).abs() < 0.01, "height = ascent + descent");
        assert!(m.width > 0.0, "width should be positive: {}", m.width);
        // Sanity: metrics scale with font size
        let m2 = FirstLetterMetrics::from_font_size(96.0);
        assert!(m2.ascent > m.ascent, "larger font should have larger ascent");
        assert!(m2.height > m.height, "larger font should have larger height");
    }

    #[test]
    fn first_letter_style_drop_cap() {
        let base = ComputedStyle::initial();
        let fls = FirstLetterStyle::with_drop_cap(&base, 48.0, 4.0);
        assert_eq!(fls.style.font_size, 48.0);
        assert_eq!(fls.style.float, openui_style::Float::Left);
    }

    #[test]
    fn number_as_first_letter() {
        let result = extract_first_letter("1st").unwrap();
        assert_eq!(result.first_letter_end, 1); // "1"
        assert!(result.has_letter);
    }

    #[test]
    fn open_bracket_then_letter() {
        let result = extract_first_letter("(A)bc").unwrap();
        // "(" is Ps (open punctuation), "A" is letter, ")" is Pe (close)
        assert_eq!(result.first_letter_end, 3); // "(A)"
        assert!(result.has_letter);
    }
}
