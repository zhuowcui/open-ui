//! SP12 Phase I2 — Float Layout Pixel Comparison Tests.
//!
//! Renders float-based layouts and validates pixel output to verify
//! that floated elements are painted at the correct positions.
//!
//! ## Running
//!
//! ```bash
//! cd bindings/rust
//! cargo test --package openui-paint --test sp12_i2_float_pixel_tests
//! ```

use std::path::{Path, PathBuf};

use skia_safe::Surface;

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::Length;
use openui_paint::{render_to_surface, render_to_png};
use openui_style::*;

// ═══════════════════════════════════════════════════════════════════════
// ── Pixel comparison helpers ────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug)]
struct PixelDiff {
    total_pixels: usize,
    mismatched_pixels: usize,
    max_channel_diff: u8,
    avg_channel_diff: f64,
    size_mismatch: bool,
}

impl PixelDiff {
    fn mismatch_percentage(&self) -> f64 {
        if self.total_pixels == 0 { return 0.0; }
        (self.mismatched_pixels as f64 / self.total_pixels as f64) * 100.0
    }
}

fn surface_to_rgba(surface: &mut Surface) -> (u32, u32, Vec<u8>) {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let w = info.width() as u32;
    let h = info.height() as u32;
    let row_bytes = (w * 4) as usize;
    let mut pixels = vec![0u8; (h as usize) * row_bytes];
    image.read_pixels(
        &info, &mut pixels, row_bytes, (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    (w, h, pixels)
}

fn pixel_diff(
    a: &(u32, u32, Vec<u8>),
    b: &(u32, u32, Vec<u8>),
    tolerance: u8,
) -> PixelDiff {
    if a.0 != b.0 || a.1 != b.1 {
        return PixelDiff {
            total_pixels: (a.0 as usize) * (a.1 as usize),
            mismatched_pixels: (a.0 as usize) * (a.1 as usize),
            max_channel_diff: 255,
            avg_channel_diff: 255.0,
            size_mismatch: true,
        };
    }
    let total_pixels = (a.0 as usize) * (a.1 as usize);
    let mut mismatched = 0usize;
    let mut max_diff: u8 = 0;
    let mut sum_diff: u64 = 0;
    for (pa, pb) in a.2.chunks(4).zip(b.2.chunks(4)) {
        let mut pixel_mismatch = false;
        for i in 0..4 {
            let d = (pa[i] as i16 - pb[i] as i16).unsigned_abs() as u8;
            if d > max_diff { max_diff = d; }
            sum_diff += d as u64;
            if d > tolerance { pixel_mismatch = true; }
        }
        if pixel_mismatch { mismatched += 1; }
    }
    let channels = total_pixels * 4;
    PixelDiff {
        total_pixels,
        mismatched_pixels: mismatched,
        max_channel_diff: max_diff,
        avg_channel_diff: if channels > 0 { sum_diff as f64 / channels as f64 } else { 0.0 },
        size_mismatch: false,
    }
}

fn has_visible_content(surface: &mut Surface) -> bool {
    let (_, _, pixels) = surface_to_rgba(surface);
    for chunk in pixels.chunks(4) {
        if chunk.len() == 4 {
            if chunk[0] != 0xFF || chunk[1] != 0xFF || chunk[2] != 0xFF {
                return true;
            }
        }
    }
    false
}

// ═══════════════════════════════════════════════════════════════════════
// ── Pixel sampling helpers ──────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

fn get_pixel(surface: &mut Surface, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = (info.width() * 4) as usize;
    let mut pixels = vec![0u8; row_bytes];
    // Request RGBA8888 explicitly — N32 is BGRA on little-endian platforms
    let single_row_info = skia_safe::ImageInfo::new(
        (info.width(), 1), skia_safe::ColorType::RGBA8888, info.alpha_type(), None,
    );
    image.read_pixels(
        &single_row_info, &mut pixels, row_bytes, (0, y),
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
        dr <= 2 && dg <= 2 && db <= 2,
        "{}: pixel ({},{}) = ({},{},{}) expected ~({},{},{}), diff=({},{},{})",
        msg, x, y, r, g, b, expected.0, expected.1, expected.2, dr, dg, db,
    );
}

// ═══════════════════════════════════════════════════════════════════════
// ── Constants ───────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

const SURFACE_W: i32 = 800;
const SURFACE_H: i32 = 600;
const PAD: i32 = 20;

// Color constants (what we expect to read back as u8 RGBA)
const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 128, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);
const WHITE: (u8, u8, u8) = (255, 255, 255);
const CYAN: (u8, u8, u8) = (0, 255, 255);
const MAGENTA: (u8, u8, u8) = (255, 0, 255);
const YELLOW: (u8, u8, u8) = (255, 255, 0);
const ORANGE: (u8, u8, u8) = (255, 165, 0);

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
    vp
}

fn add_block(doc: &mut Document, parent: NodeId, width_px: f32) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    if width_px > 0.0 {
        doc.node_mut(div).style.width = Length::px(width_px);
    }
    doc.append_child(parent, div);
    div
}

fn add_float_box(
    doc: &mut Document,
    parent: NodeId,
    w: f32, h: f32,
    float_dir: Float,
    color: Color,
) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(w);
    doc.node_mut(div).style.height = Length::px(h);
    doc.node_mut(div).style.float = float_dir;
    doc.node_mut(div).style.background_color = color;
    doc.append_child(parent, div);
    div
}

fn add_colored_block(
    doc: &mut Document,
    parent: NodeId,
    w: f32, h: f32,
    color: Color,
) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    if w > 0.0 {
        doc.node_mut(div).style.width = Length::px(w);
    }
    doc.node_mut(div).style.height = Length::px(h);
    doc.node_mut(div).style.background_color = color;
    doc.append_child(parent, div);
    div
}

fn render(doc: &Document) -> Surface {
    render_to_surface(doc, SURFACE_W, SURFACE_H).expect("render_to_surface failed")
}

fn color_from_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba8(r, g, b, 255)
}

// ═══════════════════════════════════════════════════════════════════════
// §1  Basic Float Left (~20 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn float_left_basic_position() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "red float interior");
    assert_pixel_color(&mut s, PAD + 95, PAD + 95, RED, "red float bottom-right");
}

#[test]
fn float_left_white_after_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 110, PAD + 5, WHITE, "space after float");
}

#[test]
fn float_left_white_below_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 110, WHITE, "space below float");
}

#[test]
fn float_left_top_left_corner() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 60.0, 60.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 1, PAD + 1, BLUE, "blue float top-left");
}

#[test]
fn float_left_bottom_right_corner() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 80.0, 80.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 78, PAD + 78, BLUE, "blue float bottom-right");
}

#[test]
fn float_left_renders_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 150.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "surface should have content");
}

