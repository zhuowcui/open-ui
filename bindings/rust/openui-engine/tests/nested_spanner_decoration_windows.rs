//! Actual source-sliced nested multicol decoration ownership and retained overflow.
//! These are native API/paint guards, not new Chromium reference pixels.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineOptions, NodeHandle, ViewportMetrics};
use openui_geometry::{LayoutUnit, RasterConfiguration};
use openui_layout::{Fragment, FragmentKind};
use openui_style::*;

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn set(e: &mut Engine, n: NodeHandle, v: RendererStyleValue) {
    e.set_renderer_style(n, v).unwrap();
}
fn element(e: &mut Engine, parent: NodeHandle) -> NodeHandle {
    let n = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(parent, n).unwrap();
    for v in [
        RendererStyleValue::Display(Display::Block),
        RendererStyleValue::BoxSizing(BoxSizing::ContentBox),
        RendererStyleValue::MarginTop(Length::px(0.0)),
        RendererStyleValue::MarginRight(Length::px(0.0)),
        RendererStyleValue::MarginBottom(Length::px(0.0)),
        RendererStyleValue::MarginLeft(Length::px(0.0)),
        RendererStyleValue::PaddingTop(Length::px(0.0)),
        RendererStyleValue::PaddingRight(Length::px(0.0)),
        RendererStyleValue::PaddingBottom(Length::px(0.0)),
        RendererStyleValue::PaddingLeft(Length::px(0.0)),
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    ] {
        set(e, n, v);
    }
    n
}
fn fixture(scale: f64) -> (Engine, NodeHandle) {
    let mut e = Engine::new_with_font_collection_and_options(
        ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
        openui_text::FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )
    .unwrap();
    let root = e.root();
    set(
        &mut e,
        root,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
    );
    set(
        &mut e,
        root,
        RendererStyleValue::BackgroundColor(Color::WHITE),
    );
    let body = element(&mut e, root);
    for v in [
        RendererStyleValue::PaddingTop(Length::px(20.0)),
        RendererStyleValue::PaddingRight(Length::px(20.0)),
        RendererStyleValue::PaddingBottom(Length::px(20.0)),
        RendererStyleValue::PaddingLeft(Length::px(20.0)),
    ] {
        set(&mut e, body, v);
    }
    let outer = element(&mut e, body);
    for v in [
        RendererStyleValue::Width(Length::px(400.0)),
        RendererStyleValue::Height(Length::px(110.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnRuleWidth(6),
        RendererStyleValue::ColumnRuleStyle(BorderStyle::Solid),
    ] {
        set(&mut e, outer, v);
    }
    let inner = element(&mut e, outer);
    for v in [
        RendererStyleValue::Height(Length::px(270.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnRuleWidth(3),
        RendererStyleValue::ColumnRuleStyle(BorderStyle::Solid),
        RendererStyleValue::BackgroundColor(Color::from_rgba8(144, 238, 144, 255)),
        RendererStyleValue::BorderTopWidth(10),
        RendererStyleValue::BorderRightWidth(10),
        RendererStyleValue::BorderBottomWidth(10),
        RendererStyleValue::BorderLeftWidth(10),
        RendererStyleValue::BorderTopStyle(BorderStyle::Solid),
        RendererStyleValue::BorderRightStyle(BorderStyle::Solid),
        RendererStyleValue::BorderBottomStyle(BorderStyle::Solid),
        RendererStyleValue::BorderLeftStyle(BorderStyle::Solid),
    ] {
        set(&mut e, inner, v);
    }
    let before = element(&mut e, inner);
    set(
        &mut e,
        before,
        RendererStyleValue::Height(Length::px(200.0)),
    );
    set(
        &mut e,
        before,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 255, 0, 255)),
    );
    let span = element(&mut e, inner);
    set(
        &mut e,
        span,
        RendererStyleValue::ColumnSpan(ColumnSpan::All),
    );
    set(&mut e, span, RendererStyleValue::Height(Length::px(50.0)));
    set(
        &mut e,
        span,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(173, 216, 230, 255)),
    );
    let after = element(&mut e, inner);
    set(&mut e, after, RendererStyleValue::Height(Length::px(240.0)));
    set(
        &mut e,
        after,
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 255, 0, 255)),
    );
    (e, span)
}
fn raster(scene: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(scene).unwrap()
}
fn projections<'a>(
    fragment: &'a Fragment,
    e: &Engine,
    parent: Option<&'a Fragment>,
    out: &mut Vec<(&'a Fragment, &'a Fragment)>,
) {
    if fragment.kind == FragmentKind::Box
        && fragment.owns_element_box()
        && !fragment.node_id.is_none()
        && e.native_document().node(fragment.node_id).style.column_span == ColumnSpan::All
    {
        out.push((parent.expect("retained direct spanner owner"), fragment));
    }
    for child in &fragment.children {
        projections(child, e, Some(fragment), out);
    }
}
fn observed(e: &mut Engine) -> (Frame, Vec<(i32, i32, i32, bool, usize)>) {
    e.set_layered_paint_capture_enabled(false);
    let ordinary = e.scene().unwrap();
    let frame = raster(&ordinary);
    let mut spans = Vec::new();
    projections(ordinary.fragments(), e, None, &mut spans);
    assert!(
        !spans.is_empty(),
        "actual retained source spanners required"
    );
    let values = spans
        .iter()
        .map(|(parent, span)| {
            let slice = parent
                .decoration_slice
                .expect("actual ancestor source slice");
            assert_eq!(span.decoration_slice, None);
            assert_eq!(
                span.fragmentation_visual_offset,
                openui_geometry::PhysicalOffset::zero()
            );
            (
                slice.source_block_offset.raw(),
                span.offset.top.raw(),
                span.size.height.raw(),
                span.skip_box_decoration,
                span.children.len(),
            )
        })
        .collect();
    e.set_layered_paint_capture_enabled(true);
    let captured = e.scene().unwrap();
    let instrumented = raster(&captured);
    assert_eq!(
        (frame.width, frame.height, frame.stride),
        (instrumented.width, instrumented.height, instrumented.stride)
    );
    assert!(
        frame.pixels == instrumented.pixels,
        "capture changed complete canonical frame"
    );
    let stats = e.stats();
    let generation = captured.generation();
    assert_eq!(e.scene().unwrap().generation(), generation);
    assert_eq!(e.stats(), stats);
    (frame, values)
}

