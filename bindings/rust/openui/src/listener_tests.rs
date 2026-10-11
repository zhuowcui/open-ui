use super::*;

fn mounted(document: &Document) -> Element {
    let element = Element::create(document, "div").unwrap();
    document.body().append_child(&element).unwrap();
    element
}

#[test]
fn native_removal_readdition_and_nested_dispatch_use_live_registration_identity() {
    for nested in [false, true] {
        let document = Document::new(100, 80).unwrap();
        let node = mounted(&document);
        let log = Rc::new(RefCell::new(Vec::new()));
        let target = node.clone();
        let observed = log.clone();
        node.on("click", move |_| {
            observed.borrow_mut().push(1);
            target.remove_event("click").unwrap();
            let next = observed.clone();
            target
                .on("click", move |_| next.borrow_mut().push(3))
                .unwrap();
            if nested {
                target.dispatch_click_event().unwrap();
            }
        })
        .unwrap();
        let observed = log.clone();
        node.on("click", move |_| observed.borrow_mut().push(2))
            .unwrap();
        assert!(node.dispatch_click_event().unwrap());
        assert_eq!(*log.borrow(), if nested { vec![1, 3] } else { vec![1] });
        log.borrow_mut().clear();
        assert!(node.dispatch_click_event().unwrap());
        assert_eq!(*log.borrow(), vec![3]);
        node.remove_event("click").unwrap();
    }
}

#[test]
fn node_removal_skips_remaining_callbacks_and_a_borrow_failure_keeps_node_live() {
    let document = Document::new(100, 80).unwrap();
    let node = mounted(&document);
    let log = Rc::new(RefCell::new(Vec::new()));
    let target = node.clone();
    let observed = log.clone();
    node.on("click", move |_| {
        observed.borrow_mut().push(1);
        target.remove().unwrap();
    })
    .unwrap();
    let observed = log.clone();
    node.on("click", move |_| observed.borrow_mut().push(2))
        .unwrap();
    {
        let held = document.inner.listeners.borrow();
        assert!(matches!(node.remove(), Err(Error::ReentrantMutation)));
        assert_eq!(held.values().map(Vec::len).sum::<usize>(), 2);
        assert!(node.get_attribute("id").is_ok());
    }
    assert!(node.dispatch_click_event().unwrap());
    assert_eq!(*log.borrow(), vec![1]);
    assert!(document.inner.listeners.borrow().is_empty());
}

struct DropProbe {
    node: Element,
    succeeded: Rc<Cell<bool>>,
}

impl Drop for DropProbe {
    fn drop(&mut self) {
        self.succeeded
            .set(self.node.on("drop-observed", |_| {}).is_ok());
    }
}

#[test]
fn removed_rust_capture_destructors_can_reenter_document_listener_storage() {
    let document = Document::new(100, 80).unwrap();
    let node = mounted(&document);
    let succeeded = Rc::new(Cell::new(false));
    let probe = DropProbe {
        node: node.clone(),
        succeeded: succeeded.clone(),
    };
    node.on("click", move |_| {
        let _ = &probe;
    })
    .unwrap();
    node.remove_event("click").unwrap();
    assert!(succeeded.get());
    node.remove_event("drop-observed").unwrap();
}

#[cfg(feature = "ffi-integration")]
#[test]
fn foreign_pointer_aliases_do_not_merge_rust_mouse_and_pointer_registrations() {
    for (mouse, pointer) in [
        ("mousedown", "pointerdown"),
        ("mouseup", "pointerup"),
        ("mousemove", "pointermove"),
        ("mouseenter", "pointerenter"),
        ("mouseleave", "pointerleave"),
        ("mousecancel", "pointercancel"),
    ] {
        let document = Document::new(100, 80).unwrap();
        let node = mounted(&document);
        let log = Rc::new(RefCell::new(Vec::new()));
        let observed = log.clone();
        node.on(pointer, move |_| observed.borrow_mut().push(1))
            .unwrap();
        let observed = log.clone();
        node.on(mouse, move |_| observed.borrow_mut().push(2))
            .unwrap();
        let observed = log.clone();
        let id = document
            .add_foreign_listener_for_native_facade(
                node.handle,
                mouse,
                false,
                Rc::new(move |_, _, _, _| {
                    observed.borrow_mut().push(3);
                    Ok(())
                }),
            )
            .unwrap();
        assert!(document
            .is_foreign_listener_registered_for_native_facade(id, node.handle, mouse, false)
            .unwrap());
        assert!(document
            .is_foreign_listener_registered_for_native_facade(id, node.handle, pointer, false)
            .unwrap());
        assert!(!document
            .is_foreign_listener_registered_for_native_facade(id, node.handle, pointer, true)
            .unwrap());
        let event = Event::pointer(mouse, 0, 0.0, 0.0, MouseButton::Left, Modifiers::NONE);
        document
            .invoke(node.handle, node.handle, &event, Some(false))
            .unwrap();
        assert_eq!(*log.borrow(), vec![2, 3]);
        log.borrow_mut().clear();
        let event = Event::pointer(pointer, 0, 0.0, 0.0, MouseButton::Left, Modifiers::NONE);
        document
            .invoke(node.handle, node.handle, &event, Some(false))
            .unwrap();
        assert_eq!(*log.borrow(), vec![1, 3]);
        assert!(document.remove_listener_for_native_facade(id).unwrap());
        assert!(!document
            .is_listener_registered_for_native_facade(id)
            .unwrap());
        node.remove_event(mouse).unwrap();
        node.remove_event(pointer).unwrap();
    }
}

