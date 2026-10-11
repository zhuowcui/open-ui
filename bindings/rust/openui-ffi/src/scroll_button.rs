//! Append-only C access to Engine-owned native scrolling controls.

use super::*;
use openui_engine::{ScrollButtonAvailability, ScrollButtonDirection, ScrollButtonState};

fn direction(value: u32) -> Result<ScrollButtonDirection, ApiError> {
    use ScrollButtonDirection as D;
    match value {
        0 => Ok(D::Up),
        1 => Ok(D::Right),
        2 => Ok(D::Down),
        3 => Ok(D::Left),
        4 => Ok(D::BlockStart),
        5 => Ok(D::BlockEnd),
        6 => Ok(D::InlineStart),
        7 => Ok(D::InlineEnd),
        _ => Err(invalid("unknown scroll-button direction")),
    }
}

fn raw_direction(value: ScrollButtonDirection) -> u32 {
    use ScrollButtonDirection as D;
    match value {
        D::Up => 0,
        D::Right => 1,
        D::Down => 2,
        D::Left => 3,
        D::BlockStart => 4,
        D::BlockEnd => 5,
        D::InlineStart => 6,
        D::InlineEnd => 7,
    }
}

fn button_state(
    handle: *mut OuiElement,
) -> Result<(Rc<DocumentState>, ScrollButtonState), ApiError> {
    let button = element(handle as usize)?;
    let document = element_document(&button)?;
    let snapshot = borrow_engine_mut(&document)?.scroll_button_state(button.node)?;
    Ok((document, snapshot))
}

fn write_alias(
    document: &Rc<DocumentState>,
    node: Option<NodeHandle>,
    out_element: *mut *mut OuiElement,
) -> Result<(), ApiError> {
    let alias = match node {
        Some(node) => focus_events::owned_element_handle(document, node)?,
        None => ptr::null_mut(),
    };
    // SAFETY: every caller validates nonnull writable pointer storage. The
    // Engine borrow ends before registering an alias or transferring ownership.
    unsafe { ptr::write(out_element, alias) };
    Ok(())
}

// SAFETY CONTRACT: live owning-thread origin, writable nonnull output storage.
// The returned element alias is independently owned, released with destroy.
#[no_mangle]
pub extern "C" fn oui_element_create_scroll_button_v1(
    origin_handle: *mut OuiElement,
    raw_direction: u32,
    out_button: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_button.is_null() {
            return Err(invalid("scroll-button output is null"));
        }
        let direction = direction(raw_direction)?;
        let origin = element(origin_handle as usize)?;
        let state = element_document(&origin)?;
        // Validate ownership storage before creating a retained node. No
        // callbacks run with the map or Engine borrowed.
        let mut handles = state
            .element_handles
            .try_borrow_mut()
            .map_err(|_| ApiError::new(OuiStatus::Reentrant, "element handle map is borrowed"))?;
        handles.try_reserve(1).map_err(|_| {
            ApiError::new(
                OuiStatus::OutOfMemory,
                "element handle map allocation failed",
            )
        })?;
        let mut aliases = Vec::new();
        aliases.try_reserve(1).map_err(|_| {
            ApiError::new(OuiStatus::OutOfMemory, "element alias allocation failed")
        })?;
        let mut engine = borrow_engine_mut(&state)?;
        let token = registry::register_with_factory(|token| {
            let node = engine.create_scroll_button(origin.node, direction)?;
            aliases.push(token);
            handles.insert(node, aliases);
            Ok(LocalHandle::Element(ElementRef {
                document: Rc::downgrade(&state),
                node,
            }))
        })?;
        drop(engine);
        drop(handles);
        // SAFETY: output storage is nonnull and writable under the C contract.
        unsafe { ptr::write(out_button, token as *mut OuiElement) };
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread origin and writable pointer output.
// Missing directions return NULL; errors leave output unchanged.
#[no_mangle]
pub extern "C" fn oui_element_get_scroll_button_v1(
    origin_handle: *mut OuiElement,
    raw_direction: u32,
    out_button: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_button.is_null() {
            return Err(invalid("scroll-button output is null"));
        }
        let direction = direction(raw_direction)?;
        let origin = element(origin_handle as usize)?;
        let state = element_document(&origin)?;
        let node = borrow_engine(&state)?.scroll_button(origin.node, direction)?;
        write_alias(&state, node, out_button)
    })
}

