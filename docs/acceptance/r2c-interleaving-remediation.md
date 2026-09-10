# R2c cross-workflow interleaving remediation

Date: 2026-09-07

Status: implementation and verification in progress; not release acceptance.

## Scope and safety

This follow-up covers interactions missed by the earlier green PR #12 head: source admission and
destructive cleanup, catalog maintenance and active scans, discovery and retry ownership, scan
commands and asynchronous registration, and viewer/cache ownership after an operation is retired.
It does not infer safety from isolated green tests or claim that every possible defect is excluded.

Reproductions use generated disposable sources, isolated catalogs, and controlled application
ports. Those controlled reproductions do not access retained source roots, live user catalogs,
cloud placeholders, installed services or original media. A later same-folder diagnostic reads
catalog and directory metadata; a separately requested desktop comparison also opens the existing
Release client and permits its normal derived-state work. The
[count record](library-count-reconciliation.md) preserves these distinct scopes, artifact identities,
private-path boundary and source-verification limitations. Neither is final retained-library or
signed/installed-service acceptance.
The active delivery order remains in [the canonical roadmap](../roadmap.md).

The active [controlled closeout cycle](r2c-closeout-cycle.md) freezes 24 variants and records their
baseline, observation costs and client coverage. The original discovery is complete and the single repair batch
is frozen to the existing blockers below. The latest direction resumes the remaining functional
journeys first. C01/C02 remain unresolved final-verification obligations; their current evidence
does not establish a highest-priority user incident. C01 has potential recovery impact and is not
classified harmless. C02's known failures are in automation/evidence handling. A reproduced direct
functional or source-safety failure takes priority immediately. The preceding supplement and P2
follow-up retain their unresolved checkpoints and consumed experiment limits:

