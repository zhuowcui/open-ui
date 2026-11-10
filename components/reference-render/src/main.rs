//! Render the components reference fixtures with Open UI.
//!
//!     openui-components-reference list
//!     openui-components-reference render <fixture-id> <out.png> [--viewport WxH] [--scale S]
//!     openui-components-reference geometry <fixture-id> [--viewport WxH] [--scale S]
//!
//! `geometry` prints, as JSON, the border box of the fixture's root (the
//! page's `<main>`) and of every element inside it, depth first, in CSS
//! pixels. The harness compares it with Chromium's `getBoundingClientRect`
//! to tell layout differences from paint differences.
//!
//! Each fixture ID matches a page built by `components/reference/build.mjs`
//! from the pinned shadcn/ui sources. A page is `<body>` with shadcn's
//! `bg-background text-foreground`, holding `<main class="p-4">` with the
//! fixture inside; this binary builds the same tree with the public `openui`
//! crate and renders it with Chromium's Linux LCD raster configuration.

use openui::prelude::*;
use openui_components::{
    register_fonts, with_theme, Badge, BadgeProps, BadgeVariant, Button, ButtonProps, ButtonSize,
    ButtonVariant, Card, CardAction, CardActionProps, CardContent, CardContentProps,
    CardDescription, CardDescriptionProps, CardFooter, CardFooterProps, CardHeader,
    CardHeaderProps, CardProps, CardTitle, CardTitleProps, Label, LabelProps, Orientation,
    Separator, SeparatorProps, Theme,
};

type Fixture = fn() -> ViewNode;

const THEMES: [(&str, Theme); 2] = [("light", Theme::LIGHT), ("dark", Theme::DARK)];

/// Fixtures in the order of `components/reference/fixtures.tsx`, with
/// whether each draws text. Fixtures without text skip registering the
/// bundled fonts, so a font-loading failure cannot hide their results.
const FIXTURES: &[(&str, Fixture, bool)] = &[
    ("font-geist", font_geist, true),
    ("font-geist-mono", font_geist_mono, true),
    ("font-inter", font_inter, true),
    ("separator-horizontal", separator_horizontal, false),
    ("separator-vertical", separator_vertical, false),
    ("button-default", button_default, true),
    ("button-destructive", button_destructive, true),
    ("button-outline", button_outline, true),
    ("button-secondary", button_secondary, true),
    ("button-ghost", button_ghost, true),
    ("button-link", button_link, true),
    ("button-size-xs", button_size_xs, true),
    ("button-size-sm", button_size_sm, true),
    ("button-size-lg", button_size_lg, true),
    ("button-disabled", button_disabled, true),
    ("badge-default", badge_default, true),
    ("badge-secondary", badge_secondary, true),
    ("badge-destructive", badge_destructive, true),
    ("badge-outline", badge_outline, true),
    ("badge-ghost", badge_ghost, true),
    ("badge-link", badge_link, true),
    ("card-basic", card_basic, true),
    ("card-action", card_action, true),
    ("label-basic", label_basic, true),
];

// `FontSpecimen` in fixtures.tsx: `flex flex-col gap-1 text-sm` and a font
// family, holding three lines at weights 400, 500 (`font-medium`) and 600
// (`font-semibold`). Tailwind's `text-sm` is 0.875rem with line-height
// calc(1.25 / 0.875), written as the number 1.4285714 because `line-height`
// does not accept `calc()` in Open UI yet.

fn font_geist() -> ViewNode {
    view! {
        <div
            style:display="flex"
            style:flex-direction="column"
            style:gap="4px"
            style:font-size="14px"
            style:line-height="1.4285714"
            style:font-family="Geist"
        >
            <p>"Sphinx of black quartz, judge my vow."</p>
            <p style:font-weight="500">"Pack my box with five dozen jugs."</p>
            <p style:font-weight="600">"0123456789 +-=*/ &@#%?!"</p>
        </div>
    }
}

fn font_geist_mono() -> ViewNode {
    view! {
        <div
            style:display="flex"
            style:flex-direction="column"
            style:gap="4px"
            style:font-size="14px"
            style:line-height="1.4285714"
            style:font-family="\"Geist Mono\""
        >
            <p>"Sphinx of black quartz, judge my vow."</p>
            <p style:font-weight="500">"Pack my box with five dozen jugs."</p>
            <p style:font-weight="600">"0123456789 +-=*/ &@#%?!"</p>
        </div>
    }
}

