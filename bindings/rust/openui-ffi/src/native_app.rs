//! C applications run the Rust application shell over their retained document.

use super::*;

pub(crate) fn native_error(error: openui::Error) -> ApiError {
    match error {
        openui::Error::Engine(error) => error.into(),
        openui::Error::ReentrantMutation => ApiError::new(OuiStatus::Reentrant, error.to_string()),
        openui::Error::InvalidArgument(message) => invalid(message),
        _ => ApiError::new(OuiStatus::InvalidState, error.to_string()),
    }
}

pub(crate) fn app_state(app: *mut OuiApp) -> Result<Rc<AppState>, ApiError> {
    match get(app as usize, HandleKind::App)? {
        LocalHandle::App(state) => Ok(state),
        _ => unreachable!("kind checked by registry"),
    }
}

pub(crate) fn dispatch_native_listener(
    state: &Rc<DocumentState>,
    node: NodeHandle,
    target: NodeHandle,
    native: &openui::Event,
    capture: Option<bool>,
) -> Result<(), ApiError> {
    let event_type = match native.event_type.as_str() {
        "pointerdown" | "mousedown" => 1,
        "pointerup" | "mouseup" => 2,
        "pointermove" | "mousemove" => 3,
        "click" => 4,
        "wheel" => 5,
        "keydown" => 6,
        "keyup" => 7,
        "textinput" => 8,
        "compositionstart" => 9,
        "compositionupdate" => 10,
        "compositionend" => 11,
        "focus" => 12,
        "blur" => 13,
        "pointerenter" | "mouseenter" => 14,
        "pointerleave" | "mouseleave" => 15,
        "input" => 16,
        "change" => 17,
        "beforeinput" => 18,
        "scroll" => 19,
        "animationstart" => 20,
        "animationiteration" => 21,
        "animationend" => 22,
        "animationcancel" => 23,
        "pointercancel" | "mousecancel" => 24,
        "focusin" => 25,
        "focusout" => 26,
        "select" => 27,
        "selectionchange" => 28,
        _ => return Ok(()),
    };
    let mut event = synthesized_event(
        event_type,
        OuiUtf8 {
            data: native.key_text.as_ptr(),
            length: native.key_text.len(),
        },
    );
    event.phase = match native.phase() {
        Some(openui::EventPhase::Capture) => 1,
        Some(openui::EventPhase::Target) => 2,
        Some(openui::EventPhase::Bubble) => 3,
        None => 0,
    };
    event.timestamp_ns = native.timestamp_ns();
    event.modifiers = native.modifiers as u32;
    event.x = native.mouse_x;
    event.y = native.mouse_y;
    event.delta_x = native.delta_x;
    event.delta_y = native.delta_y;
    event.key_code = native.key_code;
    event.pointer_id = native.pointer_id as u32;
    event.target = element_address(state, target).unwrap_or(0) as *mut OuiElement;
    if native.default_prevented() {
        event.flags |= OUI_EVENT_FLAG_DEFAULT_PREVENTED;
    }
    if native.propagation_stopped() {
        event.flags |= OUI_EVENT_FLAG_PROPAGATION_STOPPED;
    }
    if native.immediate_propagation_stopped() {
        event.flags |= OUI_EVENT_FLAG_IMMEDIATE_PROPAGATION_STOPPED;
    }
    let _event_scope = super::event_properties::EventPropertiesScope::new(state, native, &event);
    let _focus_scope = super::focus_events::FocusEventScope::new(state, native, &event);
    let _input_scope = super::input_events::InputEventScope::new(state, native, &event);
    match capture {
        Some(capture) => invoke_event_listeners(state, node, capture, &mut event)?,
        None => {
            invoke_event_listeners(state, node, true, &mut event)?;
            invoke_event_listeners(state, node, false, &mut event)?;
        }
    }
    if event.flags & OUI_EVENT_FLAG_DEFAULT_PREVENTED != 0 {
        native.prevent_default();
    }
    if event.flags & OUI_EVENT_FLAG_IMMEDIATE_PROPAGATION_STOPPED != 0 {
        native.stop_immediate_propagation();
    } else if event.flags & OUI_EVENT_FLAG_PROPAGATION_STOPPED != 0 {
        native.stop_propagation();
    }
    Ok(())
}

thread_local! {
    static NATIVE_RUN_ACTIVE: Cell<bool> = const { Cell::new(false) };
}

#[cfg(all(feature = "linux", target_os = "linux"))]
struct RunGuard(Rc<AppState>);

#[cfg(all(feature = "linux", target_os = "linux"))]
impl Drop for RunGuard {
    fn drop(&mut self) {
        self.0.running.set(false);
        self.0.exit_handle.borrow_mut().take();
        NATIVE_RUN_ACTIVE.with(|active| active.set(false));
    }
}

