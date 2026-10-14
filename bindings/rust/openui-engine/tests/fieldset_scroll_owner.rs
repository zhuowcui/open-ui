//! Public retained Engine fieldset consumers. Native Rust only, no markup or
//! script. These guards are source-only until compiled/run with renderer gates.
use openui_dom::ElementTag;
use openui_engine::{AnimationTimelineSource, Engine, NodeHandle};
use openui_geometry::{Length, ViewportMetrics};
use openui_style::{
    AnimationAttachmentRange, AnimationOptions, Border, BorderStyle, Color, Direction, Display,
    Edges, FillMode, FlexDirection, GridTrackBreadth, GridTrackComponent, GridTrackList,
    GridTrackSize, Keyframes, LengthValue, Overflow, Position, PropertyKeyframes,
    RendererStyleValue, StyleProperty, StyleValue, TextAlign, TimelineAxis, WritingMode,
};

fn set(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    value: impl Into<StyleValue>,
) {
    engine.set_property(node, property, value.into()).unwrap();
}
fn box_(
    engine: &mut Engine,
    parent: NodeHandle,
    tag: ElementTag,
    width: f32,
    height: f32,
) -> NodeHandle {
    let node = engine.create_native_element(tag).unwrap();
    set(engine, node, StyleProperty::Display, Display::Block);
    set(engine, node, StyleProperty::Width, LengthValue::px(width));
    set(engine, node, StyleProperty::Height, LengthValue::px(height));
    set(
        engine,
        node,
        StyleProperty::Margin,
        Edges::all(LengthValue::px(0.0)),
    );
    set(
        engine,
        node,
        StyleProperty::Padding,
        Edges::all(LengthValue::px(0.0)),
    );
    set(
        engine,
        node,
        StyleProperty::Border,
        Border {
            width: 0.0,
            style: BorderStyle::None,
            color: Color::BLACK,
        },
    );
    set(engine, node, StyleProperty::MinWidth, LengthValue::px(0.0));
    set(engine, node, StyleProperty::MinHeight, LengthValue::px(0.0));
    engine.append_child(parent, node).unwrap();
    node
}
fn fixture(scale: f64) -> (Engine, NodeHandle, NodeHandle, NodeHandle) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(500.0, 400.0, scale).unwrap()).unwrap();
    let root = engine.root();
    let fieldset = box_(&mut engine, root, ElementTag::Fieldset, 100.0, 80.0);
    set(
        &mut engine,
        fieldset,
        StyleProperty::Border,
        Border {
            width: 2.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        },
    );
    set(
        &mut engine,
        fieldset,
        StyleProperty::Overflow,
        Overflow::Hidden,
    );
    let legend = box_(&mut engine, fieldset, ElementTag::Legend, 30.0, 10.0);
    let content = box_(&mut engine, fieldset, ElementTag::Div, 100.0, 200.0);
    (engine, fieldset, legend, content)
}
fn rect(engine: &mut Engine, node: NodeHandle) -> openui_compositor::SceneRect {
    engine.bounds(node).unwrap().unwrap()
}
fn view(
    engine: &mut Engine,
    subject: NodeHandle,
    target: NodeHandle,
) -> openui_engine::AnimationId {
    engine
        .animate_with_attachment_range(
            target,
            PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::from_values(0.0_f32, 1.0_f32),
            )
            .unwrap(),
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
            AnimationTimelineSource::View {
                subject,
                axis: TimelineAxis::Block,
            },
            AnimationAttachmentRange::default(),
        )
        .unwrap()
}

