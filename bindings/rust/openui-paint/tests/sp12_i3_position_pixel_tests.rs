//! SP12 Phase I3 — Positioned Layout Pixel Comparison Tests.
//!
//! Renders positioned layouts (static, relative, absolute, fixed) and
//! validates pixel output to verify that positioned elements are painted
//! at the correct coordinates.
//!
//! ## Running
//!
//! ```bash
//! cd bindings/rust
//! cargo test --package openui-paint --test sp12_i3_position_pixel_tests
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

// Containing block dimensions for root children:
// CB width  = SURFACE_W - 2*PAD = 760 (root content area width)
// CB height = SURFACE_H = 600 (available block size from root constraint)

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

fn add_positioned_block(
    doc: &mut Document,
    parent: NodeId,
    w: f32, h: f32,
    position: Position,
    color: Color,
) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(w);
    doc.node_mut(div).style.height = Length::px(h);
    doc.node_mut(div).style.position = position;
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
// §1  Position Static (~15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn static_single_block_renders_at_content_origin() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "red block interior");
}

#[test]
fn static_block_top_left_corner() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 1, PAD + 1, RED, "near top-left corner");
}

#[test]
fn static_block_bottom_right_corner() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 98, PAD + 48, RED, "near bottom-right corner");
}

#[test]
fn static_block_white_left_of_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD - 1, PAD + 5, WHITE, "left of content area is padding");
}

#[test]
fn static_block_white_above_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD - 1, WHITE, "above content area is padding");
}

#[test]
fn static_block_white_after_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 105, PAD + 5, WHITE, "right of block");
}

#[test]
fn static_block_white_below_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 55, WHITE, "below block");
}

#[test]
fn static_block_ignores_top_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.top = Length::px(30.0);
    let mut s = render(&doc);
    // Static ignores top — block still at (PAD, PAD)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "static ignores top");
}

#[test]
fn static_block_ignores_left_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.left = Length::px(50.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "static ignores left");
}

#[test]
fn static_block_ignores_bottom_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.bottom = Length::px(30.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "static ignores bottom");
}

#[test]
fn static_block_ignores_right_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(div).style.right = Length::px(30.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "static ignores right");
}

#[test]
fn static_two_blocks_stack_vertically() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "first block red");
    assert_pixel_color(&mut s, PAD + 5, PAD + 55, BLUE, "second block blue");
}

#[test]
fn static_three_blocks_stack() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "first red");
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, BLUE, "second blue");
    assert_pixel_color(&mut s, PAD + 5, PAD + 85, GREEN, "third green");
}

#[test]
fn static_block_renders_visible_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "surface should have colored content");
}

#[test]
fn static_default_position_is_static() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    // Don't set position — default should be static
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "default position renders in flow");
}

// ═══════════════════════════════════════════════════════════════════════
// §2  Position Relative — Basic Offsets (~25 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn rel_top_offset_moves_down() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(30.0);
    let mut s = render(&doc);
    // Normal position (PAD, PAD); with top:30 → (PAD, PAD+30)
    assert_pixel_color(&mut s, PAD + 5, PAD + 35, RED, "moved down by top:30");
}

#[test]
fn rel_top_offset_vacated_space_is_white() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(30.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, WHITE, "vacated space is white");
}

#[test]
fn rel_left_offset_moves_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.left = Length::px(40.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD); with left:40 → (PAD+40, PAD)
    assert_pixel_color(&mut s, PAD + 45, PAD + 5, RED, "moved right by left:40");
}

#[test]
fn rel_left_offset_vacated_space() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.left = Length::px(40.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, WHITE, "original position is white");
}

#[test]
fn rel_bottom_offset_moves_up() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    // Place a spacer first so relative can move up into visible area
    add_colored_block(&mut doc, vp, 100.0, 60.0, Color::WHITE);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.bottom = Length::px(20.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD+60); bottom:20 moves up → (PAD, PAD+60-20) = (PAD, PAD+40)
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, BLUE, "moved up by bottom:20");
}

#[test]
fn rel_right_offset_moves_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.right = Length::px(10.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD); right:10 moves left → (PAD-10, PAD) = (10, PAD)
    assert_pixel_color(&mut s, 15, PAD + 5, BLUE, "moved left by right:10");
}

#[test]
fn rel_top_left_combined() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(30.0);
    doc.node_mut(div).style.left = Length::px(40.0);
    let mut s = render(&doc);
    // Normal (PAD, PAD) → (PAD+40, PAD+30) = (60, 50)
    assert_pixel_color(&mut s, 65, 55, RED, "top+left combined");
}

#[test]
fn rel_top_left_combined_bottom_right_corner() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(30.0);
    doc.node_mut(div).style.left = Length::px(40.0);
    let mut s = render(&doc);
    // Block at (60, 50), size 100x50 → bottom-right near (159, 99)
    assert_pixel_color(&mut s, 158, 98, RED, "bottom-right of offset block");
}

#[test]
fn rel_negative_top_moves_up() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::WHITE);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(-20.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD+40); top:-20 → (PAD, PAD+20)
    assert_pixel_color(&mut s, PAD + 5, PAD + 25, GREEN, "negative top moves up");
}

#[test]
fn rel_negative_left_moves_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::GREEN);
    doc.node_mut(div).style.left = Length::px(-10.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD); left:-10 → (PAD-10, PAD) = (10, 20)
    assert_pixel_color(&mut s, 15, PAD + 5, GREEN, "negative left moves left");
}

#[test]
fn rel_does_not_affect_next_sibling() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(100.0);
    doc.node_mut(div).style.left = Length::px(100.0);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Red is visually at (PAD+100, PAD+100) but blue sibling at (PAD, PAD+50)
    assert_pixel_color(&mut s, PAD + 5, PAD + 55, BLUE, "sibling ignores relative offset");
}

#[test]
fn rel_does_not_affect_previous_sibling() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(-60.0);
    let mut s = render(&doc);
    // Blue at (PAD, PAD); red normal at (PAD, PAD+50), moved to (PAD, PAD-10)
    // Blue should still be at its position
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, BLUE, "prev sibling unaffected");
}

#[test]
fn rel_zero_offsets_same_as_static() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(0.0);
    doc.node_mut(div).style.left = Length::px(0.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "zero offset same as static");
}

#[test]
fn rel_large_top_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(200.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 205, RED, "large top offset");
}

#[test]
fn rel_large_left_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.left = Length::px(300.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 305, PAD + 5, RED, "large left offset");
}

#[test]
fn rel_top_and_bottom_top_wins() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(25.0);
    doc.node_mut(div).style.bottom = Length::px(100.0);
    let mut s = render(&doc);
    // top wins: moves down by 25
    assert_pixel_color(&mut s, PAD + 5, PAD + 30, RED, "top wins over bottom");
}

#[test]
fn rel_left_and_right_left_wins_ltr() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.left = Length::px(35.0);
    doc.node_mut(div).style.right = Length::px(200.0);
    let mut s = render(&doc);
    // LTR: left wins, moves right by 35
    assert_pixel_color(&mut s, PAD + 40, PAD + 5, RED, "left wins over right in LTR");
}

#[test]
fn rel_bottom_only_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 80.0, Color::WHITE);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.bottom = Length::px(30.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD+80); bottom:30 → (PAD, PAD+50)
    assert_pixel_color(&mut s, PAD + 5, PAD + 55, BLUE, "bottom only moves up");
}

