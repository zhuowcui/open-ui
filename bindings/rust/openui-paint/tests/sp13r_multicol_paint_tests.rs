//! SP13-R paint regressions for fragmented stacking and decoration.

use openui_dom::{Document, ElementTag};
use openui_geometry::Length;
use openui_paint::render_to_surface;
use openui_style::{
    BorderStyle, Color, ColumnFill, ColumnSpan, Display, Overflow, OverflowClipBox, Position,
    StyleColor,
};
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
    doc.update_resolved_style(doc.root(), |style| style.display = Display::Block);
    doc.update_resolved_style(doc.root(), |style| style.background_color = Color::WHITE);

    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| style.display = Display::Block);
    doc.update_resolved_style(first, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(first, |style| style.height = Length::px(50.0));
    doc.update_resolved_style(first, |style| {
        style.background_color = Color::from_rgba8(255, 165, 0, 255)
    });
    doc.append_child(multicol, first);

    let translucent = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(translucent, |style| style.display = Display::Block);
    doc.update_resolved_style(translucent, |style| style.height = Length::px(100.0));
    doc.update_resolved_style(translucent, |style| {
        style.background_color = Color::from_rgba8(128, 0, 128, 255)
    });
    doc.update_resolved_style(translucent, |style| style.opacity = 0.5);
    doc.append_child(first, translucent);

    let later = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(later, |style| style.display = Display::Block);
    doc.update_resolved_style(later, |style| style.width = Length::px(50.0));
    doc.update_resolved_style(later, |style| style.height = Length::px(50.0));
    doc.update_resolved_style(later, |style| {
        style.background_color = Color::from_rgba8(255, 255, 0, 255)
    });
    doc.append_child(multicol, later);

    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(200 as f64, 150 as f64, 1.0).unwrap(),
    )
    .expect("multicol paint");
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

#[test]
fn fragmented_flex_item_outline_repeats_at_each_column_edge() {
    let mut doc = Document::new();
    doc.update_resolved_style(doc.root(), |style| style.display = Display::Block);
    doc.update_resolved_style(doc.root(), |style| style.background_color = Color::WHITE);

    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(40.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_top_width = 3);
        style.update_derived(|computed| computed.border_right_width = 3);
        style.update_derived(|computed| computed.border_bottom_width = 3);
        style.update_derived(|computed| computed.border_left_width = 3);
        let pink = StyleColor::Resolved(Color::from_rgba8(255, 192, 203, 255));
        style.update_derived(|computed| computed.border_top_color = pink.clone());
        style.update_derived(|computed| computed.border_right_color = pink.clone());
        style.update_derived(|computed| computed.border_bottom_color = pink.clone());
        style.update_derived(|computed| computed.border_left_color = pink);
    });
    doc.append_child(doc.root(), multicol);

    let flex = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(flex, |style| style.display = Display::Flex);
    doc.append_child(multicol, flex);

    let item = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(item, |style| {
        style.update_derived(|computed| computed.width = Length::px(30.0));
        style.update_derived(|computed| computed.height = Length::px(80.0));
        style.update_derived(|computed| computed.outline_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.outline_width = 2);
        style.update_derived(|computed| {
            computed.outline_color = StyleColor::Resolved(Color::from_rgba8(0, 0, 255, 255))
        });
    });
    doc.append_child(flex, item);

    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(120 as f64, 60 as f64, 1.0).unwrap(),
    )
    .expect("fragmented outline paint");
    for (x, y) in [(10, 1), (60, 1), (10, 43), (60, 43)] {
        assert_eq!(pixel(&mut surface, x, y), (0, 0, 255), "at ({x}, {y})");
    }
}

