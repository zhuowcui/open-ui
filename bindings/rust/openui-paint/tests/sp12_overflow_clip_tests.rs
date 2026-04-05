//! Tests for SP12 F2 — overflow clipping with save/clip_rect/restore and
//! border-radius support.
//!
//! Validates `paint_with_overflow_clip()`, `compute_clip_rect()`, border-radius
//! `clip_rrect`, scroll offset stubs, and canvas save/restore balance.

use std::sync::Arc;

use skia_safe::{surfaces, Color as SkColor, Surface};

use openui_dom::{Document, ElementTag};
use openui_geometry::{BoxStrut, LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::Fragment;
use openui_paint::{compute_clip_rect, paint_fragment, text_painter};
use openui_style::*;
use openui_text::{Font, FontDescription, TextDirection, TextShaper};

// ── Helpers ──────────────────────────────────────────────────────────

fn make_surface(width: i32, height: i32) -> Surface {
    let mut surface = surfaces::raster_n32_premul((width, height))
        .expect("Failed to create Skia surface");
    surface.canvas().clear(SkColor::WHITE);
    surface
}

fn shape_text(text: &str) -> openui_text::ShapeResult {
    let mut desc = FontDescription::new();
    desc.size = 16.0;
    desc.specified_size = 16.0;
    let font = Font::new(desc);
    let shaper = TextShaper::new();
    shaper.shape(text, &font, TextDirection::Ltr)
}

/// Check if any pixels in the given region are non-white.
fn has_non_white_pixels(surface: &mut Surface, x: i32, y: i32, w: i32, h: i32) -> bool {
    let image = surface.image_snapshot();
    let info = image.image_info();
    let row_bytes = info.min_row_bytes();
    let mut pixels = vec![0u8; info.height() as usize * row_bytes];
    image.read_pixels(&info, &mut pixels, row_bytes, (0, 0), skia_safe::image::CachingHint::Allow);

    let bpp = 4;
    let img_w = info.width() as i32;
    for py in y..(y + h).min(info.height()) {
        for px in x..(x + w).min(img_w) {
            let off = (py as usize) * row_bytes + (px as usize) * bpp;
            if off + 3 < pixels.len() {
                let (b, g, r) = (pixels[off], pixels[off + 1], pixels[off + 2]);
                if r != 0xFF || g != 0xFF || b != 0xFF {
                    return true;
                }
            }
        }
    }
    false
}

/// Build a minimal box fragment with the given size and optional borders.
fn make_box_fragment(
    doc: &mut Document,
    parent: openui_dom::NodeId,
    width: f32,
    height: f32,
    border: f32,
) -> (openui_dom::NodeId, Fragment) {
    let node = doc.create_node(ElementTag::Div);
    doc.node_mut(node).style.display = Display::Block;
    doc.node_mut(node).style.width = Length::px(width);
    doc.node_mut(node).style.height = Length::px(height);
    if border > 0.0 {
        let bw = border as i32;
        doc.node_mut(node).style.border_top_width = bw;
        doc.node_mut(node).style.border_right_width = bw;
        doc.node_mut(node).style.border_bottom_width = bw;
        doc.node_mut(node).style.border_left_width = bw;
        doc.node_mut(node).style.border_top_style = BorderStyle::Solid;
        doc.node_mut(node).style.border_right_style = BorderStyle::Solid;
        doc.node_mut(node).style.border_bottom_style = BorderStyle::Solid;
        doc.node_mut(node).style.border_left_style = BorderStyle::Solid;
    }
    doc.append_child(parent, node);
    let mut frag = Fragment::new_box(
        node,
        PhysicalSize::new(LayoutUnit::from_f32(width), LayoutUnit::from_f32(height)),
    );
    if border > 0.0 {
        frag.border = BoxStrut::new(
            LayoutUnit::from_f32(border),
            LayoutUnit::from_f32(border),
            LayoutUnit::from_f32(border),
            LayoutUnit::from_f32(border),
        );
    }
    (node, frag)
}

/// Build a text child fragment from shaped text.
fn make_text_child(
    doc: &mut Document,
    parent_node: openui_dom::NodeId,
    text: &str,
) -> Fragment {
    let sr = Arc::new(shape_text(text));
    let metrics = text_painter::metrics_from_shape_result(&sr);
    let text_node = doc.create_node(ElementTag::Text);
    doc.node_mut(text_node).text = Some(text.to_string());
    doc.node_mut(text_node).style.color = Color::BLACK;
    doc.append_child(parent_node, text_node);

    let text_width = sr.width;
    let text_height = metrics.ascent + metrics.descent;
    let mut frag = Fragment::new_text(
        text_node,
        PhysicalSize::new(
            LayoutUnit::from_f32(text_width),
            LayoutUnit::from_f32(text_height),
        ),
        sr,
        text.to_string(),
    );
    frag.baseline_offset = metrics.ascent;
    frag
}

// ── Tests ────────────────────────────────────────────────────────────

/// 1. overflow:hidden clips children to the padding box.
#[test]
fn overflow_hidden_clips_to_padding_box() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 50.0, 30.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(node).style.overflow_y = Overflow::Hidden;
    frag.has_overflow_clip = true;

    let text_frag = make_text_child(&mut doc, node, "XXXXXXXXXXXXXXXXXXXX");
    frag.children.push(text_frag);

    let mut surface = make_surface(200, 50);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Beyond x=50 should be empty (clipped).
    assert!(
        !has_non_white_pixels(&mut surface, 60, 0, 140, 50),
        "overflow:hidden should clip beyond the box"
    );
    // Inside should have content.
    assert!(
        has_non_white_pixels(&mut surface, 0, 0, 50, 30),
        "Inside the box should have visible text"
    );
}

