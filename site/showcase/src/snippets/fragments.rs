//! The short code fragments written inline in the docs pages, each in a
//! function so that it compiles against the checked-out `openui`. When a
//! page's fragment changes, change it here too. The page each comes from is
//! named above it.
#![allow(non_snake_case, unused_variables)]

use openui::prelude::*;

// docs/concepts/views.mdx
#[component]
fn Stat(label: String, value: String) -> ViewNode {
    view! {
        <div style:padding="12px 16px" style:border="1px solid #e4e4e7">
            <div style:color="#71717a">{label}</div>
            <div style:font-size="24px">{value}</div>
        </div>
    }
}

fn views_component_calls() -> Vec<ViewNode> {
    vec![
        view! { <Stat label={"Uptime".to_string()} value={"99.98%".to_string()} /> },
        Stat(StatProps {
            label: "Uptime".into(),
            value: "99.98%".into(),
        }),
    ]
}

fn views_for(todos: Signal<Vec<String>>) -> ViewNode {
    let list = view! { <ul></ul> };
    if let ViewNode::Element(ul) = &list {
        mount_view(
            ul,
            For(
                move || todos.get(),
                |todo| todo.clone(),
                |todo| view! { <li>{todo}</li> },
            ),
        );
    }
    list
}

fn views_conditional_text(done: Signal<bool>) -> ViewNode {
    view! { <span>{if done.get() { "Done" } else { "Pending" }}</span> }
}

// docs/concepts/elements.mdx
#[component]
fn Slot(child: ViewNode) -> ViewNode {
    child
}

fn elements_handle() -> ViewNode {
    let field = view! { <input placeholder="Search" /> };
    if let ViewNode::Element(input) = &field {
        input.set_accessibility_label("Search").unwrap();
    }
    view! { <div><Slot child={field} /></div> }
}

fn elements_role(heading: &Element) {
    heading
        .set_accessibility_role(AccessibilityRole::Heading)
        .unwrap();
}

// docs/concepts/styling.mdx
fn styling_literal() -> ViewNode {
    view! { <div style:padding="12px 16px" style:border="1px solid #e4e4e7" /> }
}

fn styling_dynamic(failed: Signal<bool>) -> ViewNode {
    let tone = move || {
        if failed.get() {
            Color::from_hex(0xb91c1c, false)
        } else {
            Color::from_hex(0x15803d, false)
        }
    };
    view! { <span style:color={tone()}>"…"</span> }
}

fn styling_setters(element: &Element) {
    element.set_width(LengthValue::px(200.0)).unwrap();
    element.set_background_color(Color::WHITE).unwrap();
}

fn styling_hover() -> ViewNode {
    let hovered = create_signal(false);
    view! {
        <button
            style:background-color={if hovered.get() { Color::from_hex(0xf4f4f5, false) } else { Color::WHITE }}
            on:mouseenter={move |_| hovered.set(true)}
            on:mouseleave={move |_| hovered.set(false)}
        >
            "Hover me"
        </button>
    }
}

// docs/concepts/text-and-fonts.mdx
fn text_format(done: Signal<u32>, total: Signal<u32>) -> ViewNode {
    view! { <p>{format!("{} of {} done", done.get(), total.get())}</p> }
}

// docs/concepts/events-and-state.mdx
fn events_signals(meter: Element, progress: Signal<f64>) {
    let count = create_signal(0);
    count.get();
    count.set(5);
    count.update(|n| *n += 1);
    let items = create_signal(vec![1, 2, 3]);
    let total = move || items.get().len();
    let doubled = create_memo(move || count.get() * 2);
    create_effect(move || {
        meter
            .set_attribute("value", &format!("{:.2}", progress.get()))
            .unwrap();
    });
    batch(|| {
        count.set(1);
        items.set(vec![]);
    });
}

fn events_input(draft: Signal<String>) -> ViewNode {
    view! {
        <input on:input={move |event: &Event| {
            let input = event.target().expect("input");
            draft.set(input.control_value().ok().flatten().unwrap_or_default());
        }} />
    }
}

// elements/details.mdx
fn details_change(expanded: Signal<bool>) -> ViewNode {
    view! {
        <details on:change={move |event: &Event| {
            let details = event.target().expect("details");
            expanded.set(details.is_open().unwrap_or(false));
        }}>
            <summary>{if expanded.get() { "▾ " } else { "▸ " }} "More"</summary>
        </details>
    }
}

// elements/meter-progress.mdx
fn meter_update(element: &Element, fraction: f64) -> Result<(), Error> {
    element.set_attribute("value", &format!("{:.2}", fraction))?;
    Ok(())
}

// reference/selectors.mdx
fn pseudo_placeholder(input: &Element) {
    let style = Style::default().color(Color::from_hex(0x71717a, false));
    input
        .set_pseudo_style(PseudoStyleTarget::Placeholder, &style)
        .unwrap();
}
