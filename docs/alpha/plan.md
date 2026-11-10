# Alpha plan: parallel workstreams (draft for review)

Status: draft, 9 November 2026, revised after the feasibility review below.
Week 0 is under way. D1–D4 are decided; D5 (name) is deferred and D6
(version line) is open.

## Goal

A batteries-included Rust framework for desktop apps. It renders natively, with
no webview and no JavaScript, and looks pixel-identical to Chromium rendering
the same HTML and CSS. It should have a native Rust app's startup time,
memory and CPU use, and a web framework's UI fidelity, so popular web
component libraries (shadcn/ui, Tailwind) port with little change.

We ship an **alpha soon**, iterate with users, and do not freeze the API
before we have to.

## What we already have

| Asset | State |
| --- | --- |
| Rendering engine (style, layout, paint, text, Skia) | 94.9% pixel-exact across 23,728 comparisons in 10 CSS areas. Grid measured once at 52.8% in a separate coverage suite (PR #88). Text, tables, transforms and others are unmeasured. |
| Chromium comparison harness | Ports WPT tests, captures pinned Chromium 147, compares pixel for pixel at four scales. This is our strongest asset: we can *prove* fidelity. |
| App authoring model | `view!` macro (JSX-like), `#[component]`, fine-grained signals and effects, `For`/`Show`, typed events (`on:click`). Inline `style:` properties only. |
| Native element API | `Document`/`Element`: tree mutation, attributes, classes (for lookup), focus, scrolling, form controls, animations and transitions. |
| Platforms | Linux: window, input, accessibility (AT-SPI), X11 and Wayland. Windows: builds and passes tests, DirectWrite text, experimental Win32 window. macOS: none. |
| C ABI | 131 exports, C and C++ consumers pass. |
| Examples | counter, todo, dashboard, accessibility, hello. |
| Alpha RFCs | RFC 0001, typed style rules (PR #89), and RFC 0002, CLI and app layout (PR #90), are merged under `docs/alpha/rfcs/`. |
| CLI | `openui-cli` (binary `openui`) landed in PR #93: `new` writes a minimal app, `dev` rebuilds and restarts it on change, `build` packages a release into `dist/` and `doctor` checks the toolchain. A `cli-template` CI job on Linux and Windows builds the template with `openui build` and with plain `cargo build` and requires identical first frames. |
| Colors (A2) | `oklch()`, `oklab()` and `color-mix()` parse and convert as Chromium 147 does, and paint no longer clamps color channels before Skia (PR #95): 54 of 54 swatches exact against pinned Chromium on Linux and Windows. |
| App services (B1) | Several windows on one event loop (PR #96), an async runtime on the UI thread and in the background (#98), clipboard text (#99), native open, save and folder dialogs (#100), and an app menu with accelerators and a native Windows bar (#101), drawn in the window where the platform has no menu bar (#103), and an HTTP client over rustls and the system roots (#105). |
| Components | The shadcn/ui reference harness and the separator landed in PR #91: 8 of 16 comparisons exact. All light comparisons are exact; every dark one was off by one color level from the paint clamp that PR #95 removed, and has not been remeasured since. Geist, Inter and the Lucide icons are bundled (C4, #94), and button, badge, card and label landed in #102. Every fixture with text is still an error, because the bundled WOFF2 fonts do not load on Linux with the census FreeType (engine PR #97, open). |

### Gaps that block porting shadcn/Tailwind

- No stylesheet cascade: no class or attribute selectors, no `:hover`,
  `:focus-visible`, `:active` or `[data-state=open]` styling.
- No CSS custom properties (`var(--primary)`), which shadcn theming is
  built on.
- No media queries or `prefers-color-scheme` (dark mode, responsive
  variants).
- No top layer: dialogs, popovers, dropdowns, tooltips, toasts, and no
  anchor positioning.
- No style invalidation on state change. The engine tracks hover,
  focus-visible and IME composition, but nothing restyles an element when
  that state changes.
- Two pointer-state bugs (RFC 0001). `pointerenter` and `pointerleave`
  already exist (target only, as in the DOM), but when the pointer leaves the
  window the hovered path loses hover without receiving `pointerleave`.
  `:active` matches only the pressed element, while Chromium's also covers
  its ancestors.
- Real fonts. WOFF2 fonts do not load on Linux with the census FreeType
  (PR #97, open). In the components track's previews, which are not
  measured evidence, a column flex item with text is as tall as one word per
  line, and glyphs sit at a different height from Chromium's.
- App services: no tray, notifications or global shortcuts.
- No hot reload (the CLI's `dev` rebuilds and restarts the app), no devtools,
  and one minimal project template.

## Feasibility review (9 November 2026)

**Verdict.** The direction is feasible and much of the foundation exists. The
first alpha as originally drafted was not: it promised pixel-identical
real-font components, overlays, hot reload and 6–8 concurrent tracks, and each
of those had an unmeasured or missing prerequisite. The sections below now
use the narrower alpha.1 this review recommends.

**Well founded:**

| Claim | Evidence |
| --- | --- |
| Typed styles with CSS syntax (D1) | `view!` already validates every inline property and value at compile time (`openui-macros/src/view.rs`, through `StyleProperty::from_css_name` and `openui_style::parse_literal`). The emitted code still re-parses the literal at runtime, which typed values would remove. |
| Interaction state for variants | `openui-engine/src/interaction.rs` tracks hover, focus-visible and IME composition, and `Document` dispatches `pointerenter` and `pointerleave`. |
| Windows at alpha.1 (D3) | The Win32 runtime (WGL or softbuffer, Unicode clipboard, UI Automation adapter) and DirectWrite text checked against Windows Chrome-for-Testing 147 landed on 21 October 2026. IME, drag and drop, UIA behavior and packaging are unverified. |
| Overlays can build on existing layout | `position: fixed` exists. |
| Disk | E: has 916 GB free. One `bindings/rust/target` is 50 GB. |

**Not feasible as first drafted:**

1. **Pixel-identical shadcn with real fonts.** The census mostly uses Ahem.
   Real-font evidence is weaker: Chromium Linux LCD real-font comparisons are
   817/904 exact in the latest expanded census; the rejected native consumer
   trial had 73/1,200 font images exact; and an installed-font advance bug is
   open (5.9375px where Chromium places 6px). shadcn sets nearly everything in
   Geist or Inter at 14px. A new unit, A6, measures and repairs this, and the
   alpha claims *measured* fidelity per component instead of identity.
2. **Typed rules are a runtime feature as well as a macro.** State variants
   need style invalidation, `pointerleave` when the pointer leaves the
   window, `:active` on the pressed element's ancestors, and cascade order
   and specificity reproduced exactly. A1 is the largest unit and gates every
   interactive component.
3. **Tailwind v4 colors.** `oklch()` and `color-mix()` must be parsed and
   converted exactly as Chromium does (including gamut mapping), or no themed
   component can be exact. Added to A2, and landed in PR #95.
4. **Overlays.** There is no top layer or anchor positioning. alpha.1 keeps
   one overlay (dialog); menus, select, tooltip and toast move to alpha.2.
5. **Hot reload.** Rust has no cheap code reload. alpha.1 `dev` is a fast
   rebuild and restart; live reload of styles and markup is alpha.2.
6. **Six to eight concurrent lead sessions.**
   - One full census takes about 75 minutes with 16 jobs in WSL (31 GB of the
     host's 64 GB RAM, 24 logical cores). Only one can run at a time, so per-PR
     full census does not scale beyond one engine track.
   - Each worktree builds its own Skia and a 50 GB target directory.
   - Session usage limits were already reached once in this project; eight
     leads with subagents multiply spend.
   - Plan for 3–4 concurrent leads, a shared prebuilt Skia, focused tests per
     PR and a queued full census for engine merges.
7. **macOS from day one.** It needs Mac hardware (or hosted macOS runners) and
   a macOS Chromium oracle. Neither exists, so B4 cannot start until that is
   decided.
8. **Performance claims.** Startup, idle memory and frame time against
   Electron or Tauri are unmeasured. Raster is CPU Skia; the Ganesh GPU path
   is unpromoted (408/640 focused exact). Offscreen culling is unproven:
   `openui-paint/src/paint_program.rs` notes that geometry cull bounds "have
   not been proven conservative". A5 measures before the website publishes any
   benchmark.
9. **Domains and trademarks.** Agents can check availability, but buying a
   domain or creating organizations is yours to do.

## Decisions needed

**D1. Styling model: typed styles, CSS syntax. Decided.**
Styles are strongly typed objects, not CSS text interpreted at runtime. CSS
was designed to be downloaded, parsed and interpreted; a native framework can
check everything at compile time instead. We keep CSS's *syntax*, so web
libraries translate easily.

- **`css!` / `style!` macros** parse CSS syntax at compile time into typed
  `Style` values. An unknown property, an invalid value or a wrong unit is a
  compile error. Conflicting declarations (two values for one property in
  the same rule, a shorthand overriding a longhand) are compile-time
  warnings or errors.
- **Class names and stylesheets** (`stylesheet!`, or a build step for `.css`
  files) compile selectors and state variants into typed rules: a typed
  condition (`Hover`, `FocusVisible`, `Active`, `Disabled`,
  `Data(State::Open)`, `Media(ColorScheme::Dark)`, `Media(MinWidth(768))`,
  and so on) plus a typed style. At runtime the engine evaluates only typed
  conditions on state changes, with no string selector matching and no
  parsing. An unknown class name is a compile error.
- **Tailwind** becomes a compile-time mapping from utility class names to
  typed rules. `class="flex gap-2 hover:bg-muted data-[state=open]:..."`
  keeps working, and a typo fails the build.
- **Custom properties** become typed theme tokens. `var(--primary)` resolves
  to a token reference checked at compile time; themes (light and dark)
  are typed values, swapped at runtime.
- **Authoring tools** let designers write plain CSS and class names; the
  build compiles them to the typed objects. Dynamic values go through typed
  setters.
- **Runtime:** state changes (hover, focus, active, `data-state`, media)
  invalidate and restyle only the affected elements. This needs a new
  invalidation path and the two pointer-state fixes above.
- **Fidelity:** computed results are still checked against Chromium, so
  typed rules must reproduce CSS cascade order and specificity exactly.

**D2. Parallel working policy. Decided: per-track.** AGENTS.md now allows one
branch and one PR at a time, with no worktrees. Max parallelism needs:
- one long-lived track branch per workstream, each in its own git worktree;
- small PRs per track merged to `main` independently;
- crates and directories owned by one track;
- cross-track API changes through a one-page RFC under `docs/alpha/rfcs/`.

The website lives in its own repo; docs content stays in this repo. The policy
change needs your approval.

**D3. Alpha platforms. Decided.** Windows and Linux at alpha.1, macOS in
alpha.2. Web developers are heavily on macOS, so macOS must not trail far
behind. Open: macOS needs Mac hardware or hosted macOS runners, plus a macOS
Chrome-for-Testing oracle. B4 starts once that is chosen.

**D4. Alpha quality bar. Decided.** Alpha releases require the census
and coverage suites to show **no regressions**, not 100% exactness. The v0.2
release gates (zero tolerance, hardware lab, signed reproducible builds)
become a later stability milestone and stop blocking releases. Each alpha
publishes honest conformance numbers per CSS area.

**D5. Name. Deferred.** Open UI stays the project name for now. Branding
will be decided in a later conversation and does not block alpha work.
Starting ideas for that conversation, none checked yet: *Glaze*, a finish
over a native body; *Lacquer*; *Tessera*; *Cast*, a pixel-exact casting of a
mold; *Kiln*. Names already taken in Rust or UI and best avoided: Facet,
Patina, Parity, Mica, Ember, Loom.

**D6. Version line.** Recommendation: publish under the new name as
`0.1.0-alpha.N`, with `0.x` minor bumps allowed to break the API. Keep "v0.2
qualification" as an internal engine milestone.

## Alpha definition (alpha.1)

A developer can:

1. `cargo install <name>-cli`, then `<name> new my-app --template shadcn`.
2. `<name> dev`: fast incremental rebuild and restart on save.
3. Build an app with 10 shadcn components: button, input, label, card, badge,
   separator, checkbox, switch, tabs and dialog, in light and dark themes.
   Each is compared against shadcn rendered in Chromium with our harness, and
   its exact-pixel result per profile is published. Exactness is the target,
   not the release condition.
4. Use app essentials, integrated from existing crates rather than built:
   multiple windows, an app menu, file open and save dialogs, clipboard, an
   async task runtime and an HTTP client.
5. `<name> build` produces a release binary for Windows and Linux.
6. Read a docs site: getting started, a tutorial, the "coming from the web"
   guide, the API reference, and a published CSS support matrix generated
   from the census.

Moved to alpha.2: dropdown-menu, tooltip, select, toast, scroll-area, live
reload of styles and markup, notifications, installers and macOS.

Not promised in alpha: API stability, Windows accessibility behavior, signed
installers, auto-update.

## Workstreams

Each track has a lead session. The lead fans out subagents for the
independent units listed. "Owns" means no other track edits those paths
without an RFC.

### A. Engine: rendering and web-platform mapping

Owns `openui-style`, `openui-layout`, `openui-paint`, `openui-text`,
`openui-engine` and the qualification tooling.

| Unit | Deliverable | Parallel? |
| --- | --- | --- |
| A1 Typed style rules (D1) | `css!`/`style!` macros with compile-time validation and conflict detection; `stylesheet!` compiling selectors and variants to typed conditions; typed values emitted without runtime re-parsing; style invalidation on state change; `pointerleave` when the pointer leaves the window and `:active` on ancestors; cascade order and specificity checked against Chromium | One owner; critical path and the largest unit |
| A2 Tokens, colors and media | Typed theme tokens for `var()`; `oklch()` and `color-mix()` converted exactly as Chromium does; typed media conditions (width, `prefers-color-scheme`, `prefers-reduced-motion`) | Colors start now; tokens and media after A1's data model |
| A3 Top layer and dialog | Top layer, modal dialog, focus trap, dismissal and z-order for alpha.1. Popover and anchor positioning for menus and tooltips in alpha.2 | Parallel with A1 |
| A4 Conformance by area | For alpha.1, only the areas the 10 components use. Other coverage suites (grid, merged in PR #88; tables, transforms) after alpha.1. Merges are serialized where areas share `block.rs` | Low priority until alpha.1 |
| A5 Performance and resources | Benchmark harness against Electron and Tauri (startup, idle RSS, frame time). Prove or add offscreen culling. List virtualization primitive. Results gate any published benchmark | Parallel; measure in week 0 |
| A6 Real-font text | Baseline Geist and Inter at shadcn's sizes and weights on Linux and Windows against Chromium; repair the installed-font advance and the largest real-font differences | Parallel; on the critical path for component exactness |

### B. Application framework

Owns `openui` (the public crate), `openui-platform` and the new CLI and
devtools crates.

| Unit | Deliverable | Parallel? |
| --- | --- | --- |
| B1 OS services | Integrate existing crates where possible. alpha.1: multi-window, app menu, file dialogs, clipboard, async runtime, HTTP client. Later: tray, notifications, drag and drop, global shortcuts, single instance, deep links | Parallel; one agent each |
| B2 App architecture | Router and navigation, async resources and suspense, forms and validation, global state and stores, error boundaries, i18n, asset pipeline (fonts, images, icons) | Mostly parallel |
| B3 Tooling | CLI (`new`, `dev` as rebuild and restart, `build`), project templates and test utilities (headless render, snapshot and pixel tests) for alpha.1. Live reload of styles and markup, `bundle` and the devtools inspector in alpha.2 | Parallel after the CLI skeleton |
| B4 Platform adapters | macOS adapter (window, input, text through CoreText, accessibility); Windows IME, drag and drop and packaging checks | macOS is the largest unit; blocked on hardware and a macOS oracle (D3) |

### C. Component ecosystem

New `components/` workspace (or repo), consuming the public crate only.

| Unit | Deliverable | Parallel? |
| --- | --- | --- |
| C1 Tailwind bridge | Compile-time mapping of Tailwind v4 utility classes and variants to A1's typed rules (unknown class fails the build); shadcn theme as typed tokens | After A1 lands |
| C2 Primitives | Radix-equivalent behaviors in Rust. alpha.1: dialog, tabs, roving focus, dismissable layer. alpha.2: popover, menu, select, tooltip, collapsible, portal | One agent each; dialog needs A3 |
| C3 shadcn components | One agent per component. Each is compared against shadcn rendered in Chromium with our harness and its result published | Static ones (button, card, badge, separator, label) can start now with inline styles and sRGB stand-ins for theme colors |
| C4 Assets | Lucide icons; Geist and Inter fonts bundled | Parallel |
| C5 Port assistant | Tool that translates React plus Tailwind JSX into `view!` code, to port more libraries at scale | Later in alpha |

### D. Docs and education

Owns `docs/guide/` (new) and the API reference.

- Getting started, tutorial (build a notes app) and core concepts.
- **Coming from the web**, a translation guide (see below).
- A CSS support matrix generated from census and coverage evidence.
- A cookbook, and migrating from Electron or Tauri.
- Every unit is parallel by section. The guide needs engine facts from A5.

### E. Brand and web presence

Own repo for the website.

- Name (D5), logo and visual identity, domain, GitHub org, crates.io names.
- Landing page: benchmarks against Electron and Tauri, a live side-by-side
  pixel diff with Chromium, and a component gallery.
- Docs site infrastructure (a static site generator plus rustdoc), community
  channel (Discord or GitHub Discussions), and a launch post.

### F. Release and feedback loop

- Alpha release engineering: three-platform CI, versioning, changelog,
  templates and a release checklist. Cadence: an alpha every two weeks.
- Feedback: issue templates, a "missing CSS feature" report that names the
  WPT area, and an opt-in crash report.
- **Dogfood apps**, built by agents with only the public API: a notes app, a
  settings and dashboard app, and a chat client. Each gap they hit becomes a
  tracked issue for A, B or C.

## Dependencies and critical path

```
D5 name ──► rename crates ──► E site/docs infra ──► launch
D1 ──► A1 typed rules + invalidation ──► A2 tokens/media ──► C1 Tailwind ──► C3 interactive components
A2 oklch/color-mix ─────────────────────────────────────────┘
A6 real-font text ──► C3 exact results
A3 top layer ──► C2 dialog ──► C3 dialog
B3 CLI skeleton ──► templates ──► D getting started
A5 measurements ──► E benchmarks on the website
Mac hardware decision ──► B4 macOS (long pole for alpha.2)
```

Start immediately, with no dependencies: A2 colors, A3, A5, A6, B1 (each
service), B2, B3, C3 static components, C4, D outline, E naming and site, and
F CI and cadence.

Phases:

1. **Week 0: decisions, setup and baselines.** D6 answered (D5, the name, is
   deferred); AGENTS.md updated; track branches and worktrees created; a shared prebuilt Skia for
   all worktrees; one-page RFCs for the cascade and invalidation data model
   and the CLI layout. Two measurements before building: A5's startup,
   memory and frame-time baseline, and A6's Geist and Inter baseline at
   shadcn sizes.
2. **Build.** 3–4 concurrent lead sessions, each fanning out subagents:
   - A: A1, then A2 and A3 (one engine lead, so engine merges stay serial);
   - B: B1 and B3;
   - C: C3 static components, then C1 and C2;
   - D+E, a lighter docs and brand lead, started when usage allows.

   Engine PRs run focused tests; full census runs from one queue, at least
   before each engine merge batch and each alpha. Weekly integration on
   `main` with census nonregression.
3. **alpha.1.** When the alpha definition above is met on Windows and Linux.
4. **alpha.2 onward.** macOS, installers, more components, and C5, driven by
   user feedback.

## Coming from the web: first answers

These answers go into the guide (D). Items marked *verify* need an engine
check (A5) before publication.

| Web concept | In this framework |
| --- | --- |
| The DOM | Still exists as a retained element tree (`Document`, `Element`), but you rarely touch it. You write `view!` markup with signals; updates are fine-grained, with no virtual DOM diffing (as in Solid or Leptos). |
| JS performance tuning | There is no JS engine, GC or IPC bridge. Event handlers are compiled Rust. |
| Reflow and repaint | Still real: style, layout and paint cost the same kinds of work as in Chromium. Avoiding layout thrash still matters, but there is no forced synchronous layout across a JS boundary. |
| Lazy loading | Network lazy loading mostly disappears, because assets are bundled. Lazy *construction* still matters: virtualize long lists to save memory and layout time. |
| Above the fold | No network waterfall, so it becomes "time to first frame". Keep startup work small and defer non-visible work. |
| Offscreen content | Layout runs for the whole tree. Whether offscreen content is painted or rastered is *verify*: damage tracking exists, but paint cull bounds are not yet proven conservative (A5). |
| CSS | The same CSS semantics, checked against Chromium. A support matrix shows exactly what is measured, area by area. |
| npm packages | Crates. UI libraries are ported (C3, C5), not loaded. |
| `fetch`, `localStorage` | A Rust HTTP client and storage APIs supplied by the framework (B2, B4). |

## Risks

- **Scope.** "Batteries included" is large. Keep alpha.1 to the definition
  above and let dogfood apps choose the next batteries.
- **Merge contention in the engine.** `block.rs` is about 40,000 lines;
  serialize merges there and keep A-track PRs small.
- **Typed rules versus cascade semantics.** Compile-time typed rules must
  still reproduce CSS cascade order, specificity and inheritance exactly;
  the Chromium oracle checks this.
- **Real-font text.** Component exactness depends on Geist and Inter matching
  Chromium; current real-font evidence is much weaker than the Ahem census.
  A6 measures first, and alpha claims stay per component.
- **macOS.** CoreText text must match Chromium's macOS rendering, and that
  needs Mac hardware and a macOS oracle as Windows needed one.
- **Capacity.** One full census at a time (about 75 minutes), 50 GB per
  worktree target directory and session usage limits cap useful parallelism
  at 3–4 leads. Add leads only after measuring spend.
- **Performance claims.** Nothing is published until A5 measures it; CPU
  raster at high DPI may lose frame-time comparisons until the GPU path is
  promoted.
- **Fidelity claims.** Publish measured numbers only; the support matrix keeps
  the claims honest.
- **Process.** AGENTS.md's single-PR rule must change before tracks run in
  parallel (D2).
- **Owner-only steps.** Buying domains, creating the GitHub organization and
  trademark filings are yours; agents prepare the shortlist and checks.
