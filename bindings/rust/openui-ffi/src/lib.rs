//! Panic-contained, generation-checked Open UI v0.2 C ABI.

#![deny(unsafe_op_in_unsafe_fn)]
// C callers cannot express Rust's `unsafe` call-site marker. Every exported
// boundary validates nulls and metadata before dereferencing, and documents
// the remaining readable/writable-memory preconditions in its safety contract.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod accessibility_snapshot;
mod generated;
mod native_app;
#[cfg(test)]
mod native_app_tests;
mod registry;
mod types;
mod value;

pub use native_app::{oui_app_request_exit, oui_app_run};
pub use types::*;

use generated::{property_from_raw, valid_event_type};
use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{
    AccessibilityAction, AccessibilityLive, AccessibilityRelation, AccessibilityRole,
    AnimationEventKind, AnimationId, AnimationTimeline, ControlAdjustment, Engine,
    EventPhase as EngineEventPhase, FocusOrigin, FontAxisRange, FontContainerFormat,
    FontFaceDescriptor, FontFeatureDefault, FontMetricOverrides, FontStyleRange, FontUnicodeRange,
    NodeHandle, PointerEventKind, ViewportAuthority, ViewportMetrics,
};
use openui_style::{
    parse_literal, AnimationOptions, AnimationPhase, Border, BorderStyle, Color,
    CompositeOperation, CornerRadii, Easing, Edges, FillMode, FontFamilyList, Gap,
    GenericFontFamily, IterationCount, Keyframe, Keyframes, LengthValue, LinearStop, PlayState,
    PlaybackDirection, PropertyKeyframes, StepPosition, StyleProperty, StyleValue, TimelineAxis,
    TimelineRange, Transform2D, TransformList, TransformOperation,
};
use registry::{
    borrow_engine, borrow_engine_mut, bytes, destroy, document, element, element_document, ffi,
    ffi_preserve, ffi_value, get, last_error, register, utf8, AccessibilitySnapshotState, ApiError,
    AppState, DocumentState, ElementRef, FontFaceRef, HandleKind, ListenerRecord, ListenerRef,
    LocalHandle, ResourceRef,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;
use std::rc::Rc;
use std::sync::Arc;

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new(OuiStatus::InvalidArgument, message)
}

fn check_header(actual_size: u32, actual_abi: u32, required: usize) -> Result<(), ApiError> {
    if actual_abi != OUI_ABI_VERSION {
        return Err(ApiError::new(
            OuiStatus::AbiMismatch,
            format!("ABI version {actual_abi:#010x} does not match {OUI_ABI_VERSION:#010x}"),
        ));
    }
    if (actual_size as usize) < required {
        return Err(invalid(format!(
            "struct_size {actual_size} is smaller than required {required}"
        )));
    }
    Ok(())
}

fn write_handle<T>(out: *mut *mut T, handle: LocalHandle) -> Result<(), ApiError> {
    if out.is_null() {
        return Err(invalid("output handle pointer is null"));
    }
    let token = register(handle)?;
    // SAFETY: the C contract requires `out` to point to writable storage for
    // one handle pointer. It was checked for null, and the token is opaque.
    unsafe { ptr::write(out, token as *mut T) };
    Ok(())
}

fn write_element_handle(
    out: *mut *mut OuiElement,
    state: &Rc<DocumentState>,
    node: NodeHandle,
) -> Result<(), ApiError> {
    if out.is_null() {
        return Err(invalid("output element pointer is null"));
    }
    let token = register(LocalHandle::Element(ElementRef {
        document: Rc::downgrade(state),
        node,
    }))?;
    state
        .element_handles
        .try_borrow_mut()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "element handle map is borrowed"))?
        .entry(node)
        .or_default()
        .push(token);
    // SAFETY: the caller guarantees writable storage for one opaque pointer.
    unsafe { ptr::write(out, token as *mut OuiElement) };
    Ok(())
}

fn element_address(state: &DocumentState, node: NodeHandle) -> Result<usize, ApiError> {
    state
        .element_handles
        .try_borrow()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "element handle map is borrowed"))?
        .get(&node)
        .and_then(|handles| handles.last())
        .copied()
        .ok_or_else(|| ApiError::new(OuiStatus::InvalidHandle, "node has no live C handle"))
}

fn new_document(config: &OuiDocumentConfig) -> Result<Rc<DocumentState>, ApiError> {
    check_header(
        config.struct_size,
        config.abi_version,
        size_of::<OuiDocumentConfig>(),
    )?;
    let viewport = viewport_metrics(&config.viewport)?;
    let engine = Rc::new(RefCell::new(Engine::new(viewport)?));
    let state = Rc::new(DocumentState {
        native: openui::Document::from_shared_engine(engine.clone()),
        engine,
        update_depth: Cell::new(0),
        listeners: RefCell::new(Vec::new()),
        next_listener: Cell::new(1),
        element_handles: RefCell::new(std::collections::HashMap::new()),
        animation_events: RefCell::new(Vec::new()),
    });
    let weak = Rc::downgrade(&state);
    state
        .native
        .set_foreign_event_handler(Rc::new(move |node, target, event, capture| {
            if let Some(state) = weak.upgrade() {
                native_app::dispatch_native_listener(&state, node, target, event, capture)
                    .map_err(|error| openui::Error::Platform(error.message))?;
            }
            Ok(())
        }))
        .map_err(native_app::native_error)?;
    Ok(state)
}

fn viewport_metrics(raw: &OuiViewportMetrics) -> Result<ViewportMetrics, ApiError> {
    if raw.reserved != 0 {
        return Err(invalid("viewport reserved field must be zero"));
    }
    let metrics = match raw.authority {
        1 => ViewportMetrics::from_logical_size(
            raw.logical_width,
            raw.logical_height,
            raw.device_scale_factor,
        ),
        2 => ViewportMetrics::from_physical_size(
            raw.physical_width,
            raw.physical_height,
            raw.device_scale_factor,
        ),
        _ => return Err(invalid("viewport authority is invalid")),
    }
    .map_err(|error| invalid(error.to_string()))?;
    if raw.authority == 1
        && (raw.physical_width != 0 || raw.physical_height != 0)
        && metrics.physical_size() != (raw.physical_width, raw.physical_height)
    {
        return Err(invalid(
            "logical viewport has inconsistent physical dimensions",
        ));
    }
    if raw.authority == 2
        && ((raw.logical_width != 0.0
            && raw.logical_width.to_bits() != metrics.logical_width().to_bits())
            || (raw.logical_height != 0.0
                && raw.logical_height.to_bits() != metrics.logical_height().to_bits()))
    {
        return Err(invalid(
            "physical viewport has inconsistent logical dimensions",
        ));
    }
    Ok(metrics)
}

fn element_tag(value: i32) -> Result<ElementTag, ApiError> {
    let tag = match value {
        0 => ElementTag::Div,
        1 => ElementTag::Span,
        2 => ElementTag::Break,
        3 => ElementTag::WordBreak,
        4 => ElementTag::Ruby,
        5 => ElementTag::RubyText,
        6 => ElementTag::Table,
        7 => ElementTag::TableCaption,
        8 => ElementTag::TableColumnGroup,
        9 => ElementTag::TableColumn,
        10 => ElementTag::TableHead,
        11 => ElementTag::TableBody,
        12 => ElementTag::TableFoot,
        13 => ElementTag::TableRow,
        14 => ElementTag::TableCell,
        15 => ElementTag::TableHeaderCell,
        16 => ElementTag::Image,
        17 => ElementTag::Canvas,
        18 => ElementTag::Svg,
        19 => ElementTag::IFrame,
        20 => ElementTag::Object,
        21 => ElementTag::Audio,
        22 => ElementTag::Video,
        23 => ElementTag::Input,
        24 => ElementTag::Button,
        25 => ElementTag::Meter,
        26 => ElementTag::Progress,
        27 => ElementTag::Fieldset,
        28 => ElementTag::Legend,
        29 => ElementTag::Details,
        30 => ElementTag::Summary,
        31 => ElementTag::TextArea,
        32 => ElementTag::Select,
        33 => ElementTag::Option,
        34 => ElementTag::OptGroup,
        35 => ElementTag::Form,
        36 => ElementTag::Embed,
        37 => ElementTag::Html,
        38 => ElementTag::Body,
        _ => return Err(invalid("unknown element tag")),
    };
    Ok(tag)
}

fn same_document(first: &ElementRef, second: &ElementRef) -> Result<Rc<DocumentState>, ApiError> {
    let first_document = element_document(first)?;
    let second_document = element_document(second)?;
    if !Rc::ptr_eq(&first_document, &second_document) {
        return Err(ApiError::new(
            OuiStatus::WrongDocument,
            "elements belong to different documents",
        ));
    }
    Ok(first_document)
}

fn with_element_mut(
    address: usize,
    operation: impl FnOnce(&mut Engine, NodeHandle) -> Result<(), openui_engine::EngineError>,
) -> Result<(), ApiError> {
    let element = element(address)?;
    let document = element_document(&element)?;
    let mut engine = borrow_engine_mut(&document)?;
    operation(&mut engine, element.node)?;
    Ok(())
}

fn copy_array<T: Copy>(
    data: *const T,
    length: usize,
    label: &'static str,
) -> Result<Vec<T>, ApiError> {
    if length == 0 {
        return Ok(Vec::new());
    }
    if data.is_null()
        || length > isize::MAX as usize / std::mem::size_of::<T>().max(1)
        || length > 4096
    {
        return Err(invalid(format!("invalid {label} array")));
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(length)
        .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "array allocation failed"))?;
    // SAFETY: the C contract requires `data` to reference `length` readable,
    // aligned `T` values for the call. Bounds and null were validated above.
    result.extend_from_slice(unsafe { std::slice::from_raw_parts(data, length) });
    Ok(result)
}

fn font_descriptor(raw: *const OuiFontFaceDescriptor) -> Result<FontFaceDescriptor, ApiError> {
    if raw.is_null() {
        return Err(invalid("font descriptor is null"));
    }
    // SAFETY: the C contract requires a readable descriptor structure.
    let raw = unsafe { &*raw };
    check_header(
        raw.struct_size,
        raw.abi_version,
        size_of::<OuiFontFaceDescriptor>(),
    )?;
    let known_flags = OUI_FONT_FACE_HAS_SIZE_ADJUST
        | OUI_FONT_FACE_HAS_ASCENT_OVERRIDE
        | OUI_FONT_FACE_HAS_DESCENT_OVERRIDE
        | OUI_FONT_FACE_HAS_LINE_GAP_OVERRIDE;
    if raw.reserved != 0 || raw.flags & !known_flags != 0 {
        return Err(invalid("font descriptor contains reserved values"));
    }
    let family = utf8(raw.family, "font family")?;
    let unicode_ranges = copy_array(
        raw.unicode_ranges,
        raw.unicode_range_count,
        "font Unicode range",
    )?
    .into_iter()
    .map(|range| FontUnicodeRange {
        start: range.start,
        end: range.end,
    })
    .collect();
    let feature_defaults = copy_array(
        raw.feature_defaults,
        raw.feature_default_count,
        "font feature default",
    )?
    .into_iter()
    .map(|feature| FontFeatureDefault {
        tag: feature.tag,
        value: feature.value,
    })
    .collect();
    let style = match raw.style {
        OUI_FONT_FACE_STYLE_NORMAL => FontStyleRange::Normal,
        OUI_FONT_FACE_STYLE_ITALIC => FontStyleRange::Italic,
        OUI_FONT_FACE_STYLE_OBLIQUE => {
            FontStyleRange::Oblique(FontAxisRange::new(raw.style_min, raw.style_max))
        }
        _ => return Err(invalid("font style range is invalid")),
    };
    let optional = |flag: u32, value: f32| (raw.flags & flag != 0).then_some(value);
    Ok(FontFaceDescriptor {
        family,
        face_index: raw.face_index,
        style,
        weight: FontAxisRange::new(raw.weight_min, raw.weight_max),
        stretch: FontAxisRange::new(raw.stretch_min, raw.stretch_max),
        unicode_ranges,
        feature_defaults,
        size_adjust: optional(OUI_FONT_FACE_HAS_SIZE_ADJUST, raw.size_adjust),
        metric_overrides: FontMetricOverrides {
            ascent: optional(OUI_FONT_FACE_HAS_ASCENT_OVERRIDE, raw.ascent_override),
            descent: optional(OUI_FONT_FACE_HAS_DESCENT_OVERRIDE, raw.descent_override),
            line_gap: optional(OUI_FONT_FACE_HAS_LINE_GAP_OVERRIDE, raw.line_gap_override),
        },
    })
}

fn font_face_ref(address: usize) -> Result<FontFaceRef, ApiError> {
    match get(address, HandleKind::FontFace)? {
        LocalHandle::FontFace(value) => Ok(value),
        _ => unreachable!("kind checked by registry"),
    }
}

fn font_face_document(face: &FontFaceRef) -> Result<Rc<DocumentState>, ApiError> {
    face.document
        .upgrade()
        .ok_or_else(|| ApiError::new(OuiStatus::InvalidHandle, "font document was destroyed"))
}

fn copy_bytes_to_c(
    value: &[u8],
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
    label: &'static str,
) -> Result<(), ApiError> {
    if out_length.is_null() {
        return Err(invalid(format!("{label} length output is null")));
    }
    // SAFETY: caller supplies writable storage for one size_t.
    unsafe { ptr::write(out_length, value.len()) };
    if destination.is_null() && capacity == 0 {
        return Ok(());
    }
    if capacity < value.len() {
        return Err(ApiError::new(
            OuiStatus::BufferTooSmall,
            format!("{label} destination is too small"),
        ));
    }
    if !value.is_empty() {
        if destination.is_null() {
            return Err(invalid(format!("{label} destination is null")));
        }
        // SAFETY: capacity and null were validated and buffers do not overlap.
        unsafe { ptr::copy_nonoverlapping(value.as_ptr(), destination, value.len()) };
    }
    Ok(())
}

fn copy_array_to_c<T: Copy>(
    value: &[T],
    destination: *mut T,
    capacity: usize,
    out_count: *mut usize,
    label: &'static str,
) -> Result<(), ApiError> {
    if out_count.is_null() {
        return Err(invalid(format!("{label} count output is null")));
    }
    // SAFETY: caller supplies writable storage for one size_t.
    unsafe { ptr::write(out_count, value.len()) };
    if destination.is_null() && capacity == 0 {
        return Ok(());
    }
    if capacity < value.len() {
        return Err(ApiError::new(
            OuiStatus::BufferTooSmall,
            format!("{label} destination is too small"),
        ));
    }
    if !value.is_empty() {
        if destination.is_null() {
            return Err(invalid(format!("{label} destination is null")));
        }
        // SAFETY: element capacity and null were validated.
        unsafe { ptr::copy_nonoverlapping(value.as_ptr(), destination, value.len()) };
    }
    Ok(())
}

fn animation_easing(value: OuiEasing) -> Result<Easing, ApiError> {
    if value.reserved != 0 || value.values.iter().any(|number| !number.is_finite()) {
        return Err(invalid(
            "animation easing contains invalid or reserved values",
        ));
    }
    let step_position = match value.step_position {
        0 => StepPosition::JumpStart,
        1 => StepPosition::JumpEnd,
        2 => StepPosition::JumpNone,
        3 => StepPosition::JumpBoth,
        _ => return Err(invalid("unknown step position")),
    };
    match value.kind {
        0 => Ok(Easing::Linear),
        1 => Easing::cubic_bezier(
            value.values[0],
            value.values[1],
            value.values[2],
            value.values[3],
        )
        .map_err(|error| invalid(error.to_string())),
        2 => Easing::steps(value.step_count, step_position)
            .map_err(|error| invalid(error.to_string())),
        3 => Easing::linear_stops(vec![
            LinearStop {
                input: value.values[0],
                output: value.values[1],
            },
            LinearStop {
                input: value.values[2],
                output: value.values[3],
            },
        ])
        .map_err(|error| invalid(error.to_string())),
        _ => Err(invalid("unknown animation easing kind")),
    }
}

fn animation_options(value: *const OuiAnimationOptions) -> Result<AnimationOptions, ApiError> {
    if value.is_null() {
        return Err(invalid("animation options are null"));
    }
    // SAFETY: the C contract requires a readable options structure.
    let value = unsafe { &*value };
    check_header(
        value.struct_size,
        value.abi_version,
        size_of::<OuiAnimationOptions>(),
    )?;
    let iterations = if value.iterations == -1.0 {
        IterationCount::Infinite
    } else {
        IterationCount::Number(value.iterations)
    };
    let direction = match value.direction {
        0 => PlaybackDirection::Normal,
        1 => PlaybackDirection::Reverse,
        2 => PlaybackDirection::Alternate,
        3 => PlaybackDirection::AlternateReverse,
        _ => return Err(invalid("unknown animation direction")),
    };
    let fill = match value.fill {
        0 => FillMode::None,
        1 => FillMode::Forwards,
        2 => FillMode::Backwards,
        3 => FillMode::Both,
        _ => return Err(invalid("unknown animation fill mode")),
    };
    let play_state = match value.play_state {
        0 => PlayState::Running,
        1 => PlayState::Paused,
        _ => return Err(invalid("unknown animation play state")),
    };
    let composite = match value.composite {
        0 => CompositeOperation::Replace,
        1 => CompositeOperation::Add,
        2 => CompositeOperation::Accumulate,
        _ => return Err(invalid("unknown animation composite operation")),
    };
    let options = AnimationOptions {
        delay_ms: value.delay_ms,
        duration_ms: value.duration_ms,
        iterations,
        direction,
        fill,
        play_state,
        playback_rate: value.playback_rate,
        composite,
        easing: animation_easing(value.easing)?,
    };
    options
        .validate()
        .map_err(|error| invalid(error.to_string()))?;
    Ok(options)
}

fn animation_timeline(
    owner: &Rc<DocumentState>,
    value: *const OuiAnimationTimeline,
) -> Result<AnimationTimeline, ApiError> {
    if value.is_null() {
        return Ok(AnimationTimeline::Document);
    }
    // SAFETY: the C contract requires a readable timeline structure.
    let value = unsafe { &*value };
    check_header(
        value.struct_size,
        value.abi_version,
        size_of::<OuiAnimationTimeline>(),
    )?;
    if value.kind == 0 {
        if !value.source.is_null() {
            return Err(invalid("document timelines must not specify a source"));
        }
        return Ok(AnimationTimeline::Document);
    }
    let source = element(value.source as usize)?;
    let source_document = element_document(&source)?;
    if !Rc::ptr_eq(owner, &source_document) {
        return Err(ApiError::new(
            OuiStatus::WrongDocument,
            "animation timeline source belongs to another document",
        ));
    }
    let axis = match value.axis {
        0 => TimelineAxis::Block,
        1 => TimelineAxis::Inline,
        2 => TimelineAxis::X,
        3 => TimelineAxis::Y,
        _ => return Err(invalid("unknown animation timeline axis")),
    };
    let range = TimelineRange::new(value.range_start, value.range_end)
        .map_err(|error| invalid(error.to_string()))?;
    match value.kind {
        1 => Ok(AnimationTimeline::Scroll {
            source: source.node,
            axis,
            range,
        }),
        2 => Ok(AnimationTimeline::View {
            subject: source.node,
            axis,
            range,
        }),
        _ => Err(invalid("unknown animation timeline kind")),
    }
}

fn flush_animation_events(state: &Rc<DocumentState>) -> Result<(), ApiError> {
    let events = borrow_engine_mut(state)?.drain_animation_events();
    for animation_event in &events {
        let Ok(address) = element_address(state, animation_event.target) else {
            continue;
        };
        let event_type = match animation_event.kind {
            AnimationEventKind::Start => 20,
            AnimationEventKind::Iteration => 21,
            AnimationEventKind::End => 22,
            AnimationEventKind::Cancel => 23,
        };
        let mut event = synthesized_event(
            event_type,
            OuiUtf8 {
                data: ptr::null(),
                length: 0,
            },
        );
        event.timestamp_ns = (animation_event.elapsed_time_ms.max(0.0) * 1_000_000.0) as u64;
        dispatch_event_to(state, animation_event.target, address, &mut event, false)?;
    }
    state
        .animation_events
        .try_borrow_mut()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "animation event queue is borrowed"))?
        .extend(events);
    Ok(())
}

// SAFETY CONTRACT: no pointers are accepted; this constant query is reentrant.
#[no_mangle]
pub extern "C" fn oui_abi_version() -> u32 {
    OUI_ABI_VERSION
}

// SAFETY CONTRACT: `out_error` must be writable for its declared struct size.
#[no_mangle]
pub extern "C" fn oui_error_get_last(out_error: *mut OuiErrorInfo) -> OuiStatus {
    ffi_preserve(|| {
        let saved = last_error();
        if out_error.is_null() {
            return Err(invalid("out_error is null"));
        }
        // SAFETY: the caller guarantees a readable/writable `OuiErrorInfo`.
        let output = unsafe { &mut *out_error };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiErrorInfo>(),
        )?;
        output.status = saved.status;
        output.detail = saved.detail;
        output.message_length = saved.message.len();
        Ok(())
    })
}

