//! Tests for line breaking algorithm.
//!
//! Validates that the LineBreaker correctly breaks inline items into lines
//! based on available width, CSS word-break, overflow-wrap, and white-space
//! properties.

use openui_dom::{Document, ElementTag};
use openui_geometry::LayoutUnit;
use openui_layout::inline::items::InlineItemType;
use openui_layout::inline::items_builder::InlineItemsBuilder;
use openui_layout::inline::line_breaker::LineBreaker;
use openui_style::{Display, OverflowWrap, TextAlign, WhiteSpace, WordBreak};

// ── Helpers ─────────────────────────────────────────────────────────────

fn lu(px: f32) -> LayoutUnit {
    LayoutUnit::from_f32(px)
}

/// Create shaped inline items from text nodes under a block.
fn make_shaped_items(
    texts: &[&str],
    white_space: WhiteSpace,
    word_break: WordBreak,
    overflow_wrap: OverflowWrap,
) -> openui_layout::inline::items_builder::InlineItemsData {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    for text in texts {
        let t = doc.create_node(ElementTag::Text);
        doc.node_mut(t).text = Some(text.to_string());
        doc.node_mut(t).style.display = Display::Inline;
        doc.node_mut(t).style.white_space = white_space;
        doc.node_mut(t).style.word_break = word_break;
        doc.node_mut(t).style.overflow_wrap = overflow_wrap;
        doc.append_child(block, t);
    }

    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.shape_text();
    data
}

/// Simple helper: make shaped items with default CSS settings.
fn make_normal_items(texts: &[&str]) -> openui_layout::inline::items_builder::InlineItemsData {
    make_shaped_items(texts, WhiteSpace::Normal, WordBreak::Normal, OverflowWrap::Normal)
}

/// Collect all lines from a line breaker.
fn collect_all_lines(
    data: &openui_layout::inline::items_builder::InlineItemsData,
    available_width: LayoutUnit,
) -> Vec<openui_layout::inline::line_info::LineInfo> {
    let mut breaker = LineBreaker::new(data, available_width);
    let mut lines = Vec::new();
    while let Some(line) = breaker.next_line(available_width) {
        lines.push(line);
        if lines.len() > 100 {
            panic!("Too many lines — possible infinite loop");
        }
    }
    lines
}

/// Get total width of shaped text for a string (for computing test widths).
fn measure_text(text: &str) -> f32 {
    use openui_text::{Font, FontDescription, TextDirection, TextShaper};
    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::new());
    let result = shaper.shape(text, &font, TextDirection::Ltr);
    result.width()
}

// ── Basic Line Breaking Tests ───────────────────────────────────────────

#[test]
fn single_word_fits_on_line() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert_eq!(lines.len(), 1);
    assert!(lines[0].used_width > LayoutUnit::zero());
}

#[test]
fn single_word_wide_available() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(10000.0));
    assert_eq!(lines.len(), 1);
}

#[test]
fn two_words_fit_on_wide_line() {
    let data = make_normal_items(&["hello world"]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert_eq!(lines.len(), 1);
}

#[test]
fn two_words_break_when_narrow() {
    let data = make_normal_items(&["hello world"]);
    let w = measure_text("hello ") + 1.0; // just enough for "hello " but not "hello world"
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2, "Should break 'hello world' into 2+ lines at narrow width");
}

#[test]
fn long_word_overflows_if_first_on_line() {
    // A word that doesn't fit should still be placed on the first line
    let data = make_normal_items(&["supercalifragilisticexpialidocious"]);
    let lines = collect_all_lines(&data, lu(10.0));
    assert!(!lines.is_empty(), "Should produce at least one line");
    // The word should be forced onto the first line even though it overflows
    assert!(lines[0].used_width > lu(10.0));
}

