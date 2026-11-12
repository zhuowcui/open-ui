//! `style!` compiles a CSS declaration block to a typed `Style` (RFC 0001).
//! Each declaration must equal the runtime parse of the same text, in order.

use openui::style;
use openui::typed_style::{parse_literal, Declaration, Style, StyleProperty};

/// The `Style` the runtime parser builds from `(property, value)` pairs.
fn parsed(pairs: &[(&str, &str)]) -> Style {
    pairs
        .iter()
        .map(|(name, value)| {
            let property = StyleProperty::from_css_name(name).expect(name);
            Declaration {
                property,
                value: parse_literal(property, value).unwrap_or_else(|e| panic!("{name}: {e}")),
            }
        })
        .collect()
}

fn assert_same(built: &Style, expected: &Style) {
    // Debug text tells every float bit pattern apart, including -0.0.
    assert_eq!(format!("{built:?}"), format!("{expected:?}"));
    assert_eq!(built, expected);
}

#[test]
fn braced_blocks_match_the_runtime_parse() {
    let built = style! {
        display: flex;
        justify-content: space-between;
        align-items: center;
        padding: 8px 16px;
        margin-left: -10px;
        width: 50%;
        opacity: 0.25;
        border: 1px solid red;
        color: rgb(0 0 0 / 0.5);
        background-color: #0a0a0a;
        font-family: "Inter", sans-serif;
        font-weight: 600;
    };
    let expected = parsed(&[
        ("display", "flex"),
        ("justify-content", "space-between"),
        ("align-items", "center"),
        ("padding", "8px 16px"),
        ("margin-left", "-10px"),
        ("width", "50%"),
        ("opacity", "0.25"),
        ("border", "1px solid red"),
        ("color", "rgb(0 0 0 / 0.5)"),
        ("background-color", "#0a0a0a"),
        ("font-family", "\"Inter\", sans-serif"),
        ("font-weight", "600"),
    ]);
    assert_same(&built, &expected);
}

#[test]
fn comments_and_layout_do_not_change_the_block() {
    let built = style! {
        // Rust comments are not CSS.
        padding: /* top and bottom */ 8px
                 16px; // left and right
        DISPLAY: flex
    };
    assert_same(
        &built,
        &parsed(&[("padding", "8px 16px"), ("display", "flex")]),
    );
}

#[test]
fn string_blocks_take_css_rust_cannot_tokenize() {
    let built = style!("color: #3e3; font-family: 'Inter'");
    assert_same(
        &built,
        &parsed(&[("color", "#3e3"), ("font-family", "'Inter'")]),
    );
}

macro_rules! wrapped {
    ($css:literal) => {
        style!($css)
    };
}

#[test]
fn string_blocks_work_inside_other_macros() {
    assert_same(&wrapped!("display: flex"), &parsed(&[("display", "flex")]));
}

#[test]
fn an_empty_block_is_an_empty_style() {
    assert!(style! {}.declarations().is_empty());
    assert!(style!("").declarations().is_empty());
}

#[test]
fn applied_blocks_reach_the_computed_style() {
    use openui::prelude::*;
    use openui::typed_style::{Display, Length};

    let mut app =
        HeadlessApp::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
    app.mount(|| view! { <div /> }).unwrap();
    let element = app.document().body().first_child().unwrap().unwrap();
    element
        .apply_style(&style! { display: flex; padding: 8px 16px; opacity: 0.25 })
        .unwrap();
    let style = element.computed_style().unwrap();
    assert_eq!(*style.display(), Display::Flex);
    assert_eq!(*style.opacity(), 0.25);
    assert_eq!(*style.padding_top(), Length::px(8.0));
    assert_eq!(*style.padding_left(), Length::px(16.0));
}
