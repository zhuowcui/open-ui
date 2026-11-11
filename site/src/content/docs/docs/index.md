---
title: Introduction
description: What Open UI is and how an app is put together.
---

Open UI is a framework for building native desktop apps in Rust. You
describe the interface with a `view!` macro that reads like HTML, style it
with CSS properties, and ship one native binary. There is no webview, no
JavaScript engine and no HTML at runtime: Open UI lays out and paints your
views itself, with Skia.

:::note[Alpha]
Open UI is an experimental alpha for Windows and Linux. APIs change without
notice and nothing is on crates.io yet. macOS comes next.
:::

## What an app looks like

```rust
use openui::prelude::*;
use openui::window::WindowSpec;

fn main() -> Result<(), Error> {
    App::new("com.example.counter")
        .window(WindowSpec::new("main").title("Counter"), counter)
        .run()
}

fn counter() -> ViewNode {
    let count = create_signal(0);
    view! {
        <button
            style:padding="8px 16px"
            style:border-radius="8px"
            on:click={move |_| count.set(count.get() + 1)}
        >
            {format!("Clicked {} times", count.get())}
        </button>
    }
}
```

- **Views** are Rust functions that return `view!` markup. Components are
  functions with `#[component]`.
- **State** lives in signals. When a signal changes, the views that read it
  update.
- **Styles** are CSS properties and values, checked when the app compiles.
- **App services** (windows, menus, dialogs, clipboard, HTTP and async tasks)
  are part of the framework.

## Next steps

- [Getting started](/open-ui/docs/getting-started/): install the CLI and
  run your first app.
- [Views and styles](/open-ui/docs/views-and-styles/): markup, components
  and CSS.
- [Testing](/open-ui/docs/testing/): headless tests and snapshots.
- [Components](/open-ui/components/): the shadcn/ui components that ship with
  Open UI.

The API reference is not published yet; `cargo doc --open` in your app
builds it locally.
