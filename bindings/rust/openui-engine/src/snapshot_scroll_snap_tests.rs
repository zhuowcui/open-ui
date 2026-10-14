//! Neighboring guards for the common retained Engine snap lifecycle.
//! No generated fixture, reference image or test identity participates here.

use super::*;
use crate::{EngineOptions, FontCollection, RendererNodeState, ViewportMetrics};
use openui_compositor::SoftwareCompositor;
use openui_style::{
    apply_to_computed, Color, Direction, Display, Overflow, Position, RendererStyleValue,
    ScrollSnapAlign, ScrollSnapAxis, ScrollSnapStop, ScrollbarWidth, Transform2D,
    TransformOperation, WritingMode,
};

#[derive(Clone, Copy)]
enum Input {
    Native,
    Snapshot,
    Derived,
}

fn style(engine: &mut Engine, input: Input, node: NodeHandle, values: Vec<RendererStyleValue>) {
    match input {
        Input::Native => {
            for value in values {
                engine
                    .set_property(node, value.property(), StyleValue::Renderer(value))
                    .unwrap();
            }
        }
        Input::Snapshot => {
            for value in values {
                engine.set_renderer_style(node, value).unwrap();
            }
        }
        Input::Derived => {
            let mut computed = engine.computed_style(node).unwrap().clone();
            for value in values {
                apply_to_computed(
                    &mut computed,
                    value.property(),
                    &StyleValue::Renderer(value),
                    (320.0, 240.0),
                )
                .unwrap();
            }
            engine.install_derived_style(node, computed).unwrap();
        }
    }
}

fn fixture(input: Input, scale: f64) -> (Engine, NodeHandle, NodeHandle) {
    let mut engine = Engine::new_with_font_collection_and_options(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
        FontCollection::deterministic_test(),
        EngineOptions::default(),
    )
    .unwrap();
    let origin = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), origin).unwrap();
    style(
        &mut engine,
        input,
        origin,
        vec![
            RendererStyleValue::Display(Display::Block),
            RendererStyleValue::Width(Length::px(100.0)),
            RendererStyleValue::Height(Length::px(80.0)),
            RendererStyleValue::OverflowX(Overflow::Hidden),
            RendererStyleValue::OverflowY(Overflow::Hidden),
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
            RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::X),
            RendererStyleValue::ScrollSnapStrictness(ScrollSnapStrictness::Mandatory),
        ],
    );
    let content = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(origin, content).unwrap();
    style(
        &mut engine,
        input,
        content,
        vec![
            RendererStyleValue::Display(Display::Block),
            RendererStyleValue::Position(Position::Relative),
            RendererStyleValue::Width(Length::px(400.0)),
            RendererStyleValue::Height(Length::px(300.0)),
        ],
    );
    (engine, origin, content)
}

fn area(engine: &mut Engine, input: Input, content: NodeHandle, x: f32, y: f32) -> NodeHandle {
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(content, node).unwrap();
    style(
        engine,
        input,
        node,
        vec![
            RendererStyleValue::Display(Display::Block),
            RendererStyleValue::Position(Position::Absolute),
            RendererStyleValue::Left(Length::px(x)),
            RendererStyleValue::Top(Length::px(y)),
            RendererStyleValue::Width(Length::px(20.0)),
            RendererStyleValue::Height(Length::px(20.0)),
            RendererStyleValue::BackgroundColor(Color::BLUE),
            RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Start),
            RendererStyleValue::ScrollSnapStop(ScrollSnapStop::Normal),
        ],
    );
    node
}

