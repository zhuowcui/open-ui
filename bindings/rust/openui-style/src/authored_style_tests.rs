use crate::*;

fn compose(declarations: &AuthoredStyleDeclarations, mut baseline: ComputedStyle) -> ComputedStyle {
    for declaration in declarations.contributions() {
        declaration
            .project_resolved(&mut baseline, declaration.value(), (320.0, 240.0))
            .unwrap();
    }
    baseline
}

#[test]
fn every_public_property_has_an_explicit_canonical_owner() {
    for metadata in PROPERTY_METADATA {
        let fields = metadata.property.canonical_field_groups();
        assert!(!fields.is_empty(), "{:?}", metadata.property);
        assert!(fields
            .iter()
            .enumerate()
            .all(|(index, field)| !fields[..index].contains(field)));
    }
    assert_eq!(StyleProperty::Font.canonical_field_groups().len(), 19);
    assert_eq!(StyleProperty::FontVariant.canonical_field_groups().len(), 7);
    assert_eq!(
        StyleProperty::WordWrap.canonical_field_groups(),
        StyleProperty::OverflowWrap.canonical_field_groups()
    );
    assert!(StyleProperty::Margin.affects_same_fields_as(StyleProperty::MarginLeft));
    assert!(StyleProperty::Border.affects_same_fields_as(StyleProperty::BorderImage));
    assert!(StyleProperty::BorderRadius.affects_same_fields_as(StyleProperty::BorderRadiusPercent));
    assert!(
        !StyleProperty::FontSynthesis.affects_same_fields_as(StyleProperty::FontSynthesisPosition)
    );
    assert!(
        !StyleProperty::TextEmphasis.affects_same_fields_as(StyleProperty::TextEmphasisPosition)
    );
}

#[test]
fn longhand_removal_does_not_revive_an_earlier_shorthand_and_units_stay_authored() {
    let mut declarations = AuthoredStyleDeclarations::default();
    let margin: StyleValue = Edges::all(LengthValue::Em(2.0)).into();
    assert!(declarations
        .try_set_validated(StyleProperty::Margin, margin.clone())
        .unwrap());
    assert!(!declarations
        .try_set_validated(StyleProperty::Margin, margin.clone())
        .unwrap());
    assert!(declarations
        .try_set_validated(StyleProperty::MarginLeft, LengthValue::px(3.0).into())
        .unwrap());
    assert!(declarations.remove(StyleProperty::MarginLeft));
    assert!(!declarations.remove(StyleProperty::MarginLeft));
    assert_eq!(declarations.contributions()[0].value(), &margin);
    assert!(!declarations.contributions()[0]
        .fields()
        .contains(CanonicalStyleField::MarginLeft));
    let mut baseline = ComputedStyle::initial();
    baseline.update_derived(|fields| {
        fields.font_size = 10.0;
        fields.margin_left = Length::px(9.0);
    });
    let first = compose(&declarations, baseline.clone());
    assert_eq!(first.margin_top, Length::px(20.0));
    assert_eq!(first.margin_left, Length::px(9.0));
    baseline.update_derived(|fields| fields.font_size = 20.0);
    let next = compose(&declarations, baseline);
    assert_eq!(next.margin_top, Length::px(40.0));
    assert_eq!(next.margin_left, Length::px(9.0));
    assert!(declarations
        .try_set_validated(StyleProperty::Margin, margin)
        .unwrap());
    assert_eq!(
        compose(&declarations, ComputedStyle::initial()).margin_left,
        Length::px(32.0)
    );
}

