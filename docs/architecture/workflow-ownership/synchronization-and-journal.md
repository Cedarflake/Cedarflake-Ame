# Synchronization, journal and recovery

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

`windows_usn/parsing.rs` owns checked V2/V3 wire decoding and typed record admission. Redacted
filenames remain absent identity evidence; the existing named-record facade rejects them before
historical path reconstruction. Framing, identity widths, UTF-16/component validation and complete-
page failure remain one boundary, with no second decoded page or per-record cloning. The parent
`windows_usn` module retains named reference histories and path reconstruction.

The superseded direct-desktop USN adapter and its all-roots catch-up orchestrator are retired.
Shared parsing and historical-parent reconstruction remain broker dependencies with their own
regressions. The native identity reader owns case-sensitive candidate translation; the persistent-
journal application retains failure admission and cancellation/publication authority. Historical catalog
checkpoint, lineage and migration contracts remain intact; removing an execution path grants no
schema or retained-data cleanup authority. The R2c-M entrypoint owns the retained metadata-only
acceptance wrapper and its process/path guardrails; R2c-H is historical evidence only.

- `journal_broker/windows/production_host.rs` owns production listener readiness and joined shutdown.
  Its lifetime owner linearizes connection admission against idle retirement and holds each lease
  through request drain and disconnect. `identity_query_access.rs` owns only the verified service's
  process/token query grants. The service activation owner handles bounded SCM state transitions;
  the transport connection owner admits one fully reauthenticated handover only after proven process
  retirement. Protocol dispatch and partial I/O remain in their existing owners.
