//! Repeated table body mutations through native Rust APIs and callbacks.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn child(
    document: &Document,
    parent: &Element,
    display: Display,
) -> Result<Element, openui::Error> {
    let element = Element::create(document, "div")?;
    element.set_display(display)?;
    parent.append_child(&element)?;
    Ok(element)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = std::env::args().nth(1).map(PathBuf::from);
    for scale in [1.0, 1.25, 1.5, 2.0, 3.0] {
        for outer_height in [40.0, 30.0, 20.0, 40.5] {
            let document = Document::with_viewport_metrics(ViewportMetrics::from_logical_size(
                375.0, 667.0, scale,
            )?)?;
            let root = document.body();
            root.set_background_color(Color::WHITE)?;
            let outer = child(&document, &root, Display::Block)?;
            outer.set_position(Position::Absolute)?;
            outer.set_left(Length::px(120.0))?;
            outer.set_top(Length::px(120.0))?;
            outer.set_width(LengthValue::px(135.0))?;
            outer.set_height(LengthValue::px(outer_height))?;
            outer.set_column_count(Some(4))?;
            outer.set_column_fill(ColumnFill::Auto)?;
            outer.set_column_gap(LengthValue::px(16.0))?;
            outer.set_background_color(Color::from_rgba8(255, 255, 0, 255))?;
            let spacer = child(&document, &outer, Display::Block)?;
            spacer.set_margin_bottom(LengthValue::px(-60.0))?;
            let inner = child(&document, &outer, Display::Block)?;
            inner.set_column_count(Some(1))?;
            inner.set_column_fill(ColumnFill::Auto)?;
            inner.set_background_color(Color::from_rgba8(0, 255, 0, 255))?;
            let table = child(&document, &inner, Display::Table)?;
            for (display, color) in [
                (Display::TableHeaderGroup, Color::BLUE),
                (
                    Display::TableFooterGroup,
                    Color::from_rgba8(255, 105, 180, 255),
                ),
            ] {
                let group = child(&document, &table, display)?;
                group.set_break_inside(BreakInside::Avoid)?;
                let content = child(&document, &group, Display::Block)?;
                content.set_width(LengthValue::px(20.0))?;
                content.set_height(LengthValue::px(20.0))?;
                content.set_background_color(color)?;
            }
            let row = child(&document, &table, Display::TableRow)?;
            let cell = child(&document, &row, Display::TableCell)?;
            let body = child(&document, &cell, Display::Block)?;
            body.set_height(LengthValue::px(100.0))?;
            body.set_background_color(Color::BLACK)?;
            let calls = Rc::new(Cell::new(0));
            let callback_calls = calls.clone();
            let weak_body = body.downgrade();
            let callback_body = weak_body.clone();
            body.on("click", move |_| {
                let body = callback_body
                    .upgrade()
                    .expect("native body remains attached");
                let next = callback_calls.get() + 1;
                body.set_height(LengthValue::px(if next == 1 { 50.0 } else { 100.0 }))
                    .unwrap();
                callback_calls.set(next);
            })?;
            for (state, body_height) in [("before", 100.0), ("after", 50.0), ("restored", 100.0)] {
                if state != "before" {
                    body.click()?;
                }
                let table_rects = table.client_rects()?;
                let body_rects = body.client_rects()?;
                let first_body = body_height.min(outer_height + 20.0);
                let remaining = body_height - first_body;
                let expected_count = 1 + remaining.ceil() as usize;
                assert_eq!(
                    table_rects.len(),
                    expected_count,
                    "native table continuation count"
                );
                assert_eq!(body_rects.len(), expected_count);
                assert_eq!(
                    body_rects.iter().map(|r| r.height).sum::<f32>(),
                    body_height
                );
                for (index, (table_rect, body_rect)) in
                    table_rects.iter().zip(&body_rects).enumerate()
                {
                    let last = index + 1 == expected_count;
                    let body_size = if index == 0 {
                        first_body
                    } else if last {
                        remaining - (index - 1) as f32
                    } else {
                        1.0
                    };
                    let table_size = if index == 0 {
                        first_body + 40.0
                    } else if last {
                        body_size + 40.0
                    } else {
                        outer_height.max(21.0)
                    };
                    let x = 120.0 + index as f32 * 37.75;
                    assert_eq!(
                        (
                            table_rect.x,
                            table_rect.y,
                            table_rect.width,
                            table_rect.height
                        ),
                        (x, if index == 0 { 60.0 } else { 120.0 }, 20.0, table_size)
                    );
                    assert_eq!(
                        (body_rect.x, body_rect.y, body_rect.width, body_rect.height),
                        (x, if index == 0 { 80.0 } else { 140.0 }, 20.0, body_size)
                    );
                }
                if let Some(output) = &output {
                    let directory = output.join(format!("scale-{scale}/height-{outer_height}"));
                    std::fs::create_dir_all(&directory)?;
                    std::fs::write(
                        directory.join(format!("{state}.png")),
                        document.render_to_png_buffer()?,
                    )?;
                }
                println!("scale={scale} height={outer_height} state={state} fragments={expected_count} native geometry passed");
            }
            assert_eq!(calls.get(), 2);
            let before_detach = body.client_rects()?;
            outer.detach()?;
            assert!(body.client_rects()?.is_empty());
            root.append_child(&outer)?;
            assert_eq!(body.client_rects()?, before_detach);
            drop(body);
            drop(cell);
            drop(row);
            drop(table);
            drop(inner);
            drop(spacer);
            drop(outer);
            drop(root);
            drop(document);
            assert!(
                weak_body.upgrade().is_none(),
                "callbacks must not retain the document"
            );
        }
    }
    Ok(())
}
