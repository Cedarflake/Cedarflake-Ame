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
