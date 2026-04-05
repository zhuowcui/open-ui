//! SP12 H1 — Validation tests for the WPT test infrastructure helpers.
//!
//! Verifies that `BlockTestBuilder`, `StyleBuilder`, `LayoutTestResult`,
//! and the `assert_layout!` macro work correctly.

#[path = "sp12_wpt_helpers.rs"]
mod sp12_wpt_helpers;

use sp12_wpt_helpers::*;

use openui_geometry::Length;
use openui_style::*;

// ── 1. Builder creates container with correct size ──────────────────────

#[test]
fn builder_creates_container_with_correct_size() {
    let result = BlockTestBuilder::new(800, 600).build();
    result.assert_container_width(800);
    result.assert_container_height(600);
}

// ── 2. Add single child with fixed size ─────────────────────────────────

#[test]
fn single_child_with_fixed_size() {
    let mut builder = BlockTestBuilder::new(800, 600);
    builder.add_child().width(200.0).height(100.0).done();
    let result = builder.build();

    result.assert_child_count(1);
    result.assert_child_size(0, 200, 100);
    result.assert_child_position(0, 0, 0);
}

// ── 3. Add child with margins ───────────────────────────────────────────

#[test]
fn child_with_margins() {
    // Container needs a border to prevent parent-child margin collapsing.
    let mut builder = BlockTestBuilder::new(800, 600).with_container_style(|s| {
        s.border_top_width = 1;
        s.border_top_style = BorderStyle::Solid;
    });
    builder
        .add_child()
        .width(200.0)
        .height(100.0)
        .margin(10, 20, 30, 40)
        .done();
    let result = builder.build();

    result.assert_child_count(1);
    // child offset.left = margin-left (40)
    // child offset.top = border-top (1) + margin-top (10) = 11
    result.assert_child_position(0, 40, 11);
    result.assert_child_size(0, 200, 100);
}

// ── 4. Nested children ──────────────────────────────────────────────────

#[test]
fn nested_children() {
    let mut builder = BlockTestBuilder::new(800, 600);
    builder
        .add_child()
        .width(400.0)
        .height(200.0)
        .add_child()
        .width(200.0)
        .height(50.0)
        .done()
        .done();
    let result = builder.build();

    result.assert_child_count(1);
    result.assert_child_size(0, 400, 200);
    // Nested child within its parent
    result.assert_nested_child_size(0, 0, 200, 50);
    result.assert_nested_child_position(0, 0, 0, 0);
}

// ── 5. StyleBuilder sets width/height ───────────────────────────────────

#[test]
fn style_builder_sets_width_height() {
    let style = style_builder().width(300.0).height(150.0).build();
    assert_eq!(style.width, Length::px(300.0));
    assert_eq!(style.height, Length::px(150.0));
}

// ── 6. StyleBuilder sets float/clear ────────────────────────────────────

#[test]
fn style_builder_sets_float_clear() {
    let style = style_builder().float_left().clear_both().build();
    assert_eq!(style.float, Float::Left);
    assert_eq!(style.clear, Clear::Both);
}

// ── 7. StyleBuilder sets position ───────────────────────────────────────

#[test]
fn style_builder_sets_position() {
    let style_rel = style_builder().position_relative().build();
    assert_eq!(style_rel.position, Position::Relative);

    let style_abs = style_builder().position_absolute().build();
    assert_eq!(style_abs.position, Position::Absolute);
}

// ── 8. StyleBuilder sets overflow ───────────────────────────────────────

#[test]
fn style_builder_sets_overflow() {
    let style = style_builder().overflow_hidden().build();
    assert_eq!(style.overflow_x, Overflow::Hidden);
    assert_eq!(style.overflow_y, Overflow::Hidden);
}

// ── 9. StyleBuilder sets display ────────────────────────────────────────

#[test]
fn style_builder_sets_display() {
    let style = style_builder().display(Display::Flex).build();
    assert_eq!(style.display, Display::Flex);
}

// ── 10. Assert helpers work correctly ───────────────────────────────────

#[test]
fn assert_helpers_work_correctly() {
    let mut builder = BlockTestBuilder::new(400, 300);
    builder.add_child().width(100.0).height(50.0).done();
    let result = builder.build();

    // assert_child_margin_box covers both position and size
    result.assert_child_margin_box(0, 0, 0, 100, 50);
    assert_eq!(result.child_count(), 1);
    assert_eq!(result.container_size().width, lu(400));
    assert_eq!(result.container_size().height, lu(300));
}

// ── 11. Multiple children vertical stacking ─────────────────────────────

#[test]
fn multiple_children_positioning() {
    let mut builder = BlockTestBuilder::new(800, 600);
    builder.add_child().width(800.0).height(100.0).done();
    builder.add_child().width(800.0).height(150.0).done();
    builder.add_child().width(800.0).height(200.0).done();
    let result = builder.build();

    result.assert_child_count(3);
    // Block children stack vertically
    result.assert_child_position(0, 0, 0);
    result.assert_child_position(1, 0, 100);
    result.assert_child_position(2, 0, 250);
}

// ── 12. Layout result child access ──────────────────────────────────────

#[test]
fn layout_result_child_access() {
    let mut builder = BlockTestBuilder::new(600, 400);
    builder.add_child().width(200.0).height(80.0).done();
    builder.add_child().width(300.0).height(120.0).done();
    let result = builder.build();

    // child() accessor
    let c0 = result.child(0);
    assert_eq!(c0.size.width, lu(200));
    assert_eq!(c0.size.height, lu(80));

    // child_offset / child_size accessors
    let off = result.child_offset(1);
    assert_eq!(off.top, lu(80)); // stacked below child 0
    let sz = result.child_size(1);
    assert_eq!(sz.width, lu(300));
    assert_eq!(sz.height, lu(120));
}

