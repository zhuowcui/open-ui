//! The shadcn/ui neutral theme as sRGB stand-ins.
//!
//! STAND-IN: shadcn defines every token with `oklch()`, which Open UI does not
//! parse yet. Each value here is the sRGB color Chromium 147 computes for the
//! token, as unclamped floats: `oklch(1 0 0)` is slightly outside sRGB, and
//! Chromium blends a translucent token from those floats, so 8-bit values
//! would change the painted pixels. The values and the method are recorded in
//! `components/evidence/theme-tokens-v1.json`, and a test checks every field
//! against it. Replace these with typed theme tokens once the engine track
//! lands `oklch()` and custom properties (A2).

use openui::prelude::Color;

/// A shadcn/ui color theme. Field names follow the CSS custom properties.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Theme {
    /// `--background`
    pub background: Color,
    /// `--foreground`
    pub foreground: Color,
    /// `--card`
    pub card: Color,
    /// `--card-foreground`
    pub card_foreground: Color,
    /// `--popover`
    pub popover: Color,
    /// `--popover-foreground`
    pub popover_foreground: Color,
    /// `--primary`
    pub primary: Color,
    /// `--primary-foreground`
    pub primary_foreground: Color,
    /// `--secondary`
    pub secondary: Color,
    /// `--secondary-foreground`
    pub secondary_foreground: Color,
    /// `--muted`
    pub muted: Color,
    /// `--muted-foreground`
    pub muted_foreground: Color,
    /// `--accent`
    pub accent: Color,
    /// `--accent-foreground`
    pub accent_foreground: Color,
    /// `--destructive`
    pub destructive: Color,
    /// `--border`
    pub border: Color,
    /// `--input`
    pub input: Color,
    /// `--ring`
    pub ring: Color,
    /// `--chart-1`
    pub chart_1: Color,
    /// `--chart-2`
    pub chart_2: Color,
    /// `--chart-3`
    pub chart_3: Color,
    /// `--chart-4`
    pub chart_4: Color,
    /// `--chart-5`
    pub chart_5: Color,
    /// `--sidebar`
    pub sidebar: Color,
    /// `--sidebar-foreground`
    pub sidebar_foreground: Color,
    /// `--sidebar-primary`
    pub sidebar_primary: Color,
    /// `--sidebar-primary-foreground`
    pub sidebar_primary_foreground: Color,
    /// `--sidebar-accent`
    pub sidebar_accent: Color,
    /// `--sidebar-accent-foreground`
    pub sidebar_accent_foreground: Color,
    /// `--sidebar-border`
    pub sidebar_border: Color,
    /// `--sidebar-ring`
    pub sidebar_ring: Color,
    /// Whether Tailwind's `dark:` variants apply: the page root has the
    /// `.dark` class.
    pub dark: bool,
    /// `--radius`, in CSS pixels (`0.625rem` at the 16px root font size).
    pub radius: f32,
}

const fn srgb(r: f32, g: f32, b: f32, a: f32) -> Color {
    Color { r, g, b, a }
}