#[test]
fn source_windows_suppress_only_nonowning_positive_spanner_decorations() {
    for scale in SCALES {
        let (mut e, span) = fixture(scale);
        let (initial, values) = observed(&mut e);
        let unit = |n| LayoutUnit::from_i32(n).raw();
        assert_eq!(
            values,
            [
                (unit(0), unit(110), unit(50), true, 0),
                (unit(110), unit(0), unit(50), false, 0),
                (unit(220), unit(-110), unit(50), true, 0)
            ]
        );
        set(&mut e, span, RendererStyleValue::Height(Length::px(80.0)));
        let (_, changed) = observed(&mut e);
        assert_eq!(changed.len(), 3);
        assert!(changed.iter().all(|value| value.2 == unit(80)));
        assert_eq!(
            changed.iter().map(|v| v.3).collect::<Vec<_>>(),
            [true, false, true]
        );
        set(&mut e, span, RendererStyleValue::Height(Length::px(50.0)));
        let (restored, restored_values) = observed(&mut e);
        assert_eq!(restored_values, values);
        assert!(
            restored.pixels == initial.pixels,
            "restoring source operands changed complete frame"
        );
    }
}

#[test]
fn independent_position_clone_and_effect_routes_keep_their_decoration_admission() {
    for scale in SCALES {
        for condition in 0..3 {
            let (mut e, span) = fixture(scale);
            match condition {
                0 => set(
                    &mut e,
                    span,
                    RendererStyleValue::Position(Position::Relative),
                ),
                1 => set(
                    &mut e,
                    span,
                    RendererStyleValue::BoxDecorationBreak(BoxDecorationBreak::Clone),
                ),
                2 => set(&mut e, span, RendererStyleValue::Opacity(0.5)),
                _ => unreachable!(),
            }
            let (_, values) = observed(&mut e);
            assert_eq!(values.len(), 3);
            assert!(
                values.iter().all(|value| !value.3),
                "unsupported independent route acquired suppression"
            );
        }
    }
}

