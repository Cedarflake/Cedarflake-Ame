# Maintainability refactor evidence

Date: 2026-10-17

Scope and execution order belong to the [active plan](../plans/maintainability-refactor.md).
This record covers changed boundaries, not another R2c/R3 stage acceptance run.

## Shared schema-shape validation

The extraction preserves function bodies, query order, version-dependent scan-handoff columns,
case-insensitive declared types, exact ordered column/index comparisons, the single cascade foreign
key requirement and quoted SQL tokens. Contract/repair callers import the shared owner directly.
No schema version, migration order, dependency or source-read policy changes.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` before | 9,762 | 5,358 | 0 |
| `migrations.rs` after | 9,581 | 5,327 | 0 |
| `migrations/schema_shape.rs` | 193 | 2 | 165 |

Counts include whitespace. Test-only counts include `cfg(test)` hooks/declarations as well as the
inline test module; the two lines in `schema_shape.rs` declare its dedicated test module. Existing
tests of other migration owners are outside this table. One canonicalization regression moved and
five boundary regressions were added; migration integration tests remain in place.

Local verification:

- Scoped Cargo library filter `schema_shape::tests`, serialized through the repository tool lock:
  6 passed, 0 failed, 0 ignored. The clean run used `CARGO_INCREMENTAL=0` after the sandbox's
  incremental-cache hard-link attempt emitted a fallback warning; no lint or test rule was disabled.
- `quality_format.ps1 -Check`: passed, including all 410 Dart files unchanged. The sandbox attempt
  formatted successfully but could not write Dart's analytics session; the identical check passed
  with normal tool permissions.
- Final source/diff review and `git diff --check`: passed. Historical migrations, repair admission
  and the source catalog remain unchanged by this extraction.
- Delivery: `55b4cce`. Hosted verification now runs through draft
  [PR #22](https://github.com/Cedarflake/Cedarflake-Ame/pull/22); feature-branch pushes alone are not
  CI triggers. No hosted or client pass is claimed by this local record.

Remaining M1 work is historical contract/row validation and exact compatibility-repair ownership.

## Preview expectation compatibility repair

The marker proof, immediate transaction, post-lock marker recheck and scoped derived-data reset now
belong to `migrations/preview_expectation_repair.rs`. The migration coordinator retains the same
admission points. Two existing malformed-marker/concurrent-preflight regressions moved with the
owner. Two additional regressions prove current-publication selection and atomic rollback after
handoff/link changes but before location publication. A failed attempt can subsequently complete.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 9,451 | 5,285 | 0 |
| `migrations/preview_expectation_repair.rs` | 138 | 2 | 203 |

Local Cargo filter `preview_expectation_repair::tests`: 4 passed, 0 failed, 0 ignored, with the
repository lock and `CARGO_INCREMENTAL=0`. SQL changes in the test queries after that run only wrap
whitespace; implementation and assertions are unchanged. Delivery: `607b561`. Its
[hosted run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39006815585) was in progress
when the next boundary began.

## Source revision contract and metadata repair

`migrations/source_revision_contract.rs` owns structural and row proof, the completion marker and
the transaction that invalidates legacy metadata for all aliases. The ordered v30 migration and
subsequent spool/resume migrations call that owner directly. The six production function bodies
retain their decisions and SQL; changes are symbol ownership and formatting. No schema, migration
order, source identity or metadata invalidation policy changed.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 9,228 | 5,148 | 0 |
| `migrations/source_revision_contract.rs` | 228 | 16 | 200 |

Three existing structure/marker rejection and hardlink-alias regressions moved with this owner.
The additional regression rejects completion-marker publication after invalidation, proves both
aliases retain their previous metadata and previews after rollback, and successfully retries the
repair. The existing read-only reopen and audit-count expectations remain intact.

Local Cargo filter `source_revision_contract::tests`: 4 passed, 0 failed, 0 ignored, under the
repository lock with `CARGO_INCREMENTAL=0`. Initial compilation exposed two remaining forward-migration
callers of the old parent symbols; both now call the owner directly. The passing run includes those
callers. Repository formatting (410 Dart files unchanged), final source/diff inspection and
`git diff --check` passed. Hosted checks and final client comparison remain separate evidence.