#[test]
fn fragmented_overflow_clip_uses_authored_reference_box_while_sharing_decorations() {
    let mut doc = Document::new();
    doc.update_resolved_style(doc.root(), |style| style.display = Display::Block);
    doc.update_resolved_style(doc.root(), |style| style.background_color = Color::WHITE);

    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(200.0));
        style.update_derived(|computed| computed.height = Length::px(50.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
    });
    doc.append_child(doc.root(), multicol);

    let clipped = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(clipped, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(50.0));
        style.update_derived(|computed| computed.padding_top = Length::px(5.0));
        style.update_derived(|computed| computed.padding_right = Length::px(5.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(5.0));
        style.update_derived(|computed| computed.padding_left = Length::px(5.0));
        style.update_derived(|computed| computed.border_top_width = 5);
        style.update_derived(|computed| computed.border_right_width = 5);
        style.update_derived(|computed| computed.border_bottom_width = 5);
        style.update_derived(|computed| computed.border_left_width = 5);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.overflow_x = Overflow::Clip);
        style.update_derived(|computed| computed.overflow_y = Overflow::Clip);
        style.update_derived(|computed| computed.overflow_clip_box = OverflowClipBox::BorderBox);
    });
    doc.append_child(multicol, clipped);

    for color in [Color::BLUE, Color::from_rgba8(0, 128, 0, 255)] {
        let content = doc.create_node(ElementTag::Div);
        doc.update_resolved_style(content, |style| {
            style.update_derived(|computed| computed.display = Display::Block);
            style.update_derived(|computed| computed.position = Position::Relative);
            style.update_derived(|computed| computed.top = Length::px(-20.0));
            style.update_derived(|computed| computed.left = Length::px(-20.0));
            style.update_derived(|computed| computed.width = Length::px(100.0));
            style.update_derived(|computed| computed.height = Length::px(50.0));
            style.update_derived(|computed| computed.background_color = color);
        });
        doc.append_child(clipped, content);
    }

    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(220.0, 80.0, 1.0).unwrap(),
    )
    .expect("fragmented overflow clip paint");

    assert_eq!(pixel(&mut surface, 2, 2), (0, 0, 255));
    assert_eq!(pixel(&mut surface, 110, 2), (0, 0, 255));

    doc.update_resolved_style(clipped, |style| {
        style.update_derived(|computed| computed.overflow_clip_box = OverflowClipBox::PaddingBox)
    });
    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(220.0, 80.0, 1.0).unwrap(),
    )
    .expect("fragmented padding-box overflow clip paint");
    assert_eq!(pixel(&mut surface, 110, 2), (0, 0, 0));
    assert_eq!(pixel(&mut surface, 114, 2), (0, 0, 255));

    doc.update_resolved_style(clipped, |style| {
        style.update_derived(|computed| computed.overflow_clip_box = OverflowClipBox::ContentBox)
    });
    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(220.0, 80.0, 1.0).unwrap(),
    )
    .expect("fragmented content-box overflow clip paint");
    assert_eq!(pixel(&mut surface, 114, 2), (255, 255, 255));
    assert_eq!(pixel(&mut surface, 119, 2), (0, 0, 255));

    doc.update_resolved_style(clipped, |style| {
        style.update_derived(|computed| computed.device_scale_factor = 1.25)
    });
    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(220.0, 80.0, 1.25).unwrap(),
    )
    .expect("fractional fragmented content-box overflow clip paint");
    assert_eq!(pixel(&mut surface, 147, 25), (127, 191, 127));
}

