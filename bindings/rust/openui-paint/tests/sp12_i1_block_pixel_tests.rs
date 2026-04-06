//! SP12 Phase I1 — Block Layout Pixel Comparison Tests.
//!
//! Renders block-level layouts and validates pixel output to verify
//! that block elements are painted at the correct positions with
//! correct box-model geometry (padding, border, margin, width/height).
//!
//! ## Running
//!
//! ```bash
//! cd bindings/rust
//! cargo test --package openui-paint --test sp12_i1_block_pixel_tests
//! ```

use skia_safe::Surface;

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::Length;
use openui_paint::render_to_surface;
use openui_style::*;

// ═══════════════════════════════════════════════════════════════════════
// ── Constants ───────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

const SURFACE_W: i32 = 800;
const SURFACE_H: i32 = 600;
const VP: i32 = 20;
const TOLERANCE: u8 = 2;

// Color tuples for pixel assertions
const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 128, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);
const BLACK: (u8, u8, u8) = (0, 0, 0);
const WHITE: (u8, u8, u8) = (255, 255, 255);
const CYAN: (u8, u8, u8) = (0, 255, 255);
const MAGENTA: (u8, u8, u8) = (255, 0, 255);
const YELLOW: (u8, u8, u8) = (255, 255, 0);
const ORANGE: (u8, u8, u8) = (255, 165, 0);
const GRAY: (u8, u8, u8) = (128, 128, 128);
const NAVY: (u8, u8, u8) = (0, 0, 128);
const MAROON: (u8, u8, u8) = (128, 0, 0);
const SILVER: (u8, u8, u8) = (192, 192, 192);

// ═══════════════════════════════════════════════════════════════════════
// ── Pixel sampling helpers ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn get_pixel(surface: &mut Surface, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = (info.width() * 4) as usize;
    let mut pixels = vec![0u8; row_bytes];
    let single_row_info = skia_safe::ImageInfo::new(
        (info.width(), 1),
        skia_safe::ColorType::RGBA8888,
        info.alpha_type(),
        None,
    );
    image.read_pixels(
        &single_row_info,
        &mut pixels,
        row_bytes,
        (0, y),
        skia_safe::image::CachingHint::Allow,
    );
    let idx = (x as usize) * 4;
    (pixels[idx], pixels[idx + 1], pixels[idx + 2], pixels[idx + 3])
}

fn assert_pixel_color(surface: &mut Surface, x: i32, y: i32, expected: (u8, u8, u8), msg: &str) {
    let (r, g, b, _a) = get_pixel(surface, x, y);
    let dr = (r as i16 - expected.0 as i16).unsigned_abs();
    let dg = (g as i16 - expected.1 as i16).unsigned_abs();
    let db = (b as i16 - expected.2 as i16).unsigned_abs();
    assert!(
        dr <= TOLERANCE as u16 && dg <= TOLERANCE as u16 && db <= TOLERANCE as u16,
        "{}: pixel ({},{}) = ({},{},{}) expected ~({},{},{}), diff=({},{},{})",
        msg, x, y, r, g, b, expected.0, expected.1, expected.2, dr, dg, db
    );
}

fn has_visible_content(surface: &mut Surface) -> bool {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = (info.width() * 4) as usize;
    let total_bytes = (info.height() as usize) * row_bytes;
    let mut pixels = vec![0u8; total_bytes];
    let read_info = skia_safe::ImageInfo::new(
        (info.width(), info.height()),
        skia_safe::ColorType::RGBA8888,
        info.alpha_type(),
        None,
    );
    image.read_pixels(
        &read_info,
        &mut pixels,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    for chunk in pixels.chunks(4) {
        if chunk.len() == 4 && (chunk[0] != 0xFF || chunk[1] != 0xFF || chunk[2] != 0xFF) {
            return true;
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════
// ── DOM builder helpers ─────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn setup_viewport(doc: &mut Document) -> NodeId {
    let vp = doc.root();
    doc.node_mut(vp).style.display = Display::Block;
    doc.node_mut(vp).style.background_color = Color::WHITE;
    doc.node_mut(vp).style.padding_top = Length::px(20.0);
    doc.node_mut(vp).style.padding_right = Length::px(20.0);
    doc.node_mut(vp).style.padding_bottom = Length::px(20.0);
    doc.node_mut(vp).style.padding_left = Length::px(20.0);
    doc.node_mut(vp).style.font_family = FontFamilyList::single("DejaVu Sans");
    doc.node_mut(vp).style.font_size = 16.0;
    doc.node_mut(vp).style.color = Color::BLACK;
    vp
}

fn add_colored_block(doc: &mut Document, parent: NodeId, w: f32, h: f32, color: Color) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    if w > 0.0 {
        doc.node_mut(div).style.width = Length::px(w);
    }
    if h > 0.0 {
        doc.node_mut(div).style.height = Length::px(h);
    }
    doc.node_mut(div).style.background_color = color;
    doc.append_child(parent, div);
    div
}

fn render(doc: &Document) -> Surface {
    render_to_surface(doc, SURFACE_W, SURFACE_H).expect("render_to_surface failed")
}

// ═══════════════════════════════════════════════════════════════════════
// ── 1. Normal Flow Stacking (26 tests) ─────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn flow_single_red_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "red block center");
}

#[test]
fn flow_single_blue_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 80.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 120, 60, BLUE, "blue block center");
}

#[test]
fn flow_single_green_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 150.0, 60.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 95, 50, GREEN, "green block center");
}

#[test]
fn flow_two_blocks_stack() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "first block");
    assert_pixel_color(&mut s, 70, 95, BLUE, "second block");
}

#[test]
fn flow_three_blocks_stack() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 35, RED, "first");
    assert_pixel_color(&mut s, 70, 65, GREEN, "second");
    assert_pixel_color(&mut s, 70, 95, BLUE, "third");
}

#[test]
fn flow_different_heights() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 100.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "short block");
    // second block: y=20+50=70, center y=70+50=120
    assert_pixel_color(&mut s, 70, 120, BLUE, "tall block");
}

#[test]
fn flow_different_widths() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // red: center (120,45), blue: center (70,95)
    assert_pixel_color(&mut s, 120, 45, RED, "wide block");
    assert_pixel_color(&mut s, 70, 95, BLUE, "narrow block");
}

#[test]
fn flow_auto_width_fills() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    // w=0 means auto, fills 760px container
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 400, 45, RED, "center of auto-width");
    assert_pixel_color(&mut s, 25, 45, RED, "left edge of auto-width");
    assert_pixel_color(&mut s, 775, 45, RED, "right edge of auto-width");
}

#[test]
fn flow_first_at_origin() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 50.0, 50.0, Color::RED);
    let mut s = render(&doc);
    // block starts at (VP, VP) = (20, 20)
    assert_pixel_color(&mut s, 22, 22, RED, "near top-left of block");
}

#[test]
fn flow_second_directly_below() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // blue top at y=60, center y=80
    assert_pixel_color(&mut s, 70, 62, BLUE, "just inside second block");
}

#[test]
fn flow_no_gap_between() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // red bottom row y=69, blue top row y=70
    assert_pixel_color(&mut s, 70, 69, RED, "last red row");
    assert_pixel_color(&mut s, 70, 70, BLUE, "first blue row");
}

#[test]
fn flow_ten_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let colors = [Color::RED, Color::BLUE, Color::GREEN, Color::RED, Color::BLUE,
                  Color::GREEN, Color::RED, Color::BLUE, Color::GREEN, Color::RED];
    let expected = [RED, BLUE, GREEN, RED, BLUE, GREEN, RED, BLUE, GREEN, RED];
    for &c in &colors { add_colored_block(&mut doc, vp, 80.0, 10.0, c); }
    let mut s = render(&doc);
    for i in 0..10 {
        let cy = VP + i as i32 * 10 + 5;
        assert_pixel_color(&mut s, 60, cy, expected[i], &format!("block {}", i));
    }
}

#[test]
fn flow_alternating_red_blue() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    for i in 0..4 {
        let c = if i % 2 == 0 { Color::RED } else { Color::BLUE };
        add_colored_block(&mut doc, vp, 100.0, 40.0, c);
    }
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 40, RED, "block 0");
    assert_pixel_color(&mut s, 70, 80, BLUE, "block 1");
    assert_pixel_color(&mut s, 70, 120, RED, "block 2");
    assert_pixel_color(&mut s, 70, 160, BLUE, "block 3");
}

#[test]
fn flow_narrow_left_aligned() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 50.0, 30.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 45, 35, RED, "inside narrow block");
    // just outside right edge: x=70 should be white
    assert_pixel_color(&mut s, 71, 35, WHITE, "outside narrow block");
}

#[test]
fn flow_tall_pushes_next() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 200.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // blue at y=20+200=220, center y=245
    assert_pixel_color(&mut s, 70, 245, BLUE, "after tall block");
}

#[test]
fn flow_1px_height() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 1.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 20, RED, "1px block row");
    assert_pixel_color(&mut s, 70, 21, WHITE, "below 1px block");
}

#[test]
fn flow_full_width_760() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 760.0, 30.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 400, 35, RED, "center");
    assert_pixel_color(&mut s, 20, 35, RED, "left edge");
    assert_pixel_color(&mut s, 779, 35, RED, "right edge");
}

#[test]
fn flow_small_10x10() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 10.0, 10.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 25, 25, RED, "inside 10x10");
}

