# Scan execution and source admission

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

- `application/scan_library.rs` remains the scan command facade. Scan execution, terminal source
  reconciliation, P0 handoff, and publication waiting live behind dedicated application modules.
  `scan_library/execution_registry.rs` owns process-local execution registration and revocable
  first-import capture leases. `library_synchronization/admission.rs` distinguishes a published
  baseline, a currently owned first import, and a root that requires first import. A persisted
  checkpoint is recovery data, never proof that an execution is still alive.
  `ports/scan_recovery.rs` exposes the two retained foreground recovery projections. Their
  `sqlite_catalog/scan_lifecycle/recovery.rs` implementation first converges obsolete scans in one
  transaction, prioritizing a paused initial import, then the newest initial import. It preserves
  published snapshots, retires obsolete authority/staging and updates pending locations after commit.
  The catalog-session host holds the existing idle/current-session checks around the whole operation.
- `scan_library/traversal.rs` owns directory/window iteration and its existing checkpoint,
  cancellation and progress order. `entry_processing.rs` applies one typed discovery outcome and
  reports either accepted work or detached observation. `file_preparation.rs` owns path/identity
  lookup, preservation-prior selection and metadata/preview reuse. Preservation evidence stays
  distinct from reuse evidence; source revision, metadata engine and artifact checks remain
  mandatory. The facade composes traversal, final validation and atomic publication. None of these
  boundaries adds per-file catalog reads or grants source mutation.
- `scan_library/root_selection.rs` owns registered-path precedence, unique published-directory
  identity recovery and explicit relocation intent. It holds the target namespace capability
  through a recovered or replaced binding's admission. `sqlite_catalog/scan_admission/root_binding.rs`
  owns current binding/identity uniqueness, destination conflict and old-namespace retirement
  within the transaction. `sqlite_catalog/scan_admission.rs` owns atomic source registration, old
  generation retirement, scan admission, namespace binding and queue enrollment. Its typed
  relocation input retains the expected root path/generation; registration is never a standalone
  SQL rewrite followed by an independently admitted scan. `library_scan_target.dart` preserves
  the selected root through pre-admission failure and dispatches through a separate relocation
  scanner contract. The primary session retires this intent after a Started event or refreshed
  catalog proof that the selected root owns the requested path; normal retry/resumption then uses
  that durable binding. `library_scan_source_admission.dart` owns the pre-run root facts, source
  refresh policy and retained read. Automatic recovery can recognize only a previously published
  root; explicit replacement must match its selected root ID, never a peer occupying the target.
  The primary lifecycle admits feedback only for the current run/task sequence. `library_scan_run.dart` distinguishes
  native stream completion from complete retirement of terminal source refresh and subscription
  cleanup. A subsequent retry or picker admission waits for that retirement; old terminal work
  cannot release the newer operation's admission. Existing controls remain in the primary lifecycle.
  `root_selection/coverage.rs` resolves an otherwise unregistered child against the nearest active
  parent. `local_files/directory_coverage.rs` proves its directory identity through the pinned parent
  using attribute-only, no-follow directory opens and rejects unavailable/reparse entries. The
  parent binding is rechecked before returning typed `AlreadyCovered`; this outcome does not create
  a scan, publish incomplete membership, or retire a legacy registration. Exact-path update and
  explicit/identity relocation keep precedence. The primary run retains terminal folder-selection
  ownership through stream drain and navigation, without reporting an import completion.
- `scan_library/root_consolidation.rs` owns complete first-parent import admission and held child
  directory proofs. Its typed plan crosses into `sqlite_catalog/scan_publication/root_consolidation.rs`,
  which rechecks the registration set and retires children within the parent publication transaction.
  `sqlite_catalog/root_unregistration.rs` owns the shared transactional retirement of queues, scan
  records, locations and unowned previews/assets. The explicit removal facade supplies its own
  transaction; parent consolidation supplies the publication transaction. The Dart
  `library_root_consolidation.dart` owner maps the latest query without overriding later navigation.
  `library_scan_completion_recovery.dart` retains pre-run source facts and resolves a drained stream's
  durable result. The primary session retains confirmation/refresh obligations through read failures;
  dismissal or a new import cannot discard them, and Retry performs only the outstanding read.
