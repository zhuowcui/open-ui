//! Consuming native app for text intrinsic sizing across layout contexts.
use openui::prelude::*;
use std::{cell::Cell, path::PathBuf, rc::Rc};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = PathBuf::from(args.next().ok_or("output path required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    let selected_case: Option<usize> = args.next().map(|arg| arg.parse()).transpose()?;
    if args.next().is_some() || selected_case.is_some_and(|index| index >= 200) {
        return Err("unexpected arguments".into());
    }
    let document = Document::with_font_collection(
        ViewportMetrics::from_logical_size(320.0, 160.0, scale)?,
        FontCollection::deterministic_test(),
    )?;
    let root = document.body();
    root.set_font_family(FontFamilyList::single("Ahem"))?;
    root.set_font_size(LengthValue::px(16.0))?;
    root.set_line_height(LineHeight::Number(1.0))?;
    let mut cases = Vec::new();
    for (context_index, context) in [
        "absolute",
        "inline-block",
        "float",
        "min-content",
        "max-content",
    ]
    .into_iter()
    .enumerate()
    {
        for (variant_index, variant) in [
            "word",
            "empty",
            "break-only",
            "break-word",
            "two-lines",
            "two-words",
            "split-word",
            "preserved-lines",
        ]
        .into_iter()
        .enumerate()
        {
            for (indent_index, (indent, value)) in [
                ("0px", LengthValue::px(0.0)),
                ("20px", LengthValue::px(20.0)),
                ("-20px", LengthValue::px(-20.0)),
                ("10%", LengthValue::percent(10.0)),
                ("calc(10% + 20px)", LengthValue::calc_percent_px(10.0, 20.0)),
            ]
            .into_iter()
            .enumerate()
            {
                let index = context_index * 40 + variant_index * 5 + indent_index;
                if selected_case.is_some_and(|selected| selected != index) {
                    continue;
                }
                eprintln!("creating native case-{index}: {context} {variant} {indent}");
                let parent = Element::create(&document, "div")?;
                parent.set_position(Position::Relative)?;
                parent.set_width(LengthValue::px(240.0))?;
                parent.set_height(LengthValue::px(100.0))?;
                root.append_child(&parent)?;
                let element = Element::create(&document, "div")?;
                match context {
                    "absolute" => element.set_position(Position::Absolute)?,
                    "inline-block" => element.set_display(Display::InlineBlock)?,
                    "float" => element.set_float(Float::Left)?,
                    "min-content" => {
                        element.set_width(LengthValue::Computed(Length::min_content()))?
                    }
                    "max-content" => {
                        element.set_width(LengthValue::Computed(Length::max_content()))?
                    }
                    _ => unreachable!(),
                }
                element.set_text_indent(value)?;
                parent.append_child(&element)?;
                match variant {
                    "word" => element.set_text("XXXXX")?,
                    "empty" => element.set_text("")?,
                    "two-words" => element.set_text("X XXXXX")?,
                    "preserved-lines" => {
                        element.set_white_space(WhiteSpaceShorthand {
                            collapse: WhiteSpaceCollapse::Preserve,
                            wrap: TextWrapMode::Nowrap,
                        })?;
                        element.set_text("X\nXXXXX")?;
                    }
                    "split-word" => {
                        for text in ["X", "XXXX"] {
                            let span = Element::create(&document, "span")?;
                            span.set_text(text)?;
                            element.append_child(&span)?;
                        }
                    }
                    "break-only" | "break-word" | "two-lines" => {
                        if variant == "two-lines" {
                            element.set_text("X")?;
                        }
                        let line_break = Element::create(&document, "br")?;
                        element.append_child(&line_break)?;
                        if variant != "break-only" {
                            let span = Element::create(&document, "span")?;
                            span.set_text("XXXXX")?;
                            element.append_child(&span)?;
                        }
                    }
                    _ => unreachable!(),
                }
                cases.push((index, context, variant, indent, element));
            }
        }
    }
    let mut rows = Vec::new();
    for (index, context, variant, indent, element) in &cases {
        eprintln!("measuring native case-{index}");
        let bounds = element.bounding_rect()?.ok_or("native bounds required")?;
        rows.push(format!(
            "{{\"id\":\"case-{index}\",\"context\":\"{context}\",\"variant\":\"{variant}\",\"indent\":\"{indent}\",\"width\":{},\"height\":{}}}",
            bounds.width, bounds.height
        ));
    }
    let callback_calls = Rc::new(Cell::new(0));
    let calls = callback_calls.clone();
    let target = &cases[0].4;
    let weak_target = target.downgrade();
    target.on("click", move |_| {
        weak_target
            .upgrade()
            .expect("live native target")
            .set_text_indent(LengthValue::px(28.0))
            .expect("native indentation mutation");
        calls.set(calls.get() + 1);
    })?;
    let before = target.bounding_rect()?.ok_or("before callback bounds")?;
    target.click()?;
    let after = target.bounding_rect()?.ok_or("after callback bounds")?;
    assert_eq!(callback_calls.get(), 1);
    let owned_before_width = before.width;
    if cases[0].0 == 0 {
        assert_eq!(before.width, 80.015625);
        assert_eq!(after.width, 108.015625);
    }
    assert_eq!(target.computed_style()?.text_indent.value(), 28.0);
    let weak = target.downgrade();
    drop(cases);
    drop(root);
    drop(document);
    assert!(weak.upgrade().is_none());
    assert_eq!(before.width, owned_before_width);
    std::fs::create_dir_all(&output)?;
    std::fs::write(
        output.join("geometry.json"),
        format!("[{}]\n", rows.join(",")),
    )?;
    println!(
        "native intrinsic sizes: scale={scale} cases={} callback=1 owned-bounds=2 passed",
        rows.len()
    );
    Ok(())
}
