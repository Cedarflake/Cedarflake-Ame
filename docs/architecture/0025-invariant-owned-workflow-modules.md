# ADR 0025: Keep workflows thin through invariant-owned modules

- Status: Accepted
- Date: 2026-09-06

## Context

Phase 34 crossed several real ownership boundaries at once: retained-catalog migration repair,
foreground scan finalization, atomic publication, root removal, physical database reclamation,
preview replacement, and multi-root presentation state. The reported defects required coordinated
changes, but repeatedly extending the existing top-level files also exposed structural debt.

The largest files are not automatically wrong. Some contain stable schema history or extensive
tests. The actionable problem is responsibility growth: a controller, adapter, or orchestrator that
already coordinates several subsystems should not acquire another issue-specific flag, retry loop,
SQL block, or presentation guard. Such patches obscure the owning invariant and make a local fix
silently change unrelated workflows.

## Decision

Top-level workflow files are composition boundaries. They may translate inputs, select a typed use
case, sequence owned collaborators, and publish a typed outcome. They do not own the detailed state
machine or persistence contract of every collaborator.

New behavior is placed in an invariant-owned module when any of these conditions applies:

- the existing owner already coordinates two or more independent state machines;
- the change needs its own cancellation, retry, transaction, or stale-publication rules;
- the same owner has accumulated fixes for unrelated user-visible symptoms in the current phase;
- the behavior can be tested through a narrower contract than the top-level workflow.

Extraction must move real responsibility. A forwarding wrapper with shared mutable flags or SQL
left in the original file does not establish a boundary. Each extracted module exposes typed inputs
and outcomes, owns its invariants, and has focused tests for success, cancellation, stale work, and
the relevant race or rollback boundary. Issue-specific error codes may describe an invariant
failure; they must not become an alternate control plane spanning presentation, application, and
persistence layers.

### Rust boundaries

- `application/scan_library.rs` remains the scan command facade. Scan execution, terminal source
  reconciliation, P0 handoff, and publication waiting live behind dedicated application modules.
- `application/library_synchronization/production.rs` remains the priority and worker-lifecycle
  coordinator. Journal baseline opening and closing use different typed work items in
  `journal_baseline.rs`; an opening authority is either an existing root or a first import with a
  required scan identity and start time, so mutually exclusive fields cannot form an invalid job.
- `adapters/sqlite_catalog.rs` remains the catalog and write-admission facade. Atomic replacement-
  scan publication and its bounded identity reconciliation live in
  `sqlite_catalog/scan_publication.rs`. Its `validation.rs` owner captures the fixed temporary
  validation roster and verifies typed per-item outcomes against the complete staged/live payload
  in that same publication transaction; the facade does not carry opaque exception flags.
- `sqlite_catalog/persistent_journal.rs` remains the journal facade. Root-unregistration lineage and
  cleanup live in `persistent_journal/root_unregister.rs` so removal cannot accidentally discard a
  surviving root's cross-root rename evidence.
- Database page reclamation remains a separate application operation and persistence adapter. It
  is lower priority than foreground and ordinary recovery publication, is preemptible at SQLite
  progress boundaries, and carries a generation-specific request so cancellation cannot consume a
  later cleanup request.
- `sqlite_catalog/migrations.rs` remains the ordered migration coordinator while historical steps
  are stable. `migrations/current_schema.rs` owns current-schema proof order and its consistent read
  snapshot: structural checks precede the complete authority-row audit. Process-owned validated
  sessions reuse that proof; schema markers alone are not row authority. Shared historical SQL
  validators and shrink-only compatibility repair remain the next physical split, not a completed
  extraction. Migration order and one-transaction rollback remain centralized.

### Dart boundaries

- `LibraryController` remains the presentation-facing facade. In the current slice,
  `library_scan_execution.dart` owns only mutual-exclusion admission between one primary scan and
  per-root updates; it does not yet own scan start, subscription, pause, resume, cancellation, or
  shutdown. Those lifecycle responsibilities remain in `LibraryController` as frozen debt: no new
  scan-lifecycle behavior may be added there, and the next such change must first move the complete
  primary-scan lifecycle behind a typed execution boundary. Multi-root update selection, bounded
  scheduling, retry, and cancellation live in `library_update_controller.dart`.
