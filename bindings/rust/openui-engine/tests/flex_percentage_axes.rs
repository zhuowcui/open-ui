//! Native percentage sizing keeps physical property selection independent
//! from the flex container's logical percentage bases. No markup or script.

use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    AspectRatio, BoxSizing, Direction, Display, FlexDirection, ItemAlignment, ItemPosition,
    LengthValue, Overflow, RendererStyleValue, ScrollbarGutter, StyleProperty, StyleValue,
    WritingMode,
};

fn fixture(
    mode: WritingMode,
    direction: Direction,
    flex: FlexDirection,
    scale: f64,
) -> (Engine, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap()).unwrap();
    let container = engine.create_native_element(ElementTag::Div).unwrap();
    let child = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(engine.root(), container).unwrap();
    engine.append_child(container, child).unwrap();
    for (property, value) in [
        (StyleProperty::Display, Display::Flex.into()),
        (StyleProperty::Width, LengthValue::px(120.0).into()),
        (StyleProperty::Height, LengthValue::px(100.0).into()),
        (StyleProperty::WritingMode, mode.into()),
        (StyleProperty::Direction, direction.into()),
        (StyleProperty::FlexDirection, flex.into()),
        (
            StyleProperty::AlignItems,
            ItemAlignment::new(ItemPosition::Start).into(),
        ),
        (StyleProperty::Overflow, Overflow::Hidden.into()),
    ] {
        engine.set_property(container, property, value).unwrap();
    }
    for (property, value) in [
        (StyleProperty::Width, LengthValue::percent(100.0).into()),
        (StyleProperty::Height, LengthValue::percent(100.0).into()),
        (StyleProperty::MinWidth, LengthValue::px(0.0).into()),
        (StyleProperty::MinHeight, LengthValue::px(0.0).into()),
        (StyleProperty::FlexShrink, StyleValue::Number(0.0)),
    ] {
        engine.set_property(child, property, value).unwrap();
    }
    (engine, container, child)
}

fn gutter(engine: &mut Engine, container: NodeHandle, value: ScrollbarGutter) {
    engine
        .set_property(
            container,
            StyleProperty::ScrollbarGutter,
            StyleValue::Renderer(RendererStyleValue::ScrollbarGutter(value)),
        )
        .unwrap();
}

fn size(engine: &mut Engine, node: NodeHandle) -> (f32, f32) {
    let rect = engine.bounds(node).unwrap().unwrap();
    (rect.width, rect.height)
}

fn dimensions(engine: &mut Engine, node: NodeHandle) -> [i32; 4] {
    [
        engine.client_width(node).unwrap(),
        engine.client_height(node).unwrap(),
        engine.scroll_width(node).unwrap(),
        engine.scroll_height(node).unwrap(),
    ]
}

#[test]
fn native_flex_percentage_cross_sizes_follow_logical_axes_and_real_gutters() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            for direction in [Direction::Ltr, Direction::Rtl] {
                for flex in [
                    FlexDirection::Row,
                    FlexDirection::RowReverse,
                    FlexDirection::Column,
                    FlexDirection::ColumnReverse,
                ] {
                    let (mut engine, container, child) = fixture(mode, direction, flex, scale);
                    for (value, edges) in [
                        (ScrollbarGutter::Auto, 0),
                        (ScrollbarGutter::Stable, 15),
                        (ScrollbarGutter::StableBothEdges, 30),
                        (ScrollbarGutter::Auto, 0),
                    ] {
                        gutter(&mut engine, container, value);
                        let expected = if mode == WritingMode::HorizontalTb {
                            [120 - edges, 100, 120 - edges, 100]
                        } else {
                            [120, 100 - edges, 120, 100 - edges]
                        };
                        assert_eq!(
                            dimensions(&mut engine, container),
                            expected,
                            "{mode:?} {direction:?} {flex:?} {value:?}"
                        );
                        assert_eq!(
                            size(&mut engine, child),
                            (expected[0] as f32, expected[1] as f32)
                        );
                        assert_eq!(size(&mut engine, container), (120.0, 100.0));
                        let stats = engine.lifecycle_stats();
                        gutter(&mut engine, container, value);
                        assert_eq!(dimensions(&mut engine, container), expected);
                        assert_eq!(engine.lifecycle_stats(), stats);
                    }
                }
            }
        }
    }
}

