//! Shared source ownership, reset, context and effect lifecycle guards.
//! Actual Chromium and pixel qualification are separate required gates.
use openui_dom::ElementTag;
use openui_engine::{
    AnimationTimeline, Engine, EngineError, NodeHandle, ScrollButtonDirection, ViewportMetrics,
};
use openui_style::{
    AnimationOptions, Color, ComputedStyle, CornerRadii, Display, Edges, FillMode, FontFamilyList,
    FontShorthand, FontSynthesis, FontSynthesisShorthand, Keyframes, LengthValue, LineHeight,
    Overflow, OverflowClipBox, Position, PropertyKeyframes, PseudoStyleTarget,
    RendererInternalStyleValue, RendererStyleValue, Style, StyleColor, StyleProperty, StyleValue,
    TextEmphasisFill, TextEmphasisMark, TextEmphasisPosition, TextEmphasisShorthand,
    TextEmphasisStyle, TextWrapMode, TextWrapShorthand, TextWrapStyle, TransformList,
    TransformOperation, WhiteSpaceCollapse, WhiteSpaceShorthand,
};

fn engine() -> Engine {
    Engine::new(ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap()).unwrap()
}
fn node(e: &mut Engine, parent: NodeHandle, tag: ElementTag) -> NodeHandle {
    let n = e.create_native_element(tag).unwrap();
    e.append_child(parent, n).unwrap();
    n
}
fn box_node(e: &mut Engine) -> NodeHandle {
    let root = e.root();
    let n = node(e, root, ElementTag::Div);
    e.set_property(n, StyleProperty::Width, LengthValue::px(80.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Height, LengthValue::px(60.0).into())
        .unwrap();
    n
}
fn opacity_effect(e: &mut Engine, n: NodeHandle, fill: FillMode) -> openui_engine::AnimationId {
    e.animate(
        n,
        PropertyKeyframes::typed(
            StyleProperty::Opacity,
            Keyframes::from_values(0.8_f32, 0.8_f32),
        )
        .unwrap(),
        AnimationOptions {
            duration_ms: 100.0,
            delay_ms: 50.0,
            fill,
            ..AnimationOptions::default()
        },
        AnimationTimeline::Document,
    )
    .unwrap()
}

#[test]
fn removal_restores_constructor_sources_and_raw_creation_keeps_initial_semantics() {
    let mut e = engine();
    assert_eq!(e.computed_style(e.root()).unwrap().display, Display::Block);
    assert!(!e.remove_property(e.root(), StyleProperty::Display).unwrap());
    let raw = e.create_element(ElementTag::Div).unwrap();
    e.set_property(raw, StyleProperty::Display, Display::None.into())
        .unwrap();
    assert!(e.remove_property(raw, StyleProperty::Display).unwrap());
    assert_eq!(e.computed_style(raw).unwrap().display, Display::Inline);
    for (tag, expected) in [
        (ElementTag::Div, Display::Block),
        (ElementTag::Table, Display::Table),
        (ElementTag::Image, Display::InlineBlock),
        (ElementTag::TextArea, Display::InlineBlock),
    ] {
        let n = e.create_native_element(tag).unwrap();
        e.set_property(n, StyleProperty::Display, Display::None.into())
            .unwrap();
        assert!(e.remove_property(n, StyleProperty::Display).unwrap());
        assert_eq!(e.computed_style(n).unwrap().display, expected);
        assert!(!e.remove_property(n, StyleProperty::Display).unwrap());
    }
    for tag in [
        ElementTag::Image,
        ElementTag::Canvas,
        ElementTag::Video,
        ElementTag::IFrame,
        ElementTag::Embed,
    ] {
        let n = e.create_native_element(tag).unwrap();
        e.set_property(n, StyleProperty::Overflow, Overflow::Visible.into())
            .unwrap();
        assert!(e.remove_property(n, StyleProperty::Overflow).unwrap());
        let s = e.computed_style(n).unwrap();
        assert_eq!(
            (s.overflow_x, s.overflow_y),
            (Overflow::Clip, Overflow::Clip)
        );
        assert_eq!(s.overflow_clip_box, OverflowClipBox::ContentBox);
    }
    let ordinary = box_node(&mut e);
    assert!(e.remove_property(ordinary, StyleProperty::Width).unwrap());
    assert!(e.computed_style(ordinary).unwrap().width.is_auto());
    assert_eq!(e.bounds(ordinary).unwrap().unwrap().width, 320.0);
    let foreign = e.create_svg_foreign_object().unwrap();
    e.set_property(foreign, StyleProperty::Overflow, Overflow::Visible.into())
        .unwrap();
    e.remove_property(foreign, StyleProperty::Overflow).unwrap();
    assert_eq!(
        e.computed_style(foreign).unwrap().overflow_x,
        Overflow::Hidden
    );
}

#[test]
fn longhand_reset_does_not_revive_masked_shorthand_after_font_or_viewport_changes() {
    let mut e = engine();
    let root = e.root();
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(10.0).into())
        .unwrap();
    let n = node(&mut e, root, ElementTag::Div);
    e.set_property(
        n,
        StyleProperty::Padding,
        Edges {
            top: LengthValue::Em(2.0),
            right: LengthValue::Rem(3.0),
            bottom: LengthValue::ViewportWidth(4.0),
            left: LengthValue::Em(5.0),
        }
        .into(),
    )
    .unwrap();
    e.set_property(n, StyleProperty::PaddingLeft, LengthValue::px(7.0).into())
        .unwrap();
    e.remove_property(n, StyleProperty::PaddingLeft).unwrap();
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
        .unwrap();
    e.set_viewport(ViewportMetrics::from_logical_size(500.0, 240.0, 1.0).unwrap())
        .unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(
        (
            s.padding_top.value(),
            s.padding_right.value(),
            s.padding_bottom.value(),
            s.padding_left.value()
        ),
        (40.0, 60.0, 20.0, 0.0)
    );
    assert!(!e.remove_property(n, StyleProperty::PaddingLeft).unwrap());
    e.remove_property(n, StyleProperty::Padding).unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(
        (
            s.padding_top.value(),
            s.padding_right.value(),
            s.padding_bottom.value(),
            s.padding_left.value()
        ),
        (0.0, 0.0, 0.0, 0.0)
    );
}

