// SP13 Phase F: Inline Fragmentation Tests
//
// Tests for line-level fragmentation at fragmentainer boundaries (multicol,
// print) and orphans/widows enforcement.
//
// CSS Break 3 §3: Line boxes are class-B break points.
// CSS Break 3 §4.1: Orphans and widows constraints.
//
// Blink: InlineLayoutAlgorithm::BreakLine(), BreakBeforeLine()

use openui_geometry::{LayoutUnit, PhysicalSize};
use openui_layout::inline::{
    algorithm::{apply_inline_fragmentation, resume_inline_from_break_token},
};
use openui_layout::fragmentation::{BreakToken, InlineBreakToken};
use openui_layout::Fragment;

fn lu(v: f32) -> LayoutUnit {
    LayoutUnit::from_f32(v)
}

// ── InlineBreakToken unit tests ─────────────────────────────────────────

#[test]
fn inline_break_token_creation() {
    let token = InlineBreakToken::new(3, lu(60.0));
    assert_eq!(token.lines_consumed, 3);
    assert_eq!(token.consumed_block_size, lu(60.0));
}

#[test]
fn inline_break_token_zero() {
    let token = InlineBreakToken::new(0, LayoutUnit::zero());
    assert_eq!(token.lines_consumed, 0);
    assert_eq!(token.consumed_block_size, LayoutUnit::zero());
}

#[test]
fn break_token_inline_variant() {
    let token = BreakToken::Inline(InlineBreakToken::new(5, lu(100.0)));
    match &token {
        BreakToken::Inline(t) => {
            assert_eq!(t.lines_consumed, 5);
            assert_eq!(t.consumed_block_size, lu(100.0));
        }
        _ => panic!("Expected Inline variant"),
    }
}

// ── apply_inline_fragmentation tests ────────────────────────────────────

#[test]
fn no_fragmentation_when_all_lines_fit() {
    // 4 lines × 20px = 80px. Fragmentainer is 2000px — all fit easily.
    let frag = make_fake_line_fragment(4, 20.0, 200.0);
    let num_lines = frag.children.len();
    assert_eq!(num_lines, 4);

    let fragmented = apply_inline_fragmentation(
        frag,
        lu(2000.0), // huge fragmentainer
        LayoutUnit::zero(),
        0,
        2, // orphans
        2, // widows
    );

    assert!(fragmented.break_token.is_none(), "No break token expected when all lines fit");
    assert_eq!(fragmented.children.len(), num_lines);
}

#[test]
fn fragmentation_splits_at_boundary() {
    // Create a fragment with 4 line boxes, each 20px tall.
    let frag = make_fake_line_fragment(4, 20.0, 200.0);
    assert_eq!(frag.children.len(), 4);

    // Fragmentainer is 50px — should fit 2 lines (0..40px), break before line 3.
    let result = apply_inline_fragmentation(
        frag,
        lu(50.0),
        LayoutUnit::zero(),
        0,
        1, // orphans
        1, // widows
    );

    assert_eq!(result.children.len(), 2, "Should keep 2 lines in 50px fragmentainer");
    assert!(result.break_token.is_some(), "Should have break token");
    match &result.break_token {
        Some(BreakToken::Inline(t)) => {
            assert_eq!(t.lines_consumed, 2);
        }
        _ => panic!("Expected Inline break token"),
    }
}

#[test]
fn fragmentation_with_offset_in_fragmentainer() {
    // 4 lines × 20px = 80px total. Fragmentainer is 60px but we start 30px in.
    // Available = 30px, so only 1 line fits.
    let frag = make_fake_line_fragment(4, 20.0, 200.0);

    let result = apply_inline_fragmentation(
        frag,
        lu(60.0),
        lu(30.0), // offset
        0,
        1,
        1,
    );

    assert_eq!(result.children.len(), 1, "Only 1 line should fit with 30px available");
    assert!(result.break_token.is_some());
}

#[test]
fn fragmentation_empty_when_no_space() {
    let frag = make_fake_line_fragment(3, 20.0, 200.0);

    // No space at all — offset equals fragmentainer size.
    let result = apply_inline_fragmentation(
        frag,
        lu(50.0),
        lu(50.0),
        0,
        1,
        1,
    );

    assert_eq!(result.children.len(), 0, "No lines should fit");
    assert!(result.break_token.is_some());
    match &result.break_token {
        Some(BreakToken::Inline(t)) => {
            assert_eq!(t.lines_consumed, 0);
        }
        _ => panic!("Expected Inline break token"),
    }
}

