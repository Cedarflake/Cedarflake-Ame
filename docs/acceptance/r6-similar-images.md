# R6 similarity integration evidence

Status: scoped evidence; [roadmap](../roadmap.md) owns delivery disposition.

The [engine evaluation](r6-engine-evaluation.md) records selection and resource probes.
[ADR 0038](../architecture/0038-bounded-perceptual-similarity.md) owns the admitted algorithm and
group identities. This record covers their production integration.

## R6 closeout

On 2026-11-05, [PR 47](https://github.com/Cedarflake/Cedarflake-Ame/pull/47) integrated R6 at
`6619cc00e250fd712addcf43ce362b28bad2db7f`. The reviewed head was
`0f1ae44c81f9abe5652218c1c9b468311defa815`, against base
`6958dea6768afed3959190a02cb9c1170a2a2e70`. The merged tree matches that reviewed head.

| Slice | Evidence |
| --- | --- |
| R6-01 | [Engine evaluation](r6-engine-evaluation.md), paired confirmation and partial-Czkawka comparison |
| R6-02 | Native integration, persistent identities, bounded matching, automatic lifetime and [multiple selections](#multiple-selected-images--2026-11-05) below |
| R6-03 | [Browsing and comparison](#browsing-and-comparison-integration--2026-11-04), native source updates, selection persistence and [title-bar interaction](#title-bar-drag-verification--2026-11-05) below |
| R6-04 | Local retained-library growth and sampled matches in the engine record, native desktop evidence below and the hosted run identified here |

The source review covered guarded reads and publication, matching continuations, migrations and
durable choices, bridge/composition boundaries, analysis shutdown and reused viewer interactions.
No confirmable new defect was found in that scope. Generated bridge output was checked through its
contracts and hosted guardrails. Existing native interaction and source-safety evidence was reused;
this closeout did not repeat the application sessions or qualify cloud-primary.

[CI run 41964287109](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41964287109) passed
all 14 applicable jobs and the aggregate Windows Gate. Its actual checkout was the synthetic merge
`7a21681248f6ea99d90d4ee4cf838f1bfec1f229` of the reviewed head and base above. Completed logs include
similarity decoder, migration rollback, retained/multiple choices, stale/cancelled publication,
persistent paging, non-transitive membership, lifecycle and desktop workflow regressions.
Three release-only jobs were skipped; manual performance, real-root and subprocess helper cases
retain their separate execution owners. No unresolved PR review threads remained at merge.

The accepted paired engine retains visual-layout discrimination and heavier-crop limits described
in the [partial-reuse evaluation](r6-engine-evaluation.md#partial-czkawka-reuse---2026-11-05).
Cloud-primary runs and constrained-client measurement/optimization remain outside R6. The
[roadmap](../roadmap.md#current-stage-and-acceptance-boundary) owns the final exit decision.

## Persistent identities and choices — 2026-11-04

At `b178fd87`, schema 74 separates immutable analysis runs, content evidence, bounded candidate indexes and
resumable matching from durable group references, representative choices, ignores and exclusions.
The migration runs atomically after schema 73 and preserves existing catalog and exact-group choices.

Two focused migration cases passed against the final DDL and row validation. They cover retained
exact-group choices, malformed reference-projection rejection, preservation of similarity choices
after derived evidence removal, complete rollback on a rejected version update, and successful retry.
The command used the repository tool lock and `Invoke-AmeAcceptanceScratch`, with `TEMP` and `TMP`
bound to that disposable fixture directory:

```powershell
cargo test --locked --offline --manifest-path rust/Cargo.toml --lib adapters::sqlite_catalog::migrations::similar_images -j 1 -- --test-threads=1
```

The new migration facade/schema/validation have 51/130/162 source lines, including conditional
fixture helpers, no inline test cases and 153 dedicated test lines. The domain identity owner has
58 source lines. Existing migration coordination changes only dispatch: 4,181 source lines including
fixture helpers and 2,290 inline-test lines; current-schema routing has 416 source and 36 dedicated
test lines. The SQLite facade changes only the schema constant and retains 3,167 source lines with
no inline cases. The retained dedicated catalog suite is unchanged.

## Native analysis integration — 2026-11-04

At `b452ceee`, the production reader uses the shared held source and the admitted normalization/hash
pipeline.
The evaluation probe now imports those helpers instead of retaining a second implementation.
Source discovery covers unequal encoded lengths independently of classification/model state and
reuses verified content evidence. Publication retains current-source, cancellation and run-lease
checks in the same transaction as evidence admission. Candidate matching persists a continuation
after at most 128 postings; group references and direct members survive interruption.

Eight R6 cases passed: the two migration cases above, two decoder contracts, three persistence and
matching cases, and pre-dispatch cancellation. The 140-content dense fixture stops within one posting
window, reopens the actual SQLite catalog, and completes with two groups and 141 direct memberships.
A shared middle member appears in both groups; endpoints 20 bits apart never become members of the
same reference through that middle member. Other cases cover unequal-size discovery, cached evidence,
changed sources, cancellation and expired-owner rejection. The shared source guard keeps its retained
[Windows lease evidence](r6-engine-evaluation.md); these new fixtures do not use real library roots.

The generated opaque session exists before dispatch, retains cancellation and foreground suspension,
and exposes bounded progress snapshots. FRB 2.12.0 initially failed while discovering the internal
`LocalPreviewSources` adapter. Excluding the adapter module from bridge discovery preserves all 113
existing API entry names and adds only the five similarity-session entries. Rust and Dart generated
hashes agree; the repository bridge command completed an optimized native build. The new run method
is registered with the existing asynchronous-contract gate; all 32 entries passed.

`image_hasher =3.1.1` is now a runtime dependency and `image` is pinned to `=0.25.10`. MIT notices for
the selected engine/decoder and relevant dependencies are included as an asset and registered with
the existing license page. No model, native runtime or original-media changes are included.

Source / inline-test / dedicated-test line counts are 344 / 0 / 102 for the image-reader family,
842 / 0 / 284 for similarity persistence, and 381 / 0 / 18 for the session/execution family. Source
counts include module declarations. Existing SQLite coordination adds only the feature module.

The [schema-74 CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41820364101)
reported two availability checks with outdated module inventories and one schema-33 preservation
check that treated newly created similarity tables as retained rows. The inventories now admit only
the exact added module declarations and bridge-discovery annotation. The migration case explicitly
asserts empty similarity tables and the initial dispatch row before comparing every retained row.
All ten focused availability checks, including rejecting mutations, and the schema-33 migration/reopen
case passed locally. These changes add no test cases or broad exclusions. The availability facade
has 2,603 pre-test lines and 3,550 inline-test/support lines; the migration case file has 382 lines.

## Group queries and durable operations — 2026-11-04

The native review contract now provides bounded group/member pages, seek, preference, ignore/restore,
member exclusion and exclusion-specific undo. Current fingerprint compatibility owns membership;
exact-copy representative selection reuses exact-review policy. Groups below two visible contents
are omitted. Degree changes query the existing evidence. Decision revisions reject stale writes,
and unavailable preferences remain saved without selecting replacement content.

Two SQLite fixtures passed, covering exact-copy projection, degree boundaries, preference and ignored state,
member removal, stale writes/undo, reopen, current-source replacement and group-scoped exclusions.
The 70-group fixture returns 64 then six groups and clamps seeking to the last page. The retained
choice case also passed after adding derived-evidence removal and reconstruction of the same group
through production matching, with its preference, ignore and exclusion still effective.
The review persistence family has 570 source lines, no inline cases and 353 dedicated test lines;
the application review boundary has 129 source lines.

Canonical bridge generation preserves all 118 prior API entries and adds seven review operations.
The optimized native build, bridge guardrail fixtures and all 39 asynchronous contracts passed.
The [native-analysis CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41828314572)
exposed the guardrail fixture's pre-R6 roster count of 31; the count and scoped module checks now
include the analysis session and review operations without weakening synchronous-call rejection.

## Automatic analysis lifetime — 2026-11-04

The desktop assembly now supplies one cold native session service and an explicitly owned
controller/scheduler pair. Catalog revisions and availability changes coalesce behind three seconds
of idle admission. Foreground reads and catalog updates suspend active work; navigation leaves it
running. Failures and memory pressure retain a one-minute cooldown. Environment suspension cancels
the current run; resume admits pending work. Shutdown joins completion before releasing its handle.

Three focused composition/lifecycle checks passed, including missing production bindings, close
while opening, foreground yielding, newer-publication coalescing and cooldown admission. The initial
two failures came from a guarded Flutter assertion inside the fake's disposal callback; a synchronous
assertion retains the joined-disposal requirement without conflicting with `pump`. Two existing
source-status cases also passed through the shared activity widget after its similarity connection.
The controller/scheduler pair has 370 production lines, no inline tests and 233 dedicated test lines;
the binding has 51 lines and the shared activity widget has 158.

The [review-contract CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41831699680)
reported five generated FRB files absent from the dependency check's exact roster. Those specific
generated paths are registered; handwritten files and missing generation markers remain rejected.
The dependency check passed for 504 modules, and all 18 retained boundary cases passed.

## Desktop review contracts — 2026-11-04

The native review adapter now maps all seven operations into Ame-owned Dart values and is a required
production binding. Similarity queries reuse the existing bounded review window; exact-member
decision metadata moves into its own typed slice without changing window capacity or read admission.
Separate similarity query and write owners preserve degree, group and revision authority across
navigation. Ignored queries use the broad degree; the selected normal degree remains available on
return. Undo carries the exclusion identity even after returning to the group list.

Three new application cases passed for superseded degree reads, pending-write navigation and retry,
and member exclusion/undo with a group dropping below two contents. Eight retained exact-review
cases and four production-composition cases also passed. These cases establish client state and
binding behavior; the existing real SQLite evidence owns durability and stale-write rejection.
The query/write owners have 315 production lines, the native adapter has 211, and their dedicated
cases/support have 274 lines with no inline tests. Dependency verification passed for 509 modules.

Actual desktop interaction, local-library accuracy and the complete R6 user workflow retain their
acceptance scope in the roadmap.

## Browsing and comparison integration — 2026-11-04

The Duplicates page now composes separate exact/similar query owners through a bounded mixed-group
workspace. Its right-side menus, nested navigation reset, member selection, in-place ignore/restore,
representative choice and exact-copy locations follow the implemented
[workbench contract](../product/gallery/workbench-and-navigation.md#workbench-blueprint).
The shared transient-feedback component owns the optional undo/retry action.

The comparison workflow initially stalled because both panes used one latest-selection source
scheduler: the second request retired the first. Each pane now owns its scheduler while both retain
the native two-lease cap and source guards through buffer copy. The workflow case decodes both
images and verifies both lease closures, linked/independent zoom, return context and
primary-navigation reset. It also checks mixed
ignored groups and restoration in place. This is a controlled port fixture, not native source evidence.

Nine layout/preview-demand cases passed at 400, 800, 1200 and 1280 logical pixels with normal and
doubled text. The narrow row-height expectation now allows the intentional action footer while
retaining bounded height, alignment and overflow checks. Six automatic-analysis/location cases,
eight retained exact-review cases, one preference-publication case, three paged-viewer cases and
two retained Windows semantics cases passed. The semantics entry now opens a group on every primary
return, matching the accepted reset behavior. Two obsolete manual-pause UI cases are replaced by
one case proving a saved legacy pause does not prevent automatic discovery; scheduler lifecycle
coverage remains. Scoped presentation analysis passed without warnings.
Dependency verification passed for 513 modules after retiring the two unused pause-setting
consumer registrations; new and changed dependencies remain checked.

Existing `AmePopupMenuButton`/`ameMenuChoice` retain framework popup and focus ownership. The installed
Flutter SDK supports `MenuAnchor`, `MenuItemButton` and `RadioMenuButton`; a second menu composition
is unnecessary for these short choices. Initial comparison reused SDK gestures but omitted the normal
viewer window frame and bypassed its smooth zoom/input owner. Both surfaces now share
`LibraryViewerFrame`, `LibraryViewerImageInteraction`, `LibraryViewerZoom` and the bottom
`LibraryViewerToolbar`; comparison adds only pane selection/linking and its guarded image pair.
Image transformations keep loading/failure controls stationary. The retained 11 viewer cases passed,
including full-screen return, actual-size preservation, wheel/navigation preference, pan bounds and
intermediate animation frames. The comparison workflow additionally passed caption presence,
bottom-action placement and intermediate linked/button and independent/wheel zoom.
The comparison, single viewer and toolbar have 272, 301 and 268 production lines; the extracted frame,
image interaction and comparison coordinator have 150, 99 and 77. There are no inline tests; the
extended workflow case/support has 316 dedicated test lines. Scoped analysis passed without warnings,
and dependency verification passed for 516 modules.
The desktop refinement removes speculative mobile stacking and its 400-pixel case, keeps one bottom
row, and uses the same toolbar button component and adjacent spacing for linking, reset and zoom.
Pane selection follows pointer input or arrow keys without an additional selector. The 11 retained
viewer cases and the extended comparison workflow passed, including equal button geometry/gaps and
keyboard-selected independent zoom.

The first isolated Windows client imported 51 existing fixture files and automatically found one
exact group and five standard-degree similar groups. Exact ignore stayed in its member page;
similar preference, two-copy location inspection, original pair loading and linked zoom/pan worked.
This run exposed the missing caption and divergent viewer controls. It reached the harness's
1,200-second time limit during inspection, at a 225-MiB peak working set; it did not complete normal
close or the final source-hash check. These observations do not qualify the revised controls.

The guarded `da1630d8` client passed linked wheel zoom/pan, independent right-pane zoom, bottom reset,
maximize/restore, title-bar double-click, minimize/reactivation and Escape return with both member
selections retained. Its second inspection reached the 1,200-second harness limit before the close
check; all 51 source hashes remained unchanged, and peak working set was 190 MiB. Automated title-bar
drag attempts did not produce observed movement; subsequent
[manual verification](#title-bar-drag-verification--2026-11-05) confirmed normal dragging.

The guarded `a8b1bd36` client confirmed the selector removal and equally spaced link/reset/zoom-mode
buttons in the actual 1,269-by-714 window. Changing link state preserved their placement; right-arrow
selection followed by independent zoom changed only the right image. Closing from the comparison
caption exited normally with code 0. This 213-second inspection peaked at 188 MiB and preserved all
51 fixture source hashes.

The guarded `be4b0cad` client displayed both dropdown arrows and the group menu in the requested
`重复组` / `相似组` / `已忽略` order. Selecting the second item opened the similar-group list with its
degree menu. These menu checks reuse the existing popup and selection owners without new tests.

At `dc7ff409`, member rows expose preference, exact-copy locations and Explorer actions; the
member-exclusion button, its feedback/undo handlers and unused copy are removed. The retained
workflow case passed after retiring its withdrawn exclusion steps; native decision compatibility
coverage remains. The actual Windows row had three aligned actions, and the two-copy locations
sheet opened and dismissed correctly. Normal close returned code 0 after 121 seconds, at a 221-MiB
peak working set, with all 51 fixture source hashes unchanged. The page, results and similar-card
owners have 351, 135 and 117 production lines, no inline tests; the retained workflow has 298 lines.

The guarded `3c59c25b` client, with unchanged product inputs at `b57c8cc3`, automatically found one
exact group and seven standard-degree similar groups from the same 51 files. The actual Windows
workflow showed ignored exact and similar groups together with their respective labels. Ignoring
and restoring either kind stayed in its member page; returning to the ignored list reflected each
restoration, ending with no ignored groups. A saved similarity preference remained selected.
Comparison loaded both originals, and ordinary return retained both checked members. Switching to
Library and back returned to the group list; reopening cleared temporary checks and retained the
saved preference. Strict, standard and broad degrees showed six, seven and seven groups respectively;
switching back preserved that preference. Nested pages omitted both menus and the withdrawn member
action. Normal caption close returned code 0 after 864.734 seconds, at a 219-MiB peak working set;
all 51 source hashes were unchanged and the scratch profile was retired. The automated title-bar
drag still produced no observed movement; subsequent manual verification is recorded below.

A second isolated profile admitted only disposable copies of the same two members. Moving the
selected, preferred copy outside its watched directory caused the open page to remove that member,
clear temporary selection and disable comparison without manual refresh. Returning showed no group
with the one remaining member. Restoring the unchanged copy automatically republished the group;
its saved preference remained selected and temporary checks stayed clear. Normal close returned
code 0 after 164.949 seconds, at a 191-MiB peak working set. All 51 reusable fixture hashes remained
unchanged, both moved-copy hashes matched their source, and scratch retirement completed.

The [4c35a993 CI run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41897593393) passed all
14 applicable verification jobs and the aggregate Windows Gate. Three release-only jobs were skipped.
Completed logs include passing similarity adapter, migration rollback, prior-choice preservation,
stale/cancelled publication, persistent paging and non-transitive membership cases. Existing manual
performance, real-library and helper cases retain their separate execution owners. The earlier
Static missing-brace failures are corrected. The shared drag region and native window-manager path
are unchanged from `main`; the earlier automated drag observation is resolved below.
Cloud-primary measurement is deferred outside R6; the local evidence does not qualify that client.

## Multiple selected images — 2026-11-05

Schema 76 preserves former single choices as independent selected-content rows, including choices
for retired schemes, and leaves exact-group preferences unchanged. Three focused migration cases
and two SQLite review cases passed under the repository lock and disposable scratch storage.
They cover atomic rollback/retry, retained revisions and ignores, independent add/remove,
idempotence, deterministic cover selection, stale-source rejection and compatible reanalysis.
The controller's three cases and the existing desktop workflow passed with per-member selection,
blocked writes during replacement reads, late completion, comparison continuity and accessible labels.

The guarded Windows build used the production application with an isolated catalog and the existing
51-file corpus. Two images remained selected together after closing and reopening the application.
Removing `00025` kept `00013` selected and changed the group's cover to `00013`. The final wording
check observed `选择图片` / `取消选择` in both tooltips and the native accessibility tree. Removing the
last selection restored the ordinary `00025` cover. The two persistence sessions and final wording
session closed with code 0 after 228.445, 127.623 and 190.889 seconds; each preserved all 51 source
SHA-256 hashes. Both scratch profiles were retired. No original catalog or real-library root changed.
The final build-input digest was
`778938ff9baf79757b70d9e55e0b4944eaa3ccdb833e70a2c5f504fdd7352af0`;
the debug-assertion native library's SHA-256 was
`36545846f26064066fbcfc3961b95aa1560f4199583126d80665aaeff9726ade`.

The preference migration has 78 source lines, no inline cases and 72 dedicated test lines.
Review decisions/queries have 177/200 source lines and share the 401-line persistence suite.
The similar controller and shared review window have 244/223 source lines, no inline cases;
the focused controller/workflow suites have 129/316 lines. The SQLite facade remains 3,168 source
lines; the migration coordinator has 4,185 source and 2,290 inline-test lines and only adds version
dispatch. Generated bridge output is excluded from these counts. Hosted results belong to the
exact pushed commit's checks.

## Title-bar drag verification — 2026-11-05

At `29cd10e8`, the guarded Windows client reused the matching build-input digest recorded above.
Automated drags left both the main window and ordinary viewer at screen origin `(19, 12)`;
the available input interfaces did not expose a held-button gesture for further isolation.
The user then manually dragged the comparison title bar and confirmed normal window movement.
The comparison remained usable afterward and closed through its caption button with exit code 0.
This completes the drag interaction check; the automated input discrepancy has no established cause
and requires no product change. Comparison and ordinary viewing share `LibraryViewerFrame`,
`LibraryViewerTopBar` and the unchanged `AmeWindowDragRegion`.

The session lasted 572.848 seconds and peaked at 218,607,616 bytes of working set. All 51 fixture
source SHA-256 hashes remained unchanged, and the isolated scratch profile was retired.

## Development analysis throughput — 2026-11-05

The `c90fc58c` development client analyzed 218 contents in about 888 seconds. Read-only catalog
inspection found 3 completed matching continuations and 210 pending reference searches; analysis
was active without suspension or an error. Existing groups were empty. The first 191 ready contents
also had no qualifying pair under standard or broad degree in an exhaustive stored-hash comparison.

A bounded generated-PNG diagnostic used the production decoder and normalization/hash owners:

| Image | Unoptimized Debug | Debug with optimization level 2 | Release |
| --- | ---: | ---: | ---: |
| 512 × 512 | 1,734.625 ms | 61.812 ms | 12.402 ms |
| 2048 × 2048 | 9,624.556 ms | 178.410 ms | 61.639 ms |

These totals cover decoding, normalization and both hash sizes, excluding fixture encoding, catalog
work and scheduling. Both fixtures produced identical paired-hash digests in all three profiles.
The optimized Debug measurement retains assertions and symbols; the approximately 54-fold gain on
the larger fixture is a computation result, not a whole-library throughput claim.

An optimized native comparison restored the same isolated catalog snapshot before each run over
16 generated images. First standard-degree groups appeared after 452 ms with the original matching
cadence and 224 ms with bounded slices; complete runs took 897 / 994 ms respectively. Both returned
two standard groups, retained all expected broad-degree transformations, rejected unrelated scene
membership and produced the expected signature bytes. Fresh reference admission can occur earlier;
the fixed-reference criterion and existing reference identities remain unchanged. SQLite integrity,
foreign keys and every source SHA-256 passed, and the leased derived storage was retired.

The existing Flutter review workflow now also proves publication while analysis is still running,
one in-flight refresh during a delayed query, and polling retirement after analysis stops. Its full
comparison/navigation case passed locally. Matching cancellation, lease renewal and bounded forward
progress have two focused Rust cases; their execution belongs to hosted CI.

The execution facade and matching owner contain 129 / 35 production lines, no inline cases and
122 dedicated matching-test lines. The Duplicates page and refresh owner contain 370 / 46 production
lines, no inline cases and share the 382-line workflow test. Debug configuration changes no engine,
schema, dependency version or source-read admission.

The rebuilt `26e1fffd` development client retained the previous 627 analysis outcomes. On
`local-primary`, it produced 315 additional ready signatures over 186.703 seconds: about 101 per
minute, compared with about 15 per minute in the initial observation. These are different live
image batches, so the roughly sevenfold improvement is an observed rate rather than a controlled
whole-library benchmark. The standard-degree page displayed 14 groups and then automatically rose
to 16 while analysis remained active, without further input. The updated client was left running.
Its loaded native library SHA-256 was
`38aebe53700573698ba4d714d19b70d6988875285aee94fe11c46bbf7aa87abd`.

The initial Debug rebuild encountered MSVC D8050 in `dart-sys`; the same compile succeeded with
leased, command-scoped `TEMP` / `TMP`, followed by a successful full Debug build. No system settings
changed. The [26e1fffd hosted run](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/41975539699)
was still queued behind the preceding `main` run at this observation; its Rust regressions and
remaining hosted gates were not yet verified.

## Similar-group member query latency — 2026-11-05

At `a346a246`, the optional group predicate made member counting and paging walk the current
fingerprint scheme before selecting the requested group. A read-only `local-primary` reproduction
using SQLite 3.47.1 measured 583.923 / 857.313 ms for the count/page of the same 36-member group.
The group-scoped projection fixes join order before fingerprint/location lookup; the same snapshot
returned identical rows in 5.825 / 3.108 ms. These timings isolate SQL, not total image-render latency.
Current-source checks, representatives, exclusions, choices and read-snapshot consistency remain.

A group equality predicate alone still allowed unrelated-fingerprint growth: adding 4,096 records
increased a two-member read from 2,043 to 92,155 SQLite instructions. The regression measures the
production member read rather than asserting a particular query-plan string or wall-clock limit.
The final join order passed that bound and both existing review cases locally: three cases passed,
covering choices, stale sources, exclusions, paging and seeks. Client verification and hosted checks
remain pending for this change.
