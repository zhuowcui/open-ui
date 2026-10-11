//! Public native API probe for font selection, aliasing, and wrapped intrinsic text.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc, sync::Arc};

fn snapshot(
    document: &Document,
    nodes: &[(String, Element)],
    output: &std::path::Path,
    stage: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let geometry = nodes.iter().map(|(name, node)| {
        let b = node.bounding_rect()?.ok_or("missing bounds")?;
        Ok(serde_json::json!({"name":name,"bounds":{"x":b.x,"y":b.y,"width":b.width,"height":b.height}}))
    }).collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let png = document.render_to_png_buffer()?;
    assert_eq!(png, document.render_to_png_buffer()?);
    std::fs::write(output.join(format!("{stage}.png")), png)?;
    std::fs::write(
        output.join(format!("{stage}.json")),
        serde_json::to_vec_pretty(&geometry)?,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output directory required")?);
    let width: f64 = args.next().ok_or("width required")?.parse()?;
    let height: f64 = args.next().ok_or("height required")?.parse()?;
    let scale: f64 = args.next().ok_or("scale required")?.parse()?;
    let fonts = PathBuf::from(args.next().ok_or("font directory required")?);
    let collection = FontCollection::deterministic_test();
    for (file, alias) in [
        ("Ahem.ttf", "Native Ahem Alias"),
        ("DejaVuSans.ttf", "Native Sans Alias"),
        ("DejaVuSansMono.ttf", "Native Mono Alias"),
    ] {
        collection.register(
            Arc::<[u8]>::from(std::fs::read(fonts.join(file))?),
            FontFaceDescriptor::new(alias),
        )?;
    }
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        collection,
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    let mut nodes = Vec::new();
    let mut targets = Vec::new();
    for (i, size) in [10.0, 12.0, 16.0, 23.0].into_iter().enumerate() {
        for (j, (family, alias, next_alias)) in [
            ("Ahem", "Native Ahem Alias", "Native Sans Alias"),
            ("DejaVu Sans", "Native Sans Alias", "Native Mono Alias"),
            ("DejaVu Sans Mono", "Native Mono Alias", "Native Ahem Alias"),
        ]
        .into_iter()
        .enumerate()
        {
            let node = Element::create(&document, "div")?;
            root.append_child(&node)?;
            node.set_position(Position::Absolute)?;
            node.set_left(Length::px(10.0 + j as f32 * 112.0 + 0.375))?;
            node.set_top(Length::px(10.0 + i as f32 * 115.0))?;
            node.set_width(LengthValue::px(105.0))?;
            node.set_font_size(LengthValue::px(size))?;
            node.set_font_family(FontFamilyList {
                families: vec![
                    FontFamily::Named(family.into()),
                    FontFamily::Named("DejaVu Sans".into()),
                ],
            })?;
            node.set_text("XX 'XX fi α א")?;
            targets.push((node.downgrade(), [alias.to_owned(), next_alias.to_owned()]));
            nodes.push((format!("case-{i}-{j}"), node));
        }
    }
    for (label, x, floats) in [("word", 10.0, false), ("floats", 170.0, true)] {
        let container = Element::create(&document, "div")?;
        root.append_child(&container)?;
        container.set_position(Position::Absolute)?;
        container.set_left(Length::px(x))?;
        container.set_top(Length::px(485.0))?;
        container.set_width(LengthValue::px(75.0))?;
        let atomic = Element::create(&document, "span")?;
        container.append_child(&atomic)?;
        atomic.set_display(Display::InlineBlock)?;
        atomic.set_font_size(LengthValue::px(16.0))?;
        atomic.set_font_family(FontFamilyList {
            families: vec![
                FontFamily::Named("Ahem".into()),
                FontFamily::Named("DejaVu Sans".into()),
            ],
        })?;
        atomic.set_background_color(Color::BLUE)?;
        if floats {
            for _ in 0..2 {
                let child = Element::create(&document, "div")?;
                atomic.append_child(&child)?;
                child.set_float(Float::Left)?;
                child.set_width(LengthValue::px(90.0))?;
                child.set_height(LengthValue::px(20.0))?;
                child.set_background_color(Color::RED)?;
            }
        } else {
            atomic.set_text("ABCDEFGHI")?;
        }
        targets.push((
            atomic.downgrade(),
            [
                "Native Ahem Alias".to_owned(),
                "Native Sans Alias".to_owned(),
            ],
        ));
        nodes.push((format!("atomic-{label}"), atomic));
    }
    let calls = Rc::new(Cell::new(0));
    let called = calls.clone();
    root.on("click", move |_| {
        let phase = called.get() as usize;
        assert!(phase < 2);
        for (target, families) in &targets {
            let family = &families[phase];
            target
                .upgrade()
                .expect("mounted font probe")
                .set_font_family(FontFamilyList {
                    families: vec![
                        FontFamily::Named(family.clone()),
                        FontFamily::Named("DejaVu Sans".into()),
                    ],
                })
                .expect("native family mutation");
        }
        called.set(called.get() + 1);
    })?;
    std::fs::create_dir_all(&output)?;
    snapshot(&document, &nodes, &output, "before")?;
    root.click()?;
    snapshot(&document, &nodes, &output, "alias")?;
    root.click()?;
    snapshot(&document, &nodes, &output, "after")?;
    assert_eq!(calls.get(), 2);
    let weak = nodes[0].1.downgrade();
    drop(nodes);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none());
    std::fs::write(
        output.join("lifecycle.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"callbacks":calls.get(),"weak_teardown_passed":true,"javascript_executed_by_openui":false}),
        )?,
    )?;
    Ok(())
}
