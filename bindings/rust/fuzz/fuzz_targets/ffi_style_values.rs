#![no_main]

use libfuzzer_sys::fuzz_target;
use openui_ffi::*;
use std::mem::size_of;
use std::ptr;

fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }
    let config = OuiDocumentConfig {
        struct_size: size_of::<OuiDocumentConfig>() as u32,
        abi_version: OUI_ABI_VERSION,
        width: 64,
        height: 64,
        scale_factor: 1.0,
    };
    let mut document = ptr::null_mut();
    if oui_document_create(&config, &mut document) != OuiStatus::Ok {
        return;
    }
    let mut element = ptr::null_mut();
    if oui_element_create(document, i32::from(data[0] % 39), &mut element) == OuiStatus::Ok {
        // Tags 1..=5 contain no pointers. A compound property paired with any
        // of these tags is rejected before its union payload is interpreted.
        let value = OuiStyleValue {
            tag: u32::from(data[1] % 6),
            reserved: u32::from(data[2] & 1),
            data: OuiStylePayload {
                integer: i32::from_le_bytes([
                    data[3],
                    *data.get(4).unwrap_or(&0),
                    *data.get(5).unwrap_or(&0),
                    *data.get(6).unwrap_or(&0),
                ]),
            },
        };
        let property = i32::from(*data.get(7).unwrap_or(&0));
        let _ = oui_element_set_property(element, property, &value);
        let _ = oui_element_destroy(element);
    }
    let _ = oui_document_destroy(document);
});
