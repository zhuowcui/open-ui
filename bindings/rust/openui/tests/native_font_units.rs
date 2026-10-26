use openui::prelude::*;
use std::{cell::Cell, rc::Rc};

// Expected bounds are the independently repeated pinned Chromium observations
// preserved in native-font-relative-v1. The application retains the unit; the
// callback changes only the font, without computing or setting a pixel width.
// Windows values are pinned Windows Chrome-for-Testing observations in
// docs/progress/evidence/windows-text-oracle-v1.json.
#[test]
fn retained_font_units_update_after_native_rust_callbacks() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for (unit, line_height, before_width, after_width) in [
            (LengthValue::Ch(2.5), LineHeight::Normal, 47.5, 50.0),
            (
                LengthValue::Ex(2.5),
                LineHeight::Normal,
                if cfg!(windows) { 37.484375 } else { 37.40625 },
                if cfg!(windows) { 39.984375 } else { 40.734375 },
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

// The aliased raster configuration models Chromium text whose Fontconfig
// disables antialiasing: Linux Chromium then resolves full hinting and
// rounds each HarfBuzz advance to a whole pixel, so nine 16px Ahem glyphs
// span exactly 144px (docs/progress/aliased-text-whole-pixel-advances-2026-10-25.md).
// The earlier pinned observation of 144.015625 (native-general-font-chromium-v3824)
// loaded Ahem through @font-face, which Blink renders with the default
// antialiased style instead. Windows values are pinned Windows
// Chrome-for-Testing observations in
// docs/progress/evidence/windows-text-oracle-v1.json. The fixed-width float
// neighbor also checks that text rounding does not inflate unrelated atomic
// contents.
#[test]
fn atomic_intrinsic_text_keeps_chromium_fraction_after_native_font_alias_change() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let fonts = FontCollection::deterministic_test();
        fonts
            .register(
                std::sync::Arc::from(include_bytes!("../../openui-text/fonts/Ahem.ttf").as_slice()),
                FontFaceDescriptor::new("Native Ahem Alias"),
            )
            .unwrap();
        let document = Document::with_font_collection_and_options(
            ViewportMetrics::from_logical_size(800.0, 600.0, scale).unwrap(),
            fonts,
            EngineOptions {
                raster_configuration: RasterConfiguration::deterministic_aliased(false),
            },
        )
        .unwrap();
        let root = document.body();
        let container = Element::create(&document, "div").unwrap();
        root.append_child(&container).unwrap();
        container.set_position(Position::Absolute).unwrap();
        container.set_left(Length::px(10.0)).unwrap();
        container.set_top(Length::px(485.0)).unwrap();
        container.set_width(LengthValue::px(75.0)).unwrap();
        let word = Element::create(&document, "span").unwrap();
        container.append_child(&word).unwrap();
        word.set_display(Display::InlineBlock).unwrap();
        word.set_font_size(LengthValue::px(16.0)).unwrap();
        word.set_font_family(FontFamilyList::single("Ahem"))
            .unwrap();
        word.set_text("ABCDEFGHI").unwrap();
        let owned = word.bounding_rect().unwrap().unwrap();
        let raster_policy = RasterConfiguration::deterministic_aliased(false);
        assert_eq!(
            word.computed_style().unwrap().raster_configuration,
            raster_policy
        );
        assert_eq!(
            (owned.x, owned.y, owned.width, owned.height),
            (
                10.0,
                487.0,
                144.0,
                16.0
            ),
            "word at {scale}"
        );
        let float_container = Element::create(&document, "div").unwrap();
        root.append_child(&float_container).unwrap();
        float_container.set_position(Position::Absolute).unwrap();
        float_container.set_left(Length::px(170.0)).unwrap();
        float_container.set_top(Length::px(485.0)).unwrap();
        float_container.set_width(LengthValue::px(75.0)).unwrap();
        let atomic = Element::create(&document, "span").unwrap();
        float_container.append_child(&atomic).unwrap();
        atomic.set_display(Display::InlineBlock).unwrap();
        for _ in 0..2 {
            let child = Element::create(&document, "div").unwrap();
            atomic.append_child(&child).unwrap();
            child.set_float(Float::Left).unwrap();
            child.set_width(LengthValue::px(90.0)).unwrap();
            child.set_height(LengthValue::px(20.0)).unwrap();
        }
        let floats = atomic.bounding_rect().unwrap().unwrap();
        assert_eq!(
            (floats.x, floats.y, floats.width, floats.height),
            (170.0, 485.0, 90.0, 40.0),
            "floats at {scale}"
        );
        let before_alias_pixels = document.render_to_png_buffer().unwrap();
        let target = word.downgrade();
        let calls = Rc::new(Cell::new(0));
        let called = calls.clone();
        root.on("click", move |_| {
            target
                .upgrade()
                .unwrap()
                .set_font_family(FontFamilyList::single("Native Ahem Alias"))
                .unwrap();
            called.set(called.get() + 1);
        })
        .unwrap();
        root.click().unwrap();
        assert_eq!(calls.get(), 1);
        assert_eq!(word.bounding_rect().unwrap().unwrap(), owned);
        assert_eq!(
            word.computed_style().unwrap().raster_configuration,
            raster_policy
        );
        assert_eq!(atomic.bounding_rect().unwrap().unwrap(), floats);
        // A different native family alias for the same owned face changes
        // neither the explicitly selected raster policy nor its actual pixels.
        // Capture after adding the unchanged neighboring float geometry.
        let after_alias_pixels = document.render_to_png_buffer().unwrap();
        assert_eq!(after_alias_pixels, before_alias_pixels);
        assert_eq!(after_alias_pixels, document.render_to_png_buffer().unwrap());
    }
}
