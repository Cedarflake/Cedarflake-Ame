# Workbench, shell and navigation

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Workbench blueprint

The workbench has three regions in Library and Albums: a narrow function rail, a contextual sidebar,
and the photo wall. Library shows sources and Favorites; Albums shows Favorites and the album list. The rail
remains visible while the secondary sidebar can collapse. The normal desktop window has a
1,100-by-700 logical-pixel minimum, capped by the selected monitor's available work area. Restoring
an older smaller window expands it to that minimum before showing it. The sidebar toggle
stays in the function rail immediately above Settings while either is active, with `收起侧栏` and
`展开侧栏` tooltips. Animate the sidebar boundary and photo content's left edge together over a short
transition. The gallery title and count follow that same left edge independently of the toolbar.
Empty, loading and no-result content, the bottom organization dock and the Albums landing content
follow the visible content area's center throughout the same transition. Their shared containers
own this placement so later content does not require another sidebar animation.
Smoothly move and resize visible photos when sidebar or window dimensions change,
including maximize/restore, while retaining the current browsing anchor.
Keep the gallery's right edge, toolbar and time rail anchored to the window. Ordinary actions remain
beside the title instead of moving below it at a fixed width. The gallery lays out once at its destination
width, keeping its state and preview ownership. Respect
reduced motion and reverse an interrupted toggle from the current boundary. Remember the
preference, selected source and folder expansion when switching workspaces. Primary-sidebar switches
to Albums, Duplicates or Settings clear ordinary selection and return the photo wall to the top.
When Library is organizing, those visits retain its mode, queue, source scope, browsing position and
selection for the return to Library. Albums itself remains ordinary browsing. Reuse the accepted
theme, global chrome, photo wall and viewer.

Library supports `全部图片` and `合并重复图片` in the existing upper-right filter area. The first
shows every location; the second shows one scoped representative of each confirmed exact group.
The heading always starts with the total image count for the current folder/search scope. Merged
mode appends its displayed count, for example `10000 张图片 · 去重后 72 张`; All images shows
`10000 张图片`. Both counts come from the same catalog revision and scope.
Ordinary and not-yet-analysed images remain visible. Remember the user's display choice. Each
confirmed duplicate card has a top-left stacked-image action only in `合并重复图片`; `全部图片`
has no duplicate card action. The card action shows no count, and the top-right selection checkbox
keeps its existing independent action. Both controls use compact 28-pixel rounded-square marks in
transparent 40-pixel click targets, with the existing Material Symbols icons and no outer filled disc.
Clicking the duplicate action
opens a bottom sheet with separate filename and folder lines and a current-image marker;
clicking a location reveals that file in File Explorer. Long filenames and paths wrap completely
inside the sheet's continuously scrolling list, with bounded incremental reads and no pagination
buttons. The sheet has a drag handle and no close button; Escape or a click outside dismisses it.
Closing it retains the gallery position and restores existing keyboard focus to the badge when
available, without creating focus after mouse activation. Normal local availability is omitted; retain the current-image marker
and cloud-only, offline or unavailable states. Locations outside the current folder are explicitly inspectable without
widening the gallery selection scope.

During recovery, previously confirmed folded relationships may remain visible while their unchanged
physical observations are rechecked. The badge tooltip and sheet header show one group status:
`正在更新` during analysis, `已暂停` when paused, and `等待后台` while waiting to resume. Individual
locations retain only their existing current-image and availability markers. The sheet's count and
rows come from one current display snapshot. Newly confirmed copies
join progressively, while changed identity/state or a different confirmed fingerprint removes only
the affected old relationship. Retained display never skips content verification or authorizes a
preferred-copy decision. Fresh items without prior evidence remain visible as ordinary images.

Card duplicate and selection marks are 24-pixel circles inside separate transparent 40-pixel
targets. Use the shared icon library for their marks, with no second visible background ring.
Cards reserve at least 128 logical pixels of width for the duplicate icon, selection target and
their padding. Counts no longer affect card width. Asynchronous badge arrival and hover do not
resize cards. Both loaded-window and query-wide geometry use the same floor; square mode
also respects it. The duplicate and selection targets have separate bounded space without clipping or
overlap.

