//! Consuming Rust app draft; not yet compiled or qualified.
//! Records native calls, state and callback observations. No expected state,
//! pixels or browser scripts are embedded in this app.
use openui::prelude::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;

type Registry = Rc<RefCell<Vec<(String, WeakElement)>>>;
type Errors = Rc<RefCell<Vec<String>>>;
thread_local! {
    static RAW_DISPATCH: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}
const CONTROLS: [&str; 4] = ["first", "second", "a", "b"];
const PROGRAMMATIC: [&str; 17] = [
    "enabled-set",
    "own-disabled-set",
    "group-disabled-set",
    "first-legend-set",
    "second-legend-set",
    "disabled-neighbor-uncheck",
    "group-neighbor-uncheck",
    "own-disabled-false",
    "group-disabled-false",
    "disabled-click",
    "group-disabled-click",
    "first-legend-click",
    "second-legend-click",
    "independent-forms",
    "detached-group",
    "different-names",
    "empty-names",
];
const DEFAULTS: [&str; 18] = [
    "default-clean-set",
    "default-clean-remove",
    "assign-false-then-default",
    "assign-true-then-default-remove",
    "assign-false-then-default-remove",
    "same-true-after-attribute",
    "clone-live-true",
    "clone-dirty-false",
    "clone-clean",
    "click-cancel",
    "click-callback-false",
    "click-callback-disable",
    "click-callback-default",
    "neighbor-cancel",
    "neighbor-cancel-disable",
    "neighbor-cancel-assign",
    "click-default-remove",
    "programmatic-no-events",
];
const ACTIVATION: [&str; 20] = [
    "neighbor-click",
    "already-checked-click",
    "neighbor-callback-check-old",
    "neighbor-callback-check-other",
    "neighbor-callback-disable",
    "neighbor-callback-rename",
    "neighbor-callback-form",
    "neighbor-callback-detach",
    "neighbor-callback-default",
    "neighbor-cancel-detach",
    "neighbor-cancel-check-other",
    "neighbor-cancel-rename",
    "neighbor-cancel-form",
    "neighbor-cancel-default",
    "indeterminate-click",
    "indeterminate-cancel",
    "indeterminate-callback",
    "disabled-neighbor-click",
    "fieldset-disabled-neighbor-click",
    "default-checked-neighbor-click",
];

