# Native form association and parent queries

Open UI implements element behavior in Rust. Applications call public Rust
methods and handle events with Rust callbacks; C uses the same retained engine.
Open UI never executes JavaScript. Browser reference scripts run only in the
separate offline Chromium tooling.

## Public APIs

| Operation | Rust | C |
|---|---|---|
| Query the current associated form | `Element::associated_form()` | `oui_element_associated_form_v1` |
| Query the retained parent | `Element::parent()` | `oui_element_parent_v1` |

Form association supports input, button, select, textarea, fieldset and object
elements. Other native kinds return no associated form. Connected explicit
`form` attributes resolve the first matching nonempty ID, which must name a
form. Detached controls use their nearest form ancestor. The queries perform
no layout, mutation or event dispatch.

```rust
let document = Document::new(320, 200)?;
let form = Element::create(&document, "form")?;
form.set_attribute("id", "settings")?;
document.body().append_child(&form)?;

let input = Element::create(&document, "input")?;
input.set_attribute("type", "radio")?;
input.set_attribute("name", "choice")?;
input.set_attribute("form", "settings")?;
document.body().append_child(&input)?;
let associated = input.associated_form()?;
```

A returned Rust `Element` retains its document. C returns an independently
owned element alias, or NULL when the relation is absent. Destroy each C alias
with `oui_element_destroy`; its document must remain alive while querying or
using it. Calls require the document's owning thread and a writable output.
Outputs remain unchanged on errors. Engine borrows are released before C
aliases are registered, so native callbacks can use these queries.

## Shared behavior and verification

The engine tracks radio membership across form and name changes, duplicate ID
changes, tree moves, detachment, insertion and subtree cloning. Checked-state
transitions use that shared association, including the measured detached-tree
rules. There is no script interpreter or fixture-specific renderer path.

Clean commit `a401a8ac` was measured in a private qualification clone, then
adopted unchanged onto the umbrella branch. A complete source-byte comparison
proves the public checkout contains that exact commit and source. The original
receipts keep their private-checkout labels. New qualification commands in the
canonical public checkout and its hosted checks remain unqualified here.

- All 88 new form-owner scenarios match immutable Chromium observations in
  Rust, C and C++, with 264/264 new comparisons exact.
- All 396 behavior cases in each language match: 1,188/1,188 overall. Thirty
  process-repeat pairs match; the previous selection and control guards remain
  unchanged.
- The original all-targets command passes 8,628 tests, including the existing
  deep-nesting benchmark under its unchanged stack limit. Ordinary workspace
  tests pass 8,630 with 13 ignored; all 53 headless C tests pass.
- The C ABI now has 130 exports and the same 34 layouts. Existing exports and
  layouts remain intact; the two relation queries are append-only additions.
- The native appearance app matches 160/160 cases and 640/640 PNG comparisons
  for default light 13px controls on white, five scales and eight phases.
- Focused 640/640 and primitive 960/960 are exact. Original 21,338/22,924 and
  expanded 22,141/23,728 still fail full equality, with zero render errors.
  All 48,252 rows retain the previous native pixels and Chromium identities,
  with no losses or worsened differences. The whole renderer command exits 1.

Radio keyboard focus, event order and cancellation still need qualification.
Unmeasured form operations, complete native API coverage, retained compositor,
physical lab and release qualification remain open. A final-state fixture or
JavaScript classification never completes or waives a needed native API.

See the [machine-readable evidence](../renderer/generated/native-form-owner-v1.json),
[Rust consuming app](../../bindings/rust/openui/examples/native_form_owner.rs),
[C consumer](../../examples/c_v02/form_owner.c) and
[C++ consumer](../../examples/c_v02/form_owner.cc).