#[cfg(feature = "ffi-integration")]
#[test]
fn shared_foreign_add_remove_and_extract_are_transactional_and_defer_capture_drop() {
    let document = Document::new(100, 80).unwrap();
    let node = mounted(&document);
    let succeeded = Rc::new(Cell::new(false));
    let probe = DropProbe {
        node: node.clone(),
        succeeded: succeeded.clone(),
    };
    let id = document
        .add_foreign_listener_for_native_facade(
            node.handle,
            "click",
            false,
            Rc::new(move |_, _, _, _| {
                let _ = &probe;
                Ok(())
            }),
        )
        .unwrap();
    {
        let held = document.inner.listeners.borrow_mut();
        assert!(matches!(
            document.add_foreign_listener_for_native_facade(
                node.handle,
                "click",
                false,
                Rc::new(|_, _, _, _| Ok(()))
            ),
            Err(Error::ReentrantMutation)
        ));
        assert!(matches!(
            document.remove_listener_for_native_facade(id),
            Err(Error::ReentrantMutation)
        ));
        assert!(matches!(
            document.is_listener_registered_for_native_facade(id),
            Err(Error::ReentrantMutation)
        ));
        assert_eq!(held.values().map(Vec::len).sum::<usize>(), 1);
    }
    assert!(document
        .is_listener_registered_for_native_facade(id)
        .unwrap());
    let removed = document
        .take_foreign_listeners_for_native_facade(&[id, id])
        .unwrap();
    assert_eq!(removed.len(), 1);
    assert!(!document
        .is_listener_registered_for_native_facade(id)
        .unwrap());
    assert!(!succeeded.get());
    drop(removed);
    assert!(succeeded.get());
    assert!(!document.remove_listener_for_native_facade(id).unwrap());
    let next = document
        .add_foreign_listener_for_native_facade(
            node.handle,
            "click",
            false,
            Rc::new(|_, _, _, _| Ok(())),
        )
        .unwrap();
    assert!(next > id);
    assert!(document.remove_listener_for_native_facade(next).unwrap());
    node.remove_event("drop-observed").unwrap();
}

#[cfg(feature = "ffi-integration")]
#[test]
fn legacy_hook_additions_wait_until_next_invocation() {
    let document = Document::new(100, 80).unwrap();
    let node = mounted(&document);
    let log = Rc::new(RefCell::new(Vec::new()));
    let observed = log.clone();
    node.on("click", move |_| observed.borrow_mut().push(1))
        .unwrap();
    let target = node.clone();
    let observed = log.clone();
    let once = Cell::new(false);
    document
        .set_foreign_event_handler(Rc::new(move |_, _, _, _| {
            if !once.replace(true) {
                let observed = observed.clone();
                target.on("click", move |_| observed.borrow_mut().push(2))?;
            }
            Ok(())
        }))
        .unwrap();
    let event = Event::pointer("click", 0, 0.0, 0.0, MouseButton::Left, Modifiers::NONE);
    document
        .invoke(node.handle, node.handle, &event, Some(false))
        .unwrap();
    assert_eq!(*log.borrow(), vec![1]);
    log.borrow_mut().clear();
    document
        .invoke(node.handle, node.handle, &event, Some(false))
        .unwrap();
    assert_eq!(*log.borrow(), vec![1, 2]);
    document
        .set_foreign_event_handler(Rc::new(|_, _, _, _| Ok(())))
        .unwrap();
    node.remove_event("click").unwrap();
}
