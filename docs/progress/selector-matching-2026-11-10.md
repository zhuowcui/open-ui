# Selector matching against the element tree, 10 November 2026

The engine matches typed selectors against its elements, as Chromium 147
does, for everything that depends only on the tree: types, IDs, classes,
attribute selectors, the four combinators, `:is()`, `:where()`, `:not()`,
`:has()`, `:nth-child()` with and without `of S`, `:first-child`,
`:last-child`, `:only-child`, `:empty` and `:root`. This is the fourth A1
step of [RFC 0001](../alpha/rfcs/0001-typed-style-rules.md). Pseudo-classes
that depend on interaction or form state are the next step. No style rule
is applied yet.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `Engine::matches_selector` and `matches_selector_list` match right to left from the subject, backtracking for the descendant and subsequent-sibling combinators, as Blink's `SelectorChecker`. `:has()` matches each relative selector with the anchor as its leftmost compound. | `openui-engine` | `bindings/rust/openui-engine/src/selector_match.rs` |
| Only authored elements take part: text nodes, generated pseudo-elements and the root `Viewport` node do not. The `Viewport` plays Blink's document node, so `:root` is its element child and that child counts for `:nth-child()` and `:only-child`. | `openui-engine` | same |
| Type selectors use each element kind's HTML local name (`Image` is `img`, `TableCell` is `td`). | `openui-engine` | same |
| Attribute values compare as Blink's `AttributeValueMatches` (`third_party/blink/renderer/core/css/selector_checker.cc`): `~=` with an empty or space-containing word, and `^=`, `$=`, `*=` with an empty value, match nothing. The HTML 4.01 attributes that `HTMLDocument::IsCaseSensitiveAttribute` lists (`type`, `dir`, `lang` and 43 others) compare ASCII case-insensitively without the `i` flag. | `openui-engine` | same |
| `:empty` counts any non-empty text child, whitespace included, as Blink does. | `openui-engine` | same |
| `:hover`, `:active`, the focus, form and `:open`/`:modal` pseudo-classes return `SelectorMatchError::UnsupportedPseudoClass`, never a silent non-match. | `openui-engine` | same |

## Verification

`tools/qualification/capture_selector_matching_oracle.py` builds one
fixture tree with DOM calls in the pinned Windows Chrome-for-Testing
147.0.7727.50 (after removing `head`, so that no parser fix-up adds
elements). It records the ids `document.querySelectorAll` returns for 64
selectors, 5 of which match nothing
([oracle](evidence/selector-matching-v1/chromium-windows.json)). The
fixture has every element kind the selectors name, legacy case-insensitive
attributes in upper case, an empty and a whitespace-only text child, and a
class attribute with tabs and repeated spaces.

| Check | Command | Result |
| --- | --- | --- |
| Chromium oracle | `python tools/qualification/capture_selector_matching_oracle.py --chrome E:\open-ui\out\chrome-win\win-147.0.7727.50\chrome-win64\chrome.exe` | 64 selectors |
| Matching against the oracle | `cargo test -p openui-engine --test selector_matching` (Windows) | 3 passed: all 64 select the same elements in document order; a state pseudo-class is an error; text and the viewport match nothing. With `dir` removed from the legacy list, `[dir=rtl]` matches nothing and the test fails |
| Touched crates | `cargo test -p openui-engine -p openui-style -p openui-macros -p openui` (Windows) | 945 passed, 0 failed, 15 ignored |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check` (the dependency policy hash follows the lockfile: `openui-engine` gains `serde_json` and `openui-style` with `constructor-tokens` as dev-dependencies); `generate_renderer_contract.py --check`; `check_platform_scoped_tests.py` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |
| Paired expanded census (WSL) | `wsl-branch-census-a8.sh`, base `9d7bccf6` | 22,517 of 23,728 exact before and after; 0 changed |
| CSS Grid coverage (WSL) | coverage matrix, base `04d1eeab` | 2,218 of 4,204 exact before and after; 0 changed |

The renderer contract counts a style-field name used in `openui-engine/src`
as a style consumer. An iterator's `.position()` call read as the
`position` property, so the sibling helpers use `take_while` and
`skip_while` instead.

The paired expanded census in WSL has no change. The base is `9d7bccf6`,
the recorded census base; the head is `9a44103d`, this change. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium
and Open UI are each identical in all 23,728 comparisons: 22,517 exact
before and after, with nothing gained or lost. Nothing renders from
selectors yet, so this checks that rendering is unchanged. The delta is in
the [census evidence](evidence/selector-matching-census-v1.json).

The CSS Grid coverage suite has no change either
([coverage evidence](evidence/selector-matching-coverage-v1.json)). The
base is `04d1eeab`, the recorded coverage baseline; the head is `9a44103d`.
In all 4,204 measured comparisons the status and the Open UI image are
unchanged: 2,218 exact before and after. The two references that the
baseline lists as unmeasured, whose renderer memory is unbounded, are left
out of both runs.

## Not done here

- State pseudo-classes: `:hover`, `:active`, `:focus`, `:focus-visible`,
  `:focus-within`, the form states and `:open` and `:modal`.
- The matcher walks children lists without Blink's caches and Bloom
  filters; cost is linear in siblings per step. Restyle scope comes with
  invalidation.
- Selectors are matched only on request; nothing applies style rules yet.
