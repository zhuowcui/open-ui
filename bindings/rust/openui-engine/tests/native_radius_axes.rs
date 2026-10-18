//! Native radius-axis sources, canonical ownership and shared scalar effects.
//! Exact native neighbors do not qualify Chromium raster coverage.
use openui_compositor::{SceneBoxRole, SceneNodeIdentity, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{AnimationId, AnimationTimeline, Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    AnimationOptions, Color, CompositeOperation, CornerRadii, Display, Edges, FillMode, Keyframes,
    LengthValue, Overflow, PropertyKeyframes, RendererStyleValue, StyleProperty, StyleValue,
    RADIUS_AXIS_PROPERTIES,
};
const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];
fn set(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: impl Into<StyleValue>) {
    e.set_property(n, p, v.into()).unwrap();
}
fn fixture(scale: f64) -> (Engine, NodeHandle) {
    let mut e =
        Engine::new(ViewportMetrics::from_logical_size(200.0, 180.0, scale).unwrap()).unwrap();
    let root = e.root();
    set(&mut e, root, StyleProperty::BackgroundColor, Color::WHITE);
    let n = e.create_native_element(ElementTag::Div).unwrap();
    set(&mut e, n, StyleProperty::Display, Display::Block);
    set(&mut e, n, StyleProperty::Width, LengthValue::px(100.0));
    set(&mut e, n, StyleProperty::Height, LengthValue::px(80.0));
    e.append_child(root, n).unwrap();
    (e, n)
}
fn options(composite: CompositeOperation) -> AnimationOptions {
    AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        composite,
        ..AnimationOptions::default()
    }
}
fn constant(
    e: &mut Engine,
    n: NodeHandle,
    p: StyleProperty,
    v: impl Into<StyleValue>,
    c: CompositeOperation,
) -> AnimationId {
    let v = v.into();
    e.animate(
        n,
        PropertyKeyframes::typed(p, Keyframes::from_values(v.clone(), v)).unwrap(),
        options(c),
        AnimationTimeline::Document,
    )
    .unwrap()
}
fn pair(e: &Engine, n: NodeHandle) -> (LengthValue, LengthValue) {
    (
        e.radius_axis_length(n, StyleProperty::BorderTopLeftRadiusX)
            .unwrap(),
        e.radius_axis_length(n, StyleProperty::BorderTopLeftRadiusY)
            .unwrap(),
    )
}
#[test]
fn actual_box_font_and_viewport_axes_feed_same_paint_hit_and_scene_operands() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale);
        set(&mut e, n, StyleProperty::Overflow, Overflow::Hidden);
        let child = e.create_native_element(ElementTag::Div).unwrap();
        for (p, v) in [
            (StyleProperty::Width, LengthValue::px(200.0)),
            (StyleProperty::Height, LengthValue::px(200.0)),
        ] {
            set(&mut e, child, p, v);
        }
        set(&mut e, child, StyleProperty::BackgroundColor, Color::GREEN);
        e.append_child(n, child).unwrap();
        let sources = [
            LengthValue::calc_percent_px(25.0, 5.0),
            LengthValue::percent(25.0),
            LengthValue::px(10.0),
            LengthValue::Em(1.0),
            LengthValue::ViewportWidth(10.0),
            LengthValue::px(12.0),
            LengthValue::px(8.0),
            LengthValue::calc_percent_px(10.0, 0.0),
        ];
        for (p, v) in RADIUS_AXIS_PROPERTIES.into_iter().zip(sources) {
            set(&mut e, n, p, v);
        }
        let calculated = e.scene().unwrap();
        let expected = SoftwareCompositor::default().render(&calculated).unwrap();
        let hits = [(1.0, 1.0), (50.0, 40.0), (98.0, 1.0)].map(|(x, y)| e.hit_test(x, y).unwrap());
        assert_ne!(hits[0], Some(child));
        assert_eq!(hits[1], Some(child));
        let identity = SceneNodeIdentity {
            document: n.document_id(),
            index: n.index(),
            generation: n.generation(),
        };
        let clip = &calculated
            .retained_scene()
            .unwrap()
            .clips()
            .iter()
            .find(|c| c.owner == identity && c.role == SceneBoxRole::Principal)
            .unwrap()
            .projections[0];
        assert_eq!(
            clip.border_radius_lengths[0],
            Some((
                openui_style::Length::calc_percent_px(25.0, 5.0),
                openui_style::Length::percent(25.0)
            ))
        );
        assert_eq!(
            e.computed_style(n)
                .unwrap()
                .border_radii_for_box((100.0, 80.0)),
            [(30.0, 20.0), (10.0, 16.0), (20.0, 12.0), (8.0, 8.0)]
        );
        for (p, v) in RADIUS_AXIS_PROPERTIES
            .into_iter()
            .zip([30.0, 20.0, 10.0, 16.0, 20.0, 12.0, 8.0, 8.0])
        {
            set(&mut e, n, p, LengthValue::px(v));
        }
        let fixed = e.scene().unwrap();
        assert_eq!(
            SoftwareCompositor::default().render(&fixed).unwrap().pixels,
            expected.pixels,
            "scale={scale}"
        );
        assert_eq!(
            [(1.0, 1.0), (50.0, 40.0), (98.0, 1.0)].map(|(x, y)| e.hit_test(x, y).unwrap()),
            hits
        );
        for (p, v) in RADIUS_AXIS_PROPERTIES.into_iter().zip(sources) {
            set(&mut e, n, p, v);
        }
        let root = e.root();
        set(&mut e, root, StyleProperty::FontSize, LengthValue::px(20.0));
        assert_eq!(
            e.radius_axis_length(n, StyleProperty::BorderTopRightRadiusY)
                .unwrap(),
            LengthValue::px(20.0)
        );
        e.set_viewport(ViewportMetrics::from_logical_size(240.0, 180.0, scale).unwrap())
            .unwrap();
        assert_eq!(
            e.radius_axis_length(n, StyleProperty::BorderBottomRightRadiusX)
                .unwrap(),
            LengthValue::px(24.0)
        );
        set(&mut e, n, StyleProperty::Width, LengthValue::px(200.0));
        set(&mut e, n, StyleProperty::Height, LengthValue::px(160.0));
        assert_eq!(
            e.computed_style(n)
                .unwrap()
                .border_radii_for_box((200.0, 160.0))[0],
            (55.0, 40.0)
        );
        assert_eq!(
            pair(&e, n),
            (
                LengthValue::calc_percent_px(25.0, 5.0),
                LengthValue::percent(25.0)
            )
        );
        let scene = e.scene().unwrap();
        let stats = e.stats();
        let generations = e.dirty_generations();
        for p in RADIUS_AXIS_PROPERTIES {
            e.radius_axis_length(n, p).unwrap();
        }
        assert_eq!(e.scene().unwrap().generation(), scene.generation());
        assert_eq!(e.stats(), stats);
        assert_eq!(e.dirty_generations(), generations);
        assert!(e.radius_axis_length(n, StyleProperty::Width).is_err());
        assert!(e
            .set_property(
                n,
                StyleProperty::BorderTopLeftRadiusX,
                LengthValue::Em(f32::MAX).into()
            )
            .is_err());
        assert_eq!(
            pair(&e, n),
            (
                LengthValue::calc_percent_px(25.0, 5.0),
                LengthValue::percent(25.0)
            )
        );
        assert_eq!(e.stats(), stats);
        assert_eq!(e.dirty_generations(), generations);
        e.remove_property(n, StyleProperty::BorderTopLeftRadiusX)
            .unwrap();
        assert_eq!(
            pair(&e, n),
            (LengthValue::px(0.0), LengthValue::percent(25.0))
        );
        assert_eq!(
            SoftwareCompositor::default()
                .render(&calculated)
                .unwrap()
                .pixels,
            expected.pixels
        );
    }
}
#[test]
fn circle_axis_and_legacy_overlay_effects_follow_ordered_current_underlying() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale);
        set(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::px(10.0),
        );
        set(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusY,
            LengthValue::px(20.0),
        );
        let circle = constant(
            &mut e,
            n,
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::px(5.0))),
            CompositeOperation::Add,
        );
        let x = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::px(3.0),
            CompositeOperation::Add,
        );
        assert_eq!(pair(&e, n), (LengthValue::px(18.0), LengthValue::px(25.0)));
        let overlay = constant(
            &mut e,
            n,
            StyleProperty::BorderRadiusPercent,
            StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent([(true, false); 4])),
            CompositeOperation::Replace,
        );
        assert_eq!(
            pair(&e, n),
            (LengthValue::percent(18.0), LengthValue::px(25.0))
        );
        let top = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::px(2.0),
            CompositeOperation::Accumulate,
        );
        assert_eq!(
            pair(&e, n),
            (
                LengthValue::calc_percent_px(18.0, 2.0),
                LengthValue::px(25.0)
            )
        );
        let legacy = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((7.0, 8.0))),
            CompositeOperation::Add,
        );
        // A calc lower axis projects the established false percent flag; the
        // compatibility pair is discrete Replace, not scalar additive.
        assert_eq!(pair(&e, n), (LengthValue::px(7.0), LengthValue::px(8.0)));
        e.cancel_animation(legacy).unwrap();
        assert_eq!(
            pair(&e, n),
            (
                LengthValue::calc_percent_px(18.0, 2.0),
                LengthValue::px(25.0)
            )
        );
        e.cancel_animation(top).unwrap();
        let legacy = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((7.0, 8.0))),
            CompositeOperation::Replace,
        );
        assert_eq!(
            pair(&e, n),
            (LengthValue::percent(7.0), LengthValue::px(8.0))
        );
        e.cancel_animation(legacy).unwrap();
        e.cancel_animation(overlay).unwrap();
        set(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::px(30.0),
        );
        assert_eq!(pair(&e, n), (LengthValue::px(38.0), LengthValue::px(25.0)));
        e.cancel_animation(x).unwrap();
        assert_eq!(pair(&e, n), (LengthValue::px(35.0), LengthValue::px(25.0)));
        e.cancel_animation(circle).unwrap();
        assert_eq!(pair(&e, n), (LengthValue::px(30.0), LengthValue::px(20.0)));
    }
}
#[test]
fn axis_removal_masks_old_corner_and_no_fill_reveals_live_lower_effect() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale);
        set(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((10.0, 20.0))),
        );
        set(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::px(3.0),
        );
        assert!(e
            .remove_property(n, StyleProperty::BorderTopLeftRadiusX)
            .unwrap());
        assert_eq!(pair(&e, n), (LengthValue::px(0.0), LengthValue::px(20.0)));
        let lower = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::px(5.0),
            CompositeOperation::Add,
        );
        let mut opts = options(CompositeOperation::Replace);
        opts.fill = FillMode::None;
        let upper = e
            .animate(
                n,
                PropertyKeyframes::typed(
                    StyleProperty::BorderTopLeftRadiusX,
                    Keyframes::from_values(LengthValue::px(30.0), LengthValue::px(30.0)),
                )
                .unwrap(),
                opts,
                AnimationTimeline::Document,
            )
            .unwrap();
        e.set_animation_time(50.0).unwrap();
        assert_eq!(pair(&e, n).0, LengthValue::px(30.0));
        e.set_animation_time(100.0).unwrap();
        assert_eq!(pair(&e, n).0, LengthValue::px(5.0));
        let scene = e.scene().unwrap();
        let stats = e.stats();
        let generations = e.dirty_generations();
        e.drain_animation_events();
        e.set_animation_time(100.0).unwrap();
        assert_eq!(e.scene().unwrap().generation(), scene.generation());
        assert_eq!(e.stats(), stats);
        assert_eq!(e.dirty_generations(), generations);
        assert!(e.drain_animation_events().is_empty());
        e.cancel_animation(upper).unwrap();
        e.cancel_animation(lower).unwrap();
        assert_eq!(pair(&e, n), (LengthValue::px(0.0), LengthValue::px(20.0)));
    }
}

