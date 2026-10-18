//! Integer C dimensions through the same retained Engine operations as Rust.

use super::*;

fn dimension(
    handle: *mut OuiElement,
    out_value: *mut i32,
    operation: fn(&mut Engine, NodeHandle) -> Result<i32, openui_engine::EngineError>,
) -> OuiStatus {
    ffi(|| {
        if out_value.is_null() {
            return Err(invalid("integer dimension output is null"));
        }
        let source = element(handle as usize)?;
        let state = element_document(&source)?;
        let value = {
            let mut engine = borrow_engine_mut(&state)?;
            operation(&mut engine, source.node)?
        };
        // SAFETY: the C contract provides aligned writable i32 storage. No
        // caller output is read, and errors return before writing anything.
        unsafe { ptr::write(out_value, value) };
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread element; nonnull, aligned writable i32
// output. Errors leave output unchanged; a valid no-box element returns zero.
#[no_mangle]
pub extern "C" fn oui_element_get_client_width_v1(
    handle: *mut OuiElement,
    out_value: *mut i32,
) -> OuiStatus {
    dimension(handle, out_value, Engine::client_width)
}

// SAFETY CONTRACT: same scalar output/lifetime contract as client_width_v1.
#[no_mangle]
pub extern "C" fn oui_element_get_client_height_v1(
    handle: *mut OuiElement,
    out_value: *mut i32,
) -> OuiStatus {
    dimension(handle, out_value, Engine::client_height)
}

// SAFETY CONTRACT: same scalar output/lifetime contract as client_width_v1.
#[no_mangle]
pub extern "C" fn oui_element_get_scroll_width_v1(
    handle: *mut OuiElement,
    out_value: *mut i32,
) -> OuiStatus {
    dimension(handle, out_value, Engine::scroll_width)
}

// SAFETY CONTRACT: same scalar output/lifetime contract as client_width_v1.
#[no_mangle]
pub extern "C" fn oui_element_get_scroll_height_v1(
    handle: *mut OuiElement,
    out_value: *mut i32,
) -> OuiStatus {
    dimension(handle, out_value, Engine::scroll_height)
}