- `local_files/root_location.rs` owns attribute-only lookup of one retained Windows directory
  identity and the full namespace capability returned for a candidate. It performs no media
  enumeration, content read or other-volume search. `production/root_location.rs` owns one
  background attempt, per-binding admission, current-epoch cancellation and joined shutdown.
  The locator boundary is injectable for an unavailable-volume fixture without changing the
  actual missing-path, peer-publication or recovery assertions. `sqlite_catalog/root_location_recovery.rs`
  owns same-directory binding, active-location rebasing and the durable post-binding gap in one
  transaction. Its retirement owner handles old inventory/baseline and journal-claim dependencies;
  `retirement/payload_cleanup.rs` owns indexed logical-entry, candidate and frontier deletion pages
  within that same cancellable transaction. It grants neither a separately committed preparation
  state nor permission to delete raw source spools. Root-binding validation remains separate from
  retirement so stale evidence is rejected before payload work.
  Its write-attempt owner retires priority callbacks before rollback or final commit. Directory
  discovery cannot infer provenance for a destination assembled from multiple roots.
- `scan_resume_inventory.rs` owns retained first-import membership and current-run reuse lookup.
  Its roster is independent of published membership. `scan_staging.rs` owns buffered location
  admission, source-state capture, persistence and transactional acceptance of retained locations.
  A full 128-location buffer remains available to the next atomic progress checkpoint; admitting
  another location first flushes that full buffer, preserving its bound and retry ownership.
  Its `scan_staging/retirement.rs`
  owner retires affected rows and unreferenced assets in the caller's transaction. Foreground
  cancellation atomically retires task authority and removes at most 128 staged locations; terminal
  scan rows retain the remaining cleanup identity across restart. The reclamation application owner
  drains those rows through bounded adapter transactions before reclaiming free database pages.
  Each batch excludes published, running and paused scans and preserves referenced assets and
  handoffs. Failure rolls back that batch; foreground work can preempt between batches. Unobserved
  resume locations and authoritative recovery retain their synchronous transactional cleanup.
  Retirement uses a temporary 64-MiB pager-cache suggestion and restores the previous setting on
  success or SQL failure. The incremental-vacuum adapter consumes the statement through completion
  so its admitted page limit is actually executed; SQLite progress cancellation remains active. The
  scan-resumption owner preserves the checkpoint when `ScanResumeCoverage` proves the original
  opening boundary remains covered by the same volume, root identity and journal. Its transaction
  rechecks that exact persisted boundary before accepting the proof. Uncovered resumes reset
  directory coverage while retaining inspected images; traversal closes the retained roster before
  final file validation. The v33 migration and its schema/row checks live in
  `migrations/scan_resume_inventory.rs`; they do not widen the historical migration facade.
- `scan_checkpoint.rs` owns atomic buffered-location and progress persistence for checkpoint and
  directory-completion boundaries. It composes the staging transaction without publishing progress
  ahead of accepted locations or retiring an in-memory batch before commit. Writer admission,
  checkpoint status rules and current-directory authority remain unchanged.
- `ports/scan_staging.rs` and `ports/scan_publication.rs` expose retained inventory, bounded validation
  and atomic publication through Ame values and opaque session-bound rosters/proofs. Application
  finalization owns validation order, cancellation and live-change handoff; `sqlite_catalog/scan_repository.rs`
  binds these contracts to the existing catalog session. Temporary tables, payload revalidation,
  writer admission and transaction rollback remain in the SQLite owners below that binding.
- `ports/scan_admission.rs` exposes registered-root selection, namespace-bound scan admission, resume
  coverage and consolidation candidates. `sqlite_catalog/scan_root_repository.rs` binds these operations
  without splitting their transactions. `ports/source_publication.rs` exposes the separately held root
  and ancestor capability; application admission and publication retain it for their original scopes.
