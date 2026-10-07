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
