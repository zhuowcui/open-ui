//! Select immutable raster options and mutate a retained native document.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let output = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        return Err("expected at most one output directory".into());
    }
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let selected = RasterConfiguration::deterministic_aliased(true);
        let document = Document::with_font_collection_and_options(
            ViewportMetrics::from_logical_size(64.0, 48.0, scale)?,
            FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration: selected,
            },
        )?;
        let root = document.body();
        root.set_background_color(Color::WHITE)?;
        let card = Element::create(&document, "div")?;
        card.set_width(LengthValue::px(20.0))?;
        card.set_height(LengthValue::px(20.0))?;
        card.set_border(Border {
            width: 4.0,
            style: BorderStyle::Solid,
            color: Color::BLACK,
        })?;
        card.set_background_color(Color::BLUE)?;
        root.append_child(&card)?;
        let saved_configuration = document.raster_configuration()?;
        assert_eq!(saved_configuration, selected);
        let before = card.bounding_rect()?.ok_or("native card bounds required")?;
        assert_eq!((before.width, before.height), (28.0, 28.0));
        let directory = output
            .as_ref()
            .map(|path| path.join(format!("scale-{scale}")));
        if let Some(path) = &directory {
            std::fs::create_dir_all(path)?;
            std::fs::write(path.join("before.png"), document.render_to_png_buffer()?)?;
        }
        let calls = Rc::new(Cell::new(0));
        let observed = Rc::clone(&calls);
        card.on("click", move |event| {
            let target = event.target().expect("live native event target");
            target.set_border_top_style(BorderStyle::None).unwrap();
            target.set_background_color(Color::RED).unwrap();
            observed.set(observed.get() + 1);
        })?;
        card.click()?;
        assert_eq!(calls.get(), 1);
        let after = card
            .bounding_rect()?
            .ok_or("mutated card bounds required")?;
        assert_eq!((after.width, after.height), (28.0, 24.0));
        assert_eq!(document.raster_configuration()?, saved_configuration);
        let first = document.render_to_png_buffer()?;
        assert_eq!(first, document.clone().render_to_png_buffer()?);
        if let Some(path) = &directory {
            std::fs::write(path.join("after.png"), first)?;
        }
        let weak = card.downgrade();
        drop(card);
        drop(root);
        drop(document);
        assert!(weak.upgrade().is_none());
        println!("native raster options: scale={scale} callback=1 configuration/bounds passed");
    }
    let options = EngineOptions {
        raster_configuration: RasterConfiguration::deterministic_aliased(false),
    };
    let app = App::builder()
        .engine_options(options)
        .backend(BackendPreference::Software)
        .build()?;
    assert_eq!(
        app.document().raster_configuration()?,
        options.raster_configuration
    );
    let headless = HeadlessApp::with_options(
        ViewportMetrics::from_logical_size(64.0, 48.0, 1.5)?,
        options,
    )?;
    headless
        .document()
        .body()
        .set_background_color(Color::WHITE)?;
    assert_eq!(
        headless.document().raster_configuration()?,
        options.raster_configuration
    );
    assert_eq!(headless.render_at(0.0)?, headless.render_at(0.0)?);
    Ok(())
}
