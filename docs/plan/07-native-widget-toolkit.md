# Sub-Project 7: Native Widget Toolkit & Platform Services

> Compiled, native widgets equivalent to HTML elements — powered by Chromium's rendering pipeline, with zero runtime parsing.

## Objective

Build a comprehensive widget toolkit where every standard HTML element has a native,
compiled equivalent. Applications built with Open UI should be able to render anything
a modern web app can — buttons, forms, tables, lists, dialogs — but everything is ahead-of-time
compiled native code. No HTML parser, no CSS parser, no DOM, no JavaScript engine.

Users write code (in C, Rust, or any language) that constructs widget trees. The framework
handles layout (via LayoutNG from SP4), styling (via the style system from SP5),
compositing (via cc/ from SP3), and rasterization (via Skia from SP2).

This sub-project also includes the **minimal platform services** required to make
widgets functional: text editing, clipboard, IME, cursor management, and focus handling.

## Architecture

```
┌───────────────────────────────────────────────────────────────┐
│                     Application Code                          │
│                 oui_button("Submit")                          │
│                 oui_text_input(&config)                       │
│                 oui_table(&rows, &cols)                       │
├───────────────────────────────────────────────────────────────┤
│              SP7: Native Widget Toolkit                       │
│  ┌──────────────┐  ┌───────────────┐  ┌──────────────────┐  │
│  │ Widget Types  │  │ Theme Engine  │  │ Platform Services│  │
│  │ (compiled     │  │ (control      │  │ (clipboard, IME, │  │
│  │  structs)     │  │  painting)    │  │  focus, cursor)  │  │
│  └──────┬───────┘  └──────┬────────┘  └──────┬───────────┘  │
│         │                  │                   │              │
├─────────┴──────────────────┴───────────────────┴─────────────┤
│              SP6: Scene Graph & Pipeline                      │
│         (Node tree, diffing, event dispatch, threading)       │
├──────────┬──────────┬───────────────┬────────────────────────┤
│  Style   │  Layout  │  Compositor   │       Skia             │
│  (SP5)   │  (SP4)   │    (SP3)      │      (SP2)             │
└──────────┴──────────┴───────────────┴────────────────────────┘
```

## Design Principles

1. **Every widget is a compiled struct** — no runtime interpretation
2. **Widgets compose** — they're just scene graph nodes with behavior
3. **Style properties are programmatic** — no CSS string parsing
4. **HTML semantics without HTML** — same visual/behavioral contracts
5. **Extract, don't reimplement** — reuse Chromium's rendering code where clean
6. **Progressive complexity** — simple widgets first, complex ones later

## Chromium Extraction Strategy

Chromium renders form controls through a 3-layer architecture. We extract the
DOM-free layers:

| Chromium Layer | DOM Coupling | Our Strategy |
|---|---|---|
| `WebThemeEngine` | **NONE** — pure paint interface | **Extract directly** — takes Part enum, State, Rect, ExtraParams |
| `ThemePainter` | Medium — needs Element for state | **Adapt** — replace Element queries with our widget state structs |
| `LayoutTheme` | Tight — adjusts ComputedStyle | **Reimplement** — encode default styles per widget type |

Key file: `third_party/blink/public/platform/web_theme_engine.h` (292 lines) — this
is the DOM-free interface that actually paints controls. It takes:
- `Part` enum (checkbox, radio, button, slider, progress, etc.)
- `State` flags (normal, hover, pressed, disabled)
- `gfx::Rect` for bounds
- `ExtraParams` union for control-specific data (checked state, slider value, etc.)

---

## Widget Catalog

### Phase A: Container & Text Widgets

These map directly to SP6 scene graph nodes. Minimal new code — mostly default
style presets.

| Widget | HTML Equivalent | Implementation |
|---|---|---|
| `oui_box` | `<div>`, `<section>`, `<article>`, etc. | `OUI_NODE_BOX` — already exists in SP6 |
| `oui_text` | `<p>`, `<span>`, `<h1>`–`<h6>`, `<pre>`, `<code>` | `OUI_NODE_TEXT` with style presets |
| `oui_link` | `<a>` | Text node + cursor change + click handler |
| `oui_image` | `<img>` | `OUI_NODE_IMAGE` — already exists in SP6 |
| `oui_hr` | `<hr>` | Box with border-top, zero height |
| `oui_canvas` | `<canvas>` | `OUI_NODE_CUSTOM` — already exists in SP6 |

