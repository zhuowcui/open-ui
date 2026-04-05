//! SP12 D3 — Sticky positioning integration tests.
//!
//! These tests exercise `compute_sticky_offset`, `apply_sticky_offset`,
//! `compute_sticky_constraint_rect`, and `StickyPositionData` per
//! CSS Positioned Layout Module Level 3 §3.

use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalRect, PhysicalSize};
use openui_layout::sticky::{
    apply_sticky_offset, compute_sticky_constraint_rect, compute_sticky_offset,
    StickyConstraintRect, StickyPositionData,
};
use openui_style::{ComputedStyle, Display, Position};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn lu(v: i32) -> LayoutUnit {
    LayoutUnit::from_i32(v)
}

fn offset(left: i32, top: i32) -> PhysicalOffset {
    PhysicalOffset::new(lu(left), lu(top))
}

fn size(w: i32, h: i32) -> PhysicalSize {
    PhysicalSize::new(lu(w), lu(h))
}

fn rect(x: i32, y: i32, w: i32, h: i32) -> PhysicalRect {
    PhysicalRect::from_xywh(lu(x), lu(y), lu(w), lu(h))
}

/// Standard viewport: 800×600 at origin.
fn viewport() -> PhysicalRect {
    rect(0, 0, 800, 600)
}

/// Large containing block: [0, 0] → [800, 2000].
fn large_cb() -> PhysicalRect {
    rect(0, 0, 800, 2000)
}

fn make_sticky_style(
    top: Length,
    right: Length,
    bottom: Length,
    left: Length,
) -> ComputedStyle {
    let mut s = ComputedStyle::initial();
    s.display = Display::Block;
    s.position = Position::Sticky;
    s.top = top;
    s.right = right;
    s.bottom = bottom;
    s.left = left;
    s
}

fn make_fragment(left: i32, top: i32, w: i32, h: i32) -> openui_layout::Fragment {
    let mut f = openui_layout::Fragment::new_box(
        openui_dom::NodeId::NONE,
        size(w, h),
    );
    f.offset = offset(left, top);
    f
}

fn insets(
    top: Option<i32>,
    right: Option<i32>,
    bottom: Option<i32>,
    left: Option<i32>,
) -> StickyConstraintRect {
    StickyConstraintRect {
        top: top.map(lu),
        right: right.map(lu),
        bottom: bottom.map(lu),
        left: left.map(lu),
    }
}

// ---------------------------------------------------------------------------
// 1. Sticky top — no scroll (stays in normal position)
// ---------------------------------------------------------------------------

#[test]
fn sticky_top_no_scroll() {
    // Element at y=200, scroll=0, sticky top=10. Element is already below
    // the sticky threshold → no shift needed.
    let off = compute_sticky_offset(
        offset(0, 200),
        offset(0, 0),
        viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off.top, lu(0));
    assert_eq!(off.left, lu(0));
}

// ---------------------------------------------------------------------------
// 2. Sticky top — scroll past threshold (sticks at top)
// ---------------------------------------------------------------------------

#[test]
fn sticky_top_scroll_past_threshold() {
    // Element at y=200, scroll=300, sticky top=10.
    // el_in_vp = 200 - 300 = -100.
    // start_stick = (0+10) - (-100) = 110. max(0,110) = 110.
    // CB clamp: max_positive = (2000-50) - 200 = 1750 → no clamping.
    let off = compute_sticky_offset(
        offset(0, 200),
        offset(0, 300),
        viewport(),
        &insets(Some(10), None, None, None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off.top, lu(110));
    assert_eq!(off.left, lu(0));
}

// ---------------------------------------------------------------------------
// 3. Sticky top — scroll further (stops at containing block bottom)
// ---------------------------------------------------------------------------

#[test]
fn sticky_top_clamped_by_containing_block() {
    // Element at y=200, element height=50, CB=[0..300].
    // max_positive = (300-50) - 200 = 50.
    // Scroll=500, inset top=0.
    // el_in_vp = 200 - 500 = -300.  start_stick = 0 - (-300) = 300.
    // raw = max(0,300) = 300. clamp(300, -200, 50) → 50.
    let off = compute_sticky_offset(
        offset(0, 200),
        offset(0, 500),
        viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50),
        rect(0, 0, 800, 300),
    );
    assert_eq!(off.top, lu(50));
}

// ---------------------------------------------------------------------------
// 4. Sticky bottom (sticks at bottom edge)
// ---------------------------------------------------------------------------

#[test]
fn sticky_bottom() {
    // Element at y=1500, height=50, viewport 600, scroll=800, inset bottom=20.
    // el_in_vp = 1500 - 800 = 700.
    // end_stick = (0+600-20) - (700+50) = 580 - 750 = -170.
    // min(0, -170) = -170.
    // CB clamp: max_negative = 1500 - 0 = 1500 → no clamp.
    let off = compute_sticky_offset(
        offset(0, 1500),
        offset(0, 800),
        viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off.top, lu(-170));
    assert_eq!(off.left, lu(0));
}

// ---------------------------------------------------------------------------
// 5. Sticky left (sticks at left edge)
// ---------------------------------------------------------------------------

