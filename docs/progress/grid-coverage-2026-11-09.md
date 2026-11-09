# CSS Grid coverage baseline, 9 November 2026

The WPT `css-grid` area is now ported into a coverage suite of its own and
measured against Chromium for the first time. 2,218 of 4,204 comparisons are
exact (52.8%) across the four census profiles. The suite is separate from the
expanded census. Its results are not added to the census, and they do not
qualify the renderer.

## Why a coverage suite

The expanded census covers ten WPT CSS directories: floats, position,
flexbox, multicol, overflow, sizing, break, display, box and backgrounds.
Grid layout (`openui-layout/src/grid.rs`) existed but had never been
compared with Chromium. A coverage suite keeps its own module, templates and
manifest, so the census manifests, the mapping CSV and the census templates
are unchanged.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `port_coverage_area.py` ports one WPT CSS area with the porter's deterministic Ahem profile. It writes generated builders in parts of at most 16 MB, the HTML templates, a sorted manifest and a port report. `--check` regenerates in memory and fails on any difference. It requires `CHROMIUM_WPT_ROOT`, from which the porter embeds `<img>` support images. | WPT tooling | `tools/wpt/port_coverage_area.py`, `tools/wpt/test_port_coverage_area.py` |
| The generated `css-grid` suite: 1,053 builders, their templates and manifest, and the port report. | WPT tooling | `bindings/rust/pixel-compare/src/wpt/wpt_css_grid/`, `tools/coverage/css_grid/` |
| The matrix runner has a `coverage` suite. It uses the census profiles, merges the coverage templates (rejecting any ID the census already has) and treats coverage tests as Ahem text tests, as the census treats ported text. | qualification tooling | `tools/qualification/run_renderer_matrix.py` (`contract_profiles`, `load_templates`, `coverage_ids`) |
| The porter clamps `order` to the `int` range, as Blink parses a CSS `<integer>`. `firefox-bug-1958131-crash` uses `order: -17014118346`, which did not compile. No census builder changes. | WPT tooling | `tools/wpt/port_wpt.py` |
| CI runs the porter's unit tests and `port_coverage_area.py --check css-grid css_grid` in both jobs. | CI | `.github/workflows/ci.yml` |

The port reads 1,593 files from the pinned Chromium 147 `css-grid`
directory. It skips `support`, `resources`, `crashtests` and `tentative`
directories, as the census porter does. 1,053 files are portable. 537 use
JavaScript and 3 use an unsupported vendor prefix; the port report lists
each one.

## Measurement

This is a coverage-suite matrix run in WSL at `04d1eeab`, built with the CI
`SKIA_GN_ARGS` as the census is, against Chromium 147.0.7727.50. Two
references are excluded from the run (see below), so 1,051 tests are
measured at four profiles. The rows are in the
[coverage evidence](evidence/grid-coverage-v1.json).

| Profile | Exact | Different | Errors |
| --- | --- | --- | --- |
| `legacy-800x600@1` | 570 | 442 | 39 |
| `mobile-375x667@2` | 565 | 447 | 39 |
| `desktop-1280x720@1.25` | 526 | 486 | 39 |
| `desktop-1920x1080@1.5` | 557 | 455 | 39 |
| Total | 2,218 (52.8%) | 1,830 | 156 |

512 tests are exact at every profile and 480 differ at every profile. Most
differences are large: 1,081 comparisons differ in more than 10,000 pixels,
and only 157 in 10 or fewer. So the failures are layout differences, not
rounding.

By the CSS features a test's source uses (a keyword match, so a rough
guide), the share of tests exact at every profile is lowest for orthogonal
writing modes (17 of 182), baseline alignment (17 of 152), named lines (6 of
45), subgrid (17 of 125), `grid-template-areas` (35 of 151), spanning items
(28 of 129) and self alignment (33 of 138). It is highest for `fr` tracks
(24 of 41), percentage tracks (14 of 24), `fit-content` (9 of 16) and auto
margins (10 of 14).

### Remaining failures

- **Grid area panic, 39 tests (all 156 errors).** `area_axis_range` builds
  `AxisRange { span: last - first }` inside `then_some`, which evaluates it
  even when no area has the name and `first` is still `usize::MAX`. The
  renderer build has overflow checks, so it panics with "attempt to
  subtract with overflow" for any named line that is not an area name, for
  example `placement_grid-layout-lines`. Owner: `openui-layout` grid
  placement, `bindings/rust/openui-layout/src/grid.rs` (`area_axis_range`).
- **Unbounded memory, 2 references, not measured.**
  `grid-definition_grid-support-named-grid-lines-002-ref` and `-003-ref`
  contain no grid: they are rows of left floats with `clear: left`. The
  renderer's memory grows until the kernel kills it (over 30 GB in the
  first run, which stopped the run). Under a 4 GB limit they fail
  allocating 335 MB and 671 MB. Owner: `openui-layout` block and float
  layout; the loop is not yet located.
- **Grid layout differences, 1,830 comparisons.** They concentrate in the
  features listed above. Owner: `openui-layout` grid.
- **Support images in CSS `url()`.** The porter embeds `<img>` support
  images, but a CSS `url(support/...)` stays relative to a capture
  directory that has no `support` folder. Chromium then paints without the
  image, and so does Open UI. The census has the same gap: in
  `background-origin-002` at 1x, Chromium paints red where the green image
  belongs, and the comparison is exact. Owner: WPT tooling,
  `tools/wpt/port_wpt.py` (`generate_html_template`) and the capture
  harness.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Coverage run | `run_renderer_matrix.py --suite coverage --ids-file <manifest without the two references> --jobs 16`, with the CI `SKIA_GN_ARGS` | 2,218 exact, 1,830 different, 156 errors, at `04d1eeab` |
| Port is current | `port_coverage_area.py --check css-grid css_grid`, with and without `PYTHONUTF8=1` | current |
| Windows build | `cargo build --locked -p pixel-compare`, with the `windows-msvc` CI environment | builds; no warnings from the generated module |
| Tool tests | `python -m unittest` over the CI tool test list, plus `tools.wpt.test_port_coverage_area` | 288 passed, 6 skipped |
| Closure ledgers | `generate_sp13r_multicol_closure.py`, `generate_sp17_closure.py`, `generate_sp18_closure.py`, `generate_sp20_closure.py --check`; `restore_frozen_openui_archive.py --check`; `audit.py --repository-only` | pass |
| Fixture audit | `engine_fixture_codegen.py --check` | 20 files, 0 failures |
| Renderer contract | `generate_renderer_contract.py --check` | passes |
| Windows-pending ledger | `check_platform_scoped_tests.py` | 6 recorded, ledger exact |
