//! Tests for the CSS `line-break` property (CSS Text Module Level 3 §5.2).
//!
//! Validates that the `line-break` property correctly controls CJK line breaking
//! strictness: auto, loose, normal, strict, and anywhere.
//!
//! Each test exercises `find_break_opportunities` directly to verify break
//! positions, or uses the full `LineBreaker` to verify end-to-end behavior.

use openui_layout::inline::line_breaker::{find_break_opportunities, LineBreaker};
use openui_layout::inline::items::{InlineItem, InlineItemType, CollapseType};
use openui_layout::inline::items_builder::InlineItemsData;
use openui_geometry::LayoutUnit;
use openui_style::{ComputedStyle, LineBreak, OverflowWrap, WordBreak};
use openui_dom::NodeId;
use openui_text::{Font, FontDescription, TextShaper, TextDirection};
use std::sync::Arc;

// ── Helper ──────────────────────────────────────────────────────────────

fn breaks(text: &str, word_break: WordBreak, line_break: LineBreak) -> Vec<usize> {
    find_break_opportunities(text, word_break, OverflowWrap::Normal, line_break)
}

fn breaks_normal(text: &str, line_break: LineBreak) -> Vec<usize> {
    breaks(text, WordBreak::Normal, line_break)
}

// ════════════════════════════════════════════════════════════════════════
// 1. line-break: anywhere — breaks at every typographic character unit
// ════════════════════════════════════════════════════════════════════════

#[test]
fn anywhere_breaks_between_every_latin_char() {
    let b = breaks_normal("abc", LineBreak::Anywhere);
    assert_eq!(b, vec![1, 2], "anywhere should break between every character");
}

#[test]
fn anywhere_breaks_between_every_cjk_char() {
    // "世界好" = 3 CJK ideographs, each 3 bytes in UTF-8
    let text = "世界好";
    let b = breaks_normal(text, LineBreak::Anywhere);
    // Expect breaks at byte offsets after each char (3, 6)
    assert_eq!(b.len(), 2, "anywhere: 3 CJK chars should have 2 breaks, got {:?}", b);
}

#[test]
fn anywhere_breaks_within_latin_word() {
    // "hello" — no space; normally no breaks, but anywhere breaks everywhere
    let b = breaks_normal("hello", LineBreak::Anywhere);
    assert_eq!(b, vec![1, 2, 3, 4], "anywhere should break within a Latin word");
}

#[test]
fn anywhere_breaks_in_mixed_cjk_latin() {
    // "aあb" — 1 Latin + 1 Hiragana + 1 Latin
    let text = "aあb";
    let b = breaks_normal(text, LineBreak::Anywhere);
    // 'a' = 1 byte, 'あ' = 3 bytes, 'b' = 1 byte → breaks at 1, 4
    assert_eq!(b, vec![1, 4], "anywhere: mixed CJK+Latin should break at every grapheme");
}

#[test]
fn anywhere_single_char_no_break() {
    let b = breaks_normal("x", LineBreak::Anywhere);
    assert!(b.is_empty(), "single char should have no break opportunities");
}

#[test]
fn anywhere_empty_text_no_break() {
    let b = breaks_normal("", LineBreak::Anywhere);
    assert!(b.is_empty(), "empty text should have no break opportunities");
}

#[test]
fn anywhere_overrides_word_break_normal() {
    // Even with word-break: normal, line-break: anywhere should break everywhere
    let b = find_break_opportunities("abc", WordBreak::Normal, OverflowWrap::Normal, LineBreak::Anywhere);
    assert_eq!(b, vec![1, 2]);
}

#[test]
fn anywhere_overrides_word_break_keep_all() {
    // line-break: anywhere should override keep-all behavior
    let b = find_break_opportunities("漢字", WordBreak::KeepAll, OverflowWrap::Normal, LineBreak::Anywhere);
    assert!(!b.is_empty(), "anywhere should override keep-all");
}

