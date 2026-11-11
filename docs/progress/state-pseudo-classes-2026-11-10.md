# Interaction pseudo-classes, 10 November 2026

The selector matcher now matches `:hover`, `:active`, `:focus`,
`:focus-visible` and `:focus-within` from the engine's interaction state, as
Chromium 147 does. `:focus-visible` follows Blink's heuristic, which the
engine did not before: pointer focus on a text field is visible, native
focus keeps the last focus type, and a key press makes pointer focus
visible. This is the fifth A1 step of
[RFC 0001](../alpha/rfcs/0001-typed-style-rules.md). Form and dialog states
are the next step. No style rule is applied yet.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `:hover` reads each pointer's hover path, `:active` the pressed element's chain (`Engine::is_active`, which covers the pressed element's ancestors), `:focus` the focused element, `:focus-within` the focused element and its ancestors, and `:focus-visible` the focused element when `Engine::focus_visible` holds. | `openui-engine` | `bindings/rust/openui-engine/src/selector_match.rs` |
| `Engine::focus_visible` follows Blink's `SelectorChecker::MatchesFocusVisiblePseudoClass`: the focused element matches if it takes text input (`MayTriggerVirtualKeyboard`: a textarea, or an input whose type is a text field, read-only included), if the last pointer or keyboard focus change was not from a pointer, or if a key went down after it. Native (`Script`) focus no longer counts as keyboard focus; it keeps the last focus type, as Blink's script focus does. | `openui-engine` | `bindings/rust/openui-engine/src/interaction.rs`, `lib.rs` |
| New `Engine::note_keyboard_event`, Blink's `Document::SetHadKeyboardEvent` on key down; a pointer focus change clears it. | `openui-engine` | `bindings/rust/openui-engine/src/interaction.rs` |

`Engine::focus_visible` had no caller in the workspace, so nothing renders
differently. The public `Document` does not call `note_keyboard_event` yet;
the step that applies style rules wires it to key-down dispatch.

## Verification

`tools/qualification/capture_state_pseudo_class_oracle.py` builds a tree
of absolutely positioned boxes in the pinned Windows Chrome-for-Testing
147.0.7727.50 and drives it through the DevTools protocol with real mouse
and keyboard input (`Input.dispatchMouseEvent`, `Input.dispatchKeyEvent`)
and with `element.focus()`. After each of 22 steps it records the ids
`querySelectorAll` returns for 15 selectors
([oracle](evidence/state-pseudo-classes-v1/chromium-windows.json)). The
steps cover hover, a press and drag across siblings and back, release, a
click on a button, a checkbox, a text field, a read-only text field and a
textarea, Tab and Shift+Tab, script focus after pointer, keyboard and
text-field focus, a Shift press after pointer focus, and a second click on
the focused button.

What Chromium showed:

- `:hover` follows the pointer while a button is held, and `:active` stays
  on the pressed chain.
- Pointer focus on a button or checkbox is not `:focus-visible`; on a text
  field, read-only or not, or a textarea, it is.
- Script focus after pointer focus is not visible; after keyboard focus it
  is. Tab after pointer focus is visible, and the next pointer focus change
  hides it again.
- A Shift press after pointer focus makes the focused button visible, and
  a second click on it keeps it visible.

| Check | Command | Result |
| --- | --- | --- |
| Chromium oracle | `python tools/qualification/capture_state_pseudo_class_oracle.py --chrome E:\open-ui\out\chrome-win\win-147.0.7727.50\chrome-win64\chrome.exe` | 22 steps, 15 selectors |
| Matching against the oracle | `cargo test -p openui-engine --test state_pseudo_classes` (Windows) | 1 passed: every selector selects the same elements in document order after every step. With a textarea not counted as a text field, the textarea step fails; with native focus always visible, the two script-focus-after-pointer steps fail |
| Earlier matching | `cargo test -p openui-engine --test selector_matching` (Windows) | 3 passed; a form state (`:checked`) is still an error |
| Touched crates | `cargo test -p openui-engine -p openui-style -p openui` (Windows) | 903 passed, 0 failed, 11 ignored |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check`; `generate_renderer_contract.py --check`; `check_platform_scoped_tests.py` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |
| Paired expanded census (WSL) | `wsl-branch-census-a8.sh`, base `9d7bccf6` | 22,517 of 23,728 exact before and after; 0 changed |
| CSS Grid coverage (WSL) | coverage matrix, base `04d1eeab` | 2,218 of 4,204 exact before and after; 0 changed |

The replay does what the public `Document` does around the engine: a press
on a focusable target focuses it with `FocusOrigin::Pointer`, and Tab moves
focus with `Engine::advance_focus`. Each key also calls
`note_keyboard_event`.

The paired expanded census in WSL has no change. The base is `9d7bccf6`,
the recorded census base; the head is `95480009`, this change. Both are
built with the CI `SKIA_GN_ARGS` and share one Chromium cache. Chromium
and Open UI are each identical in all 23,728 comparisons: 22,517 exact
before and after, with nothing gained or lost. No style rule reads the
matcher yet, and the census sends no input events, so this checks that
rendering is unchanged. The delta is in the [census
evidence](evidence/state-pseudo-classes-census-v1.json).

The CSS Grid coverage suite has no change either
([coverage evidence](evidence/state-pseudo-classes-coverage-v1.json)). The
base is `04d1eeab`, the recorded coverage baseline; the head is `95480009`.
In all 4,204 measured comparisons the status and the Open UI image are
unchanged: 2,218 exact before and after. The two references that the
baseline lists as unmeasured, whose renderer memory is unbounded, are left
out of both runs.

## Not done here

- Form states (`:disabled`, `:enabled`, `:checked`, `:indeterminate`,
  `:placeholder-shown`, `:read-only`, `:read-write`, `:required`,
  `:invalid`) and `:open` and `:modal`.
- Open UI has no `<label>`, so the hover and active state Blink forwards
  from a label to its control does not arise.
- Chromium moves focus to the nearest focusable ancestor of a pressed
  element, or clears it; the public `Document` focuses only a focusable
  target and otherwise leaves focus where it was. The oracle presses only
  focusable elements, or presses with nothing focused. Owner: `openui`
  (framework track).
- `FocusOrigin::Accessibility` counts as keyboard focus, as before; Blink's
  focus type for assistive-technology focus is not measured here.
