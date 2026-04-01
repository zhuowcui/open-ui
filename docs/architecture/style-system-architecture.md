# Style System Architecture

> Deep dive into Blink's style system internals and the extraction plan for Open UI's `libopenui_style`.
>
> **Chromium version:** M147 (`147.0.7727.24`)
> **Blink source root:** `third_party/blink/renderer/`

---

## 1. Style Resolution Pipeline

Style resolution transforms raw property declarations into final computed values that layout and paint can consume. Blink models this as a multi-stage pipeline defined by the CSS specification.

### Entry Point

```
third_party/blink/renderer/core/css/resolver/style_resolver.h
    class StyleResolver
        → ResolveStyle(Element&, const StyleRecalcContext&)
        → returns scoped_refptr<const ComputedStyle>
```

`StyleResolver::ResolveStyle()` is the top-level entry. For a given element, it orchestrates the full pipeline:

1. Collect matching rules (`ElementRuleCollector`)
2. Run the cascade (`StyleCascade`)
3. Apply inheritance
4. Resolve values (relative → absolute, `var()` substitution, `calc()` evaluation)
5. Return an immutable `ComputedStyle` snapshot

### Pipeline Stages

```
Specified Value ─→ Cascaded Value ─→ Computed Value ─→ Used Value ─→ Actual Value
      │                  │                  │                │              │
  Declared in        Winner of          Absolutes       Layout-         Device-
  stylesheet or      the cascade        resolved;       dependent       pixel
  inline style       (specificity,      inheritance     values          rounded
                     origin, order)     applied         resolved
```

**Specified Value.** The raw declaration as authored: `font-size: 1.5em`, `width: 50%`, `color: inherit`. Comes from matched rules or inline styles. Represented in Blink as `CSSValue` subclasses (`CSSPrimitiveValue`, `CSSIdentifierValue`, `CSSCalcValue`, etc.) rooted at:

```
core/css/css_value.h            — base class
core/css/css_primitive_value.h  — numbers, lengths, percentages
core/css/css_identifier_value.h — keyword values (e.g., `auto`, `inherit`)
core/css/css_custom_property_declaration.h — custom property (--*) values
```

**Cascaded Value.** After the cascade resolves conflicts, each property has at most one winning declaration per element. The cascade considers (in order): origin, `@layer` position, specificity, source order, and `!important`. The `StyleCascade` class (`core/css/resolver/style_cascade.h`) produces this.

**Computed Value.** The cascaded value with all relative references resolved against the element's context:

- `em` → multiply by parent's computed `font-size`
- `rem` → multiply by root element's computed `font-size`
- `%` → resolve against containing block's dimension (width or height, depending on property)
- `inherit` → copy parent's computed value
- `initial` → property's initial value from the spec
- `var(--foo)` → substitute the custom property value, then re-parse
- `calc(1em + 10px)` → evaluate expression to a single value
- `currentColor` → resolve to the computed value of `color`

Computation happens in per-property `ComputeValue()` methods generated from `css_properties.json5`. The output is stored in `ComputedStyle` (section 3).

**Used Value.** Some computed values cannot be fully resolved without layout. For example, `width: 50%` needs the containing block's width, which is only known after layout. The used value is the computed value after layout-dependent resolution. In Blink, used values are computed lazily during layout and stored on `LayoutObject`s, not in `ComputedStyle`.

Key used-value properties:
- `width`, `height` (when specified as `auto` or `%`)
- `margin: auto` (horizontal centering, flex distribution)
- `top`, `left` etc. for positioned elements with `auto` or `%` values
- Line heights in inline formatting contexts

**Actual Value.** The used value rounded to device pixels. Blink's `LayoutUnit` internally uses fixed-point arithmetic (1/64th pixel precision) and snaps to physical pixels at paint time. This is handled transparently by the layout and paint layers.

---

## 2. The Cascade

The cascade is the algorithm that resolves conflicting declarations for the same property on the same element. Blink's modern implementation lives in `StyleCascade`.

### Core Class

```
core/css/resolver/style_cascade.h
    class StyleCascade
        → Apply(CascadeFilter&)
        → AnalyzeMatchResult(...)
```

`StyleCascade` replaced the older `StyleResolver::ApplyMatchedProperties()` approach. It operates on a flat list of `CascadePriority`-tagged declarations and resolves them in a single pass.

### Cascade Sorting Order

The cascade sorts declarations by the following criteria, from highest to lowest priority:

```
1. Origin & Importance
   ┌─────────────────────────────────────────────────────────────────┐
   │  (highest) Transition declarations                              │
   │            User-agent !important                                │
   │            User !important                                      │
   │            Author !important (reverse layer order)              │
   │            Animation declarations                               │
   │            Author normal (layer order)                          │
   │            User normal                                          │
   │  (lowest)  User-agent normal                                    │
   └─────────────────────────────────────────────────────────────────┘

2. Specificity   (within same origin/importance)
3. Source order   (within same specificity; last declaration wins)
```

### Origin Ordering

Blink defines three origins in `core/css/resolver/cascade_origin.h`:

| Origin | Source | Typical Use |
|--------|--------|-------------|
| **User-Agent** | Browser's default stylesheet (`html.css`, `quirks.css`) | Default `display: block` for `<div>`, etc. |
| **User** | User preferences (high contrast, minimum font size) | Accessibility overrides |
| **Author** | Page stylesheets, inline styles | Application styling |

For normal (non-`!important`) declarations, author styles override user styles, which override user-agent styles. For `!important` declarations, this order is **reversed** — user-agent `!important` beats author `!important`.

### Specificity Calculation

Specificity is a three-component vector `(A, B, C)` computed from the selector:

