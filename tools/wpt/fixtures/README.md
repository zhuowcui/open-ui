# WPT Porter Fixtures

These are the smallest upstream snapshots needed by committed porter
idempotence tests. Tests patch `splice_text_port.WPT_ROOT` to this directory so
hosted CI and clean developer checkouts never depend on a sibling Chromium
source tree.

The current snapshots came from Chromium source commit
`09d377d9438dc95267369f74a073acd81bdde38f`:

- `external/wpt/css/css-multicol/multicol-count-002.xht`
- `external/wpt/css/CSS2/floats/float-nowrap-1.html`

The XHTML fixture has a normalized final newline. Fixture updates must remain
minimal and must still prove that `prepare_changes()` reproduces the committed
Rust builder and template byte for byte.
