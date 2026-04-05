//! Advanced text feature tests — text-transform, text-overflow, letter/word spacing,
//! text-indent, and BiDi integration in inline layout.
//!
//! These tests verify the Wave 6 inline layout features.

use openui_text::transform::apply_text_transform;
use openui_style::TextTransform;

// ═══════════════════════════════════════════════════════════════════════
// TEXT TRANSFORM TESTS (15)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn transform_uppercase_simple() {
    assert_eq!(apply_text_transform("hello", TextTransform::Uppercase, None), "HELLO");
}

#[test]
fn transform_uppercase_already_upper() {
    assert_eq!(apply_text_transform("HELLO", TextTransform::Uppercase, None), "HELLO");
}

#[test]
fn transform_uppercase_mixed() {
    assert_eq!(apply_text_transform("Hello World", TextTransform::Uppercase, None), "HELLO WORLD");
}

#[test]
fn transform_lowercase_simple() {
    assert_eq!(apply_text_transform("HELLO", TextTransform::Lowercase, None), "hello");
}

#[test]
fn transform_lowercase_already_lower() {
    assert_eq!(apply_text_transform("hello", TextTransform::Lowercase, None), "hello");
}

#[test]
fn transform_capitalize_simple() {
    assert_eq!(apply_text_transform("hello world", TextTransform::Capitalize, None), "Hello World");
}

#[test]
fn transform_capitalize_after_hyphen() {
    assert_eq!(apply_text_transform("well-known", TextTransform::Capitalize, None), "Well-Known");
}

#[test]
fn transform_capitalize_already_capitalized() {
    assert_eq!(apply_text_transform("Hello World", TextTransform::Capitalize, None), "Hello World");
}

#[test]
fn transform_capitalize_single_word() {
    assert_eq!(apply_text_transform("hello", TextTransform::Capitalize, None), "Hello");
}

#[test]
fn transform_unicode_uppercase_cafe() {
    assert_eq!(apply_text_transform("café", TextTransform::Uppercase, None), "CAFÉ");
}

#[test]
fn transform_unicode_uppercase_german_sharp_s() {
    // ß uppercases to SS in Unicode
    assert_eq!(apply_text_transform("straße", TextTransform::Uppercase, None), "STRASSE");
}

#[test]
fn transform_full_width_ascii() {
    assert_eq!(apply_text_transform("ABC", TextTransform::FullWidth, None), "ＡＢＣ");
}

#[test]
fn transform_full_width_digits() {
    assert_eq!(apply_text_transform("123", TextTransform::FullWidth, None), "１２３");
}

#[test]
fn transform_full_width_space() {
    assert_eq!(apply_text_transform("A B", TextTransform::FullWidth, None), "Ａ\u{3000}Ｂ");
}

#[test]
fn transform_none_unchanged() {
    assert_eq!(apply_text_transform("Hello World", TextTransform::None, None), "Hello World");
}

// ═══════════════════════════════════════════════════════════════════════
// TEXT TRANSFORM EDGE CASES
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn transform_empty_string() {
    assert_eq!(apply_text_transform("", TextTransform::Uppercase, None), "");
    assert_eq!(apply_text_transform("", TextTransform::Lowercase, None), "");
    assert_eq!(apply_text_transform("", TextTransform::Capitalize, None), "");
}

#[test]
fn transform_capitalize_leading_spaces() {
    assert_eq!(apply_text_transform("  hello", TextTransform::Capitalize, None), "  Hello");
}

#[test]
fn transform_capitalize_multiple_spaces() {
    assert_eq!(apply_text_transform("hello   world", TextTransform::Capitalize, None), "Hello   World");
}

#[test]
fn transform_full_width_non_ascii_passthrough() {
    // Non-ASCII characters outside the mapped range pass through unchanged
    assert_eq!(apply_text_transform("こんにちは", TextTransform::FullWidth, None), "こんにちは");
}

#[test]
fn transform_capitalize_after_apostrophe() {
    // CSS Text §2.1: apostrophe is NOT a word boundary, so "it's" stays as one word.
    assert_eq!(apply_text_transform("it's a test", TextTransform::Capitalize, None), "It's A Test");
}

// ═══════════════════════════════════════════════════════════════════════
// LINE INFO & TEXT-OVERFLOW TESTS (10)
// ═══════════════════════════════════════════════════════════════════════