// SAFETY CONTRACT: `out_length` is writable; `destination` is writable for
// `capacity` bytes when capacity is nonzero. Messages are not NUL-terminated.
#[no_mangle]
pub extern "C" fn oui_error_copy_message(
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
) -> OuiStatus {
    ffi_preserve(|| {
        let saved = last_error();
        if out_length.is_null() {
            return Err(invalid("out_length is null"));
        }
        // SAFETY: the caller guarantees writable storage for one `size_t`.
        unsafe { ptr::write(out_length, saved.message.len()) };
        if destination.is_null() && capacity == 0 {
            return Ok(());
        }
        if capacity < saved.message.len() {
            return Err(ApiError::new(
                OuiStatus::BufferTooSmall,
                "error message destination is too small",
            ));
        }
        if !saved.message.is_empty() {
            if destination.is_null() {
                return Err(invalid("destination is null"));
            }
            // SAFETY: capacity was checked and the caller guarantees that the
            // destination references that many writable bytes. Regions cannot
            // overlap because the source is Rust-owned TLS storage.
            unsafe {
                ptr::copy_nonoverlapping(saved.message.as_ptr(), destination, saved.message.len())
            };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `config` is readable and `out_app` is writable for one handle.
#[no_mangle]
pub extern "C" fn oui_app_create(
    config: *const OuiAppConfig,
    out_app: *mut *mut OuiApp,
) -> OuiStatus {
    ffi(|| {
        if config.is_null() {
            return Err(invalid("config is null"));
        }
        // SAFETY: the caller guarantees a readable `OuiAppConfig`.
        let config = unsafe { &*config };
        check_header(
            config.struct_size,
            config.abi_version,
            size_of::<OuiAppConfig>(),
        )?;
        if config.reserved != 0 {
            return Err(invalid("reserved app configuration fields must be zero"));
        }
        if config.backend > 2 {
            return Err(invalid("unknown backend preference"));
        }
        let _title = utf8(config.title, "title")?;
        let document_config = OuiDocumentConfig {
            struct_size: size_of::<OuiDocumentConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            viewport: OuiViewportMetrics {
                logical_width: f64::from(config.width),
                logical_height: f64::from(config.height),
                physical_width: config.width,
                physical_height: config.height,
                device_scale_factor: 1.0,
                authority: 1,
                reserved: 0,
            },
        };
        let state = Rc::new(AppState {
            document: new_document(&document_config)?,
            #[cfg(all(feature = "linux", target_os = "linux"))]
            title: _title,
            #[cfg(all(feature = "linux", target_os = "linux"))]
            backend: config.backend,
            running: Cell::new(false),
            has_run: Cell::new(false),
            exit_requested: Cell::new(false),
            exit_handle: RefCell::new(None),
        });
        write_handle(out_app, LocalHandle::App(state))
    })
}

// SAFETY CONTRACT: `app` is either null or an Open UI app handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_app_destroy(app: *mut OuiApp) -> OuiStatus {
    ffi(|| {
        let state = native_app::app_state(app)?;
        if state.running.get() {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "cannot destroy an app while its native run is active",
            ));
        }
        destroy(app as usize, HandleKind::App)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `app` is a live app handle and `out_document` is writable.
#[no_mangle]
pub extern "C" fn oui_app_document(
    app: *mut OuiApp,
    out_document: *mut *mut OuiDocument,
) -> OuiStatus {
    ffi(|| {
        let app = match get(app as usize, HandleKind::App)? {
            LocalHandle::App(value) => value,
            _ => unreachable!("kind checked by registry"),
        };
        write_handle(out_document, LocalHandle::Document(app.document.clone()))
    })
}

// SAFETY CONTRACT: `config` is readable and `out_document` is writable.
#[no_mangle]
pub extern "C" fn oui_document_create(
    config: *const OuiDocumentConfig,
    out_document: *mut *mut OuiDocument,
) -> OuiStatus {
    ffi(|| {
        if config.is_null() {
            return Err(invalid("config is null"));
        }
        // SAFETY: the caller guarantees a readable `OuiDocumentConfig`.
        let config = unsafe { &*config };
        write_handle(out_document, LocalHandle::Document(new_document(config)?))
    })
}

// SAFETY CONTRACT: `document` is a live document handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_document_destroy(document: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        destroy(document as usize, HandleKind::Document)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and `out_root` is writable.
#[no_mangle]
pub extern "C" fn oui_document_root(
    document_handle: *mut OuiDocument,
    out_root: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_root.is_null() {
            return Err(invalid("out_root is null"));
        }
        let state = document(document_handle as usize)?;
        let node = borrow_engine(&state)?.root();
        write_element_handle(out_root, &state, node)
    })
}

// SAFETY CONTRACT: `document` is live, `id` names readable UTF-8 bytes, and
// `out_element` is writable. A found result is an owned handle; release it
// with `oui_element_destroy`. A missing ID writes null and returns success.
#[no_mangle]
pub extern "C" fn oui_document_element_by_id(
    document_handle: *mut OuiDocument,
    id: OuiUtf8,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() {
            return Err(invalid("out_element is null"));
        }
        // SAFETY: the caller guarantees storage for one output pointer.
        unsafe { ptr::write(out_element, ptr::null_mut()) };
        let state = document(document_handle as usize)?;
        let id = utf8(id, "id")?;
        let node = borrow_engine(&state)?.element_by_id(&id);
        if let Some(node) = node {
            write_element_handle(out_element, &state, node)?;
        }
        Ok(())
    })
}

// SAFETY CONTRACT: both pointers reference live/readable objects for this call.
#[no_mangle]
pub extern "C" fn oui_document_set_viewport(
    document_handle: *mut OuiDocument,
    viewport: *const OuiDocumentConfig,
) -> OuiStatus {
    ffi(|| {
        if viewport.is_null() {
            return Err(invalid("viewport is null"));
        }
        // SAFETY: the caller guarantees a readable `OuiDocumentConfig`.
        let config = unsafe { &*viewport };
        check_header(
            config.struct_size,
            config.abi_version,
            size_of::<OuiDocumentConfig>(),
        )?;
        let viewport = viewport_metrics(&config.viewport)?;
        let state = document(document_handle as usize)?;
        borrow_engine_mut(&state)?.set_viewport(viewport)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `out_viewport` points to writable storage for one value.
#[no_mangle]
pub extern "C" fn oui_document_get_viewport(
    document_handle: *mut OuiDocument,
    out_viewport: *mut OuiViewportMetrics,
) -> OuiStatus {
    ffi(|| {
        if out_viewport.is_null() {
            return Err(invalid("out_viewport is null"));
        }
        let state = document(document_handle as usize)?;
        let metrics = borrow_engine(&state)?.viewport();
        let value = OuiViewportMetrics {
            logical_width: metrics.logical_width(),
            logical_height: metrics.logical_height(),
            physical_width: metrics.physical_width(),
            physical_height: metrics.physical_height(),
            device_scale_factor: metrics.device_scale_factor(),
            authority: match metrics.authority() {
                ViewportAuthority::Logical => 1,
                ViewportAuthority::Physical => 2,
            },
            reserved: 0,
        };
        // SAFETY: the caller guarantees writable storage and null was rejected.
        unsafe { ptr::write(out_viewport, value) };
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is a live document handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_document_begin_update(document_handle: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        let depth = state
            .update_depth
            .get()
            .checked_add(1)
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "update depth overflow"))?;
        state.update_depth.set(depth);
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and has a matching begin-update call.
#[no_mangle]
pub extern "C" fn oui_document_end_update(document_handle: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        let depth = state.update_depth.get();
        if depth == 0 {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "end_update without begin_update",
            ));
        }
        state.update_depth.set(depth - 1);
        if depth == 1 {
            borrow_engine_mut(&state)?.update()?;
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and no update transaction is open.
#[no_mangle]
pub extern "C" fn oui_document_update(document_handle: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        if state.update_depth.get() != 0 {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "cannot update during a transaction",
            ));
        }
        borrow_engine_mut(&state)?.update()?;
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and `out_update` is readable/writable
// and initialized with the v0.2 struct header.
#[no_mangle]
pub extern "C" fn oui_document_accessibility_update(
    document_handle: *mut OuiDocument,
    out_update: *mut OuiAccessibilityUpdate,
) -> OuiStatus {
    ffi(|| {
        if out_update.is_null() {
            return Err(invalid("accessibility update output is null"));
        }
        // SAFETY: the caller guarantees a readable/writable output struct.
        let output = unsafe { &mut *out_update };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiAccessibilityUpdate>(),
        )?;
        let state = document(document_handle as usize)?;
        let mut engine = borrow_engine_mut(&state)?;
        let update = engine.accessibility_update()?;
        output.generation = engine.dirty_generations().accessibility;
        output.updated_nodes = update.nodes.len();
        output.focus_id = update.focus.0;
        output.full_tree = u8::from(update.tree.is_some());
        output.reduced_motion = u8::from(engine.prefers_reduced_motion());
        output.reserved = 0;
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and `reduced` is zero or one.
#[no_mangle]
pub extern "C" fn oui_document_set_reduced_motion(
    document_handle: *mut OuiDocument,
    reduced: u8,
) -> OuiStatus {
    ffi(|| {
        if reduced > 1 {
            return Err(invalid("reduced motion must be zero or one"));
        }
        let state = document(document_handle as usize)?;
        borrow_engine_mut(&state)?.set_prefers_reduced_motion(reduced != 0);
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and `time_ms` is finite and non-negative.
#[no_mangle]
pub extern "C" fn oui_document_set_animation_time(
    document_handle: *mut OuiDocument,
    time_ms: f64,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        borrow_engine_mut(&state)?.set_animation_time(time_ms)?;
        flush_animation_events(&state)
    })
}

// SAFETY CONTRACT: `document` is live and `out_animating` is writable.
#[no_mangle]
pub extern "C" fn oui_document_is_animating(
    document_handle: *mut OuiDocument,
    out_animating: *mut u8,
) -> OuiStatus {
    ffi(|| {
        if out_animating.is_null() {
            return Err(invalid("out_animating is null"));
        }
        let state = document(document_handle as usize)?;
        let animating = u8::from(borrow_engine(&state)?.is_animating());
        // SAFETY: output storage was checked for null and must be writable.
        unsafe { ptr::write(out_animating, animating) };
        Ok(())
    })
}

macro_rules! animation_command {
    ($name:ident, $method:ident) => {
        // SAFETY CONTRACT: `document` is live and the animation ID was created by it.
        #[no_mangle]
        pub extern "C" fn $name(document_handle: *mut OuiDocument, animation_id: u64) -> OuiStatus {
            ffi(|| {
                let state = document(document_handle as usize)?;
                borrow_engine_mut(&state)?.$method(AnimationId(animation_id))?;
                flush_animation_events(&state)
            })
        }
    };
}

animation_command!(oui_document_animation_pause, pause_animation);
animation_command!(oui_document_animation_play, play_animation);
animation_command!(oui_document_animation_finish, finish_animation);
animation_command!(oui_document_animation_cancel, cancel_animation);

// SAFETY CONTRACT: `document` and animation ID are live; time is finite.
#[no_mangle]
pub extern "C" fn oui_document_animation_seek(
    document_handle: *mut OuiDocument,
    animation_id: u64,
    current_time_ms: f64,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        borrow_engine_mut(&state)?.seek_animation(AnimationId(animation_id), current_time_ms)?;
        flush_animation_events(&state)
    })
}

// SAFETY CONTRACT: `document` and animation ID are live; rate is finite/non-zero.
#[no_mangle]
pub extern "C" fn oui_document_animation_set_playback_rate(
    document_handle: *mut OuiDocument,
    animation_id: u64,
    playback_rate: f64,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        borrow_engine_mut(&state)?
            .set_animation_playback_rate(AnimationId(animation_id), playback_rate)?;
        flush_animation_events(&state)
    })
}

// SAFETY CONTRACT: `document` and animation ID are live; `out_state` is a
// readable/writable versioned output structure.
#[no_mangle]
pub extern "C" fn oui_document_animation_get_state(
    document_handle: *mut OuiDocument,
    animation_id: u64,
    out_state: *mut OuiAnimationState,
) -> OuiStatus {
    ffi(|| {
        if out_state.is_null() {
            return Err(invalid("animation state output is null"));
        }
        // SAFETY: the caller supplies readable/writable output storage.
        let output = unsafe { &mut *out_state };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiAnimationState>(),
        )?;
        let state = document(document_handle as usize)?;
        let animation = borrow_engine(&state)?.animation_state(AnimationId(animation_id))?;
        output.animation_id = animation.id.0;
        output.current_time_ms = animation.current_time_ms;
        output.iteration = animation.iteration;
        output.property = animation.property as i32;
        output.play_state = match animation.play_state {
            PlayState::Running => 0,
            PlayState::Paused => 1,
        };
        output.phase = match animation.phase {
            AnimationPhase::Before => 0,
            AnimationPhase::Active => 1,
            AnimationPhase::After => 2,
        };
        output.finished = u8::from(animation.finished);
        output.reserved = [0; 3];
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and both outputs are writable. The
// caller initializes the event output's versioned struct header.
#[no_mangle]
pub extern "C" fn oui_document_take_animation_event(
    document_handle: *mut OuiDocument,
    out_event: *mut OuiAnimationEvent,
    out_has_event: *mut u8,
) -> OuiStatus {
    ffi(|| {
        if out_event.is_null() || out_has_event.is_null() {
            return Err(invalid("animation event outputs are null"));
        }
        // SAFETY: the caller supplies readable/writable output storage.
        let output = unsafe { &mut *out_event };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiAnimationEvent>(),
        )?;
        let state = document(document_handle as usize)?;
        let event = {
            let mut events = state.animation_events.try_borrow_mut().map_err(|_| {
                ApiError::new(OuiStatus::Reentrant, "animation event queue is borrowed")
            })?;
            if events.is_empty() {
                None
            } else {
                Some(events.remove(0))
            }
        };
        if let Some(event) = event {
            output.animation_id = event.animation.0;
            output.kind = match event.kind {
                AnimationEventKind::Start => 0,
                AnimationEventKind::Iteration => 1,
                AnimationEventKind::End => 2,
                AnimationEventKind::Cancel => 3,
            };
            output.property = event.property as i32;
            output.elapsed_time_ms = event.elapsed_time_ms;
            output.iteration = event.iteration;
            output.target = element_address(&state, event.target)
                .map(|address| address as *mut OuiElement)
                .unwrap_or(ptr::null_mut());
            // SAFETY: output storage was checked for null and must be writable.
            unsafe { ptr::write(out_has_event, 1) };
        } else {
            // SAFETY: output storage was checked for null and must be writable.
            unsafe { ptr::write(out_has_event, 0) };
        }
        Ok(())
    })
}

fn render_frame(document_handle: *mut OuiDocument) -> Result<openui_compositor::Frame, ApiError> {
    let state = document(document_handle as usize)?;
    if state.update_depth.get() != 0 {
        return Err(ApiError::new(
            OuiStatus::InvalidState,
            "cannot render during a transaction",
        ));
    }
    let scene = borrow_engine_mut(&state)?.scene()?;
    Ok(SoftwareCompositor::default().render(&scene)?)
}

// SAFETY CONTRACT: `document` is live; `out_bitmap` is readable/writable and initialized.
#[no_mangle]
pub extern "C" fn oui_document_render_rgba(
    document_handle: *mut OuiDocument,
    out_bitmap: *mut OuiBitmap,
) -> OuiStatus {
    ffi(|| {
        if out_bitmap.is_null() {
            return Err(invalid("out_bitmap is null"));
        }
        // SAFETY: the caller guarantees a readable/writable `OuiBitmap`.
        let output = unsafe { &mut *out_bitmap };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiBitmap>(),
        )?;
        if !output.pixels.is_null() {
            return Err(invalid(
                "out_bitmap.pixels must be null; destroy the previous buffer first",
            ));
        }
        let frame = render_frame(document_handle)?;
        let buffer = register(LocalHandle::Buffer(Rc::new(frame.pixels)))?;
        output.width = frame.width;
        output.height = frame.height;
        output.stride = frame.stride;
        output.pixels = buffer as *mut OuiBuffer;
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and `out_buffer` is writable.
#[no_mangle]
pub extern "C" fn oui_document_render_png(
    document_handle: *mut OuiDocument,
    out_buffer: *mut *mut OuiBuffer,
) -> OuiStatus {
    ffi(|| {
        if out_buffer.is_null() {
            return Err(invalid("out_buffer is null"));
        }
        let state = document(document_handle as usize)?;
        if state.update_depth.get() != 0 {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "cannot render during a transaction",
            ));
        }
        let scene = borrow_engine_mut(&state)?.scene()?;
        let png = SoftwareCompositor::default().render_png(&scene)?;
        write_handle(out_buffer, LocalHandle::Buffer(Rc::new(png)))
    })
}

// SAFETY CONTRACT: `document` is live and `out_element` is writable.
#[no_mangle]
pub extern "C" fn oui_element_create(
    document_handle: *mut OuiDocument,
    tag: i32,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() {
            return Err(invalid("out_element is null"));
        }
        let state = document(document_handle as usize)?;
        let node = {
            let mut engine = borrow_engine_mut(&state)?;
            if tag == 39 {
                engine.create_svg_foreign_object()?
            } else {
                engine.create_native_element(element_tag(tag)?)?
            }
        };
        write_element_handle(out_element, &state, node)
    })
}

// SAFETY CONTRACT: `document` is live, `text` is readable, and `out_text` is writable.
#[no_mangle]
pub extern "C" fn oui_text_create(
    document_handle: *mut OuiDocument,
    text: OuiUtf8,
    out_text: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_text.is_null() {
            return Err(invalid("out_text is null"));
        }
        let state = document(document_handle as usize)?;
        let text = utf8(text, "text")?;
        let node = borrow_engine_mut(&state)?.create_text(text)?;
        write_element_handle(out_text, &state, node)
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
// Destroying the handle does not remove its retained node.
#[no_mangle]
pub extern "C" fn oui_element_destroy(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        if let Some(state) = element.document.upgrade() {
            state
                .listeners
                .try_borrow_mut()
                .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
                .retain(|record| record.element_address != element_handle as usize);
            if let Ok(mut handles) = state.element_handles.try_borrow_mut() {
                if let Some(addresses) = handles.get_mut(&element.node) {
                    addresses.retain(|address| *address != element_handle as usize);
                    if addresses.is_empty() {
                        handles.remove(&element.node);
                    }
                }
            }
        }
        destroy(element_handle as usize, HandleKind::Element)?;
        Ok(())
    })
}

// SAFETY CONTRACT: both handles are live and owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_append_child(
    parent: *mut OuiElement,
    child: *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        let parent = element(parent as usize)?;
        let child = element(child as usize)?;
        let state = same_document(&parent, &child)?;
        borrow_engine_mut(&state)?.append_child(parent.node, child.node)?;
        Ok(())
    })
}

// SAFETY CONTRACT: all handles are live and owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_insert_before(
    parent: *mut OuiElement,
    child: *mut OuiElement,
    before: *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        let parent = element(parent as usize)?;
        let child = element(child as usize)?;
        let before = element(before as usize)?;
        let state = same_document(&parent, &child)?;
        if !Rc::ptr_eq(&state, &element_document(&before)?) {
            return Err(ApiError::new(
                OuiStatus::WrongDocument,
                "elements belong to different documents",
            ));
        }
        borrow_engine_mut(&state)?.insert_before(parent.node, child.node, before.node)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_remove(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::remove))
}

// SAFETY CONTRACT: `element` is a live node handle owned by this thread.
// The retained node and handle remain valid for later reattachment.
#[no_mangle]
pub extern "C" fn oui_element_detach(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::detach))
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_remove_all_children(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::remove_children))
}

// SAFETY CONTRACT: `element` is live and `text` is readable for its length.
#[no_mangle]
pub extern "C" fn oui_element_set_text(
    element_handle: *mut OuiElement,
    text: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let text = utf8(text, "text")?;
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_text_content(node, text)
        })
    })
}

// SAFETY CONTRACT: `element` is live; name and value slices are readable.
#[no_mangle]
pub extern "C" fn oui_element_set_attribute(
    element_handle: *mut OuiElement,
    name: OuiUtf8,
    value: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let name = utf8(name, "attribute name")?;
        if name.is_empty() {
            return Err(invalid("attribute name is empty"));
        }
        let value = utf8(value, "attribute value")?;
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_attribute(node, name, value)
        })
    })
}

// SAFETY CONTRACT: `element` is live and the name slice is readable.
#[no_mangle]
pub extern "C" fn oui_element_remove_attribute(
    element_handle: *mut OuiElement,
    name: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let name = utf8(name, "attribute name")?;
        if name.is_empty() {
            return Err(invalid("attribute name is empty"));
        }
        with_element_mut(element_handle as usize, |engine, node| {
            engine.remove_attribute(node, &name).map(|_| ())
        })
    })
}

// SAFETY CONTRACT: `element` is live and `value` points to a readable tagged value.
#[no_mangle]
pub extern "C" fn oui_element_set_property(
    element_handle: *mut OuiElement,
    property: i32,
    value: *const OuiStyleValue,
) -> OuiStatus {
    ffi(|| {
        let property = property_from_raw(property)
            .ok_or_else(|| invalid("unknown style property identifier"))?;
        if value.is_null() {
            return Err(invalid("style value is null"));
        }
        // SAFETY: the caller guarantees one readable `OuiStyleValue`.
        let value = value::style_value(property, unsafe { &*value })?;
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_property(node, property, value)
        })
    })
}

// SAFETY CONTRACT: `element` is live; keyframes, options, optional timeline,
// and output storage remain readable/writable for the duration of this call.
#[no_mangle]
pub extern "C" fn oui_element_animate(
    element_handle: *mut OuiElement,
    property: i32,
    keyframes: *const OuiKeyframe,
    keyframe_count: usize,
    options: *const OuiAnimationOptions,
    timeline: *const OuiAnimationTimeline,
    out_animation_id: *mut u64,
) -> OuiStatus {
    ffi(|| {
        if out_animation_id.is_null() {
            return Err(invalid("animation ID output is null"));
        }
        let property = property_from_raw(property)
            .ok_or_else(|| invalid("unknown style property identifier"))?;
        let raw_keyframes = copy_array(keyframes, keyframe_count, "keyframe")?;
        let mut typed_keyframes = Vec::new();
        typed_keyframes
            .try_reserve_exact(raw_keyframes.len())
            .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "keyframe allocation failed"))?;
        for keyframe in raw_keyframes {
            if keyframe.reserved != 0 || keyframe.has_easing > 1 || !keyframe.offset.is_finite() {
                return Err(invalid("keyframe contains invalid or reserved values"));
            }
            typed_keyframes.push(Keyframe {
                offset: keyframe.offset,
                value: value::style_value(property, &keyframe.value)?,
                easing: if keyframe.has_easing == 1 {
                    Some(animation_easing(keyframe.easing)?)
                } else {
                    None
                },
            });
        }
        let keyframes =
            Keyframes::new(typed_keyframes).map_err(|error| invalid(error.to_string()))?;
        let keyframes = PropertyKeyframes::typed(property, keyframes)
            .map_err(|error| invalid(error.to_string()))?;
        let options = animation_options(options)?;
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let timeline = animation_timeline(&state, timeline)?;
        let id = borrow_engine_mut(&state)?.animate(element.node, keyframes, options, timeline)?;
        // SAFETY: output storage was checked for null and must be writable.
        unsafe { ptr::write(out_animation_id, id.0) };
        flush_animation_events(&state)
    })
}

// SAFETY CONTRACT: `element` is live; target, options, and output storage
// remain readable/writable for the duration of this call.
#[no_mangle]
pub extern "C" fn oui_element_transition(
    element_handle: *mut OuiElement,
    property: i32,
    target: *const OuiStyleValue,
    options: *const OuiAnimationOptions,
    out_animation_id: *mut u64,
) -> OuiStatus {
    ffi(|| {
        if target.is_null() || out_animation_id.is_null() {
            return Err(invalid("transition target or animation ID output is null"));
        }
        let property = property_from_raw(property)
            .ok_or_else(|| invalid("unknown style property identifier"))?;
        // SAFETY: the caller guarantees one readable tagged value.
        let target = value::style_value(property, unsafe { &*target })?;
        let options = animation_options(options)?;
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let id = borrow_engine_mut(&state)?.transition(element.node, property, target, options)?;
        // SAFETY: output storage was checked for null and must be writable.
        unsafe { ptr::write(out_animation_id, id.0) };
        flush_animation_events(&state)
    })
}

