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
attachment-range interface. The additive typed interface described below retains
Chromium's clamped intrinsic fraction for reversed browser endpoints.
The internal progress clock follows the pinned Chromium scale
of 16 microseconds per CSS pixel. Cover-relative current time converts to integer
microseconds before attachment alignment; duration retains the pinned double
seconds calculation. Auto-aligned effects include the physical scroll endpoints
in their active phase, while an interior attachment end is exclusive. Boundary
flags are retained with normalized timing: an explicit seek at unchanged geometry
does not replace them. Setting an unchanged attachment range also retains a
resolved manual start and these flags. Genuine cover/view geometry, changed
attachment endpoints, playback rate and newly resolved automatic-start changes
rebuild that context. Thus a naturally
held zero-duration effect can remain active with progress one and `fill: none`;
seeking beyond its zero end still restores the underlying value. Finite and
infinite effect timing normalizes
to the attachment range. Consequently `current_time_ms` on a progress effect
reports normalized internal milliseconds, while a document effect reports
ordinary document-clock milliseconds. Existing field and method layouts remain
unchanged.

`Document::animation_timeline_snapshot` returns owned range, physical-axis and
percentage-time data. Inactive timelines report absent resolved fields rather
than manufacturing a current time. `seek_animation_progress` takes percentage
current time relative to the whole resolved timeline duration; it retains an
inactive request until layout resolves the range. These native operations are available to Rust and C applications. The C routes are
`oui_document_seek_animation_progress_v1` and
`oui_document_get_animation_timeline_snapshot_v1`.

Sticky layout now resolves in one final shared stage after owned scroll geometry
is retained. It follows used overflow for block, flex, grid, table and viewport
boxes; anonymous table helpers do not create authored scroll boundaries. Engine
scroll invalidation follows the same retained ownership.

Progress timelines sample when retained layout or scroll position changes;
they request no document-clock animation ticks. Pause holds percentage time,
play re-aligns to the attachment range, and rate changes use Blink's separate
nonmonotonic start-time rules. A finished effect can re-enter when its scroll
position reverses. These source changes and native guards are uncompiled and
unqualified. The named range interface is now implemented in source; complete
continuation geometry, nested sticky relationships and Chromium measurements
still require completion. The
typed progress operations have additive C transport through the same Engine.
Complete timing, geometry and rendering qualification still requires recorded
Chromium and native evidence.

Overlapping effects share a retained, unanimated base for each target and
property. Native setters update that base. The Engine folds raw samples in
animation creation order, resolves the final value once, and recomputes surviving
effects when one is cancelled or its progress source becomes inactive. Computed
renderer values and native endpoints enter the same typed composition path.
These source changes still require compilation, runtime and Chromium comparison;
currentcolor, modern color spaces and compound typography interpolation remain
unfinished. Applications control these effects through native Rust APIs.
Open UI executes no JavaScript.

## Current compositor boundary

The immutable scene is still recorded as a document picture, so animation
sampling currently occurs on the UI thread before a new scene is submitted.
OpenGL presents that exact Skia raster but does not yet own promoted per-node
transform, opacity, filter, or scroll curves. Consequently the W9 qualification
case where 100 promoted layers continue at 60 Hz while the UI thread is blocked
is not claimed. Completing it requires retained per-layer pictures and GPU-side
curve application; it must not be simulated with global picture transforms or
backend-specific layout.

Layout and paint retain full transform matrices and their source arithmetic:
axis-aligned operations keep float scale and translation; explicit matrix and
3D operations keep full matrices. Transform origins use each owned physical
reference box. Native client geometry, hit testing and scroll reveal consume the
same retained transforms, including inverse plane projection and singularity
checks before flat boundaries. These changes have source review and authored
native guards; compilation, runtime and exact Chromium measurements are pending.
Depth preservation, `preserve-3d`, backface visibility, perspective-origin and
3D paint/hit ordering still require completion.


## Phase boundary timing

The source implementation follows pinned Chromium 147.0.7727.50 timing arithmetic:
local time is represented as seconds, snapped to the before boundary within one
microsecond, and then snapped to the after boundary if it has not already entered
the before phase. The same adjusted time supplies active time and progress.
This also governs fill and tiny-duration effects; the unchanged pixel gates have
zero tolerance. Native boundary, tiny-effect, and inactive-source guards are
written but have not been compiled or executed in this source checkpoint.


## Browser attachment and named ranges

`Element::animate_on_scroll_range` and `animate_on_view_range` use
`AnimationAttachmentRange`, with optional normal endpoints or typed
`TimelineRangeOffset` values. Names cover `Cover`, `Contain`, `Entry`,
`EntryCrossing`, `Exit`, `ExitCrossing`, and `Scroll`. Offsets use the existing
native `LengthValue` API, including pixels, percentages, a percentage plus pixel
calculation, and selected-font or viewport units. They resolve against the
animation target's current style and the retained scroll/view geometry. Explicit
view visibility insets use `animate_on_view_range_with_inset` and `ViewTimelineInset`;
relative lengths use the subject's style, while an absent side uses the source's
logical scroll padding. Insets and offsets use Chromium's LayoutUnit and float
arithmetic. No JavaScript executes.

`Document::animation_attachment_snapshot` returns an owned descriptor and optional
resolved named ranges and absolute CSS pixel attachment bounds. Inactive geometry
keeps the descriptor and leaves its bounds unresolved. The original timeline
snapshot remains available; its range values are cover-relative pixel offsets
when active. For an inactive browser attachment those two scalar fields are zero
and must be read together with `active`; the additive snapshot carries optional
bounds explicitly. `set_animation_attachment_range` changes endpoints on the
same retained animation. Paused effects keep their hold time; changed endpoints
resume automatic running attachment alignment. Equal endpoints preserve an
already resolved manual start. Invalid offsets leave the descriptor and
animation unchanged.

