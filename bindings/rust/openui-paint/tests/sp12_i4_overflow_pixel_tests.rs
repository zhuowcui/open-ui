//! SP12 Phase I4 — Overflow & Sizing Pixel Comparison Tests
//!
//! Verifies pixel-level correctness of CSS overflow clipping, min/max sizing,
//! and box-sizing behavior using the full render pipeline (DOM → Layout → Paint).
//!
//! Categories:
//! 1. Overflow Hidden Clipping (20 tests)
//! 2. Overflow Visible (15 tests)
//! 3. Overflow Scroll/Auto/Clip (15 tests)
//! 4. Min/Max Width (15 tests)
//! 5. Min/Max Height (15 tests)
//! 6. Box Sizing (10 tests)
//! 7. Complex Overflow Scenarios (15 tests)

use std::path::{Path, PathBuf};

use skia_safe::Surface;

use openui_dom::{Document, ElementTag, NodeId};
use openui_geometry::Length;
use openui_paint::{render_to_surface, render_to_png};
use openui_style::*;

// ═══════════════════════════════════════════════════════════════════════
// ── Constants ─────────────────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

const SURFACE_W: i32 = 800;
const SURFACE_H: i32 = 600;

// ═══════════════════════════════════════════════════════════════════════
// ── Pixel Sampling Helpers ────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Read a single pixel from the surface in native N32 format.
fn get_pixel(surface: &mut Surface, x: i32, y: i32) -> (u8, u8, u8, u8) {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = (info.width() * 4) as usize;
    let mut pixels = vec![0u8; row_bytes];
    let single_row_info = skia_safe::ImageInfo::new(
        (info.width(), 1),
        info.color_type(),
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

fn assert_pixel_color(
    surface: &mut Surface,
    x: i32,
    y: i32,
    expected: (u8, u8, u8),
    msg: &str,
) {
    let (r, g, b, _a) = get_pixel(surface, x, y);
    let dr = (r as i16 - expected.0 as i16).unsigned_abs();
    let dg = (g as i16 - expected.1 as i16).unsigned_abs();
    let db = (b as i16 - expected.2 as i16).unsigned_abs();
    assert!(
        dr <= 2 && dg <= 2 && db <= 2,
        "{}: pixel ({},{}) = ({},{},{}) expected ~({},{},{})",
        msg, x, y, r, g, b, expected.0, expected.1, expected.2
    );
}

/// Check if a pixel is approximately white (all channels >= 253).
fn pixel_is_white(surface: &mut Surface, x: i32, y: i32) -> bool {
    let (c0, c1, c2, _) = get_pixel(surface, x, y);
    c0 >= 253 && c1 >= 253 && c2 >= 253
}

/// Check if a pixel is NOT white (has visible content).
fn pixel_is_not_white(surface: &mut Surface, x: i32, y: i32) -> bool {
    !pixel_is_white(surface, x, y)
}

/// Check if two pixels have approximately the same color.
fn pixels_match(surface: &mut Surface, x1: i32, y1: i32, x2: i32, y2: i32) -> bool {
    let (a0, a1, a2, _) = get_pixel(surface, x1, y1);
    let (b0, b1, b2, _) = get_pixel(surface, x2, y2);
    (a0 as i16 - b0 as i16).unsigned_abs() <= 3
        && (a1 as i16 - b1 as i16).unsigned_abs() <= 3
        && (a2 as i16 - b2 as i16).unsigned_abs() <= 3
}

/// Extract raw pixel bytes from a Skia surface.
fn surface_to_rgba(surface: &mut Surface) -> (u32, u32, Vec<u8>) {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let w = info.width() as u32;
    let h = info.height() as u32;
    let row_bytes = (w * 4) as usize;
    let mut pixels = vec![0u8; (h as usize) * row_bytes];
    image.read_pixels(
        &info,
        &mut pixels,
        row_bytes,
        (0, 0),
        skia_safe::image::CachingHint::Allow,
    );
    (w, h, pixels)
}

/// Check if a surface has any non-white pixels.
fn has_visible_content(surface: &mut Surface) -> bool {
    let (_, _, pixels) = surface_to_rgba(surface);
    for chunk in pixels.chunks(4) {
        if chunk.len() == 4 && (chunk[0] != 0xFF || chunk[1] != 0xFF || chunk[2] != 0xFF) {
            return true;
        }
    }
    false
}

/// Check if any pixel in the given rectangular region is non-white.
fn has_non_white_in_region(
    surface: &mut Surface,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) -> bool {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = info.min_row_bytes();
    let mut pixels = vec![0u8; info.height() as usize * row_bytes];
    image.read_pixels(
        &info,
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

/// Check if an entire region is white (nothing rendered).
fn region_is_white(surface: &mut Surface, x: i32, y: i32, w: i32, h: i32) -> bool {
    !has_non_white_in_region(surface, x, y, w, h)
}

// ═══════════════════════════════════════════════════════════════════════
// ── DOM Builder Helpers ───────────────────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

/// Set up a minimal viewport root with Block display.
fn setup_viewport(doc: &mut Document) -> NodeId {
    let vp = doc.root();
    doc.node_mut(vp).style.display = Display::Block;
    vp
}

/// Create a block div with a given width and append to parent.
fn add_block(doc: &mut Document, parent: NodeId, width_px: f32) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    if width_px > 0.0 {
        doc.node_mut(div).style.width = Length::px(width_px);
    }
    doc.append_child(parent, div);
    div
}

/// Create a colored block box with specified dimensions and background.
fn add_colored_box(
    doc: &mut Document,
    parent: NodeId,
    w: f32,
    h: f32,
    color: Color,
) -> NodeId {
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(w);
    doc.node_mut(div).style.height = Length::px(h);
    doc.node_mut(div).style.background_color = color;
    doc.append_child(parent, div);
    div
}

/// Build a basic overflow test: parent with overflow setting, child extending beyond.
fn build_overflow_test(
    overflow: Overflow,
    parent_w: f32,
    parent_h: f32,
    child_w: f32,
    child_h: f32,
) -> (Document, Surface) {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(parent_w);
    doc.node_mut(parent).style.height = Length::px(parent_h);
    doc.node_mut(parent).style.overflow_x = overflow;
    doc.node_mut(parent).style.overflow_y = overflow;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(child_w);
    doc.node_mut(child).style.height = Length::px(child_h);
    doc.node_mut(child).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(parent, child);

    let surface = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    (doc, surface)
}

// ═══════════════════════════════════════════════════════════════════════
// ── Section 1: Overflow Hidden Clipping (20 tests) ───────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_hidden_clips_child_vertically() {
    // Parent 200x100, child 200x300 → child extends 200px below parent
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 100.0, 200.0, 300.0);
    // Inside parent: content visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content inside overflow:hidden parent should be visible");
    // Below parent: clipped
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "Content below overflow:hidden parent should be clipped");
}

