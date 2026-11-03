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

## Located-root capability and recovery — 2026-11-04

Against `852da552`, the located directory value moves to `ports/root_location` and retains the same
native namespace through `PublicationNamespace`. The recovery worker accepts validated catalog
sessions and `RootLocationRepository`; its named production factory selects the locator. Initial
fixture compilation exposed the old capability's shared-thread requirement, now preserved by the
boxed guard's `Send + Sync` bounds. Existing tests and assertions are unchanged.

Source comparison preserves both recovery transaction bodies, the locator and both worker/use-case
bodies apart from guard boxing and an explicit generic-session borrow. Identity validation still
precedes the write and commit; cancellation, preemption, rollback and source safety retain their
owners. Four migration entries retire, leaving 20; the exact locator factory is a documented
composition edge, bringing retained technical edges to 62. Both frozen inventories are unchanged.

All 38 existing checks pass: four native discovery, four persistence, two worker-lifetime, four
production relocation/availability and 24 dependency cases. Final test and production builds emit
no warnings. Formatting reports 794 files unchanged; the scratch lease retires, and the three changed
documents have 16 resolving local links and anchors.

The production facade has 1,662 source lines including conditional fixture helpers, no inline cases
and 934 primary dedicated-test lines; its changes select the required factory. The local-files facade
retains 2,603 source/3,460 inline-test lines and changes only its export. Recovery persistence has
288 source/242 primary dedicated-test lines, and native root discovery retains its 90-line fixture.

## Scheduling and automatic recovery — 2026-11-04

