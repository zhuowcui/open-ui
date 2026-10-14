//! Source-owned overflow-control frame clips through the public native Engine.
//! These guards are authored UNRUN; Chromium pixels remain the qualification oracle.

use openui_compositor::{Frame, SceneNodeIdentity, SceneSnapshot, SoftwareCompositor};
use openui_dom::{ElementTag, FormControlRole};
use openui_engine::{Engine, NodeHandle, RendererNodeState, ViewportMetrics};
use openui_geometry::LayoutUnit;
use openui_style::*;

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn set(e: &mut Engine, n: NodeHandle, value: RendererStyleValue) {
    e.set_property(n, value.property(), StyleValue::Renderer(value))
        .unwrap();
}

fn resize(e: &mut Engine, n: NodeHandle, value: Resize) {
    set(e, n, RendererStyleValue::Resize(value));
}

fn fixture(scale: f64, width: f32, height: f32) -> (Engine, NodeHandle) {
    let mut e = Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(160.0, 160.0, scale).unwrap(),
        openui_text::FontCollection::deterministic_test(),
    )
    .unwrap();
    let root = e.root();
    set(
        &mut e,
        root,
        RendererStyleValue::BackgroundColor(Color::WHITE),
    );
    let n = e.create_native_element(ElementTag::TextArea).unwrap();
    e.append_child(root, n).unwrap();
    set(&mut e, n, RendererStyleValue::Display(Display::Block));
    e.set_property(n, StyleProperty::Position, Position::Absolute.into())
        .unwrap();
    for p in [StyleProperty::Left, StyleProperty::Top] {
        e.set_property(n, p, LengthValue::px(32.0).into()).unwrap();
    }
    e.set_property(
        n,
        StyleProperty::Margin,
        Edges::all(LengthValue::px(0.0)).into(),
    )
    .unwrap();
    e.set_property(
        n,
        StyleProperty::Padding,
        Edges::all(LengthValue::px(0.0)).into(),
    )
    .unwrap();
    e.set_property(
        n,
        StyleProperty::Border,
        Border {
            width: 0.0,
            style: BorderStyle::None,
            color: Color::BLACK,
        }
        .into(),
    )
    .unwrap();
    set(
        &mut e,
        n,
        RendererStyleValue::BoxSizing(BoxSizing::BorderBox),
    );
    set(&mut e, n, RendererStyleValue::Width(Length::px(width)));
    set(&mut e, n, RendererStyleValue::Height(Length::px(height)));
    set(&mut e, n, RendererStyleValue::BackgroundColor(Color::WHITE));
    set(
        &mut e,
        n,
        RendererStyleValue::FontFamily(FontFamilyList::single("Ahem")),
    );
    set(&mut e, n, RendererStyleValue::FontSize(16.0));
    set(
        &mut e,
        n,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    );
    set(&mut e, n, RendererStyleValue::OverflowX(Overflow::Hidden));
    set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Hidden));
    resize(&mut e, n, Resize::Both);
    (e, n)
}

fn geometry(scene: &SceneSnapshot, n: NodeHandle) -> (f32, f32, f32, f32) {
    let identity = SceneNodeIdentity {
        document: n.document_id(),
        index: n.index(),
        generation: n.generation(),
    };
    let layer = scene
        .retained_scene()
        .unwrap()
        .layers()
        .iter()
        .find(|layer| layer.owner == identity)
        .unwrap();
    assert_eq!(
        layer.projections.len(),
        1,
        "native producer must be monolithic"
    );
    let projection = &layer.projections[0];
    (
        projection.offset.left.to_f32(),
        projection.offset.top.to_f32(),
        projection.border_box.width().to_f32(),
        projection.border_box.height().to_f32(),
    )
}

fn frame(e: &mut Engine) -> Frame {
    SoftwareCompositor::default()
        .render(&e.scene().unwrap())
        .unwrap()
}

