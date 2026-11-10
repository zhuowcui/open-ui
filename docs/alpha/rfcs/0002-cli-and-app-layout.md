# RFC 0002: CLI, app layout and async model

| | |
| --- | --- |
| Status | Proposed, 9 November 2026 |
| Track | Framework (Workstream B, units B1 and B3) |
| Owns | `openui`, `openui-platform`, new `openui-cli` |
| Affects | Components (templates), release engineering (lockfile, Skia profile) |
| Requires from engine | Nothing for alpha.1 |

Names are working names. `openui`, `openui-cli` and the `openui` binary are
renamed mechanically once D5 picks the product name.

## Summary

Apps are ordinary Cargo binaries. A CLI, `openui`, creates them (`new`),
rebuilds and restarts them on save (`dev`) and produces release binaries
(`build`). Everything an app declares (windows, menus, services) is typed Rust
in `main.rs`; there is no separate app manifest beyond a small
`[package.metadata.openui]` table. All UI state lives on one UI thread. Async
work runs as UI-thread tasks that may touch signals, or as background tasks
that may not, with a typed bridge between them. App services are thin wrappers
over existing crates, each behind a Cargo feature.

## What exists today

| Fact | Source |
| --- | --- |
| One window per process; `App::builder().title().size().build()?.run(view)` | `openui/src/app.rs:70–260` |
| winit 0.30.13 event loop; private `UserEvent` with one AccessKit variant; no public wake | `openui-platform/src/desktop.rs:26–34, 857, 1144` |
| Runtime ignores every window but the first | `desktop.rs:669, 906–911` |
| Reactive runtime is `thread_local!`; `Document` is `Rc` | `openui/src/runtime.rs:65`, `document.rs:151` |
| `Signal<T>` is `Send` when `T` is, but panics when used off the UI thread | `openui/src/signal.rs:12–15, 39` |
| No futures, executor or resource primitive in any first-party crate | survey, 9 November 2026 |
| Text clipboard on Wayland, X11 and Win32, private to the runtime | `desktop.rs:42–49`, `windows_clipboard.rs` |
| No menus, dialogs, tray or HTTP | survey |
| Headless rendering to PNG without a window | `HeadlessApp`, `app.rs:534–603` |
| Fidelity needs a specific Skia build: `FORCE_SKIA_BUILD=1` and `SKIA_GN_ARGS` gamma flags | `.github/workflows/ci.yml:60–78` |
| Workspace `[patch.crates-io]` pins skia-safe to a git rev and vendors skrifa, read-fonts, font-types, bytemuck and zbus_xml | `bindings/rust/Cargo.toml` |
| Dependency gates: MSRV 1.85, no pre-release versions, licenses Apache-2.0/MIT/BSD/ISC/Unicode/FTL, cargo-audit with warnings denied and yanked versions rejected; any `Cargo.lock` change regenerates `dependency-policy.json` | `tools/release/generate_v02_contract.py:172–188`, `check_dependency_security.py` |

The last three rows matter most for the CLI: a downstream app does not inherit
the workspace's patches or Skia flags, so `openui new` must write them.

## 1. CLI

New crate `bindings/rust/openui-cli`, binary `openui`. It shells out to
`cargo`; it never reimplements building. Until crates are published under the
final name it installs with
`cargo install --locked --git https://github.com/zhuowcui/open-ui openui-cli`.

| Command | Does | alpha.1 scope |
| --- | --- | --- |
| `openui new <name> [--template minimal\|default] [--framework-path <dir>]` | Writes a new Cargo project from an embedded template | `minimal` in the skeleton PR, `default` later; `shadcn` is added by the components track |
| `openui dev [--release] [--bin <name>] [-- <app args>]` | Builds, runs, watches, rebuilds and restarts on save | Restart only; live reload is alpha.2 |
| `openui build [--debug] [--out <dir>]` | Release build plus a `dist/` folder | Host target only (Windows x64, Linux x64) |
| `openui doctor` | Checks the toolchain Skia needs (clang/LLVM, ninja, Python, MSVC on Windows) and prints fixes | Same PR as the skeleton |

