# Dependency review

Public runtime dependencies added for Open UI v0.2 are pinned in
`bindings/rust/Cargo.lock` and must be stable releases compatible with Rust
1.85 or newer. `generate_v02_contract.py` rejects prerelease packages and
freezes the lockfile digest.

| Package | Version | License | Purpose and maintenance review |
|---|---:|---|---|
| accesskit | 0.25.0 | MIT OR Apache-2.0 | Canonical cross-platform semantic node, tree-update, and action vocabulary. Stable release from the maintained AccessKit project; its Rust 1.85 MSRV matches Open UI. |

Transitive package `uuid` 1.26.1 is used by AccessKit tree identifiers and is
covered by the same lockfile, license, MSRV, and advisory checks. Linux shell
dependencies are reviewed when the feature-gated platform crate lands.