```
A = count of ID selectors            (#id)
B = count of class/attribute/pseudo-class selectors  (.class, [attr], :hover)
C = count of type/pseudo-element selectors           (div, ::before)
```

Implementation:

```
core/css/css_selector.h
    CSSSelector::Specificity() → returns unsigned (packed A,B,C)

Packed format: (A << 16) | (B << 8) | C
    — allows direct integer comparison
```

Inline styles (`style="..."`) bypass specificity; they always win within their origin (unless overridden by `!important` from a higher origin).

### `!important` Handling

When a declaration is marked `!important`, it moves to the "important" half of its origin. The cascade processes important declarations in reverse origin order:

```
Normal cascade:    UA-normal < User-normal < Author-normal
Important cascade: Author-important < User-important < UA-important
```

Within the important sub-cascade, specificity and source order still apply. This means an author `!important` declaration beats all author normal declarations, but loses to a user `!important` declaration.

In `StyleCascade`, importance is encoded in `CascadePriority`:

```
core/css/resolver/cascade_priority.h
    CascadePriority encodes: origin + importance + tree_order + position
```

### `@layer` Cascade Layers

CSS `@layer` introduces named layers within an origin. Layers are ordered by their first declaration:

```css
@layer reset;      /* layer 0 — lowest priority */
@layer base;       /* layer 1 */
@layer components; /* layer 2 */
@layer overrides;  /* layer 3 — highest priority */
```

Declarations in later layers beat declarations in earlier layers (for normal), but the order reverses for `!important` — mirroring the origin reversal.

Blink tracks layers in:

```
core/css/cascade_layer.h
    CascadeLayer — tree structure of named layers
core/css/cascade_layer_map.h
    CascadeLayerMap — maps layer names to their canonical order
```

The layer order is folded into `CascadePriority` so that the single-pass cascade resolution handles layers transparently.

### Cascade Algorithm (Simplified)

```
for each property P on element E:
    candidates = []
    for each matched rule R (in source order):
        if R declares P:
            candidates.append((R.origin, R.layer, R.importance, R.specificity, R.order, R.value))

    sort candidates by (origin+importance, layer, specificity, order)
    cascaded_value[P] = candidates.last().value
```

The actual implementation is more efficient — `StyleCascade` avoids sorting by processing declarations in priority order using a priority-tagged expansion.

---

## 3. ComputedStyle — The Core Data Structure

`ComputedStyle` is the immutable, deduplicated snapshot of all computed CSS property values for a single element. It is the primary output of style resolution and the primary input to layout.

### Location

```
core/style/computed_style.h           — main class
core/style/computed_style_base.h      — generated base with field accessors
core/style/computed_style_builder.h   — mutable builder for construction
```

### Memory Layout

`ComputedStyle` is heavily optimized for memory. With millions of DOM nodes in complex pages, every byte matters. The layout uses three strategies:

**1. Bitfield packing for enums and small values.**

Common properties like `display`, `position`, `overflow`, `visibility` are stored as small bitfields packed together. Generated code in `computed_style_base.h` handles the bit manipulation:

```cpp
// Generated — example layout (conceptual)
struct ComputedStyleBase {
    unsigned display_ : 5;           // EDisplay enum, 20 values → 5 bits
    unsigned position_ : 3;          // EPosition enum, 6 values → 3 bits
    unsigned float_ : 2;             // EFloat enum, 3 values → 2 bits
    unsigned overflow_x_ : 3;        // EOverflow enum
    unsigned overflow_y_ : 3;
    unsigned visibility_ : 2;
    unsigned white_space_ : 3;
    unsigned text_align_ : 4;
    // ... packed into minimal bits
};
```

**2. Field groups for related properties.**

Properties are organized into groups that tend to be set together. Each group is a separate heap-allocated, ref-counted object. If a property group is identical to the parent's, the child shares the parent's pointer — no copy needed.

```
core/style/style_rare_non_inherited_data.h  — non-inherited properties that are rarely set
core/style/style_rare_inherited_data.h      — inherited properties that are rarely set
core/style/style_box_data.h                 — width, height, min/max sizing
core/style/style_surround_data.h            — margin, padding, border
core/style/style_visual_data.h              — clip, zoom
core/style/style_background_data.h          — background layers
core/style/style_inherited_data.h           — font, color, line-height
```

The grouping is defined in:

```
core/style/computed_style_field_group.json5
```

At access time, a "rare" group pointer is null or shared until a property in that group is actually written, at which point copy-on-write allocates a new group.

**3. Rare data separation.**

Most elements don't set properties like `clip-path`, `transform-origin`, or `scroll-snap-type`. These live in "rare" data groups that are only allocated when needed. The `ComputedStyle` itself only holds a pointer (8 bytes) to each rare group — null if unused.

### Conceptual Memory Map

```
ComputedStyle (main object, ~128 bytes on 64-bit)
├── Bitfields: display, position, float, overflow, visibility, ...
├── InheritedData* ──────→ [font, color, line_height, text_indent, ...]
├── BoxData* ────────────→ [width, height, min_width, max_width, ...]
├── SurroundData* ───────→ [margin, padding, border_width, border_style, ...]
├── BackgroundData* ─────→ [background_color, background_layers, ...]
├── VisualData* ─────────→ [clip, zoom, ...]
├── RareNonInheritedData* → [transform, filter, opacity, clip_path, ...]  (often null)
├── RareInheritedData* ──→ [text_shadow, word_spacing, tab_size, ...]     (often null)
└── ref_count, flags, cached_pseudo_styles, ...
```

### Style Sharing (Deduplication)