#[test]
fn accepted_legacy_discrete_pairs_keep_raw_numbers_around_new_axis_effects() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale);
        set(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::calc_percent_px(25.0, 5.0),
        );
        set(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusY,
            LengthValue::px(10.0),
        );
        let old = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((-1.0, 8.0))),
            CompositeOperation::Add,
        );
        let style = e.computed_style(n).unwrap();
        assert_eq!(style.border_top_left_radius, (-1.0, 8.0));
        assert_eq!(style.border_radius_lengths[0], None);
        assert_eq!(pair(&e, n), (LengthValue::px(-1.0), LengthValue::px(8.0)));
        assert_eq!(style.border_radii_for_box((100.0, 80.0))[0], (0.0, 8.0));
        let x = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::px(2.0),
            CompositeOperation::Add,
        );
        assert_eq!(pair(&e, n), (LengthValue::px(1.0), LengthValue::px(8.0)));
        e.cancel_animation(x).unwrap();
        assert_eq!(
            e.computed_style(n).unwrap().border_top_left_radius,
            (-1.0, 8.0)
        );
        assert_eq!(e.computed_style(n).unwrap().border_radius_lengths[0], None);
        let y = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusY,
            LengthValue::px(20.0),
            CompositeOperation::Replace,
        );
        assert_eq!(pair(&e, n), (LengthValue::px(-1.0), LengthValue::px(20.0)));
        e.cancel_animation(y).unwrap();
        e.cancel_animation(old).unwrap();
        assert_eq!(
            pair(&e, n),
            (
                LengthValue::calc_percent_px(25.0, 5.0),
                LengthValue::px(10.0)
            )
        );
        let flags = constant(
            &mut e,
            n,
            StyleProperty::BorderRadiusPercent,
            StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent([(true, false); 4])),
            CompositeOperation::Replace,
        );
        let old = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((-1.0, 8.0))),
            CompositeOperation::Replace,
        );
        assert_eq!(
            e.computed_style(n).unwrap().border_top_left_radius,
            (-1.0, 8.0)
        );
        assert_eq!(
            pair(&e, n),
            (LengthValue::percent(-1.0), LengthValue::px(8.0))
        );
        let typed = constant(
            &mut e,
            n,
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::percent(0.0),
            CompositeOperation::Add,
        );
        assert_eq!(
            pair(&e, n),
            (LengthValue::percent(0.0), LengthValue::px(8.0))
        );
        e.cancel_animation(typed).unwrap();
        e.cancel_animation(old).unwrap();
        e.cancel_animation(flags).unwrap();
        assert_eq!(
            pair(&e, n),
            (
                LengthValue::calc_percent_px(25.0, 5.0),
                LengthValue::px(10.0)
            )
        );
    }
}

