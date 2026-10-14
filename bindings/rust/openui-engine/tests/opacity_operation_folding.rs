//! Reduced native Engine guards for the pinned Chromium opacity operation rule.
//! The interior RGBA expectations were observed in all four required profiles.
//! Source admission is not a claim that these guards or the pixel gates passed.

use openui_compositor::{Frame, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_style::{
    Color, Display, LengthValue, Overflow, Position, RendererStyleValue, StyleProperty, StyleValue,
};

const PROFILES: [(f64, f64, f64); 4] = [
    (800.0, 600.0, 1.0),
    (375.0, 667.0, 2.0),
    (1280.0, 720.0, 1.25),
    (1920.0, 1080.0, 1.5),
];
const GREEN: Color = Color::GREEN;

fn engine(width: f64, height: f64, scale: f64) -> Engine {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(width, height, scale).unwrap()).unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::BackgroundColor,
            Color::WHITE.into(),
        )
        .unwrap();
    engine
}

fn box_node(
    engine: &mut Engine,
    parent: NodeHandle,
    x: f32,
    y: f32,
    width: f32,
    height: f32,
    color: Color,
) -> NodeHandle {
    let node = engine.create_native_element(ElementTag::Div).unwrap();
    engine.append_child(parent, node).unwrap();
    engine
        .set_property(node, StyleProperty::Display, Display::Block.into())
        .unwrap();
    engine
        .set_property(node, StyleProperty::Position, Position::Absolute.into())
        .unwrap();
    for (property, value) in [
        (StyleProperty::Left, x),
        (StyleProperty::Top, y),
        (StyleProperty::Width, width),
        (StyleProperty::Height, height),
    ] {
        engine
            .set_property(node, property, LengthValue::px(value).into())
            .unwrap();
    }
    engine
        .set_property(node, StyleProperty::BackgroundColor, color.into())
        .unwrap();
    node
}

fn opacity(engine: &mut Engine, node: NodeHandle, value: f32) {
    engine
        .set_property(node, StyleProperty::Opacity, StyleValue::Number(value))
        .unwrap();
}