fn font_inter() -> ViewNode {
    view! {
        <div
            style:display="flex"
            style:flex-direction="column"
            style:gap="4px"
            style:font-size="14px"
            style:line-height="1.4285714"
            style:font-family="Inter"
        >
            <p>"Sphinx of black quartz, judge my vow."</p>
            <p style:font-weight="500">"Pack my box with five dozen jugs."</p>
            <p style:font-weight="600">"0123456789 +-=*/ &@#%?!"</p>
        </div>
    }
}

fn separator_horizontal() -> ViewNode {
    view! {
        <div style:width="256px">
            <Separator orientation=Orientation::Horizontal />
        </div>
    }
}

fn separator_vertical() -> ViewNode {
    view! {
        <div style:display="flex" style:height="32px">
            <Separator orientation=Orientation::Vertical />
        </div>
    }
}

// Buttons and badges sit in a `flex` row (`Row` in fixtures.tsx).

macro_rules! button_fixture {
    ($name:ident, $variant:ident, $size:ident, $disabled:expr, $text:literal) => {
        fn $name() -> ViewNode {
            view! {
                <div style:display="flex">
                    <Button
                        variant={ButtonVariant::$variant}
                        size={ButtonSize::$size}
                        disabled={$disabled}
                    >
                        $text
                    </Button>
                </div>
            }
        }
    };
}

button_fixture!(button_default, Default, Default, false, "Button");
button_fixture!(
    button_destructive,
    Destructive,
    Default,
    false,
    "Destructive"
);
button_fixture!(button_outline, Outline, Default, false, "Outline");
button_fixture!(button_secondary, Secondary, Default, false, "Secondary");
button_fixture!(button_ghost, Ghost, Default, false, "Ghost");
button_fixture!(button_link, Link, Default, false, "Link");
button_fixture!(button_size_xs, Default, Xs, false, "Extra small");
button_fixture!(button_size_sm, Default, Sm, false, "Small");
button_fixture!(button_size_lg, Default, Lg, false, "Large");
button_fixture!(button_disabled, Default, Default, true, "Disabled");

macro_rules! badge_fixture {
    ($name:ident, $variant:ident, $text:literal) => {
        fn $name() -> ViewNode {
            view! {
                <div style:display="flex">
                    <Badge variant={BadgeVariant::$variant}>$text</Badge>
                </div>
            }
        }
    };
}

badge_fixture!(badge_default, Default, "Badge");
badge_fixture!(badge_secondary, Secondary, "Secondary");
badge_fixture!(badge_destructive, Destructive, "Destructive");
badge_fixture!(badge_outline, Outline, "Outline");
badge_fixture!(badge_ghost, Ghost, "Ghost");
badge_fixture!(badge_link, Link, "Link");

/// `className="w-full max-w-sm"` on the example card.
const EXAMPLE_CARD: &[(&str, &str)] = &[("width", "100%"), ("max-width", "384px")];

fn card_basic() -> ViewNode {
    view! {
        <Card css={EXAMPLE_CARD}>
            <CardHeader>
                <CardTitle>"Card Title"</CardTitle>
                <CardDescription>"Card Description"</CardDescription>
            </CardHeader>
            <CardContent>
                <p>"Card Content"</p>
            </CardContent>
            <CardFooter>
                <p>"Card Footer"</p>
            </CardFooter>
        </Card>
    }
}

fn card_action() -> ViewNode {
    view! {
        <Card css={EXAMPLE_CARD}>
            <CardHeader>
                <CardTitle>"Card Title"</CardTitle>
                <CardDescription>"Card Description"</CardDescription>
                <CardAction>"Card Action"</CardAction>
            </CardHeader>
            <CardContent>
                <p>"Card Content"</p>
            </CardContent>
            <CardFooter>
                <p>"Card Footer"</p>
            </CardFooter>
        </Card>
    }
}

fn label_basic() -> ViewNode {
    view! {
        <Label>"Accept terms and conditions"</Label>
    }
}

