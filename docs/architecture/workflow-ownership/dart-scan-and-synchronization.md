# Dart scan, source and synchronization lifetimes

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

- `LibraryController` is the presentation-facing composition facade. The primary-scan lifecycle
  owner contains picker admission, serialized start, run identity, subscription, terminal
  reconciliation, pause, resume, cancellation, and shutdown. Its immutable task snapshot is separate
  from gallery query and loading state: a retained task cannot reserve the viewport, and a gallery
  refresh cannot overwrite its checkpoint identity or progress. Active-run delivery and task
  projection may have separate cohesive owners; moving the entire old controller into another
  multi-responsibility file is not an extraction. `library_scan_execution.dart` remains only the
  mutual-exclusion boundary between a running primary scan and per-root updates. Multi-root update
  selection, bounded scheduling, retry, and cancellation live in `library_update_controller.dart`.
- `library_update_task.dart` owns immutable per-root update progress projection. Checkpoints,
  issues and terminal events change the task; individual asset deliveries do not invalidate the
  update list. Terminal counts remain authoritative, including cancellation between checkpoints.
  Execution identity, control replay and native stream retirement stay with the update controller.
- `library_scan_control.dart` owns per-run ordered control intent and native registration replay.
  `library_scan_run.dart` owns stream identity and drain, including protocol failures; a visible
  error is not permission to release a still-running native task.
- `library_scan_restoration.dart` owns the ordered, read-only checkpoint lookup and its stale-result
  guards. Startup restores an unfinished first import as paused; only the existing explicit user
  continuation executes it. Restoration does not own scan commands or subscriptions. Cancelling a
  retained task uses the asynchronous application command and clears its task snapshot only after
  commit; the active-run cancellation token cannot substitute for durable checkpoint cancellation.
- Source navigation consumes import/update and directory synchronization state directly; duplicate
  analysis and its publication cannot extend a completed source operation. The analysis activity
  widget owns scheduler inputs and listener disposal. Analysis publication retains its independent
  completion and query-refresh authority, including a retired run awaiting its listener.
- `library_preview_root_failures.dart` owns admission and lifetime of scan-scoped root failures.
  Admission rejects a retired source before it can release a root failure or supersede current
  queue ownership. Deferred photo repairs revalidate their operation, source and artifact before admission.
  Only a source still accepted by the current publication owner may block sibling demand. A late
  error from a retired source settles as superseded without acquiring root-wide authority. Explicit
  retry, availability cooldown, scan replacement and authority restoration retain their distinct
  release rules; pending-request completion and execution slots remain with the queue.
- `library_synchronization_refresh.dart` owns the mounted presentation's highest pending catalog
  revision, one active refresh, busy/superseded delay and failure-feedback transition. It delegates
  reads to the existing application admission/publication path. Failure waits for explicit retry
  unless another revision arrived during the attempt; an applied result still has to expose the
  requested catalog revision. Disposal cancels scheduled retry and prevents late completion from
  reading presentation state or publishing feedback. It reports failure transitions to the screen's
  notification owner without owning notification history or actions.
- `LibrarySynchronizationController` owns the desktop start/poll/stop generations, retry waits and
  terminal publication. Its gateway owns native tickets, bridge calls and Ame-only error/results;
  the adapter's snapshot mapping stamps observations without retaining controller state.
  `LibrarySynchronizationRetry` owns transient-code admission and its elapsed-time/attempt budget;
  the controller retains bounded start delays and retires their waits on stop or disposal.
  `library_synchronization_phase_projection.dart` preserves phase age within a root generation,
  while the existing failure-retention owner preserves unresolved diagnostics and current recovery
  authority. `library_synchronization_diagnostics.dart` owns report throttling only. Bootstrap retains
  one `LibrarySynchronizationLifecycleOwner`, immediate stop fencing and joined native retirement.
- `library_synchronization_notifications.dart` owns root-generation notification keys, condition
  resolution, live-only capability lifetime and typed action selection against current roots.
  Shared notification history and stale-action admission remain with `AmeNotificationController`;
  the screen retains mounted subscription guards, refresh execution and the existing update dialog.
- `library_browse_admission.dart` owns read admission independently from scan-command admission.
  A published catalog remains queryable during a primary scan; unpublished staging is never read
  as completed content. Query replacement, paging and time reads retain their distinct exclusions.
  This policy cannot grant a publication lease or bypass revision and result-generation checks.
- Visible root removal admits an opaque, single-use prepared operation before awaiting the feedback
  frame. The application coordinator owns execution, abandonment, and disposal invalidation; the
  widget supplies only the rendered-frame boundary. A committed unregister clears selection even
  when the subsequent display refresh fails.
- Root removal reserves the viewport's catalog-publication boundary through its terminal state.
  Independent root scans continue; their display refresh waits for release and reads the current
  query, rather than superseding the removal generation or restoring a pre-removal transition base.
  Reservation release is explicit on completion, failure, abandonment, and disposal.
  After the single unregister command, the mounted query projection captures the current visible
  identity before publishing the removed-root projection. `library_root_removal_page_reader.dart`
  owns the bounded anchored page and removed-root absence check; the complete timeline remains a
  separate background read. Later input retires old results and errors. Only a proven position
  supersession may repeat display publication; neither that continuation nor an explicit display
  retry repeats the unregister command. Capture failure still projects the committed removal.