#[test]
fn computed_snapshots_and_native_declarations_share_initial_offsets_resnap_geometry_and_pixels() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let mut observed = Vec::new();
        for input in [Input::Native, Input::Snapshot, Input::Derived] {
            let (mut engine, origin, content) = fixture(input, scale);
            style(
                &mut engine,
                input,
                origin,
                vec![RendererStyleValue::ScrollPaddingLeft(Length::percent(10.0))],
            );
            let first = area(&mut engine, input, content, 40.0, 20.0);
            area(&mut engine, input, content, 140.0, 20.0);
            style(
                &mut engine,
                input,
                first,
                vec![RendererStyleValue::ScrollMarginLeft(Length::px(5.0))],
            );
            engine.scene().unwrap();
            assert_eq!(engine.scroll_offset(origin).unwrap(), (25.0, 0.0));
            assert_eq!(
                engine.scroll_buttons.snap_targets[&origin],
                [Some(first), None]
            );
            assert_eq!(engine.hit_test(16.0, 25.0).unwrap(), Some(first));
            // The layout box stays at its authored location; only retained
            // scrolling places its public border box at x15.
            assert_eq!(engine.bounds(first).unwrap().unwrap().x, 15.0);
            style(
                &mut engine,
                input,
                first,
                vec![RendererStyleValue::Left(Length::px(85.0))],
            );
            let scene = engine.scene().unwrap();
            assert_eq!(engine.scroll_offset(origin).unwrap(), (70.0, 0.0));
            assert_eq!(engine.scroll_buttons.snap_targets[&origin][0], Some(first));
            let bounds = engine.bounds(first).unwrap();
            let metrics = engine.scroll_metrics(origin).unwrap();
            let frame = SoftwareCompositor::default().render(&scene).unwrap();
            observed.push((bounds, metrics, frame.pixels));
            let stats = engine.stats();
            engine.scene().unwrap();
            assert_eq!(engine.stats(), stats);
        }
        assert_eq!(observed[0], observed[1]);
        assert_eq!(observed[0], observed[2]);
    }
}

#[test]
fn snapshot_offset_requests_use_end_selection_instead_of_old_layout_target() {
    let (mut engine, origin, content) = fixture(Input::Snapshot, 1.0);
    let first = area(&mut engine, Input::Snapshot, content, 40.0, 20.0);
    let second = area(&mut engine, Input::Snapshot, content, 200.0, 20.0);
    engine.scene().unwrap();
    assert_eq!(engine.scroll_buttons.snap_targets[&origin][0], Some(first));
    engine
        .set_renderer_node_state(origin, RendererNodeState::ScrollLeft(190.0))
        .unwrap();
    engine.scene().unwrap();
    assert_eq!(engine.scroll_offset(origin).unwrap(), (200.0, 0.0));
    assert_eq!(engine.scroll_buttons.snap_targets[&origin][0], Some(second));
    assert!(engine.scroll_buttons.pending_end_axes.is_empty());
    let stats = engine.stats();
    engine.scene().unwrap();
    assert_eq!(engine.stats(), stats);
    // An ordinary application endpoint uses the same selection operation.
    engine.scroll_to(origin, 50.0, 0.0).unwrap();
    assert_eq!(engine.scroll_offset(origin).unwrap(), (40.0, 0.0));
    assert_eq!(engine.scroll_buttons.snap_targets[&origin][0], Some(first));
}

#[test]
fn proximity_endpoint_without_candidate_clears_committed_snap_identity() {
    let (mut engine, origin, content) = fixture(Input::Snapshot, 1.0);
    let first = area(&mut engine, Input::Snapshot, content, 40.0, 20.0);
    area(&mut engine, Input::Snapshot, content, 200.0, 20.0);
    engine.scene().unwrap();
    assert_eq!(engine.scroll_buttons.snap_targets[&origin][0], Some(first));
    engine
        .set_renderer_style(
            origin,
            RendererStyleValue::ScrollSnapStrictness(ScrollSnapStrictness::Proximity),
        )
        .unwrap();
    engine
        .set_renderer_node_state(origin, RendererNodeState::ScrollLeft(120.0))
        .unwrap();
    engine.scene().unwrap();
    assert_eq!(engine.scroll_offset(origin).unwrap(), (120.0, 0.0));
    assert_eq!(engine.scroll_buttons.snap_targets[&origin], [None, None]);
}

