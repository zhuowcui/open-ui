# Yanked async-executor 1.14.0, 2 November 2026

The `dependency-security` gate failed on PR 74 (run `41551226706`) with
`async-executor: locked registry version is yanked`. `async-executor`
1.14.0, published 15 February 2026, was yanked on crates.io when 1.14.1 was
published on 2 November 2026. `main` passed the gate earlier that day,
before the yank. Open UI reaches the crate only through zbus, AccessKit's
Linux AT-SPI backend.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `cargo update -p async-executor --precise 1.14.1` moves the one locked version and checksum. No other package changes. 1.14.1 requires Rust 1.63, within the 1.85 MSRV. | release tooling | `bindings/rust/Cargo.lock` |

The gate is unchanged. The fuzz lockfile does not contain the crate.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Dependency security | CI `dependency-security` job | checked on this PR |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