#[test]
fn fragmentation_single_line_fits() {
    let frag = make_fake_line_fragment(1, 20.0, 200.0);

    let result = apply_inline_fragmentation(
        frag,
        lu(50.0),
        LayoutUnit::zero(),
        0,
        2,
        2,
    );

    assert_eq!(result.children.len(), 1);
    assert!(result.break_token.is_none(), "Single line should fit, no break");
}

#[test]
fn fragmentation_single_line_fragmentainer() {
    // 5 lines × 20px. Fragmentainer fits exactly 1 line (20px).
    let frag = make_fake_line_fragment(5, 20.0, 200.0);

    let result = apply_inline_fragmentation(
        frag,
        lu(20.0),
        LayoutUnit::zero(),
        0,
        1,
        1,
    );

    assert_eq!(result.children.len(), 1);
    assert!(result.break_token.is_some());
    match &result.break_token {
        Some(BreakToken::Inline(t)) => {
            assert_eq!(t.lines_consumed, 1);
        }
        _ => panic!("Expected Inline break token"),
    }
}

// ── Orphans and widows tests ─────────────────────────────────────────────

#[test]
fn widows_steals_lines_from_current_fragmentainer() {
    // 5 lines × 20px = 100px. Fragmentainer is 80px → 4 lines fit.
    // With widows=2: remaining=1, which < 2. Need to steal 1 line.
    // So 3 lines in this fragmentainer, 2 in next.
    let frag = make_fake_line_fragment(5, 20.0, 200.0);

    let result = apply_inline_fragmentation(
        frag,
        lu(80.0),
        LayoutUnit::zero(),
        0,
        1, // orphans
        2, // widows
    );

    assert_eq!(result.children.len(), 3, "Should keep 3 lines (widows=2 needs 2 in next)");
    assert!(result.break_token.is_some());
}

#[test]
fn orphans_prevents_too_few_lines_before_break() {
    // 6 lines × 20px. Fragmentainer 80px → 4 fit.
    // orphans=3, widows=3: remaining=2 < 3, steal 1 → 3 in current.
    // But orphans=3 says keep at least 3, so 3 lines kept.
    let frag = make_fake_line_fragment(6, 20.0, 200.0);

    let result = apply_inline_fragmentation(
        frag,
        lu(80.0),
        LayoutUnit::zero(),
        0,
        3, // orphans
        3, // widows
    );

    assert_eq!(result.children.len(), 3, "Should keep 3 lines (orphans=3)");
}

#[test]
fn orphans_widows_conflict_orphans_wins() {
    // 4 lines × 20px. Fragmentainer 60px → 3 fit.
    // orphans=3, widows=3: remaining=1 < 3, want to steal 2 → 1 line.
    // But orphans=3 says keep at least 3, and we can fit 3 → keep 3.
    let frag = make_fake_line_fragment(4, 20.0, 200.0);

    let result = apply_inline_fragmentation(
        frag,
        lu(60.0),
        LayoutUnit::zero(),
        0,
        3, // orphans
        3, // widows
    );

    // Can't satisfy both (need 3+3=6 lines for only 4 lines).
    // Orphans=3 is satisfied since 3 physically fit.
    assert_eq!(result.children.len(), 3);
}

#[test]
fn default_orphans_widows() {
    // 3 lines × 20px. Fragmentainer 40px → 2 fit.
    // Default orphans=2, widows=2: remaining=1 < 2, steal 1 → 1 line.
    // But orphans=2, and only 1 line would remain → keep at least 1.
    // Actually: adjusted_fit = 2 - 1 = 1, then orphans check: 1 < 2 but
    // lines_that_fit=2 >= 2 → set to 2. Final: 2 lines.
    let frag = make_fake_line_fragment(3, 20.0, 200.0);

    let result = apply_inline_fragmentation(
        frag,
        lu(40.0),
        LayoutUnit::zero(),
        0,
        2, // orphans (default)
        2, // widows (default)
    );

    assert_eq!(result.children.len(), 2, "Orphans=2 should keep 2 lines");
}

// ── Resume from break token tests ────────────────────────────────────────

#[test]
fn resume_skips_consumed_lines() {
    let frag = make_fake_line_fragment(5, 20.0, 200.0);
    let break_token = InlineBreakToken::new(2, lu(40.0));

    let result = resume_inline_from_break_token(
        frag,
        &break_token,
        lu(2000.0), // large fragmentainer — all remaining fit
        1,
        1,
    );

    assert_eq!(result.children.len(), 3, "Should have 3 remaining lines");
    assert!(result.break_token.is_none(), "All remaining fit, no break");

    // First remaining line should be at top (offset 0).
    assert_eq!(result.children[0].offset.top, LayoutUnit::zero());
}