- `scan_publication/live_change_admission.rs` owns the distinction between unfinished and exhausted
  uncovered P0 debt. It shares retained-subtree eligibility and current capacity with the queue
  owner. Foreground publication cannot wait for a full P2 lane to drain while that foreground
  itself excludes P2 execution. Failure preserves the published snapshot and debt; existing strict
  retained recovery may transfer the original evidence once without restarting its retry history.
- `local_files/file_admission.rs` owns root-relative attribute admission and supported, terminal,
  and retryable file discovery after local-availability checks. The admitted handle supplies
  metadata, identity and revision without reopening its mutable absolute path. Its borrow remains
  live through classification; failure does not substitute evidence from another file. Final
  publication still revalidates the root-relative path. `scan_library/inspection_failure.rs` owns
  the shared retained-location and precise-retry checkpoint rules for discovery and decoder failures. Incremental terminal-media
  classification lives in `incremental_library_changes/terminal_media.rs`, not in the worker loop.
  Evidence-only completion does not create a gallery location. `rename_change.rs` owns paired-path
  composition while preserving the current lease's terminal-evidence boundary and both paths'
  source revalidation; it does not invent a second persistence contract.
- `local_files/file_revalidation.rs` owns expected-file-state comparison for staged publication
  and already opened preview sources. Windows path revalidation uses one no-recall attribute handle
  for identity, revision and final metadata, retaining placeholder admission and the existing
  identity-versus-revision failure classification. Size/time, identity and revision mismatches keep
  their ordered outcomes; the shared comparator does not acquire namespace or publication authority.
- `local_files/directory_enumeration.rs` owns publication-guarded directory cursors.
  Its `streaming.rs` owner retains the root-bound directory handle, fixed native buffer, duplicate
  detection and terminal/failed enumeration state; completion revalidates identity on that handle.
  Metadata-inventory cursors own a cloned publication capability until completion or drop;
  authoritative file visits borrow the namespace and revalidate the current relative directory
  binding after exhausting the stream. Capacity, cancellation and failure retire the cursor without
  claiming complete coverage. The application owns the bounded, sorted authoritative path set.
  `records.rs` decodes
  checked Windows directory records into Ame metadata, including UTF-16/component bounds, attributes,
  placeholders and identity/revision admission, without opening source content. The facade retains
  the metadata-inventory iterator contract, and `scan_enumeration.rs` shares the anchored directory
  open. Eager sorted-frontier helpers and enumeration instrumentation remain test-only. Raw record
  parsing, native page lifetime and application ordering remain separate from recovery batching.
- `incremental_library_changes/evidence_policy.rs` owns precedence between dirty source events,
  conservative inventory renames, metadata-engine replacement and rebuilding invalidated metadata.
  An unchanged metadata recheck retains the already advanced source generation; dirty or changed
  source evidence retains invalidation priority. `change_queue/metadata_rechecks.rs` owns bounded
  admission and its rotating cursor, while `production/metadata_status.rs` keeps unadmitted metadata
  work visible without overriding source failures. These owners reuse existing guarded P2 execution
  and catalog-delta publication rather than adding another media worker or analysis store.
- `incremental_library_changes/preparation_catalog.rs` owns the bounded read set used to decide
  whether a prepared delta survives an unrelated catalog revision. It preserves every original
  path and global identity lookup, including negative results and ordered catch-up lineage.
  `preparation_rebase.rs` requires matching root context, complete catalog observations, and current
  source-version evidence before reuse; otherwise it retires the old proof before preparing again.
  Every observed file is revalidated even when it produces no mutation. An absent path acquiring
  terminal media is a new observation, not equivalent absence. Reuse never replaces the final
  transaction's revision, root, lease, or preview guards.
- `incremental_library_changes/authoritative_path_set.rs` owns guarded path-set preparation and
  publication. Only a single exact `ConsistencyAudit` path reconciliation without a previous path
  retains the bounded read set and may rebase twice through the shared current-preparation proof.
  A changed proof keeps its original durable retry; cancellation returns its exact lease. Root,
  subtree, paired-path and other path sets retain fixed-revision publication because per-item reads
  do not prove their complete scope membership. No rebase reparses a broader lease as a single path,
  repeats media inspection, or weakens the final transaction's publication authority.
