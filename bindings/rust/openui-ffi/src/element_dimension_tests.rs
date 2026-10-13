//! Scalar dimension guards use public C operations and the retained native
//! document. Runtime results stay unclaimed until these tests actually run.

use super::*;
use std::mem::MaybeUninit;

type Getter = extern "C" fn(*mut OuiElement, *mut i32) -> OuiStatus;
const GETTERS: [Getter; 4] = [
    oui_element_get_client_width_v1,
    oui_element_get_client_height_v1,
    oui_element_get_scroll_width_v1,
    oui_element_get_scroll_height_v1,
];

fn text(value: &str) -> OuiUtf8 {
    OuiUtf8 {
        data: value.as_ptr(),
        length: value.len(),
    }
}

fn config(width: u32, height: u32, scale: f64) -> OuiDocumentConfig {
    OuiDocumentConfig {
        struct_size: size_of::<OuiDocumentConfig>() as u32,
        abi_version: OUI_ABI_VERSION,
        viewport: OuiViewportMetrics {
            logical_width: width as f64,
            logical_height: height as f64,
            physical_width: (width as f64 * scale) as u32,
            physical_height: (height as f64 * scale) as u32,
            device_scale_factor: scale,
            authority: 1,
            reserved: 0,
        },
    }
}

fn fixture(width: u32, height: u32, scale: f64) -> (*mut OuiDocument, *mut OuiElement) {
    let mut document = ptr::null_mut();
    let mut root = ptr::null_mut();
    assert_eq!(
        oui_document_create(&config(width, height, scale), &mut document),
        OuiStatus::Ok
    );
    assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
    (document, root)
}

fn child(document: *mut OuiDocument, parent: *mut OuiElement, tag: i32) -> *mut OuiElement {
    let mut element = ptr::null_mut();
    assert_eq!(
        oui_element_create(document, tag, &mut element),
        OuiStatus::Ok
    );
    assert_eq!(oui_element_append_child(parent, element), OuiStatus::Ok);
    element
}

fn property(element: *mut OuiElement, property: StyleProperty, literal: &str) {
    let mut value = MaybeUninit::<OuiStyleValue>::uninit();
    assert_eq!(
        oui_style_value_parse(property as i32, text(literal), value.as_mut_ptr()),
        OuiStatus::Ok
    );
    // SAFETY: the successful parser initialized the entire value.
    let value = unsafe { value.assume_init() };
    assert_eq!(
        oui_element_set_property(element, property as i32, &value),
        OuiStatus::Ok
    );
    if value.tag == 6 {
        // SAFETY: compound is the initialized union member for tag 6.
        assert_eq!(
            oui_style_compound_destroy(unsafe { value.data.compound }.cast_mut()),
            OuiStatus::Ok
        );
    }
}

fn dimensions(element: *mut OuiElement) -> [i32; 4] {
    GETTERS.map(|query| {
        let mut output = i32::MIN;
        assert_eq!(query(element, &mut output), OuiStatus::Ok);
        output
    })
}

fn metrics(element: *mut OuiElement) -> [f64; 4] {
    let mut output = OuiScrollMetricsV1 {
        struct_size: size_of::<OuiScrollMetricsV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        client_width: -1.0,
        client_height: -1.0,
        scroll_width: -1.0,
        scroll_height: -1.0,
    };
    let mut found = 0;
    assert_eq!(
        oui_element_get_scroll_metrics_v1(element, &mut output, &mut found),
        OuiStatus::Ok
    );
    assert_eq!(found, 1);
    [
        output.client_width,
        output.client_height,
        output.scroll_width,
        output.scroll_height,
    ]
}