#[test]
fn later_overlapping_write_prevents_a_false_shorthand_noop() {
    let mut declarations = AuthoredStyleDeclarations::default();
    let padding: StyleValue = Edges::all(LengthValue::px(2.0)).into();
    declarations
        .try_set_validated(StyleProperty::Padding, padding.clone())
        .unwrap();
    declarations
        .try_set_validated(StyleProperty::PaddingLeft, LengthValue::px(1.0).into())
        .unwrap();
    assert!(declarations
        .try_set_validated(StyleProperty::Padding, padding.clone())
        .unwrap());
    assert_eq!(
        compose(&declarations, ComputedStyle::initial()).padding_left,
        Length::px(2.0)
    );
    assert!(!declarations
        .try_set_validated(StyleProperty::Padding, padding)
        .unwrap());
    declarations
        .try_set_validated(StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    declarations
        .try_set_validated(StyleProperty::OverflowY, Overflow::Visible.into())
        .unwrap();
    assert!(declarations
        .try_set_validated(StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap());
    assert_eq!(
        compose(&declarations, ComputedStyle::initial()).overflow_y,
        Overflow::Hidden
    );
}

#[test]
fn emphasis_and_synthesis_leave_the_independent_position_longhands_intact() {
    let mut declarations = AuthoredStyleDeclarations::default();
    let position = TextEmphasisPosition {
        over: false,
        right: false,
    };
    declarations
        .try_set_validated(StyleProperty::TextEmphasisPosition, position.into())
        .unwrap();
    declarations
        .try_set_validated(
            StyleProperty::FontSynthesisPosition,
            FontSynthesis::None.into(),
        )
        .unwrap();
    declarations
        .try_set_validated(
            StyleProperty::FontSynthesis,
            FontSynthesisShorthand::AUTO.into(),
        )
        .unwrap();
    declarations
        .try_set_validated(
            StyleProperty::TextEmphasis,
            TextEmphasisShorthand {
                style: TextEmphasisStyle {
                    mark: TextEmphasisMark::Circle,
                    fill: TextEmphasisFill::Open,
                },
                color: StyleColor::CurrentColor,
                position: TextEmphasisPosition::INITIAL,
            }
            .into(),
        )
        .unwrap();
    let style = compose(&declarations, ComputedStyle::initial());
    assert_eq!(style.font_synthesis_position, FontSynthesis::None);
    assert_eq!(style.font_synthesis_weight, FontSynthesis::Auto);
    assert_eq!(style.text_emphasis_position, position);
    assert_eq!(style.text_emphasis_mark, TextEmphasisMark::Circle);
    assert_eq!(style.text_emphasis_color, StyleColor::CurrentColor);
    assert!(declarations.remove(StyleProperty::FontSynthesis));
    assert!(declarations.remove(StyleProperty::TextEmphasis));
    let reset = compose(&declarations, ComputedStyle::initial());
    assert_eq!(reset.font_synthesis_position, FontSynthesis::None);
    assert_eq!(reset.text_emphasis_position, position);
    assert_eq!(reset.text_emphasis_mark, TextEmphasisMark::None);
}

fn border_image() -> BorderImage {
    BorderImage {
        source: CssImage::Raster(ImageResourceId(7)),
        slice: std::array::from_fn(|_| BorderImageLength::Number(1.0)),
        fill: false,
        width: std::array::from_fn(|_| BorderImageLength::Number(1.0)),
        outset: std::array::from_fn(|_| BorderImageLength::Number(0.0)),
        repeat_x: BorderImageRepeat::Stretch,
        repeat_y: BorderImageRepeat::Stretch,
    }
}

#[test]
fn border_resets_supported_image_state_but_a_side_does_not() {
    let mut declarations = AuthoredStyleDeclarations::default();
    declarations
        .try_set_validated(
            StyleProperty::BorderImage,
            StyleValue::Renderer(RendererStyleValue::BorderImage(Some(border_image()))),
        )
        .unwrap();
    let border = Border {
        width: 2.0,
        style: BorderStyle::Solid,
        color: Color::RED,
    };
    declarations
        .try_set_validated(StyleProperty::BorderTop, border.into())
        .unwrap();
    assert!(compose(&declarations, ComputedStyle::initial())
        .border_image
        .is_some());
    declarations
        .try_set_validated(StyleProperty::Border, border.into())
        .unwrap();
    assert!(compose(&declarations, ComputedStyle::initial())
        .border_image
        .is_none());
    assert!(declarations.remove(StyleProperty::BorderImage));
    assert!(!declarations.remove(StyleProperty::BorderImage));
    let mut baseline = ComputedStyle::initial();
    baseline.update_derived(|fields| fields.border_image = Some(border_image()));
    let projected = compose(&declarations, baseline);
    assert!(projected.border_image.is_some());
    assert_eq!(projected.border_top_width, 2);
}

#[test]
fn radius_sources_and_native_computed_overrides_keep_distinct_ownership() {
    let mut declarations = AuthoredStyleDeclarations::default();
    let radius: StyleValue =
        CornerRadii(Edges::all(LengthValue::calc_percent_px(29.0, 1.0))).into();
    declarations
        .try_set_validated(StyleProperty::BorderRadius, radius.clone())
        .unwrap();
    let style = compose(&declarations, ComputedStyle::initial());
    assert_eq!(
        style.border_radii_for_box((5.0, 10.0))[0].0.to_bits(),
        2.4499998092651367_f32.to_bits()
    );
    assert_eq!(
        style.border_radius_lengths[0],
        Some((
            Length::calc_percent_px(29.0, 1.0),
            Length::calc_percent_px(29.0, 1.0)
        ))
    );
    declarations
        .try_set_validated(
            StyleProperty::BorderRadiusPercent,
            StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent([(true, false); 4])),
        )
        .unwrap();
    let overridden = compose(&declarations, ComputedStyle::initial());
    assert_eq!(overridden.border_radius_lengths, [None; 4]);
    assert!(declarations
        .try_set_validated(StyleProperty::BorderRadius, radius.clone())
        .unwrap());
    assert!(!declarations
        .try_set_validated(StyleProperty::BorderRadius, radius.clone())
        .unwrap());
    let restored = compose(&declarations, ComputedStyle::initial());
    assert_eq!(
        restored.border_radii_for_box((5.0, 10.0)),
        style.border_radii_for_box((5.0, 10.0))
    );
    declarations
        .try_set_validated(
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((7.0, 8.0))),
        )
        .unwrap();
    let explicit = compose(&declarations, ComputedStyle::initial());
    assert_eq!(explicit.border_radius_lengths[0], None);
    assert_eq!(explicit.border_top_left_radius, (7.0, 8.0));
    assert!(declarations.remove(StyleProperty::BorderTopLeftRadius));
    let removed = compose(&declarations, ComputedStyle::initial());
    assert_eq!(removed.border_top_left_radius, (0.0, 0.0));
    assert!(removed.border_radius_lengths[1].is_some());
    assert_eq!(
        declarations
            .contributions()
            .iter()
            .find(|value| value.property() == StyleProperty::BorderRadius)
            .unwrap()
            .value(),
        &radius
    );
    assert_eq!(
        value_from_computed(&restored, StyleProperty::BorderRadius),
        radius
    );
}

