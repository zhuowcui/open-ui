//! Text transform — CSS `text-transform` property implementation.
//!
//! Mirrors Blink's text transform logic from
//! `third_party/blink/renderer/platform/text/text_transform.h`.
//!
//! Supports: none, uppercase, lowercase, capitalize, full-width, full-size-kana.

use openui_style::TextTransform;

/// Apply the CSS `text-transform` property to text.
///
/// Blink: `ComputedStyle::ApplyTextTransform` and related code in
/// `layout_text.cc` / `text_transform.cc`.
pub fn apply_text_transform(text: &str, transform: TextTransform) -> String {
    match transform {
        TextTransform::None => text.to_string(),
        TextTransform::Uppercase => text.to_uppercase(),
        TextTransform::Lowercase => text.to_lowercase(),
        TextTransform::Capitalize => capitalize(text),
        TextTransform::FullWidth => to_full_width(text),
        TextTransform::FullSizeKana => to_full_size_kana(text),
    }
}

/// Capitalize the first letter of each word.
///
/// Blink's definition of "word" for capitalize: a letter preceded by
/// a non-letter or at the start of the string. The CSS spec (§2.1)
/// says "first typographic letter unit of each word" where word
/// boundaries include spaces, hyphens, and other punctuation but
/// NOT apostrophes within words (e.g., "it's" is one word).
fn capitalize(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut capitalize_next = true;

    for ch in text.chars() {
        if capitalize_next && ch.is_alphabetic() {
            for upper in ch.to_uppercase() {
                result.push(upper);
            }
            capitalize_next = false;
        } else {
            result.push(ch);
            // Word boundary: any non-alphabetic character except apostrophes.
            // Apostrophe within a word (e.g., "don't") is NOT a word boundary
            // per CSS Text §2.1.
            if !ch.is_alphabetic() && ch != '\'' && ch != '\u{2019}' {
                capitalize_next = true;
            }
        }
    }
    result
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
        assert_eq!(apply_text_transform("Hello World", TextTransform::None), "Hello World");
    }

    #[test]
    fn transform_uppercase() {
        assert_eq!(apply_text_transform("hello", TextTransform::Uppercase), "HELLO");
    }

    #[test]
    fn transform_lowercase() {
        assert_eq!(apply_text_transform("HELLO", TextTransform::Lowercase), "hello");
    }

    #[test]
    fn transform_capitalize() {
        assert_eq!(apply_text_transform("hello world", TextTransform::Capitalize), "Hello World");
    }

    #[test]
    fn capitalize_after_hyphen() {
        assert_eq!(apply_text_transform("well-known", TextTransform::Capitalize), "Well-Known");
    }

    #[test]
    fn unicode_uppercase() {
        assert_eq!(apply_text_transform("café", TextTransform::Uppercase), "CAFÉ");
    }

    #[test]
    fn full_width_ascii() {
        assert_eq!(apply_text_transform("ABC", TextTransform::FullWidth), "ＡＢＣ");
    }

    #[test]
    fn full_width_space() {
        assert_eq!(apply_text_transform("A B", TextTransform::FullWidth), "Ａ\u{3000}Ｂ");
    }

    #[test]
    fn full_size_kana_converts_small_kana() {
        assert_eq!(
            apply_text_transform("ぁぃぅぇぉ", TextTransform::FullSizeKana),
            "あいうえお"
        );
    }

    #[test]
    fn full_size_kana_converts_small_katakana() {
        assert_eq!(
            apply_text_transform("ァィゥェォ", TextTransform::FullSizeKana),
            "アイウエオ"
        );
    }

    #[test]
    fn full_size_kana_preserves_normal_text() {
        assert_eq!(
            apply_text_transform("こんにちは", TextTransform::FullSizeKana),
            "こんにちは"
        );
    }

    // ── SP11 Round 17 Issue 2: ゕ (U+3095) and ゖ (U+3096) mappings ──

    #[test]
    fn full_size_kana_converts_small_hiragana_ka_ke() {
        assert_eq!(
            apply_text_transform("ゕゖ", TextTransform::FullSizeKana),
            "かけ"
        );
    }

    #[test]
    fn capitalize_apostrophe_not_word_boundary() {
        // CSS Text §2.1: apostrophe within a word does NOT start a new word.
        assert_eq!(
            apply_text_transform("it's a test", TextTransform::Capitalize),
            "It's A Test"
        );
    }

    // ── SP11 Round 11 Issue 5: capitalize after punctuation ──

    #[test]
    fn capitalize_after_open_paren() {
        assert_eq!(
            apply_text_transform("(hello) world", TextTransform::Capitalize),
            "(Hello) World"
        );
    }

    #[test]
    fn capitalize_after_slash() {
        assert_eq!(
            apply_text_transform("foo/bar", TextTransform::Capitalize),
            "Foo/Bar"
        );
    }

    #[test]
    fn capitalize_after_dot() {
        assert_eq!(
            apply_text_transform("first.second", TextTransform::Capitalize),
            "First.Second"
        );
    }

    #[test]
    fn capitalize_after_colon() {
        assert_eq!(
            apply_text_transform("key:value", TextTransform::Capitalize),
            "Key:Value"
        );
    }

    #[test]
    fn capitalize_smart_apostrophe_not_word_boundary() {
        // Right single quotation mark U+2019 within a word should NOT start
        // a new word, just like ASCII apostrophe.
        assert_eq!(
            apply_text_transform("don\u{2019}t stop", TextTransform::Capitalize),
            "Don\u{2019}t Stop"
        );
    }
}
