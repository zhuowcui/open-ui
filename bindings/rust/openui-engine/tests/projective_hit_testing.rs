//! Native geometry, clips, and input use the same retained physical transform.

use openui_dom::ElementTag;
use openui_engine::{Engine, EngineError, NodeHandle, ViewportMetrics};
use openui_style::{
    ColumnFill, Display, LengthValue, Overflow, Position, RendererStyleValue, StyleProperty,
    StyleValue, Transform3D, TransformList, TransformOperation,
};

fn engine(scale: f64) -> Engine {
    Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap()
}

fn box_(engine: &mut Engine, parent: NodeHandle, width: f32, height: f32) -> NodeHandle {
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    engine
        .set_property(node, StyleProperty::Display, Display::Block.into())
        .unwrap();
    for (property, value) in [
        (StyleProperty::Width, width),
        (StyleProperty::Height, height),
    ] {
        engine
            .set_property(node, property, LengthValue::px(value).into())
            .unwrap();
    }
    engine
        .set_property(node, StyleProperty::Position, Position::Relative.into())
        .unwrap();
    engine
        .set_property(
            node,
            StyleProperty::TransformOrigin,
            StyleValue::Renderer(RendererStyleValue::TransformOrigin((
                openui_geometry::Length::px(0.0),
                openui_geometry::Length::px(0.0),
            ))),
        )
        .unwrap();
    engine.append_child(parent, node).unwrap();
    node
}

fn transform(engine: &mut Engine, node: NodeHandle, matrix: Transform3D) {
    engine
        .set_property(
            node,
            StyleProperty::Transform,
            TransformList(vec![TransformOperation::Matrix3d(matrix)]).into(),
        )
        .unwrap();
}

#[test]
fn projective_hit_bounds_and_mutations_use_actual_box_homogeneous_coordinates() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let mut e = engine(scale);
        let root = e.root();
        let node = box_(&mut e, root, 80.0, 40.0);
        let mut matrix = Transform3D::IDENTITY;
        matrix.0[3] = 0.0125;
        transform(&mut e, node, matrix);
        let bounds = e.bounds(node).unwrap().unwrap();
        assert!(bounds.width < 80.0 && bounds.width > 39.0);
        assert_eq!(e.hit_test(20.0, 10.0).unwrap(), Some(node));
        assert_ne!(e.hit_test(60.0, 10.0).unwrap(), Some(node));
        let generation = e.scene().unwrap().generation();
        let stats = e.stats();
        assert_eq!(e.hit_test(20.0, 10.0).unwrap(), Some(node));
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
        transform(&mut e, node, Transform3D::IDENTITY);
        assert_eq!(e.bounds(node).unwrap().unwrap().width, 80.0);
        assert_eq!(e.hit_test(60.0, 10.0).unwrap(), Some(node));
    }
}

#[test]
fn depth_singular_ancestor_suppresses_its_flat_descendants_but_keeps_layout_rects() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let mut e = engine(scale);
        let root = e.root();
        let parent = box_(&mut e, root, 80.0, 40.0);
        let child = box_(&mut e, parent, 60.0, 30.0);
        let mut matrix = Transform3D::IDENTITY;
        matrix.0[10] = 0.0;
        transform(&mut e, parent, matrix);
        let rects = e.client_rects(child).unwrap();
        assert!(!rects.is_empty());
        assert_eq!(rects[0].width, 60.0);
        let hit = e.hit_test(20.0, 10.0).unwrap();
        assert_ne!(hit, Some(parent));
        assert_ne!(hit, Some(child));
        transform(&mut e, parent, Transform3D::IDENTITY);
        assert_eq!(e.hit_test(20.0, 10.0).unwrap(), Some(child));
    }
}

