//! BiDi tests — UAX#9 bidirectional text analysis.
//!
//! Tests for the bidi module in openui-text, covering paragraph analysis,
//! run segmentation, visual reordering, and edge cases.

use openui_text::bidi::BidiParagraph;
use openui_text::TextDirection;

// ── Pure LTR tests ──────────────────────────────────────────────────────

#[test]
fn bidi_pure_ltr_all_levels_zero() {
    let bidi = BidiParagraph::new("Hello world", None);
    for level in bidi.levels() {
        assert_eq!(level.number(), 0, "All levels should be 0 for pure LTR");
    }
}

#[test]
fn bidi_pure_ltr_single_run() {
    let bidi = BidiParagraph::new("Hello world", None);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].level, 0);
    assert_eq!(runs[0].direction, TextDirection::Ltr);
}

#[test]
fn bidi_pure_ltr_base_direction() {
    let bidi = BidiParagraph::new("Hello world", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
}

#[test]
fn bidi_pure_ltr_run_boundaries() {
    let bidi = BidiParagraph::new("Hello world", None);
    let runs = bidi.runs();
    assert_eq!(runs[0].start, 0);
    assert_eq!(runs[0].end, "Hello world".len());
}

// ── Pure RTL tests ──────────────────────────────────────────────────────

#[test]
fn bidi_pure_rtl_hebrew_levels_one() {
    let bidi = BidiParagraph::new("שלום עולם", None);
    for level in bidi.levels() {
        assert_eq!(level.number(), 1, "All levels should be 1 for pure RTL Hebrew");
    }
}

#[test]
fn bidi_pure_rtl_hebrew_single_run() {
    let bidi = BidiParagraph::new("שלום עולם", None);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].level, 1);
    assert_eq!(runs[0].direction, TextDirection::Rtl);
}

#[test]
fn bidi_pure_rtl_arabic() {
    let bidi = BidiParagraph::new("مرحبا بالعالم", None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].direction, TextDirection::Rtl);
}

