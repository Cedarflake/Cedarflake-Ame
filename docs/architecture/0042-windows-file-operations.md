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
