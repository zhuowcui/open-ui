//! Known enclosing fragmentainers keep independent principal, column and OOF
//! source sizes. The ordered replay below is byte-identical to the predecessor;
//! the direct fixture constructs the final DOM independently. Stock final
//! geometry and whole pixels are pinned in the v4610/v4618 evidence.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_engine::ViewportMetrics;
use openui_layout::{Fragment, FragmentKind};

const PROFILES: [(f64, f64, f64); 5] = [
    (800.0, 600.0, 1.0),
    (375.0, 667.0, 2.0),
    (1280.0, 720.0, 1.25),
    (1920.0, 1080.0, 1.5),
    (800.0, 600.0, 1.75),
];

// Generated pending native replay v1. No JavaScript runs in Engine.
use openui_dom::ElementTag;
use openui_engine::{Engine, EngineError, LifecycleStats, NodeHandle};
use openui_style::{parse_literal, StyleProperty};

pub struct ReplayResult {
    pub handles: std::collections::BTreeMap<String, NodeHandle>,
    pub barriers: Vec<(usize, LifecycleStats)>,
}

fn create_capture_node(engine: &mut Engine, tag: ElementTag) -> Result<NodeHandle, EngineError> {
    let node = engine.create_native_element(tag)?;
    // The immutable browser harness hides WebKit scrollbar pseudo-elements.
    // Authored standard scrollbar precedence is rejected by this cohort.
    set_literal(engine, node, StyleProperty::ScrollbarWidth, "none")?;
    Ok(node)
}

fn set_literal(
    engine: &mut Engine,
    node: NodeHandle,
    property: StyleProperty,
    literal: &str,
) -> Result<(), EngineError> {
    let value = parse_literal(property, literal)
        .map_err(|_| EngineError::InvalidInput("unsupported authored CSS literal"))?;
    engine.set_property(node, property, value)
}

