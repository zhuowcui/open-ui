//! Image and fallback transitions through public native Rust APIs.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).map(PathBuf::from);
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for phase in [0.0, 0.25, 0.5] {
            let document = Document::with_font_collection(
                ViewportMetrics::from_logical_size(320.0, 240.0, scale)?,
                FontCollection::deterministic_test(),
            )?;
            let root = document.body();
            root.set_background_color(Color::WHITE)?;
            root.set_font_family(FontFamilyList {
                families: vec![FontFamily::Named("Ahem".into())],
            })?;
            root.set_font_size(LengthValue::px(16.0))?;
            root.set_line_height(LineHeight::Length(20.0))?;
            let cover = Element::create(&document, "div")?;
            cover.set_position(Position::Absolute)?;
            cover.set_left(Length::px(20.0 + phase))?;
            cover.set_top(Length::px(20.0 + phase))?;
            cover.set_width(LengthValue::px(100.0))?;
            cover.set_height(LengthValue::px(100.0))?;
            cover.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
            cover.set_z_index(1)?;
            root.append_child(&cover)?;
            let parent = Element::create(&document, "div")?;
            parent.set_position(Position::Absolute)?;
            parent.set_left(Length::px(20.0 + phase))?;
            parent.set_top(Length::px(20.0 + phase))?;
            parent.set_width(LengthValue::px(100.0))?;
            parent.set_height(LengthValue::px(100.0))?;
            root.append_child(&parent)?;
            let image = Element::create(&document, "img")?;
            image.set_display(Display::Inline)?;
            image.set_width(LengthValue::px(300.0))?;
            image.set_height(LengthValue::px(200.0))?;
            image.set_background_color(Color::RED)?;
            image.set_color(Color::RED)?;
            image.set_column_count(Some(5))?;
            image.set_orphans(1)?;
            image.set_widows(1)?;
            image.set_attribute("alt", "XXXXX XXXXX XXXXX XXXXX XXXXX")?;
            image.set_attribute("src", "invalid.jpg")?;
            let icon = Element::create(&document, "canvas")?;
            icon.set_display(Display::InlineBlock)?;
            icon.set_width(LengthValue::px(16.0))?;
            icon.set_height(LengthValue::px(16.0))?;
            icon.set_vertical_align(VerticalAlign::Length(-3.0))?;
            image.append_child(&icon)?;
            image.append_text_node("XXXXX XXXXX XXXXX XXXXX XXXXX")?;
            parent.append_child(&image)?;
            let resource = document.register_image_resource(
                "memory:native-inline-green",
                "image/png",
                "d49ce16b513fa1b4fcf1431bc2915799ee8effb452820f35e9e6fba251e8cafe",
                include_bytes!("../tests/assets/green-200.png").to_vec(),
            )?;
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
            let before = image.bounding_rect()?.ok_or("fallback bounds required")?;
            save("before")?;
            let calls = Rc::new(Cell::new(0));
            let callback_calls = calls.clone();
            let weak = image.downgrade();
            let callback_image = weak.clone();
            image.on("click", move |_| {
                let image = callback_image
                    .upgrade()
                    .expect("retained image remains live");
                if callback_calls.get() == 0 {
                    image
                        .set_image_resource(resource, Some((200.0, 200.0)))
                        .unwrap();
                } else {
                    image.clear_image_resource().unwrap();
                }
                callback_calls.set(callback_calls.get() + 1);
            })?;
            image.click()?;
            assert_eq!(calls.get(), 1);
            let loaded = image.bounding_rect()?.ok_or("image bounds required")?;
            assert_eq!((loaded.width, loaded.height), (300.0, 200.0));
            save("loaded")?;
            image.click()?;
            assert_eq!(calls.get(), 2);
            assert_eq!(image.bounding_rect()?, Some(before));
            save("cleared")?;
            image.clear_image_resource()?;
            assert_eq!(image.bounding_rect()?, Some(before));
            assert_eq!(image.text_content()?, "XXXXX XXXXX XXXXX XXXXX XXXXX");
            image.detach()?;
            assert!(image.client_rects()?.is_empty());
            parent.append_child(&image)?;
            assert_eq!(image.bounding_rect()?, Some(before));
            save("reattached")?;
            for (state, rect) in [
                ("before", before),
                ("loaded", loaded),
                ("cleared", before),
                ("reattached", before),
            ] {
                println!(
                    "scale={scale} phase={phase} state={state} x={} y={} width={} height={}",
                    rect.x, rect.y, rect.width, rect.height
                );
            }
            drop(icon);
            drop(image);
            drop(cover);
            drop(parent);
            drop(root);
            drop(document);
            assert!(
                weak.upgrade().is_none(),
                "callbacks must not retain the document"
            );
        }
    }
    Ok(())
}