#[test]
fn first_sliced_continuation_keeps_descendant_border_to_fragmentainer_end() {
    let mut doc = Document::new();
    doc.update_resolved_style(doc.root(), |style| style.display = Display::Block);
    doc.update_resolved_style(doc.root(), |style| style.background_color = Color::WHITE);

    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(300.0));
        style.update_derived(|computed| computed.column_count = Some(3));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Balance);
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(128, 128, 128, 255)
        });
    });
    doc.append_child(doc.root(), multicol);

    let sliced = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(sliced, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(172.0));
        style.update_derived(|computed| computed.padding_top = Length::px(5.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(3.0));
        style.update_derived(|computed| computed.border_top_width = 10);
        style.update_derived(|computed| computed.border_bottom_width = 6);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(255, 255, 0, 255)
        });
    });
    doc.append_child(multicol, sliced);

    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(150.0));
        style.update_derived(|computed| computed.border_top_width = 3);
        style.update_derived(|computed| computed.border_right_width = 3);
        style.update_derived(|computed| computed.border_bottom_width = 3);
        style.update_derived(|computed| computed.border_left_width = 3);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
    });
    doc.append_child(sliced, child);

    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(320.0, 100.0, 1.0).unwrap(),
    )
    .expect("sliced descendant border paint");

    assert_eq!(pixel(&mut surface, 1, 62), (0, 0, 0));
    assert_eq!(pixel(&mut surface, 4, 62), (255, 255, 0));
}

#[test]
fn middle_sliced_continuation_preserves_fractional_fragmentainer_edge_coverage() {
    let mut doc = Document::new();
    doc.update_resolved_style(doc.root(), |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.padding_top = Length::px(20.0));
        style.update_derived(|computed| computed.padding_right = Length::px(20.0));
        style.update_derived(|computed| computed.padding_bottom = Length::px(20.0));
        style.update_derived(|computed| computed.padding_left = Length::px(20.0));
        style.update_derived(|computed| computed.background_color = Color::WHITE);
    });

    let outer = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(outer, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(400.0));
        style.update_derived(|computed| computed.height = Length::px(110.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_rule_width = 6);
        style.update_derived(|computed| computed.column_rule_style = BorderStyle::Solid);
        style.update_derived(|computed| {
            computed.column_rule_color = StyleColor::Resolved(Color::BLACK)
        });
    });
    doc.append_child(doc.root(), outer);

    let inner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(inner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(270.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_rule_width = 3);
        style.update_derived(|computed| computed.column_rule_style = BorderStyle::Solid);
        style.update_derived(|computed| {
            computed.column_rule_color = StyleColor::Resolved(Color::from_rgba8(0, 128, 0, 255))
        });
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(144, 238, 144, 255)
        });
        style.update_derived(|computed| computed.border_top_width = 10);
        style.update_derived(|computed| computed.border_right_width = 10);
        style.update_derived(|computed| computed.border_bottom_width = 10);
        style.update_derived(|computed| computed.border_left_width = 10);
        style.update_derived(|computed| computed.border_top_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_right_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_bottom_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.border_left_style = BorderStyle::Solid);
        let purple = StyleColor::Resolved(Color::from_rgba8(128, 0, 128, 255));
        style.update_derived(|computed| computed.border_top_color = purple.clone());
        style.update_derived(|computed| computed.border_right_color = purple.clone());
        style.update_derived(|computed| computed.border_bottom_color = purple.clone());
        style.update_derived(|computed| computed.border_left_color = purple);
    });
    doc.append_child(outer, inner);

    let first = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(first, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(200.0));
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(255, 255, 0, 255)
        });
    });
    doc.append_child(inner, first);

    let spanner = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(spanner, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.column_span = ColumnSpan::All);
        style.update_derived(|computed| computed.height = Length::px(50.0));
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(173, 216, 230, 255)
        });
    });
    doc.append_child(inner, spanner);

    let trailing = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(trailing, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.height = Length::px(240.0));
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(255, 255, 0, 255)
        });
    });
    doc.append_child(inner, trailing);

    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(800.0, 600.0, 1.25).unwrap(),
    )
    .expect("fractional nested multicol paint");

    let edge_pixels = [
        pixel(&mut surface, 37, 161),
        pixel(&mut surface, 37, 162),
        pixel(&mut surface, 297, 161),
        pixel(&mut surface, 297, 162),
    ];
    assert_eq!(
        edge_pixels,
        [
            (192, 128, 64),
            (168, 104, 114),
            (192, 128, 64),
            (160, 64, 96),
        ]
    );
}