#[test]
fn rel_right_only_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.right = Length::px(5.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD); right:5 moves left → (PAD-5, PAD) = (15, 20)
    assert_pixel_color(&mut s, 16, PAD + 5, BLUE, "right only moves left");
}

#[test]
fn rel_small_top_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(1.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 6, RED, "small top:1 offset");
}

#[test]
fn rel_small_left_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.left = Length::px(1.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 6, PAD + 5, RED, "small left:1 offset");
}

#[test]
fn rel_top_bottom_right_left_only_top_left_applied() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.bottom = Length::px(50.0);
    doc.node_mut(div).style.left = Length::px(15.0);
    doc.node_mut(div).style.right = Length::px(200.0);
    let mut s = render(&doc);
    // top:10, left:15 win → (PAD+15, PAD+10) = (35, 30)
    assert_pixel_color(&mut s, 40, 35, RED, "all four set: top+left win");
}

#[test]
fn rel_second_block_with_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.left = Length::px(20.0);
    let mut s = render(&doc);
    // Blue normal at (PAD, PAD+50); with offsets → (PAD+20, PAD+60) = (40, 80)
    assert_pixel_color(&mut s, 45, 85, BLUE, "second block relative offset");
}

// ═══════════════════════════════════════════════════════════════════════
// §3  Position Relative — Complex (~15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn rel_with_margin_top() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.margin_top = Length::px(10.0);
    doc.node_mut(div).style.top = Length::px(20.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD+10) due to margin; then top:20 → (PAD, PAD+30)
    assert_pixel_color(&mut s, PAD + 5, PAD + 35, RED, "relative + margin-top");
}

#[test]
fn rel_with_margin_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.margin_left = Length::px(15.0);
    doc.node_mut(div).style.left = Length::px(10.0);
    let mut s = render(&doc);
    // Normal at (PAD+15, PAD); then left:10 → (PAD+25, PAD) = (45, 20)
    assert_pixel_color(&mut s, 50, PAD + 5, RED, "relative + margin-left");
}

#[test]
fn rel_nested_relative_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_positioned_block(&mut doc, vp, 200.0, 200.0, Position::Relative, color_from_rgb(200, 200, 200));
    doc.node_mut(outer).style.top = Length::px(10.0);
    doc.node_mut(outer).style.left = Length::px(10.0);

    let inner = add_positioned_block(&mut doc, outer, 80.0, 40.0, Position::Relative, Color::RED);
    doc.node_mut(inner).style.top = Length::px(5.0);
    doc.node_mut(inner).style.left = Length::px(5.0);
    let mut s = render(&doc);
    // Outer at (PAD+10, PAD+10) = (30, 30); inner at (30+5, 30+5) = (35, 35)
    assert_pixel_color(&mut s, 40, 40, RED, "nested relative stacks offsets");
}

#[test]
fn rel_with_explicit_margin_bottom() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.margin_bottom = Length::px(20.0);
    doc.node_mut(div).style.top = Length::px(10.0);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Red at (PAD, PAD+10); blue at (PAD, PAD+50+20) = (PAD, PAD+70)
    assert_pixel_color(&mut s, PAD + 5, PAD + 75, BLUE, "margin-bottom + relative doesn't break stacking");
}

#[test]
fn rel_block_with_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.padding_left = Length::px(10.0);
    doc.node_mut(div).style.top = Length::px(5.0);
    doc.node_mut(div).style.left = Length::px(5.0);
    let mut s = render(&doc);
    // Block at (PAD+5, PAD+5) = (25, 25); padding doesn't affect position
    assert_pixel_color(&mut s, 30, 30, RED, "relative with padding");
}

#[test]
fn rel_two_siblings_both_relative() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(a).style.left = Length::px(20.0);
    let b = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(b).style.left = Length::px(40.0);
    let mut s = render(&doc);
    // Red at (PAD+20, PAD) = (40, 20); blue at (PAD+40, PAD+50) = (60, 70)
    assert_pixel_color(&mut s, 45, PAD + 5, RED, "first relative sibling");
    assert_pixel_color(&mut s, 65, PAD + 55, BLUE, "second relative sibling");
}

#[test]
fn rel_offset_preserves_flow_for_third_sibling() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    let div = add_positioned_block(&mut doc, vp, 100.0, 30.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(200.0);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::GREEN);
    let mut s = render(&doc);
    // Green should be at (PAD, PAD+30+30) = (PAD, PAD+60) regardless of blue's offset
    assert_pixel_color(&mut s, PAD + 5, PAD + 65, GREEN, "third sibling after relative");
}

#[test]
fn rel_large_negative_top() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 100.0, Color::WHITE);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(-80.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD+100); top:-80 → (PAD, PAD+20)
    assert_pixel_color(&mut s, PAD + 5, PAD + 25, RED, "large negative top");
}

#[test]
fn rel_combined_negative_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::WHITE);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(-30.0);
    doc.node_mut(div).style.left = Length::px(-10.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD+50); offsets → (PAD-10, PAD+20) = (10, 40)
    assert_pixel_color(&mut s, 15, 45, BLUE, "combined negative offsets");
}

#[test]
fn rel_sibling_after_relative_block_correct_y() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 40.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(50.0);
    add_colored_block(&mut doc, vp, 80.0, 30.0, Color::GREEN);
    let mut s = render(&doc);
    // Green at (PAD, PAD+40) — red's original space (40px) is preserved
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, GREEN, "sibling y accounts for original height");
}

#[test]
fn rel_with_different_width_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 200.0, 40.0, Position::Relative, Color::RED);
    doc.node_mut(a).style.left = Length::px(50.0);
    let b = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Relative, Color::BLUE);
    doc.node_mut(b).style.left = Length::px(10.0);
    let mut s = render(&doc);
    // Red 200px wide at x=PAD+50=70; blue 80px wide at x=PAD+10=30
    assert_pixel_color(&mut s, 75, PAD + 5, RED, "wide block offset");
    assert_pixel_color(&mut s, 35, PAD + 45, BLUE, "narrow block offset");
}

#[test]
fn rel_percentage_top_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    // 10% of containing block height (600) = 60
    doc.node_mut(div).style.top = Length::percent(10.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD); top:10% of 600 = 60 → (PAD, PAD+60)
    assert_pixel_color(&mut s, PAD + 5, PAD + 65, RED, "percentage top offset");
}

#[test]
fn rel_percentage_left_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    // 10% of containing block width (760) = 76
    doc.node_mut(div).style.left = Length::percent(10.0);
    let mut s = render(&doc);
    // Normal at (PAD, PAD); left:10% of 760 = 76 → (PAD+76, PAD) = (96, 20)
    assert_pixel_color(&mut s, 101, PAD + 5, RED, "percentage left offset");
}

#[test]
fn rel_auto_offset_no_movement() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    // auto offsets = no movement
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "auto offsets mean no shift");
}

// ═══════════════════════════════════════════════════════════════════════
// §4  Position Absolute — Basic (~25 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn abs_top_left_zero() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(0.0);
    doc.node_mut(div).style.left = Length::px(0.0);
    let mut s = render(&doc);
    // Absolute top:0, left:0 → pixel (0, 0) from parent border-box
    assert_pixel_color(&mut s, 5, 5, BLUE, "abs at surface origin");
}

