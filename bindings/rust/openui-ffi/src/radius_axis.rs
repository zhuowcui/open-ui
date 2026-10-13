//! Append-only native radius-axis lengths through the shared Engine/source masks.
use super::*;
use std::mem::align_of;

fn aligned_prefix<T>(address: usize) -> Result<(), ApiError> {
    if address == 0
        || address % align_of::<T>() != 0
        || address.checked_add(size_of::<T>()).is_none()
    {
        return Err(invalid(
            "radius axis descriptor/output alignment or address is invalid",
        ));
    }
    Ok(())
}
fn axis_property(raw: i32) -> Result<StyleProperty, ApiError> {
    let property =
        property_from_raw(raw).ok_or_else(|| invalid("unknown style property identifier"))?;
    if property.radius_axis_coordinates().is_none() {
        return Err(ApiError::new(
            OuiStatus::WrongValueType,
            "property is not a native radius axis",
        )
        .detail(property as u32));
    }
    Ok(property)
}
fn decode(input: OuiRadiusAxisLengthV1) -> Result<openui_style::LengthValue, ApiError> {
    if input.reserved != 0 {
        return Err(invalid("radius axis reserved field must be zero"));
    }
    let value = if input.unit == 13 {
        openui_style::LengthValue::calc_percent_px(input.value, input.calc_offset)
    } else {
        if input.calc_offset != 0.0 {
            return Err(invalid(
                "non-calculated radius axis must have zero calc_offset",
            ));
        }
        value::length(OuiLength {
            value: input.value,
            unit: input.unit,
        })?
    };
    if !value.is_valid_for_radius_axis() {
        return Err(invalid("radius axis length is invalid or nonfinite"));
    }
    Ok(value)
}
fn encode(
    value: openui_style::LengthValue,
    size: u32,
    abi: u32,
) -> Result<OuiRadiusAxisLengthV1, ApiError> {
    let openui_style::LengthValue::Computed(length) = value else {
        return Err(ApiError::new(
            OuiStatus::InvalidState,
            "computed radius axis is unresolved",
        ));
    };
    let unit = if length.is_fixed() {
        0
    } else if length.is_percent() {
        1
    } else if length.is_calculated() {
        13
    } else {
        return Err(ApiError::new(
            OuiStatus::InvalidState,
            "legacy radius source is not a finite computed length",
        ));
    };
    if !length.value().is_finite() || !length.calc_offset().is_finite() {
        return Err(ApiError::new(
            OuiStatus::InvalidState,
            "computed radius axis is nonfinite",
        ));
    }
    Ok(OuiRadiusAxisLengthV1 {
        struct_size: size,
        abi_version: abi,
        value: length.value(),
        calc_offset: if unit == 13 {
            length.calc_offset()
        } else {
            0.0
        },
        unit,
        reserved: 0,
    })
}

// SAFETY CONTRACT: initialized readable size word (and ABI word if size>=8).
// After prefix acceptance, aligned readable initialized24-byte descriptor;
// aligned writable OuiStyleValue output. No caller pointer is retained.
#[no_mangle]
pub extern "C" fn oui_style_value_radius_axis_length_v1(
    property: i32,
    input: *const OuiRadiusAxisLengthV1,
    out_value: *mut OuiStyleValue,
) -> OuiStatus {
    ffi(|| {
        aligned_prefix::<OuiStyleValue>(out_value as usize)?;
        let property = axis_property(property)?;
        aligned_prefix::<OuiRadiusAxisLengthV1>(input as usize)?;
        // Header reads precede the full known-prefix copy, including when the
        // actual allocation contains only a rejected size or ABI header.
        let input = unsafe { versioned_input(input) }?;
        let value = StyleValue::Length(decode(input)?);
        let token = register(LocalHandle::PropertyCompound(property, value))?;
        // SAFETY: only the complete known output changes after all validation
        // and registration succeeds. The compound is immutable and owned.
        unsafe {
            ptr::write(
                out_value,
                OuiStyleValue {
                    tag: 6,
                    reserved: 0,
                    data: OuiStylePayload {
                        compound: token as *const OuiStyleCompound,
                    },
                },
            )
        };
        Ok(())
    })
}

// SAFETY CONTRACT: owning-thread live element and initialized output header.
// After prefix acceptance, aligned writable24-byte output; future tails remain
// untouched. Every error leaves all caller output bytes unchanged.
#[no_mangle]
pub extern "C" fn oui_element_radius_axis_length_v1(
    handle: *mut OuiElement,
    property: i32,
    output: *mut OuiRadiusAxisLengthV1,
) -> OuiStatus {
    ffi(|| {
        let property = axis_property(property)?;
        let source = element(handle as usize)?;
        let state = element_document(&source)?;
        aligned_prefix::<OuiRadiusAxisLengthV1>(output as usize)?;
        let (size, abi) = unsafe { versioned_output_header(output) }?;
        let value = {
            let engine = borrow_engine(&state)?;
            engine.radius_axis_length(source.node, property)?
        };
        let value = encode(value, size, abi)?;
        // SAFETY: caller provides the validated writable known prefix; all
        // Engine/registry borrows have ended, no callback or pointer lease.
        unsafe { ptr::write(output, value) };
        Ok(())
    })
}
