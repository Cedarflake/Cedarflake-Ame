# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-11-09

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
| R8 | Authorized file organization and recovery | In progress on `codex/r8-file-organization`; incorporates former R9 |
| R10 | Large-library maturity and release readiness | Later work; infrastructure may support earlier stages |

R8 combines the former R8 and R9 scopes into one executable workflow without an organization-preview
feature. R9 is no longer a separate stage; R10 keeps its identifier. The
[merged scope](product/organization-workflows.md#r8---file-organization-and-recovery) owns behavior;
the [execution plan](plans/r8-file-organization.md) defines Windows-backed implementation slices;
the [verification procedure](acceptance/r8-file-operations.md) defines guest-contained Windows Sandbox
and hosted Windows fixture checks. R8-01 native adapter admission is recorded; R8-02 durable
execution and recovery is next.

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

R8 implementation is active on `codex/r8-file-organization`. R8-01 Windows capability admission
under [ADR 0042](architecture/0042-windows-file-operations.md) has scoped
[native evidence](acceptance/r8-file-operations.md#r8-01-native-capability-admission--2026-11-09).
Continue through R8-02 to R8-05 in the
[execution plan](plans/r8-file-organization.md#implementation-slices): connect durable
execution and recovery, expose desktop actions, add organization rules, and complete integrated
verification. Native behavior that cannot preserve the required source-safety guarantees blocks the
affected action until resolved; it does not authorize a weaker fallback. Exit requires the connected
workflow and scoped [verification](acceptance/r8-file-operations.md), without organization preview.

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