/// 2. overflow:visible does NOT clip children.
#[test]
fn overflow_visible_does_not_clip() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 50.0, 30.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Visible;
    doc.node_mut(node).style.overflow_y = Overflow::Visible;
    // has_overflow_clip remains false

    let text_frag = make_text_child(&mut doc, node, "XXXXXXXXXXXXXXXXXXXX");
    frag.children.push(text_frag);

    let mut surface = make_surface(300, 50);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Text should extend beyond x=50 since no clipping.
    assert!(
        has_non_white_pixels(&mut surface, 55, 0, 100, 30),
        "overflow:visible should NOT clip — text should be visible beyond x=50"
    );
}

/// 3. overflow:scroll creates a clip.
#[test]
fn overflow_scroll_clips_children() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 50.0, 30.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Scroll;
    doc.node_mut(node).style.overflow_y = Overflow::Scroll;
    frag.has_overflow_clip = true;

    let text_frag = make_text_child(&mut doc, node, "XXXXXXXXXXXXXXXXXXXX");
    frag.children.push(text_frag);

    let mut surface = make_surface(200, 50);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        !has_non_white_pixels(&mut surface, 60, 0, 140, 50),
        "overflow:scroll should clip beyond the box"
    );
}

/// 4. overflow:auto creates a clip.
#[test]
fn overflow_auto_clips_children() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 50.0, 30.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Auto;
    doc.node_mut(node).style.overflow_y = Overflow::Auto;
    frag.has_overflow_clip = true;

    let text_frag = make_text_child(&mut doc, node, "XXXXXXXXXXXXXXXXXXXX");
    frag.children.push(text_frag);

    let mut surface = make_surface(200, 50);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        !has_non_white_pixels(&mut surface, 60, 0, 140, 50),
        "overflow:auto should clip beyond the box"
    );
}

/// 5. overflow:clip creates a clip (but is not scrollable).
#[test]
fn overflow_clip_keyword_clips_without_scrolling() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 50.0, 30.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Clip;
    doc.node_mut(node).style.overflow_y = Overflow::Clip;
    frag.has_overflow_clip = true;

    let text_frag = make_text_child(&mut doc, node, "XXXXXXXXXXXXXXXXXXXX");
    frag.children.push(text_frag);

    let mut surface = make_surface(200, 50);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    assert!(
        !has_non_white_pixels(&mut surface, 60, 0, 140, 50),
        "overflow:clip should clip beyond the box"
    );
    // Verify the Overflow::Clip value is not scrollable.
    assert!(!Overflow::Clip.is_scrollable());
    assert!(Overflow::Clip.is_clipping());
}