#[test]
fn masked_font_size_resets_to_inheritance_and_keeps_surviving_font_context() {
    let mut e = engine();
    let root = e.root();
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(12.0).into())
        .unwrap();
    let n = node(&mut e, root, ElementTag::Div);
    let mut font = FontShorthand::new(20.0, FontFamilyList::single("Ahem")).unwrap();
    font.line_height = LineHeight::Number(1.5);
    e.set_property(n, StyleProperty::Font, font.into()).unwrap();
    e.set_property(n, StyleProperty::FontSize, LengthValue::px(30.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Width, LengthValue::Em(2.0).into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().width.value(), 60.0);
    e.remove_property(n, StyleProperty::FontSize).unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(s.font_size, 12.0);
    assert_eq!(s.font_family, FontFamilyList::single("Ahem"));
    assert_eq!(s.line_height, LineHeight::Number(1.5));
    assert_eq!(s.width.value(), 24.0);
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(18.0).into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().font_size, 18.0);
    assert_eq!(e.computed_style(n).unwrap().width.value(), 36.0);
    e.remove_property(n, StyleProperty::Font).unwrap();
    assert_eq!(
        e.computed_style(n).unwrap().line_height,
        e.computed_style(root).unwrap().line_height
    );
}

#[test]
fn surviving_metric_units_use_final_family_size_and_line_height_after_mask_removal() {
    let mut e = Engine::new_with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 240.0, 1.0).unwrap(),
        openui_text::FontCollection::deterministic_test(),
    )
    .unwrap();
    let root = e.root();
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(20.0).into())
        .unwrap();
    let n = node(&mut e, root, ElementTag::Div);
    e.set_property(n, StyleProperty::Width, LengthValue::Ch(2.0).into())
        .unwrap();
    e.set_property(n, StyleProperty::Height, LengthValue::Lh(2.0).into())
        .unwrap();
    let mut font = FontShorthand::new(30.0, FontFamilyList::single("Ahem")).unwrap();
    font.line_height = LineHeight::Number(1.5);
    e.set_property(n, StyleProperty::Font, font.into()).unwrap();
    e.remove_property(n, StyleProperty::FontSize).unwrap();
    let neighbor = node(&mut e, root, ElementTag::Div);
    e.set_property(
        neighbor,
        StyleProperty::FontFamily,
        FontFamilyList::single("Ahem").into(),
    )
    .unwrap();
    e.set_property(
        neighbor,
        StyleProperty::LineHeight,
        LineHeight::Number(1.5).into(),
    )
    .unwrap();
    e.set_property(neighbor, StyleProperty::Width, LengthValue::Ch(2.0).into())
        .unwrap();
    e.set_property(neighbor, StyleProperty::Height, LengthValue::Lh(2.0).into())
        .unwrap();
    let a = e.computed_style(n).unwrap();
    let b = e.computed_style(neighbor).unwrap();
    assert_eq!(
        (a.font_size, a.width, a.height),
        (b.font_size, b.width, b.height)
    );
    // Chromium keeps the selected font's actual ZeroInlineSize float here;
    // a computed ch length is not an ideal em width or a layout pixel size.
    let units = openui_text::FontRelativeLengthResolver::from_style_in_collection(
        a,
        e.font_collection().clone(),
    );
    assert_eq!(a.font_size, 20.0);
    assert_eq!(a.height.value(), 60.0);
    assert_eq!(
        a.width.value(),
        units.resolve(2.0, openui_text::FontRelativeUnit::Ch)
    );
    e.set_property(root, StyleProperty::FontSize, LengthValue::px(24.0).into())
        .unwrap();
    let a = e.computed_style(n).unwrap();
    let b = e.computed_style(neighbor).unwrap();
    assert_eq!(
        (a.font_size, a.width, a.height),
        (b.font_size, b.width, b.height)
    );
    // Chromium keeps the selected font's actual ZeroInlineSize float here;
    // a computed ch length is not an ideal em width or a layout pixel size.
    let units = openui_text::FontRelativeLengthResolver::from_style_in_collection(
        a,
        e.font_collection().clone(),
    );
    assert_eq!(a.font_size, 24.0);
    assert_eq!(a.height.value(), 72.0);
    assert_eq!(
        a.width.value(),
        units.resolve(2.0, openui_text::FontRelativeUnit::Ch)
    );
}

