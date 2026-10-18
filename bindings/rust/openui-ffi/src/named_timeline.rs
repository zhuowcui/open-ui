//! Versioned owned transport into the shared native named-timeline operations.
use super::*;
use openui_engine::AnimationTimelineSource;
use openui_style::{
    AnimationAttachmentRange, TimelineNamedRange, TimelineRangeOffset, ViewTimelineInset,
};
use std::mem::align_of;

fn native_float(value: f64) -> Result<f32, ApiError> {
    let native = value as f32;
    if !value.is_finite() || !native.is_finite() {
        return Err(invalid(
            "timeline length must fit the finite native float representation",
        ));
    }
    Ok(native)
}
fn zero_length(value: OuiTimelineLengthV1) -> bool {
    value.value == 0.0 && value.calc_offset == 0.0 && value.unit == 0 && value.reserved == 0
}
fn zero_offset(value: OuiTimelineRangeOffsetV1) -> bool {
    value.name == 0 && value.reserved == 0 && zero_length(value.offset)
}
fn length(value: OuiTimelineLengthV1) -> Result<LengthValue, ApiError> {
    if value.reserved != 0 || (value.unit != 13 && value.calc_offset != 0.0) {
        return Err(invalid("nonzero reserved or unused timeline length fields"));
    }
    let number = native_float(value.value)?;
    Ok(match value.unit {
        0 => LengthValue::px(number),
        1 => LengthValue::percent(number),
        2 => LengthValue::Em(number),
        3 => LengthValue::Rem(number),
        4 => LengthValue::ViewportWidth(number),
        5 => LengthValue::ViewportHeight(number),
        8 => LengthValue::ViewportMin(number),
        9 => LengthValue::ViewportMax(number),
        10 => LengthValue::Ch(number),
        11 => LengthValue::Ex(number),
        12 => LengthValue::Lh(number),
        13 => LengthValue::calc_percent_px(number, native_float(value.calc_offset)?),
        _ => return Err(invalid("invalid timeline length unit")),
    })
}
fn name(value: u32) -> Result<TimelineNamedRange, ApiError> {
    Ok(match value {
        0 => TimelineNamedRange::Cover,
        1 => TimelineNamedRange::Contain,
        2 => TimelineNamedRange::Entry,
        3 => TimelineNamedRange::EntryCrossing,
        4 => TimelineNamedRange::Exit,
        5 => TimelineNamedRange::ExitCrossing,
        6 => TimelineNamedRange::Scroll,
        _ => return Err(invalid("unknown named timeline range")),
    })
}
fn offset(value: OuiTimelineRangeOffsetV1) -> Result<TimelineRangeOffset, ApiError> {
    if value.reserved != 0 {
        return Err(invalid("range offset reserved field is nonzero"));
    }
    TimelineRangeOffset::new(name(value.name)?, length(value.offset)?).map_err(animation_error)
}
fn range(
    input: *const OuiAnimationAttachmentRangeV1,
) -> Result<AnimationAttachmentRange, ApiError> {
    if input.is_null() {
        return Ok(AnimationAttachmentRange::default());
    }
    // SAFETY: initialized header and then readable known-prefix storage are required.
    let raw = unsafe { easing_curve::read_input(input, None) }?;
    if raw.reserved != 0
        || raw.present & !3 != 0
        || (raw.present & 1 == 0 && !zero_offset(raw.start))
        || (raw.present & 2 == 0 && !zero_offset(raw.end))
    {
        return Err(invalid("range presence or unused fields are invalid"));
    }
    let range = AnimationAttachmentRange {
        start: if raw.present & 1 != 0 {
            Some(offset(raw.start)?)
        } else {
            None
        },
        end: if raw.present & 2 != 0 {
            Some(offset(raw.end)?)
        } else {
            None
        },
    };
    range.validate().map_err(animation_error)?;
    Ok(range)
}
fn source(
    owner: &Rc<DocumentState>,
    input: *const OuiAnimationTimelineSourceV1,
) -> Result<AnimationTimelineSource, ApiError> {
    if input.is_null() {
        return Ok(AnimationTimelineSource::Document);
    }
    // SAFETY: size/version is accepted before the complete source descriptor is copied.
    let raw = unsafe { easing_curve::read_input(input, None) }?;
    if raw.reserved != 0
        || raw.inset_present & !3 != 0
        || (raw.inset_present & 1 == 0 && !zero_length(raw.inset_start))
        || (raw.inset_present & 2 == 0 && !zero_length(raw.inset_end))
    {
        return Err(invalid("source presence or unused fields are invalid"));
    }
    if raw.kind == 0 {
        if raw.axis != 0 || !raw.source.is_null() || raw.inset_present != 0 {
            return Err(invalid("document source must have zero unused fields"));
        }
        return Ok(AnimationTimelineSource::Document);
    }
    if !matches!(raw.kind, 1 | 2) || (raw.kind == 1 && raw.inset_present != 0) {
        return Err(invalid(
            "unknown source kind or scroll source with view insets",
        ));
    }
    let axis = match raw.axis {
        0 => TimelineAxis::Block,
        1 => TimelineAxis::Inline,
        2 => TimelineAxis::X,
        3 => TimelineAxis::Y,
        _ => return Err(invalid("unknown timeline axis")),
    };
    let handle = element(raw.source as usize)?;
    let state = element_document(&handle)?;
    if !Rc::ptr_eq(owner, &state) {
        return Err(ApiError::new(
            OuiStatus::WrongDocument,
            "timeline source belongs to another document",
        ));
    }
    // Resolve the native generation before allocating frames or mutating animation state.
    borrow_engine(owner)?.parent(handle.node)?;
    if raw.kind == 1 {
        return Ok(AnimationTimelineSource::Scroll {
            source: handle.node,
            axis,
        });
    }
    if raw.inset_present == 0 {
        return Ok(AnimationTimelineSource::View {
            subject: handle.node,
            axis,
        });
    }
    let inset = ViewTimelineInset::new(
        if raw.inset_present & 1 != 0 {
            Some(length(raw.inset_start)?)
        } else {
            None
        },
        if raw.inset_present & 2 != 0 {
            Some(length(raw.inset_end)?)
        } else {
            None
        },
    )
    .map_err(animation_error)?;
    Ok(AnimationTimelineSource::ViewWithInset {
        subject: handle.node,
        axis,
        inset,
    })
}
fn output_header<T>(output: *mut T) -> Result<(u32, u32), ApiError> {
    if output.is_null() || (output as usize).checked_add(size_of::<T>()).is_none() {
        return Err(invalid("snapshot output pointer or extent is invalid"));
    }
    // SAFETY: only initialized size/version words are read before acceptance.
    let header = unsafe { versioned_output_header(output) }?;
    if (output as usize) % align_of::<T>() != 0
        || (output as usize).checked_add(header.0 as usize).is_none()
    {
        return Err(invalid(
            "snapshot output alignment or declared extent is invalid",
        ));
    }
    Ok(header)
}
fn after_snapshot(
    handle: *mut OuiDocument,
    state: &Rc<DocumentState>,
    id: AnimationId,
) -> Result<(), ApiError> {
    flush_animation_events(state)?;
    // Callbacks can release the document or cancel the animation. No output or
    // alias is transferred until both identities are revalidated without borrows.
    if !Rc::ptr_eq(state, &document(handle as usize)?) {
        return Err(invalid("snapshot document identity changed"));
    }
    borrow_engine(state)?.animation_state(id)?;
    Ok(())
}
fn owned_source(state: &Rc<DocumentState>, node: NodeHandle) -> Result<*mut OuiElement, ApiError> {
    borrow_engine(state)?.parent(node)?;
    focus_events::owned_element_handle(state, node)
}
fn encode_length(value: LengthValue) -> OuiTimelineLengthV1 {
    let (value, unit, calc_offset) = match value {
        LengthValue::Computed(v) if v.is_percent() => (v.value(), 1, 0.0),
        LengthValue::Computed(v) if v.is_calculated() => (v.value(), 13, v.calc_offset()),
        LengthValue::Computed(v) => (v.value(), 0, 0.0),
        LengthValue::Em(v) => (v, 2, 0.0),
        LengthValue::Rem(v) => (v, 3, 0.0),
        LengthValue::ViewportWidth(v) => (v, 4, 0.0),
        LengthValue::ViewportHeight(v) => (v, 5, 0.0),
        LengthValue::ViewportMin(v) => (v, 8, 0.0),
        LengthValue::ViewportMax(v) => (v, 9, 0.0),
        LengthValue::Ch(v) => (v, 10, 0.0),
        LengthValue::Ex(v) => (v, 11, 0.0),
        LengthValue::Lh(v) => (v, 12, 0.0),
    };
    OuiTimelineLengthV1 {
        value: value as f64,
        calc_offset: calc_offset as f64,
        unit,
        reserved: 0,
    }
}
fn encode_name(value: TimelineNamedRange) -> u32 {
    match value {
        TimelineNamedRange::Cover => 0,
        TimelineNamedRange::Contain => 1,
        TimelineNamedRange::Entry => 2,
        TimelineNamedRange::EntryCrossing => 3,
        TimelineNamedRange::Exit => 4,
        TimelineNamedRange::ExitCrossing => 5,
        TimelineNamedRange::Scroll => 6,
    }
}
fn encode_range(value: AnimationAttachmentRange) -> OuiAnimationAttachmentRangeV1 {
    let empty = OuiTimelineRangeOffsetV1 {
        name: 0,
        reserved: 0,
        offset: OuiTimelineLengthV1 {
            value: 0.0,
            calc_offset: 0.0,
            unit: 0,
            reserved: 0,
        },
    };
    let encode = |v: TimelineRangeOffset| OuiTimelineRangeOffsetV1 {
        name: encode_name(v.name),
        reserved: 0,
        offset: encode_length(v.offset),
    };
    OuiAnimationAttachmentRangeV1 {
        struct_size: size_of::<OuiAnimationAttachmentRangeV1>() as u32,
        abi_version: OUI_ABI_VERSION,
        present: u32::from(value.start.is_some()) | (u32::from(value.end.is_some()) << 1),
        reserved: 0,
        start: value.start.map_or(empty, encode),
        end: value.end.map_or(empty, encode),
    }
}

