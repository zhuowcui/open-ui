// Native Rust consuming app: disabled attributes and callback focus transitions.
// Pinned Chromium supplies the separate offline expected observations.
use openui::prelude::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;
fn control(e: &Element) -> Value {
    let (start, end) = e.selection().unwrap().unwrap();
    json!({"value":e.control_value().unwrap().unwrap(),"start":start,"end":end,
        "direction":e.selection_direction().unwrap().unwrap().as_str(),
        "connected":e.is_connected().unwrap(),"disabled":e.get_attribute("disabled").unwrap().is_some(),
        "readonly":e.get_attribute("readonly").unwrap().is_some()})
}
fn snapshot(a: &Element, b: &Element) -> Value {
    json!({"a":control(a),"b":control(b),"active":if a.has_focus().unwrap(){"a"}else if b.has_focus().unwrap(){"b"}else{"body"}})
}
fn barrier(d: &Document) {
    for _ in 0..64 {
        if !d.has_pending_events().unwrap() {
            return;
        }
        d.dispatch_pending_events().unwrap();
    }
    panic!("native tasks did not quiesce");
}
fn main() {
    let variants = [
        "disable-focused",
        "disable-unfocused",
        "readonly-focused",
        "disable-uppercase",
        "disable-false-string",
        "disable-enable",
        "disable-twice",
        "blur-reenable",
        "blur-redirect",
        "focus-disables",
        "edited-disable",
    ];
    let mut observations = Vec::new();
    for tag in ["input", "textarea"] {
        for variant in variants {
            let d = Document::new(320, 200).unwrap();
            let a = Element::create(&d, tag).unwrap();
            let b = Element::create(&d, tag).unwrap();
            a.set_id("a").unwrap();
            b.set_id("b").unwrap();
            d.body().append_child(&a).unwrap();
            d.body().append_child(&b).unwrap();
            let rows = Rc::new(RefCell::new(Vec::<Value>::new()));
            for e in [&a, &b] {
                for kind in ["change", "blur", "focusout", "focus", "focusin"] {
                    let wa = a.downgrade();
                    let wb = b.downgrade();
                    let rows = rows.clone();
                    e.on(kind, move |event| {
                        let a = wa.upgrade().unwrap();
                        let b = wb.upgrade().unwrap();
                        let mut row = snapshot(&a, &b);
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
                        map.insert(
                            "related".into(),
                            json!(event
                                .related_target()
                                .and_then(|e| e.get_attribute("id").unwrap())),
                        );
                        map.insert("bubbles".into(), json!(event.bubbles()));
                        map.insert("cancelable".into(), json!(event.cancelable()));
                        rows.borrow_mut().push(row);
                    })
                    .unwrap();
                }
            }
            a.set_control_value("abcdef").unwrap();
            b.set_control_value("abcdef").unwrap();
            a.focus().unwrap();
            a.set_selection_range(2, 5, SelectionDirection::Forward)
                .unwrap();
            b.set_selection_range(1, 4, SelectionDirection::Forward)
                .unwrap();
            barrier(&d);
            match variant {
                "blur-reenable" => {
                    let a = a.downgrade();
                    a.upgrade()
                        .unwrap()
                        .on("blur", move |_| {
                            a.upgrade().unwrap().remove_attribute("disabled").unwrap();
                        })
                        .unwrap();
                }
                "blur-redirect" => {
                    let b = b.downgrade();
                    a.on("blur", move |_| b.upgrade().unwrap().focus().unwrap())
                        .unwrap();
                }
                "focus-disables" => {
                    b.focus().unwrap();
                    let target = a.downgrade();
                    a.on("focus", move |_| {
                        target
                            .upgrade()
                            .unwrap()
                            .set_attribute("disabled", "")
                            .unwrap()
                    })
                    .unwrap();
                    barrier(&d);
                }
                _ => {}
            }
            rows.borrow_mut().clear();
            if variant == "edited-disable" {
                d.dispatch_text_input("X").unwrap();
            }
            match variant {
                "disable-unfocused" => b.set_attribute("disabled", "").unwrap(),
                "readonly-focused" => a.set_attribute("readonly", "").unwrap(),
                "disable-uppercase" => a.set_attribute("DiSaBlEd", "").unwrap(),
                "disable-false-string" => a.set_attribute("disabled", "false").unwrap(),
                "disable-enable" => {
                    a.set_attribute("disabled", "").unwrap();
                    a.remove_attribute("disabled").unwrap();
                }
                "disable-twice" => {
                    a.set_attribute("disabled", "").unwrap();
                    a.set_attribute("disabled", "").unwrap();
                }
                "focus-disables" => a.focus().unwrap(),
                _ => a.set_attribute("disabled", "").unwrap(),
            }
            let synchronous = json!({"rows":rows.borrow().clone(),"state":snapshot(&a,&b)});
            barrier(&d);
            let observed = json!({"rows":rows.borrow().clone(),"state":snapshot(&a,&b)});
            observations.push(json!({"scenario":format!("{tag}-{variant}"),"control":tag,"variant":variant,"synchronous":synchronous,"observed":observed}));
            let wa = a.downgrade();
            let wb = b.downgrade();
            drop(a);
            drop(b);
            drop(d);
            assert!(wa.upgrade().is_none());
            assert!(wb.upgrade().is_none());
        }
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
