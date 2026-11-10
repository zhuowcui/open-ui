//! Render the components reference fixtures with Open UI.
//!
//!     openui-components-reference list
//!     openui-components-reference render <fixture-id> <out.png> [--viewport WxH] [--scale S]
//!
//! Each fixture ID matches a page built by `components/reference/build.mjs`
//! from the pinned shadcn/ui sources. A page is `<body>` with shadcn's
//! `bg-background text-foreground`, holding `<main class="p-4">` with the
//! fixture inside; this binary builds the same tree with the public `openui`
//! crate and renders it with Chromium's Linux LCD raster configuration.

use openui::prelude::*;
use openui_components::{with_theme, Orientation, Separator, SeparatorProps, Theme};

type Fixture = fn() -> ViewNode;

const THEMES: [(&str, Theme); 2] = [("light", Theme::LIGHT), ("dark", Theme::DARK)];

/// Fixtures in the order of `components/reference/fixtures.tsx`.
const FIXTURES: &[(&str, Fixture)] = &[
    ("separator-horizontal", separator_horizontal),
    ("separator-vertical", separator_vertical),
];

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

/// Mounts a fixture's view; `view!` embeds child views through components.
#[allow(non_snake_case)]
#[component]
fn FixtureView(build: Fixture) -> ViewNode {
    build()
}

fn fixture_ids() -> Vec<String> {
    FIXTURES
        .iter()
        .flat_map(|(id, _)| THEMES.iter().map(move |(theme, _)| format!("{id}-{theme}")))
        .collect()
}

fn find_fixture(id: &str) -> Option<(Fixture, Theme)> {
    FIXTURES.iter().find_map(|(base, fixture)| {
        THEMES.iter().find_map(|(name, theme)| {
            (id.strip_prefix(base)? == format!("-{name}")).then_some((*fixture, *theme))
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
    let (fixture, theme) = find_fixture(id).ok_or_else(|| format!("unknown fixture {id}"))?;
    let mut app = HeadlessApp::with_options(
        ViewportMetrics::from_logical_size(width, height, scale)?,
        EngineOptions {
            raster_configuration: RasterConfiguration::chromium_linux_lcd(),
        },
    )?;
    // `body { @apply bg-background text-foreground }` from shadcn's base layer;
    // Tailwind's preflight removes the body margin.
    let body = app.document().body();
    body.set_background_color(theme.background)?;
    body.set_color(theme.foreground)?;
    let fixture_id = id.to_owned();
    app.mount(move || {
        with_theme(theme, || {
            view! {
                <div data-fixture={fixture_id} style:padding="16px">
                    <FixtureView build=fixture />
                </div>
            }
        })
    })?;
    app.render_png_to(0.0, output)?;
    Ok(())
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
            let (mut width, mut height, mut scale) = (800.0, 600.0, 1.0);
            let mut rest = args[3..].iter();
            while let Some(flag) = rest.next() {
                let value = rest.next().ok_or_else(|| format!("{flag} needs a value"))?;
                match flag.as_str() {
                    "--viewport" => (width, height) = parse_viewport(value)?,
                    "--scale" => scale = value.parse()?,
                    other => return Err(format!("unknown option {other}").into()),
                }
            }
            render(id, output, width, height, scale)
        }
        _ => Err(
            "usage: openui-components-reference list | render <id> <out.png> \
                  [--viewport WxH] [--scale S]"
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
            let (_, theme) = find_fixture(id).expect(id);
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
