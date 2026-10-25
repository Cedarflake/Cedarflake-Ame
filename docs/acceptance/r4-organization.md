# R4 organization verification

The [roadmap](../roadmap.md) owns delivery status and the
[execution plan](../plans/r4-organization.md) owns slice exit conditions. Evidence below covers
only the named boundary; it does not establish completion of the R4 desktop workflow.

## Pre-upgrade backup foundation - 2026-10-24

The production catalog-session initializer now creates a verified online backup before advancing
an existing schema. Fresh and current catalogs skip the copy. The backup includes committed WAL
data and is retained independently of migration success. It neither enumerates nor opens media.
Only the existing rusqlite backup feature was promoted from test to production use.

Focused diagnostic command, executed serially under the repository tool lock:

```text
cargo test --locked --manifest-path rust/Cargo.toml --lib migration_backup:: -- --test-threads=1
```

All six cases passed. They cover recovery of committed WAL decisions after the live catalog
changes, no backup for fresh/current/future schemas, a retry preserving an earlier snapshot,
copy-deadline failure without schema advancement or completion receipt, destination failure
preserving source decisions, and an actual schema-45 catalog upgrading while retaining its
preferred-copy/ignore choices in both the live catalog and pre-upgrade snapshot.

The snapshot recovery test copies the completed backup to a separate fixture path and opens that
copy read-only. It verifies the old schema and decision values independently of the modified live
database. Fixtures are isolated derived storage; no real library or original catalog was used.

For recovery, retire every application process using the catalog, preserve the current catalog
and sidecars, validate the backup's exact completion receipt, schema and SQLite integrity, then
open a copy in separate derived storage with a compatible application version. Reconcile decisions
created since the backup before an explicitly authorized replacement. A missing/partial receipt
or a failed integrity check is not a recovery candidate. No automatic live-catalog replacement is
implemented by this foundation.

The new backup owner has 138 production lines and no inline test cases; its dedicated test module
has 151 lines and six cases. The existing 3,502-line catalog facade gains only the module declaration
and initialization call; it contains no inline test cases. Its existing test support and dedicated
test modules are unchanged. This boundary adds no Flutter or public bridge behavior.

The [hosted run for `5fc95d5`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40010275570)
completed with failures in Schema and Queue and Flutter Tests. The former expected a migration-row
error for an already-corrupt database; the new backup integrity check rejects it earlier. Its
updated regression preserves the raw-data and unchanged-schema assertions and checks the rejecting
owner according to the fixture's integrity result. Flutter's empty-shell tests still expected the
old app title and top-bar color; their assertions now follow the existing accepted desktop surface.
Other requested jobs in that run passed; that run does not verify the later organization changes.

## Durable group commands and schema 47 - 2026-10-24

Schema 47 adds organization records, flat album definitions, content-lineage bindings, source
associations, conflicts, operation receipts and field-specific undo evidence. These tables do not
cascade from derived assets or locations. The forward migration preserves R3 choices and the
catalog revision, creates no handled-image records, and validates both its schema and durable
relationships. Existing recovery compatibility and historical fixture downgrades include version 47.

The typed preparation command accepts at most 256 selected locations and resolves current groups
across roots in one database snapshot. Repeated members converge to one target; the current global
representative honors an applicable R3 preference. Membership authority covers location, root,
generation, path and identity/revision evidence. Pending retained display evidence cannot authorize
a partial group write. Member rows are streamed or held in SQLite temporary storage, with no full
membership list sent to Flutter and no media reads.

The first write is independent favorite intent. It revalidates the proposal in an interactive write
transaction and commits bindings, favorite changes, field revisions and the operation receipt
together. An exact retry returns its committed receipt; changing a payload under the same operation
identifier fails. Favoriting leaves album/progress and R3 duplicate-ignore choices intact. Failure
rolls back the entire batch and permits retry with the original proposal.

Focused serial diagnostics passed 14 organization/migration cases, the corrupt historical-catalog
reopen case, and three recovery compatibility cases. Coverage includes cross-root and unfolded
selection, stale content/membership/representatives, same-path generation replacement, pending
display evidence, reopen persistence, exact retry, a failed multi-group save, malformed migration
objects and invalid durable references/revisions. Tests use isolated derived catalogs and no media.

The six upgrade-backup cases also passed against schema 47, including a true version-45 fixture
without organization tables. The five existing desktop-shell widget cases passed after aligning
the title and top-surface assertions with the current production design; no UI production code
changed in that repair.

The canonical bridge generator completed and its Release build passed. Rust and Dart advertise
the same bridge content hash, `767745427`. No organization controls are exposed by this internal
foundation; album transitions, source lifecycle reconciliation, queries, continuation and the real
desktop workflow remain with their later slices.

The organization adapter has 44 production lines plus its test-module declaration. Its selection,
authority, favorite and binding owners contain 238, 85, 87 and 108 production lines respectively,
with 384 dedicated test lines and no inline cases. The migration owner has 99 production lines,
14 test-helper lines and a test-module declaration; its schema contains 85 lines and its dedicated
tests contain 148 lines. The 3,503-line catalog facade gains only module registration and the schema
version. The existing migration facade has 6,387 lines: 3,253 before its historical fixture helper,
844 helper lines and 2,290 inline-test lines. It gains only registration and migration calls.
Existing dedicated scan/change tests contain 5,521 and 3,804 lines; each changes only its expected
current schema version.

The [hosted run for `975f6a9`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40045644971)
passed 12 workload jobs and failed Schema and Queue and Rust Remaining. The failures were the
source-topology contract missing the new domain module, a current-schema assertion retaining 46,
and a historical upgrade snapshot not accounting for the new empty organization tables and initial
state row. The repairs retain exact module declarations, rejecting alternate loaders, full retained
row comparison and terminal-spool behavior. The subsequent implementation needs its own hosted run.

## Album transitions and undo foundation - 2026-10-24

Schema 48 adds direct lineage-to-group mappings, retired album metadata, prior album/conflict state
and complete operation-target receipts. The forward migration retains existing records and field
revisions, backfills self mappings and undo targets, and rejects incomplete or cyclic relationships.
Deleting a record retains only the opaque undo-target reference needed to reject a partial undo.

Album create, rename, retirement and whole-group assignment share revision-checked operation
receipts. Assignment commits organized state; deliberate removal and album retirement commit ignored
state. Defer and restore-to-pending use the same transaction owner. Favorites retain their independent
revision. Album reads use 64-entry keyset pages with a coherent total and organization revision.

Undo validates every affected field and current compatible content binding before restoring its
previous value. Later favorites survive organization undo. Later organization decisions, replaced
content, removed targets, retired destinations and conflicting album names reject restoration.
Album retirement can be undone atomically with its assignments; undoing album creation cannot
discard later members. Exact retries return the original receipt and mismatched payloads fail.

Focused serial diagnostics passed 25 organization cases, including six undo cases, followed by four
schema-48 migration and integrity cases. These cover cross-root whole-group assignment, all four
states, album paging, failures and rollback, independent favorites, name collisions, retirement,
reopen/retry and missing-target rejection. Fixtures contain derived catalogs only; no real-library
media or original catalog was opened. Group evolution, source lifecycle, query integration and the
desktop workflow are still unverified and remain required by the execution plan.

The nine source-availability contract cases, terminal-spool regression and historical schema-33
upgrade/reopen case also passed after the CI repairs. The upgrade comparison explicitly requires
every added organization table to be empty and the state row to start at zero while comparing all
retained tables unchanged. The module contract includes the public organization declaration and
rejects missing, duplicate, conditional, inline and redirected alternatives.

The adapter facade has 144 production lines and a two-line test-module declaration. Its album,
transition, receipt, undo and undo-album owners have 227, 91, 100, 143 and 71 production lines, with
no inline tests. Dedicated transition/undo modules have 263 and 233 lines. The schema-48 owner has
128 production lines, 13 test-helper lines and a test-module declaration; its dedicated tests have
177 lines. The existing migration facade has 3,256 lines before test helpers, 844 helper lines and
2,290 inline-test lines; its production change is registration and migration dispatch only.
The source-availability facade has 2,629 lines before its test module and 3,344 inline-test lines;
only its exact test contract changes. Dedicated topology, historical-upgrade and spool-retirement
tests have 422, 229 and 314 lines respectively.

Canonical bridge generation and the accompanying Release library build passed (38.62 seconds for
the Rust Release build). Rust and Dart both advertise content hash `-529801428`. The generated
changes are limited to organization types/endpoints and the corresponding bridge dispatch code.
The new implementation's hosted checks remain pending publication; no desktop interaction has
been accepted by these internal diagnostics.

## Explicit source removal and parent consolidation - 2026-10-24

Source removal now retires organization ownership inside the existing catalog removal transaction.
Groups owned by another configured source retain their decisions; orphaned groups lose their
lineage records, bindings, conflicts and field history. Opaque operation-target receipts prevent
old undo from restoring a partially removed selection. Empty album definitions remain available.
Losing derived locations alone does not grant this deletion authority.

Complete parent import transfers the covered children's organization ownership before retiring
their registrations. The publication transaction owns both changes, so cancellation and later
write failures preserve the previous child ownership. The new lifecycle owner performs only
catalog SQL and evaluates surviving ownership once per affected group.

Five adapter cases and two application scan cases passed serially. They cover shared groups,
orphaned aliases, remove/reopen/reimport, derived-location loss, failed removal, actual parent import
and failed child retirement. The application fixtures create isolated images and compare their
original bytes after each scan. No retained library or user catalog was used. Later fingerprint
association and group evolution remain separate required boundaries before R4 controls are exposed.

The lifecycle owner has 92 production lines and no inline tests; its dedicated tests have 206 lines.
The 174-line root-removal owner and 203-line parent-consolidation owner gain only composition calls.
The 491-line application scan test owner registers a 121-line dedicated organization test module.
This change adds no schema or public bridge contract. Hosted checks still need to cover its commit.

The [hosted run for `6a276ba`](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40048106213)
subsequently passed all applicable jobs, including the repaired historical fixtures and module
contracts. This covers the foundation, album transitions, undo and source lifecycle at that commit.

## Group reconciliation and content-version association - 2026-10-24

Schema 49 normalizes existing organization bindings once and adds generation and root/generation
indexes for bounded publication reads. Compatible evidence merges lineage mappings into one
canonical state. Agreeing albums converge; differing albums retain their decision provenance in a
deferred conflict with no confirmed membership. Favorite intent remains independent. Explicit
resolution and guarded undo preserve one whole-group state and the prior conflict choices.

Batch and terminal evidence publication commit these changes with the existing catalog revision.
Catalog delta and complete scan publication reassociate reused current evidence in their original
transactions. The first handled member binds already verified copies even when those copies were
identified earlier. Subsequent copies use point lookups instead of rewriting prior lineages.
Changed content cannot inherit an obsolete binding; a match to another handled group adopts its
state. Retiring an album involved in a conflict does not silently resolve that conflict.

Focused serial diagnostics cover 57 passing cases: 41 shared organization/schema-47 cases, four
schema-49 migration and index cases, four schema-48 cases, one actual catalog-delta rename/content
change case, two parent-import cases, three recovery compatibility cases, the schema-33 retained-row
upgrade and the terminal-spool regression. Relevant shared tests were repeated after the final
query-order and first-member association changes. The older schema-48 unit fixtures now validate
their explicit intermediate version before testing the remaining upgrade chain.

The 50,000-lineage fixture initially used at least 200,100 SQLite VM instructions to publish one
new copy. The same test now passes its fewer-than-20,000-instruction bound after bounded temporary
membership sets became the outer side of history lookups. This diagnoses database work, not gallery
latency or retained-library throughput. The reverse-identification-order case also removes the
original source and verifies that the remaining copy retains its album.

All new cases use derived fixture catalogs; parent-import fixtures retain their byte-preservation
assertions. No retained library, original catalog or cloud placeholder was accessed. New migration
failures roll back the version, indexes and decisions together. No dependency or bridge signature
changed. The latest commit still requires hosted checks; real R4 controls, query-scale measurements,
continuation and portable recovery remain with the following slices.

The binding, reconciliation, merge-decision, publication and album-membership owners have 210, 137,
104, 105 and 50 production lines, respectively, with no inline tests. Dedicated reconciliation and
publication tests have 319 and 119 lines; the catalog-delta organization test has 92. The schema-49
owner has 93 production lines, a 14-line test helper and a two-line test declaration, with 147
dedicated test lines. Album, transition, favorite and undo owners contain 215, 100, 59 and 149 lines.
The 245-line scan-publication owner, 113-line delta-location owner and 78/127-line evidence owners
gain only composition calls. The 3,503-line catalog facade changes its schema version; the migration
facade has 3,259 lines before its historical helpers, 844 helper lines and 2,290 inline-test lines,
and gains only module registration and migration dispatch.

## Historical upgrade assertion correction - 2026-10-24

The hosted run for `c9f7e24` found two historical version-17 upgrade fixtures that still expected
schema 48. Both now retain the freshly created catalog's current version before constructing the
historical fixture, and require the upgrade to return to that version. Placeholder preservation,
normalized location identity, namespace authority and source-state assertions remain unchanged.
The foreground-scan and incremental-update cases both pass their focused serial diagnostics.

The same hosted run separately reports 67,731,456 catalog bytes in the 10,000-image scan workload,
above its existing 64-MiB limit. That storage regression remains under investigation; these assertion
corrections do not establish a passing hosted run or change its resource bound.

## Bounded collection queries - 2026-10-24

The typed gallery query now includes Library, Favorites or one album. Query hashes, Dart equality,
bridge mapping, pagination, counts, layout and viewer anchors retain that same collection. Album
queries fold complete groups even when the ordinary gallery preference is unfolded; Favorites
retain that preference. Source/folder/search filters constrain the visible representative. Removing
a selected physical source clears its source scope while retaining the collection.

Canonical decisions supply membership; compatible version bindings and published/retained display
fingerprints supply association. Pending display evidence keeps the prior card without admitting
a write. Conflicts remove confirmed album membership, while favorites remain independent. Changed
content leaves the obsolete collection. Derived-fingerprint cleanup retains valid membership and
the preferred representative. Empty active albums return empty results; missing and retired albums
return an explicit unavailable-scope error in the read snapshot.

Twenty-three focused Rust checks pass: nine collection adapter cases, four query identity cases,
eight existing R3 gallery cases and two filtered location-anchor cases. The collection cases cover
forward/backward pages, counts/layout/anchors, scoped preferences, favorite independence, conflicts,
pending evidence, content replacement, cache cleanup, empty/deleted albums and an album move between
page and count reads. The 50,000-lineage history fixture stays below 20,000 SQLite VM instructions
for a one-item pending collection snapshot; this is a history-lookup bound, not retained-library
throughput evidence.

Sixty-three focused Flutter checks pass: two collection model/bridge cases, 56 controller cases and
five viewer navigation lifecycle cases. The changed cases verify supersession by album/Favorite
queries, collection retention after source removal and rejection of closed-viewer completions.
These use controlled catalogs and widget fixtures; actual R4 desktop controls are still absent.

The canonical generator produced the typed bridge. Its first formatting step hit Windows error
1224; after that process exited, formatting succeeded and the matching final Rust Release library
built in 40.50 seconds. Owned Rust/Dart formatting and diff checks pass. No dependency or schema
change is introduced by this slice, and no original library, user catalog or placeholder was read.
Current-head hosted verification and retained-library query measurements remain outstanding.

The collection SQL owner has 162 production lines and no inline tests, with 302 query-test and
219 evolution-test lines. Query normalization/identity moved from the catalog application facade
into a 61-line owner with 96 inline-test lines; the facade now has 239 production and 518 inline-test
lines. The 502-line gallery builder and 3,504-line catalog facade receive composition calls only.
The Dart collection owner has 42 lines; the 528-line model, 563-line catalog mapper and 1,292-line
viewport controller gain the typed field/mapping and collection-preserving removal projection.
Dedicated changed Dart test owners have 74, 3,655 and 253 lines, with no production test code.
The 803-line Rust domain type file gains the collection type, query field and folding policy.

## Organization index storage correction - 2026-10-24

Schema 49 added a generation-first organization index alongside the existing generation-first
exact-source index. Schema 50 removes that redundant index in a forward transaction while retaining
the root/scan/generation index used by bounded publication windows. It changes neither catalog
revisions nor organization decisions. Existing catalogs retain their pre-upgrade backup; freed
index pages become available for reuse under the existing storage-reclamation policy. No full
database vacuum or original-media read is added.

Seventy focused checks pass: 66 organization, collection, reconciliation, source-lifecycle and
migration cases, three recovery-contract compatibility cases and the version-27 terminal-spool
upgrade regression. The five new migration cases cover retained data and revisions, rollback after
index removal, malformed historical schema, required current indexes and indexed point/keyset reads.
The existing 50,000-lineage publication and collection cases retain their fewer-than-20,000-VM-step
bounds. Rust formatting and whitespace checks pass.

The canonical `performance_run_synthetic.ps1 -Case scan` diagnostic passes against the product code
in `d22ba79`: 10,000 images, 65,785,856 catalog bytes (62.7 MiB) and 37,109,760 resumed-catalog bytes,
both below the unchanged 64-MiB limit. Cold, warm and resumed scans take 53,450, 44,483 and 48,655 ms;
pause and cancellation take 48 and 209 ms. Row-count and source-preservation assertions pass.
The monitored test process peaks at 67,018,752 working-set bytes and completes in 209,049 ms.
The preceding Release test build takes 899,470 ms and is excluded from those scan timings.
These local timings are not a comparison with the previous hosted runner or retained-library
throughput evidence. The exact product commit's [hosted run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40053197189)
passes the 10,000-image workload and the other four synthetic workloads. Its Remaining Rust shard
finds a separate terminal-trigger test that still expects schema 49; the correction is recorded below.

The schema-50 owner has 52 production lines, a 12-line test helper and a two-line test declaration,
with 173 dedicated test lines. The schema-49 owner retains its historical DDL and exposes the two
index definitions for validation and test fixtures. The 3,504-line catalog facade changes only its
version constant. The 6,396-line migration facade gains module registration and dispatch; its
historical helpers and inline tests are unchanged. The current-schema validator selects the
version-specific index contract, and recovery admission retains explicit known-version bounds.

## Publication-order queue admissions - 2026-10-24

Schema 51 adds compact version admissions and configured-source ownership. The complete scan and
incremental catalog transactions allocate ordinals only for visible versions. Concurrent imports
therefore follow publication order even when their source generations were allocated in the reverse
order. Unchanged versions retain their ordinals without consuming sequence values. Content changes
receive new admissions; identity-proven renames retain the existing one.

