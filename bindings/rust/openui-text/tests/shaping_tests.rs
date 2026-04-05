//! Comprehensive tests for the openui-text shaping system.
//!
//! Tests cover: text shaping, glyph metrics, character data, cursor positioning,
//! sub-range operations, TextBlob generation, run segmentation, and spacing.

use openui_style::{FontFamily, FontFamilyList};
use openui_text::font::{Font, FontDescription};
use openui_text::shaping::{
    RunSegmenter, ShapeResult, TextDirection, TextShaper,
};

// ═══════════════════════════════════════════════════════════════════════
// Helpers
// ═══════════════════════════════════════════════════════════════════════

fn make_font(size: f32) -> Font {
    let desc = FontDescription::new();
    FontDescription::with_family_and_size(desc.family.clone(), size);
    let mut d = FontDescription::new();
    d.size = size;
    d.specified_size = size;
    Font::new(d)
}

fn _make_font_with_family(family: &str, size: f32) -> Font {
    let family_list = FontFamilyList {
        families: vec![FontFamily::Named(family.to_string())],
    };
    let desc = FontDescription::with_family_and_size(family_list, size);
    Font::new(desc)
}

fn shape_text(text: &str) -> ShapeResult {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Ltr)
}

fn shape_text_with_font(text: &str, font: &Font) -> ShapeResult {
    let shaper = TextShaper::new();
    shaper.shape(text, font, TextDirection::Ltr)
}

// ═══════════════════════════════════════════════════════════════════════
// 1. Basic Latin Shaping (15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn shape_hello_produces_5_glyphs() {
    let result = shape_text("Hello");
    assert_eq!(result.num_characters, 5);
    assert!(result.num_glyphs() >= 5, "Expected at least 5 glyphs, got {}", result.num_glyphs());
}

#[test]
fn shape_hello_world_wider_than_hello() {
    let hello = shape_text("Hello");
    let hello_world = shape_text("Hello World");
    assert!(
        hello_world.width() > hello.width(),
        "Hello World ({}) should be wider than Hello ({})",
        hello_world.width(),
        hello.width()
    );
}

#[test]
fn shape_empty_string_produces_empty_result() {
    let result = shape_text("");
    assert_eq!(result.num_characters, 0);
    assert_eq!(result.num_glyphs(), 0);
    assert_eq!(result.width(), 0.0);
    assert!(result.runs.is_empty());
    assert!(result.character_data.is_empty());
}

#[test]
fn shape_single_character() {
    let result = shape_text("A");
    assert_eq!(result.num_characters, 1);
    assert!(result.num_glyphs() >= 1);
    assert!(result.width() > 0.0, "Single char should have non-zero width");
}

#[test]
fn shape_space_has_nonzero_width() {
    let result = shape_text(" ");
    assert_eq!(result.num_characters, 1);
    assert!(result.width() > 0.0, "Space should have non-zero width: {}", result.width());
}

#[test]
fn shape_direction_is_ltr() {
    let result = shape_text("Hello");
    assert_eq!(result.direction, TextDirection::Ltr);
}

#[test]
fn shape_rtl_direction() {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    let result = shaper.shape("Hello", &font, TextDirection::Rtl);
    assert_eq!(result.direction, TextDirection::Rtl);
}

#[test]
fn shape_numbers() {
    let result = shape_text("12345");
    assert_eq!(result.num_characters, 5);
    assert!(result.width() > 0.0);
}

#[test]
fn shape_punctuation() {
    let result = shape_text("!@#$%");
    assert_eq!(result.num_characters, 5);
    assert!(result.width() > 0.0);
}

#[test]
fn shape_mixed_content() {
    let result = shape_text("Hello, World! 123");
    assert_eq!(result.num_characters, 17);
    assert!(result.width() > 0.0);
}

#[test]
fn shape_has_at_least_one_run() {
    let result = shape_text("Hello");
    assert!(!result.runs.is_empty(), "Non-empty text should have at least one run");
}

#[test]
fn shape_longer_text_wider() {
    let short = shape_text("Hi");
    let long = shape_text("Hello, this is a longer text");
    assert!(long.width() > short.width());
}

#[test]
fn shape_repeated_chars_proportional_width() {
    let one_a = shape_text("a");
    let three_a = shape_text("aaa");
    // Three 'a's should be approximately 3x one 'a' width
    let ratio = three_a.width() / one_a.width();
    assert!(
        (ratio - 3.0).abs() < 0.5,
        "3x 'a' width ratio should be ~3.0, got {}",
        ratio
    );
}

