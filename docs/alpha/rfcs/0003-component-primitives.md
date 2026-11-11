# RFC 0003: Component primitives

Status: proposed, 10 November 2026. Owner: components track
(Workstream C, unit C2). Consumers: shadcn components (C3). Requests to: the
framework track (`openui`, events and context) and the engine track (A1 data
attributes, A3 top layer).

## Problem

shadcn/ui builds its interactive components on Radix UI: Radix owns state,
keyboard, focus, ARIA and dismissal, and shadcn adds only classes. Open UI
has elements, events, focus and signals in its public crate, but no layer of
these behaviors, and a few pieces Radix relies on are missing (see Affected
tracks). The components that need them (tabs, dialog, checkbox and switch in
alpha.1) cannot be built and compared with shadcn in Chromium until both
exist.

## Proposal

This RFC fixes how Radix's behaviors are written for Open UI. It covers
alpha.1: roving focus, tabs, dismissable layer, focus scope, dialog, checkbox
and switch. Popover, menu, select, tooltip and collapsible follow in alpha.2
on the same pieces.

### Principles

1. **Headless.** A primitive renders elements, sets attributes and handles
   input. It sets no visual style; C3 components style it, today inline and
   later through C1's typed rules.
2. **Radix's DOM contract.** Each part renders the element, `role`, `aria-*`,
   `id` relations, `tabindex` and `data-*` attributes (`data-state`,
   `data-orientation`, `data-disabled`) that Radix renders: `radix-ui` 1.8.1
   as pinned in `components/reference/package-lock.json`, whose packages
   are `react-roving-focus` 1.2.1, `react-tabs` 1.2.1,
   `react-dismissable-layer` 1.2.1, `react-focus-scope` 1.3.1,
   `react-dialog` 1.3.1, `react-checkbox` 1.4.1 and `react-switch` 1.4.1.
   shadcn's variants select on these
   (`data-[state=active]:bg-background`), so C1 maps them to RFC
   0001's `Data(...)` conditions unchanged, and the reference pages stay
   comparable element by element.
3. **Radix's behavior, checked in Chromium.** Every keyboard, pointer and
   focus rule is copied from the pinned Radix source, and a trace harness
   (below) compares the result with Radix running in Chromium. Where this RFC
   summarizes a rule, the Radix source is the specification.
4. **Controlled or uncontrolled.** A value is either owned by the primitive
   (a default) or by the caller (a signal and a change callback), as Radix's
   `useControllableState`.
5. **Public crate only.** Primitives use `openui`'s public API. What they
   cannot do with it is listed under Affected tracks, each with the interim
   used until it lands.

### Crate and module layout

A new crate `openui-primitives` (working name) in the `components/`
workspace, so an app can use the behaviors without shadcn's look, as with
Radix's own packages. `openui-components` depends on it.

```
components/openui-primitives/src/
  lib.rs            re-exports
  controllable.rs   Controllable<T>: default value or Signal<T> + on_change
  id.rs             unique ids for aria-controls / aria-labelledby
  keys.rs           key names from keyboard events (request 2)
  collection.rs     ordered items of a group, in tree order
  roving_focus.rs   RovingFocusGroup, RovingFocusItem
  tabs.rs           Tabs, TabsList, TabsTrigger, TabsContent
  dismissable.rs    DismissableLayer and its layer stack
  focus_scope.rs    FocusScope (trapped, looped, auto focus, restore)
  dialog.rs         Dialog, Trigger, Portal, Overlay, Content, Title,
                    Description, Close
  checkbox.rs       Checkbox, CheckboxIndicator
  switch.rs         Switch, SwitchThumb
```

### Shared pieces

**State.** `Controllable<T>` holds either an internal `Signal<T>` seeded
from a default, or the caller's signal; setting it calls `on_change` in both
cases, and in the controlled case leaves the update to the caller, as Radix
does.

**Parts and context.** Radix passes the root's state to its parts through
React context. Open UI has no context API yet, so a compound primitive is
created as a handle and each part takes it as a prop:

```rust
let tabs = TabsState::new(TabsOptions { default_value: Some("account"), ..Default::default() });
view! {
    <Tabs state={tabs.clone()}>
        <TabsList state={tabs.clone()}>
            <TabsTrigger state={tabs.clone()} value="account">"Account"</TabsTrigger>
        </TabsList>
        <TabsContent state={tabs.clone()} value="account">...</TabsContent>
    </Tabs>
}
```

With a context API (request 1) the `state` props disappear and the markup
matches Radix's.

**Items in order.** Roving focus moves through a group's items in tree
order, as Radix's `Collection` sorts by DOM position. Items register with
the group on mount and unregister on cleanup; the order is read from the
tree when a key is handled, so reordering needs no bookkeeping.

**Ids.** Parts that refer to each other (`aria-controls`,
`aria-labelledby`) get ids from a per-document counter. The values are
never compared with Chromium's; only the relations are.

**Mounting.** Radix's `Presence` mounts a part while it is open (tab
content, dialog content) and unmounts it after. `Show` does the same.
Exit animations (`data-[state=closed]:animate-out`) need animation events
from typed rules and wait for C1.

### Primitives

**RovingFocusGroup.** One item has `tabindex="0"` and the rest `-1`, so Tab
enters the group once. Arrow keys move focus by orientation and direction
(`ArrowLeft`/`ArrowRight` horizontal, `ArrowUp`/`ArrowDown` vertical,
reversed in `dir="rtl"`), `Home`/`PageUp` and `End`/`PageDown` go to the
ends, `loop` wraps, disabled items are skipped, and a key with Ctrl, Alt,
Meta or Shift is ignored. Focus entering the group moves to the active item.
Shift+Tab from an item leaves the group.

**Tabs.** Root: `value`, `default_value`, `orientation`, `dir`,
`activation_mode` (automatic or manual). List: `role="tablist"`,
`aria-orientation`, a looping roving focus group. Trigger: a `button` with
`role="tab"`, `aria-selected`, `aria-controls`, `data-state`
(`active`/`inactive`) and `data-disabled`; a primary-button `mousedown`
without Ctrl selects it, Enter or Space selects it, and in automatic mode so
does receiving focus. Content: `role="tabpanel"`, `aria-labelledby`,
`tabindex="0"`, mounted only while active.

**DismissableLayer.** Escape (for the topmost layer only), a pointer down
outside the layer, and focus moving outside it each call a cancellable
handler, then `on_dismiss`. Layers form a stack in the document; a modal
layer makes the rest of the page ignore the pointer
(`disableOutsidePointerEvents`). Listeners sit on the document root in the
capture phase; "outside" is a parent walk from the event target.

**FocusScope.** When trapped, Tab and Shift+Tab cycle inside the scope.
On mount it focuses the first tabbable element that is not a link, or the
scope itself; on unmount it returns focus to the element focused before. The
engine's modal root (`Document::set_modal_root`) already contains Tab
navigation, focuses the first candidate and restores the previous focus;
the scope uses it and adds Radix's choice of first element. The engine
holds one modal root, so nested dialogs need a stack (request 4).

**Dialog.** Root: `open`, `default_open`, `modal`. Trigger:
`aria-haspopup="dialog"`, `aria-expanded`, `aria-controls`, `data-state`.
Portal: the content and overlay leave the normal tree for the top layer
(A3); until then they are appended at the end of `<body>` with
`position: fixed`. Overlay: `data-state`. Content: `role="dialog"`,
`aria-labelledby` (Title), `aria-describedby` (Description), `data-state`, a
trapped focus scope and a dismissable layer when modal; scrolling of the
page is locked while open (`overflow: hidden` on `<body>`), and everything
outside is hidden from assistive technology. Closing returns focus to the
trigger. Close: a button that closes.

**Checkbox and switch.** A `button` with `role="checkbox"` (or `"switch"`),
`aria-checked` (`true`, `false`, or `mixed` for an indeterminate checkbox)
and `data-state` (`checked`, `unchecked`, `indeterminate`). A click toggles
it; for a checkbox, Enter is ignored, as in Radix. The indicator and the
thumb carry the same `data-state`. Radix's hidden form input is out of scope
until forms (B2).

