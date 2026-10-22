//! Decoration-only split-CB source views retain the real positioned subtree.
//! No new Chromium reference pixels or existing guard expectations are changed.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineOptions, NodeHandle, ViewportMetrics};
use openui_geometry::{Length, RasterConfiguration};
use openui_layout::{Fragment, FragmentKind};
use openui_style::*;

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn set(e: &mut Engine, n: NodeHandle, v: RendererStyleValue, authored: bool) {
    if authored {
        e.set_property(n, v.property(), StyleValue::Renderer(v))
            .unwrap();
    } else {
        e.set_renderer_style(n, v).unwrap();
    }
}

fn element(e: &mut Engine, p: NodeHandle, authored: bool) -> NodeHandle {
    let n = e.create_native_element(ElementTag::Div).unwrap();
    e.append_child(p, n).unwrap();
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
        set(e, n, v, authored);
    }
    n
}

fn source_gradient() -> LinearGradient {
    LinearGradient {
        angle_degrees: 180.0,
        repeating: false,
        stops: vec![
            LinearGradientStop {
                color: Color::RED,
                position: GradientStopPosition::Percent(0.0),
            },
            LinearGradientStop {
                color: Color::BLUE,
                position: GradientStopPosition::Percent(100.0),
            },
        ],
    }
}
fn source_layer() -> BackgroundLayer {
    BackgroundLayer::new(CssImage::LinearGradient(CssLinearGradient {
        angle_degrees: 180.0,
        corner_direction: None,
        repeating: false,
        color_space: GradientColorSpace::Srgb,
        stops: vec![
            GradientStop {
                color: StyleColor::Resolved(Color::RED),
                position: GradientStopPosition::Percent(0.0),
            },
            GradientStop {
                color: StyleColor::Resolved(Color::BLUE),
                position: GradientStopPosition::Percent(100.0),
            },
        ],
    }))
}

