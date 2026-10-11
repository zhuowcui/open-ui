//! A public Rust consumer for anonymous flex text and intrinsic-width neighbors.
use openui::prelude::*;
use openui_text::{Font, FontDescription, TextShaper};
use std::{cell::Cell, path::PathBuf, rc::Rc, sync::Arc};

fn snapshot(
    document: &Document,
    nodes: &[(String, Element)],
    fonts: &Arc<FontCollection>,
    out: &std::path::Path,
    stage: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let values = nodes.iter().map(|(name, element)| {
        let b = element.bounding_rect()?.ok_or("missing bounds")?;
        Ok(serde_json::json!({"name":name,"bounds":{"x":b.x,"y":b.y,"width":b.width,"height":b.height}}))
    }).collect::<Result<Vec<_>,Box<dyn std::error::Error>>>()?;
    let shaper = TextShaper::new();
    let metrics = nodes.iter().filter(|(name,_)| name.starts_with("flex-") || name.starts_with("word-")).map(|(name, element)| {
        let description = FontDescription::from_computed_style(&element.computed_style()?);
        let font = Font::new_in_collection(description, fonts.clone());
        let samples = ["\u{a0}", "A", "DIV1", "ABCDEFGHI", " \u{a0} "].iter().map(|text| {
            let shaped = shaper.shape(text, &font, openui_text::TextDirection::Ltr);
            serde_json::json!({"text":text,"unshaped_width":font.width(text),"unshaped_ceil":(font.width(text)*64.0).ceil()/64.0,"shaped_width":shaped.width,"shaped_ceil":(shaped.width*64.0).ceil()/64.0,"runs":shaped.runs.iter().map(|run| serde_json::json!({"family":run.font_data.typeface().family_name(),"subpixel":run.font_data.sk_font().is_subpixel(),"linear_metrics":run.font_data.sk_font().is_linear_metrics(),"advances":run.advances})).collect::<Vec<_>>()})
        }).collect::<Vec<_>>();
        Ok(serde_json::json!({"name":name,"samples":samples}))
    }).collect::<Result<Vec<_>,Box<dyn std::error::Error>>>()?;
    let png = document.render_to_png_buffer()?;
    assert_eq!(png, document.render_to_png_buffer()?);
    std::fs::write(out.join(format!("{stage}.png")), png)?;
    std::fs::write(
        out.join(format!("{stage}.json")),
        serde_json::to_vec_pretty(&values)?,
    )?;
    std::fs::write(
        out.join(format!("{stage}-metrics.json")),
        serde_json::to_vec_pretty(&metrics)?,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("output")?);
    let width: f64 = args.next().ok_or("width")?.parse()?;
    let height: f64 = args.next().ok_or("height")?.parse()?;
    let scale: f64 = args.next().ok_or("scale")?.parse()?;
    std::fs::create_dir_all(&out)?;
    let font_mode = args.next().ok_or("font mode")?;
    let fonts = match font_mode.as_str() {
        "registered" => FontCollection::deterministic_test(),
        "system" => FontCollection::system(),
        _ => return Err("unknown font mode".into()),
    };
    let document = Document::with_font_collection_and_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        fonts.clone(),
        EngineOptions {
            raster_configuration: RasterConfiguration::deterministic_aliased(false),
        },
    )?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_font_size(LengthValue::px(16.0))?;
    root.set_line_height(LineHeight::Length(16.0))?;
    root.set_font_kerning(FontKerning::None)?;
    root.set_font_variant_ligatures(FontVariantLigatures::none())?;
    let mut nodes = Vec::new();
    let mut targets = Vec::new();
    for i in 0..6 {
        let flex = Element::create(&document, "div")?;
        root.append_child(&flex)?;
        flex.set_position(Position::Absolute)?;
        flex.set_left(Length::px(20.0))?;
        flex.set_top(Length::px(20.0 + i as f32 * 55.0))?;
        flex.set_width(LengthValue::px(300.0))?;
        flex.set_height(LengthValue::px(40.0))?;
        flex.set_display(Display::Flex)?;
        flex.set_justify_content(ContentAlignment::new(ContentPosition::FlexEnd))?;
        flex.set_background_color(Color::from_rgba8(0, 128, 0, 255))?;
        flex.set_font_family(if i < 3 {
            FontFamilyList {
                families: [
                    "Ahem",
                    "Droid Sans Fallback",
                    "Noto Sans Devanagari",
                    "Noto Color Emoji",
                    "DejaVu Sans",
                ]
                .into_iter()
                .map(|name| FontFamily::Named(name.to_owned()))
                .collect(),
            }
        } else {
            FontFamilyList::single("Ahem")
        })?;
        nodes.push((format!("flex-{i}"), flex.clone()));
        for j in 0..5 {
            if j > 0 {
                match i % 3 {
                    0 => targets.push(flex.create_text_child(" \n\u{a0}\n ")?),
                    1 => {
                        flex.append_text_node(" \nA\n ")?;
                    }
                    _ => {
                        let gap = Element::create(&document, "span")?;
                        flex.append_child(&gap)?;
                        gap.set_text("\u{a0}")?;
                        nodes.push((format!("gap-{i}-{j}"), gap));
                    }
                }
            }
            let item = Element::create(&document, "div")?;
            flex.append_child(&item)?;
            item.set_width(LengthValue::px(50.0))?;
            item.set_height(LengthValue::px(30.0))?;
            item.set_background_color(Color::from_rgba8(242, 210, 80, 255))?;
            item.set_color(Color::from_rgba8(41, 119, 248, 255))?;
            item.set_text(&format!("DIV{}", j + 1))?;
            nodes.push((format!("item-{i}-{j}"), item));
        }
    }
    for i in 0..2 {
        let host = Element::create(&document, "div")?;
        root.append_child(&host)?;
        host.set_position(Position::Absolute)?;
        host.set_left(Length::px(20.0 + i as f32 * 180.0))?;
        host.set_top(Length::px(420.0))?;
        host.set_width(LengthValue::px(75.0))?;
        host.set_font_family(FontFamilyList::single("Ahem"))?;
        host.set_font_kerning(if i == 0 {
            FontKerning::None
        } else {
            FontKerning::Auto
        })?;
        let word = Element::create(&document, "span")?;
        host.append_child(&word)?;
        word.set_display(Display::InlineBlock)?;
        word.set_text("ABCDEFGHI")?;
        nodes.push((format!("host-{i}"), host));
        nodes.push((format!("word-{i}"), word));
    }
    let weak = nodes[0].1.downgrade();
    let text_weak = targets[0].downgrade();
    let callbacks = Rc::new(Cell::new(0));
    let called = callbacks.clone();
    let targets_weak = targets.iter().map(TextNode::downgrade).collect::<Vec<_>>();
    root.on("click", move |_| {
        for target in &targets_weak {
            target.upgrade().unwrap().set_data(" \nA\n ").unwrap();
        }
        called.set(called.get() + 1);
    })?;
    snapshot(&document, &nodes, &fonts, &out, "before")?;
    root.click()?;
    assert_eq!(callbacks.get(), 1);
    snapshot(&document, &nodes, &fonts, &out, "after")?;
    drop(targets);
    drop(nodes);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none() && text_weak.upgrade().is_none());
    std::fs::write(
        out.join("lifecycle.json"),
        serde_json::to_vec_pretty(
            &serde_json::json!({"callbacks":callbacks.get(),"weak_teardown_passed":true,"javascript_executed_by_openui":false}),
        )?,
    )?;
    Ok(())
}