#[test]
fn flat_element_boundary_prevents_nested_depth_from_rotating_the_parent_plane() {
    let mut e = engine(1.25);
    let root = e.root();
    let parent = box_(&mut e, root, 100.0, 60.0);
    let child = box_(&mut e, parent, 80.0, 40.0);
    let mut matrix = Transform3D::IDENTITY;
    matrix.0[0] = 0.5;
    matrix.0[2] = -0.75;
    matrix.0[8] = 0.75;
    matrix.0[10] = 0.5;
    transform(&mut e, parent, matrix);
    transform(&mut e, child, matrix);
    assert_eq!(e.bounds(child).unwrap().unwrap().width, 20.0);
    assert_eq!(e.hit_test(10.0, 10.0).unwrap(), Some(child));
    assert_ne!(e.hit_test(30.0, 10.0).unwrap(), Some(child));
}

#[test]
fn full_transform_is_retained_by_saved_immutable_scenes() {
    let mut e = engine(1.5);
    let root = e.root();
    let node = box_(&mut e, root, 80.0, 40.0);
    let mut matrix = Transform3D::IDENTITY;
    matrix.0[3] = 0.0125;
    transform(&mut e, node, matrix);
    let saved = e.scene().unwrap();
    let mut before = Vec::new();
    fn transforms(fragment: &openui_layout::Fragment, out: &mut Vec<[f64; 16]>) {
        if let Some(transform) = fragment.resolved_transform {
            out.push(transform.matrix.0);
        }
        for child in &fragment.children {
            transforms(child, out);
        }
    }
    transforms(saved.fragments(), &mut before);
    assert!(before.iter().any(|m| m[3] != 0.0));
    transform(&mut e, node, Transform3D::IDENTITY);
    let changed = e.scene().unwrap();
    assert_ne!(saved.generation(), changed.generation());
    drop(e);
    let mut after = Vec::new();
    transforms(saved.fragments(), &mut after);
    assert_eq!(after, before);
}

#[test]
fn projected_overflow_clip_uses_container_plane_and_visible_axis_policy() {
    let mut e = engine(1.25);
    let root = e.root();
    let parent = box_(&mut e, root, 40.0, 40.0);
    let child = box_(&mut e, parent, 80.0, 80.0);
    let mut matrix = Transform3D::IDENTITY;
    matrix.0[0] = 0.5;
    matrix.0[2] = -0.75;
    matrix.0[8] = 0.75;
    matrix.0[10] = 0.5;
    transform(&mut e, parent, matrix);
    e.set_property(parent, StyleProperty::OverflowX, Overflow::Clip.into())
        .unwrap();
    e.set_property(parent, StyleProperty::OverflowY, Overflow::Visible.into())
        .unwrap();
    assert_eq!(e.hit_test(10.0, 55.0).unwrap(), Some(child));
    assert_ne!(e.hit_test(30.0, 15.0).unwrap(), Some(child));
    e.set_property(parent, StyleProperty::OverflowX, Overflow::Visible.into())
        .unwrap();
    assert_eq!(e.hit_test(30.0, 15.0).unwrap(), Some(child));
}

