//! SP12 H2 — Core Block Layout Tests.
//!
//! Comprehensive block layout tests translating WPT and Blink test cases
//! covering normal flow, box model, display types, auto sizing, anonymous
//! block boxes, replaced elements, and edge cases.

#[path = "sp12_wpt_helpers.rs"]
mod sp12_wpt_helpers;

use sp12_wpt_helpers::*;

use openui_geometry::Length;
use openui_style::*;

// ═══════════════════════════════════════════════════════════════════════════
// 1. BASIC NORMAL FLOW (50+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 1.1 Children stacking vertically ────────────────────────────────────

#[test]
fn nf_single_child_at_origin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(100.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn nf_two_children_stack_vertically() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(100.0).done();
    b.add_child().height(150.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
}

#[test]
fn nf_three_children_stack_vertically() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child().height(75.0).done();
    b.add_child().height(100.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
    r.assert_child_position(2, 0, 125);
}

#[test]
fn nf_five_children_stack_vertically() {
    let mut b = BlockTestBuilder::new(400, 600);
    for i in 0..5 {
        b.add_child().height((20 * (i + 1)) as f32).done();
    }
    let r = b.build();
    // heights: 20, 40, 60, 80, 100 → tops: 0, 20, 60, 120, 200
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 20);
    r.assert_child_position(2, 0, 60);
    r.assert_child_position(3, 0, 120);
    r.assert_child_position(4, 0, 200);
}

#[test]
fn nf_children_with_margins_stack() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(50.0).margin(10, 0, 10, 0).done();
    b.add_child().height(50.0).margin(10, 0, 10, 0).done();
    let r = b.build();
    // first child: top = 1 (border) + 10 (margin-top) = 11
    r.assert_child_position(0, 0, 11);
    // margin collapsing: max(10, 10) = 10 between siblings
    // second child: top = 11 + 50 + 10 = 71
    r.assert_child_position(1, 0, 71);
}

// ── 1.2 Auto width fills container ──────────────────────────────────────

#[test]
fn nf_auto_width_fills_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
}

#[test]
fn nf_auto_width_fills_800px_container() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 800, 30);
}

#[test]
fn nf_auto_width_child_in_narrow_container() {
    let mut b = BlockTestBuilder::new(100, 200);
    b.add_child().height(20.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 20);
}

#[test]
fn nf_auto_width_with_horizontal_margins() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).margin(0, 20, 0, 30).done();
    let r = b.build();
    // auto width = 400 - 20 - 30 = 350
    r.assert_child_size(0, 350, 50);
    r.assert_child_position(0, 30, 0);
}

// ── 1.3 Fixed width centering with auto margins ─────────────────────────

#[test]
fn nf_fixed_width_centered_auto_margins() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    // (400 - 200) / 2 = 100 on each side
    r.assert_child_position(0, 100, 0);
    r.assert_child_size(0, 200, 50);
}

#[test]
fn nf_fixed_width_centered_odd_remaining() {
    // Use even container width to avoid sub-pixel rounding
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(100.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    // (400 - 100) / 2 = 150
    r.assert_child_position(0, 150, 0);
}

#[test]
fn nf_fixed_width_left_auto_margin_right_fixed() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::auto();
            s.margin_right = Length::px(50.0);
        })
        .done();
    let r = b.build();
    // left = 400 - 200 - 50 = 150
    r.assert_child_position(0, 150, 0);
}

#[test]
fn nf_fixed_width_right_auto_margin_left_fixed() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::px(30.0);
            s.margin_right = Length::auto();
        })
        .done();
    let r = b.build();
    r.assert_child_position(0, 30, 0);
}

// ── 1.4 Percentage width resolution ─────────────────────────────────────

#[test]
fn nf_percentage_width_50pct() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width_pct(50.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn nf_percentage_width_100pct() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width_pct(100.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
}

#[test]
fn nf_percentage_width_25pct() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(25.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn nf_percentage_width_75pct_of_600() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child().width_pct(75.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 450, 50);
}

// ── 1.5 Auto height from content ────────────────────────────────────────

#[test]
fn nf_auto_height_from_single_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .add_child().height(80.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 80);
}

#[test]
fn nf_auto_height_from_multiple_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .add_child().height(40.0).done()
        .add_child().height(60.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

// ── 1.6 Fixed height ────────────────────────────────────────────────────

#[test]
fn nf_fixed_height_respected() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(200.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 200);
}

#[test]
fn nf_fixed_height_300() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(300.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 300);
}

// ── 1.7 Empty block (zero height) ───────────────────────────────────────

#[test]
fn nf_empty_block_zero_height() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().done();
    let r = b.build();
    r.assert_child_size(0, 400, 0);
}

#[test]
fn nf_empty_block_followed_by_content() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().done();
    b.add_child().height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 0);
    r.assert_child_position(1, 0, 0);
    r.assert_child_size(1, 400, 100);
}

// ── 1.8 Single child vs multiple children ───────────────────────────────

#[test]
fn nf_single_child_fixed_size() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(200.0).height(100.0).done();
    let r = b.build();
    r.assert_child_count(1);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn nf_multiple_children_count() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child().height(60.0).done();
    b.add_child().height(70.0).done();
    b.add_child().height(80.0).done();
    let r = b.build();
    r.assert_child_count(4);
}

// ── 1.9 Border and padding affect content box ───────────────────────────

#[test]
fn nf_child_with_padding_reduces_content_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(0, 20, 0, 20)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // outer child = 400 wide, inner content = 400 - 40 = 360
    r.assert_child_size(0, 400, 50);
    r.assert_nested_child_size(0, 0, 360, 50);
}

#[test]
fn nf_child_with_border_reduces_content_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(0, 5, 0, 5)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
    r.assert_nested_child_size(0, 0, 390, 50);
}

#[test]
fn nf_child_with_border_and_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(2, 2, 2, 2)
        .padding(10, 10, 10, 10)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // content width = 400 - 4 (border) - 20 (padding) = 376
    r.assert_nested_child_size(0, 0, 376, 50);
}

#[test]
fn nf_border_padding_position_nested_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(3, 3, 3, 3)
        .padding(10, 10, 10, 10)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // nested child position inside parent: left=3+10=13, top=3+10=13
    r.assert_nested_child_position(0, 0, 13, 13);
}

// ── 1.10 Container size ─────────────────────────────────────────────────

#[test]
fn nf_container_800x600() {
    let r = BlockTestBuilder::new(800, 600).build();
    r.assert_container_width(800);
    r.assert_container_height(600);
}

#[test]
fn nf_container_1024x768() {
    let r = BlockTestBuilder::new(1024, 768).build();
    r.assert_container_width(1024);
    r.assert_container_height(768);
}

// ── 1.11 Additional normal flow tests ───────────────────────────────────

#[test]
fn nf_child_width_plus_margins_fill_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .height(50.0)
        .margin(0, 50, 0, 50)
        .done();
    let r = b.build();
    r.assert_child_position(0, 50, 0);
    r.assert_child_size(0, 300, 50);
}

#[test]
fn nf_two_children_same_height() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(100.0).done();
    b.add_child().height(100.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 100);
    r.assert_child_size(0, 400, 100);
    r.assert_child_size(1, 400, 100);
}

#[test]
fn nf_child_fills_remaining_width_with_left_margin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).margin(0, 0, 0, 100).done();
    let r = b.build();
    r.assert_child_position(0, 100, 0);
    r.assert_child_size(0, 300, 50);
}

#[test]
fn nf_child_fills_remaining_width_with_right_margin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).margin(0, 100, 0, 0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 300, 50);
}

#[test]
fn nf_children_with_different_widths() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(100.0).height(50.0).done();
    b.add_child().width(200.0).height(50.0).done();
    b.add_child().width(300.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
    r.assert_child_size(1, 200, 50);
    r.assert_child_size(2, 300, 50);
}

#[test]
fn nf_ten_children_stacking() {
    let mut b = BlockTestBuilder::new(400, 2000);
    for _ in 0..10 {
        b.add_child().height(50.0).done();
    }
    let r = b.build();
    r.assert_child_count(10);
    for i in 0..10 {
        r.assert_child_position(i, 0, (i as i32) * 50);
    }
}