#[test]
fn modern_typography_components_survive_recomposition_and_partial_removal() {
    let mut e = engine();
    let n = box_node(&mut e);
    e.set_property(
        n,
        StyleProperty::WhiteSpace,
        WhiteSpaceShorthand {
            collapse: WhiteSpaceCollapse::BreakSpaces,
            wrap: TextWrapMode::Nowrap,
        }
        .into(),
    )
    .unwrap();
    e.set_property(
        n,
        StyleProperty::TextWrap,
        TextWrapShorthand {
            mode: TextWrapMode::Nowrap,
            style: TextWrapStyle::Pretty,
        }
        .into(),
    )
    .unwrap();
    e.set_property(n, StyleProperty::Opacity, 0.5_f32.into())
        .unwrap();
    e.remove_property(n, StyleProperty::TextWrapMode).unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(s.white_space_collapse, WhiteSpaceCollapse::BreakSpaces);
    assert_eq!(s.text_wrap_mode, TextWrapMode::Wrap);
    assert_eq!(s.text_wrap_style, TextWrapStyle::Pretty);
    e.remove_property(n, StyleProperty::WhiteSpace).unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(s.white_space_collapse, WhiteSpaceCollapse::Collapse);
    assert_eq!(s.text_wrap_style, TextWrapStyle::Pretty);
}