**Default style presets** (compiled, not parsed):
```c
// Equivalent to user-agent stylesheet defaults — but as compiled structs
static const OuiStylePreset PRESET_H1 = {
    .font_size = { .value = 2.0, .unit = OUI_UNIT_EM },
    .font_weight = OUI_FONT_WEIGHT_BOLD,
    .margin_top = { .value = 0.67, .unit = OUI_UNIT_EM },
    .margin_bottom = { .value = 0.67, .unit = OUI_UNIT_EM },
    .display = OUI_DISPLAY_BLOCK,
};
// ... PRESET_H2 through PRESET_H6, PRESET_P, PRESET_PRE, etc.
```

### Phase B: List & Table Widgets

Leverage LayoutNG's list and table layout algorithms from SP4.

| Widget | HTML Equivalent | Implementation |
|---|---|---|
| `oui_list` | `<ul>`, `<ol>` | Box with list-style layout mode |
| `oui_list_item` | `<li>` | Item with marker painting (bullet, number) |
| `oui_desc_list` | `<dl>`, `<dt>`, `<dd>` | Box with definition list layout |
| `oui_table` | `<table>` | Table layout container |
| `oui_table_row` | `<tr>` | Table row |
| `oui_table_cell` | `<td>`, `<th>` | Table cell (supports colspan/rowspan) |
| `oui_table_header` | `<thead>` | Table header group |
| `oui_table_body` | `<tbody>` | Table body group |
| `oui_table_footer` | `<tfoot>` | Table footer group |

**List marker rendering:**
```c
// Compiled enum, not CSS string
typedef enum {
    OUI_LIST_DISC,        // • (default for ul)
    OUI_LIST_CIRCLE,      // ○
    OUI_LIST_SQUARE,      // ■
    OUI_LIST_DECIMAL,     // 1. 2. 3. (default for ol)
    OUI_LIST_ALPHA_LOWER, // a. b. c.
    OUI_LIST_ALPHA_UPPER, // A. B. C.
    OUI_LIST_ROMAN_LOWER, // i. ii. iii.
    OUI_LIST_ROMAN_UPPER, // I. II. III.
    OUI_LIST_NONE,
} OuiListStyle;
```

### Phase C: Form Control Widgets

The core of this sub-project. Each widget is a compiled struct with built-in
state management, painting, and interaction handling.

#### C.1 Button

```c
OuiWidget* oui_button(const char* label);
OuiWidget* oui_button_with_icon(OuiImageSource icon, const char* label);

// Button automatically handles:
// - Hover state → visual feedback (extracted ThemePainter)
// - Active/pressed state → depression effect
// - Focus state → focus ring painting
// - Disabled state → greyed appearance
// - Keyboard activation (Enter/Space)
```

**Extraction source:** `WebThemeEngine::Paint(kPartButton, state, rect)`
**Chromium files:**
- `theme_painter.cc` → `PaintButton()` (dispatcher)
- `theme_painter_default.cc` → default button painting
- `web_theme_engine.h` → `kPartButton`, `ButtonExtraParams`

#### C.2 Text Input

```c
typedef struct {
    const char* placeholder;
    const char* initial_value;
    OuiTextInputType type;  // TEXT, PASSWORD, EMAIL, NUMBER, SEARCH, TEL, URL
    int max_length;          // 0 = unlimited
    bool readonly;
    bool disabled;
    OuiTextChangedCallback on_change;
    OuiTextSubmitCallback on_submit;  // Enter key
    void* userdata;
} OuiTextInputConfig;

OuiWidget* oui_text_input(const OuiTextInputConfig* config);
```

