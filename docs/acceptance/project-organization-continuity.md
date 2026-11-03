# Project organization: continuity verification

This record owns O7's incremental, journal and synchronization boundary evidence. The
[roadmap](../roadmap.md#project-organization-after-r5) owns disposition, the
[execution plan](../plans/project-organization.md#o7-repository-conformance-and-handover) owns scope,
and [earlier organization evidence](project-organization.md) retains the preceding slices and limits.

## Guarded incremental sources — 2026-11-04

Against `f2141737`, incremental changes and preview reconciliation consume `PublicationFileSource`
and `GuardedMediaInspector`. The native source implements `MediaSource` with the existing pinned,
root-relative reader. Initial root admission, no-recall opening, metadata reuse, terminal-media
classification, revision rebasing, queue lease authority and transaction order are preserved.
The native decoder and file revalidation implementations retain their existing behavior; no SQL,
schema, generated bridge or source authority changes are included.

Twenty-one legacy entries retire, leaving 72. Two exact factories are registered under the
[incremental source contract](../architecture/workflow-ownership/native-composition.md#incremental-source-resources),
bringing retained technical edges to 61. Neither frozen inventory is regenerated. Existing assertions
are retained; one native fixture imports the inspection trait and calls its renamed method.

All 72 focused checks pass: eight authoritative-path rebase, eight incremental revision rebase,
three changed-source publication, 16 preview reconciliation, 12 incremental admission/publication,
one first-content-read and 24 dependency cases. They cover lease replacement and cancellation,
held root/ancestor exclusion, missing/recreated/renamed sources, metadata reuse, corrupt/locked media
and zero-content-read admission. An unused import found by the initial build was removed; subsequent
test and production builds emit no warnings. No tests or assertions are added or removed, and the
leased scratch directory retires. Formatting reports 794 files unchanged; four changed documents
have 59 resolving local links and anchors. Full dependency-cycle auditing, test consolidation and final
real-library client verification remain in the O7 handover scope.

The incremental facade has 1,345 source lines and its primary dedicated fixture 3,839 lines.
The authoritative-path owner has 328 source lines, with 408 dedicated rebase-test lines; preparation
rebase fixtures retain 504 lines. Preview reconciliation has 157 source lines and 726 dedicated-test
lines. Media inspection has 227 source/134 inline-test lines. The local-files facade keeps 2,609
source/3,460 inline-test lines; its affected dedicated admission fixture has 204 lines.