#[test]
fn promoted_multicol_ancestor_transform_reaches_native_rectangles_and_input() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut e = engine(scale);
        let root = e.root();
        let columns = box_(&mut e, root, 200.0, 40.0);
        for (property, value) in [
            (
                StyleProperty::ColumnCount,
                RendererStyleValue::ColumnCount(Some(2)),
            ),
            (
                StyleProperty::ColumnFill,
                RendererStyleValue::ColumnFill(ColumnFill::Auto),
            ),
            (
                StyleProperty::ColumnGap,
                RendererStyleValue::ColumnGap(Some(openui_geometry::Length::px(0.0))),
            ),
        ] {
            e.set_property(columns, property, StyleValue::Renderer(value))
                .unwrap();
        }
        let ancestor = box_(&mut e, columns, 100.0, 100.0);
        e.set_property(
            ancestor,
            StyleProperty::Transform,
            TransformList(vec![TransformOperation::Scale(0.5, 0.5)]).into(),
        )
        .unwrap();
        let child = box_(&mut e, ancestor, 8.0, 8.0);
        e.set_property(child, StyleProperty::Position, Position::Fixed.into())
            .unwrap();
        for property in [StyleProperty::Left, StyleProperty::Top] {
            e.set_property(child, property, LengthValue::px(0.0).into())
                .unwrap();
        }
        fn has_promoted(fragment: &openui_layout::Fragment) -> bool {
            !fragment.promoted_transform_ancestors.is_empty()
                || fragment.children.iter().any(has_promoted)
        }
        // A small fixed child of the real transformed continuation stays under
        // its owning ancestor. Crossing a fragmentainer with a late relative
        // displacement genuinely requires the promoted ancestor projection.
        for promoted in [false, true, false] {
            e.set_property(
                ancestor,
                StyleProperty::Position,
                if promoted {
                    Position::Relative
                } else {
                    Position::Static
                }
                .into(),
            )
            .unwrap();
            e.set_property(
                ancestor,
                StyleProperty::Top,
                if promoted {
                    LengthValue::px(50.0)
                } else {
                    LengthValue::auto()
                }
                .into(),
            )
            .unwrap();
            assert_eq!(has_promoted(e.scene().unwrap().fragments()), promoted);
            let rectangles = e.client_rects(child).unwrap();
            assert!(!rectangles.is_empty());
            for rect in &rectangles {
                assert_eq!(rect.width, 4.0);
                assert_eq!(rect.height, 4.0);
            }
            let rect = rectangles[0];
            assert_eq!(e.hit_test(rect.x + 2.0, rect.y + 2.0).unwrap(), Some(child));
            let stats = e.stats();
            assert_eq!(e.client_rects(child).unwrap(), rectangles);
            assert_eq!(e.stats(), stats);
        }
    }
}

#[test]
fn calculated_clip_insets_restrict_input_and_can_be_removed_natively() {
    let mut e = engine(1.5);
    let root = e.root();
    let parent = box_(&mut e, root, 80.0, 40.0);
    let child = box_(&mut e, parent, 80.0, 40.0);
    e.set_property(
        parent,
        StyleProperty::ClipPathInset,
        StyleValue::Renderer(RendererStyleValue::ClipPathInset(Some([
            openui_geometry::Length::calc_percent_px(25.0, 5.0),
            openui_geometry::Length::px(0.0),
            openui_geometry::Length::px(0.0),
            openui_geometry::Length::calc_percent_px(25.0, 5.0),
        ]))),
    )
    .unwrap();
    assert_ne!(e.hit_test(20.0, 20.0).unwrap(), Some(child));
    assert_ne!(e.hit_test(30.0, 10.0).unwrap(), Some(child));
    assert_eq!(e.hit_test(30.0, 20.0).unwrap(), Some(child));
    let stats = e.stats();
    assert_eq!(e.hit_test(30.0, 20.0).unwrap(), Some(child));
    assert_eq!(e.stats(), stats);
    e.set_property(
        parent,
        StyleProperty::ClipPathInset,
        StyleValue::Renderer(RendererStyleValue::ClipPathInset(None)),
    )
    .unwrap();
    assert_eq!(e.hit_test(20.0, 20.0).unwrap(), Some(child));
    assert_eq!(e.hit_test(30.0, 10.0).unwrap(), Some(child));
}

#[test]
fn nonfinite_hit_coordinates_fail_before_work_or_state_changes() {
    let mut e = engine(1.0);
    let stats = e.stats();
    for (x, y) in [
        (f32::NAN, 0.0),
        (0.0, f32::INFINITY),
        (f32::NEG_INFINITY, 0.0),
    ] {
        assert!(matches!(
            e.hit_test(x, y),
            Err(EngineError::InvalidInput(_))
        ));
        assert_eq!(e.stats(), stats);
    }
}