**Built-in behavior (compiled, not scripted):**
- Cursor rendering with blink animation
- Text selection (click-drag, Shift+Arrow, Ctrl+A)
- Copy/Cut/Paste (via platform clipboard service)
- Undo/Redo stack
- IME composition (via platform IME service)
- Placeholder text (greyed when empty)
- Password masking (• characters)
- Scrolling for text wider than input box

**Extraction source:**
- Painting: `WebThemeEngine::Paint(kPartTextField, ...)`
- Text editing: new implementation using Skia text shaping (HarfBuzz)
- Cursor/selection: custom paint over text layout

#### C.3 Textarea (Multi-line Text Input)

```c
typedef struct {
    const char* placeholder;
    const char* initial_value;
    int rows;        // visible rows (default 4)
    int cols;        // visible columns (default 40)
    bool readonly;
    bool disabled;
    OuiResizeMode resize;  // NONE, VERTICAL, HORIZONTAL, BOTH
    OuiTextChangedCallback on_change;
    void* userdata;
} OuiTextareaConfig;

OuiWidget* oui_textarea(const OuiTextareaConfig* config);
```

Extends text input with: line wrapping, vertical scrolling, line numbers (optional).

#### C.4 Checkbox

```c
OuiWidget* oui_checkbox(bool checked);
OuiWidget* oui_checkbox_with_label(bool checked, const char* label);

// State: checked, unchecked, indeterminate
// Callback: OuiCheckChangedCallback(bool new_checked, void* userdata)
```

**Extraction source:** `WebThemeEngine::Paint(kPartCheckbox, state, rect, ButtonExtraParams{checked, indeterminate})`

#### C.5 Radio Button

```c
OuiWidget* oui_radio(const char* group_name, bool selected);
OuiWidget* oui_radio_with_label(const char* group_name, bool selected, const char* label);

// Group management: only one radio per group can be selected
// Callback: OuiRadioChangedCallback(const char* group, int selected_index, void* userdata)
```

**Extraction source:** `WebThemeEngine::Paint(kPartRadio, state, rect, ButtonExtraParams{checked})`

#### C.6 Select / Dropdown

```c
typedef struct {
    const char* label;
    const char* value;
    bool disabled;
} OuiSelectOption;

OuiWidget* oui_select(const OuiSelectOption* options, size_t count, int selected_index);

// Renders: themed dropdown button + popup option list
// Popup: positioned overlay (uses compositor layer)
// Keyboard: Arrow keys to navigate, Enter to select, Escape to close
```

**Extraction source:**
- Button: `WebThemeEngine::Paint(kPartMenuList, ...)`
- Popup: new implementation using compositor overlay

#### C.7 Slider / Range

```c
typedef struct {
    float min;
    float max;
    float value;
    float step;            // 0 = continuous
    bool disabled;
    OuiOrientation orientation;  // HORIZONTAL, VERTICAL
    OuiSliderChangedCallback on_change;
    void* userdata;
} OuiSliderConfig;

OuiWidget* oui_slider(const OuiSliderConfig* config);
```

**Extraction source:**
- Track: `WebThemeEngine::Paint(kPartSliderTrack, ...)`
- Thumb: `WebThemeEngine::Paint(kPartSliderThumb, ..., SliderExtraParams{thumb_x, thumb_y})`

#### C.8 Progress Bar

```c
OuiWidget* oui_progress(float value);                  // 0.0–1.0 determinate
OuiWidget* oui_progress_indeterminate(void);            // animated indeterminate

// Extraction source: WebThemeEngine::Paint(kPartProgressBar, ..., ProgressBarExtraParams)
```

#### C.9 Toggle / Switch

```c
OuiWidget* oui_toggle(bool on);
// Modern alternative to checkbox for on/off states
// Custom paint (not in Chromium's WebThemeEngine — new implementation)
```

### Phase D: Interactive & Overlay Widgets

| Widget | HTML Equivalent | Implementation |
|---|---|---|
| `oui_details` | `<details>/<summary>` | Collapsible container with animated reveal |
| `oui_dialog` | `<dialog>` | Modal/non-modal overlay with backdrop |
| `oui_tooltip` | (title attribute) | Delayed popup on hover |
| `oui_menu` | `<menu>` / context menu | Popup menu with keyboard navigation |
| `oui_popover` | (popover attribute) | Positioned popup anchored to trigger |

