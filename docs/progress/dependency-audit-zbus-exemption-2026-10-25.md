# Reviewed zbus audit exemption, 25 October 2026

The `dependency-security` gate has failed on every PR since RustSec marked
three zbus 5 crates unmaintained on 16 October 2026. The
[audit split record](../memory/entries/0066-dependency-audit-is-its-own-ci-job-zbus-5-unmain.md)
kept the gate strict while upstream moved. On 25 October 2026 the user
directed a reviewed, dated exemption for exactly these three notices, in its
own PR.

## Findings

The audit of PR 26 (`cargo-audit 0.22.2`, run `40175208020`) reports no
vulnerabilities. It lists three `unmaintained` warnings, all in
`bindings/rust/Cargo.lock`; the fuzz lockfile is clean.

| Advisory | Crate | Version | Notice |
| --- | --- | --- | --- |
| RUSTSEC-2026-0370 | `zvariant` | 5.9.0 | unmaintained, merged into zbus 6.0 |
| RUSTSEC-2026-0371 | `zbus_names` | 4.3.1 | unmaintained, merged into zbus 6.0 |
| RUSTSEC-2026-0372 | `zvariant_derive` | 5.9.0 | unmaintained, merged into zbus 6.0 |

## Upstream status, checked on crates.io on 25 October 2026

Open UI reaches zbus only through AccessKit's Linux AT-SPI backend
(`accesskit_unix` 0.23 and `atspi` 0.29 in the lockfile).

| Crate | Newest release | Requires |
| --- | --- | --- |
| `accesskit_unix` | 0.24.0, 2026-09-25 | `atspi ^0.29`, `zbus ^5.19` |
| `atspi` | 0.30.0, 2026-05-06 | `zbus ^5.5` |

No release moves to zbus 6, and zbus 6 also needs Rust 1.89 against the 1.85
MSRV. The notices are about maintenance, not a known vulnerability.

## Change

| Change | Owner | Source |
| --- | --- | --- |
| `REVIEWED_EXEMPTIONS` lists the three advisories with crate, kind, review date, expiry (25 January 2027) and upstream status | release tooling | `tools/release/check_dependency_security.py` |
| `classify_warnings` accepts a warning only when its group and kind are `unmaintained` and its advisory and crate match an unexpired exemption. Vulnerabilities, other warning kinds, other crates and expired entries still fail. cargo-audit's exit status 1 is accepted only when every reported warning is exempt | release tooling | same |
| An exemption that no lockfile reports fails the gate, so it is removed as soon as AccessKit moves | release tooling | same |
| Receipts record the exemption list and each lockfile's exempt and failing warnings | release tooling | same |
| Tests cover the accepted notices and each rejected variant: an extra crate, a wrong crate, an unsound kind, a vulnerability, a partial report, a clean report, and an expired entry | release tooling | `tools/release/test_dependency_security.py` |

cargo-audit still runs with an empty `ignore` list and `--deny warnings`, and
reports carrying cargo-audit ignores or filters are still rejected, so the
full report stays in the evidence. The same checker gates the release
workflow (`release-v02.yml`) and `build_v02_linux.py`, so the exemption
applies there too.

## Verification

| Check | Command | Result |
| --- | --- | --- |
| Checker tests | `python -m unittest tools.release.test_dependency_security` | 14 pass |
| The real PR 26 report classified by the new logic | `classify_warnings` on the run `40175208020` artifact | production: three exempt, none failing; fuzz: none |
| `dependency-security` CI job | PR 27, run `40177089214` | Passed; the other CI jobs passed too |

## Review

Re-check `accesskit_unix` and `atspi` before 25 January 2027. Once they move
to zbus 6, update AccessKit and delete the exemptions. The gate fails both
when an exemption expires and when an exemption is no longer reported.
