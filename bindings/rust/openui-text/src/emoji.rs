//! Emoji detection and classification utilities.
//!
//! Identifies emoji characters, variation selectors, and emoji sequence
//! components. Used to ensure the text pipeline routes emoji through the
//! correct rendering path (color glyphs via Skia's native color font
//! support) rather than forcing a monochrome paint color.
//!
//! Unicode ranges are sourced from:
//! - Unicode® Technical Standard #51: Unicode Emoji
//!   <https://www.unicode.org/reports/tr51/>
//! - emoji-data.txt from the Unicode Character Database
//!   <https://www.unicode.org/Public/UCD/latest/ucd/emoji/emoji-data.txt>
//!
//! Blink reference: `third_party/blink/renderer/platform/text/character.h`
//! (`IsEmoji`, `IsEmojiTextDefault`, `IsEmojiModifierBase`).

/// Returns `true` if the character is an emoji or emoji-related code point.
///
/// Covers the core emoji blocks, common symbols that have emoji presentation,
/// ZWJ (used in emoji sequences), variation selectors, keycap combining mark,
/// and tag characters (used in flag subdivision sequences like 🏴󠁧󠁢󠁥󠁮󠁧󠁿).
///
/// This intentionally casts a wide net — it is used to *detect* whether a
/// run may contain color glyphs, not to determine final presentation. The
/// font and variation selectors make the final presentation decision.
pub fn is_emoji(ch: char) -> bool {
    let cp = ch as u32;
    matches!(cp,
        // ── Watch, Hourglass ────────────────────────────────────────
        0x231A..=0x231B |

        // ── Various clocks and controls ─────────────────────────────
        0x23E9..=0x23F3 |
        0x23F8..=0x23FA |

        // ── Black/White small square ────────────────────────────────
        0x25AA..=0x25AB |

        // ── Play buttons ────────────────────────────────────────────
        0x25B6 |
        0x25C0 |

        // ── Medium squares ──────────────────────────────────────────
        0x25FB..=0x25FE |

        // ── Miscellaneous Symbols + Dingbats (emoji subset) ─────────
        0x2600..=0x27BF |

        // ── Supplemental Arrows-B (rightwards arrow) ────────────────
        0x2934..=0x2935 |

        // ── Miscellaneous Symbols and Arrows (partial) ──────────────
        0x2B05..=0x2B07 |
        0x2B1B..=0x2B1C |
        0x2B50 | 0x2B55 |

        // ── CJK Symbols — wavy dash, part alternation marks ─────────
        0x3030 | 0x303D |

        // ── Enclosed CJK — circled ideograph secret/congratulations ─
        0x3297 | 0x3299 |

        // ── Variation Selectors (VS15 text, VS16 emoji) ─────────────
        0xFE00..=0xFE0F |

        // ── Mahjong Tiles ───────────────────────────────────────────
        0x1F000..=0x1F02F |

        // ── Domino Tiles ────────────────────────────────────────────
        0x1F030..=0x1F09F |

        // ── Playing Cards ───────────────────────────────────────────
        0x1F0A0..=0x1F0FF |

        // ── Enclosed Alphanumeric Supplement ─────────────────────────
        0x1F100..=0x1F1FF |

        // ── Enclosed Ideographic Supplement ──────────────────────────
        0x1F200..=0x1F2FF |

        // ── Miscellaneous Symbols and Pictographs ───────────────────
        0x1F300..=0x1F5FF |

        // ── Emoticons ───────────────────────────────────────────────
        0x1F600..=0x1F64F |

        // ── Transport and Map Symbols ───────────────────────────────
        0x1F680..=0x1F6FF |

        // ── Alchemical Symbols ──────────────────────────────────────
        0x1F700..=0x1F77F |

        // ── Geometric Shapes Extended ───────────────────────────────
        0x1F780..=0x1F7FF |

        // ── Supplemental Arrows-C ───────────────────────────────────
        0x1F800..=0x1F8FF |

        // ── Supplemental Symbols and Pictographs ────────────────────
        0x1F900..=0x1F9FF |

        // ── Chess Symbols / Symbols and Pictographs Extended-A ──────
        0x1FA00..=0x1FA6F |
        0x1FA70..=0x1FAFF |

        // ── Symbols and Pictographs Extended-B ──────────────────────
        0x1FB00..=0x1FBFF |

        // ── Zero Width Joiner (used in ZWJ sequences like 👨‍👩‍👧‍👦) ──
        0x200D |

        // ── Combining Enclosing Keycap ──────────────────────────────
        0x20E3 |

        // ── Tags block (used in flag subdivision sequences) ─────────
        // E.g. 🏴󠁧󠁢󠁥󠁮󠁧󠁿 = U+1F3F4 + tag sequence + U+E007F
        0xE0020..=0xE007F
    )
}

