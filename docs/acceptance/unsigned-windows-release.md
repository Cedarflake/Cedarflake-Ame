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

## Mandatory hosted ZIP and MSI — 2026-11-07

Release orchestration now requires one read-only packaging workflow to emit both ZIP and MSI from
the verified candidate. The installer retains its pinned models and selected signature channel.
The publisher admits exactly those two files, creates new releases as drafts and makes them public
only after both remote digests match. Published verification downloads and checks both attachments.

The actual publisher script passed isolated transaction fixtures in PowerShell 7 and Windows
PowerShell 5.1, including a missing MSI, conflicting or absent digest, partial upload, failed lookup,
paginated assets and repeated publication. Partial upload retained a draft; conflicting or missing
inputs produced no release mutation. Published-download checks rejected a missing or changed MSI.
The complete installer guardrails and pinned actionlint 1.7.12 passed. The downloaded Alpha candidate
also passed the generalized unsigned verifier. Pinned WiX acquisition was exercised from retained
NuGet packages through extraction, digest checks, compiler identity and builder argument binding.

The candidate entry is 155 lines; packaging and publication own 214 and 202 lines respectively.
The affected dedicated checks contain 183 publication-transaction lines and 86 workflow-contract
lines. Moving the publication contract reduced the pre-existing installer guardrail to 1,915 lines;
unrelated portions were retained. These owners contain no inline test code. No application rebuild,
service installation, tag change or replacement of the existing Alpha assets was needed for these
orchestration checks. Hosted execution for the new commit remains to be checked after submission.

## Versioned release notes — 2026-11-09

The publisher now reads `docs/releases/<tag>.md` from the dispatch's immutable workflow commit.
The actual workflow script passed isolated publication fixtures in PowerShell 7 and Windows
PowerShell 5.1: Markdown and Chinese text survived unchanged, repeated publication updated the
body without replacing either package, and missing, empty or mismatched notes stopped all writes.
The publication contract and pinned actionlint 1.7.12 also passed. These checks made no remote writes.

The candidate and publisher contain 157 and 231 production lines; their dedicated publication
and wiring checks contain 213 and 89 lines, with no inline tests. Complete CI passed at `37e0da65`
in run `42509982166`. The Alpha 2 draft body matches the versioned document; final publication
reads the same owner automatically.

Alpha 2 run `42508623527` passed the candidate, complete regression and synthetic-workload gates.
ZIP creation also passed, but MSI creation stopped before compilation because CMake discovery
returned multiple application paths to one string parameter. The packager now selects the first
application in PATH order. An isolated two-executable PATH fixture reproduced the old parameter
binding failure and accepted the corrected selection; publication wiring and actionlint passed.
Application source and the tag remained unchanged by this packaging correction.

Run `42517797115` then passed the same application gates but exposed the hosted image's migration
to Visual Studio 2026: the installer requires the Visual Studio 2022 CMake generator. Packaging now
uses the supported `windows-2022` image; application build and runtime verification retain their
existing host. The [upstream image inventory](https://github.com/actions/runner-images/blob/main/images/windows/Windows2022-Readme.md)
confirms Visual Studio 2022, its C++ toolchain and the Windows SDK.

The manifest-verified candidate from that run produced a complete local validation MSI with Visual
Studio 2022, embedded models, native policy checks, database rejection cases and native option-state
evaluation. Native checks used leased scratch storage after the default temporary directory rejected
fixture creation. No installation or service registration ran, and this local package is not a release
attachment. The candidate's Flutter notices also contain Ame's Apache 2.0 license entry.

Run `42524891369` passed every candidate, regression, performance and package gate and uploaded
both packages. Publication then stopped because GitHub's tag lookup returns published releases,
not drafts. The publisher now discovers drafts through the authenticated paginated release list
and rejects ambiguous matching releases. Faithful draft responses reproduced the failure in the
existing transaction fixture; the corrected workflow passed it in PowerShell 7 and Windows
PowerShell 5.1, including existing drafts, later release pages and duplicate-tag rejection.
The publisher contains 252 production lines and its transaction fixture 233 dedicated test lines,
with no inline tests. Fix `953b58f3` retains every package and source-identity check.

Alpha 2 is [public](https://github.com/Cedarflake/Cedarflake-Ame/releases/tag/v0.1.0-alpha.2).
After authorized removal of the accidental empty duplicate, publication resumed from the original
draft and its existing CI-uploaded attachments. The corrected production lookup rechecked both
remote digests and the unchanged source tag against the successful candidate and packaging jobs;
no application rebuild, package replacement or new upload occurred. The original run retains its
publication failure. The independent
[published-download verification](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42535241863)
passed, and the public body matches `docs/releases/v0.1.0-alpha.2.md` after line-ending normalization.

| Asset | Bytes | SHA-256 |
| --- | --- | --- |
| Portable ZIP | 47,395,283 | `f832aec57f7fd3f0fc096f7403d8e58a3d8d9eb77f34225cd5f19f39d4b8a373` |
| MSI | 383,852,544 | `69245cd49430b75daa5ab5f04c66812636b9dd765d10799fcad570867735dd4e` |