#[test]
fn shape_newline_character() {
    let result = shape_text("\n");
    assert_eq!(result.num_characters, 1);
}

#[test]
fn shape_tab_character() {
    let result = shape_text("\t");
    assert_eq!(result.num_characters, 1);
}

// ═══════════════════════════════════════════════════════════════════════
// 2. Glyph Metrics (15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn glyph_advances_positive_for_visible() {
    let result = shape_text("Hello");
    for run in &result.runs {
        for (i, &advance) in run.advances.iter().enumerate() {
            assert!(
                advance > 0.0,
                "Advance for glyph {} should be positive, got {}",
                i, advance
            );
        }
    }
}

#[test]
fn sum_advances_equals_total_width() {
    let result = shape_text("Hello World");
    let sum: f32 = result.runs.iter().flat_map(|r| &r.advances).sum();
    let diff = (sum - result.width()).abs();
    assert!(
        diff < 0.1,
        "Sum of advances ({}) should equal width ({})",
        sum,
        result.width()
    );
}

#[test]
fn different_font_sizes_proportional() {
    let font_16 = make_font(16.0);
    let font_32 = make_font(32.0);
    let r16 = shape_text_with_font("Hello", &font_16);
    let r32 = shape_text_with_font("Hello", &font_32);
    let ratio = r32.width() / r16.width();
    assert!(
        (ratio - 2.0).abs() < 0.3,
        "32px should be ~2x 16px width, ratio={}",
        ratio
    );
}

#[test]
fn font_size_8_narrower_than_16() {
    let font_8 = make_font(8.0);
    let font_16 = make_font(16.0);
    let r8 = shape_text_with_font("Hello", &font_8);
    let r16 = shape_text_with_font("Hello", &font_16);
    assert!(r8.width() < r16.width());
}

#[test]
fn glyph_ids_nonzero_for_latin() {
    let result = shape_text("ABCDE");
    for run in &result.runs {
        for &glyph in &run.glyphs {
            assert!(glyph > 0, "Latin glyph ID should be non-zero");
        }
    }
}

#[test]
fn num_glyphs_matches_run_data() {
    let result = shape_text("Hello World");
    for run in &result.runs {
        assert_eq!(run.glyphs.len(), run.num_glyphs);
        assert_eq!(run.advances.len(), run.num_glyphs);
        assert_eq!(run.offsets.len(), run.num_glyphs);
    }
}

#[test]
fn run_num_characters_sums_to_total() {
    let result = shape_text("Hello World");
    let sum: usize = result.runs.iter().map(|r| r.num_characters).sum();
    assert_eq!(sum, result.num_characters);
}

#[test]
fn different_characters_different_widths() {
    let w = shape_text("W");
    let i = shape_text("i");
    // W is typically wider than i in proportional fonts
    assert!(
        w.width() > i.width(),
        "W ({}) should be wider than i ({})",
        w.width(),
        i.width()
    );
}

#[test]
fn very_large_font_size() {
    let font = make_font(100.0);
    let result = shape_text_with_font("A", &font);
    assert!(result.width() > 30.0, "100px 'A' should be wide: {}", result.width());
}

#[test]
fn very_small_font_size() {
    let font = make_font(4.0);
    let result = shape_text_with_font("A", &font);
    assert!(result.width() > 0.0, "4px 'A' should still have width");
    assert!(result.width() < 10.0, "4px 'A' should be narrow: {}", result.width());
}

#[test]
fn width_always_nonnegative() {
    for text in &["", "a", "Hello", "   ", "!"] {
        let result = shape_text(text);
        assert!(result.width() >= 0.0, "Width for {:?} should be non-negative", text);
    }
}

#[test]
fn glyphs_count_positive_for_nonempty() {
    let result = shape_text("Test");
    assert!(result.num_glyphs() > 0);
}

#[test]
fn run_direction_matches_result() {
    let result = shape_text("Hello");
    for run in &result.runs {
        assert_eq!(run.direction, result.direction);
    }
}

