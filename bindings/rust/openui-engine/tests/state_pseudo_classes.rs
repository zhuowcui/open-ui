//! `:hover`, `:active`, `:focus`, `:focus-visible` and `:focus-within`
//! against pinned Chromium 147.
//!
//! `tools/qualification/capture_state_pseudo_class_oracle.py` built a tree of
//! absolutely positioned boxes in Chromium, drove it with real mouse and
//! keyboard input and script focus, and recorded after each step the ids
//! `querySelectorAll` returned for each selector, in
//! `docs/progress/evidence/state-pseudo-classes-v1/chromium-windows.json`.
//! This test builds the same tree, replays the steps through the engine and
//! checks `Engine::matches_selector_list` in document order.
//!
//! The public `Document` focuses a focusable pointer target on press and
//! moves focus on Tab; the replay does the same through the engine.

use openui_dom::ElementTag;
use openui_engine::{Engine, FocusOrigin, NodeHandle, PointerEventKind, ViewportMetrics};
use openui_style::selector_parser::parse_selector_list;
use openui_style::{parse_literal, StyleProperty};
use serde_json::Value;

const ORACLE: &str = include_str!(
    "../../../../docs/progress/evidence/state-pseudo-classes-v1/chromium-windows.json"
);
const MOUSE: u64 = 1;

fn tag(name: &str) -> ElementTag {
    match name {
        "div" => ElementTag::Div,
        "button" => ElementTag::Button,
        "input" => ElementTag::Input,
        "textarea" => ElementTag::TextArea,
        other => panic!("fixture tag `{other}` has no mapping"),
    }
}

fn set(engine: &mut Engine, node: NodeHandle, property: StyleProperty, literal: &str) {
    let value = parse_literal(property, literal).unwrap();
    engine.set_property(node, property, value).unwrap();
}

/// The oracle page: `html > body` and the boxes, each with its id, in
/// document order.
fn page(oracle: &Value) -> (Engine, Vec<(NodeHandle, String)>) {
    let viewport = oracle["viewport"].as_array().unwrap();
    let mut engine = Engine::new(
        ViewportMetrics::from_logical_size(
            viewport[0].as_f64().unwrap(),
            viewport[1].as_f64().unwrap(),
            1.0,
        )
        .unwrap(),
    )
    .unwrap();
    let mut elements = Vec::new();
    let mut parent = engine.root();
    for (name, tag) in [("html", ElementTag::Html), ("body", ElementTag::Body)] {
        let element = engine.create_native_element(tag).unwrap();
        engine.append_child(parent, element).unwrap();
        engine.set_attribute(element, "id", name).unwrap();
        set(&mut engine, element, StyleProperty::Margin, "0");
        elements.push((element, name.to_owned()));
        parent = element;
    }
    for spec in oracle["boxes"].as_array().unwrap() {
        let spec = spec.as_array().unwrap();
        let text = |index: usize| spec[index].as_str().unwrap();
        let px = |index: usize| format!("{}px", spec[index].as_f64().unwrap());
        let element = engine.create_native_element(tag(text(1))).unwrap();
        engine.set_attribute(element, "id", text(0)).unwrap();
        for (name, value) in spec[7].as_object().unwrap() {
            engine
                .set_attribute(element, name, value.as_str().unwrap())
                .unwrap();
        }
        set(&mut engine, element, StyleProperty::Position, "absolute");
        set(&mut engine, element, StyleProperty::BoxSizing, "border-box");
        set(&mut engine, element, StyleProperty::Margin, "0");
        set(&mut engine, element, StyleProperty::Left, &px(3));
        set(&mut engine, element, StyleProperty::Top, &px(4));
        set(&mut engine, element, StyleProperty::Width, &px(5));
        set(&mut engine, element, StyleProperty::Height, &px(6));
        let parent = elements
            .iter()
            .find(|(_, id)| id == text(2))
            .map(|(handle, _)| *handle)
            .unwrap();
        engine.append_child(parent, element).unwrap();
        elements.push((element, text(0).to_owned()));
    }
    engine.update().unwrap();
    (engine, elements)
}

fn replay(engine: &mut Engine, elements: &[(NodeHandle, String)], action: &[Value]) {
    let number = |index: usize| action[index].as_f64().unwrap() as f32;
    match action[0].as_str().unwrap() {
        "move" => {
            engine
                .pointer_event(MOUSE, PointerEventKind::Move, number(1), number(2))
                .unwrap();
        }
        "press" => {
            let update = engine
                .pointer_event(MOUSE, PointerEventKind::Down, number(1), number(2))
                .unwrap();
            let target = update.target.unwrap();
            if engine.can_focus(target).unwrap() {
                engine
                    .focus_with_origin(target, FocusOrigin::Pointer)
                    .unwrap();
            }
        }
        "release" => {
            engine
                .pointer_event(MOUSE, PointerEventKind::Up, number(1), number(2))
                .unwrap();
        }
        "key" => {
            engine.note_keyboard_event();
            if action[1] == "Tab" {
                let direction = if action[2].as_bool().unwrap() { -1 } else { 1 };
                engine.advance_focus(direction).unwrap();
            }
        }
        "focus" => {
            let id = action[1].as_str().unwrap();
            let (element, _) = elements.iter().find(|(_, other)| other == id).unwrap();
            engine.focus(*element).unwrap();
        }
        other => panic!("unknown action `{other}`"),
    }
}

#[test]
fn interaction_states_follow_chromium() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let (mut engine, elements) = page(&oracle);
    let selectors: Vec<(&str, _)> = oracle["selectors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|text| {
            let text = text.as_str().unwrap();
            (text, parse_selector_list(text, None).unwrap())
        })
        .collect();
    let steps = oracle["steps"].as_array().unwrap();
    assert!(steps.len() >= 20, "{} steps", steps.len());
    let mut failures = Vec::new();
    for step in steps {
        for action in step["actions"].as_array().unwrap() {
            replay(&mut engine, &elements, action.as_array().unwrap());
        }
        for (text, list) in &selectors {
            let mut actual = Vec::new();
            for (element, id) in &elements {
                if engine.matches_selector_list(*element, list).unwrap() {
                    actual.push(id.as_str());
                }
            }
            let expected: Vec<&str> = step["matches"][*text]
                .as_array()
                .unwrap()
                .iter()
                .map(|id| id.as_str().unwrap())
                .collect();
            if actual != expected {
                failures.push(format!(
                    "{}: `{text}`: expected {expected:?}, got {actual:?}",
                    step["step"]
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
