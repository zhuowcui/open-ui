//! Native operations, ownership and prefix guards for additive curve inputs.
use super::*;
use std::mem::align_of;

fn curve(kind: u32, points: &[OuiLinearStopV1]) -> OuiEasingCurveV1 {
    OuiEasingCurveV1 {
        struct_size: size_of::<OuiEasingCurveV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        kind,
        step_position: 0,
        step_count: 0,
        reserved: 0,
        values: [0.0; 4],
        stops: points.as_ptr(),
        stop_count: points.len(),
        stop_stride: size_of::<OuiLinearStopV1>(),
    }
}

fn options(easing: *const OuiEasingCurveV1) -> OuiAnimationOptionsV1 {
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
        easing,
    }
}

fn frame(offset: f64, number: f32, easing: *const OuiEasingCurveV1) -> OuiKeyframeV1 {
    OuiKeyframeV1 {
        struct_size: size_of::<OuiKeyframeV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        offset,
        value: OuiStyleValue {
            tag: 2,
            reserved: 0,
            data: OuiStylePayload { number },
        },
        easing,
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
                        reserved: 0,
                    },
                },
                &mut document,
            ),
            OuiStatus::Ok
        );
        let mut root = ptr::null_mut();
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
    fn animate(
        &self,
        node: *mut OuiElement,
        frames: &[OuiKeyframeV1],
        options: &OuiAnimationOptionsV1,
    ) -> u64 {
        let mut id = u64::MAX;
        assert_eq!(
            oui_element_animate_v1(
                node,
                StyleProperty::Opacity as i32,
                frames.as_ptr(),
                frames.len(),
                size_of::<OuiKeyframeV1>(),
                options,
                ptr::null(),
                &mut id
            ),
            OuiStatus::Ok
        );
        assert_ne!(id, u64::MAX);
        id
    }
    fn opacity(&self, node: *mut OuiElement) -> f32 {
        let node = element(node as usize).unwrap();
        let state = element_document(&node).unwrap();
        let engine = borrow_engine(&state).unwrap();
        engine.computed_style(node.node).unwrap().opacity
    }
    fn active(&self) -> usize {
        let state = document(self.document as usize).unwrap();
        let engine = borrow_engine(&state).unwrap();
        engine.active_animation_count()
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

#[test]
fn additive_curve_layouts_preserve_legacy_records() {
    assert_eq!(
        (size_of::<OuiLinearStopV1>(), align_of::<OuiLinearStopV1>()),
        (16, 8)
    );
    assert_eq!(
        (
            size_of::<OuiEasingCurveV1>(),
            align_of::<OuiEasingCurveV1>()
        ),
        (80, 8)
    );
    assert_eq!(
        (
            size_of::<OuiAnimationOptionsV1>(),
            align_of::<OuiAnimationOptionsV1>()
        ),
        (64, 8)
    );
    assert_eq!(
        (size_of::<OuiKeyframeV1>(), align_of::<OuiKeyframeV1>()),
        (40, 8)
    );
    assert_eq!(
        (
            size_of::<OuiEasing>(),
            size_of::<OuiAnimationOptions>(),
            size_of::<OuiKeyframe>()
        ),
        (48, 104, 80)
    );
}

#[test]
fn authored_percent_points_preserve_29_discontinuity_and_outer_endpoints() {
    let points = [
        OuiLinearStopV1 {
            input: 0.0,
            output: 0.0,
        },
        OuiLinearStopV1 {
            input: 29.0,
            output: 1.0,
        },
        OuiLinearStopV1 {
            input: 29.0,
            output: 2.0,
        },
        OuiLinearStopV1 {
            input: 100.0,
            output: 3.0,
        },
    ];
    let decoded = easing_curve::curve(&curve(4, &points)).unwrap();
    let Easing::LinearStopsPercent(stops) = &decoded else {
        panic!("authored percent variant was lost")
    };
    assert_eq!(stops[1].input_percent.to_bits(), points[1].input.to_bits());
    assert_eq!(stops.len(), 4);
    let progress = 29.0_f64 / 100.0;
    assert!(100.0 * progress < 29.0);
    assert_eq!(decoded.sample(progress), 1.0);
    assert_eq!(decoded.sample(f64::from_bits(progress.to_bits() + 1)), 2.0);
    let outer = [
        OuiLinearStopV1 {
            input: -100.0,
            output: 0.0,
        },
        OuiLinearStopV1 {
            input: 200.0,
            output: 1.0,
        },
    ];
    let decoded = easing_curve::curve(&curve(4, &outer)).unwrap();
    assert_eq!(decoded.sample(0.0), 1.0 / 3.0);
    assert_eq!(decoded.sample(1.0), 2.0 / 3.0);
    assert!(easing_curve::curve(&curve(3, &outer)).is_err());
}

#[test]
fn c_effect_and_segment_curves_compose_and_own_their_input_copies() {
    let mut context = Context::new();
    let first = context.node(context.root);
    let second = context.node(context.root);
    let mut effect_points = [
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
    let mut segment_points = [
        OuiLinearStopV1 {
            input: 0.0,
            output: 0.0,
        },
        OuiLinearStopV1 {
            input: 0.25,
            output: 0.5,
        },
        OuiLinearStopV1 {
            input: 1.0,
            output: 1.0,
        },
    ];
    let effect = curve(3, &effect_points);
    let segment = curve(3, &segment_points);
    let timing = options(&effect);
    let mut frames = [frame(0.0, 0.0, &segment), frame(1.0, 1.0, ptr::null())];
    context.animate(first, &frames, &timing);
    context.animate(
        second,
        &[frame(0.0, 0.0, ptr::null()), frame(1.0, 1.0, ptr::null())],
        &timing,
    );
    effect_points[1].output = 0.9;
    segment_points[1].output = 0.1;
    frames[1].value.data.number = 0.0;
    let mut native =
        Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap();
    let native_node = native.create_native_element(ElementTag::Div).unwrap();
    native.append_child(native.root(), native_node).unwrap();
    native
        .animate(
            native_node,
            PropertyKeyframes::typed(
                StyleProperty::Opacity,
                Keyframes::new(vec![
                    Keyframe::new(0.0, 0.0f32).easing(
                        Easing::linear_stops(vec![
                            LinearStop {
                                input: 0.0,
                                output: 0.0,
                            },
                            LinearStop {
                                input: 0.25,
                                output: 0.5,
                            },
                            LinearStop {
                                input: 1.0,
                                output: 1.0,
                            },
                        ])
                        .unwrap(),
                    ),
                    Keyframe::new(1.0, 1.0f32),
                ])
                .unwrap(),
            )
            .unwrap(),
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                easing: Easing::linear_stops(vec![
                    LinearStop {
                        input: 0.0,
                        output: 0.0,
                    },
                    LinearStop {
                        input: 0.5,
                        output: 0.25,
                    },
                    LinearStop {
                        input: 1.0,
                        output: 1.0,
                    },
                ])
                .unwrap(),
                ..Default::default()
            },
            AnimationTimeline::Document,
        )
        .unwrap();
    native.set_animation_time(50.0).unwrap();
    context.time(50.0);
    // Effect .5 -> .25, then segment .25 -> .5; a default linear segment -> .25.
    assert_eq!(context.opacity(first), 0.5);
    assert_eq!(
        context.opacity(first),
        native.computed_style(native_node).unwrap().opacity
    );
    assert_eq!(context.opacity(second), 0.25);
    assert_eq!(effect_points[1].output, 0.9);
    assert_eq!(segment_points[1].output, 0.1);
}

#[test]
fn future_prefix_tails_and_strided_keyframes_and_points_are_ignored() {
    #[repr(C)]
    struct Point {
        prefix: OuiLinearStopV1,
        tail: u64,
    }
    #[repr(C)]
    struct Curve {
        prefix: OuiEasingCurveV1,
        tail: u64,
    }
    #[repr(C)]
    struct Timing {
        prefix: OuiAnimationOptionsV1,
        tail: u64,
    }
    #[repr(C)]
    struct Frame {
        prefix: OuiKeyframeV1,
        tail: u64,
    }
    let points = [
        Point {
            prefix: OuiLinearStopV1 {
                input: 0.0,
                output: 0.0,
            },
            tail: 7,
        },
        Point {
            prefix: OuiLinearStopV1 {
                input: 0.5,
                output: 0.25,
            },
            tail: 8,
        },
        Point {
            prefix: OuiLinearStopV1 {
                input: 1.0,
                output: 1.0,
            },
            tail: 9,
        },
    ];
    let mut curve = Curve {
        prefix: curve(3, &[]),
        tail: 11,
    };
    curve.prefix.struct_size = size_of::<Curve>() as u32;
    curve.prefix.stops = points.as_ptr().cast();
    curve.prefix.stop_count = points.len();
    curve.prefix.stop_stride = size_of::<Point>();
    let mut timing = Timing {
        prefix: options(&curve.prefix),
        tail: 12,
    };
    timing.prefix.struct_size = size_of::<Timing>() as u32;
    let mut frames = [
        Frame {
            prefix: frame(0.0, 0.0, ptr::null()),
            tail: 13,
        },
        Frame {
            prefix: frame(1.0, 1.0, ptr::null()),
            tail: 14,
        },
    ];
    for frame in &mut frames {
        frame.prefix.struct_size = size_of::<Frame>() as u32;
    }
    let mut context = Context::new();
    let node = context.node(context.root);
    let mut id = u64::MAX;
    assert_eq!(
        oui_element_animate_v1(
            node,
            StyleProperty::Opacity as i32,
            frames.as_ptr().cast(),
            frames.len(),
            size_of::<Frame>(),
            &timing.prefix,
            ptr::null(),
            &mut id
        ),
        OuiStatus::Ok
    );
    context.time(50.0);
    assert_eq!(context.opacity(node), 0.25);
    assert_eq!(
        (points[1].tail, curve.tail, timing.tail, frames[1].tail),
        (8, 11, 12, 14)
    );
}

#[test]
fn short_headers_and_bad_arrays_leave_native_state_and_outputs_unchanged() {
    let mut context = Context::new();
    let node = context.node(context.root);
    let frames = [frame(0.0, 0.0, ptr::null()), frame(1.0, 1.0, ptr::null())];
    let timing = options(ptr::null());
    let mut id = u64::MAX;
    let size_only = Box::new(4u32);
    let short = (&*size_only as *const u32).cast();
    assert_eq!(
        oui_element_animate_v1(
            node,
            StyleProperty::Opacity as i32,
            frames.as_ptr(),
            2,
            size_of::<OuiKeyframeV1>(),
            short,
            ptr::null(),
            &mut id
        ),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_element_smooth_scroll_to_v1(
            node,
            0.0,
            0.0,
            100.0,
            (&*size_only as *const u32).cast(),
            &mut id
        ),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_element_animate_v1(
            node,
            StyleProperty::Opacity as i32,
            (&*size_only as *const u32).cast(),
            1,
            size_of::<OuiKeyframeV1>(),
            &timing,
            ptr::null(),
            &mut id
        ),
        OuiStatus::InvalidArgument
    );
    assert_eq!(*size_only, 4);
    for header in [
        [4, OUI_ABI_VERSION],
        [8, OUI_ABI_VERSION],
        [8, OUI_ABI_VERSION ^ 1],
    ] {
        let expected = if header[0] >= 8 && header[1] != OUI_ABI_VERSION {
            OuiStatus::AbiMismatch
        } else {
            OuiStatus::InvalidArgument
        };
        assert_eq!(
            oui_element_animate_v1(
                node,
                StyleProperty::Opacity as i32,
                frames.as_ptr(),
                2,
                size_of::<OuiKeyframeV1>(),
                header.as_ptr().cast(),
                ptr::null(),
                &mut id
            ),
            expected
        );
        assert_eq!(
            oui_element_smooth_scroll_to_v1(node, 0.0, 0.0, 100.0, header.as_ptr().cast(), &mut id),
            expected
        );
        assert_eq!(
            oui_element_animate_v1(
                node,
                StyleProperty::Opacity as i32,
                header.as_ptr().cast(),
                1,
                size_of::<OuiKeyframeV1>(),
                &timing,
                ptr::null(),
                &mut id
            ),
            expected
        );
        assert_eq!(id, u64::MAX);
    }
    let points = [
        OuiLinearStopV1 {
            input: 0.0,
            output: 0.0,
        },
        OuiLinearStopV1 {
            input: 1.0,
            output: 1.0,
        },
    ];
    for (count, stride, pointer) in [
        (1, size_of::<OuiLinearStopV1>(), points.as_ptr()),
        (2, 1, points.as_ptr()),
        (usize::MAX, size_of::<OuiLinearStopV1>(), points.as_ptr()),
        (2, size_of::<OuiLinearStopV1>(), ptr::null()),
        (
            2,
            size_of::<OuiLinearStopV1>(),
            usize::MAX as *const OuiLinearStopV1,
        ),
    ] {
        let mut raw = curve(3, &points);
        raw.stop_count = count;
        raw.stop_stride = stride;
        raw.stops = pointer;
        assert_eq!(
            oui_element_smooth_scroll_to_v1(node, 0.0, 0.0, 100.0, &raw, &mut id),
            OuiStatus::InvalidArgument
        );
        assert_eq!(id, u64::MAX);
    }
    for (count, stride) in [
        (usize::MAX, size_of::<OuiKeyframeV1>()),
        (2, 1),
        (0, size_of::<OuiKeyframeV1>()),
    ] {
        assert_eq!(
            oui_element_animate_v1(
                node,
                StyleProperty::Opacity as i32,
                frames.as_ptr(),
                count,
                stride,
                &timing,
                ptr::null(),
                &mut id
            ),
            OuiStatus::InvalidArgument
        );
    }
    assert_eq!(context.active(), 0);
    assert!(
        !borrow_engine(&document(context.document as usize).unwrap())
            .unwrap()
            .is_animating()
    );
    assert_eq!(id, u64::MAX);
}

#[test]
fn unknown_reserved_unordered_and_nonfinite_curves_are_rejected() {
    let points = [
        OuiLinearStopV1 {
            input: 0.0,
            output: 0.0,
        },
        OuiLinearStopV1 {
            input: 1.0,
            output: 1.0,
        },
    ];
    let mut bad = curve(3, &points);
    for raw in [
        {
            bad.kind = 5;
            bad
        },
        {
            bad.kind = 3;
            bad.reserved = 1;
            bad
        },
        {
            bad.reserved = 0;
            bad.values[0] = 1.0;
            bad
        },
        {
            bad.values[0] = 0.0;
            bad.step_count = 1;
            bad
        },
    ] {
        assert!(easing_curve::curve(&raw).is_err());
    }
    for points in [
        [
            OuiLinearStopV1 {
                input: 1.0,
                output: 0.0,
            },
            OuiLinearStopV1 {
                input: 0.0,
                output: 1.0,
            },
        ],
        [
            OuiLinearStopV1 {
                input: 0.0,
                output: f64::NAN,
            },
            OuiLinearStopV1 {
                input: 1.0,
                output: 1.0,
            },
        ],
    ] {
        assert!(easing_curve::curve(&curve(4, &points)).is_err());
    }
}

#[test]
fn stale_wrong_thread_and_foreign_timeline_fail_before_mutation() {
    let mut context = Context::new();
    let node = context.node(context.root);
    let address = node as usize;
    assert_eq!(
        std::thread::spawn(move || {
            let mut output = u64::MAX;
            let status = oui_element_smooth_scroll_to_v1(
                address as *mut OuiElement,
                0.0,
                0.0,
                100.0,
                ptr::dangling::<OuiEasingCurveV1>(),
                &mut output,
            );
            assert_eq!(output, u64::MAX);
            status
        })
        .join()
        .unwrap(),
        OuiStatus::WrongThread
    );
    let other = Context::new();
    let timeline = OuiAnimationTimeline {
        struct_size: size_of::<OuiAnimationTimeline>() as u32,
        abi_version: OUI_ABI_VERSION,
        kind: 1,
        axis: 2,
        source: other.root,
        range_start: 0.0,
        range_end: 1.0,
    };
    let frames = [frame(0.0, 0.0, ptr::null()), frame(1.0, 1.0, ptr::null())];
    let mut output = u64::MAX;
    assert_eq!(
        oui_element_animate_v1(
            node,
            StyleProperty::Opacity as i32,
            frames.as_ptr(),
            2,
            size_of::<OuiKeyframeV1>(),
            &options(ptr::null()),
            &timeline,
            &mut output
        ),
        OuiStatus::WrongDocument
    );
    assert_eq!(context.active(), 0);
    assert_eq!(oui_element_destroy(node), OuiStatus::Ok);
    context.nodes.clear();
    assert_eq!(
        oui_element_smooth_scroll_to_v1(
            node,
            0.0,
            0.0,
            100.0,
            ptr::dangling::<OuiEasingCurveV1>(),
            &mut output
        ),
        OuiStatus::InvalidHandle
    );
    assert_eq!(output, u64::MAX);
}

fn set_literal(node: *mut OuiElement, property: StyleProperty, literal: &str) {
    let mut value = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
    assert_eq!(
        oui_style_value_parse(
            property as i32,
            OuiUtf8 {
                data: literal.as_ptr(),
                length: literal.len()
            },
            value.as_mut_ptr()
        ),
        OuiStatus::Ok
    );
    // SAFETY: successful parse initialized one tagged value.
    let value = unsafe { value.assume_init() };
    assert_eq!(
        oui_element_set_property(node, property as i32, &value),
        OuiStatus::Ok
    );
    if value.tag == 6 {
        assert_eq!(
            oui_style_compound_destroy(unsafe { value.data.compound }.cast_mut().cast()),
            OuiStatus::Ok
        );
    }
}
fn offset(node: *mut OuiElement) -> (f64, f64) {
    let (mut x, mut y) = (f64::NAN, f64::NAN);
    assert_eq!(
        oui_element_get_scroll_offset(node, &mut x, &mut y),
        OuiStatus::Ok
    );
    (x, y)
}

#[test]
fn c_scroll_and_reveal_use_the_shared_clock_and_copied_percent_curve() {
    let mut context = Context::new();
    let port = context.node(context.root);
    for (property, value) in [
        (StyleProperty::Width, "100px"),
        (StyleProperty::Height, "80px"),
        (StyleProperty::Position, "relative"),
        (StyleProperty::Overflow, "hidden"),
        (StyleProperty::ScrollbarWidth, "none"),
    ] {
        set_literal(port, property, value);
    }
    let content = context.node(port);
    set_literal(content, StyleProperty::Width, "300px");
    set_literal(content, StyleProperty::Height, "240px");
    let target = context.node(port);
    for (property, value) in [
        (StyleProperty::Width, "20px"),
        (StyleProperty::Height, "20px"),
        (StyleProperty::Position, "absolute"),
        (StyleProperty::Left, "150px"),
        (StyleProperty::Top, "120px"),
    ] {
        set_literal(target, property, value);
    }
    let mut points = [
        OuiLinearStopV1 {
            input: 0.0,
            output: 0.0,
        },
        OuiLinearStopV1 {
            input: 29.0,
            output: 0.25,
        },
        OuiLinearStopV1 {
            input: 29.0,
            output: 0.75,
        },
        OuiLinearStopV1 {
            input: 100.0,
            output: 1.0,
        },
    ];
    let raw = curve(4, &points);
    let mut id = u64::MAX;
    assert_eq!(
        oui_element_smooth_scroll_to_v1(port, 100.0, 80.0, 100.0, &raw, &mut id),
        OuiStatus::Ok
    );
    points[1].output = 0.0;
    context.time(29.0);
    assert_eq!(offset(port), (25.0, 20.0));
    context.time(100.0);
    assert_eq!(offset(port), (100.0, 80.0));
    assert_eq!(oui_element_scroll_to(port, 0.0, 0.0), OuiStatus::Ok);
    assert_eq!(
        oui_element_smooth_scroll_into_view_v2(target, 0, 0, 1, 100.0, ptr::null()),
        OuiStatus::Ok
    );
    context.time(150.0);
    assert_eq!(offset(port), (75.0, 60.0));
    context.time(200.0);
    assert_eq!(offset(port), (150.0, 120.0));
}

#[test]
fn transition_uses_arbitrary_curve_points_on_the_native_clock() {
    let mut context = Context::new();
    let node = context.node(context.root);
    set_literal(node, StyleProperty::Opacity, "0");
    let points = [
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
    let raw = curve(3, &points);
    let target = frame(1.0, 1.0, ptr::null()).value;
    let mut id = u64::MAX;
    assert_eq!(
        oui_element_transition_v1(
            node,
            StyleProperty::Opacity as i32,
            &target,
            &options(&raw),
            &mut id
        ),
        OuiStatus::Ok
    );
    context.time(50.0);
    assert_eq!(context.opacity(node), 0.25);
}

#[test]
fn animation_callbacks_can_reenter_after_curve_inputs_and_engine_borrows_end() {
    struct Callback {
        document: *mut OuiDocument,
        count: u32,
    }
    extern "C" fn started(_: *mut OuiEvent, data: *mut c_void) {
        // SAFETY: this native test owns the callback data until listener destroy.
        let data = unsafe { &mut *data.cast::<Callback>() };
        let mut running = 0;
        assert_eq!(
            oui_document_is_animating(data.document, &mut running),
            OuiStatus::Ok
        );
        assert_eq!(running, 1);
        data.count += 1;
    }
    let mut context = Context::new();
    let node = context.node(context.root);
    let mut data = Callback {
        document: context.document,
        count: 0,
    };
    let mut listener = ptr::null_mut();
    assert_eq!(
        oui_element_add_event_listener(
            node,
            20,
            0,
            Some(started),
            (&mut data as *mut Callback).cast(),
            &mut listener
        ),
        OuiStatus::Ok
    );
    context.animate(
        node,
        &[frame(0.0, 0.0, ptr::null()), frame(1.0, 1.0, ptr::null())],
        &options(ptr::null()),
    );
    assert_eq!(data.count, 1);
    assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
}

#[test]
fn arbitrary_stop_arrays_are_not_limited_to_the_legacy_array_cap() {
    let points = (0..4097)
        .map(|index| OuiLinearStopV1 {
            input: index as f64 / 4096.0,
            output: index as f64 / 4096.0,
        })
        .collect::<Vec<_>>();
    let decoded = easing_curve::curve(&curve(3, &points)).unwrap();
    let Easing::LinearStops(stops) = &decoded else {
        panic!("normalized variant was lost")
    };
    assert_eq!(stops.len(), points.len());
    assert_eq!(decoded.sample(0.5), 0.5);
}

#[test]
fn descriptor_alignment_reentry_and_future_entry_extent_fail_without_mutation() {
    let mut context = Context::new();
    let node = context.node(context.root);
    let mut words = [0u64; 11];
    let unaligned = unsafe { words.as_mut_ptr().cast::<u8>().add(1) };
    // SAFETY: the test allocation has room for these two unaligned header words.
    unsafe {
        ptr::write_unaligned(
            unaligned.cast::<u32>(),
            size_of::<OuiEasingCurveV1>() as u32,
        );
        ptr::write_unaligned(unaligned.add(4).cast::<u32>(), OUI_ABI_VERSION);
    }
    let mut output = u64::MAX;
    assert_eq!(
        oui_element_smooth_scroll_to_v1(node, 0.0, 0.0, 100.0, unaligned.cast(), &mut output),
        OuiStatus::InvalidArgument
    );
    let mut frames = [frame(0.0, 0.0, ptr::null()), frame(1.0, 1.0, ptr::null())];
    frames[0].struct_size += 8;
    assert_eq!(
        oui_element_animate_v1(
            node,
            StyleProperty::Opacity as i32,
            frames.as_ptr(),
            2,
            size_of::<OuiKeyframeV1>(),
            &options(ptr::null()),
            ptr::null(),
            &mut output
        ),
        OuiStatus::InvalidArgument
    );
    let state = document(context.document as usize).unwrap();
    let engine = borrow_engine(&state).unwrap();
    assert_eq!(
        oui_element_smooth_scroll_to_v1(node, 0.0, 0.0, 100.0, ptr::null(), &mut output),
        OuiStatus::Reentrant
    );
    assert!(!engine.is_animating());
    drop(engine);
    assert_eq!(output, u64::MAX);
}

#[test]
fn scalar_curve_kinds_use_the_same_checked_native_constructors() {
    let mut raw = OuiEasingCurveV1 {
        struct_size: size_of::<OuiEasingCurveV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        kind: 0,
        step_position: 0,
        step_count: 0,
        reserved: 0,
        values: [0.0; 4],
        stops: ptr::null(),
        stop_count: 0,
        stop_stride: 0,
    };
    assert_eq!(easing_curve::curve(&raw).unwrap(), Easing::Linear);
    raw.kind = 1;
    raw.values = [0.25, 0.1, 0.25, 1.0];
    assert_eq!(
        easing_curve::curve(&raw).unwrap(),
        Easing::cubic_bezier(0.25, 0.1, 0.25, 1.0).unwrap()
    );
    raw.values[0] = -0.1;
    assert!(easing_curve::curve(&raw).is_err());
    raw.kind = 2;
    raw.values = [0.0; 4];
    raw.step_position = 1;
    raw.step_count = 4;
    assert_eq!(
        easing_curve::curve(&raw).unwrap(),
        Easing::steps(4, StepPosition::JumpEnd).unwrap()
    );
    raw.step_position = 2;
    raw.step_count = 1;
    assert!(easing_curve::curve(&raw).is_err());
}
