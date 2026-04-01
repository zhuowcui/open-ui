# Contributing to Open UI

## Getting Started

1. Fork the repository
2. Clone with submodules: `git clone --recursive <your-fork>`
3. Install prerequisites (see [README.md](README.md))
4. Create a branch: `git checkout -b my-feature`
5. Make changes, commit, push, open a PR

## Code Style

### Extracted Chromium Code (`third_party/chromium/`, `src/base/`)

Follow [Chromium's C++ style guide](https://chromium.googlesource.com/chromium/src/+/main/styleguide/c++/c++.md):
- 2-space indentation
- `UpperCamelCase` for types, `lower_snake_case` for variables/functions
- `kConstantName` for constants
- Chromium `base/` types preferred over STL where extracted

### Our Code (`src/`, `include/`)

Follow the [Google C++ Style Guide](https://google.github.io/styleguide/cppguide.html) with these adjustments:
- C++20 standard
- 2-space indentation
- 100-character line limit
- `#pragma once` for include guards

### C API Headers (`include/openui/`)

Conventions for the public C API:

```c
// All symbols prefixed with oui_ (lowercase)
OuiStatus oui_compositor_create(OuiCompositor** comp);

// Types prefixed with Oui (UpperCamelCase)
typedef struct OuiCompositor OuiCompositor;

// Enums prefixed with OUI_ (UPPER_SNAKE_CASE)
typedef enum {
    OUI_STATUS_OK = 0,
    OUI_STATUS_ERROR = 1,
    OUI_STATUS_OUT_OF_MEMORY = 2,
} OuiStatus;

// Handle-based: all objects are opaque pointers
// Lifecycle: explicit create/destroy pairs
// Error handling: return OuiStatus
// No C++ in public headers (extern "C" wrappers internally)
// Thread safety documented per function
```

## Formatting

All code is formatted with `clang-format` using the config in `.clang-format`:

```bash
# Format all source files
find src include -name '*.cc' -o -name '*.h' | xargs clang-format -i

# Check formatting (CI does this)
find src include -name '*.cc' -o -name '*.h' | xargs clang-format --dry-run --Werror
```

GN files are formatted with `gn format`:

```bash
find . -name 'BUILD.gn' -o -name '*.gni' | xargs -I{} gn format {}
```

## Commit Messages

```
component: short description

Longer explanation if needed. Wrap at 72 characters.

- Bullet points are fine
- Reference issues with #123
```

Components: `skia`, `compositor`, `layout`, `style`, `scene_graph`, `platform`, `build`, `ci`, `docs`, `tools`

## Pull Requests

- One logical change per PR
- Include tests for behavioral changes
- Update docs if changing public APIs
- CI must pass before merge
- Squash-merge to keep history clean

## Architecture Decision Records (ADRs)

Significant decisions are recorded in `docs/adr/`. To propose a new decision:

1. Copy `docs/adr/TEMPLATE.md` → `docs/adr/NNN-title.md`
2. Fill in context, decision, consequences
3. Submit as a PR for review

## Reporting Issues

- Use GitHub Issues
- Include: OS version, compiler version, build type (Debug/Release)
- For rendering issues: include screenshot + minimal reproduction
