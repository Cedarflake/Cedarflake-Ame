# R2c controlled closeout cycle

Status: C04/C05 have selected generated-client evidence; C06/C07 have focused correction evidence.
Native picker/task input and candidate gates remain unresolved.
C01/C02 and local Daily failure remain unresolved.

Current implementation checkpoints: `d91580d` adds the connected root-interleaving regressions;
`dd3f556` contains the queue-readiness correction, work bounds and observation measurements;
`8745198` contains the reviewed retained-connection proof adjustment and boundary regressions.
Their focused checks pass. The subsequent retained-connection candidate passes focused owner,
caller, complete priority-group and applicable lint checks. Independent review finds no new
actionable S0/S1 code defect; its one documentation correction passes the scoped recheck.
The first complete Daily fails in the existing R2c-R guardrail before Rust/Flutter tests. The
supplement repairs that owner, passes its review and guardrails, and obtains passing hosted checks.
The second complete local Daily instead fails C01's full recovery. Earlier native-probe and
File.Replace failures remain unexplained. The supplement has reached its stopping boundary;
the approved P2 follow-up below also stops without a causal correction. The count follow-up does
not change readiness.

This record implements the [bounded execution plan](../plans/r2c-closeout.md). The canonical
[roadmap](../roadmap.md) retains stage and queue authority. Findings belong in the existing
[interleaving ledger](r2c-interleaving-remediation.md); this record maps the fixed journeys to
evidence and records execution budgets rather than introducing another issue backlog.

## Functional-first continuation

The current instruction authorizes advancing the directly user-facing frozen journeys while
registering C01/C02 for final verification. Start from clean `37c0928` on `codex/r2c`; no code
correction or failed-gate waiver is inferred from this priority change. The execution plan owns
the additional bounded client observation. Its first target is real-process paused-import
restoration with usable browsing and explicit continuation, using generated fixtures only.
Current charges before this continuation remain discovery 176, triage 54, diagnosis/repair 306,
review 23 and recording 42 active minutes.

### Paused-import process observation

The controlled session runs from 15:57:00 to 16:09:35 UTC, inside its 30-minute deadline, on
unchanged product source `37c0928`. A real Rust scan completes 32 one-pixel PNGs in root B, then
pauses the first import of root A's 1024 PNGs after Started. The seed confirms 32 ready previews,
unpublished A, a durable paused checkpoint, drained scan streams and stopped synchronization.
Seed PID 86476 exits zero and its owned Job closes without cleanup failure.

The first actual client is launched in the default execution sandbox: logs preserve the paused
state but computer use cannot discover its window. Its exact owned PID 21460 is intentionally
terminated; exit -1 and successful Job closure are retained as an unsuccessful desktop attempt.
Launching the same artifacts on the interactive desktop resolves discoverability. PID 93884 shows
32 items and a paused task, supports Settings, root navigation, original-image open, Right and Esc,
then exits normally without Continue. PID 99672 reopens the same catalog with the task still paused
and exactly 32 visible items. Explicit Continue completes all 1024 entries; the gallery shows 1056
and root A shows 1024. Both roots report synchronized and the second client exits normally.
The two interactive process receipts have zero exit, retired processes, closed Jobs and no cleanup
errors. This strengthens UX-01C and the interruption/continuation path; it does not prove UX-01B's
pending-poll shutdown or UX-02B's pre-registration Pause/Cancel race.

Source manifest verification after all processes checks all 1056 files byte for byte. Read-only
SQLite inspection reports integrity `ok`, no foreign-key violation, both published scans completed,
counts 32/1024 and no unfinished scan. Ninety-two previews are ready and 964 unrequested previews
remain pending; no full-library decode is claimed. SQLite integrity is not application FULL-reopen
validation. A source timestamp-preservation assertion is not inferred from the byte-only manifest.

Exact Debug SHA256 identities are EXE
`DEE7E6CD4E0A621D310ED75FE7165C366556855ED473D357DE30A89EB6F510FA`, DLL
`6A6716C7CA02703CDC5670B1CF9912257EF6011B7A45450FE869903A7F75A632`, and Dart kernel
`9B42FEDD8AA98BEB15D247F8FDEDDCC519DFED133EACB5037137C8621D747BBC`.
Ignored evidence lives in `build/diagnostics/r2c-functional-result.json`,
`r2c-functional-artifacts.json`, `r2c-functional-session.json`, the prepared fixture's seed manifest
and four process receipts. The two serial builds pass with complete raw output in their GUID
`r2c-p2-raw-*` directories (51672 ms and 27153 ms, no stderr or cleanup failures).

One accessibility element index expires before input. Screenshot-coordinate actions work; the
automation tree can lag the changed screenshot. Text transport does not inject `1023`, whereas a
physical `1` key enters text and Return submits it. All 1056 matches are consistent with the
current absolute-path search contract because the fixture parent contains `1`; this is not an
exact filename-filter proof. These tool observations do not establish an ordinary-input defect
or close C02's independent whole-window gate.

The fixture is deliberately 1x1 and monochrome: it provides lifecycle/count evidence, no useful
mixed-resolution, historical-date, realistic decode-cost or Release-performance conclusion. The
latest request adds that missing coverage through the execution plan's representative workload.
Charge 55 active discovery minutes conservatively for this continuation and coverage preparation,
including the seed delegate's 14 minutes and four-minute existing-workload inspection. Discovery
is now 231 minutes; no diagnosis/repair or final Daily was consumed. Subsequent representative
work is charged separately under the explicitly revised phase allocation.

### Mixed large-image and historical-date observation

The frozen source contains exactly 10000 independent files, 9000 JPEG and 1000 PNG, with the
execution plan's twelve dimension quotas, including 200 images at 8000 x 8000. Seventy-two textured
templates repeat across the files; this is not 10000 unique photographs. Total source bytes are
10921494393. The manifest covers 201 months from 2010-01 through 2026-09 and 585 effective dates.
Three thousand JPEGs carry original EXIF capture time 2012-03-04 09:10:11 with recent filesystem
dates. The remaining files have historical creation and modification times, matching ADR 0008's
creation-before-modification fallback. Expected path sets, dimensions, bytes, file identities,
timestamps and year/month/day counts are frozen before import.

The existing dependency stack builds the temporary generator in 135937 ms and generates/verifies
the complete corpus in 151884 ms. The generator's kernel peak working set is 212029440 bytes,
below its 1 GiB ceiling. Its post-generation full reread proves source BLAKE3 and timestamps;
the independent oracle freezes SHA256 evidence. Continuous system-available-memory evidence was
not captured during generation; preflight and post-generation samples cannot prove that entire
interval's 2 GiB reserve. No generator rerun is used to replace that missing evidence.
Initial resource refusals, raw build/generation output and the archived generator remain in
ignored diagnostics. No dependency or product source change is introduced.

The first actual client starts at 16:41:17 UTC on 2026-09-09, using the unchanged Debug artifact
identities above. Import visibly completes with 10000 images and real first-screen previews.
Distant timeline navigation loads historical previews without another scroll; a 1024-square image
opens and returns to its gallery anchor. A bottom-rail click/drag discrepancy remains unclassified.
The 8000-square folder shows 200 images, but the attempted image-open action is rejected by the
tool's account-usage limit and is not executed. Later state is not attributed to those actions.

On resumption, the process is gone. Its receipt records exit zero, 1425144 ms lifetime, retired
process and closed Job with no cleanup failure; the cause of that unobserved exit is not inferred.
The observer records 3989 samples, kernel peak working set 486322176 bytes, kernel peak commit
591966208 bytes and minimum sampled available memory 4212379648 bytes, without a limit breach.
Observation begins before import, after initial startup; it does not prove a continuously sampled
system reserve before that observer began.

The retained catalog subsequently contains another non-fixture root, with 48514 published items.
The additional root was manually imported during the interruption, as confirmed on resumption;
it is not an unexplained program registration. The complete original-catalog oracle correctly fails
the exact path-set/count/scan assertions: combined count is 58514. All frozen 10000 source files
still match byte SHA256, size, creation/modification times and physical file identity. The fixture
date counts match, SQLite integrity is `ok` and no foreign-key violations are observed. This is
not an uncontaminated workload pass, source-safety proof for the additional root, or real-library
acceptance. Its private catalog/logs remain ignored and are not relaunched for continuation.

The 2026-09-10 request resumes only unfinished observations in a new isolated catalog, reusing
the frozen sources and unchanged artifacts. The one additional lifetime is limited to 20 minutes
with the original memory/count/dimension limits. Import repetition establishes uncontaminated
derived storage rather than replacing the failed original oracle. The additional root is never
opened by the resumed client. Result analysis and all delegated preparation remain charged to
the existing mixed-media allowance; no C01/C02 or complete-Daily allowance is renewed.

The resumed client runs from 00:51:42 to 01:11:42 UTC on 2026-09-10. A real picker import
again visibly publishes exactly 10000 images. The following original-image observations cover
all twelve dimensions across the two lifetimes; each completed open shows the correct image
identity and textured pixels after loading, followed by Escape back to its folder gallery.

| Dimensions | Folder count | Observed original | Lifetime |
| --- | ---: | --- | --- |
| 640 x 480 | 700 | `03452-640x480-0328.jpg` | Resumed |
| 800 x 1200 | 700 | `03435-800x1200-0326.jpg` | Resumed |
| 1024 x 1024 | 600 | `03436-1024x1024-0326.jpg` | First |
| 3840 x 2160 | 2000 | `06890-3840x2160-0778.jpg` | Resumed |
| 2160 x 3840 | 1500 | `06891-2160x3840-0778.jpg` | Resumed |
| 4000 x 3000 | 1500 | `06892-4000x3000-0778.jpg` | Resumed |
| 7680 x 4320 | 1000 | `06893-7680x4320-0778.jpg` | Resumed |
| 4320 x 7680 | 1000 | `06884-4320x7680-0776.jpg` | Resumed |
| 6000 x 4000 | 500 | `03442-6000x4000-0326.jpg` | Resumed |
| 12000 x 1500 | 150 | `01797-12000x1500-0149.jpg` | Resumed |
| 1500 x 12000 | 150 | `01798-1500x12000-0149.jpg` | Resumed |
| 8000 x 8000 | 200 | `02279-8000x8000-0197.jpg` | Resumed |

In the 8000-square viewer, Right, Right and Left during loading settle on the second item with
the matching original pixels; Escape preserves the gallery viewport. Folder returns display
their warm previews. These are actual Debug client observations with real media adapters, not
Release latency/FPS measurements or a full-library decode. All explicitly verified originals
above are JPEG; PNG original-viewer coverage is not inferred from PNG catalog or gallery entries.

The final sort menu opens, but the attempt to select modification-date ordering reaches the fixed
parent deadline. Input is rejected after the process has retired. The original process receipt
is a timeout failure at 1200139 ms, with the process retired, Job closed and no cleanup failure;
stderr is empty. This is owned deadline retirement, not evidence of an application crash or
normal user-initiated shutdown. No further client lifetime replaces this incomplete observation.

The resumed observer records 4401 samples over 1164014 ms, peak working set 946176000 bytes,
sampled peak private memory 1324380160 bytes, kernel peak commit 1409355776 bytes, and minimum
sampled available system memory 4874543104 bytes. No observed memory limit is breached. The
observer starts about 36 seconds after launch and before import; kernel process peaks cover the
process lifetime, whereas continuous available-memory evidence excludes that startup interval.

After owned retirement, the full independent oracle passes: all 10000 source SHA256 hashes,
sizes, creation/modification times and physical identities match the frozen manifest; the catalog
has exactly the expected path set without duplicates, matching dimensions/bytes/capture and
fallback dates, and all expected day counts. Its one scan is completed with 10000 items and zero
issues; SQLite integrity is `ok` and foreign-key violations are empty. There are 120 ready previews
and 9880 pending previews; the latter count does not establish visible loading failures. This
read-only SQLite check does not replace application FULL reopen after interrupted recovery.

The pass strengthens mixed-size import, preview/viewer and historical-metadata evidence without
closing the complete mixed-media observation. Remaining gaps are alternate-date sorting, exact
search results, a separately repeated warm-page sequence, an intentional live update during
browsing, the unexplained bottom-rail input discrepancy, and normal shutdown for the resumed
lifetime. Frozen sources are unchanged; no live-update stimulus was introduced. Real Release
decoding/input, the other frozen transitions, C01/C02 and external acceptance remain required.
No new product root-cause family is admitted solely from these incomplete or tool observations.

One subsequent check runs the two existing `annotated_time_rail_test.dart` and
`library_time_navigation_test.dart` suites serially through `quality_test_flutter.ps1 -NoPub`.
All 9 rail and 11 navigation tests pass, including bottom-of-unloaded-content addressing,
displaced-year marker mapping, query-wide row alignment and stale-seek suppression. These fixed
fixtures do not reproduce the desktop's exact 201-month distribution and pointer coordinates,
so their passing result does not resolve that unclassified desktop symptom. No test or product
behavior is changed. Raw output is `build/diagnostics/r2c-mixed-timeline-tests.log`.

Conservatively charge the mixed-media allowance's full 90 active discovery minutes, including
fixture/oracle preparation, both lifetimes' active observation, resumption and result analysis,
and 16 delegated preparation/inspection minutes. Quota interruption and unattended process time
do not become active engineering. Discovery reaches 321/330 minutes; triage remains 54/60,
diagnosis/repair 306/390, review 23/120 and recording 42/60 before this checkpoint's final recording.
The summed charge is 746/960 minutes. The first 1425144 ms and resumed 1200139 ms client lifetimes,
build/generation runtimes and oracle/test waits remain separately recorded above or in raw output.
The mixed-media allowance and both admitted client lifetimes are consumed; unused phase time
does not authorize another client attempt or renew any failed gate's replay allowance.

The subsequent existing-test selection and result inspection add five discovery minutes,
bringing discovery to 326/330. Final evidence/document recording adds ten minutes, bringing
recording to 52/60. Before the scoped evidence review, the cumulative charge is 761/960 minutes.
These charges do not create another mixed-media pass. The original budget table later in this
record is historical; this continuation and its revised phase allocation are the current ledger.

The scoped evidence review finds no mismatch among the resumed oracle, resource/process receipts
and the three owning records, and no incomplete observation presented as a pass. Charge one review
minute: review is now 24/120 and the cumulative ledger is 762/960 active minutes. This is a check
of the new evidence record, not another product audit or acceptance waiver. Owned documentation
links and `git diff --check` pass; unchanged product source does not require another heavy gate.

The resumed oracle SHA256 is
`3F38AFE471A5F19796A77FF17E9E231CDD1885B01D97AC14D419AA063FE0071A`;
the memory receipt is `9C36D70485E5BCC9F5DA7B7EF85A6DA90BC0E3DB471B7405C9538202DC51E211`,
and the failed deadline receipt is
`C3EFCF4698E9B2B8905645E5CA7AB62BA1CED438E12F8E4F48CE04665B429060`.

Ignored provenance is `build/diagnostics/r2c-mixed-generation.json`, `r2c-mixed-session.json`,
`r2c-mixed-resume-session.json`, both GUID fixture roots' process/memory receipts and oracle JSON,
and `build/diagnostics/r2c_mixed_fixture.rs`. The oracle additionally rejects duplicate/missing
catalog paths, unhealthy databases and unfinished scans; mere total-count agreement cannot pass.

### Restart audit and remaining-controls proposal

The subsequent goal continuation starts from clean `e04daeb` on `codex/r2c`. The preceding turn
is progress: it adds actual client/source evidence and publishes the reviewed checkpoint. The
current audit compares the original 24-variant roster with those results and verifies the live
Release storage boundary in `rust/src/application/storage.rs`: the test root remains Debug-only.
No new process, experiment, source access or test invocation is started. The existing Release
executable's presence does not prove admissible isolated storage or final-source client acceptance.

At this checkpoint a remaining-controls proposal is prepared without execution. It retains
revision interleavings, pending-call restart, Release and C01/C02 as separate unresolved
obligations. Charge four discovery minutes conservatively, including one delegated coverage-map
minute, and four recording minutes for this audit/proposal: discovery 330/330, triage 54/60,
diagnosis/repair 306/390, review 24/120 and recording 56/60. Total charged time is 770/960 minutes.
The proposed transfer is not applied to these counters until approved. Further discovery cannot
consume an unrelated phase or infer permission from the remaining total. The complete goal stays
unachieved; this decision checkpoint neither resets budgets nor accepts missing evidence.

The subsequent explicit direction prioritizes core image synchronization and bulk additions/
removals. It supersedes the unexecuted controls proposal with the execution plan's core-sync pass:
the 10000-image background stays frozen; single-file changes and a 2000-add/1500-remove burst
occur only in a separately owned generated root. Discovery receives 60 unused repair minutes,
and recording receives five unused review minutes; the cumulative ceiling stays 960. C01's
failed full-load/replay and C02 allowances remain consumed. Current source confirms that the
public 19-case R2c-R runner includes C01's stopped workload, so that complete runner is not used
as a shortcut for this native-client observation. Portable LiveOnly and installed journal
downtime catch-up remain distinct acceptance claims.

### Core image synchronization and bulk changes

The pass reuses the unchanged Debug artifacts and second isolated mixed-media catalog on source
`e04daeb`. The original 10000-file source is frozen. A separate owned root starts with twelve
textured images spanning every dimension and JPEG/PNG; mutations are recorded before execution.
Copies have independent file identities. There are no retained-root reads, service changes,
source-tree caches, manual refreshes or product-code changes in this pass.

The actual client runs from 01:38:12 UTC for 1240375 ms on 2026-09-10, inside the original
30-minute session deadline. Its real picker import publishes the twelve baseline images. The
baseline wait includes the still-open picker before final Import confirmation, so its 28147 ms
does not measure product import latency. Subsequent observations use actual filesystem changes
and a read-only exact-path/catalog oracle, not direct queue insertion.

| Stimulus | Expected root count | Observed outcome |
| --- | ---: | --- |
| One JPEG added | 13 | Exact paths/count publish automatically; visible new thumbnail |
| In-place rewrite | 13 | Source generation advances from 10001 to 10014 with unchanged physical file identity; visible pixels update in place |
| Rename to another directory | 13 | Old path disappears; asset and physical identity remain stable; new path is exposed by the client |
| Delete that added JPEG | 12 | Exact removal/count publish automatically; no refresh |
| Add 2000 files, 1800 JPEG/200 PNG | 2012 | Exact 2012 paths, no duplicates, original root still 10000, completed scan with zero issues and no unresolved queue entries |
| Delete 1500 manifest-listed bulk files | 512 | **Fails:** physical source is 512, but catalog and client remain 2012 after the fixed 300-second check |