#[test]
fn abs_top_left_zero_bottom_right() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(0.0);
    doc.node_mut(div).style.left = Length::px(0.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 98, 48, BLUE, "abs bottom-right corner");
}

#[test]
fn abs_with_top_left_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(30.0);
    doc.node_mut(div).style.left = Length::px(40.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 45, 35, GREEN, "abs at (40,30)");
}

#[test]
fn abs_with_right_bottom_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.right = Length::px(0.0);
    doc.node_mut(div).style.bottom = Length::px(0.0);
    let mut s = render(&doc);
    // CB = (760, 600); left = 760-0-100 = 660; top = 600-0-50 = 550
    assert_pixel_color(&mut s, 665, 555, RED, "abs right:0 bottom:0");
}

#[test]
fn abs_right_bottom_edge_check() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.right = Length::px(0.0);
    doc.node_mut(div).style.bottom = Length::px(0.0);
    let mut s = render(&doc);
    // Block from (660,550) to (760,600); just outside
    assert_pixel_color(&mut s, 659, 555, WHITE, "left of abs right:0 block");
}

#[test]
fn abs_removed_from_flow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let abs = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(200.0);
    doc.node_mut(abs).style.left = Length::px(200.0);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Blue at (PAD, PAD) — abs doesn't push it down
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "abs removed from flow, sibling at origin");
}

#[test]
fn abs_removed_from_flow_no_gap() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    let abs = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::GREEN);
    doc.node_mut(abs).style.top = Length::px(300.0);
    doc.node_mut(abs).style.left = Length::px(300.0);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // Red at y=PAD, blue at y=PAD+40 (abs doesn't contribute)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "red before abs");
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, BLUE, "blue after abs, no gap");
}

#[test]
fn abs_explicit_width_height() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 200.0, 100.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(50.0);
    doc.node_mut(div).style.left = Length::px(50.0);
    let mut s = render(&doc);
    // Block from (50, 50) to (250, 150)
    assert_pixel_color(&mut s, 55, 55, RED, "top-left of large abs");
    assert_pixel_color(&mut s, 245, 145, RED, "bottom-right of large abs");
}

#[test]
fn abs_overlapping_normal_flow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let abs = add_positioned_block(&mut doc, vp, 80.0, 30.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(PAD as f32);
    doc.node_mut(abs).style.left = Length::px(PAD as f32);
    let mut s = render(&doc);
    // Abs at (PAD, PAD) overlaps red at (PAD, PAD); abs paints on top
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "abs overlaps normal flow");
}

#[test]
fn abs_top_only_with_left_auto() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(80.0);
    let mut s = render(&doc);
    // top:80, left:auto → uses static position for left = PAD
    assert_pixel_color(&mut s, PAD + 5, 85, RED, "abs top:80 left:auto");
}

#[test]
fn abs_left_only_with_top_auto() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.left = Length::px(100.0);
    let mut s = render(&doc);
    // left:100, top:auto → uses static position for top = PAD
    assert_pixel_color(&mut s, 105, PAD + 5, RED, "abs left:100 top:auto");
}

#[test]
fn abs_top_50_left_50() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 120.0, 60.0, Position::Absolute, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(50.0);
    doc.node_mut(div).style.left = Length::px(50.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 55, 55, GREEN, "abs 50,50 interior");
    assert_pixel_color(&mut s, 165, 105, GREEN, "abs 50,50 far corner");
}

#[test]
fn abs_right_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.right = Length::px(10.0);
    let mut s = render(&doc);
    // left = CB_width - right - width = 760 - 10 - 100 = 650
    assert_pixel_color(&mut s, 655, 15, BLUE, "abs right:10 offset");
}

#[test]
fn abs_bottom_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(div).style.left = Length::px(10.0);
    doc.node_mut(div).style.bottom = Length::px(10.0);
    let mut s = render(&doc);
    // top = CB_height - bottom - height = 600 - 10 - 50 = 540
    assert_pixel_color(&mut s, 15, 545, BLUE, "abs bottom:10 offset");
}

#[test]
fn abs_right_30_bottom_30() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.right = Length::px(30.0);
    doc.node_mut(div).style.bottom = Length::px(30.0);
    let mut s = render(&doc);
    // left = 760-30-80 = 650; top = 600-30-40 = 530
    assert_pixel_color(&mut s, 655, 535, RED, "abs right:30 bottom:30");
}

#[test]
fn abs_white_outside_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(100.0);
    doc.node_mut(div).style.left = Length::px(100.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 99, 105, WHITE, "left of abs block");
    assert_pixel_color(&mut s, 105, 99, WHITE, "above abs block");
    assert_pixel_color(&mut s, 205, 105, WHITE, "right of abs block");
    assert_pixel_color(&mut s, 105, 155, WHITE, "below abs block");
}

#[test]
fn abs_multiple_absolute_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 60.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(10.0);
    doc.node_mut(a).style.left = Length::px(10.0);
    let b = add_positioned_block(&mut doc, vp, 60.0, 40.0, Position::Absolute, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(100.0);
    doc.node_mut(b).style.left = Length::px(100.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 15, 15, RED, "first abs block");
    assert_pixel_color(&mut s, 105, 105, BLUE, "second abs block");
}

#[test]
fn abs_large_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 50.0, 30.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(400.0);
    doc.node_mut(div).style.left = Length::px(500.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 505, 405, RED, "abs with large offsets");
}

#[test]
fn abs_narrow_tall_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 20.0, 200.0, Position::Absolute, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(50.0);
    doc.node_mut(div).style.left = Length::px(50.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 55, 55, GREEN, "narrow tall abs top");
    assert_pixel_color(&mut s, 55, 245, GREEN, "narrow tall abs bottom");
}

#[test]
fn abs_wide_short_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 300.0, 20.0, Position::Absolute, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(50.0);
    doc.node_mut(div).style.left = Length::px(50.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 55, 55, GREEN, "wide short abs left");
    assert_pixel_color(&mut s, 345, 65, GREEN, "wide short abs right");
}

#[test]
fn abs_renders_visible_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 200.0, 100.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(0.0);
    doc.node_mut(div).style.left = Length::px(0.0);
    let mut s = render(&doc);
    assert!(has_visible_content(&mut s), "abs block renders content");
}

#[test]
fn abs_top_10_left_10() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Absolute, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.left = Length::px(10.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 11, 11, BLUE, "abs top-left at (10,10)");
    assert_pixel_color(&mut s, 68, 38, BLUE, "abs bottom-right near edge");
}

// ═══════════════════════════════════════════════════════════════════════
// §5  Position Absolute — Containing Block (~20 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn abs_in_relative_container_top_left_zero() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.background_color = color_from_rgb(200, 200, 200);
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(0.0);
    doc.node_mut(abs).style.left = Length::px(0.0);
    let mut s = render(&doc);
    // Container at (PAD, PAD) = (20, 20); abs at container's origin = (20, 20)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "abs in relative container at origin");
}

#[test]
fn abs_in_relative_container_with_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.background_color = color_from_rgb(200, 200, 200);
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(20.0);
    doc.node_mut(abs).style.left = Length::px(30.0);
    let mut s = render(&doc);
    // Container at (20, 20); abs at (20+30, 20+20) = (50, 40)
    assert_pixel_color(&mut s, 55, 45, RED, "abs in container with offsets");
}

