//! Every theme token is shadcn's value, and Open UI resolves it to the sRGB
//! color Chromium computes for it.

use openui_components::{Theme, Token};

const EVIDENCE: &str = include_str!("../../evidence/theme-tokens-v1.json");

fn field(theme: &Theme, token: &str) -> Token {
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
fn tokens_match_the_chromium_evidence() {
    let evidence: serde_json::Value = serde_json::from_str(EVIDENCE).unwrap();
    let tokens = evidence["tokens"].as_array().unwrap();
    assert_eq!(tokens.len(), 62);
    for token in tokens {
        let theme_name = token["theme"].as_str().unwrap();
        let theme = match theme_name {
            "light" => Theme::LIGHT,
            "dark" => Theme::DARK,
            other => panic!("unknown theme {other}"),
        };
        let name = token["token"].as_str().unwrap();
        let value = field(&theme, name);
        assert_eq!(
            value.css(),
            token["specified"].as_str().unwrap(),
            "--{name} in the {theme_name} theme"
        );
        // Chromium serializes each channel to six significant digits.
        let color = value.color();
        let resolved = [color.r, color.g, color.b, color.a];
        let rgba = token["stand_in_rgba"].as_array().unwrap();
        for (channel, expected) in resolved.iter().zip(rgba) {
            let expected = expected.as_f64().unwrap() as f32;
            assert!(
                (channel - expected).abs() <= 5e-6 * expected.abs().max(0.1),
                "--{name} in the {theme_name} theme: {resolved:?}, Chromium {rgba:?}"
            );
        }
    }
}

#[test]
fn radius_matches_the_pinned_globals() {
    let globals = include_str!("../../reference/shadcn/globals.css");
    assert!(globals.contains("--radius: 0.625rem;"));
    assert_eq!(Theme::LIGHT.radius, 10.0);
    assert_eq!(Theme::DARK.radius, 10.0);
}
