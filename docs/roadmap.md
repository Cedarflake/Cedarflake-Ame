# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-10-24

This file owns product delivery order, the current focus, blocking obligations, and stage exit
decisions. Only one stage may be active. Detailed product contracts, execution procedures, architecture,
and historical verification have separate owners in the [documentation index](README.md).
Read the linked active execution plan before working on its scope; it cannot independently advance
a stage or change this queue. Historical results never establish current-source acceptance.

## Product direction

Ame is a local-first Windows image-organizing workbench for a large personal library. Its sequence
is trustworthy catalog and browsing, exact duplicate understanding, durable virtual organization
and review, machine-assisted understanding, and finally separately authorized physical operations.
The gallery remains one shared canvas throughout. Cataloging, browsing, analysis, and virtual
organization preserve original media and do not require a second full source copy.

The [workbench contract](product/workbench.md) owns product concepts, exclusions, and reference
policy. The [gallery UI contract](product/gallery-ui.md) retains the accepted presentation rules.
[Organization workflow requirements](product/organization-workflows.md) preserve the full
R3–R10 scope; their existence does not make those capabilities available.

## Delivery sequence

| Stage | User outcome | Current disposition |
| --- | --- | --- |
| R0 | A real Windows picker-to-catalog-to-preview vertical slice | Accepted foundation |
| R1 | Progressive, resumable, multi-root catalog with source preservation | Accepted foundation |
| R2a | Accepted unified gallery and connected-control design | Accepted foundation |
| R2b | Continuous gallery, bounded previews, selection, viewer, and storage lifecycle | Accepted on 2026-08-13; preserve regression boundaries |
| R2c | Change-driven continuity with usable browsing during synchronization | Completed and frozen on 2026-10-16; preserve delivered behavior |
| R3 | Exact duplicates folded with inspectable physical locations and no source mutation | Completed on 2026-10-17; preserve delivered behavior |
| R4 | Independent Favorites, flat group-based albums, and durable organization progress | Active; R4-05 desktop workflows in progress |
| R5 | Metadata, primary classification, calibrated confidence, and human correction | Planned after R4 |
| R6 | Perceptual similarity review, distinct from exact duplicates | Planned after R5 |
| R7 | Structured and semantic discovery through the same gallery | Planned after R6 |
| R8 | Immutable physical-organization dry-run with conflicts and no execution | Planned after R7 |
| R9 | Freshly authorized file operations with revalidation, history, and recovery | Planned after R8 |
| R10 | Large-library maturity and release readiness | Later maturity work; infrastructure may support earlier stages |

R2b's [foundation evidence](acceptance/gallery-foundation.md) retains the frozen interaction baseline
and conditional adaptation gates. Accepted work is not reopened merely to complete an optional
migration checklist. R3–R10 retain their full product and safety requirements in the linked contract;
this overview does not reduce them to navigation shells or compilation milestones.

## Current stage and acceptance boundary

