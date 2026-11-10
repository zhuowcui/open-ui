//! `:active` covers the pressed element and its ancestors, and a disabled
//! pressed element too, as pinned Chromium 147 reports through real mouse
//! input (RFC 0001, state sources). The oracle is
//! `tools/qualification/capture_active_chain_oracle.py`.

use openui::prelude::*;
use openui::PointerEventKind;
use std::collections::BTreeSet;

const ORACLE: &str =
    include_str!("../../../../docs/progress/evidence/active-chain-v1/chromium-windows.json");

fn boxed(
    document: &Document,
    parent: &Element,
    tag: &str,
    id: &str,
    rect: (f32, f32, f32, f32),
) -> Element {
    let element = Element::create(document, tag).unwrap();
    element.set_attribute("id", id).unwrap();
    element.set_position(Position::Absolute).unwrap();
    element.set_box_sizing(BoxSizing::BorderBox).unwrap();
    element.set_left(Length::px(rect.0)).unwrap();
    element.set_top(Length::px(rect.1)).unwrap();
    element.set_width(LengthValue::px(rect.2)).unwrap();
    element.set_height(LengthValue::px(rect.3)).unwrap();
    parent.append_child(&element).unwrap();
    element
}

/// The oracle page's tree. Open UI has no `html` element: its root plays
/// `body`, so `html` is left out of the comparison.
fn page() -> (Document, Vec<(&'static str, Element)>) {
    let document = Document::new(200, 200).unwrap();
    let body = document.body();
    let outer = boxed(&document, &body, "div", "outer", (0.0, 0.0, 200.0, 100.0));
    let middle = boxed(
        &document,
        &outer,
        "div",
        "middle",
        (10.0, 10.0, 100.0, 80.0),
    );
    let inner = boxed(&document, &middle, "div", "inner", (10.0, 10.0, 40.0, 40.0));
    let sibling = boxed(
        &document,
        &outer,
        "div",
        "sibling",
        (130.0, 10.0, 40.0, 40.0),
    );
    let controls = boxed(
        &document,
        &body,
        "div",
        "controls",
        (0.0, 100.0, 200.0, 100.0),
    );
    let enabled = boxed(
        &document,
        &controls,
        "button",
        "enabled",
        (10.0, 10.0, 60.0, 30.0),
    );
    let disabled = boxed(
        &document,
        &controls,
        "button",
        "disabled",
        (100.0, 10.0, 60.0, 30.0),
    );
    disabled.set_attribute("disabled", "").unwrap();
    document.update_all().unwrap();
    let elements = vec![
        ("body", body),
        ("outer", outer),
        ("middle", middle),
        ("inner", inner),
        ("sibling", sibling),
        ("controls", controls),
        ("enabled", enabled),
        ("disabled", disabled),
    ];
    (document, elements)
}

#[test]
fn active_chain_matches_chromium() {
    let oracle: serde_json::Value = serde_json::from_str(ORACLE).unwrap();
    let (document, elements) = page();
    let steps = oracle["steps"].as_array().unwrap();
    assert_eq!(steps.len(), 7);
    for step in steps {
        for event in step["events"].as_array().unwrap() {
            let kind = match event[0].as_str().unwrap() {
                "press" => PointerEventKind::Down,
                "move" => PointerEventKind::Move,
                "release" => PointerEventKind::Up,
                other => panic!("unknown event {other}"),
            };
            let x = event[1].as_f64().unwrap() as f32;
            let y = event[2].as_f64().unwrap() as f32;
            document
                .dispatch_pointer_event(1, kind, x, y, MouseButton::Left, Modifiers::NONE, true)
                .unwrap();
        }
        let expected: BTreeSet<&str> = step["active"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_str().unwrap())
            .filter(|id| *id != "html")
            .collect();
        let actual: BTreeSet<&str> = elements
            .iter()
            .filter(|(_, element)| element.is_active().unwrap())
            .map(|(id, _)| *id)
            .collect();
        assert_eq!(actual, expected, "step {}", step["step"]);
    }
}

/// A disabled button becomes active when pressed but is not activated.
#[test]
fn a_pressed_disabled_button_is_active_but_not_clicked() {
    let (document, elements) = page();
    let disabled = &elements.iter().find(|(id, _)| *id == "disabled").unwrap().1;
    let clicks = std::rc::Rc::new(std::cell::Cell::new(0));
    let counted = clicks.clone();
    disabled
        .on("click", move |_| counted.set(counted.get() + 1))
        .unwrap();
    for kind in [PointerEventKind::Down, PointerEventKind::Up] {
        document
            .dispatch_pointer_event(
                1,
                kind,
                130.0,
                125.0,
                MouseButton::Left,
                Modifiers::NONE,
                true,
            )
            .unwrap();
        if kind == PointerEventKind::Down {
            assert!(disabled.is_active().unwrap());
        }
    }
    assert!(!disabled.is_active().unwrap());
    assert_eq!(clicks.get(), 0);
}