#[test]
fn abs_in_container_right_zero() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 60.0, 30.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(0.0);
    doc.node_mut(abs).style.right = Length::px(0.0);
    let mut s = render(&doc);
    // CB width = 400 (container content width); left = 400 - 0 - 60 = 340
    // Surface: container at (20,20) → abs at (20+340, 20+0) = (360, 20)
    assert_pixel_color(&mut s, 365, 25, BLUE, "abs right-aligned in container");
}

#[test]
fn abs_in_container_bottom_zero() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 60.0, 30.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.left = Length::px(0.0);
    doc.node_mut(abs).style.top = Length::px(250.0);
    let mut s = render(&doc);
    // Container at (20,20); abs at (20+0, 20+250) = (20, 270)
    assert_pixel_color(&mut s, 25, 275, BLUE, "abs near bottom of container");
}

#[test]
fn abs_in_container_right_bottom_corner() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 50.0, 25.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.right = Length::px(0.0);
    doc.node_mut(abs).style.top = Length::px(260.0);
    let mut s = render(&doc);
    // right:0 → left = 400 - 0 - 50 = 350; top:260
    // Surface: (20+350, 20+260) = (370, 280)
    assert_pixel_color(&mut s, 375, 285, RED, "abs near corner of container");
}

#[test]
fn abs_in_container_with_right_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 80.0, 40.0, Position::Absolute, Color::GREEN);
    doc.node_mut(abs).style.top = Length::px(10.0);
    doc.node_mut(abs).style.right = Length::px(20.0);
    let mut s = render(&doc);
    // left = 400 - 20 - 80 = 300; surface: (20+300, 20+10) = (320, 30)
    assert_pixel_color(&mut s, 325, 35, GREEN, "abs right:20 in container");
}

#[test]
fn abs_in_container_centering_auto_margins() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = doc.create_node(ElementTag::Div);
    doc.node_mut(abs).style.display = Display::Block;
    doc.node_mut(abs).style.width = Length::px(200.0);
    doc.node_mut(abs).style.height = Length::px(50.0);
    doc.node_mut(abs).style.position = Position::Absolute;
    doc.node_mut(abs).style.left = Length::px(0.0);
    doc.node_mut(abs).style.right = Length::px(0.0);
    doc.node_mut(abs).style.top = Length::px(0.0);
    doc.node_mut(abs).style.margin_left = Length::auto();
    doc.node_mut(abs).style.margin_right = Length::auto();
    doc.node_mut(abs).style.background_color = Color::RED;
    doc.append_child(container, abs);
    let mut s = render(&doc);
    // Centered: left = (400 - 200)/2 = 100; surface: (20+100, 20+0) = (120, 20)
    assert_pixel_color(&mut s, 125, 25, RED, "abs centered with auto margins");
}

#[test]
fn abs_container_bg_visible_around_abs_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(300.0);
    doc.node_mut(container).style.height = Length::px(200.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.background_color = color_from_rgb(200, 200, 200);
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 50.0, 30.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(50.0);
    doc.node_mut(abs).style.left = Length::px(50.0);
    let mut s = render(&doc);
    // Container bg at (20,20); abs at (70,70)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, (200, 200, 200), "container bg visible");
    assert_pixel_color(&mut s, 75, 75, RED, "abs child in container");
}

#[test]
fn abs_in_container_two_abs_children() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let a = add_positioned_block(&mut doc, container, 60.0, 30.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(10.0);
    doc.node_mut(a).style.left = Length::px(10.0);
    let b = add_positioned_block(&mut doc, container, 60.0, 30.0, Position::Absolute, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(80.0);
    doc.node_mut(b).style.left = Length::px(80.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 15, PAD + 15, RED, "first abs in container");
    assert_pixel_color(&mut s, PAD + 85, PAD + 85, BLUE, "second abs in container");
}

#[test]
fn abs_in_offset_relative_container() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(300.0);
    doc.node_mut(container).style.height = Length::px(200.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.top = Length::px(20.0);
    doc.node_mut(container).style.left = Length::px(30.0);
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 60.0, 30.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(5.0);
    doc.node_mut(abs).style.left = Length::px(5.0);
    let mut s = render(&doc);
    // Container normal at (PAD, PAD), relative moves to (PAD+30, PAD+20) = (50, 40)
    // Abs at (50+5, 40+5) = (55, 45)
    assert_pixel_color(&mut s, 60, 50, RED, "abs in offset relative container");
}

#[test]
fn abs_with_bottom_offset_in_container() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    doc.node_mut(container).style.height = Length::px(300.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 100.0, 40.0, Position::Absolute, Color::GREEN);
    doc.node_mut(abs).style.left = Length::px(10.0);
    doc.node_mut(abs).style.top = Length::px(200.0);
    let mut s = render(&doc);
    // Container at (20,20); abs at (20+10, 20+200) = (30, 220)
    assert_pixel_color(&mut s, 35, 225, GREEN, "abs top:200 in container");
}

#[test]
fn abs_multiple_non_overlapping() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(0.0);
    doc.node_mut(a).style.left = Length::px(0.0);
    let b = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(0.0);
    doc.node_mut(b).style.left = Length::px(200.0);
    let c = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::GREEN);
    doc.node_mut(c).style.top = Length::px(0.0);
    doc.node_mut(c).style.left = Length::px(400.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 50, 25, RED, "first non-overlapping abs");
    assert_pixel_color(&mut s, 250, 25, BLUE, "second non-overlapping abs");
    assert_pixel_color(&mut s, 450, 25, GREEN, "third non-overlapping abs");
}

#[test]
fn abs_in_container_inflow_sibling_at_origin() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(300.0);
    doc.node_mut(container).style.height = Length::px(200.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 60.0, 30.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(100.0);
    doc.node_mut(abs).style.left = Length::px(100.0);
    let _flow = add_colored_block(&mut doc, container, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // In-flow child at container's content origin (20, 20); abs moved away
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "in-flow sibling at container origin");
}

#[test]
fn abs_small_1x1_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 1.0, 1.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(50.0);
    doc.node_mut(div).style.left = Length::px(50.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 50, 50, RED, "1x1 abs block at exact pixel");
}

#[test]
fn abs_right_10_top_10() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.right = Length::px(10.0);
    let mut s = render(&doc);
    // left = 760 - 10 - 100 = 650; top = 10
    assert_pixel_color(&mut s, 700, 30, GREEN, "abs right:10 top:10 interior");
}

// ═══════════════════════════════════════════════════════════════════════
// §6  Position Absolute — Auto Offsets (~15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn abs_auto_offsets_uses_static_position() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    // No offsets set — uses static position = (PAD, PAD) = (20, 20)
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "auto offsets at static pos");
}

#[test]
fn abs_auto_offsets_with_preceding_inflow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    let mut s = render(&doc);
    // Static position: OOF collected during child walk, after blue block (60px).
    // block_offset = PAD + 60 = 80, so abs goes to (PAD, PAD + 60).
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "blue inflow block still at top");
    assert_pixel_color(&mut s, PAD + 5, PAD + 60 + 5, RED, "abs auto offsets below inflow");
}

#[test]
fn abs_only_top_set() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(60.0);
    let mut s = render(&doc);
    // top:60, left:auto → left = static_left = PAD
    assert_pixel_color(&mut s, PAD + 5, 65, RED, "abs only top:60");
}

#[test]
fn abs_only_left_set() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.left = Length::px(70.0);
    let mut s = render(&doc);
    // left:70, top:auto → top = static_top = PAD
    assert_pixel_color(&mut s, 75, PAD + 5, RED, "abs only left:70");
}