The bulk addition writes 2746430688 bytes across all twelve dimensions. Its final oracle settles
25148 ms after the post-copy observation begins; this is a diagnostic observation, not P95 or
event-to-visible latency. The pass includes a completed root-scoped metadata recovery following
live watcher input, so this result is not exclusively per-path P0 processing. A separate successful
background-browsing observation during additions is not established by the attempted input.

During bulk removal, the other root remains visibly browseable at 10000, and an actual
7680 x 4320 original opens correctly and returns to its gallery. Deletion checks every listed
file's path chain, identity and SHA256 before removing it; the mutation completes after 21377 ms.
The one convergence wait ends failed at 300310 ms, without a SQLite observation error. The UI
shows 2012 and an update-blocked notification whose message says automatic directory rechecking
is occurring. All 1500 removed paths remain in the published set; no new inventory consumes them.
The new root has twelve `live_notification` / `reconcile` / `subtree` tasks, each at attempt eight,
with `next_retry_unix_ms=NULL` and `metadata_inventory_required`. The original root remains intact.

This is R2C-C04, a distinct S1 functional blocker. The twelve old catalog subtrees contain 150 or
170 locations each, beyond P0's 128-path bound, while their physical contents have fallen to 25 or
45. `authoritative_library_changes.rs` therefore requests metadata inventory when merging old
catalog locations. The queue promotion owner allows journal-free recovery for a root freshness
gap, but its subtree path retries when an opening journal boundary is unavailable. No new P2
authority or inventory is created. Exhausted rows are also excluded from normal leasing, so merely
changing promotion would not repair already persisted debt. The existing LiveOnly subtree test
explicitly expects this blocked state; a causal correction must replace that expectation and
cover both fresh and exhausted gaps. ADR 0024's declared-scope recovery rule provides the intended
boundary; Supported mode with missing proof must retain its refusal. This mechanism is independently
reviewed, but a corrected regression/client counterfactual has not run. It does not explain C01.

After the failed check, the client closes normally: exit zero, process retired, Job closed,
no cleanup failure. The memory observer records 4512 samples over 1196053 ms, peak working set
1169080320 bytes, sampled peak private memory 1292091392 bytes, kernel peak commit 1337954304
bytes, and minimum sampled system-available memory 4815736832 bytes. No observed memory bound
is breached; the observer's startup gap remains explicit. Final stdout is 96791 bytes and stderr
is empty. The log-cap monitor inspected the stimulus directory while client logs were in the
separate derived-storage directory; final size is bounded, but continuous enforcement is unproved.
Bind that check to actual client logs before another admitted run.

Post-exit source verification passes all 10000 frozen SHA256 hashes, sizes, physical identities,
creation/modification dates and historical day counts. It independently passes the 512 remaining
stimulus files' exact path set, identities, hashes, sizes and mtimes, totaling 704935470 bytes.
Exactly two roots remain. The original root's exact catalog paths, dimensions, bytes, capture/
fallback dates and day counts match; the other root intentionally retains the failed 2012-item
publication. SQLite integrity is `ok` with no foreign-key violations. Source-safety success is
separate from, and does not waive, the failed synchronization result.

The initial combined verification script completes source hashing, then incorrectly compares
catalog integers with manifest decimal strings. That script failure is retained; a corrected
comparison uses the existing oracle's numeric/date normalization and completes both proofs.
It does not weaken the path, dimension, date or source assertions. The old single-root
`verified-oracle.json` is preserved; its exact-one-root rule is not reused for the expected two-root
state. Interrupted/stale UI actions remain unexecuted; notifications observed after intervening
user input are not attributed to an automated notification click.

Offline addition, restart catch-up and explicit Update are unexecuted after C04. The fixture and
failed catalog are retained without reset, manual repair or another run. Portable LiveOnly does
not prove automatic closed-process journal catch-up. C01/C02, Release and other frozen workflow
gaps remain open. The next action is the execution plan's explicit fourth-family repair decision.

Ignored evidence is discoverable through `build/diagnostics/r2c-core-sync-root.txt` and
`r2c-mixed-resume-root.txt`: exclusive stage intent/ledger files, successful and failed observation
JSON, `bulk-remove-blocked-detail.json`, `final-source-integrity.json`,
`final-source-and-old-root-oracle.json`, actual client stdout/stderr and process/memory receipts.
The blocked-detail SHA256 is `4BA3601AD96B795065C2439005AB9A834529DFECE9ECFAECCB243F1AB73EC595`;
the normal-exit receipt is `B3D882B37AFC18C6F8AA1AA3126356E8342224AB0465FD3E1E888B26BAEEC027`.
The memory receipt is `085D616FF57EBE8A1762AEB0D44B48585D398BBDAF01AF00A662D19A715B18DA`.

Conservatively charge the full 60-minute core discovery allowance, including fixture construction,
all delegated preparation/inspection, actual UI observations and source-oracle work. Add one
triage minute, four causal owner-inspection minutes, four independent-review minutes and nine
recording minutes. Discovery is now 390/390, triage 55/60, repair 310/330, review 28/115 and
recording 65/65: **848/960 active minutes before scoped evidence review**. Tool waits and the
1240375-ms client lifetime remain separate. No additional lifetime or failed-gate replay is
created by unused total time; the proposed fourth-family transfer has not been applied.

The one-minute scoped evidence review confirms those distinctions and finds no required document
correction. Review reaches 29/115 and cumulative use reaches **849/960 active minutes**. Owned
documentation links and `git diff --check` pass. Product source and artifacts remain unchanged;
this evidence checkpoint does not require another heavy gate or accept C04.

## C04 repair checkpoint and newly observed browsing failures

The [C04 record](r2c-live-gap-recovery.md) owns the causal correction, extracted ownership,
133 focused tests, complete applicable lint, Debug build, retained-task recovery, fresh +2000/-1500
client replay, memory/process receipts, source oracles and their limitations. The first wrapper's
monitor-exit capture error remains explicit; the second lifetime exits cleanly. No complete Daily,
C01/C02 experiment or third client lifetime was run. C05's blank wall, thumbnail-state and layout
observations remain unresolved and block functional acceptance.

Apply the approved C04 phase transfer, preserving all earlier charges. Conservatively charge the
full 60 repair and 15 discovery allowances, five triage minutes, five recording minutes and 14
review/recheck minutes (13 causal review/checks plus one final evidence reconciliation). Cumulative
usage is discovery **405/405**, triage **60/60**, repair **370/370**, review **43/55**, recording
**70/70**: **948/960 active minutes**. This accounting includes delegated work and failed attempts;
12 unused review minutes do not authorize another diagnostic pass or native lifetime.

The approval turn runs from 02:10:56 UTC through this closeout on 2026-09-10. Final owned tools take
293011 ms for focused checks/lint and 62374 ms for the build; native lifetimes take 116625 and
1527369 ms within the fixed 30-minute session. Intermediate tool waits remain in the retained raw
captures. These elapsed tool times are not new active-engineering allowances. The execution plan's
proposed 120-minute browsing continuation and 1080-minute cumulative ceiling await a scope decision;
they are not already charged or authorized at that checkpoint. C04 evidence does not accept the candidate or all R2c.

The subsequent explicit approval activates the browsing continuation at approximately 03:38 UTC,
starting from `507db08` and the 948-minute checkpoint. Its additional limits and stopping rules
are owned by the execution plan; no prior failed experiment or final gate is reset.

The [C05 diagnosis record](r2c-browsing-diagnosis.md) preserves the two passing controlled navigation
cases, two failing presentation cases, corrected test assumptions and unresolved native blank/gray
cause. The 30-active-minute diagnosis boundary is consumed. No product correction or new client
lifetime executes before the stopping decision.

Charge 30 diagnosis minutes (10 triage and 20 repair, including five delegated inspection minutes),
two evidence-review minutes, one scoped final check minute and the full ten recording minutes.
Cumulative usage is discovery **405/420**, triage **70/70**, repair **390/435**, review **46/75**,
recording **80/80**: **991/1080 active minutes**. C05 consumes 43 of its 120-minute supplement.
The proposed 76-minute native-evidence pivot fits within the remaining supplement and total but
requires replacement diagnostic admission and the explicit phase transfer in the execution plan.
The unused original review balance does not authorize another experiment or lifetime.

