//! Locale-aware case mapping tests for `apply_text_transform`.
//!
//! Verifies that locale-specific rules match Blink's `CaseMap` behaviour
//! (backed by ICU). Each section documents the Unicode SpecialCasing.txt
//! or CLDR rule it validates.

use openui_style::TextTransform;
use openui_text::transform::apply_text_transform;

// ═══════════════════════════════════════════════════════════════════════════
// Turkish / Azerbaijani (tr / az) — dotted vs dotless I
// Unicode SpecialCasing.txt: 0069↔0130, 0049↔0131, 0130→0069, 0049 0307→0069
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn tr_upper_i_to_i_dot() {
    // Turkish uppercase: i (U+0069) → İ (U+0130)
    assert_eq!(
        apply_text_transform("i", TextTransform::Uppercase, Some("tr")),
        "\u{0130}"
    );
}

#[test]
fn tr_upper_dotless_i_to_capital_i() {
    // Turkish uppercase: ı (U+0131) → I (U+0049)
    assert_eq!(
        apply_text_transform("\u{0131}", TextTransform::Uppercase, Some("tr")),
        "I"
    );
}

#[test]
fn tr_upper_mixed_text() {
    // "istanbul" → "İSTANBUL" (not "ISTANBUL")
    assert_eq!(
        apply_text_transform("istanbul", TextTransform::Uppercase, Some("tr")),
        "\u{0130}STANBUL"
    );
}

#[test]
fn tr_lower_capital_i_to_dotless() {
    // Turkish lowercase: I (U+0049) → ı (U+0131)
    assert_eq!(
        apply_text_transform("I", TextTransform::Lowercase, Some("tr")),
        "\u{0131}"
    );
}

#[test]
fn tr_lower_i_dot_to_i() {
    // Turkish lowercase: İ (U+0130) → i (U+0069)
    assert_eq!(
        apply_text_transform("\u{0130}", TextTransform::Lowercase, Some("tr")),
        "i"
    );
}

#[test]
fn tr_lower_i_combining_dot_above() {
    // Turkish lowercase: I + combining dot above (U+0307) → i
    // The combining dot above is absorbed.
    assert_eq!(
        apply_text_transform("I\u{0307}", TextTransform::Lowercase, Some("tr")),
        "i"
    );
}

#[test]
fn tr_lower_mixed_text() {
    // "ISTANBUL" → "ıstanbul" (not "istanbul")
    assert_eq!(
        apply_text_transform("ISTANBUL", TextTransform::Lowercase, Some("tr")),
        "\u{0131}stanbul"
    );
}

#[test]
fn tr_capitalize_istanbul() {
    // Turkish capitalize: "istanbul" → "İstanbul" (not "Istanbul")
    assert_eq!(
        apply_text_transform("istanbul", TextTransform::Capitalize, Some("tr")),
        "\u{0130}stanbul"
    );
}

#[test]
fn tr_capitalize_dotless_i_word_start() {
    // Turkish capitalize: ı at word start → I
    assert_eq!(
        apply_text_transform("\u{0131}stanbul", TextTransform::Capitalize, Some("tr")),
        "Istanbul"
    );
}

#[test]
fn tr_round_trip_lower_upper() {
    // Turkish round-trip: lower then upper should produce consistent results.
    let lowered = apply_text_transform("DİYARBAKIR", TextTransform::Lowercase, Some("tr"));
    assert_eq!(lowered, "diyarbak\u{0131}r");
    let uppered = apply_text_transform(&lowered, TextTransform::Uppercase, Some("tr"));
    assert_eq!(uppered, "D\u{0130}YARBAKIR");
}

#[test]
fn az_upper_same_as_turkish() {
    // Azerbaijani uses the same rules as Turkish.
    assert_eq!(
        apply_text_transform("i", TextTransform::Uppercase, Some("az")),
        "\u{0130}"
    );
    assert_eq!(
        apply_text_transform("I", TextTransform::Lowercase, Some("az")),
        "\u{0131}"
    );
}

#[test]
fn tr_upper_preserves_non_latin() {
    // Turkish rules only affect Latin i/ı; other scripts pass through.
    assert_eq!(
        apply_text_transform("iαi", TextTransform::Uppercase, Some("tr")),
        "\u{0130}Α\u{0130}"
    );
}

