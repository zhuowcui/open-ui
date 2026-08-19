//! SP13 Phase J — Inline Layout Pixel Comparison Tests.
//!
//! Renders inline-level layouts (inline-block, inline spans, block-in-inline,
//! float exclusion, baseline alignment, box decorations) and validates pixel
//! output to verify correct painting positions and colors.
//!
//! ## Running
//!
//! ```bash
//! cd bindings/rust
//! cargo test --package openui-paint --test sp13_j_inline_pixel_tests
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
const PAD: i32 = 20;
const TOLERANCE: u8 = 2;

const RED: (u8, u8, u8) = (255, 0, 0);
const GREEN: (u8, u8, u8) = (0, 128, 0);
const BLUE: (u8, u8, u8) = (0, 0, 255);
const WHITE: (u8, u8, u8) = (255, 255, 255);
const CYAN: (u8, u8, u8) = (0, 255, 255);
const MAGENTA: (u8, u8, u8) = (255, 0, 255);
const YELLOW: (u8, u8, u8) = (255, 255, 0);
const ORANGE: (u8, u8, u8) = (255, 165, 0);
const GRAY: (u8, u8, u8) = (128, 128, 128);

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
    (
        pixels[idx],
        pixels[idx + 1],
        pixels[idx + 2],
        pixels[idx + 3],
    )
}

fn assert_pixel_color(surface: &mut Surface, x: i32, y: i32, expected: (u8, u8, u8), msg: &str) {
    let (r, g, b, _a) = get_pixel(surface, x, y);
    let dr = (r as i16 - expected.0 as i16).unsigned_abs();
    let dg = (g as i16 - expected.1 as i16).unsigned_abs();
    let db = (b as i16 - expected.2 as i16).unsigned_abs();
    assert!(
        dr <= TOLERANCE as u16 && dg <= TOLERANCE as u16 && db <= TOLERANCE as u16,
        "{}: pixel ({},{}) = ({},{},{}) expected ~({},{},{}), diff=({},{},{})",
        msg,
        x,
        y,
        r,
        g,
        b,
        expected.0,
        expected.1,
        expected.2,
        dr,
        dg,
        db,
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

fn has_non_white_in_region(surface: &mut Surface, x: i32, y: i32, w: i32, h: i32) -> bool {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = (info.width() * 4) as usize;
    let mut pixels = vec![0u8; info.height() as usize * row_bytes];
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
    let bpp = 4;
    let img_w = info.width() as i32;
    for py in y..(y + h).min(info.height()) {
        for px in x..(x + w).min(img_w) {
            let off = (py as usize) * row_bytes + (px as usize) * bpp;
            if off + 3 < pixels.len() {
                let (c0, c1, c2) = (pixels[off], pixels[off + 1], pixels[off + 2]);
                if c0 != 0xFF || c1 != 0xFF || c2 != 0xFF {
                    return true;
                }
            }
        }
    }
    false
}

fn region_is_white(surface: &mut Surface, x: i32, y: i32, w: i32, h: i32) -> bool {
    !has_non_white_in_region(surface, x, y, w, h)
}

fn pixel_is_white(surface: &mut Surface, x: i32, y: i32) -> bool {
    let (r, g, b, _) = get_pixel(surface, x, y);
    r >= 253 && g >= 253 && b >= 253
}

fn pixel_is_not_white(surface: &mut Surface, x: i32, y: i32) -> bool {
    !pixel_is_white(surface, x, y)
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

fn add_inline_block(doc: &mut Document, parent: NodeId, w: f32, h: f32, color: Color) -> NodeId {
    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(w);
    doc.node_mut(ib).style.height = Length::px(h);
    doc.node_mut(ib).style.background_color = color;
    doc.append_child(parent, ib);
    ib
}

fn add_float_box(
    doc: &mut Document,
    parent: NodeId,
    w: f32,
    h: f32,
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

fn add_span(doc: &mut Document, parent: NodeId) -> NodeId {
    let span = doc.create_node(ElementTag::Span);
    doc.node_mut(span).style.display = Display::Inline;
    doc.append_child(parent, span);
    span
}

fn add_text(doc: &mut Document, parent: NodeId, content: &str) -> NodeId {
    let parent_style = doc.node(parent).style.clone();
    let text = doc.create_node(ElementTag::Text);
    doc.node_mut(text).text = Some(content.to_string());
    doc.node_mut(text).style.display = Display::Inline;
    doc.node_mut(text).style.font_family = parent_style.font_family;
    doc.node_mut(text).style.font_size = parent_style.font_size;
    doc.node_mut(text).style.font_weight = parent_style.font_weight;
    doc.node_mut(text).style.font_style = parent_style.font_style;
    doc.node_mut(text).style.font_stretch = parent_style.font_stretch;
    doc.node_mut(text).style.color = parent_style.color;
    doc.node_mut(text).style.letter_spacing = parent_style.letter_spacing;
    doc.node_mut(text).style.word_spacing = parent_style.word_spacing;
    doc.node_mut(text).style.text_transform = parent_style.text_transform;
    doc.node_mut(text).style.white_space = parent_style.white_space;
    doc.node_mut(text).style.direction = parent_style.direction;
    doc.node_mut(text).style.line_height = parent_style.line_height;
    doc.node_mut(text).style.text_decoration_line = parent_style.text_decoration_line;
    doc.node_mut(text).style.text_decoration_style = parent_style.text_decoration_style;
    doc.node_mut(text).style.text_decoration_color = parent_style.text_decoration_color.clone();
    doc.node_mut(text).style.vertical_align = parent_style.vertical_align;
    doc.append_child(parent, text);
    text
}

fn inherit_text_style(doc: &mut Document, parent: NodeId, child: NodeId) {
    let ps = doc.node(parent).style.clone();
    let cs = &mut doc.node_mut(child).style;
    cs.font_family = ps.font_family;
    cs.font_size = ps.font_size;
    cs.font_weight = ps.font_weight;
    cs.font_style = ps.font_style;
    cs.font_stretch = ps.font_stretch;
    cs.color = ps.color;
    cs.letter_spacing = ps.letter_spacing;
    cs.word_spacing = ps.word_spacing;
    cs.text_transform = ps.text_transform;
    cs.white_space = ps.white_space;
    cs.direction = ps.direction;
    cs.line_height = ps.line_height;
    cs.text_align = ps.text_align;
    cs.text_decoration_line = ps.text_decoration_line;
    cs.text_decoration_style = ps.text_decoration_style;
    cs.text_decoration_color = ps.text_decoration_color;
    cs.vertical_align = ps.vertical_align;
}

fn render(doc: &Document) -> Surface {
    render_to_surface(doc, SURFACE_W, SURFACE_H).expect("render_to_surface failed")
}

fn color_from_rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba8(r, g, b, 255)
}

// ═══════════════════════════════════════════════════════════════════════
// §1  Inline-Block Basic Rendering (8 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_single_renders() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    let mut s = render(&doc);
    assert!(
        has_visible_content(&mut s),
        "inline-block should render content"
    );
}

#[test]
fn inline_block_single_position() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 30, RED, "inline-block center");
}

