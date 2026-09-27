# Open UI roadmap after v0.2

The only active product line is the pure-Rust Linux/headless engine. Historical
SP1–SP20 documents explain how renderer parity was established; they are not a
second implementation roadmap.

## Close v0.2

1. Close the zero-tolerance four-profile renderer matrix against pinned
   Chromium, including the focused and primitive profile gates.
2. Complete retained per-node compositing and immutable compositor animation
   curves, including preserve-3d and backface behavior.
3. Qualify 100 promoted animations during a 250 ms UI-thread stall.
4. Exercise X11, pure Wayland, Mesa software GL, physical GPU recovery, AT-SPI,
   and clean Ubuntu/Fedora package installation.
5. Produce byte-identical x86-64 and AArch64 artifacts twice, sign them, publish
   the compatible Rust crates in dependency order, and freeze the release tag.

## After the v0.2 contract is frozen

- Add macOS and Windows platform adapters without changing engine/scene
  semantics, then qualify native accessibility and presentation backends.
- Evaluate Metal, Direct3D, and Vulkan behind the existing compositor boundary.
- Expand specialized native pickers and system services as opt-in modules.
- Continue renderer conformance by admitting new independently reviewed cases
  against Chromium without modifying historical evidence.

No JavaScript runtime, script bindings, or browser execution are planned for
Open UI. New application interaction is added through public native Rust APIs
over the shared retained engine. Network fetching is also outside this
desktop framework's roadmap.
