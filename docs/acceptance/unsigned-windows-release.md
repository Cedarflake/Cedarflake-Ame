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
