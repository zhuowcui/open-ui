# Threading model

Open UI's supported v0.2 API is single-thread-affine. Application state,
callbacks, the retained document, interaction state, style, layout, and paint
recording live on the UI thread. The compositor receives immutable scene
snapshots; it never receives mutable DOM state or application callbacks.

```text
UI thread                                      presentation owner
---------                                      ------------------
application callback
  -> Document transaction
  -> style/layout/paint invalidation
  -> immutable SceneSnapshot  ---------------> newest complete scene
                                                  -> software or OpenGL
                                                  -> present
platform event  <----------------------------- redraw/frame signal
```

## UI-thread ownership

An `Engine` captures its creating thread. Operations through `Document`,
`Element`, `TextNode`, and FFI handles validate this affinity and return an
explicit error on the wrong thread. Generation-checked handles also reject
stale or cross-document access. Application callbacks run only after internal
mutable borrows have ended, preventing reentrant aliasing.

`Document::transaction` batches mutations into one lifecycle update. Dirty
generations distinguish tree, intrinsic-size, layout, paint, compositing, and
accessibility work. An unchanged frame performs no style, layout, or paint
work.

## Scene handoff

`SceneSnapshot` is immutable and owns the data needed for presentation:
recorded content, clips, transforms, opacity, filters, scroll offsets, damage,
and accessibility bounds. Scene submission is coalesced, so presentation uses
the newest complete snapshot instead of building a backlog.

The Linux runtime currently presents on the winit event-loop thread. The
compositor API preserves an ownership boundary for moving GPU objects to a
dedicated render thread, but v0.2 release qualification must not claim that
work is complete until the retained GPU-layer and blocked-UI animation gates
pass.

## Scheduling

Redraws are requested by dirty lifecycle state, expose/resize events, or an
active animation deadline. Idle windows stop requesting frames. Resize and
surface recreation are synchronized before presentation; OpenGL failure is
reported and `Auto` may fall back to the software surface without changing the
scene.

Headless rendering has no platform event loop. It uses the same engine and
scene representation and samples its deterministic manual clock through
`render_at(time)`.

## Boundaries

- Mutable engine and application state never cross threads.
- A context or software surface is used only by its designated presentation
  owner.
- FFI callbacks follow the same non-reentrant mutable-borrow rule.
- Resource decoding is synchronous and deterministic in v0.2; networking is
  outside the product boundary.
- Compositor-thread animation is a release blocker until immutable curves and
  retained promoted layers can continue while the UI thread is blocked.

See [the rendering pipeline](rendering-pipeline-overview.md),
[current status](../progress/current-status.md), and
[release qualification](../v02/release.md) for the implemented surface and
open gates. Earlier Chromium threading research remains available in Git
history and is not the supported architecture.
