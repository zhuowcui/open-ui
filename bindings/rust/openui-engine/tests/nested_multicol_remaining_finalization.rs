//! Remaining-capacity relayout preserves the completed nested source pipeline.
//! These are native geometry and restoration guards; no new pixel oracle is authored.
use openui_compositor::{Frame, SoftwareCompositor};
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineOptions, NodeHandle, ViewportMetrics};
use openui_geometry::{Length, RasterConfiguration};
use openui_layout::{Fragment, FragmentKind};
use openui_style::*;

const SCALES: [f64; 5] = [1.0, 1.25, 1.5, 1.75, 2.0];

fn set(e: &mut Engine, node: NodeHandle, value: RendererStyleValue, authored: bool) {
    if authored {
        e.set_property(node, value.property(), StyleValue::Renderer(value))
            .unwrap();
    } else {
        e.set_renderer_style(node, value).unwrap();
    }
}

fn element(e: &mut Engine, parent: NodeHandle, tag: ElementTag, authored: bool) -> NodeHandle {
    let node = e.create_native_element(tag).unwrap();
    e.append_child(parent, node).unwrap();
    for value in [
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
        RendererStyleValue::Orphans(1),
        RendererStyleValue::Widows(1),
    ] {
        set(e, node, value, authored);
    }
    node
}

fn fixture(scale: f64, authored: bool) -> (Engine, NodeHandle, NodeHandle) {
    let mut e = Engine::new_with_font_collection_and_options(
        ViewportMetrics::from_logical_size(240.0, 180.0, scale).unwrap(),
        openui_text::FontCollection::deterministic_test(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
            ..EngineOptions::default()
        },
    )
    .unwrap();
    let root = e.root();
    for value in [
        RendererStyleValue::ScrollbarWidth(ScrollbarWidth::None),
        RendererStyleValue::BackgroundColor(Color::WHITE),
        RendererStyleValue::FontFamily(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".to_string()),
                FontFamily::Named("DejaVu Sans".to_string()),
            ],
        }),
        RendererStyleValue::FontSize(16.0),
    ] {
        set(&mut e, root, value, authored);
    }
    let body = element(&mut e, root, ElementTag::Div, authored);
    for value in [
        RendererStyleValue::PaddingTop(Length::px(20.0)),
        RendererStyleValue::PaddingRight(Length::px(20.0)),
        RendererStyleValue::PaddingBottom(Length::px(20.0)),
        RendererStyleValue::PaddingLeft(Length::px(20.0)),
    ] {
        set(&mut e, body, value, authored);
    }
    let backing = element(&mut e, body, ElementTag::Div, authored);
    for value in [
        RendererStyleValue::Width(Length::px(100.0)),
        RendererStyleValue::Height(Length::px(100.0)),
        RendererStyleValue::BackgroundColor(Color::RED),
    ] {
        set(&mut e, backing, value, authored);
    }
    let outer = element(&mut e, backing, ElementTag::Div, authored);
    for value in [
        RendererStyleValue::Width(Length::px(50.0)),
        RendererStyleValue::Height(Length::px(100.0)),
        RendererStyleValue::ColumnCount(Some(1)),
        RendererStyleValue::ColumnGap(Some(Length::px(0.0))),
        RendererStyleValue::RowGap(Some(Length::px(0.0))),
        RendererStyleValue::ColumnFill(ColumnFill::Auto),
        RendererStyleValue::BackgroundColor(Color::GREEN),
    ] {
        set(&mut e, outer, value, authored);
    }
    let prefix = element(&mut e, outer, ElementTag::Div, authored);
    set(
        &mut e,
        prefix,
        RendererStyleValue::Height(Length::px(5.0)),
        authored,
    );
    let nested = element(&mut e, outer, ElementTag::Div, authored);
    e.set_attribute(nested, "id", "nested-source").unwrap();
    for value in [
        RendererStyleValue::Width(Length::px(11.0)),
        RendererStyleValue::ColumnCount(Some(2)),
        RendererStyleValue::ColumnGap(Some(Length::px(0.0))),
        RendererStyleValue::RowGap(Some(Length::px(0.0))),
        RendererStyleValue::ColumnFill(ColumnFill::Auto),
    ] {
        set(&mut e, nested, value, authored);
    }
    let span = element(&mut e, nested, ElementTag::Span, authored);
    set(
        &mut e,
        span,
        RendererStyleValue::Position(Position::Relative),
        authored,
    );
    let text = element(&mut e, span, ElementTag::Text, authored);
    e.set_text(text, "\n      \u{a0}\n      ").unwrap();
    let atomic = element(&mut e, span, ElementTag::Div, authored);
    for value in [
        RendererStyleValue::Display(Display::InlineBlock),
        RendererStyleValue::Width(Length::px(50.0)),
        RendererStyleValue::Height(Length::px(100.0)),
        RendererStyleValue::BackgroundColor(Color::GREEN),
    ] {
        set(&mut e, atomic, value, authored);
    }
    (e, prefix, nested)
}