#[test]
fn retained_spanner_child_ink_and_zero_border_extent_remain_paintable() {
    for scale in SCALES {
        for zero_border in [false, true] {
            let (mut e, span) = fixture(scale);
            if zero_border {
                set(&mut e, span, RendererStyleValue::Height(Length::px(0.0)));
            }
            let probe = element(&mut e, span);
            set(&mut e, probe, RendererStyleValue::Width(Length::px(20.0)));
            set(&mut e, probe, RendererStyleValue::Height(Length::px(20.0)));
            set(
                &mut e,
                probe,
                RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 0, 255, 255)),
            );
            if !zero_border {
                set(
                    &mut e,
                    probe,
                    RendererStyleValue::Transform(Transform2D {
                        f: -20.0,
                        ..Transform2D::IDENTITY
                    }),
                );
            }
            let (frame, values) = observed(&mut e);
            assert!(
                values.iter().all(|value| value.4 > 0),
                "decoration ownership removed retained child ink"
            );
            if zero_border {
                assert!(
                    values.iter().all(|value| value.2 == 0 && !value.3),
                    "zero border box lost its separate overflow path"
                );
            } else {
                assert_eq!(
                    values.iter().map(|v| v.3).collect::<Vec<_>>(),
                    [true, false, true]
                );
            }
            if zero_border {
                // The original untransformed zero-height child is covered by
                // later backgrounds in Chromium. Retained overflow becomes
                // visible when its authored transform moves it above them.
                assert_eq!(values.len(), 3, "retain every raw source projection");
                assert!(
                    values.iter().all(|value| value.4 == 1),
                    "each raw projection must retain its one child"
                );
                assert_eq!(
                    frame
                        .pixels
                        .chunks_exact(4)
                        .filter(|pixel| *pixel == [255, 0, 255, 255])
                        .count(),
                    0,
                    "ordinary zero-height baseline must match Chromium coverage"
                );
                let generation = e.scene().unwrap().generation();
                let stats = e.stats();
                let span_rects = e.client_rects(span).unwrap();
                let probe_rects = e.client_rects(probe).unwrap();
                let span_bounds = e.bounds(span).unwrap();
                let probe_bounds = e.bounds(probe).unwrap();
                assert_eq!(span_rects.len(), 1);
                assert_eq!(probe_rects.len(), 1);
                assert!(span_bounds.is_some() && probe_bounds.is_some());
                assert!(!e.computed_style(probe).unwrap().has_transform());
                assert_eq!(e.scene().unwrap().generation(), generation);
                assert_eq!(e.stats(), stats, "cached baseline geometry performed work");

                // The fixture uses resolved snapshot setters. An authored
                // override must be removed to restore that original source;
                // assigning an identity matrix would retain a declaration.
                e.set_property(
                    probe,
                    StyleProperty::Transform,
                    StyleValue::Transform(TransformList(vec![TransformOperation::Matrix(
                        Transform2D {
                            f: -20.0,
                            ..Transform2D::IDENTITY
                        },
                    )])),
                )
                .unwrap();
                let (exposed, exposed_values) = observed(&mut e);
                assert_eq!(
                    exposed_values, values,
                    "authored transform changed raw source"
                );
                assert_eq!(
                    (exposed.width, exposed.height, exposed.stride),
                    (frame.width, frame.height, frame.stride)
                );
                assert_eq!(exposed.pixels.len(), frame.pixels.len());
                assert!(
                    exposed
                        .pixels
                        .chunks_exact(4)
                        .any(|pixel| pixel == [255, 0, 255, 255]),
                    "authored translation must expose retained zero-height child ink"
                );
                let generation = e.scene().unwrap().generation();
                let stats = e.stats();
                assert!(e.computed_style(probe).unwrap().has_transform());
                assert_eq!(e.client_rects(span).unwrap(), span_rects);
                assert_eq!(e.bounds(span).unwrap(), span_bounds);
                assert_eq!(
                    e.client_rects(probe).unwrap(),
                    probe_rects
                        .iter()
                        .map(|rect| {
                            let mut moved = *rect;
                            moved.y -= 20.0;
                            moved
                        })
                        .collect::<Vec<_>>()
                );
                assert_eq!(
                    e.bounds(probe).unwrap(),
                    probe_bounds.map(|mut rect| {
                        rect.y -= 20.0;
                        rect
                    })
                );
                assert_eq!(e.scene().unwrap().generation(), generation);
                assert_eq!(e.stats(), stats, "cached exposed geometry performed work");

                assert!(e.remove_property(probe, StyleProperty::Transform).unwrap());
                let (restored, restored_values) = observed(&mut e);
                assert_eq!(restored_values, values, "raw source failed to restore");
                assert_eq!(
                    (restored.width, restored.height, restored.stride),
                    (frame.width, frame.height, frame.stride)
                );
                assert_eq!(restored.pixels.len(), frame.pixels.len());
                assert!(
                    restored.pixels == frame.pixels,
                    "removing authored transform must restore the complete baseline frame"
                );
                let generation = e.scene().unwrap().generation();
                let stats = e.stats();
                assert!(!e.computed_style(probe).unwrap().has_transform());
                assert_eq!(e.client_rects(span).unwrap(), span_rects);
                assert_eq!(e.client_rects(probe).unwrap(), probe_rects);
                assert_eq!(e.bounds(span).unwrap(), span_bounds);
                assert_eq!(e.bounds(probe).unwrap(), probe_bounds);
                assert_eq!(e.scene().unwrap().generation(), generation);
                assert_eq!(e.stats(), stats, "cached restored geometry performed work");
                continue;
            }
            assert!(
                frame
                    .pixels
                    .chunks_exact(4)
                    .any(|p| p == [255, 0, 255, 255]),
                "actual retained opaque child ink missing"
            );
        }
    }
}

