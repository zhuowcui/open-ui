---
title: Getting started
description: Install the openui CLI, create an app from the default template and run it.
---

:::caution[Experimental alpha]
Open UI is pre-release software. The API changes without notice, nothing is
published to crates.io yet, and alpha.1 targets **Windows and Linux** only.
macOS is planned for alpha.2.
:::

Everything on this page comes from the `openui` CLI and the `default`
template on `main`
([`bindings/rust/openui-cli`](https://github.com/zhuowcui/open-ui/tree/main/bindings/rust/openui-cli)).

## 1. Check your tools

The first build compiles Skia from source, with the same build flags as the
project's CI, so that your app draws the same pixels the measurements use.
That needs a C++ toolchain:

| Tool | Why |
| --- | --- |
| Rust 1.85 or later | [rustup.rs](https://rustup.rs) |
| Git | the framework is fetched from git |
| Python 3 | Skia's build scripts |
| Ninja | Skia builds with it |
| clang and libclang | Skia and its Rust bindings |
| Visual Studio C++ build tools (Windows only) | "Desktop development with C++" |

## 2. Install the CLI

Until crates are published under a final name, the CLI installs from git:

```sh
cargo install --locked --git https://github.com/zhuowcui/open-ui openui-cli
```

Then let it check the toolchain:

```sh
openui doctor
```

`doctor` prints `ok` or `missing` with a fix for each tool, and fails if
anything required is missing.

## 3. Create an app

```sh
openui new my-app
cd my-app
```

`new` uses the `default` template unless you pass `--template minimal`. It
writes a Cargo project, a `.cargo/config.toml` that carries the Skia build
profile (so plain `cargo build` produces the same Skia as `openui build`),
and a `Cargo.lock` seeded from the framework's reviewed lockfile. The
`default` template gives you:

```text
my-app/
├── Cargo.toml        # openui with the desktop, menu, dialog and http features
├── assets/           # copied next to the binary by `openui build`
├── src/
│   ├── main.rs       # the App, its menu and its main window
│   ├── lib.rs        # shared State (signals)
│   ├── app.rs        # the root view
│   └── menu.rs       # File, Edit, View, Help; a file dialog and an HTTP check
└── tests/
    └── ui.rs         # a headless click test and a pixel snapshot test
```

## 4. Run it

```sh
openui dev
```

`dev` builds and runs the app, then rebuilds and restarts it when you save.
It is a restart, not live reload; live reload of styles and markup is planned
for alpha.2. The first build compiles Skia and takes a while.

## Your first window

`src/main.rs` opens one window and installs the menu:

```rust
use openui::prelude::*;
use openui::window::WindowSpec;
use my_app::{app, menu, State};

fn main() -> Result<(), Error> {
    // App state lives for the whole run and is shared by every window.
    let state = State::new();
    // Replace the id with your own reverse-DNS name.
    App::new("com.example.my_app")
        .name("my-app")
        .menu(move || menu::app_menu(state))
        .window(
            WindowSpec::new("main").title("my-app").size(720.0, 480.0),
            move || app::root(state),
        )
        .run()
}
```

`src/app.rs` is the view. Markup is the `view!` macro; styles are `style:`
attributes in CSS syntax, and every property and value is checked when the
app compiles:

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

There are no stylesheets or class selectors yet. Typed `style!`, `css!` and
`stylesheet!` macros, state variants such as `:hover`, and theme tokens are
the next steps of
[RFC 0001](https://github.com/zhuowcui/open-ui/blob/main/docs/alpha/rfcs/0001-typed-style-rules.md).

## Test it headless

`tests/ui.rs` mounts the view without a window, clicks the button through hit
testing and compares the first frame with a PNG snapshot, byte for byte. A
new project has no snapshot yet, so record it once, look at it and commit it:

```sh
OPENUI_UPDATE_SNAPSHOTS=1 cargo test
cargo test
```

After that, any change in the frame fails the test and writes
`tests/snapshots/<name>.<os>.actual.png` beside the snapshot.

## Ship it

```sh
openui build
```

`build` makes a release binary and copies it, with `assets/`, into
`dist/<name>-<version>-<os>-<arch>/`. It builds for the host only: Windows
x64 or Linux x64. There are no installers, signing or auto-update in alpha.
