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
current schema version. Hosted checks for the new implementation remain pending publication.
