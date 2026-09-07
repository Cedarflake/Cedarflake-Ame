# R2c cross-workflow interleaving remediation

Date: 2026-09-07

Status: implementation and verification in progress; not release acceptance.

## Scope and safety

This follow-up covers interactions missed by the earlier green PR #12 head: source admission and
destructive cleanup, catalog maintenance and active scans, discovery and retry ownership, scan
commands and asynchronous registration, and viewer/cache ownership after an operation is retired.
It does not infer safety from isolated green tests or claim that every possible defect is excluded.

Reproductions use generated disposable sources, isolated catalogs, and controlled application
ports. No retained source root, live user catalog, cloud placeholder, installed service, or original
media is accessed or modified. Real-library and signed/installed-service acceptance remain separate.
The active delivery order remains in [the canonical roadmap](../roadmap.md).

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

## Physical ownership review

Counts include whitespace and comments. The non-inline region may contain `cfg(test)` imports,
hooks, and helpers; it is not advertised as pure production SLOC. Dedicated-test totals describe
affected files, including their preexisting cases, not newly written lines.

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