#[derive(Debug, PartialEq, Eq)]
struct SourceRecord {
    width: i32,
    source_block: i32,
    source_offset: i32,
    margins: [i32; 2],
    first: bool,
    columns: Vec<[i32; 4]>,
}

fn collect(f: &Fragment, e: &Engine, records: &mut Vec<SourceRecord>) {
    if f.kind == FragmentKind::Box
        && f.owns_element_box()
        && !f.node_id.is_none()
        && e.native_document()
            .node(f.node_id)
            .attributes
            .get("id")
            .is_some_and(|id| id == "nested-source")
    {
        let slice = f.decoration_slice;
        records.push(SourceRecord {
            width: f.size.width.raw(),
            source_block: slice.map_or(f.size.height.raw(), |s| s.source_block_size.raw()),
            source_offset: slice.map_or(0, |s| s.source_block_offset.raw()),
            margins: [f.margin.left.raw(), f.margin.right.raw()],
            first: f.is_first_for_node,
            columns: f
                .children
                .iter()
                .filter(|c| c.kind == FragmentKind::ColumnBox)
                .map(|c| {
                    [
                        c.offset.left.raw(),
                        c.offset.top.raw(),
                        c.size.width.raw(),
                        c.size.height.raw(),
                    ]
                })
                .collect(),
        });
    }
    for child in &f.children {
        collect(child, e, records);
    }
}

fn observe(e: &mut Engine, nested: NodeHandle) -> (Frame, Vec<SourceRecord>) {
    e.set_layered_paint_capture_enabled(false);
    let ordinary = e.scene().unwrap();
    let frame = SoftwareCompositor::default().render(&ordinary).unwrap();
    let mut records = Vec::new();
    collect(ordinary.fragments(), e, &mut records);
    assert!(
        !records.is_empty(),
        "the actual nested source must be retained"
    );
    e.set_layered_paint_capture_enabled(true);
    let captured = e.scene().unwrap();
    assert!(captured.layered_paint_capture().is_some());
    let instrumented = SoftwareCompositor::default().render(&captured).unwrap();
    assert_eq!(
        (frame.width, frame.height, frame.stride),
        (instrumented.width, instrumented.height, instrumented.stride)
    );
    assert!(
        frame.pixels == instrumented.pixels,
        "capture changed the entire canonical frame"
    );
    let stats = e.stats();
    assert_eq!(e.scene().unwrap().generation(), captured.generation());
    assert!(!e.client_rects(nested).unwrap().is_empty());
    assert_eq!(
        e.stats(),
        stats,
        "cached scene and owned geometry query performed work"
    );
    assert_eq!(e.element_by_id("nested-source"), Some(nested));
    (frame, records)
}

fn assert_sequential_source(records: &[SourceRecord]) {
    let first = records
        .iter()
        .find(|r| r.first)
        .expect("actual first source piece");
    assert_eq!(first.width, 11 * 64);
    assert_eq!(first.source_offset, 0);
    assert_eq!(
        first.columns.len(),
        2,
        "both actual inline tokens must survive"
    );
    let leading = first.columns[0];
    let atomic = first.columns[1];
    assert_eq!([leading[0], leading[1], atomic[0]], [0, 0, 0]);
    assert!(leading[3] > 0);
    assert_eq!(
        atomic[1], leading[3],
        "the skipped atomic line resumes after the leading source token"
    );
    assert_eq!(first.source_block, leading[3] + atomic[3]);
    assert!(
        first.source_block > atomic[3],
        "parallel column height is not complete sequential source progress"
    );
    assert!(records.iter().all(|r| r.source_block == first.source_block));
}

#[test]
fn remaining_capacity_keeps_sequential_oversized_inline_source_and_parent_margins() {
    for scale in SCALES {
        for authored in [false, true] {
            let (mut e, _, nested) = fixture(scale, authored);
            let (_, records) = observe(&mut e, nested);
            assert_sequential_source(&records);
            assert!(records.iter().all(|r| r.margins == [0, 39 * 64]));
        }
    }
}

