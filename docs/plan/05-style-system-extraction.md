# Sub-Project 5: Style System Extraction

> Extract Blink's style cascade, inheritance, and computed value system as a standalone style resolution library.

## Objective

Produce `libopenui_style.so` — a standalone style resolution library that computes visual properties for a tree of nodes using cascade, inheritance, and property-specific value resolution. This gives Open UI a powerful, proven styling model without requiring CSS parsing or a DOM.

## Background: Blink's Style Architecture

Blink's style system resolves CSS declarations into computed values through several stages:

```
Specified Value → Cascaded Value → Computed Value → Used Value → Actual Value
```

1. **Specified values**: Declared in stylesheets or element styles
2. **Cascade**: Resolves conflicting declarations via specificity, origin, and order
3. **Computed values**: Resolve relative values (e.g., `em` → `px`, `inherit` → parent's value)
4. **Used values**: Resolve layout-dependent values (e.g., `auto` margins, percentage widths)
5. **Actual values**: Round to device pixels

### Key Components

| Component | Location | Role |
|---|---|---|
| `ComputedStyle` | `core/style/computed_style.h` | Immutable style snapshot for a node |
| `StyleResolver` | `core/css/resolver/style_resolver.h` | Main entry point for style resolution |
| `CascadeFilter` | `core/css/resolver/cascade_filter.h` | Applies cascade rules |
| `StyleCascade` | `core/css/resolver/style_cascade.h` | Modern cascade implementation |
| `ComputedStyleBuilder` | `core/style/computed_style_builder.h` | Builds `ComputedStyle` incrementally |
| `CSSPropertyRef` | `core/css/properties/` | Per-property resolution logic |
| `StyleSharingCache` | `core/css/resolver/style_sharing_candidate.h` | Caches and shares identical styles |

## Tasks

### 5.1 Extract Style Resolution

**Core style resolution pipeline:**

1. **Property registry** — Define the set of supported properties with:
   - Name, type, initial value, inherited flag
   - Parsing (from our style input format)
   - Computation (specified → computed)
   - Whether it affects layout, paint, or compositing

2. **Cascade** — Resolve conflicting property declarations:
   - Multiple style rules can apply to one node
   - Resolution order: specificity → origin → declaration order
   - `!important` handling
   - We simplify by removing browser/user-agent stylesheet concepts

3. **Inheritance** — Properties marked as inherited flow from parent to child:
   - `color`, `font-*`, `line-height`, etc. are inherited
   - `width`, `margin`, `padding`, etc. are not
   - Explicit `inherit` and `initial` keywords

4. **Value resolution** — Convert specified values to computed values:
   - Relative units (`em`, `rem`, `%`) → absolute pixels
   - `currentColor` → actual color value
   - `calc()` expressions → resolved values
   - Shorthand expansion (e.g., `margin: 10px` → all four sides)
   - Custom properties (variables) → substituted values

5. **Style sharing** — Chromium's optimization where nodes with identical styles share the same `ComputedStyle` object. Critical for memory efficiency in large trees.

### 5.2 Define the Property Set

Curate a property set that covers UI framework needs — a practical subset of CSS:

**Core box model:**
- `display`: `block`, `flex`, `grid`, `inline`, `inline-block`, `inline-flex`, `inline-grid`, `none`, `contents`
- `position`: `static`, `relative`, `absolute`, `fixed`, `sticky`
- `width`, `height`, `min-width`, `max-width`, `min-height`, `max-height`
- `margin`, `padding`, `border-width` (per-side)
- `box-sizing`: `content-box`, `border-box`
- `overflow`, `overflow-x`, `overflow-y`

**Flexbox:**
- `flex-direction`, `flex-wrap`, `flex-flow`
- `flex-grow`, `flex-shrink`, `flex-basis`, `flex`
- `align-items`, `align-self`, `align-content`
- `justify-content`, `justify-items`, `justify-self`
- `gap`, `row-gap`, `column-gap`
- `order`

**Grid:**
- `grid-template-columns`, `grid-template-rows`
- `grid-column`, `grid-row`, `grid-column-start`, `grid-column-end`, `grid-row-start`, `grid-row-end`
- `grid-auto-flow`, `grid-auto-columns`, `grid-auto-rows`

**Visual:**
- `background-color`, `background-image`, `background-*`
- `color`, `opacity`
- `border-color`, `border-style`, `border-radius`
- `box-shadow`
- `transform`, `transform-origin`
- `transition`, `animation`
- `filter`, `backdrop-filter`
- `visibility`, `z-index`
- `clip-path`

**Text & font:**
- `font-family`, `font-size`, `font-weight`, `font-style`, `font-stretch`
- `line-height`, `letter-spacing`, `word-spacing`
- `text-align`, `text-decoration`, `text-transform`, `text-overflow`
- `white-space`, `word-break`, `overflow-wrap`
- `writing-mode`, `direction`, `unicode-bidi`

**Interaction:**
- `cursor`, `pointer-events`
- `user-select`
- `scroll-behavior`, `scroll-snap-type`, `scroll-snap-align`

**Dropped from CSS** (not relevant for native UI):
- `float`, `clear` (legacy layout)
- `content` (generated content)
- Most `@` rules (we keep `@keyframes`, drop `@media` in favor of programmatic queries)
- Selectors complexity (no pseudo-elements like `::before`/`::after`)
- CSS nesting and layers (simplify for V1)

### 5.3 Style Input Format

**Structured API (primary):**
```c
OuiStyleRule* rule = oui_style_rule_create();
oui_style_rule_set_color(rule, oui_color_rgba(255, 0, 0, 255));
oui_style_rule_set_font_size(rule, oui_length_px(16));
oui_style_rule_set_margin(rule, oui_edges_uniform(oui_length_px(10)));
```

**CSS-like text (secondary, for convenience):**
```c
OuiStyleRule* rule = oui_style_rule_parse(
    "color: red; font-size: 16px; margin: 10px;"
);
```

**Theming:**
```c
// Custom properties / variables
OuiStyleContext* ctx = oui_style_context_create();
oui_style_context_set_variable(ctx, "--primary-color", "rgb(0, 120, 255)");
oui_style_context_set_variable(ctx, "--spacing-unit", "8px");

// Rules can reference variables
OuiStyleRule* rule = oui_style_rule_parse(
    "color: var(--primary-color); padding: var(--spacing-unit);"
);

// Dark/light mode: just swap the context's variables
```

### 5.4 C API Design (`include/openui/openui_style.h`)

```c
// === Style Context (holds variables, defaults) ===
OuiStyleContext* oui_style_context_create(void);
void             oui_style_context_destroy(OuiStyleContext* ctx);
void             oui_style_context_set_variable(OuiStyleContext* ctx, const char* name, const char* value);
void             oui_style_context_set_default_font(OuiStyleContext* ctx, const char* family, float size);

// === Style Rules ===
OuiStyleRule* oui_style_rule_create(void);
OuiStyleRule* oui_style_rule_parse(const char* css_text);  // Convenience: parse CSS-like text
void          oui_style_rule_destroy(OuiStyleRule* rule);

// Per-property setters (type-safe C API)
void oui_style_rule_set_display(OuiStyleRule* rule, OuiDisplay display);
void oui_style_rule_set_color(OuiStyleRule* rule, OuiColor color);
void oui_style_rule_set_font_size(OuiStyleRule* rule, OuiLength size);
void oui_style_rule_set_width(OuiStyleRule* rule, OuiLength width);
void oui_style_rule_set_flex_direction(OuiStyleRule* rule, OuiFlexDirection dir);
// ... (one setter per property in our property set)

// === Style Application ===
// Apply rules to a node (rules are ordered; later rules override earlier ones)
void oui_style_node_add_rule(OuiLayoutNode* node, OuiStyleRule* rule);
void oui_style_node_set_inline_style(OuiLayoutNode* node, OuiStyleRule* rule);

// === Style Resolution ===
// Resolve all styles in a tree (cascade + inheritance + computation)
OuiStatus oui_style_resolve(OuiLayoutNode* root, OuiStyleContext* ctx);

// === Computed Value Queries ===
OuiComputedStyle* oui_style_node_get_computed(const OuiLayoutNode* node);
OuiColor  oui_computed_style_color(const OuiComputedStyle* style);
float     oui_computed_style_font_size(const OuiComputedStyle* style);
OuiLength oui_computed_style_width(const OuiComputedStyle* style);
// ... (one getter per property)

// === Change Detection ===
// After re-resolving styles, query what changed
typedef enum {
    OUI_STYLE_CHANGE_NONE       = 0,
    OUI_STYLE_CHANGE_PAINT      = 1 << 0,  // Only visual change (color, background)
    OUI_STYLE_CHANGE_LAYOUT     = 1 << 1,  // Affects layout (width, margin, flex)
    OUI_STYLE_CHANGE_COMPOSITE  = 1 << 2,  // Affects compositing (opacity, transform)
} OuiStyleChangeFlags;

OuiStyleChangeFlags oui_style_node_get_changes(const OuiLayoutNode* node);
```

### 5.5 Integration with Layout Engine

Wire style resolution output into layout engine input:

```
oui_style_resolve(root, ctx)    →  Populates computed styles on all nodes
oui_layout_compute(root, w, h)  →  Reads computed styles to determine layout
```

The layout engine reads style properties from `OuiComputedStyle` attached to each `OuiLayoutNode`. The style system must populate:
- Display type (block, flex, grid, etc.)
- Box model values (width, height, margin, padding, border)
- Flex/grid properties
- Text properties (for inline layout)
- Position properties (static, relative, absolute, fixed)

**End-to-end validation: Style → Layout → Compositor → Skia**

### 5.6 Validation

**Cascade tests:**
- Specificity ordering
- Origin ordering
- `!important` behavior
- Inheritance (inherited properties flow, non-inherited don't)
- `inherit`, `initial`, `unset` keywords

**Value resolution tests:**
- `em` → pixels relative to parent font-size
- `%` → pixels relative to containing block
- `calc()` expressions
- Custom property substitution
- Shorthand expansion

**Performance tests:**
- Style resolution for 10,000 nodes in < 5ms
- Style sharing: memory usage with many identically-styled nodes
- Incremental style: change one variable → re-resolve only affected subtree

## Deliverables

| Deliverable | Description |
|---|---|
| `libopenui_style.so` | Style system shared library |
| `include/openui/openui_style.h` | Public C header |
| `examples/style_theme.c` | Theming demo (light/dark mode switch) |
| `tests/style/` | Cascade, inheritance, and resolution tests |
| `benchmarks/style/` | Style resolution performance benchmarks |

## Success Criteria

- [ ] Cascade resolves correctly (specificity, origin, order)
- [ ] Inheritance works for all inherited properties
- [ ] Custom properties (variables) substitute correctly
- [ ] `em`, `%`, `calc()` resolve to correct pixel values
- [ ] Style change detection correctly identifies layout vs. paint vs. composite changes
- [ ] Style resolution for 10,000 nodes < 5ms
- [ ] Full pipeline works: Style → Layout → Compositor → Skia renders correctly
