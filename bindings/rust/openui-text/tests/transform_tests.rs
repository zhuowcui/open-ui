//! Integration tests for `apply_text_transform` — edge cases and boundary conditions
//! beyond the basic coverage in the inline unit tests.

use openui_style::TextTransform;
use openui_text::transform::apply_text_transform;

// ═══════════════════════════════════════════════════════════════════════
// ── UPPERCASE EDGE CASES ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uppercase_preserves_numbers() {
    assert_eq!(
        apply_text_transform("abc123", TextTransform::Uppercase),
        "ABC123"
    );
}

#[test]
fn uppercase_preserves_punctuation() {
    assert_eq!(
        apply_text_transform("hello, world!", TextTransform::Uppercase),
        "HELLO, WORLD!"
    );
}

#[test]
fn uppercase_mixed_scripts_with_accents() {
    assert_eq!(
        apply_text_transform("café résumé", TextTransform::Uppercase),
        "CAFÉ RÉSUMÉ"
    );
}

#[test]
fn uppercase_idempotent() {
    assert_eq!(
        apply_text_transform("HELLO", TextTransform::Uppercase),
        "HELLO"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── LOWERCASE EDGE CASES ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn lowercase_mixed_case() {
    assert_eq!(
        apply_text_transform("HeLLo WoRLd", TextTransform::Lowercase),
        "hello world"
    );
}

#[test]
fn lowercase_preserves_numbers() {
    assert_eq!(
        apply_text_transform("ABC123", TextTransform::Lowercase),
        "abc123"
    );
}

#[test]
fn lowercase_unicode_accented() {
    assert_eq!(
        apply_text_transform("CAFÉ", TextTransform::Lowercase),
        "café"
    );
}

#[test]
fn lowercase_single_char() {
    assert_eq!(
        apply_text_transform("A", TextTransform::Lowercase),
        "a"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── CAPITALIZE EDGE CASES ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn capitalize_after_double_hyphen() {
    // Each hyphen sets the capitalize flag, so the letter after "--" is capitalized.
    assert_eq!(
        apply_text_transform("a--b", TextTransform::Capitalize),
        "A--B"
    );
}

#[test]
fn capitalize_numbers_pass_through_flag() {
    // Digits are not alphabetic, so they don't consume the start-of-string
    // capitalize flag — the first alphabetic char after them is capitalized.
    assert_eq!(
        apply_text_transform("123hello", TextTransform::Capitalize),
        "123Hello"
    );
}

#[test]
fn capitalize_tab_as_word_boundary() {
    assert_eq!(
        apply_text_transform("hello\tworld", TextTransform::Capitalize),
        "Hello\tWorld"
    );
}

#[test]
fn capitalize_newline_as_word_boundary() {
    assert_eq!(
        apply_text_transform("hello\nworld", TextTransform::Capitalize),
        "Hello\nWorld"
    );
}

#[test]
fn capitalize_already_uppercase() {
    assert_eq!(
        apply_text_transform("HELLO WORLD", TextTransform::Capitalize),
        "HELLO WORLD"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── FULL-WIDTH EDGE CASES ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn full_width_exclamation_and_tilde() {
    // U+0021 '!' → U+FF01 '！'
    assert_eq!(
        apply_text_transform("!", TextTransform::FullWidth),
        "！"
    );
    // U+007E '~' → U+FF5E '～'
    assert_eq!(
        apply_text_transform("~", TextTransform::FullWidth),
        "～"
    );
}

#[test]
fn full_width_digits() {
    // '0' U+0030 → U+FF10 '０'
    assert_eq!(
        apply_text_transform("0", TextTransform::FullWidth),
        "０"
    );
    // '9' U+0039 → U+FF19 '９'
    assert_eq!(
        apply_text_transform("9", TextTransform::FullWidth),
        "９"
    );
}

#[test]
fn full_width_mixed_ascii_and_non_ascii() {
    // ASCII 'A' maps to fullwidth; non-ASCII 'こ' passes through.
    assert_eq!(
        apply_text_transform("Aこ", TextTransform::FullWidth),
        "Ａこ"
    );
}

#[test]
fn full_width_control_chars_passthrough() {
    // Tab (U+0009) is not in printable ASCII range 0x21..=0x7E and not space,
    // so it passes through unchanged.
    assert_eq!(
        apply_text_transform("\t", TextTransform::FullWidth),
        "\t"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── NONE / FULL-SIZE-KANA ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn none_preserves_unicode_and_whitespace() {
    let input = "Héllo 世界\t🦀";
    assert_eq!(
        apply_text_transform(input, TextTransform::None),
        input
    );
}

#[test]
fn full_size_kana_passthrough_latin() {
    assert_eq!(
        apply_text_transform("Hello World", TextTransform::FullSizeKana),
        "Hello World"
    );
}

#[test]
fn full_size_kana_passthrough_mixed() {
    let input = "Hello こんにちは 123";
    assert_eq!(
        apply_text_transform(input, TextTransform::FullSizeKana),
        input
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── ADDITIONAL EDGE CASES ───────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn uppercase_german_eszett_expands() {
    // ß uppercases to SS in Unicode
    assert_eq!(
        apply_text_transform("straße", TextTransform::Uppercase),
        "STRASSE"
    );
}

#[test]
fn capitalize_after_apostrophe_mid_word() {
    // CSS Text §2.1: apostrophe within a word is NOT a word boundary.
    // "it's" is one word, so 's' is not capitalized.
    assert_eq!(
        apply_text_transform("it's a test", TextTransform::Capitalize),
        "It's A Test"
    );
}

#[test]
fn capitalize_only_first_letter_of_word() {
    // Only the first alphabetic char after a boundary is capitalized;
    // remaining letters in the word are untouched.
    assert_eq!(
        apply_text_transform("hELLO wORLD", TextTransform::Capitalize),
        "HELLO WORLD"
    );
}

#[test]
fn full_width_full_printable_ascii_range() {
    // Verify the entire printable ASCII range maps correctly.
    let input: String = (0x21u8..=0x7Eu8).map(|b| b as char).collect();
    let output = apply_text_transform(&input, TextTransform::FullWidth);
    for (i, ch) in output.chars().enumerate() {
        let expected = char::from_u32(0x21 + i as u32 + 0xFF01 - 0x21).unwrap();
        assert_eq!(ch, expected, "Mismatch at ASCII 0x{:02X}", 0x21 + i);
    }
}

#[test]
fn lowercase_german_eszett_unchanged() {
    // ß is already lowercase — lowercasing it should keep it as ß.
    assert_eq!(
        apply_text_transform("ß", TextTransform::Lowercase),
        "ß"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── SP11 ROUND 19 ISSUE 3: CAPITALIZE USES TITLECASE, NOT UPPERCASE ─
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn capitalize_titlecase_dz_digraph() {
    // ǳ (U+01F3) should titlecase to ǲ (U+01F2), NOT uppercase Ǳ (U+01F1).
    assert_eq!(
        apply_text_transform("\u{01F3}abc", TextTransform::Capitalize),
        "\u{01F2}abc"
    );
}

#[test]
fn capitalize_titlecase_lj_digraph() {
    // ǆ (U+01C6) should titlecase to ǅ (U+01C5).
    assert_eq!(
        apply_text_transform("\u{01C6}abc", TextTransform::Capitalize),
        "\u{01C5}abc"
    );
}

#[test]
fn capitalize_titlecase_nj_digraph() {
    // ǉ (U+01C9) should titlecase to ǈ (U+01C8).
    assert_eq!(
        apply_text_transform("\u{01C9}abc", TextTransform::Capitalize),
        "\u{01C8}abc"
    );
}

#[test]
fn capitalize_titlecase_dz_with_caron() {
    // ǌ (U+01CC) should titlecase to ǋ (U+01CB).
    assert_eq!(
        apply_text_transform("\u{01CC}abc", TextTransform::Capitalize),
        "\u{01CB}abc"
    );
}

#[test]
fn capitalize_titlecase_uppercase_dz_to_titlecase() {
    // Even Ǳ (U+01F1, uppercase) at word start should become ǲ (U+01F2, titlecase).
    assert_eq!(
        apply_text_transform("\u{01F1}abc", TextTransform::Capitalize),
        "\u{01F2}abc"
    );
}

#[test]
fn capitalize_normal_char_unaffected_by_titlecase() {
    // Normal ASCII chars: titlecase == uppercase. No regression.
    assert_eq!(
        apply_text_transform("hello world", TextTransform::Capitalize),
        "Hello World"
    );
}