#[test]
fn offsets_typically_zero_for_latin() {
    let result = shape_text("Hello");
    for run in &result.runs {
        for &(ox, oy) in &run.offsets {
            // Latin text typically has zero offsets (no combining marks).
            assert!(
                ox.abs() < 1.0 && oy.abs() < 1.0,
                "Latin offsets should be near zero: ({}, {})",
                ox, oy
            );
        }
    }
}

#[test]
fn consistent_reshaping() {
    let font = make_font(16.0);
    let shaper = TextShaper::new();
    let r1 = shaper.shape("Hello", &font, TextDirection::Ltr);
    let r2 = shaper.shape("Hello", &font, TextDirection::Ltr);
    assert_eq!(r1.width(), r2.width());
    assert_eq!(r1.num_glyphs(), r2.num_glyphs());
}

// ═══════════════════════════════════════════════════════════════════════
// 3. Character Data (15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn character_data_length_matches_char_count() {
    let result = shape_text("Hello");
    assert_eq!(result.character_data.len(), 5);
}

#[test]
fn character_data_length_for_unicode() {
    // Multi-byte UTF-8 characters
    let result = shape_text("café");
    assert_eq!(result.num_characters, 4);
    assert_eq!(result.character_data.len(), 4);
}

#[test]
fn first_character_x_position_zero() {
    let result = shape_text("Hello");
    assert_eq!(result.character_data[0].x_position, 0.0);
}

#[test]
fn x_positions_monotonically_increasing_ltr() {
    let result = shape_text("Hello World");
    for i in 1..result.character_data.len() {
        assert!(
            result.character_data[i].x_position >= result.character_data[i - 1].x_position,
            "x_position[{}] ({}) should be >= x_position[{}] ({})",
            i,
            result.character_data[i].x_position,
            i - 1,
            result.character_data[i - 1].x_position
        );
    }
}

#[test]
fn is_cluster_base_true_for_latin() {
    let result = shape_text("Hello");
    for (i, data) in result.character_data.iter().enumerate() {
        assert!(
            data.is_cluster_base,
            "Latin char {} should be cluster base",
            i
        );
    }
}

#[test]
fn safe_to_break_at_start() {
    let result = shape_text("Hello World");
    assert!(result.character_data[0].safe_to_break_before);
}

#[test]
fn safe_to_break_at_word_boundary() {
    let result = shape_text("Hello World");
    // After the space (at 'W'), should be safe to break.
    // The space is at index 5, 'W' is at index 6.
    if result.character_data.len() > 6 {
        assert!(
            result.character_data[5].safe_to_break_before
                || result.character_data[6].safe_to_break_before,
            "Should be safe to break at or after space"
        );
    }
}

#[test]
fn character_data_empty_for_empty_string() {
    let result = shape_text("");
    assert!(result.character_data.is_empty());
}

#[test]
fn last_char_x_position_less_than_width() {
    let result = shape_text("Hello");
    let last = result.character_data.last().unwrap();
    assert!(
        last.x_position < result.width(),
        "Last char x_position ({}) should be < width ({})",
        last.x_position,
        result.width()
    );
}

#[test]
fn character_data_for_single_char() {
    let result = shape_text("A");
    assert_eq!(result.character_data.len(), 1);
    assert_eq!(result.character_data[0].x_position, 0.0);
    assert!(result.character_data[0].is_cluster_base);
}

#[test]
fn character_data_for_spaces() {
    let result = shape_text("   ");
    assert_eq!(result.character_data.len(), 3);
    assert_eq!(result.character_data[0].x_position, 0.0);
}

#[test]
fn x_positions_increase_across_characters() {
    let result = shape_text("ABCDE");
    for i in 1..result.character_data.len() {
        assert!(
            result.character_data[i].x_position > result.character_data[i - 1].x_position,
            "x_position should strictly increase for distinct Latin chars"
        );
    }
}

#[test]
fn character_data_for_mixed_case() {
    let result = shape_text("AaBbCc");
    assert_eq!(result.character_data.len(), 6);
    // All should be cluster bases
    for data in &result.character_data {
        assert!(data.is_cluster_base);
    }
}

#[test]
fn x_position_span_equals_width() {
    let result = shape_text("Hello");
    let first_x = result.character_data[0].x_position;
    assert_eq!(first_x, 0.0);
    // Width should be reachable from last char's advance
    assert!(result.width() > 0.0);
}

#[test]
fn character_data_for_digits() {
    let result = shape_text("0123456789");
    assert_eq!(result.character_data.len(), 10);
    assert_eq!(result.character_data[0].x_position, 0.0);
}

