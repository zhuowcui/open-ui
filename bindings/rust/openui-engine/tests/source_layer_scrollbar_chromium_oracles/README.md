# Viewport scrollbar Chromium references

These ten assets preserve actual Chromium 147.0.7727.50 frames for the
unchanged Engine `original_edge` constructors in modes 1 and 8 at five scales.
Both include an empty 10 × 1000 root sibling and a viewport scrollbar. Their
scrollbar pixels make the previous all-blue row expectation incorrect.

Each `.rgba` stores complete top rows followed by the lower right scrollbar
strip. Every unpacked pixel was verified opaque white against both independent
Chromium repeats, their observer-off captures and all saved capture attempts.
The test reconstructs and checks the complete physical frame with exact
dimensions, packed stride, byte length and every pixel, including white pixels.
`provenance.json` records full-frame hashes, dimensions, packing and immutable
input identities. No source PNG or RGBA input was changed.

Current-source diagnostic outputs match these two constructors at all five
scales. This does not qualify the canonical renderer, other modes, original
mutation histories, default path or release. Existing constructors, ownership
checks, containing-block checks and modes 0/4/7 row assertions are unchanged.