#[test]
fn overflow_hidden_clips_child_horizontally() {
    // Parent 100x200, child 400x200 → child extends 300px right of parent
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 100.0, 200.0, 400.0, 200.0);
    // Inside parent: content visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 80, 180),
        "Content inside parent should be visible");
    // Right of parent: clipped
    assert!(region_is_white(&mut s, 110, 0, 100, 200),
        "Content right of overflow:hidden parent should be clipped");
}

#[test]
fn overflow_hidden_clips_both_axes() {
    // Parent 150x100, child 400x300 → extends in both directions
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 150.0, 100.0, 400.0, 300.0);
    // Inside: visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 130, 80),
        "Content inside parent should render");
    // Right of parent: clipped
    assert!(region_is_white(&mut s, 160, 0, 100, 100),
        "Content beyond right edge should be clipped");
    // Below parent: clipped
    assert!(region_is_white(&mut s, 0, 110, 150, 100),
        "Content below parent should be clipped");
}

#[test]
fn overflow_hidden_preserves_inner_content() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 200.0, 200.0, 200.0);
    // Child fits within parent — everything should be visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 180),
        "Child that fits within overflow:hidden parent should be fully visible");
}

#[test]
fn overflow_hidden_clips_vertical_small_parent() {
    // Very small parent, large child
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 50.0, 30.0, 50.0, 200.0);
    assert!(has_non_white_in_region(&mut s, 5, 5, 40, 20),
        "Small parent should show content within bounds");
    assert!(region_is_white(&mut s, 0, 40, 50, 50),
        "Content below 30px parent should be clipped");
}

#[test]
fn overflow_hidden_clips_horizontal_narrow_parent() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 60.0, 100.0, 300.0, 100.0);
    assert!(has_non_white_in_region(&mut s, 5, 5, 50, 90),
        "Content within 60px parent should be visible");
    assert!(region_is_white(&mut s, 70, 0, 100, 100),
        "Content beyond 60px parent should be clipped");
}

#[test]
fn overflow_hidden_clips_large_parent() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 400.0, 300.0, 400.0, 500.0);
    assert!(has_non_white_in_region(&mut s, 50, 50, 300, 200),
        "Large parent should show content");
    assert!(region_is_white(&mut s, 0, 310, 400, 50),
        "Content below 300px parent should be clipped");
}

#[test]
fn overflow_hidden_child_bg_clipped_at_right_edge() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 100.0, 500.0, 100.0);
    // Well inside: colored
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "Center of parent should have content");
    // Well outside: white
    assert!(pixel_is_white(&mut s, 250, 50),
        "Beyond parent right edge should be white");
}

#[test]
fn overflow_hidden_child_bg_clipped_at_bottom_edge() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 80.0, 200.0, 300.0);
    assert!(pixel_is_not_white(&mut s, 100, 40),
        "Center of parent should have content");
    assert!(pixel_is_white(&mut s, 100, 90),
        "Below parent bottom edge should be white");
}

#[test]
fn overflow_hidden_parent_bg_renders_fully() {
    // Parent with overflow:hidden still renders its own background
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);
    // No children

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Parent bg should fill its bounds
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Parent background should render even with overflow:hidden and no children");
    // Beyond parent: white
    assert!(region_is_white(&mut s, 210, 0, 100, 100),
        "Beyond parent should be white");
}

#[test]
fn overflow_hidden_with_padding_clips_inside_padding_box() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.padding_top = Length::px(10.0);
    doc.node_mut(parent).style.padding_right = Length::px(10.0);
    doc.node_mut(parent).style.padding_bottom = Length::px(10.0);
    doc.node_mut(parent).style.padding_left = Length::px(10.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    // Child larger than content area
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(400.0);
    doc.node_mut(child).style.height = Length::px(300.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Total border-box: 220x120 (200+20 x 100+20)
    // Content inside should be visible
    assert!(has_non_white_in_region(&mut s, 15, 15, 190, 90),
        "Content within padded parent should be visible");
    // Beyond the border box: white
    assert!(region_is_white(&mut s, 230, 0, 100, 120),
        "Content beyond padded parent border-box should be clipped");
}

#[test]
fn overflow_hidden_multiple_children_clipped() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(60.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, parent);

    // Three children, each 30px tall → total 90px, parent only 60px
    for _ in 0..3 {
        add_colored_box(&mut doc, parent, 200.0, 30.0, Color::from_rgba8(200, 0, 0, 255));
    }

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // First 60px visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 40),
        "First two children should be visible");
    // Third child (y=60..90) clipped
    assert!(region_is_white(&mut s, 0, 70, 200, 30),
        "Third child beyond parent height should be clipped");
}

#[test]
fn overflow_hidden_zero_height_clips_everything() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 0.0, 200.0, 100.0);
    // Zero-height parent clips all content
    assert!(region_is_white(&mut s, 0, 5, 200, 50),
        "Zero-height overflow:hidden parent should clip all child content");
}

#[test]
fn overflow_hidden_zero_width_clips_everything() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 0.0, 100.0, 200.0, 100.0);
    assert!(region_is_white(&mut s, 5, 0, 100, 100),
        "Zero-width overflow:hidden parent should clip all child content");
}

#[test]
fn overflow_hidden_exact_fit_child_fully_visible() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 100.0, 200.0, 100.0);
    // Child exactly fits parent — should be fully visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Child that exactly fits should be fully visible");
}

#[test]
fn overflow_hidden_with_border_clips_inside_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.border_top_width = 5;
    doc.node_mut(parent).style.border_right_width = 5;
    doc.node_mut(parent).style.border_bottom_width = 5;
    doc.node_mut(parent).style.border_left_width = 5;
    doc.node_mut(parent).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_top_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(parent).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(parent).style.border_bottom_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(parent).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    // Child extends well beyond parent
    add_colored_box(&mut doc, parent, 400.0, 300.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Border area should have content (border itself)
    assert!(has_non_white_in_region(&mut s, 0, 0, 5, 100),
        "Left border area should have content");
    // Well beyond total border-box (210x110): clipped
    assert!(region_is_white(&mut s, 220, 0, 100, 110),
        "Content beyond border-box should be clipped");
}

#[test]
fn overflow_hidden_surface_correct_dimensions() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 100.0, 400.0, 300.0);
    let image = s.image_snapshot();
    assert_eq!(image.width(), SURFACE_W);
    assert_eq!(image.height(), SURFACE_H);
}

#[test]
fn overflow_hidden_child_wider_parent_bg_still_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, parent);

    // Child only 100px wide, 50px tall — doesn't fill parent
    add_colored_box(&mut doc, parent, 100.0, 50.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Parent bg visible below child (y=50..100 at x=50)
    assert!(has_non_white_in_region(&mut s, 50, 55, 100, 40),
        "Parent background should be visible where child doesn't cover");
}

