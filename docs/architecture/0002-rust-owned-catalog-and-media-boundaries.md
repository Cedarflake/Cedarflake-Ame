# ADR 0002: Rust-owned catalog and recoverable media boundaries

- Status: Accepted for validation
- Date: 2026-08-07

## Context

Ame must index a roughly 259 GB library containing wrong extensions, inaccessible paths, damaged
files, multiple volumes, and OneDrive-backed sources. A single file or native parser failure must not
terminate the complete scan or corrupt the last trustworthy catalog.

The observed Lap v0.3.0 reference run terminated twice at the same scan position. The external
reference snapshot used for this observation is commit
`ff8b144f628cb02d9b4ac0a7bd20d93a224810ab`; it remains outside Ame's Git history. The real library
also contains many files whose extension does not match their content. These are requirements for
failure isolation, not reasons to normalize or modify the source collection.

## Decision

Rust owns:

- domain entities and application use cases;
- root registration and filesystem discovery;
- structured task state, cancellation, retry, and progress;
- SQLite schema, migrations, transactions, and query windows;
- preview scheduling and cache policy;
- media and analysis adapter ports;
- source-state revalidation and structured issue reporting.

Flutter owns presentation and ephemeral interaction state. It receives bounded query windows and
task events through the bridge and never opens the catalog database directly.

The initial SQLite catalog and preview cache live in application-managed storage outside every
source tree. Durable catalog data, user decisions, previews, analysis caches, logs, and model files
remain separately managed storage classes.

File format handling must use observed content evidence where practical rather than trusting an
extension alone. Unsupported, corrupt, inaccessible, unavailable, or changed files produce
structured issues and do not fail the complete task.

Discovery recognizes only the admitted seven formats from a bounded content prefix, including
wrong, unknown, and absent extensions. Recognition is candidate admission, not proof of valid pixels.
Header inspection stays separate from demand-driven pixel decoding. Stable malformed headers are
terminal content evidence; access failures remain retryable. Negative evidence must retain the
observed source version and be revalidated before publication, just like positive observations.
A source that changes after rejection enters the existing change lane rather than restarting the
entire scan. Inspection policy identity changes when recognition or rejection semantics change.

Pixel decode failure retires only that demand's derived result and reports a structured failure.
Explicit retry and a later source generation can recover independently. A successful decode does
not certify original-file integrity: tolerant decoding, animation frames not requested for a static
poster, and bit changes that remain valid encoded data require distinct evidence.

Pure Rust parsers may initially run inside the application process when they return recoverable
errors and pass hostile-fixture tests. Native codecs, model runtimes, and parsers capable of process
termination must run in a supervised Rust worker process. Worker failure becomes a per-item or
per-batch issue, and the task resumes according to an explicit retry policy.

## Catalog publication

- A running scan writes staged derived state associated with a task identity.
- Completed state is published atomically.
- Cancellation or failure does not replace the last trustworthy published state.
- Scan abandonment obtains cleanup authority only by atomically retiring a running or paused
  scan. A late or duplicate terminal request cannot delete published locations, retained preview
  ownership, or cross-root handoff evidence. Historical residue belongs to explicit cleanup and
  migration owners, not a second abandonment of a terminal scan.
- Source identity and state are revalidated before derived results are published.
- Future schema changes use forward migrations with migration tests from the supported baseline.
  The prerelease baseline below retires earlier Alpha upgrade paths.
- Queries use bounded keyset windows rather than loading the whole library or relying on deep
  `OFFSET` pagination.
- Filesystem discovery is sorted deterministically per directory. Directory entries are staged in
  SQLite in bounded batches and consumed through keyset windows instead of retaining an extremely
  wide directory in memory. A running scan stores its request parameters, current directory,
  pending-directory frontier, last processed relative path, and progress counters every 128 visited
  entries.
- Staged locations and issues are idempotent under checkpoint replay. After recovery, every staged
  source is revalidated before the same atomic publication boundary is used.
- ADR 0024 supersedes the original automatic-startup recovery policy. A retained unfinished first
  import, whether persisted as `running` or `paused`, requires explicit Continue. Detachment before
  first publication retains that checkpoint; detachment of a replacement scan cancels its staging
  while preserving the completed baseline. Explicit cancellation, stale input, and completed tasks
  are terminal. Registration is not a published baseline or live execution authority.
- Retaining a traversal checkpoint requires proof that changes during the observation gap are
  covered. Without that proof, explicit continuation rebuilds the first inventory behind the
  observer-first boundary; it cannot publish partial membership from the old frontier.

### Supported prerelease baseline

