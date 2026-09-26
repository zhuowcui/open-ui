# Open UI v0.2 rendering architecture

Open UI supports one renderer and one lifecycle. Rust and C are co-equal input
surfaces; neither calls through the other.

```text
Rust App/view! ───────────────┐
                              v
C ABI ───────────────> openui-engine <──── Linux/headless events
                              │
             typed style ─ layout ─ text ─ paint
                              │
                      immutable SceneSnapshot
                              │
             ┌────────────────┴────────────────┐
             v                                 v
     SoftwareCompositor                 render scheduler
       exact Skia raster                       │
             │                     OpenGL upload / softbuffer
             └─────────────────────────────────┘
                              │
                         presentation
```

## Ownership and threading

`Engine` owns the native document, authored/computed styles, stable handles,
controls, focus, pointer capture, selection, scrolling, animations, resources,
fragments, hit-test index, accessibility tree, and dirty generations. The API
is single-thread-affine. A handle contains document identity, slot index, and
generation; stale and cross-document use returns an error.

Mutations are batched in transactions. Invalidation metadata generated from
the property schema marks compositing, paint, layout, intrinsic-size, subtree,
or accessibility work. Unchanged frames do no style/layout/paint work.

Only an immutable `SceneSnapshot` crosses the scheduling boundary. Application
state, callbacks, documents, and reactive scopes never cross. Scene submission
coalesces to the newest complete generation.

## Rendering lifecycle

1. Typed mutations update retained document/control/animation state.
2. Dirty style fields are recomputed and inherited as required.
3. Layout produces and retains the complete fragment tree.
4. The hit-test index and accessibility bounds derive from those fragments.
5. Paint records exact draw order and raster parameters into a Skia picture.
6. The engine freezes pictures, geometry, damage, and generations in a scene.
7. Headless and native backends consume that same scene.

Headless rendering is deterministic: resources are synchronous, the viewport
and scale are explicit, and animation samples use a caller-supplied time.

## Interaction and accessibility

Hit testing walks back-to-front stacking order with transforms, clips,
scrolling, rounded overflow, and pointer eligibility. Normalized events route
through capture, target, and bubble phases, then ordinary control defaults.
Platform and headless-injected events share this pipeline.

Accessibility node IDs derive from stable engine handles. Initial and minimal
incremental updates use the same dirty generations as visual lifecycle work.
AccessKit actions re-enter ordinary event/control paths so keyboard, pointer,
and assistive operation cannot create divergent state.

## Platform and compositor

`openui-platform` is feature-gated, so headless builds do not pull X11 or
Wayland libraries. Winit owns the event loop; glutin provides EGL/GLX contexts;
softbuffer presents CPU frames; AccessKit supplies Linux AT-SPI integration.
Clipboard services use X11 selections on X11 and Wayland data devices on pure
Wayland.

The current OpenGL backend uploads the exact CPU Skia raster before presenting.
This preserves semantics and enables automatic software fallback, but direct
Skia GPU raster and retained per-node compositor layers remain release-candidate
work. Details are in `docs/v02/compositor.md` and `docs/v02/animations.md`.

## Historical boundary

The repository contains older GN/C++ Blink application code and SP2 Skia
experiments. They supplied reference knowledge and migration inventory only.
They are excluded from the supported workspace, packages, lifecycle, and API
contract. Git history is the compatibility archive; v0.2 does not maintain a
second production engine.
