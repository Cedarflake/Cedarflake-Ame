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
