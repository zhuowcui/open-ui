//! Source guards for the shared C normalized wheel route; execution is pending.
use super::*;
use std::cell::Cell;
use std::mem::MaybeUninit;

fn text(value: &str) -> OuiUtf8 {
    OuiUtf8 {
        data: value.as_ptr(),
        length: value.len(),
    }
}
fn property(node: *mut OuiElement, property: StyleProperty, literal: &str) {
    let mut value = MaybeUninit::<OuiStyleValue>::uninit();
    assert_eq!(
        oui_style_value_parse(property as i32, text(literal), value.as_mut_ptr()),
        OuiStatus::Ok
    );
    let value = unsafe { value.assume_init() };
    assert_eq!(
        oui_element_set_property(node, property as i32, &value),
        OuiStatus::Ok
    );
    if value.tag == 6 {
        assert_eq!(
            oui_style_compound_destroy(unsafe { value.data.compound }.cast_mut()),
            OuiStatus::Ok
        );
    }
}
fn offset(node: *mut OuiElement) -> (f64, f64) {
    let (mut x, mut y) = (-1.0, -1.0);
    assert_eq!(
        oui_element_get_scroll_offset(node, &mut x, &mut y),
        OuiStatus::Ok
    );
    (x, y)
}
struct Fixture {
    document: *mut OuiDocument,
    root: *mut OuiElement,
    source: *mut OuiElement,
    child: *mut OuiElement,
    focus: *mut OuiElement,
    nodes: Vec<*mut OuiElement>,
    listeners: Vec<*mut OuiListener>,
}
impl Fixture {
    fn new(scale: f64, tag: i32) -> Self {
        let config = OuiDocumentConfig {
            struct_size: size_of::<OuiDocumentConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            viewport: OuiViewportMetrics {
                logical_width: 320.0,
                logical_height: 240.0,
                physical_width: (320.0 * scale) as u32,
                physical_height: (240.0 * scale) as u32,
                device_scale_factor: scale,
                authority: 1,
                reserved: 0,
            },
        };
        let (mut document, mut root) = (ptr::null_mut(), ptr::null_mut());
        assert_eq!(oui_document_create(&config, &mut document), OuiStatus::Ok);
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let mut f = Self {
            document,
            root,
            source: ptr::null_mut(),
            child: ptr::null_mut(),
            focus: ptr::null_mut(),
            nodes: vec![root],
            listeners: Vec::new(),
        };
        f.source = f.child_box(root, tag, "100px", "80px");
        property(f.source, StyleProperty::Padding, "0px");
        property(f.source, StyleProperty::Border, "0px none transparent");
        property(f.source, StyleProperty::Overflow, "auto");
        property(f.source, StyleProperty::ScrollbarWidth, "none");
        f.child = f.child_box(f.source, 0, "100px", "240px");
        property(f.child, StyleProperty::BackgroundColor, "green");
        f.focus = f.child_box(root, 0, "20px", "20px");
        property(f.focus, StyleProperty::Position, "absolute");
        property(f.focus, StyleProperty::Left, "200px");
        property(f.focus, StyleProperty::Top, "0px");
        assert_eq!(
            oui_element_set_attribute(f.focus, text("tabindex"), text("0")),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_focus(f.focus), OuiStatus::Ok);
        assert_eq!(oui_document_update(document), OuiStatus::Ok);
        f
    }
    fn child_box(
        &mut self,
        parent: *mut OuiElement,
        tag: i32,
        width: &str,
        height: &str,
    ) -> *mut OuiElement {
        let mut node = ptr::null_mut();
        assert_eq!(
            oui_element_create(self.document, tag, &mut node),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_append_child(parent, node), OuiStatus::Ok);
        self.nodes.push(node);
        property(node, StyleProperty::Display, "block");
        property(node, StyleProperty::Width, width);
        property(node, StyleProperty::Height, height);
        node
    }
    fn wheel(&self, delta: f32) -> OuiEvent {
        let mut rect = OuiRect {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
        };
        assert_eq!(
            oui_element_get_bounds(self.source, &mut rect),
            OuiStatus::Ok
        );
        assert!(rect.width > 4.0 && rect.height > 4.0);
        let mut event = synthesized_event(5, text(""));
        event.x = rect.x + 2.0;
        event.y = rect.y + 2.0;
        event.delta_y = delta;
        event
    }
    fn listen(&mut self, node: *mut OuiElement, event: u32, capture: bool, context: &Observation) {
        let mut listener = ptr::null_mut();
        assert_eq!(
            oui_element_add_event_listener(
                node,
                event,
                capture as u8,
                Some(observe),
                (context as *const Observation).cast_mut().cast(),
                &mut listener
            ),
            OuiStatus::Ok
        );
        self.listeners.push(listener);
    }
    fn assert_focus(&self) {
        let mut focused = ptr::null_mut();
        assert_eq!(
            oui_document_focused_element_v1(self.document, &mut focused),
            OuiStatus::Ok
        );
        let mut same = 0;
        assert_eq!(
            oui_element_is_same_node_v1(focused, self.focus, &mut same),
            OuiStatus::Ok
        );
        assert_eq!(same, 1);
        assert_eq!(oui_element_destroy(focused), OuiStatus::Ok);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        for listener in self.listeners.drain(..) {
            let _ = oui_listener_destroy(listener);
        }
        for node in self.nodes.drain(..).rev() {
            let _ = oui_element_destroy(node);
        }
        let _ = oui_document_destroy(self.document);
    }
}
struct Observation {
    source: *mut OuiElement,
    cancel: Cell<bool>,
    capture: Cell<u32>,
    bubble: Cell<u32>,
    scroll: Cell<u32>,
    failed: Cell<bool>,
    before_y: Cell<f64>,
}
unsafe extern "C" fn observe(event: *mut OuiEvent, data: *mut c_void) {
    // SAFETY: synchronous callback receives live event/context storage.
    let event = unsafe { &mut *event };
    let data = unsafe { &*data.cast::<Observation>() };
    if event.event_type == 19 {
        data.scroll.set(data.scroll.get() + 1);
        return;
    }
    if event.event_type != 5 {
        data.failed.set(true);
        return;
    }
    match event.phase {
        1 => data.capture.set(data.capture.get() + 1),
        3 => data.bubble.set(data.bubble.get() + 1),
        _ => data.failed.set(true),
    }
    let (mut x, mut y) = (-1.0, -1.0);
    if oui_element_get_scroll_offset(data.source, &mut x, &mut y) != OuiStatus::Ok
        || oui_element_set_attribute(data.source, text("data-wheel"), text("observed"))
            != OuiStatus::Ok
    {
        data.failed.set(true);
    }
    data.before_y.set(y);
    if data.cancel.get() {
        event.flags |= OUI_EVENT_FLAG_DEFAULT_PREVENTED;
    }
}
fn observation(source: *mut OuiElement) -> Observation {
    Observation {
        source,
        cancel: Cell::new(true),
        capture: Cell::new(0),
        bubble: Cell::new(0),
        scroll: Cell::new(0),
        failed: Cell::new(false),
        before_y: Cell::new(-1.0),
    }
}

