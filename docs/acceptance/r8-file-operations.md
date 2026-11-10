# R8 filesystem verification

Status: verification procedure and scoped R8-01 through R8-04 evidence; integrated R8 acceptance is pending.

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
System recycling requires a supported identity-bound implementation; normal recycling/restoration
evidence does not cover it. The 2026-11-10 product clarification retains normal recycling and requires
changed/unverifiable items to be preserved with visible reasons. Globally disabling recycling does
not fulfill that requirement; the post-validation replacement race remains unresolved.
`StorageFile` was not admitted as a replacement: Microsoft's
[storage-object explanation](https://blogs.windows.com/windowsdeveloper/2014/06/20/common-questions-and-answers-about-files-and-app-data-part-2-files/)
describes it as a path reference that does not hold the file open. No identity-bound recycling
guarantee was established for that alternative.

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
Windows Gate succeeded. This resolves the hosted failures above. The recycling identity boundary
remains unresolved and prevents R8 delivery; no merge or release was performed.

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
