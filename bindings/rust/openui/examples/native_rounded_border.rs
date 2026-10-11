//! Native consuming app for shared rounded-border coverage qualification.
//! Arguments vary geometry and style, never WPT identifiers or expected pixels.
use openui::prelude::*;
use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output required")?);
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let phase: f32 = args.next().ok_or("phase required")?.parse()?;
    let radius: f32 = args.next().ok_or("radius required")?.parse()?;
    let width: f32 = args.next().ok_or("border width required")?.parse()?;
    let alpha: f32 = args.next().ok_or("alpha required")?.parse()?;
    let attachment = match args.next().as_deref() {
        Some("local") => BackgroundAttachment::Local,
        Some("scroll") => BackgroundAttachment::Scroll,
        _ => return Err("attachment must be local or scroll".into()),
    };
    let hidden = args.next().ok_or("hidden flag required")? == "hidden";
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    let element = Element::create(&document, "div")?;
    element.set_position(Position::Absolute)?;
    element.set_left(Length::px(20.0 + phase))?;
    element.set_top(Length::px(20.0 + phase))?;
    element.set_width(LengthValue::px(104.0))?;
    element.set_height(LengthValue::px(100.0))?;
    element.set_box_sizing(BoxSizing::ContentBox)?;
    let border = Border {
        width,
        style: BorderStyle::Solid,
        color: Color::from_rgba_f32(1.0, 0.0, 0.0, alpha),
    };
    element.set_border_top(border)?;
    element.set_border_right(border)?;
    element.set_border_bottom(border)?;
    element.set_border_left(border)?;
    element.set_border_top_left_radius((radius, radius))?;
    element.set_border_top_right_radius((radius, radius))?;
    element.set_border_bottom_right_radius((radius, radius))?;
    element.set_border_bottom_left_radius((radius, radius))?;
    element.set_background_color(Color::from_rgba8(173, 216, 230, 255))?;
    element.set_background_attachment(attachment)?;
    let overflow = if hidden {
        Overflow::Hidden
    } else {
        Overflow::Visible
    };
    element.set_overflow_x(overflow)?;
    element.set_overflow_y(overflow)?;
    root.append_child(&element)?;
    document.update_all()?;
    let before = element.bounding_rect()?.ok_or("missing initial bounds")?;
    let _initial = document.render_to_png_buffer()?;
    let count = Rc::new(Cell::new(0));
    let callback_count = count.clone();
    let weak = element.downgrade();
    element.on("click", move |_| {
        weak.upgrade()
            .expect("live element")
            .set_width(LengthValue::px(100.0))
            .expect("native mutation");
        callback_count.set(callback_count.get() + 1);
    })?;
    element.click()?;
    assert_eq!(count.get(), 1);
    let after = element.bounding_rect()?.ok_or("missing final bounds")?;
    assert_eq!(before.x, after.x);
    assert_eq!(before.y, after.y);
    assert_eq!(before.height, after.height);
    assert_eq!(before.width - after.width, 4.0);
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("openui.png"), document.render_to_png_buffer()?)?;
    std::fs::write(
        output.join("geometry.json"),
        serde_json::to_vec(
            &serde_json::json!({"callback_count": count.get(), "bounds":{"x":after.x,"y":after.y,"width":after.width,"height":after.height}}),
        )?,
    )?;
    let weak = element.downgrade();
    drop(element);
    drop(root);
    drop(document);
    assert!(
        weak.upgrade().is_none(),
        "consumer callback retained document"
    );
    Ok(())
}
