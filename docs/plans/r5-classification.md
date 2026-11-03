# R5 - Metadata and smart albums implementation plan

Status: execution plan; stage activation, order, blockers, and closure belong to the roadmap.

The [roadmap sequence](../roadmap.md#r5-implementation-sequence) selects the work below.
[R5 requirements](../product/r5-classification.md) own behavior, and the
[gallery contract](../product/gallery/smart-albums.md#smart-album-workflow) owns the accepted desktop interface.
This plan does not claim implementation, model admission, or acceptance from the interaction sketch.

## Outcome and boundaries

Deliver useful metadata/search first, then usable local initial screening and derived `智能相册`.
Keep the shared gallery/viewer and R4's whole-group manual-album, Favorite, organization, and recovery
rules. Manual classification confirmation/correction and their review workflow were withdrawn on
2026-11-01; preserve existing data and archive compatibility while removing desktop entrypoints.

R4 is the reuse foundation. Its completed verification is retained, with affected regressions run
when an R5 slice changes their inputs. The separately deferred rendering follow-up remains under its
[roadmap scope](../roadmap.md#deferred-constrained-device-rendering), not an R5 prerequisite or an
invitation to repeat the R4 acceptance cycle.

Before product changes, create the R5 development branch under the repository's Git rules. Do not
recreate the retired R4 branch. Dependency, schema, bridge, worker-isolation, and recovery choices
require their owning ADRs before implementation. The selected screening candidate is SigLIP 2
Base/16-224 with FP16 image and INT8 text encoders; the
[exploratory evidence](../acceptance/r5-model-evaluation.md) informs R5-02 admission rather than
replacing product quality, license, packaging, or resource checks. Keep large models outside scope.
Use a stage PR and exact-head checks before requesting merge; merging requires explicit authorization.
The stage PR covers R5-01 through R5-07. Keep its title and stage scope stable while implementing the
sequence; describe substantial scope changes without turning each fix into a PR rewrite. Group related
implementation, repairs, and verification around complete user workflows.

## Foundation and responsibility map

| Existing boundary | Reuse and constraint |
| --- | --- |
| [Metadata adapter ADR](../architecture/0006-exif-capture-time-adapter.md) | Preserve capture evidence, orientation, compatibility, and source safety while extending admitted metadata. |
| [Exact-content ADR](../architecture/0028-versioned-exact-content-analysis.md) | Reuse compatible group evidence and source admission; a content hash is not permanent asset identity. |
| [Organization decisions](../architecture/0030-durable-organization-decisions.md) | Reuse guarded whole-group targeting and durable lineage/field revisions without storing classification in album state. |
| [Portable archives](../architecture/0031-portable-organization-archives.md) | Version new decision records and preserve old archive readers, validation, and atomic restoration. |
| [Gallery query port](../../rust/src/ports/gallery_query.rs) | Keep pages, counts, layout, search, selection, and viewer scopes coherent and bounded. |
| [Indexed text search](../architecture/0032-indexed-catalog-text-search.md) | Preserve literal path matching through the shared query while keeping FTS data rebuildable from catalog rows. |
| [Classification contracts and runtime](../architecture/0034-versioned-classification-contracts.md) | Reuse independent category/encoder revisions and the verified worker boundary; application admission and publication retain their own authority. |
| [Classification decision authority](../architecture/0035-classification-decision-authority.md) | Keep prediction evidence and legacy human conflict/restore precedence independent of R4 state; no desktop manual-classification commands. |
| [Durable classification dispatch](../architecture/0036-durable-classification-dispatch.md) | Persist compatible shared work and publication leases independently of worker lifetime and portable human decisions; recover legacy modes into automatic desktop scheduling. |
| [Workspace navigation](../architecture/0029-functional-workbench-navigation.md) | Retain primary navigation, ordinary reset rules, and the organizing Library's retained state. |
| [Invariant-owned modules](../architecture/0025-invariant-owned-workflow-modules.md) | Give inference, category projection, retry, and recovery cohesive owners before extending shared facades. |

Inspect the live owners at each slice. These are discovery points, not a prescription to append
classification branches to existing controllers, migrations, or workflow orchestrators.

## R5-01: Metadata and bounded structured search

Depends on the delivered R4 baseline.

1. Map existing metadata and search behavior; identify the supported fields and admission gaps.
   Preserve date provenance, orientation, source identity, unavailable files, and placeholder guards.
2. Add admitted extraction behind typed metadata contracts, with versioned reuse/invalidation and
   forward migrations where required. Keep unsupported or unknown values explicit.
3. Connect FTS5 text search and structured predicates through the existing gallery query, including
   filename/path, dates, type/size, source, manual album, and supported metadata. Category predicates
   remain unavailable until R5-05 supplies their authoritative projection.
4. Connect search to pages, counts, selection, layout, and viewer navigation without whole-library
   reads or a separate search page. Keep indexes rebuildable independently of user decisions.
   The metadata-condition editor was withdrawn on 2026-11-02. Preserve native predicates and archive
   compatibility, retain the original four-choice Filter menu, and use current conditions when
   restoring organization progress so retired conditions cannot silently narrow the desktop.

Exit: metadata/search works end to end without a model; query consistency, invalidation, malformed
metadata, migration, and unchanged originals have focused evidence.

## R5-02: Taxonomy, model, and runtime admission

Follows R5-01; its output fixes the model/taxonomy compatibility used by later slices.

1. Define the finite application-owned category catalog with stable identities, display labels,
   versioned semantic descriptions and purpose-aware boundary examples. Starting from the retained
   42 topics, consolidate overlapping concepts and declare the allowed purposes and names for each
   topic. Replace production dictionary traversal and unrestricted topic/purpose composition with
   direct bounded evaluation of these definitions. Keep broad categories for unsupported refinements
   and distinguish failed analysis from legitimate `其他` content.
   Preserve legacy definitions, saved choices and guarded category activation/retirement. Admit
   refinements from distinct-content evidence; expose only categories with current results through
   coherent bounded sidebar queries. No manual category or prompt authoring is added.
   Evaluate useful coverage, overlap and errors by allowed combination. Reuse the unchanged model,
   cached features and valid runtime evidence. New cultural meaning remains limited by visible
   evidence and the local model; no online research, automatic learning or extra naming model is
   included. The former requirement for names outside the authored catalog is withdrawn.
2. Create a source-safe evaluation protocol with representative labeled groups, calibration and
   held-out partitions, duplicate-leakage prevention, and per-category/band metrics.
   Freeze the purpose-aware rubric before comparing predictions. Record the label's authoring
   authority and ambiguity; unfinished folders and provisional visual labels do not become human
   truth. Keep exact duplicates and known visual/page-series clusters in one partition. Publish
   aggregate results using logical roots and opaque sample IDs, retaining private paths and media
   outside Git. Fit candidate separation/refinement policy only on calibration groups, then evaluate
   once on the sealed holdout with unchanged descriptions, runtime and text-batch policy. Report
   uncertain labels separately rather than scoring them as `其他` or silently discarding their
   coverage. A changed rubric, encoder or matching policy needs its own compatible calibration.
3. Admit the selected SigLIP 2 configuration using retained exploratory comparisons and focused
   missing evidence. Calibrate first-pass publication and one bounded cached-feature re-screen
   against errors, useful coverage, and automatic refinement cost. Keep CLIP
   B/32 INT8 as the lightweight reference; do not repeat an unchanged broad model comparison.
4. Compare cold/warm inference, memory, throughput, cancellation, and browsing interference on the
   two-core/8-GiB baseline and local-primary. Evaluate worker/thread bounds and optional acceleration.
5. Admit the selected dependency versions, worker boundary, artifact verification, storage budget,
   calibration rules, and fallback/replacement strategy in the owning ADRs. Include code/weight
   licenses, model setup and upgrade cost, retained full text-encoder capability, and hash verification.

Exit: the rubric, candidate evidence, licenses, runtime protocol, calibrated operating policy, and
measured resource limits justify one implementation candidate. Unsupported categories or unproven
confidence bands remain explicit gaps; do not fabricate a passing quality threshold.

## R5-03: Durable classification decisions and recovery

Depends on R5-02's taxonomy and compatibility contract.

1. Persist immutable prediction/run evidence with independent effective-category projection.
2. Preserve the delivered schema and portable recovery for legacy human choices, review bookmarks,
   and operation provenance. Existing precedence, conflict, stale-authority, and anti-resurrection
   guards remain compatibility requirements, independent of R4 state.
3. Remove desktop mutation and continuation endpoints for manual classification. Retain bounded
   read-only item/annotation queries and automatic failure recovery. No destructive migration or record
   reset is required for the removed workflow.
4. Preserve explicit source-removal cleanup, unresolved-record cleanup, and atomic versioned archive
   restoration, including old R3/R4 archives and recovery without an installed model.

Exit: existing decisions survive restart, reanalysis, rebuild, and export/restore; late grouping and
changed content have deterministic treatment; removed desktop commands cannot create new manual
classification activity. Retain valid compatibility evidence rather than replaying unrelated recovery.

## R5-04: Incremental local analysis

Depends on admitted runtime/quality policy and R5-03 persistence.

1. Give task admission, encoder residency, immutable evidence publication, and feature invalidation
   cohesive typed owners under ADR 0025. Connect explicit model setup and verified local artifacts
   to the admitted supervised-worker protocol, outside source trees and durable user data.
2. Consume committed per-content R3 verification without waiting for global deduplication or scanning
   source directories again. Share compatible pending/running work and results across confirmed
   copies. Preserve exact source/configuration generations through late duplicates and changed bytes.
3. Add durable bounded queues with explicit-scope, new/changed-content, and backlog priorities plus
   fairness. Integrate foreground-read/task signals and shared resource admission with import and
   deduplication. Keep decode/inference outside database transactions and retire an old batch before
   admitting its replacement. Cached browsing must not continually restart inference.
4. Persist image and text features with independent compatibility keys. Description/category changes
   rematch cached image features; display-only changes preserve them. Bound one automatic semantic
   re-screen and make its prompts/configuration part of the immutable run evidence.
5. Implement demand loading, separately releasable text/image encoders, and bounded warm residency.
   Measure the proposed three-second initial quiet interval and 60-second idle retention with small
   batches; pressure, shutdown, and suspend release resources under the admitted deadline.
6. Resume automatic yielding only after readiness and source revalidation, and reuse committed work
   after worker failure. The desktop manual controls were withdrawn on 2026-11-02. Installed legacy
   paused setup resumes its run; disabled/stopped setup activates a fresh run under existing guards.
   Missing models retain explicit download setup. Bound retries per failure/content version and keep
   stopped identities retired; startup does not reset compatible failure budgets.
7. Connect existing progress and feedback owners. Keep directory synchronization independent,
   publish compatible batches atomically, and preserve prior valid results and visible anchors.
   Cache cleanup must not erase decisions or immediately trigger a complete feature rebuild.

Exit: real local inference publishes compatible results incrementally, resumes without repeating
completed work, respects measured resource limits, and handles corrupt/offline/placeholder inputs
without ending the desktop session or modifying sources. Focused tests cover concurrent duplicate
arrival, stale publication, starvation, foreground yielding, legacy-mode startup, virtual-clock
residency, worker termination, suspend/resume, and rule-only rematching without image inference.

Retain completed runtime and local foreground measurements when advancing to the query and desktop
slices. The paired cloud-primary browsing/classification comparison runs with the connected workflow
in R5-07 and remains required before merge; it does not require an intermediate client run before
R5-05. Revalidate retained package, model and fixture compatibility before that measurement.

## R5-05: Smart-album projections and coherent queries

Depends on R5-03 and R5-04.

1. Derive `EffectiveCategory` from compatible predictions and applicable legacy decisions. Preserve
   conflict provenance without a guessed winner; uncertain model results retain their best suggestion.
2. Keep category scope, search, sort, pages, counts, layout, selection, and viewer navigation on the
   same bounded gallery snapshot. The desktop exposes no pending/confirmed classification filters.
3. Keep unresolved/failed items reachable through the ordinary Library without inventing a category.
4. Connect committed result invalidation, grouping changes, and source removal to bounded updates.
   Background arrivals retain the current image and logical position.

Exit: category membership, counts, and windows agree; no query/selection loads every asset in Flutter.
Legacy projection/restore behavior remains compatible without exposing the retired review workflow.

## R5-06: Desktop smart albums

Depends on R5-05 and the persistence/recovery boundary from R5-03. The slice identifier is retained;
its former manual-review scope was withdrawn on 2026-11-01.

1. Keep `智能相册` below manual albums in the existing Albums sidebar, with the accepted static
   heading, collapse arrow, category rows, empty row, neutral icons, and shared gallery motion.
2. Remove confirmation/correction, deferral, review-filter/count/badge, continuation, and classification
   undo controls from the gallery, selection bar, context menus, and preview.
3. Retain read-only category display, the shared gallery filter, and ordinary Library navigation. Recover
   per-image failures silently by failure type and changed input; remove failure indicators and
   retry commands. Runtime errors use the existing bottom and viewer-top feedback templates.
4. Preserve Favorite and manual-album actions, ordinary navigation/selection rules, and R4's retained
   organization context. Smart albums remain absent from manual-album destinations and management.
5. Verify first use, loading, empty results, legacy decisions, failed/unavailable images, stale reads,
   automatic recovery, cancellation, and restart through the real application services.
6. Retain managed model setup, verified installation, bundled runtime, and automatic activation of
   supported catalog categories. Setup needs no user-authored taxonomy or model/runtime directory selection. Remove
   analysis settings and manual run controls from gallery menus and the Settings page. Missing models
   offer explicit download in the smart-album empty state; installed analysis runs automatically
   without a status-only setup prompt.
7. Complete the 2026-11-02 model-delivery follow-up: compact single-line empty row and download icon;
   one live task in the existing notification center with hide/cancel/retry; bundled verified models
   in the Windows installer; durable authorized downloads with safe range continuation after restart.
   Retain the current model identity, notices, guarded publication and bounded worker lifecycle.

Exit: browsing, automatic recovery, and manual-album/Favorite workflows remain connected, with no manual
classification flow. The sketch retains layout intent; its superseded review controls are removed.

## R5-07: Integrated quality, scale, and closure

Depends on all earlier slices and reuses their valid evidence.

1. Verify metadata-to-analysis-to-smart-album-to-manual-album flow, then late grouping, content changes,
   source removal, cleanup, archive restoration, reanalysis, cancellation, and restart.
2. Measure held-out category/band quality, usable coverage, high-confidence errors, automatic
   refinement, and failures. Keep calibration separate from final evaluation and include overlapping
   purpose/content cases. Manual classification/correction-time trials are removed from acceptance.
   Preserve legacy choices through reanalysis and recovery without offering new manual decisions.
3. Measure large-library bounded queries and two-core/8-GiB analysis while browsing and viewing.
   Record cold/warm conditions, source reads, memory/model storage, and worker cancellation/recovery.
   Complete the paired cloud-primary foreground comparison here, reusing applicable local evidence
   and retained test assets instead of repeating unchanged runtime-admission checks.
4. Follow the repository's hosted exact-commit gates and scoped local desktop/source-safe verification.
   Retain completed R4 results and run only affected regression boundaries when their inputs change.
5. Record actual evidence and remaining gaps in acceptance records, then update the roadmap's exit
   decision. R6 similarity work and the deferred R4 rendering follow-up retain their separate scopes.

Exit: the R5 product contract is connected and verified, model/runtime admission remains valid,
the initial-screening quality and resource limits are documented, and original media and durable
R4/legacy classification decisions remain intact.

## Execution and unresolved implementation choices

Before each slice, recover current decisions, inspect Git/live owners, and read the owning ADRs.
Commit implementation, focused regressions, and necessary documentation as one reversible outcome;
follow the [required workflow](../acceptance/quality/verification-workflow.md#required-engineering-workflow).
Do not start heavy local CI or new real-library copies merely because a planning document changed.

The selected model configuration and lifecycle requirements constrain the named slices. Exact runtime
admission, calibrated thresholds, measured worker budgets, schema/bridge design, and the new archive
version are outputs of those slices. Resolve material product or licensing changes before dependent
work. Routine framework composition follows the approved UI contract without introducing another
navigation or review shell.