#[test]
fn overflow_hidden_clips_bottom_right_corner_only() {
    // Child offset so it only overflows bottom-right
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(200.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    // Child taller than parent
    add_colored_box(&mut doc, parent, 200.0, 400.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 180),
        "Content within parent should be visible");
    assert!(region_is_white(&mut s, 0, 210, 200, 50),
        "Content below parent should be clipped");
}

// ═══════════════════════════════════════════════════════════════════════
// ── Section 2: Overflow Visible (15 tests) ───────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_visible_is_default_for_overflow_x() {
    let style = ComputedStyle::initial();
    assert_eq!(style.overflow_x, Overflow::Visible,
        "Default overflow_x should be Visible");
}

#[test]
fn overflow_visible_is_default_for_overflow_y() {
    let style = ComputedStyle::initial();
    assert_eq!(style.overflow_y, Overflow::Visible,
        "Default overflow_y should be Visible");
}

#[test]
fn overflow_visible_enum_not_clipping() {
    assert!(!Overflow::Visible.is_clipping(),
        "Overflow::Visible should NOT be clipping");
}

#[test]
fn overflow_visible_enum_not_scrollable() {
    assert!(!Overflow::Visible.is_scrollable(),
        "Overflow::Visible should NOT be scrollable");
}

#[test]
fn overflow_visible_child_renders_within_parent() {
    let (_, mut s) = build_overflow_test(Overflow::Visible, 200.0, 100.0, 150.0, 80.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 140, 70),
        "Child within visible overflow parent should render");
}

#[test]
fn overflow_visible_child_fills_parent() {
    let (_, mut s) = build_overflow_test(Overflow::Visible, 200.0, 100.0, 200.0, 100.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Child filling parent with overflow:visible should render fully");
}

#[test]
fn overflow_visible_child_extends_vertically() {
    // Child taller than parent with overflow:visible → should NOT be clipped
    let (_, mut s) = build_overflow_test(Overflow::Visible, 200.0, 100.0, 200.0, 300.0);
    // Inside parent: definitely visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content within parent should be visible");
    // Below parent: should still be visible (not clipped)
    assert!(has_non_white_in_region(&mut s, 10, 110, 180, 50),
        "overflow:visible should NOT clip content below parent");
}

#[test]
fn overflow_visible_child_extends_horizontally() {
    // Child wider than parent with overflow:visible
    let (_, mut s) = build_overflow_test(Overflow::Visible, 100.0, 200.0, 400.0, 200.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 80, 180),
        "Content within parent should be visible");
    // Content beyond parent right edge should be visible
    assert!(has_non_white_in_region(&mut s, 110, 10, 100, 180),
        "overflow:visible should NOT clip content beyond right edge");
}

#[test]
fn overflow_visible_parent_bg_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, parent);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Parent with overflow:visible (default) should render its background");
}

#[test]
fn overflow_visible_multiple_children() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(200.0);
    doc.append_child(vp, parent);

    add_colored_box(&mut doc, parent, 200.0, 50.0, Color::from_rgba8(200, 0, 0, 255));
    add_colored_box(&mut doc, parent, 200.0, 50.0, Color::from_rgba8(0, 200, 0, 255));
    add_colored_box(&mut doc, parent, 200.0, 50.0, Color::from_rgba8(0, 0, 200, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 5, 180, 40),
        "First child should be visible");
    assert!(has_non_white_in_region(&mut s, 10, 55, 180, 40),
        "Second child should be visible");
    assert!(has_non_white_in_region(&mut s, 10, 105, 180, 40),
        "Third child should be visible");
}

#[test]
fn overflow_visible_produces_visible_output() {
    let (_, mut s) = build_overflow_test(Overflow::Visible, 300.0, 200.0, 300.0, 200.0);
    assert!(has_visible_content(&mut s),
        "overflow:visible parent with colored child should produce visible output");
}

#[test]
fn overflow_visible_explicit_setting_same_as_default() {
    // Explicit overflow:visible should behave same as unset
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Visible;
    doc.node_mut(parent).style.overflow_y = Overflow::Visible;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    add_colored_box(&mut doc, parent, 200.0, 200.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Child extends vertically — should NOT be clipped
    assert!(has_non_white_in_region(&mut s, 10, 110, 180, 50),
        "Explicit overflow:visible should not clip overflowing content");
}

#[test]
fn overflow_visible_content_not_restricted_by_parent_size() {
    let (_, mut s) = build_overflow_test(Overflow::Visible, 200.0, 50.0, 200.0, 200.0);
    // All 200px of child height should be painted (parent only 50px tall)
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 30),
        "Top of child should be visible");
    assert!(has_non_white_in_region(&mut s, 10, 60, 180, 50),
        "Bottom portion of child beyond parent should also be visible");
}

#[test]
fn overflow_visible_both_axes_overflow() {
    let (_, mut s) = build_overflow_test(Overflow::Visible, 100.0, 80.0, 300.0, 200.0);
    // Check content extends beyond parent in both axes
    assert!(has_non_white_in_region(&mut s, 10, 10, 80, 60),
        "Content inside parent should be visible");
    assert!(has_non_white_in_region(&mut s, 110, 10, 50, 60),
        "Content extending right should be visible");
    assert!(has_non_white_in_region(&mut s, 10, 90, 80, 50),
        "Content extending below should be visible");
}

// ═══════════════════════════════════════════════════════════════════════
// ── Section 3: Overflow Scroll / Auto / Clip (15 tests) ──────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn overflow_scroll_clips_vertically() {
    let (_, mut s) = build_overflow_test(Overflow::Scroll, 200.0, 100.0, 200.0, 300.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content inside scroll parent should be visible");
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "overflow:scroll should clip content below parent");
}

#[test]
fn overflow_scroll_clips_horizontally() {
    let (_, mut s) = build_overflow_test(Overflow::Scroll, 100.0, 200.0, 400.0, 200.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 80, 180),
        "Content inside scroll parent should be visible");
    assert!(region_is_white(&mut s, 110, 0, 100, 200),
        "overflow:scroll should clip content beyond right edge");
}

#[test]
fn overflow_scroll_is_scrollable() {
    assert!(Overflow::Scroll.is_scrollable(),
        "Overflow::Scroll should be scrollable");
}

#[test]
fn overflow_scroll_is_clipping() {
    assert!(Overflow::Scroll.is_clipping(),
        "Overflow::Scroll should be clipping");
}

#[test]
fn overflow_scroll_preserves_inner_content() {
    let (_, mut s) = build_overflow_test(Overflow::Scroll, 200.0, 200.0, 200.0, 200.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 180),
        "Content fitting within scroll parent should be fully visible");
}

