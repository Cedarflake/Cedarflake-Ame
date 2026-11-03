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

## Recovery source and index contracts — 2026-11-04

Against `edda4db9`, authoritative scopes, indexed startup and metadata recovery reuse the admitted
source selectors. `PublicationDirectorySource` borrows its cursor from the held namespace, while
inventory opening and absence/final-publication guards retain their original scopes.
`IndexedRecoveryRepository` exposes connection-local hints, namespace binding and atomic candidate
publication. The three original hint-query bodies and the existing spool/queue transactions are
unchanged; their native execution token remains private to persistence.

Nine legacy entries retire, leaving 63, with 61 retained technical edges and unchanged frozen
inventories. This migration exposed complete-directory materialization before the authoritative
entry limit; the [bounded-enumeration repair](#bounded-authoritative-enumeration--2026-11-04) records
its separate correctness change.

All 51 distinct focused checks pass: seven index-publication, two runtime-selection, nine inventory,
eight authoritative-scope, one canonical-constructor/rebase and 24 dependency cases. An obsolete
constructor alias found by the production build was removed; five fixture calls now use its existing
delegate. Final test/production builds emit no warnings, formatting reports 794 files unchanged and
both leased scratch directories retire. Two changed documents have 13 resolving local links.

The metadata-recovery facade has 1,422 lines including conditional hooks, and 3,884 primary
dedicated-test lines. Authoritative
changes have 559 source/1,408 dedicated-test lines; indexed startup has 169 source/385 dedicated-test
lines. The hint repository implementation has 114 source lines. No tests or assertions are added or
removed in this contract migration.

## Bounded authoritative enumeration — 2026-11-04

Against `9af3e7c7`, the existing oversized-subtree fixture observed all 18 directory entries with an
entry limit of one. The repaired cursor reuses the pinned streaming reader and its fixed 64-KiB
Windows buffer, allowing the application to stop after one admitted entry and one overflow
observation. It preserves pageable recovery, sorted application paths and final file revalidation;
the eager sorted reader remains only in the existing test inventory adapter.

`PublicationDirectoryVisits::finish` requires complete enumeration and revalidates the current
relative directory through an attribute-only pinned-root open. The added native regression moves
an exhausted cursor's directory and replaces its path before completion. Without the binding check,
completion incorrectly succeeded; the repaired check rejects the replacement. Production source
access remains read-only and no-recall; schema and transactions are unchanged.

All 37 focused checks pass: eight authoritative-scope/terminal-media cases, five native streaming
cases and 24 dependency cases. The bounded fixture observes at most two entries and retains its
original pageable-recovery, catalog and asset-identity assertions. Test and production builds emit
no warnings. Formatting reports 794 files unchanged; both changed documents have six resolving local
links and anchors, and the reproduction/final scratch directories retire. The dependency inventory
remains at 63 migration entries and 61 retained technical edges, with neither frozen baseline changed.

The authoritative facade has 564 source lines and its dedicated fixture has 1,423 lines. The
local-files facade has 2,603 source/3,460 inline-test lines; the directory cursor module has 284 lines
including retained test-only helpers, with 144 lines in its dedicated native fixture. One existing
application test gains a bounded-read assertion and one native regression is added.