fn find(registry: &Registry, id: &str) -> Element {
    registry
        .borrow()
        .iter()
        .find(|(name, _)| name == id)
        .unwrap()
        .1
        .upgrade()
        .unwrap()
}
fn note(errors: &Errors, operation: &str, result: Result<(), Error>) {
    if let Err(error) = result {
        errors.borrow_mut().push(format!("{operation}: {error}"));
    }
}
fn attr(r: &Registry, e: &Errors, id: &str, name: &str, value: &str) {
    note(
        e,
        &format!("{id}.attribute.{name}"),
        find(r, id).set_attribute(name, value),
    );
}
fn remove_attr(r: &Registry, e: &Errors, id: &str, name: &str) {
    note(
        e,
        &format!("{id}.remove_attribute.{name}"),
        find(r, id).remove_attribute(name).map(|_| ()),
    );
}
fn checked(r: &Registry, e: &Errors, id: &str, value: bool) {
    note(
        e,
        &format!("{id}.set_checked({value})"),
        find(r, id).set_checked(value),
    );
}
fn indeterminate(r: &Registry, e: &Errors, id: &str, value: bool) {
    note(
        e,
        &format!("{id}.set_indeterminate({value})"),
        find(r, id).set_indeterminate(value),
    );
}
fn click(r: &Registry, e: &Errors, id: &str) {
    let result = if RAW_DISPATCH.with(std::cell::Cell::get) {
        find(r, id).dispatch_click_event().map(|_| ())
    } else {
        find(r, id).click()
    };
    note(e, &format!("{id}.click"), result);
}
fn move_to(r: &Registry, e: &Errors, id: &str, parent: &str) {
    note(
        e,
        &format!("{id}.append_to.{parent}"),
        find(r, parent).append_child(&find(r, id)),
    );
}
fn detach(r: &Registry, e: &Errors, id: &str) {
    note(e, &format!("{id}.detach"), find(r, id).detach());
}
fn snapshot(registry: &Registry, extended: bool) -> Value {
    let mut controls = serde_json::Map::new();
    let mut active = "body".to_owned();
    for (id, weak) in registry.borrow().iter() {
        if !CONTROLS.contains(&id.as_str()) && id != "copy" {
            continue;
        }
        let node = weak.upgrade().unwrap();
        if node.has_focus().unwrap() {
            active = id.clone();
        }
        let authored_checked = node.get_attribute("checked").unwrap().is_some();
        let mut state = json!({"checked": node.is_checked().unwrap(),
            "checkedAttribute": authored_checked, "own": node.is_own_disabled().unwrap(),
            "effective": node.is_effectively_disabled().unwrap(), "connected": node.is_connected().unwrap()});
        if extended {
            state["defaultChecked"] = json!(authored_checked);
            state["indeterminate"] = json!(node.is_indeterminate().unwrap());
        }
        controls.insert(id.clone(), state);
    }
    json!({"controls": controls, "active": active})
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
fn make(document: &Document, control: &str) -> (Vec<Element>, Registry) {
    let registry: Registry = Rc::new(RefCell::new(Vec::new()));
    let mut keep = Vec::new();
    for (id, kind) in [
        ("fa", "form"),
        ("fb", "form"),
        ("f", "fieldset"),
        ("l1", "legend"),
        ("l2", "legend"),
        ("first", "input"),
        ("second", "input"),
        ("a", "input"),
        ("b", "input"),
    ] {
        let node = Element::create(document, kind).unwrap();
        node.set_id(id).unwrap();
        if kind == "input" {
            node.set_attribute("type", control).unwrap();
            node.set_attribute("name", "g").unwrap();
        }
        registry.borrow_mut().push((id.into(), node.downgrade()));
        keep.push(node);
    }
    for id in ["fa", "fb", "f", "b"] {
        document.body().append_child(&find(&registry, id)).unwrap();
    }
    for (parent, child) in [
        ("f", "l1"),
        ("l1", "first"),
        ("f", "l2"),
        ("l2", "second"),
        ("f", "a"),
    ] {
        find(&registry, parent)
            .append_child(&find(&registry, child))
            .unwrap();
    }
    (keep, registry)
}
fn callback(registry: &Registry, errors: &Errors, target: &str, action: &'static str) {
    let r = registry.clone();
    let e = errors.clone();
    find(registry, target)
        .on("click", move |event| match action {
            "cancel" => event.prevent_default(),
            "false" => checked(&r, &e, "a", false),
            "disable-a" => attr(&r, &e, "a", "disabled", ""),
            "default-a" => attr(&r, &e, "a", "checked", ""),
            "cancel-disable" => {
                attr(&r, &e, "b", "disabled", "");
                event.prevent_default();
            }
            "cancel-assign" => {
                checked(&r, &e, "b", true);
                event.prevent_default();
            }
            "check-old" => checked(&r, &e, "a", true),
            "check-other" => checked(&r, &e, "second", true),
            "disable-b" => attr(&r, &e, "b", "disabled", ""),
            "rename" => attr(&r, &e, "b", "name", "other"),
            "form" => move_to(&r, &e, "b", "fb"),
            "detach" => detach(&r, &e, "b"),
            "cancel-detach" => {
                detach(&r, &e, "b");
                event.prevent_default();
            }
            "cancel-check-other" => {
                checked(&r, &e, "second", true);
                event.prevent_default();
            }
            "cancel-rename" => {
                attr(&r, &e, "b", "name", "other");
                event.prevent_default();
            }
            "cancel-form" => {
                move_to(&r, &e, "b", "fb");
                event.prevent_default();
            }
            "cancel-default" => {
                attr(&r, &e, "a", "checked", "");
                event.prevent_default();
            }
            "indeterminate" => indeterminate(&r, &e, "b", true),
            _ => panic!("unknown native callback operation"),
        })
        .unwrap();
}
fn prepare_programmatic(r: &Registry, e: &Errors, variant: &str) {
    match variant {
        "own-disabled-set" | "disabled-click" => attr(r, e, "a", "disabled", ""),
        "group-disabled-set"
        | "first-legend-set"
        | "second-legend-set"
        | "group-disabled-click"
        | "first-legend-click"
        | "second-legend-click" => attr(r, e, "f", "disabled", ""),
        "disabled-neighbor-uncheck" | "own-disabled-false" => {
            checked(r, e, "a", true);
            attr(r, e, "a", "disabled", "");
        }
        "group-neighbor-uncheck" | "group-disabled-false" => {
            checked(r, e, "a", true);
            attr(r, e, "f", "disabled", "");
        }
        "independent-forms" => {
            checked(r, e, "a", true);
            attr(r, e, "a", "form", "fa");
            attr(r, e, "b", "form", "fb");
        }
        "detached-group" => {
            detach(r, e, "f");
            checked(r, e, "a", true);
        }
        "different-names" => {
            checked(r, e, "a", true);
            attr(r, e, "b", "name", "other");
        }
        "empty-names" => {
            checked(r, e, "a", true);
            remove_attr(r, e, "a", "name");
            remove_attr(r, e, "b", "name");
        }
        _ => {}
    }
}
fn action_programmatic(r: &Registry, e: &Errors, variant: &str) {
    match variant {
        "first-legend-set" => checked(r, e, "first", true),
        "second-legend-set" | "detached-group" => checked(r, e, "second", true),
        "own-disabled-false" | "group-disabled-false" => checked(r, e, "a", false),
        "disabled-neighbor-uncheck"
        | "group-neighbor-uncheck"
        | "independent-forms"
        | "different-names"
        | "empty-names" => checked(r, e, "b", true),
        "disabled-click" | "group-disabled-click" => click(r, e, "a"),
        "first-legend-click" => click(r, e, "first"),
        "second-legend-click" => click(r, e, "second"),
        _ => checked(r, e, "a", true),
    }
}
fn prepare_default(r: &Registry, e: &Errors, variant: &str) {
    match variant {
        "default-clean-remove"
        | "same-true-after-attribute"
        | "clone-clean"
        | "click-default-remove" => attr(r, e, "a", "checked", ""),
        "assign-true-then-default-remove" => {
            attr(r, e, "a", "checked", "");
            checked(r, e, "a", true);
        }
        "assign-false-then-default-remove" | "clone-dirty-false" => {
            attr(r, e, "a", "checked", "");
            checked(r, e, "a", false);
        }
        "clone-live-true" => checked(r, e, "a", true),
        "click-cancel" => callback(r, e, "a", "cancel"),
        "click-callback-false" => callback(r, e, "a", "false"),
        "click-callback-disable" => callback(r, e, "a", "disable-a"),
        "click-callback-default" => callback(r, e, "a", "default-a"),
        "neighbor-cancel" => {
            checked(r, e, "a", true);
            callback(r, e, "b", "cancel");
        }
        "neighbor-cancel-disable" => {
            checked(r, e, "a", true);
            callback(r, e, "b", "cancel-disable");
        }
        "neighbor-cancel-assign" => {
            checked(r, e, "a", true);
            callback(r, e, "b", "cancel-assign");
        }
        _ => {}
    }
}
fn action_default(d: &Document, keep: &mut Vec<Element>, r: &Registry, e: &Errors, variant: &str) {
    match variant {
        "default-clean-set" => attr(r, e, "a", "checked", ""),
        "default-clean-remove"
        | "assign-true-then-default-remove"
        | "assign-false-then-default-remove" => remove_attr(r, e, "a", "checked"),
        "assign-false-then-default" => {
            checked(r, e, "a", false);
            attr(r, e, "a", "checked", "");
        }
        "same-true-after-attribute" => {
            checked(r, e, "a", true);
            remove_attr(r, e, "a", "checked");
        }
        "clone-live-true" | "clone-dirty-false" | "clone-clean" => {
            let copy = find(r, "a").clone_subtree().unwrap();
            copy.set_id("copy").unwrap();
            if variant != "clone-live-true" {
                copy.set_attribute("name", "clone").unwrap();
            }
            r.borrow_mut().push(("copy".into(), copy.downgrade()));
            d.body().append_child(&copy).unwrap();
            if variant == "clone-live-true" {
                copy.set_attribute("name", "clone").unwrap();
            }
            if variant == "clone-dirty-false" {
                copy.set_attribute("checked", "").unwrap();
            } else {
                copy.remove_attribute("checked").unwrap();
            }
            keep.push(copy);
        }
        "neighbor-cancel" | "neighbor-cancel-disable" | "neighbor-cancel-assign" => {
            click(r, e, "b")
        }
        "click-default-remove" => {
            click(r, e, "a");
            remove_attr(r, e, "a", "checked");
        }
        "programmatic-no-events" => {
            checked(r, e, "a", true);
            checked(r, e, "a", false);
            indeterminate(r, e, "a", true);
        }
        _ => click(r, e, "a"),
    }
}
fn prepare_activation(r: &Registry, e: &Errors, variant: &str) {
    match variant {
        "already-checked-click" => checked(r, e, "b", true),
        "indeterminate-click" | "indeterminate-cancel" | "indeterminate-callback" => {
            indeterminate(r, e, "b", true);
            if variant == "indeterminate-cancel" {
                callback(r, e, "b", "cancel");
            }
            if variant == "indeterminate-callback" {
                callback(r, e, "b", "indeterminate");
            }
        }
        "default-checked-neighbor-click" => attr(r, e, "a", "checked", ""),
        _ => {
            checked(r, e, "a", true);
            let action = match variant {
                "neighbor-callback-check-old" => Some("check-old"),
                "neighbor-callback-check-other" => Some("check-other"),
                "neighbor-callback-disable" => Some("disable-b"),
                "neighbor-callback-rename" => Some("rename"),
                "neighbor-callback-form" => Some("form"),
                "neighbor-callback-detach" => Some("detach"),
                "neighbor-callback-default" => Some("default-a"),
                "neighbor-cancel-detach" => Some("cancel-detach"),
                "neighbor-cancel-check-other" => Some("cancel-check-other"),
                "neighbor-cancel-rename" => Some("cancel-rename"),
                "neighbor-cancel-form" => Some("cancel-form"),
                "neighbor-cancel-default" => Some("cancel-default"),
                _ => None,
            };
            if let Some(action) = action {
                callback(r, e, "b", action);
            }
            if variant == "disabled-neighbor-click" {
                attr(r, e, "a", "disabled", "");
            }
            if variant == "fieldset-disabled-neighbor-click" {
                attr(r, e, "f", "disabled", "");
            }
        }
    }
}
fn main() {
    let arguments: Vec<_> = std::env::args().skip(1).collect();
    if !arguments.is_empty() && arguments != ["dispatch"] {
        std::process::exit(2);
    }
    RAW_DISPATCH.with(|mode| mode.set(!arguments.is_empty()));
    let mut suites = serde_json::Map::new();
    for (suite, variants) in [
        ("programmatic", PROGRAMMATIC.as_slice()),
        ("default", DEFAULTS.as_slice()),
        ("activation", ACTIVATION.as_slice()),
    ] {
        let mut observations = Vec::new();
        for control in ["radio", "checkbox"] {
            for variant in variants {
                let d = Document::new(800, 600).unwrap();
                let (mut keep, registry) = make(&d, control);
                let errors: Errors = Rc::new(RefCell::new(Vec::new()));
                let rows = Rc::new(RefCell::new(Vec::<Value>::new()));
                let extended = suite != "programmatic";
                for id in CONTROLS {
                    for kind in [
                        "click", "input", "change", "focus", "focusin", "blur", "focusout",
                    ] {
                        let r = registry.clone();
                        let output = rows.clone();
                        find(&registry, id)
                            .on(kind, move |event| {
                                let mut row = snapshot(&r, extended);
                                let map = row.as_object_mut().unwrap();
                                map.insert("type".into(), json!(event.event_type));
                                map.insert(
                                    "target".into(),
                                    json!(event
                                        .target()
                                        .unwrap()
                                        .get_attribute("id")
                                        .unwrap()
                                        .unwrap()),
                                );
                                map.insert("bubbles".into(), json!(event.bubbles()));
                                map.insert("cancelable".into(), json!(event.cancelable()));
                                output.borrow_mut().push(row);
                            })
                            .unwrap();
                    }
                }
                barrier(&d);
                match suite {
                    "programmatic" => prepare_programmatic(&registry, &errors, variant),
                    "default" => prepare_default(&registry, &errors, variant),
                    "activation" => prepare_activation(&registry, &errors, variant),
                    _ => unreachable!(),
                }
                rows.borrow_mut().clear();
                let initial = snapshot(&registry, extended);
                match suite {
                    "programmatic" => action_programmatic(&registry, &errors, variant),
                    "default" => action_default(&d, &mut keep, &registry, &errors, variant),
                    "activation" => click(&registry, &errors, "b"),
                    _ => unreachable!(),
                }
                let synchronous =
                    json!({"rows": rows.borrow().clone(), "state": snapshot(&registry, extended)});
                barrier(&d);
                let observed =
                    json!({"rows": rows.borrow().clone(), "state": snapshot(&registry, extended)});
                observations.push(
                    json!({"scenario": format!("{control}-{variant}"), "control": control,
                    "variant": variant, "initial": initial, "synchronous": synchronous,
                    "observed": observed, "api_errors": errors.borrow().clone()}),
                );
                let weak: Vec<_> = keep.iter().map(Element::downgrade).collect();
                drop(keep);
                drop(d);
                assert!(weak.iter().all(|e| e.upgrade().is_none()));
            }
        }
        suites.insert(suite.into(), json!(observations));
    }
    println!("{}", serde_json::to_string_pretty(&suites).unwrap());
}
