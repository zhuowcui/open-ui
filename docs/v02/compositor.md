# Immutable compositor contract

`SceneSnapshot` is the only visual object shared beyond `openui-engine`. It
owns a generation-numbered Skia picture, retained fragment tree, viewport, and
damage list. It contains no application callback, signal, DOM borrow, or other
mutable UI-thread state.

`SoftwareCompositor` and the explicitly selected `GaneshGlCompositor` cache the
completed RGBA frame together with its immutable recording. Reusing that same
recording and generation returns the frame without replaying paint commands.
Scene generations belong to individual documents: equal generation numbers
from different documents do not identify the same frame. The cache retains
the recording until replacement, PNG rendering, or compositor destruction;
Ganesh releases its retained recording before its graphics context.
`SceneMailbox` is a synchronized one-slot
queue: a producer replaces any unconsumed scene, and a consumer receives only
the newest complete snapshot. The mailbox exposes deterministic submission and
coalescing counters for tests and diagnostics.

On Linux, `BackendPreference::Auto` selects an EGL/GLX OpenGL presenter when a
context and shaders can be initialized. The presenter retains its texture and
only reallocates texture storage when the logical frame size changes. Resize,
swap, shader, or context failures switch `Auto` to softbuffer and report the
reason; explicit `OpenGl` reports the error without silently changing backend.
See [linux-platform.md](linux-platform.md) for the pinned rust-skia limitation
that currently keeps exact Skia raster on the CPU before GPU presentation.