#[test]
fn snapshot_snap_requires_a_scrollport_and_clamps_to_reachable_content() {
    for input in [Input::Snapshot, Input::Derived] {
        let (mut engine, origin, content) = fixture(input, 1.0);
        let target = area(&mut engine, input, content, 350.0, 20.0);
        engine.scene().unwrap();
        assert_eq!(engine.scroll_offset(origin).unwrap(), (300.0, 0.0));
        assert_eq!(engine.bounds(target).unwrap().unwrap().x, 50.0);
        style(
            &mut engine,
            input,
            origin,
            vec![
                RendererStyleValue::OverflowX(Overflow::Visible),
                RendererStyleValue::OverflowY(Overflow::Visible),
            ],
        );
        engine.scene().unwrap();
        assert_eq!(engine.scroll_offset(origin).unwrap(), (0.0, 0.0));
        assert_eq!(engine.bounds(target).unwrap().unwrap().x, 350.0);
        assert_eq!(
            engine
                .scroll_buttons
                .snap_targets
                .get(&origin)
                .copied()
                .unwrap_or([None, None]),
            [None, None]
        );
    }
}

#[test]
fn snapshot_split_alignment_writing_modes_and_subpixel_transform_share_native_geometry() {
    for writing in [
        WritingMode::HorizontalTb,
        WritingMode::VerticalRl,
        WritingMode::VerticalLr,
    ] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let mut observed = Vec::new();
            for input in [Input::Native, Input::Snapshot, Input::Derived] {
                let (mut engine, origin, content) = fixture(input, 1.25);
                style(
                    &mut engine,
                    input,
                    origin,
                    vec![
                        RendererStyleValue::WritingMode(writing),
                        RendererStyleValue::Direction(direction),
                        RendererStyleValue::ScrollSnapAxis(ScrollSnapAxis::Inline),
                    ],
                );
                style(
                    &mut engine,
                    input,
                    content,
                    vec![
                        RendererStyleValue::WritingMode(writing),
                        RendererStyleValue::Direction(direction),
                    ],
                );
                let target = area(&mut engine, input, content, 40.0, 20.0);
                style(
                    &mut engine,
                    input,
                    target,
                    vec![
                        RendererStyleValue::WritingMode(writing),
                        RendererStyleValue::Direction(direction),
                        RendererStyleValue::ScrollSnapAlignPair([
                            ScrollSnapAlign::End,
                            ScrollSnapAlign::Start,
                        ]),
                        RendererStyleValue::Transform(Transform2D {
                            a: 1.0,
                            b: 0.0,
                            c: 0.0,
                            d: 1.0,
                            e: 15.5,
                            f: 7.5,
                        }),
                    ],
                );
                let scene = engine.scene().unwrap();
                let bounds = engine.bounds(target).unwrap();
                let offset = engine.scroll_offset(origin).unwrap();
                let frame = SoftwareCompositor::default().render(&scene).unwrap();
                let ids = engine
                    .scroll_buttons
                    .snap_targets
                    .get(&origin)
                    .copied()
                    .unwrap_or([None, None])
                    .map(|id| id.map(NodeHandle::index));
                observed.push((offset, bounds, ids, frame.pixels));
            }
            assert_eq!(observed[0], observed[1]);
            assert_eq!(observed[0], observed[2]);
        }
    }
}

#[test]
fn native_percentage_transform_snap_area_tracks_its_owned_reference_box() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, origin, content) = fixture(Input::Native, scale);
        let target = area(&mut engine, Input::Native, content, 40.0, 20.0);
        engine
            .set_property(
                target,
                StyleProperty::Transform,
                openui_style::TransformList(vec![TransformOperation::Translate(
                    openui_style::LengthValue::percent(50.0),
                    openui_style::LengthValue::px(0.0),
                )])
                .into(),
            )
            .unwrap();
        for (width, expected_x) in [(20.0, 50.0), (60.0, 70.0), (20.0, 50.0)] {
            engine
                .set_property(
                    target,
                    StyleProperty::Width,
                    openui_style::LengthValue::px(width).into(),
                )
                .unwrap();
            engine.update().unwrap();
            let geometry = engine
                .snap_geometry(engine.latest_fragment.as_ref().unwrap(), origin)
                .unwrap()
                .unwrap();
            let area = geometry
                .areas
                .iter()
                .find(|area| area.handle == target)
                .unwrap();
            assert_eq!(area.rect, [expected_x, 20.0, width, 20.0], "scale={scale}");
        }
    }
}

