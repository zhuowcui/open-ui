# Open UI workspace instructions

## Authoritative implementation

`main` is the source of truth for this experimental repository. Work is split
into tracks (decision D2 in the [alpha plan](docs/alpha/plan.md)): setup,
engine, framework and components now, docs later. Each track works in its own
git worktree on its own branch, with one focused PR open at a time. Before
merging, rebase the PR on the latest `main` and rerun its checks; merge it to
`main` independently once CI is green, then start the next PR from the new
`main`. Do not create PR stacks, merge another track's PR, force-push `main`
or leave integration work for the user.

Exception: if every commit `main` gained since the PR's last green run only
touches paths the PR cannot interact with (`brand/`, `site/`, or Markdown
documentation outside generated and checked files), the PR may merge on its
existing green run without a rebase, provided GitHub reports no conflict.
Rebase and rerun when `main` gained any change to Rust code, `Cargo.toml` or
`Cargo.lock`, generators or generated files, tools, fixtures, or
`.github/workflows`, and whenever in doubt. Never enable GitHub auto-merge:
`main` has no required checks, so it merges before CI finishes.

Keep temporary builds and comparison materializations under ignored `out/`
paths, and give each worktree its own `CARGO_TARGET_DIR` there (for example
`out\cargo-<track>`), because concurrent builds in one target directory block
and invalidate each other. Existing historical checkouts are reference
material, not alternative product branches that the user must merge. The
agent owns any necessary integration and verifies it in its track's checkout,
rebased on `main`, before calling the work complete. Preserve existing
checkouts and borrowed Git object stores; this policy does not authorize
deleting them or pruning their objects.

## Track ownership

A track owns the paths below. Another track does not edit them without an
RFC (see below).

| Track | Owns |
| --- | --- |
| setup | `AGENTS.md`, `docs/alpha/plan.md`, `docs/progress/tracks/README.md`, build-environment tooling (`tools/skia_prebuilt.py`, `tools/qualification/census_lock.py`) |
| engine | `openui-style`, `openui-layout`, `openui-paint`, `openui-text`, `openui-engine` and the qualification tooling (`tools/qualification`, `tools/coverage`, `tools/wpt`, `pixel-compare`) |
| framework | `openui` (the public crate), `openui-platform`, and the new CLI and devtools crates |
| components | the new `components/` area, which uses the public `openui` crate only |
| docs (later) | `docs/guide/` (new) and the API reference |

Paths no track owns (for example the workspace `Cargo.toml` and
`Cargo.lock`, `.github/workflows`, the other crates and the examples) may be
changed by any track in a focused PR; keep such edits minimal and rebase
before merging.

A change to an API or behavior that another track depends on goes through a
one-page RFC under `docs/alpha/rfcs/` (problem, proposal, affected tracks,
migration). Merge the RFC first, or with the change once the affected tracks
have agreed.

## Shared machine resources

- Only one full census runs at a time on this machine (about 75 minutes, 16
  jobs in WSL). Run it under the lock, from Windows or WSL:
  `python3 tools/qualification/census_lock.py run --wait -- <census command>`.
  `census_lock.py status` names the holder. Use `release --force` only after
  checking that the holder's census has ended. Focused tests need no lock.
- On Windows, build Skia once per set of build inputs with
  `tools/skia_prebuilt.py` and install it in each worktree, as described in
  [the development guide](docs/DEVELOPMENT.md#shared-prebuilt-skia-windows).

## Read before continuing

- Read `docs/alpha/plan.md` and your track's file in `docs/progress/tracks/`.
- Read `docs/progress/branch-checkpoint-2026-10-17.md` for the paused checkpoint,
  experiment reconciliation, verification and the next bounded task.
- Read `docs/progress/current-status.md`, `docs/renderer/contract.md`,
  `docs/engineering-principles.md` and relevant entries in `docs/memory/`.
- Follow `CONTRIBUTING.md` and the supported Rust workspace in `bindings/rust`.
  The old Blink backend and sprint plans are historical.

## Progress and review

Each track records its progress in its own file,
`docs/progress/tracks/<track>.md`, instead of appending to
`docs/progress/current-status.md`, so parallel PRs do not conflict. Detailed
records of a single change stay in their own dated files under
`docs/progress/`.

Document each accepted behavior change and each remaining failure with its
owning subsystem, exact source, verification command and result. Distinguish
tests of the working branch from historical or private-source measurements.
Do not add selected results to a full census or claim unmeasured qualification.
Keep generated evidence reproducible and never weaken an exactness gate.
Engine changes must show no census or coverage regression (decision D4).

Keep subsequent PRs focused on one repair or coherent feature. Stop feature
work at a requested review checkpoint; do not grow a paused PR with unrelated
work. On 17 October 2026 the user explicitly directed the latest implementation
and its pending history to merge into `main`, superseding the draft PR stack
and its merge hold. On 9 November 2026 the user approved the per-track policy
above, superseding the single-branch, no-worktree rule. Keep the known native
clipping failure documented; this experimental integration does not establish
renderer or release qualification.