### `new`

- Templates live in `openui-cli/templates/<name>/` and are embedded at compile
  time (`include_str!` through a generated file list). Placeholders are plain
  `{{name}}`, `{{crate_name}}`, `{{app_id}}` and `{{framework_dependency}}`;
  no template engine dependency.
- The framework dependency is a git dependency pinned to the CLI's release tag
  (or `main` before the first tag). `--framework-path` writes path
  dependencies instead; CI and contributors use it.
- Because `[patch.crates-io]` does not propagate to dependents, `new` writes
  the same patch table into the app's `Cargo.toml`, pointing at the same git
  revision (Cargo finds `vendor/*` packages inside the git source) or at the
  local path. A CLI test fails if the template's table drifts from the
  workspace's.
- `new` seeds the app's `Cargo.lock` from the framework's reviewed lockfile,
  so a new app starts on the audited dependency versions; Cargo prunes the
  unused entries on first build.
- `new` writes `.cargo/config.toml` with an `[env]` table carrying the Skia
  build profile (`FORCE_SKIA_BUILD`, `SKIA_GN_ARGS`, `SKIA_NINJA_COMMAND`,
  `RUST_MIN_STACK`). Plain `cargo build` then produces the same Skia as CI,
  with or without the CLI. A CLI test parses `ci.yml` and fails on drift.
  Until release engineering publishes prebuilt Skia binaries for this profile,
  the first build compiles Skia from source; `doctor` and the docs say so and
  the skeleton PR measures how long it takes. Prebuilt archives must be keyed
  by the GN args, compiler and opt-level as well as rust-skia's own key, which
  omits them; otherwise an app could install a Skia that renders differently.

### `dev`

1. Run `cargo build --message-format=json-render-diagnostics` and take the
   executable path from the `compiler-artifact` message. Diagnostics stream
   through unchanged.
2. Start the app with `OPENUI_DEV=1`, `OPENUI_DEV_STATE=<target>/openui-dev/state.json`
   and a piped stdin.
3. Watch `src/`, `assets/`, `build.rs`, `Cargo.toml` and any
   `[package.metadata.openui] watch` paths by polling modification times
   every 250 ms, settling for 150 ms after the last change, ignoring `target/`
   and editor temporaries. An app's source tree is small enough that polling
   costs little and needs no dependency. The CLI's dependencies (clap 4.6,
   serde_json, toml) are all already locked, so it adds no new crates.
4. On change, rebuild. If the build fails, keep the running app and print the
   errors. If it succeeds, stop the app and start the new binary. Changes
   during a build queue exactly one further build.
5. Stopping is graceful first: closing stdin makes a dev-mode app exit through
   its normal path. After 2 s the CLI terminates the process.

In dev mode the runtime saves each window's size, position (where the platform
allows it; Wayland does not) and maximized state to the state file on exit, and
restores it by window key on start, so a restart lands where the developer
left it. (Not in the multi-window PR; it follows as its own
small PR.)

Build speed comes from keeping the app crate at `opt-level = 0` for fast
incremental builds while the framework crates run optimized
(`[profile.dev.package."*"] opt-level = 2`). One exception: on Linux,
rust-skia passes Cargo's opt-level to the Skia compiler, so Skia's optimization
level is part of the rendering configuration. The template pins
`skia-bindings` to the same opt-level in dev and release, and the skeleton PR
checks that a headless render of the template is byte-identical between
`dev` and `build`. The same PR measures save-to-new-frame time on both
platforms and records it; no target is promised before that measurement.

### `build`

`cargo build --release` (with `--locked` when `--locked` is passed; `openui new`
seeds the lockfile but apps may update it), then copy the executable and `assets/` to `dist/<package>-<version>-<os>-<arch>/`. Templates
add `#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` so
release builds open no console on Windows. Cross-compilation, icons, version
resources, installers and signing are not in alpha.1.

