//! Raster configuration must change the raster, independently of layout.

use openui_geometry::RasterConfiguration;
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
