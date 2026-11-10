//! `oklch()`, `oklab()` and `color-mix()` resolved as pinned Chromium 147 does.
//!
//! Blink stores these colors as float32 channels in their own space
//! (`platform/graphics/color.cc`), resolves `color-mix()` of absolute colors
//! at computed-value time and converts to sRGB only when painting
//! (`Color::toSkColor4f`). That conversion has no gamut mapping: channels
//! outside [0, 1] stay unclamped until Skia quantizes them. The math below
//! follows `ui/gfx/color_conversions.cc`, `skia/ext/skcms_ext.cc` and
//! `skcms.cc` operation by operation, in the same float and double widths;
//! Chromium builds with `-ffp-contract=off` and Rust never contracts, so each
//! float operation rounds as it does there.

use crate::Color;
use std::sync::LazyLock;

/// Color spaces this module resolves. Other `color-mix()` spaces are
/// rejected rather than approximated. `srgb-linear` is left out because
/// Chromium paints it with skcms's approximate `powf_`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Space {
    /// Hex, `rgb()` and named colors; channels are 0..255 as in Blink.
    SrgbLegacy,
    Srgb,
    Oklab,
    Oklch,
}

impl Space {
    fn is_rgb_like(self) -> bool {
        matches!(self, Self::SrgbLegacy | Self::Srgb)
    }
}

/// `blink::Color` in one of the supported spaces: float channels, straight
/// alpha and a `none` bit per channel (`color.h`).
#[derive(Debug, Clone, Copy, PartialEq)]
struct CssColor {
    space: Space,
    params: [f32; 3],
    alpha: f32,
    /// `none` bits for params 0..3 and alpha (index 3).
    none: [bool; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HueMethod {
    Shorter,
    Longer,
    Increasing,
    Decreasing,
}

/// Resolve a CSS color function to the sRGB color Chromium paints. Returns
/// `None` for anything outside the supported grammar.
pub(crate) fn color_function(input: &str) -> Option<Color> {
    let color = parse_color(input.trim())?;
    let [r, g, b] = to_srgb(&color);
    Some(Color::from_rgba_f32(r, g, b, color.alpha))
}

fn parse_color(input: &str) -> Option<CssColor> {
    let (name, body) = function_parts(input)?;
    if name.eq_ignore_ascii_case("oklch") {
        parse_ok_function(body, Space::Oklch)
    } else if name.eq_ignore_ascii_case("oklab") {
        parse_ok_function(body, Space::Oklab)
    } else if name.eq_ignore_ascii_case("color-mix") {
        parse_color_mix(body)
    } else {
        None
    }
}

/// Split `name(body)` with balanced parentheses and nothing after `)`.
fn function_parts(input: &str) -> Option<(&str, &str)> {
    let open = input.find('(')?;
    let name = &input[..open];
    if name.is_empty() || !name.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') {
        return None;
    }
    let body = input[open + 1..].strip_suffix(')')?;
    let mut depth = 0_i32;
    for byte in body.bytes() {
        match byte {
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth < 0 {
                    return None;
                }
            }
            _ => {}
        }
    }
    (depth == 0).then_some((name, body))
}