The original `TimelineRange` API retains oriented numeric endpoints. The new
browser interface uses Chromium's nonnegative intrinsic active fraction for
reversed/equal endpoints and preserves independent entry/exit crossing distances
for sticky subjects. It does not reinterpret an existing numeric range. The
`Scroll` name corresponds to pinned Chromium's
`ScrollTimelineNamedRangeScrollEnabled` feature; its browser capture flag remains
part of qualification evidence.

These additions are source work, not a native API qualification result. The C creation, range-update and owned-snapshot transports are implemented.
Complete fragmented subject/source geometry, nested sticky cases,
shared effect-stack runtime verification, compilation and exact Chromium/native
measurements remain required until their evidence is recorded.

### Ordering controls on inactive progress sources

The retained native API stores an explicit percentage seek while source geometry is inactive. A later successful millisecond seek, play, finish or changed running attachment range replaces that deferred request. Pause and playback-rate changes preserve the requested held percentage; rejected nonfinite controls leave it intact. Native Engine and Rust application guards cover activation after each order. These guards remain uncompiled and unexecuted; the independent v1 source review identified the stale-request defect and v2 corrects it.


### Fragmented principal-box geometry (source checkpoint)

Layout retains the first logical inline size, each continuation's consumed block
size and the generating box's block-end separately from decoration slicing.
View timeline sizes consume this retained stitched geometry. Cloned decorations
include their repeated border and padding in the stitched size; visible
continuations past a definite box's end do not enlarge that box. Block and
inline-formatting continuation routes share this metadata. Native guards cover
sliced versus cloned edges, definite-box overflow, logical vertical axes,
inline-formatting tokens, static picture bytes and hit results at five scales.
An inline line-resume token tracks source lines independently of principal-box
consumption: each nonfinal principal part reaches its fragmentainer edge. Three
60-pixel lines in 100-pixel fragmentainers therefore have a stitched block size
of 260 pixels, while the unfragmented block remains 180 pixels. The first source
review found that v1 conflated these counters; v2 records each final principal
part in its block-layout owner. These guards are written but uncompiled and
unexecuted.

This is not complete fragmentation qualification. Extracted spanner wrapper
consumption, repeated table sections, promoted positioned descendants, variable
fragmentainer inline sizes and nested sticky/fragmentation combinations remain
required source and native/Chromium measurement work. Static paint and input
still consume the actual fragment boxes; metadata does not change those boxes.


### Shared retained sticky constraints (source checkpoint)

Sticky positioning now resolves one authored element from its retained stitched
box (or inline line union). Anonymous line, column and table helpers retain
traversal and physical placement without becoming the sticky containing block.
Unsliced used border and padding are retained before continuation decoration
removal, so the principal padding rectangle does not inherit a missing last
edge from the first piece. The real block's padding rectangle supplies that
boundary; its padding and the
target's computed margins are excluded as in pinned Chromium 147. Insets use
the scrollport's content rectangle, with the actual scroll origin and floored
scroll position kept in their source operation order. Oversized boxes reduce
the end inset using the containing block's writing direction.

Nested constraints retain independent box-only and containing-block ancestor
channels. Each real continuation receives the same authored sticky translation,
while retaining its original parent-relative offset for timeline snapshots and
repeated layout barriers. Engine guards cover bordered/padded scrollports,
containing-block margins, nested sticky blocks, anonymous column continuations,
no-inset pixel/hit neutrality and unchanged layout work at five scales. Shared
layout guards also exercise an inline ancestor between the sticky box and its
block. These guards are uncompiled and unexecuted; this is no qualification
claim or updated full renderer count.

Required follow-up includes fieldset anonymous-content scrolling ownership
(the legend must remain outside that scrolled content), promoted positioned
ancestor routing, repeated table sections, extracted spanner wrappers and
nested sticky View range measurements. Pinned Chromium 147 itself applies the
first sticky ancestor in ViewTimeline and records a TODO for nested sticky
elements; the native snapshot guard retains that source behavior. All
native/Chromium geometry, hit,
static pixel and full census gates remain required. Open UI runs no JavaScript;
these operations belong to the retained native Engine used by applications.


## Explicit native scroll-animation actions (source checkpoint)

Rust applications use `Element::settle_scroll_snap` with physical X/Y point
arrays, duration and easing, then `Document::cancel_smooth_scroll` with a returned
scroll ID. The additive C routes are
`oui_element_settle_scroll_snap_with_easing_v1` and
`oui_document_cancel_smooth_scroll_v1`. They call the same Engine actions; there
is no second snap-selection or easing implementation. The existing
`oui_element_settle_scroll_snap_v1` remains the immediate retained CSS-target
operation.

C arrays and curves are copied before the action. Empty axes retain their current
offset; finite point order, ties and clamping follow the shared Rust operation.
No-op still validates duration and easing, then returns ID0 with
`has_animation=0`. Other successful calls return a document-scoped numeric scroll
ID. A zero-duration/reduced-motion action can complete immediately, so its ID is
no longer live for cancellation. Cancellation preserves the current offset and
unrelated tracks; completed or unknown IDs report the shared native error.
Validation failures preserve outputs and the existing track. Scalar output
storage must be distinct, but may alias caller inputs after the owned copy.

These additions preserve the preceding 160 exports and existing C layouts. Their
Rust/FFI guards and C11/C++17 consumers are authored source, not execution or
rendering qualification. Root's complete-checkout native and ABI gates remain
required. Open UI executes no JavaScript.
