# Maintainability refactor execution plan

Status: refactor and card-size correction locally verified; hosted checks and filter-position follow-up remain

Updated: 2026-10-17

Branches: M1 `codex/maintainability-refactor`; M2 `codex/synchronization-runtime`, based on M1 `ef294ad`;
M3 `codex/gallery-publication-lifecycles`, based on M2 `6f80d66`;
M4 `codex/gallery-retained-window`, based on M3 and including its `4335ae7` constructor correction;
M5 `codex/retire-legacy-usn`, based on M4 `86feb69`;
M6 `codex/workflow-script-ownership`, based on M5 `e4096a2`.
Follow-up: `codex/gallery-card-size-anchor`, based on M6 `1e87561`.

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

## Ordered delivery queue

Each row may contain independently reversible commits. Commit and push a finished boundary before
starting the next independent one. Pending hosted checks do not prevent unrelated authorized work.

| Order | Outcome and affected owner | Required behavior-preservation evidence | State |
| --- | --- | --- | --- |
| M1 | Extract shared schema-shape checks, historical contract/row validators and exact compatibility repair from `sqlite_catalog/migrations.rs`; preserve the ordered migration facade and move tests with each owner | Accepted/rejected schema shapes, historical upgrades, quoted SQL semantics, repair admission, idempotence and transactional rollback | Implementation, scoped review and final exact-head hosted checks passed |
| M2 | Separate synchronization registry/start-stop ownership, then Journal/Recovery lane state from `library_synchronization/production.rs`; reuse existing Live and scheduling owners; remove the unused reconnect seam | Late completion, start/stop races, retained ownership after timeout or panic, one shutdown deadline, lane priority and root fairness | Implementation, scoped review and final exact-head hosted checks passed |
| M3 | Extract synchronization revision refresh and recovered-dimension publication lifecycles from `unified_library_screen.dart` | Superseded refresh, notification lifetime, coalescing/deadline, scrolling during dimension recovery, disposal and retained position | Implementation, scoped review, final hosted checks and affected Debug comparison passed |
| M4 | Give gallery retained pages one bounded merge/cursor/trim owner behind `library_viewport_controller.dart`; keep query and publication authority distinct | Overlapping pages, forward/backward eviction, stale completions, empty pages and viewer navigation across the window | Implementation, scoped review and final hosted checks passed; combined comparison recorded with retained observations below |
| M5 | Map the test-only USN catch-up path to current broker coverage; migrate remaining useful regressions before retiring redundant code | Preserve parsing, identity, ordering and continuity scenarios; retain any uncovered historical obligation explicitly rather than deleting it | Implementation, scoped review, migrated boundary/guardrail checks and final hosted checks passed |
| M6 | Separate native-resource, transaction-recovery and execution responsibilities in the release and R2c-R common scripts; retain canonical entrypoints | Positive and rejecting guardrails, source-audit closure, interrupted recovery, process retirement, exact-path/no-reparse safety and cleanup | Implementation, focused guardrails, combined client comparison and final scoped audit complete; current stack checks are linked below |
| Follow-up A | Correct position drift across consecutive thumbnail-size changes before R4; keep it separate from the behavior-preserving refactors | Partially visible first rows, top/bottom and short galleries, small/medium/large round trips in both layout modes and rendering paths; scrolling, supersession and navigation release the retained anchor | Implementation, 31 focused checks, Debug middle/edge interaction and scoped review passed; hosted checks remain |
| Follow-up B | Preserve browsing intent when filtering temporarily removes or clamps the original anchor, then conditions expand again | Filter narrowing/expansion, missing/reappearing identity, edge clamping, empty results and intervening scroll/navigation; current publication authority remains unchanged | Current per-query capture inspected; focused diagnosis follows the independently published size correction |

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
- M1–M5 pre-stack implementation runs passed. Native stack creation triggered new M2–M6 checks
  and superseded M6's previous run. The linked current-PR checks own the changing integration status;
  closing the delivery requires their applicable jobs to pass. Documentation-only closeout preserves
  the verified product/tool inputs and does not claim a new client run.
- Native GitHub stack #28 contains the six draft PRs below, with verified ancestry and adjacent
  base/head branches. Each review unit remains independently inspectable. `main` merging remains
  an explicit human decision; R4 has not started.

| Boundary | Review and current checks | Base branch |
| --- | --- | --- |
| M1 | [PR #22](https://github.com/Cedarflake/Cedarflake-Ame/pull/22) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/22/checks) | `main` |
| M2 | [PR #23](https://github.com/Cedarflake/Cedarflake-Ame/pull/23) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/23/checks) | `codex/maintainability-refactor` |
| M3 | [PR #24](https://github.com/Cedarflake/Cedarflake-Ame/pull/24) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/24/checks) | `codex/synchronization-runtime` |
| M4 | [PR #25](https://github.com/Cedarflake/Cedarflake-Ame/pull/25) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/25/checks) | `codex/gallery-publication-lifecycles` |
| M5 | [PR #26](https://github.com/Cedarflake/Cedarflake-Ame/pull/26) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/26/checks) | `codex/gallery-retained-window` |
| M6 | [PR #27](https://github.com/Cedarflake/Cedarflake-Ame/pull/27) / [checks](https://github.com/Cedarflake/Cedarflake-Ame/pull/27/checks) | `codex/retire-legacy-usn` |
