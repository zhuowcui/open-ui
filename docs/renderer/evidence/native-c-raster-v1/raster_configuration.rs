//! Versioned C transport over the same immutable Rust Engine options.

use crate::*;
use openui_geometry::{
    RasterBackend, RasterConfiguration, RasterPixelGeometry, TextEdging, TextHinting,
    TextRasterConfiguration,
};

fn checked_header<T>(value: *const T) -> Result<(u32, u32), ApiError> {
    if value.is_null() {
        return Err(invalid("versioned configuration pointer is null"));
    }
    // SAFETY: the caller guarantees a readable size word. Read only the
    // declared prefix until its size has been validated; never borrow a full
    // structure from an older caller's shorter allocation.
    let size = unsafe { ptr::read_unaligned(value.cast::<u32>()) };
    if size < 8 {
        return Err(invalid("versioned configuration header is too small"));
    }
    // SAFETY: the declared prefix has room for the ABI word.
    let abi = unsafe { ptr::read_unaligned(value.cast::<u32>().add(1)) };
    check_header(size, abi, size_of::<T>())?;
    Ok((size, abi))
}

fn text_from_c(raw: OuiTextRasterConfigurationV1) -> Result<TextRasterConfiguration, ApiError> {
    if raw.flags & !3 != 0 {
        return Err(invalid("unknown text raster flags"));
    }
    Ok(TextRasterConfiguration {
        edging: match raw.edging {
            0 => TextEdging::Alias,
            1 => TextEdging::AntiAlias,
            2 => TextEdging::SubpixelAntiAlias,
            _ => return Err(invalid("unknown text edging")),
        },
        hinting: match raw.hinting {
            0 => TextHinting::None,
            1 => TextHinting::Slight,
            2 => TextHinting::Normal,
            3 => TextHinting::Full,
            _ => return Err(invalid("unknown text hinting")),
        },
        subpixel_positioning: raw.flags & 1 != 0,
        force_autohint: raw.flags & 2 != 0,
        lcd_phase_64ths: raw
            .lcd_phase_64ths
            .try_into()
            .map_err(|_| invalid("LCD phase exceeds i16 range"))?,
    })
}

fn from_c(raw: &OuiRasterConfigurationV1) -> Result<RasterConfiguration, ApiError> {
    Ok(RasterConfiguration {
        backend: match raw.backend {
            0 => RasterBackend::Skia,
            1 => RasterBackend::ChromiumLinux,
            2 => RasterBackend::GaneshGl,
            3 => RasterBackend::ChromiumLinuxFontations,
            _ => return Err(invalid("unknown raster backend")),
        },
        pixel_geometry: match raw.pixel_geometry {
            0 => RasterPixelGeometry::Unknown,
            1 => RasterPixelGeometry::RgbHorizontal,
            2 => RasterPixelGeometry::BgrHorizontal,
            3 => RasterPixelGeometry::RgbVertical,
            4 => RasterPixelGeometry::BgrVertical,
            _ => return Err(invalid("unknown raster pixel geometry")),
        },
        gamma_milli: raw
            .gamma_milli
            .try_into()
            .map_err(|_| invalid("raster gamma exceeds u16 range"))?,
        contrast_milli: raw
            .contrast_milli
            .try_into()
            .map_err(|_| invalid("raster contrast exceeds u16 range"))?,
        author_text: text_from_c(raw.author_text)?,
        native_text: text_from_c(raw.native_text)?,
        embedded_text: text_from_c(raw.embedded_text)?,
    })
}

fn text_to_c(value: TextRasterConfiguration) -> OuiTextRasterConfigurationV1 {
    OuiTextRasterConfigurationV1 {
        edging: match value.edging {
            TextEdging::Alias => 0,
            TextEdging::AntiAlias => 1,
            TextEdging::SubpixelAntiAlias => 2,
        },
        hinting: match value.hinting {
            TextHinting::None => 0,
            TextHinting::Slight => 1,
            TextHinting::Normal => 2,
            TextHinting::Full => 3,
        },
        flags: u32::from(value.subpixel_positioning) | (u32::from(value.force_autohint) << 1),
        lcd_phase_64ths: i32::from(value.lcd_phase_64ths),
    }
}