#[test]
fn resume_applies_further_fragmentation() {
    // 6 lines × 20px. First fragmentainer consumed 2 lines.
    // Next fragmentainer is 50px → fits 2 more lines.
    let frag = make_fake_line_fragment(6, 20.0, 200.0);
    let break_token = InlineBreakToken::new(2, lu(40.0));

    let result = resume_inline_from_break_token(
        frag,
        &break_token,
        lu(50.0),
        1,
        1,
    );

    assert_eq!(result.children.len(), 2, "Should fit 2 lines in 50px");
    assert!(result.break_token.is_some(), "Still more lines to lay out");
    match &result.break_token {
        Some(BreakToken::Inline(t)) => {
            assert_eq!(t.lines_consumed, 4, "2 consumed before + 2 now = 4");
        }
        _ => panic!("Expected Inline break token"),
    }
}

#[test]
fn resume_from_end_produces_empty() {
    let frag = make_fake_line_fragment(3, 20.0, 200.0);
    let break_token = InlineBreakToken::new(3, lu(60.0));

    let result = resume_inline_from_break_token(
        frag,
        &break_token,
        lu(100.0),
        1,
        1,
    );

    assert_eq!(result.children.len(), 0, "All lines consumed");
}

#[test]
fn resume_offsets_are_rebased_to_zero() {
    // 4 lines at offsets 0, 20, 40, 60. Skip 2, remaining at 40, 60.
    // After rebasing: 0, 20.
    let frag = make_fake_line_fragment(4, 20.0, 200.0);
    let break_token = InlineBreakToken::new(2, lu(40.0));

    let result = resume_inline_from_break_token(
        frag,
        &break_token,
        lu(2000.0),
        1,
        1,
    );

    assert_eq!(result.children.len(), 2);
    assert_eq!(result.children[0].offset.top, lu(0.0));
    assert_eq!(result.children[1].offset.top, lu(20.0));
}

// ── Fragment break_token field tests ─────────────────────────────────────

#[test]
fn fragment_break_token_default_none() {
    let frag = Fragment::new_box(openui_dom::NodeId::NONE, PhysicalSize::zero());
    assert!(frag.break_token.is_none());
}

#[test]
fn fragment_break_token_can_be_set() {
    let mut frag = Fragment::new_box(openui_dom::NodeId::NONE, PhysicalSize::zero());
    frag.break_token = Some(BreakToken::Inline(InlineBreakToken::new(1, lu(20.0))));
    assert!(frag.break_token.is_some());
}

// ── ComputedStyle orphans/widows tests ───────────────────────────────────

#[test]
fn computed_style_orphans_default() {
    let s = openui_style::ComputedStyle::initial();
    assert_eq!(s.orphans, 2, "CSS orphans initial value is 2");
}

#[test]
fn computed_style_widows_default() {
    let s = openui_style::ComputedStyle::initial();
    assert_eq!(s.widows, 2, "CSS widows initial value is 2");
}

#[test]
fn computed_style_orphans_widows_custom() {
    let mut s = openui_style::ComputedStyle::initial();
    s.orphans = 4;
    s.widows = 3;
    assert_eq!(s.orphans, 4);
    assert_eq!(s.widows, 3);
}

// ── Integration: full multi-fragmentainer simulation ─────────────────────

#[test]
fn multi_fragmentainer_simulation() {
    // Simulate laying out 6 lines across multiple fragmentainers.
    let frag = make_fake_line_fragment(6, 20.0, 200.0);
    let total = frag.children.len();
    assert_eq!(total, 6);

    // First fragmentainer: 50px → 2 lines.
    let first = apply_inline_fragmentation(
        clone_fragment(&frag),
        lu(50.0),
        LayoutUnit::zero(),
        0,
        1,
        1,
    );
    assert_eq!(first.children.len(), 2);
    let token1 = match &first.break_token {
        Some(BreakToken::Inline(t)) => t.clone(),
        _ => panic!("Expected break token"),
    };
    assert_eq!(token1.lines_consumed, 2);

    // Second fragmentainer: 50px → 2 more lines.
    let second = resume_inline_from_break_token(
        clone_fragment(&frag),
        &token1,
        lu(50.0),
        1,
        1,
    );
    assert_eq!(second.children.len(), 2);
    let token2 = match &second.break_token {
        Some(BreakToken::Inline(t)) => t.clone(),
        _ => panic!("Expected break token"),
    };
    assert_eq!(token2.lines_consumed, 4);

    // Third fragmentainer: 50px → 2 remaining lines (all fit).
    let third = resume_inline_from_break_token(
        clone_fragment(&frag),
        &token2,
        lu(50.0),
        1,
        1,
    );
    assert_eq!(third.children.len(), 2);
    assert!(third.break_token.is_none(), "All lines consumed");

    // Total lines across all fragmentainers = 6.
    let total_laid_out = first.children.len() + second.children.len() + third.children.len();
    assert_eq!(total_laid_out, 6);
}