fn replay_0(engine: &mut Engine, initial_barrier: bool) -> Result<ReplayResult, EngineError> {
    let html = create_capture_node(engine, ElementTag::Html)?;
    engine.append_child(engine.root(), html)?;
    set_literal(engine, html, StyleProperty::Overflow, "hidden")?;
    let node_0 = create_capture_node(engine, ElementTag::Body)?;
    let node_1 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_1, "class", "column")?;
    let node_2 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_2, "class", "column")?;
    let node_3 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_3, "id", "abscb")?;
    let node_4 = create_capture_node(engine, ElementTag::Div)?;
    let node_5 = create_capture_node(engine, ElementTag::Div)?;
    engine.append_child(node_0, node_1)?;
    engine.append_child(node_1, node_2)?;
    engine.append_child(node_2, node_3)?;
    engine.append_child(node_3, node_4)?;
    engine.append_child(node_3, node_5)?;
    engine.append_child(html, node_0)?;
    set_literal(engine, node_0, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_0, StyleProperty::Margin, "0")?;
    set_literal(engine, node_0, StyleProperty::Padding, "20px")?;
    set_literal(
        engine,
        node_0,
        StyleProperty::FontFamily,
        "DejaVu Sans, sans-serif",
    )?;
    set_literal(engine, node_0, StyleProperty::FontSize, "16px")?;
    set_literal(engine, node_0, StyleProperty::Color, "black")?;
    set_literal(engine, node_0, StyleProperty::BackgroundColor, "white")?;
    set_literal(engine, node_1, StyleProperty::Margin, "0")?;
    set_literal(engine, node_1, StyleProperty::Padding, "0")?;
    set_literal(engine, node_1, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_1, StyleProperty::ColumnCount, "2")?;
    set_literal(engine, node_1, StyleProperty::ColumnFill, "auto")?;
    set_literal(engine, node_1, StyleProperty::ColumnGap, "0")?;
    set_literal(engine, node_1, StyleProperty::Width, "200px")?;
    set_literal(engine, node_1, StyleProperty::Height, "100px")?;
    set_literal(engine, node_2, StyleProperty::Margin, "0")?;
    set_literal(engine, node_2, StyleProperty::Padding, "0")?;
    set_literal(engine, node_2, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_2, StyleProperty::ColumnCount, "2")?;
    set_literal(engine, node_2, StyleProperty::ColumnFill, "auto")?;
    set_literal(engine, node_2, StyleProperty::ColumnGap, "0")?;
    set_literal(engine, node_3, StyleProperty::Margin, "0")?;
    set_literal(engine, node_3, StyleProperty::Padding, "0")?;
    set_literal(engine, node_3, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_3, StyleProperty::Position, "relative")?;
    set_literal(engine, node_3, StyleProperty::Width, "100%")?;
    set_literal(engine, node_3, StyleProperty::Height, "50px")?;
    set_literal(engine, node_3, StyleProperty::BackgroundColor, "red")?;
    set_literal(engine, node_4, StyleProperty::Margin, "0")?;
    set_literal(engine, node_4, StyleProperty::Padding, "0")?;
    set_literal(engine, node_4, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_4, StyleProperty::Width, "100%")?;
    set_literal(engine, node_4, StyleProperty::BackgroundColor, "green")?;
    set_literal(engine, node_4, StyleProperty::Height, "50px")?;
    set_literal(engine, node_5, StyleProperty::Margin, "0")?;
    set_literal(engine, node_5, StyleProperty::Padding, "0")?;
    set_literal(engine, node_5, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_5, StyleProperty::Width, "100%")?;
    set_literal(engine, node_5, StyleProperty::BackgroundColor, "green")?;
    set_literal(engine, node_5, StyleProperty::Position, "absolute")?;
    set_literal(engine, node_5, StyleProperty::Height, "90px")?;
    let mut barriers = Vec::new();
    // AST operation 0: layout-barrier
    if initial_barrier {
        let _ = engine.update()?;
        barriers.push((0, engine.stats()));
    }
    // AST operation 1: create-element
    let node_6 = create_capture_node(engine, ElementTag::Div)?;
    // AST operation 2: set-style
    set_literal(engine, node_6, StyleProperty::Height, "60px")?;
    // AST operation 3: prepend
    let before_3 = engine
        .children(node_3)?
        .into_iter()
        .find(|node| ![node_6].contains(node));
    if let Some(before) = before_3 {
        engine.insert_before(node_3, node_6, before)?;
    } else {
        engine.append_or_move_child(node_3, node_6)?;
    }
    set_literal(engine, node_6, StyleProperty::Margin, "0")?;
    set_literal(engine, node_6, StyleProperty::Padding, "0")?;
    set_literal(engine, node_6, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_6, StyleProperty::Width, "100%")?;
    set_literal(engine, node_6, StyleProperty::BackgroundColor, "green")?;
    set_literal(engine, node_6, StyleProperty::Height, "60px")?;
    let mut handles = std::collections::BTreeMap::new();
    handles.insert("abscb".to_string(), node_3);
    handles.insert("binding:newDiv".to_string(), node_6);
    Ok(ReplayResult { handles, barriers })
}

