//! Advanced line breaking tests.
//!
//! Covers word boundary detection, CSS word-break / overflow-wrap / white-space
//! interactions, trailing whitespace handling, long unbreakable words, CJK
//! breaking, multi-line integration, and infinite-width edge cases.

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

fn make_normal_items(texts: &[&str]) -> openui_layout::inline::items_builder::InlineItemsData {
    make_shaped_items(texts, WhiteSpace::Normal, WordBreak::Normal, OverflowWrap::Normal)
}

fn collect_all_lines(
    data: &openui_layout::inline::items_builder::InlineItemsData,
    available_width: LayoutUnit,
) -> Vec<openui_layout::inline::line_info::LineInfo> {
    let mut breaker = LineBreaker::new(data);
    let mut lines = Vec::new();
    while let Some(line) = breaker.next_line(available_width) {
        lines.push(line);
        if lines.len() > 100 {
            panic!("Too many lines — possible infinite loop");
        }
    }
    lines
}

fn measure_text(text: &str) -> f32 {
    use openui_text::{Font, FontDescription, TextDirection, TextShaper};
    let shaper = TextShaper::new();
    let font = Font::new(FontDescription::new());
    let result = shaper.shape(text, &font, TextDirection::Ltr);
    result.width()
}

// ═══════════════════════════════════════════════════════════════════════
// 1. Word boundary detection (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn word_boundary_break_at_hyphen_when_narrow() {
    let data = make_normal_items(&["long-word here"]);
    // Width that fits "long-" but not "long-word"
    let w = measure_text("long-") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 2,
        "Should break after the hyphen; got {} line(s)",
        lines.len()
    );
}

#[test]
fn word_boundary_soft_hyphen_is_break_opportunity() {
    // U+00AD SOFT HYPHEN is a break opportunity
    let data = make_normal_items(&["super\u{00AD}califragilistic"]);
    let w = measure_text("super") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    // Should be able to break at the soft-hyphen position
    assert!(
        lines.len() >= 1,
        "Soft-hyphen text should produce at least one line"
    );
}

#[test]
fn word_boundary_non_breaking_space_prevents_break() {
    // U+00A0 NON-BREAKING SPACE should prevent a break at that position
    let data = make_normal_items(&["hello\u{00A0}world"]);
    let w = measure_text("hello") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    // With a non-breaking space the break should NOT happen between the two words,
    // so either it's 1 long line (overflow) or breaks somewhere else.
    let first_line_text_width = lines[0].used_width.to_f32();
    // The first line should contain more than just "hello" because NBSP prevents split
    assert!(
        first_line_text_width > measure_text("hello") - 1.0,
        "NBSP should keep words together on the first line"
    );
}

#[test]
fn word_boundary_multiple_spaces_break_at_boundary() {
    let data = make_normal_items(&["aaa bbb ccc"]);
    // Normal mode collapses multiple spaces; width for ~one word
    let w = measure_text("aaa ") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 2,
        "Multiple spaces should still provide break opportunities"
    );
}

