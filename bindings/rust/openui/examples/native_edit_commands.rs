//! Invoke native editing commands from consuming Rust application callbacks.

use openui::prelude::*;
use std::{cell::RefCell, error::Error, rc::Rc};

fn main() -> Result<(), Box<dyn Error>> {
    for tag in ["input", "textarea"] {
        let document = Document::with_font_collection(
            ViewportMetrics::from_logical_size(240.0, 100.0, 1.0)?,
            FontCollection::deterministic_test(),
        )?;
        let input = Element::create(&document, tag)?;
        let button = Element::create(&document, "button")?;
        let label = Element::create(&document, "div")?;
        input.set_width(LengthValue::px(100.0))?;
        input.set_height(LengthValue::px(30.0))?;
        input.set_control_value("á👩‍💻z")?;
        for element in [&input, &button, &label] {
            document.body().append_child(element)?;
        }
        let original = input.control_value()?;
        let bounds = input.bounding_rect()?;
        let states = Rc::new(RefCell::new(Vec::new()));
        let callbacks = states.clone();
        let weak_label = label.downgrade();
        input.on("input", move |event| {
            let value = event.target().unwrap().control_value().unwrap().unwrap();
            weak_label.upgrade().unwrap().set_text(&value).unwrap();
            callbacks.borrow_mut().push(value);
        })?;
        let weak_input = input.downgrade();
        button.on("click", move |_| {
            weak_input
                .upgrade()
                .unwrap()
                .edit_text(EditCommand::Delete {
                    direction: TextDirection::Backward,
                    unit: TextUnit::Grapheme,
                })
                .unwrap();
        })?;
        button.click()?;
        input.edit_text(EditCommand::Undo)?;
        input.edit_text(EditCommand::Redo)?;
        input.edit_text(EditCommand::SelectAll)?;
        assert_eq!(input.control_value()?.as_deref(), Some("á👩‍💻"));
        assert_eq!(input.selection()?, Some((0, "á👩‍💻".len())));
        assert_eq!(&*states.borrow(), &["á👩‍💻", "á👩‍💻z", "á👩‍💻"]);
        assert_eq!(label.text_content()?, "á👩‍💻");
        assert_eq!(original.as_deref(), Some("á👩‍💻z"));
        assert_eq!(bounds, input.bounding_rect()?);
        let bitmap = document.render_to_bitmap()?;
        assert_eq!(bitmap.pixels().len(), 240 * 100 * 4);
        println!("{tag}: callbacks=3, value=á👩‍💻, selection=0..14, geometry=unchanged");
        let weak = input.downgrade();
        drop(input);
        drop(button);
        drop(label);
        drop(document);
        assert!(weak.upgrade().is_none());
    }
    Ok(())
}