#[test]
fn clipped_ancestor_outline_follows_descendant_fractional_mask() {
    let mut doc = Document::new();
    doc.update_resolved_style(doc.root(), |style| style.display = Display::Block);
    doc.update_resolved_style(doc.root(), |style| style.background_color = Color::WHITE);

    let parent = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(parent, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(20.0));
        style.update_derived(|computed| computed.height = Length::px(20.0));
        style.update_derived(|computed| computed.overflow_x = Overflow::Hidden);
        style.update_derived(|computed| computed.overflow_y = Overflow::Hidden);
        style.update_derived(|computed| computed.margin_top = Length::px(10.0));
        style.update_derived(|computed| computed.margin_left = Length::px(10.0));
        style.update_derived(|computed| computed.outline_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.outline_width = 1);
        style.update_derived(|computed| computed.device_scale_factor = 1.25);
        style
            .update_derived(|computed| computed.outline_color = StyleColor::Resolved(Color::BLACK));
    });
    doc.append_child(doc.root(), parent);

    let child = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(child, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(20.0));
        style.update_derived(|computed| computed.height = Length::px(20.0));
        style.update_derived(|computed| computed.outline_style = BorderStyle::Solid);
        style.update_derived(|computed| computed.outline_width = 1);
        style.update_derived(|computed| computed.device_scale_factor = 1.25);
        style.update_derived(|computed| computed.outline_color = StyleColor::Resolved(Color::BLUE));
    });
    doc.append_child(parent, child);

    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(80.0, 80.0, 1.25).unwrap(),
    )
    .expect("coincident outline paint");
    let mut colors = std::collections::BTreeSet::new();
    for y in 0..surface.height() {
        for x in 0..surface.width() {
            colors.insert(pixel(&mut surface, x, y));
        }
    }
    assert!(colors.contains(&(51, 51, 63)), "colors: {colors:?}");
}

#[test]
fn fragmented_positioned_grid_descendants_paint_by_logical_box() {
    let mut doc = Document::new();
    doc.update_resolved_style(doc.root(), |style| style.display = Display::Block);
    doc.update_resolved_style(doc.root(), |style| style.background_color = Color::WHITE);

    let multicol = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(multicol, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.width = Length::px(100.0));
        style.update_derived(|computed| computed.height = Length::px(100.0));
        style.update_derived(|computed| computed.column_count = Some(2));
        style.update_derived(|computed| computed.column_gap = Some(Length::px(0.0)));
        style.update_derived(|computed| computed.column_fill = ColumnFill::Auto);
    });
    doc.append_child(doc.root(), multicol);

    let containing_block = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(containing_block, |style| {
        style.update_derived(|computed| computed.display = Display::Grid);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(200.0));
    });
    doc.append_child(multicol, containing_block);

    let in_flow = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(in_flow, |style| {
        style.update_derived(|computed| computed.display = Display::Grid);
        style.update_derived(|computed| computed.position = Position::Relative);
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(200.0));
        style.update_derived(|computed| computed.background_color = Color::RED);
    });
    doc.append_child(containing_block, in_flow);

    let positioned = doc.create_node(ElementTag::Div);
    doc.update_resolved_style(positioned, |style| {
        style.update_derived(|computed| computed.display = Display::Block);
        style.update_derived(|computed| computed.position = Position::Absolute);
        style.update_derived(|computed| computed.left = Length::px(0.0));
        style.update_derived(|computed| computed.top = Length::px(0.0));
        style.update_derived(|computed| computed.width = Length::px(50.0));
        style.update_derived(|computed| computed.height = Length::px(200.0));
        style.update_derived(|computed| {
            computed.background_color = Color::from_rgba8(0, 128, 0, 255)
        });
    });
    doc.append_child(containing_block, positioned);

    let mut surface = render_to_surface(
        &doc,
        openui_geometry::ViewportMetrics::from_logical_size(120.0, 120.0, 1.25).unwrap(),
    )
    .expect("fractional multicol paint");
    assert_eq!(pixel(&mut surface, 62, 25), (63, 111, 15));
}