#[test]
fn normalized_wheel_callbacks_cancel_before_default_and_preserve_focus_and_const_input() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut f = Fixture::new(scale, 0);
        let data = observation(f.source);
        f.listen(f.root, 5, true, &data);
        f.listen(f.root, 5, false, &data);
        f.listen(f.source, 19, false, &data);
        let mut event = f.wheel(20.0);
        event.timestamp_ns = 123;
        event.pointer_id = 99;
        event.key_code = 88;
        event.phase = 17;
        event.text = text("unused wheel text");
        event.target = f.focus;
        event.current_target = f.focus;
        assert_eq!(
            oui_document_dispatch_wheel_input_v1(f.document, &event),
            OuiStatus::Ok
        );
        assert_eq!(offset(f.source), (0.0, 0.0));
        assert_eq!(
            (data.capture.get(), data.bubble.get(), data.before_y.get()),
            (1, 1, 0.0)
        );
        assert_eq!(
            (
                event.flags,
                event.phase,
                event.timestamp_ns,
                event.pointer_id,
                event.key_code
            ),
            (0, 17, 123, 99, 88)
        );
        assert_eq!((event.target, event.current_target), (f.focus, f.focus));
        data.cancel.set(false);
        assert_eq!(
            oui_document_dispatch_wheel_input_v1(f.document, &event),
            OuiStatus::Ok
        );
        assert_eq!(offset(f.source), (0.0, 20.0));
        assert_eq!(data.before_y.get(), 0.0);
        f.assert_focus();
        let mut pending = u32::MAX;
        assert_eq!(
            oui_document_dispatch_pending_events_v1(f.document, &mut pending),
            OuiStatus::Ok
        );
        assert_eq!(data.scroll.get(), 1);
        assert!(!data.failed.get());
        // Generic explicit Wheel is intentionally only a notification.
        let mut raw = f.wheel(20.0);
        assert_eq!(
            oui_document_dispatch_event(f.document, f.child, &mut raw),
            OuiStatus::Ok
        );
        assert_eq!(offset(f.source), (0.0, 20.0));
        assert_eq!((data.capture.get(), data.bubble.get()), (3, 3));
    }
}

