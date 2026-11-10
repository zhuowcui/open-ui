# RFC 0001: Typed style rules

Status: active from merge, 9 November 2026; amend by PR. Owner: engine track
(Workstream A, unit A1). Consumers: framework track (`openui`, `view!`) and
components track (Tailwind bridge C1, primitives C2, shadcn components C3).

This RFC fixes the data model and the public surface that the other tracks
build against. It implements plan decision D1: CSS syntax, compiled to typed
values; the runtime evaluates typed conditions only. Details that do not change
the surface (internal indexes, field layouts) may change during A1.

## What exists today

- `view!` validates `style:prop="value"` at compile time, then calls
  `parse_literal` again at runtime (`openui-macros/src/view.rs`).
- Inline declarations are `Style` / `StyleValue` lists, stored per element as
  `AuthoredStyleDeclarations` and resolved by the engine's native cascade
  (`openui-engine/src/authored_source.rs`) over the element's native defaults.
- Classes and attributes are strings used for lookup only. Nothing matches
  selectors.
- The engine tracks hover paths per pointer, `:active` targets, focus and
  focus-visible, and disabled state (`openui-engine/src/interaction.rs`). The
  public `Document` already dispatches target-only `pointerenter` and
  `pointerleave`. Nothing restyles when this state changes.

## Data model (`openui-style`, new `rules` module)

```rust
pub struct StyleSheet { rules: &'static [StyleRule], layers: &'static [LayerName],
                        index: InvalidationIndex, atoms: AtomTable }
pub struct StyleRule  { selector: Selector, specificity: Specificity,
                        layer: LayerOrder, media: Option<MediaQueryId>,
                        declarations: Style /* typed, each with `important` */ }
pub struct Selector   { compounds: Vec<(Compound, Combinator)> } // right to left
pub struct Compound   { tag: Option<ElementTag>, classes: Vec<ClassId>,
                        attributes: Vec<AttributeCondition>,
                        states: StateSet, logical: Vec<Logical> }
pub enum Combinator   { Descendant, Child, NextSibling, SubsequentSibling }
pub enum Logical      { Is(Vec<Selector>), Where(Vec<Selector>),
                        Not(Vec<Selector>), Has(Vec<RelativeSelector>) }
pub enum AttributeCondition { Present(AttrId), Equals(AttrId, AtomId),
                        /* ~=, |=, ^=, $=, *= over atoms or static text */ }
pub enum MediaFeature { Width(Range<Length>), Height(Range<Length>),
                        PrefersColorScheme(ColorScheme), PrefersReducedMotion(bool),
                        Hover(HoverCapability), Pointer(PointerAccuracy),
                        ForcedColors(bool) }
```

`StateSet` is a bit set of the pseudo-classes Chromium 147 evaluates from
element state: `:hover`, `:active`, `:focus`, `:focus-visible`,
`:focus-within`, `:disabled`, `:enabled`, `:checked`, `:indeterminate`,
`:placeholder-shown`, `:read-only`, `:read-write`, `:required`, `:invalid`,
`:open`, `:modal`, plus the structural ones (`:first-child`, `:last-child`,
`:only-child`, `:nth-child(An+B)`, `:empty`, `:root`). An unsupported
pseudo-class is a compile error, never a silent non-match.

`ClassId`, `AttrId` and `AtomId` are interned at compile time. The typed
conditions in the plan (`Hover`, `Data(State::Open)`, `Media(MinWidth(768))`)
are the constructors of these types. Sheets are `'static` and built once
(`LazyLock`) from constructor expressions; nothing is parsed at runtime.

## Macros (`openui-macros`, style parsing owned by the engine track)

| Macro | Input | Output |
| --- | --- | --- |
| `style! { padding: 8px 16px; color: red }` | one declaration block | `Style`, typed values constructed directly |
| `css! { ...; &:hover { ... } @media (...) { ... } }` | a block with CSS nesting | `ScopedStyle`: rules whose subject is one generated class, applied with `element.apply_scoped(&S)` |
| `stylesheet!(name, "path.css")` or `stylesheet!(name { ... })` | full CSS: rules, `@layer`, `@media`, nesting | `static name: StyleSheet` and `mod name::classes` |

All three share one CSS tokenizer and parser in `openui-style` (macro-time
only). The parser reuses the existing per-property schema, so the set of
accepted values equals what `parse_literal` accepts today. Errors carry the
source span:

- unknown property, invalid value or unit, unsupported selector or at-rule:
  compile error;
- a duplicate longhand in one block, or a shorthand after a longhand it
  resets: compile warning (`#[deny]` turns it into an error);
- `view!` `style:` attributes emit typed constructors instead of
  `parse_literal`, removing the runtime re-parse.

**Class names.** `stylesheet!` emits one constant per class in
`name::classes`, with a stable mangling for names that are not identifiers
(`hover:bg-muted`, `data-[state=open]:p-2`). `view!` resolves each token of a
static `class="..."` to a path in the module brought into scope with
`use_classes!(path::to::name)`; an unknown class is an unresolved path, so the
build fails and the message names the class. Dynamic classes use the typed
constants: `class:open=move || is_open.get()`, `element.add_class(classes::open)`.
The existing string `set_class` keeps working for lookup; a string class that no
sheet declares matches nothing.

**Attributes and data state.** Each sheet lists the attribute names its
selectors read. Setting a watched attribute (`set_attribute("data-state",
"open")` or the typed `element.set_data(attr::data_state, atom::open)`) stores
its atom, so matching is integer comparison. Values no sheet mentions are kept
as text and compared only by the substring operators.

