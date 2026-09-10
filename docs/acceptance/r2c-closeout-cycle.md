# R2c controlled closeout cycle

Status: functional-first continuation active; C01/C02 and local Daily failure remain unresolved.

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
| UX-01A | Warm existing catalog starts with usable content; no-change continuity enumerates/opens no source and creates no inventory | Startup/lifecycle; `library_synchronization_lifecycle_owner_test.dart`; Rust `one_hundred_no_change_startups_complete_the_production_observer_path_without_scanning`; mapped, combined client observation pending |
| UX-01B | Close while start/poll is pending, then launch the same fixture catalog; old PID exits within the existing bound and old epoch cannot publish | Window shutdown and synchronization epoch; `window_manager_actions_test.dart`, `library_synchronization_test.dart`; owned EXE relaunch pending |
| UX-01C | Launch with interrupted first import; display paused checkpoint, require explicit Continue, preserve truthful portable capability | Scan restoration; `library_controller_test.dart`, `library_viewer_position_test.dart`; process evidence shares UX-01B/02B |
| UX-02A | Import through the actual picker with Chinese paths, wrong-extension PNG and damaged input; exact count/issues and unchanged source | Primary scan, media admission; `integration_test/scan_workflow_test.dart` controlled picker case; prior Debug native evidence mapped |
| UX-02B | Pause then cancel before native Started/registration; cancel wins, late Started does not regress feedback, execution retires | Scan control; `library_primary_scan_control_test.dart`, retained-import workflow; deterministic boundary mapped, real interruption/restart pending |
| UX-02C | Cancel replacement before commit or fail display after commit; preserve previous baseline or retry display exactly once without rescan | Publication control, committed refresh; Rust `accepted_cancel_before_projection_commit_preserves_the_published_baseline`, `library_primary_scan_workflow_test.dart`; mapped |
| UX-03A | Switch root/search/sort and cross a page while publication proceeds; latest query owns coherent page/count/timeline | Query snapshot and viewport; `library_query_snapshot_reader_test.dart`, `library_controller_test.dart`; mapped, Release input pending |
| UX-03B | Expand a folder, change revision, load another folder page; replace obsolete window and append only at the same revision | Folder paging; `library_folder_controller_test.dart`; mapped, Release input pending |
| UX-03C | Fail display after committed removal/update, replace query and explicitly retry; retain page, settle loading and never repeat source action | Query refresh/removal; `library_query_snapshot_reader_test.dart`, `library_update_refresh_test.dart`; mapped |
| UX-04A | Materialize cold then warm actual media previews; verify pixels, source version, cache ownership and unchanged source | Preview store/media adapters; `media_format_tests.rs`, existing seven-format acceptance; mapped, Release decoding pending |
| UX-04B | Rewrite fixture with identical size/ID/mtime while the old request is pending; reconcile one path and show new pixels, reject old publication | Source reconciliation/lease; `preview/tests/source_reconciliation.rs`, `preview/tests/failure.rs`; mapped, interleaving evidence to verify |
| UX-04C | Hold source exclusively or present corrupt bytes, then recover; preserve precise failure, valid source retries and no stale ready publication | Preview failure; `source_open_failure_keeps_its_cause_and_does_not_enqueue_reconciliation`, `explicit_retry_of_same_version_corrupt_source_retires_legacy_ready_ownership`; mapped |
| UX-05A | Open original, navigate both ways and return; actual decode, correct anchor and released source slots | Viewer/source reader; `integration_test/support/viewer_source_workflow.dart`, `library_viewer_position_test.dart`; Debug decode mapped, Release pending |
| UX-05B | Close/reopen while paging or buffer copy is pending; old completion/errors remain retired and new navigation works | Viewer session/source lifetime; `library_viewer_navigation_lifecycle_test.dart`, `library_source_read_lifecycle_test.dart`; mapped |
| UX-05C | Same-path source rewrite, then authoritative rename/removal; newest pixels and stable asset until authoritative removal | Viewer source generation; `library_viewer_image_test.dart`, `library_viewer_position_test.dart`; mapped, Release input pending |
| UX-06A | A/B update while queued C is cancelled; independent progress and only C is cancelled | Multi-root update; `library_update_controller_test.dart`; mapped |
| UX-06B | Remove C while A publishes and B continues, then register C while old cleanup remains; no repeated unregister, stale root or cleanup authority | Root removal, publication and cleanup; `library_update_controller_test.dart`, queue registration/retention regressions; exact connected overlap pending |
| UX-06C | Make fixture A unavailable then restore it while B updates; preserve A's catalog and B's progress | Root availability/scheduling; production offline/available and R2c-R isolation cases; mapped, connected overlap pending |
| UX-07A | Original 25-sample P0/P1/P2 workload through complete P2 publication, authority retirement, synchronized state and FULL reopen | `production/tests/priority.rs` and `priority/recovery_completion.rs`; original 300-second failure remains S1, discovery observation timing added |
| UX-07B | Same workload with per-poll versus per-epoch connection lifetime; preserve proof and production P95 bound | `priority/connection_lifetime_control.rs`; existing two-arm evidence mapped, no full-recovery substitution |
| UX-07C | Interrupt/reopen exact leases and exhaust recovery retry; retain durable failure/lineage and keep other roots eligible | Production restart/stop and queue retry owners; `production_restart_recovers_an_expired_live_gap_lease_and_retains_its_consumer_lineage`, `exhausted_recovery_candidate_prevents_authority_completion`; mapped |
| UX-08A | Jump by scrollbar/time rail then reverse before completion; immediate visible demand and no old seek rollback | Gallery visible range/time navigation; `library_time_navigation_test.dart`; mapped, Release input pending |
| UX-08B | Original ten-phase populated whole-window UIA sequence and native process exit; no invalid AXTree | `integration_test/windows_accessibility_bridge_test.dart` and existing public runner; prior local pass and hosted timeout both retained |
| UX-08C | Keyboard menus and task Retry/Cancel; correct focus return, immediate feedback and one committed action | Shared menu/task surfaces; `ame_menu_test.dart`, `library_retained_scan_interaction_test.dart`; mapped, actual client keyboard pending |

Flutter filenames above are under their existing `test/app` or `test/features/library` owners;
Rust test owners are under `rust/src`. This is one fixed cross-layer discovery pass, not a full
repository audit. No UI redesign, source operation feature, dependency or schema change is admitted.

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