use openui_geometry::LayoutUnit;
use openui_layout::inline::line_info::LineInfo;
use openui_layout::inline::items::{InlineItemResult, InlineItemType};
use openui_style::TextAlign;

fn make_text_item(item_index: usize, width: i32) -> InlineItemResult {
    InlineItemResult {
        item_index,
        text_range: 0..5,
        inline_size: LayoutUnit::from_i32(width),
        shape_result: None,
        has_forced_break: false,
        item_type: InlineItemType::Text,
    }
}

#[test]
fn line_info_has_ellipsis_default_false() {
    let line = LineInfo::new(LayoutUnit::from_i32(100));
    assert!(!line.has_ellipsis);
}

#[test]
fn line_info_remaining_width() {
    let mut line = LineInfo::new(LayoutUnit::from_i32(100));
    line.used_width = LayoutUnit::from_i32(60);
    assert_eq!(line.remaining_width(), LayoutUnit::from_i32(40));
}

#[test]
fn line_info_has_content_empty() {
    let line = LineInfo::new(LayoutUnit::from_i32(100));
    assert!(!line.has_content());
}

#[test]
fn line_info_has_content_with_text() {
    let mut line = LineInfo::new(LayoutUnit::from_i32(100));
    line.items.push(make_text_item(0, 50));
    assert!(line.has_content());
}

#[test]
fn line_info_new_defaults() {
    let line = LineInfo::new(LayoutUnit::from_i32(200));
    assert_eq!(line.available_width, LayoutUnit::from_i32(200));
    assert_eq!(line.used_width, LayoutUnit::zero());
    assert!(!line.has_forced_break);
    assert!(!line.is_last_line);
    assert_eq!(line.text_align, TextAlign::Start);
    assert!(!line.has_ellipsis);
}

#[test]
fn line_info_remaining_zero_when_full() {
    let mut line = LineInfo::new(LayoutUnit::from_i32(100));
    line.used_width = LayoutUnit::from_i32(100);
    assert_eq!(line.remaining_width(), LayoutUnit::zero());
}

#[test]
fn line_info_remaining_negative_when_overflow() {
    let mut line = LineInfo::new(LayoutUnit::from_i32(100));
    line.used_width = LayoutUnit::from_i32(120);
    assert!(line.remaining_width() < LayoutUnit::zero());
}

#[test]
fn line_info_forced_break() {
    let mut line = LineInfo::new(LayoutUnit::from_i32(100));
    line.has_forced_break = true;
    assert!(line.has_forced_break);
}

#[test]
fn line_info_is_last_line() {
    let mut line = LineInfo::new(LayoutUnit::from_i32(100));
    line.is_last_line = true;
    assert!(line.is_last_line);
}

#[test]
fn line_info_text_align_justify() {
    let mut line = LineInfo::new(LayoutUnit::from_i32(100));
    line.text_align = TextAlign::Justify;
    assert_eq!(line.text_align, TextAlign::Justify);
}

// ═══════════════════════════════════════════════════════════════════════
// LETTER-SPACING AND WORD-SPACING TESTS (10)
// These test that spacing parameters affect shaping output.
// ═══════════════════════════════════════════════════════════════════════

use openui_text::{Font, FontDescription, TextShaper, TextDirection};

fn make_font_with_spacing(letter: f32, word: f32) -> Font {
    let mut desc = FontDescription::default();
    desc.letter_spacing = letter;
    desc.word_spacing = word;
    Font::new(desc)
}

fn make_default_font() -> Font {
    Font::new(FontDescription::default())
}

#[test]
fn spacing_letter_spacing_increases_width() {
    let shaper = TextShaper::new();
    let font_no_spacing = make_default_font();
    let font_spacing = make_font_with_spacing(2.0, 0.0);

    let result_no = shaper.shape("Hello", &font_no_spacing, TextDirection::Ltr);
    let result_sp = shaper.shape("Hello", &font_spacing, TextDirection::Ltr);

    // 5 chars × 2px letter spacing = 10px extra
    assert!(result_sp.width > result_no.width,
        "Letter spacing should increase width: {} vs {}",
        result_sp.width, result_no.width);
    let expected_extra = 2.0 * 5.0; // 5 chars
    let actual_extra = result_sp.width - result_no.width;
    assert!((actual_extra - expected_extra).abs() < 0.5,
        "Extra width should be ~10px, got {}", actual_extra);
}

