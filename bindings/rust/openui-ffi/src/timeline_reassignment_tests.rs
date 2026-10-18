//! Real retained Engine/C boundary guards; execution awaits integration qualification.
use super::*;
use std::mem::MaybeUninit;

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
        duration_ms: 1000.0,
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
        if !self.root.is_null() {
            assert_eq!(oui_element_destroy(self.root), OuiStatus::Ok);
        }
        if !self.document.is_null() {
            assert_eq!(oui_document_destroy(self.document), OuiStatus::Ok);
        }
    }
}

fn animation(context: &Context, id: u64) -> OuiAnimationState {
    let mut state: OuiAnimationState = output();
    assert_eq!(
        oui_document_animation_get_state(context.document, id, &mut state),
        OuiStatus::Ok
    );
    state
}
fn percentage(context: &Context, id: u64) -> f64 {
    let mut value: OuiAnimationTimelineSnapshotV1 = output();
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(context.document, id, &mut value),
        OuiStatus::Ok
    );
    assert_eq!(oui_element_destroy(value.source), OuiStatus::Ok);
    value.current_time_percentage
}
fn raw_bytes<T>(value: &T) -> Vec<u8> {
    // These guards use fully initialized repr(C) carriers with no uninitialized padding.
    unsafe { std::slice::from_raw_parts((value as *const T).cast(), size_of::<T>()).to_vec() }
}
#[test]
fn source_setter_copies_input_and_preserves_idle_identity_and_paused_document_hold() {
    let mut c = Context::new();
    let (a, _) = c.port();
    let (b, _) = c.port();
    let target = c.node(c.root);
    assert_eq!(oui_element_scroll_to(a, 0.0, 40.0), OuiStatus::Ok);
    assert_eq!(oui_element_scroll_to(b, 0.0, 120.0), OuiStatus::Ok);
    let id = c.animate(target, &source(a, 1), ptr::null());
    assert_eq!(
        oui_document_seek_animation_progress_v1(c.document, id, 50.0),
        OuiStatus::Ok
    );
    let before = animation(&c, id);
    let owner = document(c.document as usize).unwrap();
    let generations = borrow_engine(&owner).unwrap().dirty_generations();
    let stats = borrow_engine(&owner).unwrap().stats();
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, &source(a, 1)),
        OuiStatus::Ok
    );
    assert_eq!(animation(&c, id).current_time_ms, before.current_time_ms);
    assert_eq!(
        borrow_engine(&owner).unwrap().dirty_generations(),
        generations
    );
    assert_eq!(borrow_engine(&owner).unwrap().stats(), stats);
    #[repr(C)]
    struct Future {
        known: OuiAnimationTimelineSourceV1,
        tail: u64,
    }
    let mut future = Future {
        known: source(b, 1),
        tail: 0xcafe,
    };
    future.known.struct_size = size_of::<Future>() as u32;
    let bytes = raw_bytes(&future);
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, &future.known),
        OuiStatus::Ok
    );
    assert_eq!(raw_bytes(&future), bytes);
    assert_eq!(percentage(&c, id), 75.0);
    future.known.source = a;
    future.known.kind = 99;
    assert_eq!(percentage(&c, id), 75.0);
    assert_eq!(oui_document_animation_pause(c.document, id), OuiStatus::Ok);
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, ptr::null()),
        OuiStatus::Ok
    );
    let result = animation(&c, id);
    assert_eq!(result.current_time_ms, 750.0);
    assert_eq!(result.play_state, 1);
    let mut progress: OuiAnimationTimelineSnapshotV1 = output();
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(c.document, id, &mut progress),
        OuiStatus::Ok
    );
    assert_eq!(progress.present, 0);
    assert!(progress.source.is_null());
    let mut attachment: OuiAnimationAttachmentSnapshotV1 = output();
    assert_eq!(
        oui_document_get_animation_attachment_snapshot_v1(c.document, id, &mut attachment),
        OuiStatus::Ok
    );
    assert_eq!(attachment.present, 1);
    assert!(attachment.source.is_null());
}
#[test]
fn rejected_source_metadata_leaves_the_effect_and_input_unchanged() {
    let mut c = Context::new();
    let (a, _) = c.port();
    let (b, _) = c.port();
    let target = c.node(c.root);
    let id = c.animate(target, &source(a, 1), ptr::null());
    let old = animation(&c, id);
    let owner = document(c.document as usize).unwrap();
    let generations = borrow_engine(&owner).unwrap().dirty_generations();
    let mut invalids = Vec::new();
    let mut v = source(b, 1);
    v.kind = 99;
    invalids.push(v);
    let mut v = source(b, 1);
    v.axis = 4;
    invalids.push(v);
    let mut v = source(b, 1);
    v.reserved = 1;
    invalids.push(v);
    let mut v = source(b, 1);
    v.inset_present = 1;
    invalids.push(v);
    let mut v = source(b, 2);
    v.inset_present = 1;
    v.inset_start.value = f64::MAX;
    invalids.push(v);
    let mut v = source(b, 2);
    v.inset_present = 1;
    v.inset_start.value = f64::NAN;
    invalids.push(v);
    for value in invalids {
        let before = raw_bytes(&value);
        assert_eq!(
            oui_document_set_animation_timeline_source_v1(c.document, id, &value),
            OuiStatus::InvalidArgument
        );
        assert_eq!(raw_bytes(&value), before);
        assert_eq!(animation(&c, id).current_time_ms, old.current_time_ms);
        assert_eq!(
            borrow_engine(&owner).unwrap().dirty_generations(),
            generations
        );
    }
    let short = [8u32, OUI_ABI_VERSION];
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, short.as_ptr().cast()),
        OuiStatus::InvalidArgument
    );
    let mut v = source(b, 1);
    v.abi_version = 0;
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, &v),
        OuiStatus::AbiMismatch
    );
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, u64::MAX, &source(b, 1)),
        OuiStatus::InvalidState
    );
    assert_eq!(c.count(), 1);
}
#[test]
fn source_owner_thread_borrow_and_generation_checks_precede_publication() {
    let mut c = Context::new();
    let (a, _) = c.port();
    let (b, _) = c.port();
    let target = c.node(c.root);
    let id = c.animate(target, &source(a, 1), ptr::null());
    let mut foreign = Context::new();
    let (f, _) = foreign.port();
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, &source(f, 1)),
        OuiStatus::WrongDocument
    );
    let owner = document(c.document as usize).unwrap();
    {
        let _borrow = borrow_engine_mut(&owner).unwrap();
        assert_eq!(
            oui_document_set_animation_timeline_source_v1(c.document, id, &source(b, 1)),
            OuiStatus::Reentrant
        );
    }
    let doc = c.document as usize;
    let address = b as usize;
    assert_eq!(
        std::thread::spawn(move || oui_document_set_animation_timeline_source_v1(
            doc as *mut OuiDocument,
            id,
            &source(address as *mut OuiElement, 1)
        ))
        .join()
        .unwrap(),
        OuiStatus::WrongThread
    );
    assert_eq!(oui_element_remove(a), OuiStatus::Ok);
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, &source(a, 1)),
        OuiStatus::StaleHandle
    );
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, &source(b, 1)),
        OuiStatus::Ok
    );
    let dead = c.document;
    c.document = ptr::null_mut();
    assert_eq!(oui_document_destroy(dead), OuiStatus::Ok);
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(dead, id, ptr::null()),
        OuiStatus::InvalidHandle
    );
    // Destroy aliases after their owning document dies; there is no retained callback or input pointer.
    for n in c.nodes.drain(..) {
        assert_eq!(oui_element_destroy(n), OuiStatus::Ok);
    }
    assert_eq!(oui_element_destroy(c.root), OuiStatus::Ok);
    c.root = ptr::null_mut();
    drop(c);
}
struct Reentry {
    document: *mut OuiDocument,
    id: Cell<u64>,
    calls: Cell<u32>,
    set_status: Cell<OuiStatus>,
    pause_status: Cell<OuiStatus>,
}
extern "C" fn rebind_from_start(_: *mut OuiEvent, data: *mut c_void) {
    let c = unsafe { &*data.cast::<Reentry>() };
    c.calls.set(c.calls.get() + 1);
    c.set_status
        .set(oui_document_set_animation_timeline_source_v1(
            c.document,
            c.id.get(),
            ptr::null(),
        ));
    c.pause_status
        .set(oui_document_animation_pause(c.document, c.id.get()));
}
#[test]
fn callbacks_reenter_after_commit_and_a_later_control_wins() {
    let mut c = Context::new();
    let (a, _) = c.port();
    let (b, _) = c.port();
    let target = c.node(c.root);
    assert_eq!(oui_element_scroll_to(b, 0.0, 120.0), OuiStatus::Ok);
    let mut r = range();
    r.present = 1;
    r.start = offset(6, 1, 50.0, 0.0);
    let id = c.animate(target, &source(a, 1), &r);
    let callback = Reentry {
        document: c.document,
        id: Cell::new(id),
        calls: Cell::new(0),
        set_status: Cell::new(OuiStatus::Internal),
        pause_status: Cell::new(OuiStatus::Internal),
    };
    let mut listener = ptr::null_mut();
    assert_eq!(
        oui_element_add_event_listener(
            target,
            20,
            0,
            Some(rebind_from_start),
            (&callback as *const Reentry).cast_mut().cast(),
            &mut listener
        ),
        OuiStatus::Ok
    );
    assert_eq!(
        oui_document_set_animation_timeline_source_v1(c.document, id, &source(b, 1)),
        OuiStatus::Ok
    );
    assert_eq!(callback.calls.get(), 1);
    assert_eq!(callback.set_status.get(), OuiStatus::Ok);
    assert_eq!(callback.pause_status.get(), OuiStatus::Ok);
    let state = animation(&c, id);
    // The range start is half the cover duration. Preserve CalculateCurrentTime
    // subtraction before the setTimeline EffectEnd ratio and ms reflection.
    let duration = (160.0 * 16.0 / 1000.0) / 1000.0;
    let current = (1920.0 / 1_000_000.0) - 0.5 * duration;
    assert_eq!(state.current_time_ms, (current / (duration * 0.5)) * 1000.0);
    assert_eq!(state.play_state, 1);
    let mut snapshot: OuiAnimationTimelineSnapshotV1 = output();
    assert_eq!(
        oui_document_get_animation_timeline_snapshot_v1(c.document, id, &mut snapshot),
        OuiStatus::Ok
    );
    assert_eq!(snapshot.present, 0);
    assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
}