#[test]
fn border_collapse_compounds_preserve_table_dimensions_lifetime_and_failed_outputs() {
    for scale in [1.0, 1.25, 1.5, 1.75, 2.0] {
        let (document, root) = fixture(320, 240, scale);
        let table = child(document, root, 6);
        let caption = child(document, table, 7);
        let group = child(document, table, 11);
        let row = child(document, group, 13);
        let cell = child(document, row, 14);
        for (node, id, literal) in [
            (table, StyleProperty::Width, "120px"),
            (table, StyleProperty::BoxSizing, "border-box"),
            (table, StyleProperty::BorderSpacing, "0px"),
            (table, StyleProperty::Border, "2px solid black"),
            (caption, StyleProperty::Height, "12px"),
            (cell, StyleProperty::Height, "20px"),
        ] {
            property(node, id, literal);
        }
        assert_eq!(dimensions(table), [120, 36, 118, 32]);
        assert_eq!(dimensions(caption), [120, 12, 120, 12]);
        for part in [group, row, cell] {
            assert_eq!(dimensions(part), [116, 20, 116, 20]);
        }
        property(table, StyleProperty::Transform, "translate(20px,7px)");
        assert_eq!(dimensions(table), [120, 36, 118, 32]);
        assert_eq!(oui_element_remove(caption), OuiStatus::Ok);
        assert_eq!(dimensions(table), [120, 24, 116, 20]);
        let state = document_state(document);
        for literal in ["collapse", "separate", "collapse"] {
            let input = literal.to_owned();
            let mut storage = MaybeUninit::<OuiStyleValue>::uninit();
            assert_eq!(
                oui_style_value_parse(
                    StyleProperty::BorderCollapse as i32,
                    text(&input),
                    storage.as_mut_ptr()
                ),
                OuiStatus::Ok
            );
            let value = unsafe { storage.assume_init() };
            assert_eq!(value.tag, 6);
            assert_eq!(
                oui_element_set_property(table, StyleProperty::BorderCollapse as i32, &value),
                OuiStatus::Ok
            );
            assert_eq!(
                oui_style_compound_destroy(unsafe { value.data.compound }.cast_mut()),
                OuiStatus::Ok
            );
            drop(input);
            let before_dimensions = dimensions(table);
            assert_eq!(before_dimensions[0], 120);
            assert_eq!(dimensions(cell)[1], 20);
            let before = borrow_engine_mut(&state).unwrap().stats();
            assert_eq!(
                oui_element_set_property(table, StyleProperty::BorderCollapse as i32, &value),
                OuiStatus::InvalidHandle
            );
            assert_eq!(dimensions(table), before_dimensions);
            property(table, StyleProperty::BorderCollapse, literal);
            assert_eq!(dimensions(table), before_dimensions);
            let after = borrow_engine_mut(&state).unwrap().stats();
            assert_eq!(after.layouts, before.layouts);
            assert_eq!(after.paints, before.paints);
            for invalid in ["auto", "collapse separate", "collapse; display:none"] {
                let mut output = OuiStyleValue {
                    tag: 3,
                    reserved: 0x1234,
                    data: OuiStylePayload { integer: 42 },
                };
                assert_eq!(
                    oui_style_value_parse(
                        StyleProperty::BorderCollapse as i32,
                        text(invalid),
                        &mut output
                    ),
                    OuiStatus::InvalidArgument
                );
                assert_eq!((output.tag, output.reserved), (3, 0x1234));
                assert_eq!(unsafe { output.data.integer }, 42);
                assert_eq!(dimensions(table), before_dimensions);
            }
        }
        drop(state);
        for element in [cell, row, group, caption, table, root] {
            assert_eq!(oui_element_destroy(element), OuiStatus::Ok);
        }
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }
}

