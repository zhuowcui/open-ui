// Native Rust consumer of value and selection operations.
// Reproduces immutable v3062/v3084 scenarios with actual native state and events.
use openui::prelude::*;
use serde_json::{json, Value};
use std::cell::RefCell;
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
fn main() {
    let variants = [
        ("value-changed-same-length-caret-end", 6, 6, false),
        ("value-changed-same-length-range", 1, 4, false),
        ("value-identical-forward-range", 1, 4, false),
        ("value-identical-backward-range", 1, 4, true),
        ("value-shorter-backward-range", 1, 4, true),
        ("range-identical-forward-preserve", 1, 4, false),
        ("range-identical-backward-preserve", 1, 4, true),
        ("range-changed-same-length-preserve", 1, 4, false),
        ("range-changed-caret-tail-preserve", 6, 6, false),
        ("range-shorter-backward-end", 4, 6, true),
        ("selection-none-from-backward", 1, 4, true),
        ("value-change-then-restore-selection", 1, 4, false),
    ];
    let mut observations = Vec::<Value>::new();
    for tag in ["input", "textarea"] {
        for (variant, start, end, backward) in variants {
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
            selection(&a, start, end, backward);
            selection(&b, 1, 4, false);
            barrier(&document);
            rows.borrow_mut().clear();
            match variant {
                "value-changed-same-length-caret-end" | "value-changed-same-length-range" => {
                    a.set_control_value("ghijkl").unwrap()
                }
                "value-identical-forward-range" | "value-identical-backward-range" => {
                    a.set_control_value("abcdef").unwrap()
                }
                "value-shorter-backward-range" => a.set_control_value("xyz").unwrap(),
                "range-identical-forward-preserve" | "range-identical-backward-preserve" => a
                    .replace_control_range("bcd", 1, 4, RangeSelectionMode::Preserve)
                    .unwrap(),
                "range-changed-same-length-preserve" | "range-changed-caret-tail-preserve" => a
                    .replace_control_range("ZZ", 1, 3, RangeSelectionMode::Preserve)
                    .unwrap(),
                "range-shorter-backward-end" => a
                    .replace_control_range("x", 0, 6, RangeSelectionMode::End)
                    .unwrap(),
                "selection-none-from-backward" => a
                    .set_selection_range(1, 4, SelectionDirection::None)
                    .unwrap(),
                "value-change-then-restore-selection" => {
                    a.set_control_value("ghijkl").unwrap();
                    selection(&a, 1, 4, false);
                }
                _ => unreachable!(),
            }
            let synchronous = json!({"rows":rows.borrow().clone(),"state":state(&a,&b)});
            barrier(&document);
            let observed = json!({"rows":rows.borrow().clone(),"state":state(&a,&b)});
            observations.push(json!({"scenario":format!("{tag}-{variant}"),"control":tag,
                "variant":variant,"synchronous":synchronous,"observed":observed}));
            let wa = a.downgrade();
            let wb = b.downgrade();
            drop(a);
            drop(b);
            drop(document);
            assert!(wa.upgrade().is_none());
            assert!(wb.upgrade().is_none());
        }
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
