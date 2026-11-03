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

## Synchronization readiness and projections — 2026-11-04

Against `f6189e8a`, `LibraryChangeReadiness` exposes nine read-only queue and capacity queries.
Their SQLite implementations live with the existing readiness owner. Application lane selection,
journal closing selection, status projection and retained-source pruning consume the corresponding
persistence ports. Source comparison confirms the nine native query bodies and 14 application
policy bodies are unchanged. Priority, fairness, retry timing, transaction-local lease admission and
worker/resource lifetimes retain their existing behavior.

Fourteen migration entries retire, leaving 49; the 61 retained technical edges and both frozen
inventories are unchanged. Existing fixtures import the same native implementation through its
contract. No tests or assertions are added or removed.

All 57 existing checks pass: four readiness, six recovery ownership/rotation, nine volume selection,
six status projection, eight production baseline/fairness/capacity and 24 dependency cases. Initial
fixture compilation needed the trait import in its inline test scope; final test and production
builds emit no warnings. Formatting reports 794 files unchanged, the scratch lease retires, and three
changed documents have 32 resolving local links and anchors.

The coordinator facade has 1,660 source lines including conditional fixture helpers, no inline test
cases and 934 primary dedicated-fixture lines. Journal baseline has 443 source/326 inline-test lines.
The queue facade has 1,613 source lines and its primary dedicated fixture has 6,185 lines. The
readiness owner has 617 source lines with 386 dedicated readiness-test lines; these queue owners have
no inline tests.

## Worker catalog sessions and queue dispatch — 2026-11-04

Against `3f0437e3`, `CatalogLaneSession` supplies the existing validated lane connection to Live,
Journal-drain and Recovery-drain workers. `LibraryChangeDispatch` owns the missing lane-specific
lease/legacy-control operations; their implementation moves to the native dispatch owner. Shared
session retention, cancellation points, lane selection, bounded admission and joined retirement are
preserved. Source comparison confirms four native dispatch bodies, seven worker/use-case bodies and
the original native lane-opening body are unchanged.

Seven migration entries retire, leaving 42, with 61 retained technical edges and unchanged frozen
inventories. Existing fixtures retain their native construction and assertions; only contract imports
change. No tests, transaction rules, schema, source authority or bridge contracts are added or removed.

All 55 existing checks pass: 31 worker lifecycle, cancellation, lane priority, directory/recovery and
session-identity cases, plus 24 dependency cases. Test and production builds emit no warnings.
Formatting reports 794 files unchanged, and the worker-session scratch lease retires.

The queue facade has 1,413 source lines and 6,187 primary dedicated-fixture lines; its extracted
dispatch owner has 216 source lines. The SQLite facade has 3,329 source lines including conditional
diagnostic helpers, with 91 lines in the affected session-validation fixture. Neither facade has
inline tests. The SQLite facade changes only its module declaration.

## Journal catalog and first-import admission — 2026-11-04

Against `8b79062f`, journal opening/closing, page capture, registration failures and restart fallback
consume catalog ports. First-import admission and journal selection use explicit readiness reads.
The original native baseline and capability transactions implement `JournalBaselineAdmission`;
publication permits retain their original acquisition and release points. `JournalCatalogSession`
constructs the native namespace reader from the same retained session, removing its concrete exports.

Comparison preserves six native and 17 application bodies, apart from the namespace factory
expression and one relocated trait import. Both frozen inventories are unchanged; 18 migration
entries retire, leaving 24 with the same 61 retained technical edges. Existing tests keep their
assertions and fixtures. Schema, source authority, bridge contracts and broker protocol are unchanged.

All 75 existing checks pass: 51 first-import admission, baseline, replay/fallback and namespace cases,
plus 24 dependency cases. An initially unused production trait import moves into its opening fixture;
the behavioral results remain applicable because that correction changes only import scope.
Final test and production compilation emits no warnings, and the final inventory check passes.
Formatting reports 794 files unchanged; both scratch leases retire. The three changed documents have
15 resolving local links and anchors.

The SQLite facade has 3,287 source lines including conditional fixture helpers, with no inline test
cases. Its first-import queries now share the 230-line scan-lifecycle owner and 578-line dedicated
fixture. Journal baseline persistence has 488 source/594 dedicated-test lines and no inline tests;
the journal facade changes one import and retains 1,316 source/3,131 inline-test lines. The application
baseline has 453 source/327 inline-test lines and its opening fixture has 468 lines. First-import
admission retains its 690-line dedicated fixture.