#[test]
fn currentcolor_and_dependent_text_fill_follow_final_surviving_own_color() {
    let mut e = engine();
    let root = e.root();
    e.set_property(root, StyleProperty::Color, Color::RED.into())
        .unwrap();
    let n = node(&mut e, root, ElementTag::Div);
    e.set_property(n, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    e.set_property(
        n,
        StyleProperty::BackgroundColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    e.set_property(
        n,
        StyleProperty::WebkitTextFillColor,
        StyleColor::CurrentColor.into(),
    )
    .unwrap();
    e.remove_property(n, StyleProperty::Color).unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(
        (s.color, s.background_color, s.effective_text_fill_color()),
        (Color::RED, Color::RED, Color::RED)
    );
    e.set_property(root, StyleProperty::Color, Color::GREEN.into())
        .unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(
        (s.color, s.background_color, s.effective_text_fill_color()),
        (Color::GREEN, Color::GREEN, Color::GREEN)
    );
}

#[test]
fn independent_shorthand_positions_and_radius_sources_do_not_revive() {
    let mut e = engine();
    let n = box_node(&mut e);
    let position = TextEmphasisPosition {
        over: false,
        right: false,
    };
    e.set_property(n, StyleProperty::TextEmphasisPosition, position.into())
        .unwrap();
    e.set_property(
        n,
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
    e.set_property(
        n,
        StyleProperty::FontSynthesisPosition,
        FontSynthesis::None.into(),
    )
    .unwrap();
    e.set_property(
        n,
        StyleProperty::FontSynthesis,
        FontSynthesisShorthand::AUTO.into(),
    )
    .unwrap();
    e.remove_property(n, StyleProperty::TextEmphasis).unwrap();
    e.remove_property(n, StyleProperty::FontSynthesis).unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(s.text_emphasis_position, position);
    assert_eq!(s.font_synthesis_position, FontSynthesis::None);
    e.set_property(
        n,
        StyleProperty::BorderRadius,
        CornerRadii(Edges::all(LengthValue::calc_percent_px(29.0, 1.0))).into(),
    )
    .unwrap();
    e.remove_property(n, StyleProperty::BorderTopLeftRadius)
        .unwrap();
    let s = e.computed_style(n).unwrap();
    assert_eq!(s.border_radius_lengths[0], None);
    assert_eq!(
        s.border_radius_lengths[1],
        Some((
            openui_geometry::Length::calc_percent_px(29.0, 1.0),
            openui_geometry::Length::calc_percent_px(29.0, 1.0)
        ))
    );
    // This guard verifies retained style sources, not yet paint/hit adoption.
}

#[test]
fn border_reset_and_removal_do_not_revive_an_older_image_declaration() {
    use openui_style::{
        Border, BorderImage, BorderImageLength, BorderImageRepeat, BorderStyle, CssImage,
        ImageResourceId,
    };
    let mut e = engine();
    let n = box_node(&mut e);
    let image = BorderImage {
        source: CssImage::Raster(ImageResourceId(7)),
        slice: std::array::from_fn(|_| BorderImageLength::Number(1.0)),
        fill: false,
        width: std::array::from_fn(|_| BorderImageLength::Number(1.0)),
        outset: std::array::from_fn(|_| BorderImageLength::Number(0.0)),
        repeat_x: BorderImageRepeat::Stretch,
        repeat_y: BorderImageRepeat::Stretch,
    };
    e.set_property(
        n,
        StyleProperty::BorderImage,
        StyleValue::Renderer(RendererStyleValue::BorderImage(Some(image))),
    )
    .unwrap();
    e.set_property(
        n,
        StyleProperty::Border,
        Border {
            width: 2.0,
            style: BorderStyle::Solid,
            color: Color::RED,
        }
        .into(),
    )
    .unwrap();
    assert!(e.computed_style(n).unwrap().border_image.is_none());
    assert!(e.remove_property(n, StyleProperty::BorderImage).unwrap());
    e.set_property(n, StyleProperty::Opacity, 0.5_f32.into())
        .unwrap();
    assert!(e.computed_style(n).unwrap().border_image.is_none());
    assert_eq!(e.computed_style(n).unwrap().border_top_width, 2);
}

#[test]
fn source_changes_and_removal_under_an_effect_restore_without_capturing_presentation() {
    for fill in [FillMode::Both, FillMode::Forwards, FillMode::None] {
        let mut e = engine();
        let n = box_node(&mut e);
        e.set_property(n, StyleProperty::Opacity, 0.4_f32.into())
            .unwrap();
        let id = opacity_effect(&mut e, n, fill);
        e.set_animation_time(75.0).unwrap();
        e.pause_animation(id).unwrap();
        assert_eq!(e.computed_style(n).unwrap().opacity, 0.8);
        e.set_property(n, StyleProperty::Opacity, 0.2_f32.into())
            .unwrap();
        assert_eq!(e.computed_style(n).unwrap().opacity, 0.8);
        assert!(e.remove_property(n, StyleProperty::Opacity).unwrap());
        assert_eq!(e.computed_style(n).unwrap().opacity, 0.8);
        e.cancel_animation(id).unwrap();
        assert_eq!(e.computed_style(n).unwrap().opacity, 1.0);
        assert!(!e.remove_property(n, StyleProperty::Opacity).unwrap());
    }
}

#[test]
fn no_fill_before_and_after_restore_surviving_raw_source() {
    let mut e = engine();
    let n = box_node(&mut e);
    e.set_property(n, StyleProperty::Opacity, 0.4_f32.into())
        .unwrap();
    let id = opacity_effect(&mut e, n, FillMode::None);
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.4);
    e.remove_property(n, StyleProperty::Opacity).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 1.0);
    e.set_animation_time(75.0).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.8);
    e.set_animation_time(200.0).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 1.0);
    e.cancel_animation(id).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 1.0);
}

