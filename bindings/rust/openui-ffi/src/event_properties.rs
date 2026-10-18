//! Immutable event properties for callbacks from the shared native pipeline.

use super::*;

struct ActiveProperties {
    address: usize,
    document: std::rc::Weak<DocumentState>,
    bubbles: bool,
    cancelable: bool,
}

thread_local! {
    static ACTIVE_PROPERTIES: RefCell<Vec<ActiveProperties>> = const { RefCell::new(Vec::new()) };
}

pub(crate) struct EventPropertiesScope {
    address: usize,
}

impl EventPropertiesScope {
    pub(crate) fn new(state: &Rc<DocumentState>, native: &openui::Event, event: &OuiEvent) -> Self {
        let address = event as *const OuiEvent as usize;
        ACTIVE_PROPERTIES.with(|events| {
            events.borrow_mut().push(ActiveProperties {
                address,
                document: Rc::downgrade(state),
                bubbles: native.bubbles(),
                cancelable: native.cancelable(),
            });
        });
        Self { address }
    }
}

impl Drop for EventPropertiesScope {
    fn drop(&mut self) {
        ACTIVE_PROPERTIES.with(|events| {
            let removed = events.borrow_mut().pop();
            debug_assert!(removed.is_some_and(|event| event.address == self.address));
        });
    }
}

// SAFETY CONTRACT: out_properties has a readable versioned header and writable
// current-version storage. event is checked by address without dereferencing it.
#[no_mangle]
pub extern "C" fn oui_event_properties_v1(
    event: *const OuiEvent,
    out_properties: *mut OuiEventPropertiesV1,
) -> OuiStatus {
    ffi(|| {
        if out_properties.is_null() {
            return Err(invalid("event properties output is null"));
        }
        // SAFETY: caller provides a readable size field.
        let struct_size = unsafe { ptr::read(out_properties.cast::<u32>()) };
        if struct_size < 2 * size_of::<u32>() as u32 {
            return Err(invalid("event properties header is too small"));
        }
        // SAFETY: the declared header includes the ABI version.
        let abi_version = unsafe { ptr::read(out_properties.cast::<u32>().add(1)) };
        check_header(struct_size, abi_version, size_of::<OuiEventPropertiesV1>())?;
        let context = ACTIVE_PROPERTIES.with(|events| {
            events
                .borrow()
                .iter()
                .rev()
                .find(|active| active.address == event as usize)
                .map(|active| (active.document.clone(), active.bubbles, active.cancelable))
        });
        let (document, bubbles, cancelable) = context.ok_or_else(|| {
            ApiError::new(
                OuiStatus::InvalidState,
                "no active native callback for event",
            )
        })?;
        let _document = document.upgrade().ok_or_else(|| {
            ApiError::new(OuiStatus::InvalidState, "event document was destroyed")
        })?;
        // SAFETY: validated writable current-version storage; trailing bytes
        // and error outputs remain untouched. No fallible operation follows.
        unsafe {
            ptr::write(
                out_properties,
                OuiEventPropertiesV1 {
                    struct_size: size_of::<OuiEventPropertiesV1>() as u32,
                    abi_version: OUI_ABI_VERSION,
                    bubbles: u32::from(bubbles),
                    cancelable: u32::from(cancelable),
                },
            );
        }
        Ok(())
    })
}