#[test]
fn derived_deferred_3d_snap_ancestry_stays_explicitly_unsupported() {
    let (mut engine, origin, content) = fixture(Input::Derived, 1.25);
    area(&mut engine, Input::Derived, content, 40.0, 20.0);
    let mut computed = engine.computed_style(content).unwrap().clone();
    computed.update_derived(|style| {
        style.native_transform = Some(openui_style::TransformList(vec![
            TransformOperation::PendingInterpolation {
                from: Box::new(openui_style::TransformList::default()),
                to: Box::new(openui_style::TransformList(vec![
                    TransformOperation::Rotate3d {
                        x: 0.0,
                        y: 1.0,
                        z: 0.0,
                        degrees: 45.0,
                    },
                ])),
                progress: 0.5,
            },
        ]));
    });
    engine.install_derived_style(content, computed).unwrap();
    engine.update().unwrap();
    assert!(matches!(
        engine.snap_geometry(engine.latest_fragment.as_ref().unwrap(), origin),
        Err("three-dimensional snap quads require retained 3D scene geometry")
    ));
}

#[test]
fn anonymous_table_helpers_do_not_create_extra_snap_areas_or_reapply_transform() {
    for input in [Input::Native, Input::Snapshot, Input::Derived] {
        let (mut engine, origin, content) = fixture(input, 1.0);
        let table = engine.create_native_element(ElementTag::Table).unwrap();
        engine.append_child(content, table).unwrap();
        style(
            &mut engine,
            input,
            table,
            vec![
                RendererStyleValue::Display(Display::Table),
                RendererStyleValue::Position(Position::Absolute),
                RendererStyleValue::Left(Length::px(60.0)),
                RendererStyleValue::Top(Length::px(10.0)),
                RendererStyleValue::Width(Length::px(40.0)),
                RendererStyleValue::ScrollSnapAlign(ScrollSnapAlign::Start),
                RendererStyleValue::Transform(Transform2D {
                    a: 1.0,
                    b: 0.0,
                    c: 0.0,
                    d: 1.0,
                    e: 8.5,
                    f: 0.0,
                }),
            ],
        );
        let caption = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(table, caption).unwrap();
        style(
            &mut engine,
            input,
            caption,
            vec![
                RendererStyleValue::Display(Display::TableCaption),
                RendererStyleValue::Height(Length::px(12.0)),
                RendererStyleValue::BackgroundColor(Color::BLUE),
            ],
        );
        let cell = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(table, cell).unwrap();
        style(
            &mut engine,
            input,
            cell,
            vec![
                RendererStyleValue::Display(Display::TableCell),
                RendererStyleValue::Height(Length::px(20.0)),
                RendererStyleValue::BackgroundColor(Color::RED),
            ],
        );
        engine.scene().unwrap();
        let root = engine.latest_fragment.as_ref().unwrap();
        let geometry = engine.snap_geometry(root, origin).unwrap().unwrap();
        assert_eq!(geometry.areas.len(), 1);
        assert_eq!(geometry.areas[0].handle, table);
        let area_rect = geometry.areas[0].rect;
        let offset = engine.scroll_offset(origin).unwrap();
        let rects = engine.client_rects(table).unwrap();
        assert_eq!(rects.len(), 1);
        assert_eq!(area_rect[0] as f64, rects[0].x as f64 + offset.0);
        assert_eq!(area_rect[2], rects[0].width);
        assert_eq!(engine.scroll_buttons.snap_targets[&origin][0], Some(table));
    }
}