#[test]
fn transition_targets_share_removable_source_and_invalid_requests_do_not_commit() {
    let mut e = engine();
    let n = box_node(&mut e);
    e.set_property(n, StyleProperty::Opacity, 0.2_f32.into())
        .unwrap();
    let before = e.dirty_generations();
    assert!(e
        .transition(
            n,
            StyleProperty::Opacity,
            0.9_f32,
            AnimationOptions {
                duration_ms: f64::NAN,
                ..AnimationOptions::default()
            }
        )
        .is_err());
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.2);
    assert_eq!(e.dirty_generations(), before);
    let id = e
        .transition(
            n,
            StyleProperty::Opacity,
            0.9_f32,
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
        )
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.2);
    e.remove_property(n, StyleProperty::Opacity).unwrap();
    e.cancel_animation(id).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 1.0);
}

#[test]
fn renderer_snapshot_freezes_unanimated_source_and_internal_and_derived_inputs_survive() {
    let mut e = engine();
    let root = e.root();
    e.set_property(root, StyleProperty::Color, Color::RED.into())
        .unwrap();
    let n = box_node(&mut e);
    e.set_property(n, StyleProperty::Opacity, 0.4_f32.into())
        .unwrap();
    let id = opacity_effect(&mut e, n, FillMode::Both);
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.8);
    e.set_renderer_style(
        n,
        RendererStyleValue::Width(openui_geometry::Length::px(42.0)),
    )
    .unwrap();
    e.set_property(root, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    e.cancel_animation(id).unwrap();
    assert_eq!(e.computed_style(n).unwrap().opacity, 0.4);
    assert_eq!(e.computed_style(n).unwrap().color, Color::RED);
    assert!(!e.remove_property(n, StyleProperty::Width).unwrap());
    e.set_property(n, StyleProperty::Width, LengthValue::px(70.0).into())
        .unwrap();
    e.remove_property(n, StyleProperty::Width).unwrap();
    assert_eq!(e.computed_style(n).unwrap().width.value(), 42.0);
    e.set_internal_style(n, RendererInternalStyleValue::EmbeddedDocumentText(true))
        .unwrap();
    e.set_property(n, StyleProperty::Height, LengthValue::px(3.0).into())
        .unwrap();
    e.remove_property(n, StyleProperty::Height).unwrap();
    assert!(e.computed_style(n).unwrap().embedded_document_text);
    let mut explicit = ComputedStyle::initial();
    explicit.update_derived(|s| {
        s.width = openui_geometry::Length::px(17.0);
        s.color = Color::GREEN;
    });
    e.install_derived_style(n, explicit).unwrap();
    assert!(!e.remove_property(n, StyleProperty::Width).unwrap());
    assert_eq!(e.computed_style(n).unwrap().width.value(), 17.0);
    e.set_property(root, StyleProperty::Color, Color::WHITE.into())
        .unwrap();
    assert_eq!(e.computed_style(n).unwrap().color, Color::GREEN);
}