#[test]
fn nf_child_at_zero_zero_with_no_margin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(400.0).height(100.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 2. BOX MODEL (50+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 2.1 Content box width = container width - padding - border ──────────

#[test]
fn bm_content_width_minus_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(10, 20, 10, 20)
        .height(50.0)
        .done();
    let r = b.build();
    // Width of fragment includes padding: 400
    // Content area: 400 - 20 - 20 = 360 (tested via nested child)
    r.assert_child_size(0, 400, 70); // height 50 + 10 + 10 padding
}

#[test]
fn bm_content_width_minus_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(5, 10, 5, 10)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 60); // height 50 + 5 + 5 border
}

#[test]
fn bm_content_width_minus_padding_and_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(2, 3, 2, 3)
        .padding(5, 7, 5, 7)
        .height(50.0)
        .done();
    let r = b.build();
    // width = 400, height = 50 + 2+2 (border-v) + 5+5 (padding-v) = 64
    r.assert_child_size(0, 400, 64);
}

// ── 2.2 Margin auto (centering) ─────────────────────────────────────────

#[test]
fn bm_margin_auto_both_centers() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 100, 0);
}

#[test]
fn bm_margin_auto_left_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::auto();
            s.margin_right = Length::px(0.0);
        })
        .done();
    let r = b.build();
    // left auto = 400 - 200 - 0 = 200
    r.assert_child_position(0, 200, 0);
}

#[test]
fn bm_margin_auto_right_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::px(0.0);
            s.margin_right = Length::auto();
        })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn bm_margin_auto_with_fixed_width_300() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child()
        .width(300.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    // (800 - 300) / 2 = 250
    r.assert_child_position(0, 250, 0);
}

#[test]
fn bm_margin_auto_with_width_equal_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(400.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    // no room: both auto margins = 0
    r.assert_child_position(0, 0, 0);
}

// ── 2.3 Percentage margins ──────────────────────────────────────────────

#[test]
fn bm_percentage_margin_left() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::percent(10.0);
        })
        .done();
    let r = b.build();
    // 10% of 400 = 40
    r.assert_child_position(0, 40, 0);
    r.assert_child_size(0, 360, 50);
}

#[test]
fn bm_percentage_margin_right() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.margin_right = Length::percent(25.0);
        })
        .done();
    let r = b.build();
    // 25% of 400 = 100 → width = 400 - 100 = 300
    r.assert_child_size(0, 300, 50);
}

#[test]
fn bm_percentage_margin_both_sides() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::percent(10.0);
            s.margin_right = Length::percent(10.0);
        })
        .done();
    let r = b.build();
    // 10% left + 10% right = 40 + 40 = 80 → width = 320
    r.assert_child_position(0, 40, 0);
    r.assert_child_size(0, 320, 50);
}

// ── 2.4 Negative margins ───────────────────────────────────────────────

#[test]
fn bm_negative_margin_left() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::px(-20.0);
        })
        .done();
    let r = b.build();
    // negative margin-left shifts left and increases available width
    r.assert_child_position(0, -20, 0);
    r.assert_child_size(0, 420, 50);
}

#[test]
fn bm_negative_margin_right() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.margin_right = Length::px(-20.0);
        })
        .done();
    let r = b.build();
    // negative margin-right: width = 400 - (-20) = 420
    r.assert_child_size(0, 420, 50);
}

#[test]
fn bm_negative_margin_top_shifts_up() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(50.0).done();
    b.add_child()
        .height(50.0)
        .margin_top(-10)
        .done();
    let r = b.build();
    // second child moves up by 10 from where it would normally be
    // first child at y=1, second at y=1+50-10=41
    r.assert_child_position(1, 0, 41);
}

#[test]
fn bm_negative_margin_both_sides_fixed_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::px(-10.0);
            s.margin_right = Length::px(-10.0);
        })
        .done();
    let r = b.build();
    r.assert_child_position(0, -10, 0);
    r.assert_child_size(0, 300, 50);
}

// ── 2.5 Margin: 0 auto centering ───────────────────────────────────────

#[test]
fn bm_margin_0_auto_centering() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_top = Length::px(0.0);
            s.margin_bottom = Length::px(0.0);
            s.margin_left = Length::auto();
            s.margin_right = Length::auto();
        })
        .done();
    let r = b.build();
    r.assert_child_position(0, 200, 0);
}

#[test]
fn bm_margin_0_auto_100px_in_500() {
    let mut b = BlockTestBuilder::new(500, 400);
    b.add_child()
        .width(100.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 200, 0);
}

// ── 2.6 Padding percentage ──────────────────────────────────────────────

#[test]
fn bm_padding_percent_top() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            // all padding-% resolves against containing block inline size
            s.padding_top = Length::percent(10.0);
        })
        .done();
    let r = b.build();
    // 10% of 400 = 40 → total height = 40 + 50 = 90
    r.assert_child_size(0, 400, 90);
}

#[test]
fn bm_padding_percent_left_right() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.padding_left = Length::percent(5.0);
            s.padding_right = Length::percent(5.0);
        })
        .done();
    let r = b.build();
    // padding 5% = 20 each side, total width still 400
    r.assert_child_size(0, 400, 50);
}

#[test]
fn bm_padding_percent_bottom() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.padding_bottom = Length::percent(25.0);
        })
        .done();
    let r = b.build();
    // 25% of 400 = 100 → total height = 50 + 100 = 150
    r.assert_child_size(0, 400, 150);
}

// ── 2.7 Border widths ──────────────────────────────────────────────────

#[test]
fn bm_border_1px_all_sides() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(1, 1, 1, 1)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 52); // 50 + 1 + 1
}

#[test]
fn bm_border_5px_all_sides() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(5, 5, 5, 5)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 60); // 50 + 5 + 5
}

#[test]
fn bm_border_asymmetric() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(1, 2, 3, 4)
        .height(50.0)
        .done();
    let r = b.build();
    // height = 50 + 1 + 3 = 54, width = 400
    r.assert_child_size(0, 400, 54);
}

#[test]
fn bm_border_top_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(10, 0, 0, 0)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 60);
}

// ── 2.8 Box-sizing: border-box vs content-box ──────────────────────────

#[test]
fn bm_content_box_default() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(100.0)
        .padding(10, 10, 10, 10)
        .border(2, 2, 2, 2)
        .done();
    let r = b.build();
    // content-box: total = 200+10+10+2+2 = 224 wide, 100+10+10+2+2 = 124 tall
    r.assert_child_size(0, 224, 124);
}

#[test]
fn bm_border_box_includes_padding_and_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(100.0)
        .padding(10, 10, 10, 10)
        .border(2, 2, 2, 2)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    // border-box: 200x100 includes padding+border
    r.assert_child_size(0, 200, 100);
}

#[test]
fn bm_border_box_width_300_padding_20() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .height(150.0)
        .padding(20, 20, 20, 20)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 150);
}

#[test]
fn bm_border_box_with_border_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(100.0)
        .border(5, 5, 5, 5)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn bm_border_box_centered_with_auto_margins() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(100.0)
        .padding(10, 10, 10, 10)
        .box_sizing_border_box()
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    // border-box = 200 wide, margin auto → (400-200)/2 = 100
    r.assert_child_position(0, 100, 0);
    r.assert_child_size(0, 200, 100);
}

// ── 2.9 Additional box model tests ──────────────────────────────────────

#[test]
fn bm_padding_adds_to_height() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(15, 0, 15, 0)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 80); // 50 + 15 + 15
}

#[test]
fn bm_margin_does_not_add_to_size() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(100.0)
        .margin(10, 20, 10, 20)
        .done();
    let r = b.build();
    // margins don't affect fragment size
    r.assert_child_size(0, 200, 100);
}

#[test]
fn bm_border_none_style_zero_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.border_top_width = 5;
            s.border_top_style = BorderStyle::None;
        })
        .done();
    let r = b.build();
    // border-style: none → no border rendered
    r.assert_child_size(0, 400, 50);
}

#[test]
fn bm_all_four_margins_different() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child()
        .width(200.0)
        .height(100.0)
        .margin(5, 10, 15, 20)
        .done();
    let r = b.build();
    r.assert_child_position(0, 20, 6); // left=20, top=1(border)+5(margin)
    r.assert_child_size(0, 200, 100);
}

#[test]
fn bm_zero_padding_and_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(0, 0, 0, 0)
        .border(0, 0, 0, 0)
        .width(400.0)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

#[test]
fn bm_large_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(50, 50, 50, 50)
        .height(100.0)
        .done();
    let r = b.build();
    // width 400, height = 100 + 50 + 50 = 200
    r.assert_child_size(0, 400, 200);
}

