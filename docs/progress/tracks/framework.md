# Framework track progress

Workstream B of the [alpha plan](../../alpha/plan.md): the public `openui`
crate, `openui-platform` and the CLI and tooling crates. Names are working
names until D5 picks the product name.

## Plan for alpha.1

The order and verification of each PR are in
[RFC 0002](../../alpha/rfcs/0002-cli-and-app-layout.md#6-delivery).

| # | PR | State |
| --- | --- | --- |
| 1 | RFC 0002: CLI, app layout and async model | In review |
| 2 | `openui-cli` skeleton: `new`, `dev`, `build`, `doctor` | Not started |
| 3 | Multi-window runtime | Not started |
| 4 | Async runtime integrated with the UI thread and signals | Not started |
| 5 | Clipboard service | Not started |
| 6 | File dialogs | Not started |
| 7 | App menu | Not started |
| 8 | HTTP client | Not started |
| 9 | `default` template and test utilities | Not started |

## Log

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
