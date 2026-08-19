---
id: 0035
title: SP17 W1A logical geometry preserves the full exact baseline
tags: sp17, writing-mode, constraint-space, geometry, baseline
status: active
created: 2026-08-19
updated: 2026-08-19
refs: docs/SP17-PLAN.md, bindings/rust/openui-layout/src/constraint_space.rs, bindings/rust/openui-layout/src/logical_geometry.rs, bindings/rust/openui-geometry/src/box_strut.rs
---

W1A adds `WritingDirectionMode` to every `ConstraintSpace`. Legacy constructors
remain horizontal-LTR; explicit root and child APIs convert physical viewport
or parent-logical extents once when axes are orthogonal. Production rendering
derives the root direction from computed style. `LogicalBoxStrut` centralizes
physical/logical edge round trips, and `ResolvedLogicalBox` projects computed
sizes, margins, padding, insets, and used borders into layout coordinates while
keeping final fragments physical for paint.

After a release rebuild, the immutable 3,267-ID exact manifest ran without
resume: 3,267 pass at `0.0%`, zero failures, zero errors. The authoritative full
summary was restored byte-for-byte at SHA-256
`67d50eb1ee54fcf7121df5c1f186d2468a24b2f0310303606e0a689e4d2581a4`.
No WPT builder, mapping row, runner profile, or result artifact changed.

This is not vertical-layout closure. The next slice must migrate normal block
and flex child construction to explicit parent/child directions, consume
`ResolvedLogicalBox` for layout decisions, and convert completed logical
offsets/sizes to physical fragment geometry once. Re-run the frozen exact
manifest after each horizontal-no-op boundary before surgically admitting any
of the 292 new builders.
