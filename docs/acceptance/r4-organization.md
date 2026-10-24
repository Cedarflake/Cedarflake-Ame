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

The association owner has 77 production lines, current-evidence capture 75 and matching 93; none has
inline tests. Dedicated matching and cost tests have 312 and 80 lines. The catalog archive facade has
182 production lines and still delegates each responsibility. The catalog schema and archive format
remain unchanged. Actual application must recompute matches under its write transaction and verify
the inspected checksum. Atomic restore, durable unmatched records, newer-decision/conflict handling,
recovery settings and explicit cleanup remain outstanding, as do retained-library measurements.