#[test]
fn clone_copies_source_masks_without_effects_and_reused_slot_starts_fresh() {
    let mut e = engine();
    let root = e.root();
    let p = node(&mut e, root, ElementTag::Div);
    let q = node(&mut e, root, ElementTag::Div);
    e.set_property(p, StyleProperty::FontSize, LengthValue::px(10.0).into())
        .unwrap();
    e.set_property(q, StyleProperty::FontSize, LengthValue::px(20.0).into())
        .unwrap();
    let n = node(&mut e, p, ElementTag::Div);
    e.set_property(
        n,
        StyleProperty::Margin,
        Edges::all(LengthValue::Em(2.0)).into(),
    )
    .unwrap();
    e.set_property(n, StyleProperty::MarginLeft, LengthValue::px(3.0).into())
        .unwrap();
    e.remove_property(n, StyleProperty::MarginLeft).unwrap();
    let id = opacity_effect(&mut e, n, FillMode::Both);
    let clone = e.clone_subtree(n).unwrap();
    e.append_child(q, clone).unwrap();
    let s = e.computed_style(clone).unwrap();
    assert_eq!(
        (s.margin_top.value(), s.margin_left.value(), s.opacity),
        (40.0, 0.0, 1.0)
    );
    e.detach(clone).unwrap();
    e.append_child(p, clone).unwrap();
    assert_eq!(e.computed_style(clone).unwrap().margin_top.value(), 20.0);
    e.cancel_animation(id).unwrap();
    e.remove(clone).unwrap();
    let replacement = e.create_native_element(ElementTag::Div).unwrap();
    assert_eq!(replacement.index(), clone.index());
    assert_ne!(replacement.generation(), clone.generation());
    assert_eq!(
        e.computed_style(replacement).unwrap().margin_top.value(),
        0.0
    );
    assert!(!e
        .remove_property(replacement, StyleProperty::Margin)
        .unwrap());
    assert!(matches!(
        e.remove_property(clone, StyleProperty::Margin),
        Err(EngineError::StaleHandle)
    ));
}

#[test]
fn origin_inheritance_and_pseudo_masks_follow_source_resets() {
    let mut e = engine();
    let root = e.root();
    let n = node(&mut e, root, ElementTag::Div);
    e.set_property(n, StyleProperty::FontSize, LengthValue::px(10.0).into())
        .unwrap();
    let button = e
        .create_scroll_button(n, ScrollButtonDirection::Down)
        .unwrap();
    e.set_property(button, StyleProperty::Display, Display::None.into())
        .unwrap();
    e.remove_property(button, StyleProperty::Display).unwrap();
    assert_eq!(
        e.computed_style(button).unwrap().display,
        Display::InlineBlock
    );
    let pseudo = Style::default()
        .font_size(LengthValue::Em(2.0))
        .padding(Edges::all(LengthValue::Em(1.0)))
        .padding_left(LengthValue::px(7.0))
        .background_color_value(StyleColor::CurrentColor);
    e.set_pseudo_style(n, PseudoStyleTarget::FirstLine, &pseudo)
        .unwrap();
    e.set_property(n, StyleProperty::Color, Color::BLUE.into())
        .unwrap();
    e.set_property(n, StyleProperty::FontSize, LengthValue::px(20.0).into())
        .unwrap();
    assert_eq!(e.computed_style(button).unwrap().font_size, 20.0);
    let s = e
        .computed_style(n)
        .unwrap()
        .first_line_style
        .as_ref()
        .unwrap();
    assert_eq!(
        (s.font_size, s.padding_top.value(), s.padding_left.value()),
        (40.0, 40.0, 7.0)
    );
    assert_eq!(s.background_color, Color::BLUE);
    e.remove_property(n, StyleProperty::FontSize).unwrap();
    let inherited = e.computed_style(root).unwrap().font_size;
    assert_eq!(e.computed_style(button).unwrap().font_size, inherited);
    assert_eq!(
        e.computed_style(n)
            .unwrap()
            .first_line_style
            .as_ref()
            .unwrap()
            .font_size,
        inherited * 2.0
    );
}

