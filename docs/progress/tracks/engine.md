# Engine track progress (Workstream A)

Owner: engine track lead. Plan: `docs/alpha/plan.md`, Workstream A. Each
entry names the PR, what changed, the verification command and its result.
Results here are from this branch's tests unless marked otherwise.

## Status

| Unit | State |
| --- | --- |
| RFC 0001 typed style rules | Merged, PR #89 (`b8dcfaf8`) |
| A1 typed style rules | In progress. Step 1, `view!` emits typed constructors: [record](../typed-style-constructors-2026-11-09.md). Step 2, `style!`: [record](../style-macro-2026-11-10.md). Next: `css!` and `stylesheet!` |
| A2 colors (`oklch()`, `color-mix()`) | Merged, PR #95 (`ee46e350`); 54/54 Chromium swatches exact; [record](../color-functions-2026-11-09.md) |
| A2 tokens and media | Not started; needs RFC 0002 |
| A3 top layer and dialog | Not started |
| A5 performance baseline | First internal baseline taken 9 November on one shared Windows host, outside the repository; not publishable. Finding: each dashboard relayout costs about 205 ms, all in layout; not yet profiled |
| A6 real-font baseline | Not measured; fonts are the components track's C4 files (PR #94). WOFF2 decoding on every platform is PR #97 ([record](../woff2-decode-2026-11-09.md)) |

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

## Reported by the components track (9 November 2026)

Not yet reproduced by the engine track; from `docs/progress/tracks/components.md`
on branch `components/c4-assets` and the shared findings note.

- WOFF2 faces fail to register on Linux with `MalformedFont`:
  `platform_font_data` in `bindings/rust/openui-text/src/font/collection.rs`
  passes WOFF2 bytes to FreeType undecoded under `#[cfg(not(windows))]`; the
  census FreeType has no Brotli. Windows decodes with `wuff` first, and
  Chromium always decodes. Owner: `openui-text`. Next engine PR.
- In a column flexbox, a text item with an installed real font lays out one
  word per line (7 words: 140px tall instead of 20px). Owner: layout or text
  intrinsic sizing. Unmeasured here.
- Real-font glyphs sit at a different height from Chromium's, with the same
  horizontal extents. Owner: `openui-text` metrics. Unmeasured here.

## Log

- 9 November 2026, PR #89: RFC 0001 typed style rules merged. Docs only;
  CI green.
- 9 November 2026: `oklch()`, `oklab()` and `color-mix()` parse and
  convert as Chromium 147 does, and paint no longer clamps color channels
  before Skia. Oracle: 54 swatches from pinned Chromium on Linux and
  Windows, identical; `cargo test -p openui --test native_color_functions`
  passes 54 of 54 on Windows. Unsupported forms are rejected; remaining
  limits are in the record.
- 10 November 2026, PR #95 merged (`ee46e350`).
- 10 November 2026: `view!` style attributes emit typed constructors
  instead of run-time `parse_literal` calls, the first A1 step. Every
  property's initial value and 155 corpus literals are checked against
  `parse_literal`; `cargo test -p openui --test typed_style_constructors`
  passes 3 of 3 on Windows. Census and CSS Grid coverage are unchanged.
  [Record](../typed-style-constructors-2026-11-09.md).
- 10 November 2026: `style!` compiles a CSS declaration block to typed
  styles at compile time, the second A1 step. [Record](../style-macro-2026-11-10.md).
