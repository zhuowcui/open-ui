# SP13-R — Runnable Multicol Exact Closure

SP13-R closes every runnable row that carried `sp13_multicol` ownership while
preserving the unported multicol boundary. Exact pixel equality is the only pass
threshold.

## Frozen Scope

The closure is represented by three immutable, sorted ledgers:

| Ledger | Rows | Meaning |
|---|---:|---|
| `sp13r_baseline_exact.json` | 2823 | Exact passes frozen before SP13-R |
| `sp13r_multicol_targets.json` | 351 | Runnable multicol closure targets |
| `sp13r_multicol_residuals.json` | 1018 | Unported, reason-owned multicol rows |

The target and residual ledgers are disjoint and form the complete 1369-row
multicol inventory. Every residual records its Chromium path, porter rejection,
and complete owner set. Historical SP14–SP16 ledgers are unchanged.

## Implemented Behavior

- Transactional parsing and cascade handling for multicol shorthands and longhands,
  including invalid declarations, resets, inheritance, `normal`, and fractional
  used geometry.
- Shared resolved column widths, gaps, overflow-column placement, rule centers,
  and horizontal LTR/RTL ordering.
- One fragmentation decision path for definite and indefinite fragmentainers,
  balancing and auto fill, forced/avoid/last-resort breaks, widows/orphans,
  margins, floats, overflow, and break-token continuation state.
- Spanners, pre/post-spanner rows, nested multicols, block and inline containers,
  fragmented flex containers, positioned descendants, containing blocks, and
  out-of-flow continuations.
- Fragment-owned metadata for column rules and fragmented decorations, including
  corpus-used rule styles, backgrounds, gradients, shadows, borders,
  `box-decoration-break`, clipping, and local raster backgrounds.

Vertical and sideways writing modes remain outside this closure. Their rows retain
explicit non-SP13-R owners as applicable.

## Verified Result

The authoritative exact-ID run completed with 351 passes, zero failures, and zero
errors. The final no-resume WPT run completed with:

| Metric | Value |
|---|---:|
| Runnable WPT tests | 3566 |
| Exact passes | 3267 |
| Functional failures | 299 |
| Render/diff errors | 0 |
| Unported rows | 4107 |
| Unported `sp13_multicol` residuals | 1018 |

All 2823 baseline IDs and all 351 targets remain exact. No runnable row retains
`sp13_multicol`. The generated mapping, deferred report, documentation, and status
are deterministic, and `audit.py` passes all 7 checks.

## Authoritative Commands

```bash
cd bindings/rust
cargo build --release --package pixel-compare
cd ../..

LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py \
  --ids-file tools/accountability/data/wpt_ported/sp13r_multicol_targets.json

LD_LIBRARY_PATH="$PWD/chrome/linux-147.0.7727.50/chrome-linux64" \
  python3 -u tools/accountability/run_all_pixel_comparisons.py 'wpt/'

python3 tools/accountability/generate_wpt_mapping.py
python3 tools/accountability/generate_sp12_5_csv.py
python3 tools/wpt/generate_sp13r_multicol_closure.py --check
python3 tools/accountability/audit.py
```
