# Unified gallery UI contract

Status: retained accepted presentation contract under [ADR 0009](../architecture/0009-accepted-unified-gallery-ui-contract.md).

The [roadmap](../roadmap.md) owns delivery order and current availability. Future controls described
here remain absent until their owning use cases are implemented. This document preserves the
accepted user workflow; it does not supersede technical or accessibility decisions in the ADRs.

## Scope and authority

The current UI source of truth is the user-confirmed Microsoft Photos-like structure recorded in
this document. Flutter Material 3 supplies components, tokens, focus behavior, and accessibility; it
does not redefine the information architecture.

UI implementation follows a reuse-first admission order: Flutter Material and framework widgets,
repository-owned shared components, mature external packages, then the smallest necessary custom
layer. A custom control requires a recorded capability gap and must compose around the framework
primitive that owns interaction, focus, semantics, and platform behavior rather than replacing it.

On 2026-08-07 the user accepted this structure after interactive review. ADR 0009 records the
current authority and fully supersedes ADR 0003. Later work may refine spacing, responsive behavior,
component details, and wording, but it must not silently reverse these accepted
information-architecture choices:

- one unified gallery rather than peer folder, timeline, category, search, or duplicate pages;
- sources and albums only in the sidebar;
- the Library row owns Add folder, and Settings remains pinned to the sidebar bottom;
- duplicate display and review inside the gallery filter menu rather than a standalone action;
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
accessible label are `添加文件夹到图库`; activating it opens the same folder picker as the global
`导入` action.

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

Exact duplicate handling is a gallery filter, not a peer toolbar action or navigation destination.
The compact filter action follows the Microsoft Photos grouped-menu structure while exposing only
capabilities that Ame currently supports:

```text
显示子文件夹
隐藏子文件夹
────────────────
显示所有文件
合并完全相同图片
仅显示重复图片
────────────────
审查重复组
```

The menu contains two independent single-choice groups followed by one command. Before R3 the
available default is `显示子文件夹 + 显示所有文件`. After R3 has trustworthy exact evidence, the
initial duplicate mode becomes `合并完全相同图片` so repeated physical copies do not multiply the
user's review workload; the user can still select either other mode. The first group selects whether
the current source includes descendant folders. The second group selects one exact-duplicate
display mode:

- `显示所有文件`: show every physical file instance;
- `合并完全相同图片`: fold byte-identical copies into one representative item without merging
  durable identities;
- `仅显示重复图片`: show only exact duplicate groups.

`审查重复组` is a contextual command at the bottom of the same filter menu. It enters review in the
existing gallery canvas and does not create a new page or sidebar entry.

R2a may exercise the confirmed duplicate choices with deterministic fixtures. They remain hidden in
the production shell until R3 connects trustworthy exact-duplicate evidence.

The current product indexes images, so the Microsoft Photos `所有媒体 / 照片 / 视频` group is not
copied into the early UI. Video filters appear only after video indexing becomes accepted product
scope. Classification and category choices do not become ordinary filter items. When R5 is
functional, effective category projections appear as smart-album result groups under `相册`.

A merged representative displays its copy count. Selecting a merged representative selects a
logical group, not an arbitrary physical path. Any future path mutation requires expansion and
explicit selection of `AssetLocation` values.

Duplicate review remains in the same main canvas. Early review can inspect paths, compare file
evidence, mark a preferred copy, ignore a group, and generate non-executable suggestions. It
contains no delete action.

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
position when possible.

## Gallery canvas and timeline

- Default layout is a dense, aspect-preserving justified photo wall with date headers.
- Only the visible region and a bounded overscan area are rendered and decoded.
- The right-side time rail represents the complete filtered result set, not only loaded widgets.
- Material 3 defines a vertical standard `Slider` with an optional stops configuration. The Slider
  owns pointer, keyboard, focus, hover, handle, track, and semantic behavior; Ame must not recreate
  those behaviors in a parallel custom control.
- The repository-pinned Flutter 3.44.9 `Slider` implements the Material interaction and visual core
  but does not expose the specification's native vertical-orientation API. Ame therefore uses the
  already validated thin orientation adapter around the official `Slider`; replacing it with a
  hand-built gesture or semantics implementation requires new measured evidence and an ADR change.
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
  immediately shows static placeholders in its final equal-height rectangles, never a generic
  square grid or a blank substitute view.
- Scroll-triggered detail paging uses one thin linear progress indicator at the top of the gallery.
  It does not add circular loaders to the photo wall or its boundaries.
- Slider drag writes the exact manifest-backed position at most once per rendered frame. Detail
  requests are latest-wins, bounded, cancellable or generation-guarded, and issued at a measured
  cadence outside the pointer-to-scroll critical path. Release promotes the final target, but it is
  not the first opportunity to prepare its detail page.
- A distant date click jumps directly to the resolved logical anchor rather than animating through
  the library. Cached details and previews appear immediately; otherwise final-geometry
  placeholders remain responsive while the target and guard pages load.
- Window resizing coalesces to one newest layout request per frame. The prior snapshot remains
  coherent until the replacement snapshot and its logical-anchor correction publish atomically.
  Preview decoding uses bounded width buckets, and obsolete intermediate-width computations cannot
  publish.
- Generic square placeholder slivers, aggregate-only unloaded geometry, settle-only wheel seeks,
  and interaction-specific replacement-window paths are temporary implementation debt and must be
  removed only after the ADR 0014 parity tests pass. They are no longer accepted target behavior.
- Month points and year labels use their real content scroll offsets. They are not evenly spaced:
  months containing more rendered gallery height occupy more rail distance, while dense anchors
  may cluster. The current-position indicator has a fixed visual height and does not represent the
  viewport height.
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

## Temporary task and notification feedback

An active import uses a bottom floating progress surface similar to the reference workflow:

```text
正在添加文件夹“Picture”…
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
Automatic authoritative recovery and retry remain `正在更新图库` without publishing notifications.
Only a failed monitor, exhausted recovery, bridge/catalog refresh failure, or another blocked
condition that can no longer converge automatically enters the notification queue; successful
recovery resolves that active error without adding a success notice.
An active error is not resolved merely because another automatic attempt has started. Its compact
row state and one deduplicated notification remain stable until a synchronized snapshot proves
convergence. Short SQLite writer contention during another Ame publication keeps the prior snapshot
and retries silently for a bounded interval; only contention beyond that interval becomes a
persistent catalog error.
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
- filter menu with subfolder and exact-duplicate groups, merged representative, and duplicate review;
- layout menu with both shape modes and all three thumbnail sizes;
- search results and empty results;
- preview failure and retry;
- single-image view and return-to-scroll-position behavior;
- sort menu for all confirmed keys and both directions;
- time rail including unknown capture time;
- settings page with every initial row and its plain-language help text.

## Settings page

The Settings row pinned to the sidebar opens one shallow settings canvas while the global bar and
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
  打开图片时               适应窗口 / 实际大小
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