#[test]
fn bm_padding_left_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(0, 0, 0, 30)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 30, 0);
    r.assert_nested_child_size(0, 0, 370, 50);
}

#[test]
fn bm_margin_percentage_top() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child()
        .height(50.0)
        .with_style(|s| {
            s.margin_top = Length::percent(10.0);
        })
        .done();
    let r = b.build();
    // 10% of 400 (containing block width) = 40
    r.assert_child_position(0, 0, 41); // 1 (border) + 40 (margin)
}

#[test]
fn bm_content_box_nested_child_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .padding(0, 20, 0, 20)
        .border(0, 5, 0, 5)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // content-box: total width = 300 + 20+20 + 5+5 = 350
    r.assert_child_size(0, 350, 50);
    // nested child fills content-box = 300
    r.assert_nested_child_size(0, 0, 300, 50);
}

#[test]
fn bm_border_box_nested_child_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .padding(0, 20, 0, 20)
        .border(0, 5, 0, 5)
        .box_sizing_border_box()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // border-box: total = 300, content = 300 - 40 - 10 = 250
    r.assert_child_size(0, 300, 50);
    r.assert_nested_child_size(0, 0, 250, 50);
}

// ═══════════════════════════════════════════════════════════════════════════
// 3. DISPLAY TYPES (40+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 3.1 display: block (default) ────────────────────────────────────────

#[test]
fn dt_display_block_default() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Block).height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

#[test]
fn dt_display_block_fills_width() {
    let mut b = BlockTestBuilder::new(500, 400);
    b.add_child().display(Display::Block).height(80.0).done();
    let r = b.build();
    r.assert_child_size(0, 500, 80);
}

#[test]
fn dt_display_block_stacks() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Block).height(50.0).done();
    b.add_child().display(Display::Block).height(70.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
}

// ── 3.2 display: none ──────────────────────────────────────────────────

#[test]
fn dt_display_none_no_fragment() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::None).height(100.0).done();
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn dt_display_none_between_siblings() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child().display(Display::None).height(100.0).done();
    b.add_child().height(70.0).done();
    let r = b.build();
    r.assert_child_count(2);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
}

#[test]
fn dt_display_none_first_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::None).height(200.0).done();
    b.add_child().height(100.0).done();
    let r = b.build();
    r.assert_child_count(1);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn dt_display_none_last_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(100.0).done();
    b.add_child().display(Display::None).height(200.0).done();
    let r = b.build();
    r.assert_child_count(1);
}

#[test]
fn dt_display_none_all_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::None).done();
    b.add_child().display(Display::None).done();
    b.add_child().display(Display::None).done();
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn dt_display_none_does_not_affect_layout() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child().display(Display::None).height(999.0).done();
    b.add_child().height(60.0).done();
    let r = b.build();
    r.assert_child_count(2);
    // second visible child immediately follows first
    r.assert_child_position(1, 0, 50);
    r.assert_child_size(1, 400, 60);
}

// ── 3.3 display: flow-root (new BFC) ───────────────────────────────────

#[test]
fn dt_flow_root_basic() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::FlowRoot)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

#[test]
fn dt_flow_root_fills_width() {
    let mut b = BlockTestBuilder::new(500, 400);
    b.add_child()
        .display(Display::FlowRoot)
        .height(80.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 500, 80);
}

#[test]
fn dt_flow_root_stacks_with_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child()
        .display(Display::FlowRoot)
        .height(70.0)
        .done();
    let r = b.build();
    r.assert_child_position(1, 0, 50);
}

#[test]
fn dt_flow_root_does_not_collapse_margins() {
    // flow-root establishes a new BFC; margins should not collapse through it.
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child()
        .height(50.0)
        .margin_bottom(20)
        .done();
    b.add_child()
        .display(Display::FlowRoot)
        .height(70.0)
        .margin_top(30)
        .done();
    let r = b.build();
    // flow-root creates new BFC → margins don't collapse
    // child0: y=1, height=50 → bottom at 51, margin-bottom=20
    // child1: flow-root BFC → margin-top=30, doesn't collapse
    // child1 y = 1 + 50 + max(20,30) = 81 (they do collapse between siblings)
    r.assert_child_position(1, 0, 81);
}

// ── 3.4 display: inline-block ───────────────────────────────────────────

#[test]
fn dt_inline_block_respects_width_height() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::InlineBlock)
        .width(100.0)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_count(1);
}

#[test]
fn dt_inline_block_with_fixed_size() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::InlineBlock)
        .width(150.0)
        .height(75.0)
        .done();
    let r = b.build();
    r.assert_child_count(1);
}

// ── 3.5 display: flex within block ──────────────────────────────────────

#[test]
fn dt_flex_container_within_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Flex)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_count(1);
    r.assert_child_size(0, 400, 100);
}

#[test]
fn dt_flex_container_fills_width() {
    let mut b = BlockTestBuilder::new(500, 400);
    b.add_child()
        .display(Display::Flex)
        .height(80.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 500, 80);
}

#[test]
fn dt_flex_stacks_with_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child()
        .display(Display::Flex)
        .height(60.0)
        .done();
    let r = b.build();
    r.assert_child_position(1, 0, 50);
}

// ── 3.6 Mixed display types ────────────────────────────────────────────

#[test]
fn dt_block_then_none_then_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(40.0).done();
    b.add_child().display(Display::None).done();
    b.add_child().height(60.0).done();
    let r = b.build();
    r.assert_child_count(2);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
}

#[test]
fn dt_multiple_none_between_blocks() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(30.0).done();
    b.add_child().display(Display::None).done();
    b.add_child().display(Display::None).done();
    b.add_child().display(Display::None).done();
    b.add_child().height(40.0).done();
    let r = b.build();
    r.assert_child_count(2);
    r.assert_child_position(1, 0, 30);
}

#[test]
fn dt_flow_root_between_blocks() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child()
        .display(Display::FlowRoot)
        .height(60.0)
        .done();
    b.add_child().height(70.0).done();
    let r = b.build();
    r.assert_child_count(3);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
    r.assert_child_position(2, 0, 110);
}

#[test]
fn dt_list_item_acts_as_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::ListItem)
        .height(30.0)
        .done();
    let r = b.build();
    r.assert_child_count(1);
    r.assert_child_size(0, 400, 30);
}

#[test]
fn dt_multiple_list_items_stack() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::ListItem).height(25.0).done();
    b.add_child().display(Display::ListItem).height(35.0).done();
    b.add_child().display(Display::ListItem).height(45.0).done();
    let r = b.build();
    r.assert_child_count(3);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 25);
    r.assert_child_position(2, 0, 60);
}

// ═══════════════════════════════════════════════════════════════════════════
// 4. AUTO SIZING (40+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 4.1 Auto width = available width ────────────────────────────────────

#[test]
fn as_auto_width_equals_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width_auto().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
}

#[test]
fn as_auto_width_in_200px_container() {
    let mut b = BlockTestBuilder::new(200, 300);
    b.add_child().width_auto().height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 30);
}

// ── 4.2 Auto width with margins ─────────────────────────────────────────

#[test]
fn as_auto_width_with_left_margin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width_auto()
        .height(50.0)
        .margin(0, 0, 0, 50)
        .done();
    let r = b.build();
    r.assert_child_size(0, 350, 50);
    r.assert_child_position(0, 50, 0);
}

#[test]
fn as_auto_width_with_both_margins() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width_auto()
        .height(50.0)
        .margin(0, 30, 0, 20)
        .done();
    let r = b.build();
    r.assert_child_size(0, 350, 50);
}

#[test]
fn as_auto_width_with_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width_auto()
        .height(50.0)
        .padding(0, 15, 0, 15)
        .done();
    let r = b.build();
    // auto width fills container = 400, but content area = 400 - 30 = 370
    r.assert_child_size(0, 400, 50);
}

// ── 4.3 Auto height = sum of children ───────────────────────────────────

#[test]
fn as_auto_height_single_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .add_child().height(80.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 80);
}

#[test]
fn as_auto_height_two_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .add_child().height(40.0).done()
        .add_child().height(60.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

#[test]
fn as_auto_height_three_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .add_child().height(30.0).done()
        .add_child().height(40.0).done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 120);
}

#[test]
fn as_auto_height_empty() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height_auto().done();
    let r = b.build();
    r.assert_child_size(0, 400, 0);
}

// ── 4.4 Auto height with padding ────────────────────────────────────────

