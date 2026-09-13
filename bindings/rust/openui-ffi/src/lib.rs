//! Panic-contained, generation-checked Open UI v0.2 C ABI.

#![deny(unsafe_op_in_unsafe_fn)]
// C callers cannot express Rust's `unsafe` call-site marker. Every exported
// boundary validates nulls and metadata before dereferencing, and documents
// the remaining readable/writable-memory preconditions in its safety contract.
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod generated;
mod registry;
mod types;
mod value;

pub use types::*;

use generated::{property_from_raw, valid_event_type};
use openui_compositor::SoftwareCompositor;
use openui_dom::ElementTag;
use openui_engine::{
    ControlAdjustment, Engine, EventPhase as EngineEventPhase, FocusOrigin, NodeHandle,
    PointerEventKind, Viewport,
};
use openui_style::{
    Border, BorderStyle, Color, CornerRadii, Edges, FontFamilyList, Gap, GenericFontFamily,
    StyleValue, Transform2D, TransformList, TransformOperation,
};
use registry::{
    borrow_engine, borrow_engine_mut, bytes, destroy, document, element, element_document, ffi,
    ffi_preserve, ffi_value, get, last_error, register, utf8, ApiError, AppState, DocumentState,
    ElementRef, HandleKind, ListenerRecord, ListenerRef, LocalHandle, ResourceRef,
};
use std::cell::{Cell, RefCell};
use std::ffi::c_void;
use std::mem::size_of;
use std::ptr;
use std::rc::Rc;

fn invalid(message: impl Into<String>) -> ApiError {
    ApiError::new(OuiStatus::InvalidArgument, message)
}

fn check_header(actual_size: u32, actual_abi: u32, required: usize) -> Result<(), ApiError> {
    if actual_abi != OUI_ABI_VERSION {
        return Err(ApiError::new(
            OuiStatus::AbiMismatch,
            format!("ABI version {actual_abi:#010x} does not match {OUI_ABI_VERSION:#010x}"),
        ));
    }
    if (actual_size as usize) < required {
        return Err(invalid(format!(
            "struct_size {actual_size} is smaller than required {required}"
        )));
    }
    Ok(())
}

fn write_handle<T>(out: *mut *mut T, handle: LocalHandle) -> Result<(), ApiError> {
    if out.is_null() {
        return Err(invalid("output handle pointer is null"));
    }
    let token = register(handle)?;
    // SAFETY: the C contract requires `out` to point to writable storage for
    // one handle pointer. It was checked for null, and the token is opaque.
    unsafe { ptr::write(out, token as *mut T) };
    Ok(())
}

fn write_element_handle(
    out: *mut *mut OuiElement,
    state: &Rc<DocumentState>,
    node: NodeHandle,
) -> Result<(), ApiError> {
    if out.is_null() {
        return Err(invalid("output element pointer is null"));
    }
    let token = register(LocalHandle::Element(ElementRef {
        document: Rc::downgrade(state),
        node,
    }))?;
    state
        .element_handles
        .try_borrow_mut()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "element handle map is borrowed"))?
        .entry(node)
        .or_default()
        .push(token);
    // SAFETY: the caller guarantees writable storage for one opaque pointer.
    unsafe { ptr::write(out, token as *mut OuiElement) };
    Ok(())
}

fn element_address(state: &DocumentState, node: NodeHandle) -> Result<usize, ApiError> {
    state
        .element_handles
        .try_borrow()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "element handle map is borrowed"))?
        .get(&node)
        .and_then(|handles| handles.last())
        .copied()
        .ok_or_else(|| ApiError::new(OuiStatus::InvalidHandle, "node has no live C handle"))
}

fn new_document(config: &OuiDocumentConfig) -> Result<Rc<DocumentState>, ApiError> {
    check_header(
        config.struct_size,
        config.abi_version,
        size_of::<OuiDocumentConfig>(),
    )?;
    let mut viewport = Viewport::new(config.width, config.height)?;
    if !config.scale_factor.is_finite() || config.scale_factor <= 0.0 {
        return Err(invalid("scale_factor must be finite and positive"));
    }
    viewport.scale_factor = config.scale_factor;
    Ok(Rc::new(DocumentState {
        engine: RefCell::new(Engine::new(viewport)?),
        update_depth: Cell::new(0),
        listeners: RefCell::new(Vec::new()),
        next_listener: Cell::new(1),
        element_handles: RefCell::new(std::collections::HashMap::new()),
    }))
}

fn element_tag(value: i32) -> Result<ElementTag, ApiError> {
    let tag = match value {
        0 => ElementTag::Div,
        1 => ElementTag::Span,
        2 => ElementTag::Break,
        3 => ElementTag::WordBreak,
        4 => ElementTag::Ruby,
        5 => ElementTag::RubyText,
        6 => ElementTag::Table,
        7 => ElementTag::TableCaption,
        8 => ElementTag::TableColumnGroup,
        9 => ElementTag::TableColumn,
        10 => ElementTag::TableHead,
        11 => ElementTag::TableBody,
        12 => ElementTag::TableFoot,
        13 => ElementTag::TableRow,
        14 => ElementTag::TableCell,
        15 => ElementTag::TableHeaderCell,
        16 => ElementTag::Image,
        17 => ElementTag::Canvas,
        18 => ElementTag::Svg,
        19 => ElementTag::IFrame,
        20 => ElementTag::Object,
        21 => ElementTag::Audio,
        22 => ElementTag::Video,
        23 => ElementTag::Input,
        24 => ElementTag::Button,
        25 => ElementTag::Meter,
        26 => ElementTag::Progress,
        27 => ElementTag::Fieldset,
        28 => ElementTag::Legend,
        29 => ElementTag::Details,
        30 => ElementTag::Summary,
        31 => ElementTag::TextArea,
        32 => ElementTag::Select,
        33 => ElementTag::Option,
        34 => ElementTag::OptGroup,
        35 => ElementTag::Form,
        36 => ElementTag::Embed,
        37 => ElementTag::Html,
        38 => ElementTag::Body,
        _ => return Err(invalid("unknown element tag")),
    };
    Ok(tag)
}

fn same_document(first: &ElementRef, second: &ElementRef) -> Result<Rc<DocumentState>, ApiError> {
    let first_document = element_document(first)?;
    let second_document = element_document(second)?;
    if !Rc::ptr_eq(&first_document, &second_document) {
        return Err(ApiError::new(
            OuiStatus::WrongDocument,
            "elements belong to different documents",
        ));
    }
    Ok(first_document)
}

fn with_element_mut(
    address: usize,
    operation: impl FnOnce(&mut Engine, NodeHandle) -> Result<(), openui_engine::EngineError>,
) -> Result<(), ApiError> {
    let element = element(address)?;
    let document = element_document(&element)?;
    let mut engine = borrow_engine_mut(&document)?;
    operation(&mut engine, element.node)?;
    Ok(())
}

fn copy_array<T: Copy>(
    data: *const T,
    length: usize,
    label: &'static str,
) -> Result<Vec<T>, ApiError> {
    if length == 0 {
        return Ok(Vec::new());
    }
    if data.is_null()
        || length > isize::MAX as usize / std::mem::size_of::<T>().max(1)
        || length > 4096
    {
        return Err(invalid(format!("invalid {label} array")));
    }
    let mut result = Vec::new();
    result
        .try_reserve_exact(length)
        .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "array allocation failed"))?;
    // SAFETY: the C contract requires `data` to reference `length` readable,
    // aligned `T` values for the call. Bounds and null were validated above.
    result.extend_from_slice(unsafe { std::slice::from_raw_parts(data, length) });
    Ok(result)
}

// SAFETY CONTRACT: no pointers are accepted; this constant query is reentrant.
#[no_mangle]
pub extern "C" fn oui_abi_version() -> u32 {
    OUI_ABI_VERSION
}

// SAFETY CONTRACT: `out_error` must be writable for its declared struct size.
#[no_mangle]
pub extern "C" fn oui_error_get_last(out_error: *mut OuiErrorInfo) -> OuiStatus {
    ffi_preserve(|| {
        let saved = last_error();
        if out_error.is_null() {
            return Err(invalid("out_error is null"));
        }
        // SAFETY: the caller guarantees a readable/writable `OuiErrorInfo`.
        let output = unsafe { &mut *out_error };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiErrorInfo>(),
        )?;
        output.status = saved.status;
        output.detail = saved.detail;
        output.message_length = saved.message.len();
        Ok(())
    })
}