// SAFETY CONTRACT: live owning-thread button and writable pointer output.
#[no_mangle]
pub extern "C" fn oui_scroll_button_get_origin_v1(
    button: *mut OuiElement,
    out_origin: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_origin.is_null() {
            return Err(invalid("scroll-button origin output is null"));
        }
        let (document, state) = button_state(button)?;
        write_alias(&document, Some(state.origin), out_origin)
    })
}

// SAFETY CONTRACT: live owning-thread button. Unknown/colliding directions
// fail without changing the control's identity or direction.
#[no_mangle]
pub extern "C" fn oui_scroll_button_set_direction_v1(
    button: *mut OuiElement,
    raw_direction: u32,
) -> OuiStatus {
    ffi(|| {
        let direction = direction(raw_direction)?;
        with_element_mut(button as usize, |engine, node| {
            engine.set_scroll_button_direction(node, direction)
        })
    })
}

// SAFETY CONTRACT: readable initialized version prefix and writable storage
// of the declared size. Errors leave the output and trailing bytes unchanged.
#[no_mangle]
pub extern "C" fn oui_scroll_button_get_state_v1(
    button: *mut OuiElement,
    out_state: *mut OuiScrollButtonStateV1,
) -> OuiStatus {
    ffi(|| {
        if out_state.is_null() {
            return Err(invalid("scroll-button state output is null"));
        }
        // SAFETY: caller supplies a readable size field, then a readable ABI
        // field only when the declared size covers the complete prefix.
        let struct_size = unsafe { ptr::read(out_state.cast::<u32>()) };
        if struct_size < 2 * size_of::<u32>() as u32 {
            return Err(invalid("scroll-button state header is too small"));
        }
        let abi_version = unsafe { ptr::read(out_state.cast::<u32>().add(1)) };
        check_header(
            struct_size,
            abi_version,
            size_of::<OuiScrollButtonStateV1>(),
        )?;
        let (_, state) = button_state(button)?;
        let availability = match state.availability {
            ScrollButtonAvailability::Available => 0,
            ScrollButtonAvailability::Boundary => 1,
            ScrollButtonAvailability::NoScrollport => 2,
            ScrollButtonAvailability::AuthoredDisabled => 3,
            ScrollButtonAvailability::Unsupported(_) => 4,
        };
        let (target_x, target_y) = state.target_offset.unwrap_or((0.0, 0.0));
        let output = OuiScrollButtonStateV1 {
            struct_size,
            abi_version,
            direction: raw_direction(state.direction),
            availability,
            enabled: u8::from(state.enabled),
            has_target: u8::from(state.target_offset.is_some()),
            reserved: [0; 6],
            current_x: state.current_offset.0,
            current_y: state.current_offset.1,
            target_x,
            target_y,
        };
        // SAFETY: the checked header covers the complete writable structure.
        unsafe { ptr::write(out_state, output) };
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread button and writable pointer output.
// Axis is 0 (physical X) or 1 (physical Y); phase is 0 (current) or 1 (preview).
#[no_mangle]
pub extern "C" fn oui_scroll_button_get_snap_target_v1(
    button: *mut OuiElement,
    axis: u32,
    phase: u32,
    out_target: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_target.is_null() || axis > 1 || phase > 1 {
            return Err(invalid("invalid scroll-button snap-target query"));
        }
        let (document, state) = button_state(button)?;
        let targets = if phase == 0 {
            state.current_snap_targets
        } else {
            state.snap_targets
        };
        write_alias(&document, targets[axis as usize], out_target)
    })
}

// SAFETY CONTRACT: live owning-thread button, writable length output and
// destination storage of capacity bytes. The copied UTF-8 reason has no NUL.
#[no_mangle]
pub extern "C" fn oui_scroll_button_copy_unavailable_reason_v1(
    button: *mut OuiElement,
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
) -> OuiStatus {
    ffi(|| {
        if out_length.is_null() {
            return Err(invalid("scroll-button reason length output is null"));
        }
        let (_, state) = button_state(button)?;
        let reason = match state.availability {
            ScrollButtonAvailability::Unsupported(reason) => reason,
            _ => "",
        };
        copy_bytes_to_c(
            reason.as_bytes(),
            destination,
            capacity,
            out_length,
            "scroll-button reason",
        )
    })
}

// SAFETY CONTRACT: owning-thread live button. Callbacks use the origin's
// existing listeners and execute only after every Engine/platform borrow ends.
#[no_mangle]
pub extern "C" fn oui_scroll_button_activate_v1(button: *mut OuiElement) -> OuiStatus {
    ffi(|| {
        let control = element(button as usize)?;
        let state = element_document(&control)?;
        if !borrow_engine(&state)?.is_scroll_button(control.node)? {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "element is not a scroll button",
            ));
        }
        state
            .native
            .perform_accessibility_action_for_native_facade(
                control.node,
                AccessibilityAction::Click,
            )
            .map_err(native_app::native_error)
    })
}