#[test]
fn legacy_pair_publication_keeps_lower_axis_kinds_and_intervening_overlay_order() {
    for scale in SCALES {
        for reverse_axes in [false, true] {
            for between in [None, Some((true, false)), Some((false, true))] {
                let (mut e, n) = fixture(scale);
                set(
                    &mut e,
                    n,
                    StyleProperty::BorderTopLeftRadiusX,
                    LengthValue::px(10.0),
                );
                set(
                    &mut e,
                    n,
                    StyleProperty::BorderTopLeftRadiusY,
                    LengthValue::px(20.0),
                );
                set(
                    &mut e,
                    n,
                    StyleProperty::BorderTopRightRadiusX,
                    LengthValue::px(12.0),
                );
                set(
                    &mut e,
                    n,
                    StyleProperty::BorderTopRightRadiusY,
                    LengthValue::percent(13.0),
                );
                let authored_unrelated = RADIUS_AXIS_PROPERTIES[2..]
                    .iter()
                    .map(|&property| e.radius_axis_length(n, property).unwrap())
                    .collect::<Vec<_>>();
                let mut flags = [(false, true); 4];
                flags[0] = (true, true);
                let overlay = constant(
                    &mut e,
                    n,
                    StyleProperty::BorderRadiusPercent,
                    StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent(flags)),
                    CompositeOperation::Replace,
                );
                let create_axis = |e: &mut Engine, axis| {
                    constant(
                        e,
                        n,
                        if axis == 0 {
                            StyleProperty::BorderTopLeftRadiusX
                        } else {
                            StyleProperty::BorderTopLeftRadiusY
                        },
                        LengthValue::px(if axis == 0 { 2.0 } else { 3.0 }),
                        CompositeOperation::Add,
                    )
                };
                let (x, y) = if reverse_axes {
                    let y = create_axis(&mut e, 1);
                    (create_axis(&mut e, 0), y)
                } else {
                    let x = create_axis(&mut e, 0);
                    (x, create_axis(&mut e, 1))
                };
                assert_eq!(
                    pair(&e, n),
                    (
                        LengthValue::calc_percent_px(10.0, 2.0),
                        LengthValue::calc_percent_px(20.0, 3.0),
                    )
                );
                let later_overlay = between.map(|between| {
                    flags[0] = between;
                    constant(
                        &mut e,
                        n,
                        StyleProperty::BorderRadiusPercent,
                        StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent(flags)),
                        CompositeOperation::Replace,
                    )
                });
                let unrelated = RADIUS_AXIS_PROPERTIES[2..]
                    .iter()
                    .map(|&property| e.radius_axis_length(n, property).unwrap())
                    .collect::<Vec<_>>();
                assert_eq!(unrelated[0], LengthValue::px(12.0));
                assert_eq!(unrelated[1], LengthValue::percent(13.0));
                let legacy = constant(
                    &mut e,
                    n,
                    StyleProperty::BorderTopLeftRadius,
                    StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((7.0, 8.0))),
                    CompositeOperation::Replace,
                );
                let pair_flags = between.unwrap_or((false, false));
                let expected = (
                    if pair_flags.0 {
                        LengthValue::percent(7.0)
                    } else {
                        LengthValue::px(7.0)
                    },
                    if pair_flags.1 {
                        LengthValue::percent(8.0)
                    } else {
                        LengthValue::px(8.0)
                    },
                );
                assert_eq!(pair(&e, n), expected);
                assert_eq!(e.computed_style(n).unwrap().border_radius_lengths[0], None);
                for (&property, &value) in RADIUS_AXIS_PROPERTIES[2..].iter().zip(&unrelated) {
                    assert_eq!(e.radius_axis_length(n, property).unwrap(), value);
                }
                // Removing current authored X changes the surviving lower Add,
                // while the discrete pair still owns its unchanged result.
                assert!(e
                    .remove_property(n, StyleProperty::BorderTopLeftRadiusX)
                    .unwrap());
                assert_eq!(pair(&e, n), expected);
                if let Some(later_overlay) = later_overlay {
                    e.cancel_animation(later_overlay).unwrap();
                    // The earlier overlay is followed by two real calc axes.
                    assert_eq!(pair(&e, n), (LengthValue::px(7.0), LengthValue::px(8.0)));
                }
                e.cancel_animation(legacy).unwrap();
                assert_eq!(
                    pair(&e, n),
                    (
                        LengthValue::calc_percent_px(0.0, 2.0),
                        LengthValue::calc_percent_px(20.0, 3.0),
                    )
                );
                e.cancel_animation(x).unwrap();
                assert_eq!(
                    pair(&e, n),
                    (
                        LengthValue::percent(0.0),
                        LengthValue::calc_percent_px(20.0, 3.0)
                    )
                );
                e.cancel_animation(y).unwrap();
                assert_eq!(
                    pair(&e, n),
                    (LengthValue::percent(0.0), LengthValue::percent(20.0))
                );
                e.cancel_animation(overlay).unwrap();
                assert_eq!(pair(&e, n), (LengthValue::px(0.0), LengthValue::px(20.0)));
                for (&property, &value) in
                    RADIUS_AXIS_PROPERTIES[2..].iter().zip(&authored_unrelated)
                {
                    assert_eq!(e.radius_axis_length(n, property).unwrap(), value);
                }
                let scene = e.scene().unwrap();
                let stats = e.stats();
                let generations = e.dirty_generations();
                e.drain_animation_events();
                e.set_animation_time(0.0).unwrap();
                assert_eq!(e.scene().unwrap().generation(), scene.generation());
                assert_eq!(e.stats(), stats);
                assert_eq!(e.dirty_generations(), generations);
                assert!(e.drain_animation_events().is_empty());
            }
        }
    }
}