Against `fd88f23c`, Live/Journal selection and synchronous catalog scheduling consume existing
readiness, admission and dispatch ports. Automatic recovery opens its lane through `CatalogLaneSession`
and consumes `IndexedRecoveryRepository` and `LibraryChangeQueue`. Its optional search index is
selected by the exact factory in the [composition contract](../architecture/workflow-ownership/native-composition.md#indexed-recovery-resources).
Comparison preserves all four scheduling/worker bodies except that factory expression, including
priority, retained-source handoff, root revalidation, failure deferral and failed-start restoration.

Five migration entries retire, leaving 15; the index factory brings retained technical edges to 63.
Both frozen inventories are unchanged. No tests, assertions, schema or source authority change.
All 44 existing checks pass: 20 priority, ingress, recovery authority/handoff and indexed-startup
cases, plus 24 dependency cases. Removing one unused trait import preserves those behavior results;
final test and production compilation is warning-free, and the final inventory check passes.
Formatting reports 794 files unchanged; both scratch leases retire.

The production coordinator has 1,677 source lines including conditional fixture helpers, no inline
cases and 934 primary dedicated-fixture lines. Catalog scheduling has 120 source lines and automatic
recovery has 264, including its existing conditional thread-start failure helper.

## Poll resources and availability — 2026-11-04

Against `6a6ee2f5`, `poll_catalog::WorkerCatalogSession` owns the cached worker proof, path validation
and deferred cleanup connection factory. Cleanup consumes `MetadataInventoryRepository`; its factory
remains alive through connection retirement to retain the original shared proof. Root observation
uses `SourceStateProbe`, and pending-metadata projection uses `LibraryChangeReadiness`.

Comparison preserves the pending query/projection, complete retained poll host, cached acquisition and
six worker bodies apart from the declared binding/factory calls. Five coordinator/use-case edges migrate; five
existing poll-resource edges receive exact technical registrations with their lifetime and replacement
contract. The frozen inventory retains five migration entries, with 73 technical edges. Both original
inventories remain unchanged. Existing fixtures change only their session binding; no tests or
assertions are added or removed.

All 48 existing checks pass: 24 connection reuse/replacement, close/deadline, cleanup, retained-source,
metadata-status and root-availability cases, plus 24 dependency cases. Test and production builds
emit no warnings. Formatting reports 794 files unchanged and the scratch lease retires.

The coordinator has 1,658 source/934 primary dedicated-fixture lines; poll resources have 301 source/
127 dedicated-test lines and cleanup has 189 source/207 primary dedicated-test lines. These source
counts include conditional helpers and contain no inline test cases. The native readiness owner has
631 source lines; metadata rechecks retain 183 after their unchanged projection query moves.

Review found three inherent maintenance calls through an inferred native handle, outside the
explicit-type inventory. The [maintenance migration](#catalog-maintenance-contract--2026-11-04)
records their resolution.

## Catalog maintenance contract — 2026-11-04

Against `2b4489c9`, poll preflight consumes `SynchronizationMaintenance` for media rechecks, metadata
rechecks and legacy automatic-scan retirement. The native binding owns retirement and delegates to
adapter-private recheck methods. Their original SQL, bounded admission, cursor and busy-timeout
lifetimes remain in their existing owners; application retains preflight order and its once-only flag.

Source comparison preserves both complete recheck owners, the retirement body and production
preflight. Existing fixtures change imports only. All 17 selected checks pass: 14 recheck/retirement
cases, two production-poll cases and the repository dependency inventory. Test and production builds
emit no warnings; formatting reports 794 files unchanged, and the scratch lease retires. No tests,
assertions, schema, frozen inventories or boundary-policy entries change.

The SQLite facade has 3,239 source lines including conditional helpers, with no inline cases. Its new
maintenance binding has 71 source lines and the port has 21; the retained retirement fixture has
292 lines. The five explicit migration entries and 73 technical edges remain separate from this
resolved inferred-call dependency path.

## Journal boundary probing — 2026-11-04

Against `54285cb2`, baseline opening/closing and scan resume consume `JournalBoundarySource` and its
held root reader. The adapter owns registration, protocol response conversion and error mapping;
application retains cancellation, publication and `ScanResumeCoverage` decisions. Borrowed sessions
remain active, while the resume session closes after verification and retires before its root handle.
The [composition contract](../architecture/workflow-ownership/native-composition.md#journal-boundary-resources)
admits only the two stateless selection functions. R249 retires, leaving four migration entries and
75 exact technical edges; both frozen inventories and generated contracts are unchanged.

Source comparison preserves the query/conversion and both error mappers, baseline policy and
publication bodies, worker cancellation/retirement and resume verification. Acquisition order,
close-error precedence and connection-before-root drop order are retained. Two protocol tests move
to the adapter; the cancelled-opening fixture now rejects root acquisition directly through the port.
The test count is unchanged.

All 49 selected existing checks pass: 25 behavior cases and 24 dependency cases. These include the
ordinary native journal's cross-process paused-import resume, which passes in 3.35 seconds against
generated files. It retains the checkpoint, avoids revisiting completed directories, converges the
changed membership and preserves source bytes. Test and production builds emit no warnings; final
inventory verification passes, formatting reports 794 files unchanged, and both scratch leases retire.
This bounded fixture leaves the retained-library pause timeout and final real-library client checks
with their existing acceptance owners.

The baseline owner has 350 source/215 inline-test lines; opening has 182 source/471 dedicated-test
lines. The worker has 381 source lines and no inline cases. The new protocol adapter has 147 source/
122 dedicated-test lines, and independently owned resume resources have 72 source lines.

## Journal volume readers — 2026-11-04

Against `eff3fb1c`, named/identity readers and their protocol translation move to
`adapters/persistent_journal`. Domain owns read-window/recovery rules and the shared deterministic
ID function; trim diagnostics remain in the adapter. `PersistentJournalVolumeSource` hides native
registrations and reader selection from the page worker, which retains root handles through its
unchanged capture/publication boundary. The [named factory](../architecture/workflow-ownership/native-composition.md#journal-volume-resources)
brings technical edges to 76; the four remaining fixed migration entries and both inventories are
unchanged. This protocol boundary was outside the fixed explicit-adapter roster.

Source comparison preserves all eight relocated reader/translation/test files apart from imports
and owner paths, the domain rules, ID bytes, trim diagnostic, complete capture owner, page worker,
root failure publication and 21 shared-ID consumers. Reader assertions and test counts are unchanged;
the page fixture supplies the same classified failure through its new port. Initial compilation found
two old ID imports in test support; both now use domain directly, without a compatibility export.

All 64 selected existing checks pass: 31 reader cases, shared-ID and root-failure cases, five
cross-root/retry cases, two ordinary native cross-process cases and 24 dependency cases. The native
restart and paused-import fixtures pass in 3.99 and 3.81 seconds, retaining identity, checkpoint,
source-byte and retirement assertions. Final test/production builds emit no warnings; formatting
reports 794 files unchanged and the scratch lease retires.

Source/primary dedicated-test counts are 1,151/1,112 for the named reader, 436/753 for the identity
reader, 586/668 for application capture and 111/124 for the page worker, with no inline cases. The
shared-ID move leaves the scan facade at 751/5,505 and incremental changes at 1,346/3,839; their
workflow bodies remain unchanged, and the existing namespace-ID case moves to domain.

## Journal runtime connection — 2026-11-04

Against `1dbdb5c8`, `JournalRuntimeConnection`, `JournalRuntimeFactory` and `JournalRuntimeSession`
separate native connection/caller admission from coordination. The generic coordinator and workers
consume these ports; status and fallback projection consume connected availability. The adapter
retains the same session, caller acquisition, boundary/volume sources and close-error conversion.
The [two native bindings](../architecture/workflow-ownership/native-composition.md#journal-runtime-resources)
replace the previous worker factories. R116 retires, leaving three fixed migration entries and
76 exact technical edges. Frozen inventories, generated contracts and schema are unchanged.

Source comparison preserves 48 coordinator bodies, the complete worker, page, registry facade,
reader selection, source acquisition, moved identity-admission and close-error bodies, and retirement
order. The 38 cases in modified test files retain their scenarios and assertions apart from the
LiveOnly fixture's raw-enum inspection, which now checks unavailable connection and reader access.
Test assembly uses the same native adapter with its original controlled sessions; no cases are added.
Initial compilation identified two old fixture references; their imports and state checks now use
the appropriate adapter and port boundaries.

All 84 selected existing checks pass: 60 behavior cases and 24 dependency cases. Coverage includes
watcher-before-journal startup, cached close failure, blocked/late connection, joined retirement,
baseline publication, fallback recovery and status projection. The ordinary cross-process restart
and paused-import fixtures pass in 3.71 and 3.49 seconds. One dependency parser case initially could
not create its system temporary directory; it passes in leased repository scratch, while the other
23 results remain valid. Test/production builds emit no warnings, formatting reports 794 unchanged
files, and scratch leases retire. The retained-library timeout and final real-library client check
remain with their existing acceptance owners.

The coordinator has 1,640 source lines, including conditional helpers, and 10,605 primary dedicated-
test lines; the worker has 359 source lines. Neither has inline cases. The native connection adapter
has 138 source lines including test assembly, and the port has 33. Their lifecycle coverage remains
in the existing production and runtime-registry fixtures.

## Synchronization resource bindings — 2026-11-04

Against `6ebae723`, source review confirms that R109 binds only the opaque ingress reservation in
the production resource host, and R115 selects the existing `LibraryChangeSourceStarter`. The generic
handoff consumes `LibraryChangeIngress`; SQLite retains binding validation, zero-wait admission,
commit/rollback and timeout restoration. Neither reference adds use-case policy or native operations
outside the [two scoped bindings](../architecture/workflow-ownership/native-composition.md#synchronization-observation-resources).

These entries become accepted technical composition boundaries with explicit lifetime, replacement
and regression owners. The repository inventory check passes with one migration entry (R172) and
78 technical edges. Only the policy and its owning documents change; source, tests, generated files
and frozen inventories are unchanged. The earlier ingress/scheduling evidence and `6ebae723` startup,
shutdown and late-completion results retain their scope; no product workload is repeated for this
classification. Full dependency conformance and real-library acceptance remain open under O7.

## Preview identity relocation — 2026-11-04

Against `1c0ad0fc`, missing-source path planning consumes `SourceIdentityLocator` from the existing
preview-source factory. The Windows implementation retains its complete metadata lookup, identity
validation and error bodies. Unsupported platforms retain unavailable lookup. Application keeps
exact/relocated path selection, guarded reconciliation, request-generation admission and publication;
the obsolete native function exports retire.

All 17 selected existing checks pass: 16 preview source-reconciliation cases and the repository
dependency inventory. The real directory-rename fixture preserves asset/file identity, reads zero
directory entries and verifies unchanged source bytes after preview recovery. Other cases retain
recreation, stale authority, rollback, peer publication and durable-retry assertions. Source comparison
preserves the native query, source leases, path decisions and complete reconciliation workflow;
fixtures and assertions are unchanged. Test/production builds emit no warnings, formatting reports
794 unchanged files, and leased scratch retires.

R172 retires: the fixed inventory now has zero unfinished migration entries and 78 accepted exact
technical edges. Frozen inventories, schema and generated contracts are unchanged. The roadmap
continues the broader dependency audit, test-value review and real-library verification; this count
closes only the fixed migration roster.

The reconciliation owner/path plan have 158/99 source lines and 1,004 dedicated-test lines, with no
inline cases. Native identity lookup has 70 source lines; its 420-line journal facade removes only
the old export and has no inline cases. Local preview sources have 173 source lines, retaining all
existing source/publication bodies.

## Journal protocol provenance and native-root checks — 2026-11-04

Against `7a5ca2fe`, three production application references still read the broker's wire-version
constant outside the frozen migration inventory. `JournalBoundaryProbe` now carries protocol
provenance for supported and LiveOnly results; `JournalRuntimeConnection` supplies it for unavailable
connections. Opening, closing and LiveOnly publication consume those values through ports. Native
adapters retain version 5, acquisition/error handling and resource retirement; schema, publication
authority and source access are unchanged.

The dependency gate classifies crate-root `journal_broker` and `windows_usn` as native adapters in
both directions, including aliases and re-exports. Existing parser cases cover admitted domain/port
access and rejected native/application/bridge dependencies. Its first repository run also exposed a
false positive in the test-only scan fixture: the visitor filtered conditional blocks but not
conditional `if` expressions. The corrected visitor excludes `cfg(test)` branches while retaining
branches that can compile for a production platform. No inventory or exception is added.

All 60 selected existing checks pass: 36 boundary, baseline, first-import and admission cases, plus
24 dependency checks. Existing application assertions use protocol version 7 to verify incoming
provenance independently of the native constant; the native probe retains its version assertion.
Cancellation, stale publication and atomic baseline cases remain intact. Product test count stays
2,984 and gate test count stays 24. Test and production compilation emit no warnings; leased scratch
retires after both the initial gate failure and corrected run.

The baseline owner has 352 production and 224 inline-test lines; opening has 185 production and 472
dedicated-test lines. The reference walker and rule classifier have 256/73 verification-source lines,
with 356 dedicated parser-fixture lines. The fixed inventory remains at zero migration entries and
78 exact technical edges.

A Windows semantic-index diagnostic identifies further inferred calls; source inspection confirms
two scan-recovery reads and two classification queries still call SQLite inherent methods without
port contracts. Its missing-definition diagnostics leave complete graph coverage unproven. These
calls, root-facade/cycle coverage, test consolidation and final real-library verification remain in
O7; the retained-import pause timeout keeps its existing acceptance owner.

## Foreground recovery projection boundary — 2026-11-04

Against `20934072`, idle recovery callbacks now borrow `ForegroundScanRecovery` instead of a concrete
SQLite catalog. The two previously inferred native calls resolve through that port. Their existing
convergence and query bodies move together to `sqlite_catalog/scan_lifecycle/recovery.rs`; the
transaction, paused/newest-import precedence, publication preservation and post-commit pending-row
retirement are unchanged. The catalog-session host retains its complete idle/current-session checks,
refresh handling, lock lifetime and once-only callback body. No new reader, session or heap allocation
is introduced. The now-unused `catalog-idle-recovery` registration retires.

All eight selected existing checks pass: two crash-recovery cases, shared-session exclusion, stale
session protection, detached first-import recovery, two deferred-retirement cases and the repository
inventory. Source comparison preserves all three moved bodies and the complete admission host body;
test changes only import the port, with fixtures, assertions and the 2,984-case count unchanged.
Test and production builds emit no warnings, and leased scratch retires. The fixed inventory remains
at zero migration entries, with 77 retained technical edges and unchanged frozen baselines.

The recovery implementation/port have 79/10 source lines and no inline tests; existing interrupted-scan
coverage has 549 dedicated lines. The catalog facade falls to 3,167 source lines with 11,606 primary
dedicated-test lines; the session host has 373 source and 352 dedicated-test lines. Neither has inline
cases. Classification query contracts, complete dependency audit, test consolidation and final
real-library verification remain in the current roadmap queue.

## Root facades and native entrypoints — 2026-11-04

Against `90253158`, the dependency loader now reads all three Cargo binary targets, including
feature-gated targets, and resolves their library aliases separately from their own crate paths.
Root facades follow application rules; the classification worker follows native adapter rules.
Existing rejecting fixtures cover a resolved application/native cycle, root-level construction,
binary re-exports and a feature-gated binary outside the conventional `src/bin` directory.

The unused `WindowsLibraryChangeObserver` wrapper, application re-export and synchronous stop
convenience method retire. Repository-wide references identify no production caller of the wrapper;
the real synchronization owner retains its existing `request_stop`/`finish_stop_until` lifecycle.
The three classification constructors still consumed by repository tools retain their public names
and receive exact technical registrations. Migration entries stay at zero; retained edges rise from
77 to 80 without changing either frozen inventory.

All 27 selected existing checks pass: 24 dependency checks and three observer/facade cases. The latter
retain bounded shutdown, idempotent retirement and portable planning assertions; no test is added.
The initial gate run exposed visibility-scope paths being treated as dependencies. The visitor now
skips visibility syntax while still rejecting a whole-crate re-export from ports. Removal also
exposed three unused-item warnings; their obsolete owners retire, and final compilation is warning-free.
The final gate takes 23.96 seconds, the three behavior cases 0.50 seconds, and leased scratch retires.

The loader, resolver, reference walker and rule classifier have 240/282/261/82 verification-source
lines, with 444 parser-fixture lines. The observer has 564 production lines, 23 inline test-support
lines and 804 dedicated-test lines; its production lifecycle bodies are unchanged. Inferred-call
classification, the complete cycle audit, test consolidation and real-library verification remain
in the roadmap queue, including the retained-import pause timeout.
