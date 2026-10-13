# Pending ordered native mutation replay

The version 1 cohort adds three distinct `pending/native-mutation-v1/` pixel
builders. The existing WPT builders, original HTML, references and historical
measurement files remain inputs. These builders are diagnostic; admission
requires a clean Root source and exact current Chromium comparisons in all
four required profiles.

`javascript_native_mutation_ir.mjs` parses authored scripts with Acorn offline.
It never evaluates those scripts. The accepted straight-line operations are
literal target binding, creation of a div or span, literal style assignment or
removal, node append or prepend, and layout reads. Every operation keeps its
source position and order. Unhandled statements, calls, dynamic inputs,
external scripts and event handlers reject the complete input. Nonclassic
script types and script activation/ordering attributes such as `nomodule`
also reject; inert script data is never turned into a native mutation.

`native_mutation_codegen.py` emits public typed Engine calls. Layout reads call
`Engine::update` at their position in the stream. Prepend uses live children,
`insert_before` and `append_or_move_child`, preserving node identity and
argument order. Empty style assignments call `Engine::remove_property`.

The compiler keeps immutable stylesheet rules separate from raw inline
declarations. Following removal or insertion, it recascades the bounded
selector set and reapplies surviving authored literals in cascade order.
Removing an inline display override can therefore reveal a stylesheet display
value. An assignment shadowed by a stylesheet `!important` declaration also
restores the winning source. This is a bounded lowering bridge, not a general
CSS cascade implementation: at-rules, conditional style attributes, external
stylesheets, pseudo states, effect lifecycle and background layers reject.
Publication compares ordered declarations, so moving a node between rules
with the same raw values and different shorthand order still updates its
canonical native ownership.

The capture policy matches the existing legacy box-only harness. Raw font and
viewport units and `currentcolor` reach the shared native property parser;
computed snapshots do not replace them. The offline compiler accepts a
color-only background shorthand only when background layer ownership is
absent. Original reference files are hashed without modification.
Every initial and script-created node receives the existing capture harness's
hidden scrollbar policy through the typed native setter. Authored standard
scrollbar width/color precedence is outside this cohort and rejects lowering.

Regenerate and check from the repository root, using the original WPT tree:

```sh
python3 tools/wpt/native_mutation_codegen.py \
  --fixture wpt/css_break/out-of-flow-in-multicolumn-120 "$WPT_CSS/css-break/out-of-flow-in-multicolumn-120.html" \
  --fixture wpt/css_position/position-fixed-dynamic-transformed-sibling "$WPT_CSS/css-position/position-fixed-dynamic-transformed-sibling.html" \
  --fixture wpt/css_overflow/scrollbars-chrome-bug-001 "$WPT_CSS/css-overflow/scrollbars-chrome-bug-001.html" \
  --rust bindings/rust/pixel-compare/src/pending_native_mutation_v1.rs \
  --manifest docs/renderer/generated/pending-native-mutation-replay-v1.json
```

Append `--check` for a read-only comparison of both generated outputs.
`CHROMIUM_ACORN` can select the offline Acorn module. The manifest binds that
module, the frontend, generator, native property schema, original cases,
references and generated Rust bytes. Python checks cover fail-closed lowering
and ownership restoration. Engine guards cover live insertion, removal and
the intermediate layout barriers; native execution and pixel qualification
are separate Root gates.
