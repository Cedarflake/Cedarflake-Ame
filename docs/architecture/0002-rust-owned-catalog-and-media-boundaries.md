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
- Schema changes use forward migrations with migration tests.
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

### First-import inspection scheduling

An unlimited explicit fresh import without active locations may overlap source discovery and metadata reads
with ordered catalog writes. `scan_library/inspection_prefetch.rs` owns one scoped reader and an
eight-result bounded queue per existing directory window. Only that reader opens source content;
the importing thread retains prior selection, asset identity, issue delivery, progress checkpoints
and persistence. Results remain paired with the observed file state and keep the existing metadata
engine and final publication validation. Resume and published-library scans retain prior-driven
serial inspection so reusable metadata does not cause unnecessary reads. Requests with explicit
item or entry limits also remain serial and cannot prefetch beyond their requested bound.

Pause, cancel, detachment, limits and failure close the queue and join the reader before retiring
its scan scope. A full queue cannot prevent retirement. A reader panic is an explicit scan error,
never the end of a complete inventory. An in-flight source read retains the existing synchronous
read limitation; stopping prevents subsequent admission rather than interrupting that OS read.
No schema, source authority, dependency, engine result or publication policy changes.

### Catalog connection and checkpoint policy

`sqlite_catalog/connection_policy.rs` owns writable and read-only connection configuration.
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
The importing connection also borrows a 32 MiB page-cache budget for this scope; other connections
keep their own cache settings. The typed connection scope restores the previous setting on normal
return, failure and unwinding. Cache allocation remains demand-driven; this is a connection budget,
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
- migrations work from an empty database and across every committed schema version;
- cancellation leaves a coherent catalog and reports a terminal task state;
- wrong-extension, damaged, missing, inaccessible, and Chinese-path fixtures do not panic;
- preview generation is bounded and leaves source bytes unchanged;
- a failed or cancelled scan cannot masquerade as the last completed catalog;
- worker-process protocol and restart behavior are validated before a high-risk native parser is
  admitted.

## Current validation evidence

- Schema v3 creation and forward migration from both v1 and v2 are covered by Rust tests that
  preserve an active published location and initialize the catalog revision.
- Multi-root publication, stable asset identity across a root rescan, revision-protected keyset
  queries, stale-cursor rejection, and a 1,025-location multi-page walk are covered by SQLite
  adapter tests.
- Cancellation, stale-source rejection, corrupt-file isolation, wrong-extension decoding, Chinese
  paths, external preview placement, and source-byte preservation are covered by Rust tests.
- Missing-source revalidation prevents publication, exclusively locked images are isolated as
  structured open failures, and a source path longer than 260 characters is indexed without
  changing its bytes.
- Windows offline and recall attributes are recognized before content access; an actual local file
  marked offline is skipped without decoding or changing its bytes. The runner manifest is also
  explicitly long-path aware.
- The isolated Windows integration test imports two controlled roots through the production native
  picker, reconstructs Flutter state from SQLite twice, and verifies both roots and source files.
- Schema v4 persists scan parameters and checkpoints. Migration tests cover v1, v2, and v3; an old
  uncheckpointed running task is explicitly made unrecoverable rather than guessed.
- Historical schema-v4 evidence includes a 130-image interruption fixture that resumed a persisted
  checkpoint and published every location once. Its automatic-startup Flutter behavior is
  superseded by ADR 0024's explicit-continuation contract, not current startup acceptance evidence.
- A pause/resume fixture proves that pausing does not publish or enter the automatic recovery query,
  and explicit resume completes the same task without duplicate locations. Flutter restores paused
  state without starting it and exposes pause/resume in the upper action area.
- Schema v5 adds the persisted current directory and pending frontier. A v4 migration refuses to
  invent frontier state for old running or paused tasks, while a deep-tree interruption fixture
  resumes the current directory and publishes 130 unique locations.
- Schema v6 adds a durable directory-entry snapshot. Enumeration is written in 256-entry batches and
  consumed in 256-entry keyset windows; a 1,025-entry fixture proves bounded traversal without gaps
  or duplicates. Active v5 tasks are made unrecoverable because an absent entry snapshot cannot be
  reconstructed safely.
- Schema v7 records `pending`, `ready`, and `failed` preview state with structured failure evidence.
  Existing v6 artifacts migrate as ready without losing their paths.
- Schema v8 records metadata engine identity and version plus optional normalized capture-time
  evidence. Existing v7 locations migrate as explicitly unanalyzed, compatible evidence is reused
  only for an unchanged source, and an old engine identity forces reinspection.
- Schema v9 records optional, versioned file-identity evidence. Same-volume rename and in-place edit
  reconciliation preserve logical asset identity, changed derived state is invalidated, replacements
  cannot inherit the prior asset merely because the path matches, and terminal snapshots remove
  stale derived locations and orphan asset rows.
- Discovery uses bounded header inspection rather than full pixel decoding. Flutter's lazy grid
  requests visible previews through an Ame-owned controller queue with at most two active decodes,
  removes queued requests when tiles leave the rendered area, and exposes explicit retry after a
  failure.
- Missing rebuildable preview artifacts become pending when the catalog is loaded. Windows
  integration deletes a generated artifact, reconstructs the application state, and verifies that
  the visible tile recreates it without changing source media.
- Catalog loading reports each root as available, missing, inaccessible, or offline using only root
  metadata so availability checks do not enumerate or hydrate the source.
- Configurable storage paths and an atomically enforced preview budget are validated in ADR 0005.
  Capture-time parsing, bounded raw evidence, and malformed-metadata isolation are validated in ADR
  0006. Incremental Windows path reconciliation and its focused native safety boundary are validated
  in ADR 0007. Verified storage migration and high-risk worker-process recovery remain validation
  gates for later slices. This record therefore remains `Accepted for validation`.

## Consequences

- Presentation cannot bypass application use cases with direct SQL.
- Task and publication models are designed before large engine integrations.
- Some media formats may remain unsupported until a safe adapter is admitted.
- Process isolation adds packaging and protocol work when native codecs or models are introduced.

## Replacement strategy

Persistence and media adapters can be replaced behind Ame ports. Domain identifiers, application
contracts, migration guarantees, and user decisions remain stable. An engine-specific database or
cache format cannot become Ame's source of truth.
