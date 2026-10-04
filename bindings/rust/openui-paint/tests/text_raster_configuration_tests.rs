//! Raster configuration must change the raster, independently of layout.

use openui_geometry::{
    RasterBackend, RasterConfiguration, TextEdging, TextHinting, TextRasterConfiguration,
};
use openui_paint::text_painter::paint_text;
use openui_style::{ComputedStyle, FontFamilyList};
use openui_text::{Font, FontCollection, FontDescription, TextDirection, TextShaper};
use skia_safe::{surfaces, Color, ImageInfo, PixelGeometry, SurfaceProps, SurfacePropsFlags};

fn render(role: usize, scale: f64, phase: i16) -> Vec<u8> {
    let mut style = ComputedStyle::default();
    let fields = style.fields_mut();
    fields.font_family = FontFamilyList::single("DejaVu Sans");
    fields.font_size = 17.0;
    fields.device_scale_factor = scale;
    fields.native_control_text = role == 1;
    fields.embedded_document_text = role == 2;
    fields.raster_configuration = RasterConfiguration::chromium_linux_fontations_lcd();
    let settings = match role {
        0 => &mut fields.raster_configuration.author_text,
        1 => &mut fields.raster_configuration.native_text,
        _ => &mut fields.raster_configuration.embedded_text,
    };
    settings.lcd_phase_64ths = phase;
    let font = Font::new_in_collection(
        FontDescription::from_computed_style(&style),
        FontCollection::deterministic_test(),
    );
    let shaped = TextShaper::new().shape("Xe", &font, TextDirection::Ltr);
    assert_eq!(shaped.num_glyphs(), 2);
    let info = ImageInfo::new_n32_premul((160, 100), None);
    let props = SurfaceProps::new(SurfacePropsFlags::default(), PixelGeometry::RGBH);
    let mut surface = surfaces::raster(&info, None, Some(&props)).unwrap();
    let canvas = surface.canvas();
    canvas.clear(Color::WHITE);
    canvas.scale((scale as f32, scale as f32));
    paint_text(canvas, &shaped, (12.0, 25.0), &style);
    let mut pixels = vec![0; info.compute_byte_size(info.min_row_bytes())];
    assert!(surface.image_snapshot().read_pixels(
        &info,
        &mut pixels,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    ));
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[..3] != [255; 3]));
    pixels
}

#[test]
fn lcd_phase_moves_ink_by_physical_pixels_for_each_text_role() {
    for role in 0..3 {
        for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let original = render(role, scale, 0);
            for phase in [-128, -64, 64, 128] {
                let shifted = render(role, scale, phase);
                let delta = i32::from(phase) / 64;
                for y in 0..100 {
                    for x in 0..160 {
                        let source_x = x - delta;
                        let actual = &shifted[(y * 160 + x) as usize * 4..][..4];
                        let expected = if (0..160).contains(&source_x) {
                            &original[(y * 160 + source_x) as usize * 4..][..4]
                        } else {
                            &[255; 4]
                        };
                        assert_eq!(
                            actual, expected,
                            "role {role}, scale {scale}, phase {phase}, pixel ({x}, {y})"
                        );
                    }
                }
            }
        }
    }
}

fn render_control_strike(
    role: usize,
    family: &str,
    size: f32,
    scale: f64,
    backend: RasterBackend,
    origin: (f32, f32),
) -> Vec<u8> {
    let mut style = ComputedStyle::default();
    let fields = style.fields_mut();
    fields.font_family = FontFamilyList::single(family);
    fields.font_size = size;
    fields.device_scale_factor = scale;
    fields.native_control_text = role == 1;
    fields.embedded_document_text = role == 2;
    fields.raster_configuration = RasterConfiguration::default().with_backend(backend);
    let fitted = TextRasterConfiguration {
        edging: TextEdging::AntiAlias,
        hinting: TextHinting::Full,
        subpixel_positioning: false,
        force_autohint: false,
        lcd_phase_64ths: 0,
    };
    fields.raster_configuration.native_text = fitted;
    fields.raster_configuration.embedded_text = fitted;
    let font = Font::new_in_collection(
        FontDescription::from_computed_style(&style),
        FontCollection::deterministic_test(),
    );
    let shaped = TextShaper::new().shape("X", &font, TextDirection::Ltr);
    assert_eq!(shaped.num_glyphs(), 1);
    let info = ImageInfo::new_n32_premul((200, 160), None);
    let mut surface = surfaces::raster(&info, None, None).unwrap();
    let canvas = surface.canvas();
    canvas.clear(Color::WHITE);
    canvas.scale((scale as f32, scale as f32));
    paint_text(canvas, &shaped, origin, &style);
    let mut pixels = vec![0; info.compute_byte_size(info.min_row_bytes())];
    assert!(surface.image_snapshot().read_pixels(
        &info,
        &mut pixels,
        info.min_row_bytes(),
        (0, 0),
        skia_safe::image::CachingHint::Disallow,
    ));
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[..3] != [255; 3]));
    pixels
}

#[test]
fn native_outlines_fit_at_physical_size_with_default_and_explicit_backends() {
    // A single glyph must have the same physical ink regardless of how
    // logical size and device scale divide the physical strike. This uses
    // independently shaped unit-scale glyphs and compares every output byte.
    // Full fitting makes scaling an already fitted logical outline observable.
    for role in [1, 2] {
        for family in ["DejaVu Sans Mono", "DejaVu Serif"] {
            for size in [10.0, 13.0, 17.0, 24.0] {
                for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
                    let physical = render_control_strike(
                        role,
                        family,
                        size * scale as f32,
                        1.0,
                        RasterBackend::Skia,
                        (16.0 * scale as f32, 32.0 * scale as f32),
                    );
                    for backend in [
                        RasterBackend::Skia,
                        RasterBackend::ChromiumLinux,
                        RasterBackend::ChromiumLinuxFontations,
                    ] {
                        let replay =
                            render_control_strike(role, family, size, scale, backend, (16.0, 32.0));
                        assert!(
                            replay == physical,
                            "role {role}, family {family}, size {size}, scale {scale}, backend {backend:?}"
                        );
                    }
                }
            }
        }
    }
}
