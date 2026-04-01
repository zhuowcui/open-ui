# Sub-Project 8: Platform Expansion & Ecosystem

> Expand Open UI to macOS, Windows, mobile platforms. Build developer tooling and language ecosystem.

## Objective

Take the Linux-proven Open UI framework cross-platform and build the ecosystem that makes it a viable choice for production applications. This is the sub-project that transforms Open UI from a technically impressive extraction into a practical, widely-adopted UI framework.

## Tasks

### 8.1 macOS Port

**Rendering backend:**
- Metal backend for Skia (Skia already supports Metal)
- Alternative: MoltenVK (Vulkan → Metal translation layer)
- Evaluate performance trade-offs

**Platform integration:**
- Cocoa/AppKit windowing via Objective-C bridge
- NSWindow, NSView, CAMetalLayer for rendering surface
- macOS event handling (NSEvent → Open UI events)
- Retina display support (scale factor detection)
- macOS menu bar integration
- System color scheme detection (dark/light mode)

**System services:**
- Clipboard (NSPasteboard)
- Drag and drop (NSDragging)
- File dialogs (NSOpenPanel, NSSavePanel)
- Notifications (NSUserNotification)
- IME (Input Method Editor via NSTextInputClient)

**Accessibility:**
- NSAccessibility protocol implementation
- Map Open UI accessibility tree to macOS accessibility API
- VoiceOver compatibility testing

### 8.2 Windows Port

**Rendering backend:**
- Direct3D 12 via ANGLE or native Skia D3D backend
- Vulkan (via GPU drivers)
- Software fallback (GDI + Skia CPU rasterizer)

**Platform integration:**
- Win32 windowing (HWND, message loop)
- Or WinRT/UWP for modern Windows APIs
- DPI awareness (Per-Monitor DPI Aware V2)
- Windows 11 theming (Mica, Acrylic material, rounded corners)

**System services:**
- Clipboard (OLE clipboard)
- Drag and drop (OLE D&D)
- File dialogs (IFileDialog)
- Toast notifications
- IME (TSF — Text Services Framework)

**Accessibility:**
- UI Automation (UIA) provider implementation
- Narrator compatibility testing

### 8.3 Android Port

**Rendering:**
- Vulkan (preferred, wide support on modern Android)
- OpenGL ES fallback
- SurfaceView or TextureView for rendering surface

**Integration:**
- JNI bridge from Java/Kotlin → C → Open UI
- Android lifecycle management (Activity, Fragment)
- System back button handling
- Soft keyboard integration
- System bars (status bar, navigation bar) insets

**Platform services:**
- Android clipboard, sharing intents
- Permissions handling
- System theme detection

### 8.4 iOS Port

**Rendering:**
- Metal (required on iOS, no Vulkan/OpenGL)
- CAMetalLayer for rendering surface
- UIView integration

**Integration:**
- UIKit lifecycle (UIViewController, UISceneDelegate)
- Safe area insets
- System keyboard handling
- System gestures (swipe back, etc.)
- Dynamic Type (accessibility text sizing)

### 8.5 Developer Tooling

**Inspector / DevTools:**
- Standalone inspector application (built with Open UI itself!)
- Connect to a running Open UI app via IPC
- Features:
  - Visual node tree with highlight-on-hover
  - Computed style inspector (like Chrome DevTools Elements panel)
  - Layout visualization (show flex/grid lines, margins, padding)
  - Performance timeline (frame timing, layout/paint/composite breakdown)
  - Accessibility tree inspector
  - Node search and filtering

**Hot reload:**
- Watch source files for changes
- Recompile and reload the application's UI without full restart
- Preserve application state across reloads
- For Rust: integrate with `cargo watch` and dynamic library reloading

**Performance profiler:**
- Frame time graph (real-time)
- Per-stage breakdown: style (Xms) → layout (Xms) → paint (Xms) → composite (Xms)
- GPU utilization and memory
- Tile rasterization visualization
- Overdraw detection
- Jank detection (frames > 16.67ms)

