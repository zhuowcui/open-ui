use super::*;

struct Context {
    document: *mut OuiDocument,
    root: *mut OuiElement,
    nodes: Vec<*mut OuiElement>,
}
impl Context {
    fn new(scale: f64) -> Self {
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
        let mut document = ptr::null_mut();
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_create(&config, &mut document), OuiStatus::Ok);
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        Self {
            document,
            root,
            nodes: Vec::new(),
        }
    }
    fn node(&mut self, parent: *mut OuiElement) -> *mut OuiElement {
        let mut node = ptr::null_mut();
        assert_eq!(
            oui_element_create(self.document, 0, &mut node),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_append_child(parent, node), OuiStatus::Ok);
        self.nodes.push(node);
        node
    }
    fn set(&self, node: *mut OuiElement, property: StyleProperty, text: &str) {
        let mut value = OuiStyleValue {
            tag: 0,
            reserved: 0,
            data: OuiStylePayload { number: 0.0 },
        };
        assert_eq!(
            oui_style_value_parse(
                property as i32,
                OuiUtf8 {
                    data: text.as_ptr(),
                    length: text.len()
                },
                &mut value
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_set_property(node, property as i32, &value),
            OuiStatus::Ok
        );
        if value.tag == 6 {
            // SAFETY: this successful parse returned an owned compound alias.
            assert_eq!(
                oui_style_compound_destroy(unsafe { value.data.compound } as *mut OuiStyleCompound),
                OuiStatus::Ok
            );
        }
    }
    fn scroller(&mut self) -> *mut OuiElement {
        let source = self.node(self.root);
        let content = self.node(source);
        for (property, text) in [
            (StyleProperty::Display, "block"),
            (StyleProperty::Width, "100px"),
            (StyleProperty::Height, "80px"),
            (StyleProperty::Overflow, "hidden"),
            (StyleProperty::ScrollbarWidth, "none"),
        ] {
            self.set(source, property, text);
        }
        self.set(content, StyleProperty::Display, "block");
        self.set(content, StyleProperty::Width, "300px");
        self.set(content, StyleProperty::Height, "280px");
        source
    }
    fn offset(&self, node: *mut OuiElement) -> (f64, f64) {
        let (mut x, mut y) = (f64::NAN, f64::NAN);
        assert_eq!(
            oui_element_get_scroll_offset(node, &mut x, &mut y),
            OuiStatus::Ok
        );
        (x, y)
    }
    fn time(&self, time: f64) {
        assert_eq!(
            oui_document_set_animation_time(self.document, time),
            OuiStatus::Ok
        );
    }
}
impl Drop for Context {
    fn drop(&mut self) {
        for node in self.nodes.drain(..) {
            assert_eq!(oui_element_destroy(node), OuiStatus::Ok);
        }
        assert_eq!(oui_element_destroy(self.root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(self.document), OuiStatus::Ok);
    }
}

fn steps() -> OuiEasingCurveV1 {
    OuiEasingCurveV1 {
        struct_size: size_of::<OuiEasingCurveV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        kind: 2,
        step_position: 1,
        step_count: 2,
        reserved: 0,
        values: [0.0; 4],
        stops: ptr::null(),
        stop_count: 0,
        stop_stride: 0,
    }
}

#[test]
fn explicit_snap_curve_and_cancel_use_the_shared_scroll_clock_at_five_scales() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let mut context = Context::new(scale);
        let source = context.scroller();
        let other = context.scroller();
        assert_eq!(oui_element_scroll_to(source, 40.0, 40.0), OuiStatus::Ok);
        let mut id = u64::MAX;
        let mut found = 255;
        assert_eq!(
            oui_element_settle_scroll_snap_with_easing_v1(
                source,
                [0.0, 100.0].as_ptr(),
                2,
                ptr::null(),
                0,
                100.0,
                &steps(),
                &mut id,
                &mut found
            ),
            OuiStatus::Ok
        );
        assert_eq!(found, 1);
        let mut other_id = 0;
        assert_eq!(
            oui_element_smooth_scroll_to_v1(other, 100.0, 0.0, 100.0, ptr::null(), &mut other_id),
            OuiStatus::Ok
        );
        assert_ne!(id, other_id);
        context.time(25.0);
        assert_eq!(context.offset(source), (40.0, 40.0));
        context.time(50.0);
        assert_eq!(context.offset(source), (20.0, 40.0));
        assert_eq!(
            oui_document_cancel_smooth_scroll_v1(context.document, id),
            OuiStatus::Ok
        );
        assert_eq!(context.offset(source), (20.0, 40.0));
        assert_eq!(
            oui_document_cancel_smooth_scroll_v1(context.document, id),
            OuiStatus::InvalidState
        );
        context.time(100.0);
        assert_eq!(context.offset(source), (20.0, 40.0));
        assert_eq!(context.offset(other), (100.0, 0.0));
    }
}