fn to_c(
    value: RasterConfiguration,
    size: u32,
    abi: u32,
) -> Result<OuiRasterConfigurationV1, ApiError> {
    Ok(OuiRasterConfigurationV1 {
        struct_size: size,
        abi_version: abi,
        backend: match value.backend {
            RasterBackend::Skia => 0,
            RasterBackend::ChromiumLinux => 1,
            RasterBackend::GaneshGl => 2,
            RasterBackend::ChromiumLinuxFontations => 3,
            _ => {
                return Err(ApiError::new(
                    OuiStatus::InvalidState,
                    "Rust backend has no v1 C representation",
                ))
            }
        },
        pixel_geometry: match value.pixel_geometry {
            RasterPixelGeometry::Unknown => 0,
            RasterPixelGeometry::RgbHorizontal => 1,
            RasterPixelGeometry::BgrHorizontal => 2,
            RasterPixelGeometry::RgbVertical => 3,
            RasterPixelGeometry::BgrVertical => 4,
        },
        gamma_milli: u32::from(value.gamma_milli),
        contrast_milli: u32::from(value.contrast_milli),
        author_text: text_to_c(value.author_text),
        native_text: text_to_c(value.native_text),
        embedded_text: text_to_c(value.embedded_text),
    })
}

// SAFETY CONTRACT: `out_config` has a readable initialized version header and,
// when large enough, writable storage for the complete v1 structure. Larger
// caller tails and all outputs on error remain untouched. This operation owns
// no objects and can run on any thread.
#[no_mangle]
pub extern "C" fn oui_raster_configuration_init_v1(
    preset: u32,
    out_config: *mut OuiRasterConfigurationV1,
) -> OuiStatus {
    ffi(|| {
        let (size, abi) = checked_header(out_config)?;
        let configuration = match preset {
            0 => RasterConfiguration::default(),
            1 => RasterConfiguration::deterministic_aliased(false),
            2 => RasterConfiguration::deterministic_aliased(true),
            3 => RasterConfiguration::chromium_linux_lcd(),
            4 => RasterConfiguration::chromium_linux_fontations_lcd(),
            5 => RasterConfiguration::chromium_linux_ganesh(),
            _ => return Err(invalid("unknown raster preset")),
        };
        let output = to_c(configuration, size, abi)?;
        // SAFETY: writable complete storage is guaranteed after header validation.
        unsafe { ptr::write_unaligned(out_config, output) };
        Ok(())
    })
}

// SAFETY CONTRACT: inputs have readable version headers and complete readable
// storage when their declared sizes are sufficient. `out_document` is writable
// for one opaque handle and remains untouched on error. All inputs are copied;
// they may be released immediately. The document belongs to the calling thread.
#[no_mangle]
pub extern "C" fn oui_document_create_with_raster_configuration_v1(
    config: *const OuiDocumentConfig,
    raster: *const OuiRasterConfigurationV1,
    out_document: *mut *mut OuiDocument,
) -> OuiStatus {
    ffi(|| {
        if out_document.is_null() {
            return Err(invalid("output document pointer is null"));
        }
        checked_header(config)?;
        checked_header(raster)?;
        // SAFETY: complete input sizes have been validated, and storage is readable.
        let config = unsafe { ptr::read_unaligned(config) };
        let raster = unsafe { ptr::read_unaligned(raster) };
        let state = new_document_with_raster_configuration(&config, from_c(&raster)?)?;
        write_handle(out_document, LocalHandle::Document(state))
    })
}

// SAFETY CONTRACT: same ownership, prefix and output rules as document creation.
// Presentation preference in `OuiAppConfig` is separate from raster selection.
#[no_mangle]
pub extern "C" fn oui_app_create_with_raster_configuration_v1(
    config: *const OuiAppConfig,
    raster: *const OuiRasterConfigurationV1,
    out_app: *mut *mut OuiApp,
) -> OuiStatus {
    ffi(|| {
        if out_app.is_null() {
            return Err(invalid("output app pointer is null"));
        }
        checked_header(config)?;
        checked_header(raster)?;
        // SAFETY: both complete declared input objects are readable.
        let config = unsafe { ptr::read_unaligned(config) };
        let raster = unsafe { ptr::read_unaligned(raster) };
        let state = new_app_with_raster_configuration(&config, from_c(&raster)?)?;
        write_handle(out_app, LocalHandle::App(state))
    })
}

