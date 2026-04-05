//! Text transform — CSS `text-transform` property implementation.
//!
//! Mirrors Blink's locale-aware text transform logic from
//! `third_party/blink/renderer/platform/text/text_transform.h` and
//! `third_party/blink/renderer/platform/text/case_map.cc`.
//!
//! Blink delegates to ICU's `icu::CaseMap::toLower/toUpper/toTitle` with a
//! locale parameter derived from the `lang` HTML attribute. This module
//! implements the same locale-specific rules directly:
//!
//! | Locale      | Lowercase                    | Uppercase                  | Capitalize            |
//! |-------------|------------------------------|----------------------------|-----------------------|
//! | `tr` / `az` | I→ı, İ→i, I+̇→i             | i→İ, ı→I                  | i→İ at word start     |
//! | `el`        | (default)                    | strip tonos from vowels    | (default)             |
//! | `lt`        | I/J/Į + accent → add dot     | (default)                  | (default)             |
//! | `nl`        | (default)                    | (default)                  | ij→IJ at word start   |
//! | (default)   | Unicode Default (σ→ς final)  | Unicode Default            | Unicode Titlecase     |
//!
//! References:
//! - Unicode SpecialCasing.txt <https://www.unicode.org/Public/UCD/latest/ucd/SpecialCasing.txt>
//! - CSS Text Level 3 §2 <https://drafts.csswg.org/css-text-3/#text-transform>
//! - Blink CaseMap <https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/platform/text/case_map.cc>
//!
//! Supports: none, uppercase, lowercase, capitalize, full-width, full-size-kana.

use openui_style::TextTransform;

// ═══════════════════════════════════════════════════════════════════════════
// Locale classification
// ═══════════════════════════════════════════════════════════════════════════

/// Coarse locale categories that affect case mapping.
///
/// Blink's `CaseMap` normalises the full BCP 47 tag down to the primary
/// language subtag and checks against this same set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocaleCategory {
    /// Turkish / Azerbaijani — dotted vs dotless I.
    Turkish,
    /// Greek — strip tonos on uppercase.
    Greek,
    /// Lithuanian — preserve dot-above before accents in lowercase.
    Lithuanian,
    /// Dutch — IJ digraph titlecasing.
    Dutch,
    /// All other locales — Unicode Default Case Conversion.
    Default,
}