When two elements resolve to identical computed styles, Blink shares a single `ComputedStyle` object between them. This is critical for large lists, tables, and repeated components.

```
core/css/resolver/style_sharing_candidate.h
    MatchedPropertiesCache — caches style results keyed by matched declarations
```

The sharing check compares:
1. The set of matched rules (not just the final values)
2. Inherited properties from the parent
3. Any pseudo-class state that might differ

If the cache hits, `ComputedStyle::Create()` returns a shared reference instead of allocating a new object. On typical pages, style sharing reduces memory usage by 50-70%.

```
core/style/computed_style.h
    static scoped_refptr<ComputedStyle> Create()
    static scoped_refptr<ComputedStyle> Clone(const ComputedStyle&)
```

### Immutability and Threading

Once created, a `ComputedStyle` is immutable. This is not just a design preference — it is a correctness requirement:

- **Layout reads style concurrently.** During layout, the layout tree reads `ComputedStyle` properties from many nodes. If styles could mutate during layout, data races would occur.
- **Style sharing relies on immutability.** Multiple elements hold references to the same `ComputedStyle`. Mutation would corrupt shared state.
- **Incremental style relies on diffing.** Old vs. new `ComputedStyle` are compared to determine what changed (`StyleDifference`). This requires the old style to be stable.

Construction uses the builder pattern:

```cpp
ComputedStyleBuilder builder(parent_style);
builder.SetDisplay(EDisplay::kFlex);
builder.SetColor(Color(0, 0, 0));
// ... set all properties
scoped_refptr<const ComputedStyle> style = builder.TakeStyle();
// style is now immutable
```

`ComputedStyleBuilder` (`core/style/computed_style_builder.h`) provides mutable setters. Once `TakeStyle()` is called, the builder transfers ownership and the resulting `ComputedStyle` is frozen.

---

## 4. Property System

Blink's CSS property system is largely generated from a machine-readable registry. This keeps the ~600 CSS properties consistent and reduces boilerplate.

### The Property Registry

```
core/css/css_properties.json5
```

This JSON5 file defines every CSS property in Blink. Each entry specifies:

```json5
{
    name: "margin-top",
    property_class: "Longhand",
    field_group: "surround",
    field_template: "external",        // stored in a field group
    type_name: "Length",
    initial: "Length::Fixed(0)",
    inherited: false,
    computed_style_custom_functions: ["parse", "value"],
    keywords: ["auto"],
    affected_by: "layout",             // affects layout, not just paint
    // ...
}
```

Key attributes per property:

| Attribute | Description |
|-----------|-------------|
| `name` | CSS property name (`margin-top`, `color`, `display`) |
| `property_class` | `Longhand` (single property) or `Shorthand` (expands to longhands) |
| `inherited` | Whether the property inherits from parent (e.g., `color`: yes, `width`: no) |
| `initial` | The initial (default) value per CSS specification |
| `field_group` | Which `ComputedStyle` field group stores it |
| `type_name` | C++ type for the computed value (`Length`, `Color`, `EDisplay`, etc.) |
| `affected_by` | What pipeline stage it affects: `layout`, `paint`, `composite` |
| `keywords` | Accepted keyword values (`auto`, `none`, `inherit`, etc.) |

### Generated Code

The build system processes `css_properties.json5` through Python generators to produce:

```
out/gen/third_party/blink/renderer/core/css/properties/
├── longhands/
│   ├── margin_top.h / .cc
│   ├── color.h / .cc
│   ├── display.h / .cc
│   └── ... (one file per longhand property)
├── shorthands/
│   ├── margin.h / .cc
│   ├── flex.h / .cc
│   └── ... (one file per shorthand)
├── css_property_ref.h              — unified property reference
├── css_property_instances.h        — singleton instances
└── css_unresolved_property.h       — property ID enum
```

Each generated longhand class provides:

```cpp
class MarginTop final : public Longhand {
 public:
    const CSSValue* ParseSingleValue(CSSParserTokenStream&, const CSSParserContext&) const override;
    const CSSValue* CSSValueFromComputedStyleInternal(
        const ComputedStyle&, const LayoutObject*, bool allow_visited_style) const override;
    void ApplyInitial(StyleResolverState&) const override;
    void ApplyInherit(StyleResolverState&) const override;
    void ApplyValue(StyleResolverState&, const CSSValue&, ValueMode) const override;
};
```

`CSSPropertyRef` provides a unified handle to look up any property by ID:

```
core/css/properties/css_property_ref.h
    CSSPropertyRef — resolves CSSPropertyID or custom property name to a CSSProperty*
```

### Property Categories

**Inherited vs. Non-inherited:**

| Inherited | Non-inherited |
|-----------|---------------|
| `color`, `font-*`, `line-height`, `text-align`, `visibility`, `cursor`, `direction`, `white-space`, `word-spacing`, `letter-spacing` | `width`, `height`, `margin-*`, `padding-*`, `border-*`, `display`, `position`, `flex-*`, `grid-*`, `background-*`, `transform`, `opacity`, `overflow` |

Inherited properties default to the parent's computed value; non-inherited properties default to their initial value.

**Affects-layout vs. Affects-paint vs. Affects-composite:**

| Category | Properties (examples) | Effect |
|----------|-----------------------|--------|
| **Layout** | `width`, `height`, `margin`, `padding`, `display`, `flex-*`, `grid-*`, `font-size`, `position` | Triggers full layout recalculation |
| **Paint** | `color`, `background-color`, `border-color`, `box-shadow`, `text-decoration`, `visibility` | Triggers repaint only (no re-layout) |
| **Composite** | `transform`, `opacity`, `filter`, `will-change` | Handled on compositor thread, cheapest |

