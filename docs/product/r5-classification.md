# R5 - Metadata, smart albums, and classification review

Status: product requirements; implementation availability and delivery status belong to the roadmap.

This document owns R5 behavior. The [roadmap](../roadmap.md#r5-implementation-sequence) owns order and
stage decisions; the [execution plan](../plans/r5-classification.md) defines implementation slices.
The [smart-album UI contract](gallery-ui.md#smart-album-workflow) owns placement and interaction.
R4's [organization rules](r4-organization.md#boundary-with-r5-classification) remain the baseline.

## User outcome and scope

Ame extracts useful metadata, proposes a primary category for each logical image group, and gathers
current category results into `智能相册`. Initial screening produces immediately usable suggestions;
review is optional and concentrates on unresolved cases. The user can correct whole groups in batches
and resume review after restarting. Acceptance measures total human work saved alongside classification
quality, preserving existing albums, Favorites, and organization progress.

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
album tray, and have no direct add/remove membership commands. Their names and availability follow
the configured category definitions, independently of manual-album management. A category correction
changes the derived result; it does not move a physical file or edit a manual album.

## Primary taxonomy and review status

Categories are configurable flat definitions with stable identities, display names, and classification
descriptions. Preserve the text encoder so new descriptions do not require a fixed output-head class.
Example categories and evaluation labels do not limit the product to a fixed seven- or six-class list.
Display-only renaming preserves classification compatibility; semantic description changes and
adding/removing definitions create a new classification-rule revision. Retired definitions retain
human-decision provenance; remapping a saved human choice requires explicit intent.

Each classified group has at most one effective primary category. Define purpose-aware boundary
examples before model admission: a photographed toy used as a reaction image can primarily belong
to `表情包`, with photographic content serving as supporting evidence. Visible evidence and configured
descriptions determine this precedence; being a toy or photograph alone does not establish its use.
Auxiliary content evidence does not create several primary memberships or a general tagging feature.
Evaluate the same versioned rubric for every candidate without silently reinterpreting human choices.

Use these boundary examples when authoring and evaluating descriptions; they are not a fixed product
class list:

| Intended category | Positive evidence and boundary |
| --- | --- |
| Reaction image or meme | A recognizable conversational reaction, captioned joke or expressive sticker. Photographed toys and people can qualify; photographic appearance alone does not decide the primary use. |
| Comic | Sequential storytelling, narrative dialogue or identifiable comic-page context. An isolated character crop needs additional evidence to distinguish artwork from a reaction image. |
| Illustration | Standalone drawn or painted artwork without a dominant narrative or reaction purpose. Scanning a drawing does not by itself turn its primary content into a photograph. |
| Screenshot | Screen, application, game or conversation content is the useful subject. A border around artwork alone does not establish this purpose. |
| Photograph | A real scene, person, animal or object without stronger evidence for another configured purpose. |
| Other defined content | Positive descriptions such as documents, charts or design assets. Missing evidence and analysis failure remain separate. |

Start definitions broadly enough to cover the collection and split them when a recurring distinction
helps the user retrieve or organize images. Keep the scheme editable rather than selecting a fixed
target count. R5 does not automatically create a category for each unusual image or merge a category
because it contains few groups. Cultural meaning absent from visible evidence remains uncertain;
repeated examples and corrections can inform later evaluation without automatic online research or
unannounced retraining. The [compatibility contract](../architecture/0034-versioned-classification-contracts.md)
owns bounded definitions, canonical revisions and feature reuse.

`待确认` is a review filter, not a category. `其他` is a legitimate content category, not a substitute
for missing predictions, failed analysis, or a category conflict. A result can have a proposed
category while remaining eligible for optional review.

Confidence and confirmation are separate. A calibrated high-confidence result may be used for
browsing without mandatory individual review; it remains machine-proposed until explicitly confirmed
and remains eligible for sampling. Resolve machine-only disagreement automatically using the active
compatible model/rule revision and one canonical group result. Boundary cases receive at most one
bounded automatic re-screen with more specific versioned descriptions, reusing image features.
Compare calibrated score bands and candidate separation; raw scores are not probabilities. Historical
model disagreement alone does not add human work. Remaining uncertain predictions keep their best
usable suggestion in the category view and enter the optional `待确认` filter without a blocking prompt.
Genuine incompatible human decisions retain explicit conflict handling. Failed analysis retains its
reason and retry behavior without inventing a category. Deferred classification remains unfinished
review, and no review backlog blocks ordinary browsing or use of initial screening results.

`全部结果` includes the current smart-album scope regardless of review status. `待确认` selects work
selected for optional checking, including deferred review and unresolved human conflicts. `已确认`
selects explicit human confirmations and corrections. The section's all-results view also exposes
unresolved or failed items that cannot yet
belong to one category, so these items do not disappear between category views.
High-confidence unreviewed results can be in neither review subset; confirmed counts must not be
calculated by subtracting the pending count from the total.

## Group authority and content lifecycle

Classification commands resolve to the complete current exact-duplicate group across source folders,
including when selection contains one member or a folder-filtered subset. Duplicate selections are
applied once. Before exact identity is known, each admitted content version is a provisional command
target, but model execution waits for that content's compatible exact-fingerprint verification.
Consume committed R3 content evidence incrementally; do not wait for the entire duplicate scan or
start another source-tree traversal. Verified byte-identical content shares one compatible queued
or running analysis and its result without merging stable asset identities. Late copies join that
work; interrupted, obsolete, or failed verification never authorizes image inference.

Preserve these transitions:

- A new identical copy reuses compatible evidence and the group's applicable category decision.
- Compatible human decisions remain one decision after late grouping. Different human category
  decisions retain their provenance and require an explicit whole-group category choice. They do
  not create several confirmed primary categories or change R4's album-conflict queue.
- A model disagreement cannot override a valid human decision. Retain the newer model evidence
  separately for traceability rather than reopening completed review without a content conflict.
- Machine-only disagreement is reconciled through the active configuration and bounded re-screening.
  Keep the resulting best suggestion usable and reserve optional checking for remaining uncertainty.
  A reproducible same-input/runtime inconsistency is an analysis defect with bounded recovery, not
  a request for the user to diagnose the model or a reason to repeatedly scan the library.
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

The selected initial-screening candidate is SigLIP 2 Base/16-224 with an FP16 image encoder and INT8
text encoder through ONNX. Its [local exploratory comparison](../acceptance/r5-model-evaluation.md)
guides admission; CLIP B/32 INT8 remains a lightweight reference, and large models are excluded.
Product admission still verifies artifact licenses/integrity, worker recovery and resource limits.
The unfinished historical folder classification and provisional visual labels are not human ground truth.

Use representative labeled groups from the target library under existing source-safe execution
rules. Separate calibration and held-out evaluation by content group to prevent duplicate leakage.
Record per-category quality, confidence-band coverage and errors, sampled high-confidence quality,
abstentions/failures, checks required per thousand groups, correction time, and total human work saved.
Choose thresholds and optional-review sampling jointly with those costs; maximizing correctness by
sending most images to manual review does not satisfy R5. Calibration and final evaluation remain
separate, and a raw model score is not automatically a probability or a universally valid cutoff.

Use the two-core, 8-GiB cloud-primary client as a required resource baseline, with local-primary as
a comparison. Measure CPU, memory, model storage, cold/warm latency, sustained throughput, cancellation,
and gallery/viewer interference. CPU operation must remain usable; acceleration is an evaluated
optimization rather than a prerequisite. Record measured budgets before enabling background defaults.

Analysis runs off the UI thread with bounded workers, input windows, queues, caches, and runtime
threads. Foreground previews and interaction have priority. Process newly indexed or invalidated
content incrementally, reuse compatible exact-group work, and resume committed analysis after restart.
Pause/stop must reach a bounded checkpoint; worker failure must preserve the desktop session and
completed results. Never hydrate cloud-only files or hold catalog transactions over decoding/inference.

## Analysis lifecycle

Task admission, model residency, derived results, and human decisions have separate lifetimes.
Classification starts after explicit enablement and verified model setup. Queue existing catalog
content and later changes in bounded persistent windows; eligibility still requires each content
version's completed exact verification. Initial import and foreground image work retain priority.

| Event | Scheduled work |
| --- | --- |
| Enable initial screening | Queue eligible local catalog content; process incrementally as verification completes. |
| Publish new content | Join a compatible shared task/result or enqueue one analysis after exact verification. |
| Change content | Retire incompatible publication authority and analyze the new verified version. |
| Rename a path, change representative, or change workspace | Reuse compatible features/results; do not enqueue content work from presentation events. |
| Change semantic category descriptions or active definitions | Recompute text features and bounded matching over compatible image features. |
| Rename a display label | Update presentation without invalidating semantic features or decisions. |
| Correct a category | Commit whole-group human intent immediately; model work cannot overwrite it. |
| Explicitly update the model | Incrementally build compatible new results, retaining prior valid evidence and human decisions. |
| Source becomes unavailable | Retain work and records; wait for availability without hydrating placeholders. |
| Remove a source | Retire its task authority and reconcile surviving group members under the existing removal contract. |

Use the application scheduler's foreground-read/task signals and shared resource budget. Automatic
admission starts only after the first usable gallery and a quiet interval; three seconds is the
initial candidate inherited from R3 scheduling. Once admitted, cached browsing does not repeatedly
cancel useful work. Foreground image reads, explicit resource-heavy tasks, or measured pressure yield
at bounded small-batch checkpoints. Priority is explicit requested scope, new/changed content, then
existing backlog, with bounded fairness so continuous arrivals cannot starve older work.

Load a supervised worker only when eligible work can run. Cache compatible text vectors and release
the text encoder after descriptions are encoded. Keep the image encoder warm across short gaps;
start with a 60-second idle-residency candidate, to be measured during runtime admission. Memory
pressure, disabling analysis, shutdown, and suspend retire the worker promptly within the measured
checkpoint/termination budget. These timer candidates are tuning inputs, not measured guarantees.
Releasing runtime memory does not delete installed model files, durable queue state, or decisions.

Automatic yielding resumes when its condition clears. Manual pause persists across restarts until
explicit Continue. Disabling analysis fences new dispatch; retained suggestions and human decisions
stay usable. Shutdown/suspend fence dispatch and preserve committed progress; startup/resume checks
source/configuration generations before continuing previously automatic work after foreground readiness.
An unfinished batch is safely retryable, while committed work is reused. A failed file does not block
other candidates; permanent failures wait for changed content or explicit retry, and transient or
worker failures have bounded retry/backoff rather than endless restart loops.

Publish compatible committed batches without moving the current image, selection, or viewport anchor.
Use existing task and feedback owners; directory `已同步` remains independent of classification work.
Feature cleanup affects only rebuildable data and does not immediately schedule a whole-library
rebuild. Recompute missing features on actual analysis demand. An explicitly stopped run is not
silently recreated merely because its derived cache entry is absent.

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
- useful first-pass results, bounded automatic re-screening, per-category/band evidence, and measured
  checks/corrections per thousand groups and human time saved;
- batch review, explicit continuation, undo, cancellation, retry, stale-result rejection, and restart;
- late duplicate conflicts, content changes, root removal, cleanup, migration, and archive recovery;
- real desktop paths following the accepted UI contract, with shared feedback and stable layout;
- bounded two-core/8-GiB operation while ordinary browsing and preview remain usable;
- per-content release before global deduplication finishes, shared duplicate tasks, configuration-only
  rematching, persisted manual pause, bounded warm residency, shutdown/suspend recovery, and cache cleanup;
- unchanged original media and placeholder state, with no automatic file operation or hydration.

Record actual evidence in `docs/acceptance` when implementation exists. A layout sketch, requirements
review, or documentation check does not establish functional or model-quality acceptance.