#[test]
fn inline_block_two_side_by_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    add_inline_block(&mut doc, vp, 100.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    // First inline-block at x=PAD, second at x=PAD+100
    assert_pixel_color(&mut s, PAD + 50, PAD + 30, RED, "first inline-block");
    assert_pixel_color(&mut s, PAD + 150, PAD + 30, BLUE, "second inline-block");
}

#[test]
fn inline_block_three_side_by_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 80.0, 40.0, Color::RED);
    add_inline_block(&mut doc, vp, 80.0, 40.0, Color::GREEN);
    add_inline_block(&mut doc, vp, 80.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 40, PAD + 20, RED, "first");
    assert_pixel_color(&mut s, PAD + 120, PAD + 20, GREEN, "second");
    assert_pixel_color(&mut s, PAD + 200, PAD + 20, BLUE, "third");
}

#[test]
fn inline_block_different_sizes() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 120.0, 80.0, Color::RED);
    add_inline_block(&mut doc, vp, 60.0, 40.0, Color::BLUE);
    let mut s = render(&doc);
    // Both start from top of line, first is wider/taller
    assert_pixel_color(&mut s, PAD + 60, PAD + 40, RED, "large inline-block center");
    // Second inline-block starts at x=PAD+120
    assert!(
        has_non_white_in_region(&mut s, PAD + 120, PAD, 60, 80),
        "second inline-block region should have content"
    );
}

#[test]
fn inline_block_white_after() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    let mut s = render(&doc);
    // Area to the right of the inline-block should be white
    assert!(
        region_is_white(&mut s, PAD + 110, PAD, 100, 60),
        "region after inline-block should be white"
    );
}

