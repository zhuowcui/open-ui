//! SP13-R paint regressions for fragmented stacking and decoration.

use openui_dom::{Document, ElementTag};
use openui_geometry::Length;
use openui_paint::render_to_surface;
use openui_style::{Color, ColumnFill, Display};
use skia_safe::{image::CachingHint, AlphaType, ColorType, ImageInfo, Surface};

fn pixel(surface: &mut Surface, x: i32, y: i32) -> (u8, u8, u8) {
    let image = surface.image_snapshot();
    let row_bytes = (image.width() * 4) as usize;
    let mut pixels = vec![0; row_bytes];
    let info = ImageInfo::new(
        (image.width(), 1),
        ColorType::RGBA8888,
        AlphaType::Premul,
        None,
    );
    image.read_pixels(&info, &mut pixels, row_bytes, (0, y), CachingHint::Allow);
    let offset = x as usize * 4;
    (pixels[offset], pixels[offset + 1], pixels[offset + 2])
}

#[test]
fn opacity_stacking_context_in_a_column_paints_after_later_in_flow_content() {
    let mut doc = Document::new();
    doc.node_mut(doc.root()).style.display = Display::Block;
    doc.node_mut(doc.root()).style.background_color = Color::WHITE;

    let multicol = doc.create_node(ElementTag::Div);
    {
        let style = &mut doc.node_mut(multicol).style;
        style.display = Display::Block;
        style.width = Length::px(100.0);
        style.height = Length::px(100.0);
        style.column_count = Some(2);
        style.column_gap = Some(Length::px(0.0));
        style.column_fill = ColumnFill::Auto;
    }
    doc.append_child(doc.root(), multicol);

    let first = doc.create_node(ElementTag::Div);
    doc.node_mut(first).style.display = Display::Block;
    doc.node_mut(first).style.width = Length::px(50.0);
    doc.node_mut(first).style.height = Length::px(50.0);
    doc.node_mut(first).style.background_color = Color::from_rgba8(255, 165, 0, 255);
    doc.append_child(multicol, first);

    let translucent = doc.create_node(ElementTag::Div);
    doc.node_mut(translucent).style.display = Display::Block;
    doc.node_mut(translucent).style.height = Length::px(100.0);
    doc.node_mut(translucent).style.background_color = Color::from_rgba8(128, 0, 128, 255);
    doc.node_mut(translucent).style.opacity = 0.5;
    doc.append_child(first, translucent);

    let later = doc.create_node(ElementTag::Div);
    doc.node_mut(later).style.display = Display::Block;
    doc.node_mut(later).style.width = Length::px(50.0);
    doc.node_mut(later).style.height = Length::px(50.0);
    doc.node_mut(later).style.background_color = Color::from_rgba8(255, 255, 0, 255);
    doc.append_child(multicol, later);

    let mut surface = render_to_surface(&doc, 200, 150).expect("multicol paint");
    let actual = pixel(&mut surface, 25, 75);
    for (channel, expected) in [actual.0, actual.1, actual.2]
        .into_iter()
        .zip([191_u8, 127, 64])
    {
        assert!(
            channel.abs_diff(expected) <= 2,
            "unexpected blend: {actual:?}"
        );
    }
}
