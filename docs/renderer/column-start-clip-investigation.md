# Multicolumn column-start clip investigation

Chromium is the expected pixel output. At clean checkpoint `e942aebc`, the
[four-profile census](generated/four-profile-census-v12.json) recorded
21,179/22,924 exact comparisons, 1,745 differences, and no render errors.
The prior [v11 census](generated/four-profile-census-v11.json) recorded
21,169 exact comparisons. The repair changed 24 Open UI decoded images in ten
`css_break` test IDs: ten differing comparisons became exact, 14 remained
different, and no exact comparison regressed. All 22,924 Chromium oracle
identities and decoded pixel hashes were unchanged.

The affected `block-max-height-001` test and its WPT reference produce the
same Chromium image and the same Open UI image at each required profile. The
same holds for their `001b` variants and the neighboring `block-min-height`
cases. This points to shared fragmented painting, rather than a special
max-height rule. Before the repair, the mobile 375×667@2 image had 78 or 84
different pixels from a six-device-pixel black border strip one row above the
Chromium column start. For `block-max-height-001`, Chromium had white and
Open UI had black at physical coordinate `(216, 185)`; the next row was black
in both. The [v12 census](generated/four-profile-census-v12.json) makes that
mobile profile exact for this case and its reference.

Temporary geometry tracing in `compute_column_block_clip_rect` measured a
direct child beginning 0.328125 CSS pixels above the column top, with zero
recorded start-ink overflow. The same offset occurred at 1.25 and 1.5 scale.
The column clip was expanded to the child top, allowing a fractionally shifted
continuation border into the preceding row. A first diagnostic that changed
only the content fragmentainer clip produced no image change. A second,
deliberately broad diagnostic removed all upward extension for direct
children. It made ten comparisons exact but broke 18 previously exact
comparisons, including `inline-with-float-004` and
`overflowed-block-with-no-room-after-000/001`. In those guards, measured
overflow was 30, 60, or 100 CSS pixels. That broad change was reverted.

The committed shared rule expands a horizontal column clip to a child top
only when the child extends upward by at least one physical device cell. It
keeps real overflow visible while suppressing the subpixel continuation
offset. Two negative-margin multicolumn guards remained exact at all four
required profiles. A clean run of the
[focused and primitive 40-profile suites](generated/focused-primitive-raster-v13.json)
at the same checkpoint was 640/640 and 960/960 exact, with zero errors. Its
1,600 Open UI decoded hashes, Chromium oracle identities, and Chromium
decoded hashes match the previous clean raster index.

The 1.25 and 1.5 scale `block-max-height-001` profiles still differ. At
1.25 scale, mismatched pixels fell from 1,319 to 1,287; at 1.5 scale, they
remained 1,590. Samples along fractional border edges include Chromium
channel 63 versus Open UI 64 at 1.25 scale and 127 versus 128 at 1.5 scale.
Background clip rows also remain different. These are open paint residuals,
not accepted tolerances. The original WPT tests and the neighboring overflow
guards are retained investigation cases; a minimized stand-alone reproducer
and reviewed ownership for the remaining pixels are still required.