- Viewport/query ownership and committed root-removal refresh live in dedicated application
  coordinators. The viewport has one query-generation and revision owner for retained pages, time
  navigation, and directly requested visible ranges. Root removal distinguishes the one database
  unregister command from display-only refresh retries. Gallery widgets consume immutable state
  and may not compensate for an unresolved catalog or task-lifecycle invariant.
- Visible root removal admits an opaque, single-use prepared operation before awaiting the feedback
  frame. The application coordinator owns execution, abandonment, and disposal invalidation; the
  widget supplies only the rendered-frame boundary. A committed unregister clears selection even
  when the subsequent display refresh fails.
- Root removal reserves the viewport's catalog-publication boundary through its terminal state.
  Independent root scans continue; their display refresh waits for release and reads the current
  query, rather than superseding the removal generation or restoring a pre-removal transition base.
  Reservation release is explicit on completion, failure, abandonment, and disposal.
- `CatalogReclamationController` owns reclamation presentation state, polling, command epochs,
  ordered full-storage snapshots, and terminal usage refresh. A snapshot started before or during a
  newer command cannot replace that command's result. Settings persistence and preview cleanup
  remain separate workflows; neither keeps another copy of the reclamation state machine.
  Accepted full snapshots publish storage usage and reclamation phase together. Configuration-save
  receipts have independent typed merge authority over configuration only; stale accompanying usage
  cannot replace a newer measurement, and a read begun before a confirmed save preserves that save.
  Retired-preview ownership is lifecycle evidence, not a configuration field: only ordered full
  status reads replace that list. Save completion re-reads full status to reconcile backend ownership
  changes without deriving cleanup policy from configured paths. A failed post-save refresh retains
  confirmed configuration and retries the read independently; it never repeats the successful save.
  `StorageSnapshotCoordinator` separately owns full-read scheduling and completion: same-version
  requests share one in-flight read; epoch or configuration changes coalesce one latest follow-up.
  Commands suspend, rather than discard, required refreshes and wake them after settlement. Disposal
  settles waiting callers without waiting for a stale backend read. Snapshot acceptance and
  configuration policy remain in the presentation controller, not the scheduling module.
- `LibrarySourceImage` retains the installed Flutter SDK's `FileImage` decoder but extends its
  path-only cache identity with the source lease and explicit retry generation. A same-path edit
  with unchanged size and restored modification time must resolve a different stream when its
  source generation changes. Preview-only updates do not invalidate the viewer's source stream,
  and superseded source streams cannot publish into the current image widget.
- Menus, loading feedback, task live regions, and startup orchestration use repository-owned shared
  components so one defect fix does not create a second interaction contract.

### Review and roadmap rule

File length is evidence for investigation, not an automatic rewrite trigger. Review records the
responsibilities changed and the narrow owner of every new invariant. If a complete extraction
would materially broaden the active fix, the current change establishes the typed seam, records the
physical split in `docs/roadmap.md`, and adds no further behavior to that debt area until the split
is completed.

## Rejected alternatives

### Continue appending focused patches to the current owner

Rejected. A patch can be locally correct while making cancellation, transaction, and stale-state
rules impossible to reason about together.

### Enforce a universal line-count limit

Rejected. Generated bridge files, schema history, and test fixtures have different reasons for
size. A line cap rewards cosmetic splitting and does not prove ownership.

### Rewrite all large files during the runtime repair

Rejected. That would mix behavior changes with broad churn, weaken reviewability, and risk the
accepted gallery and continuity contracts.

## Consequences

- Runtime fixes require a small amount of explicit composition code and more focused modules.
- Persistence and application races can be tested at their owning boundary instead of only through
  end-to-end fixtures.
- Some historical large files remain. Their staged splits are roadmap work, not hidden claims of
  completion.
- Future changes that extend a recorded debt owner without first establishing its boundary violate
  this decision and the repository contract.

## Verification

- focused tests cover every extracted invariant and its reported race;
- the complete serial Daily gate and Windows integrations pass after extraction;
- an independent final audit checks behavior, layering, duplicate state machines, and regressions;
- `git diff --check` and hosted PR checks pass before Phase 34 closes.
