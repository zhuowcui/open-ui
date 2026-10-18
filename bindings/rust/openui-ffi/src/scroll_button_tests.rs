use super::*;

fn state_output() -> OuiScrollButtonStateV1 {
    OuiScrollButtonStateV1 {
        struct_size: size_of::<OuiScrollButtonStateV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        direction: u32::MAX,
        availability: u32::MAX,
        enabled: 0,
        has_target: 0,
        reserved: [0; 6],
        current_x: -1.0,
        current_y: -1.0,
        target_x: -1.0,
        target_y: -1.0,
    }
}

fn fixture() -> (*mut OuiDocument, *mut OuiElement) {
    let config = OuiDocumentConfig {
        struct_size: size_of::<OuiDocumentConfig>() as u32,
        abi_version: OUI_ABI_VERSION,
        viewport: OuiViewportMetrics {
            logical_width: 320.0,
            logical_height: 240.0,
            physical_width: 320,
            physical_height: 240,
            device_scale_factor: 1.0,
            authority: 1,
            reserved: 0,
        },
    };
    let mut doc = ptr::null_mut();
    let mut origin = ptr::null_mut();
    assert_eq!(oui_document_create(&config, &mut doc), OuiStatus::Ok);
    assert_eq!(oui_element_create(doc, 0, &mut origin), OuiStatus::Ok);
    let state = document(doc as usize).unwrap();
    let scroller = element(origin as usize).unwrap().node;
    {
        let mut engine = borrow_engine_mut(&state).unwrap();
        let content = engine.create_native_element(ElementTag::Div).unwrap();
        let root = engine.root();
        engine.append_child(root, scroller).unwrap();
        engine.append_child(scroller, content).unwrap();
        for (node, property, literal) in [
            (scroller, StyleProperty::Width, "80px"),
            (scroller, StyleProperty::Height, "80px"),
            (scroller, StyleProperty::Overflow, "hidden"),
            (scroller, StyleProperty::ScrollbarWidth, "none"),
            (content, StyleProperty::Width, "80px"),
            (content, StyleProperty::Height, "500px"),
        ] {
            engine
                .set_property(node, property, parse_literal(property, literal).unwrap())
                .unwrap();
        }
    }
    (doc, origin)
}

#[test]
fn scroll_button_aliases_preserve_direction_identity_and_origin_lifetime() {
    let (doc, origin) = fixture();
    let mut buttons = Vec::new();
    for direction in 0..8 {
        let mut button = ptr::null_mut();
        assert_eq!(
            oui_element_create_scroll_button_v1(origin, direction, &mut button),
            OuiStatus::Ok
        );
        let mut value = state_output();
        assert_eq!(
            oui_scroll_button_get_state_v1(button, &mut value),
            OuiStatus::Ok
        );
        assert_eq!(value.direction, direction);
        let mut alias = ptr::null_mut();
        assert_eq!(
            oui_element_get_scroll_button_v1(origin, direction, &mut alias),
            OuiStatus::Ok
        );
        assert_ne!(alias, button);
        let mut same = 0;
        assert_eq!(
            oui_element_is_same_node_v1(button, alias, &mut same),
            OuiStatus::Ok
        );
        assert_eq!(same, 1);
        assert_eq!(oui_element_destroy(alias), OuiStatus::Ok);
        let mut related = ptr::null_mut();
        assert_eq!(
            oui_scroll_button_get_origin_v1(button, &mut related),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_is_same_node_v1(origin, related, &mut same),
            OuiStatus::Ok
        );
        assert_eq!(same, 1);
        assert_eq!(oui_element_destroy(related), OuiStatus::Ok);
        let mut collision = 1usize as *mut OuiElement;
        assert_eq!(
            oui_element_create_scroll_button_v1(origin, direction, &mut collision),
            OuiStatus::InvalidState
        );
        assert_eq!(collision, 1usize as *mut OuiElement);
        buttons.push(button);
    }
    assert_eq!(
        oui_scroll_button_set_direction_v1(buttons[0], 2),
        OuiStatus::InvalidState
    );
    let mut unchanged = state_output();
    assert_eq!(
        oui_scroll_button_get_state_v1(buttons[0], &mut unchanged),
        OuiStatus::Ok
    );
    assert_eq!(unchanged.direction, 0);
    assert_eq!(oui_element_remove(origin), OuiStatus::Ok);
    for button in buttons {
        let mut value = state_output();
        let before = value;
        assert_eq!(
            oui_scroll_button_get_state_v1(button, &mut value),
            OuiStatus::StaleHandle
        );
        assert_eq!(value, before);
        assert_eq!(oui_element_destroy(button), OuiStatus::Ok);
    }
    assert_eq!(oui_element_destroy(origin), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(doc), OuiStatus::Ok);
}

