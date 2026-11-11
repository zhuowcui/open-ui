# `style!` compiles CSS declaration blocks, 10 November 2026

`style! { padding: 8px 16px; color: red }` now builds a typed `Style` at
compile time. This is the second A1 step of
[RFC 0001](../alpha/rfcs/0001-typed-style-rules.md) (plan unit A1,
Workstream A), after [typed constructors](typed-style-constructors-2026-11-09.md).
It adds the declaration-block parser that `css!` and `stylesheet!` will
reuse for each rule's block.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| A macro-time declaration-block parser. It splits `property: value; ...` as CSS Syntax Level 3 does: comments, strings, and brackets do not split; property names are ASCII case-insensitive; `!important` is recognized in any case and spacing; whitespace runs outside strings collapse to one space. Each value goes through the runtime `parse_literal`, so the accepted values are exactly `view!`'s. Every invalid declaration is reported, not only the first. A browser drops an invalid declaration; a macro rejects the block. Nested rules and custom properties are errors. The parser exists only behind the internal `constructor-tokens` feature: the runtime still reads no CSS text. | `openui-style` | `bindings/rust/openui-style/src/declaration_block.rs` |
| `impl FromIterator<Declaration> for Style`, which the macro output uses. | `openui-style` | `bindings/rust/openui-style/src/property.rs` |
| `style!` emits `Style::from_iter([Declaration { property, value: <constructor> }, ...])`. It takes a braced block or one string literal. | `openui-macros` | `bindings/rust/openui-macros/src/style_macro.rs`, `lib.rs` |
| `openui` and its prelude re-export `style!`. | `openui` | `bindings/rust/openui/src/lib.rs`, `prelude.rs` |

### How the braced form reads CSS

Rust splits the macro input into tokens before `style!` sees it, which
loses the spacing CSS depends on: `space-between` and `a - b` lex to the
same tokens. The macro therefore reads its call's source text
(`Span::call_site().source_text()`, stable since Rust 1.66, so the 1.85
MSRV holds). It drops Rust `//` comments outside strings and brackets, and
accepts the text only if it lexes back to exactly the tokens the macro
received. Inside another macro the call site's text is not the CSS that was
passed, so the check fails and the error asks for the string form. A
`$css:literal` passed through `macro_rules!` works.

The string form, `style!("color: #3e3")`, takes CSS that Rust cannot lex,
such as hex colors like `#3e3` (an exponent with no digits) and
single-quoted strings.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Block parser | `cargo test -p openui-style --features constructor-tokens --lib declaration_block` (Windows) | 9 passed |
| Macro expansion and errors | `cargo test -p openui-macros style_macro` | 8 passed |
| `style!` against the runtime parse, applied to an element | `cargo test -p openui --test style_macro` | 6 passed |
| Touched crates | `cargo test -p openui-style -p openui-style-derive -p openui-macros -p openui` (Windows) | 427 passed, 0 failed; after rebasing onto `main` with PR #95, 448 passed, 0 failed, 12 ignored, and the same after rebasing onto PR #97 and again onto the rewritten typed style constructors (PR #109, head `9078594c`) and onto the rustix update (PR #116, `e80a0bc2`) |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check`; `generate_renderer_contract.py --check` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16`, with the CI `SKIA_GN_ARGS` | 22,517 to 22,517 exact; 0 of 23,728 Open UI images changed |
| CSS Grid coverage | `run_renderer_matrix.py --suite coverage --ids-file tools/coverage/css_grid/manifest.json --jobs 16`, compared per test with the baseline | 2,218 to 2,218 exact; 0 of 4,204 measured Open UI images changed |

`openui/tests/style_macro.rs` builds each block with `style!` and compares
it, by `PartialEq` and by `Debug` text, with a `Style` of the same
declarations parsed by `parse_literal` at run time. The braced test covers
hyphenated keywords, a negative length, percentages, a shorthand, a
slash-alpha color, a hex color and a quoted font family. Other tests cover
comments and line breaks, the string form, a `macro_rules!` wrapper, an
empty block, and an applied block reaching the computed style. The
expansion tests in `openui-macros` cover the error paths. There is no
compile-fail harness in the workspace, so these tests call the expansion
function directly.

The paired expanded census in WSL has no change. The base is `9d7bccf6`,
the census base the [typed style constructors](typed-style-constructors-2026-11-09.md)
record also uses; the head is `9141ebbd`, this change. Both are built with
the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium and Open UI
are each identical in all 23,728 comparisons: 22,517 exact before and
after, with nothing gained or lost. No census fixture goes through
`style!`, so this checks that the runtime crates are unchanged when the
`constructor-tokens` feature is off. The delta is in the
[census evidence](evidence/style-macro-census-v1.json).

The CSS Grid coverage suite has no change either
([coverage evidence](evidence/style-macro-coverage-v1.json)). The base is
`04d1eeab`, the recorded coverage baseline; the head is `9141ebbd`. In all
4,204 measured comparisons the status and the Open UI image are unchanged:
2,218 exact before and after. The two references that the baseline lists
as unmeasured, whose renderer memory is unbounded, are left out of both
runs.

## Not done here

- RFC 0001 asks for a compile warning on a duplicate longhand, or on a
  shorthand after a longhand it resets. Stable Rust has no warning API for
  proc macros, so this is not done; duplicates apply in order, as in CSS.
- `!important` is a compile error in `style!`. On an element-attached style
  it matters only against selector rules, which do not exist yet.
- Errors point at the whole macro call, not at the declaration: stable
  Rust cannot make a span for part of a token. Each message quotes the
  declaration.
- Keywords are case-sensitive in `parse_literal` (`display: FLEX` is
  rejected; Chromium accepts it). Property names are not.
