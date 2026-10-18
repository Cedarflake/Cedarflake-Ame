# Maintainability refactor execution plan

Status: M1–M8 merged; M9–M13 implemented; follow-up hosted verification in progress

Updated: 2026-10-18

Historical branches: M1 `codex/maintainability-refactor`; M2 `codex/synchronization-runtime`, based on M1 `ef294ad`;
M3 `codex/gallery-publication-lifecycles`, based on M2 `6f80d66`;
M4 `codex/gallery-retained-window`, based on M3 and including its `4335ae7` constructor correction;
M5 `codex/retire-legacy-usn`, based on M4 `86feb69`;
M6 `codex/workflow-script-ownership`, based on M5 `e4096a2`.
Combined follow-up: `codex/gallery-card-size-anchor`, based on M6 `1e87561`, retains all subsequent
gallery, journal, catalog-proof and accessibility-preparation commits through `ff1d78f`.
PR #29 owns the consolidated follow-up delivery. Obsolete local and remote branches were deleted
after their complete histories were verified as reachable from `main`.

## Outcome and boundaries

Reduce the maintenance cost identified at baseline `83c8c16` by moving independent invariants and
lifecycles to their owning modules, consolidating duplicated window rules, and retiring superseded
implementation only after its useful coverage belongs to the current path. Preserve delivered R2c/R3
behavior and the accepted UI. Keep code readable to human maintainers, with clear responsibilities,
visible policy and lifecycle authority, and cohesive tests. R4 remains unstarted.

Keep the accepted Flutter/Riverpod, Rust, typed FRB bridge and SQLite stack. Freezed generation,
the two JPEG capabilities and the patched Windows watcher retain their existing admission decisions.
Dependency removal requires equivalent capabilities and boundary evidence; this plan does not
schedule a framework replacement, package upgrade, schema change or new product feature.

