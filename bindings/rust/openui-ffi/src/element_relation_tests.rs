use super::*;

#[test]
fn relation_queries_reject_engine_and_handle_map_borrows_without_transferring_ownership() {
    let config = OuiDocumentConfig {
        struct_size: size_of::<OuiDocumentConfig>() as u32,
        abi_version: OUI_ABI_VERSION,
        viewport: OuiViewportMetrics {
            logical_width: 320.,
            logical_height: 200.,
            physical_width: 320,
            physical_height: 200,
            device_scale_factor: 1.,
            authority: 1,
            reserved: 0,
        },
    };
    let mut doc = ptr::null_mut();
    let mut body = ptr::null_mut();
    let mut form = ptr::null_mut();
    let mut input = ptr::null_mut();
    assert_eq!(oui_document_create(&config, &mut doc), OuiStatus::Ok);
    assert_eq!(oui_document_root(doc, &mut body), OuiStatus::Ok);
    assert_eq!(oui_element_create(doc, 35, &mut form), OuiStatus::Ok);
    assert_eq!(oui_element_create(doc, 23, &mut input), OuiStatus::Ok);
    assert_eq!(oui_element_append_child(body, form), OuiStatus::Ok);
    assert_eq!(oui_element_append_child(form, input), OuiStatus::Ok);
    let state = document(doc as usize).unwrap();
    for query in [oui_element_associated_form_v1, oui_element_parent_v1] {
        let before: usize = state.element_handles.borrow().values().map(Vec::len).sum();
        let mut out = 1usize as *mut OuiElement;
        let engine = borrow_engine_mut(&state).unwrap();
        assert_eq!(query(input, &mut out), OuiStatus::Reentrant);
        assert_eq!(out, 1usize as *mut OuiElement);
        drop(engine);
        let map = state.element_handles.borrow_mut();
        assert_eq!(query(input, &mut out), OuiStatus::Reentrant);
        assert_eq!(out, 1usize as *mut OuiElement);
        drop(map);
        assert_eq!(
            state
                .element_handles
                .borrow()
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            before
        );
        assert_eq!(query(input, &mut out), OuiStatus::Ok);
        assert!(!out.is_null());
        assert_eq!(
            state
                .element_handles
                .borrow()
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            before + 1
        );
        assert_eq!(oui_element_destroy(out), OuiStatus::Ok);
        assert_eq!(
            state
                .element_handles
                .borrow()
                .values()
                .map(Vec::len)
                .sum::<usize>(),
            before
        );
    }
    assert_eq!(oui_document_destroy(doc), OuiStatus::Ok);
}