#[test]
fn flow_center_of_first() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 100.0, Color::GREEN);
    let mut s = render(&doc);
    // center: (20+100, 20+50) = (120, 70)
    assert_pixel_color(&mut s, 120, 70, GREEN, "first block center");
}

#[test]
fn flow_center_of_second() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 200.0, 50.0, Color::GREEN);
    let mut s = render(&doc);
    // second: y=70, center (120, 95)
    assert_pixel_color(&mut s, 120, 95, GREEN, "second center");
}

#[test]
fn flow_third_block_y() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // third: y=20+40+40=100, center y=120
    assert_pixel_color(&mut s, 70, 120, BLUE, "third block");
}

#[test]
fn flow_five_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let colors = [Color::RED, Color::BLUE, Color::GREEN, Color::RED, Color::BLUE];
    let expected = [RED, BLUE, GREEN, RED, BLUE];
    for &c in &colors { add_colored_block(&mut doc, vp, 100.0, 50.0, c); }
    let mut s = render(&doc);
    for i in 0..5 {
        let cy = VP + i as i32 * 50 + 25;
        assert_pixel_color(&mut s, 70, cy, expected[i], &format!("block {}", i));
    }
}

#[test]
fn flow_boundary_exact() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    // boundary at y=80
    assert_pixel_color(&mut s, 70, 79, RED, "last red row");
    assert_pixel_color(&mut s, 70, 80, BLUE, "first blue row");
}

#[test]
fn flow_wide_then_narrow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 300.0, 40.0, Color::RED);
    add_colored_block(&mut doc, vp, 50.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 170, 40, RED, "wide block");
    assert_pixel_color(&mut s, 45, 80, BLUE, "narrow block");
}

#[test]
fn flow_four_colors() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 120.0, 30.0, Color::RED);
    add_colored_block(&mut doc, vp, 120.0, 30.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 120.0, 30.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 120.0, 30.0, Color::from_rgba8(255, 255, 0, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 80, 35, RED, "block 0");
    assert_pixel_color(&mut s, 80, 65, GREEN, "block 1");
    assert_pixel_color(&mut s, 80, 95, BLUE, "block 2");
    assert_pixel_color(&mut s, 80, 125, YELLOW, "block 3");
}

#[test]
fn flow_right_edge_white() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    let mut s = render(&doc);
    // right edge of block at x=219, next pixel at x=220
    assert_pixel_color(&mut s, 218, 45, RED, "inside right edge");
    assert_pixel_color(&mut s, 221, 45, WHITE, "outside right edge");
}

// ═══════════════════════════════════════════════════════════════════════
// ── 2. Box Model — Padding (25 tests) ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn padding_top_shifts() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.padding_top = Length::px(20.0);
    let mut s = render(&doc);
    // bg covers padding: (20,20) to (119,89), total h=20+50=70
    assert_pixel_color(&mut s, 70, 30, RED, "padding area is bg color");
    assert_pixel_color(&mut s, 70, 65, RED, "content area");
}

#[test]
fn padding_left_shifts() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.padding_left = Length::px(30.0);
    let mut s = render(&doc);
    // bg covers (20,20) to (149,69), total w=30+100=130
    assert_pixel_color(&mut s, 35, 45, BLUE, "left padding area");
    assert_pixel_color(&mut s, 80, 45, BLUE, "content area");
}

#[test]
fn padding_right_extends() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.padding_right = Length::px(40.0);
    let mut s = render(&doc);
    // total w=100+40=140, bg to x=20+140-1=159
    assert_pixel_color(&mut s, 155, 45, GREEN, "right padding area");
}

#[test]
fn padding_bottom_extends() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.padding_bottom = Length::px(25.0);
    let mut s = render(&doc);
    // total h=50+25=75, bg to y=20+75-1=94
    assert_pixel_color(&mut s, 70, 90, RED, "bottom padding area");
}

#[test]
fn padding_all_10px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_right = Length::px(10.0);
    doc.node_mut(div).style.padding_bottom = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    let mut s = render(&doc);
    // total 120x70, bg from (20,20) to (139,89)
    assert_pixel_color(&mut s, 25, 25, BLUE, "top-left padding");
    assert_pixel_color(&mut s, 80, 55, BLUE, "content center");
    assert_pixel_color(&mut s, 135, 85, BLUE, "bottom-right padding");
}

#[test]
fn padding_all_20px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::RED);
    doc.node_mut(div).style.padding_top = Length::px(20.0);
    doc.node_mut(div).style.padding_right = Length::px(20.0);
    doc.node_mut(div).style.padding_bottom = Length::px(20.0);
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    let mut s = render(&doc);
    // total 120x80, bg from (20,20) to (139,99)
    assert_pixel_color(&mut s, 80, 60, RED, "center of padded block");
}

#[test]
fn padding_top_50px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.padding_top = Length::px(50.0);
    let mut s = render(&doc);
    // total h=50+50=100, bg covers (20,20) to (119,119)
    assert_pixel_color(&mut s, 70, 45, GREEN, "deep in top padding");
    assert_pixel_color(&mut s, 70, 95, GREEN, "content area");
}

#[test]
fn padding_left_30px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::BLUE);
    doc.node_mut(div).style.padding_left = Length::px(30.0);
    let mut s = render(&doc);
    // total w=30+80=110, bg (20,20) to (129,59)
    assert_pixel_color(&mut s, 35, 40, BLUE, "in left padding");
}

#[test]
fn padding_asymmetric() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_right = Length::px(20.0);
    doc.node_mut(div).style.padding_bottom = Length::px(30.0);
    doc.node_mut(div).style.padding_left = Length::px(40.0);
    let mut s = render(&doc);
    // total 160x90, bg (20,20) to (179,109)
    assert_pixel_color(&mut s, 100, 65, RED, "center of asymmetric padded");
}

#[test]
fn padding_bg_in_padding_area() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 60.0, 30.0, Color::RED);
    doc.node_mut(div).style.padding_left = Length::px(50.0);
    let mut s = render(&doc);
    // padding area at x=25 (inside 50px padding), should be bg color
    assert_pixel_color(&mut s, 25, 35, RED, "bg in padding area");
}

#[test]
fn padding_top_and_bottom() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    doc.node_mut(div).style.padding_top = Length::px(15.0);
    doc.node_mut(div).style.padding_bottom = Length::px(15.0);
    let mut s = render(&doc);
    // total h=15+30+15=60
    assert_pixel_color(&mut s, 70, 25, BLUE, "top padding");
    assert_pixel_color(&mut s, 70, 70, BLUE, "bottom padding");
}

#[test]
fn padding_left_and_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::GREEN);
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    doc.node_mut(div).style.padding_right = Length::px(20.0);
    let mut s = render(&doc);
    // total w=20+80+20=120, bg (20,20) to (139,59)
    assert_pixel_color(&mut s, 25, 40, GREEN, "left padding");
    assert_pixel_color(&mut s, 135, 40, GREEN, "right padding");
}

#[test]
fn padding_large_50_all() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 60.0, 30.0, Color::RED);
    doc.node_mut(div).style.padding_top = Length::px(50.0);
    doc.node_mut(div).style.padding_right = Length::px(50.0);
    doc.node_mut(div).style.padding_bottom = Length::px(50.0);
    doc.node_mut(div).style.padding_left = Length::px(50.0);
    let mut s = render(&doc);
    // total 160x130
    assert_pixel_color(&mut s, 100, 85, RED, "center of large padded");
}

#[test]
fn padding_zero_is_default() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    // no padding: bg exactly at (20,20) to (119,69)
    assert_pixel_color(&mut s, 70, 45, RED, "no padding center");
    assert_pixel_color(&mut s, 121, 45, WHITE, "outside block");
}

#[test]
fn padding_1px_all() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.padding_top = Length::px(1.0);
    doc.node_mut(div).style.padding_right = Length::px(1.0);
    doc.node_mut(div).style.padding_bottom = Length::px(1.0);
    doc.node_mut(div).style.padding_left = Length::px(1.0);
    let mut s = render(&doc);
    // total 102x52
    assert_pixel_color(&mut s, 71, 46, BLUE, "center of 1px padded");
}

#[test]
fn padding_increases_box_height() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.padding_bottom = Length::px(30.0);
    // total h=50+30=80, second block at y=20+80=100
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 120, BLUE, "after padded block");
}

#[test]
fn padding_stacking_after_padded() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_bottom = Length::px(10.0);
    // total h=10+30+10=50
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    let mut s = render(&doc);
    // blue at y=20+50=70, center y=85
    assert_pixel_color(&mut s, 70, 85, BLUE, "stacked after padded");
}

#[test]
fn padding_nested_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(parent).style.padding_top = Length::px(20.0);
    doc.node_mut(parent).style.padding_left = Length::px(20.0);
    // child inside parent's content area at (20+20, 20+20) = (40, 40)
    add_colored_block(&mut doc, parent, 80.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 30, 30, RED, "parent padding area");
    assert_pixel_color(&mut s, 80, 60, BLUE, "child over parent");
}

#[test]
fn padding_only_top() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::GREEN);
    doc.node_mut(div).style.padding_top = Length::px(25.0);
    let mut s = render(&doc);
    // total h=25+40=65
    assert_pixel_color(&mut s, 70, 30, GREEN, "top padding");
    assert_pixel_color(&mut s, 70, 60, GREEN, "content");
}

#[test]
fn padding_only_bottom() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::GREEN);
    doc.node_mut(div).style.padding_bottom = Length::px(25.0);
    let mut s = render(&doc);
    // total h=40+25=65, bottom padding area at y=60+20-1=79
    assert_pixel_color(&mut s, 70, 80, GREEN, "bottom padding");
}

