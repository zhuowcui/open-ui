# Open UI v0.2 unsupported features

This list is part of the public v0.2 contract. Unsupported means the release
does not promise the feature, even when a low-level renderer type or historical
experiment exists in the repository.

## Platforms and graphics

- macOS, Windows, Android, iOS, and browser/WASM execution.
- Vulkan, Metal, and Direct3D presentation.
- Multiple physical-GPU/driver profiles beyond the declared Linux release
  qualification profile.
- Cross-process rendering or a browser-style GPU process.

## Browser services

- HTML loading, CSS text parsing, selectors, stylesheet injection, and a web
  cascade exposed as application APIs.
- JavaScript execution or script bindings inside documents are outside the
  product design, including future versions. Application behavior uses public
  native Rust methods and Rust event callbacks. WebAssembly execution inside
  documents is also unsupported.
- HTTP, URL fetching, cookies, browser navigation, storage, service workers,
  and developer tools.
- Embedded interactive documents, iframes, and media playback.

## Native widgets and system integration

- File, date/time, and color pickers.
- Camera, microphone, geolocation, notifications, and global shortcuts.
- Printing and PDF export.
- Rich drag sources with arbitrary cross-process MIME negotiation beyond the
  Linux event/data paths documented for v0.2.

## Text and accessibility limits

- Platform font parity outside the pinned Linux font inventory.
- A general rich-text/HTML editor; v0.2 provides production plain-text input
  and textarea editing, selection, clipboard, IME, undo, and redo.
- Accessibility adapters on non-Linux platforms.

## Current release-candidate gaps

These are implementation gaps, not accepted final-v0.2 omissions:

- direct Skia Ganesh raster builds behind an explicit selection but remains
  unqualified; OpenGL presentation currently uploads the CPU Skia frame;
- the current four-profile Chromium census and primitive matrix do not meet
  their exact gates;
- retained per-node compositor layers and compositor-owned immutable animation
  curves are incomplete;
- the strict 100 promoted animations while the UI thread is blocked gate has
  not been qualified;
- automated AT-SPI operation and physical-GPU context-loss qualification remain
  to be run in the release lab;
- the C ABI does not yet export the owned Linux event loop or a complete
  platform accessibility tree, so native X11/Wayland C application parity is
  incomplete even though retained engine/headless C parity is covered;
- full preserve-3d/backface layer semantics remain incomplete.

The release cannot be marked final until the release-candidate gaps are closed
or the product contract is explicitly revised and independently reviewed.
