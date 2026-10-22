# Open UI experimental integration checkpoint 17 October 2026

Product work is paused at this checkpoint. The user explicitly directed the
latest `codex/native-final-linux-v4314` implementation and its pending history
to merge into `main`, which is the experimental source of truth. The agent
owns integration; the user has no worktree merge backlog. Future work starts
from the latest `main`, with one focused PR at a time and no PR stack.
The known native scrollbar clipping failure remains documented below and
assigned to the next repair. The user accepts it for this experimental merge;
the native test gate and renderer/release qualification remain open.

## Review scope

[PR 4](https://github.com/zhuowcui/open-ui/pull/4) consolidates the latest
branch directly into `main`, preserving the pending history with a merge
commit. It includes [PR 3](https://github.com/zhuowcui/open-ui/pull/3), whose
head `8bcc6dfa` is already an ancestor. This replaces the initial stacked draft
arrangement. Before this workflow correction, the latest branch at `f2346abf`
was 689 commits ahead of `main` at `1d7a4cc5`; this final consolidation includes
that history and the correction below.

The 37 implementation and evidence commits following PR 3 improve nested
column source ownership, clipping and paint projection; native control geometry
and resize painting; animation timeline
reassignment and its C consumers; Grid containing-block ownership; and retained
font metadata. They also preserve Chromium observations and known failures.
The latest pre-checkpoint commit is `d18d72f7`; the accepted runtime source is
unchanged from its documentation parent `ec5ab327`. The initial checkpoint
at `f2346abf` added workspace instructions, portable evidence and stacked PR
CI coverage. The final correction replaces that workflow with one PR targeting
`main`, updates the progress records and supersedes the earlier memory entry.

`AGENTS.md` and `CONTRIBUTING.md` require future product changes, tests and
progress records in the authoritative checkout. A later PR should address one
repair or coherent feature and merge before another starts. The user's later
instruction to merge the latest experimental branch supersedes the initial
draft-only checkpoint request; it does not claim that failing checks pass.

## Historical experiments are accounted for

The committed tips of 89 existing registered worktrees and 19 immediate
`openui-*` sibling clones were compared with `d18d72f7`, covering 108 checkouts
and 104 distinct tips. For 89 checkouts, their commits are already ancestors or
have equivalent patches in the branch history. The other 19 are represented
through ten documented amended integrations and thirteen older commits
incorporated into combined or later corrections.

| Earlier experiment | Accepted implementation and evidence |
| --- | --- |
| Native control intrinsics, textarea resize defaults, menu-list lowering and their guards | `93831551` explicitly integrates corrected private controls `d3a34cb5`; later `e1cba1d1` updates the shared geometry and lowering |
| Input notification draft, callback pointer safety and Rust/C/C++ consumers | `c09eb639`; its FFI file exactly matches corrected private `cd6f9d4b`, and both C consumers match `7de4820c`; later evidence is recorded by `454c1667` |
| C focus consumer formatting | `20849fe7` retains the later hosted clang-format 18.1.3 correction |
| Amended animation, transforms, dimensions, scroll ownership and radius changes | Original source hashes and accepted commits are mapped individually in the integration inventory |

No committed source in this inventory remains unclassified. This reconciles
committed product work; it does not admit unrelated scratch files or later
private candidates as accepted implementation. Existing checkouts and borrowed
object stores are preserved. Deleting them is a separate operation.

The [integration inventory](evidence/branch-checkpoint-2026-10-17.json) records
every checkout, source hash, integration resolution and archive checksum.
The comparison can be repeated with `git cherry d18d72f7 <checkout-tip>`,
followed by the referenced integration commits for amended or combined patches.

## Portable experimental evidence

The [checkpoint archive](evidence/branch-checkpoint-2026-10-17.tar.gz) preserves:

- All 35 receipts pinned by the V12 closure record, with their original byte
  counts and SHA-256 values verified before copying.
- Thirty historical progress files from the uncommitted sibling V10 work,
  including its status text, renderer summaries and native/Chromium observations.
  They remain historical records rather than replacing the current V12 status.
- Implementation deltas for the four unadopted sources below. Each patch is
  independently relative to `d18d72f7`, covering the Rust workspace, tools,
  examples, headers and tests. Restoration into ignored output is checked
  against every affected target Git blob.

| Private source | Disposition |
| --- | --- |
| `ec3883b3` | Completed census source; the global renderer regression gate remains failed |
| `3de52662` | Selected vertical repairs; no complete qualification |
| `21b96fe7` | Combined candidate; selected native gate records 32 passes and three failures |
| `3a8c9fbe` | Diagnostic follow-up; two strict principal-source producer failures remain |

These patches preserve source for the next repair without depending on an
external worktree. They are reference material, not patches to apply wholesale
to the working branch. Their historical receipts remain tied to their measured
sources. Raw renderer outputs and reference images named by those receipts
retain their existing storage locations; the archive contains the receipts
and implementation deltas, not every full-census image.

The archive manifest lists each member and digest. It can be checked without
extracting files:

```python
import hashlib
import json
from pathlib import Path
import tarfile

manifest = json.loads(Path(
    "docs/progress/evidence/branch-checkpoint-2026-10-17.json"
).read_text())
archive = manifest["archive"]
assert hashlib.sha256(Path(archive["path"]).read_bytes()).hexdigest() == archive["sha256"]
with tarfile.open(archive["path"], "r:gz") as source:
    assert set(source.getnames()) == {entry["member"] for entry in archive["entries"]}
    for entry in archive["entries"]:
        data = source.extractfile(entry["member"]).read()
        assert len(data) == entry["bytes"]
        assert hashlib.sha256(data).hexdigest() == entry["sha256"]
```

## Renderer and release status

The latest completed census remains the private `ec3883b3` measurement:

| Suite | Exact | Different | Render errors |
| --- | ---: | ---: | ---: |
| Original | 20,931 / 22,924 | 1,993 | 0 |
| Expanded | 21,659 / 23,728 | 2,069 | 0 |

These counts do not measure the authoritative branch. Later selected gains
cannot be added to them. Native API completeness, full renderer nonregression,
compositor/3D, hardware, accessibility, performance and release qualification
remain open as recorded in [current status](current-status.md). There is no
whole-project completion percentage.

## Checkpoint verification

Fresh local verification and its source scope are recorded in
[the check results](evidence/branch-checkpoint-2026-10-17-checks.json).
The evidence files retain the original branch names and merge-readiness result
as historical facts. The later experimental merge instruction changes workflow
policy, not those measurements. The final correction changes documentation and
workflow branch filters only; runtime code and archived evidence are unchanged.

| Check | Result |
| --- | --- |
| Rust workspace formatting | Pass |
| Release source, generated style and C ABI contracts, conformance and performance schemas | Pass |
| Full accountability audit with historical PNG presence | All seven checks pass; this is not a new Chromium census |
| Seventeen Python closure, porter, renderer, schema, packaging and security modules | 300 tests pass |
| Eight additional archive, renderer, closure and C fixture generator checks | Pass |
| CI, Format Check and Hardening YAML | Initially parsed with stacked PR bases; final correction restricts PR bases to `main`, with push filters still `main` only |
| Four archived candidate deltas | Application and target blob restoration verified |
| Native build and ABI consumers | Pinned-sysroot build passes; 164 exports, 41 C consumers and 35 C++ consumers pass |
| Changed C and C++ consumers | clang-format 18.1.3 passes |
| Headless Rust workspace excluding the generated pixel executable | 9,401 passed, one failed and 13 ignored across 224 test and doctest groups; exit 101 |
| Independent clipping guard reproduction | The same named test fails at the same row and scale; exit 101 |

The failing test is
`openui-engine::native_resize_control_clip::actual_native_scrollbar_frames_are_clipped_independently_of_track_and_thumb_ink`.
Its assertion at `native_resize_control_clip.rs:138` rejects changed ink at
physical row 57 for scale 1.25. The fixture has a 14 CSS-pixel height and checks
Auto and Thin forced scrollbars on narrow native boxes. The shared paint clip
path and the guard's pixel-center boundary contract need adjudication against
pinned Chromium; neither a production diagnosis nor a guard waiver is claimed.

The ordinary local native build cannot link because this WSL host lacks the
system C runtime development objects and libraries. The pinned-sysroot build
is recorded separately. Hosted CI builds the portable configuration afresh;
its results apply only to the exact PR head they test. No local source check
or PR workflow substitutes for the open renderer and release gates.

## Next bounded task

Update 20 October 2026: the scrollbar frame clip guard below is resolved, as
recorded in [the repair record](native-scrollbar-frame-clip-2026-10-20.md).
The padded multicolumn principal-source continuity task followed; it is complete, see [the principal-source record](multicol-principal-source-2026-10-21.md). Update 21
October 2026: the Windows text work also surfaced an inline-layout gap. Blink
aligns vertical-flow line boxes on the central baseline
(`ComputedStyle::GetFontBaseline`), while Open UI uses the alphabetic
baseline. See [the Windows text record](windows-directwrite-text-2026-10-21.md);
it needs its own census-verified task. That task is complete; see
[the central baseline record](vertical-central-baseline-2026-10-21.md).

When feature work resumes, start one focused PR from the latest `main`.
First resolve the failing native scrollbar frame clip guard, owned by the
shared overflow-control painting path and its Engine integration test:

1. Reproduce the exact named failure using the command in the check results.
   Capture the offending box geometry and boundary pixels independently.
2. Compare the same native scrollbar state with pinned Chromium at all five
   scales, for Auto and Thin bars and the narrow-box dimensions in the guard.
   Fix shared paint geometry if the native output is wrong; change the guard's
   boundary contract only if the independent oracle proves that contract wrong.
   Preserve exact RGBA comparison and existing clipping regressions.
3. Run the complete affected Engine test binary, relevant paint tests and the
   native workspace gate. Run full renderer nonregression before admitting
   global renderer gains, then update status and open the focused repair PR.

The subsequent renderer task is principal-source continuity in padded
multicolumn rows. Reproduce the two strict producer failures from the archived
candidate against the authoritative branch before repairing finite principal
ownership. Keep complete decoration, physical clip boundaries, restoration
and repeat checks covered. Required public Rust behavior must be verified by
a native consumer, with C sharing the same implementation. Font backend
activation, compositor/3D and the physical release lab remain separate roadmap
work.

Pending saved-image audits retain the state recorded in their receipts; this
checkpoint does not restart them or assert that an old process is still active.
No further feature work is included after the requested pause.