// SAFETY CONTRACT: `out_length` is writable; `destination` is writable for
// `capacity` bytes when capacity is nonzero. Messages are not NUL-terminated.
#[no_mangle]
pub extern "C" fn oui_error_copy_message(
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
) -> OuiStatus {
    ffi_preserve(|| {
        let saved = last_error();
        if out_length.is_null() {
            return Err(invalid("out_length is null"));
        }
        // SAFETY: the caller guarantees writable storage for one `size_t`.
        unsafe { ptr::write(out_length, saved.message.len()) };
        if destination.is_null() && capacity == 0 {
            return Ok(());
        }
        if capacity < saved.message.len() {
            return Err(ApiError::new(
                OuiStatus::BufferTooSmall,
                "error message destination is too small",
            ));
        }
        if !saved.message.is_empty() {
            if destination.is_null() {
                return Err(invalid("destination is null"));
            }
            // SAFETY: capacity was checked and the caller guarantees that the
            // destination references that many writable bytes. Regions cannot
            // overlap because the source is Rust-owned TLS storage.
            unsafe {
                ptr::copy_nonoverlapping(saved.message.as_ptr(), destination, saved.message.len())
            };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `config` is readable and `out_app` is writable for one handle.
#[no_mangle]
pub extern "C" fn oui_app_create(
    config: *const OuiAppConfig,
    out_app: *mut *mut OuiApp,
) -> OuiStatus {
    ffi(|| {
        if config.is_null() {
            return Err(invalid("config is null"));
        }
        // SAFETY: the caller guarantees a readable `OuiAppConfig`.
        let config = unsafe { &*config };
        check_header(
            config.struct_size,
            config.abi_version,
            size_of::<OuiAppConfig>(),
        )?;
        if config.reserved != 0 {
            return Err(invalid("reserved app configuration fields must be zero"));
        }
        if config.backend > 2 {
            return Err(invalid("unknown backend preference"));
        }
        let title = utf8(config.title, "title")?;
        let document_config = OuiDocumentConfig {
            struct_size: size_of::<OuiDocumentConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            width: config.width,
            height: config.height,
            scale_factor: 1.0,
        };
        let state = Rc::new(AppState {
            document: new_document(&document_config)?,
            _title: title,
            _backend: config.backend,
        });
        write_handle(out_app, LocalHandle::App(state))
    })
}

// SAFETY CONTRACT: `app` is either null or an Open UI app handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_app_destroy(app: *mut OuiApp) -> OuiStatus {
    ffi(|| {
        destroy(app as usize, HandleKind::App)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `app` is a live app handle and `out_document` is writable.
#[no_mangle]
pub extern "C" fn oui_app_document(
    app: *mut OuiApp,
    out_document: *mut *mut OuiDocument,
) -> OuiStatus {
    ffi(|| {
        let app = match get(app as usize, HandleKind::App)? {
            LocalHandle::App(value) => value,
            _ => unreachable!("kind checked by registry"),
        };
        write_handle(out_document, LocalHandle::Document(app.document.clone()))
    })
}

// SAFETY CONTRACT: `config` is readable and `out_document` is writable.
#[no_mangle]
pub extern "C" fn oui_document_create(
    config: *const OuiDocumentConfig,
    out_document: *mut *mut OuiDocument,
) -> OuiStatus {
    ffi(|| {
        if config.is_null() {
            return Err(invalid("config is null"));
        }
        // SAFETY: the caller guarantees a readable `OuiDocumentConfig`.
        let config = unsafe { &*config };
        write_handle(out_document, LocalHandle::Document(new_document(config)?))
    })
}

// SAFETY CONTRACT: `document` is a live document handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_document_destroy(document: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        destroy(document as usize, HandleKind::Document)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and `out_root` is writable.
#[no_mangle]
pub extern "C" fn oui_document_root(
    document_handle: *mut OuiDocument,
    out_root: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_root.is_null() {
            return Err(invalid("out_root is null"));
        }
        let state = document(document_handle as usize)?;
        let node = borrow_engine(&state)?.root();
        write_element_handle(out_root, &state, node)
    })
}

// SAFETY CONTRACT: both pointers reference live/readable objects for this call.
#[no_mangle]
pub extern "C" fn oui_document_set_viewport(
    document_handle: *mut OuiDocument,
    viewport: *const OuiDocumentConfig,
) -> OuiStatus {
    ffi(|| {
        if viewport.is_null() {
            return Err(invalid("viewport is null"));
        }
        // SAFETY: the caller guarantees a readable `OuiDocumentConfig`.
        let config = unsafe { &*viewport };
        check_header(
            config.struct_size,
            config.abi_version,
            size_of::<OuiDocumentConfig>(),
        )?;
        let mut viewport = Viewport::new(config.width, config.height)?;
        viewport.scale_factor = config.scale_factor;
        let state = document(document_handle as usize)?;
        borrow_engine_mut(&state)?.set_viewport(viewport)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is a live document handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_document_begin_update(document_handle: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        let depth = state
            .update_depth
            .get()
            .checked_add(1)
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "update depth overflow"))?;
        state.update_depth.set(depth);
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and has a matching begin-update call.
#[no_mangle]
pub extern "C" fn oui_document_end_update(document_handle: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        let depth = state.update_depth.get();
        if depth == 0 {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "end_update without begin_update",
            ));
        }
        state.update_depth.set(depth - 1);
        if depth == 1 {
            borrow_engine_mut(&state)?.update()?;
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and no update transaction is open.
#[no_mangle]
pub extern "C" fn oui_document_update(document_handle: *mut OuiDocument) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        if state.update_depth.get() != 0 {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "cannot update during a transaction",
            ));
        }
        borrow_engine_mut(&state)?.update()?;
        Ok(())
    })
}

fn render_frame(document_handle: *mut OuiDocument) -> Result<openui_compositor::Frame, ApiError> {
    let state = document(document_handle as usize)?;
    if state.update_depth.get() != 0 {
        return Err(ApiError::new(
            OuiStatus::InvalidState,
            "cannot render during a transaction",
        ));
    }
    let scene = borrow_engine_mut(&state)?.scene()?;
    Ok(SoftwareCompositor::default().render(&scene)?)
}

// SAFETY CONTRACT: `document` is live; `out_bitmap` is readable/writable and initialized.
#[no_mangle]
pub extern "C" fn oui_document_render_rgba(
    document_handle: *mut OuiDocument,
    out_bitmap: *mut OuiBitmap,
) -> OuiStatus {
    ffi(|| {
        if out_bitmap.is_null() {
            return Err(invalid("out_bitmap is null"));
        }
        // SAFETY: the caller guarantees a readable/writable `OuiBitmap`.
        let output = unsafe { &mut *out_bitmap };
        check_header(
            output.struct_size,
            output.abi_version,
            size_of::<OuiBitmap>(),
        )?;
        if !output.pixels.is_null() {
            return Err(invalid(
                "out_bitmap.pixels must be null; destroy the previous buffer first",
            ));
        }
        let frame = render_frame(document_handle)?;
        let buffer = register(LocalHandle::Buffer(Rc::new(frame.pixels)))?;
        output.width = frame.width;
        output.height = frame.height;
        output.stride = frame.stride;
        output.pixels = buffer as *mut OuiBuffer;
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live and `out_buffer` is writable.
#[no_mangle]
pub extern "C" fn oui_document_render_png(
    document_handle: *mut OuiDocument,
    out_buffer: *mut *mut OuiBuffer,
) -> OuiStatus {
    ffi(|| {
        if out_buffer.is_null() {
            return Err(invalid("out_buffer is null"));
        }
        let state = document(document_handle as usize)?;
        if state.update_depth.get() != 0 {
            return Err(ApiError::new(
                OuiStatus::InvalidState,
                "cannot render during a transaction",
            ));
        }
        let scene = borrow_engine_mut(&state)?.scene()?;
        let png = SoftwareCompositor::default().render_png(&scene)?;
        write_handle(out_buffer, LocalHandle::Buffer(Rc::new(png)))
    })
}

// SAFETY CONTRACT: `document` is live and `out_element` is writable.
#[no_mangle]
pub extern "C" fn oui_element_create(
    document_handle: *mut OuiDocument,
    tag: i32,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() {
            return Err(invalid("out_element is null"));
        }
        let state = document(document_handle as usize)?;
        let node = borrow_engine_mut(&state)?.create_element(element_tag(tag)?)?;
        write_element_handle(out_element, &state, node)
    })
}

// SAFETY CONTRACT: `document` is live, `text` is readable, and `out_text` is writable.
#[no_mangle]
pub extern "C" fn oui_text_create(
    document_handle: *mut OuiDocument,
    text: OuiUtf8,
    out_text: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_text.is_null() {
            return Err(invalid("out_text is null"));
        }
        let state = document(document_handle as usize)?;
        let text = utf8(text, "text")?;
        let node = borrow_engine_mut(&state)?.create_text(text)?;
        write_element_handle(out_text, &state, node)
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
// Destroying the handle does not remove its retained node.
#[no_mangle]
pub extern "C" fn oui_element_destroy(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        if let Some(state) = element.document.upgrade() {
            state
                .listeners
                .try_borrow_mut()
                .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
                .retain(|record| record.element_address != element_handle as usize);
            if let Ok(mut handles) = state.element_handles.try_borrow_mut() {
                if let Some(addresses) = handles.get_mut(&element.node) {
                    addresses.retain(|address| *address != element_handle as usize);
                    if addresses.is_empty() {
                        handles.remove(&element.node);
                    }
                }
            }
        }
        destroy(element_handle as usize, HandleKind::Element)?;
        Ok(())
    })
}

// SAFETY CONTRACT: both handles are live and owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_append_child(
    parent: *mut OuiElement,
    child: *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        let parent = element(parent as usize)?;
        let child = element(child as usize)?;
        let state = same_document(&parent, &child)?;
        borrow_engine_mut(&state)?.append_child(parent.node, child.node)?;
        Ok(())
    })
}

