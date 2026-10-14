//! Native Rust app for ASCII range modes and state/error neighbors.
//! Requires shared range replacement and owning-thread deferred event delivery APIs.
use openui::prelude::*;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let mut observations = Vec::<Value>::new();
    for tag in ["input", "textarea"] {
        for (name, mode) in [
            ("preserve", RangeSelectionMode::Preserve),
            ("select", RangeSelectionMode::Select),
            ("start", RangeSelectionMode::Start),
            ("end", RangeSelectionMode::End),
            ("readonly", RangeSelectionMode::Preserve),
            ("disabled", RangeSelectionMode::Preserve),
            ("reverse", RangeSelectionMode::Preserve),
            ("beyond", RangeSelectionMode::Preserve),
            ("newline", RangeSelectionMode::Preserve),
            ("direction-backward", RangeSelectionMode::Preserve),
        ] {
            let document = Document::new(320, 200).unwrap();
            let text = Element::create(&document, tag).unwrap();
            text.set_id("text").unwrap();
            document.body().append_child(&text).unwrap();
            let rows = Rc::new(RefCell::new(Vec::<Value>::new()));
            for kind in [
                "beforeinput",
                "input",
                "change",
                "select",
                "selectionchange",
                "blur",
                "focusout",
                "focus",
                "focusin",
            ] {
                let rows = rows.clone();
                let weak = text.downgrade();
                text.on(kind, move |event| {
                    let text = weak.upgrade().unwrap();
                    let (start, end) = text.selection().unwrap().unwrap();
                    let metadata = event.input_info();
                    rows.borrow_mut().push(json!({
                        "type": event.event_type,
                        "bubbles": event.bubbles(),
                        "cancelable": event.cancelable(),
                        "value": text.control_value().unwrap().unwrap(),
                        "active": if text.has_focus().unwrap() { "text" } else { "body" },
                        "input_type": metadata.map(|info| info.input_type().as_str()),
                        "data": metadata.and_then(|info| info.data()),
                        "is_composing": metadata.map(|info| info.is_composing()),
                        "related": event.related_target().and_then(|element| element.get_attribute("id").unwrap()),
                        "selection_start": start,
                        "selection_end": end,
                        "direction": text.selection_direction().unwrap().unwrap().as_str(),
                    }));
                }).unwrap();
            }
            text.focus().unwrap();
            text.set_control_value("abcdef").unwrap();
            text.set_selection_range(
                2,
                5,
                if name == "direction-backward" {
                    SelectionDirection::Backward
                } else {
                    SelectionDirection::Forward
                },
            )
            .unwrap();
            if name == "readonly" {
                text.set_attribute("readonly", "").unwrap();
            }
            if name == "disabled" {
                text.set_attribute("disabled", "").unwrap();
            }
            // Pending native task delivery is part of the required public API.
            // Match the completed Chromium reference's setup frame barrier.
            document.dispatch_pending_events().unwrap();
            rows.borrow_mut().clear();
            let (start, end) = if name == "reverse" {
                (5, 2)
            } else if name == "beyond" {
                (90, 100)
            } else {
                (1, 4)
            };
            let replacement = if name == "newline" { "X\nY\rZ" } else { "XY" };
            let error = text
                .replace_control_range(replacement, start, end, mode)
                .err()
                .map(|error| format!("{error:?}"));
            document.dispatch_pending_events().unwrap();
            let (start, end) = text.selection().unwrap().unwrap();
            observations.push(json!({
                "scenario": format!("{tag}-{name}"),
                "observed": {
                    "rows": &*rows.borrow(),
                    "value": text.control_value().unwrap().unwrap(),
                    "active": if text.has_focus().unwrap() { "text" } else { "body" },
                    "selection_start": start,
                    "selection_end": end,
                    "direction": text.selection_direction().unwrap().unwrap().as_str(),
                    "error": error,
                },
            }));
        }
    }
    println!("{}", serde_json::to_string_pretty(&observations).unwrap());
}
