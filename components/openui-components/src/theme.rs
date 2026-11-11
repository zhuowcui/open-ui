//! The shadcn/ui neutral theme.
//!
//! Each token is the CSS value shadcn's `globals.css` gives it, an `oklch()`
//! color, and Open UI resolves it as Chromium 147 does. A test checks every
//! token against `components/evidence/theme-tokens-v1.json`: its specified
//! value, and the sRGB color Chromium computes for it.
//!
//! STAND-IN: shadcn reads the tokens through custom properties
//! (`var(--primary)`), which Open UI does not have yet; components write the
//! token's value in its place. Replace with typed theme tokens once the
//! engine track lands custom properties (A2).

use openui::prelude::Color;
use openui::typed_style::{parse_literal, RendererStyleValue, StyleProperty, StyleValue};

/// A theme color: the CSS value of one shadcn custom property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Token(pub &'static str);

impl Token {
    /// The CSS value, for example `oklch(0.205 0 0)`.
    pub const fn css(self) -> &'static str {
        self.0
    }

    /// The color Open UI resolves the value to.
    pub fn color(self) -> Color {
        match parse_literal(StyleProperty::Color, self.0) {
            Ok(StyleValue::Color(color))
            | Ok(StyleValue::Renderer(RendererStyleValue::Color(color))) => color,
            other => panic!("theme token `{}` is not a color: {other:?}", self.0),
        }
    }
}

impl AsRef<str> for Token {
    fn as_ref(&self) -> &str {
        self.0
    }
}

/// A shadcn/ui color theme. Field names follow the CSS custom properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// `--background`
    pub background: Token,
    /// `--foreground`
    pub foreground: Token,
    /// `--card`
    pub card: Token,
    /// `--card-foreground`
    pub card_foreground: Token,
    /// `--popover`
    pub popover: Token,
    /// `--popover-foreground`
    pub popover_foreground: Token,
    /// `--primary`
    pub primary: Token,
    /// `--primary-foreground`
    pub primary_foreground: Token,
    /// `--secondary`
    pub secondary: Token,
    /// `--secondary-foreground`
    pub secondary_foreground: Token,
    /// `--muted`
    pub muted: Token,
    /// `--muted-foreground`
    pub muted_foreground: Token,
    /// `--accent`
    pub accent: Token,
    /// `--accent-foreground`
    pub accent_foreground: Token,
    /// `--destructive`
    pub destructive: Token,
    /// `--border`
    pub border: Token,
    /// `--input`
    pub input: Token,
    /// `--ring`
    pub ring: Token,
    /// `--chart-1`
    pub chart_1: Token,
    /// `--chart-2`
    pub chart_2: Token,
    /// `--chart-3`
    pub chart_3: Token,
    /// `--chart-4`
    pub chart_4: Token,
    /// `--chart-5`
    pub chart_5: Token,
    /// `--sidebar`
    pub sidebar: Token,
    /// `--sidebar-foreground`
    pub sidebar_foreground: Token,
    /// `--sidebar-primary`
    pub sidebar_primary: Token,
    /// `--sidebar-primary-foreground`
    pub sidebar_primary_foreground: Token,
    /// `--sidebar-accent`
    pub sidebar_accent: Token,
    /// `--sidebar-accent-foreground`
    pub sidebar_accent_foreground: Token,
    /// `--sidebar-border`
    pub sidebar_border: Token,
    /// `--sidebar-ring`
    pub sidebar_ring: Token,
    /// Whether Tailwind's `dark:` variants apply: the page root has the
    /// `.dark` class.
    pub dark: bool,
    /// `--radius`, in CSS pixels (`0.625rem` at the 16px root font size).
    pub radius: f32,
}