#[test]
fn overflow_auto_clips_vertically() {
    let (_, mut s) = build_overflow_test(Overflow::Auto, 200.0, 100.0, 200.0, 300.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content inside auto parent should be visible");
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "overflow:auto should clip content below parent");
}

#[test]
fn overflow_auto_clips_horizontally() {
    let (_, mut s) = build_overflow_test(Overflow::Auto, 100.0, 200.0, 400.0, 200.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 80, 180),
        "Content inside auto parent should be visible");
    assert!(region_is_white(&mut s, 110, 0, 100, 200),
        "overflow:auto should clip content beyond right edge");
}

#[test]
fn overflow_auto_is_scrollable() {
    assert!(Overflow::Auto.is_scrollable(),
        "Overflow::Auto should be scrollable");
}

#[test]
fn overflow_auto_is_clipping() {
    assert!(Overflow::Auto.is_clipping(),
        "Overflow::Auto should be clipping");
}

#[test]
fn overflow_auto_preserves_inner_content() {
    let (_, mut s) = build_overflow_test(Overflow::Auto, 200.0, 200.0, 200.0, 200.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 180),
        "Content fitting within auto parent should be fully visible");
}

#[test]
fn overflow_clip_clips_vertically() {
    let (_, mut s) = build_overflow_test(Overflow::Clip, 200.0, 100.0, 200.0, 300.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content inside clip parent should be visible");
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "overflow:clip should clip content below parent");
}

#[test]
fn overflow_clip_not_scrollable() {
    assert!(!Overflow::Clip.is_scrollable(),
        "Overflow::Clip should NOT be scrollable");
}

#[test]
fn overflow_clip_is_clipping() {
    assert!(Overflow::Clip.is_clipping(),
        "Overflow::Clip should be clipping");
}

#[test]
fn overflow_all_default_to_visible() {
    let s = ComputedStyle::initial();
    assert_eq!(s.overflow_x, Overflow::Visible);
    assert_eq!(s.overflow_y, Overflow::Visible);
    assert!(!Overflow::Visible.is_clipping());
    assert!(!Overflow::Visible.is_scrollable());
}

#[test]
fn overflow_all_clipping_modes_clip_identically() {
    // Hidden, Scroll, Auto, Clip all produce the same visual clip
    let modes = [Overflow::Hidden, Overflow::Scroll, Overflow::Auto, Overflow::Clip];
    for mode in &modes {
        let (_, mut s) = build_overflow_test(*mode, 200.0, 100.0, 200.0, 300.0);
        assert!(region_is_white(&mut s, 0, 110, 200, 50),
            "overflow:{:?} should clip content below parent", mode);
    }
}

// ═══════════════════════════════════════════════════════════════════════
// ── Section 4: Min/Max Width (15 tests) ──────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn min_width_prevents_element_from_being_too_narrow() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(100.0);
    doc.node_mut(div).style.min_width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // min-width:200 > width:100, so element should be 200px wide
    assert!(pixel_is_not_white(&mut s, 150, 25),
        "min-width should make element at least 200px wide (pixel at x=150 should have content)");
}

#[test]
fn min_width_no_effect_when_smaller_than_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(300.0);
    doc.node_mut(div).style.min_width = Length::px(100.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // width:300 > min-width:100, so element should be 300px wide
    assert!(pixel_is_not_white(&mut s, 250, 25),
        "Element should be 300px wide since width > min-width");
}

#[test]
fn max_width_caps_element_size() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(400.0);
    doc.node_mut(div).style.max_width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // max-width:200 < width:400, so element capped at 200px
    assert!(pixel_is_not_white(&mut s, 100, 25),
        "Element should have content within 200px");
    assert!(pixel_is_white(&mut s, 250, 25),
        "max-width should cap element at 200px (no content at x=250)");
}

#[test]
fn max_width_no_effect_when_larger_than_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.max_width = Length::px(400.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 25),
        "Element should be 200px wide since width < max-width");
    assert!(pixel_is_white(&mut s, 250, 25),
        "Element should not extend beyond its 200px width");
}

#[test]
fn min_width_with_auto_width_in_constrained_parent() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_block(&mut doc, vp, 150.0);
    doc.node_mut(parent).style.height = Length::px(100.0);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    // width: auto, min-width: 200px
    doc.node_mut(child).style.min_width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(50.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // min-width should override the auto width from 150px parent
    assert!(pixel_is_not_white(&mut s, 170, 25),
        "min-width should make child wider than parent's 150px");
}

#[test]
fn max_width_constrains_auto_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    // Parent is 400px wide
    let parent = add_block(&mut doc, vp, 400.0);
    doc.node_mut(parent).style.height = Length::px(100.0);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    // width: auto (would fill parent's 400px), max-width: 200px
    doc.node_mut(child).style.max_width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(50.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 25),
        "Content within 200px should be visible");
    assert!(pixel_is_white(&mut s, 250, 25),
        "max-width should constrain auto width to 200px");
}

#[test]
fn min_max_width_both_set() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(50.0);
    doc.node_mut(div).style.min_width = Length::px(150.0);
    doc.node_mut(div).style.max_width = Length::px(300.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // width:50 clamped by min:150 → 150px
    assert!(pixel_is_not_white(&mut s, 100, 25),
        "Element should be at least 150px (min-width)");
    assert!(pixel_is_white(&mut s, 310, 25),
        "Element should not exceed 300px (max-width)");
}

#[test]
fn min_width_zero_has_no_effect() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.min_width = Length::px(0.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 25),
        "Element should be 200px wide with min-width:0");
}

#[test]
fn max_width_none_no_constraint() {
    let style = ComputedStyle::initial();
    assert!(style.max_width.is_none(),
        "Default max-width should be none (unconstrained)");
}

#[test]
fn min_width_on_nested_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_block(&mut doc, vp, 300.0);
    doc.node_mut(parent).style.height = Length::px(200.0);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(100.0);
    doc.node_mut(child).style.min_width = Length::px(250.0);
    doc.node_mut(child).style.height = Length::px(80.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 200, 40),
        "Nested child with min-width:250 should extend to at least 250px");
}

#[test]
fn max_width_on_nested_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_block(&mut doc, vp, 400.0);
    doc.node_mut(parent).style.height = Length::px(200.0);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    // width: auto (fills 400px), max-width: 150px
    doc.node_mut(child).style.max_width = Length::px(150.0);
    doc.node_mut(child).style.height = Length::px(80.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 75, 40),
        "Content within 150px should be visible");
    assert!(pixel_is_white(&mut s, 200, 40),
        "max-width:150 should prevent child from filling 400px parent");
}

