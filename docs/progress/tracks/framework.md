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
| 2 | `openui-cli` skeleton: `new`, `dev`, `build`, `doctor` | In review |
| 3 | Multi-window runtime | Not started |
| 4 | Async runtime integrated with the UI thread and signals | Not started |
| 5 | Clipboard service | Not started |
| 6 | File dialogs | Not started |
| 7 | App menu | Not started |
| 8 | HTTP client | Not started |
| 9 | `default` template and test utilities | Not started |

## Log

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
