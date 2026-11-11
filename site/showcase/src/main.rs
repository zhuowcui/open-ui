//! Render the website's screenshots with Open UI.
//!
//!     openui-site-showcase <out-dir> [scene...]
//!
//! Every image on the site that shows an app is drawn here, headless, by the
//! public `openui` crate and the shadcn/ui buttons and badges in
//! `components/openui-components`. Nothing is retouched: `render.ps1`
//! rebuilds this binary and writes the PNGs to `site/src/assets/showcase/`,
//! which the site commits and shows as they are.
//!
//! The scenes stack content in normal block flow and use flex only for
//! rows. Open UI's column flex boxes still size text items as if at their
//! min-content width, so `Card` (a column flex box) is not used here yet.

#![allow(unused_variables)] // `#[component]` functions without props.

mod example;

use openui::prelude::*;
use openui_components::{
    register_fonts, with_theme, Badge, BadgeProps, BadgeVariant, Button, ButtonProps, ButtonSize,
    ButtonVariant, Label, LabelProps, Orientation, Separator, SeparatorProps, Theme,
};

/// One image: a name, a viewport in CSS pixels and the view to draw.
struct Scene {
    name: &'static str,
    width: f64,
    height: f64,
    themes: &'static [&'static str],
    /// Draw text as the components harness does (Chromium's Linux LCD
    /// profile) instead of with grayscale antialiasing.
    harness_raster: bool,
    build: fn() -> ViewNode,
}

const SCALE: f64 = 2.0;
const BOTH: &[&str] = &["light", "dark"];

const SCENES: &[Scene] = &[
    Scene {
        name: "app",
        width: 1180.0,
        height: 720.0,
        themes: BOTH,
        harness_raster: false,
        build: app,
    },
    // The example's colors are written in its code, so it has one theme.
    Scene {
        name: "example",
        width: 440.0,
        height: 220.0,
        themes: &["light"],
        harness_raster: false,
        build: example,
    },
    // The same card for the comparison with Chromium (capture.py).
    Scene {
        name: "example-openui",
        width: 440.0,
        height: 220.0,
        themes: &["light"],
        harness_raster: true,
        build: example,
    },
    Scene {
        name: "buttons",
        width: 400.0,
        height: 340.0,
        themes: BOTH,
        harness_raster: false,
        build: buttons,
    },
    Scene {
        name: "badges",
        width: 400.0,
        height: 340.0,
        themes: BOTH,
        harness_raster: false,
        build: badges,
    },
    Scene {
        name: "notifications",
        width: 400.0,
        height: 340.0,
        themes: BOTH,
        harness_raster: false,
        build: notifications,
    },
    Scene {
        name: "team",
        width: 400.0,
        height: 340.0,
        themes: BOTH,
        harness_raster: false,
        build: team_tile,
    },
    Scene {
        name: "stat",
        width: 400.0,
        height: 340.0,
        themes: BOTH,
        harness_raster: false,
        build: stat_tile,
    },
    Scene {
        name: "settings",
        width: 400.0,
        height: 340.0,
        themes: BOTH,
        harness_raster: false,
        build: settings,
    },
];

fn theme() -> Theme {
    openui_components::current_theme()
}

// ── Building blocks ─────────────────────────────────────────────────────

/// A bordered, rounded surface: shadcn's card look, in block flow.
#[allow(non_snake_case)]
#[component]
fn Panel(children: Vec<ViewNode>) -> ViewNode {
    let t = theme();
    let panel = view! {
        <div
            style:border-radius="14px"
            style:border-top-width="1"
            style:border-right-width="1"
            style:border-bottom-width="1"
            style:border-left-width="1"
            style:border-top-style="solid"
            style:border-right-style="solid"
            style:border-bottom-style="solid"
            style:border-left-style="solid"
            style:border-top-color={t.border.into()}
            style:border-right-color={t.border.into()}
            style:border-bottom-color={t.border.into()}
            style:border-left-color={t.border.into()}
            style:background-color={t.card}
            style:color={t.card_foreground}
        ></div>
    };
    if let ViewNode::Element(element) = &panel {
        for child in children {
            mount_view(element, child);
        }
    }
    panel
}

