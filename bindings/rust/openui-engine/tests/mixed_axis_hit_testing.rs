//! Native hit testing retains local overflow axes through transforms/mutations.

use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    Border, BorderStyle, Color, Containment, LengthValue, Overflow, Position, RendererStyleValue,
    StyleProperty, StyleValue, TransformList, TransformOperation,
};

fn fixture(scale: f64, transform: usize) -> (Engine, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap();
    let container = engine.create_native_element(ElementTag::Div).unwrap();
    let child = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), container).unwrap();
    engine.append_child(container, child).unwrap();
    engine
        .set_property(
            container,
            StyleProperty::Position,
            Position::Relative.into(),
        )
        .unwrap();
    engine
        .set_property(child, StyleProperty::Position, Position::Absolute.into())
        .unwrap();
    for property in [StyleProperty::Width, StyleProperty::Height] {
        engine
            .set_property(container, property, LengthValue::px(40.0).into())
            .unwrap();
        engine
            .set_property(child, property, LengthValue::px(80.0).into())
            .unwrap();
    }
    for property in [StyleProperty::Top, StyleProperty::Left] {
        engine
            .set_property(child, property, LengthValue::px(0.0).into())
            .unwrap();
    }
    engine
        .set_property(child, StyleProperty::BackgroundColor, Color::RED.into())
        .unwrap();
    let operations = match transform {
        0 => Vec::new(),
        1 => vec![
            TransformOperation::Translate(LengthValue::px(40.0), LengthValue::px(30.0)),
            TransformOperation::Scale(1.5, 1.25),
        ],
        2 => vec![
            TransformOperation::Translate(LengthValue::px(40.0), LengthValue::px(30.0)),
            TransformOperation::Rotate(90.0),
        ],
        _ => unreachable!(),
    };
    engine
        .set_property(
            container,
            StyleProperty::Transform,
            TransformList(operations).into(),
        )
        .unwrap();
    (engine, container, child)
}

fn point(engine: &mut Engine, child: NodeHandle, transform: usize, x: f32, y: f32) -> (f32, f32) {
    let bounds = engine.bounds(child).unwrap().unwrap();
    match transform {
        0 => (bounds.x + x, bounds.y + y),
        1 => (bounds.x + x * 1.5, bounds.y + y * 1.25),
        2 => (bounds.x + 80.0 - y, bounds.y + x),
        _ => unreachable!(),
    }
}

#[test]
fn visible_overflow_axis_remains_interactive_after_transforms_and_style_mutations() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for transform in 0..3 {
            let (mut engine, container, child) = fixture(scale, transform);
            for (x, y, right, bottom) in [
                (Overflow::Visible, Overflow::Clip, true, false),
                (Overflow::Clip, Overflow::Visible, false, true),
                (Overflow::Hidden, Overflow::Hidden, false, false),
                (Overflow::Visible, Overflow::Visible, true, true),
                // Reintroduce the original clip after removing it; cached hit
                // entries must not retain the previous frame's axis policy.
                (Overflow::Visible, Overflow::Clip, true, false),
            ] {
                engine
                    .set_property(container, StyleProperty::OverflowX, x.into())
                    .unwrap();
                engine
                    .set_property(container, StyleProperty::OverflowY, y.into())
                    .unwrap();
                for (local_x, local_y, expected) in [(55.0, 15.0, right), (15.0, 55.0, bottom)] {
                    let (px, py) = point(&mut engine, child, transform, local_x, local_y);
                    assert_eq!(
                        engine.hit_test(px, py).unwrap() == Some(child),
                        expected,
                        "overflow={x:?}/{y:?}, transform={transform}, scale={scale}"
                    );
                    // Repeating a hit must preserve the unchanged scene.
                    let generation = engine.scene().unwrap().generation();
                    assert_eq!(engine.hit_test(px, py).unwrap() == Some(child), expected);
                    assert_eq!(engine.scene().unwrap().generation(), generation);
                }
            }
        }
    }
}

#[test]
fn paint_containment_clips_visible_overflow_and_mutations_restore_interactivity() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for transform in 0..3 {
            let (mut engine, container, child) = fixture(scale, transform);
            engine
                .set_property(container, StyleProperty::Overflow, Overflow::Visible.into())
                .unwrap();
            for (containment, overflowing_hit) in [
                (Containment::PAINT, false),
                (Containment::NONE, true),
                (Containment::PAINT, false),
            ] {
                engine
                    .set_property(
                        container,
                        StyleProperty::Contain,
                        StyleValue::Renderer(RendererStyleValue::Contain(containment)),
                    )
                    .unwrap();
                for (local_x, local_y, expected) in [
                    (15.0, 15.0, true),
                    (55.0, 15.0, overflowing_hit),
                    (15.0, 55.0, overflowing_hit),
                ] {
                    let (x, y) = point(&mut engine, child, transform, local_x, local_y);
                    assert_eq!(
                        engine.hit_test(x, y).unwrap() == Some(child),
                        expected,
                        "containment={containment:?}, transform={transform}, scale={scale}"
                    );
                }
            }
        }
    }
}

#[test]
fn owned_client_clip_excludes_border_ink_and_preserves_visible_overflow() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let (mut engine, container, child) = fixture(scale, 0);
        engine
            .set_property(
                container,
                StyleProperty::Border,
                Border {
                    width: 2.0,
                    style: BorderStyle::Solid,
                    color: Color::BLACK,
                }
                .into(),
            )
            .unwrap();
        for property in [StyleProperty::Left, StyleProperty::Top] {
            engine
                .set_property(child, property, LengthValue::px(-4.0).into())
                .unwrap();
        }
        for (overflow, border_hit) in [
            (Overflow::Clip, false),
            (Overflow::Visible, true),
            (Overflow::Clip, false),
        ] {
            engine
                .set_property(container, StyleProperty::Overflow, overflow.into())
                .unwrap();
            for (local, expected) in [(3.0, border_hit), (7.0, true)] {
                let (x, y) = point(&mut engine, child, 0, local, local);
                assert_eq!(
                    engine.hit_test(x, y).unwrap() == Some(child),
                    expected,
                    "overflow={overflow:?}, scale={scale}"
                );
            }
        }
    }
}
