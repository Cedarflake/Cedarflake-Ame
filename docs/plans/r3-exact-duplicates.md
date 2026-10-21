# R3 exact duplicate implementation

Status: Completed on 2026-10-17; retained delivery record under the
[roadmap](../roadmap.md#current-stage-and-acceptance-boundary).

Updated: 2026-10-18.

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

R3-01 through R3-05 are complete. PR #21 was merged as `755db99c33cfab28b158b1863b619b43844ee56a`;
its local and remote feature branches were removed. The reviewed implementation head
`3392548c572d6c24115a676e03e09bd058f042cd` passed
[Actions run 38981500571](https://github.com/Cedarflake/Cedarflake-Ame/actions/runs/38981500571):
all ten execution jobs and the Windows aggregate gate succeeded. Three signing-only release jobs
were inapplicable to the daily gate and skipped by its existing conditions.

The [verification record](../acceptance/r3-exact-duplicates.md#r3-closeout) retains the full client,
regression, storage-bound and source-preservation evidence. Final review corrected the timeline
portal parent and open-viewer member publication boundaries. No remaining blocking defect was
identified in the reviewed PR changes. The ordinary Debug client uses the final implementation;
the test session ended normally and the original profile/catalog hashes were restored and verified.
The reusable `build/test` corpus, retained catalog, evidence and restoration backups remain available.

| Delivery | Retained result |
| --- | --- |
| R3-01 | Guarded exact identification, cancellation/reuse and physical-location inspection |
| R3-02 | Incremental analysis, foreground priority, pause/resume and unchanged restart reuse |
| R3-03 | Scoped all-images/merged modes, atomic membership, counts, badges and stable browsing |
| R3-04 | Bounded group/member review and durable preferred-copy/ignore choices |
| R3-05 | Affected client checks, complete hosted gate, resource bounds and final review |

The sections below retain the delivered scope and execution requirements. Future changes preserve
these boundaries and rerun affected checks; this record does not schedule another R3 closeout cycle.

## Startup-update maintenance

The roadmap retains stage closure and schedules this scoped repair on `main`. The retained
10,000-image test catalog reached 72 merged items only after startup recovery and subsequent exact
analysis. Keep the three observed failures distinct while repairing their owning boundaries:

1. Restore new time-rail navigation while replacement geometry is missing or estimated; retain
   stable painted ticks, current date authority and retirement of old pointers. The subsequent
   report of the position marker briefly returning to the top remains in this boundary.
2. Let bounded exact analysis and complete-result publication progress during automatic recovery,
   preserving foreground source-read priority, cancellation and guarded fingerprint admission.
3. Reconcile synchronization, pending analysis and gallery publication so completion feedback does
   not precede the work that still changes visible membership; inspect delayed date evidence too.

The later reports add marker jumps and longer synchronization. The marker repair retains the
previous position across provisional windows. One replay of the pre-refactor baseline also showed
directory completion before fingerprint publication and left invalidated metadata for later
browsing. The user then prioritized resolving the observed behavior over further attribution.
Keep those observations, complete the focused repair and do not extend the comparison campaign.

The first repairs and their affected client checks are complete: new rail input remains available during
updates, bounded exact results publish progressively, metadata recovery finishes in the background,
and completion waits for settled gallery results. The client check also identified and repaired
external Slider padding that excluded endpoint clicks. The original profile and retained catalog
were restored and hash-verified. The two hosted fixture failures were reconciled with the current
schema and metadata-read behavior; all applicable jobs passed on `cab95a6`. The acceptance record
retains the commands, results and scoped comparison limits.

The subsequent request preserves existing folded groups during recovery while newly confirmed copies
join progressively. Schema 45 retains compatible display relationships separately from current
fingerprints; current hashing, changed-source invalidation and preference admission stay strict.
The existing corpus retained 72 folded groups through recovery, pause and resume, then completed
10,000 current fingerprints and metadata records. A follow-up repair lets bounded badge inspection
read newer published membership without waiting for the visible gallery revision to catch up.
Its client check now covers visible badge actions and shared group status during recovery; the
annotation plan reuses content/source indexes rather than repeatedly scanning equal-size peers.
Implementation and client verification are complete; exact-head hosted verification remains active.
The journal investigation confirmed
an expired checkpoint and added observed-bound diagnostics without changing recovery policy.

Use focused boundary regressions and the existing isolated client corpus, then inspect normal
push-triggered CI. Record runtime evidence in the
[startup-update verification](../acceptance/r3-exact-duplicates.md#startup-update-maintenance).
Known dirty events and changed or incomplete source evidence retain generation invalidation.
Preserve unrelated workspace edits and original profile backups.

### Fast recovery after journal loss

Implement the accepted Windows recovery-evidence policy from ADR 0024 on
`codex/fast-startup-recovery`. Reuse the production handle-anchored inventory and its publication
authority, retain unchanged derived results, and reconcile added, changed, renamed and missing
files through the existing candidate path. Keep ordinary startup on continuous USN replay.

The delivery includes actual-file regression coverage for offline operations, unchanged preview
and fingerprint retention, cancellation and namespace/publication races. Measure complete recovery
on the retained library, including catalog and visible-result convergence, and report the observed
elapsed time and measurement conditions. This maintenance scope has no fixed startup-time threshold.
The mapped-write timing window is an accepted boundary, not an additional full-content verification
requirement. Source mutation and hydration remain excluded. Update the owning acceptance record
with actual results and any remaining functional limitations.

The [current verification](../acceptance/r3-exact-duplicates.md#fast-recovery-after-journal-loss)
covers ordinary offline operations, terminal handoff retirement, retained-library recovery completion
and catalog reopening. The recovery completed across interrupted runs, so its elapsed times do not
establish a single uninterrupted client startup duration.

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
