//! Caller storage guards for the original versioned output APIs.

use super::*;
use std::mem::{align_of, MaybeUninit};

type Query = fn(*mut OuiDocument, *mut c_void) -> OuiStatus;

fn error(_: *mut OuiDocument, output: *mut c_void) -> OuiStatus {
    oui_error_get_last(output.cast())
}

fn accessibility(document: *mut OuiDocument, output: *mut c_void) -> OuiStatus {
    oui_document_accessibility_update(document, output.cast())
}

fn animation(document: *mut OuiDocument, output: *mut c_void) -> OuiStatus {
    oui_document_animation_get_state(document, u64::MAX, output.cast())
}

fn event(document: *mut OuiDocument, output: *mut c_void) -> OuiStatus {
    let mut has_event = 0xa5;
    let status = oui_document_take_animation_event(document, output.cast(), &mut has_event);
    if status != OuiStatus::Ok {
        assert_eq!(has_event, 0xa5);
    }
    status
}

fn bitmap(document: *mut OuiDocument, output: *mut c_void) -> OuiStatus {
    oui_document_render_rgba(document, output.cast())
}

fn font(_: *mut OuiDocument, output: *mut c_void) -> OuiStatus {
    oui_font_face_get_info(ptr::null_mut(), output.cast())
}

const QUERIES: [(usize, Query); 6] = [
    (size_of::<OuiErrorInfo>(), error),
    (size_of::<OuiAccessibilityUpdate>(), accessibility),
    (size_of::<OuiAnimationState>(), animation),
    (size_of::<OuiAnimationEvent>(), event),
    (size_of::<OuiBitmap>(), bitmap),
    (size_of::<OuiFontFaceInfo>(), font),
];

// Exact four/eight-byte allocations prove that rejected outputs need no
// storage for the payload or for a second header word in the four-byte case.
#[test]
fn versioned_getters_reject_short_storage_before_reading_payloads() {
    for (_, query) in QUERIES {
        let mut size_only = Box::new(4u32);
        assert_eq!(
            query(ptr::null_mut(), (&mut *size_only as *mut u32).cast()),
            OuiStatus::InvalidArgument
        );
        assert_eq!(*size_only, 4);
        for (version, expected) in [
            (OUI_ABI_VERSION, OuiStatus::InvalidArgument),
            (OUI_ABI_VERSION ^ 1, OuiStatus::AbiMismatch),
        ] {
            let mut header = Box::new([8u32, version]);
            let before = *header;
            assert_eq!(query(ptr::null_mut(), header.as_mut_ptr().cast()), expected);
            assert_eq!(*header, before);
        }
        assert_eq!(
            query(ptr::null_mut(), ptr::null_mut()),
            OuiStatus::InvalidArgument
        );
    }
}

struct Output<T> {
    storage: Vec<MaybeUninit<u64>>,
    marker: std::marker::PhantomData<T>,
}

impl<T> Output<T> {
    fn new(initialized: bool) -> Self {
        assert!(align_of::<T>() <= align_of::<u64>());
        let words = (size_of::<T>() + 16).div_ceil(size_of::<u64>());
        let value = if initialized {
            MaybeUninit::new(0xa5a5_a5a5_a5a5_a5a5u64)
        } else {
            MaybeUninit::uninit()
        };
        let mut output = Self {
            storage: vec![value; words],
            marker: std::marker::PhantomData,
        };
        let length = output.length();
        // Initialize just the header and future tail for successful calls.
        // Payload fields remain uninitialized unless this is an error guard.
        unsafe {
            ptr::write(output.pointer().cast::<u32>(), length as u32);
            ptr::write(output.pointer().cast::<u32>().add(1), OUI_ABI_VERSION);
            ptr::write_bytes(
                output.pointer().cast::<u8>().add(size_of::<T>()),
                0xa5,
                length - size_of::<T>(),
            );
        }
        output
    }

    fn pointer(&mut self) -> *mut T {
        self.storage.as_mut_ptr().cast()
    }

    fn length(&self) -> usize {
        self.storage.len() * size_of::<u64>()
    }

    // Only call for fully pattern-initialized storage after a rejected call.
    // Successful struct writes may leave their C padding uninitialized.
    fn initialized_bytes(&self) -> Vec<u8> {
        unsafe { std::slice::from_raw_parts(self.storage.as_ptr().cast(), self.length()) }.to_vec()
    }

    fn check_tail(&self) {
        let tail = unsafe {
            std::slice::from_raw_parts(
                self.storage.as_ptr().cast::<u8>().add(size_of::<T>()),
                self.length() - size_of::<T>(),
            )
        };
        assert!(tail.iter().all(|byte| *byte == 0xa5));
    }
}

