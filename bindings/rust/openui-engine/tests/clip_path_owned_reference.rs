//! Native inset clips use the owned border box and the calculated-length route.

use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, ViewportMetrics};
use openui_geometry::Length;
use openui_style::{CaptionSide, Color, Display, LengthValue, RendererStyleValue, StyleProperty};

fn engine(scale: f64) -> Engine {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(160.0, 140.0, scale).unwrap()).unwrap();
    engine
        .set_property(
            engine.root(),
            StyleProperty::BackgroundColor,
            Color::WHITE.into(),
        )
        .unwrap();
    engine
}

fn node(
    engine: &mut Engine,
    parent: NodeHandle,
    tag: ElementTag,
    display: Display,
    width: f32,
    height: f32,
) -> NodeHandle {
    let node = engine.create_native_element(tag).unwrap();
    engine
        .set_property(node, StyleProperty::Display, display.into())
        .unwrap();
    for (property, value) in [
        (StyleProperty::Width, width),
        (StyleProperty::Height, height),
    ] {
        engine
            .set_property(node, property, LengthValue::px(value).into())
            .unwrap();
    }
    engine.append_child(parent, node).unwrap();
    node
}

fn inset(engine: &mut Engine, node: NodeHandle, lengths: Option<[Length; 4]>) {
    engine
        .set_renderer_style(node, RendererStyleValue::ClipPathInset(lengths))
        .unwrap();
}

#[test]
fn calculated_inset_matches_its_pinned_float_value_and_retained_scene_survives_mutation() {
    // CalculationValue evaluates pixels + percent / 100 * basis. This value
    // differs by one float bit from percent * basis / 100 + pixels.
    let expected = f32::from_bits(1_075_629_260);
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine = engine(scale);
        let root = engine.root();
        let target = node(&mut engine, root, ElementTag::Div, Display::Block, 5.0, 8.0);
        engine
            .set_property(target, StyleProperty::BackgroundColor, Color::GREEN.into())
            .unwrap();
        inset(
            &mut engine,
            target,
            Some([
                Length::px(0.0),
                Length::px(0.0),
                Length::px(0.0),
                Length::calc_percent_px(29.0, 1.0),
            ]),
        );
        let calculated = engine.scene().unwrap();
        let before = engine.stats();
        inset(
            &mut engine,
            target,
            Some([
                Length::px(0.0),
                Length::px(0.0),
                Length::px(0.0),
                Length::px(expected),
            ]),
        );
        let fixed = engine.scene().unwrap();
        assert_eq!(engine.stats().layouts, before.layouts);
        let mut compositor = SoftwareCompositor::default();
        let calculated_frame = compositor.render(&calculated).unwrap();
        assert_eq!(
            calculated_frame.pixels,
            compositor.render(&fixed).unwrap().pixels,
            "scale={scale}"
        );
        inset(&mut engine, target, None);
        let unclipped = engine.scene().unwrap();
        assert_ne!(
            calculated_frame.pixels,
            compositor.render(&unclipped).unwrap().pixels
        );
        drop(engine);
        assert_eq!(
            calculated_frame.pixels,
            compositor.render(&calculated).unwrap().pixels
        );
    }
}