#[test]
fn padding_only_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::BLUE);
    doc.node_mut(div).style.padding_left = Length::px(30.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 30, 40, BLUE, "left padding");
}

#[test]
fn padding_only_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::BLUE);
    doc.node_mut(div).style.padding_right = Length::px(30.0);
    let mut s = render(&doc);
    // total w=80+30=110, right padding area at x ~105+20=125
    assert_pixel_color(&mut s, 125, 40, BLUE, "right padding");
}

#[test]
fn padding_top_left_combo() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.padding_top = Length::px(15.0);
    doc.node_mut(div).style.padding_left = Length::px(15.0);
    let mut s = render(&doc);
    // bg covers (20,20) to (134,84), total 115x65
    assert_pixel_color(&mut s, 25, 25, RED, "top-left padding corner");
}

#[test]
fn padding_different_each_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.padding_top = Length::px(5.0);
    doc.node_mut(div).style.padding_right = Length::px(10.0);
    doc.node_mut(div).style.padding_bottom = Length::px(15.0);
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    let mut s = render(&doc);
    // total 130x70
    assert_pixel_color(&mut s, 85, 55, GREEN, "center of varied padding");
}

// ═══════════════════════════════════════════════════════════════════════
// ── 3. Box Model — Borders (25 tests) ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn border_top_1px_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 1;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 20, RED, "1px top border");
}

#[test]
fn border_all_1px_black() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    for setter in [
        |d: &mut Document, id: NodeId| {
            d.node_mut(id).style.border_top_style = BorderStyle::Solid;
            d.node_mut(id).style.border_top_width = 1;
            d.node_mut(id).style.border_top_color = StyleColor::Resolved(Color::BLACK);
        },
        |d: &mut Document, id: NodeId| {
            d.node_mut(id).style.border_right_style = BorderStyle::Solid;
            d.node_mut(id).style.border_right_width = 1;
            d.node_mut(id).style.border_right_color = StyleColor::Resolved(Color::BLACK);
        },
        |d: &mut Document, id: NodeId| {
            d.node_mut(id).style.border_bottom_style = BorderStyle::Solid;
            d.node_mut(id).style.border_bottom_width = 1;
            d.node_mut(id).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
        },
        |d: &mut Document, id: NodeId| {
            d.node_mut(id).style.border_left_style = BorderStyle::Solid;
            d.node_mut(id).style.border_left_width = 1;
            d.node_mut(id).style.border_left_color = StyleColor::Resolved(Color::BLACK);
        },
    ] { setter(&mut doc, div); }
    let mut s = render(&doc);
    // total box: 102x52, borders at edges
    assert_pixel_color(&mut s, 70, 20, BLACK, "top border");
    assert_pixel_color(&mut s, 20, 45, BLACK, "left border");
}

#[test]
fn border_all_2px_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    let c = Color::RED;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 2;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 2;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 2;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 2;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(c);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 20, RED, "top border row 0");
    assert_pixel_color(&mut s, 70, 21, RED, "top border row 1");
}

#[test]
fn border_all_5px_blue() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    let c = Color::BLUE;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 5;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(c);
    let mut s = render(&doc);
    // total 110x60, border occupies 5px each side
    assert_pixel_color(&mut s, 70, 22, BLUE, "top border mid");
    assert_pixel_color(&mut s, 22, 45, BLUE, "left border mid");
    // content at center
    assert_pixel_color(&mut s, 75, 50, WHITE, "content center");
}

#[test]
fn border_left_5px_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 22, 45, RED, "left border");
    assert_pixel_color(&mut s, 27, 45, WHITE, "content after left border");
}

#[test]
fn border_right_5px_green() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::GREEN);
    let mut s = render(&doc);
    // content 100px at x=[20,119], border at x=[120,124]
    assert_pixel_color(&mut s, 122, 45, GREEN, "right border");
}

#[test]
fn border_top_10px_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 10;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 25, RED, "10px top border mid");
    assert_pixel_color(&mut s, 70, 35, WHITE, "content below border");
}

#[test]
fn border_bottom_10px_blue() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 10;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLUE);
    let mut s = render(&doc);
    // content y=[20,69], border y=[70,79]
    assert_pixel_color(&mut s, 70, 75, BLUE, "10px bottom border");
}

#[test]
fn border_all_10px_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::WHITE);
    let c = Color::RED;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 10;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 10;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 10;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 10;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(c);
    let mut s = render(&doc);
    // total 100x60, border 10px each side
    assert_pixel_color(&mut s, 25, 25, RED, "top-left border area");
    assert_pixel_color(&mut s, 70, 50, WHITE, "content center");
}

#[test]
fn border_top_and_bottom() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 5;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 22, RED, "top border");
    // bottom border: y = 20 + 5(top) + 50(content) = 75..79
    assert_pixel_color(&mut s, 70, 77, BLUE, "bottom border");
}

#[test]
fn border_left_and_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::GREEN);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // left border x=[20,24], content x=[25,124], right border x=[125,129]
    assert_pixel_color(&mut s, 22, 45, GREEN, "left border");
    assert_pixel_color(&mut s, 127, 45, RED, "right border");
}

#[test]
fn border_different_colors() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::WHITE);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 5;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLUE);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::GREEN);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::from_rgba8(255, 255, 0, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 60, 22, RED, "top red");
    // bottom border: y = 20 + 5(top) + 60(content) = 85..89
    assert_pixel_color(&mut s, 60, 87, BLUE, "bottom blue");
    assert_pixel_color(&mut s, 22, 50, GREEN, "left green");
    assert_pixel_color(&mut s, 127, 50, YELLOW, "right yellow");
}

#[test]
fn border_shifts_content_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 80.0, Color::WHITE);
    doc.node_mut(parent).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_left_width = 10;
    doc.node_mut(parent).style.border_left_color = StyleColor::Resolved(Color::RED);
    add_colored_block(&mut doc, parent, 50.0, 30.0, Color::BLUE);
    let mut s = render(&doc);
    // child content starts at x=20+10=30
    assert_pixel_color(&mut s, 55, 35, BLUE, "child shifted right");
    assert_pixel_color(&mut s, 25, 45, RED, "left border");
}

#[test]
fn border_shifts_content_down() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 80.0, Color::WHITE);
    doc.node_mut(parent).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_top_width = 10;
    doc.node_mut(parent).style.border_top_color = StyleColor::Resolved(Color::RED);
    add_colored_block(&mut doc, parent, 50.0, 30.0, Color::BLUE);
    let mut s = render(&doc);
    // child at y=20+10=30
    assert_pixel_color(&mut s, 45, 45, BLUE, "child shifted down");
    assert_pixel_color(&mut s, 70, 25, RED, "top border");
}

#[test]
fn border_1px_all_sides() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 60.0, 30.0, Color::from_rgba8(192, 192, 192, 255));
    let c = Color::BLACK;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 1;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 1;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 1;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 1;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(c);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 50, 20, BLACK, "top border 1px");
    assert_pixel_color(&mut s, 50, 35, SILVER, "content center");
}

#[test]
fn border_2px_left_only() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::WHITE);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 2;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 20, 40, GREEN, "left border px0");
    assert_pixel_color(&mut s, 21, 40, GREEN, "left border px1");
    assert_pixel_color(&mut s, 22, 40, WHITE, "content after border");
}

#[test]
fn border_affects_stacking() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 10;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    // total h = 40+10=50, second block at y=70
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 65, BLACK, "bottom border");
    assert_pixel_color(&mut s, 70, 90, BLUE, "block after border");
}

#[test]
fn border_with_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::from_rgba8(192, 192, 192, 255));
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // border top y=[20,24], padding y=[25,34], content y=[35,74]
    // border left x=[20,24], padding x=[25,34], content x=[35,114]
    // Check top border away from corner to avoid diagonal anti-aliasing
    assert_pixel_color(&mut s, 60, 22, RED, "border top");
    assert_pixel_color(&mut s, 30, 30, SILVER, "padding area");
}

#[test]
fn border_thick_5px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 120.0, 60.0, Color::WHITE);
    let c = Color::BLUE;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 5;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(c);
    let mut s = render(&doc);
    // total 130x70
    assert_pixel_color(&mut s, 85, 22, BLUE, "top border");
    assert_pixel_color(&mut s, 85, 55, WHITE, "content");
}

#[test]
fn border_top_red_bottom_blue() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::WHITE);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 5;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 22, RED, "top red");
    assert_pixel_color(&mut s, 70, 67, BLUE, "bottom blue");
}

#[test]
fn border_left_green_right_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::GREEN);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 22, 45, GREEN, "left green");
    assert_pixel_color(&mut s, 127, 45, RED, "right red");
}

#[test]
fn border_nested_with_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::WHITE);
    doc.node_mut(parent).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_top_width = 5;
    doc.node_mut(parent).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(parent).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_left_width = 5;
    doc.node_mut(parent).style.border_left_color = StyleColor::Resolved(Color::RED);
    // child starts at (25, 25) inside parent
    add_colored_block(&mut doc, parent, 80.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // Check border away from corner to avoid diagonal anti-aliasing
    assert_pixel_color(&mut s, 80, 22, RED, "parent top border");
    assert_pixel_color(&mut s, 65, 45, BLUE, "child inside");
}

#[test]
fn border_3px_all_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(128, 128, 128, 255));
    let c = Color::RED;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 3;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 3;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 3;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 3;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(c);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 21, 21, RED, "border corner");
    assert_pixel_color(&mut s, 73, 48, GRAY, "content center");
}