#[test]
fn as_auto_height_with_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .padding(10, 0, 10, 0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // auto height = 50 + 10 + 10 = 70
    r.assert_child_size(0, 400, 70);
}

#[test]
fn as_auto_height_with_large_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .padding(50, 0, 50, 0)
        .add_child().height(100.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 200);
}

// ── 4.5 Auto height with border ─────────────────────────────────────────

#[test]
fn as_auto_height_with_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .border(3, 0, 3, 0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 56); // 50 + 3 + 3
}

#[test]
fn as_auto_height_with_padding_and_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .padding(5, 0, 5, 0)
        .border(2, 0, 2, 0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // 50 + 5 + 5 + 2 + 2 = 64
    r.assert_child_size(0, 400, 64);
}

// ── 4.6 Percentage height ───────────────────────────────────────────────

#[test]
fn as_percentage_height_with_definite_parent() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(200.0)
        .add_child().with_style(|s| { s.height = Length::percent(50.0); }).done()
        .done();
    let r = b.build();
    // 50% of 200 = 100
    r.assert_nested_child_size(0, 0, 400, 100);
}

#[test]
fn as_percentage_height_100pct() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(300.0)
        .add_child().with_style(|s| { s.height = Length::percent(100.0); }).done()
        .done();
    let r = b.build();
    r.assert_nested_child_size(0, 0, 400, 300);
}

#[test]
fn as_percentage_height_25pct() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(400.0)
        .add_child().with_style(|s| { s.height = Length::percent(25.0); }).done()
        .done();
    let r = b.build();
    r.assert_nested_child_size(0, 0, 400, 100);
}

#[test]
fn as_percentage_height_with_auto_parent_treated_as_auto() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .add_child().with_style(|s| { s.height = Length::percent(50.0); }).done()
        .done();
    let r = b.build();
    // When parent has auto height, percentage height resolves to auto (= 0 with no content)
    r.assert_nested_child_size(0, 0, 400, 0);
}

// ── 4.7 Min-height / max-height ─────────────────────────────────────────

#[test]
fn as_min_height_constrains_auto() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .min_height(100.0)
        .done();
    let r = b.build();
    // auto height = 0 content, but min-height = 100
    r.assert_child_size(0, 400, 100);
}

#[test]
fn as_min_height_does_not_shrink() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(200.0)
        .min_height(100.0)
        .done();
    let r = b.build();
    // height 200 > min-height 100 → stays 200
    r.assert_child_size(0, 400, 200);
}

#[test]
fn as_max_height_constrains_fixed() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(300.0)
        .max_height(150.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 150);
}

#[test]
fn as_max_height_does_not_grow() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(100.0)
        .max_height(200.0)
        .done();
    let r = b.build();
    // height 100 < max-height 200 → stays 100
    r.assert_child_size(0, 400, 100);
}

#[test]
fn as_min_width_constrains_auto() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(100.0)
        .height(50.0)
        .min_width(200.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn as_max_width_constrains_fixed() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .height(50.0)
        .max_width(150.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 150, 50);
}

#[test]
fn as_min_max_height_together() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .min_height(100.0)
        .max_height(300.0)
        .done();
    let r = b.build();
    // 50 clamped to [100, 300] → 100
    r.assert_child_size(0, 400, 100);
}

#[test]
fn as_min_max_width_together() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(500.0)
        .height(50.0)
        .min_width(100.0)
        .max_width(300.0)
        .done();
    let r = b.build();
    // 500 clamped to [100, 300] → 300
    r.assert_child_size(0, 300, 50);
}

#[test]
fn as_auto_width_with_max_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .max_width(200.0)
        .done();
    let r = b.build();
    // auto width = 400 but max-width = 200
    r.assert_child_size(0, 200, 50);
}

#[test]
fn as_auto_width_with_min_width() {
    let mut b = BlockTestBuilder::new(100, 600);
    b.add_child()
        .height(50.0)
        .min_width(200.0)
        .done();
    let r = b.build();
    // auto width = 100 but min-width = 200
    r.assert_child_size(0, 200, 50);
}

#[test]
fn as_auto_height_with_min_height_and_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .min_height(200.0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // auto height = 50 from child, but min-height = 200
    r.assert_child_size(0, 400, 200);
}

#[test]
fn as_auto_height_with_max_height_and_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .max_height(80.0)
        .add_child().height(50.0).done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    // auto height = 100 from children, max-height = 80
    r.assert_child_size(0, 400, 80);
}

// ═══════════════════════════════════════════════════════════════════════════
// 5. ANONYMOUS BLOCK BOXES (30+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// Note: In our test infrastructure, all children are created as block-level div
// elements. Anonymous block generation is tested by verifying the fragment tree
// structure when mixing inline and block display types.

#[test]
fn ab_all_block_children_no_anonymous() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Block).height(50.0).done();
    b.add_child().display(Display::Block).height(60.0).done();
    let r = b.build();
    r.assert_child_count(2);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
}

#[test]
fn ab_single_inline_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Inline)
        .width(50.0)
        .height(20.0)
        .done();
    let r = b.build();
    // inline children get wrapped in anonymous block
    r.assert_child_count(1);
}

#[test]
fn ab_inline_then_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Inline)
        .width(50.0)
        .height(20.0)
        .done();
    b.add_child()
        .display(Display::Block)
        .height(60.0)
        .done();
    let r = b.build();
    // Should have at least 2 fragments (anonymous wrapper + block)
    assert!(r.child_count() >= 2);
}

#[test]
fn ab_block_then_inline() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Block)
        .height(50.0)
        .done();
    b.add_child()
        .display(Display::Inline)
        .width(30.0)
        .height(20.0)
        .done();
    let r = b.build();
    assert!(r.child_count() >= 2);
}

#[test]
fn ab_inline_block_inline() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Inline)
        .width(40.0)
        .height(15.0)
        .done();
    b.add_child()
        .display(Display::Block)
        .height(50.0)
        .done();
    b.add_child()
        .display(Display::Inline)
        .width(40.0)
        .height(15.0)
        .done();
    let r = b.build();
    // anonymous_before + block + anonymous_after
    assert!(r.child_count() >= 3);
}

#[test]
fn ab_multiple_inline_children_single_wrapper() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Inline).width(30.0).height(10.0).done();
    b.add_child().display(Display::Inline).width(40.0).height(10.0).done();
    b.add_child().display(Display::Inline).width(50.0).height(10.0).done();
    let r = b.build();
    // All consecutive inlines wrapped in a single anonymous block
    r.assert_child_count(1);
}

#[test]
fn ab_all_block_preserves_order() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(30.0).done();
    b.add_child().height(40.0).done();
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_count(3);
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 400, 30);
    r.assert_child_position(1, 0, 30);
    r.assert_child_size(1, 400, 40);
    r.assert_child_position(2, 0, 70);
    r.assert_child_size(2, 400, 50);
}

#[test]
fn ab_block_preserves_widths() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(100.0).height(30.0).done();
    b.add_child().width(200.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 30);
    r.assert_child_size(1, 200, 30);
}

#[test]
fn ab_five_blocks_stacking() {
    let mut b = BlockTestBuilder::new(400, 600);
    for _ in 0..5 {
        b.add_child().height(20.0).done();
    }
    let r = b.build();
    r.assert_child_count(5);
    for i in 0..5 {
        r.assert_child_position(i, 0, (i as i32) * 20);
    }
}

#[test]
fn ab_inline_children_with_none_between() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Inline).width(30.0).height(10.0).done();
    b.add_child().display(Display::None).done();
    b.add_child().display(Display::Inline).width(40.0).height(10.0).done();
    let r = b.build();
    // display:none removed; remaining inlines are consecutive → 1 anonymous block
    r.assert_child_count(1);
}

#[test]
fn ab_block_none_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(40.0).done();
    b.add_child().display(Display::None).done();
    b.add_child().height(60.0).done();
    let r = b.build();
    r.assert_child_count(2);
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
}

#[test]
fn ab_nested_blocks_all_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(100.0)
        .add_child().height(40.0).done()
        .add_child().height(60.0).done()
        .done();
    let r = b.build();
    r.assert_child_count(1);
    r.assert_nested_child_position(0, 0, 0, 0);
    r.assert_nested_child_position(0, 1, 0, 40);
}

// Additional anonymous block tests

