//! Owned explicit snap points and cancellation for shared native scroll tracks.

use super::*;
use openui_engine::ScrollAnimationId;
use std::mem::align_of;

fn owned_points(input: *const f64, count: usize) -> Result<Vec<f64>, ApiError> {
    if count == 0 {
        return Ok(Vec::new());
    }
    let extent = count
        .checked_mul(size_of::<f64>())
        .filter(|extent| *extent <= isize::MAX as usize)
        .ok_or_else(|| invalid("snap-point extent overflows"))?;
    if input.is_null()
        || (input as usize) % align_of::<f64>() != 0
        || (input as usize).checked_add(extent).is_none()
    {
        return Err(invalid("snap-point pointer or alignment is invalid"));
    }
    let mut points = Vec::new();
    points
        .try_reserve_exact(count)
        .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, "snap-point allocation failed"))?;
    for index in 0..count {
        // SAFETY: extent/alignment is accepted above. The C caller supplies
        // initialized readable elements; no reference or pointer is retained.
        let value = unsafe { ptr::read(input.add(index)) };
        if !value.is_finite() {
            return Err(invalid("snap points must be finite"));
        }
        points.push(value);
    }
    Ok(points)
}

fn outputs(id: *mut u64, present: *mut u8) -> Result<(), ApiError> {
    easing_curve::id_output(id)?;
    let id_start = id as usize;
    let id_end = id_start + size_of::<u64>();
    let present_start = present as usize;
    let present_end = present_start
        .checked_add(size_of::<u8>())
        .filter(|_| !present.is_null())
        .ok_or_else(|| invalid("snap animation presence output is invalid"))?;
    if id_start < present_end && present_start < id_end {
        return Err(invalid("snap animation outputs overlap"));
    }
    Ok(())
}

// SAFETY CONTRACT: live owning-thread document and a scroll ID created by that
// document's scroll-animation API. IDs are document-scoped numeric values, not
// globally transferable handles. Completed/unknown IDs use the shared error.
#[no_mangle]
pub extern "C" fn oui_document_cancel_smooth_scroll_v1(
    document_handle: *mut OuiDocument,
    scroll_animation_id: u64,
) -> OuiStatus {
    ffi(|| {
        let state = document(document_handle as usize)?;
        borrow_engine_mut(&state)?.cancel_smooth_scroll(ScrollAnimationId(scroll_animation_id))?;
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread element; arrays are immutable readable
// contiguous initialized doubles for their counts (NULL is allowed at count0).
// Curve storage follows OuiEasingCurveV1's readable-prefix contract. Writable
// scalar outputs are distinct and ID-aligned; they may alias original inputs
// because every input is copied before any output write or Engine action.
#[no_mangle]
pub extern "C" fn oui_element_settle_scroll_snap_with_easing_v1(
    element_handle: *mut OuiElement,
    points_x: *const f64,
    points_x_count: usize,
    points_y: *const f64,
    points_y_count: usize,
    duration_ms: f64,
    easing: *const OuiEasingCurveV1,
    out_scroll_animation_id: *mut u64,
    out_has_animation: *mut u8,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        outputs(out_scroll_animation_id, out_has_animation)?;
        if !duration_ms.is_finite() || duration_ms < 0.0 {
            return Err(invalid("snap duration must be finite and non-negative"));
        }
        let points_x = owned_points(points_x, points_x_count)?;
        let points_y = owned_points(points_y, points_y_count)?;
        let easing = easing_curve::curve(easing)?;
        let animation = borrow_engine_mut(&state)?.settle_scroll_snap(
            element.node,
            &points_x,
            &points_y,
            duration_ms,
            easing,
        )?;
        // SAFETY: outputs were checked before mutation; no fallible operation
        // or Engine borrow remains. Absent animation has deterministic ID0.
        unsafe {
            ptr::write(out_scroll_animation_id, animation.map_or(0, |id| id.0));
            ptr::write(out_has_animation, u8::from(animation.is_some()));
        }
        Ok(())
    })
}
