# SP8: React-like Rust Developer Experience

## Goal
Create a production-quality Rust framework on top of Open UI's C API that feels
like writing React. Developers write JSX-like syntax with `view!{}` macros,
use reactive signals for state, and compose components — all rendering through
Chromium's Blink pipeline via our C API.

## Current State (post-SP7)
- 90 C API functions (SP1-SP7 complete)
- 222 unit tests, 39 pixel-perfect comparison pages
- Static library: `libopenui_lib.a` (links against Chromium)
- `bindings/rust/` exists but is empty (only `.gitkeep`)
- No Rust code yet — this is SP8's deliverable

## Architecture Overview

```
┌─────────────────────────────────────────────────────────┐
│  Application Code                                        │
│  #[component] fn App() -> impl IntoView { view!{...} }  │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│  openui-macros (proc macro crate)                        │
│  view!{} macro: JSX-like → Element construction code     │
│  #[component]: fn → component factory with props struct  │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│  openui (main crate) — safe Rust API + reactive runtime  │
│  ├── Reactive: Signal<T>, Memo<T>, Effect, Scope         │
│  ├── Elements: Document, Element (RAII wrappers)         │
│  ├── Renderer: Signal → targeted C API calls (no VDOM)   │
│  ├── Components: trait IntoView, children, props         │
│  └── App: mount(), run_loop(), event pump                │
└───────────────────────┬─────────────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────────────┐
│  openui-sys (FFI crate)                                  │
│  extern "C" fn declarations matching openui.h            │
│  build.rs links libopenui_lib.a + Chromium deps          │
└───────────────────────┬─────────────────────────────────┘
                        │ C ABI
┌───────────────────────▼─────────────────────────────────┐
│  libopenui_lib.a → Blink rendering pipeline              │
└─────────────────────────────────────────────────────────┘
```

## Key Architecture Decisions

### 1. Fine-Grained Signals (NOT Virtual DOM)
Leptos-style fine-grained reactivity. Component functions run **once**. Signals
track which DOM nodes depend on them and update **only those nodes** directly
via C API calls. No VDOM diffing, no component re-renders.

**Why:** Our C API can surgically update individual elements. VDOM diffing would
be wasted computation — we can go straight from signal change to `oui_element_set_style()`.

### 2. Copy Signals
`Signal<T>` implements `Copy`. This solves Rust's biggest ergonomic problem
with closures — no `.clone()` needed to move signals into event handlers.

```rust
let count = create_signal(0);
// count is Copy — can be used in multiple closures without cloning
let increment = move || count.set(count.get() + 1);
let display = move || format!("{}", count.get());
```

### 3. Proc Macro for JSX-like Syntax
`view!{}` is a proc macro (not macro_rules) because:
- Full JSX-like HTML parsing (angle brackets, attributes, children)
- Proper span information for error messages
- IDE support via rust-analyzer

### 4. HTML Element Names
Use actual HTML tag names so React/web developers feel at home:
```rust
view! {
    <div class="container">
        <h1>"Hello, Open UI!"</h1>
        <button on:click=move |_| count.set(count.get() + 1)>
            "Count: " {count}
        </button>
    </div>
}
```

### 5. Rendering Loop Ownership
The framework owns the render loop. `App::run()` pumps
`oui_document_begin_frame()` at the target frame rate. Platform event
integration is via a callback the user provides (or a built-in event source
for supported platforms).

### 6. No Windowing (Yet)
SP8 remains offscreen. The Rust API renders to bitmaps just like the C API.
Platform windowing (winit, SDL2) is SP9. For SP8, examples render to PNG
for verification, and tests use pixel comparison.

## Crate Structure