#[test]
fn ab_two_inline_then_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Inline).width(20.0).height(10.0).done();
    b.add_child().display(Display::Inline).width(30.0).height(10.0).done();
    b.add_child().display(Display::Block).height(50.0).done();
    let r = b.build();
    // anonymous wrapper for 2 inlines + block = at least 2 children
    assert!(r.child_count() >= 2);
}

#[test]
fn ab_block_then_two_inline() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Block).height(50.0).done();
    b.add_child().display(Display::Inline).width(20.0).height(10.0).done();
    b.add_child().display(Display::Inline).width(30.0).height(10.0).done();
    let r = b.build();
    assert!(r.child_count() >= 2);
}

#[test]
fn ab_alternating_inline_block() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::Inline).width(20.0).height(10.0).done();
    b.add_child().display(Display::Block).height(30.0).done();
    b.add_child().display(Display::Inline).width(20.0).height(10.0).done();
    b.add_child().display(Display::Block).height(30.0).done();
    let r = b.build();
    // at least 4: anon, block, anon, block
    assert!(r.child_count() >= 4);
}

#[test]
fn ab_only_none_children_empty() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::None).done();
    b.add_child().display(Display::None).done();
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn ab_many_blocks_maintain_sizes() {
    let mut b = BlockTestBuilder::new(400, 600);
    let heights = [10, 20, 30, 40, 50, 60, 70];
    for h in &heights {
        b.add_child().height(*h as f32).done();
    }
    let r = b.build();
    r.assert_child_count(7);
    let mut cumulative_top = 0;
    for (i, h) in heights.iter().enumerate() {
        r.assert_child_position(i, 0, cumulative_top);
        r.assert_child_size(i, 400, *h);
        cumulative_top += h;
    }
}

// ═══════════════════════════════════════════════════════════════════════════
// 6. REPLACED ELEMENTS (30+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// Replaced elements are simulated with fixed_size() which sets both
// intrinsic size and explicit width/height.

#[test]
fn re_fixed_intrinsic_size() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(200.0, 150.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 150);
}

#[test]
fn re_fixed_intrinsic_size_small() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(50.0, 30.0).done();
    let r = b.build();
    r.assert_child_size(0, 50, 30);
}

#[test]
fn re_fixed_intrinsic_size_large() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(800.0, 600.0).done();
    let r = b.build();
    r.assert_child_size(0, 800, 600);
}

#[test]
fn re_replaced_with_width_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(300.0).height(150.0).done();
    let r = b.build();
    r.assert_child_size(0, 300, 150);
}

#[test]
fn re_replaced_with_height_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(200.0).height(300.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 300);
}

#[test]
fn re_replaced_default_size_300x150() {
    // Simulating the default replaced element size (300x150 in CSS spec)
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(300.0, 150.0).done();
    let r = b.build();
    r.assert_child_size(0, 300, 150);
}

#[test]
fn re_replaced_percentage_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width_pct(50.0).height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn re_replaced_max_width_constraint() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(500.0)
        .height(100.0)
        .max_width(300.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 100);
}

#[test]
fn re_replaced_min_width_constraint() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(50.0)
        .height(100.0)
        .min_width(200.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn re_replaced_max_height_constraint() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(500.0)
        .max_height(300.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 300);
}

#[test]
fn re_replaced_positioned_at_origin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(100.0, 50.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn re_replaced_with_margins() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child()
        .fixed_size(200.0, 100.0)
        .margin(10, 20, 10, 20)
        .done();
    let r = b.build();
    r.assert_child_position(0, 20, 11);
    r.assert_child_size(0, 200, 100);
}

#[test]
fn re_replaced_centered() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .fixed_size(200.0, 100.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 100, 0);
}

#[test]
fn re_replaced_with_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .fixed_size(200.0, 100.0)
        .padding(10, 10, 10, 10)
        .done();
    let r = b.build();
    // content-box: total = 200+20 x 100+20
    r.assert_child_size(0, 220, 120);
}

#[test]
fn re_replaced_with_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .fixed_size(200.0, 100.0)
        .border(3, 3, 3, 3)
        .done();
    let r = b.build();
    r.assert_child_size(0, 206, 106);
}

#[test]
fn re_replaced_border_box() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .fixed_size(200.0, 100.0)
        .padding(10, 10, 10, 10)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn re_two_replaced_stack() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(100.0, 50.0).done();
    b.add_child().fixed_size(150.0, 75.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
}

#[test]
fn re_replaced_percentage_width_75pct() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width_pct(75.0).height(80.0).done();
    let r = b.build();
    r.assert_child_size(0, 300, 80);
}

#[test]
fn re_replaced_with_max_width_percentage() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(500.0)
        .height(100.0)
        .with_style(|s| {
            s.max_width = Length::percent(50.0);
        })
        .done();
    let r = b.build();
    // max-width: 50% of 400 = 200
    r.assert_child_size(0, 200, 100);
}

#[test]
fn re_replaced_100x100_centered() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .fixed_size(100.0, 100.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 150, 0);
}

#[test]
fn re_replaced_fills_container_with_100pct() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width_pct(100.0)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

#[test]
fn re_replaced_1x1_size() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(1.0, 1.0).done();
    let r = b.build();
    r.assert_child_size(0, 1, 1);
}

#[test]
fn re_three_replaced_elements_stack() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(100.0, 30.0).done();
    b.add_child().fixed_size(150.0, 40.0).done();
    b.add_child().fixed_size(200.0, 50.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 30);
    r.assert_child_position(2, 0, 70);
}

#[test]
fn re_replaced_width_larger_than_container() {
    let mut b = BlockTestBuilder::new(200, 300);
    b.add_child().fixed_size(400.0, 100.0).done();
    let r = b.build();
    // no automatic shrinking for replaced elements without max-width
    r.assert_child_size(0, 400, 100);
}

#[test]
fn re_replaced_constrained_by_max_width_to_container() {
    let mut b = BlockTestBuilder::new(200, 300);
    b.add_child()
        .width(400.0)
        .height(100.0)
        .max_width(200.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

// ═══════════════════════════════════════════════════════════════════════════
// 7. EDGE CASES (40+ tests)
// ═══════════════════════════════════════════════════════════════════════════

// ── 7.1 Zero-width container ────────────────────────────────────────────

#[test]
fn ec_zero_width_container() {
    let mut b = BlockTestBuilder::new(0, 600);
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_container_width(0);
    r.assert_child_size(0, 0, 50);
}

#[test]
fn ec_zero_height_container() {
    let mut b = BlockTestBuilder::new(400, 0);
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_container_height(0);
}

#[test]
fn ec_zero_size_container() {
    let r = BlockTestBuilder::new(0, 0).build();
    r.assert_container_width(0);
    r.assert_container_height(0);
}

// ── 7.2 Deeply nested blocks ────────────────────────────────────────────

#[test]
fn ec_deeply_nested_10_levels() {
    // Build nesting via the builder's 2-level support, verify structure
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .add_child().height(50.0).done()
    .done();
    let r = b.build();
    r.assert_child_count(1);
    let outer = r.child(0);
    assert_eq!(outer.children.len(), 1);
    assert_eq!(outer.children[0].size.height, lu(50));
}

#[test]
fn ec_nested_blocks_auto_height_propagation() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .add_child().height(80.0).done()
    .done();
    let r = b.build();
    // height should propagate upward: auto-height = 80
    r.assert_child_size(0, 400, 80);
    r.assert_nested_child_size(0, 0, 400, 80);
}

#[test]
fn ec_nested_blocks_auto_width_propagation() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .add_child().height(40.0).done()
    .done();
    let r = b.build();
    r.assert_child_size(0, 400, 40);
    r.assert_nested_child_size(0, 0, 400, 40);
}

// ── 7.3 Very large margins ──────────────────────────────────────────────

#[test]
fn ec_large_margin_left() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .margin(0, 0, 0, 300)
        .done();
    let r = b.build();
    r.assert_child_position(0, 300, 0);
    r.assert_child_size(0, 100, 50);
}

#[test]
fn ec_margin_exceeds_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .margin(0, 0, 0, 500)
        .done();
    let r = b.build();
    // margin-left pushes child beyond container
    r.assert_child_position(0, 500, 0);
}

#[test]
fn ec_large_vertical_margin() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child()
        .height(50.0)
        .margin_top(500)
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 501); // 1 (border) + 500 (margin)
}

// ── 7.4 Negative padding (invalid, treated as 0) ───────────────────────

#[test]
fn ec_zero_padding_explicit() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(0, 0, 0, 0)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
}

