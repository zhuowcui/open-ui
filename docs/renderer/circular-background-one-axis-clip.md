# Circular background with a one-axis overflow clip

Chromium 147 is the sole pixel oracle. The shared repair moves the existing
horizontal overflow scissor outside the choice between direct rounded filling
and an F16 circular intermediate. Both background paths now apply the same
one-axis clip. The renderer does not select a test ID or alter finished pixels.

## Reduced native case and cause

`wpt/css_overflow/clip-008` already supplies a reduced Engine-backed case:
a 100×100 red block, a 100×100 wheat circular child with `overflow-x: clip`
and visible vertical overflow, and a 100×100 green descendant. The circle
starts at CSS x=20. At 1.25× the clipping edge is physical x=25.

The F16 circular path previously skipped the horizontal scissor used by the
direct rounded fill. Its intermediate leaked one low-coverage wheat sample
into the physical cell immediately outside that edge, at (24, 87). Open UI
produced RGBA `(255, 254, 253, 255)` where Chromium produced white,
`(255, 255, 255, 255)`. Applying the same clip around either fill keeps the
circular coverage calculation and removes the outside sample. The existing
native fixture provides the reduced oracle; its input and reference bytes
remain unchanged.

## Clean qualification

At clean commit `15f9f12d`, the rebuilt CPU Skia `pixel_compare` binary had
SHA-256
`46cb2073cf1e535d155592361cab5b6629a98ad5feb8778474186e49d78cdf71`.
The complete [v41 original census](generated/four-profile-census-v41.json)
is **21,266/22,924 exact**, 1,658 different, and zero errors. Compared with
v40, only the 1.25× `clip-008` Open UI image changed; its one wrong pixel
became exact. All other Open UI decoded images, all 22,924 Chromium oracle
identities, and all Chromium decoded images stayed fixed. No exact comparison
regressed. The raw report is
`out/renderer-evidence/rounded-clip-x-full-15f9f12d/full-summary.json`, with
SHA-256
`979b844e0e04e70073b015eb8009e222ddb25a912969c50a4904a0f782dc2094`.

The [v42 focused/primitive index](generated/focused-primitive-raster-v42.json)
is 640/640 and 960/960 exact across the 40-profile matrices, with zero errors.
All 1,600 Open UI decoded images and Chromium oracle identities and images
match the prior v41 raster evidence.

The complete [v24 expanded requalification](generated/expanded-requalification-v24.json)
is **22,069/23,728 exact**, 1,659 different, and zero errors. Only the same
original `clip-008` image changed from v23; all 804 addition images stayed
fixed. Every original result matches the separate clean census, and every
addition result matches its clean four-profile guard. All 23,728 Chromium
oracle identities and decoded images stayed fixed. The raw expanded report is
`out/renderer-evidence/rounded-clip-x-expanded-15f9f12d/expanded-summary.json`,
with SHA-256
`6ca246f4baee84344abbe6903db83562072c5b8cc6df791da9b6db5f954382de`.

The release contract still includes all 201 additions; 200 are exact at all
four profiles. The [v26 diagnostic selection](../../tools/qualification/manifests/expanded-v26.json)
records those 200 without changing the release manifest. The admitted
`display-contents-dynamic-fieldset-legend-001` case still differs by 1,315
pixels at 1.25×. The original census retains 914 unowned residual test IDs.
Neither gate is qualified as complete.

The Rust formatting and `openui-paint` tests passed. Read-only generated
contracts, release source verification, archive integrity, and the 7/7
accountability audit also passed. Historical Open UI images remain provenance
only; Chromium is the expected output.