The renewed instruction admits that pivot. Its [C05 record](r2c-browsing-diagnosis.md#renewed-native-observation-and-preview-demand-correction)
separates the blocked native session from the subsequently proven application demand defect.
The fixed 30-minute session expires without a successful input action; the user confirms no manual
window operation. Its parent deadline and clean retirement are retained, not attributed to Ame
sync/preview failure. No bulk stimulus, repaired replay or new session executes.

Conservatively charge the full 35-minute renewed diagnosis boundary (15 discovery and 20 repair,
including five delegated fixture-preparation minutes and the failed input attempts), 23
repair/verification minutes, four independent inspection/review minutes and six recording minutes.
Tool runtimes and the gap awaiting input are separate from active work. Cumulative charged usage
is discovery **420/420**, triage **70/70**, repair **433/435**, review **50/69**, recording
**86/86**: **1059/1080 active minutes**. The pivot consumes 68 of its 76-minute ceiling; the
original C05 diagnosis remains charged, for 111 of the supplement's 120 minutes. Residual time
does not extend the consumed native deadline, exhausted diagnosis or failed final-gate allowances.
The coordinator correction has 100 passing focused tests, full lint and an independent code
review; native incident causality and final-source acceptance remain open.

The final C05 evidence recheck adds 2.3 independent minutes and 4.7 conservative primary inspection
minutes: 1066 total, with C05 at 118/120 and its pivot at 75/76. The subsequent final hosted-source
readout uses three minutes of the original remaining verification-review balance, not another
C05 diagnostic or review invocation. Cumulative usage is discovery **420/420**, triage **70/70**,
repair **433/435**, review **60/69**, recording **86/86**: **1069/1080 active minutes**.
Pure waiting on the same live hosted run is recorded separately. No unused total or phase balance
renews an exhausted diagnosis, native session, independent-review invocation or Daily attempt.

Run `34443329392` on product `8cf120f` is terminal: nine verification jobs pass, Accessibility
fails C02's unchanged parent deadline, and the aggregate gate fails. Its
[source checkpoint](r2c-browsing-diagnosis.md#final-hosted-source-checkpoint) records 570 Flutter
tests, the 1503-pass/19-ignored Rust main suite, all five executed synthetic cases, the exact
failure and retained raw logs. No unchanged rerun follows and no native browsing exit is inferred.

## Approved P2 follow-up

The follow-up starts from clean `856d230` on `codex/r2c` on 2026-09-09 at 15:11 UTC.
The approved 90 active diagnosis/repair minutes and 25 review minutes remain inside the original
phase ceilings; one additional complete Daily is conditional on causal correction, focused
verification and lint. Prior failures and invocation counts remain charged. Capture verification
uses one bounded delegate; P2 SQL and application ownership remain with the primary executor.
No new C02, retained-library or unchanged full-workload replay is admitted.

The single differential experiment compares the unchanged exact-evidence SQL with an owner-first
join on the same fixed relational projection and pinned SQLite engine. It records query plans,
VM instruction counts and all ten result fields at 0, 1, 4095, 8960 and 10000 completed owners,
retaining 10000 candidate owners and 2048 completed P1 rows. The only alternative is forcing the
owned-candidate input before queue lookup. Equal evidence and a causal work reduction are required;
elapsed time alone or merely passing the original workload is not admission evidence. This query
projection does not establish a valid publishable catalog or replace application verification.

### P2 follow-up result

Raw output capture passes before the SQL experiment. The existing native accessibility Job and
cleanup owners supervise a fixed diagnostic PowerShell child through the repository's existing
`cmd` redirection pattern. Each stream has a monitored 64 MiB limit and a caller-supplied parent
deadline. The compiler-free fixture writes all byte values plus distinct tails: stdout is
6291487 bytes and stderr is 6291473 bytes; both SHA-256 comparisons match exactly. Exit 37 remains
the primary failure after an injected later cleanup failure; process exit and Job closure are
confirmed. The first capture attempt exits 1 with empty streams because its pre-held writable
output handles conflict with redirection; that failed attempt is retained. The corrected capture
holds read handles with read/write sharing. These ignored diagnostic scripts do not modify or
replace a public quality entrypoint.

The sole SQL differential completes one test, zero failures/ignored cases and 1516 filtered cases
in 0.50 seconds; compilation is 29.45 seconds and the owned command takes 31758 ms. Raw output,
exit zero, process exit and Job closure are retained. On this projection both queries already
search candidate owners by run and then look up queue rows by integer primary key. The exact
ten-field result and VM instruction count agree at every selected completion state:

| Completed owners | Original VM instructions | Owner-first VM instructions |
| --- | --- | --- |
| 0 | 100092 | 100092 |
| 1 | 100093 | 100093 |
| 4095 | 104187 | 104187 |
| 8960 | 109052 | 109052 |
| 10000 | 110092 | 110092 |

This is a negative result for the chosen join-order hypothesis on the projection, not a recovery
pass or proof of every production-catalog query plan. The projection omits catalog-wide
constraints, row payloads and concurrent workers; it cannot apportion the retained 66329 ms,
certify full publication or prove complete-state semantic equivalence. Application polling
remains separately measured at 113671 ms in that failed tail. Neither total proves the cause of
missing completion by the creation-to-FULL-reopen deadline. No product fix, count suppression,
polling change or extra experiment follows. The temporary SQL constant and probe module are
removed from the Rust tree after retaining their exact source. Consequently no new lint, focused
full workload or Daily is warranted for this documentation-only final diff; the conditional third
Daily is unstarted, not passed. C01/C02, the 24-variant gaps and external/client exits remain.

Ignored evidence is retained under `build/diagnostics`: capture verification
`r2c_p2_raw_capture_verification_2.json`, SQL source `r2c-p2-query-experiment`, and complete command
output `r2c-p2-raw-08f21cc15a154be48f90fd8ecee95a7e`. The source SHA-256 values are
`4D6A055A74808F4ECD68EAA29B1AD562FD6CE8B79B9B2C290D0E4EC6A0BEC1D7` (completion owner) and
`DA2FC85ED9D7E3973E6F9796AC2EE6ED049D8CBB9A889ECEE1F5F4F7667AA26E` (probe). Output SHA-256 values
are `A8D89C266A442254C6AB3D8A3E6DF3C2971E196DCF7C4A980E3A333A174DD1E1` (stdout) and
`1069FC269AEC65C4FED5361899EEE8C868ADF2848D901D7ECC4874231E79EDEC` (stderr). Raw paths and
machine identities remain untracked. No real root or original media is accessed by this follow-up.

Diagnosis is conservatively charged at 26 active minutes (17 primary and 9 capture delegate),
within the approved 60-minute diagnostic pass and 90-minute follow-up allowance. Cumulative
diagnosis/repair is 306/480 minutes; no repair allowance is consumed. The sole differential
experiment allowance is consumed. Stopping follows the missing causal correction and experiment
limit, not exhaustion of the active-minute ceiling. Independent review takes two active minutes
(23/120 cumulative), confirms the limited negative conclusion and that conditional verification
is not triggered. Final recording is conservatively charged at six further minutes (42/60
cumulative). Discovery and triage remain 176/54 minutes; total active charge is 601/960 minutes.
The capture child attempts take 123 ms and 1285 ms; the SQL parent takes 31758 ms including
compilation. These bounded runtimes are distinct from active work. The follow-up checkpoint is
recorded at 15:34 UTC, 23 minutes after its start; the original cycle began at 05:07 UTC.
No time or invocation counter resets.

## Parallel supplement checkpoint

The admitted supplement starts from clean `098a19f` on `codex/r2c` on 2026-09-09 at 12:40 UTC.
It admits concurrent C01 and C02/C03 diagnosis and causal repair, keeping local heavy tools serial.
It preserves all earlier failed gates and charged budgets. The plan owns the additional limits.

C01's first experiment decomposes fresh identity into handle open, normalized path, full file ID,
and handle close without changing production behavior or the original two-arm workload. The
hypotheses are a native namespace-query stall, fresh-handle open/close cost, or interference outside
those operations. Stage durations distinguish these possibilities; an isolated pass alone does not
resolve the hosted failure. The first instrumentation build fails Rust borrow checking before any
fixture executes; that log is retained. The corrected build detaches the optional identity result
from its path result before returning either error.

The corrected instrumentation builds in 59.68 seconds and its unchanged two-arm fixture passes
one test in 347.76 seconds (1515 library cases filtered, no ignored selected case). PerPoll records
1759 opens/closes and P95 516 ms; PerEpoch records one open, the required final retirement, and
P95 261 ms. Both retain 25 P0 samples, all 2048 P1 completions and the original first-page control
boundary. This is not the separate full-recovery gate. Nine identity observations exceed 25 ms:
the maximum individual open/path/ID/close observations are 64105/354/30/26929 microseconds. No
eight-second local stall is reproduced. Evidence: `r2c-c01-native-stages-before-2.log`.
The same test-only instrumentation is dispatched as `84f2f21` in hosted run `34353739432` to resolve
the original environment's missing operation attribution. Product behavior remains unchanged.

That Static/Rust job ends failed at 13:35:28 UTC: 1496 pass, one fails and 19 remain ignored in
1764.17 seconds. The PerPoll control fails at sample 24 after 5215 ms total polling; one journal-mode
proof takes 3420 ms and a recovery worker reports 5868 ms in the same proof. Sample 24 has no
fresh-identity observation above the 25 ms instrumentation threshold. Earlier PerPoll samples do
show 537/687 ms identity calls dominated by handle closure. The separate full production case passes
with 25 P0 samples, P95 197 ms, maximum 775 ms, all P1/P2 results and its required final reopen.
Across later cases, identity-handle closure repeatedly costs 1–4 seconds, with one 8154323 us close
while its open/path/ID cost 37/103/7 us. That largest close belongs to the separate 4096-file case,
not the failed control. The result establishes a native retirement stall and a separate SQL-proof
stall; it does not identify one common OS actor or justify weakening either proof. Complete log:
`r2c-hosted-34353739432-static-rust.log`, SHA-256
`47F2512A53270871AB092C22D4EA32DF5858EA0C89F576FBB122D54BDBDF2E37`.

That run also reproduces C02's existing native deadline failure in job `102473233415`: the first
native phase succeeds in 909 ms; `application-ready` exceeds its eight-second parent deadline.
Retained post-cleanup evidence is complete on attempt one with 126 elements and 7650 ms child time;
the native process exits and its Job closes without cleanup failure. This is not a File.Replace
failure. The log is `r2c-hosted-34353739432-accessibility.log`. A bounded timing owner now records
the fixed ten internal stages, cumulative evidence-publication cost and parent elapsed time on
failure. The existing record format, atomic file replacement, acceptance assertions and deadlines
are unchanged. In-memory repeated-stage conservation, malformed timing and backwards-clock checks
pass; native operation attribution still requires the single admitted stage-timed gate execution.

The stage-timed local native gate passes on the unchanged product and instrumented probe: Debug
build 51.5 seconds, both Flutter cases and all ten ordered native phases pass, with process exit,
Job closure and scratch removal confirmed. `application-ready` takes 2271 ms, including 1529 ms in
element finding and 281 ms accumulated evidence publication. The nine other phases take 564–1860 ms.
The final record excludes its own publication and process exit; successful parent enforcement is
separate evidence. Output: `r2c-uia-native-stage-timing.log` and
`r2c-uia-native-stage-timing-output.log`. No eight-second or File.Replace failure is reproduced
locally; neither historical failure is closed by this pass. Full probe guardrails also pass,
including the owned child timeout that retains its original failure and parent elapsed time.

C03's controlled directory experiment proves that legacy deletion can return success while a
metadata-only observer keeps the name visible; it disappears after that observer closes. The
original three-junction control without an extra observer passes, so the earlier failing observer
remains unidentified. The regression fails before correction with `ordinary deletion returned
before namespace retirement`. The native owner now retires the proved name when its deletion
handle closes, using the disposition contract in ADR 0024. Existing data-sharing refusal (32),
nonempty refusal (145), identity and reparse guards remain enforced, with sentinel bytes unchanged.

The extracted cleanup owner retires known resources independently and retains the original error,
cleanup errors, handle-retirement evidence and at most 64 immediately observed child identities.
Unknown children are not traversed; disappearance or attribute failure is observation evidence.
The public facade preserves its four parameters. The exact audited closure now contains two
common-owner sources and four runner sources at depth two; every held identity and close is checked.
The sole new cmdlet admission binds the pure JSON serializer to `Microsoft.PowerShell.Utility`,
with a wrong-module rejection fixture. No deletion or command-resolution guard is bypassed.

Focused native lifetime, original-error, truncated inventory, disappearing-entry and current-source
audit checks pass. The complete R2c-R guardrail passes 19 cases in 117970 ms, preserving replacement
races, junction rejection, held script identity and bounded audit checks. Evidence:
`r2c-c03-fixture-lifetime-guardrail-6.log` and its `-6-outcome.json`. Earlier integration refusals are
retained: stale native digest, obsolete exact source count, non-GUID test leaf, unresolved extracted
public helper and missing serializer admission. Each subsequent invocation changes that diagnosed
integration defect; none is an unchanged retry. This is focused verification, pending independent
review and the remaining complete Daily invocation.

Affected tool-owner size is common 3137 lines and cleanup 195 lines, with zero inline tests;
dedicated guardrail and lifetime suites are 2107 and 232 lines. The existing native primitive stays
in the common facade; no unrelated ownership is added there. The root timing owners are identity
99 lines plus 35 test-only diagnostic lines and 55 dedicated test lines; UIA evidence/probe/process
owners are 256/555/378 lines, with 57 timing-owner lines and 37/183 dedicated timing/evidence tests.
Larger physical decomposition retains the roadmap's existing debt boundary.

C02's file experiment does not reproduce error 1175. A mapping without delete sharing produces 32;
an explicit delete-shared mapping allows File.Replace. MoveFileEx fails with native error 5 while
metadata or mapped holders remain and is rejected as a replacement. No evidence-publication
protocol change follows. The directory finding does not establish shared causality with either
File.Replace or the native UIA timeout.

Supplement accounting at this checkpoint: the file-lifecycle lane conservatively charges 61 active
minutes, including six minutes of subsequent read-only C01 source analysis and its tool/wait time.
The primary lane charges 70 active minutes; actual heavy-tool runtime is additionally retained in
the cited logs. These supplement charges do not reset earlier phase usage. C01 experiment two
separates the unchanged query's prepare, execute-and-reset and statement retirement. SQLite marks
the journal-mode pragma as requiring schema preparation, so the previous outer timing cannot
attribute the stall to reading the mode alone. Production behavior, both workload arms and all
deadlines stay unchanged; no VFS replacement, busy-handler change or schema-proof weakening follows.

The complete applicable lint passes the C03 correction and UIA instrumentation, including the
native guardrails, format, warnings-denied Clippy and Dart analysis. Log:
`r2c-supplement-applicable-lint.log`. Subsequent test-only SQL substage instrumentation passes format
and all 16 retained-proof owner tests (1500 filtered) in 16.45 seconds after an 89-second build.
The unchanged two-arm control passes in 282.68 seconds: PerPoll records 1735 opens/closes and P95
257 ms; PerEpoch records 6422 polls, one open, no premature close and P95 175 ms. Both retain their
25 samples and all P1 completions; this first-page control does not replace full P2 recovery.
No measured journal-proof substage reaches 100 ms locally. All-target/all-feature Clippy with
warnings denied passes in 27.83 seconds. Original PowerShell UTF-16 logs remain untouched; explicit
UTF-8 readout copies are `r2c-c01-sql-stages-owner-tests.utf8.log` and
`r2c-c01-sql-stages-control.utf8.log`. Hosted operation attribution remains pending.

The independent supplement review consumes eight active minutes and finds one S2 correction,
with no new S0/S1: the new regressions' own sequential finally cleanup could mask their first
assertion error or skip remaining resources. All four affected fixture paths now use the same
independent cleanup owner. It also preserves the original PowerShell error text and script stack
alongside the original exception. The focused real-native lifetime and current-source audit checks
pass after correction in `r2c-c03-review-focused.log`. The complete guardrail script then passes all
19 cases in 246893 ms (`r2c-c03-review-full-guardrail.log`). Its outer ad hoc command exits one after
the completed transcript because it supplies the wrong named parameter to the tool-lock release;
that wrapper error is retained separately and is not a guardrail failure or a successful whole
command. A corrected lock-only check confirms acquisition and release after that process exits;
the guardrail is not replayed. The one scoped recheck closes S2 with no new S0/S1 in two active
minutes. Supplement review/recheck totals ten minutes; with the old 11 minutes the cumulative
review charge is 21 minutes. Both supplement review invocations are now consumed. Final-source
Daily, hosted C01 causality and the unexplained UIA/1175 boundary remain open.

The reviewed native repair is committed as `932ee17`; test-only SQL stage attribution is a separate
rollback boundary at `6903d79`. Evidence head `d9ac57a` is verified equal to remote `codex/r2c`.
Run `34363028969` carries that frozen source. Its push supersedes the earlier timing run after
the required UIA result was retained; the cancelled Static/Rust job is not a passing result.
The complete cancelled-job log proves its Rust command had finished before cancellation: 1497
library tests and three binary integration tests pass, with 19 library cases ignored. Its PerPoll
and PerEpoch controls both finish, at P95 591/674 ms respectively; the separate complete production
fixture records P95 561 ms. This valid test evidence is distinct from the cancelled whole-job status
and cannot erase the earlier failures or establish their cause. Log:
`r2c-hosted-34358832952-static-cancelled.log`, SHA-256
`A0B0D4A23F7212B8AD30F196C57B57F7CC4A783E035C86D012C4232B9DB08E4A`.
The second and final allowed complete local Daily starts on this corrected source. Admission is
the proven C03 namespace-retirement repair and completed independent recheck, not an unchanged
replay or closure of C01. This can verify the accumulated local candidate while C01's hosted
operation attribution remains open; it cannot advance the queue or waive either earlier failure.
Output and outcome are `r2c-d9ac57a-final-daily-2.log` and its `-2-outcome.json`.

Hosted UIA job `102490296029` on `d3f46af` completes successfully at 13:50:20 UTC in run `34358832952`.
Both Flutter cases, all ten ordered native phases, process exit, Job closure and scratch removal pass.
The first application phase takes 2788 ms: 1094 ms loading UIA Client, 549 ms locating the window,
857 ms finding elements and 68 ms accumulated evidence publication. Later application phases take
225–976 ms. This execution does not reproduce the earlier 7650 ms child / eight-second parent
failure or File.Replace 1175; no causal UIA repair is claimed. Complete log:
`r2c-hosted-34358832952-accessibility.log`, SHA-256
`F851A9F503E6A508C9C6583D1D5FF1217B67FEA2896757C199CDF407B5523C1C`.

### Final supplement result

All ten applicable hosted jobs pass on `d9ac57a` in
[run 34363028969](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34363028969).
Static/Rust records 1497 passing library tests, 19 ignored, and three passing binary integration
tests. The two lifetime arms record P95 478/461 ms; the separate complete production recovery
records P95 413 ms, maximum 415 ms, and passes its unchanged full completion/reopen assertions.
The SQL substage probe records no operation at or above its 100 ms reporting threshold in that
job; it does not explain the earlier SQL-proof stall. The complete Static/Rust log is
`r2c-hosted-34363028969-static-rust.log`, SHA-256
`377247402B3E629E729EC74FA900EEC13A8A34E655E447A802A9691B1C2E82F9`.

Hosted Flutter executes 71 file processes with 565 passing tests. All five synthetic jobs each
execute their one required, nonignored test; unsigned Windows builds and catalog-free Release
bridge smoke pass. Native accessibility completes all phases and confirms process exit, Job
closure and scratch removal. Controlled scan completes successfully and deliberately retains its
fixture evidence without an owned cleanup proof. Complete logs, markers and hashes are retained
under `r2c-hosted-34363028969-*`, including `-completed-readout.json`. These are controlled gates,
not real-library, signed installed-service or real Cloud Files acceptance.

The second local complete Daily runs from 14:22:36.987 to 14:54:34.661 UTC (1917695 ms) and fails.
Its complete lint, including the repaired 19-case R2c-R guardrail, format, Clippy and Dart analysis,
passes. Rust records 1496 passing tests, one failure and 19 ignored in 1646.12 seconds. Cargo's
failure stops the Daily before its Flutter, native scan/accessibility and final bridge phases;
passing hosted partitions do not substitute for that missing complete local result.

Both local lifetime controls pass at P95 248/172 ms. The failed separate full recovery has P0 P95
163 ms, maximum 169 ms and all 2048 P1 items completed. At the unchanged 300-second
creation-to-reopen bound it has staged 10000 entries, produced 9216 candidates/owners and completed
8960 owners; 64 are leased and 193 P2 rows remain pending, including the recovery control row.
The run remains comparing, control pending, authority unretired and root/checkpoint recovery-required.
It has neither final publication nor FULL reopen. This is a reproduced C01 completion failure,
not the earlier single-operation visibility stall and not a C03 cleanup failure.

The failed run records 12738 polls costing 175.028 seconds in total. Nested counters include
45.487 seconds in queue metrics, 39.345 in root availability and 25.265 in catalog revalidation;
these overlap parent totals and cannot be added together. Maximum poll cost is 114.169 ms and
maximum revalidation cost is 68.149 ms, so this trace does not reproduce the prior multi-second
identity operation. Cumulative polling/worker-progress attribution remains a next-scope hypothesis,
not a proven cause or permission to weaken polling/workload assertions.

PowerShell's transcript does not capture the entire native-output prefix. Subsequent output is
preserved in `r2c-d9ac57a-final-daily-2-native-continuation.log`; its final large failure report also
contains an explicit tool truncation. The decisive completion assertion, exact state/queue counts,
aggregate observation totals and final Rust failure summary are retained. Neither file is described
as a lossless complete native transcript. Transcript SHA-256 is
`7149D52792A4EA6F19CF8A771ABCE77E5ABE61FB8C9CC919F7BAAE7A34C3734C`; continuation SHA-256 is
`A82BEE6A750DC0191E9A6B64309D3195D2F57778DCAC8D994B0D2C4E4A595472`.
The structured outcome separately preserves command failure and start/end times. No replay is used
to replace the failure or repair the output gap.

The primary supplement allowance is conservatively charged at its 90-minute ceiling; the file
lane retains its 61-minute charge. Cumulative diagnosis/repair charge is 280 minutes of eight hours,
review/recheck is 21 minutes, and closeout recording is 26 minutes of one hour. Original discovery
and triage remain 176/54 minutes. Both complete Daily invocations have failed, the unchanged replay
allowance remains exhausted, and both supplement review invocations are complete. The bounded
supplement therefore stops at this unresolved checkpoint. C03's owner repair is reviewed and
current-source verified at its guardrail/lint boundary; C01, unexplained C02 and overall candidate
readiness remain open. Original media and real-library state were not mutation inputs.

Post-checkpoint preparation reads the already retained failure output without executing another
test. Its tail-only summary is 196185 ms, 6375 polls, 113671 ms in application polling and 66329 ms
in separate exact-evidence SQL reads. The current completion owner performs that SQL after each
poll and sleeps two milliseconds. This refines the next diagnostic hypothesis without establishing
causality or reopening execution. The execution plan contains a separate, unadmitted proposal.
Preparation and closeout are now conservatively charged at 36 of the original 60 active minutes;
diagnosis, review and consumed run counts are unchanged.

## Baseline

- Started: 2026-09-09 05:07 UTC. Branch: `codex/r2c`.
- Source: `228403f1af9c7568c52aaa3f602cc28f21f230d3`; initial working tree clean.
- Product source is unchanged from `49f648aca482f53e0358acea13bf3636096a985d`.
  The intervening commits change only repository instructions and documentation.
- Windows x64 build 26340; Windows PowerShell 5.1.26100.9233; Rust/Cargo 1.97.1;
  Flutter 3.44.9 (`6b182d2c7585eba26d4edce0f97630effd256c33`), Dart 3.12.2,
  engine `5a2a6a42cce67f965cf540fcecf616faca624aa1`.
- Existing unsigned Release executable SHA-256:
  `F7C233C29A24C74A55EF9C9DB62D2D4005100BFB3A0FC7478BDFC7D686B28372`.
  Packaged Rust DLL SHA-256:
  `240CB46DA81C072023EDC3F1956856EDE7ED56F622BC3591D391B982D0A7E922`.
  Their retained unsigned evidence identifies dirty source at `74688ec`; these hashes identify
  the starting artifacts, not a fresh build from the baseline commit.
- No application, Cargo compiler, or Flutter tester was active at the initial process check.
  Four pre-existing Dart process records remain untouched. Heavy commands run serially under the
  repository tooling lock, with one Cargo compiler job to avoid the recorded commit-memory failure.
- Controlled source changes use generated fixtures and separate derived storage only. Real roots,
  retained catalogs, installed services, signing, and cloud hydration are not discovery inputs.
  Private logs and paths remain outside tracked evidence.

## Starting evidence and unresolved obligations

The [browsing-recovery record](synchronization-browsing-recovery.md) remains the exact starting
checkpoint. Its complete first Daily failed; later focused and partition results cannot replace it.
The original mixed-load run had P0 P95 288 ms and complete P1, but only 2,944 completed P2 owners
at its 300-second creation-to-reopen deadline. The unchanged isolated pass took 274.12 seconds.
Both results remain evidence. No additional unchanged replay has been consumed by this cycle.

The mixed-load workload remains 25 P0 samples with P95 at most one second, 2,048 P1 candidates,
10,000 P2 source entries, a 4,095-entry logical page, complete publication and authority retirement,
and FULL reopen inside the existing 300-second deadline. Earlier per-stage deadlines remain intact.
Historical hosted latency attribution and the recorded hosted `application-ready` timeout remain
open in the interleaving ledger; later local success cannot erase either failure.

Real EXE exit/relaunch, actual Release decoding/input, and simultaneous multi-root update/removal
require explicit coverage mapping. Widget remounts, static preview accessibility fixtures, and
catalog-free bridge smoke do not establish those results. Missing evidence is not a diagnosed bug.

## Frozen discovery roster

Frozen on 2026-09-09 at the baseline source above. Each row names one normal, interleaving, or
failure/restart variant; supporting boundary tests do not multiply the roster. `Mapped` means the
test and owner were inspected, not that the complete client path passed. Required missing paths
remain open until recorded evidence closes them. Existing tests may be reused only after matching
their actual passing output and unchanged source; the final complete Daily remains separate.

| Variant | Fixed action and expected result | Owning boundary and evidence path |
| --- | --- | --- |
| UX-01A | Warm existing catalog starts with usable content; no-change continuity enumerates/opens no source and creates no inventory | [Controlled lifecycle and 100 no-change starts pass](#startup-query-folder-and-recovery-evidence); [native Debug populated restart](#native-debug-populated-restart) now verifies the real 10000-image gallery across two executable lifetimes. Exact zero-source-call proof remains instrumented evidence; Release and normal-profile acceptance remain open |
| UX-01B | Close while start/poll is pending, then launch the same fixture catalog; old PID exits within the existing bound and old epoch cannot publish | Window shutdown and synchronization epoch; `window_manager_actions_test.dart`, `library_synchronization_test.dart`; [native close and same-catalog relaunch verified](r2c-process-lifecycle.md); actual late-after-stop completion remains covered by controlled races |
| UX-01C | Launch with interrupted first import; display paused checkpoint, require explicit Continue, preserve truthful portable capability | [Real paused restoration and explicit continuation verified](r2c-process-lifecycle.md#recovered-real-input-pair); [combined native paused feedback, portable notice and actual Continue](r2c-process-lifecycle.md#combined-paused-restoration-and-portable-capability) verifies exact 10012-location publication, source preservation and normal exit. Its aggregate input-parser failure remains explicit beside separate offline action verification; Release and candidate gates remain open |
| UX-02A | Import through the actual picker with Chinese paths, wrong-extension PNG and damaged input; exact count/issues and unchanged source | [Complete generated Debug lifetime passes](r2c-input-controls.md#complete-native-input-and-rendered-feedback): exact three images, damaged-path/code oracle, source integrity and normal exit |
| UX-02B | Pause then cancel before native Started/registration; cancel wins, late Started does not regress feedback, execution retires | [Actual native controls, exact replay ordering and rendered feedback pass](r2c-input-controls.md#complete-native-input-and-rendered-feedback); the disclosed 120-second registration barrier remains a controlled race rather than a production latency result |
| UX-02C | Cancel replacement before commit or fail display after commit; preserve previous baseline or retry display exactly once without rescan | [Native focused Enter retries committed display exactly once without a rescan](r2c-input-controls.md#complete-native-input-and-rendered-feedback); pre-commit replacement remains separately covered by Rust `accepted_cancel_before_projection_commit_preserves_the_published_baseline` and primary-workflow tests |
| UX-03A | Switch root/search/sort and cross a page while publication proceeds; latest query owns coherent page/count/timeline | [Controlled query/page ownership cases pass](#startup-query-folder-and-recovery-evidence); [selected native Debug combined interaction passes](r2c-query-interactions.md#complete-selected-native-lifetime); [continuous native search after C08 correction passes](r2c-query-interactions.md#native-continuous-search-result); final-source and Release gates remain open |
| UX-03B | Expand a folder, change revision, load another folder page; replace obsolete window and append only at the same revision | [Original folder-controller cases pass](#startup-query-folder-and-recovery-evidence); [native replacement/append](r2c-query-interactions.md#native-folder-window-across-publication) and [C09 native position continuity](r2c-query-interactions.md#native-correction-interactions-and-retained-lifetime-gap) have functional evidence. The combined parent timing failure, final-source gates and Release input remain open |
| UX-03C | Fail display after committed removal/update, replace query and explicitly retry; retain page, settle loading and never repeat source action | [Controlled query/removal transitions pass](#controlled-query-source-and-viewer-evidence); [native committed-removal focused Enter Retry and subsequent browsing pass](r2c-query-interactions.md#native-committed-removal-display-retry); update/query supersession remains controlled and Release stays open |
| UX-04A | Materialize cold then warm actual media previews; verify pixels, source version, cache ownership and unchanged source | Preview store/media adapters and existing seven-format acceptance; [selected cold/warm Release pixels, source integrity and normal lifetime verified](r2c-release-native.md); [selected native Debug source-bound cache reuse passes at calibrated timestamp precision](#native-coldwarm-preview-ownership); Release ownership and final-source acceptance remain open |
| UX-04B | Rewrite fixture with identical size/ID/mtime while the old request is pending; reconcile one path and show new pixels, reject old publication | [Post-decode same-metadata replacement passes](#controlled-query-source-and-viewer-evidence); [native watcher publication, stale Dart result retirement and current decoded color pass](r2c-browsing-diagnosis.md#native-replacement-and-exclusive-source-recovery); Release remains open |
| UX-04C | Hold source exclusively or present corrupt bytes, then recover; preserve precise failure, valid source retries and no stale ready publication | [Controlled exclusive-open, corruption and newer-source recovery pass](#controlled-query-source-and-viewer-evidence); [native exclusive-open recovery](r2c-browsing-diagnosis.md#native-replacement-and-exclusive-source-recovery) and [C10 native compact feedback](r2c-query-interactions.md#native-correction-interactions-and-retained-lifetime-gap) have functional evidence. The combined parent timing failure, final-source gates and Release remain open |
| UX-05A | Open original, navigate both ways and return; actual decode, correct anchor and released source slots | Viewer/source reader and controlled position cases; [Release decode, Left/Right, observed return anchor and normal lifetime verified](r2c-release-native.md); [selected native Debug decode, return anchor and source-slot retirement pass](#native-original-viewer-source-slot-evidence); final-source and Release slot boundaries remain open |
| UX-05B | Close/reopen while paging or buffer copy is pending; old completion/errors remain retired and new navigation works | [Connected viewer and source-lifetime cases pass](#controlled-query-source-and-viewer-evidence); [native pending-page close/reopen](r2c-query-interactions.md#native-viewer-replacement-during-pending-paging) and [pending source-copy replacement](r2c-query-interactions.md#native-viewer-replacement-during-pending-source-copy) verify actual input, current original pixels, old-work retirement and normal exit. The copy hold precedes the actual engine file copy; Release and final-source gates remain open |
| UX-05C | Same-path source rewrite, then authoritative rename/removal; newest pixels and stable asset until authoritative removal | [Selected native Debug source identity and natural viewer return pass](r2c-query-interactions.md#settled-source-native-result) after actual source reads retire. [The C12 observer correction and complete repeat](r2c-query-interactions.md#c12-raster-observer-and-complete-source-identity-result) preserve current pixels, source/catalog checks and normal exit with empty stderr. Earlier failures remain explicit; final-source and Release acceptance remain open |
| UX-06A | A/B update while queued C is cancelled; independent progress and only C is cancelled | [Native queued cancellation and real updates pass](#native-queued-cancellation-and-real-updates), with exact 10000/512 completions, unchanged C, real catalog refresh and independent execution release; the command-admission interval is controlled |
| UX-06B | Remove C while A publishes and B continues, then register C while old cleanup remains; no repeated unregister, stale root or cleanup authority | [Native removal/publication and reimport pass](#native-removal-during-publication-and-reimport); [controlled old/new spool authority](#multi-root-current-evidence) remains separate from physical file reclamation and Release |
| UX-06C | Make fixture A unavailable then restore it while B updates; preserve A's catalog and B's progress | [Native directory loss and automatic restoration pass](#native-directory-loss-and-automatic-restoration); the [controlled peer-change/recovery case](#multi-root-current-evidence) remains separate from physical devices, Journal, Cloud Files and Release |
| UX-07A | Original 25-sample P0/P1/P2 workload through complete P2 publication, authority retirement, synchronized state and FULL reopen | `production/tests/priority.rs` and `priority/recovery_completion.rs`; [current complete Daily passes the unchanged full workload](r2c-browsing-admission.md#corrected-complete-local-quality-checkpoint). The original 300-second S1 failure, unresolved attribution and final candidate duty remain explicit; this later pass does not explain the earlier failure |
| UX-07B | Same workload with per-poll versus per-epoch connection lifetime; preserve proof and production P95 bound | `priority/connection_lifetime_control.rs`; [connection-lifetime control and full workload pass in complete Daily](r2c-release-native.md#bounded-live-worker-continuation), with [current-source local gates](r2c-browsing-admission.md#corrected-complete-local-quality-checkpoint). The [recorded hosted P0 timeout](r2c-interleaving-remediation.md#hosted-mixed-load-recurrence-on-9359618) retains its unresolved attribution; later passing controls do not close that obligation |
| UX-07C | Interrupt/reopen exact leases and exhaust recovery retry; retain durable failure/lineage and keep other roots eligible | [Controlled runtime restart, exhausted-candidate barrier and peer eligibility pass](#startup-query-folder-and-recovery-evidence); [actual raw-batch process loss](r2c-process-lifecycle.md#native-restart-after-an-incomplete-raw-inventory-batch) and [executing-lease crash with natural expiry](r2c-process-lifecycle.md#native-restart-with-an-unexpired-executing-lease) preserve lineage, complete membership and normal resumed exit. [Native exhaustion/restart with peer publication](r2c-process-lifecycle.md#native-exhausted-recovery-and-an-eligible-peer) retains eight real attempts, exact exhausted lineage, cached/peer pixels and normal exits. [C13 corrected feedback](r2c-process-lifecycle.md#correction-and-retained-state-verification) passes focused and selected native checks while preserving the failed task; final-source, Release and the full variant remain open |
| UX-08A | Jump by scrollbar/time rail then reverse before completion; immediate visible demand and no old seek rollback | Controlled time-navigation cases; [Release rail clicks load without a wheel trigger and completed warm reversal is observed](r2c-release-native.md); [native pending reversal](r2c-query-interactions.md#native-reversal-while-an-older-result-is-pending) proves obsolete-result retirement, exact new-target reads and current pixels. [C11's corrected loading composition](r2c-query-interactions.md#c11-stable-loading-region-verification) passes the unchanged native Debug stability assertion with current pixels and normal exit. [Current Release navigation and settled point observations](r2c-release-native.md#current-release-gallery-stability-and-incomplete-menu-traversal) retain source/catalog integrity and a normal lifetime; they do not replace the Debug frame oracle. Both preceding stability failures remain; final-source and remaining Release evidence stay open |
| UX-08B | Original ten-phase populated whole-window UIA sequence and native process exit; no invalid AXTree | `integration_test/windows_accessibility_bridge_test.dart` and existing public runner; [current complete Daily passes both cases and all ten phases with normal process/Job exit](r2c-browsing-admission.md#corrected-complete-local-quality-checkpoint). The [C09/C10 checkpoint](r2c-query-interactions.md#c09-retained-folder-windows-during-revision-replacement), earlier local pass and hosted timeout remain retained; final candidate acceptance remains open |
| UX-08C | Keyboard menus and task Retry/Cancel; correct focus return, immediate feedback and one committed action | [Actual menu Escape and viewer keyboard verified](r2c-process-lifecycle.md#recovered-real-input-pair); [focused keyboard Retry and pointer Pause-Cancel pass](r2c-input-controls.md#complete-native-input-and-rendered-feedback); [queued-task keyboard Cancel passes](#native-queued-cancellation-and-real-updates); [earlier Release pointer-open/Return does not reopen](r2c-release-native.md#complete-native-lifetime-with-normal-host-close); [selected Debug menu returns have a failed parent lifetime](r2c-input-controls.md#exclusive-menu-focus-observation); [current Release keyboard sort Enter/Escape/Enter and normal lifetime pass](r2c-release-native.md#native-keyboard-sort-return-on-current-release). The [bounded-digest native run](r2c-release-native.md#bounded-digest-native-import-and-menu-return) also completes layout/more Enter/Escape/Enter, exact membership and normal lifetime on current Release source. Earlier [dedicated traversal](r2c-release-native.md#dedicated-menu-control-and-traversal-coverage-gap), [photo-route import admission](r2c-release-native.md#photo-route-run-stops-before-import) and [settled-observation admission](r2c-release-native.md#settled-observation-native-result) failures remain preserved; other final Release duties remain separate |

Flutter filenames above are under their existing `test/app` or `test/features/library` owners;
Rust test owners are under `rust/src`. This is one fixed cross-layer discovery pass, not a full
repository audit. No UI redesign, source operation feature, dependency or schema change is admitted.

### Accumulated candidate hosted checkpoint

Hosted [35727071315](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35727071315), on
`9c584a05e5469cd6888ab54dbe7d8b1384efe3c0`, completes successfully with all ten required jobs and
the aggregate Windows gate passing. Three signing-only jobs are skipped. The current source
includes the primary committed-refresh admission correction and both strengthened preview suites.
Static/Rust records **1519 passing library tests, 19 ignored cases and three passing binary
integration tests**; the library suite takes **1601.72 seconds**. The connection-lifetime control,
complete P0/P1/P2 recovery and post-decode same-metadata preview regression all pass in this run.
The other jobs cover Flutter, native scan/accessibility, unsigned Release and five synthetic loads.

The full run receipt and Rust log remain under `.build/r2c-controlled-evidence/` as
`ci-35727071315-complete.json` and `ci-35727071315-static-rust.log`. Their SHA-256 values are
`6270D041DA0F219CE24A05CF39631D22B108324CD1C7E223EFCF4AB5C7BCA2F3` and
`820CAA7DE51B6B1992E4B061D4B5EC3123DC656A252DD29C7DD85C831F1C4327` respectively.
This establishes this candidate's hosted gate, not a causal explanation for the preceding C01
failure or C02's local evidence-publication failure. No unchanged local gate is replayed.
Actual remaining input/Release paths, complete local Daily, accumulated independent review and
signed/installed-service/journal/cloud/retained-library acceptance remain separate obligations.

Hosted [35734783744](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35734783744), on
`aac68edd022e248e1e929cf45af484d3952d817c`, subsequently fails Static/Rust and the aggregate gate;
nine required jobs pass and three conditional release jobs skip. The library suite records
1518 passed, one failed and 19 ignored in 1335.13 seconds. The failing connection-lifetime control
reports a P0 location absent after five seconds, a leased queue entry, 215.1184 ms queue admission,
214.1665 ms worker admission, and 61 polls totaling 5.4634599 seconds with a 5.2622367-second maximum.
Live and journal lanes are active; recovery is inactive at this failure. This recurrence preserves
C01's unresolved causal obligation and supersedes any description of the latest completed hosted
run as green. It does not by itself identify the slow operation's cause. No unchanged rerun follows.

The retained run receipt and failed log are `ci-35734783744-complete.json` and
`ci-35734783744-failed.log` under the same ignored evidence directory. Their SHA-256 values are
`A82F14ADC52D163E3FEC8409B95AF2A41347A236CD3A35EC091A056B0BB048C4` and
`5B1E57B669181FE9710AE27541D30AAD2714BDBF6B89D0DE92DE527196A4955B` respectively.

### Startup, query, folder and recovery evidence

The 2026-09-22 documentation check matches exact assertions and retained output from production
source `935961800893862a2a2d0cf50ce6f31673ce3f12`. Through `9c584a0`, only the preview-source and
stationary-gallery test files change under `lib`, `rust/src` and `test`; these owners/tests remain
unchanged. No new workload is run for this check.

- **UX-01A:** the lifecycle test shows a `cached library` text fixture while native start is held,
  closes immediately and rejects its late completion. This is a responsiveness boundary, not a
  populated gallery. The separate production test performs 100 real observer startups against a
  controlled no-change journal session. It checks zero media-content opens, source enumeration,
  spool/inventory creation or added scan rows, with one watcher handle per start and 200–300
  metadata-only availability probes. Observer handle creation is not zero filesystem activity.
- **UX-01C:** all six Rust continuity values map independently of healthy live freshness. The
  connected AmeApp test displays the live-only notice with its explicit closed-process limitation,
  deduplicates four subsequent snapshots and yields to an actual blocked-root notification.
  These controlled snapshots are separate from the verified native paused-import relaunch.
- **UX-03A:** sort, root and search queries retire a held visible-range result; late completion
  cannot restore its page or loading state. Separate cases retain the old page until anchored
  publication, reject stale query/pagination results, and read one coherent page/timeline snapshot.
  This covers ownership transitions, not a single native interaction combining every stimulus.
- **UX-03B:** eight folder cases cover coherent newer windows, complete replacement on revision
  change, cursor reset after invalidation, late-response rejection, bounded same-revision append,
  cross-revision append rejection, cache invalidation and retained folders on page failure.
- **UX-07C:** runtime restart consumes a deliberately persisted expired Live lease and retains
  its P2 consumer lineage without an automatic full scan. The exhausted-candidate test leaves
  recovery Comparing, its control leased and authority unretired. Separate retry tests retain
  failure/degraded state and exclude exhausted scheduling. The peer test leases ready-root debt
  once while blocked-root attempts remain zero, then restores eligibility after an injected
  unblock. SQL-seeded expiry and injected root status are explicit controls, not an EXE crash or
  one combined exhaustion/restart/peer lifetime.

The retained Flutter log and hash are identified in the following controlled-evidence section.
Exact passing output occurs at lines 435 (startup), 536–542/773–779 (query), 596–604 (folder),
827 (continuity mapping) and 1184 (capability notification). The retained Static/Rust log records
no-change startup at 1652, restart at 2219, exhaustion at 829/843/844 and peer eligibility at 2223.
Its unrelated C01 failure still prevents an overall job pass. Rechecking the existing hashes and
unchanged owning source validates reuse; it does not renew native or final-gate evidence.
Charge the 20-minute documentation reservation in full: 3514 conservatively reserved minutes,
not measured elapsed work. Native input/Release, C01/C02, final review and external exits remain.

### Native warm-start recorder admission

The current native UX-01A preparation finds the existing source-access counters compiled only for
Rust tests. A separate installed Windows file-I/O recorder calibration is attempted before any
client launch. `logman` returns `0x80070005` while enabling the kernel-file provider, with an
administrator-required message. The exact calibration session nevertheless exists with one empty
buffer; a subsequent successful stop and not-found query prove its retirement. No positive file
read or directory-enumeration control runs, so the 64-KiB trace supplies no zero-access evidence.
The failure and separate retirement receipt remain in `.build/r2c-warm-start/`. No system privilege
is changed, no real root is touched, and the native startup counter obligation remains open.

### Populated Release startup preparation

The admitted UX-01A method now has a prepared two-lifetime fixture on documentation head
`bdf9fb7`. It imports the unchanged 10000-image, 10921494393-byte generated mixed-size/historical
corpus into a fresh disposable profile, normally retires the first process and Job, then starts a
distinct process against that same profile. Closed catalog snapshots retain exact root, scan,
asset and source identity checks; separately observed pixels and navigation remain required.
These checks establish preparation only. The later failed admission below supplies no populated
native observation or passing UX-01A verdict.

The isolated payload retains the unsigned evidence from source `94f52f0` plus its verified dirty
source hashes, subsequently committed as `562e53d`. Those twelve source/test hashes still match.
Later runtime-source edits consist only of the reviewed test-measurement wrappers, whose non-test
calls and ordering are retained; the remaining changes are tests and documentation. This justifies
the scoped Release reuse and does not claim a fresh current-head build. The frozen manifest binds
512 current product files, 25 diagnostic helpers, 38 prepared inputs and the original Release/empty
Sandbox admission evidence. Its SHA-256 is
`1A4AEB8FF9FA1346BAFA8B2C5599AC9C1E93F188070C4FBCC32376DD99C4B96D`.

The typed lifecycle owner rejects overlapping starts, reused PIDs and foreign/late epoch receipts.
Every owned resource has independent retirement, including partial-start and cleanup failure paths.
Review identifies and corrects input admission after asynchronous waits, late start receipts and
ambiguous Sandbox close evidence. Atomic start publication now rechecks authority after flushing
and immediately before publication. Actual input rechecks the current owner, screenshot age and
deadline after recording and asynchronous reads. Sandbox request and confirmation use distinct
observed actions; the final verifier requires their complete current-run/window pairing, final
position in the action history and observed target disappearance. No action-name error waiver
can establish retirement.

Focused checks pass: 29 lifecycle, ten host-completion, 13 JavaScript input/session and seven Python
evidence tests, including asynchronous rejection and actual session composition with fake input
ports. PowerShell parsing passes. These 59 checks do not constitute native input evidence.
The unchanged host source verifier checks all 10516 retained/generated test files in 61.022 seconds.
No original-media root is read or modified.
Its final receipt is `source-integrity-1790282905687320900.json`, SHA-256
`ADE50C6B7D5D4076223A6BF9091BC08A5D1F65C976BC2B27484C40E504B482BD`.
The diagnostic owners contain 86 lifecycle, 104 process, 49 guest composition, 196 native session,
134 host supervision and 151 offline-verification lines, with no inline tests. Six dedicated test
files contain 318 lines. These are ignored acceptance helpers; product source is unchanged.

The startup source map connects `main.dart` through `mountAmeApplicationAndRevealWindow`,
`LibrarySynchronizationLifecycleOwner`, `RustLibrarySynchronization` and the existing Rust
production continuity owner. The retained complete Daily log reports the two mount/reveal and
three lifecycle tests passing, and the exact 100-start production-observer test passing. Its
controlled journal session proves zero enumeration/media-content opens/spool/inventory/new scan
rows while retaining watcher handles and bounded metadata probes. The Flutter lifecycle still
uses a cached-text stand-in. Neither that test nor future catalog equality supplies native I/O
measurement; the earlier failed ETW calibration and separately required native/external counter
evidence remain unchanged.

The configuration is in `.build/r2c-populated-startup/`, run
`9f9559e2e8f14f76b4c8e5473c0a1386`. Its initial payload/source/helper admission passed, but the
resource guard measured 6.79 GiB available against the original 7 GiB floor. Resetting only the
owned UI-control session yielded 6.86 GiB; the later check was 6.84 GiB. Those checks produced no
host-start receipt and launched no Sandbox or Ame process. The configuration remained unconsumed
until the following execution; the earlier resource failure is retained.

### Populated Release startup admission failure

On documentation head `f1f27a7`, the same frozen 512 product, 25 helper and 38 input bindings pass.
Fresh host verification passes all 10516 generated/retained fixture files in 67.095 seconds.
Run `9f9559e2e8f14f76b4c8e5473c0a1386` starts Windows Sandbox at
2026-09-25 02:59:25.8035140 UTC with 9172525056 available host bytes; its recorded minimum is
8981147648 bytes. Memory admission and the two-GiB reserve both pass.

Computer Use lists the matching Sandbox window, but window access returns
`Computer Use app approval timed out`. No screenshot or native input is obtained. The only
guest-output-directory file is the host's matching `abort.json`; no source-copy, admission,
application-start/exit, catalog, memory or guest-result receipt exists. Missing receipts do not
prove that no guest bootstrap or source-copy work executed, and the tool timeout alone does not
establish why guest readiness is absent.

The supervisor reaches its unchanged 900-second deadline and preserves
`Media lifetime exceeded its 900-second parent deadline`. Its terminal record is
`processBoundaryPassed=false`, elapsed 930992 ms including cleanup waiting, with Sandbox PID 14968
still present. At 03:41:27.3598979 UTC, a new process query finds no Sandbox or Ame process; the
current window list likewise has no Sandbox. This later disappearance supplies neither a normal
close receipt nor a passing process-boundary verdict. No application import or restart acceptance
follows from this run, and this failure does not reproduce the earlier finalization observation.

The independent result review confirms those evidence limits. The complete host source postcheck
passes all 10516 files in 51.599 seconds; guest postchecks remain unavailable. Source media mappings
remain read-only, and no real-library root or product source is changed. The original results and
the single consumed execution remain preserved; restoring tool access and proving empty guest
readiness are prerequisites to a different, bounded continuation method.

| Retained evidence | SHA-256 |
| --- | --- |
| `host-start.json` | `70816B4CDCA18890DD8C2355A31D938CDD71E34968F9AC030DAEFBCF67A91D52` |
| `host-result.json` | `F6BF9D1139B591D03DDAFFD91E33FAC7C35DF89D97D84A98BC331353CF610BD0` |
| `output/abort.json` | `8FB0C7C35619194F114952C75EC1D9F855C6F4F718737205CE22A11E82327483` |
| `.build/r2c-populated-startup/failed-startup-observation.json` | `F1E94FD8C5A783B88265A5FB4C3CA24AD8085639BADE0517EEAE321CC98A0F5D` |
| `source-integrity-1790307852410960200.json` | `E4A6ABF1AA529B46CDC3A6546C8BB4788C222DD2883340F09D4714FD108F6F14` |

Host/abort receipts are under `build/integration-storage-9f9559e2e8f14f76b4c8e5473c0a1386`;
the complete source postcheck is under `build/integration-storage-e84f07c4443e4008b0c71381991477a4`.

### Empty Sandbox readiness preparation

Follow-up `d5633c7bd2a84a1d9e268517916a371e` is prepared in `.build/r2c-sandbox-access` with
unchanged copies of the existing canary preparation, guest and host scripts. Their three SHA-256
comparisons and PowerShell parse checks pass. The guest script also matches successful predecessor
`f409552af3f142df8e7c23b97da1817d`. The new WSB has exactly two fresh script/evidence mappings,
read-only input, disabled network/clipboard, three-GiB guest memory and no media or product payload.
Host entry/reserve and the whole canary deadline remain seven/two GiB and 180 seconds.

Independent review confirms new identity, matching bindings, empty output and no reused receipts.
Configuration SHA-256 is `D2579C96DE2F464FDD4C427F7060D0F6BE8E08156EF4172A2A09DAAA888B6026`;
`preparation-result.json` SHA-256 is
`61EEE0DA096378584F29716BB4F9082F9CA3616DD8AAEF69088FBB6862A5F969`.
At preparation, no host-start receipt or native attempt exists. Preparation alone proves neither
guest readiness nor window input, normal retirement, populated startup or UX-01A. The subsequent
execution below supersedes the pending tool-access condition.

### Empty Sandbox initialization failure

On `98d0c3d`, the prepared run `d5633c7bd2a84a1d9e268517916a371e` executes with unchanged helper
and configuration bindings. Entry availability is 9023619072 bytes; the recorded minimum is
8629411840 bytes. No application or media is mapped. Computer Use selects the returned Sandbox
window, captures its actual error dialog and successfully clicks No to decline feedback. A fresh
window listing and process query subsequently find no Sandbox. Window access and native input are
therefore available; the preceding authorization-timeout explanation is not the current blocker.

The screenshot instead reports initialization error `0x80370106`, virtual machine or container
unexpected exit. No guest receipt arrives. The unchanged host supervisor preserves `passed=false`
at 180820 ms with no remaining Sandbox process. Successful error-dialog dismissal does not prove
a successful guest lifetime. No Ame launch, populated restart or image workflow is exercised.
The enabled scoped container logs and Application log have no matching event; those available
records do not establish the inner virtualization failure. No host settings, services or drivers
are changed, and no feedback is transmitted.

The host-start/result SHA-256 values are
`EF66D367108E1BA2002E7990F9AC6F751A2166A6A1C6A0962C68B39DCBA15E63` and
`2AEE121DA1AB54B63372C83D78F473EE5E5CEB8073FE3F058CDA43FBBA2E215E`.
The native observation is retained in `.build/r2c-sandbox-access/native-initialization-failure.json`
with SHA-256 `8B979CF293EEE26483F12665A810005D2D8CC32598A280A0D20D8D24C6CC985F`;
its screenshot provenance is the task's actual Computer Use output, not a separately saved image.
The [single software-rendering comparison](../plans/r2c-closeout.md#empty-sandbox-software-rendering-comparison)
keeps this failed result and changes only the next disposable guest's virtual-GPU setting.

### Software-rendering readiness comparison

Run `b512cbd1ccbb415983c54103b215e7c3` uses the identical three-helper closure and normalized
configuration, changing only fresh identity/paths and `vGPU=Disable`. Independent prelaunch review
confirms that boundary, empty output, two exact script/evidence mappings and unchanged resources,
deadline and retirement requirements. Its configuration SHA-256 is
`3A5666AD57FBF4BB9D1E76D16474E22759CA409936AD10DE3C9418797E0ED0B5`.

Native capture again shows `0x80370106`; observed No input dismisses it without sending feedback.
The host preserves `passed=false`, no guest receipt and no surviving Sandbox process at 181363 ms.
Entry/minimum host availability is 8892743680/8463683584 bytes. Disabling virtual GPU did not restore
guest readiness; this neither proves nor rules out a particular driver defect. No Ame, image or
ordinary-profile state participates in either empty-guest attempt.

Read-only package inspection reports Windows Sandbox `0.8.107.0` as `PackageOffline, DataOffline,
NotAvailable`, with no install location; the Calculator control reports `Ok`. The feature is
enabled, while neither checked servicing nor update reboot flag is present. A specific-volume
package query fails with `0x80070490`, so no particular drive is attributed. These are environment
observations, not a proven initialization cause or authority to repair host components.

Host-start/result SHA-256 values are
`6BB2A5AC744690C0138BB451CBF2D1DD32247C77DE3CBC4F3CA05E06AC52CCB1` and
`8758373D0F580C18D973EF011E2A7708B7D1AC551761324CFCBFC73801FE8297`.
`.build/r2c-sandbox-software-rendering/native-initialization-failure.json` has SHA-256
`8E616B09AF093A1A1C7147B7A0653496918B3D9E06D57B6585CD0A14EE7584C6`.
The independent Debug populated-restart method proceeds without reclassifying either failed
Sandbox run or closing the separate Release requirement.

### Native Debug populated restart

The host-isolated method calls production `main` through a Debug-only admission entrypoint, using
a fresh GUID-owned derived store and in-memory presentation preferences. It admits only the frozen
10000-image, 10921494393-byte generated corpus, with twelve dimensions and 201 historical months.
No source-copy/change helper, artificial scan hold, seeded catalog or normal-profile setting is
used. Source revision is `98d0c3da52300e49dfa2ba0b559ad3cbd356b007`; product source is unchanged.

The preceding run `2e0db80d02f0477dbc83efbe3a4eeaed` retains two failed supervisor results. Its first
native import completes in 96.723 seconds and exits normally in 659.4084 ms. The closed-catalog
verifier then fails because SQLite URI interpretation was not enabled; no second client starts.
After the read-only URI correction, exact membership, preview ownership and catalog identity pass.
A continuation within the original absolute deadline fails before production bootstrap: Python
records catalog mtime `1790316778123`, while Dart's Windows stat exposes `1790316778000`.
The byte count and exact catalog SHA-256 remain unchanged. No second entry receipt exists, so
this is an admission-helper failure, not an observed product startup failure. The original and
continuation verdicts remain failed; complete source postverification passes for all 10000 files.
Native observation also required raw JPEG support and one shared capture/evidence module instance;
those adaptations and their changed helper bindings remain with the original run.

The final corrected invocation is `057d635fd3fb450cb3063e97b8418337`. Its entry compares timestamps
at the precision both runtimes expose, while host admission still requires exact closed-catalog
hash equality. Before launch, the actual retained Python admission passes the corrected Dart guard
without consumption; 13 guard checks and three real read-only SQLite checks pass. The retained
29 lifetime/retirement, nine capture, nine stable-identity and five bounded-verifier checks cover
the reused owners. Independent prelaunch review confirms the narrow changes. Owned Dart formatting,
warnings-denied analysis, PowerShell parsing and the serial Windows Debug build pass.

The complete two-process sequence passes in 237746 ms, within the unchanged 900-second bound:

| Observation | Result |
| --- | --- |
| Native picker/import | Exactly 10000 images; observed completion in 69.897 seconds, below 300 seconds |
| First gallery | Visible thumbnails finish without scrolling or Retry; normal close takes 564.1716 ms |
| Second startup | A distinct process opens the same catalog with 10000 images and decoded content before native input |
| Root and viewer | Native root selection, decoded 7680 x 4320 original, and Escape return preserve the observed top date, tile arrangement and rail position |
| Second normal close | 446.4991 ms; both application/parent pairs exit zero, with closed Jobs/handles and no cleanup failure |
| Closed membership | Both checks find exactly the frozen 10000 members, unchanged root/scan/asset/location/source identities and no unfinished queue work |
| Preview ownership | Both checks verify 36 ready artifacts totaling 735012 bytes; this does not claim all 10000 previews were requested |
| Resources | Peak working set 560070656 bytes; peak commit 874074112 bytes; minimum sampled host availability 5985288192 bytes; no two-GiB limit breach |
| Source safety | Full before/after SHA-256, file identity, size and creation/modification-time checks pass for every file in 48.231/47.732 seconds |

The first and second PIDs are 12608 and 27728, with distinct admission epochs. Stable closed-catalog
identity projection is `d11a1d81a020566e9b1ad08b7964e999aa2c3544b371bd9f2bec6fcaebcb5d05` in both
lifetimes. The database's whole-file hash changes after ordinary second-lifetime activity; the
identity projection, rather than an assertion of byte-identical databases, proves retained members.
No continuously captured animation or subsecond timestamp continuity is claimed. The snapshots
prove the observed settled gallery return; instrumented no-source-call and frame-level lifecycle
regressions retain their independent duties. This Debug result does not close Release, normal
Known Folders, deletion/date, performance, signed-service or real-source acceptance.

Evidence is retained under `build/integration-storage-057d635fd3fb450cb3063e97b8418337`, including
native screenshot provenance, both admissions/entries, lifecycle receipts, catalog checks and source
checks. `started.json` binds all helpers, the native payload and Dart kernel. Its SHA-256 is
`933910D9AB2C9A0EC7D1FAFBCEE842431800F7B654CDE07FD4A35302143F0BD5`; sequence-result and source-after
SHA-256 values are `D676E25E4D1C17B04446F9512FEFC289C2241C6E7310073BA36BEBE02347190B` and
`FD31A5BA3199E83EC2EEC6321623267A48639C04769C446AADB0F69EE08A7EBE`.
The earlier failed run remains under its own GUID; no receipt is overwritten or reclassified.
Independent result review verifies all 30 artifact/helper bindings, saved screenshot hashes,
PID/epoch/event ordering, both closed catalogs and the complete source postcheck. It finds no result
blocker; that structured-evidence review does not independently repeat the native pixel observation.

### Hosted checkpoint before readiness continuation

[Run 36091829697](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/36091829697) completes
successfully for `109acff`: all ten required jobs and the aggregate Windows gate pass; three
signing-only jobs are skipped. The tested PR merge commit is `203b678`, whose complete tree
`2bd2dbe6e5beeef46db52ea15d194391a70560f3` exactly matches `109acff`. Readiness commit `c1077ee`
changes only this record and the execution plan, so it changes no tested product or verification
behavior.

The retained logs report 1562 passing Rust library cases, 19 unchanged ignored cases, three passing
broker cases and 96 passing Flutter test files. Both the complete mixed-load recovery and its
connection-lifetime control pass. The native accessibility record contains all ten successful
phases, two passing cases, normal primary exit and Job closure; controlled Windows scan also passes.
The unsigned artifact separately proves three runner cases, two real engine-retirement cases with
zero exit/closed Job, and the catalog-free Release bridge check. Its manifest reports a clean
tested tree. These raw records are under `.build/r2c-sandbox-access/ci-109acff-jobs` and
`ci-109acff-artifacts`, with the full hosted verdict in `ci-109acff-final.json`.

All five Release synthetic workloads execute their exact selected test with zero ignored cases
and passing resource records. The 10000-file workload records cold/warm/resume times of
14650/11942/14859 ms; it does not replace or explain the original workstation Debug failures.
This checkpoint also does not explain historical C01/C02 failures, supply the missing populated
restart or deletion/date observations, or close signed-service, real-library, Journal or Cloud
Files acceptance. At that hosted checkpoint, the empty-Sandbox prerequisite is still unexecuted;
its later failed native result is recorded above.

### Native original-viewer source-slot evidence

On product source `a1c165b2cb11c380ef4a8ee3f2f47c0af139b2b6` (documentation HEAD
`d4ef3bae476bfbe666d6da801f815a1159d82d39`), run `4918ba993fad4e818c33df99dae125c6`
retains an incomplete first attempt in `.build/r2c-viewer-slots/` and generated fixture
`integration-storage-e502ea404f104e58829bac48df9eda27`. Actual pointer opening displays the
selected red 4096 by 1024 original at position 2/2 with its current source generation and exact
RGBA. After decode, two production source leases are available, a third returns
`viewer_source_busy`, both close, and a renewed lease succeeds and closes.

The diagnostic incorrectly selects by filename while expecting Right to reach the other item.
The real query order places that selected image last, so no Right input is sent and no navigation
or return pass is claimed. Native Close exits the application with code zero: the same-host
input-to-exit upper bound is 436.5093 ms and application close-dispatch-to-exit is 326.0163 ms.
The 307091-ms parent correctly fails for missing `viewer-step-1`. The post-close screenshot
refresh reports the absent window after successful Close delivery. Owned process, monitor and Job
retirement have no cleanup failure and stderr is empty. The two new sources and all 10516 frozen
background files pass their complete postchecks; catalog counts are `2,2,2,512,10000`.
This is a fixture-selection failure, not a reproduced gallery-sort or source-slot defect.

The corrected, independently reviewed method selects the actual first and second members of the
complete root query. Five focused cases cover both filename orders plus truncated/paged, foreign
and duplicate identities. Fifteen reused observer/input/retirement source files match the first
attempt byte for byte, retaining its 20 passing guardrail cases. The 12 Dart sources analyze
without findings and the diagnostic Debug build completes in 24.5 seconds. No product code changes.

Run `a77461e6f62c4d1c84ffe472f783b177`, with separate fixture
`integration-storage-7472112b066f4070864fe8172e974459`, completes the native path:

- the real root query records green `remaining.png` before red `selected.png`;
- actual pointer opening and paired Right/Left/Escape key events produce green 1024 by 4096,
  red 4096 by 1024, then the original green image, each with current source identity and exact RGBA;
- all three decoded-original checkpoints and the returned-gallery checkpoint admit two real Rust
  source leases, reject the third with `viewer_source_busy`, close the leases and admit a renewed
  lease which also closes; probes do not overlap navigation input;
- Escape restores the original tile rectangle `[284,226,48,138]` with current decoded thumbnails;
  the observer records ten seconds of valid gallery stability and rejects gaps above one second;
- the application, parent, monitor and Job retire normally. The same-host native-input-to-exit
  upper bound is 357.7997 ms; application close-dispatch-to-exit is 253.4687 ms. Parent elapsed time
  is 126577 ms. Both application and monitor stderr are empty. Peak sampled private
  memory is 429101056 bytes, kernel peak commitment is 449216512 bytes, and the minimum sampled
  available host memory is 7881736192 bytes. All original bounds hold.
- both new source fingerprints and exact five-root membership pass the postcheck. All 10516
  background source files pass the complete content/identity/date oracle in 47.958 seconds.
  Its receipt `source-integrity-1790194486930575200.json` has SHA-256
  `8BE4A16728FFB456CA1C041551DD894767D81C9B9736D610CE2CA60B84AA5967`.

The close input has a delivered receipt; only its subsequent capture reports the already-closed
window. That capture message does not substitute for the independent zero-exit and retirement
receipts. Current source manifest, admission, result and parent SHA-256 values are respectively
`E2C5F53F9C184B16ED93B4F3D7EF8906B52134B89A1BBD76EC17B981450DA6EA`,
`662374E67E65D35C3D36265A4F298789051C4439484D752DF67361B5F3AF067F`,
`2C51D7044C86275ACEA07CCA2C740A0D3905A425C212F8A19B84D811E91289ED` and
`0F34D12ED04565006F948DFB9D4EAA72A14DF6023ACAB8149E678C71CBFEF996`.
The manifest and admission remain in `.build/r2c-viewer-slots-ordered/`; result, input and process
receipts remain with the fixture. This proves the selected native Debug source-slot boundary,
not global zero file activity, Release slot capacity or final-candidate acceptance.
Independent result review matches the structured result, query order, delivered inputs, retirement,
source postchecks and five listed hashes. Its timing-origin clarification is incorporated above;
it does not independently repeat the visual observations or all source-manifest comparisons.

### Native cold/warm preview ownership

Run `d3d07ef6d95e4523b6a81fbfb326b37b` uses unchanged product source `a1c165b` at documentation
HEAD `0117a70`, diagnostic sources in `.build/r2c-preview-cache-native/`, and generated fixture
`integration-storage-e4b3b7269f2440898e43eb0d364c97c4`. The one-time real scan adds two differently
sized PNGs beside the retained 10516-source background in an isolated derived catalog/cache.
Preparation corrects a Python helper-name collision and the independent review's missing UI-error
latch before launch. Nine focused Dart cases and five initial Python ownership cases pass; all
12 Dart files analyze cleanly and the Debug build takes 22.8 seconds. No product source changes.

Actual pointer selection reaches the two-image root. After its two UI preview calls retire, the
observer binds each current rendered source version and artifact, samples real decoded pixels,
and sends one non-forced production warm request for each actual bucket. The green portrait uses
bucket 256 and 1076 encoded bytes; the red landscape uses bucket 512 and 2421 bytes. Warm results
retain the exact source identities/generations, paths and encoded bytes. Cold and warm RGBA are
respectively `[49,200,97,255]` and `[220,32,47,255]`, within the original JPEG tolerance of three
channel values. Ten seconds of current gallery stability complete with a one-second maximum
observation gap. The warm requests are controlled native probes, not additional user gestures.

Native Close is delivered once. The application, parent, monitor and Job retire normally, with
empty application/monitor stderr. Same-host native-input-to-exit is at most 416.9755 ms;
application close-dispatch-to-exit is 308.7555 ms and parent lifetime is 73133 ms. The subsequent
capture reports the already-closed window. Peak sampled private bytes are 340447232, kernel peak
commitment is 375066624 and minimum sampled available host bytes are 7594160128; resource bounds hold.

The first closed oracle fails because it assumes millisecond timestamp precision from Dart's
API units. The retained native baseline is `1790196936000000` microseconds for both artifacts,
while Python reports fractional NTFS times. Read-only calibration with the installed Dart 3.12.2
reproduces those whole-second values through both `File.stat` and `lastModified`. The corrected
offline oracle requires the native baseline, calibration and raw filesystem timestamp to agree
at that demonstrated precision. Six Python cases now pass, including rejection of a changed
second, a mismatched calibration and an unsupported fractional baseline. Subsecond timestamp
continuity was not measured. The initial failure and reviewed-source manifest remain unchanged;
only the offline oracle and its test change among the manifest's 21 files. No second client run occurs.

The closed, read-only catalog passes `quick_check` and exact active-location/artifact ownership,
source identity/revision/generation, algorithm v3, orientation, bucket, encoded dimensions and byte
size checks. Both cache files still exactly match their cold byte captures. The two new source
fingerprints and five-root membership `2,2,2,512,10000` pass their postcheck. The full 10516-source
background oracle passes in 46.803 seconds. This closes the selected native Debug cache boundary
at the recorded precision; global zero source I/O, Release ownership and final-source duties remain.

| Retained receipt | SHA-256 |
| --- | --- |
| `reviewed-source-manifest.json` | `EA18B2648E50DCBC64E8731C0A106230CB0808A9D09C4BCAB16E0F76395D929D` |
| `native-admission.json` | `27E07E8F02352BBC7C383615CAC111FFF664B7D9DAB3540A92F8EF9A6C907E6D` |
| `result-d3d07ef6d95e4523b6a81fbfb326b37b.json` | `8708621C46A47DC65EB4CEFD4A78A21CA00401B6B4A8207B2EB0657474AFF721` |
| `d3d07ef6d95e4523b6a81fbfb326b37b.process.json` | `D63071038D6F80C4A42357B82CA92FF1C7C75161C93194754E1D92AEB50A662D` |
| `preview-ownership-initial-failure.json` | `CBB6B1B4F308933616063DC12A616914B4543EEF504124436071AA1EF3DC70BD` |
| `file-stat-calibration.jsonl` | `E7F841FD4619DB9A2EE0BE91FA4ACB8FCC06A1497B970B7EF2696A71E4FE15A6` |
| `preview-ownership.json` | `7F4EE99E19E2EF29D95583CDDBE1CE003DABF86FD45D8FD2BC85FF418F0D4AC4` |
| `source-integrity-1790197117689038700.json` | `283C0CC78C43398F483686ECA471C7512D0DD2E089285401E7BE4C7C7A91DB6D` |

The manifest, admission and calibration are in the diagnostic directory; run receipts are in its
fixture. The full background receipt remains under `integration-storage-e84f07c4443e4008b0c71381991477a4`.
Independent result review confirms the source-bound artifacts, pixels, timing endpoints, source
postchecks, hashes and limited timestamp conclusion. It does not establish subsecond continuity,
absence of every possible rewrite, or zero source I/O.

### Controlled query, source and viewer evidence

#### Retained controlled results

This 2026-09-22 check matches actual assertions and exact test output on production source
`935961800893862a2a2d0cf50ce6f31673ce3f12`. Controlled transitions retain their client limitations:

- **UX-03C:** `library_update_refresh_test.dart` preserves user-query success/failure while a
  committed refresh follows the accepted query. Genuine or inconsistent catalog reads retain the
  previous coherent page, settle loading and wait for explicit Retry. The removal case in
  `library_controller_test.dart` retains committed removal feedback after a failed display read,
  then retries only that read: one unregister and one completion sequence remain. The deferred
  refresh failure in `library_query_snapshot_reader_test.dart` retains the published revision-8
  removal page when the revision-9 read fails. These cases cover distinct transitions, not every
  possible ordering of all actions or native keyboard delivery.
- **UX-04B:** the new `same_metadata_rewrite_after_decode_rejects_old_pixels_and_recovers_preview`
  rewrites a generated PNG after decode revalidation with the same identity, byte length and
  modification time. The final guard rejects the old request, leaves its exact catalog/Ready
  ownership and artifact bytes intact, and removes staged work. A subsequent ordinary non-forced
  request enters the real initial-open reconciliation owner. Exactly one path reconciliation
  completes, source revision/generation advances, the obsolete request remains rejected, and the
  new generation produces the expected new pixel color. No private reconciliation call or manual
  database mutation advances that generation. Automatic Flutter demand and watcher delivery are
  not proved by explicitly supplying this subsequent request.
- **UX-04C:** an exclusive Windows source handle produces the precise open error with no catalog
  or queue change; release followed by explicit Retry succeeds. Same-generation corrupt input
  retains its decode failure, retires legacy Ready ownership and cannot silently become usable.
  A valid replacement published during the old corruption result survives that obsolete result;
  its unique Ready owner and new pixels remain current. Original media is not used.
- **UX-05B:** the three connected viewer-navigation cases hold real application paging across
  close/reopen; late completion cannot navigate the new session, and a newly requested boundary
  navigation joins pending paging once. Seven source-read cases cover pending admission/copy,
  release ordering, replacement and precise Retry. Five image-stream cases cover late error/frame,
  disposed codecs and current visible errors. These are controlled Flutter paths with application
  owners, not a native Release input or original-file I/O acceptance run.

Hosted [35721552835](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35721552835) supplies
the unchanged Flutter output at `.build/r2c-controlled-evidence/ci-35721552835-flutter.log`,
SHA-256 `EDCEC9A725E3737F6DA99F500E7D1014F49632454BF9E2979D7CD799C8F2D368`.
The passing job contains query/removal evidence at lines 582, 773–779 and 905–909, source-stream/read
evidence at 1090–1104, and connected viewer paging at 1165–1167. Its Rust log records the existing
preview cases at 2395–2417, but the full Rust job and aggregate fail the independent mixed-load
case recorded in the interleaving ledger; this is not an overall hosted pass.

The new test's first run passes seven cases and fails its own immediate-reconciliation expectation:
late guard rejection does not reconcile in that same call or retain the initial-open error prefix.
Preserve `source-reconciliation-direct.log`; do not weaken production or discard that failure.
The corrected three-stage oracle passes **8/8** in **7.50 seconds** after a **49.94-second** compile.
Its exact output is `.build/r2c-controlled-evidence/source-reconciliation-corrected.log`, SHA-256
`41DB2A09986BDB8729B45E7A6469E0F81148AFF61FE77AF89DF1C0B882986D8B`.
Rust format and all-target/all-feature warnings-denied Clippy pass; Clippy takes 12.58 seconds.
Production and inline-test changes are zero; the dedicated reconciliation suite is 439 lines.
Independent oracle review and final evidence review take four active minutes in total and find no
remaining blocking issue. Conservatively charge the 45-minute block in full: the cumulative
reservation is 3404 minutes, not measured elapsed time. Compilation/test wall time is separate.
Local complete lint/Daily, automatic client recovery, native input and external gates remain open.

### Stationary gallery preview delivery

`library_preview_synchronization_test.dart` adds three connected cases through the actual AmeApp,
query refresh, controller, gallery and preview queue, with only synchronization/catalog/materialize
ports controlled. It supplies a new catalog version after an obsolete preview, and before that
preview ends. The latter first proves the real controller has accepted revision 2 and source
generation 2 while the old materialization is still pending. Both cases automatically request
generation 2 in ordinary mode, decode a generated PNG through the real image widget and verify
its RGBA pixel `[18, 104, 212, 255]`. Exactly two materialization calls and one query read occur;
no scroll, pointer, keyboard Retry or direct demand call drives recovery.

The removal case publishes an empty current catalog before an old Ready result arrives. The tile
stays absent, the controller keeps zero assets and no new materialization starts. Every observed
transition frame asserts no Retry label or Flutter exception. These tests prove delivery after
an injected authoritative synchronization revision; they do not prove the Windows watcher produces
that revision, or add another native/Release large-library acceptance result. The earlier combined
generated-native bulk result retains its own source, pixel, resource and lifetime evidence.

The initial three-case pass is retained as `preview-synchronization.log`. Review found that waiting
only for the catalog read did not independently establish the second ordering. The strengthened
controller-state oracle passes **3/3** in the scoped repository Flutter runner; strict Dart analysis,
owned-file format and whitespace checks also pass. Its final log is
`.build/r2c-controlled-evidence/preview-synchronization-ordered.log`, SHA-256
`C073246655564E834BC392B779CEC58607A3EFE845C634A2465F4775EAE35531`.
The dedicated test file is 383 lines; production/inline changes are zero. Independent review takes
two active minutes and its recheck one minute. Conservatively charge the 60-minute reservation in
full, making 3464 reserved minutes; this is not measured elapsed time and does not waive any gate.

### Multi-root current evidence

The 2026-09-22 checkpoint replaces the roster's stale mapping gaps without changing production
scheduling. The older preparation failures and subsequent results below remain historical evidence.

- **UX-06A:** `library_update_cancellation_test.dart` connects the actual task surface to the real
  update controller and execution coordinator. Tapping queued C's Cancel leaves A/B's scanner
  streams untouched; their distinct progress survives, both complete independently, exactly two
  refresh callbacks occur, and all execution reservations retire. A shared controlled scanner and
  refresh callback spy establish control ownership, not native scanning or actual catalog refresh.
- **UX-06B:** both `independent update refresh waits for root removal` controller variants pass with
  one unregister, no C resurrection, coherent refreshed query state and B still active. The Rust
  `removed_root_reregistration_preserves_new_generation_while_old_spool_drains` case connects real
  A/B scans through deterministic nested callbacks, C removal, FULL reopen, C's later registration,
  old-generation cleanup and preserved new-generation source/lease authority. It checks source
  bytes and modification times. This is not a parallel-thread race, native UI duplicate-removal
  test, automatic-cleanup scheduling proof or database-file space-reclamation measurement.
- **UX-06C:** `production_offline_root_recovery_preserves_catalog_while_peer_changes_publish`
  renames the generated A directory away and restores it. A's completed catalog survives while B
  publishes once during unavailability and again while A's recovery enumeration is held. Releasing
  the gate yields synchronized roots, retired recovery authority and FULL reopen with stable asset
  and location identities. Source bytes and modification times match the expected stimulus. The
  production pipeline uses injected notifications; Windows watcher/device/Release behavior is not
  established by this fixture.

The exact Rust cases pass in hosted run
[35701651170](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35701651170) on
`16c215e4d05b8f5ba53ae82fab8cd6db1f45c76c`. Their retained Static/Rust output appears at lines 2721
and 2591 respectively in `.build/r2c-process-lifecycle-ready/ci-35701651170-static-rust.log`
(SHA-256 `6EF3FC3CEE6BFD4D1209CEB0149DCD0E5488C28DE91378C84A99BC64E0A7C4C8`).
The Rust tree is unchanged through `c62c3bb`, so this exact-case output remains applicable.

Focused Flutter verification passes **22 cases**: 16 update-controller cases, the connected
cancellation case and five existing task-surface cases. Initial preparation incorrectly expected
queued C to hold no execution reservation; the actual contract reserves it before scheduling and
releases it on cancellation. The corrected assertion checks both sides of that transition; the
failed preparation output remains in
`.build/r2c-input-controls/multi-root-reservation-expectation-failure.log`. Passing outputs are
`multi-root-focused.log` (its 16-case controller partition) and `multi-root-presentation-after.log`
under the same ignored directory. The latter contains the corrected 1+5-case result.

The shared scanner body is unchanged apart from its public class name. Dedicated test/support sizes
are 885 lines for the controller suite, 163 for the connected presentation suite and 117 for the
scanner helper; production and inline-test changes are zero. Full Dart analysis, repository format
check (230 files, no changes) and whitespace checks pass. One independent review took about three
active minutes and found no blocking issue.

Hosted [35714183280](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/35714183280) on
`c62c3bb1e579824db804b74c4c8cd95cc3845940` passes all ten required jobs and their aggregate;
three signing-only jobs are skipped. The completed raw receipt is
`.build/r2c-input-controls/ci-35714183280-complete.json`, SHA-256
`8D050E4E28BCBA66F6B2B485CB58DF62F2CB0BCA63FC6FBA9E4F0DF3C32DD1C6`.
Local full lint/Daily remains blocked by C02; focused, reused and hosted results do not close
other frozen client variants or final acceptance.

### Native queued cancellation and real updates

The 2026-09-23 continuation verifies selected Debug UX-06A and the queued-task keyboard Cancel
portion of UX-08C on product source through `7144a1954cd6c6ce18c022df956cee87b8f3bb55` and
documentation head `4d04ae3cb118225f28c8af4b7d0ebe5e4e81a1dd`. A fresh derived catalog retains
the verified generated roots with 10000, 512 and two images. All 10514 source files remain immutable.
The actual update dialog selects all three in their existing configured order. Two scanner-port
commands occupy the production controller's two execution slots, with their real Rust invocation
held until native keyboard cancellation of the queued third root. No fake progress, completion,
controller action, catalog row or preview state drives the result.

Method review identifies and closes an oracle gap before the sole native lifetime: cancelling C
must preserve both A/B reservations before the command gate opens, and each active peer must retain
its reservation through completion. Checking only final empty slots could have accepted premature
release of all reservations. The diagnostic permanently records unexpected third-root scanning,
active cancellation, resume, stale keyboard attribution and failed frame invariants. Three focused
tests cover accepted Enter, wrong/stale attribution and pointer revocation. Seven diagnostic Dart
files pass formatting and fatal-info analysis. An initial missing-brace lint and a read-only inline
Python quoting failure are preparation failures, both corrected before native execution. Builds
take 30.0 and 22.6 seconds; the second includes the reviewed reservation assertions. Diagnostic
owners contain 164 startup, 222 observation, 136 scanner, 110 run, 65 native-input and 29 admission
lines, plus 50 dedicated test lines; production and inline-test changes are zero.

Native lifetime `b39d28a64f1a40128f6cb67440590d6f` records:

- The selected two-image gallery has current decoded pixels before updates. Both commands are held
  at 80043 ms, and the queued C task is rendered at 80101 ms. Three observed Shift-Tab actions reach
  C's Cancel; a harmless F6 observation identifies that exact root/control before activation.
- Enter reaches the focused C Cancel at 148999 ms. Keyboard attribution is admitted at 149060 ms,
  independent cancellation is verified at 149063 ms, and the gate opens at 149068 ms. C's reserved
  execution retires while A/B retain theirs. No third scanner call or active cancel occurs.
- Actual A/B Rust calls begin at 149071/149077 ms and emit Started at 149102/149107 ms. Both emit real
  progress. B completes with 512 images and zero issues at 153954 ms, then its controller completion
  and released reservation are observed at 154049 ms. A completes with 10000 and zero issues at
  185237 ms, followed by its controller completion and released reservation at 185337 ms.
- Ready is traced at 185340 ms, 36272 ms after gate release: all execution slots and task loading
  retire, the primary operation becomes eligible, C stays cancelled with two catalog items and
  current pixels, and the three roots total 10514. B's last periodic Progress reports 500 images /
  514 entries; its distinct Completed/controller result is 512 images / 526 entries. A's final
  controller result reports 10000 images / 10012 entries. Progress is not used as a completion count.
- The final observer records 14547 frames and 14541 current-pixel frames, with no latched failure.
  An observed close exits normally in 347.8156 ms. The 218696 ms parent exits zero; the process,
  Job and monitor retire with no cleanup failure. Across 817 memory samples, peak working set is
  453619712 bytes, sampled private memory 389156864, kernel peak commitment 405131264 and minimum
  system availability 5321584640 bytes.
- Full before/after source verification passes in 65.255/57.428 seconds. Every generated source
  retains its expected identity, content and timestamps; the new catalog's exact per-root relative
  paths remain 10000/512/2, and SQLite quick-check passes.

Ignored `.build/r2c-update-native/` retains the method, input boundary tests and immutable artifact
admission. `build/integration-storage-51f8dee23c7449b990d2f1f1455b9fd0/` retains source receipts,
milestones, native trace, final result and owned process/resource evidence. SHA-256 identities are:

- Executable: `2A24C91530C05D88B29ACC4E6FCF73199C237DEB04159B232A9FD7E43BD98DCE`.
- Rust DLL: `77B193EA7CA1B5CEEC033038E3806490890E5DBB3E818AAD72B062D0A46F50C2`.
- Debug kernel: `FBB6B93F4FCD89BE336BB1D88DAEEF60732F8A7E277C59CC110688174E9C1959`.
- Native stdout: `E5318C8C469EE658863A18F59E11B554D2CC616AF39D416FCBFDD96BB6394351`.

This proves queued ownership and native activation followed by real scan/catalog completion.
The intentionally held command interval is not production scan latency or a backend concurrency
benchmark. UX-06B/C, complete focus return, C09/C10, Release, C01/C02 and final/external acceptance
remain separate. No product change or new full Daily is implied by this diagnostic-only slice.
Independent read-only method/result review confirms the revised reservation checks, native input,
real completions, source/member integrity, process/resource receipts and scoped document mappings
without a remaining blocker. Charge the 100-minute reservation in full through 4444 minutes.

### Native removal during publication and reimport

The 2026-09-23 continuation adds native UI overlap to UX-06B on the same product source and
documentation head `14a0f1f7ad336f449590e1a3be50689fd6a18654`. The actual two-root update dialog,
third-root removal confirmation and subsequent reimport picker drive production controllers.
Two real Rust scan calls wait at the scanner port until native removal enters the catalog port.
The actual unregister then waits for the 512-image publisher to reach display refresh while the
10000-image peer still advances. This controlled ordering uses real events and catalog operations;
no source mutation, fake progress/completion or direct controller input joins it.

Before execution, independent method review closes a read-error oracle gap: catalog first-page,
query-snapshot and timeline exceptions are permanently recorded and rethrown, and the frame
observer rejects task/query/page display errors. A subsequent Retry cannot erase a failed normal
refresh. Final reimport also requires release of the primary scan slot. Three focused positive/
negative cases cover valid overlap, missing reservations, an already completed peer, zero progress,
unrelated removal and a queued publisher. Seven Dart files pass format and fatal-info analysis;
preparation/final Debug builds take 29.3/23.6 seconds. Diagnostic owners contain 164 startup,
282 observation, 171 scanner, 96 catalog, 112 run and 21 boundary lines, plus 50 dedicated test
lines. Production and inline-test changes are zero.

Lifetime `f7f64408dff24cc38c6177949e2b5269` records the connected sequence:

- The 10000-image retained gallery has current pixels before the two updates. Actual C removal
  enters at 129370 ms and releases both real scans at 129372 ms. The 512-image scan completes with
  zero issues at 132872 ms. At 132926 ms its controller is refreshing while C removal is pending,
  both peers retain their execution reservations, and the other real scan has accepted 1012 images
  from 1024 visited entries without completing.
- One real unregister commits at 132975 ms; C is absent in the rendered catalog at 133053 ms.
  The 512-image controller completion and released slot are observed at 133619 ms. The other scan
  completes 10000 images with zero issues at 163766 ms, and its controller retires at 164092 ms.
  C remains absent through peer refreshes. Exact completed entry counts are 526 and 10012.
- Reimport becomes eligible at 164289 ms after both update leases and removal display work retire.
  The real picker already points to the admitted two-image source; Import starts one C scan at
  221244 ms, which completes two images with zero issues at 221328 ms. Selecting C produces current
  decoded images and ready at 238165 ms. Final root counts are 10000/512/2, with one unregister and
  exactly three real scans across the lifetime. No Retry or additional update is used.
- Final observation records 12547 frames and 12418 current-pixel frames without a latched failure.
  Normal close takes 403.7967 ms; the 265740 ms parent exits zero and its process, Job and monitor
  retire without cleanup failure. Across 996 samples, peak working set is 517623808 bytes, sampled
  private memory 427925504, kernel peak commitment 449241088 and minimum system availability
  5125750784 bytes.
- Full before/after integrity checks pass in 56.440/57.743 seconds. All 10514 generated sources
  preserve exact membership, identity, content and timestamps; the derived catalog's complete
  per-root active relative paths remain 10000/512/2, and SQLite quick-check passes.

Ignored `.build/r2c-overlap-native/` retains the sources, boundary tests and artifact admission;
`build/integration-storage-eba9b07350404c76a559476cea6afed0/` retains trace, milestones, result,
resource/process and source receipts. The executable and Rust DLL retain the preceding native
result's hashes. The Debug kernel SHA-256 is
`8B11AC3B9DCCDCB9375B5F9BC0E0FC771F94A7B1D59DB6290FE7D1D3AE2B6188`, and native stdout is
`22BC0A0EA8C5CEE6C49824A9EA876065166BD2B157D879EFD7DF40492188F638`.

The native result establishes this removal/publication/UI-refresh overlap and subsequent real
reimport. Existing controlled old-spool/new-registration evidence remains separate: no retired
raw-spool rows are fabricated, and physical database reclamation is not measured. UX-06C, complete
focus return, C09/C10, Release, C01/C02 and final/external acceptance remain open. This does not
establish unmodified backend latency, arbitrary concurrency coverage or full R2c acceptance.
Independent read-only result review confirms the scoped overlap, command counts, exact membership,
source integrity, process/resource evidence and document mappings without a remaining blocker.
Charge this 90-minute reservation in full through 4534 minutes.

### Native directory loss and automatic restoration

The 2026-09-23 continuation adds native UX-06C evidence on documentation head
`52dc04e5fd550090fdba66ec48edc0e8e212e219` and unchanged product source. It retains the existing
10514 generated sources with a fresh derived catalog copy, then imports two copied generated PNGs into a new
GUID-owned root through the actual picker. Only this new directory is temporarily renamed to its
checked sibling and restored. Real production synchronization observes availability; no watcher
events, controller actions or recovery scans are injected. One actual 10000-image update waits at
the scanner port until the selected two-image root is unavailable with cached pixels retained.

Before the sole lifetime, method review finds an unsafe diagnostic recovery dependency: a child
content-check failure could prevent directory restoration by repeating that same check first.
The corrected restoration owner verifies canonical endpoints, directory identity and the vacant
original target, restores the directory, then checks content without erasing the original failure.
Three Python cases pass for changed children, occupied target and foreign directory identity;
three Dart cases cover unavailable/converged status and advancing owned peer execution. Initial
analysis rejects two missing statement braces, which are corrected. Seven Dart files then pass
format and fatal-info analysis; the Debug build takes 28.8 seconds. Diagnostic Dart sizes are 169
startup, 305 observation, 138 scanner, 61 catalog, 111 run and 32 boundary lines, with 99 dedicated
test lines; there are no production or inline-test changes.

Lifetime `fdd8e052b66c49f7a510a3d723ca5c87` records the connected outcome:

- The real picker import completes two images with zero issues at 66639 ms. Selecting that root
  establishes two current decoded images, exact location identities and healthy synchronized
  production status at 87933 ms. The actual update dialog selects only the 10000-image peer.
- The peer command enters at 125452 ms. Production reports the selected root missing/unavailable
  at 125626 ms; the rendered observer records both cached images at 125664 ms and releases the
  actual Rust scan. At 125778 ms that scan has accepted 25 images from 37 entries while the root
  remains unavailable with both images visible. Four unavailable frames contain current pixels.
- The guarded helper restores the directory, preserving its identity. At 125903 ms restored
  availability overlaps the still-reserved peer update, which has reached 100 accepted images.
  Production proceeds through queue publication, metadata inventory and reconciliation; a transient
  `catalog_database_busy` issue retains updating state and clears. By 127882 ms the restored root
  is healthy and synchronized with zero pending, retry or unknown work and no blocked recovery.
  Its original two location identities remain present; no manual rescan or Retry is used.
- The peer completes exactly 10000 images with zero issues at 157598 ms. Ready at 157658 ms requires
  its completed task, independent execution retirement, both roots synchronized, two current
  selected images and exact 10516 total membership. Exactly two real scan commands occur: initial
  two-image import and the 10000-image update. The final observer has 8466 frames, 8429 current-pixel
  frames and seven production synchronization transitions without a latched diagnostic failure.
- Normal close takes 395.8089 ms. The 186412 ms parent exits zero with its owned process, Job and
  monitor retired and no cleanup failure. The directory helper exits 58885.6033 ms before close.
  Across 694 memory samples, peak working set is 505028608 bytes, sampled private memory 412897280,
  kernel peak commitment 426303488 and minimum system availability 5148241920 bytes.
- Full pre/post integrity checks pass in 73.048/57.276 seconds. All 10516 generated images preserve
  exact file identity, content and timestamps. The restored directory retains its original identity,
  and complete active catalog relative paths match 10000/512/2/2, with SQLite quick-check passing.

Ignored `.build/r2c-recovery-native/` owns the diagnostic implementation, boundary tests and artifact
admission. `build/integration-storage-e84f07c4443e4008b0c71381991477a4/` owns the isolated derived
catalog, new generated source, helper/source receipts, milestones, trace and process evidence.
The Debug kernel SHA-256 is
`122F9413C26834D5F951088E8C9F3C937EDFDEF8284758C4F08331AEA529E8F7`; native stdout is
`BA94D3759357F06253FEB01688DB954B3E716418B2758647567CC6F7283BFC16`.
This establishes the selected Debug directory-loss/recovery overlap and production availability observation,
not unmodified backend latency, physical-device reconnection, signed-service, real Journal, Cloud
Files, Release or complete R2c acceptance. The other frozen variants and final gates remain open.
Independent read-only result review confirms the same lifetime's 39 structured events, exact source
and catalog integrity, helper/process/resource evidence and all three document mappings without a
remaining blocker. Charge the full 100-minute reservation through 4634 minutes.

### Observation preparation

The first mixed-load probe measures accumulated production poll wall time separately from the
existing test observer's evidence-query wall time. It prints bounded ten-second progress reports
and terminal totals. Hypothesis: the unresolved completion cost may lie in production work,
test observation, or both; no cause has been selected. Every original query, workload, assertion,
sleep and deadline remains in place. This diagnostic-only test change cannot close item 2 alone.

The installed Release storage path intentionally excludes `CEDARFLAKE_AME_TEST_STORAGE_ROOT`
through `cfg(debug_assertions)`. An ordinary Release launch would use the configured user catalog.
It is not a safe substitute for an isolated fixture run. A reviewable client-run description is
required before any retained-root authorization request; no configuration or real catalog has
been altered for this cycle.

### First controlled mixed-load observation

The current-source single-test inventory discovers 1,504 tests. The first discovery invocation
adds only the timing counters described above and runs the exact production P0 priority test with
all features, one compiler job and one test thread. It passes in 218.85 seconds; compilation and
invocation total 264.81 seconds. All 25 P0 samples progress both lower lanes; P0 P50/P95/maximum
are 88/110/111 ms. P1 completes 2,048 candidates; P2 stages and completes all 10,000, closes its
coverage, publishes `current`, retires its authority and passes FULL reopen plus source-byte checks.

The recovery tail takes 138.051 seconds over 3,068 polls: 99.110 seconds accumulate inside production
poll calls and 31.803 seconds inside the existing test evidence queries. The remaining tail includes
the unchanged sleeps and reporting. These concurrent wall-time observations do not assign worker
CPU or disk time, identify the historical timeout cause, or justify removing observer assertions.
The original failing Daily and 274.12-second isolated pass remain unchanged in the prior record.
Local output: `build/diagnostics/r2c-closeout-mixed-observation.log`.

The Debug client preparation uses the existing production `main` with only an in-memory
`SharedPreferencesAsyncPlatform` and the existing Debug storage override. Its entrypoint refuses
non-Windows, non-Debug or missing fixture-marker execution. It introduces no production option or
Release storage bypass. Its generated sources are 2,304 bounded 320-by-240 PNGs split across roots
A/B/C, with a source hash manifest and derived storage outside those roots. The scratch entrypoint,
fixture generator and owned-process wrapper live under `build/diagnostics`; they are discovery
equipment, not a second committed automation framework or current Release evidence.

### Bounded Debug desktop observations

Both allowed desktop sessions have now been consumed. The first owned run reached its unchanged
1,800-second deadline while waiting for desktop-tool access; no application input occurred. Its
receipt records confirmed process exit and Job closure with no cleanup failure. The extended
permission wait is tool/user wait, not active engineering or evidence of an application crash.

The second session ran from 08:10 to 08:38 UTC and included two actual process lifetimes within
the same 30-minute limit. The first lifetime ended normally after 663,947 ms; the second ended
normally after 959,746 ms. Both receipts record exit code zero, process exit, Job closure and no
cleanup failures. A fresh PID and start time distinguish the relaunch from a widget remount.
The Debug executable/DLL SHA-256 identities are respectively
`DEE7E6CD4E0A621D310ED75FE7165C366556855ED473D357DE30A89EB6F510FA` and
`50CCD3C6568CCB3E538ECD34A615963B3A1D5BB5ADA02C2E1372EAB746FED2F1`.

| Observed transition | Result and evidence limit |
| --- | --- |
| Native picker imports A with four Chinese subfolders | 2,052 entries checked and 2,048 images published; actual generated PNG thumbnails appear. Wrong-extension/corrupt admission remains covered by the existing controlled scan integration, not this all-PNG fixture. |
| Attempt to pause A after visible scan progress | The scan completed before the next action took effect. This is not a successful pause, interruption or checkpoint-recovery case. |
| Open image, press Right, press Escape | Actual source view changes from `image-2047.png`, 1/2048, to `image-2046.png`, 2/2048, then returns to the gallery. Pending-read retirement and Release decoding are not inferred. |
| Close EXE, relaunch same derived storage | All 2,048 catalog images return. Expanding A shows four Chinese folders; selecting `图片-0` reports 512 images. This proves ordinary completed-catalog relaunch, not close-during-poll or interrupted-import recovery. |
| Search input | Focus is confirmed, but two literal-text tool injections produce no visible text; a single key produces `a`, a later key exposes an IME composition popup, and clear-search works. Ordinary user input failure is unproved; full search/input verification remains open. No product patch is based on this ambiguous tool/IME observation. |
| Ascending sort and time-rail drag | Query keeps 512 images, replaces the visible order, and materializes the new viewport after transient placeholders. A single-date fixture does not prove multi-date geometry or a reversal while a seek remains pending. |
| Import B while browsing A's subfolder | B publishes 128 images; A's selected subfolder remains at 512, and the all-library count becomes 2,176. |
| Start an A/B update batch and select A's subfolder | Both roots are observed updating; the next state records both complete with counts 2,048/128 and the selected subfolder at 512. This does not establish C removal during publication. |
| Remove B, then import the same directory again | B disappears from the sidebar, A remains usable, and reimported B has 128 actual thumbnails. Raw-cleanup overlap and generation isolation still require the connected persistence fixture. |

After both process lifetimes, all 2,304 fixture files match the original byte lengths and SHA-256
manifest, totaling 1,797,833 bytes. No stimulus altered source files. Both application stderr logs
are empty. The owned storage retains process receipts, application logs, the initial manifest and
`source-integrity-after.json`; machine-specific locations are intentionally omitted here.

The existing unchanged-source evidence was rechecked against the retained logs: the controlled
scan completes with exit zero and no cleanup failure; whole-window UIA completes all ten exact
phases (including 126 elements at `application-ready`) and confirms native exit. The focused
preview/source-reconciliation, query/folder, explicit-retry and source-loading suites retain their
recorded passing results. The initial Daily's four failures remain recorded; reused passing
boundaries do not transform that invocation into a pass or replace the final complete Daily.

### Remaining connected discovery evidence

The cumulative timing probe retains the exact original mixed-load test and reports a pass in
243.83 seconds, including full publication and FULL reopen. Its 7,336 production polls total
151.923 seconds. Observation includes 49.314 seconds in 14,672 root queue-metric reads and
13.590 seconds in root-availability checks. Scheduling totals 60.734 seconds. Catalog revalidation
totals 16.793 seconds. The recovery tail separately records 152.565 seconds, including 109.401
seconds in polls and 35.840 seconds in unchanged evidence queries. Nested totals are inclusive;
per-stage poll counters round each call down to milliseconds and cannot be summed as exact wall
time. These measurements identify current investigation owners, not the cause of every historical
latency outlier. Output: `build/diagnostics/r2c-closeout-mixed-stage-totals.log`.

One refinement splits the existing scheduling call without changing execution order or admission.
The original mixed load passes in 260.60 seconds, with complete P2 publication and FULL reopen.
Across 7,465 polls, live scheduling totals 32.658 seconds, journal scheduling 27.193 seconds,
recovery selection 5.236 seconds and first-import completion 3.524 seconds. Root metrics total
54.612 seconds. The 169.800-second recovery tail contains 120.891 seconds in production polls
and 41.443 seconds in unchanged evidence queries. The three discovery invocations have distinct
timing probes; none is an unchanged replay or a causal correction. Output:
`build/diagnostics/r2c-closeout-mixed-scheduling-totals.log`. Further whole-load discovery replays
are not planned; targeted query work measurements finish this observation.

UX-06B's actual admission permits C removal during unrelated A/B updates, but C reimport waits for
all update leases, including their display refreshes, to retire. The connected fixture must follow
that order and then overlap C's new generation with bounded cleanup of its old retired raw spool.
Existing tests separately prove removal scope, generation advancement and cleanup retirement;
they do not yet connect this full sequence. A focused test-only fixture is being added at the
existing metadata-inventory spool lifecycle owner without changing product behavior.

The first fixture compilation used an incorrect generation field and was corrected to the intent's
generation. Its first executable run reports 22 passing existing cases and one new-case failure:
the helper had already committed the complete inventory page, transitioning the run to `Comparing`,
where source enumeration authority is intentionally invalid. This is not evidence that FULL reopen
or old-spool cleanup revoked the new root. The corrected fixture observes the new run while raw
storage is ready and the run is still `Running`, then separately commits the complete page and
checks normal source-authority retirement. Both failed outputs remain retained.

The corrected fixture and all 22 existing spool-lifecycle cases pass together in 26.39 seconds.
The connected case proves real A publication while B still owns its scan, C removal and partial
old cleanup, C's next generation after A/B finish, current raw reads across old cleanup and FULL
reopen, and normal `Running` to `Comparing` source-authority retirement. Five generated source
files retain their relative paths, bytes and modification times. A helper mutability compile
correction is retained as a separate failed attempt. Output:
`build/diagnostics/r2c-closeout-spool-connected-4.log`. This closes the connected persistence
part of UX-06B; it does not substitute for Flutter admission, automatic cleanup scheduling,
physical database reclamation or Release interaction evidence.

UX-06C has no exact connected passing case yet. The existing single-root offline/reconnect test uses
an empty baseline; the two-root reliability test injects journal failure rather than actual root
loss and stops before recovered publication. Their passing outputs establish only those boundaries.
The missing oracle is a populated A/B catalog where A's baseline survives actual fixture loss,
B publishes new content during A's recovery, and A then converges through FULL reopen and source
checks. This remains one frozen variant, not an invitation to multiply availability combinations.

The missing production connection is now exercised by
`production_offline_root_recovery_preserves_catalog_while_peer_changes_publish` and passes in
1.56 seconds. It uses real scans for A's two-image and B's one-image baselines, renames only the
generated A source offline and back, and requires B to publish one P0 file while A is unavailable
and another while A's P2 source enumeration is held by the existing test gate. A's complete cached
page stays usable; after release both roots synchronize, exact A recovery and all queue/lineage
authority retire, FULL reopen succeeds, no automatic scan row appears, and all five resulting
source files retain the expected bytes and modification times. The two intentional B additions
are recorded separately from integrity assertions. The queued observer fixture does not establish
real Windows watcher delivery or client input. Output:
`build/diagnostics/r2c-closeout-root-availability.log`.

Release decoding/input, actual interrupted-import and pending-poll relaunch, and the named
multi-root unavailable/cleanup overlaps retain their explicit client or connected-test gaps.
No additional desktop session, broader fixture audit or new product repair is authorized by these
observations. The two original S1 obligations retain their IDs in the interleaving ledger.

## Post-discovery queue repair evidence

The terminal-history work regressions pass after extracting queue readiness and narrowing the
active-state metric projection. With 256/4096 retained completed rows, exact metrics now use
2058/28938 VM steps; live and journal path readiness each use 247/247, authoritative live readiness
248/248, and legacy recovery readiness 375/375. The 113 queue tests pass together, including due
retry, lease expiry, exhausted attempts, superseded unresolved candidates, optional-index absence,
claim ownership and rollback. Output: `build/diagnostics/r2c-c01-queue-after.log`.

The unchanged priority group passes all ten tests in 438.38 seconds. Its connection control records
PerPoll P95 199 ms with 1786 opens/closes and PerEpoch P95 103 ms with one open across 5122 polls.
The complete mixed-load case records P95 109 ms, maximum 124 ms, and all 25 P0 observations while
P1/P2 progress. All 2048 P1 candidates complete. The 162.297-second recovery tail completes all
10000 P2 entries/candidates/owners, closes journal coverage, publishes current state, retires
authority and passes FULL reopen under the original creation-to-reopen deadline. This group total
is not the duration of the single full-recovery case. Output: `build/diagnostics/r2c-c01-mixed-after.log`.
The new source still requires applicable lint, hosted evidence and final-source gates; these local
measurements do not uniquely attribute historical outliers or close R2C-C01 alone.

Physical ownership remains explicit: the queue facade is 2444 lines (2670 before extraction,
including existing test-only helpers; no inline test suite), readiness is 338 production lines,
and metrics is 229 production lines. Their new dedicated tests are 268 readiness and 119 retained-
history lines; the existing 7040-line queue test module remains decomposition debt. Production
synchronization remains 4048 owner lines plus 10486 inline-test/module lines; this change adds only
the three-line test module declaration there and puts the connected availability case in its own
390-line suite. The root-reregistration case occupies 398 dedicated-test lines. These measurements
do not waive the roadmap's remaining production runtime/lane and test-suite decomposition.

Live hosted inspection also corrects the earlier evidence snapshot: baseline `228403f` has a passing
[CI run 34313274396](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34313274396), completed
2026-09-09 05:34:16 UTC, including all ten ordinary workers and their aggregate. Its Rust result is
1485 passed, zero failed and 19 ignored; the original full mixed-load P95 is 141 ms. The earlier
failed run 34294505152 belongs to `74688ec` and fails the two availability source-topology tests
already corrected by the browsing-recovery change. Neither result verifies the current dirty repair
or erases the separately recorded intermittent native-accessibility failure.

## Same-folder count discrepancy follow-up

The dedicated [count reconciliation record](library-count-reconciliation.md) now owns the complete
metadata census, generated first-read probes and separately requested desktop comparison. The
original observations were transferred there, with subsequent results and their scope boundaries;
they were not dropped. Current same-directory totals are 48663 Photos pictures and 48624 Ame
entries. All supported-extension paths are present, while 42 other image-format candidates are
absent. The 110-entry foreground/incremental difference and historical revision anomalies remain
explicit. The older Release comparison is not final-source client acceptance or convergence proof.

## Applicable lint interruption

The first current repair lint reaches formatting and rejects two line wraps in the new spool test.
The repository formatter corrects those exact lines. The second invocation then stops earlier in
the existing native-accessibility guardrail: `Write-AmeWindowsUiaProbeRecord` throws from
`File.Replace` while sequentially replacing the progress fixture, before any live-window probe.
Both outputs are preserved (`r2c-c01-lint.log` and `r2c-c01-lint-after-format.log`). The latter is a
real failed gate, distinct from that guardrail's deliberately injected failures in preceding output.

This proven active-item lint blocker admits a narrow interruption for R2C-C02's existing probe
evidence owner: inspect atomic replacement/read lifetime and reproduce the exact failure with
owned scratch only. No desktop rerun, deadline change or retry loop is admitted. C01's controlled
tests remain passing, while its applicable lint and hosted exit remain open. The interruption does
not attribute the older application-ready timeout to this newly observed file failure without
evidence. R2C-C02 diagnostic pass 1 has a 60-minute active ceiling; a bounded 20-minute evidence
investigation is the next action. Final complete Daily invocations remain zero.

Clippy for all targets/features passes with warnings denied, and Dart analysis passes with no
issues. A first direct invocation was interrupted by PowerShell treating ordinary native stderr
as a terminating error; no Cargo/rustc process remained before the corrected invocation checked
both real exit codes. These scoped checks do not replace the incomplete applicable lint gate.

The two compiler-free evidence probes consume 11 minutes of C02 pass 1. The exact nine-stage
write/read protocol fails on the second write with Win32 1175 (`0x80070497`), preserving both old
JSON and new draft. A held non-delete-sharing reader instead yields Win32 32; a production read
followed by an exclusive open/dispose succeeds. No owned reader leak or parent polling overlap is
established. Ordinary inherited Allow ACLs and Archive attributes do not prove absence of external
filesystem interference. The original lint exception contains no retained HRESULT, so the probe's
code must not be represented as its recovered value. Private evidence and limitations are in
`build/diagnostics/r2c-uia-file-protocol-diagnosis.md`.

The sole unchanged diagnostic replay uses the original lint's approved execution context to test
the ordinary-sandbox explanation. It also fails with Win32 1175, on write four after 341 ms; both
old record and new draft remain. Output:
`build/diagnostics/r2c-uia-serial-d8e3b6a98ef84ed6a31673d2633946c3/results.json`.
This rejects an ordinary-sandbox-only explanation. That diagnostic does not rerun complete lint or
retry replacement. The responsible filesystem actor remains unknown; changing permissions or adding reader
sharing is not justified. C02 pass 1 retains its original 60-minute active ceiling.

Hosted run `34338261070` at head `a03a1c8` subsequently completes with one failed worker and its
failed aggregate. Nine workers pass and three intentionally skip. Native accessibility passes both
cases and all ten phases with application-ready 2566 ms and owned cleanup. Hosted lint also passes,
including the sequential-file guardrail; it does not explain the retained local failures.

Static/Rust fails with 1490 passed, two failed and 19 ignored in 1536.59 seconds. The connection
lifetime control fails in its PerEpoch arm at sample 18's original five-second P2 page-start bound;
its earlier PerPoll 173 ms P95 is not the production result. The full mixed-load test records
P95 1806 ms, maximum 2545 ms and two samples above one second. At its original 300-second deadline,
all 2048 P1 candidates and 10000 staged P2 entries exist, but only 9152 of 9216 admitted P2 owners
are completed. Sixty-four remain leased, the inventory is comparing, authority remains active and
the root requires recovery. No closing publication or FULL reopen is proved. The original log is
`build/diagnostics/r2c-hosted-34338261070-static-rust-completed.log`.

C01 pass 1 closes with 55 minutes charged and an unresolved hosted exit. Its second and final
diagnostic pass begins at 10:53 UTC with the unchanged 60-minute active ceiling. The next hypothesis
is unnecessary repeated catalog-file identity acquisition within a reusable session: the failed
run attributes 135.260 seconds to catalog revalidation, including multi-second identity operations.
Inspect that proof's owning boundary and all namespace-replacement callers, establish a bounded
operation counterexample before any change, and preserve exact identity/replacement safety.
Neither caching by path/time nor skipping proof, changing workloads/deadlines or another unchanged
replay is admitted. C02 remains paused at its recorded evidence checkpoint while C01 is active.

The operation-count regression fails before the second repair: 32 successful reuse proofs perform
64 fresh identity acquisitions. Its first compile attempt used an unavailable stable-library
helper and ran no tests; replacing that test-only helper preserves both logs. The actual failing
result is `build/diagnostics/r2c-c01-identity-before-2.log` (one executed failure, 0.44 seconds).

The retained-connection owner now collects typed read-only schema evidence and always performs
fresh namespace/file identity validation before returning, including on SQL failure. Initial
connection opens retain both identity observations. Sixteen owner tests pass in 12.25 seconds,
including all original maintenance, schema, active-statement and pending-publication cases and new
within-proof retarget, unreadable-old-schema, and same-file/different-WAL-namespace fixtures.
The 32-proof operation bound now passes without retaining an identity cache. Output:
`build/diagnostics/r2c-c01-identity-after.log`.

The unchanged complete priority group passes all ten tests in 426.34 seconds. Its PerPoll control
records P95 177 ms and 1787 opens/closes; PerEpoch records P95 127 ms, one open and no premature
close across 5853 polls. The production full-recovery case records P0 P95 112 ms, maximum 113 ms,
all 2048 P1 candidates and all 10000 P2 staged entries, candidates, owners and completions. Its
151.336-second recovery tail reaches completed inventory/control, retired authority, current root
and checkpoint, closing publication and FULL reopen under the original creation-to-reopen bound.
Output: `build/diagnostics/r2c-c01-identity-mixed-after.log`. These local timings are not a paired
comparison with the failed hosted machine and do not close the hosted prerequisite.

All six process-owned poll-catalog caller tests also pass with all features in 2.92 seconds,
including proof revocation, failed replacement, unwinding and retirement of file constraints.
Output: `build/diagnostics/r2c-c01-identity-caller.log`. The applicable complete lint then exits zero:
PowerShell guardrails, unchanged formatting, all-target/all-feature Clippy with warnings denied and
Dart analysis pass. Its sequential native probe guardrail passes this time, without a change to
that owner; the earlier File.Replace failures remain unexplained. Transcript:
`build/diagnostics/r2c-c01-identity-lint.log`. This is repair lint, not a final complete Daily.

The proof owner is 105 lines including its test-module declaration, with 266 dedicated general-test
and 279 dedicated namespace-test lines. Catalog identity is 89 lines including 14 new test-only
counter lines; no production path cache or additional native API is introduced. The large local-
files facade gains only a two-line test-counter re-export, with no new production responsibility;
it remains 7480 lines, comprising 4061 lines before its inline suite and 3419 suite/module lines.
All original schema, session, lease, transaction, statement and publication-buffer checks remain.

The single independent review covers baseline `228403f` through `ca7dcce` plus the candidate diff.
It finds no new actionable S0/S1 code defect after examining namespace admission, pinned SQLite
WAL lifetime, queue semantics, causal evidence and missing coverage. The retained-connection
change stays within the existing admission invariant; it does not make two time samples equivalent
for every transient namespace history. Its safety rationale does not assume a permanently held
writable identity guard. The review identifies one S2 documentation ambiguity: ADR 0024's earlier
checkpoint describes two observations while ADR 0025 describes current reuse. An explicit link and
historical-scope clarification resolve that ambiguity. The one scoped recheck confirms the
correction and finds no new contradiction, using one additional active minute. No product
code changes follow the review, and known C01/C02 and client/final-source blockers remain open.

## Execution accounting

### Original candidate verification checkpoint

Candidate source is frozen at `fa1eff03a18d6329f0231a83ac94216420e8eac7`, which is confirmed equal
to remote `codex/r2c`. No new branch is created. Hosted run
[34347019582](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34347019582) completes with
failure on that head. Freezing this candidate does not close the known C01/C02 or client obligations.

Nine hosted partitions complete successfully. The actual PR
checkout is `af962aacfd97c46c8cbec193dc02664da36d78df`. Flutter's 71 passing file processes match
the current 71-file inventory exactly and sum to 565 passing tests. The controlled Windows scan
exits zero with no cleanup failure. Accessibility passes all ten phases, with 126 elements at
application-ready in 2189 ms, and confirms native exit/Job closure. The unsigned worker and all
five synthetic workloads pass; each synthetic log proves exactly one executed, nonignored test.
Seven-format evidence retains unchanged sources and reused caches; the 50000-identity publication
has one commit, 96 overlapping polls and no partial observation. The full readout and exact retained
log hashes are under `build/diagnostics/r2c-hosted-34347019582-*`. These partitions do not replace
the failed complete local Daily or establish the missing EXE/Release interaction variants.

Static/Rust finishes at 12:17:50 UTC with 1496 passing tests, one failure and 19 ignored in
1427.08 seconds. The required connection-lifetime control fails in its PerPoll arm at sample
index 15's original five-second visibility bound: fresh end-of-proof identity costs 7975 ms,
catalog open totals 7980 ms and the P0 row remains pending. The PerEpoch arm never starts.
The separate production full-recovery case passes with P0 P95 163 ms, maximum 461 ms and no
sample over one second. Its unchanged completion path requires all 2048 P1 and 10000 P2 results,
closing publication, synchronized state, authority retirement and FULL reopen. Its detailed tail
timing is captured output, so no exact hosted tail duration is inferred. The incomplete two-arm
control remains a failed required gate; it is not a production PerEpoch failure and is not waived.
The complete log and SHA-256 are retained in the hosted readout. No unchanged replay follows.

C01's second and final diagnostic pass closes at 48 active minutes. The queue and identity work
reductions have passing causal work bounds, owner/caller regressions and local complete workload
evidence, but fresh identity operations still show multi-second stalls and the required hosted
control remains unresolved. There is no third diagnostic pass, lower workload, longer deadline or
weaker identity proof. Further C01 scope requires a bounded revised plan; the proposed C02/C03
diagnostic supplement does not reopen it. Documentation-only heads retain this failed product-
source evidence; a routine CI run triggered by recording it cannot erase the failure.

The local unsigned Windows gate passes on the clean candidate, from 11:44:18 to 11:52:17 UTC.
It builds the Release application, verifies all three engine-free native window cases and both
Debug-engine retirement cases, builds the broker and passes the isolated Release bridge smoke.
Native process exit and Job closure are confirmed with no cleanup failure. The payload contains
19 files, 79425956 bytes and 172 Rust source dependencies; the packaged Rust DLL SHA-256 is
`4d8eb022f5e7021340c6dfac08b959c2dec2d7fb33aaf9d061464dbad60ac14c`.
It accesses no catalog and is not signing, installation or real-library client acceptance.
Output and copied evidence are `build/diagnostics/r2c-fa1eff0-unsigned.log` and
`build/diagnostics/r2c-fa1eff0-unsigned-evidence.json`.

The first complete serial Daily runs after the unsigned command and exits one at 11:54:28 UTC.
It stops in lint's existing R2c-R guardrail: safe cleanup sees a nonempty owned fixture root and
refuses traversal/deletion. Rust tests, Flutter tests, both native integrations and final bridge
checks do not execute in this Daily. The original transcript is
`build/diagnostics/r2c-fa1eff0-final-daily-1.log`, SHA-256
`09A90CD17AEF0A1C6FBD6913027B1974377AA45ABECC2E654B21EC42372B4FC2`.

This new R2C-C03 verification finding is S1 for the required gate, with root-cause and repair
admission unresolved. It is not evidence of source-media deletion or a third proven independent
product defect. The guard correctly retains the scratch root. The log places failure after the
zero-test rejection and before the anchor/junction fixture completes; it contains neither the
failing instant's child inventory nor the original exception if a finally-cleanup failure masked
one. A post-failure metadata observation at 11:56:23 UTC finds the same path empty, without proving
physical identity continuity or the earlier child's lifetime. It does not justify a cleanup retry,
an external-interference attribution or grouping this with C02's File.Replace failure.

The root and logs remain untouched after read-only inspection. Private metadata evidence is
`build/diagnostics/r2c-fa1eff0-daily-retained-root.json`. No unchanged replay or second Daily is run.
The post-review finding invokes the plan's bounded replanning rule; the one further complete Daily
still requires a causal correction or verified execution-environment remedy. Passing hosted
partitions cannot replace this failed complete invocation.

Discovery closes with the two connected persistence/production cases above and explicit unresolved
EXE/Release interaction evidence. It does not declare all 24 variants accepted. The last query-cost
fixture passes FULL validation and confirms exact idle results while measuring avoidable work:
256/4096 retained completed rows require 4495/69775 VM steps for each live/journal path readiness
check, 1936/28816 for authoritative-live readiness, 4615/69895 for legacy-recovery readiness, and
14230/225430 for exact metrics. Its first setup attempted to duplicate the schema trigger's lane
row; the corrected fixture asserts that existing ownership instead. Both outputs remain retained;
the successful output is `build/diagnostics/r2c-closeout-retained-query-work-2.log`.

The single triage admits R2C-C01 and R2C-C02 from the existing ledger, in that order. No new source
safety defect or confirmed search-input defect was found. Missing client evidence retains its
original blocked variants and does not become an invented third defect family. R2C-C01 diagnostic
pass 1 begins at 09:17 UTC with a 60-minute active ceiling: verify the measured terminal-history
query work at its adapter owner, add a failing work-bound regression, establish the typed readiness
boundary, and compare the unchanged full workload after a causal correction. This is the historical
pass-1 admission; the second and final pass is recorded above. No further whole-load discovery run
is admitted; production queries were unchanged at
pass admission. The subsequent work-bound regressions fail on that source before repair:
256 terminal rows require 14,230 metric VM steps against a 7,168 counting-work ceiling and 4,495
live-readiness steps against the fixed 1,024 idle-work ceiling. Output:
`build/diagnostics/r2c-c01-work-bound-before.log`. Counts remain exact; the readiness bound applies
to a FULL-valid catalog with no unresolved rows and its ordinary schema-owned access paths.

All active engineering, including delegated inspection, preparation, failed attempts, and result
analysis, counts against the plan's phase ceilings. Tool execution/wait time is recorded separately;
concurrent inspection during a tool run still counts as active work. A handoff does not reset usage.

| Phase | Ceiling | Used at last checkpoint |
| --- | --- | --- |
| Baseline and discovery | 4 h | 176 min charged; original discovery complete, plus the explicitly requested 26-minute desktop count comparison |
| Triage | 1 h | 54 min charged, including count/admission analysis, the generated first-read probe and read-only preservation of the post-review gate failure; two repair families remain admitted |
| Diagnosis and repair | 8 h | C01 pass 1 closes at 55 min and final pass 2 closes at 48 min with the required hosted control still failing. C02 pass 1 has 26 min charged and is paused; the failed context-comparison replay remains retained |
| Independent review and scoped recheck | 2 h | 11 active minutes total: one 10-minute review plus the one-minute scoped recheck; both complete and both invocation allowances exhausted |
| Closeout recording | 1 h | 16 min charged for source freeze, verified log collection, results and unresolved checkpoint preparation |

Final complete Daily invocations: 1 failed. Unchanged diagnostic replays: 1 executed; allowance exhausted.
At the original checkpoint, one independent review and its scoped recheck were complete and their
allowances exhausted. The subsequent parallel supplement's separate review allowance is recorded
above; it does not reset these original counts or time.
The controlled cycle and the separately authorized external/client acceptance remain open.