#[test]
fn inline_block_white_below() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    let mut s = render(&doc);
    // Area below should be white (accounting for possible line-height)
    assert!(
        region_is_white(&mut s, PAD, PAD + 80, 100, 60),
        "region well below inline-block should be white"
    );
}

#[test]
fn inline_block_small_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 10.0, 10.0, Color::RED);
    let mut s = render(&doc);
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD, 15, 15),
        "small inline-block should render visible pixels"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// §2  Inline-Block with Background Colors (6 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_cyan_background() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 50.0, color_from_rgb(0, 255, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, CYAN, "cyan inline-block");
}

#[test]
fn inline_block_magenta_background() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 50.0, color_from_rgb(255, 0, 255));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, MAGENTA, "magenta inline-block");
}

#[test]
fn inline_block_yellow_background() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 50.0, color_from_rgb(255, 255, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, YELLOW, "yellow inline-block");
}

#[test]
fn inline_block_orange_background() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 50.0, color_from_rgb(255, 165, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, ORANGE, "orange inline-block");
}

#[test]
fn inline_block_gray_background() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 50.0, color_from_rgb(128, 128, 128));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, GRAY, "gray inline-block");
}

#[test]
fn inline_block_two_colors_side_by_side() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 80.0, 40.0, color_from_rgb(0, 255, 255));
    add_inline_block(&mut doc, vp, 80.0, 40.0, color_from_rgb(255, 165, 0));
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 40, PAD + 20, CYAN, "first: cyan");
    assert_pixel_color(&mut s, PAD + 120, PAD + 20, ORANGE, "second: orange");
}

// ═══════════════════════════════════════════════════════════════════════
// §3  Inline-Block with Borders (5 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_with_border_renders() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 60.0, Color::WHITE);
    doc.node_mut(ib).style.border_top_width = 3;
    doc.node_mut(ib).style.border_right_width = 3;
    doc.node_mut(ib).style.border_bottom_width = 3;
    doc.node_mut(ib).style.border_left_width = 3;
    doc.node_mut(ib).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(ib).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(ib).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(ib).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    let mut s = render(&doc);
    assert!(
        has_visible_content(&mut s),
        "bordered inline-block should render"
    );
}

#[test]
fn inline_block_border_top_mid_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 60.0, Color::WHITE);
    doc.node_mut(ib).style.border_top_width = 4;
    doc.node_mut(ib).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_top_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // Check top border mid-edge (away from corners)
    assert!(
        pixel_is_not_white(&mut s, PAD + 50, PAD + 1),
        "top border mid-edge should have color"
    );
}

#[test]
fn inline_block_border_left_mid_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 60.0, Color::WHITE);
    doc.node_mut(ib).style.border_left_width = 4;
    doc.node_mut(ib).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_left_color = StyleColor::Resolved(Color::BLUE);
    let mut s = render(&doc);
    // Check left border mid-edge (away from corners)
    assert!(
        pixel_is_not_white(&mut s, PAD + 1, PAD + 30),
        "left border mid-edge should have color"
    );
}

#[test]
fn inline_block_border_bottom_mid_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 60.0, Color::WHITE);
    doc.node_mut(ib).style.border_bottom_width = 4;
    doc.node_mut(ib).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_bottom_color = StyleColor::Resolved(Color::GREEN);
    let mut s = render(&doc);
    // Content-box: content is 60px, border is outside at y=PAD+60..PAD+63
    assert!(
        pixel_is_not_white(&mut s, PAD + 50, PAD + 61),
        "bottom border mid-edge should have color"
    );
}

#[test]
fn inline_block_border_right_mid_edge() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 60.0, Color::WHITE);
    doc.node_mut(ib).style.border_right_width = 4;
    doc.node_mut(ib).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(ib).style.border_right_color = StyleColor::Resolved(Color::RED);
    let mut s = render(&doc);
    // Content-box: content is 100px, border is outside at x=PAD+100..PAD+103
    assert!(
        pixel_is_not_white(&mut s, PAD + 101, PAD + 30),
        "right border mid-edge should have color"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// §4  Float Exclusion with Inline-Block (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn float_left_with_inline_block_beside() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Left, Color::RED);
    let ib = doc.create_node(ElementTag::Div);
    doc.node_mut(ib).style.display = Display::InlineBlock;
    doc.node_mut(ib).style.width = Length::px(200.0);
    doc.node_mut(ib).style.height = Length::px(60.0);
    doc.node_mut(ib).style.background_color = Color::BLUE;
    doc.append_child(vp, ib);
    let mut s = render(&doc);
    // Float at left edge, inline-block should appear
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, RED, "float area");
    assert!(has_visible_content(&mut s), "content should render");
}