```
bindings/rust/
├── Cargo.toml              # Workspace root
├── openui-sys/             # Phase 1: Raw FFI bindings
│   ├── Cargo.toml
│   ├── build.rs            # Links libopenui_lib.a + Chromium deps
│   └── src/
│       └── lib.rs          # extern "C" fn declarations (all 90 C API functions)
│
├── openui/                 # Phases 2-6: Safe wrapper + reactive framework
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # Public re-exports, prelude
│       ├── document.rs     # Document struct (RAII wrapper)
│       ├── element.rs      # Element struct (RAII wrapper)
│       ├── style.rs        # Type-safe CSS property setters
│       ├── events.rs       # Event types, callback registration
│       ├── runtime.rs      # Reactive runtime (signal graph, scope stack)
│       ├── signal.rs       # Signal<T>, Memo<T>, Effect
│       ├── component.rs    # IntoView trait, component rendering
│       ├── renderer.rs     # Signal→DOM reconciliation, For/Show/DynChild
│       ├── app.rs          # App builder, mount(), render loop
│       └── prelude.rs      # Convenient re-exports
│
├── openui-macros/          # Phase 4: Proc macros
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs          # #[proc_macro] and #[proc_macro_attribute] entry points
│       ├── view.rs         # view!{} parser and code generator
│       └── component.rs    # #[component] attribute processor
│
└── examples/               # Phase 7: Example applications
    ├── hello/              # Minimal "Hello World"
    │   ├── Cargo.toml
    │   └── src/main.rs
    ├── counter/            # Reactive counter
    │   ├── Cargo.toml
    │   └── src/main.rs
    ├── todo/               # Todo list app
    │   ├── Cargo.toml
    │   └── src/main.rs
    └── dashboard/          # Complex multi-component dashboard
        ├── Cargo.toml
        └── src/main.rs
```

## Developer Experience (Target API)

### Hello World
```rust
use openui::prelude::*;

fn main() {
    App::new(800, 600)
        .render(|| view! {
            <div style:display="flex" style:justify-content="center"
                 style:align-items="center" style:height="100vh">
                <h1 style:color="#333" style:font-size="48px">
                    "Hello, Open UI!"
                </h1>
            </div>
        })
        .render_to_png("hello.png");
}
```

### Reactive Counter
```rust
use openui::prelude::*;

#[component]
fn Counter(initial: i32) -> impl IntoView {
    let count = create_signal(initial);

    view! {
        <div class="counter" style:text-align="center" style:padding="20px">
            <h2>"Counter: " {count}</h2>
            <button on:click=move |_| count.set(count.get() + 1)
                    style:padding="10px 20px" style:font-size="16px">
                "Increment"
            </button>
            <button on:click=move |_| count.set(0)
                    style:margin-left="10px" style:padding="10px 20px">
                "Reset"
            </button>
        </div>
    }
}

fn main() {
    App::new(400, 300)
        .render(|| view! { <Counter initial=0 /> })
        .render_to_png("counter.png");
}
```

### Todo App
```rust
use openui::prelude::*;

#[component]
fn TodoApp() -> impl IntoView {
    let todos = create_signal(vec![
        "Learn Rust".to_string(),
        "Build with Open UI".to_string(),
    ]);
    let input_value = create_signal(String::new());

    let add_todo = move |_| {
        let val = input_value.get();
        if !val.is_empty() {
            todos.update(|list| list.push(val.clone()));
            input_value.set(String::new());
        }
    };

    view! {
        <div style:max-width="400px" style:margin="0 auto" style:padding="20px">
            <h1>"Todo List"</h1>
            <div style:display="flex" style:gap="8px">
                <input type="text" placeholder="What needs to be done?"
                       prop:value=input_value
                       on:input=move |e| input_value.set(e.target_value())
                       style:flex="1" style:padding="8px" />
                <button on:click=add_todo style:padding="8px 16px">"Add"</button>
            </div>
            <ul style:list-style="none" style:padding="0">
                <For each=move || todos.get().into_iter().enumerate()
                     key=|(i, _)| *i
                     children=move |(i, todo)| {
                    view! {
                        <li style:padding="8px" style:border-bottom="1px solid #eee">
                            {todo}
                            <button on:click=move |_| todos.update(|list| { list.remove(i); })
                                    style:float="right" style:color="red">"×"</button>
                        </li>
                    }
                } />
            </ul>
        </div>
    }
}
```