/// A panel heading: a title, a description and an optional action.
#[allow(non_snake_case)]
#[component]
fn Heading(title: String, description: String, action: Option<ViewNode>) -> ViewNode {
    let t = theme();
    let action = action.unwrap_or_else(|| view! { <span></span> });
    view! {
        <div style:display="flex" style:justify-content="space-between" style:align-items="flex-start" style:gap="12px">
            <div>
                <div style:font-size="16px" style:line-height="24px" style:font-weight="600">{title}</div>
                <div style:font-size="14px" style:line-height="20px" style:color={t.muted_foreground}>{description}</div>
            </div>
            <Slot child={action} />
        </div>
    }
}

/// Mounts a prebuilt view inside `view!`.
#[allow(non_snake_case)]
#[component]
fn Slot(child: ViewNode) -> ViewNode {
    child
}

#[allow(non_snake_case)]
#[component]
fn Avatar(initials: String) -> ViewNode {
    let t = theme();
    view! {
        <div
            style:width="32px"
            style:height="32px"
            style:flex-shrink="0"
            style:border-radius="16px"
            style:background-color={t.muted}
            style:color={t.foreground}
            style:display="flex"
            style:align-items="center"
            style:justify-content="center"
            style:font-size="12px"
            style:font-weight="500"
        >
            {initials}
        </div>
    }
}

fn button(variant: ButtonVariant, size: ButtonSize, text: &'static str) -> ViewNode {
    view! { <Button variant={variant} size={size} disabled={false}>{text}</Button> }
}

// ── The app window on the home page ────────────────────────────────────

fn app() -> ViewNode {
    let t = theme();
    view! {
        <div
            style:display="flex"
            style:height="720px"
            style:overflow="hidden"
            style:background-color={t.background}
            style:color={t.foreground}
        >
            <Sidebar />
            <div style:flex-grow="1" style:flex-basis="0px" style:min-width="0">
                <TopBar />
                <div style:padding="24px">
                    <div style:display="flex" style:align-items="flex-end" style:justify-content="space-between">
                        <div>
                            <div style:font-size="24px" style:line-height="32px" style:font-weight="600">"Overview"</div>
                            <div style:font-size="14px" style:line-height="20px" style:color={t.muted_foreground}>
                                "Everything shipped across your projects this week."
                            </div>
                        </div>
                        <div style:display="flex" style:gap="2px" style:padding="3px" style:border-radius="10px" style:background-color={t.muted}>
                            <Segment label={"7 days".to_owned()} active={true} />
                            <Segment label={"30 days".to_owned()} active={false} />
                            <Segment label={"90 days".to_owned()} active={false} />
                        </div>
                    </div>
                    <div style:display="flex" style:gap="16px" style:margin-top="20px">
                        <Stat label={"Deployments".to_owned()} value={"148".to_owned()} note={"+12% from last week".to_owned()} bar={0.72} />
                        <Stat label={"Build time".to_owned()} value={"1m 42s".to_owned()} note={"18s faster than last week".to_owned()} bar={0.46} />
                        <Stat label={"Uptime".to_owned()} value={"99.98%".to_owned()} note={"No incidents".to_owned()} bar={0.98} />
                    </div>
                    <div style:display="flex" style:gap="16px" style:margin-top="16px" style:align-items="flex-start">
                        <div style:flex-grow="1" style:flex-basis="0px" style:min-width="0">
                            <Deployments />
                        </div>
                        <div style:width="296px" style:flex-shrink="0">
                            <Team />
                        </div>
                    </div>
                </div>
            </div>
        </div>
    }
}