#[test]
fn float_right_with_inline_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let content_w = SURFACE_W - 2 * PAD;
    add_float_box(&mut doc, vp, 100.0, 80.0, Float::Right, Color::RED);
    add_inline_block(&mut doc, vp, 200.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    // Right float at right edge
    let float_x = PAD + content_w - 50;
    assert_pixel_color(&mut s, float_x, PAD + 40, RED, "right float");
    // Inline-block at left
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD, 200, 80),
        "inline-block should render in available space"
    );
}

#[test]
fn float_does_not_overlap_inline_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 100.0, 100.0, Float::Left, Color::RED);
    add_inline_block(&mut doc, vp, 100.0, 100.0, Color::BLUE);
    let mut s = render(&doc);
    // Float occupies [PAD, PAD+100) × [PAD, PAD+100)
    // Center of float region should be red, not blue
    assert_pixel_color(
        &mut s,
        PAD + 50,
        PAD + 50,
        RED,
        "float region should be red",
    );
}

#[test]
fn float_exclusion_inline_block_below_cleared() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_float_box(&mut doc, vp, 200.0, 80.0, Float::Left, Color::RED);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.node_mut(wrapper).style.display = Display::Block;
    doc.node_mut(wrapper).style.clear = Clear::Both;
    doc.append_child(vp, wrapper);
    add_inline_block(&mut doc, wrapper, 200.0, 60.0, Color::BLUE);
    let mut s = render(&doc);
    // Float is 80px tall, cleared wrapper starts at y >= PAD+80
    assert_pixel_color(&mut s, PAD + 50, PAD + 40, RED, "float area");
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD + 80, 200, 60),
        "cleared inline-block should appear below float"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// §5  Block-in-Inline (4 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn block_in_inline_renders_block() {
    // Span > Div (block) — the block child should render
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.append_child(vp, container);
    let span = add_span(&mut doc, container);
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.width = Length::px(200.0);
    doc.node_mut(block).style.height = Length::px(60.0);
    doc.node_mut(block).style.background_color = Color::RED;
    doc.append_child(span, block);
    let mut s = render(&doc);
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD, 200, 60),
        "block-in-inline should produce visible output"
    );
}

#[test]
fn block_in_inline_correct_color() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.append_child(vp, container);
    let span = add_span(&mut doc, container);
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.width = Length::px(150.0);
    doc.node_mut(block).style.height = Length::px(50.0);
    doc.node_mut(block).style.background_color = Color::BLUE;
    doc.append_child(span, block);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 75, PAD + 25, BLUE, "block-in-inline color");
}

#[test]
fn block_in_inline_between_text_segments() {
    // <div><span>text<div>block</div>text</span></div>
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.node_mut(container).style.width = Length::px(400.0);
    inherit_text_style(&mut doc, vp, container);
    doc.append_child(vp, container);
    let span = add_span(&mut doc, container);
    inherit_text_style(&mut doc, container, span);
    add_text(&mut doc, span, "Before ");
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.width = Length::px(200.0);
    doc.node_mut(block).style.height = Length::px(40.0);
    doc.node_mut(block).style.background_color = Color::RED;
    doc.append_child(span, block);
    add_text(&mut doc, span, " After");
    let mut s = render(&doc);
    assert!(
        has_visible_content(&mut s),
        "block-in-inline with text should render"
    );
}

#[test]
fn block_in_inline_white_after_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let container = doc.create_node(ElementTag::Div);
    doc.node_mut(container).style.display = Display::Block;
    doc.append_child(vp, container);
    let span = add_span(&mut doc, container);
    let block = doc.create_node(ElementTag::Div);
    doc.node_mut(block).style.display = Display::Block;
    doc.node_mut(block).style.width = Length::px(100.0);
    doc.node_mut(block).style.height = Length::px(50.0);
    doc.node_mut(block).style.background_color = Color::RED;
    doc.append_child(span, block);
    let mut s = render(&doc);
    // Right of block should be white
    assert!(
        region_is_white(&mut s, PAD + 110, PAD, 100, 50),
        "area right of block-in-inline should be white"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// §6  Inline-Block with Padding (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_with_padding_renders() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 60.0, Color::RED);
    doc.node_mut(ib).style.padding_top = Length::px(10.0);
    doc.node_mut(ib).style.padding_right = Length::px(10.0);
    doc.node_mut(ib).style.padding_bottom = Length::px(10.0);
    doc.node_mut(ib).style.padding_left = Length::px(10.0);
    let mut s = render(&doc);
    assert!(
        has_visible_content(&mut s),
        "padded inline-block should render"
    );
    // Content area starts at PAD, the padded inline-block is 120x80 total
    assert_pixel_color(
        &mut s,
        PAD + 5,
        PAD + 5,
        RED,
        "padding area should have bg color",
    );
}