The Duplicates workspace owns exact and similar group inspection and durable representative,
ignore and exclusion choices. It reuses gallery previews and viewing. Exact groups establish byte
identity; similar groups contain distinct contents selected by the admitted perceptual comparison.
Similarity adds no filter or folding to Library, Favorites or albums. No analysis or display choice
authorizes source mutation.

Put the heading/count block on the left and icon menus on the right. The layers-icon `分组` menu
offers `重复组`, `相似组` and `已忽略` in that order, without top tabs. Show `严格 / 标准 / 宽松` through the adjacent
sliders menu only in the similar-group list, initially `标准`; its tooltip and accessible name identify
the current degree. Changing degree queries existing results. `已忽略` combines exact and similar
groups with kind labels; similar groups there use the broad degree so choices remain reachable.
Use `重复图片查找` for the exact list and the selected view name for the other headings. Reserve the
group-count line during loading; omit a separate scope, description or analysis-status block.

Member, viewer and comparison pages omit both menus. The member heading uses the initial
representative filename, with the complete filename available through its tooltip. `对比图片` and
`忽略此组` / `恢复此组` share the right action row. Selecting two members enables comparison; omit a
separate instruction/count toolbar. Wide layouts align actions with the heading; compact widths and
enlarged text wrap actions below it with right alignment. Primary-destination changes reset nested
navigation and temporary selection. Ordinary return from viewing or comparison preserves the group.

Groups use responsive multi-column cards and members use full-width rows. Both reuse the
thumbnail/text skeleton and whole-card hover/open interaction. Member rows retain filename, path,
availability and accessible preference/Explorer actions; similar members also expose exact-copy
locations and `移出此组`. Keep actions inline when space permits and wrap them below at narrow widths.
Measure text and controls instead of assigning a fixed row height. Omit decorative folder icons,
a separate `首选副本` label and a redundant `查看图片` button. Group actions read `查看重复图片` or
`查看相似图片`. The location sheet states the number of copies; each location row reveals that file
in Explorer, with `当前图片` at the right and availability/wrapped paths below the filename.

Both levels scroll continuously through bounded payload/cursor windows and viewport-bound previews.
Empty and failed results use the shared centered state component; read failures retain retry.
Initial/continuation reads use the two-pixel loading overlay. Saving choices retains rows and scroll
position; catalog/analysis publication revalidates the visible range without clearing retained cards.
The full-window viewer navigates across member batches. Comparison shows two independently guarded
original images, supports linked or independent zoom/pan and reset, and returns to the member view.
Its top bar contains back, title, window dragging and the standard minimize/maximize/close controls.
Reuse the viewer's bottom toolbar for zoom mode, percentage, slider, zoom buttons and full screen;
place linking and reset there too, with the same button geometry and spacing. Pointer interaction
or the left/right arrow keys select the active pane; omit a duplicate pane-selection control. Both panes
reuse normal viewing's smooth wheel/button zoom, double-click mode switch, bounds and drag inertia.
Keep the images side by side and the bottom controls in one row within the supported desktop window.

An exact preference selects the scoped merged-gallery representative with a deterministic fallback.
A similarity preference selects one content representative only within that group. Exact copies of
that content share one member row while their locations remain inspectable. Ignore/restore saves in
place; returning to the list reflects the decision. `移出此组` excludes the represented content only
from the current similar group; shared feedback can undo that specific exclusion, including after
returning to the list. Groups below two distinct contents disappear from the list without forcing
navigation. A retained preference survives temporary exclusion or unavailability. Choices survive
reopening, degree changes and compatible reanalysis. There is no finish/next-group workflow.

Analysis is automatic while Ame is running, after initial foreground browsing has priority.
New or changed catalog content queues bounded analysis; compatible completed fingerprints survive
restart and cancellation. Visible-preview reads, original-image reads and explicit foreground tasks
take priority. Automatic synchronization and cached browsing can overlap admitted analysis so a long
update does not leave new duplicates unfolded until every other task ends. Completed evidence reaches
the gallery in bounded batches through its existing publication owner. Cloud-only files wait for local availability. Analysis never
adds startup source enumeration or a second complete source copy. Retain structured analysis progress
at the application boundary. Discovery starts without visiting Duplicates; this workspace has no
manual refresh, start, pause or stop-search controls. Legacy pause settings do not block discovery,
and navigation does not cancel admitted analysis.

