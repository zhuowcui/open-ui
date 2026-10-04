# Open UI v0.2 unsupported features

This list is part of the public v0.2 contract. Unsupported means the release
does not promise the feature, even when a low-level renderer type or historical
experiment exists in the repository.

## Platforms and graphics

- macOS, Windows, Android, iOS, and browser/WASM execution.
- Vulkan, Metal, and Direct3D presentation.
- Multiple physical-GPU/driver profiles beyond the declared Linux release
  qualification profile.
- Cross-process rendering or a browser-style GPU process.

## Browser services

- HTML loading, CSS text parsing, CSS selector parsing, stylesheet injection,
  and a web cascade exposed as application APIs. Native element lookup and
  traversal remain part of the public Rust interaction contract.
- JavaScript execution or script bindings inside documents are outside the
  product design, including future versions. Application behavior uses public
  native Rust methods and Rust event callbacks. Browser-like element behavior
  needed by an application must be implemented as a native Rust API, even when
  Chromium tests express that behavior with a script. WebAssembly execution
  inside documents is also unsupported.
- HTTP, URL fetching, cookies, browser navigation, storage, service workers,
  and developer tools.
- Embedded interactive documents, iframes, and media playback.

## Native widgets and system integration

- File, date/time, and color pickers.
- Camera, microphone, geolocation, notifications, and global shortcuts.
- Printing and PDF export.
- Rich drag sources with arbitrary cross-process MIME negotiation beyond the
  Linux event/data paths documented for v0.2.

## Text and accessibility limits

- Platform font parity outside the pinned Linux font inventory.
- A general rich-text/HTML editor; v0.2 provides production plain-text input
  and textarea editing, selection, clipboard, IME, undo, and redo.
- Accessibility adapters on non-Linux platforms.

## Current release-candidate gaps

These are implementation gaps, not accepted final-v0.2 omissions:

- direct Skia Ganesh raster builds behind an explicit selection but remains
  unqualified; OpenGL presentation currently uploads the CPU Skia frame;
- the current four-profile Chromium census does not meet its exact gate; the
  focused and primitive 40-profile CPU matrices are exact;
- complete coverage of needed element operations through public native Rust
  APIs still requires review and verification from consuming applications;
- native scroll-into-view is implemented through shared Rust/C/accessibility
  operations, but scroll-margin/padding support, full option coverage and four
  reduced endpoint pixel differences remain open; own umbrella geometry is
  30/30 exact, while endpoint images remain 16/20 exact. The
  [private screen candidate](../renderer/generated/native-scroll-insets-v3.json)
  now matches all 510 native Rust geometry and image states after a shared
  screen-origin correction, and both raster matrices remain exact. Its smaller
  C matrix has ten static nested-clip failures among 175 states. Its complete
  censuses now finish at 21,340/22,924 original and 22,143/23,728 expanded exact,
  with two exact losses preventing promotion. The
  [latest separate candidates](../renderer/generated/native-scroll-insets-v8.json)
  repair all ten C clip failures at `cf59ea29` while preserving all 510 Rust and
  1,600 raster comparisons, and fix native absolute/fixed reveal traversal and
  transform containing blocks at `6d6768a8`, with 90/90 Rust and 60/60 C states
  exact and all 510 existing Rust states unchanged. Its 1,600 raster comparison
  invariants also stay unchanged and exact. The viewport cutoff `2cc950e0`
  now passes 8,537 workspace tests and all 20 C/30 Rust new states, preserving
  all 660 earlier states. Its raster gates remain running. An already-scrolled
  page still moves fixed controls in bounds and paint: 10/20 states are exact.
  The combined `4dd50621` fix is uncompiled and pixel-unverified. Both complete
  clip censuses preserve every `dac78e25` result, including its two exact losses.
  The patches remain unapplied and unqualified;
- two live legacy contour calibration paths must be replaced by general
  raster behavior and checked against Chromium without sample corrections;
- retained per-node compositor layers and compositor-owned immutable animation
  curves are incomplete;
- the strict 100 promoted animations while the UI thread is blocked gate has
  not been qualified;
- automated AT-SPI operation and physical-GPU context-loss qualification remain
  to be run in the release lab;
- the C ABI exports owned accessibility-tree snapshots and the shared Rust
  Linux event loop; release-lab AT-SPI operation, context loss, and packaged
  native C/C++ application qualification remain open;
- full preserve-3d/backface layer semantics remain incomplete.

The release cannot be marked final until the release-candidate gaps are closed
or the product contract is explicitly revised and independently reviewed.

The native font-cache leak is repaired in `a41fdeb9`. Its own workspace and
ten read-only checks pass, and all seven hosted hardening jobs pass, including
both sanitizers and all five fuzz targets. Complete private original/expanded
sweeps change no pixel result. The [recorded evidence](../renderer/generated/native-font-cache-lifetime-v4.json)
preserves the prior failures and each successful job.
