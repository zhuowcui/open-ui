# Form pseudo-classes, 10 November 2026

The selector matcher now matches `:disabled`, `:enabled`, `:checked`,
`:indeterminate`, `:placeholder-shown`, `:read-only`, `:read-write` and
`:required` from the engine's form control state, as Chromium 147 does.
This is the sixth A1 step of
[RFC 0001](../alpha/rfcs/0001-typed-style-rules.md). `:invalid`, `:open`
and `:modal` are the next step. No style rule is applied yet, and no
control state changes.

## Change

All in `bindings/rust/openui-engine/src/selector_match.rs` (`openui-engine`):

| Pseudo-class | Matches, as Blink's `SelectorChecker` |
| --- | --- |
| `:disabled`, `:enabled` | A button, input, select, textarea, optgroup, option or fieldset, by `Engine::is_effectively_disabled` (own `disabled`, a disabled fieldset ancestor outside its first legend, an option's disabled optgroup). A disabled select leaves its options enabled. Other elements match neither. |
| `:checked` | A checked checkbox or radio (by its `type`), or a selected option. Option selectedness follows HTML for a tree built from attributes: in a single select the last option with `selected` wins; with none, a select whose display size is 1 selects its first option that is not disabled; `multiple` or `size` above 1 selects nothing by default. |
| `:indeterminate` | A checkbox with its indeterminate flag, a radio whose group has no checked radio (an unnamed radio is its own group), or a progress element without a `value` attribute, whatever its value. |
| `:placeholder-shown` | A text field (`text`, `search`, `email`, `password`, `tel`, `url`, `number`, or an unknown type) or textarea with a `placeholder` attribute, even an empty one, and an empty value. Read-only and disabled fields count. |
| `:read-write`, `:read-only` | Read-write: a textarea, or an input that supports `readonly` (the text fields and `date`, `datetime-local`, `month`, `time`, `week`), neither `readonly` nor disabled. Every other element, `html` and `body` included, is read-only. |
| `:required` | A select, textarea or input with `required`, except `hidden`, `range`, `color` and the button types. A disabled control still matches. |

Input types compare ASCII case-insensitively, and an unknown type is
`text`, as in Blink.

## Verification

`tools/qualification/capture_form_state_oracle.py` builds one tree of 71
elements under `body` with DOM calls in the pinned Windows
Chrome-for-Testing 147.0.7727.50: every input type the rules separate, checkboxes and radio
groups, textareas, selects with optgroups, `multiple`, `size` and two
selected options, nested fieldsets with legends, progress and meter
elements, a form, and a `div` with `disabled`, `readonly` and `required`.
It then makes 10 rounds of changes through properties and attributes
(`checked`, `indeterminate`, `value`, enabling and disabling fieldsets,
`readonly`, removing a progress value, changing an input's type) and
records the ids `querySelectorAll` returns for 13 selectors after the build
and after each round
([oracle](evidence/form-state-pseudo-classes-v1/chromium-windows.json)).

| Check | Command | Result |
| --- | --- | --- |
| Chromium oracle | `python tools/qualification/capture_form_state_oracle.py --chrome E:\open-ui\out\chrome-win\win-147.0.7727.50\chrome-win64\chrome.exe` | 11 steps, 13 selectors |
| Matching against the oracle | `cargo test -p openui-engine --test form_state_pseudo_classes` (Windows) | 2 passed: every selector selects the same elements in document order after every step; `:invalid`, `:open` and `:modal` are errors. With the first selected option winning instead of the last, the `:checked` steps fail; with the date and time types not read-write, the `:read-write` and `:read-only` steps fail |
| Earlier matching | `cargo test -p openui-engine --test selector_matching --test state_pseudo_classes` (Windows) | 3 and 1 passed |
| Touched crates | `cargo test -p openui-engine -p openui-style -p openui` (Windows) | 905 passed, 0 failed, 11 ignored |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check`; `generate_renderer_contract.py --check`; `check_platform_scoped_tests.py` | no drift |
| `cargo fmt --all --check` | in `bindings/rust`, in WSL (on Windows the command line for all files is now too long; `cargo fmt -p <package> --check` passes for each of the 26 packages) | pass |

The native test makes each change with the engine's own call:
`set_checked`, `set_indeterminate`, `set_control_value`, `set_attribute`
and `remove_attribute`.

CENSUS

## Not done here

- The engine keeps no HTML selectedness for options: choosing an option
  through `Engine::activate` or `adjust_control` sets the select's value
  but not every option's selected flag consistently, and setting the
  `selected` attribute does not deselect the others. `:checked` on options
  is therefore checked only for trees built from attributes; the oracle
  changes no option. Owner: `openui-engine` forms.
- `:invalid` (constraint validation), `:open` and `:modal`.
- `contenteditable` is not supported, so no element other than a text
  control is read-write.