/// Map an optional BCP 47 locale tag to a [`LocaleCategory`].
///
/// Only the primary language subtag matters; region/script are ignored.
/// Matching is case-insensitive per BCP 47 §2.1.1.
fn classify_locale(locale: Option<&str>) -> LocaleCategory {
    let tag = match locale {
        Some(s) if !s.is_empty() => s,
        _ => return LocaleCategory::Default,
    };
    // Extract the primary language subtag (everything before the first '-').
    let primary = tag.split('-').next().unwrap_or(tag);
    // Case-insensitive two-letter comparison.
    let mut buf = [0u8; 3];
    let len = primary.len().min(3);
    for (i, &b) in primary.as_bytes().iter().take(len).enumerate() {
        buf[i] = b.to_ascii_lowercase();
    }
    match (len, &buf[..len]) {
        (2, b"tr") | (2, b"az") => LocaleCategory::Turkish,
        (2, b"el") => LocaleCategory::Greek,
        (2, b"lt") => LocaleCategory::Lithuanian,
        (2, b"nl") => LocaleCategory::Dutch,
        _ => LocaleCategory::Default,
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Public entry point
// ═══════════════════════════════════════════════════════════════════════════

/// Apply the CSS `text-transform` property to text with locale awareness.
///
/// Blink: `ComputedStyle::ApplyTextTransform` → `CaseMap::toUpper/toLower/toTitle`
/// in `case_map.cc`. The locale parameter originates from the `lang` HTML
/// attribute (mapped to a BCP 47 tag).
///
/// Full-width and full-size-kana transforms are locale-independent.
///
/// # Arguments
/// * `text`      — input string
/// * `transform` — CSS `text-transform` value
/// * `locale`    — optional BCP 47 locale tag (e.g. `"tr"`, `"el"`, `"nl-NL"`)
pub fn apply_text_transform(
    text: &str,
    transform: TextTransform,
    locale: Option<&str>,
) -> String {
    match transform {
        TextTransform::None => text.to_string(),
        TextTransform::Uppercase => locale_uppercase(text, locale),
        TextTransform::Lowercase => locale_lowercase(text, locale),
        TextTransform::Capitalize => locale_capitalize(text, locale),
        TextTransform::FullWidth => to_full_width(text),
        TextTransform::FullSizeKana => to_full_size_kana(text),
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Uppercase
// ═══════════════════════════════════════════════════════════════════════════

fn locale_uppercase(text: &str, locale: Option<&str>) -> String {
    match classify_locale(locale) {
        LocaleCategory::Turkish => turkish_to_upper(text),
        LocaleCategory::Greek => greek_to_upper(text),
        _ => text.to_uppercase(),
    }
}

/// Turkish/Azerbaijani uppercase.
///
/// Unicode SpecialCasing.txt:
///   0069 → 0130  ; `i` → `İ` (LATIN SMALL LETTER I → LATIN CAPITAL LETTER I WITH DOT ABOVE)
///   0131 → 0049  ; `ı` → `I` (LATIN SMALL LETTER DOTLESS I → LATIN CAPITAL LETTER I)
fn turkish_to_upper(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            'i' => result.push('\u{0130}'),  // i → İ
            '\u{0131}' => result.push('I'),  // ı → I
            _ => {
                for c in ch.to_uppercase() {
                    result.push(c);
                }
            }
        }
    }
    result
}

/// Greek uppercase — strip tonos (accent) from vowels.
///
/// CLDR Greek (el) uppercasing rule: accented vowels lose their tonos.
/// Precomposed characters are mapped directly; combining acute (U+0301),
/// grave (U+0300), and tilde (U+0303) after a Greek base letter are removed.
///
/// References:
/// - CLDR transform el-Upper <https://unicode.org/cldr/charts/latest/transforms/el-el_FONIPA.html>
/// - ICU GreekUpper <https://github.com/nicolo-ribaudo/tc39-proposal-regex-escapes>
fn greek_to_upper(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut prev_base_greek = false;

    for ch in text.chars() {
        match ch {
            // Precomposed vowels with tonos → uppercase without accent.
            '\u{03AC}' => { result.push('\u{0391}'); prev_base_greek = true; } // ά → Α
            '\u{03AD}' => { result.push('\u{0395}'); prev_base_greek = true; } // έ → Ε
            '\u{03AE}' => { result.push('\u{0397}'); prev_base_greek = true; } // ή → Η
            '\u{03AF}' => { result.push('\u{0399}'); prev_base_greek = true; } // ί → Ι
            '\u{03CC}' => { result.push('\u{039F}'); prev_base_greek = true; } // ό → Ο
            '\u{03CD}' => { result.push('\u{03A5}'); prev_base_greek = true; } // ύ → Υ
            '\u{03CE}' => { result.push('\u{03A9}'); prev_base_greek = true; } // ώ → Ω
            // Vowels with dialytika + tonos → keep dialytika, drop tonos.
            '\u{0390}' => { result.push('\u{03AA}'); prev_base_greek = true; } // ΐ → Ϊ
            '\u{03B0}' => { result.push('\u{03AB}'); prev_base_greek = true; } // ΰ → Ϋ
            // Combining accents: strip after Greek base letter.
            '\u{0301}' | '\u{0300}' | '\u{0303}' | '\u{0344}' if prev_base_greek => {
                // Drop combining tonos / grave / tilde / dialytika-tonos.
            }
            // Iota subscript (ypogegrammeni): drop in Greek uppercase context.
            '\u{0345}' if prev_base_greek => {}
            _ => {
                prev_base_greek = is_greek_base(ch);
                for c in ch.to_uppercase() {
                    result.push(c);
                }
            }
        }
    }
    result
}

/// Returns `true` if `ch` is a Greek letter (base character).
fn is_greek_base(ch: char) -> bool {
    matches!(ch as u32,
        // Greek and Coptic block: Α-ω plus archaic letters
        0x0370..=0x03FF |
        // Greek Extended block: ἀ-ῼ
        0x1F00..=0x1FFF
    )
}

// ═══════════════════════════════════════════════════════════════════════════
// Lowercase
// ═══════════════════════════════════════════════════════════════════════════

fn locale_lowercase(text: &str, locale: Option<&str>) -> String {
    match classify_locale(locale) {
        LocaleCategory::Turkish => turkish_to_lower(text),
        LocaleCategory::Lithuanian => lithuanian_to_lower(text),
        // Rust's str::to_lowercase() implements Unicode Default Case Conversion
        // including context-dependent Greek sigma: Σ→ς at word end, Σ→σ elsewhere.
        _ => text.to_lowercase(),
    }
}

/// Turkish/Azerbaijani lowercase.
///
/// Unicode SpecialCasing.txt:
///   0049 → 0131           ; `I` → `ı` (LATIN CAPITAL LETTER I → LATIN SMALL LETTER DOTLESS I)
///   0049 0307 → 0069      ; `I` + combining dot above → `i` (dot absorbed)
///   0130 → 0069           ; `İ` → `i` (LATIN CAPITAL LETTER I WITH DOT ABOVE → LATIN SMALL LETTER I)
fn turkish_to_lower(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            'I' => {
                // I + combining dot above → i (the dot signals a dotted I).
                if i + 1 < chars.len() && chars[i + 1] == '\u{0307}' {
                    result.push('i');
                    i += 2; // consume the combining dot above
                } else {
                    result.push('\u{0131}'); // I → ı (dotless i)
                    i += 1;
                }
            }
            '\u{0130}' => {
                result.push('i'); // İ → i
                i += 1;
            }
            _ => {
                for c in chars[i].to_lowercase() {
                    result.push(c);
                }
                i += 1;
            }
        }
    }
    result
}

