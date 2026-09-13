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

The W7 presentation backend rasterizes the engine's immutable scene through the
shared software compositor and presents it through softbuffer. It tolerates
zero-sized/minimized windows, resizes the surface before each present, and
scales logical frames to the current physical surface. W8 adds OpenGL; an
explicit OpenGL preference currently returns a diagnostic error while `Auto`
selects software.

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

Select the backend through `AppBuilder::backend`. `Auto` and `Software` use
the W7 CPU presenter; `OpenGl` is rejected with an explicit diagnostic until
the W8 compositor is enabled.
