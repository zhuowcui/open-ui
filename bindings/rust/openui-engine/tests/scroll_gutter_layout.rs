//! Native Engine guards for retained flex/grid scrollbar geometry.
//! The four integer queries require the additive native dimension checkpoint.
//! These tests are source-only until compiled and run after the census audit.

use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{
    Border, BorderStyle, Color, Direction, Display, Edges, FlexDirection, GridTrackBreadth,
    GridTrackComponent, GridTrackList, GridTrackSize, LengthValue, Overflow, Position,
    RendererStyleValue, ScrollbarGutter, ScrollbarWidth, StyleProperty, StyleValue, WritingMode,
};

fn renderer(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    value: RendererStyleValue,
) {
    engine
        .set_property(node, property, StyleValue::Renderer(value))
        .unwrap();
}

fn percent_track() -> GridTrackList {
    GridTrackList::Tracks(vec![GridTrackComponent::Track(GridTrackSize::Breadth(
        GridTrackBreadth::Length(Length::percent(100.0)),
    ))])
}

fn fixture(display: Display, scale: f64) -> (Engine, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap();
    let container = engine.create_native_element(ElementTag::Div).unwrap();
    let child = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), container).unwrap();
    engine.append_child(container, child).unwrap();
    engine
        .set_property(container, StyleProperty::Display, display.into())
        .unwrap();
    engine
        .set_property(
            container,
            StyleProperty::Width,
            LengthValue::px(100.0).into(),
        )
        .unwrap();
    engine
        .set_property(
            container,
            StyleProperty::Height,
            LengthValue::px(80.0).into(),
        )
        .unwrap();
    engine
        .set_property(container, StyleProperty::Overflow, Overflow::Scroll.into())
        .unwrap();
    engine
        .set_property(
            container,
            StyleProperty::FlexDirection,
            FlexDirection::Column.into(),
        )
        .unwrap();
    renderer(
        &mut engine,
        container,
        StyleProperty::GridTemplateColumns,
        RendererStyleValue::GridTemplateColumns(percent_track()),
    );
    renderer(
        &mut engine,
        container,
        StyleProperty::GridTemplateRows,
        RendererStyleValue::GridTemplateRows(percent_track()),
    );
    for property in [StyleProperty::Width, StyleProperty::Height] {
        engine
            .set_property(child, property, LengthValue::percent(100.0).into())
            .unwrap();
    }
    engine
        .set_property(child, StyleProperty::FlexShrink, StyleValue::Number(0.0))
        .unwrap();
    for property in [StyleProperty::MinWidth, StyleProperty::MinHeight] {
        engine
            .set_property(child, property, LengthValue::px(0.0).into())
            .unwrap();
    }
    (engine, container, child)
}

fn dimensions(engine: &mut Engine, node: NodeHandle) -> [i32; 4] {
    [
        engine.client_width(node).unwrap(),
        engine.client_height(node).unwrap(),
        engine.scroll_width(node).unwrap(),
        engine.scroll_height(node).unwrap(),
    ]
}

fn size(engine: &mut Engine, node: NodeHandle) -> (f32, f32) {
    let bounds = engine.bounds(node).unwrap().unwrap();
    (bounds.width, bounds.height)
}

#[test]
fn forced_gutters_resize_real_percentage_children_without_inflating_css_sizes() {
    for display in [Display::Block, Display::Flex, Display::Grid] {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let (mut engine, container, child) = fixture(display, scale);
            for (width, client) in [
                (ScrollbarWidth::Auto, (85, 65)),
                (ScrollbarWidth::None, (100, 80)),
                (ScrollbarWidth::Thin, (90, 70)),
            ] {
                renderer(
                    &mut engine,
                    container,
                    StyleProperty::ScrollbarWidth,
                    RendererStyleValue::ScrollbarWidth(width),
                );
                assert_eq!(
                    dimensions(&mut engine, container),
                    [client.0, client.1, client.0, client.1]
                );
                assert_eq!(size(&mut engine, container), (100.0, 80.0));
                assert_eq!(size(&mut engine, child), (client.0 as f32, client.1 as f32));
            }
            renderer(
                &mut engine,
                container,
                StyleProperty::ScrollbarWidth,
                RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Auto),
            );
            engine
                .set_property(
                    container,
                    StyleProperty::Padding,
                    Edges::all(LengthValue::px(5.0)).into(),
                )
                .unwrap();
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
            assert_eq!(dimensions(&mut engine, container), [95, 75, 95, 75]);
            assert_eq!(size(&mut engine, container), (114.0, 94.0));
            assert_eq!(size(&mut engine, child), (85.0, 65.0));
            let stats = engine.stats();
            assert_eq!(dimensions(&mut engine, container), [95, 75, 95, 75]);
            assert_eq!(
                engine.stats(),
                stats,
                "unchanged dimensions must reuse retained layout"
            );
        }
    }
}