### Phase E: Scroll & Virtual Widgets

| Widget | Description |
|---|---|
| `oui_scroll_view` | Scrollable container with native scroll bars |
| `oui_virtual_list` | Virtualized list for 100k+ items (recycles nodes) |
| `oui_virtual_table` | Virtualized table for large datasets |
| `oui_lazy_image` | Image that loads when scrolled into view |

---

## Platform Services

Minimal services required to support widgets. NOT a full web API — only what
widgets need to function.

### PS.1 Text Editing Engine

Core text editing logic shared by `oui_text_input` and `oui_textarea`.

**Components:**
- **Text buffer**: UTF-8 rope data structure for efficient editing
- **Cursor model**: Position (line, column), blink state, multi-cursor (stretch)
- **Selection model**: Anchor + focus positions, multi-range (stretch)
- **Edit operations**: Insert, delete, backspace, word-delete, line-delete
- **Undo/Redo**: Operation-based undo stack with grouping
- **Text measurement**: via Skia's HarfBuzz/ICU text shaping

**API:**
```c
OuiTextEditor* oui_text_editor_create(const OuiTextEditorConfig* config);
void           oui_text_editor_insert(OuiTextEditor* ed, const char* text);
void           oui_text_editor_delete(OuiTextEditor* ed, OuiDeleteDirection dir);
void           oui_text_editor_select(OuiTextEditor* ed, OuiSelectionRange range);
const char*    oui_text_editor_get_text(OuiTextEditor* ed);
OuiTextLayout* oui_text_editor_get_layout(OuiTextEditor* ed);  // for painting
void           oui_text_editor_undo(OuiTextEditor* ed);
void           oui_text_editor_redo(OuiTextEditor* ed);
void           oui_text_editor_destroy(OuiTextEditor* ed);
```

### PS.2 Clipboard

```c
OuiStatus oui_clipboard_write_text(const char* text);
OuiStatus oui_clipboard_read_text(char* buf, size_t buf_size, size_t* out_len);
bool      oui_clipboard_has_text(void);
```

**Linux implementation:** X11 selections (XA_CLIPBOARD) or Wayland `wl_data_device`

### PS.3 Input Method Editor (IME)

For international text input (CJK, etc.):

```c
void oui_ime_enable(OuiWindow* window, OuiRect cursor_rect);
void oui_ime_disable(OuiWindow* window);
void oui_ime_update_cursor(OuiWindow* window, OuiRect cursor_rect);
// IME composition events flow through the event system
```

**Linux implementation:** IBus or Fcitx via XIM/IME protocols

### PS.4 Cursor Management

```c
typedef enum {
    OUI_CURSOR_DEFAULT,
    OUI_CURSOR_POINTER,     // hand (for links/buttons)
    OUI_CURSOR_TEXT,         // I-beam (for text inputs)
    OUI_CURSOR_CROSSHAIR,
    OUI_CURSOR_MOVE,
    OUI_CURSOR_RESIZE_NS,   // ↕
    OUI_CURSOR_RESIZE_EW,   // ↔
    OUI_CURSOR_RESIZE_NESW, // ⤡
    OUI_CURSOR_RESIZE_NWSE, // ⤢
    OUI_CURSOR_NOT_ALLOWED,
    OUI_CURSOR_GRAB,
    OUI_CURSOR_GRABBING,
    OUI_CURSOR_WAIT,
    OUI_CURSOR_PROGRESS,
} OuiCursorType;

void oui_window_set_cursor(OuiWindow* window, OuiCursorType cursor);
```

Widgets automatically set cursor on hover (text inputs → TEXT, buttons → POINTER, etc.)

### PS.5 Focus & Tab Navigation

```c
// Focus is managed by the scene graph (SP6) but widgets participate:
// - Tab order: widgets have tabindex-like ordering
// - Focus ring: painted by theme engine
// - Focus trapping: dialogs trap focus within
// - Arrow key navigation: within radio groups, select options, menus

typedef struct {
    int tab_index;    // -1 = not tabbable, 0 = natural order, >0 = explicit order
    bool auto_focus;
    bool trap_focus;  // for dialogs
} OuiFocusConfig;
```