### App metadata

```toml
[package.metadata.openui]
assets = "assets"   # copied by `build`, watched by `dev`
watch = []          # extra paths for `dev`
```

The table holds build facts only. The app's identity is Rust code:
`App::new("dev.example.notes").name("Notes")`, where `new` fills in the
reverse-DNS id (`--app-id`, default `dev.example.<crate_name>`). The runtime
uses it for the dev state file and, later, portals and single-instance.

## 2. Project layout

```
my-app/
  Cargo.toml          # dependency, patch table, metadata, dev profile
  Cargo.lock          # seeded from the framework's reviewed lockfile
  .cargo/config.toml  # Skia build profile
  .gitignore
  src/main.rs         # App declaration: windows, menus
  src/app.rs          # root component            (default template)
  src/menu.rs         # menu declaration          (default template)
  assets/             # fonts, images             (default template)
  tests/ui.rs         # headless snapshot test    (default template)
  tests/snapshots/    # accepted PNGs, per platform (default template)
```

The `openui` dependency uses one feature, `desktop`, which selects the host's
platform runtime (Win32 on Windows, X11 and Wayland on Linux) through
target-specific dependencies, plus one feature per service (section 3). Apps
no longer need the examples' `linux`/`windows` feature pair or a second
headless `main`.

## 3. Declaring an app

### Windows

```rust
use openui::prelude::*;

fn main() -> openui::Result<()> {
    App::new("dev.example.notes")
        .name("Notes")                              // default window title
        .menu(menu::app_menu)                       // default menu bar for every window
        .window(
            WindowSpec::new("main").title("Notes").size(960.0, 640.0),
            || view! { <Notes/> },
        )
        .run()
}
```

- `WindowSpec` is typed: key, title, size, minimum and maximum size,
  resizable, decorations and an optional per-window menu.
- Windows opened at run time: `window::open(spec, view) -> WindowHandle`.
  Opening a key that is already open focuses that window, which covers
  singleton windows such as settings; `WindowKey::unique()` makes a fresh key
  for document windows. `WindowHandle` has `close`, `focus`, `set_title` and
  `is_open`; `window::current()` returns the window whose event is being
  handled, and `window::all()` lists the open windows.
- Every window has its own `Document` and root reactive scope, disposed when
  the window closes. All windows share the UI thread's reactive runtime, so a
  signal created in `main` or in one window drives views in any window with no
  `Send` bound and no message passing.
- `WindowSpec::on_close_requested(|| Close::Allow | Close::Prevent)` lets an
  app veto a close, for example to ask about unsaved changes, and close later
  through its handle.
- The app exits after its last window closes unless
  `App::exit_on_last_window_closed(false)` is set; `app::exit()` exits from
  anywhere on the UI thread.
- The existing `App::builder()...run(view)` stays and becomes a single-window
  shorthand over the new runtime, so examples, tests and the C ABI keep
  working unchanged (D4).

`openui-platform` changes from one `PlatformApplication` per process to an
application object owning a map of window key to window state, all on one winit
event loop. This is internal to the framework track.

### Menus

```rust
pub fn app_menu() -> Menu {
    Menu::new()
        .submenu("File", |m| m
            .item(MenuItem::new("New Window")
                .accelerator(Accelerator::new(Modifiers::PRIMARY, Key::N))
                .on_select(|| { window::open(WindowSpec::new(WindowKey::unique()), || view! { <Notes/> }); }))
            .separator()
            .role(Role::Quit))
        .submenu("Edit", |m| m.role(Role::Undo).role(Role::Redo).separator()
            .role(Role::Cut).role(Role::Copy).role(Role::Paste).role(Role::SelectAll))
}
```

- Menus are typed values. `enabled` and `checked` accept a value or a closure
  read reactively, so an item follows a signal without manual updates.
- `Modifiers::PRIMARY` is Ctrl on Windows and Linux and Cmd on macOS, so the
  same declaration serves alpha.2.
