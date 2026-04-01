# Layout Engine Architecture — LayoutNG Deep Dive

> **Component:** `libopenui_layout`
> **Chromium Source:** `third_party/blink/renderer/core/layout/ng/`
> **Chromium Version:** M147 (`147.0.7727.24`)
> **Status:** Research (Sub-Project 1)

This document provides a comprehensive technical analysis of Blink's LayoutNG layout
engine — the system we extract into `libopenui_layout`. It catalogs every major
abstraction, algorithm, data flow, and external dependency so that we can plan a
precise extraction with minimal breakage.

---

## Table of Contents

1. [LayoutNG Overview](#1-layoutng-overview)
2. [Key Abstractions](#2-key-abstractions)
3. [Layout Algorithms](#3-layout-algorithms)
4. [The DOM Coupling Problem](#4-the-dom-coupling-problem)
5. [Text and Inline Layout](#5-text-and-inline-layout)
6. [Layout Result Caching](#6-layout-result-caching)
7. [Interaction with Style System](#7-interaction-with-style-system)
8. [Interaction with Paint](#8-interaction-with-paint)
9. [Open UI Extraction Notes](#9-open-ui-extraction-notes)

---

## 1. LayoutNG Overview

### 1.1 Why LayoutNG

Blink's original layout engine (sometimes called "legacy layout") evolved organically
from KHTML/WebKit over two decades. It suffered from deeply entangled state — layout
objects mutated themselves in place, making incremental layout fragile and
multi-column/fragmentation nearly impossible to get right.

LayoutNG is Chromium's ground-up replacement that enforces a **pure-functional layout
model**: given an input node and a constraint space, a layout algorithm produces an
immutable result. This design has several properties critical for Open UI:

- **Immutable outputs.** `NGPhysicalFragment` trees are immutable once created, making
  them safe to cache, share across threads, and feed directly into paint without
  defensive copies.
- **Explicit inputs.** All information flowing into layout is captured in
  `NGConstraintSpace`, eliminating hidden dependencies on global state.
- **Algorithm isolation.** Each formatting context (block, flex, grid, table, inline)
  is implemented as a self-contained `NGLayoutAlgorithm` subclass, making it possible
  to extract algorithms individually.
- **Fragment-based output.** The output is a tree of positioned fragments rather than
  mutated layout objects, cleanly separating "computing geometry" from "storing
  geometry."

### 1.2 Core Pipeline

The LayoutNG pipeline flows through five stages:

```
NGLayoutInputNode          What to lay out (wraps a LayoutBox)
        │
        ▼
NGConstraintSpace          How much space is available + environmental constraints
        │
        ▼
NGLayoutAlgorithm          The formatting-context-specific algorithm
        │
        ▼
NGLayoutResult             The output bundle (fragment + metadata)
        │
        ▼
NGPhysicalFragment         The positioned, immutable box/line/text fragment
```

In code, layout entry looks like:

```cpp
// third_party/blink/renderer/core/layout/ng/ng_layout_utils.cc
NGLayoutResult* NGLayoutUtils::Layout(
    NGLayoutInputNode node,
    const NGConstraintSpace& space,
    const NGBreakToken* break_token) {
  NGLayoutAlgorithm* algorithm = CreateLayoutAlgorithm(node, space, break_token);
  return algorithm->Layout();
}
```

Each call is **referentially transparent** for the same `(node, space, break_token)`
triple — a property we exploit for caching (§6).

### 1.3 Source Location

All LayoutNG code lives under:

```
third_party/blink/renderer/core/layout/ng/
├── ng_block_layout_algorithm.h / .cc
├── ng_inline_layout_algorithm.h / .cc
├── ng_flex_layout_algorithm.h / .cc
├── ng_grid_layout_algorithm.h / .cc
├── ng_table_layout_algorithm.h / .cc
├── ng_layout_result.h / .cc
├── ng_constraint_space.h / .cc
├── ng_constraint_space_builder.h / .cc
├── ng_physical_fragment.h / .cc
├── ng_physical_box_fragment.h / .cc
├── ng_fragment.h / .cc
├── ng_block_node.h / .cc
├── ng_inline_node.h / .cc
├── ng_layout_input_node.h
├── ng_break_token.h / .cc
├── ng_fragmentation_utils.h / .cc
├── inline/
│   ├── ng_inline_items_builder.h / .cc
│   ├── ng_line_breaker.h / .cc
│   ├── ng_inline_item.h / .cc
│   ├── ng_bidi_paragraph.h / .cc
│   └── ng_line_info.h / .cc
├── grid/
│   ├── ng_grid_layout_algorithm.h / .cc
│   ├── ng_grid_track_collection.h / .cc
│   └── ng_grid_placement.h / .cc
├── table/
│   ├── ng_table_layout_algorithm.h / .cc
│   ├── ng_table_section_layout_algorithm.h / .cc
│   ├── ng_table_row_layout_algorithm.h / .cc
│   └── ng_table_cell_layout_algorithm.h / .cc
├── flex/
│   └── ng_flex_layout_algorithm.h / .cc
└── mathml/
    └── ng_math_layout_algorithm.h / .cc
```

Legacy layout code that LayoutNG wraps lives one level up:

```
third_party/blink/renderer/core/layout/
├── layout_object.h / .cc        — Base class for all layout tree nodes
├── layout_box.h / .cc           — Box-model layout node
├── layout_block.h / .cc         — Block-level box
├── layout_inline.h / .cc        — Inline-level box
├── layout_text.h / .cc          — Text run
└── layout_box_model_object.h    — Box model base (margins, borders, padding)
```

---

## 2. Key Abstractions

### 2.1 NGLayoutInputNode

`NGLayoutInputNode` is the abstract base for everything that can be laid out. It is a
thin wrapper around a `LayoutBox*` pointer with a type tag.

```
// ng_layout_input_node.h
class NGLayoutInputNode {
  LayoutBox* box_;
  NGLayoutInputNodeType type_;  // kBlock, kInline, kColumnSpanAll
};
```

Two concrete subclasses exist:

| Class | Wraps | Used for |
|---|---|---|
| `NGBlockNode` | `LayoutBox` | Block-level, flex items, grid items, table cells |
| `NGInlineNode` | `LayoutBlockFlow` | The inline formatting context of a block |

**`NGBlockNode`** exposes the block-level interface:

```cpp
// ng_block_node.h
class NGBlockNode : public NGLayoutInputNode {
 public:
  NGLayoutResult* Layout(const NGConstraintSpace&,
                         const NGBreakToken*,
                         const NGEarlyBreak*) const;
  MinMaxSizesResult ComputeMinMaxSizes(WritingMode,
                                       const MinMaxSizesFloatInput&) const;
  NGBlockNode FirstChild() const;
  NGBlockNode NextSibling() const;
};
```

**`NGInlineNode`** wraps a `LayoutBlockFlow` and provides access to the flattened
inline item list (§5):

```cpp
// ng_inline_node.h
class NGInlineNode : public NGLayoutInputNode {
 public:
  const NGInlineItemsData& ItemsData(bool is_first_line) const;
  NGInlineNodeData& MutableData() const;
  void PrepareLayoutIfNeeded() const;   // builds inline items + shapes text
  bool IsBidiEnabled() const;
};
```

### 2.2 NGConstraintSpace

`NGConstraintSpace` captures **everything** the parent communicates to the child about
available space, writing mode, and formatting-context environment. It is conceptually
similar to Android's `MeasureSpec`, but far richer.

Key fields:

| Field | Type | Purpose |
|---|---|---|
| `available_size` | `LogicalSize` | Inline-size and block-size available (may be indefinite) |
| `percentage_resolution_size` | `LogicalSize` | Size used to resolve percentage lengths |
| `replaced_percentage_resolution_size` | `LogicalSize` | For replaced elements (images, etc.) |
| `bfc_offset` | `NGBfcOffset` | Current position within the Block Formatting Context |
| `exclusion_space` | `NGExclusionSpace` | Regions excluded by floats |
| `writing_mode` | `WritingMode` | Horizontal-tb, vertical-rl, vertical-lr, etc. |
| `direction` | `TextDirection` | LTR or RTL |
| `is_new_formatting_context` | `bool` | Whether this child establishes a new BFC |
| `is_fixed_inline_size` | `bool` | Inline-size is fixed (stretch, not shrink-to-fit) |
| `is_fixed_block_size` | `bool` | Block-size is fixed |
| `is_shrink_to_fit` | `bool` | Use intrinsic sizing |
| `is_intermediate_layout` | `bool` | Flex/grid intermediate pass, results won't be kept |
| `block_direction_fragmentation_type` | `NGFragmentationType` | Column or page fragmentation |
| `fragmentainer_block_size` | `LayoutUnit` | Size of the current fragmentainer |
| `fragmentainer_offset` | `LayoutUnit` | Offset within current fragmentainer |

`NGConstraintSpaceBuilder` constructs constraint spaces:

```cpp
// ng_constraint_space_builder.h
NGConstraintSpaceBuilder builder(parent_space, child_writing_mode, is_new_fc);
builder.SetAvailableSize(available);
builder.SetPercentageResolutionSize(pct_size);
builder.SetIsFixedInlineSize(true);
NGConstraintSpace child_space = builder.ToConstraintSpace();
```

**Open UI note:** `NGConstraintSpace` is almost entirely self-contained. It references
no DOM types. It is one of the cleanest extraction targets in the entire pipeline.

### 2.3 NGLayoutResult

`NGLayoutResult` bundles the output of a layout algorithm:

```cpp
// ng_layout_result.h
class NGLayoutResult : public RefCounted<NGLayoutResult> {
  scoped_refptr<const NGPhysicalFragment> physical_fragment_;
  LayoutUnit intrinsic_block_size_;        // content height before clamping
  NGBfcOffset bfc_line_offset_;            // for float placement
  NGBfcOffset bfc_block_offset_;           // resolved BFC offset
  NGExclusionSpace exclusion_space_;       // updated float exclusions
  NGBreakToken* break_token_;              // non-null if fragmented
  EBreakBetween initial_break_before_;     // break-before value
  EBreakBetween final_break_after_;        // break-after value
  Status status_;                          // kSuccess, kBfcBlockOffsetResolved, etc.
};
```

The `Status` enum is important — `kBfcBlockOffsetResolved` signals that margin
collapsing resolved the BFC offset and layout must be re-run with the resolved offset
(a two-pass protocol specific to block layout).

### 2.4 NGPhysicalFragment / NGLogicalFragment

Fragments are the **final output** of layout — an immutable tree of positioned boxes.

**`NGPhysicalFragment`** stores positions in physical coordinates (top-left origin):

```cpp
// ng_physical_fragment.h
class NGPhysicalFragment : public RefCounted<NGPhysicalFragment> {
  PhysicalSize size_;                      // width × height
  FragmentType type_;                      // kBox, kLine, kText
  NGPhysicalFragment* children_[];         // child fragments
  PhysicalOffset children_offsets_[];      // each child's physical offset
  const ComputedStyle& Style() const;
  const NGBreakToken* BreakToken() const;
};
```

Concrete subclasses:

| Class | Represents |
|---|---|
| `NGPhysicalBoxFragment` | A CSS box (block, flex item, grid item, table cell, etc.) |
| `NGPhysicalLineBoxFragment` | A line box within an inline formatting context |
| `NGPhysicalTextFragment` | A run of shaped text |

**`NGLogicalFragment`** is a lightweight view that re-interprets physical coordinates
in logical space (inline-start / block-start), respecting the writing mode. It is not
stored — it is created on-the-fly when needed:

```cpp
// ng_fragment.h
class NGLogicalFragment {
 public:
  NGLogicalFragment(WritingDirectionMode mode, const NGPhysicalFragment& fragment);
  LayoutUnit InlineSize() const;
  LayoutUnit BlockSize() const;
};
```

**Fragment tree structure** (example for a simple page):

```
NGPhysicalBoxFragment (viewport)
  └─ NGPhysicalBoxFragment (body)
       ├─ NGPhysicalBoxFragment (div.flex-container)
       │    ├─ NGPhysicalBoxFragment (flex-item-1)
       │    │    └─ NGPhysicalLineBoxFragment (line)
       │    │         └─ NGPhysicalTextFragment ("Hello")
       │    └─ NGPhysicalBoxFragment (flex-item-2)
       │         └─ NGPhysicalLineBoxFragment (line)
       │              └─ NGPhysicalTextFragment ("World")
       └─ NGPhysicalBoxFragment (div.grid-container)
            ├─ NGPhysicalBoxFragment (grid-item-1)
            └─ NGPhysicalBoxFragment (grid-item-2)
```

---

## 3. Layout Algorithms

Each CSS formatting context is implemented by a dedicated `NGLayoutAlgorithm` subclass.
They all follow the same interface:

```cpp
// ng_layout_algorithm.h
template <typename NGInputNodeType>
class NGLayoutAlgorithm {
 public:
  NGLayoutAlgorithm(NGLayoutAlgorithmParams params);
  const NGLayoutResult* Layout();
  MinMaxSizesResult ComputeMinMaxSizes();
 protected:
  NGInputNodeType Node() const;
  const NGConstraintSpace& ConstraintSpace() const;
  const ComputedStyle& Style() const;
};
```

### 3.1 Block Layout

**File:** `ng_block_layout_algorithm.h / .cc`

Block layout implements the CSS Block Formatting Context (BFC). It is the most common
algorithm and the most complex due to margin collapsing.

**Core responsibilities:**

1. **Child iteration.** Walk block-level children in document order.
2. **Margin collapsing.** Adjoining block margins collapse per CSS2 §8.3.1. LayoutNG
   tracks this via `NGMarginStrut` — a pair of positive and negative maximum margins
   that collapse when adjacent.
3. **Float interaction.** Floats are placed via `NGExclusionSpace`, which tracks
   rectangular regions excluded from inline content. The algorithm:
   - Adds floats to the exclusion space when encountered.
   - Passes the exclusion space down to children via `NGConstraintSpace`.
   - Clears past floats when `clear: left/right/both` is specified.
4. **BFC offset resolution.** When a block's BFC offset depends on its content (e.g.,
   margin collapsing with first child), layout may produce a
   `kBfcBlockOffsetResolved` result, requiring a re-layout pass with the resolved
   offset.
5. **Clearance.** `clear` forces the block below relevant floats, computed from the
   exclusion space.

**Simplified control flow:**

```
NGBlockLayoutAlgorithm::Layout()
  │
  ├─ for each child:
  │    ├─ Resolve margins (may collapse with previous sibling)
  │    ├─ Determine BFC offset for child
  │    ├─ If child is float → place in exclusion space
  │    ├─ If child establishes new BFC → create fresh constraint space
  │    ├─ child.Layout(child_constraint_space)
  │    ├─ Place child fragment at resolved offset
  │    └─ Update running block-size
  │
  ├─ Resolve intrinsic block-size
  ├─ Apply min-height / max-height clamping
  └─ Return NGLayoutResult with NGPhysicalBoxFragment
```

**Key helper classes:**

- `NGMarginStrut` — Accumulates collapsing margins.
- `NGExclusionSpace` — Tracks float exclusion rectangles for the BFC.
- `NGBfcOffset` — A (line-offset, block-offset) pair within the BFC.
- `NGUnpositionedFloat` — A float awaiting placement.

### 3.2 Inline Layout

**File:** `ng_inline_layout_algorithm.h / .cc`
**Subdirectory:** `ng/inline/`

Inline layout is the most complex algorithm because it must handle text shaping, bidi
reordering, line breaking, and the interaction of inline boxes with block-level
formatting. See §5 for the full text pipeline.

**Two-phase architecture:**

1. **Preparation** (`NGInlineNode::PrepareLayoutIfNeeded`):
   - Walks the inline-level DOM subtree.
   - Collects all inline items into a flat `NGInlineItemsData` list via
     `NGInlineItemsBuilder`.
   - Shapes text runs via HarfBuzz, producing `ShapeResult` objects.
   - Segments text for bidi via `NGBidiParagraph`.

2. **Line layout** (`NGInlineLayoutAlgorithm::Layout`):
   - Creates `NGLineBreaker` to consume items and find break opportunities.
   - For each line:
     - `NGLineBreaker::NextLine()` fills a `NGLineInfo` with items that fit.
     - Applies bidi reordering to the visual order.
     - Creates `NGPhysicalLineBoxFragment` containing `NGPhysicalTextFragment`s and
       inline box fragments.
     - Positions the line within the block.

**Key classes:**

| Class | File | Role |
|---|---|---|
| `NGInlineLayoutAlgorithm` | `ng_inline_layout_algorithm.cc` | Top-level inline layout |
| `NGLineBreaker` | `inline/ng_line_breaker.cc` | Finds line break points |
| `NGInlineItemsBuilder` | `inline/ng_inline_items_builder.cc` | DOM tree → flat item list |
| `NGInlineItem` | `inline/ng_inline_item.h` | One item (text, open-tag, close-tag, float, etc.) |
| `NGLineInfo` | `inline/ng_line_info.h` | Items + metrics for one line |
| `NGBidiParagraph` | `inline/ng_bidi_paragraph.cc` | Bidi segmentation via ICU `UBiDi` |

### 3.3 Flex Layout

**File:** `ng_flex_layout_algorithm.h / .cc` (also `flex/ng_flex_layout_algorithm.cc`)

Implements the CSS Flexible Box Layout specification (CSS Flexbox Level 1). The
algorithm closely follows the spec's 9-step procedure.

**Algorithm outline:**

```
NGFlexLayoutAlgorithm::Layout()
  │
  ├─ 1. Determine main axis and cross axis from flex-direction
  ├─ 2. Collect flex items (skip absolutely-positioned children)
  ├─ 3. Determine available main-size and cross-size
  │
  ├─ 4. For each flex line (after wrapping):
  │    ├─ a. Compute each item's flex base size and hypothetical main size
  │    │      (calls child.ComputeMinMaxSizes() for content-based sizes)
  │    ├─ b. Determine if items are frozen (inflexible) or flexible
  │    ├─ c. Resolve flexible lengths:
  │    │      ├─ Distribute free space by flex-grow (if positive free space)
  │    │      └─ Shrink items by flex-shrink (if negative free space)
  │    │      Loop until all items are frozen.
  │    ├─ d. Determine cross-size of each item
  │    ├─ e. Layout each item at determined main-size:
  │    │      child.Layout(flex_child_constraint_space)
  │    └─ f. Determine cross-size of the line (max of item cross-sizes)
  │
  ├─ 5. Resolve cross-sizes if container has definite cross-size
  ├─ 6. Align items on cross-axis (align-items / align-self)
  ├─ 7. Distribute main-axis space (justify-content)
  ├─ 8. Handle align-content for multi-line flex containers
  └─ 9. Return NGLayoutResult
```

**Key implementation details:**

- **Flex base size** resolves `flex-basis` which can be `auto`, a length, a
  percentage, or `content`. The `content` case triggers intrinsic size computation.
- **Two-pass layout.** Flex items are often laid out twice — once with an indefinite
  cross-size to determine the hypothetical main size, then again with the resolved
  cross-size. LayoutNG uses the `is_intermediate_layout` flag in `NGConstraintSpace`
  to mark the first pass.
- **Order property.** Items are laid out in `order`-sorted sequence, not DOM order.
  This requires sorting flex items before iteration.
- **Aspect ratios.** Replaced elements with aspect ratios need special handling when
  computing flex base sizes and cross-sizes.
- **Min/max constraints.** `min-width`, `max-width`, `min-height`, `max-height`
  interact with flex sizing and may trigger re-resolution of flexible lengths (the
  "clamped and frozen" loop).

### 3.4 Grid Layout

**Files:** `grid/ng_grid_layout_algorithm.h / .cc`, `grid/ng_grid_track_collection.h`,
`grid/ng_grid_placement.h`

Implements CSS Grid Layout Level 1 and Level 2 (subgrid).

**Algorithm outline:**

```
NGGridLayoutAlgorithm::Layout()
  │
  ├─ 1. Build the explicit grid from grid-template-rows / grid-template-columns
  │
  ├─ 2. Place grid items:
  │    ├─ Items with explicit placement (grid-row, grid-column)
  │    ├─ Items with partial placement (one axis specified)
  │    └─ Auto-placed items (grid-auto-flow: row | column | dense)
  │    Implemented by NGGridPlacement.
  │
  ├─ 3. Track sizing algorithm (CSS Grid §12.3):
  │    ├─ a. Initialize track sizes to base size / growth limit
  │    ├─ b. Resolve intrinsic track sizes:
  │    │      For each track sizing function (min-content, max-content,
  │    │      auto, fit-content, minmax, fr), distribute space from
  │    │      items that span only that track, then items spanning
  │    │      multiple tracks.
  │    ├─ c. Maximize tracks (distribute remaining space)
  │    ├─ d. Resolve fr units using leftover space
  │    └─ e. Stretch auto tracks if align-content / justify-content: stretch
  │    Implemented by NGGridTrackCollection.
  │
  ├─ 4. Layout each grid item:
  │    child.Layout(grid_child_constraint_space)
  │    Constraint space gives the item its resolved track sizes.
  │
  ├─ 5. Align items within their grid areas
  │    (justify-items, align-items, justify-self, align-self)
  │
  └─ 6. Return NGLayoutResult
```

**Subgrid:**

Subgrid (CSS Grid Level 2) allows a grid item to inherit track definitions from its
parent grid. Implementation:

- `NGGridLayoutAlgorithm` detects `grid-template-rows: subgrid` or
  `grid-template-columns: subgrid`.
- The subgridded axis inherits track sizes from the parent grid rather than defining
  its own.
- Track sizing for the parent must account for subgrid items' contributions, creating
  a dependency between parent and child grid sizing.

**Key implementation classes:**

| Class | Role |
|---|---|
| `NGGridLayoutAlgorithm` | Top-level grid layout |
| `NGGridPlacement` | Item placement (explicit, partial, auto) |
| `NGGridTrackCollection` | Track sizing and fr resolution |
| `NGGridNode` | Extends `NGBlockNode` with grid-specific data |

### 3.5 Table Layout

**Files:** `table/ng_table_layout_algorithm.h / .cc` and related per-level algorithms.

Table layout is split into four nested algorithms matching the table formatting model:

```
NGTableLayoutAlgorithm         — The <table> element
  └─ NGTableSectionLayoutAlgorithm  — <thead>, <tbody>, <tfoot>
       └─ NGTableRowLayoutAlgorithm      — <tr>
            └─ NGTableCellLayoutAlgorithm    — <td>, <th>
```

**Column width distribution:**

1. Compute minimum and maximum widths for every cell (calls
   `ComputeMinMaxSizes()`).
2. For each column, accumulate the minimum and maximum widths from cells
   spanning that column.
3. Distribute available table width to columns:
   - Fixed-width columns get their specified width.
   - Auto columns share remaining space proportionally to their max-content
     widths.
   - `table-layout: fixed` uses the simpler fixed-table-layout algorithm
     (first-row widths only).
4. Handle `colspan` by distributing spanned cell widths across the spanned
   columns.

**Row height distribution** follows a similar pattern: cell heights determine row
heights, then rows distribute within sections, then sections fill the table.

### 3.6 Fragmentation

**Files:** `ng_fragmentation_utils.h / .cc`, `ng_column_layout_algorithm.h / .cc`

Fragmentation handles breaking content across:

- **Multi-column layout** (`column-count`, `column-width`)
- **Pagination** (print, `@page`)

**Core mechanism: Break Tokens**

When content overflows a fragmentainer (column or page), layout returns an
`NGLayoutResult` with a non-null `NGBreakToken`. The parent then creates a new
fragmentainer and resumes layout from the break token:

```
while (break_token != nullptr) {
    result = child.Layout(constraint_space, break_token);
    PlaceFragment(result);
    break_token = result->BreakToken();
}
```

**`NGColumnLayoutAlgorithm`** orchestrates multi-column layout:

1. Create constraint space for a single column (width = column-width, height =
   column-fill determines whether columns balance).
2. Lay out content into the first column until overflow.
3. Use the break token to continue into the next column.
4. Repeat until all content is placed or column-count is reached.
5. Spanner elements (`column-span: all`) interrupt column flow and span the full
   multi-column container width.

**Break avoidance:**

- `break-before: avoid`, `break-after: avoid`, `break-inside: avoid`
- `widows`, `orphans` — minimum lines before/after a page break
- `NGEarlyBreak` — pre-scans content to find the best break point before committing

---

## 4. The DOM Coupling Problem

This is the central architectural challenge for Open UI. LayoutNG was built as part of
a web browser — it assumes a DOM exists. Extracting it means systematically severing
every DOM dependency.

### 4.1 How NGBlockNode Wraps LayoutBox Wraps Element

The dependency chain is:

```
NGBlockNode
  └─ LayoutBox (a LayoutObject subclass)
       └─ Node* (a DOM Node, usually an Element)
            └─ Document
                 └─ (frame, page, window, etc.)
```

`NGBlockNode` holds a `LayoutBox*`. `LayoutBox` inherits from `LayoutObject`, which
holds a `Node*` and a `ComputedStyle*`. The `Node*` points to the DOM element, which
in turn points to the `Document`, `LocalFrame`, `Page`, and the entire browser
object graph.

### 4.2 Where Layout Reads from DOM

Layout reads from the DOM in these specific patterns:

**Child iteration:**

```cpp
// LayoutBox / LayoutObject
LayoutObject* SlowFirstChild() const;
LayoutObject* NextSibling() const;
LayoutObject* SlowLastChild() const;

// NGBlockNode wraps this as:
NGLayoutInputNode FirstChild() const;
NGLayoutInputNode NextSibling() const;
```

The child iteration walks `LayoutObject` pointers, which mirror the DOM tree structure.
For inline content, `NGInlineNode::CollectInlines()` walks the subtree to build the
flat item list.

**Attribute queries:**

```cpp
// Reads from Element attributes:
Element::getAttribute(html_names::kColspanAttr)  // table colspan
Element::getAttribute(html_names::kRowspanAttr)  // table rowspan
HTMLTableElement::rules()                        // table rules
HTMLImageElement::naturalWidth()                 // replaced element intrinsic size
```

**Text content:**

```cpp
// LayoutText::GetText() returns the text content of a Text node:
LayoutText::TransformedText()  // after text-transform
LayoutText::OriginalText()     // before text-transform
```

**Document queries:**

```cpp
// Writing mode from the document element:
Document::GetWritingMode()
// Viewport size:
LocalFrameView::LayoutSize()
// Font resolution needs document context:
Document::GetStyleResolver()
```

### 4.3 Where Layout Reads from Style

Layout's primary style interface is through `ComputedStyle`. Every `LayoutObject` holds
a `scoped_refptr<const ComputedStyle>`. Layout algorithms access style via:

```cpp
const ComputedStyle& style = node.Style();  // NGBlockNode::Style()
```

See §7 for the complete style interface catalog.

### 4.4 Catalog of DOM Dependencies to Sever

| Dependency | Accessed via | Used for | Extraction strategy |
|---|---|---|---|
| DOM tree structure | `LayoutObject::FirstChild/NextSibling` | Child iteration | `LayoutNode` adapter with child list |
| Element attributes | `Element::getAttribute()` | Table colspan/rowspan | Store as layout node properties |
| Text content | `LayoutText::TransformedText()` | Inline layout / shaping | `TextNode` adapter with text string |
| Document | `Node::GetDocument()` | Viewport queries, font context | Inject via layout context object |
| LocalFrame | `Document::GetFrame()` | Viewport, scroll, zoom | Remove or inject via context |
| Replaced content | `LayoutReplaced::IntrinsicSize()` | Image/video sizing | Provide intrinsic size in adapter |
| Form controls | `LayoutTextControl::InnerEditorElement()` | Input/textarea | Out of scope (remove) |
| SVG | `SVGElement::*` | SVG layout | Out of scope for initial extraction |
| CSSOM | `CSSComputedStyleDeclaration` | getComputedStyle() | Not needed for layout |

### 4.5 Strategy: LayoutNode Adapter

We introduce a `LayoutNode` abstraction that provides the same interface as
`LayoutBox` / `LayoutObject` without requiring a DOM:

```cpp
// include/openui/layout_node.h (proposed)
class LayoutNode {
 public:
  virtual ~LayoutNode() = default;

  // Tree structure
  virtual LayoutNode* FirstChild() const = 0;
  virtual LayoutNode* NextSibling() const = 0;
  virtual LayoutNode* Parent() const = 0;

  // Style
  virtual const ComputedStyle& Style() const = 0;

  // Identity
  virtual bool IsInline() const = 0;
  virtual bool IsBlock() const = 0;
  virtual bool IsReplaced() const = 0;
  virtual bool IsText() const = 0;

  // Replaced element intrinsic size (images, etc.)
  virtual absl::optional<PhysicalSize> IntrinsicSize() const {
    return absl::nullopt;
  }

  // Text content (for text nodes)
  virtual StringView GetText() const { return StringView(); }

  // Table attributes
  virtual unsigned ColSpan() const { return 1; }
  virtual unsigned RowSpan() const { return 1; }
};
```

`NGBlockNode` would be modified to wrap a `LayoutNode*` instead of `LayoutBox*`, and
all DOM access would be rerouted through this interface. Application code creates
`LayoutNode` subclasses that supply tree structure, text, and style from whatever
source they use (a custom scene graph, Rust structs via FFI, etc.).

---

## 5. Text and Inline Layout

Inline layout is the most dependency-heavy part of the layout engine. This section
provides a full anatomy of the text pipeline.

### 5.1 Text Shaping Pipeline

```
Raw text string (UTF-16)
        │
        ▼
Text segmentation (script, language, font)
        │    ├─ Segments by Unicode script (Latin, Han, Arabic, etc.)
        │    ├─ Segments by font (font fallback boundaries)
        │    └─ Segments by text direction (bidi)
        ▼
HarfBuzz shaping (per segment)
        │    Input:  UTF-16 text + font + script + direction
        │    Output: glyph IDs + glyph advances + glyph offsets
        ▼
ShapeResult
        │    Stores shaped glyphs for a text run
        │    Supports subsetting (extracting a sub-range)
        │    Provides advance width for any character range
        ▼
NGInlineItem (type: kText)
        │    References the ShapeResult
        ▼
NGLineBreaker consumes items into lines
```

**Source files for text shaping:**

```
third_party/blink/renderer/platform/fonts/
├── font.h / .cc                     — Font facade
├── font_selector.h                  — Abstract font selection interface
├── font_fallback_list.h / .cc       — Prioritized list of font data
├── font_cache.h / .cc               — Global font instance cache
├── font_data_cache.h                — Caches SimpleFontData by description
├── font_description.h               — Font family, size, weight, style, etc.
├── simple_font_data.h / .cc         — Metrics for a single font face
├── shaping/
│   ├── shape_result.h / .cc         — Output of HarfBuzz shaping
│   ├── harfbuzz_shaper.h / .cc      — HarfBuzz shaping driver
│   ├── shape_result_view.h          — Lightweight sub-range view
│   ├── shape_result_buffer.h        — Concatenation of shape results
│   ├── caching_word_shaper.h / .cc  — Caches shaped words
│   └── caching_word_shape_iterator.h — Iterates words for shaping
└── opentype/
    ├── open_type_caps_support.h     — Small-caps, all-caps features
    └── font_settings.h              — OpenType feature settings
```

### 5.2 Font Selection

Font selection resolves a `FontDescription` (family list, size, weight, style) to a
concrete `SimpleFontData` object:

```
FontDescription ("Roboto", 16px, weight 400)
        │
        ▼
FontSelector::GetFontData(FontDescription)
        │    ├─ Tries each family in the font-family list
        │    ├─ Falls back to generic families (serif, sans-serif, monospace)
        │    └─ Falls back to platform default
        ▼
FontFallbackList
        │    ├─ Primary font: SimpleFontData for first matching family
        │    └─ Fallback chain: per-codepoint fallback (CJK, emoji, etc.)
        ▼
SimpleFontData
        │    ├─ Font metrics (ascent, descent, line-gap, x-height)
        │    ├─ HarfBuzz hb_font_t handle
        │    └─ Platform font handle (SkTypeface for Skia)
        ▼
FontCache (global cache, keyed by FontDescription + FontFaceCreationParams)
```

**Platform integration.** On Linux, font enumeration uses FontConfig
(`third_party/fontconfig/`). Font rasterization uses FreeType
(`third_party/freetype/`) through Skia's `SkTypeface_FreeType`.

### 5.3 Line Breaking

`NGLineBreaker` is responsible for fitting inline items into lines:

**File:** `ng/inline/ng_line_breaker.h / .cc`

**Algorithm:**

1. Start at the current position in the item list.
2. For each item:
   - If it's a text item, find break opportunities within the text using
     ICU's `ULineBreak` (line break properties) and the CSS `word-break` /
     `overflow-wrap` / `line-break` properties.
   - Measure the item's advance width using `ShapeResult::Width()`.
   - If the item fits on the current line, add it.
   - If it doesn't fit:
     - Try to break the text at an earlier break opportunity.
     - If `overflow-wrap: break-word`, break at any character boundary.
     - If no break opportunity exists, overflow the line.
3. Handle `text-align: justify` by recording stretch opportunities.
4. Handle `text-indent` on the first line.
5. Return `NGLineInfo` with the items for this line.

**Break opportunities:**

- **Soft hyphens** (`&shy;`) — break here and insert a hyphen glyph.
- **Word boundaries** — ICU `ULineBreak` property (`U_LINE_BREAK_SPACE`, etc.).
- **CJK** — break opportunities exist between most CJK characters.
- **CSS `word-break: break-all`** — break between any two characters.
- **CSS `overflow-wrap: anywhere`** — break at any point if no other break exists.

### 5.4 Bidi Text

Bidirectional text (mixing LTR and RTL scripts) is handled by:

**File:** `ng/inline/ng_bidi_paragraph.h / .cc`

1. `NGBidiParagraph` wraps ICU's `UBiDi` API.
2. During inline item preparation, `NGBidiParagraph::SetParagraph()` analyzes the
   entire paragraph text and assigns bidi embedding levels to each character.
3. `NGBidiParagraph::GetLogicalRuns()` returns runs of uniform bidi level.
4. Items are segmented by bidi level.
5. During line layout, `NGLineBreaker` handles bidi runs.
6. After line content is determined, visual reordering reorders runs according to the
   Unicode Bidirectional Algorithm (UBA, UAX #9).

**ICU dependency:** `third_party/icu/source/common/ubidi.c` — This is a hard
dependency we must extract alongside layout.

### 5.5 NGInlineItemsBuilder

**File:** `ng/inline/ng_inline_items_builder.h / .cc`

Converts the inline-level DOM subtree into a flat list of `NGInlineItem` objects:

```
DOM tree:
  <p>Hello <b>bold <i>italic</i></b> world</p>

NGInlineItem list:
  [0] kText       "Hello "         (style: p)
  [1] kOpenTag    <b>              (style: b)
  [2] kText       "bold "          (style: b)
  [3] kOpenTag    <i>              (style: b i)
  [4] kText       "italic"         (style: b i)
  [5] kCloseTag   </i>
  [6] kCloseTag   </b>
  [7] kText       " world"         (style: p)
```

**Item types:**

| `NGInlineItem::Type` | Meaning |
|---|---|
| `kText` | A shaped text run |
| `kControl` | Newline (`<br>`), tab |
| `kAtomicInline` | Inline-block, inline-table, replaced element |
| `kOpenTag` | Start of an inline box (e.g., `<span>`) |
| `kCloseTag` | End of an inline box |
| `kFloating` | A float encountered in inline context |
| `kOutOfFlowPositioned` | Absolutely-positioned element in inline context |
| `kBidiControl` | Unicode bidi control character (LRE, RLE, PDF, etc.) |
| `kListMarker` | List item marker (bullet, number) |

### 5.6 NGInlineLayoutAlgorithm

**File:** `ng_inline_layout_algorithm.h / .cc`

The top-level inline layout algorithm:

```
NGInlineLayoutAlgorithm::Layout()
  │
  ├─ Create NGLineBreaker for the inline item list
  │
  ├─ while (line_breaker.NextLine(&line_info)):
  │    │
  │    ├─ Apply bidi reordering to line_info items
  │    │
  │    ├─ Create NGPhysicalLineBoxFragment:
  │    │    ├─ For each text item → NGPhysicalTextFragment
  │    │    ├─ For each atomic inline → child NGPhysicalBoxFragment
  │    │    └─ For each inline box → adjust offsets for borders/padding
  │    │
  │    ├─ Compute line-height and baseline alignment:
  │    │    ├─ Find dominant baseline from line-height / font metrics
  │    │    ├─ Align children vertically (vertical-align)
  │    │    └─ Compute the line box height
  │    │
  │    ├─ Apply text-align (left, right, center, justify)
  │    │    Justify: distribute extra space at justification opportunities
  │    │
  │    └─ Position the line box in the block
  │
  └─ Return NGLayoutResult with all line box fragments
```

---

## 6. Layout Result Caching

### 6.1 Cache Architecture

LayoutNG caches `NGLayoutResult` objects to avoid re-laying-out unchanged subtrees.
The cache is stored on the `LayoutBox` itself:

```cpp
// layout_box.h (simplified)
class LayoutBox : public LayoutBoxModelObject {
  Vector<scoped_refptr<const NGLayoutResult>> layout_results_;
  // Multiple results when the box is fragmented (one per fragmentainer).
};
```

### 6.2 Cache Keys

A cached result can be reused if:

1. **Node identity** — same `LayoutBox`.
2. **Constraint space equality** — the `NGConstraintSpace` for the new layout matches
   the one used to produce the cached result. Key fields compared:
   - `available_size`
   - `percentage_resolution_size`
   - `is_fixed_inline_size`, `is_fixed_block_size`
   - `bfc_offset` (for blocks in a BFC)
   - `exclusion_space` (for blocks affected by floats)
   - Fragmentation state
3. **No dirty bits** — the `LayoutObject` does not have `NeedsLayout()` set.

The cache check is in `NGLayoutCacheStatus`:

```cpp
// ng_layout_utils.cc (simplified)
NGLayoutCacheStatus CalculateCacheStatus(
    const NGBlockNode& node,
    const NGConstraintSpace& new_space,
    const NGLayoutResult& cached_result) {
  if (node.GetLayoutBox()->NeedsLayout())
    return NGLayoutCacheStatus::kNeedsLayout;
  if (new_space != cached_result.GetConstraintSpaceForCaching())
    return NGLayoutCacheStatus::kConstraintSpaceChanged;
  return NGLayoutCacheStatus::kHit;
}
```

### 6.3 Cache Invalidation

The cache is invalidated when:

| Trigger | Mechanism |
|---|---|
| Style change | `LayoutObject::SetNeedsLayout()` propagates dirty bit up |
| DOM mutation | `LayoutObject::SetNeedsLayout()` on affected subtree |
| Constraint space change | Cache miss on space comparison |
| Viewport resize | Propagates new available size, causing constraint space mismatch |
| Font load | `FontSelector` notifies, style recalc triggers layout |
| Text content change | `LayoutText::SetText()` → `SetNeedsLayout()` |

### 6.4 Importance for Incremental Layout

Without caching, every frame would re-layout the entire tree — O(n) in DOM size.
With caching, only the **dirty path** (root → changed node) is re-laid-out. Siblings
with matching constraint spaces return cached results, pruning entire subtrees.

For Open UI, this means:

- We must preserve the caching infrastructure.
- Our `LayoutNode` adapter must provide stable identity for cache keys.
- We must provide a mechanism for applications to mark nodes dirty when their
  properties change (equivalent to `SetNeedsLayout()`).

---

## 7. Interaction with Style System

### 7.1 ComputedStyle Getters Used by Layout

Layout algorithms read from `ComputedStyle` through a large number of getters. These
getters collectively define the **style→layout interface** — the contract between
`libopenui_style` and `libopenui_layout`.

**Box model:**

| Getter | Returns | Used by |
|---|---|---|
| `Display()` | `EDisplay` | Algorithm selection (block, flex, grid, inline, etc.) |
| `Position()` | `EPosition` | static, relative, absolute, fixed, sticky |
| `Float()` | `EFloat` | left, right, none |
| `Clear()` | `EClear` | left, right, both, none |
| `Width()`, `Height()` | `Length` | Specified dimensions |
| `MinWidth()`, `MaxWidth()` | `Length` | Min/max constraints |
| `MinHeight()`, `MaxHeight()` | `Length` | Min/max constraints |
| `MarginTop()` ... `MarginLeft()` | `Length` | Margins |
| `PaddingTop()` ... `PaddingLeft()` | `Length` | Padding |
| `BorderTopWidth()` ... `BorderLeftWidth()` | `LayoutUnit` | Border widths |
| `BoxSizing()` | `EBoxSizing` | content-box vs border-box |
| `OverflowX()`, `OverflowY()` | `EOverflow` | visible, hidden, scroll, auto |

**Flexbox:**

| Getter | Used by |
|---|---|
| `FlexDirection()` | Main axis direction |
| `FlexWrap()` | Wrapping behavior |
| `FlexGrow()`, `FlexShrink()` | Flexibility factors |
| `FlexBasis()` | Base size |
| `JustifyContent()` | Main-axis alignment |
| `AlignItems()`, `AlignSelf()` | Cross-axis alignment |
| `AlignContent()` | Multi-line cross-axis alignment |
| `Order()` | Visual ordering |

**Grid:**

| Getter | Used by |
|---|---|
| `GridTemplateColumns()`, `GridTemplateRows()` | Track definitions |
| `GridAutoColumns()`, `GridAutoRows()` | Implicit track sizing |
| `GridAutoFlow()` | Auto-placement algorithm |
| `GridColumnStart()` ... `GridRowEnd()` | Item placement |
| `GridColumnGap()`, `GridRowGap()` | Gutters |
| `JustifyItems()`, `AlignItems()` | Default item alignment |

**Inline / Text:**

| Getter | Used by |
|---|---|
| `GetFont()` | Font for shaping and metrics |
| `LineHeight()` | Line box height calculation |
| `TextAlign()` | Horizontal alignment |
| `TextAlignLast()` | Last-line alignment |
| `TextIndent()` | First-line indentation |
| `WordBreak()` | Line break rules |
| `OverflowWrap()` | Overflow wrapping |
| `WhiteSpaceCollapse()` | Whitespace handling |
| `TextWrap()` | Text wrap mode |
| `Hyphens()` | Hyphenation |
| `Direction()` | Text direction (LTR / RTL) |
| `GetWritingMode()` | Horizontal / vertical writing |
| `VerticalAlign()` | Inline vertical alignment |
| `TextTransform()` | Uppercase, lowercase, capitalize |
| `LetterSpacing()` | Inter-character spacing |
| `WordSpacing()` | Inter-word spacing |
| `TextDecorationLine()` | Underline, overline, line-through |

**Fragmentation:**

| Getter | Used by |
|---|---|
| `BreakBefore()`, `BreakAfter()` | Forced / avoid breaks |
| `BreakInside()` | Avoid break inside |
| `Widows()`, `Orphans()` | Pagination constraints |
| `ColumnCount()`, `ColumnWidth()` | Multi-column |
| `ColumnGap()` | Column gutter |
| `ColumnFill()` | balance / auto |
| `ColumnSpan()` | all / none |

**Sizing:**

| Getter | Used by |
|---|---|
| `ContainIntrinsicWidth()`, `ContainIntrinsicHeight()` | `contain-intrinsic-size` |
| `AspectRatio()` | Preferred aspect ratio |
| `ContainerType()` | Container queries |
| `ContainSizeAxes()` | `contain: size` / `contain: inline-size` |

### 7.2 Style Change → Layout Invalidation Flow

```
Style recalculation
        │
        ▼
ComputedStyle diff (StyleDifference)
        │    Compares old and new ComputedStyle
        │    Determines: NeedsLayout, NeedsPaintInvalidation, etc.
        ▼
LayoutObject::SetNeedsLayout(reason)
        │    Sets self_needs_layout_ = true
        │    Propagates: sets child_needs_layout_ on ancestors up to root
        │    (LayoutObject::MarkContainerChainForLayout)
        ▼
LayoutView is marked as needing layout
        │
        ▼
Next frame: LocalFrameView::UpdateLayout()
        │    Walks from LayoutView down
        │    Only visits nodes with dirty bits
        │    Clears bits after layout completes
        ▼
Layout complete, dirty bits cleared
```

### 7.3 Dirty Bit Propagation

`LayoutObject` maintains two dirty flags:

```cpp
class LayoutObject {
  bool self_needs_layout_ : 1;        // This node's layout is stale
  bool child_needs_layout_ : 1;       // Some descendant needs layout
  bool needs_simplified_layout_ : 1;  // Only position changed, not size
  bool positioned_child_needs_layout_ : 1; // An OOF child needs layout
};
```

`SetNeedsLayout()` sets `self_needs_layout_` and walks up the ancestor chain setting
`child_needs_layout_`. This ensures the layout tree walk visits the dirty path.

**`SetNeedsSimplifiedLayout()`** is an optimization: if only the node's position
changed (not its size), it can be repositioned without full re-layout.

---

## 8. Interaction with Paint

### 8.1 Fragment Tree → Paint

After layout completes, the `NGPhysicalFragment` tree is the input to paint. Paint
walks the fragment tree (not the DOM or layout object tree) to generate display items.

```
NGPhysicalBoxFragment (root)
        │
        ▼
NGBoxFragmentPainter::Paint()
        │
        ├─ Paint background (PaintBoxDecorationBackground)
        ├─ Paint border (PaintBorder)
        ├─ For each child fragment:
        │    ├─ If NGPhysicalBoxFragment → recurse NGBoxFragmentPainter
        │    ├─ If NGPhysicalLineBoxFragment → NGLinePainter
        │    │    └─ For each text/inline child → NGTextFragmentPainter
        │    └─ If NGPhysicalTextFragment → NGTextFragmentPainter
        ├─ Paint outline (PaintOutline)
        └─ Paint overflow (scroll, clip)
```

### 8.2 Key Paint Classes

**Source:** `third_party/blink/renderer/core/paint/ng/`

| Class | File | Responsibility |
|---|---|---|
| `NGBoxFragmentPainter` | `ng_box_fragment_painter.cc` | Paints box fragments (background, border, children) |
| `NGTextFragmentPainter` | `ng_text_fragment_painter.cc` | Paints text fragments (glyph runs) |
| `NGTextPainter` | `ng_text_painter.cc` | Low-level text drawing (calls Skia) |
| `NGLinePainter` | (integrated) | Line box paint orchestration |
| `NGHighlightPainter` | `ng_highlight_painter.cc` | Selection, spelling, grammar highlights |
| `NGDecorationPainter` | `ng_decoration_painter.cc` | Text decorations (underline, etc.) |

### 8.3 Display Items

Paint generates a flat list of **display items** (draw commands):

```cpp
// Display item types relevant to layout output:
DrawingDisplayItem        — Skia picture (for backgrounds, borders, text)
ForeignLayerDisplayItem   — Composited layer reference
ScrollHitTestDisplayItem  — For hit-testing scrollable areas
```

These display items are consumed by the compositor (`cc/`) for tiling, rasterization,
and GPU compositing — which is `libopenui_compositor`'s domain.

### 8.4 Paint ↔ Layout Contract

Paint depends on these properties of the fragment tree:

1. **Fragment positions** — physical offsets for positioning draw operations.
2. **Fragment sizes** — for clipping, background sizing.
3. **ComputedStyle** — for visual properties (color, background, border-style,
   text-decoration, opacity, etc.).
4. **ShapeResult** — for drawing text glyphs.
5. **Ink overflow** — pre-computed bounding box including shadows, outlines, etc.

Paint does **not** re-read from the DOM. This is crucial for Open UI — it means the
fragment tree is a complete, self-contained input to paint.

---

## 9. Open UI Extraction Notes

### 9.1 Core Challenge: Replacing DOM with LayoutNode

The fundamental extraction task is replacing every DOM access in layout with a call
through our `LayoutNode` adapter interface (§4.5). This requires:

1. **Introducing `LayoutNode`** as an abstract base class.
2. **Modifying `NGBlockNode`** to hold `LayoutNode*` instead of `LayoutBox*`.
3. **Modifying `NGInlineNode`** to iterate inline children via `LayoutNode` instead of
   `LayoutObject` tree walking.
4. **Providing `LayoutContext`** — a replacement for `Document` that provides:
   - Viewport dimensions
   - Default writing mode / direction
   - Font selector / font cache access
   - Locale for line-breaking and hyphenation rules

### 9.2 What We Keep

These components transfer largely intact:

| Component | Notes |
|---|---|
| `NGBlockLayoutAlgorithm` | Core block layout. Minimal DOM dependency. |
| `NGFlexLayoutAlgorithm` | Self-contained flex spec implementation. |
| `NGGridLayoutAlgorithm` | Self-contained grid spec implementation. |
| `NGTableLayoutAlgorithm` | Requires attribute adapter for colspan/rowspan. |
| `NGInlineLayoutAlgorithm` | Heavy adaptation needed for text pipeline. |
| `NGConstraintSpace` | Nearly DOM-free already. Clean extraction target. |
| `NGConstraintSpaceBuilder` | Same. |
| `NGLayoutResult` | Minimal adaptation. |
| `NGPhysicalFragment` tree | The entire fragment hierarchy. |
| `NGBreakToken` | Fragmentation infrastructure. |
| `NGColumnLayoutAlgorithm` | Multi-column. |
| `NGExclusionSpace` | Float exclusions. |
| `NGMarginStrut` | Margin collapsing. |
| Logical/physical coordinate utilities | Writing mode transforms. |

### 9.3 What We Sever

| Dependency | Replacement |
|---|---|
| `LayoutObject` tree iteration | `LayoutNode` adapter tree |
| `Element` attribute access | Properties on `LayoutNode` |
| `Document` queries | `LayoutContext` object |
| `LocalFrame` / `LocalFrameView` | `LayoutContext` viewport info |
| `Node*` back-pointers | Optional opaque user-data pointer on `LayoutNode` |
| `LayoutObject::SetNeedsLayout()` | `LayoutNode::MarkDirty()` with ancestor propagation |
| `StyleResolver` | `libopenui_style` (extracted separately in SP5) |
| `V8` / JavaScript bindings | Not needed. |
| `DOMTokenList`, `NamedNodeMap`, etc. | Not needed. |

### 9.4 What We Extract Alongside

Layout cannot function without the text infrastructure. These must be co-extracted:

| Component | Source | Why |
|---|---|---|
| **HarfBuzz** | `third_party/harfbuzz-ng/` | Text shaping — no alternative. |
| **ICU** | `third_party/icu/` | Unicode properties, bidi, line breaking, locale. |
| **FreeType** | `third_party/freetype/` | Font loading and glyph rasterization on Linux. |
| **FontConfig** | `third_party/fontconfig/` | System font enumeration on Linux. |
| **Skia font backend** | `third_party/skia/` | `SkTypeface`, glyph metrics, rasterization. |
| **Font classes** | `blink/renderer/platform/fonts/` | `Font`, `FontSelector`, `SimpleFontData`, `FontFallbackList`, `FontCache`. |
| **Shaping classes** | `blink/renderer/platform/fonts/shaping/` | `HarfBuzzShaper`, `ShapeResult`, `CachingWordShaper`. |
| **Text utilities** | `blink/renderer/platform/text/` | `Character`, `TextBreakIterator`, `Hyphenation`. |

Additionally, layout depends on Chromium's `base/` library for:

- `base::span`, `base::CheckedNumeric` — safe arithmetic
- `WTF::String`, `WTF::Vector`, `WTF::HashMap` — collections and strings
- `scoped_refptr` — reference counting for fragments and results

These are handled by SP1 (ADR 003: base extraction strategy).

### 9.5 Key Risks

| Risk | Severity | Mitigation |
|---|---|---|
| **Inline layout complexity** | High | Inline layout touches the most DOM APIs (text content, inline box tree). Extract inline items builder first, validate with unit tests before proceeding to line breaking. |
| **Text shaping dependency depth** | High | HarfBuzz → ICU → FreeType → FontConfig → system fonts. The entire chain must work. Start with a minimal font setup (single font, no fallback) and incrementally add complexity. |
| **ComputedStyle surface area** | Medium | Layout reads ~100+ style properties. Define a `LayoutStyle` interface that exposes only what layout needs, rather than extracting all of `ComputedStyle`. |
| **WTF/base dependency** | Medium | Layout uses WTF types pervasively (`WTF::String`, `WTF::Vector`, `WTF::AtomicString`). Extract or shim these before attempting layout extraction. |
| **Caching correctness** | Medium | Cache invalidation is tied to `LayoutObject` dirty bits. Must reimplement on `LayoutNode` with identical semantics. |
| **Multi-pass layout** | Low | Flex and grid do multi-pass layout. The `is_intermediate_layout` flag and two-pass constraint space pattern must be preserved. |
| **Float interaction** | Low | Floats cross formatting context boundaries via `NGExclusionSpace`. Already well-encapsulated. |
| **Writing modes** | Low | Logical ↔ physical coordinate transforms are pervasive but mechanical. Already well-encapsulated in utility functions. |

### 9.6 Extraction Order

Recommended extraction sequence within SP4:

```
1. NGConstraintSpace + NGConstraintSpaceBuilder
   └─ Nearly zero DOM dependencies. Ideal first target.

2. NGPhysicalFragment hierarchy
   └─ Immutable output types. Depends on ComputedStyle (read-only).

3. NGLayoutResult + NGBreakToken
   └─ Bundles fragments with metadata.

4. NGBlockLayoutAlgorithm
   └─ First complete algorithm. Exercises child iteration.
   └─ Requires LayoutNode adapter for child traversal.

5. NGFlexLayoutAlgorithm
   └─ Self-contained. Validates multi-pass layout support.

6. NGGridLayoutAlgorithm
   └─ Self-contained. Validates track sizing extraction.

7. Font infrastructure (HarfBuzz, FontCache, ShapeResult)
   └─ Must precede inline layout.

8. NGInlineItemsBuilder + NGInlineItem
   └─ Requires LayoutNode adapter for inline tree + text content.

9. NGLineBreaker + NGBidiParagraph
   └─ Requires ICU, shaped text, break iterators.

10. NGInlineLayoutAlgorithm
    └─ Ties it all together. The final and most complex piece.

11. NGTableLayoutAlgorithm
    └─ Needs colspan/rowspan from LayoutNode adapter.

12. NGColumnLayoutAlgorithm (fragmentation)
    └─ Builds on break tokens from step 3.
```

---

## References

- [CSS Box Model (CSS2 §8)](https://www.w3.org/TR/CSS2/box.html)
- [CSS Flexbox Level 1](https://www.w3.org/TR/css-flexbox-1/)
- [CSS Grid Level 1](https://www.w3.org/TR/css-grid-1/)
- [CSS Grid Level 2 (Subgrid)](https://www.w3.org/TR/css-grid-2/)
- [CSS Multi-column Layout](https://www.w3.org/TR/css-multicol-1/)
- [CSS Writing Modes Level 4](https://www.w3.org/TR/css-writing-modes-4/)
- [Unicode Bidirectional Algorithm (UAX #9)](https://unicode.org/reports/tr9/)
- [HarfBuzz Documentation](https://harfbuzz.github.io/)
- [Chromium LayoutNG Design Document](https://docs.google.com/document/d/1uxbDh4uONFQOiGuiumlJBLGgO4KDWB8ZEkp7Rd47fw4/)
- [Chromium Source: `third_party/blink/renderer/core/layout/ng/`](https://source.chromium.org/chromium/chromium/src/+/main:third_party/blink/renderer/core/layout/ng/)