This classification drives `StyleDifference` (section 8).

### Shorthand Expansion

Shorthands are syntactic sugar that expand to multiple longhand properties:

```
margin: 10px 20px
    → margin-top: 10px
    → margin-right: 20px
    → margin-bottom: 10px
    → margin-left: 20px

flex: 1 0 auto
    → flex-grow: 1
    → flex-shrink: 0
    → flex-basis: auto

border: 1px solid black
    → border-top-width: 1px      border-top-style: solid      border-top-color: black
    → border-right-width: 1px    border-right-style: solid     border-right-color: black
    → border-bottom-width: 1px   border-bottom-style: solid    border-bottom-color: black
    → border-left-width: 1px     border-left-style: solid      border-left-color: black
```

Shorthands are expanded at parse time. The cascade and `ComputedStyle` only deal with longhands. Shorthand classes live in:

```
core/css/properties/shorthands/
    margin.h    — expands to margin-{top,right,bottom,left}
    padding.h   — expands to padding-{top,right,bottom,left}
    flex.h      — expands to flex-grow, flex-shrink, flex-basis
    border.h    — expands to 12 longhands (width × 4, style × 4, color × 4)
    // ...
```

### Custom Properties (CSS Variables)

Custom properties (`--*`) are handled differently from standard properties:

```
core/css/css_custom_property_declaration.h  — stores a custom property's value
core/css/css_variable_data.h                — tokenized variable data for substitution
core/css/css_variable_resolver.h            — resolves var() references
```

**Declaration.** Custom properties are stored as raw token sequences, not parsed values:

```css
--spacing: calc(8px * 2);  /* stored as token stream, not "16px" */
```

**Substitution.** `var()` references are resolved during value computation:

```css
padding: var(--spacing);
/* → substitute token stream of --spacing */
/* → parse "calc(8px * 2)" as a length */
/* → compute to 16px */
```

**Inheritance.** Custom properties are always inherited (unless registered with `@property` and `inherits: false`). The `CSSVariableResolver` walks the token stream, substitutes all `var()` references (including nested ones), detects cycles, and falls back to the property's initial value on failure.

**Registered custom properties** (`@property`) can declare a syntax, initial value, and inheritance:

```
core/css/css_property_registration.h
    PropertyRegistration — syntax, initial value, inherits flag
```

This allows Blink to type-check and animate registered custom properties.

---

## 5. Inheritance

CSS inheritance flows computed values from parent to child for properties marked as inherited.

### How Inheritance Works

During style resolution, for each property on an element:

```
if property has a cascaded value:
    use cascaded value (resolved to computed)
elif property is inherited:
    use parent's computed value
else:
    use property's initial value
```

This is implemented in the per-property `Apply*` methods:

```cpp
// Generated for each property:
void Color::ApplyInherit(StyleResolverState& state) const {
    state.StyleBuilder().SetColor(state.ParentStyle()->Color());
}

void Color::ApplyInitial(StyleResolverState& state) const {
    state.StyleBuilder().SetColor(Color::Black());  // initial value per spec
}
```

### CSS-Wide Keywords

Four keywords can be used with any property:

| Keyword | Behavior |
|---------|----------|
| `inherit` | Use parent's computed value, even for non-inherited properties |
| `initial` | Use the property's CSS-defined initial value |
| `unset` | Acts as `inherit` for inherited properties, `initial` for non-inherited |
| `revert` | Rolls back to the previous cascade origin's value |

`revert` is the most complex — it requires re-running the cascade without the current origin:

```
core/css/resolver/style_cascade.h
    StyleCascade handles revert by tracking per-origin cascaded values
    and falling back to the previous origin when revert is encountered
```

### Inheritance Optimization: Shared Parent Groups

Blink avoids copying inherited values individually for each child. Instead, `ComputedStyle` field groups that contain only inherited properties can be shared by pointer between parent and child:

```
Parent ComputedStyle
├── InheritedData* ──→ [font: 16px Arial, color: #333, line-height: 1.5]

Child ComputedStyle (inherits all, overrides none)
├── InheritedData* ──→ (same pointer as parent — no allocation)

Child ComputedStyle (overrides color)
├── InheritedData* ──→ [font: 16px Arial, color: #f00, line-height: 1.5]  (new copy)
```

This copy-on-write strategy is managed by the ref-counted field groups. A child starts with the parent's group pointers and only allocates a new group when it needs to modify an inherited value. For a typical page where most children inherit all text properties, this saves enormous memory.

---

## 6. Value Resolution

Value resolution converts specified/cascaded values into fully computed absolute values.

### Relative Units

All relative length units are resolved against known reference values:

| Unit | Resolution |
|------|------------|
| `em` | Multiply by the element's own computed `font-size` (or parent's, for the `font-size` property itself) |
| `rem` | Multiply by the root element's computed `font-size` |
| `%` | Multiply by the containing block's dimension. Which dimension depends on the property (e.g., `width: 50%` → 50% of containing block width, `padding-top: 10%` → 10% of containing block **width**) |
| `vh` / `vw` | 1% of viewport height / width |
| `vmin` / `vmax` | 1% of the smaller / larger viewport dimension |
| `ch` | Width of the "0" glyph in the element's font |
| `ex` | x-height of the element's font |

Resolution happens in:

```
core/css/css_primitive_value.h
    CSSPrimitiveValue::ComputeLength(const CSSLengthResolver&)

core/css/css_length_resolver.h
    CSSLengthResolver — provides the context: font size, viewport size, etc.
```

### `calc()` Expressions

`calc()` expressions can mix units and operators:

