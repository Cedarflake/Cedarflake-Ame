# R10 large-library maturity and Beta readiness

Status: scoped execution plan. The [roadmap](../roadmap.md#r10--beta-readiness) owns activation,
slice status and the release decision. The
[product scope](../product/organization-workflows.md#r10---large-library-maturity-and-release-readiness)
owns R10's boundaries. The proposed first Beta is `v0.1.0-beta.1` on Windows 11 x64.

## Outcome and retained foundation

Make the delivered catalog, discovery and organization workflows suitable for sustained Beta use.
Prioritize observed responsiveness, bounded resource use, recovery and upgrading an existing library.
Preserve the current Material surfaces, SigLIP 2 integration and Windows file-operation policies.

R8 file operations, R9 album/year movement, MSI/ZIP packaging, migration backups and analysis-worker
admission already exist. Inspect their remaining integration gaps rather than reimplementing them.
Alpha 3 is the published upgrade baseline; R9 and viewer image copying landed after that release.

Start from current source and the owning evidence below. Historical pending CI and superseded
failures are not new blockers without a current failure or missing affected verification.

| Area | Existing owner | Question R10 must close |
| --- | --- | --- |
| Gallery, smart albums and review navigation | [Interaction evidence](../acceptance/material-motion.md#classification-navigation-and-foreground-reads), [ADR 0014](../architecture/0014-query-wide-gallery-layout-manifest.md) | Release input-to-usable-content latency, continuous held dragging and resource stability during background publication |
| Semantic discovery and workers | [R7 evidence](../acceptance/r7-semantic-discovery.md), [ADR 0041](../architecture/0041-resource-aware-analysis-scheduling.md) | Completed-index query costs and pressure/recovery behavior beyond the recorded partial-index runs |
| Synchronization and storage | [Current performance procedures](../acceptance/quality/performance-and-real-library.md), [ADR 0005](../architecture/0005-storage-governance-and-budget.md) | Target-scale catch-up, authoritative recovery, combined-root use and cache enforcement |
| Installation and upgrades | [ADR 0027](../architecture/0027-windows-installer-lifecycle.md), [Release evidence](../acceptance/unsigned-windows-release.md#alpha-3-publication--2026-11-12) | Actual Alpha-to-Beta application/data compatibility and recovery after an interrupted upgrade |
| Diagnosis and version display | `ame_bootstrap_failure.dart`, `ame_settings_page.dart` | A usable diagnostics export beyond bootstrap error copying; About and license pages must use the build version instead of their hard-coded Alpha 2 text |

## Execution slices

### R10-01 — Responsiveness and release baseline

Use the retained `local-primary` library and existing compatible analysis data. Compare the same
queries and workload before and after each optimization; record process-cold and warm results
separately. Release establishes startup and query latency; Profile establishes frame timing.

Cover startup, primary/secondary sidebar switches, albums and smart categories, exact/similar/ignored
views, organization entry/resume/exit, semantic search, viewer opening/closing and filmstrip paging.
Measure from input to usable content, retaining query/bridge/layout timings only to locate the cost.
Include classification and synchronization publishing while the user navigates.

Working targets for the project workstation are a usable retained gallery within five seconds of a
fresh process and warm navigation P95 within one second. Measure at least five fresh launches and
20 warm switches per affected path. Separate model cold initialization, first import and unavailable
sources; they must preserve usable controls and truthful feedback. Record a required target change
in the roadmap before treating a slower result as accepted.

Exercise both layouts with a continuous multi-second timeline hold, distant movement and reversal.
The viewport must follow before release, preserve dates and anchors, and never let an old request
restore an earlier position. Use ADR 0014's frame/resize criteria; fix measured owning costs without
adding navigation waits, whole-library invalidation or another scroll authority.

Exit evidence: the before/after interaction table, affected regressions and observed desktop paths.
Reuse the accepted viewer/selection behavior unless an affected change requires another check.

### R10-02 — Resources, interruption and sustained use

Run one bounded 60-minute mixed browsing/search/analysis session. Record the client and worker
memory separately, GPU allocation where available, retained image/detail counts, cache growth,
cancellation and shutdown. Repeated navigation must settle into a bounded resource range; extend
the run only when a concrete leak or unresolved trend needs more observation.

Complete or reuse a compatible target-library semantic index once, then measure warm queries and
restart reuse. Exercise the existing GPU/CPU serial fallback and memory-pressure thresholds without
starting a separate constrained-device optimization project. Verify preview eviction, cleanup and
model/feature storage separately from durable decisions and file-operation history.

Reuse current ADR 0024 reliability entrypoints for cold/warm scanning, closed-app catch-up,
authoritative recovery and combined `local-primary`/`cloud-primary` reads. Bind available roots through
the ignored local-context mapping and existing read-only safeguards. Record unavailable roots as
an evidence gap; do not substitute the retired direct-USN harness or hydrate cloud placeholders.
Use disposable Sandbox files for mutation, locked-file, missing-root and interruption scenarios.

Reuse targeted worker, catalog, migration and R8/R9 recovery cases. Verify restarting after a worker
failure or interrupted application retains completed work and exposes actionable unfinished work.
Keep the accepted Windows recycle race and recurrence-only pause observation at their existing scope.

Measure 79,000-, 250,000- and 1,000,000-item compact manifests with the existing benchmark. Implement
hierarchical storage or a layout isolate only if ADR 0014's measured replacement conditions trigger.

Exit evidence: bounded resource results, target-scale recovery timings and successful restart of the
affected workflows. Existing fixture budgets apply to their own processes, not to total model memory.

### R10-03 — Upgrade and compatibility

In disposable Windows Sandbox, install the published Alpha 3 MSI, populate representative catalog,
favorite, album, representative/retained-image, organization and file-operation records, then upgrade
to the candidate. Also check a compatible older catalog fixture, repair, uninstall and portable
directory replacement. Verify model/runtime discovery and the installed broker after upgrade.

Reuse migration backups and completion receipts. Interrupt migration and installer upgrade at their
existing controlled boundaries; verify the preceding compatible installation/data can be recovered.
An older executable must reject an unsupported newer schema clearly. Do not promise that arbitrary
binary downgrades can read a migrated database or silently discard newer user decisions.

Make About, licenses, diagnostics, executable metadata, package receipts and release notes obtain
their version from the same build identity. The Flutter build number must increase across Alpha to
Beta so the existing MSI version mapping remains monotonic.

The first Beta uses documented MSI upgrades and portable-package replacement. An in-app automatic
downloader/installer remains a separate product decision; it is not needed for this upgrade outcome.

Exit evidence: preserved representative records, working installed/portable clients, recovery results
and consistent version reporting. Reuse unchanged installer guardrails instead of recreating them.

### R10-04 — Diagnostics and recovery guidance

Add one diagnostics-export action to the existing Settings/About surface and retain diagnosis when
startup fails. Define a narrow application export contract and platform adapter before connecting UI.
The export should include build/OS identity, selected analysis runtime, storage/schema versions,
bounded task/error summaries and recent relevant logs, with native error-code namespaces intact.

Default exports omit images, catalog contents and credentials, and replace personal paths/account
names with logical labels. Bound log bytes and collection time. The user chooses a local destination;
the result uses existing feedback and folder-opening controls. Decide the exact format and preview
the affected surface during this slice, without adding a task center or redesigning Settings.

Write a short user guide covering first import, installed versus portable behavior, model readiness,
manual upgrade, backup/recovery, interrupted organization and how to provide a diagnostic bundle.
Reuse existing contracts for warnings and known limits; keep implementation details in technical docs.

Exit evidence: an exported bundle that explains a representative failure, no unintended personal
content, and a working destination/retry path. Documentation links must be reachable from the README.

### R10-05 — Beta candidate and publication

Select one candidate after the preceding slices close. Run applicable full regression, native Windows
integration, synthetic workloads and the existing release-candidate build once for that revision.
Investigate failures at their owner; do not reduce checks to obtain a release or restart all work
after an unrelated notes/publisher repair.

Prepare `docs/releases/v0.1.0-beta.1.md` with changes since Alpha 3, including R9 and viewer copying,
upgrade instructions and remaining accepted limits. Publish an unsigned GitHub prerelease with both
MSI and portable ZIP from one verified candidate. Retain the current model payload differences,
checksums, immutable tag/package checks and [artifact reuse](release-artifact-reuse.md) workflow.
Verify downloaded packages and the release body before recording publication.

## Working order and exit decision

Start with R10-01's baseline, then R10-02's bounded resources/recovery, R10-03's upgrade path and
R10-04's diagnostics. Finish R10-05 after each owning result is available. Commit independently
reversible repairs with their focused evidence; use an implementation branch when R10 is activated.
This planning change does not start those workloads or publish the Beta.

The roadmap may declare first-Beta readiness when normal workflows meet the agreed responsiveness
targets, no known blocking correctness/data-loss failure remains, resource/recovery and upgrade
evidence is complete, diagnostics are usable, and the exact candidate's applicable gates pass.
Record accepted nonblocking limitations in its release notes rather than relabeling them as fixed.

Code signing, new locales, official Expressive migration and the previously deferred constrained-device
work do not block the first Beta. Locale infrastructure remains conditional on stable copy and an
explicit locale decision. Million-item architecture changes remain measurement-triggered.
Do not expand ordinary PR CI with the long session or real-library workload; retain scoped development
checks and run the maturity/release workloads through their existing acceptance and release owners.
