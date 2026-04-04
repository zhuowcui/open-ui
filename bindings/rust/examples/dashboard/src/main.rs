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
    let bg = if active != 0 { "#edf2f7" } else { "transparent" };
    let fw = if active != 0 { "600" } else { "400" };
    view! {
        <div
            style:padding="10px 16px"
            style:border-radius="6px"
            style:margin-bottom="4px"
            style:cursor="pointer"
            style:font-size="14px"
            style:color="#2d3748"
            style:background-color={move || bg}
            style:font-weight={move || fw}
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

fn main() {
    let mut app = App::new(1200, 800);

    app.render(|| {
        let show_details = create_signal(true);
        let doc = current_document();

        // ── Root container ──────────────────────────────────────
        let root = Element::create(doc, "div").expect("create root");
        root.set_style("display", "flex").expect("style");
        root.set_style("flex-direction", "column").expect("style");
        root.set_style("min-height", "100vh").expect("style");
        root.set_style("background-color", "#f7fafc").expect("style");

        // ── Header ──────────────────────────────────────────────
        mount_view(
            &root,
            Header(HeaderProps {
                title: "Dashboard".to_string(),
            }),
        );

        // ── Body: sidebar + main ────────────────────────────────
        let body = Element::create(doc, "div").expect("create body");
        body.set_style("display", "flex").expect("style");
        body.set_style("flex", "1").expect("style");

        // Sidebar navigation
        let sidebar = Element::create(doc, "nav").expect("create sidebar");
        sidebar.set_style("width", "220px").expect("style");
        sidebar.set_style("background-color", "white").expect("style");
        sidebar
            .set_style("border-right", "1px solid #e2e8f0")
            .expect("style");
        sidebar.set_style("padding", "16px 8px").expect("style");

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
        body.append_child(&sidebar);
        std::mem::forget(sidebar);

        // Main content area
        let main_el = Element::create(doc, "main").expect("create main");
        main_el.set_style("flex", "1").expect("style");
        main_el.set_style("padding", "24px").expect("style");

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
        let metrics_row = Element::create(doc, "div").expect("create metrics row");
        metrics_row.set_style("display", "flex").expect("style");
        metrics_row.set_style("gap", "16px").expect("style");
        metrics_row
            .set_style("margin-bottom", "24px")
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
        main_el.append_child(&metrics_row);
        std::mem::forget(metrics_row);

        // Activity feed section
        let activity_section = Element::create(doc, "div").expect("create activity");
        activity_section
            .set_style("background-color", "white")
            .expect("style");
        activity_section
            .set_style("border-radius", "8px")
            .expect("style");
        activity_section.set_style("padding", "20px").expect("style");
        activity_section
            .set_style("border", "1px solid #e2e8f0")
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
        main_el.append_child(&activity_section);
        std::mem::forget(activity_section);

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

        body.append_child(&main_el);
        std::mem::forget(main_el);

        root.append_child(&body);
        std::mem::forget(body);

        ViewNode::Element(root)
    });

    app.run_frames(1).render_to_png("dashboard.png");

    println!("Rendered dashboard.png");
}
