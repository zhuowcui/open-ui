use openui::prelude::*;
use openui::typed_style::{
    Length, RendererStyleValue, Transform3D, TransformList, TransformOperation,
};

fn fixture(scale: f64, width: f32, transform: TransformList) -> (Document, Element) {
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(400.0, 200.0, scale).unwrap(),
        FontCollection::deterministic_test(),
    )
    .unwrap();
    document.body().set_background_color(Color::WHITE).unwrap();
    let node = Element::create(&document, "div").unwrap();
    node.set_width(LengthValue::px(width)).unwrap();
    node.set_height(LengthValue::px(40.0)).unwrap();
    node.set_background_color(Color::BLUE).unwrap();
    node.set_property(
        StyleProperty::TransformOrigin,
        StyleValue::Renderer(RendererStyleValue::TransformOrigin((
            Length::px(0.0),
            Length::px(0.0),
        ))),
    )
    .unwrap();
    node.set_property(StyleProperty::Transform, transform.into())
        .unwrap();
    document.body().append_child(&node).unwrap();
    (document, node)
}

fn translate(x: f32) -> TransformList {
    TransformList(vec![TransformOperation::Translate(
        LengthValue::px(x),
        LengthValue::px(0.0),
    )])
}

#[test]
fn deferred_suffix_resolves_same_frame_animated_width_and_later_paused_geometry() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, node) = fixture(scale, 100.0, TransformList::default());
        let options = AnimationOptions {
            duration_ms: 100.0,
            fill: FillMode::Both,
            easing: Easing::Linear,
            ..AnimationOptions::default()
        };
        let width_id = node
            .animate(
                StyleProperty::Width,
                Keyframes::from_values(LengthValue::px(100.0), LengthValue::px(200.0)),
                options.clone(),
            )
            .unwrap();
        let transform_id = node
            .animate(
                StyleProperty::Transform,
                Keyframes::from_values(
                    TransformList(vec![TransformOperation::Translate(
                        LengthValue::percent(100.0),
                        LengthValue::px(0.0),
                    )]),
                    TransformList(vec![TransformOperation::Scale(1.0, 1.0)]),
                ),
                options,
            )
            .unwrap();
        document.advance_time(50.0).unwrap();
        let (expected, _) = fixture(scale, 150.0, translate(75.0));
        let first = document.render_to_png_buffer().unwrap();
        assert_eq!(first, expected.render_to_png_buffer().unwrap());
        document.pause_animation(transform_id).unwrap();
        document.seek_animation(width_id, 100.0).unwrap();
        let (expected, _) = fixture(scale, 200.0, translate(100.0));
        let second = document.render_to_png_buffer().unwrap();
        assert_eq!(second, expected.render_to_png_buffer().unwrap());
        assert_ne!(first, second);
        assert_eq!(document.render_to_png_buffer().unwrap(), second);
        document.cancel_animation(transform_id).unwrap();
        document.cancel_animation(width_id).unwrap();
    }
}

#[test]
fn static_perspective_and_axis_rotation_reach_same_full_matrix_raster_route() {
    let h = std::f64::consts::SQRT_2 / 2.0;
    let matrix = Transform3D([
        h as f32,
        0.0,
        -h as f32,
        (h / 500.0) as f32,
        0.0,
        1.0,
        0.0,
        0.0,
        h as f32,
        0.0,
        h as f32,
        (-h / 500.0) as f32,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, node) = fixture(
            scale,
            100.0,
            TransformList(vec![
                TransformOperation::Perspective(LengthValue::px(500.0)),
                TransformOperation::Rotate3d {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    degrees: 45.0,
                },
            ]),
        );
        let (expected, _) = fixture(
            scale,
            100.0,
            TransformList(vec![TransformOperation::Matrix3d(matrix)]),
        );
        let pixels = document.render_to_png_buffer().unwrap();
        assert_eq!(pixels, expected.render_to_png_buffer().unwrap());
        let (plain, _) = fixture(scale, 100.0, TransformList::default());
        assert_ne!(pixels, plain.render_to_png_buffer().unwrap());
        // The native getter retains the authored operations for effect stacks;
        // a lossy affine accessor would turn this into an unrelated matrix.
        assert_eq!(
            node.computed_style()
                .unwrap()
                .native_transform
                .as_ref()
                .unwrap()
                .0
                .len(),
            2
        );
    }
}

#[test]
fn depth_singular_transform_hides_subtree_and_recovers_on_mutation() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, node) = fixture(
            scale,
            100.0,
            TransformList(vec![TransformOperation::Scale3d(1.0, 1.0, 0.0)]),
        );
        let child = Element::create(&document, "div").unwrap();
        child.set_width(LengthValue::px(40.0)).unwrap();
        child.set_height(LengthValue::px(40.0)).unwrap();
        child.set_background_color(Color::RED).unwrap();
        node.append_child(&child).unwrap();
        let blank = Document::with_font_collection(
            ViewportMetrics::from_logical_size(400.0, 200.0, scale).unwrap(),
            FontCollection::deterministic_test(),
        )
        .unwrap();
        blank.body().set_background_color(Color::WHITE).unwrap();
        assert_eq!(
            document.render_to_png_buffer().unwrap(),
            blank.render_to_png_buffer().unwrap()
        );
        node.set_property(
            StyleProperty::Transform,
            TransformList(vec![TransformOperation::Scale3d(1.0, 1.0, 1.0)]).into(),
        )
        .unwrap();
        let visible = document.render_to_png_buffer().unwrap();
        assert_ne!(visible, blank.render_to_png_buffer().unwrap());
        node.set_property(
            StyleProperty::Transform,
            TransformList(vec![TransformOperation::Scale3d(1.0, 1.0, 0.0)]).into(),
        )
        .unwrap();
        assert_eq!(
            document.render_to_png_buffer().unwrap(),
            blank.render_to_png_buffer().unwrap()
        );
    }
}

#[test]
fn default_flat_descendants_use_the_parent_projected_plane() {
    fn child(document: &Document, parent: &Element) {
        let child = Element::create(document, "div").unwrap();
        child.set_width(LengthValue::px(60.0)).unwrap();
        child.set_height(LengthValue::px(30.0)).unwrap();
        child.set_background_color(Color::RED).unwrap();
        child
            .set_property(
                StyleProperty::TransformOrigin,
                StyleValue::Renderer(RendererStyleValue::TransformOrigin((
                    Length::px(0.0),
                    Length::px(0.0),
                ))),
            )
            .unwrap();
        child
            .set_property(
                StyleProperty::Transform,
                TransformList(vec![TransformOperation::Rotate3d {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    degrees: -45.0,
                }])
                .into(),
            )
            .unwrap();
        parent.append_child(&child).unwrap();
    }
    let h = std::f64::consts::SQRT_2 / 2.0;
    let flat = Transform3D([
        h as f32,
        0.0,
        0.0,
        (h / 500.0) as f32,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]);
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (actual, parent) = fixture(
            scale,
            100.0,
            TransformList(vec![
                TransformOperation::Perspective(LengthValue::px(500.0)),
                TransformOperation::Rotate3d {
                    x: 0.0,
                    y: 1.0,
                    z: 0.0,
                    degrees: 45.0,
                },
            ]),
        );
        child(&actual, &parent);
        let (expected, parent) = fixture(
            scale,
            100.0,
            TransformList(vec![TransformOperation::Matrix3d(flat)]),
        );
        child(&expected, &parent);
        assert_eq!(
            actual.render_to_png_buffer().unwrap(),
            expected.render_to_png_buffer().unwrap()
        );
    }
}
