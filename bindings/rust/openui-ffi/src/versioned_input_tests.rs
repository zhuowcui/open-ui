//! Storage and mutation guards for caller-owned versioned inputs.

use super::*;

fn config() -> OuiDocumentConfig {
    OuiDocumentConfig {
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
    }
}

fn easing() -> OuiEasing {
    OuiEasing {
        kind: 0,
        step_position: 0,
        step_count: 0,
        reserved: 0,
        values: [0.0; 4],
    }
}

fn options() -> OuiAnimationOptions {
    OuiAnimationOptions {
        struct_size: size_of::<OuiAnimationOptions>() as u32,
        abi_version: OUI_ABI_VERSION,
        delay_ms: 0.0,
        duration_ms: 100.0,
        iterations: 1.0,
        playback_rate: 1.0,
        direction: 0,
        fill: 3,
        play_state: 0,
        composite: 0,
        easing: easing(),
    }
}

struct Context {
    document: *mut OuiDocument,
    root: *mut OuiElement,
}

impl Context {
    fn new() -> Self {
        let mut document = ptr::null_mut();
        assert_eq!(oui_document_create(&config(), &mut document), OuiStatus::Ok);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        Self { document, root }
    }

    fn animate(
        &self,
        options: *const OuiAnimationOptions,
        timeline: *const OuiAnimationTimeline,
    ) -> OuiStatus {
        let frames = [0.0f32, 1.0].map(|value| OuiKeyframe {
            offset: f64::from(value),
            value: OuiStyleValue {
                tag: 2,
                reserved: 0,
                data: OuiStylePayload { number: value },
            },
            easing: easing(),
            has_easing: 0,
            reserved: 0,
        });
        let mut output = u64::MAX;
        let status = oui_element_animate(
            self.root,
            StyleProperty::Opacity as i32,
            frames.as_ptr(),
            frames.len(),
            options,
            timeline,
            &mut output,
        );
        // These calls are deliberately rejected before creating an animation.
        assert_eq!(output, u64::MAX);
        status
    }