impl Theme {
    /// The light neutral theme (`:root`).
    pub const LIGHT: Theme = Theme {
        // oklch(1 0 0)
        background: srgb(0.999871, 1.00005, 1.00007, 1.0),
        // oklch(0.145 0 0)
        foreground: srgb(0.0393766, 0.039393, 0.0393945, 1.0),
        // oklch(1 0 0)
        card: srgb(0.999871, 1.00005, 1.00007, 1.0),
        // oklch(0.145 0 0)
        card_foreground: srgb(0.0393766, 0.039393, 0.0393945, 1.0),
        // oklch(1 0 0)
        popover: srgb(0.999871, 1.00005, 1.00007, 1.0),
        // oklch(0.145 0 0)
        popover_foreground: srgb(0.0393766, 0.039393, 0.0393945, 1.0),
        // oklch(0.205 0 0)
        primary: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.985 0 0)
        primary_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.97 0 0)
        secondary: srgb(0.960462, 0.960638, 0.960654, 1.0),
        // oklch(0.205 0 0)
        secondary_foreground: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.97 0 0)
        muted: srgb(0.960462, 0.960638, 0.960654, 1.0),
        // oklch(0.556 0 0)
        muted_foreground: srgb(0.451457, 0.451545, 0.451553, 1.0),
        // oklch(0.97 0 0)
        accent: srgb(0.960462, 0.960638, 0.960654, 1.0),
        // oklch(0.205 0 0)
        accent_foreground: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.577 0.245 27.325)
        destructive: srgb(0.906415, -0.0958511, 0.0423579, 1.0),
        // oklch(0.922 0 0)
        border: srgb(0.898044, 0.898209, 0.898224, 1.0),
        // oklch(0.922 0 0)
        input: srgb(0.898044, 0.898209, 0.898224, 1.0),
        // oklch(0.708 0 0)
        ring: srgb(0.630079, 0.630198, 0.630209, 1.0),
        // oklch(0.646 0.222 41.116)
        chart_1: srgb(0.960654, 0.288238, -0.154334, 1.0),
        // oklch(0.6 0.118 184.704)
        chart_2: srgb(-0.164675, 0.589295, 0.537325, 1.0),
        // oklch(0.398 0.07 227.392)
        chart_3: srgb(0.06359, 0.306725, 0.393768, 1.0),
        // oklch(0.828 0.189 84.429)
        chart_4: srgb(1.00023, 0.727298, -0.229805, 1.0),
        // oklch(0.769 0.188 70.08)
        chart_5: srgb(0.994197, 0.602133, -0.217475, 1.0),
        // oklch(0.985 0 0)
        sidebar: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.145 0 0)
        sidebar_foreground: srgb(0.0393766, 0.039393, 0.0393945, 1.0),
        // oklch(0.205 0 0)
        sidebar_primary: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.985 0 0)
        sidebar_primary_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.97 0 0)
        sidebar_accent: srgb(0.960462, 0.960638, 0.960654, 1.0),
        // oklch(0.205 0 0)
        sidebar_accent_foreground: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.922 0 0)
        sidebar_border: srgb(0.898044, 0.898209, 0.898224, 1.0),
        // oklch(0.708 0 0)
        sidebar_ring: srgb(0.630079, 0.630198, 0.630209, 1.0),
        dark: false,
        radius: 10.0,
    };

    /// The dark neutral theme (`.dark`).
    pub const DARK: Theme = Theme {
        // oklch(0.145 0 0)
        background: srgb(0.0393766, 0.039393, 0.0393945, 1.0),
        // oklch(0.985 0 0)
        foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.205 0 0)
        card: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.985 0 0)
        card_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.205 0 0)
        popover: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.985 0 0)
        popover_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.922 0 0)
        primary: srgb(0.898044, 0.898209, 0.898224, 1.0),
        // oklch(0.205 0 0)
        primary_foreground: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.269 0 0)
        secondary: srgb(0.149357, 0.149392, 0.149396, 1.0),
        // oklch(0.985 0 0)
        secondary_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.269 0 0)
        muted: srgb(0.149357, 0.149392, 0.149396, 1.0),
        // oklch(0.708 0 0)
        muted_foreground: srgb(0.630079, 0.630198, 0.630209, 1.0),
        // oklch(0.269 0 0)
        accent: srgb(0.149357, 0.149392, 0.149396, 1.0),
        // oklch(0.985 0 0)
        accent_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.704 0.191 22.216)
        destructive: srgb(1.0055, 0.391199, 0.403904, 1.0),
        // oklch(1 0 0 / 10%)
        border: srgb(0.999871, 1.00005, 1.00007, 0.1),
        // oklch(1 0 0 / 15%)
        input: srgb(0.999871, 1.00005, 1.00007, 0.15),
        // oklch(0.556 0 0)
        ring: srgb(0.451457, 0.451545, 0.451553, 1.0),
        // oklch(0.488 0.243 264.376)
        chart_1: srgb(0.0778321, 0.279166, 0.901828, 1.0),
        // oklch(0.696 0.17 162.48)
        chart_2: srgb(-0.216948, 0.73882, 0.489318, 1.0),
        // oklch(0.769 0.188 70.08)
        chart_3: srgb(0.994197, 0.602133, -0.217475, 1.0),
        // oklch(0.627 0.265 303.9)
        chart_4: srgb(0.677871, 0.275996, 1.02715, 1.0),
        // oklch(0.645 0.246 16.439)
        chart_5: srgb(1.00331, 0.124723, 0.33905, 1.0),
        // oklch(0.205 0 0)
        sidebar: srgb(0.0905095, 0.0905347, 0.090537, 1.0),
        // oklch(0.985 0 0)
        sidebar_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.488 0.243 264.376)
        sidebar_primary: srgb(0.0778321, 0.279166, 0.901828, 1.0),
        // oklch(0.985 0 0)
        sidebar_primary_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(0.269 0 0)
        sidebar_accent: srgb(0.149357, 0.149392, 0.149396, 1.0),
        // oklch(0.985 0 0)
        sidebar_accent_foreground: srgb(0.980129, 0.980308, 0.980325, 1.0),
        // oklch(1 0 0 / 10%)
        sidebar_border: srgb(0.999871, 1.00005, 1.00007, 0.1),
        // oklch(0.556 0 0)
        sidebar_ring: srgb(0.451457, 0.451545, 0.451553, 1.0),
        dark: true,
        radius: 10.0,
    };
}
