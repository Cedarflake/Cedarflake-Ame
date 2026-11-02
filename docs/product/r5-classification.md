# R5 - Metadata and smart albums

Status: product requirements; implementation availability and delivery status belong to the roadmap.

This document owns R5 behavior. The [roadmap](../roadmap.md#r5-implementation-sequence) owns order and
stage decisions; the [execution plan](../plans/r5-classification.md) defines implementation slices.
The [smart-album UI contract](gallery-ui.md#smart-album-workflow) owns placement and interaction.
R4's [organization rules](r4-organization.md#boundary-with-r5-classification) remain the baseline.

## User outcome and scope

Ame extracts useful metadata, proposes a primary category for each logical image group, and gathers
current category results into `智能相册`. Initial screening produces usable suggestions, with bounded
automatic re-screening for ambiguous results. Users browse these results through the existing gallery
and can independently add images to manual albums or Favorites.

R5 includes bounded metadata and structured search, local model evaluation and inference, calibrated
confidence, immutable analysis evidence, derived smart albums, and failure recovery. It has no manual
classification confirmation, correction, deferral, or review-continuation workflow. Existing saved
classification decisions remain compatible with reading and backup restoration; removing the workflow
does not erase those records or change their established precedence.

The existing gallery, viewer, source-safety rules, and whole-group organization targets remain.
Perceptual similarity, semantic search, face recognition, identity naming, embedded metadata writes,
and physical file operations remain outside R5. The [stage overview](organization-workflows.md)
retains their owners.

## Vocabulary and independent decisions

Use `智能相册` for the visible entry and sidebar section. Its category results derive from
`EffectiveCategory` and have no user-managed membership list.

| Concept | Authority and effect |
| --- | --- |
| Model prediction | Immutable output with model/run identity, taxonomy version, parameters, confidence, and evidence. It remains model evidence. |
| Legacy category decision | An existing durable human choice retained for compatibility; reanalysis and restoration preserve its authority. The current desktop creates no new choices. |
| Effective category | An applicable legacy decision, otherwise a compatible current model result. Conflicting legacy decisions retain their provenance without a guessed winner. |
| Smart album | A bounded view of groups with that effective category. |
| Manual album | R4's single user-album assignment for the complete group. |
| Favorite | R4's independent preference, compatible with either kind of album view. |

A group can appear in the `动漫／插画` smart album, the user's `壁纸` album, and Favorites together.
It still has only one manual-album assignment. Smart albums are not destinations in the album tray
and have no add/remove membership commands. Their names and availability follow application-managed
category definitions independently of manual-album management.

## Primary taxonomy and result quality

Categories are application-managed flat definitions with stable identities, display names, and
classification descriptions. Ame supplies the starting vocabulary and maintains useful categories
from recurring library evidence. Enabling smart albums does not require the user to name categories,
write comparison descriptions or manage model prompts. The desktop offers no manual definition
creation, editing, or removal. Previously saved definitions and their protection remain compatible.
Preserve the text encoder so new descriptions do not require a fixed output-head class. Example
categories and evaluation labels do not limit the product to a fixed seven- or six-class list; making
that list manually editable alone does not satisfy category discovery or maintenance.
Display-only renaming preserves classification compatibility; semantic description changes and
adding/removing definitions create a new classification-rule revision. Retired definitions retain
legacy decision provenance; automatic maintenance does not remap a saved choice.
If fresh evidence supports a previously retired automatic category again, restore its existing
identity and original meaning through current admission. Legacy user edits and manual retirement protect
that definition from automatic restoration; historical origin metadata alone is insufficient.

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

Start definitions broadly enough to cover the collection. Ame must detect recurring distinctions
that support useful additions or splits, and excessive overlap that warrants a scheme adjustment.
Candidate discovery, naming and admission belong to the application; a user-authored category table
is not a prerequisite. Admit the maintenance policy against useful distinctions, stability,
and coverage rather than a fixed target count. A larger hard-coded list is not a replacement for
this policy. Isolated unusual images and small occupancy alone do not create or remove a category.
Automatic maintenance preserves stable identities and saved legacy choices; it never silently
remaps a saved human category or requests confirmation for every image or routine vocabulary change.
Addition capacity follows distinct verified content and the recurrence required for a useful category;
duplicate copies do not expand it. The capacity is a ceiling rather than a target. Users do not need
to choose a category count, and a smaller library does not by itself remove retained definitions.
An automatic refinement can return to its original purpose category after complete current-library
evidence falls below the recurrence needed to create it and every remaining image has that purpose
as its unique best alternative. Changed meanings, missing evidence and saved legacy choices preserve
the category. A fresh compatible assessment can later restore the original refinement identity.
An existing broader refinement of the same purpose can also receive a sparse category when its
declared meaning contains the narrower context, it remains recurrent, and all affected images agree
on that alternative. Similar names or scores alone do not combine distinct uses.
Cultural meaning absent from visible evidence remains uncertain; labeled evaluation examples
can inform quality assessment without automatic online research or unannounced retraining. The
[compatibility contract](../architecture/0034-versioned-classification-contracts.md) owns bounded
definitions, canonical revisions, policy admission and feature reuse.

`其他` is a legitimate content category, separate from missing predictions, failed analysis, and
unresolved legacy conflicts. Smart albums do not expose pending/confirmed review filters or badges.

Resolve machine-only disagreement automatically using the active compatible model/rule revision and
one canonical group result. Boundary cases receive at most one bounded automatic re-screen with more
specific versioned descriptions, reusing image features. Compare calibrated score bands and candidate
separation; raw scores are not probabilities. Remaining uncertainty keeps its best usable suggestion
in the category view without requesting a user decision. Confidence and ambiguity remain analysis
evidence for evaluation, rather than a manual-review queue.

The ordinary Library includes groups without usable category evidence, including failed analysis
and unresolved legacy conflicts. Per-image analysis failures remain internal and recover silently
according to the failure and changed inputs. Cards and previews show no analysis-failure label,
indicator, notification, or retry command. These items remain reachable without a separate all-results
entry or an invented category.

## Group authority and content lifecycle

Classification results resolve to the complete current exact-duplicate group across source folders.
Before exact identity is known, each admitted content version has provisional identity; model
execution waits for that content's compatible exact-fingerprint verification.
Consume committed R3 content evidence incrementally; do not wait for the entire duplicate scan or
start another source-tree traversal. Verified byte-identical content shares one compatible queued
or running analysis and its result without merging stable asset identities. Late copies join that
work; interrupted, obsolete, or failed verification never authorizes image inference.

Preserve these transitions:

- A new identical copy reuses compatible evidence and the group's applicable category decision.
- Compatible human decisions remain one decision after late grouping. Different human category
  decisions retain their provenance with no effective winner. The current desktop does not offer
  conflict resolution for these legacy records or change R4's album-conflict queue.
- A model disagreement cannot override a valid human decision. Retain the newer model evidence
  separately for traceability without changing the saved category authority.
- Machine-only disagreement is reconciled through the active configuration and bounded re-screening.
  Keep the resulting best suggestion usable while retaining its uncertainty evidence.
  A reproducible same-input/runtime inconsistency is an analysis defect with bounded recovery, not
  a request for the user to diagnose the model or a reason to repeatedly scan the library.
- Changed content invalidates incompatible prediction and decision bindings. Unchanged copies keep
  their decisions; genuinely different content receives fresh analysis or a compatible destination
  group's existing decision. A path or representative change alone is not a new category decision.
- Temporarily unavailable sources retain compatible records. Explicit root removal and manual
  unresolved-record cleanup follow R4's surviving-group and deliberate-removal rules; reimport alone
  cannot resurrect removed decisions. Deleting a manual album does not delete category decisions.

Revalidate content, group generation, model compatibility, and affected decision revisions before
publishing results or admitting a retry. Retained display evidence must not authorize a stale
write. Old model tasks, pages, and viewer callbacks cannot publish into a newer source or query scope.

## Classification and organization progress

| Action | Classification effect | R4 organization effect |
| --- | --- | --- |
| Publish a prediction | Update compatible evidence and derived results | None |
| Recover failed analysis | Resume the compatible shared task when its recovery condition is met | None |
| Add or move to a manual album | None | Apply R4 membership and organized-state rules |
| Remove from the last manual album | None | Become `已忽略` under R4 |
| Favorite or unfavorite | None | Change Favorite only |

Adding an item to a manual album from a smart-album view uses the existing tray and ordinary-mode
behavior. A group ignored for R4 organization can still have a category result. Classification does
not infer an album destination, mark organization complete, or change Favorite state.

Smart albums retain normal browsing, bounded selection, sorting, the shared gallery filter, and viewer
navigation. There are no single-image or batch confirmation/correction commands, deferred-review
actions, review-status filters, classification undo, or classification continuation. Opening an image
or menu does not create manual classification activity. New results and grouping changes preserve
the currently viewed image and browsing position under the shared gallery rules.

R4's organization mode, chronological undo/redo, bookmark, and retained Library context remain
independent. Existing classification bookmarks and operation history are retained only for archive
compatibility and explicit cleanup; the desktop does not resume or extend them.

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

The desktop metadata-condition form was withdrawn on 2026-11-02. The shared Filter menu retains only
descendant and duplicate-display choices. Keep metadata extraction and typed native query/archive
compatibility; no additional condition editor is required. Continuing organization uses the current
desktop query without resurrecting retired metadata restrictions from a saved bookmark.

## Model admission and low-resource execution

Evaluate candidate models and runtimes before selecting dependencies. Record exact versions, code
and weight licenses, redistribution terms, download/storage sizes, artifact integrity, Windows CPU
support, failure isolation, maintenance, and replacement contracts. A runtime mentioned in planning
is not an admitted dependency. Model setup and downloads require an explicit user action.
The normal setup offers the application's admitted model and manages its verified files. Ship the
compatible inference runtime with the application. Users enable smart albums and authorize any
required model download; they do not choose model/runtime directories or supply category definitions.
The Settings page has no smart-album section or status prompt. Missing models offer download from
the smart-album empty state; installed models need no setup prompt. Installation failure uses the
shared download feedback's retry action without enabling analysis against partial files.

The selected initial-screening candidate is SigLIP 2 Base/16-224 with an FP16 image encoder and INT8
text encoder through ONNX. Its [local exploratory comparison](../acceptance/r5-model-evaluation.md)
guides admission; CLIP B/32 INT8 remains a lightweight reference, and large models are excluded.
Product admission still verifies artifact licenses/integrity, worker recovery and resource limits.
The unfinished historical folder classification and provisional visual labels are not human ground truth.

Use representative labeled groups from the target library under existing source-safe execution
rules. Separate calibration and held-out evaluation by content group to prevent duplicate leakage.
Record per-category quality, confidence-band coverage and errors, sampled high-confidence quality,
abstentions/failures, and the effect and cost of bounded automatic re-screening. Choose thresholds
against usable coverage and observed errors. Manual classification trials, correction counts, and
human correction-time comparisons are outside R5 acceptance. Calibration and final evaluation remain
separate, and a raw model score is not automatically a probability or a universally valid cutoff.

Use the two-core, 8-GiB cloud-primary client as a required resource baseline, with local-primary as
a comparison. Measure CPU, memory, model storage, cold/warm latency, sustained throughput, cancellation,
and gallery/viewer interference. CPU operation must remain usable; acceleration is an evaluated
optimization rather than a prerequisite. Record measured budgets before enabling background defaults.

Analysis runs off the UI thread with bounded workers, input windows, queues, caches, and runtime
threads. Foreground previews and interaction have priority. Process newly indexed or invalidated
content incrementally, reuse compatible exact-group work, and resume committed analysis after restart.
Automatic yielding and shutdown must reach a bounded checkpoint; worker failure must preserve the desktop session and
completed results. Never hydrate cloud-only files or hold catalog transactions over decoding/inference.

## Analysis lifecycle

Task admission, model residency, derived results, and retained legacy decisions have separate lifetimes.
Classification runs automatically after explicit model download and verified setup. The desktop
offers no pause, continue, stop or disable controls. Queue existing catalog
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
pressure, shutdown, and suspend retire the worker promptly within the measured
checkpoint/termination budget. These timer candidates are tuning inputs, not measured guarantees.
Releasing runtime memory does not delete installed model files, durable queue state, or decisions.

Automatic yielding resumes when its condition clears. The manual-control workflow was withdrawn on
2026-11-02. At startup, an installed legacy paused setup resumes the same run; installed disabled or
stopped setup activates a new run through the existing guarded configuration transaction. A stopped
run identity remains retired, compatible evidence and failure budgets remain reusable, and missing
models never trigger a download. Shutdown/suspend fence dispatch and preserve committed progress;
startup/resume checks source/configuration generations before continuing after foreground readiness.
An unfinished batch is safely retryable, while committed work is reused. A failed file does not block
other candidates. Short-lived worker failures retry after 5 seconds and then 30 seconds; three
failed attempts exhaust that input's budget. Permanent or exhausted failures wait for a changed
source/content version or model/rule compatibility. An unchanged catalog notification, representative
change, pause/continue, or application restart does not reset the budget. Unavailable sources wait
for relevant availability evidence or a verified surviving copy without consuming that budget.
No per-image recovery step requires user action. Setup and unknown runtime errors stop analysis,
retain queued work, and appear through the existing error feedback; successful model/configuration
reopening clears the runtime error. Enablement does not report success when session startup fails.

Publish compatible committed batches without moving the current image, selection, or viewport anchor.
Use existing task and feedback owners; directory `已同步` remains independent of classification work.
Feature cleanup affects only rebuildable data and does not immediately schedule a whole-library
rebuild. Recompute missing features on actual analysis demand. An explicitly stopped run is not
silently recreated merely because its derived cache entry is absent.

## Persistence and recovery

Persist immutable model evidence and rebuildable effective-category projections separately from
R4 decisions and legacy classification records. Runs retain model/taxonomy versions and parameter
identity. Reanalysis, model removal, and cache cleanup preserve applicable saved category decisions.

Retain the existing schema and versioned portable recovery compatibility for category definitions,
legacy choices, association evidence, bookmarks, and operation history. Removing desktop commands
requires no destructive migration or record purge. Existing records retain their established
precedence through restart, rebuild, reanalysis, and restoration; conflicting legacy choices remain
unresolved rather than selecting a model winner. Models, original media, and rebuildable projections
remain outside decision archives. R3/R4 archives retain their original behavior.

Explicit source removal and the existing unresolved-record cleanup continue to retire eligible
classification records with their lineage. Preserve the current confirmation, eligibility recheck,
anti-resurrection guards, and explicit-backup recovery.

## Acceptance requirements

R5 must demonstrate:

- preserved R4 album/Favorite/progress behavior across analysis, retries, and model updates;
- coherent group-based category pages, counts, search, selection, time/name rails, and viewer scope;
- useful first-pass results, bounded automatic re-screening, per-category/band evidence, and measured
  coverage, error, and automatic re-screening results;
- absence of manual classification controls and review filters, with cause-aware recovery, cancellation,
  stale-result rejection, and restart;
- late duplicates, content changes, root removal, cleanup, migration, and legacy archive recovery;
- real desktop paths following the accepted UI contract, with shared feedback and stable layout;
- bounded two-core/8-GiB operation while ordinary browsing and preview remain usable;
- per-content release before global deduplication finishes, shared duplicate tasks, configuration-only
  rematching, automatic recovery from legacy modes, bounded warm residency, shutdown/suspend recovery, and cache cleanup;
- unchanged original media and placeholder state, with no automatic file operation or hydration.

Record actual evidence in `docs/acceptance` when implementation exists. A layout sketch, requirements
review, or documentation check does not establish functional or model-quality acceptance.