#[test]
fn wrapper_client_geometry_and_content_scroll_geometry_are_distinct_owned_native_values() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        let wrapper = rect(&mut engine, fieldset);
        let original_legend = rect(&mut engine, legend);
        let original_content = rect(&mut engine, content);
        assert_eq!((wrapper.width, wrapper.height), (104.0, 84.0));
        assert_eq!(engine.client_rects(fieldset).unwrap(), vec![wrapper]);
        assert_eq!(
            (
                engine.client_width(fieldset).unwrap(),
                engine.client_height(fieldset).unwrap()
            ),
            (100, 80)
        );
        assert_eq!(
            (
                engine.scroll_width(fieldset).unwrap(),
                engine.scroll_height(fieldset).unwrap()
            ),
            (100, 200)
        );
        let metrics = engine.scroll_metrics(fieldset).unwrap().unwrap();
        assert_eq!((metrics.client_width, metrics.client_height), (100.0, 72.0));
        assert_eq!(original_content.y - wrapper.y, 10.0);
        engine.scroll_to(fieldset, 0.0, 1000.0).unwrap();
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 128.0));
        assert_eq!(rect(&mut engine, legend), original_legend);
        assert_eq!(rect(&mut engine, fieldset), wrapper);
        assert_eq!(rect(&mut engine, content).y, original_content.y - 128.0);
        assert_eq!(
            engine
                .hit_test(original_legend.x + 5.0, original_legend.y + 5.0)
                .unwrap(),
            Some(legend)
        );
        engine.scroll_to(fieldset, 0.0, 0.0).unwrap();
        assert_eq!(rect(&mut engine, content), original_content);
    }
}

#[test]
fn flex_and_grid_format_the_content_without_making_the_rendered_legend_an_item() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for display in [Display::Flex, Display::Grid] {
            let (mut engine, fieldset, legend, first) = fixture(scale);
            set(&mut engine, fieldset, StyleProperty::Display, display);
            set(
                &mut engine,
                fieldset,
                StyleProperty::FlexDirection,
                FlexDirection::Row,
            );
            if display == Display::Grid {
                let track = GridTrackComponent::Track(GridTrackSize::Breadth(
                    GridTrackBreadth::Length(Length::px(20.0)),
                ));
                set(
                    &mut engine,
                    fieldset,
                    StyleProperty::GridTemplateColumns,
                    StyleValue::Renderer(RendererStyleValue::GridTemplateColumns(
                        GridTrackList::Tracks(vec![track.clone()]),
                    )),
                );
                set(
                    &mut engine,
                    fieldset,
                    StyleProperty::GridTemplateRows,
                    StyleValue::Renderer(RendererStyleValue::GridTemplateRows(
                        GridTrackList::Tracks(vec![track.clone(), track]),
                    )),
                );
            }
            set(
                &mut engine,
                first,
                StyleProperty::Width,
                LengthValue::px(20.0),
            );
            set(
                &mut engine,
                first,
                StyleProperty::Height,
                LengthValue::px(20.0),
            );
            set(&mut engine, first, StyleProperty::FlexShrink, 0.0);
            let second = box_(&mut engine, fieldset, ElementTag::Div, 20.0, 20.0);
            set(&mut engine, second, StyleProperty::FlexShrink, 0.0);
            let a = rect(&mut engine, first);
            let b = rect(&mut engine, second);
            let title = rect(&mut engine, legend);
            assert_eq!(a.y - title.y, 10.0);
            assert_eq!(a.x, rect(&mut engine, fieldset).x + 2.0);
            if display == Display::Flex {
                assert_eq!((b.x - a.x, b.y - a.y), (20.0, 0.0));
            } else {
                assert_eq!((b.x - a.x, b.y - a.y), (0.0, 20.0));
            }
            assert_eq!(engine.client_rects(fieldset).unwrap().len(), 1);
        }
    }
}

