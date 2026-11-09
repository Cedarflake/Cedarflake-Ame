# ADR 0042: Execute authorized file operations through Windows APIs

- Status: Accepted; durable application authorization is required before enabling actions
- Date: 2026-11-09

## Context

Physical organization needs native transfers, cancellation and Recycle Bin integration while Ame
retains authorization, durable history and catalog consistency. The
[R8 plan](../plans/r8-file-organization.md) defines supported actions and delivery order; this
decision does not enable mutation or replace the per-action confirmation.

## Decision

Use the already admitted `windows` and `windows-sys` bindings. Windows performs transfers and
deletion; Ame application policy resolves the physical item, destination and deletion mode,
records intent before submission, and reconciles actual per-item results. Native interfaces and
HRESULT values remain inside the adapter.

Use `CopyFile2` with `COPY_FILE_FAIL_IF_EXISTS` for copies and `SetFileInformationByHandle` with
`FileRenameInfo` and `ReplaceIfExists = false` for same-volume moves and renames. Hold the exact
revalidated source handle and guarded source/destination directory namespaces. The source handle
denies concurrent content writes; a rename operates on that handle rather than reopening a path.
An occupied destination returns to Ame's numbered-name allocation without overwriting it. Shell's
automatic collision naming is localized and does not implement the required `(1)`, `(2)` policy.

Use `IFileOperation` for recycling and explicitly authorized permanent deletion on a dedicated,
joined STA thread in the current user's session. Admit only one native mutation at a time.

Bind each submitted item to the authorized source identity/state and target policy. Reject stale,
unavailable or escaping paths before submission and revalidate at the pre-operation callback.
Hold admitted directory namespaces through execution. Shell paths alone are not identity evidence.
No callback may retarget an operation to a replacement duplicate representative.

Use `IFileOperationProgressSink` for deletion admission, per-item results and pre-item cancellation.
An overall successful return does not complete an item without its result; also inspect
`GetAnyOperationsAborted`. Keep uncertain results for disk/catalog reconciliation rather than
replaying them. Shell progress is an estimate, not measured bytes. CopyFile2 reports byte progress;
its callback reads atomic cancellation and returns `COPYFILE2_PROGRESS_CANCEL`, allowing Windows
to remove its partial copy. Cancellation remains stopping until the native call and thread retire;
completed mutations are not undone by a late cancellation request.

Default deletion requests recycling and preserves native confirmation of permanent fallback.
Explicit permanent deletion carries separate action authorization. Do not set blanket
`FOF_NOCONFIRMATION` or answer native dialogs automatically in production. Attach necessary system
dialogs to the Ame window and suppress only the redundant transfer progress window.

Allocate numbered destination names without overwriting. Native conflict behavior, racing creation,
non-recyclable deletion prompts and per-item restoration require admission evidence. A supported
system Recycle Bin route may provide restoration; do not use global Explorer Undo or private
Recycle Bin metadata. Permanent deletion exposes no restore action.

For cross-volume moves, use native copy, durable receipt recording, bounded byte verification with
both identities held, and separately authorized source removal through `FileDispositionInfo` on
the exact source handle. Flush completed copy streams before returning the copy receipt. The
destination stays protected from concurrent writes/deletion throughout verification and source
removal. Cancellation or verification failure retains the original. SQLite and filesystem mutation
are separate durable transitions, never one atomic transaction.

## Durable intent and outcome journal

Schema v79 keeps physical-operation authorization and append-only transition snapshots in the
existing catalog, separate from virtual-organization history and filesystem-notification work.
Authorization binds a batch ordinal to the exact asset, location, source/root generations, native
source proof, action and destination naming policy. Reusing that ordinal with different intent
fails. Native admission rechecks the catalog binding in the same transaction that records its start;
filesystem evidence is checked by the native adapter immediately before execution.

Each transition compares its expected version and atomically appends the next event and updates
the journal head. No catalog transaction spans native I/O. A cross-volume move records the completed
copy and its destination identity before a separate source-removal transition. An unknown result
remains unresolved; neither opening the journal nor enumerating unfinished work replays it.
Numbered destination names remain within the authorized directory, identity and base-name policy.

The application executor selects the first free numbered name, records it before native submission,
and retries an occupied name only after the native adapter reports no overwrite. Allocation is bounded
to 10,000 candidates per attempt. Copy receipts must commit before source-removal admission; a recording
failure stops the workflow. An interrupted operation cannot reenter ordinary execution. Recovery
inspects pinned parent directories and exact file evidence without mutation; a proven completed move
can regain its receipt. A retained copy requires an explicit continuation before source removal.
Missing or conflicting evidence remains uncertain. The runtime serializes recovery with active work.

A joined application worker owns one bounded page of at most 128 durable item IDs and the validated
catalog connection. Its process-wide task permit excludes other execution/recovery workers until
retirement. Cancellation stops future items and records their cancellation; a completed native
mutation still receives catalog publication. Close cancels and joins the current call. A worker
failure retains its journal for inspection and cannot release authority to automatic replay.

