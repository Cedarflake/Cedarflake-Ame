# R2c maintenance evidence

This record tracks the four required maintenance deliveries in the
[closeout plan](../plans/r2c-closeout.md#completion-goal-and-required-deliveries).
They preserve existing policy and lifecycle contracts. They do not themselves diagnose or close
the previously reported native browsing failures, nor establish final R2c acceptance.

## M01 status and task-kind projection

The baseline `22c5a0c` retains the nested policy assessed at `c358661`. The rewrite makes removal,
query refresh without scan feedback, absent scan, feedback-free catalog state and scan feedback
precedence explicit. A primary scan with a null task kind still shadows the legacy task kind.
No state-model, UI-policy, schema or bridge change is introduced.

The initial 23-case projection suite passed against the original getter before modification.
The final 25-case suite adds explicit empty/completed feedback boundaries and passes after the
rewrite. Expectations include committed removal, root presence, copy/replacement, query activity,
scan feedback and dependent scanning/processing/admission flags.

## M02 preview request ordering

`library_preview_order.dart` owns a pure, allocation-free comparison: priority first, then existing
demand rank before an absent rank, lower rank first, and finally earlier sequence. Exact ties retain
the selected request. The queue still excludes active locations before comparison and keeps its
request/source/query authority, concurrency, cancellation, cooldown and result-publication rules.
The queue re-exports the existing priority type so caller imports remain valid.

Seven policy tests cover independent priority/rank/tie examples and all 3600 combinations of five
priorities, four rank states and three arrival sequences against the frozen original expression.
All 29 existing queue tests pass, including active-location/retry ordering, obsolete source results,
priority replacement, root blocking and hard concurrency limits. Across these policy suites,
primary control/workflow and primary feedback, the serial focused command passes 92 tests.

These focused results precede final-source Daily, Windows/client and remaining functional evidence.
They cannot waive M03/M04 or the frozen 24-variant roster.

## Policy-slice verification checkpoint

The canonical complete lint gate passes: compiler-free positive/negative guardrails, formatting,
all-target/all-feature Clippy with warnings denied, and the Dart analyzer. Expected diagnostic output
from deliberately rejecting guardrail fixtures is not a native application failure. One independent
read-only review reports no actionable finding after comparing HEAD precedence and imports; three
active minutes are charged within the existing review allowance.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `library_state.dart` | 398 | 0 | 305 new projection cases; existing workflow suites retained |
| `library_preview_queue.dart` | 701 | 0 | 1061 existing queue suite |
| `library_preview_order.dart` | 24 | 0 | 111 new policy suite |

The ordering extraction adds no per-request allocation or catalog/filesystem access. The state
rewrite adds no state or asynchronous work. Neither change touches source media, generated bridges,
dependencies or persistence. Physical queue size remains explicit; no unrelated lifecycle refactor
was mixed into its pure selection-policy change.

## M03 original-source cost baseline

The unchanged Rust implementation at `148e23e` ran the existing 10000-file generated benchmark once.
Its original 60-second cold-scan assertion failed: cold 72090 ms, warm 34666 ms, pause 4 ms, resume
59654 ms, cancel 128 ms; fixture creation 14589 ms. Peak observed working set was 31612928 bytes
against 536870912. Catalog sizes were 54165504 and 31080448 bytes. The test process took 190.96
seconds after 27.01 seconds of compilation. These are original-source measurements, not evidence
of an extraction regression or a passing benchmark.

The surrounding invocation also used an incorrect lock-release parameter, `LockStream` instead of
`Mutex`, and exited with that additional error after the benchmark failure. The process exited and
released its process-local mutex; no owned test process remained. Preserve both failures. Correct
the invocation for later commands, keep the cold threshold unchanged, and diagnose stage costs
before another performance acceptance run. Correctness extraction can proceed independently; the
failed cost gate and same-workload comparison remain open with the other final performance duties.

## M03 scan ownership and correctness

The scan command now composes traversal, final validation and publication. Traversal owns directory
and entry windows, cancellation, progress and checkpoint ordering. Typed entry application owns
directory/ignored/failed/file outcomes and detached observation. File preparation owns distinct
path, identity, preservation and reusable prior evidence, with unchanged source-version, metadata
engine and preview-artifact checks. The shared finalization checkpoint interval remains 128.
The extraction adds no catalog query, transaction, source write, dependency or schema change.

All 89 existing scan cases pass; two explicit manual/authorization-bound cases remain ignored.
Compilation takes 24.47 seconds and execution 70.72 seconds. Six dedicated entry-boundary cases
then pass in 1.60 seconds after 11.37 seconds of compilation: detach before staging on discovery
or metadata feedback, successful staging/checkpoint order, precise retry without invented assets,
published same-identity metadata reuse, and changed-source preservation without metadata reuse.

Initial compilation exposed a moved shared constant and a formerly implicit test import. Initial
new-case failures exposed fixture assumptions: invalid source-revision evidence, counting pending
staging through raw SQL, reusing an unpublished prior, and querying authoritative retry paths for
a foreground scan. The fixtures now use canonical source identity, the staging count contract,
actual baseline publication and the correct retained-rejection observation. Production checks were
not weakened. The changed-source case proves prior selection; existing scan tests retain the
complete failure/publication oracles. Independent review plus one scoped fixture recheck reports
no actionable issue, charging six active minutes. Performance acceptance remains open above.

### M03 event-boundary cost checkpoint

The 2026-09-24 measurement uses production base `562e53d` plus three test-only files. It retains the
original workstation Debug command, 10000 generated 2-by-2 PNG files, cold/warm/pause/resume/cancel
sequence, 60/60/5/60/5-second limits and 512 MiB working-set ceiling. Hosted synthetic jobs use
Release; their passing results cannot replace this Debug gate. Production, bridge, schema and
dependencies are unchanged. Intervening product changes and host conditions also prevent treating
this measurement as an isolated causal comparison of the earlier M03 extraction.

`tests/phase_cost.rs` records monotonic intervals without retaining events or adding I/O. All events
require the current scan identity and lifecycle; discovery cannot follow finalization, the first
validation counter must be zero, and issues or non-success terminal states reject the measurement.
Completion requires the full, issue-free, unlimited inventory. Repeated complete counters retain
the first timestamp. Nine deterministic boundary cases pass after independent method review
identified and corrected the initial observer's unchecked event variants and missing zero boundary.
The first four-case result is superseded by this nine-case result, not a failed product test.

The initial capture failed before workload launch because its tool process inherited equal `Path`
and `PATH` entries. The original admission and empty output remain in `.build/r2c-scan-phase-cost/`.
A process-only normalization was first checked to reduce two equal entries to one without changing
the value. The corrected capture rejects any other duplicate shape; it changes no system settings.
Only one actual workload ran, with the canonical command under the repository mutex. Its source and
helper hashes remained unchanged, and its actual failing exit code is retained.

| Operation | Entry to Started (s) | Started to first Finalizing (s) | First to complete validation counter (s) | Complete counter to Completed (s) | Completed to return (s) | Total (s) |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Cold | 0.269 | 55.297 | 12.071 | 0.842 | 0.015 | 68.494 |
| Warm | 0.022 | 23.431 | 11.870 | 2.443 | 0.008 | 37.774 |
| Resume | 0.071 | 55.366 | 12.175 | 0.865 | 0.015 | 68.491 |

Each operation observed one complete-counter event. Values are rounded; raw microseconds are
retained. Started-to-Finalizing includes traversal, identity/metadata work, staging, checkpoints
and validation preparation. Complete-counter-to-Completed includes validation cleanup, namespace
revalidation and publication; it is not a pure SQLite or scheduler measurement.

The original cold assertion **fails** at 68.494 seconds. Recorded resume cost also exceeds its
60-second limit, although that later assertion is not reached. Warm, pause (10 ms) and cancel
(142 ms) measurements remain below their unchanged limits. Peak observed working set is 34193408
bytes; catalog sizes are 54145024 and 31244288 bytes. Fixture creation takes 13.357 seconds,
compilation 32.15 seconds, test execution 199.73 seconds and the captured command 234.974 seconds.
The membership/cancelled-staging assertions before the cold limit pass. The later warm, pause,
resume and cancel limits, catalog-size assertions, two selected-source byte checks and final
source-file count do not execute after that failure. Their recorded measurements do not stand in
for passing assertions; this failed run is not a source-integrity pass. No real-root input is supplied.

The dominant current interval is before final validation. Cold and resume spend about 55 seconds
there versus 23 seconds warm, but the event boundary cannot distinguish inspection, identity reads,
staging or host I/O cost. It does not establish a cause or authorize an optimization. The standalone
workload has no concurrent native-client import peer; its short final interval cannot explain or
dismiss the earlier Sandbox finalization wait. M03 cost acceptance remains open; the native report's
current [observation-only disposition](r2c-browsing-admission.md#finalization-observation-disposition)
is separate. Further workload execution requires a changed, bounded method that measures the
responsible operations inside this interval; repeating this unchanged benchmark is not admitted.

Raw output, the failed result, source hashes and helper hashes are retained in
`.build/r2c-scan-phase-cost-verified/benchmark-result.json` and its matching stdout/stderr. The test
module is 5497 dedicated-test lines; the new observer is 165 and its boundary suite 222. The scan
facade remains 728 production lines with no inline tests. These counts do not justify unrelated
decomposition. Complete lint passes in 134.565 seconds with the frozen test sources unchanged.
Complete serial Daily then passes in 1815.852 seconds on the same three source hashes: 1548 Rust
library tests, three broker tests, 96 Flutter test files, native Windows scan, all ten whole-window
accessibility phases, 16 asynchronous bridge contracts and matching hashes. Nineteen existing
manual/authorization-bound Rust tests remain ignored. Native scan finishes in 52.455 seconds;
scan and accessibility report successful exit and no cleanup failure. The failed manual performance
workload is not repeated by Daily. Independent evidence review confirms the phase numbers, hashes
and failed/unexecuted assertion boundaries; it does not claim a passing performance gate.

The ignored local result receipts bind the raw output and frozen inputs:

| Receipt in `.build/r2c-scan-phase-cost-verified/` | SHA-256 |
| --- | --- |
| `benchmark-result.json` | `944081F6CCCBB5E217CB5FF24221D85DB0E13BF98C5F9433BEE1486CB6DD8201` |
| `lint-result.json` | `2830EDD92B65E55A0E373F14182E16D7962D681231141C750BC07AB1975DC8AA` |
| `daily-result.json` | `3F36A6DE46D1FB7CAC53F327E314E5F4F54034C1E455166FC3389850149EEEFF` |

The existing unsigned Release and selected native position evidence at `562e53d` remains applicable
to its unchanged product sources; these new files compile only in Rust tests. That reuse is not a
new Release build, full client acceptance or a replacement for the failed Debug cost gate. The next
owning-operation measurement was queued at that checkpoint; its subsequent result follows.

Hosted [36002444315](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/36002444315) completes
successfully on committed phase-observer checkpoint `8d83a51`: all ten required jobs and the aggregate
Windows gate pass, while three signing-only jobs skip. That run does not cover the subsequent
operation-observer working tree and does not replace the failed workstation Debug performance gate.

### M03 owning-operation cost checkpoint

The second bounded diagnostic pass uses `8d83a51` plus six Rust files. A fixed six-category
accumulator belongs to one synchronous test thread and one scan scope. Timers retain that scope's
identity through drop; unfinished/overlapping scopes, overlapping operations and inconsistent
elapsed totals reject reporting. No per-file record or path list is retained. Non-test expansion
executes the original call expressions and original iterator; application behavior, source guards,
transactions, schema, bridge and dependencies are unchanged.

The first focused invocation fails to compile a new test's parent-module import; it never runs the
workload. After that correction, 11 focused cases and lint pass. Independent method review then
finds that directory creation returns a lazy iterator: timing only its constructor omits `next()`.
A test-only iterator now times each original `next()` without prefetching or changing its release
point. Three additional cases cover laziness/exhaustion, early exit and unwind retirement, and the
two-image scan counts creation, all iterator reads and source visits. Final focused evidence has
14 passes, zero failures/ignored and unchanged inputs: 44.32 seconds compilation, 0.33 seconds test
execution and 49.088 seconds captured command time. The scoped recheck admits one measurement.
Final complete lint passes in 245.458 seconds; the earlier 201-second pass is superseded.

| Owning operation (seconds) | Cold | Warm | Resume |
| --- | ---: | ---: | ---: |
| Source discovery: iterator creation, each `next()`, and source visits | 11.835 | 8.872 | 9.220 |
| Prior selection and identity preparation | 4.605 | 1.679 | 3.881 |
| Media inspection, including metadata reuse | 39.150 | 0.005 | 30.233 |
| Successful location staging | 18.968 | 14.399 | 14.101 |
| Directory persistence reads and writes | 0.183 | 0.162 | 0.170 |
| Traversal checkpoint persistence | 0.086 | 0.103 | 0.083 |
| Accounted operation time | 74.827 | 25.219 | 57.688 |
| Unaccounted whole-scan time | 13.941 | 15.011 | 13.228 |
| Whole scan, truncated to milliseconds | 88.767 | 40.230 | 70.916 |

Each scan records 20002 discovery calls, 10000 calls in each prior/inspection/staging category,
86 directory-persistence calls and 79 traversal checkpoints. Discovery includes one iterator
construction, 10001 iterator reads including exhaustion, and 10000 source visits. The resumed
workload also traverses 10000 entries under the existing resume policy; the observer does not
change that policy. Raw microseconds are retained; summing individually truncated intervals can
differ by a few microseconds from the combined duration.

Measured operations account for all but approximately 0.380 / 0.248 / 0.328 seconds of the
Started-to-Finalizing intervals. Whole-scan unaccounted time also includes admission, final
validation, publication, return, object conversion and measurement overhead. Validation takes
11.940 / 11.846 / 11.778 seconds; complete-counter-to-Completed takes 1.066 / 2.885 / 1.039 seconds.
Each scan emits one complete-counter event. These aggregate boundaries cannot assign native system
call, decoder, SQLite, host scheduling or storage-device cost within an operation.

The unchanged cold limit **fails** at 88.767 seconds. Resume is also measured above 60 seconds,
although its assertion is not reached. Warm 40.230 seconds, pause 12 ms and cancel 184 ms are recorded
without converting later unexecuted assertions into passes. Peak observed test working set is
34803712 bytes against the unchanged 536870912-byte ceiling. Catalog sizes are 54128640 and
31150080 bytes. Fixture creation takes 35.141 seconds, compilation 22.62 seconds, test execution
247.12 seconds and the captured command 271.190 seconds. Cargo exits 101 and the capture exits 1;
source/helper hashes are unchanged. Membership, resumed terminal state and cancelled-staging checks
before the cold limit pass. Later latency/catalog assertions and the two source-byte/final source
count checks do not execute. This remains a failed performance run, not a source-integrity pass.

The dominant measured owner is media inspection, followed by location staging. The low warm-path
cost is consistent with the existing metadata-reuse branch; this pass does not separately count
branch hits or identify which required source-validation or decoder operation can safely be
removed from cold work. The existing
`inspect_with_discovery` path also reopens/revalidates the configured root before data access;
switching to it is not an established optimization. Source inspection also contains test-only
observation work. Its contribution is not measured separately. No guard removal, longer namespace
lock lifetime, counter change or speculative fast path follows from this result.

The 35.141-second fixture creation and changed individual operation costs prevent interpreting
the difference from the preceding 68.494-second run as an isolated instrumentation or product
regression. Original Debug performance acceptance remains open. This standalone workload has no
concurrent native import peer and does not establish the earlier wait's cause; its current
[observation-only disposition](r2c-browsing-admission.md#finalization-observation-disposition)
does not waive the performance failure. The second diagnostic workload allowance is consumed;
another unchanged or exploratory workload is not admitted by this checkpoint. A later repair needs
a reproduced cause, its owning boundary, a causal regression and a separately recorded bounded method.

Physical review sizes are 740 lines in the scan facade, 211 in entry processing and 346 in traversal,
including test-only observation declarations; they contain no inline test functions. The new
test-only collector/iterator owner is 172 lines and its dedicated suite 338. The existing main scan
test module is 5512 lines. These additions do not authorize unrelated decomposition.

Original compile failure and intermediate passing evidence remain in
`.build/r2c-scan-operation-cost/` and `.build/r2c-scan-operation-cost-verified/`. Final receipts bind
the new measurement and raw stdout/stderr in `.build/r2c-scan-operation-cost-final/`:

| Receipt | SHA-256 |
| --- | --- |
| `focused-result.json` | `9F0DEA04342884EBF1A39F655311257BDB7ADE88A4677B4345D44985B663C601` |
| `lint-result.json` | `C1301DBEC1E5CE846EE5E9833CC6A34D103B47D0C2498E09C8EE308F8F320818` |
| `benchmark-result.json` | `40949710548F4070F7465AFF7B3A40ABE301C3688FF4F7B625BC811825FE99EF` |
| `daily-result.json` | `3FDB6BB858081470831CDAB0B2CE0B3E07057E981887AA79C45BB26971A1EBC2` |

Complete serial Daily passes in 2156.569 seconds with exit zero and unchanged source/helper hashes:
1562 Rust tests pass with the existing 19 ignored, three broker binary tests pass, and all 96 Flutter
test files pass. The actual Windows picker/scan/preview integration passes in 79.276 seconds without
cleanup failures. All ten native UIA phases pass; the primary process exits, the owned Job closes,
and the owned scratch directory is removed. Sixteen asynchronous bridge contracts and matching
content hashes also pass. Controlled rejecting examples earlier in the log remain separate from
the final native completion records.

The current connection-lifetime control and complete P0/P1/P2 mixed-load case pass within that
Daily. They do not explain preceding failures or close final candidate acceptance. Daily does not
repeat the ignored performance workload or substitute for its failed gate. This test-only change
does not claim a new isolated Release client run or signed installed-service evidence.

The final independent evidence recheck confirms the receipt/log bindings, totals and retained
failure boundaries. The next populated-startup method remains queued; that review does not admit
or execute its separate workload.

## M04 navigation ownership and lifecycle

`LibraryTimeNavigationRequests` owns pending and active requests, duplicate-target sharing, latest
intent, the explicit visible-range owner, the existing 120 ms blocked retry, and matching loading
cleanup. The viewport retains query, publication and query-transition generations, catalog reads,
retained pages and immutable state projection. Navigation acceptance and publication authority are
both required; no generation is merged and no duplicate mutable ownership is retained.

Eleven new deterministic cases cover pending/active sharing, target replacement, old cleanup versus
new loading, passive/explicit admission, passive retirement, query replacement with late success
and failure, retry after current failure, blocked timing, incompatible work and disposal of active,
queued and timer-owned work. Together with controller, primary workflow, query-refresh and
bidirectional paging regressions, 93 tests pass. The affected preview coordinator (10), time-rail
publication (6), time navigation (11) and viewer/gallery geometry (2) suites also pass: 29 tests.
These include both tested window widths and stale-pointer retirement.

Independent review against `148e23e` reports no actionable issue and charges 2.2 active minutes.
The complete lint attempt passes its compiler-free guardrails, formatting and all-target/all-feature
Clippy, then correctly fails on six constructor initializing-formal style findings. Correcting only
those declarations preserves the public constructor parameters; formatting reports no changes and
the full Dart analyzer then passes with no issues. This is a corrected partition result, not a
passing complete Daily. Final native, performance and accumulated-source gates remain required.

| Owner | Production lines | Inline test lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `scan_library.rs` | 728, down from 1231; main function 395, down from 891 | 0 | 5454 existing main test module, plus existing child suites |
| `scan_library/traversal.rs` | 304 | 0 | Existing scan workflows above |
| `scan_library/entry_processing.rs` | 202 | 0 | 319 new boundary suite |
| `scan_library/file_preparation.rs` | 178 | 0 | Same entry suite and existing scan workflows |
| `library_viewport_controller.dart` | 1344, down from 1597 | 0 | Existing controller/query/paging suites |
| `library_time_navigation_requests.dart` | 297 | 0 | 269 new lifecycle suite |

The remaining facade sizes are explicit, not proof of an all-controller or all-scan audit. These
extractions address the four assessed responsibilities; other physical decomposition stays in its
existing roadmap order. No current-source final acceptance or C05 closure follows from these tests.