#[test]
fn float_left_small_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 10.0, 10.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "small red float");
}

#[test]
fn float_left_wide_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 400.0, 50.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "wide float start");
    assert_pixel_color(&mut s, PAD + 395, PAD + 25, RED, "wide float end");
}

#[test]
fn float_left_tall_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 50.0, 300.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 25, PAD + 5, BLUE, "tall float top");
    assert_pixel_color(&mut s, PAD + 25, PAD + 295, BLUE, "tall float bottom");
}

#[test]
fn float_left_green_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 120.0, 80.0, Float::Left, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 60, PAD + 40, GREEN, "green float center");
}

#[test]
fn float_left_has_content_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 50.0, 50.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn float_left_precise_right_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    // Just inside right edge of float
    assert_pixel_color(&mut s, PAD + 99, PAD + 50, RED, "inside right edge");
    // Just outside right edge
    assert_pixel_color(&mut s, PAD + 101, PAD + 50, WHITE, "outside right edge");
}

#[test]
fn float_left_precise_bottom_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 99, RED, "inside bottom edge");
    assert_pixel_color(&mut s, PAD + 50, PAD + 101, WHITE, "outside bottom edge");
}

#[test]
fn float_left_white_above() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD - 5, WHITE, "white above viewport padding");
}

#[test]
fn float_left_100x50_area() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, BLUE, "center of 100x50 float");
}

#[test]
fn float_left_custom_color() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, color_from_rgb(255, 255, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 50, YELLOW, "yellow float center");
}

#[test]
fn float_left_200x200_four_corners() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 200.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 2, PAD + 2, RED, "top-left");
    assert_pixel_color(&mut s, PAD + 197, PAD + 2, RED, "top-right");
    assert_pixel_color(&mut s, PAD + 2, PAD + 197, RED, "bottom-left");
    assert_pixel_color(&mut s, PAD + 197, PAD + 197, RED, "bottom-right");
}

#[test]
fn float_left_no_content_outside() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 200, PAD + 200, WHITE, "far from float");
}

#[test]
fn float_left_adjacent_to_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    // At viewport padding boundary
    assert_pixel_color(&mut s, PAD - 1, PAD + 50, WHITE, "left of content area");
    assert_pixel_color(&mut s, PAD + 1, PAD + 50, RED, "just inside content area");
}

#[test]
fn float_left_300x30_horizontal() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 300.0, 30.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 150, PAD + 15, BLUE, "horizontal strip center");
}

// ═══════════════════════════════════════════════════════════════════════
// §2  Basic Float Right (~20 tests)
// ═══════════════════════════════════════════════════════════════════════

// Container content width = SURFACE_W - 2*PAD = 760
const CONTENT_W: i32 = SURFACE_W - 2 * PAD;

#[test]
fn float_right_basic_position() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    // Right float: x = PAD + CONTENT_W - 100 = 20 + 660 = 680
    let right_x = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, right_x + 5, PAD + 5, RED, "right float interior");
    assert_pixel_color(&mut s, right_x + 95, PAD + 50, RED, "right float right side");
}

#[test]
fn float_right_white_to_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let right_x = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, right_x - 10, PAD + 50, WHITE, "white left of right float");
}

#[test]
fn float_right_white_below() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let right_x = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, right_x + 50, PAD + 110, WHITE, "white below right float");
}

#[test]
fn float_right_left_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 80.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 200;
    assert_pixel_color(&mut s, rx + 1, PAD + 40, BLUE, "right float left edge");
    assert_pixel_color(&mut s, rx - 1, PAD + 40, WHITE, "before right float");
}

#[test]
fn float_right_wide_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 500.0, 50.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 500;
    assert_pixel_color(&mut s, rx + 10, PAD + 25, RED, "wide right float left");
    assert_pixel_color(&mut s, rx + 490, PAD + 25, RED, "wide right float right");
}

#[test]
fn float_right_small_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 20.0, 20.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 20;
    assert_pixel_color(&mut s, rx + 10, PAD + 10, RED, "small right float");
}

#[test]
fn float_right_tall_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 50.0, 300.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 50;
    assert_pixel_color(&mut s, rx + 25, PAD + 5, BLUE, "tall right float top");
    assert_pixel_color(&mut s, rx + 25, PAD + 295, BLUE, "tall right float bottom");
}

#[test]
fn float_right_green() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Right, Color::GREEN);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD + 40, GREEN, "green right float");
}

#[test]
fn float_right_has_visible_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn float_right_precise_left_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx, PAD + 50, RED, "at left edge of right float");
    assert_pixel_color(&mut s, rx - 2, PAD + 50, WHITE, "just outside left edge");
}

#[test]
fn float_right_precise_bottom() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD + 99, RED, "inside bottom");
    assert_pixel_color(&mut s, rx + 50, PAD + 101, WHITE, "below bottom");
}

#[test]
fn float_right_full_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, CONTENT_W as f32, 50.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 25, RED, "full width right float left");
    assert_pixel_color(&mut s, PAD + CONTENT_W - 5, PAD + 25, RED, "full width right float right");
}

#[test]
fn float_right_150x150_center() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 150.0, 150.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 150;
    assert_pixel_color(&mut s, rx + 75, PAD + 75, BLUE, "center of right float");
}

#[test]
fn float_right_custom_color() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, color_from_rgb(0, 255, 255));
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD + 50, CYAN, "cyan right float");
}

#[test]
fn float_right_no_content_at_left_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 50, WHITE, "left side is white");
}

#[test]
fn float_right_four_corners() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 200.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 200;
    assert_pixel_color(&mut s, rx + 2, PAD + 2, RED, "top-left");
    assert_pixel_color(&mut s, rx + 197, PAD + 2, RED, "top-right");
    assert_pixel_color(&mut s, rx + 2, PAD + 197, RED, "bottom-left");
    assert_pixel_color(&mut s, rx + 197, PAD + 197, RED, "bottom-right");
}

#[test]
fn float_right_300x30_strip() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 300.0, 30.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 300;
    assert_pixel_color(&mut s, rx + 150, PAD + 15, BLUE, "center of strip");
}

#[test]
fn float_right_white_above() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD - 5, WHITE, "white above right float");
}

#[test]
fn float_right_adjacent_to_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    let right_pad = PAD + CONTENT_W;
    assert_pixel_color(&mut s, right_pad - 1, PAD + 50, RED, "just inside right edge");
    assert_pixel_color(&mut s, right_pad + 1, PAD + 50, WHITE, "right padding area");
}

#[test]
fn float_right_250x80_dimensions() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 250.0, 80.0, Float::Right, Color::GREEN);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 250;
    assert_pixel_color(&mut s, rx + 125, PAD + 40, GREEN, "center of 250x80");
}

