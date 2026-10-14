//! Owned native relation queries over the same retained Engine used by Rust.
use super::*;

fn query_relation(
    element_handle: *mut OuiElement,
    out_element: *mut *mut OuiElement,
    query: fn(&Engine, NodeHandle) -> Result<Option<NodeHandle>, openui_engine::EngineError>,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() {
            return Err(invalid("related element output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let related = {
            let engine = borrow_engine(&state)?;
            query(&engine, element.node)?
        };
        // The engine borrow ends before registering the independently owned
        // alias. The map is acquired before ownership is registered.
        let result = if let Some(node) = related {
            focus_events::owned_element_handle(&state, node)?
        } else {
            ptr::null_mut()
        };
        // SAFETY: caller supplies writable storage for one pointer. Errors
        // leave it unchanged; ownership transfers only after fallible work.
        unsafe { ptr::write(out_element, result) };
        Ok(())
    })
}

// SAFETY CONTRACT: owning-thread live element and writable pointer output.
#[no_mangle]
pub extern "C" fn oui_element_associated_form_v1(
    element_handle: *mut OuiElement,
    out_form: *mut *mut OuiElement,
) -> OuiStatus {
    query_relation(element_handle, out_form, Engine::associated_form)
}

// SAFETY CONTRACT: owning-thread live node and writable pointer output.
#[no_mangle]
pub extern "C" fn oui_element_parent_v1(
    element_handle: *mut OuiElement,
    out_parent: *mut *mut OuiElement,
) -> OuiStatus {
    query_relation(element_handle, out_parent, Engine::parent)
}
