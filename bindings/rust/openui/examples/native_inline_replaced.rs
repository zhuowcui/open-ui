//! Inline image mutation through public Rust methods and callbacks.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).map(PathBuf::from);
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for phase in [0.0, 0.25, 0.5] {
            let document = Document::with_viewport_metrics(ViewportMetrics::from_logical_size(
                320.0, 240.0, scale,
            )?)?;
            let root = document.body();
            root.set_background_color(Color::WHITE)?;
            let parent = Element::create(&document, "div")?;
            parent.set_display(Display::Block)?;
            parent.set_position(Position::Absolute)?;
            parent.set_left(Length::px(20.0 + phase))?;
            parent.set_top(Length::px(20.0 + phase))?;
            parent.set_width(LengthValue::px(150.0))?;
            parent.set_height(LengthValue::px(150.0))?;
            root.append_child(&parent)?;
            let image = Element::create(&document, "img")?;
            let resource = document.register_image_resource(
                "memory:native-inline-green",
                "image/png",
                "d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe",
                include_bytes!("../tests/assets/green-200.png").to_vec(),
            )?;
            image.set_image_resource(resource, Some((200.0, 200.0)))?;
            image.set_display(Display::Inline)?;
            image.set_width(LengthValue::px(150.0))?;
            image.set_height(LengthValue::px(150.0))?;
            parent.append_child(&image)?;
            let before = image
                .bounding_rect()?
                .ok_or("inline image bounds required")?;
            assert_eq!(Some(before), parent.bounding_rect()?);
            let directory = output
                .as_ref()
                .map(|p| p.join(format!("scale-{scale}/phase-{phase}")));
            let save = |state: &str| -> Result<(), Box<dyn std::error::Error>> {
                if let Some(path) = &directory {
                    std::fs::create_dir_all(path)?;
                    std::fs::write(
                        path.join(format!("{state}.png")),
                        document.render_to_png_buffer()?,
                    )?;
                }
                Ok(())
            };
            save("before")?;
            let calls = Rc::new(Cell::new(0));
            let callback_calls = calls.clone();
            let weak = image.downgrade();
            let callback_image = weak.clone();
            image.on("click", move |_| {
                let image = callback_image
                    .upgrade()
                    .expect("native image remains attached");
                image.set_width(LengthValue::px(100.0)).unwrap();
                callback_calls.set(callback_calls.get() + 1);
            })?;
            image.click()?;
            assert_eq!(calls.get(), 1);
            let after = image
                .bounding_rect()?
                .ok_or("mutated image bounds required")?;
            assert_eq!(
                (after.x, after.y, after.width, after.height),
                (before.x, before.y, 100.0, 150.0)
            );
            assert_eq!(before.width, 150.0);
            save("after")?;
            image.set_display(Display::None)?;
            assert!(image.client_rects()?.is_empty());
            save("hidden")?;
            image.set_display(Display::Inline)?;
            assert_eq!(image.bounding_rect()?, Some(after));
            image.detach()?;
            assert!(image.client_rects()?.is_empty());
            parent.append_child(&image)?;
            assert_eq!(image.bounding_rect()?, Some(after));
            save("reattached")?;
            drop(image);
            drop(parent);
            drop(root);
            drop(document);
            assert!(
                weak.upgrade().is_none(),
                "callbacks must not retain the document"
            );
            println!("scale={scale} phase={phase} callback=1 owned-bounds/detach/teardown passed");
        }
    }
    Ok(())
}
