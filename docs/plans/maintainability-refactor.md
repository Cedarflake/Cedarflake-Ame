# Maintainability refactor execution plan

Status: active maintenance, selected by the [roadmap](../roadmap.md#current-execution-queue)

Updated: 2026-10-17

Branch: `codex/maintainability-refactor`

## Outcome and boundaries

Reduce the maintenance cost identified at baseline `83c8c16` by moving independent invariants and
lifecycles to their owning modules, consolidating duplicated window rules, and retiring superseded
implementation only after its useful coverage belongs to the current path. Preserve delivered R2c/R3
behavior and the accepted UI. R4 remains unstarted.

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
| M1 | Extract shared schema-shape checks, historical contract/row validators and exact compatibility repair from `sqlite_catalog/migrations.rs`; preserve the ordered migration facade and move tests with each owner | Accepted/rejected schema shapes, historical upgrades, quoted SQL semantics, repair admission, idempotence and transactional rollback | Shared schema-shape owner locally verified; remaining contracts/repairs pending |
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
- M6 guardrails must prove passing and rejecting cases. No service installation, signing or release
  publication is included.
- Review the final changes for behavior, async/resource authority, data preservation, dependency
  changes and unnecessary abstractions. Deliver the branch and the verified/remaining results.
  Merging into `main` is not part of this authorization.

## Resume checkpoint

Read the roadmap, this queue, the applicable ADR entries, then inspect Git status and current CI.
Preserve completed boundaries and still-valid evidence. Update this checkpoint after each delivery;
do not infer completion from moved files or recreate an already extracted owner.

- Branch created from `83c8c16`; no unrelated working-tree edits were present.
- M1 shared schema-shape extraction is locally verified; hosted verification follows the branch
  delivery. Historical contract/row validation and exact compatibility repair are the next boundary.
- [Verification evidence](../acceptance/maintainability-refactor.md) records checks and physical
  ownership counts. M1 and the overall refactor remain incomplete.
