use openui::prelude::*;
use std::cell::Cell;
use std::rc::Rc;

#[test]
fn native_raster_selection_survives_callbacks_cloning_and_resize() {
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let selected = RasterConfiguration::deterministic_aliased(true);
        let document = Document::with_font_collection_and_options(
            ViewportMetrics::from_logical_size(32.0, 24.0, scale).unwrap(),
            FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration: selected,
            },
        )
        .unwrap();
        let copy = document.clone();
        let original = document.raster_configuration().unwrap();
        let root = document.body();
        root.set_background_color(Color::WHITE).unwrap();
        let card = Element::create(&document, "div").unwrap();
        card.set_width(LengthValue::px(8.0)).unwrap();
        card.set_height(LengthValue::px(8.0)).unwrap();
        card.set_background_color(Color::RED).unwrap();
        root.append_child(&card).unwrap();
        let before = card.bounding_rect().unwrap().unwrap();
        let before_pixels = document.render_to_png_buffer().unwrap();
        let calls = Rc::new(Cell::new(0));
        let count = Rc::clone(&calls);
        card.on("click", move |event| {
            event
                .target()
                .unwrap()
                .set_background_color(Color::BLUE)
                .unwrap();
            count.set(count.get() + 1);
        })
        .unwrap();
        card.click().unwrap();
        assert_eq!(calls.get(), 1);
        document
            .set_viewport(ViewportMetrics::from_logical_size(48.0, 32.0, scale).unwrap())
            .unwrap();
        assert_eq!(card.bounding_rect().unwrap().unwrap(), before);
        assert_eq!(document.raster_configuration().unwrap(), selected);
        assert_eq!(copy.raster_configuration().unwrap(), original);
        let frame = copy.render_to_bitmap().unwrap();
        assert_eq!(
            (frame.width(), frame.height()),
            ((48.0 * scale) as u32, (32.0 * scale) as u32)
        );
        let offset = frame.stride() + 4;
        assert_eq!(&frame.pixels()[offset..offset + 4], &[0, 0, 255, 255]);
        let after_pixels = document.render_to_png_buffer().unwrap();
        assert_ne!(before_pixels, after_pixels);
        assert_eq!(after_pixels, copy.render_to_png_buffer().unwrap());
        let weak = card.downgrade();
        drop(card);
        drop(root);
        drop(copy);
        drop(document);
        assert!(weak.upgrade().is_none());
    }
}

#[test]
fn default_native_constructors_preserve_rendering_and_configuration() {
    let viewport = ViewportMetrics::from_logical_size(32.0, 24.0, 1.25).unwrap();
    let fonts = FontCollection::deterministic_test();
    let existing = Document::with_font_collection(viewport, fonts.clone()).unwrap();
    let explicit =
        Document::with_font_collection_and_options(viewport, fonts, EngineOptions::default())
            .unwrap();
    for document in [&existing, &explicit] {
        let root = document.body();
        root.set_background_color(Color::WHITE).unwrap();
        root.set_font_family(FontFamilyList::single("Ahem"))
            .unwrap();
        root.set_font_size(LengthValue::px(16.0)).unwrap();
        root.set_text("X").unwrap();
        assert_eq!(
            document.raster_configuration().unwrap(),
            RasterConfiguration::default()
        );
    }
    assert_eq!(
        existing.render_to_png_buffer().unwrap(),
        explicit.render_to_png_buffer().unwrap()
    );
}

#[test]
fn window_and_headless_apps_keep_explicit_native_raster_selection() {
    let selected = RasterConfiguration::deterministic_aliased(false);
    let options = EngineOptions {
        raster_configuration: selected,
    };
    let app = App::builder()
        .engine_options(options)
        .backend(BackendPreference::Software)
        .build()
        .unwrap();
    assert_eq!(app.document().raster_configuration().unwrap(), selected);
    let headless = HeadlessApp::with_options(
        ViewportMetrics::from_logical_size(32.0, 24.0, 1.5).unwrap(),
        options,
    )
    .unwrap();
    let root = headless.document().body();
    root.set_background_color(Color::RED).unwrap();
    let first = headless.render_at(0.0).unwrap();
    let second = headless.render_at(0.0).unwrap();
    assert_eq!(
        headless.document().raster_configuration().unwrap(),
        selected
    );
    assert_eq!((first.width(), first.height()), (48, 36));
    assert_eq!(first.pixels(), second.pixels());
    assert_eq!(&first.pixels()[..4], &[255, 0, 0, 255]);
}

#[test]
fn selecting_ganesh_does_not_fall_back_to_cpu_raster() {
    let selected = RasterConfiguration::chromium_linux_ganesh();
    let document = Document::with_options(
        ViewportMetrics::from_logical_size(32.0, 24.0, 1.0).unwrap(),
        EngineOptions {
            raster_configuration: selected,
        },
    )
    .unwrap();
    document.body().set_background_color(Color::RED).unwrap();
    assert!(document.render_to_bitmap().is_err());
    assert_eq!(document.raster_configuration().unwrap(), selected);
}
