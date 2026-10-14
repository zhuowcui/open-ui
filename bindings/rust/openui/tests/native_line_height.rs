use openui::prelude::*;
use std::{cell::Cell, rc::Rc};

fn document(scale: f64) -> Document {
    Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
        FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )
    .unwrap()
}

// These widths are independent pinned Chromium observations in the immutable
// reference-only-3642 archive. Line-height feeds a retained 2.5lh width; the
// native callback changes only the parent font, exercising both metric contexts.
#[test]
fn relative_line_height_updates_from_native_callbacks_and_inherits_fixed_values() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let document = document(scale);
        document
            .body()
            .set_font_size(LengthValue::px(16.0))
            .unwrap();
        let parent = Element::create(&document, "div").unwrap();
        document.body().append_child(&parent).unwrap();
        parent
            .set_font_family(FontFamilyList::single("Ahem"))
            .unwrap();
        parent.set_font_size(LengthValue::px(18.72)).unwrap();
        parent.set_line_height(LineHeight::Number(1.25)).unwrap();
        let mut nodes = Vec::new();
        for (unit, before, after) in [
            (LengthValue::Ch(2.0), 150.0, 150.0),
            (LengthValue::Ex(2.0), 121.625, 121.625),
            (LengthValue::Lh(2.0), 116.953125, 127.1875),
            (LengthValue::Em(1.5), 114.03125, 114.03125),
            (LengthValue::Rem(1.5), 60.0, 60.0),
            (LengthValue::percent(127.5), 96.546875, 96.546875),
        ] {
            let node = Element::create(&document, "div").unwrap();
            parent.append_child(&node).unwrap();
            node.set_width(LengthValue::Lh(2.5)).unwrap();
            node.set_line_height_length(unit).unwrap();
            node.set_font_size(LengthValue::px(30.41)).unwrap();
            let descendant = Element::create(&document, "div").unwrap();
            node.append_child(&descendant).unwrap();
            descendant.set_font_size(LengthValue::px(11.0)).unwrap();
            descendant.set_width(LengthValue::Lh(2.5)).unwrap();
            assert_eq!(
                node.bounding_rect().unwrap().unwrap().width,
                before,
                "{unit:?}/{scale}"
            );
            assert_eq!(descendant.bounding_rect().unwrap().unwrap().width, before);
            assert_eq!(
                node.computed_style().unwrap().line_height,
                descendant.computed_style().unwrap().line_height
            );
            nodes.push((node, descendant, before, after));
        }
        let owned = nodes[2].0.computed_style().unwrap();
        let weak_parent = parent.downgrade();
        let called = Rc::new(Cell::new(0));
        let count = called.clone();
        nodes[0]
            .0
            .on("click", move |_| {
                weak_parent
                    .upgrade()
                    .unwrap()
                    .set_font_size(LengthValue::px(20.37))
                    .unwrap();
                count.set(count.get() + 1);
            })
            .unwrap();
        nodes[0].0.click().unwrap();
        assert_eq!(called.get(), 1);
        for (node, child, _, after) in &nodes {
            assert_eq!(node.bounding_rect().unwrap().unwrap().width, *after);
            assert_eq!(child.bounding_rect().unwrap().unwrap().width, *after);
        }
        assert_ne!(
            owned.line_height,
            nodes[2].0.computed_style().unwrap().line_height
        );
        assert_eq!(
            document.render_to_png_buffer().unwrap(),
            document.render_to_png_buffer().unwrap()
        );
        let weak = nodes[0].0.downgrade();
        drop(nodes);
        drop(parent);
        drop(document);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn relative_line_height_follows_shorthand_order_pseudos_and_animation_cancel() {
    let document = document(1.0);
    let node = Element::create(&document, "div").unwrap();
    document.body().append_child(&node).unwrap();
    node.set_font_family(FontFamilyList::single("Ahem"))
        .unwrap();
    node.set_font_size(LengthValue::px(20.0)).unwrap();
    node.set_width(LengthValue::Lh(2.0)).unwrap();
    node.set_line_height_length(LengthValue::Ch(2.0)).unwrap();
    assert_eq!(node.bounding_rect().unwrap().unwrap().width, 80.0);
    let mut shorthand = FontShorthand::new(24.0, FontFamilyList::single("Ahem")).unwrap();
    shorthand.line_height = LineHeight::Number(1.5);
    node.set_font(shorthand).unwrap();
    assert_eq!(node.bounding_rect().unwrap().unwrap().width, 72.0);
    node.set_line_height_length(LengthValue::Ex(2.5)).unwrap();
    assert_eq!(node.bounding_rect().unwrap().unwrap().width, 96.0);
    let pseudo = Style::default().line_height_length(LengthValue::Ch(1.5));
    node.set_pseudo_style(PseudoStyleTarget::FirstLine, &pseudo)
        .unwrap();
    assert_eq!(
        node.computed_style()
            .unwrap()
            .first_line_style
            .as_ref()
            .unwrap()
            .line_height,
        LineHeight::Length(36.0)
    );
    let animation = node
        .animate(
            StyleProperty::LineHeight,
            Keyframes::from_values(LengthValue::Ex(2.5), LengthValue::Ex(5.0)),
            AnimationOptions {
                duration_ms: 1000.0,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
        )
        .unwrap();
    document.seek_animation(animation, 500.0).unwrap();
    assert_eq!(node.bounding_rect().unwrap().unwrap().width, 144.0);
    node.set_font_size(LengthValue::px(32.0)).unwrap();
    assert_eq!(node.bounding_rect().unwrap().unwrap().width, 192.0);
    document.cancel_animation(animation).unwrap();
    assert_eq!(node.bounding_rect().unwrap().unwrap().width, 128.0);
    let copy = node.clone_subtree().unwrap();
    document.body().append_child(&copy).unwrap();
    assert_eq!(copy.bounding_rect().unwrap().unwrap().width, 128.0);
}

#[test]
fn invalid_relative_line_heights_leave_retained_style_intact() {
    let document = document(1.0);
    let node = document.body();
    node.set_line_height_length(LengthValue::Em(1.5)).unwrap();
    node.set_font_size(LengthValue::px(0.0)).unwrap();
    let before = node.computed_style().unwrap();
    for value in [
        LengthValue::percent(-0.5),
        LengthValue::ViewportWidth(-1.0),
        LengthValue::Ch(-1.0),
        LengthValue::Ex(f32::NAN),
        LengthValue::auto(),
        LengthValue::none(),
    ] {
        assert!(node.set_line_height_length(value).is_err());
        assert_eq!(
            node.computed_style().unwrap().line_height,
            before.line_height
        );
    }
}
