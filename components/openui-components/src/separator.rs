//! Separator: shadcn/ui `separator.tsx` over Radix `Separator.Root`.
//!
//! shadcn classes: `shrink-0 bg-border`, plus `h-px w-full` when horizontal
//! and `h-full w-px` when vertical. Radix renders a `div` with
//! `data-orientation` and, because shadcn's separator is decorative,
//! `role="none"`.

use openui::prelude::*;

use crate::current_theme;

/// The separator's direction.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Orientation {
    /// A 1px tall line as wide as its container.
    #[default]
    Horizontal,
    /// A 1px wide line as tall as its container.
    Vertical,
}

/// A decorative 1px line in the theme's border color.
///
/// `orientation` is required in `view!`, which does not fill omitted
/// optional props yet: `<Separator orientation=Orientation::Horizontal />`.
#[allow(non_snake_case)]
#[component]
pub fn Separator(orientation: Orientation) -> ViewNode {
    let border = current_theme().border;
    match orientation {
        Orientation::Horizontal => view! {
            <div
                data-slot="separator"
                data-orientation="horizontal"
                role="none"
                style:flex-shrink="0"
                style:background-color={border}
                style:height="1px"
                style:width="100%"
            />
        },
        Orientation::Vertical => view! {
            <div
                data-slot="separator"
                data-orientation="vertical"
                role="none"
                style:flex-shrink="0"
                style:background-color={border}
                style:height="100%"
                style:width="1px"
            />
        },
    }
}