#[test]
fn scroll_button_c_boundaries_validate_headers_threads_and_borrows_before_ownership() {
    use std::mem::align_of;
    assert_eq!(
        (
            size_of::<OuiScrollButtonStateV1>(),
            align_of::<OuiScrollButtonStateV1>()
        ),
        (56, 8)
    );
    let (doc, origin) = fixture();
    let state = document(doc as usize).unwrap();
    let mut button = 1usize as *mut OuiElement;
    let engine = borrow_engine_mut(&state).unwrap();
    assert_eq!(
        oui_element_create_scroll_button_v1(origin, 2, &mut button),
        OuiStatus::Reentrant
    );
    assert_eq!(button, 1usize as *mut OuiElement);
    drop(engine);
    let handles = state.element_handles.borrow_mut();
    assert_eq!(
        oui_element_create_scroll_button_v1(origin, 2, &mut button),
        OuiStatus::Reentrant
    );
    assert_eq!(button, 1usize as *mut OuiElement);
    drop(handles);
    let mut absent = 1usize as *mut OuiElement;
    assert_eq!(
        oui_element_get_scroll_button_v1(origin, 2, &mut absent),
        OuiStatus::Ok
    );
    assert!(absent.is_null());
    assert_eq!(
        oui_element_create_scroll_button_v1(origin, 99, &mut button),
        OuiStatus::InvalidArgument
    );
    assert_eq!(button, 1usize as *mut OuiElement);
    assert_eq!(
        oui_element_create_scroll_button_v1(origin, 2, ptr::null_mut()),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_element_create_scroll_button_v1(origin, 2, &mut button),
        OuiStatus::Ok
    );
    let short = 4u32;
    assert_eq!(
        oui_scroll_button_get_state_v1(button, (&short as *const u32).cast_mut().cast()),
        OuiStatus::InvalidArgument
    );
    let prefix = [8u32, OUI_ABI_VERSION];
    assert_eq!(
        oui_scroll_button_get_state_v1(button, prefix.as_ptr().cast_mut().cast()),
        OuiStatus::InvalidArgument
    );
    let mut bad = state_output();
    bad.abi_version += 1;
    let unchanged = bad;
    assert_eq!(
        oui_scroll_button_get_state_v1(button, &mut bad),
        OuiStatus::AbiMismatch
    );
    assert_eq!(bad, unchanged);
    #[repr(C)]
    struct Extended {
        value: OuiScrollButtonStateV1,
        trailing: [u8; 16],
    }
    let mut extended = Extended {
        value: state_output(),
        trailing: [0x6d; 16],
    };
    extended.value.struct_size = size_of::<Extended>() as u32;
    assert_eq!(
        oui_scroll_button_get_state_v1(button, &mut extended.value),
        OuiStatus::Ok
    );
    assert_eq!(extended.trailing, [0x6d; 16]);
    assert_eq!(extended.value.struct_size, size_of::<Extended>() as u32);
    let address = button as usize;
    assert_eq!(
        std::thread::spawn(move || {
            let mut value = state_output();
            oui_scroll_button_get_state_v1(address as *mut OuiElement, &mut value)
        })
        .join()
        .unwrap(),
        OuiStatus::WrongThread
    );
    let engine = borrow_engine_mut(&state).unwrap();
    let mut out = state_output();
    let before = out;
    assert_eq!(
        oui_scroll_button_get_state_v1(button, &mut out),
        OuiStatus::Reentrant
    );
    assert_eq!(out, before);
    drop(engine);
    let handles = state.element_handles.borrow_mut();
    assert_eq!(
        oui_scroll_button_get_origin_v1(button, &mut absent),
        OuiStatus::Reentrant
    );
    assert!(absent.is_null());
    drop(handles);
    assert_eq!(
        oui_scroll_button_get_snap_target_v1(button, 99, 0, &mut absent),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_scroll_button_get_snap_target_v1(button, 0, 99, &mut absent),
        OuiStatus::InvalidArgument
    );
    assert!(absent.is_null());
    assert_eq!(
        oui_element_set_native_appearance_v1(button, 2),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_element_set_scroll_snap_strictness_v1(origin, 99),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_scroll_button_activate_v1(origin),
        OuiStatus::InvalidState
    );
    assert_eq!(oui_element_destroy(button), OuiStatus::Ok);
    assert_eq!(oui_element_destroy(origin), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(doc), OuiStatus::Ok);
}

