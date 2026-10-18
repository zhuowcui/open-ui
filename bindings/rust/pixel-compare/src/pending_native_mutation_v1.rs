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

pub fn replay_0(engine: &mut Engine) -> Result<ReplayResult, EngineError> {
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
    let _ = engine.update()?;
    barriers.push((0, engine.stats()));
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

pub fn replay_1(engine: &mut Engine) -> Result<ReplayResult, EngineError> {
    let html = create_capture_node(engine, ElementTag::Html)?;
    engine.append_child(engine.root(), html)?;
    set_literal(engine, html, StyleProperty::Overflow, "hidden")?;
    let node_0 = create_capture_node(engine, ElementTag::Body)?;
    let node_1 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_1, "class", "spacer")?;
    let node_2 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_2, "id", "target")?;
    let node_3 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_3, "class", "transformed")?;
    let node_4 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_4, "class", "fixed")?;
    engine.append_child(node_0, node_1)?;
    engine.append_child(node_0, node_2)?;
    engine.append_child(node_2, node_3)?;
    engine.append_child(node_2, node_4)?;
    engine.append_child(html, node_0)?;
    set_literal(engine, node_0, StyleProperty::BoxSizing, "content-box")?;
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
    set_literal(engine, node_0, StyleProperty::Margin, "0")?;
    set_literal(engine, node_1, StyleProperty::Margin, "0")?;
    set_literal(engine, node_1, StyleProperty::Padding, "0")?;
    set_literal(engine, node_1, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_1, StyleProperty::Height, "100px")?;
    set_literal(engine, node_2, StyleProperty::Margin, "0")?;
    set_literal(engine, node_2, StyleProperty::Padding, "0")?;
    set_literal(engine, node_2, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_2, StyleProperty::Position, "relative")?;
    set_literal(engine, node_2, StyleProperty::Display, "none")?;
    set_literal(engine, node_3, StyleProperty::Margin, "0")?;
    set_literal(engine, node_3, StyleProperty::Padding, "0")?;
    set_literal(engine, node_3, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_3, StyleProperty::Transform, "translateX(0)")?;
    set_literal(engine, node_4, StyleProperty::Margin, "0")?;
    set_literal(engine, node_4, StyleProperty::Padding, "0")?;
    set_literal(engine, node_4, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_4, StyleProperty::Position, "fixed")?;
    set_literal(engine, node_4, StyleProperty::Top, "0")?;
    set_literal(engine, node_4, StyleProperty::Left, "0")?;
    set_literal(engine, node_4, StyleProperty::Width, "100%")?;
    set_literal(engine, node_4, StyleProperty::Height, "50px")?;
    set_literal(engine, node_4, StyleProperty::BackgroundColor, "blue")?;
    let mut barriers = Vec::new();
    // AST operation 0: bind-target
    // AST operation 1: layout-barrier
    let _ = engine.update()?;
    barriers.push((1, engine.stats()));
    // AST operation 2: remove-style
    engine.remove_property(node_2, StyleProperty::Display)?;
    let mut handles = std::collections::BTreeMap::new();
    handles.insert("target".to_string(), node_2);
    handles.insert("binding:target".to_string(), node_2);
    Ok(ReplayResult { handles, barriers })
}

pub fn replay_2(engine: &mut Engine) -> Result<ReplayResult, EngineError> {
    let html = create_capture_node(engine, ElementTag::Html)?;
    engine.append_child(engine.root(), html)?;
    set_literal(engine, html, StyleProperty::Overflow, "hidden")?;
    let node_0 = create_capture_node(engine, ElementTag::Body)?;
    let node_1 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_1, "id", "container")?;
    let node_2 = create_capture_node(engine, ElementTag::Div)?;
    engine.set_attribute(node_2, "id", "target")?;
    engine.append_child(node_0, node_1)?;
    engine.append_child(node_1, node_2)?;
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
    set_literal(engine, node_1, StyleProperty::Width, "200px")?;
    set_literal(engine, node_1, StyleProperty::Height, "150px")?;
    set_literal(engine, node_1, StyleProperty::Border, "1px solid black")?;
    set_literal(engine, node_1, StyleProperty::Overflow, "scroll")?;
    set_literal(engine, node_2, StyleProperty::Margin, "0")?;
    set_literal(engine, node_2, StyleProperty::Padding, "0")?;
    set_literal(engine, node_2, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_2, StyleProperty::Width, "100px")?;
    set_literal(engine, node_2, StyleProperty::Height, "50px")?;
    set_literal(engine, node_2, StyleProperty::BackgroundColor, "green")?;
    set_literal(engine, node_2, StyleProperty::Transform, "scale(1)")?;
    let mut barriers = Vec::new();
    // AST operation 0: layout-barrier
    let _ = engine.update()?;
    barriers.push((0, engine.stats()));
    // AST operation 1: set-style
    set_literal(engine, node_2, StyleProperty::Transform, "scale(1.5)")?;
    set_literal(engine, node_2, StyleProperty::Margin, "0")?;
    set_literal(engine, node_2, StyleProperty::Padding, "0")?;
    set_literal(engine, node_2, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_2, StyleProperty::Width, "100px")?;
    set_literal(engine, node_2, StyleProperty::Height, "50px")?;
    set_literal(engine, node_2, StyleProperty::BackgroundColor, "green")?;
    set_literal(engine, node_2, StyleProperty::Transform, "scale(1.5)")?;
    // AST operation 2: layout-barrier
    let _ = engine.update()?;
    barriers.push((2, engine.stats()));
    // AST operation 3: set-style
    set_literal(engine, node_2, StyleProperty::Transform, "scale(4.0)")?;
    set_literal(engine, node_2, StyleProperty::Margin, "0")?;
    set_literal(engine, node_2, StyleProperty::Padding, "0")?;
    set_literal(engine, node_2, StyleProperty::BoxSizing, "content-box")?;
    set_literal(engine, node_2, StyleProperty::Width, "100px")?;
    set_literal(engine, node_2, StyleProperty::Height, "50px")?;
    set_literal(engine, node_2, StyleProperty::BackgroundColor, "green")?;
    set_literal(engine, node_2, StyleProperty::Transform, "scale(4.0)")?;
    let mut handles = std::collections::BTreeMap::new();
    handles.insert("container".to_string(), node_1);
    handles.insert("target".to_string(), node_2);
    Ok(ReplayResult { handles, barriers })
}
