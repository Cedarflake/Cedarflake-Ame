# R5 - Metadata, smart albums, and classification review

Status: product requirements; implementation availability and delivery status belong to the roadmap.

This document owns R5 behavior. The [roadmap](../roadmap.md#r5-implementation-sequence) owns order and
stage decisions; the [execution plan](../plans/r5-classification.md) defines implementation slices.
The [smart-album UI contract](gallery-ui.md#smart-album-workflow) owns placement and interaction.
R4's [organization rules](r4-organization.md#boundary-with-r5-classification) remain the baseline.

## User outcome and scope

Ame extracts useful metadata, proposes a primary category for each logical image group, and gathers
current category results into `智能相册`. The user can browse these results immediately, confirm or
correct them in batches, and resume unfinished classification review after restarting. Classification
reduces manual work while preserving the user's existing albums, Favorites, and organization progress.

R5 includes bounded metadata and structured search, local model evaluation and inference, calibrated
confidence, durable category decisions, derived smart albums, and human review. It preserves the
existing gallery, viewer, source-safety rules, and whole-group command targets. Perceptual similarity,
semantic search, face recognition, identity naming, embedded metadata writes, and physical file
operations remain outside R5. The [stage overview](organization-workflows.md) retains their owners.

## Vocabulary and independent decisions

Use `智能相册` for the visible entry and sidebar section. Its category results are derived from
`EffectiveCategory`; they are not additional user-owned album memberships. `确认分类` accepts the
current proposal, and `更改分类` saves an explicit category choice.

| Concept | Authority and effect |
| --- | --- |
| Model prediction | Immutable output with model/run identity, taxonomy version, parameters, confidence, and evidence. It is never a human confirmation. |
| Category decision | A durable human confirmation or correction for compatible group content; a later model run cannot overwrite it. |
| Effective category | The applicable human decision, otherwise a compatible current model result. Unresolved human conflicts have no silently selected winner. |
| Smart album | A bounded view of groups with that effective category. It has no editable membership list. |
| Classification review | Whether classification needs attention, was deferred, or was explicitly confirmed; independent of R4 organization state. |
| Manual album | R4's single confirmed user-album assignment for the complete group. |
| Favorite | R4's independent preference, compatible with either kind of album view. |

A group can appear in the `动漫／插画` smart album, the user's `壁纸` album, and Favorites together.
It still has only one confirmed manual-album assignment. Smart albums are not destinations in the
album tray, and cannot be created, renamed, deleted, or directly added to or removed from. A category
correction changes the derived result; it does not move a physical file or edit a manual album.

## Primary taxonomy and review status

The initial primary categories are:

| Category | Visible name |
| --- | --- |
| Photo | `照片` |
| Anime or illustration | `动漫／插画` |
| Screenshot | `截图` |
| Meme or reaction image | `表情包` |
| Document image | `文档` |
| Design asset | `设计素材` |
| Other | `其他` |

Each classified group has at most one effective primary category. Before model admission, document
the labeling rubric and representative boundary cases, including anime screenshots, captioned
reaction images, scanned documents, and design exports. Evaluate the same rubric for every candidate.
Version taxonomy changes rather than silently reinterpreting existing decisions. This requirement
does not introduce user-defined category trees or general multi-label tagging.

`待确认` is a review filter, not a category. `其他` is a legitimate content category, not a substitute
for missing predictions, failed analysis, or a category conflict. A result can have a proposed
category while still requiring confirmation.

Confidence and confirmation are separate. A calibrated high-confidence result may be used for
browsing without mandatory individual review; it remains machine-proposed until explicitly confirmed
and remains eligible for sampling. Medium/low-confidence results, selected samples, category conflicts,
and analysis failures require attention. Failed analysis retains a failure reason and retry behavior;
it does not invent a category. Deferred classification remains unfinished review.

`全部结果` includes the current smart-album scope regardless of review status. `待确认` selects work
requiring attention, including deferred review. `已确认` selects explicit human confirmations and
corrections. The section's all-results view also exposes unresolved or failed items that cannot yet
belong to one category, so these items do not disappear between category views.
High-confidence unreviewed results can be in neither review subset; confirmed counts must not be
calculated by subtracting the pending count from the total.

## Group authority and content lifecycle

Classification commands resolve to the complete current exact-duplicate group across source folders,
including when selection contains one member or a folder-filtered subset. Duplicate selections are
applied once. Before exact identity is known, each admitted content version is a provisional group.
Compatible byte-identical content reuses model analysis without merging stable asset identities.

Preserve these transitions:

- A new identical copy reuses compatible evidence and the group's applicable category decision.
- Compatible human decisions remain one decision after late grouping. Different human category
  decisions retain their provenance and require an explicit whole-group category choice. They do
  not create several confirmed primary categories or change R4's album-conflict queue.
- A model disagreement cannot override a valid human decision. Retain the newer model evidence
  separately for traceability rather than reopening completed review without a content conflict.
- Changed content invalidates incompatible prediction and decision bindings. Unchanged copies keep
  their decisions; genuinely different content receives fresh analysis or a compatible destination
  group's existing decision. A path or representative change alone is not a new category decision.
- Temporarily unavailable sources retain compatible records. Explicit root removal and manual
  unresolved-record cleanup follow R4's surviving-group and deliberate-removal rules; reimport alone
  cannot resurrect removed decisions. Deleting a manual album does not delete category decisions.

Revalidate content, group generation, model compatibility, and affected decision revisions before
publishing results or committing a command. Retained display evidence must not authorize a stale
write. Old model tasks, pages, and viewer callbacks cannot publish into a newer source or query scope.

## Human review and organization progress

`确认分类` stores the category actually shown to the user as a durable human choice. It binds the
accepted evidence and current content, so a delayed response cannot accept a different prediction.
`更改分类` stores the explicit replacement category and completes classification review for those
groups. Mixed-category confirmation accepts each group's own displayed prediction; a correction
applies the one explicitly chosen category to the selected groups. Groups without an admissible
prediction require correction or retry rather than a fabricated confirmation.

| Action | Classification effect | R4 organization effect |
| --- | --- | --- |
| Publish a prediction | Update compatible evidence and derived results | None |
| Confirm or correct a category | Save human category intent and review completion | None |
| Defer classification review | Retain unfinished classification work | None |
| Add or move to a manual album | None | Apply R4 membership and organized-state rules |
| Remove from the last manual album | None | Become `已忽略` under R4 |
| Favorite or unfavorite | None | Change Favorite only |

Classification writes never infer an album destination or mark R4 work organized. Adding an item to
a manual album from a smart-album view uses the existing album tray and ordinary-mode behavior; it
does not confirm the category. A group ignored for R4 organization can still have a category result.

Support keyboard-first review, bounded query-wide selection, batch confirmation/correction, and
operation-specific undo. Freeze the selected query and current group authority; validate again when
applying a batch. Publish only committed receipts, preserve completed work after cancellation, and
make uncertain retries idempotent. Report partial completion accurately instead of claiming that an
entire batch failed or succeeded. Undo changes only the affected category/review fields and respects
newer decisions, source removal, cleanup, and backup restoration.

Classification review has its own durable scope, logical anchor, progress, and operation authority.
Record remaining review work from committed query counts; estimate time only when measured throughput
supports it, without treating unknown or failed analysis as completed work.
Reuse the bounded continuation mechanisms from R4 without sharing its organization-mode session or
overwriting its bookmark. Ordinary workspace switches retain R4's selection/position rules; explicit
classification continuation restores its saved work. New results and grouping changes preserve the
currently viewed image and resolve an obsolete continuation anchor to the next eligible group.
Confirmation remains in the current category view; only a review filter that no longer matches
removes it from that filtered result. A committed category correction can change its category view.

## Metadata and structured search

Extract trustworthy capture evidence, dimensions, orientation, and supported camera/basic metadata
through admitted adapters. Preserve provenance and unknown values; filesystem dates are not invented
capture dates. Keep original bytes and embedded metadata unchanged. Reuse compatible extracted data,
with explicit invalidation when content or extraction parameters change.

Provide SQLite FTS5 text search and bounded structured predicates over filename, path, date, type,
size, source, manual album, category, and supported metadata. Search, pages, counts, layout, selection,
and viewer navigation must describe the same query. Make category predicates available when their
projection is connected; metadata/search delivery does not require a classification model. Indexes
are rebuildable independently of durable decisions and do not require full-library UI materialization.

## Model admission and low-resource execution

Evaluate candidate models and runtimes before selecting dependencies. Record exact versions, code
and weight licenses, redistribution terms, download/storage sizes, artifact integrity, Windows CPU
support, failure isolation, maintenance, and replacement contracts. A runtime mentioned in planning
is not an admitted dependency. Model setup and downloads require an explicit user action.

Use representative labeled groups from the target library under existing source-safe execution
rules. Separate calibration and held-out evaluation by content group to prevent duplicate leakage.
Record per-category quality, confidence-band coverage and errors, sampled high-confidence quality,
abstentions/failures, and expected human-review work. Thresholds follow measured calibration; a raw
model score is not automatically a probability or a universally valid cutoff.

Use the two-core, 8-GiB cloud-primary client as a required resource baseline, with local-primary as
a comparison. Measure CPU, memory, model storage, cold/warm latency, sustained throughput, cancellation,
and gallery/viewer interference. CPU operation must remain usable; acceleration is an evaluated
optimization rather than a prerequisite. Record measured budgets before enabling background defaults.

Analysis runs off the UI thread with bounded workers, input windows, queues, caches, and runtime
threads. Foreground previews and interaction have priority. Process newly indexed or invalidated
content incrementally, reuse compatible exact-group work, and resume committed analysis after restart.
Pause/stop must reach a bounded checkpoint; worker failure must preserve the desktop session and
completed results. Never hydrate cloud-only files or hold catalog transactions over decoding/inference.

## Persistence and recovery

Persist model evidence, human decisions, effective-category projection, and review state separately
under Ame-owned contracts. Immutable analysis runs include model/taxonomy versions and parameter
identity. Reanalysis retains older evidence and applicable human intent; model and index cleanup
cannot delete human confirmation, correction, continuation, or operation history.

Add forward migrations and recoverable upgrade snapshots. Version the portable recovery contract to
include category decisions, their association evidence, review bookmarks, and applicable undo
provenance. Retain R4/R3 archive compatibility and explicit-restore authority. The existing R4 archive
format does not already carry these new records. Models, original media, and rebuildable projections
remain outside decision archives. Unavailable model binaries cannot invalidate a restored human choice.
Extend the existing unresolved-record inspection/cleanup counts to name affected classification
records, retaining its one entry, confirmation, eligibility recheck, and explicit-backup recovery.

## Acceptance requirements

R5 must demonstrate:

- preserved R4 album/Favorite/progress behavior across confirmation, correction, and model updates;
- coherent group-based category pages, counts, search, selection, time/name rails, and viewer scope;
- calibrated results with per-category/band evidence and a measured reduction in human-review work;
- batch review, explicit continuation, undo, cancellation, retry, stale-result rejection, and restart;
- late duplicate conflicts, content changes, root removal, cleanup, migration, and archive recovery;
- real desktop paths following the accepted UI contract, with shared feedback and stable layout;
- bounded two-core/8-GiB operation while ordinary browsing and preview remain usable;
- unchanged original media and placeholder state, with no automatic file operation or hydration.

Record actual evidence in `docs/acceptance` when implementation exists. A layout sketch, requirements
review, or documentation check does not establish functional or model-quality acceptance.
