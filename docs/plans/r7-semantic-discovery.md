# R7: Semantic discovery and composed gallery filters

Status: execution plan; activation and slice disposition belong to the
[roadmap](../roadmap.md#r7-implementation-sequence).

## Outcome and boundaries

Find images through a natural-language description or another library image, combined with explicit
catalog conditions, inside the existing gallery and viewer. Preserve source media, user decisions,
literal filename/path search, duplicate display, shared scrolling and viewer continuity.

Use the admitted SigLIP 2 package and existing default-enabled GPU image runtime. Text encoding
initially retains the CPU runtime. Do not add model selection, download another model, restore manual
classification review, or turn semantic neighbors into perceptual duplicate groups. R8 file-operation
planning and physical changes remain outside this stage.

The [R7 scope](../product/organization-workflows.md#r7---semantic-discovery-and-advanced-search),
[gallery interaction contract](../product/gallery-ui.md#design-principles-and-rationale),
[query filters](../architecture/0033-structured-gallery-filters.md),
[feature compatibility](../architecture/0034-versioned-classification-contracts.md#features-and-matching)
and [GPU execution](../architecture/0039-gpu-classification-execution.md) constrain implementation.
Replace the superseded desktop-filter withdrawal in its ADR when that surface is reconnected.

## Planned interaction

- Extend the existing search bar with `画面内容` and `文件名和路径` modes. Content search is the initial
  default once available; save the last selected mode. Preserve the existing literal path predicate
  and punctuation handling in filename/path mode.
- Search the current folder, album or smart-album scope, and offer an explicit `整个图库` scope.
  Show the effective scope. Clearing the query returns to that scope's ordinary gallery.
- Debounce committed input and support immediate Enter submission. IME composition does not submit.
  A superseding request cancels or retires old work; only the current request can publish or clear
  its loading state. Keep usable results visible during replacement.
- Semantic results use relevance order and the shared gallery, selection, preview and filmstrip.
  Hide date-position navigation when order is relevance-based. Returning from the viewer retains
  the searched image and gallery position.
- Add `查找相关图片` to an existing library-image action menu. A library image is the reference;
  exclude that reference from its own results and retain existing exact-duplicate display policy.
  Do not add another navigation destination or another image-viewer implementation.
- Keep the filter menu's descendant and duplicate-display shortcuts. `更多筛选` opens a compact
  desktop dialog composed from existing Material controls and application theme tokens.
- Group conditions by date, file properties and image dimensions; keep camera text secondary.
  Use the retained date-field, extension, byte-range, width/height-range and camera contracts.
  Unset fields add no restriction. Conditions compose with each other and either search mode.
- Edit a draft and publish it once with `应用`. Cancel or dismiss preserves the applied conditions.
  `清除筛选` clears metadata restrictions while preserving the selected folder and duplicate display.
  Show concise applied-condition summaries above results with individual removal actions.
- Use the existing feedback owners for preparation, partial coverage, failure and retry. A search
  with incomplete indexing remains usable and identifies the affected coverage. Background index
  growth cannot reorder an active result list; expose an intentional update action when needed.

## Implementation slices

### R7-01: Composed filters and search interaction foundations

Reconnect the retained typed metadata filters through the existing query path. A presentation-owned
draft editor owns parsing, field errors, commit/cancel and summary labels; the page only connects
the result to the existing query transition. Reuse framework dialog, menu, text, date and chip controls.
Do not extend the large screen with a second form or query state machine.

Prove combined predicates, invalid input, cancellation, clearing/removing conditions, current-scope
composition, viewer return and stable browsing position. Keep unavailable semantic actions absent
until their use cases are connected. This slice is independently deliverable without a model run.

### R7-02: Reusable image features and durable semantic coverage

Extract only the common feature identity, encoder and storage responsibilities needed by two real
consumers. Classification retains category policy; discovery owns indexing and coverage. Reuse
compatible cached image vectors before encoding, and publish completed features to discovery without
waiting for taxonomy work or a whole-library analysis pass.

Create a separate rebuildable semantic index outside the catalog/source trees. The current 256-MiB
classification LRU cache is not the coverage authority. Keep exact content identity, image pipeline,
embedding space, dimensions, asset/location mappings and publication generation explicit. Existing
CPU and GPU pipeline distinctions remain intact.

Index work is bounded, resumable and idempotent. New/changed content is queued; rename updates only
mapping when content evidence is current. Retired locations stop appearing immediately. Local-only
source admission and fresh identity checks apply before vector publication; placeholders are skipped
without hydration. Share foreground admission and worker ownership rather than starting competing
unbounded analysis loops. Missing/corrupt index data is rebuildable without losing organization data.

Evaluate USearch behind an Ame-owned port against exact cosine retrieval before selecting it.
Record the exact version, license/distribution implications, Windows build, filtered recall,
read/write lifecycle, cancellation, memory and disk costs in an ADR. An index needs bounded resident
memory and explicit disk-budget behavior; no silent eviction may present incomplete coverage as full.
Keep original vector precision until lower-precision storage has equivalent retrieval evidence.

### R7-03: Text retrieval and stable gallery result sessions

Connect text encoding, scope-aware retrieval and relevance ordering end to end. Reuse the text worker
for a bounded search session and release it after inactivity or shutdown; measure cold initialization
separately from warm queries. Interactive work has priority over background indexing at existing
admission boundaries. Existing GPU preference and runtime-failure behavior remain authoritative.

Retain the supervised serial model scheduler for this stage. The
[paired-worker evaluation](../acceptance/r7-semantic-discovery.md#parallel-worker-evaluation--2026-11-07)
found similar per-item encoding speed with additional memory use; parallel execution mainly preserves
background progress during a search session. Concurrent image/text workers and automatic hardware
tiers are deferred. Preserve completed features and queued work when foreground search takes priority;
resume background work after the bounded search session. A superseded search cannot cancel a newer
request or clear its results. Revisit concurrency if actual desktop search waiting or repeated model
replacement becomes a measured user-visible bottleneck.

Apply catalog restrictions to candidate eligibility during retrieval. Do not narrow only an already
truncated global top-K. Fix a bounded result manifest to the query identity, catalog/index generation,
model contract and stable tie-break order. Gallery pages, counts, full-result selection and viewer
windows consume that same manifest. Declare the result limit and incomplete coverage explicitly.

Add typed native API and Dart port/adapter mappings without exposing engine vectors or identifiers
to presentation. Persist search mode through the existing preference boundary. Content queries must
not be restored as literal path queries after restart. Model absence, invalid/overlong input, empty
scope, index failure, supersession and shutdown have explicit outcomes and recovery paths.

### R7-04: Related-image discovery and composed workflow

Use a selected library image's compatible feature as the query. Reuse the same scoped retrieval,
result manifest, filters, gallery and viewer from R7-03. Missing reference evidence uses admitted
bounded preparation or a clear unavailable result. Reference identity is revalidated before publishing.
Changing reference retires prior work, and leaving discovery restores its originating scope.

### R7-05: Quality, desktop continuity and delivery

Recheck the retained 368-preview/24-bilingual-query evaluation with the admitted runtime and actual
index, including reference ranks, unrelated descriptions and restrictive filters. Fixed-target ranks
describe that corpus only; cosine values are not percentages of confidence. Compare approximate
retrieval with exact ranking so index loss is distinguishable from model quality.

Measure query latency, cold start, cancellation, incremental indexing, disk growth and resident memory
on local-primary with the existing source-safe execution owner. Report actual library size and indexed
coverage. Existing small-corpus GPU encoding measurements do not establish whole-library search latency.
Constrained-client optimization remains deferred; resource bounds and usable foreground browsing remain
required on the supported desktop.

Verify the running desktop's modes, combined filters, partial/empty/error feedback, rapid typing,
IME, keyboard submission, viewer/filmstrip navigation, reference search and restart continuation.
Inspect owned scope, run the relevant local diagnosis/formatting, commit and push each reversible
outcome, and inspect existing exact-commit CI. PR creation, merge and release retain their separate
authorization boundaries; this plan grants none.

## Ownership and completion evidence

Domain owns query and feature identities; application owns coverage, retrieval, scheduling and
result-session lifetimes; ports expose narrow encoding/index contracts; adapters own engine calls,
database/index formats and bridge conversion. Follow [ADR 0037](../architecture/0037-feature-module-dependency-boundaries.md)
and [ADR 0025](../architecture/0025-invariant-owned-workflow-modules.md) before extending a mixed owner.

Catalog schema or saved-scope changes use forward migrations and preserve existing archives and
decisions. New sidecar storage uses independent identity/schema admission and atomic publication.
Index generations may be discarded/rebuilt; this must not remove source files or durable intent.

The roadmap records slice disposition. Accepted technical decisions belong in their ADR, and scoped
verification, measurements, failures and remaining limits belong in the R7 acceptance record.
Keep temporary models, copied media, catalogs, raw traces and exploratory scripts outside Git.