#[test]
fn sticky_left() {
    // Element at x=300, scroll_x=400, viewport 800, inset left=15.
    // el_in_vp = 300 - 400 = -100.
    // start_stick = (0+15) - (-100) = 115. max(0,115)=115.
    let off = compute_sticky_offset(
        offset(300, 0),
        offset(400, 0),
        viewport(),
        &insets(None, None, None, Some(15)),
        size(80, 50),
        rect(0, 0, 2000, 600),
    );
    assert_eq!(off.left, lu(115));
    assert_eq!(off.top, lu(0));
}

// ---------------------------------------------------------------------------
// 6. Sticky right (sticks at right edge)
// ---------------------------------------------------------------------------

#[test]
fn sticky_right() {
    // Element at x=1200, width=80, scroll_x=500, viewport 800, inset right=10.
    // el_in_vp = 1200 - 500 = 700.
    // end_stick = (0+800-10) - (700+80) = 790-780 = 10.
    // min(0, 10) = 0 → element still inside viewport, no sticking.
    let off = compute_sticky_offset(
        offset(1200, 0),
        offset(500, 0),
        viewport(),
        &insets(None, Some(10), None, None),
        size(80, 50),
        rect(0, 0, 3000, 600),
    );
    assert_eq!(off.left, lu(0));

    // Now scroll further so element goes past viewport right:
    // scroll_x=700, el_in_vp = 1200 - 700 = 500.
    // end_stick = 790 - (500+80) = 790-580 = 210. min(0,210) = 0 → still fine.
    // scroll_x=1000, el_in_vp = 200. end_stick = 790 - 280 = 510. min(0,510)=0.
    // We need the element to be past the right edge:
    // scroll_x=200, el_in_vp = 1000. end_stick = 790 - 1080 = -290. min(0,-290)=-290.
    let off2 = compute_sticky_offset(
        offset(1200, 0),
        offset(200, 0),
        viewport(),
        &insets(None, Some(10), None, None),
        size(80, 50),
        rect(0, 0, 3000, 600),
    );
    assert_eq!(off2.left, lu(-290));
}

// ---------------------------------------------------------------------------
// 7. Sticky top+bottom (dual insets)
// ---------------------------------------------------------------------------

#[test]
fn sticky_top_and_bottom() {
    // Both top=10 and bottom=10, element height=50.
    // Element at y=200, scroll=300.
    // start_stick = (0+10) - (200-300) = 10 - (-100) = 110.
    // end_stick = (0+600-10) - (-100+50) = 590 - (-50) = 640.
    // raw = max(110, min(0, 640)) = max(110, 0) = 110.
    let off = compute_sticky_offset(
        offset(0, 200),
        offset(0, 300),
        viewport(),
        &insets(Some(10), None, Some(10), None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off.top, lu(110));
}

// ---------------------------------------------------------------------------
// 8. Sticky with margin (margin is part of normal_flow_offset)
// ---------------------------------------------------------------------------

#[test]
fn sticky_with_margin() {
    // Margin pushes the element further down. The normal_flow_offset includes
    // the margin, so sticky just works on the resulting position.
    // Element at y=220 (200 content + 20 margin), scroll=300, sticky top=0.
    // el_in_vp = 220 - 300 = -80. start_stick = 0 - (-80) = 80.
    let off = compute_sticky_offset(
        offset(0, 220),
        offset(0, 300),
        viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off.top, lu(80));
}

// ---------------------------------------------------------------------------
// 9. No sticky offset when position: static
// ---------------------------------------------------------------------------

#[test]
fn no_sticky_offset_when_static() {
    let style = ComputedStyle::initial(); // position: static
    let mut frag = make_fragment(10, 200, 100, 50);
    let original_offset = frag.offset;

    apply_sticky_offset(
        &mut frag,
        &style,
        offset(0, 300),
        viewport(),
        lu(800),
        lu(2000),
        large_cb(),
    );

    assert_eq!(frag.offset, original_offset);
}

// ---------------------------------------------------------------------------
// 10. Sticky constraint within containing block
// ---------------------------------------------------------------------------

#[test]
fn sticky_constrained_to_containing_block() {
    // Small CB [0..250], element at y=200, height=50, scroll=500, top=0.
    // max_positive = (250 - 50) - 200 = 0.
    // start_stick = 0 - (200-500) = 300. raw = max(0,300)=300.
    // clamp(300, -200, 0) → 0.
    let off = compute_sticky_offset(
        offset(0, 200),
        offset(0, 500),
        viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50),
        rect(0, 0, 800, 250),
    );
    assert_eq!(off.top, lu(0));
}

// ---------------------------------------------------------------------------
// 11. Nested sticky containers (inner CB)
// ---------------------------------------------------------------------------

#[test]
fn nested_sticky_containers() {
    // Outer CB is [100..500]. Element at y=150, scroll=200, sticky top=5.
    // el_in_vp = 150 - 200 = -50. start_stick = (0+5)-(-50) = 55.
    // max_positive = (500-40) - 150 = 310. max_negative = 150 - 100 = 50.
    // clamp(55, -50, 310) → 55.
    let off = compute_sticky_offset(
        offset(0, 150),
        offset(0, 200),
        viewport(),
        &insets(Some(5), None, None, None),
        size(100, 40),
        rect(0, 100, 800, 400),
    );
    assert_eq!(off.top, lu(55));
}

