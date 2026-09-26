# Organization workflow requirements

Status: planned product scope, not implementation or acceptance evidence.

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
show every physical instance or only exact duplicate groups.

Scope:

- written engine candidate evaluation and contract tests;
- size grouping, candidate pruning, versioned `ContentFingerprint` evidence, and immutable analysis
  runs;
- exact groups derived from compatible fingerprint evidence without redefining `Asset` as a hash
  group or permanently merging stable identities;
- deterministic representative selection, copy-count badges, and `AssetLocation` expansion with
  local, offline, unavailable, and cloud-backed state;
- exact duplicate display modes and review command within the gallery filter menu;
- contextual duplicate-group review in the existing canvas, including preferred-copy and ignore
  decisions that remain durable but do not authorize deletion;
- explicit distinction between logical group selection and physical-location selection;
- compatible reuse of expensive later analysis across exact byte identity without presenting reused
  model evidence as a human decision;
- invalidation and regrouping through R2c when content changes, a same-path replacement occurs, a
  location is removed, or compatible identity evidence changes.

R3 acceptance includes cancellation, retry, stale-result rejection, wrong-extension and damaged
files, cloud placeholders without hydration, exact group stability across restart, source-byte and
source-entry preservation, and bounded memory, database, and bridge behavior on the target library.

## R4 - Virtual organization and review foundation

User outcome:

The user can place assets in Favorites and user-created virtual albums without changing source
files, record durable decisions, and resume an interrupted organization or review session at the
exact logical item and progress position.

Scope:

- durable user-owned data separated from rebuildable catalog, preview, and model evidence;
- one album-membership model containing the built-in Favorites group and user-created albums rather
  than parallel favorite and album authorities;
- virtual many-to-many membership that survives compatible reindexing and never implies a physical
  destination;
- an expandable `相册` sidebar section and the selection-owned `加入相册` workflow defined in section
  4 without creating another gallery application;
- durable `UserOverride` infrastructure that later analysis stages can consume without overwriting
  human intent;
- persistent `ReviewSession` identity, owning query, logical asset anchor, progress, decisions,
  keyboard mapping, undo boundary, deferred items, issue state, and cross-restart continuation;
- one lightweight `继续整理` entry in the existing gallery when resumable work exists;
- migrations, backup or export, restoration, and tests proving that catalog rebuilds do not erase
  albums, review progress, or user decisions.

R4 does not yet invent model predictions. It establishes the durable authority and review mechanics
that prevent later automation from being mistaken for human confirmation.

## R5 - Metadata, primary classification, and human review

User outcome:

Ame processes the bulk of common image understanding, publishes calibrated primary-category
predictions, and asks the user to review only sampled, uncertain, conflicting, or failed results.
The user can correct categories quickly, close the application, and continue from the same durable
review session later.

Primary taxonomy:

- photo;
- anime or illustration;
- screenshot;
- meme or reaction image;
- document image;
- design asset;
- other;
- needs review.

Scope:

- supported metadata extraction through admitted adapters, including capture evidence, dimensions,
  camera or basic metadata where trustworthy, and no embedded metadata writes;
- SQLite FTS5 and bounded structured search over filename, path, date, type, size, source, album,
  category, and supported metadata;
- written model and runtime admission evidence, local inference, bounded model storage, immutable
  analysis runs, model provenance, versioned parameters, confidence, and failure evidence;
- separate `ModelPrediction`, `UserOverride`, `EffectiveCategory`, and `ReviewStatus` persistence;
- confidence bands calibrated against the target library rather than hard-coded assumptions: high
  confidence may reduce routine review but remains eligible for sampling, medium and low confidence
  enter review, and conflicts or analysis failures receive explicit queues;
- compatible analysis reuse for byte-identical content without coupling user decisions to an
  absolute path or silently copying an override to unrelated content;
- effective-category smart albums derived atomically from the current compatible model run and
  durable overrides;
- keyboard-first individual and safe batch review, accept-model, category correction, defer, undo,
  progress, estimated remaining work, and cross-restart continuation through R4's `ReviewSession`;
- bounded analysis of newly indexed or invalidated assets without rebuilding every result group;
- reanalysis that preserves older model evidence for traceability and never erases user intent;
- no face recognition, identity naming, or automatic file operation.

R5 acceptance records coverage and error evidence per confidence band, sampled high-confidence
quality, the remaining human-review count, review throughput, override survival after reanalysis,
and source preservation. Success is reduced trustworthy human work, not a large headline number of
automatically labelled images.

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
- explicit handling when one asset belongs to multiple albums selected for physical materialization:
  choose a primary album, keep the current location, use effective category, or exclude the item;
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