#[test]
fn abs_only_right_set() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.right = Length::px(50.0);
    let mut s = render(&doc);
    // right:50, left:auto, width:80 → left = 760 - 50 - 80 = 630
    // top:auto → static_top = PAD
    assert_pixel_color(&mut s, 635, PAD + 5, RED, "abs only right:50");
}

#[test]
fn abs_only_bottom_set() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.bottom = Length::px(50.0);
    let mut s = render(&doc);
    // bottom:50, top:auto → top = 600 - 50 - 40 = 510
    // left:auto → static_left = PAD
    assert_pixel_color(&mut s, PAD + 5, 515, RED, "abs only bottom:50");
}

#[test]
fn abs_auto_offsets_first_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Absolute, Color::BLUE);
    let mut s = render(&doc);
    // First child, static pos = (PAD, PAD)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "first abs child auto offsets");
}

#[test]
fn abs_auto_offsets_white_outside() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Absolute, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 65, PAD + 5, WHITE, "right of auto-positioned abs");
}

#[test]
fn abs_auto_multiple_abs_all_at_static_pos() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Absolute, Color::RED);
    let b = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Absolute, Color::BLUE);
    let mut s = render(&doc);
    // Both collected before layout, both get static_pos (PAD, PAD)
    // Blue painted last, so it's on top
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "later abs on top at static pos");
}

#[test]
fn abs_auto_offset_in_container() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(300.0);
    doc.node_mut(container).style.height = Length::px(200.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 50.0, 25.0, Position::Absolute, Color::GREEN);
    let mut s = render(&doc);
    // Container at (20,20); abs at static pos within container
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, GREEN, "abs auto in container");
}

#[test]
fn abs_top_zero_left_auto() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.top = Length::px(0.0);
    // left auto → static_left = PAD
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, 5, RED, "abs top:0, left auto at static x");
}

#[test]
fn abs_left_zero_top_auto() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(div).style.left = Length::px(0.0);
    // top auto → static_top = PAD
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 5, PAD + 5, RED, "abs left:0, top auto at static y");
}

#[test]
fn abs_auto_offset_different_size() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 200.0, 100.0, Position::Absolute, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 50, GREEN, "large auto-positioned abs");
}

#[test]
fn abs_auto_small_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 10.0, 10.0, Position::Absolute, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "small auto-positioned abs");
    assert_pixel_color(&mut s, PAD + 15, PAD + 5, WHITE, "outside small abs");
}

// ═══════════════════════════════════════════════════════════════════════
// §7  Position Fixed (~15 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn fixed_top_left_zero() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Fixed, Color::RED);
    doc.node_mut(div).style.top = Length::px(0.0);
    doc.node_mut(div).style.left = Length::px(0.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 5, 5, RED, "fixed at surface origin");
}

#[test]
fn fixed_with_top_left_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Fixed, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(30.0);
    doc.node_mut(div).style.left = Length::px(40.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 45, 35, BLUE, "fixed with offsets");
}

#[test]
fn fixed_with_right_bottom_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Fixed, Color::RED);
    doc.node_mut(div).style.right = Length::px(0.0);
    doc.node_mut(div).style.bottom = Length::px(0.0);
    let mut s = render(&doc);
    // Same as absolute: left=660, top=550
    assert_pixel_color(&mut s, 665, 555, RED, "fixed right:0 bottom:0");
}

#[test]
fn fixed_overlapping_normal_flow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 100.0, Color::GREEN);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Fixed, Color::RED);
    doc.node_mut(div).style.top = Length::px(PAD as f32);
    doc.node_mut(div).style.left = Length::px(PAD as f32);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "fixed overlaps normal flow");
}

#[test]
fn fixed_removed_from_flow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let fixed = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Fixed, Color::RED);
    doc.node_mut(fixed).style.top = Length::px(300.0);
    doc.node_mut(fixed).style.left = Length::px(300.0);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Blue at (PAD, PAD) — fixed doesn't push it
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "fixed removed from flow");
}

#[test]
fn fixed_top_10_left_10() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Fixed, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.left = Length::px(10.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 15, 15, GREEN, "fixed top:10 left:10");
}

#[test]
fn fixed_right_20_top_20() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Fixed, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(20.0);
    doc.node_mut(div).style.right = Length::px(20.0);
    let mut s = render(&doc);
    // left = 760 - 20 - 80 = 660; top = 20
    assert_pixel_color(&mut s, 665, 25, BLUE, "fixed right:20 top:20");
}

#[test]
fn fixed_bottom_30_left_30() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Fixed, Color::RED);
    doc.node_mut(div).style.left = Length::px(30.0);
    doc.node_mut(div).style.bottom = Length::px(30.0);
    let mut s = render(&doc);
    // top = 600 - 30 - 40 = 530; left = 30
    assert_pixel_color(&mut s, 35, 535, RED, "fixed bottom:30 left:30");
}

#[test]
fn fixed_auto_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Fixed, Color::RED);
    let mut s = render(&doc);
    // Auto offsets → static pos = (PAD, PAD)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "fixed auto offsets at static pos");
}

#[test]
fn fixed_large_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 50.0, 30.0, Position::Fixed, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(300.0);
    doc.node_mut(div).style.left = Length::px(400.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 405, 305, BLUE, "fixed large offsets");
}

#[test]
fn fixed_white_around_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Fixed, Color::RED);
    doc.node_mut(div).style.top = Length::px(200.0);
    doc.node_mut(div).style.left = Length::px(200.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 199, 225, WHITE, "left of fixed");
    assert_pixel_color(&mut s, 225, 199, WHITE, "above fixed");
    assert_pixel_color(&mut s, 305, 225, WHITE, "right of fixed");
    assert_pixel_color(&mut s, 225, 255, WHITE, "below fixed");
}

#[test]
fn fixed_same_as_absolute_for_viewport() {
    // Fixed and absolute behave the same when parent is viewport
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let abs = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(50.0);
    doc.node_mut(abs).style.left = Length::px(50.0);
    let mut s1 = render(&doc);
    let abs_pixel = get_pixel(&mut s1, 55, 55);

    let mut doc2 = Document::new();
    let vp2 = setup_viewport(&mut doc2);
    let fixed = add_positioned_block(&mut doc2, vp2, 80.0, 40.0, Position::Fixed, Color::RED);
    doc2.node_mut(fixed).style.top = Length::px(50.0);
    doc2.node_mut(fixed).style.left = Length::px(50.0);
    let mut s2 = render(&doc2);
    let fixed_pixel = get_pixel(&mut s2, 55, 55);

    assert_eq!(abs_pixel, fixed_pixel, "fixed == absolute for viewport children");
}

#[test]
fn fixed_multiple_fixed_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Fixed, Color::RED);
    doc.node_mut(a).style.top = Length::px(0.0);
    doc.node_mut(a).style.left = Length::px(0.0);
    let b = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Fixed, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(100.0);
    doc.node_mut(b).style.left = Length::px(100.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 5, 5, RED, "first fixed");
    assert_pixel_color(&mut s, 105, 105, BLUE, "second fixed");
}