#[allow(non_snake_case)]
#[component]
fn Sidebar() -> ViewNode {
    let t = theme();
    view! {
        <div
            style:position="relative"
            style:width="232px"
            style:flex-shrink="0"
            style:padding="12px"
            style:box-sizing="border-box"
            style:background-color={t.sidebar}
            style:color={t.sidebar_foreground}
            style:border-right-width="1"
            style:border-right-style="solid"
            style:border-right-color={t.sidebar_border.into()}
        >
            <div style:display="flex" style:align-items="center" style:gap="10px" style:padding="6px 8px 18px 8px">
                <div
                    style:width="32px"
                    style:height="32px"
                    style:border-radius="8px"
                    style:background-color={t.sidebar_primary}
                    style:color={t.sidebar_primary_foreground}
                    style:display="flex"
                    style:align-items="center"
                    style:justify-content="center"
                    style:font-size="14px"
                    style:font-weight="600"
                >
                    "A"
                </div>
                <div>
                    <div style:font-size="14px" style:line-height="20px" style:font-weight="600">"Acme Inc."</div>
                    <div style:font-size="12px" style:line-height="16px" style:color={t.muted_foreground}>"Team plan"</div>
                </div>
            </div>
            <GroupLabel text={"Platform".to_owned()} />
            <NavItem label={"Overview".to_owned()} count={"".to_owned()} active={true} />
            <NavItem label={"Projects".to_owned()} count={"12".to_owned()} active={false} />
            <NavItem label={"Deployments".to_owned()} count={"".to_owned()} active={false} />
            <NavItem label={"Analytics".to_owned()} count={"".to_owned()} active={false} />
            <div style:height="16px"></div>
            <GroupLabel text={"Workspace".to_owned()} />
            <NavItem label={"Team".to_owned()} count={"6".to_owned()} active={false} />
            <NavItem label={"Billing".to_owned()} count={"".to_owned()} active={false} />
            <NavItem label={"Settings".to_owned()} count={"".to_owned()} active={false} />
            <div
                style:position="absolute"
                style:left="12px"
                style:right="12px"
                style:bottom="12px"
                style:display="flex"
                style:align-items="center"
                style:gap="10px"
                style:padding="8px"
            >
                <Avatar initials={"JL".to_owned()} />
                <div style:min-width="0">
                    <div style:font-size="14px" style:line-height="20px" style:font-weight="500">"Jordan Lee"</div>
                    <div style:font-size="12px" style:line-height="16px" style:color={t.muted_foreground}>"jordan@acme.dev"</div>
                </div>
            </div>
        </div>
    }
}

#[allow(non_snake_case)]
#[component]
fn GroupLabel(text: String) -> ViewNode {
    let t = theme();
    view! {
        <div
            style:padding="4px 8px"
            style:font-size="12px"
            style:line-height="16px"
            style:font-weight="500"
            style:color={t.muted_foreground}
        >
            {text}
        </div>
    }
}

#[allow(non_snake_case)]
#[component]
fn NavItem(label: String, count: String, active: bool) -> ViewNode {
    let t = theme();
    let background = if active {
        t.sidebar_accent
    } else {
        Color::TRANSPARENT
    };
    let color = if active {
        t.sidebar_accent_foreground
    } else {
        t.sidebar_foreground
    };
    let weight = if active {
        FontWeight::MEDIUM
    } else {
        FontWeight::NORMAL
    };
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:justify-content="space-between"
            style:height="32px"
            style:margin-top="2px"
            style:padding="0 8px"
            style:border-radius="6px"
            style:font-size="14px"
            style:font-weight={weight}
            style:background-color={background}
            style:color={color}
        >
            <span>{label}</span>
            <span style:font-size="12px" style:color={t.muted_foreground}>{count}</span>
        </div>
    }
}

#[allow(non_snake_case)]
#[component]
fn TopBar() -> ViewNode {
    let t = theme();
    view! {
        <div
            style:height="56px"
            style:box-sizing="border-box"
            style:display="flex"
            style:align-items="center"
            style:justify-content="space-between"
            style:padding="0 24px"
            style:border-bottom-width="1"
            style:border-bottom-style="solid"
            style:border-bottom-color={t.border.into()}
        >
            <div style:display="flex" style:align-items="center" style:gap="8px" style:font-size="14px">
                <span style:color={t.muted_foreground}>"Acme Inc."</span>
                <span style:color={t.muted_foreground}>"/"</span>
                <span style:font-weight="500">"Overview"</span>
            </div>
            <div style:display="flex" style:align-items="center" style:gap="8px">
                <Slot child={button(ButtonVariant::Outline, ButtonSize::Sm, "Export")} />
                <Slot child={button(ButtonVariant::Default, ButtonSize::Sm, "New project")} />
            </div>
        </div>
    }
}

