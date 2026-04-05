//! Tests for SP11 Round 23 code review fixes — openui-paint crate.
//!
//! Issue 3: Locale plumbed from ComputedStyle to FontDescription (paint crate).
//! Issue 4: 3D border styles (inset, outset, groove, ridge) have
//! side-dependent shading: top+left vs bottom+right.

use skia_safe::{surfaces, Color as SkColor, Surface};
use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, Length, PhysicalOffset, PhysicalSize};
use openui_layout::Fragment;
use openui_paint::paint_fragment;
use openui_style::*;

fn make_surface(width: i32, height: i32) -> Surface {
    let mut surface = surfaces::raster_n32_premul((width, height))
        .expect("Failed to create Skia surface");
    surface.canvas().clear(SkColor::WHITE);
    surface
}

/// Sample average color in a rectangular region (returns BGRA averages).
fn sample_avg_color(surface: &mut Surface, x: i32, y: i32, w: i32, h: i32) -> (f32, f32, f32) {
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
    let mut r_sum = 0.0f64;
    let mut g_sum = 0.0f64;
    let mut b_sum = 0.0f64;
    let mut count = 0u32;
    for py in y..(y + h).min(info.height()) {
        for px in x..(x + w).min(info.width() as i32) {
            let offset = (py as usize) * row_bytes + (px as usize) * bpp;
            if offset + 3 < pixels.len() {
                b_sum += pixels[offset] as f64;
                g_sum += pixels[offset + 1] as f64;
                r_sum += pixels[offset + 2] as f64;
                count += 1;
            }
        }
    }
    if count == 0 {
        return (255.0, 255.0, 255.0);
    }
    (
        (r_sum / count as f64) as f32,
        (g_sum / count as f64) as f32,
        (b_sum / count as f64) as f32,
    )
}

fn make_3d_bordered_box(doc: &mut Document, border_style: BorderStyle) -> Fragment {
    let vp = doc.root();
    let div = doc.create_node(ElementTag::Div);
    {
        let s = &mut doc.node_mut(div).style;
        s.display = Display::Block;
        s.width = Length::px(80.0);
        s.height = Length::px(80.0);
        // 8px border on all sides, gray color for easy shading tests.
        s.border_top_width = 8;
        s.border_right_width = 8;
        s.border_bottom_width = 8;
        s.border_left_width = 8;
        let gray = Color { r: 0.6, g: 0.6, b: 0.6, a: 1.0 };
        s.border_top_color = StyleColor::Resolved(gray);
        s.border_right_color = StyleColor::Resolved(gray);
        s.border_bottom_color = StyleColor::Resolved(gray);
        s.border_left_color = StyleColor::Resolved(gray);
        s.border_top_style = border_style;
        s.border_right_style = border_style;
        s.border_bottom_style = border_style;
        s.border_left_style = border_style;
    }
    doc.append_child(vp, div);

    let mut frag = Fragment::new_box(
        div,
        PhysicalSize::new(LayoutUnit::from_f32(80.0), LayoutUnit::from_f32(80.0)),
    );
    // Set border insets on the fragment.
    frag.border.top = LayoutUnit::from_i32(8);
    frag.border.right = LayoutUnit::from_i32(8);
    frag.border.bottom = LayoutUnit::from_i32(8);
    frag.border.left = LayoutUnit::from_i32(8);
    frag
}

// ── Issue 4: Inset border has side-dependent shading ────────────────────

#[test]
fn inset_border_top_is_darker_than_bottom() {
    // For inset: top+left are darkened, bottom+right are lightened.
    // With base gray (0.6, 0.6, 0.6):
    //   darkened = (0.3, 0.3, 0.3) → ~76 in 0-255
    //   lightened = (0.8, 0.8, 0.8) → ~204 in 0-255
    let mut doc = Document::new();
    let frag = make_3d_bordered_box(&mut doc, BorderStyle::Inset);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Sample top border region (y=0..8, x=10..70) — should be dark.
    let (top_r, top_g, _top_b) = sample_avg_color(&mut surface, 10, 2, 60, 4);
    // Sample bottom border region (y=72..80, x=10..70) — should be light.
    let (bot_r, bot_g, _bot_b) = sample_avg_color(&mut surface, 10, 74, 60, 4);

    assert!(
        top_r < bot_r,
        "Inset: top border R ({top_r}) should be darker than bottom R ({bot_r})"
    );
    assert!(
        top_g < bot_g,
        "Inset: top border G ({top_g}) should be darker than bottom G ({bot_g})"
    );
}

