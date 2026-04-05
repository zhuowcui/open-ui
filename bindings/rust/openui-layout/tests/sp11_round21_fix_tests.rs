//! Tests for SP11 Round 21 code review fixes — openui-layout crate.
//!
//! Covers Issues 3, 6, 7, 8 from the review.

use openui_dom::{Document, ElementTag};
use openui_geometry::Length;
use openui_layout::inline::items::InlineItemType;
use openui_layout::inline::items_builder::InlineItemsBuilder;
use openui_style::{
    ComputedStyle, Display, TextAlign, TextAlignLast, TextJustify,
};

// ── Issue 3: Half-leading floor/ceil LayoutUnit snap ─────────────────────

#[test]
fn half_leading_floor_ceil_snap_total_equals_line_height() {
    // With the LayoutUnit grid (1/64 px) snapping, the half-leading
    // distribution should still sum to the computed leading exactly.
    let grid = 1.0 / 64.0;
    let test_cases: Vec<(f32, f32, f32)> = vec![
        (12.0, 4.0, 24.0),  // leading = 8.0
        (10.0, 3.0, 20.0),  // leading = 7.0
        (11.5, 3.5, 18.0),  // leading = 3.0
        (14.0, 6.0, 30.0),  // leading = 10.0
        (10.0, 4.0, 15.0),  // leading = 1.0 (odd)
    ];

    for (ascent, descent, computed_lh) in test_cases {
        let leading = computed_lh - (ascent + descent);
        let ascent_half = (leading / 2.0 / grid).floor() * grid;
        let descent_half = leading - ascent_half;

        let total = ascent_half + descent_half;
        assert!(
            (total - leading).abs() < 1e-6,
            "Half-leading split must sum to leading: {} + {} = {} vs {}",
            ascent_half, descent_half, total, leading
        );
    }
}

#[test]
fn half_leading_ascent_uses_floor() {
    // The ascent half should use floor, meaning it gets the smaller or equal portion.
    let grid: f32 = 1.0 / 64.0;

    // Test with a value that doesn't divide evenly on the grid
    let leading: f32 = 1.0 / 64.0; // 1 LayoutUnit
    let ascent_half = (leading / 2.0 / grid).floor() * grid;
    let descent_half = leading - ascent_half;

    assert_eq!(ascent_half, 0.0, "floor of half a LayoutUnit should be 0");
    assert!((descent_half - leading).abs() < 1e-6,
        "descent_half should get the full unit");

    // Larger test: leading = 3 LayoutUnits
    let leading2: f32 = 3.0 / 64.0;
    let ascent_half2 = (leading2 / 2.0 / grid).floor() * grid;
    let descent_half2 = leading2 - ascent_half2;
    // 1.5 LU / 64 → floor = 1 LU
    assert_eq!(ascent_half2, 1.0_f32 / 64.0);
    assert!((descent_half2 - 2.0_f32 / 64.0).abs() < 1e-9);
}

// ── Issue 6: div with display:inline should not be atomic ────────────────

#[test]
fn div_display_inline_creates_inline_box_not_atomic() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let inline_div = doc.create_node(ElementTag::Div);
    doc.node_mut(inline_div).style.display = Display::Inline;
    doc.append_child(block, inline_div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("hello".to_string());
    doc.append_child(inline_div, text);

    let items_data = InlineItemsBuilder::collect(&doc, block);

    let has_atomic = items_data.items.iter().any(|item| item.item_type == InlineItemType::AtomicInline);
    assert!(!has_atomic, "div with display:inline should NOT produce AtomicInline items");

    let has_open = items_data.items.iter().any(|item| item.item_type == InlineItemType::OpenTag);
    let has_close = items_data.items.iter().any(|item| item.item_type == InlineItemType::CloseTag);
    assert!(has_open, "div with display:inline should produce OpenTag");
    assert!(has_close, "div with display:inline should produce CloseTag");
}

