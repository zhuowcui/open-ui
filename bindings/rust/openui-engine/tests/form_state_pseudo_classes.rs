//! The form pseudo-classes against pinned Chromium 147.
//!
//! `tools/qualification/capture_form_state_oracle.py` built one tree of form
//! controls in Chromium, changed properties and attributes step by step, and
//! recorded after each step the ids `querySelectorAll` returned for each
//! selector, in
//! `docs/progress/evidence/form-state-pseudo-classes-v1/chromium-windows.json`.
//! This test builds the same tree, makes the same changes through the engine
//! and checks `Engine::matches_selector_list` in document order.

use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, SelectorMatchError, ViewportMetrics};
use openui_style::selector_parser::parse_selector_list;
use serde_json::Value;

const ORACLE: &str = include_str!(
    "../../../../docs/progress/evidence/form-state-pseudo-classes-v1/chromium-windows.json"
);

fn tag(name: &str) -> ElementTag {
    match name {
        "div" => ElementTag::Div,
        "button" => ElementTag::Button,
        "input" => ElementTag::Input,
        "textarea" => ElementTag::TextArea,
        "select" => ElementTag::Select,
        "option" => ElementTag::Option,
        "optgroup" => ElementTag::OptGroup,
        "fieldset" => ElementTag::Fieldset,
        "legend" => ElementTag::Legend,
        "progress" => ElementTag::Progress,
        "meter" => ElementTag::Meter,
        "form" => ElementTag::Form,
        other => panic!("fixture tag `{other}` has no mapping"),
    }
}

/// Build `spec` under `parent`; push each element with its id in document
/// order.
fn build(
    engine: &mut Engine,
    parent: NodeHandle,
    spec: &Value,
    out: &mut Vec<(NodeHandle, String)>,
) {
    let element = engine
        .create_native_element(tag(spec["tag"].as_str().unwrap()))
        .unwrap();
    engine.append_child(parent, element).unwrap();
    for (name, value) in spec["attributes"].as_object().unwrap() {
        engine
            .set_attribute(element, name, value.as_str().unwrap())
            .unwrap();
    }
    out.push((
        element,
        spec["attributes"]["id"].as_str().unwrap().to_owned(),
    ));
    for child in spec["children"].as_array().unwrap() {
        build(engine, element, child, out);
    }
}

fn fixture(oracle: &Value) -> (Engine, Vec<(NodeHandle, String)>) {
    let mut engine =
        Engine::new(ViewportMetrics::from_logical_size(200.0, 200.0, 1.0).unwrap()).unwrap();
    let html = engine.create_native_element(ElementTag::Html).unwrap();
    engine.append_child(engine.root(), html).unwrap();
    engine.set_attribute(html, "id", "html").unwrap();
    let body = engine.create_native_element(ElementTag::Body).unwrap();
    engine.append_child(html, body).unwrap();
    engine.set_attribute(body, "id", "body").unwrap();
    let mut elements = vec![(html, "html".to_owned()), (body, "body".to_owned())];
    for spec in oracle["fixture"].as_array().unwrap() {
        build(&mut engine, body, spec, &mut elements);
    }
    (engine, elements)
}

/// Make one change of the oracle through the engine.
fn change(engine: &mut Engine, elements: &[(NodeHandle, String)], change: &[Value]) {
    let id = change[1].as_str().unwrap();
    let (element, _) = *elements.iter().find(|(_, other)| other == id).unwrap();
    let name = change[2].as_str().unwrap();
    match (change[0].as_str().unwrap(), name) {
        ("property", "checked") => engine
            .set_checked(element, change[3].as_bool().unwrap())
            .unwrap(),
        ("property", "indeterminate") => engine
            .set_indeterminate(element, change[3].as_bool().unwrap())
            .unwrap(),
        ("property", "value") => engine
            .set_control_value(element, change[3].as_str().unwrap())
            .unwrap(),
        ("attribute", _) => engine
            .set_attribute(element, name, change[3].as_str().unwrap())
            .unwrap(),
        ("remove", _) => {
            engine.remove_attribute(element, name).unwrap();
        }
        (kind, _) => panic!("unknown change `{kind}` of `{name}`"),
    }
}

#[test]
fn form_states_follow_chromium() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let (mut engine, elements) = fixture(&oracle);
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
    assert!(steps.len() >= 10, "{} steps", steps.len());
    let mut failures = Vec::new();
    for step in steps {
        for each in step["changes"].as_array().unwrap() {
            change(&mut engine, &elements, each.as_array().unwrap());
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

#[test]
fn validity_and_dialog_states_are_not_matched_silently() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let (engine, elements) = fixture(&oracle);
    for name in ["invalid", "open", "modal"] {
        let list = parse_selector_list(&format!(":{name}"), None).unwrap();
        assert_eq!(
            engine.matches_selector_list(elements[3].0, &list),
            Err(SelectorMatchError::UnsupportedPseudoClass(name))
        );
    }
}
