# Column paint phase captures

These raw RGBA crops come from the immutable Chromium 147.0.7727.50
captures recorded in `provenance.json`. The complete source PNGs and their
independent repeats are unchanged. Every pixel outside each crop is white;
the test restores that verified complement and compares the whole frame.

Balance003 crops are copied byte for byte from the previously pinned assets.
OOF018 crops are a new lossless copy from the actual positioned phase captures.
No numerical tolerance or invented uniform fractional edge is used.

These guards run in the workspace tests. The paired expanded census of
23 October 2026 measures both tests exact at every profile; see
`docs/progress/column-paint-phases-2026-10-23.md`. Grid009 geometry is
separate work.
