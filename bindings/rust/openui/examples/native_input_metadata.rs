//! Native Rust consumer of input event metadata.
use openui::prelude::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;

fn active(weak: &WeakElement) -> &'static str {
    if weak
        .upgrade()
        .is_some_and(|element| element.has_focus().unwrap())
    {
        "text"
    } else {
        "body"
    }
}
fn key(document: &Document, code: i32, name: &str, modifiers: Modifiers) {
    document
        .dispatch_key_event(KeyEventType::Down, code, Some(name), modifiers)
        .unwrap();
    document
        .dispatch_key_event(KeyEventType::Up, code, Some(name), modifiers)
        .unwrap();
}
fn main() {
    let cases: &[(&str, &str, &[&str])] = &[
        ("insert-input", "input", &["insert", "blur"]),
        ("delete-backward", "input", &["insert", "backspace", "blur"]),
        (
            "delete-forward",
            "input",
            &["insert", "home", "delete", "blur"],
        ),
        ("input-enter", "input", &["insert", "enter", "blur"]),
        ("textarea-enter", "textarea", &["insert", "enter", "blur"]),
        ("undo-redo", "input", &["insert", "undo", "redo", "blur"]),
        (
            "beforeinput-canceled",
            "input",
            &["cancel", "insert", "blur"],
        ),
        (
            "delete-word-backward",
            "input",
            &["insert-words", "word-backspace", "blur"],
        ),
        (
            "delete-word-forward",
            "input",
            &["insert-words", "home", "word-delete", "blur"],
        ),
        (
            "cut-keyboard",
            "input",
            &["insert-words", "select-all", "cut", "blur"],
        ),
        (
            "paste-keyboard",
            "input",
            &[
                "insert-words",
                "select-all",
                "copy",
                "home",
                "paste",
                "blur",
            ],
        ),
    ];
    let mut observations: Vec<Value> = Vec::new();
    for &(scenario, tag, actions) in cases {
        let document = Document::new(320, 200).unwrap();
        let text = Element::create(&document, tag).unwrap();
        text.set_id("text").unwrap();
        document.body().append_child(&text).unwrap();
        let weak = text.downgrade();
        let rows = Rc::new(RefCell::new(Vec::new()));
        let saved = Rc::new(RefCell::new(Vec::new()));
        for kind in ["beforeinput", "input", "change", "blur", "focusout"] {
            let weak = weak.clone();
            let rows = rows.clone();
            let saved = saved.clone();
            text.on(kind, move |event| {
                let info = event.input_info();
                rows.borrow_mut().push(json!({
                    "type": event.event_type,
                    "bubbles": event.bubbles(), "cancelable": event.cancelable(),
                    "value": event.target().unwrap().control_value().unwrap().unwrap(),
                    "active": active(&weak),
                    "input_type": info.map(|info| info.input_type().as_str()),
                    "data": info.and_then(|info| info.data()),
                    "is_composing": info.map(|info| info.is_composing()),
                }));
                if let Some(info) = info {
                    let original = info.clone();
                    let mut event_copy = event.clone();
                    event_copy.event_type = "changed-by-consumer".into();
                    event_copy.key_text = "changed-by-consumer".into();
                    event_copy.is_composing = !event_copy.is_composing;
                    assert_eq!(event_copy.input_info(), Some(&original));
                    saved.borrow_mut().push((event_copy, original));
                }
            })
            .unwrap();
        }
        text.focus().unwrap();
        for &action in actions {
            match action {
                "insert" => document.dispatch_text_input("A").unwrap(),
                "insert-words" => document.dispatch_text_input("one two").unwrap(),
                "word-backspace" => key(&document, 8, "Backspace", Modifiers::CTRL),
                "word-delete" => key(&document, 46, "Delete", Modifiers::CTRL),
                "select-all" => key(&document, 65, "a", Modifiers::CTRL),
                "cut" => key(&document, 88, "x", Modifiers::CTRL),
                "copy" => key(&document, 67, "c", Modifiers::CTRL),
                "paste" => key(&document, 86, "v", Modifiers::CTRL),
                "blur" => text.blur().unwrap(),
                "cancel" => {
                    text.on("beforeinput", |event| event.prevent_default())
                        .unwrap();
                }
                "backspace" => key(&document, 8, "Backspace", Modifiers::NONE),
                "home" => key(&document, 36, "Home", Modifiers::NONE),
                "delete" => key(&document, 46, "Delete", Modifiers::NONE),
                "enter" => key(&document, 13, "Enter", Modifiers::NONE),
                "undo" => key(&document, 90, "z", Modifiers::CTRL),
                "redo" => key(&document, 89, "y", Modifiers::CTRL),
                _ => unreachable!(),
            }
        }
        observations.push(json!({"scenario": scenario, "observed": {
            "rows": &*rows.borrow(), "value": text.control_value().unwrap().unwrap(),
            "active": active(&weak),
        }}));
        drop(text);
        drop(document);
        assert!(weak.upgrade().is_none());
        for (event, original) in saved.borrow().iter() {
            assert!(event.target().is_none());
            assert!(event.current_target().is_none());
            assert_eq!(event.input_info(), Some(original));
        }
    }
    println!("{}", serde_json::to_string(&observations).unwrap());
}