```css
width: calc(100% - 2 * 20px);
font-size: calc(1rem + 0.5vw);
```

Blink represents `calc()` as an expression tree:

```
core/css/css_math_function_value.h
    CSSMathFunctionValue — root of a calc() expression
core/css/css_math_expression_node.h
    CSSMathExpressionNode — tree nodes: literals, binary ops (+, -, *, /), functions (min, max, clamp)
```

Resolution evaluates the expression tree bottom-up:

1. Resolve each leaf to a canonical unit (`px` for lengths, plain number for ratios)
2. Perform arithmetic operations
3. Clamp to valid ranges (e.g., lengths cannot be negative for `width`)

`min()`, `max()`, and `clamp()` are modeled as `CSSMathExpressionOperation` nodes with specialized evaluation.

### `currentColor` Resolution

`currentColor` is a special keyword that resolves to the computed value of the `color` property on the same element:

```css
border-color: currentColor;  /* uses the element's computed color */
```

Resolution order matters: `color` must be computed before any property that references `currentColor`. Blink handles this by processing `color` early in the property application order (controlled by property priority in `css_properties.json5`).

```
core/css/properties/longhands/color.h
    — always resolved before properties that may reference currentColor
```

### `env()` Environment Variables

`env()` provides access to user-agent-defined values, such as safe area insets on devices with notches:

```css
padding-top: env(safe-area-inset-top, 0px);
```

```
core/css/css_environment_variables.h
    CSSEnvironmentVariables — key-value store of env() variables
```

Resolution substitutes the variable value at computed-value time, with a fallback if the variable is not defined.

### Computed Value vs. Used Value

Some properties cannot be fully resolved during style computation because they depend on layout geometry:

| Property | Computed Value | Used Value (after layout) |
|----------|---------------|---------------------------|
| `width: 50%` | `50%` (percentage preserved) | `400px` (50% of 800px container) |
| `width: auto` | `auto` | `600px` (determined by layout algorithm) |
| `margin: auto` | `auto` | `50px` (auto-distribution of remaining space) |
| `top: 50%` | `50%` | `300px` (50% of containing block height) |

The CSS spec defines which properties preserve percentages as computed values and which resolve them. `ComputedStyle` stores the computed value; the layout engine resolves to used values during layout.

In Blink:

```
core/layout/layout_box.h
    LayoutBox::ComputedCSSContentBoxRect()  — used values after layout
    LayoutBox::PhysicalPaddingBoxRect()     — resolved padding box
```

---

## 7. Style Invalidation

Style invalidation determines which elements need their styles re-resolved after a change. Minimizing unnecessary work is critical for performance.

### What Triggers Re-Style

| Trigger | Example | Scope |
|---------|---------|-------|
| DOM mutation | `appendChild()`, `removeChild()` | Inserted/removed subtree + siblings (for `:nth-child`) |
| Class change | `element.classList.add('active')` | Element + descendants (if rules use `.active` in ancestor selectors) |
| Attribute change | `element.setAttribute('disabled', '')` | Element + descendants |
| Pseudo-class change | `:hover`, `:focus`, `:checked` | Element + descendants |
| Style rule mutation | `sheet.insertRule(...)` | Potentially global (depending on selector) |
| Custom property change | `element.style.setProperty('--color', 'red')` | Element + all descendants that reference the variable |
| Media query match | Viewport resize, prefers-color-scheme change | Global (all rules in affected `@media` blocks) |

### Invalidation Sets

To avoid re-styling the entire document on every class change, Blink pre-computes **invalidation sets** — compact data structures that record which selectors *might* be affected by a given change.

```
core/css/invalidation/invalidation_set.h
    InvalidationSet — set of classes/IDs/tag-names/attributes that a selector depends on
core/css/invalidation/style_invalidator.h
    StyleInvalidator — walks the tree applying invalidation sets
core/css/rule_set.h
    RuleSet::CollectInvalidationSetsForClass/Id/Attribute/PseudoClass()
```

When a stylesheet is parsed, Blink analyzes each selector to build invalidation sets:

```css
/* Selector: .sidebar .item:hover */
→ Class "sidebar" invalidates descendants (class "item")
→ Class "item" invalidates self (pseudo-class :hover may change)
→ Pseudo-class :hover on class "item" invalidates self
```

When `classList.add('sidebar')` is called on an element, Blink looks up the invalidation set for class `sidebar` and marks only those descendants that match class `item` as needing re-style.

### Marking Dirty

```
core/css/style_engine.h
    StyleEngine — central coordinator for style operations
    StyleEngine::MarkStyleDirty(Node&) — marks a node for re-style
    StyleEngine::MarkAllElementsForStyleRecalc() — nuclear option
```

Dirty flags on `Node`:

```
core/dom/node.h
    Node::SetNeedsStyleRecalc(StyleChangeType)

    StyleChangeType:
        kNoStyleChange
        kLocalStyleChange      — only this element's style changed
        kSubtreeStyleChange    — this element + all descendants need re-style
        kNeedsReattachStyleChange — layout tree needs to be rebuilt
```

### Scoped Invalidation

Blink avoids full-tree re-style by scoping invalidation:

1. **Self-invalidation:** Only the changed element is marked dirty (e.g., inline style change).
2. **Subtree invalidation:** The element and all its descendants (e.g., inherited custom property changed).
3. **Sibling invalidation:** Adjacent siblings (e.g., `+` or `~` combinators, `:nth-child`).
4. **Document-level invalidation:** Entire tree (e.g., new stylesheet added). This is the most expensive and is avoided when possible.

After marking, `StyleEngine::RecalcStyle()` walks only the dirty subtree:

