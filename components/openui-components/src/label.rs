//! Label: shadcn/ui `label.tsx` over Radix `Label.Root`, a `<label>`.
//!
//! Classes: `flex items-center gap-2 text-sm leading-none font-medium
//! select-none`. Open UI has no `user-select` property, which does not
//! change the pixels; the disabled-peer styles are interactive and not
//! built yet.

use openui::prelude::*;

use crate::css::element;

/// A shadcn/ui label holding `children`.
#[allow(non_snake_case)]
#[component]
pub fn Label(children: Vec<ViewNode>) -> ViewNode {
    let label = element(
        "label",
        "label",
        &[
            ("display", "flex"),
            ("align-items", "center"),
            ("gap", "8px"),
            ("font-size", "14px"),
            // `leading-none` wins over `text-sm`'s line height.
            ("line-height", "1"),
            ("font-weight", "500"),
        ],
    );
    for child in children {
        mount_view(&label, child);
    }
    ViewNode::Element(label)
}