fn fixture(scale: f64, authored: bool, condition: usize) -> (Engine, NodeHandle) {
    let mut e = Engine::new_with_font_collection_and_options(
        ViewportMetrics::from_logical_size(480.0, 360.0, scale).unwrap(),
        openui_text::FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
            ..EngineOptions::default()
        },
    )
    .unwrap();
    let root = e.root();
    set(
        &mut e,
        root,
        RendererStyleValue::BackgroundColor(Color::WHITE),
        authored,
    );
    set(
        &mut e,
        root,
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        authored,
    );
    let body = element(&mut e, root, authored);
    for v in [
        RendererStyleValue::PaddingLeft(Length::px(24.0)),
        RendererStyleValue::PaddingTop(Length::px(24.0)),
    ] {
        set(&mut e, body, v, authored);
    }
    let outer = element(&mut e, body, authored);
    for v in [
        RendererStyleValue::Width(Length::px(120.0)),
        RendererStyleValue::Height(Length::px(120.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnGap(Some(Length::px(0.0))),
        RendererStyleValue::ColumnFill(ColumnFill::Auto),
    ] {
        set(&mut e, outer, v, authored);
    }
    let inner = element(&mut e, outer, authored);
    for v in [
        RendererStyleValue::Width(Length::px(60.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnGap(Some(Length::px(0.0))),
        RendererStyleValue::ColumnFill(ColumnFill::Auto),
        RendererStyleValue::BackgroundColor(Color::RED),
    ] {
        set(&mut e, inner, v, authored);
    }
    let cb = element(&mut e, inner, authored);
    set(
        &mut e,
        cb,
        RendererStyleValue::Position(Position::Relative),
        authored,
    );
    set(
        &mut e,
        cb,
        RendererStyleValue::Height(Length::px(400.0)),
        authored,
    );
    if condition == 2 {
        set(
            &mut e,
            cb,
            RendererStyleValue::PaddingTop(Length::px(4.0)),
            authored,
        );
    }
    let before = element(&mut e, cb, authored);
    set(
        &mut e,
        before,
        RendererStyleValue::Height(Length::px(160.0)),
        authored,
    );
    let spanner = element(&mut e, cb, authored);
    set(
        &mut e,
        spanner,
        RendererStyleValue::ColumnSpan(ColumnSpan::All),
        authored,
    );
    set(
        &mut e,
        spanner,
        RendererStyleValue::Height(Length::px(20.0)),
        authored,
    );
    let positioned = element(&mut e, cb, authored);
    for v in [
        RendererStyleValue::Position(Position::Absolute),
        RendererStyleValue::Top(Length::px(10.0)),
        RendererStyleValue::Bottom(Length::px(30.0)),
        RendererStyleValue::Width(Length::px(30.0)),
        RendererStyleValue::BackgroundColor(Color::from_rgba8(96, 48, 160, 255)),
    ] {
        set(&mut e, positioned, v, authored);
    }
    match condition {
        0 | 2 => {}
        1 => {
            set(
                &mut e,
                positioned,
                RendererStyleValue::Top(Length::px(-10.0)),
                authored,
            );
            set(
                &mut e,
                positioned,
                RendererStyleValue::Bottom(Length::auto()),
                authored,
            );
            set(
                &mut e,
                positioned,
                RendererStyleValue::Height(Length::px(430.0)),
                authored,
            );
        }
        3 => set(
            &mut e,
            positioned,
            RendererStyleValue::BoxDecorationBreak(BoxDecorationBreak::Clone),
            authored,
        ),
        4 => {
            set(
                &mut e,
                positioned,
                RendererStyleValue::BorderTopWidth(2),
                authored,
            );
            set(
                &mut e,
                positioned,
                RendererStyleValue::BorderTopStyle(BorderStyle::Solid),
                authored,
            );
        }
        5 => set(
            &mut e,
            positioned,
            RendererStyleValue::BoxShadow(vec![BoxShadow {
                offset_x: 1.0,
                offset_y: 2.0,
                blur_radius: 0.0,
                spread_radius: 1.0,
                color: Color::BLACK,
                inset: false,
            }]),
            authored,
        ),
        6..=9 => {
            // Fractions are real authored source inputs; fallback is selected
            // by background semantics, never by the fractional phase.
            set(
                &mut e,
                positioned,
                RendererStyleValue::Top(Length::px(10.25)),
                authored,
            );
            set(
                &mut e,
                positioned,
                RendererStyleValue::Bottom(Length::px(30.5)),
                authored,
            );
            match condition {
                6 => set(
                    &mut e,
                    positioned,
                    RendererStyleValue::BackgroundLinearGradient(Some(source_gradient())),
                    authored,
                ),
                7 => {
                    set(
                        &mut e,
                        positioned,
                        RendererStyleValue::BackgroundClip(BackgroundClip::PaddingBox),
                        authored,
                    );
                    set(
                        &mut e,
                        positioned,
                        RendererStyleValue::PaddingTop(Length::px(4.0)),
                        authored,
                    );
                }
                8 => {
                    set(
                        &mut e,
                        positioned,
                        RendererStyleValue::BackgroundClip(BackgroundClip::ContentBox),
                        authored,
                    );
                    set(
                        &mut e,
                        positioned,
                        RendererStyleValue::PaddingTop(Length::px(4.0)),
                        authored,
                    );
                }
                9 => set(
                    &mut e,
                    positioned,
                    RendererStyleValue::BackgroundLayers(vec![source_layer()]),
                    authored,
                ),
                _ => unreachable!(),
            }
        }
        _ => unreachable!(),
    }
    let ink = element(&mut e, positioned, authored);
    for v in [
        RendererStyleValue::Width(Length::px(12.0)),
        RendererStyleValue::Height(Length::px(20.0)),
        RendererStyleValue::BackgroundColor(Color::from_rgba8(255, 0, 255, 255)),
        RendererStyleValue::Transform(Transform2D {
            e: 4.0,
            f: 2.0,
            ..Transform2D::IDENTITY
        }),
    ] {
        set(&mut e, ink, v, authored);
    }
    (e, positioned)
}

#[derive(Debug, PartialEq, Eq)]
struct SourceProjection {
    child_size: [i32; 2],
    child_offset: [i32; 2],
    cb_source_size: i32,
    split: i32,
    consumed: i32,
    principal: i32,
    child_source_start: i32,
    child_coordinates: Vec<[i32; 4]>,
}

fn source_projections(f: &Fragment, parent: Option<&Fragment>, out: &mut Vec<SourceProjection>) {
    if f.kind == FragmentKind::Box && f.owns_element_box() {
        if let Some(positioned) = f.positioned_fragmentation {
            if let Some(split) = positioned.split_containing_block_source_offset {
                let cb = parent.expect("retained actual CB parent");
                assert_eq!(positioned.containing_block_node, cb.node_id);
                let g = cb
                    .box_geometry
                    .expect("actual principal continuation source");
                assert!(g.continuation_record && !g.is_repeated);
                assert!(g.writing_direction.is_horizontal());
                assert!(
                    g.block_size.raw() >= 0,
                    "recorded principal intervals cannot be negative"
                );
                assert_eq!(f.box_geometry, None);
                assert_eq!(f.decoration_slice, None);
                assert_eq!(f.decoration_paint_block_size, None);
                assert_eq!(positioned.fragmentainer_index, None);
                assert!(
                    !f.children.is_empty(),
                    "decoration view lost retained child ink"
                );
                out.push(SourceProjection {
                    child_size: [f.size.width.raw(), f.size.height.raw()],
                    child_offset: [f.offset.left.raw(), f.offset.top.raw()],
                    cb_source_size: positioned.containing_block_size.height.raw(),
                    split: split.raw(),
                    consumed: g.consumed_block_size.raw(),
                    principal: g.block_size.raw(),
                    child_source_start: (f.offset.top + g.consumed_block_size
                        - positioned.containing_block_offset.top)
                        .raw(),
                    child_coordinates: f
                        .children
                        .iter()
                        .map(|child| {
                            [
                                child.offset.left.raw(),
                                child.offset.top.raw(),
                                child.size.width.raw(),
                                child.size.height.raw(),
                            ]
                        })
                        .collect(),
                });
            }
        }
    }
    for child in &f.children {
        source_projections(child, Some(f), out);
    }
}

fn raster(scene: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(scene).unwrap()
}

fn observe(e: &mut Engine) -> (Frame, Vec<SourceProjection>) {
    e.set_layered_paint_capture_enabled(false);
    let ordinary = e.scene().unwrap();
    let frame = raster(&ordinary);
    let mut source = Vec::new();
    source_projections(ordinary.fragments(), None, &mut source);
    assert!(
        !source.is_empty(),
        "the split-CB source route must actually execute"
    );
    assert!(
        source.iter().any(|record| record.principal > 0),
        "a positive owned source window must actually execute"
    );
    e.set_layered_paint_capture_enabled(true);
    let captured = e.scene().unwrap();
    assert!(captured.layered_paint_capture().is_some());
    let instrumented = raster(&captured);
    assert_eq!(
        (frame.width, frame.height, frame.stride),
        (instrumented.width, instrumented.height, instrumented.stride)
    );
    assert!(
        frame.pixels == instrumented.pixels,
        "same-walk capture changed the entire canonical frame"
    );
    let stats = e.stats();
    assert_eq!(e.scene().unwrap().generation(), captured.generation());
    assert_eq!(e.stats(), stats, "cached scene performed work");
    (frame, source)
}

// CB principal publication is distinct from raw OOF carriers and their paint.
// The six exact base CB extents are the immutable Chrome4912 observation;
// the remaining neighbors assert source conservation, not new pixel goldens.
fn principal_windows(source: &[SourceProjection]) -> Vec<(i32, i32)> {
    source
        .iter()
        .map(|p| (p.split + p.consumed, p.principal))
        .collect()
}

fn assert_padding_box_source(source: &[SourceProjection], top: f32, bottom: f32) {
    let total = ((400.0 + top + bottom) * 64.0) as i32;
    let mut next = 0;
    for record in source {
        assert_eq!(
            record.split + record.consumed,
            next,
            "principal source must remain contiguous"
        );
        assert!(record.principal > 0);
        next += record.principal;
        assert_eq!(record.cb_source_size, total);
        assert_eq!(
            record.child_size[1],
            total - 40 * 64,
            "raw solved OOF source remains complete"
        );
        assert_eq!(
            record.child_source_start,
            10 * 64,
            "all source coordinates share the solved CB basis"
        );
        assert!(!record.child_coordinates.is_empty());
    }
    assert_eq!(
        next, total,
        "capacity or repeated padding must not manufacture principal source"
    );
}

#[test]
fn padded_split_principal_source_conserves_actual_probe_padding_and_short_tail() {
    for scale in SCALES {
        for authored in [false, true] {
            let (mut e, positioned) = fixture(scale, authored, 2);
            let (baseline, source) = observe(&mut e);
            assert_padding_box_source(&source, 4.0, 0.0);
            assert_eq!(
                principal_windows(&source),
                vec![
                    (0, 82 * 64),
                    (82 * 64, 82 * 64),
                    (164 * 64, 18 * 64),
                    (182 * 64, 18 * 64),
                    (200 * 64, 120 * 64),
                    (320 * 64, 84 * 64)
                ]
            );
            let cb = e.parent(positioned).unwrap().unwrap();
            set(
                &mut e,
                cb,
                RendererStyleValue::PaddingTop(Length::px(7.5)),
                authored,
            );
            set(
                &mut e,
                cb,
                RendererStyleValue::PaddingBottom(Length::px(3.25)),
                authored,
            );
            let (_, changed) = observe(&mut e);
            assert_padding_box_source(&changed, 7.5, 3.25);
            set(
                &mut e,
                cb,
                RendererStyleValue::PaddingTop(Length::px(4.0)),
                authored,
            );
            set(
                &mut e,
                cb,
                RendererStyleValue::PaddingBottom(Length::px(0.0)),
                authored,
            );
            let (restored, restored_source) = observe(&mut e);
            assert_eq!(restored_source, source);
            assert_eq!(restored.pixels, baseline.pixels);
        }
    }
}

fn source_parent_column_capacities(f: &Fragment, parent: Option<&Fragment>, out: &mut Vec<i32>) {
    if f.children.iter().any(|child| {
        child.positioned_fragmentation.is_some_and(|p| {
            p.containing_block_node == f.node_id && p.split_containing_block_source_offset.is_some()
        })
    }) {
        if let Some(column) = parent.filter(|p| p.kind == FragmentKind::ColumnBox) {
            out.push(column.size.height.raw());
        }
    }
    for child in &f.children {
        source_parent_column_capacities(child, Some(f), out);
    }
}

#[test]
fn padded_split_first_and_fresh_capacities_share_source_progress_and_live_outer_size() {
    for scale in SCALES {
        for authored in [false, true] {
            let (mut e, positioned) = fixture(scale, authored, 2);
            let (baseline, source) = observe(&mut e);
            let cb = e.parent(positioned).unwrap().unwrap();
            let inner = e.parent(cb).unwrap().unwrap();
            let outer = e.parent(inner).unwrap().unwrap();
            set(
                &mut e,
                outer,
                RendererStyleValue::Height(Length::px(160.0)),
                authored,
            );
            let (_, changed) = observe(&mut e);
            assert_padding_box_source(&changed, 4.0, 0.0);
            let scene = e.scene().unwrap();
            let mut capacities = Vec::new();
            source_parent_column_capacities(scene.fragments(), None, &mut capacities);
            assert!(capacities.contains(&(160*64)), "a fresh column must use actual new enclosing capacity independently of short source progress");
            assert!(
                changed
                    .iter()
                    .any(|p| p.split > 0 && p.principal < 160 * 64),
                "source end keeps a short principal"
            );
            set(
                &mut e,
                outer,
                RendererStyleValue::Height(Length::px(120.0)),
                authored,
            );
            let (restored, restored_source) = observe(&mut e);
            assert_eq!(restored_source, source);
            assert_eq!(restored.pixels, baseline.pixels);
        }
    }
}

#[test]
fn padded_split_source_admission_keeps_clone_border_and_genuine_oof_overflow_neighbors() {
    for scale in SCALES {
        for authored in [false, true] {
            for condition in [1, 3, 4, 5, 6, 7, 8, 9] {
                let (mut e, positioned) = fixture(scale, authored, condition);
                let (baseline, source) = observe(&mut e);
                let cb = e.parent(positioned).unwrap().unwrap();
                set(
                    &mut e,
                    cb,
                    RendererStyleValue::BackgroundColor(Color::BLUE),
                    authored,
                );
                let (_, changed) = observe(&mut e);
                assert_eq!(
                    changed, source,
                    "paint-only changes cannot alter source admission"
                );
                set(
                    &mut e,
                    cb,
                    RendererStyleValue::BackgroundColor(Color::TRANSPARENT),
                    authored,
                );
                let (restored, restored_source) = observe(&mut e);
                assert_eq!(restored_source, source);
                assert_eq!(restored.pixels, baseline.pixels);
            }
        }
    }
}

// Append-only Root diagnostic: the strict producer tests above are unchanged.
fn continuity_observation_style(text: &mut String, e: &Engine, role: &str, handle: NodeHandle) {
    use std::fmt::Write as _;
    let s = e.computed_style(handle).unwrap();
    writeln!(text,
        "STYLE role={role} display={:?} position={:?} box_sizing={:?} direction={:?} writing_mode={:?} width={:?} height={:?} top={:?} bottom={:?} padding={:?} border_widths={:?} border_styles={:?} column_count={:?} column_fill={:?} column_gap={:?} column_span={:?} decoration_break={:?} background_clip={:?} background_layer_count={} compatibility_gradient={} shadow_count={}",
        s.display,s.position,s.box_sizing,s.direction,s.writing_mode,s.width,s.height,s.top,s.bottom,
        [s.padding_top,s.padding_right,s.padding_bottom,s.padding_left],
        [s.border_top_width,s.border_right_width,s.border_bottom_width,s.border_left_width],
        [s.border_top_style,s.border_right_style,s.border_bottom_style,s.border_left_style],
        s.column_count,s.column_fill,s.column_gap,s.column_span,s.box_decoration_break,
        s.background_clip,s.background_layers.len(),s.background_linear_gradient.is_some(),s.box_shadow.len()).unwrap();
}

fn continuity_observation_create_new(path: &std::path::Path, bytes: &[u8]) {
    use std::io::Write as _;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .unwrap();
    f.write_all(bytes).unwrap();
    f.sync_all().unwrap();
}

fn continuity_observation_raw_tree(
    text: &mut String,
    fragment: &Fragment,
    path: &mut Vec<usize>,
    count: &mut usize,
) {
    use std::fmt::Write as _;
    *count += 1;
    assert!(*count <= 2048, "bounded complete retained tree observation");
    writeln!(text,
        "RAW path={:?} node={:?} kind={:?} owns_raw={} offset={:?} size={:?} first={} last={} skip={} padding={:?} border={:?} geometry={:?} decoration={:?} decoration_limit={:?} positioned={:?} fragmentation_axis={:?} visual_offset={:?} has_clip={} block_only_clip={} inline_only_clip={} child_count={}",
        path,fragment.node_id,fragment.kind,fragment.owns_element_box(),fragment.offset,fragment.size,
        fragment.is_first_for_node,fragment.is_last_for_node,fragment.skip_box_decoration,
        fragment.padding,fragment.border,fragment.box_geometry,fragment.decoration_slice,
        fragment.decoration_paint_block_size,fragment.positioned_fragmentation,
        fragment.fragmentation_writing_direction,fragment.fragmentation_visual_offset,
        fragment.has_overflow_clip,fragment.block_axis_clip_only,fragment.inline_axis_clip_only,
        fragment.children.len()).unwrap();
    writeln!(text,
        "RAW_SOURCE path={:?} box_ownership={:?} principal_box={:?} margin={:?} overflow={:?} visual_offsets={:?} token={:?} multicol={:?} clip_rects={:?} ink_start={:?} ink_end={:?} promoted={:?}",
        path,fragment.box_ownership,fragment.principal_box_rect,fragment.margin,fragment.overflow_rect,
        fragment.fragmentation_visual_offsets,fragment.break_token,fragment.multicol_fragmentation,
        fragment.decoration_clip_rects,fragment.column_block_start_ink_overflow,
        fragment.column_block_end_ink_overflow,fragment.promoted_transform_ancestors).unwrap();
    for (index, child) in fragment.children.iter().enumerate() {
        path.push(index);
        continuity_observation_raw_tree(text, child, path, count);
        path.pop();
    }
}

#[test]
#[ignore = "Root-only unchanged baseline/fractional-padding/live-outer source observation; fresh output directory required"]
fn root_only_observe_completed_split_row_admission_fractional_padding_and_live_outer() {
    use std::fmt::Write as _;
    let directory = std::path::PathBuf::from(
        std::env::var_os("OPENUI_COMPLETED_SPLIT_ROW_OUTPUT")
            .expect("Root must supply OPENUI_COMPLETED_SPLIT_ROW_OUTPUT"),
    );
    std::fs::create_dir(&directory).unwrap();
    let mut rows =
        String::from("authored\trepeat\tstate\twidth\theight\tstride\tbytes\trgba\tmetadata\n");
    let mut count = 0usize;
    for authored in [false, true] {
        for repeat in 0..2 {
            let (mut e, positioned) = fixture(1.0, authored, 2);
            let cb = e.parent(positioned).unwrap().unwrap();
            let inner = e.parent(cb).unwrap().unwrap();
            let outer = e.parent(inner).unwrap().unwrap();
            let ink = e.children(positioned).unwrap()[0];
            for state in [
                "baseline",
                "fractional-padding",
                "fractional-restored",
                "outer-160",
                "outer-restored",
            ] {
                match state {
                    "fractional-padding" => {
                        set(
                            &mut e,
                            cb,
                            RendererStyleValue::PaddingTop(Length::px(7.5)),
                            authored,
                        );
                        set(
                            &mut e,
                            cb,
                            RendererStyleValue::PaddingBottom(Length::px(3.25)),
                            authored,
                        );
                    }
                    "fractional-restored" => {
                        set(
                            &mut e,
                            cb,
                            RendererStyleValue::PaddingTop(Length::px(4.0)),
                            authored,
                        );
                        set(
                            &mut e,
                            cb,
                            RendererStyleValue::PaddingBottom(Length::px(0.0)),
                            authored,
                        );
                    }
                    "outer-160" => set(
                        &mut e,
                        outer,
                        RendererStyleValue::Height(Length::px(160.0)),
                        authored,
                    ),
                    "outer-restored" => set(
                        &mut e,
                        outer,
                        RendererStyleValue::Height(Length::px(120.0)),
                        authored,
                    ),
                    _ => {}
                }
                eprintln!(
                    "PADDED_CONTINUITY_BEGIN authored={authored} repeat={repeat} state={state}"
                );
                // Reuse canonical/capture full-RGBA equality and cached NoWork;
                // observe() has no contiguous-origin or expected principal-vector assertion.
                let (frame, source) = observe(&mut e);
                assert_eq!(frame.stride, frame.width as usize * 4);
                assert_eq!(frame.pixels.len(), frame.stride * frame.height as usize);
                let stats = e.stats();
                let scene = e.scene().unwrap();
                let mut text = String::new();
                writeln!(
                    text,
                    "CASE scale=1 authored={authored} repeat={repeat} state={state}"
                )
                .unwrap();
                writeln!(text, "FRAME width={} height={} stride={} bytes={} format=RGBA8888-premultiplied viewport={:?} raster_configuration={:?} backend={:?} generation={:?}",
                    frame.width,frame.height,frame.stride,frame.pixels.len(),frame.viewport,
                    frame.raster_configuration,frame.raster_backend_identity,frame.scene_generation).unwrap();
                writeln!(text, "SOURCE_PROJECTIONS {:?}", source).unwrap();
                let mut raw_count = 0usize;
                continuity_observation_raw_tree(
                    &mut text,
                    scene.fragments(),
                    &mut Vec::new(),
                    &mut raw_count,
                );
                writeln!(text, "RAW_COUNT {raw_count}").unwrap();
                for (role, handle) in [
                    ("outer", outer),
                    ("inner", inner),
                    ("cb", cb),
                    ("positioned", positioned),
                    ("ink", ink),
                ] {
                    continuity_observation_style(&mut text, &e, role, handle);
                    let rects = e.client_rects(handle).unwrap();
                    assert!(
                        rects.len() <= 2048,
                        "bounded complete client vector observation"
                    );
                    writeln!(
                        text,
                        "GEOMETRY role={role} client_rects={:?} bounds={:?}",
                        rects,
                        e.bounds(handle).unwrap()
                    )
                    .unwrap();
                }
                assert_eq!(
                    e.stats(),
                    stats,
                    "cached geometry observation performed work"
                );
                assert!(
                    text.len() <= 4 * 1024 * 1024,
                    "bounded whole retained tree metadata"
                );
                let stem = format!("authored-{}-repeat-{repeat}-{state}", u8::from(authored));
                let rgba = format!("{stem}.rgba");
                let metadata = format!("{stem}.txt");
                continuity_observation_create_new(&directory.join(&rgba), &frame.pixels);
                continuity_observation_create_new(&directory.join(&metadata), text.as_bytes());
                writeln!(
                    rows,
                    "{authored}\t{repeat}\t{state}\t{}\t{}\t{}\t{}\t{rgba}\t{metadata}",
                    frame.width,
                    frame.height,
                    frame.stride,
                    frame.pixels.len()
                )
                .unwrap();
                eprintln!("PADDED_CONTINUITY_END authored={authored} repeat={repeat} state={state} raw_count={raw_count}");
                count += 1;
            }
        }
    }
    assert_eq!(
        count, 20,
        "all five exact unchanged source states and both setter routes and independent repeats must execute"
    );
    continuity_observation_create_new(&directory.join("frames.tsv"), rows.as_bytes());
}
