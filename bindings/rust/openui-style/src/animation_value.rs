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

/// An owned legacy color contribution whose `currentcolor` term is resolved
/// only after the ordered color effect stack is complete. The existing
/// [`AnimationColor`] representation and public [`Color`] stay unchanged.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationColorExpression {
    color: AnimationColor,
    current_color_fraction: f64,
}

impl AnimationColorExpression {
    pub fn from_color(color: Color) -> Self {
        Self::from_animation_color(AnimationColor::from_color(color))
    }

    pub fn from_animation_color(color: AnimationColor) -> Self {
        Self {
            color,
            current_color_fraction: 0.0,
        }
    }

    pub fn current_color() -> Self {
        Self {
            color: AnimationColor {
                channels: [0.0; 3],
                alpha: 0.0,
            },
            current_color_fraction: 1.0,
        }
    }

    pub fn from_style_color(color: StyleColor) -> Self {
        match color {
            StyleColor::Resolved(color) => Self::from_color(color),
            StyleColor::CurrentColor => Self::current_color(),
        }
    }

    pub fn interpolate(self, to: Self, progress: f64) -> Self {
        Self {
            color: self.color.interpolate(to.color, progress),
            current_color_fraction: blend(
                self.current_color_fraction,
                to.current_color_fraction,
                progress,
            ),
        }
    }

    pub fn compose(self, effect: Self, underlying_fraction: f64) -> Self {
        Self {
            color: self.color.compose(effect.color, underlying_fraction),
            current_color_fraction: self.current_color_fraction * underlying_fraction
                + effect.current_color_fraction,
        }
    }

    pub fn depends_on_current_color(self) -> bool {
        self.current_color_fraction != 0.0
    }

    pub fn is_finite(self) -> bool {
        self.color.channels.iter().all(|v| v.is_finite())
            && self.color.alpha.is_finite()
            && self.current_color_fraction.is_finite()
    }