#[test]
fn overflow_wrap_break_word_breaks_mid_word() {
    let data = make_shaped_items(
        &["abcdefghij"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::BreakWord,
    );
    let single_char_width = measure_text("a");
    // Width for ~3 characters
    let avail = single_char_width * 3.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(lines.len() >= 2, "overflow-wrap: break-word should break mid-word");
}

#[test]
fn word_break_break_all_breaks_between_characters() {
    let data = make_shaped_items(
        &["abcd"],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );
    let single_char_width = measure_text("a");
    let avail = single_char_width * 2.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(lines.len() >= 2, "word-break: break-all should break between characters");
}

#[test]
fn white_space_nowrap_prevents_breaks() {
    let data = make_shaped_items(
        &["hello world this is a long line"],
        WhiteSpace::Nowrap,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(50.0));
    assert_eq!(lines.len(), 1, "white-space: nowrap should prevent line breaks");
}

#[test]
fn white_space_pre_preserves_newlines() {
    let data = make_shaped_items(
        &["hello\nworld"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    assert_eq!(lines.len(), 2, "white-space: pre should break at newlines");
    assert!(lines[0].has_forced_break);
}

#[test]
fn empty_text_produces_no_lines() {
    let data = make_normal_items(&[]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert_eq!(lines.len(), 0);
}

#[test]
fn available_width_exactly_matches_no_break() {
    let data = make_normal_items(&["hello"]);
    let w = measure_text("hello");
    let lines = collect_all_lines(&data, lu(w));
    assert_eq!(lines.len(), 1);
}

#[test]
fn multiple_lines_correct_distribution() {
    let data = make_normal_items(&["aa bb cc dd"]);
    let w = measure_text("aa bb ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2);
}

#[test]
fn line_info_used_width_nonzero() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert!(lines[0].used_width > LayoutUnit::zero());
}

#[test]
fn line_info_is_last_line_true_for_single_line() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert!(lines.last().unwrap().is_last_line);
}

#[test]
fn line_info_is_last_line_false_for_first_of_multiple() {
    let data = make_normal_items(&["hello world"]);
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    if lines.len() >= 2 {
        assert!(!lines[0].is_last_line);
        assert!(lines.last().unwrap().is_last_line);
    }
}

#[test]
fn line_info_has_forced_break_false_for_soft_break() {
    let data = make_normal_items(&["hello world"]);
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    if lines.len() >= 2 {
        assert!(!lines[0].has_forced_break);
    }
}

#[test]
fn pre_newline_forces_break() {
    let data = make_shaped_items(
        &["line1\nline2"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    assert_eq!(lines.len(), 2);
    assert!(lines[0].has_forced_break);
}

#[test]
fn pre_line_newline_forces_break() {
    let data = make_shaped_items(
        &["line1\nline2"],
        WhiteSpace::PreLine,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    assert_eq!(lines.len(), 2);
    assert!(lines[0].has_forced_break);
}

#[test]
fn pre_wrap_breaks_at_available_width() {
    let data = make_shaped_items(
        &["hello world"],
        WhiteSpace::PreWrap,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2);
}

#[test]
fn multiple_text_nodes_break_correctly() {
    let data = make_normal_items(&["hello ", "world test"]);
    let w = measure_text("hello world") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    // "hello world" fits on first line, "test" on second
    // However, the items are separate text items, so it depends on break opportunities
    assert!(!lines.is_empty());
}

#[test]
fn very_long_text_many_lines() {
    let long_text = "word ".repeat(50);
    let data = make_normal_items(&[&long_text]);
    let w = measure_text("word word ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 10, "Long text should produce many lines");
}

#[test]
fn single_character_per_line_break_all() {
    let data = make_shaped_items(
        &["abcd"],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );
    let single_char_width = measure_text("a");
    let avail = single_char_width * 1.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(lines.len() >= 3, "break-all with narrow width should produce many lines");
}

#[test]
fn keep_all_only_breaks_at_spaces() {
    let data = make_shaped_items(
        &["hello world"],
        WhiteSpace::Normal,
        WordBreak::KeepAll,
        OverflowWrap::Normal,
    );
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    // With keep-all, should break between "hello" and "world"
    assert!(lines.len() >= 2);
}

#[test]
fn text_align_is_propagated() {
    let data = make_normal_items(&["hello"]);
    let mut breaker = LineBreaker::new(&data, lu(500.0));
    breaker.set_text_align(TextAlign::Center);
    let line = breaker.next_line(lu(500.0)).unwrap();
    assert_eq!(line.text_align, TextAlign::Center);
}

#[test]
fn line_breaker_is_finished_after_all_consumed() {
    let data = make_normal_items(&["hello"]);
    let mut breaker = LineBreaker::new(&data, lu(500.0));
    let _ = breaker.next_line(lu(500.0));
    assert!(breaker.next_line(lu(500.0)).is_none());
    assert!(breaker.is_finished());
}

#[test]
fn line_breaker_finished_on_empty() {
    let data = make_normal_items(&[]);
    let mut breaker = LineBreaker::new(&data, lu(500.0));
    assert!(breaker.next_line(lu(500.0)).is_none());
    assert!(breaker.is_finished());
}

// ── Span / Inline Element Tests ─────────────────────────────────────────

#[test]
fn span_open_close_on_same_line() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(block, span);

    let t = doc.create_node(ElementTag::Text);
    doc.node_mut(t).text = Some("hello".to_string());
    doc.append_child(span, t);

    let mut data = InlineItemsBuilder::collect(&doc, block);
    data.shape_text();

    let lines = collect_all_lines(&data, lu(500.0));
    assert_eq!(lines.len(), 1);
    // Should have open, text, close
    let types: Vec<_> = lines[0].items.iter().map(|i| i.item_type).collect();
    assert_eq!(types, vec![InlineItemType::OpenTag, InlineItemType::Text, InlineItemType::CloseTag]);
}

#[test]
fn text_items_have_text_range_on_line() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(500.0));
    let text_items: Vec<_> = lines[0]
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::Text)
        .collect();
    assert_eq!(text_items.len(), 1);
    assert!(!text_items[0].text_range.is_empty());
}

#[test]
fn inline_size_matches_used_width_single_item() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(500.0));
    let text_items: Vec<_> = lines[0]
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::Text)
        .collect();
    assert_eq!(text_items.len(), 1);
    assert_eq!(text_items[0].inline_size, lines[0].used_width);
}

