//! Canonical public property schema and typed declaration values.

use crate::{
    BorderStyle, Color, ComputedStyle, ContentAlignment, ContentDistribution, ContentPosition,
    Display, FlexDirection, FlexWrap, FontFamilyList, FontWeight, GenericFontFamily, ItemAlignment,
    ItemPosition, ListStyleType, Overflow, Position, StyleColor, Transform2D,
};
use openui_geometry::{Length, LengthType};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueKind {
    Display,
    Position,
    Overflow,
    Length,
    Edges,
    Color,
    Number,
    Integer,
    FlexDirection,
    FlexWrap,
    ItemAlignment,
    ContentAlignment,
    Gap,
    FontFamily,
    FontWeight,
    Border,
    CornerRadii,
    Cursor,
    ListStyle,
    Transform,
    PointerEvents,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum InvalidationClass {
    Composite,
    Paint,
    Layout,
    Intrinsic,
    Subtree,
    Accessibility,
}

impl InvalidationClass {
    pub fn includes(self, other: Self) -> bool {
        self >= other
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterpolationKind {
    Discrete,
    Number,
    Integer,
    Length,
    LengthList,
    Color,
    Transform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyMetadata {
    pub property: StyleProperty,
    pub css_name: &'static str,
    pub rust_type: &'static str,
    pub value_kind: ValueKind,
    pub initial: &'static str,
    pub inherited: bool,
    pub invalidation: InvalidationClass,
    pub interpolation: InterpolationKind,
}

include!("generated_properties.rs");

/// A length before viewport and font-relative units are resolved for layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LengthValue {
    Computed(Length),
    Em(f32),
    Rem(f32),
    ViewportWidth(f32),
    ViewportHeight(f32),
}

impl LengthValue {
    pub const fn px(value: f32) -> Self {
        Self::Computed(Length::px(value))
    }

    pub const fn percent(value: f32) -> Self {
        Self::Computed(Length::percent(value))
    }

    pub const fn auto() -> Self {
        Self::Computed(Length::auto())
    }

    pub const fn none() -> Self {
        Self::Computed(Length::none())
    }

    pub fn resolve(self, viewport: (f32, f32), font_size: f32, root_font_size: f32) -> Length {
        match self {
            Self::Computed(value) => value,
            Self::Em(value) => Length::px(value * font_size),
            Self::Rem(value) => Length::px(value * root_font_size),
            Self::ViewportWidth(value) => Length::px(viewport.0 * value / 100.0),
            Self::ViewportHeight(value) => Length::px(viewport.1 * value / 100.0),
        }
    }
}

impl From<Length> for LengthValue {
    fn from(value: Length) -> Self {
        Self::Computed(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edges<T> {
    pub top: T,
    pub right: T,
    pub bottom: T,
    pub left: T,
}

impl<T: Copy> Edges<T> {
    pub const fn all(value: T) -> Self {
        Self {
            top: value,
            right: value,
            bottom: value,
            left: value,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Gap {
    pub row: LengthValue,
    pub column: LengthValue,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Border {
    pub width: f32,
    pub style: BorderStyle,
    pub color: Color,
}

impl Border {
    pub const NONE: Self = Self {
        width: 0.0,
        style: BorderStyle::None,
        color: Color::BLACK,
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerRadii(pub Edges<LengthValue>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Cursor {
    Auto,
    Default,
    Pointer,
    Text,
    Move,
    NotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PointerEvents {
    Auto,
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TransformOperation {
    Translate(LengthValue, LengthValue),
    Scale(f32, f32),
    Rotate(f32),
    Matrix(Transform2D),
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct TransformList(pub Vec<TransformOperation>);

#[derive(Debug, Clone, PartialEq)]
pub enum StyleValue {
    Display(Display),
    Position(Position),
    Overflow(Overflow),
    Length(LengthValue),
    Edges(Edges<LengthValue>),
    Color(Color),
    Number(f32),
    Integer(i32),
    FlexDirection(FlexDirection),
    FlexWrap(FlexWrap),
    ItemAlignment(ItemAlignment),
    ContentAlignment(ContentAlignment),
    Gap(Gap),
    FontFamily(FontFamilyList),
    FontWeight(FontWeight),
    Border(Border),
    CornerRadii(CornerRadii),
    Cursor(Cursor),
    ListStyle(ListStyleType),
    Transform(TransformList),
    PointerEvents(PointerEvents),
}

macro_rules! impl_style_value {
    ($ty:ty, $variant:ident) => {
        impl From<$ty> for StyleValue {
            fn from(value: $ty) -> Self {
                Self::$variant(value)
            }
        }
    };
}

impl_style_value!(Display, Display);
impl_style_value!(Position, Position);
impl_style_value!(Overflow, Overflow);
impl_style_value!(LengthValue, Length);
impl_style_value!(Edges<LengthValue>, Edges);
impl_style_value!(Color, Color);
impl_style_value!(f32, Number);
impl_style_value!(i32, Integer);
impl_style_value!(FlexDirection, FlexDirection);
impl_style_value!(FlexWrap, FlexWrap);
impl_style_value!(ItemAlignment, ItemAlignment);
impl_style_value!(ContentAlignment, ContentAlignment);
impl_style_value!(Gap, Gap);
impl_style_value!(FontFamilyList, FontFamily);
impl_style_value!(FontWeight, FontWeight);
impl_style_value!(Border, Border);
impl_style_value!(CornerRadii, CornerRadii);
impl_style_value!(Cursor, Cursor);
impl_style_value!(ListStyleType, ListStyle);
impl_style_value!(TransformList, Transform);
impl_style_value!(PointerEvents, PointerEvents);

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: StyleProperty,
    pub value: StyleValue,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Style {
    declarations: Vec<Declaration>,
}

impl Style {
    pub fn declarations(&self) -> &[Declaration] {
        &self.declarations
    }

    fn with(mut self, property: StyleProperty, value: impl Into<StyleValue>) -> Self {
        self.declarations.push(Declaration {
            property,
            value: value.into(),
        });
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LiteralError(pub String);

impl std::fmt::Display for LiteralError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LiteralError {}

fn invalid(property: StyleProperty, input: &str) -> LiteralError {
    LiteralError(format!(
        "invalid literal `{input}` for `{}` (expected {})",
        property.metadata().css_name,
        property.metadata().rust_type
    ))
}

fn length(input: &str) -> Option<LengthValue> {
    let input = input.trim();
    match input {
        "0" => Some(LengthValue::px(0.0)),
        "auto" => Some(LengthValue::auto()),
        "none" => Some(LengthValue::none()),
        _ => {
            for (suffix, make) in [
                ("px", LengthValue::px as fn(f32) -> LengthValue),
                ("%", LengthValue::percent),
                ("em", LengthValue::Em),
                ("rem", LengthValue::Rem),
                ("vw", LengthValue::ViewportWidth),
                ("vh", LengthValue::ViewportHeight),
            ] {
                if let Some(number) = input
                    .strip_suffix(suffix)
                    .and_then(|v| v.trim().parse().ok())
                {
                    return Some(make(number));
                }
            }
            None
        }
    }
}

fn edges(input: &str) -> Option<Edges<LengthValue>> {
    let values: Vec<_> = input
        .split_whitespace()
        .map(length)
        .collect::<Option<_>>()?;
    match values.as_slice() {
        [a] => Some(Edges::all(*a)),
        [v, h] => Some(Edges {
            top: *v,
            right: *h,
            bottom: *v,
            left: *h,
        }),
        [t, h, b] => Some(Edges {
            top: *t,
            right: *h,
            bottom: *b,
            left: *h,
        }),
        [t, r, b, l] => Some(Edges {
            top: *t,
            right: *r,
            bottom: *b,
            left: *l,
        }),
        _ => None,
    }
}

fn color(input: &str) -> Option<Color> {
    match input.trim().to_ascii_lowercase().as_str() {
        "transparent" => Some(Color::TRANSPARENT),
        "black" => Some(Color::BLACK),
        "white" => Some(Color::WHITE),
        "red" => Some(Color::RED),
        "green" => Some(Color::GREEN),
        "blue" => Some(Color::BLUE),
        value if value.starts_with('#') && (value.len() == 7 || value.len() == 9) => {
            u32::from_str_radix(&value[1..], 16)
                .ok()
                .map(|hex| Color::from_hex(hex, value.len() == 9))
        }
        _ => None,
    }
}

fn border(input: &str) -> Option<Border> {
    if input.trim() == "none" {
        return Some(Border::NONE);
    }
    let mut parts = input.split_whitespace();
    let width = length(parts.next()?)?;
    let LengthValue::Computed(width) = width else {
        return None;
    };
    if width.length_type() != LengthType::Fixed {
        return None;
    }
    let style = match parts.next()? {
        "solid" => BorderStyle::Solid,
        "dotted" => BorderStyle::Dotted,
        "dashed" => BorderStyle::Dashed,
        "double" => BorderStyle::Double,
        _ => return None,
    };
    let color = color(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }
    Some(Border {
        width: width.value(),
        style,
        color,
    })
}

pub fn parse_literal(property: StyleProperty, input: &str) -> Result<StyleValue, LiteralError> {
    use StyleProperty as P;
    let result = match property {
        P::Display => match input {
            "none" => Some(Display::None),
            "inline" => Some(Display::Inline),
            "block" => Some(Display::Block),
            "flex" => Some(Display::Flex),
            "grid" => Some(Display::Grid),
            "inline-block" => Some(Display::InlineBlock),
            "inline-flex" => Some(Display::InlineFlex),
            "inline-grid" => Some(Display::InlineGrid),
            "flow-root" => Some(Display::FlowRoot),
            "table" => Some(Display::Table),
            "list-item" => Some(Display::ListItem),
            "contents" => Some(Display::Contents),
            _ => None,
        }
        .map(StyleValue::Display),
        P::Position => match input {
            "static" => Some(Position::Static),
            "relative" => Some(Position::Relative),
            "absolute" => Some(Position::Absolute),
            "fixed" => Some(Position::Fixed),
            "sticky" => Some(Position::Sticky),
            _ => None,
        }
        .map(StyleValue::Position),
        P::Overflow => match input {
            "visible" => Some(Overflow::Visible),
            "hidden" => Some(Overflow::Hidden),
            "scroll" => Some(Overflow::Scroll),
            "auto" => Some(Overflow::Auto),
            "clip" => Some(Overflow::Clip),
            _ => None,
        }
        .map(StyleValue::Overflow),
        P::Width
        | P::Height
        | P::MinWidth
        | P::MinHeight
        | P::MaxWidth
        | P::MaxHeight
        | P::MarginTop
        | P::MarginRight
        | P::MarginBottom
        | P::MarginLeft
        | P::PaddingTop
        | P::PaddingRight
        | P::PaddingBottom
        | P::PaddingLeft
        | P::FlexBasis
        | P::RowGap
        | P::ColumnGap
        | P::FontSize => length(input).map(StyleValue::Length),
        P::Margin | P::Padding => edges(input).map(StyleValue::Edges),
        P::BackgroundColor | P::Color => color(input).map(StyleValue::Color),
        P::Opacity | P::FlexGrow | P::FlexShrink => input.parse().ok().map(StyleValue::Number),
        P::ZIndex => input.parse().ok().map(StyleValue::Integer),
        P::FlexDirection => match input {
            "row" => Some(FlexDirection::Row),
            "row-reverse" => Some(FlexDirection::RowReverse),
            "column" => Some(FlexDirection::Column),
            "column-reverse" => Some(FlexDirection::ColumnReverse),
            _ => None,
        }
        .map(StyleValue::FlexDirection),
        P::FlexWrap => match input {
            "nowrap" => Some(FlexWrap::Nowrap),
            "wrap" => Some(FlexWrap::Wrap),
            "wrap-reverse" => Some(FlexWrap::WrapReverse),
            _ => None,
        }
        .map(StyleValue::FlexWrap),
        P::AlignItems => match input {
            "normal" => Some(ItemPosition::Normal),
            "stretch" => Some(ItemPosition::Stretch),
            "center" => Some(ItemPosition::Center),
            "start" => Some(ItemPosition::Start),
            "end" => Some(ItemPosition::End),
            "flex-start" => Some(ItemPosition::FlexStart),
            "flex-end" => Some(ItemPosition::FlexEnd),
            "baseline" => Some(ItemPosition::Baseline),
            _ => None,
        }
        .map(|v| StyleValue::ItemAlignment(ItemAlignment::new(v))),
        P::JustifyContent => match input {
            "normal" => Some(ContentAlignment::default()),
            "center" => Some(ContentAlignment::new(ContentPosition::Center)),
            "start" => Some(ContentAlignment::new(ContentPosition::Start)),
            "end" => Some(ContentAlignment::new(ContentPosition::End)),
            "flex-start" => Some(ContentAlignment::new(ContentPosition::FlexStart)),
            "flex-end" => Some(ContentAlignment::new(ContentPosition::FlexEnd)),
            "space-between" => Some(ContentAlignment::with_distribution(
                ContentDistribution::SpaceBetween,
            )),
            "space-around" => Some(ContentAlignment::with_distribution(
                ContentDistribution::SpaceAround,
            )),
            "space-evenly" => Some(ContentAlignment::with_distribution(
                ContentDistribution::SpaceEvenly,
            )),
            _ => None,
        }
        .map(StyleValue::ContentAlignment),
        P::Gap => {
            let v: Vec<_> = input
                .split_whitespace()
                .map(length)
                .collect::<Option<_>>()
                .unwrap_or_default();
            match v.as_slice() {
                [a] => Some(StyleValue::Gap(Gap {
                    row: *a,
                    column: *a,
                })),
                [r, c] => Some(StyleValue::Gap(Gap {
                    row: *r,
                    column: *c,
                })),
                _ => None,
            }
        }
        P::FontFamily => match input.trim() {
            "sans-serif" => Some(FontFamilyList::generic(GenericFontFamily::SansSerif)),
            "serif" => Some(FontFamilyList::generic(GenericFontFamily::Serif)),
            "monospace" => Some(FontFamilyList::generic(GenericFontFamily::Monospace)),
            value if !value.is_empty() => {
                Some(FontFamilyList::single(value.trim_matches(['\'', '"'])))
            }
            _ => None,
        }
        .map(StyleValue::FontFamily),
        P::FontWeight => match input {
            "normal" => Some(FontWeight::NORMAL),
            "bold" => Some(FontWeight::BOLD),
            value => value
                .parse::<f32>()
                .ok()
                .filter(|v| (1.0..=1000.0).contains(v))
                .map(FontWeight),
        }
        .map(StyleValue::FontWeight),
        P::Border | P::BorderTop | P::BorderRight | P::BorderBottom | P::BorderLeft => {
            border(input).map(StyleValue::Border)
        }
        P::BorderRadius => edges(input).map(|v| StyleValue::CornerRadii(CornerRadii(v))),
        P::Cursor => match input {
            "auto" => Some(Cursor::Auto),
            "default" => Some(Cursor::Default),
            "pointer" => Some(Cursor::Pointer),
            "text" => Some(Cursor::Text),
            "move" => Some(Cursor::Move),
            "not-allowed" => Some(Cursor::NotAllowed),
            _ => None,
        }
        .map(StyleValue::Cursor),
        P::ListStyleType => match input {
            "none" => Some(ListStyleType::None),
            "disc" => Some(ListStyleType::Disc),
            _ => None,
        }
        .map(StyleValue::ListStyle),
        P::Transform => {
            if input == "none" {
                Some(StyleValue::Transform(TransformList::default()))
            } else {
                None
            }
        }
        P::PointerEvents => match input {
            "auto" => Some(PointerEvents::Auto),
            "none" => Some(PointerEvents::None),
            _ => None,
        }
        .map(StyleValue::PointerEvents),
    };
    result.ok_or_else(|| invalid(property, input))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyTypeError {
    pub property: StyleProperty,
    pub expected: ValueKind,
}

/// Apply a typed declaration to the renderer's computed style.
pub fn apply_to_computed(
    style: &mut ComputedStyle,
    property: StyleProperty,
    value: &StyleValue,
    viewport: (f32, f32),
) -> Result<InvalidationClass, PropertyTypeError> {
    use StyleProperty as P;
    let resolve = |v: LengthValue| v.resolve(viewport, style.font_size, 16.0);
    let mismatch = || PropertyTypeError {
        property,
        expected: property.metadata().value_kind,
    };
    match (property, value) {
        (P::Display, StyleValue::Display(v)) => style.display = *v,
        (P::Position, StyleValue::Position(v)) => style.position = *v,
        (P::Overflow, StyleValue::Overflow(v)) => {
            style.overflow_x = *v;
            style.overflow_y = *v;
        }
        (P::Width, StyleValue::Length(v)) => style.width = resolve(*v),
        (P::Height, StyleValue::Length(v)) => style.height = resolve(*v),
        (P::MinWidth, StyleValue::Length(v)) => style.min_width = resolve(*v),
        (P::MinHeight, StyleValue::Length(v)) => style.min_height = resolve(*v),
        (P::MaxWidth, StyleValue::Length(v)) => style.max_width = resolve(*v),
        (P::MaxHeight, StyleValue::Length(v)) => style.max_height = resolve(*v),
        (P::Margin, StyleValue::Edges(v)) => {
            style.margin_top = resolve(v.top);
            style.margin_right = resolve(v.right);
            style.margin_bottom = resolve(v.bottom);
            style.margin_left = resolve(v.left);
        }
        (P::MarginTop, StyleValue::Length(v)) => style.margin_top = resolve(*v),
        (P::MarginRight, StyleValue::Length(v)) => style.margin_right = resolve(*v),
        (P::MarginBottom, StyleValue::Length(v)) => style.margin_bottom = resolve(*v),
        (P::MarginLeft, StyleValue::Length(v)) => style.margin_left = resolve(*v),
        (P::Padding, StyleValue::Edges(v)) => {
            style.padding_top = resolve(v.top);
            style.padding_right = resolve(v.right);
            style.padding_bottom = resolve(v.bottom);
            style.padding_left = resolve(v.left);
        }
        (P::PaddingTop, StyleValue::Length(v)) => style.padding_top = resolve(*v),
        (P::PaddingRight, StyleValue::Length(v)) => style.padding_right = resolve(*v),
        (P::PaddingBottom, StyleValue::Length(v)) => style.padding_bottom = resolve(*v),
        (P::PaddingLeft, StyleValue::Length(v)) => style.padding_left = resolve(*v),
        (P::BackgroundColor, StyleValue::Color(v)) => style.background_color = *v,
        (P::Color, StyleValue::Color(v)) => style.color = *v,
        (P::Opacity, StyleValue::Number(v)) => style.opacity = v.clamp(0.0, 1.0),
        (P::ZIndex, StyleValue::Integer(v)) => style.z_index = Some(*v),
        (P::FlexDirection, StyleValue::FlexDirection(v)) => style.flex_direction = *v,
        (P::FlexWrap, StyleValue::FlexWrap(v)) => style.flex_wrap = *v,
        (P::FlexGrow, StyleValue::Number(v)) => style.flex_grow = *v,
        (P::FlexShrink, StyleValue::Number(v)) => style.flex_shrink = *v,
        (P::FlexBasis, StyleValue::Length(v)) => style.flex_basis = resolve(*v),
        (P::AlignItems, StyleValue::ItemAlignment(v)) => style.align_items = *v,
        (P::JustifyContent, StyleValue::ContentAlignment(v)) => style.justify_content = *v,
        (P::Gap, StyleValue::Gap(v)) => {
            style.row_gap = Some(resolve(v.row));
            style.column_gap = Some(resolve(v.column));
        }
        (P::RowGap, StyleValue::Length(v)) => style.row_gap = Some(resolve(*v)),
        (P::ColumnGap, StyleValue::Length(v)) => style.column_gap = Some(resolve(*v)),
        (P::FontFamily, StyleValue::FontFamily(v)) => style.font_family = v.clone(),
        (P::FontSize, StyleValue::Length(v)) => style.font_size = resolve(*v).value(),
        (P::FontWeight, StyleValue::FontWeight(v)) => style.font_weight = *v,
        (
            P::Border | P::BorderTop | P::BorderRight | P::BorderBottom | P::BorderLeft,
            StyleValue::Border(v),
        ) => {
            let targets = match property {
                P::Border => [true, true, true, true],
                P::BorderTop => [true, false, false, false],
                P::BorderRight => [false, true, false, false],
                P::BorderBottom => [false, false, true, false],
                _ => [false, false, false, true],
            };
            if targets[0] {
                style.border_top_width = v.width as i32;
                style.border_top_style = v.style;
                style.border_top_color = StyleColor::Resolved(v.color);
            }
            if targets[1] {
                style.border_right_width = v.width as i32;
                style.border_right_style = v.style;
                style.border_right_color = StyleColor::Resolved(v.color);
            }
            if targets[2] {
                style.border_bottom_width = v.width as i32;
                style.border_bottom_style = v.style;
                style.border_bottom_color = StyleColor::Resolved(v.color);
            }
            if targets[3] {
                style.border_left_width = v.width as i32;
                style.border_left_style = v.style;
                style.border_left_color = StyleColor::Resolved(v.color);
            }
        }
        (P::BorderRadius, StyleValue::CornerRadii(v)) => {
            let px = |value: LengthValue| resolve(value).value();
            style.border_top_left_radius = (px(v.0.top), px(v.0.top));
            style.border_top_right_radius = (px(v.0.right), px(v.0.right));
            style.border_bottom_right_radius = (px(v.0.bottom), px(v.0.bottom));
            style.border_bottom_left_radius = (px(v.0.left), px(v.0.left));
        }
        (P::ListStyleType, StyleValue::ListStyle(v)) => style.list_style_type = *v,
        (P::Transform, StyleValue::Transform(v)) => {
            let mut matrix = Transform2D::IDENTITY;
            for op in &v.0 {
                match op {
                    TransformOperation::Matrix(v) => matrix = *v,
                    TransformOperation::Scale(x, y) => {
                        matrix.a *= x;
                        matrix.d *= y;
                    }
                    TransformOperation::Translate(x, y) => {
                        matrix.e += resolve(*x).value();
                        matrix.f += resolve(*y).value();
                    }
                    TransformOperation::Rotate(deg) => {
                        let r = deg.to_radians();
                        let (s, c) = r.sin_cos();
                        matrix = Transform2D {
                            a: c,
                            b: s,
                            c: -s,
                            d: c,
                            e: matrix.e,
                            f: matrix.f,
                        };
                    }
                }
            }
            style.transform = matrix;
        }
        (P::Cursor, StyleValue::Cursor(_)) => {}
        (P::PointerEvents, StyleValue::PointerEvents(v)) => style.pointer_events = *v,
        _ => return Err(mismatch()),
    }
    Ok(property.metadata().invalidation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_lookup_and_metadata_are_stable() {
        assert_eq!(
            StyleProperty::from_css_name("padding"),
            Some(StyleProperty::Padding)
        );
        assert_eq!(StyleProperty::Opacity as u16, 22);
        assert_eq!(
            StyleProperty::Opacity.metadata().invalidation,
            InvalidationClass::Composite
        );
    }

    #[test]
    fn parses_checked_literals_and_applies_them() {
        let padding = parse_literal(StyleProperty::Padding, "8px 16px").unwrap();
        let mut style = ComputedStyle::initial();
        apply_to_computed(&mut style, StyleProperty::Padding, &padding, (800.0, 600.0)).unwrap();
        assert_eq!(style.padding_top, Length::px(8.0));
        assert_eq!(style.padding_right, Length::px(16.0));
        assert!(parse_literal(StyleProperty::Padding, "wat").is_err());
    }

    #[test]
    fn resolves_viewport_units_deterministically() {
        assert_eq!(
            LengthValue::ViewportHeight(100.0).resolve((800.0, 600.0), 16.0, 16.0),
            Length::px(600.0)
        );
    }
}