// SAFETY CONTRACT: all handles are live and owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_insert_before(
    parent: *mut OuiElement,
    child: *mut OuiElement,
    before: *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        let parent = element(parent as usize)?;
        let child = element(child as usize)?;
        let before = element(before as usize)?;
        let state = same_document(&parent, &child)?;
        if !Rc::ptr_eq(&state, &element_document(&before)?) {
            return Err(ApiError::new(
                OuiStatus::WrongDocument,
                "elements belong to different documents",
            ));
        }
        borrow_engine_mut(&state)?.insert_before(parent.node, child.node, before.node)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_remove(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::remove))
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_remove_all_children(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::remove_children))
}

// SAFETY CONTRACT: `element` is live and `text` is readable for its length.
#[no_mangle]
pub extern "C" fn oui_element_set_text(
    element_handle: *mut OuiElement,
    text: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let text = utf8(text, "text")?;
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_text(node, text)
        })
    })
}

// SAFETY CONTRACT: `element` is live; name and value slices are readable.
#[no_mangle]
pub extern "C" fn oui_element_set_attribute(
    element_handle: *mut OuiElement,
    name: OuiUtf8,
    value: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let name = utf8(name, "attribute name")?;
        if name.is_empty() {
            return Err(invalid("attribute name is empty"));
        }
        let value = utf8(value, "attribute value")?;
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_attribute(node, name, value)
        })
    })
}

// SAFETY CONTRACT: `element` is live and the name slice is readable.
#[no_mangle]
pub extern "C" fn oui_element_remove_attribute(
    element_handle: *mut OuiElement,
    name: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let name = utf8(name, "attribute name")?;
        if name.is_empty() {
            return Err(invalid("attribute name is empty"));
        }
        with_element_mut(element_handle as usize, |engine, node| {
            engine.remove_attribute(node, &name).map(|_| ())
        })
    })
}

// SAFETY CONTRACT: `element` is live and `value` points to a readable tagged value.
#[no_mangle]
pub extern "C" fn oui_element_set_property(
    element_handle: *mut OuiElement,
    property: i32,
    value: *const OuiStyleValue,
) -> OuiStatus {
    ffi(|| {
        let property = property_from_raw(property)
            .ok_or_else(|| invalid("unknown style property identifier"))?;
        if value.is_null() {
            return Err(invalid("style value is null"));
        }
        // SAFETY: the caller guarantees one readable `OuiStyleValue`.
        let value = value::style_value(property, unsafe { &*value })?;
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_property(node, property, value)
        })
    })
}