Fresh catalogs are created at schema 84 in one transaction, with the current tables, indexes,
triggers and initial singleton state. Schema 83 remains the supported prerelease baseline and
upgrades transactionally to 84's classification display evidence under
[ADR 0034](0034-versioned-classification-contracts.md#category-contract). The upgrade validates the
baseline before changing it and preserves catalogs, user decisions and operation history.
Earlier Alpha schemas and future unsupported versions are rejected without resetting or deleting
the stored data. No retired historical migration chain or migration-backup mechanism runs at startup.

This deliberate Alpha reset retires upgrade paths for development formats before the supported
baseline. It does not retire recovery for interrupted current operations, organization archives,
source validation or future forward migrations. Storage settings use their current version 2 and
model installation settings use version 3 under the same fresh/current/unsupported distinction.
The shared default-directory transition is owned by ADR 0045; any one-time operator relocation
remains outside application startup.

Validate the fresh schema and initial records against an independently created pre-change current
catalog. Preserve current-format reopen, rejection without mutation, transaction rollback and
retained user-decision coverage. An executable that only supports schema 83 rejects an upgraded
catalog without changing it; rolling back the display behavior must keep schema 84 support.

### Catalog connection and checkpoint policy

`sqlite_catalog/connection_policy.rs` owns writable and read-only connection configuration.
Both connection types use a 32 MiB demand-driven page-cache budget through SQLite's
connection-local [`cache_size`](https://www.sqlite.org/pragma.html#pragma_cache_size).
The retained-library organization queries repeatedly revisit catalog and decision indexes; the
default approximately 2 MiB budget evicts needed pages during these reads. The larger budget
reduces that churn without retaining connections or query results. It is not an immediate
allocation or a process-wide memory ceiling: concurrent connections retain independent budgets,
and query working memory remains separate. The setting is not persisted in the catalog; removing
it restores SQLite's default on the next connection without a migration.
Writable connections use SQLite's [passive automatic checkpoint](https://www.sqlite.org/wal.html#performance_considerations)
at 256 WAL pages, approximately
1 MiB at the default 4 KiB page size. This reduces the accumulated checkpoint work that one
committing writer performs, at the cost of more frequent checkpoints. Existing FULL synchronization,
foreign-key enforcement and write admission remain intact; the checkpoint threshold grants no
additional publication or source authority.

An admitted foreground first import into a catalog with no published roots temporarily uses
32,768 pages, approximately 128 MiB. Complete-library measurements on constrained storage found
that the smaller window repeatedly rewrote database pages during this write-heavy phase.
`connection_policy/initial_import.rs` owns the scoped window, shared through that catalog's writer
admission. Every writer applies the current threshold before beginning a transaction, including
retained synchronization connections. Returning, failing, pausing or cancelling the scan retires
the window; subsequent writes restore 256 pages. Imports alongside an already published root
retain the ordinary policy. FULL synchronization and the existing scan checkpoints remain intact.
The importing connection also establishes a 32 MiB page-cache budget for this scope, matching the
ordinary connection budget; other connections keep their own settings. The typed connection scope
restores the previous setting on normal return, failure and unwinding. Cache allocation remains
demand-driven; this is a connection budget,
not a reservation or a process-wide memory ceiling.
The larger window trades less repeated database I/O for a longer individual passive checkpoint;
real-client import and interruption measurements remain required for this path.

The threshold is a scheduling trigger, not a hard WAL size limit or a wall-clock guarantee.
A transaction may cross it, and an active reader may prevent checkpoint progress. Preserve those
read snapshots and retry checkpointing on subsequent commits. Read-only connections retain their
short protocol-read deadline and query-only policy. No background worker, schema migration or
catalog rewrite is introduced. Reverting the connection-local threshold restores SQLite's default
without changing stored data or transaction semantics.

The synchronization runtime's retained poll connection disables SQLite's implicit last-connection
checkpoint using
[`SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE`](https://www.sqlite.org/c3ref/c_dbconfig_defensive.html#sqlitedbconfignockptonclose).
Its retirement releases handles without adding an unbounded checkpoint to the joined
shutdown deadline. FULL commits remain durable in the WAL, and a subsequent connection reads that
committed state. Ordinary connections keep SQLite's default final checkpoint, so initialization and
short-lived operations preserve their existing completion boundary. Commit-triggered passive
checkpoints and explicit maintenance remain available to all writable connections.
Retained WAL files are catalog data, not disposable cache;
catalog backup or relocation must preserve the database and WAL together or use SQLite backup.
Re-enabling the retained connection's default close policy restores final checkpointing without a
schema migration.

## Validation gates

- the catalog is created outside a selected source directory;
- fresh initialization matches the supported baseline; current data reopens and unsupported
  versions are rejected without mutation;
- cancellation leaves a coherent catalog and reports a terminal task state;
- wrong-extension, damaged, missing, inaccessible, and Chinese-path fixtures do not panic;
- preview generation is bounded and leaves source bytes unchanged;
- a failed or cancelled scan cannot masquerade as the last completed catalog;
- worker-process protocol and restart behavior are validated before a high-risk native parser is
  admitted.

## Consequences

- Presentation cannot bypass application use cases with direct SQL.
- Task and publication models are designed before large engine integrations.
- Some media formats may remain unsupported until a safe adapter is admitted.
- Process isolation adds packaging and protocol work when native codecs or models are introduced.

## Replacement strategy

Persistence and media adapters can be replaced behind Ame ports. Domain identifiers, application
contracts, migration guarantees, and user decisions remain stable. An engine-specific database or
cache format cannot become Ame's source of truth.
