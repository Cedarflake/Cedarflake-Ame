# Maintainability refactor evidence

Date: 2026-10-17

Scope and execution order belong to the [active plan](../plans/maintainability-refactor.md).
This record covers changed boundaries, not another R2c/R3 stage acceptance run.
The [final comparison and review](#final-comparison-and-review) owns the combined candidate result;
earlier pending statements below describe their delivery checkpoints.

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
The subsequent dimension boundary and client comparison are recorded below.

## M3 recovered-dimension publication

`LibraryGalleryDimensionPublication` owns pending, deferred and published dimensions, the frozen
visible-range anchor, query/revision invalidation, manifest overlay caching and publication timing.
The 160-ms quiet delay and 600-ms maximum delay remain unchanged. The screen retains actual scroll
objects, query-transition composition and the layout animation. Scrolling still retires query
position authority before dimension reclassification; transitions defer publication until completion.

Nine owner cases passed for quiet/deadline publication, anchor retention, cache reuse, offscreen
reclassification, query/revision replacement, transition admission, navigation and disposal. The
43 existing screen cases and two synchronization-position cases passed, including recovered batches,
sidebar/window geometry, timeline navigation and later input during refresh success/failure.
Source comparison preserves admission, callback ordering and timer cancellation. No UI structure,
copy, duration, catalog authority or dependency changed.

The screen now has 1,732 production lines, the dimension owner 265, and its dedicated tests 244;
none contains inline tests. Exact-head hosted checks remain outstanding.

### M3 affected Debug workflow

The ordinary Windows Debug build with both M3 owners used the retained 10,000-file `build/test`
corpus and its catalog. The previous restored profile receipt was archived before creating a new
lease against the current original settings/catalog. No original catalog snapshot was substituted
for later user changes. The test profile retained a 256-MiB preview budget and separate derived storage.

Observed scrolling during synchronization publication, sidebar collapse and rapid repeated toggles,
maximize/restore at two window sizes, duplicate group/member navigation, two preferred-copy changes,
member preview forward/back navigation, return to Gallery, then maximize/restore and eight rapid
sidebar clicks. The app remained usable and did not terminate. Preferred-copy changes retained the
member rows without replacing the page with loading feedback. Header and image leading edges still
followed sidebar changes; right controls remained at the window edge.

The retained catalog entered its existing `journal_trim` recovery path at startup. Catalog evidence
showed recovery retirement and all 30,008 retained queue records completed. Duplicate counts changed
as those source generations were reanalyzed; an opened obsolete group displayed its existing changed-
location state. Subsequent current-group preview and preferred-copy checks succeeded. This run
exercised concurrent publication; the final combined comparison must also cover stable results.

The client closed through its normal window action. No active thread/handle-owning Ame process
remained before restoration. Settings, preferences and original catalog components matched their
pre-run hashes. The corpus and retained test catalog remain available; no source mutation or cloud
hydration was requested. The run establishes the affected interaction check, not a measured speedup,
native error-log audit or final M1–M6 comparison.

## M4 retained catalog window

`LibraryRetainedCatalogWindow` now owns page insertion, replacement by location identity, boundary
cursor advancement and whole-page eviction. Its immutable result returns only the retained assets,
offset, cursors and added count. The viewport keeps read admission, query/revision validation,
publication generations, direction-specific loading/errors and completion semantics.

The extraction preserves these policy boundaries: incoming replacements use the last observation
for an existing location; overlap-only and empty pages advance only their directional cursor;
forward completion reports newly added items while backward completion reports a nonempty response.
Previous-page offsets keep the existing timeline clamp and missing-timeline fallback. Exactly 5,000
items do not trim; overflow removes whole distant pages until at most 3,500 remain, retaining at least
one page. Query/time/root replacement resets the window before publishing its accepted state.

All 13 owner cases passed, including both directions, null terminal cursors, overlap replacement,
the exact high watermark, overflow/reversal, external window adoption, query reset and a single
oversized page. Ten existing bidirectional page/viewer cases and 56 controller cases also passed,
covering stale success/failure, opposite-direction prefetch, queued navigation retirement, previous-
cursor advancement, bounded detail eviction, time jumps and visible-range loading. No source,
dependency, bridge, schema, UI or capacity contract changed.

The viewport has 1,291 production lines versus 1,453 at baseline. The window owner has 196 production
lines and its dedicated test file 269; neither production owner contains inline tests.

The ordinary M4 Debug client reused the retained corpus/catalog, switched to all 10,000 images,
positioned the time rail near the middle, scrolled forward and backward, and opened/navigated the
viewer. The viewer showed indices 4,915/4,916/4,915. During concurrent duplicate reanalysis, the first
visible image after return changed from `09272-3840x2160-1424.jpg` to
`09395-3840x2160-1465.jpg` (index 4,663). This observation needs a baseline comparison before final
behavior-equivalence acceptance; the cause is not established by this refactor.

After pausing duplicate analysis at 72 groups, repeated entry, next/previous navigation
(4,663/4,664/4,663) and return retained `09395-3840x2160-1465.jpg` and the visible geometry. Source
comparison leaves query/publication admission and directional completion/error handling unchanged.
The app closed normally, active process resources retired, and settings/preferences/original catalog
hashes were restored. Exact-head CI and the concurrent-return comparison remain outstanding.
The final M3 run `39014696432` rejected eight `prefer_initializing_formals` diagnostics in the two
new constructors. They now use the pinned Dart SDK's named initializing formals, preserving the
public argument names, private callback fields and callback types. The focused analyzer and owner
regressions cover this correction; the replacement commit still requires its own hosted results.

## M5 superseded USN path

The retired adapter was compiled only by `cfg(all(windows, test))`; its application orchestrator
was `cfg(test)`. The only remaining execution caller was the historical R2c-H acceptance fixture.
ADR 0024 already superseded its direct raw-volume handle, all-roots enrollment and fixed-volume
record/byte fallback. No current synchronization caller, bridge, dependency or schema was replaced.

| Retired coverage | Current owning boundary and disposition |
| --- | --- |
| Five V2/V3 parsing and malformed-record cases | The 12 `windows_usn/parsing` cases retain Unicode, 128-bit identity, framing, malformed names/version/alignment and no partial-page admission |
| Three later-parent deletion/rename reconstruction cases | Moved to `windows_usn/reference_tests.rs`, directly exercising the shared broker dependency |
| Shared volume read and downtime create/rename/delete distribution | `persistent_journal_continuity::tests::one_physical_volume_read_distributes_final_state_candidates_to_independent_roots` and session-reader shared-request cases |
| Case-sensitive paths and roots | A candidate regression now exercises the active identity translator; broker `case_sensitive_containment_keeps_photos_siblings_distinct` retains root containment |
| Journal/root-set/catalog-revision mismatch | Current broker response binding rejects root generation, volume, journal and range mismatch; old global root-set/catalog-revision rebaseline policy is intentionally superseded |
| Record, candidate and byte overflow | Identity-page bounded progress, broker request evidence limits and SQLite atomic-capacity rejection retain current limits; the old fixed-volume fallback is intentionally retired |
| Five old all-roots enqueue/checkpoint/fallback cases | Current per-root failure isolation, durable typed failure admission and atomic volume publication/capacity rollback cover persistence authority; a new cancellation-after-read case proves no enrollment/checkpoint/failure publication |
| Raw-volume smoke and controlled watcher workload | Current ordinary identity/broker and R2c-R/M controlled workflows retain these responsibilities; no raw-volume fallback smoke remains |
| R2c-H real-root read-only sampler and its two fixture cases | Historical evidence is retained. The retired sampler's byte-hash/growth tests are not claimed as current coverage; R2c-M performs metadata-only inventory and R2c-R owns current change-driven acceptance |
| Wrapper authorization, path isolation, process ownership and resource ceilings | Consolidated into the existing R2c-M entrypoint and guardrail suite, including wrong token, missing cloud acknowledgement, overlapping/nonempty/junction storage, owned-child termination, unrelated-child survival, exit-code retention and final resource samples |

The legacy adapter had 1,183 implementation lines and 866 inline-test lines; the unused application
owner had 250 implementation lines plus its test-module declaration and 447 dedicated-test lines.
Its 919-line historical acceptance fixture is retired. The current 596-line shared historical
wrapper becomes the 572-line R2c-M entrypoint, replacing the 25-line forwarding entry. The separate
216-line H guardrail file is consolidated into the 239-line M suite. Shared reconstruction has 69
dedicated test lines and cancellation 50; the identity candidate case adds 22 lines to its owning suite.
Historical checkpoint and queue-batch types remain for persistence/migration fixtures; production
catch-up lineage, catalog tables and migration histories are untouched.

Focused checks covered the 15 shared USN cases, 14 current application continuity/capture cases,
broker root/identity/limit rejection and both SQLite atomic-publication/capacity rollback cases.
The first compile exposed an unused legacy plan-preparation export; its sole caller had retired,
so the function and export were removed before the clean recompilation. The new case-sensitive
identity candidate and all eight root-availability/source-loading audit
cases passed. The consolidated R2c-M guardrails passed, including their positive and rejecting
cases. The three changed PowerShell scripts parsed successfully; repository format checked 416 Dart
files without changes and Rust formatting passed. Final diff/whitespace review found only the owned
retirement, coverage and documentation changes. Exact-head hosted results remain pending.

## M6 script ownership

The release common facade decreased from 3,526 to 606 production lines. Native interop (372),
transaction state (552), write-ahead moves (328), uninstall (587), interrupted recovery (558),
service control (279) and application deployment (245) now have separate owners. None contains
inline tests. All 91 existing PowerShell function bodies and the native initialization block
are preserved, including marker admission, operation ordering, rollback and cleanup conditions.
The existing dedicated installer suite now reads the complete 12-file module roster for its
source contracts, alongside its behavioral assertions.

The changed release scripts parsed successfully. Installer, ACL, identity and uninstall-policy
guardrails passed, including rejecting paths and interrupted-transaction recovery. These checks
use controlled fixtures; no service installation, signing, source-media operation or production
MSI change was performed.

The R2c-R common facade decreased from 3,102 to 253 production lines. Compiler bootstrap (700),
lazy native resources (1,278), controlled process execution (152) and source audit (752) are separate
owners without inline tests. The native compilation unit remains intact so its verified types,
temporary environment and process/handle contracts are admitted together. Process and native
implementation digests are unchanged. The exact guardrail-script digest was updated after reviewing
its changed assertions; the path-move exception remains restricted to that one verified script.

Splitting static imports exposed the auditor's direct-module-only function discovery. It now binds
the transitive static imports before validating calls, using the existing identity snapshots and
depth/source/byte/function/scope budgets. A two-import fixture admits a safe leaf function and
rejects a leaf containing path-addressed deletion without executing either. Production retains its
nine-source budget: the common closure has seven sources and the real runner has nine at depth two;
a tenth source is rejected. The test-only self-audit uses its explicit 13-admission budget for 12
unique files, including a repeated cleanup import. No production budget or rejection was widened.

The complete R2c-R guardrail suite passed: all 19 current case mappings, compiler ownership and
failure injection, namespace retirement, worker-report tamper execution, no-reparse identity,
bounded audit and owned-process timeout/retirement. Changed scripts passed syntax and whitespace
checks. Final exact-head hosted results and combined client comparison remain outstanding.

## Final comparison and review

The combined implementation is `976dee84c5b513b03e27e0e957e356a3b8d8d0f6`; the unchanged product
baseline is `83c8c16e263acab5270f14ca6617b38a1cd5cd6c`. Both ordinary Windows Debug builds used
the same retained `build/test` corpus: 10,000 images, 72 duplicate groups and a 256-MiB preview
budget. Each comparison began from the same SQLite backup, including the two existing durable
duplicate decisions. The baseline and candidate were built and exercised serially.

| Affected workflow | Observed baseline and candidate result |
| --- | --- |
| All-images mode, middle timeline navigation, forward/backward scrolling | The same September 2020 region and image returned; no top reset or persistent blank window |
| Gallery preview, next/previous, return | `05646-7680x4320-0580.png` remained the anchor at indices 5,285/5,286/5,285, with the original gallery geometry on return |
| Duplicate group, preferred-copy replacement and member preview | Both preferred choices updated locally; member rows stayed present without full-page loading replacement; preview and Gallery round trips retained context |
| Maximize/restore followed by eight rapid sidebar clicks | Both clients remained responsive; title/photo leading edges followed the sidebar and right controls stayed at the window edge |
| Equal-height cards, medium/small/large/medium | Candidate retained the same date/image context; returning to medium restored the original arrangement and visible coordinates |
| Square cards, medium/small/large/medium with the sidebar expanded | Both builds showed the same existing approximately one-row anchor shift; the same date and image remained visible |
| Catalog update while navigating the viewer | Both runs retained the same image during next/previous navigation and catalog publication, then returned to their original gallery geometry |

The square-card observation is reproducible in both builds: the identified image moved from the
first full row at approximately y=219 to the following row at y=383 after the size cycle. This is
an existing behavior in the refactor comparison, not an introduced regression. The subsequent
[card-size correction](#card-size-position-follow-up) addresses it separately.

During each update comparison, the catalog was observed in its running state while the viewer was
open. Both updates completed with 10,013 visited entries and 10,000 accepted images. The same image's
ordinal changed from 5,285 to 2,374 after publication while its identity and return position remained
stable. Existing exact fingerprints were reused: the terminal exact-analysis outcome recorded zero
new fingerprints and zero bytes read. These runs cover concurrent catalog publication, not a new
full fingerprint pass or a performance benchmark.

The earlier M4 return-position observation during active duplicate reanalysis did not recur in the
paired stable-state or catalog-update flows. Its original active-rehash timing was not reproduced;
the cause remains unclassified and no repair is claimed. Existing stale-publication, viewer-refresh
and directional-window regressions retain their separate coverage. This observation remains visible
for later reproduction rather than being converted into a confirmed refactor regression.

Each client exited through its normal window action. Before profile restoration, no Ame process
retained active threads or handles. Original settings, preferences and catalog components matched
their protected hashes after restoration. The retained test catalog and its durable choices were
restored from the comparison snapshot. Disposable preview cache cleanup followed process retirement,
restored-profile verification and workspace/no-reparse checks. The corpus, catalog, restoration
receipts and final candidate Debug build remain available. No original-media mutation, cloud
hydration, service installation or signing was performed.

### Final scoped audit

The final review reused the completed M1/M2 boundary reviews and checked the combined source against
the baseline. It covered migration proof/repair ordering and transactional admission; synchronization
start/stop, retained-worker and Journal/Recovery authority; gallery refresh/dimension disposal and
supersession; bounded page insertion, directional completion and eviction; transferred USN coverage;
and release/acceptance resource, recovery and static-import audit boundaries.

The release extraction retains all 91 function bodies. R2c-R retains its 48 existing functions;
only closure discovery changes, with one additional static-import registration owner. Positive and
rejecting transitive-import cases retain the nine-source production limit. M3/M4 retain query and
publication generations, callback ordering, timers and whole-page watermarks. The final refresh and
dimension owners contain 120 and 260 production lines, with their dedicated regressions beside them.

No confirmed new defect was found in the reviewed changes. Responsibility and lifecycle state now
have explicit owners; the remaining migration coordinator contains ordered historical DDL and
integration coverage, rather than a relocated copy of the original validator/repair monolith.
No dependency manifest, lockfile, generated bridge, schema version or workflow definition changed.
Final source/diff inspection and whitespace checks passed. The review does not assert a measured
speedup, complete elimination of maintenance debt or resolution of the two observations above.

### Final hosted implementation checks

These are the final implementation heads recorded by their pull-request workflow runs. The workflow
checks the PR merge snapshot using `github.sha`; the head column identifies the corresponding
published source branch. Documentation-only closeout does not change those product/tool inputs.

| Boundary | Source head | Hosted run | Result |
| --- | --- | --- | --- |
| M1 | `ef294adf82e15de82618c0cdf4a56f7bf7076b48` | [39009808560](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39009808560) | Passed |
| M2 | `6f80d66b695b223d6ac6ace1e76a778cd50eec6d` | [39013287820](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39013287820) | Passed |
| M3 | `4335ae7dda9c2aab5ba6780545000308f102db34` | [39015763595](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39015763595) | Passed |
| M4 | `86feb69bfe162a1598a8e77ead957f5894ab2ede` | [39015795301](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39015795301) | Passed |
| M5 | `e4096a242bece28484868fd54552abda300b424c` | [39016474033](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39016474033) | Passed |
| M6 / combined | `976dee84c5b513b03e27e0e957e356a3b8d8d0f6` | [39017884231](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39017884231) | Superseded by stack creation before Static and Rust completed; other execution jobs passed |

Completed runs contain 11 successful jobs. The three release-mode jobs are skipped by the existing
Daily workflow conditions; Daily's separate unsigned-release checks remain included. No new signed
release or installation qualification is claimed.

Native stack #28 contains draft PRs #22–#27 with each base matching the previous head branch.
Creation triggered fresh M2–M6 runs and superseded the incomplete M6 pre-stack run. The plan's
[current-check links](../plans/maintainability-refactor.md#delivery-checkpoint) retain the live
integration results for the published stack heads; the pre-stack table does not replace them.
Closeout changes only this evidence, the execution plan and the roadmap. UTF-8, line endings,
final-newline, local-link and whitespace checks cover those documents; product and tool inputs
remain identical to the combined implementation above. Original branch history and `main` are
unchanged by stack creation. Merge remains a human decision.

## Card-size position follow-up

The correction on `codex/gallery-card-size-anchor` follows M6 `1e87561`. Consecutive manual layout
choices now retain one image anchor instead of repeatedly selecting a new horizontal center cell
after the column count changes. A separate presentation owner releases that anchor on subsequent
scrolling, navigation, publication, detachment or disposal. Matching generations admit completion.
Explicit top/bottom intent takes precedence over image fractions; an initially short gallery stays
at the top, and retained bottom intent survives shrinking to a non-scrollable viewport.

Focused reproduction found both the round-trip row drift and edge-specific failures: before the
edge correction, equal-height large cards moved the top by 94.96 pixels and left the bottom by
276.64 pixels in the fixed fixture. The final regression run passed 20 size/edge widget cases,
seven continuity lifecycle cases and four existing position-admission cases through
`quality_test_flutter.ps1`. Coverage includes both layout modes, local-window and query-manifest
rendering, three partially visible-row offsets, top/bottom, two/ten-image galleries, a temporary
zero scroll extent, later scrolling and stale completion. Round trips preserve coordinates within
0.01 pixels. The fixture asserts user-visible positions, not only callback invocation.

The ordinary Debug client reused the retained 10,000-image corpus and 72-group catalog. Both layout
modes completed medium/small/large/medium cycles at a partial first row without the previous drift.
After further scrolling, the new position remained the anchor. A rebuilt client then exercised
top/bottom density and shape choices: the top title/first row stayed at y=207/227, and the last
image's lower edge stayed at approximately y=617. Both client runs exited normally; original
profile/catalog hashes were restored and verified, and the comparison catalog and choices restored.
These checks do not yet cover the separately requested continuous-filter restoration behavior.

The scoped review traced retained identity, scroll-listener retirement, edge admission and current
generation acknowledgement. No confirmed new defect was found in that scope. There are no new
dependencies, schema/bridge changes, source writes or changes to scrolling primitives. The screen
contains 1,741 production lines; the new continuity owner contains 64. The wall/reflow/position
owners contain 1,980/293/131 production lines, with zero inline-test lines. Dedicated continuity and
size regressions contain 214/301 lines. Hosted checks for the published follow-up remain required.

## Consecutive-filter position follow-up

The follow-up on `codex/gallery-filter-anchor` starts from card-size correction `b85ad76`.
Per-query capture previously replaced the original image with a temporary fallback after narrowing
the result. Five focused cases reproduced the lost identity before the change. The existing query
position owner now retains one image/fraction across explicit filters, separately from in-flight
publication and the current viewport used by passive refresh. Browsing, date navigation, folder
selection, changed ordering and disposal release it. Failed reads retain retry intent without
granting an obsolete completion any new publication authority.

The focused local runs passed 11 filter lifecycle cases, ten actual-wall widget cases and 16 existing
query-transition cases through `quality_test_flutter.ps1`. Both equal-height and square walls restore
the original middle offset within 0.01 pixels after narrowing to one image, the bottom, an excluded
image or empty results. Actual drag input then establishes a new anchor. Existing viewer, passive
refresh, late success/failure and disposal boundaries continue to pass. The fallback-first-image
check uses the actual row alignment; the fixture's 58-pixel date heading is not assumed to be part
of that image's anchor.

The rebuilt ordinary Debug client reused the retained `build/test` library after the user released
the window. Equal-height browsing narrowed from the middle to `02930`, then `029300` with no results,
then cleared the search. The original `02930-640x480-0270.png` returned at the same approximately
y=206 upper edge. After changing to square cards and scrolling to a new position, successive numeric
filters excluded the original image, narrowed to `09999-3840x2160-1999.jpg`, then produced no results.
Clearing the search restored `06314-2160x3840-0673.jpg` at approximately y=215. This also exercised
retirement of the earlier retained position after actual scrolling. Both layouts returned to
10,000 total images and 72 merged results. Native keyboard input established these results;
unsuccessful text-injection attempts are not treated as product failures or completed checks.

The client exited normally. Original preferences, storage settings and catalog component hashes
were restored and verified; the comparison catalog and durable choices were restored separately.
The restored profile receipt was retained outside the reusable derived directory. Cleanup removed
341 disposable preview-cache files (5,950,187 bytes) after retirement, restoration and no-reparse
checks; the corpus, catalog and Debug output remain available.

The scoped source review covers filter capture, scope/order reset, passive refresh and current
generation cleanup. No confirmed new defect was found in those changes. Query transition and screen
contain 271 and 1,757 production lines with no inline tests; dedicated filter lifecycle/pixel files
contain 217 and 238 lines. No dependency, schema, bridge, scrolling primitive or source mutation changes.

## Timeline update stability follow-up

The follow-up starts from filter correction `8ebb825`. The projection cache can publish estimated
month geometry while a replacement manifest or its matching scroll extent is unavailable. The
existing frame owner previously admitted that estimate over an already complete frame, then
admitted the complete frame again. Focused widget cases reproduced both estimated-row and unsettled-
extent replacements. A same-revision pointer case also reproduced an old drag committing after
replacement geometry; extending its assertion exposed the retained drag value after pointer retirement.

The rail presentation now retains the complete frame through those intermediate states. Its input
key retires a pointer when current geometry is lost. A generation-checked cancellation notification
uses the existing navigation cancellation path to release the matching drag value and seek intent.
First-load estimates, query/controller reset and current complete geometry remain admitted. No
debounce timer, alternate scroll state, dependency, catalog policy or source-media operation was added.

The local lock-aware runs passed 25 cases: 11 publication/input cases, three exact-layout mapping
cases and 11 existing navigation cases. Checks include missing geometry, estimated geometry,
unsettled extent, replacement admission, current-line alignment after cancellation, new input,
obsolete callbacks, query/controller changes, loaded/unloaded targets and wheel/seek supersession.
Presentation and navigation contain 144 and 802 production lines with no inline tests; the
publication regression contains 356 lines.

Hosted Flutter run `39026390953` then exposed three existing date-publication regressions: an already
released navigation request lost its alignment when replacement geometry retired rail input.
Cancellation now requires a matching unfinished drag. The original date-publication expectations
remain unchanged, including immediate/delayed four-column geometry, resolved target, missing receipt,
changed query and changed layout. All six date-publication and 11 input/publication cases passed in
the focused lock-aware rerun; the 14 unaffected navigation/mapping results above remain applicable.
The navigation owner now contains 806 production lines.

Exact-head run [39029121263](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39029121263)
on `db1842afecb8ac3565e65861709b177018ea5edb` subsequently passed all 11 required jobs, including
the complete Flutter and Rust suites, native scan/accessibility, five synthetic workloads and
unsigned Release verification. The three separate signing-channel jobs retain their configured skips.

The rebuilt Debug client completed an explicit update of the existing corpus: 10,013 visited entries,
10,000 accepted images, and 37.984 seconds recorded by the completed scan. During enumeration,
checking and final publication, the observed year marks retained their layout without alternating
between estimated and complete arrangements. A date-rail click reached November 2015; subsequent
wheel movement advanced the wall and current-position line. The final gallery retained 10,000 total
images and 72 merged results, with zero unfinished queue entries and a current journal checkpoint.
The source tile returned to `已同步`. These observations cover this native update; the focused tests
separately establish transient geometry and late-pointer boundaries.

### Prolonged updating label observation

Before rebuilding, the retained client displayed `正在更新` after its image work had completed.
Read-only catalog inspection found no unfinished queue entries, retries or failures. The journal
checkpoint continued advancing through a captured volume range, then admitted a newer range; the
observed intervening pages added no image work. It reached the captured end and the source tile
changed to `已同步` without a restart or repair. This was an active journal catch-up, with no reproduced
stale terminal label. The source of all volume activity was not attributed. The broad label does not
distinguish this checking phase from image processing; retain that presentation observation without
claiming a wording or catch-up performance correction. The subsequent explicit update above also
returned to the synchronized state.

### Journal catch-up latency diagnosis

The prolonged update observation led to a measured namespace-query performance correction. A
bounded local probe replayed 60 native pages from the same
retained volume interval, using only metadata reads and an isolated copy of the existing test catalog.
No original catalog or source file was modified. The production identity adapter covered 3,147,568
journal bytes with no current or historical locations inside the selected test root.

With the unchanged production namespace query and bundled SQLite 3.53.2, the measured loop took
18.041 seconds: volume reads accounted for 1.077 seconds, current identity resolution for 1.317,
snapshot lookup for 0.762, and historical namespace lookup for 14.835. The latter had a 241.947 ms
median and a 373.512 ms maximum per page. These costs exceed the existing 200 ms capture quantum,
so they also limit how many pages the production capture window can process before yielding.
The probe measures those stages, not end-to-end client convergence or incoming volume-write rate.

An independent read-only query-plan experiment on the same 10,000-image catalog found that the
joined request roster permits repeated scans of unrelated file and directory identities. Expressing
the same bounded roster as an `IN` predicate lets the file/directory composite indexes bind their
identity columns. The Python SQLite experiment is diagnostic evidence; the pinned native result,
semantic regressions and client catch-up measurement must determine delivery acceptance.

The production query now applies the bounded identity roster directly to each namespace branch,
preserving the root/generation filter, active-publication and inventory-state admission, aliases,
deduplication, ordering, and 256-input/4,096-output limits. No schema, index, package, page-size,
capture-quantum, cancellation or lane-priority change is involved.

The same native 60-page interval and catalog copy then took 4.030 seconds. Historical namespace
lookup fell to 0.977 seconds total, with a 16.191 ms median; native volume read and current-identity
resolution remained approximately 1.0 and 1.3 seconds. This comparison demonstrates the affected
read/lookup cost, not the complete client's convergence time. The one-off native probe was removed
from product source after its measurements were retained locally.

Three [namespace regressions](../../rust/src/adapters/sqlite_catalog/journal_namespace/tests.rs)
passed. The original production query exceeded the 20,000-operation guard even with 64 unrelated
file/directory rows. The corrected query uses 6,401 operations for the same 256-identity miss with
both 64 and 10,000 rows. The semantic checks retain multiple aliases, directory identities, repeated
requested identities, root/generation exclusion, inactive roots, empty input and both capacity bounds.
The guard observes the production query through SQLite's progress callback; it does not assert a
wall-clock threshold or copy the optimized SQL into a separate test implementation.

The ordinary Debug build succeeded and reused the existing 10,000-image corpus. Its first startup
encountered an expired retained journal checkpoint (`identity_journal_trim`), completed metadata
recovery and returned to `已同步`. That recovery is separate from ordinary catch-up timing. From
the resulting valid checkpoint, a second startup followed creation and retirement of 4,000 tiny
files outside the source tree. Read-only sampling every two seconds observed 1,888,784 journal
bytes of checkpoint advancement and 12.141 seconds from the first catching-up sample to current.
Every sampled image-work queue was empty. The final native interface showed `已同步`, 10,000 total
images and 72 merged results. This establishes convergence for the bounded fixture; it does not
attribute all unrelated volume writes or assert a universal catch-up deadline.

The client exited normally. Original settings, preferences and catalog component hashes were
restored and verified; the reusable comparison catalog and durable choices were restored separately.
The restored receipt and backups remain retained. Disposable preview data and the isolated probe
catalog were removed after process-retirement, exact-path and no-reparse checks. Source media,
the reusable corpus and the built Debug application remain intact.

Scoped review covered equivalence of the three namespace branches, root/publication authority,
transaction/error handling, aliases, deduplication and output capacity. No confirmed new defect
was found. The namespace owner contains 167 production lines, no inline tests and 216 dedicated
test lines. The separate poll-connection identity-latency diagnostic is excluded from this change.

Hosted run [39031217615](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39031217615)
passed all 11 required jobs on `79213fc10330baa88ffe0f74b0a82b61b4774045`, including static/Rust,
Flutter, both native integrations, five synthetic workloads and unsigned Release verification.
The three separate signing-channel jobs retained their configured skips.

### Hosted catalog-proof latency attribution

The remaining mixed-load hosted failures require a separate diagnosis from journal namespace SQL.
The existing operation timer now separates `proof_namespace_resolution` and `proof_pinned_identity`
inside the fresh retained-connection proof. Timing/output exist only in test builds; production
still performs the same namespace and held-handle identity observations on every admission. No
cached proof, retry, relaxed assertion or altered scheduling deadline is introduced.

The local controlled two-arm lifetime case passed with 25 measured samples per arm. The retained
connection arm reported 190 ms P0 P95 and 200 ms maximum, with P1/P2 active and progressing in all
samples; neither added substage exceeded the existing 100 ms reporting threshold. This did not
reproduce the hosted delay.

Diagnostic run [39031367037](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39031367037)
passed all 11 required jobs on `d6959e15a99389348c63eeea24ea407bef690bc2`; the Rust library suite
reported 1,989 passes and 22 existing ignores. Its raw log recorded 51 namespace-resolution calls
at or above the reporting threshold, ranging from 100 to 574 ms. No held-handle identity observation
reached that threshold. The retained-connection control and production workload reported P0 P95
of 316 and 324 ms, with P1/P2 active and progressing in all 25 samples. These readings localize the
remaining extra work without explaining every earlier P2 wait timeout from a passing run.

The namespace owner now excludes the component-exact shared suffix before expanding the differing
DOS path prefix. This avoids another name lookup through the actively written catalog and its
already-matching parents. [Windows long-name expansion](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getlongpathnamew)
can query parent directories; the optimization reduces that query scope instead of caching its
answer. Raw spellings are validated before component removal, including embedded-NUL rejection.
The complete held-file final path and identity are still observed on every proof. A junction alias,
unresolved prefix or unsupported spelling retains the independent current-path lookup.

Four prefix/namespace regressions and seven native identity regressions passed. They cover exact
suffix boundaries, differing filename/case components, ambiguous paths, fresh short-path observation,
ancestor rename, junction retarget during successful/failed SQL proof and a distinct WAL namespace
for the same database file. The junction fixture includes a shared directory suffix. The initial
sandbox run denied its temporary junction/hard-link operations; the same checks then passed with
ordinary-user execution and no compiler warnings. No assertion or timeout changed.

The corrected local two-arm mixed-load case passed in 117.41 seconds. Its retained-connection arm
reported 85 ms P0 P95 and a 98 ms maximum, with P1/P2 active and progressing in all 25 samples.
It used one poll connection across 4,196 polls; the per-poll control reported 155 ms P95 across
1,500 opens/closes. Neither identity substage reached the 100 ms reporting threshold. This is
local boundary evidence; the changed source still requires its own hosted run.

Scoped review covered raw-path admission, component boundaries, case-sensitive equality, fresh
identity authority, alias fallback and unchanged SQL/error ordering. No confirmed new defect was
found. The namespace owner has 177 production lines, no inline tests, 102 dedicated namespace-test
lines and 351 native identity-test lines. The non-mutating format gate passed with 421 Dart files
unchanged, and the final diff passed whitespace checks. No source-media,
schema, dependency, generated bridge or scheduling-policy change is involved.

### Accessibility runtime preparation

Follow-up B run `39024909889` reached its eight-second application-ready deadline after 614 ms of
type loading and 6,176 ms of client loading. Its last progress record was locating the window with
zero native elements observed; all six assembly markers were complete. In successful timeline run
`39029121263`, the first type/client loads took 66/584 ms, followed by approximately 24/22 ms in
later fresh probe processes. These observations localize the failed budget to cold preparation;
they do not identify the operating-system cause of every loading delay.

One owned preparation child now loads and verifies the fixed assemblies before application startup.
It has the same eight-second limit, and the existing whole-run clock starts before preparation.
All ten native phases, per-phase process limits, fresh assembly validation, UIA snapshots and exact
assertions remain. Preparation has no window/element observation or phase-acknowledgement authority;
a load failure still fails the run and retires its owned process. No runtime replacement, added
dependency, extended deadline, retry or application change is involved.

The four changed PowerShell scripts parsed successfully. The focused accessibility guardrail entrypoint
passed its preparation, assembly identity/milestone, deadline, child cleanup, cache-scope, timing,
evidence, incomplete/out-of-order transcript and engine-error refusal checks. The real preparation
child completed in 189 ms locally; its attempt/window/element counts stayed zero, and an application
phase request was rejected. A focused rerun after tightening fixture retirement completed in 196 ms.

The hosted Accessibility job [117396476839](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/39032787496/job/117396476839)
passed on `29c3c8b58766e8d9b0aad1d7f5db75e41fe0e529`. Preparation completed in 240 ms with all six
assembly milestones and no native observations. Application-ready completed in 3,538 ms, observing
146 native elements; all ten application phases passed in order. The completion record confirms
exit code zero, no run or cleanup failure, retired process/job ownership and removed scratch storage.
This establishes the complete hosted workflow for the correction, not the operating-system cause
of the earlier intermittent load delay. The remaining jobs and earlier failed branch heads retain
their separate integration status.

The workflow facade contains 388 production lines with no inline tests; the new preparation child
has 45. Its dedicated regression has 63 lines and is called by the existing 501-line guardrail
entrypoint. Source review covered independent preparation ownership, remaining whole-run budget,
failure propagation, unchanged acknowledgement authority and exact scratch-file cleanup.

### Review stack consolidation

The twelve-PR review surface was consolidated into native GitHub stack #35, containing existing
PRs #22–#27 and #29 in dependency order. The old stack #28 was unstacked. Branch
`codex/gallery-card-size-anchor` was fast-forwarded from `b85ad76` to `ff1d78f`, retaining every
original PR head as an ancestor and the complete source tree of the previous combined candidate.
PRs #30–#34 were closed without merging; their branches and commits remain available. No PR was
created, no commit was rewritten, and `main` remains at `83c8c16e263acab5270f14ca6617b38a1cd5cd6c`.

The seven remaining PRs are drafts with each base matching the preceding head branch. This changes
the review surface without changing product or tool inputs. Existing verification retains its
recorded scope; current branch checks and the combined candidate still govern maintenance closeout.