## Cascade, specificity and inheritance

Rules apply in Chromium 147 order, encoded in one sortable priority like Blink's
`CascadePriority` (`core/css/resolver/cascade_priority.h`):

1. origin and importance: transitions; important UA; important author;
   animations; normal author; UA (the engine's native defaults);
2. element-attached styles: `style:` and `set_property` declarations beat every
   selector rule of the same origin and importance;
3. cascade layers: for normal declarations unlayered beats later layers beats
   earlier layers; for `!important` the order reverses;
4. specificity, computed by the macro exactly as Blink's
   `CSSSelector::Specificity()` (`:where()` is zero; `:is()`, `:not()` and
   `:has()` take their most specific argument; nesting `&` behaves as `:is()`),
   including its packed form and overflow behavior;
5. order of appearance: sheet adoption order, then rule order in a sheet.

Inheritance stays as today: inherited properties start from the parent's
computed style before declarations apply. CSS-wide keywords (`inherit`,
`initial`, `unset`, `revert`, `revert-layer`) become a typed declaration value.
`var()` is reserved: a declaration value may be a typed token expression, and
token values cascade and inherit per element like custom properties. Tailwind
v4 composes values through internal variables (`--tw-ring-shadow`,
`--tw-shadow` and others), so token expressions must allow several token slots
and fallbacks in one value. RFC 0002 (A2) specifies tokens; this RFC only fixes
where they sit.

With no sheet adopted, resolution is exactly today's path: the census and
coverage fixtures use no sheets, so they cannot change (D4).

## Invalidation and restyle

Each sheet carries an invalidation index built at compile time, after Blink's
rule invalidation data (`core/css/invalidation/`): for every class, attribute
and state bit, whether it occurs in a rule's subject compound (restyle the
element), in an ancestor compound (restyle matching descendants), in a sibling
compound (restyle later siblings and their subtrees), or inside `:has()`
(restyle the anchoring ancestors). On a change the engine:

1. looks up the changed feature and collects candidate elements;
2. re-matches only those elements and recomputes their cascaded declarations;
3. re-resolves them through the existing native cascade, which already
   propagates inherited values to descendants;
4. compares old and new computed values and marks the strongest invalidation
   class of the properties that changed, so a hover color change is paint only.

Media changes (viewport width, color scheme, reduced motion) re-evaluate
each media query once and restyle under the rules whose result flipped. Tree
insertion and removal restyle the inserted subtree, siblings affected by
structural pseudo-classes, and `:has()` anchors.

State sources, matching Chromium:

- `:hover` is the hit-test path of each pointer, including the control of a
  hovered `<label>`; `:active` is the pressed target and its ancestors.
- After a scroll or layout change the engine re-hit-tests the last pointer
  position on the next `update()`, as Chromium's synthetic mouse move does,
  and emits `pointerenter` / `pointerleave` for the changed path.
- When the pointer leaves the window, every node on its path receives
  `pointerleave` and loses `:hover`. Today `PointerPhase::Leave` maps to
  `Cancel`, which clears hover without these events; A1 fixes this.
- `:focus-visible` follows Chromium's heuristic: keyboard focus matches;
  pointer focus matches only for elements that take text input.

## Public API for the other tracks

Exported from `openui` (the engine track adds these thin wrappers and
re-exports in A1 PRs under this RFC; the framework track owns their later
shape and documentation):

- `style!`, `css!`, `stylesheet!`, `use_classes!`; types `Style`,
  `ScopedStyle`, `StyleSheet`, `ClassId`, `AttrId`, `AtomId`, `ColorScheme`.
- `Document::adopt_stylesheet(&'static StyleSheet) -> SheetHandle`,
  `Document::remove_stylesheet(SheetHandle)`,
  `Document::set_color_scheme(ColorScheme)`.
- `Element::add_class`, `remove_class`, `toggle_class` and `set_classes` with
  `ClassId`; `Element::set_data(AttrId, AtomId)`; `Element::apply_scoped`.
- `view!`: static `class="..."` is checked at compile time; `class:name=signal`
  toggles a class; `style:` stays element-attached.
- Events: `on:pointerenter` and `on:pointerleave` keep DOM semantics (target
  only, no bubbling), with the fixes above.

The Tailwind bridge (C1) produces a `StyleSheet` through the same compiler, so
utility classes reach `view!` through `use_classes!` like any other sheet.

## Verification

- Unit tests for the parser, specificity (against values Blink reports), layer
  order and invalidation sets.
- A new cascade qualification suite: each case is one HTML file and one CSS
  file. Chromium 147 loads both; the native fixture compiles the same CSS file
  with `stylesheet!` and builds the same tree. The suite compares pixels at the
  census profiles and a dump of computed values (`getComputedStyle` against
  `Element::computed_style`), in static states and after scripted pointer,
  focus and attribute changes driven through CDP and the native public API.
- Restyle scope tests assert which elements were restyled and which
  invalidation class was marked.
- Every A1 PR runs the full census and coverage nonregression before merge.

## Out of scope for alpha.1

Shadow DOM and `::part`, `@container`, `@scope`, `@supports` beyond constant
folding at compile time, runtime CSS text, and live reload of sheets (alpha.2).
Pseudo-elements other than those the engine already styles (`::before`,
`::after`, `::marker`, `::first-line`, `::first-letter`, `::placeholder`) are
compile errors until they are implemented; shadcn's `selection:` and `file:`
variants need `::selection` and `::file-selector-button` later.
