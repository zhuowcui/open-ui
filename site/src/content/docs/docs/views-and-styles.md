---
title: Views and styles
description: Markup with view!, components, signals and CSS styles.
---

## Markup

`view!` takes HTML-like markup: elements, text in quotes, and Rust
expressions in braces.

```rust
view! {
    <section style:padding="24px">
        <h2>"Inbox"</h2>
        <p>{format!("{} unread", unread.get())}</p>
    </section>
}
```

An expression that reads a signal updates when the signal changes. Events
are `on:` attributes that take a closure, such as
`on:click={move |_| count.set(count.get() + 1)}`.

## Components

A component is a function marked `#[component]`. Its arguments are its
props, and `children` receives the markup between its tags.

```rust
#[component]
fn Stat(label: String, value: String) -> ViewNode {
    view! {
        <div style:padding="16px" style:border-radius="12px">
            <div style:color="#71717a">{label}</div>
            <div style:font-size="28px" style:font-weight="600">{value}</div>
        </div>
    }
}

view! { <Stat label={"Uptime".to_owned()} value={"99.98%".to_owned()} /> }
```

## Styles

Styles are CSS properties and values on `style:` attributes. They are parsed
when your app compiles, so a typo is an error you see right away. The value
can also be a Rust expression, such as a color that depends on state.

```rust
let color = if failed { Color::from_hex(0xdc2626, false) } else { Color::BLACK };
view! { <span style:color={color} style:font-weight="500">"Build"</span> }
```

Layout and paint follow the web: block and inline flow, flexbox, grid,
positioning, borders, backgrounds, rounded corners and web fonts. Open UI lays these out the way a browser
does; see [Rendering](/open-ui/docs/rendering/).

Class-based stylesheets, `:hover`-style state variants and theme tokens are
in progress for the next alpha.

## Components library

Open UI ships shadcn/ui's components rebuilt in Rust: button, badge, card,
label and separator so far. See [Components](/open-ui/components/).