// SAFETY CONTRACT: app is a live owning-thread handle. config is readable;
// callback and user_data stay valid until this blocking call returns. Callbacks
// must not unwind, retain event pointers, or begin another native run.
#[no_mangle]
pub extern "C" fn oui_app_run(
    app_handle: *mut OuiApp,
    config: *const OuiAppRunConfig,
) -> OuiStatus {
    ffi(|| {
        let state = app_state(app_handle)?;
        if config.is_null() {
            return Err(invalid("app run configuration is null"));
        }
        // SAFETY: the versioned configuration starts with a readable u32 size.
        // Read the header before constructing a reference to the full layout.
        let struct_size = unsafe { ptr::read(config.cast::<u32>()) };
        if struct_size < 2 * size_of::<u32>() as u32 {
            return Err(invalid("app run configuration header is too small"));
        }
        // SAFETY: a declared header includes the ABI-version field.
        let abi_version = unsafe { ptr::read(config.cast::<u32>().add(1)) };
        check_header(struct_size, abi_version, size_of::<OuiAppRunConfig>())?;
        // SAFETY: the validated size covers the complete current layout. The
        // caller supplies readable storage for its declared struct_size.
        let config = unsafe { *config };
        if config.flags != 0 || config.reserved != 0 {
            return Err(invalid("app run reserved fields must be zero"));
        }
        if NATIVE_RUN_ACTIVE.with(Cell::get) || state.running.get() {
            return Err(ApiError::new(
                OuiStatus::Reentrant,
                "a native application run is already active",
            ));
        }
        if state.has_run.get() {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "an app can attempt its native run only once",
            ));
        }
        #[cfg(not(all(feature = "linux", target_os = "linux")))]
        return Err(native_error(openui::Error::PlatformUnavailable));
        #[cfg(all(feature = "linux", target_os = "linux"))]
        {
            let mut application = native_application(&state)?;
            let exit = application.exit_handle();
            if state.exit_requested.get() {
                exit.request_exit();
            }
            *state.exit_handle.borrow_mut() = Some(exit);
            state.running.set(true);
            state.has_run.set(true);
            NATIVE_RUN_ACTIVE.with(|active| active.set(true));
            let _guard = RunGuard(state.clone());
            let address = app_handle as usize;
            application.on_platform_event(move |event| {
                let events = state
                    .document
                    .native
                    .drain_animation_events()
                    .expect("native event processing released document borrows");
                state.document.animation_events.borrow_mut().extend(events);
                if let Some(callback) = config.callback {
                    let raw = platform_event(event);
                    // SAFETY: callback/user_data validity spans this run. Text
                    // and path borrow the event for this call only. No engine,
                    // registry, document, or listener-list borrow is held.
                    unsafe { callback(address as *mut OuiApp, &raw, config.user_data) };
                }
            });
            application.run_document().map_err(native_error)
        }
    })
}

// SAFETY CONTRACT: app is a live handle owned by the calling UI thread.
#[no_mangle]
pub extern "C" fn oui_app_request_exit(app_handle: *mut OuiApp) -> OuiStatus {
    ffi(|| {
        let state = app_state(app_handle)?;
        state.exit_requested.set(true);
        if let Some(exit) = state
            .exit_handle
            .try_borrow()
            .map_err(|_| ApiError::new(OuiStatus::Reentrant, "exit handle is borrowed"))?
            .as_ref()
        {
            exit.request_exit();
        }
        Ok(())
    })
}

#[cfg(all(feature = "linux", target_os = "linux"))]
pub(crate) fn native_application(state: &AppState) -> Result<openui::App, ApiError> {
    let viewport = borrow_engine(&state.document)?.viewport();
    openui::App::from_document(
        state.document.native.clone(),
        openui::WindowOptions {
            title: state.title.clone(),
            size: openui::LogicalSize::new(viewport.logical_width(), viewport.logical_height()),
            backend: match state.backend {
                1 => openui::BackendPreference::OpenGl,
                2 => openui::BackendPreference::Software,
                _ => openui::BackendPreference::Auto,
            },
        },
    )
    .map_err(native_error)
}

