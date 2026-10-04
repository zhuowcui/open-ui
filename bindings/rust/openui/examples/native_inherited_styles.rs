//! A consuming app that relies on inherited styles through public Rust APIs.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(128.0, 128.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    root.set_color(Color::RED)?;
    root.set_font_size(LengthValue::px(20.0))?;
    let child = Element::create(&document, "div")?;
    child.set_width(LengthValue::Em(4.0))?;
    child.set_height(LengthValue::px(32.0))?;
    child.set_border_top_width(LengthValue::px(4.0))?;
    child.set_border_top_style(BorderStyle::Solid)?;
    root.append_child(&child)?;
    let initial = child.computed_style()?;
    assert_eq!(initial.color, Color::RED, "native child must inherit color");
    assert_eq!(initial.font_size, 20.0);
    assert_eq!(initial.width.value(), 80.0);
    let before = document.render_to_png_buffer()?;

    let count = Rc::new(Cell::new(0));
    let callback_count = count.clone();
    let callback_root = root.downgrade();
    child.on("click", move |_| {
        let root = callback_root.upgrade().expect("live native root");
        root.set_color(Color::BLUE).expect("native color mutation");
        root.set_font_size(LengthValue::px(24.0))
            .expect("native font mutation");
        callback_count.set(callback_count.get() + 1);
    })?;
    child.click()?;
    assert_eq!(count.get(), 1);
    let changed = child.computed_style()?;
    assert_eq!(changed.color, Color::BLUE);
    assert_eq!(changed.font_size, 24.0);
    assert_eq!(changed.width.value(), 96.0);
    let after = document.render_to_png_buffer()?;
    assert_ne!(before, after);

    child.detach()?;
    let detached = child.computed_style()?;
    assert_eq!(detached.color, Color::BLACK);
    assert_eq!(detached.font_size, 16.0);
    assert_eq!(detached.width.value(), 64.0);
    let other = Element::create(&document, "div")?;
    other.set_color(Color::GREEN)?;
    other.set_font_size(LengthValue::px(12.0))?;
    root.append_child(&other)?;
    other.append_child(&child)?;
    let reparented = child.computed_style()?;
    assert_eq!(reparented.color, Color::GREEN);
    assert_eq!(reparented.font_size, 12.0);
    assert_eq!(reparented.width.value(), 48.0);
    let clone = child.clone_subtree()?;
    assert_eq!(clone.computed_style()?.color, Color::BLACK);
    assert_eq!(clone.computed_style()?.font_size, 16.0);
    root.append_child(&clone)?;
    assert_eq!(clone.computed_style()?.color, Color::BLUE);
    assert_eq!(clone.computed_style()?.width.value(), 96.0);
    let weak = child.downgrade();
    drop(clone);
    drop(child);
    drop(other);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none());
    assert_eq!(initial.color, Color::RED);
    assert_eq!(changed.color, Color::BLUE);
    assert_eq!(reparented.color, Color::GREEN);
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("before.png"), before)?;
    std::fs::write(output.join("after.png"), after)?;
    println!("native inherited styles: scale={scale} callback=1 owned-snapshots=4 passed");
    Ok(())
}
