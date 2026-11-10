//! Button: shadcn/ui `button.tsx` (a `<button>` styled by `buttonVariants`).
//!
//! Base classes: `inline-flex shrink-0 items-center justify-center gap-2
//! rounded-md text-sm font-medium whitespace-nowrap outline-none
//! disabled:pointer-events-none disabled:opacity-50`, plus Tailwind's
//! preflight for buttons (`font: inherit; color: inherit;
//! background-color: transparent`). Open UI's literals have no `inherit`;
//! its button already inherits the font, and the text color is set from the
//! theme, which is what `inherit` gives on a page whose text is
//! `--foreground`. Hover, focus and `aria-invalid` styles
//! are interactive and not built yet. The icon sizes (`icon`, `icon-xs`,
//! `icon-sm`, `icon-lg`) are left out until Open UI can draw Lucide icons.

use openui::prelude::*;

use crate::css::{
    black_shadow, element, set, set_border, set_border_color, set_box_shadow, set_color,
    with_opacity,
};
use crate::current_theme;

/// `variant` in shadcn's `buttonVariants`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// `bg-primary text-primary-foreground`
    #[default]
    Default,
    /// `bg-destructive text-white dark:bg-destructive/60`
    Destructive,
    /// `border bg-background shadow-xs dark:border-input dark:bg-input/30`
    Outline,
    /// `bg-secondary text-secondary-foreground`
    Secondary,
    /// No background or border until hovered.
    Ghost,
    /// `text-primary underline-offset-4`, underlined on hover.
    Link,
}

/// `size` in shadcn's `buttonVariants`, except the icon sizes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonSize {
    /// `h-9 px-4 py-2`
    #[default]
    Default,
    /// `h-6 gap-1 px-2 text-xs`
    Xs,
    /// `h-8 gap-1.5 px-3`
    Sm,
    /// `h-10 px-6`
    Lg,
}

/// A shadcn/ui button holding `children`.
///
/// Every prop is required in `view!`, which does not fill omitted optional
/// props yet.
#[allow(non_snake_case)]
#[component]
pub fn Button(
    variant: ButtonVariant,
    size: ButtonSize,
    disabled: bool,
    children: Vec<ViewNode>,
) -> ViewNode {
    let theme = current_theme();
    // `rounded-md`: calc(var(--radius) * 0.8).
    let radius = format!("{}px", theme.radius * 0.8);
    let button = element(
        "button",
        "button",
        &[
            ("background-color", "transparent"),
            ("display", "inline-flex"),
            ("flex-shrink", "0"),
            ("align-items", "center"),
            ("justify-content", "center"),
            ("gap", "8px"),
            ("border-radius", &radius),
            // `text-sm`: 0.875rem with line-height calc(1.25 / 0.875), which
            // is 20px at this size.
            ("font-size", "14px"),
            ("line-height", "20px"),
            ("font-weight", "500"),
            ("white-space", "nowrap"),
            ("outline-style", "none"),
        ],
    );
    // Chromium drops a button's native look once the author styles its
    // background or border; Open UI is told explicitly.
    button
        .set_native_appearance(false)
        .expect("authored button appearance");
    button
        .set_attribute("data-variant", variant.name())
        .expect("set data-variant");
    button
        .set_attribute("data-size", size.name())
        .expect("set data-size");
    set_color(&button, "color", theme.foreground);

    match size {
        ButtonSize::Default => set(&button, &[("height", "36px"), ("padding", "8px 16px")]),
        ButtonSize::Xs => set(
            &button,
            &[
                ("height", "24px"),
                ("gap", "4px"),
                ("padding", "0 8px"),
                // `text-xs`: 0.75rem with line-height calc(1 / 0.75).
                ("font-size", "12px"),
                ("line-height", "16px"),
            ],
        ),
        ButtonSize::Sm => set(
            &button,
            &[("height", "32px"), ("gap", "6px"), ("padding", "0 12px")],
        ),
        ButtonSize::Lg => set(&button, &[("height", "40px"), ("padding", "0 24px")]),
    }

    match variant {
        ButtonVariant::Default => {
            set_color(&button, "background-color", theme.primary);
            set_color(&button, "color", theme.primary_foreground);
        }
        ButtonVariant::Destructive => {
            let background = if theme.dark {
                with_opacity(theme.destructive, 60.0)
            } else {
                theme.destructive
            };
            set_color(&button, "background-color", background);
            // `text-white`: Tailwind's `--color-white`, #fff.
            set(&button, &[("color", "#fff")]);
        }
        ButtonVariant::Outline => {
            set_border(&button, "1");
            // `shadow-xs`: 0 1px 2px 0 rgb(0 0 0 / 0.05)
            set_box_shadow(&button, vec![black_shadow(0.0, 1.0, 2.0, 0.0, 0.05)]);
            if theme.dark {
                set_border_color(&button, theme.input);
                set_color(&button, "background-color", with_opacity(theme.input, 30.0));
            } else {
                // Every element's border is `--border` (shadcn's base layer).
                set_border_color(&button, theme.border);
                set_color(&button, "background-color", theme.background);
            }
        }
        ButtonVariant::Secondary => {
            set_color(&button, "background-color", theme.secondary);
            set_color(&button, "color", theme.secondary_foreground);
        }
        ButtonVariant::Ghost => {}
        ButtonVariant::Link => {
            set_color(&button, "color", theme.primary);
            set(&button, &[("text-underline-offset", "4px")]);
        }
    }

    if disabled {
        button.set_attribute("disabled", "").expect("set disabled");
        set(&button, &[("pointer-events", "none"), ("opacity", "0.5")]);
    }

    for child in children {
        mount_view(&button, child);
    }
    ViewNode::Element(button)
}

impl ButtonVariant {
    /// The variant's name in shadcn's `data-variant` attribute.
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Destructive => "destructive",
            Self::Outline => "outline",
            Self::Secondary => "secondary",
            Self::Ghost => "ghost",
            Self::Link => "link",
        }
    }
}

impl ButtonSize {
    /// The size's name in shadcn's `data-size` attribute.
    pub fn name(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Xs => "xs",
            Self::Sm => "sm",
            Self::Lg => "lg",
        }
    }
}
