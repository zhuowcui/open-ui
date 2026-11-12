//! Inline styles written as CSS declarations.
//!
//! Components set the declarations Tailwind compiles their classes to,
//! property by property, through `Element::set_property`. The values are
//! the computed values in `components/reference/pages/reference.css`; where
//! Open UI has no longhand, the shorthand with the same computed value is
//! used (`grid-row: 1 / span 2` for `row-start-1 row-span-2`).
//!
//! Every element on a shadcn page gets Tailwind's preflight first:
//! `box-sizing: border-box; margin: 0; padding: 0; border: 0 solid`.
//! [`element`] applies the same reset.

use openui::prelude::Color;
use openui::typed_style::{
    parse_literal, BoxShadow, RendererStyleValue, StyleProperty, StyleValue,
};
use openui::{current_document, Element};

/// Create `tag` in the current document with shadcn's `data-slot` and
/// Tailwind's preflight reset, then `declarations`.
pub(crate) fn element(tag: &str, slot: &str, declarations: &[(&str, &str)]) -> Element {
    let element = Element::create(&current_document(), tag).expect("create element");
    element
        .set_attribute("data-slot", slot)
        .expect("set data-slot");
    set(
        &element,
        &[
            ("box-sizing", "border-box"),
            ("margin", "0"),
            ("padding", "0"),
        ],
    );
    // `border: 0 solid`, leaving the color at its initial `currentcolor`.
    set_border(&element, "0");
    set(&element, declarations);
    element
}

/// Set each `(property, value)` declaration on `element`.
///
/// The declarations are constants of this crate, so an unknown property or
/// a value Open UI cannot parse is a bug here and panics with its name.
pub(crate) fn set(element: &Element, declarations: &[(&str, &str)]) {
    for (name, value) in declarations {
        let property = StyleProperty::from_css_name(name)
            .unwrap_or_else(|| panic!("Open UI has no CSS property `{name}`"));
        let parsed = parse_literal(property, value)
            .unwrap_or_else(|error| panic!("`{name}: {value}` does not parse: {error}"));
        element
            .set_property(property, parsed)
            .unwrap_or_else(|error| panic!("`{name}: {value}`: {error}"));
    }
}

/// Set a color property from a CSS color: a theme [`Token`](crate::Token)
/// or a value built from one ([`with_opacity`]).
pub(crate) fn set_color(element: &Element, name: &str, color: impl AsRef<str>) {
    set(element, &[(name, color.as_ref())]);
}

/// Set a solid border of `width` on all four sides, as Tailwind's `border`
/// class does over preflight's `border-style: solid`. `width` is in whole
/// CSS pixels: Open UI's `border-*-width` literal is a bare integer, not a
/// length. Open UI's `border`
/// shorthand literal needs a width, a style and a color, and accepts no
/// `currentcolor`, so the longhands are set instead.
pub(crate) fn set_border(element: &Element, width: &str) {
    for side in ["top", "right", "bottom", "left"] {
        set(
            element,
            &[
                (&format!("border-{side}-width"), width),
                (&format!("border-{side}-style"), "solid"),
            ],
        );
    }
}

/// Set all four border colors (`border-color`, which Open UI has only as
/// longhands).
pub(crate) fn set_border_color(element: &Element, color: impl AsRef<str>) {
    for side in ["top", "right", "bottom", "left"] {
        set_color(element, &format!("border-{side}-color"), color.as_ref());
    }
}

/// Tailwind's opacity modifier (`bg-input/30`):
/// `color-mix(in oklab, <color> <percent>%, transparent)`.
pub(crate) fn with_opacity(color: impl AsRef<str>, percent: u8) -> String {
    format!(
        "color-mix(in oklab, {} {percent}%, transparent)",
        color.as_ref()
    )
}

/// One `box-shadow` layer: `<x> <y> <blur> <spread> rgb(0 0 0 / <alpha>)`.
///
/// Open UI has no `box-shadow` literal, so shadows are built as typed
/// values. Tailwind's shadow utilities also list four transparent
/// `0 0 #0000` layers (rings and inset shadows), which draw nothing and are
/// left out.
pub(crate) fn black_shadow(x: f32, y: f32, blur: f32, spread: f32, alpha: f32) -> BoxShadow {
    BoxShadow {
        offset_x: x,
        offset_y: y,
        blur_radius: blur,
        spread_radius: spread,
        color: Color {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: alpha,
        },
        inset: false,
    }
}

/// Set `box-shadow` to `layers`, first layer on top.
pub(crate) fn set_box_shadow(element: &Element, layers: Vec<BoxShadow>) {
    set_typed(
        element,
        StyleProperty::BoxShadow,
        RendererStyleValue::BoxShadow(layers),
    );
}

/// Set a property Open UI has no literal for from its typed value.
pub(crate) fn set_typed(element: &Element, property: StyleProperty, value: RendererStyleValue) {
    element
        .set_property(property, StyleValue::Renderer(value))
        .unwrap_or_else(|error| panic!("{property:?}: {error}"));
}