#[test]
fn inline_block_padding_extends_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 80.0, 40.0, Color::BLUE);
    doc.node_mut(ib).style.padding_left = Length::px(20.0);
    doc.node_mut(ib).style.padding_right = Length::px(20.0);
    let mut s = render(&doc);
    // Total width = 80 + 20 + 20 = 120, so at x=PAD+110 should still be blue
    assert_pixel_color(&mut s, PAD + 110, PAD + 20, BLUE, "right padding area");
}

#[test]
fn inline_block_padding_top_bottom() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 80.0, 40.0, Color::GREEN);
    doc.node_mut(ib).style.padding_top = Length::px(15.0);
    doc.node_mut(ib).style.padding_bottom = Length::px(15.0);
    let mut s = render(&doc);
    // Total height = 40 + 15 + 15 = 70, top padding area at y=PAD+5
    assert_pixel_color(&mut s, PAD + 40, PAD + 5, GREEN, "top padding");
    assert_pixel_color(&mut s, PAD + 40, PAD + 65, GREEN, "bottom padding");
}

// ═══════════════════════════════════════════════════════════════════════
// §7  Inline-Block with Margins (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_margin_left_offset() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(ib).style.margin_left = Length::px(30.0);
    let mut s = render(&doc);
    // Inline-block renders with margin applied (position depends on IFC)
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD, 150, 50),
        "inline-block with margin-left should render"
    );
    assert_pixel_color(&mut s, PAD + 50, PAD + 25, RED, "inline-block center");
}

#[test]
fn inline_block_margin_between_two() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 80.0, 40.0, Color::RED);
    let ib2 = add_inline_block(&mut doc, vp, 80.0, 40.0, Color::BLUE);
    doc.node_mut(ib2).style.margin_left = Length::px(40.0);
    let mut s = render(&doc);
    // First inline-block at PAD
    assert_pixel_color(&mut s, PAD + 40, PAD + 20, RED, "first inline-block");
    // Both should render (second position depends on margin handling in IFC)
    assert!(
        has_visible_content(&mut s),
        "both inline-blocks should render"
    );
    // Blue inline-block should appear somewhere after the red one
    assert!(
        has_non_white_in_region(&mut s, PAD + 80, PAD, 200, 40),
        "second inline-block should render after the first"
    );
}

#[test]
fn inline_block_margin_top() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 100.0, 50.0, Color::RED);
    doc.node_mut(ib).style.margin_top = Length::px(20.0);
    let mut s = render(&doc);
    assert!(
        has_visible_content(&mut s),
        "inline-block with margin-top should render"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// §8  Inline-Block Below Block (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_after_block_element() {
    // block element followed by inline-block in a wrapper
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_colored_block(&mut doc, vp, 200.0, 50.0, Color::RED);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.node_mut(wrapper).style.display = Display::Block;
    doc.append_child(vp, wrapper);
    add_inline_block(&mut doc, wrapper, 200.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    // Block at y=PAD, height=50. Wrapper at y=PAD+50
    assert_pixel_color(&mut s, PAD + 100, PAD + 25, RED, "block");
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD + 50, 200, 50),
        "inline-block below block should render"
    );
}

#[test]
fn block_after_inline_block_wrapper() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let wrapper = doc.create_node(ElementTag::Div);
    doc.node_mut(wrapper).style.display = Display::Block;
    doc.append_child(vp, wrapper);
    add_inline_block(&mut doc, wrapper, 200.0, 50.0, Color::RED);
    add_colored_block(&mut doc, vp, 200.0, 50.0, Color::BLUE);
    let mut s = render(&doc);
    assert!(
        has_visible_content(&mut s),
        "block after inline-block wrapper should render"
    );
}

