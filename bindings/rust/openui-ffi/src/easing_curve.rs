//! Caller-owned curve descriptors decoded into the shared native animation types.

use super::*;
use openui_style::PercentageLinearStop;
use std::mem::align_of;

// Read only the size/version header until its extent is accepted. The caller
// keeps the initialized descriptor and any referenced points immutable and
// readable for the call; no caller pointer is retained by an Engine operation.
unsafe fn read_input<T: Copy>(input: *const T, stride: Option<usize>) -> Result<T, ApiError> {
    if input.is_null() || (input as usize).checked_add(size_of::<T>()).is_none() {
        return Err(invalid("descriptor pointer or prefix address is invalid"));
    }
    let (declared, _) = unsafe { versioned_output_header(input.cast_mut()) }?;
    let declared = declared as usize;
    if stride.is_some_and(|stride| declared > stride)
        || (input as usize).checked_add(declared).is_none()
        || (input as usize) % align_of::<T>() != 0
    {
        return Err(invalid("descriptor extent or alignment is invalid"));
    }
    // SAFETY: the header accepted the complete known prefix; alignment and
    // address overflow were checked. The C contract supplies readable storage.
    Ok(unsafe { ptr::read(input) })
}

fn array_extent<T>(base: *const T, count: usize, stride: usize) -> Result<(), ApiError> {
    if count == 0
        || base.is_null()
        || stride < size_of::<T>()
        || stride % align_of::<T>() != 0
        || (base as usize) % align_of::<T>() != 0
    {
        return Err(invalid(
            "array pointer, count, stride or alignment is invalid",
        ));
    }
    let extent = count
        .checked_mul(stride)
        .filter(|extent| *extent <= isize::MAX as usize)
        .ok_or_else(|| invalid("array extent overflows"))?;
    (base as usize)
        .checked_add(extent)
        .ok_or_else(|| invalid("array address overflows"))?;
    Ok(())
}

fn reserve<T>(count: usize, name: &str) -> Result<Vec<T>, ApiError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| ApiError::new(OuiStatus::OutOfMemory, format!("{name} allocation failed")))?;
    Ok(values)
}

fn stops<T>(
    curve: &OuiEasingCurveV1,
    convert: impl Fn(OuiLinearStopV1) -> T,
) -> Result<Vec<T>, ApiError> {
    if curve.stop_count < 2 {
        return Err(invalid("linear easing requires at least two stops"));
    }
    array_extent(curve.stops, curve.stop_count, curve.stop_stride)?;
    let mut values = reserve(curve.stop_count, "linear-stop")?;
    for index in 0..curve.stop_count {
        // SAFETY: array extent/alignment is checked above; callers supply each
        // readable initialized point. Stride tails are ignored, never retained.
        let point = unsafe {
            ptr::read(
                curve
                    .stops
                    .cast::<u8>()
                    .add(index * curve.stop_stride)
                    .cast(),
            )
        };
        values.push(convert(point));
    }
    Ok(values)
}

pub(super) fn curve(input: *const OuiEasingCurveV1) -> Result<Easing, ApiError> {
    if input.is_null() {
        return Ok(Easing::Linear);
    }
    // SAFETY: exported calls require the readable initialized header/prefix.
    let raw = unsafe { read_input(input, None) }?;
    if raw.reserved != 0 || raw.values.iter().any(|value| !value.is_finite()) {
        return Err(invalid("curve contains invalid or reserved values"));
    }
    let has_array = !raw.stops.is_null() || raw.stop_count != 0 || raw.stop_stride != 0;
    match raw.kind {
        0 if raw.step_position == 0
            && raw.step_count == 0
            && raw.values == [0.0; 4]
            && !has_array =>
        {
            Ok(Easing::Linear)
        }
        1 if raw.step_position == 0 && raw.step_count == 0 && !has_array => {
            Easing::cubic_bezier(raw.values[0], raw.values[1], raw.values[2], raw.values[3])
                .map_err(|error| invalid(error.to_string()))
        }
        2 if raw.values == [0.0; 4] && !has_array => {
            let position = match raw.step_position {
                0 => StepPosition::JumpStart,
                1 => StepPosition::JumpEnd,
                2 => StepPosition::JumpNone,
                3 => StepPosition::JumpBoth,
                _ => return Err(invalid("unknown step position")),
            };
            Easing::steps(raw.step_count, position).map_err(|error| invalid(error.to_string()))
        }
        3 | 4 if raw.step_position == 0 && raw.step_count == 0 && raw.values == [0.0; 4] => {
            if raw.kind == 3 {
                Easing::linear_stops(stops(&raw, |point| LinearStop {
                    input: point.input,
                    output: point.output,
                })?)
            } else {
                // Preserve authored percentage bytes. Conversion to normalized
                // units belongs to the shared gfx-compatible sampler only.
                Easing::linear_stops_percent(stops(&raw, |point| PercentageLinearStop {
                    input_percent: point.input,
                    output: point.output,
                })?)
            }
            .map_err(|error| invalid(error.to_string()))
        }
        _ => Err(invalid("unknown curve kind or nonzero unused curve fields")),
    }
}

fn options(input: *const OuiAnimationOptionsV1) -> Result<AnimationOptions, ApiError> {
    // SAFETY: the readable size/version header is checked before the prefix.
    let raw = unsafe { read_input(input, None) }?;
    animation_options_values(
        AnimationTimingInput {
            delay_ms: raw.delay_ms,
            duration_ms: raw.duration_ms,
            iterations: raw.iterations,
            playback_rate: raw.playback_rate,
            direction: raw.direction,
            fill: raw.fill,
            play_state: raw.play_state,
            composite: raw.composite,
        },
        curve(raw.easing)?,
    )
}

