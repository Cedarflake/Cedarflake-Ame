# Catalog, queries, maintenance and migrations

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

- `adapters/sqlite_catalog.rs` remains the catalog and write-admission facade. Atomic replacement-
  scan publication and its bounded identity reconciliation live in
  `sqlite_catalog/scan_publication.rs`. Its `validation.rs` owner captures the fixed temporary
  validation roster and verifies typed per-item outcomes against the complete staged/live payload
  in that same publication transaction; the facade does not carry opaque exception flags.
  `ports/scan_publication_control.rs` exposes read-only scan interruption evidence without exporting
  application command values. `scan_publication/transaction.rs` owns attempt-scoped priority
  preemption, SQLite progress interruption, callback retirement, and the transaction boundary;
  application publication owns retry admission and the pause/cancel/suspend terminal policy.
  `connection_policy/initial_import.rs` owns the temporary checkpoint window for an empty catalog's
  admitted first import. The scan owns its lifetime, the catalog writer admission shares its state,
  and each writable transaction applies the current threshold without changing writer priority or
  durability. Retained connections cannot keep a retired import policy.
- `location_row_write.rs` owns the asset/location row binding and insertion SQL inside the caller's
  transaction. Its typed write carries the already-admitted scan, root, source revision and source
  generation, and reuses the connection's bounded prepared-statement cache for the location upsert.
  Statement and binding lifetimes end before commit; rollback and optional-value replacement retain
  the caller's atomicity. Identity-group comparison, generation allocation and active-alias
  invalidation remain upstream and cannot be bypassed by row serialization.
- `source_reconciliation.rs` owns captured source-state admission, generation allocation and
  physical-alias propagation inside that same write transaction. A staged observation updates only
  aliases in its scan and retains their independent namespace-verification roster; live observations
  retain their active and unfinished-scan scope. Staging cannot invalidate active preview ownership.
  `migrations/retained_source_state.rs` admits only shrink-only retirement of an older unverified
  first-import alias with a newer verified same-asset sibling. Complete validation before commit
  preserves rejection and rollback for every other inconsistent catalog state.
- `identity_group_state.rs` owns physical-identity state aggregation and consistency reads for an
  active publication or selected scan. Staged state takes precedence only when that scan contains
  the identity; an empty result falls back to the active publication. The connection's bounded
  statement cache reuses compiled SQL, never observed state. Each call executes against its current
  transaction or fresh read snapshot, releases the statement before returning, and retains the
  existing unknown-revision and conflicting-alias rules. Generation assignment, captured-state
  comparison and alias invalidation remain separate upstream decisions.
- `catalog_delta/terminal_evidence_scope.rs` owns normalized terminal-media evidence containment
  and bounded retirement for completed path/subtree/root leases. The delta transaction keeps lease,
  namespace, revision, rollback and source revalidation authority. Too much retained evidence
  returns the existing inventory requirement instead of expanding an unbounded cleanup.
- `sqlite_catalog/write_admission.rs` owns writer ordering, preemption notification, waiting
  registration, and permit retirement. Priority order remains authoritative across lanes; within
  one priority, admission follows registration order rather than condition-variable wakeup order.
  Nonblocking acquisition cannot bypass a registered same-priority waiter. A waiting registration
  retires its own identity on timeout or unwind, without revoking an active permit or consuming
  another waiter's position. Callbacks run outside the admission mutex; completed-write epochs
  advance only when the admitted permit retires.
- `change_queue/lease_write.rs` owns recovery contention across readiness selection and lease
  publication. Its zero-busy-wait connection scope starts before the first selection read and
  restores configuration on every return or unwind. Busy or locked selection leaves durable work
  untouched for a later poll; other errors remain failures. Readiness still precedes writer
  registration, the bounded FIFO admission remains intact, and the transaction revalidates the
  selected root, generation and authority before publishing a lease.
- `domain/gallery_time_snapshot.rs` owns semantic month resolution, display-order fallback and
  bounded target-window selection. `sqlite_catalog/gallery_time_snapshot.rs` resolves that intent
  and reads the timeline, roots and page inside one `GalleryReadTransaction`. The target ordinal
  remains distinct from the centered page start, so changed row grouping retains a loaded prefix.
  Ordinary time anchors and paging cursors keep their strict revision/query authority; this read
  grants no source access, catalog mutation or stale-cursor permission.
- `sqlite_catalog/gallery/time_anchor.rs` owns strict month-predecessor lookup within the caller's
  read transaction. Typed query/anchor inputs retain the month predicate, ordered ties, filters and
  complete asset validation. Capture/creation dates add equivalent bounds on the existing ordered
  date expression; unknown dates retain their NULL rule. Month bounds narrow database work without
  granting source access, changing schema or weakening revision/query admission in the read facade.
