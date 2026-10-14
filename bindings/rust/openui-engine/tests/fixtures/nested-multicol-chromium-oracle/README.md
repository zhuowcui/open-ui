# Nested multicolumn Chromium oracle

These RGBA8 crops come from pinned Chromium 147.0.7727.50, at the five scales
used by `nested_multicol_outer_fragmentation.rs`. The source PNGs remain unchanged.
`provenance.json` records the exact capture inputs, hashes, and crop bounds.

For each scale, both repeated ordered browser mutations and the independently
constructed final HTML produce identical full image bytes. Every pixel outside
the crop was checked to be opaque white. The test compares every pixel in the
full native frame against these bytes and that verified exterior, with no tolerance.

The former solid-green expectation was incorrect at fractional scales: Chromium
also draws blended pixels where the internal fragmented boxes meet. These files
correct that expectation using the browser oracle; they do not change rendering.
Browser scripts are used only by the external capture tool. The Engine test uses
ordered native Rust mutations and an independent native final-state construction.