```
core/css/style_engine.h
    StyleEngine::RecalcStyle()
        → walks the tree top-down
        → skips clean subtrees (no dirty flag)
        → calls StyleResolver::ResolveStyle() for each dirty node
        → diffs old vs. new ComputedStyle → produces StyleDifference
```

---

## 8. Interaction with Layout

Style is the input to layout. The boundary between the two subsystems is well-defined: `ComputedStyle` is the interface.

### What Layout Reads from ComputedStyle

| Layout Phase | Properties Read |
|--------------|-----------------|
| **Box generation** | `display` (block, flex, grid, inline, none, contents) |
| **Positioning scheme** | `position` (static, relative, absolute, fixed, sticky) |
| **Sizing** | `width`, `height`, `min-width`, `max-width`, `min-height`, `max-height`, `box-sizing`, `aspect-ratio` |
| **Spacing** | `margin-*`, `padding-*`, `border-*-width` |
| **Flex layout** | `flex-direction`, `flex-wrap`, `flex-grow`, `flex-shrink`, `flex-basis`, `align-items`, `justify-content`, `align-self`, `order`, `gap` |
| **Grid layout** | `grid-template-*`, `grid-auto-*`, `grid-column-*`, `grid-row-*`, `gap` |
| **Inline/text** | `font-size`, `font-family`, `line-height`, `text-align`, `white-space`, `word-break`, `letter-spacing` |
| **Overflow** | `overflow-x`, `overflow-y` |
| **Float/clear** | `float`, `clear` |
| **Z-ordering** | `z-index` |

### Style Change Classification: `StyleDifference`

When a style re-resolve produces a new `ComputedStyle`, Blink diffs it against the old one to determine the minimal work needed:

```
core/style/computed_style.h
    ComputedStyle::VisualInvalidationDiff(const ComputedStyle& old, const ComputedStyle& new)
        → returns StyleDifference

core/layout/layout_object.h
    StyleDifference {
        needs_layout: bool,           // geometry changed (width, margin, flex-*, etc.)
        needs_paint_invalidation: bool,  // visual changed (color, background, border-color)
        needs_recomposite: bool,      // compositor properties changed (transform, opacity)
        needs_reshape: bool,          // text shape changed (font-*, letter-spacing)
    }
```

This classification is performance-critical. The cost hierarchy:

```
Layout (most expensive)
  └── Reflow the affected subtree, recompute all geometry
Paint invalidation (moderate)
  └── Repaint affected layers, no geometry change
Compositor update (cheapest)
  └── Update compositor properties on the GPU, no main-thread work
```

### When Style Change Triggers Layout vs. Only Paint

**Triggers layout (re-flow):**
- Any sizing property: `width`, `height`, `min-*`, `max-*`
- Any spacing property: `margin-*`, `padding-*`, `border-*-width`
- Display type change: `display`
- Position scheme change: `position`
- Flex/grid property changes
- Font metrics changes: `font-size`, `font-family`, `line-height`

**Triggers paint only (no re-flow):**
- `color`, `background-color`, `border-color`
- `box-shadow`, `text-shadow`
- `text-decoration`
- `visibility` (element still occupies space)
- `outline-*`

**Triggers compositor update only (cheapest):**
- `transform` (on its own layer)
- `opacity` (on its own layer)
- `filter` (on its own layer)
- Properties on elements with `will-change`

Blink determines this classification by comparing field-by-field in the `ComputedStyle` diff. Each field group knows whether its properties affect layout, paint, or compositing.

---

## 9. Open UI Extraction Notes

This section documents the extraction boundary: what we take from Blink's style system, what we drop, and what we modify. The goal is a standalone `libopenui_style.so` that provides cascade, inheritance, and computed value resolution without requiring a DOM, CSS parser, or browser environment.

### What We Keep

| Component | Blink Source | Open UI Use |
|-----------|-------------|-------------|
| **Cascade machinery** | `StyleCascade`, `CascadePriority` | Resolve conflicting rules by specificity and order |
| **Inheritance system** | Per-property `ApplyInherit` / `ApplyInitial` | Flow inherited values down the node tree |
| **Computed value resolution** | Per-property `ApplyValue` + `CSSLengthResolver` | Resolve `em`, `rem`, `%`, `calc()` to absolute pixels |
| **Style sharing** | `MatchedPropertiesCache` | Deduplicate identical `ComputedStyle` objects |
| **ComputedStyle structure** | Field groups, bitfield packing, rare data | Memory-efficient style storage |
| **Change detection** | `StyleDifference` diffing | Classify changes as layout / paint / composite |
| **Custom properties** | `CSSVariableResolver`, variable substitution | `var(--*)` theming support |
| **Shorthand expansion** | Per-shorthand expansion logic | `margin: 10px` → four longhands |

### What We Drop

| Component | Reason |
|-----------|--------|
| **CSS parsing** (`CSSParser`, `CSSTokenizer`) | Open UI uses a structured C API with type-safe per-property setters. No CSS text parsing needed (except optional convenience API). |
| **Selector matching** (`CSSSelectorList`, `SelectorChecker`) | Open UI attaches rules directly to nodes. No selector-based matching. |
| **`@media` queries** | Open UI exposes viewport dimensions programmatically. Applications handle responsive logic in code. |
| **Pseudo-elements** (`::before`, `::after`, `::placeholder`) | No generated content. UI is explicitly constructed via the scene graph API. |
| **User-agent stylesheet** (`html.css`) | No HTML semantics. All initial values come from the CSS property spec defaults. |
| **CSSOM** (`CSSStyleSheet`, `CSSStyleDeclaration`, `getComputedStyle()`) | Not needed. Computed values are queried through the C API. |
| **Animation integration** (`CSSAnimations`, `@keyframes` resolution) | Animations are handled by the compositor layer, not the style system. |

