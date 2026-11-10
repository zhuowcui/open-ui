# Cross-track RFCs

A change to an API or behavior that another track depends on is proposed
here first, as one page named `NNNN-short-title.md` (next free number). See
[track ownership](../../../AGENTS.md#track-ownership).

Each RFC has four sections:

1. **Problem**: what the owning track needs and why the current API does not
   serve it.
2. **Proposal**: the new or changed API or behavior, with a short example.
3. **Affected tracks**: who consumes it and what each must change.
4. **Migration**: the order of PRs, and how `main` stays green between them.

Merge the RFC first, or in the PR that makes the change once the affected
tracks have agreed.
