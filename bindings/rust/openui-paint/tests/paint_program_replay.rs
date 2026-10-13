use openui_dom::{Document, ElementTag};
use openui_geometry::{LayoutUnit, ViewportMetrics};
use openui_layout::{block_layout, ConstraintSpace};
use openui_paint::{
    rasterize_picture, record_fragment, record_fragment_with_program, PaintCaptureStatus,
};
use openui_style::{Color, Overflow, StyleProperty};

fn pixels(picture: &openui_paint::RecordedPicture) -> Vec<u8> {
    let mut surface = rasterize_picture(picture).unwrap();
    let image = surface.image_snapshot();
    let info = image
        .image_info()
        .with_color_type(skia_safe::ColorType::RGBA8888);
    let mut bytes = vec![0; info.compute_min_byte_size()];
    assert!(image.read_pixels(
        &info,
        &mut bytes,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Allow
    ));
    bytes
}

#[test]
fn same_phase_capture_preserves_cpu_tiles_viewport_controls_and_document_drop() {
    // The wide content exercises the existing tile admission; the smaller
    // document exercises direct recording. Both use the real layout/paint walk.
    for (width, scrolling) in [(120.0, false), (720.0, true)] {
        for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
            let mut doc = Document::new();
            let root = doc.root();
            doc.apply_style_property(
                root,
                StyleProperty::BackgroundColor,
                &Color::WHITE.into(),
                (width, 72.0),
            )
            .unwrap();
            if scrolling {
                doc.apply_style_property(
                    root,
                    StyleProperty::OverflowY,
                    &Overflow::Scroll.into(),
                    (width, 72.0),
                )
                .unwrap();
            }
            let box_ = doc.create_node(ElementTag::Div);
            let box_style = doc.node(box_).style.derive(|style| {
                style.width = openui_geometry::Length::px(width as f32);
                style.height = openui_geometry::Length::px(96.0);
                style.background_color = Color::GREEN;
                style.opacity = 0.2;
            });
            doc.node_mut(box_).style = box_style;
            doc.append_child(root, box_);
            let text = doc.create_node(ElementTag::Text);
            doc.node_mut(text).text = Some("native retained text".into());
            doc.append_child(box_, text);
            let viewport = ViewportMetrics::from_logical_size(width, 72.0, scale).unwrap();
            let space =
                ConstraintSpace::for_root(LayoutUnit::from_f64(width), LayoutUnit::from_f64(72.0));
            let fragment = block_layout(&doc, root, &space);
            let before = record_fragment(&doc, &fragment, viewport).unwrap();
            let captured = record_fragment_with_program(&doc, &fragment, viewport).unwrap();
            let after = record_fragment(&doc, &fragment, viewport).unwrap();
            assert_eq!(
                captured.content_program.status(),
                PaintCaptureStatus::Complete
            );
            assert!(!captured.content_program.authored_layers_are_partitioned());
            let expected = pixels(&before);
            assert_eq!(pixels(&captured.canonical), expected);
            assert_eq!(pixels(&after), expected);
            drop(fragment);
            drop(doc);
            let replayed = captured.recording_from_program().unwrap();
            assert_eq!(pixels(&replayed), expected, "width {width}, scale {scale}");
            let replayed_again = captured.recording_from_program().unwrap();
            assert_eq!(pixels(&replayed_again), expected);
            if scrolling {
                assert!(captured.viewport_overlay_program.is_some());
            } else {
                // Retain only the program, then release every canonical/template
                // picture before replay. Its cache lease must remain independent.
                let program = captured.content_program.clone();
                let configuration = captured.canonical.raster_configuration;
                let lcd = captured.canonical.uses_lcd_surface();
                drop(replayed_again);
                drop(replayed);
                drop(captured);
                drop(after);
                drop(before);
                assert!(skia_safe::graphics::font_cache_count_used() > 0);
                let info = skia_safe::ImageInfo::new_n32_premul(
                    (
                        viewport.physical_width() as i32,
                        viewport.physical_height() as i32,
                    ),
                    Some(skia_safe::ColorSpace::new_srgb()),
                );
                let props = openui_paint::raster_surface_properties(configuration, lcd);
                let mut surface = skia_safe::surfaces::raster(&info, None, props.as_ref()).unwrap();
                surface.canvas().scale((scale as f32, scale as f32));
                program.replay(surface.canvas()).unwrap();
                let image = surface.image_snapshot();
                let rgba = image
                    .image_info()
                    .with_color_type(skia_safe::ColorType::RGBA8888);
                let mut actual = vec![0; rgba.compute_min_byte_size()];
                assert!(image.read_pixels(
                    &rgba,
                    &mut actual,
                    rgba.min_row_bytes(),
                    (0, 0),
                    skia_safe::image::CachingHint::Allow
                ));
                assert_eq!(actual, expected, "standalone program scale {scale}");
                drop(program);
                assert_eq!(skia_safe::graphics::font_cache_count_used(), 0);
            }
        }
    }
}
