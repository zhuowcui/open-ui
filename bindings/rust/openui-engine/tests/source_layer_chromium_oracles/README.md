# Source-layer Chromium frame oracles

These fifteen lossless RGBA crops come from Chromium 147.0.7727.50. Two independent browser captures agree for each initial, overlapping-alpha and fieldset fixture at all five scales. The source trees, styles, used rectangles, resources and physical viewport dimensions were checked before and after capture. `provenance.json` binds the exact PNGs, decoded frames, input HTML, source epochs and terminal receipts.

The test checks the complete frame. It asserts exact physical dimensions, stride and byte count; every pixel outside the stored ink crop must equal the measured opaque-white exterior. No pixel, channel or tolerance was adjusted.

Initial is the scene before adding its black child. Alpha is the overlapping subtree before adding the fieldset. Fieldset includes the original subtree and actual scrolled fieldset helper. All existing constructor and structural ownership assertions remain. Other fixtures retain their own expectations.

Historical canonical and exact-spine bytes remain preserved. Chromium is the correctness target; the old renderer differences are not a requirement for future output. These guards do not qualify default renderer selection, final Viz execution, presentation tracks or release readiness.
