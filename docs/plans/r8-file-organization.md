# R8 file organization and recovery

Status: scoped execution plan. The [roadmap](../roadmap.md#r8-implementation-sequence) owns activation,
order and delivery status; the [product scope](../product/organization-workflows.md#r8---file-organization-and-recovery)
owns supported behavior. This plan schedules no source-file changes by itself.

## Delivery approach

Use Windows to perform file operations and manage the system Recycle Bin. Ame owns the selected
scope, user authorization, operation history, progress projection and catalog reconciliation.
Reuse Windows conflict and error handling where it preserves that scope. Keep the accepted absence
of an organization-preview feature and the existing desktop controls and feedback conventions.

Start product implementation on `codex/r8-file-organization`: the work crosses native adapters,
persistence, bridge contracts and presentation. Keep this planning change separate from execution.
Do not introduce a new privileged Windows service, private recycle bin or general-purpose copy
engine. A system capability gap first receives a bounded Windows API evaluation.

## Implementation boundaries

| Owner | Work |
| --- | --- |
| Rust application | Resolve exact physical targets, prepare immutable requests, persist authorization and intent, coordinate execution and recovery |
| Windows adapter | Invoke Shell operations, own COM/thread lifetime, translate callbacks, cancellation and actual item results into Ame values |
| Catalog adapter and reconciliation | Persist operation records with forward migrations; reconcile verified file changes with asset locations and durable decisions |
| Flutter application and presentation | Submit intents through the typed bridge, display bounded progress/history and connect existing gallery actions |

Follow [ADR 0037](../architecture/0037-feature-module-dependency-boundaries.md) and the
[repository layout](../development/repository-layout.md). Keep operation history distinct from the
existing filesystem-change journal. Reuse source-identity and R2c reconciliation contracts; do not
add file mutation to scan, preview or analysis owners. Native Shell types stay inside the adapter.

## Windows capability admission

The primary integration candidate is
[`IFileOperation`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifileoperation).
The repository already includes the `windows` crate with Shell and COM features. Use a dedicated
STA execution owner in the current user session, outside the Flutter UI thread and the existing
MTA search-index worker. R8-01 records the concrete API, FFI safety and lifetime decision in an ADR.

Validate these adapter behaviors before exposing mutation controls:

- Copy, move and rename return actual per-item outcomes and destination identities, including
  names changed by an explicitly chosen collision policy. Queuing a request is not completion.
- Consume [`IFileOperationProgressSink`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifileoperationprogresssink)
  callbacks and check [`GetAnyOperationsAborted`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-getanyoperationsaborted)
  after execution, even when the overall result is successful. Preserve failed, skipped, cancelled
  and uncertain results separately. System progress estimates must not be presented as measured bytes.
- Verify how cancellation reaches an active operation and measure when it stops. Keep a stopping
  state until callbacks and execution retire. Do not infer pause/resume support from timer callbacks.
- Validate recycle-only behavior through supported
  [operation flags](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags)
  and pre-operation admission. A non-recyclable item must be rejected before deletion; inspecting
  [`PostDeleteItem`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperationprogresssink-postdeleteitem)
  afterwards cannot prevent permanent deletion. Record the actual recycled item for recovery.
- Preserve explicit overwrite and conflict choices. Do not use a blanket Yes-to-All flag to hide
  dialogs. Keep Ame's progress surface and avoid a duplicate native progress window; required
  system dialogs must have the correct owner window and cannot silently broaden the request.

For cross-volume moves, first establish the system's completion, failure and interruption behavior
against the product's content-preservation requirements. If a single native move cannot satisfy
them, compose native copy, bounded verification and separately authorized native source removal.
Do not prescribe a custom temporary-file protocol or claim a file operation and SQLite commit form
one transaction. Any changed safety guarantee needs an explicit scope decision before implementation.

Use supported Windows restoration facilities for recycled items. Resolve a reliable per-item
restore path in R8-01; if only the system Recycle Bin UI is available, expose that route and its
limitation explicitly. Never automate global Explorer Undo or edit Recycle Bin internals. Windows
undo state is session-dependent, so durable Ame history and restart reconciliation remain separate.

## Implementation slices

### R8-01: Windows adapter admission

Record the platform ADR and FFI safety contract, then prove the selected native APIs on disposable
fixtures. Cover same-volume actions, cross-volume behavior, recycle-only rejection, restoration,
callbacks, cancellation and joined shutdown. Establish bounded batches and one mutation lane; let
Windows own transfer mechanics. Document unsupported cases instead of silently substituting a
custom engine. Exit with the native contract and concrete evidence needed by R8-02.

### R8-02: Durable operations and catalog reconciliation

Implement typed requests, immutable authorization, item outcomes and a bounded operation-history
query. Persist intent before mutation; no new item starts when durable recording is unavailable.
Use forward migrations and preserve existing catalogs and user decisions. Never hold a catalog
transaction across native work.

Apply actual verified results through the existing identity/reconciliation boundary. Correlate
system notifications with the operation so the same change is not published twice. Cross-volume
identity changes retain proven logical continuity; copied files follow the existing asset/location
rules without merging durable identities. Target selection does not silently register a new root.

On restart, reconcile in-flight records with disk and catalog state before offering retry or
supported recovery. Unknown outcomes remain unresolved until checked; never replay a mutation
solely because its completion callback was lost. Exit with recovery proven at native-result and
catalog-commit boundaries, before UI mutation becomes available.

### R8-03: Connected desktop file actions

Connect copy, move, rename and move-to-Recycle-Bin actions to existing gallery selection and context
menus through the application owner. Distinguish selected image groups from affected physical
files; unresolved duplicate-location choices require an explicit selection. Use existing destination
selection, menus, feedback and bounded record surfaces rather than a preview page or task center.

Show real progress, partial results, cancellation and supported retry/restore actions. Keep gallery,
viewer, albums, duplicates and search coherent after each verified change. Exit with the complete
sample-copy interaction, including unavailable targets and a cancelled/partially completed batch.

### R8-04: Rule-based organization

Connect category, album, date and source rules from the product scope to the same request/execution
path. Validate names and destinations in the background and resolve blocking conflicts in context.
Rule changes retire unstarted requests; unresolved album assignments do not choose a destination.
Keep rules and item generation bounded, without a second engine, preview or saved-plan workflow.
Exit with deterministic rule results and the same authorization/recovery behavior as direct actions.

### R8-05: Integrated verification and delivery

Follow the [filesystem verification procedure](../acceptance/r8-file-operations.md): native adapter
fixtures, application recovery tests and the real desktop flow each prove their own boundary.
Use existing change-scoped CI selection; extended volume/resource checks belong to the affected
integration or release owner. Record results and limits in that acceptance owner, keeping stage
disposition in the roadmap. Deliver independently reversible slices with their necessary tests and
documents; no speculative test inventory or new full suite for unrelated UI edits.
