//! Character orientation for `text-orientation: mixed` vertical text.
//!
//! When `text-orientation: mixed` is in effect, CJK and certain symbol
//! characters are rendered upright while Latin, Cyrillic, and other
//! "narrow" scripts are rotated 90° clockwise.
//!
//! The classification follows Unicode UTR #50 (Unicode Vertical Text Layout)
//! and matches Blink's `Character::IsUprightInMixedVertical()` from
//! `third_party/blink/renderer/platform/text/character.h`.
//!
//! Reference: <https://www.unicode.org/reports/tr50/>

/// Returns `true` if `ch` should be rendered upright in vertical mixed mode.
///
/// Characters classified as U (Upright) or Tu (Transformed Upright) in
/// UTR #50 return `true`. Characters classified as R (Rotated) or Tr
/// (Transformed Rotated) return `false`.
///
/// This is a simplified but accurate table covering the blocks that appear
/// in real-world CJK content. Blink uses the ICU `uscript_getVerticalOrientation`
/// API plus manual overrides; our ranges are extracted from the same data.
pub fn is_upright_in_mixed_vertical(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        // CJK Radicals Supplement
        0x2E80..=0x2EFF |
        // Kangxi Radicals
        0x2F00..=0x2FDF |
        // Ideographic Description Characters
        0x2FF0..=0x2FFF |
        // CJK Symbols and Punctuation
        0x3000..=0x303F |
        // Hiragana
        0x3040..=0x309F |
        // Katakana
        0x30A0..=0x30FF |
        // Bopomofo
        0x3100..=0x312F |
        // Hangul Compatibility Jamo
        0x3130..=0x318F |
        // Kanbun
        0x3190..=0x319F |
        // Bopomofo Extended
        0x31A0..=0x31BF |
        // CJK Strokes
        0x31C0..=0x31EF |
        // Katakana Phonetic Extensions
        0x31F0..=0x31FF |
        // Enclosed CJK Letters and Months
        0x3200..=0x32FF |
        // CJK Compatibility
        0x3300..=0x33FF |
        // CJK Unified Ideographs Extension A
        0x3400..=0x4DBF |
        // Yijing Hexagram Symbols
        0x4DC0..=0x4DFF |
        // CJK Unified Ideographs
        0x4E00..=0x9FFF |
        // Yi Syllables
        0xA000..=0xA48F |
        // Yi Radicals
        0xA490..=0xA4CF |
        // Hangul Syllables
        0xAC00..=0xD7AF |
        // Hangul Jamo Extended-B
        0xD7B0..=0xD7FF |
        // CJK Compatibility Ideographs
        0xF900..=0xFAFF |
        // Vertical Forms
        0xFE10..=0xFE1F |
        // CJK Compatibility Forms
        0xFE30..=0xFE4F |
        // Small Form Variants
        0xFE50..=0xFE6F |
        // Halfwidth and Fullwidth Forms — fullwidth range only
        0xFF01..=0xFF60 |
        0xFFE0..=0xFFE6 |
        // Ideographic symbols and punctuation (Unicode 11.0+)
        0x16FE0..=0x16FFF |
        // Tangut
        0x17000..=0x187FF |
        // Tangut Components
        0x18800..=0x18AFF |
        // Khitan Small Script
        0x18B00..=0x18CFF |
        // Tangut Supplement
        0x18D00..=0x18D7F |
        // Kana Extended-B
        0x1AFF0..=0x1AFFF |
        // Kana Supplement
        0x1B000..=0x1B0FF |
        // Kana Extended-A
        0x1B100..=0x1B12F |
        // Small Kana Extension
        0x1B130..=0x1B16F |
        // Nushu
        0x1B170..=0x1B2FF |
        // Enclosed Ideographic Supplement
        0x1F200..=0x1F2FF |
        // CJK Unified Ideographs Extension B
        0x20000..=0x2A6DF |
        // CJK Unified Ideographs Extension C
        0x2A700..=0x2B73F |
        // CJK Unified Ideographs Extension D
        0x2B740..=0x2B81F |
        // CJK Unified Ideographs Extension E
        0x2B820..=0x2CEAF |
        // CJK Unified Ideographs Extension F
        0x2CEB0..=0x2EBEF |
        // CJK Compatibility Ideographs Supplement
        0x2F800..=0x2FA1F |
        // CJK Unified Ideographs Extension G
        0x30000..=0x3134F |
        // CJK Unified Ideographs Extension H
        0x31350..=0x323AF
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── CJK Ideographs ─────────────────────────────────────────────

    #[test]
    fn cjk_unified_ideograph_upright() {
        // 一 (U+4E00) — first CJK Unified Ideograph
        assert!(is_upright_in_mixed_vertical('一'));
    }

    #[test]
    fn cjk_ideograph_water() {
        // 水 (U+6C34)
        assert!(is_upright_in_mixed_vertical('水'));
    }

    #[test]
    fn cjk_ideograph_last() {
        // 鿿 (U+9FFF) — last of main CJK block
        assert!(is_upright_in_mixed_vertical('\u{9FFF}'));
    }

    #[test]
    fn cjk_extension_a() {
        assert!(is_upright_in_mixed_vertical('\u{3400}'));
    }

    #[test]
    fn cjk_extension_b() {
        assert!(is_upright_in_mixed_vertical('\u{20000}'));
    }

    // ── Kana ────────────────────────────────────────────────────────

    #[test]
    fn hiragana_a() {
        // あ (U+3042)
        assert!(is_upright_in_mixed_vertical('あ'));
    }

    #[test]
    fn katakana_a() {
        // ア (U+30A2)
        assert!(is_upright_in_mixed_vertical('ア'));
    }

    #[test]
    fn katakana_phonetic_extension() {
        assert!(is_upright_in_mixed_vertical('\u{31F0}'));
    }

    // ── Hangul ──────────────────────────────────────────────────────

    #[test]
    fn hangul_syllable_ga() {
        // 가 (U+AC00)
        assert!(is_upright_in_mixed_vertical('가'));
    }

    #[test]
    fn hangul_compatibility_jamo() {
        assert!(is_upright_in_mixed_vertical('\u{3131}')); // ㄱ
    }

    // ── CJK Symbols and Punctuation ────────────────────────────────

    #[test]
    fn ideographic_space() {
        // U+3000 — ideographic space
        assert!(is_upright_in_mixed_vertical('\u{3000}'));
    }

    #[test]
    fn ideographic_comma() {
        // 、 (U+3001)
        assert!(is_upright_in_mixed_vertical('、'));
    }

    #[test]
    fn ideographic_period() {
        // 。 (U+3002)
        assert!(is_upright_in_mixed_vertical('。'));
    }

    // ── Fullwidth Forms ─────────────────────────────────────────────

    #[test]
    fn fullwidth_exclamation() {
        // ！ (U+FF01)
        assert!(is_upright_in_mixed_vertical('！'));
    }

    #[test]
    fn fullwidth_a() {
        // Ａ (U+FF21)
        assert!(is_upright_in_mixed_vertical('Ａ'));
    }

    // ── Latin / Rotated scripts ─────────────────────────────────────

    #[test]
    fn latin_a_rotated() {
        assert!(!is_upright_in_mixed_vertical('A'));
    }

    #[test]
    fn latin_z_rotated() {
        assert!(!is_upright_in_mixed_vertical('z'));
    }

    #[test]
    fn digit_0_rotated() {
        assert!(!is_upright_in_mixed_vertical('0'));
    }

    #[test]
    fn cyrillic_a_rotated() {
        // А (U+0410)
        assert!(!is_upright_in_mixed_vertical('А'));
    }

    #[test]
    fn greek_alpha_rotated() {
        // α (U+03B1)
        assert!(!is_upright_in_mixed_vertical('α'));
    }

    #[test]
    fn arabic_alef_rotated() {
        // ا (U+0627)
        assert!(!is_upright_in_mixed_vertical('ا'));
    }

    #[test]
    fn basic_latin_punctuation_rotated() {
        assert!(!is_upright_in_mixed_vertical('.'));
        assert!(!is_upright_in_mixed_vertical(','));
        assert!(!is_upright_in_mixed_vertical('!'));
    }

    #[test]
    fn space_rotated() {
        assert!(!is_upright_in_mixed_vertical(' '));
    }

    // ── Boundary checks ─────────────────────────────────────────────

    #[test]
    fn just_below_cjk_radicals() {
        assert!(!is_upright_in_mixed_vertical('\u{2E7F}'));
    }

    #[test]
    fn start_of_cjk_radicals() {
        assert!(is_upright_in_mixed_vertical('\u{2E80}'));
    }

    #[test]
    fn halfwidth_katakana_rotated() {
        // Halfwidth katakana (U+FF61..U+FF9F) are in the halfwidth range, NOT fullwidth
        assert!(!is_upright_in_mixed_vertical('\u{FF61}'));
    }

    // ── Yi ──────────────────────────────────────────────────────────

    #[test]
    fn yi_syllable_upright() {
        assert!(is_upright_in_mixed_vertical('\u{A000}'));
    }

    // ── Enclosed / Compatibility ────────────────────────────────────

    #[test]
    fn enclosed_cjk() {
        // ㊀ (U+3280)
        assert!(is_upright_in_mixed_vertical('\u{3280}'));
    }

    #[test]
    fn cjk_compatibility_ideograph() {
        assert!(is_upright_in_mixed_vertical('\u{F900}'));
    }

    // ── Vertical Forms block ────────────────────────────────────────

    #[test]
    fn vertical_form() {
        assert!(is_upright_in_mixed_vertical('\u{FE10}'));
    }

    // ── Emoji (not in upright ranges) ───────────────────────────────

    #[test]
    fn basic_emoji_rotated() {
        // Most emoji are R (rotated) in UTR#50
        assert!(!is_upright_in_mixed_vertical('\u{1F600}')); // 😀
    }
}