#[test]
#[ignore = "Root-only original zero exposure/restore transport; fresh D output required"]
fn root_only_observe_zero_spanner_authored_transform_and_restore_all_profiles() {
    use openui_compositor::SceneRect;
    use std::fs::{self, OpenOptions};
    use std::io::Write;
    use std::path::{Component, Path, PathBuf};

    struct Observation {
        frame: Frame,
        projections: Vec<(i32, i32, i32, bool, usize)>,
        span_rects: Vec<SceneRect>,
        probe_rects: Vec<SceneRect>,
        span_bounds: Option<SceneRect>,
        probe_bounds: Option<SceneRect>,
        has_transform: bool,
    }

    fn observe_state(e: &mut Engine, span: NodeHandle, probe: NodeHandle) -> Observation {
        // The unchanged original helper asserts capture-off/on complete RGBA
        // equality and cached generation/stats NoWork before returning.
        let (frame, values) = observed(e);
        let generation = e.scene().unwrap().generation();
        let stats = e.stats();
        let span_rects = e.client_rects(span).unwrap();
        let probe_rects = e.client_rects(probe).unwrap();
        let span_bounds = e.bounds(span).unwrap();
        let probe_bounds = e.bounds(probe).unwrap();
        let has_transform = e.computed_style(probe).unwrap().has_transform();
        assert_eq!(e.scene().unwrap().generation(), generation);
        assert_eq!(e.stats(), stats);
        Observation {
            frame,
            projections: values,
            span_rects,
            probe_rects,
            span_bounds,
            probe_bounds,
            has_transform,
        }
    }

    fn rect_json(rect: &SceneRect) -> String {
        assert!([rect.x, rect.y, rect.width, rect.height]
            .iter()
            .all(|value| value.is_finite()));
        format!("[{},{},{},{}]", rect.x, rect.y, rect.width, rect.height)
    }

    fn rects_json(rects: &[SceneRect]) -> String {
        format!(
            "[{}]",
            rects.iter().map(rect_json).collect::<Vec<_>>().join(",")
        )
    }

    fn bounds_json(bounds: &Option<SceneRect>) -> String {
        bounds
            .as_ref()
            .map(rect_json)
            .unwrap_or_else(|| "null".to_string())
    }

    fn write_state(output: &Path, scale: f64, repeat: usize, state: &str, value: &Observation) {
        assert_eq!(value.frame.width as f64, 800.0 * scale);
        assert_eq!(value.frame.height as f64, 600.0 * scale);
        assert_eq!(value.frame.stride, value.frame.width as usize * 4);
        assert_eq!(
            value.frame.pixels.len(),
            value.frame.stride * value.frame.height as usize
        );
        let folder = output.join(format!("zero-span0-scale-{scale}-repeat-{repeat}-{state}"));
        fs::create_dir(&folder).unwrap();
        let mut raw = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(folder.join("frame.rgba"))
            .unwrap();
        raw.write_all(&value.frame.pixels).unwrap();
        raw.flush().unwrap();
        let projection_json = value
            .projections
            .iter()
            .map(|(offset, top, height, skip, children)| {
                format!(
                    "{{\"parent_source_block_offset_raw\":{offset},\"span_top_raw\":{top},\"span_height_raw\":{height},\"skip_box_decoration\":{skip},\"retained_child_count\":{children}}}"
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        let magenta_cells = value
            .frame
            .pixels
            .chunks_exact(4)
            .filter(|pixel| *pixel == [255, 0, 255, 255])
            .count();
        let metadata = format!(
            "{{\"scale\":{scale},\"case\":\"zero-span0\",\"repeat\":{repeat},\"state\":\"{state}\",\"width\":{},\"height\":{},\"stride\":{},\"bytes\":{},\"observed_opaque_magenta_cells\":{magenta_cells},\"capture_control_full_RGBA_equal\":true,\"api_cached_scene_NoWork\":true,\"style_has_transform\":{},\"projections\":[{projection_json}],\"span_client_rects\":{},\"probe_client_rects\":{},\"span_bounds\":{},\"probe_bounds\":{},\"visibility_or_renderer_qualification\":false}}\n",
            value.frame.width,
            value.frame.height,
            value.frame.stride,
            value.frame.pixels.len(),
            value.has_transform,
            rects_json(&value.span_rects),
            rects_json(&value.probe_rects),
            bounds_json(&value.span_bounds),
            bounds_json(&value.probe_bounds),
        );
        assert!(metadata.len() < 65_536);
        let mut record = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(folder.join("frame.json"))
            .unwrap();
        record.write_all(metadata.as_bytes()).unwrap();
        record.flush().unwrap();
    }

    let output = PathBuf::from(
        std::env::var_os("OPENUI_ZERO_SPANNER_EXPOSURE_TRANSPORT_OUTPUT")
            .expect("Root must provide a fresh D exposure transport directory"),
    );
    assert!(output.is_absolute());
    assert!(!output.components().any(|part| part == Component::ParentDir));
    let parent = output
        .parent()
        .expect("output parent")
        .canonicalize()
        .unwrap();
    assert!(parent.starts_with(Path::new("/mnt/d/openui-v02-qualification-d174ea0b")));
    assert!(
        !output.exists(),
        "Root exposure transport output must be fresh"
    );
    fs::create_dir(&output).unwrap();

    let mut frame_count = 0;
    let mut restoration_count = 0;
    for scale in SCALES {
        for repeat in 0..2 {
            // Exact original zero-height/untransformed declarations and DOM order.
            let (mut e, span) = fixture(scale);
            set(&mut e, span, RendererStyleValue::Height(Length::px(0.0)));
            let probe = element(&mut e, span);
            set(&mut e, probe, RendererStyleValue::Width(Length::px(20.0)));
            set(&mut e, probe, RendererStyleValue::Height(Length::px(20.0)));
            set(
                &mut e,
                probe,
                RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 0, 255, 255)),
            );
            let baseline = observe_state(&mut e, span, probe);
            assert!(!baseline.has_transform);
            write_state(&output, scale, repeat, "baseline", &baseline);
            frame_count += 1;

            e.set_property(
                probe,
                StyleProperty::Transform,
                StyleValue::Transform(TransformList(vec![TransformOperation::Matrix(
                    Transform2D {
                        f: -20.0,
                        ..Transform2D::IDENTITY
                    },
                )])),
            )
            .unwrap();
            let exposed = observe_state(&mut e, span, probe);
            assert!(exposed.has_transform);
            write_state(&output, scale, repeat, "exposed", &exposed);
            frame_count += 1;

            // Removing the authored property restores none. A specified identity
            // matrix would retain a transform/containing-block source instead.
            assert!(e.remove_property(probe, StyleProperty::Transform).unwrap());
            let restored = observe_state(&mut e, span, probe);
            assert!(!restored.has_transform);
            assert_eq!(restored.projections, baseline.projections);
            assert_eq!(restored.span_rects, baseline.span_rects);
            assert_eq!(restored.probe_rects, baseline.probe_rects);
            assert_eq!(restored.span_bounds, baseline.span_bounds);
            assert_eq!(restored.probe_bounds, baseline.probe_bounds);
            assert_eq!(
                (
                    restored.frame.width,
                    restored.frame.height,
                    restored.frame.stride
                ),
                (
                    baseline.frame.width,
                    baseline.frame.height,
                    baseline.frame.stride
                )
            );
            assert!(
                restored.frame.pixels == baseline.frame.pixels,
                "scale={scale} repeat={repeat} complete original zero frame did not restore"
            );
            write_state(&output, scale, repeat, "restored", &restored);
            frame_count += 1;
            restoration_count += 1;
        }
    }
    assert_eq!(frame_count, 30);
    assert_eq!(restoration_count, 10);
    let summary = b"{\"canonical_frames_written\":30,\"capture_controls_equal\":30,\"internal_canonical_rasters\":60,\"api_cached_scene_NoWork_checks\":30,\"whole_frame_source_and_API_restoration_pairs\":10,\"fresh_engine_repeats_per_scale\":2,\"original_strict_guards_modified\":true,\"expected_visibility_or_pixel_values_added\":false,\"expected_visibility_or_pixel_values_scope\":\"ignored_observer_only\",\"visibility_or_renderer_qualification\":false}\n";
    let mut record = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(output.join("observation-summary.json"))
        .unwrap();
    record.write_all(summary).unwrap();
    record.flush().unwrap();
}
