# Typed animations and deterministic clocks

Open UI v0.2 represents animation values with the same `StyleProperty` and
`StyleValue` schema used for ordinary mutations. There is no animation-only CSS
parser or string property path. `Keyframes<T>` preserves the Rust value type
until `PropertyKeyframes::typed` validates it against generated property
metadata; C keyframes pass through the same generated value-tag validation.

`AnimationOptions` covers delay (including negative delay), duration, finite or
infinite iteration count, normal/reverse/alternate direction, fill mode,
running/paused state, playback rate, replace/add/accumulate composition, and a
typed easing curve. Available curves are linear, cubic Bézier, all four CSS
step positions, and ordered piecewise-linear stops. Per-keyframe easing
overrides the animation default for its interval. Unsupported value pairs fall
back to deterministic discrete interpolation.

The engine owns generation-checked `AnimationId` values and samples them from a
manual document clock. It also exposes scroll and view timelines with typed
axes and ranges. Pause, play, seek, playback-rate change, finish, cancellation,
iteration boundaries, removal of animated nodes, and reduced-motion completion
are exposed as native operations. Their complete behavior still requires
qualification against pinned Chromium. Lifecycle events use the normal
capture/target/bubble event path in both Rust and C. Smooth scrolling and snap settling share the clock and
easing implementation.

Headless callers use `HeadlessApp::render_at`; native Linux redraws continue
only while `Document::is_animating()` is true. Documents without animation instances
use the ordinary static paint path.

## Input validation and remaining timing work

Easing curves built directly through the public Rust enum use the same
validation as the checked constructors. Animation options, per-keyframe
overrides, smooth scrolling and snap settling validate before retaining a
curve. An invalid curve must return an error without replacing a running
scroll animation or publishing a changed scene. A snap request validates its
timing even when it would otherwise be a no-op. Scroll and view timelines
validate their directly constructed ranges before retaining an animation;
finite reversed ranges remain supported.

The source guards for these input rules still need compilation and runtime
verification on the complete integrated checkpoint. They do not qualify every
animation operation. In particular, playback-rate changes currently retain
unscaled elapsed time while the effect sampler applies the new rate. Seeking
to 80 ms and then reversing a 100 ms opacity animation can therefore change
its sampled value immediately. Local-time preservation, reverse completion,
zero-rate behavior, scroll/view timeline axes and lifecycle event boundaries
need shared repairs and exact Chromium qualification. The
[Web Animations timing model](https://www.w3.org/TR/web-animations-1/#the-current-time-of-an-animation)
defines the browser behavior used as the oracle; consuming Open UI applications
perform the equivalent operations through native Rust APIs.

## Current compositor boundary

The immutable scene is still recorded as a document picture, so animation
sampling currently occurs on the UI thread before a new scene is submitted.
OpenGL presents that exact Skia raster but does not yet own promoted per-node
transform, opacity, filter, or scroll curves. Consequently the W9 qualification
case where 100 promoted layers continue at 60 Hz while the UI thread is blocked
is not claimed. Completing it requires retained per-layer pictures and GPU-side
curve application; it must not be simulated with global picture transforms or
backend-specific layout.

The renderer currently stores a 2D projected transform in `ComputedStyle`.
Typed transform lists accept translate/scale/rotate/matrix operations in both
2D and 3D plus perspective, and compatible lists interpolate deterministically.
Depth preservation, `preserve-3d`, backface visibility, and perspective-origin
remain gated on a real 3D layer tree rather than being approximated in paint.