- Roles map to the platform's standard item and behavior (Quit, Undo, Redo,
  Cut, Copy, Paste, Select All, Close Window, About).
- Accelerators are matched by the runtime before the focused document sees the
  key, as native menus do, and work whether or not the menu is open.
- `on_select` runs on the UI thread with the active window available through
  `window::current()`.

Platform implementation:

| Platform | Menu bar |
| --- | --- |
| Windows | Native `HMENU` attached to each winit window by muda; accelerators through winit's message hook |
| Linux | Drawn by the framework inside the window from the same declaration: a `position: fixed` bar with a spacer before the app's root, so the page lays out below it and scrolls under it; dropdowns positioned under their top item; a full-window layer under them that closes them on an outside click; keyboard navigation (F10, Alt with a mnemonic, arrows, Enter, Space, Escape, item mnemonics) and menu roles in the accessibility tree |
| macOS (alpha.2) | Native app menu from the same declaration |

Linux has no native menu bar that works with a winit window: muda's Linux
backend attaches only to a GTK window, and its `gtk` 0.18 tree fails our
cargo-audit gate (glib RUSTSEC-2024-0429, proc-macro-error
RUSTSEC-2024-0370; gtk 0.19 needs Rust 1.92). GNOME has no global menu. A KDE
global menu over `com.canonical.dbusmenu`, using the zbus 5 already locked, is
a later option. If the engine's accessibility mapping lacks menu roles, the
menu PR asks for them through an RFC.

The menu arrives in two PRs. The first adds the typed model, declared with
`AppSpec::menu(build)` and rebuilt reactively per window, the
accelerators and roles on every platform, and the native Windows menu bar.
The second draws the Linux menu bar. Function keys are
`Key::Function(n)`, so that `Key::F` stays the letter. The engine already
accepts AccessKit's menu roles (`Element::set_accessibility_role`), so no
engine RFC is needed.

### Services

Each service is a module of `openui` behind its own feature. The default
template enables all alpha.1 services; headless consumers enable none, so they
gain no new dependencies (ADR 004).

| Service | Module (feature) | API sketch | Built on | Notes |
| --- | --- | --- | --- | --- |
| Windows | `window` (`desktop`) | `WindowSpec`, `window::open`, `WindowHandle` | winit 0.30.13, already locked | |
| Menus | `menu` (`menu`) | `Menu`, `MenuItem`, `Role`, `Accelerator` | Windows: muda `=0.19.3`, `default-features = false` (Apache-2.0 OR MIT, MSRV 1.73, reuses windows-sys 0.61). Linux: framework-drawn | Newer muda needs Rust 1.90 |
| Clipboard | `clipboard` (`desktop`) | `clipboard::read_text()`, `clipboard::write_text(text)` | The runtime's existing smithay-clipboard, x11-clipboard and Win32 code, made public | Text only in alpha.1; the document's own clipboard stays in sync |
| File dialogs | `dialog` (`dialog`) | `dialog::open_file().filter("Text", &["txt", "md"]).pick().await -> Result<Option<PathBuf>, Error>`; `pick_many`, `save_file()`, `pick_folder()` | Windows: `IFileDialog` through the `windows` 0.62.2 crate already locked by AccessKit. Linux: the XDG desktop portal's `FileChooser` over the zbus 5.13 already locked, with a `zenity` fallback. No new crates | Dialogs run on a background thread, owned by the current window (X11 and Win32; Wayland parents need an exported surface, so they open unparented). Errors are returned rather than read as a cancel. rfd 0.16.1 was planned, but it adds about 50 crates, including ICU 2.4, which needs Rust 1.88 |
| Async tasks | `task` (`desktop`) | section 4 | async-task 4.7.1, async-executor 1.14.1, async-io 2.6.0, blocking 1.7.0, all already locked (Apache-2.0/MIT) | No new crates |
| HTTP | `http` (`http`) | `http::get(url).send().await?.text()`, `http::Client` with timeouts and headers; JSON with `serde` | ureq `=3.5.1` with rustls on the ring provider and rustls-platform-verifier, run on the blocking pool | System root certificates; no OpenSSL, no aws-lc (which needs NASM on Windows) and no webpki-roots (CDLA-Permissive-2.0) |