// ═══════════════════════════════════════════════════════════════════════
// §3  Two Floats Side by Side (~20 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn two_left_floats_side_by_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 50, RED, "first left float");
    assert_pixel_color(&mut s, PAD + 150, PAD + 50, BLUE, "second left float");
}

#[test]
fn two_left_floats_boundary() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 99, PAD + 50, RED, "red right edge");
    assert_pixel_color(&mut s, PAD + 100, PAD + 50, BLUE, "blue left edge");
}

#[test]
fn two_left_floats_different_widths() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 80.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 150.0, 80.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 100, PAD + 40, RED, "first wider float");
    assert_pixel_color(&mut s, PAD + 275, PAD + 40, BLUE, "second narrower float");
}

#[test]
fn two_left_floats_white_after_both() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 210, PAD + 50, WHITE, "white after both floats");
}

#[test]
fn three_left_floats_side_by_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::GREEN);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 30, RED, "first of three");
    assert_pixel_color(&mut s, PAD + 150, PAD + 30, GREEN, "second of three");
    assert_pixel_color(&mut s, PAD + 250, PAD + 30, BLUE, "third of three");
}

#[test]
fn two_right_floats_side_by_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    // First right float at right edge, second to its left
    let rx1 = PAD + CONTENT_W - 100;
    let rx2 = PAD + CONTENT_W - 200;
    assert_pixel_color(&mut s, rx1 + 50, PAD + 50, RED, "first right float");
    assert_pixel_color(&mut s, rx2 + 50, PAD + 50, BLUE, "second right float");
}

#[test]
fn two_right_floats_boundary() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx2 = PAD + CONTENT_W - 200;
    assert_pixel_color(&mut s, rx2 + 99, PAD + 50, BLUE, "blue right edge");
    assert_pixel_color(&mut s, rx2 + 100, PAD + 50, RED, "red left edge");
}

#[test]
fn left_and_right_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 50, RED, "left float");
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD + 50, BLUE, "right float");
}

#[test]
fn left_and_right_white_between() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 380, PAD + 50, WHITE, "between L and R floats");
}

#[test]
fn two_left_different_heights() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 150.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 100, RED, "red is taller");
    assert_pixel_color(&mut s, PAD + 150, PAD + 100, WHITE, "blue shorter, white below");
}

#[test]
fn three_right_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 80.0, 60.0, Float::Right, Color::RED);
    add_float_box(&mut doc, vp, 80.0, 60.0, Float::Right, Color::GREEN);
    add_float_box(&mut doc, vp, 80.0, 60.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx1 = PAD + CONTENT_W - 80;
    let rx2 = PAD + CONTENT_W - 160;
    let rx3 = PAD + CONTENT_W - 240;
    assert_pixel_color(&mut s, rx1 + 40, PAD + 30, RED, "first right");
    assert_pixel_color(&mut s, rx2 + 40, PAD + 30, GREEN, "second right");
    assert_pixel_color(&mut s, rx3 + 40, PAD + 30, BLUE, "third right");
}

#[test]
fn interleaved_left_right_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Right, Color::BLUE);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, RED, "first left");
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD + 25, BLUE, "first right");
    assert_pixel_color(&mut s, PAD + 150, PAD + 25, GREEN, "second left");
}

#[test]
fn two_left_200px_each() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "first 200px float");
    assert_pixel_color(&mut s, PAD + 205, PAD + 5, BLUE, "second 200px float");
}

#[test]
fn two_left_narrow_gap_check() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 150.0, 60.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 150.0, 60.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 148, PAD + 30, RED, "red end");
    assert_pixel_color(&mut s, PAD + 152, PAD + 30, BLUE, "blue start");
}

#[test]
fn four_left_floats_row() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 40.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 40.0, Float::Left, Color::GREEN);
    add_float_box(&mut doc, vp, 100.0, 40.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, vp, 100.0, 40.0, Float::Left, color_from_rgb(255, 255, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 20, RED, "first");
    assert_pixel_color(&mut s, PAD + 150, PAD + 20, GREEN, "second");
    assert_pixel_color(&mut s, PAD + 250, PAD + 20, BLUE, "third");
    assert_pixel_color(&mut s, PAD + 350, PAD + 20, YELLOW, "fourth");
}

#[test]
fn left_and_right_large() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 300.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 300.0, 100.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 150, PAD + 50, RED, "large left float");
    let rx = PAD + CONTENT_W - 300;
    assert_pixel_color(&mut s, rx + 150, PAD + 50, BLUE, "large right float");
}

#[test]
fn left_and_right_different_heights() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 200.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    // Red extends below blue
    assert_pixel_color(&mut s, PAD + 50, PAD + 150, RED, "red extends down");
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD + 100, WHITE, "below blue float");
}

#[test]
fn two_left_then_white_gap() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // Verify white gap between float area and right edge
    assert_pixel_color(&mut s, PAD + 400, PAD + 25, WHITE, "wide gap");
}

// ═══════════════════════════════════════════════════════════════════════
// §4  Float Wrapping (~20 tests)
// ═══════════════════════════════════════════════════════════════════════

fn setup_container(doc: &mut Document, container_w: f32) -> NodeId {
    let vp = setup_viewport(doc);
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(container_w);
    doc.node_mut(c).style.height = Length::px(400.0);
    doc.append_child(vp, c);
    c
}

#[test]
fn float_wrap_second_drops_below() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 300.0);
    add_float_box(&mut doc, c, 200.0, 50.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 200.0, 50.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 100, PAD + 25, RED, "first float");
    // Second drops below first (200+200=400 > 300)
    assert_pixel_color(&mut s, PAD + 100, PAD + 75, BLUE, "wrapped second float");
}

#[test]
fn float_wrap_three_floats_two_rows() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 250.0);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::GREEN);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 20, RED, "row1 first");
    assert_pixel_color(&mut s, PAD + 150, PAD + 20, GREEN, "row1 second");
    // Third wraps to row 2
    assert_pixel_color(&mut s, PAD + 50, PAD + 60, BLUE, "row2 first");
}

#[test]
fn float_wrap_right_drops_below() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 300.0);
    add_float_box(&mut doc, c, 200.0, 50.0, Float::Right, Color::RED);
    add_float_box(&mut doc, c, 200.0, 50.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx1 = PAD + 100; // 300 - 200 = 100
    assert_pixel_color(&mut s, rx1 + PAD - PAD + 100 + PAD, PAD + 25, RED, "first right float");
}

#[test]
fn float_wrap_exact_fit_no_wrap() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 200.0);
    add_float_box(&mut doc, c, 100.0, 50.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 100.0, 50.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // Exactly fits (100+100 = 200)
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, RED, "fits: first");
    assert_pixel_color(&mut s, PAD + 150, PAD + 25, BLUE, "fits: second");
}