#[test]
fn overflow_and_legend_style_mutations_rebuild_the_same_content_scroll_owner() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        let later = box_(&mut engine, fieldset, ElementTag::Legend, 30.0, 20.0);
        let title = rect(&mut engine, legend);
        engine.scroll_to(fieldset, 0.0, 20.0).unwrap();
        set(
            &mut engine,
            fieldset,
            StyleProperty::Overflow,
            Overflow::Clip,
        );
        assert_eq!(rect(&mut engine, legend), title);
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 0.0));
        engine.scroll_to(fieldset, 0.0, 20.0).unwrap();
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 0.0));
        set(
            &mut engine,
            fieldset,
            StyleProperty::Overflow,
            Overflow::Hidden,
        );
        set(&mut engine, legend, StyleProperty::Display, Display::None);
        assert_eq!(
            rect(&mut engine, content).y - rect(&mut engine, fieldset).y,
            20.0
        );
        let new_title = rect(&mut engine, later);
        engine.scroll_to(fieldset, 0.0, 40.0).unwrap();
        assert_eq!(rect(&mut engine, later), new_title);
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 40.0));
        set(&mut engine, legend, StyleProperty::Display, Display::Block);
        engine.scroll_to(fieldset, 0.0, 0.0).unwrap();
        assert_eq!(rect(&mut engine, legend), title);
    }
}

#[test]
fn view_timeline_uses_content_membership_and_excludes_the_rendered_legend() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        let root = engine.root();
        let target_a = box_(&mut engine, root, ElementTag::Div, 5.0, 5.0);
        let target_b = box_(&mut engine, root, ElementTag::Div, 5.0, 5.0);
        let content_animation = view(&mut engine, content, target_a);
        let legend_animation = view(&mut engine, legend, target_b);
        engine.update().unwrap();
        assert_eq!(
            engine
                .animation_timeline_snapshot(content_animation)
                .unwrap()
                .unwrap()
                .source,
            Some(fieldset)
        );
        assert_eq!(
            engine
                .animation_timeline_snapshot(legend_animation)
                .unwrap()
                .unwrap()
                .source,
            Some(root)
        );
        let legend_before = engine
            .animation_timeline_snapshot(legend_animation)
            .unwrap()
            .unwrap();
        engine.scroll_to(fieldset, 0.0, 20.0).unwrap();
        assert_eq!(
            engine
                .animation_timeline_snapshot(legend_animation)
                .unwrap()
                .unwrap(),
            legend_before
        );
        assert_eq!(
            engine
                .animation_timeline_snapshot(content_animation)
                .unwrap()
                .unwrap()
                .scroll_offset,
            Some(20.0)
        );
    }
}

#[test]
fn sticky_content_uses_the_content_scrollport_and_the_legend_stays_outside() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        set(
            &mut engine,
            content,
            StyleProperty::Height,
            LengthValue::px(20.0),
        );
        set(
            &mut engine,
            content,
            StyleProperty::Position,
            Position::Sticky,
        );
        set(
            &mut engine,
            content,
            StyleProperty::Top,
            LengthValue::px(0.0),
        );
        box_(&mut engine, fieldset, ElementTag::Div, 100.0, 200.0);
        let title = rect(&mut engine, legend);
        let y = rect(&mut engine, content).y;
        for offset in [20.0, 40.0, 20.0, 0.0] {
            engine.scroll_to(fieldset, 0.0, offset).unwrap();
            assert_eq!(rect(&mut engine, content).y, y);
            assert_eq!(rect(&mut engine, legend), title);
        }
    }
}

#[test]
fn native_reveal_moves_content_and_never_scrolls_the_legend_through_its_fieldset() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        let target = box_(&mut engine, content, ElementTag::Div, 20.0, 20.0);
        set(
            &mut engine,
            target,
            StyleProperty::MarginTop,
            LengthValue::px(150.0),
        );
        engine
            .scroll_into_view(target, openui_engine::ScrollIntoViewOptions::default())
            .unwrap();
        assert!(engine.scroll_offset(fieldset).unwrap().1 > 0.0);
        let before = engine.scroll_offset(fieldset).unwrap();
        engine
            .scroll_into_view(legend, openui_engine::ScrollIntoViewOptions::default())
            .unwrap();
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), before);
    }
}

