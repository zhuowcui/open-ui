# Fieldset content scrollbar style, 26 October 2026

`css_flexbox/fieldset-as-item-overflow` failed at every profile (1,500 pixels
at 1×).

## Finding

The test gives a fieldset `overflow-y: scroll`. The capture harness hides
WebKit scrollbar pseudo-elements, so Chrome draws no scrollbar and the green
content fills the full 100px. Open UI drew a 15px classic scrollbar.

Open UI models the harness rule as `scrollbar-width: none` on every element
(`pixel-compare` `FixtureEngine`). A fieldset's scroll container, though, is
its anonymous content box, whose style Open UI derives from the fieldset
(`fieldset.rs` `content_style`). That derivation copied overflow but not the
scrollbar style, so the anonymous box kept the initial `auto` width.

In Blink, an anonymous box without a node takes its scrollbar style from its
parent (`ScrollbarStyleSource` in
`third_party/blink/renderer/core/paint/paint_layer_scrollable_area.cc` at tag
147.0.7727.50). The fieldset's custom scrollbar style therefore governs its
content box's scrollbars.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| The anonymous fieldset content box copies the fieldset's `scrollbar-width`, `scrollbar-color` thumb and track | `openui-layout` | `fieldset.rs` `content_style` |

## Measurement

Paired expanded census in WSL against the
[vertical table item](flex-vertical-table-item-size-2026-10-26.md) run (clone
commits `33204b70` and `8a6b1490`), sharing one Chromium cache:

| Result | Comparisons |
| --- | --- |
| Exact | 22,065 to 22,069 |
| Gained | 4 |
| Lost, improved, worsened | none |

`fieldset-as-item-overflow` becomes exact at all four profiles. No other
comparison changes. Chromium is identical in every comparison. Rows are in the
[evidence](evidence/fieldset-content-scrollbar-source-census-v1.json).

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Paired expanded census | `build_renderer.py`; `run_renderer_matrix.py --suite expanded --jobs 16` | 22,065 to 22,069 exact; 4 gains, 0 losses |
| Layout tests | `cargo test --locked -p openui-layout` | 5,928 passed, 0 failed |
| Renderer contract | `generate_renderer_contract.py --check` | passes with the regenerated author style inventory |
| Windows workspace | `cargo test --workspace --locked --no-fail-fast`, with the `windows-msvc` CI environment | 9,459 passed, 0 failed, 20 ignored |