#[test]
fn legacy_whitespace_and_wrap_payloads_publish_canonical_components_and_summaries() {
    let mut declarations = AuthoredStyleDeclarations::default();
    declarations
        .try_set_validated(
            StyleProperty::WhiteSpace,
            StyleValue::Renderer(RendererStyleValue::WhiteSpace(WhiteSpace::PreWrap)),
        )
        .unwrap();
    declarations
        .try_set_validated(StyleProperty::TextWrapStyle, TextWrapStyle::Pretty.into())
        .unwrap();
    declarations
        .try_set_validated(StyleProperty::TextWrapMode, TextWrapMode::Nowrap.into())
        .unwrap();
    let style = compose(&declarations, ComputedStyle::initial());
    assert_eq!(style.white_space_collapse, WhiteSpaceCollapse::Preserve);
    assert_eq!(style.text_wrap_mode, TextWrapMode::Nowrap);
    assert_eq!(style.text_wrap_style, TextWrapStyle::Pretty);
    assert_eq!(style.white_space, WhiteSpace::Pre);
    assert_eq!(style.text_wrap, TextWrap::Nowrap);
    declarations
        .try_set_validated(
            StyleProperty::TextWrap,
            StyleValue::Renderer(RendererStyleValue::TextWrap(TextWrap::Balance)),
        )
        .unwrap();
    let next = compose(&declarations, ComputedStyle::initial());
    assert_eq!(next.white_space_collapse, WhiteSpaceCollapse::Preserve);
    assert_eq!(next.text_wrap_mode, TextWrapMode::Wrap);
    assert_eq!(next.text_wrap_style, TextWrapStyle::Balance);
    assert_eq!(next.white_space, WhiteSpace::PreWrap);
    assert_eq!(next.text_wrap, TextWrap::Balance);
    declarations.remove(StyleProperty::TextWrap);
    let reset = compose(&declarations, ComputedStyle::initial());
    assert_eq!(reset.white_space_collapse, WhiteSpaceCollapse::Preserve);
    assert_eq!(reset.text_wrap_style, TextWrapStyle::Auto);
    let mut direct = ComputedStyle::initial();
    assert!(RendererStyleValue::WhiteSpace(WhiteSpace::BreakSpaces)
        .apply(&mut direct, StyleProperty::WhiteSpace));
    assert_eq!(direct.white_space_collapse, WhiteSpaceCollapse::BreakSpaces);
    assert!(
        RendererStyleValue::TextWrap(TextWrap::Nowrap).apply(&mut direct, StyleProperty::TextWrap)
    );
    assert_eq!(direct.text_wrap_mode, TextWrapMode::Nowrap);
    assert_eq!(direct.white_space, WhiteSpace::BreakSpaces);
}

