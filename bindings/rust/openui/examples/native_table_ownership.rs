//! Native table ownership oracle consumer; all interaction uses public Rust APIs.
//! CLI: OUTPUT WIDTH HEIGHT SCALE base|translated|opacity|clipped|scrolled
use openui::prelude::*;
use std::{cell::Cell, error::Error, path::PathBuf, rc::Rc};
type ProbeResult<T> = Result<T, Box<dyn Error>>;
fn add(document: &Document, parent: &Element, id: &str, display: Display) -> ProbeResult<Element> {
    let element = Element::create(document, "div")?;
    element.set_id(id)?;
    element.set_display(display)?;
    parent.append_child(&element)?;
    Ok(element)
}
fn size(element: &Element, width: f32, height: f32) -> ProbeResult<()> {
    element.set_width(LengthValue::px(width))?;
    element.set_height(LengthValue::px(height))?;
    Ok(())
}
fn snapshot(nodes: &[(&str, Element)]) -> ProbeResult<serde_json::Value> {
    let rect =
        |r: openui::Rect| serde_json::json!({"x":r.x,"y":r.y,"width":r.width,"height":r.height});
    let records=nodes.iter().map(|(name,element)| Ok(serde_json::json!({"name":name,"bounds":element.bounding_rect()?.map(rect),"client_rects":element.client_rects()?.into_iter().map(rect).collect::<Vec<_>>()}))).collect::<ProbeResult<Vec<_>>>()?;
    Ok(serde_json::json!(records))
}
fn main() -> ProbeResult<()> {
    let mut args = std::env::args().skip(1);
    let out = PathBuf::from(args.next().ok_or("OUTPUT")?);
    let width: f64 = args.next().ok_or("WIDTH")?.parse()?;
    let height: f64 = args.next().ok_or("HEIGHT")?.parse()?;
    let scale: f64 = args.next().ok_or("SCALE")?.parse()?;
    let state = args.next().ok_or("STATE")?;
    if args.next().is_some() {
        return Err("too many arguments".into());
    }
    if !["base", "translated", "opacity", "clipped", "scrolled"].contains(&state.as_str()) {
        return Err("invalid STATE".into());
    }
    std::fs::create_dir_all(&out)?;
    let callbacks = Rc::new(Cell::new(0_u32));
    let weak = {
        let document = Document::with_font_collection_and_options(
            ViewportMetrics::from_logical_size(width, height, scale)?,
            FontCollection::deterministic_test(),
            EngineOptions {
                raster_configuration: RasterConfiguration::deterministic_aliased(false),
            },
        )?;
        let body = document.body();
        body.set_background_color(Color::WHITE)?;
        let scroller = add(&document, &body, "scroller", Display::Block)?;
        size(&scroller, 360.0, 120.0)?;
        scroller.set_overflow(Overflow::Hidden)?;
        let content = add(&document, &scroller, "content", Display::Block)?;
        size(&content, 460.0, 220.0)?;
        content.set_position(Position::Relative)?;
        let table = add(&document, &content, "table", Display::Table)?;
        table.set_position(Position::Absolute)?;
        table.set_left(Length::px(40.0))?;
        table.set_top(Length::px(24.0))?;
        table.set_width(LengthValue::px(120.0))?;
        table.set_border_collapse(BorderCollapse::Separate)?;
        table.set_border_spacing((Length::px(0.0), Length::px(0.0)))?;
        let caption = add(&document, &table, "caption", Display::TableCaption)?;
        caption.set_height(LengthValue::px(16.0))?;
        caption.set_background_color(Color::BLUE)?;
        let group = add(&document, &table, "group", Display::TableRowGroup)?;
        let cell = add(&document, &group, "cell", Display::TableCell)?;
        size(&cell, 120.0, 24.0)?;
        cell.set_background_color(Color::RED)?;
        let improper = add(&document, &content, "improper-table", Display::Table)?;
        improper.set_position(Position::Absolute)?;
        improper.set_left(Length::px(220.0))?;
        improper.set_top(Length::px(24.0))?;
        improper.set_width(LengthValue::px(100.0))?;
        improper.set_border_collapse(BorderCollapse::Separate)?;
        improper.set_border_spacing((Length::px(0.0), Length::px(0.0)))?;
        let second_caption = add(
            &document,
            &improper,
            "improper-caption",
            Display::TableCaption,
        )?;
        second_caption.set_height(LengthValue::px(12.0))?;
        second_caption.set_background_color(Color::BLACK)?;
        let improper_group = add(
            &document,
            &improper,
            "improper-group",
            Display::TableRowGroup,
        )?;
        let first = add(&document, &improper_group, "improper-first", Display::Block)?;
        size(&first, 40.0, 8.0)?;
        first.set_background_color(Color::GREEN)?;
        let second = add(
            &document,
            &improper_group,
            "improper-second",
            Display::Block,
        )?;
        size(&second, 50.0, 10.0)?;
        second.set_background_color(Color::BLUE)?;
        match state.as_str() {
            "translated" => {
                table.set_transform(TransformList(vec![TransformOperation::Translate(
                    LengthValue::px(13.0),
                    LengthValue::px(7.0),
                )]))?;
                group.set_transform(TransformList(vec![TransformOperation::Translate(
                    LengthValue::px(4.0),
                    LengthValue::px(3.0),
                )]))?;
                improper_group.set_transform(TransformList(vec![
                    TransformOperation::Translate(LengthValue::px(3.0), LengthValue::px(2.0)),
                ]))?;
            }
            "opacity" => {
                table.set_opacity(0.5)?;
                group.set_opacity(0.5)?;
                improper_group.set_opacity(0.5)?;
            }
            "clipped" => {
                table.set_clip_path_inset(Some([
                    Length::px(4.0),
                    Length::px(6.0),
                    Length::px(5.0),
                    Length::px(8.0),
                ]))?;
                group.set_clip_path_inset(Some([
                    Length::px(2.0),
                    Length::px(3.0),
                    Length::px(2.0),
                    Length::px(3.0),
                ]))?;
            }
            "scrolled" => {
                scroller.scroll_to(20.0, 12.0)?;
            }
            _ => {}
        }
        let nodes = vec![
            ("scroller", scroller.clone()),
            ("content", content.clone()),
            ("table", table.clone()),
            ("caption", caption.clone()),
            ("group", group.clone()),
            ("cell", cell.clone()),
            ("improper-table", improper.clone()),
            ("improper-caption", second_caption.clone()),
            ("improper-group", improper_group.clone()),
            ("improper-first", first.clone()),
            ("improper-second", second.clone()),
        ];
        let geometry = snapshot(&nodes)?;
        let points = [
            (45.0, 28.0),
            (45.0, 43.0),
            (55.0, 36.0),
            (58.0, 55.0),
            (42.0, 42.0),
            (158.0, 62.0),
            (225.0, 38.0),
            (225.0, 48.0),
            (235.0, 44.0),
            (355.0, 110.0),
        ];
        let hits=points.iter().map(|&(x,y)|Ok(serde_json::json!({"x":x,"y":y,"id":document.hit_test(x,y)?.and_then(|element|element.get_attribute("id").ok().flatten())}))).collect::<ProbeResult<Vec<_>>>()?;
        let count = callbacks.clone();
        body.on("click", move |_| count.set(count.get() + 1))?;
        let frame = document.render_to_png_buffer()?;
        if document.render_to_png_buffer()? != frame {
            return Err("unchanged frame differs".into());
        }
        body.dispatch_click_event()?;
        if callbacks.get() != 1 || document.render_to_png_buffer()? != frame {
            return Err("callback changed pixels".into());
        }
        let all_single_owned_box = nodes
            .iter()
            .all(|(_, node)| node.client_rects().is_ok_and(|rects| rects.len() == 1));
        std::fs::write(out.join("frame.png"), frame)?;
        std::fs::write(
            out.join("bounds.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"state":state,"nodes":geometry,"hits":hits,"scroll_left":scroller.scroll_left()?,"scroll_top":scroller.scroll_top()?}),
            )?,
        )?;
        std::fs::write(
            out.join("lifecycle.json"),
            serde_json::to_vec_pretty(
                &serde_json::json!({"native_click_callbacks":callbacks.get(),"unchanged_frame_byte_repeat":true,"count_only_callback_preserved_pixels":true,"all_single_owned_box":all_single_owned_box,"javascript_executed_by_openui":false}),
            )?,
        )?;
        table.downgrade()
    };
    if weak.upgrade().is_some() {
        return Err("native document leaked".into());
    }
    let path = out.join("lifecycle.json");
    let mut lifecycle: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    lifecycle["weak_teardown_passed"] = true.into();
    std::fs::write(path, serde_json::to_vec_pretty(&lifecycle)?)?;
    Ok(())
}
