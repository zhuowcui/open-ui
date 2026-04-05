//! BiDi integration tests — extended coverage for UAX#9 bidirectional
//! text analysis scenarios beyond the base test suite.
//!
//! Covers numbers in RTL context, neutral character resolution, base
//! direction auto-detection, deeply nested bidi, visual reordering
//! correctness, single-character edge cases, level access, and text
//! accessor invariants.

use openui_text::bidi::BidiParagraph;
use openui_text::TextDirection;

// ── Numbers in RTL context ──────────────────────────────────────────────

#[test]
fn bidi_digits_char_level_in_hebrew_context() {
    // Digits embedded in Hebrew get even (LTR) level via UAX#9 I2.
    let text = "שלום 123";
    let bidi = BidiParagraph::new(text, None);
    // chars: ש(0) ל(1) ו(2) ם(3) ' '(4) 1(5) 2(6) 3(7)
    for i in 5..=7 {
        let level = bidi.level_at(i);
        assert_eq!(
            level % 2,
            0,
            "Digit at char {} should have even level, got {}",
            i,
            level
        );
    }
}

#[test]
fn bidi_arabic_with_numbers_number_run_is_ltr() {
    let text = "مرحبا 123";
    let bidi = BidiParagraph::new(text, None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
    let runs = bidi.runs();
    let number_run = runs.iter().find(|r| {
        let slice = &bidi.text()[r.start..r.end];
        slice.contains('1')
    });
    assert!(number_run.is_some(), "Should have a run containing the digits");
    assert_eq!(
        number_run.unwrap().direction,
        TextDirection::Ltr,
        "Number run in Arabic context should be LTR"
    );
}

#[test]
fn bidi_forced_rtl_numbers_exact_level_two() {
    // Pure digits in a forced-RTL paragraph resolve to level 2 per UAX#9 I2.
    let bidi = BidiParagraph::new("999", Some(TextDirection::Rtl));
    assert_eq!(bidi.level_at(0), 2);
    assert_eq!(bidi.level_at(1), 2);
    assert_eq!(bidi.level_at(2), 2);
}

// ── Neutral characters ──────────────────────────────────────────────────

#[test]
fn bidi_space_between_hebrew_words_absorbs_rtl() {
    // In "hello שלום עולם world", the space between the two Hebrew words
    // is flanked by R on both sides → N1 resolves it to R direction.
    let text = "hello שלום עולם world";
    let bidi = BidiParagraph::new(text, None);
    // chars: h(0)e(1)l(2)l(3)o(4) (5)ש(6)ל(7)ו(8)ם(9) (10)ע(11)ו(12)ל(13)ם(14) (15)w(16)...
    let space_level = bidi.level_at(10);
    assert_eq!(
        space_level % 2,
        1,
        "Space between Hebrew words should be odd (RTL) level, got {}",
        space_level
    );
}

#[test]
fn bidi_punctuation_in_ltr_all_level_zero() {
    let bidi = BidiParagraph::new("Hello, world!", None);
    let runs = bidi.runs();
    for run in &runs {
        assert_eq!(
            run.level, 0,
            "All runs in pure LTR+punctuation should be level 0"
        );
    }
}

#[test]
fn bidi_comma_between_hebrew_gets_odd_level() {
    // Comma between two RTL segments resolves to R via N1.
    let bidi = BidiParagraph::new("שלום,עולם", None);
    // chars: ש(0)ל(1)ו(2)ם(3),(4)ע(5)ו(6)ל(7)ם(8)
    let comma_level = bidi.level_at(4);
    assert_eq!(
        comma_level % 2,
        1,
        "Comma between Hebrew words should be odd (RTL), got {}",
        comma_level
    );
}

// ── Base direction auto-detect ──────────────────────────────────────────

#[test]
fn bidi_auto_detect_arabic_first_strong_rtl() {
    // First strong character is Arabic → paragraph direction RTL.
    let bidi = BidiParagraph::new("مرحبا hello", None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
}

#[test]
fn bidi_auto_detect_spaces_then_latin_ltr() {
    // Leading neutral characters (spaces) then Latin → first strong is L → LTR.
    let bidi = BidiParagraph::new("   Hello", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
}

#[test]
fn bidi_only_weak_chars_default_ltr() {
    // Digits and spaces are weak/neutral — no strong characters → default LTR.
    let bidi = BidiParagraph::new("456 789", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    let runs = bidi.runs();
    for run in &runs {
        assert_eq!(run.direction, TextDirection::Ltr);
    }
}

// ── Deeply nested / complex bidi ────────────────────────────────────────

#[test]
fn bidi_english_hebrew_english_at_least_three_runs() {
    let bidi = BidiParagraph::new("abc שלום xyz", None);
    let runs = bidi.runs();
    assert!(
        runs.len() >= 3,
        "English-Hebrew-English should produce at least 3 runs, got {}",
        runs.len()
    );
    assert_eq!(runs[0].direction, TextDirection::Ltr);
    assert!(runs.iter().any(|r| r.direction == TextDirection::Rtl));
}

#[test]
fn bidi_hebrew_english_hebrew_at_least_three_runs() {
    let bidi = BidiParagraph::new("שלום abc עולם", None);
    let runs = bidi.runs();
    assert!(
        runs.len() >= 3,
        "Hebrew-English-Hebrew should produce at least 3 runs, got {}",
        runs.len()
    );
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
}

#[test]
fn bidi_long_mixed_byte_coverage_equals_length() {
    let text = "The שלום quick עולם brown fox jumps מעל the lazy dog";
    let bidi = BidiParagraph::new(text, None);
    let runs = bidi.runs();
    let total_bytes: usize = runs.iter().map(|r| r.end - r.start).sum();
    assert_eq!(
        total_bytes,
        text.len(),
        "Total byte coverage should equal text length"
    );
}

// ── Visual reordering correctness ───────────────────────────────────────

#[test]
fn bidi_visual_pure_rtl_single_run_rtl_direction() {
    let bidi = BidiParagraph::new("שלום עולם", None);
    let visual = bidi.visual_runs();
    assert_eq!(visual.len(), 1);
    assert_eq!(visual[0].direction, TextDirection::Rtl);
    assert_eq!(visual[0].level, 1);
}

#[test]
fn bidi_visual_ltr_rtl_ltr_middle_run_is_rtl() {
    // In LTR base, the embedded RTL run retains its direction in visual output.
    let bidi = BidiParagraph::new("Hello שלום world", None);
    let visual = bidi.visual_runs();
    let rtl_visual = visual.iter().find(|r| r.direction == TextDirection::Rtl);
    assert!(rtl_visual.is_some(), "RTL run should be present in visual runs");
    assert_eq!(rtl_visual.unwrap().level, 1, "RTL run should have level 1");
}

#[test]
fn bidi_visual_all_ltr_equals_logical() {
    let bidi = BidiParagraph::new("abc def ghi", None);
    let logical = bidi.runs();
    let visual = bidi.visual_runs();
    assert_eq!(logical.len(), visual.len());
    for (l, v) in logical.iter().zip(visual.iter()) {
        assert_eq!(l.start, v.start);
        assert_eq!(l.end, v.end);
    }
}

#[test]
fn bidi_rtl_text_forced_ltr_base_hebrew_level_one() {
    // Hebrew text with forced LTR base: characters still resolve to level 1.
    let bidi = BidiParagraph::new("שלום עולם", Some(TextDirection::Ltr));
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    for i in 0..bidi.levels().len() {
        assert_eq!(
            bidi.level_at(i),
            1,
            "Char {} should be level 1 in forced-LTR Hebrew paragraph, got {}",
            i,
            bidi.level_at(i)
        );
    }
}

// ── Single character tests ──────────────────────────────────────────────

#[test]
fn bidi_single_latin_char_run_direction_ltr() {
    let bidi = BidiParagraph::new("x", None);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].direction, TextDirection::Ltr);
    assert_eq!(runs[0].start, 0);
    assert_eq!(runs[0].end, 1);
}

#[test]
fn bidi_single_hebrew_char_run_byte_range() {
    let bidi = BidiParagraph::new("ב", None);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].direction, TextDirection::Rtl);
    assert_eq!(runs[0].start, 0);
    // Hebrew ב is 2 bytes in UTF-8
    assert_eq!(runs[0].end, "ב".len());
}