/// Split at top-level occurrences of `separator` (outside parentheses).
fn split_top_level(input: &str, separator: impl Fn(char) -> bool) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0_i32;
    let mut start = 0;
    for (index, ch) in input.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth -= 1,
            ch if depth == 0 && separator(ch) => {
                parts.push(&input[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn tokens(input: &str) -> Vec<&str> {
    split_top_level(input, |ch| ch.is_ascii_whitespace())
        .into_iter()
        .filter(|token| !token.is_empty())
        .collect()
}

/// A CSS `<number>` token: sign, digits with an optional fraction (or a
/// leading-point fraction) and an optional exponent. Rust's `f64` parse is
/// correctly rounded, as Blink's tokenizer conversion is.
fn css_number(input: &str) -> Option<f64> {
    let bytes = input.as_bytes();
    let mut index = usize::from(matches!(bytes.first(), Some(b'+' | b'-')));
    let digits = |index: &mut usize| {
        let start = *index;
        while bytes.get(*index).is_some_and(u8::is_ascii_digit) {
            *index += 1;
        }
        *index - start
    };
    let whole = digits(&mut index);
    let mut fraction = 0;
    if bytes.get(index) == Some(&b'.') {
        index += 1;
        fraction = digits(&mut index);
        if fraction == 0 {
            return None;
        }
    }
    if whole == 0 && fraction == 0 {
        return None;
    }
    if matches!(bytes.get(index), Some(b'e' | b'E')) {
        index += 1;
        if matches!(bytes.get(index), Some(b'+' | b'-')) {
            index += 1;
        }
        if digits(&mut index) == 0 {
            return None;
        }
    }
    if index != bytes.len() {
        return None;
    }
    input.parse::<f64>().ok().filter(|value| value.is_finite())
}

enum Channel {
    None,
    Value(f64),
}

/// `ConsumeChannel` + `ResolveColorChannelForNumericLiteral`: a number, a
/// percentage of `percentage_base`, or `none`. Hue takes a number or `deg`
/// and no percentage.
fn channel(token: &str, percentage_base: Option<f64>, hue: bool) -> Option<Channel> {
    if token.eq_ignore_ascii_case("none") {
        return Some(Channel::None);
    }
    if let Some(number) = token.strip_suffix('%') {
        let base = percentage_base?;
        return Some(Channel::Value(css_number(number)? / 100.0 * base));
    }
    if hue {
        let lower = token.to_ascii_lowercase();
        if let Some(number) = lower.strip_suffix("deg") {
            return Some(Channel::Value(css_number(number)?));
        }
    }
    css_number(token).map(Channel::Value)
}

/// `ResolveAlphaForNumericLiteral`: clamp to [0, 1]; not rounded to 8 bits
/// for these spaces.
fn alpha_channel(token: &str) -> Option<Channel> {
    if token.eq_ignore_ascii_case("none") {
        return Some(Channel::None);
    }
    let value = match token.strip_suffix('%') {
        Some(number) => css_number(number)? / 100.0,
        None => css_number(token)?,
    };
    Some(Channel::Value(value.clamp(0.0, 1.0)))
}

/// Percentage reference values from `color_function.h` `kMetadataMap`:
/// L 100% is 1, and C (or a and b) 100% is 0.4.
const OK_PERCENT_BASE: [f64; 3] = [1.0, 0.4, 0.4];

fn parse_ok_function(body: &str, space: Space) -> Option<CssColor> {
    let slash = split_top_level(body, |ch| ch == '/');
    let (channels, alpha) = match slash.as_slice() {
        [channels] => (*channels, None),
        [channels, alpha] => (*channels, Some(*alpha)),
        _ => return None,
    };
    let channels = tokens(channels);
    if channels.len() != 3 {
        return None;
    }
    let mut params = [0.0_f64; 3];
    let mut none = [false; 4];
    for (index, token) in channels.iter().enumerate() {
        let hue = space == Space::Oklch && index == 2;
        let base = (!hue).then_some(OK_PERCENT_BASE[index]);
        match channel(token, base, hue)? {
            Channel::None => none[index] = true,
            Channel::Value(value) => params[index] = value,
        }
    }
    if space == Space::Oklch && !none[2] {
        // `MakePerColorSpaceAdjustments`: wrap the hue in double.
        params[2] = (params[2] % 360.0 + 360.0) % 360.0;
    }
    let mut alpha_value = 1.0;
    if let Some(alpha) = alpha {
        let alpha = tokens(alpha);
        let [alpha] = alpha.as_slice() else {
            return None;
        };
        match alpha_channel(alpha)? {
            Channel::None => none[3] = true,
            Channel::Value(value) => alpha_value = value,
        }
    }
    Some(from_color_space(
        space,
        params.map(|value| value as f32),
        alpha_value as f32,
        none,
    ))
}

/// `Color::FromColorSpace`: `none` is stored as 0; L is clamped to [0, 1]
/// and chroma to at least 0; alpha to [0, 1].
fn from_color_space(space: Space, mut params: [f32; 3], alpha: f32, none: [bool; 4]) -> CssColor {
    for (param, is_none) in params.iter_mut().zip(none) {
        if is_none {
            *param = 0.0;
        }
    }
    if matches!(space, Space::Oklab | Space::Oklch) {
        params[0] = params[0].clamp(0.0, 1.0);
    }
    if space == Space::Oklch {
        params[1] = params[1].max(0.0);
    }
    CssColor {
        space,
        params,
        alpha: if none[3] { 0.0 } else { alpha.clamp(0.0, 1.0) },
        none,
    }
}

// ── color-mix() ─────────────────────────────────────────────────────────

fn parse_color_mix(body: &str) -> Option<CssColor> {
    let parts = split_top_level(body, |ch| ch == ',');
    let (space, hue_method, operands) = match parts.as_slice() {
        [first, a, b] => {
            let (space, method) = interpolation_space(first)?;
            (space, method, [*a, *b])
        }
        // `in <space>` is optional in 147 and defaults to oklab.
        [a, b] => (Space::Oklab, HueMethod::Shorter, [*a, *b]),
        _ => return None,
    };
    let [(color1, p1), (color2, p2)] = [mix_operand(operands[0])?, mix_operand(operands[1])?];
    // Literal percentages must lie in [0%, 100%]; both zero is invalid.
    if [p1, p2]
        .iter()
        .flatten()
        .any(|p| !(0.0..=100.0).contains(p))
    {
        return None;
    }
    if p1 == Some(0.0) && p2 == Some(0.0) {
        return None;
    }
    let (progress, alpha_multiplier) = normalize_percentages(p1, p2);
    Some(mix(
        color1,
        color2,
        space,
        hue_method,
        progress as f32,
        alpha_multiplier as f32,
    ))
}

fn interpolation_space(input: &str) -> Option<(Space, HueMethod)> {
    let words = tokens(input);
    let (in_keyword, rest) = words.split_first()?;
    if !in_keyword.eq_ignore_ascii_case("in") {
        return None;
    }
    let (space, rest) = rest.split_first()?;
    let space = match space.to_ascii_lowercase().as_str() {
        "srgb" => Space::Srgb,
        "oklab" => Space::Oklab,
        "oklch" => Space::Oklch,
        _ => return None,
    };
    let method = match rest {
        [] => HueMethod::Shorter,
        [method, hue] if space == Space::Oklch && hue.eq_ignore_ascii_case("hue") => {
            match method.to_ascii_lowercase().as_str() {
                "shorter" => HueMethod::Shorter,
                "longer" => HueMethod::Longer,
                "increasing" => HueMethod::Increasing,
                "decreasing" => HueMethod::Decreasing,
                _ => return None,
            }
        }
        _ => return None,
    };
    Some((space, method))
}

/// A color and an optional percentage, in either order.
fn mix_operand(input: &str) -> Option<(CssColor, Option<f64>)> {
    let words = tokens(input);
    let percentage = |token: &str| css_number(token.strip_suffix('%')?);
    match words.as_slice() {
        [color] => Some((operand_color(color)?, None)),
        [a, b] => match (percentage(a), percentage(b)) {
            (Some(p), None) => Some((operand_color(b)?, Some(p))),
            (None, Some(p)) => Some((operand_color(a)?, Some(p))),
            _ => None,
        },
        _ => None,
    }
}

/// An absolute operand color. `currentcolor` makes the mix depend on
/// context and is not resolved here.
fn operand_color(input: &str) -> Option<CssColor> {
    if let Some(color) = parse_color(input) {
        return Some(color);
    }
    // The legacy parser stores whole 8-bit channels and `a / 255.f`, as
    // `Color::FromRGBA32` does; Blink keeps the channels as 0..255 floats.
    let legacy = crate::property::legacy_srgb_color(input)?;
    let channel = |value: f32| (value * 255.0).round();
    Some(CssColor {
        space: Space::SrgbLegacy,
        params: [channel(legacy.r), channel(legacy.g), channel(legacy.b)],
        alpha: legacy.a,
        none: [false; 4],
    })
}

/// `CSSColorMixValue::NormalizePercentages`, in double. Returns the mix
/// progress (the second color's share) and the alpha multiplier.
fn normalize_percentages(p1: Option<f64>, p2: Option<f64>) -> (f64, f64) {
    let mut a = p1.map_or(0.5, |p| p.clamp(0.0, 100.0) / 100.0);
    let mut b = p2.map_or(0.5, |p| p.clamp(0.0, 100.0) / 100.0);
    if p1.is_some() && p2.is_none() {
        b = 1.0 - a;
    } else if p2.is_some() && p1.is_none() {
        a = 1.0 - b;
    }
    let mut alpha_multiplier = 1.0;
    let scale = a + b;
    if scale != 0.0 {
        a /= scale;
        b /= scale;
        if scale <= 1.0 {
            alpha_multiplier = scale;
        }
    }
    let progress = if a == 0.0 { 1.0 } else { b };
    (progress, alpha_multiplier)
}

/// `Color::FromColorMix` and `Color::InterpolateColors`.
fn mix(
    color1: CssColor,
    color2: CssColor,
    space: Space,
    hue_method: HueMethod,
    progress: f32,
    alpha_multiplier: f32,
) -> CssColor {
    let mut c1 = convert_for_interpolation(color1, space);
    let mut c2 = convert_for_interpolation(color2, space);
    substitute_missing(&mut c1, &mut c2);
    let alpha1 = premultiply(&mut c1);
    let alpha2 = premultiply(&mut c2);
    let progress = f64::from(progress);
    let mut params = [0.0_f32; 3];
    let mut none = [false; 4];
    for index in 0..3 {
        if c1.none[index] && c2.none[index] {
            none[index] = true;
        } else if space == Space::Oklch && index == 2 {
            params[index] =
                hue_interpolation(c1.params[index], c2.params[index], progress, hue_method);
        } else {
            params[index] = blend(c1.params[index], c2.params[index], progress);
        }
    }
    let alpha_none = c1.none[3] && c2.none[3];
    none[3] = alpha_none;
    let alpha = if alpha_none {
        0.0
    } else {
        blend(alpha1, alpha2, progress)
    };
    let mut result = from_color_space(space, params, alpha, none);
    unpremultiply(&mut result);
    result.alpha *= alpha_multiplier;
    if result.space == Space::SrgbLegacy {
        result.space = Space::Srgb;
    }
    result
}

/// `blink::Blend(float, float, double)`: the difference in float, the
/// product and sum in double, narrowed once.
fn blend(from: f32, to: f32, progress: f64) -> f32 {
    (f64::from(from) + f64::from(to - from) * progress) as f32
}

fn hue_interpolation(mut h1: f32, mut h2: f32, progress: f64, method: HueMethod) -> f32 {
    let diff = h2 - h1;
    match method {
        HueMethod::Shorter => {
            if diff > 180.0 {
                h1 += 360.0;
            } else if diff < -180.0 {
                h2 += 360.0;
            }
        }
        HueMethod::Longer => {
            if 0.0 < diff && diff < 180.0 {
                h1 += 360.0;
            } else if -180.0 < diff && diff <= 0.0 {
                h2 += 360.0;
            }
        }
        HueMethod::Increasing => {
            if h2 < h1 {
                h2 += 360.0;
            }
        }
        HueMethod::Decreasing => {
            if h1 < h2 {
                h1 += 360.0;
            }
        }
    }
    angle_to_unit_circle_degrees(blend(h1, h2, progress))
}

fn angle_to_unit_circle_degrees(angle: f32) -> f32 {
    (angle % 360.0 + 360.0) % 360.0
}

/// `SubstituteMissingParameters`: a `none` channel takes the other color's
/// value when only one side is missing.
fn substitute_missing(c1: &mut CssColor, c2: &mut CssColor) {
    for index in 0..4 {
        let value1 = if index == 3 {
            c1.alpha
        } else {
            c1.params[index]
        };
        let value2 = if index == 3 {
            c2.alpha
        } else {
            c2.params[index]
        };
        if c1.none[index] && !c2.none[index] {
            set_channel(c1, index, value2);
        } else if c2.none[index] && !c1.none[index] {
            set_channel(c2, index, value1);
        }
    }
}

fn set_channel(color: &mut CssColor, index: usize, value: f32) {
    if index == 3 {
        color.alpha = value;
    } else {
        color.params[index] = value;
    }
    color.none[index] = false;
}

/// `PremultiplyColor`: hue is not premultiplied; returns the original alpha.
fn premultiply(color: &mut CssColor) -> f32 {
    let alpha = color.alpha;
    if color.none[3] {
        return alpha;
    }
    for index in 0..3 {
        if !(color.space == Space::Oklch && index == 2) {
            color.params[index] *= alpha;
        }
    }
    color.alpha = 1.0;
    alpha
}

fn unpremultiply(color: &mut CssColor) {
    if color.none[3] || color.alpha == 0.0 {
        return;
    }
    for index in 0..3 {
        if !(color.space == Space::Oklch && index == 2) {
            color.params[index] /= color.alpha;
        }
    }
}

/// `ConvertToColorSpaceForInterpolation`: record analogous missing
/// components, convert (with `none` read as 0) and mark the carried ones
/// missing again. A color already in the space keeps its `none` bits.
fn convert_for_interpolation(color: CssColor, space: Space) -> CssColor {
    if color.space == space {
        return color;
    }
    let carried = analogous_missing(&color, space);
    let mut converted = convert(color, space);
    for (index, carry) in carried.into_iter().enumerate() {
        if carry {
            set_channel(&mut converted, index, 0.0);
            converted.none[index] = true;
        }
    }
    converted
}

/// `GetAnalogousMissingComponents` for the supported spaces: RGB channels
/// carry between RGB-like spaces and lightness between oklab and oklch;
/// alpha always carries.
fn analogous_missing(color: &CssColor, space: Space) -> [bool; 4] {
    let mut carried = [false, false, false, color.none[3]];
    if color.space.is_rgb_like() && space.is_rgb_like() {
        carried[..3].copy_from_slice(&color.none[..3]);
    } else if matches!(
        (color.space, space),
        (Space::Oklab, Space::Oklch) | (Space::Oklch, Space::Oklab)
    ) {
        carried[0] = color.none[0];
    }
    carried
}

/// `Color::ConvertToColorSpace`. Missing channels convert as 0. Converting
/// into oklch marks a hue missing when chroma is at most 0.02.
fn convert(color: CssColor, space: Space) -> CssColor {
    let mut params = color.params;
    for (param, none) in params.iter_mut().zip(color.none) {
        if none {
            *param = 0.0;
        }
    }
    let none = [false, false, false, color.none[3]];
    let params = match (color.space, space) {
        (from, to) if from == to => params,
        (_, Space::SrgbLegacy) => unreachable!("nothing converts into the legacy space"),
        (from, Space::Srgb) => to_srgb_params(from, params),
        (Space::Oklch, Space::Oklab) => lch_to_lab(params),
        (from, Space::Oklab) => xyz_d50_to_oklab(to_xyz_d50(from, params)),
        // This branch of Blink does not wrap the hue into [0, 360).
        (Space::Oklab, Space::Oklch) => lab_to_lch(params),
        (from, Space::Oklch) => {
            let [l, c, h] = lab_to_lch(xyz_d50_to_oklab(to_xyz_d50(from, params)));
            [l, c, angle_to_unit_circle_degrees(h)]
        }
    };
    let mut result = from_color_space(space, params, color.alpha, none);
    if space == Space::Oklch
        && color.space != Space::Oklch
        && params[1] <= ACHROMATIC_CHROMA_THRESHOLD
    {
        result.params[2] = 0.0;
        result.none[2] = true;
    }
    result
}

/// `kAchromaticChromaThreshold` in `color.cc`.
const ACHROMATIC_CHROMA_THRESHOLD: f32 = 0.02;

// ── Conversion math (`ui/gfx/color_conversions.cc`, `skcms_ext.cc`) ──────

type Matrix = [[f32; 3]; 3];

/// `skcms::Matrix3x3_apply`: `m0*v0 + m1*v1 + m2*v2`, left to right.
fn apply(m: &Matrix, v: [f32; 3]) -> [f32; 3] {
    std::array::from_fn(|r| m[r][0] * v[0] + m[r][1] * v[1] + m[r][2] * v[2])
}

/// `skcms_Matrix3x3_concat`.
fn concat(a: &Matrix, b: &Matrix) -> Matrix {
    std::array::from_fn(|r| {
        std::array::from_fn(|c| a[r][0] * b[0][c] + a[r][1] * b[1][c] + a[r][2] * b[2][c])
    })
}

/// `skcms_Matrix3x3_invert`: cofactors and determinant in double, each
/// entry narrowed to float.
fn invert(src: &Matrix) -> Matrix {
    let v = |r: usize, c: usize| f64::from(src[r][c]);
    let (a00, a01, a02) = (v(0, 0), v(1, 0), v(2, 0));
    let (a10, a11, a12) = (v(0, 1), v(1, 1), v(2, 1));
    let (a20, a21, a22) = (v(0, 2), v(1, 2), v(2, 2));
    let mut b0 = a00 * a11 - a01 * a10;
    let mut b1 = a00 * a12 - a02 * a10;
    let mut b2 = a01 * a12 - a02 * a11;
    let (mut b3, mut b4, mut b5) = (a20, a21, a22);
    let determinant = b0 * b5 - b1 * b4 + b2 * b3;
    let invdet = 1.0 / determinant;
    b0 *= invdet;
    b1 *= invdet;
    b2 *= invdet;
    b3 *= invdet;
    b4 *= invdet;
    b5 *= invdet;
    let mut dst = [[0.0_f32; 3]; 3];
    dst[0][0] = (a11 * b5 - a12 * b4) as f32;
    dst[1][0] = (a02 * b4 - a01 * b5) as f32;
    dst[2][0] = b2 as f32;
    dst[0][1] = (a12 * b3 - a10 * b5) as f32;
    dst[1][1] = (a00 * b5 - a02 * b3) as f32;
    dst[2][1] = (-b1) as f32;
    dst[0][2] = (a10 * b4 - a11 * b3) as f32;
    dst[1][2] = (a01 * b3 - a00 * b4) as f32;
    dst[2][2] = b0 as f32;
    dst
}

/// `skcms_TransferFunction` fields g, a, b, c, d, e, f.
#[derive(Debug, Clone, Copy)]
struct TransferFunction {
    g: f32,
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

/// `SkNamedTransferFn::kSRGB`.
const SRGB_TRANSFER: TransferFunction = TransferFunction {
    g: 2.4,
    a: (1.0 / 1.055) as f32,
    b: (0.055 / 1.055) as f32,
    c: (1.0 / 12.92) as f32,
    d: 0.04045,
    e: 0.0,
    f: 0.0,
};

/// Chromium's own `TransferFunction_eval` (`skia/ext/skcms_ext.cc`): odd
/// extension below zero, plain `powf`.
fn transfer_eval(tf: &TransferFunction, x: f32) -> f32 {
    let sign = if x < 0.0 { -1.0_f32 } else { 1.0 };
    let x = x.abs();
    if x <= tf.d {
        sign * (tf.c * x + tf.f)
    } else {
        sign * ((tf.a * x + tf.b).max(0.0).powf(tf.g) + tf.e)
    }
}

/// Chromium's own `TransferFunction_invert`.
fn transfer_invert(src: &TransferFunction) -> TransferFunction {
    let mut inv = TransferFunction {
        g: 0.0,
        a: 0.0,
        b: 0.0,
        c: 0.0,
        d: 0.0,
        e: 0.0,
        f: 0.0,
    };
    inv.d = src.c * src.d + src.f;
    if inv.d > 0.0 {
        inv.c = 1.0 / src.c;
        inv.f = -src.f / src.c;
    }
    let k = src.a.powf(-src.g);
    inv.g = 1.0 / src.g;
    inv.a = k;
    inv.b = -k * src.e;
    inv.e = -src.b / src.a;
    if inv.a * inv.d + inv.b < 0.0 {
        inv.b = -inv.a * inv.d;
    }
    let s = transfer_eval(src, 1.0);
    let sign = if s < 0.0 { -1.0_f32 } else { 1.0 };
    let s = s * sign;
    if s < inv.d {
        inv.f = 1.0 - sign * (inv.c * s);
    } else {
        inv.e = 1.0 - sign * (inv.a * s + inv.b).powf(inv.g);
    }
    inv
}

/// `kOklab_to_LMS` in `color_conversions.cc`.
const OKLAB_TO_LMS: Matrix = [
    [
        0.999_999_998_450_519_814_32,
        0.396_337_792_173_767_856_78,
        0.215_803_758_060_758_803_39,
    ],
    [
        1.000_000_008_881_760_776_7,
        -0.105_561_342_323_656_349_4,
        -0.063_854_174_771_705_903_402,
    ],
    [
        1.000_000_054_672_410_917_7,
        -0.089_484_182_094_965_759_684,
        -1.291_485_537_864_091_739_9,
    ],
];

/// `kXYZD65_to_LMS` in `color_conversions.cc`.
const XYZ_D65_TO_LMS: Matrix = [
    [
        0.819_022_443_216_431_9,
        0.361_906_256_280_122_1,
        -0.128_873_782_612_164_14,
    ],
    [
        0.032_983_667_198_027_1,
        0.929_286_846_896_554_6,
        0.036_144_668_169_998_44,
    ],
    [
        0.048_177_199_566_046_255,
        0.264_239_524_944_227_64,
        0.633_547_825_813_693_7,
    ],
];

/// `SkNamedGamut::kSRGB`: the ICC 16.16 fixed-point D50 matrix.
const SRGB_TO_XYZ_D50: Matrix = {
    const fn fixed(value: i32) -> f32 {
        value as f32 * 1.525_878_906_25e-5
    }
    [
        [fixed(0x6FA2), fixed(0x6299), fixed(0x24A0)],
        [fixed(0x38F5), fixed(0xB785), fixed(0x0F84)],
        [fixed(0x0390), fixed(0x18DA), fixed(0xB6CF)],
    ]
};

/// `skcms_AdaptToXYZD50(0.3127, 0.3290)`: Bradford adaptation in float.
fn adapt_to_xyz_d50(wx: f32, wy: f32) -> Matrix {
    let w_xyz = [wx / wy, 1.0, (1.0 - wx - wy) / wy];
    let w_d50 = [0.96422_f32, 1.0, 0.82521];
    const XYZ_TO_LMS: Matrix = [
        [0.8951, 0.2664, -0.1614],
        [-0.7502, 1.7135, 0.0367],
        [0.0389, -0.0685, 1.0296],
    ];
    const LMS_TO_XYZ: Matrix = [
        [0.986_992_9, -0.147_054_3, 0.159_962_7],
        [0.432_305_3, 0.518_360_3, 0.049_291_2],
        [-0.008_528_7, 0.040_042_8, 0.968_486_7],
    ];
    let src = apply(&XYZ_TO_LMS, w_xyz);
    let dst = apply(&XYZ_TO_LMS, w_d50);
    let diagonal = [
        [dst[0] / src[0], 0.0, 0.0],
        [0.0, dst[1] / src[1], 0.0],
        [0.0, 0.0, dst[2] / src[2]],
    ];
    concat(&LMS_TO_XYZ, &concat(&diagonal, &XYZ_TO_LMS))
}

struct Conversions {
    lms_to_xyz_d65: Matrix,
    lms_to_oklab: Matrix,
    d65_to_d50: Matrix,
    d50_to_d65: Matrix,
    xyz_d50_to_srgb_linear: Matrix,
    srgb_encode: TransferFunction,
}

static CONVERSIONS: LazyLock<Conversions> = LazyLock::new(|| {
    let d65_to_d50 = adapt_to_xyz_d50(0.3127, 0.3290);
    Conversions {
        lms_to_xyz_d65: invert(&XYZ_D65_TO_LMS),
        lms_to_oklab: invert(&OKLAB_TO_LMS),
        d50_to_d65: invert(&d65_to_d50),
        d65_to_d50,
        xyz_d50_to_srgb_linear: invert(&SRGB_TO_XYZ_D50),
        srgb_encode: transfer_invert(&SRGB_TRANSFER),
    }
});

/// `gfx::LchToLab`: degrees to radians as `h * pi_f / 180` in float, then
/// `cosf` and `sinf`.
fn lch_to_lab([l, c, h]: [f32; 3]) -> [f32; 3] {
    let radians = h * std::f32::consts::PI / 180.0;
    [l, c * radians.cos(), c * radians.sin()]
}

/// `gfx::LabToLch`: `sqrtf`, then `atan2f` and `RadToDeg` as
/// `rad * 180 / pi_f` in float. The caller decides whether to wrap the hue.
fn lab_to_lch([l, a, b]: [f32; 3]) -> [f32; 3] {
    let chroma = (a * a + b * b).sqrt();
    let hue = b.atan2(a) * 180.0 / std::f32::consts::PI;
    [l, chroma, hue]
}

/// `gfx::OklabToXYZD50` without gamut mapping.
fn oklab_to_xyz_d50(lab: [f32; 3]) -> [f32; 3] {
    let lms = apply(&OKLAB_TO_LMS, lab);
    let cubed = lms.map(|x| x * x * x);
    let xyz_d65 = apply(&CONVERSIONS.lms_to_xyz_d65, cubed);
    apply(&CONVERSIONS.d65_to_d50, xyz_d65)
}

/// `gfx::XYZD50ToSRGB`: unclamped, sign-extended encoding.
fn xyz_d50_to_srgb(xyz: [f32; 3]) -> [f32; 3] {
    let linear = apply(&CONVERSIONS.xyz_d50_to_srgb_linear, xyz);
    linear.map(|channel| transfer_eval(&CONVERSIONS.srgb_encode, channel))
}

/// `gfx::SRGBLegacyToSRGB`: divide in double, narrow to float.
fn legacy_to_srgb(params: [f32; 3]) -> [f32; 3] {
    params.map(|channel| (f64::from(channel) / 255.0) as f32)
}

/// `Color::ToXYZD50`.
fn to_xyz_d50(space: Space, params: [f32; 3]) -> [f32; 3] {
    match space {
        Space::SrgbLegacy => srgb_to_xyz_d50(legacy_to_srgb(params)),
        Space::Srgb => srgb_to_xyz_d50(params),
        Space::Oklab => oklab_to_xyz_d50(params),
        Space::Oklch => oklab_to_xyz_d50(lch_to_lab(params)),
    }
}

/// `gfx::SRGBToXYZD50`: Chromium's accurate decode, then the D50 matrix.
fn srgb_to_xyz_d50(srgb: [f32; 3]) -> [f32; 3] {
    apply(
        &SRGB_TO_XYZ_D50,
        srgb.map(|channel| transfer_eval(&SRGB_TRANSFER, channel)),
    )
}

/// `powExt(x, 1.0f / 3.0f)`: a float `powf`, not `cbrt`.
fn pow_ext(x: f32, power: f32) -> f32 {
    if x < 0.0 {
        -(-x).powf(power)
    } else {
        x.powf(power)
    }
}

/// `gfx::XYZD50ToOklab`; both inverses are computed at run time from the
/// forward matrices, as `skcms::Matrix3x3_apply_inverse` does.
fn xyz_d50_to_oklab(xyz: [f32; 3]) -> [f32; 3] {
    let xyz_d65 = apply(&CONVERSIONS.d50_to_d65, xyz);
    let lms_cubed = apply(&XYZ_D65_TO_LMS, xyz_d65);
    let lms = lms_cubed.map(|x| pow_ext(x, 1.0 / 3.0));
    apply(&CONVERSIONS.lms_to_oklab, lms)
}

/// `Color::ToSRGB` without gamut mapping, as float channels.
fn to_srgb_params(space: Space, params: [f32; 3]) -> [f32; 3] {
    match space {
        Space::SrgbLegacy => legacy_to_srgb(params),
        Space::Srgb => params,
        Space::Oklab | Space::Oklch => xyz_d50_to_srgb(to_xyz_d50(space, params)),
    }
}

/// `Color::ToSRGB` for painting (`toSkColor4f`, gamut mapping disabled).
fn to_srgb(color: &CssColor) -> [f32; 3] {
    let mut params = color.params;
    for (param, none) in params.iter_mut().zip(color.none) {
        if none {
            *param = 0.0;
        }
    }
    to_srgb_params(color.space, params)
}

#[cfg(test)]
#[path = "color_function_tests.rs"]
mod tests;
