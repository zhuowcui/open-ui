# ADR 003: Extract `base/` First, Shim Later

## Status

Accepted

## Context

Every Chromium component depends heavily on `base/` — Chromium's foundational utility library providing threading, memory management, containers, callbacks, logging, time, and more. When extracting `cc/`, layout, or style, we inevitably pull in `base/` dependencies.

We have two strategies:

1. **Reimplement from scratch**: Replace `base/` types with standard C++20, Abseil, or our own implementations. Reduces coupling but risks subtle behavioral differences.

2. **Extract the real `base/`**: Pull in the actual Chromium `base/` code that our components use. Faithful to Chromium's behavior but increases the amount of code we maintain.

## Decision

**Extract first, shim later.**

Phase 1: Extract the actual `base/` source files that our target components depend on. This ensures behavioral correctness — the extracted components work exactly as they do in Chromium.

Phase 2 (later): Once the extraction is stable and tested, identify `base/` modules that can be replaced with lighter alternatives:
- `base::flat_map` → `std::flat_map` (C++23) or `absl::flat_hash_map`
- `base::span` → `std::span` (C++20)
- `base::optional` → `std::optional` (already C++17)
- `LOG()` / `CHECK()` → lightweight logging shim
- `base::TimeTicks` → `std::chrono::steady_clock`

Phase 3 (much later): Evaluate reimplementing the harder pieces:
- `base::TaskRunner` / `base::SequencedTaskRunner` — threading infrastructure
- `base::OnceCallback` / `base::RepeatingCallback` — callback system
- `base::RefCounted` / `base::WeakPtr` — reference counting

## Consequences

**Benefits:**
- Behavioral correctness from day one — no subtle differences
- Faster initial extraction — don't need to debug reimplementation bugs
- Clear path to optimization — we know exactly what we're replacing
- Components "just work" because they're running on the real `base/`

**Drawbacks:**
- Larger initial extraction footprint
- We temporarily carry more Chromium code than we'd like
- `base/` may pull in its own transitive dependencies

**Mitigation:**
- The `base/` minimal subset spec (Phase C of SP1) identifies exactly which pieces we need
- We only extract the subset, not all of `base/`
- Target: < 20% of full `base/` in our minimal subset
- Clear roadmap for replacing with lighter alternatives once stable