#[test]
fn wheel_chain_follows_content_membership_but_explicit_scroll_targets_the_element() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        set(
            &mut engine,
            fieldset,
            StyleProperty::OverflowY,
            Overflow::Auto,
        );
        set(
            &mut engine,
            content,
            StyleProperty::Width,
            LengthValue::px(80.0),
        );
        let title = rect(&mut engine, legend);
        assert!(!engine.scroll_wheel_from_target(legend, 0.0, 20.0).unwrap());
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 0.0));
        assert!(engine.scroll_wheel_from_target(content, 0.0, 20.0).unwrap());
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 20.0));
        assert_eq!(rect(&mut engine, legend), title);
        assert!(engine.scroll_wheel(fieldset, 0.0, 20.0).unwrap());
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 40.0));
        assert!(!engine.scroll_wheel_from_target(legend, 0.0, 20.0).unwrap());
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 40.0));
        let before = engine.scroll_offset(fieldset).unwrap();
        assert!(engine
            .scroll_wheel_from_target(content, 0.0, f64::NAN)
            .is_err());
        assert_eq!(engine.scroll_offset(fieldset).unwrap(), before);
    }
}

#[test]
fn no_legend_and_nested_fieldset_keep_independent_retained_scroll_state() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        set(&mut engine, legend, StyleProperty::Display, Display::None);
        assert_eq!(
            rect(&mut engine, content).y - rect(&mut engine, fieldset).y,
            2.0
        );
        let inner = box_(&mut engine, content, ElementTag::Fieldset, 60.0, 40.0);
        set(
            &mut engine,
            inner,
            StyleProperty::Overflow,
            Overflow::Hidden,
        );
        let title = box_(&mut engine, inner, ElementTag::Legend, 20.0, 8.0);
        let leaf = box_(&mut engine, inner, ElementTag::Div, 60.0, 100.0);
        let title_before = rect(&mut engine, title);
        let leaf_before = rect(&mut engine, leaf);
        engine.scroll_to(inner, 0.0, 10.0).unwrap();
        assert_eq!(rect(&mut engine, title), title_before);
        assert_eq!(rect(&mut engine, leaf).y, leaf_before.y - 10.0);
        engine.scroll_to(fieldset, 0.0, 20.0).unwrap();
        assert_eq!(rect(&mut engine, title).y, title_before.y - 20.0);
        assert_eq!(engine.scroll_offset(inner).unwrap(), (0.0, 10.0));
        assert_eq!(rect(&mut engine, leaf).y, leaf_before.y - 30.0);
    }
}

#[test]
fn positioned_legend_descendants_and_content_descendants_use_distinct_boxes() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        set(
            &mut engine,
            fieldset,
            StyleProperty::Position,
            Position::Relative,
        );
        let legend_child = box_(&mut engine, legend, ElementTag::Div, 5.0, 5.0);
        let content_child = box_(&mut engine, content, ElementTag::Div, 5.0, 5.0);
        for child in [legend_child, content_child] {
            set(
                &mut engine,
                child,
                StyleProperty::Position,
                Position::Absolute,
            );
            set(&mut engine, child, StyleProperty::Top, LengthValue::px(0.0));
            set(
                &mut engine,
                child,
                StyleProperty::Left,
                LengthValue::px(0.0),
            );
        }
        let wrapper = rect(&mut engine, fieldset);
        assert_eq!(rect(&mut engine, legend_child).y, wrapper.y + 2.0);
        assert_eq!(rect(&mut engine, content_child).y, wrapper.y + 10.0);
        let a = rect(&mut engine, legend_child);
        let b = rect(&mut engine, content_child);
        engine.scroll_to(fieldset, 0.0, 20.0).unwrap();
        assert_eq!(rect(&mut engine, legend_child), a);
        assert_eq!(rect(&mut engine, content_child).y, b.y - 20.0);
    }
}

