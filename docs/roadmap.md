# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-10-29

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
| R4 | Independent Favorites, flat group-based albums, and durable organization progress | Active; R4-07 integrated verification in progress |
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
queue-layout checkpoint passed hosted verification, including its cancelled-request fixture
correction. The subsequent conflict and batch-status head failed two availability-inventory assertions;
the exact domain-module inventory is now corrected with passing focused admission and rejection
checks. The published inventory repair and subsequent native-check commit both passed hosted
verification.
R4-05 now connects viewer Favorite/unfavorite, undo/retry and the Favorite collection.
Controlled desktop checks cover transient Favorite feedback without notification history,
create-in-picker assignment, defer/ignore/restore, saved continuation, collection persistence and
ordinary-scope restoration after mode exit. Album management, viewer assignment/movement/removal,
sidebar scopes and durable album settings are connected and covered by focused regressions;
native undo, album management, settings and stale-save retry now have controlled client evidence. The
query-wide selection foundation is
implemented with bounded staging, atomic group writes and cancellation; its focused adapter and
session checks pass. Gallery selection now connects whole-query album/Favorite commands, shared
picker creation, cancellation and transient undo; focused Rust and Flutter checks pass. The shared
batch path has controlled desktop evidence for whole-query album assignment,
picker cancellation, restart retention and batch Favorite/unfavorite without history entries.
Preparation feedback ends when
the picker takes ownership. Organization mode connects its menu entry, shared counts, queue controls,
contextual actions and durable
continuation through the existing gallery and viewer. Both queue layouts use continuous rows;
focused regressions and controlled desktop checks pass. Persistent conflict labels, involved-album
inspection and resolution are connected through the shared notification, deferred queue and picker.
Controlled desktop checks cover discovery after notice expiry, cancellation, committed resolution
and restart retention; focused checks cover retired albums, pagination, failed saves and undo.
Committed album retirement now leaves affected active and saved browsing scopes before refreshing;
controlled Windows integration verifies deletion and undo, with focused publication and lifecycle
regressions. Native unavailable-item integration also preserves pending decisions while navigating
and retrying a blocked original. These desktop workflows have passed their controlled client and
hosted checks. R4-06 now implements portable archive export, restoration admission and catalog-evidence
association preview. Catalog publication now reattaches rebuilt versions through compatible native
evidence, with focused group, conflict, source-removal and migration checks. Atomic archive
application now connects the application and typed bridge, preserving current decisions, operation
history, queue order, continuation and unmatched evidence. Focused restore, cancellation, migration
and queue-cost checks pass. Settings export, inspection and guarded restoration are connected,
with focused controller/widget checks and native export, matched/unmatched restore and matched-profile
restart evidence.
Manual unresolved-record inspection, guarded cleanup and settings confirmation are connected;
focused boundary checks pass, and native age selection and confirmation counts are verified.
Repeated native restore, unmatched-profile restart and late association after importing the
retained test corpus now have controlled client evidence. Native cleanup publication, post-cleanup
reimport and restart leave cleared decisions absent while retaining album definitions and source media.
The atomic-restore checkpoint failed hosted historical upgrade checks because shared reconciliation
read archive tables before their migration. Schema-aware local versus imported authority now passes
focused upgrade, rollback and current-catalog rejection checks. Hosted verification passes the
repaired upgrade paths and expected-table inventory. The later archive checkpoint exposes two
static flow-control violations and a source-management integration setup error; both pass subsequent
hosted verification. The settings-page control inventory now includes cleanup's age selector;
its focused and hosted checks pass. The ordinary-journal import failure has recurred;
independent fallback now retires the superseded first-import execution owner, with focused
handoff/restart/rollback checks passing. Subsequent hosted synchronization passes; retain the
numeric native-read diagnostic for recurrence. The cleanup checkpoint also exposed a one-copy
publication cost regression. Schema 55's maintained unresolved-association lookup restores bounded
group updates, with migration, cleanup/restoration, query-growth and exact-head hosted checks passing.
The R4-05 control follow-up adds a bottom album tray, state-aware Favorite/album commands and
persistent chronological undo/redo across viewed groups. Focused regressions and the controlled
desktop move/remove and restart paths pass. Schema-56 recovery-shape admission, Dart style and
stale picker/archive test expectations are repaired with focused checks; hosted verification now
passes through product commit `7e11d8a7`. Deferred/ignored controls expose state-aware removal in
gallery selection, detailed viewing and the album tray, with native removal/undo evidence in the
R4 acceptance record.
Owning-query repairs reduce retained-library album, Favorite and organization-queue read costs.
The complete Debug client verifies populated collections, single-group Favorite publication,
new arrivals and an empty deferred queue on a full catalog copy. Optimized-client timing now covers
collection reads, committed-save publication, empty queues and saved continuation. Preview-usage
publication now yields to an active writer, with focused contention checks passing. The current Profile
client displays an album in 4.05 seconds during the write, before saving finishes, and prepares/saves
47,832 Favorite changes in 27.00 seconds. Remaining query/publication waits and constrained-device
resource acceptance remain open. The subsequent same-package cloud run completes its 12,007-file,
11,985-group workflow after production metadata reconciliation. It saves 11,984 Favorite changes in
7.895 seconds while opening an album in 1.505 seconds, with at most 500 retained assets. Its
build/raster P95 of 20.019/73.751 ms leaves constrained-device rendering diagnosis open. Per-interaction
measurement supports a shared-surface clipping repair, with lower sidebar and mode-entry costs;
queue/viewer transitions and retained-library read waits remain open. The related sidebar-reflow
edge-position repair passes focused and native checks; preserve that boundary while continuing
constrained-device transition and read-latency work.
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