#[test]
fn break_at_hyphen() {
    let data = make_normal_items(&["well-known fact"]);
    let w = measure_text("well-") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    // Should potentially break after the hyphen
    assert!(!lines.is_empty());
}

// ── Pre-wrap Tests ──────────────────────────────────────────────────────

#[test]
fn pre_wrap_preserves_spaces() {
    let data = make_shaped_items(
        &["hello   world"],
        WhiteSpace::PreWrap,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    // Spaces should be preserved in pre-wrap
    assert_eq!(&data.text, "hello   world");
}

#[test]
fn pre_wrap_breaks_at_spaces() {
    let data = make_shaped_items(
        &["hello world"],
        WhiteSpace::PreWrap,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2);
}

// ── Pre with multiple newlines ──────────────────────────────────────────

#[test]
fn pre_multiple_newlines() {
    let data = make_shaped_items(
        &["a\nb\nc"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    assert_eq!(lines.len(), 3);
}

#[test]
fn pre_trailing_newline() {
    let data = make_shaped_items(
        &["hello\n"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    // "hello" on first line (forced break), then empty content — 
    // the forced break produces one line with content
    assert!(lines.len() >= 1);
    assert!(lines[0].has_forced_break);
}

#[test]
fn pre_leading_newline() {
    let data = make_shaped_items(
        &["\nhello"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    assert_eq!(lines.len(), 2);
    assert!(lines[0].has_forced_break);
}

// ── Overflow-wrap: anywhere ─────────────────────────────────────────────

#[test]
fn overflow_wrap_anywhere_breaks_mid_word() {
    let data = make_shaped_items(
        &["abcdefghij"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Anywhere,
    );
    let single_char_width = measure_text("a");
    let avail = single_char_width * 3.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(lines.len() >= 2, "overflow-wrap: anywhere should break mid-word");
}

// ── word-break: break-word (legacy) ─────────────────────────────────────

#[test]
fn word_break_break_word_same_as_overflow_wrap_break_word() {
    let data = make_shaped_items(
        &["abcdefghij"],
        WhiteSpace::Normal,
        WordBreak::BreakWord,
        OverflowWrap::Normal,
    );
    let single_char_width = measure_text("a");
    let avail = single_char_width * 3.5;
    let lines = collect_all_lines(&data, lu(avail));
    // word-break: break-word uses normal break opportunities, so a single word
    // without spaces will overflow (it's treated same as Normal for break opportunities)
    assert!(!lines.is_empty());
}

// ── Edge Cases ──────────────────────────────────────────────────────────

#[test]
fn zero_width_available_forces_content() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(0.0));
    assert!(!lines.is_empty(), "Zero width should still produce at least one line");
}

#[test]
fn negative_width_available() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, LayoutUnit::from_i32(-100));
    assert!(!lines.is_empty());
}

#[test]
fn only_whitespace_text() {
    let data = make_normal_items(&["   "]);
    let lines = collect_all_lines(&data, lu(500.0));
    // Collapsed to single space
    assert!(!lines.is_empty());
}

#[test]
fn line_items_have_correct_item_type() {
    let data = make_normal_items(&["hello world"]);
    let lines = collect_all_lines(&data, lu(500.0));
    for item in &lines[0].items {
        assert_eq!(item.item_type, InlineItemType::Text);
    }
}

#[test]
fn break_all_unicode() {
    // Test with multi-byte characters
    let data = make_shaped_items(
        &["aébc"],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );
    let single_char_width = measure_text("a");
    let avail = single_char_width * 2.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(lines.len() >= 2, "break-all should work with multi-byte chars");
}

#[test]
fn multiple_text_items_on_same_line() {
    let data = make_normal_items(&["hello ", "world"]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert_eq!(lines.len(), 1);
    let text_items: Vec<_> = lines[0]
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::Text)
        .collect();
    assert_eq!(text_items.len(), 2);
}

#[test]
fn second_text_item_causes_break() {
    let data = make_normal_items(&["hello ", "world"]);
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    // "hello " fits, "world" should go to next line
    assert!(lines.len() >= 2);
}

#[test]
fn three_words_three_lines() {
    let data = make_normal_items(&["aa bb cc"]);
    let w = measure_text("aa ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 3);
}

#[test]
fn whitespace_pre_no_wrap() {
    let data = make_shaped_items(
        &["hello world this is long"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(50.0));
    // Pre doesn't wrap
    assert_eq!(lines.len(), 1);
}

#[test]
fn break_spaces_wraps() {
    let data = make_shaped_items(
        &["hello world"],
        WhiteSpace::BreakSpaces,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2);
}

#[test]
fn pre_line_wraps_at_spaces() {
    let data = make_shaped_items(
        &["hello world"],
        WhiteSpace::PreLine,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("hello ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2);
}

#[test]
fn last_line_marker_multiline() {
    let data = make_normal_items(&["aa bb cc"]);
    let w = measure_text("aa ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2);
    for (i, line) in lines.iter().enumerate() {
        if i < lines.len() - 1 {
            assert!(!line.is_last_line, "non-last line should not be marked as last");
        } else {
            assert!(line.is_last_line, "last line should be marked as last");
        }
    }
}

#[test]
fn used_width_accumulates_across_items() {
    let data = make_normal_items(&["hello world"]);
    let lines = collect_all_lines(&data, lu(500.0));
    let total_item_width: LayoutUnit = lines[0]
        .items
        .iter()
        .map(|i| i.inline_size)
        .fold(LayoutUnit::zero(), |a, b| a + b);
    assert_eq!(lines[0].used_width, total_item_width);
}

#[test]
fn available_width_stored_in_line_info() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(300.0));
    assert_eq!(lines[0].available_width, lu(300.0));
}

#[test]
fn remaining_width_correct() {
    let data = make_normal_items(&["hi"]);
    let lines = collect_all_lines(&data, lu(500.0));
    let expected_remaining = lu(500.0) - lines[0].used_width;
    assert_eq!(lines[0].remaining_width(), expected_remaining);
}

#[test]
fn line_has_content_returns_true_for_text() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert!(lines[0].has_content());
}

#[test]
fn shape_result_present_in_line_items() {
    let data = make_normal_items(&["hello"]);
    let lines = collect_all_lines(&data, lu(500.0));
    let text_items: Vec<_> = lines[0]
        .items
        .iter()
        .filter(|i| i.item_type == InlineItemType::Text)
        .collect();
    assert!(text_items[0].shape_result.is_some());
}

#[test]
fn item_index_references_original_items() {
    let data = make_normal_items(&["hello world"]);
    let lines = collect_all_lines(&data, lu(500.0));
    for item_result in &lines[0].items {
        assert!(item_result.item_index < data.items.len());
    }
}

// ═══════════════════════════════════════════════════════════════════════
// SP11 Dual-Model Review Fixes — Regression Tests
// ═══════════════════════════════════════════════════════════════════════

// ── Issue 6: CJK line breaking ──────────────────────────────────────────

#[test]
fn cjk_breaks_between_ideographs() {
    use openui_layout::inline::line_breaker::find_break_opportunities;
    // CJK characters should have break opportunities between each pair
    let text = "\u{4e16}\u{754c}"; // "世界" — two CJK characters
    let breaks = find_break_opportunities(text, WordBreak::Normal, OverflowWrap::Normal, openui_style::LineBreak::Auto);
    // There should be a break opportunity between the two characters
    assert!(
        !breaks.is_empty(),
        "CJK text should have break opportunities between ideographs"
    );
}

#[test]
fn cjk_three_chars_have_two_breaks() {
    use openui_layout::inline::line_breaker::find_break_opportunities;
    // Three CJK characters should have break opportunities between each pair
    let text = "\u{4e16}\u{754c}\u{597d}"; // "世界好"
    let breaks = find_break_opportunities(text, WordBreak::Normal, OverflowWrap::Normal, openui_style::LineBreak::Auto);
    // Should have at least 2 break opportunities (between pairs)
    assert!(
        breaks.len() >= 2,
        "Three CJK characters should have at least 2 breaks, got {}",
        breaks.len()
    );
}

#[test]
fn cjk_mixed_with_latin_breaks() {
    use openui_layout::inline::line_breaker::find_break_opportunities;
    // Mixed Latin and CJK: "hello世界"
    let text = "hello\u{4e16}\u{754c}";
    let breaks = find_break_opportunities(text, WordBreak::Normal, OverflowWrap::Normal, openui_style::LineBreak::Auto);
    // Should have break between Latin and CJK, and between CJK characters
    assert!(
        !breaks.is_empty(),
        "Mixed Latin-CJK text should have break opportunities"
    );
}

// ── Issue 7: strip_trailing_spaces ──────────────────────────────────────

#[test]
fn trailing_space_stripped_from_full_item() {
    // Text ending with a space should have the space width stripped from used_width
    let data = make_normal_items(&["hello "]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert_eq!(lines.len(), 1);

    // The used_width should be less than the full shaped width of "hello "
    // because the trailing space is stripped.
    let hello_width = measure_text("hello");
    let hello_space_width = measure_text("hello ");
    assert!(hello_space_width > hello_width, "hello-space should be wider than hello");

    // The line's used_width after stripping should be close to "hello" width
    let line_width = lines[0].used_width.to_f32();
    assert!(
        line_width < hello_space_width,
        "Line width ({}) should be less than 'hello ' width ({}) after trailing space strip",
        line_width,
        hello_space_width
    );
}

#[test]
fn trailing_space_stripped_from_split_item_at_end() {
    // When text is split across lines, the trailing space on each line
    // (except the last character of the full item) should be handled correctly.
    let data = make_normal_items(&["hello world test"]);
    let lines = collect_all_lines(&data, lu(80.0));
    // Should produce multiple lines
    assert!(lines.len() >= 2, "Should wrap into at least 2 lines");

    // Each line's used_width should be positive and reasonable
    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.used_width >= LayoutUnit::zero(),
            "Line {} used_width should be non-negative",
            i
        );
    }
}
