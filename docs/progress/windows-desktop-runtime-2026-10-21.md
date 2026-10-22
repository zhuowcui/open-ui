# Win32 desktop runtime, 21 October 2026

The plan's last Windows step: the native window, input and presentation
runtime now runs on Windows as well as Linux. It remains an experimental
development platform; v0.2 promises Linux only.

## Design

The winit-based runtime in `openui-platform/src/linux.rs` was already
portable apart from its clipboard and GL configuration. It becomes the shared
`desktop.rs` runtime:

| Concern | Linux (`linux` feature) | Windows (`windows` feature) |
| --- | --- | --- |
| Window and input | winit X11 and Wayland | winit Win32 |
| OpenGL presentation | glutin EGL/GLX | glutin WGL |
| Software presentation | softbuffer X11/Wayland | softbuffer Win32 |
| Accessibility | AccessKit AT-SPI (`accesskit_unix`) | AccessKit UI Automation (`accesskit_windows`) |
| Clipboard | smithay-clipboard / x11-clipboard | Win32 `CF_UNICODETEXT` through `windows-sys` (`windows_clipboard.rs`) |

Every desktop gate widens from `all(feature = "linux", target_os = "linux")`
to also accept `all(feature = "windows", target_os = "windows")`, in
`openui-platform`, `openui`, `openui-ffi` and the five examples. Each gains a
`windows` feature. Dependencies stay target-gated, so `--all-features`
builds on both systems.

The C ABI exposes dropped and hovered file paths as
`OsStr::as_encoded_bytes()`. That is identical to the previous native bytes on
Unix and UTF-8 (WTF-8 for unpaired surrogates) on Windows. The Win32 clipboard
uses `windows-sys` rather than `clipboard-win`, whose BSL-1.0 license is not
in the release license policy.

The lockfile change is additive: `glutin_wgl_sys`, plus Windows `windows-sys`
edges. `docs/v02/generated/dependency-policy.json` is regenerated.

## Verification

Local measurements on the Windows host of the
[Windows baseline](windows-baseline-2026-10-19.md):

| Check | Command | Result |
| --- | --- | --- |
| Windows desktop build | `cargo build --locked -p openui-platform -p openui -p openui-ffi -p hello --features "openui-platform/windows openui/windows openui-ffi/windows hello/windows"` | Pass |
| Rust example | `OUI_BACKEND=software\|opengl\|auto OUI_SMOKE_EXIT=1 hello.exe` | Exit 0 for each backend |
| Native C and C++ window consumers | `python tools/ffi/verify_native_window.py --library <target>/debug/openui_ffi.dll --protocol win32 --backend software\|opengl` | Both pass on both backends. C: 2 frames, resize, 5 input events, focus 4, blur 3. C++: 2 frames, 3 input events |
| Platform, framework and FFI tests | `cargo test --locked -p openui-platform -p openui -p openui-ffi --features "... windows"` | All pass, including the Win32 clipboard round trip and the Windows native-app tests |
| C ABI of the desktop DLL | `python tools/ffi/verify_abi.py --library <target>/debug/openui_ffi.dll` | 164 exports exact; 41 C and 35 C++ consumers pass |
| All features | `cargo check --locked --workspace --all-features` | Pass |
| Generator and source checks | the 16 ledger checks of the Python CI job | Pass |
| Linux runtime in WSL | `cargo test --locked -p openui-platform -p openui -p openui-ffi --features "... linux"` | 432 passed, 0 failed, 8 ignored |
| Linux all features in WSL | `cargo check --locked --workspace --all-features` | Pass |

Hosted `windows-msvc` CI adds the desktop build, those tests, the `hello`
smoke run and the native window consumers on the software backend. OpenGL is
verified locally, because hosted runners may lack a modern OpenGL driver.

## Not covered

Win32 IME composition, drag and drop, and UI Automation behavior are compiled
and wired through the shared runtime, but no consumer exercises them yet.
High-DPI per-monitor changes and Windows packaging are also unverified. These
are not claimed.

## Update, 21 October 2026

UI Automation behavior is now exercised: a UI Automation client finds, focuses,
invokes, toggles, sets values on and closes a native Open UI window, locally and
in hosted `windows-msvc` CI. See [the UI Automation record](windows-ui-automation-2026-10-21.md).
IME composition, drag and drop, per-monitor DPI changes and packaging remain
unverified.