/// Mounts a fixture's view; `view!` embeds child views through components.
#[allow(non_snake_case)]
#[component]
fn FixtureView(build: Fixture) -> ViewNode {
    build()
}

fn fixture_ids() -> Vec<String> {
    FIXTURES
        .iter()
        .flat_map(|(id, _, _)| THEMES.iter().map(move |(theme, _)| format!("{id}-{theme}")))
        .collect()
}

fn find_fixture(id: &str) -> Option<(Fixture, bool, Theme)> {
    FIXTURES.iter().find_map(|(base, fixture, text)| {
        THEMES.iter().find_map(|(name, theme)| {
            (id.strip_prefix(base)? == format!("-{name}")).then_some((*fixture, *text, *theme))
        })
    })
}

fn render(
    id: &str,
    output: &str,
    width: f64,
    height: f64,
    scale: f64,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = mount_fixture(id, width, height, scale)?;
    app.render_png_to(0.0, output)?;
    Ok(())
}

/// The border boxes of the fixture root and its descendant elements.
fn geometry(
    id: &str,
    width: f64,
    height: f64,
    scale: f64,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let app = mount_fixture(id, width, height, scale)?;
    app.document().update_all()?;
    let root = app
        .document()
        .body()
        .first_child()?
        .ok_or("the fixture root is missing")?;
    let mut boxes = Vec::new();
    let mut stack = vec![(root, 0usize)];
    while let Some((element, depth)) = stack.pop() {
        let rect = element.bounding_rect()?;
        boxes.push(serde_json::json!({
            "depth": depth,
            "slot": element.get_attribute("data-slot")?,
            "rect": rect.map(|r| [r.x, r.y, r.width, r.height]),
        }));
        let mut children = Vec::new();
        let mut child = element.first_child()?;
        while let Some(next) = child {
            child = next.next_sibling()?;
            children.push((next, depth + 1));
        }
        stack.extend(children.into_iter().rev());
    }
    Ok(serde_json::json!({ "fixture": id, "boxes": boxes }))
}

fn mount_fixture(
    id: &str,
    width: f64,
    height: f64,
    scale: f64,
) -> Result<HeadlessApp, Box<dyn std::error::Error>> {
    let (fixture, text, theme) = find_fixture(id).ok_or_else(|| format!("unknown fixture {id}"))?;
    let mut app = HeadlessApp::with_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        EngineOptions {
            raster_configuration: RasterConfiguration::chromium_linux_lcd(),
        },
    )?;
    if text {
        register_fonts(app.document())?;
    }
    // `body { @apply bg-background text-foreground }` from shadcn's base layer;
    // Tailwind's preflight removes the body margin.
    let body = app.document().body();
    body.set_background_color(theme.background)?;
    body.set_color(theme.foreground)?;
    let fixture_id = id.to_owned();
    // Preflight gives `html` the `--font-sans` family (Geist) and
    // `line-height: 1.5`; the page has no text outside `<main>`, so setting
    // them on `<main>` instead inherits the same values.
    app.mount(move || {
        with_theme(theme, || {
            view! {
                <div
                    data-fixture={fixture_id}
                    style:padding="16px"
                    style:font-family="Geist"
                    style:line-height="1.5"
                >
                    <FixtureView build=fixture />
                </div>
            }
        })
    })?;
    Ok(app)
}

/// `[--viewport WxH] [--scale S]`, defaulting to the 800x600 profile at 1x.
fn parse_surface(args: &[String]) -> Result<(f64, f64, f64), Box<dyn std::error::Error>> {
    let (mut width, mut height, mut scale) = (800.0, 600.0, 1.0);
    let mut rest = args.iter();
    while let Some(flag) = rest.next() {
        let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
        match flag.as_str() {
            "--viewport" => (width, height) = parse_viewport(value)?,
            "--scale" => scale = value.parse()?,
            other => return Err(format!("unknown option {other}").into()),
        }
    }
    Ok((width, height, scale))
}

