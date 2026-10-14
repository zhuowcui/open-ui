//! Public native Rust radius setters feed shared paint and input geometry.
use openui::prelude::*;
use openui_style::{Color, CornerRadii, Display, Edges, Overflow};
#[test]
fn calculated_native_radius_matches_fixed_source_and_retains_owned_pixels_after_mutation() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let document = Document::with_font_collection(
            ViewportMetrics::from_logical_size(160.0, 140.0, scale).unwrap(),
            FontCollection::deterministic_test(),
        )
        .unwrap();
        document.body().set_background_color(Color::WHITE).unwrap();
        let port = Element::create(&document, "div").unwrap();
        port.set_property(StyleProperty::Display, Display::Block.into())
            .unwrap();
        port.set_width(LengthValue::px(100.0)).unwrap();
        port.set_height(LengthValue::px(100.0)).unwrap();
        port.set_property(StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        port.set_border_radius(CornerRadii(Edges::all(LengthValue::calc_percent_px(
            29.0, 1.0,
        ))))
        .unwrap();
        document.body().append_child(&port).unwrap();
        let child = Element::create(&document, "div").unwrap();
        child
            .set_property(StyleProperty::Display, Display::Block.into())
            .unwrap();
        child.set_width(LengthValue::px(100.0)).unwrap();
        child.set_height(LengthValue::px(100.0)).unwrap();
        child.set_background_color(Color::GREEN).unwrap();
        port.append_child(&child).unwrap();
        let first = document.render_to_bitmap().unwrap();
        let hit = document.hit_test(3.0, 3.0).unwrap();
        assert!(hit
            .as_ref()
            .is_none_or(|element| !element.is_same_node(&child).unwrap()));
        assert!(document
            .hit_test(50.0, 50.0)
            .unwrap()
            .unwrap()
            .is_same_node(&child)
            .unwrap());
        let before = port.computed_style().unwrap();
        assert_eq!(
            before.border_radii_for_box((100.0, 100.0)),
            [(30.0, 30.0); 4]
        );
        port.set_border_radius(CornerRadii(Edges::all(LengthValue::px(30.0))))
            .unwrap();
        let fixed = document.render_to_bitmap().unwrap();
        assert_eq!(first.pixels(), fixed.pixels(), "scale={scale}");
        let repeated_hit = document.hit_test(3.0, 3.0).unwrap();
        match (&hit, &repeated_hit) {
            (Some(a), Some(b)) => assert!(a.is_same_node(b).unwrap()),
            (None, None) => {}
            _ => panic!("radius mutation changed corner target identity"),
        }
        assert_eq!(
            before.border_radius_lengths[0].unwrap().0,
            openui::typed_style::Length::calc_percent_px(29.0, 1.0)
        );
        assert_eq!(document.render_to_bitmap().unwrap(), fixed);
        port.set_border_radius(CornerRadii(Edges::all(LengthValue::px(0.0))))
            .unwrap();
        assert!(document
            .hit_test(3.0, 3.0)
            .unwrap()
            .unwrap()
            .is_same_node(&child)
            .unwrap());
        assert_ne!(
            first.pixels(),
            document.render_to_bitmap().unwrap().pixels()
        );
        child.remove().unwrap();
        port.remove().unwrap();
        drop(child);
        drop(port);
        drop(document);
        assert!(!first.pixels().is_empty());
    }
}
