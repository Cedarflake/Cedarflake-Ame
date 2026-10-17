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
| `migrations.rs` before | 9,763 | 5,357 | 0 |
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

## Queue lane and recovery authority contracts

The lane projection and P2 recovery-authority proofs now have separate owners, each with direct
structural/row validation and its existing acceptance/rejection tests. Historical DDL remains with
the ordered version steps. All six moved function bodies preserve their decisions, SQL and error
codes after symbol relocation and formatting; origin precedence, missing-lane rejection, authority
identity and canonical journal boundaries are unchanged.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 8,985 | 4,978 | 0 |
| `migrations/change_lane_contract.rs` | 129 | 2 | 59 |
| `migrations/recovery_authority_contract.rs` | 144 | 2 | 117 |

Three existing regressions moved with their owners. This mechanical extraction uses hosted Rust
compilation and tests; no new local runtime pass is claimed. Source-body comparison and final diff
inspection preserve migration admission order and every affected test assertion.

## Inventory and terminal evidence contracts

Inventory shape and row relationships now belong to `metadata_inventory_contract`; terminal media
evidence belongs to `terminal_media_contract`. Five production function bodies preserve SQL, error
codes, version-dependent fields and validation depth after symbol relocation and formatting. Six
existing rejection regressions moved alongside these owners. The shared v19 test-catalog factory
remains a test-only fixture built through the original schema creator.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 8,661 | 4,829 | 0 |
| `migrations/metadata_inventory_contract.rs` | 255 | 2 | 124 |
| `migrations/terminal_media_contract.rs` | 90 | 2 | 27 |
| `migrations/legacy_test_catalog.rs` | 0 | 0 | 11 |

Source comparison and owned-file diff inspection preserve every moved assertion and the original
migration transaction/admission boundaries. Compilation and the moved tests are assigned to PR CI;
this record makes no new local runtime claim.

## Recovery, namespace and spool contracts

Recovery execution, configured-root namespace, live-gap consumer ownership and spool shape/version
now have separate proof owners. Current-schema and compatibility repair import the owning proof
directly. Eleven function bodies preserve their SQL, validation depth, error codes and decisions
after symbol relocation and formatting. The existing spool-row validator remains unchanged.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 7,855 | 4,384 | 0 |
| `migrations/recovery_execution_contract.rs` | 234 | 2 | 99 |
| `migrations/root_namespace_contract.rs` | 165 | 0 | 0 |
| `migrations/live_gap_contract.rs` | 244 | 0 | 0 |
| `migrations/inventory_spool_contract.rs` | 217 | 0 | 0 |
| `migrations/recovery_test_catalog.rs` | 0 | 0 | 363 |

Five frontier/reopen regressions moved with execution proof. Shared recovery and historical-version
fixtures moved without changing their SQL or assertions; baseline tests now import the fixture
owner directly. Cross-version migration and multi-contract recovery regressions remain integration
tests in the coordinator.

Local Cargo filter `recovery_execution_contract::tests`: 5 passed, 0 failed, 0 ignored, using the
repository lock and `CARGO_INCREMENTAL=0`. The successful test build includes all moved contracts
and fixture consumers. Hosted CI retains the complete historical-upgrade and recovery workload.

## Catch-up, handoff and historical scan ownership

Checkpoint/lineage proof, batch handoff proof, v19 compatibility repair and v17/v18 scan authority
now have four owners. The twelve moved function bodies preserve all query decisions, bounds,
transaction acquisition/rechecks and migration admission points. Twenty existing boundary tests
moved without changing their assertions; the v19 factory remains shared and test-only.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 6,632 | 3,722 | 0 |
| `migrations/catch_up_contract.rs` | 396 | 2 | 236 |
| `migrations/scan_handoff_contract.rs` | 199 | 2 | 65 |
| `migrations/catch_up_repair.rs` | 448 | 2 | 166 |
| `migrations/scan_owner_contract.rs` | 216 | 2 | 207 |

Local Cargo filter `catch_up_repair::tests`: 4 passed, 0 failed, 0 ignored, under the repository lock
with `CARGO_INCREMENTAL=0`. The clean run followed removal of two unused coordinator imports. All
test targets compiled; the remaining moved regressions and integration paths retain hosted ownership.

## Journal schema, row and payload proof

