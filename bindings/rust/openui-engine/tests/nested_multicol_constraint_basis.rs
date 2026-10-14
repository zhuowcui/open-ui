//! Native nested multicol sizing uses the parent column's available inline
//! size and percentage basis. The original ordered witness and final DOM are
//! preserved; saved stock PNG comparison remains the renderer gate.
use openui_compositor::{Frame, SceneSnapshot, SoftwareCompositor};
use openui_engine::ViewportMetrics;
use openui_layout::{Fragment, FragmentKind};

const PROFILES: [(f64, f64, f64); 4] = [
    (800.0, 600.0, 1.0),
    (375.0, 667.0, 2.0),
    (1280.0, 720.0, 1.25),
    (1920.0, 1080.0, 1.5),
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
    barrier_count: usize,
}

fn only_child(engine: &Engine, parent: NodeHandle) -> NodeHandle {
    let children = engine.children(parent).unwrap();
    assert_eq!(children.len(), 1);
    children[0]
}

fn fixture(width: f64, height: f64, scale: f64, initial_barrier: bool) -> Fixture {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(width, height, scale).unwrap()).unwrap();
    let replay = replay_0(&mut engine, initial_barrier).unwrap();
    let html = only_child(&engine, engine.root());
    let body = only_child(&engine, html);
    let outer = only_child(&engine, body);
    let inner = only_child(&engine, outer);
    let cb = only_child(&engine, inner);
    assert_eq!(replay.handles["abscb"], cb);
    // Debug role attributes affect no selector/property in this native tree.
    engine
        .set_attribute(outer, "id", "constraint-outer")
        .unwrap();
    engine
        .set_attribute(inner, "id", "constraint-inner")
        .unwrap();
    Fixture {
        engine,
        outer,
        inner,
        cb,
        barrier_count: replay.barriers.len(),
    }
}

fn owners<'a>(fragment: &'a Fragment, engine: &Engine, id: &str, out: &mut Vec<&'a Fragment>) {
    if fragment.owns_element_box()
        && engine
            .native_document()
            .node(fragment.node_id)
            .attributes
            .get("id")
            .map(String::as_str)
            == Some(id)
    {
        out.push(fragment);
    }
    for child in &fragment.children {
        owners(child, engine, id, out);
    }
}

fn check_basis(
    f: &mut Fixture,
    expected_inner: f64,
    expected_column: f64,
    authored_clip: bool,
) -> SceneSnapshot {
    let scene = f.engine.scene().unwrap();
    let mut inner = Vec::new();
    owners(scene.fragments(), &f.engine, "constraint-inner", &mut inner);
    assert!(!inner.is_empty(), "actual owned inner fragments");
    let mut saw_column = false;
    for fragment in &inner {
        assert_eq!(fragment.border_box_rect().width().to_f64(), expected_inner);
        if !authored_clip {
            assert!(
                !fragment.has_overflow_clip || fragment.block_axis_clip_only,
                "CSS-visible owned nested box cannot acquire a both-axis clip: {fragment:?}"
            );
        }
        for column in &fragment.children {
            if column.kind == FragmentKind::ColumnBox {
                saw_column = true;
                assert_eq!(column.size.width.to_f64(), expected_column);
            }
        }
    }
    assert!(saw_column, "actual retained inner columns");
    if authored_clip {
        assert!(inner.iter().any(|fragment| fragment.has_overflow_clip));
        let style = f.engine.computed_style(f.inner).unwrap();
        assert_eq!(
            (style.overflow_x, style.overflow_y),
            (
                openui_style::Overflow::Hidden,
                openui_style::Overflow::Hidden
            )
        );
    }
    let mut cb = Vec::new();
    owners(scene.fragments(), &f.engine, "abscb", &mut cb);
    assert!(!cb.is_empty(), "actual relative containing-block fragments");
    for fragment in &cb {
        assert_eq!(fragment.border_box_rect().width().to_f64(), expected_column);
        if let Some(geometry) = fragment.box_geometry {
            assert_eq!(geometry.source_size.width.to_f64(), expected_column);
        }
    }
    let cb_node = cb[0].node_id;
    fn check_positioned(
        fragment: &Fragment,
        cb_node: openui_dom::NodeId,
        expected: f64,
        count: &mut usize,
    ) {
        if let Some(positioned) = fragment.positioned_fragmentation {
            if positioned.containing_block_node == cb_node {
                *count += 1;
                assert_eq!(positioned.containing_block_size.width.to_f64(), expected);
            }
        }
        for child in &fragment.children {
            check_positioned(child, cb_node, expected, count);
        }
    }
    let mut positioned = 0;
    check_positioned(scene.fragments(), cb_node, expected_column, &mut positioned);
    assert!(
        positioned > 0,
        "actual OOF containing-block ownership survives"
    );
    for rect in f.engine.client_rects(f.inner).unwrap() {
        assert_eq!(f64::from(rect.width), expected_inner);
    }
    for rect in f.engine.client_rects(f.cb).unwrap() {
        assert_eq!(f64::from(rect.width), expected_column);
    }
    scene
}