#[test]
fn alias_removal_uses_the_same_owner_and_does_not_revive_an_old_value() {
    let mut declarations = AuthoredStyleDeclarations::default();
    declarations
        .try_set_validated(StyleProperty::OverflowWrap, OverflowWrap::Anywhere.into())
        .unwrap();
    declarations
        .try_set_validated(StyleProperty::WordWrap, OverflowWrap::BreakWord.into())
        .unwrap();
    assert_eq!(declarations.contributions().len(), 1);
    assert_eq!(
        compose(&declarations, ComputedStyle::initial()).overflow_wrap,
        OverflowWrap::BreakWord
    );
    assert!(declarations.remove(StyleProperty::OverflowWrap));
    assert!(declarations.contributions().is_empty());
    assert_eq!(
        compose(&declarations, ComputedStyle::initial()).overflow_wrap,
        OverflowWrap::Normal
    );
}

#[test]
fn wrapping_computed_roundtrips_keep_combinations_missing_from_legacy_summaries() {
    for (collapse, mode, wrap_style) in [
        (
            WhiteSpaceCollapse::BreakSpaces,
            TextWrapMode::Nowrap,
            TextWrapStyle::Pretty,
        ),
        (
            WhiteSpaceCollapse::PreserveBreaks,
            TextWrapMode::Nowrap,
            TextWrapStyle::Balance,
        ),
        (
            WhiteSpaceCollapse::Collapse,
            TextWrapMode::Wrap,
            TextWrapStyle::Stable,
        ),
    ] {
        let whitespace = WhiteSpaceShorthand {
            collapse,
            wrap: mode,
        };
        let wrapping = TextWrapShorthand {
            mode,
            style: wrap_style,
        };
        let mut source = ComputedStyle::initial();
        apply_to_computed(
            &mut source,
            StyleProperty::WhiteSpace,
            &whitespace.into(),
            (320.0, 240.0),
        )
        .unwrap();
        apply_to_computed(
            &mut source,
            StyleProperty::TextWrap,
            &wrapping.into(),
            (320.0, 240.0),
        )
        .unwrap();
        let read_whitespace = value_from_computed(&source, StyleProperty::WhiteSpace);
        let read_wrapping = value_from_computed(&source, StyleProperty::TextWrap);
        assert_eq!(read_whitespace, whitespace.into());
        assert_eq!(read_wrapping, wrapping.into());
        let mut copy = ComputedStyle::initial();
        apply_to_computed(
            &mut copy,
            StyleProperty::WhiteSpace,
            &read_whitespace,
            (320.0, 240.0),
        )
        .unwrap();
        apply_to_computed(
            &mut copy,
            StyleProperty::TextWrap,
            &read_wrapping,
            (320.0, 240.0),
        )
        .unwrap();
        assert_eq!(copy.white_space_collapse, collapse);
        assert_eq!(copy.text_wrap_mode, mode);
        assert_eq!(copy.text_wrap_style, wrap_style);
        assert_eq!(copy.white_space, source.white_space);
        assert_eq!(copy.text_wrap, source.text_wrap);
    }
}