#[test]
fn float_wrap_four_in_two_rows() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 200.0);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::GREEN);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, color_from_rgb(255, 255, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 20, RED, "row1 left");
    assert_pixel_color(&mut s, PAD + 150, PAD + 20, BLUE, "row1 right");
    assert_pixel_color(&mut s, PAD + 50, PAD + 60, GREEN, "row2 left");
    assert_pixel_color(&mut s, PAD + 150, PAD + 60, YELLOW, "row2 right");
}

#[test]
fn float_wrap_first_row_has_content() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 300.0);
    add_float_box(&mut doc, c, 200.0, 80.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 200.0, 80.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn float_wrap_six_in_two_rows() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 300.0);
    for _ in 0..3 {
        add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::RED);
    }
    for _ in 0..3 {
        add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::BLUE);
    }
    let mut s = render(&doc);
    // Row 1: 3 red
    assert_pixel_color(&mut s, PAD + 50, PAD + 20, RED, "row1");
    // Row 2: 3 blue
    assert_pixel_color(&mut s, PAD + 50, PAD + 60, BLUE, "row2");
}

#[test]
fn float_wrap_single_oversized() {
    // Single float wider than container still renders at left edge
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 100.0);
    add_float_box(&mut doc, c, 200.0, 50.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, RED, "oversized float still at left");
}

#[test]
fn float_wrap_different_heights_row1() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 250.0);
    add_float_box(&mut doc, c, 100.0, 80.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // Both on row 1
    assert_pixel_color(&mut s, PAD + 50, PAD + 20, RED, "taller float");
    assert_pixel_color(&mut s, PAD + 150, PAD + 20, BLUE, "shorter float");
}

#[test]
fn float_wrap_large_then_small() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 300.0);
    add_float_box(&mut doc, c, 250.0, 60.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 50.0, 60.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // 250+50=300, exactly fits
    assert_pixel_color(&mut s, PAD + 125, PAD + 30, RED, "large float");
    assert_pixel_color(&mut s, PAD + 275, PAD + 30, BLUE, "small float");
}

#[test]
fn float_wrap_white_between_rows() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 150.0);
    add_float_box(&mut doc, c, 100.0, 30.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 100.0, 30.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // First at y=PAD, second wraps to y=PAD+30
    assert_pixel_color(&mut s, PAD + 50, PAD + 15, RED, "row1");
    assert_pixel_color(&mut s, PAD + 50, PAD + 45, BLUE, "row2");
}

#[test]
fn float_wrap_five_in_container() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 300.0);
    for _ in 0..5 {
        add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::RED);
    }
    let mut s = render(&doc);
    // Row 1: 3 floats, Row 2: 2 floats
    assert_pixel_color(&mut s, PAD + 250, PAD + 20, RED, "row1 third");
    assert_pixel_color(&mut s, PAD + 50, PAD + 60, RED, "row2 first");
}

#[test]
fn float_wrap_eight_equal_floats() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 400.0);
    for _ in 0..8 {
        add_float_box(&mut doc, c, 100.0, 30.0, Float::Left, Color::BLUE);
    }
    let mut s = render(&doc);
    // Row 1: 4, Row 2: 4
    assert_pixel_color(&mut s, PAD + 350, PAD + 15, BLUE, "row1 last");
    assert_pixel_color(&mut s, PAD + 350, PAD + 45, BLUE, "row2 last");
}

#[test]
fn float_wrap_wide_container_no_wrap() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 600.0);
    add_float_box(&mut doc, c, 100.0, 50.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 100.0, 50.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, c, 100.0, 50.0, Float::Left, Color::GREEN);
    let mut s = render(&doc);
    // All fit on one row
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, RED, "all same row 1");
    assert_pixel_color(&mut s, PAD + 150, PAD + 25, BLUE, "all same row 2");
    assert_pixel_color(&mut s, PAD + 250, PAD + 25, GREEN, "all same row 3");
}

#[test]
fn float_wrap_one_px_overflow() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 199.0);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 100.0, 40.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // 100+100=200 > 199, second wraps
    assert_pixel_color(&mut s, PAD + 50, PAD + 20, RED, "first row");
    assert_pixel_color(&mut s, PAD + 50, PAD + 60, BLUE, "wrapped row");
}

#[test]
fn float_wrap_mixed_left_right_overflow() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 300.0);
    add_float_box(&mut doc, c, 200.0, 50.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 200.0, 50.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    // L+R = 400 > 300, right may drop
    assert_pixel_color(&mut s, PAD + 100, PAD + 25, RED, "left float stays");
}

#[test]
fn float_wrap_ten_tiny_floats() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 200.0);
    for _ in 0..10 {
        add_float_box(&mut doc, c, 50.0, 20.0, Float::Left, Color::RED);
    }
    let mut s = render(&doc);
    // Row 1: 4 (50*4=200), Row 2: 4, Row 3: 2
    assert_pixel_color(&mut s, PAD + 25, PAD + 10, RED, "row1");
    assert_pixel_color(&mut s, PAD + 25, PAD + 30, RED, "row2");
    assert_pixel_color(&mut s, PAD + 25, PAD + 50, RED, "row3");
}

#[test]
fn float_wrap_produces_visible_content() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 100.0);
    add_float_box(&mut doc, c, 60.0, 30.0, Float::Left, Color::RED);
    add_float_box(&mut doc, c, 60.0, 30.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

// ═══════════════════════════════════════════════════════════════════════
// §5  Clear Left/Right/Both (~20 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn clear_left_moves_below_left_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Left;
    let mut s = render(&doc);
    // Cleared block below float at y >= PAD+100
    assert_pixel_color(&mut s, PAD + 50, PAD + 110, BLUE, "cleared block below float");
}

#[test]
fn clear_left_not_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Left;
    let mut s = render(&doc);
    // At float level, only red
    assert_pixel_color(&mut s, PAD + 50, PAD + 50, RED, "float area is red not blue");
}

#[test]
fn clear_right_moves_below_right_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Right;
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 110, BLUE, "cleared below right float");
}

#[test]
fn clear_both_moves_below_all_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 120.0, Float::Right, Color::GREEN);
    let clr = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Both;
    let mut s = render(&doc);
    // Must be below the taller float (120)
    assert_pixel_color(&mut s, PAD + 50, PAD + 130, BLUE, "below both floats");
}

#[test]
fn clear_both_full_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Both;
    let mut s = render(&doc);
    // Cleared block gets full width
    assert_pixel_color(&mut s, PAD + 5, PAD + 90, BLUE, "cleared block left edge");
    assert_pixel_color(&mut s, PAD + CONTENT_W - 5, PAD + 90, BLUE, "cleared block right edge");
}

#[test]
fn clear_left_no_left_float_no_effect() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    let blk = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(blk).style.clear = Clear::Left;
    let mut s = render(&doc);
    // No left float to clear, block beside right float at top
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, BLUE, "no left float, block at top");
}