#[test]
fn fixed_between_static_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    let fixed = add_positioned_block(&mut doc, vp, 60.0, 30.0, Position::Fixed, Color::GREEN);
    doc.node_mut(fixed).style.top = Length::px(400.0);
    doc.node_mut(fixed).style.left = Length::px(400.0);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // Fixed removed from flow: red at y=PAD, blue at y=PAD+40
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "static before fixed");
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, BLUE, "static after fixed, no gap");
}

// ═══════════════════════════════════════════════════════════════════════
// §8  Stacking and Overlap (~20 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overlap_later_static_block_on_top_via_relative() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 80.0, Color::RED);
    let div = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(-40.0);
    let mut s = render(&doc);
    // Blue normal at (PAD, PAD+80), relative to (PAD, PAD+40)
    // Overlaps red at y=60..90
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, BLUE, "relative on top of static");
}

#[test]
fn overlap_relative_over_static_at_same_y() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let div = add_positioned_block(&mut doc, vp, 80.0, 30.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.top = Length::px(-30.0);
    let mut s = render(&doc);
    // Blue overlaps red near the bottom
    assert_pixel_color(&mut s, PAD + 5, PAD + 25, BLUE, "relative overlaps static from below");
}

#[test]
fn overlap_abs_over_static() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let abs = add_positioned_block(&mut doc, vp, 80.0, 30.0, Position::Absolute, Color::GREEN);
    doc.node_mut(abs).style.top = Length::px(PAD as f32 + 10.0);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 + 10.0);
    let mut s = render(&doc);
    // Abs at (30, 30) overlaps red at (20, 20)
    assert_pixel_color(&mut s, PAD + 15, PAD + 15, GREEN, "abs on top of static");
}

#[test]
fn overlap_abs_over_relative() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let rel = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Relative, Color::RED);
    doc.node_mut(rel).style.left = Length::px(10.0);
    let abs = add_positioned_block(&mut doc, vp, 80.0, 30.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(PAD as f32);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 + 10.0);
    let mut s = render(&doc);
    // Abs on top of relative
    assert_pixel_color(&mut s, PAD + 15, PAD + 5, BLUE, "abs over relative");
}

#[test]
fn overlap_two_abs_source_order() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 100.0, 100.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(10.0);
    doc.node_mut(a).style.left = Length::px(10.0);
    let b = add_positioned_block(&mut doc, vp, 100.0, 100.0, Position::Absolute, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(50.0);
    doc.node_mut(b).style.left = Length::px(50.0);
    let mut s = render(&doc);
    // Non-overlapping area of red
    assert_pixel_color(&mut s, 15, 15, RED, "red in non-overlap area");
    // Overlap area: blue wins (later in source)
    assert_pixel_color(&mut s, 55, 55, BLUE, "blue on top in overlap");
    // Non-overlapping area of blue
    assert_pixel_color(&mut s, 145, 145, BLUE, "blue in non-overlap area");
}

#[test]
fn overlap_three_abs_stacking() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 80.0, 80.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(0.0);
    doc.node_mut(a).style.left = Length::px(0.0);
    let b = add_positioned_block(&mut doc, vp, 80.0, 80.0, Position::Absolute, Color::GREEN);
    doc.node_mut(b).style.top = Length::px(20.0);
    doc.node_mut(b).style.left = Length::px(20.0);
    let c = add_positioned_block(&mut doc, vp, 80.0, 80.0, Position::Absolute, Color::BLUE);
    doc.node_mut(c).style.top = Length::px(40.0);
    doc.node_mut(c).style.left = Length::px(40.0);
    let mut s = render(&doc);
    // At (10, 10): only red
    assert_pixel_color(&mut s, 10, 10, RED, "only red at top-left");
    // At (25, 25): red+green overlap → green wins
    assert_pixel_color(&mut s, 25, 25, GREEN, "green over red");
    // At (45, 45): all three overlap → blue wins
    assert_pixel_color(&mut s, 45, 45, BLUE, "blue over all");
    // At (90, 90): only blue
    assert_pixel_color(&mut s, 90, 90, BLUE, "only blue at bottom-right");
}

#[test]
fn overlap_fixed_over_static() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    let fixed = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Fixed, Color::BLUE);
    doc.node_mut(fixed).style.top = Length::px(PAD as f32);
    doc.node_mut(fixed).style.left = Length::px(PAD as f32);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "fixed over static");
}

#[test]
fn overlap_fixed_over_abs() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let abs = add_positioned_block(&mut doc, vp, 100.0, 100.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(10.0);
    doc.node_mut(abs).style.left = Length::px(10.0);
    let fixed = add_positioned_block(&mut doc, vp, 80.0, 80.0, Position::Fixed, Color::GREEN);
    doc.node_mut(fixed).style.top = Length::px(30.0);
    doc.node_mut(fixed).style.left = Length::px(30.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 15, 15, RED, "abs in non-overlap");
    assert_pixel_color(&mut s, 35, 35, GREEN, "fixed over abs in overlap");
}

#[test]
fn overlap_color_at_exact_boundary() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(0.0);
    doc.node_mut(a).style.left = Length::px(0.0);
    let b = add_positioned_block(&mut doc, vp, 100.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(50.0);
    doc.node_mut(b).style.left = Length::px(0.0);
    let mut s = render(&doc);
    // At y=49: red; at y=50: blue
    assert_pixel_color(&mut s, 50, 49, RED, "red at boundary");
    assert_pixel_color(&mut s, 50, 50, BLUE, "blue at boundary");
}

#[test]
fn overlap_abs_partially_covers_static() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    let abs = add_positioned_block(&mut doc, vp, 50.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(PAD as f32 + 25.0);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 + 75.0);
    let mut s = render(&doc);
    // Red visible outside abs
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "red visible outside overlap");
    // Blue on top inside overlap
    assert_pixel_color(&mut s, PAD + 80, PAD + 30, BLUE, "blue overlapping static");
}

#[test]
fn overlap_multiple_positioned_types() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 150.0, 150.0, Color::RED);
    let rel = add_positioned_block(&mut doc, vp, 100.0, 100.0, Position::Relative, Color::GREEN);
    doc.node_mut(rel).style.top = Length::px(-120.0);
    doc.node_mut(rel).style.left = Length::px(25.0);
    let abs = add_positioned_block(&mut doc, vp, 60.0, 60.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(PAD as f32 + 50.0);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 + 50.0);
    let mut s = render(&doc);
    // At (PAD+5, PAD+5): red (static, painted first)
    // Green overlaps from (PAD+25, PAD+30) region
    // Blue at (70, 70) overlaps both
    assert_pixel_color(&mut s, PAD + 55, PAD + 55, BLUE, "abs on top of all");
}

#[test]
fn overlap_abs_does_not_affect_sibling_position() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    let abs = add_positioned_block(&mut doc, vp, 200.0, 200.0, Position::Absolute, Color::GREEN);
    doc.node_mut(abs).style.top = Length::px(300.0);
    doc.node_mut(abs).style.left = Length::px(300.0);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // Blue at (PAD, PAD+40) even though abs is huge
    assert_pixel_color(&mut s, PAD + 50, PAD + 45, BLUE, "sibling unaffected by large abs");
}

#[test]
fn overlap_rel_over_preceding_static_wider_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 80.0, Color::RED);
    let rel = add_positioned_block(&mut doc, vp, 150.0, 40.0, Position::Relative, Color::BLUE);
    doc.node_mut(rel).style.top = Length::px(-50.0);
    let mut s = render(&doc);
    // Blue overlaps red
    assert_pixel_color(&mut s, PAD + 5, PAD + 35, BLUE, "wide relative over static");
}

