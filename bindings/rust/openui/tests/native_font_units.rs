use openui::prelude::*;
use std::{cell::Cell, rc::Rc};

// Expected bounds are the independently repeated pinned Chromium observations
// preserved in native-font-relative-v1. The application retains the unit; the
// callback changes only the font, without computing or setting a pixel width.
#[test]
fn retained_font_units_update_after_native_rust_callbacks() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for (unit, line_height, before_width, after_width) in [
            (LengthValue::Ch(2.5), LineHeight::Normal, 47.5, 50.0),
            (
                LengthValue::Ex(2.5),
                LineHeight::Normal,
                37.40625,
                40.734375,
            ),
            (
                LengthValue::Lh(2.5),
                LineHeight::Number(1.2),
                56.125,
                61.046875,
            ),
            (
                LengthValue::Lh(2.5),
                LineHeight::Percentage(127.5),
                59.421875,
                64.671875,
            ),
        ] {
            let document = Document::with_font_collection_and_options(
                ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
                FontCollection::deterministic_test(),
                EngineOptions {
                    raster_configuration: RasterConfiguration::deterministic_aliased(false),
                },
            )
            .unwrap();
            let parent = document.body();
            parent
                .set_font_family(FontFamilyList::single("Ahem"))
                .unwrap();
            parent.set_font_size(LengthValue::px(18.72)).unwrap();
            let element = Element::create(&document, "div").unwrap();
            parent.append_child(&element).unwrap();
            element.set_position(Position::Absolute).unwrap();
            element.set_left(Length::px(0.0)).unwrap();
            element.set_top(Length::px(0.0)).unwrap();
            element.set_height(LengthValue::px(10.0)).unwrap();
            // Declare the width before the final line-height context.
            element.set_width(unit).unwrap();
            element.set_line_height(line_height).unwrap();
            let owned_style = element.computed_style().unwrap();
            let owned_bounds = element.bounding_rect().unwrap().unwrap();
            assert_eq!(owned_bounds.width, before_width, "{unit:?} at {scale}");
            let weak = element.downgrade();
            let target = parent.downgrade();
            let calls = Rc::new(Cell::new(0));
            let called = calls.clone();
            element
                .on("click", move |_| {
                    target
                        .upgrade()
                        .unwrap()
                        .set_font_size(LengthValue::px(20.37))
                        .unwrap();
                    called.set(called.get() + 1);
                })
                .unwrap();
            element.click().unwrap();
            assert_eq!(
                element.bounding_rect().unwrap().unwrap().width,
                after_width,
                "{unit:?} at {scale}"
            );
            assert_eq!(calls.get(), 1);
            assert_eq!(owned_style.font_size, 18.72);
            assert_eq!(owned_bounds.width, before_width);
            assert_eq!(
                document.render_to_png_buffer().unwrap(),
                document.render_to_png_buffer().unwrap()
            );
            drop(element);
            drop(parent);
            drop(document);
            assert!(weak.upgrade().is_none());
        }
    }
}

#[test]
fn retained_font_units_refresh_after_shared_font_registration() {
    let fonts = FontCollection::deterministic_test();
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(800.0, 600.0, 1.0).unwrap(),
        fonts.clone(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )
    .unwrap();
    let element = Element::create(&document, "div").unwrap();
    document.body().append_child(&element).unwrap();
    element
        .set_font_family(FontFamilyList::single("Retained unit font"))
        .unwrap();
    element.set_font_size(LengthValue::px(20.0)).unwrap();
    element.set_width(LengthValue::Ch(2.0)).unwrap();
    let before = element.computed_style().unwrap();
    // Register through the shared collection rather than this Document.
    let face = fonts
        .register(
            std::sync::Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice()),
            FontFaceDescriptor::new("Retained unit font"),
        )
        .unwrap();
    assert_eq!(element.computed_style().unwrap().width.value(), 40.0);
    assert_eq!(element.bounding_rect().unwrap().unwrap().width, 40.0);
    assert_ne!(before.width.value(), 40.0);
    fonts.unregister(face).unwrap();
    assert_eq!(element.computed_style().unwrap().width, before.width);
}