#[allow(non_snake_case)]
#[component]
fn Segment(label: String, active: bool) -> ViewNode {
    let t = theme();
    let background = if active {
        t.background
    } else {
        Color::TRANSPARENT
    };
    let color = if active {
        t.foreground
    } else {
        t.muted_foreground
    };
    view! {
        <div
            style:padding="4px 12px"
            style:border-radius="7px"
            style:font-size="13px"
            style:line-height="20px"
            style:font-weight="500"
            style:background-color={background}
            style:color={color}
        >
            {label}
        </div>
    }
}

/// A figure with a label, a bar filled to `bar` (0 to 1) and a note.
#[allow(non_snake_case)]
#[component]
fn Stat(label: String, value: String, note: String, bar: f64) -> ViewNode {
    let t = theme();
    let fill = LengthValue::percent((bar * 100.0) as f32);
    view! {
        <div style:flex-grow="1" style:flex-basis="0px" style:min-width="0">
            <Panel><div style:padding="20px">
                <div style:font-size="14px" style:line-height="20px" style:color={t.muted_foreground}>{label}</div>
                <div style:font-size="28px" style:line-height="36px" style:font-weight="600" style:margin-top="4px">{value}</div>
                <div style:height="6px" style:border-radius="3px" style:background-color={t.muted} style:margin-top="12px">
                    <div style:width={fill} style:height="6px" style:border-radius="3px" style:background-color={t.primary}></div>
                </div>
                <div style:font-size="12px" style:line-height="16px" style:color={t.muted_foreground} style:margin-top="10px">{note}</div>
            </div></Panel>
        </div>
    }
}

#[allow(non_snake_case)]
#[component]
fn Deployments() -> ViewNode {
    view! {
        <Panel><div style:padding="20px 24px 8px 24px">
            <Heading
                title={"Recent deployments".to_owned()}
                description={"The last five builds across every project.".to_owned()}
                action={Some(button(ButtonVariant::Ghost, ButtonSize::Sm, "View all"))}
            />
            <div style:margin-top="12px">
                <DeployRow project={"marketing-site".to_owned()} branch={"main".to_owned()} status={0} time={"2m ago".to_owned()} />
                <DeployRow project={"billing-api".to_owned()} branch={"fix/invoice-rounding".to_owned()} status={1} time={"6m ago".to_owned()} />
                <DeployRow project={"desktop-app".to_owned()} branch={"release/0.4".to_owned()} status={0} time={"24m ago".to_owned()} />
                <DeployRow project={"docs".to_owned()} branch={"main".to_owned()} status={2} time={"1h ago".to_owned()} />
                <DeployRow project={"auth-service".to_owned()} branch={"feat/passkeys".to_owned()} status={3} time={"3h ago".to_owned()} />
            </div>
        </div></Panel>
    }
}

/// `status`: 0 ready, 1 building, 2 failed, 3 queued.
#[allow(non_snake_case)]
#[component]
fn DeployRow(project: String, branch: String, status: i32, time: String) -> ViewNode {
    let t = theme();
    let badge = match status {
        0 => view! { <Badge variant={BadgeVariant::Default}>"Ready"</Badge> },
        1 => view! { <Badge variant={BadgeVariant::Secondary}>"Building"</Badge> },
        2 => view! { <Badge variant={BadgeVariant::Destructive}>"Failed"</Badge> },
        _ => view! { <Badge variant={BadgeVariant::Outline}>"Queued"</Badge> },
    };
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:gap="12px"
            style:padding="11px 0"
            style:border-top-width="1"
            style:border-top-style="solid"
            style:border-top-color={t.border.into()}
        >
            <div style:flex-grow="1" style:flex-basis="0px" style:min-width="0">
                <div style:font-size="14px" style:line-height="20px" style:font-weight="500">{project}</div>
                <div style:font-size="12px" style:line-height="16px" style:font-family="\"Geist Mono\"" style:color={t.muted_foreground}>
                    {branch}
                </div>
            </div>
            <div style:width="80px" style:display="flex"><Slot child={badge} /></div>
            <div style:width="56px" style:text-align="right" style:font-size="12px" style:color={t.muted_foreground}>{time}</div>
        </div>
    }
}