Badge updates may appear in place. Changes that fold/reorder photo cards are coalesced until
interaction is idle and preserve the current logical image anchor. An open viewer retains its current
image and navigation sequence. The scheduler and query publication owners, rather than a loading
message or animation, establish these guarantees.

## Global shell

```text
Ame | [在图库中搜索] | 通知 | 最小化 / 最大化 / 关闭
```

The Library row's folder-plus action is the current entry point into the Ame-owned `AddLibraryRoot`
use case. It opens the folder picker and owns validation, progress, cancellation, and error state.
The global bar contains application identity, gallery search, one icon-only notification-history
control, and app-drawn window controls; library import and settings do not appear there. The
notification control is immediately before the caption controls. It uses the ordinary notification
icon when read and the notification-with-dot icon when unread, without a text label or count.

The application mark in the global bar retains its outer silhouette and central cutout.
A two-logical-pixel contour in the theme's primary color surrounds a `primaryContainer` fill,
separated by a 1.5-logical-pixel transparent gap. The cutout has no inner stroke. The mark shares a
28-logical-pixel size and horizontal center with the primary navigation rail. The Windows application
icon uses the colored artwork; the title-bar mark retains the same vector shape. The adjacent
"Cedarflake Ame" label uses semibold text in the theme's primary
color and follows the existing compact-window rule that keeps only the mark visible.

There is no permanent Task button or task-center navigation entry. While an import or library update
is active, a temporary bottom progress surface uses the concrete action name, reports progress and
offers cancellation. After completion, the same surface changes to an explicit completed result,
retains the final counts, removes cancellation, and remains until the user acknowledges it.

## Left sidebar

The global bar and function rail share `surfaceContainerHigh`. The source/folder sidebar uses
`surfaceContainerLow`, and the main workspace retains `surfaceContainerLowest`. The sidebar and
main workspace have matching 18-logical-pixel top-left corners, with the enclosing surface visible
behind each corner. Theme-derived tonal surfaces distinguish these levels without divider lines.

The Library sidebar contains only navigation and source scope:

- `图库` followed by a separate `文件夹` heading with the trailing import action;
- imported folders in one aligned source list, without separating OneDrive and local folders into
  different navigation hierarchies;
- expandable folder trees when functional.

Albums is a separate primary destination between Library and Duplicates. Its secondary sidebar
contains an independent Favorites entry followed by the flat album list, creation and management
actions; it contains no source folders. Favorites navigation belongs only to Albums and does not
change album membership. Until a collection is selected, the content area invites selection or creation from
the sidebar. Each browsing destination retains its own query. Primary-sidebar switches reset ordinary
position and selection; an organizing Library retains its mode, queue, position and selection across
Albums, Duplicates and Settings visits. The accepted narrow-window sidebar rule, tonal surfaces,
resizing and transition remain shared.
The album creation button and each album's More button share the same trailing alignment and inset.
The album section heading uses semibold. Album names match Favorites and folder navigation: medium
by default and semibold when selected, retaining the existing font sizes and selected-state colors.
Library and Albums share the same sidebar structure: the Library/Favorites destination comes first,
followed by a `文件夹`/`相册` section heading. The plus button on the right of that heading imports a
source folder or creates an album. The Library destination itself has no trailing import action.
An empty manual-album list uses the same static icon-and-text row as an empty folder list, with
`还没有创建相册`. It appears only after loading confirms that the album total is zero, not for an
empty page of a larger collection or a loading/error state.
Library sidebar multi-selection includes only imported top-level sources. Child-folder rows remain
browsing destinations, with no selection control, and are excluded from Select all and batch removal.
Albums and imported top-level folders support saved drag order. Upgrade preserves the existing order;
new entries append, and renames, refreshes and restarts retain the arrangement. Child-folder ordering
and physical paths are unchanged. Album pages follow the same saved order.
In sidebar multi-selection, an unselected row retains its original leading icon. A selected row uses
a filled circular check from the existing icon library. The heading keeps the selected count at
the left and Select all, removal/deletion and plus at the right. Select all is an 18-pixel neutral
outlined square with no selected color fill; deletion and plus use 20-pixel icons, with all three
actions in 36-pixel targets. Normal navigation resumes after selection is cleared; dragging is
available outside multi-selection. Pointer selection does not leave a row focus highlight after
the pointer leaves; keyboard navigation retains visible focus. Primary-workspace switches clear
secondary-sidebar selection independently of the organization-mode photo-selection rules.
Escape clears sidebar selection. Album Select all applies to the current page and names that scope
when further pages exist; paging resumes after selection is cleared. A single confirmation lists
the selected entries. Batch management preserves completed removals if stopped or interrupted by a
failure, and retry processes only the remaining entries.