#[test]
fn clear_right_no_right_float_no_effect() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let blk = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(blk).style.clear = Clear::Right;
    let mut s = render(&doc);
    // No right float to clear, block flows beside left float
    assert_pixel_color(&mut s, PAD + 150, PAD + 25, BLUE, "no right float, block beside left");
}

#[test]
fn clear_both_two_left_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::GREEN);
    let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Both;
    let mut s = render(&doc);
    // Below the taller float (80)
    assert_pixel_color(&mut s, PAD + 50, PAD + 90, BLUE, "below both left floats");
}

#[test]
fn clear_left_then_another_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 30.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Left;
    add_colored_block(&mut doc, vp, 0.0, 30.0, Color::GREEN);
    let mut s = render(&doc);
    // Blue at y=60, green at y=90
    assert_pixel_color(&mut s, PAD + 50, PAD + 70, BLUE, "cleared block");
    assert_pixel_color(&mut s, PAD + 50, PAD + 100, GREEN, "block after cleared");
}

#[test]
fn clear_none_stays_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    let blk = add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(blk).style.clear = Clear::None;
    let mut s = render(&doc);
    // Block beside float (clear:none is default)
    assert_pixel_color(&mut s, PAD + 150, PAD + 25, BLUE, "block beside float");
}

#[test]
fn clear_both_produces_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Both;
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn clear_left_with_tall_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 200.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Left;
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 210, BLUE, "below tall float");
}

#[test]
fn clear_right_with_tall_right_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 200.0, Float::Right, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Right;
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 210, BLUE, "below tall right float");
}

#[test]
fn clear_both_asymmetric_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 150.0, Float::Right, Color::GREEN);
    let clr = add_colored_block(&mut doc, vp, 0.0, 30.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Both;
    let mut s = render(&doc);
    // Must be below the taller (right) float
    assert_pixel_color(&mut s, PAD + 50, PAD + 160, BLUE, "below taller right float");
}

#[test]
fn clear_left_white_at_float_level() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Left;
    let mut s = render(&doc);
    // At float level, right side should be white (no blue block there)
    assert_pixel_color(&mut s, PAD + 200, PAD + 40, WHITE, "white at float level");
}

#[test]
fn clear_both_float_then_clear_then_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 30.0, Color::GREEN);
    doc.node_mut(clr).style.clear = Clear::Both;
    add_float_box(&mut doc, vp, 100.0, 40.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // Red at top, green below red, blue below green
    assert_pixel_color(&mut s, PAD + 50, PAD + 30, RED, "first float");
    assert_pixel_color(&mut s, PAD + 50, PAD + 70, GREEN, "cleared block");
    assert_pixel_color(&mut s, PAD + 50, PAD + 100, BLUE, "float after clear");
}

#[test]
fn clear_left_multiple_left_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 40.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::GREEN);
    let clr = add_colored_block(&mut doc, vp, 0.0, 30.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Left;
    let mut s = render(&doc);
    // Below the taller left float
    assert_pixel_color(&mut s, PAD + 50, PAD + 70, BLUE, "below taller left float");
}

#[test]
fn clear_both_with_three_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 40.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::GREEN);
    add_float_box(&mut doc, vp, 100.0, 60.0, Float::Right, Color::BLUE);
    let clr = add_colored_block(&mut doc, vp, 0.0, 30.0, color_from_rgb(255, 255, 0));
    doc.node_mut(clr).style.clear = Clear::Both;
    let mut s = render(&doc);
    // Below tallest (80)
    assert_pixel_color(&mut s, PAD + 50, PAD + 90, YELLOW, "below all three floats");
}

// ═══════════════════════════════════════════════════════════════════════
// §6  Float with Content Wrapping (~15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn content_wrap_block_beside_left_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Block flows beside float
    assert_pixel_color(&mut s, PAD + 250, PAD + 25, BLUE, "block beside left float");
}

#[test]
fn content_wrap_block_beside_right_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Right, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, BLUE, "block beside right float");
}

#[test]
fn content_wrap_multiple_blocks_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 200.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 250, PAD + 25, BLUE, "first block beside float");
    assert_pixel_color(&mut s, PAD + 250, PAD + 75, GREEN, "second block beside float");
}

#[test]
fn content_wrap_block_reduced_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Blue block should start at x=PAD+200 and end before right padding
    assert_pixel_color(&mut s, PAD + 200, PAD + 25, BLUE, "block starts at float edge");
    assert_pixel_color(&mut s, PAD + CONTENT_W - 5, PAD + 25, BLUE, "block extends to right");
}

#[test]
fn content_wrap_block_between_two_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 150.0, 100.0, Float::Right, Color::GREEN);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Block between floats: x from 200 to CONTENT_W-150
    assert_pixel_color(&mut s, PAD + 350, PAD + 25, BLUE, "block between floats");
}

#[test]
fn content_wrap_block_not_at_float_position() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Blue should not overlap red
    assert_pixel_color(&mut s, PAD + 100, PAD + 25, RED, "float area stays red");
}

#[test]
fn content_wrap_block_after_float_expires() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 50.0, Float::Left, Color::RED);
    // First block beside float
    add_colored_block(&mut doc, vp, 0.0, 30.0, Color::BLUE);
    // Second block also beside float (30 < 50)
    add_colored_block(&mut doc, vp, 0.0, 30.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 250, PAD + 15, BLUE, "first beside float");
    assert_pixel_color(&mut s, PAD + 250, PAD + 45, GREEN, "second beside float");
}

#[test]
fn content_wrap_block_before_float_unaffected() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    // Block before float at top, full width
    assert_pixel_color(&mut s, PAD + 5, PAD + 20, BLUE, "block before float left");
    assert_pixel_color(&mut s, PAD + CONTENT_W - 5, PAD + 20, BLUE, "block before float right");
}

#[test]
fn content_wrap_float_then_block_then_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 0.0, 40.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 150, PAD + 20, BLUE, "first block");
    assert_pixel_color(&mut s, PAD + 150, PAD + 60, GREEN, "second block");
}

#[test]
fn content_wrap_right_float_block_at_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 300.0, 100.0, Float::Right, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Block starts at left edge
    assert_pixel_color(&mut s, PAD + 5, PAD + 25, BLUE, "block at left edge");
}

#[test]
fn content_wrap_produces_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn content_wrap_block_with_fixed_width_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 300.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Fixed width block beside float
    assert_pixel_color(&mut s, PAD + 250, PAD + 25, BLUE, "fixed width block beside float");
}

