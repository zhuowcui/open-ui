//! Native Rust consuming app for form association and radio state transitions.
//! Calls public APIs and records observations without expected values or scripts.
use openui::prelude::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
type Registry = Rc<RefCell<Vec<(String, WeakElement)>>>;
fn find(registry: &Registry, label: &str) -> Element {
    registry
        .borrow()
        .iter()
        .find(|(name, _)| name == label)
        .unwrap()
        .1
        .upgrade()
        .unwrap()
}
fn name_of(registry: &Registry, element: Option<Element>) -> Option<String> {
    element.and_then(|element| {
        registry.borrow().iter().find_map(|(name, weak)| {
            weak.upgrade().and_then(|candidate| {
                candidate
                    .is_same_node(&element)
                    .unwrap()
                    .then(|| name.clone())
            })
        })
    })
}
fn snapshot(registry: &Registry) -> Value {
    let mut controls = serde_json::Map::new();
    let mut active = "body".to_owned();
    for label in ["a", "b", "c", "d", "e"] {
        let node = find(registry, label);
        if node.has_focus().unwrap() {
            active = label.to_owned();
        }
        controls.insert(
            label.into(),
            json!({
                "checked":node.is_checked().unwrap(),
                "checkedAttribute":node.get_attribute("checked").unwrap().is_some(),
                "formAttribute":node.get_attribute("form").unwrap(),
                "connected":node.is_connected().unwrap(),
                "parent":name_of(registry,node.parent().unwrap())
            }),
        );
    }
    json!({"controls":controls,"active":active})
}
fn barrier(document: &Document) {
    for _ in 0..64 {
        if !document.has_pending_events().unwrap() {
            return;
        }
        document.dispatch_pending_events().unwrap();
    }
    panic!("native event queue did not quiesce");
}
fn apply(registry: &Registry, operation: &Value, errors: &Rc<RefCell<Vec<String>>>) {
    let target = find(registry, operation["target"].as_str().unwrap());
    let result = match operation["op"].as_str().unwrap() {
        "attribute" => target.set_attribute(
            operation["name"].as_str().unwrap(),
            operation["value"].as_str().unwrap(),
        ),
        "remove_attribute" => target
            .remove_attribute(operation["name"].as_str().unwrap())
            .map(|_| ()),
        "detach" => target.detach(),
        "append" => find(registry, operation["parent"].as_str().unwrap()).append_child(&target),
        "insert" => find(registry, operation["parent"].as_str().unwrap()).insert_before(
            &target,
            &find(registry, operation["before"].as_str().unwrap()),
        ),
        "checked" => target.set_checked(operation["value"].as_bool().unwrap()),
        _ => panic!("unknown native operation"),
    };
    if let Err(error) = result {
        errors.borrow_mut().push(format!("{operation}: {error}"));
    }
}
fn make(document: &Document, control: &str) -> (Vec<Element>, Registry) {
    let registry: Registry = Rc::new(RefCell::new(Vec::new()));
    let body = document.body();
    registry
        .borrow_mut()
        .push(("body".into(), body.downgrade()));
    let mut keep = vec![body];
    for (label, kind) in [
        ("wrap", "div"),
        ("fa", "form"),
        ("fb", "form"),
        ("parking", "div"),
        ("blocker", "div"),
    ] {
        let node = Element::create(document, kind).unwrap();
        document.body().append_child(&node).unwrap();
        node.set_id(label).unwrap();
        registry.borrow_mut().push((label.into(), node.downgrade()));
        keep.push(node);
    }
    let newform = Element::create(document, "form").unwrap();
    registry
        .borrow_mut()
        .push(("newform".into(), newform.downgrade()));
    keep.push(newform);
    for (label, parent, owner) in [
        ("a", "wrap", Some("fa")),
        ("b", "wrap", Some("fb")),
        ("c", "fa", None),
        ("d", "fb", None),
        ("e", "body", None),
    ] {
        let node = Element::create(document, "input").unwrap();
        node.set_id(label).unwrap();
        node.set_attribute("type", control).unwrap();
        node.set_attribute("name", "g").unwrap();
        if let Some(owner) = owner {
            node.set_attribute("form", owner).unwrap();
        }
        find(&registry, parent).append_child(&node).unwrap();
        registry.borrow_mut().push((label.into(), node.downgrade()));
        keep.push(node);
    }
    for label in ["a", "b", "e"] {
        find(&registry, label).set_checked(true).unwrap();
    }
    (keep, registry)
}
fn main() {
    let path = std::env::args().nth(1).expect("native fixture path");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut observations = Vec::new();
    for control in fixture["controls"].as_array().unwrap() {
        let control = control.as_str().unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let document = Document::new(800, 600).unwrap();
            let (keep, registry) = make(&document, control);
            let errors = Rc::new(RefCell::new(Vec::<String>::new()));
            let rows = Rc::new(RefCell::new(Vec::<Value>::new()));
            for label in ["a", "b", "c", "d", "e"] {
                for kind in [
                    "click", "input", "change", "focus", "focusin", "blur", "focusout",
                ] {
                    let r = registry.clone();
                    let output = rows.clone();
                    find(&registry, label)
                        .on(kind, move |event| {
                            let mut row = snapshot(&r);
                            let values = row.as_object_mut().unwrap();
                            values.insert("type".into(), json!(event.event_type));
                            values.insert("target".into(), json!(name_of(&r, event.target())));
                            values.insert("bubbles".into(), json!(event.bubbles()));
                            values.insert("cancelable".into(), json!(event.cancelable()));
                            output.borrow_mut().push(row);
                        })
                        .unwrap();
                }
            }
            barrier(&document);
            for operation in case["pre"].as_array().unwrap() {
                apply(&registry, operation, &errors);
            }
            rows.borrow_mut().clear();
            let initial = snapshot(&registry);
            let mut steps = Vec::new();
            for operation in case["steps"].as_array().unwrap() {
                apply(&registry, operation, &errors);
                let synchronous = json!({"rows":rows.borrow().clone(),"state":snapshot(&registry)});
                barrier(&document);
                let observed = json!({"rows":rows.borrow().clone(),"state":snapshot(&registry)});
                steps.push(
                    json!({"operation":operation,"synchronous":synchronous,"observed":observed}),
                );
            }
            let variant = case["name"].as_str().unwrap();
            observations.push(json!({"scenario":format!("{control}-{variant}"),"control":control,"variant":variant,"initial":initial,"steps":steps,"api_errors":errors.borrow().clone()}));
            let weak: Vec<_> = keep.iter().map(Element::downgrade).collect();
            drop(keep);
            drop(document);
            assert!(weak.iter().all(|e| e.upgrade().is_none()));
        }
    }
    println!("{}", serde_json::to_string(&observations).unwrap());
}
