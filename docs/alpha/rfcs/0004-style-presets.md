# RFC 0004: Opt-in style presets

Status: proposed, 11 November 2026. Owner: engine track (Workstream A).
Consumers: framework track (`openui`, app and window API), components track
(shadcn library), docs and website. Builds on
[RFC 0001](0001-typed-style-rules.md).

## Problem

Div has no user-agent stylesheet. An element starts from CSS initial
values plus a native `display` default chosen by its kind
(`Engine::create_native_element`). The tag name is not kept:
`tag_definition` in `openui/src/element.rs` maps `p`, `h1` to `h6`, `ul`,
`ol`, `section` and others to `ElementTag::Div`. So `<h1>` and `<p>` render
the same, and no selector can tell them apart: the A1 matcher, not yet
merged, compares a type selector with the one name `local_name` gives each
`ElementTag`. Form controls are the
exception: `openui-paint` ports Chromium's `NativeThemeBase` painting for
buttons, text fields, selects, checkboxes, radios, range, meter, progress,
color, date and file inputs, with control metrics hard-coded in the painter.

Unstyled stays the default. Developers also need a known starting point:

- (a) **none**: today's behavior, the default;
- (b) **browser**: Chromium 147's defaults, which also covers Edge;
- (c) **components**: the shadcn-based library's base (browser defaults,
  Tailwind's preflight and the theme tokens);
- (d) **custom**: the developer's own base sheets, or none.

Today the only browser defaults are partial tables in the WPT porter
(`_UA_DEFAULTS` in `tools/wpt/port_wpt.py`), which bake Chromium's values
into each census fixture as explicit styles.

## Proposal

### API (`openui`, over `openui-engine`)

```rust
pub enum UserAgentStyle { None, Browser }            // (a), (b)
pub struct StylePreset {
    pub user_agent: UserAgentStyle,
    pub base: &'static [&'static StyleSheet],        // author origin, adopted first
}
impl StylePreset {
    pub const NONE: Self;                            // default
    pub const BROWSER: Self;                         // Browser, no base sheets
}

App::new("dev.example.notes")
    .style_preset(StylePreset::BROWSER)               // default for every window
    .window(WindowSpec::new("main").style_preset(div_components::PRESET), view)
    .run()?;
document.set_style_preset(StylePreset::NONE)?;       // at run time
```

A preset belongs to a `Document`, so each window can differ. The app value
is the default for new windows. Changing it after the first frame restyles
the whole document (for example a devtools toggle). (d) is any
`StylePreset { user_agent, base }` built from the developer's own
`stylesheet!` sheets.

### Cascade

RFC 0001's cascade already has a user-agent origin, below every author rule.
`UserAgentStyle::Browser` fills that origin with one compiled `StyleSheet`.
`None` leaves it empty, so resolution is exactly today's path: initial values
plus native `display` defaults. `base` sheets are ordinary author sheets,
adopted before any other. They keep their own `@layer` statements, so the
document's layer order starts with theirs. That is how a web page loads
Tailwind's preflight, so the components preset layers exactly like the
shadcn reference pages. Element-attached `style:` declarations still beat
every rule. `revert` in an author rule rolls back to the preset's
user-agent value.

### Browser preset source and licensing

Chromium's `core/html/resources/html.css` is LGPL-2.0-or-later (KHTML and
Apple copyright). Firefox's `html.css` and `forms.css` are MPL-2.0. Div is
Apache-2.0 and links its sheets statically, so neither file is copied.
The preset is written from the WHATWG HTML Rendering section. The WHATWG
licenses that text CC BY 4.0, and portions in source code BSD-3-Clause; the
preset records the attribution in `NOTICE`. Where Chromium differs (form
control metrics, `font: -webkit-small-control`, focus rings, platform
sheets such as `linux.css`), the values come from Chromium's computed
style, measured by the oracle below. They are facts measured from the
running browser; no Chromium text is copied. **This needs a legal check
before the preset ships.** A Firefox preset is out of scope. MPL-2.0 is
file-level copyleft, so it could live in its own MPL-licensed file later,
but there is no Firefox oracle to verify it against.

## Prerequisites

1. **Element identity.** `NodeData` keeps a `LocalName` beside its kind: an
   interned HTML name (`h1`, `p`, `blockquote`, `code`, ...). `ElementTag`
   stays the layout and paint kind. Type selectors match `LocalName`, which
   amends RFC 0001's `Compound { tag: Option<ElementTag> }` to
   `Option<LocalName>`. Owners: `openui-dom` (no owning track; one focused
   PR), `openui-engine` (`create_native_element_named`), and the framework
   track (`tag_definition` passes the name; `view!` accepts the HTML
   elements the preset styles, not only today's list).
2. **Stylesheets and the cascade**, from A1 (`stylesheet!`, origins,
   layers, invalidation), plus what the user-agent sheet itself needs:
   system colors (`Canvas`, `CanvasText`, `ButtonFace`, `Field`,
   `LinkText`), `light-dark()` and `color-scheme`, and the internal values
   Chromium uses, compiled for the user-agent origin only.
3. **Form controls.** The `appearance` property (`auto`, `none`) controls
   whether native theme painting runs. So do Chromium's "styled control"
   rule (`LayoutTheme::IsControlStyled`), under which an author background or
   border drops the native look, and its `AdjustStyle` metrics, which move
   out of the painter into the preset. `None` keeps today's painting
   unchanged.
4. **Platform variants.** The browser preset selects the Windows or Linux
   additions that Chromium 147 applies on that platform.

## Verification

A **user-agent qualification suite**. Each page has no author CSS: the
HTML elements the preset styles, tables, lists, and every control type
in each state. Chromium 147 on Windows and Linux dumps `getComputedStyle`
for every longhand and captures pixels at the census profiles. The native
fixture builds the same tree with `StylePreset::BROWSER` and compares
`Element::computed_style` value by value, and pixels exactly. A property
that is not compared counts as a failure, not a pass. The components preset
is verified by the C3 reference pages it already serves. `None` must leave
the census and coverage unchanged (D4). As a final check, the porter's
`_UA_DEFAULTS` can be replaced by the preset, and the census must not change.

## Affected tracks

| Track | Change |
| --- | --- |
| engine | `LocalName` matching, user-agent origin per `Document`, the browser preset sheet in `openui-style`, `appearance` and the styled-control rule, system colors, the qualification suite |
| framework | `AppSpec::style_preset`, `WindowSpec::style_preset`, `Document::set_style_preset`, `tag_definition` passing names, a wider `view!` element list, API docs |
| components | `div_components::PRESET`: browser defaults, preflight and tokens as base sheets in their own layers; components state the preset they expect |
| docs / website | a "Choosing a starting style" guide, with an example for each preset |

## Migration

Each step keeps `None` the default, so `main` stays green and census and
coverage cannot change until a fixture opts in.

1. This RFC.
2. `LocalName` in `openui-dom` and the engine; the framework passes names.
   No visible change.
3. Type selectors on `LocalName` (engine), amending RFC 0001.
4. Engine: a user-agent origin slot per `Document`, empty by default.
5. A1's `stylesheet!` and cascade land (already planned).
6. Browser preset in parts, each with its suite: text and sections, then
   lists and tables, then forms (after prerequisite 3).
7. Framework API and docs; then the components preset after C1.

The project moves to `build-with-div/div` soon; this RFC moves with the
others.
