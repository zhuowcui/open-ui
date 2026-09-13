//! Dashboard — multi-component layout with Show, For, and dynamic styles.
//!
//! Demonstrates component composition via `#[component]`, conditional
//! rendering with `Show`, list rendering with `For`, and manual layout
//! composition using the Element API with `mount_view`. The layout uses
//! a header, sidebar, and main content area arranged with flexbox.

use openui::prelude::*;

// ── Components ──────────────────────────────────────────────────────

/// Header bar spanning the full width of the viewport.
#[allow(non_snake_case)]
#[component]
fn Header(title: String) -> ViewNode {
    view! {
        <header
            style:display="flex"
            style:align-items="center"
            style:padding="0 24px"
            style:height="56px"
            style:background-color="#1a202c"
            style:color="white"
            style:font-family="sans-serif"
        >
            <h1 style:font-size="20px" style:font-weight="700" style:margin="0">
                {title}
            </h1>
        </header>
    }
}

/// A single metric card displaying a label and value.
#[allow(non_snake_case)]
#[component]
fn MetricCard(label: String, value: String, color: String) -> ViewNode {
    view! {
        <div
            style:background-color="white"
            style:border-radius="8px"
            style:padding="20px"
            style:min-width="160px"
            style:border="1px solid #e2e8f0"
        >
            <p style:font-size="13px" style:color="#718096"
               style:margin="0" style:margin-bottom="4px">
                {label}
            </p>
            <p style:font-size="28px" style:font-weight="700"
               style:margin="0">
                <span>{value}</span>
            </p>
        </div>
    }
}

/// A single navigation item in the sidebar.
#[allow(non_snake_case)]
#[component]
fn NavItem(label: String, active: i32) -> ViewNode {
    let bg = if active != 0 {
        Color::from_hex(0xedf2f7, false)
    } else {
        Color::TRANSPARENT
    };
    let fw = if active != 0 {
        FontWeight::SEMI_BOLD
    } else {
        FontWeight::NORMAL
    };
    view! {
        <div
            style:padding="10px 16px"
            style:border-radius="6px"
            style:margin-bottom="4px"
            style:cursor="pointer"
            style:font-size="14px"
            style:color="#2d3748"
            style:background-color={bg}
            style:font-weight={fw}
        >
            <span>{label}</span>
        </div>
    }
}

/// A row in the activity feed.
#[allow(non_snake_case)]
#[component]
fn ActivityRow(text: String, time: String) -> ViewNode {
    view! {
        <div
            style:display="flex"
            style:justify-content="space-between"
            style:padding="10px 0"
            style:border-bottom="1px solid #edf2f7"
            style:font-size="14px"
        >
            <span style:color="#2d3748">{text}</span>
            <span style:color="#a0aec0" style:font-size="12px">{time}</span>
        </div>
    }
}

