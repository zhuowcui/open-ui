# Setup track progress

## Week-0 setup 9 November 2026

The alpha plan is committed at `docs/alpha/plan.md`. `AGENTS.md` and
`CONTRIBUTING.md` now describe the per-track policy (D2): a worktree, branch
and one focused PR at a time per track, each merged to `main` on green CI
after rebasing; track ownership; one-page RFCs under `docs/alpha/rfcs/`; and
progress files in this directory.

### Census lock

`tools/qualification/census_lock.py` runs a command while holding
`<main checkout>/out/census.lock`, an exclusively created file that Windows
(`E:\open-ui\out\census.lock`) and WSL (`/mnt/e/open-ui/out/census.lock`)
both see. It finds the main checkout without git, because WSL git cannot
follow a worktree made by Windows git. A dead holder on the same system is
reclaimed; a holder on the other system needs `release --force`.

| Check | Where | Result |
| --- | --- | --- |
| `python -m unittest tools.qualification.test_census_lock` | Windows and WSL | 8 passed on each |
| WSL holds `run -- sleep 30`; Windows runs `status` and `run` | Windows and WSL | Windows names the WSL holder and `run` exits 1 without running; free after |

### Shared prebuilt Skia (Windows)

`tools/skia_prebuilt.py build` compiles Skia once with the CI
`SKIA_GN_ARGS` and packs it as a rust-skia binary-cache archive under
`<main checkout>/out/skia-prebuilt/<rev>-<inputs hash>/`; `env` points a
worktree's Cargo at it with `FORCE_SKIA_BINARIES_DOWNLOAD` and a `file://`
`SKIA_BINARIES_URL`. See
[the development guide](../../DEVELOPMENT.md#shared-prebuilt-skia-windows).

Findings:

- rust-skia's key holds the rust-skia commit, target and Skia features, not
  `SKIA_GN_ARGS` or the compiler, so the script names the archive directory
  by a hash of those as well.
- rust-skia's own export hook refuses to run on Windows (it calls the
  workspace's `freetype-woff2` feature redundant there), so the script asks
  rust-skia for the key with a forced download from a missing file, builds
  from source, and packs the files rust-skia's export would ship.
- On Linux rust-skia passes Cargo's `OPT_LEVEL` to the Skia compiler but not
  into the key, so the script is Windows-only and WSL keeps building Skia
  from source.
- Two from-source builds of the same inputs in different target directories
  already differ in bytes: every library differs only in COFF object and
  archive-member timestamps, and `skia-bindings.lib` also in the debug-info
  path of its output directory and a CodeView hash derived from it.
  `bindings.rs` and `icudtl.dat` are identical. The archive is a byte copy of
  one such build.
- The Skia compile is about 3 minutes on this 24-core host and about 140 MB
  per Skia feature set. The workspace's own Rust artifacts dominate a target
  directory (35 GB of `debug\deps` in `out\cargo-win-a8`), and they cannot be
  shared between worktrees that build concurrently.

Verification on this branch (based on `a0eae3ed`), Windows, with
`E:\open-ui\out\win-env-a8.ps1`:

| Check | Command | Result |
| --- | --- | --- |
| Build the archive | `python tools\skia_prebuilt.py build` (default `check --locked --workspace --all-targets`, target `out\skp`) | key `a31b86ba3b767344d39a-x86_64-pc-windows-msvc-jpegd-jpege-pdf-textlayout`, 29 MB archive, 7 min 10 s including the workspace check |
| Install in a fresh target | `skia_prebuilt.py env`, then `cargo test --locked --no-fail-fast -p openui-paint` in `out\skc` | build script reports `DOWNLOAD AND INSTALL SUCCEEDED`; no Skia compile |
| Focused tests, source vs prebuilt | `cargo test --locked --no-fail-fast -p openui-paint` in `out\skp` (`FORCE_SKIA_BUILD`) and `out\skc` (archive) | 1,284 passed, 0 failed, 1 ignored in both |
| Installed files | SHA-256 of the installed `*.lib`, `icudtl.dat`, `bindings.rs` | `out\skc` equals the archive's source build exactly |
| Every registered test rendered, source vs prebuilt | `pixel_compare render <id> <png>` for all 7,209 IDs from `pixel_compare list`, with each build's `pixel-compare` | 7,168 PNGs byte-identical, 0 different; the same 41 `css-grid` tests fail to render in both (layout panics and unbounded growth already recorded in the [grid coverage record](../grid-coverage-2026-11-09.md)) |

`pixel_compare render-all` itself cannot write these IDs on Windows: it
does not create the subdirectories that IDs containing `/` need. The
comparison above renders each ID with `render` and a flattened file name
instead; that `render-all` defect is left to the engine track.
