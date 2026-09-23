# R2c process lifecycle verification

Status: **Selected native close, checkpoint continuation and interrupted raw-inventory recovery verified**.

This record covers the 2026-09-22 generated-only method in the
[active execution plan](../plans/r2c-closeout.md#remaining-process-lifecycle-and-input-method).
Production source was `2004bafa56eec9c58c7fe6e3fd4f4f876cde5db6`. It adds no product behavior,
changes no acceptance threshold, and does not establish complete UX-01/02/08 or R2c acceptance.
The recovered input pair uses the same native/Dart artifacts on `16c215e`; the intervening changes
affect test contracts and documentation only.

## Boundary verification and preparation

The current shutdown, synchronization lifecycle, scan-control and retained-task interaction suites
passed 63 tests: 3 lifecycle-owner, 5 window-action, 15 scan-control, 32 synchronization and 8 retained
interaction cases. Those controlled races include late success/failure and pre-registration control;
they do not prove actual operating-system input or a complete process restart workflow.

The native entry uses the real bridge, real native start tickets and stop fences, the unchanged
250-millisecond poll policy, existing window close action and six-second shutdown bound. In-memory
preferences and marked Debug-only derived storage isolate it from the user catalog. An owned Job,
900-second parent deadline, 2 GiB client ceiling and 2 GiB minimum available system memory bound each
lifetime. The planned two interactive lifetimes also share a 30-minute deadline.

The first seed run `4c74a0d34be64a58a21241a6a961363c` failed with
`first_import_continuity_preflight_timeout`: the diagnostic invoked scanning before starting the
production synchronization runtime. The native first-import owner correctly returned not-ready
without a runtime owner and preserved its 15-second preflight limit. The process exited 1 after
16988 ms; its Job closed and cleanup had no failures. Keep its directory, executable identities,
bootstrap error, 58 memory samples and source receipts. It supplies no successful lifecycle evidence.

The corrected seed first awaits the real synchronization lifecycle's successful start. It uses a
fresh derived store and generated baseline while retaining the failed stores. Production source,
preflight, polling and scan/publication guards remain unchanged.

## Native results

| Phase | Run identity | PID | Result |
| --- | --- | --- | --- |
| Seed | `9163c521a30f4abeb26c94513e75da77` | 33956 | Passed; 12 published baseline images and a durable paused first-import checkpoint with 163 visited entries, 151 accepted images and zero issues; normal close 459.082 ms |
| StartClose | `7c42595bbecc4722aff225d98df6c844` | 30608 | Passed; close dispatched while the real start future was pending; same checkpoint and baseline retained; normal close 1307.317 ms |
| PollClose | `9a34a81942fe4ac5b154e262784d40ba` | 36912 | Passed; close dispatched while the real poll future was pending; same checkpoint and baseline retained; normal close 338.960 ms |
| Restore | `7f547c9c7e254ecb9b8dd4730d4223e6` | 3752 | Incomplete; paused task and baseline pixels observed, but desktop input could not be delivered reliably; owned process deliberately retired with exit -1 |
| Continue | No admission | None | Not run after the failed interactive phase |

Both pending-call targets naturally completed before native stop completed in these executions.
Their real overlap proves the close path while a call is outstanding, not a native result arriving
after stop. Controlled delayed-result tests retain that separate race obligation. All three passing
phases exited normally with no Job/monitor cleanup failures. Forced retirement of Restore is never
counted as normal-close evidence.

The observation review found two potential false passes. Memory records now use create-new run
identity and verify run/PID/parent ownership, including distinct PIDs across phases. Parent
verification now reads the complete trace after process exit and rejects late diagnostic failures,
running publication after stop, missing target settlement, and missing application-state observation.
A next-event-turn read observes the actual state after the application processes the bridge result;
this also detects a silent state change after the watch stream has closed. It delays neither the
original result nor shutdown. An unchanged initial stopped state need not emit another notification.
The evidence guardrail passed one positive and ten rejecting fixtures, including silent late state
mutation. Independent review and two changed-boundary rechecks consumed about 7 minutes 50 seconds;
the last scoped review found the previously reported observation gap closed.

There was one recorded diagnostic-only kernel transition after the successful seed, before
StartClose. Every native executable/DLL and production source stayed unchanged; subsequent phases
used the same new kernel, SHA-256
`6C2145B9D352E249AE915BCCBCF96EF99ADF6D9ABE38C3117A85762CBA60D839`.
Preparation, transition, admissions, raw traces and receipts remain under the ignored
`.build/r2c-process-lifecycle` and `.build/r2c-process-lifecycle-ready` diagnostics and their named
GUID-derived stores. Their Dart analysis and Debug builds passed. Initial diagnostic import/bracing
lint failures were corrected before native admission; they are not product verification failures.

## Source and resource evidence

The frozen corpus contains 10000 generated mixed-size/historical images, 10921494393 bytes; the
fresh baseline contains 12 images, 34549397 bytes. Full read-only checks cover exact membership,
content hashes, file identity, frozen size, creation time and modification time. The initial check
passed in 45.170 seconds, the failed-seed post-check in 41.110 seconds, and the corrected-set post-check
in 57.103 seconds. Baseline records prove identity, bytes and stable modification time during each
read; they do not independently freeze its original creation/modification times as the large oracle
does. No source mutation, hydration or real-library access was part of these lifetimes.

| Phase | Samples | Peak working set bytes | Sampled peak private bytes | Kernel peak commit bytes | Minimum system available bytes |
| --- | --- | --- | --- | --- | --- |
| Seed | 61 | 434163712 | 384307200 | 451358720 | 8801300480 |
| StartClose | 6 | 381964288 | 321007616 | 357277696 | 8173383680 |
| PollClose | 6 | 385294336 | 329330688 | 451608576 | 8172642304 |
| Restore | 1138 | 489881600 | 462151680 | 468815872 | 7592394752 |

## Initial desktop-input failure

The desktop tool returned `unknown screenshotId screenshot-0` twice, including after fresh window
selection, and `coordinate input geometry is unavailable` for the accessibility-targeted click.
A Tab operation returned without error but the client recorded no keyboard event. Focus subsequently
belonged to another application. Input was stopped; only the verified test executable, parent PID
and admission-time process was retired. The test process and monitor exited, its Job closed, and
cleanup reported no failures. No further input was sent to other applications.

This failed Restore cannot claim settings/menu/viewer keyboard interaction, an unchanged checkpoint
after a normal interactive close, or explicit continuation. It remains failed after the later pair
passes; its admission, process and source evidence are retained.

## Recovered real-input pair

A fresh Computer Use session delivers a click, the A key and normal Alt+F4 close to a disposable,
catalog-free native window. Its visible counters and append-only received-input log agree; the owned
process exits zero and its Job closes without cleanup failures. The corrected method uses a freshly
selected window and the documented coordinate overload without an explicit screenshot ID. It does
not inject input through PowerShell. The probe is retained under ignored
`.build/input-driver-probe/e9e59f3694bb4030b941159d35a05d64`.

New create-new admissions reuse the successful Seed/StartClose/PollClose results and the retained
paused catalog. They reject the retired failed Restore PID, wrong fixtures, missing preceding
results and already consumed admissions. One actual admission check and six rejecting fixtures
pass. Independent review identifies a deadline race at process exit; the corrected parent checks
the retained process exit time against both lifetime and shared interactive deadlines. Two accepted
and two rejected deadline fixtures pass. Review and its scoped recheck consume about 1 minute
55 seconds, with no remaining blocking finding in this admission boundary.

| Phase | Run identity | PID | Result |
| --- | --- | --- | --- |
| Recovered Restore | `fba0dd1d63594e86aeac4f24d2aeca19` | 23844 | Passed; actual settings, gallery, menu, two original-image previews, Right/Escape input and normal close; checkpoint and 12-image baseline unchanged |
| Recovered Continue | `01a1969711b849a58bad708ecbf43c5a` | 34820 | Passed; actual Continue click resumes the same checkpoint and publishes exactly 10000 corpus images plus the original 12 baseline locations |

Restore observes 760 frames, 537 with current gallery pixels and 71 with a menu. It records settings,
paused feedback, two distinct viewer locations and returned gallery pixels, without implicit resume
or any visible Retry frame. Before and after, the checkpoint retains 163 visited entries, 151
accepted images, zero issues and catalog revision 1. The original previews include 8000 × 8000 and
1500 × 12000 fixtures.

Continue begins with that same checkpoint. Its 648 observed frames retain current baseline pixels
with no visible Retry control. A complete bounded traversal reads 32 pages, each at most 320 items,
at catalog revision 2: 10012 distinct locations, exactly all 10000 frozen relative paths and all 12
original baseline identities. Membership verification completes 117681 ms after native startup.
This proves complete membership and baseline usability, not decoding every one of the 10000 images.

Both phases exit zero through the actual title-bar close action. Parent lifetimes are 195632 and
166147 ms; close-to-exit times are 438.2255 and 352.5795 ms. Their complete traces contain one native
stop, one stopped final state, no pending calls at audit and zero later running publications.
Both processes exit, Jobs close and monitors retire without cleanup failures.

| Phase | Samples | Peak working set bytes | Sampled peak private bytes | Kernel peak commit bytes | Minimum system available bytes |
| --- | --- | --- | --- | --- | --- |
| Recovered Restore | 727 | 904667136 | 1028005888 | 1374281728 | 4693176320 |
| Recovered Continue | 619 | 433717248 | 370167808 | 384323584 | 5349814272 |

Full before/after source checks pass in 49.394/48.772 seconds against the same frozen 10000-image
corpus and 12-image baseline. The timestamp limitation described above is unchanged. Raw results,
complete membership, source receipts, process traces and memory records remain in the existing
GUID-derived store named by `.build/r2c-process-lifecycle-ready` admissions. No source file was
modified and no real-root or cloud content was accessed.

## Remaining final-gate limits

These results close the selected interrupted-EXE restoration and explicit-continuation path. Actual
OS picker, pre-registration Pause/Cancel, task Retry/Cancel keyboard interaction, truthful portable
capability, the complete local UIA path and Release input retain their separate frozen obligations.

The [preview contract correction](r2c-browsing-diagnosis.md#preview-contract-ci-correction) retains
the concurrent hosted failure and focused follow-up. Its test-only changes do not invalidate these
native production artifacts. Hosted run `35701651170` on `16c215e` passes all ten required jobs and
its aggregate gate. Complete local Daily, C01/C02 and externally gated acceptance remain separate
obligations; that hosted success does not erase their retained local failures.

## Native restart after an incomplete raw inventory batch

The 2026-09-23 [process-loss method](../plans/r2c-closeout.md#native-process-loss-during-a-persisted-recovery-lease)
uses production source `2f20a65ad4779b0b6332265cc488c120e9054bff`, an actual Debug Ame EXE,
production synchronization/scanning and marked isolated derived storage. The retained generated
10000/512/2/2 roots stay unchanged. A fifth root contains exactly 10000 new, valid 8-by-8 PNGs;
its initial real scan is setup, not picker or large-image decoding evidence. The original mixed-size,
historical-date corpus remains the visible gallery. No real-root, service, Journal or cloud run occurs.

The diagnostic's initial Dart validation finds an unused import and an unbraced conditional;
both are corrected before launch. A focused Python test catches a same-name module import cycle,
also corrected before launch. The final three Dart sources analyze cleanly and the Debug build
passes in 26.3 seconds. Four initial ownership tests and actual-catalog SQL/import checks pass;
the changed continuation adds a fifth test distinguishing a yielded batch from an executing lease.
Method review adds successor-generation, natural-expiry, exact authority/run and spool-retirement
assertions. These are diagnostic corrections, not production defects or a new product gate.

### Preserved exact-lease capture failure

Run `3d1dd31356fb4b54ae62f422eb035d77` imports all 10000 new images, observes the new directory
unavailable while retained pixels remain usable, and restores that same generated directory identity.
A read-only observer captures P2 control 6528, generation 1, and inventory/authority
`watcher-gap-promotion-6528` while leased. The owned parent terminates only its retained Ame handle
(PID 16592, parent 17776). Exit is intentionally abnormal, code -1, within a 65459-ms parent lifetime.

The post-exit exact-leased assertion fails. Before termination reaches the worker, it commits one
128-entry raw batch and returns control 6528 to `pending`, refunds the attempt to zero and records
`metadata_inventory_required`. Generation 1, the original run and authority remain intact; the
spool/directory are `enumerating`, with zero published staged entries and exactly 128 provisional
raw entries. This is bounded continuation rather than lost work. The original observer error and
aggregate failure remain unchanged. The source directory is restored; both Jobs and owned
processes retire without cleanup failure. The failed helper's exit timestamp was not retained;
this is not a successful exact-lease or complete normal-lifetime result.

### Changed continuation of the retained crash

After a separate admission of only this persisted `Yielded128` state, run
`592aa4a0576d486895afda4595f8c1f2` starts a different actual Ame process (PID 5624, parent 41528).
It uses exactly the same EXE, kernel and DLL hashes. It does not repeat the crash, modify SQL,
reset the catalog, shorten leases or change clocks. The first observed successor is generation 2;
that observation time is not a precise lease-acquisition timestamp.

The read-only terminal oracle passes 107.987 seconds after its observer starts. At application
108090 ms, the production state is synchronized with `liveOnly` capability, and 12 current gallery
tiles have decoded pixels. The same control completes at generation 89, catalog revision 788;
all 10000 entries are staged, the original gap 6527 retains its consumed claim to control 6528,
the recovery authority is retired and the captured spool is logically `retired`. Exact final
membership is 10000/10000/512/2/2. The `id`, `root_id` and `status` projection of all ten scan
records—nine retained plus the explicit initial setup scan—is unchanged across restart, so no
new full scan supplies the result. This is not a comparison of every scan-record column.

Actual window observation shows usable retained images while the fifth root displays updating.
A subsequent native wheel action displays the next dated rows with decoded images; that action
occurs after convergence and is not evidence of input overlapping unfinished recovery. The real
title-bar Close exits zero in 443.3918 ms. The complete resumed parent lasts 173066 ms, with 1050
observed pixel frames and no latched application or cleanup error. Both Jobs, the observer and
the memory monitor retire; the observer exit timestamp is retained.

| Lifetime | Memory samples | Peak working set bytes | Sampled peak private bytes | Kernel peak commit bytes | Minimum system available bytes |
| --- | --- | --- | --- | --- | --- |
| Failed exact-lease capture | 239 | 398086144 | 350666752 | 362921984 | 6040629248 |
| Yielded-batch restart | 645 | 775090176 | 718376960 | 728723456 | 5497733120 |

The full 20516-file pre-crash source oracle passes in 83.047 seconds. The complete post-crash /
pre-restart oracle passes in 75.218 seconds. The final post-restart oracle passes in 82.084 seconds
(`integrity-1790150136800018600.json`), including exact identities, bytes, timestamps and all five
catalog memberships. Independent result review confirms this bounded result and its retained
limitations. Raw admissions, failures, process/memory/source receipts and the unchanged original
crash remain under
`build/integration-storage-564c1792d10d494eb328fd22a2914c72`; diagnostic owners are in ignored
`.build/r2c-crash-native`. Receipt SHA-256 values are:

| Receipt | SHA-256 |
| --- | --- |
| Failed crash parent | `DBC4EE90505732DD52365FE0855CE26EA749EE24A0BD8D3F2A10D01321F18264` |
| Resumed parent | `7E5F4E8A8F7116A3F7FE9B00BE8003291BEA0A8C92ACA93B531C48DE3E44461F` |
| Recovery terminal oracle | `6CC64EF319632E91EF971AD4AC07210839505462824F0FAF1AFF7D23517461DC` |

This establishes recovery of the selected incomplete raw inventory after real process loss.
It does not establish executing-lease expiry, exhausted retries in this same lifetime, exact source
read counts, continuation from entry 129 without rereading, physical spool reclamation, Release,
complete UX-07C or R2c. Controlled exhaustion/peer-eligibility cases retain their separate evidence;
C01/C02, final candidate gates and external acceptance remain open.
