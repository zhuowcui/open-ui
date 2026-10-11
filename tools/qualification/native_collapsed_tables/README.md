# Native collapsed-table qualification inputs

Pixels target **Chromium 147.0.7727.50**, with zero tolerance. This is source-only
preparation. No result here is a renderer, API or release qualification.

The 26 reviewed originals in `original/` retain their reference bytes and
SHA-256 values. `lower_guards.py` parses them offline and writes typed native
DOM/style descriptors in `fixtures.json`. The native example
`native_collapsed_table.rs` uses public `Document` and `Element` APIs. It has no
JavaScript runtime, HTML parser, CSS parser, hidden renderer state or private
Engine access. Its callbacks mutate retained native state on the UI thread.

The separately instrumented HTML in `oracle/` adds stable probe identities to
the HTML5-parsed DOM. Reparse checks preserve the original element tree, text
and author styles. Parser-inserted `tbody` nodes are recorded explicitly in the
native input. The capture owner must additionally prove that instrumentation
leaves every original pixel unchanged. Five separately versioned neighboring
inputs cover none/hidden borders, unequal conflict winners, shared column-group
joint order, section-limited rowspans and ordinary box-border painting.

Native output retains initial, repeated, callback and restored PNG bytes, every
public bounds/client-rectangle result, fixed hit targets, raw scroll dimensions
and offsets, reflected spans, callback targets and weak-handle cleanup. Real
continuation rectangles are retained. The two multicolumn cases must actually
produce table continuations. Callback failures and partial output are saved and
remain unqualified.

`run_matrix.py` is an exclusive owner to schedule after the prior owner has a
fresh audit and saved reaping receipt. It has **not been executed**. It requires
a complete checkout at a clean commit, hashes the Chromium/capture/font/source
inputs, builds an
external public consuming application with locked dependencies, requires fresh
compiler artifacts for every expected local package, and retains the binary
and artifact hashes. Compiler JSON and metadata use separate stdout/stderr
logs. It runs Engine, layout, paint, public span and conformance guards and
checks their named successful executions. Browser processes are stopped before
native raster commands begin. All commands record their actual exit status.
The default scope is 31 cases over the four original and forty focused profiles,
with two independent browser and native runs and three compared states.

The external consumer lock seed preserves the previously measured consuming
application dependency graph with its new package name. TOML was parsed; Cargo
has not verified it against the current merged workspace. A scheduled locked
metadata/build failure is an open gate and must be repaired before measurement.

Open qualification scope includes font selection/strikes, raw fractional native
versus integer browser scroll dimensions, empty-section geometry, source
ownership and effect-group counts, unchanged-frame zero-work counters, real
multicolumn continuation slicing and hit ordering. The public facade does not
expose its internal border-winner graph or effect-group counts; those require
the Engine/layout/paint guards. The owner always leaves `matrix_qualified`
false until a fresh independent audit and saved process reaping. Complete
focused/original/frozen/primitive evidence, hosted CI and release hardware gates
remain required beyond this consumer matrix.
