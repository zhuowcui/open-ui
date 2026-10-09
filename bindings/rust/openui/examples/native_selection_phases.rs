// Native Rust consumer of deferred selection notifications.
// Reproduces immutable v3062/v3084 scenarios with actual native state and events.
use openui::prelude::*;
use serde_json::{json, Value};
use std::cell::{Cell, RefCell};
use std::rc::Rc;

fn control(element: &Element) -> Value {
    let (start, end) = element.selection().unwrap().unwrap();
    json!({"value":element.control_value().unwrap().unwrap(),"start":start,"end":end,
        "direction":element.selection_direction().unwrap().unwrap().as_str(),
        "connected":element.is_connected().unwrap()})
}
fn state(a: &Element, b: &Element) -> Value {
    json!({"a":control(a),"b":control(b),"active":if a.has_focus().unwrap(){"a"}
        else if b.has_focus().unwrap(){"b"}else{"body"}})
}
fn barrier(document: &Document) {
    for _ in 0..64 {
        if !document.has_pending_events().unwrap() {
            return;
        }
        document.dispatch_pending_events().unwrap();
    }
    panic!("native selection callbacks failed to quiesce");
}
fn selection(e: &Element, start: usize, end: usize, backward: bool) {
    e.set_selection_range(
        start,
        end,
        if backward {
            SelectionDirection::Backward
        } else {
            SelectionDirection::Forward
        },
    )
    .unwrap();
}
fn once_mutation(
    a: &Element,
    b: &Element,
    kind: &str,
    target_b: bool,
    detach: bool,
    start: usize,
    end: usize,
    backward: bool,
) {
    let wa = a.downgrade();
    let wb = b.downgrade();
    let fired = Cell::new(false);
    a.on(kind, move |_| {
        if fired.replace(true) {
            return;
        }
        let target = if target_b {
            wb.upgrade().unwrap()
        } else {
            wa.upgrade().unwrap()
        };
        if detach {
            target.detach().unwrap();
        } else {
            selection(&target, start, end, backward);
        }
    })
    .unwrap();
}
fn main() {
    let variants = [
        "cross-a-b",
        "cross-b-a",
        "reenter-selectionchange-self",
        "reenter-select-self",
        "reenter-selectionchange-other-pending",
        "reenter-select-other",
        "detach-on-selectionchange",
        "direction-only",
        "select-other-pending",
        "select-self-two-pending",
        "selectionchange-other-delivered",
        "detach-on-select-two-pending",
    ];
    let mut observations = Vec::<Value>::new();
    for tag in ["input", "textarea"] {
        for variant in variants {
            let document = Document::new(320, 200).unwrap();
            let a = Element::create(&document, tag).unwrap();
            a.set_id("a").unwrap();
            let b = Element::create(&document, tag).unwrap();
            b.set_id("b").unwrap();
            document.body().append_child(&a).unwrap();
            document.body().append_child(&b).unwrap();
            let rows = Rc::new(RefCell::new(Vec::<Value>::new()));
            for element in [&a, &b] {
                for kind in ["select", "selectionchange"] {
                    let wa = a.downgrade();
                    let wb = b.downgrade();
                    let rows = rows.clone();
                    element
                        .on(kind, move |event| {
                            let a = wa.upgrade().unwrap();
                            let b = wb.upgrade().unwrap();
                            let mut row = state(&a, &b);
                            let row = row.as_object_mut().unwrap();
                            row.insert("type".into(), json!(event.event_type));
                            row.insert(
                                "target".into(),
                                json!(event
                                    .target()
                                    .unwrap()
                                    .get_attribute("id")
                                    .unwrap()
                                    .unwrap()),
                            );
                            row.insert("bubbles".into(), json!(event.bubbles()));
                            row.insert("cancelable".into(), json!(event.cancelable()));
                            rows.borrow_mut().push(Value::Object(row.clone()));
                        })
                        .unwrap();
                }
            }
            a.set_control_value("abcdef").unwrap();
            b.set_control_value("abcdef").unwrap();
            a.focus().unwrap();
            selection(&a, 1, 4, false);
            selection(&b, 1, 4, false);
            barrier(&document);
            rows.borrow_mut().clear();
            match variant {
                "cross-a-b" => {
                    selection(&a, 2, 5, false);
                    selection(&b, 0, 3, true);
                }
                "cross-b-a" => {
                    selection(&b, 0, 3, true);
                    selection(&a, 2, 5, false);
                }
                "reenter-selectionchange-self" => {
                    once_mutation(&a, &b, "selectionchange", false, false, 0, 3, false);
                    selection(&a, 2, 5, false);
                }
                "reenter-select-self" => {
                    once_mutation(&a, &b, "select", false, false, 0, 3, false);
                    selection(&a, 2, 5, false);
                }
                "reenter-selectionchange-other-pending" => {
                    once_mutation(&a, &b, "selectionchange", true, false, 2, 5, false);
                    selection(&a, 2, 5, false);
                    selection(&b, 0, 3, true);
                }
                "reenter-select-other" => {
                    once_mutation(&a, &b, "select", true, false, 0, 3, true);
                    selection(&a, 2, 5, false);
                }
                "detach-on-selectionchange" => {
                    once_mutation(&a, &b, "selectionchange", false, true, 0, 0, false);
                    selection(&a, 2, 5, false);
                }
                "direction-only" => selection(&a, 1, 4, true),
                "select-other-pending" => {
                    once_mutation(&a, &b, "select", true, false, 2, 5, false);
                    selection(&a, 2, 5, false);
                    selection(&b, 0, 3, true);
                }
                "select-self-two-pending" => {
                    once_mutation(&a, &b, "select", false, false, 1, 5, false);
                    selection(&a, 2, 5, false);
                    selection(&a, 0, 3, false);
                }
                "selectionchange-other-delivered" => {
                    once_mutation(&a, &b, "selectionchange", true, false, 2, 5, false);
                    selection(&b, 0, 3, true);
                    selection(&a, 2, 5, false);
                }
                "detach-on-select-two-pending" => {
                    once_mutation(&a, &b, "select", false, true, 0, 0, false);
                    selection(&a, 2, 5, false);
                    selection(&a, 0, 3, false);
                }
                _ => unreachable!(),
            }
            let synchronous = json!({"rows":rows.borrow().clone(),"state":state(&a,&b)});
            barrier(&document);
            let observed = json!({"rows":rows.borrow().clone(),"state":state(&a,&b)});
            observations.push(json!({"scenario":format!("{tag}-{variant}"),"control":tag,
                "variant":variant,"synchronous":synchronous,"observed":observed}));
            let weak_a = a.downgrade();
            let weak_b = b.downgrade();
            drop(a);
            drop(b);
            drop(document);
            assert!(weak_a.upgrade().is_none());
            assert!(weak_b.upgrade().is_none());
        }
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