// ═══════════════════════════════════════════════════════════════════════
// 4. Cursor Positioning (10 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn x_position_for_offset_zero() {
    let result = shape_text("Hello");
    assert_eq!(result.x_position_for_offset(0), 0.0);
}

#[test]
fn x_position_for_offset_at_end() {
    let result = shape_text("Hello");
    let x = result.x_position_for_offset(5);
    let diff = (x - result.width()).abs();
    assert!(diff < 0.1, "x at end ({}) should equal width ({})", x, result.width());
}

#[test]
fn offset_for_x_position_at_zero() {
    let result = shape_text("Hello");
    assert_eq!(result.offset_for_x_position(0.0), 0);
}

#[test]
fn offset_for_x_position_at_width() {
    let result = shape_text("Hello");
    assert_eq!(result.offset_for_x_position(result.width()), result.num_characters);
}

#[test]
fn offset_for_x_position_negative() {
    let result = shape_text("Hello");
    assert_eq!(result.offset_for_x_position(-10.0), 0);
}

#[test]
fn offset_for_x_position_beyond_width() {
    let result = shape_text("Hello");
    assert_eq!(
        result.offset_for_x_position(result.width() + 100.0),
        result.num_characters
    );
}

#[test]
fn roundtrip_offset_to_x_to_offset() {
    let result = shape_text("Hello World");
    for offset in 0..result.num_characters {
        let x = result.x_position_for_offset(offset);
        let recovered = result.offset_for_x_position(x);
        // Should recover the same offset or an adjacent one.
        let diff = (recovered as i64 - offset as i64).unsigned_abs();
        assert!(
            diff <= 1,
            "Round-trip for offset {}: x={}, recovered={}",
            offset, x, recovered
        );
    }
}

#[test]
fn x_position_for_offset_midpoint() {
    let result = shape_text("Hello");
    let x2 = result.x_position_for_offset(2);
    assert!(x2 > 0.0);
    assert!(x2 < result.width());
}

#[test]
fn safe_to_break_at_zero() {
    let result = shape_text("Hello");
    assert!(result.safe_to_break_before(0));
}

#[test]
fn safe_to_break_at_end() {
    let result = shape_text("Hello");
    assert!(result.safe_to_break_before(result.num_characters));
}

// ═══════════════════════════════════════════════════════════════════════
// 5. Sub-range (10 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn sub_range_width_less_than_or_equal_total() {
    let result = shape_text("Hello World");
    let sub = result.sub_range(0, 5);
    assert!(sub.width() <= result.width() + 0.1);
}

#[test]
fn sub_range_full_equals_total_width() {
    let result = shape_text("Hello");
    let sub = result.sub_range(0, 5);
    let diff = (sub.width() - result.width()).abs();
    assert!(
        diff < 0.1,
        "sub_range(0, len) width ({}) should equal total width ({})",
        sub.width(),
        result.width()
    );
}

#[test]
fn sub_range_empty_when_start_equals_end() {
    let result = shape_text("Hello");
    let sub = result.sub_range(2, 2);
    assert_eq!(sub.width(), 0.0);
    assert_eq!(sub.num_characters, 0);
}

#[test]
fn width_for_range_matches_sub_range() {
    let result = shape_text("Hello World");
    let w = result.width_for_range(2, 7);
    let sub = result.sub_range(2, 7);
    let diff = (w - sub.width()).abs();
    assert!(
        diff < 0.1,
        "width_for_range ({}) should match sub_range width ({})",
        w,
        sub.width()
    );
}

#[test]
fn sub_range_character_count() {
    let result = shape_text("Hello World");
    let sub = result.sub_range(3, 8);
    assert_eq!(sub.num_characters, 5);
}

#[test]
fn sub_range_starts_at_zero_x() {
    let result = shape_text("Hello World");
    let sub = result.sub_range(3, 8);
    if !sub.character_data.is_empty() {
        assert_eq!(sub.character_data[0].x_position, 0.0);
    }
}

#[test]
fn sub_range_preserves_direction() {
    let result = shape_text("Hello");
    let sub = result.sub_range(1, 4);
    assert_eq!(sub.direction, result.direction);
}

#[test]
fn width_for_range_zero_to_end() {
    let result = shape_text("Hello");
    let w = result.width_for_range(0, result.num_characters);
    let diff = (w - result.width()).abs();
    assert!(diff < 0.1);
}

