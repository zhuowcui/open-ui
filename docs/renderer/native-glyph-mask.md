# Native glyph masks and vertical font units

Open UI runs no JavaScript. Apps create and operate elements through public
native Rust APIs and Rust callbacks. Every needed browser-style element
operation remains native framework work. Pinned Chromium supplies expected
pixels and geometry; old Open UI images preserve history.

## Glyph correction and complete results

The native app reduces the earlier sizing failure to text inside a bordered
box. It uses public Rust methods, moves the box from a Rust callback, checks
owned geometry and handle teardown, and renders at five scales. Both runs
produce identical output. The same forty original Chromium captures and
geometry observations remain unchanged throughout these causal steps:

| Clean private source | Change | Native images exact | Native rectangles exact |
|---|---|---:|---:|
| `db177050` | Caption/shaping parent | 16/20 | 20/20 |
| `58647d67` | Remove rotated terminal-row repaint | 18/20 | 20/20 |
| `58ab7b42` | Shared physical aliased strike; remove font-specific ink clip | 20/20 | 20/20 |
| `4839e107` | Remove leading-row repaint; retain resolved aliased positioning | 20/20 | 20/20 |

The first strike trial passes the native app but loses eight focused and
sixteen primitive comparisons. Its full censuses are not run. The revised
`4839e107` passes the complete focused and primitive matrices and restores six
sizing neighbors. Its twenty-neighbor selection has eighteen exact comparisons
and two remaining caption differences.

Both complete censuses then finish on the same clean, unchanged source and
its verified executable:

| Suite | Exact | Different | Errors | Exact gains | Exact losses | Comparisons with more wrong pixels |
|---|---:|---:|---:|---:|---:|---:|
| Original | 21,432/22,924 | 1,492 | 0 | 101 | 3 | 5 |
| Expanded | 22,234/23,728 | 1,494 | 0 | 102 | 5 | 7 |
| Focused | 640/640 | 0 | 0 | 0 | 0 | 0 |
| Primitive | 960/960 | 0 | 0 | 0 | 0 | 0 |

Both full pixel gates exit 1. The whole owner also exits 1 because of
regressions. The correction remains **private, unapplied and unqualified**.
The accepted original result remains **21,334/22,924 exact**. No release
state is admitted. All Chromium image and oracle identities stay fixed.
The eighteen read-only source checks pass; workspace and C/C++ qualification
are not run on this rejected revision. Earlier parent results do not qualify it.

The regression review owns all seven unique worsened comparisons. Text and
paint own the glyph work, with native controls or positioned text as the
additional subsystem. The input has an extra black row. Large viewport-font
sticky text loses a row at fractional scales. Two existing select-control
differences also worsen. The evidence includes bounds, connected regions,
channel deltas and changes from the prior native image. Precise causes and
further reduced consuming Rust examples remain required. This review does not
close the formal release ownership ledger.

## Upright vertical `ch` through the native Rust API

Private `36b5db31` builds on the rejected glyph source without assuming that
parent is qualified. It resolves `ch` from the primary zero glyph in the
element's inline direction. A font without vertical tables supplies separately
rounded raw ascent and descent, excluding leading. Missing-zero fallbacks are
implemented, but missing-zero and metric-override cases are not runtime tested.

The unchanged consuming Rust app matches **1,440/1,440 full rectangles** at
five scales, with 160 gains and no losses. Both runs repeat identically and
execute 720 Rust callbacks each. Existing repeated Chromium inputs stay
unchanged. Its strict geometry gate exits 0 and all eighteen source checks pass.
The app captures no native pixels. The correction remains private and unapplied.
Automatic typed `Ch`, `Ex` and `Lh` declarations, complete inherited and nested
contexts, animation updates and C parity remain native API work.

## Outline investigation

A separate public Rust app draws a blue outline around one empty rectangle
and moves it from a Rust callback. Its neighboring size and five scales give
twenty states. All twenty rectangles match Chromium. Both native runs and all
forty independent Chromium captures repeat identically.

Only **15/20 images are exact**; five have one-step red/green differences at
corners. The strict gate exits 1. At scale 1.5, the empty rectangle differs at
the same coordinate as the caption, but with the opposite channel delta.
It exposes a shared outline coverage/rounding gap and **does not reproduce
the caption failure exactly**. Further context reduction is required. No
pixel patch, reference rewrite or tolerance is introduced.

The [completed evidence](generated/native-glyph-mask-v1.json) preserves clean
source reconstruction, native apps, actual exits, canonical complete reports,
every changed comparison's images, repeated captures, source checks, failed
preparations, pinned primary-source review and byte-preserving storage moves.
Archive members are freshly hash-verified. Full renderer, native API,
compositor, hardware and release qualification remain open.