#[test]
fn fragmentation_preserves_baselines() {
    let frag = make_fake_line_fragment(4, 20.0, 200.0);

    // Set baselines on the fake fragment.
    let mut frag_with_bl = frag;
    for (i, child) in frag_with_bl.children.iter_mut().enumerate() {
        child.baseline_offset = 15.0 + i as f32;
    }

    let result = apply_inline_fragmentation(
        frag_with_bl,
        lu(45.0),
        LayoutUnit::zero(),
        0,
        1,
        1,
    );

    assert_eq!(result.children.len(), 2);
    assert!(result.first_baseline.is_some());
    assert!(result.last_baseline.is_some());
    // First baseline = first child offset + baseline_offset
    let expected_first = lu(0.0) + LayoutUnit::from_f32(15.0);
    assert_eq!(result.first_baseline.unwrap(), expected_first);
}

#[test]
fn no_fragmentation_zero_fragmentainer_size() {
    let frag = make_fake_line_fragment(3, 20.0, 200.0);

    // fragmentainer_block_size = 0 → no fragmentation context.
    let result = apply_inline_fragmentation(
        frag,
        LayoutUnit::zero(),
        LayoutUnit::zero(),
        0,
        2,
        2,
    );

    assert_eq!(result.children.len(), 3, "No fragmentation when size is 0");
    assert!(result.break_token.is_none());
}

#[test]
fn fragmentation_empty_fragment() {
    let frag = Fragment::new_box(openui_dom::NodeId::NONE, PhysicalSize::new(lu(200.0), LayoutUnit::zero()));

    let result = apply_inline_fragmentation(
        frag,
        lu(50.0),
        LayoutUnit::zero(),
        0,
        2,
        2,
    );

    assert_eq!(result.children.len(), 0);
    assert!(result.break_token.is_none(), "Empty fragment needs no break token");
}

#[test]
fn height_adjusted_after_fragmentation() {
    let frag = make_fake_line_fragment(4, 25.0, 200.0);
    // Total height = 100px. Fragmentainer = 60px → 2 lines fit (50px).

    let result = apply_inline_fragmentation(
        frag,
        lu(60.0),
        LayoutUnit::zero(),
        0,
        1,
        1,
    );

    assert_eq!(result.children.len(), 2);
    // Height should be reduced to cover only the kept lines.
    assert_eq!(result.size.height, lu(50.0));
}

// ── Helper: create a fake fragment with N line boxes ─────────────────────

fn make_fake_line_fragment(num_lines: usize, line_height: f32, width: f32) -> Fragment {
    let total_height = num_lines as f32 * line_height;
    let mut frag = Fragment::new_box(
        openui_dom::NodeId::NONE,
        PhysicalSize::new(lu(width), lu(total_height)),
    );

    for i in 0..num_lines {
        let mut line = Fragment::new_box(
            openui_dom::NodeId::NONE,
            PhysicalSize::new(lu(width), lu(line_height)),
        );
        line.offset = openui_geometry::PhysicalOffset::new(
            LayoutUnit::zero(),
            lu(i as f32 * line_height),
        );
        line.baseline_offset = line_height * 0.75; // approximate
        frag.children.push(line);
    }

    frag.first_baseline = frag.children.first()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));
    frag.last_baseline = frag.children.last()
        .map(|f| f.offset.top + LayoutUnit::from_f32(f.baseline_offset));

    frag
}

/// Clone a fragment (deep copy for test isolation).
fn clone_fragment(frag: &Fragment) -> Fragment {
    let mut new_frag = Fragment::new_box(frag.node_id, frag.size);
    new_frag.offset = frag.offset;
    new_frag.first_baseline = frag.first_baseline;
    new_frag.last_baseline = frag.last_baseline;
    new_frag.baseline_offset = frag.baseline_offset;
    new_frag.children = frag.children.iter().map(|c| {
        let mut child = Fragment::new_box(c.node_id, c.size);
        child.offset = c.offset;
        child.baseline_offset = c.baseline_offset;
        child
    }).collect();
    new_frag
}