    pub fn resolve(self, current_color: Color) -> Color {
        if self.current_color_fraction == 0.0 {
            return self.color.resolve();
        }
        // .50 AddPremultipliedColor: keyword fraction * float parameter *
        // float alpha, each promoted to double. This is deliberately distinct
        // from Create's float parameter * float alpha endpoint capture.
        let parameter = |channel: f32| (channel as f64 * 255.0) as f32;
        let fraction = self.current_color_fraction;
        let alpha = current_color.a as f64;
        AnimationColor {
            channels: std::array::from_fn(|i| {
                self.color.channels[i]
                    + fraction
                        * parameter([current_color.r, current_color.g, current_color.b][i]) as f64
                        * alpha
            }),
            alpha: self.color.alpha + fraction * alpha,
        }
        .resolve()
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
    fn radius_legacy_percentage_flag(self, percent: bool) -> Self {
        // The existing computed override clears raw sources and reinterprets
        // the legacy numeric projection. Calc's projection is its percentage
        // coefficient, matching ordinary property publication.
        let numeric = if self.kind == LengthType::Fixed {
            self.pixels
        } else {
            self.percent
        };
        Self {
            pixels: if percent { 0.0 } else { numeric },
            percent: if percent { numeric } else { 0.0 },
            kind: if percent {
                LengthType::Percent
            } else {
                LengthType::Fixed
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
    /// A color with a retained dependent keyword contribution.
    ColorExpression {
        value: AnimationColorExpression,
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
            StyleValue::Typography(TypographyValue::StyleColor(StyleColor::CurrentColor)) => {
                Self::ColorExpression {
                    value: AnimationColorExpression::current_color(),
                    kind: AnimationColorKind::ResolvedStyleColor,
                }
            }
            StyleValue::ColorExpression(value) => Self::ColorExpression {
                value,
                kind: AnimationColorKind::Color,
            },
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
    /// Preserve the schema's author representation while canonicalizing all
    /// accepted Color/StyleColor/renderer endpoints into one working color kind.
    pub fn from_style_value_for_property(property: StyleProperty, value: StyleValue) -> Self {
        let mut value = Self::from_style_value(value);
        let kind = if property.metadata().value_kind == crate::ValueKind::Color {
            AnimationColorKind::Color
        } else {
            AnimationColorKind::ResolvedStyleColor
        };
        match &mut value {
            Self::Color { kind: actual, .. } | Self::ColorExpression { kind: actual, .. } => {
                *actual = kind
            }
            _ => {}
        }
        value
    }

    fn color_expression(&self) -> Option<(AnimationColorExpression, AnimationColorKind)> {
        match self {
            Self::Color { value, kind } => Some((
                AnimationColorExpression::from_animation_color(*value),
                *kind,
            )),
            Self::ColorExpression { value, kind } => Some((*value, *kind)),
            _ => None,
        }
    }

    /// Resolve dependent keyword terms against the final cascade color. Own
    /// `color: currentcolor` endpoints must instead be captured from the parent
    /// before sampling, as CSSColorInterpolationType::MaybeConvertInherit does.
    pub fn into_style_value_with_color_context(
        self,
        property: StyleProperty,
        current_color: Color,
    ) -> StyleValue {
        match self {
            Self::ColorExpression {
                value,
                kind: AnimationColorKind::Color,
            } => StyleValue::Color(value.resolve(current_color)),
            Self::ColorExpression {
                value,
                kind: AnimationColorKind::ResolvedStyleColor,
            } => StyleValue::Typography(TypographyValue::StyleColor(StyleColor::Resolved(
                value.resolve(current_color),
            ))),
            other => other.into_style_value(property),
        }
    }

    /// Numeric operand for the existing discrete renderer corner transport.
    /// Preserve its accepted source value; new native axes clamp separately.
    #[doc(hidden)]
    pub fn radius_legacy_numeric(&self) -> Option<f32> {
        match self {
            Self::Length(value) => Some(if value.kind == LengthType::Fixed {
                value.pixels as f32
            } else {
                value.percent as f32
            }),
            _ => None,
        }
    }

    /// Legacy corner pair setters keep their existing percentage flags when
    /// replacing raw sources. Calc projects a non-percent legacy flag.
    #[doc(hidden)]
    pub fn radius_percentage_flag(&self) -> Option<bool> {
        match self {
            Self::Length(value) => Some(value.kind == LengthType::Percent),
            _ => None,
        }
    }

    /// Apply only the legacy native percentage overlay to an axis working
    /// value. Its f64 numeric projection survives lower-effect composition.
    #[doc(hidden)]
    pub fn with_radius_percentage_flag(&self, percent: bool) -> Option<Self> {
        match self {
            Self::Length(value) => Some(Self::Length(value.radius_legacy_percentage_flag(percent))),
            _ => None,
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
            Self::ColorExpression { value, .. } => StyleValue::ColorExpression(value),
            Self::Length(value) => {
                let mut length = value.resolve();
                if property.radius_axis_coordinates().is_some() {
                    if let LengthValue::Computed(value) = length {
                        // Nonnegative radius range clamps at final publication,
                        // never between Add/Accumulate effects. Calc retains its
                        // coefficients and clamps against the actual box later.
                        length = if value.is_fixed() {
                            LengthValue::px(value.value().max(0.0))
                        } else if value.is_percent() {
                            LengthValue::percent(value.value().max(0.0))
                        } else {
                            length
                        };
                    }
                }
                StyleValue::Length(length)
            }
            Self::Edges(value) => StyleValue::Edges(edges(value)),
            Self::CornerRadii(value) => StyleValue::CornerRadii(crate::CornerRadii(edges(value))),
            Self::Gap(value) => StyleValue::Gap(Gap {
                row: value[0].resolve(),
                column: value[1].resolve(),
            }),
        }
    }
    pub fn interpolate(&self, to: &Self, progress: f64) -> Option<Self> {
        if matches!(self, Self::ColorExpression { .. })
            || matches!(to, Self::ColorExpression { .. })
        {
            if let (Some((from, kind)), Some((to, to_kind))) =
                (self.color_expression(), to.color_expression())
            {
                if kind == to_kind {
                    return Some(Self::ColorExpression {
                        value: from.interpolate(to, progress),
                        kind,
                    });
                }
            }
        }
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
        if matches!(self, Self::ColorExpression { .. })
            || matches!(underlying, Self::ColorExpression { .. })
        {
            if let (Some((effect, kind)), Some((base, base_kind))) =
                (self.color_expression(), underlying.color_expression())
            {
                if kind == base_kind {
                    return Self::ColorExpression {
                        value: base.compose(effect, 1.0),
                        kind,
                    };
                }
            }
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
    fn currentcolor_interpolation_retains_a_fraction_until_final_color_resolution() {
        let sampled = AnimationColorExpression::current_color()
            .interpolate(AnimationColorExpression::from_color(Color::BLUE), 0.5);
        assert!(sampled.depends_on_current_color());
        assert_eq!(
            sampled.resolve(Color::RED),
            Color::from_rgba_f32(0.5, 0.0, 0.5, 1.0)
        );
        assert_eq!(
            sampled.resolve(Color::BLACK),
            Color::from_rgba_f32(0.0, 0.0, 0.5, 1.0)
        );
    }

    #[test]
    fn additive_currentcolor_terms_keep_keyword_alpha_outside_the_numeric_equal_alpha_branch() {
        let keyword = AnimationColorExpression::current_color();
        let doubled = keyword.compose(keyword, 1.0);
        // Two unresolved keyword fractions contribute twice the final alpha.
        // Resolving each first would hit Composite's equal-alpha branch and
        // incorrectly preserve 0.25 while doubling straight red.
        assert_eq!(
            doubled.resolve(Color::from_rgba_f32(1.0, 0.0, 0.0, 0.25)),
            Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5)
        );
        let numeric = AnimationColor::from_color(Color::from_rgba_f32(1.0, 0.0, 0.0, 0.25));
        assert_eq!(
            numeric.compose(numeric, 1.0).resolve(),
            Color::from_rgba_f32(2.0, 0.0, 0.0, 0.25)
        );
    }

    #[test]
    fn color_expression_round_trip_keeps_context_free_property_sampling_owned() {
        let keyframes = crate::PropertyKeyframes::typed(
            StyleProperty::BackgroundColor,
            crate::Keyframes::from_values(
                StyleColor::CurrentColor,
                StyleColor::Resolved(Color::BLUE),
            ),
        )
        .unwrap();
        let StyleValue::ColorExpression(sampled) = keyframes.sample(0.5, &crate::Easing::Linear)
        else {
            panic!("dependent currentcolor was resolved before a color context existed")
        };
        assert_eq!(
            sampled.resolve(Color::RED),
            Color::from_rgba_f32(0.5, 0.0, 0.5, 1.0)
        );
        let composed = CompositedStyleValue::from_style_value_for_property(
            StyleProperty::BackgroundColor,
            StyleValue::ColorExpression(sampled),
        );
        assert_eq!(
            composed
                .into_style_value_with_color_context(StyleProperty::BackgroundColor, Color::RED),
            StyleValue::Color(Color::from_rgba_f32(0.5, 0.0, 0.5, 1.0))
        );
    }

    #[test]
    fn own_currentcolor_captures_the_parent_before_premultiplying_endpoints() {
        let value: StyleValue = StyleColor::CurrentColor.into();
        let parent = Color::from_rgba_f32(0.1, 0.2, 0.3, 0.1);
        assert_eq!(
            value.capture_own_current_color(StyleProperty::Color, parent),
            StyleValue::Color(parent)
        );
        assert_eq!(
            value.capture_own_current_color(StyleProperty::BackgroundColor, parent),
            value
        );
        let color = AnimationColor::from_color(parent);
        // Captured numeric endpoints preserve Create's f32 product. The later
        // dependent-keyword contribution uses double multiplication instead.
        assert_eq!(color.interpolate(color, 0.5).resolve(), parent);
        let parameter = (parent.r as f64 * 255.0) as f32;
        assert_ne!(color.channels[0], parameter as f64 * parent.a as f64);
    }

    #[test]
    fn invalid_or_wrong_property_expressions_are_rejected_without_mutation() {
        let mut style = crate::ComputedStyle::initial();
        let before = style.color;
        let invalid = StyleValue::ColorExpression(AnimationColorExpression {
            color: AnimationColor {
                channels: [f64::NAN, 0.0, 0.0],
                alpha: 1.0,
            },
            current_color_fraction: 0.0,
        });
        assert!(crate::apply_to_computed(
            &mut style,
            StyleProperty::Color,
            &invalid,
            (100.0, 80.0)
        )
        .is_err());
        assert_eq!(style.color, before);
        assert!(crate::apply_to_computed(
            &mut style,
            StyleProperty::Width,
            &StyleValue::ColorExpression(AnimationColorExpression::current_color()),
            (100.0, 80.0)
        )
        .is_err());
        assert!(style.width.is_auto());
    }

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
