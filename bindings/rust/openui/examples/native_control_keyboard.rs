//! Native Rust keyboard consuming app. Fixture data describes native operations,
//! never scripts or expected browser states.
use openui::prelude::*;
use openui_style::{Display, Visibility};
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
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
    let mut active = "body".to_owned();
    let mut controls = serde_json::Map::new();
    for label in ["a", "b", "c"] {
        let node = find(registry, label);
        if node.has_focus().unwrap() {
            active = label.to_owned();
        }
        controls.insert(label.into(), json!({"checked":node.is_checked().unwrap(),"indeterminate":node.is_indeterminate().unwrap(),"activePseudo":node.is_active().unwrap(),"connected":node.is_connected().unwrap()}));
    }
    json!({"active":active,"controls":controls})
}
fn apply(registry: &Registry, op: &Value) -> Result<(), Error> {
    let node = find(registry, op["target"].as_str().unwrap());
    match op["op"].as_str().unwrap() {
        "attribute" => {
            node.set_attribute(op["name"].as_str().unwrap(), op["value"].as_str().unwrap())
        }
        "checked" => node.set_checked(op["value"].as_bool().unwrap()),
        "focus" => node.focus(),
        "detach" => node.detach(),
        "style" => match (op["name"].as_str().unwrap(), op["value"].as_str().unwrap()) {
            ("display", "none") => node.set_property(StyleProperty::Display, Display::None.into()),
            ("visibility", "hidden") => {
                node.set_property(StyleProperty::Visibility, Visibility::Hidden.into())
            }
            _ => panic!("unsupported native diagnostic style"),
        },
        _ => panic!("unsupported native diagnostic operation"),
    }
}
fn barrier(document: &Document) {
    for _ in 0..64 {
        if !document.has_pending_events().unwrap() {
            return;
        }
        document.dispatch_pending_events().unwrap();
    }
    panic!("native callback queue did not quiesce");
}
fn cdp_modifiers(bits: i32) -> i32 {
    let bits = Modifiers(bits as u32);
    i32::from(bits.contains(Modifiers::ALT))
        | (i32::from(bits.contains(Modifiers::CTRL)) << 1)
        | (i32::from(bits.contains(Modifiers::META)) << 2)
        | (i32::from(bits.contains(Modifiers::SHIFT)) << 3)
}
fn native_modifiers(bits: u64) -> Modifiers {
    let mut result = Modifiers::NONE;
    for (mask, value) in [
        (1, Modifiers::ALT),
        (2, Modifiers::CTRL),
        (4, Modifiers::META),
        (8, Modifiers::SHIFT),
    ] {
        if bits & mask != 0 {
            result = result | value;
        }
    }
    result
}
fn main() {
    let path = std::env::args().nth(1).expect("native fixture path");
    let fixture: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let mut observations = Vec::new();
    for case in fixture["scenarios"].as_array().unwrap() {
        let document = Document::new(800, 600).unwrap();
        let registry: Registry = Rc::new(RefCell::new(Vec::new()));
        let rows = Rc::new(RefCell::new(Vec::<Value>::new()));
        let errors = Rc::new(RefCell::new(Vec::<String>::new()));
        let handled = Rc::new(Cell::new(false));
        let handler = case["handler"].clone();
        let mut owners = vec![document.body()];
        registry
            .borrow_mut()
            .push(("body".into(), document.body().downgrade()));
        for label in ["fa", "fb"] {
            let form = Element::create(&document, "form").unwrap();
            form.set_attribute("id", label).unwrap();
            document.body().append_child(&form).unwrap();
            registry.borrow_mut().push((label.into(), form.downgrade()));
            owners.push(form);
        }
        let kind = case["kind"].as_str().unwrap();
        for label in ["a", "b", "c"] {
            let node =
                Element::create(&document, if kind == "button" { "button" } else { "input" })
                    .unwrap();
            node.set_attribute("id", label).unwrap();
            node.set_attribute("type", if kind == "button" { "button" } else { kind })
                .unwrap();
            if kind != "button" {
                node.set_attribute("name", "g").unwrap();
            }
            document.body().append_child(&node).unwrap();
            registry.borrow_mut().push((label.into(), node.downgrade()));
            owners.push(node);
        }
        for label in ["a", "b", "c", "body"] {
            for event_type in [
                "keydown", "keyup", "click", "input", "change", "focus", "focusin", "blur",
                "focusout",
            ] {
                let registry = registry.clone();
                let rows = rows.clone();
                let errors = errors.clone();
                let handled = handled.clone();
                let handler = handler.clone();
                find(&registry,label).on(event_type,move |event| {
                    let target=name_of(&registry,event.target());
                    if target.as_deref()!=Some(label) {return;}
                    let state=snapshot(&registry);
                    let keyboard=matches!(event.event_type.as_str(),"keydown"|"keyup");
                    rows.borrow_mut().push(json!({"type":event.event_type,"target":target,"related":name_of(&registry,event.related_target()),"bubbles":event.bubbles(),"cancelable":event.cancelable(),"key":if keyboard {Some(event.key_text.as_str())} else {None},"keyCode":if keyboard {event.key_code} else {0},"modifiers":cdp_modifiers(event.modifiers),"active":state["active"],"controls":state["controls"]}));
                    if !handler.is_null() && !handled.get() && handler["event"]==event.event_type && handler["target"]==label {
                        handled.set(true);if handler["prevent"].as_bool().unwrap() {event.prevent_default();}
                        for op in handler["operations"].as_array().unwrap() {
                            if let Err(error)=apply(&registry,op) {errors.borrow_mut().push(error.to_string());}
                        }
                    }
                }).unwrap();
            }
        }
        if kind != "button" {
            find(&registry, "a").set_checked(true).unwrap();
        }
        find(&registry, "a").focus().unwrap();
        for op in case["pre"].as_array().unwrap() {
            apply(&registry, op).unwrap();
        }
        barrier(&document);
        rows.borrow_mut().clear();
        handled.set(false);
        let initial = snapshot(&registry);
        let mut steps = Vec::new();
        for op in case["steps"].as_array().unwrap() {
            let result = if op["op"] == "key" {
                let down = op["phase"] == "down";
                let code = op["code"].as_i64().unwrap() as i32;
                document.dispatch_key_input(
                    if down {
                        KeyEventType::Down
                    } else {
                        KeyEventType::Up
                    },
                    code,
                    op["key"].as_str(),
                    if down && code == 32 {
                        Some(" ")
                    } else if down && code == 13 {
                        Some("\r")
                    } else {
                        None
                    },
                    native_modifiers(op["modifiers"].as_u64().unwrap()),
                )
            } else {
                apply(&registry, op)
            };
            if let Err(error) = result {
                errors.borrow_mut().push(error.to_string());
            }
            let synchronous = json!({"rows":rows.borrow().clone(),"state":snapshot(&registry),"errors":errors.borrow().clone()});
            barrier(&document);
            let observed = json!({"rows":rows.borrow().clone(),"state":snapshot(&registry),"errors":errors.borrow().clone()});
            steps.push(json!({"operation":op,"synchronous":synchronous,"observed":observed}));
        }
        observations
            .push(json!({"scenario":case["name"],"kind":kind,"initial":initial,"steps":steps}));
        drop(owners);
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