Explicit source removal forgets unshared admission history while retaining shared versions. Parent
consolidation transfers historical ownership before covered registrations retire. Failed publication
or removal rolls these writes back with the catalog. Upgrade seeds a deterministic baseline from
already visible versions without changing catalog revisions or organization decisions; it cannot
recover historical publication times that were never recorded.

Twenty-four focused serial checks pass: six admission cases, three schema-51 cases, the actual
incremental rename/content-change case, the terminal-trigger regression, five existing organization
source-lifecycle cases, five schema-50 cases and three recovery-compatibility cases. Coverage includes
unpublished staging, reverse-order concurrent publication, injected failure and retry, unchanged
sequence reuse, shared-source removal, parent transfer, reopen, migration preservation and rollback,
and rejection of missing indexes or orphaned ownership. The one-version update remains below
20,000 SQLite VM instructions with 50,000 historical admissions. This bounds that history lookup;
it does not establish queue-opening or retained-library performance.

The first concurrent-import test queried rows before the staging buffer had been flushed. Its
fixture now flushes staged rows without publishing them and verifies both source generations before
reversing publication order. The terminal-trigger test now checks agreement between the opened
catalog's two version markers instead of pinning an unrelated historical schema number. Its source
preservation, rejected execution, retirement, cleanup and repeated-reopen assertions remain intact.

Rust formatting and diff checks pass. These diagnostics use isolated derived catalogs and controlled
source fixtures; no retained library, original catalog or cloud placeholder was accessed. There is
no dependency, bridge or UI change. Commit `6c67ac3` passes the unchanged hosted 10,000-image storage
workload, 50,000-identity publication workload, static checks and unsigned Release build in
[its own run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40054906425). The Remaining Rust
shard also passes, including the corrected terminal-trigger assertion. Its unfinished Flutter,
Catalog, Schema/Queue and Synchronization/Scan jobs were cancelled when the next branch checkpoint
started under the configured superseding-run policy; the complete run therefore is not passing.
These results do not cover the later queue queries or establish retained-library performance.

The admission owner has 160 production lines and no inline tests, with 266 dedicated test lines.
The migration owner has 106 production lines, a 14-line test helper and a two-line test declaration,
with 122 dedicated test lines. Existing source-lifecycle, scan-publication and delta-location owners
have 94, 244 and 117 production lines, respectively, with no inline test bodies; they gain only
composition calls. The delta regression has 111 lines and the terminal-spool test owner has 317.
The 3,504-line catalog facade changes only its version constant. The migration facade gains three
registration/dispatch lines; its 844-line historical helper section and 2,290-line inline test section
remain unchanged. The current-schema owner gains only version-selected validation calls.

## Organization queue queries and coherent progress - 2026-10-24

Pending, deferred and ignored selectors now share the existing gallery query boundary. Pages,
layout and logical asset anchors fold each current group and use its first surviving admission,
independently of the selected representative. New independent versions enter the front. Collection,
source, folder and search filters remain active, while ordinary browsing retains its own sorting
preferences. Queue pages have no chronological buckets.

The page, timeline and all four progress counts use one SQLite read transaction. A concurrent save
therefore cannot mix old entries with new counts. Only a bounded page and five scalar counts cross
the generated bridge. Existing client query generations reject superseded responses, including
their progress counts. Source-removal projection retains the queue selector after clearing the
retired physical scope. Display-only retained evidence continues to reject organization writes.

Thirty-seven focused Rust checks pass: 17 collection/queue cases, five query-identity cases, seven
ordinary page cases, seven ordinary anchor cases and one additional history-cost case. The queue
cases cover provisional groups, forward/backward pages, scoped representatives, new arrivals,
ordinary-mode decisions, independent Favorites, late conflicts, content splits, preferred-copy
replacement, cache cleanup, empty scopes and concurrent publication. With 50,000 unrelated admission
and decision records, both one-page pending and ignored snapshots remain below 20,000 SQLite VM
instructions and return the expected complete counts. This bounds unrelated-history lookup; it
does not measure a very large single group's lineage or retained-library queue-opening latency.

Seventy-one focused Flutter checks pass: three queue model/bridge cases, two retained collection
cases, 56 controller cases and ten coherent-snapshot reader cases. These verify typed counts,
scope retention, supersession and committed-source removal. The canonical generator and matching
Rust Release library build pass. An earlier test compilation found a fully specified historical
query fixture missing the new optional selector; the fixture now explicitly selects ordinary mode.

No dependency or schema change is introduced by this query slice. All diagnostics use controlled
derived catalogs; no retained library, original catalog or cloud placeholder was accessed. Desktop
state publication, real interaction and retained-library measurements remain outstanding. Durable
continuation is recorded in the next section. Commit `395e395` has a
[hosted run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40056219158): static, Flutter,
Windows Scan, accessibility, unsigned Release and three synthetic jobs pass at this checkpoint.
Remaining Rust reports 886 passing tests and two failures because the exact source-module guard
does not yet register the new queue module. Other shards and synthetic jobs are still running.
The continuation change adds exact declarations for both queue and review domain modules; all nine
focused availability checks pass, retaining rejection of alternate paths, duplicate declarations,
generated modules and source enumeration. The correction requires hosted coverage on its own head.

The queue SQL owner has 106 production lines, the extracted ordering owner 75 and the collection
owner 177, with no inline tests. Dedicated queue/history tests have 325 and 81 lines. The gallery
builder shrinks to 444 lines; its snapshot facade adds transaction composition only. Query identity
has 74 production lines and 124 inline-test lines. The 815-line Rust domain type file gains the
optional selector and folding/order policy, while the new queue domain owner has 27 lines.
The Dart queue owner has 23 lines; the 540-line model, 587-line catalog mapper and 1,295-line viewport
controller receive typed fields, mapping and scope-preserving composition. Changed dedicated Dart
test owners have 89, 516 and 3,670 lines. Generated bridge output is excluded from handwritten counts.

## Durable organization continuation - 2026-10-24

Schema 52 adds a singleton active review session, a separate durable bookmark and operation receipts.
Entering mode establishes save authority without creating activity or replacing prior work. A
position checkpoint records the current lineage, retained scope and admission boundary. Ordinary
album or Favorite changes leave this bookmark intact. Continuation follows the current group and
representative, or finds the next eligible item after the retained boundary; new arrivals ahead of
it do not replace the saved work.

Session identity and monotonic position sequences reject old checkpoints. A current-item action
commits its organization decision, after-current bookmark and receipt atomically. Retrying a committed
operation returns its original receipt without rewinding newer work. Unassociated anchors remain
recorded and can supply another eligible target. No prior activity, unavailable scope, unavailable
anchor, empty scope and exhausted queue suffix have distinct typed results. Source availability
still belongs to the existing viewer checks; catalog association does not assert that a file opens.

Explicit source removal clears its scoped review data and orphaned anchors while preserving a
globally scoped group with another surviving source. Parent consolidation transfers the physical
scope before retiring the child. Upgrade creates no activity, preserves decisions and revisions,
and uses the existing pre-upgrade backup boundary. No dependency or source-media mutation is added.

Twenty-seven focused Rust checks pass: 13 continuation cases, three schema-52 migration cases,
three existing admission-migration cases, three recovery-compatibility cases and five organization
source-lifecycle cases. They cover reopen, ordinary work, new arrivals, old/closed session responses,
out-of-order positions, atomic failure/retry, stale group commands, late grouping, preferred-copy
changes, derived-evidence cleanup, independent changed content, unavailable anchors, empty versus
exhausted queues, scoped rejection, identity-preserving location changes, shared-source removal and
parent consolidation. Migration checks retain rollback, missing-schema and orphaned-record rejection.

The parent-consolidation fixture initially changed the relative path without its corresponding
parent-folder field. After matching the production row contract, the unchanged target and scope
assertions pass. This checks the production organization-transfer boundary with controlled catalog
rows; it is not a complete desktop parent-import acceptance run. The additional nine source-read
guard checks described above pass, giving 36 focused checks for this checkpoint.

The canonical bridge generator succeeds with matching Rust/Dart content hash `-719090103`; its
optimized Rust library build finishes in 112 seconds. Final Rust formatting and whitespace checks
pass. SQL and row construction are expanded for reviewability without changing their decisions.
Diagnostics use isolated derived catalogs and controlled fixtures, without accessing a retained
library, original catalog or placeholder. Hosted verification must cover the committed implementation.
Actual desktop session lifetime, source-open failure, save-to-viewer advancement and restart flows
remain R4-05 work. Retained-library latency and large-group lineage costs remain unmeasured.

The review adapter has 99 production lines; scope, position, operation and continuation owners have
183, 142, 105 and 72 lines, respectively. The gallery candidate reader has 102 production lines.
None contains inline tests. Dedicated review and lifecycle tests have 418 and 102 lines. The
migration owner has 113 production lines, a 14-line test helper and a two-line test declaration,
with 102 dedicated test lines. The catalog facade changes only its version constant; the migration
facade gains registration/dispatch calls and retains its historical helpers and 2,290-line inline
test section. The source-read guard change adds 12 exact-contract lines inside the existing
3,356-line inline test module; its 2,629-line production/support section is unchanged. The dedicated
topology test owner has 436 lines. Generated bridge output is excluded from handwritten counts.

## Viewer Favorites and shared feedback - 2026-10-24

The viewer reads the current whole-group Favorite state through a bounded inspection API. Inspection
does not prepare a command, materialize records or read original media. Four focused Rust tests cover
read-only inspection, repeated-member normalization, cross-root decisions after reopen, pending
publication, conflict state, independent changed content and invalid/stale input. One conflict fixture
initially republished an already written fingerprint; publishing its retained generation batch fixes
the fixture while preserving its conflict assertions. Generated Rust/Dart content hashes agree at
`-389473781`, and the matching Rust Release library builds successfully.

The Favorite action retains the committed state while saving, admits one write at a time and keeps
the original operation identity after an unknown receipt. Confirmed stale selections are prepared
again. A failed gallery refresh does not replay a successful write. Undo uses the original receipt
and current revision, with separate handling for decisions that can no longer be undone. The
collection entry uses the existing sidebar query path, and current viewer content survives removal
from Favorites.

Results reuse `LibraryViewerNotice` at the top of the image viewport. Source notices and original-image
failures retain priority; no additional bottom viewer surface remains. The notification timer is
extracted into `AmeNotificationLifetime` without changing its five-second or persistent behavior.
Internal error details are excluded from the product notice; diagnostic output retains the error code.
The notice has no added close button. A bounded outcome owner holds the latest successful receipt
without adding notification history or unread state; old undo callbacks and dismissals cannot affect
a newer outcome. Save failures retain the existing actionable notification path.

Sixty-two focused Flutter checks cover seven operation lifecycle cases, three Favorite button cases,
five collection/query-navigation cases, seven existing notification cases, two viewer-result cases,
eight source-image cases, 25 viewer-session cases and five viewer-navigation lifecycle widget cases.
Coverage includes lost receipts, stale preparation, refresh-only retry, undo, disposal, late image
responses, top placement, automatic dismissal, source-notice priority and continued browsing after
query changes. The initial collection fixture used an obsolete folder-field name; it now asserts the
actual `folderRelativePath` contract. Scoped static analysis and formatting pass after correcting the
new code's flow-control and nullable-element style findings.