Versions are the newest whose declared `rust-version` is at most 1.85, pinned
with `=` because the workspace resolver does not choose by MSRV.

Services that wait on the user or the network are async (section 4), so a
dialog never blocks the UI thread.

## 4. Async model

UI state never leaves the UI thread; work that does not touch UI state may.

| | `spawn_local(fut)` | `spawn(fut)` |
| --- | --- | --- |
| Runs on | UI thread | Background executor threads |
| Bound | `'static` (no `Send`) | `Send + 'static` |
| May touch signals, `Document`, `Element` | Yes | No (compile error, see below) |
| Use for | Awaiting dialogs, HTTP and timers; updating state | CPU work and long computations; `spawn_blocking(f)` for blocking calls |
| Returns | `Task<T>`; dropping cancels, `.detach()` keeps running | `Task<T>`, awaitable from either side |

- **UI executor.** `async_task::spawn_local` with a schedule function that
  pushes the runnable onto a queue and posts one coalesced `Wake` user event
  through winit's `EventLoopProxy`. The loop runs queued runnables for a
  bounded turn, then returns to input and rendering: no polling, no busy wait.
  This is the only option that accepts `!Send` futures with a waker driven by a
  foreign event loop; Tokio's `LocalSet` and `async-executor`'s
  `LocalExecutor` both need to own the loop through `block_on` or `tick`.
- **Background work.** `spawn` runs on a small `async-executor` pool and
  `spawn_blocking` on `blocking`'s pool. IO and timers come from `async-io`,
  whose reactor has its own thread, so its futures work in either kind of task
  with no runtime context. All of these are already locked through zbus.
- **Tokio compatibility.** An optional `tokio` feature starts a Tokio
  multi-thread runtime on first use and enters its context while the UI
  executor polls, so Tokio-based crates (reqwest, sqlx) also work inside
  `spawn_local`, and `spawn` then uses Tokio. Templates do not enable it.
- **Timers.** `time::sleep` and `time::interval`, from `async-io`.
- **Scopes cancel tasks.** A task spawned inside a component's reactive scope
  is cancelled when the scope is disposed (the component unmounts or its
  window closes), so a late response never writes to a dead view.
- **Signals become `!Send`.** `Signal`, `Memo` and the other reactive handles
  gain a `!Send` marker, turning today's run-time panic into a compile error.
  `signal.sender()` returns a `Send` handle that posts the new value to the UI
  thread, for the rare background task that must push a result without being
  awaited.
- **Resources.** `create_resource(source, fetch)` is the small bridge most
  apps need: it re-runs `fetch` when `source` changes, cancels the previous
  run, and exposes `loading()`, `get() -> Option<Result<T, E>>` and
  `refetch()` as reactive reads. Suspense and error boundaries remain B2.
- **Never block the UI thread.** There is no `block_on` on the UI thread. For
  tests, `HeadlessApp::run_until_idle()` runs the UI executor until it and the
  background tasks it awaits are idle.

## 5. Test utilities

`openui::testing` (feature `testing`, normally a dev-dependency feature)
wraps `HeadlessApp`:

- `TestApp::new(size).mount(view)`, lookups (`by_id`, `by_text`), input
  helpers (`click(&element)`, `type_text`, `press(key)`) built on the existing
  `Document` dispatch methods, and `run_until_idle()`.
- `assert_snapshot!(app, "name")` renders to PNG and compares exact RGBA with
  `tests/snapshots/name.<os>.png`. Snapshots are per platform because system
  fonts differ. `OPENUI_UPDATE_SNAPSHOTS=1` writes or replaces them; a missing
  snapshot fails unless that variable is set, so CI never accepts a new image
  silently.

## 6. Delivery