// SAFETY CONTRACT: the document is live on its owning thread; output prefix and
// tail rules match init. The returned fields are owned values, with no pointers
// or engine borrows surviving the call. Viewport and tree mutations cannot
// change the document's raster selection.
#[no_mangle]
pub extern "C" fn oui_document_get_raster_configuration_v1(
    handle: *mut OuiDocument,
    out_config: *mut OuiRasterConfigurationV1,
) -> OuiStatus {
    ffi(|| {
        let (size, abi) = checked_header(out_config)?;
        let state = document(handle as usize)?;
        let configuration = borrow_engine(&state)?.raster_configuration();
        let output = to_c(configuration, size, abi)?;
        // SAFETY: the caller supplies writable complete output storage.
        unsafe { ptr::write_unaligned(out_config, output) };
        Ok(())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_output() -> OuiRasterConfigurationV1 {
        OuiRasterConfigurationV1 {
            struct_size: size_of::<OuiRasterConfigurationV1>() as u32,
            abi_version: OUI_ABI_VERSION,
            backend: 99,
            pixel_geometry: 99,
            gamma_milli: 99,
            contrast_milli: 99,
            author_text: OuiTextRasterConfigurationV1 {
                edging: 99,
                hinting: 99,
                flags: 99,
                lcd_phase_64ths: 99,
            },
            native_text: OuiTextRasterConfigurationV1 {
                edging: 99,
                hinting: 99,
                flags: 99,
                lcd_phase_64ths: 99,
            },
            embedded_text: OuiTextRasterConfigurationV1 {
                edging: 99,
                hinting: 99,
                flags: 99,
                lcd_phase_64ths: 99,
            },
        }
    }

    fn viewport(scale: f64) -> OuiDocumentConfig {
        OuiDocumentConfig {
            struct_size: size_of::<OuiDocumentConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            viewport: OuiViewportMetrics {
                logical_width: 64.0,
                logical_height: 48.0,
                physical_width: 0,
                physical_height: 0,
                device_scale_factor: scale,
                authority: 1,
                reserved: 0,
            },
        }
    }

    #[test]
    fn c_raster_options_share_native_engine_and_keep_owned_selection() {
        #[repr(C)]
        struct Extended {
            raster: OuiRasterConfigurationV1,
            tail: u64,
        }
        let configurations = [
            RasterConfiguration::default(),
            RasterConfiguration::deterministic_aliased(false),
            RasterConfiguration::deterministic_aliased(true),
            RasterConfiguration::chromium_linux_lcd(),
            RasterConfiguration::chromium_linux_fontations_lcd(),
            RasterConfiguration::chromium_linux_ganesh(),
        ];
        for (preset, expected) in configurations.into_iter().enumerate() {
            let mut input = Extended {
                raster: empty_output(),
                tail: 0x1122334455667788,
            };
            input.raster.struct_size = size_of::<Extended>() as u32;
            assert_eq!(
                oui_raster_configuration_init_v1(preset as u32, &mut input.raster),
                OuiStatus::Ok
            );
            assert_eq!(input.tail, 0x1122334455667788);
            let original = input.raster;
            let mut handle = ptr::null_mut();
            assert_eq!(
                oui_document_create_with_raster_configuration_v1(
                    &viewport(1.25),
                    &input.raster,
                    &mut handle
                ),
                OuiStatus::Ok
            );
            // Callers may reuse/release input storage immediately after construction.
            input.raster = empty_output();
            let state = document(handle as usize).unwrap();
            assert_eq!(
                borrow_engine(&state).unwrap().raster_configuration(),
                expected
            );
            let owned_scene = borrow_engine_mut(&state).unwrap().scene().unwrap();
            // Mutation through the native Rust document reaches this C document.
            state
                .native
                .body()
                .set_background_color(Color::RED)
                .unwrap();
            let mut root = ptr::null_mut();
            assert_eq!(oui_document_root(handle, &mut root), OuiStatus::Ok);
            let output = OuiStyleValue {
                tag: 1,
                reserved: 0,
                data: OuiStylePayload {
                    length: OuiLength {
                        value: 32.0,
                        unit: 0,
                    },
                },
            };
            assert_eq!(
                oui_element_set_property(root, StyleProperty::Width as i32, &output),
                OuiStatus::Ok
            );
            let mut changed = viewport(2.0);
            changed.viewport.logical_width = 80.0;
            assert_eq!(oui_document_set_viewport(handle, &changed), OuiStatus::Ok);
            let mut current = Extended {
                raster: empty_output(),
                tail: 0x9988776655443322,
            };
            current.raster.struct_size = size_of::<Extended>() as u32;
            assert_eq!(
                oui_document_get_raster_configuration_v1(handle, &mut current.raster),
                OuiStatus::Ok
            );
            assert_eq!(current.raster, original);
            assert_eq!(current.tail, 0x9988776655443322);
            assert_eq!(owned_scene.raster_configuration(), expected);
            assert_eq!(
                borrow_engine_mut(&state)
                    .unwrap()
                    .scene()
                    .unwrap()
                    .raster_configuration(),
                expected
            );
            drop(state);
            assert_eq!(oui_element_destroy(root), OuiStatus::Ok);
            assert_eq!(oui_document_destroy(handle), OuiStatus::Ok);
            assert_eq!(owned_scene.raster_configuration(), expected);
        }
    }

    #[test]
    fn c_raster_custom_fields_reach_the_shared_engine_without_narrowing() {
        let expected = RasterConfiguration {
            backend: RasterBackend::ChromiumLinuxFontations,
            pixel_geometry: RasterPixelGeometry::BgrVertical,
            gamma_milli: u16::MAX,
            contrast_milli: 0,
            author_text: TextRasterConfiguration {
                edging: TextEdging::AntiAlias,
                hinting: TextHinting::Full,
                subpixel_positioning: false,
                force_autohint: true,
                lcd_phase_64ths: i16::MIN,
            },
            native_text: TextRasterConfiguration {
                edging: TextEdging::Alias,
                hinting: TextHinting::None,
                subpixel_positioning: true,
                force_autohint: false,
                lcd_phase_64ths: 0,
            },
            embedded_text: TextRasterConfiguration {
                edging: TextEdging::SubpixelAntiAlias,
                hinting: TextHinting::Normal,
                subpixel_positioning: true,
                force_autohint: true,
                lcd_phase_64ths: i16::MAX,
            },
        };
        let raw = OuiRasterConfigurationV1 {
            struct_size: size_of::<OuiRasterConfigurationV1>() as u32,
            abi_version: OUI_ABI_VERSION,
            backend: 3,
            pixel_geometry: 4,
            gamma_milli: 65535,
            contrast_milli: 0,
            author_text: OuiTextRasterConfigurationV1 {
                edging: 1,
                hinting: 3,
                flags: 2,
                lcd_phase_64ths: -32768,
            },
            native_text: OuiTextRasterConfigurationV1 {
                edging: 0,
                hinting: 0,
                flags: 1,
                lcd_phase_64ths: 0,
            },
            embedded_text: OuiTextRasterConfigurationV1 {
                edging: 2,
                hinting: 2,
                flags: 3,
                lcd_phase_64ths: 32767,
            },
        };
        let mut handle = ptr::null_mut();
        assert_eq!(
            oui_document_create_with_raster_configuration_v1(&viewport(1.5), &raw, &mut handle),
            OuiStatus::Ok
        );
        let state = document(handle as usize).unwrap();
        assert_eq!(state.native.raster_configuration().unwrap(), expected);
        assert_eq!(
            borrow_engine(&state).unwrap().raster_configuration(),
            expected
        );
        let mut queried = empty_output();
        assert_eq!(
            oui_document_get_raster_configuration_v1(handle, &mut queried),
            OuiStatus::Ok
        );
        assert_eq!(queried, raw);
        drop(state);
        assert_eq!(oui_document_destroy(handle), OuiStatus::Ok);
    }

    #[test]
    fn c_raster_options_reject_short_headers_bad_fields_and_preserve_outputs() {
        // These allocations really contain only the declared prefix. Miri
        // must catch any attempt to form a reference to the complete struct.
        for size in [0_u32, 1, 4, 7] {
            let mut size_word = size;
            assert_eq!(
                oui_raster_configuration_init_v1(0, ptr::addr_of_mut!(size_word).cast()),
                OuiStatus::InvalidArgument
            );
            assert_eq!(size_word, size);
        }
        let mut prefix = [8_u32, OUI_ABI_VERSION];
        assert_eq!(
            oui_raster_configuration_init_v1(0, prefix.as_mut_ptr().cast()),
            OuiStatus::InvalidArgument
        );
        assert_eq!(prefix, [8, OUI_ABI_VERSION]);
        let mut raw = empty_output();
        let initial = raw;
        assert_eq!(
            oui_raster_configuration_init_v1(99, &mut raw),
            OuiStatus::InvalidArgument
        );
        assert_eq!(raw, initial);
        raw.abi_version = 1;
        let wrong_abi = raw;
        assert_eq!(
            oui_raster_configuration_init_v1(0, &mut raw),
            OuiStatus::AbiMismatch
        );
        assert_eq!(raw, wrong_abi);
        let mut storage = [0x5a_u8; size_of::<OuiRasterConfigurationV1>() + 2];
        // SAFETY: there is complete writable storage, deliberately unaligned.
        let unaligned = unsafe { storage.as_mut_ptr().add(1).cast() };
        unsafe { ptr::write_unaligned(unaligned, initial) };
        assert_eq!(
            oui_raster_configuration_init_v1(3, unaligned),
            OuiStatus::Ok
        );
        assert_eq!(storage[0], 0x5a);
        assert_eq!(*storage.last().unwrap(), 0x5a);
        // SAFETY: the API initialized the complete declared output.
        let unaligned_output = unsafe { ptr::read_unaligned(unaligned) };
        assert_eq!(
            from_c(&unaligned_output).unwrap(),
            RasterConfiguration::chromium_linux_lcd()
        );
        raw = initial;
        assert_eq!(oui_raster_configuration_init_v1(3, &mut raw), OuiStatus::Ok);
        let valid = raw;
        let mut invalids = Vec::new();
        raw.backend = 99;
        invalids.push(raw);
        raw = valid;
        raw.pixel_geometry = 99;
        invalids.push(raw);
        raw = valid;
        raw.gamma_milli = 65536;
        invalids.push(raw);
        raw = valid;
        raw.contrast_milli = 65536;
        invalids.push(raw);
        raw = valid;
        raw.author_text.edging = 3;
        invalids.push(raw);
        raw = valid;
        raw.native_text.hinting = 4;
        invalids.push(raw);
        raw = valid;
        raw.embedded_text.flags = 4;
        invalids.push(raw);
        raw = valid;
        raw.author_text.lcd_phase_64ths = 32768;
        invalids.push(raw);
        raw = valid;
        raw.native_text.lcd_phase_64ths = -32769;
        invalids.push(raw);
        let mut sentinel_storage = 0_u8;
        let sentinel = ptr::addr_of_mut!(sentinel_storage).cast::<OuiDocument>();
        let app_config = OuiAppConfig {
            struct_size: size_of::<OuiAppConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            title: OuiUtf8 {
                data: ptr::null(),
                length: 0,
            },
            width: 64,
            height: 48,
            backend: 0,
            reserved: 0,
        };
        for invalid in invalids {
            let mut handle = sentinel;
            assert_eq!(
                oui_document_create_with_raster_configuration_v1(
                    &viewport(1.0),
                    &invalid,
                    &mut handle
                ),
                OuiStatus::InvalidArgument
            );
            assert_eq!(handle, sentinel);
            let mut app = sentinel.cast::<OuiApp>();
            assert_eq!(
                oui_app_create_with_raster_configuration_v1(&app_config, &invalid, &mut app),
                OuiStatus::InvalidArgument
            );
            assert_eq!(app, sentinel.cast());
        }
        let mut handle = sentinel;
        assert_eq!(
            oui_document_create_with_raster_configuration_v1(
                prefix.as_ptr().cast(),
                &valid,
                &mut handle
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!(handle, sentinel);
        assert_eq!(
            oui_document_create_with_raster_configuration_v1(
                &viewport(1.0),
                prefix.as_ptr().cast(),
                &mut handle
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!(handle, sentinel);
        let mut app = sentinel.cast::<OuiApp>();
        assert_eq!(
            oui_app_create_with_raster_configuration_v1(prefix.as_ptr().cast(), &valid, &mut app),
            OuiStatus::InvalidArgument
        );
        assert_eq!(app, sentinel.cast());
        assert_eq!(
            oui_app_create_with_raster_configuration_v1(
                &app_config,
                prefix.as_ptr().cast(),
                &mut app
            ),
            OuiStatus::InvalidArgument
        );
        assert_eq!(app, sentinel.cast());
    }

    #[test]
    fn c_raster_query_checks_thread_lifetime_and_reentrancy() {
        let mut input = empty_output();
        assert_eq!(
            oui_raster_configuration_init_v1(4, &mut input),
            OuiStatus::Ok
        );
        let mut handle = ptr::null_mut();
        assert_eq!(
            oui_document_create_with_raster_configuration_v1(&viewport(1.0), &input, &mut handle),
            OuiStatus::Ok
        );
        let state = document(handle as usize).unwrap();
        let held = borrow_engine_mut(&state).unwrap();
        let mut output = empty_output();
        assert_eq!(
            oui_document_get_raster_configuration_v1(handle, &mut output),
            OuiStatus::Reentrant
        );
        assert_eq!(output, empty_output());
        drop(held);
        drop(state);
        let address = handle as usize;
        assert_eq!(
            std::thread::spawn(move || {
                let mut output = empty_output();
                let result = oui_document_get_raster_configuration_v1(
                    address as *mut OuiDocument,
                    &mut output,
                );
                assert_eq!(output, empty_output());
                result
            })
            .join()
            .unwrap(),
            OuiStatus::WrongThread
        );
        assert_eq!(oui_document_destroy(handle), OuiStatus::Ok);
        assert_eq!(
            oui_document_get_raster_configuration_v1(handle, &mut output),
            OuiStatus::InvalidHandle
        );
        assert_eq!(output, empty_output());
    }

    #[test]
    fn c_app_and_default_constructors_use_the_same_immutable_rust_options() {
        let mut raster = empty_output();
        assert_eq!(
            oui_raster_configuration_init_v1(4, &mut raster),
            OuiStatus::Ok
        );
        let config = OuiAppConfig {
            struct_size: size_of::<OuiAppConfig>() as u32,
            abi_version: OUI_ABI_VERSION,
            title: OuiUtf8 {
                data: b"Raster".as_ptr(),
                length: 6,
            },
            width: 64,
            height: 48,
            backend: 0,
            reserved: 0,
        };
        let mut app = ptr::null_mut();
        assert_eq!(
            oui_app_create_with_raster_configuration_v1(&config, &raster, &mut app),
            OuiStatus::Ok
        );
        let mut handle = ptr::null_mut();
        assert_eq!(oui_app_document(app, &mut handle), OuiStatus::Ok);
        let mut output = empty_output();
        assert_eq!(
            oui_document_get_raster_configuration_v1(handle, &mut output),
            OuiStatus::Ok
        );
        assert_eq!(output, raster);
        assert_eq!(oui_app_destroy(app), OuiStatus::Ok);
        assert_eq!(
            oui_document_get_raster_configuration_v1(handle, &mut output),
            OuiStatus::Ok
        );
        assert_eq!(output, raster);
        assert_eq!(oui_document_destroy(handle), OuiStatus::Ok);
        assert_eq!(
            oui_document_create(&viewport(1.0), &mut handle),
            OuiStatus::Ok
        );
        assert_eq!(
            oui_document_get_raster_configuration_v1(handle, &mut output),
            OuiStatus::Ok
        );
        assert_eq!(from_c(&output).unwrap(), RasterConfiguration::default());
        assert_eq!(oui_document_destroy(handle), OuiStatus::Ok);
    }
}