impl Theme {
    /// The light neutral theme (`:root`).
    pub const LIGHT: Theme = Theme {
        background: Token("oklch(1 0 0)"),
        foreground: Token("oklch(0.145 0 0)"),
        card: Token("oklch(1 0 0)"),
        card_foreground: Token("oklch(0.145 0 0)"),
        popover: Token("oklch(1 0 0)"),
        popover_foreground: Token("oklch(0.145 0 0)"),
        primary: Token("oklch(0.205 0 0)"),
        primary_foreground: Token("oklch(0.985 0 0)"),
        secondary: Token("oklch(0.97 0 0)"),
        secondary_foreground: Token("oklch(0.205 0 0)"),
        muted: Token("oklch(0.97 0 0)"),
        muted_foreground: Token("oklch(0.556 0 0)"),
        accent: Token("oklch(0.97 0 0)"),
        accent_foreground: Token("oklch(0.205 0 0)"),
        destructive: Token("oklch(0.577 0.245 27.325)"),
        border: Token("oklch(0.922 0 0)"),
        input: Token("oklch(0.922 0 0)"),
        ring: Token("oklch(0.708 0 0)"),
        chart_1: Token("oklch(0.646 0.222 41.116)"),
        chart_2: Token("oklch(0.6 0.118 184.704)"),
        chart_3: Token("oklch(0.398 0.07 227.392)"),
        chart_4: Token("oklch(0.828 0.189 84.429)"),
        chart_5: Token("oklch(0.769 0.188 70.08)"),
        sidebar: Token("oklch(0.985 0 0)"),
        sidebar_foreground: Token("oklch(0.145 0 0)"),
        sidebar_primary: Token("oklch(0.205 0 0)"),
        sidebar_primary_foreground: Token("oklch(0.985 0 0)"),
        sidebar_accent: Token("oklch(0.97 0 0)"),
        sidebar_accent_foreground: Token("oklch(0.205 0 0)"),
        sidebar_border: Token("oklch(0.922 0 0)"),
        sidebar_ring: Token("oklch(0.708 0 0)"),
        dark: false,
        radius: 10.0,
    };

    /// The dark neutral theme (`.dark`).
    pub const DARK: Theme = Theme {
        background: Token("oklch(0.145 0 0)"),
        foreground: Token("oklch(0.985 0 0)"),
        card: Token("oklch(0.205 0 0)"),
        card_foreground: Token("oklch(0.985 0 0)"),
        popover: Token("oklch(0.205 0 0)"),
        popover_foreground: Token("oklch(0.985 0 0)"),
        primary: Token("oklch(0.922 0 0)"),
        primary_foreground: Token("oklch(0.205 0 0)"),
        secondary: Token("oklch(0.269 0 0)"),
        secondary_foreground: Token("oklch(0.985 0 0)"),
        muted: Token("oklch(0.269 0 0)"),
        muted_foreground: Token("oklch(0.708 0 0)"),
        accent: Token("oklch(0.269 0 0)"),
        accent_foreground: Token("oklch(0.985 0 0)"),
        destructive: Token("oklch(0.704 0.191 22.216)"),
        border: Token("oklch(1 0 0 / 10%)"),
        input: Token("oklch(1 0 0 / 15%)"),
        ring: Token("oklch(0.556 0 0)"),
        chart_1: Token("oklch(0.488 0.243 264.376)"),
        chart_2: Token("oklch(0.696 0.17 162.48)"),
        chart_3: Token("oklch(0.769 0.188 70.08)"),
        chart_4: Token("oklch(0.627 0.265 303.9)"),
        chart_5: Token("oklch(0.645 0.246 16.439)"),
        sidebar: Token("oklch(0.205 0 0)"),
        sidebar_foreground: Token("oklch(0.985 0 0)"),
        sidebar_primary: Token("oklch(0.488 0.243 264.376)"),
        sidebar_primary_foreground: Token("oklch(0.985 0 0)"),
        sidebar_accent: Token("oklch(0.269 0 0)"),
        sidebar_accent_foreground: Token("oklch(0.985 0 0)"),
        sidebar_border: Token("oklch(1 0 0 / 10%)"),
        sidebar_ring: Token("oklch(0.556 0 0)"),
        dark: true,
        radius: 10.0,
    };
}