`journal_contract` owns structural admission and ordered row checks. Its child modules group schema,
state, lineage, payload relationships and canonical values. The 28 relocated function bodies retain
their decisions and SQL after name/format normalization; the three canonical DDL arrays are
byte-identical. Normal persistence writes call the same lineage/payload proofs at the same transaction
points. No schema version, accepted state, repair rule or publication authority changed.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 4,595 | 3,592 | 0 |
| `migrations/journal_contract.rs` | 260 | 2 | 133 |
| `journal_contract/schema.rs` | 332 | 0 | 0 |
| `journal_contract/rows.rs` | 198 | 0 | 0 |
| `journal_contract/state_rows.rs` | 241 | 0 | 0 |
| `journal_contract/lineage.rs` | 344 | 0 | 0 |
| `journal_contract/payload_children.rs` | 187 | 0 | 0 |
| `journal_contract/payload_intents.rs` | 217 | 0 | 0 |
| `journal_contract/lineage_payload.rs` | 220 | 0 | 0 |
| `journal_contract/values.rs` | 118 | 0 | 0 |

Local Cargo filter `journal_contract::tests`: 3 passed, 0 failed, 0 ignored under the repository
lock with `CARGO_INCREMENTAL=0`. This covers malformed range shape, same-count replacement CHECK
constraints and quoted SQL mutations. All test targets compiled without warnings; the complete
journal write and historical migration workload retains hosted CI ownership.

Hosted run `39008498192` for `0747bfa` rejected the coordinator's `schema_version` import in its
non-test library build. That symbol was used only by a test. The test now refers to `schema_shape`
directly and the ordinary library import is removed; no warning suppression or gate change is used.
The focused ordinary-library `cargo check --lib --locked` then completed without warnings.

## Exact compatibility repair ownership

Terminal inventory cleanup, retired-gap residue, interrupted explicit scan recovery and exact
prerelease Journal shape repair have separate owners. Eight function bodies retain the original
admission predicates, SQL, transaction acquisition and validation points. The existing
`current_recovery_state` coordinator still applies related repairs atomically. Seventeen existing
regressions and their assertions moved with the responsible owners; shared setup now lives beside
the repair it represents. Full catalog reopen tests continue to cover interrupted foreground
claim restoration, handoff-owned assets and cleanup rollback.

| Owner | Production/support lines | Inline test-only lines | Dedicated test/fixture lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after | 4,124 | 2,406 | 0 |
| `terminal_inventory_repair.rs` | 84 | 4 | 523 |
| `retired_live_gap_repair.rs` | 131 | 4 | 623 |
| `interrupted_foreground_repair.rs` | 156 | 0 | 0 |
| `prerelease_journal_repair.rs` | 116 | 2 | 52 |
| `current_recovery_state.rs` | 77 | 2 | 25 |

Cargo filter `repair::tests` passed 24 tests, including the moved terminal, retired-gap and
prerelease cases. Three unused relocation imports were then removed without changing test bodies.
The clean rebuild and `current_recovery_state::tests` passed the combined inventory/gap repair
regression (1 passed, 0 failed). All local checks used the repository lock and disabled incremental
cache; hosted CI retains full upgrade/reopen coverage.

## M1 proof-order ownership and scoped review

`current_schema` now owns both historical prerequisite proof sequences and the audit instrumentation.
Contract validators reference this owner directly; the migration coordinator retains version
dispatch, stable DDL and upgrade transactions. Four relocated proof-order bodies are unchanged
after name/format normalization. The two admission-order regressions moved with the owner.

| Owner | Production/support lines | Inline test-only lines | Dedicated test lines |
| --- | ---: | ---: | ---: |
| `migrations.rs` after M1 | 4,026 | 2,347 | 0 |
| `current_schema.rs` | 238 | 29 | 36 |

The complete baseline migration subtree contains 163 named tests. All remain after M1, with eight
added boundary regressions (171 total). The parent file's total size changes from 15,120 to 6,373
lines while historical integration tests remain beside version dispatch. Counts include whitespace;
final counts assign complete `cfg(test)` items to test-only lines and retain intervening whitespace
in production/support. The baseline classification above corrects a one-line attribution difference.

The clean focused build and `current_schema::tests` passed 2 tests. An initial missing direct
`ContractValidationDepth` import in the baseline validator was corrected at its actual owner.
No assertions, warning rules or check workloads were relaxed.