The requested R4 sidebar-management follow-up connects saved drag ordering for albums and imported
top-level source folders, icon-initiated multi-selection and batch album deletion/source removal.
Library and Albums use matching destination and section-heading rows, with creation/import at the
heading's trailing edge. Focused checks and controlled native batch interactions pass. Album and
image-card context menus now reuse existing commands and current-state decisions, with focused and
native checks for their connected entrypoints. Compact selection controls and primary-workspace
selection reset retain their own verified boundary.
Child-folder ordering is unchanged. These requested controls precede the
remaining retained-library performance work and retain the existing source-safety and recovery rules.

The requested notification maintenance standardizes bottom feedback and image-preview top notices
through shared templates under the [gallery contract](product/gallery-ui.md#feedback-templates).
Its [scoped verification](acceptance/notification-feedback.md) retains the affected checks and client
evidence. This does not change R4's remaining performance order or stage boundary.

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
ordinary-mode edits, stale saves, rollback, group evolution and source lifecycle. R4-05 connects
the viewer Favorite path, album management, viewer membership commands, collection entries and
album settings connected. The query-wide selection command boundary passes focused adapter and
session checks. Controlled desktop checks cover the core organization-mode loop, restart,
Favorite history behavior, whole-query album assignment and restored ordinary context. Batch
Favorite commands, durable default-album settings, management, undo and stale-save retry now have
controlled client evidence. Gallery selection and the core
organization-mode workflows and persistent conflict inspection/resolution are connected. Controlled
desktop checks cover conflict discovery, cancellation, resolution, restart and unavailable-image
navigation/retry without completing pending work. The corrected availability source inventory and
native workflows have passed hosted verification. R4-06 implements the versioned archive codec,
snapshot export, staged restoration validation and bounded association preview behind the typed
bridge. Native-version reassociation is connected to the existing publication transaction, including
cross-root aliases and contradictory-content rejection. Atomic restore and repeat/conflict handling
are implemented through the application and typed bridge. Restored queue order and continuation,
retained R3 choices and source-removal precedence have focused regression coverage.
Native export, matched restore, restart of the matched
profile and unmatched restore into an empty profile now have controlled client evidence. Cleanup
age selection and confirmation counts are verified. Repeated restoration preserves the same
decisions, and reimport associates all retained test records without duplicate albums or groups;
both unmatched and associated records survive restart. Native cleanup publication, reimport and
restart now verify that cleared decisions remain absent while album definitions and source media
survive. Manual unresolved-record cleanup passes its focused boundary checks; R4-06 is verified.
Hosted verification passes the schema-55 association-cost repair and earlier follow-ups. Retain the
native journal-read diagnostic for recurrence and repair the retained-library query latency exposed
by the catalog-only baseline before integrated performance acceptance.
Indexed representative evidence, bounded connection caches, non-repeated group scope and
published-source query plans are implemented. Shared chronological membership/count reads now take
about 2.37 seconds for the retained album and 1.90–1.94 seconds for the one-item pending queue,
including its shared progress counts. Empty deferred/ignored reads take 0.66–0.68 seconds including
counts. Focused query regressions and the retained-catalog workload pass. Exact-head hosted run
`40228493472` passes all 14 applicable jobs on `491d0692`, including the timeline change, schema-57
recovery admission and retained-update loading regression.
The full Debug client preserves 47,833 organized groups while startup
synchronization admits 12 new pending groups. Its populated album/Favorite, single-group unfavorite
and empty deferred-queue interactions pass with sampled resources. Optimized-client measurements
locate additional waits in collection reads, saved continuation and committed-list publication.
Preview last-use publication now yields immediately to an active organization writer, with bounded
read/write-contention regressions. Empty album, Favorite, deferred and ignored reads now avoid
location traversal when no matching decisions exist, with focused and retained-catalog checks passing.
Chronological timelines reuse one scoped group relation for representatives and physical counts.
The retained Profile client completes the measured workflow; concurrent album publication takes
4.05 seconds while the writer remains active. Position-preserving reads now reuse proven fallback
ordinals and reject ineligible anchor membership before expanding scoped groups. The retained
catalog comparison reduces removed-anchor reads from 7.8 to 3.54–3.58 seconds; the affected client
refresh falls from 9.72 to 5.72 seconds, with unchanged position and count assertions. The current
standalone client completes the retained workflow with collection entries of 3.46–4.59 seconds,
single-group saved-action feedback of 72 ms, committed-list publication of 5.63 seconds and final
empty-Favorite publication of 6.39 seconds. Carry these measured waits into integrated resource
acceptance; keep collection entry, anchored refresh and writer timing separate. Successful
representative anchors now reuse their transaction-local group proof while retaining point scope
checks. Focused and paired native verification pass, with lower retained-anchor and album-refresh
costs; remaining publication and removed-anchor waits stay in the R4-07 queue.
Folded collection queries also reuse their resolved canonical record for membership and grouping.
Focused query/batch checks and the retained native flow pass; its album refresh changes from about
7.2 to 6.7 seconds. Keep the remaining read cost and constrained-device rendering in R4-07.
Preferred-copy queries now gate representative evidence through an indexed location/asset lookup.
Schema-59 migration and bounded-history regressions pass. Alternating retained-catalog reads reduce
album/Favorite grouping by 22–23%, with pending reads essentially unchanged. The paired native
album refresh changes from 3.80 to 3.51 seconds while preserving position, counts and decoded
previews. Continue with the remaining publication waits, then constrained-device rendering and
integrated acceptance in the existing order.
Shared queue counts now reuse resolved canonical membership instead of repeating record lookups.
The retained-catalog counts remain 47,845 total/12 pending/47,833 organized while their reads fall
from 727–760 ms to 535–555 ms. Scoped, unbound-copy, content-change and snapshot regressions pass.
The current Skia client also completes the retained cloud-primary transition workload with its
original catalog unchanged. Most measured menu/queue frames are inexpensive, but first-viewer
and organization-entry raster spikes remain. Continue the retained-read/publication work, then
resolve these constrained-device peaks before R4-07 closure; preserve the accepted interface,
full-resolution viewing and source-read lifetime. Historical root fixtures now match the original
schema; their upgrade, sidebar-order and exact-commit hosted checks pass. Notification undo now
publishes album retirement, workspace switching serializes mode entry, and schema 60 preserves
the mode-entry arrival boundary across archive restoration. Their focused regressions and complete
exact-commit hosted verification pass. The current retained-scale native workflow passes with
collection entry around 2 seconds, single-group feedback at 66 ms, committed-list publication at
3.09 seconds and a 500-asset client window, including browsing during the full-library writer.
The read-plan alternative regressed dense collection timelines and was discarded; retain the
published-source-first implementation and its expanded historical-location regression. Continue
with constrained-device frame peaks before final R4-07 closure. The
[first-use rendering evidence](acceptance/r4-organization.md#constrained-device-first-use-rendering---2026-10-30)
narrows this gap to viewer and mode-entry graphics upload/synchronization; the warm-up, motion and
cache experiments provide no adopted repair. The reported album-title loading flash is repaired,
with delayed-query widget, native frame and complete hosted checks passing. The
[retained closeout](acceptance/r4-organization.md#retained-collection-closeout---2026-10-30)
records the current native measurements and resource bounds.
Bounded statement reuse reduces full-library album preparation/commit from 13.2/62.6 seconds to
3.6/32.4 seconds and Favorite preparation/commit from 15.5/38.2 seconds to 5.1/13.6 seconds on the
same retained catalog. Guard, rollback, undo and final-state checks pass. Keep visible publication
and remaining read/write waits separate from these backend measurements. Header-height changes now
preserve the viewport anchor in the current frame without repeatedly restarting photo reflow;
focused and Windows Profile component checks pass. The current complete client records 5,254 frames
with build/raster P95 of 0.818/1.475 ms and at most 500 retained assets. These aggregate frame results
do not close the measured publication waits or constrained-device acceptance.
Sort-driven date, name and current-entry arrival-range rails have focused regression and build
coverage. Native interaction verifies date years/ticks, numeric name sections, near-top-to-top
dragging and current-entry arrival ranges. Single-image ranges remain visible when new and earlier
images share a row, and re-entry resets the arrival boundary. Schema-57 recovery
admission passes focused repair, rejection, upgrade and hosted checks.
Full-catalog cancellation verifies prompt return during preparation and halfway through a
47,833-group write, with all durable records unchanged after reopen. Native preparation and
in-flight save cancellation now also preserve all durable organization tables across restart.
The native check exposed a post-commit form wait: successful actions now return their receipt
without waiting for the independently owned gallery refresh. Focused and native checks cover the
commit, cancellation and refresh-retry boundary. Exact-head hosted run `40515066782` passes all
14 applicable jobs and the aggregate Windows Gate on `737e35c9`.
The original recovery profile remains intact. Native inspection and cancellation use one settings
entry with range selection inside the dialog. Authorized cleanup in an independent restore fixture
now verifies bottom-notification publication and non-resurrection after reimport and restart.
The user-confirmed cloud-primary connection is verified. A manifest-verified standalone package
preserves its older checkout and installed SDK. The independent catalog completes 19,006-group
album/Favorite preparation and writes, but the first preview fails its 90-second deadline with
`preview_source_missing` before decoding. Metadata checks find available year-based source files
and stale records under an absent subdirectory. Production synchronization reconciles a fresh copy
to 12,007 files and 11,985 groups in 146.332 seconds; the unchanged package then completes the client
workflow. Album/Favorite entry takes 1.86–2.98 seconds, and saved continuation displays its image in
2.05 seconds. Per-interaction diagnosis now isolates unnecessary full-surface clipping and background
work. Its shared-surface repair lowers measured sidebar and mode-entry raster costs, while queue and
viewer transitions remain costly. The geometry owner now retains the gallery's start/end position
through sidebar reflow, with focused and native verification. Continue diagnosis of remaining
transition and collection-read costs before responsiveness acceptance. Original catalogs remain
unchanged and the normal local controlled test window has been restored.
The authorized SDK upgrade pins Flutter stable 3.47.8 across local and hosted builds. Matched Release
visual checks pass on local-primary and cloud-primary, including the latter's Microsoft Basic Render
adapter. The stable Impeller path displays correctly but increases cloud transition cost and memory
use in the matched comparison. Ame therefore explicitly retains Skia through Flutter's public API.
SDK lifecycle, image I/O scheduling, analysis, formatting and the final Release build pass;
exact-commit hosted checks remain in this delivery. Completed cloud runtime packages are retired;
one working catalog/profile and compact reports remain. The R4 acceptance record owns measurements
and provenance; retained-library query latency remains the next performance item.
The requested primary Albums destination is connected under R4-05: Library, Albums and
Duplicates retain separate navigation roles with one shared gallery, and Favorites belongs to the
Albums sidebar. Organization mode is scoped to all-source Library or the selected source/folder,
with collection entry and cross-scope continuation excluded. Ordinary workspace switches reset
selection and position; organizing Library retains its mode and browse context across primary-sidebar
visits to Albums, Duplicates and Settings.
Its focused checks and controlled desktop verification belong to the R4 evidence record.
R4-07 retains the complete integrated flow and retained-library
resource measurements. The
[execution plan](plans/r4-organization.md) defines the dependencies, work and exit conditions below;
this roadmap owns their order and status. The complete R4 stage remains unaccepted.

| Order | Slice | Required result |
| --- | --- | --- |
| R4-01 | [Durable group decisions and command targets](plans/r4-organization.md#r4-01-durable-group-decisions-and-command-targets) | Stable organization identity, whole-group commands, transactional records and recoverable migration foundation |
| R4-02 | [Organization transitions and group reconciliation](plans/r4-organization.md#r4-02-organization-transitions-and-group-reconciliation) | Single-album/Favorite decisions, shared progress, undo, merges/splits and safe source lifecycle handling |
| R4-03 | [Bounded album and Favorite queries](plans/r4-organization.md#r4-03-bounded-album-and-favorite-queries) | Consistent, bounded gallery scopes, pages, counts, layout and viewer navigation |
| R4-04 | [Continuous queues and durable continuation](plans/r4-organization.md#r4-04-continuous-queues-and-durable-continuation) | Inherited gallery sorting, optional new-first ordering, stable logical continuation and shared counts across ordinary and organization modes |
| R4-05 | [Connected albums, Favorites and organization mode](plans/r4-organization.md#r4-05-connected-albums-favorites-and-organization-mode) | Ordinary album/Favorite workflows and accepted organization controls, continuation and persistent conflict resolution |
| R4-06 | [Export, restoration and unresolved-record cleanup](plans/r4-organization.md#r4-06-export-restoration-and-unresolved-record-cleanup) | Versioned portable recovery, preserved rebuilds and explicit cleanup without resurrection on reimport |
| R4-07 | [Integrated verification and stage closure](plans/r4-organization.md#r4-07-integrated-verification-and-stage-closure) | Verified complete desktop flow, retained-library resource bounds, recovery and source preservation |

R4-01 is the first implementation slice. Establish group evolution and explicit source-removal
safety before exposing album writes. Migration backup belongs to R4-01; the later recovery settings
do not postpone protection of existing user data. Requirements and accepted control placement remain
with their product owners rather than being re-decided during implementation.
The [R4 evidence record](acceptance/r4-organization.md) covers internal boundaries and the connected
desktop workflows. Recovery settings have native export and matched/unmatched restore evidence;
cleanup, reimport and integrated cancellation also have native evidence. Retained-library resource
verification remains outstanding.

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