#[test]
fn origin_owned_cascade_reaches_sibling_hidden_and_detached_button_descendants() {
    let mut e = engine();
    let root = e.root();
    let p = node(&mut e, root, ElementTag::Div);
    let q = node(&mut e, root, ElementTag::Div);
    e.set_property(p, StyleProperty::FontSize, LengthValue::px(10.0).into())
        .unwrap();
    e.set_property(q, StyleProperty::FontSize, LengthValue::px(20.0).into())
        .unwrap();
    let origin = node(&mut e, p, ElementTag::Div);
    let button = e
        .create_scroll_button(origin, ScrollButtonDirection::Down)
        .unwrap();
    assert_eq!(e.parent(button).unwrap(), Some(p));
    let child = node(&mut e, button, ElementTag::Span);
    e.set_property(child, StyleProperty::FontSize, LengthValue::Em(0.5).into())
        .unwrap();
    e.set_property(
        origin,
        StyleProperty::FontSize,
        LengthValue::px(24.0).into(),
    )
    .unwrap();
    assert_eq!(e.computed_style(button).unwrap().font_size, 24.0);
    assert_eq!(e.computed_style(child).unwrap().font_size, 12.0);
    e.remove_property(origin, StyleProperty::FontSize).unwrap();
    assert_eq!(e.computed_style(button).unwrap().font_size, 10.0);
    assert_eq!(e.computed_style(child).unwrap().font_size, 5.0);
    e.set_property(p, StyleProperty::FontSize, LengthValue::px(14.0).into())
        .unwrap();
    assert_eq!(e.computed_style(button).unwrap().font_size, 14.0);
    assert_eq!(e.computed_style(child).unwrap().font_size, 7.0);

    e.detach(origin).unwrap();
    assert_eq!(e.parent(button).unwrap(), None);
    e.set_property(
        origin,
        StyleProperty::FontSize,
        LengthValue::px(30.0).into(),
    )
    .unwrap();
    assert_eq!(e.computed_style(button).unwrap().font_size, 30.0);
    assert_eq!(e.computed_style(child).unwrap().font_size, 15.0);
    e.set_property(origin, StyleProperty::Display, Display::None.into())
        .unwrap();
    e.append_child(q, origin).unwrap();
    assert_eq!(e.parent(button).unwrap(), None);
    e.remove_property(origin, StyleProperty::FontSize).unwrap();
    assert_eq!(e.computed_style(button).unwrap().font_size, 20.0);
    assert_eq!(e.computed_style(child).unwrap().font_size, 10.0);
    e.set_property(q, StyleProperty::FontSize, LengthValue::px(22.0).into())
        .unwrap();
    assert_eq!(e.computed_style(button).unwrap().font_size, 22.0);
    assert_eq!(e.computed_style(child).unwrap().font_size, 11.0);
}