// ── 13. assert_layout! macro ────────────────────────────────────────────

#[test]
fn assert_layout_macro_works() {
    let mut builder = BlockTestBuilder::new(800, 600);
    builder.add_child().width(200.0).height(100.0).done();
    builder.add_child().width(300.0).height(150.0).done();
    let result = builder.build();

    assert_layout!(result, child_count 2);
    assert_layout!(result, child(0) at (0, 0) size (200, 100));
    assert_layout!(result, child(1) at (0, 100));
    assert_layout!(result, child(1) size (300, 150));
    assert_layout!(result, container width 800);
    assert_layout!(result, container height 600);
}

// ── 14. Child with padding and border ───────────────────────────────────

#[test]
fn child_with_padding_and_border() {
    let mut builder = BlockTestBuilder::new(800, 600);
    builder
        .add_child()
        .width(200.0)
        .height(100.0)
        .padding(5, 10, 5, 10)
        .border(2, 2, 2, 2)
        .done();
    let result = builder.build();

    // Content size is 200x100, padding adds 20 horiz / 10 vert,
    // border adds 4 horiz / 4 vert → total 224 x 114
    result.assert_child_size(0, 224, 114);
}

// ── 15. StyleBuilder margin/padding/border ──────────────────────────────

#[test]
fn style_builder_sets_margin_padding_border() {
    let style = style_builder()
        .margin(10, 20, 30, 40)
        .padding(5, 6, 7, 8)
        .border_width(1, 2, 3, 4)
        .build();

    assert_eq!(style.margin_top, Length::px(10.0));
    assert_eq!(style.margin_right, Length::px(20.0));
    assert_eq!(style.margin_bottom, Length::px(30.0));
    assert_eq!(style.margin_left, Length::px(40.0));

    assert_eq!(style.padding_top, Length::px(5.0));
    assert_eq!(style.padding_right, Length::px(6.0));
    assert_eq!(style.padding_bottom, Length::px(7.0));
    assert_eq!(style.padding_left, Length::px(8.0));

    assert_eq!(style.border_top_width, 1);
    assert_eq!(style.border_right_width, 2);
    assert_eq!(style.border_bottom_width, 3);
    assert_eq!(style.border_left_width, 4);
    assert_eq!(style.border_top_style, BorderStyle::Solid);
}

// ── 16. StyleBuilder with_style closure ─────────────────────────────────

#[test]
fn style_builder_with_closure() {
    let style = style_builder()
        .with(|s| {
            s.z_index = Some(42);
            s.opacity = 0.5;
        })
        .build();

    assert_eq!(style.z_index, Some(42));
    assert!((style.opacity - 0.5).abs() < f32::EPSILON);
}

// ── 17. Container style override ────────────────────────────────────────

#[test]
fn container_style_override() {
    let custom_style = style_builder()
        .display(Display::Block)
        .width(500.0)
        .height(400.0)
        .overflow_hidden()
        .build();

    let result = BlockTestBuilder::new(500, 400)
        .container_style(custom_style)
        .build();

    result.assert_container_width(500);
    result.assert_container_height(400);
}

// ── 18. Empty container ─────────────────────────────────────────────────

#[test]
fn empty_container() {
    let result = BlockTestBuilder::new(800, 600).build();
    result.assert_child_count(0);
    result.assert_container_width(800);
    result.assert_container_height(600);
}

// ── 19. Child with float ────────────────────────────────────────────────

#[test]
fn child_with_float_left() {
    let mut builder = BlockTestBuilder::new(800, 600);
    builder
        .add_child()
        .width(200.0)
        .height(100.0)
        .float_left()
        .done();
    let result = builder.build();

    result.assert_child_count(1);
    result.assert_child_position(0, 0, 0);
    result.assert_child_size(0, 200, 100);
}

// ── 20. StyleBuilder box-sizing border-box ──────────────────────────────

#[test]
fn style_builder_box_sizing() {
    let style = style_builder().box_sizing_border_box().build();
    assert_eq!(style.box_sizing, BoxSizing::BorderBox);
}

// ── 21. StyleBuilder percentage dimensions ──────────────────────────────

#[test]
fn style_builder_percentage_dimensions() {
    let style = style_builder().width_pct(50.0).height_pct(25.0).build();
    assert_eq!(style.width, Length::percent(50.0));
    assert_eq!(style.height, Length::percent(25.0));
}

// ── 22. ChildBuilder with_style closure ─────────────────────────────────

#[test]
fn child_builder_with_style_closure() {
    let mut builder = BlockTestBuilder::new(800, 600);
    builder
        .add_child()
        .width(200.0)
        .height(100.0)
        .with_style(|s| {
            s.opacity = 0.8;
        })
        .done();
    let result = builder.build();

    // Layout still works correctly
    result.assert_child_count(1);
    result.assert_child_size(0, 200, 100);
}

// ── 23. with_container_style mutator ────────────────────────────────────

#[test]
fn with_container_style_mutator() {
    let result = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.padding_top = Length::px(10.0);
            s.padding_bottom = Length::px(10.0);
        })
        .build();

    result.assert_container_width(800);
    // Container has padding so total size includes padding
    result.assert_container_height(620);
}