#[allow(non_snake_case)]
#[component]
fn Team() -> ViewNode {
    view! {
        <Panel><div style:padding="20px 24px 24px 24px">
            <Heading title={"Team".to_owned()} description={"Six people in this workspace.".to_owned()} action={None} />
            <div style:margin-top="16px">
                <Member initials={"JL".to_owned()} name={"Jordan Lee".to_owned()} role={"Owner".to_owned()} />
                <Member initials={"SR".to_owned()} name={"Sam Rivera".to_owned()} role={"Admin".to_owned()} />
                <Member initials={"AK".to_owned()} name={"Aiko Kato".to_owned()} role={"Member".to_owned()} />
                <Member initials={"MO".to_owned()} name={"Mara Okafor".to_owned()} role={"Member".to_owned()} />
            </div>
            <div style:margin-top="18px" style:display="flex">
                <Slot child={button(ButtonVariant::Outline, ButtonSize::Sm, "Invite people")} />
            </div>
        </div></Panel>
    }
}

#[allow(non_snake_case)]
#[component]
fn Member(initials: String, name: String, role: String) -> ViewNode {
    let t = theme();
    view! {
        <div style:display="flex" style:align-items="center" style:gap="10px" style:margin-top="12px">
            <Avatar initials={initials} />
            <div style:flex-grow="1" style:flex-basis="0px" style:min-width="0" style:font-size="14px" style:line-height="20px" style:font-weight="500">
                {name}
            </div>
            <div style:font-size="12px" style:color={t.muted_foreground}>{role}</div>
        </div>
    }
}

// ── The code example ───────────────────────────────────────────────────

fn example() -> ViewNode {
    centered(example::deploy_card())
}

// ── Component tiles ────────────────────────────────────────────────────

fn centered(child: ViewNode) -> ViewNode {
    let t = theme();
    view! {
        <div
            style:display="flex"
            style:align-items="center"
            style:justify-content="center"
            style:height="100vh"
            style:overflow="hidden"
            style:background-color={t.background}
            style:color={t.foreground}
        >
            <Slot child={child} />
        </div>
    }
}

fn buttons() -> ViewNode {
    centered(view! {
        <div>
            <div style:display="flex" style:gap="8px" style:justify-content="center">
                <Slot child={button(ButtonVariant::Default, ButtonSize::Default, "Button")} />
                <Slot child={button(ButtonVariant::Secondary, ButtonSize::Default, "Secondary")} />
                <Slot child={button(ButtonVariant::Outline, ButtonSize::Default, "Outline")} />
            </div>
            <div style:display="flex" style:gap="8px" style:justify-content="center" style:margin-top="12px">
                <Slot child={button(ButtonVariant::Ghost, ButtonSize::Default, "Ghost")} />
                <Slot child={button(ButtonVariant::Destructive, ButtonSize::Default, "Delete")} />
                <Slot child={button(ButtonVariant::Link, ButtonSize::Default, "Link")} />
            </div>
        </div>
    })
}

fn badges() -> ViewNode {
    centered(view! {
        <div>
            <div style:display="flex" style:gap="8px" style:justify-content="center">
                <Badge variant={BadgeVariant::Default}>"Badge"</Badge>
                <Badge variant={BadgeVariant::Secondary}>"Secondary"</Badge>
                <Badge variant={BadgeVariant::Outline}>"Outline"</Badge>
            </div>
            <div style:display="flex" style:gap="8px" style:justify-content="center" style:margin-top="12px">
                <Badge variant={BadgeVariant::Destructive}>"Destructive"</Badge>
                <Badge variant={BadgeVariant::Ghost}>"Ghost"</Badge>
                <Badge variant={BadgeVariant::Link}>"Link"</Badge>
            </div>
        </div>
    })
}

fn notifications() -> ViewNode {
    centered(view! {
        <div style:width="320px">
            <Panel><div style:padding="24px">
                <Heading title={"Notifications".to_owned()} description={"You have 3 unread messages.".to_owned()} action={None} />
                <Note title={"Build finished".to_owned()} time={"1 minute ago".to_owned()} />
                <Note title={"New comment on #42".to_owned()} time={"1 hour ago".to_owned()} />
                <div style:margin-top="20px" style:display="flex">
                    <Slot child={button(ButtonVariant::Default, ButtonSize::Default, "Mark all as read")} />
                </div>
            </div></Panel>
        </div>
    })
}