#[test]
fn projection_preserves_currentcolor_source_and_type_errors_are_atomic() {
    let mut declarations = AuthoredStyleDeclarations::default();
    let current: StyleValue = StyleColor::CurrentColor.into();
    declarations
        .try_set_validated(StyleProperty::BackgroundColor, current.clone())
        .unwrap();
    declarations
        .try_set_validated(StyleProperty::Color, Color::RED.into())
        .unwrap();
    let first = compose(&declarations, ComputedStyle::initial());
    assert!(first.background_current_color);
    assert_eq!(first.background_color, Color::RED);
    declarations
        .try_set_validated(StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    let next = compose(&declarations, ComputedStyle::initial());
    assert_eq!(next.background_color, Color::BLUE);
    assert_eq!(declarations.contributions()[0].value(), &current);
    let mut candidate = next.clone();
    let declaration = &declarations.contributions()[0];
    assert!(declaration
        .project_resolved(&mut candidate, &StyleValue::Number(9.0), (320.0, 240.0))
        .is_err());
    assert_eq!(candidate.background_color, next.background_color);
    assert_eq!(
        candidate.background_current_color,
        next.background_current_color
    );
    assert_eq!(candidate.color, next.color);
}

#[test]
fn independent_radius_axes_preserve_masked_sources_and_old_pair_presence() {
    let mut declarations = AuthoredStyleDeclarations::default();
    let radius: StyleValue = CornerRadii(Edges::all(LengthValue::Em(2.0))).into();
    declarations
        .try_set_validated(StyleProperty::BorderRadius, radius.clone())
        .unwrap();
    declarations
        .try_set_validated(
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::calc_percent_px(25.0, -3.0).into(),
        )
        .unwrap();
    assert!(declarations.remove(StyleProperty::BorderTopLeftRadiusX));
    let mut baseline = ComputedStyle::initial();
    baseline.update_derived(|fields| fields.font_size = 10.0);
    let a = compose(&declarations, baseline.clone());
    assert_eq!(a.border_radii_for_box((100.0, 80.0))[0], (0.0, 20.0));
    baseline.update_derived(|fields| fields.font_size = 20.0);
    let b = compose(&declarations, baseline);
    assert_eq!(b.border_radii_for_box((100.0, 80.0))[0], (0.0, 40.0));
    assert_eq!(declarations.contributions()[0].value(), &radius);
    assert!(!declarations.contributions()[0]
        .fields()
        .contains(CanonicalStyleField::BorderTopLeftRadiusX));
    assert!(declarations.contributions()[0]
        .fields()
        .contains(CanonicalStyleField::BorderTopLeftRadiusY));

    declarations
        .try_set_validated(
            StyleProperty::BorderTopLeftRadius,
            StyleValue::Renderer(RendererStyleValue::BorderTopLeftRadius((7.0, 8.0))),
        )
        .unwrap();
    let old_pair = compose(&declarations, ComputedStyle::initial());
    assert_eq!(old_pair.border_radius_lengths[0], None);
    assert_eq!(old_pair.border_top_left_radius, (7.0, 8.0));
    declarations
        .try_set_validated(
            StyleProperty::BorderTopLeftRadiusX,
            LengthValue::calc_percent_px(25.0, -3.0).into(),
        )
        .unwrap();
    let mixed = compose(&declarations, ComputedStyle::initial());
    assert_eq!(
        mixed.border_radius_lengths[0],
        Some((Length::calc_percent_px(25.0, -3.0), Length::px(8.0)))
    );
    assert_eq!(mixed.border_radii_for_box((100.0, 80.0))[0], (22.0, 8.0));
    declarations.remove(StyleProperty::BorderTopLeftRadiusX);
    assert_eq!(
        compose(&declarations, ComputedStyle::initial()).border_radii_for_box((100.0, 80.0))[0],
        (0.0, 8.0)
    );
    assert!(declarations
        .try_set_validated(StyleProperty::BorderRadius, radius)
        .unwrap());
}

#[test]
fn ordered_radius_percentage_override_remains_separate_from_axis_ownership() {
    let mut declarations = AuthoredStyleDeclarations::default();
    let x: StyleValue = LengthValue::calc_percent_px(25.0, 5.0).into();
    declarations
        .try_set_validated(StyleProperty::BorderTopLeftRadiusX, x.clone())
        .unwrap();
    declarations
        .try_set_validated(
            StyleProperty::BorderTopLeftRadiusY,
            LengthValue::px(8.0).into(),
        )
        .unwrap();
    declarations
        .try_set_validated(
            StyleProperty::BorderRadiusPercent,
            StyleValue::Renderer(RendererStyleValue::BorderRadiusPercent([(true, false); 4])),
        )
        .unwrap();
    let legacy = compose(&declarations, ComputedStyle::initial());
    assert_eq!(legacy.border_radius_lengths, [None; 4]);
    assert_eq!(legacy.border_radii_for_box((100.0, 80.0))[0], (25.0, 8.0));
    assert!(declarations
        .try_set_validated(StyleProperty::BorderTopLeftRadiusX, x.clone())
        .unwrap());
    let later = compose(&declarations, ComputedStyle::initial());
    assert_eq!(later.border_radii_for_box((100.0, 80.0))[0], (30.0, 8.0));
    assert!(!declarations
        .try_set_validated(StyleProperty::BorderTopLeftRadiusX, x)
        .unwrap());
    assert!(declarations.remove(StyleProperty::BorderRadiusPercent));
    assert_eq!(
        compose(&declarations, ComputedStyle::initial()).border_radii_for_box((100.0, 80.0))[0],
        (30.0, 8.0)
    );
    assert_eq!(CanonicalStyleField::BorderTopLeftRadius as u16, 128);
    assert_eq!(CanonicalStyleField::WebkitTextFillColor as u16, 229);
    assert_eq!(CanonicalStyleField::BorderTopLeftRadiusX as u16, 230);
    assert_eq!(CanonicalStyleField::BorderBottomLeftRadiusY as u16, 237);
    assert!(StyleProperty::BorderRadius.affects_same_fields_as(StyleProperty::BorderRadiusPercent));
    assert!(StyleProperty::BorderTopLeftRadius
        .affects_same_fields_as(StyleProperty::BorderTopLeftRadiusX));
    assert!(!StyleProperty::BorderTopLeftRadiusX
        .affects_same_fields_as(StyleProperty::BorderTopLeftRadiusY));
}