### What We Simplify

**Property set.** We support ~80 curated properties (see below) instead of Blink's ~600. This dramatically reduces generated code and `ComputedStyle` size.

**Origin model.** We retain the cascade algorithm but simplify origins:

```
Blink:    user-agent → user → author (each with normal + !important)
Open UI:  default → theme → component → inline (each with normal + !important)
```

Our four origins map naturally to UI framework layering:
- **Default:** Framework-provided initial styles
- **Theme:** Application theme (light/dark mode, brand colors)
- **Component:** Component-level styles
- **Inline:** Per-instance overrides

**No `@layer`.** For V1 we omit cascade layers. Our four-origin model provides sufficient layering for UI framework use cases. Layers can be added later if needed.

**`ComputedStyle` field groups.** With ~80 properties instead of ~600, we can simplify the field group structure. Rare data groups remain valuable — most nodes won't set `transform`, `filter`, or `clip-path`.

### Key Extraction Boundary

```
┌─────────────────────────────────────────────────────────┐
│                    Application Code                      │
│   oui_style_rule_set_color(rule, color);                │
│   oui_style_rule_set_margin(rule, edges);               │
│   oui_style_node_add_rule(node, rule);                  │
│   oui_style_resolve(root, ctx);                         │
│   style = oui_style_node_get_computed(node);            │
│   color = oui_computed_style_color(style);              │
└──────────────────────┬──────────────────────────────────┘
                       │ C API boundary
┌──────────────────────▼──────────────────────────────────┐
│                   libopenui_style                        │
│                                                          │
│   OuiStyleRule        → specified values (input)         │
│   OuiStyleCascade     → cascade resolution               │
│   OuiInheritance      → parent-to-child propagation      │
│   OuiValueResolver    → em/rem/%/calc → px               │
│   OuiComputedStyle    → immutable output snapshot         │
│   OuiStyleDifference  → change classification             │
│   OuiStyleSharingCache → deduplication                    │
│                                                          │
│   Input:  C API property setters (structured)            │
│   Output: ComputedStyle (read by layout engine)          │
└─────────────────────────────────────────────────────────┘
```

**`ComputedStyle` is the output.** The style system's sole job is to produce an immutable `ComputedStyle` for every node in the tree. Layout reads `ComputedStyle`; it never reads style rules directly.

**C API properties are the input.** Instead of CSS text, the application calls type-safe setters. Internally, these create specified values in the same format Blink uses, so the cascade and resolution pipeline operates identically.

### Property Set for Open UI

The curated property set, organized by domain:

**Box Model (20 properties):**

| Property | Type | Inherited | Affects |
|----------|------|-----------|---------|
| `display` | keyword | no | layout |
| `position` | keyword | no | layout |
| `box-sizing` | keyword | no | layout |
| `width`, `height` | length/percentage/auto | no | layout |
| `min-width`, `max-width` | length/percentage/auto/none | no | layout |
| `min-height`, `max-height` | length/percentage/auto/none | no | layout |
| `margin-top`, `-right`, `-bottom`, `-left` | length/percentage/auto | no | layout |
| `padding-top`, `-right`, `-bottom`, `-left` | length/percentage | no | layout |
| `overflow-x`, `overflow-y` | keyword | no | layout |

**Flexbox (12 properties):**

| Property | Type | Inherited | Affects |
|----------|------|-----------|---------|
| `flex-direction` | keyword | no | layout |
| `flex-wrap` | keyword | no | layout |
| `flex-grow` | number | no | layout |
| `flex-shrink` | number | no | layout |
| `flex-basis` | length/percentage/auto | no | layout |
| `align-items`, `align-self` | keyword | no | layout |
| `align-content` | keyword | no | layout |
| `justify-content`, `justify-items`, `justify-self` | keyword | no | layout |
| `order` | integer | no | layout |

**Grid (10 properties):**

| Property | Type | Inherited | Affects |
|----------|------|-----------|---------|
| `grid-template-columns`, `grid-template-rows` | track list | no | layout |
| `grid-auto-columns`, `grid-auto-rows` | track size | no | layout |
| `grid-auto-flow` | keyword | no | layout |
| `grid-column-start`, `grid-column-end` | line | no | layout |
| `grid-row-start`, `grid-row-end` | line | no | layout |
| `gap`, `row-gap`, `column-gap` | length/percentage | no | layout |

**Visual (14 properties):**

| Property | Type | Inherited | Affects |
|----------|------|-----------|---------|
| `color` | color | **yes** | paint |
| `background-color` | color | no | paint |
| `opacity` | number (0–1) | no | composite |
| `border-top-color`, `-right-color`, `-bottom-color`, `-left-color` | color | no | paint |
| `border-top-style`, `-right-style`, `-bottom-style`, `-left-style` | keyword | no | paint |
| `border-top-width`, `-right-width`, `-bottom-width`, `-left-width` | length | no | layout |
| `border-top-left-radius`, `-top-right-radius`, `-bottom-right-radius`, `-bottom-left-radius` | length/percentage | no | paint |
| `box-shadow` | shadow list | no | paint |
| `transform` | transform list | no | composite |
| `transform-origin` | position | no | composite |
| `filter` | filter list | no | composite |
| `visibility` | keyword | **yes** | paint |
| `z-index` | integer/auto | no | paint |
| `clip-path` | shape/reference | no | composite |

**Font & Text (16 properties):**