// SAFETY CONTRACT: `element` is live and `out_rect` is writable.
#[no_mangle]
pub extern "C" fn oui_element_get_bounds(
    element_handle: *mut OuiElement,
    out_rect: *mut OuiRect,
) -> OuiStatus {
    ffi(|| {
        if out_rect.is_null() {
            return Err(invalid("out_rect is null"));
        }
        let source = element(element_handle as usize)?;
        let state = element_document(&source)?;
        let bounds = borrow_engine_mut(&state)?
            .bounds(source.node)?
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element has no layout box"))?;
        // SAFETY: the caller guarantees writable storage for one `OuiRect`.
        unsafe {
            ptr::write(
                out_rect,
                OuiRect {
                    x: bounds.x,
                    y: bounds.y,
                    width: bounds.width,
                    height: bounds.height,
                },
            )
        };
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live. `out_count` is writable. When copying,
// `rects` is writable for `capacity` rectangles and does not overlap outputs.
#[no_mangle]
pub extern "C" fn oui_element_get_client_rects_v1(
    element_handle: *mut OuiElement,
    rects: *mut OuiRect,
    capacity: usize,
    out_count: *mut usize,
) -> OuiStatus {
    ffi(|| {
        if out_count.is_null() {
            return Err(invalid("rectangle count output is null"));
        }
        let source = element(element_handle as usize)?;
        let state = element_document(&source)?;
        let owned: Vec<_> = borrow_engine_mut(&state)?
            .client_rects(source.node)?
            .into_iter()
            .map(|rect| OuiRect {
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            })
            .collect();
        copy_array_to_c(&owned, rects, capacity, out_count, "client rectangles")
    })
}

// SAFETY CONTRACT: `out_metrics` has a readable initialized version header and,
// when its declared size is sufficient, writable storage for the complete
// structure. `out_has_metrics` is writable, nonoverlapping byte storage.
// Both outputs are unchanged on error; a missing layout box returns zero sizes.
#[no_mangle]
pub extern "C" fn oui_element_get_scroll_metrics_v1(
    element_handle: *mut OuiElement,
    out_metrics: *mut OuiScrollMetricsV1,
    out_has_metrics: *mut u8,
) -> OuiStatus {
    ffi(|| {
        if out_metrics.is_null() || out_has_metrics.is_null() {
            return Err(invalid("scroll metrics output is null"));
        }
        // Read the prefix before borrowing the full versioned structure. A
        // caller with an older, shorter allocation can be rejected safely.
        let struct_size = unsafe { ptr::read(out_metrics.cast::<u32>()) };
        if struct_size < 2 * size_of::<u32>() as u32 {
            return Err(invalid("scroll metrics header is too small"));
        }
        let abi_version = unsafe { ptr::read(out_metrics.cast::<u32>().add(1)) };
        check_header(struct_size, abi_version, size_of::<OuiScrollMetricsV1>())?;
        let source = element(element_handle as usize)?;
        let state = element_document(&source)?;
        let metrics = borrow_engine_mut(&state)?.scroll_metrics(source.node)?;
        let output = OuiScrollMetricsV1 {
            struct_size,
            abi_version,
            client_width: metrics.map_or(0.0, |value| value.client_width),
            client_height: metrics.map_or(0.0, |value| value.client_height),
            scroll_width: metrics.map_or(0.0, |value| value.scroll_width),
            scroll_height: metrics.map_or(0.0, |value| value.scroll_height),
        };
        // SAFETY: both outputs were validated above and the C contract requires
        // writable, properly aligned and nonoverlapping caller storage.
        unsafe {
            ptr::write(out_metrics, output);
            ptr::write(out_has_metrics, u8::from(metrics.is_some()));
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live; offsets must be finite.
#[no_mangle]
pub extern "C" fn oui_element_scroll_to(
    element_handle: *mut OuiElement,
    x: f64,
    y: f64,
) -> OuiStatus {
    ffi(|| {
        with_element_mut(element_handle as usize, |engine, node| {
            engine.scroll_to(node, x, y)
        })
    })
}

fn scroll_into_view_options(
    block: u32,
    inline: u32,
    container: u32,
) -> Result<openui_engine::ScrollIntoViewOptions, ApiError> {
    use openui_engine::{ScrollAlignment, ScrollIntoViewContainer, ScrollIntoViewOptions};
    let alignment = |value| match value {
        0 => Ok(ScrollAlignment::Start),
        1 => Ok(ScrollAlignment::Center),
        2 => Ok(ScrollAlignment::End),
        3 => Ok(ScrollAlignment::Nearest),
        _ => Err(invalid("invalid scroll alignment")),
    };
    Ok(ScrollIntoViewOptions {
        block: alignment(block)?,
        inline: alignment(inline)?,
        container: match container {
            0 => ScrollIntoViewContainer::All,
            1 => ScrollIntoViewContainer::Nearest,
            _ => return Err(invalid("invalid scroll container selection")),
        },
    })
}

// SAFETY CONTRACT: `element_handle` is a live element handle. All options are
// validated before borrowing the shared Engine; panics remain contained.
#[no_mangle]
pub extern "C" fn oui_element_scroll_into_view_v1(
    element_handle: *mut OuiElement,
    block: u32,
    inline: u32,
    container: u32,
) -> OuiStatus {
    ffi(|| {
        let options = scroll_into_view_options(block, inline, container)?;
        with_element_mut(element_handle as usize, |engine, node| {
            engine.scroll_into_view(node, options)
        })
    })
}

// SAFETY CONTRACT: `element_handle` is live. Duration is finite/non-negative.
// No callback is invoked while the Engine/platform state is borrowed.
#[no_mangle]
pub extern "C" fn oui_element_smooth_scroll_into_view_v1(
    element_handle: *mut OuiElement,
    block: u32,
    inline: u32,
    container: u32,
    duration_ms: f64,
) -> OuiStatus {
    ffi(|| {
        let options = scroll_into_view_options(block, inline, container)?;
        if !duration_ms.is_finite() || duration_ms < 0.0 {
            return Err(invalid("scroll duration must be finite and non-negative"));
        }
        with_element_mut(element_handle as usize, |engine, node| {
            engine
                .smooth_scroll_into_view(node, options, duration_ms)
                .map(|_| ())
        })
    })
}

// SAFETY CONTRACT: `element` is live and both output pointers are writable.
#[no_mangle]
pub extern "C" fn oui_element_get_scroll_offset(
    element_handle: *mut OuiElement,
    out_x: *mut f64,
    out_y: *mut f64,
) -> OuiStatus {
    ffi(|| {
        if out_x.is_null() || out_y.is_null() {
            return Err(invalid("scroll output pointer is null"));
        }
        let source = element(element_handle as usize)?;
        let state = element_document(&source)?;
        let (x, y) = {
            let mut engine = borrow_engine_mut(&state)?;
            engine.update()?;
            engine.scroll_offset(source.node)?
        };
        // SAFETY: both pointers are caller-provided writable `double` slots.
        unsafe {
            ptr::write(out_x, x);
            ptr::write(out_y, y);
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_focus(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::focus))
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_blur(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::blur))
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_set_pointer_capture(
    element_handle: *mut OuiElement,
    pointer_id: u32,
) -> OuiStatus {
    ffi(|| {
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_pointer_capture(pointer_id as u64, node)
        })
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_release_pointer_capture(
    element_handle: *mut OuiElement,
    pointer_id: u32,
) -> OuiStatus {
    ffi(|| {
        with_element_mut(element_handle as usize, |engine, node| {
            engine.release_pointer_capture(pointer_id as u64, node)
        })
    })
}

// SAFETY CONTRACT: `element` is live, callback is callable for the listener
// lifetime, user_data obeys application ownership, and `out_listener` is writable.
#[no_mangle]
pub extern "C" fn oui_element_add_event_listener(
    element_handle: *mut OuiElement,
    event_type: i32,
    capture: u8,
    callback: Option<OuiEventCallback>,
    user_data: *mut c_void,
    out_listener: *mut *mut OuiListener,
) -> OuiStatus {
    ffi(|| {
        if out_listener.is_null() {
            return Err(invalid("out_listener is null"));
        }
        if event_type <= 0 || !valid_event_type(event_type as u32) {
            return Err(invalid("unknown event type"));
        }
        if capture > 1 {
            return Err(invalid("capture must be zero or one"));
        }
        let callback = callback.ok_or_else(|| invalid("event callback is null"))?;
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine(&state)?.element_tag(element.node)?;
        let id = state.next_listener.get();
        state.next_listener.set(
            id.checked_add(1)
                .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "listener ID overflow"))?,
        );
        state
            .listeners
            .try_borrow_mut()
            .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
            .push(ListenerRecord {
                id,
                node: element.node,
                event_type: event_type as u32,
                capture: capture != 0,
                callback,
                user_data,
                element_address: element_handle as usize,
            });
        write_handle(
            out_listener,
            LocalHandle::Listener(ListenerRef {
                document: Rc::downgrade(&state),
                id,
            }),
        )
    })
}

// SAFETY CONTRACT: `listener` is a live listener handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_listener_destroy(listener: *mut OuiListener) -> OuiStatus {
    ffi(|| {
        let listener = match destroy(listener as usize, HandleKind::Listener)? {
            LocalHandle::Listener(value) => value,
            _ => unreachable!("kind checked by registry"),
        };
        if let Some(state) = listener.document.upgrade() {
            state
                .listeners
                .try_borrow_mut()
                .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
                .retain(|record| record.id != listener.id);
        }
        Ok(())
    })
}

fn invoke_event_listeners(
    state: &DocumentState,
    node: NodeHandle,
    capture: bool,
    event: &mut OuiEvent,
) -> Result<(), ApiError> {
    let callbacks: Vec<_> = state
        .listeners
        .try_borrow()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
        .iter()
        .filter(|listener| {
            listener.node == node
                && listener.event_type == event.event_type
                && listener.capture == capture
        })
        .cloned()
        .collect();
    for listener in callbacks {
        event.current_target = listener.element_address as *mut OuiElement;
        // SAFETY: listener registration requires the callback to remain
        // callable and user_data to remain valid until listener destruction.
        // No engine or listener-list borrow is held, so reentrant API calls are
        // explicitly permitted. The event pointer lives through the call.
        unsafe { (listener.callback)(event, listener.user_data) };
    }
    Ok(())
}

fn validate_event(event: &OuiEvent) -> Result<(), ApiError> {
    check_header(event.struct_size, event.abi_version, size_of::<OuiEvent>())?;
    if !valid_event_type(event.event_type) {
        return Err(invalid("unknown event type"));
    }
    if event.flags & !(OUI_EVENT_FLAG_DEFAULT_PREVENTED | OUI_EVENT_FLAG_PROPAGATION_STOPPED) != 0
        || event.modifiers & !0x1f != 0
    {
        return Err(invalid("event contains unknown flag bits"));
    }
    utf8(event.text, "event text")?;
    if !event.x.is_finite()
        || !event.y.is_finite()
        || !event.delta_x.is_finite()
        || !event.delta_y.is_finite()
    {
        return Err(invalid("event coordinates and deltas must be finite"));
    }
    Ok(())
}

fn synthesized_event(event_type: u32, text: OuiUtf8) -> OuiEvent {
    OuiEvent {
        struct_size: size_of::<OuiEvent>() as u32,
        abi_version: OUI_ABI_VERSION,
        event_type,
        phase: 0,
        flags: 0,
        modifiers: 0,
        timestamp_ns: 0,
        x: 0.0,
        y: 0.0,
        delta_x: 0.0,
        delta_y: 0.0,
        key_code: 0,
        pointer_id: 0,
        text,
        target: ptr::null_mut(),
        current_target: ptr::null_mut(),
    }
}

fn dispatch_event_to(
    state: &Rc<DocumentState>,
    target: NodeHandle,
    target_address: usize,
    event: &mut OuiEvent,
    apply_default: bool,
) -> Result<(), ApiError> {
    let route = borrow_engine(state)?.event_route(target)?;
    event.flags &= !(OUI_EVENT_FLAG_DEFAULT_PREVENTED | OUI_EVENT_FLAG_PROPAGATION_STOPPED);
    event.target = target_address as *mut OuiElement;
    for step in route.steps {
        match step.phase {
            EngineEventPhase::Capture => {
                event.phase = 1;
                invoke_event_listeners(state, step.node, true, event)?;
            }
            EngineEventPhase::Target => {
                event.phase = 2;
                invoke_event_listeners(state, step.node, true, event)?;
                invoke_event_listeners(state, step.node, false, event)?;
            }
            EngineEventPhase::Bubble => {
                event.phase = 3;
                invoke_event_listeners(state, step.node, false, event)?;
            }
        }
        if event.flags & OUI_EVENT_FLAG_PROPAGATION_STOPPED != 0 {
            break;
        }
    }
    event.current_target = ptr::null_mut();
    if apply_default && event.flags & OUI_EVENT_FLAG_DEFAULT_PREVENTED == 0 {
        let text = utf8(event.text, "event text")?;
        match event.event_type {
            4 => {
                let changed = borrow_engine_mut(state)?.activate(target)?.changed;
                for changed in changed {
                    let Ok(address) = element_address(state, changed) else {
                        continue;
                    };
                    for event_type in [16, 17] {
                        let mut derived = *event;
                        derived.event_type = event_type;
                        dispatch_event_to(state, changed, address, &mut derived, false)?;
                    }
                }
            }
            8 => borrow_engine_mut(state)?.insert_text(target, &text)?,
            10 => borrow_engine_mut(state)?.update_composition(target, &text)?,
            11 if text.is_empty() => borrow_engine_mut(state)?.cancel_composition(target)?,
            11 => borrow_engine_mut(state)?.commit_composition(target, &text)?,
            12 => {
                borrow_engine_mut(state)?.focus_with_origin(target, FocusOrigin::Accessibility)?;
            }
            13 => borrow_engine_mut(state)?.blur(target)?,
            _ => {}
        }
    }
    Ok(())
}

// SAFETY CONTRACT: document and target are live same-document handles; `event`
// is readable/writable for its declared size through all synchronous callbacks.
#[no_mangle]
pub extern "C" fn oui_document_dispatch_event(
    document_handle: *mut OuiDocument,
    target_handle: *mut OuiElement,
    event: *mut OuiEvent,
) -> OuiStatus {
    ffi(|| {
        if event.is_null() {
            return Err(invalid("event is null"));
        }
        // SAFETY: the caller guarantees a readable/writable `OuiEvent` that
        // remains alive for this synchronous dispatch.
        let event = unsafe { &mut *event };
        validate_event(event)?;
        let state = document(document_handle as usize)?;
        let target = element(target_handle as usize)?;
        if !Rc::ptr_eq(&state, &element_document(&target)?) {
            return Err(ApiError::new(
                OuiStatus::WrongDocument,
                "event target belongs to another document",
            ));
        }
        dispatch_event_to(&state, target.node, target_handle as usize, event, true)
    })
}

// SAFETY CONTRACT: document is an owning-thread handle; key is readable for its
// declared versioned size and UTF-8 slices remain readable through the call.
// Listener callbacks and user_data follow the existing synchronous contract.
#[no_mangle]
pub extern "C" fn oui_document_dispatch_key_input_v1(
    document_handle: *mut OuiDocument,
    key: *const OuiEvent,
    committed_text: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        if key.is_null() {
            return Err(invalid("key descriptor is null"));
        }
        // SAFETY: a versioned descriptor begins with a readable size field.
        let struct_size = unsafe { ptr::read(key.cast::<u32>()) };
        if struct_size < 2 * size_of::<u32>() as u32 {
            return Err(invalid("key descriptor header is too small"));
        }
        // SAFETY: the declared header covers the ABI-version field.
        let abi_version = unsafe { ptr::read(key.cast::<u32>().add(1)) };
        check_header(struct_size, abi_version, size_of::<OuiEvent>())?;
        // SAFETY: the validated declared size covers the complete descriptor.
        let key = unsafe { *key };
        validate_event(&key)?;
        if key.flags != 0 {
            return Err(invalid("key descriptor flags must be zero"));
        }
        let event_type = match key.event_type {
            6 => openui::KeyEventType::Down,
            7 => openui::KeyEventType::Up,
            _ => return Err(invalid("normalized key input requires key-down or key-up")),
        };
        let key_text = utf8(key.text, "logical key name")?;
        let committed_text = utf8(committed_text, "committed text")?;
        state
            .native
            .dispatch_key_input(
                event_type,
                key.key_code,
                (!key_text.is_empty()).then_some(key_text.as_str()),
                (!committed_text.is_empty()).then_some(committed_text.as_str()),
                openui::Modifiers(key.modifiers),
            )
            .map_err(native_app::native_error)
    })
}

// SAFETY CONTRACT: document is an owning-thread handle; text remains readable
// through this call. Callbacks execute after engine and listener borrows end.
#[no_mangle]
pub extern "C" fn oui_document_dispatch_text_input_v1(
    document_handle: *mut OuiDocument,
    text: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        let text = utf8(text, "committed text")?;
        state
            .native
            .dispatch_text_input(&text)
            .map_err(native_app::native_error)
    })
}

// SAFETY CONTRACT: `document` is live and `event` is readable/writable for its
// declared size through all synchronous callbacks. The event type is pointer
// down, up, or move; hit testing supplies the target.
#[no_mangle]
pub extern "C" fn oui_document_dispatch_pointer_event(
    document_handle: *mut OuiDocument,
    event: *mut OuiEvent,
) -> OuiStatus {
    ffi(|| {
        if event.is_null() {
            return Err(invalid("event is null"));
        }
        // SAFETY: the caller guarantees a live, writable event for this call.
        let event = unsafe { &mut *event };
        validate_event(event)?;
        let kind = match event.event_type {
            1 => PointerEventKind::Down,
            2 => PointerEventKind::Up,
            3 => PointerEventKind::Move,
            24 => PointerEventKind::Cancel,
            _ => return Err(invalid("pointer dispatch requires a pointer event type")),
        };
        let state = document(document_handle as usize)?;
        let update = borrow_engine_mut(&state)?.pointer_event(
            event.pointer_id as u64,
            kind,
            event.x,
            event.y,
        )?;
        for (nodes, event_type) in [
            (update.left.as_slice(), 15),
            (update.entered.as_slice(), 14),
        ] {
            for node in nodes {
                let Ok(address) = element_address(&state, *node) else {
                    continue;
                };
                let mut boundary_event = *event;
                boundary_event.event_type = event_type;
                dispatch_event_to(&state, *node, address, &mut boundary_event, false)?;
            }
        }
        let Some(target) = update.target else {
            event.target = ptr::null_mut();
            event.current_target = ptr::null_mut();
            return Ok(());
        };
        let address = element_address(&state, target)?;
        dispatch_event_to(&state, target, address, event, false)?;
        if event.flags & OUI_EVENT_FLAG_DEFAULT_PREVENTED != 0 {
            return Ok(());
        }
        if kind == PointerEventKind::Down {
            let _ = borrow_engine_mut(&state)?.focus_with_origin(target, FocusOrigin::Pointer);
        }
        if let Some((changed, fraction)) = update.range_value {
            let did_change = borrow_engine_mut(&state)?.set_range_fraction(changed, fraction)?;
            let changed_address = element_address(&state, changed)?;
            if did_change {
                let mut input = *event;
                input.event_type = 16;
                dispatch_event_to(&state, changed, changed_address, &mut input, false)?;
            }
            if kind == PointerEventKind::Up {
                let mut change = *event;
                change.event_type = 17;
                dispatch_event_to(&state, changed, changed_address, &mut change, false)?;
            }
        }
        if let Some(activation) = update.activation {
            let activation_address = element_address(&state, activation)?;
            let mut click = *event;
            click.event_type = 4;
            dispatch_event_to(&state, activation, activation_address, &mut click, true)?;
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live, coordinates are finite, and
// `out_element` is writable. The returned pointer is a borrowed existing
// element handle and must not outlive that handle.
#[no_mangle]
pub extern "C" fn oui_document_hit_test(
    document_handle: *mut OuiDocument,
    x: f32,
    y: f32,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() || !x.is_finite() || !y.is_finite() {
            return Err(invalid("hit-test output and coordinates are invalid"));
        }
        // SAFETY: the caller guarantees storage for one output pointer.
        unsafe { ptr::write(out_element, ptr::null_mut()) };
        let state = document(document_handle as usize)?;
        if let Some(node) = borrow_engine_mut(&state)?.hit_test(x, y)? {
            let address = element_address(&state, node)?;
            // SAFETY: output storage was validated above.
            unsafe { ptr::write(out_element, address as *mut OuiElement) };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live, direction is -1 or 1, and
// `out_element` is writable. The returned pointer is borrowed.
#[no_mangle]
pub extern "C" fn oui_document_advance_focus(
    document_handle: *mut OuiDocument,
    direction: i32,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() {
            return Err(invalid("focused element output is null"));
        }
        // SAFETY: the caller guarantees storage for one output pointer.
        unsafe { ptr::write(out_element, ptr::null_mut()) };
        let state = document(document_handle as usize)?;
        if let Some(node) = borrow_engine_mut(&state)?.advance_focus(direction)? {
            let address = element_address(&state, node)?;
            // SAFETY: output storage was validated above.
            unsafe { ptr::write(out_element, address as *mut OuiElement) };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live. `root` is null to leave modal mode or
// is a live element handle from the same document.
#[no_mangle]
pub extern "C" fn oui_document_set_modal_root(
    document_handle: *mut OuiDocument,
    root_handle: *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        let root = if root_handle.is_null() {
            None
        } else {
            let root = element(root_handle as usize)?;
            if !Rc::ptr_eq(&state, &element_document(&root)?) {
                return Err(ApiError::new(
                    OuiStatus::WrongDocument,
                    "modal root belongs to another document",
                ));
            }
            Some(root.node)
        };
        borrow_engine_mut(&state)?.set_modal_root(root)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and `value` is readable for its length.
#[no_mangle]
pub extern "C" fn oui_element_set_control_value(
    element_handle: *mut OuiElement,
    value: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let value = utf8(value, "control value")?;
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_control_value(element.node, value)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live, `out_length` is writable, and
// `destination` is writable for capacity bytes when capacity is nonzero.
#[no_mangle]
pub extern "C" fn oui_element_copy_control_value(
    element_handle: *mut OuiElement,
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
) -> OuiStatus {
    ffi(|| {
        if out_length.is_null() {
            return Err(invalid("control value length output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let value = borrow_engine(&state)?
            .control_state(element.node)?
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element is not a control"))?
            .value
            .as_bytes()
            .to_vec();
        // SAFETY: output storage for one size_t is guaranteed by the caller.
        unsafe { ptr::write(out_length, value.len()) };
        if destination.is_null() && capacity == 0 {
            return Ok(());
        }
        if capacity < value.len() {
            return Err(ApiError::new(
                OuiStatus::BufferTooSmall,
                "control value destination is too small",
            ));
        }
        if !value.is_empty() {
            if destination.is_null() {
                return Err(invalid("control value destination is null"));
            }
            // SAFETY: capacity was validated and source/destination do not overlap.
            unsafe { ptr::copy_nonoverlapping(value.as_ptr(), destination, value.len()) };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live editable control handle.
#[no_mangle]
pub extern "C" fn oui_element_set_selection(
    element_handle: *mut OuiElement,
    anchor: usize,
    focus: usize,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_selection(element.node, anchor, focus)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and both output pointers are writable.
#[no_mangle]
pub extern "C" fn oui_element_get_selection(
    element_handle: *mut OuiElement,
    out_anchor: *mut usize,
    out_focus: *mut usize,
) -> OuiStatus {
    ffi(|| {
        if out_anchor.is_null() || out_focus.is_null() {
            return Err(invalid("selection output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let selection = borrow_engine(&state)?
            .control_state(element.node)?
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element is not a control"))?
            .selection();
        // SAFETY: both output pointers were validated above.
        unsafe {
            ptr::write(out_anchor, selection.0);
            ptr::write(out_focus, selection.1);
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and `out_flags` is writable.
#[no_mangle]
pub extern "C" fn oui_element_get_control_flags(
    element_handle: *mut OuiElement,
    out_flags: *mut u32,
) -> OuiStatus {
    ffi(|| {
        if out_flags.is_null() {
            return Err(invalid("control flags output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let control = borrow_engine(&state)?
            .control_state(element.node)?
            .cloned()
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element is not a control"))?;
        let mut flags = 0;
        for (enabled, flag) in [
            (control.disabled, OUI_CONTROL_DISABLED),
            (control.checked, OUI_CONTROL_CHECKED),
            (control.selected, OUI_CONTROL_SELECTED),
            (control.open, OUI_CONTROL_OPEN),
            (control.indeterminate, OUI_CONTROL_INDETERMINATE),
            (control.password, OUI_CONTROL_PASSWORD),
        ] {
            if enabled {
                flags |= flag;
            }
        }
        // SAFETY: output storage was validated above.
        unsafe { ptr::write(out_flags, flags) };
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live checkbox/radio and checked is 0 or 1.
#[no_mangle]
pub extern "C" fn oui_element_set_checked(
    element_handle: *mut OuiElement,
    checked: u8,
) -> OuiStatus {
    ffi(|| {
        if checked > 1 {
            return Err(invalid("checked must be zero or one"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_checked(element.node, checked != 0)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live checkbox and indeterminate is 0 or 1.
#[no_mangle]
pub extern "C" fn oui_element_set_indeterminate(
    element_handle: *mut OuiElement,
    indeterminate: u8,
) -> OuiStatus {
    ffi(|| {
        if indeterminate > 1 {
            return Err(invalid("indeterminate must be zero or one"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_indeterminate(element.node, indeterminate != 0)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live adjustable control and adjustment is
// a declared OuiControlAdjustment value.
#[no_mangle]
pub extern "C" fn oui_element_adjust_control(
    element_handle: *mut OuiElement,
    adjustment: i32,
) -> OuiStatus {
    ffi(|| {
        let adjustment = match adjustment {
            0 => ControlAdjustment::Previous,
            1 => ControlAdjustment::Next,
            2 => ControlAdjustment::PageBackward,
            3 => ControlAdjustment::PageForward,
            4 => ControlAdjustment::Minimum,
            5 => ControlAdjustment::Maximum,
            _ => return Err(invalid("unknown control adjustment")),
        };
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.adjust_control(element.node, adjustment)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and `out_id` is writable.
#[no_mangle]
pub extern "C" fn oui_element_get_accessibility_id(
    element_handle: *mut OuiElement,
    out_id: *mut u64,
) -> OuiStatus {
    ffi(|| {
        if out_id.is_null() {
            return Err(invalid("accessibility id output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let id = borrow_engine(&state)?
            .accessibility_node_id(element.node)?
            .0;
        // SAFETY: output storage was validated above.
        unsafe { ptr::write(out_id, id) };
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and role is a declared
// OuiAccessibilityRole value.
#[no_mangle]
pub extern "C" fn oui_element_set_accessibility_role(
    element_handle: *mut OuiElement,
    role: i32,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let mut engine = borrow_engine_mut(&state)?;
        let role = match role {
            0 => return Ok(engine.clear_accessibility_role(element.node)?),
            1 => AccessibilityRole::GenericContainer,
            2 => AccessibilityRole::Button,
            3 => AccessibilityRole::CheckBox,
            4 => AccessibilityRole::RadioButton,
            5 => AccessibilityRole::TextInput,
            6 => AccessibilityRole::MultilineTextInput,
            7 => AccessibilityRole::ComboBox,
            8 => AccessibilityRole::ListBoxOption,
            9 => AccessibilityRole::Slider,
            10 => AccessibilityRole::Image,
            11 => AccessibilityRole::Link,
            12 => AccessibilityRole::Dialog,
            13 => AccessibilityRole::Heading,
            14 => AccessibilityRole::Status,
            15 => AccessibilityRole::Alert,
            16 => AccessibilityRole::Unknown,
            _ => return Err(invalid("unknown accessibility role")),
        };
        engine.set_accessibility_role(element.node, role)?;
        Ok(())
    })
}

macro_rules! accessibility_string_setter {
    ($name:ident, $method:ident, $label:literal) => {
        #[doc = concat!("SAFETY CONTRACT: `element` is live and ", $label, " is readable for its length.")]
        #[no_mangle]
        pub extern "C" fn $name(element_handle: *mut OuiElement, value: OuiUtf8) -> OuiStatus {
            ffi(|| {
                let value = utf8(value, $label)?;
                let element = element(element_handle as usize)?;
                let state = element_document(&element)?;
                borrow_engine_mut(&state)?.$method(element.node, value)?;
                Ok(())
            })
        }
    };
}

accessibility_string_setter!(
    oui_element_set_accessibility_label,
    set_accessibility_label,
    "accessibility label"
);
accessibility_string_setter!(
    oui_element_set_accessibility_description,
    set_accessibility_description,
    "accessibility description"
);
accessibility_string_setter!(
    oui_element_set_accessibility_value,
    set_accessibility_value,
    "accessibility value"
);

// SAFETY CONTRACT: `element` is live and live is a declared
// OuiAccessibilityLive value.
#[no_mangle]
pub extern "C" fn oui_element_set_accessibility_live(
    element_handle: *mut OuiElement,
    live: i32,
) -> OuiStatus {
    ffi(|| {
        let live = match live {
            0 => AccessibilityLive::Off,
            1 => AccessibilityLive::Polite,
            2 => AccessibilityLive::Assertive,
            _ => return Err(invalid("unknown accessibility live mode")),
        };
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_accessibility_live(element.node, live)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and hidden is zero or one.
#[no_mangle]
pub extern "C" fn oui_element_set_accessibility_hidden(
    element_handle: *mut OuiElement,
    hidden: u8,
) -> OuiStatus {
    ffi(|| {
        if hidden > 1 {
            return Err(invalid("accessibility hidden must be zero or one"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_accessibility_hidden(element.node, hidden != 0)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and `targets` is either null for zero
// targets or references target_count aligned, readable element pointers.
#[no_mangle]
pub extern "C" fn oui_element_set_accessibility_relation(
    element_handle: *mut OuiElement,
    relation: i32,
    targets: *const *mut OuiElement,
    target_count: usize,
) -> OuiStatus {
    ffi(|| {
        let relation = match relation {
            0 => AccessibilityRelation::LabelledBy,
            1 => AccessibilityRelation::DescribedBy,
            2 => AccessibilityRelation::Controls,
            3 => AccessibilityRelation::Details,
            _ => return Err(invalid("unknown accessibility relation")),
        };
        let target_addresses = copy_array(targets, target_count, "accessibility target")?;
        let source = element(element_handle as usize)?;
        let state = element_document(&source)?;
        let mut target_nodes = Vec::new();
        target_nodes
            .try_reserve_exact(target_addresses.len())
            .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "relation allocation failed"))?;
        for target in target_addresses {
            let target = element(target as usize)?;
            if !Rc::ptr_eq(&state, &element_document(&target)?) {
                return Err(ApiError::new(
                    OuiStatus::WrongDocument,
                    "accessibility relation crosses documents",
                ));
            }
            target_nodes.push(target.node);
        }
        borrow_engine_mut(&state)?.set_accessibility_relation(
            source.node,
            relation,
            &target_nodes,
        )?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live, action is declared, and value is
// readable for its length. Anchor/focus are used only for selection actions.
#[no_mangle]
pub extern "C" fn oui_element_perform_accessibility_action(
    element_handle: *mut OuiElement,
    action: i32,
    value: OuiUtf8,
    anchor: usize,
    focus: usize,
) -> OuiStatus {
    ffi(|| {
        let action_value = utf8(value, "accessibility action value")?;
        let action = match action {
            0 => AccessibilityAction::Click,
            1 => AccessibilityAction::Focus,
            2 => AccessibilityAction::Blur,
            3 => AccessibilityAction::Increment,
            4 => AccessibilityAction::Decrement,
            5 => AccessibilityAction::Expand,
            6 => AccessibilityAction::Collapse,
            7 => AccessibilityAction::SetValue(action_value),
            8 => AccessibilityAction::ReplaceSelectedText(action_value),
            9 => AccessibilityAction::SetTextSelection { anchor, focus },
            10 => AccessibilityAction::ScrollIntoView,
            _ => return Err(invalid("unknown accessibility action")),
        };
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let ordinary_event_type = match &action {
            AccessibilityAction::Click => Some(4),
            AccessibilityAction::Focus => Some(12),
            AccessibilityAction::Blur => Some(13),
            _ => None,
        };
        if let Some(event_type) = ordinary_event_type {
            let mut event = synthesized_event(event_type, value);
            dispatch_event_to(
                &state,
                element.node,
                element_handle as usize,
                &mut event,
                true,
            )?;
            return Ok(());
        }

        let changed = borrow_engine_mut(&state)?
            .perform_accessibility_action(element.node, action)?
            .changed;
        for changed in changed {
            let Ok(address) = element_address(&state, changed) else {
                continue;
            };
            for event_type in [16, 17] {
                let mut event = synthesized_event(event_type, value);
                dispatch_event_to(&state, changed, address, &mut event, false)?;
            }
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `values` references four readable lengths and `out_compound` is writable.
#[no_mangle]
pub extern "C" fn oui_edges_create(
    values: *const OuiLength,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let values = copy_array(values, 4, "edge")?;
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Edges(Edges {
                top: value::length(values[0])?,
                right: value::length(values[1])?,
                bottom: value::length(values[2])?,
                left: value::length(values[3])?,
            })),
        )
    })
}

// SAFETY CONTRACT: `out_compound` is writable; both lengths are initialized.
#[no_mangle]
pub extern "C" fn oui_gap_create(
    row: OuiLength,
    column: OuiLength,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Gap(Gap {
                row: value::length(row)?,
                column: value::length(column)?,
            })),
        )
    })
}

fn generic_font(value: i32) -> Result<GenericFontFamily, ApiError> {
    match value {
        0 => Ok(GenericFontFamily::None),
        1 => Ok(GenericFontFamily::Serif),
        2 => Ok(GenericFontFamily::SansSerif),
        3 => Ok(GenericFontFamily::Monospace),
        4 => Ok(GenericFontFamily::Cursive),
        5 => Ok(GenericFontFamily::Fantasy),
        6 => Ok(GenericFontFamily::SystemUi),
        7 => Ok(GenericFontFamily::Math),
        8 => Ok(GenericFontFamily::Emoji),
        9 => Ok(GenericFontFamily::FangSong),
        10 => Ok(GenericFontFamily::UiSerif),
        11 => Ok(GenericFontFamily::UiSansSerif),
        12 => Ok(GenericFontFamily::UiMonospace),
        13 => Ok(GenericFontFamily::UiRounded),
        _ => Err(invalid("unknown generic font family")),
    }
}

// SAFETY CONTRACT: `name` is readable and `out_compound` is writable. A
// nonzero generic family ignores name; zero requires a nonempty named family.
#[no_mangle]
pub extern "C" fn oui_font_family_create(
    name: OuiUtf8,
    generic_family: i32,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let family = if generic_family == 0 {
            let name = utf8(name, "font family")?;
            if name.is_empty() {
                return Err(invalid("named font family is empty"));
            }
            FontFamilyList::single(name)
        } else {
            FontFamilyList::generic(generic_font(generic_family)?)
        };
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::FontFamily(family)),
        )
    })
}

fn border_style(value: i32) -> Result<BorderStyle, ApiError> {
    match value {
        0 => Ok(BorderStyle::None),
        1 => Ok(BorderStyle::Hidden),
        2 => Ok(BorderStyle::Dotted),
        3 => Ok(BorderStyle::Dashed),
        4 => Ok(BorderStyle::Solid),
        5 => Ok(BorderStyle::Double),
        6 => Ok(BorderStyle::Groove),
        7 => Ok(BorderStyle::Ridge),
        8 => Ok(BorderStyle::Inset),
        9 => Ok(BorderStyle::Outset),
        _ => Err(invalid("unknown border style")),
    }
}

fn color(value: OuiColor) -> Color {
    Color::from_rgba8(value.red, value.green, value.blue, value.alpha)
}

// SAFETY CONTRACT: `out_compound` is writable and scalar inputs are initialized.
#[no_mangle]
pub extern "C" fn oui_border_create(
    width: f32,
    style: i32,
    color_value: OuiColor,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        if !width.is_finite() || width < 0.0 {
            return Err(invalid("border width must be finite and non-negative"));
        }
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Border(Border {
                width,
                style: border_style(style)?,
                color: color(color_value),
            })),
        )
    })
}

// SAFETY CONTRACT: `values` references four readable lengths and `out_compound` is writable.
#[no_mangle]
pub extern "C" fn oui_corner_radii_create(
    values: *const OuiLength,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let values = copy_array(values, 4, "corner radius")?;
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::CornerRadii(CornerRadii(Edges {
                top: value::length(values[0])?,
                right: value::length(values[1])?,
                bottom: value::length(values[2])?,
                left: value::length(values[3])?,
            }))),
        )
    })
}

// SAFETY CONTRACT: `operations` references `operation_count` readable records
// (at most 4096), and `out_compound` is writable.
#[no_mangle]
pub extern "C" fn oui_transform_create(
    operations: *const OuiTransformOperation,
    operation_count: usize,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let operations = copy_array(operations, operation_count, "transform operation")?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(operations.len())
            .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "transform allocation failed"))?;
        for operation in operations {
            if operation.reserved != 0 || operation.values.iter().any(|value| !value.is_finite()) {
                return Err(invalid("invalid transform operation payload"));
            }
            result.push(match operation.kind {
                1 => TransformOperation::Translate(
                    value::length(operation.x)?,
                    value::length(operation.y)?,
                ),
                2 => TransformOperation::Scale(operation.values[0], operation.values[1]),
                3 => TransformOperation::Rotate(operation.values[0]),
                4 => TransformOperation::Matrix(Transform2D {
                    a: operation.values[0],
                    b: operation.values[1],
                    c: operation.values[2],
                    d: operation.values[3],
                    e: operation.values[4],
                    f: operation.values[5],
                }),
                _ => return Err(invalid("unknown transform operation kind")),
            });
        }
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Transform(TransformList(result))),
        )
    })
}

fn c_length(value: LengthValue) -> OuiLength {
    match value {
        LengthValue::Computed(value) if value.is_auto() => OuiLength {
            value: 0.0,
            unit: 6,
        },
        LengthValue::Computed(value) if value.is_none() => OuiLength {
            value: 0.0,
            unit: 7,
        },
        LengthValue::Computed(value) if value.is_percent() => OuiLength {
            value: value.value(),
            unit: 1,
        },
        LengthValue::Computed(value) => OuiLength {
            value: value.value(),
            unit: 0,
        },
        LengthValue::Em(value) => OuiLength { value, unit: 2 },
        LengthValue::Rem(value) => OuiLength { value, unit: 3 },
        LengthValue::ViewportWidth(value) => OuiLength { value, unit: 4 },
        LengthValue::ViewportHeight(value) => OuiLength { value, unit: 5 },
        LengthValue::ViewportMin(value) => OuiLength { value, unit: 8 },
        LengthValue::ViewportMax(value) => OuiLength { value, unit: 9 },
    }
}

fn c_enum(value: &StyleValue) -> Option<i32> {
    use openui_style::{ContentDistribution, ContentPosition, ItemPosition, OverflowAlignment};
    Some(match value {
        StyleValue::Display(value) => *value as i32,
        StyleValue::Position(value) => *value as i32,
        StyleValue::Overflow(value) => *value as i32,
        StyleValue::FlexDirection(value) => *value as i32,
        StyleValue::FlexWrap(value) => *value as i32,
        StyleValue::ItemAlignment(value) if value.overflow == OverflowAlignment::Default => {
            match value.position {
                ItemPosition::Normal => 0,
                ItemPosition::Stretch => 1,
                ItemPosition::Center => 2,
                ItemPosition::Start => 3,
                ItemPosition::End => 4,
                ItemPosition::FlexStart => 5,
                ItemPosition::FlexEnd => 6,
                ItemPosition::Baseline => 7,
                ItemPosition::Auto => 8,
                ItemPosition::SelfStart => 9,
                ItemPosition::SelfEnd => 10,
                ItemPosition::Left => 11,
                ItemPosition::Right => 12,
                ItemPosition::LastBaseline => 13,
                ItemPosition::Legacy => 14,
            }
        }
        StyleValue::ContentAlignment(value) if value.overflow == OverflowAlignment::Default => {
            match (value.position, value.distribution) {
                (ContentPosition::Normal, ContentDistribution::Default) => 0,
                (ContentPosition::Start, ContentDistribution::Default) => 1,
                (ContentPosition::End, ContentDistribution::Default) => 2,
                (ContentPosition::Center, ContentDistribution::Default) => 3,
                (ContentPosition::FlexStart, ContentDistribution::Default) => 4,
                (ContentPosition::FlexEnd, ContentDistribution::Default) => 5,
                (ContentPosition::Normal, ContentDistribution::SpaceBetween) => 6,
                (ContentPosition::Normal, ContentDistribution::SpaceAround) => 7,
                (ContentPosition::Normal, ContentDistribution::SpaceEvenly) => 8,
                (ContentPosition::Normal, ContentDistribution::Stretch) => 9,
                (ContentPosition::Baseline, ContentDistribution::Default) => 10,
                (ContentPosition::LastBaseline, ContentDistribution::Default) => 11,
                (ContentPosition::Left, ContentDistribution::Default) => 12,
                (ContentPosition::Right, ContentDistribution::Default) => 13,
                _ => return None,
            }
        }
        StyleValue::Cursor(value) => *value as i32,
        StyleValue::ListStyle(value) => *value as i32,
        StyleValue::PointerEvents(value) => *value as i32,
        _ => return None,
    })
}

// SAFETY CONTRACT: `literal` is readable and `out_value` points to one
// writable tagged-value record. A returned compound payload is caller-owned.
#[no_mangle]
pub extern "C" fn oui_style_value_parse(
    property: i32,
    literal: OuiUtf8,
    out_value: *mut OuiStyleValue,
) -> OuiStatus {
    ffi(|| {
        if out_value.is_null() {
            return Err(invalid("style value output is null"));
        }
        let property: StyleProperty = property_from_raw(property)
            .ok_or_else(|| invalid("unknown style property identifier"))?;
        let literal = utf8(literal, "style literal")?;
        let parsed =
            parse_literal(property, &literal).map_err(|error| invalid(error.to_string()))?;
        let tag = generated::expected_value_tag(property);
        let native_value = || -> Result<_, ApiError> {
            Ok(OuiStylePayload {
                compound: register(LocalHandle::PropertyCompound(property, parsed.clone()))?
                    as *const OuiStyleCompound,
            })
        };
        let (tag, data) = match (tag, &parsed) {
            (1, StyleValue::Length(value)) => (
                1,
                OuiStylePayload {
                    length: c_length(*value),
                },
            ),
            (2, StyleValue::Number(value)) => (2, OuiStylePayload { number: *value }),
            (2, StyleValue::FontWeight(value)) => (2, OuiStylePayload { number: value.0 }),
            (3, StyleValue::Integer(value)) => (3, OuiStylePayload { integer: *value }),
            (3, StyleValue::Renderer(openui_style::RendererStyleValue::ColumnCount(value))) => (
                3,
                OuiStylePayload {
                    integer: match value {
                        None => 0,
                        Some(count) => i32::try_from(*count)
                            .map_err(|_| invalid("column count exceeds the C integer range"))?,
                    },
                },
            ),
            (
                3,
                StyleValue::Renderer(
                    openui_style::RendererStyleValue::Orphans(value)
                    | openui_style::RendererStyleValue::Widows(value),
                ),
            ) => match i32::try_from(*value) {
                Ok(value) => (3, OuiStylePayload { integer: value }),
                Err(_) => (6, native_value()?),
            },
            (4, StyleValue::Color(value)) => (
                4,
                OuiStylePayload {
                    color: OuiColor {
                        red: (value.r * 255.0).round() as u8,
                        green: (value.g * 255.0).round() as u8,
                        blue: (value.b * 255.0).round() as u8,
                        alpha: (value.a * 255.0).round() as u8,
                    },
                },
            ),
            (5, value) => match c_enum(value) {
                Some(value) => (5, OuiStylePayload { enum_value: value }),
                None => (6, native_value()?),
            },
            // Renderer longhands own one property; retain that identity in
            // the carrier so a different compound property rejects it before
            // touching the retained document.
            (6, StyleValue::Renderer(_)) => (6, native_value()?),
            (6, _) => (
                6,
                OuiStylePayload {
                    compound: register(LocalHandle::Compound(parsed.clone()))?
                        as *const OuiStyleCompound,
                },
            ),
            _ => (6, native_value()?),
        };
        // SAFETY: the caller promises writable storage and null was rejected.
        unsafe {
            ptr::write(
                out_value,
                OuiStyleValue {
                    tag,
                    reserved: 0,
                    data,
                },
            )
        };
        Ok(())
    })
}

// SAFETY CONTRACT: `out_value` points to one writable tagged-value record.
// The returned property-bound compound is owned by the calling thread.
#[no_mangle]
pub extern "C" fn oui_style_value_color_f32_v1(
    property: i32,
    red: f32,
    green: f32,
    blue: f32,
    alpha: f32,
    out_value: *mut OuiStyleValue,
) -> OuiStatus {
    ffi(|| {
        if out_value.is_null() {
            return Err(invalid("style value output is null"));
        }
        let property = property_from_raw(property)
            .ok_or_else(|| invalid("unknown style property identifier"))?;
        if ![red, green, blue, alpha]
            .into_iter()
            .all(|channel| channel.is_finite() && (0.0..=1.0).contains(&channel))
        {
            return Err(invalid(
                "color channels must be finite and in the range 0..=1",
            ));
        }
        let color = Color::from_rgba_f32(red, green, blue, alpha);
        let value = match property {
            StyleProperty::TextDecorationColor => {
                StyleValue::Renderer(openui_style::RendererStyleValue::TextDecorationColor(
                    openui_style::StyleColor::Resolved(color),
                ))
            }
            StyleProperty::TextEmphasisColor => {
                StyleValue::Renderer(openui_style::RendererStyleValue::TextEmphasisColor(
                    openui_style::StyleColor::Resolved(color),
                ))
            }
            _ if property.metadata().value_kind == openui_style::ValueKind::Color => {
                StyleValue::Color(color)
            }
            _ => {
                return Err(ApiError::new(
                    OuiStatus::WrongValueType,
                    "property does not accept a color value",
                )
                .detail(property as u32));
            }
        };
        let compound = register(LocalHandle::PropertyCompound(property, value))?;
        // SAFETY: the caller promises writable storage; null was rejected.
        // All validation and allocation have succeeded before changing it.
        unsafe {
            ptr::write(
                out_value,
                OuiStyleValue {
                    tag: 6,
                    reserved: 0,
                    data: OuiStylePayload {
                        compound: compound as *const OuiStyleCompound,
                    },
                },
            );
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `compound` is a live immutable compound handle.
#[no_mangle]
pub extern "C" fn oui_style_compound_destroy(compound: *mut OuiStyleCompound) -> OuiStatus {
    ffi(|| {
        destroy(compound as usize, HandleKind::Compound)?;
        Ok(())
    })
}

// SAFETY CONTRACT: document is live; descriptor and byte slice are readable
// for the duration of this call; `out_face` is writable.
#[no_mangle]
pub extern "C" fn oui_document_register_font(
    document_handle: *mut OuiDocument,
    data: *const u8,
    byte_length: usize,
    descriptor: *const OuiFontFaceDescriptor,
    out_face: *mut *mut OuiFontFace,
) -> OuiStatus {
    ffi(|| {
        if byte_length == 0 || byte_length > 64 * 1024 * 1024 {
            return Err(invalid("font byte length is outside the supported range"));
        }
        if out_face.is_null() {
            return Err(invalid("font face output handle is null"));
        }
        let state = document(document_handle as usize)?;
        let descriptor = font_descriptor(descriptor)?;
        let owned: Arc<[u8]> = bytes(data, byte_length)?.into();
        let face = borrow_engine_mut(&state)?.register_font_face(owned, descriptor)?;
        write_handle(
            out_face,
            LocalHandle::FontFace(FontFaceRef {
                document: Rc::downgrade(&state),
                face,
            }),
        )
    })
}

// SAFETY CONTRACT: face is live and `out_info` is writable.
#[no_mangle]
pub extern "C" fn oui_font_face_get_info(
    face_handle: *mut OuiFontFace,
    out_info: *mut OuiFontFaceInfo,
) -> OuiStatus {
    ffi(|| {
        if out_info.is_null() {
            return Err(invalid("font info output is null"));
        }
        // SAFETY: caller supplies readable/writable storage for the header.
        let output = unsafe { &mut *out_info };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiFontFaceInfo>(),
        )?;
        let face = font_face_ref(face_handle as usize)?;
        let state = font_face_document(&face)?;
        let engine = borrow_engine(&state)?;
        let info = engine.font_face_info(face.face)?;
        let (style, style_min, style_max) = match info.descriptor.style {
            FontStyleRange::Normal => (OUI_FONT_FACE_STYLE_NORMAL, 0.0, 0.0),
            FontStyleRange::Italic => (OUI_FONT_FACE_STYLE_ITALIC, 0.0, 0.0),
            FontStyleRange::Oblique(range) => (OUI_FONT_FACE_STYLE_OBLIQUE, range.min, range.max),
        };
        let mut flags = 0;
        if info.descriptor.size_adjust.is_some() {
            flags |= OUI_FONT_FACE_HAS_SIZE_ADJUST;
        }
        if info.descriptor.metric_overrides.ascent.is_some() {
            flags |= OUI_FONT_FACE_HAS_ASCENT_OVERRIDE;
        }
        if info.descriptor.metric_overrides.descent.is_some() {
            flags |= OUI_FONT_FACE_HAS_DESCENT_OVERRIDE;
        }
        if info.descriptor.metric_overrides.line_gap.is_some() {
            flags |= OUI_FONT_FACE_HAS_LINE_GAP_OVERRIDE;
        }
        *output = OuiFontFaceInfo {
            struct_size: size_of::<OuiFontFaceInfo>() as u32,
            abi_version: OUI_ABI_VERSION,
            collection_id: info.handle.collection_id(),
            face_id: info.handle.face_id(),
            collection_generation: engine.font_collection().generation(),
            byte_length: info.byte_length,
            format: match info.format {
                FontContainerFormat::Ttf => OUI_FONT_CONTAINER_TTF,
                FontContainerFormat::Otf => OUI_FONT_CONTAINER_OTF,
                FontContainerFormat::Collection => OUI_FONT_CONTAINER_COLLECTION,
                FontContainerFormat::Woff => OUI_FONT_CONTAINER_WOFF,
                FontContainerFormat::Woff2 => OUI_FONT_CONTAINER_WOFF2,
            },
            face_index: info.descriptor.face_index,
            style,
            flags,
            style_min,
            style_max,
            weight_min: info.descriptor.weight.min,
            weight_max: info.descriptor.weight.max,
            stretch_min: info.descriptor.stretch.min,
            stretch_max: info.descriptor.stretch.max,
            size_adjust: info.descriptor.size_adjust.unwrap_or(0.0),
            ascent_override: info.descriptor.metric_overrides.ascent.unwrap_or(0.0),
            descent_override: info.descriptor.metric_overrides.descent.unwrap_or(0.0),
            line_gap_override: info.descriptor.metric_overrides.line_gap.unwrap_or(0.0),
            family_length: info.descriptor.family.len(),
            unicode_range_count: info.descriptor.unicode_ranges.len(),
            feature_default_count: info.descriptor.feature_defaults.len(),
            sha256: info.sha256,
        };
        Ok(())
    })
}

// SAFETY CONTRACT: face is live; destination is writable for capacity bytes.
#[no_mangle]
pub extern "C" fn oui_font_face_copy_family(
    face_handle: *mut OuiFontFace,
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
) -> OuiStatus {
    ffi(|| {
        let face = font_face_ref(face_handle as usize)?;
        let state = font_face_document(&face)?;
        let info = borrow_engine(&state)?.font_face_info(face.face)?;
        copy_bytes_to_c(
            info.descriptor.family.as_bytes(),
            destination,
            capacity,
            out_length,
            "font family",
        )
    })
}

// SAFETY CONTRACT: face is live; destination is writable for capacity elements.
#[no_mangle]
pub extern "C" fn oui_font_face_copy_unicode_ranges(
    face_handle: *mut OuiFontFace,
    destination: *mut OuiFontUnicodeRange,
    capacity: usize,
    out_count: *mut usize,
) -> OuiStatus {
    ffi(|| {
        let face = font_face_ref(face_handle as usize)?;
        let state = font_face_document(&face)?;
        let ranges = borrow_engine(&state)?
            .font_face_info(face.face)?
            .descriptor
            .unicode_ranges
            .iter()
            .map(|range| OuiFontUnicodeRange {
                start: range.start,
                end: range.end,
            })
            .collect::<Vec<_>>();
        copy_array_to_c(
            &ranges,
            destination,
            capacity,
            out_count,
            "font Unicode range",
        )
    })
}

// SAFETY CONTRACT: face is live; destination is writable for capacity elements.
#[no_mangle]
pub extern "C" fn oui_font_face_copy_feature_defaults(
    face_handle: *mut OuiFontFace,
    destination: *mut OuiFontFeatureDefault,
    capacity: usize,
    out_count: *mut usize,
) -> OuiStatus {
    ffi(|| {
        let face = font_face_ref(face_handle as usize)?;
        let state = font_face_document(&face)?;
        let features = borrow_engine(&state)?
            .font_face_info(face.face)?
            .descriptor
            .feature_defaults
            .iter()
            .map(|feature| OuiFontFeatureDefault {
                tag: feature.tag,
                value: feature.value,
            })
            .collect::<Vec<_>>();
        copy_array_to_c(
            &features,
            destination,
            capacity,
            out_count,
            "font feature default",
        )
    })
}

// SAFETY CONTRACT: face is a live registered font handle.
#[no_mangle]
pub extern "C" fn oui_font_face_unregister(face_handle: *mut OuiFontFace) -> OuiStatus {
    ffi(|| {
        let face = font_face_ref(face_handle as usize)?;
        let state = font_face_document(&face)?;
        borrow_engine_mut(&state)?.unregister_font_face(face.face)?;
        Ok(())
    })
}

// SAFETY CONTRACT: face is a live font handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_font_face_destroy(face_handle: *mut OuiFontFace) -> OuiStatus {
    ffi(|| {
        destroy(face_handle as usize, HandleKind::FontFace)?;
        Ok(())
    })
}

// SAFETY CONTRACT: document is live; all slices are readable for their
// lengths; `out_resource` is writable. Input bytes are copied synchronously.
#[no_mangle]
pub extern "C" fn oui_document_register_image(
    document_handle: *mut OuiDocument,
    source: OuiUtf8,
    mime_type: OuiUtf8,
    sha256: OuiUtf8,
    data: *const u8,
    byte_length: usize,
    out_resource: *mut *mut OuiResource,
) -> OuiStatus {
    ffi(|| {
        if out_resource.is_null() {
            return Err(invalid("out_resource is null"));
        }
        let state = document(document_handle as usize)?;
        let source = utf8(source, "resource source")?;
        let mime_type = utf8(mime_type, "resource MIME type")?;
        let sha256 = utf8(sha256, "resource hash")?;
        let data = bytes(data, byte_length)?;
        let resource =
            borrow_engine_mut(&state)?.register_image_resource(source, mime_type, sha256, data);
        write_handle(
            out_resource,
            LocalHandle::Resource(ResourceRef {
                document: Rc::downgrade(&state),
                resource,
            }),
        )
    })
}

// SAFETY CONTRACT: both handles are live; intrinsic dimensions are both zero
// (unspecified) or both finite and positive.
#[no_mangle]
pub extern "C" fn oui_element_set_image(
    element_handle: *mut OuiElement,
    resource_handle: *mut OuiResource,
    intrinsic_width: f32,
    intrinsic_height: f32,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let resource = match get(resource_handle as usize, HandleKind::Resource)? {
            LocalHandle::Resource(value) => value,
            _ => unreachable!("kind checked by registry"),
        };
        let resource_document = resource.document.upgrade().ok_or_else(|| {
            ApiError::new(OuiStatus::InvalidHandle, "resource document was destroyed")
        })?;
        if !Rc::ptr_eq(&state, &resource_document) {
            return Err(ApiError::new(
                OuiStatus::WrongDocument,
                "resource belongs to another document",
            ));
        }
        let dimensions = match (intrinsic_width, intrinsic_height) {
            (0.0, 0.0) => None,
            (width, height)
                if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 =>
            {
                Some((width, height))
            }
            _ => return Err(invalid("invalid intrinsic image dimensions")),
        };
        borrow_engine_mut(&state)?.set_image_resource(
            element.node,
            resource.resource,
            dimensions,
        )?;
        Ok(())
    })
}

// SAFETY CONTRACT: `resource` is a live resource handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_resource_destroy(resource: *mut OuiResource) -> OuiStatus {
    ffi(|| {
        destroy(resource as usize, HandleKind::Resource)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `buffer` is a live buffer handle. Returned bytes remain
// readable until the buffer is destroyed and must not be mutated or freed.
#[no_mangle]
pub extern "C" fn oui_buffer_data(buffer: *const OuiBuffer) -> *const u8 {
    ffi_value(ptr::null(), || {
        match get(buffer as usize, HandleKind::Buffer)? {
            LocalHandle::Buffer(value) => Ok(value.as_ptr()),
            _ => unreachable!("kind checked by registry"),
        }
    })
}

// SAFETY CONTRACT: `buffer` is a live buffer handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_buffer_length(buffer: *const OuiBuffer) -> usize {
    ffi_value(0, || match get(buffer as usize, HandleKind::Buffer)? {
        LocalHandle::Buffer(value) => Ok(value.len()),
        _ => unreachable!("kind checked by registry"),
    })
}

// SAFETY CONTRACT: `buffer` is a live buffer handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_buffer_destroy(buffer: *mut OuiBuffer) -> OuiStatus {
    ffi(|| {
        destroy(buffer as usize, HandleKind::Buffer)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use openui_style::StyleProperty;
    use std::mem::{align_of, size_of};

    fn empty_utf8() -> OuiUtf8 {
        OuiUtf8 {
            data: ptr::null(),
            length: 0,
        }
    }

    fn text(value: &str) -> OuiUtf8 {
        OuiUtf8 {
            data: value.as_ptr(),
            length: value.len(),
        }
    }

    fn document_config(width: u32, height: u32) -> OuiDocumentConfig {
        OuiDocumentConfig {
            struct_size: size_of::<OuiDocumentConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            viewport: OuiViewportMetrics {
                logical_width: f64::from(width),
                logical_height: f64::from(height),
                physical_width: width,
                physical_height: height,
                device_scale_factor: 1.0,
                authority: 1,
                reserved: 0,
            },
        }
    }

    fn create_document(width: u32, height: u32) -> *mut OuiDocument {
        let mut document = ptr::null_mut();
        assert_eq!(
            oui_document_create(&document_config(width, height), &mut document),
            OuiStatus::Ok
        );
        assert!(!document.is_null());
        document
    }

    fn create_element(
        document: *mut OuiDocument,
        tag: i32,
        parent: *mut OuiElement,
    ) -> *mut OuiElement {
        let mut element = ptr::null_mut();
        assert_eq!(
            oui_element_create(document, tag, &mut element),
            OuiStatus::Ok
        );
        if !parent.is_null() {
            assert_eq!(oui_element_append_child(parent, element), OuiStatus::Ok);
        }
        element
    }

    fn set_length(element: *mut OuiElement, property: i32, value: f32) -> OuiStatus {
        oui_element_set_property(
            element,
            property,
            &OuiStyleValue {
                tag: 1,
                reserved: 0,
                data: OuiStylePayload {
                    length: OuiLength { value, unit: 0 },
                },
            },
        )
    }

    fn event(event_type: u32, value: &str) -> OuiEvent {
        OuiEvent {
            struct_size: size_of::<OuiEvent>() as u32,
            abi_version: OUI_ABI_VERSION,
            event_type,
            phase: 0,
            flags: 0,
            modifiers: 0,
            timestamp_ns: 0,
            x: 0.0,
            y: 0.0,
            delta_x: 0.0,
            delta_y: 0.0,
            key_code: 0,
            pointer_id: 0,
            text: text(value),
            target: ptr::null_mut(),
            current_target: ptr::null_mut(),
        }
    }

    fn linear_easing() -> OuiEasing {
        OuiEasing {
            kind: 0,
            step_position: 0,
            step_count: 0,
            reserved: 0,
            values: [0.0; 4],
        }
    }

    fn animation_options(duration_ms: f64) -> OuiAnimationOptions {
        OuiAnimationOptions {
            struct_size: size_of::<OuiAnimationOptions>() as u32,
            abi_version: OUI_ABI_VERSION,
            delay_ms: 0.0,
            duration_ms,
            iterations: 1.0,
            playback_rate: 1.0,
            direction: 0,
            fill: 3,
            play_state: 0,
            composite: 0,
            easing: linear_easing(),
        }
    }

    fn number_keyframe(offset: f64, value: f32) -> OuiKeyframe {
        OuiKeyframe {
            offset,
            value: OuiStyleValue {
                tag: 2,
                reserved: 0,
                data: OuiStylePayload { number: value },
            },
            easing: linear_easing(),
            has_easing: 0,
            reserved: 0,
        }
    }

    #[test]
    fn frozen_layout_metadata_matches_rust() {
        assert_eq!((size_of::<OuiUtf8>(), align_of::<OuiUtf8>()), (16, 8));
        assert_eq!((size_of::<OuiLength>(), align_of::<OuiLength>()), (8, 4));
        assert_eq!((size_of::<OuiColor>(), align_of::<OuiColor>()), (4, 1));
        assert_eq!(
            (size_of::<OuiStylePayload>(), align_of::<OuiStylePayload>()),
            (8, 8)
        );
        assert_eq!(
            (size_of::<OuiStyleValue>(), align_of::<OuiStyleValue>()),
            (16, 8)
        );
        assert_eq!(
            (size_of::<OuiAppConfig>(), align_of::<OuiAppConfig>()),
            (40, 8)
        );
        assert_eq!(
            (
                size_of::<OuiViewportMetrics>(),
                align_of::<OuiViewportMetrics>()
            ),
            (40, 8)
        );
        assert_eq!(
            (
                size_of::<OuiDocumentConfig>(),
                align_of::<OuiDocumentConfig>()
            ),
            (48, 8)
        );
        assert_eq!(
            (
                size_of::<OuiScrollMetricsV1>(),
                align_of::<OuiScrollMetricsV1>()
            ),
            (40, 8)
        );
        assert_eq!(
            (
                size_of::<OuiFontUnicodeRange>(),
                align_of::<OuiFontUnicodeRange>()
            ),
            (8, 4)
        );
        assert_eq!(
            (
                size_of::<OuiFontFeatureDefault>(),
                align_of::<OuiFontFeatureDefault>()
            ),
            (8, 4)
        );
        assert_eq!(
            (
                size_of::<OuiFontFaceDescriptor>(),
                align_of::<OuiFontFaceDescriptor>()
            ),
            (112, 8)
        );
        assert_eq!(
            (size_of::<OuiFontFaceInfo>(), align_of::<OuiFontFaceInfo>()),
            (152, 8)
        );
        assert_eq!((size_of::<OuiRect>(), align_of::<OuiRect>()), (16, 4));
        assert_eq!((size_of::<OuiBitmap>(), align_of::<OuiBitmap>()), (32, 8));
        assert_eq!(
            (
                size_of::<OuiTransformOperation>(),
                align_of::<OuiTransformOperation>()
            ),
            (48, 4)
        );
        assert_eq!((size_of::<OuiEasing>(), align_of::<OuiEasing>()), (48, 8));
        assert_eq!(
            (
                size_of::<OuiAnimationOptions>(),
                align_of::<OuiAnimationOptions>()
            ),
            (104, 8)
        );
        assert_eq!(
            (
                size_of::<OuiAnimationTimeline>(),
                align_of::<OuiAnimationTimeline>()
            ),
            (40, 8)
        );
        assert_eq!(
            (size_of::<OuiKeyframe>(), align_of::<OuiKeyframe>()),
            (80, 8)
        );
        assert_eq!(
            (
                size_of::<OuiAnimationState>(),
                align_of::<OuiAnimationState>()
            ),
            (48, 8)
        );
        assert_eq!(
            (
                size_of::<OuiAnimationEvent>(),
                align_of::<OuiAnimationEvent>()
            ),
            (48, 8)
        );
        assert_eq!((size_of::<OuiEvent>(), align_of::<OuiEvent>()), (88, 8));
        assert_eq!(
            (
                size_of::<OuiAccessibilityUpdate>(),
                align_of::<OuiAccessibilityUpdate>()
            ),
            (40, 8)
        );
        assert_eq!(
            (
                size_of::<OuiAccessibilitySnapshotInfo>(),
                align_of::<OuiAccessibilitySnapshotInfo>()
            ),
            (56, 8)
        );
        assert_eq!(
            (
                size_of::<OuiAccessibilityNodeInfo>(),
                align_of::<OuiAccessibilityNodeInfo>()
            ),
            (120, 8)
        );
        assert_eq!(
            (
                size_of::<OuiAccessibilityNodeState>(),
                align_of::<OuiAccessibilityNodeState>()
            ),
            (112, 8)
        );
        assert_eq!(
            (size_of::<OuiErrorInfo>(), align_of::<OuiErrorInfo>()),
            (24, 8)
        );
    }

    #[test]
    fn c_font_registration_queries_and_removal_validate_lifetimes() {
        let document = create_document(64, 64);
        let family = "C application Ahem";
        let ranges = [OuiFontUnicodeRange {
            start: 0x20,
            end: 0x7e,
        }];
        let features = [OuiFontFeatureDefault {
            tag: *b"kern",
            value: 0,
        }];
        let descriptor = OuiFontFaceDescriptor {
            struct_size: size_of::<OuiFontFaceDescriptor>() as u32,
            abi_version: OUI_ABI_VERSION,
            family: text(family),
            face_index: 0,
            style: 2,
            style_min: -12.0,
            style_max: 18.0,
            weight_min: 300.0,
            weight_max: 700.0,
            stretch_min: 75.0,
            stretch_max: 125.0,
            unicode_ranges: ranges.as_ptr(),
            unicode_range_count: ranges.len(),
            feature_defaults: features.as_ptr(),
            feature_default_count: features.len(),
            size_adjust: 1.1,
            ascent_override: 0.8,
            descent_override: 0.2,
            line_gap_override: 0.1,
            flags: 0x0f,
            reserved: 0,
        };
        let font_bytes = include_bytes!("../../openui-text/fonts/Ahem.ttf");
        let mut face = ptr::null_mut();
        assert_eq!(
            oui_document_register_font(
                document,
                font_bytes.as_ptr(),
                font_bytes.len(),
                &descriptor,
                &mut face,
            ),
            OuiStatus::Ok
        );
        assert!(!face.is_null());

        let mut info = OuiFontFaceInfo {
            struct_size: size_of::<OuiFontFaceInfo>() as u32,
            abi_version: OUI_ABI_VERSION,
            collection_id: 0,
            face_id: 0,
            collection_generation: 0,
            byte_length: 0,
            format: 0,
            face_index: 0,
            style: 0,
            flags: 0,
            style_min: 0.0,
            style_max: 0.0,
            weight_min: 0.0,
            weight_max: 0.0,
            stretch_min: 0.0,
            stretch_max: 0.0,
            size_adjust: 0.0,
            ascent_override: 0.0,
            descent_override: 0.0,
            line_gap_override: 0.0,
            family_length: 0,
            unicode_range_count: 0,
            feature_default_count: 0,
            sha256: [0; 32],
        };
        assert_eq!(oui_font_face_get_info(face, &mut info), OuiStatus::Ok);
        assert_ne!(info.collection_id, 0);
        assert_ne!(info.face_id, 0);
        assert_eq!(info.byte_length, font_bytes.len());
        assert_eq!(info.format, OUI_FONT_CONTAINER_TTF);
        assert_eq!(info.style, OUI_FONT_FACE_STYLE_OBLIQUE);
        assert_eq!(info.flags, 0x0f);
        assert_eq!(info.family_length, family.len());
        assert_eq!(info.unicode_range_count, 1);
        assert_eq!(info.feature_default_count, 1);
        assert_ne!(info.sha256, [0; 32]);

        let mut family_length = 0;
        assert_eq!(
            oui_font_face_copy_family(face, ptr::null_mut(), 0, &mut family_length),
            OuiStatus::Ok
        );
        let mut copied_family = vec![0; family_length];
        assert_eq!(
            oui_font_face_copy_family(
                face,
                copied_family.as_mut_ptr(),
                copied_family.len(),
                &mut family_length,
            ),
            OuiStatus::Ok
        );
        assert_eq!(copied_family, family.as_bytes());

        let mut range_count = 0;
        assert_eq!(
            oui_font_face_copy_unicode_ranges(face, ptr::null_mut(), 0, &mut range_count),
            OuiStatus::Ok
        );
        let mut copied_ranges = vec![OuiFontUnicodeRange { start: 0, end: 0 }; range_count];
        assert_eq!(
            oui_font_face_copy_unicode_ranges(
                face,
                copied_ranges.as_mut_ptr(),
                copied_ranges.len(),
                &mut range_count,
            ),
            OuiStatus::Ok
        );
        assert_eq!(copied_ranges[0].start, ranges[0].start);
        assert_eq!(copied_ranges[0].end, ranges[0].end);

        let mut feature_count = 0;
        assert_eq!(
            oui_font_face_copy_feature_defaults(face, ptr::null_mut(), 0, &mut feature_count,),
            OuiStatus::Ok
        );
        let mut copied_features = vec![
            OuiFontFeatureDefault {
                tag: [0; 4],
                value: 0,
            };
            feature_count
        ];
        assert_eq!(
            oui_font_face_copy_feature_defaults(
                face,
                copied_features.as_mut_ptr(),
                copied_features.len(),
                &mut feature_count,
            ),
            OuiStatus::Ok
        );
        assert_eq!(copied_features[0].tag, features[0].tag);
        assert_eq!(copied_features[0].value, features[0].value);

        assert_eq!(oui_font_face_unregister(face), OuiStatus::Ok);
        assert_eq!(
            oui_font_face_get_info(face, &mut info),
            OuiStatus::StaleHandle
        );
        assert_eq!(oui_font_face_destroy(face), OuiStatus::Ok);
        assert_eq!(oui_font_face_destroy(face), OuiStatus::InvalidHandle);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_element_text_content_renders_and_matches_native_rust() {
        use openui::prelude::*;

        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let mut config = document_config(800, 600);
            config.viewport.device_scale_factor = scale;
            // Logical dimensions are authoritative; derive physical dimensions
            // at this scale instead of retaining the helper's scale-1 values.
            config.viewport.physical_width = 0;
            config.viewport.physical_height = 0;
            let mut c_document = ptr::null_mut();
            assert_eq!(oui_document_create(&config, &mut c_document), OuiStatus::Ok);
            let mut root = ptr::null_mut();
            assert_eq!(oui_document_root(c_document, &mut root), OuiStatus::Ok);
            let c_text = create_element(c_document, 0, root);

            let descriptor = OuiFontFaceDescriptor {
                struct_size: size_of::<OuiFontFaceDescriptor>() as u32,
                abi_version: OUI_ABI_VERSION,
                family: text("Ahem"),
                face_index: 0,
                style: OUI_FONT_FACE_STYLE_NORMAL,
                style_min: 0.0,
                style_max: 0.0,
                weight_min: 400.0,
                weight_max: 400.0,
                stretch_min: 100.0,
                stretch_max: 100.0,
                unicode_ranges: ptr::null(),
                unicode_range_count: 0,
                feature_defaults: ptr::null(),
                feature_default_count: 0,
                size_adjust: 0.0,
                ascent_override: 0.0,
                descent_override: 0.0,
                line_gap_override: 0.0,
                flags: 0,
                reserved: 0,
            };
            let bytes = include_bytes!("../../openui-text/fonts/Ahem.ttf");
            let mut face = ptr::null_mut();
            assert_eq!(
                oui_document_register_font(
                    c_document,
                    bytes.as_ptr(),
                    bytes.len(),
                    &descriptor,
                    &mut face,
                ),
                OuiStatus::Ok
            );
            let mut family = ptr::null_mut();
            assert_eq!(
                oui_font_family_create(text("Ahem"), 0, &mut family),
                OuiStatus::Ok
            );
            assert_eq!(
                oui_element_set_property(
                    c_text,
                    StyleProperty::FontFamily as i32,
                    &OuiStyleValue {
                        tag: 6,
                        reserved: 0,
                        data: OuiStylePayload { compound: family },
                    },
                ),
                OuiStatus::Ok
            );
            assert_eq!(oui_style_compound_destroy(family), OuiStatus::Ok);
            for (property, literal) in [
                (StyleProperty::Position, "absolute"),
                (StyleProperty::Left, "20px"),
                (StyleProperty::Top, "20px"),
                (StyleProperty::FontSize, "20px"),
                (StyleProperty::LineHeight, "1"),
            ] {
                let mut value = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
                assert_eq!(
                    oui_style_value_parse(property as i32, text(literal), value.as_mut_ptr()),
                    OuiStatus::Ok
                );
                // SAFETY: the successful constructor initializes the value.
                let value = unsafe { value.assume_init() };
                assert_eq!(
                    oui_element_set_property(c_text, property as i32, &value),
                    OuiStatus::Ok
                );
                if value.tag == 6 {
                    // SAFETY: the tag selects this initialized union member.
                    assert_eq!(
                        oui_style_compound_destroy(unsafe { value.data.compound } as *mut _),
                        OuiStatus::Ok
                    );
                }
            }

            let rust_document = openui::Document::with_font_collection(
                ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
                FontCollection::deterministic_test(),
            )
            .unwrap();
            let rust_text = openui::Element::create(&rust_document, "div").unwrap();
            rust_text.set_position(Position::Absolute).unwrap();
            rust_text.set_left(Length::px(20.0)).unwrap();
            rust_text.set_top(Length::px(20.0)).unwrap();
            rust_text
                .set_font_family(FontFamilyList::single("Ahem"))
                .unwrap();
            rust_text.set_font_size(LengthValue::px(20.0)).unwrap();
            rust_text.set_line_height(LineHeight::Number(1.0)).unwrap();
            rust_document.body().append_child(&rust_text).unwrap();
            for content in ["X", "XX", "", "X"] {
                rust_text.set_text(content).unwrap();
                assert_eq!(oui_element_set_text(c_text, text(content)), OuiStatus::Ok);
                let mut actual = OuiRect::default();
                assert_eq!(oui_element_get_bounds(c_text, &mut actual), OuiStatus::Ok);
                let expected = rust_text.bounding_rect().unwrap().unwrap();
                assert_eq!(
                    (actual.x, actual.y, actual.width, actual.height),
                    (expected.x, expected.y, expected.width, expected.height),
                    "C text setter must create authored text children: scale={scale}, text={content:?}"
                );
            }
            assert_eq!(oui_element_destroy(c_text), OuiStatus::Ok);
            assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
            assert_eq!(oui_font_face_destroy(face), OuiStatus::Ok);
            assert_eq!(oui_document_destroy(c_document), OuiStatus::Ok);
        }
    }

    #[test]
    fn c_font_metadata_matches_pinned_multiformat_fixtures() {
        fn hash(hex: &str) -> [u8; 32] {
            let mut result = [0_u8; 32];
            for (index, byte) in result.iter_mut().enumerate() {
                *byte = u8::from_str_radix(&hex[index * 2..index * 2 + 2], 16).unwrap();
            }
            result
        }

        let fixtures: &[(&str, &[u8], u32, &str)] = &[
            (
                "C certification TTF",
                include_bytes!("../../openui-text/fonts/certification/fixture.ttf"),
                OUI_FONT_CONTAINER_TTF,
                "b719ecb31c5b21fc573c03f6421c74ac63c271a5a3ff841e34f9705fb94b8448",
            ),
            (
                "C certification OTF",
                include_bytes!("../../openui-text/fonts/certification/fixture.otf"),
                OUI_FONT_CONTAINER_OTF,
                "235f7207a202026a0a73c38c64713d58eace082bdf605f5abd2a28166fab61aa",
            ),
            (
                "C certification WOFF",
                include_bytes!("../../openui-text/fonts/certification/fixture.woff"),
                OUI_FONT_CONTAINER_WOFF,
                "4e0e5135fa60f01e456e74ea50af23537d39cfc7822ab01dfa499a6efd8544e0",
            ),
            (
                "C certification WOFF2",
                include_bytes!("../../openui-text/fonts/certification/fixture.woff2"),
                OUI_FONT_CONTAINER_WOFF2,
                "984e2d1e7062a65a5ac746f38520a9ce0e1058e33533bfddd243bb5cfba061d4",
            ),
            (
                "C certification TTC",
                include_bytes!("../../openui-text/fonts/certification/fixture.ttc"),
                OUI_FONT_CONTAINER_COLLECTION,
                "29456016c3d05578b3702eb38258f2872e82048c811ae7e5927406ec8048dc55",
            ),
            (
                "C certification OTC",
                include_bytes!("../../openui-text/fonts/certification/fixture.otc"),
                OUI_FONT_CONTAINER_COLLECTION,
                "d646722927f94b620e35e3ed3a99ad34e856b400dd2fb191a51d3d357a97d216",
            ),
        ];
        let document = create_document(64, 64);
        for (family, bytes, expected_format, expected_hash) in fixtures {
            let descriptor = OuiFontFaceDescriptor {
                struct_size: size_of::<OuiFontFaceDescriptor>() as u32,
                abi_version: OUI_ABI_VERSION,
                family: text(family),
                face_index: 0,
                style: OUI_FONT_FACE_STYLE_NORMAL,
                style_min: 0.0,
                style_max: 0.0,
                weight_min: 400.0,
                weight_max: 400.0,
                stretch_min: 100.0,
                stretch_max: 100.0,
                unicode_ranges: ptr::null(),
                unicode_range_count: 0,
                feature_defaults: ptr::null(),
                feature_default_count: 0,
                size_adjust: 0.0,
                ascent_override: 0.0,
                descent_override: 0.0,
                line_gap_override: 0.0,
                flags: 0,
                reserved: 0,
            };
            let mut face = ptr::null_mut();
            assert_eq!(
                oui_document_register_font(
                    document,
                    bytes.as_ptr(),
                    bytes.len(),
                    &descriptor,
                    &mut face,
                ),
                OuiStatus::Ok,
                "{family}",
            );
            let mut info = OuiFontFaceInfo {
                struct_size: size_of::<OuiFontFaceInfo>() as u32,
                abi_version: OUI_ABI_VERSION,
                collection_id: 0,
                face_id: 0,
                collection_generation: 0,
                byte_length: 0,
                format: 0,
                face_index: u32::MAX,
                style: 0,
                flags: 0,
                style_min: 0.0,
                style_max: 0.0,
                weight_min: 0.0,
                weight_max: 0.0,
                stretch_min: 0.0,
                stretch_max: 0.0,
                size_adjust: 0.0,
                ascent_override: 0.0,
                descent_override: 0.0,
                line_gap_override: 0.0,
                family_length: 0,
                unicode_range_count: 0,
                feature_default_count: 0,
                sha256: [0; 32],
            };
            assert_eq!(oui_font_face_get_info(face, &mut info), OuiStatus::Ok);
            assert_eq!(info.format, *expected_format, "{family}");
            assert_eq!(info.face_index, 0, "{family}");
            assert_eq!(info.byte_length, bytes.len(), "{family}");
            assert_eq!(info.sha256, hash(expected_hash), "{family}");
            assert_eq!(oui_font_face_unregister(face), OuiStatus::Ok);
            assert_eq!(oui_font_face_destroy(face), OuiStatus::Ok);
        }
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_animation_api_samples_controls_and_reports_events() {
        let document = create_document(100, 100);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let element = create_element(document, 1, root);
        let keyframes = [number_keyframe(0.0, 0.0), number_keyframe(1.0, 1.0)];
        let options = animation_options(100.0);
        let mut animation_id = 0;
        assert_eq!(
            oui_element_animate(
                element,
                StyleProperty::Opacity as i32,
                keyframes.as_ptr(),
                keyframes.len(),
                &options,
                ptr::null(),
                &mut animation_id,
            ),
            OuiStatus::Ok
        );
        assert_ne!(animation_id, 0);
        let mut animating = 0;
        assert_eq!(
            oui_document_is_animating(document, &mut animating),
            OuiStatus::Ok
        );
        assert_eq!(animating, 1);
        assert_eq!(
            oui_document_set_animation_time(document, 50.0),
            OuiStatus::Ok
        );
        let mut state = OuiAnimationState {
            struct_size: size_of::<OuiAnimationState>() as u32,
            abi_version: OUI_ABI_VERSION,
            animation_id: 0,
            current_time_ms: 0.0,
            iteration: 0,
            property: 0,
            play_state: 0,
            phase: 0,
            finished: 0,
            reserved: [0; 3],
        };
        assert_eq!(
            oui_document_animation_get_state(document, animation_id, &mut state),
            OuiStatus::Ok
        );
        assert_eq!(state.current_time_ms, 50.0);
        assert_eq!(state.phase, 1);
        assert_eq!(
            oui_document_animation_pause(document, animation_id),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_document_animation_seek(document, animation_id, 100.0),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_document_animation_get_state(document, animation_id, &mut state),
            OuiStatus::Ok
        );
        assert_eq!(state.finished, 1);
        let mut output_event = OuiAnimationEvent {
            struct_size: size_of::<OuiAnimationEvent>() as u32,
            abi_version: OUI_ABI_VERSION,
            animation_id: 0,
            kind: 0,
            property: 0,
            elapsed_time_ms: 0.0,
            iteration: 0,
            target: ptr::null_mut(),
        };
        let mut has_event = 0;
        assert_eq!(
            oui_document_take_animation_event(document, &mut output_event, &mut has_event),
            OuiStatus::Ok
        );
        assert_eq!(has_event, 1);
        assert_eq!(output_event.animation_id, animation_id);
        assert_eq!(output_event.property, StyleProperty::Opacity as i32);
        assert_eq!(
            oui_document_animation_cancel(document, animation_id),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_destroy(element), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn malformed_inputs_report_structured_thread_local_errors() {
        assert_eq!(
            oui_document_create(ptr::null(), ptr::null_mut()),
            OuiStatus::InvalidArgument
        );
        let mut info = OuiErrorInfo {
            struct_size: size_of::<OuiErrorInfo>() as u32,
            abi_version: OUI_ABI_VERSION,
            status: 0,
            detail: 0,
            message_length: 0,
        };
        assert_eq!(oui_error_get_last(&mut info), OuiStatus::Ok);
        assert_eq!(info.status, OuiStatus::InvalidArgument as i32);
        let mut length = 0;
        assert_eq!(
            oui_error_copy_message(ptr::null_mut(), 0, &mut length),
            OuiStatus::Ok
        );
        let mut message = vec![0; length];
        assert_eq!(
            oui_error_copy_message(message.as_mut_ptr(), message.len(), &mut length),
            OuiStatus::Ok
        );
        assert!(std::str::from_utf8(&message).unwrap().contains("config"));

        let mut bad = document_config(10, 10);
        bad.abi_version = 1;
        let mut output = ptr::null_mut();
        assert_eq!(
            oui_document_create(&bad, &mut output),
            OuiStatus::AbiMismatch
        );
    }

    #[test]
    fn typed_tree_renders_and_buffers_have_explicit_lifetimes() {
        let document = create_document(64, 64);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let div = create_element(document, 0, root);
        assert_eq!(set_length(div, 4, 32.0), OuiStatus::Ok);
        assert_eq!(set_length(div, 5, 32.0), OuiStatus::Ok);
        assert_eq!(
            oui_element_set_property(
                div,
                20,
                &OuiStyleValue {
                    tag: 4,
                    reserved: 0,
                    data: OuiStylePayload {
                        color: OuiColor {
                            red: 255,
                            green: 0,
                            blue: 0,
                            alpha: 255,
                        },
                    },
                },
            ),
            OuiStatus::Ok
        );
        let mut bitmap = OuiBitmap {
            struct_size: size_of::<OuiBitmap>() as u32,
            abi_version: OUI_ABI_VERSION,
            width: 0,
            height: 0,
            stride: 0,
            pixels: ptr::null_mut(),
        };
        assert_eq!(
            oui_document_render_rgba(document, &mut bitmap),
            OuiStatus::Ok
        );
        assert_eq!((bitmap.width, bitmap.height, bitmap.stride), (64, 64, 256));
        assert_eq!(oui_buffer_length(bitmap.pixels), 64 * 64 * 4);
        assert!(!oui_buffer_data(bitmap.pixels).is_null());
        assert_eq!(oui_buffer_destroy(bitmap.pixels), OuiStatus::Ok);
        assert_eq!(oui_buffer_length(bitmap.pixels), 0);

        let mut png = ptr::null_mut();
        assert_eq!(oui_document_render_png(document, &mut png), OuiStatus::Ok);
        let length = oui_buffer_length(png);
        assert!(length > 8);
        // SAFETY: the buffer API promises `length` readable bytes while `png` is live.
        let png_bytes = unsafe { std::slice::from_raw_parts(oui_buffer_data(png), length) };
        assert_eq!(&png_bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(oui_buffer_destroy(png), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(div), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn rust_and_c_paths_produce_identical_headless_pixels() {
        // Compare the consuming Rust and C application APIs, including their
        // native element defaults. Raw Engine construction intentionally keeps
        // CSS initial values for callers installing a complete resolved style.
        let rust_document = openui::Document::new(64, 64).unwrap();
        let direct = openui::Element::create(&rust_document, "div").unwrap();
        rust_document.body().append_child(&direct).unwrap();
        direct
            .set_property(
                openui_style::StyleProperty::Width,
                openui_style::LengthValue::px(32.0).into(),
            )
            .unwrap();
        direct
            .set_property(
                openui_style::StyleProperty::Height,
                openui_style::LengthValue::px(32.0).into(),
            )
            .unwrap();
        direct
            .set_property(
                openui_style::StyleProperty::BackgroundColor,
                Color::RED.into(),
            )
            .unwrap();
        let expected = rust_document.render_to_bitmap().unwrap().pixels().to_vec();

        let document = create_document(64, 64);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let div = create_element(document, 0, root);
        assert_eq!(set_length(div, 4, 32.0), OuiStatus::Ok);
        assert_eq!(set_length(div, 5, 32.0), OuiStatus::Ok);
        let red = OuiStyleValue {
            tag: 4,
            reserved: 0,
            data: OuiStylePayload {
                color: OuiColor {
                    red: 255,
                    green: 0,
                    blue: 0,
                    alpha: 255,
                },
            },
        };
        assert_eq!(oui_element_set_property(div, 20, &red), OuiStatus::Ok);
        let mut bitmap = OuiBitmap {
            struct_size: size_of::<OuiBitmap>() as u32,
            abi_version: OUI_ABI_VERSION,
            width: 0,
            height: 0,
            stride: 0,
            pixels: ptr::null_mut(),
        };
        assert_eq!(
            oui_document_render_rgba(document, &mut bitmap),
            OuiStatus::Ok
        );
        let length = oui_buffer_length(bitmap.pixels);
        // SAFETY: the live buffer exposes exactly `length` immutable bytes.
        let actual = unsafe { std::slice::from_raw_parts(oui_buffer_data(bitmap.pixels), length) };
        assert_eq!(actual, expected);
        assert_eq!(oui_buffer_destroy(bitmap.pixels), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(div), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_scroll_metrics_validate_output_headers_and_ownership() {
        let document_handle = create_document(32, 32);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document_handle, &mut root), OuiStatus::Ok);
        let mut output = OuiScrollMetricsV1 {
            struct_size: size_of::<OuiScrollMetricsV1>() as u32,
            abi_version: OUI_ABI_VERSION,
            client_width: -1.0,
            client_height: -1.0,
            scroll_width: -1.0,
            scroll_height: -1.0,
        };
        let initial = output;
        let mut found = 55;
        assert_eq!(
            oui_element_get_scroll_metrics_v1(root, ptr::null_mut(), &mut found),
            OuiStatus::InvalidArgument
        );
        assert_eq!(
            oui_element_get_scroll_metrics_v1(root, &mut output, ptr::null_mut()),
            OuiStatus::InvalidArgument
        );
        output.abi_version = 1;
        let wrong_version = output;
        assert_eq!(
            oui_element_get_scroll_metrics_v1(root, &mut output, &mut found),
            OuiStatus::AbiMismatch
        );
        assert_eq!(output, wrong_version);
        assert_eq!(found, 55);
        output = initial;
        let mut short_header = [8_u32, OUI_ABI_VERSION];
        assert_eq!(
            oui_element_get_scroll_metrics_v1(root, short_header.as_mut_ptr().cast(), &mut found),
            OuiStatus::InvalidArgument
        );
        assert_eq!(short_header, [8, OUI_ABI_VERSION]);
        assert_eq!(found, 55);
        let state = document(document_handle as usize).unwrap();
        let held = borrow_engine(&state).unwrap();
        assert_eq!(
            oui_element_get_scroll_metrics_v1(root, &mut output, &mut found),
            OuiStatus::Reentrant
        );
        assert_eq!(output, initial);
        assert_eq!(found, 55);
        drop(held);
        drop(state);

        let address = root as usize;
        let wrong_thread = std::thread::spawn(move || {
            let mut output = initial;
            let mut found = 55;
            let status = oui_element_get_scroll_metrics_v1(
                address as *mut OuiElement,
                &mut output,
                &mut found,
            );
            assert_eq!(output, initial);
            assert_eq!(found, 55);
            status
        })
        .join()
        .unwrap();
        assert_eq!(wrong_thread, OuiStatus::WrongThread);
        assert_eq!(oui_document_destroy(document_handle), OuiStatus::Ok);
        assert_eq!(
            oui_element_get_scroll_metrics_v1(root, &mut output, &mut found),
            OuiStatus::InvalidHandle
        );
        assert_eq!(output, initial);
        assert_eq!(found, 55);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
    }

    #[test]
    fn c_scroll_metrics_preserve_larger_caller_storage_and_absent_boxes() {
        #[repr(C)]
        struct Extended {
            metrics: OuiScrollMetricsV1,
            tail: u64,
        }
        let document = create_document(80, 50);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let mut output = Extended {
            metrics: OuiScrollMetricsV1 {
                struct_size: size_of::<Extended>() as u32,
                abi_version: OUI_ABI_VERSION,
                client_width: -1.0,
                client_height: -1.0,
                scroll_width: -1.0,
                scroll_height: -1.0,
            },
            tail: 0xdead_beef_1234_5678,
        };
        let mut found = 0;
        assert_eq!(
            oui_element_get_scroll_metrics_v1(root, &mut output.metrics, &mut found),
            OuiStatus::Ok
        );
        assert_eq!(found, 1);
        assert_eq!(output.metrics.client_width, 80.0);
        assert_eq!(output.metrics.client_height, 50.0);
        assert_eq!(output.metrics.scroll_width, 80.0);
        assert_eq!(output.metrics.scroll_height, 50.0);
        assert_eq!(output.metrics.struct_size, size_of::<Extended>() as u32);
        assert_eq!(output.tail, 0xdead_beef_1234_5678);
        let owned = output.metrics;
        let detached = create_element(document, 0, ptr::null_mut());
        assert_eq!(
            oui_element_get_scroll_metrics_v1(detached, &mut output.metrics, &mut found),
            OuiStatus::Ok
        );
        assert_eq!(found, 0);
        assert_eq!(output.metrics.client_width, 0.0);
        assert_eq!(output.metrics.client_height, 0.0);
        assert_eq!(output.metrics.scroll_width, 0.0);
        assert_eq!(output.metrics.scroll_height, 0.0);
        assert_eq!(output.tail, 0xdead_beef_1234_5678);
        assert_eq!(oui_element_remove(detached), OuiStatus::Ok);
        let absent = output.metrics;
        assert_eq!(
            oui_element_get_scroll_metrics_v1(detached, &mut output.metrics, &mut found),
            OuiStatus::StaleHandle
        );
        assert_eq!(output.metrics, absent);
        assert_eq!(found, 0);
        assert_eq!(oui_element_destroy(detached), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
        assert_eq!(owned.client_width, 80.0);
        assert_eq!(owned.client_height, 50.0);
    }

    #[test]
    fn app_and_document_handles_have_independent_explicit_lifetimes() {
        let title = "Open\0UI";
        let config = OuiAppConfig {
            struct_size: size_of::<OuiAppConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            title: text(title),
            width: 80,
            height: 40,
            backend: 2,
            reserved: 0,
        };
        let mut app = ptr::null_mut();
        assert_eq!(oui_app_create(&config, &mut app), OuiStatus::Ok);
        let mut document = ptr::null_mut();
        assert_eq!(oui_app_document(app, &mut document), OuiStatus::Ok);
        assert_eq!(oui_app_destroy(app), OuiStatus::Ok);
        assert_eq!(oui_document_update(document), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_id_lookup_returns_owned_attached_element_handles() {
        let document = create_document(64, 64);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let child = create_element(document, 0, root);
        assert_eq!(
            oui_element_set_attribute(child, text("id"), text("control")),
            OuiStatus::Ok
        );

        let mut found = ptr::null_mut();
        assert_eq!(
            oui_document_element_by_id(document, text("control"), &mut found),
            OuiStatus::Ok
        );
        assert!(!found.is_null());
        assert_ne!(found, child);
        assert_eq!(oui_element_detach(found), OuiStatus::Ok);

        let mut missing = found;
        assert_eq!(
            oui_document_element_by_id(document, text("control"), &mut missing),
            OuiStatus::Ok
        );
        assert!(missing.is_null());
        assert_eq!(oui_element_append_child(root, child), OuiStatus::Ok);
        assert_eq!(
            oui_document_element_by_id(document, text("control"), &mut missing),
            OuiStatus::Ok
        );
        assert!(!missing.is_null());

        assert_eq!(oui_element_destroy(found), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(missing), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(child), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn ownership_generation_and_value_tags_are_validated() {
        let first = create_document(32, 32);
        let second = create_document(32, 32);
        let first_element = create_element(first, 0, ptr::null_mut());
        let second_element = create_element(second, 0, ptr::null_mut());
        assert_eq!(
            oui_element_append_child(first_element, second_element),
            OuiStatus::WrongDocument
        );
        let wrong = OuiStyleValue {
            tag: 2,
            reserved: 0,
            data: OuiStylePayload { number: 12.0 },
        };
        assert_eq!(
            oui_element_set_property(first_element, 4, &wrong),
            OuiStatus::WrongValueType
        );
        assert_eq!(oui_element_remove(first_element), OuiStatus::Ok);
        assert_eq!(set_length(first_element, 4, 1.0), OuiStatus::StaleHandle);
        assert_eq!(oui_element_destroy(first_element), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(second_element), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(first), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(second), OuiStatus::Ok);
    }

    #[test]
    fn detached_c_nodes_keep_their_handles_until_explicit_removal() {
        let document = create_document(64, 64);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let parent = create_element(document, 0, root);
        let child = create_element(document, 0, parent);
        let mut text_node = ptr::null_mut();
        assert_eq!(
            oui_text_create(document, text("native"), &mut text_node),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_append_child(parent, text_node), OuiStatus::Ok);

        assert_eq!(oui_element_detach(parent), OuiStatus::Ok);
        assert_eq!(oui_element_detach(parent), OuiStatus::Ok);
        assert_eq!(set_length(child, 4, 20.0), OuiStatus::Ok);
        assert_eq!(oui_element_detach(text_node), OuiStatus::Ok);
        assert_eq!(oui_element_append_child(parent, text_node), OuiStatus::Ok);
        assert_eq!(oui_element_append_child(root, parent), OuiStatus::Ok);
        assert_eq!(oui_element_remove(parent), OuiStatus::Ok);
        assert_eq!(set_length(child, 4, 30.0), OuiStatus::StaleHandle);

        assert_eq!(oui_element_destroy(text_node), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(child), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(parent), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn transactions_and_compound_types_are_checked() {
        let document = create_document(32, 32);
        assert_eq!(oui_document_end_update(document), OuiStatus::InvalidState);
        assert_eq!(oui_document_begin_update(document), OuiStatus::Ok);
        assert_eq!(oui_document_update(document), OuiStatus::InvalidState);
        assert_eq!(oui_document_end_update(document), OuiStatus::Ok);

        let element = create_element(document, 0, ptr::null_mut());
        let edges = [OuiLength {
            value: 2.0,
            unit: 0,
        }; 4];
        let mut compound = ptr::null_mut();
        assert_eq!(
            oui_edges_create(edges.as_ptr(), &mut compound),
            OuiStatus::Ok
        );
        let compound_value = OuiStyleValue {
            tag: 6,
            reserved: 0,
            data: OuiStylePayload { compound },
        };
        assert_eq!(
            oui_element_set_property(element, 15, &compound_value),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_set_property(element, 37, &compound_value),
            OuiStatus::WrongValueType
        );
        assert_eq!(oui_style_compound_destroy(compound), OuiStatus::Ok);
        assert_eq!(
            oui_element_set_property(element, 15, &compound_value),
            OuiStatus::InvalidHandle
        );
        assert_eq!(oui_element_destroy(element), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    struct CallbackData {
        code: i32,
        log: *mut Vec<i32>,
        mutate: *mut OuiElement,
    }

    unsafe extern "C" fn record_event(event: *mut OuiEvent, user_data: *mut c_void) {
        // SAFETY: the test keeps both stack allocations alive through dispatch.
        let data = unsafe { &mut *(user_data as *mut CallbackData) };
        // SAFETY: dispatch supplies a live event for the callback duration.
        let event = unsafe { &mut *event };
        // SAFETY: `log` points to the live test vector and callbacks are synchronous.
        unsafe { &mut *data.log }.push(data.code * 10 + event.phase as i32);
        if !data.mutate.is_null() {
            assert_eq!(
                oui_element_set_text(data.mutate, text("updated")),
                OuiStatus::Ok
            );
        }
    }

    #[test]
    fn event_order_is_deterministic_and_callbacks_can_reenter() {
        let document = create_document(32, 32);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let target = create_element(document, 24, root);
        let mut log = Vec::new();
        let mut capture = CallbackData {
            code: 1,
            log: &mut log,
            mutate: ptr::null_mut(),
        };
        let mut at_target = CallbackData {
            code: 2,
            log: &mut log,
            mutate: target,
        };
        let mut bubble = CallbackData {
            code: 3,
            log: &mut log,
            mutate: ptr::null_mut(),
        };
        let mut first_listener = ptr::null_mut();
        let mut second_listener = ptr::null_mut();
        let mut third_listener = ptr::null_mut();
        assert_eq!(
            oui_element_add_event_listener(
                root,
                4,
                1,
                Some(record_event),
                &mut capture as *mut _ as *mut c_void,
                &mut first_listener,
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_add_event_listener(
                target,
                4,
                0,
                Some(record_event),
                &mut at_target as *mut _ as *mut c_void,
                &mut second_listener,
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_add_event_listener(
                root,
                4,
                0,
                Some(record_event),
                &mut bubble as *mut _ as *mut c_void,
                &mut third_listener,
            ),
            OuiStatus::Ok
        );
        let mut event = OuiEvent {
            struct_size: size_of::<OuiEvent>() as u32,
            abi_version: OUI_ABI_VERSION,
            event_type: 4,
            phase: 0,
            flags: 0,
            modifiers: 0,
            timestamp_ns: 7,
            x: 1.0,
            y: 1.0,
            delta_x: 0.0,
            delta_y: 0.0,
            key_code: 0,
            pointer_id: 0,
            text: empty_utf8(),
            target: ptr::null_mut(),
            current_target: ptr::null_mut(),
        };
        assert_eq!(
            oui_document_dispatch_event(document, target, &mut event),
            OuiStatus::Ok
        );
        assert_eq!(log, [11, 22, 33]);
        assert_eq!(event.target, target);
        assert!(event.current_target.is_null());
        assert_eq!(oui_listener_destroy(first_listener), OuiStatus::Ok);
        assert_eq!(oui_listener_destroy(second_listener), OuiStatus::Ok);
        assert_eq!(oui_listener_destroy(third_listener), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(target), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_events_hit_testing_and_control_state_share_engine_defaults() {
        let document = create_document(100, 100);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let checkbox = create_element(document, 23, root);
        assert_eq!(
            oui_element_set_property(
                checkbox,
                1,
                &OuiStyleValue {
                    tag: 5,
                    reserved: 0,
                    data: OuiStylePayload { enum_value: 2 },
                },
            ),
            OuiStatus::Ok
        );
        assert_eq!(set_length(checkbox, 4, 40.0), OuiStatus::Ok);
        assert_eq!(set_length(checkbox, 5, 40.0), OuiStatus::Ok);
        assert_eq!(
            oui_element_set_attribute(checkbox, text("type"), text("checkbox")),
            OuiStatus::Ok
        );
        let mut click = event(4, "");
        assert_eq!(
            oui_document_dispatch_event(document, checkbox, &mut click),
            OuiStatus::Ok
        );
        let mut flags = 0;
        assert_eq!(
            oui_element_get_control_flags(checkbox, &mut flags),
            OuiStatus::Ok
        );
        assert_ne!(flags & OUI_CONTROL_CHECKED, 0);

        assert_eq!(oui_element_set_checked(checkbox, 0), OuiStatus::Ok);
        let mut pointer = event(1, "");
        pointer.x = 10.0;
        pointer.y = 10.0;
        pointer.pointer_id = 7;
        assert_eq!(
            oui_document_dispatch_pointer_event(document, &mut pointer),
            OuiStatus::Ok
        );
        pointer.event_type = 2;
        assert_eq!(
            oui_document_dispatch_pointer_event(document, &mut pointer),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_get_control_flags(checkbox, &mut flags),
            OuiStatus::Ok
        );
        assert_ne!(flags & OUI_CONTROL_CHECKED, 0);

        let mut hit = ptr::null_mut();
        assert_eq!(
            oui_document_hit_test(document, 10.0, 10.0, &mut hit),
            OuiStatus::Ok
        );
        assert_eq!(hit, checkbox);

        let input = create_element(document, 23, root);
        assert_eq!(
            oui_element_set_control_value(input, text("a👩‍💻")),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_set_selection(input, 1, 1), OuiStatus::Ok);
        let mut input_event = event(8, "b");
        assert_eq!(
            oui_document_dispatch_event(document, input, &mut input_event),
            OuiStatus::Ok
        );
        let mut value_length = 0;
        assert_eq!(
            oui_element_copy_control_value(input, ptr::null_mut(), 0, &mut value_length),
            OuiStatus::Ok
        );
        let mut value = vec![0; value_length];
        assert_eq!(
            oui_element_copy_control_value(
                input,
                value.as_mut_ptr(),
                value.len(),
                &mut value_length,
            ),
            OuiStatus::Ok
        );
        assert_eq!(std::str::from_utf8(&value).unwrap(), "ab👩‍💻");

        assert_eq!(oui_element_destroy(input), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(checkbox), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn normalized_c_keyboard_and_text_share_cancelable_document_defaults() {
        struct Observations {
            target: *mut OuiElement,
            cancel_key: bool,
            cancel_text: bool,
            make_read_only: bool,
            events: Vec<(u32, String, String)>,
        }
        unsafe extern "C" fn observe(event: *mut OuiEvent, user_data: *mut c_void) {
            // SAFETY: the test owns both values through synchronous dispatch.
            let event = unsafe { &mut *event };
            let seen = unsafe { &mut *user_data.cast::<Observations>() };
            let mut value = [0; 64];
            let mut length = 0;
            assert_eq!(
                oui_element_copy_control_value(
                    seen.target,
                    value.as_mut_ptr(),
                    value.len(),
                    &mut length
                ),
                OuiStatus::Ok
            );
            let event_text = if event.text.length == 0 {
                String::new()
            } else {
                // SAFETY: event text remains readable through this callback.
                String::from_utf8(
                    unsafe { std::slice::from_raw_parts(event.text.data, event.text.length) }
                        .to_vec(),
                )
                .unwrap()
            };
            seen.events.push((
                event.event_type,
                event_text,
                String::from_utf8(value[..length].to_vec()).unwrap(),
            ));
            if (event.event_type == 6 && seen.cancel_key)
                || (event.event_type == 18 && seen.cancel_text)
            {
                event.flags |= OUI_EVENT_FLAG_DEFAULT_PREVENTED;
            }
            if event.event_type == 18 && seen.make_read_only {
                assert_eq!(
                    oui_element_set_attribute(seen.target, text("readonly"), empty_utf8()),
                    OuiStatus::Ok
                );
            }
        }
        fn value(target: *mut OuiElement) -> String {
            let mut bytes = [0; 64];
            let mut length = 0;
            assert_eq!(
                oui_element_copy_control_value(
                    target,
                    bytes.as_mut_ptr(),
                    bytes.len(),
                    &mut length
                ),
                OuiStatus::Ok
            );
            String::from_utf8(bytes[..length].to_vec()).unwrap()
        }
        let document = create_document(160, 100);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let textarea = create_element(document, 31, root);
        assert_eq!(
            oui_element_set_control_value(textarea, text("é👍z")),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_set_selection(textarea, 2, 6), OuiStatus::Ok);
        assert_eq!(oui_element_focus(textarea), OuiStatus::Ok);
        let mut seen = Box::new(Observations {
            target: textarea,
            cancel_key: false,
            cancel_text: false,
            make_read_only: false,
            events: Vec::new(),
        });
        let mut listeners = Vec::new();
        for kind in [6, 18, 16, 4] {
            let mut listener = ptr::null_mut();
            assert_eq!(
                oui_element_add_event_listener(
                    textarea,
                    kind,
                    0,
                    Some(observe),
                    (&mut *seen as *mut Observations).cast(),
                    &mut listener
                ),
                OuiStatus::Ok
            );
            listeners.push(listener);
        }
        let mut enter = event(6, "Enter");
        enter.key_code = 13;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &enter, text("\r")),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é\nz");
        assert_eq!(
            seen.events,
            vec![
                (6, "Enter".into(), "é👍z".into()),
                (18, "\n".into(), "é👍z".into()),
                (16, "\n".into(), "é\nz".into())
            ]
        );
        let mut anchor = 0;
        let mut focus = 0;
        assert_eq!(
            oui_element_get_selection(textarea, &mut anchor, &mut focus),
            OuiStatus::Ok
        );
        assert_eq!((anchor, focus), (3, 3));
        let mut key_up = event(7, "Enter");
        key_up.key_code = 13;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key_up, text("\r")),
            OuiStatus::Ok
        );
        let mut shortcut = event(6, "q");
        shortcut.key_code = 81;
        for modifiers in [2, 8] {
            shortcut.modifiers = modifiers;
            assert_eq!(
                oui_document_dispatch_key_input_v1(document, &shortcut, text("q")),
                OuiStatus::Ok
            );
            assert_eq!(value(textarea), "é\nz");
        }
        seen.events.clear();
        seen.cancel_key = true;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &enter, text("\r")),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é\nz");
        assert_eq!(seen.events.len(), 1);
        seen.cancel_key = false;
        seen.cancel_text = true;
        seen.events.clear();
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &enter, text("\r")),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é\nz");
        assert_eq!(seen.events.len(), 2);
        assert_eq!(
            oui_document_dispatch_text_input_v1(document, text("blocked")),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é\nz");
        seen.cancel_text = false;
        let mut undo = event(6, "z");
        undo.key_code = 90;
        undo.modifiers = 2;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &undo, text("z")),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é👍z");
        // Undo restores the value and collapses selection at the end. Establish
        // the intended replacement range through the native application API.
        assert_eq!(oui_element_set_selection(textarea, 2, 6), OuiStatus::Ok);
        seen.events.clear();
        assert_eq!(
            oui_document_dispatch_text_input_v1(document, text("!")),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é!z");
        assert_eq!(
            seen.events,
            vec![
                (18, "!".into(), "é👍z".into()),
                (16, "!".into(), "é!z".into())
            ]
        );
        seen.make_read_only = true;
        seen.events.clear();
        assert_eq!(
            oui_document_dispatch_text_input_v1(document, text("blocked")),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é!z");
        assert_eq!(seen.events.len(), 1);
        seen.events.clear();
        assert_eq!(
            oui_document_dispatch_text_input_v1(document, text("blocked")),
            OuiStatus::Ok
        );
        assert!(seen.events.is_empty());
        let mut backspace = event(6, "Backspace");
        backspace.key_code = 8;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &backspace, empty_utf8()),
            OuiStatus::Ok
        );
        assert_eq!(value(textarea), "é!z");
        for listener in listeners {
            assert_eq!(oui_listener_destroy(listener), OuiStatus::Ok);
        }
        assert_eq!(oui_element_destroy(textarea), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn normalized_c_input_validates_headers_utf8_and_thread_ownership() {
        let document = create_document(100, 100);
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, ptr::null(), empty_utf8()),
            OuiStatus::InvalidArgument
        );
        let short_size = 4_u32;
        assert_eq!(
            oui_document_dispatch_key_input_v1(
                document,
                (&short_size as *const u32).cast(),
                empty_utf8()
            ),
            OuiStatus::InvalidArgument
        );
        let short_header = [8_u32, OUI_ABI_VERSION];
        assert_eq!(
            oui_document_dispatch_key_input_v1(
                document,
                short_header.as_ptr().cast(),
                empty_utf8()
            ),
            OuiStatus::InvalidArgument
        );
        let mut key = event(6, "a");
        key.key_code = 65;
        key.abi_version = 1;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key, text("a")),
            OuiStatus::AbiMismatch
        );
        key.abi_version = OUI_ABI_VERSION;
        key.event_type = 4;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key, empty_utf8()),
            OuiStatus::InvalidArgument
        );
        key.event_type = 6;
        key.modifiers = 1 << 31;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key, empty_utf8()),
            OuiStatus::InvalidArgument
        );
        key.modifiers = 0;
        key.flags = OUI_EVENT_FLAG_DEFAULT_PREVENTED;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key, empty_utf8()),
            OuiStatus::InvalidArgument
        );
        key.flags = 0;
        let invalid_bytes = [0xff_u8];
        let invalid_text = OuiUtf8 {
            data: invalid_bytes.as_ptr(),
            length: 1,
        };
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key, invalid_text),
            OuiStatus::InvalidArgument
        );
        assert_eq!(
            oui_document_dispatch_text_input_v1(document, invalid_text),
            OuiStatus::InvalidArgument
        );
        key.text = invalid_text;
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key, empty_utf8()),
            OuiStatus::InvalidArgument
        );
        key.text = text("a");
        let address = document as usize;
        let wrong_thread = std::thread::spawn(move || {
            oui_document_dispatch_text_input_v1(address as *mut OuiDocument, text("a"))
        })
        .join()
        .unwrap();
        assert_eq!(wrong_thread, OuiStatus::WrongThread);
        assert_eq!(
            oui_document_dispatch_key_input_v1(document, &key, text("a")),
            OuiStatus::Ok
        );
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
        assert_eq!(
            oui_document_dispatch_text_input_v1(document, empty_utf8()),
            OuiStatus::InvalidHandle
        );
    }

    #[test]
    fn c_composition_dispatch_commits_final_text_and_restores_canceled_edits() {
        let document = create_document(100, 100);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let input = create_element(document, 23, root);
        assert_eq!(
            oui_element_set_control_value(input, text("kept")),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_set_selection(input, 0, 4), OuiStatus::Ok);
        for (kind, value) in [(9, ""), (10, "preview"), (10, ""), (11, "漢字")] {
            let mut event = event(kind, value);
            assert_eq!(
                oui_document_dispatch_event(document, input, &mut event),
                OuiStatus::Ok
            );
        }
        let mut bytes = vec![0; "漢字".len()];
        let mut written = 0;
        assert_eq!(
            oui_element_copy_control_value(input, bytes.as_mut_ptr(), bytes.len(), &mut written),
            OuiStatus::Ok
        );
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "漢字");
        for (kind, value) in [(9, ""), (10, "preview"), (11, "")] {
            let mut event = event(kind, value);
            assert_eq!(
                oui_document_dispatch_event(document, input, &mut event),
                OuiStatus::Ok
            );
        }
        assert_eq!(
            oui_element_copy_control_value(input, bytes.as_mut_ptr(), bytes.len(), &mut written),
            OuiStatus::Ok
        );
        assert_eq!(std::str::from_utf8(&bytes).unwrap(), "漢字");
        assert_eq!(oui_element_destroy(input), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_accessibility_updates_actions_and_validation_share_engine_state() {
        let document = create_document(100, 100);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let checkbox = create_element(document, 23, root);
        assert_eq!(set_length(checkbox, 4, 40.0), OuiStatus::Ok);
        assert_eq!(set_length(checkbox, 5, 40.0), OuiStatus::Ok);
        assert_eq!(
            oui_element_set_attribute(checkbox, text("type"), text("checkbox")),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_set_accessibility_label(checkbox, text("Ship")),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_set_accessibility_role(checkbox, 99),
            OuiStatus::InvalidArgument
        );
        let mut log = Vec::new();
        let mut click_data = CallbackData {
            code: 4,
            log: &mut log,
            mutate: ptr::null_mut(),
        };
        let mut input_data = CallbackData {
            code: 6,
            log: &mut log,
            mutate: ptr::null_mut(),
        };
        let mut change_data = CallbackData {
            code: 7,
            log: &mut log,
            mutate: ptr::null_mut(),
        };
        let mut click_listener = ptr::null_mut();
        let mut input_listener = ptr::null_mut();
        let mut change_listener = ptr::null_mut();
        assert_eq!(
            oui_element_add_event_listener(
                checkbox,
                4,
                0,
                Some(record_event),
                &mut click_data as *mut _ as *mut c_void,
                &mut click_listener,
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_add_event_listener(
                checkbox,
                16,
                0,
                Some(record_event),
                &mut input_data as *mut _ as *mut c_void,
                &mut input_listener,
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_add_event_listener(
                checkbox,
                17,
                0,
                Some(record_event),
                &mut change_data as *mut _ as *mut c_void,
                &mut change_listener,
            ),
            OuiStatus::Ok
        );
        let mut id = 0;
        assert_eq!(
            oui_element_get_accessibility_id(checkbox, &mut id),
            OuiStatus::Ok
        );
        assert_ne!(id, 0);
        let mut update = OuiAccessibilityUpdate {
            struct_size: size_of::<OuiAccessibilityUpdate>() as u32,
            abi_version: OUI_ABI_VERSION,
            generation: 0,
            updated_nodes: 0,
            focus_id: 0,
            full_tree: 0,
            reduced_motion: 0,
            reserved: 0,
        };
        assert_eq!(
            oui_document_accessibility_update(document, &mut update),
            OuiStatus::Ok
        );
        assert_eq!(update.full_tree, 1);
        assert!(update.updated_nodes >= 2);
        let mut first_snapshot = ptr::null_mut();
        assert_eq!(
            accessibility_snapshot::oui_document_accessibility_snapshot(
                document,
                ptr::null(),
                &mut first_snapshot,
            ),
            OuiStatus::Ok
        );
        let mut state = OuiAccessibilityNodeState {
            struct_size: size_of::<OuiAccessibilityNodeState>() as u32,
            abi_version: OUI_ABI_VERSION,
            ..Default::default()
        };
        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_get_node_state(
                first_snapshot,
                id,
                &mut state,
            ),
            OuiStatus::Ok
        );
        assert_ne!(state.flags & (1 << 5), 0);
        assert_eq!(state.flags & (1 << 6), 0);
        assert_eq!(
            oui_element_perform_accessibility_action(checkbox, 0, empty_utf8(), 0, 0),
            OuiStatus::Ok
        );
        assert_eq!(log, [42, 62, 72]);
        let mut second_snapshot = ptr::null_mut();
        assert_eq!(
            accessibility_snapshot::oui_document_accessibility_snapshot(
                document,
                first_snapshot,
                &mut second_snapshot,
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_get_node_state(
                second_snapshot,
                id,
                &mut state,
            ),
            OuiStatus::Ok
        );
        assert_ne!(state.flags & (1 << 6), 0);
        assert_eq!(
            oui_document_accessibility_update(document, &mut update),
            OuiStatus::Ok
        );
        assert_eq!(update.full_tree, 0);
        assert_eq!(update.updated_nodes, 1);
        assert_eq!(oui_document_set_reduced_motion(document, 1), OuiStatus::Ok);
        assert_eq!(
            oui_document_accessibility_update(document, &mut update),
            OuiStatus::Ok
        );
        assert_eq!(update.reduced_motion, 1);

        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_destroy(second_snapshot),
            OuiStatus::Ok
        );
        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_destroy(first_snapshot),
            OuiStatus::Ok
        );

        assert_eq!(oui_listener_destroy(click_listener), OuiStatus::Ok);
        assert_eq!(oui_listener_destroy(input_listener), OuiStatus::Ok);
        assert_eq!(oui_listener_destroy(change_listener), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(checkbox), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_accessibility_snapshot_rejects_wrong_document_and_outlives_its_document() {
        let first_document = create_document(40, 40);
        let second_document = create_document(40, 40);
        let mut first_snapshot = ptr::null_mut();
        assert_eq!(
            accessibility_snapshot::oui_document_accessibility_snapshot(
                first_document,
                ptr::null(),
                &mut first_snapshot,
            ),
            OuiStatus::Ok
        );
        let mut unrelated_snapshot = ptr::null_mut();
        assert_eq!(
            accessibility_snapshot::oui_document_accessibility_snapshot(
                second_document,
                first_snapshot,
                &mut unrelated_snapshot,
            ),
            OuiStatus::WrongDocument
        );
        assert!(unrelated_snapshot.is_null());
        let mut info = OuiAccessibilitySnapshotInfo {
            struct_size: size_of::<OuiAccessibilitySnapshotInfo>() as u32,
            abi_version: OUI_ABI_VERSION + 1,
            ..Default::default()
        };
        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_get_info(first_snapshot, &mut info),
            OuiStatus::AbiMismatch
        );
        assert_eq!(oui_document_destroy(first_document), OuiStatus::Ok);
        info.abi_version = OUI_ABI_VERSION;
        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_get_info(first_snapshot, &mut info),
            OuiStatus::Ok
        );
        assert!(info.node_count >= 1);
        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_destroy(first_snapshot),
            OuiStatus::Ok
        );
        assert_eq!(
            accessibility_snapshot::oui_accessibility_snapshot_get_info(first_snapshot, &mut info),
            OuiStatus::InvalidHandle
        );
        assert_eq!(oui_document_destroy(second_document), OuiStatus::Ok);
    }

    #[test]
    fn handles_are_thread_affine_without_dereferencing_foreign_tokens() {
        let document = create_document(16, 16);
        let address = document as usize;
        let status = std::thread::spawn(move || oui_document_update(address as *mut OuiDocument))
            .join()
            .unwrap();
        assert_eq!(status, OuiStatus::WrongThread);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn panics_are_contained() {
        assert_eq!(
            registry::ffi(|| -> Result<(), ApiError> { panic!("contained") }),
            OuiStatus::Internal
        );
    }

    #[test]
    fn c_typography_literal_values_are_typed_and_owned() {
        let document = create_document(64, 64);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);

        let mut writing_mode = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
        assert_eq!(
            oui_style_value_parse(
                StyleProperty::WritingMode as i32,
                text("vertical-rl"),
                writing_mode.as_mut_ptr(),
            ),
            OuiStatus::Ok
        );
        // SAFETY: successful parsing initialized the output record.
        let writing_mode = unsafe { writing_mode.assume_init() };
        assert_eq!(writing_mode.tag, 6);
        assert_eq!(
            oui_element_set_property(root, StyleProperty::WritingMode as i32, &writing_mode),
            OuiStatus::Ok
        );
        // The element stores an owned clone, so the transport handle may be
        // destroyed immediately after submission.
        assert_eq!(
            oui_style_compound_destroy(unsafe { writing_mode.data.compound } as *mut _),
            OuiStatus::Ok
        );

        let mut spacing = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
        assert_eq!(
            oui_style_value_parse(
                StyleProperty::LetterSpacing as i32,
                text("2px"),
                spacing.as_mut_ptr(),
            ),
            OuiStatus::Ok
        );
        // SAFETY: successful parsing initialized the output record.
        let spacing = unsafe { spacing.assume_init() };
        assert_eq!(spacing.tag, 2);
        assert_eq!(unsafe { spacing.data.number }, 2.0);

        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn native_scalar_extensions_keep_owned_property_identity_and_full_values() {
        use openui_style::{OverflowAlignment, StyleColor};
        let document_handle = create_document(64, 64);
        let element_handle = create_element(document_handle, 0, ptr::null_mut());
        for (property, literal) in [
            (StyleProperty::AlignSelf, "safe center"),
            (StyleProperty::AlignContent, "unsafe end"),
            (StyleProperty::BorderBottomColor, "currentcolor"),
            (StyleProperty::ScrollbarTrackColor, "auto"),
            (StyleProperty::Orphans, "4294967295"),
        ] {
            let mut parsed = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
            assert_eq!(
                oui_style_value_parse(property as i32, text(literal), parsed.as_mut_ptr()),
                OuiStatus::Ok
            );
            // SAFETY: successful parsing initialized the output and selected its payload.
            let parsed = unsafe { parsed.assume_init() };
            assert_eq!(parsed.tag, 6);
            let compound = unsafe { parsed.data.compound } as usize;
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &parsed),
                OuiStatus::Ok
            );
            let reference = element(element_handle as usize).unwrap();
            let state = element_document(&reference).unwrap();
            let snapshot = borrow_engine(&state)
                .unwrap()
                .computed_style(reference.node)
                .unwrap()
                .clone();
            match property {
                StyleProperty::AlignSelf => {
                    assert_eq!(snapshot.align_self.overflow, OverflowAlignment::Safe)
                }
                StyleProperty::AlignContent => {
                    assert_eq!(snapshot.align_content.overflow, OverflowAlignment::Unsafe)
                }
                StyleProperty::BorderBottomColor => {
                    assert_eq!(snapshot.border_bottom_color, StyleColor::CurrentColor)
                }
                StyleProperty::ScrollbarTrackColor => {
                    assert_eq!(snapshot.scrollbar_track_color, None)
                }
                StyleProperty::Orphans => assert_eq!(snapshot.orphans, u32::MAX),
                _ => unreachable!(),
            }
            assert_eq!(
                oui_element_set_property(element_handle, StyleProperty::Width as i32, &parsed),
                OuiStatus::WrongValueType
            );
            assert_eq!(
                oui_element_set_property(
                    element_handle,
                    StyleProperty::WritingMode as i32,
                    &parsed
                ),
                OuiStatus::WrongValueType
            );
            let mut reserved = parsed;
            reserved.reserved = 1;
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &reserved),
                OuiStatus::WrongValueType
            );
            assert_eq!(
                format!("{snapshot:?}"),
                format!(
                    "{:?}",
                    borrow_engine(&state)
                        .unwrap()
                        .computed_style(reference.node)
                        .unwrap()
                )
            );
            std::thread::spawn(move || {
                let own_document = create_document(16, 16);
                let own_element = create_element(own_document, 0, ptr::null_mut());
                let foreign = OuiStyleValue {
                    tag: 6,
                    reserved: 0,
                    data: OuiStylePayload {
                        compound: compound as *const OuiStyleCompound,
                    },
                };
                assert_eq!(
                    oui_element_set_property(own_element, property as i32, &foreign),
                    OuiStatus::WrongThread
                );
                assert_eq!(
                    oui_style_compound_destroy(compound as *mut OuiStyleCompound),
                    OuiStatus::WrongThread
                );
                assert_eq!(oui_element_destroy(own_element), OuiStatus::Ok);
                assert_eq!(oui_document_destroy(own_document), OuiStatus::Ok);
            })
            .join()
            .unwrap();
            assert_eq!(
                oui_style_compound_destroy(compound as *mut OuiStyleCompound),
                OuiStatus::Ok
            );
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &parsed),
                OuiStatus::InvalidHandle
            );
            // Submitting cloned the value into retained state before its carrier was released.
            assert_eq!(
                format!("{snapshot:?}"),
                format!(
                    "{:?}",
                    borrow_engine(&state)
                        .unwrap()
                        .computed_style(reference.node)
                        .unwrap()
                )
            );
        }
        assert_eq!(oui_element_destroy(element_handle), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document_handle), OuiStatus::Ok);
    }

    #[test]
    fn native_float_colors_preserve_precision_property_identity_and_ownership() {
        use openui_style::StyleColor;
        let document_handle = create_document(64, 64);
        let element_handle = create_element(document_handle, 0, ptr::null_mut());
        let color = Color::from_rgba_f32(0.123456, 0.234567, 0.345678, 0.5);
        let properties = [
            StyleProperty::BackgroundColor,
            StyleProperty::Color,
            StyleProperty::BorderTopColor,
            StyleProperty::BorderRightColor,
            StyleProperty::BorderBottomColor,
            StyleProperty::BorderLeftColor,
            StyleProperty::ColumnRuleColor,
            StyleProperty::OutlineColor,
            StyleProperty::ScrollbarTrackColor,
            StyleProperty::ScrollbarThumbColor,
            StyleProperty::TextDecorationColor,
            StyleProperty::TextEmphasisColor,
        ];
        for property in properties {
            let mut output = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
            assert_eq!(
                oui_style_value_color_f32_v1(
                    property as i32,
                    color.r,
                    color.g,
                    color.b,
                    color.a,
                    output.as_mut_ptr()
                ),
                OuiStatus::Ok
            );
            // SAFETY: success initialized the tagged record and compound payload.
            let output = unsafe { output.assume_init() };
            assert_eq!((output.tag, output.reserved), (6, 0));
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &output),
                OuiStatus::Ok
            );
            let reference = element(element_handle as usize).unwrap();
            let state = element_document(&reference).unwrap();
            let snapshot = borrow_engine(&state)
                .unwrap()
                .computed_style(reference.node)
                .unwrap()
                .clone();
            let resolved = match property {
                StyleProperty::BackgroundColor => snapshot.background_color,
                StyleProperty::Color => snapshot.color,
                StyleProperty::BorderTopColor => snapshot.border_top_color.resolve(&color),
                StyleProperty::BorderRightColor => snapshot.border_right_color.resolve(&color),
                StyleProperty::BorderBottomColor => snapshot.border_bottom_color.resolve(&color),
                StyleProperty::BorderLeftColor => snapshot.border_left_color.resolve(&color),
                StyleProperty::ColumnRuleColor => snapshot.column_rule_color.resolve(&color),
                StyleProperty::OutlineColor => snapshot.outline_color.resolve(&color),
                StyleProperty::ScrollbarTrackColor => snapshot.scrollbar_track_color.unwrap(),
                StyleProperty::ScrollbarThumbColor => snapshot.scrollbar_thumb_color.unwrap(),
                StyleProperty::TextDecorationColor => {
                    assert_eq!(snapshot.text_decoration_color, StyleColor::Resolved(color));
                    snapshot.text_decoration_color.resolve(&color)
                }
                StyleProperty::TextEmphasisColor => {
                    assert_eq!(snapshot.text_emphasis_color, StyleColor::Resolved(color));
                    snapshot.text_emphasis_color.resolve(&color)
                }
                _ => unreachable!(),
            };
            assert_eq!(resolved, color);
            assert_ne!(resolved.a, 128.0 / 255.0);
            assert_eq!(
                oui_element_set_property(element_handle, StyleProperty::Width as i32, &output),
                OuiStatus::WrongValueType
            );
            let other_color = if property == StyleProperty::Color {
                StyleProperty::BackgroundColor
            } else {
                StyleProperty::Color
            };
            assert_eq!(
                oui_element_set_property(element_handle, other_color as i32, &output),
                OuiStatus::WrongValueType
            );
            let compound = unsafe { output.data.compound } as usize;
            std::thread::spawn(move || {
                assert_eq!(
                    oui_style_compound_destroy(compound as *mut OuiStyleCompound),
                    OuiStatus::WrongThread
                );
            })
            .join()
            .unwrap();
            assert_eq!(
                oui_style_compound_destroy(compound as *mut OuiStyleCompound),
                OuiStatus::Ok
            );
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &output),
                OuiStatus::InvalidHandle
            );
            assert_eq!(
                format!("{snapshot:?}"),
                format!(
                    "{:?}",
                    borrow_engine(&state)
                        .unwrap()
                        .computed_style(reference.node)
                        .unwrap()
                )
            );
        }
        assert_eq!(oui_element_destroy(element_handle), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document_handle), OuiStatus::Ok);
    }

    #[test]
    fn invalid_float_colors_leave_output_unchanged() {
        for index in 0..4 {
            for invalid_channel in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, -0.01, 1.01] {
                let mut channels = [0.5; 4];
                channels[index] = invalid_channel;
                let mut output = OuiStyleValue {
                    tag: 77,
                    reserved: 42,
                    data: OuiStylePayload { integer: 123 },
                };
                assert_eq!(
                    oui_style_value_color_f32_v1(
                        StyleProperty::Color as i32,
                        channels[0],
                        channels[1],
                        channels[2],
                        channels[3],
                        &mut output
                    ),
                    OuiStatus::InvalidArgument
                );
                assert_eq!(
                    (output.tag, output.reserved, unsafe { output.data.integer }),
                    (77, 42, 123)
                );
            }
        }
        for (property, expected) in [
            (i32::MAX, OuiStatus::InvalidArgument),
            (StyleProperty::Width as i32, OuiStatus::WrongValueType),
        ] {
            let mut output = OuiStyleValue {
                tag: 77,
                reserved: 42,
                data: OuiStylePayload { integer: 123 },
            };
            assert_eq!(
                oui_style_value_color_f32_v1(property, 0.0, 0.0, 0.0, 1.0, &mut output),
                expected
            );
            assert_eq!(
                (output.tag, output.reserved, unsafe { output.data.integer }),
                (77, 42, 123)
            );
        }
        assert_eq!(
            oui_style_value_color_f32_v1(
                StyleProperty::Color as i32,
                0.0,
                0.0,
                0.0,
                1.0,
                ptr::null_mut()
            ),
            OuiStatus::InvalidArgument
        );
    }

    #[test]
    fn native_fragment_keywords_reach_engine_and_keep_owned_property_identity() {
        use openui_style::{
            BoxDecorationBreak, BreakInside, BreakValue, ColumnFill, ColumnSpan, ColumnWrap,
            RendererStyleValue as R,
        };
        let document_handle = create_document(64, 48);
        let element_handle = create_element(document_handle, 0, ptr::null_mut());
        for (property, literal, expected) in [
            (
                StyleProperty::ColumnFill,
                "auto",
                R::ColumnFill(ColumnFill::Auto),
            ),
            (
                StyleProperty::BreakInside,
                "avoid",
                R::BreakInside(BreakInside::Avoid),
            ),
            (
                StyleProperty::BreakBefore,
                "column",
                R::BreakBefore(BreakValue::Column),
            ),
            (
                StyleProperty::BreakAfter,
                "avoid-page",
                R::BreakAfter(BreakValue::AvoidPage),
            ),
            (
                StyleProperty::ColumnSpan,
                "all",
                R::ColumnSpan(ColumnSpan::All),
            ),
            (
                StyleProperty::ColumnWrap,
                "nowrap",
                R::ColumnWrap(ColumnWrap::NoWrap),
            ),
            (
                StyleProperty::BoxDecorationBreak,
                "clone",
                R::BoxDecorationBreak(BoxDecorationBreak::Clone),
            ),
            (
                StyleProperty::BorderTopStyle,
                "outset",
                R::BorderTopStyle(BorderStyle::Outset),
            ),
            (
                StyleProperty::BorderRightStyle,
                "inset",
                R::BorderRightStyle(BorderStyle::Inset),
            ),
            (
                StyleProperty::BorderBottomStyle,
                "ridge",
                R::BorderBottomStyle(BorderStyle::Ridge),
            ),
            (
                StyleProperty::BorderLeftStyle,
                "groove",
                R::BorderLeftStyle(BorderStyle::Groove),
            ),
            (
                StyleProperty::ColumnRuleStyle,
                "double",
                R::ColumnRuleStyle(BorderStyle::Double),
            ),
            (
                StyleProperty::OutlineStyle,
                "dashed",
                R::OutlineStyle(BorderStyle::Dashed),
            ),
        ] {
            let mut parsed = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
            assert_eq!(
                oui_style_value_parse(property as i32, text(literal), parsed.as_mut_ptr()),
                OuiStatus::Ok,
                "native keyword constructor must accept {property:?} {literal}"
            );
            // SAFETY: a successful constructor initialized the tagged record.
            let parsed = unsafe { parsed.assume_init() };
            assert_eq!((parsed.tag, parsed.reserved), (6, 0));
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &parsed),
                OuiStatus::Ok
            );
            let reference = element(element_handle as usize).unwrap();
            let state = element_document(&reference).unwrap();
            let snapshot = borrow_engine(&state)
                .unwrap()
                .computed_style(reference.node)
                .unwrap()
                .clone();
            assert_eq!(
                openui_style::value_from_computed(&snapshot, property),
                StyleValue::Renderer(expected.clone())
            );
            let other = if property == StyleProperty::ColumnFill {
                StyleProperty::BreakInside
            } else {
                StyleProperty::ColumnFill
            };
            assert_eq!(
                oui_element_set_property(element_handle, other as i32, &parsed),
                OuiStatus::WrongValueType
            );
            let mut invalid = parsed;
            invalid.reserved = 1;
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &invalid),
                OuiStatus::WrongValueType
            );
            let compound = unsafe { parsed.data.compound } as usize;
            if property == StyleProperty::ColumnFill {
                std::thread::spawn(move || {
                    let own_document = create_document(16, 16);
                    let own_element = create_element(own_document, 0, ptr::null_mut());
                    let foreign = OuiStyleValue {
                        tag: 6,
                        reserved: 0,
                        data: OuiStylePayload {
                            compound: compound as *const OuiStyleCompound,
                        },
                    };
                    assert_eq!(
                        oui_element_set_property(own_element, property as i32, &foreign),
                        OuiStatus::WrongThread
                    );
                    assert_eq!(
                        oui_style_compound_destroy(compound as *mut OuiStyleCompound),
                        OuiStatus::WrongThread
                    );
                    assert_eq!(oui_element_destroy(own_element), OuiStatus::Ok);
                    assert_eq!(oui_document_destroy(own_document), OuiStatus::Ok);
                })
                .join()
                .unwrap();
            }
            assert_eq!(
                oui_style_compound_destroy(compound as *mut OuiStyleCompound),
                OuiStatus::Ok
            );
            assert_eq!(
                oui_element_set_property(element_handle, property as i32, &parsed),
                OuiStatus::InvalidHandle
            );
            assert_eq!(
                openui_style::value_from_computed(
                    &borrow_engine(&state)
                        .unwrap()
                        .computed_style(reference.node)
                        .unwrap(),
                    property
                ),
                StyleValue::Renderer(expected)
            );
            assert_eq!(
                format!("{snapshot:?}"),
                format!(
                    "{:?}",
                    borrow_engine(&state)
                        .unwrap()
                        .computed_style(reference.node)
                        .unwrap()
                )
            );
            let mut output = OuiStyleValue {
                tag: 77,
                reserved: 42,
                data: OuiStylePayload { integer: 123 },
            };
            assert_eq!(
                oui_style_value_parse(property as i32, text("invalid-keyword"), &mut output),
                OuiStatus::InvalidArgument
            );
            assert_eq!(
                (output.tag, output.reserved, unsafe { output.data.integer }),
                (77, 42, 123)
            );
        }
        assert_eq!(oui_element_destroy(element_handle), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document_handle), OuiStatus::Ok);
    }

    #[test]
    fn native_table_display_literals_keep_existing_scalar_encoding() {
        let document_handle = create_document(64, 48);
        let element_handle = create_element(document_handle, 0, ptr::null_mut());
        for (literal, expected) in [
            ("inline-table", openui_style::Display::InlineTable),
            ("table-row-group", openui_style::Display::TableRowGroup),
            (
                "table-header-group",
                openui_style::Display::TableHeaderGroup,
            ),
            (
                "table-footer-group",
                openui_style::Display::TableFooterGroup,
            ),
            ("table-row", openui_style::Display::TableRow),
            ("table-cell", openui_style::Display::TableCell),
            (
                "table-column-group",
                openui_style::Display::TableColumnGroup,
            ),
            ("table-column", openui_style::Display::TableColumn),
            ("table-caption", openui_style::Display::TableCaption),
        ] {
            let mut parsed = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
            assert_eq!(
                oui_style_value_parse(
                    StyleProperty::Display as i32,
                    text(literal),
                    parsed.as_mut_ptr()
                ),
                OuiStatus::Ok
            );
            let parsed = unsafe { parsed.assume_init() };
            assert_eq!(
                (parsed.tag, parsed.reserved, unsafe {
                    parsed.data.enum_value
                }),
                (5, 0, expected as i32)
            );
            assert_eq!(
                oui_element_set_property(element_handle, StyleProperty::Display as i32, &parsed),
                OuiStatus::Ok
            );
            let reference = element(element_handle as usize).unwrap();
            let state = element_document(&reference).unwrap();
            assert_eq!(
                borrow_engine(&state)
                    .unwrap()
                    .computed_style(reference.node)
                    .unwrap()
                    .display,
                expected
            );
        }
        assert_eq!(oui_element_destroy(element_handle), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document_handle), OuiStatus::Ok);
    }

    #[test]
    fn invalid_native_literals_preserve_output_and_existing_scalar_encodings() {
        for (property, literal) in [
            (StyleProperty::FilterBlur, "NaN"),
            (StyleProperty::OverflowClipMargin, "inf"),
            (StyleProperty::Left, "NaNpx"),
            (StyleProperty::ColumnWidth, "infem"),
            (StyleProperty::AlignSelf, "safe stretch"),
            (StyleProperty::AlignContent, "safe space-between"),
            (StyleProperty::Orphans, "-1"),
            (StyleProperty::Widows, "4294967296"),
        ] {
            let mut output = OuiStyleValue {
                tag: 77,
                reserved: 42,
                data: OuiStylePayload { integer: 123 },
            };
            assert_eq!(
                oui_style_value_parse(property as i32, text(literal), &mut output),
                OuiStatus::InvalidArgument
            );
            assert_eq!(
                (output.tag, output.reserved, unsafe { output.data.integer }),
                (77, 42, 123)
            );
        }
        for (property, literals) in [
            (
                StyleProperty::AlignItems,
                &[
                    "normal",
                    "stretch",
                    "center",
                    "start",
                    "end",
                    "flex-start",
                    "flex-end",
                    "baseline",
                    "auto",
                    "self-start",
                    "self-end",
                    "left",
                    "right",
                    "last baseline",
                    "legacy",
                ][..],
            ),
            (
                StyleProperty::JustifyContent,
                &[
                    "normal",
                    "start",
                    "end",
                    "center",
                    "flex-start",
                    "flex-end",
                    "space-between",
                    "space-around",
                    "space-evenly",
                    "stretch",
                    "baseline",
                    "last baseline",
                    "left",
                    "right",
                ][..],
            ),
        ] {
            for (number, literal) in literals.iter().enumerate() {
                let mut output = std::mem::MaybeUninit::<OuiStyleValue>::uninit();
                assert_eq!(
                    oui_style_value_parse(property as i32, text(literal), output.as_mut_ptr()),
                    OuiStatus::Ok
                );
                let output = unsafe { output.assume_init() };
                assert_eq!(output.tag, 5);
                assert_eq!(unsafe { output.data.enum_value }, number as i32);
                assert_eq!(
                    value::style_value(property, &output).unwrap(),
                    parse_literal(property, literal).unwrap()
                );
            }
        }
    }
}
