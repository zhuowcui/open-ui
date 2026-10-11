//! Consuming native app for the pinned Chromium focus-notification scenarios.
//! Build and run headlessly; stdout contains the observed callback sequence.

use openui::prelude::*;
use openui::{Event, EventPhase};
use std::cell::RefCell;
use std::rc::Rc;

const TYPES: [&str; 4] = ["focus", "focusin", "blur", "focusout"];
const SCENARIOS: [&str; 12] = [
    "ordinary",
    "blur-redirect",
    "focus-redirect",
    "focusout-redirect",
    "focusin-redirect",
    "blur-remove-pending",
    "blur-disable-pending",
    "target-capture-order",
    "capture-stop",
    "target-capture-stop",
    "target-capture-immediate-stop",
    "prevent-default",
];

#[derive(Debug)]
struct Observation {
    event_type: String,
    target: String,
    current: String,
    phase: u32,
    capture: bool,
    related: Option<String>,
    active: String,
    bubbles: bool,
    cancelable: bool,
    prevented: bool,
}

fn id(element: Element) -> String {
    element.get_attribute("id").unwrap().unwrap()
}

fn observe(scenario: &str) -> (Vec<Observation>, String) {
    let saved = Rc::new(RefCell::new(Vec::<Event>::new()));
    let rows = Rc::new(RefCell::new(Vec::new()));
    {
        let document = Document::new(800, 600).unwrap();
        let body = document.body();
        body.set_id("body").unwrap();
        let host = Element::create(&document, "div").unwrap();
        host.set_id("host").unwrap();
        body.append_child(&host).unwrap();
        let [a, b, c] = ["a", "b", "c"].map(|name| {
            let element = Element::create(&document, "input").unwrap();
            element.set_id(name).unwrap();
            host.append_child(&element).unwrap();
            element
        });
        for element in [&body, &host, &a, &b, &c] {
            for event_type in TYPES {
                // Deliberately register noncapture first, like the reference.
                for capture in [false, true] {
                    let rows = rows.clone();
                    let saved = saved.clone();
                    let document = document.clone();
                    let callback = move |event: &Event| {
                        rows.borrow_mut().push(Observation {
                            event_type: event.event_type.clone(),
                            target: id(event.target().unwrap()),
                            current: id(event.current_target().unwrap()),
                            phase: match event.phase().unwrap() {
                                EventPhase::Capture => 1,
                                EventPhase::Target => 2,
                                EventPhase::Bubble => 3,
                            },
                            capture,
                            related: event.related_target().map(id),
                            active: document
                                .focused_element()
                                .unwrap()
                                .map(id)
                                .unwrap_or_else(|| "body".into()),
                            bubbles: event.bubbles(),
                            cancelable: event.cancelable(),
                            prevented: event.default_prevented(),
                        });
                        saved.borrow_mut().push(event.clone());
                    };
                    if capture {
                        element.on_capture(event_type, callback).unwrap();
                    } else {
                        element.on(event_type, callback).unwrap();
                    }
                }
            }
        }
        match scenario {
            "blur-redirect" => {
                let next = c.clone();
                a.on("blur", move |_| next.focus().unwrap()).unwrap();
            }
            "focus-redirect" => {
                let next = b.clone();
                a.on("focus", move |_| next.focus().unwrap()).unwrap();
            }
            "focusout-redirect" => {
                let next = c.clone();
                a.on("focusout", move |_| next.focus().unwrap()).unwrap();
            }
            "focusin-redirect" => {
                let next = b.clone();
                a.on("focusin", move |_| next.focus().unwrap()).unwrap();
            }
            "blur-remove-pending" => {
                let next = b.clone();
                // Browser remove detaches; native remove destroys a handle.
                a.on("blur", move |_| next.detach().unwrap()).unwrap();
            }
            "blur-disable-pending" => {
                let next = b.clone();
                a.on("blur", move |_| next.set_attribute("disabled", "").unwrap())
                    .unwrap();
            }
            "capture-stop" => {
                host.on_capture("focus", Event::stop_propagation).unwrap();
            }
            "target-capture-stop" => {
                a.on_capture("focus", Event::stop_propagation).unwrap();
            }
            "target-capture-immediate-stop" => {
                a.on_capture("focus", Event::stop_immediate_propagation)
                    .unwrap();
            }
            "prevent-default" => {
                for event_type in TYPES {
                    a.on_capture(event_type, Event::prevent_default).unwrap();
                }
            }
            "ordinary" | "target-capture-order" => {}
            _ => unreachable!("fixed scenario inventory"),
        }
        a.focus().unwrap();
        if !matches!(scenario, "focus-redirect" | "focusin-redirect") {
            b.focus().unwrap();
        }
        if scenario == "ordinary" {
            b.focus().unwrap();
            a.blur().unwrap();
            b.blur().unwrap();
            b.blur().unwrap();
        }
        let final_focus = document
            .focused_element()
            .unwrap()
            .map(id)
            .unwrap_or_else(|| "body".into());
        assert!(!rows.borrow().is_empty());
        for event in saved.borrow().iter() {
            assert!(event.phase().is_none());
            assert!(event.current_target().is_none());
        }
        // Release callback-owned app handles, including detached input b.
        // Saved event values only retain weak target/related-target handles.
        for element in [&body, &host, &a, &b, &c] {
            for event_type in TYPES {
                element.remove_event(event_type).unwrap();
            }
        }
        rows.borrow_mut().push(Observation {
            event_type: "final".into(),
            target: final_focus,
            current: String::new(),
            phase: 0,
            capture: false,
            related: None,
            active: String::new(),
            bubbles: false,
            cancelable: false,
            prevented: false,
        });
    }
    assert!(
        saved
            .borrow()
            .iter()
            .all(|event| { event.target().is_none() && event.related_target().is_none() }),
        "saved event must not keep its native document alive"
    );
    let mut observations = std::mem::take(&mut *rows.borrow_mut());
    let final_focus = observations.pop().unwrap().target;
    (observations, final_focus)
}

fn main() {
    println!("[");
    for (index, scenario) in SCENARIOS.into_iter().enumerate() {
        let (rows, final_focus) = observe(scenario);
        println!("{{\"scenario\":{scenario:?},\"final\":{final_focus:?},\"rows\":[");
        for (position, row) in rows.iter().enumerate() {
            // Every string here is an assigned ASCII ID or fixed event name.
            let related = row
                .related
                .as_ref()
                .map_or_else(|| "null".into(), |id| format!("{id:?}"));
            println!(
                "{{\"type\":{:?},\"target\":{:?},\"current\":{:?},\"phase\":{},\"capture\":{},\"related\":{},\"active\":{:?},\"bubbles\":{},\"cancelable\":{},\"prevented\":{}}}{}",
                row.event_type, row.target, row.current, row.phase, row.capture,
                related, row.active, row.bubbles, row.cancelable, row.prevented,
                if position + 1 == rows.len() { "" } else { "," },
            );
        }
        println!(
            "]}}{}",
            if index + 1 == SCENARIOS.len() {
                ""
            } else {
                ","
            }
        );
    }
    println!("]");
}
