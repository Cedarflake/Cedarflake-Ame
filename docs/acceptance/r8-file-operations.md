# R8 filesystem verification

Status: verification procedure and R8 native, application, desktop and integrated-review evidence.

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
| Merged-card deletion and explicit preferred-member deletion | A merged ordinary-gallery card deletes its confirmed frozen group; a group-member or single-file action deletes only its bound file. Remaining duplicates receive a persisted preference; empty groups disappear. Failed/cancelled files keep their preference, and late callbacks cannot delete a replacement |
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
Warning-denying Clippy and the fixed dependency-boundary check passed. Those foundation checks cover
persistence and migration; the executor checks below cover their connected admission order.

The largest new production policy owner has 214 lines with no inline tests; the journal and
migration fixtures contain 361 dedicated test lines. Existing coordinators retain 2,988 production
lines in `sqlite_catalog.rs`, 4,195 production and 2,289 inline-test lines in `migrations.rs`, and
430 production lines in `current_schema.rs`; additions there only register the new scoped owners.

The port-based executor now records selected numbered names before submission and separates native
copy, durable copy receipt, source-removal admission and native completion. Restart inspection uses
parent identity and file-state evidence without replaying operations. Two additional cases use actual
SQLite with a controlled native collaborator: a failed start write makes no native call; a failed copy
receipt write preserves the original; a collision selects `图片(1).jpg`; and reopening after a lost
source-removal callback recovers the retained move result without another mutation. All six scoped
journal/migration/executor cases and warning-denying Clippy passed. The collaborator supplies controlled
file observations; these cases do not replace the native Sandbox evidence or desktop acceptance.

The executor has 231 production lines, restart inspection has 128, and their dedicated SQLite tests
have 249 lines. The shared transition owner now has 204 production lines with no inline tests.

R8-01 was committed as `7a5288e1`, and the journal foundation as `64c6add2`. Branch pushes do not start the repository's main/PR CI workflow;
hosted verification remains pending. No new PR or release was created, and no real source media or
production catalog was used. Subsequent catalog/native integration evidence follows below; runtime
and desktop delivery continue under the roadmap.

## R8-02 catalog publication and native recovery — 2026-11-09

The executor was committed as `6f3d58d3`. Catalog publication now uses the existing location,
source-generation and organization owners in one transaction with its journal result. Moving retains
favorite/album association; deleting a selected preferred file saves a remaining exact duplicate as
the replacement. A compatible target already published by synchronization retains its observed
generation. Pending deletion notifications use the v80 point index and update preference in their
own catalog transaction, even when the native result has not yet been recorded.

The eight scoped SQLite/executor/migration cases passed in 29.08 seconds. The publication cases
exercise both notification-first and operation-first ordering, rollback of a rejected publication,
retained favorites, exact selected-file removal, persisted replacement preference and idempotent
retry. Reopening a v79 journal retains an unfinished copied move through the v80 upgrade. The first
notification-order run exposed Windows-style relative paths in the new publication path; using the
existing slash-normalized catalog format resolved it. Warning-denying Clippy passed for all targets,
and the fixed dependency inventory passed in 34.00 seconds.

The explicitly isolated native fixture
`application::file_operations::native_tests::file_operation_sandbox_recovers_native_move_and_publishes_copy_without_replay`
then passed in Windows Sandbox in 5.00 seconds. It is ignored by ordinary workstation tests and
requires the guest identity before creating any files. A generated 157-byte PNG, sibling sentinel,
catalog and destinations were all guest-local. The host input mapping was read-only; networking and
clipboard redirection were disabled.

| Native integration boundary | Observed result |
| --- | --- |
| Same-volume move followed by a rejected receipt write | File moved once; journal remained executing; reopening and disk inspection recovered the completion without replay |
| Output held for metadata/publication | Image dimensions were read through the existing inspector; a competing writable open was rejected |
| Rejected catalog-publication commit | Old catalog location remained and destination bytes were preserved; retry published the moved location and retained its favorite |
| Numbered native copy outside registered roots | Existing target and source stayed intact; `图片(1).png` contained the original bytes; no new root was registered |
| Fixture containment | The sibling sentinel was unchanged; the owned Sandbox stopped and its leased host staging directory was cleaned |