#[test]
fn native_control_adjustments_and_closed_details_are_not_source_inputs() {
    let mut e = engine();
    let root = e.root();
    let textarea = node(&mut e, root, ElementTag::TextArea);
    e.set_property(textarea, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    e.remove_property(textarea, StyleProperty::Overflow)
        .unwrap();
    assert_eq!(
        (
            e.computed_style(textarea).unwrap().overflow_x,
            e.computed_style(textarea).unwrap().overflow_y
        ),
        (Overflow::Auto, Overflow::Auto)
    );
    let fieldset = node(&mut e, root, ElementTag::Fieldset);
    let legend = node(&mut e, fieldset, ElementTag::Legend);
    let content = node(&mut e, fieldset, ElementTag::Div);
    e.set_property(
        fieldset,
        StyleProperty::Width,
        LengthValue::px(100.0).into(),
    )
    .unwrap();
    e.set_property(
        fieldset,
        StyleProperty::Height,
        LengthValue::px(80.0).into(),
    )
    .unwrap();
    e.set_property(fieldset, StyleProperty::Overflow, Overflow::Hidden.into())
        .unwrap();
    e.set_property(legend, StyleProperty::Height, LengthValue::px(10.0).into())
        .unwrap();
    e.set_property(content, StyleProperty::Height, LengthValue::px(20.0).into())
        .unwrap();
    e.scene().unwrap();
    assert_eq!(e.computed_style(fieldset).unwrap().display, Display::Block);
    e.remove_property(fieldset, StyleProperty::Overflow)
        .unwrap();
    assert_eq!(
        e.computed_style(fieldset).unwrap().overflow_x,
        Overflow::Visible
    );
    assert_eq!(e.computed_style(legend).unwrap().display, Display::Block);
    assert_eq!(e.bounds(legend).unwrap().unwrap().height, 10.0);
    let details = node(&mut e, root, ElementTag::Details);
    let summary = node(&mut e, details, ElementTag::Summary);
    let child = node(&mut e, details, ElementTag::Div);
    e.set_property(summary, StyleProperty::Height, LengthValue::px(5.0).into())
        .unwrap();
    e.set_property(child, StyleProperty::Height, LengthValue::px(7.0).into())
        .unwrap();
    e.scene().unwrap();
    assert_eq!(e.computed_style(child).unwrap().display, Display::Block);
    e.set_property(child, StyleProperty::Display, Display::InlineBlock.into())
        .unwrap();
    e.remove_property(child, StyleProperty::Display).unwrap();
    assert_eq!(e.computed_style(child).unwrap().display, Display::Block);
    e.set_attribute(details, "open", "").unwrap();
    assert_eq!(e.bounds(child).unwrap().unwrap().height, 7.0);
}

#[test]
fn missing_equal_and_invalid_operations_keep_idle_source_and_generation_state() {
    let mut e = engine();
    let n = box_node(&mut e);
    e.scene().unwrap();
    let before = e.stats();
    let dirty = e.dirty_generations();
    assert!(!e.remove_property(n, StyleProperty::MarginLeft).unwrap());
    e.set_property(n, StyleProperty::Width, LengthValue::px(80.0).into())
        .unwrap();
    assert!(matches!(
        e.set_property(n, StyleProperty::Width, StyleValue::Color(Color::RED)),
        Err(EngineError::PropertyType { .. })
    ));
    let mut foreign = engine();
    assert!(matches!(
        foreign.remove_property(n, StyleProperty::Width),
        Err(EngineError::WrongDocument)
    ));
    assert_eq!(e.computed_style(n).unwrap().width.value(), 80.0);
    e.scene().unwrap();
    assert_eq!(e.stats(), before);
    assert_eq!(e.dirty_generations(), dirty);
}

#[test]
fn transform_reset_under_live_effect_defers_containing_block_policy_until_cancel() {
    let mut e = engine();
    let owner = box_node(&mut e);
    e.set_property(
        owner,
        StyleProperty::MarginLeft,
        LengthValue::px(20.0).into(),
    )
    .unwrap();
    let child = node(&mut e, owner, ElementTag::Div);
    e.set_property(child, StyleProperty::Position, Position::Absolute.into())
        .unwrap();
    e.set_property(child, StyleProperty::Width, LengthValue::px(8.0).into())
        .unwrap();
    e.set_property(child, StyleProperty::Height, LengthValue::px(8.0).into())
        .unwrap();
    e.set_property(child, StyleProperty::Left, LengthValue::px(0.0).into())
        .unwrap();
    e.set_property(child, StyleProperty::Top, LengthValue::px(0.0).into())
        .unwrap();
    let identity = TransformList(vec![TransformOperation::Scale(1.0, 1.0)]);
    e.set_property(owner, StyleProperty::Transform, identity.clone().into())
        .unwrap();
    let id = e
        .animate(
            owner,
            PropertyKeyframes::typed(
                StyleProperty::Transform,
                Keyframes::from_values(identity.clone(), identity),
            )
            .unwrap(),
            AnimationOptions {
                duration_ms: 100.0,
                fill: FillMode::Both,
                ..AnimationOptions::default()
            },
            AnimationTimeline::Document,
        )
        .unwrap();
    let retained = e.bounds(child).unwrap().unwrap();
    let before = e.stats().layouts;
    assert!(e.remove_property(owner, StyleProperty::Transform).unwrap());
    assert_eq!(e.bounds(child).unwrap().unwrap(), retained);
    assert_eq!(e.stats().layouts, before);
    e.cancel_animation(id).unwrap();
    assert!(!e
        .computed_style(owner)
        .unwrap()
        .has_transform_containing_block());
    assert_eq!(e.bounds(child).unwrap().unwrap().x, 0.0);
    assert_eq!(e.stats().layouts, before + 1);
}

#[test]
fn cursor_query_uses_surviving_source_and_inherits_after_removal() {
    let mut e = engine();
    let root = e.root();
    let parent = node(&mut e, root, ElementTag::Div);
    let child = node(&mut e, parent, ElementTag::Div);
    assert_eq!(e.cursor(child).unwrap(), openui_style::Cursor::Auto);
    e.set_property(
        parent,
        StyleProperty::Cursor,
        openui_style::Cursor::Pointer.into(),
    )
    .unwrap();
    assert_eq!(e.cursor(child).unwrap(), openui_style::Cursor::Pointer);
    e.set_property(
        child,
        StyleProperty::Cursor,
        openui_style::Cursor::Auto.into(),
    )
    .unwrap();
    assert_eq!(e.cursor(child).unwrap(), openui_style::Cursor::Auto);
    assert!(e.remove_property(child, StyleProperty::Cursor).unwrap());
    assert_eq!(e.cursor(child).unwrap(), openui_style::Cursor::Pointer);
    assert!(!e.remove_property(child, StyleProperty::Cursor).unwrap());
    assert!(e.remove_property(parent, StyleProperty::Cursor).unwrap());
    assert_eq!(e.cursor(child).unwrap(), openui_style::Cursor::Auto);
}
