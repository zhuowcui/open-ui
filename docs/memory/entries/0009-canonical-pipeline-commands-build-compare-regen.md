---
id: 0009
title: Canonical pipeline commands (build/compare/regen/audit)
tags: commands, pipeline, accountability
status: active
created: 2026-07-08
updated: 2026-07-08
---

Build: cd bindings/rust && cargo build --release --package pixel-compare
Compare (full): LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'
  (replace 'wpt/' with a filter like 'wpt/css_break/...' for focused runs — but see summary.json overwrite gotcha)
Regen tracking: python3 tools/accountability/generate_wpt_mapping.py && python3 tools/accountability/generate_sp12_5_csv.py
Audit (must be 7/7): python3 tools/accountability/audit.py
Debug fragment tree: cd bindings/rust && ./target/release/pixel_compare debug wpt/<suite>/<test-id>
Regen WPT templates (layout): python3 tools/wpt/batch_port_sp12.py