| Property | Type | Inherited | Affects |
|----------|------|-----------|---------|
| `font-family` | family list | **yes** | layout |
| `font-size` | length/percentage/keyword | **yes** | layout |
| `font-weight` | number/keyword | **yes** | layout |
| `font-style` | keyword | **yes** | layout |
| `line-height` | number/length/percentage/normal | **yes** | layout |
| `letter-spacing` | length/normal | **yes** | layout |
| `word-spacing` | length/normal | **yes** | layout |
| `text-align` | keyword | **yes** | layout |
| `text-decoration` | shorthand | no | paint |
| `text-transform` | keyword | **yes** | layout |
| `text-overflow` | keyword | no | paint |
| `white-space` | keyword | **yes** | layout |
| `word-break` | keyword | **yes** | layout |
| `overflow-wrap` | keyword | **yes** | layout |
| `direction` | keyword | **yes** | layout |
| `writing-mode` | keyword | **yes** | layout |

**Interaction (3 properties):**

| Property | Type | Inherited | Affects |
|----------|------|-----------|---------|
| `cursor` | keyword | **yes** | none (input handling) |
| `pointer-events` | keyword | **yes** | none (input handling) |
| `user-select` | keyword | no | none (input handling) |

**Shorthands (convenience, expand to longhands):**

`margin`, `padding`, `border`, `border-width`, `border-style`, `border-color`, `border-radius`, `flex`, `flex-flow`, `grid-column`, `grid-row`, `gap`, `overflow`, `font`, `background`

### Implementation Strategy

1. **Generate from a property registry.** Define our ~80 properties in a JSON5 file (modeled after `css_properties.json5`). Generate `ComputedStyle` fields, per-property accessors, cascade application code, and the C API wrappers.

2. **Reuse Blink's cascade algorithm.** Port `StyleCascade`'s priority resolution. Replace CSS selector matching with direct rule attachment.

3. **Reuse Blink's value resolution.** Port `CSSLengthResolver`, `calc()` evaluation, `var()` substitution. These are self-contained algorithms.

4. **Simplify `ComputedStyle`.** With ~80 properties, we need fewer field groups. Keep the bitfield packing and rare-data patterns — they remain valuable even at smaller scale.

5. **Keep style sharing.** The `MatchedPropertiesCache` approach is directly applicable. Nodes with the same set of rules and the same parent style produce the same `ComputedStyle`.

6. **Reuse `StyleDifference` diffing.** Port the field-group comparison logic. Each property knows whether it affects layout, paint, or compositing.

---

## Appendix A: File Reference

Quick reference to the key Blink source files discussed in this document. All paths are relative to `third_party/blink/renderer/`.

| File | Description |
|------|-------------|
| `core/css/resolver/style_resolver.h` | Top-level style resolution entry point |
| `core/css/resolver/style_cascade.h` | Modern cascade implementation |
| `core/css/resolver/cascade_priority.h` | Cascade priority encoding (origin + importance + layer + specificity) |
| `core/css/resolver/cascade_filter.h` | Cascade filtering |
| `core/css/resolver/cascade_origin.h` | Origin definitions (UA, user, author) |
| `core/css/resolver/style_resolver_state.h` | Mutable state during style resolution |
| `core/css/resolver/style_sharing_candidate.h` | Style sharing cache |
| `core/css/css_properties.json5` | Machine-readable property registry |
| `core/css/properties/css_property_ref.h` | Unified property reference |
| `core/css/properties/longhands/*.h` | Per-longhand-property resolution logic |
| `core/css/properties/shorthands/*.h` | Per-shorthand expansion logic |
| `core/css/css_value.h` | Base class for all CSS values |
| `core/css/css_primitive_value.h` | Numeric / length / percentage values |
| `core/css/css_math_function_value.h` | `calc()`, `min()`, `max()`, `clamp()` |
| `core/css/css_math_expression_node.h` | Expression tree for math functions |
| `core/css/css_custom_property_declaration.h` | Custom property (`--*`) declarations |
| `core/css/css_variable_data.h` | Tokenized variable data |
| `core/css/css_variable_resolver.h` | `var()` substitution |
| `core/css/css_length_resolver.h` | Context for resolving relative lengths |
| `core/css/css_environment_variables.h` | `env()` variable store |
| `core/css/css_property_registration.h` | `@property` registration |
| `core/css/cascade_layer.h` | `@layer` tree structure |
| `core/css/cascade_layer_map.h` | Layer ordering map |
| `core/css/css_selector.h` | Selector representation and specificity |
| `core/css/rule_set.h` | Rule set and invalidation set construction |
| `core/css/invalidation/invalidation_set.h` | Invalidation set data structure |
| `core/css/invalidation/style_invalidator.h` | Tree-walking invalidation applicator |
| `core/css/style_engine.h` | Central style coordination, dirty marking |
| `core/style/computed_style.h` | Immutable computed style snapshot |
| `core/style/computed_style_base.h` | Generated field accessors |
| `core/style/computed_style_builder.h` | Mutable builder for `ComputedStyle` |
| `core/style/computed_style_field_group.json5` | Field group definitions |
| `core/style/style_rare_non_inherited_data.h` | Rarely-set non-inherited properties |
| `core/style/style_rare_inherited_data.h` | Rarely-set inherited properties |
| `core/style/style_box_data.h` | Box model sizing data |
| `core/style/style_surround_data.h` | Margin, padding, border data |
| `core/style/style_visual_data.h` | Visual properties data |
| `core/style/style_background_data.h` | Background layer data |
| `core/style/style_inherited_data.h` | Font, color, line-height data |
| `core/layout/layout_object.h` | Layout object, `StyleDifference` |
| `core/layout/layout_box.h` | Box layout, used value queries |