// SAFETY CONTRACT: `element` is live and `out_rect` is writable.
#[no_mangle]
pub extern "C" fn oui_element_get_bounds(
    element_handle: *mut OuiElement,
    out_rect: *mut OuiRect,
) -> OuiStatus {
    ffi(|| {
        if out_rect.is_null() {
            return Err(invalid("out_rect is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let bounds = borrow_engine_mut(&state)?
            .bounds(element.node)?
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element has no layout box"))?;
        // SAFETY: the caller guarantees writable storage for one `OuiRect`.
        unsafe {
            ptr::write(
                out_rect,
                OuiRect {
                    x: bounds.x,
                    y: bounds.y,
                    width: bounds.width,
                    height: bounds.height,
                },
            )
        };
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live; offsets must be finite.
#[no_mangle]
pub extern "C" fn oui_element_scroll_to(
    element_handle: *mut OuiElement,
    x: f64,
    y: f64,
) -> OuiStatus {
    ffi(|| {
        with_element_mut(element_handle as usize, |engine, node| {
            engine.scroll_to(node, x, y)
        })
    })
}

// SAFETY CONTRACT: `element` is live and both output pointers are writable.
#[no_mangle]
pub extern "C" fn oui_element_get_scroll_offset(
    element_handle: *mut OuiElement,
    out_x: *mut f64,
    out_y: *mut f64,
) -> OuiStatus {
    ffi(|| {
        if out_x.is_null() || out_y.is_null() {
            return Err(invalid("scroll output pointer is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let (x, y) = borrow_engine(&state)?.scroll_offset(element.node)?;
        // SAFETY: both pointers are caller-provided writable `double` slots.
        unsafe {
            ptr::write(out_x, x);
            ptr::write(out_y, y);
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_focus(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::focus))
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_blur(element_handle: *mut OuiElement) -> OuiStatus {
    ffi(|| with_element_mut(element_handle as usize, Engine::blur))
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_set_pointer_capture(
    element_handle: *mut OuiElement,
    pointer_id: u32,
) -> OuiStatus {
    ffi(|| {
        with_element_mut(element_handle as usize, |engine, node| {
            engine.set_pointer_capture(pointer_id as u64, node)
        })
    })
}

// SAFETY CONTRACT: `element` is a live element handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_element_release_pointer_capture(
    element_handle: *mut OuiElement,
    pointer_id: u32,
) -> OuiStatus {
    ffi(|| {
        with_element_mut(element_handle as usize, |engine, node| {
            engine.release_pointer_capture(pointer_id as u64, node)
        })
    })
}

// SAFETY CONTRACT: `element` is live, callback is callable for the listener
// lifetime, user_data obeys application ownership, and `out_listener` is writable.
#[no_mangle]
pub extern "C" fn oui_element_add_event_listener(
    element_handle: *mut OuiElement,
    event_type: i32,
    capture: u8,
    callback: Option<OuiEventCallback>,
    user_data: *mut c_void,
    out_listener: *mut *mut OuiListener,
) -> OuiStatus {
    ffi(|| {
        if out_listener.is_null() {
            return Err(invalid("out_listener is null"));
        }
        if event_type <= 0 || !valid_event_type(event_type as u32) {
            return Err(invalid("unknown event type"));
        }
        if capture > 1 {
            return Err(invalid("capture must be zero or one"));
        }
        let callback = callback.ok_or_else(|| invalid("event callback is null"))?;
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine(&state)?.element_tag(element.node)?;
        let id = state.next_listener.get();
        state.next_listener.set(
            id.checked_add(1)
                .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "listener ID overflow"))?,
        );
        state
            .listeners
            .try_borrow_mut()
            .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
            .push(ListenerRecord {
                id,
                node: element.node,
                event_type: event_type as u32,
                capture: capture != 0,
                callback,
                user_data,
                element_address: element_handle as usize,
            });
        write_handle(
            out_listener,
            LocalHandle::Listener(ListenerRef {
                document: Rc::downgrade(&state),
                id,
            }),
        )
    })
}

// SAFETY CONTRACT: `listener` is a live listener handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_listener_destroy(listener: *mut OuiListener) -> OuiStatus {
    ffi(|| {
        let listener = match destroy(listener as usize, HandleKind::Listener)? {
            LocalHandle::Listener(value) => value,
            _ => unreachable!("kind checked by registry"),
        };
        if let Some(state) = listener.document.upgrade() {
            state
                .listeners
                .try_borrow_mut()
                .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
                .retain(|record| record.id != listener.id);
        }
        Ok(())
    })
}

fn invoke_event_listeners(
    state: &DocumentState,
    node: NodeHandle,
    capture: bool,
    event: &mut OuiEvent,
) -> Result<(), ApiError> {
    let callbacks: Vec<_> = state
        .listeners
        .try_borrow()
        .map_err(|_| ApiError::new(OuiStatus::Reentrant, "listener list is borrowed"))?
        .iter()
        .filter(|listener| {
            listener.node == node
                && listener.event_type == event.event_type
                && listener.capture == capture
        })
        .cloned()
        .collect();
    for listener in callbacks {
        event.current_target = listener.element_address as *mut OuiElement;
        // SAFETY: listener registration requires the callback to remain
        // callable and user_data to remain valid until listener destruction.
        // No engine or listener-list borrow is held, so reentrant API calls are
        // explicitly permitted. The event pointer lives through the call.
        unsafe { (listener.callback)(event, listener.user_data) };
    }
    Ok(())
}

fn validate_event(event: &OuiEvent) -> Result<(), ApiError> {
    check_header(event.struct_size, event.abi_version, size_of::<OuiEvent>())?;
    if !valid_event_type(event.event_type) {
        return Err(invalid("unknown event type"));
    }
    if event.flags & !(OUI_EVENT_FLAG_DEFAULT_PREVENTED | OUI_EVENT_FLAG_PROPAGATION_STOPPED) != 0
        || event.modifiers & !0x1f != 0
    {
        return Err(invalid("event contains unknown flag bits"));
    }
    utf8(event.text, "event text")?;
    if !event.x.is_finite()
        || !event.y.is_finite()
        || !event.delta_x.is_finite()
        || !event.delta_y.is_finite()
    {
        return Err(invalid("event coordinates and deltas must be finite"));
    }
    Ok(())
}

fn dispatch_event_to(
    state: &Rc<DocumentState>,
    target: NodeHandle,
    target_address: usize,
    event: &mut OuiEvent,
    apply_default: bool,
) -> Result<(), ApiError> {
    let route = borrow_engine(state)?.event_route(target)?;
    event.flags &= !(OUI_EVENT_FLAG_DEFAULT_PREVENTED | OUI_EVENT_FLAG_PROPAGATION_STOPPED);
    event.target = target_address as *mut OuiElement;
    for step in route.steps {
        match step.phase {
            EngineEventPhase::Capture => {
                event.phase = 1;
                invoke_event_listeners(state, step.node, true, event)?;
            }
            EngineEventPhase::Target => {
                event.phase = 2;
                invoke_event_listeners(state, step.node, true, event)?;
                invoke_event_listeners(state, step.node, false, event)?;
            }
            EngineEventPhase::Bubble => {
                event.phase = 3;
                invoke_event_listeners(state, step.node, false, event)?;
            }
        }
        if event.flags & OUI_EVENT_FLAG_PROPAGATION_STOPPED != 0 {
            break;
        }
    }
    event.current_target = ptr::null_mut();
    if apply_default && event.flags & OUI_EVENT_FLAG_DEFAULT_PREVENTED == 0 {
        let text = utf8(event.text, "event text")?;
        match event.event_type {
            4 => {
                let changed = borrow_engine_mut(state)?.activate(target)?.changed;
                for changed in changed {
                    let Ok(address) = element_address(state, changed) else {
                        continue;
                    };
                    for event_type in [16, 17] {
                        let mut derived = *event;
                        derived.event_type = event_type;
                        dispatch_event_to(state, changed, address, &mut derived, false)?;
                    }
                }
            }
            8 => borrow_engine_mut(state)?.insert_text(target, &text)?,
            10 => borrow_engine_mut(state)?.update_composition(target, &text)?,
            11 => borrow_engine_mut(state)?.finish_composition(target)?,
            12 => {
                borrow_engine_mut(state)?.focus_with_origin(target, FocusOrigin::Accessibility)?;
            }
            13 => borrow_engine_mut(state)?.blur(target)?,
            _ => {}
        }
    }
    Ok(())
}

// SAFETY CONTRACT: document and target are live same-document handles; `event`
// is readable/writable for its declared size through all synchronous callbacks.
#[no_mangle]
pub extern "C" fn oui_document_dispatch_event(
    document_handle: *mut OuiDocument,
    target_handle: *mut OuiElement,
    event: *mut OuiEvent,
) -> OuiStatus {
    ffi(|| {
        if event.is_null() {
            return Err(invalid("event is null"));
        }
        // SAFETY: the caller guarantees a readable/writable `OuiEvent` that
        // remains alive for this synchronous dispatch.
        let event = unsafe { &mut *event };
        validate_event(event)?;
        let state = document(document_handle as usize)?;
        let target = element(target_handle as usize)?;
        if !Rc::ptr_eq(&state, &element_document(&target)?) {
            return Err(ApiError::new(
                OuiStatus::WrongDocument,
                "event target belongs to another document",
            ));
        }
        dispatch_event_to(&state, target.node, target_handle as usize, event, true)
    })
}

// SAFETY CONTRACT: `document` is live and `event` is readable/writable for its
// declared size through all synchronous callbacks. The event type is pointer
// down, up, or move; hit testing supplies the target.
#[no_mangle]
pub extern "C" fn oui_document_dispatch_pointer_event(
    document_handle: *mut OuiDocument,
    event: *mut OuiEvent,
) -> OuiStatus {
    ffi(|| {
        if event.is_null() {
            return Err(invalid("event is null"));
        }
        // SAFETY: the caller guarantees a live, writable event for this call.
        let event = unsafe { &mut *event };
        validate_event(event)?;
        let kind = match event.event_type {
            1 => PointerEventKind::Down,
            2 => PointerEventKind::Up,
            3 => PointerEventKind::Move,
            _ => return Err(invalid("pointer dispatch requires a pointer event type")),
        };
        let state = document(document_handle as usize)?;
        let update = borrow_engine_mut(&state)?.pointer_event(
            event.pointer_id as u64,
            kind,
            event.x,
            event.y,
        )?;
        for (nodes, event_type) in [
            (update.left.as_slice(), 15),
            (update.entered.as_slice(), 14),
        ] {
            for node in nodes {
                let Ok(address) = element_address(&state, *node) else {
                    continue;
                };
                let mut boundary_event = *event;
                boundary_event.event_type = event_type;
                dispatch_event_to(&state, *node, address, &mut boundary_event, false)?;
            }
        }
        let Some(target) = update.target else {
            event.target = ptr::null_mut();
            event.current_target = ptr::null_mut();
            return Ok(());
        };
        let address = element_address(&state, target)?;
        dispatch_event_to(&state, target, address, event, false)?;
        if event.flags & OUI_EVENT_FLAG_DEFAULT_PREVENTED != 0 {
            return Ok(());
        }
        if kind == PointerEventKind::Down {
            let _ = borrow_engine_mut(&state)?.focus_with_origin(target, FocusOrigin::Pointer);
        }
        if let Some((changed, fraction)) = update.range_value {
            let did_change = borrow_engine_mut(&state)?.set_range_fraction(changed, fraction)?;
            let changed_address = element_address(&state, changed)?;
            if did_change {
                let mut input = *event;
                input.event_type = 16;
                dispatch_event_to(&state, changed, changed_address, &mut input, false)?;
            }
            if kind == PointerEventKind::Up {
                let mut change = *event;
                change.event_type = 17;
                dispatch_event_to(&state, changed, changed_address, &mut change, false)?;
            }
        }
        if let Some(activation) = update.activation {
            let activation_address = element_address(&state, activation)?;
            let mut click = *event;
            click.event_type = 4;
            dispatch_event_to(&state, activation, activation_address, &mut click, true)?;
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live, coordinates are finite, and
// `out_element` is writable. The returned pointer is a borrowed existing
// element handle and must not outlive that handle.
#[no_mangle]
pub extern "C" fn oui_document_hit_test(
    document_handle: *mut OuiDocument,
    x: f32,
    y: f32,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() || !x.is_finite() || !y.is_finite() {
            return Err(invalid("hit-test output and coordinates are invalid"));
        }
        // SAFETY: the caller guarantees storage for one output pointer.
        unsafe { ptr::write(out_element, ptr::null_mut()) };
        let state = document(document_handle as usize)?;
        if let Some(node) = borrow_engine_mut(&state)?.hit_test(x, y)? {
            let address = element_address(&state, node)?;
            // SAFETY: output storage was validated above.
            unsafe { ptr::write(out_element, address as *mut OuiElement) };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live, direction is -1 or 1, and
// `out_element` is writable. The returned pointer is borrowed.
#[no_mangle]
pub extern "C" fn oui_document_advance_focus(
    document_handle: *mut OuiDocument,
    direction: i32,
    out_element: *mut *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        if out_element.is_null() {
            return Err(invalid("focused element output is null"));
        }
        // SAFETY: the caller guarantees storage for one output pointer.
        unsafe { ptr::write(out_element, ptr::null_mut()) };
        let state = document(document_handle as usize)?;
        if let Some(node) = borrow_engine_mut(&state)?.advance_focus(direction)? {
            let address = element_address(&state, node)?;
            // SAFETY: output storage was validated above.
            unsafe { ptr::write(out_element, address as *mut OuiElement) };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `document` is live. `root` is null to leave modal mode or
// is a live element handle from the same document.
#[no_mangle]
pub extern "C" fn oui_document_set_modal_root(
    document_handle: *mut OuiDocument,
    root_handle: *mut OuiElement,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        let root = if root_handle.is_null() {
            None
        } else {
            let root = element(root_handle as usize)?;
            if !Rc::ptr_eq(&state, &element_document(&root)?) {
                return Err(ApiError::new(
                    OuiStatus::WrongDocument,
                    "modal root belongs to another document",
                ));
            }
            Some(root.node)
        };
        borrow_engine_mut(&state)?.set_modal_root(root)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and `value` is readable for its length.
#[no_mangle]
pub extern "C" fn oui_element_set_control_value(
    element_handle: *mut OuiElement,
    value: OuiUtf8,
) -> OuiStatus {
    ffi(|| {
        let value = utf8(value, "control value")?;
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_control_value(element.node, value)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live, `out_length` is writable, and
// `destination` is writable for capacity bytes when capacity is nonzero.
#[no_mangle]
pub extern "C" fn oui_element_copy_control_value(
    element_handle: *mut OuiElement,
    destination: *mut u8,
    capacity: usize,
    out_length: *mut usize,
) -> OuiStatus {
    ffi(|| {
        if out_length.is_null() {
            return Err(invalid("control value length output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let value = borrow_engine(&state)?
            .control_state(element.node)?
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element is not a control"))?
            .value
            .as_bytes()
            .to_vec();
        // SAFETY: output storage for one size_t is guaranteed by the caller.
        unsafe { ptr::write(out_length, value.len()) };
        if destination.is_null() && capacity == 0 {
            return Ok(());
        }
        if capacity < value.len() {
            return Err(ApiError::new(
                OuiStatus::BufferTooSmall,
                "control value destination is too small",
            ));
        }
        if !value.is_empty() {
            if destination.is_null() {
                return Err(invalid("control value destination is null"));
            }
            // SAFETY: capacity was validated and source/destination do not overlap.
            unsafe { ptr::copy_nonoverlapping(value.as_ptr(), destination, value.len()) };
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live editable control handle.
#[no_mangle]
pub extern "C" fn oui_element_set_selection(
    element_handle: *mut OuiElement,
    anchor: usize,
    focus: usize,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_selection(element.node, anchor, focus)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and both output pointers are writable.
#[no_mangle]
pub extern "C" fn oui_element_get_selection(
    element_handle: *mut OuiElement,
    out_anchor: *mut usize,
    out_focus: *mut usize,
) -> OuiStatus {
    ffi(|| {
        if out_anchor.is_null() || out_focus.is_null() {
            return Err(invalid("selection output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let selection = borrow_engine(&state)?
            .control_state(element.node)?
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element is not a control"))?
            .selection();
        // SAFETY: both output pointers were validated above.
        unsafe {
            ptr::write(out_anchor, selection.0);
            ptr::write(out_focus, selection.1);
        }
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is live and `out_flags` is writable.
#[no_mangle]
pub extern "C" fn oui_element_get_control_flags(
    element_handle: *mut OuiElement,
    out_flags: *mut u32,
) -> OuiStatus {
    ffi(|| {
        if out_flags.is_null() {
            return Err(invalid("control flags output is null"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let control = borrow_engine(&state)?
            .control_state(element.node)?
            .cloned()
            .ok_or_else(|| ApiError::new(OuiStatus::InvalidState, "element is not a control"))?;
        let flags = (control.disabled as u32) * OUI_CONTROL_DISABLED
            | (control.checked as u32) * OUI_CONTROL_CHECKED
            | (control.selected as u32) * OUI_CONTROL_SELECTED
            | (control.open as u32) * OUI_CONTROL_OPEN
            | (control.indeterminate as u32) * OUI_CONTROL_INDETERMINATE
            | (control.password as u32) * OUI_CONTROL_PASSWORD;
        // SAFETY: output storage was validated above.
        unsafe { ptr::write(out_flags, flags) };
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live checkbox/radio and checked is 0 or 1.
#[no_mangle]
pub extern "C" fn oui_element_set_checked(
    element_handle: *mut OuiElement,
    checked: u8,
) -> OuiStatus {
    ffi(|| {
        if checked > 1 {
            return Err(invalid("checked must be zero or one"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_checked(element.node, checked != 0)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live checkbox and indeterminate is 0 or 1.
#[no_mangle]
pub extern "C" fn oui_element_set_indeterminate(
    element_handle: *mut OuiElement,
    indeterminate: u8,
) -> OuiStatus {
    ffi(|| {
        if indeterminate > 1 {
            return Err(invalid("indeterminate must be zero or one"));
        }
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.set_indeterminate(element.node, indeterminate != 0)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `element` is a live adjustable control and adjustment is
// a declared OuiControlAdjustment value.
#[no_mangle]
pub extern "C" fn oui_element_adjust_control(
    element_handle: *mut OuiElement,
    adjustment: i32,
) -> OuiStatus {
    ffi(|| {
        let adjustment = match adjustment {
            0 => ControlAdjustment::Previous,
            1 => ControlAdjustment::Next,
            2 => ControlAdjustment::PageBackward,
            3 => ControlAdjustment::PageForward,
            4 => ControlAdjustment::Minimum,
            5 => ControlAdjustment::Maximum,
            _ => return Err(invalid("unknown control adjustment")),
        };
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine_mut(&state)?.adjust_control(element.node, adjustment)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `values` references four readable lengths and `out_compound` is writable.
#[no_mangle]
pub extern "C" fn oui_edges_create(
    values: *const OuiLength,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let values = copy_array(values, 4, "edge")?;
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Edges(Edges {
                top: value::length(values[0])?,
                right: value::length(values[1])?,
                bottom: value::length(values[2])?,
                left: value::length(values[3])?,
            })),
        )
    })
}

// SAFETY CONTRACT: `out_compound` is writable; both lengths are initialized.
#[no_mangle]
pub extern "C" fn oui_gap_create(
    row: OuiLength,
    column: OuiLength,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Gap(Gap {
                row: value::length(row)?,
                column: value::length(column)?,
            })),
        )
    })
}

fn generic_font(value: i32) -> Result<GenericFontFamily, ApiError> {
    match value {
        0 => Ok(GenericFontFamily::None),
        1 => Ok(GenericFontFamily::Serif),
        2 => Ok(GenericFontFamily::SansSerif),
        3 => Ok(GenericFontFamily::Monospace),
        4 => Ok(GenericFontFamily::Cursive),
        5 => Ok(GenericFontFamily::Fantasy),
        6 => Ok(GenericFontFamily::SystemUi),
        7 => Ok(GenericFontFamily::Math),
        8 => Ok(GenericFontFamily::Emoji),
        9 => Ok(GenericFontFamily::FangSong),
        10 => Ok(GenericFontFamily::UiSerif),
        11 => Ok(GenericFontFamily::UiSansSerif),
        12 => Ok(GenericFontFamily::UiMonospace),
        13 => Ok(GenericFontFamily::UiRounded),
        _ => Err(invalid("unknown generic font family")),
    }
}

// SAFETY CONTRACT: `name` is readable and `out_compound` is writable. A
// nonzero generic family ignores name; zero requires a nonempty named family.
#[no_mangle]
pub extern "C" fn oui_font_family_create(
    name: OuiUtf8,
    generic_family: i32,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let family = if generic_family == 0 {
            let name = utf8(name, "font family")?;
            if name.is_empty() {
                return Err(invalid("named font family is empty"));
            }
            FontFamilyList::single(name)
        } else {
            FontFamilyList::generic(generic_font(generic_family)?)
        };
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::FontFamily(family)),
        )
    })
}

fn border_style(value: i32) -> Result<BorderStyle, ApiError> {
    match value {
        0 => Ok(BorderStyle::None),
        1 => Ok(BorderStyle::Hidden),
        2 => Ok(BorderStyle::Dotted),
        3 => Ok(BorderStyle::Dashed),
        4 => Ok(BorderStyle::Solid),
        5 => Ok(BorderStyle::Double),
        6 => Ok(BorderStyle::Groove),
        7 => Ok(BorderStyle::Ridge),
        8 => Ok(BorderStyle::Inset),
        9 => Ok(BorderStyle::Outset),
        _ => Err(invalid("unknown border style")),
    }
}

fn color(value: OuiColor) -> Color {
    Color::from_rgba8(value.red, value.green, value.blue, value.alpha)
}

// SAFETY CONTRACT: `out_compound` is writable and scalar inputs are initialized.
#[no_mangle]
pub extern "C" fn oui_border_create(
    width: f32,
    style: i32,
    color_value: OuiColor,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        if !width.is_finite() || width < 0.0 {
            return Err(invalid("border width must be finite and non-negative"));
        }
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Border(Border {
                width,
                style: border_style(style)?,
                color: color(color_value),
            })),
        )
    })
}

// SAFETY CONTRACT: `values` references four readable lengths and `out_compound` is writable.
#[no_mangle]
pub extern "C" fn oui_corner_radii_create(
    values: *const OuiLength,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let values = copy_array(values, 4, "corner radius")?;
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::CornerRadii(CornerRadii(Edges {
                top: value::length(values[0])?,
                right: value::length(values[1])?,
                bottom: value::length(values[2])?,
                left: value::length(values[3])?,
            }))),
        )
    })
}

// SAFETY CONTRACT: `operations` references `operation_count` readable records
// (at most 4096), and `out_compound` is writable.
#[no_mangle]
pub extern "C" fn oui_transform_create(
    operations: *const OuiTransformOperation,
    operation_count: usize,
    out_compound: *mut *mut OuiStyleCompound,
) -> OuiStatus {
    ffi(|| {
        let operations = copy_array(operations, operation_count, "transform operation")?;
        let mut result = Vec::new();
        result
            .try_reserve_exact(operations.len())
            .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "transform allocation failed"))?;
        for operation in operations {
            if operation.reserved != 0 || operation.values.iter().any(|value| !value.is_finite()) {
                return Err(invalid("invalid transform operation payload"));
            }
            result.push(match operation.kind {
                1 => TransformOperation::Translate(
                    value::length(operation.x)?,
                    value::length(operation.y)?,
                ),
                2 => TransformOperation::Scale(operation.values[0], operation.values[1]),
                3 => TransformOperation::Rotate(operation.values[0]),
                4 => TransformOperation::Matrix(Transform2D {
                    a: operation.values[0],
                    b: operation.values[1],
                    c: operation.values[2],
                    d: operation.values[3],
                    e: operation.values[4],
                    f: operation.values[5],
                }),
                _ => return Err(invalid("unknown transform operation kind")),
            });
        }
        write_handle(
            out_compound,
            LocalHandle::Compound(StyleValue::Transform(TransformList(result))),
        )
    })
}

// SAFETY CONTRACT: `compound` is a live immutable compound handle.
#[no_mangle]
pub extern "C" fn oui_style_compound_destroy(compound: *mut OuiStyleCompound) -> OuiStatus {
    ffi(|| {
        destroy(compound as usize, HandleKind::Compound)?;
        Ok(())
    })
}

// SAFETY CONTRACT: document is live; all slices are readable for their
// lengths; `out_resource` is writable. Input bytes are copied synchronously.
#[no_mangle]
pub extern "C" fn oui_document_register_image(
    document_handle: *mut OuiDocument,
    source: OuiUtf8,
    mime_type: OuiUtf8,
    sha256: OuiUtf8,
    data: *const u8,
    byte_length: usize,
    out_resource: *mut *mut OuiResource,
) -> OuiStatus {
    ffi(|| {
        if out_resource.is_null() {
            return Err(invalid("out_resource is null"));
        }
        let state = document(document_handle as usize)?;
        let source = utf8(source, "resource source")?;
        let mime_type = utf8(mime_type, "resource MIME type")?;
        let sha256 = utf8(sha256, "resource hash")?;
        let data = bytes(data, byte_length)?;
        let resource =
            borrow_engine_mut(&state)?.register_image_resource(source, mime_type, sha256, data);
        write_handle(
            out_resource,
            LocalHandle::Resource(ResourceRef {
                document: Rc::downgrade(&state),
                resource,
            }),
        )
    })
}

// SAFETY CONTRACT: both handles are live; intrinsic dimensions are both zero
// (unspecified) or both finite and positive.
#[no_mangle]
pub extern "C" fn oui_element_set_image(
    element_handle: *mut OuiElement,
    resource_handle: *mut OuiResource,
    intrinsic_width: f32,
    intrinsic_height: f32,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let resource = match get(resource_handle as usize, HandleKind::Resource)? {
            LocalHandle::Resource(value) => value,
            _ => unreachable!("kind checked by registry"),
        };
        let resource_document = resource.document.upgrade().ok_or_else(|| {
            ApiError::new(OuiStatus::InvalidHandle, "resource document was destroyed")
        })?;
        if !Rc::ptr_eq(&state, &resource_document) {
            return Err(ApiError::new(
                OuiStatus::WrongDocument,
                "resource belongs to another document",
            ));
        }
        let dimensions = match (intrinsic_width, intrinsic_height) {
            (0.0, 0.0) => None,
            (width, height)
                if width.is_finite() && height.is_finite() && width > 0.0 && height > 0.0 =>
            {
                Some((width, height))
            }
            _ => return Err(invalid("invalid intrinsic image dimensions")),
        };
        borrow_engine_mut(&state)?.set_image_resource(
            element.node,
            resource.resource,
            dimensions,
        )?;
        Ok(())
    })
}

// SAFETY CONTRACT: `resource` is a live resource handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_resource_destroy(resource: *mut OuiResource) -> OuiStatus {
    ffi(|| {
        destroy(resource as usize, HandleKind::Resource)?;
        Ok(())
    })
}

// SAFETY CONTRACT: `buffer` is a live buffer handle. Returned bytes remain
// readable until the buffer is destroyed and must not be mutated or freed.
#[no_mangle]
pub extern "C" fn oui_buffer_data(buffer: *const OuiBuffer) -> *const u8 {
    ffi_value(ptr::null(), || {
        match get(buffer as usize, HandleKind::Buffer)? {
            LocalHandle::Buffer(value) => Ok(value.as_ptr()),
            _ => unreachable!("kind checked by registry"),
        }
    })
}

// SAFETY CONTRACT: `buffer` is a live buffer handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_buffer_length(buffer: *const OuiBuffer) -> usize {
    ffi_value(0, || match get(buffer as usize, HandleKind::Buffer)? {
        LocalHandle::Buffer(value) => Ok(value.len()),
        _ => unreachable!("kind checked by registry"),
    })
}

// SAFETY CONTRACT: `buffer` is a live buffer handle owned by this thread.
#[no_mangle]
pub extern "C" fn oui_buffer_destroy(buffer: *mut OuiBuffer) -> OuiStatus {
    ffi(|| {
        destroy(buffer as usize, HandleKind::Buffer)?;
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::{align_of, size_of};

    fn empty_utf8() -> OuiUtf8 {
        OuiUtf8 {
            data: ptr::null(),
            length: 0,
        }
    }

    fn text(value: &str) -> OuiUtf8 {
        OuiUtf8 {
            data: value.as_ptr(),
            length: value.len(),
        }
    }

    fn document_config(width: u32, height: u32) -> OuiDocumentConfig {
        OuiDocumentConfig {
            struct_size: size_of::<OuiDocumentConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            width,
            height,
            scale_factor: 1.0,
        }
    }

    fn create_document(width: u32, height: u32) -> *mut OuiDocument {
        let mut document = ptr::null_mut();
        assert_eq!(
            oui_document_create(&document_config(width, height), &mut document),
            OuiStatus::Ok
        );
        assert!(!document.is_null());
        document
    }

    fn create_element(
        document: *mut OuiDocument,
        tag: i32,
        parent: *mut OuiElement,
    ) -> *mut OuiElement {
        let mut element = ptr::null_mut();
        assert_eq!(
            oui_element_create(document, tag, &mut element),
            OuiStatus::Ok
        );
        if !parent.is_null() {
            assert_eq!(oui_element_append_child(parent, element), OuiStatus::Ok);
        }
        element
    }

    fn set_length(element: *mut OuiElement, property: i32, value: f32) -> OuiStatus {
        oui_element_set_property(
            element,
            property,
            &OuiStyleValue {
                tag: 1,
                reserved: 0,
                data: OuiStylePayload {
                    length: OuiLength { value, unit: 0 },
                },
            },
        )
    }

    fn event(event_type: u32, value: &str) -> OuiEvent {
        OuiEvent {
            struct_size: size_of::<OuiEvent>() as u32,
            abi_version: OUI_ABI_VERSION,
            event_type,
            phase: 0,
            flags: 0,
            modifiers: 0,
            timestamp_ns: 0,
            x: 0.0,
            y: 0.0,
            delta_x: 0.0,
            delta_y: 0.0,
            key_code: 0,
            pointer_id: 0,
            text: text(value),
            target: ptr::null_mut(),
            current_target: ptr::null_mut(),
        }
    }

    #[test]
    fn frozen_layout_metadata_matches_rust() {
        assert_eq!((size_of::<OuiUtf8>(), align_of::<OuiUtf8>()), (16, 8));
        assert_eq!((size_of::<OuiLength>(), align_of::<OuiLength>()), (8, 4));
        assert_eq!((size_of::<OuiColor>(), align_of::<OuiColor>()), (4, 1));
        assert_eq!(
            (size_of::<OuiStylePayload>(), align_of::<OuiStylePayload>()),
            (8, 8)
        );
        assert_eq!(
            (size_of::<OuiStyleValue>(), align_of::<OuiStyleValue>()),
            (16, 8)
        );
        assert_eq!(
            (size_of::<OuiAppConfig>(), align_of::<OuiAppConfig>()),
            (40, 8)
        );
        assert_eq!(
            (
                size_of::<OuiDocumentConfig>(),
                align_of::<OuiDocumentConfig>()
            ),
            (24, 8)
        );
        assert_eq!((size_of::<OuiRect>(), align_of::<OuiRect>()), (16, 4));
        assert_eq!((size_of::<OuiBitmap>(), align_of::<OuiBitmap>()), (32, 8));
        assert_eq!(
            (
                size_of::<OuiTransformOperation>(),
                align_of::<OuiTransformOperation>()
            ),
            (48, 4)
        );
        assert_eq!((size_of::<OuiEvent>(), align_of::<OuiEvent>()), (88, 8));
        assert_eq!(
            (size_of::<OuiErrorInfo>(), align_of::<OuiErrorInfo>()),
            (24, 8)
        );
    }

    #[test]
    fn malformed_inputs_report_structured_thread_local_errors() {
        assert_eq!(
            oui_document_create(ptr::null(), ptr::null_mut()),
            OuiStatus::InvalidArgument
        );
        let mut info = OuiErrorInfo {
            struct_size: size_of::<OuiErrorInfo>() as u32,
            abi_version: OUI_ABI_VERSION,
            status: 0,
            detail: 0,
            message_length: 0,
        };
        assert_eq!(oui_error_get_last(&mut info), OuiStatus::Ok);
        assert_eq!(info.status, OuiStatus::InvalidArgument as i32);
        let mut length = 0;
        assert_eq!(
            oui_error_copy_message(ptr::null_mut(), 0, &mut length),
            OuiStatus::Ok
        );
        let mut message = vec![0; length];
        assert_eq!(
            oui_error_copy_message(message.as_mut_ptr(), message.len(), &mut length),
            OuiStatus::Ok
        );
        assert!(std::str::from_utf8(&message).unwrap().contains("config"));

        let mut bad = document_config(10, 10);
        bad.abi_version = 1;
        let mut output = ptr::null_mut();
        assert_eq!(
            oui_document_create(&bad, &mut output),
            OuiStatus::AbiMismatch
        );
    }

    #[test]
    fn typed_tree_renders_and_buffers_have_explicit_lifetimes() {
        let document = create_document(64, 64);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let div = create_element(document, 0, root);
        assert_eq!(set_length(div, 4, 32.0), OuiStatus::Ok);
        assert_eq!(set_length(div, 5, 32.0), OuiStatus::Ok);
        assert_eq!(
            oui_element_set_property(
                div,
                20,
                &OuiStyleValue {
                    tag: 4,
                    reserved: 0,
                    data: OuiStylePayload {
                        color: OuiColor {
                            red: 255,
                            green: 0,
                            blue: 0,
                            alpha: 255,
                        },
                    },
                },
            ),
            OuiStatus::Ok
        );
        let mut bitmap = OuiBitmap {
            struct_size: size_of::<OuiBitmap>() as u32,
            abi_version: OUI_ABI_VERSION,
            width: 0,
            height: 0,
            stride: 0,
            pixels: ptr::null_mut(),
        };
        assert_eq!(
            oui_document_render_rgba(document, &mut bitmap),
            OuiStatus::Ok
        );
        assert_eq!((bitmap.width, bitmap.height, bitmap.stride), (64, 64, 256));
        assert_eq!(oui_buffer_length(bitmap.pixels), 64 * 64 * 4);
        assert!(!oui_buffer_data(bitmap.pixels).is_null());
        assert_eq!(oui_buffer_destroy(bitmap.pixels), OuiStatus::Ok);
        assert_eq!(oui_buffer_length(bitmap.pixels), 0);

        let mut png = ptr::null_mut();
        assert_eq!(oui_document_render_png(document, &mut png), OuiStatus::Ok);
        let length = oui_buffer_length(png);
        assert!(length > 8);
        // SAFETY: the buffer API promises `length` readable bytes while `png` is live.
        let png_bytes = unsafe { std::slice::from_raw_parts(oui_buffer_data(png), length) };
        assert_eq!(&png_bytes[..8], b"\x89PNG\r\n\x1a\n");
        assert_eq!(oui_buffer_destroy(png), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(div), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn rust_and_c_paths_produce_identical_headless_pixels() {
        let mut engine = Engine::new(Viewport::new(64, 64).unwrap()).unwrap();
        let direct = engine.create_element(ElementTag::Div).unwrap();
        engine.append_child(engine.root(), direct).unwrap();
        engine
            .set_property(
                direct,
                openui_style::StyleProperty::Width,
                openui_style::LengthValue::px(32.0).into(),
            )
            .unwrap();
        engine
            .set_property(
                direct,
                openui_style::StyleProperty::Height,
                openui_style::LengthValue::px(32.0).into(),
            )
            .unwrap();
        engine
            .set_property(
                direct,
                openui_style::StyleProperty::BackgroundColor,
                Color::RED.into(),
            )
            .unwrap();
        let expected = SoftwareCompositor::default()
            .render(&engine.scene().unwrap())
            .unwrap()
            .pixels;

        let document = create_document(64, 64);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let div = create_element(document, 0, root);
        assert_eq!(set_length(div, 4, 32.0), OuiStatus::Ok);
        assert_eq!(set_length(div, 5, 32.0), OuiStatus::Ok);
        let red = OuiStyleValue {
            tag: 4,
            reserved: 0,
            data: OuiStylePayload {
                color: OuiColor {
                    red: 255,
                    green: 0,
                    blue: 0,
                    alpha: 255,
                },
            },
        };
        assert_eq!(oui_element_set_property(div, 20, &red), OuiStatus::Ok);
        let mut bitmap = OuiBitmap {
            struct_size: size_of::<OuiBitmap>() as u32,
            abi_version: OUI_ABI_VERSION,
            width: 0,
            height: 0,
            stride: 0,
            pixels: ptr::null_mut(),
        };
        assert_eq!(
            oui_document_render_rgba(document, &mut bitmap),
            OuiStatus::Ok
        );
        let length = oui_buffer_length(bitmap.pixels);
        // SAFETY: the live buffer exposes exactly `length` immutable bytes.
        let actual = unsafe { std::slice::from_raw_parts(oui_buffer_data(bitmap.pixels), length) };
        assert_eq!(actual, expected);
        assert_eq!(oui_buffer_destroy(bitmap.pixels), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(div), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn app_and_document_handles_have_independent_explicit_lifetimes() {
        let title = "Open\0UI";
        let config = OuiAppConfig {
            struct_size: size_of::<OuiAppConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            title: text(title),
            width: 80,
            height: 40,
            backend: 2,
            reserved: 0,
        };
        let mut app = ptr::null_mut();
        assert_eq!(oui_app_create(&config, &mut app), OuiStatus::Ok);
        let mut document = ptr::null_mut();
        assert_eq!(oui_app_document(app, &mut document), OuiStatus::Ok);
        assert_eq!(oui_app_destroy(app), OuiStatus::Ok);
        assert_eq!(oui_document_update(document), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn ownership_generation_and_value_tags_are_validated() {
        let first = create_document(32, 32);
        let second = create_document(32, 32);
        let first_element = create_element(first, 0, ptr::null_mut());
        let second_element = create_element(second, 0, ptr::null_mut());
        assert_eq!(
            oui_element_append_child(first_element, second_element),
            OuiStatus::WrongDocument
        );
        let wrong = OuiStyleValue {
            tag: 2,
            reserved: 0,
            data: OuiStylePayload { number: 12.0 },
        };
        assert_eq!(
            oui_element_set_property(first_element, 4, &wrong),
            OuiStatus::WrongValueType
        );
        assert_eq!(oui_element_remove(first_element), OuiStatus::Ok);
        assert_eq!(set_length(first_element, 4, 1.0), OuiStatus::StaleHandle);
        assert_eq!(oui_element_destroy(first_element), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(second_element), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(first), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(second), OuiStatus::Ok);
    }

    #[test]
    fn transactions_and_compound_types_are_checked() {
        let document = create_document(32, 32);
        assert_eq!(oui_document_end_update(document), OuiStatus::InvalidState);
        assert_eq!(oui_document_begin_update(document), OuiStatus::Ok);
        assert_eq!(oui_document_update(document), OuiStatus::InvalidState);
        assert_eq!(oui_document_end_update(document), OuiStatus::Ok);

        let element = create_element(document, 0, ptr::null_mut());
        let edges = [OuiLength {
            value: 2.0,
            unit: 0,
        }; 4];
        let mut compound = ptr::null_mut();
        assert_eq!(
            oui_edges_create(edges.as_ptr(), &mut compound),
            OuiStatus::Ok
        );
        let compound_value = OuiStyleValue {
            tag: 6,
            reserved: 0,
            data: OuiStylePayload { compound },
        };
        assert_eq!(
            oui_element_set_property(element, 15, &compound_value),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_set_property(element, 37, &compound_value),
            OuiStatus::WrongValueType
        );
        assert_eq!(oui_style_compound_destroy(compound), OuiStatus::Ok);
        assert_eq!(
            oui_element_set_property(element, 15, &compound_value),
            OuiStatus::InvalidHandle
        );
        assert_eq!(oui_element_destroy(element), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    struct CallbackData {
        code: i32,
        log: *mut Vec<i32>,
        mutate: *mut OuiElement,
    }

    unsafe extern "C" fn record_event(event: *mut OuiEvent, user_data: *mut c_void) {
        // SAFETY: the test keeps both stack allocations alive through dispatch.
        let data = unsafe { &mut *(user_data as *mut CallbackData) };
        // SAFETY: dispatch supplies a live event for the callback duration.
        let event = unsafe { &mut *event };
        // SAFETY: `log` points to the live test vector and callbacks are synchronous.
        unsafe { &mut *data.log }.push(data.code * 10 + event.phase as i32);
        if !data.mutate.is_null() {
            assert_eq!(
                oui_element_set_text(data.mutate, text("updated")),
                OuiStatus::Ok
            );
        }
    }

    #[test]
    fn event_order_is_deterministic_and_callbacks_can_reenter() {
        let document = create_document(32, 32);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let target = create_element(document, 24, root);
        let mut log = Vec::new();
        let mut capture = CallbackData {
            code: 1,
            log: &mut log,
            mutate: ptr::null_mut(),
        };
        let mut at_target = CallbackData {
            code: 2,
            log: &mut log,
            mutate: target,
        };
        let mut bubble = CallbackData {
            code: 3,
            log: &mut log,
            mutate: ptr::null_mut(),
        };
        let mut first_listener = ptr::null_mut();
        let mut second_listener = ptr::null_mut();
        let mut third_listener = ptr::null_mut();
        assert_eq!(
            oui_element_add_event_listener(
                root,
                4,
                1,
                Some(record_event),
                &mut capture as *mut _ as *mut c_void,
                &mut first_listener,
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_add_event_listener(
                target,
                4,
                0,
                Some(record_event),
                &mut at_target as *mut _ as *mut c_void,
                &mut second_listener,
            ),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_add_event_listener(
                root,
                4,
                0,
                Some(record_event),
                &mut bubble as *mut _ as *mut c_void,
                &mut third_listener,
            ),
            OuiStatus::Ok
        );
        let mut event = OuiEvent {
            struct_size: size_of::<OuiEvent>() as u32,
            abi_version: OUI_ABI_VERSION,
            event_type: 4,
            phase: 0,
            flags: 0,
            modifiers: 0,
            timestamp_ns: 7,
            x: 1.0,
            y: 1.0,
            delta_x: 0.0,
            delta_y: 0.0,
            key_code: 0,
            pointer_id: 0,
            text: empty_utf8(),
            target: ptr::null_mut(),
            current_target: ptr::null_mut(),
        };
        assert_eq!(
            oui_document_dispatch_event(document, target, &mut event),
            OuiStatus::Ok
        );
        assert_eq!(log, [11, 22, 33]);
        assert_eq!(event.target, target);
        assert!(event.current_target.is_null());
        assert_eq!(oui_listener_destroy(first_listener), OuiStatus::Ok);
        assert_eq!(oui_listener_destroy(second_listener), OuiStatus::Ok);
        assert_eq!(oui_listener_destroy(third_listener), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(target), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn c_events_hit_testing_and_control_state_share_engine_defaults() {
        let document = create_document(100, 100);
        let mut root = ptr::null_mut();
        assert_eq!(oui_document_root(document, &mut root), OuiStatus::Ok);
        let checkbox = create_element(document, 23, root);
        assert_eq!(
            oui_element_set_property(
                checkbox,
                1,
                &OuiStyleValue {
                    tag: 5,
                    reserved: 0,
                    data: OuiStylePayload { enum_value: 2 },
                },
            ),
            OuiStatus::Ok
        );
        assert_eq!(set_length(checkbox, 4, 40.0), OuiStatus::Ok);
        assert_eq!(set_length(checkbox, 5, 40.0), OuiStatus::Ok);
        assert_eq!(
            oui_element_set_attribute(checkbox, text("type"), text("checkbox")),
            OuiStatus::Ok
        );
        let mut click = event(4, "");
        assert_eq!(
            oui_document_dispatch_event(document, checkbox, &mut click),
            OuiStatus::Ok
        );
        let mut flags = 0;
        assert_eq!(
            oui_element_get_control_flags(checkbox, &mut flags),
            OuiStatus::Ok
        );
        assert_ne!(flags & OUI_CONTROL_CHECKED, 0);

        assert_eq!(oui_element_set_checked(checkbox, 0), OuiStatus::Ok);
        let mut pointer = event(1, "");
        pointer.x = 10.0;
        pointer.y = 10.0;
        pointer.pointer_id = 7;
        assert_eq!(
            oui_document_dispatch_pointer_event(document, &mut pointer),
            OuiStatus::Ok
        );
        pointer.event_type = 2;
        assert_eq!(
            oui_document_dispatch_pointer_event(document, &mut pointer),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_element_get_control_flags(checkbox, &mut flags),
            OuiStatus::Ok
        );
        assert_ne!(flags & OUI_CONTROL_CHECKED, 0);

        let mut hit = ptr::null_mut();
        assert_eq!(
            oui_document_hit_test(document, 10.0, 10.0, &mut hit),
            OuiStatus::Ok
        );
        assert_eq!(hit, checkbox);

        let input = create_element(document, 23, root);
        assert_eq!(
            oui_element_set_control_value(input, text("a👩‍💻")),
            OuiStatus::Ok
        );
        assert_eq!(oui_element_set_selection(input, 1, 1), OuiStatus::Ok);
        let mut input_event = event(8, "b");
        assert_eq!(
            oui_document_dispatch_event(document, input, &mut input_event),
            OuiStatus::Ok
        );
        let mut value_length = 0;
        assert_eq!(
            oui_element_copy_control_value(input, ptr::null_mut(), 0, &mut value_length),
            OuiStatus::Ok
        );
        let mut value = vec![0; value_length];
        assert_eq!(
            oui_element_copy_control_value(
                input,
                value.as_mut_ptr(),
                value.len(),
                &mut value_length,
            ),
            OuiStatus::Ok
        );
        assert_eq!(std::str::from_utf8(&value).unwrap(), "ab👩‍💻");

        assert_eq!(oui_element_destroy(input), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(checkbox), OuiStatus::Ok);
        assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn handles_are_thread_affine_without_dereferencing_foreign_tokens() {
        let document = create_document(16, 16);
        let address = document as usize;
        let status = std::thread::spawn(move || oui_document_update(address as *mut OuiDocument))
            .join()
            .unwrap();
        assert_eq!(status, OuiStatus::WrongThread);
        assert_eq!(oui_document_destroy(document), OuiStatus::Ok);
    }

    #[test]
    fn panics_are_contained() {
        assert_eq!(
            registry::ffi(|| -> Result<(), ApiError> { panic!("contained") }),
            OuiStatus::Internal
        );
    }
}
