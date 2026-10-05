//! Public Rust border mutations, owned style/geometry and document teardown.
use openui::prelude::*;
use openui::typed_style::{BorderStyle, StyleColor};
use std::{cell::Cell, path::PathBuf, rc::Rc};

const PAIRS: [([u8; 3], [u8; 3]); 8] = [
    ([0, 0, 0], [255, 255, 255]),
    ([1, 1, 1], [128, 128, 128]),
    ([50, 50, 50], [70, 70, 70]),
    ([0, 0, 40], [0, 128, 0]),
    ([149, 0, 0], [150, 0, 0]),
    ([0, 91, 0], [0, 92, 0]),
    ([10, 10, 10], [85, 85, 85]),
    ([20, 40, 70], [40, 20, 30]),
];

fn color(rgb: [u8; 3], alpha: f32) -> Color {
    Color::from_rgba_f32(
        f32::from(rgb[0]) / 255.0,
        f32::from(rgb[1]) / 255.0,
        f32::from(rgb[2]) / 255.0,
        alpha,
    )
}

fn set_color(element: &Element, value: Color) -> Result<(), Error> {
    element.set_border_top_color(StyleColor::Resolved(value))?;
    element.set_border_right_color(StyleColor::Resolved(value))?;
    element.set_border_bottom_color(StyleColor::Resolved(value))?;
    element.set_border_left_color(StyleColor::Resolved(value))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let output = PathBuf::from(arguments.next().ok_or("output directory required")?);
    let scale: f64 = arguments.next().ok_or("device scale required")?.parse()?;
    if arguments.next().is_some() {
        return Err("unexpected border contrast arguments".into());
    }
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(432.0, 432.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    let callbacks = Rc::new(Cell::new(0));
    let mut cards = Vec::new();
    let mut styles = Vec::new();
    let mut bounds = Vec::new();
    for (before, after) in PAIRS {
        for border_style in [
            BorderStyle::Inset,
            BorderStyle::Outset,
            BorderStyle::Groove,
            BorderStyle::Ridge,
        ] {
            for alpha in [1.0, 0.5] {
                let index = cards.len();
                let phase = (index % 4) as f32 * 0.25;
                let left = 12.0 + (index % 8) as f32 * 52.0 + phase;
                let top = 12.0 + (index / 8) as f32 * 52.0 + phase;
                let card = Element::create(&document, "div")?;
                card.set_position(Position::Absolute)?;
                card.set_left(Length::px(left))?;
                card.set_top(Length::px(top))?;
                card.set_width(LengthValue::px(20.0))?;
                card.set_height(LengthValue::px(20.0))?;
                card.set_border_top_width(4)?;
                card.set_border_right_width(4)?;
                card.set_border_bottom_width(4)?;
                card.set_border_left_width(4)?;
                card.set_border_top_style(border_style)?;
                card.set_border_right_style(border_style)?;
                card.set_border_bottom_style(border_style)?;
                card.set_border_left_style(border_style)?;
                if (index / 8) % 2 == 1 {
                    card.set_background_color(Color::RED)?;
                }
                set_color(&card, color(before, alpha))?;
                root.append_child(&card)?;
                let saved = card.computed_style()?;
                assert_eq!(
                    saved.border_top_color.resolve(&saved.color),
                    color(before, alpha)
                );
                styles.push((saved, before, alpha));
                let rect = card
                    .bounding_rect()?
                    .ok_or("owned border bounds required")?;
                assert_eq!(
                    (rect.x, rect.y, rect.width, rect.height),
                    (left, top, 28.0, 28.0)
                );
                bounds.push(rect);
                let count = callbacks.clone();
                let weak = card.downgrade();
                card.on("click", move |_| {
                    set_color(
                        &weak.upgrade().expect("attached native card"),
                        color(after, alpha),
                    )
                    .expect("native border mutation");
                    count.set(count.get() + 1);
                })?;
                cards.push((card, color(after, alpha)));
            }
        }
    }
    assert_eq!(cards.len(), 64);
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;
    for (index, (card, expected)) in cards.iter().enumerate() {
        card.click()?;
        let style = card.computed_style()?;
        for edge in [
            style.border_top_color,
            style.border_right_color,
            style.border_bottom_color,
            style.border_left_color,
        ] {
            assert_eq!(edge.resolve(&style.color), *expected);
        }
        assert_eq!(
            card.bounding_rect()?
                .ok_or("retained border bounds required")?,
            bounds[index]
        );
    }
    assert_eq!(callbacks.get(), 64);
    std::fs::write(output.join("after.png"), document.render_to_png_buffer()?)?;
    let owned_bounds = bounds
        .iter()
        .map(|rect| {
            format!(
                "{{\"x\":{},\"y\":{},\"width\":{},\"height\":{}}}",
                rect.x, rect.y, rect.width, rect.height
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let weak = cards[0].0.downgrade();
    drop(cards);
    drop(root);
    drop(document);
    assert!(
        weak.upgrade().is_none(),
        "native border callbacks must not retain the document"
    );
    for (snapshot, before, alpha) in styles {
        assert_eq!(
            snapshot.border_top_color.resolve(&snapshot.color),
            color(before, alpha)
        );
    }
    std::fs::write(output.join("contract.json"), format!(
        "{{\"cards\":64,\"callbacks\":64,\"bounds\":[{owned_bounds}],\"owned_style_preserved\":true,\"owned_bounds_unchanged\":true,\"document_expired\":true,\"scale\":{scale}}}\n"
    ))?;
    println!("native border contrast: 64 mutations, owned state and teardown passed");
    Ok(())
}