- `sqlite_catalog/gallery/location_anchor.rs` owns preferred-location and stable-asset resolution.
  `anchor_position.rs` owns surviving-ordinal fallback, clamping and the centered window's
  predecessor inside the caller's read transaction. A successful ordinal read retains its proven
  position; counting is needed only for an identity-resolved position or an out-of-range fallback.
  A known capture/creation date excludes missing dates from its preceding range and adds an
  inclusive ordered-date bound; the complete cursor predicate still decides exact ties.
  Unknown-date and other ordering policies remain distinct. Both owners share query filters,
  published sources and order expressions rather than duplicating membership or source policy.
- `sqlite_catalog/gallery/filters.rs` distinguishes logical organization membership from physical
  source/folder/search scope. Point-anchor filters establish membership before expanding eligible
  groups. `organization_groups.rs` applies the same membership proof to the requested active copy
  before selecting its scoped representative; a copy outside the physical scope can still resolve
  through another member. Eligibility retains the shared empty-decision and published-source relation.
- `sqlite_catalog/gallery/organization_groups.rs` owns scoped organization-group selection,
  preferred representatives and group-anchor resolution. Its group query retains physical member
  counts for chronological timeline aggregation; page filters and continuation reuse the same
  scope and representative rules. Compatible bindings and canonical record expressions remain in
  `gallery/organization.rs`. All reads retain the caller's snapshot and published-source boundary.
- `sqlite_catalog/gallery/organization_counts.rs` owns shared progress aggregation across queue
  states inside the caller's collection and physical scope. It reuses canonical record joins for
  both group identity and disposition without resolving a preferred representative. Queue filtering
  and admission ordering remain in `organization_queue.rs`; read transaction ownership is unchanged.