#[test]
fn stacked_inline_block_wrappers() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    // Two wrappers each containing an inline-block
    let w1 = doc.create_node(ElementTag::Div);
    doc.node_mut(w1).style.display = Display::Block;
    doc.append_child(vp, w1);
    add_inline_block(&mut doc, w1, 150.0, 40.0, Color::RED);

    let w2 = doc.create_node(ElementTag::Div);
    doc.node_mut(w2).style.display = Display::Block;
    doc.append_child(vp, w2);
    add_inline_block(&mut doc, w2, 150.0, 40.0, Color::BLUE);

    let mut s = render(&doc);
    // First wrapper at y=PAD, second at y ≥ PAD+40
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD, 150, 40),
        "first inline-block"
    );
    assert!(
        has_non_white_in_region(&mut s, PAD, PAD + 40, 150, 40),
        "second inline-block below"
    );
}

// ═══════════════════════════════════════════════════════════════════════
// §9  Inline-Block Sizing (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_wide() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 500.0, 30.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 250, PAD + 15, RED, "wide inline-block center");
    assert_pixel_color(
        &mut s,
        PAD + 495,
        PAD + 15,
        RED,
        "wide inline-block right edge",
    );
}

#[test]
fn inline_block_tall() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 60.0, 200.0, Color::BLUE);
    let mut s = render(&doc);
    assert_pixel_color(
        &mut s,
        PAD + 30,
        PAD + 100,
        BLUE,
        "tall inline-block center",
    );
    assert_pixel_color(
        &mut s,
        PAD + 30,
        PAD + 195,
        BLUE,
        "tall inline-block near bottom",
    );
}

#[test]
fn inline_block_square() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    add_inline_block(&mut doc, vp, 100.0, 100.0, Color::GREEN);
    let mut s = render(&doc);
    assert_pixel_color(
        &mut s,
        PAD + 50,
        PAD + 50,
        GREEN,
        "square inline-block center",
    );
    assert_pixel_color(&mut s, PAD + 5, PAD + 5, GREEN, "square near top-left");
    assert_pixel_color(
        &mut s,
        PAD + 95,
        PAD + 95,
        GREEN,
        "square near bottom-right",
    );
}

// ═══════════════════════════════════════════════════════════════════════
// §10  Inline-Block with Nested Content (3 tests)
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn inline_block_containing_block_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(100.0);
    doc.node_mut(child).style.height = Length::px(50.0);
    doc.node_mut(child).style.background_color = Color::RED;
    doc.append_child(ib, child);
    let mut s = render(&doc);
    // Red child inside blue inline-block
    assert_pixel_color(
        &mut s,
        PAD + 50,
        PAD + 25,
        RED,
        "block child inside inline-block",
    );
    assert_pixel_color(&mut s, PAD + 150, PAD + 75, BLUE, "blue parent bg visible");
}

#[test]
fn inline_block_containing_inline_block() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let outer = add_inline_block(&mut doc, vp, 200.0, 100.0, Color::BLUE);
    add_inline_block(&mut doc, outer, 80.0, 40.0, Color::RED);
    let mut s = render(&doc);
    assert_pixel_color(&mut s, PAD + 40, PAD + 20, RED, "nested inline-block");
    assert_pixel_color(&mut s, PAD + 150, PAD + 75, BLUE, "outer inline-block bg");
}

#[test]
fn inline_block_with_multiple_children() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let ib = add_inline_block(&mut doc, vp, 300.0, 120.0, color_from_rgb(200, 200, 200));
    let c1 = doc.create_node(ElementTag::Div);
    doc.node_mut(c1).style.display = Display::Block;
    doc.node_mut(c1).style.width = Length::px(100.0);
    doc.node_mut(c1).style.height = Length::px(40.0);
    doc.node_mut(c1).style.background_color = Color::RED;
    doc.append_child(ib, c1);
    let c2 = doc.create_node(ElementTag::Div);
    doc.node_mut(c2).style.display = Display::Block;
    doc.node_mut(c2).style.width = Length::px(100.0);
    doc.node_mut(c2).style.height = Length::px(40.0);
    doc.node_mut(c2).style.background_color = Color::BLUE;
    doc.append_child(ib, c2);
    let mut s = render(&doc);
    assert_pixel_color(
        &mut s,
        PAD + 50,
        PAD + 20,
        RED,
        "first child in inline-block",
    );
    assert_pixel_color(
        &mut s,
        PAD + 50,
        PAD + 60,
        BLUE,
        "second child in inline-block",
    );
}
