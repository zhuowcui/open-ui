use super::*;

/// Skia's legacy 8-bit store, `Sk4f_toL32`: `pin(v * 255 + 0.5, 0, 255)`
/// then truncation.
fn to_byte(value: f32) -> u8 {
    (value * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

fn bytes(color: Color) -> [u8; 4] {
    [color.r, color.g, color.b, color.a].map(to_byte)
}

fn resolve(input: &str) -> Color {
    color_function(input).unwrap_or_else(|| panic!("{input} should resolve"))
}

fn assert_channels(input: &str, expected: [f32; 3]) {
    let color = resolve(input);
    for (actual, expected) in [color.r, color.g, color.b].into_iter().zip(expected) {
        assert!(
            (actual - expected).abs() <= 2e-8,
            "{input}: {actual} != {expected}"
        );
    }
}

#[test]
fn derived_matrices_match_chromium_float_results() {
    let expected = [
        [1.047_857_28, 0.022_907_32, -0.050_162_22],
        [0.029_570_45, 0.990_475_65, -0.017_061_51],
        [-0.009_240_47, 0.015_052_92, 0.751_970_95],
    ];
    for (row, expected) in CONVERSIONS.d65_to_d50.iter().zip(expected) {
        for (actual, expected) in row.iter().zip(expected) {
            assert!((actual - expected).abs() < 1e-8, "{actual} != {expected}");
        }
    }
    let encode = CONVERSIONS.srgb_encode;
    assert_eq!(encode.g, 1.0 / 2.4_f32);
    assert_eq!(encode.c, 12.92);
    assert!((encode.a - 1.137_119_05).abs() < 1e-7);
    assert!((encode.d - 0.003_130_805).abs() < 1e-9);
}

#[test]
fn shadcn_neutral_tokens_resolve_to_chromium_channels() {
    // Float values from a float32 emulation of Chromium 147's
    // `Color::toSkColor4f` (see docs/progress/tracks/engine.md).
    assert_channels(
        "oklch(0.205 0 0)",
        [0.090_509_50, 0.090_534_66, 0.090_537_01],
    );
    assert_channels(
        "oklch(0.985 0 0)",
        [0.980_129_00, 0.980_307_94, 0.980_324_86],
    );
    assert_eq!(bytes(resolve("oklch(0.205 0 0)")), [23, 23, 23, 255]);
    assert_eq!(bytes(resolve("oklch(0.985 0 0)")), [250, 250, 250, 255]);
}

#[test]
fn out_of_gamut_channels_stay_unclamped() {
    let red = resolve("oklch(0.577 0.245 27.325)");
    assert!((red.g - -0.095_851_08).abs() < 1e-7, "{}", red.g);
    assert_eq!(bytes(red), [231, 0, 11, 255]);
    let white = resolve("oklch(1 0 0 / 10%)");
    assert!(white.g > 1.0 && white.b > 1.0 && white.r < 1.0);
    assert_eq!(white.a, 0.1_f32);
}

#[test]
fn oklch_channels_clamp_and_wrap_as_blink_stores_them() {
    assert_eq!(resolve("oklch(-0.5 0.1 10)"), resolve("oklch(0 0.1 10)"));
    assert_eq!(resolve("oklch(1.5 0.1 10)"), resolve("oklch(1 0.1 10)"));
    assert_eq!(resolve("oklch(0.5 -0.1 10)"), resolve("oklch(0.5 0 10)"));
    assert_eq!(resolve("oklch(0.5 0.1 370)"), resolve("oklch(0.5 0.1 10)"));
    assert_eq!(
        resolve("oklch(0.5 0.1 -350deg)"),
        resolve("oklch(0.5 0.1 10)")
    );
    assert_eq!(resolve("oklch(50% 25% 10)"), resolve("oklch(0.5 0.1 10)"));
    assert_eq!(resolve("OKLCH(0.5 0.1 none)"), resolve("oklch(0.5 0.1 0)"));
    assert_eq!(resolve("oklch(0.5 0.1 10/50%)").a, 0.5);
    assert_eq!(resolve("oklch(0.5 0.1 10 / 2)").a, 1.0);
    assert_eq!(resolve("oklch(0.5 0.1 10 / none)").a, 0.0);
}

#[test]
fn malformed_color_functions_are_rejected() {
    for input in [
        "oklch(0.5 0.1)",
        "oklch(0.5 0.1 10 20)",
        "oklch(0.5, 0.1, 10)",
        "oklch(0.5 0.1 10%)",
        "oklch(0.5 0.1 1rad)",
        "oklch(0.5 0.1 10 / 1 / 1)",
        "oklch(5. 0.1 10)",
        "oklch(inf 0.1 10)",
        "oklch(calc(1) 0.1 10)",
        "oklch(from red l c h)",
        "oklch(0.5 0.1 10) x",
        "oklab(0.5 0.1 0.1deg)",
        "color-mix(in hsl, red, blue)",
        "color-mix(in oklab, red 120%, blue)",
        "color-mix(in oklab, red 0%, blue 0%)",
        "color-mix(in oklab, currentcolor, blue)",
        "color-mix(in oklab red, blue)",
        "color-mix(in srgb shorter hue, red, blue)",
        "color-mix(red)",
    ] {
        assert!(
            color_function(input).is_none(),
            "{input} should be rejected"
        );
    }
}

#[test]
fn mix_percentages_normalize_in_double() {
    assert_eq!(normalize_percentages(None, None), (0.5, 1.0));
    assert_eq!(normalize_percentages(Some(30.0), None), (0.7, 1.0));
    assert_eq!(normalize_percentages(None, Some(30.0)), (0.3, 1.0));
    let (progress, multiplier) = normalize_percentages(Some(20.0), Some(30.0));
    assert!((progress - 0.6).abs() < 1e-15 && (multiplier - 0.5).abs() < 1e-15);
    assert_eq!(normalize_percentages(Some(60.0), Some(60.0)), (0.5, 1.0));
    assert_eq!(normalize_percentages(Some(0.0), Some(50.0)).0, 1.0);
}

#[test]
fn mixing_with_transparent_keeps_oklab_channels() {
    let mixed = resolve("color-mix(in oklab, oklch(0.205 0 0) 50%, transparent)");
    let solid = resolve("oklch(0.205 0 0)");
    assert_eq!([mixed.r, mixed.g, mixed.b], [solid.r, solid.g, solid.b]);
    assert_eq!(mixed.a, 0.5);
}

#[test]
fn hue_methods_follow_blink() {
    let mix = |method| hue_interpolation(350.0, 10.0, 0.5, method);
    assert_eq!(mix(HueMethod::Shorter), 0.0);
    assert_eq!(mix(HueMethod::Longer), 180.0);
    assert_eq!(mix(HueMethod::Increasing), 0.0);
    assert_eq!(mix(HueMethod::Decreasing), 180.0);
}

#[test]
fn tailwind_opacity_mixes_match_the_chromium_emulation() {
    let white = resolve("color-mix(in oklab, white 10%, transparent)");
    assert_channels(
        "color-mix(in oklab, white 10%, transparent)",
        [1.000_000_60, 0.999_999_88, 1.000_000_12],
    );
    assert_eq!(white.a, 0.100_000_024);
    assert_eq!(bytes(white), [255, 255, 255, 26]);

    let blue = resolve("color-mix(in oklab, #3b82f6 50%, transparent)");
    assert_channels(
        "color-mix(in oklab, #3b82f6 50%, transparent)",
        [0.231_373_31, 0.509_803_59, 0.964_705_94],
    );
    assert_eq!(bytes(blue), [59, 130, 246, 128]);
}

#[test]
fn achromatic_operand_takes_the_other_hue_in_oklch() {
    assert_channels(
        "color-mix(in oklch, oklch(0.7 0.1 200), white)",
        [0.658_911_41, 0.847_995_82, 0.857_289_08],
    );
    assert_eq!(
        bytes(resolve("color-mix(in oklch, oklch(0.7 0.1 200), white)")),
        [168, 216, 219, 255]
    );
}

#[test]
fn srgb_mix_keeps_the_half_tie_that_skia_rounds_up() {
    let mixed = resolve("color-mix(in srgb, red 30%, blue)");
    assert_eq!([mixed.r, mixed.g, mixed.b], [0.3_f32, 0.0, 0.7]);
    assert_eq!(mixed.r * 255.0, 76.5);
    assert_eq!(bytes(mixed), [77, 0, 179, 255]);
}

#[test]
fn style_parser_accepts_color_functions() {
    use crate::{parse_literal, StyleProperty, StyleValue};
    let value = parse_literal(StyleProperty::BackgroundColor, "oklch(0.205 0 0)").unwrap();
    assert_eq!(value, StyleValue::Color(resolve("oklch(0.205 0 0)")));
    assert!(parse_literal(StyleProperty::Color, "color-mix(in oklab, red, blue)").is_ok());
    assert!(parse_literal(StyleProperty::Color, "oklch(0.5 0.1)").is_err());
}