fn raster(scene: &SceneSnapshot) -> Frame {
    SoftwareCompositor::default().render(scene).unwrap()
}

#[test]
fn ordered_prepend_and_equal_final_dom_use_parent_column_constraints() {
    for (width, height, scale) in PROFILES {
        let mut ordered = fixture(width, height, scale, true);
        let mut direct = fixture(width, height, scale, false);
        assert_eq!(ordered.barrier_count, 1);
        assert_eq!(direct.barrier_count, 0);
        let ordered_scene = check_basis(&mut ordered, 100.0, 50.0, false);
        let direct_scene = check_basis(&mut direct, 100.0, 50.0, false);
        let mut owned_inner = Vec::new();
        owners(
            ordered_scene.fragments(),
            &ordered.engine,
            "constraint-inner",
            &mut owned_inner,
        );
        assert!(owned_inner
            .iter()
            .all(|part| part.margin.right.to_f64() == 0.0));
        let a = raster(&ordered_scene);
        let b = raster(&direct_scene);
        assert_eq!((a.width, a.height, a.stride), (b.width, b.height, b.stride));
        assert_eq!(a.pixels, b.pixels, "same final native DOM @{scale}");
        let stats = ordered.engine.stats();
        assert_eq!(
            ordered.engine.scene().unwrap().generation(),
            ordered_scene.generation()
        );
        assert_eq!(
            ordered.engine.stats(),
            stats,
            "unchanged frame does no lifecycle work"
        );
    }
}

#[test]
fn nested_constraints_preserve_authored_sizes_edges_and_clip_neighbors() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for neighbor in 0..8 {
            let mut f = fixture(800.0, 600.0, scale, false);
            let (inner, column, authored_clip) = match neighbor {
                0 => {
                    set_literal(
                        &mut f.engine,
                        f.outer,
                        StyleProperty::BackgroundColor,
                        "blue",
                    )
                    .unwrap();
                    (100.0, 50.0, false)
                }
                1 => {
                    set_literal(
                        &mut f.engine,
                        f.outer,
                        StyleProperty::Border,
                        "2px solid blue",
                    )
                    .unwrap();
                    (100.0, 50.0, false)
                }
                2 => {
                    set_literal(&mut f.engine, f.inner, StyleProperty::Width, "80px").unwrap();
                    (80.0, 40.0, false)
                }
                3 => {
                    set_literal(&mut f.engine, f.inner, StyleProperty::Padding, "4px").unwrap();
                    (100.0, 46.0, false)
                }
                4 => {
                    set_literal(&mut f.engine, f.outer, StyleProperty::ColumnFill, "balance")
                        .unwrap();
                    (100.0, 50.0, false)
                }
                5 => {
                    let preceding = create_capture_node(&mut f.engine, ElementTag::Div).unwrap();
                    set_literal(&mut f.engine, preceding, StyleProperty::Height, "5px").unwrap();
                    f.engine.insert_before(f.outer, preceding, f.inner).unwrap();
                    (100.0, 50.0, false)
                }
                6 => {
                    set_literal(&mut f.engine, f.inner, StyleProperty::Height, "50px").unwrap();
                    (100.0, 50.0, false)
                }
                7 => {
                    set_literal(&mut f.engine, f.inner, StyleProperty::Overflow, "hidden").unwrap();
                    (100.0, 50.0, true)
                }
                _ => unreachable!(),
            };
            check_basis(&mut f, inner, column, authored_clip);
        }
    }
}