fn changed_pixels_inside(a: &Frame, b: &Frame, border: (f32, f32, f32, f32)) -> usize {
    assert_eq!((a.width, a.height, a.stride), (b.width, b.height, b.stride));
    let scale = a.viewport.device_scale_factor() as f32;
    let (left, top, width, height) = border;
    let mut changed = 0;
    for y in 0..a.height as usize {
        for x in 0..a.width as usize {
            let i = y * a.stride + x * 4;
            if a.pixels[i..i + 4] != b.pixels[i..i + 4] {
                changed += 1;
                let cx = x as f32 + 0.5;
                let cy = y as f32 + 0.5;
                assert!(
                    cx >= left * scale && cx < (left + width) * scale,
                    "control ink outside physical border x={x}, scale={scale}"
                );
                assert!(
                    cy >= top * scale && cy < (top + height) * scale,
                    "control ink outside physical border y={y}, scale={scale}"
                );
            }
        }
    }
    changed
}

#[test]
fn tiny_actual_native_boxes_use_computed_resize_admission_and_strict_border_clips() {
    for scale in SCALES {
        for width in [0.0, 1.0, 14.0, 15.0, 16.0] {
            for height in [0.0, 1.0, 14.0, 15.0, 16.0] {
                let (mut e, n) = fixture(scale, width, height);
                for overflow in [
                    Overflow::Auto,
                    Overflow::Hidden,
                    Overflow::Clip,
                    Overflow::Visible,
                ] {
                    set(&mut e, n, RendererStyleValue::OverflowX(overflow));
                    set(&mut e, n, RendererStyleValue::OverflowY(overflow));
                    resize(&mut e, n, Resize::None);
                    let absent = frame(&mut e);
                    resize(&mut e, n, Resize::Both);
                    let scene = e.scene().unwrap();
                    assert_eq!(geometry(&scene, n), (32.0, 32.0, width, height));
                    let present = SoftwareCompositor::default().render(&scene).unwrap();
                    let changed =
                        changed_pixels_inside(&absent, &present, (32.0, 32.0, width, height));
                    // The native style adjustment normalizes Visible to
                    // Auto before CanResize observes the computed style.
                    let normalized = if overflow == Overflow::Visible {
                        Overflow::Auto
                    } else {
                        overflow
                    };
                    let eligible = matches!(normalized, Overflow::Auto | Overflow::Hidden);
                    assert_eq!(e.computed_style(n).unwrap().overflow_x, normalized);
                    assert_eq!(e.computed_style(n).unwrap().is_scroll_container(), eligible);
                    if !eligible || width == 0.0 || height == 0.0 {
                        assert_eq!(changed, 0, "scale={scale}, {width}x{height}, {overflow:?}");
                    } else if width >= 14.0 && height >= 14.0 {
                        assert!(
                            changed > 0,
                            "resize has no 15px fit cutoff: scale={scale}, {width}x{height}"
                        );
                    }
                }
                // The normalized mixed pair remains a scroll container.
                set(&mut e, n, RendererStyleValue::OverflowX(Overflow::Visible));
                set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Hidden));
                assert!(e.computed_style(n).unwrap().is_scroll_container());
                let s = e.scene().unwrap();
                let stats = e.stats();
                let dirty = e.dirty_generations();
                assert_eq!(e.scene().unwrap().generation(), s.generation());
                assert_eq!(e.stats(), stats);
                assert_eq!(e.dirty_generations(), dirty);
            }
        }
    }
}

#[test]
fn full_border_scope_is_independent_of_padding_client_radius_direction_and_writing_mode() {
    for scale in SCALES {
        for writing in [
            WritingMode::HorizontalTb,
            WritingMode::VerticalRl,
            WritingMode::VerticalLr,
        ] {
            for direction in [Direction::Ltr, Direction::Rtl] {
                let (mut e, n) = fixture(scale, 14.0, 14.0);
                set(&mut e, n, RendererStyleValue::WritingMode(writing));
                set(&mut e, n, RendererStyleValue::Direction(direction));
                for decorated in [false, true] {
                    let borders = if decorated { [2, 1, 3, 4] } else { [0; 4] };
                    for value in [
                        RendererStyleValue::BorderTopWidth(borders[0]),
                        RendererStyleValue::BorderRightWidth(borders[1]),
                        RendererStyleValue::BorderBottomWidth(borders[2]),
                        RendererStyleValue::BorderLeftWidth(borders[3]),
                        RendererStyleValue::BorderTopStyle(BorderStyle::Solid),
                        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
                        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
                        RendererStyleValue::BorderLeftStyle(BorderStyle::Solid),
                    ] {
                        set(&mut e, n, value);
                    }
                    e.set_property(
                        n,
                        StyleProperty::Padding,
                        Edges::all(LengthValue::px(if decorated { 2.0 } else { 0.0 })).into(),
                    )
                    .unwrap();
                    e.set_property(
                        n,
                        StyleProperty::BorderRadius,
                        CornerRadii(Edges::all(LengthValue::px(if decorated {
                            6.0
                        } else {
                            0.0
                        })))
                        .into(),
                    )
                    .unwrap();
                    resize(&mut e, n, Resize::None);
                    let absent = frame(&mut e);
                    resize(&mut e, n, Resize::Both);
                    let s = e.scene().unwrap();
                    let border = geometry(&s, n);
                    let present = SoftwareCompositor::default().render(&s).unwrap();
                    assert!(changed_pixels_inside(&absent, &present, border) > 0);
                    assert_eq!(e.computed_style(n).unwrap().writing_mode, writing);
                    assert_eq!(e.computed_style(n).unwrap().direction, direction);
                }
            }
        }
    }
}