The executed test binary SHA-256 was
`902ad5a919ea3e32052a1549571b1975f0ddd432a6149184c5067436bfedf946`.
This proves the native/persistence connection for the named fixture, not a desktop confirmation flow.
Cross-volume native behavior retains the R8-01 evidence; its durable ordering retains the executor
fault cases. The following record covers joined runtime and scan ordering; desktop actions retain
their separate interaction verification.

Reviewability: catalog publication has 250 production lines, application publication 121 and the
guarded output adapter 132, with no inline tests. The publication fixtures contain 436 dedicated
test lines and the isolated native fixture 253. The shared location-mutation owner has 154 production
lines; the 2,044-line catalog-delta facade only exposes its existing scoped helpers in this change.

## R8-02 joined runtime and scan ordering — 2026-11-09

The application task opens its validated catalog on one joined worker and serializes execution with
recovery. A controlled active native call verified stopping state, refusal of a second task, joined
close, durable cancellation of the unstarted item and retention of the original catalog location.
The existing deletion case now also publishes a complete replacement scan before the native receipt;
that scan saves the remaining exact duplicate as preferred in its own transaction.

All nine scoped journal/executor/publication cases passed in 35.60 seconds; the separate guest-only
case was skipped on the host. Warning-denying Clippy passed. Existing scan-unwind/rollback coverage
passed, and the fixed dependency inventory passed in 33.72 seconds with the exact production-factory
admissions. No schema version or native API changed in this increment.

The guest-only native fixture then passed in Windows Sandbox in 5.78 seconds with its numbered copy
submitted through the production task factory. Progress reached finished with one published result,
close joined successfully, and original/occupied/sibling bytes remained intact. The executed binary
SHA-256 was `c226243f405aa85685764945acbbc8b582277d7876d71dda4b2ae5c5c67b6202`.
The owned Sandbox stopped and its leased host staging was cleaned. This remains native/application
verification; desktop confirmation, rule organization and integrated hosted CI are outstanding.

Reviewability: the task lifetime owner has 162 production lines, its execution loop 110 and scan
correlation 90, with no inline tests. The cancellation fixture has 172 dedicated test lines; the
existing native fixture now has 284. The 252-line scan transaction owner only sequences the typed
correlation owner around its existing projection publication.

## R8-03 connected desktop actions — 2026-11-10

The typed desktop bridge connects selected physical locations and query-wide selections to the
existing executor. Preparation uses disk-backed SQLite selection tables and bounded pages; confirmation
binds the submitted action and destination. Gallery menus, batch actions and the viewer's upper-right
delete button use the shared dialog and feedback components. History reads 40 records per page;
unfinished outcomes require inspection, and retry creates a fresh selection and confirmation.

The Debug client ran in a 3-GiB Windows Sandbox with read-only host inputs, no network or clipboard,
and guest-local source files, catalog and targets. No real source root or production catalog was used.
The tested native DLL SHA-256 was
`91d78116901a998d8429d90c2ee19b74e96856adcb0d3291fba2e558089ee09f`.

| Desktop interaction | Observed result |
| --- | --- |
| Copy and move through the context menu and system folder picker | Completed outputs retained the fixture bytes; catalog counts followed the actual source changes |
| Case-only rename | `sample-2.png` became `Sample-2.png`; all five baseline hashes remained unchanged and only the current spelling appeared in the five active catalog locations |
| Destination removed between picking and confirmation | The dialog requested another folder; source files remained intact and no destination was recreated |
| Delete confirmation and cancellation | Permanent deletion was initially unchecked, selecting it changed the warning, and cancelling performed no mutation; the next confirmation reset it to unchecked |
| Copy 1,285 selected files to an occupied destination, then stop | The UI and journal agreed on 1,051 completed and 234 cancelled items, with zero failed items. Existing outputs remained intact, new outputs used numbered names, and all 1,285 source hashes and 2,336 output hashes matched their expected contents |
| Viewer deletion and system restore | Recycling `batch-1280.png` reduced physical/catalog counts from 1,285 to 1,284 and persisted a system-bin receipt. The history action opened Windows Recycle Bin; restoring that one entry restored its bytes and both counts to 1,285 |

