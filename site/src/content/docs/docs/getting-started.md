---
title: Getting started
description: Install the openui CLI, create an app, run it, change it and build it.
---

This guide takes you from nothing to a running app and a release build in
five steps. It works on Windows and Linux.

The packages and the CLI are still named `openui` while the rename to Div
lands.

## Before you start

You need [Rust](https://rustup.rs) 1.85 or later, Git, Python 3, Ninja and
clang. On Windows you also need the Visual Studio C++ build tools ("Desktop
development with C++"). The CLI checks all of this for you in the next step.

## 1. Install the CLI

```sh
cargo install --locked --git https://github.com/zhuowcui/open-ui openui-cli
openui doctor
```

`openui doctor` lists each tool it needs and how to install any that are
missing.

## 2. Create an app

```sh
openui new my-app
cd my-app
```

```text
my-app/
├── Cargo.toml
├── assets/          # files shipped next to your binary
├── src/
│   ├── main.rs      # the app, its menu and its main window
│   ├── lib.rs       # app state
│   ├── app.rs       # the root view
│   └── menu.rs      # the app menu
└── tests/
    └── ui.rs        # a headless UI test
```

## 3. Run it

```sh
openui dev
```

A window opens. `openui dev` rebuilds and restarts the app each time you
save. The first build takes a few minutes, because it compiles the Skia
graphics library; later builds are quick.

## 4. Change it

Open `src/app.rs`. The root view is plain Rust: markup in `view!`, styles as
`style:` attributes, and signals from the app state for the click count and
a status line.

```rust
use crate::State;
use openui::prelude::*;

pub fn root(state: State) -> ViewNode {
    let count = state.count;
    view! {
        <div
            style:display="flex"
            style:flex-direction="column"
            style:align-items="center"
            style:justify-content="center"
            style:gap="16px"
            style:min-height="100vh"
            style:font-family="sans-serif"
        >
            <h1 style:font-size="24px">"my-app"</h1>
            <button
                style:padding="8px 20px"
                style:font-size="16px"
                on:click={move |_| count.set(count.get() + 1)}
            >
                "Count"
            </button>
            <p>{match count.get() {
                1 => "Clicked 1 time".to_owned(),
                clicks => format!("Clicked {clicks} times"),
            }}</p>
            <p style:color="#555555">{state.status.get()}</p>
        </div>
    }
}
```

Change the heading or a color and save; the app restarts with your change.
A misspelled property or an invalid value is a compile error, not a silent
no-op.

## 5. Build it

```sh
openui build
```

You get a release binary with your `assets/` beside it in
`dist/my-app-<version>-<os>-<arch>/`. It is one native executable: no
browser runtime and no bundled webview.

## Next

- [Views and `view!`](/docs/concepts/views/) explains markup and
  components, and [Elements](/docs/concepts/elements/) the elements
  every view is made of.
- The [Elements reference](/elements/) covers each element, such as
  the [button](/elements/button/) and [input](/elements/input/)
  in this app.
- [Testing](/docs/services/testing/) covers the tests in
  `tests/ui.rs`.
