# Sub-Project 4: DOM Adapter & C API

> Build the stable C API (`openui.h`) that wraps Chromium's real blink rendering pipeline. External programs create element trees, set CSS properties, trigger layout, and query geometry — all through C function calls backed by actual Chromium code.

## Objective

SP3 proved we can programmatically create real blink DOM elements (`HTMLDivElement`, etc.), set styles via `SetInlineStyleProperty()`, run layout via `UpdateStyleAndLayout()`, and query computed geometry — all within Chromium's build system. SP4 wraps this into a **stable C ABI** that any language can call via FFI.

**End result:** A C program links against `libopenui.so`, creates a document, builds an element tree with CSS properties set via typed function calls, triggers layout, and reads back pixel-precise positions and sizes — all powered by Chromium's real LayoutNG algorithms.

## Architecture

```
External program (C, Rust, Go, Python, etc.)
    │  FFI / dlopen
    ▼
openui.h — Stable C ABI (opaque handles + functions)
    │
    ▼
openui_impl.cc — C++ adapter (thin translation layer)
    │  Uses blink::Persistent<> for GC-safe references
    │  Maps tag strings → HTMLElement subclasses
    │  Maps CSSPropertyID → SetInlineStyleProperty()
    │
    ▼
Blink internals (DummyPageHolder, HTMLDivElement, LayoutNG, etc.)
    │  Full style resolution → layout → paint pipeline
    ▼
Geometry results (LayoutBox positions, sizes, baselines)
```

### Key Design Principles

1. **Opaque handles** — `OuiDocument*`, `OuiElement*` are opaque pointers to C++ wrapper objects. Users never touch blink internals.
2. **GC safety** — Wrapper objects hold `blink::Persistent<Element>` (prevents Oilpan GC from collecting live elements).
3. **Real blink objects** — No stubs. `oui_element_create(doc, "div")` creates a real `HTMLDivElement` in a real blink `Document`.
4. **Typed style API** — Two approaches:
   - **Generic**: `oui_element_set_style(e, "width", "200px")` — uses `SetInlineStyleProperty(CSSPropertyID, String)` internally
   - **Typed convenience**: `oui_element_set_width(e, oui_px(200))` — type-safe, no string parsing at call site
5. **Single init/shutdown** — `oui_init()` / `oui_shutdown()` manage the heavy blink bootstrap (V8, ICU, ResourceBundle, mojo) once per process.

## What SP3 Already Proved

These blink APIs work and are the foundation for SP4's implementation:

| Capability | Blink API | SP3 Test |
|---|---|---|
| Create elements | `MakeGarbageCollected<HTMLDivElement>(doc)` | All 20 tests |
| Set inline styles | `elem->setAttribute(html_names::kStyleAttr, ...)` | All 20 tests |
| Set style properties | `elem->SetInlineStyleProperty(CSSPropertyID, value, unit)` | Available (not yet used in tests) |
| Build DOM tree | `parent->AppendChild(child)` | Nested layout test |
| Set text content | `doc.createTextNode(text)` + `AppendChild()` | Text rendering test |
| Trigger layout | `doc.UpdateStyleAndLayout(reason)` | All 20 tests |
| Full lifecycle | `doc.View()->UpdateAllLifecyclePhasesForTest()` | Paint tests |
| Query box geometry | `ToLayoutBox(elem->GetLayoutObject())->Size()` | All layout tests |
| Query offsets | `elem->OffsetTop(nullptr)`, `OffsetLeft(nullptr)` | Absolute positioning test |
| Computed style | `elem->GetComputedStyle()->FontSize()` | Computed style test |
| Viewport control | `DummyPageHolder(gfx::Size(w, h))` | All tests (800×600) |
| Paint artifacts | `doc.View()->GetPaintArtifact()` | Paint lifecycle test |

## Tasks

### Phase A: Runtime Bootstrap & Document (3 tasks)