History and unfinished-item reads use bounded keyset windows. Schema validation checks immutable
authority, ordered transitions and agreement with each journal head while retaining one item in
memory. Migration preserves existing albums, decisions and history. These records are durable data;
preview/cache cleanup and source-root removal do not delete them. Rollback cannot undo completed
native mutations or downgrade a catalog while discarding its operation records.

## Catalog publication and notification ordering

Publish a verified native receipt, its location changes, durable organization association and
replacement duplicate preference in one catalog transaction. A failed transaction retains the native
receipt for publication retry; it never repeats the file operation. Remove only the authorized source
identity/version. A newer file at the old path is outside that receipt's scope.

For a destination under a registered root, hold its native identity and source state against writes
and deletion through metadata inspection and catalog commit. Revalidate both the immediate parent
and the registered root namespace. Use existing metadata inspection and source-generation owners,
including the catalog's normalized relative paths. Destination selection does not register a root;
an outside destination retains its verified result and applicable durable organization binding.

Moves retain the original logical asset and organization decisions across the verified identity
change. Copies receive independent asset identity; an already published observation of the same
physical output is reused. Conflicting durable destination associations fail publication without
overwriting either decision. Analysis results remain versioned evidence rather than being rewritten
as operation history.

System notifications may publish before or after native completion is recorded. Reuse an observed
destination's compatible source generation. When a validated notification removes an authorized
deletion target first, save its replacement exact-duplicate preference in that same publication.
Schema v80 adds a partial index over pending deletion location/generation for this point correlation;
normal synchronization does not scan the operation history. Notification evidence alone does not
manufacture a native completion result or authorize another mutation.

Full-scan publication captures pending authorized deletions through the same partial index before
replacing its projection, then saves replacement preferences in that transaction. Its temporary
SQLite table retains only operation IDs and duplicate keys; records are streamed rather than held
as a whole-root collection. Rollback retires both the projection and preference changes together.

## FFI and resource safety

The adapter and its isolated admission probe may use `unsafe` for the named Windows operations:
CopyFile2 and its progress callback, FlushFileBuffers on completed copy streams,
SetFileInformationByHandle with FileRenameInfo or FileDispositionInfo, Shell/COM
apartment initialization/retirement, interface creation, operation configuration/submission,
callback registration, execution/result inspection, dialog-owner validation and Shell display-name
conversion. Use typed bindings; no handwritten vtables, global callback registry or cross-thread
raw interface pointers. Source opening and identity checks reuse ADR 0007/0024 guards.

Copy callback context and UTF-16 input arrays outlive the synchronous call; access only the union
member selected by the native message tag. Copy only the destination identity from the borrowed
stream-start handle, closing its temporary duplicate before callback return; compare it with the
completed destination so a replacement file cannot become the receipt. Callbacks retain no native
pointers, use checked mutex access and atomic progress/cancellation, and cannot unwind. Rename
buffers use pointer-aligned storage, checked byte lengths and space for the complete UTF-16 tail;
the replace flag remains zero. Keep the source handle live through the call. Native errors and
post-operation inspection decide whether a result is known or requires reconciliation.

Initialize COM on the owning thread, drop all interfaces and unadvise sinks before matching
`CoUninitialize`, and join the owner before shutdown. Null-terminated UTF-16 inputs stay live for
each synchronous call. Callback interface/string parameters are borrowed only during that callback;
copy required values into Ame-owned data. Release Shell-allocated strings with `CoTaskMemFree`.
Callbacks cannot unwind across FFI, retain dangling references or hold a catalog transaction during
native execution. HWNDs must identify the current live owner window and convey no mutation authority.
The isolated probe may display its bounded, generated receipt with `MessageBoxW`; its UTF-16
buffers outlive the dialog. This diagnostic does not provide product confirmation or execution.

## Alternatives and consequences

`std::fs` transfers and private trash would duplicate Windows behavior and recovery. Shell command
strings would lose typed callbacks and exact per-item results. A privileged service is unnecessary
for user-owned files and would broaden authority. The Windows adapter retains replaceability through
an Ame port. The COM callback macro requires a direct reference to the already resolved
`windows-core` version; its version and license do not change.

## Validation and replacement

Follow the [R8 verification procedure](../acceptance/r8-file-operations.md), starting with new
disposable files inside Windows Sandbox. Verify actual bytes and paths, cancellation, collisions,
native prompt responses and restoration before exposing the corresponding action. The acceptance
record owns results and capability limits; this ADR records no successful native execution.

Replace the native adapter behind the same authorization/result contract if a required capability
cannot be admitted. Keep unfinished actions unavailable and preserve recorded outcomes; rollback
never replays or reverses filesystem changes automatically.

## References

- [IFileOperation](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifileoperation)
- [Operation flags](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-ifileoperation-setoperationflags)
- [Progress sink](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifileoperationprogresssink)
- [CopyFile2](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-copyfile2)
- [Copy cancellation](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nc-winbase-pcopyfile2_progress_routine)
- [Handle-bound rename](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-file_rename_info)
