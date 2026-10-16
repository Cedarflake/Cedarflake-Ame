# R3 exact duplicate implementation

Status: R3-01 through R3-04 implementation and client checks recorded; affected Actions verification in progress under the
[roadmap](../roadmap.md#current-execution-queue).

Updated: 2026-10-17.

## Outcome and scope

Deliver the [R3 product contract](../product/organization-workflows.md#r3---exact-duplicate-understanding):
identify byte-identical images, inspect every physical location, fold confirmed groups in the
existing gallery, and preserve preferred-copy and ignore decisions across restarts and reanalysis.
The [gallery contract](../product/gallery-ui.md#filter-and-exact-duplicate-behavior) owns the filter
choices and the Duplicates workspace. [ADR 0029](../architecture/0029-functional-workbench-navigation.md)
owns the function rail and collapsible Library sidebar. The roadmap alone owns stage order and closure.

Reuse the frozen R2c catalog, source identities and generations, guarded source access, incremental
publication, bounded gallery queries and preview/viewer lifecycle. Source files and cloud-only
placeholders remain unchanged. Asset identities remain independent of content hashes. Analysis
evidence is rebuildable; human decisions remain durable.

Favorites, albums and general resumable review sessions remain R4. Perceptual similarity remains R6.
Physical file organization and deletion remain their separately authorized later workflows. R3
records compatible content evidence that later analysis can reuse; it does not introduce those
analysis engines or convert reused results into human decisions.

## Execution checkpoint

Preparation starts from product commit `5ea62d05eb7f84a64a12967fdba20fda9008f371`.
The `codex/r3-exact-duplicates` implementation adds guarded full-content analysis,
versioned fingerprints, schema 40/41, bounded group/location queries and a generated desktop bridge.
[ADR 0028](../architecture/0028-versioned-exact-content-analysis.md) records the BLAKE3 adapter
choice and identity rules. Automatic incremental analysis and both gallery display modes are
connected, including atomic gallery membership publication.

The retained 10,000-file corpus produced 72 groups matching its existing digest oracle, with
cancellation and continuation exercised in the client. The
[R3 verification record](../acceptance/r3-exact-duplicates.md) retains the exact coverage and limits.
The accepted three-region workbench replaces the More-menu/bottom-sheet prototype under ADR 0029.
The function rail, persistent collapsible folder sidebar and duplicate-results workspace with
location reveal are connected. Automatic analysis now follows catalog publication and yields to
foreground browsing and media reads; pause persists across restart. Merged gallery mode includes
duplicate-location icons without counts; all-images mode leaves cards unmarked. The selected mode
is restored after restart. Retain the existing analysis implementation and verification;
the changed client paths have been checked against the reusable corpus, including unchanged
reopening with zero additional content reads. The Settings-menu accessibility failure was reproduced
with the retained gallery's timeline slider. Its always-mounted value-indicator portal now retires
when the gallery is hidden; the focused regression and native menu-close/return-position check pass.
The hosted native, scan, release and synthetic checks passed on `515f5e6`. Its two analyzer findings
and three gallery regressions are corrected: rows retain exact item ordinals, and dimension recovery
checks rendered card widths. The content-sized card controls also passed the isolated client check.
The card badge opens a centered location dialog with fully wrapping file/folder lines and a current
image marker. This replaces the anchored menu after long paths and controls were clipped in the
client. Narrow-window/text-scaling checks and native paging, scrolling and dismissal checks pass.
Group review connects visible image previews, the existing viewer and durable preferred-copy/ignore choices.
Native checks reused the 10,000-file corpus, including preference/ignore retention after restart.
The current head still requires its applicable Actions results.

The current UI revision retains responsive group cards and compact single-column member rows,
with vertically centered preference/reveal icons on the right. Header actions align right, count
space remains stable, and empty feedback reuses the gallery component. Continuous virtual scrolling
replaces review pagination while bounded catalog windows and preview demand remain separate.
Preference writes and their subsequent catalog publication preserve rendered members until a
complete replacement range is ready. The 22 focused controller/window/layout/preference cases pass;
the two viewer boundary cases also passed before the background-revalidation addition.
The rebuilt Debug reuses the retained 10,000-file catalog; native inspection confirms group cards,
112-pixel member rows and preference switching without clearing the visible list. Applicable checks
for the submitted head remain owned by Actions. Preserve the active client/profile backup until
the visual-review session is closed.

The new crash during window restore/sidebar changes was traced to the duplicate member viewer's
initial offstage frame. Its route now owns the slider overlay lifetime; the previously failing
retained-tree regression and 13 affected cases pass. The normal Debug client completes member
preview/information, gallery return, maximize/restore and repeated sidebar interaction with no
native accessibility errors. This is a scoped crash repair; current-head hosted verification and
the remaining requested UI corrections retain their separate status.

| Order | Delivery | Current state |
| --- | --- | --- |
| R3-01 | Identify exact duplicates and inspect their physical locations | Committed in PR #21; client and native Settings-menu restoration checks recorded; applicable Actions results pending |
| R3-02 | Maintain duplicate results through incremental changes | Committed; automatic scheduling, foreground priority, unchanged reuse and closed-process additions/removals checked; native pause/resume and paused restart now checked; remaining automated results belong to Actions |
| R3-03 | Connect both gallery display modes and duplicate badges | Implemented; 10,000 all-images / 72 merged entries, location reveal, restart preference and automatic 74-to-72 folding checked in the reused client corpus; new regressions await Actions |
| R3-04 | Review groups and preserve preferred-copy/ignore decisions | Implemented with schema 42, continuous bounded review windows, source-checked preference writes and reversible ignore; affected client checks recorded; automated checks belong to Actions |
| R3-05 | Complete affected client/performance verification and close R3 | Inspect current-head Actions and finish only the affected remaining checks; retain prior analysis and source-preservation results |

Each delivery includes its implementation, connected interface, relevant regressions and necessary
documentation. Commit and push it before beginning the next independent delivery. Update this
checkpoint with results; keep changing progress, commit IDs and measurements out of the goal text.

## R3-01: Identify duplicates and inspect locations

### Engine decision within the first delivery

Compare `czkawka_core` integration with a narrow fingerprint adapter using the existing `blake3`
dependency. The reviewed upstream reference is Czkawka 12.0.2. Its
[core manifest](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/Cargo.toml)
declares MIT and includes BLAKE3 alongside additional image, audio, video and archive dependencies;
the [Krokiet application](https://github.com/qarmin/czkawka/blob/12.0.2/krokiet/Cargo.toml)
has its separate GPL-3.0-only declaration. Evaluate the core independently of the GUI executable.

The existing [duplicate pipeline](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/src/tools/duplicate/core.rs)
provides size grouping, partial-content filtering, full hashing, progress and cancellation. Its
[hashing entry point](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/src/tools/duplicate/mod.rs#L276)
opens a path itself, and its [cache](https://github.com/qarmin/czkawka/blob/12.0.2/czkawka_core/src/common/cache.rs#L133)
checks path, size and modification time. Any integration must retain Ame's guarded read ownership,
source-generation validation, bounded catalog candidates and durable analysis evidence.

Select the option that minimizes the complete integration and maintenance cost while meeting the
same contract. Evaluate supported APIs, dependency/license closure, Windows packaging, cancellation,
read/resource bounds and fixed-fixture correctness. Use a focused prototype only for an unanswered
integration question; do not build both complete product implementations. Record the selected
version, alternatives, adaptation needs and replacement boundary in an ADR before introducing the
dependency or persistence contract. The current source review is preparation, not a performance result.

### Rules to settle before schema and query changes

- Define fingerprint scheme, engine/version/parameters, immutable run identity and compatibility
  with the existing source identity, revision and generation. Partial-content hashes only prune
  candidates; a confirmed exact group requires compatible complete-content evidence.
- Define deterministic representative selection, physical-location counts, and hardlink treatment.
  Every real location remains inspectable even when multiple paths share one physical file identity.
- Define how global duplicate evidence composes with source/folder/date/search scope, including
  groups whose members fall outside the current query. Specify count and representative behavior
  before connecting paging and selection; filtering must not silently widen the selected scope.
- Define how preferred-copy and ignore decisions will survive regrouping without treating the
  current group hash as permanent asset identity. R3-04 implements their user workflow.

### Connected implementation and exit

Read candidates from the existing catalog in bounded windows, group by size and prune unnecessary
reads. Read admitted source content through the owning filesystem port; publish fingerprint and
group evidence only when the source state still matches. Use forward migrations that preserve
existing catalogs and user data. Analysis work has its own progress, cancellation, retry and
restart-safe result reuse, without blocking gallery startup or browsing.

Expose `查看重复位置` for a confirmed group in the dedicated Duplicates workspace, with each path
and its local, offline, unavailable or cloud-backed state. Result validity participates in existing
source-generation invalidation from the first publication: a changed member cannot keep presenting
an obsolete exact match while automatic recomputation is still pending. R3-02 completes that
automatic scheduling and convergence. Default folding remains off until R3-03.

Exit through a real duplicate group whose locations can be inspected in Ame, stable results after
restart, cancellation/retry and stale-result rejection, and the applicable adapter/migration tests.
Both unchanged originals and cloud-only placeholders retain their source state.

## R3-02: Incremental maintenance

Connect analysis scheduling and group invalidation to the existing R2c publication boundary. Cover
addition, in-place modification, same-path replacement, rename/move, location removal, hardlink alias
changes and unavailable roots. Preserve compatible unchanged fingerprints and recompute only
affected candidates. Absence and unavailability remain distinct. A cancelled or older run cannot
publish into a newer source generation or erase a newer analysis result.

Exit through normal changes and closed-process changes followed by reopening: groups converge,
unchanged evidence is reused, and neither startup nor incremental analysis adds a whole-root scan.
Test reading races and rollback at the publication boundary. Keep source reads for explicitly
scheduled analysis separate from the unchanged R2c synchronization startup counters.

## R3-03: Gallery folding

Connect `全部图片` and `合并重复图片` in the existing gallery filter menu. Duplicate-only results
remain in the Duplicates workspace. The
initial default becomes folding once reliable exact evidence and incremental maintenance are
connected. Preserve the user's selected mode. Pending or unreadable items remain visible without
an invented duplicate classification.

Publish representative items and duplicate-location actions through bounded queries. Keep sorting, result
counts, date distribution, pagination, complete-query selection, viewer navigation and return
anchors consistent with the selected mode and scope. Logical group selection and explicit physical
location selection have distinct typed meanings. Preserve the delivered gallery/viewer layout and
interaction outside the added R3 controls.

Exit through both gallery modes and duplicate-only results in the actual client, including groups spanning page boundaries,
source/date scopes and incremental changes, with no missing item, duplicate page, jump or unbounded
bridge payload. Apply the Microsoft and Google UI/UX skills to the affected controls and copy.

## R3-04: Group review and durable choices

Connect group review in the Duplicates workspace, retaining its selected group and progress while
the user visits Library or Settings.
Allow the user to inspect members, select a preferred copy and ignore a group. Implement the
decision identity and regrouping rules settled in R3-01; persist human choices separately from
rebuildable analysis and honor them after restart, compatible reanalysis and catalog maintenance.
These choices do not delete, move or replace files.

Exit through a complete group-review workflow, restoration of the user's choices and regressions
for changed membership, removed preferred locations and stale analysis. Limit durable-decision
infrastructure to these R3 needs; the general album/review-session workflows remain R4.

## R3-05: Affected verification and closure

Add boundary regressions with each preceding delivery and let the existing Actions workflows run
the applicable automated tests, static checks and builds. Inspect failures for the pushed commit
and correct their cause. Local work covers necessary diagnosis, formatting/diff inspection and
actual affected client workflows; do not replay local CI or the historical R2c acceptance matrix.

Reuse `build/test` and its varied image dimensions, dates, encoded sizes and non-uniform content.
Add only missing controlled cases for exact copies, same-size different content, same pixels with
different bytes, hardlinks, damaged/wrong-extension files, unavailable sources and read-time changes.
For known real roots use `local-primary`/`cloud-primary`, bounded read scopes and isolated derived
storage. Never hydrate placeholders or change source media or the original catalog for a test.

Measure first-analysis work, unchanged reopening, incremental changes, cancellation latency and
browsing under analysis load. Preserve bounded memory, disk reads, database windows and bridge
payloads. Reuse still-valid results and rerun only affected boundaries. After process retirement and
profile restoration, clean disposable outputs while retaining the reusable corpus, original data
and unresolved restoration backups.

Close R3 when all four user workflows are connected, incremental/restart behavior is correct,
choices persist, applicable CI and client checks pass, and source preservation and resource bounds
are verified. Record remaining unrelated maintenance separately; stage closure belongs in the roadmap.