### Conditional Rendering & Dynamic Styles
```rust
view! {
    // Conditional rendering
    <Show when=move || count.get() > 0 fallback=|| view! { <p>"No items"</p> }>
        <p>"Items: " {count}</p>
    </Show>

    // Dynamic styles (signal-driven)
    <div style:background-color=move || if active.get() { "#4CAF50" } else { "#ccc" }
         style:opacity=move || if visible.get() { "1" } else { "0" }
         style:transition="all 0.3s ease">
        "Dynamic box"
    </div>
}
```

## Phases

### Phase 1: openui-sys — Raw FFI Bindings
Hand-written `extern "C"` declarations for all 90 C API functions. No bindgen
(our header is clean enough to write by hand, and hand-written gives us better
Rust types).

**Deliverables:**
- `openui-sys/src/lib.rs` — All 90 functions as `unsafe extern "C" fn`
- `openui-sys/build.rs` — Links `libopenui_lib.a` + transitive Chromium deps
- `openui-sys/Cargo.toml`
- Workspace `Cargo.toml`

**Key challenge:** Linking. We must discover and link all transitive Chromium
dependencies. `build.rs` will:
1. Read the library output from a configurable env var `OPENUI_LIB_DIR`
   (defaults to `~/chromium/src/out/Release`)
2. Parse the ninja build graph to find all `.a` files that `openui_lib` depends on
3. Emit `cargo:rustc-link-lib=static=...` for each

**Validation:** `cargo build` succeeds. A trivial test calls `oui_init()` +
`oui_shutdown()`.

### Phase 2: openui — Safe Element & Document Wrappers
RAII wrappers around the C API handles. `Element` and `Document` implement
`Drop` to call the C cleanup functions. Type-safe style setters.

**Deliverables:**
- `document.rs` — `Document::new(w, h)`, `.body()`, `.layout()`, `.render_to_png()`,
  `.begin_frame()`, `.load_html()`, etc. All C doc functions wrapped.
- `element.rs` — `Element::create(doc, tag)`, `.append_child()`, `.set_style()`,
  `.set_text()`, `.set_attribute()`, etc. All C element functions wrapped.
  Implements `Drop` to call `oui_element_destroy()`.
- `style.rs` — Type-safe CSS property methods:
  ```rust
  elem.style().width(px(200)).height(pct(100.0)).display(Display::Flex);
  ```
- `events.rs` — Event callback registration with closures:
  ```rust
  elem.on("click", |event| { /* ... */ });
  ```

**Validation:** Can build the element tree equivalent of the SP6 pixel test
pages entirely from Rust, render to PNG, and get identical output.

### Phase 3: Reactive Runtime — Signals, Memo, Effect
The core reactive primitives. No UI concerns — pure reactive graph.

**Deliverables:**
- `runtime.rs` — `Runtime` struct with:
  - Arena-allocated signal storage (generational indices for `Copy` signals)
  - Scope stack for automatic cleanup
  - Dependency tracking (which effects depend on which signals)
  - Batched update queue
- `signal.rs`:
  - `create_signal<T>(initial) -> Signal<T>` — returns `Copy` handle
  - `Signal<T>::get(&self) -> T` (where T: Clone) — reads + subscribes
  - `Signal<T>::set(&self, val: T)` — writes + notifies dependents
  - `Signal<T>::update(&self, f: impl FnOnce(&mut T))` — in-place mutation
  - `create_memo<T>(f: impl Fn() -> T) -> Memo<T>` — derived, cached
  - `create_effect(f: impl Fn())` — side effect, runs on dependency change

**Implementation detail — generational arena:**
```rust
struct Signal<T> {
    id: GenerationalIndex,  // u32 index + u32 generation
    _marker: PhantomData<T>,
}
impl<T> Copy for Signal<T> {}
impl<T> Clone for Signal<T> { fn clone(&self) -> Self { *self } }
```
Signals are indices into a thread-local arena. The runtime resolves the index
to the actual value. This makes `Signal<T>` a 8-byte `Copy` type regardless
of `T`.

