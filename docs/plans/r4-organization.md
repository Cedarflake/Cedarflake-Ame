# R4 - Virtual organization implementation plan

Status: implementation reference; current delivery and acceptance status are owned by the roadmap.

The [roadmap](../roadmap.md#r4-implementation-sequence) selects this sequence and owns stage status,
priority, blockers and closure. This plan defines the work and exit conditions for each slice.
[R4 requirements](../product/r4-organization.md) own behavior;
[the gallery contract](../product/gallery-ui.md#review-workflow) owns the accepted desktop controls.
Implementation was delivered on `codex/r4-organization`. Completing an internal slice does not
expose or complete the later desktop workflow.

## Outcome and scope

The user can assign an entire logical image group to one flat album, keep Favorites independently,
and resume unfinished organization in the existing gallery and viewer. Ordinary browsing and
organization mode share durable decisions and counts. Organization inherits gallery sorting and
offers `新加入优先`; sort changes retain the current image and logical continuation position.

Implement the data and group-lifecycle rules before exposing album writes. Build bounded queries
and continuation next, then connect ordinary album/Favorite use and organization mode. Finish portable
recovery and manual cleanup before closing R4. Migration backup and source-removal safety belong
to the first affected slices; they are not deferred to the final recovery screen.

R4 changes Ame-owned records only. Preserve original media, cloud-placeholder behavior and the
delivered R2c/R3 browsing and synchronization boundaries. Classification and `智能相册` follow the
[R5 plan](r5-classification.md); perceptual similarity and physical file operations retain their
later stages. No separate review workspace, mobile layout, new startup-wide hash pass or second
source copy is part of this plan.

R4's [R5 boundary](../product/r4-organization.md#boundary-with-r5-classification) distinguishes manual
album membership and organization progress from category results and review. Reuse this completed
foundation without adding classification decisions to R4's state fields or reopening its delivery.

## Original starting points

At plan creation, the inspected source provided exact-group evidence, bounded group/member queries,
representative selection and durable R3 preferred-copy/ignore choices. These were the reuse
foundation; album membership, organization progress and resumable organization sessions were
the planned additions.

| Existing owner | Reuse and boundary |
| --- | --- |
| [Exact-content ADR](../architecture/0028-versioned-exact-content-analysis.md#group-and-query-semantics) and [duplicate review domain](../../rust/src/domain/exact_duplicate_review.rs) | Reuse compatible evidence and current representatives. A digest identifies evidence, not a permanent asset or organization identity. |
| [Duplicate review application](../../rust/src/application/exact_duplicate_review.rs) and [repository port](../../rust/src/ports/exact_duplicate_review.rs) | Follow typed use cases and bounded reads. Give R4 its own organization owner rather than extending the R3 ignore flag. |
| [SQLite migrations](../../rust/src/adapters/sqlite_catalog/migrations.rs) and [R3 decision migration](../../rust/src/adapters/sqlite_catalog/migrations/exact_duplicate_review.rs) | Preserve existing catalogs and durable choices; add forward migrations and verify historical upgrades. |
| [Gallery query port](../../rust/src/ports/gallery_query.rs) and [Flutter catalog contract](../../lib/features/library/application/library_catalog.dart) | Extend query scope consistently across pages, counts, layout, selection and viewer navigation. |
| [Library controller](../../lib/features/library/application/library_controller.dart) and [duplicate review controller](../../lib/features/library/application/library_duplicate_review_controller.dart) | Reuse publication and lifetime boundaries. Add cohesive organization/query/session owners instead of growing these controllers with unrelated state. |
| [Storage policy](../architecture/0005-storage-governance-and-budget.md) and [workspace navigation](../architecture/0029-functional-workbench-navigation.md) | Keep decisions separate from rebuildable caches and retain the existing gallery, viewer and navigation shell. |

These are inspection entrypoints, not a fixed list of files to modify. Before each slice, inspect
the live owners and choose the narrow boundary under
[ADR 0025](../architecture/0025-invariant-owned-workflow-modules.md).

## R4-01: Durable group decisions and command targets

Depends on the delivered R3 baseline. This is the internal foundation for every later write; it
does not expose unfinished album or organization controls.

Work:

1. Record the R4 storage and identity decision in an ADR before schema or public bridge changes.
   Define stable organization references, provisional singleton identity, current group evidence,
   decision revisions, conflict provenance and the relationship to existing `Asset`/`AssetLocation`
   identities. Specify how valid records survive catalog rebuilds without treating paths or a
   current digest as permanent identity.
2. Define typed application commands and errors for group-target resolution and organization
   changes. Normalize a selection to complete current groups across roots, deduplicate repeated
   selected members and resolve the current representative where a file target is needed. Preserve
   R3's scoped visible selection and physical-location inspection.
3. Establish durable records for albums, favorite intent, album/progress decisions, unresolved
   conflicts and operation revisions. Define the persistence contract for continuation and undo
   before later slices add their full workflows. Keep R3 duplicate ignore distinct.
4. Add forward migrations with recoverable pre-upgrade backup, preservation checks and a tested
   recovery procedure. The migration must retain existing catalogs, preferences and duplicate
   choices; it cannot require clearing user data or rescanning originals.
5. Establish transaction and publication rules: validate current group/content authority before
   commit, write related changes together, publish only committed results and reject stale writes.
   Keep transactions and global locks outside filesystem reads and UI waits.

Exit conditions:

- folded, unfolded and folder-filtered selections resolve to the same intended whole-group targets;
- same-path replacement, stale generations and changed representatives cannot authorize an old write;
- the first organization write survives reopen, and injected failures leave its previous committed
  state intact;
- migration, backup recovery, typed bridge and repository-boundary checks cover the new contract.

Resolve storage layout, operation receipts and compatibility details here. A newly discovered
choice that changes user-visible behavior must be settled before its dependent implementation;
it must not silently replace the already confirmed product rules.

## R4-02: Organization transitions and group reconciliation

Depends on R4-01. Implement one application-owned state transition path shared by ordinary browsing,
organization mode and later restoration.

Work:

1. Implement flat-album create/rename/delete and whole-group assign/move/remove operations, plus
   independent favorite/unfavorite commands. Enforce one confirmed album per group in persistence.
2. Commit album assignment and `已整理` together. Deliberate removal, including album deletion,
   records `已忽略`; defer and restore-to-pending retain their distinct meanings. Same-destination
   assignment is idempotent, and favorites do not change organization progress.
3. Reconcile later duplicate identification using current evidence. Compatible assignments converge;
   differing album assignments retain their provenance and produce one deferred conflict. A content
   split removes obsolete inherited membership, preserves unchanged members and gives independent
   unhandled content pending state, or adopts a compatible destination group's state.
4. Connect source lifecycle handling now: identity-proven rename and temporary unavailability retain
   applicable decisions; explicit source removal deletes its associations and orphaned organization
   records while preserving groups still represented by another configured source. Reimport must
   not resurrect deliberately removed records.
5. Implement version-checked undo and retry. Undo affects the actual album/progress or favorite
   changes of its operation, preserves unrelated/newer decisions and rejects incompatible content.
   Repeated analysis/publication cannot replay a user action or manufacture a second decision.
6. Publish coherent group counts and changes through bounded queries/events. A save failure retains
   the last committed view and exposes a retryable result without advancing the viewer.

Exit conditions:

- cross-folder late merges, same-album merges, conflicting merges, content splits and replacement
  races preserve the required assignments and all four organization states;
- removing one group entry cannot reveal another physical copy as an album member;
- explicit root removal, surviving shared groups, temporary offline sources and reimport have
  separate verified results;
- retries, interruption and stale undo preserve atomicity and newer decisions.

## R4-03: Bounded album and Favorite queries

Depends on R4-02. Connect organization records to the existing gallery query boundary before building
the user controls. This slice delivers query contracts, not a claim that the album interface is ready.

Work:

1. Add bounded album and Favorite scopes to catalog queries, counts, timeline/layout data, selection
   and viewer navigation. Albums always fold confirmed duplicates. Keep source/folder filters and
   the current representative consistent across every result surface.
2. Reuse query snapshots, keyset/range reads and position retention across asynchronous group changes.
   A late response from an old album, source or query cannot replace the current gallery or viewer.
3. Return counts and pages from coherent catalog state without loading all memberships into Flutter
   or issuing a database query for every visible image. Include empty, deleted-album, unavailable,
   pending-evidence and conflicting-assignment cases.
4. Connect typed bridge/catalog contracts and result invalidation to committed organization changes.
   Keep UI scope, whole-group command targets and explicit physical-location inspection distinct.

Exit conditions:

- album/Favorite pages, counts, geometry and viewer navigation agree for the same query snapshot;
- late duplicate folding and representative replacement preserve position and never duplicate entries;
- invalidated queries reject stale completion, and empty/deleted scopes have explicit results;
- reads and bridge payloads remain bounded in populated fixtures, with no source scan or media read.

## R4-04: Continuous queues and durable continuation

Depends on R4-02 and the bounded query integration in R4-03. This adds queue and position ownership;
the underlying organization states already exist and ordinary-mode work must retain them.

Work:

1. Provide bounded pending, deferred and ignored queries and group counts. Share gallery sorting,
   with an organization-only `新加入优先` option using library admission and a stable tie breaker.
   Capture dates and duplicate-copy arrival must not manufacture new tasks.
2. Persist the organization scope, queue ordering and logical continuation anchor separately from
   handled-group state. Store relevant content/group authority and deterministic fallback information,
   not just a row number or scroll offset.
3. Resolve continuation after insertions, ordinary-mode edits, merges, splits and representative
   changes. Preserve the current item during background updates; if the saved item is already handled,
   continue with the next eligible item in its retained scope. Keep unavailable records while allowing
   the user to work on other items.
4. Persist committed progress/continuation effects at the operation boundary. Distinguish successful
   saves, cancelled actions and browsing-position checkpoints. A late read or save response cannot
   move a newer viewer session or overwrite its anchor.
5. Expose a typed continuation result, including no prior activity, no resumable position, unavailable
   anchor and empty scope. Reopening or exiting mode restores committed state without a startup scan.

Exit conditions:

- inherited sorting, explicit sort changes and `新加入优先` share page and continuation order;
- new arrivals and sort changes preserve the open image and logical continuation;
- newly identified copies reuse work, and counts reconcile once per group after merges/splits;
- restart and interruption restore the correct logical position and committed decisions together;
- ordinary-mode operations cannot overwrite the organization-mode anchor;
- queries and restoration remain bounded at the retained library scale.

## R4-05: Connected albums, Favorites and organization mode

Depends on R4-03 and R4-04. Connect the approved interface to the real services; the earlier browser
mockup is a layout reference, not implementation or acceptance evidence.

Work in this order within the slice:

1. Connect the primary destinations in `图库`, `相册`, `去重` order. Put flat albums in the Albums
   secondary sidebar and expose independent Favorites there only. Preserve each browsing
   destination's scope through the shared gallery and viewer. Primary-sidebar visits to Albums,
   Duplicates or Settings clear ordinary selection and reset position; an organizing Library retains
   its mode, queue, position and selection across those visits and restores them on return.
   Connect album management and the single-destination picker in ordinary selection/viewer controls.
   Include create-in-picker, mixed
   selections, move/remove, independent favorite actions, cancellation, errors and undo. A normal-mode
   save updates shared progress while retaining the current image and organization anchor.
2. Connect `加入相册前询问` and `默认加入的相册` to durable settings with the confirmed defaults. Missing
   defaults and unresolved conflicts open the picker. Favorites is never an album destination.
3. Add `整理模式` / `退出整理` to the Library overflow menu. All-source Library and selected
   source/folder views define the queue scope; Favorites and albums have no organization mode.
   Keep the gallery and viewer as the shared canvas, preserving normal-mode sort, selection and
   browsing position. Offer continuation only for a saved position matching the current source scope.
4. Put total, organized, pending and ignored counts in the existing count row. Use two rows in the
   upper-right operation area: existing browsing/selection actions above, then one dropdown with
   the current queue's icon, name and a chevron, followed by exit and continuation buttons with icons
   and labels.
   Retain the queue name and count in the trigger's tooltip and accessible name.
   The menu offers `待整理`, `稍后整理` and `已忽略` without counts, with the shared dot selection marker.
   Align the title and count with the corresponding action rows. Animate their geometry together
   on entry and exit, respecting reduced motion. Hide continuation
   until prior activity and a resumable position both exist. Retain content and control appearance
   during short query changes. Preserve bottom notification placement and reuse the right-side
   scroll rail in both modes. Date sorts keep the complete time annotations and navigation; name
   sorting supplies letter ranges. `新加入优先` marks the range first admitted after the current
   mode entry, retaining that boundary until exit.
5. Keep album actions in the contextual action row. Put `稍后整理` and ignore in More only during
   organization mode, with `取消稍后整理` and `取消忽略` when applicable. Successful manual-album
   assignment completes organization; the organization-mode viewer advances only after commit.
   Keep chronological undo and redo in that mode's More menu, including previous-group operations.
   Use the bottom album tray with a deselectable single manual-album destination; its ignore/defer
   actions also appear only in organization mode. Retain state-aware Favorite and album commands.
6. Make album conflicts persistently discoverable in `稍后整理`, with the involved albums and a
   destination choice. Coalesce background notices, retain conflicts after notice dismissal and
   clear them only after a valid resolution commits.
7. Complete keyboard/focus, undo, busy/save failure, empty queue, unavailable item and cross-restart
   interactions. Verify the two-row header transition, unchanged bottom feedback placement, supported
   desktop widths, text scaling, and reduced motion under the accepted narrow-window rules.

Exit conditions:

- album management, assignment, movement, removal, favorites, settings and undo complete through the
  real gallery/viewer and survive restart; expose their controls only with these connected paths;
- first use, ordinary-mode work, mode entry/exit and resumed work behave as the accepted contract;
- no `继续整理` appears merely because pending groups exist;
- new arrivals, background identification and conflict resolution preserve current viewing context;
- no pending UI action claims success, changes counts or advances before its transaction commits;
- the real Windows client proves these paths with populated, empty, failed and unavailable states.

### Sidebar management follow-up

Deliver saved drag ordering for albums and imported top-level folders before batch management.
Preserve the existing order on upgrade, append new items, retain order across renames and refreshes,
and verify reopen, failed saves and concurrent item creation/removal. Keep child-folder ordering and
physical directory structure outside this change. Album paging must follow the saved order.

Use one shared section heading for `文件夹` and `相册`, below the Library and Favorites destinations.
The trailing plus imports a source or creates an album. Follow with icon-initiated multi-selection:
the selected count stays at the heading's left, with neutral square Select all, removal/deletion
and plus grouped at the right. Primary-workspace switches clear secondary-sidebar selection.
Reuse existing confirmation and feedback surfaces. Source removal and album deletion retain
their distinct data effects, including surviving duplicate groups and deliberate-removal history.
Source selection, Select all and batch removal include imported top-level folders only; expanded
child folders retain ordinary browsing without selection checkboxes.
Verify cancellation, partial/stale admission, disposal and normal navigation after selection ends.
Album rows also expose their existing edit menu through secondary click and keyboard gestures.
Image-card menus use current group state for Favorite and album actions, retain existing selection,
and reject delayed actions after target replacement. Reuse the album tray and guarded commands.

## R4-06: Export, restoration and unresolved-record cleanup

Depends on R4-01 through R4-05. Migration recovery was established in R4-01; this slice adds the
complete user-facing portable recovery and cleanup workflows.

Work:

1. Define and version the export format and compatibility rules. Include album definitions,
   favorites, decisions, conflicts, organization progress and association evidence, including
   existing durable R3 choices. Exclude source media and rebuildable preview/model data.
2. Validate and stage restoration before atomic publication. Repeated restoration is idempotent;
   invalid input and failed writes preserve current data. Retain unmatched records for later valid
   association and expose unresolved conflicts without overwriting newer user decisions.
3. Connect catalog rebuild/reindex to this durable record boundary. Define and verify deletion/
   restore precedence so reimport, rebuild and automatic reassociation cannot reverse deliberate
   source removal or cleanup; applying an explicit backup remains a separate user action.
4. Add the settings export/restore controls and manual `清理未关联记录`. Show eligible counts, age and
   affected record kinds, confirm the chosen cleanup and revalidate eligibility at commit so records
   that became valid again survive. No automatic age-based purge is introduced.

Exit conditions:

- upgrade recovery, catalog rebuild, export/restore and repeated restore preserve applicable data;
- malformed/newer unsupported exports, partial failures and conflicting restored decisions remain
  actionable without destroying the current catalog;
- unmatched records stay inspectable until matched or explicitly cleaned;
- explicit source removal, later reimport, restored backups and cleanup each retain their distinct
  authority; every workflow preserves original media and placeholder state.

## R4-07: Integrated verification and stage closure

Depends on all preceding slices. Every slice carries its focused regressions and applicable client
checks; this final slice covers their integration and missing scale/failure evidence.

Work:

1. Run the connected flow on the reusable `build/test` corpus: ordinary assignment, late identification,
   conflicting albums, defer/ignore, new arrivals, continuation, content changes, source removal and
   recovery. Use controlled fixtures for mutations and retain the original library/profile.
2. Verify the affected read-only paths on the configured logical roots at the retained library scale.
   Measure album opening, queue entry, continuation, save-to-visible feedback, counts and background
   reconciliation separately; record cold/warm conditions, CPU, memory, query windows and source reads.
   Include the two-core/8-GB environment when available. A small fixture cannot establish large-library
   performance, and this plan introduces no arbitrary five-second target.
3. Exercise bounded memory/bridge payloads, cancellation, disposal, stale completions, save interruption,
   migration and restoration failure. Keep browsing responsive during catalog and duplicate updates;
   confirm no new source enumeration, full hashing or hydration is required to restore R4 state.
4. Verify the real desktop interactions under the existing UI verification map and the applicable
   hosted checks for the exact implementation commit. Record results and remaining gaps in an R4
   acceptance record when implementation evidence exists.
5. Preserve the completed SDK/renderer decision in
   [ADR 0001](../architecture/0001-flutter-rust-desktop-stack.md#windows-sdk-and-renderer): Flutter
   3.47.8 with explicit Skia. The stable-SDK comparison and real-pixel evidence are retained in the
   linked acceptance record. This reference plan does not schedule another renderer switch or
   repeat the completed comparison; a future change follows that ADR's measured replacement boundary.
6. Update the roadmap's exit decision only after the complete product contract is demonstrated.
   Preserve the frozen R2c/R3 regressions without replaying unrelated milestone acceptance.

The roadmap's accepted 2026-10-30 disposition separates the remaining cloud-primary first-use
frame optimization from R4 merge preparation. Keep its measurements and source safeguards in the
linked evidence; follow the [deferred scope](../roadmap.md#deferred-constrained-device-rendering)
instead of repeating completed integration checks. This changes the delivery boundary, not the
product's organization, recovery or original-media requirements.

## Maintenance and later-stage handoff

R4's implementation sequence is a retained reference. Follow the current roadmap for maintenance
or later-stage work; do not resume a completed slice or recreate its retired development branch.
Recover current user decisions and Git state, read the relevant ADRs, inspect the actual owners,
and apply the repository's branch/commit rules to the new scope. The
[organization ADR](../architecture/0030-durable-organization-decisions.md) owns the admitted identity,
transaction and upgrade-backup boundaries.

Keep implementation, focused regressions and necessary documentation together for each reversible
outcome. Use the [required engineering workflow](../acceptance/quality-gates.md#required-engineering-workflow),
including hosted ownership of automated gates and scoped, serial local client verification.
Do not add an extra full local CI prerequisite or treat UI shells as working capabilities.

At each handoff, retain the exact revision, completed user path, applicable checks, source-safety
evidence and next unresolved boundary. Put measurements and failures in `docs/acceptance`, technical
decisions in ADRs and current delivery status in the roadmap. Keep this document focused on how to
implement and verify the selected slices.