#[test]
fn padding_uses_parent_percentage_basis_and_remains_in_the_content_formatting_box() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        set(
            &mut engine,
            fieldset,
            StyleProperty::Padding,
            Edges::all(LengthValue::px(5.0)),
        );
        let wrapper = rect(&mut engine, fieldset);
        assert_eq!((wrapper.width, wrapper.height), (114.0, 94.0));
        assert_eq!(rect(&mut engine, content).y - wrapper.y, 15.0);
        assert_eq!(rect(&mut engine, legend).x - wrapper.x, 7.0);
        assert_eq!(
            (
                engine.client_width(fieldset).unwrap(),
                engine.client_height(fieldset).unwrap()
            ),
            (110, 90)
        );
        let metrics = engine.scroll_metrics(fieldset).unwrap().unwrap();
        assert_eq!((metrics.client_width, metrics.client_height), (110.0, 82.0));
        assert_eq!(engine.scroll_height(fieldset).unwrap(), 210);
        set(
            &mut engine,
            fieldset,
            StyleProperty::Padding,
            Edges::all(LengthValue::percent(10.0)),
        );
        let wrapper = rect(&mut engine, fieldset);
        assert_eq!((wrapper.width, wrapper.height), (204.0, 184.0));
        // Parent inline basis500 yields50px on every physical edge; the
        // anonymous box must not reinterpret10% against its own200px size.
        assert_eq!(rect(&mut engine, content).x - wrapper.x, 52.0);
        assert_eq!(rect(&mut engine, content).y - wrapper.y, 60.0);
        assert_eq!(engine.client_width(fieldset).unwrap(), 200);
    }
}

#[test]
fn vertical_content_scroll_uses_the_logical_block_axis_without_moving_legend() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for writing in [WritingMode::VerticalRl, WritingMode::VerticalLr] {
            let (mut engine, fieldset, legend, content) = fixture(scale);
            set(&mut engine, fieldset, StyleProperty::WritingMode, writing);
            set(
                &mut engine,
                fieldset,
                StyleProperty::Width,
                LengthValue::px(80.0),
            );
            set(
                &mut engine,
                fieldset,
                StyleProperty::Height,
                LengthValue::px(100.0),
            );
            set(
                &mut engine,
                legend,
                StyleProperty::Width,
                LengthValue::px(10.0),
            );
            set(
                &mut engine,
                legend,
                StyleProperty::Height,
                LengthValue::px(30.0),
            );
            set(
                &mut engine,
                content,
                StyleProperty::Width,
                LengthValue::px(200.0),
            );
            set(
                &mut engine,
                content,
                StyleProperty::Height,
                LengthValue::px(100.0),
            );
            let wrapper = rect(&mut engine, fieldset);
            let title = rect(&mut engine, legend);
            let child = rect(&mut engine, content);
            assert_eq!((wrapper.width, wrapper.height), (84.0, 104.0));
            assert_eq!(
                (
                    engine.client_width(fieldset).unwrap(),
                    engine.client_height(fieldset).unwrap()
                ),
                (80, 100)
            );
            let metrics = engine.scroll_metrics(fieldset).unwrap().unwrap();
            assert_eq!((metrics.client_width, metrics.client_height), (72.0, 100.0));
            assert_eq!(
                (
                    engine.scroll_width(fieldset).unwrap(),
                    engine.scroll_height(fieldset).unwrap()
                ),
                (200, 100)
            );
            let offset = if writing == WritingMode::VerticalRl {
                -20.0
            } else {
                20.0
            };
            engine.scroll_to(fieldset, offset, 0.0).unwrap();
            assert_eq!(engine.scroll_offset(fieldset).unwrap(), (offset, 0.0));
            assert_eq!(rect(&mut engine, content).x, child.x - offset as f32);
            assert_eq!(rect(&mut engine, legend), title);
            engine.scroll_to(fieldset, 0.0, 0.0).unwrap();
            assert_eq!(rect(&mut engine, content), child);
        }
    }
}

