//! Public native literal input must retain the same structural transform source
//! and use the same actual-box resolution, ownership and unchanged-frame path.

use openui_compositor::{Frame, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    parse_literal, Color, Display, LengthValue, Position, StyleProperty, StyleValue, Transform2D,
    TransformList, TransformOperation,
};

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn fixture(scale: f64) -> (Engine, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(400.0, 300.0, scale).unwrap()).unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::BackgroundColor,
            Color::WHITE.into(),
        )
        .unwrap();
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), node).unwrap();
    for (property, value) in [
        (StyleProperty::Display, Display::Block.into()),
        (StyleProperty::Position, Position::Absolute.into()),
        (StyleProperty::Left, LengthValue::px(40.0).into()),
        (StyleProperty::Top, LengthValue::px(40.0).into()),
        (StyleProperty::Width, LengthValue::px(100.0).into()),
        (StyleProperty::Height, LengthValue::px(40.0).into()),
        (StyleProperty::BackgroundColor, Color::GREEN.into()),
    ] {
        engine.set_property(node, property, value).unwrap();
    }
    (engine, node)
}

fn literal(engine: &mut Engine, node: NodeHandle, input: &str) {
    // The string and parsed carrier die here; Engine owns the retained source.
    let input = input.to_owned();
    engine
        .set_property(
            node,
            StyleProperty::Transform,
            parse_literal(StyleProperty::Transform, &input).unwrap(),
        )
        .unwrap();
}
fn render(engine: &mut Engine, compositor: &mut SoftwareCompositor) -> Frame {
    compositor.render(&engine.scene().unwrap()).unwrap()
}

#[test]
fn native_literals_and_structural_operations_share_geometry_pixels_and_idle_work() {
    let cases = [
        (
            "translate(20px,7px)",
            TransformList(vec![TransformOperation::Translate(
                LengthValue::px(20.0),
                LengthValue::px(7.0),
            )]),
        ),
        (
            "scale(2,3)",
            TransformList(vec![TransformOperation::Scale(2.0, 3.0)]),
        ),
        (
            "rotate(.25turn)",
            TransformList(vec![TransformOperation::Rotate(90.0)]),
        ),
        (
            "matrix(1,0,0,1,20,7)",
            TransformList(vec![TransformOperation::Matrix(Transform2D {
                e: 20.0,
                f: 7.0,
                ..Transform2D::IDENTITY
            })]),
        ),
        (
            "translateX(20px) translateY(7px) scaleX(2) scaleY(125%)",
            TransformList(vec![
                TransformOperation::Translate(LengthValue::px(20.0), LengthValue::px(0.0)),
                TransformOperation::Translate(LengthValue::px(0.0), LengthValue::px(7.0)),
                TransformOperation::Scale(2.0, 1.0),
                TransformOperation::Scale(1.0, 1.25),
            ]),
        ),
        (
            "translate(20px,7px) scale(2,3)",
            TransformList(vec![
                TransformOperation::Translate(LengthValue::px(20.0), LengthValue::px(7.0)),
                TransformOperation::Scale(2.0, 3.0),
            ]),
        ),
    ];
    for scale in SCALES {
        for (input, operations) in &cases {
            let (mut parsed, node) = fixture(scale);
            let (mut typed, typed_node) = fixture(scale);
            literal(&mut parsed, node, input);
            typed
                .set_property(
                    typed_node,
                    StyleProperty::Transform,
                    StyleValue::Transform(operations.clone()),
                )
                .unwrap();
            let mut raster = SoftwareCompositor::default();
            let mut typed_raster = SoftwareCompositor::default();
            let frame = render(&mut parsed, &mut raster);
            let expected = render(&mut typed, &mut typed_raster);
            assert_eq!(
                parsed.bounds(node).unwrap(),
                typed.bounds(typed_node).unwrap(),
                "{input}@{scale}"
            );
            assert_eq!(
                parsed.computed_style(node).unwrap().native_transform,
                typed.computed_style(typed_node).unwrap().native_transform
            );
            assert_eq!(frame.pixels, expected.pixels, "{input}@{scale}");
            let before = parsed.stats();
            let generation = parsed.scene().unwrap().generation();
            literal(&mut parsed, node, input);
            assert_eq!(parsed.scene().unwrap().generation(), generation);
            assert_eq!(parsed.stats().layouts, before.layouts);
            assert_eq!(parsed.stats().paints, before.paints);
            assert_eq!(render(&mut parsed, &mut raster).pixels, frame.pixels);
        }
    }
}

#[test]
fn literal_lengths_resolve_after_actual_box_and_font_mutations() {
    for scale in SCALES {
        let (mut engine, node) = fixture(scale);
        literal(&mut engine, node, "translate(50%,25%)");
        let before = engine.bounds(node).unwrap().unwrap();
        assert_eq!(
            (before.x, before.y, before.width, before.height),
            (90.0, 50.0, 100.0, 40.0)
        );
        engine
            .set_property(node, StyleProperty::Width, LengthValue::px(200.0).into())
            .unwrap();
        engine
            .set_property(node, StyleProperty::Height, LengthValue::px(80.0).into())
            .unwrap();
        let after = engine.bounds(node).unwrap().unwrap();
        assert_eq!(
            (after.x, after.y, after.width, after.height),
            (140.0, 60.0, 200.0, 80.0)
        );
        literal(&mut engine, node, "translate(1em,2em)");
        for size in [20.0, 30.0] {
            engine
                .set_property(node, StyleProperty::FontSize, LengthValue::px(size).into())
                .unwrap();
            let bounds = engine.bounds(node).unwrap().unwrap();
            assert_eq!((bounds.x, bounds.y), (40.0 + size, 40.0 + size * 2.0));
        }
        literal(&mut engine, node, "none");
        let reset = engine.bounds(node).unwrap().unwrap();
        assert_eq!(
            (reset.x, reset.y, reset.width, reset.height),
            (40.0, 40.0, 200.0, 80.0)
        );
    }
}

#[test]
fn invalid_literals_leave_scene_unchanged_and_removed_sources_do_not_survive() {
    for scale in SCALES {
        let (mut engine, node) = fixture(scale);
        let mut raster = SoftwareCompositor::default();
        literal(&mut engine, node, "translate(20px,7px)");
        let frame = render(&mut engine, &mut raster);
        let bounds = engine.bounds(node).unwrap();
        let source = engine
            .computed_style(node)
            .unwrap()
            .native_transform
            .clone();
        let generation = engine.scene().unwrap().generation();
        let stats = engine.stats();
        for input in [
            "translate(20px,7px) junk",
            "scale(NaN)",
            "scale(1e100)",
            "translate(calc(50% + 1px))",
        ] {
            assert!(parse_literal(StyleProperty::Transform, input).is_err());
            assert_eq!(engine.bounds(node).unwrap(), bounds);
            assert_eq!(
                engine.computed_style(node).unwrap().native_transform,
                source
            );
            assert_eq!(engine.scene().unwrap().generation(), generation);
            assert_eq!(engine.stats().layouts, stats.layouts);
            assert_eq!(engine.stats().paints, stats.paints);
            assert_eq!(render(&mut engine, &mut raster).pixels, frame.pixels);
        }
        engine.remove(node).unwrap();
        assert!(engine.bounds(node).is_err());
        let removed = render(&mut engine, &mut raster);
        assert!(removed
            .pixels
            .chunks_exact(4)
            .all(|pixel| pixel == [255, 255, 255, 255]));
        let retained = engine.scene().unwrap();
        drop(engine);
        assert_eq!(raster.render(&retained).unwrap().pixels, removed.pixels);
    }
}