#[test]
fn min_width_with_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(100.0);
    doc.node_mut(div).style.min_width = Length::px(200.0);
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    doc.node_mut(div).style.padding_right = Length::px(20.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_visible_content(&mut s),
        "Element with min-width and padding should render");
    // min-width:200 (content) + 40px padding = 240px total border-box
    assert!(pixel_is_not_white(&mut s, 100, 25),
        "Element with min-width:200 + padding should be at least 200px content");
}

#[test]
fn max_width_with_border() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(400.0);
    doc.node_mut(div).style.max_width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_visible_content(&mut s),
        "Element with max-width and border should render");
    // max-width:200 (content) + 10px border = 210px total
    assert!(pixel_is_white(&mut s, 260, 25),
        "max-width should constrain element even with borders");
}

// ═══════════════════════════════════════════════════════════════════════
// ── Section 5: Min/Max Height (15 tests) ─────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn min_height_prevents_element_from_being_too_short() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(50.0);
    doc.node_mut(div).style.min_height = Length::px(150.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // min-height:150 > height:50, so element should be 150px tall
    assert!(pixel_is_not_white(&mut s, 100, 120),
        "min-height should make element at least 150px tall (pixel at y=120)");
}

#[test]
fn min_height_no_effect_when_smaller_than_height() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(200.0);
    doc.node_mut(div).style.min_height = Length::px(50.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // height:200 > min-height:50, so element stays 200px
    assert!(pixel_is_not_white(&mut s, 100, 150),
        "Element should be 200px tall since height > min-height");
}

#[test]
fn max_height_caps_element_size() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(400.0);
    doc.node_mut(div).style.max_height = Length::px(150.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 75),
        "Content within max-height should be visible");
    assert!(pixel_is_white(&mut s, 100, 160),
        "max-height should cap element at 150px");
}

#[test]
fn max_height_no_effect_when_larger_than_height() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.max_height = Length::px(300.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "Element should be 100px tall");
    assert!(pixel_is_white(&mut s, 100, 110),
        "Element should not extend beyond its 100px height");
}

#[test]
fn min_height_with_auto_height() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    // height: auto (default), min-height: 100px
    doc.node_mut(div).style.min_height = Length::px(100.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);
    // No children, so auto height would be 0
    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // min-height should ensure element is at least 100px tall
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "min-height should make auto-height element at least 100px");
}

#[test]
fn max_height_with_auto_height_and_tall_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    // height: auto, max-height: 100px
    doc.node_mut(parent).style.max_height = Length::px(100.0);
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, parent);

    // Child is 300px tall → parent would grow but max-height caps it
    add_colored_box(&mut doc, parent, 200.0, 300.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content within max-height should be visible");
    // max-height caps parent at 100px. Child may still render below
    // (no overflow clip by default), but parent bg stops at 100px.
}

#[test]
fn min_max_height_both_set() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(30.0);
    doc.node_mut(div).style.min_height = Length::px(100.0);
    doc.node_mut(div).style.max_height = Length::px(250.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // height:30 clamped by min:100 → 100px
    assert!(pixel_is_not_white(&mut s, 100, 80),
        "Element should be at least 100px tall (min-height)");
}

#[test]
fn min_height_zero_no_effect() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.min_height = Length::px(0.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "Element should be 100px tall with min-height:0");
}

#[test]
fn max_height_none_no_constraint() {
    let style = ComputedStyle::initial();
    assert!(style.max_height.is_none(),
        "Default max-height should be none (unconstrained)");
}

#[test]
fn max_height_with_overflow_hidden_clips_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.max_height = Length::px(80.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, parent);

    add_colored_box(&mut doc, parent, 200.0, 300.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 60),
        "Content inside max-height + hidden should be visible");
    assert!(region_is_white(&mut s, 0, 90, 200, 50),
        "Content beyond max-height with overflow:hidden should be clipped");
}

#[test]
fn min_height_on_nested_element() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_block(&mut doc, vp, 300.0);
    doc.node_mut(parent).style.height = Length::px(300.0);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(30.0);
    doc.node_mut(child).style.min_height = Length::px(120.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 100),
        "Nested element with min-height:120 should extend to y=120");
}

#[test]
fn max_height_on_nested_element() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = add_block(&mut doc, vp, 300.0);
    doc.node_mut(parent).style.height = Length::px(400.0);

    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.width = Length::px(200.0);
    doc.node_mut(child).style.height = Length::px(300.0);
    doc.node_mut(child).style.max_height = Length::px(100.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "Nested element within max-height should be visible");
    assert!(pixel_is_white(&mut s, 100, 110),
        "Nested element should not exceed max-height:100");
}

#[test]
fn min_height_with_colored_background() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.min_height = Length::px(80.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Auto height with no children = 0, but min-height makes it 80px
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 60),
        "Background should fill min-height area");
}

// ═══════════════════════════════════════════════════════════════════════
// ── Section 6: Box Sizing (10 tests) ─────────────────────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn box_sizing_content_box_is_default() {
    let style = ComputedStyle::initial();
    assert_eq!(style.box_sizing, BoxSizing::ContentBox,
        "Default box-sizing should be content-box");
}

#[test]
fn content_box_padding_adds_to_total_size() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::ContentBox;
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    doc.node_mut(div).style.padding_right = Length::px(20.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // content-box: total = 200 + 20 + 20 = 240px wide
    assert!(pixel_is_not_white(&mut s, 230, 50),
        "content-box with padding 20+20 on width 200 should be 240px total");
}

#[test]
fn border_box_padding_included_in_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    doc.node_mut(div).style.padding_right = Length::px(20.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // border-box: total = 200px (padding included)
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "border-box element should have content within 200px");
    assert!(pixel_is_white(&mut s, 210, 50),
        "border-box with width:200 should not extend beyond 200px");
}

#[test]
fn content_box_border_adds_to_total_size() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::ContentBox;
    doc.node_mut(div).style.border_left_width = 10;
    doc.node_mut(div).style.border_right_width = 10;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // content-box: total = 200 + 10 + 10 = 220px
    assert!(pixel_is_not_white(&mut s, 215, 50),
        "content-box with border 10+10 on width 200 should extend to 220px");
}

#[test]
fn border_box_border_included_in_width() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(div).style.border_left_width = 10;
    doc.node_mut(div).style.border_right_width = 10;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // border-box: total = 200px (borders included)
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "border-box element should render within 200px");
    assert!(pixel_is_white(&mut s, 210, 50),
        "border-box with width:200 should not extend beyond 200px even with borders");
}

#[test]
fn content_box_padding_and_border_cumulative() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::ContentBox;
    doc.node_mut(div).style.padding_left = Length::px(15.0);
    doc.node_mut(div).style.padding_right = Length::px(15.0);
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // content-box: total = 200 + 30 (padding) + 10 (border) = 240px
    assert!(pixel_is_not_white(&mut s, 235, 50),
        "content-box with padding+border should extend to 240px");
}