Desktop verification exposed two integration defects. Delayed scan observations could republish the
old spelling after a case-only rename; handle-based filename revalidation now rejects that stale
observation without reading source content. Shell parsing also rejected an extended filesystem path
with `0x80070057`; the Shell boundary now uses the normal drive/UNC spelling while retaining identity
and source-state validation. The guest native fixture covers case-only recovery, a competing hardlink
name and recycling an extended-path source. Its executable SHA-256 was
`423f6fde374a0e49e72d3f6f34f90e8ad701ad1973e40df62e162c3531f8617c`.

Focused SQLite selection/execution checks, the delayed scan/case-alias regressions, Rust dependency
boundaries, warning-denying all-target Clippy, Dart analysis and the Windows Debug build passed.
Four focused Flutter cases cover joined shutdown, preserving a worker failure during stopping,
retiring a late selection handle and resetting deletion consent. The final stopping-error correction
was verified at the controlled lifecycle boundary; it changes no native action or successful UI path.
Hosted CI has not run for this branch, whose pushes do not trigger the main/PR workflow. Rule-based
organization and exceptional-result TXT output remain R8-04 work.

Reviewability: selection lifetime, authorization, execution paging and SQLite selection have
219, 225, 200 and 209 production lines respectively, without inline tests. Desktop lifecycle,
confirmation and history have 209, 210 and 235 production lines. Their dedicated selection, native
and Flutter fixtures have 184, 359 and 223 lines. The existing `local_files.rs` facade retains 2,543
lines before its 3,364-line inline-test module; its revalidation responsibility was moved into the
existing scoped owner rather than extended in place.

## R8-04 organization rules and result reports — 2026-11-10

Schema v81 records immutable export scope, skipped items and independent report status. Copy/move
confirmation selects direct, category, album, date or source organization through the existing
choice control. Rules freeze against the selected catalog snapshot; native authorization rechecks
source and destination identities. Exceptional outcomes produce a unique UTF-8 TXT in the selected
root; report regeneration reads history and cannot submit media operations.

Focused SQLite checks cover the forward migration and rollback, retained export authority,
independent report failure/retry, album conflicts, classification, date and source destinations.
The first rule fixture omitted required EXIF provenance; completing that fixture made the scoped
rule case pass. The existing migration fixture now downgrades through the canonical migration chain
instead of leaving newer tables behind. Four existing Flutter lifecycle/consent cases, scoped Dart
analysis and the Debug build passed. Hosted checks remain pending.

The guest-only native export case passed in 4.70 seconds. It preserved an occupied destination,
published `图片(1).png` under the selected album, skipped an unassigned image, retained the original
report when a report-only retry failed, then generated a second unique report without another copy.
A clean date-organized batch produced two verified outputs and no TXT. Sources and the unregistered
target-root status were unchanged. The executed test binary SHA-256 was
`4551b9a459eb591f889d2566291d58c99a7a1e71d2421f6d516a3e8592d87151`.

The desktop Debug client used the same isolated 3-GiB Sandbox policy as R8-03. Its native DLL SHA-256
was `d3b8c2fd9bf28fa7910f7587b018e08d83e748c9c6555dcafb146996c34c23d2`.

| Desktop interaction | Observed result |
| --- | --- |
| Select four displayed images and copy by date | Four outputs appeared under `2026-11`; each SHA-256 matched its original. The five physical sources and catalog locations remained unchanged. |
| Copy one image without an album using the album rule | Zero copied, one skipped; feedback identified the skip and a UTF-8 report recorded the source and missing-album reason. |
| Make the guest target unavailable and regenerate its report | History displayed report-write failure while preserving the media-operation results. Restoring the same directory and retrying saved another uniquely named report. |
| Read back actual files and catalog | Exactly four media-operation records, two export batches, one skip and one registered source root remained. Both reports were present, no temporary report remained, and the sibling sentinel was unchanged. |

The temporary PowerShell check initially read its own Chinese literals using the guest's legacy
encoding; saving that check with a UTF-8 signature corrected the assertion. Both generated TXT files
decoded as strict UTF-8. The client closed normally, the owned Sandbox stopped and its leased host
staging directory was cleaned. No real media or production profile was used.

Reviewability: export persistence, rule resolution, native directory/report handling and report
formatting contain 251, 165, 155 and 141 production lines, with no inline tests. Selection
authorization remains 217 lines with its 105-line candidate owner; report task coordination is
82 lines. Desktop action lifetime, file history and export history contain 228, 255 and 147 lines.
The export migration contains 91 production and 67 inline-test/helper lines. Dedicated persistence/rule
and guest-native export fixtures contain 256 and 244 lines respectively.