/// 6. Canvas save/restore balance — the canvas save count should be the
/// same before and after painting a fragment with overflow clipping.
#[test]
fn canvas_save_restore_balanced() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 50.0, 30.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(node).style.overflow_y = Overflow::Hidden;
    frag.has_overflow_clip = true;

    let text_frag = make_text_child(&mut doc, node, "ABC");
    frag.children.push(text_frag);

    let mut surface = make_surface(100, 50);
    let canvas = surface.canvas();
    let save_count_before = canvas.save_count();
    paint_fragment(canvas, &frag, &doc, PhysicalOffset::zero());
    let save_count_after = canvas.save_count();

    assert_eq!(
        save_count_before, save_count_after,
        "Canvas save/restore must be balanced: before={}, after={}",
        save_count_before, save_count_after
    );
}

/// 7. Border-radius with overflow:hidden uses rrect clip.
/// Verify that content at corners beyond the radius is clipped.
#[test]
fn border_radius_with_overflow_hidden_clips_corners() {
    let mut doc = Document::new();
    let vp = doc.root();

    // 100×100 box with a large border-radius (50px = circle) and overflow:hidden.
    let (node, mut frag) = make_box_fragment(&mut doc, vp, 100.0, 100.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(node).style.overflow_y = Overflow::Hidden;
    doc.node_mut(node).style.border_top_left_radius = (50.0, 50.0);
    doc.node_mut(node).style.border_top_right_radius = (50.0, 50.0);
    doc.node_mut(node).style.border_bottom_right_radius = (50.0, 50.0);
    doc.node_mut(node).style.border_bottom_left_radius = (50.0, 50.0);
    frag.has_overflow_clip = true;

    // Child box that fills the entire parent with a colored background.
    let child_node = doc.create_node(ElementTag::Div);
    doc.node_mut(child_node).style.display = Display::Block;
    doc.node_mut(child_node).style.width = Length::px(100.0);
    doc.node_mut(child_node).style.height = Length::px(100.0);
    doc.node_mut(child_node).style.background_color = Color::from_rgba_f32(1.0, 0.0, 0.0, 1.0);
    doc.append_child(node, child_node);

    let child_frag = Fragment::new_box(
        child_node,
        PhysicalSize::new(LayoutUnit::from_f32(100.0), LayoutUnit::from_f32(100.0)),
    );
    frag.children.push(child_frag);

    let mut surface = make_surface(120, 120);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // The corner at (0,0) should be white (clipped by border-radius).
    // Check a small 3×3 area at the very top-left corner.
    assert!(
        !has_non_white_pixels(&mut surface, 0, 0, 3, 3),
        "Top-left corner should be clipped by border-radius"
    );

    // The center of the box should be painted.
    assert!(
        has_non_white_pixels(&mut surface, 40, 40, 20, 20),
        "Center of the box should be painted"
    );
}

/// 8. Nested overflow containers — inner clip should further restrict painting.
#[test]
fn nested_overflow_containers() {
    let mut doc = Document::new();
    let vp = doc.root();

    // Outer box: 100×100 with overflow:hidden
    let (outer_node, mut outer_frag) = make_box_fragment(&mut doc, vp, 100.0, 100.0, 0.0);
    doc.node_mut(outer_node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(outer_node).style.overflow_y = Overflow::Hidden;
    outer_frag.has_overflow_clip = true;

    // Inner box: 60×60 at offset (20,20) with overflow:hidden
    let (inner_node, mut inner_frag) = make_box_fragment(&mut doc, outer_node, 60.0, 60.0, 0.0);
    doc.node_mut(inner_node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(inner_node).style.overflow_y = Overflow::Hidden;
    inner_frag.has_overflow_clip = true;
    inner_frag.offset = PhysicalOffset::new(
        LayoutUnit::from_f32(20.0),
        LayoutUnit::from_f32(20.0),
    );

    // Text that's wider than the inner box (60px)
    let text_frag = make_text_child(&mut doc, inner_node, "XXXXXXXXXXXXXXXXXXXX");
    inner_frag.children.push(text_frag);

    outer_frag.children.push(inner_frag);

    let mut surface = make_surface(200, 200);
    paint_fragment(surface.canvas(), &outer_frag, &doc, PhysicalOffset::zero());

    // Beyond inner box (x > 20+60 = 80) should be empty.
    assert!(
        !has_non_white_pixels(&mut surface, 90, 20, 110, 60),
        "Content beyond inner box should be clipped"
    );
    // Inside inner box should have content.
    assert!(
        has_non_white_pixels(&mut surface, 20, 20, 60, 30),
        "Inside inner box should have visible text"
    );
}

/// 9. Mixed overflow-x:hidden + overflow-y:visible — both trigger clip
/// because CSS spec says if one axis is not `visible`, the other computes
/// to a non-visible value too (the used value is adjusted).
/// The painter clips based on the style values set.
#[test]
fn overflow_x_hidden_overflow_y_visible_clips() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 50.0, 50.0, 0.0);
    // Per CSS spec, when one axis is not visible, the other computes to auto.
    // But in our model we test the painter's behavior: if overflow_x != visible,
    // the fragment gets has_overflow_clip and the painter clips in both axes.
    doc.node_mut(node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(node).style.overflow_y = Overflow::Visible;
    frag.has_overflow_clip = true; // layout would set this

    let text_frag = make_text_child(&mut doc, node, "XXXXXXXXXXXXXXXXXXXX");
    frag.children.push(text_frag);

    let mut surface = make_surface(200, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // The clip rect covers the entire padding box in both axes (since
    // clip_rect is a rectangle, it clips both axes).
    assert!(
        !has_non_white_pixels(&mut surface, 60, 0, 140, 100),
        "Clipping should constrain content beyond the box width"
    );
}

/// 10. Paint order: background → border → clip → children → restore.
/// Verify that borders are visible outside the clip area while children
/// inside are properly clipped.
#[test]
fn paint_order_background_border_clip_children_restore() {
    let mut doc = Document::new();
    let vp = doc.root();

    // 100×100 box with 10px red borders, overflow:hidden, blue background.
    let (node, mut frag) = make_box_fragment(&mut doc, vp, 100.0, 100.0, 10.0);
    doc.node_mut(node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(node).style.overflow_y = Overflow::Hidden;
    doc.node_mut(node).style.background_color = Color::from_rgba_f32(0.0, 0.0, 1.0, 1.0); // blue
    let red = Color::from_rgba_f32(1.0, 0.0, 0.0, 1.0);
    doc.node_mut(node).style.border_top_color = StyleColor::Resolved(red);
    doc.node_mut(node).style.border_right_color = StyleColor::Resolved(red);
    doc.node_mut(node).style.border_bottom_color = StyleColor::Resolved(red);
    doc.node_mut(node).style.border_left_color = StyleColor::Resolved(red);
    frag.has_overflow_clip = true;

    // Text child that extends beyond the box
    let text_frag = make_text_child(&mut doc, node, "XXXXXXXXXXXXXXXXXXXX");
    frag.children.push(text_frag);

    let mut surface = make_surface(200, 120);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // The border area (0..10, 0..100) should have non-white pixels (red border).
    assert!(
        has_non_white_pixels(&mut surface, 0, 0, 10, 100),
        "Left border should be visible"
    );
    // Right border (90..100 at top).
    assert!(
        has_non_white_pixels(&mut surface, 90, 0, 10, 10),
        "Right border area should be visible"
    );
    // Beyond the box (x>100) should be white (children clipped).
    assert!(
        !has_non_white_pixels(&mut surface, 110, 0, 90, 120),
        "Content beyond border-box should not be painted"
    );
}

/// 11. compute_clip_rect returns the padding box dimensions.
#[test]
fn compute_clip_rect_returns_padding_box() {
    let mut frag = Fragment::new_box(
        openui_dom::NodeId::NONE,
        PhysicalSize::new(LayoutUnit::from_f32(200.0), LayoutUnit::from_f32(100.0)),
    );
    frag.border = BoxStrut::new(
        LayoutUnit::from_f32(5.0),  // top
        LayoutUnit::from_f32(10.0), // right
        LayoutUnit::from_f32(5.0),  // bottom
        LayoutUnit::from_f32(10.0), // left
    );

    let offset = PhysicalOffset::new(LayoutUnit::from_f32(20.0), LayoutUnit::from_f32(30.0));
    let (x, y, w, h) = compute_clip_rect(&frag, offset);

    // padding box x = offset.left + border.left = 20 + 10 = 30
    assert!((x - 30.0).abs() < 0.01, "clip x: expected 30, got {}", x);
    // padding box y = offset.top + border.top = 30 + 5 = 35
    assert!((y - 35.0).abs() < 0.01, "clip y: expected 35, got {}", y);
    // padding box w = 200 - 10 - 10 = 180
    assert!((w - 180.0).abs() < 0.01, "clip w: expected 180, got {}", w);
    // padding box h = 100 - 5 - 5 = 90
    assert!((h - 90.0).abs() < 0.01, "clip h: expected 90, got {}", h);
}

/// 12. compute_clip_rect clamps to zero when borders exceed box size.
#[test]
fn compute_clip_rect_clamps_to_zero() {
    let mut frag = Fragment::new_box(
        openui_dom::NodeId::NONE,
        PhysicalSize::new(LayoutUnit::from_f32(20.0), LayoutUnit::from_f32(20.0)),
    );
    frag.border = BoxStrut::new(
        LayoutUnit::from_f32(15.0),
        LayoutUnit::from_f32(15.0),
        LayoutUnit::from_f32(15.0),
        LayoutUnit::from_f32(15.0),
    );

    let (_, _, w, h) = compute_clip_rect(&frag, PhysicalOffset::zero());
    assert!(w >= 0.0, "clip width must be non-negative, got {}", w);
    assert!(h >= 0.0, "clip height must be non-negative, got {}", h);
}

/// 13. Overflow enum properties are consistent.
#[test]
fn overflow_enum_properties() {
    assert!(!Overflow::Visible.is_clipping());
    assert!(Overflow::Hidden.is_clipping());
    assert!(Overflow::Scroll.is_clipping());
    assert!(Overflow::Auto.is_clipping());
    assert!(Overflow::Clip.is_clipping());

    assert!(!Overflow::Visible.is_scrollable());
    assert!(!Overflow::Hidden.is_scrollable());
    assert!(Overflow::Scroll.is_scrollable());
    assert!(Overflow::Auto.is_scrollable());
    assert!(!Overflow::Clip.is_scrollable());
}

/// 14. has_border_radius helper on ComputedStyle.
#[test]
fn has_border_radius_helper() {
    let mut style = ComputedStyle::initial();
    assert!(!style.has_border_radius(), "Initial style should have no border-radius");

    style.border_top_left_radius = (10.0, 10.0);
    assert!(style.has_border_radius(), "Should detect non-zero top-left radius");

    let mut style2 = ComputedStyle::initial();
    style2.border_bottom_right_radius = (5.0, 0.0);
    assert!(style2.has_border_radius(), "Should detect partial non-zero radius");
}

/// 15. Save/restore balance with border-radius + opacity layer.
#[test]
fn save_restore_balanced_with_border_radius_and_opacity() {
    let mut doc = Document::new();
    let vp = doc.root();

    let (node, mut frag) = make_box_fragment(&mut doc, vp, 80.0, 80.0, 0.0);
    doc.node_mut(node).style.overflow_x = Overflow::Hidden;
    doc.node_mut(node).style.overflow_y = Overflow::Hidden;
    doc.node_mut(node).style.border_top_left_radius = (10.0, 10.0);
    doc.node_mut(node).style.border_top_right_radius = (10.0, 10.0);
    doc.node_mut(node).style.border_bottom_right_radius = (10.0, 10.0);
    doc.node_mut(node).style.border_bottom_left_radius = (10.0, 10.0);
    doc.node_mut(node).style.opacity = 0.5; // also adds a layer
    frag.has_overflow_clip = true;

    let text_frag = make_text_child(&mut doc, node, "ABC");
    frag.children.push(text_frag);

    let mut surface = make_surface(100, 100);
    let canvas = surface.canvas();
    let count_before = canvas.save_count();
    paint_fragment(canvas, &frag, &doc, PhysicalOffset::zero());
    let count_after = canvas.save_count();

    assert_eq!(
        count_before, count_after,
        "Canvas save/restore must be balanced even with opacity + border-radius clip"
    );
}