**A1: `oui_init()` / `oui_shutdown()`** — Extract the initialization sequence from `rendering_test.cc`'s `main()` into reusable functions. This includes:
- `base::AtExitManager` (or integration with caller's main)
- ICU initialization
- DiscardableMemoryAllocator
- FeatureList
- ResourceBundle with `content_shell.pak`
- Mojo
- V8 snapshot loading + flags
- `blink::Platform::InitializeBlink()`
- `OpenUIPlatform` instantiation
- `blink::Initialize()` (equivalent of `InitializeWithoutIsolateForTesting`)
- Thread scheduler setup

Expose as: `OuiStatus oui_init(const OuiInitConfig* config)` and `void oui_shutdown()`.

**A2: `OuiDocument` wrapper** — Create document wrapper that owns a `DummyPageHolder` (or equivalent minimal page). Provides:
- `OuiDocument* oui_document_create(int viewport_w, int viewport_h)` — creates `DummyPageHolder` with given size
- `void oui_document_destroy(OuiDocument* doc)` — releases the page holder
- `void oui_document_set_viewport(OuiDocument* doc, int w, int h)` — `FrameView::SetLayoutSize()`
- `OuiStatus oui_document_layout(OuiDocument* doc)` — `doc.UpdateStyleAndLayout()`
- `OuiStatus oui_document_update_all(OuiDocument* doc)` — `View()->UpdateAllLifecyclePhasesForTest()`
- Internally holds `std::unique_ptr<DummyPageHolder>` + `blink::test::TaskEnvironment`

**A3: `OuiElement` wrapper** — Element lifecycle and DOM tree manipulation:
- `OuiElement* oui_element_create(OuiDocument* doc, const char* tag)` — maps tag name to HTML element class:
  - `"div"` → `HTMLDivElement`, `"span"` → `HTMLSpanElement`, `"p"` → `HTMLParagraphElement`, etc.
  - Uses a static lookup table (tag string → factory function)
  - Returns opaque handle wrapping `blink::Persistent<Element>`
- `void oui_element_destroy(OuiElement* elem)` — releases Persistent, removes from DOM if attached
- `void oui_element_append_child(OuiElement* parent, OuiElement* child)` — `parent->AppendChild(child)`
- `void oui_element_remove_child(OuiElement* parent, OuiElement* child)` — `parent->RemoveChild(child)`
- `void oui_element_insert_before(OuiElement* parent, OuiElement* child, OuiElement* ref)` — `parent->InsertBefore(child, ref)`
- `OuiElement* oui_document_body(OuiDocument* doc)` — returns wrapper for `<body>`

### Phase B: Style API (3 tasks)

**B1: Generic style setter** — The workhorse API. Accepts any CSS property name and value as strings:
```c
OuiStatus oui_element_set_style(OuiElement* e, const char* property, const char* value);
OuiStatus oui_element_remove_style(OuiElement* e, const char* property);
void      oui_element_clear_styles(OuiElement* e);
```
Implementation: look up `CSSPropertyID` from property name string → call `element->SetInlineStyleProperty(id, String(value))`. This leverages blink's CSS value parser for the value string (e.g., `"200px"`, `"flex"`, `"1fr 2fr auto"`). Covers **all 799 CSS properties** with a single function.

**B2: Typed convenience setters** — For the ~30 most common properties, provide type-safe C functions that avoid string parsing at the call site:
```c
// Length values (px, %, em, auto, etc.)
void oui_element_set_width(OuiElement* e, OuiLength len);
void oui_element_set_height(OuiElement* e, OuiLength len);
void oui_element_set_min_width(OuiElement* e, OuiLength len);
void oui_element_set_max_width(OuiElement* e, OuiLength len);
void oui_element_set_margin_top(OuiElement* e, OuiLength len);
// ... margin_right, margin_bottom, margin_left, padding_*

// Enum values
void oui_element_set_display(OuiElement* e, OuiDisplay display);
void oui_element_set_position(OuiElement* e, OuiPosition pos);
void oui_element_set_flex_direction(OuiElement* e, OuiFlexDirection dir);
void oui_element_set_align_items(OuiElement* e, OuiAlignItems align);
void oui_element_set_justify_content(OuiElement* e, OuiJustifyContent jc);
void oui_element_set_overflow(OuiElement* e, OuiOverflow overflow);

// Color values
void oui_element_set_color(OuiElement* e, uint32_t rgba);
void oui_element_set_background_color(OuiElement* e, uint32_t rgba);

// Numeric values
void oui_element_set_flex_grow(OuiElement* e, float grow);
void oui_element_set_flex_shrink(OuiElement* e, float shrink);
void oui_element_set_opacity(OuiElement* e, float opacity);
void oui_element_set_z_index(OuiElement* e, int z);
```
Implementation: Each calls `SetInlineStyleProperty()` with the correct `CSSPropertyID`, value, and `CSSPrimitiveValue::UnitType`.

**B3: OuiLength and enum types** — Define the C-side value types:
```c
typedef struct { float value; OuiUnit unit; } OuiLength;
// Helper constructors:
static inline OuiLength oui_px(float v)  { return (OuiLength){v, OUI_UNIT_PX}; }
static inline OuiLength oui_pct(float v) { return (OuiLength){v, OUI_UNIT_PERCENT}; }
static inline OuiLength oui_em(float v)  { return (OuiLength){v, OUI_UNIT_EM}; }
static inline OuiLength oui_auto(void)   { return (OuiLength){0, OUI_UNIT_AUTO}; }

typedef enum { OUI_DISPLAY_BLOCK, OUI_DISPLAY_FLEX, OUI_DISPLAY_GRID, ... } OuiDisplay;
typedef enum { OUI_POSITION_STATIC, OUI_POSITION_RELATIVE, OUI_POSITION_ABSOLUTE, ... } OuiPosition;
// ... one enum per CSS enum-valued property
```

### Phase C: Layout Query & Hit Testing (3 tasks)

**C1: Geometry queries** — Read layout results after `oui_document_layout()`:
```c
float oui_element_get_offset_x(const OuiElement* e);     // OffsetLeft(nullptr)
float oui_element_get_offset_y(const OuiElement* e);     // OffsetTop(nullptr)
float oui_element_get_width(const OuiElement* e);         // LayoutBox::Size().Width()
float oui_element_get_height(const OuiElement* e);        // LayoutBox::Size().Height()
float oui_element_get_content_width(const OuiElement* e); // ContentSize().Width()
float oui_element_get_content_height(const OuiElement* e);
float oui_element_get_baseline(const OuiElement* e);      // FirstBaseline or IntrinsicBaseline

// Bounding rect relative to document
OuiRect oui_element_get_bounding_rect(const OuiElement* e);

// Computed style readback
const char* oui_element_get_computed_style(const OuiElement* e, const char* property);
```
Implementation: Cast `GetLayoutObject()` to `LayoutBox`, read geometry from fragment/box model.

**C2: Hit testing** — Find element at a given point:
```c
OuiElement* oui_document_hit_test(OuiDocument* doc, float x, float y);
```
Implementation: Uses blink's `HitTestResult` + `HitTestRequest` infrastructure. Requires paint phase to have run (`oui_document_update_all`).

**C3: Scroll geometry** — Overflow content queries:
```c
float oui_element_get_scroll_width(const OuiElement* e);
float oui_element_get_scroll_height(const OuiElement* e);
float oui_element_get_scroll_x(const OuiElement* e);
float oui_element_get_scroll_y(const OuiElement* e);
void  oui_element_set_scroll_position(OuiElement* e, float x, float y);
```

### Phase D: Text & Font (2 tasks)

**D1: Text content** — Set text on elements:
```c
void oui_element_set_text_content(OuiElement* e, const char* text);
```
Implementation: `element->setTextContent(String::FromUTF8(text))` — uses blink's standard DOM API which creates/updates child text nodes.

**D2: Font properties** — Convenience setters for font (also settable via generic `oui_element_set_style`):
```c
void oui_element_set_font_family(OuiElement* e, const char* family);
void oui_element_set_font_size(OuiElement* e, OuiLength size);
void oui_element_set_font_weight(OuiElement* e, int weight); // 100-900
void oui_element_set_font_style(OuiElement* e, OuiFontStyle style);
void oui_element_set_line_height(OuiElement* e, OuiLength lh);
```

### Phase E: Shared Library Build (2 tasks)

**E1: `libopenui.so` build target** — Add GN target in `openui/BUILD.gn`:
```gn
shared_library("libopenui") {
  sources = [ "openui_impl.cc" ]
  public = [ "openui.h" ]
  deps = [ blink rendering deps ]
  output_name = "openui"
}
```
The shared library links against blink's static libraries and exports only the C API symbols.

**E2: Symbol visibility** — Mark all `oui_*` functions with `__attribute__((visibility("default")))` (via `OUI_EXPORT` macro). All other symbols hidden. Verify with `nm -D libopenui.so | grep ' T '`.

### Phase F: Testing & Verification (3 tasks)

**F1: Unit tests (C++ side)** — Comprehensive GTest suite in `openui/openui_api_test.cc`:
- Init/shutdown lifecycle
- Document create/destroy
- Element create/destroy for every supported tag
- Style setting (generic + typed) for every convenience setter
- DOM tree manipulation (append, remove, insert before, reorder)
- Layout trigger and geometry query for: block, flex, grid, table, multi-column, absolute, fixed, inline-block
- Text content and font metrics
- Hit testing
- Scroll geometry
- Error handling (null pointers, invalid tags, invalid property names)
- **Target: 80+ tests**

**F2: C consumer tests** — Pure C program that links against `libopenui.so` and exercises the API:
- Proves the C ABI is actually usable from C (not just C++)
- Tests opaque handle semantics
- Tests value type constructors (`oui_px()`, `oui_pct()`, etc.)
- Flexbox layout example
- Grid layout example
- **Target: 20+ tests**

**F3: Multi-agent review** — Same loop as SP3: Opus 4.6 + GPT 5.4 review cycles until both find no issues.

## File Layout

```
chromium/src/openui/
├── BUILD.gn                  # Updated: add libopenui + api_test targets
├── openui.h                  # C API header (the public interface)
├── openui_impl.h             # Internal C++ types (OuiDocumentImpl, OuiElementImpl)
├── openui_impl.cc            # C API implementation
├── openui_init.h             # Init/shutdown internals
├── openui_init.cc            # Blink bootstrap extracted from rendering_test.cc main()
├── openui_element_factory.h  # Tag name → HTML element factory
├── openui_element_factory.cc # Static lookup table
├── openui_api_test.cc        # C++ GTest API tests (80+)
├── openui_c_test.c           # Pure C consumer test (20+)
├── rendering_test.cc         # (existing) SP3 rendering tests
└── smoke_test.cc             # (existing) SP3 Skia smoke test
```

Mirror in `open-ui/` repo:
```
open-ui/
├── include/openui/openui.h   # Copy of the public header
├── src/api/                   # Copies of implementation files
└── tests/api/                 # Copies of test files
```

## C API Header — Complete Design

```c
#ifndef OPENUI_H_
#define OPENUI_H_

#include <stdint.h>
#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

#ifdef _WIN32
  #ifdef OUI_BUILD
    #define OUI_EXPORT __declspec(dllexport)
  #else
    #define OUI_EXPORT __declspec(dllimport)
  #endif
#else
  #define OUI_EXPORT __attribute__((visibility("default")))
#endif

// ─── Opaque handles ─────────────────────────────────────────
typedef struct OuiDocument OuiDocument;
typedef struct OuiElement  OuiElement;

// ─── Status codes ───────────────────────────────────────────
typedef enum {
    OUI_OK = 0,
    OUI_ERROR_NOT_INITIALIZED = -1,
    OUI_ERROR_ALREADY_INITIALIZED = -2,
    OUI_ERROR_INVALID_ARGUMENT = -3,
    OUI_ERROR_UNKNOWN_TAG = -4,
    OUI_ERROR_UNKNOWN_PROPERTY = -5,
    OUI_ERROR_INVALID_VALUE = -6,
    OUI_ERROR_LAYOUT_REQUIRED = -7,
    OUI_ERROR_INTERNAL = -99,
} OuiStatus;

// ─── Value types ────────────────────────────────────────────
typedef enum {
    OUI_UNIT_PX,
    OUI_UNIT_PERCENT,
    OUI_UNIT_EM,
    OUI_UNIT_REM,
    OUI_UNIT_VW,
    OUI_UNIT_VH,
    OUI_UNIT_AUTO,
    OUI_UNIT_NONE,        // For "none" keyword (e.g., max-width: none)
    OUI_UNIT_FR,          // Grid fractional unit
} OuiUnit;

typedef struct { float value; OuiUnit unit; } OuiLength;
typedef struct { float x, y, width, height; } OuiRect;

// Helper constructors (inline for C99+)
static inline OuiLength oui_px(float v)  { OuiLength l = {v, OUI_UNIT_PX};      return l; }
static inline OuiLength oui_pct(float v) { OuiLength l = {v, OUI_UNIT_PERCENT};  return l; }
static inline OuiLength oui_em(float v)  { OuiLength l = {v, OUI_UNIT_EM};       return l; }
static inline OuiLength oui_rem(float v) { OuiLength l = {v, OUI_UNIT_REM};      return l; }
static inline OuiLength oui_vw(float v)  { OuiLength l = {v, OUI_UNIT_VW};       return l; }
static inline OuiLength oui_vh(float v)  { OuiLength l = {v, OUI_UNIT_VH};       return l; }
static inline OuiLength oui_fr(float v)  { OuiLength l = {v, OUI_UNIT_FR};       return l; }
static inline OuiLength oui_auto(void)   { OuiLength l = {0, OUI_UNIT_AUTO};     return l; }
static inline OuiLength oui_none(void)   { OuiLength l = {0, OUI_UNIT_NONE};     return l; }

// ─── Display values ─────────────────────────────────────────
typedef enum {
    OUI_DISPLAY_BLOCK, OUI_DISPLAY_INLINE, OUI_DISPLAY_INLINE_BLOCK,
    OUI_DISPLAY_FLEX, OUI_DISPLAY_INLINE_FLEX,
    OUI_DISPLAY_GRID, OUI_DISPLAY_INLINE_GRID,
    OUI_DISPLAY_TABLE, OUI_DISPLAY_TABLE_ROW, OUI_DISPLAY_TABLE_CELL,
    OUI_DISPLAY_NONE, OUI_DISPLAY_CONTENTS,
} OuiDisplay;

typedef enum {
    OUI_POSITION_STATIC, OUI_POSITION_RELATIVE,
    OUI_POSITION_ABSOLUTE, OUI_POSITION_FIXED, OUI_POSITION_STICKY,
} OuiPosition;

typedef enum {
    OUI_FLEX_ROW, OUI_FLEX_ROW_REVERSE,
    OUI_FLEX_COLUMN, OUI_FLEX_COLUMN_REVERSE,
} OuiFlexDirection;

typedef enum {
    OUI_OVERFLOW_VISIBLE, OUI_OVERFLOW_HIDDEN,
    OUI_OVERFLOW_SCROLL, OUI_OVERFLOW_AUTO,
} OuiOverflow;

// ─── Initialization ─────────────────────────────────────────
typedef struct {
    const char* resource_pak_path;  // Path to content_shell.pak (NULL = auto-detect)
} OuiInitConfig;

OUI_EXPORT OuiStatus oui_init(const OuiInitConfig* config);
OUI_EXPORT void      oui_shutdown(void);

// ─── Document ───────────────────────────────────────────────
OUI_EXPORT OuiDocument* oui_document_create(int viewport_width, int viewport_height);
OUI_EXPORT void         oui_document_destroy(OuiDocument* doc);
OUI_EXPORT void         oui_document_set_viewport(OuiDocument* doc, int width, int height);
OUI_EXPORT OuiStatus    oui_document_layout(OuiDocument* doc);
OUI_EXPORT OuiStatus    oui_document_update_all(OuiDocument* doc);

// ─── Element lifecycle ──────────────────────────────────────
OUI_EXPORT OuiElement* oui_element_create(OuiDocument* doc, const char* tag);
OUI_EXPORT void        oui_element_destroy(OuiElement* elem);
OUI_EXPORT OuiElement* oui_document_body(OuiDocument* doc);

// ─── DOM tree ───────────────────────────────────────────────
OUI_EXPORT void oui_element_append_child(OuiElement* parent, OuiElement* child);
OUI_EXPORT void oui_element_remove_child(OuiElement* parent, OuiElement* child);
OUI_EXPORT void oui_element_insert_before(OuiElement* parent, OuiElement* child, OuiElement* before);
OUI_EXPORT OuiElement* oui_element_first_child(const OuiElement* parent);
OUI_EXPORT OuiElement* oui_element_next_sibling(const OuiElement* elem);
OUI_EXPORT OuiElement* oui_element_parent(const OuiElement* elem);

// ─── Generic style (accepts any CSS property/value as strings) ──
OUI_EXPORT OuiStatus oui_element_set_style(OuiElement* e, const char* property, const char* value);
OUI_EXPORT OuiStatus oui_element_remove_style(OuiElement* e, const char* property);
OUI_EXPORT void      oui_element_clear_styles(OuiElement* e);

// ─── Typed convenience setters ──────────────────────────────
// Layout dimensions
OUI_EXPORT void oui_element_set_width(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_height(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_min_width(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_min_height(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_max_width(OuiElement* e, OuiLength len);
OUI_EXPORT void oui_element_set_max_height(OuiElement* e, OuiLength len);

// Box model
OUI_EXPORT void oui_element_set_margin(OuiElement* e, OuiLength top, OuiLength right, OuiLength bottom, OuiLength left);
OUI_EXPORT void oui_element_set_padding(OuiElement* e, OuiLength top, OuiLength right, OuiLength bottom, OuiLength left);

// Display & positioning
OUI_EXPORT void oui_element_set_display(OuiElement* e, OuiDisplay display);
OUI_EXPORT void oui_element_set_position(OuiElement* e, OuiPosition pos);

// Flexbox
OUI_EXPORT void oui_element_set_flex_direction(OuiElement* e, OuiFlexDirection dir);
OUI_EXPORT void oui_element_set_flex_grow(OuiElement* e, float grow);
OUI_EXPORT void oui_element_set_flex_shrink(OuiElement* e, float shrink);
OUI_EXPORT void oui_element_set_flex_basis(OuiElement* e, OuiLength basis);

// Colors
OUI_EXPORT void oui_element_set_color(OuiElement* e, uint32_t rgba);
OUI_EXPORT void oui_element_set_background_color(OuiElement* e, uint32_t rgba);

// Numeric
OUI_EXPORT void oui_element_set_opacity(OuiElement* e, float opacity);
OUI_EXPORT void oui_element_set_z_index(OuiElement* e, int z);

// ─── Text ───────────────────────────────────────────────────
OUI_EXPORT void oui_element_set_text_content(OuiElement* e, const char* text);
OUI_EXPORT void oui_element_set_font_family(OuiElement* e, const char* family);
OUI_EXPORT void oui_element_set_font_size(OuiElement* e, OuiLength size);
OUI_EXPORT void oui_element_set_font_weight(OuiElement* e, int weight);

// ─── Geometry queries (after layout) ────────────────────────
OUI_EXPORT float   oui_element_get_offset_x(const OuiElement* e);
OUI_EXPORT float   oui_element_get_offset_y(const OuiElement* e);
OUI_EXPORT float   oui_element_get_width(const OuiElement* e);
OUI_EXPORT float   oui_element_get_height(const OuiElement* e);
OUI_EXPORT OuiRect oui_element_get_bounding_rect(const OuiElement* e);

// ─── Computed style readback ────────────────────────────────
OUI_EXPORT const char* oui_element_get_computed_style(const OuiElement* e, const char* property);

// ─── Hit testing ────────────────────────────────────────────
OUI_EXPORT OuiElement* oui_document_hit_test(OuiDocument* doc, float x, float y);

// ─── Scroll ─────────────────────────────────────────────────
OUI_EXPORT float oui_element_get_scroll_width(const OuiElement* e);
OUI_EXPORT float oui_element_get_scroll_height(const OuiElement* e);

#ifdef __cplusplus
}
#endif
#endif // OPENUI_H_
```

## Deliverables

| Deliverable | Description |
|---|---|
| `openui/openui.h` | Complete C API header (stable ABI) |
| `openui/openui_impl.cc` | C++ implementation wrapping blink |
| `openui/openui_init.cc` | Blink bootstrap (extracted from rendering_test.cc) |
| `openui/openui_element_factory.cc` | Tag name → HTMLElement factory |
| `openui/BUILD.gn` | Updated with libopenui + test targets |
| `openui/openui_api_test.cc` | 80+ C++ GTest API tests |
| `openui/openui_c_test.c` | 20+ pure C consumer tests |

## Success Criteria

- [ ] `oui_init()` → `oui_document_create()` → `oui_element_create("div")` → set styles → `oui_document_layout()` → `oui_element_get_width()` returns correct value
- [ ] Flexbox layout: 3 children with `flex-grow: 1` in 900px container → each gets 300px
- [ ] Grid layout: 2×2 grid with `1fr 1fr` → correct cell positions
- [ ] Text wrapping: text in 200px container wraps at boundary
- [ ] Generic style API: `oui_element_set_style(e, "transform", "rotate(45deg)")` works
- [ ] Hit testing: click at (150, 150) returns correct overlapping element
- [ ] Pure C program links and runs correctly against `libopenui.so`
- [ ] 80+ C++ tests + 20+ C tests all passing
- [ ] Multi-agent review (Opus 4.6 + GPT 5.4) finds no issues
