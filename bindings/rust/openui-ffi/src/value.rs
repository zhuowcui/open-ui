use crate::generated::expected_value_tag;
use crate::registry::{get, ApiError, HandleKind, LocalHandle};
use crate::types::{OuiLength, OuiStatus, OuiStyleValue};
use openui_style::{
    Color, ContentAlignment, ContentDistribution, ContentPosition, Cursor, Display, FlexDirection,
    FlexWrap, FontWeight, ItemAlignment, ItemPosition, LengthValue, ListStyleType, Overflow,
    PointerEvents, Position, RendererStyleValue, StyleProperty, StyleValue,
};

pub(crate) fn length(value: OuiLength) -> Result<LengthValue, ApiError> {
    if !value.value.is_finite() {
        return Err(ApiError::new(
            OuiStatus::InvalidArgument,
            "length must be finite",
        ));
    }
    match value.unit {
        0 => Ok(LengthValue::px(value.value)),
        1 => Ok(LengthValue::percent(value.value)),
        2 => Ok(LengthValue::Em(value.value)),
        3 => Ok(LengthValue::Rem(value.value)),
        4 => Ok(LengthValue::ViewportWidth(value.value)),
        5 => Ok(LengthValue::ViewportHeight(value.value)),
        8 => Ok(LengthValue::ViewportMin(value.value)),
        9 => Ok(LengthValue::ViewportMax(value.value)),
        10 => Ok(LengthValue::Ch(value.value)),
        11 => Ok(LengthValue::Ex(value.value)),
        12 => Ok(LengthValue::Lh(value.value)),
        6 if value.value == 0.0 => Ok(LengthValue::auto()),
        7 if value.value == 0.0 => Ok(LengthValue::none()),
        6 | 7 => Err(ApiError::new(
            OuiStatus::InvalidArgument,
            "auto and none lengths require a zero numeric payload",
        )),
        _ => Err(ApiError::new(
            OuiStatus::InvalidArgument,
            "unknown length unit",
        )),
    }
}

pub(crate) fn style_value(
    property: StyleProperty,
    value: &OuiStyleValue,
) -> Result<StyleValue, ApiError> {
    let expected = expected_value_tag(property);
    if value.reserved != 0 {
        return Err(ApiError::new(
            OuiStatus::WrongValueType,
            "style value reserved field must be zero",
        ));
    }
    if value.tag == 6 {
        // A property-bound native value retains information that a scalar
        // payload cannot represent. It cannot be reused as another property.
        let address = unsafe { value.data.compound as usize };
        return match get(address, HandleKind::Compound)? {
            LocalHandle::PropertyCompound(owner, value) if owner == property => Ok(value),
            LocalHandle::Compound(value) if expected == 6 => Ok(value),
            _ => Err(ApiError::new(
                OuiStatus::WrongValueType,
                "compound value does not match property",
            )
            .detail(property as u32)),
        };
    }
    if value.tag != expected {
        return Err(ApiError::new(
            OuiStatus::WrongValueType,
            format!(
                "property `{}` expects style value tag {expected}, received {}",
                property.metadata().css_name,
                value.tag
            ),
        )
        .detail(property as u32));
    }
    // SAFETY: the active union member is selected only after validating the
    // caller-provided discriminant against generated property metadata.
    unsafe {
        match value.tag {
            1 => Ok(StyleValue::Length(length(value.data.length)?)),
            2 => {
                let number = value.data.number;
                if !number.is_finite() {
                    return Err(ApiError::new(
                        OuiStatus::InvalidArgument,
                        "numeric style values must be finite",
                    ));
                }
                if property == StyleProperty::FontWeight {
                    if !(1.0..=1000.0).contains(&number) {
                        return Err(ApiError::new(
                            OuiStatus::InvalidArgument,
                            "font weight must be in the range 1..=1000",
                        ));
                    }
                    Ok(StyleValue::FontWeight(FontWeight(number)))
                } else {
                    Ok(StyleValue::Number(number))
                }
            }
            3 if property == StyleProperty::ColumnCount => {
                let count = value.data.integer;
                if count < 0 {
                    return Err(ApiError::new(
                        OuiStatus::InvalidArgument,
                        "column count must be zero (auto) or positive",
                    ));
                }
                Ok(StyleValue::Renderer(
                    openui_style::RendererStyleValue::ColumnCount(
                        (count != 0).then_some(count as u32),
                    ),
                ))
            }
            3 => Ok(StyleValue::Integer(value.data.integer)),
            4 => {
                let color = value.data.color;
                Ok(StyleValue::Color(Color::from_rgba8(
                    color.red,
                    color.green,
                    color.blue,
                    color.alpha,
                )))
            }
            5 => enum_value(property, value.data.enum_value),
            _ => Err(ApiError::new(
                OuiStatus::WrongValueType,
                "unknown style value tag",
            )),
        }
    }
}