    fn finish(self) {
        assert_eq!(oui_element_destroy(self.root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(self.document), OuiStatus::Ok);
    }
}

type Query = fn(&Context, *const c_void) -> OuiStatus;

fn app(_: &Context, input: *const c_void) -> OuiStatus {
    let mut output = ptr::dangling_mut::<OuiApp>();
    let before = output;
    let status = oui_app_create(input.cast(), &mut output);
    assert_eq!(output, before);
    status
}

fn document(_: &Context, input: *const c_void) -> OuiStatus {
    let mut output = ptr::dangling_mut::<OuiDocument>();
    let before = output;
    let status = oui_document_create(input.cast(), &mut output);
    assert_eq!(output, before);
    status
}

fn viewport(context: &Context, input: *const c_void) -> OuiStatus {
    oui_document_set_viewport(context.document, input.cast())
}

fn font(context: &Context, input: *const c_void) -> OuiStatus {
    let data = [0u8];
    let mut output = ptr::dangling_mut::<OuiFontFace>();
    let before = output;
    let status = oui_document_register_font(
        context.document,
        data.as_ptr(),
        data.len(),
        input.cast(),
        &mut output,
    );
    assert_eq!(output, before);
    status
}

fn animation_options(context: &Context, input: *const c_void) -> OuiStatus {
    context.animate(input.cast(), ptr::null())
}

fn animation_timeline(context: &Context, input: *const c_void) -> OuiStatus {
    context.animate(&options(), input.cast())
}

const QUERIES: [Query; 6] = [
    app,
    document,
    viewport,
    font,
    animation_options,
    animation_timeline,
];

#[test]
fn short_versioned_inputs_are_rejected_without_reading_payload_or_mutating_document() {
    let context = Context::new();
    for query in QUERIES {
        let size_only = Box::new(4u32);
        assert_eq!(
            query(&context, (&*size_only as *const u32).cast()),
            OuiStatus::InvalidArgument
        );
        assert_eq!(*size_only, 4);
        for (version, expected) in [
            (OUI_ABI_VERSION, OuiStatus::InvalidArgument),
            (OUI_ABI_VERSION ^ 1, OuiStatus::AbiMismatch),
        ] {
            let header = Box::new([8u32, version]);
            assert_eq!(query(&context, header.as_ptr().cast()), expected);
            assert_eq!(*header, [8, version]);
        }
    }
    let mut animating = 1;
    assert_eq!(
        oui_document_is_animating(context.document, &mut animating),
        OuiStatus::Ok
    );
    assert_eq!(animating, 0);
    let mut actual = std::mem::MaybeUninit::<OuiViewportMetrics>::uninit();
    assert_eq!(
        oui_document_get_viewport(context.document, actual.as_mut_ptr()),
        OuiStatus::Ok
    );
    let actual = unsafe { actual.assume_init() };
    assert_eq!(
        (
            actual.logical_width,
            actual.logical_height,
            actual.physical_width,
            actual.physical_height
        ),
        (16.0, 16.0, 16, 16)
    );
    context.finish();
}

#[repr(C)]
struct Extended<T> {
    base: T,
    tail: [u8; 16],
}

#[test]
fn valid_inputs_accept_future_extents_and_preserve_caller_storage() {
    let mut document_config = Extended {
        base: config(),
        tail: [0xa5; 16],
    };
    document_config.base.struct_size = size_of::<Extended<OuiDocumentConfig>>() as u32;
    let mut document = ptr::null_mut();
    assert_eq!(
        oui_document_create(&document_config.base, &mut document),
        OuiStatus::Ok
    );
    assert_eq!(document_config.tail, [0xa5; 16]);
    assert_eq!(
        document_config.base.struct_size as usize,
        size_of::<Extended<OuiDocumentConfig>>()
    );
    document_config.base.viewport.logical_width = 24.0;
    document_config.base.viewport.physical_width = 24;
    assert_eq!(
        oui_document_set_viewport(document, &document_config.base),
        OuiStatus::Ok
    );
    assert_eq!(document_config.tail, [0xa5; 16]);
    let mut actual = std::mem::MaybeUninit::<OuiViewportMetrics>::uninit();
    assert_eq!(
        oui_document_get_viewport(document, actual.as_mut_ptr()),
        OuiStatus::Ok
    );
    assert_eq!(unsafe { actual.assume_init() }.logical_width, 24.0);

    let mut animation_options = Extended {
        base: options(),
        tail: [0xa5; 16],
    };
    animation_options.base.struct_size = size_of::<Extended<OuiAnimationOptions>>() as u32;
    assert!(super::animation_options(&animation_options.base).is_ok());
    assert_eq!(animation_options.tail, [0xa5; 16]);
    let mut timeline = Extended {
        base: OuiAnimationTimeline {
            struct_size: size_of::<Extended<OuiAnimationTimeline>>() as u32,
            abi_version: OUI_ABI_VERSION,
            kind: 0,
            axis: 0,
            source: ptr::null_mut(),
            range_start: 0.0,
            range_end: 1.0,
        },
        tail: [0xa5; 16],
    };
    let state = super::document(document as usize).unwrap();
    assert!(matches!(
        super::animation_timeline(&state, &timeline.base),
        Ok(AnimationTimeline::Document)
    ));
    assert!(matches!(
        super::animation_timeline(&state, ptr::null()),
        Ok(AnimationTimeline::Document)
    ));
    assert_eq!(timeline.tail, [0xa5; 16]);
    // An ABI mismatch in a future-sized input leaves its values untouched.
    timeline.base.abi_version ^= 1;
    assert!(super::animation_timeline(&state, &timeline.base).is_err());
    assert_eq!(timeline.tail, [0xa5; 16]);
    assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
}
