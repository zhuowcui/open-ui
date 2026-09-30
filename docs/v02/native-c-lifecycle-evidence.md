# Native C application lifecycle evidence

At clean source checkpoint `2368ac62df214fa9b006ade853d7959ff0046c42`,
`oui_app_run` runs the existing retained document through Rust `App` and its
Linux adapter. `oui_app_request_exit` and versioned platform callbacks expose
the blocking lifecycle to C and C++ applications. The engine, control defaults,
clipboard, composition, and accessibility action path are shared with Rust.

## Clean local window runs

Both consumers opened a window, changed retained content after the first
successful presentation, presented a second frame, requested exit, and
released their handles. The C consumer also checked backend selection, viewport
delivery, callback rendering, changed pixels, rejection of a nested run and
active app destruction, and document use after app destruction.

| Window protocol | Presentation | C | C++ | Report |
|---|---|---|---|---|
| X11 | Software | 2 frames, exit 0 | 2 frames, exit 0 | [Source and binary identities](generated/native-c-x11-software-smoke-v1.json) |
| X11 | OpenGL | 2 frames, exit 0 | 2 frames, exit 0 | [Source and binary identities](generated/native-c-x11-opengl-smoke-v1.json) |
| Pure Wayland | Software | 2 frames, exit 0 | 2 frames, exit 0 | [Source and binary identities](generated/native-c-wayland-software-smoke-v1.json) |

The reports were emitted directly by `tools/ffi/verify_native_window.py` from a
clean tree. They bind the source commit, library, header, tool, consumer sources,
and binaries, and preserve each process's output and exit status. X11 runs unset
Wayland display/socket variables; Wayland runs unset `DISPLAY`.

These runs used the local WSLg display and the pinned Chromium build toolchain.
X11 initially lacked `libxkbcommon-x11`; providing the matching Ubuntu runtime
libraries allowed the runs above to pass. The Linux package dependencies and
hosted platform setup now include that library. Each report explicitly records
`release_qualification: false`. These functional runs do not establish a
physical GPU profile, reference-machine performance, context-loss behavior,
packaged installation, or end-to-end AT-SPI operation.

## API and regression checks

- All 104 previous C exports, 27 previous struct layouts, and earlier event IDs
  are preserved. The two new exports bring the total to 106. The ABI generator,
  checksum, symbol check, five headless C examples, and C++ header consumer pass.
- The locked Rust workspace passed 8,472 tests, with 13 ignored. The 43 public
  application conformance scenarios passed. The final Linux Rust/C/platform
  unit suite passed 102 tests, and the default headless C unit suite passed 22.
- Tests verify native keyboard editing, reentrant C callbacks, cancelled
  keydown and beforeinput defaults, separation of key names and committed text,
  punctuation without navigation-code collisions, and typing on noneditable
  controls. An AccessKit action request reaches the same checkbox and C change
  listener through Rust `App`; release-lab AT-SPI service operation remains open.
- All 236 Python closure and renderer-evidence tests, three packaging tests,
  read-only generated-source checks, and the 7/7 accountability audit pass.
- The 5,731 historical images and canonical rendering comparison binary remain
  unchanged. No renderer comparison count is updated by this API work.

Hosted PR checks run the native C/C++ consumers on X11 software/Mesa GL and pure
Wayland software, and retain their JSON reports. The release workflow builds
the C library with `linux`, checks that native consumer sources are packaged,
and runs the installed Debian C/C++ windows. Tag builds, both architecture
installations, hardware/accessibility qualification, and publication remain
pending in the [release ledger](release.md).

## Native IME checkpoint

At clean checkpoint `04394c863b4f8a8d69a5768a3fa5e9d4a2998411`, native text
composition commits the final text after an empty preview, groups preview
updates into one undo step, and restores the original value and selection on
cancellation. Rust applications call the public `Document` input methods;
the C native loop shares that document and engine. The legacy explicit C
composition dispatcher now uses the same engine commit/cancel operations.
The [interaction contract](interaction-controls.md) records callback and
focus behavior and the consuming Rust regressions.

Both native consumers still present two frames and exit successfully on all
three previously exercised paths:

| Window protocol | Presentation | C / C++ | Clean report |
|---|---|---|---|
| X11 | Software | Both pass | [v2 identities](generated/native-c-x11-software-smoke-v2.json) |
| X11 | OpenGL | Both pass | [v2 identities](generated/native-c-x11-opengl-smoke-v2.json) |
| Pure Wayland | Software | Both pass | [v2 identities](generated/native-c-wayland-software-smoke-v2.json) |

The locked workspace passes 8,480 tests with 13 ignored, the public application
suite passes 50 scenarios, and the Linux-enabled Rust/C/platform suites pass
185 tests with eight ignored. The 106-export ABI, five headless C consumers,
C++ header check, 239 Python tests, read-only generators, historical archive
integrity, and accountability audit pass. All six ordinary hosted PR checks
passed at this source checkpoint; the five optional hardening jobs were skipped.
The [complete static raster guards](../renderer/generated/focused-primitive-raster-v43.json)
pass all 1,600 comparisons, with
[no image or oracle-identity changes](../renderer/generated/native-ime-raster-delta-v1.json).

These window reports remain WSLg functional smoke evidence with
`release_qualification: false`. Native OS IME operation and AT-SPI service
inspection still require the release lab. Original and expanded renderer
census counts are unchanged by this evidence update; final release gates
remain open.