#[test]
fn border_asymmetric_widths() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 2;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 8;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 20, RED, "thin top");
    assert_pixel_color(&mut s, 24, 40, BLUE, "thick left");
}

#[test]
fn border_content_center_bg() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::GREEN);
    let c = Color::RED;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 5;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(c);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(c);
    let mut s = render(&doc);
    // content center: (25+50, 25+30) = (75, 55)
    assert_pixel_color(&mut s, 75, 55, GREEN, "bg shows through content");
}

// ═══════════════════════════════════════════════════════════════════════
// ── 4. Box Model — Margins (26 tests) ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn margin_top_10px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.margin_top = Length::px(10.0);
    let mut s = render(&doc);
    // block at y=20+10=30, center y=55
    assert_pixel_color(&mut s, 70, 55, RED, "margin-top shifts block");
    assert_pixel_color(&mut s, 70, 25, WHITE, "gap from margin");
}

#[test]
fn margin_top_20px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 65, BLUE, "center after 20px margin");
}

#[test]
fn margin_top_50px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.margin_top = Length::px(50.0);
    let mut s = render(&doc);
    // block at y=70, center y=95
    assert_pixel_color(&mut s, 70, 95, GREEN, "center after 50px margin");
}

#[test]
fn margin_bottom_10px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.margin_bottom = Length::px(10.0);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // blue at y=20+50+10=80, center y=105
    assert_pixel_color(&mut s, 70, 105, BLUE, "after margin-bottom gap");
}

#[test]
fn margin_left_10px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::px(10.0);
    let mut s = render(&doc);
    // block at x=30, center x=80
    assert_pixel_color(&mut s, 80, 45, RED, "shifted by left margin");
    assert_pixel_color(&mut s, 25, 45, WHITE, "margin gap");
}

#[test]
fn margin_left_30px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.margin_left = Length::px(30.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 100, 45, BLUE, "center after left 30");
}

#[test]
fn margin_auto_center_200() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // centered: x=20+(760-200)/2=300, center x=400
    assert_pixel_color(&mut s, 400, 45, RED, "centered 200px");
}

#[test]
fn margin_auto_center_400() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 400.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // centered: x=20+(760-400)/2=200, center x=400
    assert_pixel_color(&mut s, 400, 45, BLUE, "centered 400px");
}

#[test]
fn margin_auto_center_100() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // centered: x=20+(760-100)/2=350, center x=400
    assert_pixel_color(&mut s, 400, 45, GREEN, "centered 100px");
}

#[test]
fn margin_all_10px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.margin_top = Length::px(10.0);
    doc.node_mut(div).style.margin_right = Length::px(10.0);
    doc.node_mut(div).style.margin_bottom = Length::px(10.0);
    doc.node_mut(div).style.margin_left = Length::px(10.0);
    let mut s = render(&doc);
    // block at (30, 30), center (80, 55)
    assert_pixel_color(&mut s, 80, 55, RED, "all margins 10px");
}

#[test]
fn margin_top_pushes_second() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    let div2 = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    doc.node_mut(div2).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    // blue at y=20+40+20=80, center y=100
    assert_pixel_color(&mut s, 70, 100, BLUE, "margin-top pushes second");
}

#[test]
fn margin_bottom_separates() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div1 = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    doc.node_mut(div1).style.margin_bottom = Length::px(30.0);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // blue at y=20+40+30=90, center y=110
    assert_pixel_color(&mut s, 70, 110, BLUE, "margin-bottom separates");
}

#[test]
fn margin_left_shifts_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::GREEN);
    doc.node_mut(div).style.margin_left = Length::px(50.0);
    let mut s = render(&doc);
    // block at x=70, center x=110
    assert_pixel_color(&mut s, 110, 40, GREEN, "left margin shift");
}

#[test]
fn margin_large_top() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.margin_top = Length::px(100.0);
    let mut s = render(&doc);
    // block at y=120, center y=145
    assert_pixel_color(&mut s, 70, 145, RED, "large margin top");
}

#[test]
fn margin_between_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(15.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(15.0);
    let mut s = render(&doc);
    // Margins collapse: gap = max(15,15) = 15, d2 at y=20+30+15=65
    assert_pixel_color(&mut s, 70, 35, RED, "first block");
    assert_pixel_color(&mut s, 70, 80, BLUE, "second block (collapsed)");
}

#[test]
fn margin_auto_centers() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 300.0, 40.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // centered: x=20+(760-300)/2=250, center x=400
    assert_pixel_color(&mut s, 400, 40, RED, "auto center");
    // left side should be white
    assert_pixel_color(&mut s, 245, 40, WHITE, "left of centered block");
}

#[test]
fn margin_zero_default() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 22, 22, RED, "block starts at VP with no margin");
}

#[test]
fn margin_1px_all() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(div).style.margin_top = Length::px(1.0);
    doc.node_mut(div).style.margin_left = Length::px(1.0);
    let mut s = render(&doc);
    // block at (21, 21), center (71, 46)
    assert_pixel_color(&mut s, 71, 46, BLUE, "1px margins");
}

#[test]
fn margin_top_only() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::GREEN);
    doc.node_mut(div).style.margin_top = Length::px(25.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 65, GREEN, "margin-top only");
}

#[test]
fn margin_bottom_only() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(40.0);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    let mut s = render(&doc);
    // blue at y=20+30+40=90, center y=105
    assert_pixel_color(&mut s, 70, 105, BLUE, "margin-bottom only");
}

#[test]
fn margin_left_only() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::px(40.0);
    let mut s = render(&doc);
    // block at x=60, center x=100
    assert_pixel_color(&mut s, 100, 40, RED, "margin-left only");
}

#[test]
fn margin_auto_narrow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 60.0, 30.0, Color::BLUE);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // centered: x=20+(760-60)/2=370, center x=400
    assert_pixel_color(&mut s, 400, 35, BLUE, "narrow centered");
}

#[test]
fn margin_auto_wide() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 600.0, 30.0, Color::GREEN);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // centered: x=20+(760-600)/2=100, center x=400
    assert_pixel_color(&mut s, 400, 35, GREEN, "wide centered");
}

#[test]
fn margin_top_and_bottom() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    doc.node_mut(div).style.margin_top = Length::px(15.0);
    doc.node_mut(div).style.margin_bottom = Length::px(15.0);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // red at y=35, center y=55
    assert_pixel_color(&mut s, 70, 55, RED, "red with top margin");
    // blue at y=35+40+15=90, center y=110
    assert_pixel_color(&mut s, 70, 110, BLUE, "blue after bottom margin");
}

#[test]
fn margin_auto_center_exact() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 40.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // block starts at x=300, ends at x=499
    assert_pixel_color(&mut s, 302, 40, RED, "just inside left edge");
    assert_pixel_color(&mut s, 497, 40, RED, "just inside right edge");
    assert_pixel_color(&mut s, 295, 40, WHITE, "outside left");
}

#[test]
fn margin_combined_top_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::BLUE);
    doc.node_mut(div).style.margin_top = Length::px(30.0);
    doc.node_mut(div).style.margin_left = Length::px(30.0);
    let mut s = render(&doc);
    // block at (50, 50), center (90, 70)
    assert_pixel_color(&mut s, 90, 70, BLUE, "top+left margins");
}

// ═══════════════════════════════════════════════════════════════════════
// ── 5. Width / Height (25 tests) ───────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn size_width_100() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "inside 100px wide");
    assert_pixel_color(&mut s, 121, 45, WHITE, "outside 100px");
}

#[test]
fn size_width_200() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 120, 45, BLUE, "inside 200px");
    assert_pixel_color(&mut s, 221, 45, WHITE, "outside 200px");
}

#[test]
fn size_width_400() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 400.0, 50.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 220, 45, GREEN, "inside 400px");
    assert_pixel_color(&mut s, 421, 45, WHITE, "outside 400px");
}

#[test]
fn size_width_760_full() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 760.0, 40.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 400, 40, RED, "center of full-width");
    assert_pixel_color(&mut s, 779, 40, RED, "right edge of full-width");
}

#[test]
fn size_auto_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 400, 40, BLUE, "auto fills center");
    assert_pixel_color(&mut s, 22, 40, BLUE, "auto fills left");
    assert_pixel_color(&mut s, 777, 40, BLUE, "auto fills right");
}

#[test]
fn size_height_50() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 69, RED, "bottom of 50px");
    assert_pixel_color(&mut s, 70, 71, WHITE, "below 50px");
}

#[test]
fn size_height_100() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 100.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 119, GREEN, "bottom of 100px");
    assert_pixel_color(&mut s, 70, 121, WHITE, "below 100px");
}

#[test]
fn size_height_200() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 200.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 120, BLUE, "center of 200px tall");
}

#[test]
fn size_width_50_percent() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(50.0);
    doc.node_mut(div).style.height = Length::px(40.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // 50% of 760 = 380, center x = 20+190 = 210
    assert_pixel_color(&mut s, 210, 40, RED, "50% width center");
    assert_pixel_color(&mut s, 401, 40, WHITE, "outside 50%");
}

#[test]
fn size_width_100_percent() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(100.0);
    doc.node_mut(div).style.height = Length::px(40.0);
    doc.node_mut(div).style.background_color = Color::BLUE;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 400, 40, BLUE, "100% fills container");
}