#[test]
fn normalized_wheel_reuses_nested_chain_and_fieldset_content_ownership() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        for tag in [0, 27] {
            let f = Fixture::new(scale, tag);
            assert_eq!(
                oui_document_dispatch_wheel_input_v1(f.document, &f.wheel(20.0)),
                OuiStatus::Ok
            );
            assert_eq!(offset(f.source), (0.0, 20.0));
            assert_eq!(oui_element_scroll_to(f.source, 0.0, 0.0), OuiStatus::Ok);
            property(f.source, StyleProperty::Overflow, "hidden");
            assert_eq!(
                oui_document_dispatch_wheel_input_v1(f.document, &f.wheel(20.0)),
                OuiStatus::Ok
            );
            assert_eq!(offset(f.source), (0.0, 0.0));
            property(f.source, StyleProperty::Overflow, "clip");
            assert_eq!(
                oui_document_dispatch_wheel_input_v1(f.document, &f.wheel(20.0)),
                OuiStatus::Ok
            );
            assert_eq!(offset(f.source), (0.0, 0.0));
        }
        let mut f = Fixture::new(scale, 0);
        property(f.source, StyleProperty::Height, "60px");
        property(f.child, StyleProperty::Height, "180px");
        let outer = f.child_box(f.root, 0, "120px", "100px");
        property(outer, StyleProperty::Overflow, "auto");
        property(outer, StyleProperty::ScrollbarWidth, "none");
        assert_eq!(oui_element_append_child(outer, f.source), OuiStatus::Ok);
        let tail = f.child_box(outer, 0, "100px", "180px");
        property(tail, StyleProperty::BackgroundColor, "red");
        assert_eq!(oui_element_scroll_to(f.source, 0.0, 120.0), OuiStatus::Ok);
        assert_eq!(offset(f.source), (0.0, 120.0));
        assert_eq!(
            oui_document_dispatch_wheel_input_v1(f.document, &f.wheel(20.0)),
            OuiStatus::Ok
        );
        assert_eq!(offset(f.source), (0.0, 120.0));
        assert_eq!(offset(outer), (0.0, 20.0));
        f.assert_focus();
    }
}

#[test]
fn normalized_wheel_preserves_both_signed_axes_zero_and_no_hit() {
    let f = Fixture::new(1.0, 0);
    property(f.child, StyleProperty::Width, "240px");
    let mut event = f.wheel(25.0);
    event.delta_x = 30.0;
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &event),
        OuiStatus::Ok
    );
    assert_eq!(offset(f.source), (30.0, 25.0));
    event.delta_x = -10.0;
    event.delta_y = -5.0;
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &event),
        OuiStatus::Ok
    );
    assert_eq!(offset(f.source), (20.0, 20.0));
    event.delta_x = 0.0;
    event.delta_y = 0.0;
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &event),
        OuiStatus::Ok
    );
    assert_eq!(offset(f.source), (20.0, 20.0));
    event.x = 4000.0;
    event.y = 4000.0;
    event.delta_x = 10.0;
    event.delta_y = 10.0;
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &event),
        OuiStatus::Ok
    );
    assert_eq!(offset(f.source), (20.0, 20.0));
    f.assert_focus();
}

