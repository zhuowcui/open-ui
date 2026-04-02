# Sub-Project 8: Rust Bindings & Developer Experience

> Idiomatic Rust crate wrapping Open UI's C API with safe abstractions, derive macros, and excellent DX.

## Objective

Create an `openui` Rust crate that wraps the C API with safe, ergonomic Rust abstractions. Developers should be able to `cargo add openui` and build pixel-perfect UIs with a declarative API.

## Tasks

### Phase A: FFI Foundation
1. `openui-sys` crate — raw C FFI bindings generated from `openui.h`
2. Safety wrappers — ownership semantics, Drop implementations, Result types
3. Build script — links against `libopenui_rendering.so`, handles platform detection

### Phase B: Idiomatic API
4. Element builder pattern — `Element::div().width(px(200)).flex_direction(Row).child(...)`
5. Style DSL — type-safe style properties with compile-time validation
6. Layout queries — `.rect()`, `.width()`, `.height()` returning typed values
7. Event handling — closures for mouse/keyboard/focus events

### Phase C: Declarative UI
8. Derive macros for component definition
9. Reactive state management (signals/subscriptions)
10. Declarative element tree construction (similar to RSX/JSX)
11. Diffing engine for efficient tree updates

### Phase D: Developer Experience
12. Hot reload support (rebuild + re-render without restart)
13. Inspector/debugger tool (element tree visualization)
14. Comprehensive documentation with examples
15. Template project (`cargo generate openui-template`)

## Deliverables

| Deliverable | Description |
|---|---|
| `openui-sys` crate | Raw FFI bindings |
| `openui` crate | Safe, idiomatic Rust API |
| Documentation | docs.rs-hosted API reference + guide |
| Examples | 5+ example applications |
| Template project | `cargo generate` starter |

## Success Criteria

- [ ] `cargo add openui` → build a hello world window
- [ ] Safe Rust API with no unsafe in user code
- [ ] Declarative UI construction with reactive state
- [ ] All HTML element types accessible through Rust API
- [ ] Widget gallery example renders all elements
