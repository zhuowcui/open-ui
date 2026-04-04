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
        TextTransform::FullSizeKana => text.to_string(), // Pass-through: rare, future work
    }
}

/// Capitalize the first letter of each word.
///
/// Blink's definition of "word" for capitalize: a letter preceded by
/// a non-letter or at the start of the string. The CSS spec (§2.1)
/// says "first typographic letter unit of each word" where word
/// boundaries include spaces, hyphens, and other punctuation.
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
            // Word boundaries: whitespace, hyphens, certain punctuation
            if ch.is_whitespace() || ch == '-' || ch == '\'' {
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
    fn full_size_kana_passthrough() {
        assert_eq!(
            apply_text_transform("こんにちは", TextTransform::FullSizeKana),
            "こんにちは"
        );
    }
}
