# R4 - Virtual organization and resumable review

Status: retained product requirements; delivery and acceptance status belong to the roadmap.

This document owns R4 behavior and acceptance requirements. The [roadmap](../roadmap.md) selects
delivery order and implementation status; the [gallery UI contract](gallery-ui.md) owns control
placement, wording, and desktop interaction. The [workflow overview](organization-workflows.md)
retains the surrounding stages. Technical storage and bridge choices remain with their owning ADRs.

## User outcome

The user organizes logical images into flat virtual albums, keeps Favorites independently, and
continues unfinished organization from the existing gallery and image viewer. Progress records both
which image groups have been handled and where work stopped. New images join this ongoing workflow
without resetting earlier work or creating a separate review workbench.

R4 changes only Ame-owned records. It does not move, copy, rename, delete, or write original media,
hydrate cloud-only files, or require a second source copy. Classification and `智能相册` belong to
[R5](r5-classification.md); later analysis preserves the human decisions established here.

## Albums, Favorites, and duplicate groups

Albums has its own primary sidebar destination, ordered between Library and Duplicates. Its flat
album list belongs to that destination's secondary sidebar. Physical sources stay under Library;
Favorites is accessible only from Albums as an independent collection; Library lists source folders.
Both destinations reuse the existing gallery and viewer, retaining their own scope when switching.
Primary-sidebar switches to Albums, Duplicates or Settings clear ordinary selection and return the
photo wall to the top. An organizing Library retains its mode, queue, position and selection across
those visits and restores them on return.

In this document, an album means a manual, user-owned album. It contains logical image groups and
does not nest. A group has at most one confirmed manual-album assignment. Moving it replaces that
assignment for the entire group. Source folders describe physical locations and do not define
album membership or duplicate-group boundaries.

Compatible exact-duplicate evidence defines the current group across folders and library roots.
Until duplicates are identified, each image is a provisional singleton group. Albums always fold
confirmed duplicates, including when identification finishes after images were added to an album.
The view converges automatically without requiring re-addition or manual refresh.

Every organization action resolves its selection to complete current groups. Selecting a member
in an unfolded view or through a restricted folder/filter scope still targets its whole group;
selecting several members of that group does not repeat the operation. Adding, moving, or removing
an album entry applies to the group, so removing one entry cannot reveal another copy in its place.
For a mixed selection, removal changes only groups with a confirmed album; groups without an album
retain their existing progress. An unresolved conflict still requires an explicit destination.
Each group has a representative image. Group actions that need a file target resolve the current
representative rather than the incidentally selected copy. Explicit physical-location inspection
continues to show the actual locations and grants no file-mutation authority.

