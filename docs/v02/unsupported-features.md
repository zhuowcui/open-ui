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
- Open UI never executes JavaScript or provides script bindings. This holds
  for every version. Applications call public native
  Rust methods and handle events with Rust callbacks. Every needed browser
  element operation must be implemented in Rust and exposed to the consuming
  app. A missing public method belongs under implementation gaps, even when
  the corresponding Chromium test uses JavaScript. WebAssembly execution
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
- retained `ch`, `ex` and `lh` declarations work in the measured native app
  cases, but mixed-unit animation, font-relative line-height declarations and
  complete adjusted-font/root/orientation contexts remain native API work;
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
  [latest separate candidates](../renderer/generated/native-scroll-insets-v9.json)
  repair all ten C clip failures at `cf59ea29` while preserving all 510 Rust and
  1,600 raster comparisons, and fix native absolute/fixed reveal traversal and
  transform containing blocks at `6d6768a8`, with 90/90 Rust and 60/60 C states
  exact and all 510 existing Rust states unchanged. Its 1,600 raster comparison
  invariants also stay unchanged and exact. The viewport cutoff `2cc950e0`
  now passes 8,537 workspace tests and all 20 C/30 Rust new states, preserving
  all 660 earlier states. Both raster gates pass unchanged. An already-scrolled
  page still moves fixed controls in bounds and paint: 10/20 states are exact.
  The combined `4dd50621` fix passes 8,538 workspace tests, ten read-only checks,
  all 935 native Rust/C geometry/image states and both raster gates. All 885
  earlier states stay unchanged; all ten fixed-control failures are repaired.
  Its original/expanded censuses finish at 21,341/22,924 and 22,144/23,728 exact,
  zero errors, actual exits 1. Six exact matches are lost against `cf59ea29`
  and five against the applied umbrella; all Chromium inputs and 804 addition
  results stay unchanged. The included `c5769f2d`
  scroll-edge change repairs two earlier exact losses in a 144-comparison
  selection, but introduces three new exact losses against `dac78e25` at
  1.5 scale. Those paint-owned regressions prevent promotion. Both complete
  clip censuses preserve every `dac78e25` result, including its two exact losses.
  The [later scroll investigation](../renderer/generated/native-scroll-insets-v11.json)
  also finds missing native trailing-margin extents and eight primitive
  regressions from assigning compositor clip ownership to a non-overflowing
  `auto` box. The [shared Rust correction](../renderer/generated/native-scroll-insets-v12.json)
  repairs all eight raster failures and passes 8,539 workspace tests and all
  935 earlier native states. Fourteen opaque white images and 20 collapsed-margin
  API states still fail, including ten newly wrong scroll-height fields.
  The [completed follow-up](../renderer/generated/native-scroll-insets-v16.json)
  records 21,350/22,924 original and 22,153/23,728 expanded exact at private
  `45ddeee3`, zero errors, actual exits 1. One multicolumn exact regression
  prevents promotion. Private `944068e1` repairs native writing-mode
  inheritance and passes all 765 Rust/45 C collapsed-margin states, all 935
  prior native images and both raster matrices unchanged. Fourteen opaque
  white images still fail; full censuses were not started on that candidate.
  Other inherited-property behavior and opaque scrolling paint-chunk
  ownership remain open.
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
