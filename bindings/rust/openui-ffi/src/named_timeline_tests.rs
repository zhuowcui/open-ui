//! Real retained Engine/C boundary guards; execution awaits integration qualification.
use super::*;
use std::mem::{align_of, MaybeUninit};

const ZERO: OuiTimelineLengthV1 = OuiTimelineLengthV1 {
    value: 0.0,
    calc_offset: 0.0,
    unit: 0,
    reserved: 0,
};
fn offset(name: u32, unit: u32, value: f64, calc_offset: f64) -> OuiTimelineRangeOffsetV1 {
    OuiTimelineRangeOffsetV1 {
        name,
        reserved: 0,
        offset: OuiTimelineLengthV1 {
            value,
            calc_offset,
            unit,
            reserved: 0,
        },
    }
}
fn range() -> OuiAnimationAttachmentRangeV1 {
    OuiAnimationAttachmentRangeV1 {
        struct_size: size_of::<OuiAnimationAttachmentRangeV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        present: 0,
        reserved: 0,
        start: offset(0, 0, 0.0, 0.0),
        end: offset(0, 0, 0.0, 0.0),
    }
}
fn source(node: *mut OuiElement, kind: u32) -> OuiAnimationTimelineSourceV1 {
    OuiAnimationTimelineSourceV1 {
        struct_size: size_of::<OuiAnimationTimelineSourceV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        kind,
        axis: 0,
        source: node,
        inset_present: 0,
        reserved: 0,
        inset_start: ZERO,
        inset_end: ZERO,
    }
}
fn output<T>() -> T {
    // All new transport fields are numeric values/pointers, so all-zero is valid.
    let mut value = unsafe { MaybeUninit::<T>::zeroed().assume_init() };
    unsafe {
        ptr::write_unaligned((&mut value as *mut T).cast::<u32>(), size_of::<T>() as u32);
        ptr::write_unaligned((&mut value as *mut T).cast::<u32>().add(1), OUI_ABI_VERSION);
    }
    value
}
fn options() -> OuiAnimationOptionsV1 {
    OuiAnimationOptionsV1 {
        struct_size: size_of::<OuiAnimationOptionsV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        delay_ms: 0.0,
        duration_ms: 100.0,
        iterations: 1.0,
        playback_rate: 1.0,
        direction: 0,
        fill: 3,
        play_state: 0,
        composite: 0,
        easing: ptr::null(),
    }
}
fn frame(offset: f64, number: f32) -> OuiKeyframeV1 {
    OuiKeyframeV1 {
        struct_size: size_of::<OuiKeyframeV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        offset,
        value: OuiStyleValue {
            tag: 2,
            reserved: 0,
            data: OuiStylePayload { number },
        },
        easing: ptr::null(),
    }
}
struct Context {
    document: *mut OuiDocument,
    root: *mut OuiElement,
    nodes: Vec<*mut OuiElement>,
}
impl Context {
    fn new() -> Self {
        let mut document = ptr::null_mut();
        let mut root = ptr::null_mut();
        assert_eq!(
            oui_document_create(
                &OuiDocumentConfig {
                    struct_size: size_of::<OuiDocumentConfig>() as u32,
                    abi_version: OUI_ABI_VERSION,
                    viewport: OuiViewportMetrics {
                        logical_width: 320.0,
                        logical_height: 240.0,
                        physical_width: 320,
                        physical_height: 240,
                        device_scale_factor: 1.0,
                        authority: 1,
                        reserved: 0
                    }
                },
                &mut document
            ),
            OuiStatus::Ok
        );
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
    fn set(&self, node: *mut OuiElement, property: StyleProperty, value: impl Into<StyleValue>) {
        let handle = element(node as usize).unwrap();
        let state = element_document(&handle).unwrap();
        borrow_engine_mut(&state)
            .unwrap()
            .set_property(handle.node, property, value.into())
            .unwrap();
    }
    fn port(&mut self) -> (*mut OuiElement, *mut OuiElement) {
        let port = self.node(self.root);
        self.set(port, StyleProperty::Width, LengthValue::px(100.0));
        self.set(port, StyleProperty::Height, LengthValue::px(80.0));
        self.set(
            port,
            StyleProperty::Overflow,
            openui_style::Overflow::Hidden,
        );
        let content = self.node(port);
        self.set(content, StyleProperty::Width, LengthValue::px(100.0));
        self.set(content, StyleProperty::Height, LengthValue::px(240.0));
        (port, content)
    }
    fn animate(
        &self,
        node: *mut OuiElement,
        source: *const OuiAnimationTimelineSourceV1,
        range: *const OuiAnimationAttachmentRangeV1,
    ) -> u64 {
        let frames = [frame(0.0, 0.0), frame(1.0, 1.0)];
        let mut id = u64::MAX;
        assert_eq!(
            oui_element_animate_with_attachment_range_v1(
                node,
                StyleProperty::Opacity as i32,
                frames.as_ptr(),
                2,
                size_of::<OuiKeyframeV1>(),
                &options(),
                source,
                range,
                &mut id
            ),
            OuiStatus::Ok
        );
        id
    }
    fn count(&self) -> usize {
        let state = document(self.document as usize).unwrap();
        let n = borrow_engine(&state).unwrap().active_animation_count();
        n
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

#[test]
fn append_only_named_transport_layouts_match_the_public_contract() {
    assert_eq!(
        (
            size_of::<OuiAnimationTimeline>(),
            align_of::<OuiAnimationTimeline>()
        ),
        (40, 8)
    );
    assert_eq!(
        (
            size_of::<OuiAnimationOptionsV1>(),
            size_of::<OuiKeyframeV1>()
        ),
        (64, 40)
    );
    assert_eq!(
        (
            size_of::<OuiTimelineLengthV1>(),
            size_of::<OuiTimelineRangeOffsetV1>()
        ),
        (24, 32)
    );
    assert_eq!(
        (
            size_of::<OuiAnimationAttachmentRangeV1>(),
            size_of::<OuiAnimationTimelineSourceV1>()
        ),
        (80, 80)
    );
    assert_eq!(
        (
            size_of::<OuiAnimationTimelineSnapshotV1>(),
            size_of::<OuiAnimationAttachmentSnapshotV1>()
        ),
        (96, 248)
    );
}
#[test]
fn named_range_inputs_roundtrip_every_native_unit_and_name_without_resolving_document_geometry() {
    let mut context = Context::new();
    let node = context.node(context.root);
    let id = context.animate(node, ptr::null(), ptr::null());
    for name in 0..7 {
        for unit in [0, 1, 2, 3, 4, 5, 8, 9, 10, 11, 12, 13] {
            let mut input = range();
            input.present = 1;
            input.start = offset(name, unit, -12.1, if unit == 13 { 3.1 } else { 0.0 });
            assert_eq!(
                oui_document_set_animation_attachment_range_v1(context.document, id, &input),
                OuiStatus::Ok
            );
            let mut snapshot: OuiAnimationAttachmentSnapshotV1 = output();
            assert_eq!(
                oui_document_get_animation_attachment_snapshot_v1(
                    context.document,
                    id,
                    &mut snapshot
                ),
                OuiStatus::Ok
            );
            assert_eq!(snapshot.present, 1);
            assert!(snapshot.source.is_null());
            assert_eq!(snapshot.range.start.name, name);
            assert_eq!(snapshot.range.start.offset.unit, unit);
            assert_eq!(snapshot.range.start.offset.value, (-12.1_f64 as f32) as f64);
            assert_eq!(
                snapshot.range.start.offset.calc_offset,
                if unit == 13 {
                    (3.1_f64 as f32) as f64
                } else {
                    0.0
                }
            );
        }
    }
    let mut timeline: OuiAnimationTimelineSnapshotV1 = output();
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(context.document, id, &mut timeline),
        OuiStatus::Ok
    );
    assert_eq!(timeline.present, 0);
    assert!(timeline.source.is_null());
    assert_eq!(
        oui_document_seek_animation_progress_v1(context.document, id, 25.0),
        OuiStatus::InvalidState
    );
}
#[test]
fn short_bad_reserved_and_nonfinite_inputs_leave_animation_id_and_stack_unchanged() {
    let mut context = Context::new();
    let node = context.node(context.root);
    let frames = [frame(0.0, 0.0), frame(1.0, 1.0)];
    let count = context.count();
    for bad in [
        offset(99, 0, 1.0, 0.0),
        offset(0, 6, 0.0, 0.0),
        offset(0, 0, f64::MAX, 0.0),
        offset(0, 13, 0.0, f64::NAN),
    ] {
        let mut input = range();
        input.present = 1;
        input.start = bad;
        let mut id = u64::MAX;
        assert_eq!(
            oui_element_animate_with_attachment_range_v1(
                node,
                StyleProperty::Opacity as i32,
                frames.as_ptr(),
                2,
                size_of::<OuiKeyframeV1>(),
                &options(),
                ptr::null(),
                &input,
                &mut id
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!(id, u64::MAX);
        assert_eq!(context.count(), count);
    }
    let short4 = [4_u32];
    let short8 = [8_u32, OUI_ABI_VERSION];
    for input in [short4.as_ptr().cast(), short8.as_ptr().cast()] {
        let mut id = u64::MAX;
        assert_eq!(
            oui_element_animate_with_attachment_range_v1(
                node,
                StyleProperty::Opacity as i32,
                frames.as_ptr(),
                2,
                size_of::<OuiKeyframeV1>(),
                &options(),
                ptr::null(),
                input,
                &mut id
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!(id, u64::MAX);
    }
    let mut input = range();
    input.abi_version = 0;
    assert_eq!(
        oui_document_set_animation_attachment_range_v1(context.document, 999, &input),
        OuiStatus::AbiMismatch
    );
    input = range();
    input.reserved = 1;
    assert_eq!(
        oui_document_set_animation_attachment_range_v1(context.document, 999, &input),
        OuiStatus::InvalidArgument
    );
}
#[test]
fn snapshots_accept_only_initialized_headers_and_preserve_future_tail_and_error_bytes() {
    let mut context = Context::new();
    let node = context.node(context.root);
    let id = context.animate(node, ptr::null(), ptr::null());
    let mut short4 = [4_u32];
    let mut short8 = [8_u32, OUI_ABI_VERSION];
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(
            context.document,
            id,
            short4.as_mut_ptr().cast()
        ),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_document_get_animation_attachment_snapshot_v1(
            context.document,
            id,
            short8.as_mut_ptr().cast()
        ),
        OuiStatus::InvalidArgument
    );
    assert_eq!(short4, [4]);
    assert_eq!(short8, [8, OUI_ABI_VERSION]);
    #[repr(C)]
    struct Future {
        known: OuiAnimationAttachmentSnapshotV1,
        tail: [u64; 2],
    }
    let mut future = Future {
        known: output(),
        tail: [0xabba, 0xdeed],
    };
    future.known.struct_size = size_of::<Future>() as u32;
    assert_eq!(
        oui_document_get_animation_attachment_snapshot_v1(context.document, id, &mut future.known),
        OuiStatus::Ok
    );
    assert_eq!(future.tail, [0xabba, 0xdeed]);
    assert_eq!(future.known.struct_size, size_of::<Future>() as u32);
    let mut raw = [0x5a5a5a5a5a5a5a5a_u64; 40];
    let p = raw.as_mut_ptr().cast::<OuiAnimationAttachmentSnapshotV1>();
    unsafe {
        ptr::write_unaligned(
            p.cast::<u32>(),
            size_of::<OuiAnimationAttachmentSnapshotV1>() as u32,
        );
        ptr::write_unaligned(p.cast::<u32>().add(1), OUI_ABI_VERSION);
    }
    let before = raw;
    assert_eq!(
        oui_document_get_animation_attachment_snapshot_v1(context.document, u64::MAX, p),
        OuiStatus::InvalidState
    );
    assert_eq!(raw, before);
    unsafe {
        ptr::write_unaligned(p.cast::<u32>().add(1), 0);
    }
    let before = raw;
    assert_eq!(
        oui_document_get_animation_attachment_snapshot_v1(context.document, id, p),
        OuiStatus::AbiMismatch
    );
    assert_eq!(raw, before);
}
#[test]
fn wrong_document_wrong_thread_and_borrowed_engine_never_transfer_outputs_or_aliases() {
    let mut a = Context::new();
    let mut b = Context::new();
    let target = a.node(a.root);
    let foreign = b.node(b.root);
    let frames = [frame(0.0, 0.0), frame(1.0, 1.0)];
    let mut id = u64::MAX;
    assert_eq!(
        oui_element_animate_with_attachment_range_v1(
            target,
            StyleProperty::Opacity as i32,
            frames.as_ptr(),
            2,
            size_of::<OuiKeyframeV1>(),
            &options(),
            &source(foreign, 1),
            ptr::null(),
            &mut id
        ),
        OuiStatus::WrongDocument
    );
    assert_eq!(id, u64::MAX);
    assert_eq!(a.count(), 0);
    let document_address = a.document as usize;
    assert_eq!(
        std::thread::spawn(move || {
            let mut snapshot: OuiAnimationTimelineSnapshotV1 = output();
            snapshot.present = 99;
            let status = oui_document_get_animation_timeline_snapshot_v1(
                document_address as *mut OuiDocument,
                1,
                &mut snapshot,
            );
            assert_eq!(snapshot.present, 99);
            status
        })
        .join()
        .unwrap(),
        OuiStatus::WrongThread
    );
    let id = a.animate(target, ptr::null(), ptr::null());
    let state = document(a.document as usize).unwrap();
    let _borrow = borrow_engine_mut(&state).unwrap();
    let mut snapshot: OuiAnimationAttachmentSnapshotV1 = output();
    snapshot.present = 99;
    assert_eq!(
        oui_document_get_animation_attachment_snapshot_v1(a.document, id, &mut snapshot),
        OuiStatus::Reentrant
    );
    assert_eq!(snapshot.present, 99);
}
#[test]
fn scroll_and_view_snapshots_equal_the_shared_native_owned_geometry_and_release_aliases() {
    let mut context = Context::new();
    let (port, subject) = context.port();
    let target = context.node(context.root);
    for kind in [1, 2] {
        let mut input = source(if kind == 1 { port } else { subject }, kind);
        if kind == 2 {
            input.inset_present = 3;
            input.inset_start = OuiTimelineLengthV1 {
                unit: 13,
                value: 10.0,
                calc_offset: 2.0,
                reserved: 0,
            };
            input.inset_end = OuiTimelineLengthV1 {
                unit: 2,
                value: -0.5,
                calc_offset: 0.0,
                reserved: 0,
            };
        }
        let mut range = range();
        range.present = 3;
        range.start = offset(2, 1, 0.0, 0.0);
        range.end = offset(4, 1, 100.0, 0.0);
        let id = context.animate(target, &input, &range);
        range.start.offset.value = 999.0;
        input.inset_start.value = 999.0;
        let state = document(context.document as usize).unwrap();
        let native = borrow_engine_mut(&state)
            .unwrap()
            .animation_attachment_snapshot(AnimationId(id))
            .unwrap()
            .unwrap();
        let mut snapshot: OuiAnimationAttachmentSnapshotV1 = output();
        assert_eq!(
            oui_document_get_animation_attachment_snapshot_v1(context.document, id, &mut snapshot),
            OuiStatus::Ok
        );
        assert_eq!(snapshot.present, 3);
        assert_eq!(snapshot.range.start.offset.value, 0.0);
        let resolved = native.resolved.unwrap();
        for (actual, expected) in snapshot.named.iter().zip([
            resolved.named.cover,
            resolved.named.contain,
            resolved.named.entry,
            resolved.named.entry_crossing,
            resolved.named.exit,
            resolved.named.exit_crossing,
            resolved.named.scroll,
        ]) {
            assert_eq!((actual.start, actual.end), (expected.start, expected.end));
        }
        assert_eq!(
            (snapshot.attachment.start, snapshot.attachment.end),
            (resolved.attachment.start, resolved.attachment.end)
        );
        assert_eq!(
            element(snapshot.source as usize).unwrap().node,
            resolved.source
        );
        assert_eq!(oui_element_destroy(snapshot.source), OuiStatus::Ok);
        let native = borrow_engine_mut(&state)
            .unwrap()
            .animation_timeline_snapshot(AnimationId(id))
            .unwrap()
            .unwrap();
        let mut timeline: OuiAnimationTimelineSnapshotV1 = output();
        assert_eq!(
            oui_document_get_animation_timeline_snapshot_v1(context.document, id, &mut timeline),
            OuiStatus::Ok
        );
        assert_eq!(timeline.active, u32::from(native.active));
        assert_eq!(timeline.scroll_offset, native.scroll_offset.unwrap());
        assert_eq!(timeline.range_start, native.range_start);
        assert_eq!(timeline.range_end, native.range_end);
        assert_eq!(oui_element_destroy(timeline.source), OuiStatus::Ok);
    }
}
#[test]
fn named_effect_owns_arbitrary_easing_points_and_pending_controls_use_the_same_native_path() {
    let mut context = Context::new();
    let (port, _) = context.port();
    let target = context.node(context.root);
    let mut points = [
        OuiLinearStopV1 {
            input: 0.0,
            output: 0.0,
        },
        OuiLinearStopV1 {
            input: 0.5,
            output: 0.25,
        },
        OuiLinearStopV1 {
            input: 1.0,
            output: 1.0,
        },
    ];
    let curve = OuiEasingCurveV1 {
        struct_size: size_of::<OuiEasingCurveV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        kind: 3,
        step_position: 0,
        step_count: 0,
        reserved: 0,
        values: [0.0; 4],
        stops: points.as_ptr(),
        stop_count: 3,
        stop_stride: size_of::<OuiLinearStopV1>(),
    };
    let mut timing = options();
    timing.easing = &curve;
    let frames = [frame(0.0, 0.0), frame(1.0, 1.0)];
    let mut id = 0;
    assert_eq!(
        oui_element_animate_with_attachment_range_v1(
            target,
            StyleProperty::Opacity as i32,
            frames.as_ptr(),
            2,
            size_of::<OuiKeyframeV1>(),
            &timing,
            &source(port, 1),
            ptr::null(),
            &mut id
        ),
        OuiStatus::Ok
    );
    points[1].output = 0.9;
    assert_eq!(
        oui_document_seek_animation_progress_v1(context.document, id, 25.0),
        OuiStatus::Ok
    );
    let handle = element(target as usize).unwrap();
    let state = element_document(&handle).unwrap();
    assert_eq!(
        borrow_engine(&state)
            .unwrap()
            .computed_style(handle.node)
            .unwrap()
            .opacity,
        0.125
    );
    context.set(port, StyleProperty::Display, openui_style::Display::None);
    assert_eq!(
        oui_document_seek_animation_progress_v1(context.document, id, 25.0),
        OuiStatus::Ok
    );
    assert_eq!(
        oui_document_animation_seek(context.document, id, 20.0),
        OuiStatus::Ok
    );
    context.set(port, StyleProperty::Display, openui_style::Display::Block);
    let mut snapshot: OuiAnimationTimelineSnapshotV1 = output();
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(context.document, id, &mut snapshot),
        OuiStatus::Ok
    );
    assert_eq!(snapshot.current_time_ms, 20.0);
    assert_eq!(oui_element_destroy(snapshot.source), OuiStatus::Ok);
}

struct CancelCallback {
    document: *mut OuiDocument,
    id: Cell<u64>,
    source: *mut OuiElement,
    remove_source: bool,
    calls: Cell<u32>,
}
extern "C" fn cancel_during_snapshot(_: *mut OuiEvent, data: *mut c_void) {
    let context = unsafe { &*data.cast::<CancelCallback>() };
    let mut active = 0;
    assert_eq!(
        oui_document_is_animating(context.document, &mut active),
        OuiStatus::Ok
    );
    if context.remove_source {
        assert_eq!(oui_element_remove(context.source), OuiStatus::Ok);
    } else {
        assert_eq!(
            oui_document_animation_cancel(context.document, context.id.get()),
            OuiStatus::Ok
        );
    }
    context.calls.set(context.calls.get() + 1);
}
#[test]
fn snapshot_callbacks_may_cancel_or_remove_sources_without_borrows_output_writes_or_orphaned_aliases(
) {
    for remove_source in [false, true] {
        let mut context = Context::new();
        let (port, _) = context.port();
        let target = context.node(context.root);
        let callback = CancelCallback {
            document: context.document,
            id: Cell::new(0),
            source: port,
            remove_source,
            calls: Cell::new(0),
        };
        let mut listener = ptr::null_mut();
        assert_eq!(
            oui_element_add_event_listener(
                target,
                20,
                0,
                Some(cancel_during_snapshot),
                (&callback as *const CancelCallback).cast_mut().cast(),
                &mut listener
            ),
            OuiStatus::Ok
        );
        let state = document(context.document as usize).unwrap();
        let target = element(target as usize).unwrap();
        let source = element(port as usize).unwrap();
        let id = borrow_engine_mut(&state)
            .unwrap()
            .animate_with_attachment_range(
                target.node,
                PropertyKeyframes::typed(
                    StyleProperty::Opacity,
                    Keyframes::from_values(StyleValue::Number(0.0), StyleValue::Number(1.0)),
                )
                .unwrap(),
                AnimationOptions {
                    duration_ms: 100.0,
                    fill: FillMode::Both,
                    ..AnimationOptions::default()
                },
                openui_engine::AnimationTimelineSource::Scroll {
                    source: source.node,
                    axis: TimelineAxis::Block,
                },
                openui_style::AnimationAttachmentRange::default(),
            )
            .unwrap();
        callback.id.set(id.0);
        let aliases_before: usize = state.element_handles.borrow().values().map(Vec::len).sum();
        let mut raw = [0xabababababababab_u64; 40];
        let output = raw.as_mut_ptr().cast::<OuiAnimationAttachmentSnapshotV1>();
        unsafe {
            ptr::write_unaligned(
                output.cast::<u32>(),
                size_of::<OuiAnimationAttachmentSnapshotV1>() as u32,
            );
            ptr::write_unaligned(output.cast::<u32>().add(1), OUI_ABI_VERSION);
        }
        let before = raw;
        assert_eq!(
            oui_document_get_animation_attachment_snapshot_v1(context.document, id.0, output),
            if remove_source {
                OuiStatus::StaleHandle
            } else {
                OuiStatus::InvalidState
            }
        );
        assert_eq!(callback.calls.get(), 1);
        assert_eq!(raw, before);
        assert_eq!(
            state
                .element_handles
                .borrow()
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            aliases_before
        );
        assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
    }
}
#[test]
fn stale_sources_and_unaligned_full_outputs_are_rejected_before_any_output_change() {
    let mut context = Context::new();
    let (port, _) = context.port();
    let target = context.node(context.root);
    let frames = [frame(0.0, 0.0), frame(1.0, 1.0)];
    let source = source(port, 1);
    assert_eq!(oui_element_remove(port), OuiStatus::Ok);
    let mut id = u64::MAX;
    assert_eq!(
        oui_element_animate_with_attachment_range_v1(
            target,
            StyleProperty::Opacity as i32,
            frames.as_ptr(),
            2,
            size_of::<OuiKeyframeV1>(),
            &options(),
            &source,
            ptr::null(),
            &mut id
        ),
        OuiStatus::StaleHandle
    );
    assert_eq!(id, u64::MAX);
    let id = context.animate(target, ptr::null(), ptr::null());
    let mut storage = [0x55_u8; 112];
    let base = storage.as_mut_ptr() as usize;
    let shift = if base % 8 == 0 { 1 } else { 0 };
    let p = unsafe { storage.as_mut_ptr().add(shift) }.cast::<OuiAnimationTimelineSnapshotV1>();
    unsafe {
        ptr::write_unaligned(
            p.cast::<u32>(),
            size_of::<OuiAnimationTimelineSnapshotV1>() as u32,
        );
        ptr::write_unaligned(p.cast::<u32>().add(1), OUI_ABI_VERSION);
    }
    let before = storage;
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(context.document, id, p),
        OuiStatus::InvalidArgument
    );
    assert_eq!(storage, before);
}

#[test]
fn future_input_tails_and_unused_source_fields_have_explicit_owned_contracts() {
    let mut context = Context::new();
    let (port, _) = context.port();
    let target = context.node(context.root);
    #[repr(C)]
    struct FutureSource {
        known: OuiAnimationTimelineSourceV1,
        tail: u64,
    }
    #[repr(C)]
    struct FutureRange {
        known: OuiAnimationAttachmentRangeV1,
        tail: u64,
    }
    let mut input = FutureSource {
        known: source(port, 1),
        tail: 0xcafe,
    };
    input.known.struct_size = size_of::<FutureSource>() as u32;
    let mut attachment = FutureRange {
        known: range(),
        tail: 0xbeef,
    };
    attachment.known.struct_size = size_of::<FutureRange>() as u32;
    let id = context.animate(target, &input.known, &attachment.known);
    assert_eq!(input.tail, 0xcafe);
    assert_eq!(attachment.tail, 0xbeef);
    let mut snapshot: OuiAnimationTimelineSnapshotV1 = output();
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(context.document, id, &mut snapshot),
        OuiStatus::Ok
    );
    assert_eq!(oui_element_destroy(snapshot.source), OuiStatus::Ok);
    let frames = [frame(0.0, 0.0), frame(1.0, 1.0)];
    for (kind, axis, reserved, inset_present) in [
        (9, 0, 0, 0),
        (1, 9, 0, 0),
        (1, 0, 1, 0),
        (1, 0, 0, 1),
        (2, 0, 0, 4),
    ] {
        let mut input = source(port, kind);
        input.axis = axis;
        input.reserved = reserved;
        input.inset_present = inset_present;
        let mut output = u64::MAX;
        assert_eq!(
            oui_element_animate_with_attachment_range_v1(
                target,
                StyleProperty::Opacity as i32,
                frames.as_ptr(),
                2,
                size_of::<OuiKeyframeV1>(),
                &options(),
                &input,
                ptr::null(),
                &mut output
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!(output, u64::MAX);
    }
    let state = document(context.document as usize).unwrap();
    let before = borrow_engine(&state).unwrap().stats();
    assert_eq!(
        oui_document_seek_animation_progress_v1(context.document, id, f64::NAN),
        OuiStatus::InvalidArgument
    );
    assert_eq!(borrow_engine(&state).unwrap().stats(), before);
}