struct Fixture {
    engine: Engine,
    outer: NodeHandle,
    inner: NodeHandle,
    cb: NodeHandle,
    oof: Option<NodeHandle>,
}
fn only(e: &Engine, n: NodeHandle) -> NodeHandle {
    let c = e.children(n).unwrap();
    assert_eq!(c.len(), 1);
    c[0]
}
fn identify(
    mut e: Engine,
    outer: NodeHandle,
    inner: NodeHandle,
    cb: NodeHandle,
    oof: Option<NodeHandle>,
) -> Fixture {
    for (n, id) in [
        (outer, "outer-source"),
        (inner, "inner-source"),
        (cb, "abscb"),
    ] {
        e.set_attribute(n, "id", id).unwrap();
    }
    if let Some(n) = oof {
        e.set_attribute(n, "id", "oof-source").unwrap();
    }
    Fixture {
        engine: e,
        outer,
        inner,
        cb,
        oof,
    }
}
fn ordered(w: f64, h: f64, scale: f64) -> Fixture {
    let mut e = Engine::new(ViewportMetrics::from_logical_size(w, h, scale).unwrap()).unwrap();
    let replay = replay_0(&mut e, true).unwrap();
    assert_eq!(replay.barriers.len(), 1);
    let html = only(&e, e.root());
    let body = only(&e, html);
    let outer = only(&e, body);
    let inner = only(&e, outer);
    let cb = only(&e, inner);
    let oof = e
        .children(cb)
        .unwrap()
        .into_iter()
        .find(|n| {
            e.computed_style(*n)
                .unwrap()
                .position
                .is_absolutely_positioned()
        })
        .unwrap();
    identify(e, outer, inner, cb, Some(oof))
}
fn lit(e: &mut Engine, n: NodeHandle, p: StyleProperty, v: &str) {
    set_literal(e, n, p, v).unwrap();
}
fn clean(e: &mut Engine, n: NodeHandle) {
    for (p, v) in [
        (StyleProperty::Margin, "0"),
        (StyleProperty::Padding, "0"),
        (StyleProperty::BoxSizing, "content-box"),
    ] {
        lit(e, n, p, v);
    }
}
// Independent final-state construction: no initial DOM, prepend or first barrier.
fn direct(
    w: f64,
    h: f64,
    scale: f64,
    vertical: bool,
    mode: &str,
    rtl: bool,
    heights: &[f64],
    include_oof: bool,
    prefix: f64,
) -> Fixture {
    let mut e = Engine::new(ViewportMetrics::from_logical_size(w, h, scale).unwrap()).unwrap();
    let html = create_capture_node(&mut e, ElementTag::Html).unwrap();
    let root = e.root();
    e.append_child(root, html).unwrap();
    lit(&mut e, html, StyleProperty::Overflow, "hidden");
    let body = create_capture_node(&mut e, ElementTag::Body).unwrap();
    clean(&mut e, body);
    lit(&mut e, body, StyleProperty::Padding, "20px");
    lit(&mut e, body, StyleProperty::BackgroundColor, "white");
    let outer = create_capture_node(&mut e, ElementTag::Div).unwrap();
    clean(&mut e, outer);
    lit(&mut e, outer, StyleProperty::WritingMode, mode);
    lit(
        &mut e,
        outer,
        StyleProperty::Direction,
        if rtl { "rtl" } else { "ltr" },
    );
    lit(
        &mut e,
        outer,
        StyleProperty::Width,
        if vertical { "100px" } else { "200px" },
    );
    lit(
        &mut e,
        outer,
        StyleProperty::Height,
        if vertical { "200px" } else { "100px" },
    );
    let inner = create_capture_node(&mut e, ElementTag::Div).unwrap();
    clean(&mut e, inner);
    for n in [outer, inner] {
        lit(&mut e, n, StyleProperty::ColumnCount, "2");
        lit(&mut e, n, StyleProperty::ColumnFill, "auto");
        lit(&mut e, n, StyleProperty::ColumnGap, "0");
    }
    let cb = create_capture_node(&mut e, ElementTag::Div).unwrap();
    clean(&mut e, cb);
    lit(&mut e, cb, StyleProperty::Position, "relative");
    let inline = if vertical {
        StyleProperty::Height
    } else {
        StyleProperty::Width
    };
    let block = if vertical {
        StyleProperty::Width
    } else {
        StyleProperty::Height
    };
    lit(&mut e, cb, inline, "100%");
    lit(&mut e, cb, block, "50px");
    lit(&mut e, cb, StyleProperty::BackgroundColor, "red");
    e.append_child(html, body).unwrap();
    e.append_child(body, outer).unwrap();
    if prefix != 0.0 {
        let n = create_capture_node(&mut e, ElementTag::Div).unwrap();
        clean(&mut e, n);
        lit(&mut e, n, block, &format!("{prefix}px"));
        e.append_child(outer, n).unwrap();
    }
    e.append_child(outer, inner).unwrap();
    e.append_child(inner, cb).unwrap();
    for height in heights {
        let n = create_capture_node(&mut e, ElementTag::Div).unwrap();
        clean(&mut e, n);
        lit(&mut e, n, inline, "100%");
        lit(&mut e, n, block, &format!("{height}px"));
        lit(&mut e, n, StyleProperty::BackgroundColor, "green");
        e.append_child(cb, n).unwrap();
    }
    let oof = include_oof.then(|| {
        let n = create_capture_node(&mut e, ElementTag::Div).unwrap();
        clean(&mut e, n);
        lit(&mut e, n, inline, "100%");
        lit(&mut e, n, block, "90px");
        lit(&mut e, n, StyleProperty::Position, "absolute");
        lit(&mut e, n, StyleProperty::BackgroundColor, "green");
        e.append_child(cb, n).unwrap();
        n
    });
    identify(e, outer, inner, cb, oof)
}
fn owners<'a>(f: &'a Fragment, e: &Engine, id: &str, out: &mut Vec<&'a Fragment>) {
    if f.owns_element_box()
        && e.native_document()
            .node(f.node_id)
            .attributes
            .get("id")
            .map(String::as_str)
            == Some(id)
    {
        out.push(f);
    }
    for c in &f.children {
        owners(c, e, id, out);
    }
}
fn collect<'a>(s: &'a SceneSnapshot, e: &Engine, id: &str) -> Vec<&'a Fragment> {
    let mut v = Vec::new();
    owners(s.fragments(), e, id, &mut v);
    v
}
fn check_capacity(f: &mut Fixture, capacity: f64, principal: f64, vertical: bool) -> SceneSnapshot {
    let s = f.engine.scene().unwrap();
    let parts = collect(&s, &f.engine, "inner-source");
    assert_eq!(
        parts.len(),
        1,
        "one actual inner source row, not a repeated completed row"
    );
    let p = parts[0];
    assert_eq!(
        p.multicol_fragmentation
            .unwrap()
            .fragmentainer_block_size
            .to_f64(),
        capacity
    );
    let r = f.engine.bounds(f.inner).unwrap().unwrap();
    assert_eq!(
        (f64::from(r.width), f64::from(r.height)),
        if vertical {
            (principal, 100.0)
        } else {
            (100.0, principal)
        }
    );
    let columns: Vec<_> = p
        .children
        .iter()
        .filter(|c| c.kind == FragmentKind::ColumnBox)
        .collect();
    assert!(!columns.is_empty());
    for c in columns {
        assert_eq!(
            (c.size.width.to_f64(), c.size.height.to_f64()),
            if vertical {
                (capacity, 50.0)
            } else {
                (50.0, capacity)
            }
        );
    }
    let cb = collect(&s, &f.engine, "abscb");
    assert!(!cb.is_empty());
    for c in cb {
        if let Some(g) = c.box_geometry {
            assert_eq!(
                (g.source_size.width.to_f64(), g.source_size.height.to_f64()),
                (50.0, 50.0)
            );
        }
    }
    s
}
fn raster(s: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(s).unwrap()
}
// Exact Chromium RGBA8 crops and the independently verified opaque-white
// exterior. Provenance and unmodified source PNG hashes live beside the bytes.
fn exact_chromium_final_frame(frame: &Frame, scale: f64) {
    let (side, width, height, pixels): (usize, u32, u32, &[u8]) = match scale {
        1.0 => (
            100,
            800,
            600,
            include_bytes!("fixtures/nested-multicol-chromium-oracle/scale-1_0.rgba"),
        ),
        2.0 => (
            200,
            750,
            1334,
            include_bytes!("fixtures/nested-multicol-chromium-oracle/scale-2_0.rgba"),
        ),
        1.25 => (
            125,
            1600,
            900,
            include_bytes!("fixtures/nested-multicol-chromium-oracle/scale-1_25.rgba"),
        ),
        1.5 => (
            150,
            2880,
            1620,
            include_bytes!("fixtures/nested-multicol-chromium-oracle/scale-1_5.rgba"),
        ),
        1.75 => (
            175,
            1400,
            1050,
            include_bytes!("fixtures/nested-multicol-chromium-oracle/scale-1_75.rgba"),
        ),
        _ => panic!("no pinned Chromium oracle for scale {scale}"),
    };
    assert_eq!(
        (frame.width, frame.height),
        (width, height),
        "pinned Chromium viewport @{scale}"
    );
    assert_eq!(pixels.len(), side * side * 4);
    let start = (20.0 * scale) as usize;
    let end = start + side;
    for y in 0..frame.height as usize {
        for x in 0..frame.width as usize {
            let off = y * frame.stride as usize + x * 4;
            let wanted = if x >= start && x < end && y >= start && y < end {
                let source = ((y - start) * side + x - start) * 4;
                &pixels[source..source + 4]
            } else {
                &[255, 255, 255, 255]
            };
            assert_eq!(
                &frame.pixels[off..off + 4],
                wanted,
                "whole-image Chromium oracle {x},{y} @{scale}"
            );
        }
    }
}
#[test]
fn ordered_and_independently_built_final_state_own_one_complete_inner_row() {
    for (w, h, scale) in PROFILES {
        let mut a = ordered(w, h, scale);
        let mut b = direct(
            w,
            h,
            scale,
            false,
            "horizontal-tb",
            false,
            &[60.0, 50.0],
            true,
            0.0,
        );
        let sa = check_capacity(&mut a, 100.0, 100.0, false);
        let sb = check_capacity(&mut b, 100.0, 100.0, false);
        for f in [&mut a, &mut b] {
            let oof = f.oof.unwrap();
            let rect = f.engine.bounds(oof).unwrap().unwrap();
            assert_eq!(
                (rect.x, rect.y, rect.width, rect.height),
                (70.0, 30.0, 50.0, 90.0)
            );
            let rectangles = f.engine.client_rects(oof).unwrap();
            assert_eq!(rectangles.len(), 1);
            let r = rectangles[0];
            assert_eq!(
                (
                    f64::from(r.x),
                    f64::from(r.y),
                    f64::from(r.width),
                    f64::from(r.height)
                ),
                (70.0, 30.0, 50.0, 90.0)
            );
            let s = f.engine.scene().unwrap();
            let active = collect(&s, &f.engine, "oof-source");
            assert_eq!(active.len(), 1, "one OOF source interval");
            let p = active[0].positioned_fragmentation.unwrap();
            assert_eq!(p.static_position.top.to_f64(), 110.0);
            assert_eq!(
                (
                    p.containing_block_size.width.to_f64(),
                    p.containing_block_size.height.to_f64()
                ),
                (50.0, 50.0)
            );
            assert_eq!(p.fragmentainer_index, Some(1));
        }
        let fa = raster(&sa);
        let fb = raster(&sb);
        assert_eq!(fa.pixels, fb.pixels, "ordered and final @{scale}");
        exact_chromium_final_frame(&fa, scale);
        let generation = sa.generation();
        let stats = a.engine.stats();
        assert_eq!(a.engine.scene().unwrap().generation(), generation);
        assert_eq!(a.engine.stats(), stats, "true unchanged frame");
    }
}
#[test]
fn short_principal_and_short_first_remainder_keep_independent_column_capacity() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for (mode, vertical) in [
            ("horizontal-tb", false),
            ("vertical-rl", true),
            ("vertical-lr", true),
        ] {
            for rtl in [false, true] {
                let mut short = direct(
                    800.0,
                    600.0,
                    scale,
                    vertical,
                    mode,
                    rtl,
                    &[50.0],
                    false,
                    0.0,
                );
                check_capacity(&mut short, 100.0, 50.0, vertical);
                let mut remainder = direct(
                    800.0,
                    600.0,
                    scale,
                    vertical,
                    mode,
                    rtl,
                    &[60.0, 50.0],
                    false,
                    30.0,
                );
                let s = check_capacity(&mut remainder, 70.0, 70.0, vertical);
                let p = collect(&s, &remainder.engine, "inner-source")[0];
                assert_eq!(
                    p.multicol_fragmentation
                        .unwrap()
                        .continuation_fragmentainer_block_size
                        .to_f64(),
                    100.0
                );
            }
        }
    }
}
#[test]
fn real_translation_and_authored_overflow_do_not_become_a_zero_offset_keyword_clip() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut f = direct(
            800.0,
            600.0,
            scale,
            false,
            "horizontal-tb",
            false,
            &[60.0, 50.0],
            true,
            0.0,
        );
        let s = check_capacity(&mut f, 100.0, 100.0, false);
        let original = raster(&s);
        for top in ["11px", "-11px", "0px"] {
            lit(&mut f.engine, f.cb, StyleProperty::Top, top);
            let scene = f.engine.scene().unwrap();
            let parts = collect(&scene, &f.engine, "abscb");
            assert!(!parts.is_empty());
            for p in parts {
                if let Some(g) = p.box_geometry {
                    assert_eq!(g.source_border_box_rect.size.height.to_f64(), 50.0);
                }
            }
            if top == "0px" {
                assert_eq!(
                    raster(&scene).pixels,
                    original.pixels,
                    "cancel genuine movement @{scale}"
                );
            }
        }
        lit(&mut f.engine, f.cb, StyleProperty::Overflow, "hidden");
        let scene = f.engine.scene().unwrap();
        assert!(collect(&scene, &f.engine, "abscb")
            .iter()
            .any(|p| p.has_overflow_clip));
        lit(&mut f.engine, f.cb, StyleProperty::Overflow, "visible");
        let scene = f.engine.scene().unwrap();
        assert_eq!(
            raster(&scene).pixels,
            original.pixels,
            "restore authored overflow @{scale}"
        );
    }
}

