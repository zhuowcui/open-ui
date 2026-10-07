//! Versioned focus metadata over the shared native Document callback route.

use super::*;

struct ActiveFocusEvent {
    address: usize,
    document: std::rc::Weak<DocumentState>,
    native: openui::Event,
}

thread_local! {
    static ACTIVE_FOCUS_EVENTS: RefCell<Vec<ActiveFocusEvent>> = const { RefCell::new(Vec::new()) };
}

pub(crate) struct FocusEventScope {
    address: usize,
}

impl FocusEventScope {
    pub(crate) fn new(
        state: &Rc<DocumentState>,
        native: &openui::Event,
        event: &OuiEvent,
    ) -> Option<Self> {
        if !matches!(event.event_type, 12 | 13 | 25 | 26) {
            return None;
        }
        let address = event as *const OuiEvent as usize;
        ACTIVE_FOCUS_EVENTS.with(|events| {
            events.borrow_mut().push(ActiveFocusEvent {
                address,
                document: Rc::downgrade(state),
                native: native.clone(),
            });
        });
        Some(Self { address })
    }
}

impl Drop for FocusEventScope {
    fn drop(&mut self) {
        ACTIVE_FOCUS_EVENTS.with(|events| {
            let removed = events.borrow_mut().pop();
            debug_assert!(removed.is_some_and(|event| event.address == self.address));
        });
    }
}

fn owned_element_handle(
    document: &Rc<DocumentState>,
    node: NodeHandle,
) -> Result<*mut OuiElement, ApiError> {
    // Acquire the map before registering ownership so a failed borrow cannot
    // leak a newly registered handle. No application callback runs here.
    let mut handles = document
        .element_handles
        .try_borrow_mut()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "element handle map is borrowed"))?;
    let token = register(LocalHandle::Element(ElementRef {
        document: Rc::downgrade(document),
        node,
    }))?;
    handles.entry(node).or_default().push(token);
    Ok(token as *mut OuiElement)
}

// SAFETY CONTRACT: out_element is writable storage for one pointer.
#[no_mangle]
pub extern "C" fn oui_document_focused_element_v1(
    document_handle: *mut OuiDocument,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() {
            return Err(invalid("focused element output is null"));
        }
        let state = document(document_handle as usize)?;
        let focused = borrow_engine(&state)?.focused();
        let handle = if let Some(node) = focused {
            owned_element_handle(&state, node)?
        } else {
            ptr::null_mut()
        };
        // SAFETY: caller supplies writable pointer storage. All fallible work
        // finishes before the new handle's ownership transfers to the caller.
        unsafe { ptr::write(out_element, handle) };
        Ok(())
    })
}

// SAFETY CONTRACT: out_info is writable storage with a readable versioned
// header. event is a borrowed pointer received by an owning-thread focus
// callback; it is validated against active scopes without dereferencing it.
#[no_mangle]
pub extern "C" fn oui_event_focus_info_v1(
    event: *const OuiEvent,
    out_info: *mut OuiFocusEventInfoV1,
) -> OuiStatus {
    ffi(|| {
        if out_info.is_null() {
            return Err(invalid("focus event info output is null"));
        }
        // SAFETY: the caller supplies a readable size field.
        let struct_size = unsafe { ptr::read(out_info.cast::<u32>()) };
        if struct_size < 2 * size_of::<u32>() as u32 {
            return Err(invalid("focus event info header is too small"));
        }
        // SAFETY: the declared header includes the ABI version.
        let abi_version = unsafe { ptr::read(out_info.cast::<u32>().add(1)) };
        check_header(struct_size, abi_version, size_of::<OuiFocusEventInfoV1>())?;
        let context = ACTIVE_FOCUS_EVENTS.with(|events| {
            events
                .borrow()
                .iter()
                .rev()
                .find(|active| active.address == event as usize)
                .map(|active| (active.document.clone(), active.native.clone()))
        });
        let (document, native) = context.ok_or_else(|| {
            ApiError::new(
                OuiStatus::InvalidState,
                "no active native focus callback for event",
            )
        })?;
        let document = document.upgrade().ok_or_else(|| {
            ApiError::new(
                OuiStatus::InvalidState,
                "focus event document was destroyed",
            )
        })?;
        let related_target = if let Some(node) = native.related_node_for_native_facade() {
            owned_element_handle(&document, node)?
        } else {
            ptr::null_mut()
        };
        // SAFETY: the validated header covers writable current-version storage.
        // All fallible operations finish before ownership transfers to caller.
        unsafe {
            ptr::write(
                out_info,
                OuiFocusEventInfoV1 {
                    struct_size: size_of::<OuiFocusEventInfoV1>() as u32,
                    abi_version: OUI_ABI_VERSION,
                    bubbles: u32::from(native.bubbles()),
                    cancelable: u32::from(native.cancelable()),
                    related_target,
                },
            );
        }
        Ok(())
    })
}