`收藏夹` scopes the gallery by favorite state independently of album membership. Albums do not nest,
and each current image group has at most one confirmed album assignment. A group can appear in
Favorites and its album at the same time. Activating either navigation entry scopes the unified
gallery without moving, copying, renaming, or deleting source files. The
[R4 requirements](../r4-organization.md#albums-favorites-and-duplicate-groups) own group normalization,
late folding, independent favorites, and assignment rules.

Selecting an album immediately displays its known name in the content heading. Asynchronous
metadata loading retains that name; a confirmed rename or removal updates it when the result arrives.

When R5 is functional, the same `相册` navigation area contains a distinct `智能相册` section below
manual albums. Its result groups derive from `EffectiveCategory`, independently of manual-album
membership. Smart albums have no create, rename, delete, drag-order, or membership-selection controls
and never appear as targets in the album tray. Compatible model results and retained legacy decisions
determine their category projection. The [smart-album workflow](smart-albums.md#smart-album-workflow) owns this interaction.

Every imported root is a folder from the user's perspective. Cloud-backed, offline, unavailable,
or removable-media properties may appear as a row status or badge, but they do not create separate
OneDrive and `此电脑` groups. Folder icons, labels, optional status text, and overflow actions use
shared column constraints so every source row aligns.

The folder-plus icon is a separate hit target from the `图库` navigation row. Its tooltip and
accessible label are `添加目录到图库`; activating it opens the same folder picker as the global
`导入` action.

Selecting an exact registered source retains its normal update behavior. Selecting an unregistered
child already covered by a registered parent opens that parent's existing folder scope without
another source registration or scan. Coverage does not complete an unfinished parent import or
remove legacy registrations. A primary import or retained paused import still excludes another
primary import; selecting folders does not create an import queue.

Selecting a new parent of registered child folders imports the complete parent and then replaces
the covered registrations together with that publication. The existing child galleries remain
available until it succeeds; interruption or failure cannot remove them. The current child-folder
view follows its corresponding folder under the parent while preserving search, sorting and
descendant scope. An unresolved completion or failed display refresh retains its Retry action and
blocks a new import until the result is confirmed and displayed; Retry does not repeat the scan.

Clicking a source or child folder scopes the same gallery. Clicking its expansion control only
changes the tree. Show expansion controls only for folders with cataloged child folders. Root
windows load in bounded, serial catalog reads before expansion and refresh when the catalog changes;
empty roots keep their selection and overflow actions, and failed reads retain a retry action.
A source-row overflow action, secondary click, or keyboard context-menu request
opens the same Material menu containing:

- `更新图库`;
- `在资源管理器中打开`;
- `从 Ame 中移除`.

Removing a source unregisters it from Ame without deleting or modifying files on disk. Once R4 is
connected, its confirmation also explains the affected organization-record cleanup under the
[removal and recovery rules](../r4-organization.md#persistence-removal-and-recovery). Temporary source
unavailability does not trigger that cleanup, and groups still present in other sources retain
their valid records. Drawing this menu does not complete the source-removal use case.

Do not add Timeline, Categories, Search, Sort, Filter, Settings, Task Activity, or Duplicate Review
to the Library's source/folder sidebar. The primary function rail retains Albums, Duplicates, and
Settings under ADR 0029; R5 category collections belong to Albums' `智能相册` section.