#[test]
fn size_width_25_percent() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(25.0);
    doc.node_mut(div).style.height = Length::px(40.0);
    doc.node_mut(div).style.background_color = Color::GREEN;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // 25% of 760 = 190, center x = 20+95 = 115
    assert_pixel_color(&mut s, 115, 40, GREEN, "25% width center");
}

#[test]
fn size_width_75_percent() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(75.0);
    doc.node_mut(div).style.height = Length::px(40.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // 75% of 760 = 570, center x = 20+285 = 305
    assert_pixel_color(&mut s, 305, 40, RED, "75% width center");
}

#[test]
fn size_width_10px_tiny() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 10.0, 10.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 25, 25, RED, "tiny block center");
    assert_pixel_color(&mut s, 31, 25, WHITE, "outside tiny");
}

#[test]
fn size_height_1px() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 1.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 20, BLUE, "1px tall");
    assert_pixel_color(&mut s, 70, 21, WHITE, "below 1px");
}

#[test]
fn size_100x100() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 100.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 70, GREEN, "100x100 center");
}

#[test]
fn size_200x200() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 200.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 120, 120, RED, "200x200 center");
}

#[test]
fn size_50x50() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 50.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 45, 45, BLUE, "50x50 center");
}

#[test]
fn size_300x150() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 300.0, 150.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 170, 95, GREEN, "300x150 center");
}

#[test]
fn size_500x50() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 500.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 270, 45, RED, "500x50 center");
}

#[test]
fn size_different_widths_stacked() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    add_colored_block(&mut doc, vp, 200.0, 30.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 300.0, 30.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 35, RED, "100px wide");
    assert_pixel_color(&mut s, 120, 65, BLUE, "200px wide");
    assert_pixel_color(&mut s, 170, 95, GREEN, "300px wide");
}

#[test]
fn size_different_heights_stacked() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 20.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 30, RED, "h=20");
    assert_pixel_color(&mut s, 70, 70, BLUE, "h=60");
    // green at y=20+20+60=100, center y=120
    assert_pixel_color(&mut s, 70, 120, GREEN, "h=40");
}

#[test]
fn size_50_percent_is_380() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(50.0);
    doc.node_mut(div).style.height = Length::px(30.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // 380px wide, right edge at x=20+380-1=399
    assert_pixel_color(&mut s, 398, 35, RED, "right edge of 50%");
    assert_pixel_color(&mut s, 401, 35, WHITE, "outside 50%");
}

#[test]
fn size_10_percent_is_76() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(10.0);
    doc.node_mut(div).style.height = Length::px(30.0);
    doc.node_mut(div).style.background_color = Color::BLUE;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // 10% of 760 = 76, center x=20+38=58
    assert_pixel_color(&mut s, 58, 35, BLUE, "10% center");
}

#[test]
fn size_height_300() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 300.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 170, GREEN, "tall 300px center");
}

// ═══════════════════════════════════════════════════════════════════════
// ── 6. Margin Collapsing Visual (25 tests) ─────────────────────────
// ═══════════════════════════════════════════════════════════════════════
//
// These tests are intentionally flexible: they verify blocks render with
// correct colors at well-inside positions, without asserting exact gap
// sizes. The engine may or may not implement margin collapsing.

#[test]
fn collapse_adjacent_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(20.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 80.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(30.0);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "should render content");
    assert_pixel_color(&mut s, 70, 60, RED, "first block center");
    // second block: at y >= 120 (collapsed) or y >= 150 (not collapsed), center >= 160
    // check at y=190 which is inside block in both cases
    assert_pixel_color(&mut s, 70, 190, BLUE, "second block deep inside");
}

#[test]
fn collapse_first_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(25.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(25.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 50, RED, "first block");
}

#[test]
fn collapse_second_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(25.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(25.0);
    let mut s = render(&doc);
    // second at y>=105 (collapsed) or y>=130 (not), center at >= 135 or >= 160
    // y=160 is inside in both cases
    assert_pixel_color(&mut s, 70, 160, BLUE, "second block");
}

#[test]
fn collapse_both_colored() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 120.0, 70.0, Color::GREEN);
    doc.node_mut(d1).style.margin_bottom = Length::px(40.0);
    let d2 = add_colored_block(&mut doc, vp, 120.0, 70.0, Color::RED);
    doc.node_mut(d2).style.margin_top = Length::px(40.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 80, 55, GREEN, "first green");
    // Collapsed gap=40, d2 at y=20+70+40=130, center=165
    assert_pixel_color(&mut s, 80, 165, RED, "second red deep inside");
}

#[test]
fn collapse_three_blocks_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(15.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(d2).style.margin_top = Length::px(15.0);
    doc.node_mut(d2).style.margin_bottom = Length::px(15.0);
    let d3 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(d3).style.margin_top = Length::px(15.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "first");
    // Collapsed: d2 at y=85, d3 at y=150
    assert_pixel_color(&mut s, 70, 110, GREEN, "second deep inside");
    assert_pixel_color(&mut s, 70, 175, BLUE, "third deep inside");
}

#[test]
fn collapse_large_margins_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(50.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(50.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 50, RED, "first");
    // Collapsed gap=50, d2 at y=130, center=160
    assert_pixel_color(&mut s, 70, 160, BLUE, "second deep");
}

#[test]
fn collapse_small_margins_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::GREEN);
    doc.node_mut(d1).style.margin_bottom = Length::px(5.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    doc.node_mut(d2).style.margin_top = Length::px(5.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 50, GREEN, "first");
    assert_pixel_color(&mut s, 70, 130, RED, "second");
}

#[test]
fn collapse_zero_margin_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "first no margin");
    assert_pixel_color(&mut s, 70, 95, BLUE, "second no margin");
}

#[test]
fn collapse_different_colors() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 150.0, 60.0, Color::from_rgba8(255, 165, 0, 255));
    doc.node_mut(d1).style.margin_bottom = Length::px(20.0);
    let d2 = add_colored_block(&mut doc, vp, 150.0, 60.0, Color::from_rgba8(0, 255, 255, 255));
    doc.node_mut(d2).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 95, 50, ORANGE, "orange block");
    // Collapsed gap=20, d2 at y=100, center=130
    assert_pixel_color(&mut s, 95, 130, CYAN, "cyan block deep");
}

#[test]
fn collapse_red_blue() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(30.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 80.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(30.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 60, RED, "red center");
    assert_pixel_color(&mut s, 70, 200, BLUE, "blue deep");
}

#[test]
fn collapse_green_red() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 70.0, Color::GREEN);
    doc.node_mut(d1).style.margin_bottom = Length::px(20.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 70.0, Color::RED);
    doc.node_mut(d2).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 55, GREEN, "green");
    assert_pixel_color(&mut s, 70, 175, RED, "red deep");
}

#[test]
fn collapse_10px_margins() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(10.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(10.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "first");
    assert_pixel_color(&mut s, 70, 115, BLUE, "second");
}

#[test]
fn collapse_20px_margins() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(d1).style.margin_bottom = Length::px(20.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(d2).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, GREEN, "first");
    assert_pixel_color(&mut s, 70, 135, RED, "second deep");
}

#[test]
fn collapse_50px_margins() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    doc.node_mut(d1).style.margin_bottom = Length::px(50.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    doc.node_mut(d2).style.margin_top = Length::px(50.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 50, BLUE, "first");
    // Collapsed gap=50, d2 at y=130, center=160
    assert_pixel_color(&mut s, 70, 160, RED, "second deep");
}

#[test]
fn collapse_nested_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 0.0, Color::WHITE);
    doc.node_mut(parent).style.margin_top = Length::px(10.0);
    let child = add_colored_block(&mut doc, parent, 100.0, 50.0, Color::RED);
    doc.node_mut(child).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "nested should render");
}

#[test]
fn collapse_parent_child_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 300.0, 0.0, Color::WHITE);
    doc.node_mut(parent).style.margin_top = Length::px(15.0);
    let child = add_colored_block(&mut doc, parent, 100.0, 60.0, Color::GREEN);
    doc.node_mut(child).style.margin_top = Length::px(15.0);
    let mut s = render(&doc);
    // child should be visible regardless of collapse behavior
    assert!(has_visible_content(&mut s), "parent-child renders");
}

#[test]
fn collapse_three_colors() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(10.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(d2).style.margin_top = Length::px(10.0);
    doc.node_mut(d2).style.margin_bottom = Length::px(10.0);
    let d3 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    doc.node_mut(d3).style.margin_top = Length::px(10.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, RED, "red");
    // green and blue centers are inside their blocks regardless of collapse
    assert_pixel_color(&mut s, 70, 110, GREEN, "green");
    assert_pixel_color(&mut s, 70, 180, BLUE, "blue deep");
}

#[test]
fn collapse_visible_check() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(40.0);
    let d2 = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(40.0);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "two blocks visible");
}

#[test]
fn collapse_all_render() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    for i in 0..4 {
        let c = [Color::RED, Color::GREEN, Color::BLUE, Color::from_rgba8(255, 255, 0, 255)][i];
        let d = add_colored_block(&mut doc, vp, 100.0, 40.0, c);
        doc.node_mut(d).style.margin_top = Length::px(10.0);
        doc.node_mut(d).style.margin_bottom = Length::px(10.0);
    }
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "four blocks visible");
}

#[test]
fn collapse_mixed_sizes() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(10.0);
    let d2 = add_colored_block(&mut doc, vp, 150.0, 40.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 60, RED, "tall red");
    assert_pixel_color(&mut s, 95, 140, BLUE, "short blue");
}

