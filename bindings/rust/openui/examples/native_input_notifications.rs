//! Native input/change metadata and cancellation through public Rust APIs.
use openui::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let document = Document::new(320, 200).unwrap();
    let text = Element::create(&document, "input").unwrap();
    let checkbox = Element::create(&document, "input").unwrap();
    text.set_id("text").unwrap();
    checkbox.set_id("box").unwrap();
    checkbox.set_attribute("type", "checkbox").unwrap();
    document.body().append_child(&text).unwrap();
    document.body().append_child(&checkbox).unwrap();
    let rows = Rc::new(RefCell::new(Vec::new()));
    for element in [&text, &checkbox] {
        for kind in ["beforeinput", "input", "change"] {
            let rows = rows.clone();
            let id = element.get_attribute("id").unwrap().unwrap();
            element.on(kind, move |event| {
                let before = event.default_prevented();
                if event.event_type != "beforeinput" || event.key_text == "blocked" {
                    event.prevent_default();
                }
                rows.borrow_mut().push(format!(
                    r#"{{"type":"{}","target":"{}","bubbles":{},"cancelable":{},"before":{},"after":{}}}"#,
                    event.event_type, id, event.bubbles(), event.cancelable(),
                    before, event.default_prevented()
                ));
            }).unwrap();
        }
    }
    text.focus().unwrap();
    document.dispatch_text_input("A").unwrap();
    document.dispatch_text_input("blocked").unwrap();
    checkbox.click().unwrap();
    println!(
        r#"{{"rows":[{}],"value":"{}","checked":{}}}"#,
        rows.borrow().join(","),
        text.control_value().unwrap().unwrap(),
        checkbox.is_checked().unwrap()
    );
}
