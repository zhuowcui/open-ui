# Native disabled groups

Open UI runs no JavaScript, in any version. Apps call native Rust methods and
supply Rust callbacks. C uses the same retained Document. Pinned Chromium
defines behavior and pixels; old Open UI images remain historical evidence.

## Implemented native APIs

`Element::is_own_disabled` reads a control's own disabled state.
`Element::is_effectively_disabled` includes inherited fieldset state and the
first authored legend exception. Both queries avoid layout and preserve
reflected attributes. Ordinary container disabled attributes do not disable
controls. Options inherit only from their direct optgroup. Hidden legends
still participate in authored child order.

Attribute changes and tree mutations use the shared Document lifecycle.
Callbacks run after engine borrows end. Moving or detaching a focused subtree
dispatches blur before removal. Operands are validated before callbacks and
revalidated before mutation. Painting, input and accessibility share effective
state. Native apps can query it and use the normal attribute/tree methods.

C adds `oui_element_is_own_disabled_v1` and
`oui_element_is_effectively_disabled_v1`. Outputs remain unchanged on error;
ownership, lifetime, thread and borrow guards apply. The checkable follow-up
brings the ABI to 128 exports and 34 layouts, preserving all previous 127
exports and every layout. C
accessibility actions use the shared Rust Document.

## Measured scope

The implementation is integrated into the umbrella PR. Fresh measurements
on clean public `6ba3ef58` reproduce the private disabled-group behavior.
This qualifies the measured native behavior and preserves the existing exact
raster checks. The complete Chromium pixel gates still fail.

- 88/88 Rust, 88/88 C and 88/88 C++ behavior scenarios match pinned Chromium.
- 21 repeated process pairs produce identical output.
- All previous 109 scenarios in each language remain unchanged.
- 197 unit guards and 23 integration guards pass.
- All 24 C and 18 C++ consumers compile, link and run.
- Focused 640/640 and primitive 960/960 comparisons are exact.
- Original 21,338/22,924 and expanded 22,141/23,728 are exact,
  with zero errors, no exact losses and no worsened rows. Full gates remain open.
- All 8,618 workspace tests and 51 headless C tests pass; 13 pre-existing
  ignored workspace tests remain recorded.
- All twenty local checks and seven explicit hosted hardening jobs pass.
- Three virtual X11/Wayland reports verify the same source, tool, header and
  consumer bytes. Both C and C++ applications run; physical lab work stays open.

The C/C++ state inventory compares five common fields: own, property,
effective, enabled and connected. Rust additionally observes kind and parent.
Other focus and neighboring cases compare complete reference rows. The first
private attempt failed on a wrong null-handle test expectation. The correction
follows the existing InvalidArgument contract. Both attempts are preserved.

[Public receipts, logs, identities and comparisons](../renderer/generated/native-disabled-groups-public-v1.json).
The earlier [private evidence](../renderer/generated/native-disabled-groups-private-v1.json)
remains unchanged. The first public workspace attempt stopped for disk space
during compilation and produced no test results. Its completed artifact audit
preceded cache cleanup and the successful retry. The first public pixel attempt
stopped because an old wrapper required an old owner name; it executed no app
or pixel comparison. The corrected wrapper binds the exact parent, source,
binary and output paths while preserving reference bytes and comparison logic.

## Integrated checkable follow-up

The shared checkbox/radio implementation is integrated into the umbrella
branch. On clean public `54cf4b3c`, apps match **220/220 checkable behavior
cases in each of Rust, C and C++**, with identical repeated output. The 88
fieldset cases per language and all previous selection regressions also stay
exact. These measure programmatic assignment,
authored defaults, form/tree grouping, clones, callback state, cancellation,
indeterminate state and the distinction between ordinary activation and raw
click events. Apps call actual native APIs. Open UI executes no JavaScript.

The public Rust appearance app matches **160/160 cases and all 640 PNG
comparisons** at five scales and eight subpixel phases. Its scope is the default
light appearance on white for checked/unchecked and disabled/enabled controls.
Source and artifact audits were saved before subsequent documentation and C
formatting updates. All 8,623 workspace tests and 52 headless C tests pass on
that source, with 13 ignored workspace tests recorded. Five successful stages,
including the workspace run, are explicitly inherited from the audited
disk-interrupted owner; that owner's failed result remains preserved. All
100 recorded stages pass, including 25 C and 19 C++ consumers. The ABI has
128 exports and 34 layouts. Seven hosted hardening jobs and three virtual
window reports pass. Physical lab qualification remains open.
[Public control behavior, appearance and provenance](../renderer/generated/native-checkable-public-v1.json).

The private runtime is clean `a17f8a33`; the corrected C/C++ app source is clean
`cef6a7eb`. Their only difference is the consuming C example; framework and
header bytes are unchanged. All 198 unit guards, 27 integration guards, 25 C
and 19 C++ consumers pass. The private ABI has 128 exports and the same 34
layouts; those additions are now present on the umbrella branch. Failed attempts and completed
source/artifact audits are preserved. This is scoped behavior evidence, with
no complete native-owner or renderer qualification on the final private source.

The code adds Rust `Element::dispatch_click_event` and C
`oui_element_clone_subtree_v1`, and routes checkable activation through the
shared Rust Document. It separates live state from authored attributes and
runs preactivation before callbacks, restoring state when canceled. These
changes use native Rust operations and Rust callbacks. The later
[native form-owner follow-up](native-form-owner.md) verifies 88 additional
association and radio-membership scenarios through public Rust, C and C++
APIs. Unmeasured form operations, radio keyboard behavior and complete API
coverage remain open. Fixture lowering does not complete a public app API.
[Private observations, code patch and provenance](../renderer/generated/native-checkable-private-v1.json).

The default appearance correction on clean private `365ed4a7` matches
**160/160 native app cases**, including all **640 PNG comparisons**, across
five scales and eight subpixel phases. A real Rust click callback changes
fieldset disabled state. Checked and disabled controls now use shared native
theme paths instead of the old sampled checkbox contour. This qualifies only
the measured light appearance on a white background; hover, pressed, focus,
keyboard, mixed state and other sizes remain unqualified.

Its complete renderer trial remains **21,338/22,924 original** and
**22,141/23,728 expanded** exact, with zero errors. Focused **640/640** and
primitive **960/960** remain exact. All nine comparison fields stay unchanged
across all 48,252 rows. Every build artifact and source identity was audited
before integration. Both full pixel gates still fail.
[Appearance evidence and completed renderer audit](../renderer/generated/native-checkable-theme-private-v1.json).

## Remaining work

A complete fresh public renderer census, complete control behavior and
appearance, and the required hosted Chromium equality job remain open. The
hosted historical-image job checks archive integrity; its success does not
qualify current rendering against Chromium. Pinned inputs for hosted pixel
qualification still need to be supplied and verified.

Composition, complete native APIs, remaining pixels, compositor behavior,
physical hardware and release qualification remain open.
