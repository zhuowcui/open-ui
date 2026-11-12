//! Badge: shadcn/ui `badge.tsx` (a `<span>` styled by `badgeVariants`).
//!
//! Base classes: `inline-flex w-fit shrink-0 items-center justify-center
//! gap-1 overflow-hidden rounded-full border border-transparent px-2
//! py-0.5 text-xs font-medium whitespace-nowrap`. The `outline` variant
//! replaces `border-transparent` with `border-border`. Hover styles apply
//! only to link badges (`[a&]:hover`) and focus styles are interactive;
//! neither is built yet.

use openui::prelude::*;
use openui::typed_style::{Length, LengthValue, StyleProperty};

use crate::css::{element, set, set_border, set_border_color, set_color, with_opacity};
use crate::current_theme;

/// `variant` in shadcn's `badgeVariants`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum BadgeVariant {
    /// `bg-primary text-primary-foreground`
    #[default]
    Default,
    /// `bg-secondary text-secondary-foreground`
    Secondary,
    /// `bg-destructive text-white dark:bg-destructive/60`
    Destructive,
    /// `border-border text-foreground`
    Outline,
    /// No background or border.
    Ghost,
    /// `text-primary underline-offset-4`
    Link,
}

/// A shadcn/ui badge holding `children`.
#[allow(non_snake_case)]
#[component]
pub fn Badge(variant: BadgeVariant, children: Vec<ViewNode>) -> ViewNode {
    let theme = current_theme();
    let badge = element(
        "span",
        "badge",
        &[
            ("display", "inline-flex"),
            ("flex-shrink", "0"),
            ("align-items", "center"),
            ("justify-content", "center"),
            ("gap", "4px"),
            ("overflow", "hidden"),
            // `rounded-full` is calc(infinity * 1px); any radius of at least
            // half the height draws the same pill once the corners are
            // scaled to fit.
            ("border-radius", "9999px"),
            ("padding", "2px 8px"),
            // `text-xs`: 0.75rem with line-height calc(1 / 0.75).
            ("font-size", "12px"),
            ("line-height", "16px"),
            ("font-weight", "500"),
            ("white-space", "nowrap"),
        ],
    );
    // `w-fit`. Open UI's length literal has no intrinsic keywords, so the
    // typed value is set.
    badge
        .set_property(
            StyleProperty::Width,
            LengthValue::Computed(Length::fit_content()).into(),
        )
        .expect("set width");
    badge
        .set_attribute("data-variant", variant.name())
        .expect("set data-variant");
    set_border(&badge, "1");
    set_border_color(&badge, "transparent");

    match variant {
        BadgeVariant::Default => {
            set_color(&badge, "background-color", theme.primary);
            set_color(&badge, "color", theme.primary_foreground);
        }
        BadgeVariant::Secondary => {
            set_color(&badge, "background-color", theme.secondary);
            set_color(&badge, "color", theme.secondary_foreground);
        }
        BadgeVariant::Destructive => {
            let background = if theme.dark {
                with_opacity(theme.destructive, 60)
            } else {
                theme.destructive.css().to_owned()
            };
            set_color(&badge, "background-color", background);
            set(&badge, &[("color", "#fff")]);
        }
        BadgeVariant::Outline => {
            set_border_color(&badge, theme.border);
            set_color(&badge, "color", theme.foreground);
        }
        BadgeVariant::Ghost => {}
        BadgeVariant::Link => {
            set_color(&badge, "color", theme.primary);
            set(&badge, &[("text-underline-offset", "4px")]);
        }
    }

    for child in children {
        mount_view(&badge, child);
    }
    ViewNode::Element(badge)
}

impl BadgeVariant {
    /// The variant's name in shadcn's `data-variant` attribute.
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Secondary => "secondary",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
            Self::Ghost => "ghost",
            Self::Link => "link",
        }
    }
}
