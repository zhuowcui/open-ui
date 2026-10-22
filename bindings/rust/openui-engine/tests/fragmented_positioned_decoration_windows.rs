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

fn opaque_child_mask(frame: &Frame) -> Vec<bool> {
    frame
        .pixels
        .chunks_exact(4)
        .map(|px| px == [255, 0, 255, 255])
        .collect()
}

#[test]
fn split_positioned_inset_windows_preserve_transformed_child_ink_and_live_source_restore() {
    for scale in SCALES {
        for authored in [false, true] {
            let (mut e, positioned) = fixture(scale, authored, 0);
            let (initial, original_source) = observe(&mut e);
            assert!(original_source
                .iter()
                .all(|p| p.child_size == [30 * 64, 360 * 64]
                    && (p.principal == 0 || p.child_source_start == 10 * 64)));
            assert!(
                original_source
                    .iter()
                    .any(|p| p.principal > 0 && p.split > 0 && p.consumed > 0),
                "later negative-offset continuation must execute"
            );
            let mask = opaque_child_mask(&initial);
            assert!(
                mask.iter().any(|v| *v),
                "opaque transformed descendant must paint"
            );
            set(
                &mut e,
                positioned,
                RendererStyleValue::BackgroundColor(Color::TRANSPARENT),
                authored,
            );
            let (transparent, transparent_source) = observe(&mut e);
            assert_eq!(
                transparent_source, original_source,
                "own decoration mutation changed retained source topology"
            );
            assert_eq!(
                opaque_child_mask(&transparent),
                mask,
                "own decoration window changed child ink"
            );
            set(
                &mut e,
                positioned,
                RendererStyleValue::BackgroundColor(Color::from_rgba8(96, 48, 160, 255)),
                authored,
            );
            let (restored, restored_source) = observe(&mut e);
            assert_eq!(restored_source, original_source);
            assert!(
                restored.pixels == initial.pixels,
                "restoring own decoration changed the complete frame"
            );
            set(
                &mut e,
                positioned,
                RendererStyleValue::Top(Length::px(16.0)),
                authored,
            );
            let (_, changed) = observe(&mut e);
            assert!(changed.iter().all(|p| p.child_size == [30 * 64, 354 * 64]
                && (p.principal == 0 || p.child_source_start == 16 * 64)));
            set(
                &mut e,
                positioned,
                RendererStyleValue::Top(Length::px(10.0)),
                authored,
            );
            let (final_frame, final_source) = observe(&mut e);
            assert_eq!(final_source, original_source);
            assert!(
                final_frame.pixels == initial.pixels,
                "restoring true child source changed the complete frame"
            );
        }
    }
}

#[test]
fn split_positioned_beyond_cb_padding_clone_border_and_shadow_neighbors_retain_source_and_children()
{
    for scale in SCALES {
        for authored in [false, true] {
            for condition in 1..=9 {
                let (mut e, positioned) = fixture(scale, authored, condition);
                let (initial, original_source) = observe(&mut e);
                for p in &original_source {
                    assert_eq!(p.child_size[0], 30 * 64);
                    if condition == 1 {
                        assert_eq!(p.child_size[1], 430 * 64);
                        if p.principal > 0 {
                            assert_eq!(p.child_source_start, -10 * 64);
                            assert!(p.child_source_start + p.child_size[1] > p.cb_source_size);
                        }
                    } else {
                        let (start, insets) = if condition >= 6 {
                            (10 * 64 + 16, 40 * 64 + 48)
                        } else {
                            (10 * 64, 40 * 64)
                        };
                        if p.principal > 0 {
                            assert_eq!(p.child_source_start, start);
                        }
                        assert_eq!(
                            p.child_size[1],
                            p.cb_source_size - insets,
                            "used top+bottom resolve against the actual CB"
                        );
                    }
                    assert_eq!(p.child_coordinates.len(), 1);
                    assert_eq!(p.child_coordinates[0][2..], [12 * 64, 20 * 64]);
                }
                let mask = opaque_child_mask(&initial);
                assert!(mask.iter().any(|v| *v));
                set(
                    &mut e,
                    positioned,
                    RendererStyleValue::BackgroundColor(Color::TRANSPARENT),
                    authored,
                );
                let (transparent, source) = observe(&mut e);
                assert_eq!(source, original_source);
                assert_eq!(opaque_child_mask(&transparent), mask);
                set(
                    &mut e,
                    positioned,
                    RendererStyleValue::BackgroundColor(Color::from_rgba8(96, 48, 160, 255)),
                    authored,
                );
                let (restored, source) = observe(&mut e);
                assert_eq!(source, original_source);
                assert!(
                    restored.pixels == initial.pixels,
                    "fallback restoration condition={condition} @{scale}, authored={authored}"
                );
            }
        }
    }
}