## Affected tracks

The components track builds the primitives (C2) and the shadcn components on
them (C3); C1 maps their `data-*` attributes to typed-rule conditions. The
framework and engine tracks are asked for the following. Each owning track
designs its API; the names here only state the need. Until a request lands,
the primitives use the interim shown.

| # | Need | Track | Interim |
| --- | --- | --- | --- |
| 1 | A context API: `provide_context` / `use_context` scoped to the reactive owner, so parts find their root | framework | the state handle as a prop |
| 2 | Key names on keyboard events (`Key::ArrowRight`, `Home`, `End`, `PageUp`, `PageDown`, `Escape`, `Enter`, `Space`, `Tab`) and `repeat`, as the DOM's `KeyboardEvent.key`; today a listener sees `key_code` and `key_text` | framework (events) | a table from `key_code`, tested on Linux and Windows |
| 3 | `view!` to fill omitted optional props, so `<TabsTrigger value="a">` needs no `disabled=false`. `#[component]` already accepts `#[prop(optional)]` (the props field becomes `Option<T>`), but `view!` builds the props as a struct literal of the attributes given, so an omitted prop is a missing field and a given one must be written `Some(..)` | framework (`openui-macros`) | every prop passed |
| 4 | A modal root stack (push and pop) for nested dialogs | engine | one modal dialog at a time |
| 5 | The top layer, with `inert` outside a modal (A3) | engine | the modal root and `position: fixed` at the end of `<body>` |
| 6 | Restyle on attribute change (A1), so `data-state` drives shadcn's variants | engine | components set inline styles from the state signals |
| 7 | `Element::contains(&Element)` | framework | a parent walk |

Already present and used as is: `tabindex` in sequential focus order
(negative values leave it), `focus`/`blur`, `focusin`/`focusout` with the
related element, `Document::focused_element`, `Document::advance_focus`,
`Document::set_modal_root`, capture-phase listeners (`Element::on_capture`)
and `Show`.

## Migration

1. This RFC merges first. It changes no code.
2. The components track adds `openui-primitives` to the `components/`
   workspace, one PR per group: roving focus and tabs; checkbox and switch;
   dismissable layer, focus scope and dialog. Each uses the interims above
   and brings its trace fixtures. `components/` is its own workspace on the
   public crate only, so `main` stays green between these PRs.
3. As each request lands in its owning track, a components PR replaces the
   interim. Its trace and pixel results must not get worse.

## Verification

**Behavior traces.** Each interactive fixture gets a script of steps:
a key press, a click at an element's center, or a focus change. The harness
runs it twice:

- on the shadcn reference page in pinned Chromium, through CDP
  (`Input.dispatchKeyEvent`, `Input.dispatchMouseEvent`);
- on the Open UI fixture, through the public `Document` dispatch API in a
  headless app.

After each step it records the focused element and, for every element in
the fixture, its `role`, `aria-*`, `data-*`, `tabindex` and whether it is
mounted. Elements are named by `data-slot` and position, so ids do not
matter. The two traces must be equal step by step; the first difference is
reported with both states.

The first scripts:

- tabs: Tab into the list, arrows in both directions with wrapping, Home
  and End, a disabled trigger skipped, automatic and manual activation, and
  a click;
- dialog: open with the trigger, focus inside, Tab and Shift+Tab cycling,
  Escape closing and focus back on the trigger, a click on the overlay;
- checkbox and switch: a click and Space.

**Pixels.** Every state a script reaches that shadcn can render statically
(the second tab active, the dialog open, a checked checkbox) is also a
reference page, compared pixel by pixel like the static components.

**Unit tests.** Controllable state, the item order, the key table and the
layer stack are tested in the crate.

## Out of scope for alpha.1

Popover, dropdown menu, select, tooltip, collapsible and a general portal
(alpha.2, with anchor positioning from A3); typeahead; exit animations;
Radix's hidden form inputs; and touch-specific behavior.