#[test]
fn collapse_red_green_blue() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 80.0, 60.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(15.0);
    let d2 = add_colored_block(&mut doc, vp, 80.0, 60.0, Color::GREEN);
    doc.node_mut(d2).style.margin_top = Length::px(15.0);
    doc.node_mut(d2).style.margin_bottom = Length::px(15.0);
    let d3 = add_colored_block(&mut doc, vp, 80.0, 60.0, Color::BLUE);
    doc.node_mut(d3).style.margin_top = Length::px(15.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 60, 50, RED, "red");
    // Collapsed: d2 at y=95, d3 at y=170, center=200
    assert_pixel_color(&mut s, 60, 125, GREEN, "green");
    assert_pixel_color(&mut s, 60, 200, BLUE, "blue deep");
}

#[test]
fn collapse_blocks_not_empty() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(128, 0, 0, 255));
    doc.node_mut(d1).style.margin_bottom = Length::px(20.0);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(0, 0, 128, 255));
    doc.node_mut(d2).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, MAROON, "maroon block");
    // Collapsed gap=20, d2 at y=90, center=115
    assert_pixel_color(&mut s, 70, 115, NAVY, "navy block deep");
}

#[test]
fn collapse_stacked_four() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let colors = [Color::RED, Color::GREEN, Color::BLUE, Color::from_rgba8(255, 165, 0, 255)];
    let expected = [RED, GREEN, BLUE, ORANGE];
    for &c in &colors {
        let d = add_colored_block(&mut doc, vp, 80.0, 40.0, c);
        doc.node_mut(d).style.margin_bottom = Length::px(10.0);
    }
    let mut s = render(&doc);
    // just check first and last are correct colors at approximate centers
    assert_pixel_color(&mut s, 60, 40, expected[0], "first");
    assert_pixel_color(&mut s, 60, 90, expected[1], "second");
}

#[test]
fn collapse_narrow_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 40.0, 40.0, Color::RED);
    doc.node_mut(d1).style.margin_bottom = Length::px(20.0);
    let d2 = add_colored_block(&mut doc, vp, 40.0, 40.0, Color::BLUE);
    doc.node_mut(d2).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 40, 40, RED, "narrow red");
    // Collapsed gap=20, d2 at y=80, center=100
    assert_pixel_color(&mut s, 40, 100, BLUE, "narrow blue deep");
}

// ═══════════════════════════════════════════════════════════════════════
// ── 7. Background Colors (25 tests) ────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn bg_red_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 120.0, 60.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 80, 50, RED, "solid red bg");
}

#[test]
fn bg_green_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 120.0, 60.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 80, 50, GREEN, "solid green bg");
}

#[test]
fn bg_blue_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 120.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 80, 50, BLUE, "solid blue bg");
}

#[test]
fn bg_black_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 120.0, 60.0, Color::BLACK);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 80, 50, BLACK, "solid black bg");
}

#[test]
fn bg_custom_gray() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 120.0, 60.0, Color::from_rgba8(128, 128, 128, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 80, 50, GRAY, "solid gray bg");
}

#[test]
fn bg_nested_child_over_parent() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    add_colored_block(&mut doc, parent, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // child at (20,20), covers parent partially
    assert_pixel_color(&mut s, 70, 45, BLUE, "child covers parent");
    assert_pixel_color(&mut s, 70, 95, RED, "parent below child");
}

#[test]
fn bg_child_covers_parent_center() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::GREEN);
    add_colored_block(&mut doc, parent, 200.0, 100.0, Color::RED);
    let mut s = render(&doc);
    // child same size, completely covers parent
    assert_pixel_color(&mut s, 120, 70, RED, "child covers parent entirely");
}

#[test]
fn bg_parent_visible_around() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(parent).style.padding_top = Length::px(20.0);
    doc.node_mut(parent).style.padding_left = Length::px(20.0);
    add_colored_block(&mut doc, parent, 80.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // parent padding at (25,25) should be red
    assert_pixel_color(&mut s, 25, 25, RED, "parent visible in padding");
    // child at (40,40)
    assert_pixel_color(&mut s, 80, 60, BLUE, "child bg");
}

#[test]
fn bg_with_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.padding_top = Length::px(20.0);
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    let mut s = render(&doc);
    // padding area shows bg color
    assert_pixel_color(&mut s, 30, 30, GREEN, "bg in padding");
}

#[test]
fn bg_with_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // border at top, bg below
    assert_pixel_color(&mut s, 70, 22, RED, "border");
    assert_pixel_color(&mut s, 70, 50, GREEN, "bg in content");
}

#[test]
fn bg_full_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 25, 45, RED, "full width left");
    assert_pixel_color(&mut s, 400, 45, RED, "full width center");
    assert_pixel_color(&mut s, 775, 45, RED, "full width right");
}

#[test]
fn bg_two_stacked() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 150.0, 40.0, Color::RED);
    add_colored_block(&mut doc, vp, 150.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 95, 40, RED, "first bg");
    assert_pixel_color(&mut s, 95, 80, BLUE, "second bg");
}

#[test]
fn bg_three_stacked() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 35, RED, "first");
    assert_pixel_color(&mut s, 70, 65, GREEN, "second");
    assert_pixel_color(&mut s, 70, 95, BLUE, "third");
}

#[test]
fn bg_dark_gray() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(64, 64, 64, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, (64, 64, 64), "dark gray");
}

#[test]
fn bg_light_gray() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(192, 192, 192, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, SILVER, "light gray");
}

#[test]
fn bg_yellow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(255, 255, 0, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, YELLOW, "yellow");
}

#[test]
fn bg_cyan() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(0, 255, 255, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, CYAN, "cyan");
}

#[test]
fn bg_magenta() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(255, 0, 255, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, MAGENTA, "magenta");
}

#[test]
fn bg_nested_two_levels() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_colored_block(&mut doc, vp, 300.0, 200.0, Color::RED);
    doc.node_mut(outer).style.padding_top = Length::px(30.0);
    doc.node_mut(outer).style.padding_left = Length::px(30.0);
    let inner = add_colored_block(&mut doc, outer, 150.0, 80.0, Color::GREEN);
    doc.node_mut(inner).style.padding_top = Length::px(15.0);
    doc.node_mut(inner).style.padding_left = Length::px(15.0);
    add_colored_block(&mut doc, inner, 60.0, 30.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 25, 25, RED, "outer padding");
    assert_pixel_color(&mut s, 55, 55, GREEN, "inner padding");
    // innermost child at (50+15, 50+15) = (65, 65)
    assert_pixel_color(&mut s, 95, 80, BLUE, "innermost block");
}

#[test]
fn bg_viewport_white() {
    let mut doc = Document::new();
    let _vp = setup_viewport(&mut doc);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 400, 300, WHITE, "viewport is white");
    assert_pixel_color(&mut s, 10, 10, WHITE, "viewport padding area");
}

#[test]
fn bg_center_color() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 300.0, 200.0, Color::from_rgba8(255, 165, 0, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 170, 120, ORANGE, "orange center");
}

#[test]
fn bg_corner_color() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    let mut s = render(&doc);
    // near top-left corner of block
    assert_pixel_color(&mut s, 22, 22, BLUE, "near top-left");
    // near bottom-right: (219, 119)
    assert_pixel_color(&mut s, 217, 117, BLUE, "near bottom-right");
}

#[test]
fn bg_multiple_colors() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 25.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 25.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 100.0, 25.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 100.0, 25.0, Color::from_rgba8(255, 255, 0, 255));
    add_colored_block(&mut doc, vp, 100.0, 25.0, Color::from_rgba8(0, 255, 255, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 32, RED, "1st");
    assert_pixel_color(&mut s, 70, 57, GREEN, "2nd");
    assert_pixel_color(&mut s, 70, 82, BLUE, "3rd");
    assert_pixel_color(&mut s, 70, 107, YELLOW, "4th");
    assert_pixel_color(&mut s, 70, 132, CYAN, "5th");
}

#[test]
fn bg_orange() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(255, 165, 0, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, ORANGE, "orange bg");
}

#[test]
fn bg_navy() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::from_rgba8(0, 0, 128, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 45, NAVY, "navy bg");
}

// ═══════════════════════════════════════════════════════════════════════
// ── 8. Complex Combinations (26 tests) ─────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn combo_margin_border_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.margin_top = Length::px(10.0);
    doc.node_mut(div).style.margin_left = Length::px(10.0);
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 3;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 3;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // box starts at (30, 30) due to margins
    // border top y=[30,32], border left x=[30,32]
    // border away from corner to avoid anti-aliasing
    assert_pixel_color(&mut s, 60, 31, RED, "border top");
    // padding at (33, 33), content at (43, 43)
    assert_pixel_color(&mut s, 38, 38, GREEN, "padding area bg");
    assert_pixel_color(&mut s, 75, 60, GREEN, "content area");
}

#[test]
fn combo_nested_three_levels() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let l1 = add_colored_block(&mut doc, vp, 400.0, 300.0, Color::RED);
    doc.node_mut(l1).style.padding_top = Length::px(20.0);
    doc.node_mut(l1).style.padding_left = Length::px(20.0);
    let l2 = add_colored_block(&mut doc, l1, 200.0, 150.0, Color::GREEN);
    doc.node_mut(l2).style.padding_top = Length::px(15.0);
    doc.node_mut(l2).style.padding_left = Length::px(15.0);
    let l3 = add_colored_block(&mut doc, l2, 80.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 30, 30, RED, "level 1 padding");
    assert_pixel_color(&mut s, 50, 50, GREEN, "level 2 padding");
    // l3 at (20+20+15, 20+20+15) = (55, 55)
    assert_pixel_color(&mut s, 95, 85, BLUE, "level 3 content");
}