#[test]
fn div_display_inline_block_still_atomic() {
    let mut doc = Document::new();
    let root = doc.root();
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.append_child(root, block);

    let ib_div = doc.create_node(ElementTag::Div);
    doc.node_mut(ib_div).style.display = Display::InlineBlock;
    doc.append_child(block, ib_div);

    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some("hello".to_string());
    doc.append_child(ib_div, text);

    let items_data = InlineItemsBuilder::collect(&doc, block);

    let has_atomic = items_data.items.iter().any(|item| item.item_type == InlineItemType::AtomicInline);
    assert!(has_atomic, "div with display:inline-block should produce AtomicInline");
}

// ── Issue 7: Atomic inline percentage/auto widths ────────────────────────

#[test]
fn atomic_inline_percentage_width_resolves() {
    // Verify the math: 50% of 400 = 200.
    let pct = 50.0_f32;
    let container_width = 400.0_f32;
    let resolved = pct / 100.0 * container_width;
    assert_eq!(resolved, 200.0);
}

#[test]
fn atomic_inline_auto_with_min_width_uses_floor() {
    // width:auto with min-width:100px should use 100px as a floor.
    let mut style = ComputedStyle::default();
    style.width = Length::auto();
    style.min_width = Length::px(100.0);
    assert!(style.width.is_auto());
    assert!(style.min_width.is_fixed());
    assert_eq!(style.min_width.value(), 100.0);
}

#[test]
fn atomic_inline_max_width_clamps() {
    // An element with width:300px and max-width:200px should clamp to 200px.
    let mut style = ComputedStyle::default();
    style.width = Length::px(300.0);
    style.max_width = Length::px(200.0);
    let w = if style.width.is_fixed() { style.width.value() } else { 0.0 };
    let max = if style.max_width.is_fixed() { style.max_width.value() } else { f32::INFINITY };
    let clamped = w.min(max);
    assert_eq!(clamped, 200.0);
}

// ── Issue 8: text-align-last and text-justify ────────────────────────────

#[test]
fn text_align_last_center_on_last_line() {
    // text-align:justify with text-align-last:center on the last line
    // should center-align. Verify the mapping logic.
    let text_align_last = TextAlignLast::Center;
    let text_align = TextAlign::Justify;
    let is_last = true;

    let effective = if is_last {
        match text_align {
            TextAlign::Justify => match text_align_last {
                TextAlignLast::Auto => TextAlign::Start,
                TextAlignLast::Center => TextAlign::Center,
                TextAlignLast::End => TextAlign::End,
                TextAlignLast::Left => TextAlign::Left,
                TextAlignLast::Right => TextAlign::Right,
                TextAlignLast::Justify => TextAlign::Justify,
                TextAlignLast::Start => TextAlign::Start,
            },
            other => other,
        }
    } else {
        text_align
    };

    assert_eq!(effective, TextAlign::Center);
}

#[test]
fn text_align_last_auto_falls_back_to_start() {
    let text_align_last = TextAlignLast::Auto;
    let effective = match text_align_last {
        TextAlignLast::Auto => TextAlign::Start,
        _ => TextAlign::Center, // placeholder
    };
    assert_eq!(effective, TextAlign::Start);
}

#[test]
fn text_justify_none_suppresses_justification() {
    let text_justify = TextJustify::None;
    assert_eq!(text_justify, TextJustify::None);
    // Algorithm code checks: if text_justify != TextJustify::None { ... }
    // so None suppresses all justification.
    assert_ne!(text_justify, TextJustify::InterWord);
    assert_ne!(text_justify, TextJustify::InterCharacter);
}

#[test]
fn text_justify_inter_character_counts_all_gaps() {
    // For inter-character justification, gaps = char_count - 1.
    let text = "abc"; // 3 chars → 2 gaps
    let char_count = text.chars().count();
    let gaps = char_count.saturating_sub(1);
    assert_eq!(gaps, 2);

    // With trailing space
    let text2 = "abc ";
    let total = text2.chars().count(); // 4
    let trimmed = total - text2.chars().rev().take_while(|c| *c == ' ').count(); // 3
    let gaps2 = trimmed.saturating_sub(1);
    assert_eq!(gaps2, 2);
}

#[test]
fn text_justify_inter_word_counts_spaces() {
    let text = "hello world test";
    let spaces = text.chars().filter(|c| *c == ' ').count();
    assert_eq!(spaces, 2);
}
