# Object fallback host clipping

Pinned Blink's UA stylesheet gives `img`, `canvas`, `video`, `iframe`, `embed`,
and `object` a content-box `overflow: clip` host default. Open UI applies that
default to the first five native element kinds. Its `object` fallback children
still use native block flow and need separate host-clip qualification.

Applying the same default to `object` in the clean `d0860d3f` full census
changed one previously exact comparison:
`wpt/css_overflow/table-header-group-overflow-crash` at 1920×1080@1.5 lost a
96-pixel caption row at physical y=24. The source contains an `object` without
external data whose fallback is a table with a caption. The new content-box
clip cut that fallback paint. This is a renderer regression, even though a
different image fixture became exact and the overall exact count stayed flat.

The scoped repair at `6676cf60` defers the `object` host default while keeping
the other five defaults. The [clean six-case guard](generated/object-fallback-host-clip-v1.json)
covers every original fixture builder that creates an `object`, across all four
required profiles: 24/24 exact, zero errors. Its Open UI and Chromium images
are byte-identical to the pre-UA full census, including the restored caption
row. This selected guard does not replace a complete post-repair census.

This decision does not establish Chromium-equivalent computed style for a
native `object` with fallback content. Its fallback layout and clip ownership
must be reconciled before enabling that UA default. Open UI does not embed
interactive documents; consuming applications use native Rust element APIs.