#[test]
fn border_box_padding_and_border_included() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(div).style.padding_left = Length::px(15.0);
    doc.node_mut(div).style.padding_right = Length::px(15.0);
    doc.node_mut(div).style.border_left_width = 5;
    doc.node_mut(div).style.border_right_width = 5;
    doc.node_mut(div).style.border_left_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(div).style.border_left_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.border_right_color = StyleColor::Resolved(Color::BLACK);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // border-box: total = 200px (everything included)
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "border-box element should render");
    assert!(pixel_is_white(&mut s, 210, 50),
        "border-box total should be exactly 200px");
}

#[test]
fn content_box_vs_border_box_different_total_size() {
    // Same width:200, padding:20 → content-box is wider than border-box
    let mut doc1 = Document::new();
    let vp1 = setup_viewport(&mut doc1);
    let div1 = doc1.create_node(ElementTag::Div);
    doc1.node_mut(div1).style.display = Display::Block;
    doc1.node_mut(div1).style.width = Length::px(200.0);
    doc1.node_mut(div1).style.height = Length::px(50.0);
    doc1.node_mut(div1).style.box_sizing = BoxSizing::ContentBox;
    doc1.node_mut(div1).style.padding_left = Length::px(20.0);
    doc1.node_mut(div1).style.padding_right = Length::px(20.0);
    doc1.node_mut(div1).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc1.append_child(vp1, div1);
    let mut s1 = render_to_surface(&doc1, SURFACE_W, SURFACE_H).unwrap();

    let mut doc2 = Document::new();
    let vp2 = setup_viewport(&mut doc2);
    let div2 = doc2.create_node(ElementTag::Div);
    doc2.node_mut(div2).style.display = Display::Block;
    doc2.node_mut(div2).style.width = Length::px(200.0);
    doc2.node_mut(div2).style.height = Length::px(50.0);
    doc2.node_mut(div2).style.box_sizing = BoxSizing::BorderBox;
    doc2.node_mut(div2).style.padding_left = Length::px(20.0);
    doc2.node_mut(div2).style.padding_right = Length::px(20.0);
    doc2.node_mut(div2).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc2.append_child(vp2, div2);
    let mut s2 = render_to_surface(&doc2, SURFACE_W, SURFACE_H).unwrap();

    // content-box: 240px total, border-box: 200px total
    // At x=210: content-box has content, border-box does not
    let cb_has_content = pixel_is_not_white(&mut s1, 210, 25);
    let bb_has_content = pixel_is_not_white(&mut s2, 210, 25);
    assert!(cb_has_content && !bb_has_content,
        "content-box should be wider than border-box at x=210");
}

#[test]
fn border_box_vertical_sizing() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(div).style.padding_top = Length::px(15.0);
    doc.node_mut(div).style.padding_bottom = Length::px(15.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // border-box: height 100px includes padding
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "border-box with height:100 should render within 100px");
    assert!(pixel_is_white(&mut s, 100, 110),
        "border-box with height:100 should not extend beyond 100px");
}

// ═══════════════════════════════════════════════════════════════════════
// ── Section 7: Complex Overflow Scenarios (15 tests) ─────────────────
// ═══════════════════════════════════════════════════════════════════════

#[test]
fn nested_overflow_hidden_inner_clips_further() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    // Outer: 300x200, overflow:hidden
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(300.0);
    doc.node_mut(outer).style.height = Length::px(200.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Hidden;
    doc.node_mut(outer).style.overflow_y = Overflow::Hidden;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, outer);

    // Inner: 200x100, overflow:hidden
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.width = Length::px(200.0);
    doc.node_mut(inner).style.height = Length::px(100.0);
    doc.node_mut(inner).style.overflow_x = Overflow::Hidden;
    doc.node_mut(inner).style.overflow_y = Overflow::Hidden;
    doc.node_mut(inner).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(outer, inner);

    // Grandchild: large, red
    add_colored_box(&mut doc, inner, 500.0, 500.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Inside inner box: content
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content inside inner box should be visible");
    // Between inner and outer (x=210, y=50): outer bg only (inner clips children)
    assert!(region_is_white(&mut s, 310, 0, 100, 200),
        "Content beyond outer box should be clipped");
}

#[test]
fn nested_overflow_outer_hidden_inner_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    // Outer: overflow:hidden
    let outer = doc.create_node(ElementTag::Div);
    doc.node_mut(outer).style.display = Display::Block;
    doc.node_mut(outer).style.width = Length::px(200.0);
    doc.node_mut(outer).style.height = Length::px(100.0);
    doc.node_mut(outer).style.overflow_x = Overflow::Hidden;
    doc.node_mut(outer).style.overflow_y = Overflow::Hidden;
    doc.node_mut(outer).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, outer);

    // Inner: overflow:visible (default)
    let inner = doc.create_node(ElementTag::Div);
    doc.node_mut(inner).style.display = Display::Block;
    doc.node_mut(inner).style.width = Length::px(200.0);
    doc.node_mut(inner).style.height = Length::px(50.0);
    doc.append_child(outer, inner);

    add_colored_box(&mut doc, inner, 200.0, 200.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Outer clips at 100px height, even though inner is overflow:visible
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "Outer overflow:hidden should clip inner's visible overflow");
}

#[test]
fn three_level_nesting_overflow_hidden() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let l1 = doc.create_node(ElementTag::Div);
    doc.node_mut(l1).style.display = Display::Block;
    doc.node_mut(l1).style.width = Length::px(300.0);
    doc.node_mut(l1).style.height = Length::px(250.0);
    doc.node_mut(l1).style.overflow_x = Overflow::Hidden;
    doc.node_mut(l1).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, l1);

    let l2 = doc.create_node(ElementTag::Div);
    doc.node_mut(l2).style.display = Display::Block;
    doc.node_mut(l2).style.width = Length::px(250.0);
    doc.node_mut(l2).style.height = Length::px(200.0);
    doc.node_mut(l2).style.overflow_x = Overflow::Hidden;
    doc.node_mut(l2).style.overflow_y = Overflow::Hidden;
    doc.append_child(l1, l2);

    let l3 = doc.create_node(ElementTag::Div);
    doc.node_mut(l3).style.display = Display::Block;
    doc.node_mut(l3).style.width = Length::px(200.0);
    doc.node_mut(l3).style.height = Length::px(150.0);
    doc.node_mut(l3).style.overflow_x = Overflow::Hidden;
    doc.node_mut(l3).style.overflow_y = Overflow::Hidden;
    doc.append_child(l2, l3);

    add_colored_box(&mut doc, l3, 600.0, 600.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 130),
        "Content inside innermost box should render");
    assert!(region_is_white(&mut s, 310, 0, 100, 250),
        "Nothing should leak beyond outermost box");
}

