# Tiny Thin scrollbar arrow Chromium frames

These five RGBA crops are copied from the immutable Chromium 147.0.7727.50
ordinary empty Div captures in `native-control-ordinary-div-chromium-runtime-v4754`.
`provenance.json` pins both independent browser repeats, the original PNG and full
RGBA files, dimensions, crop rectangles, constructor and source revision.
The complete frame is the crop surrounded by opaque white; that reconstruction
matches every retained browser byte at every scale. No existing reference changes.

The native guard uses the existing Engine fixture with explicit replaced metadata
removal, Resize::None and both Thin scrollbars. Its retained TextArea role differs
from the browser Div. This guard measures the bounded complete frame through the
canonical compositor; it does not claim universal role, raster or compositor
dispatch equivalence. All new guards are authored UNRUN and await Root execution.
