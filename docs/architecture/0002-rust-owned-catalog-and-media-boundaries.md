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

Pure Rust parsers may initially run inside the application process when they return recoverable
errors and pass hostile-fixture tests. Native codecs, model runtimes, and parsers capable of process
termination must run in a supervised Rust worker process. Worker failure becomes a per-item or
per-batch issue, and the task resumes according to an explicit retry policy.

## Catalog publication

- A running scan writes staged derived state associated with a task identity.
- Completed state is published atomically.
- Cancellation or failure does not replace the last trustworthy published state.
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
- Only a persisted `running` task is eligible for automatic startup recovery. A `paused` task keeps
  its checkpoint private and requires an explicit resume action. Cancellation, detachment, stale
  input, and completed tasks are terminal.
- If the saved traversal position no longer exists, recovery becomes stale rather than restarting
  from an ambiguous point or publishing a partial catalog.

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
- A 130-image interruption fixture resumes from a persisted checkpoint and publishes every location
  exactly once. Flutter tests verify automatic startup recovery with restored progress.
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