#[test]
fn overlap_source_order_two_fixed_blocks() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 100.0, 100.0, Position::Fixed, Color::RED);
    doc.node_mut(a).style.top = Length::px(50.0);
    doc.node_mut(a).style.left = Length::px(50.0);
    let b = add_positioned_block(&mut doc, vp, 100.0, 100.0, Position::Fixed, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(80.0);
    doc.node_mut(b).style.left = Length::px(80.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 55, 55, RED, "first fixed in non-overlap");
    assert_pixel_color(&mut s, 85, 85, BLUE, "second fixed on top in overlap");
    assert_pixel_color(&mut s, 175, 175, BLUE, "second fixed in non-overlap");
}

#[test]
fn overlap_abs_fully_covers_static() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    let abs = add_positioned_block(&mut doc, vp, 200.0, 100.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(PAD as f32 - 5.0);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 - 5.0);
    let mut s = render(&doc);
    // Abs fully covers the static red block
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, BLUE, "abs fully covers static");
}

#[test]
fn overlap_two_relative_blocks_overlapping() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 100.0, 60.0, Position::Relative, Color::RED);
    doc.node_mut(a).style.top = Length::px(0.0);
    let b = add_positioned_block(&mut doc, vp, 100.0, 60.0, Position::Relative, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(-30.0);
    let mut s = render(&doc);
    // A at (PAD, PAD); B normal at (PAD, PAD+60), shifted to (PAD, PAD+30)
    // Overlap from y=PAD+30 to y=PAD+60
    assert_pixel_color(&mut s, PAD + 5, PAD + 35, BLUE, "later relative over earlier");
}

#[test]
fn overlap_abs_and_rel_at_same_pixel() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let rel = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Relative, Color::RED);
    doc.node_mut(rel).style.top = Length::px(0.0);
    doc.node_mut(rel).style.left = Length::px(0.0);
    let abs = add_positioned_block(&mut doc, vp, 80.0, 40.0, Position::Absolute, Color::GREEN);
    doc.node_mut(abs).style.top = Length::px(PAD as f32);
    doc.node_mut(abs).style.left = Length::px(PAD as f32);
    let mut s = render(&doc);
    // Both at same pixel area; abs painted after rel → green on top
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, GREEN, "abs over rel at same pos");
}

#[test]
fn overlap_verify_white_in_non_overlapping_gap() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 50.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(10.0);
    doc.node_mut(a).style.left = Length::px(10.0);
    let b = add_positioned_block(&mut doc, vp, 50.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(70.0);
    doc.node_mut(b).style.left = Length::px(70.0);
    let mut s = render(&doc);
    // Gap between blocks should be white
    assert_pixel_color(&mut s, 65, 65, WHITE, "white gap between abs blocks");
}

// ═══════════════════════════════════════════════════════════════════════
// §9  Additional tests to reach 150+
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn static_block_different_colors() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::GREEN);
    add_colored_block(&mut doc, vp, 100.0, 30.0, Color::BLUE);
    add_colored_block(&mut doc, vp, 100.0, 30.0, color_from_rgb(255, 255, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "1st block red");
    assert_pixel_color(&mut s, PAD + 5, PAD + 35, GREEN, "2nd block green");
    assert_pixel_color(&mut s, PAD + 5, PAD + 65, BLUE, "3rd block blue");
    assert_pixel_color(&mut s, PAD + 5, PAD + 95, YELLOW, "4th block yellow");
}

#[test]
fn rel_top_offset_preserves_block_size() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 120.0, 60.0, Position::Relative, Color::RED);
    doc.node_mut(div).style.top = Length::px(15.0);
    let mut s = render(&doc);
    // Block at (PAD, PAD+15); verify all four corners
    assert_pixel_color(&mut s, PAD + 1, PAD + 16, RED, "TL corner");
    assert_pixel_color(&mut s, PAD + 118, PAD + 16, RED, "TR corner");
    assert_pixel_color(&mut s, PAD + 1, PAD + 74, RED, "BL corner");
    assert_pixel_color(&mut s, PAD + 118, PAD + 74, RED, "BR corner");
}

#[test]
fn abs_four_corners_of_surface() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let tl = add_positioned_block(&mut doc, vp, 30.0, 30.0, Position::Absolute, Color::RED);
    doc.node_mut(tl).style.top = Length::px(0.0);
    doc.node_mut(tl).style.left = Length::px(0.0);
    let tr = add_positioned_block(&mut doc, vp, 30.0, 30.0, Position::Absolute, Color::GREEN);
    doc.node_mut(tr).style.top = Length::px(0.0);
    doc.node_mut(tr).style.right = Length::px(0.0);
    let bl = add_positioned_block(&mut doc, vp, 30.0, 30.0, Position::Absolute, Color::BLUE);
    doc.node_mut(bl).style.left = Length::px(0.0);
    doc.node_mut(bl).style.bottom = Length::px(0.0);
    let br = add_positioned_block(&mut doc, vp, 30.0, 30.0, Position::Absolute, color_from_rgb(255, 255, 0));
    doc.node_mut(br).style.right = Length::px(0.0);
    doc.node_mut(br).style.bottom = Length::px(0.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 5, 5, RED, "top-left corner block");
    assert_pixel_color(&mut s, 740, 5, GREEN, "top-right corner block");
    // bottom blocks at y = 600 - 30 = 570
    assert_pixel_color(&mut s, 5, 575, BLUE, "bottom-left corner block");
    assert_pixel_color(&mut s, 740, 575, YELLOW, "bottom-right corner block");
}

#[test]
fn abs_with_margin_top() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(80.0);
    doc.node_mut(div).style.height = Length::px(40.0);
    doc.node_mut(div).style.position = Position::Absolute;
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.left = Length::px(10.0);
    doc.node_mut(div).style.margin_top = Length::px(5.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // top:10 + margin-top:5 = 15
    assert_pixel_color(&mut s, 15, 16, RED, "abs with margin-top offset");
}

#[test]
fn abs_with_margin_left() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(80.0);
    doc.node_mut(div).style.height = Length::px(40.0);
    doc.node_mut(div).style.position = Position::Absolute;
    doc.node_mut(div).style.top = Length::px(10.0);
    doc.node_mut(div).style.left = Length::px(10.0);
    doc.node_mut(div).style.margin_left = Length::px(5.0);
    doc.node_mut(div).style.background_color = Color::RED;
    doc.append_child(vp, div);
    let mut s = render(&doc);
    // left:10 + margin-left:5 = 15
    assert_pixel_color(&mut s, 16, 15, RED, "abs with margin-left offset");
}

#[test]
fn rel_three_blocks_each_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 80.0, 30.0, Position::Relative, Color::RED);
    doc.node_mut(a).style.left = Length::px(0.0);
    let b = add_positioned_block(&mut doc, vp, 80.0, 30.0, Position::Relative, Color::GREEN);
    doc.node_mut(b).style.left = Length::px(100.0);
    let c = add_positioned_block(&mut doc, vp, 80.0, 30.0, Position::Relative, Color::BLUE);
    doc.node_mut(c).style.left = Length::px(200.0);
    let mut s = render(&doc);
    // A at (PAD, PAD); B at (PAD+100, PAD+30); C at (PAD+200, PAD+60)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "first rel");
    assert_pixel_color(&mut s, PAD + 105, PAD + 35, GREEN, "second rel offset");
    assert_pixel_color(&mut s, PAD + 205, PAD + 65, BLUE, "third rel offset");
}