#[test]
fn collapsed_sibling_margin_is_part_of_the_actual_remaining_capacity() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for (margin, capacity) in [("20px", 50.0), ("-20px", 90.0)] {
            let mut f = direct(
                800.0,
                600.0,
                scale,
                false,
                "horizontal-tb",
                false,
                &[20.0],
                false,
                30.0,
            );
            lit(&mut f.engine, f.inner, StyleProperty::MarginTop, margin);
            let scene = check_capacity(&mut f, capacity, 50.0, false);
            let p = collect(&scene, &f.engine, "inner-source")[0];
            assert_eq!(
                p.multicol_fragmentation
                    .unwrap()
                    .continuation_fragmentainer_block_size
                    .to_f64(),
                100.0
            );
        }
    }
}

#[test]
fn avoided_leading_unit_uses_a_fresh_outer_column_before_remaining_capacity_relayout() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut f = direct(
            800.0,
            600.0,
            scale,
            false,
            "horizontal-tb",
            false,
            &[80.0],
            false,
            30.0,
        );
        lit(&mut f.engine, f.cb, StyleProperty::Height, "auto");
        lit(&mut f.engine, f.cb, StyleProperty::BreakInside, "avoid");
        let scene = f.engine.scene().unwrap();
        let bounds = f.engine.bounds(f.inner).unwrap().unwrap();
        assert_eq!(
            (bounds.x, bounds.y, bounds.width, bounds.height),
            (120.0, 20.0, 100.0, 80.0)
        );
        let inner = collect(&scene, &f.engine, "inner-source");
        assert_eq!(inner.len(), 1);
        assert_eq!(
            inner[0]
                .multicol_fragmentation
                .unwrap()
                .fragmentainer_block_size
                .to_f64(),
            100.0
        );
    }
}