#[allow(non_snake_case)]
#[component]
fn Note(title: String, time: String) -> ViewNode {
    let t = theme();
    view! {
        <div style:display="flex" style:gap="12px" style:align-items="flex-start" style:margin-top="16px">
            <div style:width="8px" style:height="8px" style:flex-shrink="0" style:border-radius="4px" style:margin-top="6px" style:background-color={t.primary}></div>
            <div>
                <div style:font-size="14px" style:line-height="20px" style:font-weight="500">{title}</div>
                <div style:font-size="13px" style:line-height="20px" style:color={t.muted_foreground}>{time}</div>
            </div>
        </div>
    }
}

fn team_tile() -> ViewNode {
    centered(view! {
        <div style:width="320px">
            <Team />
        </div>
    })
}

fn stat_tile() -> ViewNode {
    centered(view! {
        <div style:width="300px" style:display="flex">
            <Stat label={"Monthly revenue".to_owned()} value={"$24,580".to_owned()} note={"+8.2% from last month".to_owned()} bar={0.64} />
        </div>
    })
}

fn settings() -> ViewNode {
    let t = theme();
    centered(view! {
        <div style:width="320px">
            <Label>"Open UI"</Label>
            <div style:font-size="14px" style:line-height="20px" style:margin-top="4px" style:color={t.muted_foreground}>
                "A native UI framework for Rust."
            </div>
            <div style:margin="16px 0">
                <Separator orientation={Orientation::Horizontal} />
            </div>
            <div style:display="flex" style:align-items="center" style:gap="12px" style:height="20px" style:font-size="14px">
                <span>"Docs"</span>
                <Separator orientation={Orientation::Vertical} />
                <span>"Source"</span>
                <Separator orientation={Orientation::Vertical} />
                <span>"Blog"</span>
            </div>
        </div>
    })
}

// ── Rendering ──────────────────────────────────────────────────────────

/// Chromium's Linux raster profile with grayscale text antialiasing:
/// subpixel (LCD) text shows color fringes once a browser scales the image.
fn raster() -> RasterConfiguration {
    let mut raster = RasterConfiguration::chromium_linux_lcd();
    for text in [
        &mut raster.author_text,
        &mut raster.native_text,
        &mut raster.embedded_text,
    ] {
        text.edging = TextEdging::AntiAlias;
    }
    raster
}

fn render(
    scene: &Scene,
    theme_name: &str,
    out: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let theme = if theme_name == "dark" {
        Theme::DARK
    } else {
        Theme::LIGHT
    };
    let mut app = HeadlessApp::with_options(
        ViewportMetrics::from_logical_size(scene.width, scene.height, SCALE)?,
        EngineOptions {
            raster_configuration: if scene.harness_raster {
                RasterConfiguration::chromium_linux_lcd()
            } else {
                raster()
            },
        },
    )?;
    register_fonts(app.document())?;
    let body = app.document().body();
    body.set_background_color(theme.background)?;
    body.set_color(theme.foreground)?;
    let build = scene.build;
    app.mount(move || {
        with_theme(theme, || {
            view! {
                <div style:font-family="Geist" style:line-height="1.5" style:font-size="14px">
                    <Slot child={build()} />
                </div>
            }
        })
    })?;
    let path = out.join(format!("{}-{theme_name}.png", scene.name));
    app.render_png_to(0.0, path.to_str().ok_or("non-UTF-8 path")?)?;
    println!("{}", path.display());
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out = std::path::PathBuf::from(
        args.first()
            .ok_or("usage: openui-site-showcase <out-dir> [scene...]")?,
    );
    std::fs::create_dir_all(&out)?;
    let only = &args[1..];
    for scene in SCENES {
        if !only.is_empty() && !only.iter().any(|name| name == scene.name) {
            continue;
        }
        for theme_name in scene.themes {
            render(scene, theme_name, &out)?;
        }
    }
    Ok(())
}
