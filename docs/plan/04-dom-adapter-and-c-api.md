# Sub-Project 4: DOM Adapter & C API

> Create the minimal DOM adapter layer and first C API so applications can programmatically build element trees, set styles, compute layout, and query geometry.

## Objective

With the rendering pipeline compiling (SP3), this sub-project creates the **programmatic interface** that replaces HTML/CSS parsing. Users build element trees through a C API. Internally, our adapter creates the minimal DOM objects (Element, Node, Document) that Blink's layout engine expects, and translates programmatic style-setting into ComputedStyle objects.

**End result:** A C program can create a `<div>` with flexbox children, set CSS properties on them via function calls, trigger layout, and read back computed positions and sizes — all using Chromium's real LayoutNG algorithms.

## Architecture

```
C API (openui.h)
    │
    ▼
Adapter Layer (src/adapter/)
    │ Creates/manages:
    │ - OuiDocument → Blink Document (minimal)
    │ - OuiElement → Blink Element (minimal) → LayoutObject
    │ - OuiStyle settings → ComputedStyle
    │
    ▼
Blink Rendering (extracted in SP3)
    │ Style resolution, LayoutNG, Paint
    ▼
Results (geometry, fragments)
```

## Tasks

### Phase A: Minimal DOM Adapter

1. **A1: OuiDocument** — Wraps a minimal Blink `Document`. Owns the lifecycle, provides the context for element creation. Manages the document lifecycle (style recalc, layout, paint phases).

2. **A2: OuiElement** — Wraps a minimal Blink `Element`. Supports: create, destroy, add/remove child, set tag name (maps to HTML element types). The element auto-creates its `LayoutObject` based on display type.

3. **A3: Minimal Node/Element stubs** — Flesh out the DOM stubs from SP3 so they satisfy LayoutNG's traversal patterns (child iteration, parent access, sibling access, containing block lookup).

### Phase B: Programmatic Style API

4. **B1: Style property setters** — C API functions to set individual CSS properties on an element: `oui_element_set_display()`, `oui_element_set_width()`, `oui_element_set_flex_direction()`, etc. Internally, these write directly into the element's `ComputedStyle`, bypassing CSS cascade/parsing.

5. **B2: Shorthand properties** — Support CSS shorthands through the C API (e.g., `oui_element_set_margin()` sets all four edges, `oui_element_set_flex()` sets grow/shrink/basis).

6. **B3: Style enumeration** — Map all CSS visual properties to C API functions. Target the full set from MDN's CSS reference that affects layout/visual rendering (skip properties that only affect interaction or browser behavior).

### Phase C: Layout Computation & Query

7. **C1: Layout trigger** — `oui_document_layout()` runs style resolution + LayoutNG on the element tree. Internally calls `Document::UpdateStyleAndLayout()` (or our equivalent).

8. **C2: Geometry queries** — After layout: `oui_element_get_x()`, `_get_y()`, `_get_width()`, `_get_height()`, `_get_baseline()`. These read from the LayoutObject's fragment output.

9. **C3: Hit testing** — `oui_element_hit_test(doc, x, y)` → returns the element at that point. Uses Blink's existing hit testing infrastructure.

10. **C4: Scroll queries** — `oui_element_get_scroll_width()`, `_get_scroll_height()` for overflow content.

### Phase D: Text Elements

11. **D1: Text nodes** — `oui_element_set_text_content()` creates/updates text child nodes. Text flows through Blink's inline layout (line breaking, shaping, bidi).

12. **D2: Font API** — `oui_element_set_font_family()`, `_set_font_size()`, `_set_font_weight()`. Maps to ComputedStyle font properties. Uses Blink's font infrastructure (HarfBuzz, fontconfig).

### Phase E: Verification

13. **E1: Basic layout tests** — Create div with fixed size → verify geometry. Create flexbox → verify child positions. Create grid → verify track sizing.

14. **E2: WPT subset** — Port a representative subset of Web Platform Tests, driving them through our C API instead of HTML. Flexbox (~50 tests), Grid (~50 tests), Block (~30 tests).

15. **E3: Text layout tests** — Verify line breaking, text wrapping, font metrics, bidi text.

## C API Design (preview)

```c
// === Document ===
OuiDocument* oui_document_create(void);
void         oui_document_destroy(OuiDocument* doc);
OuiStatus    oui_document_layout(OuiDocument* doc, float viewport_width, float viewport_height);

// === Element lifecycle ===
OuiElement*  oui_element_create(OuiDocument* doc, const char* tag);  // "div", "span", "button", etc.
void         oui_element_destroy(OuiElement* elem);
void         oui_element_append_child(OuiElement* parent, OuiElement* child);
void         oui_element_remove_child(OuiElement* parent, OuiElement* child);
void         oui_element_set_root(OuiDocument* doc, OuiElement* root);

// === Style (CSS properties set programmatically) ===
void oui_element_set_display(OuiElement* e, OuiDisplay display);
void oui_element_set_width(OuiElement* e, OuiLength width);
void oui_element_set_height(OuiElement* e, OuiLength height);
void oui_element_set_margin(OuiElement* e, OuiEdgeValues margins);
void oui_element_set_padding(OuiElement* e, OuiEdgeValues padding);
void oui_element_set_flex_direction(OuiElement* e, OuiFlexDirection dir);
void oui_element_set_background_color(OuiElement* e, uint32_t rgba);
void oui_element_set_font_size(OuiElement* e, OuiLength size);
void oui_element_set_color(OuiElement* e, uint32_t rgba);
// ... 100+ more CSS property setters

// === Geometry queries (after layout) ===
float oui_element_get_x(const OuiElement* e);
float oui_element_get_y(const OuiElement* e);
float oui_element_get_width(const OuiElement* e);
float oui_element_get_height(const OuiElement* e);

// === Text ===
void oui_element_set_text_content(OuiElement* e, const char* text, size_t len);

// === Hit testing ===
OuiElement* oui_document_hit_test(OuiDocument* doc, float x, float y);
```

## Deliverables

| Deliverable | Description |
|---|---|
| `src/adapter/` | DOM adapter implementation |
| `include/openui/openui.h` | Unified C API header (initial version) |
| `tests/api/` | C API test suite (layout correctness, text, hit testing) |
| `examples/layout_flexbox.c` | Flexbox layout example via C API |

## Success Criteria

- [ ] Create flexbox container with children → get correct layout positions via C API
- [ ] Create grid container → correct track sizing and cell placement
- [ ] Text wrapping at container width boundary
- [ ] Hit testing returns correct element
- [ ] Port 130+ WPT layout tests, >90% passing