### PS.6 Timers

```c
// Simple timer API for widget internals (cursor blink, tooltip delay, etc.)
OuiTimerId oui_timer_set(uint32_t ms, OuiTimerCallback cb, void* userdata);
OuiTimerId oui_timer_set_interval(uint32_t ms, OuiTimerCallback cb, void* userdata);
void       oui_timer_cancel(OuiTimerId id);
```

### PS.7 Scroll Physics

```c
// Scroll behavior is built into oui_scroll_view but exposed for custom use:
typedef struct {
    float friction;            // deceleration rate
    float velocity_threshold;  // minimum velocity to start fling
    bool  snap_enabled;        // snap to child boundaries
    float snap_offset;
    OuiScrollBarVisibility scrollbar;  // AUTO, ALWAYS, NEVER
} OuiScrollConfig;
```

---

## Styling Model

Widgets accept style properties programmatically. This is the compiled equivalent
of CSS — same properties, but as struct fields and function calls instead of parsed
strings.

### Style Properties (Subset of CSS)

All properties that the style system (SP5) extracts from Chromium are available.
Widgets use them through the same `OuiStyleRule` API from SP5/SP6:

```c
// Layout properties
oui_style_display(style, OUI_DISPLAY_FLEX);
oui_style_flex_direction(style, OUI_FLEX_ROW);
oui_style_justify_content(style, OUI_JUSTIFY_CENTER);
oui_style_align_items(style, OUI_ALIGN_CENTER);
oui_style_gap(style, 16.0f, OUI_UNIT_PX);
oui_style_padding(style, 8.0f, OUI_UNIT_PX);
oui_style_margin(style, 0.0f, OUI_UNIT_AUTO);
oui_style_width(style, 100.0f, OUI_UNIT_PERCENT);
oui_style_height(style, 48.0f, OUI_UNIT_PX);

// Visual properties
oui_style_background_color(style, oui_color_hex("#1a1a2e"));
oui_style_border(style, 1.0f, OUI_BORDER_SOLID, oui_color_hex("#333"));
oui_style_border_radius(style, 8.0f);
oui_style_box_shadow(style, 0, 4, 8, oui_color_rgba(0, 0, 0, 0.3));
oui_style_opacity(style, 0.9f);
oui_style_overflow(style, OUI_OVERFLOW_HIDDEN);

// Text properties
oui_style_font_size(style, 16.0f, OUI_UNIT_PX);
oui_style_font_family(style, "Inter, sans-serif");
oui_style_font_weight(style, OUI_FONT_WEIGHT_BOLD);
oui_style_color(style, oui_color_hex("#ffffff"));
oui_style_text_align(style, OUI_TEXT_ALIGN_CENTER);
oui_style_line_height(style, 1.5f, OUI_UNIT_UNITLESS);
oui_style_text_decoration(style, OUI_TEXT_DECORATION_UNDERLINE);

// Transform & animation
oui_style_transform(style, oui_transform_rotate(45.0f));
oui_style_transition(style, "background-color", 200, OUI_EASE_IN_OUT);
```

### Theme System

Provides default visual appearance for all widgets. Extracted from Chromium's
`LayoutTheme` / `WebThemeEngine` defaults:

```c
typedef struct {
    // Colors
    OuiColor primary;
    OuiColor secondary;
    OuiColor background;
    OuiColor surface;
    OuiColor on_primary;
    OuiColor on_surface;
    OuiColor error;
    OuiColor disabled;
    OuiColor border;
    OuiColor focus_ring;

    // Typography
    const char* font_family;
    float font_size_base;
    float font_size_sm;
    float font_size_lg;

    // Spacing
    float spacing_xs;
    float spacing_sm;
    float spacing_md;
    float spacing_lg;

    // Shape
    float border_radius;
    float border_width;

    // Control sizes
    float control_height;
    float control_height_sm;
    float control_height_lg;
} OuiTheme;

// Built-in themes
const OuiTheme* oui_theme_light(void);
const OuiTheme* oui_theme_dark(void);
OuiTheme*       oui_theme_create(void);  // custom theme
void            oui_app_set_theme(OuiApp* app, const OuiTheme* theme);
```