#[test]
fn overflow_hidden_with_colored_borders_visible() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.border_top_width = 5;
    doc.node_mut(parent).style.border_right_width = 5;
    doc.node_mut(parent).style.border_bottom_width = 5;
    doc.node_mut(parent).style.border_left_width = 5;
    doc.node_mut(parent).style.border_top_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_right_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_bottom_style = BorderStyle::Solid;
    doc.node_mut(parent).style.border_left_style = BorderStyle::Solid;
    let bcolor = Color::from_rgba8(200, 0, 0, 255);
    doc.node_mut(parent).style.border_top_color = StyleColor::Resolved(bcolor);
    doc.node_mut(parent).style.border_right_color = StyleColor::Resolved(bcolor);
    doc.node_mut(parent).style.border_bottom_color = StyleColor::Resolved(bcolor);
    doc.node_mut(parent).style.border_left_color = StyleColor::Resolved(bcolor);
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    add_colored_box(&mut doc, parent, 400.0, 300.0, Color::from_rgba8(0, 200, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Border area should be visible
    assert!(has_non_white_in_region(&mut s, 0, 0, 5, 100),
        "Left border should be painted");
    // Content inside should be visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 190, 80),
        "Content within border should be visible");
    // Beyond border-box (210x110): nothing
    assert!(region_is_white(&mut s, 220, 0, 100, 110),
        "Nothing beyond border-box");
}

#[test]
fn overflow_scroll_with_nested_boxes() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let scroll_box = doc.create_node(ElementTag::Div);
    doc.node_mut(scroll_box).style.display = Display::Block;
    doc.node_mut(scroll_box).style.width = Length::px(200.0);
    doc.node_mut(scroll_box).style.height = Length::px(100.0);
    doc.node_mut(scroll_box).style.overflow_x = Overflow::Scroll;
    doc.node_mut(scroll_box).style.overflow_y = Overflow::Scroll;
    doc.node_mut(scroll_box).style.background_color = Color::from_rgba8(200, 200, 200, 255);
    doc.append_child(vp, scroll_box);

    add_colored_box(&mut doc, scroll_box, 200.0, 50.0, Color::from_rgba8(200, 0, 0, 255));
    add_colored_box(&mut doc, scroll_box, 200.0, 50.0, Color::from_rgba8(0, 200, 0, 255));
    add_colored_box(&mut doc, scroll_box, 200.0, 50.0, Color::from_rgba8(0, 0, 200, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // First two children visible (100px total), third clipped
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 40),
        "First child in scroll box should be visible");
    assert!(has_non_white_in_region(&mut s, 10, 60, 180, 30),
        "Second child in scroll box should be visible");
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "Third child should be clipped by scroll overflow");
}

#[test]
fn overflow_auto_with_tall_content() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(80.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Auto;
    doc.node_mut(parent).style.overflow_y = Overflow::Auto;
    doc.append_child(vp, parent);

    add_colored_box(&mut doc, parent, 200.0, 200.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 60),
        "Visible portion should render");
    assert!(region_is_white(&mut s, 0, 90, 200, 50),
        "Tall content should be clipped by overflow:auto");
}

#[test]
fn overflow_clip_with_wide_content() {
    let (_, mut s) = build_overflow_test(Overflow::Clip, 150.0, 100.0, 500.0, 100.0);
    assert!(has_non_white_in_region(&mut s, 10, 10, 130, 80),
        "Content inside clip box should render");
    assert!(region_is_white(&mut s, 160, 0, 100, 100),
        "Wide content should be clipped by overflow:clip");
}

#[test]
fn overflow_hidden_surface_produces_output() {
    let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, 100.0, 200.0, 100.0);
    assert!(has_visible_content(&mut s),
        "overflow:hidden with colored boxes should produce visible output");
}

#[test]
fn multiple_overflow_containers_stacked() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    // Two stacked overflow:hidden containers
    let box1 = doc.create_node(ElementTag::Div);
    doc.node_mut(box1).style.display = Display::Block;
    doc.node_mut(box1).style.width = Length::px(200.0);
    doc.node_mut(box1).style.height = Length::px(80.0);
    doc.node_mut(box1).style.overflow_x = Overflow::Hidden;
    doc.node_mut(box1).style.overflow_y = Overflow::Hidden;
    doc.node_mut(box1).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, box1);
    add_colored_box(&mut doc, box1, 200.0, 200.0, Color::from_rgba8(0, 200, 0, 255));

    let box2 = doc.create_node(ElementTag::Div);
    doc.node_mut(box2).style.display = Display::Block;
    doc.node_mut(box2).style.width = Length::px(200.0);
    doc.node_mut(box2).style.height = Length::px(80.0);
    doc.node_mut(box2).style.overflow_x = Overflow::Hidden;
    doc.node_mut(box2).style.overflow_y = Overflow::Hidden;
    doc.node_mut(box2).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, box2);
    add_colored_box(&mut doc, box2, 200.0, 200.0, Color::from_rgba8(200, 200, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // First box (y=0..80)
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 60),
        "First overflow box should render");
    // Second box starts at y=80
    assert!(has_non_white_in_region(&mut s, 10, 90, 180, 60),
        "Second overflow box should render below first");
    // Between the two boxes: should transition (first ends at y=80, second starts at y=80)
    // After both (y>160): white
    assert!(region_is_white(&mut s, 0, 170, 200, 50),
        "Nothing should render below both 80px boxes");
}

#[test]
fn overflow_hidden_with_zero_dimension_element() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    // Zero-height child
    add_colored_box(&mut doc, parent, 200.0, 0.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Parent bg should still be visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Parent bg should render even with zero-height child");
}

#[test]
fn overflow_hidden_with_bg_and_no_children() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(128, 128, 128, 255);
    doc.append_child(vp, parent);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Overflow:hidden box with bg and no children should render background");
    assert!(region_is_white(&mut s, 210, 0, 100, 100),
        "Background should not extend beyond box");
}

#[test]
fn all_overflow_modes_render_something() {
    let modes = [
        Overflow::Visible,
        Overflow::Hidden,
        Overflow::Scroll,
        Overflow::Auto,
        Overflow::Clip,
    ];
    for mode in &modes {
        let (_, mut s) = build_overflow_test(*mode, 200.0, 100.0, 200.0, 100.0);
        assert!(has_visible_content(&mut s),
            "overflow:{:?} should produce visible output with colored boxes", mode);
    }
}

#[test]
fn overflow_hidden_child_partially_visible() {
    // Child starts visible but extends beyond parent
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, parent);

    // First child takes 70px, second child 80px → 150px total, parent is 100px
    add_colored_box(&mut doc, parent, 200.0, 70.0, Color::from_rgba8(200, 0, 0, 255));
    add_colored_box(&mut doc, parent, 200.0, 80.0, Color::from_rgba8(0, 200, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // First child fully visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 50),
        "First child should be fully visible");
    // Second child partially visible (y=70..100) and partially clipped (y=100..150)
    assert!(has_non_white_in_region(&mut s, 10, 75, 180, 20),
        "Top of second child should be visible");
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "Bottom of second child should be clipped");
}

