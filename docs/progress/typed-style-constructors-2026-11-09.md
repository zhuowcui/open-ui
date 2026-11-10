# view! style attributes emit typed constructors, 9 November 2026

`view!` now turns each static `style:prop="literal"` into the constructor
expression of its typed value. Before, it parsed the literal at compile time
only to validate it, then emitted a `parse_literal` call that parsed it again
at run time. This is the first A1 step of [RFC 0001](../alpha/rfcs/0001-typed-style-rules.md)
(plan unit A1, Workstream A): the same machinery will let `style!`, `css!`
and `stylesheet!` emit typed rules.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| New internal crate: `#[derive(ConstructorTokens)]` generates, for a value type, a function that returns an expression rebuilding a given value. A variant marked `#[constructor_tokens(unsupported)]` returns `Unsupported` instead. | `openui-style` (macro support) | `bindings/rust/openui-style-derive/src/lib.rs` |
| The `ConstructorTokens` trait, path helpers and leaf impls (floats as `f32::from_bits`/`f64::from_bits` so every bit pattern rebuilds, integers, `bool`, `char`, `String`, `Option`, `Box`, `Vec`, arrays, tuples). `Length` keeps private fields, so it is rebuilt through its `const fn` constructors and the result is compared bit for bit; a payload no constructor builds (`MinIntrinsic`) is `Unsupported`. | `openui-style` | `bindings/rust/openui-style/src/constructor_tokens.rs` |
| The derive is applied to the 188 types reachable from `StyleValue` through the variants a literal produces, behind the internal `constructor-tokens` feature. Only `openui-macros` enables it, so the runtime crates compile as before. `StyleValue::ColorExpression`, the four animation-only `TransformOperation` variants and `CssImage::Raster` (a per-document handle) are `Unsupported`. `RendererStyleValue` gets the attribute from the property generator. | `openui-style` | `color.rs`, `computed.rs`, `enums.rs`, `font_types.rs`, `layout_systems.rs`, `property.rs`, `typography.rs`; `tools/style/generate_properties.py` |
| `view!` emits `__el.set_property(Property, <constructor>)`; an `Unsupported` value is a compile error at the literal. | `openui-macros` | `bindings/rust/openui-macros/src/view.rs` (`typed_value_tokens`) |
| Release lists include the new crate before `openui-style`; the dependency policy hash follows the lockfile. | release tooling | `tools/release/build_v02_linux.py`, `.github/workflows/release-v02.yml`, `docs/v02/generated/dependency-policy.json` |

The values a literal can produce are unchanged: `view!` still accepts
exactly what `parse_literal` accepts.

## Verification

The hidden macro `openui_macros::__typed_style_literal_cases!` parses, at
compile time, every property's initial value that `parse_literal` accepts
and every line of `openui-macros/src/typed_style_corpus.txt`, and emits each
as `(property, literal, constructed value)`. Every corpus line must parse,
so the corpus cannot shrink silently.
`openui/tests/typed_style_constructors.rs` then checks:

- each constructed value equals the runtime `parse_literal` of its literal,
  both by `PartialEq` and by `Debug` text (which tells `-0.0` from `0.0` and
  covers the value types without `PartialEq`);
- every property with a literal grammar has at least one case. A property
  counts as having one if it is a stable property, a renderer property with
  an arm in the generated `parse_renderer_author_literal`, or any property
  that accepts one of a fixed list of probe literals (which catches the
  hand-written renderer parsers);
- a `view!` element reaches the expected computed style.

With the `f32` encoding deliberately off by one bit, the first and third
tests fail; restored, they pass.

| Check | Command | Result |
| --- | --- | --- |
| Constructor round trip, coverage, `view!` | `cargo test -p openui --test typed_style_constructors` (Windows) | 3 passed |
| Leaf and `Length` encodings | `cargo test -p openui-style --features constructor-tokens --lib constructor_tokens` | 3 passed |
| Touched crates | `cargo test -p openui-style -p openui-style-derive -p openui-macros -p openui` (Windows) | all passed |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,517 to 22,517 exact; 0 of 23,728 Open UI images changed |
| CSS Grid coverage | `run_renderer_matrix.py --suite coverage --ids-file tools/coverage/css_grid/manifest.json --jobs 16`, compared per test with the baseline | 2,218 to 2,218 exact; 0 of 4,204 measured Open UI images changed |

The paired expanded census in WSL has no change. The base is `9d7bccf6`,
the census base the [color functions](color-functions-2026-11-09.md)
record also uses; the head is `c73bd93f`, this change. Both are built with
the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium and Open UI
are each identical in all 23,728 comparisons: 22,517 exact before and
after, with nothing gained or lost. No census fixture goes through `view!`,
so this checks that the runtime crates are unchanged when the
`constructor-tokens` feature is off. The delta is in the
[census evidence](evidence/typed-style-constructors-census-v1.json).

The CSS Grid coverage suite has no change either
([coverage evidence](evidence/typed-style-constructors-coverage-v1.json)).
The base is `04d1eeab`, the recorded coverage baseline; the head is
`132a706a`, this change plus this record. In all 4,204 measured
comparisons the status and the Open UI image are unchanged: 2,218 exact
before and after. The two references that the baseline lists as
unmeasured, whose renderer memory is unbounded, are left out of both runs.

## Findings while building the corpus

`parse_literal` rejects these literals, so `view!` rejects them too. They are
recorded for later typed-style work, not changed here:

- a percentage alpha after the slash, such as `rgb(255 255 255 / 10%)`;
  a number alpha (`rgb(0 0 0 / 0.5)`) parses;
- `min-content`, `max-content` and `fit-content` for `width`, and `content`
  for `flex-basis`;
- `repeat(2, 1fr)` for `grid-template-columns`;
- `calc(50% - 10px)` for `width`;
- 61 of the 124 renderer properties have no arm in
  `parse_renderer_author_literal`; 10 of those have hand-written parsers,
  and the other 51 (for example `float`, `clear`, `aspect-ratio`,
  `box-shadow`, `content`) accept no literal, so `view!` cannot set them.