#[test]
fn legend_auto_margins_and_pinned_legacy_alignment_obey_fieldset_direction() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for direction in [Direction::Ltr, Direction::Rtl] {
            let (mut engine, fieldset, legend, _) = fixture(scale);
            set(&mut engine, fieldset, StyleProperty::Direction, direction);
            set(
                &mut engine,
                legend,
                StyleProperty::TextAlign,
                TextAlign::Right,
            );
            assert_eq!(
                rect(&mut engine, legend).x - rect(&mut engine, fieldset).x,
                72.0
            );
            set(
                &mut engine,
                legend,
                StyleProperty::TextAlign,
                TextAlign::Left,
            );
            assert_eq!(
                rect(&mut engine, legend).x - rect(&mut engine, fieldset).x,
                2.0
            );
            set(
                &mut engine,
                legend,
                StyleProperty::TextAlign,
                TextAlign::End,
            );
            assert_eq!(
                rect(&mut engine, legend).x - rect(&mut engine, fieldset).x,
                if direction == Direction::Ltr {
                    2.0
                } else {
                    72.0
                }
            );
            set(
                &mut engine,
                legend,
                StyleProperty::MarginLeft,
                LengthValue::auto(),
            );
            set(
                &mut engine,
                legend,
                StyleProperty::MarginRight,
                LengthValue::auto(),
            );
            assert_eq!(
                rect(&mut engine, legend).x - rect(&mut engine, fieldset).x,
                37.0
            );
            set(
                &mut engine,
                legend,
                StyleProperty::MarginRight,
                LengthValue::px(0.0),
            );
            assert_eq!(
                rect(&mut engine, legend).x - rect(&mut engine, fieldset).x,
                72.0
            );
            assert_eq!(engine.scroll_offset(fieldset).unwrap(), (0.0, 0.0));
        }
    }
}

#[test]
fn owned_scroll_paint_moves_only_content_and_unchanged_reads_reuse_the_retained_scene() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, content) = fixture(scale);
        set(
            &mut engine,
            fieldset,
            StyleProperty::OverflowY,
            Overflow::Scroll,
        );
        set(
            &mut engine,
            legend,
            StyleProperty::BackgroundColor,
            Color::RED,
        );
        let first = box_(&mut engine, content, ElementTag::Div, 100.0, 20.0);
        let second = box_(&mut engine, content, ElementTag::Div, 100.0, 180.0);
        set(
            &mut engine,
            first,
            StyleProperty::BackgroundColor,
            Color::GREEN,
        );
        set(
            &mut engine,
            second,
            StyleProperty::BackgroundColor,
            Color::BLUE,
        );
        let mut compositor = openui_compositor::SoftwareCompositor::default();
        let before = compositor.render(&engine.scene().unwrap()).unwrap();
        let title = rect(&mut engine, legend);
        let sample = |frame: &openui_compositor::Frame, x: f64, y: f64| {
            let px = (x * scale).floor() as usize;
            let py = (y * scale).floor() as usize;
            let index = py * frame.stride + px * 4;
            <[u8; 4]>::try_from(&frame.pixels[index..index + 4]).unwrap()
        };
        let title_ink = sample(&before, 10.0, 5.0);
        let content_ink = sample(&before, 20.0, 15.0);
        engine.scroll_to(fieldset, 0.0, 20.0).unwrap();
        let after = compositor.render(&engine.scene().unwrap()).unwrap();
        assert_eq!(rect(&mut engine, legend), title);
        assert_eq!(sample(&after, 10.0, 5.0), title_ink);
        assert_ne!(sample(&after, 20.0, 15.0), content_ink);
        let stats = engine.stats();
        assert_eq!(compositor.render(&engine.scene().unwrap()).unwrap(), after);
        assert_eq!(engine.stats(), stats);
        engine.scroll_to(fieldset, 0.0, 0.0).unwrap();
        assert_eq!(
            compositor.render(&engine.scene().unwrap()).unwrap().pixels,
            before.pixels
        );
    }
}

