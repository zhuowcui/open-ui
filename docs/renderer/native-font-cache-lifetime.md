# Native font-cache lifetime

Open UI executes no JavaScript. Applications use native Rust methods and
callbacks. Chromium remains the pixel reference. This private lifecycle fix
does not change the measured umbrella renderer result: 21,334/22,924 exact,
1,590 different, zero render errors. The pixel gate still fails.

## Cause and implementation

The own-source address/leak sanitizer jobs each pass 29 FFI assertions, then
report 2,619 leaked bytes in 59 allocations. The tree-mutation fuzz target
reports 2,606 bytes in 59 allocations for the four-byte input
`[50, 255, 255, 64]`, which creates a div and requests a scene.

A C++ reproducer against the pinned Skia build isolates font-metrics cache
retention: one manager leaks 2,597 bytes in 59 allocations; three leak three
times that amount. Retiring Skia's font cache clears those reports. The
[ownership review](generated/native-nested-scroll-v9.json) preserves the
source, logs, failed setups and corrected fuzz-input interpretation.

The private Rust candidate tracks font-cache ownership in font collections,
resolved fonts, immutable paint recordings and independently cloned scrolling
content layers. Each owner releases its Skia resources before releasing its
cache-lifetime token. Only the last token retires the shared font cache.
Acquisition and retirement are serialized, so a new client cannot start during
the previous clients' final retirement. Other live documents, fonts and layers
keep their warm cache.

A recording retains an opaque lifetime token. It retains no mutable font
registry, DOM or application callback through that token. Existing scene,
application and C API signatures remain unchanged. There is no global
Fontconfig shutdown, leak suppression, dependency upgrade or pixel patch.

## Verification and remaining gates

At clean private checkpoint `09fbc363`, the
[versioned evidence](generated/native-font-cache-lifetime-v1.json) records:

- three isolated lifetime tests passing, including eight concurrent font
  clients, drawing scenes after engine destruction on another thread, and
  independently cloned content layers;
- 548 text, paint, engine and compositor library tests passing, zero failures
  or ignored tests;
- unchanged engine layout/paint/scene counters for an unchanged scene, with
  one compositor raster followed by frame reuse;
- identical source identity before and after testing;
- the first extended attempt's compilation failure, corrected by applying the
  test's scrolling style through the typed style-property operation.

These checks verify ownership and replay behavior. They do not establish exact
Chromium pixels or a sanitizer pass. Caller-owned raw Skia object clones have
their own lifetimes; the tokens cover framework recording and layer objects.

The candidate remains unapplied to the umbrella PR. Its own-source ASan/LSan,
recovered fuzz input, all five fuzz targets, complete workspace, ABI consumers,
and clean Chromium focused/primitive/original/expanded checks remain open.
Earlier failed reports and references remain unchanged.

All three hosted workflows at the umbrella documentation checkpoint
`ac1b37b2` completed successfully, with six passing jobs and five skipped jobs.
Those skips do not qualify the failed manual sanitizer and fuzz gates.
