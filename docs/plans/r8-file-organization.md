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

## Planned interaction decisions

Ordinary gallery and viewer deletion acts on the explicitly selected physical file, including a
preferred file. A preferred/selected marker is not a global deletion lock. A merged duplicate card
does not authorize deleting its hidden copies. After verified deletion, automatically choose and
persist a new preferred file from the remaining exact duplicates and refresh the card; remove the
card when no copies remain. A cancelled or failed deletion that leaves the file intact preserves
the original preference. Keep exact-group single preference and similar-group multiple selections
as group-cleanup defaults; unselected membership alone does not authorize deletion.

| Situation | Planned behavior |
| --- | --- |
| Name already exists at the destination | Keep the extension and allocate `name(1).jpg`, `name(2).jpg`, and so on, using the first free number without overwriting existing files. Record the actual final name. |
| Delete confirmation | Show the affected file count and a `永久删除` checkbox, unchecked for every new request. Confirming normally requests the system Recycle Bin; checking the option and confirming explicitly authorizes permanent deletion. Cancel or close performs no deletion. |
| File changed after selection or cannot be verified | Do not delete it. Show the affected file and reason, retain a recorded execution rejection, and require a new selection and confirmation after viewing the current image. Preparation failure is reported before execution; never claim every selected file was deleted. |
| Windows cannot recycle, including an oversized file | Preserve the native Windows confirmation and follow the user's response. Permanent fallback requires an affirmative system confirmation; cancellation or a refusal leaves the file intact. Ame never answers the dialog or silently retries permanently. |
| Multiple album destinations in album-based organization | Skip that image, continue the other items, and summarize skipped items at the end. Do not choose an album or create multiple copies implicitly. This does not change virtual album membership. |

When an export/organization run has automatic renames, skipped items, failures or unfinished items,
write one UTF-8 TXT explanation in the selected export root. Include the operation time, source
filename/path, actual destination/name, result and reason for each exceptional item. Generate it
from recorded outcomes, use a unique report filename, and preserve earlier reports. A clean run
needs no report. Keep report-write failures separate from completed file operations and surface
them in Ame; retain the operation history. In-place deletion without an export root does not create
a sidecar in the source library. This is a result report, not a plan export or preview.

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
- Validate default recycling, explicitly selected permanent deletion, and native fallback confirmation
  through supported
  [operation flags](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags)
  and pre-operation admission. Prove that declining the system's permanent-delete prompt preserves
  a non-recyclable file. Use
  [`PostDeleteItem`](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperationprogresssink-postdeleteitem)
  to record the actual outcome and recycled item, not to establish prior authorization. Permanent
  deletion has no Recycle Bin restore action.
- Admit numbered collision naming without overwrite, including another writer occupying a chosen
  name before execution. Bind allocation to the authorized destination and naming policy; record
  the resolved target before native submission. Do not use a blanket Yes-to-All flag to hide
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
fixtures in Windows Sandbox or hosted Windows. Cover same-volume actions, cross-volume behavior,
numbered collisions, both deletion modes, fallback-confirmation refusal/acceptance, restoration,
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
Reconcile successful preferred-file deletion and replacement selection in one catalog publication;
late callbacks cannot retarget an authorized deletion to the newly chosen representative.

On restart, reconcile in-flight records with disk and catalog state before offering retry or
supported recovery. Unknown outcomes remain unresolved until checked; never replay a mutation
solely because its completion callback was lost. Exit with recovery proven at native-result and
catalog-commit boundaries, before UI mutation becomes available.

### R8-03: Connected desktop file actions

Connect copy, move, rename and confirmed deletion actions to existing gallery selection and context
menus through the application owner. Distinguish selected image groups from affected physical
files; unresolved duplicate-location choices require an explicit selection. Use existing destination
selection, menus, feedback and bounded record surfaces rather than a preview page or task center.
In the image viewer, place the delete button beside Favorite and Add to Album in the
upper-right image-action group, before the window controls.

Show real progress, partial results, cancellation and supported retry/restore actions. Keep gallery,
viewer, albums, duplicates and search coherent after each verified change. Exit with the complete
sample-copy interaction, including unavailable targets and a cancelled/partially completed batch.

### R8-04: Rule-based organization

Connect category, album, date and source rules from the product scope to the same request/execution
path. Validate names and destinations in the background and resolve blocking conflicts in context.
Rule changes retire unstarted requests; multiple album destinations are skipped and reported.
Keep rules and item generation bounded, without a second engine, preview or saved-plan workflow.
Connect the TXT result report to recorded exceptional outcomes. Exit with deterministic rule results
and the same authorization/recovery behavior as direct actions.

### R8-05: Integrated verification and delivery

Follow the [filesystem verification procedure](../acceptance/r8-file-operations.md): native adapter
fixtures, application recovery tests and the real desktop flow each prove their own boundary.
Use existing change-scoped CI selection; extended volume/resource checks belong to the affected
integration or release owner. Record results and limits in that acceptance owner, keeping stage
disposition in the roadmap. Deliver independently reversible slices with their necessary tests and
documents; no speculative test inventory or new full suite for unrelated UI edits.
