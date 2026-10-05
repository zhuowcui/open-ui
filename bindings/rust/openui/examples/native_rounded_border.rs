//! Native rounded-border mutations, owned state, geometry and teardown.
use openui::prelude::*;
use openui::typed_style::{BorderStyle, StyleColor};
use std::{cell::Cell, path::PathBuf, rc::Rc};

const ALPHAS: [(u8, u8); 4] = [(51, 153), (102, 51), (153, 255), (255, 102)];
const RADII: [[(f32, f32); 4]; 4] = [
    [(6.0, 6.0); 4],
    [(12.0, 12.0); 4],
    [(10.0, 10.0), (12.0, 12.0), (8.0, 8.0), (6.0, 6.0)],
    [(12.0, 8.0); 4],
];

fn set_color(element: &Element, color: Color) -> Result<(), Error> {
    element.set_border_top_color(StyleColor::Resolved(color))?;
    element.set_border_right_color(StyleColor::Resolved(color))?;
    element.set_border_bottom_color(StyleColor::Resolved(color))?;
    element.set_border_left_color(StyleColor::Resolved(color))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut arguments = std::env::args().skip(1);
    let output = PathBuf::from(arguments.next().ok_or("output directory required")?);
    let scale: f64 = arguments.next().ok_or("device scale required")?.parse()?;
    if arguments.next().is_some() {
        return Err("unexpected rounded border arguments".into());
    }
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(620.0, 800.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    let callbacks = Rc::new(Cell::new(0));
    let mut cards = Vec::new();
    let mut snapshots = Vec::new();
    let mut bounds = Vec::new();
    for index in 0..65 {
        let (left, top, width, height, border, radii, before, after, background) = if index == 0 {
            (
                20.0,
                28.0,
                482.0,
                256.0,
                16,
                [(60.0, 60.0); 4],
                Color::from_rgba8(60, 150, 255, 102),
                Color::from_rgba8(255, 80, 20, 153),
                Color::TRANSPARENT,
            )
        } else {
            let grid = index - 1;
            let phase = (grid % 4) as f32 * 0.25;
            let (before_alpha, after_alpha) = ALPHAS[(grid / 4) % 4];
            (
                12.0 + (grid % 8) as f32 * 70.0 + phase,
                340.0 + (grid / 8) as f32 * 52.0 + phase,
                24.0,
                20.0,
                4,
                RADII[grid / 16],
                Color::from_rgba8(60, 150, 255, before_alpha),
                Color::from_rgba8(255, 80, 20, after_alpha),
                if (grid / 4) % 2 == 0 {
                    Color::TRANSPARENT
                } else {
                    Color::RED
                },
            )
        };
        let card = Element::create(&document, "div")?;
        card.set_position(Position::Absolute)?;
        card.set_left(Length::px(left))?;
        card.set_top(Length::px(top))?;
        card.set_width(LengthValue::px(width))?;
        card.set_height(LengthValue::px(height))?;
        card.set_border_top_width(border)?;
        card.set_border_right_width(border)?;
        card.set_border_bottom_width(border)?;
        card.set_border_left_width(border)?;
        card.set_border_top_style(BorderStyle::Solid)?;
        card.set_border_right_style(BorderStyle::Solid)?;
        card.set_border_bottom_style(BorderStyle::Solid)?;
        card.set_border_left_style(BorderStyle::Solid)?;
        card.set_border_top_left_radius(radii[0])?;
        card.set_border_top_right_radius(radii[1])?;
        card.set_border_bottom_right_radius(radii[2])?;
        card.set_border_bottom_left_radius(radii[3])?;
        card.set_background_color(background)?;
        set_color(&card, before)?;
        root.append_child(&card)?;
        let snapshot = card.computed_style()?;
        assert_eq!(snapshot.border_top_color.resolve(&snapshot.color), before);
        snapshots.push((snapshot, before));
        let rect = card
            .bounding_rect()?
            .ok_or("owned rounded border bounds required")?;
        assert_eq!(
            (rect.x, rect.y, rect.width, rect.height),
            (
                left,
                top,
                width + 2.0 * border as f32,
                height + 2.0 * border as f32
            )
        );
        bounds.push(rect);
        let count = callbacks.clone();
        let weak = card.downgrade();
        card.on("click", move |_| {
            set_color(
                &weak.upgrade().expect("attached native rounded border"),
                after,
            )
            .expect("native rounded border mutation");
            count.set(count.get() + 1);
        })?;
        cards.push((card, after));
    }
    assert_eq!(cards.len(), 65);
    std::fs::create_dir_all(&output)?;
    std::fs::write(output.join("before.png"), document.render_to_png_buffer()?)?;
    for (index, (card, after)) in cards.iter().enumerate() {
        card.click()?;
        let snapshot = card.computed_style()?;
        for edge in [
            snapshot.border_top_color,
            snapshot.border_right_color,
            snapshot.border_bottom_color,
            snapshot.border_left_color,
        ] {
            assert_eq!(edge.resolve(&snapshot.color), *after);
        }
        assert_eq!(
            card.bounding_rect()?
                .ok_or("retained border bounds required")?,
            bounds[index]
        );
    }
    assert_eq!(callbacks.get(), 65);
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
    for (snapshot, before) in snapshots {
        assert_eq!(snapshot.border_top_color.resolve(&snapshot.color), before);
    }
    std::fs::write(output.join("contract.json"), format!(
        "{{\"cards\":65,\"callbacks\":65,\"bounds\":[{owned_bounds}],\"owned_style_preserved\":true,\"owned_bounds_unchanged\":true,\"document_expired\":true,\"scale\":{scale}}}\n"
    ))?;
    println!("native rounded border: 65 mutations, owned state and teardown passed");
    Ok(())
}
