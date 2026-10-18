//! Native Rust consumer paired with the C/C++ floating-point color consumers.
use openui::prelude::*;
use openui_style::StyleColor;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(64.0, 64.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::BLACK)?;
    root.set_overflow(Overflow::Hidden)?;
    let card = Element::create(&document, "div")?;
    card.set_width(LengthValue::px(32.0))?;
    card.set_height(LengthValue::px(32.0))?;
    root.append_child(&card)?;
    let color = Color::from_rgba_f32(0.123456, 0.234567, 0.345678, 0.5);
    card.set_background_color(color)?;
    card.set_color(color)?;
    card.set_border_top_color(StyleColor::Resolved(color))?;
    card.set_border_right_color(StyleColor::Resolved(color))?;
    card.set_border_bottom_color(StyleColor::Resolved(color))?;
    card.set_border_left_color(StyleColor::Resolved(color))?;
    card.set_column_rule_color(StyleColor::Resolved(color))?;
    card.set_outline_color(StyleColor::Resolved(color))?;
    card.set_scrollbar_track_color(Some(color))?;
    card.set_scrollbar_thumb_color(Some(color))?;
    card.set_text_decoration_color(StyleColor::Resolved(color))?;
    card.set_text_emphasis_color(StyleColor::Resolved(color))?;
    let snapshot = card.computed_style()?;
    assert_eq!(snapshot.background_color, color);
    assert_eq!(snapshot.color, color);
    for resolved in [
        snapshot.border_top_color.resolve(&color),
        snapshot.border_right_color.resolve(&color),
        snapshot.border_bottom_color.resolve(&color),
        snapshot.border_left_color.resolve(&color),
        snapshot.column_rule_color.resolve(&color),
        snapshot.outline_color.resolve(&color),
        snapshot.scrollbar_track_color.unwrap(),
        snapshot.scrollbar_thumb_color.unwrap(),
        snapshot.text_decoration_color.resolve(&color),
        snapshot.text_emphasis_color.resolve(&color),
    ] {
        assert_eq!(resolved, color);
    }
    card.set_background_color(Color::from_rgba_f32(1.0, 0.0, 0.0, 0.5))?;
    let before = document.render_to_png_buffer()?;
    let count = Rc::new(Cell::new(0));
    let callbacks = count.clone();
    let callback_card = card.downgrade();
    card.on("click", move |_| {
        callback_card
            .upgrade()
            .expect("attached card")
            .set_background_color(Color::from_rgba_f32(0.0, 0.0, 1.0, 0.5))
            .expect("native callback color mutation");
        callbacks.set(callbacks.get() + 1);
    })?;
    card.click()?;
    assert_eq!(count.get(), 1);
    let after = document.render_to_png_buffer()?;
    assert_ne!(before, after);
    let weak = card.downgrade();
    drop(card);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none());
    assert_eq!(snapshot.background_color, color);
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("before.png"), before)?;
    std::fs::write(output.join("after.png"), after)?;
    println!(
        "native Rust float colors: scale={scale} properties=12 callback=1 owned-snapshot=1 passed"
    );
    Ok(())
}
