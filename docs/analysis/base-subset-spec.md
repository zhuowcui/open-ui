# base/ Minimal Subset Specification

> Derived from automated analysis of `cc/`, `blink/layout/`, `blink/css/`, and `blink/style/` against Chromium M147.

## Key Finding: Skia Has Zero base/ Dependencies

Skia is fully self-contained and uses none of Chromium's `base/` library.
This makes it the ideal first extraction target.

## base/ Usage by Component

| Subsystem | cc/ (unique includes) | layout/ | css/ | Extraction Strategy |
|---|---|---|---|---|
| **threading** | 20 | 1 | 1 | Must extract — core to compositor |
| **memory** | 17 | 4 | 4 | Must extract — pervasive ownership model |
| **callbacks** | 5 | 2 | 3 | Must extract — used everywhere |
| **synchronization** | 4 | 0 | 1 | Must extract — thread safety |
| **containers** | 10 | 2 | 5 | Replace with std/absl (flat_map → absl::flat_hash_map) |
| **strings** | 9 | 0 | 3 | Replace with std::string_view |
| **time** | 7 | 2 | 2 | Replace with std::chrono |
| **numerics** | 7 | 2 | 2 | Replace with std equivalents |
| **logging** | 8 | 5 | 5 | Stub with simple fprintf/assert |
| **tracing** | 13 | 2 | 3 | Stub with no-op macros |
| **metrics** | 6 | 1 | 2 | Stub with no-op |
| **observer** | 2 | 0 | 0 | Reimplement (simple pattern) |
| **files** | 2 | 0 | 0 | Likely not needed |
| **other** | 59 | 11 | 21 | Evaluate individually |

## Must-Extract Subset (Tier 1)

These are fundamental to cc/ and cannot be stubbed:

### Threading (20 unique includes in cc/)
```
base/threading/thread.h
base/threading/thread_checker.h
base/task/single_thread_task_runner.h
base/task/task_runner.h
base/task/sequenced_task_runner.h
base/task/thread_pool.h
base/sequence_checker.h
base/run_loop.h
base/message_loop/message_pump_type.h
base/threading/platform_thread.h
```

### Memory (17 unique includes in cc/)
```
base/memory/scoped_refptr.h
base/memory/ref_counted.h
base/memory/weak_ptr.h
base/memory/raw_ptr.h
base/memory/ptr_util.h
base/memory/discardable_memory.h
base/memory/shared_memory_mapping.h
```

### Callbacks (5 unique includes in cc/)
```
base/functional/callback.h
base/functional/callback_helpers.h
base/functional/bind.h
base/functional/function_ref.h
```

### Synchronization (4 unique includes in cc/)
```
base/synchronization/lock.h
base/synchronization/waitable_event.h
base/synchronization/condition_variable.h
base/atomic_ref_count.h
```

## Can-Stub Subset (Tier 2)

These can be replaced with trivial implementations:

### Logging → fprintf/assert
```
base/check.h          → OUI_CHECK() macro
base/check_op.h       → OUI_CHECK_EQ/NE/LT/GT macros
base/logging.h        → OUI_LOG() with severity
base/notreached.h     → OUI_NOTREACHED() = __builtin_unreachable()
```

### Tracing → no-op macros
```
base/trace_event/trace_event.h        → empty macro
base/trace_event/traced_value.h       → no-op class
base/trace_event/memory_allocator_dump.h → no-op
```

### Metrics → no-op
```
base/metrics/histogram_macros.h → empty macros
base/metrics/histogram_functions.h → empty functions
```

## Can-Replace Subset (Tier 3)

These have direct standard library or absl equivalents:

| base/ Header | Replacement |
|---|---|
| `base/containers/flat_map.h` | `absl::flat_hash_map` or `std::map` |
| `base/containers/flat_set.h` | `absl::flat_hash_set` or `std::set` |
| `base/containers/span.h` | `std::span` (C++20) |
| `base/strings/stringprintf.h` | `std::format` (C++20) or `fmt` |
| `base/strings/string_number_conversions.h` | `std::to_string` / `std::from_chars` |
| `base/time/time.h` | `std::chrono::steady_clock` |
| `base/time/time_delta.h` | `std::chrono::duration` |
| `base/numerics/safe_conversions.h` | `std::in_range` (C++20) + static_cast |
| `base/numerics/clamped_math.h` | Manual clamping or `std::clamp` |

## Estimated Extraction Size

| Category | Files | Strategy |
|---|---|---|
| Must-extract (Tier 1) | ~40-50 headers + impls | Extract from Chromium base/ |
| Can-stub (Tier 2) | ~15-20 headers | Write thin wrappers (~500 LoC) |
| Can-replace (Tier 3) | ~15-20 headers | Typedef/alias to std/absl |
| Not needed | ~90% of base/ | Skip entirely |

**Estimated total: < 10% of base/ needed** (was initially estimated at < 20%)

## Cross-Layer Dependency Summary

| From → To | Cross-includes | Key Headers |
|---|---|---|
| cc/ → blink/layout | **0** | Clean boundary ✅ |
| cc/ → blink/css | **0** | Clean boundary ✅ |
| layout/ → css/ | 29 headers | `style_resolver.h` (20 refs), `style_engine.h` (19 refs) |
| layout/ → style/ | 34 headers | `computed_style.h` (62 refs!), constants (17 refs) |
| css/ → style/ | ~30 headers | `computed_style.h` dominates |

### Extraction Order Implication

1. **Skia** — zero base/ deps, zero cross-layer deps → extract first
2. **cc/** — zero blink deps, heavy base/ threading deps → extract second, bring threading subset
3. **style/** — foundation that layout and css depend on → extract third
4. **layout/ + css/** — tightly coupled, extract together → extract last

This matches our planned sub-project order (SP2→SP3→SP5→SP4).