#[test]
fn combo_centered_with_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 60.0, Color::WHITE);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    let bc = Color::BLUE;
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 3;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(bc);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 3;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(bc);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 3;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(bc);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 3;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(bc);
    let mut s = render(&doc);
    // total width = 3+200+3=206, centered: x=20+(760-206)/2=297
    // border at x=297, content at x=300
    assert_pixel_color(&mut s, 400, 50, WHITE, "centered content");
}

#[test]
fn combo_centered_with_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 60.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    doc.node_mut(div).style.padding_top = Length::px(15.0);
    doc.node_mut(div).style.padding_left = Length::px(15.0);
    let mut s = render(&doc);
    // total w=15+200+0=215 (only left padding), centered: x=20+(760-215)/2=292
    // or rather, background covers from x=292 for 215px
    assert_pixel_color(&mut s, 400, 50, RED, "centered padded");
}

#[test]
fn combo_full_box_model() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.margin_top = Length::px(10.0);
    doc.node_mut(div).style.margin_left = Length::px(10.0);
    doc.node_mut(div).style.padding_top = Length::px(15.0);
    doc.node_mut(div).style.padding_left = Length::px(15.0);
    doc.node_mut(div).style.padding_right = Length::px(15.0);
    doc.node_mut(div).style.padding_bottom = Length::px(15.0);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // margin shifts to (30, 30)
    // border top at y=[30,34], border left at x=[30,34]
    // padding from (35, 35) for 15px
    // content from (50, 50) for 100x50
    // border away from corner to avoid anti-aliasing
    assert_pixel_color(&mut s, 80, 32, RED, "border top");
    assert_pixel_color(&mut s, 40, 40, GREEN, "padding bg");
    assert_pixel_color(&mut s, 80, 75, GREEN, "content bg");
}

#[test]
fn combo_two_blocks_border_margin() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    doc.node_mut(d1).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(d1).style.border_bottom_width = 5;
    doc.node_mut(d1).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(d1).style.margin_bottom = Length::px(10.0);
    // d1 total h = 40+5=45, then 10px margin
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 70, 40, RED, "first content");
    assert_pixel_color(&mut s, 70, 62, BLACK, "first border");
    // blue at y=20+45+10=75
    assert_pixel_color(&mut s, 70, 95, BLUE, "second block");
}

#[test]
fn combo_nested_centered() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_colored_block(&mut doc, vp, 400.0, 200.0, Color::RED);
    doc.node_mut(outer).style.margin_left = Length::auto();
    doc.node_mut(outer).style.margin_right = Length::auto();
    // outer centered: x=20+(760-400)/2=200
    let inner = add_colored_block(&mut doc, outer, 100.0, 50.0, Color::BLUE);
    doc.node_mut(inner).style.margin_left = Length::auto();
    doc.node_mut(inner).style.margin_right = Length::auto();
    // inner centered in 400px: x=200+(400-100)/2=350
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 400, 45, BLUE, "inner centered");
    assert_pixel_color(&mut s, 250, 100, RED, "outer visible");
}

#[test]
fn combo_border_and_bg() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 150.0, 80.0, Color::from_rgba8(0, 255, 255, 255));
    let bc = Color::from_rgba8(128, 0, 0, 255);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 4;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(bc);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 4;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(bc);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 4;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(bc);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 4;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(bc);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 22, 22, MAROON, "border");
    assert_pixel_color(&mut s, 95, 55, CYAN, "bg inside border");
}

#[test]
fn combo_padding_and_bg_overlap() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(parent).style.padding_top = Length::px(25.0);
    doc.node_mut(parent).style.padding_left = Length::px(25.0);
    let child = add_colored_block(&mut doc, parent, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // parent bg in padding: (25, 25)
    assert_pixel_color(&mut s, 30, 30, RED, "parent padding");
    // child at (45, 45), center (95, 65)
    assert_pixel_color(&mut s, 95, 65, BLUE, "child");
}

#[test]
fn combo_margin_and_bg() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 80.0, Color::GREEN);
    doc.node_mut(div).style.margin_top = Length::px(30.0);
    doc.node_mut(div).style.margin_left = Length::px(40.0);
    let mut s = render(&doc);
    // block at (60, 50), center (160, 90)
    assert_pixel_color(&mut s, 160, 90, GREEN, "shifted green");
    assert_pixel_color(&mut s, 55, 50, WHITE, "margin gap");
}

#[test]
fn combo_all_properties() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 120.0, 60.0, Color::BLUE);
    doc.node_mut(div).style.margin_top = Length::px(10.0);
    doc.node_mut(div).style.margin_left = Length::px(10.0);
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    doc.node_mut(div).style.padding_right = Length::px(10.0);
    doc.node_mut(div).style.padding_bottom = Length::px(10.0);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 2;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 2;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 2;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 2;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // margin→(30,30), border→(30,30)-(31,31), pad→(32,32), content→(42,42)
    assert_pixel_color(&mut s, 30, 30, RED, "border");
    assert_pixel_color(&mut s, 36, 36, BLUE, "padding bg");
}

#[test]
fn combo_wide_block_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 600.0, 60.0, Color::from_rgba8(192, 192, 192, 255));
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 3;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 3;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 300, 21, BLACK, "top border wide");
    assert_pixel_color(&mut s, 300, 55, SILVER, "content wide");
}

#[test]
fn combo_narrow_centered_padded() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 80.0, 40.0, Color::RED);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    let mut s = render(&doc);
    // total w=10+80=90, centered: x=20+(760-90)/2=355
    assert_pixel_color(&mut s, 400, 35, RED, "centered padded narrow");
}

#[test]
fn combo_stacked_with_borders() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    doc.node_mut(d1).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(d1).style.border_bottom_width = 3;
    doc.node_mut(d1).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    let d2 = add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    doc.node_mut(d2).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(d2).style.border_bottom_width = 3;
    doc.node_mut(d2).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    let mut s = render(&doc);
    // d1 content y=[20,49], border y=[50,52]
    // d2 content y=[53,82], border y=[83,85]
    assert_pixel_color(&mut s, 70, 35, RED, "first content");
    assert_pixel_color(&mut s, 70, 51, BLACK, "first border");
    assert_pixel_color(&mut s, 70, 68, BLUE, "second content");
}

#[test]
fn combo_nested_padding_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 300.0, 200.0, Color::RED);
    doc.node_mut(parent).style.padding_top = Length::px(20.0);
    doc.node_mut(parent).style.padding_left = Length::px(20.0);
    let child = add_colored_block(&mut doc, parent, 120.0, 60.0, Color::BLUE);
    doc.node_mut(child).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(child).style.border_top_width = 3;
    doc.node_mut(child).style.border_top_color = StyleColor::Resolved(Color::GREEN);
    doc.node_mut(child).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(child).style.border_left_width = 3;
    doc.node_mut(child).style.border_left_color = StyleColor::Resolved(Color::GREEN);
    let mut s = render(&doc);
    // parent padding at (25, 25) → RED
    assert_pixel_color(&mut s, 25, 25, RED, "parent padding");
    // Check border away from corner to avoid anti-aliasing
    assert_pixel_color(&mut s, 80, 41, GREEN, "child border top");
    // child content at (43, 43)
    assert_pixel_color(&mut s, 100, 70, BLUE, "child content");
}

#[test]
fn combo_percent_width_margin() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::percent(50.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // 50% of 760 = 380, centered: x=20+(760-380)/2=210
    assert_pixel_color(&mut s, 400, 45, RED, "50% centered");
}

#[test]
fn combo_auto_width_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::GREEN);
    doc.node_mut(div).style.padding_left = Length::px(30.0);
    doc.node_mut(div).style.padding_right = Length::px(30.0);
    let mut s = render(&doc);
    // auto width: box fills 760, padding inside, bg covers full width
    assert_pixel_color(&mut s, 25, 45, GREEN, "left padding area");
    assert_pixel_color(&mut s, 775, 45, GREEN, "right padding area");
}

#[test]
fn combo_border_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(div).style.padding_top = Length::px(20.0);
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 5;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    let mut s = render(&doc);
    // border-box: total = 200x100 including border+padding
    // border top 5px, pad 20px, content = 200-5-20=175 wide, 100-5-20=75 tall
    // border away from corner to avoid anti-aliasing
    assert_pixel_color(&mut s, 100, 22, BLACK, "border area");
    assert_pixel_color(&mut s, 30, 30, RED, "padding area");
    // right edge of box should be at x=219
    assert_pixel_color(&mut s, 221, 60, WHITE, "outside border-box");
}

#[test]
fn combo_content_box_explicit() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    doc.node_mut(div).style.box_sizing = BoxSizing::ContentBox;
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    let mut s = render(&doc);
    // content-box: total = 10+200=210 wide, 10+100=110 tall
    // right edge at x=20+210-1=229
    assert_pixel_color(&mut s, 228, 75, BLUE, "inside content-box total");
    assert_pixel_color(&mut s, 231, 75, WHITE, "outside content-box");
}