/// Lithuanian lowercase — preserve dot-above before combining accents.
///
/// Unicode SpecialCasing.txt (condition: lt):
///   0049 ; 0069 0307 ; … ; lt More_Above  — I → i + combining dot above
///   004A ; 006A 0307 ; … ; lt More_Above  — J → j + combining dot above
///   012E ; 012F 0307 ; … ; lt More_Above  — Į → į + combining dot above
///   00CC ; 0069 0307 0300 ; … ; lt         — Ì → i + dot above + grave
///   00CD ; 0069 0307 0301 ; … ; lt         — Í → i + dot above + acute
///   0128 ; 0069 0307 0303 ; … ; lt         — Ĩ → i + dot above + tilde
///
/// "More_Above" means the next combining character has Canonical_Combining_Class = 230 (Above).
fn lithuanian_to_lower(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 8);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        match chars[i] {
            // Precomposed: Ì → i + dot above + grave
            '\u{00CC}' => {
                result.push('i');
                result.push('\u{0307}');
                result.push('\u{0300}');
                i += 1;
            }
            // Precomposed: Í → i + dot above + acute
            '\u{00CD}' => {
                result.push('i');
                result.push('\u{0307}');
                result.push('\u{0301}');
                i += 1;
            }
            // Precomposed: Ĩ → i + dot above + tilde
            '\u{0128}' => {
                result.push('i');
                result.push('\u{0307}');
                result.push('\u{0303}');
                i += 1;
            }
            // I / J / Į followed by a combining mark above → add dot-above
            'I' | 'J' | '\u{012E}' if has_combining_above_after(&chars, i) => {
                for c in chars[i].to_lowercase() {
                    result.push(c);
                }
                result.push('\u{0307}'); // insert combining dot above
                i += 1;
            }
            _ => {
                for c in chars[i].to_lowercase() {
                    result.push(c);
                }
                i += 1;
            }
        }
    }
    result
}