#[test]
fn overflow_hidden_with_max_height_combined() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    // height: auto, max-height: 100px, overflow: hidden
    doc.node_mut(parent).style.max_height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, parent);

    add_colored_box(&mut doc, parent, 200.0, 300.0, Color::from_rgba8(200, 0, 0, 255));

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 80),
        "Content within max-height should render");
    assert!(region_is_white(&mut s, 0, 110, 200, 50),
        "Content beyond max-height + overflow:hidden should be clipped");
}

#[test]
fn overflow_hidden_preserves_sibling_after_clipped_box() {
    // A sibling after an overflow:hidden box should render normally
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let hidden_box = doc.create_node(ElementTag::Div);
    doc.node_mut(hidden_box).style.display = Display::Block;
    doc.node_mut(hidden_box).style.width = Length::px(200.0);
    doc.node_mut(hidden_box).style.height = Length::px(50.0);
    doc.node_mut(hidden_box).style.overflow_x = Overflow::Hidden;
    doc.node_mut(hidden_box).style.overflow_y = Overflow::Hidden;
    doc.node_mut(hidden_box).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(vp, hidden_box);
    add_colored_box(&mut doc, hidden_box, 200.0, 200.0, Color::from_rgba8(0, 200, 0, 255));

    // Sibling after the overflow:hidden box
    let sibling = doc.create_node(ElementTag::Div);
    doc.node_mut(sibling).style.display = Display::Block;
    doc.node_mut(sibling).style.width = Length::px(200.0);
    doc.node_mut(sibling).style.height = Length::px(50.0);
    doc.node_mut(sibling).style.background_color = Color::from_rgba8(0, 0, 200, 255);
    doc.append_child(vp, sibling);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Hidden box at y=0..50
    assert!(has_non_white_in_region(&mut s, 10, 10, 180, 30),
        "Overflow:hidden box should render");
    // Sibling at y=50..100
    assert!(has_non_white_in_region(&mut s, 10, 60, 180, 30),
        "Sibling after overflow:hidden box should render normally");
}

#[test]
fn overflow_hidden_clips_at_varying_widths() {
    // Verify clipping at multiple different widths
    for width in [50.0_f32, 100.0, 200.0, 300.0] {
        let (_, mut s) = build_overflow_test(Overflow::Hidden, width, 80.0, width + 200.0, 80.0);
        let check_x = (width as i32) + 20;
        assert!(pixel_is_white(&mut s, check_x, 40),
            "Content at x={} should be clipped for parent width={}", check_x, width);
    }
}

#[test]
fn min_width_initial_value_is_auto() {
    let style = ComputedStyle::initial();
    assert!(style.min_width.is_auto(),
        "Default min-width should be auto");
}

#[test]
fn min_height_initial_value_is_auto() {
    let style = ComputedStyle::initial();
    assert!(style.min_height.is_auto(),
        "Default min-height should be auto");
}

#[test]
fn box_sizing_border_box_with_height_and_padding() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let div = doc.create_node(ElementTag::Div);
    doc.node_mut(div).style.display = Display::Block;
    doc.node_mut(div).style.width = Length::px(200.0);
    doc.node_mut(div).style.height = Length::px(100.0);
    doc.node_mut(div).style.box_sizing = BoxSizing::BorderBox;
    doc.node_mut(div).style.padding_top = Length::px(20.0);
    doc.node_mut(div).style.padding_bottom = Length::px(20.0);
    doc.node_mut(div).style.padding_left = Length::px(20.0);
    doc.node_mut(div).style.padding_right = Length::px(20.0);
    doc.node_mut(div).style.background_color = Color::from_rgba8(0, 200, 0, 255);
    doc.append_child(vp, div);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // border-box: total = 200x100 (padding included)
    assert!(pixel_is_not_white(&mut s, 100, 50),
        "border-box element should render");
    assert!(pixel_is_white(&mut s, 210, 50),
        "Width should be exactly 200px");
    assert!(pixel_is_white(&mut s, 100, 110),
        "Height should be exactly 100px");
}

#[test]
fn overflow_hidden_clips_at_varying_heights() {
    for height in [40.0_f32, 80.0, 120.0, 200.0] {
        let (_, mut s) = build_overflow_test(Overflow::Hidden, 200.0, height, 200.0, height + 200.0);
        let check_y = (height as i32) + 20;
        assert!(pixel_is_white(&mut s, 100, check_y),
            "Content at y={} should be clipped for parent height={}", check_y, height);
    }
}

#[test]
fn overflow_hidden_parent_and_child_same_bg_color() {
    // When parent and child have same bg, parent area should be uniformly colored
    let color = Color::from_rgba8(100, 100, 200, 255);
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);
    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(200.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.node_mut(parent).style.background_color = color;
    doc.append_child(vp, parent);

    add_colored_box(&mut doc, parent, 200.0, 200.0, color);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Two pixels within parent should have the same color
    assert!(pixels_match(&mut s, 50, 25, 150, 75),
        "Parent area should be uniformly colored");
    // Beyond parent should be white
    assert!(pixel_is_white(&mut s, 100, 110),
        "Beyond parent should be white");
}

#[test]
fn overflow_hidden_with_min_width_child() {
    let mut doc = Document::new();
    let vp = setup_viewport(&mut doc);

    let parent = doc.create_node(ElementTag::Div);
    doc.node_mut(parent).style.display = Display::Block;
    doc.node_mut(parent).style.width = Length::px(100.0);
    doc.node_mut(parent).style.height = Length::px(100.0);
    doc.node_mut(parent).style.overflow_x = Overflow::Hidden;
    doc.node_mut(parent).style.overflow_y = Overflow::Hidden;
    doc.append_child(vp, parent);

    // Child with min-width wider than parent
    let child = doc.create_node(ElementTag::Div);
    doc.node_mut(child).style.display = Display::Block;
    doc.node_mut(child).style.min_width = Length::px(300.0);
    doc.node_mut(child).style.height = Length::px(50.0);
    doc.node_mut(child).style.background_color = Color::from_rgba8(200, 0, 0, 255);
    doc.append_child(parent, child);

    let mut s = render_to_surface(&doc, SURFACE_W, SURFACE_H).unwrap();
    // Inside parent: visible
    assert!(has_non_white_in_region(&mut s, 10, 10, 80, 30),
        "Content inside parent should be visible");
    // Beyond parent: clipped despite min-width
    assert!(region_is_white(&mut s, 110, 0, 100, 100),
        "Content beyond parent should be clipped by overflow:hidden");
}