#[test]
fn combo_multi_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_colored_block(&mut doc, vp, 400.0, 300.0, Color::from_rgba8(192, 192, 192, 255));
    doc.node_mut(parent).style.padding_top = Length::px(10.0);
    doc.node_mut(parent).style.padding_left = Length::px(10.0);
    add_colored_block(&mut doc, parent, 200.0, 50.0, Color::RED);
    add_colored_block(&mut doc, parent, 200.0, 50.0, Color::GREEN);
    add_colored_block(&mut doc, parent, 200.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // children at (30, 30), (30, 80), (30, 130)
    assert_pixel_color(&mut s, 130, 55, RED, "child 1");
    assert_pixel_color(&mut s, 130, 105, GREEN, "child 2");
    assert_pixel_color(&mut s, 130, 155, BLUE, "child 3");
    // parent visible below children
    assert_pixel_color(&mut s, 130, 200, SILVER, "parent below children");
}

#[test]
fn combo_deep_nesting() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let l1 = add_colored_block(&mut doc, vp, 500.0, 400.0, Color::RED);
    doc.node_mut(l1).style.padding_top = Length::px(10.0);
    doc.node_mut(l1).style.padding_left = Length::px(10.0);
    let l2 = add_colored_block(&mut doc, l1, 300.0, 250.0, Color::GREEN);
    doc.node_mut(l2).style.padding_top = Length::px(10.0);
    doc.node_mut(l2).style.padding_left = Length::px(10.0);
    let l3 = add_colored_block(&mut doc, l2, 150.0, 100.0, Color::BLUE);
    doc.node_mut(l3).style.padding_top = Length::px(10.0);
    doc.node_mut(l3).style.padding_left = Length::px(10.0);
    let l4 = add_colored_block(&mut doc, l3, 60.0, 40.0, Color::from_rgba8(255, 255, 0, 255));
    let mut s = render(&doc);
    // l4 at x=20+10+10+10+10=60, y=60
    assert_pixel_color(&mut s, 90, 80, YELLOW, "deepest level");
}

#[test]
fn combo_mixed_sizes() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 50.0, 20.0, Color::RED);
    add_colored_block(&mut doc, vp, 200.0, 80.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 45, 30, RED, "tiny first");
    assert_pixel_color(&mut s, 120, 80, GREEN, "medium second");
    // third at y=20+20+80=120, center y=140
    assert_pixel_color(&mut s, 70, 140, BLUE, "third");
}

#[test]
fn combo_overlapping_bg() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_colored_block(&mut doc, vp, 300.0, 200.0, Color::RED);
    let inner = add_colored_block(&mut doc, outer, 300.0, 100.0, Color::GREEN);
    add_colored_block(&mut doc, inner, 300.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // blue covers green covers red at top
    assert_pixel_color(&mut s, 170, 45, BLUE, "top: blue over green over red");
    assert_pixel_color(&mut s, 170, 95, GREEN, "middle: green over red");
    assert_pixel_color(&mut s, 170, 170, RED, "bottom: just red");
}

#[test]
fn combo_full_page() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    // header
    let h = add_colored_block(&mut doc, vp, 0.0, 60.0, Color::from_rgba8(0, 0, 128, 255));
    // content with margin
    let c = add_colored_block(&mut doc, vp, 0.0, 300.0, Color::WHITE);
    doc.node_mut(c).style.margin_top = Length::px(10.0);
    // footer
    let f = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::from_rgba8(128, 0, 0, 255));
    doc.node_mut(f).style.margin_top = Length::px(10.0);
    let mut s = render(&doc);
    // header at y=20, h=60
    assert_pixel_color(&mut s, 400, 50, NAVY, "header");
    // content at y=20+60+10=90
    assert_pixel_color(&mut s, 400, 240, WHITE, "content");
    // footer at y=90+300+10=400
    assert_pixel_color(&mut s, 400, 420, MAROON, "footer");
}

#[test]
fn combo_three_level_nesting_borders() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let l1 = add_colored_block(&mut doc, vp, 300.0, 200.0, Color::RED);
    doc.node_mut(l1).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(l1).style.border_top_width = 3;
    doc.node_mut(l1).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(l1).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(l1).style.border_left_width = 3;
    doc.node_mut(l1).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    let l2 = add_colored_block(&mut doc, l1, 200.0, 100.0, Color::GREEN);
    doc.node_mut(l2).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(l2).style.border_top_width = 2;
    doc.node_mut(l2).style.border_top_color = StyleColor::Resolved(Color::BLUE);
    doc.node_mut(l2).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(l2).style.border_left_width = 2;
    doc.node_mut(l2).style.border_left_color = StyleColor::Resolved(Color::BLUE);
    let mut s = render(&doc);
    // Check border away from corner to avoid anti-aliasing
    assert_pixel_color(&mut s, 100, 21, BLACK, "l1 border top");
    assert_pixel_color(&mut s, 80, 24, BLUE, "l2 border top");
    assert_pixel_color(&mut s, 80, 60, GREEN, "l2 content");
}

#[test]
fn combo_everything() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 160.0, 80.0, Color::from_rgba8(0, 255, 255, 255));
    doc.node_mut(div).style.margin_top = Length::px(20.0);
    doc.node_mut(div).style.margin_left = Length::auto();
    doc.node_mut(div).style.margin_right = Length::auto();
    doc.node_mut(div).style.padding_top = Length::px(10.0);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    doc.node_mut(div).style.padding_right = Length::px(10.0);
    doc.node_mut(div).style.padding_bottom = Length::px(10.0);
    doc.node_mut(div).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_top_width = 3;
    doc.node_mut(div).style.border_top_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_width = 3;
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_bottom_width = 3;
    doc.node_mut(div).style.border_bottom_color = StyleColor::Resolved(Color::RED);
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_width = 3;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // total w=3+10+160+10+3=186, centered: x=20+(760-186)/2=307
    // margin_top=20: y=40
    // border at (307, 40)
    // content center at approx x=400, y=40+3+10+40=93
    assert_pixel_color(&mut s, 400, 93, CYAN, "everything combined center");
}


// ═══════════════════════════════════════════════════════════════════════
// ── Additional tests ────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn flow_block_below_bottom_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    // below block at y=70 should be white
    assert_pixel_color(&mut s, 70, 71, WHITE, "below block is white");
}

#[test]
fn combo_border_box_vs_content_box_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    // border-box block: total width = 200 including 5px border each side
    let bb = add_colored_block(&mut doc, vp, 200.0, 40.0, Color::RED);
    doc.node_mut(bb).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(bb).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(bb).style.border_left_width = 5;
    doc.node_mut(bb).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(bb).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(bb).style.border_right_width = 5;
    doc.node_mut(bb).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    // content-box block: total width = 200 + 5 + 5 = 210
    let cb = add_colored_block(&mut doc, vp, 200.0, 40.0, Color::BLUE);
    doc.node_mut(cb).style.box_sizing = BoxSizing::ContentBox;
    doc.node_mut(cb).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(cb).style.border_left_width = 5;
    doc.node_mut(cb).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(cb).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(cb).style.border_right_width = 5;
    doc.node_mut(cb).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    let mut s = render(&doc);
    // border-box: right edge at x=219, x=220 is white
    assert_pixel_color(&mut s, 221, 40, WHITE, "bb right outside");
    // content-box: right edge at x=229, check content at x=222
    assert_pixel_color(&mut s, 222, 80, BLUE, "cb right inside");
}

#[test]
fn bg_opacity_renders() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 200.0, 80.0, Color::RED);
    doc.node_mut(div).style.opacity = 0.5;
    let mut s = render(&doc);
    // semi-transparent red over white: ~(255, 128, 128)
    let (r, g, b, _) = get_pixel(&mut s, 120, 60);
    // red channel should still be high, green/blue should be elevated from blending
    assert!(r > 200, "red channel high with opacity: got {}", r);
    assert!(g > 80, "green blended up with opacity: got {}", g);
}

#[test]
fn combo_two_centered_stacked() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let d1 = add_colored_block(&mut doc, vp, 300.0, 50.0, Color::RED);
    doc.node_mut(d1).style.margin_left = Length::auto();
    doc.node_mut(d1).style.margin_right = Length::auto();
    let d2 = add_colored_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    doc.node_mut(d2).style.margin_left = Length::auto();
    doc.node_mut(d2).style.margin_right = Length::auto();
    let mut s = render(&doc);
    // both centered at x=400
    assert_pixel_color(&mut s, 400, 45, RED, "first centered");
    assert_pixel_color(&mut s, 400, 95, BLUE, "second centered");
    // first wider than second: at x=340, first=red, second=white
    assert_pixel_color(&mut s, 340, 45, RED, "first wide edge");
    // d2 width=200 centered at x=300..499, so x=260 is outside d2
    assert_pixel_color(&mut s, 260, 95, WHITE, "second narrow gap");
}

#[test]
fn padding_nested_double() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_colored_block(&mut doc, vp, 300.0, 200.0, Color::RED);
    doc.node_mut(outer).style.padding_top = Length::px(30.0);
    doc.node_mut(outer).style.padding_left = Length::px(30.0);
    let inner = add_colored_block(&mut doc, outer, 120.0, 80.0, Color::GREEN);
    doc.node_mut(inner).style.padding_top = Length::px(20.0);
    doc.node_mut(inner).style.padding_left = Length::px(20.0);
    add_colored_block(&mut doc, inner, 40.0, 20.0, Color::BLUE);
    let mut s = render(&doc);
    // outer padding at (25, 25) → RED
    assert_pixel_color(&mut s, 25, 25, RED, "outer padding");
    // inner padding at (55, 55) → GREEN
    assert_pixel_color(&mut s, 55, 55, GREEN, "inner padding");
    // innermost at (70, 70), center (90, 80) → BLUE
    assert_pixel_color(&mut s, 90, 80, BLUE, "innermost");
}