#[test]
fn anywhere_preserves_grapheme_clusters() {
    // ZWJ emoji should be treated as a single typographic unit
    let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"; // 👨‍👩‍👧
    let text = format!("a{}b", family);
    let b = breaks_normal(&text, LineBreak::Anywhere);
    // Should NOT break inside the ZWJ sequence
    for &pos in &b {
        assert!(
            pos == 1 || pos == 1 + family.len(),
            "anywhere should not break inside ZWJ emoji; break at {} unexpected",
            pos,
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// 2. line-break: strict — prohibits breaks before certain CJK chars
// ════════════════════════════════════════════════════════════════════════

#[test]
fn strict_no_break_before_small_hiragana_a() {
    // "あぁ" — small kana ぁ (U+3041) should not have a break before it in strict
    let text = "あぁ";
    let b = breaks_normal(text, LineBreak::Strict);
    // In Normal mode UAX#14 may allow a break between them; Strict should remove it.
    let normal_b = breaks_normal(text, LineBreak::Normal);
    assert!(
        b.len() <= normal_b.len(),
        "strict should have fewer or equal breaks compared to normal: strict={:?}, normal={:?}",
        b, normal_b,
    );
}

#[test]
fn strict_no_break_before_small_katakana_a() {
    // "アァ" — small katakana ァ (U+30A1)
    let text = "アァ";
    let b = breaks_normal(text, LineBreak::Strict);
    // Verify no break before ァ
    let after_first = "ア".len(); // 3 bytes
    assert!(
        !b.contains(&after_first),
        "strict should not allow break before small katakana ァ; breaks={:?}",
        b,
    );
}

#[test]
fn strict_no_break_before_prolonged_sound_mark() {
    // "カー" — prolonged sound mark ー (U+30FC)
    let text = "カー";
    let b = breaks_normal(text, LineBreak::Strict);
    let after_first = "カ".len();
    assert!(
        !b.contains(&after_first),
        "strict should not allow break before prolonged sound mark ー; breaks={:?}",
        b,
    );
}

#[test]
fn strict_no_break_before_iteration_mark_noma() {
    // "人々" — iteration mark 々 (U+3005)
    let text = "人々";
    let b = breaks_normal(text, LineBreak::Strict);
    let after_first = "人".len();
    assert!(
        !b.contains(&after_first),
        "strict should not allow break before iteration mark 々; breaks={:?}",
        b,
    );
}

#[test]
fn strict_no_break_before_iteration_mark_vertical() {
    // "字〻" — vertical iteration mark 〻 (U+303B)
    let text = "字〻";
    let b = breaks_normal(text, LineBreak::Strict);
    let after_first = "字".len();
    assert!(
        !b.contains(&after_first),
        "strict should not allow break before vertical iteration mark 〻; breaks={:?}",
        b,
    );
}

#[test]
fn strict_no_break_before_ideographic_period() {
    // "世。" — ideographic full stop 。 (U+3002)
    let text = "世。";
    let b = breaks_normal(text, LineBreak::Strict);
    let after_first = "世".len();
    assert!(
        !b.contains(&after_first),
        "strict should not allow break before ideographic period 。; breaks={:?}",
        b,
    );
}

#[test]
fn strict_no_break_before_ideographic_comma() {
    // "世、" — ideographic comma 、 (U+3001)
    let text = "世、";
    let b = breaks_normal(text, LineBreak::Strict);
    let after_first = "世".len();
    assert!(
        !b.contains(&after_first),
        "strict should not allow break before ideographic comma 、; breaks={:?}",
        b,
    );
}

#[test]
fn strict_no_break_before_right_corner_bracket() {
    // "世」" — right corner bracket 」 (U+300D)
    let text = "世」";
    let b = breaks_normal(text, LineBreak::Strict);
    let after_first = "世".len();
    assert!(
        !b.contains(&after_first),
        "strict should not allow break before right corner bracket 」; breaks={:?}",
        b,
    );
}

#[test]
fn strict_allows_normal_cjk_breaks() {
    // Between two regular CJK ideographs, strict should still allow breaks
    let text = "世界"; // Neither char is in the strict-no-break list
    let b = breaks_normal(text, LineBreak::Strict);
    assert!(!b.is_empty(), "strict should allow breaks between regular CJK ideographs; breaks={:?}", b);
}

#[test]
fn strict_allows_latin_space_breaks() {
    let b = breaks_normal("hello world", LineBreak::Strict);
    assert!(
        b.contains(&6),
        "strict should not affect Latin space breaks; breaks={:?}",
        b,
    );
}

#[test]
fn strict_with_multiple_small_kana() {
    // "あぁいぃうぅ" — pairs of kana + small kana
    let text = "あぁいぃうぅ";
    let b = breaks_normal(text, LineBreak::Strict);
    // Breaks before small kana (ぁ at 3, ぃ at 9, ぅ at 15) should be removed
    let small_kana_positions: Vec<usize> = vec![3, 9, 15]; // byte offsets of small kana
    for &pos in &small_kana_positions {
        assert!(
            !b.contains(&pos),
            "strict should not break before small kana at byte {}: breaks={:?}",
            pos, b,
        );
    }
}

// ════════════════════════════════════════════════════════════════════════
// 3. line-break: loose — allows extra breaks around CJK comma/period
// ════════════════════════════════════════════════════════════════════════

#[test]
fn loose_allows_break_before_ideographic_comma() {
    // "世、界" — loose should allow a break before 、
    let text = "世、界";
    let b = breaks_normal(text, LineBreak::Loose);
    let comma_offset = "世".len(); // byte offset of 、
    assert!(
        b.contains(&comma_offset),
        "loose should allow break before ideographic comma 、; breaks={:?}",
        b,
    );
}

#[test]
fn loose_allows_break_before_ideographic_period() {
    // "世。界" — loose should allow a break before 。
    let text = "世。界";
    let b = breaks_normal(text, LineBreak::Loose);
    let period_offset = "世".len();
    assert!(
        b.contains(&period_offset),
        "loose should allow break before ideographic period 。; breaks={:?}",
        b,
    );
}

#[test]
fn loose_allows_break_before_fullwidth_comma() {
    // "世，界" — fullwidth comma ， (U+FF0C)
    let text = "世，界";
    let b = breaks_normal(text, LineBreak::Loose);
    let offset = "世".len();
    assert!(
        b.contains(&offset),
        "loose should allow break before fullwidth comma ，; breaks={:?}",
        b,
    );
}

#[test]
fn loose_allows_break_before_fullwidth_period() {
    // "世．界" — fullwidth full stop ． (U+FF0E)
    let text = "世．界";
    let b = breaks_normal(text, LineBreak::Loose);
    let offset = "世".len();
    assert!(
        b.contains(&offset),
        "loose should allow break before fullwidth full stop ．; breaks={:?}",
        b,
    );
}

#[test]
fn loose_preserves_normal_breaks() {
    // Loose should not remove any normal UAX#14 break opportunities
    let text = "hello world";
    let normal_b = breaks_normal(text, LineBreak::Normal);
    let loose_b = breaks_normal(text, LineBreak::Loose);
    for &pos in &normal_b {
        assert!(
            loose_b.contains(&pos),
            "loose should preserve all normal breaks; missing break at byte {}: normal={:?}, loose={:?}",
            pos, normal_b, loose_b,
        );
    }
}

#[test]
fn loose_more_breaks_than_strict_for_cjk() {
    // "世、界。好" — loose should have more breaks than strict
    let text = "世、界。好";
    let strict_b = breaks_normal(text, LineBreak::Strict);
    let loose_b = breaks_normal(text, LineBreak::Loose);
    assert!(
        loose_b.len() >= strict_b.len(),
        "loose should have >= breaks compared to strict: loose={:?}, strict={:?}",
        loose_b, strict_b,
    );
}

// ════════════════════════════════════════════════════════════════════════
// 4. line-break: normal — standard UAX#14 behavior
// ════════════════════════════════════════════════════════════════════════

#[test]
fn normal_cjk_breaks_between_ideographs() {
    let text = "世界好";
    let b = breaks_normal(text, LineBreak::Normal);
    assert!(!b.is_empty(), "normal should allow CJK ideograph breaks");
}

#[test]
fn normal_latin_space_breaks() {
    let b = breaks_normal("the quick brown fox", LineBreak::Normal);
    assert_eq!(b, vec![4, 10, 16], "normal should break at spaces in Latin text");
}

#[test]
fn normal_hyphen_break() {
    let b = breaks_normal("well-known", LineBreak::Normal);
    assert!(b.contains(&5), "normal should break after hyphen");
}

// ════════════════════════════════════════════════════════════════════════
// 5. line-break: auto — same behavior as normal
// ════════════════════════════════════════════════════════════════════════

#[test]
fn auto_same_as_normal_for_latin() {
    let text = "hello world test";
    let auto_b = breaks_normal(text, LineBreak::Auto);
    let normal_b = breaks_normal(text, LineBreak::Normal);
    assert_eq!(auto_b, normal_b, "auto should behave identically to normal for Latin text");
}

#[test]
fn auto_same_as_normal_for_cjk() {
    let text = "世界好人";
    let auto_b = breaks_normal(text, LineBreak::Auto);
    let normal_b = breaks_normal(text, LineBreak::Normal);
    assert_eq!(auto_b, normal_b, "auto should behave identically to normal for CJK text");
}

#[test]
fn auto_same_as_normal_for_mixed() {
    let text = "hello世界test";
    let auto_b = breaks_normal(text, LineBreak::Auto);
    let normal_b = breaks_normal(text, LineBreak::Normal);
    assert_eq!(auto_b, normal_b, "auto should behave identically to normal for mixed text");
}

// ════════════════════════════════════════════════════════════════════════
// 6. Interaction with word-break property
// ════════════════════════════════════════════════════════════════════════

#[test]
fn strict_with_break_all_still_restricts() {
    // word-break: break-all + line-break: strict
    // break-all adds grapheme-level breaks, but strict should still remove
    // breaks before strict-no-break characters
    let text = "カー"; // ー is strict-no-break-before
    let b = find_break_opportunities(text, WordBreak::BreakAll, OverflowWrap::Normal, LineBreak::Strict);
    let after_ka = "カ".len();
    assert!(
        !b.contains(&after_ka),
        "strict should still prohibit break before ー even with break-all; breaks={:?}",
        b,
    );
}

#[test]
fn loose_with_keep_all_adds_loose_breaks() {
    // word-break: keep-all + line-break: loose
    // keep-all suppresses CJK-between-CJK breaks, but loose should still add
    // breaks before comma/period
    let text = "世、界";
    let b = find_break_opportunities(text, WordBreak::KeepAll, OverflowWrap::Normal, LineBreak::Loose);
    let comma_offset = "世".len();
    assert!(
        b.contains(&comma_offset),
        "loose should add break before 、 even with keep-all; breaks={:?}",
        b,
    );
}

#[test]
fn anywhere_overrides_all_word_break_modes() {
    let text = "abc";
    for wb in [WordBreak::Normal, WordBreak::BreakAll, WordBreak::KeepAll, WordBreak::BreakWord] {
        let b = find_break_opportunities(text, wb, OverflowWrap::Normal, LineBreak::Anywhere);
        assert_eq!(b, vec![1, 2], "anywhere should produce same breaks regardless of word-break: {:?}", wb);
    }
}

// ════════════════════════════════════════════════════════════════════════
// 7. CJK text tests (Japanese, Chinese)
// ════════════════════════════════════════════════════════════════════════

#[test]
fn japanese_hiragana_strict_small_kana() {
    // Japanese text with small kana — strict prevents breaks before them
    // "おかあさん" → normal allows breaks, strict restricts before small chars
    // Use: "あぃう" where ぃ is small kana
    let text = "あぃう";
    let strict_b = breaks_normal(text, LineBreak::Strict);
    let small_i_offset = "あ".len(); // byte offset of ぃ
    assert!(
        !strict_b.contains(&small_i_offset),
        "strict should not break before small kana ぃ; breaks={:?}",
        strict_b,
    );
}

#[test]
fn chinese_ideographs_normal_breaks() {
    // Chinese text: "你好世界" — 4 ideographs
    let text = "你好世界";
    let b = breaks_normal(text, LineBreak::Normal);
    assert!(
        b.len() >= 2,
        "Chinese ideographs should have multiple break opportunities in normal mode; breaks={:?}",
        b,
    );
}

#[test]
fn chinese_with_comma_strict_vs_loose() {
    // "你，好" — Chinese with fullwidth comma
    let text = "你，好";
    let strict_b = breaks_normal(text, LineBreak::Strict);
    let loose_b = breaks_normal(text, LineBreak::Loose);
    // Loose should have break before ，; strict should not (it's in the no-break list
    // indirectly via the fullwidth comma in loose set — but ， is not in strict list,
    // so check that loose has more opportunities)
    assert!(
        loose_b.len() >= strict_b.len(),
        "loose should have >= breaks than strict for Chinese + comma: loose={:?}, strict={:?}",
        loose_b, strict_b,
    );
}

#[test]
fn japanese_katakana_prolonged_strict() {
    // "ラーメン" — ー after ラ is prolonged sound mark
    let text = "ラーメン";
    let strict_b = breaks_normal(text, LineBreak::Strict);
    let chouon_offset = "ラ".len(); // byte offset of ー
    assert!(
        !strict_b.contains(&chouon_offset),
        "strict should not break before ー in ラーメン; strict={:?}",
        strict_b,
    );
}

// ════════════════════════════════════════════════════════════════════════
// 8. Mixed CJK and Latin text
// ════════════════════════════════════════════════════════════════════════

#[test]
fn mixed_latin_cjk_normal_breaks() {
    let text = "Hello世界World";
    let b = breaks_normal(text, LineBreak::Normal);
    assert!(
        !b.is_empty(),
        "mixed Latin-CJK should have break opportunities; breaks={:?}",
        b,
    );
}

#[test]
fn mixed_latin_cjk_strict_preserves_latin_breaks() {
    let text = "hello world 世界";
    let b = breaks_normal(text, LineBreak::Strict);
    assert!(
        b.contains(&6), // after "hello "
        "strict should still allow Latin space breaks; breaks={:?}",
        b,
    );
}

#[test]
fn mixed_cjk_latin_anywhere_breaks_all() {
    let text = "aあb";
    let b = breaks_normal(text, LineBreak::Anywhere);
    assert_eq!(b.len(), 2, "anywhere should break at every grapheme boundary in mixed text; breaks={:?}", b);
}

// ════════════════════════════════════════════════════════════════════════
// 9. LineBreak enum properties
// ════════════════════════════════════════════════════════════════════════

#[test]
fn line_break_initial_is_auto() {
    assert_eq!(LineBreak::INITIAL, LineBreak::Auto);
}

#[test]
fn line_break_default_is_auto() {
    assert_eq!(LineBreak::default(), LineBreak::Auto);
}

#[test]
fn line_break_repr_values() {
    assert_eq!(LineBreak::Auto as u8, 0);
    assert_eq!(LineBreak::Loose as u8, 1);
    assert_eq!(LineBreak::Normal as u8, 2);
    assert_eq!(LineBreak::Strict as u8, 3);
    assert_eq!(LineBreak::Anywhere as u8, 4);
}

#[test]
fn line_break_equality() {
    assert_eq!(LineBreak::Auto, LineBreak::Auto);
    assert_ne!(LineBreak::Strict, LineBreak::Loose);
    assert_ne!(LineBreak::Anywhere, LineBreak::Normal);
}

#[test]
fn line_break_clone_copy() {
    let a = LineBreak::Strict;
    let b = a; // Copy
    let c = a.clone(); // Clone
    assert_eq!(a, b);
    assert_eq!(a, c);
}

#[test]
fn line_break_debug_format() {
    assert_eq!(format!("{:?}", LineBreak::Auto), "Auto");
    assert_eq!(format!("{:?}", LineBreak::Strict), "Strict");
}

// ════════════════════════════════════════════════════════════════════════
// 10. ComputedStyle integration
// ════════════════════════════════════════════════════════════════════════

#[test]
fn computed_style_default_line_break_is_auto() {
    let style = ComputedStyle::default();
    assert_eq!(style.line_break, LineBreak::Auto);
}

#[test]
fn computed_style_line_break_can_be_set() {
    let mut style = ComputedStyle::default();
    style.line_break = LineBreak::Strict;
    assert_eq!(style.line_break, LineBreak::Strict);
    style.line_break = LineBreak::Loose;
    assert_eq!(style.line_break, LineBreak::Loose);
    style.line_break = LineBreak::Anywhere;
    assert_eq!(style.line_break, LineBreak::Anywhere);
}

// ════════════════════════════════════════════════════════════════════════
// 11. Full LineBreaker integration with line-break: anywhere
// ════════════════════════════════════════════════════════════════════════

#[test]
fn linebreaker_anywhere_breaks_every_char() {
    let text = "abcde";
    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::default());
    let sr = shaper.shape(text, &font, TextDirection::Ltr);
    let sr_arc = Arc::new(sr);

    let mut style = ComputedStyle::default();
    style.line_break = LineBreak::Anywhere;

    let items_data = InlineItemsData {
        text: text.to_string(),
        items: vec![InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        }],
        styles: vec![style],
    };

    // Width to fit ~2 characters
    let two_char_width = sr_arc.width_for_range(0, 2);
    let narrow = LayoutUnit::from_f32(two_char_width + 0.5);

    let mut breaker = LineBreaker::new(&items_data, narrow);
    let mut lines = Vec::new();
    while let Some(line) = breaker.next_line(narrow) {
        lines.push(line);
    }

    // With anywhere + narrow width, we should get multiple lines
    assert!(
        lines.len() >= 2,
        "line-break: anywhere with narrow width should produce multiple lines; got {}",
        lines.len(),
    );
}

#[test]
fn linebreaker_strict_prevents_break_before_chouon() {
    // "カー" in a narrow container — strict should NOT break before ー
    let text = "カー";
    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::default());
    let sr = shaper.shape(text, &font, TextDirection::Ltr);
    let sr_arc = Arc::new(sr);

    let mut style = ComputedStyle::default();
    style.line_break = LineBreak::Strict;

    let items_data = InlineItemsData {
        text: text.to_string(),
        items: vec![InlineItem {
            item_type: InlineItemType::Text,
            text_range: 0..text.len(),
            node_id: NodeId::NONE,
            shape_result: Some(sr_arc.clone()),
            style_index: 0,
            end_collapse_type: CollapseType::NotCollapsible,
            is_end_collapsible_newline: false,
            bidi_level: 0,
            intrinsic_inline_size: None,
        }],
        styles: vec![style],
    };

    // Narrow width — less than the full text but more than one character
    let one_char_width = sr_arc.width_for_range(0, 1);
    let narrow = LayoutUnit::from_f32(one_char_width + 0.5);

    let mut breaker = LineBreaker::new(&items_data, narrow);
    let line1 = breaker.next_line(narrow);
    assert!(line1.is_some());
    let line1 = line1.unwrap();

    // In strict mode, there should be no break between カ and ー,
    // so the entire text should be forced on one line (overflow)
    let line1_end = line1.items.last().map(|i| i.text_range.end).unwrap_or(0);
    assert_eq!(
        line1_end,
        text.len(),
        "strict should force カー on one line (no break before ー); line ended at byte {}",
        line1_end,
    );
}
