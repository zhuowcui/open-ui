---
id: 0080
title: Anonymous table parts borrow the table node
tags: layout, table, decoration, anonymous, census
status: active
created: 2026-10-25
updated: 2026-10-25
refs: bindings/rust/openui-layout/src/block.rs, docs/progress/table-anonymous-part-decoration-2026-10-25.md
---

Anonymous table sections/rows/cells carry their table's node_id. Any rule keyed on "child box with the same node" (e.g. the structural-wrapper skip_box_decoration rule for captioned tables in block.rs) must exclude same-node table parts, or the table loses its background. Fixing it gained 49 census comparisons (21,701 -> 21,750). Diagnosis tip: `pixel_compare debug <test>` dumps fragments; instrument paint_fragment_box_decoration / the Box arm of the main paint match to see skip_box_decoration.