// ---------------------------------------------------------------------------
// 12. Sticky with percentage insets
// ---------------------------------------------------------------------------

#[test]
fn sticky_percentage_insets() {
    // 10% of CB block size 2000 = 200.
    let style = make_sticky_style(
        Length::percent(10.0),
        Length::auto(),
        Length::auto(),
        Length::auto(),
    );
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(2000));
    assert_eq!(cr.top, Some(lu(200)));
    assert_eq!(cr.bottom, None);
    assert_eq!(cr.left, None);
    assert_eq!(cr.right, None);

    // Now use the resolved inset to compute the offset:
    // Element at y=100, scroll=400, inset_top=200.
    // el_in_vp = 100-400 = -300. start_stick = 200-(-300) = 500.
    let off = compute_sticky_offset(
        offset(0, 100),
        offset(0, 400),
        viewport(),
        &cr,
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off.top, lu(500));
}

// ---------------------------------------------------------------------------
// 13. Sticky at scroll=0 (no effect)
// ---------------------------------------------------------------------------

#[test]
fn sticky_at_scroll_zero() {
    // Element at y=100, scroll=0, inset top=0.
    // el_in_vp = 100. start_stick = 0 - 100 = -100. max(0,-100) = 0.
    let off = compute_sticky_offset(
        offset(0, 100),
        offset(0, 0),
        viewport(),
        &insets(Some(0), None, None, None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off.top, lu(0));
    assert_eq!(off.left, lu(0));
}

// ---------------------------------------------------------------------------
// 14. apply_sticky_offset mutates fragment correctly
// ---------------------------------------------------------------------------

#[test]
fn apply_sticky_offset_mutates_fragment() {
    let style = make_sticky_style(
        Length::px(10.0),
        Length::auto(),
        Length::auto(),
        Length::auto(),
    );
    let mut frag = make_fragment(0, 200, 100, 50);

    apply_sticky_offset(
        &mut frag,
        &style,
        offset(0, 300),  // scroll
        viewport(),
        lu(800),
        lu(2000),
        large_cb(),
    );

    // start_stick = (0+10) - (200-300) = 110. max(0,110)=110.
    // fragment.offset.top = 200 + 110 = 310.
    assert_eq!(frag.offset.top, lu(310));
    assert_eq!(frag.offset.left, lu(0));
}

// ---------------------------------------------------------------------------
// 15. StickyPositionData roundtrip
// ---------------------------------------------------------------------------

#[test]
fn sticky_position_data_stores_all_fields() {
    let data = StickyPositionData {
        normal_flow_offset: offset(10, 20),
        insets: insets(Some(5), None, Some(10), None),
        element_size: size(200, 100),
        containing_block_rect: rect(0, 0, 800, 2000),
    };

    assert_eq!(data.normal_flow_offset.left, lu(10));
    assert_eq!(data.normal_flow_offset.top, lu(20));
    assert_eq!(data.insets.top, Some(lu(5)));
    assert_eq!(data.insets.bottom, Some(lu(10)));
    assert_eq!(data.insets.right, None);
    assert_eq!(data.element_size.width, lu(200));
    assert_eq!(data.containing_block_rect.height(), lu(2000));
}

// ---------------------------------------------------------------------------
// 16. compute_sticky_constraint_rect resolves all four edges
// ---------------------------------------------------------------------------

#[test]
fn constraint_rect_all_edges() {
    let style = make_sticky_style(
        Length::px(10.0),
        Length::px(20.0),
        Length::px(30.0),
        Length::px(40.0),
    );
    let cr = compute_sticky_constraint_rect(&style, lu(800), lu(600));
    assert_eq!(cr.top, Some(lu(10)));
    assert_eq!(cr.right, Some(lu(20)));
    assert_eq!(cr.bottom, Some(lu(30)));
    assert_eq!(cr.left, Some(lu(40)));
}

// ---------------------------------------------------------------------------
// 17. Sticky bottom sticks when scrolling up
// ---------------------------------------------------------------------------

#[test]
fn sticky_bottom_sticks_scrolling_up() {
    // Element at y=100, height=50, viewport 600, scroll=0, inset bottom=20.
    // el_in_vp = 100. end_stick = (600-20)-(100+50)=580-150=430. min(0,430)=0 → no stick.
    let off_no = compute_sticky_offset(
        offset(0, 100),
        offset(0, 0),
        viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off_no.top, lu(0));

    // Now the element is "above" the viewport bottom with element below fold:
    // Element at y=1800, scroll=1100.
    // el_in_vp = 1800-1100 = 700. end_stick = 580 - 750 = -170. min(0,-170)=-170.
    let off_yes = compute_sticky_offset(
        offset(0, 1800),
        offset(0, 1100),
        viewport(),
        &insets(None, None, Some(20), None),
        size(100, 50),
        large_cb(),
    );
    assert_eq!(off_yes.top, lu(-170));
}
