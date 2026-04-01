# Sub-Project 7: Rust Bindings & Developer Experience

> Idiomatic Rust crate that makes building UIs with Open UI feel native to Rust developers.

## Objective

Produce the `openui` Rust crate — a safe, ergonomic, and idiomatic Rust wrapper over the Open UI C API. This includes raw FFI bindings (`openui-sys`), safe abstractions (`openui`), and proc macros (`openui-macros`) for a declarative UI syntax. The goal is that a Rust developer's experience of building an Open UI app should feel as natural as using SwiftUI feels to a Swift developer.

## Crate Architecture

```
openui (workspace)
├── openui-sys/          # Raw FFI bindings (auto-generated)
├── openui/              # Safe, idiomatic Rust API
└── openui-macros/       # Proc macros for declarative UI syntax
```

## Tasks

### 7.1 `openui-sys` — Raw FFI Bindings

**Auto-generated with `bindgen`:**
- Input: `include/openui/openui.h` and all sub-headers
- Output: `openui-sys/src/bindings.rs`
- Build script (`build.rs`) links against `libopenui.so`

**Considerations:**
- Opaque pointer types → Rust opaque types
- `OuiStatus` → checked in safe wrapper
- Callback function pointers → raw function pointers
- Run `bindgen` at build time or check in generated code (prefer build time for freshness)

### 7.2 `openui` — Safe Rust API

**Core design principles:**
- **Ownership**: Handles have clear owners. `Drop` calls destroy.
- **Borrowing**: Read-only access through `&` references where appropriate.
- **No raw pointers in public API**: Everything is wrapped in safe types.
- **Error handling**: `Result<T, OuiError>` everywhere.
- **Builder pattern**: Fluent node construction.

**Type mappings:**

| C Type | Rust Type |
|---|---|
| `OuiApp*` | `App` (owns, `Drop` calls `oui_app_destroy`) |
| `OuiWindow*` | `Window` (owns, tied to `App` lifetime) |
| `OuiNodeDesc*` | `NodeDesc<'a>` (borrows style rules, callbacks) |
| `OuiStyleRule*` | `StyleRule` (owns) |
| `OuiStyleContext*` | `StyleContext` (owns) |
| `OuiColor` | `Color` (Copy, value type) |
| `OuiLength` | `Length` (enum: `Px(f32)`, `Percent(f32)`, `Auto`) |
| `OuiStatus` | `Result<T, Error>` |
| Callbacks | `Fn` trait objects or closures |

**Example — safe Rust API:**

```rust
use openui::prelude::*;

fn main() -> Result<()> {
    let app = App::builder()
        .name("My App")
        .build()?;

    let window = Window::builder(&app)
        .title("Hello Open UI")
        .size(800, 600)
        .build_ui(build_ui)
        .build()?;

    app.run()
}

fn build_ui(ctx: &mut BuildContext) -> Node {
    let count = ctx.state::<i32>("count", || 0);

    Node::column()
        .gap(16.0)
        .padding(32.0)
        .children([
            Node::text(&format!("Count: {}", count.get()))
                .font_size(24.0)
                .color(Color::WHITE),

            Node::row()
                .gap(8.0)
                .children([
                    button("Decrement", move || count.update(|n| n - 1)),
                    button("Increment", move || count.update(|n| n + 1)),
                ]),
        ])
}

fn button(label: &str, on_click: impl Fn() + 'static) -> Node {
    Node::box_node()
        .padding_h(16.0)
        .padding_v(8.0)
        .background(Color::BLUE)
        .border_radius(4.0)
        .on_click(move |_| on_click())
        .child(
            Node::text(label)
                .color(Color::WHITE)
                .font_size(14.0)
        )
}
```

### 7.3 State Management

**Reactive state model:**

```rust
// State hook (similar to React useState)
let count = ctx.state::<i32>("count", || 0);
count.get()           // → &i32
count.set(42)         // triggers re-render
count.update(|n| n + 1)  // update in-place, triggers re-render

// Derived state (recomputes when dependencies change)
let doubled = ctx.derived("doubled", || count.get() * 2);

// Effect (runs when dependencies change)
ctx.effect("fetch-data", [user_id.get()], || {
    // Fetch data, update state...
});
```

