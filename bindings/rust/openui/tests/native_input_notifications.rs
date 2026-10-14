use openui::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

#[test]
fn input_notification_cannot_cancel_or_relabel_a_completed_native_edit() {
    let document = Document::new(320, 200).unwrap();
    let text = Element::create(&document, "input").unwrap();
    document.body().append_child(&text).unwrap();
    let saved = Rc::new(RefCell::new(None));
    let retained = saved.clone();
    text.on("input", move |event| {
        assert!(!event.cancelable());
        event.prevent_default();
        assert!(!event.default_prevented());
        let mut clone = event.clone();
        clone.event_type = "beforeinput".into();
        clone.prevent_default();
        assert!(!clone.default_prevented());
        *retained.borrow_mut() = Some(clone);
    })
    .unwrap();
    text.focus().unwrap();
    document.dispatch_text_input("A").unwrap();
    assert_eq!(text.control_value().unwrap().as_deref(), Some("A"));
    let saved = saved.borrow_mut().take().unwrap();
    assert_eq!(saved.phase(), None);
    assert!(saved.current_target().is_none());
    drop(text);
    drop(document);
    assert!(saved.target().is_none());
}
