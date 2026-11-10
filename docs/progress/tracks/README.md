# Track progress

Each alpha track (see the [alpha plan](../../alpha/plan.md) and
[workspace instructions](../../../AGENTS.md)) keeps one progress file here,
`<track>.md`: `setup.md`, `engine.md`, `framework.md`, `components.md` and,
later, `docs.md`. A track edits only its own file, so parallel PRs do not
conflict. `docs/progress/current-status.md` holds the history before the
alpha tracks and is no longer appended to.

Add the newest entry at the top. Each entry has a dated heading and states:

- what changed, and the PR that merged it;
- the owning subsystem and exact source;
- each verification command, where it ran (Windows, WSL or CI) and its
  result;
- what remains failing or unmeasured.

Put a long record of one change in its own dated file under
`docs/progress/` and link it from the entry.
