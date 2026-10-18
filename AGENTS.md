# Open UI workspace instructions

## Authoritative implementation

`main` is the source of truth for this experimental repository. Work in this
checkout, starting each focused task from the latest `main`. Use one short-lived
branch and one PR targeting `main`; merge it before starting the next task.
Do not create PR stacks or leave integration work for the user.

Do not create implementation worktrees or sibling clones by default. Keep
temporary builds and comparison materializations under ignored `out/` paths.
Existing historical checkouts are reference material, not alternative product
branches that the user must merge. The agent owns any necessary integration
and verifies it in the authoritative checkout before calling the work complete.
Preserve existing checkouts and borrowed Git object stores; this policy does
not authorize deleting them or pruning their objects.

## Read before continuing

- Read `docs/progress/branch-checkpoint-2026-10-17.md` for the paused checkpoint,
  experiment reconciliation, verification and the next bounded task.
- Read `docs/progress/current-status.md`, `docs/renderer/contract.md`,
  `docs/engineering-principles.md` and relevant entries in `docs/memory/`.
- Follow `CONTRIBUTING.md` and the supported Rust workspace in `bindings/rust`.
  The old Blink backend and sprint plans are historical.

## Progress and review

Document each accepted behavior change and each remaining failure with its
owning subsystem, exact source, verification command and result. Distinguish
tests of the working branch from historical or private-source measurements.
Do not add selected results to a full census or claim unmeasured qualification.
Keep generated evidence reproducible and never weaken an exactness gate.

Keep subsequent PRs focused on one repair or coherent feature. Stop feature
work at a requested review checkpoint; do not grow a paused PR with unrelated
work. On 17 October 2026 the user explicitly directed the latest implementation
and its pending history to merge into `main`, superseding the draft PR stack
and its merge hold. Keep the known native clipping failure documented; this
experimental integration does not establish renderer or release qualification.