// SAFETY CONTRACT: owning-thread live element, initialized readable versioned
// inputs/strided keyframes, writable aligned u64 output; caller pointers are
// immutable/nonoverlapping for the call. The shared decoder copies all inputs.
#[no_mangle]
pub extern "C" fn oui_element_animate_with_attachment_range_v1(
    element_handle: *mut OuiElement,
    property: i32,
    frames: *const OuiKeyframeV1,
    frame_count: usize,
    frame_stride: usize,
    timing: *const OuiAnimationOptionsV1,
    source_input: *const OuiAnimationTimelineSourceV1,
    range_input: *const OuiAnimationAttachmentRangeV1,
    output: *mut u64,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        borrow_engine(&state)?.parent(element.node)?;
        easing_curve::id_output(output)?;
        let property =
            property_from_raw(property).ok_or_else(|| invalid("unknown animation property"))?;
        let source = source(&state, source_input)?;
        let range = range(range_input)?;
        let options = easing_curve::options(timing)?;
        let frames = easing_curve::keyframes(property, frames, frame_count, frame_stride)?;
        let id = borrow_engine_mut(&state)?.animate_with_attachment_range(
            element.node,
            frames,
            options,
            source,
            range,
        )?;
        flush_animation_events(&state)?;
        // SAFETY: checked writable ID, after all Engine/registry borrows end.
        unsafe { ptr::write(output, id.0) };
        Ok(())
    })
}
// SAFETY CONTRACT: owning-thread live document and readable optional source
// descriptor. Copy/validate it before action; callbacks run after Engine borrows.
#[no_mangle]
pub extern "C" fn oui_document_set_animation_timeline_source_v1(
    handle: *mut OuiDocument,
    id: u64,
    input: *const OuiAnimationTimelineSourceV1,
) -> OuiStatus {
    ffi(|| {
        let state = document(handle as usize)?;
        let source = source(&state, input)?;
        borrow_engine_mut(&state)?.set_animation_timeline_source(AnimationId(id), source)?;
        flush_animation_events(&state)
    })
}