**CLI tools:**
- `openui new <name>` — scaffold a new project
- `openui run` — build and run with hot reload
- `openui inspect` — launch inspector connected to running app
- `openui bench` — run performance benchmarks
- `openui doctor` — check system requirements and configuration

### 8.6 Additional Language Bindings

**Python (`openui-python`):**
- pybind11 or ctypes-based bindings
- Pythonic API with context managers and decorators
- `pip install openui`
- Target: rapid prototyping, data visualization, scientific tools

**Go (`openui-go`):**
- cgo-based bindings
- Go-idiomatic API with interfaces and goroutine integration
- `go get github.com/open-ui/openui-go`

**C# (`OpenUI.NET`):**
- P/Invoke bindings
- .NET-idiomatic API with LINQ, async/await
- NuGet package
- Target: enterprise applications, .NET shops

**Swift (`OpenUI-Swift`):**
- C interop (Swift can call C directly)
- SwiftUI-inspired API
- Swift Package Manager
- Target: macOS/iOS developers who want cross-platform

**Kotlin (`openui-kotlin`):**
- JNI/JNA bindings
- Kotlin-idiomatic API with coroutines, DSL builders
- Maven Central / Gradle dependency
- Target: Android developers, JVM ecosystem

### 8.7 Widget Library

A standard set of pre-built, themeable UI components:

**Basic:**
- Button (text, icon, icon+text)
- Text (label, paragraph, heading)
- TextInput (single-line, multi-line, password)
- Checkbox, RadioButton, Switch
- Slider, ProgressBar
- Image

**Navigation:**
- TabBar, TabView
- NavigationBar, NavigationStack
- Drawer, Sidebar
- Breadcrumbs

**Data display:**
- List, VirtualList (lazy loading for 100k+ items)
- Table (sortable, filterable)
- Tree view
- Card, Badge, Chip/Tag

**Overlay:**
- Dialog, Modal
- Toast, Snackbar
- Tooltip, Popover
- Menu, ContextMenu, Dropdown

**Layout:**
- Divider, Spacer
- Accordion, Collapsible
- SplitView

### 8.8 Documentation & Community

**Website:**
- Landing page (hero demo, features, benchmarks)
- Interactive playground (compile and render Open UI in-browser via WASM?)
- API reference (generated from rustdoc / Doxygen)
- Blog for announcements and deep dives

**Community:**
- GitHub Discussions for Q&A
- Discord or Matrix server
- Contributing guide with good first issues
- Regular releases with changelogs
- Security policy and vulnerability reporting

## Deliverables

| Deliverable | Description |
|---|---|
| macOS support | Full platform integration with Metal |
| Windows support | Full platform integration with D3D/Vulkan |
| Android support | JNI bridge, Vulkan/GLES rendering |
| iOS support | Metal rendering, UIKit integration |
| DevTools inspector | Visual debugging application |
| Hot reload | Change → see result without restart |
| Performance profiler | Frame timing and pipeline analysis |
| CLI tools | `openui` command-line tool |
| Python bindings | `pip install openui` |
| Go bindings | `go get` package |
| C# bindings | NuGet package |
| Swift bindings | SPM package |
| Kotlin bindings | Maven package |
| Widget library | 30+ standard UI components |
| Website & docs | Full documentation site |

## Success Criteria

- [ ] Same Open UI application runs on Linux, macOS, and Windows with no code changes
- [ ] Android and iOS ports render correctly on physical devices
- [ ] DevTools inspector can connect to and inspect a running application
- [ ] Hot reload works with < 1 second turnaround
- [ ] At least 3 language bindings published to their respective package managers
- [ ] Widget library covers the 80% use case for typical business applications
- [ ] Documentation site is live and comprehensive
- [ ] At least 10 community-contributed example applications

## Notes

This is the most expansive sub-project and will likely be broken into further sub-phases when the time comes. The ordering within this phase should be:

1. macOS + Windows ports (biggest impact, desktop-first)
2. DevTools + profiler (essential for developer adoption)
3. Widget library (essential for developer productivity)
4. Additional language bindings (broadens adoption)
5. Mobile ports (Android + iOS)
6. Community + ecosystem