**R2c is frozen and R3 is completed. R4 is active on `codex/r4-organization`.**
R4-01's durable records, guarded whole-group commands, first favorite transaction and migration
recovery are implemented and covered by hosted checks. R4-02 adds album transitions, version-checked
undo, late-group conflicts and content-version reassociation. Explicit source removal and parent
consolidation preserve or retire organization ownership transactionally. Focused diagnostics pass
for these boundaries. R4-03's album/Favorite scopes now cover bounded pages, counts, layout and
viewer anchors, with focused Rust and Flutter checks. The catalog-storage correction passes the
unchanged local and hosted 10,000-image workloads. The remaining stale terminal-trigger version
assertion is corrected with its focused regression. R4-04 now records publication-order admissions
transactionally, including source transfer/removal and migration. Pending, deferred and ignored
queries share bounded pages, group counts and logical anchors; focused Rust and Flutter regressions
pass. Durable continuation now retains logical work across reopen, group changes and ordinary-mode
edits, with atomic progress saves and stale-session rejection. Focused continuation, migration and
source-lifecycle checks pass. The viewer-Favorite checkpoint passed hosted verification. The current
product checkpoint still requires completed hosted verification after its cancelled-request
fixture correction.
R4-05 now connects viewer Favorite/unfavorite, undo/retry and the Favorite collection.
Controlled desktop checks cover transient Favorite feedback without notification history,
create-in-picker assignment, defer/ignore/restore, saved continuation, collection persistence and
ordinary-scope restoration after mode exit. Album management, viewer assignment/movement/removal,
sidebar scopes and durable album settings are connected and covered by focused regressions;
native undo, remaining management/settings paths and failure interactions remain pending. The
query-wide selection foundation is
implemented with bounded staging, atomic group writes and cancellation; its focused adapter and
session checks pass. Gallery selection now connects whole-query album/Favorite commands, shared
picker creation, cancellation and transient undo; focused Rust and Flutter checks pass. Organization
mode now connects its menu entry, shared counts, bottom queues, contextual actions and durable
continuation through the existing gallery and viewer. Both queue layouts use continuous rows;
focused regressions and controlled desktop checks pass. Persistent
involved-album conflict inspection and resolution remain to be connected.
Retain the real-interaction and retained-library
performance obligations.
The [R3 execution record](plans/r3-exact-duplicates.md) and
[closeout evidence](acceptance/r3-exact-duplicates.md#r3-closeout) retain the delivered
[R3 product contract](product/organization-workflows.md#r3---exact-duplicate-understanding),
client verification, source preservation, resource bounds and hosted results.

The R2c product baseline is commit `5ea62d05eb7f84a64a12967fdba20fda9008f371`. The
[frozen closeout checkpoint](plans/r2c-closeout.md#current-execution-checkpoint) and
[final recovery verification](acceptance/r2c-update-checkpoint.md#r2c-freeze-on-2026-10-16)
retain its delivered behavior, hosted result and deferred incident. Previous R2c queue instructions,
paused-R3 statements and candidate-readiness requirements are historical. They do not schedule
another R2c closeout cycle or block the selected R3 work. Historical test records retain their
actual coverage and failures; stage closure does not rewrite those observations.

The earlier Debug process termination during recovery is deferred until it recurs. Retain its
existing log and Windows error report, with the cause unresolved. Do not schedule another crash
investigation or replay from that old incident. A new occurrence can receive a scoped maintenance
fix without reopening the R2c milestone. New source-safety failures or defects that directly prevent
R3's selected workflow are handled at their owning layer under the repository contract.

## Current execution queue

The requested sidebar maintenance hides expansion for leaf roots and reserves source status for
import and directory synchronization, independently of duplicate analysis. Its [scoped evidence](acceptance/library-sidebar.md) records the focused
regressions and client checks; it does not start a new stage.

The requested image-rendering maintenance covers adaptive original-image sampling, sharper cropped
previews, larger card densities in both gallery layouts and neighboring-image row packing. Its
[scoped evidence](acceptance/image-rendering.md) owns the focused regressions, Windows client checks
and retained resource limits. Hosted verification follows publication; this maintenance does not
start R4.

The source-status maintenance repairs retained local journal registration failures that left an
empty queue without a recovery consumer. Known failures now revalidate the root and journal opening
before entering the existing background verification. Focused regressions and the retained-library
client pass, including a second startup with ordinary catch-up and `已同步`. The
[scoped evidence](acceptance/r3-exact-duplicates.md#retained-registration-failure-and-source-status)
records recovery, status precedence and source preservation. Hosted verification follows publication;
this maintenance does not reopen R2c/R3 or start R4.

The requested recovery-order maintenance adds bounded failed-read admission into Windows Search
preparation followed by materialized background verification. Once fallback is admitted, complete
verification owns this round without waiting for USN to recover. Preserve completed inventory work,
source revalidation and truthful continuity state. Its
[scoped evidence](acceptance/r3-exact-duplicates.md#journal-fallback-order) owns the regressions and
client verification. This is maintenance of the accepted synchronization path.

The requested foreground-image maintenance adjusts the Windows engine's image I/O
thread to normal priority at startup. Its scope is the reproducible upload wait on
a two-core device; the separate five-minute Release observation is deferred by user
direction. Preserve source leases, cancellation, catalog state and other worker
priorities. The [scoped evidence](acceptance/viewer-scheduling.md) owns the measured
diagnosis and verification. The actual-engine checks and cloud Debug comparison
pass, and commit `a8cbc84` passed its applicable hosted checks. This maintenance does
not start R4.

The requested [first-import performance maintenance](plans/first-import-performance.md) stops at
the current candidate by user direction on 2026-10-22. Retain the measured catalog improvements and
the subsequent coalesced progress commit; do not schedule further optimization or client imports.
The measured 80.381-second cloud-primary import exceeds the earlier 60-second limit, and the latest
progress-commit change has no new client timing. PR #44 integrated this candidate alongside the scoped
browsing follow-ups as `664da6a` after current-head hosted verification. This disposition does not
claim the one-minute outcome or start R4. Its timings, source preservation, cache limitations and
verification belong in the [acceptance record](acceptance/first-import-performance.md).

R3 is closed after exact identification, physical-location inspection, both gallery modes,
duplicate review, incremental maintenance and durable choices passed their affected verification.
Preserve its source-read boundaries, deterministic representatives, bounded windows, retained viewer
navigation and accepted workbench behavior. The execution and acceptance records retain the completed
R3-01 through R3-05 deliveries; they are no longer an active repair queue.

The active stage is [R4 virtual organization and review](product/r4-organization.md).
Its requirements cover whole-group single-album assignment, independent Favorites, continuous
organization progress, conflict handling, and recovery/cleanup. The
[gallery contract](product/gallery-ui.md#review-workflow) owns the accepted desktop interaction.
Requirements are defined.
The [R4 execution plan](plans/r4-organization.md) follows the
[implementation sequence below](#r4-implementation-sequence). R4-01 establishes durable identity,
guarded group commands and migration recovery before exposing organization controls.
Preserve the existing visual language; use the approved sketch for placement and workflow.
Do not restart completed R2c/R3 acceptance or
advance physical file operations, perceptual similarity or release signing through this closeout.

The requested maintenance scope
[fast recovery after journal loss](plans/r3-exact-duplicates.md#fast-recovery-after-journal-loss)
is completed on `main` through PR #43. It applies Windows Search hints through guarded actual-file checks
at startup, then verifies the complete source quietly using the original materialized inventory.
The accepted Windows metadata-reuse boundary and multi-root scheduling remain intact; direct
streaming capture is withdrawn. The corrected production entrypoint has completed its retained-library
client check. The sidebar keeps `正在核对`, and confirmed gallery-viewer changes offer reload or return
through the shared floating notice without moving the image. Focused regressions and the affected
client interactions pass. The requested date-condition follow-up retains the time rail's painted
frame across query replacement so its existing motion can continue; old-query input is retired.
Exact-commit hosted verification passed before merge, and the delivery branch was removed locally
and remotely. The owning
[verification](acceptance/r3-exact-duplicates.md#fast-recovery-after-journal-loss) separates indexed
refresh, complete verification and the earlier interrupted timings. There is no fixed startup-time
threshold for this maintenance scope.

The time-rail return-to-top follow-up retires superseded queued and active navigation even when
the new target is already loaded. Focused regressions and the corrected near-top-to-top client
sequence pass. Hosted verification passed and PR #44 integrated the change into `main`; its branch
was removed locally and remotely. The [scoped evidence](acceptance/r3-exact-duplicates.md#returning-the-time-rail-to-the-loaded-top)
retains the two request-lifetime boundaries. This does not reopen a completed stage.

The duplicate-review follow-up uses a bounded direct-position read for distant group navigation
and retained-position revalidation, with ordinary adjacent keyset paging and preserved user choices.
Focused regressions and the rebuilt real-library client interaction pass; the user confirmed the
loading issue resolved. Its [scoped evidence](acceptance/r3-exact-duplicates.md#direct-positioning-in-duplicate-review)
and implementation were integrated through PR #44 after hosted verification.

The earlier [startup-update repair](plans/r3-exact-duplicates.md#startup-update-maintenance)
retains its delivered behavior. Time-rail input,
progressive duplicate folding and post-sync gallery convergence are implemented and client-verified
on `main`. Stable existing folded cards, progressive confirmed copies, shared update status and
bounded badge inspection are also implemented and client-verified. The `journal_trim` investigation
confirmed expired checkpoint retention and added observed-bound diagnostics. Preserve guarded source
evidence and the existing test library. This scoped repair does not reopen R2c/R3 milestones or start R4.

The [maintainability refactor](plans/maintainability-refactor.md) is implemented on six sequential
responsibility branches: historical migration proof and compatibility repair, synchronization runtime
ownership, gallery refresh/dimension lifecycles, retained-page rules, superseded test paths and
release/acceptance script ownership. Native GitHub stack #35 contained PRs #22–#27 and the
consolidated follow-up PR #29. The plan links current hosted checks and the completed client comparison,
scoped audit and retained observations. The six branch heads and combined source candidate passed
their applicable hosted checks. All seven PRs merged into `main` after explicit authorization on
2026-10-18; the stack is closed. Gallery notification ownership and catalog preview-reset transactions
subsequently merged through PR #36. M9–M13 then merged through native stack #40 at `4808928`:
gallery interaction ownership, native-file enumeration and parsing, persistent-journal decomposition,
hosted CI parallelism and catalog/gallery test organization. All five PRs are merged and the stack
is closed; original commits remain reachable and the five delivery branches were removed. The
[final maintenance closeout](acceptance/maintainability-refactor.md#second-round-closeout)
records the preserved behavior, client checks, final audit, complete hosted coverage and timing.
The requested maintenance queue is complete. Preserve isolated hosted timing workloads, serial
workstation execution and the delivered ownership boundaries. The plan retains the five original
maintenance findings and their evidence.
This delivery does not start R4 or reopen R2c/R3 acceptance.
The same plan records the completed card-size position correction, continuous-filter anchor retention,
timeline stability during updates and journal catch-up correction as separate behavior changes after
the six refactor boundaries. Hosted mixed-load diagnostics and the catalog-proof correction retain
their evidence in the linked acceptance record.

## R4 implementation sequence

R4-01's internal foundation passed its hosted checks. R4-02's transitions, source lifecycle and
group reconciliation and R4-03's bounded collection queries are implemented. The storage correction
passes the unchanged local and hosted 10,000-image workloads. R4-04's transactional admission ordering
passes focused publication, source-lifecycle and migration checks. Its bounded queue queries and
coherent progress counts and durable continuation pass focused regressions, including reopen,
ordinary-mode edits, stale saves, rollback, group evolution and source lifecycle. R4-05 is active with
the viewer Favorite path, album management, viewer membership commands, collection entries and
album settings connected. The query-wide selection command boundary passes focused adapter and
session checks. Controlled desktop checks cover the core organization-mode loop, restart,
Favorite history behavior and restored ordinary context. Remaining native undo, batch,
management/settings and failure paths still require verification. Gallery selection and the core
organization-mode workflows are connected; persistent involved-album conflict inspection and
resolution remain pending. Keep the product
checkpoint's hosted verification and retained-library query measurements outstanding. R4-06 and
R4-07 remain pending. The
[execution plan](plans/r4-organization.md) defines the dependencies, work and exit conditions below;
this roadmap owns their order and status. No R4 user workflow has passed acceptance yet.

| Order | Slice | Required result |
| --- | --- | --- |
| R4-01 | [Durable group decisions and command targets](plans/r4-organization.md#r4-01-durable-group-decisions-and-command-targets) | Stable organization identity, whole-group commands, transactional records and recoverable migration foundation |
| R4-02 | [Organization transitions and group reconciliation](plans/r4-organization.md#r4-02-organization-transitions-and-group-reconciliation) | Single-album/Favorite decisions, shared progress, undo, merges/splits and safe source lifecycle handling |
| R4-03 | [Bounded album and Favorite queries](plans/r4-organization.md#r4-03-bounded-album-and-favorite-queries) | Consistent, bounded gallery scopes, pages, counts, layout and viewer navigation |
| R4-04 | [Continuous queues and durable continuation](plans/r4-organization.md#r4-04-continuous-queues-and-durable-continuation) | New groups at the front, stable logical continuation and shared counts across ordinary and organization modes |
| R4-05 | [Connected albums, Favorites and organization mode](plans/r4-organization.md#r4-05-connected-albums-favorites-and-organization-mode) | Ordinary album/Favorite workflows and accepted organization controls, continuation and persistent conflict resolution |
| R4-06 | [Export, restoration and unresolved-record cleanup](plans/r4-organization.md#r4-06-export-restoration-and-unresolved-record-cleanup) | Versioned portable recovery, preserved rebuilds and explicit cleanup without resurrection on reimport |
| R4-07 | [Integrated verification and stage closure](plans/r4-organization.md#r4-07-integrated-verification-and-stage-closure) | Verified complete desktop flow, retained-library resource bounds, recovery and source preservation |

R4-01 is the first implementation slice. Establish group evolution and explicit source-removal
safety before exposing album writes. Migration backup belongs to R4-01; the later recovery settings
do not postpone protection of existing user data. Requirements and accepted control placement remain
with their product owners rather than being re-decided during implementation.
The [R4 evidence record](acceptance/r4-organization.md) covers the current internal foundation;
organization controls remain absent until their complete user paths are connected.

## Frozen R2c ownership

The [R2c closeout plan](plans/r2c-closeout.md) is historical and has no active execution queue.
The detailed runtime, native-client and platform observations remain in the
[acceptance index](acceptance/README.md). The pre-freeze queue is available at the
[frozen source revision](https://github.com/Cedarflake/Cedarflake-Ame/blob/5ea62d05eb7f84a64a12967fdba20fda9008f371/docs/roadmap.md).
Use the existing records when a relevant boundary changes; do not replay old pending instructions
merely because a new session reads them.

ADR [0024](architecture/0024-windows-change-driven-library-continuity.md) continues to own the
synchronization design. The former ADR 0023 model and its older acceptance slices remain historical.
The signed distribution and installer lifecycle retain their separate qualification under
[ADR 0027](architecture/0027-windows-installer-lifecycle.md). Signing, service installation and
Sandbox diagnostics are outside the R3 queue; future changes to those workflows retain their own
scope, verification and authorization. Historical gaps remain documented without becoming
additional R3 entry gates.

## Retained ownership and decomposition obligations

ADR [0025](architecture/0025-invariant-owned-workflow-modules.md) owns module boundaries. Review
the live owner before extending it; a forwarding wrapper or lower line count is not an extraction.
The retained debt constrains changes to the affected owners. It does not schedule a broad R2c
refactor before independent R3 work; extract the boundary needed by the selected delivery.

- Preserve typed ownership of scan execution/publication, mutually exclusive first-import versus
  existing-root journal opening, query/viewport generation, and committed-removal display refresh.
  Already extracted owners are regression boundaries, not another implementation checklist.
- Historical migrations now have shared schema-shape, contract-validation and compatibility-repair
  owners; production synchronization has dedicated runtime and Journal/Recovery owners alongside Live.
  Preserve these delivered boundaries rather than rescheduling their extraction.
- Move tests with their owners. Revisit viewer, selection, or sidebar decomposition only when an
  independent lifecycle requires it. Stable migration history, cohesive layout, and shared quality
  or release entrypoints are not split merely for size.
- The catalog's row serialization, physical-identity state reads and source reconciliation have
  dedicated owners. Preserve the reconciliation transaction across observation admission,
  generation allocation and retained-alias updates. This does not reorder the retained migration
  and synchronization-runtime split duties.
- A repair establishes the required typed boundary and records production/inline-test/dedicated-test
  size. If a larger split exceeds the active repair, retain it here as debt and add no further
  behavior to that debt owner until the required split is complete. This does not authorize a broad rewrite
  or expand the active R3 delivery.

## R2c exit and non-negotiable constraints

R2c is closed. The following constraints preserve its behavior while later stages evolve. The
[continuity contract](acceptance/library-continuity.md) retains the invariant and regression cases;
[quality gates](acceptance/quality-gates.md) owns execution commands and hosted/client boundaries.

- Ordinary supported changes converge through the change-driven production path; unchanged
  synchronization startup performs zero source-root enumeration/media opens/inventory creation.
  P0 retains its one-second P95 under P1/P2 work; continuous closed-process single changes retain
  the two-second P95 bound. R3 analysis reads are separately scheduled and accounted for.
- Cached content remains usable during continuity work. No stale result, partial scan, failed
  recovery, unavailable root or interrupted task can publish false completion, freshness or absence.
- Original media, original catalogs and durable user intent remain intact. No routine full scan,
  cloud hydration, catalog reset, weakened identity/lease guard or broader broker privilege may
  hide a failure. Schema changes retain forward migration and data-preservation requirements.
- Reuse the frozen R2c verification and run the applicable regression checks when later changes
  affect its boundaries. The historical final-audit and platform matrices retain their recorded
  results; they are not an active stage-wide acceptance cycle.
- Unsigned Debug/Release remain supported. Signing credentials and signing-dependent installation
  qualification do not become R3 prerequisites. Bounded read-only verification of known roots
  follows the standing repository policy. Source mutation, service installation, release publication
  and branch merging retain their applicable authorization requirements.

## Maintaining this roadmap

Update only stage disposition, the bounded queue, blockers, required decomposition, and exit
decisions here after checking current source and evidence. Replace an obsolete status instead of
appending another dated repair narrative. Put detailed procedures in the linked execution plan,
stable product behavior in product contracts, accepted technical decisions in ADRs, and commands,
test totals, measurements, failed attempts, and completed repair history in acceptance records.

Historical records and the pre-organization source are discoverable from the
[acceptance index](acceptance/README.md#historical-roadmap-provenance). They remain evidence rather
than a second active roadmap. Do not copy their implementation snapshots back into this file or
infer current acceptance from an old passing head.