/// Check if position `pos + 1` holds a combining mark with CCC = 230 (Above).
///
/// We check the most common combining marks used in Lithuanian text:
/// U+0300 grave, U+0301 acute, U+0302 circumflex, U+0303 tilde,
/// U+0304 macron, U+0306 breve, U+0307 dot above, U+0308 diaeresis,
/// U+030A ring above, U+030B double acute, U+030C caron.
fn has_combining_above_after(chars: &[char], pos: usize) -> bool {
    if let Some(&next) = chars.get(pos + 1) {
        matches!(next as u32, 0x0300..=0x0314)
    } else {
        false
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// Capitalize (titlecase first letter of each word)
// ═══════════════════════════════════════════════════════════════════════════

fn locale_capitalize(text: &str, locale: Option<&str>) -> String {
    match classify_locale(locale) {
        LocaleCategory::Turkish => turkish_capitalize(text),
        LocaleCategory::Dutch => dutch_capitalize(text),
        _ => capitalize(text),
    }
}

/// Capitalize the first letter of each word (default/root locale).
///
/// CSS Text §2.1 "first typographic letter unit of each word": word
/// boundaries include spaces, hyphens, and other punctuation (but NOT
/// apostrophes within words). Digits are treated as word-internal —
/// "1st" is one word, so the 's' is NOT capitalised.
fn capitalize(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            for tc in to_titlecase(ch) {
                result.push(tc);
            }
            capitalize_next = false;
        } else {
            result.push(ch);
            if ch.is_alphanumeric() {
                capitalize_next = false;
            } else if ch != '\'' && ch != '\u{2019}' {
                capitalize_next = true;
            }
        }
    }
    result
}

/// Turkish/Azerbaijani capitalize — `i` at word start → `İ`.
///
/// All other titlecasing uses the default `to_titlecase` mapping,
/// except that `i` (U+0069) titlecases to `İ` (U+0130) and
/// `ı` (U+0131) titlecases to `I` (U+0049).
fn turkish_capitalize(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            match ch {
                'i' => result.push('\u{0130}'), // i → İ
                '\u{0131}' => result.push('I'), // ı → I
                _ => {
                    for tc in to_titlecase(ch) {
                        result.push(tc);
                    }
                }
            }
            capitalize_next = false;
        } else {
            result.push(ch);
            if ch.is_alphanumeric() {
                capitalize_next = false;
            } else if ch != '\'' && ch != '\u{2019}' {
                capitalize_next = true;
            }
        }
    }
    result
}

/// Dutch capitalize — `ij` digraph at word start → `IJ`.
///
/// In Dutch, the digraph "ij" is treated as a single letter. When it appears
/// at the start of a word, both characters are uppercased: "ijsselmeer" → "IJsselmeer".
/// The Unicode IJ digraph (U+0133 ĳ / U+0132 Ĳ) is handled by `to_titlecase`.
fn dutch_capitalize(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let chars: Vec<char> = text.chars().collect();
    let mut capitalize_next = true;
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];
        if capitalize_next && ch.is_alphabetic() {
            // Check for ij digraph (two separate ASCII characters).
            if (ch == 'i' || ch == 'I')
                && i + 1 < chars.len()
                && (chars[i + 1] == 'j' || chars[i + 1] == 'J')
            {
                result.push('I');
                result.push('J');
                i += 2;
            } else {
                for tc in to_titlecase(ch) {
                    result.push(tc);
                }
                i += 1;
            }
            capitalize_next = false;
        } else {
            result.push(ch);
            if ch.is_alphanumeric() {
                capitalize_next = false;
            } else if ch != '\'' && ch != '\u{2019}' {
                capitalize_next = true;
            }
            i += 1;
        }
    }
    result
}

