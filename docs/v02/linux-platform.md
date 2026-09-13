# Linux platform runtime

The `openui` crate keeps native dependencies behind its opt-in `linux` feature.
Without that feature, `App::run` returns `PlatformUnavailable` and every
headless API remains usable without linking X11, Wayland, D-Bus, or OpenGL
libraries.

With `linux` enabled, `App::run` owns a winit 0.30 event loop supporting native
X11 and Wayland. Window, pointer, touch, wheel, keyboard, scale, resize, focus,
file-drop, and IME notifications are normalized into logical coordinates before
entering the framework. Application callbacks run after platform and engine
borrows have been released. Redraws are requested for input, resize, expose,
accessibility actions, or active animation; settled applications use winit's
waiting control flow.

Both presentation paths consume the exact frame produced from the engine's
immutable scene. `Software` presents it through softbuffer. `OpenGl` creates an
EGL or GLX surface through glutin and uploads the frame to a retained texture;
the GPU performs scale-correct presentation and buffer swaps. `Auto` attempts
OpenGL first and falls back to software during initialization or after a
presentation failure. Every backend selection and fallback is reported through
`PlatformEvent::BackendChanged`, including the diagnostic reason.

The pinned rust-skia revision cannot currently compile its optional Ganesh GL
API because its Rust `GpuStats` declaration is one field behind the bundled
Skia C++ declaration. W8 therefore preserves the exact CPU Skia raster and uses
OpenGL for composition/presentation. Direct picture replay into a Skia GPU
surface remains blocked until that pinned dependency is repaired; the engine,
scene generation, layout, damage, and resulting pixels are backend-independent.

Both paths tolerate zero-sized/minimized windows and resize their native
surfaces before presentation. The software compositor caches the last raster by
scene generation, so repeated presentation of an unchanged snapshot performs
no new Skia raster work. A one-slot `SceneMailbox` coalesces queued immutable
snapshots for render-thread integrations without allowing DOM or callbacks to
cross the thread boundary.

Clipboard ownership is backend-correct. X11 uses the CLIPBOARD selection and
UTF8_STRING through x11rb. Pure Wayland uses the seat data device through
smithay-clipboard; it does not require XWayland or the data-control protocol.
Copy, cut, and paste synchronize the native clipboard with the engine editing
pipeline.

The AccessKit winit adapter is created before the window is shown. Its Unix
backend publishes the retained semantic tree over AT-SPI, and action requests
return to the ordinary engine event/control path. IME is enabled on the native
window and preedit/commit events drive the same deterministic composition model
used by headless injection.

All four checked-in examples retain their deterministic headless default and
also build as native applications. For example:

```sh
cargo run -p counter --features linux
```

Select the backend through `AppBuilder::backend`. `Auto` prefers OpenGL,
`OpenGl` makes initialization or presentation failure explicit, and `Software`
forces softbuffer.