Scoped review covered unchanged migration order and SQL, exact repair admission, same-transaction
rollback, consistent read snapshots, direct normal-write proof reuse, test/fixture ownership and
absence of dependency/schema/source-read changes. It found no remaining M1 implementation issue.
This is source and boundary evidence; the final pushed head still requires its hosted gates, and
the combined refactor retains the final client comparison in the execution plan.

## M2 runtime lifecycle boundary

`production/runtime_lifecycle` owns registry state, start/stop admission, operation completion and
retirement. The existing Windows command facade translates raw tickets through this owner; mutable
registry state stays internal. The 32 moved function bodies retain their predicates, lock order,
deadline calculations, result admission and panic handling after formatting. Lifecycle fixture
resources are shared through the test-only `runtime_test_support` owner.

| Owner | Production/support lines | Inline test-only lines | Dedicated test/fixture lines |
| --- | ---: | ---: | ---: |
| `production.rs` before | 3,507 | 10,884 | Existing child suites |
| `production.rs` after lifecycle extraction | 2,345 | 8,079 | Existing child suites |
| `runtime_lifecycle.rs` | 46 | 2 | 169 shared lifecycle fixture lines |
| `runtime_lifecycle/state.rs` | 360 | 10 | 0 |
| `runtime_lifecycle/admission.rs` | 298 | 22 | 727 |
| `runtime_lifecycle/operation.rs` | 273 | 24 | 836 panic and 367 late-completion lines |
| `runtime_lifecycle/retirement.rs` | 229 | 25 | 371 shutdown and 74 catalog-close lines |
| `runtime_test_support.rs` | 0 | 0 | 208 |

All 89 tests previously in the parent and its poll-catalog suite remain, with their assertions
unchanged. The relocated lifecycle subset contains 39 tests. The scoped local run passed 37 and
encountered filesystem access denial in two temporary-catalog rename assertions. Running those
same two tests with normal permissions passed both, including delayed close, retained ownership,
one-time retirement and post-close rename. No test assertions or time limits changed. The final
ordinary-library check passed without warnings after limiting parent panic/condition-variable
imports to their actual test configuration. Complete cross-platform and integration gates remain
hosted; Journal/Recovery extraction and final client comparison are still outstanding.

The first M2 hosted run (`16680d1`, run `39010460085`) rejected two stale R2c-R test mappings after
the shutdown cases moved to `runtime_lifecycle/tests/shutdown.rs`. The exact selectors and source
paths now target that owner, including the existing acceptance table. A focused matrix check
admitted all 19 cases and rejected the former source mapping. No cases, assertions or rejection
rules were removed; the corrected commit still requires its hosted checks.

## M2 Journal selection and worker ownership

The coordinator retains Live/stopping admission and status publication. `JournalLane` owns root and
volume cursors, read/drain alternation and capacity-debt selection. Its branch order remains ready
Journal drain, session availability, recovery admission, closing boundary, opening boundary, then
bounded volume read. Missing caller/session, unhealthy/unavailable roots, insufficient capacity and
the intervening drain turn retain their prior outcomes. Cursor updates still precede catalog-session
validation and worker spawn; a failed spawn does not silently rewind fairness.

`JournalTask` owns a typed opening, closing, page or drain worker and its result/cancellation/join
lifetime. Page registration guards remain held through capture, cancellation retains its atomic
flag, and shutdown receives the original absolute deadline. The original registration-failure and
volume-selection regressions now sit with their owners. Controlled lifecycle tests use the task's
test-only construction/progress boundary instead of accessing its mutable internals.

| Owner | Production/support lines | Inline test-only lines | Dedicated test/fixture lines |
| --- | ---: | ---: | ---: |
| `production.rs` after Journal extraction | 1,897 | 7,831 | Existing child suites |
| `journal_lane.rs` | 214 | 7 | Existing volume, admission and priority suites |
| `journal_work.rs` | 300 | 22 | Existing lifecycle suite |
| `journal_work/page.rs` | 112 | 2 | 110 |
| `journal_volume.rs` | Unchanged | Unchanged | 489, including the relocated selection test |