/// Map a character to its Unicode titlecase form.
///
/// Handles the characters where titlecase differs from uppercase:
/// digraph ligatures, ß→Ss, Armenian ligature, and titlecase-form identity.
/// For everything else, falls back to `char::to_uppercase()`.
fn to_titlecase(ch: char) -> Vec<char> {
    match ch {
        // Latin small letter dz digraph variants
        '\u{01F3}' => vec!['\u{01F2}'], // ǳ → ǲ
        '\u{01F1}' => vec!['\u{01F2}'], // Ǳ → ǲ
        // Latin small letter lj digraph variants
        '\u{01C6}' => vec!['\u{01C5}'], // ǆ → ǅ
        '\u{01C4}' => vec!['\u{01C5}'], // Ǆ → ǅ
        // Latin small letter nj digraph variants
        '\u{01C9}' => vec!['\u{01C8}'], // ǉ → ǈ
        '\u{01C7}' => vec!['\u{01C8}'], // Ǉ → ǈ
        // Latin small letter dz variants
        '\u{01CC}' => vec!['\u{01CB}'], // ǌ → ǋ
        '\u{01CA}' => vec!['\u{01CB}'], // Ǌ → ǋ
        // Already-titlecase forms map to themselves
        '\u{01F2}' => vec!['\u{01F2}'], // ǲ → ǲ
        '\u{01C5}' => vec!['\u{01C5}'], // ǅ → ǅ
        '\u{01C8}' => vec!['\u{01C8}'], // ǈ → ǈ
        '\u{01CB}' => vec!['\u{01CB}'], // ǋ → ǋ
        // German sharp s: titlecase is Ss (not SS)
        '\u{00DF}' => vec!['S', 's'],   // ß → Ss
        // Armenian ligature ech-yiwn
        '\u{0587}' => vec!['\u{0535}', '\u{0582}'], // և → Եւ
        // Unicode Latin ligatures — titlecase differs from uppercase
        '\u{FB00}' => vec!['F', 'f'],       // ﬀ → Ff
        '\u{FB01}' => vec!['F', 'i'],       // ﬁ → Fi
        '\u{FB02}' => vec!['F', 'l'],       // ﬂ → Fl
        '\u{FB03}' => vec!['F', 'f', 'i'],  // ﬃ → Ffi
        '\u{FB04}' => vec!['F', 'f', 'l'],  // ﬄ → Ffl
        '\u{FB05}' => vec!['S', 't'],       // ﬅ → St
        '\u{FB06}' => vec!['S', 't'],       // ﬆ → St
        // Dutch IJ digraph
        '\u{0133}' => vec!['\u{0132}'],     // ĳ → Ĳ
        '\u{0132}' => vec!['\u{0132}'],     // Ĳ → Ĳ (already uppercase)
        _ => ch.to_uppercase().collect(),
    }
}

/// Convert ASCII characters to their fullwidth equivalents.
///
/// ASCII printable range U+0021..=U+007E maps to fullwidth U+FF01..=U+FF5E.
/// Space (U+0020) maps to ideographic space (U+3000).
fn to_full_width(text: &str) -> String {
    text.chars()
        .map(|ch| {
            let code = ch as u32;
            if (0x21..=0x7E).contains(&code) {
                // Map ASCII printable to fullwidth
                char::from_u32(code + 0xFF01 - 0x21).unwrap_or(ch)
            } else if ch == ' ' {
                '\u{3000}' // Ideographic space
            } else {
                ch
            }
        })
        .collect()
}

