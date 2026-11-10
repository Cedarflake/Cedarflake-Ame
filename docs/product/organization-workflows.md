# Organization workflow requirements

Status: stage product requirements; implementation and acceptance status belong to the roadmap.

The [roadmap](../roadmap.md#delivery-sequence) alone selects delivery order and the active stage.
These requirements retain the complete R3–R10 outcomes, exclusions, and acceptance boundaries without
maintaining a second progress tracker. They complement the [workbench contract](workbench.md) and
[gallery UI contract](gallery-ui.md). File operations remain unavailable until their separately
authorized execution workflow; neither a requirement nor an internal operation plan authorizes
source changes.

## R3 - Exact duplicate understanding

User outcome:

The gallery folds byte-identical content by default so the user does not repeatedly review the same
image. A folded representative shows its physical-copy count, and `查看重复位置` expands every real
`AssetLocation`, availability state, and path without modifying a file. The user can still switch to
show every physical instance in Library or inspect exact duplicate groups in the Duplicates workspace.

Scope:

- written engine candidate evaluation and contract tests;
- size grouping, candidate pruning, versioned `ContentFingerprint` evidence, and immutable analysis
  runs;
- exact groups derived from compatible fingerprint evidence without redefining `Asset` as a hash
  group or permanently merging stable identities;
- deterministic representative selection, copy-count badges, and `AssetLocation` expansion with
  local, offline, unavailable, and cloud-backed state;
- `全部图片` and `合并重复图片` in the gallery filter menu, with a duplicate-location badge in both modes;
- a function rail, collapsible Library source sidebar and separate Duplicates workspace under ADR 0029;
- duplicate-group review in that workspace, including preferred-copy and ignore
  decisions that remain durable but do not authorize deletion;
- explicit distinction between logical group selection and physical-location selection;
- compatible reuse of expensive later analysis across exact byte identity without presenting reused
  model evidence as a human decision;
- automatic bounded analysis after catalog publication and while browsing is idle, with pause and
  continuation controls, compatible fingerprint reuse and priority for visible previews;
- invalidation and regrouping through R2c when content changes, a same-path replacement occurs, a
  location is removed, or compatible identity evidence changes.

R3 acceptance includes cancellation, retry, stale-result rejection, wrong-extension and damaged
files, cloud placeholders without hydration, exact group stability across restart, source-byte and
source-entry preservation, and bounded memory, database, and bridge behavior on the target library.

## R4 - Virtual organization and review foundation

User outcome:

The user can organize each exact-duplicate group into one flat virtual album, keep Favorites
independently, and resume organization in the existing gallery and viewer without changing source
files. [R4 requirements](r4-organization.md) own the detailed rules and acceptance boundary.

Scope:

- whole-group album actions, late duplicate folding, single-album assignment, and independent
  favorites;
- shared pending, deferred, organized, and ignored progress, including deferred album conflicts and
  content-change reconciliation;
- organization mode entered from the existing gallery, stable continuation, new groups at the queue
  front, and the [accepted desktop controls](gallery/organization.md#review-workflow);
- durable album/Favorite decisions, transactional progress, undo, migrations,
  export/restoration, explicit source-removal cleanup, and manual unresolved-record cleanup.

R4 does not yet invent model predictions. It establishes the durable authority and review mechanics
that prevent later automation from being mistaken for human confirmation.

## R5 - Metadata and smart albums

Ame extracts useful metadata and publishes usable initial-screening suggestions into `智能相册`.
Application-managed flat categories use purpose-aware matching and bounded automatic re-screening.
[R5 requirements](r5-classification.md) own taxonomy, model admission, recovery, resource, and
acceptance boundaries; the [execution plan](../plans/r5-classification.md) follows the roadmap order.

Scope:

- bounded metadata extraction and FTS5/structured search that preserve sources;
- admitted local models, calibrated confidence, immutable evidence, and bounded two-core/8-GiB execution;
- derived smart albums under Albums, sharing the gallery/viewer and independent of manual membership;
- incremental compatible analysis reuse, automatic group reconciliation, and failed-image retry;
- retained legacy category decisions and versioned archive compatibility.

Manual classification confirmation/correction, deferral, review filters, classification undo and
continuation are removed. Classification has no effect on R4 organization progress; assigning a
manual album still completes organization. Acceptance measures category/band quality, useful coverage,
automatic refinement, resource use, recovery, and source preservation without manual correction trials.

## R6 - Perceptual similarity review

User outcome:

The user reviews visually near-duplicate candidates such as recompressed, resized, or rotated images
without confusing them with byte-identical copies or semantic search results.

R6's proposed scope, interaction choices and implementation sequence live in the
[R6 plan](../plans/r6-similar-images.md). They are planning material; the current product contract
does not adopt them as implemented requirements.

## R7 - Semantic discovery and advanced search

User outcome:

The user can compose trustworthy structured search with natural-language visual discovery while
semantic relationships remain clearly separate from duplicates, categories, and file-operation
authority.

Scope:

- composed bounded queries over source, folder, date, exact-duplicate state, effective category,
  album, review status, dimensions, type, filename, path, and admitted metadata;
- an admitted local embedding runtime with versioned parameters, bounded model and index storage,
  cancellation, replacement tests, and traceable analysis runs;
- CLIP-style natural-language image search and semantic-neighbor discovery as separate evidence
  types;
- query-wide result manifests, stable identity, time distribution where meaningful, and bounded
  detail windows in the existing gallery;
- no use of semantic similarity as exact duplicate proof, category override, automatic keep/delete
  decision, or filesystem-operation authority.

## R8 - File organization and recovery

User outcome:

The user can select images, choose a file action and destination or naming rule, resolve blocking
conflicts, and authorize execution. Ame checks current files, reports progress and partial results,
and supports cancellation and recovery after interruption.

This combines the former R8 and R9 scopes. There is no organization-preview page, optional preview,
before/after listing, saved-plan manager, or plan export. The action control identifies the affected
scope and destination; conflict handling and execution feedback remain part of the operation.

Windows capabilities perform file transfers, renames, recycling and explicitly authorized permanent
deletion. Ame owns authorization, operation records, progress, catalog consistency and recovery
coordination. Reuse the system Recycle Bin by default. Permanent deletion requires the user's
explicit choice in Ame's deletion confirmation or an affirmative Windows confirmation when
recycling is unavailable; no silent
fallback is allowed. The [execution plan](../plans/r8-file-organization.md#planned-interaction-decisions)
owns the planned interaction details, system-interface admission and rollout.

Deletion authorization applies to the file version the user selected and confirmed. An editor or
synchronization program changing that file does not authorize deleting its replacement. Reject a
changed or unverifiable item, preserve it, and require the user to view the current image and make
a fresh selection and confirmation. Do not infer that an unchanged filename means unchanged intent.
Show the affected filename and reason in operation results; changed files must be identified as
not deleted, not hidden behind an overall completion message. A partial batch retains separate
completed and unresolved counts. Unknown native outcomes remain pending verification rather than
being described as either deleted or preserved.

Scope:

- organization rules based on effective category, selected virtual albums, date, source, or explicit
  user choices;
- background validation of target roots and path templates, including invalid or reserved names,
  path length, unsupported targets, and source overlap;
- skipping and reporting multiple album destinations during album-based physical organization;
  Favorites remain independent, and neither a favorite nor an unresolved album candidate implies a
  physical destination;
- deterministic target-collision, existing-file, unavailable-source, cloud-placeholder, permission,
  cross-volume, and insufficient-space analysis;
- an internal immutable `OperationPlan` containing source, destination and collision policy, action
  kind, reason, expected source state and fingerprint evidence, target preconditions, conflicts,
  warnings, and estimated space impact;
- explicit resolution of the physical locations affected when one gallery item represents exact
  duplicates; grouping alone never authorizes removing or moving every copy;
- ordinary gallery deletion may remove the preferred file; successful deletion chooses a new
  preferred file from remaining duplicates, or removes the card when none remain;
- fresh action authorization bound to one immutable `OperationPlan`; background preparation never
  authorizes execution, and a changed scope, destination root, deletion mode or collision policy
  requires renewed authorization rather than modification of the earlier evidence;
- immediate per-item revalidation of root generation, physical identity, source state, compatible
  fingerprint evidence, target state, available space, and permissions before mutation;
- system-backed same-volume move or rename with numbered collision names, no overwrite and
  recoverable outcomes;
- cross-volume transfer that establishes a complete, verified destination before authorized source
  removal and preserves a valid copy through interruption; use native transfer semantics where they
  satisfy these guarantees, otherwise compose native copy, bounded verification and source removal;
- durable `OperationJournal` entries for intended action, before evidence, each state transition,
  verification, result, failure, compensation, and recovery decision;
- idempotent restart recovery, cancellation at safe boundaries, retry, skip, and clear partial-
  failure reporting without presenting an incomplete run as success;
- atomic or explicitly staged catalog reconciliation after verified physical changes, using actual
  system-reported destinations; preserve durable user decisions through moves and recovery;
- source and target safety fixtures for Chinese and long paths, conflicts, locked files, unavailable
  roots, cloud placeholders, cross-volume interruption, database failure, and process termination;
- deletion confirmation defaults to recycling and offers an unchecked permanent-delete option;
- a TXT result explanation in the export root for automatic renames, skipped items and other
  exceptional outcomes, derived from durable operation records.

System-backed execution replaces the earlier prescribed temporary-target publication sequence.
Content preservation for transfers, explicit deletion authority and applicable recovery remain
required outcomes; the system's internal copy algorithm and an Ame-specific quarantine are not
product requirements.

Preflight remains deterministic for the same trustworthy inputs and leaves source and target media
unchanged. Execution changes only the authorized items, rejects stale evidence, and preserves
applicable recovery without offering a restore for permanent deletion. The
[filesystem verification procedure](../acceptance/r8-file-operations.md) owns disposable fixtures,
failure checks, and Windows integration evidence.

## R10 - Large-library maturity and release readiness

Scope:

- cold and warm scan performance;
- cancellation latency and crash recovery;
- peak memory and cache-size enforcement;
- catalog migration and application upgrade tests;
- remaining condition-triggered million-item manifest adaptation and extended synchronization
  catch-up evidence that was not required for earlier target-library value, including target-root
  authoritative recovery and publication timing;
- general installed-product updates, diagnostics export, and recovery documentation;
- formal i18n infrastructure and additional locale catalogs only after product copy is stable and
  the supported locales are separately confirmed;
- controlled read-only combined scan of both real roots;
- regression comparison against recorded Lap behavior without importing Lap code.

Code signing is optional, not an R10 exit criterion or a prerequisite for verified unsigned
Windows distribution. The existing automated GitHub publication pipeline remains a separately
selected signed channel; it does not provide unsigned automated publication.

Release packaging, hosted quality workflows, and portable-archive verification may be built earlier
as cross-cutting infrastructure. Their existence does not make R10 complete or turn release work
into a second active product stage.

## Product value checkpoints

These checkpoints describe user value and do not redefine the repository's current semantic
version, which is managed by the release process:

| Checkpoint | Required stages | User value |
| --- | --- | --- |
| Trustworthy foundation | R2b and R2c | Stable large-library canvas with continuously trustworthy source state |
| Exact and virtual organization | R3 and R4 | Exact repetition is folded, physical locations are inspectable, and durable virtual organization can begin |
| Smart albums | R5 | Automatic initial screening supplies useful category views while preserving legacy records and R4 decisions |
| Similarity understanding | R6 | Near-duplicate candidates can be compared and reviewed without automatic deletion |
| Advanced discovery | R7 | Structured and semantic search help rediscover understood content |
| Authorized organization | R8 | File actions execute with explicit authorization, revalidation, journaling, and recovery |
| Release maturity | R10 | Installation, migration, diagnostics, resource limits, and long-term reliability are ready |