fn application_view() -> ViewNode {
    let show_details = create_signal(true);
    let doc = current_document();

    // ── Root container ──────────────────────────────────────
    let root = Element::create(&doc, "div").expect("create root");
    root.set_property(StyleProperty::Display, Display::Flex.into())
        .expect("style");
    root.set_property(StyleProperty::FlexDirection, FlexDirection::Column.into())
        .expect("style");
    root.set_property(
        StyleProperty::MinHeight,
        LengthValue::ViewportHeight(100.0).into(),
    )
    .expect("style");
    root.set_property(
        StyleProperty::BackgroundColor,
        Color::from_hex(0xf7fafc, false).into(),
    )
    .expect("style");

    // ── Header ──────────────────────────────────────────────
    mount_view(
        &root,
        Header(HeaderProps {
            title: "Dashboard".to_string(),
        }),
    );

    // ── Body: sidebar + main ────────────────────────────────
    let body = Element::create(&doc, "div").expect("create body");
    body.set_property(StyleProperty::Display, Display::Flex.into())
        .expect("style");
    body.set_property(StyleProperty::FlexGrow, 1.0_f32.into())
        .expect("style");

    // Sidebar navigation
    let sidebar = Element::create(&doc, "nav").expect("create sidebar");
    sidebar
        .set_property(StyleProperty::Width, LengthValue::px(220.0).into())
        .expect("style");
    sidebar
        .set_property(StyleProperty::BackgroundColor, Color::WHITE.into())
        .expect("style");
    sidebar
        .set_property(
            StyleProperty::BorderRight,
            Border {
                width: 1.0,
                style: BorderStyle::Solid,
                color: Color::from_hex(0xe2e8f0, false),
            }
            .into(),
        )
        .expect("style");
    sidebar
        .set_property(
            StyleProperty::Padding,
            Edges {
                top: LengthValue::px(16.0),
                right: LengthValue::px(8.0),
                bottom: LengthValue::px(16.0),
                left: LengthValue::px(8.0),
            }
            .into(),
        )
        .expect("style");

    let nav_labels = ["Overview", "Analytics", "Projects", "Team", "Settings"];
    for (i, label) in nav_labels.iter().enumerate() {
        let active = if i == 0 { 1 } else { 0 };
        mount_view(
            &sidebar,
            NavItem(NavItemProps {
                label: label.to_string(),
                active,
            }),
        );
    }
    body.append_child(&sidebar).expect("append sidebar");

    // Main content area
    let main_el = Element::create(&doc, "main").expect("create main");
    main_el
        .set_property(StyleProperty::FlexGrow, 1.0_f32.into())
        .expect("style");
    main_el
        .set_property(
            StyleProperty::Padding,
            Edges::all(LengthValue::px(24.0)).into(),
        )
        .expect("style");

    // Section title
    mount_view(
        &main_el,
        view! {
            <h2 style:font-size="22px" style:color="#1a202c"
                style:margin="0" style:margin-bottom="20px"
                style:font-family="sans-serif">
                "Overview"
            </h2>
        },
    );

    // Metric cards row
    let metrics_row = Element::create(&doc, "div").expect("create metrics row");
    metrics_row
        .set_property(StyleProperty::Display, Display::Flex.into())
        .expect("style");
    metrics_row
        .set_property(
            StyleProperty::Gap,
            Gap {
                row: LengthValue::px(16.0),
                column: LengthValue::px(16.0),
            }
            .into(),
        )
        .expect("style");
    metrics_row
        .set_property(StyleProperty::MarginBottom, LengthValue::px(24.0).into())
        .expect("style");

    let metrics = [
        ("Users", "12,847", "#3182ce"),
        ("Revenue", "$48.2k", "#38a169"),
        ("Orders", "1,024", "#d69e2e"),
        ("Growth", "+14.2%", "#e53e3e"),
    ];
    for (label, value, color) in &metrics {
        mount_view(
            &metrics_row,
            MetricCard(MetricCardProps {
                label: label.to_string(),
                value: value.to_string(),
                color: color.to_string(),
            }),
        );
    }
    main_el.append_child(&metrics_row).expect("append metrics");

    // Activity feed section
    let activity_section = Element::create(&doc, "div").expect("create activity");
    activity_section
        .set_property(StyleProperty::BackgroundColor, Color::WHITE.into())
        .expect("style");
    activity_section
        .set_property(
            StyleProperty::BorderRadius,
            CornerRadii(Edges::all(LengthValue::px(8.0))).into(),
        )
        .expect("style");
    activity_section
        .set_property(
            StyleProperty::Padding,
            Edges::all(LengthValue::px(20.0)).into(),
        )
        .expect("style");
    activity_section
        .set_property(
            StyleProperty::Border,
            Border {
                width: 1.0,
                style: BorderStyle::Solid,
                color: Color::from_hex(0xe2e8f0, false),
            }
            .into(),
        )
        .expect("style");

    mount_view(
        &activity_section,
        view! {
            <h3 style:font-size="16px" style:color="#2d3748"
                style:margin="0" style:margin-bottom="12px"
                style:font-family="sans-serif">
                "Recent Activity"
            </h3>
        },
    );

    // Activity list rendered with For
    let activities = create_signal(vec![
        ("New user registered: alice@example.com", "2 min ago"),
        ("Order #1024 completed", "15 min ago"),
        ("Deployment to production succeeded", "1 hour ago"),
        ("Team meeting notes updated", "3 hours ago"),
        ("Monthly report generated", "5 hours ago"),
    ]);

    let activity_list = For(
        move || activities.get(),
        |item| item.0.to_string(),
        |item| {
            ActivityRow(ActivityRowProps {
                text: item.0.to_string(),
                time: item.1.to_string(),
            })
        },
    );
    mount_view(&activity_section, activity_list);
    main_el
        .append_child(&activity_section)
        .expect("append activity");

    // Conditional details panel rendered with Show
    let details_panel = Show(
        move || show_details.get(),
        || view! { <div /> },
        || {
            view! {
                <div
                    style:background-color="white"
                    style:border-radius="8px"
                    style:padding="20px"
                    style:border="1px solid #e2e8f0"
                    style:margin-top="20px"
                >
                    <h3 style:font-size="16px" style:color="#2d3748"
                        style:margin="0" style:margin-bottom="12px">
                        "System Status"
                    </h3>
                    <div style:display="flex" style:gap="24px">
                        <div>
                            <span style:color="#48bb78" style:font-size="14px">
                                "● API: Healthy"
                            </span>
                        </div>
                        <div>
                            <span style:color="#48bb78" style:font-size="14px">
                                "● Database: Connected"
                            </span>
                        </div>
                        <div>
                            <span style:color="#48bb78" style:font-size="14px">
                                "● Cache: 98% hit rate"
                            </span>
                        </div>
                    </div>
                </div>
            }
        },
    );
    mount_view(&main_el, details_panel);

    body.append_child(&main_el).expect("append main");

    root.append_child(&body).expect("append body");

    ViewNode::Element(root)
}

#[cfg(feature = "linux")]
fn main() -> Result<(), Error> {
    App::builder()
        .title("Open UI Dashboard")
        .size(LogicalSize::new(1200.0, 800.0))
        .build()?
        .run(application_view)
}

#[cfg(not(feature = "linux"))]
fn main() -> Result<(), Error> {
    let mut app = HeadlessApp::new(Viewport::new(1200, 800)?)?;
    app.mount(application_view)?;

    app.render_png_to(0.0, "dashboard.png")?;

    println!("Rendered dashboard.png");
    Ok(())
}