The added controls compose Material `IconButton` and `ListTile`, existing `AmeTooltip`, sidebar spacing
and viewer notice tokens. Their toggle, focus and selected-state APIs were checked in the installed
Flutter 3.44.9 source and the official [IconButton](https://api.flutter.dev/flutter/material/IconButton-class.html)
and [ListTile](https://api.flutter.dev/flutter/material/ListTile-class.html) references. No framework,
dependency, source-write capability or schema migration is added by this slice.

The inspection owner has 148 production lines and 120 dedicated test lines. The Dart service, action
lifecycle, request and transient-outcome owners have 151, 207, 80 and 42 lines. The existing viewer session and bounded
navigation reader have 354 and 117 lines, with 594 dedicated session-test lines. The notification
surface and shared lifetime have 273 and 60 production lines. These production owners have no inline
tests. The 1,638-line screen and 446-line navigation widget receive composition only; storage and
command policy stay in their dedicated owners. Generated bridge output is excluded from these counts.

The isolated Windows client imported the existing 700-file fixture corpus and converged to six exact
groups. A Favorite saved during group publication rejected stale authority, then succeeded through
retry. The Favorites scope showed one group representing 210 physical images. Unfavorite removed the
group from that scope while retaining the displayed image. These observations preceded the final
top-notice, transient-history and viewer-position corrections. The Debug build for the initial
top-notice correction succeeds; the final native undo, restart and presentation recheck remain deferred
under the current no-window-operation constraint. The test process remains open, so disposable profile
cleanup is also pending. SHA-256, file size, modification time and file count match for all 700 source
fixtures; no retained-library catalog or original media was used.

Hosted run [40057826622](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40057826622)
for `4eddf115bab727fe766b43f2e0076f96b0601e03` passed the other applicable jobs but failed Schema and
Queue. Its v33-to-current preservation fixture omitted seven newly introduced empty tables and the
new admission sequence initialized to zero. The corrected fixture asserts those exact additions and
still compares every retained row and prior sequence. All four foreground-queue-index migration tests
pass locally, including conflicting-object rollback and invalid-authority rejection. This result does
not establish hosted verification for the current changes. The follow-up run
[40060729462](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40060729462) for
`c1059eb513ec9c6a6bdf25264bf102e28f8398c2` completed successfully, including Catalog, Schema and Queue,
Flutter, static checks, unsigned packaging, synchronization/scan, native accessibility and the
applicable synthetic workloads.

## Album management, viewer membership and settings - 2026-10-24

The secondary sidebar now opens album scopes and provides create, rename and delete through the
existing Material menu/dialog controls. The viewer uses the same single-destination picker for
assignment, movement and removal. It can create an album in that picker, keeps the selected
destination across pages, waits for the committed receipt before closing and displays failures
inline. Ordinary-mode commands retain the current image and use the existing whole-group
transaction; deleting a selected album returns the gallery to the library.

Prompt/default preferences use the existing presentation store. Prompting starts enabled with no
default; disabled prompting uses a valid saved destination. Missing defaults and conflicts still
require an explicit choice. Settings cancellation changes nothing, and a failed preference write
restores the previous value. Neither Favorites nor physical folders are album destinations.

Navigation and pickers retain one 64-album page. A new indexed, read-only album lookup resolves a
saved default beyond that first page without fetching earlier pages. Its focused Rust test passes
for a 65-album fixture, including missing/retired IDs and unchanged operation count. The generated
Rust/Dart bridge hashes agree at `835197144`. Code generation completed; its trailing format command
initially encountered Windows file-mapping error 1224. Retrying that remaining formatting step
succeeded, and the matching optimized Rust library built in 108 seconds. No schema or dependency
change is introduced.

Thirty focused Flutter checks pass: five album-command cases, three destination-proposal cases,
four dialog cases, three viewer-action cases, one composed gallery/navigation case, two settings
widget cases, five shared-preference cases and seven existing organization-action cases. They cover
unknown receipts and same-operation retry, stale authority, empty-album undo, cancelled preparation,
default behavior, old viewer proposals, create-in-picker, pagination, input errors, removal,
preference rollback and unchanged notification history/unread state. The composed gallery check
initially used a nonexistent collection constructor, then exposed a missing scan-test dependency;
using the actual library scope and existing no-op scanner isolates the intended path. Its rename,
delete, return-to-library and transient undo assertions pass.

Metadata-only receipts can undo without inventing a changed-group count. Outside the viewer, the
existing gallery notification surface displays the transient receipt; the viewer retains its
shared top notice. Form failures do not enter global notification history. The ordinary notification
and task-surface priorities remain unchanged. The new controls were checked against the pinned
Flutter 3.44.9 `RadioGroup`/`RadioListTile` API and reuse the accepted settings, menus, typography,
spacing and scrolling owners.

The Dart service and command facade have 295 and 219 production lines. Album request sessions,
proposals and page providers have 151, 42 and 21 lines. Editor, picker, viewer action, navigation and
settings controls have 150, 259, 127, 188 and 137 production lines, with 833 dedicated album-test
lines. Scoped analysis and the repository format check pass. These production owners contain no
inline tests. The 1,694-line screen receives composition
and collection-title changes. The existing Rust album adapter has 239 production lines and its
inspection test owner has 155 lines, including the new 35-line lookup case. Generated bridge files
are excluded from handwritten counts.

These checks use isolated catalogs or service fixtures and do not enumerate, decode or mutate
original media. The user's no-window-operation instruction remains in force: no real client was
opened, restarted or operated for this slice. Actual album controls, keyboard/focus, restart,
retained-library latency and low-core measurements remain unverified. Gallery bulk selection,
organization mode, involved-album conflict inspection and recovery settings remain subsequent
R4 work. Hosted verification must cover the published album implementation commit.

The album checkpoint `09b2cdcc7c7ce2b924b6a51c61ff7c4d5d0c4231` has a
[hosted run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40062430535). Flutter reports
one failure in the existing import-feedback test: its global `LinearProgressIndicator` finder now
matches both the import progress and the album-list loading indicator. The repair scopes the finder
to the existing import task surface while retaining the exact validated/total ratio assertion.
All seven focused import-feedback checks pass after the repair. Product progress behavior is
unchanged. The correction is committed as `d2bdd3173fd66f54c74bfd2ea66c57c8a56900e0`.
The album run completed with all other applicable jobs passing; the aggregate gate reflects the
Flutter failure. Follow-up hosted verification remains required for the combined correction.

## Query-wide organization selection boundary - 2026-10-24

The batch application boundary accepts explicit selections or all matching gallery results with
exclusions. IDs arrive in chunks of at most 256; the existing gallery predicates retain collection,
folder, search and duplicate-display scope. A private SQLite connection stores selected IDs,
locations and deduplicated whole-group targets in temporary tables. Its temporary page cache is
configured for 2 MiB with file-backed overflow. Two concurrent drafts are permitted; no transaction
remains open while a destination choice waits. Preparation returns scalar counts and the common
album, without sending the full group/member graph to Flutter.

The adapter reads targets in 64-group windows, retaining catalog/organization revisions and current
representative/membership authority. Commit revalidates these before applying the existing per-group
Favorite or organization transition under one receipt and one transaction. Cancellation and late
failures roll back all changes. Receipt retry precedes new revision admission, and reused operation
IDs cannot accept different actions. Undo uses the existing field-specific history. The new opaque
bridge exposes preparation, progress, application and close; closing signals cancellation before
waiting for an active operation and releases its temporary connection and capacity permit.

The existing 83 organization/migration regressions pass after extracting shared group writes.
All ten new adapter cases pass, covering 600 explicit groups across input chunks,
300 exclusions, mixed assignments and Favorites, late-merge conflicts, write rollback and retry,
unpublished content changes, intervening decisions, cancellation after a real record write,
whole-batch undo and preserved Favorites. Folded/unfolded exclusions and restricted folder selections
preserve whole-group authority. The application check proves that close cancels held preparation
before waiting, retires the connection once and returns its capacity permit. Twelve source-availability
checks pass, including exact module-loading rejection and retained offline/rename recovery. Together
these checkpoints cover 106 focused Rust cases. Initial diagnostics found a new receipt-kind value
rejected by the existing schema; the batch now uses the established Favorite/organization kinds without a migration.
The folder fixture was corrected to the existing relative-folder contract.

Canonical bridge generation succeeds with Rust/Dart content hash `-688938423`; its matching optimized
Rust library builds in 117 seconds. The repository format check covers 495 Dart files without changes,
and the final whitespace check passes. No dependency or persistent schema changes are introduced.
These controlled catalog fixtures do not open or alter original media. The 600-group case establishes
coverage beyond one input chunk, not retained-library performance or desktop acceptance.

The batch adapter has 328 production/support lines and a 14-line test-only cancellation hook;
temporary staging has 199 production lines, and the gallery selection builder has 28. The application
session has 127 production lines plus its two-line test declaration; the port, domain and API owners
have 65, 36 and 46 production lines, respectively. Shared organization/Favorite write owners have
120 and 83 lines with no inline tests. Dedicated batch adapter and application tests have 503 and
92 lines. Large existing domain, catalog and gallery facades have 817, 3,505 and 446 lines and receive
only registration/reexport changes. The source-read guard adds six exact-contract lines in its
3,362-line inline test section; its 2,629-line production/support section is unchanged. Its dedicated
topology tests have 443 lines. Generated bridge output is excluded from handwritten counts.

Gallery selection buttons and picker lifetimes have not yet been connected to this boundary.
Real client interaction, retained-library save latency, resource/cancellation measurements and
low-core behavior remain outstanding, along with the rest of R4-05 through R4-07. Window operation
remains paused by user instruction. Hosted verification must cover the published combined head.

The published batch foundation `97b9ff5a6e0d9ea1a9e5bcadb0dd02cc0136bcf7` completed its
[hosted run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40064409008) with two substantive
failures: Clippy requires a collapsed conditional in batch admission, and the settings fixture's
global dropdown-icon count includes the new default-album control. The import-feedback correction
passes, and all other applicable jobs pass. The settings repair retains the exact five ordinary
choice controls and separately checks the album destination arrow; all fifteen focused settings
tests pass. The batch conditional is corrected with the connected selection implementation.
The combined corrected product head still requires hosted verification.

## Connected gallery selection - 2026-10-24

The gallery selection toolbar now connects album assignment/movement/removal and explicit Favorite/
unfavorite actions to the query-wide batch boundary. All matching results retain their exclusions;
only bounded ID chunks cross the bridge, without expanding the visible card window into a full
membership list. Mixed selections resolve to whole groups. Album removal preserves already
unassigned progress and does not create records for untouched pending groups. Conflicting selections
require an album choice. The existing default-destination policy, picker, inline errors and transient
receipt/undo surfaces are shared with the viewer. Successful Favorite/unfavorite/undo results leave
notification history and unread state unchanged.

A typed selection session owns preparation, retirement and unknown-result retry. Once the album
picker opens it owns the batch, so creating a destination can refresh the gallery and retire its
launching toolbar without cancelling the pending assignment. Ready-batch refresh revalidates its
frozen groups and decision versions in one snapshot; it never expands the original query or rebases
an uncertain write. Version evidence is read with the existing group inspection rather than through
additional per-group lookups. A changed group or intervening decision requires reselection.

The shared picker permits cancellation only for commands that support it, waits for retirement,
and still publishes a successful receipt when commit wins cancellation. Selection intent survives
ordinary revision rebinding but changes with user gestures, so a late command cannot clear a newer
selection. The installed Flutter 3.44.9 `PopScope` implementation confirms the blocked-pop callback;
the controls reuse existing Material text buttons, Ame menus and album dialogs. No new responsive
layout, overlay design or dependency is introduced.

The final organization/migration Rust filter passes all 98 cases, including fifteen batch cases and
five inspection cases. The new boundaries cover create-in-picker refresh, frozen all-matching scope,
intervening target decisions, changed content, retained unknown receipts, mixed removal and undo.
Fifty-three distinct focused Flutter checks pass across organization actions (7), selection sessions
(6), destination proposals (3), album dialogs (4), viewer album actions (3), selection actions (9),
selection intent (5), composed gallery selection (1), and settings (15). These include cancellation
before opening, during preparation/save, commit-versus-cancel, retry identity, conflict/default policy,
toolbar disposal, transient Favorites/undo and query-wide selection beyond the single loaded card.
Initial fixture failures required waiting for the preparation error listener and radio-selection
rebuild; the existing single-image save-cancellation assertion is retained. Scoped Dart analysis with
fatal infos, Clippy for all targets with warnings denied, and the 504-file format check pass. Generated
Rust/Dart content hashes agree at `1093802923`.

The 1,714-line screen and 528-line header receive composition changes. The selection widget has 275
production lines, the selection session 101, its bridge adapter 137, the shared action owner 237,
and the picker 302; none contains inline tests. Dedicated session, selection-widget, gallery
composition and selection-intent tests have 193, 363, 76 and 69 lines respectively. Rust preparation,
inspection and transition owners have 147, 176 and 129 production lines. The batch facade has 252
production/support lines and a 14-line test-only hook; its refresh and mutation tests have 123 and
279 dedicated lines. Generated bridge output is excluded from handwritten counts.

All checks use isolated catalogs or service fixtures. Original media, original catalogs and cloud
placeholder state were not opened or modified for this slice. No persistent schema changes are
introduced. Window operation remains paused: real pointer/keyboard/focus, restart, retained-library
latency, cancellation/resource measurements and two-core behavior remain unverified. Organization
mode, involved-album conflict inspection and R4 recovery settings remain in the active execution
queue. The published selection head requires its own hosted verification before acceptance.

## Shared gallery organization mode

The gallery overflow now enters and exits organization mode through the existing query transition.
The original count row shows total, organized and pending groups from one matching query/revision
snapshot. The bottom dock reserves content space for pending, deferred and ignored queues, with
an independent, same-height resume button only when prior activity has a resumable target. The
existing Material segmented control is also used elsewhere in the application; its installed
Flutter 3.44.9 implementation supplies selection, disabled state, focus and semantics. No mobile
layout, new dependency or replacement notification design is introduced.

The viewer reuses its album picker and top notice. Assignment, deferral, ignore and restore commands
save the current decision and continuation atomically through the existing native review API;
advancement waits for the committed receipt. Favorites remain independent. Selection commands add
the same progress actions using the existing bounded batch owner. Exiting restores the saved ordinary
query, selection and visible anchor subject to existing query authority. A representative replacement
admitted by the viewer renews the review target without duplicate checkpoints. Background updates
coalesce continuation reads and do not move the displayed image.

Sixty-seven distinct focused Flutter checks pass: review sessions (7), mode lifecycle (8), review
widgets (7), composed gallery review (1), count authority (1), query transitions (17), ordinary
viewer album actions (3), selection actions (9), ordinary gallery counts (4), and coherent snapshot
reads (10). These cover inert first entry, saved-scope restoration, unknown-result retry identity,
newer checkpoints versus late commits, close-during-read, commit-only advancement, album picker
saves, defer/ignore/restore, query-wide deferral, unchanged ordinary counts, restored viewport
fractions, and dock/resume alignment at 1x and 2x text scaling. The final mode and composed-gallery
checks were repeated after representative adoption changed. Scoped Dart analysis with fatal infos
passes after correcting a missing conditional block in that final change; owned Dart files are
formatted. These are headless interaction and state checks, not native desktop acceptance.

The 1,840-line screen and 575-line header compose the new owners. The 1,300-line viewport controller
only forwards the coherent count snapshot; its publication lifecycle is unchanged. The state model
has 406 lines, and the query/revision count guard belongs to its dedicated value type. The mode owner
has 297 production lines, review adapter 189, session 99, request owner 86, contextual controls 161,
and dock 96, with no inline tests. Dedicated session, mode, widget and composed-gallery tests have
185, 220, 242 and 119 lines respectively. Session authority, presentation navigation and native
translation remain separate responsibilities under ADR 0025.

Hosted run [40066452840](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40066452840) for
selection commit `369fe6dd8c5077fdd4d2b048eca947d47855182a` reports a Flutter failure while the run is
still in progress. Its cancelled-picker assertion expected rejection after the request was retired.
Commit `6dd398c10e76a38968da3446311913f54f63f2ff` corrects that assertion, retains the no-write check,
and checks empty notification history and unread state. All five focused album command tests pass.
This does not establish hosted verification for either that correction or the current mode slice.

The mode slice changes no schema, Rust implementation, generated bridge or media adapter. The
existing atomic continuation and source-lifecycle evidence remains applicable. Its tests use service
fixtures and isolated catalogs; no original media, original catalog or cloud placeholder was opened
or modified. Window operation remains paused. Native keyboard/focus, top-notice consistency,
restart, retained-library latency/resource bounds and two-core behavior remain pending, as do
persistent involved-album conflict discovery and the later recovery settings. R4 remains active
under the canonical roadmap.

## Controlled desktop continuation and queue layout - 2026-10-24

Desktop interaction was explicitly resumed after the earlier pause. The Debug client at
`0fc9199dec10cba9039e3626fbcfd0ca8037cc26` built in 81.1 seconds and used the retained isolated
`build/test/derived/r4-favorites` profile with 700 fixture files representing six exact groups.
Favorite and unfavorite displayed the shared viewer notice with undo and no close icon. Returning
to notification history after both operations showed no entries or unread marker.

The actual gallery overflow opened organization mode with six pending groups and no resume button
on first use. Deferring one group committed pending/deferred counts of five/one and advanced to the
next image. Creating album `r4` inside the shared picker and assigning that next group committed one
organized group and advanced again. Favoriting the following group retained the current image and
pending count. Ignoring it committed one ignored group and advanced to `04973-640x480-0497.jpg`.

After closing and restarting only this isolated client, the Favorite collection retained its
210-copy group and album `r4` retained its 23-copy group. Entering organization mode from that album
showed its empty pending scope; resume restored the saved global scope and the exact previously
viewed image. The ignored group retained its Favorite state. Restoring it to pending closed the
exhausted viewer, displayed an empty ignored queue, and changed the coherent counts to four pending,
one deferred, zero ignored and one organized. The exhausted continuation no longer offered resume.
Exiting mode restored the original ordinary `r4` album scope, its date heading and equal-height layout.

Native inspection found an unwanted unknown-capture-date heading in organization queues. Both
loaded and manifest layout builders still derived grouping from the retained ordinary sort key.
A typed grouping policy now uses the query's chronological eligibility, participates in snapshot
reuse/replacement, and supplies the virtual extent calculation. All three queues form continuous
rows independently of asset or manifest dates. Ordinary date grouping, row gaps, admission ordering
and source-read behavior remain unchanged. The corrected Debug client built in 30.1 seconds; native
equal-height and square layouts both omit the date heading and retain the bottom queue controls.

Nineteen focused Flutter checks pass across layout snapshots (14), virtual geometry (4), and the
composed review screen (1). The six queue/layout combinations cover mixed known/unknown dates,
loaded item order, continuous packing and rejection of a previously grouped snapshot. The first
new assertion incorrectly treated the existing trailing row gap as a header; the final assertion
checks headers specifically and retains that gap. Scoped Dart analysis with fatal infos and owned
format checks pass. There is no schema, Rust, generated bridge or dependency change.

The grouping policy has 12 production lines, loaded layout 431, snapshot layout 462 and virtual
geometry 203. The 1,532-line gallery wall receives only 12 composition lines passing the policy;
its lifecycle ownership is unchanged. These owners contain no inline tests. Dedicated snapshot,
geometry and composed-screen tests have 347, 130 and 120 lines respectively.

All 700 fixture names, sizes, UTC modification ticks and SHA-256 hashes match the retained pre-run
baseline. The first comparison accidentally wrapped the PowerShell JSON array as one item; the
corrected comparison reads its 700 entries and reports zero differences. Original libraries,
original catalogs and placeholders were outside this client profile. The client remains open on
the isolated ordinary album. Native undo is still unverified: the two earlier interaction attempts
reached the transient control after it expired, so they establish neither a pass nor an application
failure. Other outstanding native paths, persistent conflict resolution and retained-scale evidence
remain under the roadmap.

Hosted run [40068207427](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40068207427)
covers `0fc9199dec10cba9039e3626fbcfd0ca8037cc26` and is still running at this checkpoint. The queue
layout correction requires hosted verification for its own published commit. These controlled
results do not close R4 or establish retained-library performance.

## Persistent album conflict discovery and resolution - 2026-10-24

A separate read-only conflict port exposes involved-album pages of 64, a scalar current-group count,
and visible annotations in batches of at most 256 locations. The SQLite adapter reuses current
content/group inspection and the gallery display-binding projection. Retired album names remain
inspectable without becoming valid destinations. Reads neither create organization records nor
open media. The generated Rust/Dart bridge agrees on content hash `1194935527`; this slice changes
no schema, migration or dependency.

One serialized status reader coalesces catalog publications. The shared notification opens the
global deferred queue only on explicit action, preserving the ordinary browsing context for exit.
Per-image labels and the viewer action retain conflict visibility after the notice disappears.
The existing picker presents the involved albums and uses the existing whole-group assignment,
atomic review continuation and undo commands. Cancel and failed saves retain the conflict; the
default album cannot bypass explicit conflict resolution.

Four focused Rust regressions pass, covering late-group reconciliation, reopening, retired album
provenance, resolution/undo, 64+1 paging, annotation limits, stale versions, content invalidation,
unpublished evidence and read-only inspection. Thirty-one distinct focused Flutter checks pass:
conflict workflow (7), ordinary picker/editor (4), ordinary viewer album action (3), mode lifecycle
(9), review widgets (7), and composed review screen (1). They cover notification coalescing and
dismissal, serialized stale-result admission, failed-read recovery, cancellation, failed-save retry
identity, global deferred entry and ordinary-context restoration. The final picker change reran
the 22 affected picker/review checks; the nine mode lifecycle checks retain their valid result.

The first dialog attempt enabled outer scrolling around shrink-wrapped lazy viewports and failed
Flutter intrinsic-size layout assertions. Constraining only the destination list removed those
assertions but a later 800x560 / 2x text interaction exposed an unreachable destination row. The
final dialog has a scrollable body with bounded page contents, shared scroll motion and fixed
framework actions. Both 1x and 2x checks now page forward/back, select a destination and cancel
without hit-test misses or overflow. The installed Flutter 3.44.9 `AlertDialog`, `RadioGroup` and
`SingleChildScrollView` retain layout, selection, focus and scrolling ownership.

Scoped Dart analysis, Rust formatting and Clippy across all targets/features with warnings denied
pass. The native bridge build completed, and the first Debug client build took 65.7 seconds.
The final layout's scoped analysis passes and its Debug client rebuild took 30.1 seconds, using
the same installed SDK and isolated client profile.

The SQLite conflict reader has 158 production lines and 256 dedicated test lines. Dart conflict
translation/window batching has 167 production lines, serialized status reading 53, notification
ownership 35, provenance display 101 and the label 41. The existing picker has 317 lines and viewer
album action 155. The 1,854-line screen receives 14 composition lines, the 464-line photo tile four,
and the 309-line mode owner a 12-line navigation entry. None of these owners contains inline tests;
the dedicated conflict workflow and mode suites have 365 and 240 lines. Generated bridge output is
excluded from handwritten ownership counts.

The native client used the existing 700-file/six-group test library. A closed-client SQLite backup
was retained before placing the already favorited 210-copy group in a controlled two-album conflict
between `r4` and `R4 conflict`. This is a desktop UI fixture; the Rust regressions separately prove
production late-group reconciliation. Startup displayed one shared warning without navigating.
After the notice expired, the gallery label and notification-history action remained available.
That action opened the global deferred queue with two groups, including the marked conflict.

Opening the marked image showed its conflict action and retained favorite. The picker displayed
both involved albums. Cancel returned to the same image with its conflict still present. Assigning
the group committed organized/deferred/pending counts of 2/1/3, removed the conflict and returned
from the exhausted viewer to the queue. Exiting restored ordinary browsing. After restarting only
the isolated client, `R4 conflict` retained 210 physical copies folded into one group, with its
Favorite still set and no conflict label or renewed warning.
The final rebuilt client then moved that group from ordinary global browsing into `r4`, retained
the current image and favorite, and reopened the picker with the saved destination selected.
Escape cancelled the picker and returned to the same preview. This final layout check leaves the
test group in `r4`; the prior restart result describes the preceding resolved-conflict state.

All 700 fixture names, sizes, UTC modification ticks and SHA-256 hashes match the retained baseline;
the original library, original catalog and cloud placeholders were not touched. Scoped client logs
contain no Flutter exception, overflow or organization-save failure. The transient undo control
expired before the attempted native clicks, so native undo remains unverified; adapter and headless
undo evidence remains valid. Other native management/settings, batch and failure paths and retained
library/two-core measurements remain in the roadmap. The previous queue-layout head's hosted run
`40069517946` was still running when inspected; this slice requires its own published-head checks.

## Whole-query album assignment and preparation handoff - 2026-10-24

The controlled client selected all six logical groups across the 700-file query through the gallery
menu. Cancelling the shared picker retained all six selections. Reopening it and choosing
`R4 conflict` saved all groups, cleared selection after commit and showed 700 physical copies folded
into six album entries. Favorites remained the original 210-copy group. After restarting the
isolated client, the batch picker recognized `R4 conflict` as the common saved destination.

Native inspection found that the toolbar continued to say `正在准备` while the ready picker was
waiting for a destination. The selection owner now rebuilds when it transfers ownership and hides
its preparation feedback while the dialog owns the operation. It retains the busy action guard,
dialog cancellation and commit lifetime. The existing boundary regression holds preparation,
observes its status, completes preparation and verifies that the status is gone before choosing a
destination. All nine selection-action checks, scoped analysis and owned format checks pass.

The rebuilt Debug client took 29.9 seconds. Its actual batch picker leaves no preparation text in
the background toolbar, and Escape restores the six-item selection with enabled actions. The owner
has 304 production lines, no inline tests and 371 dedicated test lines. The change adds no schema,
bridge, dependency or new state flag. All 700 fixture filenames, sizes, UTC modification ticks and
SHA-256 hashes still match the retained baseline; scoped client logs contain no exception, overflow
or save failure.

Run [40069517946](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40069517946) completed
successfully for queue-layout commit `ad052b81b22fd664459aec5005d75e9c7f1f0698`.
Conflict run [40071330712](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40071330712)
for `60bddbcba35eb1b2c20e28c7039ec9cfc531f83a` is still in progress at this checkpoint, without a
reported failure. This handoff correction needs checks for its own published head. Native transient
undo remains unverified: the control expired between tool observations, and direct accessibility
clicks rejected their returned indexes. Ctrl+A produced no observed selection in this client; its
cause has not been isolated. Menu-driven whole-query selection is verified.

## Native undo, album retirement and settings - 2026-10-24

The existing isolated 700-file client completed batch Favorite and unfavorite: Favorites changed
from six groups to zero, with an empty notification history. Setting the default album to r4
and disabling 加入相册前询问 moved the ordinary viewer's current group directly into that album,
without opening a picker or changing the image. Both preferences survived a process restart.
Clearing the default then opened the picker even with asking disabled; cancelling preserved the
current image and membership. Native rename changed r4 to r5. All 700 filenames, sizes, UTC
modification ticks and SHA-256 hashes match the retained baseline.

The controlled Windows integration now invokes the production organization adapter through the
gallery, viewer, picker and album editor. It checks Favorite undo, assignment undo, removal-to-ignore
and undo, empty-name validation, concurrent-revision rejection with successful retry, rename undo,
delete cancellation and deletion undo. Each result is read back through Rust, and ordinary viewer
operations retain the selected asset. The existing scan fixture's source-byte and directory-entry
checks remain after the added workflow. No organization service or query adapter is mocked.

The first run, 8486a0aa3c0c482a9b650dc1d8ce7b23, failed after deleting the currently selected album.
The committed command refreshed the retired album before the navigation callback ran, producing
an unavailable-query warning whose Retry action displaced Undo. The new publication owner receives
the committed retirement, serially retires affected active and saved ordinary scopes, and only then
refreshes the catalog. Failed publication retains its obligation for retry; a detached or replaced
projection cannot complete another view's work. Organization mode preserves a different current
scope and forgets a deleted saved scope before returning to ordinary browsing.

The repaired canonical Windows run, 61cfa0c649234af38689d6d3166ef7f6, passed all four integration
cases, including the complete organization workflow and following source-management checks.
Its owned runner completed in 94.534 seconds, with a 36.9-second Debug build and no cleanup failure.
The expected stale-write rejection remained visible in the form; retry committed the new name.
Deleting the selected album now returns to the library, and Undo restores the same album identity
and organized membership without an unavailable-query warning.

Native Flutter keyboard dispatch also verifies gallery Ctrl+A/Escape and text-field Ctrl+A without
selecting gallery items behind the editor. Computer Use's earlier Ctrl+A and text insertion produced
no visible result; these observations are retained separately from the passing application-dispatch
checks and do not establish a physical-keyboard fault. The new native checks reach transient Undo
while it is visible, resolving the earlier tool-timing verification gap.

Twenty-nine focused Flutter checks pass across organization actions, publication, mode transitions
and album navigation. This includes overlapping commits, failed retirement/read retry, replacement
projection identity, disposal, newer navigation and saved-scope retirement. Scoped analysis and
owned formatting pass. The publication owner has 130 production lines and 181 dedicated test lines;
the action facade has 210 production lines, the mode owner 339 with 361 dedicated test lines, and the
screen 1,853 with only registration composition changed. These owners contain no inline tests.
The native workflow has 288 test lines. There is no schema, bridge, dependency or source-media change.

The previous conflict run 40071330712 was cancelled. Run
[40071947604](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40071947604), for
9cd4c7b078d97c9f156ba5dacfc01dd0dcd57180, has two failures in Rust Remaining because the exact
availability module inventory lacks domain::organization_conflicts; the other inspected completed
jobs passed. That inventory repair and this publication change require their published-head checks.
Retained-library measurements, unavailable-item integration and R4 recovery/cleanup remain outstanding.

## Conflict module source-contract admission - 2026-10-24

Run 40071947604 completed with two failing source-contract assertions in Rust Remaining; the final
gate failed and release builds were skipped. The exact domain inventory omitted the new
`organization_conflicts` declaration. Its module contains only four typed data records and no
filesystem operations. The inventory now admits its existing public, attribute-free, non-inline
declaration exactly once. The topology fixture also applies all ten rejected loading mutations to
it, including duplicate declarations, alternate paths, conditional loading and generated bodies.
No production availability behavior or safety assertion is weakened.

All nine focused `root_availability_` checks pass, including both failed assertions and the adjacent
admission, loading-environment and metadata-only contracts. The suite took 30.75 seconds after a
90-second compilation; owned rustfmt checks and diff checks pass. The changed local-files owner
has 2,629 lines before its inline test module and 3,368 inline test lines; this change adds only six
inventory lines within that test module. Its dedicated topology fixture has 450 test lines.
The published repair and album-retirement change require current-head hosted verification.

## Unavailable-image organization continuity - 2026-10-24

The controlled Windows scan workflow now enters organization mode with two pending groups, blocks
one original through a read-only exclusive native handle and verifies the existing preview fallback.
Moving to the next image decodes that original; returning retains the blocked image and its issue.
Both groups remain pending with unchanged album and Favorite state. Releasing the handle and using
the visible Retry action decodes the same original. Leaving the viewer still shows two pending
groups, and exiting organization mode returns to ordinary browsing.

The fixture uses [CreateFileW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-createfilew)
with `GENERIC_READ`, `OPEN_EXISTING` and no sharing, followed by
[CloseHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-closehandle).
It admits only canonical paths in the owned native fixture directories, rejects reparse aliases and
an out-of-corpus README path, and releases the handle idempotently in `finally`. It performs no source
write or deletion and introduces no dependency. The existing source-byte and directory-entry checks
remain after the organization and source-management workflows.

Initial runs 2280cf0891fa43b590336b7cb99e667f and 84c4e59c5db540e6872522204b8523d9 found fixture
problems: extended Windows paths were rejected, then the PowerShell helper did not receive its
release signal from Dart. The latter reproduced in a standalone Dart process, while the same helper
released normally with Python. Explicit flushing and a single-byte protocol did not resolve that
transport failure. The final fixture owns the native handle in process; the obsolete helper was
removed. Its isolated probe confirmed denied reads while held and unchanged readable bytes after
release, including a second idempotent release through the native workflow.

Final run 28e4a4d08a8940e59acc57c41a7006e2 passed all four integration cases, including
`AME_ORGANIZATION_UNAVAILABLE_WORKFLOW_PASSED` and `AME_ORGANIZATION_NATIVE_WORKFLOW_PASSED`.
The runner completed in 97.288 seconds with a 37.5-second Debug build, exit code zero and no cleanup
failure. Scoped analysis with fatal infos, owned formatting and diff checks pass. The unavailable
workflow has 95 test lines and its source-handle fixture has 106; the existing 148-line source-viewer
helper only exposes its original-decoding assertion for reuse. No production behavior, schema or
bridge is changed by this checkpoint.

The inventory repair is published as 5cc6525a8f14315e9cebe86262d920a9cb19e00f. Its run
[40074028181](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40074028181) completed
successfully. The subsequent native-check commit cb2fb80528bba6127be43ebd9189f8b9955ada35 is covered by
[40075785832](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40075785832). It completed
successfully with all 15 applicable jobs passing before restoration-admission publication;
retained-library measurements and R4 export, restoration and cleanup remain outstanding.

## Portable organization archive export boundary

The [archive decision](../architecture/0031-portable-organization-archives.md) introduces a versioned
JSON Lines codec and a catalog-snapshot exporter behind a narrow repository port and typed bridge.
Twenty explicit record families cover durable R3/R4 intent, committed continuation, history and
association evidence. Local integer admission keys are translated to root identifiers; active
review authority, source media and previews are excluded. Each encoded line is limited to 1 MiB,
and parsing never buffers the complete archive. The completion footer verifies counts and BLAKE3.

The production boundary admits one export at a time, excludes source and application-storage
destinations, copies committed WAL state in bounded SQLite backup steps, then streams the immutable
snapshot into an exclusively owned output. Completed output is flushed and validated before
no-clobber publication. Failure and cancellation release scratch ownership; current catalog writes
can continue while archive records are serialized. Format validation alone is not restore admission.

The first focused run passed 12 tests in 11.56 seconds after an 88-second test build. It covers all
record families and Unicode, an empty catalog, every truncated prefix, unsupported/unknown fields,
tampering, count mismatches, oversized input, read/write failures, phase cancellation, existing and
concurrently created destinations, source/storage admission and concurrent committed decisions.
The populated fixture's source root is unavailable, proving export does not need original-file reads.
The canonical bridge generator completed its Rust Release build in 58.27 seconds. These fixture
timings do not establish retained-library export throughput.

After adding exclusive export admission, both application-boundary checks passed in 0.03 seconds
and all nine existing root-availability checks passed in 28.71 seconds. This brings the focused
archive coverage to 13 distinct tests while retaining the first run's unchanged codec/export
results. Owned formatting and diff checks pass. Hosted verification remains required for the
published implementation commit.

The new catalog facade has 163 production lines, its codec 239, record projection 146 and snapshot
owner 66. Dedicated export and codec tests have 197 and 173 lines. The application owner has 98
production lines and 46 lines for its two inline boundary tests. Existing catalog and application
facades only declare the new responsibility; no migration or source-reading workflow is extended.

Settings integration, restoration/merge policy, unresolved-record cleanup and large-library
temporary-storage/resource measurements remain outstanding. No R4-06 user workflow is accepted by
this backend checkpoint; previous native UI evidence is retained without replay.

## Staged restoration admission

The inspection API now streams a backup into an exclusively owned staging database, checks framing
and completion, validates record relationships, and returns canonical-group/active-album counts and
the verified archive identity/checksum. No archived rows are applied to the current catalog. The
input is held read-only against concurrent writers on Windows, with no-recall opening and resident
attribute admission. A shared operation lease bounds export and inspection concurrency.

The focused `organization_archive` run passed all 21 tests in 21.95 seconds after a 64-second test
build. The canonical bridge generator's Release build passed in 49.96 seconds. The unchanged export
checks ran with the modified parser; the new cases cover alias counts, Unicode and SQL-like text,
missing references, absent canonical mappings, group cycles, incomplete undo pairs, retired-album
assignment, conflicting states, invalid booleans, duplicate keys, revision ceilings and missing
bookmark scope. Malformed relational fixtures have recomputed valid counts and checksums, so their
rejection establishes a separate boundary from byte-integrity checks. Legitimate deleted undo
targets remain accepted and retained.

Cancellation during 512-record staging preserves both input bytes and the live catalog and removes
owned scratch. Windows tests prove an input read lease denies concurrent writes and that an existing
writer prevents inspection until released. Attribute-level regressions reject offline/recall states
while admitting resident reparse files; no cloud-placeholder fixture was hydrated. Storage-full and
constraint errors retain distinct failure categories. Inspection does not establish whole-library
recovery throughput or real-client settings behavior.

The staging owner has 131 production lines, fixed stage schema 131 and relational validation 75.
Input admission has 51 production lines and 26 inline-test lines; dedicated staging tests have 284
lines. The codec now has 252 production lines and the catalog facade 178, with separate dedicated
tests. Typed API/application owners remain narrow; the current catalog schema is unchanged. Owned
formatting and diff checks pass. Hosted checks are required for the new implementation head.

Atomic restoration, foreign-identity reassociation, retry/conflict policy, settings and unresolved
cleanup remain outstanding. Archive-local generations and ordinals cannot enter live bindings or
queue authority before trustworthy mapping; unmatched evidence must remain durable through that
later publication boundary. The roadmap advances to R4-06 while retaining R4-07's integrated
workflow, retained-library resource and source-preservation obligations.

## Archive association preview

Inspection now compares validated archive evidence with a single read-only catalog snapshot and
returns matched/unmatched canonical-group counts, resolved/unresolved preferred-copy counts, and
the inspected catalog/organization revisions. Source locations, historical bindings and preferred
choices use 256-record windows. An indexed, disk-backed projection uses bounded per-database page
caches; repeated native versions are aggregated before matching. No archive generation, filename
or asset identifier alone authorizes a match, and no source-media read or mutation is introduced.

The focused `organization_archive` run passed all 30 tests in 52.78 seconds after a 40.85-second test
build. Nine added regressions cover rebuilt/renamed native versions, changed revision/volume/size,
missing or unsupported evidence, published compatible fingerprints across volumes, contrary current
fingerprints, canonical alias counts, physical preference ambiguity, cancellation and concurrent
publication. A controlled 600-binding/600-copy case remains below its 150,000 SQLite-instruction
ceiling while rejecting contrary content. This checks lookup growth, not real-library throughput.
Cancellation after a 256-location window removes owned staging, and a later inspection succeeds.
Archive bytes and live catalog changes are checked independently of the temporary matching tables.

Review added the reverse hardlink case: several archived aliases followed by one surviving current
alias still cannot identify the old preferred location. After that guard and its staging lookup
index were added, all ten matching regressions passed in 34.04 seconds after a 35.95-second test
build. The unchanged 21 codec/export/admission results are retained, giving applicable passing
evidence for 31 distinct archive checks. The Rust library Clippy check passes with warnings denied,
including the previously reported callback declaration. The final Release build passes in 48.44
seconds; generated bridge output, owned formatting and staged whitespace checks are current.

The first focused run exposed a fixture deletion-order error: published fingerprint references were
still present when its evidence was replaced. Correcting the fixture's dependent-row order retains
foreign-key enforcement and the original negative cases. A preceding build also identified unsigned
SQLite conversions; the new readers now use the existing checked conversion helper.

Hosted run [40077809287](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40077809287), for
`4bcf19102b6a75ff8a557528374313214c1a6e2d`, finished with the Static job and aggregate gate failing:
`clippy::type_complexity` in the archive record
consumer callback. Its declaration now uses a generic callback without changing record validation
or suppressing the lint. The other jobs from that run retain their own outcomes; the new implementation
requires checks on its own published commit.

Hosted run [40079620605](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40079620605)
completed successfully for `d2d6865a97d3d8f4b0438f48adb070fc24deebe4`, covering the corrected
declaration and association-preview implementation. Signing-only jobs remained outside this run.

The association owner has 77 production lines, current-evidence capture 75 and matching 93; none has
inline tests. Dedicated matching and cost tests have 312 and 80 lines. The catalog archive facade has
182 production lines and still delegates each responsibility. The catalog schema and archive format
remain unchanged. Actual application must recompute matches under its write transaction and verify
the inspected checksum. Atomic restore, durable unmatched records, newer-decision/conflict handling,
recovery settings and explicit cleanup remain outstanding, as do retained-library measurements.

## Native-version reassociation after derived rebuilds

Publication now recovers retained organization bindings through the accepted Windows native-file
identity, ChangeTime and byte-length evidence. It reuses the existing canonical-group reconciliation
and binding transaction. Contrary compatible fingerprints or conflicting historical content keys
leave native recovery unresolved; the ordinary exact-content path retains its authority. Schema 53
adds an indexed lookup over durable native versions, with no changes to original media, archive
format, existing records or catalog revisions. Existing recovery-shape validation explicitly includes
schema 53 while continuing to reject unknown future schemas.

Twelve focused native-recovery and migration tests passed in 26.82 seconds. They cover changed
derived identities/generations, retained albums, favorites and continuation, reopen and repeat
publication, unsupported/missing/changed evidence, contrary fingerprints, whole-transaction rollback
and explicit-source-removal precedence. The actual scan-publication adapter path also recovers
existing and newly imported aliases across roots, presents one album group, and preserves that group
after removing one owner. These fixtures exercise database publication without reading source media;
they do not establish the future settings restore workflow.

The alias-cost regression first failed at 24,380,300 SQLite instructions for 1,001 current aliases
and 1,001 historical bindings. Deduplicating native keys and canonical groups removes the repeated
pairwise comparison. The same workload now passes its unchanged one-million-instruction ceiling
both when attaching and when republishing already recovered aliases. The separate 50,000 unrelated
record case stays below 30,000 instructions. These are query-growth checks, not retained-library
throughput measurements.

The affected existing publication, reconciliation, source-lifecycle and upgrade-backup suites also
passed: 3, 8, 5 and 6 checks respectively. The three recovery-shape tests then passed in 7.06 seconds,
covering schema 53, unknown-version rejection and damaged-contract rejection. Together with the 12
new focused checks, that is 37 applicable passing checks. The first cross-root test build reported an incremental-cache hardlink
fallback; subsequent diagnostic compilation disabled incremental artifacts for that process and
completed without that warning. No compiler lint or test requirement was relaxed.

The dedicated association owner has 110 production lines and no inline tests. Existing record,
reconciliation and publication owners have 211, 137 and 110 production lines, with no inline tests;
the organization facade has 176 production lines and delegates this responsibility. The index
migration has 38 production lines and an 11-line test-only downgrade helper. Dedicated recovery,
cost, scan-publication and migration tests have 325, 108, 84 and 94 lines. The catalog/migration
facades add only the schema version and migration composition points. Full archive application,
durable unmatched archive import, settings, explicit cleanup and R4-07's retained workload remain
outstanding. Hosted verification must cover this implementation's own published commit.

## Atomic portable restoration and retained ordering

Schema 54 retains archive provenance, namespaced version/root evidence, per-field restored authority,
queue positions and unresolved R3 choices. The application and generated bridge expose one guarded
restore operation. Input is staged before writer admission; archive identity, checksum and both
catalog revisions are rechecked before changes. Decisions, receipts, history, evidence, queue and
bookmark changes commit together. Existing local choices win, identical requests return their
committed receipt, and a different explicit restore can recover cleaned records without giving old
undo receipts authority over the new values. Source media are neither opened nor changed.

Archived admissions retain their relative order; later independent arrivals stay at the front.
Association refreshes a saved logical group's position without replacing its scope or active session.
Unmatched evidence survives export and later publication. Explicit source removal retires its
unshared recovery authority before reimport. R3 choices are applied in the existing publication
transaction, including terminal publication without any R4 records. A preferred physical copy needs
unique compatible native evidence in both the archive and current catalog. Format 2 retains the
original native-copy count; version 1 conversion derives it only after verifying the original input.

The archive suite passed 49 checks in 237.93 seconds, followed by four migration checks in 9.41
seconds. Two earlier preferred-copy fixture failures used a fictional asset identifier; the fixtures
now obtain the published identifier. Following the final queue and staging-validation changes,
40 affected checks passed: nine ordinary queue reads, six restored queue/continuation cases, one
malformed-relationship matrix, six public restore transactions, four migrations, three publication
cases, three restored-authority cases and eight existing duplicate-gallery queries. These cover
reopen, lost-input replay, stale confirmation, changed payload, cancellation at every restore phase,
injected failure with retry, revision overflow, explicit reapplication and retained old-undo limits.

The first queue projection caused an ignored-queue read to execute at least 1,207,900 SQLite
instructions with 50,000 unrelated admissions. Replacing the union projection with indexed
current-version links fixes that regression. The final workload includes another 50,000 unrelated
recovery admissions and still passes the unchanged 20,000-instruction ceiling for both pending and
ignored reads. This establishes bounded lookup growth, not large-library restoration throughput.

The canonical bridge generator and its native Release build completed before the final queue and
staging-validation edits. The generated API includes restore request, progress and committed receipt.
The final source's focused Rust compilation passed with incremental artifacts disabled for that
process. The final format check reports 536 Dart files with no changes, and the bridge verifier
passes all 17 asynchronous contracts with matching content hashes. The first format invocation
completed formatting but failed to update the SDK telemetry timestamp inside the sandbox; the same
scoped check passed with that access admitted. No full local Daily or retained-library run is claimed.

The archive facade has 193 production lines plus a three-line test-module declaration; the restore
composition and transaction owner have 38 and 89 production lines with the same declarations.
Admission, ordering and bookmark owners have 136, 106 and 71 production lines. Recovery queue,
source ownership and R3 choice owners have 92, 121 and 84 production lines, without inline tests.
The schema declaration has 140 lines. Dedicated decision, transaction, queue, R3-choice and migration
tests have 277, 237, 326, 261 and 100 lines respectively. These owners keep receipt admission,
transaction lifetime, queue ordering, logical continuation and publication association distinct.

The previous published head `b124e52b9442c6bd9df677f0df934eeb04d2bc1f` failed
[hosted run 40082529906](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40082529906)
in the existing ordinary-journal process-exit/restart case: catalog convergence completed through
fallback after native attribute reads failed, but the required journal-current state did not.
The same unchanged case passed locally in 3.54 seconds. The native error code was absent from the
hosted log, and the cause remains unresolved; these restore changes do not claim to repair it.
Settings integration, inspectable unresolved records and explicit cleanup remain outstanding.
R4-07 still owns real desktop verification, retained-library throughput/memory and cancellation
latency; phase checkpoints alone do not establish cancellation latency for large SQL operations.

## Historical upgrades through archive-aware reconciliation

Commit `7333bf319cefc2845b024bd36e31006ec305e4f9` failed
[hosted run 40090754973](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40090754973):
55 checks across four Rust partitions reached shared organization reconciliation before schema 54
installed `organization_recovery_admissions` and `organization_restored_fields`. The failures include
old scan/catalog upgrades and schema 49's existing conflict normalization. They have the same missing
table cause. Static checks, Flutter tests, native scan/accessibility checks, unsigned Release and
the five synthetic workloads passed. The unchanged ordinary-journal restart case also passed in
this run; that observation does not identify the previous run's native-read failure cause.

The repair resolves restoration capabilities from the transaction's schema version. Earlier schemas
retain every existing decision's local authority and have no portable recovery history. Schema 54
still requires its actual provenance and recovery tables; missing current tables remain errors.
This preserves schema 49's merge/conflict rules without introducing archive objects early, accepting
an incomplete current catalog, or changing source media.

Twenty-seven focused checks pass: four reconciliation migrations, five restoration migrations,
ten media-admission checks, three version-17 upgrades, two prerelease repair paths and three restored
authority cases. Existing rollback, conflict provenance, Favorite and retained-location assertions
remain unchanged. A new current-schema corruption case proves a missing recovery table cannot enter
the legacy path. These checks use controlled temporary catalogs and fixtures; original libraries and
catalogs are untouched. Hosted verification remains required for the repair commit.

The schema capability owner has 27 production lines. Recovery dispatch and restored-field authority
have 62 and 82 production lines, with no inline tests. Dedicated restoration migration tests have
121 lines. The organization facade adds only module composition; the existing migration and
application regression owners retain their assertions.

## Primary Albums navigation

The function rail orders Library, Albums and Duplicates. Albums owns the existing flat album list;
Library retains Favorites and physical sources. Both reuse one gallery, viewer and sidebar transition.
Favorites is also accessible above the Albums list through the same collection entry. Five focused
query-navigation cases pass with that addition, including opening Favorites from Albums, retaining
the selected primary destination, and restoring each destination's scope on return. Both entries
share favorite state without changing album membership.
The new navigation owner retains two browse snapshots, serializes rapid destination requests and
replays failed navigation through the existing retry surface. Album retirement invalidates saved
and in-flight restoration. Conflict notices open the visible Library deferred queue.

Twelve focused Flutter cases pass across browse-workspace ownership, shell retention, retained
semantics, album management and organization-mode integration. They cover independent source/album
scope, selection and logical position; mode exit; failed-switch retry; latest-request ordering;
retirement during restoration; disposal; destination order; sidebar retention and compact-width
access; rename/delete/undo; and conflict-notice navigation from Albums. The native organization
workflow now enters Albums before managing albums. Real-window verification remains pending while
window operation is suspended; these widget checks do not establish that desktop result.

The existing NavigationRail, retained workspace/focus composition, AmeScrollMotion and gallery
message provide the controls; no dependency, schema or source-media change is introduced. The
installed framework capabilities and alternatives remain those admitted by ADR 0029. The new
navigation owner has 110 production lines and 142 dedicated test lines, with no inline tests.
The composed shell has 306 lines and the album sidebar/landing has 39. The existing unified screen
has 1,877 lines; it delegates navigation ownership rather than adding its state machine there.
The shell, album-screen and organization-screen regressions have 167, 142 and 151 lines respectively.

## Retained migration table inventory

The schema/queue partition of run `40093347873` on `da12c50a37ef078fb28b5407675db2893fb780dc`
passes 342 cases and fails the version-33 retained-row comparison. Comparing the complete logged
maps identifies ten additional empty schema-54 archive/recovery tables and no changed retained
table or row. The fixture's exact post-upgrade inventory now names and asserts those empty tables
before comparing every retained row. It still rejects an unexpected table, nonempty newly admitted
table or changed historical record.

The unchanged migration/reopen test passes locally for both preexisting-index states and both opens;
execution takes 8.88 seconds after compilation. This is an expected-schema fixture correction, with
no product, schema or migration change. The hosted run has completed: the other three Rust
partitions, Static, Flutter, native scan/accessibility, unsigned Release and all five synthetic
workloads pass. This inventory assertion is its only failing test; verification of the correction
and subsequent implementation head remains pending.

## Settings archive integration checkpoint

Settings connects native destination/input selection, export progress and cancellation, validated
inspection, restore confirmation, guarded application, retry and committed refresh. The persistent
controller keeps work across navigation. Cancellation waits for a terminal result; uncertain retries
reuse the operation identifier, while changed inspection evidence requires renewed confirmation.
The generated opaque task separates atomic cancellation from the worker's admission lease.

Seven controller tests and four settings widget tests pass, covering failed/missing/mismatched
results, preserved retry identity, stale confirmation, cancellation/commit ordering, disposal, picker
cancellation, restore confirmation and the scrollable dialog at 800 by 600 with enlarged text.
Three native task tests pass for cancellation before dispatch, single use, retained worker admission
and cancellation isolation. The generated Rust/Dart content hashes and 17 asynchronous bridge
contracts pass. The canonical generator also removes trailing whitespace from its generated Rust
output, using the same normalization already applied to generated Freezed files.

The archive-capable Windows Debug build completed before the primary Albums navigation changes.
The reused isolated client profile opened from schema 52 to 54 with catalog revision 40, organization
revision 28, two albums and six organized groups retained. A subsequent read-only check finds zero
restore receipts. All 700 controlled source files retain their SHA-256, length and modification time.
Native picker/export/restore interaction and restart verification remain pending while window
operation is suspended. Small-fixture checks do not discharge retained-library performance work.

The archive controller, typed gateway and settings section have 165, 130 and 200 production lines.
The native task owner has 59 production lines plus its three-line test-module declaration and
49 dedicated test lines. Controller and settings tests have 183 and 201 lines; these owners contain
no inline Dart tests. The settings page adds composition only, and application admission moves to
the task owner rather than adding lifecycle flags to the archive facade.

The hosted archive checkpoint `35ef95a` exposes two missing Dart flow-control blocks and a native
workflow setup error. Static analysis rejects the unbraced multiline guards. The native organization
workflow reaches its completion marker, then source management attempts to find a Library-only
source menu while the Albums destination remains selected. The correction adds the required blocks
and explicitly enters Library before the existing management assertions. It changes neither the
source update/removal assertions nor their deadlines. Native verification of this correction belongs
to the subsequent hosted head; local window interaction remains suspended.

Run `40096057273` has completed on `35ef95a780fd32180bd260d22efe8f70247ff3c6`. The schema/queue,
catalog and remaining Rust partitions, Flutter, Windows accessibility, unsigned x64 Release and
all five synthetic workloads pass. Static and Windows Scan fail at the two boundaries above.
Synchronization and Scan also reproduces the ordinary-journal import failure: its nested client
does not reach a continuous checkpoint and reports `persistent_journal_metadata_fallback`, with
pending live/recovery work. That partition records 416 passes, one failure and 12 ignored tests.
The separate protected release jobs are skipped. `ae53b81` corrects the static blocks and native
workflow navigation; the journal cause and verification of subsequent heads remain outstanding.

## Manual unresolved-record cleanup

Settings now connects age selection, inspection, affected counts, confirmation, application,
cancellation and retry through the typed cleanup port. It shares the existing organization-records
section and standard settings/dialog controls. The default 90-day filter, 30-day option and all-record
option remain manual choices. Cleanup preserves album definitions and original media. Committed
results refresh shared organization state; uncertain retries reuse their operation identifier and
stale inspections require a new confirmation.

The association-lifetime owner captures first loss in the location/root publication transaction,
retains it across repeated observation, and clears the whole lineage after compatible reassociation.
Three focused cases pass for first loss/restoration, a surviving copy with an inactive watcher, and
transaction rollback. Twelve cleanup and restoration cases cover age selection, canonical aliases,
retained R3 choices, recent shared-generation history, unpublished/inactive sources, cancelled
windows, injected write failure, repeat receipts, stale confirmation, explicit backup recovery and
no resurrection after reimport. The preferred-copy cases restore a real archive into a catalog with
ambiguous native aliases and matching published content: the unresolved choice remains manually
cleanable, while an already resolved choice is excluded.
The local `cleanup` test filter completes with 116 passes, including those 12 boundary cases, in
334.06 seconds after compilation; the additional matches are existing cleanup regressions.

Four controller and two settings widget cases pass for retained task lifetime, superseded
confirmation, uncertain retry identity, cancellation/commit ordering, changed records, missing
terminal results, cancelled confirmation, explicit confirmation and empty results. The widget
surface uses 800 by 600 with 1.4 text scaling. Canonical formatting, focused Dart analysis and the
generated content-hash/17 existing asynchronous bridge-contract checks passed. Those existing
Future-contract checks do not validate the new streaming cleanup API's native execution.

The native source-module inventory explicitly admits `organization_cleanup` with its exact public,
external declaration and retains alternate-loading rejection cases. This is a test inventory update;
source availability policy is unchanged. All four focused topology admission/rejection cases pass
in 36.98 seconds after compilation. Real-window navigation, archive export/restore/restart,
cleanup interaction and retained-library resource measurements remain outstanding while window
operation is suspended. No source library or original catalog participated in these cleanup tests.

Handwritten ownership sizes: association lifetime 171 production lines; cleanup transaction facade
127, inspection 228 and retirement 139; domain 81, application 64 and native API 76. Their dedicated
lifetime and cleanup tests have 106 and 353 lines; the existing duplicate-restoration test owner has
334. Dart controller/state/gateway have 155/81/119 production lines, and the settings row/confirmation
have 169/65. Dedicated controller and widget tests each have 107 lines; these new production owners
contain no inline tests. The existing local-file facade retains 2,629 production/support lines and
3,380 inline-test lines; only six inventory lines were added to that test section. Its dedicated
source-topology test owner has 458 lines. Generated bridge output is excluded from these counts.

## First-import fallback handoff

The ordinary-journal failure in run `40096057273` first reports
`identity_journal_file_attributes`, then admits independent metadata fallback. The published first
import still owns an unfinished journal window and a pending control. Each new inventory is rejected
with `metadata_inventory_baseline_authority_conflict`, leaving both recovery owners and live work
pending. This is a persistent execution-ownership conflict after fallback admission.

The separate first-import handoff owner retires that published import's pending control and authority
only after a current independent fallback receipt exists for the same root and generation. Receipt
admission and retirement share one transaction. Startup repairs retained overlapping owners even while
the replacement is pending. Baseline readers exclude retired unfinished execution while preserving
the failed window, cursor and completed history. No schema, source-media or journal-success claim
changes.

Four focused checks pass in 10.07 seconds after compilation: independent inventory admission with
unchanged failed-window evidence, durable handoff after reopening a prior overlapping state,
injected retirement failure rolling back the whole admission, and ordinary first-import replay
retaining its authority without a fallback receipt. The original native journal test passes locally
in 3.64 seconds before this handoff correction, so it does not reproduce or explain the hosted native
attribute error. Test-only diagnostics now retain the failing stage and numeric Windows error without
paths; the native-read cause remains open pending matching hosted evidence.

Run `40101178770` on `0ff69b35b76af6088fa75317d1aed08128ac37ed` confirms Static, Windows Scan,
Windows Accessibility and unsigned Release passing. Its Flutter failure is the settings-page fixture
expecting five choice controls after cleanup adds the sixth age selector. Other jobs are still running
at this checkpoint. This run predates the handoff correction; exact-head hosted verification remains
required. Local window operation is still suspended.

Handwritten ownership sizes: fallback composition 204 production lines, first-import handoff 66,
baseline reads 236 and inventory recovery-journal checks 110. The dedicated handoff tests have 234
lines. The existing persistent-journal facade retains 1,315 production/support lines and 3,131
inline-test lines; this change adds only the dedicated test-module declaration there. The native
lookup-error owner has 72 lines, including the five-line test-only diagnostic.

The settings-page fixture now includes the cleanup age control in its exact six-choice inventory,
covering shared popup menus and arrow icons as well as the cleanup row's presence. Its unchanged
storage, cancellation, keyboard, disabled-state and feedback assertions remain in place. All 15 cases
in `ame_settings_page_test.dart` pass locally. This correction changes no product behavior.

## Bounded association-lifetime restoration

Run `40101178770` completes with two failures on `0ff69b35b76af6088fa75317d1aed08128ac37ed`:
the corrected settings control inventory and the catalog's one-copy publication growth regression.
Catalog reports 653 passes, one failure and three ignored tests. The remaining three Rust partitions,
Static, Windows Scan, Windows Accessibility, unsigned Release and all five synthetic workloads pass.
The intermediate `3038650` run `40103198803` is superseded after this second failure is diagnosed;
it is not evidence for the following correction.

Clearing association timestamps previously expanded every historical lineage in the current group,
including already associated records. Publishing one copy against 50,000 such lineages used at least
600,200 SQLite steps, exceeding the unchanged 20,000-step assertion. Schema 55 adds a derived lookup
of unresolved records indexed by canonical group. Triggers maintain timestamp changes, group merges
and deletions atomically. Publication restores only matching unresolved members; first-loss writes
leave existing times intact. Migration backfills the lookup without modifying decisions, queue order,
catalog revisions or source media. Historical migration publication retains its schema-aware path.

Twelve focused checks pass in 39.36 seconds after compilation, including the original unchanged
one-copy publication test below its 20,000-step limit. The new mixed-history case retains 50,000
associated aliases and 50,000 unrelated unresolved groups: restoration by canonical group and by
source generation each stays below 5,000 SQLite steps and preserves all unrelated records.
The other cases cover migration/reopen, retained values, rollback/retry, timestamp and mapping
maintenance, missing/wrong lookup rows, missing maintenance triggers, ordinary association lifetime
and current-version compatibility admission/rejection.

Fifteen additional focused checks pass in 45.75 seconds: all ten cleanup cases, four historical
organization-reconciliation migration cases, and the version-33 retained-row upgrade with repeated
reopen. These include explicit backup restoration after cleanup, cancellation after a retirement
window, matching without a revision change and complete alias retirement. The historical table
inventory admits the new empty derived table explicitly. No UI interaction or original source
participated. These query-growth fixtures do not replace retained-library client/resource measurement.

Handwritten ownership sizes: association lifetime 198 production lines with 195 dedicated test lines;
association-index migration/validation 84 production/support lines plus its three-line test declaration,
schema objects 53 and dedicated tests 186. Current schema composition has 327 production/support
lines. The existing migration facade retains 4,121 production/support and 2,290 inline-test lines;
only module and migration dispatch calls were added. The existing catalog facade has 3,506
production/support lines and changes only its schema constant. Compatibility shape checks have 109
production/support lines, and the retained-row inventory test owner has 261 dedicated lines.
The new owners have no inline test bodies. Exact-head hosted verification remains pending.

Run [40104428180](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40104428180)
passes on `2736fc3f091ce9b4c9390701d6236a6aa75e8e25`: Static, Flutter, all four Rust
partitions, Windows Scan, Windows Accessibility, unsigned Windows x64 Release and all five synthetic
workloads. The three separately protected release jobs are skipped. This verifies the association
lookup, settings inventory and fallback handoff corrections; the native attribute-read error did
not recur, and its original cause remains unexplained.

## Retained-catalog organization query baseline

The manual `profile_retained_organization_queries` diagnostic uses the actual gallery, organization,
batch and continuation ports on a disposable copy of the retained `local-primary` catalog. Input
must resolve inside `build/test/derived`; the driver rejects path escape/reparse ancestry, opens that
input read-only and creates its writable copy in a separately owned temporary directory. It does
not resolve application storage, enumerate source roots, read images or generate previews. Its
scalar JSONL report contains no source paths or image identifiers and refuses to overwrite an
existing report. Automatic tests leave this explicitly ignored diagnostic unexecuted.

The prepared schema-45 snapshot contains 63,077 published physical files and 59,538 fingerprints.
It folds to 47,833 groups. Capturing the snapshot took 4.370 seconds, and the original database
file's SHA-256 remained unchanged across capture. The isolated input's SHA-256 also remained unchanged
after the complete diagnostic. Ordinary migrations and real group commands populate the working
copy; no SQL shortcut creates its album or Favorite decisions. The writable copy is retired after
the run, while the retained input and redacted report remain in ignored derived storage.

The production implementation is `2736fc3`; only the manual diagnostic differs from that revision.
An optimized Release build completes the diagnostic in 483.92 seconds after 7 minutes 11 seconds of
compilation. Connections are opened normally and the OS cache is not flushed. Each snapshot returns
at most 128 assets; each layout chunk returns at most 128 identifiers. The ranges below contain the
initial sample and three immediate repetitions, not statistically established percentiles.

| Catalog operation | Measured elapsed time |
| --- | --- |
| One-time schema-45 upgrade and open of the copy | 51.645 s |
| Pending snapshot, including shared counts | 11.565–12.896 s |
| Pending layout / following page | 8.013 s / 5.337 s |
| Save continuation / repeat continuation reads | 3.790 s / 3.734–3.753 s |
| Prepare / commit assignment of all 47,833 groups | 15.373 s / 62.567 s |
| Prepare / commit Favorite for the same groups | 20.164 s / 29.646 s |
| Reopen the populated copy | 2.737 s |
| Populated album snapshot | 12.119–13.175 s |
| Populated Favorite snapshot | 12.264–15.068 s |
| Empty pending queue snapshot after assignment | 11.184–12.105 s |
| Single-group unfavorite commit / resulting Favorite snapshot | 3.317 ms / 11.424 s |

All grouping, page bounds, layout totals, continuation identity and final Favorite-count assertions
pass. They expose an unresolved large-library latency problem: even an empty queue and the read
following a fast single-group write remain slow. These are database-port measurements, not desktop
save-to-visible latency or a retained-library performance pass. Continuous CPU/peak-memory sampling,
two-core measurements and client frames remain outstanding. The original process sampler could not
observe the sandbox worker; its timeout is not a zero-resource result.

The diagnostic is selected under the repository tool lock with `AME_R4_RETAINED_SNAPSHOT` pointing
only to the prepared isolated snapshot, then `cargo test --locked --release --manifest-path
rust/Cargo.toml --lib profile_retained_organization_queries -- --ignored --nocapture --test-threads=1`.
Its 180-second batch deadline is an operational cancellation guard, not a product acceptance target.
The dedicated diagnostic contains 316 test/support lines and adds one module declaration to the
existing organization test owner; it changes no production behavior. Canonical formatting passes.

## Viewer organization action placement

Favorite, album, defer and organization-menu actions now occupy the viewer's upper-right action
group, before the window controls and outside the filename drag region. Every direct action uses
the existing Material icon-button and tooltip components, including album-conflict and defer states.
The bottom bar retains browsing, metadata and zoom. Fullscreen moves the same mounted action subtree
into a themed upper-right surface, preserving pending action ownership during placement changes.

The eleven focused viewer checks pass, including action placement outside the drag region,
callback dispatch and identical element retention across fullscreen and return. All seven review
widget checks and seven conflict-workflow checks also pass. The Windows Debug build succeeds in
30.3 seconds. The controlled native client verifies ordinary and organization-mode icon placement,
Favorite/unfavorite from the new controls, fullscreen action availability and return to the original
window. The 700 reusable source fixtures retain their SHA-256, sizes and modification times.
Sequential undo/redo remains a separate requested follow-up; this placement change preserves the
existing command behavior. The bottom queue-dock experiment remains outside this delivery.

Handwritten owners contain 420 production lines in the image viewer, 109 in top/navigation controls,
247 in the bottom toolbar, 168 in review actions and 157 in the album action. They contain no inline
tests. The existing dedicated viewer test owner contains 843 lines. Hosted verification for this
placement change passed hosted run `40111403224` at `c4b8187e8dd67abfc0e8e70dd9245792aae649a5`.

## Contextual organization controls and chronological history

The selection row places Favorite first and More last. Favorite preparation reads whole-group
counts and reuses the same guarded batch for the chosen write: all-favorited and all-unfavorited
selections expose only their applicable action, while a mixed batch exposes both. Unfavorite uses
the installed Material Symbols heart-minus icon. Album assignment distinguishes unassigned, fully
assigned and mixed batches, with removal of confirmed assignments and no redundant same-album move.

The album picker uses Flutter's `showModalBottomSheet` and `CheckboxListTile`, retaining a single
destination while allowing it to be unchecked. It exposes ignore/defer and omits Cancel; Escape and
the backdrop use the existing cancellation boundary. This follows the requested bottom-tray
placement without replacing framework routing, focus or accessibility. The installed SDK's
`bottom_sheet.dart` exposes scroll control, safe area, drag handle and drag-dismissal parameters;
the [official Flutter API](https://api.flutter.dev/flutter/material/showModalBottomSheet.html)
confirms these capabilities. The [Material component page](https://m3.material.io/components/bottom-sheets/overview)
requires JavaScript in the available reader. The previous dialog lacks the requested bottom anchor;
a custom overlay is unnecessary. Album pages remain bounded and use existing scroll owners.

Schema 56 adds a history cursor and indexed field/album authority without changing historical
archive payloads. The native window upgrades its retained schema-55 fixture to schema 56 with all
six group decisions unchanged. Thirty-eight focused Rust checks pass across history (8), existing
undo (6), migration (3), archive restoration (7), cleanup (10) and historical inventory (4). They
cover repeated same-field inversions, cross-group batches, reopen, unknown acknowledgements,
stale heads, new branches, rollback, album lifecycle, content/group guards and archive epoch reset.

Fifty-four focused Flutter checks pass across organization actions (10), album viewer actions (5),
album widgets (5), conflict workflows (7), history menu (4), review widgets (8) and selection actions
(15). They cover contextual commands, unchecked/current destinations, form retry, cancellation,
disposed selection, commit-winning-cancellation and stable history intent during background changes.
A retained review tray rejects a replacement entry rather than applying its progress action to the
new image. Preview and selection menus retire with their captured targets.
The initial mixed-batch assertion expected the old move wording; it now checks the explicit mixed
assignment label. Bridge verification passes all 17 asynchronous contracts and matching hashes.
The Debug build completes in 64.0 seconds; the final target-lifetime revision rebuilds in 28.7 seconds.

Controlled Windows interaction on the reusable 700-file corpus verifies:

- Favorite at the left and More at the right; a selected favorited group offers only Unfavorite
  with the heart-minus glyph;
- bottom-tray placement, unchecked destination, replacement selection and committed whole-group
  movement, while Favorite remains unchanged;
- undo from a different viewed group without moving the current preview, followed by process
  restart, redo and another undo;
- organization-mode title marking and the three bottom queue icons, with the separate continuation
  action retained on the same row;
- removal from an album, preserved Favorite and ignored progress, then an unassigned picker with
  Add only and no Remove/Move action. Undo restores the complete previous group decision.

All six group decisions match the pre-test baseline after the native actions are undone. All 700
source fixtures retain SHA-256, sizes and modification times. No source media is modified. Existing
unrelated edits to gallery empty-state and duplicate copy remain outside this delivery; the full
read-only format check flags the former, while all owned Dart files are formatted. Exact-head hosted
verification follows publication; retained-library performance and recovery/cleanup acceptance remain
with the active roadmap.

Handwritten production owners contain 357 selection-action lines, 415 album-picker lines, 355
organization-adapter lines and 197 action-controller lines. Their dedicated selection/picker tests
contain 436/287 lines. History owns 189 production lines plus 120 authority lines, with 407 dedicated
test lines; migration owns 92 production lines, 16 test-support/declaration lines and 64 schema lines,
with 110 dedicated test lines. The history menu has 166 dedicated test lines. These owners have no
inline test bodies. No new dependency or license boundary is introduced.

## Organization scope and contextual control refinement

Organization entry now admits only all-source Library or a selected source/folder. Collection
workspaces cannot enter it, and continuation must match the current source, folder and descendant
policy. Favorites is available only in Albums. Ordinary workspace switches clear selection and
reset the position; an organizing Library retains its query, queue, selection and logical anchor
across primary-sidebar visits to Albums, Duplicates and Settings. The restored collection query is
admitted before organization-session reactivation, and failure preserves a retryable destination
snapshot. Duplicates and Settings retain the active review session while their surfaces are visible.

Ordinary selection/viewing omits ignore, defer and the organization history menu. The album tray
uses checkbox state without the selected-summary or routine whole-group explanation. Create,
remove and add/move actions use the installed Material Symbols glyphs. Selection preparation
no longer inserts text into the button row. A separate presentation owner waits 500 milliseconds
before showing its progress line and retires the delay when the command or tray takes over.

Ninety distinct focused Flutter checks pass: workspace navigation (11), organization mode (23),
query navigation (5), integrated review (2), viewer album action (4), picker widgets (5), conflict
workflow (7), review widgets (9), batch actions (18), history menu (4) and album settings (2).
These include cross-scope continuation rejection, ordinary versus organizing round trips,
restoration retry, hidden ordinary-mode controls, checkbox selection, cancellation and group writes.
The controlled preparation case checks identical button bounds before loading, after 500 milliseconds
and with the tray open. A 400-millisecond preparation completes without ever showing the indicator.
The previous settings test's removed Cancel button is replaced by route dismissal with the no-write
assertion retained. All 23 owned Dart files pass formatting and targeted analysis. The whole-tree
format check retains the pre-existing unrelated gallery-empty-state finding.

The final Debug build completes in 36.7 seconds. Controlled Windows interaction on the retained
700-file corpus confirms that an ordinary selected, scrolled Library returns to the top without
selection after primary-sidebar visits to Albums, Duplicates and Settings. An organizing Library
returns with its source scope, deferred queue, mode marker and selection intact; Albums remains an
ordinary browse surface. The native selection row hides organization history outside the mode and
exposes undo, redo and ignore inside it. The organization
tray retains the requested icons and conditional progress actions without the removed captions.
Nonzero logical-anchor restoration and failed restoration retry are covered by the focused
workspace tests. All 700 source fixtures retain their SHA-256, sizes and modification times.

The prior control commit's hosted run `40116594303` fails on the removed settings Cancel control,
three Dart style findings, seven current-schema recovery tests and three archive-restoration tests.
The recovery compatibility admission still stops at schema 55 despite schema 56 retaining the same
recovery structures.
The archive cases fail current-history authority validation.
These hosted failures remain distinct from the passing focused UI checks.

Production owners contain 147 workspace-navigation lines, 361 organization-mode lines, 368
selection-action lines, 51 delayed-progress lines, 421 picker lines and 1,891 screen-composition
lines, with no inline test bodies. Dedicated workspace/selection/mode tests contain 226/525/426
lines; integrated query/review tests contain 577/256. This refinement introduces no schema,
bridge, dependency or license change.

## Schema 56 verification follow-up

The completed hosted run `40116594303` exposes two recovery admission bounds still ending at schema
55. Schema 56 changes organization history without changing the inventory-spool or live-gap
contracts. Both explicit bounds now admit 56, retaining exact DDL/marker checks and rejection of
unknown future versions. No migration, validator relaxation or source-media change is introduced.

Three lower-level archive-restoration tests reopen catalogs after bypassing the production writer's
history-epoch reset. Their transaction helper now performs that same finalization after updating the
organization revision. The production writer and current-history validation remain unchanged; the
existing reopen, rollback, ordering and retry assertions are retained.

The history request uses a typed initializing formal, and its menu test drops an unused import.
The removed settings Cancel control and selection-test braces are handled by the preceding UI
change. Targeted Dart analysis, formatting, Rust formatting and all four history-menu checks pass.

All 44 focused Rust checks pass: compatibility shape (3), atomic current recovery (1), retired
live-gap repair (8), terminal inventory repair (7) and archive restoration (25). They cover all ten
previously failing Rust cases, exact-shape rejection, rollback, restart, queue preservation,
cancelled/failed restoration and idempotent replay. The compatibility owner has 109 lines and 56
dedicated test lines; the affected archive test owner has 278 dedicated test/support lines.
Current-head hosted verification follows publication; the retained-library R4 acceptance remains open.

## State-aware progress removal

Gallery selection, the detailed viewer and the album tray share one typed progress-control owner.
Deferred groups replace defer with `移除`; ignored groups replace ignore with the same action.
Both submit the existing return-to-pending command, retaining Favorite state and source media.
Single-image controls use the inspected decision rather than the queue from which viewing began.
Batch controls use the queue-scoped selection admitted and revalidated by the existing batch owner.
Unresolved album conflicts retain their deferred assignment and cannot be removed through this control.
An open menu retains its observed action and cannot silently switch ignore into remove after refresh.

Forty-three distinct focused Flutter checks pass: album preparation (3), review widgets (13), batch
selection (22) and history/progress menu (5). They cover actual restore requests from viewer and
tray, deferred/ignored batch actions, preserved selection scope/exclusions, stale review entries,
conflict admission, cancellation, uncertain saves and history changes while the menu is open.
Owned Dart formatting and targeted analysis pass. The final Debug build completes in 25.7 seconds.

Controlled desktop interaction on the retained 700-file corpus verifies deferred removal from the
detail tray and ignored removal from the detail menu. Each changes the corresponding queue count
from one to zero and pending from zero to one. Native undo restores ignored state and updates the
open viewer's menu back to remove. Subsequent chronological undo restores the pre-test deferred
decision. Read-only catalog snapshots confirm unchanged albums and Favorites across all six groups;
all 700 source fixtures retain their SHA-256, sizes and modification times.

Production owners contain 72 progress-control lines, 377 selection-action lines, 423 picker lines,
134 menu lines, 141 review-action lines and 173 viewer-album-action lines, with no inline tests.
Dedicated selection/review/menu tests contain 574/359/204 lines. No schema, generated bridge,
dependency or licensing change is introduced. Hosted verification follows this publication;
the broader recovery, retained-library performance and R4 integrated acceptance remain open.

## Native archive export and restoration

The Debug client built from `f096ffd5` exports the retained test profile through the native Save
dialog into a new `.ame-backup` file outside source storage. The 753,800-byte archive has format
version 2 and source schema 56. Its inspection reports two albums, six groups, one Favorite group
and a saved continuation, matching the read-only catalog snapshot.

Restoration into the same profile reports six matched groups and no unmatched groups. The actual
confirmation and completion flow succeeds, producing one durable restore receipt. Catalog inspection
retains the five organized groups, one deferred group and independent Favorite state without duplicate
albums. After closing and reopening the client, the album displays five groups covering 490 physical
files and Favorites displays one group covering 25 files, consistent with the stored decisions.

A separately created empty test profile restores the same archive through the native file picker.
The preview correctly reports zero matched groups and six unmatched groups. Completion retains both
album definitions and all six decisions without configuring a source or inventing gallery members.
Read-only inspection confirms five organized records, one deferred record, one Favorite bit and one
restore receipt, with all six records marked unassociated.

The cleanup UI excludes these recent records under the default 90-day threshold. Selecting all
unassociated records presents six image records, zero duplicate choices, five album memberships,
one Favorite and six progress records, all unassociated for less than one day. The destructive
confirmation remains pending; this checkpoint does not establish native cleanup publication or
post-cleanup reimport behavior. Fresh-profile restart, late association and repeated native restore
remain for the following recovery check. Existing adapter regressions retain their separate scope.

All 700 reusable source fixtures retain their SHA-256, sizes and modification times. The archive,
test profiles and diagnostic outputs remain in ignored derived storage. No original library or
original catalog is changed. Hosted run `40122920970` completes with all Rust partitions, Static,
native integrations, unsigned Release and the five synthetic workloads passing. Flutter fails one
integrated review assertion that still expects defer after selecting the deferred queue; the requested
remove action correctly dispatches restore. The corrected assertion also checks the visible remove
label; both integrated review tests pass locally. Hosted verification of the corrected checkpoint,
retained-library performance and integrated R4 acceptance remain open.

### Repeated native restoration and late association

The complete Debug client builds from product commit `7e11d8a7` in 49.8 seconds without warnings.
It includes the three preserved, uncommitted presentation edits in duplicate wording, the gallery
header and empty-state wording; no archive, association or persistence code differs from that commit.
Unlike the earlier navigation-only check, this uses the matching current frontend and Rust adapter.

A fresh isolated profile applies the existing 753,800-byte archive twice through the native picker,
inspection, confirmation and completion flow. Both inspections report zero matched and six unmatched
groups. Read-only catalog inspection after each application finds the same two albums, five organized
groups, one deferred group and one independent Favorite. The verified archive identity remains single;
two explicit restore operations are retained without duplicating decisions or imported history.
Closing and reopening this still-empty profile preserves all six unmatched records.

Importing the existing 700-file reusable source corpus through the native folder picker automatically
associates all six records. The client shows five folded groups covering 490 files in the restored
album, one Favorite group covering 25 files, and organization counts of five organized, zero pending,
one deferred and zero ignored. The deferred queue contains its retained image. Selecting all ages
in unresolved-record inspection returns no eligible records. No manual reassignment or additional
backup application is needed after import.

Another client restart retains these associations and displays the album's five groups again.
Read-only catalog inspection confirms the same album, Favorite and disposition values, with all
unassociated timestamps cleared. All 700 fixture SHA-256 values, sizes and modification times remain
unchanged. The original test profile and the separate pending-cleanup profile remain intact; the
archive and test outputs stay in ignored derived storage. This closes the repeated-restore,
unmatched-profile restart and late-association client gaps. Native cleanup publication and
post-cleanup reimport remain unverified pending the existing action-time confirmation.

## Organization read cost and connection cache budget

The representative join previously materialized the complete published/retained fingerprint view.
It now resolves one compatible evidence generation and joins that fingerprint by its existing key.
A regression with 50,000 unrelated published fingerprints keeps each collection, queue and saved
continuation read below 20,000 SQLite VM steps. It covers current and retained display evidence,
preferred copies, shared counts and empty scopes. All 19 organization query checks pass.

Separate read-only trials on the populated disposable catalog identified page-cache churn: the
same representative query took 2.273 seconds with a 2 MiB budget, 1.765 with 8 MiB, 0.926 with
32 MiB and 0.672 with 64 MiB. The connection policy adopts 32 MiB per connection, allocated on
demand. Eleven connection-policy checks pass, covering independent reader/writer budgets,
non-persistence, read-only authority, rollback, durable reopen and first-import scope restoration.
The focused Debug compilation emitted the local incremental-cache hard-link fallback warning;
the subsequent Release build completed without that warning in 2 minutes 54 seconds.

The complete retained-profile diagnostic uses the same unchanged schema-45 input, 63,077 physical
files, 47,833 groups, normal migrations and real batch commands as the preceding baseline. It runs
serially without a concurrent catalog capture, returns at most 128 assets/layout identifiers and
does not flush the OS cache. The initial sample and three immediate repetitions give these ranges;
they are observations rather than percentiles or desktop frame measurements.

| Catalog operation | Measured elapsed time |
| --- | --- |
| One-time upgrade and open | 21.003 s |
| Pending snapshot, including shared counts | 4.904–5.179 s |
| Pending layout / following page | 3.658 s / 3.213 s |
| Save continuation / repeat continuation reads | 2.202 s / 2.146–2.172 s |
| Prepare / commit assignment of all 47,833 groups | 12.803 s / 58.442 s |
| Prepare / commit Favorite for the same groups | 15.842 s / 36.071 s |
| Reopen the populated copy | 1.926 s |
| Populated album snapshot | 4.168–4.404 s |
| Populated Favorite snapshot | 4.119–4.175 s |
| Empty pending queue snapshot after assignment | 4.334–4.476 s |
| Single-group unfavorite commit / resulting Favorite snapshot | 6.228 ms / 4.312 s |

The diagnostic passes in 264.37 seconds. The enclosing worker measurement records 268.169 seconds
wall time, 253.344 seconds lifetime CPU, a 92.406 MiB peak working set and an 81.766 MiB sampled
peak private allocation. Its 514 samples use a 500 ms interval; the first observes only 0.047 seconds
of worker CPU. Compilation is excluded. The input SHA-256 remains unchanged and source-media
reads remain zero. Reports and resource samples are retained in ignored derived storage.

These changes reduce the measured read waits substantially; they do not complete the performance
exit. Empty queues still take over four seconds, continuation reads exceed two seconds, and the
large Favorite commit is slower than the earlier baseline. Client save-to-visible timing, concurrent
connection costs, constrained-device behavior and integrated resource acceptance remain open.
The earlier fingerprint-only trial did not meaningfully improve populated reads, so it is not used
as performance acceptance.

The affected query owner contains 183 production lines, connection policy 61 production lines,
and their new/existing dedicated regression owners 151 and 217 lines; there are no inline tests.
No schema, bridge, dependency, licensing or source-authority changes are introduced. Rust formatting
and owned whitespace checks pass. The whole-tree format check reports an unrelated, uncommitted
gallery-state file; that file is preserved and excluded from this delivery. Hosted checks for the
published implementation remain required.

## Organization scope and progress-count query cost

Organization reads now apply their complete source, folder, search and collection scope before
choosing group representatives, without repeating those predicates on the resulting locations.
Progress counts aggregate canonical groups directly instead of resolving preferred copies. The
regression denies access to the preferred-copy table while checking shared counts across all-source,
root, Favorite and empty-search scopes. Twenty organization-query, thirteen continuation and
twenty-four ordinary gallery-query checks pass. The focused Debug and Release builds emit no warnings.

The unchanged retained-profile diagnostic uses the same schema-45 input and real command path,
63,077 physical files and 47,833 groups. It runs serially with fresh connections and no OS-cache
flush. The first snapshot and three immediate repetitions are reported together:

| Catalog operation | Measured elapsed time |
| --- | --- |
| One-time upgrade and open | 20.853 s |
| Pending snapshot, including shared counts | 3.578–3.761 s |
| Pending layout / following page | 2.825 s / 2.497 s |
| Save continuation / repeat continuation reads | 1.623 s / 1.597–1.618 s |
| Prepare / commit assignment of all 47,833 groups | 12.152 s / 55.634 s |
| Prepare / commit Favorite for the same groups | 15.739 s / 34.174 s |
| Reopen the populated copy | 2.075 s |
| Populated album snapshot | 3.316–3.442 s |
| Populated Favorite snapshot | 3.301–3.325 s |
| Empty pending queue snapshot after assignment | 3.352–4.014 s |
| Single-group unfavorite commit / resulting Favorite snapshot | 3.928 ms / 3.558 s |

The diagnostic passes in 232.96 seconds. The worker measurement records 236.596 seconds wall time,
223.234 seconds lifetime CPU, a 94.910 MiB peak working set and a 99.727 MiB sampled peak private
allocation. Its 453 samples use a 500 ms interval and exclude compilation. All result, count,
continuation and bounded-page assertions pass; input SHA-256 remains unchanged and source-media
reads remain zero. The full reports remain in ignored derived storage. Empty-queue waits, large
batch writes, client feedback, concurrent connection costs and constrained-device acceptance remain
open; these catalog measurements do not establish desktop response times.

The affected gallery facade contains 397 production lines; filter, duplicate, organization,
progress-count and continuation owners contain 64/152/183/111/101 production lines, with no inline
tests. The new aggregation regression contains 99 dedicated test lines. No schema, bridge,
dependency, licensing or source-authority changes are introduced. Owned Rust formatting and
whitespace checks pass. The whole-tree format check still reports only the preserved, unrelated
gallery-state edit. Exact-commit hosted verification remains required.

## Returning to the retained gallery

Hosted run `40127335693` exposes an empty-shell navigation failure: returning from Settings forces
a catalog read even when the current query's first window is already retained. The local reproduction
reports `bridge_query_snapshot_load_failed` and keeps Settings visible. The unchanged one-frame
return assertion now passes after `LibraryReviewQueryTransition` takes ownership of target and
reload selection. First-window reuse still passes through the application query admission and
position-reset path. Later windows, active time anchors, pending reads and failed queries retain
refresh/admission behavior; changed collections and saved review positions still read their scope.

Thirty-four focused checks pass across the query-transition boundary, workspace lifetime, empty
shell, collection navigation and integrated organization flow. They include failed restoration,
supersession, album retirement, disposal, bounded-window reset and preserved review selection.
The new boundary contains 80 production lines and 184 dedicated test lines; the shared screen
is reduced to 1,875 production lines with no inline tests. Its remaining responsibilities are
unchanged; this extraction does not resolve all screen-composition debt.

The current Profile frontend builds without warnings in 138.8 seconds. Native checks use a disposable
copy paired with the existing `f096ffd5` Debug Rust adapter so the test-storage override remains
effective; the generated bridge/API contract is unchanged between these revisions. This combination
verifies the changed frontend interaction, not Release packaging or backend performance. Empty
Library returns from Settings. In a copied 700-file/six-group test catalog, ordinary browsing with
one selected image and a scrolled viewport returns to the first row with selection cleared.
Organization mode retains its deferred queue and selected group across the same Settings visit.
All 700 source fixture hashes, sizes and modification times remain unchanged. The separate recovery
profile retains its six unmatched records and pending cleanup authority.

The same hosted run separately fails the exclusive-WAL-lock fixture during lock acquisition,
before recovery dispatch. Its diagnostic is retained for the following cause-level repair; this
navigation checkpoint does not claim that Rust partition or exact-commit CI has passed.

## Exclusive-WAL-lock fixture preparation

Run `40127335693` finishes with two failing partitions: the empty-shell Flutter case above and
Schema/Queue's exclusive-lock fixture. Its other Rust partitions, Static, native integrations,
unsigned Release and synthetic workloads pass. The lock failure reproduces locally at blocker
creation, before the timed recovery call. Configuring the new connection's page-cache budget
already joins the WAL, contradicting the fixture's required unopened read boundary.

The fixture now sets only its existing busy deadline before establishing the exclusive blocker;
normal catalog configuration runs after unlock and before retry. Its one-second nonblocking
dispatch assertion, unchanged queue/attempt state, restored deadline, autocommit and successful
subsequent lease assertions remain intact. All six lease-admission checks pass in 9.96 seconds.
The initial Debug compile reports an incremental-cache hard-link fallback; a subsequent compilation
of the same source with process-local incremental caching disabled completes without warnings in
72 seconds. This diagnostic changes no persistent compiler setting or warning policy.

Only the 290-line dedicated test owner and this evidence change; production lock behavior, schema,
bridge, dependencies and source authority remain unchanged. Rust formatting and whitespace checks
pass. Hosted verification of the combined published fixes remains required.

Hosted run [40130669974](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40130669974)
passes on `7e11d8a7478e8dc95f64f5067d4e75a82f6ebbb9`: Static, Flutter, all four Rust
partitions, Windows accessibility and scan integrations, unsigned Release, all five synthetic
workloads and the aggregate gate succeed. The three protected/signed-release jobs are skipped by
their existing conditions. This covers the preceding query, navigation and fixture repairs.

## Full-catalog Debug client interaction - 2026-10-25

The complete Debug client from product commit `7e11d8a7` opens a separate, schema-56 copy of the
retained `local-primary` catalog: 63,077 physical files, 47,833 organized and favorited groups,
and one album containing those groups. The copy uses SQLite's read-only online backup and the
Debug test-storage override. Three existing, uncommitted presentation/copy edits remain present;
this is a working-tree client observation, not exact-commit packaging acceptance.

Normal startup synchronization observes 77 journal entries and publishes one checkpoint without a
reported journal failure. Twelve new independent images appear in the library, giving 63,089 files
and 47,845 groups. The album and Favorites still show their original 63,077 files and 47,833 groups.
Collection changes retain the previous frame with the existing top progress indicator until the
new query completes.

Selecting one Favorite offers only unfavorite. After commit, the shared transient notice appears,
the selected group leaves Favorites, selection clears, and the collection shows 63,076 files and
47,832 groups. A read-only post-exit check confirms one organized, non-favorite record and 47,832
organized favorite records; all 47,833 retain the album. Organization mode shows 47,833 organized
and 12 pending groups. The deferred queue completes with its empty state and zero count.

The owned client exits normally. A 500 ms sampler records 864 samples over 444.493 seconds, with
a peak working set of 568.191 MiB and peak private allocation of 530.309 MiB. Observed lifetime CPU
rises from 46.266 seconds at the first sample to 475.844 seconds at the last; startup before the
first sample is not included in the sampling interval. This interval includes UI observation gaps,
ordinary synchronization, visible preview reads and Debug rendering. It is not a steady-idle or
optimized-build resource benchmark. The bounded log check finds no exception, overflow, panic or
nonzero journal-failure entry.

The input catalog copy's SHA-256 remains unchanged; all application writes target the separate
test profile. The native client reads original images for normal previews, so the earlier catalog
diagnostic's zero-media-read claim does not apply here. No source mutation or hydration command is
issued. Reports, resource samples and logs remain in ignored derived storage. The previous empty
recovery profile is reopened with all six unmatched records and its pending cleanup preview intact.

These observations cover full-catalog client state and committed-save feedback. Screenshot timing
includes tool/observation intervals and is not reported as precise interaction latency. Profile or
Release response times, concurrent-connection cost attribution, large-write waits and constrained
two-core/8-GB measurements remain open. Ordinary optimized builds ignore the Debug-only storage
override and must not be used as isolated clients without an admitted isolation path.

## Published-source organization query plans - 2026-10-25

Organization queries now join locations through both their root and active scan, keeping roots
outermost so SQLite can use the existing root/generation index. The shared gallery scope owner
selects this source relation for albums, Favorites and organization queues. Windows, layout,
timeline and source counts share that policy; ordinary Library ordering retains its previous
query source. Group representatives and progress aggregation keep their existing canonical
membership, compatible evidence and preferred-copy rules.

A new regression adds 25,000 unpublished historical locations beside three current locations.
The unfolded Favorite snapshot previously executed about 4,376,000 SQLite instructions because
its plan resolves organization state before rejecting the old scan. The repaired snapshot and
layout reads each stay below the unchanged 20,000-instruction bound. The same case checks folded
Favorites, a whole-library album, a single-source album and an empty pending queue, including
exact location IDs, physical/group counts and progress totals. Its initial fixture omission of
the required preview size is corrected before measuring these reads.

All 66 focused cases pass: 21 organization-query, 13 continuation, 24 ordinary gallery-query and
eight exact-duplicate gallery checks. Debug compilation and owned formatting complete without
warnings. The gallery facade contains 394 production lines; scope, organization, queue-count and
timeline owners contain 77, 186, 112 and 124 production lines, respectively, with no inline tests.
The new regression has 88 dedicated lines and reuses the existing 151-line cost-test owner.
No schema, cache-budget, bridge, dependency or source-authority change is introduced.

The same Release retained-catalog workload completes against the unchanged isolated input with
63,077 physical files and 47,833 groups. Each collection snapshot returns at most 128 assets;
populated album/Favorite timelines still contain 109 buckets. Four snapshot samples include the
first read and three repeats, with fresh catalog connections but no OS cache flush:

| Operation | Before this change | Published-source query |
| --- | --- | --- |
| Pending snapshot | 3.578–3.761 s | 2.360–2.655 s |
| Album snapshot | 3.316–3.442 s | 2.814–3.046 s |
| Favorite snapshot | 3.301–3.325 s | 2.784–2.836 s |
| Empty pending snapshot | 3.352–4.014 s | 2.357–2.503 s |
| Warm continuation, three samples | 1.597–1.618 s | 1.341–1.382 s |

All-group album assignment takes 11.932 s to prepare and 54.251 s to commit; all-group favorite
preparation/commit takes 15.116/34.008 s. These writes show no material improvement from the
previous 12.152/55.634 s and 15.739/34.174 s. A single unfavorite commits in 3.657 ms, followed by a
2.812-second Favorite snapshot. The remaining read and whole-library write waits stay open.

The worker exits successfully after 213.707 seconds, using 201.797 CPU seconds. Sampling every
500 ms across 408 samples records peak working set/private memory of 94.453/83.039 MiB.
Compilation is excluded. The input SHA-256 is unchanged, source-media reads are zero, and the
workload's result assertions pass. This measures catalog ports, not native-client response time;
the earlier full-client Debug observation remains separate. Hosted run
[40135559945](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40135559945) for `dcf79c9d`
passes its completed jobs but is superseded while Catalog is still running. Current verification
includes this source change and the retained cancellation case at `c69f7adb` in
[CI run 40137916874](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/40137916874), which is
complete with all applicable jobs passing. The superseded run is cancelled rather than recorded
as a complete pass.

## Retained-catalog batch cancellation - 2026-10-25

The retained cancellation diagnostic uses the same complete local-primary catalog, with 63,077
physical files, 47,833 organized and favorited groups, and one album. It backs up the admitted
snapshot into disposable derived storage and exercises the production batch control. It does not
operate the pending recovery window or read original media.

Cancellation is requested from a separate observer after actual preparation/write progress, with
the full 47,833-group selection retained. The observed outcomes are:

| Phase | Groups completed when requested | Groups completed when stopped | Request to return |
| --- | ---: | ---: | ---: |
| Preparation | 11,958 | 11,959 | 2.872 ms |
| Saving, before transaction commit | 23,918 | 23,919 | 4.516 ms |

Both return `organization_selection_cancelled` and publish the cancelled terminal phase. Preparation
has run for 5.584 seconds when it returns. A separate complete preparation takes 15.013 seconds;
the interrupted save runs for 17.211 seconds. Reopening after cancellation takes 1.776 seconds.
After both cancellations, a deterministic streaming digest of every organization table and catalog
revision matches the original. Reopen retains all 47,833 Favorites, and no receipt for the cancelled
write exists. This covers record values, history and continuation rather than only row totals.

The Release worker passes its one explicit manual case and exits with code zero. Its 53.547-second
measured lifetime uses 47.984 CPU seconds. The 97 samples at 500 ms record peak working set/private
memory of 88.148/78.527 MiB. Compilation is excluded, OS cache is not flushed, both retained input
hashes remain unchanged, and source-media reads are zero. These are individual catalog-operation
samples, not percentile estimates or native-client cancellation timings.

The diagnostic wrapper initially requests the old report filename after the successful worker and
resource capture. Its output path is corrected, and the existing worker log, phase records,
completion record and original input hash are verified without rerunning the workload. The manual
case is retained as a 200-line dedicated test owner; its existing snapshot/profiling owner has 318
test/support lines and no production changes. Compilation of the ignored manual case belongs to
ordinary CI; its retained input remains local and is never sent to hosted runners.

## Full-catalog Profile client timing - 2026-10-25

The native client uses the same complete local-primary snapshot: 63,077 physical files, 47,833
organized and favorited groups, and one album. Each attempt receives a fresh derived catalog and
preview cache. The final copy takes 4.851 seconds, outside the measured client lifetime. The OS
file cache is not flushed. Visible previews may read original media through the existing guarded
preview path; directory synchronization and exact-duplicate analysis are disabled. There is no
source enumeration, full-library media hashing or placeholder hydration in this workload.

This is Flutter Profile with an optimized Rust library retaining debug assertions and isolation
diagnostics, not the ordinary Release artifact. The native library SHA-256 is
`8e7e2c4cfa8876a8ab1855d5c4a598fce97e6b9a1d4494f1383f7199787e2a4e`.
The frontend includes the three existing local copy/layout edits; the measurements describe this
working-tree diagnostic. Product code is otherwise unchanged from the published query repair.

Three album/Favorite reads complete in an earlier attempt before a later test-driver failure.
Those valid segments are retained without rerunning the three-read sequence. The final attempt
passes the remaining functional workflow and exits with code zero. Each operation below has one
sample unless explicitly marked otherwise; these ranges are not percentile estimates.

| Client operation | Observed time |
| --- | ---: |
| Initial catalog visible, from catalog load | 6.256 s |
| First visible preview, from the same start | 6.271 s |
| Album catalog visible, three reads | 4.708–5.360 s |
| First album preview, same three reads | 4.716–5.383 s |
| Favorite catalog visible, three reads | 4.743–4.894 s |
| First Favorite preview, same three reads | 4.748–4.905 s |
| Single unfavorite button feedback | 0.166 s |
| Updated Favorite count/list after returning from the viewer, from the same click | 10.697 s |
| Enter empty pending queue | 4.436 s |
| Empty deferred queue, including the prior mode-read wait before its button enables | 6.556 s |
| Continue saved work: viewer actions ready | 5.515 s |
| Continue saved work: original image visible | 5.525 s |
| Prepare all 47,832 remaining Favorites | 21.868 s |
| Commit removal of all 47,832 Favorites | 38.862 s |
| Album visible after a click during that write | 38.857 s |
| First album preview after that click | 38.867 s |
| Empty Favorites visible after commit and publication refresh | 10.291 s |

Initial timing starts after native/storage initialization and is not process-launch-to-ready time.
Single-group save timing includes the real viewer action and return to the updated collection.
Continuation is checked after leaving and reentering organization mode in the same process: the
saved group matches the reopened viewer, including its actual image. This does not add a new
cross-process continuation result.

The concurrent write uses the production organization batch service while real sidebar navigation
requests the album. Saving has completed 17 of 47,832 groups when navigation begins. Both catalog
and preview become visible only after saving has finished. Final membership and counts are correct,
but the 38.9-second blocked read leaves usable concurrent browsing unverified. This workload does
not represent directory synchronization or duplicate-analysis contention, and its functional pass
does not close that performance gap. The cause of the serialized client wait still needs diagnosis.

The final driver lifetime is 151.935 seconds, with 281.094 client CPU seconds. Across 279 samples at
500 ms, peak working set/private memory is 395.402/512.719 MiB. In-process observation records peak
RSS of 401.820 MiB and at most 500 retained assets, matching the configured 500-asset window. Across
5,988 frames, build/raster P95 is 0.859/1.448 ms. These frame statistics coexist with the measured
query waits and do not establish prompt navigation or low-end-device acceptance.

Local evidence is retained in ignored `build/test/derived/r4-client-measure`: the final timing,
resource and driver reports, and the earlier three-read segment in `attempt-03-key-simulation`.
Earlier attempts preserve their partial results and failures: Windows binary-path launch, equivalent
path separator comparison, Profile key simulation, popup mounting, disabled queue input and a card
corner tap that did not open the viewer. The final harness uses the absolute binary, filesystem
identity, visible return button, enabled-control checks and card-center taps. No product guard,
timeout or result assertion is removed to obtain the passing run.

Every attempt preserves the retained input SHA-256, and the final diagnostic client exits. Before
reopening the original recovery profile, its closed catalog SHA-256 still matches the pre-measurement
baseline. The restored native window again shows the pending cleanup confirmation for six unmatched
groups: five album memberships, one Favorite and six progress records. Read-only inspection confirms
the same record summaries, two album definitions, restore receipt and operation counts. No cleanup
is executed. Cleanup publication/reimport and the constrained-device identity confirmation remain
outside this measurement result.

## Cleanup range inside the confirmation dialog - 2026-10-25

Settings now has one `查看并清理` entry. The dialog owns range selection and fresh inspection,
including empty results and inspection retry. Changing range clears the prior confirmation authority;
only the current nonempty `LibraryCleanupReady` can be returned to the unchanged application apply
guard. Closing the dialog requests cancellation of an unfinished inspection and cannot reopen it
when a late result arrives. Uncertain apply failures retain the existing operation-identity retry.

The implementation reuses the shared `SettingsChoice` and Material
[AlertDialog](https://api.flutter.dev/flutter/material/AlertDialog-class.html), whose scrollable
content and separate actions cover the filter, affected counts and explicit confirmation. The
[Material dialog reference](https://m3.material.io/components/dialogs/overview) is script-dependent;
the installed Flutter 3.44.9 source confirms `showDialog` result delivery, modal dismissal and closed
focus traversal. No custom dialog, popup, focus or persistence mechanism is introduced.

All five cleanup-row cases and 15 settings cases pass, including 1.4 text scaling, empty-to-populated
range changes, failed inspection retry, cancellation with late completion and confirmation using the
new inspection. Formatting and strict analysis pass. Production row/dialog owners have 117/153 lines
with no inline tests; dedicated row tests have 213 lines. The existing 1,198-line settings suite only
changes its control inventory. No schema, bridge, source or backend cleanup policy changes.

The rebuilt isolated Windows client shows one settings entry, an empty 90-day result, then all six
unmatched groups after choosing all records inside the same dialog. Escape closes the dialog and
returns to the single entry. Read-only inspection retains the same six records, two albums, restore
receipt and operation counts. No native cleanup is executed; cleanup publication/reimport remains
the separate pending recovery check. Hosted verification follows this implementation commit.

## Separate organization exit in the bottom dock - 2026-10-25

The bottom dock now places `退出整理模式` independently to the left of the queue surface,
using the existing tonal button style and the existing mode-exit owner. The overflow-menu exit
remains available, and the continuation button retains its separate right-hand position and
saved-position visibility rule. No new progress write or navigation policy is introduced.

All 13 review-widget cases pass, including equal heights and vertical alignment at 1.0 and 2.0
text scales, queue switching and restoring the captured ordinary-browsing context through the
new button without a progress write. Formatting and strict analysis pass. The production dock
has 147 lines and no inline tests; its dedicated widget suite has 374 lines.

The rebuilt isolated Windows client shows the separate left-hand button beside the empty queues.
Clicking it removes the dock and title mode indicator and restores the ordinary Library controls.
The test uses the existing empty recovery profile and does not import, modify or clean up media
or unmatched records. Hosted verification follows publication on the existing R4 branch.

## Centered content during sidebar motion - 2026-10-25

The gallery viewport's empty-state and footer slots and the Albums landing surface now consume
the existing sidebar animation with centered alignment. The content retains its destination width;
the visible center follows the moving boundary. The footer retains its intrinsic height through
the pinned SDK's [OverflowBox fit](https://api.flutter.dev/flutter/widgets/OverflowBox/fit.html).
No new animation clock, catalog request or intermediate photo-wall layout is introduced.

Sixteen focused widget cases pass across the content, sidebar, right-edge and photo-layout suites.
They cover intermediate frames, rapid reversal, 1.0/2.0 text scaling, immediate reduced-motion
placement, bounded destination-width layouts, final hit testing and stationary right-edge controls.
The first regression exposed an unconstrained footer width that cancelled the center offset;
the shared owner now reserves the destination width before applying the visible-width alignment.
Formatting and strict analysis pass. Production content/viewport/workbench owners have 97/63/310
lines and no inline tests; the new dedicated regression has 142 lines.

The rebuilt isolated Windows client verifies collapse and expansion while organization mode stays
active, with the empty-library content and bottom dock centered in each visible area. Existing
unavailable-position feedback and queue selection remain present. Native inspection covers the
interaction and final geometry; deterministic widget checks cover the animation's intermediate
frames. Source media and recovery records remain untouched. Queue-change loading feedback is a
separate reported follow-up.

## Stable organization transitions and icon controls - 2026-10-25

Mode entry, queue switching and exit now retain the published empty surface instead of replacing it
with a spinner, an hourglass and disabled import styling. The dock blocks repeated input without
changing button colors; the existing mode guard still rejects concurrent keyboard commands.
Query and mode feedback uses a 250-millisecond delay in the shared loading region, with cancellation
on completion and disposal. Direct page and timeline positioning retain immediate feedback.
The bottom controls now use icons with named tooltips and queue counts, and the header adds ignored
groups to its count row. Existing continuation visibility and whole-group progress rules remain intact.

Forty focused widget cases pass across empty-query feedback, first-import states, loading geometry,
loading-region lifetime, sidebar motion, review controls, unified loading and unified review. They
cover short and sustained waits, timer retirement, repeated pointer/callback commands, removed and
restored semantic tap actions, unchanged geometry, initial loading, 1.0/2.0 text scaling, scoped
counts and ordinary-context restoration. The initial busy-control regression exposed a test-owned
semantics handle disposed after the binding check; disposing it in the case's `finally` resolves
the harness failure. Strict analysis and the Windows Debug build pass. The unavailable-image
integration assertion now uses the queue's accessible name; that workflow was analyzed but not rerun.
The read-only repository format check reported the empty-state file and one queue-lookup assertion.
The assertion is corrected within its changed lines; the staged empty-state source and affected
assertions pass formatting. Unrelated working-tree formatting remains outside this commit.

The rebuilt Windows client verifies entry, all three queue selections, the independent exit button,
re-entry, tooltip names/counts, accessible button labels and sidebar collapse/expansion. The header
shows the ignored count, and the dock stays icon-only and centered. Native checks cover the actual
interactions and final layout; deterministic widget checks cover transient frames and delayed
feedback. The existing empty recovery profile is retained, with no import or cleanup performed.

The dock and controls owners have 42/141 production lines, and the loading owner has 78; they have
no inline tests. The existing screen/header have 1,879/585 lines and only receive composition/copy
changes. Dedicated review-widget, empty-feedback and unified-review suites have 437/70/263 lines;
the affected unavailable-image integration owner has 95. No schema, bridge, dependency or source
policy changes are introduced. Publication remains on the existing R4 branch. Its pushes have no
automatic workflow trigger without a pull request; no hosted run covers this checkpoint yet.

## Floating review dock and queue scrollbar - 2026-10-25

The independent exit and continuation buttons use 40-by-40 logical-pixel surfaces instead of 52,
with their 20-by-20 icons unchanged and centered against the existing queue surface. The dock floats
over the full photo viewport without a reserved footer row or full-width background. Transparent
space passes input to the gallery, and existing trailing scroll padding clears the last images.
Passive continuation text is omitted; mode failures retain retry through the shared top notice.
Organization queues use an interactive right-side Material scrollbar on the existing controller;
ordinary browsing keeps its time navigation. Tooltips, continuation admission, command guards and
bounded loading are unchanged.

All 46 focused cases pass: 24 gallery interactions, 14 review widgets, three sidebar-motion cases,
two overlay cases, two unified-review cases and one loading-geometry case. These cover 1.0/2.0 text
scales, unchanged icon bounds, control alignment, pointer passage outside the dock, top-notice retry,
and scrollbar dragging to distant content and back with both gallery layouts and manifest modes.
Scoped formatting, strict analysis of the ten affected Dart files and the Windows Debug build pass.

The rebuilt isolated client uses the retained mixed test library. Native inspection confirms photos
behind the compact dock, no passive text above it and a draggable right scrollbar. Dragging to the
queue's end loads the visible images after release and leaves the last row clear of the controls.
Dragging back near the top also succeeds. The final top gesture was interrupted by detected user
input; the subsequent observed top view contains loaded images. Automated drag coverage remains
separate from that interrupted native gesture. The window remains open in organization mode.

Production owners contain 1,877 lines in the screen (composition changes only), 73 in the viewport,
45 in the scroll surface, 23 in dock placement, 141 in dock controls, 29 in the mode notice and 53 in
viewer organization controls, with no inline tests. Dedicated interaction, review-widget and overlay
suites contain 237, 445 and 105 lines. No schema, bridge, dependency or source-policy changes are
introduced. Publication stays on the existing R4 branch; its push alone has no hosted workflow trigger.

## Shared gallery sorting in organization mode - 2026-10-25

Organization entry inherits the ordinary gallery sort and exposes the existing sort menu, with
`新加入优先` as the explicit admission-descending option. The three queues retain continuous layout.
Changing only the sort carries the visible image anchor through the shared query transition.
Continuation uses the active same-source sort and reanchors a handled logical group before finding
the next eligible group. A missing anchor cannot supply a cursor under a different sort.

Schema 57 adds scope sorting and generic bookmark boundaries without replacing decision tables.
Existing bookmarks retain admission-descending order. Archive version 3 includes these values;
versions 1 and 2 expand to their original admission order on read. Restored admission offsets change
only admission cursors. No source media reads, mutations or new dependencies are introduced.

The organization-filtered Rust run passes 241 cases, with two existing ignored diagnostics.
One root-consolidation fixture collided on its shared scan identifier under two test threads;
its serial rerun passes. Targeted checks cover all five sort choices, both directions, missing
dates, bounded paging, continuation after ordinary edits, reopen, order changes, migration rollback,
retained decisions and old/new archive compatibility. The strengthened date fixture initially lacked
the schema-required EXIF provenance; adding that evidence makes both ordering cases pass.
All 17 asynchronous bridge contracts and generated content hashes pass.

The saved-order owner has 91 production lines, continuation 110, the migration 109 plus three test
declaration lines, and archive compatibility 31. The existing archive codec has 266 production lines
plus three test declaration lines. The screen/header/mode have 1,882/600/376 production lines with
no inline tests; screen changes only compose the query. The migration registry retains 6,417 total
lines: 4,127 before its inline test module (including 52 historical test-helper lines) and 2,290 in
that module, and receives only migration dispatch. Catalog/model/layout Dart owners retain
589/548/432 lines with no inline tests. The Rust catalog facade and domain module retain 3,506/821
total lines, including existing test hooks/declarations; their changes are schema/enum dispatch.
Dedicated sort/migration/mode/unified-review suites have 126/89/457/278 lines.

All 31 focused Flutter cases pass across organization mode, unified review and query navigation.
Strict analysis of the 12 affected handwritten Dart files and the Windows Debug build pass.
The retained 10,000-file isolated client upgrades successfully and keeps its 72 logical groups and
existing progress counts. Native inspection confirms ordinary name sorting is inherited on entry,
the organization sort menu remains available, and selecting `新加入优先` changes the order while
retaining the prior visible image in the viewport. Ordinary menus omit this organization-only choice.
The test window remains open. Large-library performance acceptance remains with the existing R4 gate.

## Organization action labels and placement - 2026-10-25

Gallery selection and the detailed viewer place defer inside the existing organization More menu.
Batch ignore uses `忽略所选`; applied states use `取消稍后整理` and `取消忽略` across selection,
viewing and the album tray. Cancellation restores pending state without changing Favorites.
The menu retains the action observed when opened and rejects it if the group's current action
has changed. Conflict and in-flight save guards remain active.

All 44 focused Flutter cases pass: six organization-menu, 14 review-widget, 22 selection-action
and two unified-review cases. Coverage includes both stale menu actions, conflict-disabled
cancellation, receipt-bound advancement, failed-save retry and preserved Favorites. Strict analysis
and formatting of the eight affected Dart files pass. The repository formatting check reports
only the pre-existing, unrelated edit in `library_gallery_states.dart`; that file remains untouched
and outside this delivery. The Windows Debug build passes.

Native verification uses the retained generated library. The selection row has no direct defer
button, and its More menu shows `取消稍后整理` and `忽略所选`. Cancelling the one deferred group
changes pending from 65 to 66 and deferred from one to zero. The ignored-image viewer shows
`稍后整理` and `取消忽略` in its upper More menu. Undo restores pending/deferred to 65/one;
the five organized groups and one ignored group remain unchanged. The isolated window remains open.

The state-label, menu, viewer-action and selection-action owners contain 72, 159, 125 and 369
production lines with no inline tests. Their dedicated menu/review/selection/unified suites contain
216/455/573/281 lines. No schema, bridge, dependency or source-policy change is introduced.

## Organization header and positional rail - 2026-10-25

Organization controls occupy the second row of the upper-right action area. One dropdown displays
the current queue's icon and name before the labeled exit and continuation buttons. Selection retains this
row, and first-use state omits continuation. The former bottom dock is removed; notification
placement is unchanged. The shared annotated rail accepts an explicit scroll range without date
buckets and renders positional scrolling without date labels or markers, including ordinary name
sorting. Entry and exit animate row height and opacity over 180 ms, with continuous reversal,
retired input during exit, and immediate reduced-motion transitions.

The scroll surface owns the current query's drag lifecycle through Flutter's public
`ScrollPosition.drag` API. This preserves loaded-window pagination before the full wall layout is
available and cancels obsolete gestures on query/controller replacement or disposal. Material Slider,
checked popup items, TextButton, SizeTransition and FadeTransition continue to own input, focus,
accessibility and motion on the installed SDK.

All 58 focused Flutter cases pass across the annotated rail (9), gallery interaction (24), scroll
surface (2), sidebar motion (3), header motion (2), review overlays (2), review widgets (14), and unified review (2).
Coverage includes distant and near-top-to-top dragging in both wall layouts, stale callbacks,
viewport resize, short queues, enlarged text, first-use continuation, selection, dropdown admission
while busy, and a retained scroll rail after leaving name-sorted organization. Strict analysis
of the 15 affected handwritten Dart files and the Windows Debug build pass. The unavailable-image
integration helper uses the new control identity and scopes menu lookup to its popup; the full
integration suite is not rerun locally. The repository formatting check reports only the existing,
unrelated edit in `library_gallery_states.dart`; owned files pass formatting and whitespace checks.

Native verification uses the retained 10,000-file generated library. Both header rows remain visible
during selection; dragging to the bottom and then from near the top to the top displays images
without another wheel event. Continuation opens the saved image, and exit restores ordinary browsing.
The final dropdown displays all three labels and counts without truncation, updates its icon and
name after selection, and ordinary name sorting exposes the same positional rail with working drag.
The existing five organized, 65 pending, one deferred and one ignored groups remain unchanged.

The screen/header composition owners contain 1,883/587 production lines. Header-row layout, review
toolbar, scroll surface, annotated rail and viewport contain 106/93/148/454/61 lines, with no inline
tests. Dedicated scroll/interaction/sidebar/header-motion/overlay/review/unified tests contain
98/247/147/90/113/482/312 lines. No schema, bridge, dependency or source-policy changes are introduced.

## Sidebar action icon consistency - 2026-10-25

Album creation and More actions use the same themed icon size as source navigation, removing their
separate 20-pixel overrides. Scoped formatting, strict analysis and the Windows Debug build pass.
The retained isolated client shows the aligned action icons and opens the album More menu correctly;
no album membership, source files or navigation behavior changes.

## Header alignment and continuous geometry - 2026-10-25

The organization dropdown now shares the sorting/layout menus' dot marker and exclusive-selection
semantics. The wide header aligns the title with browsing/selection actions and the counts with
organization actions. Both sides and the gallery boundary interpolate their measured geometry over
260 ms. This replaces the previous minimum-height plateau, during which controls expanded before
the gallery began moving. Enlarged count text stays on one horizontally scrollable line instead of
changing the header height on mode entry. Exit immediately retires secondary input; interrupted
transitions start from the displayed progress, and reduced motion remains immediate.

The installed Flutter 3.44.9 implementation and official
[Table](https://api.flutter.dev/flutter/widgets/Table-class.html) and
[CustomMultiChildLayout](https://api.flutter.dev/flutter/rendering/MultiChildLayoutDelegate-class.html)
contracts were checked. Table supplies static content-sized rows, while a multi-child delegate
cannot derive its own height from its children. The four-cell header render owner supplies only
measured layout interpolation; existing Material buttons, popup routes, scroll views, focus,
semantics and animation controllers retain their framework ownership. The Material menu reference
remains the [official component](https://m3.material.io/components/menus/overview); its page requires
JavaScript, so capability evidence comes from the installed SDK and existing shared menu owner.

All 30 focused Flutter cases pass: header motion (2), actual-header alignment (2), review widgets
(14), sidebar/right-edge geometry (6), unified review (2), and gallery counts (4). The alignment
cases cover the real count wording at normal and doubled text scale, early gallery movement,
rebuilds during entry, interrupted reversal and return to the original height. Strict analysis and
formatting of seven owned Dart files pass. The repository-wide format check still reports only the
pre-existing unrelated edit in `library_gallery_states.dart`; that file remains outside this change.

The final Windows Debug build passes. In the retained generated-library client, the corresponding
header rows align during ordinary organization and selection; collapsing and expanding the sidebar
preserve the right action edge. The mode menu displays the dot and complete labels/counts, and
switching to deferred updates the selected mode. Entering and exiting organization returns to the
appropriate header and gallery geometry. Frame timing and interrupted reversal are verified by the
focused widget tests, not inferred from still screenshots. The original 72 groups and progress counts
remain unchanged; no organization decisions or original media were modified.

The header, animation, measured-row, shared menu-choice and review-toolbar owners contain
544/153/125/44/92 production lines, with no inline tests. Their dedicated alignment/motion/review
tests contain 128/90/481 lines. No schema, bridge, dependency, source-policy or queue-order change
is introduced.