/// Convert small kana to their full-size equivalents.
///
/// CSS Text Level 4 §2: `text-transform: full-size-kana` converts small
/// Hiragana and Katakana to their full-size forms. This matches Blink's
/// `ApplyFullSizeKanaTransform` in `text_transform.cc`.
fn to_full_size_kana(text: &str) -> String {
    text.chars()
        .map(|ch| match ch {
            // Small Hiragana → Full-size Hiragana
            'ぁ' => 'あ', 'ぃ' => 'い', 'ぅ' => 'う', 'ぇ' => 'え', 'ぉ' => 'お',
            'っ' => 'つ', 'ゃ' => 'や', 'ゅ' => 'ゆ', 'ょ' => 'よ', 'ゎ' => 'わ',
            'ゕ' => 'か', 'ゖ' => 'け', // U+3095 → U+304B, U+3096 → U+3051
            // Small Katakana → Full-size Katakana
            'ァ' => 'ア', 'ィ' => 'イ', 'ゥ' => 'ウ', 'ェ' => 'エ', 'ォ' => 'オ',
            'ッ' => 'ツ', 'ャ' => 'ヤ', 'ュ' => 'ユ', 'ョ' => 'ヨ', 'ヮ' => 'ワ',
            'ヵ' => 'カ', 'ヶ' => 'ケ',
            // Half-width small Katakana → Full-size Katakana
            'ｧ' => 'ア', 'ｨ' => 'イ', 'ｩ' => 'ウ', 'ｪ' => 'エ', 'ｫ' => 'オ',
            'ｯ' => 'ツ', 'ｬ' => 'ヤ', 'ｭ' => 'ユ', 'ｮ' => 'ヨ',
            _ => ch,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transform_none() {
        assert_eq!(apply_text_transform("Hello World", TextTransform::None, None), "Hello World");
    }

    #[test]
    fn transform_uppercase() {
        assert_eq!(apply_text_transform("hello", TextTransform::Uppercase, None), "HELLO");
    }

    #[test]
    fn transform_lowercase() {
        assert_eq!(apply_text_transform("HELLO", TextTransform::Lowercase, None), "hello");
    }

    #[test]
    fn transform_capitalize() {
        assert_eq!(apply_text_transform("hello world", TextTransform::Capitalize, None), "Hello World");
    }

    #[test]
    fn capitalize_after_hyphen() {
        assert_eq!(apply_text_transform("well-known", TextTransform::Capitalize, None), "Well-Known");
    }

    #[test]
    fn unicode_uppercase() {
        assert_eq!(apply_text_transform("café", TextTransform::Uppercase, None), "CAFÉ");
    }

    #[test]
    fn full_width_ascii() {
        assert_eq!(apply_text_transform("ABC", TextTransform::FullWidth, None), "ＡＢＣ");
    }

    #[test]
    fn full_width_space() {
        assert_eq!(apply_text_transform("A B", TextTransform::FullWidth, None), "Ａ\u{3000}Ｂ");
    }

    #[test]
    fn full_size_kana_converts_small_kana() {
        assert_eq!(
            apply_text_transform("ぁぃぅぇぉ", TextTransform::FullSizeKana, None),
            "あいうえお"
        );
    }

    #[test]
    fn full_size_kana_converts_small_katakana() {
        assert_eq!(
            apply_text_transform("ァィゥェォ", TextTransform::FullSizeKana, None),
            "アイウエオ"
        );
    }

    #[test]
    fn full_size_kana_preserves_normal_text() {
        assert_eq!(
            apply_text_transform("こんにちは", TextTransform::FullSizeKana, None),
            "こんにちは"
        );
    }

    // ── SP11 Round 17 Issue 2: ゕ (U+3095) and ゖ (U+3096) mappings ──

    #[test]
    fn full_size_kana_converts_small_hiragana_ka_ke() {
        assert_eq!(
            apply_text_transform("ゕゖ", TextTransform::FullSizeKana, None),
            "かけ"
        );
    }

    #[test]
    fn capitalize_apostrophe_not_word_boundary() {
        // CSS Text §2.1: apostrophe within a word does NOT start a new word.
        assert_eq!(
            apply_text_transform("it's a test", TextTransform::Capitalize, None),
            "It's A Test"
        );
    }

    // ── SP11 Round 11 Issue 5: capitalize after punctuation ──

    #[test]
    fn capitalize_after_open_paren() {
        assert_eq!(
            apply_text_transform("(hello) world", TextTransform::Capitalize, None),
            "(Hello) World"
        );
    }

    #[test]
    fn capitalize_after_slash() {
        assert_eq!(
            apply_text_transform("foo/bar", TextTransform::Capitalize, None),
            "Foo/Bar"
        );
    }

    #[test]
    fn capitalize_after_dot() {
        assert_eq!(
            apply_text_transform("first.second", TextTransform::Capitalize, None),
            "First.Second"
        );
    }

    #[test]
    fn capitalize_after_colon() {
        assert_eq!(
            apply_text_transform("key:value", TextTransform::Capitalize, None),
            "Key:Value"
        );
    }

    #[test]
    fn capitalize_smart_apostrophe_not_word_boundary() {
        // Right single quotation mark U+2019 within a word should NOT start
        // a new word, just like ASCII apostrophe.
        assert_eq!(
            apply_text_transform("don\u{2019}t stop", TextTransform::Capitalize, None),
            "Don\u{2019}t Stop"
        );
    }
}