#[test]
fn captioned_table_inset_uses_the_authored_wrapper_with_top_bottom_and_removed_captions() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut engine = engine(scale);
        let root = engine.root();
        let table = node(
            &mut engine,
            root,
            ElementTag::Table,
            Display::Table,
            100.0,
            80.0,
        );
        engine
            .set_renderer_style(
                table,
                RendererStyleValue::BorderSpacing((Length::px(0.0), Length::px(0.0))),
            )
            .unwrap();
        let caption = node(
            &mut engine,
            table,
            ElementTag::TableCaption,
            Display::TableCaption,
            100.0,
            20.0,
        );
        engine
            .set_property(caption, StyleProperty::BackgroundColor, Color::RED.into())
            .unwrap();
        let row = node(
            &mut engine,
            table,
            ElementTag::TableRow,
            Display::TableRow,
            100.0,
            80.0,
        );
        let cell = node(
            &mut engine,
            row,
            ElementTag::TableCell,
            Display::TableCell,
            100.0,
            80.0,
        );
        engine
            .set_property(cell, StyleProperty::BackgroundColor, Color::GREEN.into())
            .unwrap();
        inset(
            &mut engine,
            table,
            Some([
                Length::percent(25.0),
                Length::px(0.0),
                Length::px(0.0),
                Length::px(0.0),
            ]),
        );
        let clipped = engine.scene().unwrap();
        let reference = engine.client_rects(table).unwrap();
        assert_eq!(reference.len(), 1);
        // Pinned Chromium147.0.7727.50 reports the authored table wrapper,
        // including the caption. Its inset reference has the same 100px height.
        assert_eq!(reference[0].y, 0.0);
        assert_eq!(reference[0].height, 100.0);
        let frame = SoftwareCompositor::default().render(&clipped).unwrap();
        let pixel = |pixels: &[u8], width: u32, y: f64| -> [u8; 4] {
            let x = (10.0 * scale).floor() as usize;
            let y = (y * scale).floor() as usize;
            pixels[(y * width as usize + x) * 4..][..4]
                .try_into()
                .unwrap()
        };
        assert_eq!(
            pixel(&frame.pixels, frame.width, 10.0),
            [255, 255, 255, 255],
            "caption must be clipped, scale={scale}"
        );
        assert_eq!(
            pixel(&frame.pixels, frame.width, 30.0),
            [0, 128, 0, 255],
            "inset begins at 25% of the wrapper height, scale={scale}"
        );
        assert_eq!(pixel(&frame.pixels, frame.width, 60.0), [0, 128, 0, 255]);

        engine
            .set_renderer_style(
                caption,
                RendererStyleValue::CaptionSide(CaptionSide::Bottom),
            )
            .unwrap();
        let bottom = engine.scene().unwrap();
        assert_eq!(engine.client_rects(table).unwrap(), reference);
        assert_eq!(engine.client_rects(caption).unwrap()[0].y, 80.0);
        assert_eq!(engine.client_rects(cell).unwrap()[0].y, 0.0);
        let bottom_frame = SoftwareCompositor::default().render(&bottom).unwrap();
        assert_eq!(
            pixel(&bottom_frame.pixels, bottom_frame.width, 10.0),
            [255; 4]
        );
        assert_eq!(
            pixel(&bottom_frame.pixels, bottom_frame.width, 30.0),
            [0, 128, 0, 255]
        );
        assert_eq!(
            pixel(&bottom_frame.pixels, bottom_frame.width, 90.0),
            [255, 0, 0, 255]
        );

        engine.detach(caption).unwrap();
        let detached = engine.scene().unwrap();
        assert!(engine.client_rects(caption).unwrap().is_empty());
        assert_eq!(engine.client_rects(table).unwrap()[0].height, 80.0);
        let detached_frame = SoftwareCompositor::default().render(&detached).unwrap();
        assert_eq!(
            pixel(&detached_frame.pixels, detached_frame.width, 10.0),
            [255; 4]
        );
        assert_eq!(
            pixel(&detached_frame.pixels, detached_frame.width, 30.0),
            [0, 128, 0, 255]
        );
        assert_eq!(
            pixel(&detached_frame.pixels, detached_frame.width, 90.0),
            [255; 4]
        );
        inset(&mut engine, table, None);
        let restored = engine.scene().unwrap();
        let restored_frame = SoftwareCompositor::default().render(&restored).unwrap();
        assert_ne!(detached_frame.pixels, restored_frame.pixels);
        let stats = engine.stats();
        assert_eq!(engine.scene().unwrap().generation(), restored.generation());
        assert_eq!(engine.stats(), stats);
        drop(engine);
        assert_eq!(
            frame.pixels,
            SoftwareCompositor::default()
                .render(&clipped)
                .unwrap()
                .pixels
        );
        assert_eq!(
            bottom_frame.pixels,
            SoftwareCompositor::default()
                .render(&bottom)
                .unwrap()
                .pixels
        );
    }
}