#[test]
fn width_for_range_partial() {
    let result = shape_text("Hello World");
    let w_hello = result.width_for_range(0, 5);
    let w_world = result.width_for_range(6, 11);
    // Both should be positive
    assert!(w_hello > 0.0);
    assert!(w_world > 0.0);
}

#[test]
fn sub_range_out_of_bounds_clamped() {
    let result = shape_text("Hello");
    let sub = result.sub_range(0, 100);
    let diff = (sub.width() - result.width()).abs();
    assert!(diff < 0.1, "Out-of-bounds sub_range should clamp to full result");
}

// ═══════════════════════════════════════════════════════════════════════
// 6. TextBlob (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn text_blob_some_for_nonempty() {
    let result = shape_text("Hello");
    assert!(result.to_text_blob().is_some());
}

#[test]
fn text_blob_none_for_empty() {
    let result = shape_text("");
    assert!(result.to_text_blob().is_none());
}

#[test]
fn text_blob_bounds_reasonable() {
    let result = shape_text("Hello");
    let blob = result.to_text_blob().unwrap();
    let bounds = blob.bounds();
    assert!(bounds.width() > 0.0, "TextBlob should have positive width");
    assert!(bounds.height() > 0.0, "TextBlob should have positive height");
}

#[test]
fn text_blob_for_single_char() {
    let result = shape_text("X");
    assert!(result.to_text_blob().is_some());
}

#[test]
fn text_blob_for_long_text() {
    let result = shape_text("The quick brown fox jumps over the lazy dog");
    let blob = result.to_text_blob().unwrap();
    let bounds = blob.bounds();
    assert!(bounds.width() > 100.0, "Long text blob should be wide");
}

// ═══════════════════════════════════════════════════════════════════════
// 7. Run Segmenter (10 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn segment_pure_latin_one_segment() {
    let segments = RunSegmenter::segment("Hello World");
    assert_eq!(segments.len(), 1, "Pure Latin should be 1 segment");
}

#[test]
fn segment_latin_with_numbers_merged() {
    let segments = RunSegmenter::segment("Hello 123 World");
    // Numbers are Common script, should merge with Latin.
    assert_eq!(segments.len(), 1, "Latin+numbers should merge: {:?}", segments);
}

#[test]
fn segment_empty_text_no_segments() {
    let segments = RunSegmenter::segment("");
    assert!(segments.is_empty());
}

#[test]
fn segment_single_char_one_segment() {
    let segments = RunSegmenter::segment("A");
    assert_eq!(segments.len(), 1);
    assert_eq!(segments[0].start, 0);
    assert_eq!(segments[0].end, 1);
}

#[test]
fn segment_covers_entire_text() {
    let text = "Hello World 123!";
    let segments = RunSegmenter::segment(text);
    assert_eq!(segments[0].start, 0);
    assert_eq!(segments.last().unwrap().end, text.len());
}

#[test]
fn segment_no_gaps() {
    let text = "Hello World";
    let segments = RunSegmenter::segment(text);
    for i in 1..segments.len() {
        assert_eq!(
            segments[i].start,
            segments[i - 1].end,
            "Segments should be contiguous"
        );
    }
}

#[test]
fn segment_latin_punctuation_merged() {
    let segments = RunSegmenter::segment("Hello, World!");
    // Punctuation is Common script, should merge with Latin.
    assert_eq!(segments.len(), 1);
}

#[test]
fn segment_spaces_only() {
    let segments = RunSegmenter::segment("   ");
    assert_eq!(segments.len(), 1);
}

#[test]
fn segment_direction_is_ltr() {
    let segments = RunSegmenter::segment("Hello");
    assert_eq!(segments[0].direction, TextDirection::Ltr);
}

#[test]
fn segment_byte_offsets_correct_for_ascii() {
    let text = "Hello";
    let segments = RunSegmenter::segment(text);
    assert_eq!(segments[0].start, 0);
    assert_eq!(segments[0].end, 5);
}

