// Native Rust consuming app. Chromium observations live in separate offline tooling.
use openui::prelude::*;
use serde_json::{json, Map, Value};
use std::cell::RefCell;
use std::rc::Rc;

const IDS: [&str; 12] = [
    "f",
    "l1",
    "l2",
    "normal",
    "nf",
    "first",
    "second",
    "a",
    "innerfirst",
    "inner",
    "b",
    "nl",
];
const FIELDS: [&str; 6] = ["first", "second", "a", "innerfirst", "inner", "b"];
const VARIANTS: [(&str, &str); 18] = [
    ("group-normal", "a"),
    ("group-first-legend", "first"),
    ("group-second-legend", "second"),
    ("group-nested-first", "innerfirst"),
    ("group-nested-normal", "inner"),
    ("inner-first-legend", "innerfirst"),
    ("inner-normal", "inner"),
    ("disable-enable", "a"),
    ("disable-twice", "a"),
    ("false-attribute", "a"),
    ("ordinary-container", "a"),
    ("blur-reenable", "a"),
    ("blur-redirect", "a"),
    ("edited-disable", "a"),
    ("legend-reorder", "first"),
    ("move-out-before-fixup", "a"),
    ("remove-fieldset", "a"),
    ("both-groups", "innerfirst"),
];
const NEIGHBORS: [(&str, &str); 14] = [
    ("append-control-same-parent", "a"),
    ("append-focused-subtree", "a"),
    ("append-fieldset-same-parent", "a"),
    ("append-control-out", "a"),
    ("insert-self-before", "a"),
    ("append-unrelated", "a"),
    ("insert-legend-before-first", "first"),
    ("remove-first-legend", "first"),
    ("remove-other-legend", "first"),
    ("first-hidden-legend", "first"),
    ("second-hidden-first", "second"),
    ("readonly-group", "a"),
    ("detached-group-state", "a"),
    ("move-disabled-legend-out", "first"),
];
fn find<'a>(tree: &'a [(String, Element)], id: &str) -> &'a Element {
    &tree.iter().find(|(name, _)| name == id).unwrap().1
}
fn make(d: &Document, tag: &str) -> Vec<(String, Element)> {
    let tree: Vec<_> = IDS
        .iter()
        .map(|id| {
            let kind = match *id {
                "f" | "nf" => "fieldset",
                "l1" | "l2" | "nl" => "legend",
                "normal" => "div",
                _ => tag,
            };
            let e = Element::create(d, kind).unwrap();
            if *id != "nl" {
                e.set_id(id).unwrap();
            }
            (id.to_string(), e)
        })
        .collect();
    d.body().append_child(find(&tree, "f")).unwrap();
    d.body().append_child(find(&tree, "b")).unwrap();
    for (parent, child) in [
        ("f", "l1"),
        ("l1", "first"),
        ("f", "l2"),
        ("l2", "second"),
        ("f", "normal"),
        ("normal", "a"),
        ("f", "nf"),
        ("nf", "nl"),
        ("nl", "innerfirst"),
        ("nf", "inner"),
    ] {
        find(&tree, parent)
            .append_child(find(&tree, child))
            .unwrap();
    }
    for id in FIELDS {
        let e = find(&tree, id);
        e.set_control_value("abcdef").unwrap();
        e.set_selection_range(2, 5, SelectionDirection::Forward)
            .unwrap();
    }
    tree
}
fn snapshot(tree: &[(String, Element)]) -> Value {
    let mut controls = Map::new();
    let mut active = "body";
    for id in FIELDS {
        let e = find(tree, id);
        let (start, end) = e.selection().unwrap().unwrap();
        if e.has_focus().unwrap() {
            active = id;
        }
        controls.insert(
            id.into(),
            json!({
                "value":e.control_value().unwrap().unwrap(),"start":start,"end":end,
                "direction":e.selection_direction().unwrap().unwrap().as_str(),
                "connected":e.is_connected().unwrap(),
                "ownDisabled":e.get_attribute("disabled").unwrap().is_some(),
                "disabledProperty":e.is_own_disabled().unwrap(),
                "effectivelyDisabled":e.is_effectively_disabled().unwrap(),
                "readonly":e.get_attribute("readonly").unwrap().is_some(),
            }),
        );
    }
    json!({"controls":controls,"active":active,
        "groupDisabled":find(tree,"f").get_attribute("disabled").unwrap().is_some(),
        "innerGroupDisabled":find(tree,"nf").get_attribute("disabled").unwrap().is_some()})
}
fn barrier(d: &Document) {
    for _ in 0..64 {
        if !d.has_pending_events().unwrap() {
            return;
        }
        d.dispatch_pending_events().unwrap();
    }
    panic!("native task queue did not quiesce");
}
fn disabled(tree: &[(String, Element)], id: &str) {
    find(tree, id).set_attribute("disabled", "").unwrap();
}
fn main() {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "focus".into());
    if mode == "state" {
        state_main();
        return;
    }
    assert!(mode == "focus" || mode == "neighbors");
    let variants = if mode == "focus" {
        &VARIANTS[..]
    } else {
        &NEIGHBORS[..]
    };
    let mut observations = Vec::new();
    for tag in ["input", "textarea"] {
        for &(variant, focus_id) in variants {
            let d = Document::new(800, 600).unwrap();
            let tree = make(&d, tag);
            let rows = Rc::new(RefCell::new(Vec::<Value>::new()));
            for id in FIELDS {
                for kind in ["change", "blur", "focusout", "focus", "focusin"] {
                    let weak: Vec<_> = tree
                        .iter()
                        .map(|(id, e)| (id.clone(), e.downgrade()))
                        .collect();
                    let observed = rows.clone();
                    find(&tree, id)
                        .on(kind, move |event| {
                            let tree: Vec<_> = weak
                                .iter()
                                .map(|(id, e)| (id.clone(), e.upgrade().unwrap()))
                                .collect();
                            let mut row = snapshot(&tree);
                            let object = row.as_object_mut().unwrap();
                            object.insert("type".into(), json!(event.event_type));
                            object.insert(
                                "target".into(),
                                json!(event
                                    .target()
                                    .unwrap()
                                    .get_attribute("id")
                                    .unwrap()
                                    .unwrap()),
                            );
                            object.insert(
                                "related".into(),
                                json!(event
                                    .related_target()
                                    .and_then(|e| e.get_attribute("id").unwrap())),
                            );
                            object.insert("bubbles".into(), json!(event.bubbles()));
                            object.insert("cancelable".into(), json!(event.cancelable()));
                            observed.borrow_mut().push(row);
                        })
                        .unwrap();
                }
            }
            barrier(&d);
            match variant {
                "legend-reorder"
                | "insert-legend-before-first"
                | "remove-first-legend"
                | "remove-other-legend"
                | "move-disabled-legend-out" => disabled(&tree, "f"),
                "first-hidden-legend" | "second-hidden-first" => find(&tree, "l1")
                    .set_property(StyleProperty::Display, Display::None.into())
                    .unwrap(),
                "readonly-group" => find(&tree, "a").set_attribute("readonly", "").unwrap(),
                "blur-reenable" => {
                    let f = find(&tree, "f").downgrade();
                    find(&tree, "a")
                        .on("blur", move |_| {
                            f.upgrade().unwrap().remove_attribute("disabled").unwrap();
                        })
                        .unwrap();
                }
                "blur-redirect" => {
                    let b = find(&tree, "b").downgrade();
                    find(&tree, "a")
                        .on("blur", move |_| b.upgrade().unwrap().focus().unwrap())
                        .unwrap();
                }
                _ => {}
            }
            let focused = find(&tree, focus_id).focus();
            if variant == "first-hidden-legend" {
                assert!(matches!(
                    focused,
                    Err(Error::Engine(openui_engine::EngineError::NotFocusable))
                ));
            } else {
                focused.unwrap();
            }
            barrier(&d);
            if variant == "edited-disable" {
                d.dispatch_text_input("X").unwrap();
            }
            let initial = snapshot(&tree);
            rows.borrow_mut().clear();
            match variant {
                "inner-first-legend" | "inner-normal" => disabled(&tree, "nf"),
                "ordinary-container" => disabled(&tree, "normal"),
                "false-attribute" => find(&tree, "f").set_attribute("disabled", "false").unwrap(),
                "disable-enable" => {
                    disabled(&tree, "f");
                    find(&tree, "f").remove_attribute("disabled").unwrap();
                }
                "disable-twice" => {
                    disabled(&tree, "f");
                    disabled(&tree, "f");
                }
                "legend-reorder" => find(&tree, "f").append_child(find(&tree, "l1")).unwrap(),
                "move-out-before-fixup" => {
                    disabled(&tree, "f");
                    d.body().append_child(find(&tree, "a")).unwrap();
                }
                "remove-fieldset" => find(&tree, "f").detach().unwrap(),
                "both-groups" => {
                    disabled(&tree, "nf");
                    disabled(&tree, "f");
                }
                "append-control-same-parent" => find(&tree, "normal")
                    .append_child(find(&tree, "a"))
                    .unwrap(),
                "append-focused-subtree" => find(&tree, "f")
                    .append_child(find(&tree, "normal"))
                    .unwrap(),
                "append-fieldset-same-parent" => d.body().append_child(find(&tree, "f")).unwrap(),
                "append-control-out" => d.body().append_child(find(&tree, "a")).unwrap(),
                "insert-self-before" => find(&tree, "normal")
                    .insert_before(find(&tree, "a"), find(&tree, "a"))
                    .unwrap(),
                "append-unrelated" => find(&tree, "f").append_child(find(&tree, "l2")).unwrap(),
                "insert-legend-before-first" => find(&tree, "f")
                    .insert_before(find(&tree, "l2"), find(&tree, "l1"))
                    .unwrap(),
                "remove-first-legend" => find(&tree, "l1").detach().unwrap(),
                "remove-other-legend" => find(&tree, "l2").detach().unwrap(),
                "detached-group-state" => {
                    find(&tree, "f").detach().unwrap();
                    disabled(&tree, "f");
                }
                "move-disabled-legend-out" => d.body().append_child(find(&tree, "l1")).unwrap(),
                _ => disabled(&tree, "f"),
            }
            let synchronous = json!({"rows":rows.borrow().clone(),"state":snapshot(&tree)});
            barrier(&d);
            let observed = json!({"rows":rows.borrow().clone(),"state":snapshot(&tree)});
            observations.push(json!({"scenario":format!("{tag}-{variant}"),"control":tag,"variant":variant,
                "focus_id":focus_id,"initial":initial,"synchronous":synchronous,"observed":observed}));
            let weak: Vec<_> = tree.iter().map(|(_, e)| e.downgrade()).collect();
            drop(tree);
            drop(d);
            assert!(weak.iter().all(|e| e.upgrade().is_none()));
        }
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
const STATE_VARIANTS: [&str; 12] = [
    "all-enabled",
    "outer-disabled",
    "inner-disabled",
    "both-disabled",
    "optgroup-disabled",
    "select-disabled",
    "option-own",
    "option-move-out",
    "option-wrapper",
    "legend-hidden",
    "legend-second-first",
    "ordinary-disabled",
];
fn state_snapshot(tree: &[(String, Element)], d: &Document) -> Value {
    let mut result = Map::new();
    for id in [
        "f",
        "nf",
        "first",
        "second",
        "a",
        "innerfirst",
        "inner",
        "b",
        "s",
        "og",
        "o",
        "o2",
        "normal",
    ] {
        let e = find(tree, id);
        let kind = match e.kind().unwrap() {
            ElementTag::Fieldset => "fieldset",
            ElementTag::Input => "input",
            ElementTag::TextArea => "textarea",
            ElementTag::Select => "select",
            ElementTag::OptGroup => "optgroup",
            ElementTag::Option => "option",
            ElementTag::Div => "div",
            _ => panic!("unexpected native element kind"),
        };
        let supports = matches!(
            e.kind().unwrap(),
            ElementTag::Fieldset
                | ElementTag::Input
                | ElementTag::TextArea
                | ElementTag::Select
                | ElementTag::OptGroup
                | ElementTag::Option
        );
        let parent = e.parent().unwrap().map(|parent| {
            if parent.is_same_node(&d.body()).unwrap() {
                "body".to_string()
            } else {
                parent.get_attribute("id").unwrap().unwrap_or_else(|| {
                    match parent.kind().unwrap() {
                        ElementTag::Legend => "legend".into(),
                        _ => panic!("unexpected parent"),
                    }
                })
            }
        });
        let disabled = e.is_effectively_disabled().unwrap();
        result.insert(id.into(),json!({"kind":kind,"own":e.get_attribute("disabled").unwrap().is_some(),
            "property":if supports{Some(e.is_own_disabled().unwrap())}else{None},"effective":disabled,
            "enabled":supports&&!disabled,"connected":e.is_connected().unwrap(),"parent":parent}));
    }
    Value::Object(result)
}
fn state_main() {
    let mut observations = Vec::new();
    for tag in ["input", "textarea"] {
        for variant in STATE_VARIANTS {
            let d = Document::new(800, 600).unwrap();
            let mut tree = make(&d, tag);
            for (id, kind, parent) in [
                ("s", "select", "f"),
                ("og", "optgroup", "s"),
                ("o", "option", "og"),
                ("o2", "option", "s"),
                ("wrap", "span", "s"),
            ] {
                let e = Element::create(&d, kind).unwrap();
                e.set_id(id).unwrap();
                find(&tree, parent).append_child(&e).unwrap();
                tree.push((id.into(), e));
            }
            barrier(&d);
            if variant == "option-move-out" || variant == "option-wrapper" {
                disabled(&tree, "og");
            }
            let initial = state_snapshot(&tree, &d);
            match variant {
                "all-enabled" => {}
                "outer-disabled" => disabled(&tree, "f"),
                "inner-disabled" => disabled(&tree, "nf"),
                "both-disabled" => {
                    disabled(&tree, "f");
                    disabled(&tree, "nf");
                }
                "optgroup-disabled" => disabled(&tree, "og"),
                "select-disabled" => disabled(&tree, "s"),
                "option-own" => disabled(&tree, "o"),
                "option-move-out" => find(&tree, "s").append_child(find(&tree, "o")).unwrap(),
                "option-wrapper" => find(&tree, "wrap").append_child(find(&tree, "o")).unwrap(),
                "legend-hidden" => {
                    find(&tree, "l1")
                        .set_property(StyleProperty::Display, Display::None.into())
                        .unwrap();
                    disabled(&tree, "f");
                }
                "legend-second-first" => {
                    disabled(&tree, "f");
                    find(&tree, "f")
                        .insert_before(find(&tree, "l2"), find(&tree, "l1"))
                        .unwrap();
                }
                "ordinary-disabled" => disabled(&tree, "normal"),
                _ => unreachable!(),
            }
            let synchronous = state_snapshot(&tree, &d);
            barrier(&d);
            let observed = state_snapshot(&tree, &d);
            observations.push(
                json!({"scenario":format!("{tag}-{variant}"),"control":tag,"variant":variant,
                "initial":initial,"synchronous":synchronous,"observed":observed}),
            );
            let weak: Vec<_> = tree.iter().map(|(_, e)| e.downgrade()).collect();
            drop(tree);
            drop(d);
            assert!(weak.iter().all(|e| e.upgrade().is_none()));
        }
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