- `application/library_synchronization/production.rs` remains the priority and worker-lifecycle
  coordinator. `production/runtime_lifecycle` owns the process registry: `state` holds the epoch,
  typed start tickets, admitted stop-fence ledger and retained runtime; `admission` linearizes
  ownership changes; `operation` admits results and retains panicked owners; `retirement` drains
  that same owner within its original deadline. Its facade exposes reserve/start/poll/stop and
  first-import ownership without exposing mutable registry state. Constructors and polling remain
  outside the registry mutex. Timeout, panic and late completion retain the original resource and
  epoch authority until retirement is proven. Lifecycle tests sit beside this boundary, while
  controlled source/journal resource fixtures have a shared test-only owner.
  `journal_lane` owns the root/volume rotation cursors and read/drain alternation. It selects a
  typed `JournalWork` after the coordinator's unchanged Live/stopping gate: ready Journal drain,
  recovery admission, closing boundary, opening boundary, then a capacity-admitted volume page.
  Cursor updates retain their original admission point even when worker startup fails. `journal_work`
  owns worker creation, cancellation, result retirement and the shared shutdown deadline; its page
  owner retains opaque registrations through the existing bounded capture operation under the
  [volume resource contract](native-composition.md#journal-volume-resources). Status projection
  and cross-lane priority remain in their existing owners.
  Readiness probes, volume/recovery selection, retained-source pruning and journal status projection
  consume persistence ports; application owners retain priority, rotation and retry decisions.
  `journal_work/fallback` owns the failed-read budget and root-boundary isolation across workers;
  successful reads and superseding checkpoints retire that evidence. `persistent_journal/recovery_failure`
  owns atomic failure admission. `persistent_journal/metadata_fallback` owns the independent receipt
  and retains the journal cursor; `journal_fallback_session` owns one retry at the next process startup
  and a fresh pass after interrupted verification. `metadata_inventory/recovery_journal` selects the
  journal or independently admitted inventory completion proof. The schema-46 owner validates that
  receipt separately from retained historical journal windows.
  `recovery_lane` owns per-root retry timing and fair root rotation. Its typed selection preserves
  candidate-drain, legacy-capacity-debt and recovery-control precedence after the coordinator's
  higher-lane gate. Journal capacity debt restricts the eligible roots without admitting a new control
  page. Success clears only the completed root's retry history; source and task lifetimes remain
  independent of scheduling state.
  `recovery_sources` owns retained inventory sources by change identity and admits a source only
  while its persisted authority/run remains current. A typed transfer binds the original change ID
  to one shared take operation; failed worker startup restores that same owner. Catalog-session
  validation precedes handoff, and pruning resolves all authority reads before removing stale
  entries. Completed pages publish their retained source through the same owner.
  `recovery_work` owns the admitted Recovery worker's cancellation, phase, result and join lifetime.
  Its automatic and drain implementations accept an immutable root binding, bounded policy and
  `CatalogLaneSession`; their lane-local catalogs implement the required persistence ports.
  The coordinator retains slot admission, catalog-session validation, result publication and retry
  scheduling. Automatic work takes the retained source before opening its lane, revalidates the
  root before processing, and restores the transfer if thread creation fails. Timeout retains the
  same task and shutdown uses the existing absolute deadline. Optional search-index selection follows
  the [indexed-recovery composition contract](native-composition.md#indexed-recovery-resources).
  Session replacement follows the admitted runtime lifecycle. A successful retirement allows a
  new factory connection; a cached close failure retains the old owner and prevents construction.
  There is no separate, unused runtime reconnect path with weaker close-failure semantics.
  Cross-lane integration suites live under `production/tests` by startup, first import, baseline,
  live-gap lineage, recovery authority/status/fairness, priority admission, inventory paging, capacity
  debt and publication guards. Shared controlled fixtures remain in their test-only parent; tests
  specific to a worker or lifecycle stay beside that owner.
  Journal baseline opening and closing use different typed work items in
  `journal_baseline.rs`; an opening authority is either an existing root or a first import with a
  required scan identity and start time, so mutually exclusive fields cannot form an invalid job.
  `journal_baseline/opening.rs` owns opening probe admission, capability/baseline publication and
  its typed recorded/retired outcome. A revoked first-import lease retires only its own work;
  database and broker failures remain errors, and a committed receipt survives later control.
  Opening and closing capture consume `JournalBoundarySource`; the native root lease and protocol
  conversion follow the [boundary resource contract](native-composition.md#journal-boundary-resources).
  `FirstImportReadiness` supplies pristine-scan and inactive-root reads from `scan_lifecycle`.
  `JournalBaselineAdmission` acquires the first-import publication permit inside the existing write
  transaction and retains it through commit. `JournalRecoveryRepository` checks retained inventory
  and resumes the original cursor only after its recovery authority retires; application still owns
  startup retry and interrupted-verification policy.
- `production/live_work.rs` owns an admitted Live worker's thread, result, cancellation and bounded
  retirement. `live_work/batch.rs` owns continuation across ready authoritative scopes, bounded by
  the queue lease limit and a monotonic admission quantum. Each scope reloads root/generation and
  catalog revision; its existing namespace, lease and publication guards remain authoritative.
  Partial committed results survive a later terminal error. A retry, deferral or supersession ends
  the quantum, and ordinary path fallback never extends a partially consumed scope batch. Root
  choice and rotation remain in the coordinator; a presentation poll is not a per-scope execution
  permit. The quantum stops new admission rather than promising interruption of a running scope.
- `production/root_location.rs` owns missing-root selection, binding-specific retry state and joined
  discovery-worker retirement. It consumes the located namespace capability and `RootLocationRepository`;
  `sqlite_catalog/root_location_recovery.rs` retains the complete preemptible recovery transaction.
  Native locator selection follows the [root-location composition contract](native-composition.md#root-location-resources).
- `library_synchronization/observer_handoff.rs` owns the uncommitted observation plan, partial
  capacity publication, and its opaque writer reservation through `ports/LibraryChangeIngress`.
  The runtime cannot overwrite that plan or claim freshness before submission. Adapter-owned
  `change_queue/ingress.rs` binds reservation identity and owns zero-wait transaction attempts;
  `write_admission.rs` retains ordering independently from an active permit. Production
  `catalog_scheduling.rs` consumes the readiness, admission and dispatch ports for synchronous
  follow-up writes and prevents same-poll self-waiting behind any retained ingress reservation.
  Worker retirement and status projection remain outside that write-admission boundary.
  `poll_catalog::WorkerCatalogSession` owns the cached worker proof and deferred cleanup connection
  factory under the [catalog resource contract](native-composition.md#validated-catalog-session).
  `inventory_cleanup` consumes that factory and `MetadataInventoryRepository`, retaining its existing
  pre/post-acquisition cancellation checks, idle delay, contention backoff and joined retirement.
  Poll preflight consumes `SynchronizationMaintenance`. The SQLite binding in
  `sqlite_catalog/synchronization_maintenance.rs` retains legacy automatic-scan retirement and calls
  the adapter-private media/metadata recheck owners. Their bounded selection, cursor advancement,
  nonblocking Recovery admission and transaction-local revalidation remain unchanged. Application
  retains operation order, the once-per-runtime legacy-retirement flag and its timestamp refresh.
- `sqlite_catalog/persistent_journal.rs` remains the journal facade. Root-unregistration lineage and
  cleanup live in `persistent_journal/root_unregister.rs` so removal cannot accidentally discard a
  surviving root's cross-root rename evidence.
  `baseline_lifecycle.rs` owns admission and closing-capture transactions; `baseline_state.rs`
  owns baseline decoding and coverage reads. `volume_publication.rs` retains the complete volume
  transaction, calling its range/checkpoint, pending-rename and cross-root-lineage owners without
  passing commit authority. `completed_history.rs` terminalizes ranges and cleans only closed proof
  clusters inside the caller's transaction. The facade retains the port and existing recovery
  owners; extraction does not introduce a second transaction or reorder durable publication.
- `sqlite_catalog/change_queue/root_retirement.rs` owns queue-generation retirement and the
  separate proof for retiring removed-root recovery authority. Cancellation and namespace
  replacement cannot acquire removal authority while a root or run remains. The removal
  transaction and `migrations/retired_root_authority.rs` share the same strict durable proof;
  compatibility repair preserves history, validates the complete catalog, and rolls back on failure.
- `sqlite_catalog/change_queue/readiness.rs` implements `ports/LibraryChangeReadiness` for read-only
  lane eligibility, recovery readiness, pending-metadata status and journal headroom. Its typed query
  context binds root, generation, time and policy while preserving
  the original input-validation requirements of each operation. Eligibility filters exclude
  terminal queue history before evaluating due work; retry, expired-lease, recovery-affinity and lineage checks remain part of
  their original query. Readiness is a scheduling hint, never a replacement for transactional
  lease admission. `change_queue/metrics.rs` separately owns exact observation totals: terminal
  records are counted without evaluating active-state projections, in the same SQL statement and
  snapshot as active counts and failure evidence. Neither owner depends on one hard-coded index
  name or discards retained history to reduce observation cost.
  `migrations/foreground_queue_index.rs` owns the v34 optional active-origin access path and its
  atomic upgrade. Readiness retains both lane authority and the guarded origin constraint; the
  migration coordinator only dispatches the step and retains its existing compatibility checks.
- `ports/LibraryChangeDispatch` exposes lane-specific authoritative/control leases and bounded
  compaction of unowned legacy recovery controls. `sqlite_catalog/change_queue/dispatch.rs` binds
  these operations to the existing lease selection, Live-gap transfer and recovery transaction.
  Root-generation checks, candidate/authority exclusions, evidence merging and atomic retirement
  remain inside persistence; worker cancellation and completed-result accumulation stay in application.
- `sqlite_catalog/metadata_inventory/lifecycle.rs` owns inventory-start and terminal-cleanup
  admission. Restoring an existing run reads its frontier and active-root evidence in one short
  read snapshot; a missing run ends that snapshot before writer admission and epoch allocation.
  Empty cleanup is a read-only decision. A positive hint never carries deletion authority into the
  later transaction: that transaction selects again. Caller-owned transactions cannot be silently
  reused or ended. Raw-spool retirement remains a separate ownership obligation, not proof supplied
  by the logical-entry cleanup budget.
- `sqlite_catalog/metadata_inventory/spool_initialization.rs` owns source initialization and
  incomplete-directory reset. Its write transaction revalidates the persisted run identity and
  status, active root, exact current queue lease, and matching unretired recovery owner before
  creating or resetting raw storage. A caller's earlier `Running` snapshot is not authority after
  cancellation or lease handoff. Normal progress fields are not immutable identity, and a current
  continuation may still reset incomplete directories while preserving completed ones. This
  initialization returns the adapter-owned execution identity used by later source operations.
- `sqlite_catalog/metadata_inventory/spool_execution.rs` owns the immutable run request and exact
  current queue lease for raw source access. Readiness, directory selection, and ordered paging
  share authority validation and their data reads in one short, independently owned read snapshot.
  Directory start, append, and completion revalidate that same execution inside their write
  transaction. Caller-owned transactions cannot supply or receive implicit read ownership.
  `spool_read.rs` and `spool_write.rs` own those queries and mutations; the inventory facade does
  not keep duplicate run-id-only raw APIs. A retained application continuation explicitly accepts
  its new lease through the source port without resetting its iterator or completed directories.
  Failed rebinding preserves the previous token, which remains subject to current authority checks.
  `DurableLocalMetadataInventory` packs small directories into one source quantum, bounded by both
  entry reads and directory opens. Each directory keeps its own opening/closing identity check and
  atomic persistence; cancellation is checked between entries and directories. Completed directories
  remain reusable after source reconstruction, while an incomplete directory follows the existing reset.
  Active enumeration checks admission before advancing its iterator and again before committing;
  a failed attempt is discarded by the application before reconstruction. No database transaction
  spans source I/O. This execution fence does not replace generation-specific delayed retirement.
- `sqlite_catalog/spool_retirement.rs` owns monotonic retirement of run-bound raw observations.
  Schema v32 separates their storage lifetime from run/root/recovery-authority lifetime. Retired
  headers carry historical binding only, never execution, replay, or absence authority. Parent
  retirement/deletion and storage retirement share a transaction; worker pause and valid lease
  handoff do not retire storage. Reopening an interrupted raw enumerator marks only its incomplete
  directories as `resetting`; source preparation clears a bounded page per turn before returning
  them to `pending`. Completed directories and the initial subtree observation remain intact, and
  stale leases cannot reset or consume either generation. A recovery authority may bind its next
  epoch while earlier storage is still retired; uniqueness applies only to executable storage, not
  historical headers. Physical cleanup selects again under its own bounded transaction,
  removes entries before empty directories and headers, and includes historical NULL-parent
  observations without inventing missing source authority. It never drains the full debt inside
  completion, cancellation, removal, or a single application call. Restart and idle maintenance
  retain responsibility for later batches. The v31-to-v32 migration copies legitimate active raw
  data before replacing foreign-key ownership, validates the complete resulting catalog before
  commit, and rolls back on failure; its one-time schema-copy cost is separate from runtime batch
  bounds. Current implementation and verification of this decision remain roadmap evidence.
- `metadata_inventory/spool_cleanup.rs` and `candidate_cleanup.rs` own bounded physical selection.
  Entry queries select a limited owner set before indexed payload paging; empty-directory/header
  selection limits inspected metadata before testing emptiness. A final SQL `LIMIT` after a payload
  sort is not a bounded-work contract. The empty-debt hint uses the schema-owned partial initial-
  observation index and short-circuits retired storage; idle maintenance cannot traverse active raw
  file payload. `cleanup_transaction.rs` owns nonblocking maintenance
  admission, zero SQLite busy wait, read/write progress interruption, and callback retirement before
  rollback. `ports/InventoryCleanupControl` carries read-only runtime cancellation; it does not grant
  source or execution authority. `production/inventory_cleanup.rs` retains the single batch worker
  and later cleanup obligation independently of roots, including idle rechecks and the existing
  absolute shutdown deadline. Cancellation preserves committed debt for a later owner.
- `sqlite_catalog/change_queue/terminal_cleanup.rs` owns terminal queue retention and deletion.
  Candidate ownership, live-gap consumer references, and unretired recovery authority exclude a
  row before the batch limit, alongside journal and active-scan lineage. A terminal status is not
  deletion authority over retained dependents. Their owning lifecycle retires those references;
  queue cleanup must not discard them to avoid a foreign-key failure.
  Baseline history is retired as a per-root/generation prefix: a retained earlier window protects
  its successor from deletion, including when the same batch releases the earlier window's last
  dependency. Later batches advance the prefix without leaving a historical journal transition
  unprovable on reopen.
- Cancelled lease return belongs to `sqlite_catalog/change_queue/lease_deferral.rs`. It owns bounded
  batch admission, exact lease-generation classification, attempt refunds, and atomic rollback;
  incremental workers submit one typed batch rather than orchestrating per-lease transactions.