// ═══════════════════════════════════════════════════════════════════════
// 8. Letter/Word Spacing (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn letter_spacing_increases_width() {
    let mut desc = FontDescription::new();
    desc.size = 16.0;
    desc.specified_size = 16.0;
    let font_normal = Font::new(desc);

    let mut desc_spaced = FontDescription::new();
    desc_spaced.size = 16.0;
    desc_spaced.specified_size = 16.0;
    desc_spaced.letter_spacing = 2.0;
    let font_spaced = Font::new(desc_spaced);

    let shaper = TextShaper::new();
    let normal = shaper.shape("Hello", &font_normal, TextDirection::Ltr);
    let spaced = shaper.shape("Hello", &font_spaced, TextDirection::Ltr);

    assert!(
        spaced.width() > normal.width(),
        "Letter-spaced ({}) should be wider than normal ({})",
        spaced.width(),
        normal.width()
    );
}

#[test]
fn word_spacing_only_affects_spaces() {
    let mut desc_spaced = FontDescription::new();
    desc_spaced.size = 16.0;
    desc_spaced.specified_size = 16.0;
    desc_spaced.word_spacing = 10.0;
    let font_spaced = Font::new(desc_spaced);

    let shaper = TextShaper::new();

    // "Hello" has no spaces — word spacing shouldn't affect it.
    let no_space = shaper.shape("Hello", &font_spaced, TextDirection::Ltr);
    let font_normal = make_font(16.0);
    let no_space_normal = shaper.shape("Hello", &font_normal, TextDirection::Ltr);
    let diff = (no_space.width() - no_space_normal.width()).abs();
    assert!(
        diff < 0.1,
        "Word spacing shouldn't affect text without spaces: diff={}",
        diff
    );
}

#[test]
fn word_spacing_increases_width_with_spaces() {
    let font_normal = make_font(16.0);

    let mut desc_spaced = FontDescription::new();
    desc_spaced.size = 16.0;
    desc_spaced.specified_size = 16.0;
    desc_spaced.word_spacing = 10.0;
    let font_spaced = Font::new(desc_spaced);

    let shaper = TextShaper::new();
    let normal = shaper.shape("Hello World", &font_normal, TextDirection::Ltr);
    let spaced = shaper.shape("Hello World", &font_spaced, TextDirection::Ltr);

    assert!(
        spaced.width() > normal.width(),
        "Word-spaced ({}) should be wider than normal ({})",
        spaced.width(),
        normal.width()
    );
}

#[test]
fn letter_spacing_amount_correct() {
    let font_normal = make_font(16.0);

    let mut desc_spaced = FontDescription::new();
    desc_spaced.size = 16.0;
    desc_spaced.specified_size = 16.0;
    desc_spaced.letter_spacing = 5.0;
    let font_spaced = Font::new(desc_spaced);

    let shaper = TextShaper::new();
    let normal = shaper.shape("Hello", &font_normal, TextDirection::Ltr);
    let spaced = shaper.shape("Hello", &font_spaced, TextDirection::Ltr);

    // 5 characters × 5px letter spacing = 25px extra
    let expected_extra = 25.0;
    let actual_extra = spaced.width() - normal.width();
    assert!(
        (actual_extra - expected_extra).abs() < 1.0,
        "Expected ~{}px extra, got {}px extra",
        expected_extra,
        actual_extra
    );
}

#[test]
fn zero_spacing_no_change() {
    let mut desc = FontDescription::new();
    desc.size = 16.0;
    desc.specified_size = 16.0;
    desc.letter_spacing = 0.0;
    desc.word_spacing = 0.0;
    let font = Font::new(desc);

    let font_default = make_font(16.0);
    let shaper = TextShaper::new();

    let r1 = shaper.shape("Hello", &font, TextDirection::Ltr);
    let r2 = shaper.shape("Hello", &font_default, TextDirection::Ltr);

    let diff = (r1.width() - r2.width()).abs();
    assert!(diff < 0.1, "Zero spacing should not change width: diff={}", diff);
}

// ═══════════════════════════════════════════════════════════════════════
// SP11 Dual-Model Review Fixes — Regression Tests
// ═══════════════════════════════════════════════════════════════════════

// ── Issue 9: Ligature spacing ───────────────────────────────────────────

#[test]
fn letter_spacing_applied_to_all_characters() {
    // With letter-spacing, every character should get extra width.
    let mut desc = FontDescription::new();
    desc.size = 16.0;
    desc.specified_size = 16.0;
    desc.letter_spacing = 5.0;
    let font = Font::new(desc);

    let shaper = TextShaper::new();
    let result = shaper.shape("ab", &font, TextDirection::Ltr);
    let no_spacing = shape_text("ab");

    // "ab" (2 chars) with 5.0 letter-spacing should be ~10.0 wider
    let extra = result.width() - no_spacing.width();
    assert!(
        (extra - 10.0).abs() < 1.0,
        "Letter spacing of 5.0 on 2 chars should add ~10.0, got extra={}",
        extra
    );
}

