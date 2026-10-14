//! C ABI consumers share the native Engine reveal destination and ownership.
use openui_ffi::*;
use openui_style::{Overflow, StyleProperty};
use std::{mem::size_of, ptr};

struct Fixture {
    document: *mut OuiDocument,
    elements: Vec<*mut OuiElement>,
    port: *mut OuiElement,
    target: *mut OuiElement,
}

fn set_length(node: *mut OuiElement, property: StyleProperty, value: f32) {
    assert_eq!(
        oui_element_set_property(
            node,
            property as i32,
            &OuiStyleValue {
                tag: 1,
                reserved: 0,
                data: OuiStylePayload {
                    length: OuiLength { value, unit: 0 }
                },
            }
        ),
        OuiStatus::Ok
    );
}

fn set_enum(node: *mut OuiElement, property: StyleProperty, value: i32) {
    assert_eq!(
        oui_element_set_property(
            node,
            property as i32,
            &OuiStyleValue {
                tag: 5,
                reserved: 0,
                data: OuiStylePayload { enum_value: value },
            }
        ),
        OuiStatus::Ok
    );
}

fn offset(node: *mut OuiElement) -> (f64, f64) {
    let (mut x, mut y) = (f64::NAN, f64::NAN);
    assert_eq!(
        oui_element_get_scroll_offset(node, &mut x, &mut y),
        OuiStatus::Ok
    );
    (x, y)
}

impl Fixture {
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
                &mut document
            ),
            OuiStatus::Ok
        );
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let mut elements = vec![root];
        let mut create = |parent| {
            let mut node = ptr::null_mut();
            // OUI_TAG_DIV is the existing C tag0, with native block defaults.
            assert_eq!(oui_element_create(document, 0, &mut node), OuiStatus::Ok);
            assert_eq!(oui_element_append_child(parent, node), OuiStatus::Ok);
            elements.push(node);
            node
        };
        let port = create(root);
        let target = create(port);
        let tail = create(port);
        for (node, property, value) in [
            (port, StyleProperty::Width, 100.0),
            (port, StyleProperty::Height, 80.0),
            (target, StyleProperty::Width, 20.0),
            (target, StyleProperty::Height, 20.0),
            (target, StyleProperty::MarginLeft, 150.0),
            (target, StyleProperty::MarginTop, 120.0),
            (tail, StyleProperty::Width, 400.0),
            (tail, StyleProperty::Height, 300.0),
            (port, StyleProperty::ScrollPaddingLeft, 20.0),
            (port, StyleProperty::ScrollPaddingTop, 25.0),
            (port, StyleProperty::ScrollPaddingRight, 5.0),
            (port, StyleProperty::ScrollPaddingBottom, 10.0),
            (target, StyleProperty::ScrollMarginLeft, 10.0),
            (target, StyleProperty::ScrollMarginTop, 15.0),
            (target, StyleProperty::ScrollMarginRight, 9.0),
            (target, StyleProperty::ScrollMarginBottom, 7.0),
        ] {
            set_length(node, property, value);
        }
        set_enum(port, StyleProperty::Overflow, Overflow::Hidden as i32);
        let mut width = OuiStyleValue {
            tag: 0,
            reserved: 0,
            data: OuiStylePayload { integer: 0 },
        };
        assert_eq!(
            oui_style_value_parse(
                StyleProperty::ScrollbarWidth as i32,
                OuiUtf8 {
                    data: b"none".as_ptr(),
                    length: 4
                },
                &mut width
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_set_property(port, StyleProperty::ScrollbarWidth as i32, &width),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_style_compound_destroy(unsafe { width.data.compound } as *mut OuiStyleCompound),
            OuiStatus::Ok
        );
        Self {
            document,
            elements,
            port,
            target,
        }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        for node in self.elements.drain(..).rev() {
            assert_eq!(oui_element_destroy(node), OuiStatus::Ok);
        }
        assert_eq!(oui_document_destroy(self.document), OuiStatus::Ok);
    }
}

#[test]
fn c_instant_smooth_and_accessibility_reveal_use_shared_native_insets() {
    let fixture = Fixture::new();
    assert_eq!(
        oui_element_scroll_into_view_v1(fixture.target, 0, 0, 1),
        OuiStatus::Ok
    );
    assert_eq!(offset(fixture.port), (120.0, 80.0));
    assert_eq!(oui_element_scroll_to(fixture.port, 0.0, 0.0), OuiStatus::Ok);
    assert_eq!(
        oui_element_smooth_scroll_into_view_v1(fixture.target, 0, 0, 1, 100.0),
        OuiStatus::Ok
    );
    assert_eq!(offset(fixture.port), (0.0, 0.0));
    assert_eq!(
        oui_document_set_animation_time(fixture.document, 100.0),
        OuiStatus::Ok
    );
    assert_eq!(offset(fixture.port), (120.0, 80.0));
    assert_eq!(oui_element_scroll_to(fixture.port, 0.0, 0.0), OuiStatus::Ok);
    assert_eq!(
        oui_element_perform_accessibility_action(
            fixture.target,
            10,
            OuiUtf8 {
                data: ptr::null(),
                length: 0
            },
            0,
            0
        ),
        OuiStatus::Ok
    );
    assert_eq!(offset(fixture.port), (84.0, 77.0));
}

#[test]
fn c_reveal_validation_and_thread_ownership_preserve_current_destination() {
    let fixture = Fixture::new();
    assert_eq!(
        oui_element_scroll_into_view_v1(fixture.target, 0, 0, 1),
        OuiStatus::Ok
    );
    assert_eq!(
        oui_element_scroll_into_view_v1(fixture.target, 4, 0, 1),
        OuiStatus::InvalidArgument
    );
    assert_eq!(
        oui_element_smooth_scroll_into_view_v1(fixture.target, 0, 0, 1, f64::NAN),
        OuiStatus::InvalidArgument
    );
    let address = fixture.target as usize;
    assert_eq!(
        std::thread::spawn(move || oui_element_scroll_into_view_v1(
            address as *mut OuiElement,
            0,
            0,
            1
        ))
        .join()
        .unwrap(),
        OuiStatus::WrongThread
    );
    assert_eq!(offset(fixture.port), (120.0, 80.0));
}