#[test]
fn fractional_physical_placement_clips_in_current_paint_space() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 14.25, 14.25);
        e.set_property(n, StyleProperty::Left, LengthValue::px(32.25).into())
            .unwrap();
        e.set_property(n, StyleProperty::Top, LengthValue::px(32.75).into())
            .unwrap();
        resize(&mut e, n, Resize::None);
        let absent = frame(&mut e);
        resize(&mut e, n, Resize::Both);
        let s = e.scene().unwrap();
        assert_eq!(geometry(&s, n), (32.25, 32.75, 14.25, 14.25));
        let present = SoftwareCompositor::default().render(&s).unwrap();
        assert!(changed_pixels_inside(&absent, &present, (32.0, 33.0, 15.0, 14.0)) > 0);
    }
}

#[test]
fn current_font_attributes_and_capture_context_change_layout_without_owning_theme_frames() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 14.0, 14.0);
        e.remove_property(n, StyleProperty::Width).unwrap();
        e.remove_property(n, StyleProperty::Height).unwrap();
        // Snapshot-hidden presentation keeps capture A's metric source
        // separate from an authored scrollbar-width contribution.
        e.remove_property(n, StyleProperty::ScrollbarWidth).unwrap();
        e.set_renderer_style(n, RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None))
            .unwrap();
        set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Auto));
        let mut previous = None;
        for (font, size, cols, rows) in [("Ahem", 16.0, "2", "1"), ("DejaVu Sans", 20.0, "7", "3")]
        {
            set(
                &mut e,
                n,
                RendererStyleValue::FontFamily(FontFamilyList::single(font)),
            );
            set(&mut e, n, RendererStyleValue::FontSize(size));
            set(
                &mut e,
                n,
                RendererStyleValue::LineHeight(LineHeight::Length(size + 4.0)),
            );
            e.set_attribute(n, "cols", cols).unwrap();
            e.set_attribute(n, "rows", rows).unwrap();
            let mut captured_widths = Vec::new();
            for capture in [
                ScrollbarWidth::Auto,
                ScrollbarWidth::Thin,
                ScrollbarWidth::None,
            ] {
                e.request_native_control_geometry(n, FormControlRole::TextArea, capture)
                    .unwrap();
                let s = e.scene().unwrap();
                let border = geometry(&s, n);
                captured_widths.push(border.2);
                assert_eq!(
                    e.computed_style(n).unwrap().scrollbar_width,
                    ScrollbarWidth::None
                );
                let current = SoftwareCompositor::default().render(&s).unwrap();
                // Compare with an ordinary, explicitly sized native control:
                // A's capture context changes sizing, never the resizer frame.
                let (mut fixed, fixed_n) = fixture(scale, border.2, border.3);
                set(
                    &mut fixed,
                    fixed_n,
                    RendererStyleValue::FontFamily(FontFamilyList::single(font)),
                );
                set(&mut fixed, fixed_n, RendererStyleValue::FontSize(size));
                set(
                    &mut fixed,
                    fixed_n,
                    RendererStyleValue::LineHeight(LineHeight::Length(size + 4.0)),
                );
                assert_eq!(frame(&mut fixed).pixels, current.pixels);
                if capture == ScrollbarWidth::Auto {
                    if let Some(old) = previous {
                        assert_ne!((border.2, border.3), old);
                    }
                    previous = Some((border.2, border.3));
                }
                resize(&mut e, n, Resize::None);
                let absent = frame(&mut e);
                resize(&mut e, n, Resize::Both);
                assert!(changed_pixels_inside(&absent, &current, border) > 0);
            }
            assert_eq!(captured_widths[0] - captured_widths[1], 5.0);
            assert_eq!(captured_widths[0] - captured_widths[2], 15.0);
        }
        set(&mut e, n, RendererStyleValue::Width(Length::px(14.0)));
        set(&mut e, n, RendererStyleValue::Height(Length::px(14.0)));
        set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Hidden));
        let before = frame(&mut e);
        e.set_renderer_node_state(n, RendererNodeState::Replaced(None))
            .unwrap();
        // Explicit metadata removal uses the actual ordinary-box producer;
        // a tag does not prove native intrinsic or continuation ownership.
        let s = e.scene().unwrap();
        let border = geometry(&s, n);
        assert_ne!(border.2, 0.0);
        resize(&mut e, n, Resize::None);
        let absent = frame(&mut e);
        resize(&mut e, n, Resize::Both);
        let ordinary = frame(&mut e);
        assert!(changed_pixels_inside(&absent, &ordinary, border) > 0);
        assert_eq!(before.pixels, ordinary.pixels);
        // With explicit metadata ownership the native Visible-to-Auto style
        // adjustment no longer runs. Computed Visible rejects resizing.
        set(&mut e, n, RendererStyleValue::OverflowX(Overflow::Visible));
        set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Visible));
        assert!(!e.computed_style(n).unwrap().is_scroll_container());
        let visible = frame(&mut e);
        resize(&mut e, n, Resize::None);
        assert_eq!(frame(&mut e).pixels, visible.pixels);
    }
}

