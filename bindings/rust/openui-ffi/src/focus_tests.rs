use super::*;

fn text(value: &str) -> OuiUtf8 {
    OuiUtf8 {
        data: value.as_ptr(),
        length: value.len(),
    }
}

struct Fixture {
    document: *mut OuiDocument,
    root: *mut OuiElement,
    nodes: [*mut OuiElement; 3],
}

impl Fixture {
    fn new() -> Self {
        let config = OuiDocumentConfig {
            struct_size: size_of::<OuiDocumentConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            viewport: OuiViewportMetrics {
                logical_width: 320.0,
                logical_height: 200.0,
                physical_width: 320,
                physical_height: 200,
                device_scale_factor: 1.0,
                authority: 1,
                reserved: 0,
            },
        };
        let mut fixture = Self {
            document: ptr::null_mut(),
            root: ptr::null_mut(),
            nodes: [ptr::null_mut(); 3],
        };
        assert_eq!(
            oui_document_create(&config, &mut fixture.document),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_document_root(fixture.document, &mut fixture.root),
            OuiStatus::Ok
        );
        for (node, id) in fixture.nodes.iter_mut().zip(["a", "b", "c"]) {
            assert_eq!(
                oui_element_create(fixture.document, 23, node),
                OuiStatus::Ok
            );
            assert_eq!(oui_element_append_child(fixture.root, *node), OuiStatus::Ok);
            assert_eq!(
                oui_element_set_attribute(*node, text("id"), text(id)),
                OuiStatus::Ok
            );
        }
        fixture
    }