All 146 tests in the production subtree remain. Normal-library compilation passed without warnings.
The moved registration-failure regression, eight volume/admission regressions and six shutdown
regressions passed. The simultaneous P0/P1/P2 priority case initially failed at the Windows namespace
guard under sandbox permissions. The unchanged case passed with normal permissions, including full
recovery publication and reopen; its 25 P0 samples measured 88 ms P95 and 89 ms maximum while both
lower lanes advanced. This is a controlled fixture result, not a before/after performance claim.
Full hosted checks and the final client comparison remain separate.

## M2 Recovery scheduling ownership

`RecoveryLane` now owns root rotation and the bounded per-root exponential retry schedule. The
coordinator keeps the Live/Journal gate and passes the selected Journal capacity-debt roots. The
selection predicates, branch order, cursor publication and saturating time arithmetic match the
previous implementation after owner qualification and formatting. Candidate drains still precede
legacy debt, unresolved candidates block a new control page, and debt restriction cannot grant a
new recovery authority.

The retry and fair-rotation tests moved beside this owner; their former whole-runtime fixture is
unnecessary for these rules. The production completion regression still proves that one successful
root cannot clear another root's retry record. All three focused cases and ordinary-library
compilation passed. No queue policy, retry bound, source operation or published contract changed.

| Owner | Production/support lines | Inline test-only lines | Dedicated test/fixture lines |
| --- | ---: | ---: | ---: |
| `production.rs` after Recovery scheduling extraction | 1,747 | 7,587 | Existing child suites |
| `recovery_lane.rs` | 188 | 8 | 216 |

Recovery task/resource ownership and the unused reconnect seam remain in the M2 queue. Hosted
checks for the next pushed commit and final client comparison remain required.

## M2 retained Recovery sources

`RecoverySources` owns retained inventory sources by immutable change ID, current authority/run
pruning and result publication. `RecoverySourceTransfer` binds the original change ID to the shared
single-take operation, including the existing poison error; failed startup restores only that
transfer's owner. Catalog-session validation still precedes source removal, non-inventory work
cannot consume a retained source, and pruning performs all fallible authority reads before removal.
These ownership moves preserve the previous call order and synchronization primitive.

The four existing regressions moved with their controlled source/lease fixtures. They passed for
catalog-validation failure, worker-spawn failure restoration, different-change isolation and missing
authority pruning. Ordinary-library compilation passed without warnings. The production debt test
continues to inspect the same empty-source invariant through read-only test access.

| Owner | Production/support lines | Inline test-only lines | Dedicated test/fixture lines |
| --- | ---: | ---: | ---: |
| `production.rs` after retained-source extraction | 1,708 | 7,374 | Existing child suites |
| `recovery_sources.rs` | 90 | 8 | 223 |

Recovery worker lifetime and reconnect retirement remain pending; this boundary does not change
source reads, durable authority, publication policy or shutdown timing.

## M2 Recovery worker ownership

`RecoveryTask` owns its private thread, cancellation, phase, receiver and deadline-bound retirement.
Automatic recovery and candidate/legacy drains live beside it; slot admission, catalog validation,
status publication and retry scheduling remain in the coordinator. Root generation, continuity
revision and change identity remain distinct. The automatic worker still takes its retained source,
opens the Recovery lane, reloads root authority, then defers or processes the same bounded work.
Flattening the nested result closures preserves that error and resource-lifetime order.

All 146 production-subtree tests remain. The 39 lifecycle regressions passed with normal Windows
permissions, including delayed and panicked worker retirement. Four source-transfer cases, the two
relocated stop/timeout cases, three active-phase projection cases and ordinary-library compilation
passed. Source transfer still rolls
back on thread creation failure, and a timed-out worker remains owned until a later successful join.
The final client comparison and exact-commit hosted gates remain separate.

| Owner | Production/support lines | Inline test-only lines | Dedicated test/fixture lines |
| --- | ---: | ---: | ---: |
| `production.rs` after Recovery worker extraction | 1,374 | 7,229 | Existing child suites |
| `recovery_work.rs` | 146 | 53 | 126 |
| `recovery_work/automatic.rs` | 197 | 14 | Existing source-transfer and integration suites |
| `recovery_work/drain.rs` | 145 | 0 | Existing priority and lifecycle suites |

## M2 unused reconnect retirement