#[test]
fn bidi_pure_rtl_base_direction() {
    let bidi = BidiParagraph::new("שלום", None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
}

// ── Mixed LTR+RTL tests ────────────────────────────────────────────────

#[test]
fn bidi_mixed_ltr_rtl_multiple_runs() {
    let bidi = BidiParagraph::new("Hello שלום world", None);
    let runs = bidi.runs();
    // Should have at least 2 runs (LTR and RTL parts)
    assert!(runs.len() >= 2, "Mixed text should produce multiple runs, got {}", runs.len());
}

#[test]
fn bidi_mixed_first_run_ltr() {
    let bidi = BidiParagraph::new("Hello שלום world", None);
    let runs = bidi.runs();
    assert_eq!(runs[0].direction, TextDirection::Ltr);
}

#[test]
fn bidi_mixed_has_rtl_run() {
    let bidi = BidiParagraph::new("Hello שלום world", None);
    let runs = bidi.runs();
    let has_rtl = runs.iter().any(|r| r.direction == TextDirection::Rtl);
    assert!(has_rtl, "Should have at least one RTL run");
}

#[test]
fn bidi_mixed_base_direction_from_first_strong() {
    // First strong char is Latin → LTR base
    let bidi = BidiParagraph::new("Hello שלום", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
}

#[test]
fn bidi_mixed_rtl_base_from_first_strong() {
    // First strong char is Hebrew → RTL base
    let bidi = BidiParagraph::new("שלום Hello", None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
}

// ── Numbers in RTL context ──────────────────────────────────────────────

#[test]
fn bidi_numbers_in_rtl_remain_ltr() {
    // Numbers in RTL context should have an even (LTR) level.
    // Per UAX#9, European numbers in RTL context get level 2.
    let bidi = BidiParagraph::new("שלום 123 עולם", None);
    let runs = bidi.runs();
    let number_run = runs.iter().find(|r| {
        let text = &bidi.text()[r.start..r.end];
        text.contains("123")
    });
    assert!(number_run.is_some(), "Should find a run containing numbers");
    let nr = number_run.unwrap();
    // Even level means LTR display order for the numbers
    assert_eq!(nr.level % 2, 0, "Numbers should have even level (LTR direction), got level {}", nr.level);
}

#[test]
fn bidi_numbers_with_rtl_direction() {
    let bidi = BidiParagraph::new("שלום 42", None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
    let runs = bidi.runs();
    // There should be a run with an even level for the numbers
    let has_even_level = runs.iter().any(|r| r.level % 2 == 0);
    assert!(has_even_level, "Numbers should have even (LTR) level");
}

// ── Visual reordering tests ─────────────────────────────────────────────

#[test]
fn bidi_visual_reorder_pure_ltr_unchanged() {
    let bidi = BidiParagraph::new("Hello world", None);
    let visual = bidi.visual_runs();
    assert_eq!(visual.len(), 1);
    assert_eq!(visual[0].start, 0);
}

#[test]
fn bidi_visual_reorder_pure_rtl_unchanged() {
    let bidi = BidiParagraph::new("שלום עולם", None);
    let visual = bidi.visual_runs();
    assert_eq!(visual.len(), 1);
}

#[test]
fn bidi_visual_reorder_mixed() {
    let bidi = BidiParagraph::new("Hello שלום world", None);
    let visual = bidi.visual_runs();
    // Visual runs should have content
    assert!(!visual.is_empty());
    // Total byte coverage should equal the text length
    let total_bytes: usize = visual.iter().map(|r| r.end - r.start).sum();
    assert_eq!(total_bytes, "Hello שלום world".len());
}

// ── Base direction override tests ───────────────────────────────────────

#[test]
fn bidi_forced_ltr_on_rtl_text() {
    let bidi = BidiParagraph::new("שלום", Some(TextDirection::Ltr));
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
}

#[test]
fn bidi_forced_rtl_on_ltr_text() {
    let bidi = BidiParagraph::new("Hello", Some(TextDirection::Rtl));
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
}

#[test]
fn bidi_auto_detect_from_latin() {
    let bidi = BidiParagraph::new("Hello", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
}

#[test]
fn bidi_auto_detect_from_hebrew() {
    let bidi = BidiParagraph::new("שלום", None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
}

// ── Edge cases ──────────────────────────────────────────────────────────

#[test]
fn bidi_empty_text() {
    let bidi = BidiParagraph::new("", None);
    assert_eq!(bidi.runs().len(), 0);
    assert_eq!(bidi.visual_runs().len(), 0);
    assert_eq!(bidi.levels().len(), 0);
}

#[test]
fn bidi_single_ltr_char() {
    let bidi = BidiParagraph::new("A", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].level, 0);
}

#[test]
fn bidi_single_rtl_char() {
    let bidi = BidiParagraph::new("א", None);
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].level, 1);
}

#[test]
fn bidi_only_spaces() {
    let bidi = BidiParagraph::new("   ", None);
    // Spaces are neutral — take base direction
    let runs = bidi.runs();
    assert!(!runs.is_empty());
}

#[test]
fn bidi_only_numbers() {
    let bidi = BidiParagraph::new("12345", None);
    // Numbers alone → LTR
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
}

#[test]
fn bidi_level_at_byte_ltr() {
    let bidi = BidiParagraph::new("Hello", None);
    assert_eq!(bidi.level_at_byte(0), 0);
    assert_eq!(bidi.level_at_byte(3), 0);
}

#[test]
fn bidi_level_at_char_ltr() {
    let bidi = BidiParagraph::new("Hello", None);
    assert_eq!(bidi.level_at(0), 0);
    assert_eq!(bidi.level_at(4), 0);
}

#[test]
fn bidi_level_at_char_out_of_bounds() {
    let bidi = BidiParagraph::new("Hi", None);
    assert_eq!(bidi.level_at(100), 0);
}

#[test]
fn bidi_run_byte_offsets_cover_full_text() {
    let text = "Hello שלום world";
    let bidi = BidiParagraph::new(text, None);
    let runs = bidi.runs();
    if !runs.is_empty() {
        assert_eq!(runs[0].start, 0);
        assert_eq!(runs.last().unwrap().end, text.len());
    }
}

#[test]
fn bidi_visual_runs_same_byte_coverage() {
    let text = "Hello שלום world";
    let bidi = BidiParagraph::new(text, None);
    let logical = bidi.runs();
    let visual = bidi.visual_runs();
    let logical_bytes: usize = logical.iter().map(|r| r.end - r.start).sum();
    let visual_bytes: usize = visual.iter().map(|r| r.end - r.start).sum();
    assert_eq!(logical_bytes, visual_bytes);
}

#[test]
fn bidi_multibyte_utf8_ltr() {
    // café has multi-byte é
    let bidi = BidiParagraph::new("café", None);
    let runs = bidi.runs();
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0].end, "café".len());
}

#[test]
fn bidi_mixed_scripts_cjk() {
    // CJK characters are LTR
    let bidi = BidiParagraph::new("Hello 你好 world", None);
    assert_eq!(bidi.base_direction(), TextDirection::Ltr);
    let runs = bidi.runs();
    // All should be LTR
    for run in &runs {
        assert_eq!(run.level % 2, 0, "CJK should be LTR");
    }
}

#[test]
fn bidi_paragraph_text_accessor() {
    let text = "Hello שלום";
    let bidi = BidiParagraph::new(text, None);
    assert_eq!(bidi.text(), text);
}

#[test]
fn bidi_forced_rtl_numbers_level() {
    let bidi = BidiParagraph::new("123", Some(TextDirection::Rtl));
    assert_eq!(bidi.base_direction(), TextDirection::Rtl);
    // Numbers are weakly LTR, but in RTL paragraph they get level 2
    let runs = bidi.runs();
    assert!(!runs.is_empty());
    assert!(runs[0].level % 2 == 0, "Numbers should be even (LTR) level");
}