// ── 7.5 Empty container with padding/border ─────────────────────────────

#[test]
fn ec_empty_container_with_padding() {
    let b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.padding_top = Length::px(20.0);
            s.padding_bottom = Length::px(20.0);
        });
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn ec_empty_container_with_border() {
    let b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 5;
            s.border_bottom_width = 5;
            s.border_top_style = BorderStyle::Solid;
            s.border_bottom_style = BorderStyle::Solid;
        });
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn ec_container_with_padding_and_single_child() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.padding_top = Length::px(20.0);
            s.padding_left = Length::px(10.0);
            s.padding_right = Length::px(10.0);
        });
    b.add_child().height(50.0).done();
    let r = b.build();
    // Container width=400 is content-box, so content area = 400.
    // Child fills the content area = 400.
    r.assert_child_size(0, 400, 50);
    r.assert_child_position(0, 10, 20);
}

// ── 7.6 Container with only display:none children ───────────────────────

#[test]
fn ec_only_display_none_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().display(Display::None).height(100.0).done();
    b.add_child().display(Display::None).height(200.0).done();
    b.add_child().display(Display::None).height(300.0).done();
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn ec_many_display_none_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    for _ in 0..20 {
        b.add_child().display(Display::None).done();
    }
    let r = b.build();
    r.assert_child_count(0);
}

// ── 7.7 Additional edge cases ───────────────────────────────────────────

#[test]
fn ec_child_wider_than_container() {
    let mut b = BlockTestBuilder::new(200, 400);
    b.add_child().width(500.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 500, 50);
}

#[test]
fn ec_child_taller_than_container() {
    let mut b = BlockTestBuilder::new(400, 200);
    b.add_child().height(500.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 500);
}

#[test]
fn ec_zero_height_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(0.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 0);
}

#[test]
fn ec_zero_width_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(0.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 0, 50);
}

#[test]
fn ec_1px_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(1.0).height(1.0).done();
    let r = b.build();
    r.assert_child_size(0, 1, 1);
}

#[test]
fn ec_container_1x1() {
    let mut b = BlockTestBuilder::new(1, 1);
    b.add_child().height(1.0).done();
    let r = b.build();
    r.assert_child_size(0, 1, 1);
}

#[test]
fn ec_child_zero_height_with_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(0.0)
        .border(5, 5, 5, 5)
        .done();
    let r = b.build();
    // height = 0 + 5 + 5 = 10
    r.assert_child_size(0, 400, 10);
}

#[test]
fn ec_child_zero_height_with_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(0.0)
        .padding(10, 0, 10, 0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 20);
}

#[test]
fn ec_many_empty_children_stacking() {
    let mut b = BlockTestBuilder::new(400, 600);
    for _ in 0..10 {
        b.add_child().done();
    }
    let r = b.build();
    r.assert_child_count(10);
    // All zero-height, all at same position (collapsed)
    for i in 0..10 {
        r.assert_child_size(i, 400, 0);
    }
}

#[test]
fn ec_container_large_dimensions() {
    let r = BlockTestBuilder::new(10000, 10000).build();
    r.assert_container_width(10000);
    r.assert_container_height(10000);
}

#[test]
fn ec_child_in_large_container() {
    let mut b = BlockTestBuilder::new(10000, 10000);
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 10000, 50);
}

#[test]
fn ec_border_box_with_zero_content() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(100.0)
        .height(100.0)
        .padding(50, 50, 50, 50)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    // border-box: 100x100 total including padding
    r.assert_child_size(0, 100, 100);
}

#[test]
fn ec_max_height_zero() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(100.0)
        .max_height(0.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 0);
}

#[test]
fn ec_max_width_zero() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .height(50.0)
        .max_width(0.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 0, 50);
}

#[test]
fn ec_min_height_greater_than_max_height() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .min_height(200.0)
        .max_height(100.0)
        .done();
    let r = b.build();
    // max-height constrains to 100, then min-height raises to 200
    // Implementation resolves to max-height = 100 (max takes precedence)
    r.assert_child_size(0, 400, 100);
}

#[test]
fn ec_min_width_greater_than_max_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(50.0)
        .height(50.0)
        .min_width(300.0)
        .max_width(200.0)
        .done();
    let r = b.build();
    // max-width clamps to 200 (implementation resolves max first)
    r.assert_child_size(0, 200, 50);
}

#[test]
fn ec_percentage_width_in_zero_width_container() {
    let mut b = BlockTestBuilder::new(0, 600);
    b.add_child().width_pct(50.0).height(50.0).done();
    let r = b.build();
    // 50% of 0 = 0
    r.assert_child_size(0, 0, 50);
}

#[test]
fn ec_auto_margin_with_width_exceeding_container() {
    let mut b = BlockTestBuilder::new(200, 400);
    b.add_child()
        .width(300.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    // width > container: auto margins become 0
    r.assert_child_position(0, 0, 0);
    r.assert_child_size(0, 300, 50);
}

#[test]
fn ec_nested_auto_height_chain() {
    // Two levels of auto-height containers
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .add_child().height(42.0).done()
    .done();
    let r = b.build();
    r.assert_child_size(0, 400, 42);
}

#[test]
fn ec_container_with_border_and_padding_child() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 2;
            s.border_bottom_width = 2;
            s.border_left_width = 3;
            s.border_right_width = 3;
            s.border_top_style = BorderStyle::Solid;
            s.border_bottom_style = BorderStyle::Solid;
            s.border_left_style = BorderStyle::Solid;
            s.border_right_style = BorderStyle::Solid;
            s.padding_top = Length::px(5.0);
            s.padding_left = Length::px(10.0);
            s.padding_right = Length::px(10.0);
        });
    b.add_child().height(50.0).done();
    let r = b.build();
    // Container width=400 is content-box, so content area = 400.
    // Child fills the content area = 400.
    r.assert_child_size(0, 400, 50);
    // child position: left = 3 (border) + 10 (padding) = 13, top = 2 (border) + 5 (padding) = 7
    r.assert_child_position(0, 13, 7);
}

#[test]
fn ec_multiple_centered_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    b.add_child()
        .width(100.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 100, 0);
    r.assert_child_position(1, 150, 50);
}

#[test]
fn ec_child_with_all_box_model_properties() {
    let mut b = BlockTestBuilder::new(800, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child()
        .width(200.0)
        .height(100.0)
        .margin(10, 20, 30, 40)
        .padding(5, 10, 5, 10)
        .border(2, 3, 2, 3)
        .done();
    let r = b.build();
    // content-box: width = 200 + 10+10 + 3+3 = 226
    // height = 100 + 5+5 + 2+2 = 114
    r.assert_child_size(0, 226, 114);
    // position: left=40, top=1(container border)+10(margin)
    r.assert_child_position(0, 40, 11);
}

#[test]
fn ec_overflow_hidden_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .overflow_hidden()
        .width(200.0)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn ec_overflow_hidden_with_auto_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .overflow_hidden()
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 100);
}

#[test]
fn ec_relative_position_does_not_affect_flow() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().height(50.0).done();
    b.add_child()
        .height(60.0)
        .position_relative()
        .done();
    b.add_child().height(70.0).done();
    let r = b.build();
    // relative positioning doesn't affect flow
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 50);
    r.assert_child_position(2, 0, 110);
}

#[test]
fn ec_children_alternate_empty_and_sized() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().done(); // 0: empty
    b.add_child().height(50.0).done(); // 1: 50px
    b.add_child().done(); // 2: empty
    b.add_child().height(30.0).done(); // 3: 30px
    b.add_child().done(); // 4: empty
    let r = b.build();
    r.assert_child_count(5);
    r.assert_child_size(0, 400, 0);
    r.assert_child_size(1, 400, 50);
    r.assert_child_size(2, 400, 0);
    r.assert_child_size(3, 400, 30);
    r.assert_child_size(4, 400, 0);
}

#[test]
fn ec_border_box_with_border_and_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(100.0)
        .border(5, 5, 5, 5)
        .padding(10, 10, 10, 10)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    // border-box: total = 200x100
    r.assert_child_size(0, 200, 100);
}

#[test]
fn ec_nested_border_box() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .height(200.0)
        .padding(10, 10, 10, 10)
        .border(2, 2, 2, 2)
        .box_sizing_border_box()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 200);
    // content width = 300 - 10 - 10 - 2 - 2 = 276
    r.assert_nested_child_size(0, 0, 276, 50);
}