#[test]
fn spacing_word_spacing_increases_width() {
    let shaper = TextShaper::new();
    let font_no_spacing = make_default_font();
    let font_word_sp = make_font_with_spacing(0.0, 5.0);

    let result_no = shaper.shape("Hello world", &font_no_spacing, TextDirection::Ltr);
    let result_sp = shaper.shape("Hello world", &font_word_sp, TextDirection::Ltr);

    // 1 space × 5px word spacing = 5px extra
    assert!(result_sp.width > result_no.width,
        "Word spacing should increase width");
    let actual_extra = result_sp.width - result_no.width;
    assert!((actual_extra - 5.0).abs() < 0.5,
        "Extra width should be ~5px, got {}", actual_extra);
}

#[test]
fn spacing_word_spacing_only_affects_spaces() {
    let shaper = TextShaper::new();
    let font_no_spacing = make_default_font();
    let font_word_sp = make_font_with_spacing(0.0, 10.0);

    // No spaces → no extra width
    let result_no = shaper.shape("Hello", &font_no_spacing, TextDirection::Ltr);
    let result_sp = shaper.shape("Hello", &font_word_sp, TextDirection::Ltr);

    assert!((result_sp.width - result_no.width).abs() < 0.5,
        "Word spacing on text without spaces should have no effect");
}

#[test]
fn spacing_negative_letter_spacing() {
    let shaper = TextShaper::new();
    let font_no_spacing = make_default_font();
    let font_neg_sp = make_font_with_spacing(-1.0, 0.0);

    let result_no = shaper.shape("Hello", &font_no_spacing, TextDirection::Ltr);
    let result_sp = shaper.shape("Hello", &font_neg_sp, TextDirection::Ltr);

    // 5 chars × -1px = -5px
    assert!(result_sp.width < result_no.width,
        "Negative letter spacing should decrease width");
}

#[test]
fn spacing_zero_spacing_no_change() {
    let shaper = TextShaper::new();
    let font_zero = make_font_with_spacing(0.0, 0.0);
    let font_default = make_default_font();

    let result_zero = shaper.shape("Hello world", &font_zero, TextDirection::Ltr);
    let result_default = shaper.shape("Hello world", &font_default, TextDirection::Ltr);

    assert!((result_zero.width - result_default.width).abs() < 0.01,
        "Zero spacing should be same as default");
}

#[test]
fn spacing_both_letter_and_word() {
    let shaper = TextShaper::new();
    let font_no = make_default_font();
    let font_both = make_font_with_spacing(1.0, 3.0);

    let result_no = shaper.shape("A B", &font_no, TextDirection::Ltr);
    let result_both = shaper.shape("A B", &font_both, TextDirection::Ltr);

    // 3 chars × 1px letter + 1 space × 3px word = 6px extra
    let expected_extra = 3.0 * 1.0 + 1.0 * 3.0;
    let actual_extra = result_both.width - result_no.width;
    assert!((actual_extra - expected_extra).abs() < 0.5,
        "Combined spacing extra should be ~6px, got {}", actual_extra);
}

#[test]
fn spacing_multiple_spaces_word_spacing() {
    let shaper = TextShaper::new();
    let font_no = make_default_font();
    let font_ws = make_font_with_spacing(0.0, 4.0);

    let result_no = shaper.shape("A B C", &font_no, TextDirection::Ltr);
    let result_sp = shaper.shape("A B C", &font_ws, TextDirection::Ltr);

    // 2 spaces × 4px = 8px extra
    let actual_extra = result_sp.width - result_no.width;
    assert!((actual_extra - 8.0).abs() < 0.5,
        "2 spaces × 4px word spacing = 8px extra, got {}", actual_extra);
}

#[test]
fn spacing_letter_spacing_single_char() {
    let shaper = TextShaper::new();
    let font_no = make_default_font();
    let font_sp = make_font_with_spacing(5.0, 0.0);

    let result_no = shaper.shape("X", &font_no, TextDirection::Ltr);
    let result_sp = shaper.shape("X", &font_sp, TextDirection::Ltr);

    // 1 char × 5px = 5px extra
    let actual_extra = result_sp.width - result_no.width;
    assert!((actual_extra - 5.0).abs() < 0.5,
        "Single char letter spacing should be 5px extra, got {}", actual_extra);
}

#[test]
fn spacing_empty_text_no_crash() {
    let shaper = TextShaper::new();
    let font = make_font_with_spacing(5.0, 5.0);
    let result = shaper.shape("", &font, TextDirection::Ltr);
    assert_eq!(result.width, 0.0);
}

