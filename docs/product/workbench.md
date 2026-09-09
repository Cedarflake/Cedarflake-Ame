# Workbench product contract

Status: retained product decisions; implementation availability follows the [roadmap](../roadmap.md).

This document owns product purpose, concepts, scope, and reference policy. It does not select the
active delivery stage or claim that a planned capability is implemented. Technical decisions remain
owned by the accepted [ADRs](../architecture/README.md), and engineering rules by
[the repository contract](../../AGENTS.md).

## 1. Product definition

Cedarflake Ame is an independently implemented, local-first Windows organizing workbench for more
than 70,000 personal images. It is not another general-purpose photo album. Its central workflow is
to establish a trustworthy catalog, collapse exact repetition, let machines process the bulk of
image understanding, preserve human corrections and review progress, build virtual organization,
and only then propose changes to real files.

The unified gallery remains the primary working canvas, not the product endpoint. Source, time,
search, duplicate state, classification, albums, and review sessions scope the same canvas rather
than becoming separate competing applications.

The real target library is approximately 259 GB across two machine-local roots:

- `local-primary`: the primary locally stored image library;
- `cloud-primary`: the primary cloud-backed image library.

Their exact filesystem paths and machine-specific identity are stored only in the ignored
`.agents/local-context.toml` mapping. Repository documents, tests, logs, and commits use only these
logical IDs. The mapping is discovery data and does not itself authorize source mutation or a new
real-library acceptance run.

Ame must not require a second complete copy of these sources. Cataloging, browsing, analysis,
review, and virtual organization do not modify original media. R8 produces only an immutable,
non-executable organization plan. File-changing operations enter the product only in the separately
approved and freshly authorized R9 execution stage.

Product success is measured by reduced human work and trustworthy decisions, not only by indexed
file count or gallery feature breadth. Each applicable stage records:

- exact duplicate groups and physical locations understood without source mutation;
- model coverage split by confidence and the number of items still requiring human review;
- review throughput, resumable progress, undo behavior, and durable user decisions;
- virtual organization coverage and unresolved organization conflicts;
- the proportion of a proposed physical plan that is ready, blocked, unchanged, or requires an
  explicit choice;
- source-state, cloud-placeholder, and operation-safety evidence proving that Ame never presented
  guesses as user decisions or changed files before authorization.

## 2. Confirmed product decisions

- Ame is implemented independently. It is not a fork, derivative, or adapter around Lap.
- Mature algorithm and infrastructure libraries should be integrated behind Ame-owned ports after
  license, adoption, maintenance, Windows, quality, and replaceability evaluation.
- Lap is an external reference for product behavior, implementation study, performance comparison,
  and failure cases. Its GPL source, components, assets, schema, and internal types must not enter
  Ame's Git history.
- The application has one unified library canvas. Source, time, search, sort, and exact-duplicate
  state are scopes or display conditions, not peer tabs or nested pages.
- The gallery has no visible pagination. The UI uses continuous lazy rendering while the backend
  uses bounded cursor windows.
- The right scrollbar is also a year/month timeline navigator.
- The left sidebar owns navigation and source scope only. Duplicate review is not a sidebar entry.
- Gallery operations live in the upper-right contextual action area.
- Selecting one or more items replaces browsing actions with selection-specific actions.
- Exact duplicate display is distinct from perceptual and semantic similarity.
- Exact duplicate understanding precedes model classification so byte-identical content can reuse
  compatible analysis work rather than being processed repeatedly.
- Common-type classification follows durable human-decision and review-session foundations. Model
  quality, confidence calibration, and sampling evidence must be evaluated against the real target
  library before high-confidence predictions can reduce the review queue.
- Classification result groups are derived smart-album projections, not ordinary user albums.
  Users cannot create, delete, rename, or directly edit smart-album membership, but they can correct
  an asset's effective category through a durable `UserOverride`; the projection then updates from
  that effective result.
- A high-confidence prediction may be displayed by default or selected for sampling, but it is not
  recorded as a user-confirmed decision. Model evidence, user intent, the effective category, and
  review status remain distinct.
- Review is a resumable productivity workflow with a stable query, cursor, progress, keyboard
  actions, undo, and cross-restart continuation. It is not a temporary selection mode or a generic
  background task entry.
- User albums and Favorites are virtual, many-to-many organization. They never imply one physical
  destination and never move or copy source files.
- Person recognition, face clustering, identity naming, pixel editing, RAW development, cloud sync,
  and a Lightroom-style editor are outside the product scope.
- Delete, move, copy, rename, quarantine, and recycle-bin execution remain unavailable until R9 and
  require a freshly authorized plan, current-state revalidation, an operation journal, and explicit
  partial-failure and recovery behavior.

### 2.1 Stable workbench concepts

The product contract keeps the following concepts distinct. A later ADR may refine storage or API shape but
must preserve their authority boundaries:

- `LibraryRoot`: one configured source and its availability, scan, and synchronization policy.
- `Asset`: one stable logical visual item independent of an absolute path. It is not defined as a
  content-hash group and may survive an identity-proven rename, move, or compatible in-place edit.
- `AssetLocation`: one physical file instance belonging to a root.
- `ContentFingerprint`: versioned evidence of exact byte identity for one compatible source state.
- `ExactDuplicateGroup`: the current projection of assets or locations proven byte-identical by
  compatible fingerprints. Folding this group in the gallery never merges durable identities or
  authorizes removal of a physical copy.
