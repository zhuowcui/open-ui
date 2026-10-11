use crate::prelude::*;
use crate::style::Error;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn associated_form_alias_retains_document_and_checks_stale_handles() {
    let document = Document::new(320, 200).unwrap();
    let form = Element::create(&document, "form").unwrap();
    let input = Element::create(&document, "input").unwrap();
    form.append_child(&input).unwrap();
    let alias = input.associated_form().unwrap().unwrap();
    assert!(alias.is_same_node(&form).unwrap());
    let weak = form.downgrade();
    drop(input);
    drop(form);
    drop(document);
    assert!(weak.upgrade().is_some());
    alias.remove().unwrap();
    assert!(alias.associated_form().is_err());
    assert!(weak.upgrade().is_none());
}

#[test]
fn associated_form_query_supports_native_callbacks_and_rejects_reentrant_borrows() {
    let document = Document::new(320, 200).unwrap();
    let form = Element::create(&document, "form").unwrap();
    let input = Element::create(&document, "input").unwrap();
    form.set_id("owner").unwrap();
    input.set_attribute("form", "owner").unwrap();
    document.body().append_child(&form).unwrap();
    document.body().append_child(&input).unwrap();
    let calls = Rc::new(Cell::new(0));
    let output = calls.clone();
    let weak = input.downgrade();
    input
        .on("click", move |_| {
            let input = weak.upgrade().unwrap();
            let form = input.associated_form().unwrap().unwrap();
            assert_eq!(form.get_attribute("id").unwrap().as_deref(), Some("owner"));
            form.set_id("renamed").unwrap();
            assert!(input.associated_form().unwrap().is_none());
            output.set(output.get() + 1);
        })
        .unwrap();
    input.click().unwrap();
    assert_eq!(calls.get(), 1);
    assert!(!document.has_pending_events().unwrap());
    let held = document.inner.engine.borrow_mut();
    assert!(matches!(
        input.associated_form(),
        Err(Error::ReentrantMutation)
    ));
    drop(held);
    assert!(input.associated_form().unwrap().is_none());
}

#[test]
fn native_form_associated_kinds_resolve_nearest_detached_and_explicit_connected_owner() {
    let document = Document::new(320, 200).unwrap();
    let form = Element::create(&document, "form").unwrap();
    form.set_id("owner").unwrap();
    document.body().append_child(&form).unwrap();
    let other = Element::create(&document, "form").unwrap();
    for kind in [
        "input", "button", "select", "textarea", "fieldset", "object",
    ] {
        let element = Element::create(&document, kind).unwrap();
        element.set_attribute("form", "owner").unwrap();
        other.append_child(&element).unwrap();
        assert!(element
            .associated_form()
            .unwrap()
            .unwrap()
            .is_same_node(&other)
            .unwrap());
        document.body().append_child(&other).unwrap();
        assert!(element
            .associated_form()
            .unwrap()
            .unwrap()
            .is_same_node(&form)
            .unwrap());
        other.detach().unwrap();
        element.remove().unwrap();
    }
    let div = Element::create(&document, "div").unwrap();
    div.set_attribute("form", "owner").unwrap();
    form.append_child(&div).unwrap();
    assert!(div.associated_form().unwrap().is_none());
}