/// Returns `true` if the character is Variation Selector 16 (U+FE0F),
/// which requests emoji presentation for the preceding character.
///
/// When VS16 follows a character that has both text and emoji presentations,
/// the emoji (color) presentation is selected.
#[inline]
pub fn is_emoji_presentation_selector(ch: char) -> bool {
    ch == '\u{FE0F}'
}

/// Returns `true` if the character is Variation Selector 15 (U+FE0E),
/// which requests text (monochrome) presentation for the preceding character.
///
/// When VS15 follows a character that has both text and emoji presentations,
/// the text (non-color) presentation is selected.
#[inline]
pub fn is_text_presentation_selector(ch: char) -> bool {
    ch == '\u{FE0E}'
}

/// Returns `true` if the character is a Zero Width Joiner (U+200D).
///
/// ZWJ is used to combine multiple emoji into a single glyph cluster,
/// e.g. 👨‍👩‍👧‍👦 (family) or 👩‍💻 (woman technologist).
/// During shaping, ZWJ sequences must be kept as a single cluster.
#[inline]
pub fn is_zero_width_joiner(ch: char) -> bool {
    ch == '\u{200D}'
}

/// Returns `true` if the character is an emoji modifier (skin tone).
///
/// Emoji modifiers (U+1F3FB–U+1F3FF) represent Fitzpatrick skin tone
/// types 1-2 through 6. They modify the preceding emoji base character.
#[inline]
pub fn is_emoji_modifier(ch: char) -> bool {
    let cp = ch as u32;
    (0x1F3FB..=0x1F3FF).contains(&cp)
}

/// Returns `true` if the character is a Regional Indicator Symbol
/// (U+1F1E6–U+1F1FF).
///
/// Pairs of regional indicators form flag emoji. For example,
/// 🇺🇸 = U+1F1FA (Regional Indicator U) + U+1F1F8 (Regional Indicator S).
/// During shaping, pairs of regional indicators must remain in a single
/// grapheme cluster.
#[inline]
pub fn is_regional_indicator(ch: char) -> bool {
    let cp = ch as u32;
    (0x1F1E6..=0x1F1FF).contains(&cp)
}

/// Returns `true` if the character is a tag character (U+E0020–U+E007F).
///
/// Tag characters are used in flag subdivision sequences (e.g.
/// 🏴󠁧󠁢󠁥󠁮󠁧󠁿 for England). The sequence is: black flag (U+1F3F4) +
/// tag letters + cancel tag (U+E007F).
#[inline]
pub fn is_tag_character(ch: char) -> bool {
    let cp = ch as u32;
    (0xE0020..=0xE007F).contains(&cp)
}