#[test]
fn actual_native_scrollbar_frames_are_clipped_independently_of_track_and_thumb_ink() {
    for scale in SCALES {
        for width in [0.0, 1.0, 14.0, 15.0, 16.0] {
            let (mut e, n) = fixture(scale, width, 14.0);
            e.set_renderer_node_state(n, RendererNodeState::Replaced(None))
                .unwrap();
            resize(&mut e, n, Resize::None);
            for style in [ScrollbarWidth::Auto, ScrollbarWidth::Thin] {
                set(
                    &mut e,
                    n,
                    RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
                );
                set(&mut e, n, RendererStyleValue::OverflowX(Overflow::Hidden));
                set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Hidden));
                let absent = frame(&mut e);
                set(&mut e, n, RendererStyleValue::ScrollbarWidth(style));
                set(&mut e, n, RendererStyleValue::OverflowX(Overflow::Scroll));
                set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Scroll));
                let scene = e.scene().unwrap();
                let border = geometry(&scene, n);
                assert_eq!(border, (32.0, 32.0, width, 14.0));
                let identity = SceneNodeIdentity {
                    document: n.document_id(),
                    index: n.index(),
                    generation: n.generation(),
                };
                let projection = &scene
                    .retained_scene()
                    .unwrap()
                    .layers()
                    .iter()
                    .find(|layer| layer.owner == identity)
                    .unwrap()
                    .projections[0];
                let mut fragment = scene.fragments().as_ref();
                for index in projection.fragment_path.iter() {
                    fragment = &fragment.children[*index];
                }
                assert!(fragment.owns_scroll_box());
                let bars = fragment.element_scrollbars.unwrap();
                assert!(bars.horizontal && bars.vertical);
                assert_eq!(
                    bars.thickness.to_i32(),
                    if style == ScrollbarWidth::Auto {
                        15
                    } else {
                        10
                    }
                );
                let present = SoftwareCompositor::default().render(&scene).unwrap();
                let changed = changed_pixels_inside(&absent, &present, border);
                if width == 0.0 {
                    assert_eq!(changed, 0);
                } else {
                    assert!(changed > 0);
                }
            }
        }
    }
}