### User-Agent Defaults

Every widget type has compiled default styles (equivalent to browser user-agent
stylesheet). These are `static const` structs — zero runtime cost:

```c
// Internal: compiled user-agent defaults per widget type
static const OuiWidgetDefaults BUTTON_DEFAULTS = {
    .display = OUI_DISPLAY_INLINE_FLEX,
    .align_items = OUI_ALIGN_CENTER,
    .justify_content = OUI_JUSTIFY_CENTER,
    .padding_h = 16.0f,
    .padding_v = 8.0f,
    .border_radius = 4.0f,
    .cursor = OUI_CURSOR_POINTER,
    .font_family = "system-ui",
    .font_size = 14.0f,
    .appearance = OUI_APPEARANCE_BUTTON,
};
```

---

## Task Breakdown

### Phase A: Foundation (depends on SP6)

| Task | Description |
|---|---|
| 7-A1 | Extract `WebThemeEngine` interface from Chromium — DOM-free paint API |
| 7-A2 | Implement `OuiThemeEngine` — our version of WebThemeEngine using Skia |
| 7-A3 | Build theme system — light/dark themes with customization API |
| 7-A4 | Create widget base class/struct — common state (hover, focus, disabled) + paint dispatch |
| 7-A5 | Implement compiled user-agent defaults — static style presets per widget type |
| 7-A6 | Integrate focus management — tab order, focus ring painting, keyboard navigation |

### Phase B: Container, Text & Structure Widgets

| Task | Description |
|---|---|
| 7-B1 | Semantic container widgets — section, article, header, footer, nav, main (style presets) |
| 7-B2 | Text widgets — h1-h6, p, pre, code, blockquote, em, strong, span (style presets) |
| 7-B3 | Link widget — cursor change, hover underline, click navigation callback |
| 7-B4 | List widgets — ul/ol/li with marker rendering (disc, decimal, alpha, roman) |
| 7-B5 | Horizontal rule — hr equivalent |
| 7-B6 | Table widgets — table/tr/td/th with border collapsing, colspan/rowspan |

### Phase C: Form Controls

| Task | Description |
|---|---|
| 7-C1 | Button widget — themed painting, hover/active/focus/disabled states |
| 7-C2 | Text editing engine — UTF-8 buffer, cursor, selection, undo/redo |
| 7-C3 | Text input widget — single-line, password masking, placeholder |
| 7-C4 | Textarea widget — multi-line, resize handle, line wrapping |
| 7-C5 | Checkbox widget — check/uncheck/indeterminate, themed painting |
| 7-C6 | Radio button widget — group management, themed painting |
| 7-C7 | Select/dropdown widget — popup menu, keyboard navigation |
| 7-C8 | Slider/range widget — track+thumb, drag interaction |
| 7-C9 | Progress bar widget — determinate + indeterminate animation |
| 7-C10 | Toggle/switch widget — modern on/off control |

### Phase D: Platform Services

| Task | Description |
|---|---|
| 7-D1 | Clipboard service — text read/write (X11 + Wayland) |
| 7-D2 | IME service — input method integration (IBus/Fcitx) |
| 7-D3 | Cursor management — system cursor changes per widget state |
| 7-D4 | Timer service — for cursor blink, tooltip delay, animations |
| 7-D5 | Scroll physics — smooth scroll, fling, snap, scroll bar painting |

### Phase E: Interactive & Overlay Widgets

| Task | Description |
|---|---|
| 7-E1 | Details/summary (collapsible) — animated expand/collapse |
| 7-E2 | Dialog widget — modal + non-modal, backdrop, focus trapping |
| 7-E3 | Tooltip widget — delay-based hover popup |
| 7-E4 | Context menu widget — right-click popup, keyboard navigation |
| 7-E5 | Popover widget — anchored popup positioning |

### Phase F: Virtual & Performance Widgets