#[test]
fn content_wrap_three_blocks_stacked_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 200.0, Float::Left, Color::RED);
    add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 0.0, 40.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 0.0, 40.0, color_from_rgb(255, 255, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 250, PAD + 20, BLUE, "block 1");
    assert_pixel_color(&mut s, PAD + 250, PAD + 60, GREEN, "block 2");
    assert_pixel_color(&mut s, PAD + 250, PAD + 100, YELLOW, "block 3");
}

#[test]
fn content_wrap_block_narrow_between_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 300.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 300.0, 100.0, Float::Right, Color::GREEN);
    // Remaining space: 760 - 300 - 300 = 160
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 350, PAD + 25, BLUE, "narrow block between floats");
}

// ═══════════════════════════════════════════════════════════════════════
// §7  Float and BFC Interaction (~15 tests)
// ═══════════════════════════════════════════════════════════════════════

fn add_bfc_block(doc: &mut Document, parent: NodeId, w: f32, h: f32, color: Color) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    if w > 0.0 {
        doc.node_mut(div).style.width = Length::px(w);
    }
    doc.node_mut(div).style.height = Length::px(h);
    doc.node_mut(div).style.background_color = color;
    doc.node_mut(div).style.overflow_x = Overflow::Hidden;
    doc.node_mut(div).style.overflow_y = Overflow::Hidden;
    doc.append_child(parent, div);
    div
}

#[test]
fn bfc_overflow_hidden_beside_left_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_bfc_block(&mut doc, vp, 300.0, 80.0, Color::BLUE);
    let mut s = render(&doc);
    // BFC block should be beside float, not overlapping
    assert_pixel_color(&mut s, PAD + 100, PAD + 50, RED, "float area is red");
    assert_pixel_color(&mut s, PAD + 250, PAD + 40, BLUE, "BFC block beside float");
}

#[test]
fn bfc_overflow_hidden_beside_right_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Right, Color::RED);
    add_bfc_block(&mut doc, vp, 300.0, 80.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, BLUE, "BFC block beside right float");
}

#[test]
fn bfc_does_not_overlap_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_bfc_block(&mut doc, vp, 400.0, 80.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 150, PAD + 50, RED, "float not covered by BFC");
}

#[test]
fn bfc_between_two_floats() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Right, Color::GREEN);
    add_bfc_block(&mut doc, vp, 300.0, 80.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 350, PAD + 40, BLUE, "BFC between two floats");
}

#[test]
fn bfc_flow_root_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    let fr = doc.create_node(ElementTag::Div);
    doc.node_mut(fr).style.display = Display::FlowRoot;
    doc.node_mut(fr).style.width = Length::px(300.0);
    doc.node_mut(fr).style.height = Length::px(80.0);
    doc.node_mut(fr).style.background_color = Color::BLUE;
    doc.append_child(vp, fr);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 250, PAD + 40, BLUE, "flow-root beside float");
}

#[test]
fn bfc_has_visible_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    add_bfc_block(&mut doc, vp, 300.0, 80.0, Color::BLUE);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn bfc_overflow_hidden_not_affected_by_float_left_position() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 150.0, 100.0, Float::Left, Color::RED);
    add_bfc_block(&mut doc, vp, 200.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    // BFC block placed beside float
    assert_pixel_color(&mut s, PAD + 250, PAD + 30, BLUE, "BFC beside small float");
}

#[test]
fn bfc_overflow_auto_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    let auto_block = doc.create_node(ElementTag::Div);
    doc.node_mut(auto_block).style.display = Display::Block;
    doc.node_mut(auto_block).style.width = Length::px(300.0);
    doc.node_mut(auto_block).style.height = Length::px(80.0);
    doc.node_mut(auto_block).style.background_color = Color::BLUE;
    doc.node_mut(auto_block).style.overflow_x = Overflow::Auto;
    doc.node_mut(auto_block).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, auto_block);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 250, PAD + 40, BLUE, "overflow:auto BFC beside float");
}

#[test]
fn bfc_scroll_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    let scroll_block = doc.create_node(ElementTag::Div);
    doc.node_mut(scroll_block).style.display = Display::Block;
    doc.node_mut(scroll_block).style.width = Length::px(300.0);
    doc.node_mut(scroll_block).style.height = Length::px(80.0);
    doc.node_mut(scroll_block).style.background_color = Color::BLUE;
    doc.node_mut(scroll_block).style.overflow_x = Overflow::Scroll;
    doc.node_mut(scroll_block).style.overflow_y = Overflow::Scroll;
    doc.append_child(vp, scroll_block);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 250, PAD + 40, BLUE, "overflow:scroll BFC beside float");
}

#[test]
fn bfc_small_beside_large_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 400.0, 100.0, Float::Left, Color::RED);
    add_bfc_block(&mut doc, vp, 200.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 500, PAD + 30, BLUE, "small BFC beside large float");
}

#[test]
fn bfc_cleared_below_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    let bfc = add_bfc_block(&mut doc, vp, 0.0, 50.0, Color::BLUE);
    doc.node_mut(bfc).style.clear = Clear::Left;
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 110, BLUE, "BFC cleared below float");
}

#[test]
fn bfc_overflow_hidden_with_float_and_clear() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    let bfc = add_bfc_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(bfc).style.clear = Clear::Both;
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 90, BLUE, "BFC cleared with overflow:hidden");
}

#[test]
fn bfc_inline_block_beside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::RED);
    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(200.0);
    doc.node_mut(ib).style.height = Length::px(60.0);
    doc.node_mut(ib).style.background_color = Color::BLUE;
    doc.append_child(vp, ib);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "inline-block beside float");
}

#[test]
fn bfc_float_inside_overflow_hidden() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = add_bfc_block(&mut doc, vp, 400.0, 200.0, Color::BLUE);
    add_float_box(&mut doc, container, 100.0, 80.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    // Red float inside blue BFC container
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, RED, "float inside BFC");
    assert_pixel_color(&mut s, PAD + 200, PAD + 100, BLUE, "BFC background visible");
}

// ═══════════════════════════════════════════════════════════════════════
// §8  Nested Floats (~10 tests)
// ═══════════════════════════════════════════════════════════════════════

fn add_float_container(
    doc: &mut Document,
    parent: NodeId,
    w: f32, h: f32,
    float_dir: Float,
    color: Color,
) -> NodeId {
    let div = add_float_box(doc, parent, w, h, float_dir, color);
    div
}

#[test]
fn nested_float_inside_floated_container() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 200.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, outer, 100.0, 80.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    // Red inner float at top-left of blue outer
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, RED, "inner float");
    assert_pixel_color(&mut s, PAD + 200, PAD + 100, BLUE, "outer float bg");
}

#[test]
fn nested_two_floats_inside_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 200.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, outer, 100.0, 80.0, Float::Left, Color::RED);
    add_float_box(&mut doc, outer, 100.0, 80.0, Float::Left, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, RED, "first inner float");
    assert_pixel_color(&mut s, PAD + 150, PAD + 40, GREEN, "second inner float");
}