struct Callback {
    origin: *mut OuiElement,
    button: *mut OuiElement,
    cancel: Cell<bool>,
    clicks: Cell<u32>,
    changes: Cell<u32>,
    failures: RefCell<Vec<String>>,
}

unsafe extern "C" fn clicked(event: *mut OuiEvent, context: *mut c_void) {
    // SAFETY: this owning-thread synchronous callback retains both values
    // until return. Record failures without unwinding across the C boundary.
    let context = unsafe { &*context.cast::<Callback>() };
    let event = unsafe { &mut *event };
    if event.target != context.origin || event.current_target != context.origin {
        context
            .failures
            .borrow_mut()
            .push("projected event identity".into());
    }
    if event.event_type == 4 {
        context.clicks.set(context.clicks.get() + 1);
        let mut snapshot = state_output();
        if oui_scroll_button_get_state_v1(context.button, &mut snapshot) != OuiStatus::Ok {
            context
                .failures
                .borrow_mut()
                .push("callback retained Engine access".into());
        }
        if context.cancel.get() {
            event.flags |= OUI_EVENT_FLAG_DEFAULT_PREVENTED;
        }
    } else {
        context.changes.set(context.changes.get() + 1);
    }
}

#[test]
fn scroll_button_c_activation_shares_cancelable_origin_events_and_focus_projection() {
    let (doc, origin) = fixture();
    let mut button = ptr::null_mut();
    assert_eq!(
        oui_element_create_scroll_button_v1(origin, 2, &mut button),
        OuiStatus::Ok
    );
    let mut context = Callback {
        origin,
        button,
        cancel: Cell::new(true),
        clicks: Cell::new(0),
        changes: Cell::new(0),
        failures: RefCell::new(Vec::new()),
    };
    let mut listeners = Vec::new();
    for kind in [4, 16, 17] {
        let mut listener = ptr::null_mut();
        assert_eq!(
            oui_element_add_event_listener(
                origin,
                kind,
                0,
                Some(clicked),
                (&mut context as *mut Callback).cast(),
                &mut listener
            ),
            OuiStatus::Ok
        );
        listeners.push(listener);
    }
    assert_eq!(oui_scroll_button_activate_v1(button), OuiStatus::Ok);
    let mut snapshot = state_output();
    assert_eq!(
        oui_scroll_button_get_state_v1(button, &mut snapshot),
        OuiStatus::Ok
    );
    assert_eq!(snapshot.current_y, 0.0);
    context.cancel.set(false);
    assert_eq!(oui_scroll_button_activate_v1(button), OuiStatus::Ok);
    assert_eq!(
        oui_scroll_button_get_state_v1(button, &mut snapshot),
        OuiStatus::Ok
    );
    assert_eq!(snapshot.current_y, 70.0);
    assert_eq!(context.clicks.get(), 2);
    assert_eq!(context.changes.get(), 0);
    assert!(
        context.failures.borrow().is_empty(),
        "{:?}",
        context.failures.borrow()
    );
    let mut focused = ptr::null_mut();
    assert_eq!(
        oui_document_focused_element_v1(doc, &mut focused),
        OuiStatus::Ok
    );
    assert!(!focused.is_null());
    let mut same = 0;
    assert_eq!(
        oui_element_is_same_node_v1(origin, focused, &mut same),
        OuiStatus::Ok
    );
    assert_eq!(same, 1);
    assert_eq!(oui_element_destroy(focused), OuiStatus::Ok);
    let copied = snapshot;
    for listener in listeners {
        assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
    }
    assert_eq!(oui_element_destroy(button), OuiStatus::Ok);
    assert_eq!(oui_element_destroy(origin), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(doc), OuiStatus::Ok);
    assert_eq!(copied.current_y, 70.0);
}