#[test]
fn bidi_single_digit_level_zero_base_ltr() {
    let bidi = BidiParagraph::new("7", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    assert_eq!(bidi.level_at(0), 0);
    assert_eq!(bidi.runs().len(), 1);
}

// ── Level access ────────────────────────────────────────────────────────

#[test]
fn bidi_level_at_char_hebrew_and_latin_positions() {
    let text = "ab שלום cd";
    let bidi = BidiParagraph::new(text, None);
    // chars: a(0)b(1) (2)ש(3)ל(4)ו(5)ם(6) (7)c(8)d(9)
    assert_eq!(bidi.level_at(0), 0, "Latin 'a' should be level 0");
    assert_eq!(bidi.level_at(1), 0, "Latin 'b' should be level 0");
    assert_eq!(bidi.level_at(3) % 2, 1, "Hebrew ש should be odd level");
    assert_eq!(bidi.level_at(6) % 2, 1, "Hebrew ם should be odd level");
    assert_eq!(bidi.level_at(8), 0, "Latin 'c' should be level 0");
    assert_eq!(bidi.level_at(9), 0, "Latin 'd' should be level 0");
}

#[test]
fn bidi_level_at_byte_hebrew_char_boundaries() {
    let text = "שלום";
    let bidi = BidiParagraph::new(text, None);
    // Hebrew chars are 2 bytes each: ש(0-1) ל(2-3) ו(4-5) ם(6-7)
    assert_eq!(bidi.level_at_byte(0), 1, "Byte 0 (start of ש)");
    assert_eq!(bidi.level_at_byte(2), 1, "Byte 2 (start of ל)");
    assert_eq!(bidi.level_at_byte(4), 1, "Byte 4 (start of ו)");
    assert_eq!(bidi.level_at_byte(6), 1, "Byte 6 (start of ם)");
}

#[test]
fn bidi_level_at_byte_out_of_bounds() {
    let bidi = BidiParagraph::new("Hi", None);
    assert_eq!(bidi.level_at_byte(500), 0);
}

// ── Text accessor invariants ────────────────────────────────────────────

#[test]
fn bidi_text_accessor_preserves_rtl_input() {
    let text = "שלום עולם";
    let bidi = BidiParagraph::new(text, None);
    assert_eq!(bidi.text(), text);
}

#[test]
fn bidi_all_runs_byte_total_equals_text_len() {
    let text = "The שלום quick brown fox";
    let bidi = BidiParagraph::new(text, None);
    let runs = bidi.runs();
    let total: usize = runs.iter().map(|r| r.end - r.start).sum();
    assert_eq!(total, text.len());
}

#[test]
fn bidi_runs_are_contiguous_no_gaps() {
    let text = "Hello שלום world עולם end";
    let bidi = BidiParagraph::new(text, None);
    let runs = bidi.runs();
    assert!(runs.len() >= 2, "Need multiple runs for contiguity test");
    for i in 1..runs.len() {
        assert_eq!(
            runs[i].start,
            runs[i - 1].end,
            "Run {} start ({}) should equal run {} end ({})",
            i,
            runs[i].start,
            i - 1,
            runs[i - 1].end
        );
    }
}

// ── Additional edge cases ───────────────────────────────────────────────

#[test]
fn bidi_mixed_with_parentheses_neutral_chars() {
    let text = "Hello (שלום) world";
    let bidi = BidiParagraph::new(text, None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    let runs = bidi.runs();
    assert!(runs.iter().any(|r| r.direction == TextDirection::Rtl));
    let total: usize = runs.iter().map(|r| r.end - r.start).sum();
    assert_eq!(total, text.len());
}

#[test]
fn bidi_multiple_number_groups_in_rtl() {
    let text = "שלום 12 עולם 34 סוף";
    let bidi = BidiParagraph::new(text, None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
    let runs = bidi.runs();
    let ltr_runs: Vec<_> = runs.iter().filter(|r| r.direction == TextDirection::Ltr).collect();
    assert!(
        ltr_runs.len() >= 2,
        "Should have at least 2 LTR runs for separate number groups, got {}",
        ltr_runs.len()
    );
}

#[test]
fn bidi_exclamation_after_hebrew_absorbs_rtl() {
    // Trailing punctuation after RTL text: N1/N2 resolves it to R direction.
    let bidi = BidiParagraph::new("שלום!", None);
    // chars: ש(0)ל(1)ו(2)ם(3)!(4)
    let excl_level = bidi.level_at(4);
    assert_eq!(
        excl_level % 2,
        1,
        "Exclamation after Hebrew should get odd (RTL) level, got {}",
        excl_level
    );
}
