//! Working animation values, retained until the ordered effect stack is complete.
//! Arithmetic follows pinned Chromium 147.0.7727.50 InterpolableNumber,
//! InterpolableColor and NumberPropertyFunctions, without changing public Color.
use crate::{
    Color, CompositeOperation, Edges, FontWeight, Gap, InterpolationKind, LengthValue, StyleColor,
    StyleProperty, StyleValue, TransformList, TypographyValue,
};
use openui_geometry::{Length, LengthType};

fn blend(a: f64, b: f64, progress: f64) -> f64 {
    // InterpolableNumber::Interpolate has these exact endpoint/equality guards.
    if progress == 0.0 || a == b {
        a
    } else if progress == 1.0 {
        b
    } else {
        a * (1.0 - progress) + b * progress
    }
}

/// Premultiplied legacy sRGB channels in 0..255 parameter units. Native Color stores
/// straight f32 sRGB; working channels retain double precision across effects.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationColor {
    channels: [f64; 3],
    alpha: f64,
}
impl AnimationColor {
    pub fn from_color(color: Color) -> Self {
        // Legacy Color parameters use 0..255 units. The pinned SkColor4f
        // constructor multiplies float RGB by double 255 before storing float.
        // Create then multiplies that float parameter by float alpha, assigning
        // the rounded float product to the double working channel.
        let parameter = |channel: f32| (channel as f64 * 255.0) as f32;
        Self {
            channels: [
                (parameter(color.r) * color.a) as f64,
                (parameter(color.g) * color.a) as f64,
                (parameter(color.b) * color.a) as f64,
            ],
            alpha: color.a as f64,
        }
    }
    pub fn interpolate(self, to: Self, progress: f64) -> Self {
        Self {
            channels: std::array::from_fn(|i| blend(self.channels[i], to.channels[i], progress)),
            alpha: blend(self.alpha, to.alpha, progress),
        }
    }
    pub fn compose(self, effect: Self, underlying_fraction: f64) -> Self {
        // .50 InterpolableColor::Composite preserves equal alpha, including
        // equal translucent alpha. It is not generic four-vector addition.
        Self {
            channels: std::array::from_fn(|i| {
                self.channels[i] * underlying_fraction + effect.channels[i]
            }),
            alpha: if self.alpha == effect.alpha {
                self.alpha
            } else {
                self.alpha * underlying_fraction + effect.alpha
            },
        }
    }
    pub fn resolve(self) -> Color {
        // CSSColorInterpolationType::ApplyStandardPropertyValue takes Resolve,
        // whose alpha clamp precedes unpremultiplication (not GetColor's order).
        let alpha = self.alpha.clamp(0.0, 1.0);
        let rgb = if alpha == 0.0 {
            self.channels
        } else {
            self.channels.map(|v| v / alpha)
        };
        // Resolve constructs float legacy parameters. ToSkColor4f then calls
        // gfx::SRGBLegacyToSRGB: float parameter / double 255, stored as float.
        // Keep both conversions; folding the scale changes float rounding.
        let normalized = rgb.map(|parameter| (parameter as f32 as f64 / 255.0) as f32);
        Color::from_rgba_f32(normalized[0], normalized[1], normalized[2], alpha as f32)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationNumberKind {
    Number,
    Integer,
    FontWeight,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnimationColorKind {
    Color,
    ResolvedStyleColor,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationLength {
    percent: f64,
    pixels: f64,
    kind: LengthType,
}
impl AnimationLength {
    fn from_length(value: LengthValue) -> Option<Self> {
        let LengthValue::Computed(value) = value else {
            return None;
        };
        Some(match value.length_type() {
            LengthType::Fixed => Self {
                percent: 0.0,
                pixels: value.value() as f64,
                kind: LengthType::Fixed,
            },
            LengthType::Percent => Self {
                percent: value.value() as f64,
                pixels: 0.0,
                kind: LengthType::Percent,
            },
            LengthType::Calculated => Self {
                percent: value.value() as f64,
                pixels: value.calc_offset() as f64,
                kind: LengthType::Calculated,
            },
            _ => return None,
        })
    }
    fn interpolate(self, to: Self, progress: f64) -> Self {
        Self {
            percent: blend(self.percent, to.percent, progress),
            pixels: blend(self.pixels, to.pixels, progress),
            kind: if self.kind == to.kind {
                self.kind
            } else {
                LengthType::Calculated
            },
        }
    }
    fn add(self, to: Self) -> Self {
        Self {
            percent: self.percent + to.percent,
            pixels: self.pixels + to.pixels,
            kind: if self.kind == to.kind {
                self.kind
            } else {
                LengthType::Calculated
            },
        }
    }
    fn resolve(self) -> LengthValue {
        LengthValue::Computed(match self.kind {
            LengthType::Fixed => Length::px(self.pixels as f32),
            LengthType::Percent => Length::percent(self.percent as f32),
            _ => Length::calc_percent_px(self.percent as f32, self.pixels as f32),
        })
    }
}

/// Typed intermediate value for one property. Opaque/discrete styles retain
/// their ordinary representation; numbers, lengths and colors remain working
/// values until all lower and upper effects have been composed.
#[derive(Debug, Clone, PartialEq)]
pub enum CompositedStyleValue {
    Style(StyleValue),
    Number {
        value: f64,
        kind: AnimationNumberKind,
    },
    Color {
        value: AnimationColor,
        kind: AnimationColorKind,
    },
    Length(AnimationLength),
    Edges([AnimationLength; 4]),
    CornerRadii([AnimationLength; 4]),
    Gap([AnimationLength; 2]),
}
impl CompositedStyleValue {
    pub fn from_style_value(value: StyleValue) -> Self {
        if let StyleValue::Renderer(renderer) = &value {
            if let Some(value) = renderer.animation_value() {
                return value;
            }
        }
        let lengths = |edges: Edges<LengthValue>| {
            Some([
                AnimationLength::from_length(edges.top)?,
                AnimationLength::from_length(edges.right)?,
                AnimationLength::from_length(edges.bottom)?,
                AnimationLength::from_length(edges.left)?,
            ])
        };
        match value {
            StyleValue::Number(value) => Self::Number {
                value: value as f64,
                kind: AnimationNumberKind::Number,
            },
            StyleValue::Integer(value) => Self::Number {
                value: value as f64,
                kind: AnimationNumberKind::Integer,
            },
            StyleValue::FontWeight(value) => Self::Number {
                value: value.0 as f64,
                kind: AnimationNumberKind::FontWeight,
            },
            StyleValue::Color(value) => Self::Color {
                value: AnimationColor::from_color(value),
                kind: AnimationColorKind::Color,
            },
            StyleValue::Typography(TypographyValue::StyleColor(StyleColor::Resolved(value))) => {
                Self::Color {
                    value: AnimationColor::from_color(value),
                    kind: AnimationColorKind::ResolvedStyleColor,
                }
            }
            StyleValue::Length(value) => AnimationLength::from_length(value)
                .map(Self::Length)
                .unwrap_or(Self::Style(StyleValue::Length(value))),
            StyleValue::Edges(value) => lengths(value)
                .map(Self::Edges)
                .unwrap_or(Self::Style(StyleValue::Edges(value))),
            StyleValue::CornerRadii(value) => lengths(value.0)
                .map(Self::CornerRadii)
                .unwrap_or(Self::Style(StyleValue::CornerRadii(value))),
            StyleValue::Gap(value) => AnimationLength::from_length(value.row)
                .zip(AnimationLength::from_length(value.column))
                .map(|(a, b)| Self::Gap([a, b]))
                .unwrap_or(Self::Style(StyleValue::Gap(value))),
            other => Self::Style(other),
        }
    }
    pub fn into_style_value(self, property: StyleProperty) -> StyleValue {
        let edges = |values: [AnimationLength; 4]| Edges {
            top: values[0].resolve(),
            right: values[1].resolve(),
            bottom: values[2].resolve(),
            left: values[3].resolve(),
        };
        match self {
            Self::Style(value) => value,
            Self::Number {
                value,
                kind: AnimationNumberKind::Number,
            } => StyleValue::Number(value.clamp(-(f32::MAX as f64), f32::MAX as f64) as f32),
            Self::Number {
                value,
                kind: AnimationNumberKind::FontWeight,
            } => StyleValue::FontWeight(FontWeight(value as f32)),
            Self::Number {
                value,
                kind: AnimationNumberKind::Integer,
            } => {
                // The standard integer number path uses positive-infinity
                // half rounding for these signed properties, and std::round
                // for column-count/orphans/widows.
                let value = match property.metadata().css_name {
                    "z-index" | "order" | "reading-order" | "math-depth" => (value + 0.5).floor(),
                    "orphans" | "widows" => value.round().clamp(1.0, i16::MAX as f64),
                    "column-count" => value.round().clamp(1.0, u16::MAX as f64),
                    _ => value.round(),
                };
                StyleValue::Integer(value as i32)
            }
            Self::Color {
                value,
                kind: AnimationColorKind::Color,
            } => StyleValue::Color(value.resolve()),
            Self::Color {
                value,
                kind: AnimationColorKind::ResolvedStyleColor,
            } => StyleValue::Typography(TypographyValue::StyleColor(StyleColor::Resolved(
                value.resolve(),
            ))),
            Self::Length(value) => StyleValue::Length(value.resolve()),
            Self::Edges(value) => StyleValue::Edges(edges(value)),
            Self::CornerRadii(value) => StyleValue::CornerRadii(crate::CornerRadii(edges(value))),
            Self::Gap(value) => StyleValue::Gap(Gap {
                row: value[0].resolve(),
                column: value[1].resolve(),
            }),
        }
    }
    pub fn interpolate(&self, to: &Self, progress: f64) -> Option<Self> {
        Some(match (self, to) {
            (Self::Number { value: a, kind: ak }, Self::Number { value: b, kind: bk })
                if ak == bk =>
            {
                Self::Number {
                    value: blend(*a, *b, progress),
                    kind: *ak,
                }
            }
            (Self::Color { value: a, kind: ak }, Self::Color { value: b, kind: bk })
                if ak == bk =>
            {
                Self::Color {
                    value: a.interpolate(*b, progress),
                    kind: *ak,
                }
            }
            (Self::Length(a), Self::Length(b)) => Self::Length(a.interpolate(*b, progress)),
            (Self::Edges(a), Self::Edges(b)) => {
                Self::Edges(std::array::from_fn(|i| a[i].interpolate(b[i], progress)))
            }
            (Self::CornerRadii(a), Self::CornerRadii(b)) => {
                Self::CornerRadii(std::array::from_fn(|i| a[i].interpolate(b[i], progress)))
            }
            (Self::Gap(a), Self::Gap(b)) => {
                Self::Gap(std::array::from_fn(|i| a[i].interpolate(b[i], progress)))
            }
            _ => return None,
        })
    }
    pub fn compose(
        self,
        property: StyleProperty,
        underlying: &Self,
        operation: CompositeOperation,
    ) -> Self {
        if operation == CompositeOperation::Replace
            || property.metadata().interpolation == InterpolationKind::Discrete
        {
            return self;
        }
        match (&self, underlying) {
            (Self::Number { value: b, kind: bk }, Self::Number { value: a, kind: ak })
                if ak == bk =>
            {
                Self::Number {
                    value: a + b,
                    kind: *ak,
                }
            }
            (Self::Color { value: b, kind: bk }, Self::Color { value: a, kind: ak })
                if ak == bk =>
            {
                Self::Color {
                    value: a.compose(*b, 1.0),
                    kind: *ak,
                }
            }
            (Self::Length(b), Self::Length(a)) => Self::Length(a.add(*b)),
            (Self::Edges(b), Self::Edges(a)) => {
                Self::Edges(std::array::from_fn(|i| a[i].add(b[i])))
            }
            (Self::CornerRadii(b), Self::CornerRadii(a)) => {
                Self::CornerRadii(std::array::from_fn(|i| a[i].add(b[i])))
            }
            (Self::Gap(b), Self::Gap(a)) => Self::Gap(std::array::from_fn(|i| a[i].add(b[i]))),
            (Self::Style(StyleValue::Transform(b)), Self::Style(StyleValue::Transform(a))) => {
                Self::Style(StyleValue::Transform(
                    if operation == CompositeOperation::Add {
                        let mut list = a.0.clone();
                        list.extend(b.0.iter().cloned());
                        TransformList(list)
                    } else {
                        crate::accumulate_transform_lists(a, b)
                    },
                ))
            }
            _ => self,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn legacy_parameter_units_preserve_constant_decimal_color() {
        let color = Color::from_rgba_f32(0.1, 0.2, 0.3, 0.1);
        let value = AnimationColor::from_color(color);
        // Normalized-premultiplied float arithmetic changes the red channel
        // from bits 1036831949 to 1036831950 even for identical endpoints.
        assert_eq!(value.interpolate(value, 0.25).resolve(), color);
        assert_eq!(
            value.interpolate(value, 0.5).resolve().r.to_bits(),
            0.1_f32.to_bits()
        );
    }
    #[test]
    fn color_equal_translucent_alpha_follows_pinned_composite_branch() {
        let red = AnimationColor::from_color(Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5));
        let blue = AnimationColor::from_color(Color::from_rgba_f32(0.0, 0.0, 1.0, 0.5));
        assert_eq!(
            red.compose(blue, 1.0).resolve(),
            Color::from_rgba_f32(1.0, 0.0, 1.0, 0.5)
        );
    }
    #[test]
    fn color_unequal_alpha_is_clamped_only_after_stack() {
        let a = AnimationColor::from_color(Color::from_rgba_f32(1.0, 0.0, 0.0, 0.75));
        let b = AnimationColor::from_color(Color::from_rgba_f32(0.0, 0.0, 1.0, 0.5));
        let c = AnimationColor::from_color(Color::from_rgba_f32(0.0, 1.0, 0.0, 0.25));
        assert_eq!(
            a.compose(b, 1.0).compose(c, 1.0).resolve(),
            Color::from_rgba_f32(0.75, 0.25, 0.5, 1.0)
        );
    }
    #[test]
    fn premultiplied_interpolation_uses_double_then_final_color_float() {
        let opaque = AnimationColor::from_color(Color::RED);
        let transparent = AnimationColor::from_color(Color::from_rgba_f32(0.0, 0.0, 1.0, 0.0));
        assert_eq!(
            opaque.interpolate(transparent, 0.5).resolve(),
            Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
        );
    }
    #[test]
    fn numeric_fraction_survives_large_opposing_effects_until_publication() {
        let base = CompositedStyleValue::from_style_value(StyleValue::Number(0.5));
        let positive = CompositedStyleValue::from_style_value(StyleValue::Number(16_777_216.0));
        let negative = CompositedStyleValue::from_style_value(StyleValue::Number(-16_777_216.0));
        let value = positive.compose(StyleProperty::Opacity, &base, CompositeOperation::Add);
        let value = negative.compose(StyleProperty::Opacity, &value, CompositeOperation::Add);
        assert_eq!(
            value.into_style_value(StyleProperty::Opacity),
            StyleValue::Number(0.5)
        );
    }
    #[test]
    fn signed_integer_half_rounds_towards_positive_infinity_after_stack() {
        let a = CompositedStyleValue::Number {
            value: -0.5,
            kind: AnimationNumberKind::Integer,
        };
        assert_eq!(
            a.into_style_value(StyleProperty::ZIndex),
            StyleValue::Integer(0)
        );
    }
}