**State is stored in the framework**, keyed by the node's key + state name. This allows the framework to persist state across re-renders (like React hooks).

### 7.4 `openui-macros` — Proc Macros

**`ui!` macro — JSX/SwiftUI-like declarative syntax:**

```rust
fn build_ui(ctx: &mut BuildContext) -> Node {
    let items = ctx.state::<Vec<String>>("items", Vec::new);

    ui! {
        Column(gap: 16, padding: 32) {
            Text("Todo List")
                .font_size(24)
                .font_weight(FontWeight::BOLD)

            for (i, item) in items.get().iter().enumerate() {
                Row(key: format!("item-{i}"), gap: 8) {
                    Text(item)
                        .flex_grow(1.0)

                    Button("Delete")
                        .on_click(move |_| {
                            items.update(|list| { list.remove(i); });
                        })
                }
            }

            Button("Add Item")
                .on_click(|_| {
                    items.update(|list| list.push(format!("Item {}", list.len())));
                })
        }
    }
}
```

**`#[component]` macro:**

```rust
#[component]
fn TodoItem(text: &str, on_delete: impl Fn()) -> Node {
    let hovered = use_state(false);

    ui! {
        Row(gap: 8, padding_v: 4) {
            Text(text)
                .flex_grow(1.0)
                .color(if *hovered.get() { Color::BLUE } else { Color::WHITE })

            Button("×")
                .on_click(move |_| on_delete())
        }
        .on_hover(move |h| hovered.set(h))
    }
}
```

### 7.5 Styling in Rust

```rust
// Inline styles (builder pattern)
Node::box_node()
    .width(Length::Percent(100.0))
    .height(Length::Px(48.0))
    .background(Color::hex("#1a1a2e"))
    .border_radius(8.0)

// Reusable style rules
let card_style = StyleRule::new()
    .background(Color::hex("#16213e"))
    .border_radius(12.0)
    .padding(16.0)
    .shadow(Shadow::new(0.0, 4.0, 8.0, Color::rgba(0, 0, 0, 0.3)));

Node::box_node()
    .style(&card_style)
    .child(/* ... */)

// Theming
let theme = Theme::new()
    .set("--primary", Color::hex("#0f3460"))
    .set("--secondary", Color::hex("#533483"))
    .set("--accent", Color::hex("#e94560"))
    .set("--text", Color::WHITE)
    .set("--spacing", 8.0);

App::builder()
    .theme(theme)
    .build()?;
```

### 7.6 Example Applications

1. **Hello World** — Window with "Hello, Open UI!" text
2. **Counter** — Increment/decrement with state management
3. **Todo App** — Full CRUD with list, input, delete, toggle
4. **Layout Gallery** — Showcasing flex, grid, scroll, text wrapping
5. **Animation Demo** — Transitions, transforms, opacity animations
6. **Custom Paint** — Direct Skia drawing for charts/graphs
7. **Theming** — Light/dark mode switching
8. **Complex App** — Multi-window app with navigation, forms, lists

### 7.7 Documentation

- **rustdoc**: Full API documentation with examples on every type/method
- **Book**: "Getting Started with Open UI" (mdBook)
  - Installation
  - Hello World tutorial
  - Core concepts (nodes, styles, state, events)
  - Layout guide (flex, grid)
  - Styling and theming
  - Custom rendering
  - Performance tips
  - Architecture deep dive

## Deliverables

| Deliverable | Description |
|---|---|
| `openui-sys` crate | Raw FFI bindings |
| `openui` crate | Safe, idiomatic Rust API |
| `openui-macros` crate | `ui!` and `#[component]` proc macros |
| 8 example applications | From hello world to complex app |
| API documentation | rustdoc for all public types |
| Getting Started book | mdBook tutorial |

## Success Criteria

- [ ] `cargo add openui` → build and run a hello world in < 5 minutes
- [ ] All 8 example applications compile and run correctly
- [ ] Zero `unsafe` in application code (all unsafety contained in `openui-sys`)
- [ ] Proc macros produce helpful error messages for invalid syntax
- [ ] rustdoc coverage: every public type and function documented with examples
- [ ] Memory safety: no leaks or UB (validated with Miri where possible)
- [ ] Compile time: hello world builds in < 30 seconds (release mode)
