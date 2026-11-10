# Engine track progress (Workstream A)

Owner: engine track lead. Plan: `docs/alpha/plan.md`, Workstream A. Each
entry names the PR, what changed, the verification command and its result.
Results here are from this branch's tests unless marked otherwise.

## Status

| Unit | State |
| --- | --- |
| RFC 0001 typed style rules | Written: `docs/alpha/rfcs/0001-typed-style-rules.md` |
| A1 typed style rules | Not started |
| A2 colors (`oklch()`, `color-mix()`) | Not started |
| A2 tokens and media | Not started; needs RFC 0002 |
| A3 top layer and dialog | Not started |
| A5 performance baseline | Not measured |
| A6 real-font baseline | Not measured |

## Findings while writing RFC 0001

- `pointerenter` and `pointerleave` already exist: `Document::dispatch_pointer_event`
  (`bindings/rust/openui/src/document.rs`) dispatches them target-only from the
  engine's hover-path diff. The plan's "no pointer enter or leave events" is
  stale; the remaining work is restyle on hover change, events on window exit
  and re-hover after scroll or layout.
- Window exit loses events: `PointerPhase::Leave` maps to
  `PointerEventKind::Cancel` (`bindings/rust/openui/src/app.rs`), and
  `Engine::pointer_event` still hit-tests the last position for `Cancel`, so the
  hovered path loses hover without receiving `pointerleave`. Owned by A1.
- `Engine::is_active` matches only the pressed element; Chromium's `:active`
  also covers its ancestors. Owned by A1.