#[test]
fn fixed_right_bottom_corner() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 40.0, 20.0, Position::Fixed, Color::RED);
    doc.node_mut(div).style.right = Length::px(0.0);
    doc.node_mut(div).style.bottom = Length::px(0.0);
    let mut s = render(&doc);
    // left = 760-0-40=720; top = 600-0-20=580
    assert_pixel_color(&mut s, 725, 585, RED, "fixed at right-bottom corner");
}

#[test]
fn abs_in_container_top_left_offsets() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(300.0);
    doc.node_mut(container).style.height = Length::px(200.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.background_color = color_from_rgb(220, 220, 220);
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 40.0, 20.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(50.0);
    doc.node_mut(abs).style.left = Length::px(100.0);
    let mut s = render(&doc);
    // Container at (20,20); abs at (120, 70)
    assert_pixel_color(&mut s, 125, 75, RED, "abs inside container with offsets");
    assert_pixel_color(&mut s, 119, 75, (220, 220, 220), "container bg left of abs");
}

#[test]
fn overlap_abs_covers_center_of_static() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 100.0, Color::RED);
    let abs = add_positioned_block(&mut doc, vp, 60.0, 40.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(PAD as f32 + 30.0);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 + 70.0);
    let mut s = render(&doc);
    // Red around the abs; blue in the center
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "red top-left");
    assert_pixel_color(&mut s, PAD + 75, PAD + 35, BLUE, "blue center overlay");
    assert_pixel_color(&mut s, PAD + 195, PAD + 95, RED, "red bottom-right");
}

#[test]
fn rel_offset_then_abs_overlay() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let rel = add_positioned_block(&mut doc, vp, 120.0, 60.0, Position::Relative, Color::RED);
    doc.node_mut(rel).style.top = Length::px(10.0);
    doc.node_mut(rel).style.left = Length::px(10.0);
    let abs = add_positioned_block(&mut doc, vp, 40.0, 20.0, Position::Absolute, Color::GREEN);
    doc.node_mut(abs).style.top = Length::px(PAD as f32 + 20.0);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 + 20.0);
    let mut s = render(&doc);
    // Rel at (30, 30); abs at (40, 40)
    assert_pixel_color(&mut s, 35, 35, RED, "relative before abs overlay");
    assert_pixel_color(&mut s, 45, 45, GREEN, "abs on top of relative");
}

#[test]
fn static_block_fills_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 0.0, 50.0, Color::RED);
    let mut s = render(&doc);
    // Block with no explicit width should fill content area (760px)
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "left side of full-width block");
    assert_pixel_color(&mut s, SURFACE_W - PAD - 5, PAD + 5, RED, "right side of full-width block");
}

#[test]
fn abs_two_non_overlapping_vertically() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let a = add_positioned_block(&mut doc, vp, 200.0, 50.0, Position::Absolute, Color::RED);
    doc.node_mut(a).style.top = Length::px(10.0);
    doc.node_mut(a).style.left = Length::px(10.0);
    let b = add_positioned_block(&mut doc, vp, 200.0, 50.0, Position::Absolute, Color::BLUE);
    doc.node_mut(b).style.top = Length::px(70.0);
    doc.node_mut(b).style.left = Length::px(10.0);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, 50, 30, RED, "top abs");
    assert_pixel_color(&mut s, 50, 65, WHITE, "gap between abs blocks");
    assert_pixel_color(&mut s, 50, 90, BLUE, "bottom abs");
}

#[test]
fn rel_bottom_offset_from_second_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::RED);
    add_colored_block(&mut doc, vp, 100.0, 40.0, Color::WHITE);
    let div = add_positioned_block(&mut doc, vp, 100.0, 40.0, Position::Relative, Color::BLUE);
    doc.node_mut(div).style.bottom = Length::px(40.0);
    let mut s = render(&doc);
    // Blue normal at (PAD, PAD+80); bottom:40 → (PAD, PAD+40)
    assert_pixel_color(&mut s, PAD + 5, PAD + 45, BLUE, "bottom offset from third position");
}

#[test]
fn fixed_with_explicit_size_check() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = add_positioned_block(&mut doc, vp, 150.0, 75.0, Position::Fixed, Color::GREEN);
    doc.node_mut(div).style.top = Length::px(100.0);
    doc.node_mut(div).style.left = Length::px(100.0);
    let mut s = render(&doc);
    // Block at (100, 100), size 150x75
    assert_pixel_color(&mut s, 101, 101, GREEN, "fixed TL");
    assert_pixel_color(&mut s, 248, 174, GREEN, "fixed BR");
    assert_pixel_color(&mut s, 99, 101, WHITE, "left of fixed");
    assert_pixel_color(&mut s, 255, 101, WHITE, "right of fixed");
}

#[test]
fn abs_in_container_white_outside_container() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(200.0);
    doc.node_mut(container).style.height = Length::px(100.0);
    doc.node_mut(container).style.position = Position::Relative;
    doc.node_mut(container).style.background_color = color_from_rgb(200, 200, 200);
    doc.append_child(vp, container);

    let abs = add_positioned_block(&mut doc, container, 40.0, 20.0, Position::Absolute, Color::RED);
    doc.node_mut(abs).style.top = Length::px(5.0);
    doc.node_mut(abs).style.left = Length::px(5.0);
    let mut s = render(&doc);
    // Container at (20,20), size 200x100
    // Outside container should be white
    assert_pixel_color(&mut s, PAD + 205, PAD + 5, WHITE, "right of container");
    assert_pixel_color(&mut s, PAD + 5, PAD + 105, WHITE, "below container");
}

#[test]
fn overlap_static_rel_abs_fixed_all_four() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    // Static (red)
    add_colored_block(&mut doc, vp, 200.0, 200.0, Color::RED);
    // Relative (green) overlapping from below
    let rel = add_positioned_block(&mut doc, vp, 150.0, 150.0, Position::Relative, Color::GREEN);
    doc.node_mut(rel).style.top = Length::px(-170.0);
    doc.node_mut(rel).style.left = Length::px(25.0);
    // Absolute (blue) overlapping both
    let abs = add_positioned_block(&mut doc, vp, 100.0, 100.0, Position::Absolute, Color::BLUE);
    doc.node_mut(abs).style.top = Length::px(PAD as f32 + 50.0);
    doc.node_mut(abs).style.left = Length::px(PAD as f32 + 50.0);
    // Fixed (yellow) on top of everything
    let fixed = add_positioned_block(&mut doc, vp, 60.0, 60.0, Position::Fixed, color_from_rgb(255, 255, 0));
    doc.node_mut(fixed).style.top = Length::px(PAD as f32 + 70.0);
    doc.node_mut(fixed).style.left = Length::px(PAD as f32 + 70.0);
    let mut s = render(&doc);
    // Check each layer at a unique spot
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, RED, "static red visible");
    assert_pixel_color(&mut s, PAD + 30, PAD + 40, GREEN, "relative green on red");
    assert_pixel_color(&mut s, PAD + 55, PAD + 55, BLUE, "abs blue on green/red");
    assert_pixel_color(&mut s, PAD + 75, PAD + 75, YELLOW, "fixed yellow on top of all");
}