// SAFETY CONTRACT: owning-thread live control; enabled is exactly 0 or 1.
#[no_mangle]
pub extern "C" fn oui_element_set_native_appearance_v1(
    element: *mut OuiElement,
    enabled: u8,
) -> OuiStatus {
    ffi(|| {
        if enabled > 1 {
            return Err(invalid("native appearance must be zero or one"));
        }
        with_element_mut(element as usize, |engine, node| {
            engine.set_native_control_appearance(node, enabled != 0)
        })
    })
}

// SAFETY CONTRACT: owning-thread live authored origin and declared strictness.
#[no_mangle]
pub extern "C" fn oui_element_set_scroll_snap_strictness_v1(
    origin: *mut OuiElement,
    strictness: u32,
) -> OuiStatus {
    ffi(|| {
        let strictness = match strictness {
            0 => openui_engine::ScrollSnapStrictness::Mandatory,
            1 => openui_engine::ScrollSnapStrictness::Proximity,
            _ => return Err(invalid("unknown scroll-snap strictness")),
        };
        with_element_mut(origin as usize, |engine, node| {
            engine.set_scroll_snap_strictness(node, strictness)
        })
    })
}

// SAFETY CONTRACT: live owning-thread authored origin and writable byte
// output. The copied value is 0 or 1; errors leave the output unchanged.
#[no_mangle]
pub extern "C" fn oui_element_settle_scroll_snap_v1(
    origin: *mut OuiElement,
    out_changed: *mut u8,
) -> OuiStatus {
    ffi(|| {
        if out_changed.is_null() {
            return Err(invalid("scroll-snap changed output is null"));
        }
        let origin = element(origin as usize)?;
        let state = element_document(&origin)?;
        let changed = borrow_engine_mut(&state)?.settle_native_scroll_snap(origin.node)?;
        // SAFETY: caller guarantees writable nonnull storage for one byte.
        unsafe { ptr::write(out_changed, u8::from(changed)) };
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread origin and optional target in the same
// document. NULL clears the preference. Errors leave the preference unchanged.
#[no_mangle]
pub extern "C" fn oui_element_set_scroll_snap_target_v1(
    origin: *mut OuiElement,
    target: *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        let origin = element(origin as usize)?;
        let (document, target) = if target.is_null() {
            (element_document(&origin)?, None)
        } else {
            let target = element(target as usize)?;
            (same_document(&origin, &target)?, Some(target.node))
        };
        borrow_engine_mut(&document)?.set_scroll_snap_target(origin.node, target)?;
        Ok(())
    })
}
