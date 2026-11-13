# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-11-13

This file owns current stage disposition, execution order, blockers, decomposition obligations and
exit decisions. Read the linked active plan before implementation. Product requirements, technical
decisions, procedures and historical evidence remain in their named owners in the
[documentation index](README.md).

## Product direction

Ame is a local-first Windows image-organizing workbench for large personal libraries. Delivery
progresses from trustworthy catalog and browsing to exact duplicates, durable virtual organization,
machine-assisted discovery and separately authorized physical operations. Cataloging, browsing,
analysis and virtual organization preserve original media.

The [workbench contract](product/workbench.md) owns concepts and exclusions;
[organization workflows](product/organization-workflows.md) retain the complete R3–R10 scope.

## Delivery sequence

| Stage | Outcome | Disposition |
| --- | --- | --- |
| R0 | Windows picker, catalog and preview | Accepted foundation |
| R1 | Progressive, resumable multi-root catalog | Accepted foundation |
| R2a | Unified gallery and connected controls | Accepted foundation |
| R2b | Continuous browsing, viewing and storage | Accepted; preserve [foundation boundaries](acceptance/gallery-foundation.md) |
| R2c | Change-driven continuity during browsing | Completed and frozen on 2026-10-16 |
| R3 | Exact duplicates and physical-location inspection | Completed on 2026-10-17 |
| R4 | Favorites, albums and durable organization progress | Merged into `main` on 2026-10-30 |
| R5 | Metadata, smart albums and automatic initial screening | Merged into `main` on 2026-11-03 |
| R6 | Perceptual similarity review | Merged into `main` on 2026-11-05 |
| R7 | Structured and semantic discovery | Merged into `main` on 2026-11-08 through [PR 53](https://github.com/Cedarflake/Cedarflake-Ame/pull/53) |
| R8 | Authorized file organization and recovery | Merged into `main` on 2026-11-11 through [PR 57](https://github.com/Cedarflake/Cedarflake-Ame/pull/57); incorporates former R9 |
| R9 | Move album files into year-based folders | Merged into `main` on 2026-11-12 through [PR 60](https://github.com/Cedarflake/Cedarflake-Ame/pull/60) |
| R10 | Large-library maturity and release readiness | Planned for `v0.1.0-beta.1`; R10-01 through R10-05 await implementation |

R8 combines the former R8 and R9 scopes into one executable workflow without an organization-preview
feature. The R9 identifier now denotes the separate album-by-year movement scope below; the former
R9 recovery work remains delivered within R8. R10 keeps its identifier. The
[merged scope](product/organization-workflows.md#r8---file-organization-and-recovery) owns behavior;
the [execution plan](plans/r8-file-organization.md) defines Windows-backed implementation slices;
the [verification procedure](acceptance/r8-file-operations.md) defines guest-contained Windows Sandbox
and hosted Windows fixture checks. R8-01 through R8-05 are integrated; the
[closeout record](acceptance/r8-file-operations.md#r8-integration--2026-11-11) retains final review,
current-head verification and the accepted Windows recycling limit.

## Current stage and acceptance boundary

R4 and R5 are integrated. Their
[R4 merge evidence](acceptance/r4-organization.md#r4-merge-verification---2026-10-30) and
[R5 integrated evidence](acceptance/r5-closeout.md) retain coverage, accepted limits and merge authority.

R6-01 through R6-04 are integrated through
[PR 47](https://github.com/Cedarflake/Cedarflake-Ame/pull/47). The
[implementation plan](plans/r6-similar-images.md) retains the slice definitions; the
[closeout record](acceptance/r6-similar-images.md#r6-closeout) maps them to evidence and accepted limits.
[ADR 0038](architecture/0038-bounded-perceptual-similarity.md) owns the comparison boundary.

Preserve the delivered R5 curated catalog, purpose restrictions, automatic analysis, legacy decisions,
archive compatibility and authorized model-installation recovery. The
[R5 requirements](product/r5-classification.md) own current behavior and withdrawn controls.
Reuse completed acceptance when its inputs are unchanged; new behavior receives its affected checks.

## Current execution queue

R8 and its follow-ups, including the loading-feedback update, and R9 album file organization are
integrated. The current release is `v0.1.0-alpha.3`. Transition-performance preparation precedes
R10's Beta release phase. The R10
[product scope](product/organization-workflows.md#r10---large-library-maturity-and-release-readiness)
is decomposed in the [execution plan](plans/r10-beta-readiness.md). Implementation has not started.

### Gallery group discovery

The [follow-up plan](plans/gallery-group-discovery.md) was integrated through
[PR 62](https://github.com/Cedarflake/Cedarflake-Ame/pull/62). It delivers scope-consistent semantic
results, exclusive grouped-gallery filters, member and cleanup actions, and labeled group menus.
The [integration record](acceptance/r8-file-operations.md#gallery-group-discovery-integration--2026-11-13)
retains audit, verification and merge boundaries. Remaining query waits belong to R10; this
maintenance does not reopen the completed R7/R8 stages.

### Transition performance

The [motion-performance plan](plans/motion-performance.md) is active on
`codex/motion-performance`. MP0/MP2 have assessments for all transition families, including first-open
viewer and native size/state changes in the viewer and both gallery layouts. MP1 retains card
content, paints sidebar clipping/translation without relayout, reuses viewer compositing layers,
reduces compact-manifest comparison/row calculation, yields between snapshot conversion chunks,
and separates Dart frame scheduling from the Windows message loop.
MP3 local affected-path verification and source review are recorded; exact-head hosted CI remains
outstanding in [PR 63](https://github.com/Cedarflake/Cedarflake-Ame/pull/63). The
[measurement record](acceptance/material-motion.md#transition-performance--2026-11-13) retains gains,
first-use shader/compositor costs, remaining frame outliers and the limit on prolonged held-mouse
resizing coverage. No animation timing, appearance or interaction policy changed.
The scope covers existing transitions across the application while preserving their presentation,
behavior and styling. Measurements select the optimization targets; R10's broader responsiveness,
resource qualification and Beta release remain separate.

### R10 — Beta readiness

R10-01 through R10-05 proceed in order: release responsiveness, bounded resources and recovery,
upgrade compatibility, diagnostics and user guidance, then the first Beta candidate/publication.
All five slices are pending. Begin with measured Release/Profile behavior on the retained library;
reuse delivered R0–R9 capabilities and current valid evidence. The plan owns the initial latency
targets, detailed workloads and completion evidence.

First-Beta exit requires the agreed performance and recovery results, preserved data across upgrade,
usable diagnostics, no known blocking defect and passing applicable candidate gates. Both MSI and
portable ZIP are required. Signing, additional locales, an automatic updater and the previously
deferred constrained-device optimization are not first-Beta prerequisites. Million-item structural
adaptation remains conditional on the existing measured limits.

### Loading feedback

The [loading-indicator plan](plans/loading-indicator.md), dialog/viewer, selection and responsiveness
follow-ups were merged through [PR 59](https://github.com/Cedarflake/Cedarflake-Ame/pull/59).
The [integration record](acceptance/material-motion.md#loading-feedback-integration--2026-11-12)
retains merge authority, verification boundaries and remaining query latency. The local and remote
work branches have been deleted.

### R9 — Album files by year

The [R9 product scope](product/organization-workflows.md#r9---album-files-by-year) and
[execution plan](plans/r9-album-files-by-year.md) use gallery dates and album/year destinations.
The plan retains existing metadata and Windows file operations, with ExifTool evaluated as a
comparison tool rather than a prerequisite. Implementation and disposable Sandbox verification are
recorded in the [R9 acceptance section](acceptance/r8-file-operations.md#r9-album-files-by-year--2026-11-12).
R9 is integrated through [PR 60](https://github.com/Cedarflake/Cedarflake-Ame/pull/60); the
[integration record](acceptance/r8-file-operations.md#r9-integration--2026-11-12) retains the merged
revision and verification boundary.

### Duplicate cleanup entry points

The [R8 follow-up](plans/r8-file-organization.md#duplicate-cleanup-entry-points) was merged through
[PR 58](https://github.com/Cedarflake/Cedarflake-Ame/pull/58) on 2026-11-11. Its
[acceptance record](acceptance/r8-file-operations.md#duplicate-cleanup-follow-up--2026-11-11)
owns group/member/all-group cleanup and retained selection evidence. The
[integration record](acceptance/material-motion.md#integration--2026-11-11) retains the authorized
merge before final CI completion and the remaining continuous held-drag observation gap.

### Material motion adoption

The [motion plan](plans/material-motion.md) M1–M4 and the subsequent viewer, timeline and loading
changes were merged through [PR 58](https://github.com/Cedarflake/Cedarflake-Ame/pull/58).
The [integration record](acceptance/material-motion.md#integration--2026-11-11) links their
verification boundaries. Both work branches have been deleted locally and remotely.

### Existing delivery records

Release artifact reuse is implemented in [PR 56](https://github.com/Cedarflake/Cedarflake-Ame/pull/56).
The [plan](plans/release-artifact-reuse.md) retains its scope; the
[evidence](acceptance/unsigned-windows-release.md#release-artifact-continuation--2026-11-09)
records scoped CI and read-only artifact reuse.

Gallery navigation stability was integrated on 2026-11-09 through
[PR 55](https://github.com/Cedarflake/Cedarflake-Ame/pull/55). The
[repair plan](plans/gallery-navigation-stability.md) retains its implementation boundaries; the
[evidence and remaining limits](acceptance/image-rendering.md#continuous-timeline-dragging-and-coherent-layout-publication)
own interaction, query-cost and hosted verification.

The standalone Material migration was integrated on 2026-11-08 through
[PR 54](https://github.com/Cedarflake/Cedarflake-Ame/pull/54).
[ADR 0001](architecture/0001-flutter-rust-desktop-stack.md#standalone-material-library)
owns package compatibility and preserved desktop behavior; the [evidence record](acceptance/material-ui-migration.md)
owns verification and the authorized merge before CI completion.

GPU analysis acceleration was merged into `main` on 2026-11-07 through
[PR 52](https://github.com/Cedarflake/Cedarflake-Ame/pull/52). The
[evidence](acceptance/gpu-analysis-acceleration.md#integration-disposition--2026-11-07) retains local
verification, review scope and the authorized merge before CI completion.

Post-R6 maintenance is integrated into `main`. The owning records retain the local regression and
rebuilt-client evidence for [similarity throughput](acceptance/r6-similar-images.md#development-analysis-throughput--2026-11-05),
[group entry and duplicate icons](acceptance/r6-similar-images.md#similar-group-member-query-latency--2026-11-05),
[viewer categories and spacing](acceptance/r5-desktop-classification.md#viewer-classification-continuity--2026-11-05),
[date navigation and filmstrip paging](acceptance/image-rendering.md#time-navigation-after-catalog-publication--2026-11-05),
and [hot-restart attachment and shutdown](acceptance/desktop-lifecycle-continuity.md).
The generic `其他` category has been removed from browsing. Similarity parameters and source safety
remain unchanged. These repairs do not reopen R6; R7 retains its own verification boundary below.

Project organization O1–O7 was merged into `main` on 2026-11-04 through
[PR 46](https://github.com/Cedarflake/Cedarflake-Ame/pull/46). The
[handover](acceptance/project-organization-continuity.md#organization-handover--2026-11-04) maps the
delivered scope to evidence.

R7's delivered scope follows the [semantic-discovery plan](plans/r7-semantic-discovery.md):
composed filters, shared feature/index coverage, text retrieval, related-image discovery and desktop
acceptance. It reuses SigLIP 2 and the existing default-enabled GPU image runtime;
multi-model selection and the larger FG-CLIP 2 evaluation are outside this scope.

The requested CI reduction is implemented: quick/full/nightly-release selection, affected Flutter
suites and removal of incidental assertions. [CI evidence](acceptance/r7-semantic-discovery.md#ci-reduction--2026-11-07)
retains its measured results and revision limits. Preserve that policy without another broad test-cleanup project.

R6 retains the paired engine with accepted visual-layout discrimination and heavier-crop limits.
Cloud-primary runs, constrained-client measurements and optimization remain deferred outside R6.
The [closeout record](acceptance/r6-similar-images.md#r6-closeout) retains their evidence and disposition.

The historical pause timeout remains an accepted observation with a
[recurrence-based follow-up](acceptance/project-organization-continuity.md#pause-timing-diagnosis--2026-11-04).
It did not recur in current Windows or bounded timing verification. It no longer
blocks this rollout; its cause is not recorded as fixed.

## Project organization after R5

[ADR 0037](architecture/0037-feature-module-dependency-boundaries.md) owns dependencies and composition;
the [repository layout](development/repository-layout.md) owns placement. The
[execution plan](plans/project-organization.md) defines O1–O7 and each slice's exit conditions.

| Scope | Disposition |
| --- | --- |
| O1–O7 | Merged into `main` on 2026-11-04; [handover](acceptance/project-organization-continuity.md#organization-handover--2026-11-04) |

New and materially changed modules follow the accepted dependency and layout contracts. Keep the
fixed legacy inventory as provenance and retain only the exact admitted technical boundaries.
Preserve established scan, synchronization, analysis, query and source-read lifetimes.

## Deferred constrained-device rendering

The user deferred first-viewer and mode-entry graphics peaks on the two-core, 8-GiB cloud-primary
client on 2026-10-30. This remains separate from project organization and does not reopen R4 or R5.
Its [retained diagnosis](acceptance/r4-organization.md#constrained-device-first-use-rendering---2026-10-30)
owns measurements and rejected candidates.

When selected, resume from those traces and compare the same Skia interaction on cloud-primary and
local-primary. Preserve accepted controls and motion, full-resolution viewing, cancellation and
source leases. Do not repeat warm-up, reduced-motion or cache candidates without new evidence.
This follow-up keeps its existing scope without another full stage-acceptance cycle.

## Retained ownership and decomposition obligations

[ADR 0025](architecture/0025-invariant-owned-workflow-modules.md) and its scoped owner contracts bind
any affected extension. Existing extractions are regression boundaries, not tasks to repeat. A new
multi-responsibility extension must first establish its typed owner; record only an actual unfinished
larger split here, with affected scope and resume condition. This documentation reorganization
schedules no additional broad split.

## R8 implementation sequence

R8-01 through R8-05 are integrated through [PR 57](https://github.com/Cedarflake/Cedarflake-Ame/pull/57).
The [plan](plans/r8-file-organization.md#implementation-slices) retains the combined R8/R9 scope;
the [integration record](acceptance/r8-file-operations.md#r8-integration--2026-11-11) owns review,
Sandbox and desktop evidence, final CI and the merged revision. Preserve the explicit deletion scope,
changed-file rejection and permanent-deletion handle guarantee under
[ADR 0042](architecture/0042-windows-file-operations.md), including its accepted last-check recycling
race. Organization preview remains excluded. The [dialog inventory](acceptance/r8-file-operations.md#dialog-screenshot-inventory--2026-11-10)
retains the reviewed surfaces; the shared [templates](product/gallery/dialogs.md) own their requirements.

## R7 implementation sequence

R7-01 through R7-05 are integrated through [PR 53](https://github.com/Cedarflake/Cedarflake-Ame/pull/53).
The [plan](plans/r7-semantic-discovery.md#implementation-slices) retains slice definitions;
the [integration record](acceptance/r7-semantic-discovery.md#integrated-verification--2026-11-08)
owns review, desktop verification, CI provenance and the authorized merge before the latest CI
finished. Preserve the measured partial-index coverage and deferred constrained-device limits in
that acceptance record.

## R4 implementation sequence

R4-01 through R4-07 are integrated. The [execution plan](plans/r4-organization.md) retains slice
definitions; the [acceptance record](acceptance/r4-organization.md) owns implementation and merge evidence.

## R5 implementation sequence

R5-01 through R5-07 are integrated. The [execution plan](plans/r5-classification.md) retains slice
definitions; the [closeout record](acceptance/r5-closeout.md) maps each to evidence and accepted limits.

## Frozen R2c ownership

R2c's [closeout plan](plans/r2c-closeout.md) and
[freeze record](acceptance/r2c-update-checkpoint.md#r2c-freeze-on-2026-10-16) retain its historical queue.
They do not start another closeout cycle. [Completed maintenance](acceptance/completed-maintenance.md)
indexes subsequent deliveries, deferred observations and original roadmap provenance.

## R2c exit and non-negotiable constraints

Preserve the [continuity contract](acceptance/library-continuity.md) and
[ADR 0024](architecture/0024-windows-change-driven-library-continuity.md): trustworthy publication,
bounded change-driven work, usable retained browsing and source safety. Evidence retains its revision
and scope. Installed-service qualification retains its separate authorization and acceptance owner under
[ADR 0027](architecture/0027-windows-installer-lifecycle.md). Code signing is optional and is not
an R10 completion requirement.

## Maintaining this roadmap

Replace obsolete status rather than appending a repair narrative. Keep active order, actual blockers,
required decomposition and exit decisions here. Put execution steps in plans, stable rules in their
contract or ADR, and completed work, measurements and failures in acceptance records. Follow the
[documentation standard](development/documentation.md) when updating any of these owners.