One PR each, merged in order; each has tests and, for services, a runnable
example built and smoke-tested on Windows and Linux CI.

| # | PR | Verification |
| --- | --- | --- |
| 1 | This RFC | Review |
| 2 | `openui-cli`: `new` (`minimal`), `dev`, `build`, `doctor`; `desktop` feature; the runtime's dev-mode exit on stdin close, through a public `EventLoopProxy` wake | Unit tests for templating, patch and Skia-profile drift; an integration test that runs `new --framework-path` then `cargo check` on both CI platforms; measured first build and save-to-frame times |
| 3 | Multi-window runtime and `WindowSpec` | Platform tests with two windows; existing examples unchanged |
| 4 | Async: UI executor, background runtime, `!Send` signals, `create_resource` | Headless tests for wakeups, cancellation on scope disposal, compile-fail test for a signal in `spawn` |
| 5 | Clipboard service over the existing native clipboard | Platform tests; example |
| 6 | File dialogs | Example; headless builds unaffected |
| 7 | Menus: native on Windows, drawn on Linux | Accelerator and role tests; example |
| 8 | HTTP client | Tests against a local server; example |
| 9 | `default` template and `openui::testing` | Template integration test runs its own snapshot test |

Every PR that changes `Cargo.lock` regenerates
`docs/v02/generated/dependency-policy.json` after rebasing, never by merging
the file by hand, because other tracks change the lockfile too.

## 7. Impact on other tracks

- **Engine:** none required. If a window-level feature later needs engine
  support (for example per-window top layer for menus), it comes as its own
  RFC.
- **Components:** the `shadcn` template lives at
  `openui-cli/templates/shadcn/`; the components track owns its content and
  the framework track reviews it. Components use only the public `openui`
  API.
- **Release engineering:** prebuilt Skia binaries for the template's Skia
  profile would cut first-build time; publishing must eventually replace the
  git `[patch.crates-io]` table, which crates.io does not allow.

## Alternatives considered

- **A separate `openui.toml` manifest for windows and menus.** Rejected: it
  would be parsed at run time and checked later than Rust code, against the
  typed direction of D1. `[package.metadata.openui]` holds only build facts.
- **Tokio as the default runtime.** Rejected for alpha.1: Tokio cannot run a
  `!Send` executor under winit's loop, so the UI executor is `async-task`
  either way, and the smol crates already locked cover IO, timers and blocking
  work with no new dependencies. The `tokio` feature keeps the Tokio ecosystem
  reachable.
- **reqwest for HTTP.** Its default TLS is rustls on aws-lc-rs, which needs a
  C compiler and NASM on Windows, and it requires Tokio. ureq on rustls and
  ring needs no NASM, OpenSSL or Tokio. reqwest works with the `tokio`
  feature.
- **muda with GTK on Linux.** Rejected: it needs a GTK window and main loop,
  and its gtk 0.18 tree fails the cargo-audit gate.
- **arboard for the clipboard.** Unneeded: the runtime already has a working
  text clipboard on all three window systems, and arboard's Wayland support
  depends on the data-control protocol, which not every compositor offers.
- **notify for the `dev` watcher.** Deferred: the newest notify within MSRV
  1.85 (`=8.2.1`, with notify-debouncer-full `=0.7.0`) is CC0-1.0, outside
  the listed licenses, and polling is enough for an app's source tree. If
  large projects need it, adopting notify needs a license review first.
- **`cargo-generate` for templates.** Rejected: heavy dependency tree for what
  is a handful of files with four placeholders.
- **Hot reload of Rust code (subsecond-style patching).** Deferred: platform
  specific and fragile; alpha.2 targets live reload of styles and markup
  instead.

## Open questions

1. **The framework dependency of generated apps.** A git tag pin until crates
   are published under the final name (D5, D6); the CLI PR uses `main` until
   the first alpha tag exists.
2. **Linux menu bar look.** The drawn menu bar needs a visual design. alpha.1
   uses a plain, system-neutral style; the components track may restyle it
   later through the public API.