- `SimilarityGroup`: a reviewable candidate relationship between visually similar but non-identical
  assets. It is never presented as exact duplicate evidence.
- `ModelPrediction`: immutable engine output with model, version, parameters, confidence, evidence,
  and analysis-run identity.
- `UserOverride`: durable human intent that survives reanalysis and is never overwritten by a newer
  model run.
- `EffectiveCategory`: the current category projection resolved from compatible model evidence and
  any applicable user override.
- `ReviewStatus`: whether a result is unreviewed, selected for sampling, confirmed, deferred, or
  otherwise requires attention; it is independent of prediction confidence.
- `Album` and `AlbumMembership`: user-owned virtual many-to-many organization, including the
  built-in Favorites group, without source-file mutation.
- `ReviewSession`: a persistent review query, position, progress, shortcuts, decisions, and undo
  boundary that can be resumed after restart.
- `OperationPlan`: an immutable dry-run proposal describing intended filesystem actions, expected
  source state, conflicts, targets, reasons, and estimated impact. It does not authorize execution.
- `OperationJournal`: durable execution and recovery evidence created only by the authorized R9
  workflow.

## 3. Reference application policy: Lap

Local reference repository: an optional machine-local checkout outside Ame's repository and Git
history. Its exact filesystem path is not repository data.

Verified reference revision:

`ff8b144f628cb02d9b4ac0a7bd20d93a224810ab`

Allowed reference uses:

- compare public information architecture, gallery behavior, timeline, search, and task feedback;
- study how a mature photo application divides modules and packages dependencies;
- identify implementation risks and construct independent test cases;
- benchmark comparable user workflows on the same machine;
- use observed failures as negative acceptance criteria for Ame.

Prohibited uses:

- copying or adapting Lap source, Vue components, CSS, icons, assets, SQL schema, or internal types;
- linking Lap crates or bundling Lap into Ame;
- importing Lap commits or files into Ame's history;
- presenting visual imitation as product design evidence without validating Ame's own user workflow.

Historical reference failures are retained in the [foundation evidence](../acceptance/gallery-foundation.md#historical-reference-observations).

## Engine admission

An engine does not become default because another application uses it or its README lists a feature.
Each candidate requires license review, adoption and maintenance evidence, Windows integration cost,
fixed-corpus quality, cold and warm performance, failure isolation, cancellation behavior, cache
impact, Chinese and long-path behavior, and a replacement contract test.

Rejected and experimental engines remain documented with evidence. Ame-owned native implementations
may serve as benchmarks or fallbacks but are not automatically preferred over mature libraries.

## Product exclusions and anti-drift constraints

- Do not start a later stage to avoid finishing the active stage's difficult acceptance criteria.
- Do not treat a static UI, mock data, compilation, or a screenshot as a completed vertical slice.
- Do not add a control to the production shell before its use case is connected; confirmed controls
  may be exercised with deterministic fixtures only in the explicit R2a prototype surface.
- Do not add a navigation entry for an unavailable future feature.
- Do not place duplicate review in the sidebar.
- Do not add a standalone duplicate toolbar action; exact duplicate display and review belong to
  the gallery filter menu.
- Do not add video media filters until video indexing is accepted and connected end to end.
- Do not expose internal storage, task, database, adapter, or analysis vocabulary in ordinary
  settings.
- Do not mix English placeholder text into the initial Simplified Chinese UI or introduce a language
  selector before formal i18n scope is accepted.
- Do not turn classification, similarity, or search into separate competing gallery applications.
- Do not expose classification, category filters, smart albums, or model placeholders before R5.
- Do not represent classification as an ordinary filter or editable album membership. R5 smart
  albums are derived `EffectiveCategory` projections over current trustworthy catalog, compatible
  model evidence, and durable user overrides.
- Do not let users create, delete, rename, add to, or remove from a smart album result group.
- Do not interpret that membership rule as a ban on category correction. Users correct category
  authority through `UserOverride`, and the derived projection must follow the effective result.
- Do not record a high-confidence model prediction as user-confirmed or completed review.
- Do not erase, replace, or silently reinterpret a durable user decision when a model is rerun.
- Do not key smart-album membership by path or allow rename, move, edit, replacement, deletion, or
  root unavailability to leave stale visible results.
- Do not turn review sessions into a dashboard, AI center, task center, or second gallery.
- Do not define `Asset` as an exact hash group or let one fingerprint merge durable identities.
- Do not call perceptual or semantic similarity an exact duplicate.
- Do not turn internal scan, preview, hash, or analysis jobs into a permanent Task navigation entry.
- Do not display a chronological time rail or date headings while the active sort is by name.
- Do not sort only the currently loaded Flutter window; every sort and direction requires a bounded
  complete-result query contract.
- Do not allow a third-party engine to redefine Ame's domain or database.
- Do not confuse R1 explicit-rescan reconciliation with R2c automatic detection and catch-up.
- Do not start R3 until R2c can prove that the catalog does not silently remain stale.
- Do not treat filesystem notifications or staged inventory rows as authoritative file or asset
  state.
- Do not respond to ordinary source changes by repeatedly scanning every configured root.
- Do not mutate source or target media during R8, and do not execute an R9 action without fresh
  authorization bound to the exact immutable plan and current-state revalidation.
- Do not silently change the UI framework, bridge, database, taxonomy, or reference policy.
- After context compaction, query recent task history and verify live files before resuming.