fn parse_viewport(value: &str) -> Result<(f64, f64), Box<dyn std::error::Error>> {
    let (width, height) = value
        .split_once('x')
        .ok_or("--viewport expects WIDTHxHEIGHT")?;
    Ok((width.parse()?, height.parse()?))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("list") => {
            for id in fixture_ids() {
                println!("{id}");
            }
            Ok(())
        }
        Some("render") => {
            let id = args.get(1).ok_or("missing fixture id")?;
            let output = args.get(2).ok_or("missing output path")?;
            let (width, height, scale) = parse_surface(&args[3..])?;
            render(id, output, width, height, scale)
        }
        Some("geometry") => {
            let id = args.get(1).ok_or("missing fixture id")?;
            let (width, height, scale) = parse_surface(&args[2..])?;
            println!("{}", geometry(id, width, height, scale)?);
            Ok(())
        }
        _ => Err(
            "usage: openui-components-reference list | render <id> <out.png> \
                  [--viewport WxH] [--scale S] | geometry <id> [--viewport WxH] [--scale S]"
                .into(),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_fixture_id_resolves_to_its_theme() {
        let ids = fixture_ids();
        assert_eq!(ids.len(), FIXTURES.len() * THEMES.len());
        for id in &ids {
            let (_, _, theme) = find_fixture(id).expect(id);
            let expected = if id.ends_with("-dark") {
                Theme::DARK
            } else {
                Theme::LIGHT
            };
            assert_eq!(theme, expected);
        }
        assert!(find_fixture("separator-horizontal").is_none());
        assert!(find_fixture("separator-horizontal-dim").is_none());
    }

    #[test]
    fn fixture_ids_match_the_reference_manifest() {
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../../reference/pages/manifest.json")).unwrap();
        let mut reference: Vec<String> = manifest["fixtures"]
            .as_array()
            .unwrap()
            .iter()
            .map(|fixture| fixture["id"].as_str().unwrap().to_owned())
            .collect();
        let mut ours = fixture_ids();
        reference.sort();
        ours.sort();
        assert_eq!(ours, reference);
    }

    #[test]
    fn geometry_lists_the_fixture_tree_depth_first() {
        let value = geometry("card-action-light", 800.0, 600.0, 1.0).unwrap();
        let slots: Vec<(u64, Option<&str>)> = value["boxes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| (b["depth"].as_u64().unwrap(), b["slot"].as_str()))
            .collect();
        assert_eq!(
            slots,
            [
                (0, None),
                (1, Some("card")),
                (2, Some("card-header")),
                (3, Some("card-title")),
                (3, Some("card-description")),
                (3, Some("card-action")),
                (2, Some("card-content")),
                (3, None),
                (2, Some("card-footer")),
                (3, None),
            ]
        );
    }

    /// Preflight gives buttons `font: inherit` and `letter-spacing: inherit`;
    /// Open UI has no `inherit` literal, so its button must already inherit.
    #[test]
    fn buttons_inherit_the_page_font() {
        let app = mount_fixture("button-outline-dark", 800.0, 600.0, 1.0).unwrap();
        let row = app.document().body().first_child().unwrap().unwrap();
        let row = row.first_child().unwrap().unwrap();
        let button = row.first_child().unwrap().unwrap();
        assert_eq!(
            button.get_attribute("data-slot").unwrap().as_deref(),
            Some("button")
        );
        let (row, button) = (
            row.computed_style().unwrap(),
            button.computed_style().unwrap(),
        );
        assert_eq!(
            format!("{:?}", button.font_family()),
            format!("{:?}", row.font_family())
        );
        assert_eq!(button.letter_spacing(), row.letter_spacing());
        assert_eq!(
            format!("{:?}", button.font_style()),
            format!("{:?}", row.font_style())
        );
    }

    #[test]
    fn renders_every_fixture_at_the_requested_surface_size() {
        let directory = std::env::temp_dir().join("openui-components-reference-test");
        std::fs::create_dir_all(&directory).unwrap();
        for id in fixture_ids() {
            let path = directory.join(format!("{id}.png"));
            render(&id, path.to_str().unwrap(), 375.0, 667.0, 2.0).unwrap();
            let png = std::fs::read(&path).unwrap();
            // PNG IHDR: width and height are big-endian u32 at bytes 16..24.
            let width = u32::from_be_bytes(png[16..20].try_into().unwrap());
            let height = u32::from_be_bytes(png[20..24].try_into().unwrap());
            assert_eq!((width, height), (750, 1334), "{id}");
        }
    }
}
