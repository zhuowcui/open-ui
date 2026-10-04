//! Native font, animation and pseudo-style operations from a consuming Rust app.
use openui::prelude::*;
use openui_geometry::Length;
use std::{cell::Cell, path::Path, rc::Rc};

fn check<T: std::fmt::Debug + PartialEq>(
    failures: &mut Vec<String>,
    name: &str,
    actual: T,
    expected: T,
) {
    if actual != expected {
        failures.push(format!("{name}: got {actual:?}, expected {expected:?}"));
    }
}

fn observe(
    document: &Document,
    child: &Element,
    output: &Path,
    name: &str,
    font: f32,
    width: f32,
    failures: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let bounds = child
        .bounding_rect()?
        .ok_or("missing native child bounds")?;
    let style = child.computed_style()?;
    println!("{name} font={} width={}", style.font_size, bounds.width);
    check(failures, &format!("{name} font"), style.font_size, font);
    check(failures, &format!("{name} width"), bounds.width, width);
    std::fs::write(
        output.join(format!("{name}.png")),
        document.render_to_png_buffer()?,
    )?;
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let output = std::path::PathBuf::from(args.next().ok_or("output directory required")?);
    let scale: f64 = args.next().ok_or("device scale required")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected argument".into());
    }
    std::fs::create_dir_all(&output)?;
    let document =
        Document::with_viewport_metrics(ViewportMetrics::from_logical_size(320.0, 240.0, scale)?)?;
    let root = document.body();
    root.set_background_color(Color::WHITE)?;
    root.set_overflow(Overflow::Hidden)?;
    root.set_font_size(LengthValue::px(20.0))?;
    root.set_color(Color::RED)?;
    let child = Element::create(&document, "div")?;
    child.set_width(LengthValue::Em(3.0))?;
    child.set_height(LengthValue::px(16.0))?;
    child.set_border_top_width(4)?;
    child.set_border_top_style(BorderStyle::Solid)?;
    root.append_child(&child)?;
    let mut failures = Vec::new();
    let options = AnimationOptions {
        duration_ms: 100.0,
        fill: FillMode::Both,
        ..AnimationOptions::default()
    };
    observe(
        &document,
        &child,
        &output,
        "initial",
        20.0,
        60.0,
        &mut failures,
    )?;
    let animation = root.animate(
        StyleProperty::FontSize,
        Keyframes::from_values(LengthValue::Em(1.0), LengthValue::Em(2.0)),
        options.clone(),
    )?;
    observe(
        &document,
        &child,
        &output,
        "animation-start",
        16.0,
        48.0,
        &mut failures,
    )?;
    document.advance_time(50.0)?;
    observe(
        &document,
        &child,
        &output,
        "animation-half",
        24.0,
        72.0,
        &mut failures,
    )?;
    document.cancel_animation(animation)?;
    observe(
        &document,
        &child,
        &output,
        "animation-cancel",
        20.0,
        60.0,
        &mut failures,
    )?;
    let animation = root.transition(
        StyleProperty::FontSize,
        LengthValue::px(32.0),
        options.clone(),
    )?;
    document.cancel_animation(animation)?;
    observe(
        &document,
        &child,
        &output,
        "transition-cancel",
        32.0,
        96.0,
        &mut failures,
    )?;
    let calls = Rc::new(Cell::new(0));
    let callback_calls = calls.clone();
    let weak_root = root.downgrade();
    child.on("click", move |_| {
        weak_root
            .upgrade()
            .expect("live root")
            .set_color(Color::BLUE)
            .expect("native mutation");
        callback_calls.set(callback_calls.get() + 1);
    })?;
    child.click()?;
    check(&mut failures, "native click callback count", calls.get(), 1);
    observe(
        &document,
        &child,
        &output,
        "transition-after-color",
        32.0,
        96.0,
        &mut failures,
    )?;
    let animation = child.transition(StyleProperty::FontSize, LengthValue::px(40.0), options)?;
    document.cancel_animation(animation)?;
    observe(
        &document,
        &child,
        &output,
        "child-transition-cancel",
        40.0,
        120.0,
        &mut failures,
    )?;
    root.set_color(Color::RED)?;
    observe(
        &document,
        &child,
        &output,
        "child-transition-after-color",
        40.0,
        120.0,
        &mut failures,
    )?;
    root.set_font_size(LengthValue::px(20.0))?;
    child.set_font_size(LengthValue::Computed(Length::calc_percent_px(150.0, -2.0)))?;
    observe(
        &document,
        &child,
        &output,
        "calc-before",
        28.0,
        84.0,
        &mut failures,
    )?;
    let owned_calc = child.computed_style()?;
    root.set_font_size(LengthValue::px(24.0))?;
    observe(
        &document,
        &child,
        &output,
        "calc-after",
        34.0,
        102.0,
        &mut failures,
    )?;
    child.set_font_size(LengthValue::px(40.0))?;
    root.set_line_height(LineHeight::Percentage(150.0))?;
    check(
        &mut failures,
        "inherited percentage line-height",
        child.computed_style()?.line_height,
        LineHeight::Length(36.0),
    );
    root.set_line_height(LineHeight::Number(1.5))?;
    check(
        &mut failures,
        "inherited unitless line-height",
        child.computed_style()?.line_height,
        LineHeight::Number(1.5),
    );
    let pseudo = Element::create(&document, "div")?;
    root.append_child(&pseudo)?;
    root.set_font_size(LengthValue::px(20.0))?;
    pseudo.set_pseudo_style(
        PseudoStyleTarget::FirstLine,
        &Style::default()
            .font_size(LengthValue::Computed(Length::calc_percent_px(150.0, 2.0)))
            .font_weight(FontWeight::BOLD),
    )?;
    let owned_pseudo = pseudo
        .computed_style()?
        .first_line_style
        .clone()
        .ok_or("missing native pseudo style")?;
    check(
        &mut failures,
        "pseudo before font",
        owned_pseudo.font_size,
        32.0,
    );
    check(
        &mut failures,
        "pseudo before color",
        owned_pseudo.color,
        Color::RED,
    );
    root.set_font_size(LengthValue::px(24.0))?;
    root.set_color(Color::BLUE)?;
    let changed_pseudo = pseudo
        .computed_style()?
        .first_line_style
        .clone()
        .ok_or("missing changed native pseudo style")?;
    check(
        &mut failures,
        "pseudo after font",
        changed_pseudo.font_size,
        38.0,
    );
    check(
        &mut failures,
        "pseudo after color",
        changed_pseudo.color,
        Color::BLUE,
    );
    check(
        &mut failures,
        "pseudo after weight",
        changed_pseudo.font_weight,
        FontWeight::BOLD,
    );
    let weak_child = child.downgrade();
    drop(pseudo);
    drop(child);
    drop(root);
    drop(document);
    check(
        &mut failures,
        "document lifetime",
        weak_child.upgrade().is_none(),
        true,
    );
    check(
        &mut failures,
        "owned calc snapshot",
        owned_calc.font_size,
        28.0,
    );
    check(
        &mut failures,
        "owned pseudo snapshot",
        owned_pseudo.font_size,
        32.0,
    );
    if !failures.is_empty() {
        return Err(failures.join("\n").into());
    }
    println!("native relative styles: scale={scale} callback=1 images=10 owned-snapshots=2 passed");
    Ok(())
}