#[test]
fn normalized_wheel_invalid_inputs_leave_offsets_and_descriptor_untouched() {
    let f = Fixture::new(1.0, 0);
    let valid = f.wheel(20.0);
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, ptr::null()),
        OuiStatus::InvalidArgument
    );
    for case in 0..8 {
        let mut bad = valid;
        match case {
            0 => bad.struct_size = 4,
            1 => bad.abi_version = 0,
            2 => bad.event_type = 1,
            3 => bad.flags = OUI_EVENT_FLAG_DEFAULT_PREVENTED,
            4 => bad.modifiers = 0x20,
            5 => bad.x = f32::NAN,
            6 => bad.delta_y = f32::INFINITY,
            _ => bad.delta_x = f32::NEG_INFINITY,
        }
        assert_eq!(
            oui_document_dispatch_wheel_input_v1(f.document, &bad),
            if case == 1 {
                OuiStatus::AbiMismatch
            } else {
                OuiStatus::InvalidArgument
            }
        );
        assert_eq!(offset(f.source), (0.0, 0.0));
    }
    let bytes = [0xff_u8];
    let mut bad = valid;
    bad.text = OuiUtf8 {
        data: bytes.as_ptr(),
        length: 1,
    };
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &bad),
        OuiStatus::InvalidArgument
    );
    let storage = [0_u64; 32];
    let misaligned = unsafe { storage.as_ptr().cast::<u8>().add(1).cast::<OuiEvent>() };
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, misaligned),
        OuiStatus::InvalidArgument
    );
    let overflow = (usize::MAX - 7) as *const OuiEvent;
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, overflow),
        OuiStatus::InvalidArgument
    );
    #[repr(C)]
    struct Extended {
        event: OuiEvent,
        tail: [u8; 8],
    }
    let mut extended = Extended {
        event: valid,
        tail: [0xa5; 8],
    };
    extended.event.struct_size = size_of::<Extended>() as u32;
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &extended.event),
        OuiStatus::Ok
    );
    assert_eq!(extended.tail, [0xa5; 8]);
    assert_eq!(offset(f.source), (0.0, 20.0));
}

#[test]
fn normalized_wheel_ownership_reentry_and_callback_unwind_use_existing_boundaries() {
    let f = Fixture::new(1.0, 0);
    let event = f.wheel(20.0);
    let state = document(f.document as usize).unwrap();
    let borrow = borrow_engine(&state).unwrap();
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &event),
        OuiStatus::Reentrant
    );
    drop(borrow);
    assert_eq!(offset(f.source), (0.0, 0.0));
    let address = f.document as usize;
    let status = std::thread::spawn(move || {
        oui_document_dispatch_wheel_input_v1(
            address as *mut OuiDocument,
            &synthesized_event(5, text("")),
        )
    })
    .join()
    .unwrap();
    assert_eq!(status, OuiStatus::WrongThread);
    state
        .native
        .body()
        .on("wheel", |_| panic!("native wheel callback containment"))
        .unwrap();
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &event),
        OuiStatus::Internal
    );
    assert_eq!(offset(f.source), (0.0, 0.0));
    state.native.body().remove_event("wheel").unwrap();
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(f.document, &event),
        OuiStatus::Ok
    );
    assert_eq!(offset(f.source), (0.0, 20.0));
    drop(f);
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(address as *mut OuiDocument, &event),
        OuiStatus::InvalidHandle
    );
    assert_eq!(
        oui_document_dispatch_wheel_input_v1(ptr::null_mut(), &event),
        OuiStatus::InvalidArgument
    );
}