#[test]
fn wrapper_decoration_survives_content_scroll_and_table_grid_delegation() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (mut engine, fieldset, legend, _) = fixture(scale);
        set(
            &mut engine,
            fieldset,
            StyleProperty::BackgroundColor,
            Color::BLUE,
        );
        set(
            &mut engine,
            fieldset,
            StyleProperty::Border,
            Border {
                width: 4.0,
                style: BorderStyle::Solid,
                color: Color::RED,
            },
        );
        set(
            &mut engine,
            legend,
            StyleProperty::BackgroundColor,
            Color::GREEN,
        );
        let root = engine.root();
        let neighbor = box_(&mut engine, root, ElementTag::Div, 100.0, 80.0);
        set(
            &mut engine,
            neighbor,
            StyleProperty::BackgroundColor,
            Color::BLUE,
        );
        set(
            &mut engine,
            neighbor,
            StyleProperty::Border,
            Border {
                width: 4.0,
                style: BorderStyle::Solid,
                color: Color::RED,
            },
        );
        let table = box_(&mut engine, root, ElementTag::Table, 80.0, 40.0);
        set(&mut engine, table, StyleProperty::Display, Display::Table);
        set(
            &mut engine,
            table,
            StyleProperty::BackgroundColor,
            Color::GREEN,
        );
        set(
            &mut engine,
            table,
            StyleProperty::Border,
            Border {
                width: 4.0,
                style: BorderStyle::Solid,
                color: Color::BLUE,
            },
        );
        let caption = box_(&mut engine, table, ElementTag::Div, 80.0, 10.0);
        set(
            &mut engine,
            caption,
            StyleProperty::Display,
            Display::TableCaption,
        );
        set(
            &mut engine,
            caption,
            StyleProperty::BackgroundColor,
            Color::RED,
        );
        let cell = box_(&mut engine, table, ElementTag::Div, 60.0, 20.0);
        set(
            &mut engine,
            cell,
            StyleProperty::Display,
            Display::TableCell,
        );
        let wrapper = rect(&mut engine, fieldset);
        let ordinary = rect(&mut engine, neighbor);
        let grid = rect(&mut engine, table);
        let cell_box = rect(&mut engine, cell);
        let mut compositor = openui_compositor::SoftwareCompositor::default();
        let before = compositor.render(&engine.scene().unwrap()).unwrap();
        let sample = |frame: &openui_compositor::Frame, x: f32, y: f32| {
            let px = (x as f64 * scale).floor() as usize;
            let py = (y as f64 * scale).floor() as usize;
            let index = py * frame.stride + px * 4;
            <[u8; 4]>::try_from(&frame.pixels[index..index + 4]).unwrap()
        };
        assert_eq!(
            sample(&before, wrapper.x + 50.0, wrapper.y + 30.0),
            [0, 0, 255, 255]
        );
        assert_eq!(
            sample(&before, wrapper.x + 2.0, wrapper.y + 30.0),
            [255, 0, 0, 255]
        );
        assert_eq!(
            sample(&before, ordinary.x + 50.0, ordinary.y + 30.0),
            [0, 0, 255, 255]
        );
        assert_eq!(
            sample(&before, ordinary.x + 2.0, ordinary.y + 30.0),
            [255, 0, 0, 255]
        );
        assert_eq!(
            sample(
                &before,
                cell_box.x + cell_box.width / 2.0,
                cell_box.y + cell_box.height / 2.0
            ),
            [0, 128, 0, 255]
        );
        assert_eq!(
            sample(&before, grid.x + 2.0, cell_box.y + cell_box.height / 2.0),
            [0, 0, 255, 255]
        );
        engine.scroll_to(fieldset, 0.0, 20.0).unwrap();
        assert_eq!(
            compositor.render(&engine.scene().unwrap()).unwrap().pixels,
            before.pixels
        );
        set(
            &mut engine,
            fieldset,
            StyleProperty::BackgroundColor,
            Color::RED,
        );
        let mutated = compositor.render(&engine.scene().unwrap()).unwrap();
        assert_eq!(
            sample(&mutated, wrapper.x + 50.0, wrapper.y + 30.0),
            [255, 0, 0, 255]
        );
        assert_eq!(
            sample(&mutated, grid.x + 2.0, cell_box.y + cell_box.height / 2.0),
            [0, 0, 255, 255]
        );
    }
}