#[test]
fn native_flex_cross_preferred_minimum_and_maximum_percentages_share_one_basis() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            for flex in [FlexDirection::Row, FlexDirection::Column] {
                let (mut engine, container, child) = fixture(mode, Direction::Ltr, flex, scale);
                gutter(&mut engine, container, ScrollbarGutter::Stable);
                let physical_main_horizontal =
                    (mode == WritingMode::HorizontalTb) != flex.is_column();
                let (cross_property, cross_min, cross_max) = if physical_main_horizontal {
                    (
                        StyleProperty::Height,
                        StyleProperty::MinHeight,
                        StyleProperty::MaxHeight,
                    )
                } else {
                    (
                        StyleProperty::Width,
                        StyleProperty::MinWidth,
                        StyleProperty::MaxWidth,
                    )
                };
                let available = if mode == WritingMode::HorizontalTb {
                    (105.0, 100.0)
                } else {
                    (120.0, 85.0)
                };
                let cross_base = if physical_main_horizontal {
                    available.1
                } else {
                    available.0
                };
                engine
                    .set_property(child, cross_property, LengthValue::percent(25.0).into())
                    .unwrap();
                let expected_size = |cross| {
                    if physical_main_horizontal {
                        (available.0, cross)
                    } else {
                        (cross, available.1)
                    }
                };
                assert_eq!(size(&mut engine, child), expected_size(cross_base * 0.25));
                engine
                    .set_property(child, cross_min, LengthValue::percent(50.0).into())
                    .unwrap();
                assert_eq!(size(&mut engine, child), expected_size(cross_base * 0.5));
                engine
                    .set_property(child, cross_min, LengthValue::px(0.0).into())
                    .unwrap();
                engine
                    .set_property(child, cross_property, LengthValue::percent(100.0).into())
                    .unwrap();
                engine
                    .set_property(child, cross_max, LengthValue::percent(40.0).into())
                    .unwrap();
                assert_eq!(size(&mut engine, child), expected_size(cross_base * 0.4));
                assert_eq!(size(&mut engine, container), (120.0, 100.0));
            }
        }
    }
}

#[test]
fn native_flex_cross_percentage_basis_survives_an_orthogonal_item() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in [WritingMode::HorizontalTb, WritingMode::VerticalRl] {
            for flex in [FlexDirection::Row, FlexDirection::Column] {
                let (mut engine, container, child) = fixture(mode, Direction::Ltr, flex, scale);
                gutter(&mut engine, container, ScrollbarGutter::Stable);
                let child_mode = if mode == WritingMode::HorizontalTb {
                    WritingMode::VerticalRl
                } else {
                    WritingMode::HorizontalTb
                };
                engine
                    .set_property(child, StyleProperty::WritingMode, child_mode.into())
                    .unwrap();
                let expected = if mode == WritingMode::HorizontalTb {
                    (105.0, 100.0)
                } else {
                    (120.0, 85.0)
                };
                assert_eq!(size(&mut engine, child), expected);
            }
        }
    }
}

#[test]
fn native_flex_aspect_ratio_max_cross_transfer_uses_physical_ratio_and_logical_base() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            for direction in [Direction::Ltr, Direction::Rtl] {
                for flex in [
                    FlexDirection::Row,
                    FlexDirection::RowReverse,
                    FlexDirection::Column,
                    FlexDirection::ColumnReverse,
                ] {
                    for box_sizing in [BoxSizing::ContentBox, BoxSizing::BorderBox] {
                        let (mut engine, container, child) = fixture(mode, direction, flex, scale);
                        gutter(&mut engine, container, ScrollbarGutter::Stable);
                        let physical_main_horizontal =
                            (mode == WritingMode::HorizontalTb) != flex.is_column();
                        let (main_property, cross_max, cross_base) = if physical_main_horizontal {
                            (
                                StyleProperty::Width,
                                StyleProperty::MaxHeight,
                                if mode == WritingMode::HorizontalTb {
                                    100.0
                                } else {
                                    85.0
                                },
                            )
                        } else {
                            (
                                StyleProperty::Height,
                                StyleProperty::MaxWidth,
                                if mode == WritingMode::HorizontalTb {
                                    105.0
                                } else {
                                    120.0
                                },
                            )
                        };
                        engine
                            .set_property(child, main_property, LengthValue::auto().into())
                            .unwrap();
                        engine
                            .set_property(child, StyleProperty::BoxSizing, box_sizing.into())
                            .unwrap();
                        engine
                            .set_property(
                                child,
                                StyleProperty::AspectRatio,
                                StyleValue::Renderer(RendererStyleValue::AspectRatio(Some(
                                    AspectRatio {
                                        ratio: (2.0, 1.0),
                                        auto_flag: false,
                                    },
                                ))),
                            )
                            .unwrap();
                        // Both dimensions are derived from the definite cross size.
                        // The 2:1 ratio is physical width/height in every writing mode.
                        // No border/padding means both box-sizing paths share geometry.
                        for percent in [50.0, 25.0, 50.0] {
                            engine
                                .set_property(
                                    child,
                                    cross_max,
                                    LengthValue::percent(percent).into(),
                                )
                                .unwrap();
                            let cross = cross_base * percent / 100.0;
                            let expected = if physical_main_horizontal {
                                (cross * 2.0, cross)
                            } else {
                                (cross, cross / 2.0)
                            };
                            assert_eq!(
                                size(&mut engine, child),
                                expected,
                                "{mode:?} {direction:?} {flex:?} {box_sizing:?} max {percent}%"
                            );
                            assert_eq!(size(&mut engine, container), (120.0, 100.0));
                            let stats = engine.lifecycle_stats();
                            engine
                                .set_property(
                                    child,
                                    cross_max,
                                    LengthValue::percent(percent).into(),
                                )
                                .unwrap();
                            assert_eq!(size(&mut engine, child), expected);
                            assert_eq!(engine.lifecycle_stats(), stats);
                        }
                    }
                }
            }
        }
    }
}
