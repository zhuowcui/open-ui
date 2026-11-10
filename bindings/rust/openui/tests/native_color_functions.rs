//! `oklch()`, `oklab()` and `color-mix()` against pinned Chromium 147.
//!
//! The swatch list and the Chromium center pixels live in
//! `docs/progress/evidence/color-functions-v1`; regenerate the oracle with
//! `tools/qualification/capture_color_function_oracle.py`. This test builds
//! the same swatches through the public API and requires every center pixel
//! to match exactly.
use openui::prelude::*;
use openui::typed_style::parse_literal;

const SPEC: &str =
    include_str!("../../../../docs/progress/evidence/color-functions-v1/swatches.txt");
#[cfg(target_os = "windows")]
const ORACLE: &str =
    include_str!("../../../../docs/progress/evidence/color-functions-v1/chromium-windows.json");
#[cfg(not(target_os = "windows"))]
const ORACLE: &str =
    include_str!("../../../../docs/progress/evidence/color-functions-v1/chromium-linux.json");

const SIZE: f32 = 20.0;
const COLUMNS: usize = 10;
const VIEWPORT: f64 = 200.0;

fn swatches() -> Vec<(&'static str, &'static str)> {
    SPEC.lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let (backdrop, color) = line.split_once('|').expect("backdrop | color");
            (backdrop.trim(), color.trim())
        })
        .collect()
}

fn square(document: &Document, color: &str) -> Element {
    let element = Element::create(document, "div").unwrap();
    element.set_display(Display::Block).unwrap();
    element.set_width(LengthValue::px(SIZE)).unwrap();
    element.set_height(LengthValue::px(SIZE)).unwrap();
    element
        .set_property(
            StyleProperty::BackgroundColor,
            parse_literal(StyleProperty::BackgroundColor, color)
                .unwrap_or_else(|error| panic!("{color}: {error}")),
        )
        .unwrap();
    element
}

#[test]
fn color_function_swatches_match_chromium_center_pixels() {
    let oracle: serde_json::Value = serde_json::from_str(ORACLE).unwrap();
    let expected = oracle["swatches"].as_array().unwrap();
    let items = swatches();
    assert_eq!(items.len(), expected.len(), "oracle is stale: recapture it");

    let document = Document::with_viewport_metrics(
        ViewportMetrics::from_logical_size(VIEWPORT, VIEWPORT, 1.0).unwrap(),
    )
    .unwrap();
    let root = document.body();
    root.set_overflow(Overflow::Hidden).unwrap();
    root.set_background_color(Color::WHITE).unwrap();
    for (index, (backdrop, color)) in items.iter().enumerate() {
        let cell = square(&document, backdrop);
        cell.set_position(Position::Absolute).unwrap();
        cell.set_left(Length::px((index % COLUMNS) as f32 * SIZE))
            .unwrap();
        cell.set_top(Length::px((index / COLUMNS) as f32 * SIZE))
            .unwrap();
        cell.append_child(&square(&document, color)).unwrap();
        root.append_child(&cell).unwrap();
    }
    let bitmap = document.render_to_bitmap().unwrap();

    let mut mismatches = Vec::new();
    for (index, ((backdrop, color), oracle)) in items.iter().zip(expected).enumerate() {
        assert_eq!(oracle["color"].as_str(), Some(*color));
        assert_eq!(oracle["backdrop"].as_str(), Some(*backdrop));
        let x = (index % COLUMNS) * SIZE as usize + SIZE as usize / 2;
        let y = (index / COLUMNS) * SIZE as usize + SIZE as usize / 2;
        let offset = y * bitmap.stride() + x * 4;
        let actual = bitmap.pixels()[offset..offset + 4].to_vec();
        let wanted: Vec<u8> = oracle["rgba"]
            .as_array()
            .unwrap()
            .iter()
            .map(|channel| channel.as_u64().unwrap() as u8)
            .collect();
        if actual != wanted {
            mismatches.push(format!("{backdrop} | {color}: {actual:?} != {wanted:?}"));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}
