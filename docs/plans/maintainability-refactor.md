# Maintainability refactor execution plan

Status: active maintenance, selected by the [roadmap](../roadmap.md#current-execution-queue)

Updated: 2026-10-17

Branch: `codex/maintainability-refactor`

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
| M1 | Extract shared schema-shape checks, historical contract/row validators and exact compatibility repair from `sqlite_catalog/migrations.rs`; preserve the ordered migration facade and move tests with each owner | Accepted/rejected schema shapes, historical upgrades, quoted SQL semantics, repair admission, idempotence and transactional rollback | Implementation and scoped review complete; final exact-head hosted checks pending |
| M2 | Separate synchronization registry/start-stop ownership, then Journal/Recovery lane state from `library_synchronization/production.rs`; reuse existing Live and scheduling owners; remove the unused reconnect seam | Late completion, start/stop races, retained ownership after timeout or panic, one shutdown deadline, lane priority and root fairness | Pending M1 |
| M3 | Extract synchronization revision refresh and recovered-dimension publication lifecycles from `unified_library_screen.dart` | Superseded refresh, notification lifetime, coalescing/deadline, scrolling during dimension recovery, disposal and retained position | Pending M2 |
| M4 | Give gallery retained pages one bounded merge/cursor/trim owner behind `library_viewport_controller.dart`; keep query and publication authority distinct | Overlapping pages, forward/backward eviction, stale completions, empty pages and viewer navigation across the window | Pending M3 |
| M5 | Map the test-only USN catch-up path to current broker coverage; migrate remaining useful regressions before retiring redundant code | Preserve parsing, identity, ordering and continuity scenarios; retain any uncovered historical obligation explicitly rather than deleting it | Pending M4 |
| M6 | Separate native-resource, transaction-recovery and execution responsibilities in the release and R2c-R common scripts; retain canonical entrypoints | Positive and rejecting guardrails, source-audit closure, interrupted recovery, process retirement, exact-path/no-reparse safety and cleanup | Pending M5 |

## First boundary: schema-shape validation

The shared owner will retain the current table-column, index, foreign-key and normalized SQL
comparisons, including version-aware scan-handoff columns. It must not widen accepted schemas,
change quoted tokens, add queries per catalog row or change the order of migration/repair admission.
Existing callers should use this owner directly. Focused regressions cover both valid shapes and
the malformed boundaries that distinguish them.

Subsequent M1 boundaries are historical contract families and narrowly admitted repairs. Keep stable
DDL/version steps in place unless they belong to an extracted invariant. A single relocated migration
monolith, shared mutable bag or forwarding facade does not complete M1.

## Verification and closeout

- Hosted CI owns routine formatting, lint, Rust/Flutter tests, bridge checks and applicable native
  gates. Inspect each pushed source commit; retain pending and failed results explicitly.
- Use focused local diagnosis for unresolved boundary questions. Do not rerun the full hosted suite
  locally. Record production, inline-test and dedicated-test sizes for large changed owners.
- M3/M4 require actual Debug interaction with the existing `build/test` library: sidebar rapid
  toggling, maximize/restore, scroll and resize, synchronization refresh, duplicate review/preview
  round trip and preferred-copy changes. Preserve and restore the user's active profile and catalog.
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

## Resume checkpoint

Read the roadmap, this queue, the applicable ADR entries, then inspect Git status and current CI.
Preserve completed boundaries and still-valid evidence. Update this checkpoint after each delivery;
do not infer completion from moved files or recreate an already extracted owner.

- Branch created from `83c8c16`; no unrelated working-tree edits were present.
- M1 shared schema-shape extraction is delivered as `55b4cce`; preview-repair ownership as `607b561`.
  Source-revision metadata proof/repair is delivered as `20c0ed9`. Queue-lane and recovery-authority
  proofs are delivered as `7f9fae7`; inventory and terminal evidence as `e2d7c39`; recovery, namespace,
  live-gap and spool proofs as `4dfc6cf`. Catch-up/handoff proof and v18/v19 compatibility repair
  are delivered as `0747bfa` with 20 regressions. Journal proof is delivered as `781763f` with its
  three rejection regressions and normal-write callers. Terminal inventory, retired-gap, interrupted
  foreground and prerelease journal repairs are delivered as `9ef344c` with 17 existing regressions.
  Proof ordering and audit counters now belong to `current_schema`, with two admission-order
  regressions. M1 retains all 163 baseline migration tests and adds eight boundary cases. Final
  M1 CI remains pending; continue M2 on its sequential branch while those checks run.
- Authorized draft [PR #22](https://github.com/Cedarflake/Cedarflake-Ame/pull/22) triggers the existing
  hosted checks. Inspect the current pushed head; newer pushes supersede older in-progress runs.
  Feature-branch pushes alone are not CI triggers.
- [Verification evidence](../acceptance/maintainability-refactor.md) records checks and physical
  ownership counts. The overall refactor and final client comparison remain incomplete.