#[test]
fn ancestor_transform_clip_scroll_and_sibling_ink_preserve_control_source_space() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 14.0, 14.0);
        let root = e.root();
        let parent = e.create_native_element(ElementTag::Div).unwrap();
        e.append_child(root, parent).unwrap();
        e.append_or_move_child(parent, n).unwrap();
        set(&mut e, parent, RendererStyleValue::Display(Display::Block));
        e.set_property(parent, StyleProperty::Position, Position::Absolute.into())
            .unwrap();
        for p in [StyleProperty::Left, StyleProperty::Top] {
            e.set_property(parent, p, LengthValue::px(24.0).into())
                .unwrap();
        }
        set(&mut e, parent, RendererStyleValue::Width(Length::px(80.0)));
        set(&mut e, parent, RendererStyleValue::Height(Length::px(80.0)));
        set(
            &mut e,
            parent,
            RendererStyleValue::OverflowX(Overflow::Hidden),
        );
        set(
            &mut e,
            parent,
            RendererStyleValue::OverflowY(Overflow::Hidden),
        );
        set(
            &mut e,
            parent,
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        );
        set(
            &mut e,
            parent,
            RendererStyleValue::Transform(Transform2D {
                e: 8.0,
                f: 6.0,
                ..Transform2D::IDENTITY
            }),
        );
        for p in [StyleProperty::Left, StyleProperty::Top] {
            e.set_property(n, p, LengthValue::px(56.0).into()).unwrap();
        }
        let spacer = e.create_native_element(ElementTag::Div).unwrap();
        e.append_child(parent, spacer).unwrap();
        set(&mut e, spacer, RendererStyleValue::Width(Length::px(160.0)));
        set(
            &mut e,
            spacer,
            RendererStyleValue::Height(Length::px(160.0)),
        );
        let sibling = e.create_native_element(ElementTag::Div).unwrap();
        e.append_child(root, sibling).unwrap();
        e.set_property(sibling, StyleProperty::Position, Position::Absolute.into())
            .unwrap();
        e.set_property(sibling, StyleProperty::Left, LengthValue::px(130.0).into())
            .unwrap();
        e.set_property(sibling, StyleProperty::Top, LengthValue::px(130.0).into())
            .unwrap();
        set(&mut e, sibling, RendererStyleValue::Width(Length::px(12.0)));
        set(
            &mut e,
            sibling,
            RendererStyleValue::Height(Length::px(12.0)),
        );
        set(
            &mut e,
            sibling,
            RendererStyleValue::BackgroundColor(Color::GREEN),
        );
        e.scroll_to(parent, 40.0, 40.0).unwrap();
        resize(&mut e, n, Resize::None);
        let absent = frame(&mut e);
        resize(&mut e, n, Resize::Both);
        let present = frame(&mut e);
        assert!(changed_pixels_inside(&absent, &present, (48.0, 46.0, 14.0, 14.0)) > 0);
        let x = (135.0 * scale) as usize;
        let y = (135.0 * scale) as usize;
        let i = y * present.stride + x * 4;
        assert_eq!(&present.pixels[i..i + 4], &[0, 128, 0, 255]);
    }
}

#[test]
fn owned_recording_survives_engine_drop_and_replay_restores_the_caller_canvas() {
    for scale in SCALES {
        let (mut e, n) = fixture(scale, 14.0, 14.0);
        let scene = e.scene().unwrap();
        let stats = e.stats();
        let dirty = e.dirty_generations();
        assert_eq!(e.scene().unwrap().generation(), scene.generation());
        assert_eq!(e.stats(), stats);
        assert_eq!(e.dirty_generations(), dirty);
        let recording =
            openui_paint::record_fragment(e.native_document(), scene.fragments(), e.viewport())
                .unwrap();
        let expected = SoftwareCompositor::default().render(&scene).unwrap();
        resize(&mut e, n, Resize::None);
        assert_ne!(frame(&mut e).pixels, expected.pixels);
        drop(e);
        assert_eq!(
            SoftwareCompositor::default().render(&scene).unwrap().pixels,
            expected.pixels
        );
        let mut surface = openui_paint::rasterize_picture(&recording).unwrap();
        let canvas = surface.canvas();
        canvas.save();
        canvas.translate((3.0, 4.0));
        let mut caller_clip = recording.canonical_picture().cull_rect();
        caller_clip.right = 100.0;
        caller_clip.bottom = 100.0;
        canvas.clip_rect(caller_clip, None, false);
        let count = canvas.save_count();
        let matrix = canvas.local_to_device_as_3x3();
        let clip = canvas.device_clip_bounds();
        scene.replay(canvas);
        assert_eq!(canvas.save_count(), count);
        assert_eq!(canvas.local_to_device_as_3x3(), matrix);
        assert_eq!(canvas.device_clip_bounds(), clip);
        canvas.restore();
        assert!(surface.image_snapshot().width() > 0);
        // Exact LayoutUnit negative-half direction is covered by the pure
        // first/current operand guards; no float tolerance is introduced.
        assert_eq!(LayoutUnit::from_f32(-0.5).round_i32(), 0);
    }
}