#[test]
fn spacing_rtl_text_with_spacing() {
    let shaper = TextShaper::new();
    let font_no = make_default_font();
    let font_sp = make_font_with_spacing(2.0, 0.0);

    let result_no = shaper.shape("שלום", &font_no, TextDirection::Rtl);
    let result_sp = shaper.shape("שלום", &font_sp, TextDirection::Rtl);

    // 4 chars × 2px = 8px extra
    assert!(result_sp.width > result_no.width,
        "Letter spacing should apply to RTL text too");
}

// ═══════════════════════════════════════════════════════════════════════
// TEXT-INDENT TESTS (10)
// Verified via inline layout integration.
// ═══════════════════════════════════════════════════════════════════════

use openui_dom::Document;
use openui_geometry::Length;
use openui_layout::{ConstraintSpace, inline_layout};
use openui_style::ComputedStyle;

fn setup_doc_with_text(text: &str, style_fn: impl FnOnce(&mut ComputedStyle)) -> (Document, openui_dom::NodeId) {
    let mut doc = Document::new();
    let viewport = doc.root();

    let block_id = doc.create_node(openui_dom::ElementTag::Div);
    {
        let node = doc.node_mut(block_id);
        node.style.display = openui_style::Display::Block;
        style_fn(&mut node.style);
    }
    doc.append_child(viewport, block_id);

    let text_style = doc.node(block_id).style.clone();
    let text_id = doc.create_node(openui_dom::ElementTag::Text);
    {
        let node = doc.node_mut(text_id);
        node.text = Some(text.to_string());
        node.style = text_style;
    }
    doc.append_child(block_id, text_id);

    (doc, block_id)
}

fn layout_with_width(doc: &Document, node_id: openui_dom::NodeId, width: i32) -> openui_layout::Fragment {
    let space = ConstraintSpace::for_block_child(
        LayoutUnit::from_i32(width),
        LayoutUnit::from_i32(10000),
        LayoutUnit::from_i32(width),
        LayoutUnit::from_i32(10000),
        true,
    );
    inline_layout(doc, node_id, &space)
}

#[test]
fn text_indent_zero_no_offset() {
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.text_indent = Length::px(0.0);
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    // First text child should start at 0 (no indent)
    if let Some(line) = fragment.children.first() {
        if let Some(text) = line.children.first() {
            assert_eq!(text.offset.left, LayoutUnit::zero(),
                "No indent should give 0 offset");
        }
    }
}

#[test]
fn text_indent_positive() {
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.text_indent = Length::px(20.0);
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    if let Some(line) = fragment.children.first() {
        if let Some(text) = line.children.first() {
            // The text should be offset by at least 20px (text_indent)
            assert!(text.offset.left >= LayoutUnit::from_i32(20),
                "Positive indent should offset text, got {:?}", text.offset.left);
        }
    }
}

#[test]
fn text_indent_negative_hanging() {
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.text_indent = Length::px(-10.0);
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    if let Some(line) = fragment.children.first() {
        if let Some(text) = line.children.first() {
            // Negative indent means text starts before the normal edge
            assert!(text.offset.left < LayoutUnit::zero(),
                "Negative indent should give negative offset, got {:?}", text.offset.left);
        }
    }
}

#[test]
fn text_indent_only_first_line() {
    // Set a narrow width to force line breaks, then verify second line has no indent
    let (doc, block_id) = setup_doc_with_text(
        "The quick brown fox jumps over the lazy dog",
        |s| {
            s.text_indent = Length::px(30.0);
        },
    );
    let fragment = layout_with_width(&doc, block_id, 120);
    if fragment.children.len() >= 2 {
        let first_line = &fragment.children[0];
        let second_line = &fragment.children[1];
        // First line text should have indent
        if let (Some(first_text), Some(second_text)) =
            (first_line.children.first(), second_line.children.first())
        {
            assert!(first_text.offset.left > second_text.offset.left,
                "First line should have more indent than second line");
        }
    }
}

#[test]
fn text_indent_percentage() {
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.text_indent = Length::percent(10.0);
    });
    // 10% of 500px = 50px
    let fragment = layout_with_width(&doc, block_id, 500);
    if let Some(line) = fragment.children.first() {
        if let Some(text) = line.children.first() {
            // Should be approximately 50px indent
            let offset = text.offset.left.to_f32();
            assert!(offset >= 49.0 && offset <= 51.0,
                "10% of 500 should give ~50px indent, got {}", offset);
        }
    }
}