**Validation:** Unit tests for signal get/set, memo caching, effect tracking,
nested effects, batch updates, scope cleanup.

### Phase 4: view! Macro — JSX-like Proc Macro
The crown jewel. Parses JSX-like syntax and emits element construction code.

**Deliverables:**
- `openui-macros/src/view.rs` — Parser + code generator
- `openui-macros/src/component.rs` — `#[component]` attribute
- `openui-macros/src/lib.rs` — Entry points

**Syntax supported:**
```rust
view! {
    // Static HTML elements
    <div class="foo" id="bar">
        <span>"text content"</span>
    </div>

    // Dynamic text (signal interpolation)
    <p>"Count: " {count}</p>

    // Event handlers
    <button on:click=move |_| count.set(count.get() + 1)>"Click"</button>

    // Style props (static)
    <div style:width="100px" style:background-color="red" />

    // Style props (dynamic — signal-driven)
    <div style:opacity=move || if visible.get() { "1" } else { "0" } />

    // HTML attributes
    <input type="text" placeholder="Enter text" />

    // Boolean attributes
    <input disabled />

    // Component instantiation
    <Counter initial=42 />

    // Children
    <Card>
        <h2>"Title"</h2>
        <p>"Body"</p>
    </Card>

    // Conditional rendering
    <Show when=move || logged_in.get()
          fallback=|| view! { <p>"Please log in"</p> }>
        <Dashboard />
    </Show>

    // List rendering
    <For each=move || items.get()
         key=|item| item.id
         children=move |item| view! { <TodoItem item=item /> } />
}
```

**Code generation:** The macro expands `<div class="foo">{count}</div>` into:
```rust
{
    let __el = Element::create(&__doc, "div");
    __el.set_attribute("class", "foo");
    // For {count}: create an effect that updates the text node when count changes
    let __text = Element::create(&__doc, "span");
    create_effect(move || {
        __text.set_text(&count.get().to_string());
    });
    __el.append_child(&__text);
    __el
}
```

**Validation:** Macro compiles. Generated code builds. Counter example renders
correctly to PNG.

### Phase 5: Component System
Components are functions that return `impl IntoView`. The `#[component]`
attribute generates a props struct and factory.

**Deliverables:**
- `component.rs` — `IntoView` trait, `View` enum, component rendering
- `renderer.rs` — `Show`, `For`, `DynChild` components
- Props derivation in the `#[component]` macro

**`#[component]` expansion:**
```rust
// Input:
#[component]
fn Counter(initial: i32, #[prop(optional)] label: String) -> impl IntoView { ... }

// Expands to:
#[allow(non_camel_case_types)]
struct CounterProps {
    initial: i32,
    label: Option<String>,
}
fn Counter(props: CounterProps) -> impl IntoView {
    let initial = props.initial;
    let label = props.label.unwrap_or_default();
    // ... original body
}
```

**`For` component — keyed list rendering:**
```rust
pub fn For<T, K, V>(
    each: impl Fn() -> Vec<T> + 'static,
    key: impl Fn(&T) -> K + 'static,
    children: impl Fn(T) -> V + 'static,
) -> impl IntoView
where
    K: Eq + Hash + 'static,
    V: IntoView,
```
When the signal changes, `For` diffs the key list and:
- Removes elements for deleted keys
- Creates elements for new keys
- Reorders existing elements (moves in DOM)

**Validation:** Component props work. `For` renders lists and updates correctly
when items are added/removed. `Show` conditionally renders.

### Phase 6: App Shell & Render Loop
The entry point that ties everything together.

**Deliverables:**
- `app.rs` — `App::new(w, h)`, `.render(component)`, `.render_to_png(path)`,
  `.run_frames(n)`, `.dispatch_click(x, y)`, etc.