#[test]
fn copied_points_may_alias_success_output_and_outlive_caller_storage() {
    let mut context = Context::new(1.0);
    let source = context.scroller();
    let mut point = [100.0_f64];
    let id_output = point.as_mut_ptr().cast::<u64>();
    let mut found = 255;
    assert_eq!(
        oui_element_settle_scroll_snap_with_easing_v1(
            source,
            point.as_ptr(),
            1,
            ptr::null(),
            0,
            100.0,
            ptr::null(),
            id_output,
            &mut found
        ),
        OuiStatus::Ok
    );
    assert_eq!(found, 1);
    // SAFETY: success initialized this aligned output after copying the point.
    let id = unsafe { ptr::read(id_output) };
    point[0] = 0.0;
    context.time(50.0);
    assert_eq!(context.offset(source), (50.0, 0.0));
    assert_eq!(
        oui_document_cancel_smooth_scroll_v1(context.document, id),
        OuiStatus::Ok
    );
    assert_eq!(point, [0.0]);
}

#[test]
fn no_op_and_invalid_inputs_preserve_outputs_and_the_existing_track() {
    let mut context = Context::new(1.0);
    let source = context.scroller();
    let (mut id, mut found) = (u64::MAX, 255);
    assert_eq!(
        oui_element_settle_scroll_snap_with_easing_v1(
            source,
            ptr::null(),
            0,
            ptr::null(),
            0,
            100.0,
            ptr::null(),
            &mut id,
            &mut found
        ),
        OuiStatus::Ok
    );
    assert_eq!((id, found), (0, 0));
    let mut live = 0;
    assert_eq!(
        oui_element_smooth_scroll_to_v1(source, 100.0, 0.0, 100.0, ptr::null(), &mut live),
        OuiStatus::Ok
    );
    let mut bad_curve = steps();
    bad_curve.step_count = 0;
    let nonfinite = [f64::INFINITY];
    let finite = [1.0];
    for (points, count, duration, curve) in [
        (ptr::null(), 0, -1.0, ptr::null()),
        (ptr::null(), 0, f64::NAN, ptr::null()),
        (ptr::null(), 0, 100.0, &bad_curve as *const OuiEasingCurveV1),
        (ptr::null(), 1, 100.0, ptr::null()),
        (nonfinite.as_ptr(), 1, 100.0, ptr::null()),
        (finite.as_ptr(), usize::MAX, 100.0, ptr::null()),
    ] {
        id = u64::MAX;
        found = 255;
        assert_eq!(
            oui_element_settle_scroll_snap_with_easing_v1(
                source,
                points,
                count,
                ptr::null(),
                0,
                duration,
                curve,
                &mut id,
                &mut found
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!((id, found), (u64::MAX, 255));
        assert_eq!(context.offset(source), (0.0, 0.0));
    }
    context.time(50.0);
    assert_eq!(context.offset(source), (50.0, 0.0));
    assert_eq!(
        oui_document_cancel_smooth_scroll_v1(context.document, live),
        OuiStatus::Ok
    );
}

#[test]
fn output_overlap_misalignment_and_handle_liveness_are_checked_before_action() {
    let mut context = Context::new(1.0);
    let source = context.scroller();
    let mut output = [u64::MAX; 3];
    let mut found = 255;
    let misaligned = output
        .as_mut_ptr()
        .cast::<u8>()
        .wrapping_add(1)
        .cast::<u64>();
    for destination in [ptr::null_mut(), misaligned] {
        assert_eq!(
            oui_element_settle_scroll_snap_with_easing_v1(
                source,
                [100.0].as_ptr(),
                1,
                ptr::null(),
                0,
                100.0,
                ptr::null(),
                destination,
                &mut found
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!(output, [u64::MAX; 3]);
        assert_eq!(found, 255);
    }
    let destination = output.as_mut_ptr();
    assert_eq!(
        oui_element_settle_scroll_snap_with_easing_v1(
            source,
            [100.0].as_ptr(),
            1,
            ptr::null(),
            0,
            100.0,
            ptr::null(),
            destination,
            destination.cast()
        ),
        OuiStatus::InvalidArgument
    );
    assert_eq!(output, [u64::MAX; 3]);
    assert_eq!(context.offset(source), (0.0, 0.0));
    let address = context.document as usize;
    assert_eq!(
        std::thread::spawn(move || oui_document_cancel_smooth_scroll_v1(
            address as *mut OuiDocument,
            1
        ))
        .join()
        .unwrap(),
        OuiStatus::WrongThread
    );
    assert_eq!(
        oui_document_cancel_smooth_scroll_v1(context.document, u64::MAX),
        OuiStatus::InvalidState
    );
    assert_eq!(
        oui_document_cancel_smooth_scroll_v1(ptr::null_mut(), 1),
        OuiStatus::InvalidArgument
    );
}

#[test]
fn unsorted_tied_and_outside_points_follow_the_rust_engine_selection_and_clamp() {
    for points in [
        vec![100.0, 0.0],
        vec![-50.0, 0.0, 120.0, 300.0],
        vec![300.0],
        vec![],
    ] {
        let mut context = Context::new(1.0);
        let mut native = Context::new(1.0);
        let source = context.scroller();
        let reference = native.scroller();
        assert_eq!(oui_element_scroll_to(source, 50.0, 40.0), OuiStatus::Ok);
        assert_eq!(oui_element_scroll_to(reference, 50.0, 40.0), OuiStatus::Ok);
        let (mut id, mut found) = (u64::MAX, 255);
        assert_eq!(
            oui_element_settle_scroll_snap_with_easing_v1(
                source,
                points.as_ptr(),
                points.len(),
                ptr::null(),
                0,
                100.0,
                ptr::null(),
                &mut id,
                &mut found
            ),
            OuiStatus::Ok
        );
        let target = element(reference as usize).unwrap();
        let owner = element_document(&target).unwrap();
        let expected = borrow_engine_mut(&owner)
            .unwrap()
            .settle_scroll_snap(target.node, &points, &[], 100.0, Easing::Linear)
            .unwrap();
        assert_eq!(found, u8::from(expected.is_some()));
        if expected.is_none() {
            assert_eq!(id, 0);
        }
        for time in [0.0, 25.0, 50.0, 100.0] {
            context.time(time);
            native.time(time);
            assert_eq!(context.offset(source), native.offset(reference));
        }
    }
}

#[test]
fn engine_reentry_and_retired_handles_leave_outputs_and_tracks_unchanged() {
    let mut context = Context::new(1.0);
    let source = context.scroller();
    let mut live = 0;
    assert_eq!(
        oui_element_smooth_scroll_to_v1(source, 100.0, 0.0, 100.0, ptr::null(), &mut live),
        OuiStatus::Ok
    );
    let state = document(context.document as usize).unwrap();
    let held = borrow_engine(&state).unwrap();
    let (mut id, mut found) = (u64::MAX, 255);
    assert_eq!(
        oui_document_cancel_smooth_scroll_v1(context.document, live),
        OuiStatus::Reentrant
    );
    assert_eq!(
        oui_element_settle_scroll_snap_with_easing_v1(
            source,
            [0.0].as_ptr(),
            1,
            ptr::null(),
            0,
            100.0,
            ptr::null(),
            &mut id,
            &mut found
        ),
        OuiStatus::Reentrant
    );
    assert_eq!((id, found), (u64::MAX, 255));
    drop(held);
    context.time(50.0);
    assert_eq!(context.offset(source), (50.0, 0.0));
    let old_document = context.document;
    drop(state);
    drop(context);
    assert_eq!(
        oui_document_cancel_smooth_scroll_v1(old_document, live),
        OuiStatus::InvalidHandle
    );
    assert_eq!(
        oui_element_settle_scroll_snap_with_easing_v1(
            source,
            [0.0].as_ptr(),
            1,
            ptr::null(),
            0,
            100.0,
            ptr::null(),
            &mut id,
            &mut found
        ),
        OuiStatus::InvalidHandle
    );
    assert_eq!((id, found), (u64::MAX, 255));
}
