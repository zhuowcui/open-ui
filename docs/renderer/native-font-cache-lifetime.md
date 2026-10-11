# Native font-cache lifetime

Open UI executes no JavaScript. Applications use native Rust methods and
callbacks. Chromium remains the pixel reference. This lifecycle fix
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

The applied Rust implementation tracks font-cache ownership in font collections,
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

The [next snapshot](generated/native-font-cache-lifetime-v4.json) records both
complete private pixel sweeps at clean `ed52e817`: 21,334/22,924 original and
22,137/23,728 expanded exact, zero errors, with actual exits 1. Every native and
Chromium image hash, oracle identity, difference signature, status and pixel
count agrees with `d174ea0b`. All 804 additions are unchanged; 200 of 201 cases
are exact at every required profile. The full pixel gate still fails.

The fix is applied and pushed in umbrella checkpoint `a41fdeb9`. Native source,
tools, headers and examples are identical to the tested private candidate.
Its own Linux-enabled workspace passes 8,526 tests, zero failures and 13
ignored, with clean source unchanged. All ten read-only checks pass. The first
workspace wrapper mishandled the tracked diagnostic PNG; its actual test exit
0 and wrapper exit 1 are preserved. The diagnostic was saved and restored,
and the corrected run passes with matching source identities.

All three pull-request workflows at `a41fdeb9` pass, with six successful jobs
and five skipped jobs. The separate seven-job
[manual hardening run](https://github.com/zhuowcui/open-ui/actions/runs/37149510887)
passes all seven jobs: address/leak sanitizers, Miri, C UBSan, Linux conformance
and windows, MSRV and all five fuzz targets. Each sanitizer passes 29 FFI
assertions with no report; all seven complete logs are preserved. Pull-request
skips remain open results. The original renderer failures, complete needed
native API coverage and release-lab gates remain open.