The old same-runtime reconnect method had no production callers and replaced its session even when
close failed. It and its obsolete dead-code allowance are removed. Production behavior remains the
existing lifecycle: successful retirement admits a fresh factory session; a failed close stays
owned, caches its result and prevents another construction epoch. The Windows transport's bounded,
authenticated service handover remains unchanged.

Two current-path regressions replace the two tests of that dormant method. Both passed: a second
factory session opens only after the first runtime retires, and repeated stop/start attempts after
a close failure neither close twice nor connect a replacement. The production subtree still has
146 tests. The R2c-R matrix retains 19 cases with its two session selectors and descriptions updated;
the matrix check passes and rejects a stale source path. No live-close policy was relaxed.

Ordinary-library compilation and formatting passed after restricting the retired method's error
import to its remaining test callers. The parent now has 1,342 production/support and 7,157 inline
test-only lines. Shared controlled
resource fixtures contain 224 dedicated test lines; the session lifecycle suite contains 89.
The remaining inline integration suites still require organization by boundary before M2 closeout.

## M2 integration ownership and scoped review

The remaining integration tests now have explicit startup, first-import, baseline, live-gap,
recovery-state/authority/fairness, priority, inventory-page, legacy-capacity and publication-guard
owners. Shared controlled sources and durable-gap fixtures remain in `production/tests.rs`.
No production behavior changed in this final organization step. All 39 moved function bodies match
after owner qualification and formatting, including the formatter's removal of one trailing comma.
All 146 test names and existing attributes remain. The full test binary compiled and both current
session-lifecycle cases passed; the updated 19-case acceptance matrix passes and rejects a stale
source owner. The remaining regression execution belongs to the exact-commit hosted suite.

| Final M2 owner | Production/support lines | Inline test-only lines | Dedicated test/fixture lines |
| --- | ---: | ---: | ---: |
| `production.rs` | 1,342 | 299 | Boundary suites below |
| `production/tests.rs` | 0 | 0 | 928 shared fixtures, declarations and clock case |
| Startup, first import and journal baseline suites | 0 | 0 | 495, 355 and 259 |
| Live-gap, recovery authority and recovery status suites | 0 | 0 | 773, 308 and 243 |
| Inventory pages and publication guards | 0 | 0 | 936 and 521 |
| Priority admission, recovery fairness and legacy capacity debt | 0 | 0 | 572, 527 and 922 |

Scoped review compared lifecycle admission/retirement, Live/Journal/Recovery precedence, per-root
retry and fairness, source transfer and result publication with M2's base. The coordinator retains
cross-lane policy; extracted owners hold private mutable state and typed immutable inputs. Existing
shutdown deadlines, capacity/page limits, root/change/run identities, error outcomes and source-read
authority remain. The unused reconnect path was removed without altering the active fail-closed
session policy. No dependencies, schema, bridge or UI contracts changed. M2 implementation and scoped
review are complete; final-head CI and the combined client comparison remain outstanding.

## Hosted M1 checkpoint

Final M1 commit `ef294adf82e15de82618c0cdf4a56f7bf7076b48` passed all applicable jobs in
[run 39009808560](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39009808560).
Signing-dependent jobs retain their configured skips. M2's final `6f80d66` run remains in progress;
the combined client comparison remains part of goal closeout.

## M3 synchronization refresh lifecycle

`LibrarySynchronizationRefresh` owns revision coalescing, one in-flight attempt, delayed retry and
failure-feedback transitions. The screen retains synchronization subscriptions, root feedback,
notification wording and action routing; the application still owns query admission and catalog
publication. The same 250-ms busy/superseded delay and failure retry policy remain. Disposal is an
explicit owner boundary, so a late result cannot read a retired screen or publish feedback.

Nine focused cases passed for highest-revision selection, queued revisions during failure,
explicit retry and deduplicated feedback, busy/superseded delay, browse admission, published-revision
proof and disposal before/during an attempt. The 20 existing viewer/refresh cases also passed,
including failure notification/retry, root-warning resolution, scan-feedback precedence, rename,
authoritative removal and stale viewer lookup. No widget layout, strings or query authority changed.

The screen is now 1,924 production lines, the refresh owner 123, and its dedicated tests 218.
Recovered-dimension ownership and actual Debug comparison remain pending in M3; these focused
results do not establish final client or hosted acceptance.
