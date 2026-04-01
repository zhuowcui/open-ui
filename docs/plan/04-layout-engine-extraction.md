# Sub-Project 4: Layout Engine Extraction

> Extract Blink's layout engine, decoupled from the DOM, as a standalone layout computation library.

## Objective

Produce `libopenui_layout.so` — a standalone layout engine that computes positions and sizes for a tree of styled nodes. This is the component that gives Chromium its pixel-perfect CSS layout: Block, Inline, Flexbox, Grid, Table, and more. We extract it without requiring a DOM, replacing `Element`/`Node` with our own `LayoutNode` abstraction.

## Background: Blink's Layout Architecture

Blink has two layout systems:
1. **Legacy layout** — The original layout engine, being phased out
2. **LayoutNG** — The next-generation layout engine, now handling most layout types

**We target LayoutNG exclusively.** It has cleaner interfaces, better performance, and is the active development focus.

### LayoutNG Key Concepts

- **NGLayoutInputNode** — Input to layout (wraps a `LayoutBox` which wraps a DOM `Element`)
- **NGConstraintSpace** — Available space + constraints for layout (like `MeasureSpec` in Android)
- **NGLayoutResult** — Output of layout: fragment tree with positions/sizes
- **NGPhysicalFragment** — Positioned box in the output fragment tree
- **NGLayoutAlgorithm** — Abstract base for layout algorithms (Block, Flex, Grid, etc.)
- **NGBlockNode** — Block-level input node
- **NGInlineNode** — Inline-level input (text runs, inline boxes)

### The DOM Coupling Problem

LayoutNG's input (`NGLayoutInputNode`) wraps `LayoutBox`, which wraps `Element`, which is a DOM node. This coupling is pervasive:
- Style is fetched via `Element::GetComputedStyle()`
- Children are iterated via DOM child traversal
- Text content comes from DOM `Text` nodes
- Layout queries the DOM for attributes (e.g., `<table>` structure)

**Our job: replace this coupling with a standalone `LayoutNode` that provides the same information without a DOM.**

## Tasks

### 4.1 Extract Layout Algorithms

**Target algorithms from `third_party/blink/renderer/core/layout/ng/`:**

| Algorithm | Source | Complexity |
|---|---|---|
| Block layout | `ng_block_layout_algorithm.cc` | Medium — foundational, well-defined |
| Inline layout | `ng_inline_layout_algorithm.cc`, `ng_line_breaker.cc` | High — text shaping, bidi, line breaking |
| Flexbox | `ng_flex_layout_algorithm.cc` | Medium — well-specified by CSS spec |
| Grid | `ng_grid_layout_algorithm.cc` | High — complex track sizing, placement |
| Table | `ng_table_layout_algorithm.cc` | Medium — legacy but necessary |
| Fragmentation | `ng_fragmentation_utils.cc` | Medium — for pagination/multi-column |
| Custom layout | New | We add this — user-defined layout callback |

**For each algorithm, extract:**
- The algorithm implementation
- Its constraint space inputs
- Its fragment/result outputs
- Helper utilities it depends on

### 4.2 Create LayoutNode Abstraction

Replace `Element` → `LayoutBox` → `NGLayoutInputNode` with our own:

```
OuiLayoutNode
├── style: OuiComputedStyle*          // From our style system (SP5), or set directly
├── children: OuiLayoutNode*[]        // Child nodes
├── layout_type: OuiLayoutType        // Block, Inline, Flex, Grid, Table, Custom
├── text_content: const char*         // For text nodes
├── intrinsic_size: OuiSize           // For replaced elements (images, etc.)
├── custom_layout: OuiLayoutCallback  // For custom layout algorithms
└── layout_result: OuiLayoutResult*   // Output after layout
```

**Adapter layer:**
- Create `OuiLayoutInputNode` that implements the interfaces LayoutNG expects
- This adapter reads from `OuiLayoutNode` instead of DOM `Element`
- Map our property names to LayoutNG's internal property access patterns

### 4.3 Text Infrastructure

Text layout is deeply intertwined with the layout engine:

**HarfBuzz text shaping:**
- Extract Blink's HarfBuzz integration (`platform/fonts/shaping/`)
- Shape text runs with font fallback
- Handle complex scripts (Arabic reshaping, Indic conjuncts, CJK vertical text)
- Cache shaped results

**ICU for Unicode:**
- Line breaking (`ICU BreakIterator`)
- Bidirectional text (`ICU BiDi`)
- Unicode character properties
- Segmentation (grapheme clusters, word boundaries)

**Font infrastructure:**
- Font enumeration (fontconfig on Linux)
- Font matching (family, weight, style, stretch)
- Font metrics (ascent, descent, line height, cap height)
- Font fallback chains
- Emoji rendering (color fonts, COLR/CPAL, CBDT/CBLC)

**Line breaking (`ng_line_breaker.cc`):**
- This is one of the most complex components — it handles:
  - Soft hyphens, word breaks, break opportunities
  - CJK line break rules
  - White space collapsing
  - Floats interacting with lines
  - Inline-level box breaking

### 4.4 C API Design (`include/openui/openui_layout.h`)

```c
// === Layout Node Tree ===
OuiLayoutNode* oui_layout_node_create(OuiLayoutType type);
void           oui_layout_node_destroy(OuiLayoutNode* node);
void           oui_layout_node_add_child(OuiLayoutNode* parent, OuiLayoutNode* child);
void           oui_layout_node_remove_child(OuiLayoutNode* parent, OuiLayoutNode* child);
void           oui_layout_node_set_text(OuiLayoutNode* node, const char* text, size_t len);
void           oui_layout_node_set_image_size(OuiLayoutNode* node, int width, int height);

// === Style Properties (direct setting, or use SP5 style system) ===
void oui_layout_node_set_display(OuiLayoutNode* node, OuiDisplay display);
void oui_layout_node_set_position(OuiLayoutNode* node, OuiPosition position);
void oui_layout_node_set_width(OuiLayoutNode* node, OuiLength width);
void oui_layout_node_set_height(OuiLayoutNode* node, OuiLength height);
void oui_layout_node_set_min_width(OuiLayoutNode* node, OuiLength width);
void oui_layout_node_set_max_width(OuiLayoutNode* node, OuiLength width);
void oui_layout_node_set_margin(OuiLayoutNode* node, OuiEdges margin);
void oui_layout_node_set_padding(OuiLayoutNode* node, OuiEdges padding);
void oui_layout_node_set_border_width(OuiLayoutNode* node, OuiEdges border);

// Flexbox
void oui_layout_node_set_flex_direction(OuiLayoutNode* node, OuiFlexDirection dir);
void oui_layout_node_set_flex_wrap(OuiLayoutNode* node, OuiFlexWrap wrap);
void oui_layout_node_set_flex_grow(OuiLayoutNode* node, float grow);
void oui_layout_node_set_flex_shrink(OuiLayoutNode* node, float shrink);
void oui_layout_node_set_flex_basis(OuiLayoutNode* node, OuiLength basis);
void oui_layout_node_set_align_items(OuiLayoutNode* node, OuiAlign align);
void oui_layout_node_set_align_self(OuiLayoutNode* node, OuiAlign align);
void oui_layout_node_set_justify_content(OuiLayoutNode* node, OuiJustify justify);
void oui_layout_node_set_gap(OuiLayoutNode* node, OuiLength row_gap, OuiLength column_gap);

// Grid
void oui_layout_node_set_grid_template_columns(OuiLayoutNode* node, const OuiTrackList* tracks);
void oui_layout_node_set_grid_template_rows(OuiLayoutNode* node, const OuiTrackList* tracks);
void oui_layout_node_set_grid_column(OuiLayoutNode* node, OuiGridPlacement placement);
void oui_layout_node_set_grid_row(OuiLayoutNode* node, OuiGridPlacement placement);

// === Layout Computation ===
OuiStatus oui_layout_compute(OuiLayoutNode* root, float available_width, float available_height);

// === Layout Results ===
OuiRect oui_layout_node_get_rect(const OuiLayoutNode* node);      // Position + size
float   oui_layout_node_get_x(const OuiLayoutNode* node);
float   oui_layout_node_get_y(const OuiLayoutNode* node);
float   oui_layout_node_get_width(const OuiLayoutNode* node);
float   oui_layout_node_get_height(const OuiLayoutNode* node);
float   oui_layout_node_get_baseline(const OuiLayoutNode* node);
OuiSize oui_layout_node_get_scroll_size(const OuiLayoutNode* node); // Content overflow

// === Intrinsic Size Queries ===
float oui_layout_node_get_min_content_width(const OuiLayoutNode* node);
float oui_layout_node_get_max_content_width(const OuiLayoutNode* node);

// === Hit Testing ===
OuiLayoutNode* oui_layout_hit_test(OuiLayoutNode* root, float x, float y);

// === Custom Layout ===
typedef OuiLayoutResult (*OuiCustomLayoutFn)(OuiLayoutNode* node,
                                              float available_width, float available_height,
                                              void* userdata);
void oui_layout_node_set_custom_layout(OuiLayoutNode* node, OuiCustomLayoutFn fn, void* userdata);
```