#[test]
fn tr_locale_with_region() {
    // "tr-TR" should still be recognised as Turkish.
    assert_eq!(
        apply_text_transform("i", TextTransform::Uppercase, Some("tr-TR")),
        "\u{0130}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Greek (el) — strip tonos on uppercase
// CLDR Greek uppercase: accented vowels lose their tonos; dialytika is kept.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn el_upper_alpha_tonos() {
    // ά (U+03AC) → Α (U+0391)
    assert_eq!(
        apply_text_transform("\u{03AC}", TextTransform::Uppercase, Some("el")),
        "\u{0391}"
    );
}

#[test]
fn el_upper_epsilon_tonos() {
    // έ (U+03AD) → Ε (U+0395)
    assert_eq!(
        apply_text_transform("\u{03AD}", TextTransform::Uppercase, Some("el")),
        "\u{0395}"
    );
}

#[test]
fn el_upper_eta_tonos() {
    // ή (U+03AE) → Η (U+0397)
    assert_eq!(
        apply_text_transform("\u{03AE}", TextTransform::Uppercase, Some("el")),
        "\u{0397}"
    );
}

#[test]
fn el_upper_iota_tonos() {
    // ί (U+03AF) → Ι (U+0399)
    assert_eq!(
        apply_text_transform("\u{03AF}", TextTransform::Uppercase, Some("el")),
        "\u{0399}"
    );
}

#[test]
fn el_upper_omicron_tonos() {
    // ό (U+03CC) → Ο (U+039F)
    assert_eq!(
        apply_text_transform("\u{03CC}", TextTransform::Uppercase, Some("el")),
        "\u{039F}"
    );
}

#[test]
fn el_upper_upsilon_tonos() {
    // ύ (U+03CD) → Υ (U+03A5)
    assert_eq!(
        apply_text_transform("\u{03CD}", TextTransform::Uppercase, Some("el")),
        "\u{03A5}"
    );
}

#[test]
fn el_upper_omega_tonos() {
    // ώ (U+03CE) → Ω (U+03A9)
    assert_eq!(
        apply_text_transform("\u{03CE}", TextTransform::Uppercase, Some("el")),
        "\u{03A9}"
    );
}

#[test]
fn el_upper_iota_dialytika_tonos() {
    // ΐ (U+0390) → Ϊ (U+03AA) — dialytika kept, tonos removed.
    assert_eq!(
        apply_text_transform("\u{0390}", TextTransform::Uppercase, Some("el")),
        "\u{03AA}"
    );
}

#[test]
fn el_upper_upsilon_dialytika_tonos() {
    // ΰ (U+03B0) → Ϋ (U+03AB) — dialytika kept, tonos removed.
    assert_eq!(
        apply_text_transform("\u{03B0}", TextTransform::Uppercase, Some("el")),
        "\u{03AB}"
    );
}

#[test]
fn el_upper_mixed_word() {
    // "αθήνα" → "ΑΘΗΝΑ" (accent stripped from ή)
    assert_eq!(
        apply_text_transform("\u{03B1}\u{03B8}\u{03AE}\u{03BD}\u{03B1}", TextTransform::Uppercase, Some("el")),
        "\u{0391}\u{0398}\u{0397}\u{039D}\u{0391}"
    );
}

#[test]
fn el_upper_no_accent_chars_unchanged() {
    // Unaccented Greek uppercases normally.
    assert_eq!(
        apply_text_transform("αβγ", TextTransform::Uppercase, Some("el")),
        "ΑΒΓ"
    );
}

#[test]
fn el_upper_combining_acute_after_greek() {
    // α + combining acute (U+0301) → Α (combining mark stripped)
    assert_eq!(
        apply_text_transform("\u{03B1}\u{0301}", TextTransform::Uppercase, Some("el")),
        "\u{0391}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Greek sigma — context-dependent σ vs ς (default Unicode rules)
// Rust's str::to_lowercase() handles this per Unicode Default Case Conversion.
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn sigma_word_final() {
    // Σ at end of word → ς (final sigma)
    assert_eq!(
        apply_text_transform("ΚΟΣΜΟΣ", TextTransform::Lowercase, None),
        "κοσμος" // Wait... let me check
    );
    // Actually: ΚΟΣΜΟΣ → κοσμος. The final Σ → ς, middle Σ → σ.
    // κ-ο-σ-μ-ο-ς
    let result = apply_text_transform("ΚΟΣΜΟΣ", TextTransform::Lowercase, None);
    assert!(result.ends_with('ς'), "Final sigma should be ς, got: {}", result);
}

#[test]
fn sigma_word_medial() {
    // Σ in middle of word → σ
    let result = apply_text_transform("ΚΟΣΜΟΣ", TextTransform::Lowercase, None);
    // Position 2 (0-indexed) should be σ (medial)
    let chars: Vec<char> = result.chars().collect();
    assert_eq!(chars[2], 'σ', "Medial sigma should be σ");
}

#[test]
fn sigma_isolated() {
    // Single Σ with NO preceding cased letter does NOT satisfy Final_Sigma
    // (Unicode SpecialCasing.txt requires a preceding cased letter).
    assert_eq!(
        apply_text_transform("Σ", TextTransform::Lowercase, None),
        "σ"
    );
    // But with a preceding cased letter, it IS final:
    assert_eq!(
        apply_text_transform("ΑΣ", TextTransform::Lowercase, None),
        "ας"
    );
}

#[test]
fn sigma_before_space() {
    // Σ before space → ς (word-final)
    let result = apply_text_transform("ΣΟΣ ΣΟΣ", TextTransform::Lowercase, None);
    // Each word "ΣΟΣ" → "σος" (medial σ, final ς)
    assert!(result.contains("σος"), "Expected σος in: {}", result);
}

#[test]
fn sigma_all_caps_word() {
    // "ΟΔΥΣΣΕΥΣ" → "οδυσσευς" (both medial σ and final ς)
    let result = apply_text_transform("ΟΔΥΣΣΕΥΣ", TextTransform::Lowercase, None);
    assert!(result.ends_with('ς'), "Expected final ς in: {}", result);
    // The two Σ in the middle should be σ
    assert!(result.contains("σσ"), "Expected medial σσ in: {}", result);
}

// ═══════════════════════════════════════════════════════════════════════════
// Lithuanian (lt) — dot-above preservation in lowercase
// Unicode SpecialCasing.txt: I/J/Į + More_Above → insert U+0307
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn lt_lower_i_grave() {
    // Ì (U+00CC) → i + combining dot above + combining grave
    assert_eq!(
        apply_text_transform("\u{00CC}", TextTransform::Lowercase, Some("lt")),
        "i\u{0307}\u{0300}"
    );
}

#[test]
fn lt_lower_i_acute() {
    // Í (U+00CD) → i + combining dot above + combining acute
    assert_eq!(
        apply_text_transform("\u{00CD}", TextTransform::Lowercase, Some("lt")),
        "i\u{0307}\u{0301}"
    );
}

#[test]
fn lt_lower_i_tilde() {
    // Ĩ (U+0128) → i + combining dot above + combining tilde
    assert_eq!(
        apply_text_transform("\u{0128}", TextTransform::Lowercase, Some("lt")),
        "i\u{0307}\u{0303}"
    );
}

#[test]
fn lt_lower_capital_i_before_combining_accent() {
    // I + combining acute (U+0301) → i + dot above + combining acute
    assert_eq!(
        apply_text_transform("I\u{0301}", TextTransform::Lowercase, Some("lt")),
        "i\u{0307}\u{0301}"
    );
}

#[test]
fn lt_lower_capital_j_before_combining_accent() {
    // J + combining acute (U+0301) → j + dot above + combining acute
    assert_eq!(
        apply_text_transform("J\u{0301}", TextTransform::Lowercase, Some("lt")),
        "j\u{0307}\u{0301}"
    );
}

#[test]
fn lt_lower_capital_i_no_accent_after() {
    // I not followed by accent → just i (standard lowercase)
    assert_eq!(
        apply_text_transform("I", TextTransform::Lowercase, Some("lt")),
        "i"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Dutch (nl) — IJ digraph titlecasing
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn nl_capitalize_ij_digraph() {
    // "ijsselmeer" → "IJsselmeer" (both i and j uppercased)
    assert_eq!(
        apply_text_transform("ijsselmeer", TextTransform::Capitalize, Some("nl")),
        "IJsselmeer"
    );
}

#[test]
fn nl_capitalize_ij_mid_word() {
    // "bijzonder" → "Bijzonder" (ij not at word start, so only B capitalized)
    assert_eq!(
        apply_text_transform("bijzonder", TextTransform::Capitalize, Some("nl")),
        "Bijzonder"
    );
}

#[test]
fn nl_capitalize_multiple_words() {
    // "ijs en ijsje" → "IJs En IJsje"
    assert_eq!(
        apply_text_transform("ijs en ijsje", TextTransform::Capitalize, Some("nl")),
        "IJs En IJsje"
    );
}

#[test]
fn nl_capitalize_uppercase_ij() {
    // "IJsselmeer" → "IJsselmeer" (already correct)
    assert_eq!(
        apply_text_transform("IJsselmeer", TextTransform::Capitalize, Some("nl")),
        "IJsselmeer"
    );
}

#[test]
fn nl_capitalize_unicode_ij_digraph() {
    // ĳ (U+0133) at word start → Ĳ (U+0132, handled by to_titlecase)
    assert_eq!(
        apply_text_transform("\u{0133}ssel", TextTransform::Capitalize, Some("nl")),
        "\u{0132}ssel"
    );
}

#[test]
fn nl_locale_with_region() {
    // "nl-NL" should still be recognised as Dutch.
    assert_eq!(
        apply_text_transform("ijsselmeer", TextTransform::Capitalize, Some("nl-NL")),
        "IJsselmeer"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Default locale — no special rules
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn default_locale_none() {
    // None locale uses Unicode Default Case Conversion.
    assert_eq!(
        apply_text_transform("hello", TextTransform::Uppercase, None),
        "HELLO"
    );
}

#[test]
fn default_locale_empty_string() {
    // Empty string locale treated as default.
    assert_eq!(
        apply_text_transform("hello", TextTransform::Uppercase, Some("")),
        "HELLO"
    );
}

#[test]
fn default_locale_english() {
    // English has no special case rules.
    assert_eq!(
        apply_text_transform("hello", TextTransform::Uppercase, Some("en")),
        "HELLO"
    );
    assert_eq!(
        apply_text_transform("HELLO", TextTransform::Lowercase, Some("en")),
        "hello"
    );
}

#[test]
fn default_locale_french() {
    // French has no special case rules. Accents are preserved.
    assert_eq!(
        apply_text_transform("café", TextTransform::Uppercase, Some("fr")),
        "CAFÉ"
    );
}

#[test]
fn default_locale_german_eszett() {
    // German ß → SS in uppercase (Unicode default, not locale-specific).
    assert_eq!(
        apply_text_transform("straße", TextTransform::Uppercase, Some("de")),
        "STRASSE"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Mixed scripts with locale
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn tr_upper_mixed_latin_cjk() {
    // Turkish rules affect Latin 'i' but not CJK.
    assert_eq!(
        apply_text_transform("i世界i", TextTransform::Uppercase, Some("tr")),
        "\u{0130}世界\u{0130}"
    );
}

#[test]
fn el_upper_mixed_greek_latin() {
    // Greek accent stripping only on Greek; Latin unaffected.
    assert_eq!(
        apply_text_transform("café ά", TextTransform::Uppercase, Some("el")),
        "CAF\u{00C9} \u{0391}"
    );
}

// ═══════════════════════════════════════════════════════════════════════════
// Edge cases
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn empty_string_with_locale() {
    assert_eq!(apply_text_transform("", TextTransform::Uppercase, Some("tr")), "");
    assert_eq!(apply_text_transform("", TextTransform::Lowercase, Some("el")), "");
    assert_eq!(apply_text_transform("", TextTransform::Capitalize, Some("nl")), "");
}

#[test]
fn single_char_turkish() {
    assert_eq!(apply_text_transform("i", TextTransform::Uppercase, Some("tr")), "\u{0130}");
    assert_eq!(apply_text_transform("I", TextTransform::Lowercase, Some("tr")), "\u{0131}");
}

#[test]
fn locale_case_insensitive() {
    // BCP 47 tags are case-insensitive.
    assert_eq!(
        apply_text_transform("i", TextTransform::Uppercase, Some("TR")),
        "\u{0130}"
    );
    assert_eq!(
        apply_text_transform("i", TextTransform::Uppercase, Some("Tr")),
        "\u{0130}"
    );
}

#[test]
fn fullwidth_ignores_locale() {
    // Full-width transform is locale-independent.
    assert_eq!(
        apply_text_transform("ABC", TextTransform::FullWidth, Some("tr")),
        "\u{FF21}\u{FF22}\u{FF23}"
    );
}

#[test]
fn fullsize_kana_ignores_locale() {
    // Full-size-kana transform is locale-independent.
    assert_eq!(
        apply_text_transform("ぁ", TextTransform::FullSizeKana, Some("tr")),
        "あ"
    );
}

#[test]
fn none_ignores_locale() {
    assert_eq!(
        apply_text_transform("hello", TextTransform::None, Some("tr")),
        "hello"
    );
}

#[test]
fn tr_capitalize_multiple_words() {
    // "istanbul ilçesi" → "İstanbul İlçesi"
    assert_eq!(
        apply_text_transform("istanbul ilçesi", TextTransform::Capitalize, Some("tr")),
        "\u{0130}stanbul \u{0130}lçesi"
    );
}

#[test]
fn el_lowercase_uses_default_sigma() {
    // Greek lowercase with el locale still handles sigma correctly
    // (lowercase dispatch falls through to default for Greek).
    let result = apply_text_transform("ΚΟΣΜΟΣ", TextTransform::Lowercase, Some("el"));
    assert!(result.ends_with('ς'), "Expected final ς in Greek lowercase: {}", result);
}
