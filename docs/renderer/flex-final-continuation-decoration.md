# Final flex continuation decoration extent

Chromium is the pixel target. This investigation retains the pinned capture,
the zero-tolerance comparator, and the historical Open UI archive unchanged.

`wpt/css_break/flexbox_multi-line-row-flex-fragmentation-030` had 187 differing
pixels at 1280×720@1.25 and 75 at 1920×1080@1.5. The 1.25× difference was
bounded by physical pixels x=87–149, y=87–149. Making the two absolute green
overlays transparent in the [reduced source](reproducers/flex-final-decoration-no-abs.html)
exposed 2,047 differing pixels in the same bounds:
[Chromium](reproducers/flex-final-decoration-no-abs-chromium.png) painted the
multicol red behind the final flex continuation, while the
[old Open UI image](reproducers/flex-final-decoration-no-abs-before.png)
painted green through the fragmentainer.

The flex container has a definite 150 CSS pixel block size and 100 CSS pixel
columns. Its second fragment uses a 100 pixel visual interval so children can
continue within that column, but only the first 50 pixels belong to the flex
container's own background. Layout carried the source block size and consumed
offset, yet gave paint no own-decoration limit for this expanded final
fragment. Paint therefore filled the whole 100 pixel interval.

Layout now derives a fragment's remaining decoration extent from its source
slice when the visual interval extends past the source box end. Children keep
their full visual interval. The paint clip preserves normal fractional inline
edge coverage for this newly derived limit. An existing, independently
assigned decoration budget keeps its hard inline clip: a broader diagnostic
changed the previously exact `monolithic-overflow-005.tentative` by 50 pixels
at 1280×720@1.25. The guarded rule keeps that test exact at all four profiles.

In the dirty diagnostic run of all 125 neighboring row flex fragmentation
cases at four profiles, the previous 444/500 exact comparisons became
447/500. Only three Open UI decoded images changed: `-029` at 1.25× and
`-030` at 1.25× and 1.5×; each became exact. None of the 500 Chromium oracle
identities or decoded hashes changed. The original `-030` is now exact at all
four profiles. The 92 multicol layout tests and 65 text paint tests pass.
Full clean census
and release qualification remain open.
