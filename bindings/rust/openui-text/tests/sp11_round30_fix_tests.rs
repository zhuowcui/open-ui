//! Tests for SP11 Round 30 code review fixes — openui-text crate.
//!
//! Issue 3: Proportional glyph range mapping returns empty range for
//! single-character ranges when ratio < 1.0. Fixed by using floor()/ceil()
//! instead of round()/round().

use openui_text::font::{Font, FontDescription};
use openui_text::shaping::{ShapeResult, ShapeResultRun, TextDirection};
use std::sync::Arc;

fn make_font(size: f32) -> Font {
    let mut d = FontDescription::default();
    d.size = size;
    d.specified_size = size;
    Font::new(d)
}

// ── Issue 3: Proportional glyph range mapping ───────────────────────────

#[test]
fn r30_proportional_single_char_not_empty() {
    // 3 glyphs, 10 chars → ratio = 0.3
    // char_start=0, char_end=1 → old: round(0)=0, round(0.3)=0 → empty!
    // fixed: floor(0)=0, ceil(0.3)=1 → (0, 1)
    let font = make_font(16.0);
    let fd = font.primary_font().expect("need font_data").clone();

    let run = ShapeResultRun {
        font_data: Arc::clone(&fd),
        glyphs: vec![1, 2, 3],
        advances: vec![5.0, 5.0, 5.0],
        offsets: vec![(0.0, 0.0); 3],
        clusters: vec![],  // empty clusters → proportional fallback
        start_index: 0,
        num_characters: 10,
        num_glyphs: 3,
        direction: TextDirection::Ltr,
    };

    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 0, 1);
    assert!(
        ge > gs,
        "Single-char range [0,1) must produce non-empty glyph range, got ({}, {})",
        gs, ge,
    );
    assert_eq!((gs, ge), (0, 1));
}

#[test]
fn r30_proportional_middle_char_not_empty() {
    // 3 glyphs, 10 chars → ratio = 0.3
    // char_start=4, char_end=5 → floor(1.2)=1, ceil(1.5)=2 → (1, 2)
    let font = make_font(16.0);
    let fd = font.primary_font().expect("need font_data").clone();

    let run = ShapeResultRun {
        font_data: Arc::clone(&fd),
        glyphs: vec![1, 2, 3],
        advances: vec![5.0, 5.0, 5.0],
        offsets: vec![(0.0, 0.0); 3],
        clusters: vec![],
        start_index: 0,
        num_characters: 10,
        num_glyphs: 3,
        direction: TextDirection::Ltr,
    };

    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 4, 5);
    assert!(
        ge > gs,
        "Single-char range [4,5) must produce non-empty glyph range, got ({}, {})",
        gs, ge,
    );
}

#[test]
fn r30_proportional_full_range_covers_all_glyphs() {
    // Full character range should map to full glyph range.
    let font = make_font(16.0);
    let fd = font.primary_font().expect("need font_data").clone();

    let run = ShapeResultRun {
        font_data: Arc::clone(&fd),
        glyphs: vec![1, 2, 3],
        advances: vec![5.0, 5.0, 5.0],
        offsets: vec![(0.0, 0.0); 3],
        clusters: vec![],
        start_index: 0,
        num_characters: 10,
        num_glyphs: 3,
        direction: TextDirection::Ltr,
    };

    let (gs, ge) = ShapeResult::glyph_range_for_char_range(&run, 0, 10);
    assert_eq!((gs, ge), (0, 3));
}

#[test]
fn r30_proportional_clamped_to_num_glyphs() {
    // End should never exceed num_glyphs.
    let font = make_font(16.0);
    let fd = font.primary_font().expect("need font_data").clone();

    let run = ShapeResultRun {
        font_data: Arc::clone(&fd),
        glyphs: vec![1, 2],
        advances: vec![5.0, 5.0],
        offsets: vec![(0.0, 0.0); 2],
        clusters: vec![],
        start_index: 0,
        num_characters: 3,
        num_glyphs: 2,
        direction: TextDirection::Ltr,
    };

    let (_, ge) = ShapeResult::glyph_range_for_char_range(&run, 0, 3);
    assert!(ge <= run.num_glyphs, "ge={} should be <= num_glyphs={}", ge, run.num_glyphs);
}