## R8-05 integrated review — 2026-11-10

Review of `bb617233` reproduced a deletion admission gap in an isolated native Shell probe.
After `PreDeleteItem` admission, the probe renamed the original and created another file at its
old path. Windows recycled the replacement, leaving the retained original intact. Holding parent
directories and checking identity before callback return does not bind the subsequent Shell
operation to that file. The probe executable SHA-256 was
`891065c908534f664ca1db1a5c1c9c65e0dbe1f13f2d2a312df6aba7acc1130d`.
This initially blocked identity-preserving recycling. The final 2026-11-10 product decision retains
normal Windows recycling with the accepted post-validation replacement race under
[ADR 0042](../architecture/0042-windows-file-operations.md#recycling-identity-boundary).
Detected changed/unverifiable items still remain undeleted with visible reasons. The race remains
a known limit rather than a repaired behavior; globally disabling recycling is not the selected outcome.
`StorageFile` was not admitted as a replacement: Microsoft's
[storage-object explanation](https://blogs.windows.com/windowsdeveloper/2014/06/20/common-questions-and-answers-about-files-and-app-data-part-2-files/)
describes it as a path reference that does not hold the file open. No identity-bound recycling
guarantee was established for that alternative.

A further isolated probe supplied the original file ID through `IFileSystemBindData2` and
`STR_FILE_SYS_BIND_DATA_WIN7_FORMAT` before the same pre-delete replacement. Windows called
`GetFileID` once, but still recycled the replacement: the original remained at its retained name,
the replacement path disappeared, and the callback reported `0x00270008`. The parser's ID therefore
does not close the execution race. The [interface](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nn-shobjidl_core-ifilesystembinddata2)
and [binding keys](https://learn.microsoft.com/en-us/windows/win32/shell/str-constants) describe parsing
metadata, without an identity-bound deletion contract. Probe SHA-256:
`8463e5ac549fda03e5a3592026301ce4648ea1d716b16aa8f0ca0a5d1442a454`.
This used guest-created files, read-only executable inputs and a separate empty receipt mapping;
the owned Sandbox and scratch were retired. No product mutation route changed.

A held-handle probe then compared read/delete sharing with read-only sharing. Normal recycling
succeeded with delete sharing, while a concurrent write open failed with `os error 32`. The same-name
replacement still succeeded and Windows recycled that replacement. Denying delete sharing preserved
the file but also prevented normal recycling. Probe SHA-256:
`a5ce6644981f6c8e9fa19f0ba89b56cac8fbea5e76f916bd1e61c1c910a7c5a8`.
All fixtures were guest-created; the receipt-only output mapping, owned Sandbox and leased scratch
retired after readback. This evidence supports the accepted limitation, not a claim that the race closed.

The production adapter now holds that write-denying source handle through Shell execution and
callback retirement. It releases the handle before checking the final path so an explicitly confirmed
native permanent fallback can finish unlinking. The existing native probe rejected an active writer
without changing bytes, then recycled the fixture after the writer closed. On a disposable 96-MiB
guest volume, declining the native oversized-file prompt preserved all 24 MiB and their BLAKE3 hash;
accepting the next fixture's prompt removed only that fixture and returned no recovery action.
All three scenarios passed with production probe SHA-256
`f504b8e4f55ccb0ff4d7239909c28b4582659b1c85f7b714a028669ca6b4948b`.
The owned Sandbox and scratch retired. One source-guard regression passed using leased temporary
storage: existing/new writers are denied, Shell-compatible renaming remains allowed, stale identity
is rejected, and releasing the guard restores write access. No broad test suite was added.
The source-opening owner contains 86 production lines and 59 inline-test lines; the Shell deletion
owner contains 361 production lines and no inline tests, with its behavior covered by the guest probe.

Explicit permanent deletion now uses the same guarded `FileDispositionInfo` removal owner as
cross-volume move completion. In a fresh guest, cancellation and a stale request preserved both
the original and a replacement; a new authorization removed only the original. Move completion
also rejected different copied bytes and removed its source only after matching verification.
Both native scenarios passed; probe SHA-256 was
`d2f28bc862ed148c3675578f9675b4b93e1b38d4c2fd9bba81541ee2cc4a5421`.
Each Sandbox used guest-local files and read-only host inputs, then stopped and retired its scratch.
The shared removal owner has 62 production lines and no inline tests; the existing guest probe
retains cancellation, stale-source and selected-identity assertions.

[Hosted full verification](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42683368989)
targets `bb617233`. Windows build, scan, accessibility and three Rust partitions passed. Flutter
reported two obsolete menu assertions (six entries and no deletion action). Those assertions now
check lazy menu creation, keyboard/copy-path behavior and the admitted file actions; 18 scoped
Flutter checks passed locally.
Static failed in the release-script ZIP fixture's Windows PowerShell 5.1 assembly loading and
cleanup; the [cause-level correction](unsigned-windows-release.md#release-artifact-continuation--2026-11-09)
passed its explicit 5.1 entrypoint locally. The earlier local pass used PowerShell 7.
Catalog reached its 90-minute job deadline after 931 of 950 cases passed, with no failed assertion;
its log continued completing cases until cancellation. Approximately 18 minutes belonged to
classification and 13 to organization. Those existing cases now run in the scan/synchronization
and schema/queue partitions respectively, retaining four workers, serial cases, all-target/all-feature
inventory coverage, original assertions and deadlines. Partition admission/rejection checks passed
in PowerShell 7 and 5.1; the existing 3,046-case library inventory retained exactly one owner per case.
Workflow lint and hosted wiring checks passed in their PowerShell 7 execution context. An extra
PowerShell 5.1 invocation of the unchanged hosted-wiring check encountered its JSON-array projection
incompatibility; that check remains owned by PowerShell 7. This run predates these corrections and
the permanent-deletion repair.

The [follow-up run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42698940352) targets
`cecc4663`. Every product test and build job passed; Catalog completed in approximately 49 minutes
and the longest Rust partition in 55 minutes. Static found the two generated export bridge files
absent from the exact generated-file roster. Both are now explicitly registered; dependency
verification, all-target/all-feature Clippy and Dart analysis passed locally.

[Final full CI](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42707789166) passed on
`cc330c179d2378f4354e2f43eae089c9582e175b`, finishing at 22:53 UTC on 2026-11-09. Static, all four Rust
partitions, Flutter, Windows scan, native accessibility, Windows build verification and the aggregate
Windows Gate succeeded. This resolves the hosted failures above. The native recycling race was
subsequently accepted as the bounded limitation above; no merge or release was performed.

## Deletion feedback and confirmation — 2026-11-10

The confirmation now places the file count in its title, keeps the destination explanation on one
line, and places the unchecked permanent-deletion option before its label. Checking it changes
the action to permanent deletion. Changed/unverifiable deletion records show their reason directly
and require a fresh selection rather than retrying the previous confirmation.

In the actual Debug client inside Windows Sandbox, the normal recycling action remained enabled.
Holding a synthetic source open for writing made explicit permanent deletion fail with Windows
sharing violation 32. Feedback reported zero deleted and one unfinished; history displayed the
preserved-file explanation without opening technical details and offered no retry. All five source
SHA-256 values and the sibling sentinel remained unchanged after releasing the handles.
Scoped Dart analysis, the existing four confirmation/lifecycle tests and the Windows Debug build
passed. These checks cover the desktop change; they do not resolve the separate recycling race.
The mapping, confirmation, feedback and history owners contain 166, 242, 119 and 267 production-file
lines with no inline tests; the existing dedicated test file contains 237 lines.

## Merged-card deletion scope and readable paths — 2026-11-10

Ordinary merged-gallery deletion now freezes the complete compatible group before confirmation,
including current copies in other folders. Its typed selection cannot authorize copy, move or
rename. Group-member buttons and single-file viewers retain explicit-location deletion. The existing
SQLite selection regression covers cross-folder membership, all-matching exclusions, preference
changes, stale generations and rejection of retained unverified membership. The focused command
admission check and 19 affected Flutter checks passed, including confirmation reset and cancellation.
Bridge hashes and 39 asynchronous contracts, Dart dependency boundaries, handwritten Dart analysis
and the Windows Debug build passed.

The actual Debug client used six guest-local synthetic files in a fresh Windows Sandbox. Deleting
one member of a two-file group showed a one-file confirmation, recycled only that file and retained
the other member as preferred. Deleting a different merged gallery card showed a two-file whole-group
confirmation, recycled both files and removed that card. The system Recycle Bin displayed exactly
these three files; Ame history displayed three completed records with readable drive-letter paths.
The remaining three files matched their baseline SHA-256 values and the sibling sentinel was
unchanged. Host inputs were read-only; the Sandbox stopped and its leased scratch was removed.
Display-only path normalization leaves native execution paths intact. This workflow does not resolve
the separate post-validation Shell replacement race above.

Completed deletion feedback uses the affected-file summary as its single result line, omitting the
redundant completion heading while retaining the file detail and history/dismissal actions.
The selection toolbar joins its submitted operation, then retires that selection after a published
result. Cancellation or a zero-result failure retains selection; the existing intent/query guard
preserves newer selections. In a second Sandbox client, deleting a selected two-file group restored
the gallery heading and two remaining images, while the notice showed only the affected-file summary.
Both remaining file hashes and the sibling sentinel matched their baseline. The owned Sandbox and
scratch were retired. Existing application and selection regressions passed after the lifecycle
change; completion remains unavailable until native cleanup and catalog publication finish.

[Full CI](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42738000957) passed on
`500e551f5ce3aa051fce8b32db3bb61e9f1d01da` at 02:38 UTC on 2026-11-10. Static, all four Rust partitions,
Flutter, Windows scan, accessibility, build verification and Windows Gate succeeded. This run
precedes the dialog changes below and does not cover their implementation or the later recycle guard.

The selection facade and group expansion owner contain 255 and 98 production lines without inline
tests. Authorization contains 229 production lines and 30 inline-test lines; its SQLite selection
fixture contains 253 dedicated-test lines. The confirmation, history and group-card owners contain
251, 270 and 247 production lines without inline tests. File-action lifetime, selection toolbar and
feedback contain 232, 421 and 127 production lines; the affected application test file has 286 lines.

## Dialog screenshot inventory — 2026-11-10

The ignored local artifact `build/review/dialogs-2026-11-10/index.html` groups 37 actual-client
screenshots from `500e551f`, with entry points, states and a matching Markdown/JSON index. It covers
file confirmations, operation/report history, albums, folder removal/update, filtering and both date
modes, model installation, organization restoration/cleanup, thumbnail cleanup and notifications.
Two earlier failure screenshots are labeled separately; superseded path/header screenshots are
excluded from the current gallery. Images are unmodified, using the light theme and guest fixtures.

The additional `build/review/dialogs-special-2026-11-10/index.html` captures the interrupted-move
confirmation in both themes, restart/continuation, current error details, retry refresh and dark
album validation. Other theme/loading/error variants are not exhaustive. These inventories support
the requested layout review without authorizing a redesign.

The guest-only session created a temporary album, exported its archive, generated one skipped-item
TXT, and copied one sample. Restoration, cleanup and deletion confirmations were cancelled; model
download was not started. All six source SHA-256 values and the sibling sentinel remained unchanged;
the sole copied image matched its source, and the report decoded as strict UTF-8. The client closed
normally, then the owned Sandbox stopped and its leased staging directory was removed.

## Dialog templates and connected actions — 2026-11-10

The [dialog templates](../product/gallery/dialogs.md) now own confirmations, short forms, result
locations and diagnostic details. `AmeDialog` composes the installed Material UI 1.10.0 `AlertDialog`
with framework scrolling, focus, dismissal and styling. Its fields and object rows replace repeated
layout composition; they add no dialog controller. The existing bottom trays remain unchanged.
Official [dialogs](https://m3.material.io/components/dialogs/overview) and
[`AlertDialog`](https://api.flutter.dev/flutter/material/AlertDialog-class.html) were reviewed;
installed `InputDecoration.icon`, `CheckboxListTile`, `DatePickerThemeData` and
`GregorianCalendarDelegate` provide the required controls without a custom calendar or input owner.

The actual Debug client in Windows Sandbox verified inline album icon selection, saving and invalid
name feedback, album deletion identity, copy destination/rule selection, single-file rename, default
unchecked permanent deletion, and restoration contents with only nonzero rows. The album icon uses
the input's external icon slot with square constraints: its selected background stays circular,
the underline starts at the name, and validation text does not change its vertical alignment.
Rename remains single-file.
Cancellation preserved the draft's source files and existing decisions.

Copy/move organization, standalone settings, cleanup range and history mode now share the Material
text-button choice. Settings row actions use filled tonal buttons; the album action retains its picker.
The rebuilt Sandbox client verified selected markers, theme saving, album-picker dismissal, cleanup
range refresh and cancellation, budget selection, and switching file records to result reports.
The earlier organization check retained its value/explanation updates and cancellation behavior.
Formatting, scoped analysis, the existing 27 settings regressions and the Debug build passed; these
checks submitted no file operation. The updated screenshots are under
`build/review/menu-controls-updated-2026-11-10`. Storage settings has 726 production lines and its
dedicated settings-page test has 1,228 lines; no new tests or inline tests were added.
Sandbox verified filled actionable buttons, plain-text unavailable statuses, single-line catalog
sizes, and opening/cancelling the `清理` action. Scoped analysis, the Debug build and the 20 existing
cleanup/settings-page regressions passed. Existing action callbacks are unchanged.
The format-chip row now reserves vertical paint space inside its horizontal viewport. The rebuilt
Sandbox client verified light/dark selected and unselected edges, horizontal scrolling, and applying
JPG with PNG. Its four existing filter regressions, scoped analysis and Debug build passed.

A category copy with no classification produced one skipped item. A second direct copy held the
synthetic source open for writing and produced Windows sharing violation 32. Both completion notices
placed status and the relevant count in their first line, with the saved-report message below.
The actual failure details displayed the localized reason and a separately labeled error code.
Opening report locations selected their TXT in Explorer; changing the preview cache and restarting
exposed the old-directory confirmation, whose location action selected that directory in Explorer.
Cleanup and restoration were cancelled. The date picker displayed the date and weekday separately,
updated both when a different day was chosen, and returned the selected date to the filter draft.

The local artifact `build/review/dialogs-update-2026-11-10/index.html` contains 18 unmodified client
screenshots with entry points and coverage notes. The six synthetic source hashes and sibling
sentinel were unchanged; three result reports decoded as strict UTF-8, no image copy was published,
and the exported album archive remained available. The client and temporary lock retired, then the
owned Sandbox stopped and its leased scratch was removed. This run used the light theme; it does
not add evidence for the separate recycling identity race.

A fresh Sandbox rechecked the external icon's default, selected and invalid-name states, saved the
chosen icon, and reproduced the sharing violation with its `OS: 32` label. Screenshots 01–03 and 18
were refreshed. Its six source hashes and sentinel were unchanged, one report decoded as strict
UTF-8, and no image copy was published; the client, lock, Sandbox and leased scratch retired.
Scoped formatting, analysis and the Debug build passed for this follow-up.
The final copy preserves the original `os error 32` text instead of shortening it to `OS: 32`;
the special-state inventory below captures that current text. Formatting and scoped analysis passed.

Scoped Dart formatting, analysis, dependency verification, the Debug build, and retained album,
sidebar-removal, archive-scroll and file-action lifetime regressions passed. An initial nested
shrink-wrapped list could not supply intrinsic height to `AlertDialog`; the bounded item viewport
resolved it while retaining the selection and cancellation checks. Storage settings contains 721
production lines after extracting its 52-line cleanup confirmation. The filter sheet contains 396
production lines, album navigation 250, file confirmation 258 and album editor 193, with no inline
tests. The shared dialog/location/diagnostic/calendar files contain 77/99/32/15 production lines.
The two adjusted dedicated test files contain 346 and 273 lines; no new test inventory was added.

### Recovery and dark-state verification

The continuation confirmation now uses `AmeDialog` and names the affected file. A guest-only SQLite
trigger interrupted a real cross-volume move before source removal, retaining its durable `copied`
state; no history rows were fabricated. After removing the trigger, cancelling confirmation kept
both hashes unchanged. A normal client restart retained the continuation entry. In the dark theme,
Tab/Enter completed the move and refreshed its record; the target matched the original hash and only
that source location disappeared. Both confirmation themes appear in the 11-image special-state index.

The same client reproduced a stale-history defect: retry submitted work, then refreshed before its
completion. The history owner now joins the submission's existing completion future and loads the
latest page. A delayed-worker regression catches the ordering gap. In the rebuilt client, retrying
the real sharing-violation record displayed the new successful copy without reopening the tray;
the actual numbered destination appeared while the older failure remained historical evidence.
Failure details displayed `os error 32` separately from the localized explanation. The dark album
form retained circular icons and input alignment with its empty-name error; cancellation created
no album. An input-method check supplied a valid full-width question mark, so that rename and its
reversal establish Unicode-name handling, not rejection of ASCII `?`.

Final guest readback verified five retained sources, one moved image and two matching copies,
the untouched sibling sentinel, six operation records (five published, one failed), three strict
UTF-8 reports, and no Flutter exception/overflow matches in the captured client logs. The client,
temporary lock, Sandbox and leased scratch retired. Formatting, scoped analysis, the focused six-case
file-operation suite including the new regression, and the Debug build passed. File history
contains 288 production lines without inline tests; its reused dedicated test file contains 341 lines.
These scoped results do not change the native recycling boundary recorded in the integrated review.

The `d2665ffe` [PR check run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/42803484954)
reached two obsolete theme-control locators in Flutter and native Windows accessibility checks:
both required `OutlinedButton` after the settings control became a standalone menu. They now use
the theme control's stable key while retaining menu interaction, retained-tree validation and native
UIA checkpoints. Both cases in the affected Flutter semantics file passed locally; the native runner
remains owned by the subsequent PR check run.

### Interrupted transfer content continuity

Recovery without a durable same-volume receipt now requires the original version's supported
content fingerprint. The SQLite proof read uses the authorized generation, identity, revision and
length, rejects conflicting analysis/organization evidence, and releases the read before native I/O.
The adapter hashes the candidate under its existing write/deletion guard and retains the verified
snapshot for publication. Normal move/rename execution keeps its source handle through receipt
capture. No schema, bridge or ordinary-transfer hashing changes were introduced.

A fresh Windows Sandbox ran the production journal, executor and native adapter against guest-only
PNG files. An in-place edit preserved FileID, length and modification time but changed bytes and
revision: reconciliation retained `Uncertain`, published no receipt or destination association,
and left the edited bytes untouched. An unchanged transfer recovered and published successfully;
subsequent fingerprint publication accepted its organization binding. A transfer lacking original
content evidence remained uncertain. None of these recoveries replayed a native mutation.

The retained native workflow also passed lost-receipt recovery, publication rollback, favorite
continuity, numbered copy, case-only rename and collision checks. The recycling handle fixture
passed in the same guest. Its workstation run had been rejected by the source namespace ACL guard
before opening the tested handle; the other 15 selected non-guest cases passed locally. The new
SQLite proof case covers absent evidence, stale revision, durable-binding fallback and conflict.
The Sandbox fixtures finished in 6.13, 5.70 and 0.01 seconds. The binary SHA-256 was
`d49bf015ce84c2c5cdfc9656a739831d9376fd5f0a879a10fb58e1452451715e`.
The owned Sandbox stopped and its leased input/output staging was cleaned.

An independent source review found no remaining issue within the repaired recovery and receipt
capture boundaries. Warning-denying Clippy passed for all targets/features; the fixed Rust dependency
inventory passed in 34.43 seconds. Production line counts are 198 for application recovery, 206 for the native
facade, 45 for native content verification, 152 for the journal facade and 63 for proof lookup,
with no inline tests. The added native and SQLite fixtures contain 196 and 97 dedicated test lines;
the retained native fixture contains 364. Hosted current-head verification remains the PR owner.

### Same-directory transfer admission

Selection preparation compares verified parent identity and exact filename before authorizing a
move, so ordinary and extended Windows path spellings cannot enter collision renaming for the same
entry. The existing Sandbox export fixture was extended through the production selection/task path:
an extended source path and ordinary destination directory yielded zero executable items, one skip
with `文件已在目标位置`, unchanged original bytes/name and no numbered file. Same-directory copy still
created a matching numbered copy, and an explicit `.png` to `.PNG` rename succeeded.

That fixture, including retained organization/report checks, passed in 5.92 seconds with binary
SHA-256 `bf00b8b1cfb45d326645434cbee48084f59986643776e287a953cd7408e3ad79`.
Files and catalog were guest-local; the Sandbox stopped and leased staging was cleaned. Compilation,
formatting and diff checks passed. Candidate admission has 106 production lines without inline tests;
the reused dedicated native export fixture has 319 lines. No schema, bridge or UI change was needed.