The [repository contract](../../AGENTS.md) owns safety and delivery rules. [ADR 0025](../architecture/0025-invariant-owned-workflow-modules.md)
owns responsibility boundaries; amend its applicable entries as boundaries are implemented.
[Quality gates](../acceptance/quality-gates.md#required-engineering-workflow) owns verification execution.
This plan tracks work and completion conditions without duplicating those rules.

## Current maintenance slice

The first round merged into `main` at `40cb302` with the complete file tree of audited candidate
`a894b03`. PRs #22–#27 and #29 are merged and native stack #35 is closed. Original commits remain
reachable from `main`; the temporary history branch was removed with the obsolete delivery branches.
The acceptance record owns merge-history details and current checks.

M7 and M8 merged through PR #36 at `fcb53e2`, preserving the complete verified `2dadf96` tree and
history. Its local and remote branch was deleted after ancestry, tree and dependency checks.
M9, M10 and M11 were implemented in order on separate branches and review PRs. Native GitHub stack
#40 contains [M9 / #37](https://github.com/Cedarflake/Cedarflake-Ame/pull/37),
[M10 / #38](https://github.com/Cedarflake/Cedarflake-Ame/pull/38) and
[M11 / #39](https://github.com/Cedarflake/Cedarflake-Ame/pull/39). M9 starts from the merged baseline;
each later branch retains the preceding work without rewriting published history. Stack creation
preserved all three heads. The user authorized merging the current maintenance branches on
2026-10-18 after applicable checks and final review. Existing notification
text, UI layout, action admission and source-safety boundaries are preserved.

| Order | Outcome | Proving boundary | State |
| --- | --- | --- | --- |
| M7 | Move gallery synchronization notification state and typed action selection out of the screen | Repeated conditions, root generations/removal, resolved and stale actions, refresh retry, live-only capability and existing widget wiring | Merged through PR #36; focused checks, native checks, review and exact-head CI passed |
| M8 | Move complete preview-reset transactions out of the catalog facade | Full/selective reset, handoff owners, dimensions and source-state preservation, invalid scope and rollback | Merged through PR #36; focused checks, native checks, review and exact-head CI passed |
| M9 | Consolidate shared interaction ownership across the gallery wall's complete and estimated layout paths | Wheel input, scroll notifications, visible range, preview demand and position feedback in both paths; preserve layout-specific behavior, virtualization, anchors and timeline authority | Implemented on `codex/gallery-wall-interactions`; focused checks, native checks, scoped review and all 11 hosted jobs passed |
| M10 | Separate directory enumeration and native file-access responsibilities in `local_files.rs`, beginning with enumeration and record parsing | Corrupt directory records, cancellation, unavailable/cloud-only files, handle lifetime and identity revalidation; preserve no-hydration and source-media safety | Implemented on `codex/native-directory-enumeration`; focused native boundaries, production Clippy, scoped review, combined Debug comparison and all 11 hosted jobs passed |
| M11 | Separate persistent-journal baseline lifecycle, atomic batch publication and completed-record maintenance | Batch records, checkpoints and root associations commit or roll back together; preserve recovery, bounded reads, cancellation and browsing priority | Implemented on `codex/persistent-journal-ownership`; 73 focused persistence regressions, production Clippy, combined Debug comparison, scoped review and all 11 hosted jobs passed |

Keep shared notification history and synchronization refresh with their existing owners. Keep each
preview reset atomic, including pending-location flush order and all affected ownership tables.
Use focused regressions and affected Debug notification/cache workflows, followed by hosted CI and
scoped review. Record production and test sizes with the evidence. M9–M11 are the authorized
current delivery. Their order puts visible interaction ownership
before the higher-risk source-file and durable-journal boundaries. Move the affected tests with
each owner; retain genuinely shared fixtures without making test-file cleanup a separate workstream.

Closeout also includes the merged-main mixed-load failure: recovery selection can wait on a read
lock before the existing bounded write phase. Its reproduced boundary repair and unchanged-load
verification stay in PR #36; the [acceptance record](../acceptance/maintainability-refactor.md#recovery-selection-contention-follow-up)
distinguishes the confirmed lock-wait gap from the still-unclassified stage within the original
hosted timing. Exact-head run `39057472846` passed all 11 applicable jobs before the authorized merge.

## Requested verification-maintenance follow-ups

After M9–M11 implementation, the user requested shorter CI latency and responsibility-owned tests.
Proceed in the order below without reopening product acceptance. Preserve existing workloads,
assertions, expected ignores, data safety and isolated timing measurements. Keep each outcome in a
separate commit and branch, and extend the existing review stack after verification. The user's
2026-10-18 merge authorization covers this queue after the mixed-load failures are resolved and
verification is complete. The user confirmed keeping the existing dependent PRs and merging the
whole maintenance stack together; do not replace the CI and test PRs with independent deliveries.
Preserve published history and remove obsolete branches only after their commits are reachable
from `main`.

| Order | Outcome | Proving boundary | State |
| --- | --- | --- | --- |
| M12 | Run static checks and four responsibility-based Rust partitions on isolated hosted runners | The compiled all-target/all-feature inventory belongs to exactly one partition; each partition remains case-serial, failures reach the stable aggregate gate, and workstation execution remains serial | Implemented on `codex/ci-rust-partitions`; 2,031 compiled tests accounted for; hosted mixed-load writer-admission delay under diagnosis |
| M13 | Organize catalog and unified-gallery tests by responsibility, moving fixtures with their actual users | Preserve every test, assertion, ignore and lifecycle guard; verify body equivalence and full inventory, with focused checks and hosted coverage | Implemented on `codex/catalog-gallery-test-ownership`; 111 Rust cases and 43 widget cases relocated, body/inventory preservation and local widget checks passed; hosted checks pending |

M12's initial partition is schema migrations plus change queue, remaining catalog tests,
synchronization plus scan tests, and all remaining Rust tests. The baseline log attributes roughly
9–13 minutes of test work to each group; actual hosted wall time must be measured after implementation.
No new test framework, dependency, reduced coverage, raised deadline or shared-runner concurrency is
included. M13 moves single-owner helpers beside their tests and keeps only genuinely shared fixtures
in support modules; file size alone does not define the split.

Concurrency is limited to separate GitHub-hosted runners. All local builds, tests and verification
retain the repository mutex and execute sequentially, including inventory checks and focused
regressions. The workstation Daily default remains the complete serial suite; hosted partition
selection does not authorize local fan-out.

The mixed-load diagnosis identified automatic WAL checkpoint work inside a committing writer.
M12 also owns the focused connection-policy repair and its durability/read-snapshot regressions;
the [acceptance record](../acceptance/maintainability-refactor.md#automatic-checkpoint-burst-attribution)
retains the measured cause, separate slow-storage observations and remaining hosted checks.
Keep the workload, P0 latency bound and shutdown deadline unchanged.

## Completed first-round delivery queue

Each row may contain independently reversible commits. Commit and push a finished boundary before
starting the next independent one. Pending hosted checks do not prevent unrelated authorized work.
Further diagnostics and CI corrections use the existing review units rather than adding PRs.
Native stack #35 contained six refactor PRs and one combined follow-up PR. The behavior changes below
retain separate outcomes and evidence within that combined review unit.

| Order | Outcome and affected owner | Required behavior-preservation evidence | State |
| --- | --- | --- | --- |
| M1 | Extract shared schema-shape checks, historical contract/row validators and exact compatibility repair from `sqlite_catalog/migrations.rs`; preserve the ordered migration facade and move tests with each owner | Accepted/rejected schema shapes, historical upgrades, quoted SQL semantics, repair admission, idempotence and transactional rollback | Implementation, scoped review and final exact-head hosted checks passed |
| M2 | Separate synchronization registry/start-stop ownership, then Journal/Recovery lane state from `library_synchronization/production.rs`; reuse existing Live and scheduling owners; remove the unused reconnect seam | Late completion, start/stop races, retained ownership after timeout or panic, one shutdown deadline, lane priority and root fairness | Implementation, scoped review and final hosted checks passed |
| M3 | Extract synchronization revision refresh and recovered-dimension publication lifecycles from `unified_library_screen.dart` | Superseded refresh, notification lifetime, coalescing/deadline, scrolling during dimension recovery, disposal and retained position | Implementation, scoped review, final hosted checks and affected Debug comparison passed |
| M4 | Give gallery retained pages one bounded merge/cursor/trim owner behind `library_viewport_controller.dart`; keep query and publication authority distinct | Overlapping pages, forward/backward eviction, stale completions, empty pages and viewer navigation across the window | Implementation, scoped review, final hosted checks and affected Debug comparison passed; retained observation is recorded below |
| M5 | Map the test-only USN catch-up path to current broker coverage; migrate remaining useful regressions before retiring redundant code | Preserve parsing, identity, ordering and continuity scenarios; retain any uncovered historical obligation explicitly rather than deleting it | Implementation, scoped review, migrated boundary/guardrail checks and final hosted checks passed |
| M6 | Separate native-resource, transaction-recovery and execution responsibilities in the release and R2c-R common scripts; retain canonical entrypoints | Positive and rejecting guardrails, source-audit closure, interrupted recovery, process retirement, exact-path/no-reparse safety and cleanup | Implementation, focused guardrails, combined client comparison, final scoped audit and final hosted checks passed |
| Follow-up A | Correct position drift across consecutive thumbnail-size changes before R4; keep it separate from the behavior-preserving refactors | Partially visible first rows, top/bottom and short galleries, small/medium/large round trips in both layout modes and rendering paths; scrolling, supersession and navigation release the retained anchor | Implementation, 31 focused checks, Debug middle/edge interaction, scoped review and 11 hosted execution jobs passed |
| Follow-up B | Preserve browsing intent when filtering temporarily removes or clamps the original anchor, then conditions expand again | Filter narrowing/expansion, missing/reappearing identity, edge clamping, empty results and intervening scroll/navigation; current publication authority remains unchanged | Implementation, 37 focused checks, both native layouts, accessibility preparation and all 11 consolidated hosted jobs passed |
| Follow-up C | Stabilize timeline marks during catalog updates without delaying direct navigation | Complete-layout/estimated-layout transitions, revision replacement, unavailable geometry, first load and changed scope; retained paint cannot grant stale input authority | Native update observation and 31 relevant focused cases passed; date-publication cancellation corrected; all 11 required hosted jobs passed on `db1842a` |
| Follow-up D | Diagnose and correct excessive journal catch-up time after image processing has finished | Separate volume read, identity resolution, catalog work and scheduling delay; measure bounded backlog progress and incoming work; preserve continuity, cancellation, source safety and browsing priority | Unbounded namespace lookup work corrected; three focused regressions, same-interval native measurement, Debug convergence, scoped review and all 11 required hosted jobs passed on `79213fc` |

## Verification and closeout

- Hosted CI owns routine formatting, lint, Rust/Flutter tests, bridge checks and applicable native
  gates. Inspect each pushed source commit; retain pending and failed results explicitly.
- Use focused local diagnosis for unresolved boundary questions. Do not rerun the full hosted suite
  locally. Record production, inline-test and dedicated-test sizes for large changed owners.
- M3/M4 require actual Debug interaction with the existing `build/test` library: sidebar rapid
  toggling, maximize/restore, scroll and resize, small/medium/large card sizes in both layout modes,
  synchronization refresh, duplicate review/preview round trip and preferred-copy changes. Preserve
  and restore the user's active profile and catalog.
- Before closeout, repeat the affected existing user workflows on the final candidate and compare
  against the retained baseline: navigation, input response, motion, loading feedback, scroll/selection
  retention, preview behavior and relevant bounded resource/performance results. Record regressions
  and resolve their causes; report improvements only with observed evidence. Intermediate boundary
  checks alone do not close this final comparison.
- M6 guardrails must prove passing and rejecting cases. No service installation, signing or release
  publication is included.
- Review the final changes for behavior, async/resource authority, data preservation, dependency
  changes, human readability, clear module organization and unnecessary abstractions.
- After the ordered implementation is finished, create the native GitHub
  [stacked pull requests](https://docs.github.com/en/rest/pulls/stacks) review surface. Keep completed
  responsibility groups on sequential branches/PRs so their changes remain separately reviewable;
  the existing branch and PR #22 own M1. Preserve published history when creating later branches.
  Verify repository admission at stack creation. Each branch and the combined candidate must have applicable
  passing checks and resolved relevant findings before it becomes a merge candidate. A stack is
  a review surface, not acceptance evidence. Merging into `main` remains an explicit human decision.
  Deliver the stack and final client/CI/review evidence, including any unresolved limitation.

## Delivery checkpoint

The [final comparison and review](../acceptance/maintainability-refactor.md#final-comparison-and-review)
records the combined Debug result, final implementation heads, hosted runs and audit scope.
Earlier per-boundary evidence remains in that record; completed extractions are not a new queue.

- Final implementation heads are M1 `ef294ad`, M2 `6f80d66`, M3 `4335ae7`, M4 `86feb69`,
  M5 `e4096a2` and M6 `976dee8`. Published history is preserved across the six branches.
- The combined comparison against `83c8c16` used the same 10,000-image catalog snapshot and
  durable choices. Stable browsing, concurrent catalog publication, preview, duplicate choices,
  size changes and sidebar/window transitions were exercised. Original profiles and catalogs were
  restored and verified after each run; final candidate Debug output remains available.
- The baseline square-card shift is corrected by Follow-up A, including explicit top/bottom rules;
  its [focused evidence](../acceptance/maintainability-refactor.md#card-size-position-follow-up) is separate
  from the behavior-preserving refactors. The earlier M4 active-rehash return observation did not recur in the paired flows;
  its exact timing remains unclassified. These observations are recorded without claiming a fix.
- The final scoped review found no confirmed new defect. No dependency, schema, generated bridge
  or workflow-definition change was introduced. The untracked `devtools_options.yaml` remains
  outside the owned submission.
- M1–M6 and the consolidated follow-up each passed all 11 required jobs. Their exact source commits
  and runs are recorded in the [final stack verification](../acceptance/maintainability-refactor.md#final-stack-verification).
  The combined result covers `aa3f786`; subsequent closeout edits only record these results and
  preserve verified product/tool inputs. Current-PR links below expose checks on later record commits;
  these edits use document checks and do not claim a new client run.
- Earlier stack runs exposed mixed-load poll-connection priority failures: M2 run
  `39020716332` timed out awaiting sample 11's P2 source page; M6 run `39020913937` measured P0 P95
  at 1.962 seconds and M4 run `39020716478` at 1.188 seconds against the unchanged one-second bound.
  Diagnostic run `39031367037` passed on `d6959e1` and recorded 51 slow namespace resolutions
  of 100–574 ms, with no held-handle identity observation above the 100 ms reporting threshold.
  The correction resolves only the differing disk-path prefix afresh; the complete held-file path
  and identity remain freshly checked. Its 11 focused namespace/identity regressions and controlled
  mixed-load case passed. Exact-head run `39036117005` passed all 11 required jobs on `ff1d78f`,
  including production P0 P95 of 435 ms with P1/P2 progressing in all 25 samples. This addresses
  the measured extra work without treating a passing run as an explanation of every older timeout.
- Follow-up B's Accessibility job reached its eight-second deadline while loading UIA components;
  application-ready element checks had not completed. A separately bounded preparation probe now
  validates the runtime before application startup; local guardrails and the complete hosted
  Accessibility job passed on `29c3c8b`. Its correction is included in consolidated PR #29, whose
  current checks own integration verification. Follow-up C's released date-seek cancellation
  regression is corrected and run `39029121263` passed all required jobs. Existing assertions,
  timeouts and workloads remain unchanged.
- Native GitHub stack #35 contained the six refactor PRs and combined follow-up PR #29 below.
  PRs #30–#34 were closed without merging after their complete history was retained by #29;
  their branches were retained until the authorized post-merge cleanup. The former stack #28 was
  unstacked, with no history rewriting or new PR.
  The seven review units retained adjacent base/head branches before the authorized merge on
  2026-10-18. The current maintenance slice above owns subsequent work; R4 has not started.

| Boundary | Review and current checks | Base branch |
| --- | --- | --- |
| M1 | [PR #22](https://github.com/Cedarflake/Cedarflake-Ame/pull/22) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/22/checks) | `main` |
| M2 | [PR #23](https://github.com/Cedarflake/Cedarflake-Ame/pull/23) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/23/checks) | `codex/maintainability-refactor` |
| M3 | [PR #24](https://github.com/Cedarflake/Cedarflake-Ame/pull/24) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/24/checks) | `codex/synchronization-runtime` |
| M4 | [PR #25](https://github.com/Cedarflake/Cedarflake-Ame/pull/25) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/25/checks) | `codex/gallery-publication-lifecycles` |
| M5 | [PR #26](https://github.com/Cedarflake/Cedarflake-Ame/pull/26) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/26/checks) | `codex/gallery-retained-window` |
| M6 | [PR #27](https://github.com/Cedarflake/Cedarflake-Ame/pull/27) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/27/checks) | `codex/retire-legacy-usn` |
| Follow-ups A–D, catalog proof and accessibility preparation | [PR #29](https://github.com/Cedarflake/Cedarflake-Ame/pull/29) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/29/checks) | `codex/workflow-script-ownership` |