fn rounded_clip(engine: &mut Engine, node: NodeHandle) {
    engine
        .set_property(node, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    for (property, value) in [
        (
            StyleProperty::BorderTopLeftRadius,
            RendererStyleValue::BorderTopLeftRadius((7.0, 7.0)),
        ),
        (
            StyleProperty::BorderTopRightRadius,
            RendererStyleValue::BorderTopRightRadius((7.0, 7.0)),
        ),
        (
            StyleProperty::BorderBottomRightRadius,
            RendererStyleValue::BorderBottomRightRadius((7.0, 7.0)),
        ),
        (
            StyleProperty::BorderBottomLeftRadius,
            RendererStyleValue::BorderBottomLeftRadius((7.0, 7.0)),
        ),
    ] {
        engine
            .set_property(node, property, StyleValue::Renderer(value))
            .unwrap();
    }
}

fn pixel(frame: &Frame, x: f32, y: f32) -> &[u8] {
    let scale = frame.viewport.device_scale_factor() as f32;
    let index = (y * scale).floor() as usize * frame.stride + (x * scale).floor() as usize * 4;
    &frame.pixels[index..index + 4]
}

fn render(engine: &mut Engine, compositor: &mut SoftwareCompositor) -> Frame {
    compositor.render(&engine.scene().unwrap()).unwrap()
}

#[test]
fn leaf_and_transparent_wrapper_use_the_same_single_draw_alpha_operation() {
    for (width, height, scale) in PROFILES {
        for wrapper in [false, true] {
            let mut engine = engine(width, height, scale);
            let root = engine.root();
            box_node(&mut engine, root, 20.0, 20.0, 100.0, 100.0, GREEN);
            let group = box_node(
                &mut engine,
                root,
                20.0,
                20.0,
                100.0,
                100.0,
                if wrapper {
                    Color::TRANSPARENT
                } else {
                    Color::RED
                },
            );
            if wrapper {
                box_node(&mut engine, group, 0.0, 0.0, 100.0, 100.0, Color::RED);
            }
            opacity(&mut engine, group, 0.1);
            let mut compositor = SoftwareCompositor::default();
            let frame = render(&mut engine, &mut compositor);
            for coordinate in [30.0, 60.0, 90.0] {
                assert_eq!(pixel(&frame, coordinate, coordinate), [26, 115, 0, 255]);
            }
            opacity(&mut engine, group, 0.2);
            assert_eq!(
                pixel(&render(&mut engine, &mut compositor), 60.0, 60.0),
                [51, 102, 0, 255]
            );
            opacity(&mut engine, group, 0.1);
            let scene = engine.scene().unwrap();
            let stats = engine.stats();
            let unchanged = engine.scene().unwrap();
            assert_eq!(
                engine.stats(),
                stats,
                "unchanged scene performed engine work"
            );
            drop(engine);
            let retained = compositor.render(&scene).unwrap();
            assert_eq!(retained.pixels, frame.pixels);
            assert_eq!(compositor.render(&unchanged).unwrap(), retained);
        }
    }
}

#[test]
fn overlapping_draws_and_text_keep_subtree_opacity_until_the_last_restore() {
    for (width, height, scale) in PROFILES {
        let mut engine = engine(width, height, scale);
        let root = engine.root();
        let group = box_node(&mut engine, root, 20.0, 20.0, 100.0, 100.0, Color::RED);
        opacity(&mut engine, group, 0.2);
        box_node(&mut engine, group, 30.0, 0.0, 70.0, 100.0, GREEN);
        let mut compositor = SoftwareCompositor::default();
        let frame = render(&mut engine, &mut compositor);
        assert_eq!(pixel(&frame, 30.0, 60.0), [255, 203, 203, 255]);
        assert_eq!(pixel(&frame, 60.0, 60.0), [203, 229, 203, 255]);
        engine.remove_children(group).unwrap();
        engine
            .set_property(group, StyleProperty::BackgroundColor, GREEN.into())
            .unwrap();
        let text = engine
            .create_text("overlapping glyphs remain a group")
            .unwrap();
        engine.append_child(group, text).unwrap();
        let frame = render(&mut engine, &mut compositor);
        assert_eq!(pixel(&frame, 90.0, 90.0), [203, 229, 203, 255]);
        engine.remove_children(group).unwrap();
        assert_eq!(
            pixel(&render(&mut engine, &mut compositor), 90.0, 90.0),
            [204, 230, 204, 255]
        );
    }
}

#[test]
fn clip_before_group_can_fold_but_clip_inside_group_retains_coverage() {
    for (width, height, scale) in PROFILES {
        let mut engine = engine(width, height, scale);
        let root = engine.root();
        let clip = box_node(&mut engine, root, 20.0, 20.0, 100.0, 100.0, GREEN);
        rounded_clip(&mut engine, clip);
        let child = box_node(&mut engine, clip, 0.0, 0.0, 100.0, 100.0, Color::RED);
        opacity(&mut engine, child, 0.2);
        let mut compositor = SoftwareCompositor::default();
        let frame = render(&mut engine, &mut compositor);
        assert_eq!(pixel(&frame, 60.0, 60.0), [51, 102, 0, 255]);
        assert_eq!(pixel(&frame, 15.0, 15.0), [255, 255, 255, 255]);
        engine.remove_children(root).unwrap();
        let group = box_node(
            &mut engine,
            root,
            20.0,
            20.0,
            100.0,
            100.0,
            Color::TRANSPARENT,
        );
        opacity(&mut engine, group, 0.2);
        rounded_clip(&mut engine, group);
        box_node(&mut engine, group, 0.0, 0.0, 120.0, 120.0, GREEN);
        let frame = render(&mut engine, &mut compositor);
        assert_eq!(pixel(&frame, 60.0, 60.0), [203, 229, 203, 255]);
        assert_eq!(pixel(&frame, 130.0, 130.0), [255, 255, 255, 255]);
    }
}

/// Native two-column reduction of the measured structural-clip ordering seam.
/// The inherited column clip is already the full visible fragment boundary.
#[test]
fn redundant_continuation_clip_preserves_single_draw_opacity_at_five_scales() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine = engine(320.0, 240.0, scale);
        let root = engine.root();
        let columns = box_node(&mut engine, root, 20.0, 20.0, 40.0, 40.0, GREEN);
        for (property, value) in [
            (
                StyleProperty::ColumnCount,
                RendererStyleValue::ColumnCount(Some(2)),
            ),
            (
                StyleProperty::ColumnFill,
                RendererStyleValue::ColumnFill(openui_style::ColumnFill::Auto),
            ),
            (
                StyleProperty::ColumnGap,
                RendererStyleValue::ColumnGap(Some(openui_geometry::Length::px(0.0))),
            ),
        ] {
            engine
                .set_property(columns, property, StyleValue::Renderer(value))
                .unwrap();
        }
        let group = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(columns, group).unwrap();
        engine
            .set_property(group, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(group, StyleProperty::Position, Position::Relative.into())
            .unwrap();
        opacity(&mut engine, group, 0.1);
        let positioned = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(group, positioned).unwrap();
        engine
            .set_property(positioned, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(
                positioned,
                StyleProperty::Position,
                Position::Absolute.into(),
            )
            .unwrap();
        engine
            .set_property(
                positioned,
                StyleProperty::Width,
                LengthValue::percent(100.0).into(),
            )
            .unwrap();
        let fill = engine.create_native_element(ElementTag::Div).unwrap();
        engine.append_child(positioned, fill).unwrap();
        engine
            .set_property(fill, StyleProperty::Display, Display::Block.into())
            .unwrap();
        engine
            .set_property(fill, StyleProperty::Height, LengthValue::px(80.0).into())
            .unwrap();
        engine
            .set_property(fill, StyleProperty::BackgroundColor, Color::RED.into())
            .unwrap();
        let scene = engine.scene().unwrap();
        let stats = engine.stats();
        assert_eq!(engine.scene().unwrap().generation(), scene.generation());
        assert_eq!(
            engine.stats(),
            stats,
            "unchanged scene performed engine work"
        );
        drop(engine);
        let mut compositor = SoftwareCompositor::default();
        let frame = compositor.render(&scene).unwrap();
        for y in 0..frame.height {
            for x in 0..frame.width {
                let inside = x >= (20.0 * scale) as u32
                    && x < (60.0 * scale) as u32
                    && y >= (20.0 * scale) as u32
                    && y < (60.0 * scale) as u32;
                let expected = if inside {
                    [26, 115, 0, 255]
                } else {
                    [255, 255, 255, 255]
                };
                let offset = y as usize * frame.stride + x as usize * 4;
                assert_eq!(
                    &frame.pixels[offset..offset + 4],
                    expected,
                    "scale={scale} x={x} y={y}"
                );
            }
        }
        assert_eq!(compositor.render(&scene).unwrap(), frame);
    }
}