#[test]
fn nested_float_right_inside_float_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 200.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, outer, 100.0, 80.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    // Inner right float at right edge of outer (300-100=200 from outer left)
    assert_pixel_color(&mut s, PAD + 250, PAD + 40, RED, "inner right float");
}

#[test]
fn nested_float_with_sibling() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 200.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, outer, 100.0, 80.0, Float::Left, Color::RED);
    add_float_box(&mut doc, vp, 200.0, 100.0, Float::Left, Color::GREEN);
    let mut s = render(&doc);
    // Green float beside outer (at x=PAD+300)
    assert_pixel_color(&mut s, PAD + 400, PAD + 50, GREEN, "sibling float beside outer");
}

#[test]
fn nested_float_produces_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 200.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, outer, 100.0, 80.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn nested_deeply_nested_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let l1 = add_float_container(&mut doc, vp, 400.0, 300.0, Float::Left, Color::BLUE);
    let l2 = add_float_container(&mut doc, l1, 300.0, 200.0, Float::Left, color_from_rgb(0, 255, 255));
    add_float_box(&mut doc, l2, 100.0, 80.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, RED, "deeply nested float");
}

#[test]
fn nested_two_floated_containers_side_by_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let c1 = add_float_container(&mut doc, vp, 200.0, 150.0, Float::Left, Color::RED);
    let c2 = add_float_container(&mut doc, vp, 200.0, 150.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, c1, 80.0, 60.0, Float::Left, Color::GREEN);
    add_float_box(&mut doc, c2, 80.0, 60.0, Float::Left, color_from_rgb(255, 255, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 40, PAD + 30, GREEN, "inner in first container");
    assert_pixel_color(&mut s, PAD + 240, PAD + 30, YELLOW, "inner in second container");
}

#[test]
fn nested_right_float_container_with_inner_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 200.0, Float::Right, Color::BLUE);
    add_float_box(&mut doc, outer, 100.0, 80.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    let outer_x = PAD + CONTENT_W - 300;
    assert_pixel_color(&mut s, outer_x + 50, PAD + 40, RED, "inner left in right container");
}

#[test]
fn nested_float_with_clear_inside() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 300.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, outer, 100.0, 60.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, outer, 0.0, 40.0, Color::GREEN);
    doc.node_mut(clr).style.clear = Clear::Left;
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 30, RED, "inner float");
    assert_pixel_color(&mut s, PAD + 50, PAD + 70, GREEN, "cleared block inside");
}

#[test]
fn nested_float_outer_bg_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_float_container(&mut doc, vp, 300.0, 200.0, Float::Left, Color::BLUE);
    add_float_box(&mut doc, outer, 50.0, 50.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    // Outer blue visible outside inner red
    assert_pixel_color(&mut s, PAD + 100, PAD + 100, BLUE, "outer bg visible");
}

// ═══════════════════════════════════════════════════════════════════════
// §9  Float with Margins (~10 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn float_margin_left_shifts_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    doc.node_mut(f).style.margin_left = Length::px(30.0);
    let mut s = render(&doc);
    // Float shifted right by margin
    assert_pixel_color(&mut s, PAD + 25, PAD + 50, WHITE, "margin area is white");
    assert_pixel_color(&mut s, PAD + 35, PAD + 50, RED, "float after margin");
}

#[test]
fn float_margin_top_shifts_down() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    doc.node_mut(f).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 10, WHITE, "margin-top area white");
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, RED, "float after margin-top");
}

#[test]
fn float_margin_right_on_left_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f1 = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    doc.node_mut(f1).style.margin_right = Length::px(20.0);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // Second float at 100+20=120
    assert_pixel_color(&mut s, PAD + 50, PAD + 50, RED, "first float");
    assert_pixel_color(&mut s, PAD + 115, PAD + 50, WHITE, "margin gap");
    assert_pixel_color(&mut s, PAD + 125, PAD + 50, BLUE, "second float after gap");
}

#[test]
fn float_margin_right_on_right_float() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::RED);
    doc.node_mut(f).style.margin_right = Length::px(30.0);
    let mut s = render(&doc);
    // Right float pushed left by margin-right
    let rx = PAD + CONTENT_W - 100 - 30;
    assert_pixel_color(&mut s, rx + 50, PAD + 50, RED, "right float with margin-right");
}

#[test]
fn float_all_margins() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    doc.node_mut(f).style.margin_top = Length::px(10.0);
    doc.node_mut(f).style.margin_right = Length::px(10.0);
    doc.node_mut(f).style.margin_bottom = Length::px(10.0);
    doc.node_mut(f).style.margin_left = Length::px(10.0);
    let mut s = render(&doc);
    // Float at (PAD+10, PAD+10), size 100x100
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, WHITE, "margin-left area");
    assert_pixel_color(&mut s, PAD + 15, PAD + 15, RED, "inside float");
}

#[test]
fn float_large_margin_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    doc.node_mut(f).style.margin_left = Length::px(100.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 50, WHITE, "large margin white");
    assert_pixel_color(&mut s, PAD + 150, PAD + 50, RED, "float after large margin");
}

#[test]
fn float_margin_bottom_affects_clear() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    doc.node_mut(f).style.margin_bottom = Length::px(20.0);
    let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
    doc.node_mut(clr).style.clear = Clear::Left;
    let mut s = render(&doc);
    // Clear moves below float + margin-bottom = 80+20=100
    assert_pixel_color(&mut s, PAD + 50, PAD + 110, BLUE, "cleared below float+margin");
}

#[test]
fn float_margin_left_and_margin_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    doc.node_mut(f).style.margin_left = Length::px(20.0);
    doc.node_mut(f).style.margin_right = Length::px(20.0);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    // First at x=PAD+20, second at x=PAD+20+100+20=PAD+140
    assert_pixel_color(&mut s, PAD + 70, PAD + 50, RED, "first with margins");
    assert_pixel_color(&mut s, PAD + 190, PAD + 50, BLUE, "second after gap");
}

#[test]
fn float_margin_produces_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    doc.node_mut(f).style.margin_left = Length::px(50.0);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s));
}

#[test]
fn float_margin_stacking_two_floats_with_margins() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f1 = add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::RED);
    doc.node_mut(f1).style.margin_right = Length::px(10.0);
    let f2 = add_float_box(&mut doc, vp, 100.0, 60.0, Float::Left, Color::BLUE);
    doc.node_mut(f2).style.margin_left = Length::px(10.0);
    let mut s = render(&doc);
    // Gap = 10+10 = 20
    assert_pixel_color(&mut s, PAD + 50, PAD + 30, RED, "first float");
    assert_pixel_color(&mut s, PAD + 110, PAD + 30, WHITE, "gap between");
    assert_pixel_color(&mut s, PAD + 170, PAD + 30, BLUE, "second float");
}