#[test]
fn remaining_capacity_margin_context_survives_rtl_and_authored_routes() {
    for scale in SCALES {
        for authored in [false, true] {
            for (rtl, automatic, start) in
                [(false, false, 1.25), (false, true, 0.0), (true, false, 0.0)]
            {
                let (mut e, _, nested) = fixture(scale, authored);
                if rtl {
                    let root = e.root();
                    set(
                        &mut e,
                        root,
                        RendererStyleValue::Direction(Direction::Rtl),
                        authored,
                    );
                }
                if automatic {
                    set(
                        &mut e,
                        nested,
                        RendererStyleValue::MarginLeft(Length::auto()),
                        authored,
                    );
                    set(
                        &mut e,
                        nested,
                        RendererStyleValue::MarginRight(Length::auto()),
                        authored,
                    );
                } else if start != 0.0 {
                    set(
                        &mut e,
                        nested,
                        RendererStyleValue::MarginLeft(Length::px(start)),
                        authored,
                    );
                }
                let (_, records) = observe(&mut e, nested);
                let root = e.root();
                let outer = e.parent(nested).unwrap().expect("actual outer parent");
                assert_eq!(
                    e.computed_style(root).unwrap().direction,
                    if rtl { Direction::Rtl } else { Direction::Ltr },
                );
                // Renderer inputs are per-element computed snapshots. A later
                // parent mutation does not rewrite their inherited fields;
                // native authored declarations retain normal cascade behavior.
                let child_direction = if authored && rtl {
                    Direction::Rtl
                } else {
                    Direction::Ltr
                };
                assert_eq!(e.computed_style(outer).unwrap().direction, child_direction);
                assert_eq!(e.computed_style(nested).unwrap().direction, child_direction);
                let expected = if automatic {
                    [39 * 32, 39 * 32]
                } else if rtl {
                    if authored {
                        [39 * 64, 0]
                    } else {
                        [0, 39 * 64]
                    }
                } else {
                    [80, 39 * 64 - 80]
                };
                assert!(
                    records
                        .iter()
                        .all(|r| r.width == 11 * 64 && r.margins == expected),
                    "parent-column used margins @{scale}: {records:?}"
                );
            }
        }
    }
}

#[test]
fn remaining_capacity_live_prefix_and_balance_restoration_preserve_capture_and_idle() {
    for scale in SCALES {
        for authored in [false, true] {
            let (mut e, prefix, nested) = fixture(scale, authored);
            let (original, original_source) = observe(&mut e, nested);
            assert_sequential_source(&original_source);
            set(
                &mut e,
                prefix,
                RendererStyleValue::Height(Length::px(0.0)),
                authored,
            );
            let _ = observe(&mut e, nested);
            set(
                &mut e,
                prefix,
                RendererStyleValue::Height(Length::px(5.0)),
                authored,
            );
            let (restored, restored_source) = observe(&mut e, nested);
            assert_eq!(restored_source, original_source);
            assert!(
                restored.pixels == original.pixels,
                "prefix restore changed the whole frame @{scale}"
            );
            set(
                &mut e,
                nested,
                RendererStyleValue::ColumnFill(ColumnFill::Balance),
                authored,
            );
            let (_, balanced_source) = observe(&mut e, nested);
            assert!(balanced_source
                .iter()
                .all(|r| r.width == 11 * 64 && r.margins == [0, 39 * 64]));
            set(
                &mut e,
                nested,
                RendererStyleValue::ColumnFill(ColumnFill::Auto),
                authored,
            );
            let (final_frame, final_source) = observe(&mut e, nested);
            assert_eq!(final_source, original_source);
            assert!(
                final_frame.pixels == original.pixels,
                "Auto/Balance restore changed the whole frame @{scale}"
            );
        }
    }
}

#[test]
fn remaining_capacity_explicit_renderer_rtl_snapshots_keep_parent_used_margins() {
    for scale in SCALES {
        let (mut e, _, nested) = fixture(scale, false);
        let root = e.root();
        let outer = e.parent(nested).unwrap().expect("actual outer parent");
        // This caller supplies complete RTL snapshots to both real boxes.
        // It does not rely on a later root setter changing child snapshots.
        set(
            &mut e,
            outer,
            RendererStyleValue::Direction(Direction::Rtl),
            false,
        );
        set(
            &mut e,
            nested,
            RendererStyleValue::Direction(Direction::Rtl),
            false,
        );
        let (_, records) = observe(&mut e, nested);
        assert_eq!(e.computed_style(root).unwrap().direction, Direction::Ltr);
        assert_eq!(e.computed_style(outer).unwrap().direction, Direction::Rtl);
        assert_eq!(e.computed_style(nested).unwrap().direction, Direction::Rtl);
        assert!(
            records
                .iter()
                .all(|record| record.width == 11 * 64 && record.margins == [39 * 64, 0]),
            "explicit renderer RTL used margins @{scale}: {records:?}",
        );
    }
}
