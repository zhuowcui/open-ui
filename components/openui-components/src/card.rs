//! Card: shadcn/ui `card.tsx`, seven `div` slots.
//!
//! `Card`: `flex flex-col gap-6 rounded-xl border bg-card py-6
//! text-card-foreground shadow-sm`. `CardHeader`: `@container/card-header
//! grid auto-rows-min grid-rows-[auto_auto] items-start gap-2 px-6`, plus
//! `grid-cols-[1fr_auto]` when it holds a `CardAction`. The
//! `[.border-b]:pb-6` and `[.border-t]:pt-6` refinements apply only when a
//! caller adds a border class and are not built.

use openui::prelude::*;
use openui::typed_style::{
    ContainerType, GridLine, GridPlacement, RendererStyleValue, StyleProperty,
};

use crate::css::{
    black_shadow, element, set, set_border, set_border_color, set_box_shadow, set_color, set_typed,
};
use crate::current_theme;

fn mount(parent: &Element, children: Vec<ViewNode>) {
    for child in children {
        mount_view(parent, child);
    }
}

/// The card's outer box. `css` adds declarations, as shadcn's `className`
/// adds classes (`w-full max-w-sm` is
/// `&[("width", "100%"), ("max-width", "384px")]`).
#[allow(non_snake_case)]
#[component]
pub fn Card(css: &'static [(&'static str, &'static str)], children: Vec<ViewNode>) -> ViewNode {
    let theme = current_theme();
    // `rounded-xl`: calc(var(--radius) * 1.4).
    let radius = format!("{}px", theme.radius * 1.4);
    let card = element(
        "div",
        "card",
        &[
            ("display", "flex"),
            ("flex-direction", "column"),
            ("gap", "24px"),
            ("border-radius", &radius),
            ("padding", "24px 0"),
        ],
    );
    // `shadow-sm`: 0 1px 3px 0 rgb(0 0 0 / 0.1), 0 1px 2px -1px rgb(0 0 0 / 0.1)
    set_box_shadow(
        &card,
        vec![
            black_shadow(0.0, 1.0, 3.0, 0.0, 0.1),
            black_shadow(0.0, 1.0, 2.0, -1.0, 0.1),
        ],
    );
    set_border(&card, "1");
    set_border_color(&card, theme.border);
    set_color(&card, "background-color", theme.card);
    set_color(&card, "color", theme.card_foreground);
    set(&card, css);
    mount(&card, children);
    ViewNode::Element(card)
}

/// The header grid: title and description in two rows, and an optional
/// `CardAction` in a second column spanning both.
#[allow(non_snake_case)]
#[component]
pub fn CardHeader(children: Vec<ViewNode>) -> ViewNode {
    let header = element(
        "div",
        "card-header",
        &[
            ("display", "grid"),
            ("grid-auto-rows", "min-content"),
            ("grid-template-rows", "auto auto"),
            ("align-items", "flex-start"),
            ("gap", "8px"),
            ("padding", "0 24px"),
        ],
    );
    // `@container/card-header`. Open UI has no literal for either property,
    // so the typed values are set.
    set_typed(
        &header,
        StyleProperty::ContainerType,
        RendererStyleValue::ContainerType(ContainerType::InlineSize),
    );
    set_typed(
        &header,
        StyleProperty::ContainerName,
        RendererStyleValue::ContainerName(vec!["card-header".to_owned()]),
    );
    mount(&header, children);
    // `has-data-[slot=card-action]:grid-cols-[1fr_auto]`
    if has_child_slot(&header, "card-action") {
        set(&header, &[("grid-template-columns", "1fr auto")]);
    }
    ViewNode::Element(header)
}

fn has_child_slot(parent: &Element, slot: &str) -> bool {
    let mut child = parent.first_child().expect("first child");
    while let Some(element) = child {
        if element
            .get_attribute("data-slot")
            .expect("data-slot")
            .as_deref()
            == Some(slot)
        {
            return true;
        }
        child = element.next_sibling().expect("next sibling");
    }
    false
}

/// `leading-none font-semibold`
#[allow(non_snake_case)]
#[component]
pub fn CardTitle(children: Vec<ViewNode>) -> ViewNode {
    let title = element(
        "div",
        "card-title",
        &[("line-height", "1"), ("font-weight", "600")],
    );
    mount(&title, children);
    ViewNode::Element(title)
}

/// `text-sm text-muted-foreground`
#[allow(non_snake_case)]
#[component]
pub fn CardDescription(children: Vec<ViewNode>) -> ViewNode {
    let description = element(
        "div",
        "card-description",
        &[("font-size", "14px"), ("line-height", "20px")],
    );
    set_color(&description, "color", current_theme().muted_foreground);
    mount(&description, children);
    ViewNode::Element(description)
}

/// `col-start-2 row-span-2 row-start-1 self-start justify-self-end`
#[allow(non_snake_case)]
#[component]
pub fn CardAction(children: Vec<ViewNode>) -> ViewNode {
    let action = element(
        "div",
        "card-action",
        &[("align-self", "flex-start"), ("justify-self", "flex-end")],
    );
    // `col-start-2` and `row-start-1 row-span-2`, as typed values: Open UI
    // has no grid placement literal.
    set_typed(
        &action,
        StyleProperty::GridColumn,
        RendererStyleValue::GridColumn(GridPlacement {
            start: GridLine::Line {
                index: 2,
                name: None,
            },
            end: GridLine::Auto,
        }),
    );
    set_typed(
        &action,
        StyleProperty::GridRow,
        RendererStyleValue::GridRow(GridPlacement {
            start: GridLine::Line {
                index: 1,
                name: None,
            },
            end: GridLine::Span {
                count: 2,
                name: None,
            },
        }),
    );
    mount(&action, children);
    ViewNode::Element(action)
}

/// `px-6`
#[allow(non_snake_case)]
#[component]
pub fn CardContent(children: Vec<ViewNode>) -> ViewNode {
    let content = element("div", "card-content", &[("padding", "0 24px")]);
    mount(&content, children);
    ViewNode::Element(content)
}

/// `flex items-center px-6`
#[allow(non_snake_case)]
#[component]
pub fn CardFooter(children: Vec<ViewNode>) -> ViewNode {
    let footer = element(
        "div",
        "card-footer",
        &[
            ("display", "flex"),
            ("align-items", "center"),
            ("padding", "0 24px"),
        ],
    );
    mount(&footer, children);
    ViewNode::Element(footer)
}