fn enum_value(property: StyleProperty, value: i32) -> Result<StyleValue, ApiError> {
    use StyleProperty as P;
    let result = match property {
        P::Display => match value {
            0 => Some(Display::None),
            1 => Some(Display::Inline),
            2 => Some(Display::Block),
            3 => Some(Display::Flex),
            4 => Some(Display::Grid),
            5 => Some(Display::InlineBlock),
            6 => Some(Display::InlineFlex),
            7 => Some(Display::InlineGrid),
            8 => Some(Display::FlowRoot),
            9 => Some(Display::Table),
            10 => Some(Display::ListItem),
            11 => Some(Display::Contents),
            12 => Some(Display::InlineTable),
            13 => Some(Display::TableRowGroup),
            14 => Some(Display::TableHeaderGroup),
            15 => Some(Display::TableFooterGroup),
            16 => Some(Display::TableRow),
            17 => Some(Display::TableCell),
            18 => Some(Display::TableColumnGroup),
            19 => Some(Display::TableColumn),
            20 => Some(Display::TableCaption),
            _ => None,
        }
        .map(StyleValue::Display),
        P::Position => match value {
            0 => Some(Position::Static),
            1 => Some(Position::Relative),
            2 => Some(Position::Absolute),
            3 => Some(Position::Fixed),
            4 => Some(Position::Sticky),
            _ => None,
        }
        .map(StyleValue::Position),
        P::Overflow | P::OverflowX | P::OverflowY => match value {
            0 => Some(Overflow::Visible),
            1 => Some(Overflow::Hidden),
            2 => Some(Overflow::Scroll),
            3 => Some(Overflow::Auto),
            4 => Some(Overflow::Clip),
            _ => None,
        }
        .map(|value| match property {
            P::OverflowX => StyleValue::Renderer(RendererStyleValue::OverflowX(value)),
            P::OverflowY => StyleValue::Renderer(RendererStyleValue::OverflowY(value)),
            _ => StyleValue::Overflow(value),
        }),
        P::FlexDirection => match value {
            0 => Some(FlexDirection::Row),
            1 => Some(FlexDirection::RowReverse),
            2 => Some(FlexDirection::Column),
            3 => Some(FlexDirection::ColumnReverse),
            _ => None,
        }
        .map(StyleValue::FlexDirection),
        P::FlexWrap => match value {
            0 => Some(FlexWrap::Nowrap),
            1 => Some(FlexWrap::Wrap),
            2 => Some(FlexWrap::WrapReverse),
            _ => None,
        }
        .map(StyleValue::FlexWrap),
        P::AlignItems | P::AlignSelf | P::JustifyItems | P::JustifySelf => match value {
            0 => Some(ItemPosition::Normal),
            1 => Some(ItemPosition::Stretch),
            2 => Some(ItemPosition::Center),
            3 => Some(ItemPosition::Start),
            4 => Some(ItemPosition::End),
            5 => Some(ItemPosition::FlexStart),
            6 => Some(ItemPosition::FlexEnd),
            7 => Some(ItemPosition::Baseline),
            8 => Some(ItemPosition::Auto),
            9 => Some(ItemPosition::SelfStart),
            10 => Some(ItemPosition::SelfEnd),
            11 => Some(ItemPosition::Left),
            12 => Some(ItemPosition::Right),
            13 => Some(ItemPosition::LastBaseline),
            14 => Some(ItemPosition::Legacy),
            _ => None,
        }
        .map(|value| {
            let value = ItemAlignment::new(value);
            match property {
                P::AlignSelf => StyleValue::Renderer(RendererStyleValue::AlignSelf(value)),
                P::JustifyItems => StyleValue::Renderer(RendererStyleValue::JustifyItems(value)),
                P::JustifySelf => StyleValue::Renderer(RendererStyleValue::JustifySelf(value)),
                _ => StyleValue::ItemAlignment(value),
            }
        }),
        P::JustifyContent | P::AlignContent => match value {
            0 => Some(ContentAlignment::default()),
            1 => Some(ContentAlignment::new(ContentPosition::Start)),
            2 => Some(ContentAlignment::new(ContentPosition::End)),
            3 => Some(ContentAlignment::new(ContentPosition::Center)),
            4 => Some(ContentAlignment::new(ContentPosition::FlexStart)),
            5 => Some(ContentAlignment::new(ContentPosition::FlexEnd)),
            6 => Some(ContentAlignment::with_distribution(
                ContentDistribution::SpaceBetween,
            )),
            7 => Some(ContentAlignment::with_distribution(
                ContentDistribution::SpaceAround,
            )),
            8 => Some(ContentAlignment::with_distribution(
                ContentDistribution::SpaceEvenly,
            )),
            9 => Some(ContentAlignment::with_distribution(
                ContentDistribution::Stretch,
            )),
            10 => Some(ContentAlignment::new(ContentPosition::Baseline)),
            11 => Some(ContentAlignment::new(ContentPosition::LastBaseline)),
            12 => Some(ContentAlignment::new(ContentPosition::Left)),
            13 => Some(ContentAlignment::new(ContentPosition::Right)),
            _ => None,
        }
        .map(|value| match property {
            P::AlignContent => StyleValue::Renderer(RendererStyleValue::AlignContent(value)),
            _ => StyleValue::ContentAlignment(value),
        }),
        P::Cursor => match value {
            0 => Some(Cursor::Auto),
            1 => Some(Cursor::Default),
            2 => Some(Cursor::Pointer),
            3 => Some(Cursor::Text),
            4 => Some(Cursor::Move),
            5 => Some(Cursor::NotAllowed),
            _ => None,
        }
        .map(StyleValue::Cursor),
        P::ListStyleType => match value {
            0 => Some(ListStyleType::None),
            1 => Some(ListStyleType::Disc),
            2 => Some(ListStyleType::DisclosureOpen),
            3 => Some(ListStyleType::DisclosureClosed),
            _ => None,
        }
        .map(StyleValue::ListStyle),
        P::PointerEvents => match value {
            0 => Some(PointerEvents::Auto),
            1 => Some(PointerEvents::None),
            _ => None,
        }
        .map(StyleValue::PointerEvents),
        _ => None,
    };
    result.ok_or_else(|| {
        ApiError::new(
            OuiStatus::InvalidArgument,
            format!(
                "enum value {value} is invalid for `{}`",
                property.metadata().css_name
            ),
        )
        .detail(property as u32)
    })
}
