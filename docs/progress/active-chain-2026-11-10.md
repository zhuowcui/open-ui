# `:active` covers the pressed element's ancestors, 10 November 2026

Pressing an element now makes it and every ancestor `:active`, as in
Chromium; a pressed disabled control is `:active` too. Before this change,
only the pressed element itself was `:active`, and a disabled one was not.
This is the `:active` fix in
[RFC 0001](../alpha/rfcs/0001-typed-style-rules.md) (state sources; plan
unit A1, Workstream A). Style rules such as `.card:active` need it when the
press lands on a descendant.

## Finding

`Engine::is_active` (`openui-engine/src/interaction.rs`) compared the
queried node with the pressed target of each pointer, so ancestors never
matched. The pointer-down arm stored the target only when it was not
disabled, so a pressed disabled control was not `:active` either.

Chromium 147 sets the chain in `Document::UpdateActiveState`
(`third_party/blink/renderer/core/dom/document.cc`). On press it marks
every inclusive flat-tree ancestor of the hit element as in the active
chain and sets `:active` on them. Later moves cannot change the chain until
the release clears it. `Element::SetActive` (`core/dom/element.cc`) does not
check whether the element is disabled. These sources were read at
147.0.7727.24, the local source checkout, which is on the same branch as
the pinned 147.0.7727.50.

The pinned Chromium was then measured with real mouse input.
`tools/qualification/capture_active_chain_oracle.py` drives the pinned
Windows Chrome-for-Testing 147.0.7727.50 through
`Input.dispatchMouseEvent` and records which elements match `:active` after
each step ([oracle](evidence/active-chain-v1/chromium-windows.json)):

| Step | Chromium `:active` |
| --- | --- |
| Press a nested `div` | it, its two ancestor `div`s, `body`, `html` |
| Move to a sibling while pressed | unchanged |
| Release | none |
| Press an enabled `button` | it, its parent, `body`, `html` |
| Press a disabled `button` | it, its parent, `body`, `html` |

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `Engine::is_active` is true for the pressed element and every node on its path to the root (the same path hover uses). | `openui-engine` | `bindings/rust/openui-engine/src/interaction.rs` |
| Pointer-down records the pressed target even when it is disabled. Release still activates only an enabled target, and the range drag still skips a disabled range. | `openui-engine` | same |

`Element::is_active` and the C ABI's `oui_element_is_active_v1` call
`Engine::is_active`, so both report the chain.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Chromium oracle | `python tools/qualification/capture_active_chain_oracle.py --chrome E:\open-ui\out\chrome-win\win-147.0.7727.50\chrome-win64\chrome.exe` | 7 steps, table above |
| Open UI against the oracle; a disabled button is active but not clicked | `cargo test -p openui --test active_chain` (Windows) | 2 passed. With the engine change reverted, both fail: after "press inner" only `inner` is active, expected `body`, `outer`, `middle`, `inner`; the disabled button is not active |
| Touched crates | `cargo test -p openui-engine -p openui -p openui-ffi --features openui/windows --no-fail-fast` (Windows) | 921 passed, 0 failed, 11 ignored |
| Generators | `generate_properties.py --check`; `generate_v02_contract.py --check`; `generate_renderer_contract.py --check` | no drift |
| `cargo fmt --all --check` | in `bindings/rust` | pass |

Open UI has no `html` element; its root element plays `body`. The test maps
Chromium's `body` to that root and leaves `html` out.

CENSUS

## Not done here

- Open UI computes the chain from the pressed element when it is queried,
  where Chromium freezes it at press. They differ only if the pressed
  element is moved to another parent while pressed.
- No style rule reads `:active` yet; selectors arrive with the rest of
  RFC 0001. Only `is_active` queries change.
- Keyboard activation (`Space` on a button) does not set `:active` in Open
  UI; that is unchanged.
