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
step positions, and ordered piecewise-linear stops. The animation's easing
transforms progress across the whole effect before selecting a keyframe
interval. That interval then applies its own easing, defaulting to linear.
An explicit keyframe curve composes with the whole-effect curve. Unsupported
value pairs fall back to deterministic discrete interpolation.

`Easing::linear_stops_percent` accepts owned `PercentageLinearStop` values and
preserves authored percentages, including repeated knots and knots outside
0% and 100%. The existing normalized `LinearStop` representation keeps its
validation and converts each input to percentage units by ordinary IEEE
multiplication. Cubic and linear curves extrapolate beyond the effect interval;
their outputs are not clamped. Ordered duplicate keyframe offsets select the
outgoing interval at an exact offset.

Step discontinuities use the effect's phase and actual iteration direction.
Pinned Chromium 147.0.7727.50 applies a separate phase rule to per-keyframe
steps: the left limit is selected in the before phase. Its reverse-after
keyframe limit differs from whole-effect easing. The shared native sampler
preserves those pinned rules; these source changes still require runtime and
Chromium qualification.

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

The source guards for these input and easing rules still need compilation and runtime
verification on the complete integrated checkpoint. They do not qualify every
animation operation.

Typed keyframe conversion reserves its owned storage fallibly. Allocation
failure retains its error type through the Engine, including `transition`, so
the C interface can report out of memory. This does not qualify every allocation
in animation creation or sampling; those paths still need ownership and failure
verification.

## Animation local clock

The Engine records a local time at a document-clock anchor. A running animation
advances that time by the elapsed document time multiplied by its playback rate.
The effect then samples the local time without applying the rate a second time.
Pausing, seeking and changing the rate preserve this same local time, including
when a native application seeks while paused and then reverses playback.

A zero playback rate holds the current picture and requests no animation ticks.
It can be resumed by assigning a nonzero rate. Finishing with a zero rate returns
an error. Forward playback holds the effect end; reverse playback holds zero.
An explicit seek beyond a finished boundary preserves the requested time, while
an ordinary tick holds the boundary. Playing a finished document animation
rewinds to its beginning or end according to the rate. An infinite forward effect
cannot be finished, but an already running infinite effect can be reversed and
finished at zero. Creating reverse playback at an infinite end is rejected.

`AnimationOptions::sample` retains its elapsed-time interpretation for direct
callers. The Engine samples its owned local clock and actual iteration direction
before applying whole-effect easing and per-keyframe easing. Animation
lifecycle callbacks follow effect phase transitions, including reverse completion;
a skipped group of iterations emits one event, and event elapsed time describes
the active effect interval rather than elapsed wall-clock time. Rust application
controls and both absolute and delta clock advances dispatch these callbacks
after releasing Engine borrows. Alternating direction is computed before the
public integer iteration count saturates; infinite current iteration is treated
as even. These rules are implemented from pinned Chromium 147.0.7727.50 source and have native guards,
including checks that a rate change preserves rendered bytes. Their compilation,
runtime and Chromium qualification are still pending.

## Retained scroll and view timelines

Scroll and view effects now use a post-layout snapshot from the same retained
Engine as native element mutations. Inline and block axes resolve through the
source's writing mode. Negative physical scroll origins use offset magnitude.
A view effect selects its nearest owned scroll container and measures the
subject before transforms, scrolling and sticky visual displacement. Its cover
range includes the source's scroll-padding and retained sticky constraints.

`TimelineRange` endpoints are native CSS-pixel attachment offsets. Scroll
attachments measure them from the scroll origin; view attachments measure them
from the cover-range start. Reversed endpoints retain their oriented native
range behavior; this older oriented native interface is not the complete browser
attachment-range interface. The required additive named attachment interface must
retain Chromium's clamped intrinsic fraction for reversed browser endpoints.
The internal progress clock follows the pinned Chromium scale
of 16 microseconds per CSS pixel. Cover-relative current time converts to integer
microseconds before attachment alignment; duration retains the pinned double
seconds calculation. Auto-aligned effects include the physical scroll endpoints
in their active phase, while an interior attachment end or an explicit seek keeps
its exclusive phase boundary. Finite and infinite effect timing normalizes
to the attachment range. Consequently `current_time_ms` on a progress effect
reports normalized internal milliseconds, while a document effect reports
ordinary document-clock milliseconds. Existing field and method layouts remain
unchanged.

`Document::animation_timeline_snapshot` returns owned range, physical-axis and
percentage-time data. Inactive timelines report absent resolved fields rather
than manufacturing a current time. `seek_animation_progress` takes percentage
current time relative to the whole resolved timeline duration; it retains an
inactive request until layout resolves the range. These are native Rust APIs.

Sticky layout now resolves in one final shared stage after owned scroll geometry
is retained. It follows used overflow for block, flex, grid, table and viewport
boxes; anonymous table helpers do not create authored scroll boundaries. Engine
scroll invalidation follows the same retained ownership.

Progress timelines sample when retained layout or scroll position changes;
they request no document-clock animation ticks. Pause holds percentage time,
play re-aligns to the attachment range, and rate changes use Blink's separate
nonmonotonic start-time rules. A finished effect can re-enter when its scroll
position reverses. These source changes and native guards are uncompiled and
unqualified. Named view ranges, complete continuation geometry, nested sticky
relationships and their Chromium measurements still require completion. The
new typed progress operations also require additive C transport and consumers.

Overlapping effects and cancellation also need qualification: each animation
retains an underlying value, updated by authored property setters, so restoring it must be reviewed
against surviving effects and later authored changes. The
[Web Animations timing model](https://www.w3.org/TR/web-animations-1/#the-current-time-of-an-animation)
defines the browser behavior used as the oracle. Applications perform the
equivalent operations through native Rust APIs; Open UI executes no JavaScript.

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


### Phase boundary timing

The source implementation follows pinned Chromium 147.0.7727.50 timing arithmetic:
local time is represented as seconds, snapped to the before boundary within one
microsecond, and then snapped to the after boundary if it has not already entered
the before phase. The same adjusted time supplies active time and progress.
This also governs fill and tiny-duration effects; the unchanged pixel gates have
zero tolerance. Native boundary, tiny-effect, and inactive-source guards are
written but have not been compiled or executed in this source checkpoint.
