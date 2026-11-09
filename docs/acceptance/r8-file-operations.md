# R8 filesystem verification

Status: verification procedure and scoped R8-01 admission evidence; integrated R8 acceptance is pending.

The [roadmap](../roadmap.md) owns activation and delivery status. The
[merged product scope](../product/organization-workflows.md#r8---file-organization-and-recovery)
owns authorization and file-operation behavior. Windows Sandbox is the planned local isolation
route; hosted Windows checks provide repeatable integration coverage.

## Disposable data and execution boundaries

Stage client binaries and sample copies from the established `build/test` corpus in a leased
`Invoke-AmeAcceptanceScratch` directory, then pass them into Windows Sandbox through a read-only
mapped input folder. Copy them into guest-local writable storage before testing. Keep source
fixtures, targets, catalog, and journal in separate guest-owned subdirectories. Follow the existing
[storage and cleanup procedure](quality/execution-safety.md#execution-scope-and-storage); preserve
the reusable corpus and original catalogs. Never give the test process real-library roots or a
production profile. No real photographs or catalogs enter hosted CI.

Use a scoped [`.wsb` configuration](https://learn.microsoft.com/en-us/windows/security/application-security/application-isolation/windows-sandbox/windows-sandbox-configure-using-wsb-file)
with networking and clipboard redirection disabled. Do not expose host source folders or writable
host mappings to mutation fixtures. Capture compact evidence before closing the sandbox; recovery
checks restart Ame within the same sandbox so its test files and history survive the app restart.
Confirm each required native capability in R8-01; the scoped evidence below does not enable UI actions.

Scratch storage supplies ownership and cleanup, not operating-system isolation. The production
operation boundary must validate resolved roots, file identity and target state, and reject path
escape or unapproved reparse traversal. First exercise boundary attacks and process termination in
Windows Sandbox or a disposable hosted Windows job, using synthetic files and fixture-only sentinel
targets. Local client verification follows those checks. Never point a deliberately hostile fixture
at user data.

Record relative paths, file sizes and content hashes before and after a run. Include untouched
sibling files outside the operation's allowed source and target roots but inside owned test storage.
Assert the exact permitted changes and retain compact journal/catalog evidence; success messages
alone do not establish integrity. Join owned processes before cleanup. Link fixtures must remove
their owned links without traversal before returning to scratch cleanup, which rejects reparse points.

## Required outcomes

Use the real Windows operation adapter and persisted catalog/journal for execution checks. The
[execution plan](../plans/r8-file-organization.md#windows-capability-admission) owns API admission.
Fault injection supplies a reproducible interruption or error; assertions inspect persisted results
after recovery. Test Ame's parameters, authority, result mapping and recovery rather than duplicating
the system's internal copying algorithm.

| Scenario | Required observation |
| --- | --- |
| Preflight, rejected or stale authorization | Source and target paths and bytes stay unchanged; a changed selection cannot execute an earlier request |
| Copy, same-volume move or rename | Only authorized paths change; content is preserved; copy keeps its source; catalog identity and durable album/favorite decisions remain correct |
| Numbered collision naming | Existing files stay unchanged; occupied suffixes and racing destination creation lead to another available name, never overwrite; actual names reach the catalog and TXT report |
| Locked file, unavailable root, invalid name, Chinese or long path | Supported paths work; rejected items retain their sources and existing targets; partial results identify the affected items |
| Root replacement, path escape, reparse point or changed target | Execution stops before an unauthorized write; out-of-scope sentinel files stay unchanged |
| Cross-volume move | A complete verified target is published before authorized source removal; a failed or interrupted copy retains the source |
| Cancellation, database failure, process termination | Restart reconciles disk, journal and catalog without unapproved loss, repeating a completed mutation, or reporting unfinished work as complete |
| Delete confirmation and supported system restoration | Each new confirmation defaults to recycling; cancel/close leaves files intact; an explicitly checked and confirmed permanent deletion is recorded without a restore action; unrelated files and Recycle Bin contents stay untouched |
| Non-recyclable item and Windows fallback prompt | Declining or cancelling preserves the file; affirmative native confirmation permits only the confirmed deletion; absent confirmation cannot authorize permanent fallback |
| Preferred file deleted in the ordinary gallery | Only the bound file is deleted; a remaining duplicate becomes the persisted preference and displayed representative, or the card disappears when none remain; unchanged failed/cancelled files keep their preference; a late callback cannot delete the replacement |
| Multiple album destinations and TXT result report | Ambiguous items are skipped without changing files or memberships; other items proceed; final feedback and one uniquely named report contain actual exceptional outcomes; clean runs create no report and report-write failure does not replay mutations |
| Native completion and progress callbacks | Overall success with aborted or failed items is not reported as full success; actual renamed destinations reach the catalog; estimates and stopping states remain accurate |
| Cloud-only or changed source | No implicit hydration; stale or unavailable evidence cannot authorize mutation |

Inject failures at Ame-owned boundaries: durable intent, native submission, receipt of an item
result, and catalog reconciliation. In native integration, also interrupt real transfer and exercise
disk-full, collision, lock and cancellation behavior. Do not fabricate callbacks for undocumented
Windows internal stages. A missing completion callback must leave a recoverable uncertain outcome.
Check both sides of a durable transition only where recovery differs; avoid equivalent combinations.
Recovery preserves the original authorization scope and rechecks files before further mutation.

## Windows integration and workload selection

Small integrity and recovery fixtures run for changes affecting file operations. Desktop checks
cover choosing an action, its target, conflict resolution, progress, cancellation and recovery with
sample copies; no preview UI is part of acceptance. Existing unrelated UI changes do not acquire a
new full filesystem suite. Follow the [test-design standard](../development/testing.md) and
[existing CI selection](quality/gate-selection.md).

Sandbox and hosted Windows integration verify actual file locks, volume identity, supported recycle
behavior and native publication semantics. Cross-volume acceptance requires two genuinely distinct volumes
in that disposable environment; two directories or an injected cross-volume error do not prove it.
Prepare a bounded disposable volume fixture there when implementing this workflow, with cleanup
scoped to its created resources. Do not repartition or configure the workstation for these checks.

Permission and disk-full errors can be injected for focused recovery tests; native guarantees still
need their applicable Windows integration evidence. Include an oversized or otherwise non-recyclable
fixture and exercise both answers to the native permanent-deletion prompt, without a Yes-to-All
shortcut. A completion callback alone does not prove prior authorization. Test restoration only for
successfully recycled items through the admitted system route and restart Ame to verify its
independent history, without relying on Explorer's undo stack. A missing runner capability
needs a scoped equivalent Windows check or remains an explicit gap for that action. Small fixtures
establish correctness; large batches and sustained resource checks belong to affected performance
acceptance or release qualification.

The native probe supplies disposable operation and volume fixtures. The durable executor,
interruption recovery and connected desktop workflow retain their separate implementation and
verification work in the roadmap.

## R8-01 native capability admission — 2026-11-09

An isolated Windows Sandbox probe exercised `IFileOperation` with newly created 39-byte text files.
The sandbox had networking and clipboard redirection disabled and a read-only leased input mapping.
The executable was copied into guest-local storage before the successful runs; executing directly
from the read-only mapping had failed to create a guest fixture. The owned sandbox was stopped and
the leased staging directory cleaned afterward. No source-library files or production catalog were used.

| Observation | Adapter consequence |
| --- | --- |
| Copy, same-volume move and rename retained the exact fixture bytes; copy retained its source | Native transfer mechanics are viable; final target identity and per-item results still need production integration |
| A copy collision produced `图片(1) - 副本.txt` without replacing the existing `图片(1).txt` | Use native fail-if-exists transfers with Ame's numbered-name allocation instead of Shell automatic naming |
| Move, rename and recycle returned `COPYENGINE_S_DONT_PROCESS_CHILDREN` (`0x00270008`) as the per-item success | Do not require numeric zero for success or treat every positive HRESULT as completed work |
| Rejecting `PreCopyItem` with `E_ABORT` left the source intact and created no destination; the abort flag alone remained false | Combine execution return, item callback and abort flag when mapping cancellation |
| Recycling removed the original path and returned a Recycle Bin item | Keep recycling distinct from permanent deletion; a callback alone does not establish fallback authorization |
| Declining the native permanent-delete prompt retained the file and produced no item callback; accepting a separate new sample removed its source and returned no recycled item | Preserve refusal, cancellation and permanent deletion as distinct results |
| Windows Recycle Bin's selected-item restore removed the test entry from the bin; a subsequent guest check read the exact original 39 bytes at the original path | The system Recycle Bin UI is a verified recovery route for this fixture |

The production adapter was then exercised by `rust/examples/file_operation_probe.rs` in a new
Sandbox, with guest-local samples and a new 96 MiB expandable VHDX. The two source/target volumes
had distinct native identities. The main fixture is a fixed 51-byte text payload; cancellation
uses 64 MiB and non-recyclable fallback uses 24 MiB. Sibling sentinels remain unchanged.

| Production-adapter check | Observed result |
| --- | --- |
| CopyFile2 copy, same-volume handle move/rename, occupied numbered destination | Exact bytes and expected identities retained; existing destination unchanged |
| Cancelled request, escaping filename, source changed after preparation | No requested destination created; source retained |
| Copy cancellation after the first 1 MiB of a 64 MiB file | Cancellation requested at 5.8196 ms; the complete fixture including source-hash verification finished at 46.8469 ms; original hash unchanged and partial destination absent. This is a local sample observation, not a throughput benchmark. |
| Verified move completion with intentionally different destination bytes | Original retained; after a fresh matching destination proof, only the original was removed |
| Cross-volume move | Handle rename rejected the different volume unchanged; native copy followed by held-identity byte verification and source removal preserved the complete destination |
| Recycle and explicit permanent deletion | Recycle returned the system-bin recovery route; refusing the system permanent-delete prompt preserved the sample; confirming a separate sample removed it without a recovery route |
| Non-recyclable fallback on the disposable volume | Windows displayed its actual oversized-file warning; refusing retained the complete original hash, accepting a separate sample removed only that file without a bin recovery route |

The tested probe executable SHA-256 was
`4a778fa94ed5b7a3a5afbb23dd091979949ad8e80117b6c591e681b280373b6c`.
Build and warning-denying Clippy checks passed for the library and probe. A subsequent review made
thread-start failure return an error and distinguished an interrupted mutation lane from a busy
one; neither changes the admitted Windows operations. The dependency-boundary check passed after
correcting the composition metadata's decision anchor. The 11 existing root-availability checks
passed with the new module declarations registered; no availability guards were removed.

Native capability evidence supports the durable application layer. The operation journal,
numbered-name allocation, interruption/restart reconciliation and desktop authorization are not
enabled by this adapter alone. Racing destination creation and interruption at persistence/native
boundaries remain part of integrated recovery verification. The owned Sandbox and its leased host
staging directory were retired after the run; no production catalog or real library was used.

Reviewability: the largest new production owner, `shell_delete.rs`, has 354 production lines and
no inline tests; the dedicated native probe has 450 lines across its entrypoint and scenario files.
The existing `local_files.rs` retains 2,607 lines before its inline-test module and 3,364 inline-test
lines; its change registers the separate 53-line operation-source owner and admits the two new module
declarations in the existing source-safety loading registry. Its availability call-closure guards
and production behavior are unchanged.

## R8-02 durable journal foundation — 2026-11-09

Schema v79 adds immutable physical-operation authorization and versioned transition history under
[ADR 0042](../architecture/0042-windows-file-operations.md#durable-intent-and-outcome-journal).
The journal binds exact catalog/source identities, retains copied-destination evidence separately
from source removal, and does not execute pending records when reopened. Native admission and
catalog publication remain separate transitions; history queries are bounded to 128 items.

Four focused SQLite fixtures verified rejected/stale authorization, atomic rollback when intent or
result recording fails, idempotent authorization, stale callback rejection, bounded history, and
reopening a copied move without restarting it. They also rejected a corrupted journal head. The
forward-migration fixture retained existing albums and organization history and rolled back a
partially failed schema upgrade. The existing album-icon migration regression also passed.
Warning-denying Clippy and the fixed dependency-boundary check passed. These checks do not exercise the forthcoming
application executor or establish integrated restart/catalog reconciliation.

The largest new production policy owner has 214 lines with no inline tests; the journal and
migration fixtures contain 361 dedicated test lines. Existing coordinators retain 2,988 production
lines in `sqlite_catalog.rs`, 4,195 production and 2,289 inline-test lines in `migrations.rs`, and
430 production lines in `current_schema.rs`; additions there only register the new scoped owners.

R8-01 was committed as `7a5288e1`. Branch pushes do not start the repository's main/PR CI workflow;
hosted verification remains pending. No new PR or release was created, and no real source media or
production catalog was used. The durable executor, disk reconciliation, catalog publication and
desktop controls remain R8-02/R8-03 work in the roadmap.