#[test]
fn vertical_owned_oof_and_actual_additional_rows_keep_distinct_source_intervals() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for mode in ["vertical-rl", "vertical-lr"] {
            for rtl in [false, true] {
                let mut f = direct(
                    800.0,
                    600.0,
                    scale,
                    true,
                    mode,
                    rtl,
                    &[60.0, 50.0],
                    true,
                    0.0,
                );
                let s = check_capacity(&mut f, 100.0, 100.0, true);
                let oof = collect(&s, &f.engine, "oof-source");
                assert_eq!(
                    oof.len(),
                    1,
                    "one owned mapped interval {mode}/{rtl} @{scale}"
                );
                assert_eq!(
                    oof[0].positioned_fragmentation.unwrap().fragmentainer_index,
                    Some(1)
                );
                let r = f.engine.bounds(f.oof.unwrap()).unwrap().unwrap();
                assert_eq!((r.width, r.height), (90.0, 50.0));
                let generation = s.generation();
                let stats = f.engine.stats();
                assert_eq!(f.engine.scene().unwrap().generation(), generation);
                assert_eq!(f.engine.stats(), stats);

                // 300px normal flow needs three 100px inner columns, hence
                // a real second declared-column row in the outer context.
                // The earlier OOF still belongs to its one source interval.
                let tail = create_capture_node(&mut f.engine, ElementTag::Div).unwrap();
                clean(&mut f.engine, tail);
                lit(&mut f.engine, tail, StyleProperty::Width, "190px");
                lit(&mut f.engine, tail, StyleProperty::Height, "100%");
                f.engine.append_child(f.cb, tail).unwrap();
                let s = f.engine.scene().unwrap();
                let rows = collect(&s, &f.engine, "inner-source");
                assert_eq!(rows.len(), 2, "actual additional row {mode}/{rtl} @{scale}");
                assert!(
                    rows.iter().any(|part| part.box_geometry.is_some_and(|g| {
                        g.continuation_record && g.consumed_block_size.to_f64() == 100.0
                    })),
                    "a consumed source row, not a cloned completed row"
                );
                assert_eq!(collect(&s, &f.engine, "oof-source").len(), 1);
            }
        }
    }
}