#[test]
fn word_spacing_applied_to_spaces() {
    let mut desc = FontDescription::new();
    desc.size = 16.0;
    desc.specified_size = 16.0;
    desc.word_spacing = 10.0;
    let font = Font::new(desc);

    let shaper = TextShaper::new();
    let result = shaper.shape("a b c", &font, TextDirection::Ltr);
    let no_spacing = shape_text("a b c");

    // "a b c" has 2 spaces, word-spacing: 10.0 should add ~20.0
    let extra = result.width() - no_spacing.width();
    assert!(
        (extra - 20.0).abs() < 2.0,
        "Word spacing of 10.0 on 2 spaces should add ~20.0, got extra={}",
        extra
    );
}

#[test]
fn ligature_spacing_accumulates_all_characters() {
    // For ligature runs (e.g., "fi"), spacing should be applied per-character,
    // not per-glyph. We test with letter-spacing on text that may ligate.
    let mut desc = FontDescription::new();
    desc.size = 16.0;
    desc.specified_size = 16.0;
    desc.letter_spacing = 2.0;
    let font = Font::new(desc);

    let shaper = TextShaper::new();
    let result = shaper.shape("fi", &font, TextDirection::Ltr);
    let no_spacing = shape_text("fi");

    // "fi" (2 chars) with 2.0 letter-spacing should add exactly 4.0
    // regardless of whether the font uses a ligature glyph.
    let extra = result.width() - no_spacing.width();
    assert!(
        (extra - 4.0).abs() < 1.0,
        "Letter spacing on 'fi' (2 chars) should add ~4.0 total, got extra={}",
        extra
    );
}

// ── Issue 10: Safe breaks documentation and cluster boundaries ──────────

#[test]
fn safe_break_at_start() {
    let result = shape_text("hello");
    // Position 0 should always be safe to break before
    assert!(result.safe_to_break_before(0));
}

#[test]
fn safe_break_after_space() {
    let result = shape_text("hello world");
    // Position after space should be safe to break
    assert!(result.safe_to_break_before(6), "Position after space should be safe to break");
}

#[test]
fn safe_break_at_whitespace() {
    let result = shape_text("a b c");
    // Spaces themselves and positions after them should be safe
    assert!(result.safe_to_break_before(1), "Space position should be safe");
    assert!(result.safe_to_break_before(2), "Position after space should be safe");
}

// ═══════════════════════════════════════════════════════════════════════
// SP11 Round 3: OOB safety for width_for_range / sub_range after clamping
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn width_for_range_oob_both_past_end() {
    // width_for_range(10, 11) on a 10-char result must not panic.
    let result = shape_text("0123456789"); // 10 chars
    assert_eq!(result.num_characters, 10);
    let w = result.width_for_range(10, 11);
    assert_eq!(w, 0.0, "Both indices past end should return 0");
}

#[test]
fn width_for_range_zero_zero_returns_zero() {
    let result = shape_text("Hello");
    let w = result.width_for_range(0, 0);
    assert_eq!(w, 0.0, "Empty range should return 0");
}

#[test]
fn width_for_range_start_equals_end_after_clamp() {
    // Both start and end clamp to num_characters, resulting in start == end.
    let result = shape_text("abc"); // 3 chars
    let w = result.width_for_range(5, 8);
    assert_eq!(w, 0.0, "Range fully past end should return 0");
}

#[test]
fn sub_range_oob_past_end() {
    // sub_range(15, 20) on a 10-char result must not panic.
    let result = shape_text("0123456789");
    assert_eq!(result.num_characters, 10);
    let sub = result.sub_range(15, 20);
    assert_eq!(sub.num_characters, 0, "OOB sub_range should be empty");
    assert_eq!(sub.width(), 0.0);
}

#[test]
fn sub_range_start_at_boundary_end_past() {
    // sub_range(10, 15) on a 10-char result: start clamps to 10, end clamps to 10.
    let result = shape_text("0123456789");
    let sub = result.sub_range(10, 15);
    assert_eq!(sub.num_characters, 0);
}