#[cfg(all(feature = "linux", target_os = "linux"))]
fn platform_event(event: &openui_platform::PlatformEvent) -> OuiPlatformEvent {
    use openui_platform::{KeyPhase, PlatformEvent, PointerButton, PointerPhase};
    use std::os::unix::ffi::OsStrExt;
    let mut raw = OuiPlatformEvent {
        struct_size: size_of::<OuiPlatformEvent>() as u32,
        abi_version: OUI_ABI_VERSION,
        event_type: 0,
        phase: 0,
        flags: 0,
        backend: 0,
        pointer_id: 0,
        x: 0.0,
        y: 0.0,
        delta_x: 0.0,
        delta_y: 0.0,
        key_code: 0,
        modifiers: 0,
        button: 0,
        reserved: 0,
        frame_number: 0,
        time_ms: 0.0,
        text: OuiUtf8 {
            data: ptr::null(),
            length: 0,
        },
        path: ptr::null(),
        path_length: 0,
        viewport: OuiViewportMetrics {
            logical_width: 0.0,
            logical_height: 0.0,
            physical_width: 0,
            physical_height: 0,
            device_scale_factor: 0.0,
            authority: 0,
            reserved: 0,
        },
    };
    let utf8 = |text: &str| OuiUtf8 {
        data: text.as_ptr(),
        length: text.len(),
    };
    let modifiers = |value: &openui_platform::Modifiers| {
        u32::from(value.shift)
            | (u32::from(value.control) << 1)
            | (u32::from(value.alt) << 2)
            | (u32::from(value.meta) << 3)
    };
    match event {
        PlatformEvent::BackendChanged(status) => {
            raw.event_type = 1;
            raw.backend = match status.active {
                openui_platform::BackendPreference::Auto => 0,
                openui_platform::BackendPreference::OpenGl => 1,
                openui_platform::BackendPreference::Software => 2,
            };
            if let Some(reason) = &status.fallback_reason {
                raw.text = utf8(reason);
                raw.flags = 4;
            }
        }
        PlatformEvent::Resized(viewport) => {
            raw.event_type = 2;
            raw.viewport = OuiViewportMetrics {
                logical_width: viewport.logical_width(),
                logical_height: viewport.logical_height(),
                physical_width: viewport.physical_width(),
                physical_height: viewport.physical_height(),
                device_scale_factor: viewport.device_scale_factor(),
                authority: 2,
                reserved: 0,
            };
        }
        PlatformEvent::Pointer {
            pointer_id,
            phase,
            x,
            y,
            button,
            modifiers: keys,
        } => {
            raw.event_type = 3;
            raw.pointer_id = *pointer_id;
            raw.x = *x;
            raw.y = *y;
            raw.phase = match phase {
                PointerPhase::Move => 0,
                PointerPhase::Down => 1,
                PointerPhase::Up => 2,
                PointerPhase::Cancel => 3,
                PointerPhase::Leave => 4,
            };
            raw.button = match button {
                PointerButton::Left => 0,
                PointerButton::Middle => 1,
                PointerButton::Right => 2,
                PointerButton::Other(value) => 3 + u32::from(*value),
            };
            raw.modifiers = modifiers(keys);
        }
        PlatformEvent::Wheel {
            x,
            y,
            delta_x,
            delta_y,
            modifiers: keys,
        } => {
            raw.event_type = 4;
            raw.x = *x;
            raw.y = *y;
            raw.delta_x = *delta_x;
            raw.delta_y = *delta_y;
            raw.modifiers = modifiers(keys);
        }
        PlatformEvent::Key {
            phase,
            key_code,
            text,
            modifiers: keys,
            repeat,
        } => {
            raw.event_type = 5;
            raw.phase = u32::from(*phase == KeyPhase::Up);
            raw.key_code = *key_code;
            raw.modifiers = modifiers(keys);
            raw.flags = u32::from(*repeat);
            if let Some(text) = text {
                raw.text = utf8(text);
            }
        }
        PlatformEvent::TextInput(text) => {
            raw.event_type = 6;
            raw.text = utf8(text);
        }
        PlatformEvent::CompositionStart => raw.event_type = 7,
        PlatformEvent::CompositionUpdate(text) => {
            raw.event_type = 8;
            raw.text = utf8(text);
        }
        PlatformEvent::CompositionEnd(text) => {
            raw.event_type = 9;
            raw.text = utf8(text);
        }
        PlatformEvent::Focused(focused) => {
            raw.event_type = 10;
            raw.flags = u32::from(*focused) << 1;
        }
        PlatformEvent::DroppedFile(path) | PlatformEvent::HoveredFile(path) => {
            raw.event_type = if matches!(event, PlatformEvent::DroppedFile(_)) {
                11
            } else {
                12
            };
            raw.path = path.as_os_str().as_bytes().as_ptr();
            raw.path_length = path.as_os_str().as_bytes().len();
        }
        PlatformEvent::HoveredFileCancelled => raw.event_type = 13,
        PlatformEvent::Presented {
            frame_number,
            time_ms,
        } => {
            raw.event_type = 14;
            raw.frame_number = *frame_number;
            raw.time_ms = *time_ms;
        }
        PlatformEvent::CloseRequested => raw.event_type = 15,
    }
    raw
}