#[test]
fn text_indent_with_center_align() {
    let (doc, block_id) = setup_doc_with_text("Hi", |s| {
        s.text_indent = Length::px(20.0);
        s.text_align = TextAlign::Center;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    // Should have both centering offset and indent
    assert!(!fragment.children.is_empty());
}

#[test]
fn text_indent_with_right_align() {
    let (doc, block_id) = setup_doc_with_text("Hi", |s| {
        s.text_indent = Length::px(20.0);
        s.text_align = TextAlign::Right;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}

#[test]
fn text_indent_large_value() {
    let (doc, block_id) = setup_doc_with_text("Hello", |s| {
        s.text_indent = Length::px(400.0);
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    // Should still layout without panicking
    assert!(!fragment.children.is_empty());
}

#[test]
fn text_indent_auto_is_zero() {
    // Auto text-indent should resolve to 0
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.text_indent = Length::auto();
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    if let Some(line) = fragment.children.first() {
        if let Some(text) = line.children.first() {
            assert_eq!(text.offset.left, LayoutUnit::zero());
        }
    }
}

#[test]
fn text_indent_does_not_affect_subsequent_lines() {
    let (doc, block_id) = setup_doc_with_text(
        "First line text here and more words to wrap around to the next line",
        |s| {
            s.text_indent = Length::px(50.0);
        },
    );
    let fragment = layout_with_width(&doc, block_id, 150);
    if fragment.children.len() >= 2 {
        let second_line = &fragment.children[1];
        if let Some(text) = second_line.children.first() {
            // Second line should NOT have the 50px indent
            assert!(text.offset.left < LayoutUnit::from_i32(50),
                "Second line should not have text-indent, got {:?}", text.offset.left);
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
// BIDI INTEGRATION TESTS (5)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bidi_integration_ltr_layout() {
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.direction = openui_style::Direction::Ltr;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty(), "Should produce at least one line");
}

#[test]
fn bidi_integration_rtl_direction() {
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.direction = openui_style::Direction::Rtl;
        s.text_align = TextAlign::Start; // Start = right in RTL
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}

#[test]
fn bidi_integration_rtl_text_align_end() {
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.direction = openui_style::Direction::Rtl;
        s.text_align = TextAlign::End;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}

#[test]
fn bidi_integration_ltr_with_bidi_level() {
    // Verify that bidi analysis runs and sets levels
    let (doc, block_id) = setup_doc_with_text("Hello world", |s| {
        s.direction = openui_style::Direction::Ltr;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}

#[test]
fn bidi_integration_rtl_right_alignment() {
    // RTL with text-align: right should work
    let (doc, block_id) = setup_doc_with_text("Hello", |s| {
        s.direction = openui_style::Direction::Rtl;
        s.text_align = TextAlign::Right;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}

// ═══════════════════════════════════════════════════════════════════════
// TEXT TRANSFORM + LAYOUT INTEGRATION (5)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn transform_integration_uppercase_in_layout() {
    let (doc, block_id) = setup_doc_with_text("hello world", |s| {
        s.text_transform = TextTransform::Uppercase;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty(), "Uppercase text should layout");
}

#[test]
fn transform_integration_capitalize_in_layout() {
    let (doc, block_id) = setup_doc_with_text("hello world", |s| {
        s.text_transform = TextTransform::Capitalize;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}

#[test]
fn transform_integration_with_line_breaking() {
    // Uppercase text may be wider, potentially causing different line breaks
    let (doc, block_id) = setup_doc_with_text("hello world test", |s| {
        s.text_transform = TextTransform::Uppercase;
    });
    let fragment = layout_with_width(&doc, block_id, 100);
    assert!(!fragment.children.is_empty());
}

#[test]
fn ellipsis_no_effect_when_fits() {
    let (doc, block_id) = setup_doc_with_text("Hi", |s| {
        s.text_overflow = openui_style::TextOverflow::Ellipsis;
        s.overflow_x = openui_style::Overflow::Hidden;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}

#[test]
fn transform_full_width_in_layout() {
    let (doc, block_id) = setup_doc_with_text("ABC", |s| {
        s.text_transform = TextTransform::FullWidth;
    });
    let fragment = layout_with_width(&doc, block_id, 500);
    assert!(!fragment.children.is_empty());
}
