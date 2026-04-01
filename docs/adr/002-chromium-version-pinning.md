# ADR 002: Chromium Version Pinning Strategy

## Status

Accepted

## Context

Chromium releases a new stable version approximately every 4 weeks. The rendering pipeline evolves with each release — new layout features, compositor optimizations, Skia updates, and style system changes.

We need a strategy for which Chromium version to base our extraction on and how to stay current.

## Decision

1. **Pin to Chromium M147** (tag `147.0.7727.24`) as our initial extraction base. This is the latest stable release at project inception (March 2026).

2. **Quarterly review cadence**: Every 3 months, evaluate upstream changes in the rendering pipeline:
   - Review changelogs for `cc/`, `blink/renderer/core/layout/`, `blink/renderer/core/css/`, `third_party/skia/`
   - Assess whether any changes are worth backporting (performance improvements, bug fixes, new layout features)
   - If updating, bump the submodule to the latest stable release at that time

3. **Version tracking**: The `CHROMIUM_VERSION` file in the repo root always contains the pinned version tag.

4. **No continuous tracking**: We do NOT attempt to follow Chromium head or even every stable release. Stability of our extraction is more important than being on the latest version.

## Consequences

**Benefits:**
- Stable extraction target — no surprise breakages from upstream churn
- Clear versioning for users ("Open UI is based on Chromium M147")
- Focused development without constantly adapting to upstream changes
- Quarterly reviews catch important improvements without being overwhelming

**Drawbacks:**
- We may miss upstream performance improvements or bug fixes between reviews
- Security patches in the rendering pipeline may be delayed
- As we diverge from upstream, applying updates becomes harder over time

**Mitigation:**
- Security-critical fixes are evaluated immediately, not on the quarterly cadence
- We maintain notes on our divergences from upstream to ease future updates
- Automated tooling (`tools/analyze_upstream_delta.py`) compares our pin vs. latest stable