| ID | Scenario / severity | Evidence and owning obligation | Blocked exit |
| --- | --- | --- | --- |
| R2C-C01 | UX-07A/B / S1 | Queue-work and retained-proof corrections pass their causal bounds and focused regressions. Current hosted controls and full recovery pass, but the second complete local Daily fails the unchanged 300-second full-recovery bound with 8960/10000 owners completed; authority, final publication and FULL reopen remain incomplete. Both local lifetime controls pass. The P2 follow-up proves future raw output capture, then finds identical plans/work/results for its original versus owner-first SQL projection; no causal correction is supported and conditional Daily is unstarted. Earlier native-retirement and SQL-proof outliers remain unexplained. The cycle record retains exact state, limits of this negative result and the original native-output gap. Further experimentation requires bounded replanning. | Item 2, candidate readiness |
| R2C-C02 | UX-08B / S1 | The hosted `application-ready` eight-second timeout recurs before stage instrumentation. Instrumented local and hosted ten-phase UIA runs pass, with exact completion and owned-process retirement; the earlier timeout and File.Replace error 1175 remain unexplained. Controlled file-sharing experiments do not reproduce 1175 and reject MoveFileEx as a replacement. Preserve the original protocol, complete whole-window path and deadline; C03 does not establish shared causality. | Item 3, candidate readiness |
| R2C-C03 | Final-source gate / S1 finding, owner repair verified | The supplement reproduces legacy delete-pending namespace retention under a metadata observer and corrects the native owner. Focused regressions, independent review/recheck, current-source 19-case guardrails, full lint and hosted checks pass. Cleanup preserves the original error and independently retires known resources, with bounded remaining-entry evidence. The second complete local Daily passes this boundary and later fails C01. The original observer remains unidentified; this repair does not explain C02 or make the whole candidate ready. | Owner boundary verified; overall Daily/readiness blocked by C01/C02 |
| R2C-C04 | UX-03A/06A live bulk removal / S1 finding, focused/client verified | The original 2012-versus-512 failure is preserved. Typed LiveOnly subtree promotion, capacity and retained-debt ownership correct the causal gap without resetting retries or inventing journal continuity. Final 133 focused tests, lint and generated-client old-debt/fresh-burst oracles pass; exact 512 survivors and unchanged 10000 background are verified. The [C04 record](r2c-live-gap-recovery.md) retains all failures, source proofs, resource/cleanup limits and missing final-source gates. | Local count/recovery correction verified; full candidate readiness remains blocked |
| R2C-C05 | UX-03A/04A/05A/08A browsing / S1, causality unresolved | Direct timeline navigation leaves a blank wall until another scroll; an idle 2012-image middle viewport is independently observed blank. User reports rail disappearance, transient whole-wall thumbnail errors after deletion, persistent gray cards without retry after jumping, and a brief viewer-return layout change. The [observation record](r2c-live-gap-recovery.md#c05-browsing-findings--unresolved) distinguishes captured evidence from unverified cause and preserves the pre-removal database. These symptoms are not assumed to share a cause or to follow from C04's backend correction. | Item 3, usable browsing and candidate readiness; bounded scope decision pending |

The [mixed-media observation](r2c-closeout-cycle.md#mixed-large-image-and-historical-date-observation)
verifies exact 10000-file inventory/source preservation and historical dates, with actual original
opens spanning twelve dimensions. Its continuation ends at the fixed deadline with coverage gaps;
it does not establish a new product defect or close C01/C02. A bottom-rail input discrepancy is
unverified and deferred under UX-08A: the owning boundary is time-rail hit mapping, projection and
seek/viewport alignment. The current evidence does not distinguish a product failure from tool
coordinates; require a deterministic reproduction before any correction. The manually added
non-fixture root in the first catalog is accounted for separately and is not a registration bug.

Current C01 checkpoint `dd3f556` removes the reproduced terminal-history projection/eligibility
work. All 113 queue cases and ten priority cases pass; the unchanged complete mixed workload
retains P95 109 ms and all 10000 P2 results through FULL reopen. Clippy and Dart analysis pass.
An earlier applicable lint is blocked by a newly observed `File.Replace` failure in C02's progress-
record guardrail. Its narrow diagnostic interruption and preserved failure are recorded in the
[cycle evidence](r2c-closeout-cycle.md#applicable-lint-interruption). Hosted run `34338261070`
then fails both priority connection-control and full mixed-load cases: production P95 is 1806 ms,
and only 9152 P2 owners finish within 300 seconds. Hosted lint/native gates pass. C01's second and
final diagnostic pass targets catalog-identity revalidation cost while preserving the source-
replacement admission invariant. Retained-connection checkpoint `8745198` passes all 16 owner and
six caller cases, the original ten-test priority group and complete applicable lint; its local full
mixed-load P95 is 112 ms with complete recovery and FULL reopen. The unchanged file guardrail also
passes in this invocation, without explaining the earlier failures. Independent review and its
scoped documentation recheck complete with no new actionable S0/S1 defect. Current hosted run
`34347019582` then passes production full recovery at P95 163 ms but fails the required PerPoll
control at sample index 15: fresh identity takes 7975 ms and visibility exceeds five seconds.
The PerEpoch control arm does not start. Rust reports 1496 passed, one failed and 19 ignored;
the nine other workers pass. The original C01 final diagnostic pass ends unresolved. The explicit
parallel supplement now admits bounded causal repair; it does not renew unchanged replay allowance.
The cycle record owns the retained logs and supplement evidence.

Missing EXE-restart, safe Release-client and connected multi-root-overlap evidence is tracked against
the frozen variants. Those gaps are not new diagnosed defects or additional admitted repair families.
No third repair family is admitted. The missing EXE/Release paths remain mandatory coverage work
under item 3, not waived cases. Pass 1 for R2C-C01 was admitted with a 60-minute active ceiling to
test whether eligibility checks traverse completed history and exact metrics evaluate active-state
projections on terminal rows. Its query VM-work regressions and full-load comparison preserve the
schema, optional-index contract, workload and deadlines; the typed readiness owner precedes the
facade change. The cycle record owns current pass-2 accounting. Item 3 repair
otherwise waits for item 2's controlled and hosted exit; the supplement permits only the existing
file-lifecycle blocker repair concurrently. Later local success cannot close unexplained hosted
failures, and a failed diagnostic hypothesis cannot create another implicit investigation round.

The additional UX-02A/UX-03A [count investigation](library-count-reconciliation.md) now reproduces
48663 Photos pictures versus 48624 Ame entries in the same directory, with videos counted separately.
Its census finds 42 absent image-format candidates and no missing currently supported-extension
path. The retained successful-scan count is another 110 lower, matching exact terminal-failure paths
later published by incremental reconciliation. Retained revision issues remain unexplained; the
earlier larger-gap estimate was withdrawn as unreliable. Severity and repair admission remain pending the owning-policy
disposition; neither format expansion nor a third repair family is silently admitted.

## Owning invariants

| Boundary | Reproduced failure | Required repair |
| --- | --- | --- |
| Retired-preview cleanup / import | A source registered after cleanup's initial check could have its image deleted | A shared resolved-scope reservation lasts through destructive cleanup or source registration; conflicts reject before mutation, unrelated roots proceed |
| Cleanup / external namespace replacement | Replacing a checked cache directory with a generated, cataloged source at Started caused its image to be deleted | Pin the existing namespace through deletion; an initially absent namespace cannot acquire later files |
| Automatic recovery and staging disposal / namespace replacement | Replacing the cache before recovery deletion or after preview staging deleted a generated source with the same leaf | Hold operation-scoped namespace identity before inventory or staging, across capacity handoffs, until all installation and cleanup is settled |
| Temporary encoding / leaf collision | A preexisting hard link at the generated temporary name was truncated and then deleted | Claim a new leaf exclusively, encode and flush through that held handle, and never remove an unclaimed collision |
| Database maintenance / scan | Successful checkpoint discarded valid proof, making new readers fail throughout a running scan | Checkpoint preserves proof; structural maintenance shares atomic admission with scan session protection and releases it on every outcome |
| Signature discovery / retry | A temporary read lock dropped exact retry ownership for unknown or absent suffixes | Typed retryable file discovery enters the existing durable Live handoff; neither read failure nor suffix guesses establish absence |
| Primary control / registration | Cancel or pause returned false before the native worker registered and was forgotten | The exact scan run retains ordered user intent through Started, terminal events, and shutdown |
| Final scan publication / control | Accepted cancellation before projection replacement still committed a new catalog | Existing scan intent participates in bounded SQL interruption and COMMIT admission; rollback preserves the baseline, while a committed receipt wins over late control |
| Viewer close / reopen | Old pagination changed a reopened same-item viewer or retained its busy state | Viewer session and request identity own continuation and finalization; retired work cannot mutate another session |
| Capacity retry / store invalidation | Recovery replaces the store while a capacity retry retains stale accounting, rejecting free space or omitting installed bytes from the current owner | Reacquire one typed accounting/installation owner with each generation or reclamation permit; never carry the old store across exclusion gaps |
| Native verification / exceptional cleanup | Job disposal or process wait failure skipped later release, environment restoration, and evidence persistence | Independently settle every owned resource, preserve the original failure, and retain scratch evidence when exit or output capture is unconfirmed |
| Native window / controller lifetime | A font-change message delivered to a real HWND before controller creation or after retirement terminates the test process | Recheck the current controller and engine after plugin dispatch before reloading fonts; keep ordinary and default message handling |
| Native scope exit / child-window destruction | Actual engine child destruction reenters a still-exposed retiring controller | Retire controller ownership before member destruction and retire the window before COM teardown |
| Incremental preparation / concurrent publication | An unrelated live revision repeats the full prepared batch; unchanged and absent paths have incomplete source proof | Revalidate every source observation and reuse only a complete matching bounded read set, preserving final transaction authority |

The first boundary takes priority because it crosses source-media safety. A second uncoordinated
path check or one disabled UI button does not close its race. Full catalog validation during an
active scan is also not an acceptable workaround: recovery must not retire a live scan's claim.
Changes to this decision are explicit in [ADR 0005](../architecture/0005-storage-governance-and-budget.md).
Module ownership follows [ADR 0025](../architecture/0025-invariant-owned-workflow-modules.md).

## Verification ledger

- Baseline desired-behavior checkpoint regression fails at
  `catalog_validated_session_stale_while_scan_active`, after the real checkpoint succeeded and the
  original adapter session still passed its identity/schema check.
- Both permanent locked-discovery regressions fail on the baseline because the expected durable
  path owner is absent. They use a read-only Windows lock and no byte/mtime change to trigger retry.
- The permanent cleanup/import regression fails on the baseline because the overlapping foreground
  import succeeds after cleanup admission. This regression stops before deletion; the earlier
  diagnostic independently demonstrated deletion of only the generated fixture.
- Repaired catalog-session/reclamation verification passes 32 tests with no warnings, including the
  checkpoint regression, structural maintenance deferral during a real scan, busy preservation,
  success/error/interruption/unwind retirement, and independent-catalog admission. The reverse
  interleaving completes a real structural conversion while retaining maintenance admission; an
  arriving foreground scan must preempt that exact maintenance attempt and acquire a renewed proof.
  An in-flight validator also defers maintenance without waiting under the registry lock.
- Source/cleanup admission passes five production workflow tests and three reservation tests.
  These establish two-way registration exclusion and cancellation/detachment/unwind release,
  not operating-system namespace stability or real reparse-alias race coverage.
- Both cache-reacquisition regressions fail before repair through the production storage-resolved
  entrypoint. Recovery before reclamation leaves usable capacity but the request reports `Failed`;
  recovery after reclamation leaves the current owner accounting for zero bytes after a 1,973-byte
  artifact is installed. Each regression was observed independently; a poisoned test mutex from the
  first combined failing run is not counted as a second behavioral reproduction.
- A protocol-error review of scan-control repair rejected premature stream completion: visible
  failure must retain the original run until its actual stream drains, matching batch-update
  ownership. The earlier 12-test pass predates that correction and does not verify the final slice.
- The next locked-discovery run passes four tests but rejects ordinary-document completion because
  the actual gallery contains two items instead of one. The incremental terminal-media owner was
  unconditionally publishing a failed media location. The repair retains the original assertion
  and adds no-gallery-location and known-media hardlink failure/recovery checks. The initial repair
  also suppressed negative evidence; the later full-suite result below rejects that part.
- Independent viewer review also rejects a fake-only completion claim: real pagination deduplicates
  an in-flight request, so a new Next action after closing and reopening the same boundary item must
  join its actual completion. Tests must cover both passive reopen (no old automatic navigation) and
  reopen plus a new navigation action (one underlying page load and a successful new selection).
- Final focused Rust runs pass cache reacquisition (2), locked discovery and identity alias (6),
  preview materialization (22), reclamation (2), health (7), recovery (4), incremental changes (55),
  and mixed media input (8). The reacquisition cases are included in the materialization suite;
  these counts are not summed as unique tests.
- The namespace-replacement regression first fails after actual deletion of a generated source
  image. The repaired cleanup filter passes all 19 tests, including that original assertion,
  root/ancestor rename exclusion, cancellation/detachment/unwind release, and a root created only
  after Started remaining untouched. This is manual-cleanup evidence, not a claim that every cache
  writer or background remover has the same namespace capability.
- These focused runs preceded the complete repair set. No old hosted run or bug-presence assertion
  is counted as verification of later changes.
- Two additional desired-behavior probes independently fail through startup recovery and production
  preview materialization: both report `replacement_admitted=true` and `source_survives=false`,
  then fail reading the generated source with Windows error 2. No production repair was present
  during these runs. Directory protection must cover background workflows, not only manual cleanup.
- Final directional page-owner tests (8) and real viewport/viewer interleavings (6) pass. A new
  primary-task widget test stalled in its asynchronous setup/drain sequence; its exact owned tester
  was stopped after parent-chain verification. The interrupted run is not a passing scan-control
  gate, and later unexecuted cases are not counted as separate product failures.
- Repaired automatic recovery namespace tests (2), superseded staging disposal (1), and capacity
  handoff tests (4, including the two accounting regressions) pass with generated sources intact.
  An idle-store fixture initially reused an obsolete NULL source revision after the first request
  adopted its revision; its retry now uses the real publication receipt. That fixture correction
  and the final namespace suite still require the next compiled run.
- The temporary-leaf desired-behavior regression fails before the writer repair with both
  `source_unchanged=false` and `existing_leaf_survives=false`. The exclusive-creation repair has
  received an independent code review; its green execution remains pending.
- Isolating the protocol-feedback widget did not eliminate its stall. The installed Dart SDK's
  synchronous controller close can return a shared root-zone completed Future when its done
  future was not captured before synchronous delivery. The fixture now captures that same
  controller's `done` before closing it, preserving the actual drain boundary inside Flutter's
  fake-async test zone. With production code and assertions unchanged, the widget passes through
  the failed-state frame and both teardown stages. This is a test-fixture diagnosis, not evidence
  of a production rendering deadlock or a reason to bypass native accessibility acceptance.
- The next compiled Rust run passes the original temporary-leaf collision regression with both
  source bytes and the preexisting leaf preserved. It then passes 47 focused tests: namespace (5),
  storage capability (4), preparation (2), actionable capability failure (1), recovery (6), preview
  application lifecycle (26), and existing restart-activation ownership (3). The idle-store fixture
  now proves current accounting and release of OS handles after the real publication receipt.
  The collision assertion was subsequently strengthened to require successful encoding under a
  different exclusively owned leaf, so unconditional generation failure cannot satisfy the test.
- The final diagnostic-free Dart run passes 44 tests across six files: primary control (15),
  primary-task feedback (1), page-operation ownership (8), bidirectional viewport/viewer interaction
  (6), viewer-session ownership (11), and actual application widgets (3).
- The existing controller (56), primary workflow (15), image viewer (10), retained-import
  interaction (8), multi-root update flow (5), and viewer position (20) suites also pass: 114
  additional tests. Strict Dart analysis reports no issues with warnings and information treated
  as failures. The complete Flutter Daily partition subsequently passes all 70 test files with no
  skipped widget tests.
- Independent final-publication review adds one confirmed control defect. At the production hook
  before replacement-projection deletion, an independent reader still sees the published baseline;
  the real `cancel_scan` returns true, yet the scan replaces it and reports Completed. The desired
  Cancelled assertion fails. The reverse test passes: a cancellation first sent from the Completed
  event cannot undo an already committed replacement. Repair must bind existing scan intent into
  the publication transaction without changing first-import journal-permit semantics.
- Final-publication implementation and independent review are complete. The
  extracted transaction owner retires progress and priority callbacks before rollback, including
  unwinding, and distinguishes actual SQLite interruption from unrelated SQL errors. Eight new
  tests cover pre/post-publication control, first-import manual continuation, replacement baseline
  preservation, retry-callback cancellation, and same-connection cleanup after a panic. Cancellation
  cuts off at COMMIT admission; this is not a fixed cancellation-latency guarantee while waiting for
  shared write admission. The final compiled run passes all eight new tests, the existing live
  preemption/rollback regression, and the strengthened exclusive-staging regression. The original
  pre-commit cancellation reproduction now retains one baseline location and persists Cancelled;
  post-commit cancellation retains the completed two-location replacement. The injected panic is
  caught by its test, which then proves rollback and successful same-connection SQL/terminal cleanup.

Full static/Rust Daily and native Windows gates remain pending at this checkpoint. Focused evidence
above does not substitute for those gates or the separately authorized real-library acceptance.
The first full static run reached Clippy after the guardrails, then rejected a legacy preview-store
constructor whose remaining callers were test-only. That constructor is now explicitly test-only;
all-target/all-feature Clippy passes with warnings denied. No lint suppression or production
fallback was introduced. The complete static partition then passed its guardrails, formatting,
Clippy, and strict Dart analysis, but the Rust library suite failed: 1319 passed, one failed,
19 ignored. The mixed non-media/malformed inventory test expected 128 gallery mutations and observed
32. Only 32 malformed-media locations is correct; losing the other 96 version-bound negative
observations is not. The repaired terminal-media owner now completes that batch with 128 exact
version-bound observations and 32 gallery mutations. The compiled follow-up passes the dedicated
inventory suite (3), the complete incremental application suite (57), locked discovery (6), and
the atomic evidence/lease regression (1). These overlapping filters are not summed as unique tests.
The tests reopen the catalog, require zero unchanged candidates, and overwrite a non-media file
with PNG bytes while preserving its file identity, byte count, and modification time. Paired-rename
tests retain current-path evidence without assigning previous-path evidence to the current lease.
The independent follow-up review found no additional defect in that boundary. No aggregate Rust
success is claimed from the focused rerun.

A later static invocation failed the existing R2c-R owned-child fixture before compilation: its
15-second parent deadline expired without exactly one child marker. The fixture now flushes six
phase/timing markers and reports the original bounded-process result on that assertion; its exact
source digest was refreshed without broadening source admission. The complete focused guardrail
then passed all 19 cases under the unchanged deadline and cleanup assertions. This rerun did not
reproduce the failure and does not establish or repair its cause. The next static invocation also
passes that unchanged guardrail, all other compiler-free boundaries, formatting, and all-target,
all-feature warnings-denied Clippy. Its final Dart analysis rejects a missing import in the temporary
UIA heartbeat instrumentation; the corrected strict analysis passes. The temporary Dart change is
subsequently removed exactly, not retained as a product repair. These corrected partition results
do not claim one completed full Daily invocation or a new hosted-head result.

The subsequent hosted run [34102982670](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34102982670)
passes on exact head `223c0a7d7ba480ac7d5067b01112a61f50635f61`: Static and Rust, all Flutter
tests, controlled Windows scan, native accessibility, all five synthetic workloads (including seven
media formats and 50,000-identity publication), and unsigned Windows x64 verification. Protected
signing and signed verification remain separate and were not executed. This is completed hosted
evidence for that head, not verification of subsequent working-tree changes or resolution of the
Windows 11 client failures below.

Hosted run [34108751295](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34108751295)
also passes every required job on `f9fab3891806bd84c7bfaef590dbe1702ab6eff5`, including the stricter
photo-menu assertions and persisted probe evidence. It predates the native font-change repair and
new runner gate; it does not verify those subsequent changes.

## Native process lifecycle follow-up

No user-launched Ame process is assumed. A read-only local snapshot found one historical process
record with zero threads, zero handles, and no available executable path; its parent no longer
exists. The available test logs do not cover that record's creation time. Windows Application Error
and Error Reporting records do identify its parent Ame instance crashing in `flutter_windows.dll`
with `0xc0000005` and then `0xc000041d`, at the same module offset and application start time.
The archived report contains metadata but no retained dump. This proves a historical native crash,
not an executing scan or the cause of the zero-thread record. No matching stack is available, and
the tool-cleanup repair does not claim to explain or remove that record or repair the Flutter crash.

The accessibility runner did contain independently confirmed exceptional-cleanup gaps: failure in
job disposal or process waiting bypassed subsequent resource disposal, environment restoration,
and current evidence persistence. An environment setter could fail before its restoration scope,
and Pop-Location failure could skip tool-lock release. A completion-write failure also replaced the
original run failure. The extracted cleanup owner preserves separate stage errors and original
failure precedence. Initial compiler-free failure-injection and existing accessibility guardrails
pass. Independent review then rejected unconditional scratch removal after failed output capture
or unconfirmed process exit. The correction also retains evidence after failed Job closure or phase
capture; five controlled tiny log/JSON fixtures prove exact bytes survive and destructive cleanup
is not called, while resource release continues. The final compiler-free regression passes.

The current controlled native scan run passes with actual Debug application execution, retained
first-import/manual-continuation interactions, terminal cancellation, and successful owned-tree
cleanup. The subsequent native accessibility run fails at the first populated-application UIA
checkpoint. A second run adds narrowly separated cache-activation, subtree-query, and cache-disposal
stages and fails at the same checkpoint: cache activation returns, but `FindAll(Subtree,
TrueCondition)` has not returned before the unchanged eight-second parent deadline. Its last
`elementCount = 0` is an incomplete query record, not proof of an empty native tree. Neither run
reports a rejected AXTree update, but neither passes native accessibility acceptance. Both record
confirmed primary-process exit and Job closure, no cleanup failures, and persisted current failure
evidence. Read-only post-run process snapshots contain only the historical zero-thread record;
no new Ame crash event is found for this controlled-run interval. UIA traversal and the historical
native crash remain separate unresolved investigations.

A third unchanged-deadline native run adds a temporary, bounded real-timer heartbeat to the existing
checkpoint, without changing its waiting or frame policy. At `application-ready`, heartbeat output
continues from 505 to 8001 milliseconds; 104 frame pumps and acknowledgement checks complete while
the external probe remains inside `FindAll`. This rules out a stalled Dart frame/checkpoint loop
for that run, not native-provider or RPC failure. The run still fails, confirms owned process exit
and Job closure, and leaves no new Ame process record. Its raw output is retained in ignored local
evidence; the heartbeat is removed from the working tree after collection.

Independent review of the stage instrumentation also finds that a cache-disposal progress-write
exception can replace the preceding UIA read error. The shared cleanup owner now retains the exact
original exception while separately recording progress/disposal errors, and still disposes the
activation token once. Compiler-free regressions cover simultaneous read, progress, and disposal
failure plus original collection identity for zero, one, and multiple elements. The suite is wired
into the existing accessibility guardrail, passes in the complete guardrail run, and receives an
independent scope/collection-shape review. No UIA scope, deadline, or accepted phase is relaxed.

A temporary direct-children query before the unchanged subtree query allows one native run to
complete both tests and all ten UIA phases under the original deadline. This is a diagnostic
perturbation, not a product repair: the successful probe metrics were discarded by the parent,
so its child count and query timing were not retained. After removing that query exactly, the
original gate fails again at `application-ready`; its first test also reports no hit-testable
photo tile after a distant jump. That assertion did not record the scroll fraction or overlay
state, so it does not yet establish whether an undismissed menu or gallery layout caused the miss.

The evidence owner now returns identity-checked successful records only after process completion
and cleanup. The runner preserves them before acknowledgement without substituting metrics for
the original ordered phase contract. Compiler-free failure/identity tests, actual transcript
composition, and the full native process guardrails pass; independent review finds no additional
defect in that boundary. A subsequent original-query native run passes the virtual-gallery test
but still times out inside the populated-shell subtree query. Its successful activation metrics
are now preserved. Both recent failing runs confirm owned process exit and Job closure with no
cleanup failure; post-run snapshots contain only the historical zero-thread record.

The virtual-gallery test is also strengthened to check the actual four photo-menu actions and
popup route, then require their absence and route closure immediately after Escape. A missing
`MenuAnchor` alone never established those properties. Failure-only hit-test context records the
scroll fraction, mounted tiles, overlay counts, geometry, and focus without adding waits or changing
the production menu. The strengthened virtual-gallery test passes in two subsequent native runs
and receives an independent review. The test remains framework route/hit-test evidence, not a claim
that these four photo-menu items were individually traversed by the native probe.

Repeating the direct-children diagnostic does not reproduce its earlier success: two children return
in 413 milliseconds, but the subsequent original subtree query still exceeds the eight-second
deadline. A separate temporary 15-second parent window also fails inside that query. Both retain
the complete failure evidence and confirm owned-tree cleanup. The prequery and extended window are
removed exactly; neither is accepted as a fix or a relaxed gate. The diagnostic establishes that
some native queries respond, but not which part of full traversal is blocked.

## Native font-change lifetime reproduction

A separate hidden-HWND fixture compiles the production `FlutterWindow` and `Win32Window` handlers,
without starting Flutter or accessing a catalog. It sends a reentrant font-change message from real
`WM_CREATE` before controller initialization, or from destruction after controller retirement while
the HWND remains valid. Baseline `f9fab38` exits normally for the ordinary-message control, but both
desired-normal-exit font cases terminate with `0xc000041d`. The verified red record reads exit codes
from the owned Job's primary handle before disposal; an earlier diagnostic with unavailable
`Process.ExitCode` values is not counted as exit evidence. Dispatch and normal-return counters also
prevent a callback exception swallowed by Windows from producing a passing result.

The handler now checks the current controller and engine after plugin delegation. The same three
owned processes exit zero after repair, with confirmed process exit, Job closure, and no cleanup
errors. A separate CTest execution runs all three cases, none skipped, in 2.02 seconds. Independent
review finds no blocking issue in the handler or native fixture: normal initialized handling and
default processing remain unchanged by source inspection. The fixture does not dynamically exercise
an initialized engine or failed engine initialization. Matching a generic exception exit does not
attribute the earlier archived Flutter crash or the zero-thread process record to this fault.

The complete controlled Windows scan subsequently passes in 266,249 milliseconds, with all three native
interaction tests executed, exit zero, and no cleanup failures. Its generated fixture storage and
logs remain isolated and retained. The full accessibility query remains a separate open failure.
The public lifecycle gate then configures and compiles the production dispatcher, actually runs all
three CTest cases in 3.17 seconds, and accepts that invocation's fresh report. Independent script
review separately injects simultaneous execution and lock-release failures, finding that the public
facade's `finally` can replace the original failure. The corrected facade retains both errors,
preserves the original execution exception, and publishes success only after lock release. Four
real-facade AST execution cases prove ordinary, execution-only, unlock-only, and simultaneous-failure
outcomes, including exception identity and exactly one release. Independent review and Windows
PowerShell 5.1 execution pass; PowerShell 7 is not installed locally and is not claimed as tested.
The complete lint entrypoint subsequently passes all guardrails, formatting, all-target/all-feature
Clippy with warnings denied, and strict Dart analysis. The final public entrypoint is then run
again: configure/build succeed, all three native cases actually execute in 2.23 seconds, and the
fresh completion report passes validation before success is returned. Hosted verification of this
repair remains pending at this checkpoint.

After the font-change repair, another full original-scope accessibility run still fails at
`application-ready`. It passes the virtual-gallery test and native activation, then records one
application window and the `finding-elements` boundary at 1,674 milliseconds before the unchanged
eight-second parent timeout. Owned-process exit and Job closure are confirmed, with no cleanup
errors. This is further evidence that the reproduced font fault and accessibility timeout are
separate; the current marker does not yet distinguish the managed query from callback output
transfer or local cache-response construction.

## Real-engine scope-exit reproduction

The next isolated experiment uses production `FlutterWindow::OnCreate`, the pinned Debug engine,
and a compiled zero-dependency Dart `main` that performs no work. Plugin registration is only a
link stub; no Ame bootstrap, retained catalog, or source root is entered. A current-thread observer
records the actual parent notification from destruction of the verified `FLUTTERVIEW` child, without
injecting that message or overriding the window's lifecycle. COM remains initialized until the
whole window scope ends. A first-chance access-violation observer records evidence and always
continues exception search; success separately rejects an observed AV.

On `f58e483`, explicit `Destroy()` exits zero. Natural scope exit reaches the actual child-destroy
notification, then faults in `flutter_windows.dll` at RVA `0x1d7d0`, with `RCX = 0`, reading address
`0x10`; the owned primary process exits `0xc000041d`. The matching pinned symbol is
`FlutterWindowsView::GetEngine`. MSVC's member `unique_ptr` destructor still exposes the pointer
while invoking its deleter; its reset operation clears that pointer first. Calling the existing
`OnDestroy()` from the derived destructor body therefore removes the reentrant access to the
half-destroyed controller. The identical two cases then both exit zero, observe one real child
notification, retire both HWNDs, and report no AV. All four owned process trees confirm primary exit
and Job closure without cleanup errors. No new Ame or experiment process remains afterward.

The old report has the same module offset, which is strong correlation, not a recovered historical
stack. Its zero-thread, zero-handle process record remains present; neither that record's exact
lifetime nor its removal is claimed. The main entrypoint separately moves window/project lifetime
inside the successful COM scope, handles initialization failure without an unmatched uninitialization,
and treats `GetMessage` failure as failure rather than dispatching invalid message state. Independent
source review confirms the ownership order; formal native-gate integration and subsequent full-head
verification remain pending at this checkpoint.

The subsequent public native gate compiles and executes all five cases: the original three
engine-free cases pass in 3.16 seconds, and the two real-engine cases pass in 8.62 seconds. Both
engine cases observe exactly one natural child-destroy notification, complete the entire window
scope before COM release, retire both HWNDs, and reject any observed access violation. The parent
records exit zero, confirmed process exit and Job closure, and no cleanup failures. No new native
fixture or Ame process remains after execution. The fixture builds an offline no-op kernel in fresh
isolated storage from the already prepared pinned Debug SDK; it does not load Ame plugins or media.
Compiler-free checks additionally reproduce and repair command quoting for compiler paths containing
spaces: the same actual invocation now handles spaces and ampersands, preserves stdout and stderr,
and rejects nonzero exit status. Independent review covers this command boundary, strict result
admission, and original-error precedence. These passes close the formal exit regression, not the
remaining complete-head quality or client accessibility gates.

Hosted run `34114741707` on `f58e483` passes all native, Flutter, five synthetic, and unsigned Release
jobs, including the engine-free window gate. Static checks pass, but Rust reports 1322 passed,
one failed, and 19 ignored. The failing mixed-load priority test records P0 P95 1344 milliseconds
against its 1000-millisecond requirement, with 25 samples and three over one second. P1 completes
2048 candidates, P2 reads 10000 entries and stages its real 4095-entry page; both lanes progress
through every sample. Most slow-poll records spend their time outside the measured stages. This
does not yet prove a worker, SQLite-close, or scheduler cause. Connection retirement and same-sample
poll timing need direct evidence before a repair; replaying CI or relaxing the workload is not
accepted verification.

The first local retirement-instrumented priority run fails before collecting all 25 samples:
sample 3 leaves P1 at 192 completed while P2 advances from 384 to 512 reads. Its first three P0
samples are below one second, which does not establish the required P95. A focused regression
first validates the added failure-only queue SQL against the current schema. The next single run
then fails at sample 7: P0 is visible in 569 milliseconds, P1 remains at 448 completed with 64
leased items, and P2 advances from 896 to 1024 reads. There is no live worker; the uncancelled
`ame-p1-journal-drain` worker is still executing. Its leases have not expired, no retry is scheduled,
and runtime shutdown has not begun. This excludes waiting for live-worker retirement for that
failure, but does not distinguish media preparation from database publication. The workload and
five-second progress deadline remain unchanged. Neither local run supplies a complete P95 result
or repairs the separate hosted long-tail failure.

Stage instrumentation on the next single run identifies repeated preparation rather than write
admission as the immediate P1 delay. Preparing 64 paths takes 2562 milliseconds, followed by 265
milliseconds of source revalidation. The first transaction rejects a stale global catalog revision
before mutations; re-preparing and revalidating that same batch costs another 2603 milliseconds.
Both write admissions take less than one millisecond, and the eventual COMMIT takes 19 milliseconds.
The batch finishes at 6094 milliseconds, after the unchanged lower-lane progress assertion fails.
The live change is another path in the same P1 root, not a different root. This evidence requires
reuse only when the complete catalog read set and source versions remain valid; it does not justify
removing the global transaction revision guard. Independent source review also finds missing
pre-publication validation for unchanged locations and incorrect equivalence between a formerly
absent path and newly observed terminal media. These source-proof defects must be repaired before
prepared results can safely be reused. No complete P95 or repair result is claimed from this run.

A separate native accessibility diagnostic removes all six requested properties and retains only
the default runtime identity in the same whole-window subtree query. The current Debug application
build and framework interaction test pass, but the query still does not return within the unchanged
eight-second parent deadline. The boundary record is written immediately before `FindAll`; there
is no return record. The six property reads are therefore not a necessary condition for this
timeout. This does not yet separate native provider traversal, the MSAA/UIA bridge, or managed
response handling. Owned primary exit and Job closure both succeed with no cleanup errors. The
temporary diagnostic is removed from the production probe after retaining its failure output;
this experiment does not acknowledge or satisfy an accessibility phase.

The next experiment uses the same populated-application checkpoint and a separately compiled
system-only MSAA helper inside the existing probe Job. A PID/class/parent-validated `FLUTTERVIEW`
returns its `IAccessible`; bounded downward enumeration completes 119 objects and 118 edges to
depth 10, with no simple children. Interface and VARIANT release and COM teardown all return before
the helper reports completion; the complete probe takes 4209 milliseconds. It intentionally fails
the business phase instead of acknowledging it. This establishes responsiveness of that Flutter
fragment's downward MSAA traversal, not equivalence to whole-window UIA parent/sibling navigation
or successful screen-reader operation. Framework interaction passes; the owning application and
probe trees exit with no cleanup error. Both native process listings still contain only the old
zero-thread record afterward. The temporary probe branch is removed after preserving the trace.

A native COM comparison uses the system CUIAutomation client with the same raw cache properties
and required application elements. The first attempt expires while loading the managed client,
before invoking the helper, and cannot compare query behavior. A second attempt excludes that
managed load: COM initialization, automation creation, and desktop acquisition return within 157
milliseconds, but the desktop's immediate-child ProcessId-filtered search does not return before
the unchanged eight-second parent deadline. The application-subtree query has not started. This
narrows that run to native window discovery, not managed result conversion, and does not attribute
the earlier subtree timeout to the same cause. Both experiments retain failure evidence and close
their owned process trees without cleanup errors; neither acknowledges an accessibility phase.
The temporary production-probe branch is removed afterward. A read-only termination-signal check
and a separate limited-information query for the historical process both return access denied;
its actual termination state and external references remain unproved.

A direct-owned-window native COM comparison avoids desktop enumeration by passing the validated
application HWND to `ElementFromHandle`. COM creation and cache setup return in 78 milliseconds,
but that call does not return before the unchanged eight-second parent deadline; no subtree query
has started. The framework interaction still passes, and the owned application/probe processes exit
with Job closure and no cleanup errors. This is diagnostic evidence only.

The same native query helper then runs against a separate hidden standard Win32 window with three
system controls and a normal message pump. `ElementFromHandle` returns at 187 milliseconds and the
complete subtree query returns at 750 milliseconds, exposing five cached elements. All property
reads, interface releases, and COM teardown return by 765 milliseconds. The helper exits with the
expected missing-Ame-canary status; the fixture receives verified `WM_CLOSE` and exits normally.
The 6735-millisecond owned run confirms both descendant exits, primary exit, Job closure, and no
cleanup errors. This rules out universal query failure for this client configuration, not every
platform or Flutter failure mode. The Ame window/provider message path remains under investigation;
standard-control success does not satisfy the Ame accessibility gate.

Hosted run `34132450514` passes all required jobs on `c23a813`, including the new real-engine exit
gate, Static and Rust, Flutter, native interactions, all five synthetic workloads, and unsigned x64
verification. Protected signing remains outside ordinary PR authority. That head does not include
the following preparation repair, and hosted success does not settle the Windows 11 client query
failure or the historical process record.

## Prepared delta and source revalidation follow-up

The incremental worker now retains a bounded, complete catalog read set for each prepared batch.
Same-root unrelated live publication can reuse it only after matching every original path and
global identity observation, complete root context, and current source evidence. Negative results,
ordered catch-up lineage, and full preview state are included. Budget exhaustion falls back to
ordinary preparation; the old proof is dropped before a new one is retained. Source revalidation
now includes unchanged files and rejects an absent path that acquires terminal media. Final SQL
revision, root, lease, and preview checks and the two-rebase limit remain unchanged.

All 69 incremental tests pass, including 12 new controlled cases and the existing cancellation,
bounded-rebase, rename, corrupt-input, and replacement cases. The first-publication tests exercise
the actual prepare/revalidate boundary without a catalog revision conflict; they are not native UI
tests. Independent review finds no additional actionable defect in this slice. No retained catalog
or real source root is accessed.

The unchanged mixed-load test now completes all 25 measured samples with progress in both lower
lanes. Recorded P0 visibility is P50 581, P95 658, and maximum 660 milliseconds, with no sample over
one second; P1 eventually completes all 2048 candidates. The overall test still fails: after the
60-second post-measurement deadline, P2 has enumerated 9600 of 10000 entries and has not published
its default 4095-entry page. Its spool is still enumerating and its worker is active. These sample
statistics do not constitute a passing mixed-load gate. The remaining P2 convergence path requires
separate investigation; no workload or timeout was changed to obtain these results. Source review
confirms one retained Windows enumeration cursor, not repeated directory-prefix traversal. The
remaining work repeatedly yields after 128 raw entries in this fixture, retires its worker and
lease, and reopens sessions and transactions on the next poll. Remaining P1 work also defers new
P2 admission within the final deadline. Stage measurements are still needed to identify the dominant
cost; a running spool with no logical page is not proof of lost staging or publication failure.

The subsequent complete local lint invocation stops in the unchanged R2c-R process-timeout
guardrail before formatting, Clippy, or Dart analysis. Its 15-second owned parent records native
initialization complete at 3944 milliseconds and `before-child-start` at 4011 milliseconds, but no
child identity marker. This is an incomplete guardrail execution, not evidence that the expected
descendant cleanup assertion passed. Separate formatting (212 unchanged files), all-target/all-feature
Clippy with warnings denied, strict Dart analysis, all 14 asynchronous bridge contracts and matching
hashes, and whitespace checks pass. These do not erase the original full-gate failure. A fresh
process inventory finds no new Ame or fixture process;
the four Dart tools belong to the editor and are not terminated.

## Inventory admission and remaining closeout findings

The next unchanged mixed-load diagnostic again completes all 25 P0 samples and full P1 work, but
fails P2 convergence at 9344 of 10000 source entries. P0 sample P50/P95/maximum are 544/704/730
milliseconds; the complete test remains failed. Stage timing excludes the first 25 rounds because
their source-read gate deliberately waits for live visibility. In the ungated rounds, worker and
spool session opening average approximately 95 and 69 milliseconds. Existing-run restoration and
the two empty cleanup calls consume approximately 3.3 seconds across 49 rounds. Connection lifetime,
source namespace proof, lease transfer, workload, and deadlines are unchanged.

Two real-catalog regressions first reproduce unnecessary writer admission: resuming the same run
with its complete frontier, and cleanup containing only active staging, each advances the writer
epoch from 10 to 11 without a mutation. Root-replacement and strict terminal-expiry controls pass.
The lifecycle owner now returns matching run/frontier/root evidence from a consistent read snapshot
and skips empty cleanup without writer admission. Actual epoch allocation and deletion still
reselect inside the original writer transaction. Additional regressions preserve a caller's active
transaction when this standalone operation is rejected. Verification is tracked separately from
the mixed-load and raw-spool findings; this change does not establish complete cleanup correctness.

Hosted `f19e632` run `34137096253` passes every required component except Static and Rust: 1334 Rust
tests pass, one fails, and 19 explicitly gated cases remain ignored in that invocation. The mixed
fixture completes all 2048 P1 candidates and enumerates all 10000 P2 entries with 4095 staged entries,
but P0 P95 is 1.284 seconds. Samples 20, 22, and 24 exceed one second. Existing stage diagnostics
place their longest polls respectively in observation (1165 milliseconds) and resource retirement
(998 and 1120 milliseconds). Separate slow polls also expose catalog-open and retirement costs.
This is not evidence that the whole latency failure belongs to preparation, nor permission to
weaken its one-second requirement or return before resource ownership is safely retired.

A second Windows message diagnostic writes directly to a fresh isolated file after the original
stderr experiment produced no hook evidence. Both current-thread hooks report successful installation.
All 25 recorded `WM_GETOBJECT` entries have matching returns; the longest pair takes 47 milliseconds,
and the ten populated-application pairs take at most 16 milliseconds. The unchanged native gate
still times out in the application subtree query. No hook-uninstall or C++ scope-retirement marker
is recorded; outer primary exit, Job closure, and empty cleanup errors prove owned-tree cleanup,
not normal C++ destruction. The temporary production hookup is removed afterward. Object acquisition
and subsequent COM/parent navigation remain distinct unverified boundaries.

Separate read-only timing of the unchanged PowerShell safety audit records 3487 milliseconds for
native initialization, 9102 for the verified process-boundary audit, and 10 for held-snapshot
revalidation. Parsing the same source takes 63 milliseconds; its existing AST-budget walk takes
5325 milliseconds for 9791 nodes. This explains substantial preparation cost within the earlier
15-second process guard, not a passing child-start/cleanup assertion. No safety audit or timeout is
removed by this measurement. The node counter now checks the unchanged limit locally and invokes
the existing diagnostic helper only at the first rejected node. Accepted counts, cumulative source
budgets, rejection high-water, checked arithmetic, and source-identity audits remain unchanged.
Known empty/literal and cumulative-budget regressions pass; counting the updated 9812-node source
takes 601 milliseconds. The complete R2c-R guardrail subsequently passes all 19 cases with its
original process deadlines and cleanup assertions. Its exact protected-script digest is updated
for the reviewed extraction of the node-budget tests. The complete lint entrypoint then passes
its guardrails, formatting, all-target/all-feature Clippy, and Dart analysis on September 8.
Daily and final-head native/hosted acceptance remain separate obligations.

Independent raw-spool review identifies two further obligations. Terminal completion, termination,
and epoch replacement delete whole spools through cascading directory/entry foreign keys inside
one write transaction; the 4096-entry logical-staging cleanup limit does not bound that raw work.
Subtree initial entries also use a NULL directory component in their only composite foreign key,
so their relationship is not protected by the directory cascade or foreign-key check. These require
production-lifecycle regressions and a separate retirement/cleanup design. They must not be described
as fixed by empty-cleanup admission, nor assumed to explain all historical storage or responsiveness
reports without reproduction.

### Removed-root recovery authority

The production subtree-spool lifecycle fixture passes full catalog reopen before unregistering
its root. After the actual unregister transaction, reopen fails with
`catalog_recovery_execution_contract_unverifiable`: the queue is superseded, its inventory run
is absent, and its recovery authority still has no retirement timestamp. This is a reproduced
startup blocker, separate from the NULL raw-entry leak and the P0 Live latency requirement.

The root-removal transaction now retires that authority after root/run deletion and before commit.
The shared generation-retirement function is moved without semantic changes; retained roots used
by cancellation or namespace replacement cannot acquire removal authority. The compatibility
owner uses the same proof: an absent root and run, an inactive matching generation, no matching
publication namespace, and a matching P2 superseded queue with no lease, retry, or successor.
Only the retirement timestamp changes, never to a value before its authorization. Schema remains
v31. Structural validation precedes repair, and complete row validation precedes commit; unrelated
corruption or a failed update rolls back every repair.

All ten dedicated regressions pass on the current working diff: authorized/running/comparing
removal, other-root and completed-history preservation, clock rollback, idempotent full reopen,
eight missing-proof counterexamples, damaged schema, removal rollback, and compatibility rollback.
These controlled catalogs do not authorize a retained-catalog repair run or prove the separate
raw-spool cleanup, mixed-load performance, native UIA, or final-head gates.

The existing migration suite also passes all 97 tests. The separate lifecycle suite remains
27 passing and two failing raw-spool tests: the real unregister regression now passes full reopen
and source-byte preservation, then fails only its zero-orphan-row assertion. Independent review
finds no additional P0/P1 blocker in the removal/compatibility slice; it does not audit away those
explicitly unresolved cleanup failures.

### Current mixed-load measurements

The unchanged controlled workload still fails after the no-op inventory-admission repair.
Its 25 observed P0 samples have P50 561, P95 679, and maximum 707 milliseconds, with none over one
second. P1 completes all 2048 candidates. P2 reads all 10000 source entries through one opened
spool and reaches a ready spool, but stages no logical page before the unchanged 60-second
post-measurement deadline. Those sample statistics are derived from the failed run's individual
records, not a passing aggregate report or proof of complete recovery.

New observation measurements separate admission reads, observer polling, root-availability
inspection, and admission revalidation. The only substage above 100 milliseconds in this local
run is a 109-millisecond observer poll. Test-only catalog retirement decomposes the original
field-destruction order without deferring cleanup or retaining a connection. No connection-retirement
sample crosses 100 milliseconds here. Its connection measurement includes rusqlite cache/hook
cleanup and SQLite close, not just the FFI call. The hosted long-tail failure remains unclosed;
this local result cannot establish its cause, or an equal-environment regression comparison.

The failure log contains exactly 80 inventory starts and 79 finishes. Reading 10000 entries with
the existing 128-entry raw bound takes 79 rounds; even the EOF/ready commit returns `Yielded`.
The next round must read and stage the first logical page. The failure's staging count and later
spool snapshot are separate reads, so neither proves repeated ready-state livelock. The next
measurement correlates EOF commit, preparation readiness, logical-page return, and staging commit
against the unchanged deadline, with cancellation and worker-completion state captured before
fixture teardown. It fails earlier at 9984 source entries: the worker is still running, cancellation
is false, and EOF has not been reached. This excludes a logical-page commit stall as the cause of
that particular failure.

A subsequent test-only raw-step trace passes the same one-test gate, with P0 P95 597 milliseconds,
all 2048 P1 candidates, all 10000 P2 entries, and the required 4095-entry logical page. It commits
that page only 58.775 seconds into the unchanged 60-second post-measurement period. This narrow
margin, following two failures, is not stable convergence or a current-head CI pass.

| Observed post-measurement interval | Elapsed |
| --- | ---: |
| Deadline origin to next P2 source-step entry; log contains seven remaining P1 drains | 25.858 s |
| 54 source namespace checks | 3.826 s |
| 54 bounded directory-read steps | 0.178 s |
| 54 source-spool catalog-open calls, including validation | 4.308 s |
| 54 raw commits, including their writer admission | 2.967 s |
| 54 gaps from one raw commit to the next source-step entry | 20.851 s |
| First logical-page admission through commit | 0.506 s |

These are selected intervals, not a complete additive trace. Across the post-measurement recovery
writer-admission observations, P95 is 66.668 milliseconds and maximum 188.632 milliseconds. The
gate logs 187054 competing low-writer operations and retires normally. Its ordering places the
seven P1 drains before the next P2 source step, but does not separately time every P1 operation.
Raw reads are therefore not the dominant measured cost; continuation admission, catalog opening,
and runtime handoff require ownership-level investigation before changing scheduling. Independent
inspection confirms that the measured same-priority admission permits barging, but these timings
do not establish barging as the primary cause. Temporary per-step instrumentation is removed after
capture; the original workload, page sizes, priority requirements, and deadlines are unchanged.

### Same-priority writer retirement and fairness

A real-thread regression parks an older Recovery waiter immediately after its actual queue
registration, releases the active permit, and attempts a new nonblocking Recovery write. The
pre-fix implementation admits the newcomer: exactly one regression fails, with both owned threads
retired before the assertion. This proves overtaking independently of scheduler timing; it does
not prove the source of the hosted long-tail latency.

Writer admission now has a separate module and FIFO identities within each priority. Both blocking
and nonblocking acquisition preserve older same-priority registrations. Higher-priority work
still precedes Recovery, and an active permit is never revoked by a waiting request. Pending
registration is scope-owned so a timeout or preemption-callback unwind removes only that waiter
and wakes the next eligible request. No SQLite, source-namespace, batch-size, or deadline policy
changes accompany the extraction.

The initial admission-filter run passes 60 tests, including five new regressions. After adding the
same-priority middle-timeout case, the dedicated six-case module passes independently. These are
overlapping runs, not 66 distinct tests. Coverage includes deliberately reversed wakeup order,
nonblocking overtaking, higher-priority preference, timeout removal, neighboring waiter identity,
and both interactive and ordinary callback-unwind paths. Full current-diff lint, mixed-load,
native UIA, and final-head verification are not established by these focused results.
Independent review found two weaknesses in the new test evidence: the younger neighbor had not
yet registered when the alleged interior timeout occurred, and an unconditional join could hang
after the channel deadline. The corrected six-case run passes with all three actual registrations
present before middle retirement and bounded thread-completion confirmation before each join.
Epoch assertions distinguish waiting-request retirement from completed writes. The follow-up
review finds no remaining issue in that correction scope. Warnings-denied all-target/all-feature
Clippy also passes for the extracted production owner; this is not a whole-lifecycle audit.

The subsequent unchanged mixed-load test passes on this working diff: 25 samples, P0 P50 271
milliseconds, P95 323 milliseconds, maximum 333 milliseconds, and no sample over one second.
P1 completes all 2048 candidates, while P2 reads all 10000 raw entries and stages the required
4095-entry logical page. Both lanes advance in all 25 sample intervals. The competing writer
records 149401 operations; the test and its teardown finish normally in 149.19 seconds. This
single controlled pass does not attribute all improvement to FIFO, establish repeatability,
or replace the failed hosted head, full current-head gates, or original local UIA acceptance.
The complete repository lint subsequently exits successfully on the reviewed working diff,
including its guardrails, unchanged formatting, warnings-denied Clippy, and Dart analysis.
The complete Daily suite and current-head hosted verification have not been rerun for this slice.

An identity-only replacement for the raw source's fresh namespace binding is not applied. The
current namespace capability can include ACL-protected ancestors without retained handles, and
file-ID equality alone does not reproduce its component/reparse validation. Source protection
must not be weakened to reduce repeated-open cost.

### Nullable raw observations and terminal retention

The original three real-spool regressions now pass after run-bound initial observations are
explicitly deleted in the same transaction as their header. Previously the nullable composite
directory foreign key left the subtree's first observation behind after termination or root
removal. The shared retirement owner covers the three explicit runtime run retirements, root
unregistration, ordinary terminal queue cleanup, and exact legacy-terminal spool repair. Schema
31 remains unchanged; this is not the proposed delayed-retirement migration, and it does not
recover historical orphan entries whose header is already absent.

Expanded real-source coverage checks cancellation, explicit supersession, legacy repair, and a
failure injected after the initial observation has been deleted but before header deletion.
Rollback preserves the original run, raw rows, recovery ownership, source bytes, and full reopen.
The explicit supersession case is not evidence for the separate newer-epoch replacement entrypoint.

Following successful real subtree recovery into terminal queue pruning exposes another failure:
the current selector attempts to delete an event still referenced by a retained candidate owner
and receives a foreign-key error. The first eight-case run has seven passes and this one failure;
completion has already retired all raw observations before that error. The pre-existing selector
does not account for candidate or live-gap consumer references. Its extracted terminal-cleanup
owner now excludes those rows and unretired authority before LIMIT, preserving every existing
journal, successor, and scan-lineage condition. It must not erase the references to make deletion
succeed. The corrected real completion path preserves candidate ownership during early pruning,
then retires the run through its existing retention boundary and successfully prunes the released
events. A separate full-validity fixture proves that a consumed live-gap claim protects its older
control from deletion; a one-row cleanup first retires the eligible gap and then the released
control, so protected rows cannot occupy the entire batch limit.

Independent read-only review of the initial spool change finds no new P0/P1 in the exact owner
selection or rollback boundary, but does not establish all cascade entrypoints. Logical run
retention and persistent-journal queue pruning require separate reachability checks. The current
logical cleanup change preserves a still-owned spool instead of cascading through it. Neither
the row-delete helper nor the entry limit establishes bounded terminal work: the application
still drains cleanup until completion, and the header still cascades raw directory data. These
remain open alongside recovery of already unowned legacy rows, native UIA, and final-head gates.

The complete application inventory test module passes 55 tests with no failures or ignored cases
in 119.58 seconds. It includes eleven spool-lifecycle cases, the actual newer-epoch begin path,
and two independently imported source roots: retiring one root preserves the other root's raw
initial observation, run, authority, and source bytes through full reopen. The complete queue
test module passes 94 tests with no failures or ignored cases in 38.85 seconds. The complete
migration module passes 97 tests with no failures or ignored cases in 102.63 seconds. These counts
include their focused regressions and must not be added to overlapping earlier runs. The final
narrow read-only review finds no additional P0/P1 in the new retention selection, transaction,
or consistent `has_more` conditions. Full current-diff lint subsequently passes its compiler-free
guardrails, formatting, all-target/all-feature Clippy, and strict Dart analysis with exit code zero.
Complete Daily, hosted CI, and original local UIA acceptance remain separate obligations.

### Native parent-call diagnostic

The bounded MSAA experiment returns within the original eight-second probe deadline: 119 proxy
objects, 118 parent calls, interface releases, and COM teardown complete in a 5128-millisecond
probe. Root parent retrieval reports no parent. Every parent proxy has a different canonical COM
pointer from the traversal's earlier parent proxy, which is inconclusive rather than a proven
semantic-tree defect: Microsoft explicitly warns that interface pointers may differ for the same
UI element and directs clients to compare accessible properties instead of pointers.
See [Microsoft's interface-pointer guidance](https://learn.microsoft.com/en-us/windows/win32/winauto/getting-an-accessible-object-interface-pointer).

The experiment deliberately does not accept the application-ready phase. Complete UIA traversal
and local interaction acceptance remain open. The temporary probe hook is removed; the formal
probe and native entrypoint have no diff. Captured evidence confirms parent-process exit, owned
Job closure, no cleanup failures, and retained diagnostic logs; a subsequent process query finds
neither owned test process. The historical zero-thread/zero-handle record remains present and is
not relabelled as a running test or a cleared incident.

### Same-source verification checkpoint: 1ea769d

On 2026-09-08 the complete local Daily gate passes on source head
`1ea769d9fe0c7c295bf8fba8db85c548a1e920ee`, with an unchanged working tree. The Rust
main test binary reports 1371 passes, no failures, and 19 explicitly ignored cases in 1040.88
seconds; nested process-test summaries are not additional main-suite cases. All Flutter files,
controlled Windows scan, the original ten-phase native UIA sequence, and bridge checks pass.
The unchanged mixed-load test passes its 25-sample, one-second P95 and actual P1/P2 progress
requirements. Successful captured test output does not expose a new numerical percentile.

[Hosted run 34207127797](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34207127797)
tests PR merge `9eb3fb484a73829c185ad2dab201ef7d4eb5329a`, containing the same source head and
base `3da6c96f3356d23ca1a80803d8a3f0dd02e62b8c`; that base is already an ancestor of the source
head. Static/Rust, Flutter, Windows scan, all five synthetic workloads, and unsigned x64 verification
pass. The Rust main suite reports 1371 passes, no failures, and 19 ignored cases in 894.31 seconds.
The first accessibility attempt fails before window querying: its last verified progress is
`loading-uia-client` at 1777 milliseconds, followed by the unchanged eight-second parent timeout.
Its owned application and probe cleanup succeeds. One rerun of only that job passes all ten phases;
the application-ready probe completes in 2337 milliseconds. Preserve the initial failure: this
does not prove the cold assembly-loading incident is fixed. Protected release jobs remain outside
ordinary PR authority.

The original local whole-window UIA sequence also now passes all ten phases, with application
exit and Job closure verified. Read-only process and CIM checks no longer enumerate the historical
PID 17412; neither observation establishes why the earlier query stalled or the process record
persisted. This is new successful original-path evidence, not a root-cause or permanent-fix claim.
A subsequent isolated conversion diagnostic obtains the Flutter fragment's `IAccessible` and
receives `E_FAIL` (`0x80004005`) immediately from `ElementFromIAccessible`; conversion, interface
release, and COM teardown return by 16 milliseconds. No UIA element is returned, so neither planned
parent nor sibling navigation executes. The enclosing probe finishes in 704 milliseconds and
deliberately does not accept a business phase. The temporary probe hook is removed after verified
owned-process cleanup. [Microsoft documents conversion as a distinct, fallible API](https://learn.microsoft.com/en-us/windows/win32/api/uiautomationclient/nf-uiautomationclient-iuiautomation-elementfromiaccessible);
this result does not identify the earlier whole-window timeout's cause.

No retained catalog or real source root is used. Raw-spool cascade latency, the application-wide
cleanup drain, historical unowned raw observations, broader physical decomposition, and the
unattributed intermittent native/client incidents remain open beyond this verified slice.

### Stale source initialization after terminalization

The delayed-retirement audit first exposes a prerequisite in the existing schema-31 source
initializer. A controlled generated-root regression captures a production `Running` run, opens its
source, commits cancellation through `terminate_metadata_inventory`, then submits the old snapshot
to the production initializer. The original initializer succeeds and recreates one raw header and
one directory. A subsequent FULL open repairs those rows; the reproduction therefore does not
establish a startup failure or source-media loss. It establishes an unauthorized post-terminal
storage mutation that must be rejected at write admission rather than repaired on next startup.

Initialization and incomplete-directory reset now share a dedicated SQL owner. Inside its existing
Recovery write transaction, it re-reads immutable run identity and current status, revalidates the
root publication boundary, and checks both exact queue lease generation and matching unretired P2
authority. Terminal snapshots cannot recreate storage; released and subsequently reacquired leases
cannot reset another execution's incomplete prefix. A legitimate newer lease can still reset that
prefix while preserving completed parent observations. Lease expiry semantics remain owned by the
queue classifier; this change does not introduce a second wall-clock expiry policy.

The cancellation reproduction is red before the guard and green afterward. The focused spool
lifecycle suite passes 15 cases, including four new tests covering cancellation, failed/superseded
runs, released/reacquired leases with valid continuation, and immutable epoch mismatch. Generated
source bytes and FULL reopen are checked where applicable. These cases overlap the inventory and
full Rust suites and must not be added to their counts. The complete inventory application module
passes 59 cases in 72.44 seconds, including retained-source handoff and 10000-entry page reopening.
Independent review finds no actionable gap in this initializer extraction and guard; it neither
runs tests nor audits later raw APIs.

The complete local Daily gate then passes on fixed source head `64d16c5`, with the worktree clean
throughout verification. The main Rust suite reports 1375 passes, zero failures, and 19 unchanged
ignored cases in 990.33 seconds; its nested child summaries are not additional tests. The original
25-sample mixed-load P95 threshold and P1/P2 progress assertions pass, without a fresh numerical P95
in the successful default output. Complete lint, all Flutter files, controlled Windows scan, and
all 14 asynchronous bridge checks pass. The scan completes in 94535 milliseconds with no cleanup
failure and retains its generated evidence storage under the existing ownership policy.

The original full-window native accessibility gate passes all ten ordered phases without probe
overrides or reruns. Its application-ready query completes in 1549 milliseconds and traverses 126
elements; owned application exit and Job closure are confirmed, with no cleanup failures. A final
read-only process probe enumerates neither Ame nor Flutter tester, nor the historical PID. The
complete log is `build/r2c_interleaving_audit/daily_64d16c5_native_capture_20260908.log`; native detail
is retained beside it as `uia_64d16c5_20260908.log`. This successful same-source run is not attribution
of the historical native incidents, and hosted verification of the resulting PR head remains a
separate checkpoint.

Delayed physical retirement, stale retained-source write/read fencing, historical NULL-parent
orphans, and whole-cleanup draining remain open. Inspection also identifies correlated ordinal and
entry-count checks in the spool validator; a deterministic execution-step regression and a separate
query-owner change are required before calling that cost bounded. No schema, migration, source
format, UI, lease-handoff protocol, or performance threshold changes in this slice.

### Retained source execution across lease handoff

Two deterministic generated-root regressions exercise real source objects after returning and
reacquiring the same recovery queue entry. Before the execution fence, an old source appends a
second raw row (one becomes two), and another old source reads a ready spool successfully. Both
desired-behavior assertions fail. This proves unauthorized raw access, not an observed stale
gallery publication or source-media mutation; the downstream publication boundary is separate.

Raw access now carries an adapter-owned immutable run request and exact lease identity.
Initialization, raw writes, and raw reads share the current run/root/queue/authority proof.
Readiness, frontier selection, ordered rows, and run progress are read in an independently owned
consistent snapshot. Every write checks again inside its committing transaction. An active source
checks before advancing its iterator; its one catalog connection can span a bounded source batch,
but no database transaction or write permit spans filesystem I/O.

The application explicitly rebinds a retained source to the current lease. Successful rebinding
does not reinitialize storage or reopen the iterator; failed rebinding does not overwrite the
previous token. A source failure propagates through the existing attempt boundary and discards
that retained iterator before reconstruction. Queue expiry semantics, raw batch size, page size,
schema, and final publication requirements remain unchanged.

Coverage includes every raw directory API under an old lease, terminal/root-removal revocation,
legitimate continuation with exact source-read counts, and caller-transaction isolation. A
deterministic two-connection interleaving commits cancellation after read admission: the admitted
page retains its original rows and frontier, the next read fails, and logical staging of that old
page into the cancelled run is rejected. This does not promise revocation of an already admitted
read snapshot; it proves snapshot consistency and the separate staging boundary.

The first expanded inventory run reports 63 passes and one instrumentation assertion failure:
the new exact-read counter was not enabled, and its directory-open assertion initially referenced
the non-durable enumerator. The test now enables and reads the existing durable-source counters.
That failed test observation is not a demonstrated product re-enumeration defect. The fresh
20-case spool lifecycle suite passes in 18.05 seconds, and the separate caller-transaction case
passes. Complete current-diff lint then passes, including formatting, all-target/all-feature
Clippy with warnings denied, and the Dart analyzer. These focused counts overlap the subsequent
same-source Daily and must not be added together.

Independent read-only review of the execution token, raw API extraction, source adapter, and
application continuation finds no actionable defect in that boundary. It does not run builds,
replace the new regressions, or audit delayed retirement. The preceding `e253cbc` hosted run
`34218414939` passes all ordinary jobs and the aggregate without a rerun; its three protected
release jobs retain their existing skip conditions. That earlier head is not verification of this
new execution fence.

### Same-source verification checkpoint: 67971f3

The fixed source subsequently completes the local Daily sequence through its final bridge and
whitespace boundary. The main Rust suite reports 1381 passed, zero failed, and 19 ignored in
1052.07 seconds; two nested child summaries are not additional cases. The unchanged mixed-load
test passes locally, but successful default Rust output supplies no fresh numeric P95. Broker
binary integration passes three cases, and every Flutter test file passes. Controlled Windows
scan completes in 103951 milliseconds with exit zero and no cleanup failure; its generated
storage is retained under the existing evidence-only cleanup policy. The original full-window
UIA sequence passes all ten phases, with application-ready at 1419 milliseconds and 126 elements.
Native accessibility records exit zero, confirmed primary exit, closed owned Job, and successful
scratch removal. A post-run process query finds no Ame or Flutter tester process. Logs remain in
ignored `build/r2c_interleaving_audit/daily_67971f3_native_capture_20260908.log` and
`uia_67971f3_20260908.log`. This is not attribution of the historical client incidents.

Hosted run `34222730066` on the same source fails Static/Rust with 1380 passed, one failed, and
19 ignored. Its sole failure is the unchanged 25-sample mixed-load requirement: visible P95 is
1226 milliseconds, P50 112 milliseconds, maximum 1239 milliseconds, and three samples exceed
one second. P1 completes 2048 candidates; P2 consumes 10000 source entries and stages the required
4095-entry page. Both lanes progress in all 25 samples. Other ordinary jobs pass; the aggregate
fails and the three protected-release-only jobs keep their existing skip conditions. No rerun is
requested to replace this failure.

The three measured slow samples identify distinct poll boundaries: sample 15 takes 1116
milliseconds with 967 milliseconds in catalog opening; sample 20 takes 1226 milliseconds with
1092 milliseconds in observer polling; sample 22 takes 1239 milliseconds with 1129 milliseconds
in actual connection retirement. Longer diagnostic polls outside those samples are not assigned
to them. The exact failure is retained in
`build/r2c_interleaving_audit/ci_67971f3_mixed_load_102049431306.log`. The connection measurement
includes rusqlite cache/hook cleanup and SQLite close, not just the FFI call. It does not prove a
last-connection checkpoint while the fixture still retains other catalog connections.

The next diagnostic separates observer retirement, root reconciliation, observer start/drain,
pending-plan persistence, and queue metrics under the existing debug-only slow-stage threshold.
It changes neither invocation order nor retry, cadence, workload, lane progress, or acceptance
limits. These measurements are not a latency fix. Current-head performance closeout remains open
independently of the verified retained-source execution fence and pending raw retirement work.

The instrumented local mixed-load run passes in 123.05 seconds: 25 samples, P50/P95/maximum
173/199/204 milliseconds, no sample above one second, all 2048 P1 candidates completed, and the
10000-entry P2 source plus 4095-entry logical page completed. Both lanes progress in all samples.
No new 100-millisecond slow-stage record is emitted, so that run does not reproduce or explain
the hosted stalls. Six existing validated-session identity/schema/reuse regressions also pass.
A preceding mistyped harness option was rejected before any test ran and supplies no test result.
The test-only catalog timers additionally separate initial identity checks, open/configuration,
the validation/return tail, and statement-cache disposal from the remaining connection close.
Retirement still occurs inside the same measured poll; no timer moves work outside the gate.
Complete lint passes, including formatting, all-target/all-feature warnings-denied Clippy, and
the Dart analyzer. Narrow independent review finds no change to invocation count, destruction
order, error propagation, or measured poll ownership. Full new-source and hosted gates remain
separate; review does not establish a cause for the hosted latency.

Pinned SQLite source inspection confirms that a non-final WAL connection still closes its own
shared-memory handle and takes Windows VFS global/node mutexes. With the repository's existing
`SQLITE_ENABLE_SETLK_TIMEOUT` build flag, pending overlapped file-lock I/O also has a wait that is
not an interruptible busy-timeout wall-clock guarantee. These are candidate wait paths, not
identified stacks from the failed samples. The narrow independent investigation does not
attribute the failure to checkpointing, change timeout policy, or claim an application fix.

### 2026-09-08 diagnostic-source verification

Fixed source `d533cf917b4cbd47d30241a7a18e96644063d1b7` completes local Daily with
exit zero. Main Rust reports 1381 passed, zero failed, and 19 ignored in 1035.65 seconds;
the nested child summaries are not additional cases. Broker integration passes three cases
in 2.42 seconds, every Flutter test file passes, and the final 14 asynchronous bridge
contracts and whitespace gate pass. The unchanged mixed-load test passes, but default
successful output supplies no numeric P95 for this full run.

Controlled Windows scan reports passed in 96716 milliseconds, exit zero, and no cleanup
failure; its generated storage is retained under the existing evidence-only cleanup policy.
Original full-window UIA passes all ten phases, including application-ready with 126 elements
in 1422 milliseconds. Its completion proves primary process exit, owned Job closure, and
scratch removal. A post-Daily process query finds no Ame or Flutter tester process. Local
evidence is retained in ignored `build/r2c_interleaving_audit/` as
`daily_d533cf9_native_capture_20260908.log` and `uia_d533cf9_20260908.log`.

Hosted [run 34228475245](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/34228475245)
on that same source completes successfully. Static/Rust reports 1381 passed, zero failed,
and 19 ignored in 1243.23 seconds; its original mixed-load case passes without printing
a numeric P95. All ten ordinary component jobs pass. Three protected-release-only jobs
retain their existing skip conditions; they are not signed-release acceptance. No failed
run was rerun to obtain this result. These results establish this source's gate execution,
not a causal repair of the prior intermittent latency or historical client incidents.
Connection reuse has not been implemented, and retained raw-source retirement remains open.

### 2026-09-08 spool-row audit cost

The raw-retirement call-chain review found two independent obligations: immediate parent deletion
still cascades through the entire raw spool, and full-open row validation repeatedly counts earlier
directories and each directory's entries. Worker pause is lease return, not terminal retirement;
neither obligation permits discarding resumable source progress.

An in-memory fixture uses the production schema, two independent active roots/runs, and four raw
entries per completed directory. After correcting fixture-only SQL type, page-index, and active-root
constraints, the unchanged extracted validator passes three correctness tests but fails the work
regression: increasing each run from 64 to 256 directories raises bundled SQLite VM instructions
from 218671 to 3380527. These are operation counts, not user-library timing measurements.

The replacement uses per-run ordered row numbers and materialized per-run/per-directory counts.
The same sizes take 23540 and 93044 instructions; all four focused tests pass (3.06 seconds).
Coverage retains empty and incomplete directories, nullable initial observations, independent runs,
legacy/current status rules, ordinal gaps/fractions, compensating incorrect counts, retired owners,
and missing non-null parents. No schema, version, index, threshold, source-media access, or migration
repair policy changes. Independent read-only review found no actionable defect in this diff's SQL
equivalence, structural preconditions, or call boundary. It does not establish a fixed temporary-
storage bound or all skewed-data costs. Focused tests and this narrow review do not constitute
whole-project closeout.

The first Daily invocation stopped at Cargo's ordinary `Checking` stderr because the outer
PowerShell logging pipeline converted that output into `NativeCommandError`. It did not reach
Rust tests and is not a product-test result. Read-only process inspection confirmed no remaining
Cargo, Clippy, Rust, Dart, tester, or Ame process before restarting the unchanged gate with native
stream-preserving transcript capture. The original log remains retained separately.

Fixed-source `23e265a3b60bd240a1dab2b8107c4613508b8800` then completes the serial local Daily gate:
lint, 1385 main Rust tests (1099.06 seconds; zero failed, 19 existing dedicated cases ignored),
three broker tests, all Flutter files, controlled Windows scan, ten original whole-window UIA
phases, bridge contracts, and whitespace checks. Nested child-process test summaries are included
in the main result, not added to it. The unchanged 25-sample/P1/P2 mixed-load test passes without
printing a successful numeric P95. This does not explain the earlier hosted long tail.

Scan run `8284c7fd327e45cc90c51d5f3b43c394` finishes in 94290 milliseconds with exit zero and no
cleanup failures; its fixture remains under the existing retention policy. The native application-
ready probe traverses 126 elements in 1893 milliseconds. All ten UIA phases complete on first
attempt, with no run/cleanup error, confirmed process exit and Job closure, and owned scratch
removal. A subsequent read-only process inventory finds no Ame or Flutter tester process. The
three changed source files retain their pre-gate SHA-256 values throughout verification.

The ignored local evidence directory `build/r2c_interleaving_audit` retains the PowerShell
transcript, selected native output captured from the priority-test phase onward, and the complete
named UIA log. The selected native output is not represented as complete Daily stdout. Hosted run
`34236045583` completes successfully for the same source, with all ten ordinary jobs passing.
Static/Rust job `102093901251` reports 1385 passed, zero failed and 19 ignored in 1194.48 seconds,
plus three broker tests in 2.23 seconds. All four new cases and the unchanged mixed-load gate pass;
successful numeric P95 is not printed. Selected job evidence is retained separately. The three
protected-release-only skips do not establish signing or release acceptance. No hosted rerun is
used to replace a failure.

The related logical-frontier predecessor query remains unchanged; its page admission limits the
stack to 1025 entries. That distinct path and skewed-data resource costs are not covered by a claim
that all catalog validation is now linear or has a fixed memory budget.

Raw-data retirement, historical unowned observations, application-wide cleanup draining, and the
separately observed mixed-load latency remain open. Removing repeated row audits does not establish
a per-operation reclamation budget or explain the prior connection-close tail.

## Fixed closeout item 1 — retirement and bounded reclamation (locally verified)

This checkpoint follows the fixed queue recorded at `505bc260`. Item 1 meets its local exit after
the final-source evidence below; item 2 is now the only active implementation item. Historical
failed checkpoints remain recorded below. This is not hosted full-head verification or R2c
acceptance, and it does not close the queued mixed-load, full client matrix, or readiness decision.

The original controlled cancellation regression retained a two-entry raw spool before cancellation
and observed all its raw rows disappear in the cancellation transaction. Schema v32 now separates
physical raw storage from the run and recovery-authority foreign-key lifetime. Terminal operations
mark storage retired in the same transaction; children use restricted deletion, and retired headers
cannot become executable again. An authority can start a new epoch while its previous epoch awaits
reclamation. A follow-up regression explicitly opens and stages that successor before cleanup; it
first exposed the old full-authority UNIQUE constraint, which is now limited to active storage.

One cleanup attempt removes at most 4096 raw plus logical entries, separately at most 4096 expired
candidate-owner records, and at most 128 empty raw directories, raw headers, and terminal summaries
per category. Summary deletion excludes remaining candidate owners rather than cascading through
them. The seven-day summary/owner retention policy is unchanged. Historical NULL-parent observations
without a header are reclaimed without inventing a missing run, root, or execution authority.
Reopening an incomplete OS enumerator marks its directory `resetting` without deleting its prefix;
source preparation retires at most 128 old entries per turn before starting the replacement
enumerator. Completed directories and the initial subtree observation remain intact.

Cleanup uses non-blocking maintenance admission and a zero SQLite busy timeout for the attempt.
Foreground pressure interrupts its SQL, with the progress callback removed before rollback,
including unwind. A dedicated regression observes a real Live-lane waiter preempt maintenance,
then commit through a separate SQLite connection after cleanup rollback. The prior timeout and
later connection usability are checked. This is not a fixed wall-clock shutdown guarantee.

Application calls now perform one batch and preserve `has_more`; they do not drain the complete
catalog. The production runtime owns subsequent batches independently of configured roots, with
idle rechecks, bounded contention backoff, and the existing absolute stop deadline. Real production
poll tests, not mocked cleanup callbacks, establish that 4097 historical orphan entries leave one
after the first poll and resume after runtime reconstruction. A separate 129-header/directory case
leaves one of each after the first batch and reaches idle on the next. These fixtures have no roots,
scans, queue records, recovery authority, or source-watcher starts.

Focused evidence collected during this checkpoint:

- The 105-test migration suite passes after integrating v32 with existing terminal and live-gap
  compatibility repair. Subsequent active-authority uniqueness and retired-binding refinements are
  covered by the successor lifecycle regression; final-source migration gates remain required.
- The `metadata_inventory::tests` filter executes 74 tests, all passing, including controlled
  10000-entry enumeration, cancellation, restart, stale execution, root replacement, source-byte
  preservation, and the 257-candidate-owner batch limit. Its earlier new-fixture failure used an
  invalid `upsert` intent; correcting it to the existing `reconcile` contract did not change product
  policy or relax a constraint.
- The `bounded_cleanup` filter executes 15 passing tests at its earlier checkpoint; this overlaps
  the suites above and is not an additional aggregate acceptance count. The later actual Live
  writer commit regression executes separately and passes.
- All-target/all-feature Clippy with warnings denied passes after removing three redundant borrows.
  Scoped Rust formatting and tracked whitespace validation pass. Later test additions still require
  the final applicable format/lint gate.

The v31-to-v32 schema copy is a one-time migration cost, not runtime bounded reclamation. It retains
legitimate ready/incomplete raw observations and source-revision tokens exactly, preserves historical
orphans, rejects damaged DDL, and restores old schema, rows, and version after an injected final
version-write failure. Source media is not read or modified by those SQL-only migration fixtures.
Independent item review, resource-boundary review, and the applicable final-source gates remain
open; no real-library acceptance, native UI result, hosted result, or release claim is inferred.

### Item 1 bounded-work review — 2026-09-09

The independent review identified three P2 defects blocking this item's exit, not new product scope:

- Raw-entry JOIN ordering sorted the remaining run before applying the delete limit. A pinned
  rusqlite VM-step regression reproduced 13626 versus 106810 instructions for 1024 versus 8192
  retained entries with a one-entry batch. Selection now bounds the retired owner set before
  indexed entry paging, and bounds inspected directories/headers before testing emptiness.
- A nullable retired header identity passed row validation and could never match its final
  deletion. The unreleased v32 schema now requires a 1–256-character non-null identity. Exact row
  proof rejects NULL, empty, oversized, and non-text stored identities even under matching DDL.
  Three native regression tests cover insert boundaries, FULL reopen rejection without mutation,
  and atomic rejection of a damaged v31 migration.
- The empty-debt hint chose a full raw index rather than the initial-observation partial index.
  The existing two-root raw fixture reproduced 6174/49182 VM instructions with 1024/8192 active
  entries per root and no cleanup debt. Explicit selection of the schema-owned partial index plus
  retired-header short-circuiting now takes 26/26 instructions. The raw row contract is validated
  before measurement. A final narrow independent review confirms this finding is resolved without
  reopening unchanged paths; the other newly reviewed batch/cancellation paths have no remaining
  reported blocker. This is not a whole-product audit verdict.

Five pinned SQL-work regressions pass. For 1024 versus 8192 retained objects and the same one-row
batch, VM instruction counts are respectively 343/343 (raw entries), 385/385 (nonempty directories),
426/426 (retired headers), 991/991 (logical entries), and 1141/1141 (candidate owners). The last two
execute the complete transaction SQL, including remaining-work detection, with real parent/FK
graphs and FULL reopen before and after. These measurements prove these batch shapes, not a
universal wall-clock latency bound or the queued mixed-load causal correction.

Runtime stop now supplies a typed read-only cancellation control to the batch port. Its
pre-admission read and write progress handlers observe cancellation; short statements are checked
again before commit. Four focused native tests pass: pre-cancelled admission, read interruption,
actual DELETE interruption after its first mutation with full rollback, and cancellation after
short SQL before commit. Timeout restoration, handler retirement, and subsequent connection use
are checked. Catalog opening and the existing absolute worker-retirement deadline remain separate
boundaries; these tests do not claim that every native call is forcibly interruptible.

The updated `bounded_cleanup` checkpoint executed 17 passing tests before the latest four
cancellation tests; these overlap earlier suites and are not aggregate acceptance counts. Final
scoped formatting, Cargo formatting check, and all-target/all-feature Clippy with warnings denied
pass at that checkpoint. The final empty-debt hint then received its own passing native regression.
Complete final-source Daily and hosted evidence are still required. An initial Daily invocation
stopped at the shell capture boundary: merging native stderr into a Stop-policy PowerShell pipeline
misclassified Cargo's successful completion message as `NativeCommandError`. It did not execute
the complete Rust suite. Its log is retained separately; the plain canonical Daily invocation is
the replacement, with no quality-script or product-policy change.

That canonical Daily run passes lint but stops at the main Rust suite: 1400 passed, 20 failed,
19 ignored in 601.32 seconds. Flutter tests and native integrations have not run at this checkpoint.
The failed stream is retained at `build/r2c_interleaving_audit/daily_item1_native_tests_20260909.log`.
Fifteen failures share a duplicate test-only v30 downgrade helper which skipped schema 32, leaving
v31 columns behind a v29/v26 marker. Removing that duplicate and using the canonical migration
fixture restores the first v26 reopen regression and seven explicit-recovery tests; production
schema validation is unchanged. The other five failures expose old current-version or immediate
physical-spool-deletion expectations. Their updates must retain exact terminal authority and
eventual bounded reclamation evidence before the complete gate is retried.

The exact 20-test rerun passes 19 and fails the 4096-file production test at its newly added FULL
reopen, after recovery and shutdown succeed. A separate diagnostic reproduction records a completed
baseline at journal 44, closing USN 20, and the legitimately advanced current checkpoint at USN 21
with matching volume and root identities. The baseline row validator incorrectly requires exact
cursor equality forever. This is a production startup risk, not an immediate-deletion assertion or
reason to freeze the simulated journal. It blocks item 1's production completion/reopen exit.
The diagnostic print has been removed; failure evidence is retained. The correction must preserve
retired authority, namespace/journal identity, monotonic coverage, and active recovery validation.

The first corrected validator passes its 13 focused cases but still fails the original production
reopen. Bounded failure evidence shows the exact checkpointed, completed range 20–21 was enrolled
before the P2 final receipt. Requiring range enrollment after baseline completion adds an invalid
ordering between independent P1 and P2 publication. Removing that requirement preserves exact range,
namespace, coverage, and lifecycle proof; a dedicated regression retains this interleaving. Both
failed production reproductions remain evidence, not successful verification.

The unchanged Dart source passes the canonical Daily Flutter partition: 70 files and 552 tests,
exit 0. No Ame or Flutter tester process remains at the post-partition check. At that checkpoint,
final-source Rust and native Windows evidence was still pending; it did not advance another item.

The revised 14-case baseline suite and original 4096-file production test pass together: 15 passed
in 72.92 seconds, including the later final-receipt interleaving and FULL reopen. Independent review
then identifies a remaining P2 at terminal queue pruning: a consumed live-gap claim can retain an
older completed baseline while the same candidate batch deletes its gap and a later journal-reset
baseline. The remaining old journal can no longer explain the current checkpoint. The initial
SQL-only reproduction uses SQLite 3.51.2 and is not pinned/native acceptance. Its owning correction
must preserve per-root/generation baseline-prefix deletion and prove each batch through the pinned
catalog API and FULL reopen; the validator's journal identity proof must not be weakened.

The pinned native regression now reproduces that exact candidate error without the prefix guard:
the first batch deletes three records instead of the two permitted records. With the guard restored,
the same test passes in 1.30 seconds and checks history `[901,902] -> [902] -> []`, FULL reopen after
each batch, foreign-key integrity, and other-root active and unexpired records. The final scoped
independent review closes this P2 with no direct omission; it is not a second whole-product audit.
The canonical Daily Static partition passes against the final Rust source: 1435 main-library tests
pass with zero failures, 19 explicit specialized/worker-entry ignores, and zero filtered tests in
662.48 seconds; broker-binary integration passes 3/3. Lint, formatting, warnings-denied Clippy,
fatal-warning/info Dart analysis, 14 asynchronous bridge contracts, and tracked whitespace checks
also pass. A prior attempt stopped at the owned test import order; the formatter corrected that
order before this complete rerun. Both streams are retained separately. This Static result does
not close the mixed-load historical causal investigation.

The final native Daily scan partition passes 3/3 in 64965 milliseconds, covering retained-import
manual waiting and durable cancellation, real picker cancellation, and controlled-folder import.
Its completion receipt reports no run or cleanup failure and retains isolated storage as evidence.
The final native accessibility partition passes 2/2 with all ten ordered UIA phases, no rejected
AXTree update, and explicit primary-process exit and owned-Job closure. The post-run process check
finds no Ame or Flutter tester. Together with Static and the unchanged-source Flutter partition,
all four canonical Daily partitions pass. Only controlled test media was accessed; retained
libraries were not accessed and no source mutation or cloud hydration was authorized or performed.

Item 1's bounded retirement, migration/reopen, stale-execution, rollback/cancellation, eventual
rootless cleanup, and other-root/Live progress obligations now have focused and complete local gate
evidence. The independent bounded-work review and final baseline-prefix finding are closed. This
permits the fixed queue to advance to item 2, not a claim of zero defects or whole-product readiness.
The original mixed-load failure still requires a causal account; the accumulated client workflow
matrix, historical native attribution, final hosted checks, and external acceptance remain open.

Physical measurements at the first formatted checkpoint: the catalog facade is 5355 lines, inventory
adapter 3173, inventory application 1519; the migration owner remains 15509 (10243 before inline
tests, 5266 inline-test region), and synchronization production remains 14626 (4117/10509).
They remain physical decomposition debt. New production owners are the v32 migration (250), raw
cleanup (82), reset (68), candidate cleanup (36), cleanup transaction (139), runtime cleanup (199),
and cancellation port (20). Application cleanup is 72 lines, with 43 before its inline tests and
29 in that test region. New dedicated suites and fixtures have separate ownership: transaction
tests 245+131, SQL-work tests 237+133, migration tests/fixtures 168+149+80, runtime tests 207+259,
and application bounded cleanup/reset/retirement 138+98+64. These sizes describe checkpoint files,
not newly added SLOC; no broader large-file cleanup is claimed.

After the completed-baseline proof extraction, `migrations.rs` has 15189 lines: 9923 before the
inline-test module and 5266 in that module. The extracted validator has 435 non-inline lines;
its dedicated tests have 429 lines plus 222 for terminal-pruning lifecycle coverage. Queue terminal
cleanup has 129 non-inline lines. The production synchronization owner has 14727 lines, split
4117/10610; its bounded failure-only reopen evidence is test code, not a new runtime policy.
These final measurements replace the earlier checkpoint for those owners only.

## Fixed closeout item 2 — mixed-load latency (active)

The item-1 source `150cd64` subsequently passes hosted run `34259927908`, including its ten
ordinary workers and the aggregate Windows gate. The separate signing/release workflow jobs remain
intentionally skipped. This verifies that source only, not the following item-2 correction.

On the same workstation, the unchanged original mixed-load fixture at `150cd64` records all 25
samples with P50 71 ms, P95 90 ms, and maximum 97 ms. It completes all 2048 P1 candidates, reads
10000 P2 source entries, and stages the required 4095-entry logical page. The retained baseline
is `build/r2c_interleaving_audit/item2_baseline_150cd64_20260909.log`. It does not reproduce the
historical 1226 ms P95 failure and is not evidence that the historical cause disappeared.

The serialized poll previously opened and destroyed its SQLite connection on every call. The
new `PollCatalogOwner` retains exactly one exclusive connection, with RAII checkout/return,
process-session renewal, and the original bounded identity/WAL/header/schema-cookie proof.
Transactions, busy statements, and staged publication buffers reject reuse without cleanup that
could conceal ownership. The close belongs to the existing journal-close worker and original stop
deadline; it must finish before Empty is published. Failed thread creation leaves ownership in
the runtime. This removes the repeated operations implicated by the historical 967 ms open and
1129 ms retirement samples, rather than moving one close per poll outside the measurement.
The new native poll diagnostic explicitly names the final step `checkout_return_ms`, not
`retirement_ms`; actual connection destruction remains measured separately at owned stop in tests.

The first reuse candidate records P50 53 ms, P95 68 ms, and maximum 70 ms on the original fixture,
with unchanged sample count, candidates, source entries, page size, deadlines, and both lower lanes
progressing in all 25 samples. Its retained log is
`build/r2c_interleaving_audit/item2_reuse_first_20260909.log`. Neither current run contains a slow
observation substage. These timings alone cannot attribute the historical 1092 ms observer sample;
that attribution remains required for item 2's exit. The same-workload lifetime comparison below
separately verifies the removed connection operations.

Focused evidence includes 11 adapter tests for proof drift, dirty connections, diagnostic seam
order, and maintenance while the idle handle remains alive; real WAL truncation, incremental
reclamation, and legacy VACUUM all run without first closing that handle. Production's original
100-poll regression now also observes exactly one connection open, no close during polling, and
one close at completed stop. Owner/runtime tests cover error/unwind return, duplicate checkout,
maintenance proof revocation without a header change, Windows replacement rejection while held,
replacement after retirement, close failure, and close timeout retaining the Draining epoch.

Independent review identified an install-error path that lost the epoch's catalog binding after
the old connection was retired. A deterministic negative test first fails when an invalidated
proof is replaced, the controlled fixture schema changes during retirement, and the failed open
is followed by a different catalog request. The correction keeps the epoch path in the same
transition owner independently of its connection state. The regression and all eight owner/runtime
tests then pass; the bounded review confirms that finding closed. The initial test run also caught
a fixture that requested a missing replacement file instead of creating a replacement database;
only that fixture setup was corrected. All-target/all-feature Clippy with warnings denied passes.

The complete production synchronization module passes 111/111 in 125.32 seconds, including the
unchanged original mixed-load test, the 100-poll open/close assertions, 100 no-change startups,
real bounded P2 work, epoch races, panic handling, and stop/restart cases. Its log is
`build/r2c_interleaving_audit/item2_production_first_20260909.log`. The 16 process-session and
maintenance regressions also pass. These are affected-scope checks, not a replacement for item 4's
final-source full gates or item 3's client workflow acceptance.

Physical ownership at this correction: the poll owner is 240 lines with no inline test module;
its dedicated owner/runtime suites are 127 and 74 lines. Adapter revalidation is 72 non-inline
lines with dedicated proof and maintenance suites of 247 and 132 lines. The catalog facade falls
from 5355 to 5334 lines. Synchronization production is 14728 lines (4105 before its inline test
module and 10623 in that module); the existing larger physical debt is not disguised or expanded
with a new unrelated responsibility.

This remains an internal correction within item 2, not completion of item 2 or R2c. No new workflow
or unrelated refactor is admitted, and no retained library, source media, or cloud placeholder is
accessed. No schema, dependency, generated bridge, or presentation contract changes are introduced.

### Same-workload connection-lifetime control

The follow-up changes only the test fixture and its comparison owner. Both arms execute the same
production poll and original workload generator: 25 P0 samples, 2048 P1 candidates, 10000 P2 entries,
the 4095-entry logical page, competing lower-priority writes, and the original deadlines. The
per-poll control retires its actual connection before the outer poll/event-to-visible stopwatch
returns. The per-epoch arm uses the production lifetime unchanged. Counters observe real opens and
destruction, not checkout counts. Fixture shutdown must also observe the per-epoch connection's
single eventual close. The removed policy is not an alternative product acceptance path; both the
original test and the current per-epoch arm retain P0 P95 at most one second.

The serial workstation comparison on the `3d43a4b` product source passes in 108.14 seconds:

| Lifetime | Polls | Opens during polls | Closes during polls | P0 P50 / P95 / maximum |
| --- | ---: | ---: | ---: | --- |
| Per poll, test-only control | 1556 | 1556 | 1556 | 72 / 101 / 103 ms |
| Per epoch, production | 3671 | 1 | 0 | 61 / 79 / 80 ms |

The control spends 945 ms in cumulative poll retirement; production has no poll retirement and
closes once at completed stop. Both arms complete all P1 candidates, read all P2 source entries,
publish the required logical page, and record both lower lanes active and progressing in all 25
samples. Their poll counts differ because they drive the same gated work at different call costs;
these are not matched-count microbenchmarks or a universal speedup claim. The retained output is
`build/r2c_interleaving_audit/item2_lifetime_control_20260909.log`.

Neither arm reproduces the historical 1092 ms observation sample. Its original source lacked the
later inner-observation measurements, so the old outer duration cannot identify a specific SQL,
filesystem, or observer operation. Source inspection of the existing Windows SQLite lock and close
paths supplies hypotheses, not an execution trace or an established dependency defect; no compiler
flag, driver, timeout, or scheduling policy is changed on that basis.

The original production test separately passes after the fixture extraction in 51.63 seconds,
with P0 P95 61 ms and unchanged work/progress assertions. All six fixture failure/unwind regressions
and all-target/all-feature Clippy with warnings denied pass. A narrow independent read-only review
finds no actionable issue in comparison fidelity, timing boundaries, connection counters, or
fixture cleanup. This is not another full product audit or evidence of historical attribution.
The comparison is a non-ignored Rust test already included in the existing Static and Rust gate;
no duplicate benchmark workflow or relaxed acceptance path is introduced. The shared fixture is
983 dedicated-test lines, and the separate comparison owner is 103 dedicated-test lines; neither
adds production behavior or a second workload implementation.

### Observer work boundary and constrained-scheduling probe

A detached `d533cf9` experiment retains the historical behavior plus its existing observation
timers. Limiting only the test process to logical-CPU affinity mask 3 is verified on the running
process. The unchanged original fixture passes in 75.84 seconds, with P0 P95 234 ms and maximum
264 ms; no slow observation substage appears. A freshly rebuilt `8f4b56b` test launched under the
same affinity restriction passes in 71.24 seconds, P95 167 ms and maximum 171 ms. Both preserve the
25 samples, P1/P2 work, logical page, and deadlines. Logs remain in the ignored evidence directory
as `item2_observer_d533_two_cpu_20260909.log` and `item2_observer_8f4_two_cpu_20260909.log`.
The `8f4b56b` source also completes hosted run `34267352705` successfully; that result does not
verify the following observer-metrics correction. This constrained scheduling experiment does not
reproduce the old sample or establish CPU contention as its cause; no further repetition of that
experiment is used as a repair.

The experiment also rejects an invalid current-source check: sharing a compiled target directory
between worktrees let Cargo report a fresh test executable that still listed the historical 1400
tests. That no-run result was not accepted as current evidence. The named package's rebuildable
output was removed, current source rebuilt, and the 1474-test list plus new lifetime-control case
verified before the current-source experiment. No source media or catalog was cleared.

Separate production-port counterexamples establish a real work-boundary defect in `observer_poll`:
`load_library_change_root_queue_metrics` filters its results correctly but reads unrelated roots
and generations. Its shared nullable-OR predicate affects the outer aggregate and both scalar
subqueries. This blocks the active observer-work obligation and is addressed within item 2, not
used as retrospective attribution of the historical 1092 ms sample.

With target-root data fixed, growing unrelated rows from 256 to 4096 gives these SQLite VM counts:

| Unrelated retained data | Before, 256 / 4096 | After, 256 / 4096 |
| --- | ---: | ---: |
| Other-root terminal history | 1200 / 16560 | 211 / 211 |
| Same-root previous generation | 1712 / 24752 | 209 / 209 |
| Newer other-root exhausted failures | 2773 / 41173 | 245 / 245 |
| Other-root explicit recovery claims | 4783 / 73903 | 211 / 211 |

All four regressions fail before the correction and pass afterward. Each also tests an empty
target root after correction, with 161 / 161 steps, and verifies that global statistics still
include unrelated data. Latest-failure coverage inserts the other-root failures after the target
failure so a reverse global rowid walk cannot hide behind an early match.

The extracted metrics owner supplies static root/generation predicates without interpolating
identifiers or user values. The root latest-failure query sorts an integer cast of its integer
rowid to avoid a cross-root reverse-rowid walk without a hard dependency on a named index, and
explicit claims are checked by unique gap reference from the scoped queue. Global behavior and
all count, failure, and capacity-deferral projections are preserved. No new schema, index,
dependency, retry policy, deadline, or synchronization lane is added; a large target root still
requires work proportional to its own retained records.

Independent review first identifies an unsafe new hard index dependency and an incomplete test
generation transition. A FULL-accepted fixture with a missing eligible index reproduces the
candidate query failure. Removing the hard dependency preserves readable results for both missing
and same-name wrong-definition indexes, with 145 and 150 VM steps respectively for an empty target.
The fixture now uses the existing generation transaction to retire and create journal authority,
then completes a FULL reopen before every measurement. Repeating both old-query RED and final-query
GREEN on these valid fixtures produces the table above. Five focused tests pass in 6.95 seconds;
the earlier incomplete-fixture counters are not substituted for this complete evidence. The final
valid-fixture logs are `item2_root_metrics_valid_red_20260909.log` and
`item2_root_metrics_valid_green_20260909.log` in the ignored evidence directory. The final narrow
review confirms both findings closed without a new schema dependency or projection-semantic change.

Final-source verification passes all 105 queue tests in 38.15 seconds and all 112 production
synchronization tests in 267.92 seconds, serially with no ignored tests in either selected suite.
The latter includes the unchanged original mixed-load latency gate and connection-lifetime
comparison. All-target, all-feature Clippy with warnings denied passes in 22.88 seconds. These
results supersede intermediate candidate-query runs; they do not close historical observation
attribution or the remaining item-2 exit obligations.

The new production metrics owner is 227 lines without inline tests; its dedicated work-boundary
suite is 270 lines. Persistence falls from 1689 to 1486 lines; the queue facade is 2668 lines and its
existing dedicated test facade is 7039 lines. Those larger owners remain physical debt, not new
extension points for unrelated work.

### Complete mixed-load recovery verification

The production P0 latency case now continues its existing runtime beyond the first P2 logical
page. Its 25 measured samples, P95 limit, 2048 P1 candidates, 10000 raw entries, 4095-entry page,
per-sample limits, 60-second first-page deadline, and two-second stop deadline remain unchanged.
The two connection-lifetime control arms still end at that original page boundary. Only the
production gate adds the complete-recovery tail, using a creation-to-reopen budget of 300 seconds
with the same clock origin as the existing 4096-file complete-recovery fixture. This new tail is
not included in event-to-visible latency samples and does not renew any earlier deadline.

Completion evidence follows the original returned recovery control ID through its exact run,
authority, baseline, checkpoint, root state, and candidate owners. All 10000 owners must complete,
the baseline must publish, execution authority must retire, and the production root snapshot must
be synchronized. After owned stop, the full-schema counter must prove a new FULL validation.
Bounded 256-path windows then verify every terminal-media location and matching evidence, retain
the 25 prior visible P0 results, and compare all controlled P2 source bytes. The P2 payload remains
the original malformed JPEG fixture, not 10000 successfully decoded images. Retired raw storage
may remain for its existing cleanup owner but cannot still be executable.

An intermediate run with the original journal simulator completes in 177.97 seconds, with P0 P95
71 ms and a 116061 ms completion tail. That simulator gives two roots on the same volume different
journal ends, allowing P2 to close an empty window despite P1's new records. This is a fixture
fidelity gap, not evidence of a production failure. The intermediate log remains at
`build/r2c_interleaving_audit/item2_completion_empty_window_intermediate_20260909.log`; it does not
verify the subsequent shared-volume correction, nonempty closing-window assertion, or final
source.

The journal simulator is now one shared test owner for all three existing consumers. Both roots
report the same published volume end, and an empty P2 candidate result covers that requested
nonempty interval. Focused contracts also retain P1's page size, cursor, candidate paths, and
read/close counts. The final complete-recovery assertion requires closing USN greater than opening
USN, so a return to the old empty-window shortcut cannot pass. Narrow independent review reports
no blocker in the exact-control evidence, same-workload boundaries, deadline preservation, or
fixture retirement; it does not attribute the historical observation sample.

The corrected final source passes both new journal contracts, then all 114 production
synchronization tests in 412.17 seconds with zero failures or ignored tests. This includes the
complete production P0 case, both original first-page lifetime arms, all six extracted fixture
failure/unwind cases, and both additional legacy-capacity users of the shared journal simulator.
The complete case proves the nonempty closing window, synchronized snapshot, FULL validation,
persisted per-path terminal evidence, unchanged source bytes, and retention of prior P0 results.
Its original one-second P95 assertion remains active; this ordinary captured suite result does
not supply a fresh numerical P95 for a benchmark table. These are complete controlled-recovery
checks, not retained-library, UI, or historical latency attribution evidence.
All-target, all-feature Clippy with warnings denied also passes in 18.33 seconds. The captured
test/Clippy stream is retained as
`build/r2c_interleaving_audit/item2_completion_final_gates_20260909.log`; only its initial
compilation-only chunk is omitted.

The priority workload owner falls from 983 to 782 dedicated-test lines. Its unchanged retirement
owner and six failure/unwind tests are now 238 lines; full recovery verification owns 233 lines,
the shared journal fixture and its tests 291, and lifetime comparison 109. The production facade
falls from 14728 to 14584 lines, with 4105 before its inline test module and 10479 in that region;
its production behavior is unchanged by these extractions. These counts retain the larger
facade's physical debt rather than claiming repository-wide decomposition.

### Reserved observer ingress under a held writer — 2026-09-09

This is another causal check within fixed closeout item 2, not a new delivery stage. The original
`observer_poll=1092ms` record has no retained substage trace or matching Static/Rust profiling
artifact. Its original source already measured root availability; a missing availability timer
does not explain that record. Neither subsequent green runs nor this counterexample identify its
unique historical cause.

The current observer calls durable enqueue on the serialized poll thread. That path previously
joined the shared writer admission's unbounded condition-variable wait before SQLite's separate
five-second busy timeout. A real temporary-catalog Recovery transaction, actual observer plan,
and registration signal reproduce the problem independently of machine speed: the poll cannot
return during the held writer's 100ms control window. The failing run records
`queue_persistence=108ms`; the writer and observer are released and joined before the assertion.
Its RED output is retained in
`build/r2c_interleaving_audit/item2_observer_held_writer_red_20260909.log`.

The correction adds an opaque ingress reservation through a narrow persistence port. Queue
position survives deferred attempts; transaction and active permit do not. Priority notification
runs outside the mutex, registered same-priority order remains FIFO, and a queued Recovery relay
cannot take the place of deferred P0. SQLite enqueue attempts use zero busy wait and restore the
connection afterward. Catalog identity, root generation, and lane are checked before transaction
entry. Failed multi-row enqueue rolls back earlier rows. Ordinary worker enqueue, schema
migrations, source access, bridge contracts, and frontend behavior are unchanged.

The application handoff owns the pending plan and reservation. Writer contention retains both;
capacity backpressure publishes only a committed prefix and releases admission so consumers can
run. Non-contention failure releases admission without losing the plan. A later gap cannot
overwrite pending precise observations; pending ingress cannot appear synchronized. Stop, removed
or replaced roots, and an unsuccessful production poll retire their admission ownership.

Retained ordering also requires a catalog-wide coordinator boundary: first-import finalization,
LiveOnly opening persistence, and recovery-control leasing cannot synchronously queue behind the
same poll's pending P0. These operations share `production/catalog_scheduling.rs`; completed
worker collection and read-only status projection remain outside it. The actual two-root production
fixture proves pending A ingress alongside B opening, bounded return while the Recovery writer is
still held, eventual path publication, B LiveOnly completion, both roots synchronized, unchanged
source bytes, owned stop, and FULL reopen. Temporarily removing only this scheduling guard makes
that test fail at the intercepted second writer registration without stranding its thread; the
guard is restored before final-source checks. Its counterfactual RED evidence is
`build/r2c_interleaving_audit/item2_observer_self_wait_red_20260909.log`.

Focused checks pass 29 core synchronization cases, nine writer-admission cases, three direct
ingress transaction/binding cases, and the two-root production counterexample. Independent review
closed the coordinator self-wait finding and found no further defect in these changed boundaries.
Final-source checks additionally pass all 115 production synchronization tests: 102 runtime cases
in 416.43 seconds, six catalog-poll cases in 2.25 seconds, and seven inventory-cleanup cases in
1.37 seconds. The original mixed-load P95 assertion, complete P1/P2 work, and both connection-lifetime
arms remain active; ordinary captured output does not provide a fresh numerical P95 measurement.
All 108 queue tests pass in 40.64 seconds, including the three direct ingress cases already counted
above; all 13 import-admission cases pass in 8.92 seconds. There are no failures or ignored tests
in these focused suites. All-target, all-feature Clippy with warnings denied passes in 13.82 seconds;
Rust formatting, all 14 asynchronous bridge contracts with matching hashes, and whitespace checks
also pass. Production output is retained in
`build/r2c_interleaving_audit/item2_observer_production_final_20260909.log`; the remaining gate output
and explicitly marked terminal summary are retained in
`build/r2c_interleaving_audit/item2_observer_remaining_final_20260909.log`.

These are local final-source checks for this correction, not a full client acceptance result.
The unchanged complete production workload on committed source `7c529b9` subsequently passes in
200.28 seconds: 25 samples, P50/P95/maximum 77/91/182 ms, no sample over one second, all 2048 P1
candidates, and 10000 P2 entries through the original 4095-entry first-page boundary. Both lower
lanes progress in every sample. The full-publication tail takes 128629 ms and verifies all 10000
completed owners, completed control/run/baseline, retired authority, current checkpoint/root, owned
stop, FULL reopen, and source-byte preservation. Its numerical evidence is retained in
`build/r2c_interleaving_audit/item2_final_numeric_7c529b9_20260909.log`; the file explicitly marks
one truncated intermediate output chunk, while the aggregate and final result remain intact.
The complete local `quality_lint.ps1` also passes on this product source, including compiler-free
guardrails, unchanged formatting, all-target/all-feature Clippy, and strict Dart analysis. Its
output is retained in `build/r2c_interleaving_audit/item2_lint_7c529b9_20260909.log`.
Historical observer attribution and the whole item-2 exit remain separately open.
An exit-evidence review separates this gap from the verified current defects: the original run's
seven retained artifacts contain no Static/Rust profiling trace, and its outer observer duration
cannot distinguish internal operations. Repeating the lifetime comparison, constrained-CPU probe,
or controlled writer hold would add no historical discrimination. The fixed queue still requires
that attribution; it is not silently satisfied by these counterexamples or a later successful run.
No real-library access, hydration, or Ame desktop process is part of these controlled checks.

Physical ownership is explicit: observer handoff has 119 non-inline lines, ingress port 26,
SQLite ingress 108, reserved writer owner 310, and catalog scheduling 90. The core runtime is 877
non-inline lines. The production facade is 14531 lines: 4049 before its inline test module and
10482 in that region. Its larger physical debt is not closed by this extraction. Dedicated
admission, reservation, ingress-transaction, observer-contention, and production-contention
files have 327, 110, 140, 265, and 322 lines respectively; the first includes preexisting tests.

### Current hosted mixed-load failure — active item 2

Run `34283874470` on `42c9058` uses the same product source as `7c529b9`, but its Static/Rust
worker `102254793403` fails the production arm of the existing connection-lifetime comparison.
The main suite reports 1473 passed, one failed, and 19 ignored in 1386.92 seconds. The other nine
ordinary workers pass, including accessibility; that later pass does not repair the intermittent
item-3 failure below. Protected-release conditions remain unchanged. No failed run is rerun.

The failing production arm retains 25 samples, all 2048 P1 candidates, all 10000 P2 source entries,
the 4095-entry first page, and lower-lane progress in every sample. P0 P95/maximum are 1429/2830 ms,
with three samples above one second. Its 5045 polls open one connection and close none. Sample 9
contains 2283 ms in the catalog stage, sample 17 contains 1318 ms there, and sample 22 contains
1269 ms in queue persistence. These are current failures, not proof of the old 1092 ms cause.
The full failed-case output is retained untruncated in
`build/r2c_interleaving_audit/ci_42c9058_mixed_case_complete_102254793403.log` (967 lines).

The next diagnostic boundary separates checkout revalidation from session-registry lookup, each
identity/proof operation, and ingress reservation, admission, BEGIN, cleanup, enqueue, and COMMIT.
It also measures timeout set/restore and writer-permit/reservation retirement. Inner and outer
timers, sample start/visible boundaries, and lifetime-arm reports use the same immediate stderr
stream with thread correlation, rather than mixing live inner records with delayed libtest output.
This preserves slow successful operations and numerical results in hosted logs; best-effort writes
remain non-panicking during guard retirement. Sample markers surround the original stopwatch and
do not move any workload out of its measured interval.
It changes no query, invocation order, timeout, workload, admission, or proof. SQLite operation
timers exist only in test builds; the two outer checkout timers use existing debug diagnostics.
COMMIT is measured outside SQLite so its automatic-checkpoint tail is not mistaken for a fast
statement-profile callback. A zero busy timeout clears SQLite's busy and set-lock timeouts, but
does not prove a wall-clock bound on file I/O or all SQLite work. No inner operation is yet
identified by the captured failed run; these measurements are not a behavioral correction.

The unchanged local comparison before the final correlation/retirement-timer additions passes in
135.52 seconds. Per-poll and
production per-epoch P95 are 141 and 101 ms respectively; production maximum is 102 ms with zero
samples above one second. Both arms retain the original workload and lane-progress assertions.
The output in `build/r2c_interleaving_audit/item2_operation_diagnostics_42c9058_20260909.log`
explicitly contains one truncated intermediate chunk; both aggregates and the final result are
intact. No recorded new slow-operation timer identifies a cause. Eleven reusable-connection,
three reserved-ingress, eight poll-owner/lifecycle, and nineteen admission-related regressions pass
after adding retirement timers. All-target/all-feature Clippy with warnings denied also passes at
that checkpoint. These intermediate checks are not final-source or hosted acceptance. Item 2 remains active;
no further implementation item, changed exit criterion, or acceptance claim follows from this pass.

The final correlated comparison passes in 137.27 seconds under ordinary libtest capture, without
`--nocapture`. Its untruncated `item2_correlated_operation_trace_20260909.log` retains both arms,
all 50 start/visible marker pairs, original workload totals, and final retirement assertions.
Per-poll P95 is 160 ms; production P95/maximum are 103/124 ms with no sample over one second.
Production retains one connection through 3701 polls with zero poll-time closes. This verifies
the evidence transport, not the cause of the hosted failure. Final-source all-target/all-feature
Clippy, Rust format check, all 14 asynchronous bridge contracts, and whitespace validation pass.

The timing owner is 45 lines with no production clock/log state. Affected non-inline owners are
85 lines for reusable proof, 123 for ingress, 314 for admission, 245 for poll catalog, 91 for poll
timings, and 25 for observation timings. Dedicated priority/lifetime tests have 796/115 lines.
This instrumentation does not close the previously recorded facade or large-fixture physical debt.

### Current-path identity work — active item 2

Hosted run `34289089508` on `1513eab` passes all ten ordinary workers and the aggregate, with
1474 main Rust tests passing, zero failing, and 19 ignored. Protected-release conditions remain
unchanged. The same-workload production arm has 25 samples, P95 227 ms, maximum 1387 ms, and one
sample above one second. It retains all 2048 P1 candidates, all 10000 P2 source entries, the
4095-entry page, lower-lane progress in every sample, and one open with zero closes in 4734 polls.
Sample 20 contains a 1259 ms `reusable_identity_before` operation inside its 1306 ms queue-admission
interval. A separate `proof_identity_after` takes 4214 ms after the measured sample window; it
must not be assigned to sample 20. The complete comparison is retained in
`build/r2c_interleaving_audit/ci_1513eab_mixed_case_102271350680.log`.

The timed identity operation previously did `fs::canonicalize(path)` and then separately opened
the path for FileIdInfo. The correction obtains the normalized final path and ID from the same
fresh attribute-only handle. Windows flags, both observations around the SQL proof, schema/header
checks, process-registry revocation, held read guards, and their release order are unchanged. No
source media, database schema, dependency, workload, timeout, or percentile rule changes. The
platform behavior is documented by Microsoft's
[GetFinalPathNameByHandleW contract](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getfinalpathnamebyhandlew).
This removes one redundant open/close per identity observation, not all synchronous filesystem
latency. The current timer does not distinguish open, final-path query, FileIdInfo, and close, so
neither this reduction nor a green run uniquely attributes the older failures.

Three direct identity tests pass for Chinese and long paths (including the larger final-path
buffer), ordinary replacement, missing-file refusal, and temporary-handle release. All twelve
reusable-connection tests pass, including the new two-FULL-valid-catalog junction counterexample:
identical schema/header proof cannot authorize the old connection after the configured ancestor
redirects to B; the retained handle still reads A, revalidation is stale, a freshly validated
session reads B, and both revisions remain unchanged. The after-proof observation is independently
required to reject the same retarget with the previously valid before-proof identity.
All eight poll-owner/lifecycle tests also pass. These are focused adapter/owner results, not Windows
UI or final-head acceptance.
The 33 typed-read/retry tests pass with the extracted guard, including terminal reparse rejection,
replacement and ABA refusal, no database creation after deletion, and release-before-backoff.
The strengthened junction test is rerun by its complete exact name and passes both before- and
after-proof rejection.

The unchanged same-workload lifetime comparison passes in 280.34 seconds. Per-poll P95 is 282 ms;
production per-epoch P95/maximum are 161/207 ms with no sample above one second, one connection
open and zero poll-time closes in 4450 polls. Both arms retain all workload and lane-progress
assertions. Its complete stream is retained in
`build/r2c_interleaving_audit/item2_single_handle_identity_20260909.log`. An independently started
`flutter run -d windows` remained active on this workstation and was not terminated; these numbers
are not an isolated before/after performance comparison. The narrow independent static review
found no blocking defect in the changed identity helper and facade composition; that review does
not close the latency item's outstanding hosted evidence.
The complete standard `quality_lint.ps1` gate passes on the corrected source, including format
checks, all-target/all-feature Clippy with warnings denied, and Dart analysis. The former standalone
file-ID export now exists only for its remaining test callers; no warning suppression is added.

The catalog-specific filesystem owner contains 75 lines without inline tests, its direct tests
55 lines, and the separate junction suite 173 lines. The local-files facade decreases to 7466
lines (4058 before its inline test region and 3408 within it); the SQLite facade decreases to
5328 non-inline lines. Existing physical decomposition debt is not closed by this extraction.

### New-head hosted accessibility failure — queued item 3

Run `34280948135` on `7c529b9` finishes with nine ordinary workers passing, one failing, and a
failed aggregate gate. Static/Rust reports 1474 passed, zero failed, and 19 explicitly ignored
main-suite tests in 1265.41 seconds; two nested child summaries are not additional main-suite
cases. The original mixed-load gate and complete recovery assertions pass without a new numeric
percentile in hosted captured output. All five synthetic workloads, Flutter tests, controlled
Windows scan, and unsigned x64 verification pass. The three protected-release-only jobs retain
their intentional skip conditions. No result is inferred from the preceding head.

The sole failing worker, `102245297409`, exceeds the unchanged `application-ready` parent deadline
after the previous
`native-semantics-ready` phase passed. Its last verified probe progress is `loading-uia-client`
at 201 ms; no completed application-ready UIA traversal is reported. Completion evidence confirms
primary process exit and owned Job closure, with no cleanup failure. This is a current hosted
acceptance failure, not a new synchronization assertion, an AXTree diagnostic, or proof of a
particular assembly-loading cause. The original 400-line job log is retained untruncated as
`build/r2c_interleaving_audit/ci_7c529b9_uia_102245297409.log`. No rerun is used to replace it.
The finding belongs to the already queued native/client verification item and blocks final-head
readiness; it does not authorize changing the fixed implementation order or extending its timeout.

## Physical ownership review

Counts include whitespace and comments. The non-inline region may contain `cfg(test)` imports,
hooks, and helpers; it is not advertised as pure production SLOC. Dedicated-test totals describe
affected files, including their preexisting cases, not newly written lines.

The writer-fairness extraction leaves `sqlite_catalog.rs` at 5342 lines and moves ordering and
registration ownership into a 275-line module with no inline tests. Its dedicated test module
contains 310 lines. The facade still owns connection and transaction composition and remains
physical decomposition debt; these counts do not claim that the entire catalog adapter is small.

| Rust owner | Total | Non-inline region | Inline-test region |
| --- | ---: | ---: | ---: |
| `local_files.rs` | 7494 | 4086 | 3408 |
| `preview_cache.rs` | 1802 | 908 | 894 |
| `incremental_library_changes.rs` | 1745 | 1745 | 0 |
| `scan_library.rs` | 1231 | 1231 | 0 |
| `preview.rs` | 1662 | 611 | 1051 |
| `preview_cleanup.rs` | 1062 | 604 | 458 |
| `preview_recovery.rs` | 575 | 302 | 273 |
| `catalog_session.rs` | 371 | 371 | 0 |

Affected dedicated Rust tests span 21 files and 8581 lines at the cache-remediation checkpoint.
The seven Dart test/support files contain 1375 lines. The screen is 2321 lines, viewport 1642,
primary-scan lifecycle 610, viewer session 234, scan run 116, page owner 77, and scan control 62.
Dedicated owners now hold file admission, source/cleanup reservation, namespace authority,
preparation, staging encoding, maintenance admission, terminal media, store admission, and scan
failure handoff. No duplicate production state machine or naming violation was found in this review.
Large-file debt remains, including the 4142-line incremental test owner and the long scan execution
function; these changes do not claim completion of the roadmap's broader physical decomposition.

The formatted publication repair adds a 25-line read-only port and a 234-line transaction owner.
Its SQL facade is now 1020 lines; the application publication owner is 196 lines, including its
existing inline policy tests. Four dedicated control/transaction test files contain 665 lines
(266, 145, 130, and 124). These are a later checkpoint, not additions to the cache-checkpoint totals
above; no new behavior was appended to the long scan execution function for this repair.

The terminal-media follow-up extracts a 103-line rename-composition owner and retains a 110-line
terminal-evidence owner. The new incremental terminal suite has 177 lines. The inventory test
facade decreases to 3889 lines, with 321 lines in its dedicated terminal-media suite. These are
later physical measurements, not additions to the earlier cache-checkpoint test totals.

At the cleanup checkpoint, the accessibility facade has 377 lines, process owner 416, evidence owner 180, and shared
cleanup owner 155; its dedicated failure-injection suites have 308 and 82 lines. The existing guardrail
entrypoint has 494 lines and invokes both suites; the native probe has 546 lines. These are
physical boundaries, not a claim
that native execution or original crash attribution is complete.

The subsequent success-evidence extraction leaves the facade at 380 lines, process owner at 377,
and evidence owner at 243. Its dedicated compiler-free evidence suite has 173 lines; the existing
guardrail entrypoint has 498. It moves completion validation rather than duplicating it, and adds
no alternate native traversal or deadline policy.

The native integration file now has 940 dedicated-test lines, a net increase of 50 for real menu
assertions and failure context. It adds no production code or new responsibility; existing probe
protocol and controlled catalog fixtures remain candidates for a meaningful future test split.

The native window owner is 142 production lines, with no inline tests; its repair adds two net
lines. The dedicated HWND fixture has 150 lines and its independent CMake target 42. The public
runner has 27 lines, its internal execution/result owner 133, and the dedicated compiler-free suite
224. Native test building does not add another engine or window state machine to production.

At the real-engine retirement checkpoint, the window owner has 146 production lines and the native
entrypoint 60, both without inline tests. The separate engine fixture has 146 C++ lines and a 52-line
CMake target. Its input, result, and execution owners have 108, 50, and 203 lines; the dedicated
compiler-free suite has 236. The existing public facade is 31 lines and its combined guardrail 242.
The two production ownership corrections do not add another lifetime flag or callback state machine.

At the preparation checkpoint, the incremental facade has 1754 production lines with no inline
tests; its read-set and rebase owners have 231 and 88, and rename composition has 104. The dedicated
test facade has 3773 lines, the extracted race repository helper 394, and the new regression suite
504. The synchronization runtime has 4077 non-inline and 10502 inline-test lines; its extracted poll
diagnostic owner has 87 production lines and the dedicated priority suite has 965. The existing
runtime/lane and test decomposition debt remains open; this repair does not add another scheduling
policy to that large owner.

At the inventory-admission and removed-root-authority checkpoint, the metadata-inventory facade
has 3740 lines and its lifecycle owner 220, with no inline-test modules. Its dedicated admission
and race suites have 203, 106, and 264 lines; the separate raw-spool lifecycle suite has 296 and
retains its unresolved desired-behavior failures. Root retirement has 118 production lines,
compatibility repair 30, and current-schema composition 89. Shared queue persistence decreases
to 1783 lines. The dedicated removal and compatibility suites have 433 and 288 lines. Historical
migration SQL and facade decomposition remain explicit debt, not closed by these small owners.

At the subsequent nullable-spool/retention checkpoint, raw-spool retirement has 53 production
lines, terminal queue cleanup 119, and inventory lifecycle 236, with no inline tests. Shared
queue persistence decreases to 1689 lines and the metadata-inventory facade to 3726. Dedicated
spool setup/lifecycle, retirement, and scope-isolation suites have 322, 275, and 152 lines;
the queue-retention suite has 83. The split follows raw ownership, logical retention, queue
dependency retention, and their separate regression fixtures rather than arbitrary line caps.

At the stale-initialization checkpoint, the metadata-inventory facade decreases to 3581 lines and
the separate initialization/reset owner contains 208 lines, both without inline-test modules.
The dedicated shared spool lifecycle fixture has 342 lines and the stale-source suite 208. The
remaining spool writes, paging, and migration validation are still separate physical decomposition
obligations; extracting initialization does not make the entire inventory facade small.

At the execution-fencing checkpoint, the metadata-inventory facade decreases to 3162 non-inline
lines. Initialization, execution proof, raw reading, and raw writing contain 164, 167, 171, and
268 lines, respectively, with no inline test cases. The durable source has 278 production lines;
application composition has 1543 non-inline lines. Dedicated execution, admission/interleaving,
and read-transaction test/support files contain 129, 182, and 98 lines. The existing inventory test
facade has 3909 lines and shared spool fixture 344. These counts are a new checkpoint, not additions
to preceding totals. Migration validation, the remaining inventory facade, and larger physical
decomposition remain debt; the split removes raw SQL responsibility rather than forwarding it.

The poll diagnostic checkpoint leaves the runtime at 4092 non-inline and 10502 inline-test lines,
the core observer coordinator at 886 non-inline lines, and the catalog facade at 5355 non-inline
lines. Existing poll-stage logging has 87 lines; its shared observation timer has 21. Catalog
open and retirement instrumentation have 39 and 56 test-only lines and no new test cases. This
moves one timing helper instead of copying it; it does not claim to decompose runtime lifecycle,
SQL ownership, or the remaining large test region.

The spool-row checkpoint reduces `migrations.rs` from 15553 to 15486 lines: 10223 before its inline
test module and 5263 in that module. Its extracted relational validator has 87 non-inline lines and
212 dedicated-test lines. Historical migration SQL remains oversized; this change moves one real
proof responsibility, not the whole migration owner or its test region.