This defines the R4 command target, not the gallery's visible selection. Preserve
[R3's scoped browsing and physical-location rules](../architecture/0028-versioned-exact-content-analysis.md#group-and-query-semantics)
while resolving organization commands through the complete group.

`收藏夹` is an independent favorite preference, not an album. A group can be both favorited and
assigned to an album. Favoriting, unfavoriting, album moves, and organization progress have separate
effects: changing a favorite does not change the album or complete organization, and changing an
album does not remove the favorite. Favorites never appear as a destination in the album picker or
the default-album setting.

These group-level behaviors must preserve stable `Asset` and `AssetLocation` identities underneath
the projection. They do not redefine an asset as a content hash or permanently merge identities.
Representative changes and compatible regrouping must not erase durable user intent.

## Conflicts and content changes

### Album conflicts after duplicate identification

When provisional groups are confirmed identical, reconcile their album decisions as one group.
Compatible assignments remain one assignment. If the groups have different album assignments,
place the merged group in `稍后整理` and identify the reason as `相册归属冲突`.

Preserve the competing assignments for inspection and resolution, rather than publishing them as
several confirmed memberships or silently choosing a winner. Show the affected group and album
choices; choosing one destination assigns the whole group there and marks it organized. The
unresolved conflict survives restart and repeated analysis, remains discoverable after a temporary
notice disappears, and does not block unrelated groups. Cancelling the choice leaves it deferred.

### Splitting a group after a content change

A confirmed content change can detach an image from its former duplicate group. Remove the
detached image's inherited album relationship; retain the album and the unchanged members' valid
assignment and progress. This is removal of a relationship, not deletion of an album or source file.

If the changed image matches another existing exact group, resolve it through that group's current
assignment and progress. If it becomes an independent group, retain only decisions valid for its
current identity and content version. Without such an organization decision, it has no album and
enters the front of `待整理`. The former group's completion is not evidence that the changed content
has been organized.

This automatic invalidation differs from deliberately removing a group from an album: deliberate
removal records `已忽略`; unhandled changed content returns to `待整理`. A same-path replacement cannot
inherit decisions solely from its path or name. Revalidate current identity and content evidence
before applying either a saved decision or a pending user operation.

## Organization progress

Each current group has one organization state, shared by ordinary browsing and organization mode:

| State | Meaning and entry conditions |
| --- | --- |
| `待整理` | A new or newly independent group with no applicable organization decision. |
| `稍后整理` | Explicitly deferred work, or an unresolved album-assignment conflict. |
| `已整理` | A group with a successfully committed, unambiguous album assignment. |
| `已忽略` | A deliberate choice to leave a group unassigned, including removing it from its album. |

Successfully adding a group to an album marks it organized in the same operation. No extra
`标为已整理`, `保存并完成`, or `完成并下一张` step is required. Moving an organized group to another album
preserves its organized state. Removing it from its album, including by deleting that album,
records `已忽略` rather than returning old work to the pending queue.

Ignoring a group leaves it visible in the ordinary gallery. The user can restore it to pending or
assign it to an album. A newly imported group with no album starts pending, not ignored. Organization
ignore is separate from R3's duplicate-review ignore decision; neither changes the other's state,
gallery folding, or source files.

In organization mode, an already-deferred or ignored group replaces the corresponding action with
`取消稍后整理` or `取消忽略`. This returns it to pending and preserves Favorites. Gallery selection,
detailed viewing and the album tray share this state-based behavior. Selection and viewer controls
place defer and ignore in their More menu; batch ignore uses `忽略所选`. Conflicting assignments stay
in deferred work until an explicit album choice resolves them.

Ordinary browsing uses these same transitions and counts. It remains in the current gallery or
viewer after an album operation; it does not enter organization mode or advance to another image.
Explicit ignore and defer controls belong to organization mode only; ordinary browsing, albums and
Favorites omit them from both the album tray and More menu.
Cancelling a picker changes nothing. Repeated assignment to the same album is idempotent. Counts
include each group once, regardless of physical-copy count or display mode, and reconcile when
groups merge or split.

Provide undo for organization changes. Undo restores the operation's actual album/progress changes
without undoing unrelated favorites, overwriting newer decisions, or applying to incompatible
content. A failed save leaves the previous committed state available and exposes retry; only a
successful commit may advance an organization-mode viewer or publish updated completion counts.

Expose R4's chronological `撤销操作` and `重做操作` controls only in organization mode, including
operations on a previously viewed group. They reverse actual committed Favorite, album and
organization changes rather than resetting the current group to pending. Successive reversals retain compatible
field-version evidence. A new committed edit after undo starts a new branch and clears redo.
Persist the history position with the operation so restart or an uncertain response cannot apply
the same step twice. Empty history disables the corresponding action. Source removal, cleanup,
backup restoration and incompatible content retain their existing authority boundaries.

## Queue and continuation

Organization mode is available in Library and its source/folder views only. The Library entry covers
all configured sources; selecting a source or folder restricts its queues, counts and continuation
to that source/folder and the selected descendant policy. Favorites and albums remain ordinary
browsing collections. Scope filters choose which groups are presented; commands still apply to the
whole duplicate group, including its copies in other folders.

Organization is continuous work, not a frozen batch that excludes later imports. Entering the mode
inherits the ordinary gallery's sort and retains its sort controls. Pending, deferred and ignored
queues follow the selected capture time, creation time, modification time or natural-name order.
The organization-only `新加入优先` option puts newly admitted independent groups first, including
groups found during startup synchronization. This option uses library admission, not capture date.
Newly identified copies of an existing group reuse its progress instead of creating another task.

Both modes share one scroll rail selected by sorting: chronological sorts retain the original
year/month navigation, and natural-name sorting shows letter ranges. `新加入优先` highlights only
groups first admitted after entering the current organization mode. Retain this entry boundary
through sorting, queue and workspace changes; capture a new boundary after exit and re-entry.
This display boundary does not mark images handled or replace saved continuation.

Keep handled-group state separate from the saved continuation position. Persist the owning scope,
queue order, stable logical anchor, and progress; an ordinal such as "image 800" is insufficient.
New arrivals, duplicate folding, sorting, and representative changes preserve the current image and
browsing anchor. Changing the organization sort updates continuation and next-image order while
leaving the retained ordinary browsing preference intact.

`继续整理` requires previous organization activity and a resumable position in the current
source/folder scope. A saved position in a different scope is not offered here and cannot silently
change the selected folder or narrow the all-source view. Merely having pending images does not
make it appear. It returns to the saved logical position rather than jumping to
new arrivals at the queue head. If that item has already been handled, continue with the next
eligible item in the retained scope and selected sort. An unavailable item keeps its record and issue state while
the user can continue with other items; unavailability is not completion.

Album operations in ordinary mode update shared progress without replacing the saved organization
anchor with the ordinary browsing position. Exiting organization mode or closing Ame preserves
progress and continuation. Opening the saved work, not a separate dashboard or task page, resumes
the same gallery/viewer workflow.

The [review UI contract](gallery-ui.md#review-workflow) owns the overflow-menu entry, count row,
two-row upper-right operation area, continuation button, and contextual actions. R4 targets the desktop
application and does not introduce a mobile layout.

## Boundary with R5 classification

R5 adds a separate `智能相册` section below manual albums in Albums. Its results derive from category
authority and use the same gallery/viewer. They do not count as extra manual-album assignments or
appear in the album tray. Manual albums retain their creation, rename, deletion, ordering, and
membership controls; classification results follow the [R5 rules](r5-classification.md).

Classification prediction, confirmation, correction, and deferred review do not change R4's
`待整理`, `稍后整理`, `已整理`, or `已忽略` state. `待确认` describes classification review, not the
organization queue. Assigning a manual album from any supported view still completes organization;
removing that assignment still records `已忽略`. Favorite changes remain independent.

Manual albums, Favorites, and smart albums remain ordinary browsing scopes without R4 organization
mode. R5 classification actions are available in their own review context, including its shared
photo wall and viewer; this does not expose R4's ignore/defer or chronological undo/redo controls
outside organization mode. Operation-specific classification undo uses the shared feedback surface.

Classification continuation has its own scope and authority. It cannot overwrite the organizing
Library's bookmark, retained selection, or mode. R5 extends migrations, portable decision recovery,
and source-removal/cleanup behavior for its new records; existing R4 archives do not already contain
classification decisions. Preserve R4's completed behavior while verifying these new boundaries.

## Persistence, removal, and recovery

Albums, album decisions, favorites, organization states, conflict choices, review progress, and undo
boundaries are durable user data. R5 adds its own category `UserOverride` records under the same
preservation principle. Catalog indexes, previews, and model evidence remain separately rebuildable.
Save each completed operation immediately and atomically with its progress/continuation effects;
closing the application is not the save trigger.
The application owns these transactions and reconciliation policies, not presentation code.

| Event | Required treatment |
| --- | --- |
| Identity-proven rename or move, with compatible content | Preserve organization and favorite decisions; update the location. |
| Representative changes or another identical copy appears | Preserve the existing group's decisions and continuation. |
| Source is temporarily offline or inaccessible | Retain records and report availability; do not treat the source as explicitly removed. |
| User explicitly removes a library source | Remove that source's catalog associations and orphaned organization records; do not retain them for automatic resurrection on reimport. |
| Other configured sources still contain members of the same group | Remove only the departing source associations; retain the surviving group's valid decisions. |
| Catalog reindex/rebuild, preview cleanup, or reanalysis | Preserve durable user records and re-associate them only with trustworthy identity/content evidence. |
| An identity cannot yet be reliably associated | Retain an unresolved record until it is matched or explicitly cleaned; do not guess from a filename/path. |

Settings provides manual cleanup for long-unresolved records. Show the affected record count and
unassociated duration, explain the album/favorite/progress records being removed, and confirm the
selected cleanup. Revalidate eligibility when applying it so records that have become valid again
are preserved. Cleanup is separate from thumbnail cleanup, does not delete original media, and
requires an explicit user action rather than automatic age-based expiration.

Settings exposes one `查看并清理` entry. Its dialog contains the inspection range, defaulting to
records unassociated for more than 90 days, with 30-day and all-record choices. Changing the range
inspects it again before cleanup can be confirmed; an empty result keeps the range selector available.
These are manual inspection filters, not retention deadlines. Confirmation shows the oldest
and newest unassociated duration, canonical group counts and affected album, Favorite, progress and
unresolved duplicate-choice counts. A group is counted once even when several provenance records
contribute to it. Cleanup keeps album definitions and original images. It has no ordinary undo;
applying an existing backup remains an explicit recovery option. Cancelled or failed writes retain
all selected records. Changed eligibility requires another inspection and confirmation.

Schema upgrades preserve user data through forward migrations and a recoverable pre-upgrade
backup. R4 provides versioned export and restoration of durable organization data and the identity
evidence needed to reconnect it, including existing durable duplicate choices. Exports do not
package originals, thumbnails, or model files. Restore validates the format and identities before
publication, preserves the current data on failure, and does not duplicate albums or decisions when
repeated. Unmatched restored records remain available for association or explicit cleanup. Backups
are restored explicitly; reimport alone must not undo a deliberate source removal or record cleanup.

Persistence queries and result windows stay bounded. Restoring album views, counts, or a review
position must not require loading the entire library into the UI, rescanning original files, or
adding a new startup-wide content-hash pass. Reuse the admitted identity and duplicate-analysis
pipelines, retaining their source-read, cancellation, and cloud-placeholder safeguards.

## Acceptance requirements

Preserve the following R4 behavior through affected implementation changes and the
[repository verification workflow](../acceptance/quality-gates.md#delivery-and-verification):

- cross-folder duplicates fold in albums after late identification; folded and unfolded selections
  target the same complete groups and current representatives;
- single-album moves, independent favorite/unfavorite actions, removal-to-ignore, return-to-pending,
  ordinary-mode organization, cancellation, repeated operations, and undo preserve their boundaries;
- conflicting album assignments become deferred work with persistent, actionable conflict feedback;
  restart and repeated analysis preserve the conflict until a valid choice resolves it;
- content changes retain unchanged group members, detach obsolete inherited membership, and apply
  either a compatible destination group's decision or the new group's pending state;
- new groups follow the selected sort and join the front under `新加入优先`, duplicate copies do not add work,
  and continuation restores a logical position after insertion, regrouping, and restart;
- first-use mode entry, the upper-right organization controls, contextual actions, keyboard/focus
  behavior, empty queues, unavailable images, and save failures work in the existing desktop gallery
  and viewer;
- committed album and progress changes survive interruption together; retry and stale undo cannot
  overwrite newer decisions or attach old decisions to changed content;
- temporary source loss preserves data, explicit source removal cleans its records, shared groups
  survive through other sources, and manual unresolved-record cleanup preserves live associations;
- forward migration, catalog rebuild, export/restore, malformed restore input, repeat restoration,
  and unmatched records preserve the stated recovery and explicit-removal behavior;
- opening albums and continuing work remain bounded at the target library scale, with measurements
  of the affected user paths;
- original bytes and filesystem entries remain unchanged, and cloud-only sources are not hydrated.
