# Unified gallery UI contract

Status: accepted presentation contract under [ADR 0009](../architecture/0009-accepted-unified-gallery-ui-contract.md)
and [ADR 0029](../architecture/0029-functional-workbench-navigation.md).

The [roadmap](../roadmap.md) owns delivery order and current availability. Future controls described
here remain absent until their owning use cases are implemented. This document preserves the
accepted user workflow; it does not supersede technical or accessibility decisions in the ADRs.

## Scope and authority

The current UI source of truth is the user-confirmed Microsoft Photos-like structure recorded in
this document. Flutter Material 3 supplies components, tokens, focus behavior, and accessibility; it
does not redefine the information architecture.

The UX reference application is **Microsoft Photos Legacy**, as clarified on 2026-10-10. Compare
new interaction decisions with that application and the accepted behavior below. Observations of
the newer Microsoft Photos application retain their recorded version and evidence scope; they do
not establish Photos Legacy behavior or supersede this contract.

The [component implementation requirements](#component-implementation) own reuse order, capability
evidence and framework interaction boundaries for every affected surface.

On 2026-08-07 the user accepted this structure after interactive review. ADR 0009 records the
current authority and fully supersedes ADR 0003. Later work may refine spacing, responsive behavior,
component details, and wording, but it must not silently reverse these accepted
information-architecture choices:

- one reusable gallery for folder, timeline, category and search scopes;
- a permanent function rail selects the Library and Duplicates workspaces;
- the Library workspace owns the collapsible source/folder sidebar and Add folder;
- Settings remains pinned to the bottom of the function rail;
- the gallery filter changes duplicate display; the Duplicates workspace owns group review;
- temporary action-specific import progress rather than permanent task navigation;
- a plain-language settings canvas rather than a dialog or engineering dashboard;
- Simplified Chinese as the initial application language.

ADR 0003 remains historical evidence only. Its standalone duplicate action, task activity, provider
grouping, and other superseded surfaces are not current alternatives.

Simplified Chinese (`zh-CN`) is the only initial application language. All user-visible titles,
actions, menus, tooltips, accessible labels, progress text, empty states, confirmations, and error
explanations use concise Simplified Chinese. File names, paths, metadata values, and quoted operating-
system details retain their original content. Internal code identifiers and stable error codes remain
English and must be translated into understandable Chinese at the presentation boundary rather than
leaking raw implementation messages.

R2 does not add a language selector or a complete localization runtime. User-facing copy is kept in
a presentation-owned string catalog instead of being scattered through widgets, so formal i18n can
be introduced later without rewriting the UI structure.

The completed R2a prototype remains acceptance evidence, not a second application entry point.
Unavailable controls must not ship as dead production actions, and a fixture or screenshot is not
feature-completion evidence.

## Design principles and rationale

These principles explain the accepted surfaces below and guide new choices within their scope.
Keep the reason and applicable exception with a changed decision. New capabilities or reversals of
accepted navigation require a product decision; routine spacing, component composition and responsive
adjustments follow the established design without another approval step.

| Principle | Reason and application | Adaptation boundary |
| --- | --- | --- |
| Give each visible element a useful role | Photos, names and paths are the work. A title identifies context, supporting text adds needed information, and controls offer distinct actions. Omit duplicated explanations, decorative icons beside sufficient previews, and buttons that repeat the same row action. | Preserve actionable errors, exceptional availability, accessible names and discoverability. A familiar auxiliary icon can save space; an unfamiliar action may need visible wording. |
| Preserve spatial relationships | Stable controls and retained image identity let people keep their place. The title and photo wall follow the sidebar's leading boundary; window-edge controls stay anchored. Counts reserve space, and data updates preserve the current item and browsing position. | Window constraints and enlarged text may require intentional reflow. Background updates must respect interaction and publication ownership rather than hiding jumps with animation. |
| Choose density for the information | Group cards support visual comparison; member rows expose long names and paths. Height follows actual text, preview and target sizes. Shared alignment and spacing make neighboring views predictable. | Cards, rows and text/icon actions are contextual choices, not universal formats. Check wrapping and reachability at compact widths and enlarged text; keep accessible targets. |
| Match feedback to the affected action | Saving a preference updates that decision while the reviewed content remains usable. Initial loads, continuation reads, explicit refresh and failures retain their appropriate feedback. Counts and wording describe the actual scope and state. | A local update must still expose in-flight admission and failure. Retained content cannot imply freshness or success that the application has not established. |
| Use motion to explain continuity | Sidebar, window and viewer transitions show where content moves and preserve the user's visual reference. Scrolling shares the established inertia behavior across surfaces. | Retarget interrupted motion from what is displayed, respect reduced motion and preserve direct input. Animation must not delay actions, mask state defects or require unbounded layout/preview work. |
| Share behavior with appearance | Common loading, empty, scrolling, hover, focus and preview owners keep the same operation recognizable across features. Repair an invariant at its owner so adjacent callers inherit the correction. | Reuse cohesive primitives and composition. Do not force different tasks into one flag-heavy component or change established behavior merely to make shapes match. |

For a changed workflow, establish its entry, primary action, resulting state and return path before
expanding its implementation. Compare it with the adjacent accepted surface and run the affected
interaction while the change is still small. The
[UI verification map](../acceptance/quality-gates.md#ui-workflow-verification) links these decisions
to shared owners, existing regressions and client checks. It is the discovery point for implementation
and evidence; this product contract does not duplicate that inventory or record test results.

## Component implementation

- Prefer installed framework/design-system components, shared repository components, mature
  packages, then minimal custom code. The [dependency policy](../../AGENTS.md#8-dependency-and-open-source-policy) also applies to UI packages; appearance cannot
  justify stale, low-adoption, poorly documented or difficult-to-replace dependencies.
- Before a new/substantially redesigned Flutter control, inspect `https://m3.material.io/components`
  and the pinned SDK API/implementation. Record the official component, verified installed
  capability, evaluated alternatives and product gap in its decision/evidence. Memory, visual
  similarity or design availability does not establish SDK support.
- Before custom controls, record evaluated components and missing behavior. Compose around framework
  scrolling, selection, menus, dialogs, focus, input and accessibility;
  do not replace those primitives. Reuse tokens, components and state mechanisms instead of isolated
  visual constants or copied behavior. Organize behavior, structure, then presentation.
- Use virtualization/lazy slivers, bounded thumbnail decoding/caches and stable item identity/scroll
  position across incremental updates. Keep business/persistence state out of views; never send
  full-resolution images or unbounded result sets across the desktop bridge.
- Represent loading, empty, partial, cancelled, failed and stale states explicitly. Meet keyboard,
  focus, contrast, text scaling and screen-reader accessibility expectations.
- Apply the [content and action rules](#content-and-action-wording) to
  affected text and adjacent controls. Copy-only work preserves layout, navigation and behavior;
  authorized interaction changes must implement and verify the wording's promised action/result.

## Content and action wording

- Before editing, verify where the text appears, what triggers it, its adjacent titles and buttons,
  and the states before and after the action. When a change affects interpretation or presentation,
  check it in the actual interface for meaning, wrapping, truncation, and its relationship to the
  available actions.
- Titles describe what is happening; body text adds information needed in that situation; buttons
  name the action they perform. Include instructions in body text when the context needs them.
  An already clear button does not need a repeated instruction pointing to it, and a description
  can be omitted when there is no useful information to add.
- Organize copy by interface location and usage scenario. Document the affected locations of shared
  text. When the same words serve different purposes in different places, separate definitions may
  be appropriate to avoid changing another scenario unintentionally. Comments explain purpose and
  state conditions so maintainers can locate the interface and understand when the text appears.
- Preserve user-approved terms and wording, and edit existing text within the current authorization.
  Copy-only work preserves layout, navigation, and behavior. When the user explicitly requests
  interaction changes, implement the corresponding behavior and verify that the wording matches
  the actual action and result.

## Workbench blueprint

The workbench has three regions in Library: a narrow function rail, a source/folder sidebar,
and the photo wall. The rail remains visible while the secondary sidebar can collapse. Its toggle
stays in the function rail immediately above Settings while Library is active, with `收起侧栏` and
`展开侧栏` tooltips. Animate the sidebar boundary and photo content's left edge together over a short
transition. The gallery title and count follow that same left edge independently of the toolbar.
Smoothly move and resize visible photos when sidebar or window dimensions change,
including maximize/restore, while retaining the current browsing anchor.
Keep the gallery's right edge, toolbar and time rail anchored to the window; the toolbar
also stays right-aligned when it wraps below the title. The gallery lays out once at its destination
width, keeping its state and preview ownership. Respect
reduced motion and reverse an interrupted toggle from the current boundary. Remember the
preference and retain selected source, folder expansion, selection and gallery scroll position when
switching workspaces. Reuse the accepted theme, global chrome, photo wall and viewer.

Library supports `全部图片` and `合并重复图片` in the existing upper-right filter area. The first
shows every location; the second shows one scoped representative of each confirmed exact group.
The heading always starts with the total image count for the current folder/search scope. Merged
mode appends its displayed count, for example `10000 张图片 · 去重后 72 张`; All images shows
`10000 张图片`. Both counts come from the same catalog revision and scope.
Ordinary and not-yet-analysed images remain visible. Remember the user's display choice. Each
confirmed duplicate card has a top-left stacked-image action only in `合并重复图片`; `全部图片`
has no duplicate card action. The card action shows no count, and the top-right selection checkbox
keeps its existing independent action. Clicking the duplicate action
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

Cards reserve at least 128 logical pixels of width for the duplicate icon, selection target and
their padding. Counts no longer affect card width. Asynchronous badge arrival and hover do not
resize cards. Both loaded-window and query-wide geometry use the same floor; square mode
also respects it. The duplicate and selection targets have separate bounded space without clipping or
overlap.

The Duplicates workspace owns duplicate-only results, analysis controls, group inspection and
preferred-copy/ignore decisions. It reuses gallery preview and viewing capabilities. Exact duplicates
are byte-identical; perceptual similarity is a separate later capability and is absent until connected.
No analysis or display choice authorizes source mutation.

Use `重复图片查找` as the workspace heading and show the current category's total number of
duplicate image groups beneath it, reserving that line while changing categories. Omit the scope,
description and separate analysis-status text. Put the heading/count block on the left, the
category switch at the content area's horizontal center, and refresh/pause actions on the right
in one row. Equal flexible space on both sides keeps the switch centered independently of label
length. Compact widths and enlarged text stack these blocks while retaining the centered switch
and right-aligned actions.
Analysis and review actions pair familiar leading icons with their existing text labels. Group
review adds ignore/restore to the right-aligned action row. The subpage's back/title block and
actions share one vertically centered row when space permits; compact widths and enlarged text
move actions below the heading while retaining right alignment. Keep the full filename available
through the existing path tooltip when the heading is truncated. Groups use responsive multi-column
cards; group members use full-width single-column rows so paths have enough space. Both reuse the
thumbnail/text skeleton. Group cards retain their lower-right text action; member rows place
preference and Explorer icon buttons on the right, vertically centered, without a footer. Measure
text and controls to reserve only their needed height, including enlarged availability text.
The selected group's initial representative filename is the subpage heading; align
its rows beneath that heading. Member rows use their image preview without an additional
decorative folder icon or a separate `首选副本` label. The whole card provides hover feedback and
opens its group or image. Member icons retain hover and accessible action names; omit
the redundant `查看图片` button. Group actions read `查看重复图片`; the gallery sheet heading states
the number of duplicate images. Each location row is the Explorer reveal action, without a second
reveal button. Put `当前图片` at the right of its row; keep availability and wrapped paths beneath
the filename. Result loading uses the gallery's two-pixel overlay at the boundary
between toolbar and cards. Saving or switching a preference retains the member rows and scroll
position. Subsequent catalog/analysis publication revalidates the visible range in the background
and replaces it atomically, without clearing cards or flashing a page loader. Manual refresh and
initial/continuation reads retain their loading feedback; read failures retain a retry action.

Both levels scroll continuously without Previous/Next page buttons. Load catalog batches for the
visible range and retain bounded payload and cursor caches; preview demand remains viewport-bound.
Empty and failed results reuse the gallery's centered icon/title/detail component.
Result groups and their members include bounded image previews and inspectable file locations.
Opening a member uses the existing full-window image
viewer, including navigation across catalog batches; returning preserves the reviewed group and scroll
position. `设为首选` selects the representative used
by merged gallery queries, with a deterministic in-scope fallback when that location no longer
matches. `忽略此组` moves it to `已忽略`, where `恢复此组` restores it to the regular review list.
These choices survive reopening and compatible reanalysis. Ignoring a group leaves its gallery
images and badges visible, and neither action changes a source file.

Analysis is automatic while Ame is running, after initial foreground browsing has priority.
New or changed catalog content queues bounded analysis; compatible completed fingerprints survive
restart and cancellation. Visible-preview reads, original-image reads and explicit foreground tasks
take priority. Automatic synchronization and cached browsing can overlap admitted analysis so a long
update does not leave new duplicates unfolded until every other task ends. Completed evidence reaches
the gallery in bounded batches through its existing publication owner. Cloud-only files wait for local availability. Analysis never
adds startup source enumeration or a second complete source copy. Retain structured analysis progress
and expose manual pause/resume in Duplicates; automatic analysis begins without a visit to that page.

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

There is no permanent Task button or task-center navigation entry. While an import or library update
is active, a temporary bottom progress surface uses the concrete action name, reports progress and
offers cancellation. After completion, the same surface changes to an explicit completed result,
retains the final counts, removes cancellation, and remains until the user acknowledges it.

## Left sidebar

The sidebar contains only navigation and source scope:

- `图库` with a trailing folder-plus add-source action;
- an expandable `相册` section when R4 is functional, containing the system-provided `收藏夹` and
  user-created album groups;
- imported folders in one aligned source list, without separating OneDrive and local folders into
  different navigation hierarchies;
- expandable folder trees when functional.

`收藏夹` is not a separate data model or a peer outside the album system. It is the built-in album
group for the default collection workflow. User-created groups use the same durable membership
contract. Activating any album entry only scopes the unified gallery to that group and never moves,
copies, renames, or deletes source files.

When R5 is functional, the same `相册` navigation area also contains a distinct `智能相册` subsection.
Its result groups are derived from `EffectiveCategory`, not ordinary album membership. Users cannot
create, delete, rename, directly add, or directly remove their images, and smart albums never appear
as targets in the `加入相册` dialog. Correcting a category through `UserOverride` recomputes the
effective result and therefore moves the asset between derived groups without editing membership.

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
changes the tree. A source-row overflow action, secondary click, or keyboard context-menu request
opens the same Material menu containing:

- `更新图库`;
- `在资源管理器中打开`;
- `从 Ame 中移除`.

Removing a source unregisters it from Ame only. A confirmation must state that files on disk are not
deleted or modified. The source-removal business use case is not implied complete by drawing this
menu.

Do not place Timeline, Categories, Search, Sort, Filter, Settings, Task Activity, or Duplicate Review
in the sidebar.

## Unified gallery header

Normal browsing state:

```text
图库 · 结果数量                          选择 | 排序 | 筛选 | 布局 | 更多
```

Selection state:

```text
已选择 N 个项目                    取消 | 加入相册 | 比较 | 重复信息 | 更多
```

The normal toolbar is replaced rather than nested when selection begins. Selection is keyed by
stable Ame asset identity and survives lazy item disposal and scrolling.

`加入相册` becomes visible only after R4 connects durable album membership. It is an action in this
upper-right selection-specific area, while entries under the sidebar's `相册` section are navigation
scopes. The action and entries share durable membership data but never share interaction
responsibility.

When the album prompt is enabled, activating `加入相册` opens one Material dialog before membership
changes are applied:

```text
选择要加入的相册

☑ 收藏夹
☐ 用户分组 A
☐ 用户分组 B

[新建相册]                                      [取消] [确定]
```

- `收藏夹` is the initial default selection, but the user may clear it and select one or more other
  groups;
- the dialog supports membership in multiple groups rather than forcing one exclusive destination;
- reopening it reflects actual membership; multi-item selection uses checked, mixed, and unchecked
  states so existing membership is not misrepresented;
- confirmation updates only Ame-owned album membership and never changes source files;
- when prompting is disabled, `加入相册` immediately adds the selected assets to the configured
  default group and reports the result with a reversible confirmation surface;
- the production control remains absent until membership persistence, settings, errors, undo, and
  the complete user path are connected.

In normal browsing, a gallery tile reveals its upper-right selection checkbox on pointer hover or
keyboard focus. Selection mode shows checkboxes on every visible tile, and selected tiles always
retain the checkbox, check mark, and Material primary-color outline. Activating the tile body opens
the viewer; activating the checkbox changes selection without opening. Touch and assistive
technology receive an always-available semantic selection action and do not depend on hover.

Versions before R9 do not show executable delete, move, or copy placeholders. R8 may show clearly
non-executable plan actions only when the dry-run workflow is connected end to end; filesystem
execution appears only when R9's implementation and safety gates are accepted.

Every gallery item supports a Material context menu opened by secondary click or the platform
keyboard context-menu gesture. It targets the item under the pointer without losing an existing
multi-selection. R2b connects only actions with real non-mutating or catalog-only behavior:

- `打开`;
- `查看信息`;
- `复制路径`;
- `在文件资源管理器中打开`.

Later stages may add `查看重复位置`, `加入相册`, and other accepted actions when their underlying use
cases exist. Edit, print, share, move, copy-file, rename, and delete actions from Microsoft Photos
are not copied as inert placeholders. Menu placement, focus, dismissal, keyboard navigation, and
semantics use Flutter Material primitives rather than a hand-built overlay.

The browsing toolbar's `更多` menu initially contains:

- `全选` (`Ctrl+A`);
- `不选择任何项目` (`Esc` or `Ctrl+D`).

`全选` covers the complete current source, search, sort, and filter result rather than only the
loaded Flutter window. Its bounded representation is the current query identity plus explicit
exclusions; it must not materialize every matching asset ID. Changing the owning query clears the
selection so its meaning cannot drift. `不选择任何项目` returns to browsing state and is disabled
when no selection exists. The menu uses Flutter's Material popup-menu primitives, while application
shortcuts continue to use `Shortcuts` and `Actions` behavior.

Menus opened beside a window edge retain the shared viewport margin. Labels and shortcut hints use
bounded flexible layout so narrow windows and platform font metrics cannot paint outside the menu.

## Sort behavior

The compact sort action opens two independent choice groups:

```text
拍摄日期 | 创建日期 | 修改日期 | 名字
升序 | 降序
```

The initial default is `拍摄日期 + 降序`. A capture-time sort keeps missing evidence in an explicit
unknown section rather than substituting another timestamp. Date headers use the selected date
source. Name sorting does not retain false date headers or a chronological time rail.

R2 UI fixtures show this confirmed menu. Production options become visible only with a bounded,
revision-safe backend query for the corresponding key and direction; sorting a partial Flutter
window locally is forbidden.

## Filter and exact-duplicate behavior

The gallery Filter menu has two independent single-choice groups:

```text
显示子目录
隐藏子目录
────────────────
全部图片
合并重复图片
```

The source group controls descendants of the current folder. The duplicate group controls whether
all physical instances or one representative per confirmed exact group appears. Before trustworthy
exact analysis and incremental maintenance are connected, every image remains visible. Once folding
is connected, it becomes the initial default while preserving an explicit saved choice.

Consecutive filter/search changes retain the original visible image and its fractional viewport
position. A missing image, empty result or automatic clamp to an edge does not replace that intent;
when conditions include the image again, restore it within the available scroll range. Actual
scrolling, timeline navigation, source/folder selection or a changed ordering establishes a new
browsing context. Passive catalog refresh preserves the currently displayed result and leaves the
latent filter intent available for the next explicit filter change.

The Duplicates workspace owns duplicate-only results and group review. A top-left icon in merged
gallery mode opens every known current location of that group, including relevant availability
states and full paths. All-images mode has no duplicate card marker. Location activation uses the existing File Explorer reveal action. A representative
selects a logical group; physical operations require explicit location selection in their later workflow.

Source/folder/date/search scope still limits gallery membership. A group spanning folders cannot
silently widen that scope. Pending and unreadable files remain visible. Review can retain preferred
copies and ignored groups without changing source files. Perceptual similarity remains distinct.

## Layout behavior

The compact layout action follows the Microsoft Photos two-group menu:

```text
等高
方形
────────
小
中等
大
```

The first group selects the layout shape and the second independently selects thumbnail size.
Initial defaults are `等高 + 中等`.

- `等高` uses an aspect-preserving justified photo wall.
- `方形` uses a uniform square grid and may crop thumbnails for presentation only; source media is
  never changed.
- `小`, `中等`, and `大` adjust the target visual density without changing the bounded lazy-loading
  and decoding rules.

Both choices apply to the same unified gallery canvas and preserve stable item identity and scroll
position when possible. Consecutive density or shape choices retain the same image and fractional
viewport point until browsing moves on, so a size round trip does not accumulate a row of drift.
Scrolling or navigation establishes the next position; an earlier layout choice must not pull the
viewport back after that input. A layout choice made at the top or bottom retains that edge;
content that initially fits in one viewport stays at the top. Existing bottom intent survives
temporarily fitting in one viewport, so expanding the cards again returns to the bottom.
Other anchors clamp to the available scroll extent.

## Gallery canvas and timeline

- Default layout is a dense, aspect-preserving justified photo wall with date headers.
- Only the visible region and a bounded overscan area are rendered and decoded.
- The right-side time rail represents the complete filtered result set, not only loaded widgets.
- During updates in the same query, a complete rail remains visible until replacement geometry
  agrees with the gallery extent. Temporary window estimates cannot replace that complete frame;
  estimates remain available on first load and after a query change. Retained paint grants no
  stale navigation authority. New clicks, drags and keyboard input resolve the visible date through
  the current result, without waiting for complete layout geometry. Geometry replacement retires
  interrupted gestures and their position override, while ordinary gallery scrolling remains native
  and immediate.
- Material 3 defines a vertical standard `Slider` with an optional stops configuration. The Slider
  owns pointer, keyboard, focus, hover, handle, track, and semantic behavior; Ame must not recreate
  those behaviors in a parallel custom control.
- The repository-pinned Flutter 3.44.9 `Slider` implements the Material interaction and visual core
  but does not expose the specification's native vertical-orientation API. Ame therefore uses the
  already validated thin orientation adapter around the official `Slider`; replacing it with a
  hand-built gesture or semantics implementation requires new measured evidence and an ADR change.
  Endpoint spacing belongs to the track shape inside the full pointer surface. Outer Slider
  padding must not leave visible endpoint marks outside the clickable region.
- Material divisions are equidistant selectable stops and therefore cannot represent Ame's
  nonuniform month offsets without changing their semantics. Year/month marks are a narrow
  annotation layer driven by the complete-result timeline data; they do not own dragging, focus, or
  value selection.
- The gallery `ScrollController` is the sole authoritative scroll-position state. The Slider value
  is a projection of that global offset and writes back to the same controller; no second timeline
  position, independently synchronized scroll model, or page-local Slider state may be introduced.
- The complete query has one compact, revision-bound layout manifest containing stable ordering,
  orientation-corrected aspect ratios, date groups, and availability flags. Rust supplies it in
  bounded chunks, and Flutter stores compact typed data rather than full asset records, paths,
  metadata, or previews. Its memory cost and fallback representation must pass the gates in ADR
  0014.
- Orientation-corrected width and height are durable catalog evidence, not preview-cache metadata.
  They remain authoritative across restart, preview failure or cleanup, viewport resize, and an
  identity-proven rename or move. A temporarily unavailable source retains its last trustworthy
  dimensions together with separate availability evidence. Source replacement, incompatible media
  inspection, or a confirmed content edit requires reinspection and one atomic catalog revision;
  an authoritative location removal is the only normal path that removes its dimensions.
- One deterministic layout snapshot derives final row membership, item rectangles, cumulative row
  offsets, date anchors, and total extent from that manifest. Placeholder, failed-preview, and
  decoded states use the same rectangle. Preview completion or eviction with already-known
  dimensions must never recompose rows. When compatible decoding first recovers previously unknown
  dimensions, Ame freezes the actual viewport-center card and current logical range, coalesces that
  geometry evidence, and atomically replaces the snapshot with a pre-paint anchor correction.
  Reflow cannot widen its own recovery range, and later evidence remains deferred until native
  scrolling establishes a new idle range instead of chaining one relayout into another.
- Full `LibraryAsset` details are queried in bounded revision-safe keyset pages. The current
  controller is known to merge those pages into a growing `state.assets` list; Profile evidence
  determines its target-library cost before a high/low-watermark cache replaces that retained-list
  baseline. No single page or 160-item replacement window is the gallery's global presentation
  model.
- Preview readiness lives in an identity-keyed store outside layout state. Visible and near-
  viewport previews receive priority, obsolete generations cannot publish, and expensive decoding
  may be deferred during high-velocity scrolling. Within each priority, stationary demand begins at
  the actual center card and expands toward both sides; recovered geometry waits until native user
  scrolling is idle.
- A preview is a rebuildable, budgeted artifact whose identity includes stable location and source
  state, preview algorithm and version, orientation contract, and a bounded physical-pixel size
  bucket. Ame selects the smallest compatible bucket that satisfies the current display size and
  scale, rather than generating an unbounded variant for every logical pixel width. Preview
  absence, failure, staleness, regeneration, or eviction never changes durable dimensions.
- Wheel, touchpad, keyboard, accessibility, and ballistic movement remain native relative activity
  on Flutter's one `Scrollable` and do not enter an asynchronous intent queue. Slider drag, date
  click, restored position, source navigation, search navigation, and resize submit programmatic
  intents to one coordinator only where their writes require arbitration. Both paths preserve one
  query- and revision-bound logical viewport anchor rather than synchronizing several pixel offsets.
- Wheel and touchpad movement use native relative scrolling. Crossing a detail-page boundary
  prefetches bounded pages before and after the viewport without replacing the canvas. A cold page
  paints non-interactive card backgrounds in its final equal-height rectangles behind the shared
  loading line, as specified by ADR 0014. Detail and preview arrival preserve those rectangles and
  the precise position already reached by the rail in an unchanged layout.
- Mouse-wheel deceleration is shared across Ame scroll regions through `AmeScrollMotion` and
  `AmeScrollController`, including navigation, settings, details, tasks and the viewer filmstrip.
  Each region retains its own native scroll position. Wheel movement is immediate except for a
  90 ms ease-out tail of at most 24 logical pixels; repeated input preserves its total distance,
  and reversal, pointer-down, direct positioning or layout correction retires the old tail.
  Reduced-motion mode and precision touchpad input retain native movement. Vertical mouse-wheel
  input also scrolls horizontal regions; native horizontal/Shift input has first claim, and an
  inner region at its boundary leaves the event available to its parent.
  The pinned SDK's `ScrollController.createScrollPosition` and `PrimaryScrollController` provide
  the extension points. Material `ScrollBehavior` has no controller factory, and `ScrollPhysics`
  alone does not animate mouse-wheel jumps. `AmeScrollMotionTheme` carries a separate primary
  controller into captured Material routes without replacing their menus, focus or semantics.
  Framework controls that require their own private controllers retain those controllers.
- A cold thumbnail reveals its first decoded frame with a 120 ms ease-out opacity transition
  inside the existing rectangle. Synchronous cache hits appear immediately. Active or recent
  scrolling, disabled animations and initially inactive ticker scopes skip the reveal; further scrolling
  finishes an active reveal immediately. Muting an already visible surface preserves its current
  blend through viewer entry or exit. Image opacity owns the effect without a whole-wall layer,
  staggered delays, shimmer or changes to row geometry.
- Scroll-triggered detail paging uses one thin linear progress indicator at the top of the gallery.
  It overlays the gallery without resizing its viewport or intercepting gallery input. It does not
  add circular loaders to the photo wall or its boundaries.
- Slider drag writes the exact manifest-backed position at most once per rendered frame. Detail
  requests are latest-wins, bounded, cancellable or generation-guarded, and issued at a measured
  cadence outside the pointer-to-scroll critical path. Release promotes the final target, but it is
  not the first opportunity to prepare its detail page.
- A distant date click jumps directly to the resolved logical anchor rather than animating through
  the library. Cached details and previews appear immediately; otherwise the same final geometry
  remains scrollable behind the shared loading line while the target and guard pages load.
- Window resizing coalesces to one newest layout request per frame. The prior snapshot remains
  coherent until the replacement snapshot and its logical-anchor correction publish atomically.
  Preview decoding uses bounded width buckets, and obsolete intermediate-width computations cannot
  publish.
- Generic square placeholder slivers, aggregate-only unloaded geometry, settle-only wheel seeks,
  and interaction-specific replacement-window paths are temporary implementation debt and must be
  removed only after the ADR 0014 parity tests pass. They are no longer accepted target behavior.
- Month points and year labels use their real content scroll offsets. They are not evenly spaced:
  months containing more rendered gallery height occupy more rail distance, while dense anchors
  may cluster. Year-text collision handling never redistributes the scroll axis or month points;
  marker thinning uses only dot collisions because text has a separate column. The current-position
  indicator has a fixed visual height and does not represent the viewport height.
- Layout changes interpolate retained date labels, month points and the current-position line over
  180 ms. Sidebar changes, resized viewports, gallery layout changes and query-condition changes
  share this behavior. Wheel, slider and keyboard position input stays immediate; reduced motion
  skips the transition. The visual rail retains its prior frame while a changed query awaits its
  layout, then uses that frame as the transition origin. Prior-query ticks cannot accept input;
  navigation resumes when the painted frame and navigation projection belong to the current query.
- Year-label collision handling follows the annotated-scrollbar rule: retain the first collection
  label, remove colliding upper labels, and keep at least 4 px between visible labels.
- During drag, the active date label follows the current-position line, the gray hover preview is
  suppressed, and timeline marker dots remain visible rather than disappearing beneath the line.
- A year or month jump immediately changes the manifest-backed position and requests the bounded
  target detail page and guard pages. A stale response from an older query, request generation, or
  layout must not change the current gallery geometry or position.
- Source, search, date-sort, and duplicate-state changes recompute the time distribution.
- Unknown capture time has an explicit section and deterministic fallback ordering.
- No user-visible pagination or page transitions are introduced.
- Opening an image and returning restores its prior gallery item and scroll position.

## Single-image loading and zoom

While an original image is loading, its progress indicator stays at the center of the image
viewport. Zoom, pan and wheel input transform the image or retained preview, never the indicator.
Loading completion retires the indicator through the current source image's frame lifecycle.

The viewer enters with a 180 ms opacity transition and exits in 100 ms. Its gallery stays mounted
at the existing scroll position. Closing immediately retires viewer input, accessibility focus and
source-read authority; at most one non-interactive closing surface remains for the visual exit.
Reopening replaces that surface, and further pointer input finishes entry immediately. Disabled
animations use an immediate transition. Original pixels reveal over the same image's small preview
in 100 ms without resetting the zoom transform. The existing preview element stays mounted across
the original's first frame, including after cache eviction, and is released when the reveal finishes.
These transitions use Flutter's image opacity and fade primitives, without an additional package,
layout interpolation or a queue of outgoing full-resolution images.
Duplicate review opens the same viewer with the shared fade component and timing; its full-window
route adds no page zoom or slide. Returning retains the underlying review list and its position.

An open gallery viewer retains its displayed source version after a confirmed replacement and shows
`图片已更新，重新加载以查看最新内容。` with `重新加载`. Reload adopts the latest confirmed catalog
version through the existing guarded source reader. A confirmed removal keeps the viewer open and
shows `这张图片已不在图库中。` with `返回图库`; its obsolete ordinal and adjacent navigation are
unavailable. Same-version renames update the name without a content warning. Query revisions,
selection changes and disposal retire delayed lookup results; a failed lookup cannot claim absence.
Gallery-page rebuilds cannot silently replace the retained viewer version before the user reloads.
These notices share the existing viewer fallback notice, neutral notification surface, rounded
corners and action styling. They float within the image viewport without moving its contents. A
confirmed source notice takes precedence over retry feedback for the old source version.

The zoom percentage keeps its source-pixel reference: actual size (`1:1`) is 100%, enlargement by
one quarter is 125%, and reduction by half is 50%. The default opening preference is Automatic:
images exceeding the available width or height shrink to fit the viewport, while fitting images
stay at 100% actual size. This mode never enlarges a small image to fill the window. Automatic,
fit-to-window and actual-size preferences persist across restart; an existing saved choice is
preserved. One toolbar button switches between fit and actual size; its icon and tooltip describe
the next action. Automatic remains the opening preference in Settings, so manual zoom and the
toggle affect only the current image. Explicit fit preserves
the whole image within the available width and height; its percentage follows the resulting pixel
scale rather than being relabeled as 100%. Resizing the image area preserves actual size or a manual
pixel scale, while automatic and fit continue to follow the available area.

The bottom toolbar groups the thumbnail-strip toggle, information button and more-actions menu
first on the left, followed by compact file type/dimensions/size and the current image count.
The count includes the image unit and is separated from file metadata by a thin vertical divider.
The bottom More menu opens upward from its fixed lower edge above the button and closes toward
that same edge. Its placement and animation leave the triggering button unobscured.
The top bar retains the filename, back action and window controls. Both sides of the bottom
toolbar use the same icon size and text style. Zoom mode, percentage, zoom slider and full-screen
action are on the right; full screen is the last action. The strip starts hidden and remembers its
visibility. It uses bounded catalog paging and lazily creates visible thumbnails; selecting one opens
that image in the current query order. Its ordinary mouse wheel moves horizontally through the native
scroll position, alongside native horizontal-wheel, touchpad and scrollbar input.
Small triangle buttons at the scrollbar ends move the strip by one thumbnail without changing
the selected image. Each disables at its corresponding end. They use Material `IconButton`;
the installed `ScrollbarThemeData.mainAxisMargin` reserves their space outside the native track.
New thumbnail pixels retain the placeholder until decoding finishes; a larger preview of the same
source retains the already decoded image, and scrolling skips the first-frame fade.

The strip is first created when expanded. Collapsing retains its lazy visible elements until the
viewer closes, while removing its preview demand, focus and animation activity. Every expansion
centers the current image within the natural scroll bounds while reusing compatible previews.
Near the start or end, the strip aligns to that boundary without adding empty space to force
centering. Selection changes while the strip remains open only scroll
when needed to bring the thumbnail fully into view. Manual strip scrolling leaves the main image
unchanged.

Completed preview metadata and verified sizes each keep at most 256 recently used entries beyond
their immediate request lifetime. They reuse compatible results when returning to an image; source
generation changes and root/cache invalidation retire incompatible evidence. Decoded pixels remain
in Flutter's existing bounded image cache, with only the current lazy strip window retained while
collapsed. Closing the viewer releases the strip elements and requests; disk previews retain their
normal storage-budget lifetime. A missing or damaged displayed preview admits one automatic repair
for that source while visible, without loading the rest of the library. Failure leaves the image
selectable, and a late result cannot revive a hidden or replaced thumbnail's repair.

This retention composes the installed Flutter `Visibility.maintainState` and `TickerMode` behavior;
it does not introduce another pixel cache or persist strip scroll offsets across application restarts.

The bottom more-actions menu opens above its button with a small gap, keeping the toolbar visible.

F11 or the rightmost button toggles native full screen. Escape exits full screen first, otherwise
returns to the gallery. Full screen preserves normal window placement and maximized state and is
not restored on startup. Images change through manual navigation; there is no automatic playback.

The controls compose Material `IconButton`, `Slider`, `ListView.builder` and `Scrollbar`, whose
toggle, keyboard, lazy-item and scroll-position APIs are available in the pinned Flutter SDK.
The product-specific strip adds selection, catalog-page continuity and bounded preview demand;
scroll physics, focus and input dispatch remain framework-owned.

## Temporary task and notification feedback

An active import uses a bottom floating progress surface similar to the reference workflow:

```text
正在添加目录“Picture”…
已检查 12,340 个文件 · 已找到 10,826 张图片
进度条                                                       取消
```

Completion changes the same surface to `导入完成`, retains the final checked, imported, and issue
counts, removes the cancel action, and remains until the user chooses `知道了`. Cancellation and
failure use action-specific messages. The completed result is dismissible task feedback, not a
permanent validation card, status bar, task center, or generic task entry.

Bottom notifications and import feedback share one Material surface contract for color, width,
corner radius, elevation, and placement. The same event must not produce competing gray and white
notification surfaces.

The sidebar keeps only compact per-source state such as `已同步`, `正在更新图库`, `更新受阻`, or
`目录不可用`. Failure and reconciliation detail belongs to the bottom notification queue and
the bounded current-session history opened from the global bar. A detail record may contain a
localized reason, bounded affected-work counts, source path, technical code, and one connected retry
action when the failed application operation can be replayed. Automatic root recovery never exposes
a control that starts a manual full scan. Active conditions deduplicate until resolved; acknowledgement removes a
surface from the queue but cannot change catalog freshness. Scan progress and terminal scan feedback
retain priority over notifications.
For root reconciliation, the active-condition identity is the root itself: observer-health,
freshness-cause, and issue-code transitions update one record until the root is synchronized.
Startup recovery first applies Windows Search hints through guarded file checks. After this initial
refresh, ordinary background verification keeps cached gallery content and anchors available, with
`正在核对` in the source row and no additional progress notification. It does not display `已同步`
until the backend and gallery have actually converged. A missing index takes the same quiet verification path. Unavailable sources,
failed monitors and blocked recovery retain their existing visible feedback. Changes found by
verification refresh only affected results through the existing anchored gallery lifecycle.
Index hints are never a second visible source of names, dimensions or preview versions. The gallery
consumes only guarded catalog publications. Later verified additions, removals and replacements
use the existing revision and source-generation boundaries so an older request cannot restore stale
information. A version change invalidates derived previews and analysis together; unchanged items
retain their cached results. Index omissions may leave cached records visible until verification
reaches them, without establishing synchronized freshness.
Other automatic authoritative recovery remains `正在更新图库`; an eligible deferred retry displays
`等待自动重试`. Neither publishes a fault notification.
An unconfirmed startup or continuity transition has a neutral source-row label and creates no
notification history merely because its first snapshot is incomplete.
Only a failed monitor, exhausted recovery, bridge/catalog refresh failure, or another blocked
condition that can no longer converge automatically enters the notification queue; successful
recovery resolves that active error without adding a success notice.
An active error remains visible while the current snapshot still reports blocked recovery or an
unhealthy monitor, including when unrelated automatic work is running. A healthy, unblocked
processing or eligible-retry snapshot withdraws that error surface and displays progress in the
source row; the historical error remains in current-session history. The last issue code can remain
as historical context during healthy, unblocked processing, retry or confirmed convergence; it
does not alone establish a current fault. Blocked recovery and unhealthy monitoring retain their
priority over progress. `已同步` requires a synchronized source snapshot and settled gallery results.
Automatic analysis admission, active analysis and committed-result publication share `正在更新`
in the source row, so foreground yielding and short internal phases do not repeatedly change its text.
Explicit pause and analysis failure retain their duplicate-analysis wording. The Duplicates workspace
keeps its detailed progress. Once synchronization and visible results settle, the source row returns
immediately to its completed state, independently of notification dismissal. Terminal analysis still
waits for the committed query refresh; a failed refresh retains the existing failure and reload
workflow. These result phases affect only a healthy
source that has finished directory synchronization, and never hide unavailable or blocked states.
Short SQLite writer contention during another Ame publication keeps the prior snapshot
and retries silently for a bounded interval; only contention beyond that interval becomes a
persistent catalog error.
Resolved condition records are labelled as history, stop their active elapsed timer, and no longer
offer a recovery action. A notification action rechecks its retained identity and current action
binding when invoked, so an already-open detail view cannot trigger a retired or replaced action.
Open notification details follow the same retained notification ID as its explanation and action
change. Resolution or history eviction removes its timer and action and marks the view as history;
a later condition on the same root cannot replace the event currently being inspected.
Manual-update explanations and actions require current blocked recovery authority. Historical
manual-recovery codes cannot override healthy progress or confirmed convergence. An unavailable
source explains access restoration before any recovery operation, and a source without a
synchronization snapshot displays `图库尚未核对` rather than inferred update progress.
SQLite read-modify-write paths acquire an immediate writer transaction before observing mutable
state, while an empty path-queue poll remains read-only. This serializes concurrent publication with
the configured busy timeout instead of failing a deferred WAL snapshot during write upgrade.
Foreground polling and background inventory or authoritative recovery keep ordinary writer contention
silent and retryable for 30 seconds before projecting one persistent failure.

## Accepted UI regression states

Deterministic presentation fixtures and focused regressions retain coverage for these accepted
states without becoming production data paths:

- empty library;
- active import and import failure;
- populated unified gallery;
- source tree, unavailable source, and source overflow menu;
- selection and cancellation;
- filter menu with subfolder and exact-duplicate groups, merged representatives and location badges;
- function-rail navigation, retained collapsible source sidebar and duplicate review in Duplicates;
- layout menu with both shape modes and all three thumbnail sizes;
- search results and empty results;
- preview failure and retry;
- single-image view and return-to-scroll-position behavior;
- sort menu for all confirmed keys and both directions;
- time rail including unknown capture time;
- settings page with every initial row and its plain-language help text.

## Settings page

The Settings button pinned to the function rail opens one shallow settings canvas while the global bar and
sidebar remain visible. It is not a dialog, engineering dashboard, or hierarchy of abstract
configuration pages. The visual pattern follows Microsoft Photos: a clear `设置` title, a centered
readable column, plain section headings, and full-width rows containing an icon, a user-facing
title, one short explanation, and a control on the right.

Initial settings are limited to behavior that is understandable and connected end to end:

```text
个性化
  应用主题                 跟随系统 / 浅色 / 深色

浏览
  查看图片时的鼠标滚轮     放大或缩小 / 上一张或下一张
  打开图片时               自动（默认） / 适应窗口 / 实际大小
  缩略图加载速度           小 / 中 / 大

相册（R4 接通后）
  加入相册前询问           开 / 关
  默认加入的相册           收藏夹 / 用户创建的相册组

存储
  图库数据位置             当前路径 | 更改
  缩略图位置               当前路径 | 更改
  缩略图最大占用空间       容量选项
  清理缩略图               清理

关于
  Ame 版本
  开源软件声明
```

The completed R2b preview lifecycle uses the following storage labels and descriptions. They
explain the consequence instead of exposing implementation terms:

- `图库数据位置`: `保存 Ame 的图库索引和设置，不会移动或复制原图片`;
- `缩略图位置`: `保存可重新生成的预览，不会保存第二份原图`;
- `缩略图最大占用空间`: `达到上限后自动清理较少使用的缩略图`;
- `清理缩略图`: `缩略图会在需要时重新生成，不会删除原图片`.

Automatic reclamation, foreground cleanup, and startup recovery are connected end to end, so this
target wording replaces the earlier admission-only fallback text. A future partial implementation
must not claim that reclamation occurred until the corresponding verified workflow is restored.

Changing a storage location must show whether restart or migration is required before confirmation.
Clearing thumbnails must name the rebuild cost and confirm that source files are untouched. Theme,
viewer, preview loading speed, album, and storage choices persist across restarts. Preview loading
speed defaults to `中`; changing it applies to subsequent queue starts without cancelling active
decodes. `加入相册前询问` initially defaults to on, and `默认加入的相册` initially defaults to
`收藏夹`. When prompting is on, the configured group
is preselected in the dialog and may be changed for that operation. When prompting is off, the same
setting is the direct destination. These rows remain absent until R4 connects album membership end
to end.

Clearing previews removes only rebuildable artifact files and resets compatible preview-index
entries to pending. It retains catalog width and height, orientation evidence, capture metadata,
source configuration, user decisions, and operation history. Before and after cleanup, a fixed query
and viewport must produce the same row membership, item rectangles, total extent, and logical scroll
anchor; visible pixels then regenerate through normal demand priority.

ADR 0005 owns high/low-watermark eviction, verified cleanup, startup recovery, bounded variants, and
switch-and-regenerate preview relocation. R2b implements that lifecycle: a new root activates after
restart, the previous root becomes explicitly retired only after successful activation, and cleanup
removes only verified Ame-managed artifacts after confirmation. Ame never silently migrates or
deletes source files or unrelated files from either root.

Do not expose database schema, catalog revisions, adapter or engine names, raw worker counts, queue
depth, hash algorithms, cache keys, memory limits, analysis-run identifiers, or other engineering
vocabulary in ordinary settings. Diagnostics may later be exported from `关于`, but do not become a
permanent settings dashboard. Account, OneDrive-account, Clipchamp, video, and classification rows
from the reference application are not copied unless Ame later owns those capabilities.

Only functional settings appear in the production page. R2a fixtures may demonstrate this confirmed
page, but unavailable rows remain absent from the production shell until connected.

## Explicit current UI exclusions

The current UI prototype and early production shell do not show:

- classification, category filters, category pages, classification review, or model status;
- perceptual or semantic similarity;
- people, faces, or identity features;
- editing tools;
- delete, move, copy, rename, recycle-bin, quarantine, or dry-run execution controls;
- a permanent task center, read-only-validation entry, cache diagnostics, or engineering limits.

Classification remains an R5 feature. It later appears as effective-category smart albums that
scope the same unified gallery without becoming filter choices, peer gallery applications, or
editable user albums. Users correct category authority through the review workflow rather than by
directly editing derived smart-album membership.

## Review workflow

When R4 introduces persistent `ReviewSession` state, Ame may show one lightweight contextual
`继续整理` surface above the existing gallery. It appears only when review work exists and may
summarize classification items, similarity groups, or other accepted queues. It is not a home
dashboard, AI center, task center, or new sidebar destination.

Activating it reuses the unified gallery with a session-owned query and visible progress. The
review surface must support keyboard-first decisions, multi-selection where the decision is safe,
undo, defer, issue states, and cross-restart continuation. Closing the application preserves the
session's durable progress; reopening does not guess completion from model confidence or gallery
scroll position.
