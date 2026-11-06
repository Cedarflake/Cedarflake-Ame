# Unsigned Windows publication verification

Date: 2026-11-05

Scope: [PR 49](https://github.com/Cedarflake/Cedarflake-Ame/pull/49), initially
`4c472aa99ec1a49e4d42f4c7338d224566acd2de`, with the corrections accompanying this record.
[ADR 0015](../architecture/0015-windows-release-distribution.md) owns the release policy.

PR 49 extends the existing candidate and publication pipeline with unsigned verification.
The overlapping PR 50 was closed; it added a separate unsigned publication workflow.

Two defects were corrected before integration: literal `__DOLLAR__` placeholders prevented
artifact/source outputs from being evaluated, and the installer guardrail still expected two
tag-bound checkouts after unsigned verification added a third. The latter caused Static job
`127200558945` to fail. Output mutation cases now reject both literal placeholders; the existing
trust checks retain exact tag/source binding for both channels.

Focused checks passed on the corrected working tree:

- Windows PowerShell 5.1 installer guardrails, including ACL, identity and uninstall fixtures.
- Hosted gate contract, portable archive and publication policy regressions.
- All workflow files through the repository entrypoint with pinned actionlint 1.7.12.
- Unsigned candidate admission with a bounded manifest fixture and locally built application and
  broker binaries. Tampered bytes, a different source commit and an unlisted file were rejected.
  Other payload entries were structural fixtures; this was not a packaged application launch.
- Whitespace checks for the affected patch.

Checks used repository locking and leased scratch storage. They did not install a service, access
real-library sources, sign binaries or publish a release. Full hosted candidate execution and
post-publication verification remain the release workflow's responsibility; no release was
dispatched during this review. The user's merge instruction permits integration before CI finishes.

## Alpha MSI supplement — 2026-11-06

The supplemental `v0.1.0-alpha.1` MSI uses the exact application payload from successful
[release run 42210989286](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42210989286),
source `ab9146fdfbe2eb44436394dbcc36a31dea14e273`. Candidate admission rechecked the tag, source,
complete file manifest and executable identities. Application runtime evidence remains that run's
evidence; this supplement does not rebuild or relaunch the application against the real catalog.

The MSI contains 41 files, including the pinned classification models, tokenizer and notices.
Its size is 369,717,248 bytes; SHA-256 is
`bda9182efd714d1437d7edec6d071c15728c1754da08ff1dbac049ef8c0f1dfb`.
Application version `0.1.0-alpha.1+1` maps to MSI `ProductVersion` `0.1.1` under
[ADR 0027](../architecture/0027-windows-installer-lifecycle.md). The existing portable asset and
version tag remain unchanged.

Version-admission regressions passed in PowerShell 7 and Windows PowerShell 5.1. Native installer
policy, actual MSI database validation, 28 rejecting mutations and 16 native option transitions
passed. The model-bearing package has four embedded cabinets; the layout guard now verifies every
cabinet rather than incorrectly requiring exactly one, and rejects missing or detached payloads.

Explicitly authorized Windows 11 host acceptance completed fresh installation, forced repair and
uninstall with exit code 0 for all three operations. Installed and repaired payload hashes, model
files, registered version, running service identity, restricted SID and required privileges matched
the receipt. Uninstall removed the service, product registration and installation directory.
All 20 existing catalog/model data files retained their paths, lengths and SHA-256 hashes. No
source-media access was needed. This run qualifies the supplemental package's install/repair/remove
path; historical upgrade and rollback coverage is not replayed or relabeled as a new result.
