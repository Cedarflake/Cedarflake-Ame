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
not establish hosted verification for the current changes.