**App API:**
```rust
let app = App::new(800, 600);
app.render(|| view! { <Counter initial=0 /> });

// For testing: simulate interaction
app.dispatch_click(200.0, 150.0);
app.run_frames(1);  // Process the click, re-render
app.render_to_png("after_click.png");

// For real apps (future — SP9):
// app.run();  // Enters event loop with winit/SDL2
```

**Validation:** Full round-trip: create app, mount component, render PNG,
verify pixel output matches expected.

### Phase 7: Examples & Integration Tests
Build the showcase applications and verify they all render correctly.

**Deliverables:**
- `examples/hello/` — Static content, no state
- `examples/counter/` — Reactive state, event handling
- `examples/todo/` — List rendering with `For`, input binding
- `examples/dashboard/` — Multi-component, complex layout, dynamic styles

**Each example:**
1. Compiles with `cargo build`
2. Runs and produces a PNG
3. PNG is pixel-compared against the HTML equivalent rendered through
   `oui_document_load_html()` (same Blink pipeline → should match)

**Validation:** All 4 examples compile, run, produce correct output.
README updated with example screenshots.

### Phase 8: Testing & Multi-Model Review
- All Rust code compiles with zero warnings (`#![deny(warnings)]`)
- Unit tests for: signals (15+), memo (5+), effects (10+), element wrappers (10+),
  macro expansion (10+), component rendering (10+), For/Show (10+)
- Integration tests: each example produces correct PNG output
- `cargo test` passes all tests
- `cargo clippy` clean
- Multi-model review (Opus 4.6 + GPT-5.4) confirms:
  - All reactive primitives are correct (no leaks, no stale subscriptions)
  - Macro handles all documented syntax
  - No unsafe code except in openui-sys
  - Error handling is complete
  - API is ergonomic and production-quality

## Exit Conditions
1. `cargo build` succeeds for workspace + all examples (zero warnings)
2. `cargo test` passes 70+ tests
3. `cargo clippy` clean
4. All 4 examples render correct PNG output
5. Signals: get/set/update, memo caching, effect auto-tracking all work
6. view! macro: all documented syntax compiles and renders correctly
7. #[component]: props, optional props, children all work
8. For/Show: dynamic list rendering, conditional rendering work
9. Events: on:click and other handlers dispatch through Blink correctly
10. Multi-model review confirms completeness and quality
11. README updated with examples and getting-started guide

## Dependency Graph

```
Phase 1 (openui-sys) ─── Phase 2 (safe wrappers) ───┐
                                                      ├── Phase 4 (view! macro)
Phase 3 (signals) ────────────────────────────────────┘        │
                                                               ▼
Phase 5 (components) ← Phase 4 (view! macro)
         │
         ▼
Phase 6 (app shell) ← Phase 2 + Phase 3 + Phase 5
         │
         ▼
Phase 7 (examples) ← Phase 6
         │
         ▼
Phase 8 (testing & review) ← All phases
```

## Notes

- **No VDOM diffing algorithm.** Signals track dependencies at the granularity
  of individual DOM operations. When `count.set(5)` is called, only the text
  node showing the count is updated — not the entire component tree.

- **Thread model:** Single-threaded. The reactive runtime uses thread-local
  storage. Blink is also single-threaded. This simplifies everything.

- **Linking complexity.** The hardest part of Phase 1 is discovering all
  transitive Chromium library dependencies for the linker. We'll use ninja's
  dependency graph or manually enumerate the ~20-30 static libraries needed.

- **Unsafe boundary.** All `unsafe` code is confined to `openui-sys`. The
  `openui` crate is 100% safe Rust (except for the FFI calls, which are
  wrapped in safe abstractions with lifetime guarantees).

- **No async/await.** Reactivity is synchronous. Signal updates propagate
  immediately (batched within a single frame). Async data fetching is out of
  scope for SP8.

- **SP9 will add windowing.** SP8 renders to bitmaps. SP9 adds winit/SDL2
  integration for real windows, event loops, and live rendering.