#[test]
fn outset_border_top_is_lighter_than_bottom() {
    // For outset: top+left are lightened, bottom+right are darkened.
    let mut doc = Document::new();
    let frag = make_3d_bordered_box(&mut doc, BorderStyle::Outset);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    let (top_r, top_g, _top_b) = sample_avg_color(&mut surface, 10, 2, 60, 4);
    let (bot_r, bot_g, _bot_b) = sample_avg_color(&mut surface, 10, 74, 60, 4);

    assert!(
        top_r > bot_r,
        "Outset: top border R ({top_r}) should be lighter than bottom R ({bot_r})"
    );
    assert!(
        top_g > bot_g,
        "Outset: top border G ({top_g}) should be lighter than bottom G ({bot_g})"
    );
}

#[test]
fn inset_left_is_darker_than_right() {
    // For inset: left is darkened, right is lightened.
    let mut doc = Document::new();
    let frag = make_3d_bordered_box(&mut doc, BorderStyle::Inset);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Sample left border region (x=0..8, y=10..70).
    let (left_r, _left_g, _) = sample_avg_color(&mut surface, 2, 10, 4, 60);
    // Sample right border region (x=72..80, y=10..70).
    let (right_r, _right_g, _) = sample_avg_color(&mut surface, 74, 10, 4, 60);

    assert!(
        left_r < right_r,
        "Inset: left border R ({left_r}) should be darker than right R ({right_r})"
    );
}

#[test]
fn groove_border_has_different_halves() {
    // Groove: outer half uses inset shading, inner half uses outset shading.
    // The top border should have dark outer half and light inner half.
    let mut doc = Document::new();
    let frag = make_3d_bordered_box(&mut doc, BorderStyle::Groove);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Outer half of top border (y=0..4) — should be dark (inset shading for top).
    let (outer_r, _, _) = sample_avg_color(&mut surface, 10, 1, 60, 3);
    // Inner half of top border (y=4..8) — should be light (outset shading for top).
    let (inner_r, _, _) = sample_avg_color(&mut surface, 10, 5, 60, 3);

    assert!(
        outer_r < inner_r,
        "Groove: outer half of top ({outer_r}) should be darker than inner half ({inner_r})"
    );
}

#[test]
fn ridge_border_is_opposite_of_groove() {
    // Ridge: outer half uses outset shading, inner half uses inset shading.
    // The top border should have light outer half and dark inner half.
    let mut doc = Document::new();
    let frag = make_3d_bordered_box(&mut doc, BorderStyle::Ridge);

    let mut surface = make_surface(100, 100);
    paint_fragment(surface.canvas(), &frag, &doc, PhysicalOffset::zero());

    // Outer half of top border (y=0..4) — should be light (outset shading for top).
    let (outer_r, _, _) = sample_avg_color(&mut surface, 10, 1, 60, 3);
    // Inner half of top border (y=4..8) — should be dark (inset shading for top).
    let (inner_r, _, _) = sample_avg_color(&mut surface, 10, 5, 60, 3);

    assert!(
        outer_r > inner_r,
        "Ridge: outer half of top ({outer_r}) should be lighter than inner half ({inner_r})"
    );
}

// ── Issue 3: Locale plumbed from ComputedStyle to FontDescription (paint) ─

#[test]
fn paint_style_to_font_description_plumbs_locale() {
    let mut style = ComputedStyle::default();
    style.locale = Some("zh-Hans".to_string());

    let desc = openui_paint::text_painter::style_to_font_description(&style);
    assert_eq!(
        desc.locale.as_deref(),
        Some("zh-Hans"),
        "Paint crate's FontDescription should carry locale from ComputedStyle"
    );
}

#[test]
fn paint_style_to_font_description_locale_none_default() {
    let style = ComputedStyle::default();
    let desc = openui_paint::text_painter::style_to_font_description(&style);
    assert_eq!(
        desc.locale, None,
        "Paint FontDescription should have locale=None when ComputedStyle has no locale"
    );
}
