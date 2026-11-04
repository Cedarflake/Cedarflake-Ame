# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-11-04

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
| R6 | Perceptual similarity review | Planned next; not started |
| R7 | Structured and semantic discovery | Planned after R6 |
| R8 | Immutable physical-organization dry-run | Planned after R7 |
| R9 | Freshly authorized physical operations and recovery | Planned after R8 |
| R10 | Large-library maturity and release readiness | Later work; infrastructure may support earlier stages |

## Current stage and acceptance boundary

R4 and R5 are integrated; no later product stage is active. Their
[R4 merge evidence](acceptance/r4-organization.md#r4-merge-verification---2026-10-30) and
[R5 integrated evidence](acceptance/r5-closeout.md) retain coverage, accepted limits and merge authority.

Preserve the delivered R5 curated catalog, purpose restrictions, automatic analysis, legacy decisions,
archive compatibility and authorized model-installation recovery. The
[R5 requirements](product/r5-classification.md) own current behavior and withdrawn controls.
Reuse completed acceptance when its inputs are unchanged; new behavior receives its affected checks.

## Current execution queue

Project organization O1–O7 was merged into `main` on 2026-11-04 through
[PR 46](https://github.com/Cedarflake/Cedarflake-Ame/pull/46). The
[handover](acceptance/project-organization-continuity.md#organization-handover--2026-11-04) maps the
delivered scope to evidence. No later product stage is active.

The historical pause timeout remains an accepted observation with a
[recurrence-based follow-up](acceptance/project-organization-continuity.md#pause-timing-diagnosis--2026-11-04).
It did not recur in current Windows or bounded timing verification. It no longer
blocks this rollout; its cause is not recorded as fixed. R6 remains planned and has not started.

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
and scope. Signing and installed-service qualification keep their separate authorization and
acceptance owners under [ADR 0027](architecture/0027-windows-installer-lifecycle.md).

## Maintaining this roadmap

Replace obsolete status rather than appending a repair narrative. Keep active order, actual blockers,
required decomposition and exit decisions here. Put execution steps in plans, stable rules in their
contract or ADR, and completed work, measurements and failures in acceptance records. Follow the
[documentation standard](development/documentation.md) when updating any of these owners.
