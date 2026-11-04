# Media previews and storage capabilities

Status: binding scoped requirements under [0025-invariant-owned-workflow-modules](../0025-invariant-owned-workflow-modules.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

- `media_inspector/jpeg_headers.rs` owns streamed JPEG header parsing, source-error retention and
  the read budget. The media-inspection facade selects the detected format; this adapter returns
  Ame dimensions/metadata and shares the existing EXIF/orientation policy. It neither opens a new
  source nor changes source revalidation, preview decoding or inspection-engine identities.
- `application/preview_health.rs` owns final missing-file and accounting observations for both
  startup recovery and ordinary catalog reads. Its typed persistence port pairs publication
  exclusion with a zero-wait conditional transaction; deferred maintenance cannot block foreground
  publication or downgrade a newer same-key artifact. The recovery worker owns traversal, not a
  second invalidation protocol.
- `preview/store_admission.rs` selects the active store only after obtaining generation or
  reclamation access and owns both for the admitted lifetime. The preview facade carries storage
  configuration, not a previously selected store, across a capacity-reclamation gap.
- `local_files/preview_source.rs` owns initial root-bound source opening and classifies a missing
  child separately from root unavailability and other I/O failures. `PreviewSourceFactory` exposes
  separate read and publication leases; `SourceMediaInspector` inspects the admitted reader without
  reopening a path. Native identity/revision checks and lease retirement stay in their original owners.
  The initial observation retires stale
  preview work, never grants catalog-removal authority. `preview/source_reconciliation.rs` uses the
  existing exact-source admission and guarded path-set reconciliation; an existing path owner keeps
  its lease and work. The admission port distinguishes a superseded request, existing path work,
  a granted lease and an unavailable lease. Proven supersession or existing work retires the old
  preview request; failure to acquire authority is not proof of retirement. Recreated sources and
  root identity are revalidated before any catalog delta. After an admitted attempt, acknowledged
  durable retry or deferral retains completion ownership and retires the stale preview request.
  A failed persistence operation propagates its error and cannot claim that transfer.
  Missing-source path planning consumes `SourceIdentityLocator` from the existing preview-source
  binding. Native identity lookup retains root/volume/file validation and bounded metadata-only
  resolution; application retains exact/relocated selection and the same guarded publication path.
- `ports/preview_materialization.rs` exposes preview generation through Ame values and a read/seek
  source. `PreviewSource::reader_from_start` supplies byte-zero access to the already admitted source;
  native file duplication and cursor reset stay in `local_files/preview_source.rs`. Readers may share
  a cursor and are consumed sequentially. The port grants no path reopen, source mutation or new
  publication authority. The native store retains its existing decode limits, JPEG fallback, cache
  admission and staged publication; application source revalidation and held guards remain unchanged.
- `ports/preview_cache_storage.rs` exposes cache identity, recognition, preparation and held-directory
  capabilities. Cleanup/recovery consume these contracts and the catalog port; format details and
  native namespace proofs remain adapter-owned. Store borrowing exposes materialization and atomic
  accounting ports without changing access-first selection or capacity-handoff lifetime.
- `application/storage/preview_activation.rs` owns the cross-database restart obligation for
  switch-and-regenerate. Target initialization and idempotent catalog reset precede pending-intent
  retirement. The storage facade selects this use case; it does not carry compensating SQL or a
  second crash-recovery state machine. Its dedicated tests exercise both commit boundaries and
  source-safe fallback independently of ordinary settings tests.
- `storage/catalog_admission.rs` owns process-local arbitration between configuration persistence
  and scan registration. `storage/configuration_update.rs` owns complete settings validation and
  save. The scan facade calls the typed admission boundary only around registration; it does not
  duplicate storage policy or hold a configuration permit during enumeration.
- `storage/source_cleanup_admission.rs` owns the shared resolved-scope reservation between source
  registration and destructive preview cleanup. Its typed permits exclude conflicting operations
  without retaining a registry lock during registration commits, callbacks, or deletion.
- `local_files/preview_cache_namespace.rs` owns the operation-scoped capability over the existing
  Windows namespace proof. A missing-root cleanup cannot acquire later deletion authority; a bound
  operation retains root and ancestor guards through generation, capacity handoffs, and cleanup.
  Idle accounting retains identity evidence only. `preview_cache/preparation.rs` distinguishes
  unavailable write capability from failed inventory so cache limitations cannot abort catalog
  startup or retire an incomplete migration. `preview_cache/staging_encoding.rs` owns exclusive
  temporary-file creation, held-handle encoding, flush, and cleanup of its own failed output.

The preview-publication adapter owns typed request/source/root authority, artifact registration,
the conditional location update, and effective-time invalidation in one transaction. Its catalog
facade delegates that complete use case. Artifact publication and metadata publication remain named
operations under the same commit/rollback boundary; no source read or decode enters the transaction.

The preview-cleanup adapter owns complete full-reset and outside-root-reset operations. It retains
scope validation before pending-location flush, then clears location and both handoff-owner preview
state together with artifact-index deletion in one write transaction. The facade delegates these
operations without splitting their commit or rollback boundary. Filesystem cleanup, active-store
invalidation and preview-root activation remain application responsibilities.
