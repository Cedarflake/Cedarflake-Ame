# Cedarflake Ame Roadmap

Status: canonical active delivery plan

Last planning update: 2026-11-28

Ame is a local-first Windows workbench for browsing, finding and organizing large image libraries.
The next milestone is `v0.1.0-beta.1`. Product boundaries belong to the
[workbench contract](product/workbench.md) and [organization workflows](product/organization-workflows.md).

## Delivery sequence

<a id="current-stage-and-acceptance-boundary"></a>

| Stage | Outcome | Status |
| --- | --- | --- |
| R0–R2 | Catalog, browsing and change-driven library updates | Complete; [foundation](acceptance/gallery-foundation.md), [R2c closeout](acceptance/r2c-update-checkpoint.md#r2c-freeze-on-2026-10-16) |
| R3 | Exact duplicates and file locations | Complete; [evidence](acceptance/r3-exact-duplicates.md) |
| <a id="r4-implementation-sequence"></a>R4 | Favorites, albums and organization progress | Complete; [evidence](acceptance/r4-organization.md#r4-merge-verification---2026-10-30) |
| <a id="r5-implementation-sequence"></a>R5 | Metadata, smart albums and initial screening | Complete; [evidence](acceptance/r5-closeout.md) |
| R6 | Similar-image review | Complete; [evidence](acceptance/r6-similar-images.md#r6-closeout) |
| <a id="r7-implementation-sequence"></a>R7 | Structured and semantic search | Complete; [evidence](acceptance/r7-semantic-discovery.md#integrated-verification--2026-11-08) |
| <a id="r8-implementation-sequence"></a>R8 | File operations and recovery | Complete; [evidence](acceptance/r8-file-operations.md#r8-integration--2026-11-11) |
| <a id="r9--album-files-by-year"></a>R9 | Move album files into year-based folders | Complete; [evidence](acceptance/r8-file-operations.md#r9-integration--2026-11-12) |
| R10 | Large-library maturity and first Beta | In progress; R10-01–04 complete, R10-05 next |

## Current execution queue

### R10 — Beta readiness

Use the [R10 execution plan](plans/r10-beta-readiness.md) for the remaining candidate work.

| Slice | Outcome | Status |
| --- | --- | --- |
| R10-01 / R10a | Responsive startup and browsing | Complete; [evidence](acceptance/r10-responsiveness.md) |
| R10-02 / R10b | Bounded resources and interruption recovery | Complete; [evidence](acceptance/r10-resource-recovery.md) |
| R10-03 / R10c | Installation, upgrade and version identity | Complete; [evidence](acceptance/r10-upgrade-compatibility.md) |
| <a id="diagnostics-and-recovery-guidance"></a>R10-04 | Local diagnostics and failure recovery | Complete; [evidence](acceptance/r10-diagnostics.md) |
| R10-05 | Qualify and publish `v0.1.0-beta.1` | Pending |

Next steps:

1. Select the Beta candidate and resolve any blocking defect affecting normal use.
2. Complete the applicable candidate gates and prepare the MSI, portable ZIP and release notes
   from the same verified revision.
3. Prepare the Store MSIX upload package with the Beta, then publish through the separately
   authorized release/submission workflows.

Beta exit requires the agreed responsiveness, resource, recovery and upgrade results, usable
diagnostics, no known blocking correctness or data-loss defect, and passing candidate gates.
Startup and cached reads retain their [five-second / one-second targets](acceptance/r2c-m-replacement-reliability.md#required-evidence).
Code signing, additional locales, official Expressive migration and constrained-device optimization
are not prerequisites. Million-item architecture changes remain measurement-triggered.
Reuse unchanged acceptance evidence; candidate qualification does not reopen completed stages.

### Microsoft Store distribution

MSIX packaging and Store-managed updates are implemented; see the
[distribution plan](plans/store-msix.md) and [acceptance](acceptance/store-msix.md).
The Store upload package is due with the Beta release.

The remaining [Store-signed Release asset workflow](plans/store-msix.md#store-signed-github-release-asset)
needs first Store publication, Entra submission credentials and an eligible Windows user session
for package download. Complete its end-to-end verification when those prerequisites are available.
Store submission and attaching the signed package remain separately authorized release actions.

## Delivered supporting work

These capabilities are integrated and are not pending work. Their linked records own implementation
details, verification and limitations.

| Capability | Record |
| --- | --- |
| <a id="msi-localization"></a>MSI installation and maintenance in English, Simplified Chinese and Japanese | [Installer acceptance](acceptance/unsigned-windows-release.md#msi-localization--2026-11-28) |
| Smart-album term editing | [Term editing](acceptance/smart-album-terms.md) |
| <a id="application-updates"></a>Application updates | [Update lifecycle](acceptance/application-updates.md) |
| <a id="desktop-internationalization"></a>Desktop English, Simplified Chinese and Japanese | [Desktop internationalization](acceptance/desktop-internationalization.md) |
| <a id="prerelease-compatibility-cleanup"></a>Retirement of obsolete Alpha migration paths | [Supported baseline](acceptance/prerelease-compatibility.md) |
| <a id="material-desktop-workbench"></a>Material wide-screen workbench | [Workbench](acceptance/material-ui-migration.md) |
| <a id="material-motion-adoption"></a><a id="transition-performance"></a><a id="loading-feedback"></a>Motion, transitions and loading feedback | [Motion](acceptance/material-motion.md) |
| <a id="gallery-group-discovery"></a>Grouped-gallery filters and member actions | [Group discovery](acceptance/r8-file-operations.md#gallery-group-discovery-integration--2026-11-13) |
| <a id="project-organization-after-r5"></a>Feature-module organization O1–O7 | [Handover](acceptance/project-organization-continuity.md#organization-handover--2026-11-04); [dependency contract](architecture/0037-feature-module-dependency-boundaries.md) |

## Deferred work

| Item | Resume condition |
| --- | --- |
| Viewer crash when expanding the thumbnail strip | Deferred at the user's request; investigate before treating it as fixed or deciding whether it blocks the selected Beta candidate |
| Nativeapi window and screen plugin migration | Resume only when requested, using the retained [window-lifecycle findings](acceptance/material-ui-migration.md#flutter-350-compatibility-2026-11-17) |
| R6 cloud-primary and constrained-client qualification | Retains its separate [R6 closeout scope](acceptance/r6-similar-images.md#r6-closeout) |
| Historical pause timeout | Reopen diagnosis only if it recurs; [retained observation](acceptance/project-organization-continuity.md#pause-timing-diagnosis--2026-11-04) |
| Native Debug termination during R2c recovery | Reopen only upon recurrence; [freeze record](acceptance/r2c-update-checkpoint.md#r2c-freeze-on-2026-10-16) |

### Deferred constrained-device rendering

First-viewer and mode-entry graphics peaks on the constrained cloud-primary client remain deferred.
Resume from the [existing traces](acceptance/r4-organization.md#constrained-device-first-use-rendering---2026-10-30),
preserving accepted controls, motion and full-resolution viewing. Previously rejected warm-up,
reduced-motion and cache approaches need new evidence before another attempt.

## Maintaining this roadmap

Keep milestones, current priorities, real blockers and exit conditions here. Replace obsolete status;
do not append commit histories, CI results, debugging sessions or per-change completion reports.
Detailed steps belong in plans, stable decisions in product/architecture documents and verification
in the [acceptance index](acceptance/README.md). The [documentation standard](development/documentation.md)
owns these boundaries.