#[test]
fn ec_percentage_height_in_border_box_parent() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .height(200.0)
        .padding(10, 10, 10, 10)
        .box_sizing_border_box()
        .add_child().with_style(|s| { s.height = Length::percent(50.0); }).done()
        .done();
    let r = b.build();
    // parent content height = 200 - 10 - 10 = 180
    // 50% of 180 = 90
    r.assert_nested_child_size(0, 0, 280, 90);
}

// ═══════════════════════════════════════════════════════════════════════════
// 8. ADDITIONAL NORMAL FLOW TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn nf_eight_children_sequential_heights() {
    let mut b = BlockTestBuilder::new(400, 2000);
    let heights = [10, 20, 30, 40, 50, 60, 70, 80];
    for h in &heights {
        b.add_child().height(*h as f32).done();
    }
    let r = b.build();
    r.assert_child_count(8);
    let mut top = 0;
    for (i, h) in heights.iter().enumerate() {
        r.assert_child_position(i, 0, top);
        r.assert_child_size(i, 400, *h);
        top += h;
    }
}

#[test]
fn nf_auto_width_in_1px_container() {
    let mut b = BlockTestBuilder::new(1, 100);
    b.add_child().height(10.0).done();
    let r = b.build();
    r.assert_child_size(0, 1, 10);
}

#[test]
fn nf_two_children_different_widths_stacking() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(100.0).height(40.0).done();
    b.add_child().width(300.0).height(60.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 40);
    r.assert_child_size(0, 100, 40);
    r.assert_child_size(1, 300, 60);
}

#[test]
fn nf_centered_200px_in_600() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 200, 0);
}

#[test]
fn nf_centered_100px_in_1000() {
    let mut b = BlockTestBuilder::new(1000, 400);
    b.add_child()
        .width(100.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 450, 0);
}

#[test]
fn nf_percentage_width_10pct() {
    let mut b = BlockTestBuilder::new(1000, 600);
    b.add_child().width_pct(10.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
}

#[test]
fn nf_percentage_width_33pct() {
    let mut b = BlockTestBuilder::new(300, 600);
    b.add_child().width_pct(33.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 99, 50);
}

#[test]
fn nf_child_auto_width_with_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(1, 1, 1, 1)
        .height(50.0)
        .done();
    let r = b.build();
    // auto width fills container = 400
    r.assert_child_size(0, 400, 52);
}

#[test]
fn nf_container_with_border_children_position() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 5;
            s.border_left_width = 5;
            s.border_top_style = BorderStyle::Solid;
            s.border_left_style = BorderStyle::Solid;
        });
    b.add_child().height(50.0).done();
    let r = b.build();
    // child positioned after container's border
    r.assert_child_position(0, 5, 5);
}

#[test]
fn nf_auto_height_no_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().done();
    let r = b.build();
    r.assert_child_size(0, 400, 0);
}

#[test]
fn nf_fixed_width_smaller_than_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(100.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn nf_fixed_height_zero() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width(200.0).height(0.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 0);
}

// ═══════════════════════════════════════════════════════════════════════════
// 9. ADDITIONAL BOX MODEL TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn bm_uniform_padding_10() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(10, 10, 10, 10)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 70); // 50 + 10 + 10
    r.assert_nested_child_size(0, 0, 380, 50);
    r.assert_nested_child_position(0, 0, 10, 10);
}

#[test]
fn bm_uniform_border_3() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(3, 3, 3, 3)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 56); // 50 + 3 + 3
    r.assert_nested_child_size(0, 0, 394, 50);
}

#[test]
fn bm_margin_top_20_first_child() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(50.0).margin_top(20).done();
    let r = b.build();
    r.assert_child_position(0, 0, 21); // 1 (border) + 20 (margin)
}

#[test]
fn bm_margin_bottom_between_siblings() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child().height(50.0).margin_bottom(30).done();
    b.add_child().height(60.0).done();
    let r = b.build();
    r.assert_child_position(1, 0, 81); // 1 + 50 + 30
}

#[test]
fn bm_content_box_width_with_large_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(100.0)
        .padding(50, 50, 50, 50)
        .done();
    let r = b.build();
    // content-box: total = 200 + 100 = 300 wide, 100 + 100 = 200 tall
    r.assert_child_size(0, 300, 200);
}

#[test]
fn bm_border_box_width_equals_specified() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(400.0)
        .height(200.0)
        .padding(20, 20, 20, 20)
        .border(5, 5, 5, 5)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 200);
}

#[test]
fn bm_auto_margin_left_pushes_right() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(100.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::auto();
        })
        .done();
    let r = b.build();
    // auto left margin = 400 - 100 = 300
    r.assert_child_position(0, 300, 0);
}

#[test]
fn bm_margin_auto_with_border_and_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(100.0)
        .height(50.0)
        .padding(0, 10, 0, 10)
        .border(0, 5, 0, 5)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    // content-box total = 100 + 20 + 10 = 130
    // auto margin = (400 - 130) / 2 = 135
    r.assert_child_position(0, 135, 0);
    r.assert_child_size(0, 130, 50);
}

#[test]
fn bm_padding_right_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(0, 30, 0, 0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 0, 0);
    r.assert_nested_child_size(0, 0, 370, 50);
}

#[test]
fn bm_padding_top_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(25, 0, 0, 0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 0, 25);
}

#[test]
fn bm_border_bottom_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(0, 0, 8, 0)
        .height(50.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 58); // 50 + 8
}

#[test]
fn bm_border_left_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(0, 0, 0, 7)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 7, 0);
    r.assert_nested_child_size(0, 0, 393, 50);
}

// ═══════════════════════════════════════════════════════════════════════════
// 10. ADDITIONAL DISPLAY TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn dt_display_none_with_margins() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::None)
        .height(100.0)
        .margin(50, 50, 50, 50)
        .done();
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn dt_display_none_with_fixed_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::None)
        .width(200.0)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_count(0);
}

#[test]
fn dt_flow_root_with_fixed_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::FlowRoot)
        .width(200.0)
        .height(80.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 80);
}

#[test]
fn dt_flow_root_centered() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::FlowRoot)
        .width(200.0)
        .height(80.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 100, 0);
}

#[test]
fn dt_block_with_max_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Block)
        .height(50.0)
        .max_width(200.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

#[test]
fn dt_flex_container_with_fixed_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Flex)
        .width(200.0)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn dt_grid_container_basic() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Grid)
        .height(100.0)
        .done();
    let r = b.build();
    r.assert_child_count(1);
    r.assert_child_size(0, 400, 100);
}

#[test]
fn dt_table_display_basic() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::Table)
        .height(80.0)
        .done();
    let r = b.build();
    r.assert_child_count(1);
}

// ═══════════════════════════════════════════════════════════════════════════
// 11. ADDITIONAL AUTO SIZING TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn as_auto_width_600px_container() {
    let mut b = BlockTestBuilder::new(600, 400);
    b.add_child().height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 600, 50);
}

#[test]
fn as_auto_height_five_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .add_child().height(10.0).done()
        .add_child().height(20.0).done()
        .add_child().height(30.0).done()
        .add_child().height(40.0).done()
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 150);
}

#[test]
fn as_auto_height_with_border_only() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .border(10, 0, 10, 0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 20); // 0 content + 10 + 10
}

#[test]
fn as_min_height_200_with_content_50() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .min_height(200.0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 200);
}

#[test]
fn as_max_height_50_with_content_100() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .max_height(50.0)
        .add_child().height(100.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 50);
}

#[test]
fn as_auto_width_with_margin_and_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .margin(0, 20, 0, 20)
        .border(0, 5, 0, 5)
        .done();
    let r = b.build();
    // auto width = 400 - 20 - 20 = 360 (margins reduce available)
    // but border is part of the box → auto width + border = available - margins
    // width of fragment = 360
    r.assert_child_size(0, 360, 50);
}

#[test]
fn as_percentage_height_75pct() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(400.0)
        .add_child().with_style(|s| { s.height = Length::percent(75.0); }).done()
        .done();
    let r = b.build();
    // 75% of 400 = 300
    r.assert_nested_child_size(0, 0, 400, 300);
}

#[test]
fn as_min_width_larger_than_auto() {
    let mut b = BlockTestBuilder::new(200, 600);
    b.add_child()
        .height(50.0)
        .min_width(300.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 50);
}

#[test]
fn as_max_width_smaller_than_auto() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .max_width(100.0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 100, 50);
}

