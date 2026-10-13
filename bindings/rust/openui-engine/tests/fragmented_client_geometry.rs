//! Measured Chromium147 source client quads remain distinct from physical ink.
//! Oracle: native-promoted-fragment-chromium-v4408, unchanged fixture/PNG bytes.
//! The nested in-flow principal-part discrepancy remains a required layout gate.

use openui_compositor::{Frame, SceneRect, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{
    Color, ColumnFill, Display, LengthValue, Position, RendererStyleValue, StyleProperty,
    StyleValue, TransformList, TransformOperation,
};

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn rect(x: f32, y: f32, width: f32, height: f32) -> SceneRect {
    SceneRect {
        x,
        y,
        width,
        height,
    }
}

fn engine(scale: f64) -> Engine {
    let mut e =
        Engine::new(ViewportMetrics::from_logical_size(400.0, 240.0, scale).unwrap()).unwrap();
    e.set_property(
        e.root(),
        StyleProperty::BackgroundColor,
        Color::WHITE.into(),
    )
    .unwrap();
    e
}

fn box_(e: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
    let node = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(parent, node).unwrap();
    for (property, value) in [
        (StyleProperty::Display, Display::Block.into()),
        (StyleProperty::Width, LengthValue::px(width).into()),
        (StyleProperty::Height, LengthValue::px(height).into()),
    ] {
        e.set_property(node, property, value).unwrap();
    }
    node
}

fn columns(e: &mut Engine, height: f32) -> NodeHandle {
    let root = e.root();
    let node = box_(e, root, 200.0, height);
    for (property, value) in [
        (StyleProperty::ColumnCount, StyleValue::Integer(2)),
        (StyleProperty::ColumnGap, LengthValue::px(0.0).into()),
        (StyleProperty::ColumnFill, ColumnFill::Auto.into()),
    ] {
        e.set_property(node, property, value).unwrap();
    }
    node
}

fn scale(e: &mut Engine, node: NodeHandle, factor: f32) {
    e.set_property(
        node,
        StyleProperty::TransformOrigin,
        StyleValue::Renderer(RendererStyleValue::TransformOrigin((
            Length::percent(50.0),
            Length::percent(50.0),
        ))),
    )
    .unwrap();
    e.set_property(
        node,
        StyleProperty::Transform,
        TransformList(vec![TransformOperation::Scale(factor, factor)]).into(),
    )
    .unwrap();
}

fn top(e: &mut Engine, node: NodeHandle, value: f32) {
    e.set_property(node, StyleProperty::Position, Position::Relative.into())
        .unwrap();
    e.set_property(node, StyleProperty::Top, LengthValue::px(value).into())
        .unwrap();
}

fn fixed(e: &mut Engine, parent: NodeHandle, height: f32) -> NodeHandle {
    let node = box_(e, parent, 8.0, height);
    for (property, value) in [
        (StyleProperty::Position, Position::Fixed.into()),
        (StyleProperty::Left, LengthValue::px(0.0).into()),
        (StyleProperty::Top, LengthValue::px(0.0).into()),
        (StyleProperty::BackgroundColor, Color::GREEN.into()),
    ] {
        e.set_property(node, property, value).unwrap();
    }
    node
}

fn simple(
    scale_factor: f64,
    height: f32,
    visual_top: Option<f32>,
) -> (Engine, NodeHandle, NodeHandle) {
    let mut e = engine(scale_factor);
    let columns = columns(&mut e, 40.0);
    let outer = box_(&mut e, columns, 100.0, height);
    scale(&mut e, outer, 0.5);
    if let Some(value) = visual_top {
        top(&mut e, outer, value);
    }
    let child = fixed(&mut e, outer, 8.0);
    (e, outer, child)
}

fn nested(scale_factor: f64) -> (Engine, NodeHandle, NodeHandle, NodeHandle) {
    let mut e = engine(scale_factor);
    let columns = columns(&mut e, 50.0);
    let outer = box_(&mut e, columns, 100.0, 68.0);
    e.set_property(
        outer,
        StyleProperty::PaddingTop,
        LengthValue::px(12.0).into(),
    )
    .unwrap();
    top(&mut e, outer, 7.0);
    scale(&mut e, outer, 0.5);
    let inner = box_(&mut e, outer, 80.0, 50.0);
    top(&mut e, inner, 11.0);
    scale(&mut e, inner, 0.75);
    let child = fixed(&mut e, inner, 120.0);
    (e, outer, inner, child)
}

fn pixel(frame: &Frame, scale: f64, x: f64, y: f64) -> [u8; 4] {
    let x = (x * scale).floor() as usize;
    let y = (y * scale).floor() as usize;
    let offset = y * frame.stride + x * 4;
    frame.pixels[offset..offset + 4].try_into().unwrap()
}

#[test]
fn source_client_rectangles_and_physical_ink_keep_their_measured_distinct_coordinates() {
    for scale in SCALES {
        for height in [20.0, 100.0] {
            for visual_top in [None, Some(50.0)] {
                let (mut e, outer, child) = simple(scale, height, visual_top);
                let movement = visual_top.unwrap_or(0.0);
                let source_y = height * 0.25 + movement;
                let expected = if height == 20.0 {
                    vec![rect(25.0, source_y, 50.0, 10.0)]
                } else {
                    vec![
                        rect(25.0, source_y, 50.0, 20.0),
                        rect(75.0, source_y, 50.0, 20.0),
                        rect(125.0, source_y, 50.0, 10.0),
                    ]
                };
                assert_eq!(
                    e.client_rects(outer).unwrap(),
                    expected,
                    "outer{height}/{visual_top:?}@{scale}"
                );
                assert_eq!(
                    e.client_rects(child).unwrap(),
                    vec![rect(25.0, source_y, 4.0, 4.0)]
                );
                let mut raster = SoftwareCompositor::default();
                let scene = e.scene().unwrap();
                let frame = raster.render(&scene).unwrap();
                let physical_y = if height == 20.0 { 5.0 } else { 10.0 } + movement;
                // These interior pixels come from retained immutable Chromium
                // PNGs. They do not alter the raster reference or admit tolerance.
                assert_eq!(
                    pixel(&frame, scale, 27.0, physical_y as f64 + 2.0),
                    [0, 128, 0, 255]
                );
                if height == 100.0 {
                    assert_eq!(
                        pixel(&frame, scale, 27.0, source_y as f64 + 2.0),
                        [255, 255, 255, 255]
                    );
                    // Chromium's recorded client-center query hits the outer,
                    // while the fixed child's painted ink is at another origin.
                    assert_eq!(e.hit_test(27.0, source_y + 2.0).unwrap(), Some(outer));
                } else {
                    assert_eq!(e.hit_test(27.0, source_y + 2.0).unwrap(), Some(child));
                }
                let stats = e.stats();
                let generation = scene.generation();
                assert_eq!(e.client_rects(outer).unwrap(), expected);
                assert_eq!(e.scene().unwrap().generation(), generation);
                assert_eq!(e.stats(), stats);
                assert_eq!(
                    raster.render(&e.scene().unwrap()).unwrap().pixels,
                    frame.pixels
                );
            }
        }
    }
}

#[test]
fn nested_source_transforms_map_real_positioned_continuations_before_column_advance() {
    for scale in SCALES {
        let (mut e, outer, _inner, child) = nested(scale);
        assert_eq!(
            e.client_rects(outer).unwrap(),
            vec![rect(25.0, 27.0, 50.0, 25.0), rect(75.0, 27.0, 50.0, 15.0)]
        );
        let expected = vec![
            rect(30.0, 41.625, 3.0, 14.25),
            rect(67.5, 37.125, 3.0, 18.75),
            rect(105.0, 37.125, 3.0, 12.0),
        ];
        assert_eq!(e.client_rects(child).unwrap(), expected);
        let stats = e.stats();
        let scene = e.scene().unwrap();
        assert_eq!(e.client_rects(child).unwrap(), expected);
        assert_eq!(e.scene().unwrap().generation(), scene.generation());
        assert_eq!(e.stats(), stats);
    }
}

#[test]
fn measured_nested_in_flow_principal_parts_remain_a_required_layout_gate() {
    for scale in SCALES {
        let (mut e, _outer, inner, _child) = nested(scale);
        // Chromium's first inner part is 38px; the preceding native diagnosis
        // retains 27px. Source projection must not resize the box to hide that
        // independent layout defect or substitute the child's correct 38px part.
        assert_eq!(
            e.client_rects(inner).unwrap(),
            vec![
                rect(30.0, 41.625, 30.0, 14.25),
                rect(67.5, 37.125, 30.0, 4.5)
            ]
        );
    }
}

#[test]
fn source_reference_mutation_removal_and_fitting_neighbor_preserve_native_ownership() {
    for scale in SCALES {
        let (mut e, outer, child) = simple(scale, 100.0, None);
        assert_eq!(
            e.client_rects(child).unwrap(),
            vec![rect(25.0, 25.0, 4.0, 4.0)]
        );
        e.set_property(outer, StyleProperty::Height, LengthValue::px(20.0).into())
            .unwrap();
        assert_eq!(
            e.client_rects(child).unwrap(),
            vec![rect(25.0, 5.0, 4.0, 4.0)]
        );
        e.set_property(
            outer,
            StyleProperty::Transform,
            TransformList::default().into(),
        )
        .unwrap();
        assert_eq!(
            e.client_rects(child).unwrap(),
            vec![rect(0.0, 0.0, 8.0, 8.0)]
        );
        let stats = e.stats();
        let generation = e.scene().unwrap().generation();
        assert_eq!(e.bounds(child).unwrap(), Some(rect(0.0, 0.0, 8.0, 8.0)));
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
        e.remove(child).unwrap();
        assert!(e.client_rects(child).is_err());
    }
}