// SAFETY CONTRACT: owning-thread live document and readable optional descriptor.
#[no_mangle]
pub extern "C" fn oui_document_set_animation_attachment_range_v1(
    handle: *mut OuiDocument,
    id: u64,
    input: *const OuiAnimationAttachmentRangeV1,
) -> OuiStatus {
    ffi(|| {
        let state = document(handle as usize)?;
        let range = range(input)?;
        borrow_engine_mut(&state)?.set_animation_attachment_range(AnimationId(id), range)?;
        flush_animation_events(&state)
    })
}
// SAFETY CONTRACT: owning-thread live document. Uses the same private timeline
// control as Rust, including inactive pending percentages and later activation.
#[no_mangle]
pub extern "C" fn oui_document_seek_animation_progress_v1(
    handle: *mut OuiDocument,
    id: u64,
    percentage: f64,
) -> OuiStatus {
    ffi(|| {
        let state = document(handle as usize)?;
        if !percentage.is_finite() {
            return Err(invalid("animation percentage must be finite"));
        }
        borrow_engine_mut(&state)?.seek_animation_progress(AnimationId(id), percentage)?;
        flush_animation_events(&state)
    })
}
// SAFETY CONTRACT: initialized size/version prefix and writable aligned known
// output prefix; future tails untouched. Returned source is an owned C alias.
#[no_mangle]
pub extern "C" fn oui_document_get_animation_timeline_snapshot_v1(
    handle: *mut OuiDocument,
    id: u64,
    output: *mut OuiAnimationTimelineSnapshotV1,
) -> OuiStatus {
    ffi(|| {
        let state = document(handle as usize)?;
        let (struct_size, abi_version) = output_header(output)?;
        let snapshot = { borrow_engine_mut(&state)?.animation_timeline_snapshot(AnimationId(id))? };
        after_snapshot(handle, &state, AnimationId(id))?;
        let mut value = OuiAnimationTimelineSnapshotV1 {
            struct_size,
            abi_version,
            present: 0,
            active: 0,
            physical_axis: 0,
            reserved: 0,
            source: ptr::null_mut(),
            scroll_offset: 0.0,
            cover_start: 0.0,
            cover_end: 0.0,
            range_start: 0.0,
            range_end: 0.0,
            duration_ms: 0.0,
            current_time_ms: 0.0,
            current_time_percentage: 0.0,
        };
        if let Some(snapshot) = snapshot {
            value.present = 1;
            value.active = u32::from(snapshot.active);
            value.range_start = snapshot.range_start;
            value.range_end = snapshot.range_end;
            if let Some(axis) = snapshot.physical_axis {
                value.present |= 1 << 2;
                value.physical_axis = match axis {
                    TimelineAxis::Block => 0,
                    TimelineAxis::Inline => 1,
                    TimelineAxis::X => 2,
                    TimelineAxis::Y => 3,
                };
            }
            for (bit, field, scalar) in [
                (3, &mut value.scroll_offset, snapshot.scroll_offset),
                (4, &mut value.cover_start, snapshot.cover_start),
                (5, &mut value.cover_end, snapshot.cover_end),
                (6, &mut value.duration_ms, snapshot.duration_ms),
                (7, &mut value.current_time_ms, snapshot.current_time_ms),
                (
                    8,
                    &mut value.current_time_percentage,
                    snapshot.current_time_percentage,
                ),
            ] {
                if let Some(scalar) = scalar {
                    *field = scalar;
                    value.present |= 1 << bit;
                }
            }
            if let Some(source) = snapshot.source {
                value.source = owned_source(&state, source)?;
                value.present |= 1 << 1;
            }
        }
        // All fallible validation/callback/ownership work is complete. The alias
        // transfers here; no later fallible operation can orphan it.
        unsafe { ptr::write(output, value) };
        Ok(())
    })
}
// SAFETY CONTRACT: same prefix/output ownership rules as timeline snapshot.
#[no_mangle]
pub extern "C" fn oui_document_get_animation_attachment_snapshot_v1(
    handle: *mut OuiDocument,
    id: u64,
    output: *mut OuiAnimationAttachmentSnapshotV1,
) -> OuiStatus {
    ffi(|| {
        let state = document(handle as usize)?;
        let (struct_size, abi_version) = output_header(output)?;
        let snapshot =
            { borrow_engine_mut(&state)?.animation_attachment_snapshot(AnimationId(id))? };
        after_snapshot(handle, &state, AnimationId(id))?;
        let empty = OuiTimelineRangeBoundsV1 {
            start: 0.0,
            end: 0.0,
        };
        let mut value = OuiAnimationAttachmentSnapshotV1 {
            struct_size,
            abi_version,
            present: 0,
            reserved: 0,
            range: encode_range(AnimationAttachmentRange::default()),
            source: ptr::null_mut(),
            named: [empty; 7],
            attachment: empty,
            entry_crossing_distance: 0.0,
            exit_crossing_distance: 0.0,
        };
        if let Some(snapshot) = snapshot {
            value.present = 1;
            value.range = encode_range(snapshot.range);
            if let Some(resolved) = snapshot.resolved {
                value.present |= 2;
                value.named = [
                    resolved.named.cover,
                    resolved.named.contain,
                    resolved.named.entry,
                    resolved.named.entry_crossing,
                    resolved.named.exit,
                    resolved.named.exit_crossing,
                    resolved.named.scroll,
                ]
                .map(|r| OuiTimelineRangeBoundsV1 {
                    start: r.start,
                    end: r.end,
                });
                value.attachment = OuiTimelineRangeBoundsV1 {
                    start: resolved.attachment.start,
                    end: resolved.attachment.end,
                };
                value.entry_crossing_distance = resolved.entry_crossing_distance;
                value.exit_crossing_distance = resolved.exit_crossing_distance;
                value.source = owned_source(&state, resolved.source)?;
            }
        }
        unsafe { ptr::write(output, value) };
        Ok(())
    })
}
