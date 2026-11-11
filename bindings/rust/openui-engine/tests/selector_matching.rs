//! Selector matching against pinned Chromium 147.
//!
//! `tools/qualification/capture_selector_matching_oracle.py` built one
//! element tree in Chromium and recorded, in
//! `docs/progress/evidence/selector-matching-v1/chromium-windows.json`, the
//! ids `querySelectorAll` returned for each selector. This test builds the
//! same tree, `html > body > fixture`, and checks that
//! `Engine::matches_selector` selects the same elements in document order.

use openui_dom::ElementTag;
use openui_engine::{Engine, NodeHandle, SelectorMatchError, ViewportMetrics};
use openui_style::selector_parser::parse_selector_list;
use serde_json::Value;

const ORACLE: &str =
    include_str!("../../../../docs/progress/evidence/selector-matching-v1/chromium-windows.json");

fn tag(name: &str) -> ElementTag {
    match name {
        "div" => ElementTag::Div,
        "span" => ElementTag::Span,
        "button" => ElementTag::Button,
        "img" => ElementTag::Image,
        "table" => ElementTag::Table,
        "tbody" => ElementTag::TableBody,
        "tr" => ElementTag::TableRow,
        "td" => ElementTag::TableCell,
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
    if let Some(text) = spec.get("text") {
        let node = engine.create_text(text.as_str().unwrap()).unwrap();
        engine.append_child(parent, node).unwrap();
        return;
    }
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

#[test]
fn matching_follows_chromium() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let (engine, elements) = fixture(&oracle);
    let cases = oracle["cases"].as_array().unwrap();
    assert!(cases.len() >= 60, "{} cases", cases.len());
    for case in cases {
        let text = case["selector"].as_str().unwrap();
        let list = parse_selector_list(text, None).unwrap_or_else(|e| panic!("`{text}`: {e:?}"));
        let mut actual = Vec::new();
        for (element, id) in &elements {
            if engine.matches_selector_list(*element, &list).unwrap() {
                actual.push(id.as_str());
            }
        }
        let expected: Vec<&str> = case["matches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .collect();
        assert_eq!(actual, expected, "`{text}`");
    }
}

#[test]
fn form_state_pseudo_classes_are_not_matched_silently() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let (engine, elements) = fixture(&oracle);
    let list = parse_selector_list(".box:checked", None).unwrap();
    assert_eq!(
        engine.matches_selector_list(elements[2].0, &list),
        Err(SelectorMatchError::UnsupportedPseudoClass("checked"))
    );
}

#[test]
fn text_and_the_viewport_are_not_elements() {
    let oracle: Value = serde_json::from_str(ORACLE).unwrap();
    let (mut engine, _) = fixture(&oracle);
    let universal = &parse_selector_list("*", None).unwrap()[0];
    assert!(!engine.matches_selector(engine.root(), universal).unwrap());
    let text = engine.create_text("x").unwrap();
    let root = engine.root();
    engine.append_child(root, text).unwrap();
    assert!(!engine.matches_selector(text, universal).unwrap());
}