    fn native(&self, id: &str) -> openui::Element {
        document(self.document as usize)
            .unwrap()
            .native
            .element_by_id(id)
            .unwrap()
            .unwrap()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for node in self.nodes {
            assert_eq!(oui_element_destroy(node), OuiStatus::Ok);
        }
        assert_eq!(oui_element_destroy(self.root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(self.document), OuiStatus::Ok);
    }
}

struct Callback {
    document: *mut OuiDocument,
    nodes: [*mut OuiElement; 3],
    events: RefCell<Vec<(usize, u32, Option<String>)>>,
    redirect: Cell<bool>,
    failed: Cell<bool>,
}

unsafe extern "C" fn observe(event: *mut OuiEvent, user_data: *mut c_void) {
    // SAFETY: the fixture owns the event and immutable context throughout
    // synchronous dispatch. Interior borrows end before any reentrant call.
    let event = unsafe { &*event };
    let data = unsafe { &*user_data.cast::<Callback>() };
    let Some(index) = data.nodes.iter().position(|node| *node == event.target) else {
        data.failed.set(true);
        return;
    };
    let focused = document(data.document as usize)
        .unwrap()
        .native
        .focused_element()
        .unwrap()
        .map(|node| node.get_attribute("id").unwrap().unwrap());
    data.events
        .borrow_mut()
        .push((index, event.event_type, focused));
    if event.target != event.current_target
        || oui_element_set_attribute(data.nodes[2], text("data-event"), text("seen"))
            != OuiStatus::Ok
    {
        data.failed.set(true);
    }
    if index == 0
        && event.event_type == 13
        && data.redirect.replace(false)
        && oui_element_focus(data.nodes[2]) != OuiStatus::Ok
    {
        data.failed.set(true);
    }
}

fn listen(fixture: &Fixture, context: &Callback) -> Vec<*mut OuiListener> {
    let mut listeners = Vec::new();
    for node in fixture.nodes {
        for event in [12, 13, 11] {
            let mut listener = ptr::null_mut();
            assert_eq!(
                oui_element_add_event_listener(
                    node,
                    event,
                    0,
                    Some(observe),
                    (context as *const Callback).cast_mut().cast(),
                    &mut listener
                ),
                OuiStatus::Ok
            );
            listeners.push(listener);
        }
    }
    listeners
}

#[test]
fn c_focus_and_blur_share_rust_callbacks_state_and_composition_cancellation() {
    let fixture = Fixture::new();
    let state = document(fixture.document as usize).unwrap();
    let context = Callback {
        document: fixture.document,
        nodes: fixture.nodes,
        events: RefCell::new(Vec::new()),
        redirect: Cell::new(false),
        failed: Cell::new(false),
    };
    let listeners = listen(&fixture, &context);
    let [a, b, c] = fixture.nodes;
    assert_eq!(oui_element_focus(a), OuiStatus::Ok);
    assert_eq!(oui_element_focus(a), OuiStatus::Ok);
    assert_eq!(oui_element_blur(b), OuiStatus::Ok);
    fixture.native("a").set_control_value("kept").unwrap();
    state.native.dispatch_composition_start().unwrap();
    state.native.dispatch_composition_update("preview").unwrap();
    assert_eq!(oui_element_focus(b), OuiStatus::Ok);
    assert_eq!(
        fixture.native("a").control_value().unwrap().as_deref(),
        Some("kept")
    );
    fixture.native("c").focus().unwrap();
    assert_eq!(
        oui_element_perform_accessibility_action(c, 2, text(""), 0, 0),
        OuiStatus::Ok
    );
    assert_eq!(
        oui_element_perform_accessibility_action(a, 1, text(""), 0, 0),
        OuiStatus::Ok
    );
    assert_eq!(
        *context.events.borrow(),
        [
            (0, 12, Some("a".into())),
            (0, 11, Some("a".into())),
            (0, 13, None),
            (1, 12, Some("b".into())),
            (1, 13, None),
            (2, 12, Some("c".into())),
            (2, 13, None),
            (0, 12, Some("a".into())),
        ]
    );
    assert!(!context.failed.get());
    context.events.borrow_mut().clear();
    context.redirect.set(true);
    assert_eq!(oui_element_focus(b), OuiStatus::Ok);
    assert_eq!(
        *context.events.borrow(),
        [(0, 13, None), (2, 12, Some("c".into()))]
    );
    assert!(fixture.native("c").has_focus().unwrap());
    for listener in listeners {
        assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
    }
}

#[test]
fn c_focus_and_blur_validate_ownership_borrows_and_contain_callback_panics() {
    let fixture = Fixture::new();
    let [a, b, _] = fixture.nodes;
    assert_eq!(
        oui_element_focus(ptr::null_mut()),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_element_blur(ptr::null_mut()),
        OuiStatus::InvalidArgument
    );
    let state = document(fixture.document as usize).unwrap();
    let borrow = state.engine.borrow_mut();
    assert_eq!(oui_element_focus(a), OuiStatus::Reentrant);
    assert_eq!(oui_element_blur(a), OuiStatus::Reentrant);
    drop(borrow);
    let address = a as usize;
    assert_eq!(
        std::thread::spawn(move || (
            oui_element_focus(address as *mut OuiElement),
            oui_element_blur(address as *mut OuiElement),
        ))
        .join()
        .unwrap(),
        (OuiStatus::WrongThread, OuiStatus::WrongThread)
    );
    assert_eq!(
        oui_element_set_attribute(b, text("disabled"), text("")),
        OuiStatus::Ok
    );
    assert_eq!(oui_element_focus(b), OuiStatus::InvalidState);
    let native = fixture.native("a");
    native
        .on("focus", |_| panic!("focus callback containment"))
        .unwrap();
    assert_eq!(oui_element_focus(a), OuiStatus::Internal);
    assert!(native.has_focus().unwrap());
    native.remove_event("focus").unwrap();
    native
        .on("blur", |_| panic!("blur callback containment"))
        .unwrap();
    assert_eq!(oui_element_blur(a), OuiStatus::Internal);
    assert!(state.native.focused_element().unwrap().is_none());
    native.remove_event("blur").unwrap();
    assert_eq!(oui_element_focus(a), OuiStatus::Ok);
    assert_eq!(oui_element_blur(a), OuiStatus::Ok);
    assert_eq!(oui_element_remove(a), OuiStatus::Ok);
    assert_eq!(oui_element_focus(a), OuiStatus::StaleHandle);
    assert_eq!(oui_element_blur(a), OuiStatus::StaleHandle);
}

#[test]
fn c_sequential_and_modal_focus_share_callbacks_composition_and_final_output() {
    let fixture = Fixture::new();
    let state = document(fixture.document as usize).unwrap();
    let context = Callback {
        document: fixture.document,
        nodes: fixture.nodes,
        events: RefCell::new(Vec::new()),
        redirect: Cell::new(false),
        failed: Cell::new(false),
    };
    let listeners = listen(&fixture, &context);
    let [a, b, c] = fixture.nodes;
    assert_eq!(
        oui_element_set_attribute(b, text("tabindex"), text("-1")),
        OuiStatus::Ok
    );
    assert_eq!(oui_element_focus(b), OuiStatus::Ok);
    let mut focused = ptr::null_mut();
    assert_eq!(
        oui_document_advance_focus(fixture.document, 1, &mut focused),
        OuiStatus::Ok
    );
    assert_eq!(focused, c);
    assert_eq!(oui_element_focus(b), OuiStatus::Ok);
    assert_eq!(
        oui_document_advance_focus(fixture.document, -1, &mut focused),
        OuiStatus::Ok
    );
    assert_eq!(focused, a);
    assert_eq!(
        oui_document_advance_focus(fixture.document, 0, &mut focused),
        OuiStatus::InvalidArgument
    );
    assert!(focused.is_null());
    assert!(fixture.native("a").has_focus().unwrap());
    assert_eq!(
        *context.events.borrow(),
        [
            (1, 12, Some("b".into())),
            (1, 13, None),
            (2, 12, Some("c".into())),
            (2, 13, None),
            (1, 12, Some("b".into())),
            (1, 13, None),
            (0, 12, Some("a".into())),
        ]
    );
    context.events.borrow_mut().clear();
    let mut modal = ptr::null_mut();
    assert_eq!(
        oui_element_create(fixture.document, 0, &mut modal),
        OuiStatus::Ok
    );
    assert_eq!(oui_element_append_child(fixture.root, modal), OuiStatus::Ok);
    fixture.native("c").detach().unwrap();
    assert_eq!(oui_element_append_child(modal, c), OuiStatus::Ok);
    state.native.dispatch_composition_start().unwrap();
    state.native.dispatch_composition_update("preview").unwrap();
    assert_eq!(
        oui_document_set_modal_root(fixture.document, modal),
        OuiStatus::Ok
    );
    assert!(fixture.native("c").has_focus().unwrap());
    assert_eq!(
        fixture.native("a").control_value().unwrap().as_deref(),
        Some("")
    );
    assert_eq!(
        oui_document_set_modal_root(fixture.document, ptr::null_mut()),
        OuiStatus::Ok
    );
    assert_eq!(
        oui_document_set_modal_root(fixture.document, ptr::null_mut()),
        OuiStatus::Ok
    );
    assert_eq!(
        *context.events.borrow(),
        [
            (0, 11, Some("a".into())),
            (0, 13, None),
            (2, 12, Some("c".into())),
            (2, 13, None),
            (0, 12, Some("a".into())),
        ]
    );
    assert!(!context.failed.get());
    for listener in listeners {
        assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
    }
    assert_eq!(oui_element_destroy(modal), OuiStatus::Ok);
}

struct PhaseCallback {
    root: *mut OuiElement,
    input: *mut OuiElement,
    rows: RefCell<Vec<(u32, bool, u32, u32)>>,
    failed: Cell<bool>,
}

unsafe extern "C" fn observe_phase(event: *mut OuiEvent, user_data: *mut c_void) {
    // SAFETY: the fixture retains both pointers through synchronous dispatch.
    let event = unsafe { &mut *event };
    let data = unsafe { &*user_data.cast::<PhaseCallback>() };
    data.rows.borrow_mut().push((
        event.event_type,
        event.current_target == data.root,
        event.phase,
        event.flags & OUI_EVENT_FLAG_DEFAULT_PREVENTED,
    ));
    if event.target != data.input
        || oui_element_set_attribute(data.input, text("data-phase"), text("seen")) != OuiStatus::Ok
    {
        data.failed.set(true);
    }
    event.flags |= OUI_EVENT_FLAG_DEFAULT_PREVENTED;
}

#[test]
fn c_focus_requests_capture_without_bubbling_and_ignore_cancellation() {
    let fixture = Fixture::new();
    let input = fixture.nodes[0];
    let context = PhaseCallback {
        root: fixture.root,
        input,
        rows: RefCell::new(Vec::new()),
        failed: Cell::new(false),
    };
    let mut listeners = Vec::new();
    for node in [fixture.root, input] {
        for event in [12, 13] {
            for capture in [0, 1] {
                let mut listener = ptr::null_mut();
                assert_eq!(
                    oui_element_add_event_listener(
                        node,
                        event,
                        capture,
                        Some(observe_phase),
                        (&context as *const PhaseCallback).cast_mut().cast(),
                        &mut listener
                    ),
                    OuiStatus::Ok
                );
                listeners.push(listener);
            }
        }
    }
    for kind in [12, 13] {
        let mut event = synthesized_event(kind, text(""));
        assert_eq!(
            oui_document_dispatch_event(fixture.document, input, &mut event),
            OuiStatus::Ok
        );
        assert_eq!(event.current_target, ptr::null_mut());
        assert_eq!(event.flags & OUI_EVENT_FLAG_DEFAULT_PREVENTED, 0);
        assert_eq!(fixture.native("a").has_focus().unwrap(), kind == 12);
    }
    assert_eq!(
        *context.rows.borrow(),
        [
            (12, true, 1, 0),
            (12, false, 2, 0),
            (12, false, 2, 0),
            (13, true, 1, 0),
            (13, false, 2, 0),
            (13, false, 2, 0),
        ]
    );
    assert!(!context.failed.get());
    for listener in listeners {
        assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
    }
}

#[test]
fn c_pointer_focus_cancels_the_shared_composition_and_delivers_focus_callbacks() {
    let fixture = Fixture::new();
    for id in ["a", "b", "c"] {
        let input = fixture.native(id);
        input.set_display(openui_style::Display::Block).unwrap();
        input.set_width(LengthValue::px(100.0)).unwrap();
        input.set_height(LengthValue::px(24.0)).unwrap();
    }
    let state = document(fixture.document as usize).unwrap();
    let context = Callback {
        document: fixture.document,
        nodes: fixture.nodes,
        events: RefCell::new(Vec::new()),
        redirect: Cell::new(false),
        failed: Cell::new(false),
    };
    let listeners = listen(&fixture, &context);
    assert_eq!(oui_element_focus(fixture.nodes[0]), OuiStatus::Ok);
    state.native.dispatch_composition_start().unwrap();
    state.native.dispatch_composition_update("preview").unwrap();
    let bounds = fixture.native("b").bounding_rect().unwrap().unwrap();
    assert!(bounds.width > 0.0 && bounds.height > 0.0);
    let mut event = synthesized_event(1, text(""));
    event.x = bounds.x + bounds.width * 0.5;
    event.y = bounds.y + bounds.height * 0.5;
    event.pointer_id = 7;
    assert_eq!(
        oui_document_dispatch_pointer_event(fixture.document, &mut event),
        OuiStatus::Ok
    );
    assert_eq!(event.target, fixture.nodes[1]);
    assert!(fixture.native("b").has_focus().unwrap());
    assert_eq!(
        fixture.native("a").control_value().unwrap().as_deref(),
        Some("")
    );
    assert_eq!(
        *context.events.borrow(),
        [
            (0, 12, Some("a".into())),
            (0, 11, Some("a".into())),
            (0, 13, None),
            (1, 12, Some("b".into())),
        ]
    );
    assert!(!context.failed.get());
    for listener in listeners {
        assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
    }
}
