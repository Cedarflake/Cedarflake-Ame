# Organization workflow requirements

Status: stage product requirements; implementation and acceptance status belong to the roadmap.

The [roadmap](../roadmap.md#delivery-sequence) alone selects delivery order and the active stage.
These requirements retain the complete R3–R10 outcomes, exclusions, and acceptance boundaries without
maintaining a second progress tracker. They complement the [workbench contract](workbench.md) and
[gallery UI contract](gallery-ui.md). File operations remain unavailable until their separately
authorized execution workflow; neither a requirement nor a dry-run plan authorizes source changes.

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
  front, and the [accepted desktop controls](gallery-ui.md#review-workflow);
- durable album/Favorite decisions, transactional progress, undo, migrations,
  export/restoration, explicit source-removal cleanup, and manual unresolved-record cleanup.

R4 does not yet invent model predictions. It establishes the durable authority and review mechanics
that prevent later automation from being mistaken for human confirmation.

## R5 - Metadata, primary classification, and human review

User outcome:

Ame extracts useful metadata, publishes calibrated primary categories into `智能相册`, and helps
the user confirm or correct groups in batches. [R5 requirements](r5-classification.md) own the full
taxonomy, model admission, review, recovery, resource, and acceptance boundaries; the
[execution plan](../plans/r5-classification.md) follows the roadmap's implementation sequence.

Scope:

- metadata extraction and FTS5/structured search through bounded, source-preserving adapters;
- admitted local models, calibrated confidence, immutable analysis evidence, and bounded execution
  on the two-core/8-GiB baseline;
- separate predictions, human category decisions, effective categories, and classification review;
- derived smart albums under Albums, sharing the gallery/viewer and independent of manual membership;
- whole-group confirmation/correction, sampling, deferred review, undo, and durable continuation;
- incremental compatible analysis reuse, group reconciliation, and versioned decision recovery.

`待确认` is a review filter, not a primary category. Confirmation does not complete R4 organization;
manual-album assignment remains the operation that does. R5 has its own classification session and
bookmark while reusing R4's guarded commands and continuation principles. Acceptance measures
category/band quality, remaining human work, review throughput, recovery, and source preservation.

## R6 - Perceptual similarity review

User outcome:

The user reviews visually near-duplicate candidates such as recompressed, resized, or rotated images
without confusing them with byte-identical copies or semantic search results.

Scope:

- comparative candidate-engine evaluation;
- versioned, explainable thresholds and evidence for recompression, resize, rotation, crop, watermark,
  or other admitted transformations;
- candidate groups, side-by-side comparison, dimensions and file-size evidence, preferred-copy,
  keep-both, confirm, and not-similar decisions;
- immutable analysis runs plus durable review decisions and progress through `ReviewSession`;
- bounded candidate generation, cancellation, retry, stale-result rejection, and reanalysis without
  erasing historical evidence;
- no automatic deletion, exact-duplicate label, or implicit physical-location choice.

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

## R8 - Physical organization dry-run

User outcome:

The user can define how virtual knowledge would map to a target filesystem, inspect the complete
result, and resolve conflicts without Ame changing any source or target file.

Scope:

- organization rules based on effective category, selected virtual albums, date, source, or explicit
  user choices;
- target-root and path-template preview with invalid-name, reserved-name, path-length, unsupported
  target, and source-overlap validation;
- explicit handling of unresolved album-assignment conflicts before physical materialization;
  Favorites remain independent, and neither a favorite nor an unresolved album candidate implies a
  physical destination;
- deterministic target-collision, existing-file, unavailable-source, cloud-placeholder, permission,
  cross-volume, and insufficient-space analysis;
- an immutable, non-executable `OperationPlan` containing source and target, action kind, reason,
  expected source state and fingerprint evidence, target preconditions, conflicts, warnings, and
  estimated space impact;
- summary counts for move, copy, rename, keep-in-place, exact duplicate locations, conflicts,
  collisions, unavailable items, and items requiring another decision;
- plan review, filtering, export, and regeneration as a new plan rather than mutation of historical
  evidence;
- no filesystem execution control, delete shortcut, implicit conflict resolution, or authorization
  carried forward into R9.

R8 acceptance proves that repeated generation from the same trustworthy inputs is deterministic,
that stale or incomplete evidence is reported rather than guessed, and that source and target trees
remain byte-for-byte and entry-for-entry unchanged.

## R9 - Explicitly authorized filesystem execution

User outcome:

After reviewing a specific current plan, the user can freshly authorize its supported actions.
Ame revalidates every item, records durable execution evidence, handles partial failure, and can
recover safely after interruption.

Scope:

- fresh authorization bound to one immutable `OperationPlan`; viewing or generating a plan never
  authorizes execution;
- immediate per-item revalidation of root generation, physical identity, source state, compatible
  fingerprint evidence, target state, available space, and permissions before mutation;
- same-volume move or rename behavior with explicit overwrite policy and recoverable intermediate
  state;
- cross-volume copy to a temporary target, bounded content verification, atomic target publication,
  catalog update, and only then the separately planned source recycle-bin or quarantine action;
- durable `OperationJournal` entries for intended action, before evidence, each state transition,
  verification, result, failure, compensation, and recovery decision;
- idempotent restart recovery, cancellation at safe boundaries, retry, skip, and clear partial-
  failure reporting without presenting an incomplete run as success;
- atomic or explicitly staged catalog reconciliation after verified physical changes;
- source and target safety fixtures for Chinese and long paths, conflicts, locked files, unavailable
  roots, cloud placeholders, cross-volume interruption, database failure, and process termination;
- permanent deletion remains unavailable by default and requires a separately accepted policy and
  explicit action-level authorization if ever introduced.

## R10 - Large-library maturity and release readiness

Scope:

- cold and warm scan performance;
- cancellation latency and crash recovery;
- peak memory and cache-size enforcement;
- catalog migration and application upgrade tests;
- remaining condition-triggered million-item manifest adaptation and extended synchronization
  catch-up evidence that was not required for earlier target-library value, including target-root
  authoritative recovery and publication timing;
- general installed-product updates, signing maturity beyond the R2c broker installer, diagnostics
  export, and recovery documentation;
- formal i18n infrastructure and additional locale catalogs only after product copy is stable and
  the supported locales are separately confirmed;
- controlled read-only combined scan of both real roots;
- regression comparison against recorded Lap behavior without importing Lap code.

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
| Machine-assisted review | R5 | Classification reduces manual work while corrections and review progress remain durable |
| Similarity understanding | R6 | Near-duplicate candidates can be compared and reviewed without automatic deletion |
| Advanced discovery | R7 | Structured and semantic search help rediscover understood content |
| Safe physical proposal | R8 | A complete non-executable organization plan can be inspected and resolved |
| Authorized organization | R9 | Reviewed filesystem changes execute with revalidation, journaling, and recovery |
| Release maturity | R10 | Installation, migration, diagnostics, resource limits, and long-term reliability are ready |
