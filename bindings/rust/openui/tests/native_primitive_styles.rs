use openui::prelude::*;
use openui::typed_style::parse_literal;
use std::{cell::Cell, rc::Rc};

fn native_card(scale: f64) -> (Document, Element) {
    let document = Document::with_viewport_metrics(
        ViewportMetrics::from_logical_size(320.0, 240.0, scale).unwrap(),
    )
    .unwrap();
    let root = document.body();
    root.set_overflow(Overflow::Hidden).unwrap();
    root.set_background_color(Color::WHITE).unwrap();
    let card = Element::create(&document, "div").unwrap();
    card.set_position(Position::Absolute).unwrap();
    card.set_display(Display::Block).unwrap();
    card.set_width(LengthValue::px(80.0)).unwrap();
    card.set_height(LengthValue::px(40.0)).unwrap();
    card.set_font_size(LengthValue::px(20.0)).unwrap();
    card.set_color(Color::GREEN).unwrap();
    card.set_background_color(Color::RED).unwrap();
    card.set_border(Border {
        width: 1.0,
        style: BorderStyle::Solid,
        color: Color::BLACK,
    })
    .unwrap();
    root.append_child(&card).unwrap();
    (document, card)
}

#[test]
fn native_author_values_match_typed_app_state_and_pixels_through_callbacks() {
    // Two consuming applications use the public author-value and typed paths.
    // The reference is API equivalence, not a Chromium qualification claim.
    use RendererStyleValue as R;
    let values = [
        (
            "safe end",
            R::AlignContent(ContentAlignment {
                position: ContentPosition::End,
                distribution: ContentDistribution::Default,
                overflow: OverflowAlignment::Safe,
            }),
        ),
        (
            "unsafe center",
            R::AlignSelf(ItemAlignment::with_overflow(
                ItemPosition::Center,
                OverflowAlignment::Unsafe,
            )),
        ),
        (
            "currentcolor",
            R::BorderBottomColor(StyleColor::CurrentColor),
        ),
        ("1", R::BorderBottomWidth(1)),
        (
            "#123456",
            R::BorderLeftColor(StyleColor::Resolved(Color::from_rgba8(
                0x12, 0x34, 0x56, 255,
            ))),
        ),
        ("2", R::BorderLeftWidth(2)),
        (
            "blue",
            R::BorderRightColor(StyleColor::Resolved(Color::BLUE)),
        ),
        ("3", R::BorderRightWidth(3)),
        ("red", R::BorderTopColor(StyleColor::Resolved(Color::RED))),
        ("4", R::BorderTopWidth(4)),
        ("auto", R::Bottom(Length::auto())),
        ("2em", R::ColumnHeight(Some(Length::px(40.0)))),
        ("currentcolor", R::ColumnRuleColor(StyleColor::CurrentColor)),
        ("2", R::ColumnRuleWidth(2)),
        ("96px", R::ColumnWidth(Some(Length::px(96.0)))),
        ("1.25", R::FilterBlur(1.25)),
        ("0.4", R::FilterGrayscale(0.4)),
        (
            "self-end",
            R::JustifyItems(ItemAlignment::new(ItemPosition::SelfEnd)),
        ),
        (
            "last baseline",
            R::JustifySelf(ItemAlignment::new(ItemPosition::LastBaseline)),
        ),
        ("2em", R::Left(Length::px(40.0))),
        ("-7", R::Order(-7)),
        ("4294967295", R::Orphans(u32::MAX)),
        ("currentcolor", R::OutlineColor(StyleColor::CurrentColor)),
        ("-2", R::OutlineOffset(-2)),
        ("2.5", R::OverflowClipMargin(2.5)),
        ("clip", R::OverflowX(Overflow::Clip)),
        ("hidden", R::OverflowY(Overflow::Hidden)),
        ("auto", R::Right(Length::auto())),
        ("auto", R::ScrollbarTrackColor(None)),
        ("10vh", R::Top(Length::px(24.0))),
        ("3", R::Widows(3)),
        ("2", R::OutlineWidth(2)),
        (
            "#123456",
            R::ScrollbarThumbColor(Some(Color::from_rgba8(0x12, 0x34, 0x56, 255))),
        ),
        ("0.25", R::ShapeImageThreshold(0.25)),
        ("1.5rem", R::ShapeMargin(Length::px(24.0))),
    ];
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let (document, card) = native_card(scale);
        let (reference_document, reference_card) = native_card(scale);
        for (literal, expected) in &values {
            let property = expected.property();
            card.set_property(property, parse_literal(property, literal).unwrap())
                .unwrap();
            reference_card
                .set_property(property, StyleValue::Renderer(expected.clone()))
                .unwrap();
        }
        let original = card.computed_style().unwrap();
        let original_bounds = card.bounding_rect().unwrap().unwrap();
        assert_eq!((original_bounds.x, original_bounds.y), (40.0, 24.0));
        assert_eq!(
            (original_bounds.width, original_bounds.height),
            (85.0, 45.0)
        );
        assert_eq!(
            format!("{original:?}"),
            format!("{:?}", reference_card.computed_style().unwrap())
        );
        let original_pixels = document.render_to_bitmap().unwrap().pixels().to_vec();
        assert_eq!(
            original_pixels,
            reference_document
                .render_to_bitmap()
                .unwrap()
                .pixels()
                .to_vec()
        );

        let calls = Rc::new(Cell::new(0));
        let callback_calls = calls.clone();
        let weak = card.downgrade();
        let callback_card = weak.clone();
        card.on("click", move |_| {
            let card = callback_card.upgrade().unwrap();
            card.set_background_color(Color::BLUE).unwrap();
            for (property, literal) in [
                (StyleProperty::Left, "4em"),
                (StyleProperty::FilterBlur, "0"),
                (StyleProperty::FilterGrayscale, "0"),
                (StyleProperty::ColumnWidth, "auto"),
                (StyleProperty::ScrollbarTrackColor, "green"),
            ] {
                card.set_property(property, parse_literal(property, literal).unwrap())
                    .unwrap();
            }
            callback_calls.set(callback_calls.get() + 1);
        })
        .unwrap();
        card.click().unwrap();
        reference_card.set_background_color(Color::BLUE).unwrap();
        reference_card.set_left(Length::px(80.0)).unwrap();
        reference_card.set_filter_blur(0.0).unwrap();
        reference_card.set_filter_grayscale(0.0).unwrap();
        reference_card.set_column_width(None).unwrap();
        reference_card
            .set_scrollbar_track_color(Some(Color::GREEN))
            .unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(card.bounding_rect().unwrap().unwrap().x, 80.0);
        assert_eq!(
            format!("{:?}", card.computed_style().unwrap()),
            format!("{:?}", reference_card.computed_style().unwrap())
        );
        let changed_pixels = document.render_to_bitmap().unwrap().pixels().to_vec();
        let sample =
            (40.0 * scale) as usize * (320.0 * scale) as usize * 4 + (100.0 * scale) as usize * 4;
        assert_eq!(&changed_pixels[sample..sample + 4], &[0, 0, 255, 255]);
        assert_ne!(original_pixels, changed_pixels);
        assert_eq!(
            changed_pixels,
            reference_document
                .render_to_bitmap()
                .unwrap()
                .pixels()
                .to_vec()
        );
        assert_eq!(original.left, Length::px(40.0));
        assert_eq!(original.filter_blur, 1.25);
        assert_eq!(original.scrollbar_track_color, None);
        drop(card);
        drop(document);
        assert!(weak.upgrade().is_none());
        assert_eq!(original.orphans, u32::MAX);
    }
}
