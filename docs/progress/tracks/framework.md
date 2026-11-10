# Framework track progress

Workstream B of the [alpha plan](../../alpha/plan.md): the public `openui`
crate, `openui-platform` and the CLI and tooling crates. Names are working
names until D5 picks the product name.

## Plan for alpha.1

The order and verification of each PR are in
[RFC 0002](../../alpha/rfcs/0002-cli-and-app-layout.md#6-delivery).

| # | PR | State |
| --- | --- | --- |
| 1 | RFC 0002: CLI, app layout and async model | Merged (#90) |
| 2 | `openui-cli` skeleton: `new`, `dev`, `build`, `doctor` | Merged (#93) |
| 3 | Multi-window runtime | Merged (#96) |
| 4 | Async runtime integrated with the UI thread and signals | In review |
| 5 | Clipboard service | Not started |
| 6 | File dialogs | Not started |
| 7 | App menu | Not started |
| 8 | HTTP client | Not started |
| 9 | `default` template and test utilities | Not started |

## Log

### 9 November 2026: async runtime

Change: a new `async` feature of `openui` (enabled by `desktop`; headless
builds gain nothing without it) adds `openui::task` with `spawn_local`,
`spawn`, `spawn_blocking`, `Task<T>` (dropping cancels, `detach`) and
`task::time::{sleep, interval}`, plus `Signal::sender()` and
`create_resource(source, fetch)` with `get`, `loading` and `refetch`. It uses
async-task, async-executor, async-io, blocking and futures-lite, all already in
the lockfile (no new crates). UI tasks run on the UI thread in bounded turns
of 8 ms; their wakers post one coalesced wake through a new
`openui-platform::LoopWaker` and `WindowController::{started, run_tasks}`.
`Signal` and `Memo` are now `!Send`, so a signal moved into `spawn` is a
compile error (a `compile_fail` doctest checks it). `HeadlessApp` gains
`run_until_idle` (fails with `Error::TasksStillRunning` after 30 s) and
`run_ready_tasks`.

Differences from the RFC: a UI task cancelled with its scope never completes
when awaited (its awaiter normally shares the scope); background tasks are
cancelled by dropping their `Task`, not by scope; the optional `tokio` feature
is left for a later PR.

Verification on Windows (working branch):

- `cargo test -p openui --features async --test tasks`: 5 passed (UI task
  awaiting background work, a timer and blocking work; intervals; scope
  disposal and `Task` drop cancel and drop the futures; senders from another
  thread, including to a disposed signal; resources keep only the latest
  result and refetch).
- `cargo test -p openui --features async --doc task`: 2 passed, including
  the `compile_fail` signal-in-`spawn` check.
- `cargo test -p openui-platform --features windows --test wake`: a wake
  from another thread reaches an idle loop (5 wakes sent, coalesced).
- `cargo check --workspace --tests`: passed with `!Send` signals.
- The `tasks` example with `OUI_SMOKE_EXIT=1` (software, auto) exits 0 after
  its background computation and resource fetch finish.
- Linux (WSL2, WSLg Wayland): `wake` and `multi_window` passed with software
  and OpenGL. CI now runs the async tests in `rust-parity` and
  `windows-msvc`, and the `wake` test and `tasks` smoke on X11 and Wayland.

### 9 November 2026: multi-window runtime

Change: `openui-platform` runs any number of windows on one winit event loop
(`run_windows` with a `WindowController` that queues `WindowCommand`s); each
window keeps its own presenter, AccessKit adapter, IME and input state.
`openui` adds `App::new(id).name(..).window(WindowSpec, view).run()` and the
`window` module (`open`, `current`, `all`, `WindowKey::unique`,
`WindowHandle::{close, focus, set_title, is_open}`,
`WindowSpec::on_close_requested` returning `Close::Allow`/`Prevent`) and
`app::exit()`. The existing `App::builder()...run(view)` and the C ABI's
`run_document` now adopt their `App` as one window of the same runtime, so
`window::open` works from them too; `platform::run(app, options)` remains as a
single-window controller.

Findings:

- With two animating windows on Windows, the OS delivers paint requests to the
  top window first; re-requesting a frame as soon as a window drew starved the
  other completely (window 2 presented 41,590 frames and window 1 none in
  60 s). Frames are now requested in rounds: each window that wants a frame
  draws once per round; minimized windows are left out and a round ends after
  250 ms, so a window that is never asked to paint cannot stop the others.
- The OpenGL presenter assumed its context stayed current. With one context
  per window it now makes its context current before presenting and before
  deleting its GL objects.
- `window::all()` was added to the RFC's API. Dev-mode window geometry restore
  (RFC section 1) moves to its own small PR.

Verification on Windows (working branch):

- `cargo test -p openui-platform --features windows --test multi_window` with
  `OUI_BACKEND` = auto, software and opengl: passed. Two windows open, both
  present, opening an open id focuses it and drops the new app, retitling
  works, closing one keeps the other presenting, and the loop exits after the
  last.
- `cargo test -p openui --features windows --lib window`: 5 passed (key
  dedupe, close, close veto with `window::current()`, exit flag reset).
- `OUI_SMOKE_EXIT=1` smokes of `hello` (software, auto) and the new
  `multi-window` example (software, auto, opengl): exit 0.
- `tools/platform/verify_windows_uia.ps1` against `accessibility.exe`,
  software and auto: passed, including closing the window.
- Linux (WSL2 Ubuntu, WSLg Wayland): the `multi_window` test passed with
  software and OpenGL. X11 could not run there (no `libxkbcommon-x11`); CI's
  hardening job now runs the test and the example smoke on X11 (software,
  Mesa OpenGL) and Wayland (software), and `windows-msvc` smokes the example.

### 9 November 2026: `openui-cli` skeleton

Change: a new `bindings/rust/openui-cli` crate (binary `openui`, no new
dependencies) with `new`, `dev`, `build` and `doctor`, and a `minimal`
template. `openui new` writes the Skia build profile from CI into the app's
`.cargo/config.toml`, the workspace `[patch.crates-io]` table (rewritten to the
framework's git source or a local `--framework-path`) and a lockfile seeded from
the workspace's. `openui dev` polls the app's sources, rebuilds, and restarts
the app; it stops the old app by closing its stdin (`OPENUI_DEV`, handled in
`openui-platform/src/desktop.rs`). `openui` gains a `desktop` feature that
selects the host platform, and an `OPENUI_RENDER_PNG` hook that writes the
first frame of `App::run` to a PNG and exits.

Verification on Windows (working branch):

- `cargo test -p openui-cli`: 26 passed. Two tests parse the `rust-parity`
  and `windows-msvc` env blocks in `.github/workflows/ci.yml` so the template's
  Skia profile cannot drift from CI.
- `cargo test -p openui render_png_hook_matches_the_headless_first_frame`:
  passed.
- `python tools/framework/check_cli_template.py`: an app made by
  `openui new demo --framework-path <repo>` and built by `openui build`
  (release) and by plain `cargo build` (dev), with the Skia variables removed
  from the environment, rendered byte-identical PNGs
  (SHA-256 `780C6C3A…261C`). Clean builds took 327 s (release) and 325 s (dev)
  with a cold Skia; an edit-and-rebuild took 3.7 s and the first frame
  appeared in 0.2–0.4 s (dev) and under 0.05 s (release).
- Linux is verified by the new `cli-template` CI job (ubuntu-24.04 and
  windows-2025); its report is uploaded as an artifact.

Follow-up for the engine track: census renders use release Skia (opt-level 3)
while workspace CI tests use dev; the template pins `skia-bindings` to
opt-level 3 in both profiles so apps match census.

### 9 November 2026: RFC 0002

Change: `docs/alpha/rfcs/0002-cli-and-app-layout.md` proposes the CLI
commands, the project template, typed declarations of windows, menus and
services, the async model and the PR order above. Docs only; no code changed.

Findings recorded in the RFC that later PRs depend on:

- A downstream app inherits neither the workspace `[patch.crates-io]` table
  nor the CI Skia build flags, so `openui new` must write both.
- `Signal<T>` is `Send` today and panics off the UI thread
  (`openui/src/signal.rs`); the async PR makes it `!Send`.
- `openui-platform` checks with `--features linux,windows` on Windows
  (`cargo check -p openui-platform --features linux,windows --locked`,
  passed locally), so a single host-selecting `desktop` feature is viable;
  Linux is checked in the CLI PR.