// ═══════════════════════════════════════════════════════════════════════
// §10  Additional Float Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn float_left_padding_on_container() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(400.0);
    doc.node_mut(c).style.height = Length::px(200.0);
    doc.node_mut(c).style.padding_left = Length::px(30.0);
    doc.node_mut(c).style.padding_top = Length::px(15.0);
    doc.node_mut(c).style.background_color = color_from_rgb(200, 200, 200);
    doc.append_child(vp, c);
    add_float_box(&mut doc, c, 100.0, 80.0, Float::Left, Color::RED);
    let mut s = render(&doc);
    // Float inside container respects padding
    assert_pixel_color(&mut s, PAD + 35, PAD + 20, RED, "float inside padded container");
}

#[test]
fn float_right_padding_on_container() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let c = doc.create_node(ElementTag::Div);
    doc.node_mut(c).style.display = Display::Block;
    doc.node_mut(c).style.width = Length::px(400.0);
    doc.node_mut(c).style.height = Length::px(200.0);
    doc.node_mut(c).style.padding_right = Length::px(30.0);
    doc.node_mut(c).style.background_color = color_from_rgb(200, 200, 200);
    doc.append_child(vp, c);
    add_float_box(&mut doc, c, 100.0, 80.0, Float::Right, Color::RED);
    let mut s = render(&doc);
    // Container content-box is 400, float at right edge of content
    let rx = PAD + 300; // 400 content - 100 float = 300
    assert_pixel_color(&mut s, rx + 50, PAD + 40, RED, "right float in padded container");
}

#[test]
fn float_left_then_clear_left_then_float_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 30.0, Color::GREEN);
    doc.node_mut(clr).style.clear = Clear::Left;
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Left, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, RED, "first float");
    assert_pixel_color(&mut s, PAD + 50, PAD + 60, GREEN, "cleared block");
    assert_pixel_color(&mut s, PAD + 50, PAD + 95, BLUE, "float after clear");
}

#[test]
fn float_right_then_clear_right_then_float_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Right, Color::RED);
    let clr = add_colored_block(&mut doc, vp, 0.0, 30.0, Color::GREEN);
    doc.node_mut(clr).style.clear = Clear::Right;
    add_float_box(&mut doc, vp, 100.0, 50.0, Float::Right, Color::BLUE);
    let mut s = render(&doc);
    let rx = PAD + CONTENT_W - 100;
    assert_pixel_color(&mut s, rx + 50, PAD + 25, RED, "first right float");
    assert_pixel_color(&mut s, PAD + 50, PAD + 60, GREEN, "cleared block");
    assert_pixel_color(&mut s, rx + 50, PAD + 95, BLUE, "second right float after clear");
}

#[test]
fn float_left_50pct_width() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 400.0);
    let f = doc.create_node(ElementTag::Div);
    doc.node_mut(f).style.display = Display::Block;
    doc.node_mut(f).style.width = Length::percent(50.0);
    doc.node_mut(f).style.height = Length::px(60.0);
    doc.node_mut(f).style.float = Float::Left;
    doc.node_mut(f).style.background_color = Color::RED;
    doc.append_child(c, f);
    let mut s = render(&doc);
    // 50% of 400 = 200px wide float
    assert_pixel_color(&mut s, PAD + 100, PAD + 30, RED, "50% width float");
    assert_pixel_color(&mut s, PAD + 210, PAD + 30, WHITE, "beyond 50% float");
}

#[test]
fn float_two_50pct_fill_container() {
    let mut doc = Document::new();
    let c = setup_container(&mut doc, 400.0);
    for _ in 0..2 {
        let f = doc.create_node(ElementTag::Div);
        doc.node_mut(f).style.display = Display::Block;
        doc.node_mut(f).style.width = Length::percent(50.0);
        doc.node_mut(f).style.height = Length::px(60.0);
        doc.node_mut(f).style.float = Float::Left;
        doc.node_mut(f).style.background_color = Color::RED;
        doc.append_child(c, f);
    }
    let mut s = render(&doc);
    // Two 50% floats fill container
    assert_pixel_color(&mut s, PAD + 50, PAD + 30, RED, "first 50%");
    assert_pixel_color(&mut s, PAD + 250, PAD + 30, RED, "second 50%");
}

#[test]
fn float_left_with_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    doc.node_mut(f).style.border_top_width = 5;
    doc.node_mut(f).style.border_right_width = 5;
    doc.node_mut(f).style.border_bottom_width = 5;
    doc.node_mut(f).style.border_left_width = 5;
    let mut s = render(&doc);
    // Content area inside border
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, RED, "float with border content");
}

#[test]
fn float_left_with_padding_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let f = add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    doc.node_mut(f).style.padding_top = Length::px(10.0);
    doc.node_mut(f).style.padding_left = Length::px(10.0);
    doc.node_mut(f).style.padding_right = Length::px(10.0);
    doc.node_mut(f).style.padding_bottom = Length::px(10.0);
    let mut s = render(&doc);
    // Padding area is also red (same bg color)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "float padding area");
    assert_pixel_color(&mut s, PAD + 60, PAD + 50, RED, "float content area");
}

// ═══════════════════════════════════════════════════════════════════════
// §11  Aggregate / Smoke Tests
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn all_float_scenarios_produce_output() {
    // Smoke test: build several scenarios and verify they all produce visible pixels
    let scenarios: Vec<(&str, Box<dyn Fn() -> Document>)> = vec![
        ("left_float", Box::new(|| {
            let mut doc = Document::new();
            let vp = setup_viewport(&mut doc);
            add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
            doc
        })),
        ("right_float", Box::new(|| {
            let mut doc = Document::new();
            let vp = setup_viewport(&mut doc);
            add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::BLUE);
            doc
        })),
        ("two_left", Box::new(|| {
            let mut doc = Document::new();
            let vp = setup_viewport(&mut doc);
            add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
            add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::BLUE);
            doc
        })),
        ("left_right", Box::new(|| {
            let mut doc = Document::new();
            let vp = setup_viewport(&mut doc);
            add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
            add_float_box(&mut doc, vp, 100.0, 100.0, Float::Right, Color::BLUE);
            doc
        })),
        ("clear_both", Box::new(|| {
            let mut doc = Document::new();
            let vp = setup_viewport(&mut doc);
            add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
            let clr = add_colored_block(&mut doc, vp, 0.0, 40.0, Color::BLUE);
            doc.node_mut(clr).style.clear = Clear::Both;
            doc
        })),
    ];

    for (name, builder) in scenarios {
        let doc = builder();
        let mut surface = render(&doc);
        assert!(has_visible_content(&mut surface), "{} should produce visible pixels", name);
    }
}