fn assert_tiny_thin_arrow_chromium_frame(
    actual: &Frame,
    width: usize,
    crop: (usize, usize, usize, usize),
    rgba: &[u8],
) {
    assert_eq!(
        (actual.width as usize, actual.height as usize, actual.stride),
        (width, width, width * 4)
    );
    assert_eq!(actual.pixels.len(), width * width * 4);
    let (left, top, crop_width, crop_height) = crop;
    assert!(left + crop_width <= width && top + crop_height <= width);
    assert_eq!(rgba.len(), crop_width * crop_height * 4);
    let mut expected = vec![255_u8; width * width * 4];
    for row in 0..crop_height {
        let source = row * crop_width * 4;
        let destination = (top + row) * width * 4 + left * 4;
        expected[destination..destination + crop_width * 4]
            .copy_from_slice(&rgba[source..source + crop_width * 4]);
    }
    for y in 0..width {
        for x in 0..width {
            let i = (y * width + x) * 4;
            assert_eq!(
                &actual.pixels[i..i + 4],
                &expected[i..i + 4],
                "Tiny Thin arrow Chromium pixel ({x},{y}) at scale {}",
                actual.viewport.device_scale_factor()
            );
        }
    }
}

#[test]
fn tiny_thin_arrow_parts_match_complete_pinned_chromium_frames() {
    let cases: [(f64, usize, (usize, usize, usize, usize), &[u8]); 5] = [
        (
            1.0,
            160,
            (32, 32, 14, 14),
            include_bytes!("native_scrollbar_arrow_chromium_oracles/tiny-thin14-scale-1.rgba"),
        ),
        (
            1.25,
            200,
            (40, 40, 18, 18),
            include_bytes!("native_scrollbar_arrow_chromium_oracles/tiny-thin14-scale-1_25.rgba"),
        ),
        (
            1.5,
            240,
            (48, 48, 21, 21),
            include_bytes!("native_scrollbar_arrow_chromium_oracles/tiny-thin14-scale-1_5.rgba"),
        ),
        (
            1.75,
            280,
            (56, 56, 25, 25),
            include_bytes!("native_scrollbar_arrow_chromium_oracles/tiny-thin14-scale-1_75.rgba"),
        ),
        (
            2.0,
            320,
            (64, 64, 28, 28),
            include_bytes!("native_scrollbar_arrow_chromium_oracles/tiny-thin14-scale-2.rgba"),
        ),
    ];
    for (scale, width, crop, rgba) in cases {
        let (mut e, n) = fixture(scale, 14.0, 14.0);
        e.set_renderer_node_state(n, RendererNodeState::Replaced(None))
            .unwrap();
        resize(&mut e, n, Resize::None);
        set(
            &mut e,
            n,
            RendererStyleValue::ScrollbarWidth(ScrollbarWidth::Thin),
        );
        set(&mut e, n, RendererStyleValue::OverflowX(Overflow::Scroll));
        set(&mut e, n, RendererStyleValue::OverflowY(Overflow::Scroll));
        let scene = e.scene().unwrap();
        assert_eq!(geometry(&scene, n), (32.0, 32.0, 14.0, 14.0));
        let computed = e.computed_style(n).unwrap();
        assert_eq!(computed.resize, Resize::None);
        assert_eq!(computed.scrollbar_width, ScrollbarWidth::Thin);
        assert_eq!(
            (computed.overflow_x, computed.overflow_y),
            (Overflow::Scroll, Overflow::Scroll)
        );
        let actual = SoftwareCompositor::default().render(&scene).unwrap();
        assert_tiny_thin_arrow_chromium_frame(&actual, width, crop, rgba);
    }
}
