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

Those earlier checks establish ownership and replay behavior. Caller-owned raw
Skia object clones have their own lifetimes; the tokens cover framework
recording and layer objects.

The [next immutable snapshot](generated/native-font-cache-lifetime-v2.json)
records the completed local checks:

- At clean `09fbc363`, address and leak sanitizers each pass 29 FFI assertions
  and the three isolated lifetime tests, with no sanitizer report. A clean
  `d174ea0b` negative control reproduces 2,619 leaked bytes in 59 allocations
  after passing its 29 assertions. Rust/std and bindings compile under the
  sanitizer; the existing pinned CPU Skia archive is reused. This local scope
  does not replace hosted qualification.
- The same source passes 8,526 locked Linux-enabled workspace tests, zero
  failures and 13 ignored, with source identity unchanged.
- Clean `ed52e817` differs only in standalone fuzz-lock metadata. It handles
  the recovered input without a leak and completes all five fuzz targets for
  at least 30 seconds each, with actual exits 0 and source unchanged. Clean
  umbrella `79ad3af8` still reproduces the 2,606-byte/59-allocation fuzz leak.
- Its fresh pinned build runs ten C and four C++ headless consumers, preserving
  112 exports and the header checksum. All ten read-only checks pass.
- Its clean 40-profile matrices are 640/640 focused and 960/960 primitive
  exact. Every native/Chromium image hash, oracle identity, difference
  signature, status and differing-pixel count agrees with `d174ea0b`.

The first sanitizer setup used a wrong Skia source path; the next exposed a
host linker/sysroot setup failure. Neither qualifies as a test pass. The first
fuzz negative control also let Cargo rewrite a stale standalone lockfile;
that run remains diagnostic. The corrected lock synchronizes already-declared
native 0.2.0 versions and the existing Skia pin. It adds no dependency upgrade.
CI now checks locked metadata before fuzzing and unchanged source after each
target. A read-only helper path failure and its successful retry are retained.

The font candidate remains unapplied to the umbrella PR. Its complete original
and expanded pixel sweeps are running; incomplete results are not admitted as
qualification. Application and own-source hosted hardening remain required.
Earlier failed reports and references remain unchanged.

All three hosted workflows at umbrella documentation checkpoint `067cc794`
completed successfully, with six passing jobs and five skipped jobs. Those
skips and the private local results do not close the failed manual sanitizer
and fuzz gates. The full renderer, native API and release-lab gates remain open.
