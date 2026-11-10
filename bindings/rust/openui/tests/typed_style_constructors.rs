//! `view!` parses `style:` literals at compile time and emits constructors
//! (RFC 0001). These tests check that every emitted constructor builds the
//! same value, bit for bit, as the runtime parser does for that literal.

use std::collections::{BTreeSet, HashSet};

use openui::typed_style::{parse_literal, StyleProperty, StyleValue, PROPERTY_METADATA};

fn cases() -> Vec<(StyleProperty, &'static str, StyleValue)> {
    openui_macros::__typed_style_literal_cases!()
}

#[test]
fn compile_time_constructors_rebuild_the_runtime_parse() {
    let cases = cases();
    for (property, literal, constructed) in &cases {
        let parsed = parse_literal(*property, literal)
            .unwrap_or_else(|error| panic!("{property:?} `{literal}`: {error}"));
        // Debug distinguishes every float bit pattern that matters here,
        // including -0.0, and covers the value types without PartialEq.
        assert_eq!(
            format!("{constructed:?}"),
            format!("{parsed:?}"),
            "{} `{literal}`",
            property.metadata().css_name
        );
        assert_eq!(constructed, &parsed);
    }
}

/// Renderer properties that `parse_literal` accepts some literal for: the
/// arms of the generated `parse_renderer_author_literal`. Renderer properties
/// without an arm accept no literal, so `view!` cannot set them.
fn renderer_literal_properties() -> BTreeSet<String> {
    let source = include_str!("../../openui-style/src/generated_properties.rs");
    let start = source
        .find("fn parse_renderer_author_literal(")
        .expect("generated renderer literal parser");
    let body = &source[start..];
    let body = &body[..body
        .find("\n}\n")
        .expect("end of the renderer literal parser")];
    body.lines()
        .filter_map(|line| line.trim().strip_prefix("StyleProperty::"))
        .filter_map(|rest| rest.split_once(" =>").map(|(name, _)| name.to_owned()))
        .collect()
}

#[test]
fn every_property_with_a_literal_grammar_has_a_constructor_case() {
    let covered: HashSet<StyleProperty> =
        cases().iter().map(|(property, _, _)| *property).collect();
    let renderer = renderer_literal_properties();
    assert!(
        renderer.len() > 50,
        "found {} renderer literal arms",
        renderer.len()
    );
    let missing: Vec<&str> = PROPERTY_METADATA
        .iter()
        .filter(|metadata| {
            metadata.initial != "renderer-initial"
                || renderer.contains(&format!("{:?}", metadata.property))
        })
        .filter(|metadata| !covered.contains(&metadata.property))
        .map(|metadata| metadata.css_name)
        .collect();
    assert!(
        missing.is_empty(),
        "properties with a literal grammar but no constructor case: {missing:?}"
    );
}

#[test]
fn view_style_attributes_reach_the_computed_style() {
    use openui::prelude::*;
    use openui::typed_style::{Display, Length};

    let mut app =
        HeadlessApp::new(ViewportMetrics::from_logical_size(64.0, 64.0, 1.0).unwrap()).unwrap();
    app.mount(|| {
        view! {
            <div style:padding="8px 16px" style:opacity="0.25" style:display="flex" />
        }
    })
    .unwrap();
    let element = app.document().body().first_child().unwrap().unwrap();
    let style = element.computed_style().unwrap();
    assert_eq!(*style.display(), Display::Flex);
    assert_eq!(*style.opacity(), 0.25);
    assert_eq!(*style.padding_top(), Length::px(8.0));
    assert_eq!(*style.padding_left(), Length::px(16.0));
}