#[test]
fn auto_overflow_adds_both_gutters_and_removes_them_after_native_mutation() {
    for display in [Display::Flex, Display::Grid] {
        let (mut engine, container, child) = fixture(display, 1.0);
        engine
            .set_property(container, StyleProperty::Overflow, Overflow::Auto.into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Width, LengthValue::px(150.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Height, LengthValue::px(120.0).into())
            .unwrap();
        assert_eq!(dimensions(&mut engine, container), [85, 65, 150, 120]);
        engine.scroll_to(container, 1000.0, 1000.0).unwrap();
        assert_eq!(engine.scroll_offset(container).unwrap(), (65.0, 55.0));
        engine
            .set_property(child, StyleProperty::Width, LengthValue::px(20.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Height, LengthValue::px(20.0).into())
            .unwrap();
        assert_eq!(dimensions(&mut engine, container), [100, 80, 100, 80]);
        assert_eq!(engine.scroll_offset(container).unwrap(), (0.0, 0.0));
    }
}

#[test]
fn stable_hidden_gutters_follow_rtl_vertical_axes_and_both_edges() {
    for display in [Display::Flex, Display::Grid] {
        let (mut engine, container, child) = fixture(display, 1.0);
        engine
            .set_property(container, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        engine
            .set_property(container, StyleProperty::Direction, Direction::Rtl.into())
            .unwrap();
        renderer(
            &mut engine,
            container,
            StyleProperty::ScrollbarGutter,
            RendererStyleValue::ScrollbarGutter(ScrollbarGutter::Stable),
        );
        assert_eq!(dimensions(&mut engine, container), [85, 80, 85, 80]);
        let parent = engine.bounds(container).unwrap().unwrap();
        let bounds = engine.bounds(child).unwrap().unwrap();
        assert_eq!((bounds.x - parent.x, bounds.width), (15.0, 85.0));
        renderer(
            &mut engine,
            container,
            StyleProperty::ScrollbarGutter,
            RendererStyleValue::ScrollbarGutter(ScrollbarGutter::StableBothEdges),
        );
        assert_eq!(dimensions(&mut engine, container), [70, 80, 70, 80]);
        let bounds = engine.bounds(child).unwrap().unwrap();
        assert_eq!((bounds.x - parent.x, bounds.width), (15.0, 70.0));
        engine
            .set_property(container, StyleProperty::Direction, Direction::Ltr.into())
            .unwrap();
        engine
            .set_property(
                container,
                StyleProperty::WritingMode,
                WritingMode::VerticalRl.into(),
            )
            .unwrap();
        for (gutter, expected_height, top) in [
            (ScrollbarGutter::Stable, 65, 0.0),
            (ScrollbarGutter::StableBothEdges, 50, 15.0),
        ] {
            renderer(
                &mut engine,
                container,
                StyleProperty::ScrollbarGutter,
                RendererStyleValue::ScrollbarGutter(gutter),
            );
            assert_eq!(
                dimensions(&mut engine, container),
                [100, expected_height, 100, expected_height]
            );
            let parent = engine.bounds(container).unwrap().unwrap();
            let bounds = engine.bounds(child).unwrap().unwrap();
            assert_eq!(
                (bounds.y - parent.y, bounds.width, bounds.height),
                (top, 100.0, expected_height as f32)
            );
        }
        engine
            .set_property(container, StyleProperty::Overflow, Overflow::Clip.into())
            .unwrap();
        assert_eq!(dimensions(&mut engine, container), [100, 80, 100, 80]);
    }
}

#[test]
fn positioned_percentages_use_the_same_gutter_reduced_containing_box() {
    for display in [Display::Flex, Display::Grid] {
        let (mut engine, container, child) = fixture(display, 1.0);
        engine
            .set_property(
                container,
                StyleProperty::Position,
                Position::Relative.into(),
            )
            .unwrap();
        engine
            .set_property(container, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        engine
            .set_property(container, StyleProperty::Direction, Direction::Rtl.into())
            .unwrap();
        renderer(
            &mut engine,
            container,
            StyleProperty::ScrollbarGutter,
            RendererStyleValue::ScrollbarGutter(ScrollbarGutter::Stable),
        );
        engine
            .set_property(child, StyleProperty::Position, Position::Absolute.into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Left, LengthValue::px(0.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Top, LengthValue::px(0.0).into())
            .unwrap();
        assert_eq!(size(&mut engine, child), (85.0, 80.0));
        let parent = engine.bounds(container).unwrap().unwrap();
        let child_bounds = engine.bounds(child).unwrap().unwrap();
        assert_eq!(
            (child_bounds.x - parent.x, child_bounds.y - parent.y),
            (15.0, 0.0)
        );
    }
}

#[test]
fn intrinsic_flex_grid_and_parent_measurements_include_reserved_gutters() {
    for display in [Display::InlineFlex, Display::InlineGrid] {
        let (mut engine, container, child) = fixture(display, 1.0);
        engine
            .set_property(
                container,
                StyleProperty::Width,
                LengthValue::Computed(Length::max_content()).into(),
            )
            .unwrap();
        engine
            .set_property(container, StyleProperty::Height, LengthValue::auto().into())
            .unwrap();
        renderer(
            &mut engine,
            container,
            StyleProperty::GridTemplateColumns,
            RendererStyleValue::GridTemplateColumns(GridTrackList::None),
        );
        renderer(
            &mut engine,
            container,
            StyleProperty::GridTemplateRows,
            RendererStyleValue::GridTemplateRows(GridTrackList::None),
        );
        engine
            .set_property(child, StyleProperty::Width, LengthValue::px(20.0).into())
            .unwrap();
        engine
            .set_property(child, StyleProperty::Height, LengthValue::px(10.0).into())
            .unwrap();
        assert_eq!(size(&mut engine, container), (35.0, 25.0));
        assert_eq!(dimensions(&mut engine, container), [20, 10, 20, 10]);
        let parent = engine.create_native_element(ElementTag::Div).unwrap();
        engine
            .set_property(parent, StyleProperty::Display, Display::InlineFlex.into())
            .unwrap();
        engine
            .set_property(
                parent,
                StyleProperty::Width,
                LengthValue::Computed(Length::max_content()).into(),
            )
            .unwrap();
        engine.append_child(engine.root(), parent).unwrap();
        engine.append_child(parent, container).unwrap();
        assert_eq!(size(&mut engine, parent).0, 35.0);
    }
}

#[test]
fn narrow_rtl_left_gutter_clamps_without_shifting_the_outer_box() {
    for display in [Display::Flex, Display::Grid] {
        let (mut engine, container, child) = fixture(display, 1.0);
        engine
            .set_property(container, StyleProperty::Overflow, Overflow::Hidden.into())
            .unwrap();
        engine
            .set_property(container, StyleProperty::Direction, Direction::Rtl.into())
            .unwrap();
        engine
            .set_property(container, StyleProperty::Width, LengthValue::px(8.0).into())
            .unwrap();
        renderer(
            &mut engine,
            container,
            StyleProperty::ScrollbarGutter,
            RendererStyleValue::ScrollbarGutter(ScrollbarGutter::Stable),
        );
        assert_eq!(dimensions(&mut engine, container), [0, 80, 0, 80]);
        assert_eq!(size(&mut engine, container), (8.0, 80.0));
        let parent = engine.bounds(container).unwrap().unwrap();
        assert_eq!(engine.bounds(child).unwrap().unwrap().x - parent.x, 8.0);
    }
}

#[test]
fn a_flex_assigned_grid_height_controls_percentage_rows_after_gutter_reservation() {
    let (mut engine, container, child) = fixture(Display::Grid, 1.0);
    let parent = engine.create_native_element(ElementTag::Div).unwrap();
    engine
        .set_property(parent, StyleProperty::Display, Display::Flex.into())
        .unwrap();
    engine
        .set_property(
            parent,
            StyleProperty::FlexDirection,
            FlexDirection::Column.into(),
        )
        .unwrap();
    engine
        .set_property(parent, StyleProperty::Width, LengthValue::px(100.0).into())
        .unwrap();
    engine
        .set_property(parent, StyleProperty::Height, LengthValue::px(160.0).into())
        .unwrap();
    engine
        .set_property(container, StyleProperty::FlexGrow, StyleValue::Number(1.0))
        .unwrap();
    engine
        .set_property(
            container,
            StyleProperty::FlexBasis,
            LengthValue::px(0.0).into(),
        )
        .unwrap();
    engine.append_child(engine.root(), parent).unwrap();
    engine.append_child(parent, container).unwrap();
    assert_eq!(size(&mut engine, container), (100.0, 160.0));
    assert_eq!(dimensions(&mut engine, container), [85, 145, 85, 145]);
    assert_eq!(size(&mut engine, child), (85.0, 145.0));
}

#[test]
fn gutter_and_width_style_mutations_rebuild_retained_geometry_once() {
    for display in [Display::Block, Display::Flex, Display::Grid] {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let (mut engine, container, child) = fixture(display, scale);
            engine
                .set_property(container, StyleProperty::Overflow, Overflow::Hidden.into())
                .unwrap();
            assert_eq!(dimensions(&mut engine, container), [100, 80, 100, 80]);
            for (gutter, client) in [
                (ScrollbarGutter::Stable, (85, 80)),
                (ScrollbarGutter::StableBothEdges, (70, 80)),
                (ScrollbarGutter::Auto, (100, 80)),
            ] {
                let before = engine.stats();
                renderer(
                    &mut engine,
                    container,
                    StyleProperty::ScrollbarGutter,
                    RendererStyleValue::ScrollbarGutter(gutter),
                );
                assert_eq!(
                    dimensions(&mut engine, container),
                    [client.0, client.1, client.0, client.1]
                );
                assert_eq!(size(&mut engine, container), (100.0, 80.0));
                assert_eq!(size(&mut engine, child), (client.0 as f32, client.1 as f32));
                assert!(engine.stats().layouts > before.layouts);
                let scene = engine.scene().unwrap();
                let settled = engine.stats();
                renderer(
                    &mut engine,
                    container,
                    StyleProperty::ScrollbarGutter,
                    RendererStyleValue::ScrollbarGutter(gutter),
                );
                assert_eq!(
                    dimensions(&mut engine, container),
                    [client.0, client.1, client.0, client.1]
                );
                assert_eq!(engine.scene().unwrap().generation(), scene.generation());
                assert_eq!(engine.stats(), settled);
            }
            engine
                .set_property(container, StyleProperty::Overflow, Overflow::Scroll.into())
                .unwrap();
            assert_eq!(dimensions(&mut engine, container), [85, 65, 85, 65]);
            for (width, client) in [
                (ScrollbarWidth::Thin, (90, 70)),
                (ScrollbarWidth::None, (100, 80)),
                (ScrollbarWidth::Auto, (85, 65)),
            ] {
                let before = engine.stats();
                renderer(
                    &mut engine,
                    container,
                    StyleProperty::ScrollbarWidth,
                    RendererStyleValue::ScrollbarWidth(width),
                );
                assert_eq!(
                    dimensions(&mut engine, container),
                    [client.0, client.1, client.0, client.1]
                );
                assert_eq!(size(&mut engine, container), (100.0, 80.0));
                assert_eq!(size(&mut engine, child), (client.0 as f32, client.1 as f32));
                assert!(engine.stats().layouts > before.layouts);
                let scene = engine.scene().unwrap();
                let settled = engine.stats();
                renderer(
                    &mut engine,
                    container,
                    StyleProperty::ScrollbarWidth,
                    RendererStyleValue::ScrollbarWidth(width),
                );
                assert_eq!(
                    dimensions(&mut engine, container),
                    [client.0, client.1, client.0, client.1]
                );
                assert_eq!(engine.scene().unwrap().generation(), scene.generation());
                assert_eq!(engine.stats(), settled);
            }
        }
    }
}