#[test]
fn scalar_dimensions_validate_ownership_borrows_and_preserve_output_storage() {
    let (document, root) = fixture(320, 240, 1.0);
    let node = child(document, root, 0);
    property(node, StyleProperty::Width, "40.5px");
    property(node, StyleProperty::Height, "20.5px");
    #[repr(C)]
    struct Output {
        value: i32,
        tail: [u8; 12],
    }
    for (index, query) in GETTERS.into_iter().enumerate() {
        let mut output = Output {
            value: i32::MIN,
            tail: [0x6d; 12],
        };
        assert_eq!(query(node, ptr::null_mut()), OuiStatus::InvalidArgument);
        assert_eq!(
            query(ptr::null_mut(), &mut output.value),
            OuiStatus::InvalidArgument
        );
        assert_eq!(output.value, i32::MIN);
        assert_eq!(output.tail, [0x6d; 12]);
        assert_eq!(
            query(1usize as *mut OuiElement, &mut output.value),
            OuiStatus::InvalidHandle
        );
        assert_eq!(output.value, i32::MIN);
        let state = document_state(document);
        let borrow = borrow_engine(&state).unwrap();
        assert_eq!(query(node, &mut output.value), OuiStatus::Reentrant);
        assert_eq!(output.value, i32::MIN);
        drop(borrow);
        drop(state);
        let address = node as usize;
        let (status, unchanged) = std::thread::spawn(move || {
            let mut output = i32::MIN;
            let status = query(address as *mut OuiElement, &mut output);
            (status, output)
        })
        .join()
        .unwrap();
        assert_eq!(status, OuiStatus::WrongThread);
        assert_eq!(unchanged, i32::MIN);
        // Scalar storage has no struct_size/abi_version prefix to initialize.
        assert_eq!(query(node, &mut output.value), OuiStatus::Ok);
        assert_eq!(output.value, [41, 21, 41, 21][index]);
        assert_eq!(output.tail, [0x6d; 12]);
    }
    assert_eq!(oui_element_remove(node), OuiStatus::Ok);
    for query in GETTERS {
        let mut output = -113;
        assert_eq!(query(node, &mut output), OuiStatus::StaleHandle);
        assert_eq!(output, -113);
    }
    assert_eq!(oui_element_destroy(node), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    for query in GETTERS {
        let mut output = -113;
        assert_eq!(query(root, &mut output), OuiStatus::InvalidHandle);
        assert_eq!(output, -113);
    }
    assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
}

fn document_state(handle: *mut OuiDocument) -> Rc<DocumentState> {
    document(handle as usize).unwrap()
}

#[test]
fn scalar_dimensions_keep_fractional_metrics_and_native_mutations_in_the_same_document() {
    for scale in [1.0, 1.25, 1.5, 2.0] {
        let (document, root) = fixture(320, 240, scale);
        let scroller = child(document, root, 0);
        let content = child(document, scroller, 0);
        assert_eq!(
            oui_element_set_attribute(scroller, text("id"), text("native-dimensions")),
            OuiStatus::Ok
        );
        for (node, property_id, literal) in [
            (scroller, StyleProperty::Width, "100.5px"),
            (scroller, StyleProperty::Height, "80.5px"),
            (scroller, StyleProperty::Border, "2px solid black"),
            (scroller, StyleProperty::Overflow, "scroll"),
            (scroller, StyleProperty::ScrollbarWidth, "auto"),
            (content, StyleProperty::Width, "200.5px"),
            (content, StyleProperty::Height, "160.5px"),
        ] {
            property(node, property_id, literal);
        }
        assert_eq!(dimensions(scroller), [86, 66, 201, 161]);
        let raw = metrics(scroller);
        assert_eq!(raw, [85.5, 65.5, 200.5, 160.5]);
        let operation = OuiTransformOperation {
            kind: 2,
            reserved: 0,
            x: OuiLength {
                value: 0.0,
                unit: 0,
            },
            y: OuiLength {
                value: 0.0,
                unit: 0,
            },
            values: [2.0, 3.0, 0.0, 0.0, 0.0, 0.0],
        };
        let mut transform = ptr::null_mut();
        assert_eq!(
            oui_transform_create(&operation, 1, &mut transform),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_set_property(
                scroller,
                StyleProperty::Transform as i32,
                &OuiStyleValue {
                    tag: 6,
                    reserved: 0,
                    data: OuiStylePayload {
                        compound: transform,
                    },
                },
            ),
            OuiStatus::Ok
        );
        // The document owns the copied transform after the input is released.
        assert_eq!(oui_style_compound_destroy(transform), OuiStatus::Ok);
        assert_eq!(dimensions(scroller), [86, 66, 201, 161]);
        assert_eq!(metrics(scroller), raw);
        let state = document_state(document);
        let native = state
            .native
            .element_by_id("native-dimensions")
            .unwrap()
            .unwrap();
        assert_eq!(
            [
                native.client_width().unwrap(),
                native.client_height().unwrap(),
                native.scroll_width().unwrap(),
                native.scroll_height().unwrap()
            ],
            [86, 66, 201, 161]
        );
        native.set_overflow(openui_style::Overflow::Hidden).unwrap();
        assert_eq!(dimensions(scroller), [101, 81, 201, 161]);
        assert_eq!(metrics(scroller), [100.5, 80.5, 200.5, 160.5]);
        drop(native);
        drop(state);
        assert_eq!(oui_element_destroy(content), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(scroller), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }
}

#[test]
fn scalar_dimensions_reflect_no_box_lifecycle_and_independent_document_ownership() {
    let (document, root) = fixture(320, 240, 1.0);
    let (other_document, other_root) = fixture(96, 48, 1.0);
    assert_eq!(dimensions(root), [320, 240, 320, 240]);
    assert_eq!(dimensions(other_root), [96, 48, 96, 48]);
    let inline = child(document, root, 1);
    assert_eq!(
        oui_element_set_text(inline, text("native inline")),
        OuiStatus::Ok
    );
    assert_eq!(dimensions(inline), [0; 4]);
    property(inline, StyleProperty::Display, "inline-block");
    assert_eq!(oui_element_set_text(inline, text("")), OuiStatus::Ok);
    property(inline, StyleProperty::Width, "40.5px");
    property(inline, StyleProperty::Height, "20.5px");
    assert_eq!(dimensions(inline), [41, 21, 41, 21]);
    assert_eq!(oui_element_detach(inline), OuiStatus::Ok);
    assert_eq!(dimensions(inline), [0; 4]);
    assert_eq!(
        oui_element_append_child(other_root, inline),
        OuiStatus::WrongDocument
    );
    assert_eq!(dimensions(inline), [0; 4]);
    assert_eq!(dimensions(other_root), [96, 48, 96, 48]);
    assert_eq!(oui_element_append_child(root, inline), OuiStatus::Ok);
    assert_eq!(dimensions(inline), [41, 21, 41, 21]);
    property(inline, StyleProperty::Display, "contents");
    assert_eq!(dimensions(inline), [0; 4]);
    property(inline, StyleProperty::Display, "none");
    assert_eq!(dimensions(inline), [0; 4]);
    assert_eq!(oui_element_destroy(inline), OuiStatus::Ok);
    assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    assert_eq!(oui_element_destroy(other_root), OuiStatus::Ok);
    assert_eq!(oui_document_destroy(other_document), OuiStatus::Ok);
}