- `catalog_reclamation/execution.rs` owns page-reclamation sequencing through `CatalogSpaceRepository`
  and `CatalogCapacity`; retry helpers consume the same repository contract. Its native resource
  factories follow the [composition contract](native-composition.md#catalog-reclamation-resources).
  Database page reclamation remains a separate application operation and persistence adapter. It
  is lower priority than foreground and ordinary recovery publication, is preemptible at SQLite
  progress boundaries, and carries a generation-specific request so cancellation cannot consume a
  later cleanup request. `catalog_reclamation/operation_registry.rs` owns atomic request admission,
  worker retirement, pending-request handoff, run identity, and attempt control. The worker consumes
  typed transitions instead of separately reading status and appending work that may have lost its
  executor. Its terminal status cannot be overwritten by a delayed preemption notification.
- `catalog_session/maintenance.rs` owns maintenance admission and proof retention alongside
  the session cache's validation and foreground-scan protection transitions. It releases the
  transition mutex before the operation and wakes waiters on success, failure, interruption, and
  unwind. Its dedicated bounded page-reclamation entrypoint preserves validated proof only after
  successful or normally interrupted incremental reclamation: SQLite's
  [incremental vacuum](https://www.sqlite.org/pragma.html#pragma_incremental_vacuum) changes physical
  pages, not logical schema or rows. The adapter recognizes `SQLITE_INTERRUPT` separately from
  unrelated errors even when cancellation arrives concurrently. Structural conversion, errors and
  unwind still retire the proof.
  Independent invalidation takes precedence over retention. Checkpointing remains outside structural
  invalidation. The reclamation facade selects
  these contracts rather than maintaining a second active-scan flag or invalidation protocol.
- `sqlite_catalog/reusable_connection.rs` owns retained-connection proof. Object/session identity,
  write-admission ownership, autocommit, active statements and pending publication are checked before
  reuse. Typed schema observations remain separate from fresh namespace identity. A retained
  connection observes its already opened database; before returning, one fresh path-and-file-ID
  observation must match the validated session, even if SQL proof failed. Namespace replacement
  takes stale-session precedence over errors from the retired database so the application can renew
  the session. A newly opened writable connection retains its before-open and after-proof identity
  checks. No path/time cache or assumption of a permanent writable identity guard substitutes for
  fresh evidence; same-file hardlinks in different WAL namespaces remain distinct.
  `sqlite_catalog/catalog_namespace.rs` owns the held identity and current namespace observation.
  Windows connections retain a no-delete regular-file guard, matching SQLite's existing sharing
  restriction. An exactly matching, normalized local disk path may query its current final path and
  complete file ID through that guard; neither captured value alone admits reuse. Alias, relative,
  UNC, ambiguous and differently spelled paths reopen the configured namespace. Ancestor movement
  must either be prevented by the open objects or change the observed final path and reject reuse.
  The guard retires after its SQLite connection. Non-Windows paths retain fresh path observation.
- `sqlite_catalog/migrations.rs` remains the ordered migration coordinator while historical steps
  are stable. `migrations/schema_shape.rs` owns shared column, index, foreign-key and normalized SQL
  comparisons, including version-dependent handoff columns. Contract and repair owners use it
  directly so they share one acceptance rule without depending on coordinator implementation.
  Quoted SQL tokens retain their exact meaning. `migrations/current_schema.rs` owns current-schema
  proof order and its consistent read snapshot: structural checks precede the complete authority-row
  audit. Historical v19/pre-live-gap proof sequences and test-only audit counters belong to that
  owner as well; version steps and individual contracts call it directly. Process-owned validated
  sessions reuse that proof; schema markers alone are not row authority. Spool-row proof lives in
  `migrations/inventory_spool_rows.rs`: ordered per-run ordinals and
  once-grouped per-directory entry counts replace correlated recounting, without omitting parent,
  lifecycle, or foreign-key evidence. Structural proof and snapshot ownership remain upstream.
  `migrations/preview_expectation_repair.rs` owns the preview-repair marker and the exact legacy
  reset of unbacked ready expectations. It rechecks the marker after acquiring its immediate
  transaction and publishes the marker atomically with derived expectation/link changes; retained
  scans, backed previews and other states remain intact.
  `migrations/source_revision_contract.rs` owns source generation/revision proof, the metadata
  completion marker and precontract invalidation. Both forward migrations and current-catalog
  repair use this owner directly. The repair accepts an absent marker only after current-schema
  proof, rejects a damaged existing marker, and publishes alias-wide invalidation with the marker
  in one immediate transaction. Identity evidence and the coordinator's admission order remain intact.
  `migrations/change_lane_contract.rs` proves the queue-origin lane projection;
  `migrations/recovery_authority_contract.rs` proves immutable P2 authority and canonical opening
  boundaries. Each owns structural/row acceptance and its rejection regressions. Historical DDL and
  version-step ordering stay in the coordinator; the validation bodies no longer live there.
  `migrations/metadata_inventory_contract.rs` owns inventory shape, scope/generation, staged counts
  and entry relations; `migrations/terminal_media_contract.rs` owns terminal evidence shape and
  root ownership. They retain version-dependent source fields and structure-before-row validation.
  `migrations/recovery_execution_contract.rs`, `root_namespace_contract.rs`, `live_gap_contract.rs`
  and `inventory_spool_contract.rs` own the separate execution, namespace, gap-consumer and spool
  proofs. Current-schema and compatibility repair call them directly; the existing spool-row
  validator remains shared. Recovery test catalogs have a test-only owner, so proof tests do not
  depend on fixtures hidden inside the migration coordinator's test module.
  `migrations/catch_up_contract.rs` and `scan_handoff_contract.rs` own checkpoint/lineage and
  batch-handoff proof. `catch_up_repair.rs` owns the exact v19 compatibility repair;
  `scan_owner_contract.rs` owns the v17/v18 authority markers and scan-owner repair. Their
  existing preflight, lock recheck and transaction boundaries remain separate from version dispatch.
  `migrations/journal_contract.rs` owns journal structural admission and row-proof order. Its schema,
  state-row, lineage and payload modules prove separate persisted relationships using canonical
  values. Normal journal writes reuse the lineage/payload proofs directly within their existing
  transactions; historical DDL steps retain the same canonical schema definitions.
  `terminal_inventory_repair.rs` owns exact terminal-run cleanup; `retired_live_gap_repair.rs`
  admits only retired-root/generation residue. `current_recovery_state.rs` keeps their combined
  repair and validation in one transaction. `interrupted_foreground_repair.rs` owns lease/claim
  restoration and cleanup after an interrupted explicit scan. `prerelease_journal_repair.rs`
  admits only the exact historical trigger/window shapes. Each retains its original admission,
  recheck and rollback rules; ordinary migration ordering remains in the coordinator.
- `migrations/persistent_journal_baseline.rs` owns structural and row proof for recovery windows.
  A completed window is historical coverage, not a demand to freeze the current checkpoint at its
  closing cursor. Later progress requires compatible identity and retained range evidence; a later
  recovery requires its own exact authority and window. History cannot authorize a regressed
  checkpoint, an unrelated namespace or journal, or an unfinished window's premature completion.
  This validator remains inside the current-schema owner's consistent read snapshot and preserves
  the narrow historical migration exception rather than adding startup retry or repair policy.

Content-signature admission and decoder failure classification have small adapter owners, separate
from filesystem traversal and metadata extraction. Generated media fixtures, format adapter
contracts, application replacement lifecycles, and performance workloads have distinct test modules.
Negative source observations belong to a version-bound validation roster, not diagnostic strings.
First-import journal completion owns read-only readiness selection and transactional revalidation;
an empty maintenance poll must not acquire priority write authority and interrupt useful work.
The scan publication receipt carries the count established by its committing transaction; an
enumeration counter cannot replace the published result after concurrent reconciliation. Cursor
queries retain strict keyset range selection independently of whether the requested page is first.