fn keyframes(
    property: StyleProperty,
    input: *const OuiKeyframeV1,
    count: usize,
    stride: usize,
) -> Result<PropertyKeyframes, ApiError> {
    array_extent(input, count, stride)?;
    let mut frames = reserve(count, "keyframe")?;
    for index in 0..count {
        // SAFETY: checked array arithmetic bounds the known prefix and stride;
        // each entry's own header is accepted before copying its payload.
        let raw = unsafe {
            read_input(
                input
                    .cast::<u8>()
                    .add(index * stride)
                    .cast::<OuiKeyframeV1>(),
                Some(stride),
            )
        }?;
        if !raw.offset.is_finite() {
            return Err(invalid("keyframe offset must be finite"));
        }
        frames.push(Keyframe {
            offset: raw.offset,
            value: value::style_value(property, &raw.value)?,
            easing: if raw.easing.is_null() {
                None
            } else {
                Some(curve(raw.easing)?)
            },
        });
    }
    let frames = Keyframes::new(frames).map_err(|error| invalid(error.to_string()))?;
    PropertyKeyframes::typed(property, frames).map_err(|error| invalid(error.to_string()))
}

fn id_output(output: *mut u64) -> Result<(), ApiError> {
    if output.is_null()
        || (output as usize) % align_of::<u64>() != 0
        || (output as usize).checked_add(size_of::<u64>()).is_none()
    {
        return Err(invalid("animation ID output is null or misaligned"));
    }
    Ok(())
}

// SAFETY CONTRACT: live owning-thread element; initialized readable versioned
// descriptors/strided arrays and writable, aligned, nonoverlapping ID storage.
// All inputs are copied before Engine mutation; callbacks run without borrows.
#[no_mangle]
pub extern "C" fn oui_element_animate_v1(
    element_handle: *mut OuiElement,
    property: i32,
    frames: *const OuiKeyframeV1,
    frame_count: usize,
    frame_stride: usize,
    timing: *const OuiAnimationOptionsV1,
    timeline: *const OuiAnimationTimeline,
    output: *mut u64,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        id_output(output)?;
        let property =
            property_from_raw(property).ok_or_else(|| invalid("unknown style property"))?;
        let frames = keyframes(property, frames, frame_count, frame_stride)?;
        let options = options(timing)?;
        let timeline = animation_timeline(&state, timeline)?;
        let id = borrow_engine_mut(&state)?.animate(element.node, frames, options, timeline)?;
        flush_animation_events(&state)?;
        // SAFETY: caller supplies checked writable aligned ID storage.
        unsafe { ptr::write(output, id.0) };
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread element, readable target/descriptors,
// writable aligned ID. Inputs are owned before the shared transition is called.
#[no_mangle]
pub extern "C" fn oui_element_transition_v1(
    element_handle: *mut OuiElement,
    property: i32,
    target: *const OuiStyleValue,
    timing: *const OuiAnimationOptionsV1,
    output: *mut u64,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        id_output(output)?;
        if target.is_null()
            || (target as usize) % align_of::<OuiStyleValue>() != 0
            || (target as usize)
                .checked_add(size_of::<OuiStyleValue>())
                .is_none()
        {
            return Err(invalid("transition target is null or misaligned"));
        }
        let property =
            property_from_raw(property).ok_or_else(|| invalid("unknown style property"))?;
        // SAFETY: readable aligned tagged target storage is required by C.
        let target = value::style_value(property, unsafe { &*target })?;
        let options = options(timing)?;
        let id = borrow_engine_mut(&state)?.transition(element.node, property, target, options)?;
        flush_animation_events(&state)?;
        // SAFETY: checked writable aligned output, after callbacks/borrows end.
        unsafe { ptr::write(output, id.0) };
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread element, readable optional curve and
// writable aligned ID. No caller point or descriptor pointer is retained.
#[no_mangle]
pub extern "C" fn oui_element_smooth_scroll_to_v1(
    element_handle: *mut OuiElement,
    x: f64,
    y: f64,
    duration_ms: f64,
    easing: *const OuiEasingCurveV1,
    output: *mut u64,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        id_output(output)?;
        let easing = curve(easing)?;
        let id =
            borrow_engine_mut(&state)?.smooth_scroll_to(element.node, x, y, duration_ms, easing)?;
        // SAFETY: checked writable aligned ID; no Engine borrow is retained.
        unsafe { ptr::write(output, id.0) };
        Ok(())
    })
}

// SAFETY CONTRACT: live owning-thread element and readable optional curve.
// Uses the same reveal plan and clock as native Rust; caller inputs are copied.
#[no_mangle]
pub extern "C" fn oui_element_smooth_scroll_into_view_v2(
    element_handle: *mut OuiElement,
    block: u32,
    inline: u32,
    container: u32,
    duration_ms: f64,
    easing: *const OuiEasingCurveV1,
) -> OuiStatus {
    ffi(|| {
        let element = element(element_handle as usize)?;
        let state = element_document(&element)?;
        let options = scroll_into_view_options(block, inline, container)?;
        let easing = curve(easing)?;
        borrow_engine_mut(&state)?.smooth_scroll_into_view_with_easing(
            element.node,
            options,
            duration_ms,
            easing,
        )?;
        Ok(())
    })
}