### 4.5 Integration with Compositor

Map layout results to compositor layers:

```
Layout Tree                    Compositor Layer Tree
┌─────────┐                    ┌──────────────────┐
│ Root     │──────────────────▶│ Root Layer        │
│ (block)  │                    │                  │
│ ┌───────┐│                    │ ┌──────────────┐ │
│ │ Scroll ││──────────────────▶│ │ Scroll Layer │ │
│ │ ┌─────┐││                    │ │ ┌──────────┐ │ │
│ │ │ Box  │││──paint callback──▶│ │ │ Content  │ │ │
│ │ └─────┘││                    │ │ └──────────┘ │ │
│ └───────┘│                    │ └──────────────┘ │
└─────────┘                    └──────────────────┘
```

- Each layout node that creates a stacking context → compositor layer
- Scroll containers → scrollable compositor layers
- Transforms, opacity, clips → compositor property trees
- Paint callbacks use Skia C API to render node content

### 4.6 Validation

**Port Chromium's layout tests:**
- Select a representative subset of Web Platform Tests for layout
- Flexbox: css-flexbox-1 test suite (~500 tests)
- Grid: css-grid-1 test suite (~700 tests)
- Block layout: basic block formatting context tests
- Inline layout: text wrapping, line breaking, bidi

**Complex scenarios:**
- Nested flex in grid
- RTL and vertical writing modes
- Mixed text scripts (Latin + CJK + Arabic in one paragraph)
- Overflow and scrolling
- Percentage-based sizing with complex dependency chains
- `min-content` / `max-content` intrinsic sizes

**Performance:**
- Layout 10,000 nodes in < 16ms
- Incremental layout (change 1 node → re-layout only affected subtree)
- Layout cache hit rate for unchanged subtrees

## Deliverables

| Deliverable | Description |
|---|---|
| `libopenui_layout.so` | Layout engine shared library |
| `include/openui/openui_layout.h` | Public C header |
| `examples/layout_flex.c` | Flexbox layout demo with rendering |
| `examples/layout_grid.c` | Grid layout demo |
| `examples/layout_text.c` | Complex text layout demo |
| `tests/layout/` | Ported layout test suite |
| `benchmarks/layout/` | Layout performance benchmarks |

## Success Criteria

- [ ] Flexbox layout passes > 90% of CSS Flexbox test suite
- [ ] Grid layout passes > 90% of CSS Grid test suite
- [ ] Text layout handles Latin, CJK, Arabic, and emoji correctly
- [ ] Hit testing returns correct node for any point
- [ ] Layout of 10,000 nodes completes in < 16ms
- [ ] Incremental layout works (dirty subtree only)
- [ ] Integration with compositor: layout → paint → composite pipeline works end-to-end

## Key Risks

- **DOM coupling depth**: The layout engine's DOM dependency is extremely deep. Creating the LayoutNode adapter is the single hardest task in this sub-project.
- **Text complexity**: Inline layout and line breaking are enormously complex. We may need to accept reduced correctness initially and iterate.
- **LayoutNG instability**: LayoutNG is still evolving in Chromium. Our pinned version snapshot freezes this, but we lose upstream improvements.
