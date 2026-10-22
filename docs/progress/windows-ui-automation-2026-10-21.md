# Windows UI Automation check, 21 October 2026

The Win32 runtime record left UI Automation behavior compiled but unexercised.
This change adds the first operating-system accessibility check: a UI
Automation client drives a native Open UI window on Windows through the same
interfaces a screen reader uses.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `accessibility` example: a button whose label counts its activations, a named checkbox and a named text field, behind the `linux` and `windows` features, headless otherwise | examples | `bindings/rust/examples/accessibility` |
| UI Automation verifier: launches the example, then checks the framework, the named controls and their control types, focus, invoke, toggle, value and window close through the .NET UI Automation client | Windows platform tooling | `tools/platform/verify_windows_uia.ps1` |
| `windows-msvc` CI builds the example with the `windows` feature and runs the verifier | CI | `.github/workflows/ci.yml` |

An invoke is observable without reading pixels: the button's accessible name
comes from its text, which changes from `Pressed 0 times` to `Pressed 1 times`.
The lockfile gains only the new example package; the dependency policy is
regenerated.

## Verification

Local measurements on the Windows host of the
[Windows baseline](windows-baseline-2026-10-19.md).

| Check | Command | Result |
| --- | --- | --- |
| Example, desktop and headless | `cargo build --locked -p accessibility --features windows`; `cargo build --locked -p accessibility` | Pass |
| UI Automation round trip | `pwsh -NoProfile -File tools/platform/verify_windows_uia.ps1 -Executable <target>/debug/accessibility.exe` | Passed: framework `Open UI`; button, checkbox and edit found by name and control type; focus moved to the edit and back; two invokes renamed the button `Pressed 2 times`; toggle Off to On; value `Ada Lovelace`; window closed with exit code 0 |
| Generated contract | `python tools/release/generate_v02_contract.py --check` | Pass after regenerating the dependency policy |

## Not covered

The check uses the UI Automation client, not a particular screen reader.
Narrator and NVDA speech, text ranges, live regions and high contrast are
unverified. IME composition and drag and drop still have no operating-system
level check on Windows; their shared event paths are covered by unit tests.