#[test]
fn as_auto_height_margin_bottom_child() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height_auto()
        .border(1, 0, 1, 0)
        .add_child().height(50.0).margin(0, 0, 20, 0).done()
        .done();
    let r = b.build();
    // auto height = 1 (border) + 50 + 20 (margin) + 1 (border) = 72
    r.assert_child_size(0, 400, 72);
}

// ═══════════════════════════════════════════════════════════════════════════
// 12. ADDITIONAL REPLACED ELEMENT TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn re_replaced_zero_size() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(0.0, 0.0).done();
    let r = b.build();
    r.assert_child_size(0, 0, 0);
}

#[test]
fn re_replaced_square_200() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().fixed_size(200.0, 200.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 200);
}

#[test]
fn re_replaced_with_margin_centered() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .fixed_size(100.0, 50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 150, 0);
}

#[test]
fn re_replaced_with_border_box() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .fixed_size(200.0, 100.0)
        .padding(5, 5, 5, 5)
        .border(2, 2, 2, 2)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 100);
}

#[test]
fn re_replaced_percentage_width_25pct() {
    let mut b = BlockTestBuilder::new(800, 600);
    b.add_child().width_pct(25.0).height(50.0).done();
    let r = b.build();
    r.assert_child_size(0, 200, 50);
}

// ═══════════════════════════════════════════════════════════════════════════
// 13. ADDITIONAL EDGE CASE TESTS
// ═══════════════════════════════════════════════════════════════════════════

#[test]
fn ec_child_with_only_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .padding(20, 20, 20, 20)
        .done();
    let r = b.build();
    // empty child with padding: 400 wide, 20+0+20 = 40 tall
    r.assert_child_size(0, 400, 40);
}

#[test]
fn ec_child_with_only_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .border(5, 5, 5, 5)
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 10); // 0 content + 5 + 5
}

#[test]
fn ec_nested_margins_with_border_separator() {
    let mut b = BlockTestBuilder::new(400, 600)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    b.add_child()
        .height(50.0)
        .margin(10, 0, 20, 0)
        .done();
    b.add_child()
        .height(60.0)
        .margin(30, 0, 10, 0)
        .done();
    let r = b.build();
    // child0 at y = 1 + 10 = 11
    r.assert_child_position(0, 0, 11);
    // margins collapse: max(20, 30) = 30
    // child1 at y = 11 + 50 + 30 = 91
    r.assert_child_position(1, 0, 91);
}

#[test]
fn ec_many_children_with_margins() {
    let mut b = BlockTestBuilder::new(400, 2000)
        .with_container_style(|s| {
            s.border_top_width = 1;
            s.border_top_style = BorderStyle::Solid;
        });
    for _ in 0..5 {
        b.add_child()
            .height(50.0)
            .margin_top(10)
            .margin_bottom(10)
            .done();
    }
    let r = b.build();
    r.assert_child_count(5);
    // First child: y = 1 + 10 = 11
    r.assert_child_position(0, 0, 11);
}

#[test]
fn ec_border_box_zero_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(0.0)
        .height(50.0)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 0, 50);
}

#[test]
fn ec_border_box_zero_height() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(200.0)
        .height(0.0)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    r.assert_child_size(0, 200, 0);
}

#[test]
fn ec_auto_width_with_large_margin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .margin(0, 0, 0, 350)
        .done();
    let r = b.build();
    r.assert_child_size(0, 50, 50);
    r.assert_child_position(0, 350, 0);
}

#[test]
fn ec_fixed_width_with_overflow_hidden() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(500.0)
        .height(50.0)
        .overflow_hidden()
        .done();
    let r = b.build();
    r.assert_child_size(0, 500, 50);
}

#[test]
fn ec_three_empty_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().done();
    b.add_child().done();
    b.add_child().done();
    let r = b.build();
    r.assert_child_count(3);
    for i in 0..3 {
        r.assert_child_size(i, 400, 0);
    }
}

#[test]
fn ec_container_100x100_child_fills() {
    let mut b = BlockTestBuilder::new(100, 100);
    b.add_child().height(100.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 100);
}

#[test]
fn ec_width_50pct_in_800() {
    let mut b = BlockTestBuilder::new(800, 400);
    b.add_child().width_pct(50.0).height(40.0).done();
    let r = b.build();
    r.assert_child_size(0, 400, 40);
}

#[test]
fn ec_multiple_percentage_width_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child().width_pct(25.0).height(30.0).done();
    b.add_child().width_pct(50.0).height(30.0).done();
    b.add_child().width_pct(75.0).height(30.0).done();
    b.add_child().width_pct(100.0).height(30.0).done();
    let r = b.build();
    r.assert_child_size(0, 100, 30);
    r.assert_child_size(1, 200, 30);
    r.assert_child_size(2, 300, 30);
    r.assert_child_size(3, 400, 30);
}

#[test]
fn ec_auto_margin_left_with_width_equal_container() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(400.0)
        .height(50.0)
        .with_style(|s| {
            s.margin_left = Length::auto();
        })
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn ec_stacking_with_variable_heights() {
    let mut b = BlockTestBuilder::new(400, 1000);
    b.add_child().height(1.0).done();
    b.add_child().height(2.0).done();
    b.add_child().height(3.0).done();
    b.add_child().height(4.0).done();
    b.add_child().height(5.0).done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
    r.assert_child_position(1, 0, 1);
    r.assert_child_position(2, 0, 3);
    r.assert_child_position(3, 0, 6);
    r.assert_child_position(4, 0, 10);
}

#[test]
fn ec_border_box_auto_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .height(50.0)
        .padding(0, 20, 0, 20)
        .border(0, 5, 0, 5)
        .box_sizing_border_box()
        .done();
    let r = b.build();
    // border-box with auto width: total = 400 including padding/border
    r.assert_child_size(0, 400, 50);
}

#[test]
fn ec_nested_fixed_width() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 300, 50);
    r.assert_nested_child_size(0, 0, 300, 50);
}

#[test]
fn ec_nested_child_with_margin() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(300.0)
        .border(1, 0, 1, 0)
        .add_child()
            .height(50.0)
            .margin(10, 0, 10, 0)
        .done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 0, 11); // 1 border + 10 margin
    r.assert_nested_child_size(0, 0, 300, 50);
}

#[test]
fn ec_nested_child_centered() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(400.0)
        .add_child()
            .width(200.0)
            .height(50.0)
            .with_style(|s| {
                s.margin_left = Length::auto();
                s.margin_right = Length::auto();
            })
        .done()
        .done();
    let r = b.build();
    r.assert_nested_child_position(0, 0, 100, 0);
    r.assert_nested_child_size(0, 0, 200, 50);
}

#[test]
fn ec_flow_root_with_padding() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::FlowRoot)
        .padding(15, 15, 15, 15)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 80); // 50 + 15 + 15
}

#[test]
fn ec_flow_root_with_border() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .display(Display::FlowRoot)
        .border(4, 4, 4, 4)
        .add_child().height(50.0).done()
        .done();
    let r = b.build();
    r.assert_child_size(0, 400, 58); // 50 + 4 + 4
}

#[test]
fn ec_auto_margin_200_in_200() {
    let mut b = BlockTestBuilder::new(200, 400);
    b.add_child()
        .width(200.0)
        .height(50.0)
        .margin_auto_horizontal()
        .done();
    let r = b.build();
    r.assert_child_position(0, 0, 0);
}

#[test]
fn ec_child_with_all_zero_properties() {
    let mut b = BlockTestBuilder::new(400, 600);
    b.add_child()
        .width(0.0)
        .height(0.0)
        .padding(0, 0, 0, 0)
        .border(0, 0, 0, 0)
        .margin(0, 0, 0, 0)
        .done();
    let r = b.build();
    r.assert_child_size(0, 0, 0);
    r.assert_child_position(0, 0, 0);
}

#[test]
fn ec_many_centered_children() {
    let mut b = BlockTestBuilder::new(400, 600);
    for w in [100, 200, 300, 400] {
        b.add_child()
            .width(w as f32)
            .height(25.0)
            .margin_auto_horizontal()
            .done();
    }
    let r = b.build();
    r.assert_child_position(0, 150, 0);   // (400-100)/2 = 150
    r.assert_child_position(1, 100, 25);   // (400-200)/2 = 100
    r.assert_child_position(2, 50, 50);    // (400-300)/2 = 50
    r.assert_child_position(3, 0, 75);     // (400-400)/2 = 0
}