/// Returns `true` if a string contains any emoji characters.
///
/// Useful for quickly determining whether a text run might need the
/// color font rendering path. This checks individual code points; it
/// does not validate that they form well-formed emoji sequences.
pub fn contains_emoji(text: &str) -> bool {
    text.chars().any(is_emoji)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Core emoji detection ────────────────────────────────────────

    #[test]
    fn grinning_face_is_emoji() {
        assert!(is_emoji('😀')); // U+1F600
    }

    #[test]
    fn red_heart_is_emoji() {
        assert!(is_emoji('❤')); // U+2764
    }

    #[test]
    fn thumbs_up_is_emoji() {
        assert!(is_emoji('👍')); // U+1F44D
    }

    #[test]
    fn rocket_is_emoji() {
        assert!(is_emoji('🚀')); // U+1F680
    }

    #[test]
    fn sun_is_emoji() {
        assert!(is_emoji('☀')); // U+2600
    }

    #[test]
    fn watch_is_emoji() {
        assert!(is_emoji('⌚')); // U+231A
    }

    #[test]
    fn hourglass_is_emoji() {
        assert!(is_emoji('⌛')); // U+231B
    }

    #[test]
    fn black_small_square_is_emoji() {
        assert!(is_emoji('▪')); // U+25AA
    }

    #[test]
    fn play_button_is_emoji() {
        assert!(is_emoji('▶')); // U+25B6
    }

    #[test]
    fn reverse_button_is_emoji() {
        assert!(is_emoji('◀')); // U+25C0
    }

    // ── Emoticons block ─────────────────────────────────────────────

    #[test]
    fn first_emoticon_is_emoji() {
        assert!(is_emoji('\u{1F600}')); // 😀 Grinning Face
    }

    #[test]
    fn last_emoticon_is_emoji() {
        assert!(is_emoji('\u{1F64F}')); // 🙏 Person with Folded Hands
    }

    // ── Transport and Map Symbols ───────────────────────────────────

    #[test]
    fn airplane_is_emoji() {
        assert!(is_emoji('\u{2708}')); // ✈ Airplane
    }

    #[test]
    fn car_is_emoji() {
        assert!(is_emoji('\u{1F697}')); // 🚗 Automobile
    }

    // ── Supplemental Symbols and Pictographs ────────────────────────

    #[test]
    fn brain_is_emoji() {
        assert!(is_emoji('\u{1F9E0}')); // 🧠 Brain
    }

    #[test]
    fn superhero_is_emoji() {
        assert!(is_emoji('\u{1F9B8}')); // 🦸 Superhero
    }

    // ── Symbols and Pictographs Extended-A ──────────────────────────

    #[test]
    fn lungs_is_emoji() {
        assert!(is_emoji('\u{1FAC1}')); // 🫁 Lungs
    }

    #[test]
    fn mirror_ball_is_emoji() {
        assert!(is_emoji('\u{1FAA9}')); // 🪩 Mirror Ball
    }

    // ── Miscellaneous Symbols (partial) ─────────────────────────────

    #[test]
    fn umbrella_is_emoji() {
        assert!(is_emoji('\u{2614}')); // ☔ Umbrella with Rain Drops
    }

    #[test]
    fn hot_beverage_is_emoji() {
        assert!(is_emoji('\u{2615}')); // ☕ Hot Beverage
    }

    #[test]
    fn zodiac_aries_is_emoji() {
        assert!(is_emoji('\u{2648}')); // ♈ Aries
    }

    #[test]
    fn zodiac_pisces_is_emoji() {
        assert!(is_emoji('\u{2653}')); // ♓ Pisces
    }

    #[test]
    fn anchor_is_emoji() {
        assert!(is_emoji('\u{2693}')); // ⚓ Anchor
    }

    #[test]
    fn wheelchair_is_emoji() {
        assert!(is_emoji('\u{267F}')); // ♿ Wheelchair
    }

    // ── Mahjong, Domino, Playing Cards ──────────────────────────────

    #[test]
    fn mahjong_tile_is_emoji() {
        assert!(is_emoji('\u{1F004}')); // 🀄 Mahjong Tile Red Dragon
    }

    #[test]
    fn playing_card_is_emoji() {
        assert!(is_emoji('\u{1F0CF}')); // 🃏 Playing Card Joker
    }

    // ── Non-emoji characters ────────────────────────────────────────

    #[test]
    fn latin_a_is_not_emoji() {
        assert!(!is_emoji('A'));
    }

    #[test]
    fn digit_0_is_not_emoji() {
        assert!(!is_emoji('0'));
    }

    #[test]
    fn space_is_not_emoji() {
        assert!(!is_emoji(' '));
    }

    #[test]
    fn cjk_ideograph_is_not_emoji() {
        // 漢 (U+6F22) — CJK ideograph, not emoji
        assert!(!is_emoji('漢'));
    }

    #[test]
    fn hiragana_is_not_emoji() {
        assert!(!is_emoji('あ'));
    }

    #[test]
    fn cyrillic_is_not_emoji() {
        assert!(!is_emoji('Д'));
    }

    #[test]
    fn arabic_is_not_emoji() {
        assert!(!is_emoji('ع'));
    }

    #[test]
    fn basic_punctuation_is_not_emoji() {
        assert!(!is_emoji('.'));
        assert!(!is_emoji(','));
        assert!(!is_emoji(';'));
    }

    #[test]
    fn copyright_sign_is_not_emoji() {
        // U+00A9 — often rendered as emoji with VS16 but the base
        // character is below our emoji range threshold.
        assert!(!is_emoji('©'));
    }

    // ── Variation selectors ─────────────────────────────────────────

    #[test]
    fn vs16_is_emoji_presentation() {
        assert!(is_emoji_presentation_selector('\u{FE0F}'));
    }

    #[test]
    fn vs15_is_text_presentation() {
        assert!(is_text_presentation_selector('\u{FE0E}'));
    }

    #[test]
    fn vs16_is_not_text_presentation() {
        assert!(!is_text_presentation_selector('\u{FE0F}'));
    }

    #[test]
    fn vs15_is_not_emoji_presentation() {
        assert!(!is_emoji_presentation_selector('\u{FE0E}'));
    }

    #[test]
    fn variation_selectors_are_emoji() {
        // Both VS15 and VS16 are in the emoji detection range
        assert!(is_emoji('\u{FE0E}'));
        assert!(is_emoji('\u{FE0F}'));
    }

    #[test]
    fn other_variation_selectors_are_emoji() {
        // VS1 through VS14 (U+FE00–U+FE0D)
        assert!(is_emoji('\u{FE00}'));
        assert!(is_emoji('\u{FE0D}'));
    }

    // ── Zero Width Joiner ───────────────────────────────────────────

    #[test]
    fn zwj_detected() {
        assert!(is_zero_width_joiner('\u{200D}'));
    }

    #[test]
    fn zwj_is_emoji() {
        assert!(is_emoji('\u{200D}'));
    }

    #[test]
    fn non_zwj_not_detected() {
        assert!(!is_zero_width_joiner('\u{200C}')); // ZWNJ
        assert!(!is_zero_width_joiner('a'));
    }

    // ── Emoji modifiers (skin tones) ────────────────────────────────

    #[test]
    fn skin_tone_light_is_modifier() {
        assert!(is_emoji_modifier('\u{1F3FB}')); // Fitzpatrick Type 1-2
    }

    #[test]
    fn skin_tone_dark_is_modifier() {
        assert!(is_emoji_modifier('\u{1F3FF}')); // Fitzpatrick Type 6
    }

    #[test]
    fn skin_tone_medium_is_modifier() {
        assert!(is_emoji_modifier('\u{1F3FD}')); // Fitzpatrick Type 4
    }

    #[test]
    fn non_modifier_not_detected() {
        assert!(!is_emoji_modifier('A'));
        assert!(!is_emoji_modifier('\u{1F3FA}')); // Just below modifier range
        assert!(!is_emoji_modifier('\u{1F400}')); // Just above modifier range
    }

    #[test]
    fn modifiers_are_emoji() {
        assert!(is_emoji('\u{1F3FB}'));
        assert!(is_emoji('\u{1F3FF}'));
    }

    // ── Regional indicators (flags) ─────────────────────────────────

    #[test]
    fn regional_indicator_a() {
        assert!(is_regional_indicator('\u{1F1E6}')); // Regional Indicator A
    }

    #[test]
    fn regional_indicator_z() {
        assert!(is_regional_indicator('\u{1F1FF}')); // Regional Indicator Z
    }

    #[test]
    fn regional_indicators_are_emoji() {
        assert!(is_emoji('\u{1F1E6}'));
        assert!(is_emoji('\u{1F1FF}'));
    }

    #[test]
    fn non_regional_not_detected() {
        assert!(!is_regional_indicator('A'));
        assert!(!is_regional_indicator('\u{1F1E5}')); // Below range
        assert!(!is_regional_indicator('\u{1F200}')); // Above range
    }

    // ── Tag characters ──────────────────────────────────────────────

    #[test]
    fn tag_space_is_tag() {
        assert!(is_tag_character('\u{E0020}')); // Tag Space
    }

    #[test]
    fn tag_cancel_is_tag() {
        assert!(is_tag_character('\u{E007F}')); // Cancel Tag
    }

    #[test]
    fn tag_latin_g_is_tag() {
        assert!(is_tag_character('\u{E0067}')); // Tag Latin Small Letter G
    }

    #[test]
    fn tags_are_emoji() {
        assert!(is_emoji('\u{E0020}'));
        assert!(is_emoji('\u{E007F}'));
    }

    #[test]
    fn below_tag_range_not_detected() {
        assert!(!is_tag_character('\u{E001F}'));
    }

    #[test]
    fn above_tag_range_not_detected() {
        assert!(!is_tag_character('\u{E0080}'));
    }

    // ── Keycap combining mark ───────────────────────────────────────

    #[test]
    fn combining_enclosing_keycap() {
        assert!(is_emoji('\u{20E3}')); // Combining Enclosing Keycap
    }

    // ── contains_emoji ──────────────────────────────────────────────

    #[test]
    fn string_with_emoji() {
        assert!(contains_emoji("Hello 😀 World"));
    }

    #[test]
    fn string_without_emoji() {
        assert!(!contains_emoji("Hello World"));
    }

    #[test]
    fn empty_string_no_emoji() {
        assert!(!contains_emoji(""));
    }

    #[test]
    fn pure_emoji_string() {
        assert!(contains_emoji("😀🚀❤"));
    }

    #[test]
    fn cjk_text_no_emoji() {
        assert!(!contains_emoji("漢字テスト"));
    }

    #[test]
    fn mixed_text_with_flag_indicators() {
        // Regional indicators (flag components)
        assert!(contains_emoji("Go \u{1F1FA}\u{1F1F8}!"));
    }

    // ── Boundary / edge cases ───────────────────────────────────────

    #[test]
    fn just_below_misc_symbols_block() {
        assert!(!is_emoji('\u{25FF}')); // Not in emoji range
    }

    #[test]
    fn start_of_misc_symbols() {
        assert!(is_emoji('\u{2600}')); // ☀ Black Sun with Rays
    }

    #[test]
    fn end_of_dingbats_extended() {
        assert!(is_emoji('\u{27BF}')); // End of our Dingbats range
    }

    #[test]
    fn above_dingbats_not_emoji() {
        assert!(!is_emoji('\u{27C0}')); // Above our Dingbats range
    }

    #[test]
    fn start_of_mahjong() {
        assert!(is_emoji('\u{1F000}')); // 🀀 Mahjong Tile East Wind
    }

    #[test]
    fn null_character_is_not_emoji() {
        assert!(!is_emoji('\0'));
    }

    #[test]
    fn newline_is_not_emoji() {
        assert!(!is_emoji('\n'));
    }

    // ── Geometric shapes in extended range ──────────────────────────

    #[test]
    fn geometric_shapes_extended() {
        assert!(is_emoji('\u{1F780}')); // Start of Geometric Shapes Extended
    }

    // ── Supplemental Arrows-C ───────────────────────────────────────

    #[test]
    fn supplemental_arrows_c() {
        assert!(is_emoji('\u{1F800}')); // Start of Supplemental Arrows-C
    }

    // ── Chess symbols ───────────────────────────────────────────────

    #[test]
    fn chess_symbols() {
        assert!(is_emoji('\u{1FA00}')); // Start of Chess Symbols block
    }
}