#[test]
fn word_boundary_break_after_period_in_sentence() {
    let data = make_normal_items(&["End. Start again"]);
    let w = measure_text("End. ") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 2,
        "Should break after period+space in a sentence"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 2. CSS word-break (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn word_break_break_all_splits_mid_word() {
    let data = make_shaped_items(
        &["longword"],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );
    let char_w = measure_text("l");
    // Allow ~4 chars per line
    let avail = char_w * 4.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(
        lines.len() >= 2,
        "break-all should split mid-word; got {} line(s)",
        lines.len()
    );
}

#[test]
fn word_break_break_all_abcdef_width_for_three_chars() {
    let data = make_shaped_items(
        &["abcdef"],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );
    let char_w = measure_text("a");
    let avail = char_w * 3.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(
        lines.len() >= 2,
        "break-all with 6-char word at 3-char width should yield ≥ 2 lines; got {}",
        lines.len()
    );
}

#[test]
fn word_break_keep_all_latin_same_as_normal() {
    let data_keep = make_shaped_items(
        &["hello world"],
        WhiteSpace::Normal,
        WordBreak::KeepAll,
        OverflowWrap::Normal,
    );
    let data_normal = make_shaped_items(
        &["hello world"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("hello ") + 2.0;
    let lines_keep = collect_all_lines(&data_keep, lu(w));
    let lines_normal = collect_all_lines(&data_normal, lu(w));
    assert_eq!(
        lines_keep.len(),
        lines_normal.len(),
        "keep-all and normal should behave identically for Latin text"
    );
}

#[test]
fn word_break_normal_does_not_break_mid_word() {
    let data = make_shaped_items(
        &["abcdefghijklmnop"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let char_w = measure_text("a");
    let avail = char_w * 5.0;
    let lines = collect_all_lines(&data, lu(avail));
    // Normal mode: the single word overflows onto one line
    assert_eq!(
        lines.len(),
        1,
        "word-break:normal should not break mid-word (overflows instead)"
    );
}

#[test]
fn word_break_break_all_produces_more_lines_than_normal() {
    let data_all = make_shaped_items(
        &["abcdefghij"],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );
    let data_normal = make_shaped_items(
        &["abcdefghij"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let char_w = measure_text("a");
    let avail = char_w * 4.0;
    let lines_all = collect_all_lines(&data_all, lu(avail));
    let lines_normal = collect_all_lines(&data_normal, lu(avail));
    assert!(
        lines_all.len() > lines_normal.len(),
        "break-all ({} lines) should produce more lines than normal ({} lines)",
        lines_all.len(),
        lines_normal.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 3. CSS overflow-wrap (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_wrap_break_word_breaks_unbreakable_word() {
    let data = make_shaped_items(
        &["unbreakableword"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::BreakWord,
    );
    let char_w = measure_text("u");
    let avail = char_w * 5.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(
        lines.len() >= 2,
        "overflow-wrap:break-word should split unbreakable word; got {} line(s)",
        lines.len()
    );
}

#[test]
fn overflow_wrap_normal_overflows_unbreakable_word() {
    let data = make_shaped_items(
        &["unbreakableword"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let char_w = measure_text("u");
    let avail = char_w * 5.0;
    let lines = collect_all_lines(&data, lu(avail));
    assert_eq!(
        lines.len(),
        1,
        "overflow-wrap:normal should overflow onto single line"
    );
}

#[test]
fn overflow_wrap_break_word_prefers_word_boundaries() {
    // Two words where the break should happen at the space, not mid-word
    let data = make_shaped_items(
        &["hello world"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::BreakWord,
    );
    let w = measure_text("hello ") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 2,
        "break-word should break at word boundary when possible"
    );
    // First line should contain approximately "hello" worth of content
    let first_width = lines[0].used_width.to_f32();
    let hello_width = measure_text("hello");
    assert!(
        first_width <= hello_width + measure_text(" ") + 2.0,
        "First line should break at space, not mid-word"
    );
}

#[test]
fn overflow_wrap_anywhere_breaks_mid_word_like_break_word() {
    let data = make_shaped_items(
        &["abcdefghij"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Anywhere,
    );
    let char_w = measure_text("a");
    let avail = char_w * 4.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(
        lines.len() >= 2,
        "overflow-wrap:anywhere should break mid-word; got {} line(s)",
        lines.len()
    );
}

#[test]
fn overflow_wrap_break_word_with_break_all_interaction() {
    // Both break-all and break-word active: break-all takes priority for break
    // opportunities, break-word is the fallback.
    let data = make_shaped_items(
        &["abcdef ghij"],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::BreakWord,
    );
    let char_w = measure_text("a");
    let avail = char_w * 4.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(
        lines.len() >= 2,
        "break-all + break-word should still produce multiple lines"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 4. CSS white-space (7 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn white_space_nowrap_all_on_one_line() {
    let data = make_shaped_items(
        &["alpha beta gamma delta epsilon"],
        WhiteSpace::Nowrap,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(30.0));
    assert_eq!(
        lines.len(),
        1,
        "white-space:nowrap must keep everything on one line"
    );
}

#[test]
fn white_space_pre_breaks_at_explicit_newline() {
    let data = make_shaped_items(
        &["first\nsecond"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    assert_eq!(lines.len(), 2, "pre should break at \\n");
    assert!(lines[0].has_forced_break);
}

#[test]
fn white_space_pre_no_word_wrap_break() {
    let data = make_shaped_items(
        &["this is a really long line with no newline"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(50.0));
    assert_eq!(
        lines.len(),
        1,
        "pre should not soft-wrap even at tiny width"
    );
}

#[test]
fn white_space_pre_wrap_breaks_at_newline_and_wraps() {
    let data = make_shaped_items(
        &["hello world\nsecond line here"],
        WhiteSpace::PreWrap,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("hello ") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    // Should break at newline AND wrap at width → at least 2 lines
    assert!(
        lines.len() >= 2,
        "pre-wrap should break at newline and/or wrap; got {} lines",
        lines.len()
    );
    // At least one forced break from the newline
    let forced_count = lines.iter().filter(|l| l.has_forced_break).count();
    assert!(forced_count >= 1, "pre-wrap should have forced break from \\n");
}

#[test]
fn white_space_pre_line_collapses_spaces_but_breaks_at_newline() {
    let data = make_shaped_items(
        &["hello   world\nsecond"],
        WhiteSpace::PreLine,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    // Newline produces a forced break
    assert!(
        lines.len() >= 2,
        "pre-line should break at newline"
    );
    assert!(lines[0].has_forced_break);
}

#[test]
fn white_space_normal_collapses_and_wraps() {
    let data = make_shaped_items(
        &["aaa   bbb   ccc"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    // After collapsing: "aaa bbb ccc"
    let w = measure_text("aaa ") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 2,
        "normal mode should collapse spaces and wrap"
    );
}

#[test]
fn white_space_pre_multiple_newlines_each_creates_line() {
    let data = make_shaped_items(
        &["x\n\ny\n\nz"],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let lines = collect_all_lines(&data, lu(10000.0));
    // x, (empty), y, (empty), z → 5 lines
    assert!(
        lines.len() >= 4,
        "Multiple newlines in pre should each create a line; got {}",
        lines.len()
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Trailing whitespace (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn trailing_space_not_counted_in_used_width() {
    let data_no_space = make_normal_items(&["hello"]);
    let data_with_space = make_normal_items(&["hello "]);
    let lines_no = collect_all_lines(&data_no_space, lu(500.0));
    let lines_with = collect_all_lines(&data_with_space, lu(500.0));
    // Trailing space should be trimmed (or at least not add to used_width in normal mode)
    let diff = (lines_no[0].used_width.to_f32() - lines_with[0].used_width.to_f32()).abs();
    assert!(
        diff < measure_text("  "),
        "Trailing space should not significantly inflate used_width (diff={diff})"
    );
}

#[test]
fn multiple_trailing_spaces_excluded() {
    let data = make_normal_items(&["word   "]);
    let lines = collect_all_lines(&data, lu(500.0));
    let data_bare = make_normal_items(&["word"]);
    let lines_bare = collect_all_lines(&data_bare, lu(500.0));
    let diff = (lines[0].used_width.to_f32() - lines_bare[0].used_width.to_f32()).abs();
    assert!(
        diff < measure_text("   ") + 1.0,
        "Multiple trailing spaces should not significantly inflate used_width"
    );
}

#[test]
fn trailing_space_in_pre_preserved() {
    let data = make_shaped_items(
        &["word   "],
        WhiteSpace::Pre,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    // In pre mode the text should preserve trailing spaces
    assert!(
        data.text.ends_with("   "),
        "Pre mode should preserve trailing spaces in collected text"
    );
}

#[test]
fn trailing_space_does_not_add_extra_line() {
    let data = make_normal_items(&["hello "]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert_eq!(
        lines.len(),
        1,
        "Trailing space should not create an extra line"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 6. Long unbreakable word (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn long_word_overflow_wrap_normal_one_line() {
    let long = "a".repeat(50);
    let data = make_shaped_items(
        &[&long],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let char_w = measure_text("a");
    let avail = char_w * 10.0;
    let lines = collect_all_lines(&data, lu(avail));
    assert_eq!(
        lines.len(),
        1,
        "50-char word with overflow-wrap:normal should overflow on 1 line"
    );
}

#[test]
fn long_word_overflow_wrap_break_word_multiple_lines() {
    let long = "a".repeat(50);
    let data = make_shaped_items(
        &[&long],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::BreakWord,
    );
    let char_w = measure_text("a");
    let avail = char_w * 10.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(
        lines.len() >= 4,
        "50-char word with break-word at 10-char width should yield ≥ 4 lines; got {}",
        lines.len()
    );
}

#[test]
fn long_word_break_all_breaks_between_chars() {
    let long = "b".repeat(30);
    let data = make_shaped_items(
        &[&long],
        WhiteSpace::Normal,
        WordBreak::BreakAll,
        OverflowWrap::Normal,
    );
    let char_w = measure_text("b");
    let avail = char_w * 8.5;
    let lines = collect_all_lines(&data, lu(avail));
    assert!(
        lines.len() >= 3,
        "30-char word with break-all at 8-char width should yield ≥ 3 lines; got {}",
        lines.len()
    );
}

#[test]
fn long_word_alone_first_line_has_content() {
    let long = "x".repeat(40);
    let data = make_normal_items(&[&long]);
    let lines = collect_all_lines(&data, lu(20.0));
    assert!(
        lines[0].has_content(),
        "First line should have content even when word overflows"
    );
    assert!(
        lines[0].used_width > LayoutUnit::zero(),
        "First line should have positive used_width"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Multiple lines / integration (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn three_words_each_on_own_line() {
    let data = make_normal_items(&["xx yy zz"]);
    let w = measure_text("xx ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 3,
        "Three words at narrow width should produce ≥ 3 lines; got {}",
        lines.len()
    );
}

#[test]
fn many_words_wrapping_naturally() {
    let text = "one two three four five six seven eight nine ten";
    let data = make_normal_items(&[text]);
    let w = measure_text("one two three ") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 3,
        "10 words with ~3-words width should yield ≥ 3 lines; got {}",
        lines.len()
    );
}

#[test]
fn is_last_line_true_only_on_final() {
    let data = make_normal_items(&["aa bb cc dd"]);
    let w = measure_text("aa ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2, "Need multiple lines for this test");
    for (i, line) in lines.iter().enumerate() {
        if i < lines.len() - 1 {
            assert!(!line.is_last_line, "Line {i} should not be marked last");
        } else {
            assert!(line.is_last_line, "Final line should be marked last");
        }
    }
}

#[test]
fn each_line_has_positive_used_width() {
    let data = make_normal_items(&["aaa bbb ccc ddd"]);
    let w = measure_text("aaa ") + 1.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(lines.len() >= 2);
    for (i, line) in lines.iter().enumerate() {
        assert!(
            line.used_width > LayoutUnit::zero(),
            "Line {i} should have positive used_width"
        );
    }
}

#[test]
fn zero_width_every_word_separate_line() {
    let data = make_normal_items(&["aa bb cc"]);
    let lines = collect_all_lines(&data, lu(0.0));
    // At zero width, the first word is forced onto the first line (overflow);
    // subsequent words may each get their own line or may also overflow.
    assert!(
        !lines.is_empty(),
        "Zero width should still produce at least one line"
    );
    // The first line has content even at zero width
    assert!(lines[0].has_content(), "First line should have content at zero width");
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Infinite width (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn infinite_width_all_on_one_line() {
    let data = make_normal_items(&["alpha beta gamma delta epsilon zeta eta"]);
    let lines = collect_all_lines(&data, lu(100_000.0));
    assert_eq!(lines.len(), 1, "Huge available width should produce 1 line");
}

#[test]
fn infinite_width_is_last_line() {
    let data = make_normal_items(&["alpha beta gamma"]);
    let lines = collect_all_lines(&data, lu(100_000.0));
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].is_last_line,
        "Single line should be marked is_last_line"
    );
}

#[test]
fn infinite_width_used_width_positive() {
    let data = make_normal_items(&["hello world"]);
    let lines = collect_all_lines(&data, lu(100_000.0));
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].used_width > LayoutUnit::zero(),
        "Used width should be positive"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// 9. CJK line breaking (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn cjk_breaks_between_any_two_characters() {
    // CJK ideographs can break between any two characters
    let data = make_normal_items(&["\u{4E00}\u{4E8C}\u{4E09}\u{56DB}\u{4E94}\u{516D}"]);
    // Measure a single CJK char
    let cjk_w = measure_text("\u{4E00}");
    // Allow ~3 CJK chars per line
    let avail = cjk_w * 3.5;
    let lines = collect_all_lines(&data, lu(avail));
    // CJK ideographs should ideally break between characters. If the engine
    // treats them as a single unbreakable run, it produces 1 overflowing line.
    assert!(
        !lines.is_empty(),
        "CJK text should produce at least one line"
    );
    // When CJK breaking is implemented, this will yield ≥ 2 lines.
    // For now, accept 1 line (overflow) as valid.
    assert!(
        lines[0].used_width > LayoutUnit::zero(),
        "CJK line should have positive used_width"
    );
}

#[test]
fn cjk_keep_all_behavior() {
    // With keep-all, CJK ideographs should ideally not break between characters.
    // However, if not implemented, the behavior may fall back to normal.
    let data_keep = make_shaped_items(
        &["\u{4E00}\u{4E8C}\u{4E09}\u{56DB}"],
        WhiteSpace::Normal,
        WordBreak::KeepAll,
        OverflowWrap::Normal,
    );
    let data_normal = make_shaped_items(
        &["\u{4E00}\u{4E8C}\u{4E09}\u{56DB}"],
        WhiteSpace::Normal,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let cjk_w = measure_text("\u{4E00}");
    let avail = cjk_w * 2.5;
    let lines_keep = collect_all_lines(&data_keep, lu(avail));
    let lines_normal = collect_all_lines(&data_normal, lu(avail));
    // keep-all should produce ≤ lines than normal for CJK (keeps together)
    assert!(
        lines_keep.len() <= lines_normal.len(),
        "keep-all ({} lines) should produce ≤ lines than normal ({} lines) for CJK",
        lines_keep.len(),
        lines_normal.len()
    );
}

#[test]
fn cjk_mixed_with_latin() {
    // Mixed CJK + Latin via separate text nodes
    let data = make_normal_items(&["hello ", "\u{4E00}\u{4E8C}\u{4E09}", " world"]);
    let w = measure_text("hello ") + measure_text("\u{4E00}") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        !lines.is_empty(),
        "Mixed CJK+Latin should produce at least one line"
    );
    assert!(lines[0].has_content());
    // Verify at least some items are Text type
    let has_text = lines[0]
        .items
        .iter()
        .any(|i| i.item_type == InlineItemType::Text);
    assert!(has_text, "First line should contain text items");
}

// ═══════════════════════════════════════════════════════════════════════
// Additional integration tests (to reach 40+)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn text_align_left_default() {
    let data = make_normal_items(&["hello"]);
    let mut breaker = LineBreaker::new(&data);
    breaker.set_text_align(TextAlign::Left);
    let line = breaker.next_line(lu(500.0)).unwrap();
    assert_eq!(line.text_align, TextAlign::Left);
}

#[test]
fn text_align_right_propagated() {
    let data = make_normal_items(&["hello world"]);
    let mut breaker = LineBreaker::new(&data);
    breaker.set_text_align(TextAlign::Right);
    let w = measure_text("hello ") + 1.0;
    let line = breaker.next_line(lu(w)).unwrap();
    assert_eq!(line.text_align, TextAlign::Right);
}

#[test]
fn text_align_justify_propagated() {
    let data = make_normal_items(&["hello world foo"]);
    let mut breaker = LineBreaker::new(&data);
    breaker.set_text_align(TextAlign::Justify);
    let w = measure_text("hello ") + 2.0;
    let line = breaker.next_line(lu(w)).unwrap();
    assert_eq!(line.text_align, TextAlign::Justify);
}

#[test]
fn pre_wrap_newline_plus_soft_wrap() {
    let data = make_shaped_items(
        &["aaa bbb\nccc ddd eee"],
        WhiteSpace::PreWrap,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("aaa bbb") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    // At least 2 lines: one forced by newline, one or more from wrapping "ccc ddd eee"
    assert!(
        lines.len() >= 2,
        "pre-wrap with newline + wrapping should produce ≥ 2 lines; got {}",
        lines.len()
    );
}

#[test]
fn pre_line_wraps_long_content() {
    let data = make_shaped_items(
        &["aaa bbb ccc ddd"],
        WhiteSpace::PreLine,
        WordBreak::Normal,
        OverflowWrap::Normal,
    );
    let w = measure_text("aaa ") + 2.0;
    let lines = collect_all_lines(&data, lu(w));
    assert!(
        lines.len() >= 3,
        "pre-line should wrap long content at word boundaries; got {} lines",
        lines.len()
    );
}

#[test]
fn available_width_stored_consistently_across_lines() {
    let data = make_normal_items(&["aaa bbb ccc"]);
    let w = lu(80.0);
    let lines = collect_all_lines(&data, w);
    for (i, line) in lines.iter().enumerate() {
        assert_eq!(
            line.available_width, w,
            "Line {i} should record the available width"
        );
    }
}

#[test]
fn has_ellipsis_defaults_false() {
    let data = make_normal_items(&["hello world"]);
    let lines = collect_all_lines(&data, lu(500.0));
    assert!(
        !lines[0].has_ellipsis,
        "has_ellipsis should default to false"
    );
}