| Task | Description |
|---|---|
| 7-F1 | Virtual list — recycling container for 100k+ items |
| 7-F2 | Virtual table — large dataset rendering with sort/filter |
| 7-F3 | Lazy image — viewport-triggered loading |

### Phase G: Integration & Testing

| Task | Description |
|---|---|
| 7-G1 | Widget gallery example — all widgets in one application |
| 7-G2 | Form example — registration form with validation |
| 7-G3 | Dashboard example — tables, charts, cards, navigation |
| 7-G4 | Accessibility audit — screen reader testing for all widgets |
| 7-G5 | Performance benchmark — 10k widgets, frame timing |
| 7-G6 | Documentation — widget API reference + usage guide |

---

## Example: Full Form Application

```c
#include <openui/openui.h>
#include <openui/widgets.h>

typedef struct {
    char name[256];
    char email[256];
    bool subscribe;
    int role_index;
} FormState;

OuiNodeDesc* build_form(FormState* state) {
    OuiNodeDesc* form = oui_box("form");
    oui_style(form, style_card);

    OuiNodeDesc* children[] = {
        // Heading
        oui_heading(1, "Sign Up"),

        // Name field
        oui_label("Name"),
        oui_text_input(&(OuiTextInputConfig){
            .placeholder = "Enter your name",
            .initial_value = state->name,
            .on_change = on_name_change,
            .userdata = state,
        }),

        // Email field
        oui_label("Email"),
        oui_text_input(&(OuiTextInputConfig){
            .type = OUI_INPUT_EMAIL,
            .placeholder = "you@example.com",
            .initial_value = state->email,
            .on_change = on_email_change,
            .userdata = state,
        }),

        // Checkbox
        oui_checkbox_with_label(state->subscribe, "Subscribe to newsletter"),

        // Dropdown
        oui_label("Role"),
        oui_select(
            (OuiSelectOption[]){
                { "Developer", "dev" },
                { "Designer", "design" },
                { "Manager", "mgr" },
            },
            3,
            state->role_index
        ),

        // Submit button
        oui_button("Create Account"),
    };

    oui_children(form, children, sizeof(children) / sizeof(children[0]));
    return form;
}
```

This code is fully compiled — no HTML, no CSS strings, no runtime parsing.
The layout, styling, and painting are all handled by the extracted Chromium
pipeline.

---

## Deliverables

| Deliverable | Description |
|---|---|
| `include/openui/widgets.h` | Widget C API header |
| `include/openui/theme.h` | Theme system C API header |
| `include/openui/platform.h` | Platform services C API header |
| `src/widgets/` | Widget implementations |
| `src/theme/` | Theme engine (extracted from Chromium) |
| `src/platform/` | Platform service implementations (Linux) |
| `examples/widget_gallery.c` | All widgets showcase |
| `examples/form_app.c` | Registration form example |
| `examples/dashboard.c` | Dashboard with tables/charts |
| `tests/widgets/` | Widget unit tests |
| `docs/widgets/` | Widget API reference |

## Success Criteria

- [ ] All Phase A-C widgets render correctly with light and dark themes
- [ ] Text input supports full editing: cursor, selection, copy/paste, undo/redo, IME
- [ ] Tab navigation works across all focusable widgets
- [ ] Form example works end-to-end (input → validate → submit)
- [ ] Virtual list handles 100k items at 60fps
- [ ] Screen reader can navigate widget gallery
- [ ] Widget gallery example < 20MB binary size (stripped)
- [ ] All widgets match visual fidelity of equivalent HTML elements

## Dependencies

- **SP6** (Scene Graph) — widgets are scene graph nodes
- **SP5** (Style System) — widgets use style properties
- **SP4** (Layout Engine) — widgets use layout algorithms
- **SP2** (Skia) — theme engine paints via Skia

## Notes

- **Deferred:** `<video>`, `<audio>`, `<svg>`, `<iframe>` — too complex for initial release.
  These may be added as extension modules later.
- **Date/time pickers:** Implement as composite widgets (text input + dialog) rather than
  extracting Chromium's shadow DOM approach.
- **File input:** Implement as button + native file dialog, not the HTML shadow DOM.
