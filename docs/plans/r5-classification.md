# R5 - Metadata, smart albums, and review implementation plan

Status: execution plan; stage activation, order, blockers, and closure belong to the roadmap.

The [roadmap sequence](../roadmap.md#r5-implementation-sequence) selects the work below.
[R5 requirements](../product/r5-classification.md) own behavior, and the
[gallery contract](../product/gallery-ui.md#smart-album-workflow) owns the accepted desktop interface.
This plan does not claim implementation, model admission, or acceptance from the interaction sketch.

## Outcome and boundaries

Deliver useful metadata/search first, then usable local initial screening, derived `智能相册`, and
optional individual/batch review that reduces total human work. Keep the shared gallery/viewer and
R4's whole-group single-manual-album, Favorite, organization, and recovery rules. Category confirmation and correction
never change manual membership or organization progress.

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

## Foundation and responsibility map

| Existing boundary | Reuse and constraint |
| --- | --- |
| [Metadata adapter ADR](../architecture/0006-exif-capture-time-adapter.md) | Preserve capture evidence, orientation, compatibility, and source safety while extending admitted metadata. |
| [Exact-content ADR](../architecture/0028-versioned-exact-content-analysis.md) | Reuse compatible group evidence and source admission; a content hash is not permanent asset identity. |
| [Organization decisions](../architecture/0030-durable-organization-decisions.md) | Reuse guarded whole-group targeting and durable lineage/field revisions without storing classification in album state. |
| [Portable archives](../architecture/0031-portable-organization-archives.md) | Version new decision records and preserve old archive readers, validation, and atomic restoration. |
| [Gallery query port](../../rust/src/ports/gallery_query.rs) | Keep pages, counts, layout, search, selection, and viewer scopes coherent and bounded. |
| [Indexed text search](../architecture/0032-indexed-catalog-text-search.md) | Preserve literal path matching through the shared query while keeping FTS data rebuildable from catalog rows. |
| [Organization review domain](../../rust/src/domain/organization_review.rs) | Reuse continuation principles; existing organization sessions do not already implement classification review. |
| [Workspace navigation](../architecture/0029-functional-workbench-navigation.md) | Retain primary navigation, ordinary reset rules, and the organizing Library's retained state. |
| [Invariant-owned modules](../architecture/0025-invariant-owned-workflow-modules.md) | Give inference, category projection, review authority, and recovery cohesive owners before extending shared facades. |

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

Exit: metadata/search works end to end without a model; query consistency, invalidation, malformed
metadata, migration, and unchanged originals have focused evidence.

## R5-02: Taxonomy, model, and runtime admission

Follows R5-01; its output fixes the model/taxonomy compatibility used by later slices.

1. Define configurable flat category identities, display labels, versioned semantic descriptions,
   retirement/remapping rules, and purpose-aware ambiguous examples. Keep `待确认` outside the
   taxonomy and distinguish failed analysis from a legitimate `其他` category. Sample labels are
   evaluation inputs, not a fixed product class list.
2. Create a source-safe evaluation protocol with representative labeled groups, calibration and
   held-out partitions, duplicate-leakage prevention, and per-category/band metrics.
3. Admit the selected SigLIP 2 configuration using retained exploratory comparisons and focused
   missing evidence. Calibrate first-pass publication and one bounded cached-feature re-screen
   against errors, optional checks/corrections per thousand groups, and human time saved. Keep CLIP
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

1. Define Ame-owned prediction/run, human decision, effective category, review, conflict, and receipt
   contracts. Establish their independent authority before schema or public bridge changes.
2. Implement guarded whole-group confirmation/correction and conflict reconciliation. Bind confirmation
   to the displayed compatible prediction; reject changed content or superseded command authority.
   Preserve explicit human conflicts, while machine-only differences use one current suggestion and
   automatic re-screening without automatically creating a required human decision.
3. Store review progress and logical continuation separately from R4's organization session/bookmark.
   Support field-scoped undo and idempotent retries without changing album or Favorite state.
4. Extend migrations, explicit source-removal cleanup, unresolved-record cleanup, and versioned
   portable recovery together. Include association evidence and retain R4/R3 compatibility.

Exit: decisions survive restart, reanalysis, rebuild, and export/restore; late grouping and changed
content have deterministic treatment; stale writes/undo and cleanup cannot resurrect invalid intent.
These commands remain internal until their complete desktop workflow is connected.

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
   batches; pressure, disable, shutdown, and suspend release resources under the admitted deadline.
6. Preserve explicit pause across restarts, resume automatic yielding only after readiness and source
   revalidation, and reuse committed work after worker failure. Bound retries per failure/content
   version; stop/cancel fences late publications and never silently recreates the same stopped run.
7. Connect existing progress and feedback owners. Keep directory synchronization independent,
   publish compatible batches atomically, and preserve prior valid results and visible anchors.
   Cache cleanup must not erase decisions or immediately trigger a complete feature rebuild.

Exit: real local inference publishes compatible results incrementally, resumes without repeating
completed work, respects measured resource limits, and handles corrupt/offline/placeholder inputs
without ending the desktop session or modifying sources. Focused tests cover concurrent duplicate
arrival, stale publication, starvation, foreground yielding, manual pause after restart, virtual-clock
residency, worker termination, suspend/resume, and rule-only rematching without image inference.

## R5-05: Smart-album projections and coherent queries

Depends on R5-03 and R5-04.

1. Derive `EffectiveCategory` from compatible prediction and applicable human decisions. Preserve
   human-conflict provenance; unresolved human conflicts have no guessed winner. Machine uncertainty
   retains the best usable suggestion with optional review, without multiple primary memberships.
2. Add smart-album/category and review-status scope to the typed gallery query. Keep search, sort,
   pages, counts, layout, selection, continuation, and viewer navigation on the same snapshot.
3. Implement all-results and category views with `全部结果`, `待确认`, and `已确认` review filters.
   Keep unresolved/failed items reachable from the all-results scope without inventing a category.
   The optional review filter never blocks browsing the current suggestions or creates modal review.
4. Connect committed result invalidation, group merging/splitting, and source removal to bounded
   updates. Background arrivals retain the user's current image and logical position.

Exit: model and human updates move the correct whole groups between derived views; counts and query
windows agree, and no category read or selection requires every asset in Flutter memory.

## R5-06: Desktop classification review

Depends on R5-05 and the guarded writes/recovery from R5-03.

1. Add the `智能相册` section below manual albums in the existing Albums secondary sidebar. Use the
   same gallery/viewer, selected-state styling, neutral menu icons, and sidebar motion.
2. Put review status in the existing filter menu. Connect contextual batch confirmation/correction,
   the preview's top category entry, and applicable image context-menu commands.
3. Retain independent Favorite and manual-album actions. Smart albums never enter the destination
   tray, R4 organization mode, or manual-album create/delete/drag-selection controls.
4. Connect keyboard review, deferred classification, explicit continuation, stable publication,
   and operation-specific undo through the shared bottom/top feedback templates. Preserve R4's
   organization-only chronological undo/redo and its saved Library context.
5. Verify first-use/setup, empty, unclassified, uncertain, conflicting, failed, unavailable, stale,
   saving, cancellation, and restart states with the real application services.
6. Connect flat category-name/description configuration through the existing contextual/settings
   patterns. Keep semantic-rule editing distinct from manual-album membership and preserve explicit
   decisions when definitions are renamed or retired; no extra primary workspace or tagging system.

Exit: the accepted desktop interaction completes real review and manual-album workflows, preserves
selection/position rules, and displays only committed results. The sketch supplies interaction
intent; its sample media, counts, icons, and decorative details are not new product requirements.

## R5-07: Integrated quality, scale, and closure

Depends on all earlier slices and reuses their valid evidence.

1. Verify metadata-to-analysis-to-review-to-manual-album flow, then late grouping, content changes,
   source removal, cleanup, archive restoration, reanalysis, cancellation, and restart.
2. Measure held-out category/band quality, sampled high-confidence errors, optional checks and
   corrections per thousand groups, actual correction time, and override survival. Compare total
   human work with manual organization; a larger review queue alone cannot establish better delivery.
   Keep calibration data separate from final evaluation and include purpose/content overlap cases.
3. Measure large-library bounded queries and two-core/8-GiB analysis while browsing and viewing.
   Record cold/warm conditions, source reads, memory/model storage, and worker cancellation/recovery.
4. Follow the repository's hosted exact-commit gates and scoped local desktop/source-safe verification.
   Retain completed R4 results and run only affected regression boundaries when their inputs change.
5. Record actual evidence and remaining gaps in acceptance records, then update the roadmap's exit
   decision. R6 similarity work and the deferred R4 rendering follow-up retain their separate scopes.

Exit: the R5 product contract is connected and verified, model/runtime admission remains valid,
human work is measurably reduced, and original media and durable R4/R5 decisions remain intact.

## Execution and unresolved implementation choices

Before each slice, recover current decisions, inspect Git/live owners, and read the owning ADRs.
Commit implementation, focused regressions, and necessary documentation as one reversible outcome;
follow the [required workflow](../acceptance/quality-gates.md#required-engineering-workflow).
Do not start heavy local CI or new real-library copies merely because a planning document changed.

The selected model configuration and lifecycle requirements constrain the named slices. Exact runtime
admission, calibrated thresholds, measured worker budgets, schema/bridge design, and the new archive
version are outputs of those slices. Resolve material product or licensing changes before dependent
work. Routine framework composition follows the approved UI contract without introducing another
navigation or review shell.