fn create_document() -> *mut OuiDocument {
    let config = OuiDocumentConfig {
        struct_size: size_of::<OuiDocumentConfig>() as u32,
        abi_version: OUI_ABI_VERSION,
        viewport: OuiViewportMetrics {
            logical_width: 16.0,
            logical_height: 16.0,
            physical_width: 16,
            physical_height: 16,
            device_scale_factor: 1.0,
            authority: 1,
            reserved: 0,
        },
    };
    let mut document = ptr::null_mut();
    assert_eq!(oui_document_create(&config, &mut document), OuiStatus::Ok);
    document
}

#[test]
fn rejected_versioned_queries_preserve_outputs_and_secondary_flags() {
    let document = create_document();
    for (size, query) in QUERIES.into_iter().skip(1) {
        let mut output = Output::<OuiFontFaceInfo>::new(true);
        // All storage is initialized; make its declared legacy extent match
        // the query. Bitmap also requires the pixels field initialized null.
        unsafe { ptr::write(output.pointer().cast::<u32>(), size as u32) };
        if size == size_of::<OuiBitmap>() {
            let bitmap = output.pointer().cast::<OuiBitmap>();
            unsafe { ptr::write(ptr::addr_of_mut!((*bitmap).pixels), ptr::null_mut()) };
        }
        let before = output.initialized_bytes();
        assert_ne!(
            query(ptr::null_mut(), output.pointer().cast()),
            OuiStatus::Ok
        );
        assert_eq!(output.initialized_bytes(), before);
    }
    let mut state = Output::<OuiAnimationState>::new(true);
    let before = state.initialized_bytes();
    assert_ne!(animation(document, state.pointer().cast()), OuiStatus::Ok);
    assert_eq!(state.initialized_bytes(), before);

    let mut bitmap = Output::<OuiBitmap>::new(true);
    unsafe {
        ptr::write(
            ptr::addr_of_mut!((*bitmap.pointer()).pixels),
            ptr::null_mut(),
        )
    };
    let before = bitmap.initialized_bytes();
    assert_eq!(oui_document_begin_update(document), OuiStatus::Ok);
    assert_eq!(
        oui_document_render_rgba(document, bitmap.pointer()),
        OuiStatus::InvalidState
    );
    assert_eq!(bitmap.initialized_bytes(), before);
    assert_eq!(oui_document_end_update(document), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
}

#[test]
fn versioned_outputs_accept_uninitialized_payloads_and_preserve_future_tails() {
    assert_eq!(
        oui_document_create(ptr::null(), ptr::null_mut()),
        OuiStatus::InvalidArgument
    );
    let mut error = Output::<OuiErrorInfo>::new(false);
    assert_eq!(oui_error_get_last(error.pointer()), OuiStatus::Ok);
    let info = unsafe { ptr::read(error.pointer()) };
    assert_eq!(info.struct_size as usize, error.length());
    assert_eq!(info.status, OuiStatus::InvalidArgument as i32);
    assert!(info.message_length > 0);
    error.check_tail();

    let document = create_document();
    let mut update = Output::<OuiAccessibilityUpdate>::new(false);
    assert_eq!(
        oui_document_accessibility_update(document, update.pointer()),
        OuiStatus::Ok
    );
    let info = unsafe { ptr::read(update.pointer()) };
    assert_eq!(info.struct_size as usize, update.length());
    assert_eq!(info.reserved, 0);
    update.check_tail();

    let mut bitmap = Output::<OuiBitmap>::new(false);
    // pixels is the only initialized payload input required by this API.
    unsafe {
        ptr::write(
            ptr::addr_of_mut!((*bitmap.pointer()).pixels),
            ptr::null_mut(),
        )
    };
    assert_eq!(
        oui_document_render_rgba(document, bitmap.pointer()),
        OuiStatus::Ok
    );
    let info = unsafe { ptr::read(bitmap.pointer()) };
    assert_eq!(info.struct_size as usize, bitmap.length());
    assert_eq!((info.width, info.height, info.stride), (16, 16, 64));
    assert_eq!(oui_buffer_length(info.pixels), 16 * 16 * 4);
    bitmap.check_tail();
    assert_eq!(oui_buffer_destroy(info.pixels), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
}

#[test]
fn empty_animation_queue_leaves_event_storage_untouched() {
    let document = create_document();
    let mut event = Output::<OuiAnimationEvent>::new(true);
    let before = event.initialized_bytes();
    let mut has_event = 0xa5;
    assert_eq!(
        oui_document_take_animation_event(document, event.pointer(), &mut has_event),
        OuiStatus::Ok
    );
    assert_eq!(has_event, 0);
    assert_eq!(event.initialized_bytes(), before);

    // The empty queue also accepts uninitialized event payload storage.
    let mut event = Output::<OuiAnimationEvent>::new(false);
    assert_eq!(
        oui_document_take_animation_event(document, event.pointer(), &mut has_event),
        OuiStatus::Ok
    );
    assert_eq!(has_event, 0);
    event.check_tail();
    assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
}
