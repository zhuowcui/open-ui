//! Use typed fragment styles and retained mutation from a consuming Rust app.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let output = arguments.next().map(PathBuf::from);
    if arguments.next().is_some() {
        return Err("expected at most one output directory".into());
    }
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        let document = Document::with_font_collection(
            ViewportMetrics::from_logical_size(64.0, 48.0, scale)?,
            FontCollection::deterministic_test(),
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
        card.set_column_fill(ColumnFill::Auto)?;
        card.set_column_wrap(ColumnWrap::NoWrap)?;
        card.set_column_span(ColumnSpan::None)?;
        card.set_break_inside(BreakInside::Avoid)?;
        card.set_break_before(BreakValue::AvoidColumn)?;
        card.set_break_after(BreakValue::AvoidPage)?;
        card.set_box_decoration_break(BoxDecorationBreak::Slice)?;
        root.append_child(&card)?;
        let before_style = card.computed_style()?;
        let before = card.bounding_rect()?.ok_or("native card bounds required")?;
        assert_eq!(
            (before.x, before.y, before.width, before.height),
            (0.0, 0.0, 28.0, 28.0)
        );
        assert_eq!(*before_style.column_fill(), ColumnFill::Auto);
        assert_eq!(*before_style.break_inside(), BreakInside::Avoid);
        let directory = output
            .as_ref()
            .map(|path| path.join(format!("scale-{scale}")));
        if let Some(path) = &directory {
            std::fs::create_dir_all(path)?;
            std::fs::write(path.join("before.png"), document.render_to_png_buffer()?)?;
        }
        let calls = Rc::new(Cell::new(0));
        let observed = calls.clone();
        card.on("click", move |event: &Event| {
            let target = event.target().expect("live native target");
            target.set_column_fill(ColumnFill::Balance).unwrap();
            target.set_break_inside(BreakInside::Auto).unwrap();
            target.set_border_top_style(BorderStyle::None).unwrap();
            target.set_background_color(Color::RED).unwrap();
            observed.set(observed.get() + 1);
        })?;
        card.click()?;
        assert_eq!(calls.get(), 1);
        let after = card
            .bounding_rect()?
            .ok_or("mutated native card bounds required")?;
        assert_eq!(
            (after.x, after.y, after.width, after.height),
            (0.0, 0.0, 28.0, 24.0)
        );
        assert_eq!(card.client_rects()?, vec![after]);
        assert_eq!(*card.computed_style()?.column_fill(), ColumnFill::Balance);
        assert_eq!(*card.computed_style()?.break_inside(), BreakInside::Auto);
        assert_eq!(*before_style.column_fill(), ColumnFill::Auto);
        assert_eq!(*before_style.border_top_style(), BorderStyle::Solid);
        assert_eq!(before.height, 28.0);
        if let Some(path) = &directory {
            std::fs::write(path.join("after.png"), document.render_to_png_buffer()?)?;
        }
        let weak = card.downgrade();
        drop(card);
        drop(root);
        drop(document);
        assert!(weak.upgrade().is_none());
        println!(
            "native Rust fragment keywords: scale={scale} callback=1 owned-styles/bounds passed"
        );
    }
    Ok(())
}
