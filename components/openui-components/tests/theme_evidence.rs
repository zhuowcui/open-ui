//! Every stand-in theme color matches the recorded Chromium conversion.

use openui::prelude::Color;
use openui_components::Theme;

const EVIDENCE: &str = include_str!("../../evidence/theme-tokens-v1.json");

fn field(theme: &Theme, token: &str) -> Color {
    match token {
        "background" => theme.background,
        "foreground" => theme.foreground,
        "card" => theme.card,
        "card-foreground" => theme.card_foreground,
        "popover" => theme.popover,
        "popover-foreground" => theme.popover_foreground,
        "primary" => theme.primary,
        "primary-foreground" => theme.primary_foreground,
        "secondary" => theme.secondary,
        "secondary-foreground" => theme.secondary_foreground,
        "muted" => theme.muted,
        "muted-foreground" => theme.muted_foreground,
        "accent" => theme.accent,
        "accent-foreground" => theme.accent_foreground,
        "destructive" => theme.destructive,
        "border" => theme.border,
        "input" => theme.input,
        "ring" => theme.ring,
        "chart-1" => theme.chart_1,
        "chart-2" => theme.chart_2,
        "chart-3" => theme.chart_3,
        "chart-4" => theme.chart_4,
        "chart-5" => theme.chart_5,
        "sidebar" => theme.sidebar,
        "sidebar-foreground" => theme.sidebar_foreground,
        "sidebar-primary" => theme.sidebar_primary,
        "sidebar-primary-foreground" => theme.sidebar_primary_foreground,
        "sidebar-accent" => theme.sidebar_accent,
        "sidebar-accent-foreground" => theme.sidebar_accent_foreground,
        "sidebar-border" => theme.sidebar_border,
        "sidebar-ring" => theme.sidebar_ring,
        other => panic!("theme has no field for --{other}"),
    }
}

#[test]
fn stand_in_colors_match_the_chromium_evidence() {
    let evidence: serde_json::Value = serde_json::from_str(EVIDENCE).unwrap();
    let tokens = evidence["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 62);
    for token in tokens {
        let theme = match token["theme"].as_str().unwrap() {
            "light" => Theme::LIGHT,
            "dark" => Theme::DARK,
            other => panic!("unknown theme {other}"),
        };
        let name = token["token"].as_str().unwrap();
        let rgba = token["stand_in_rgba"].as_array().unwrap();
        let channel = |index: usize| rgba[index].as_f64().unwrap() as f32;
        let expected = Color {
            r: channel(0),
            g: channel(1),
            b: channel(2),
            a: channel(3),
        };
        let theme_name = token["theme"].as_str().unwrap();
        assert_eq!(
            field(&theme, name),
            expected,
            "--{name} in the {theme_name} theme"
        );
    }
}

#[test]
fn radius_matches_the_pinned_globals() {
    let globals = include_str!("../../reference/shadcn/globals.css");
    assert!(globals.contains("--radius: 0.625rem;"));
    assert_eq!(Theme::LIGHT.radius, 10.0);
    assert_eq!(Theme::DARK.radius, 10.0);
}
