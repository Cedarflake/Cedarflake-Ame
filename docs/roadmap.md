# Cedarflake Ame Roadmap

Status: active delivery plan

Last confirmed with the user: 2026-08-22

Last implementation-status synchronization: 2026-09-02

Repository: this repository root

Canonical discovery entry: the repository-root `AGENTS.md` points to `docs/roadmap.md` and
requires every new project session, post-compaction continuation, roadmap-status review, and
product-work delegation to read it completely. This file is the only active roadmap copy; do not
fork or mirror it into another editable roadmap.

This roadmap is stored in the repository so delivery intent survives workstation and session
changes. Durable engineering and architecture rules remain in `AGENTS.md`; accepted technical
decisions remain in `docs/architecture/`.

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
  and a Lightroom-style editor are outside this roadmap.
- Delete, move, copy, rename, quarantine, and recycle-bin execution remain unavailable until R9 and
  require a freshly authorized plan, current-state revalidation, an operation journal, and explicit
  partial-failure and recovery behavior.

### 2.1 Stable workbench concepts

The roadmap keeps the following concepts distinct. A later ADR may refine storage or API shape but
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

Known reference evidence from the real library:

- Lap v0.3.0 terminated twice at the same scan position with Windows `0xc0000409`.
- The recovery point was near a file named as JPEG whose content was valid PNG.
- A read-only header probe found thousands of JPG/PNG extension-content mismatches.
- A large group of `G:` files returned access-denied during content reads.
- Ame therefore requires per-file failure isolation, format detection based on evidence rather than
  extension alone, structured issue reporting, and recoverable tasks.

## 4. Confirmed UI information architecture

### 4.1 Scope and authority

The current UI source of truth is the user-confirmed Microsoft Photos-like structure recorded in
this section. Flutter Material 3 supplies components, tokens, focus behavior, and accessibility; it
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

### 4.2 Global shell

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

### 4.3 Left sidebar

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

### 4.4 Unified gallery header

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

### 4.5 Sort behavior

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

### 4.6 Filter and exact-duplicate behavior

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

### 4.7 Layout behavior

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

### 4.8 Gallery canvas and timeline

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

### 4.9 Temporary task and notification feedback

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

### 4.10 Accepted UI regression states

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

### 4.11 Settings page

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

### 4.12 Explicit current UI exclusions

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

### 4.13 Review workflow

When R4 introduces persistent `ReviewSession` state, Ame may show one lightweight contextual
`继续整理` surface above the existing gallery. It appears only when review work exists and may
summarize classification items, similarity groups, or other accepted queues. It is not a home
dashboard, AI center, task center, or new sidebar destination.

Activating it reuses the unified gallery with a session-owned query and visible progress. The
review surface must support keyboard-first decisions, multi-selection where the decision is safe,
undo, defer, issue states, and cross-restart continuation. Closing the application preserves the
session's durable progress; reopening does not guess completion from model confidence or gallery
scroll position.

## 5. Accepted technical baseline for R0 validation

The following baseline is accepted for validation, not assumed successful until R0 gates pass:

- Flutter stable and Dart for the Windows desktop presentation layer;
- Flutter Material 3 components and design tokens;
- Riverpod for presentation and ephemeral UI state only;
- a Rust workspace for domain, application, catalog, task, and adapter ownership;
- SQLite through a Rust-owned persistence adapter;
- `flutter_rust_bridge` behind a narrow generated bridge boundary;
- Tokio with explicit cancellation and bounded concurrency;
- structured Rust logging through `tracing`;
- recoverable worker processes for native codecs and other high-risk parsers when introduced;
- Windows 11 x64 as the first release and benchmark target.

Fallback is not chosen by preference. If Flutter/Rust integration fails an R0 acceptance gate, the
failure evidence and alternative must be recorded in a replacement ADR before changing the stack.

## 6. Delivery sequence

Only one roadmap stage may be active at a time.

### R0 - Technical validation

User outcome:

A user selects a real test directory, Rust discovers supported images, persists a small catalog,
generates bounded previews, and Flutter displays them in a Material 3 lazy gallery while showing
real progress and allowing cancellation.

Required acceptance evidence:

- reproducible Windows toolchain and build instructions;
- accepted ADRs for UI stack, bridge, persistence ownership, and process isolation direction;
- a real directory picker and real filesystem input rather than hard-coded or mock assets;
- Rust-owned scan use case with structured progress, cancellation, and per-file issues;
- a forward-migrated SQLite catalog in application data, never in the image directory;
- preview cache stored outside source trees;
- Flutter lazy gallery using real indexed records and generated preview paths;
- ordinary, empty, cancelled, partial-error, and completed UI states;
- source media byte identity unchanged by the test;
- Rust format, Clippy, tests, Flutter analyze, Flutter tests, Windows build, and diff checks;
- a small packaged or release-mode launch verification.

R0 is a feasibility gate, not the first product release.

### R1 - Progressive multi-root catalog

User outcome:

Multiple roots become visible immediately and can be browsed progressively while durable indexing
continues, pauses, resumes, and reuses unchanged catalog evidence during an explicit rescan. R1's
incremental reconciliation does not by itself mean that changes are detected automatically while
Ame is running or while it is closed; continuous detection and catch-up belong to R2c.

Scope:

- `LibraryRoot`, `Asset`, and `AssetLocation` persistence;
- multiple roots and root availability states;
- file discovery, core metadata, capture-time evidence, and incremental reconciliation;
- configurable catalog and preview-cache locations and budgets;
- OneDrive availability detection without automatic hydration;
- scan checkpoints, restart recovery, cancellation, structured issues, and atomic publication;
- viewport-priority preview scheduling.

Acceptance includes corrupt, locked, missing, Chinese-path, long-path, wrong-extension, multiple-volume,
and unavailable-file fixtures followed by controlled read-only real-library scans.

### R2 - Unified gallery, UI first

User outcome:

The user browses one continuous date-grouped gallery, changes source folders from the sidebar, jumps
through time from the right rail, opens images, selects items, and searches filenames without visible
pagination.

R2a - UI contract and interactive prototype:

- reconcile the obsolete portions of ADR 0003 before treating any existing UI behavior as a
  requirement;
- implement the exact shell, source tree, source menu, normal toolbar, selection toolbar, sort menu,
  grouped filter menu, grouped layout menu, duplicate-review canvas, photo wall, time rail, import
  progress surface, image viewer, and settings page defined in section 4;
- use deterministic presentation fixtures to make every required state interactive and screenshot
  reviewable without pretending the fixtures are production catalog behavior;
- keep UI components driven by Ame-owned view models and callbacks rather than Rust or third-party
  engine types;
- review the complete UI flow with the user to validate its implementation and visual details without
  treating rejected legacy navigation as an open design alternative;
- hide unavailable production controls until their backing use case is connected.

R2a acceptance:

- no peer tabs for folder, timeline, categories, search, or duplicates;
- no mixed Chinese and English placeholder copy in the user-facing prototype;
- no classification surface or placeholder;
- no permanent task, read-only validation, cache, or engineering-status navigation;
- source removal is visibly distinguished from deleting source files;
- normal and selected toolbars replace each other rather than nesting;
- sort keys and direction match section 4.5;
- no standalone duplicate toolbar action exists; exact duplicate modes and review are owned by the
  filter menu defined in section 4.6;
- filter and layout choices match sections 4.6 and 4.7, including two independent layout groups;
- settings use plain user-facing rows from section 4.11 and contain no engineering dashboard;
- the prototype covers every state in section 4.10 at desktop and constrained window widths;
- keyboard, focus, tooltip, contrast, and text-scaling behavior is testable;
- user review accepts the UI structure before R2b expands business integration.

R2b - production behavior integration:

- bounded keyset windows ordered by capture time and stable asset identity;
- bounded lazy photo wall with `等高 / 方形` shape and `小 / 中等 / 大` density choices;
- right-side year/month distribution and arbitrary date jump using the stable global virtual-scroll
  contract in section 4.8 rather than loaded-window proportions or page replacement;
- global search field with basic filename and path search;
- normal and selection contextual toolbars;
- full-view presentation, details surface, and stable selection across scrolling;
- source scoping, descendant-folder filtering, source-tree expansion, rescan, Explorer opening, and
  safe source unregistration;
- revision-safe sorting by capture date, creation date, modification date, and natural file name in
  ascending or descending order;
- temporary import progress connected to the persisted scan state;
- persisted theme, viewer, catalog-location, preview-location, preview-budget, and clear-preview
  settings with the safety explanations defined in section 4.11;
- durable orientation-corrected aspect-ratio evidence and a bounded preview lifecycle that preserve
  final gallery geometry through restart, cleanup, failure, regeneration, and eviction;
- responsive, keyboard, focus, scaling, and accessibility behavior.

R2b is delivered as small end-to-end slices after UI acceptance. A fixture-backed control, bridge
type, database query, or rendered screenshot alone does not complete a use case.

R2b owns two deliberately separate lifecycles:

- **Aspect-ratio evidence**: media inspection records orientation-corrected width and height with
  compatible source state and engine identity. Restart and preview-cache operations reuse those
  dimensions without decoding the source merely to recover layout. An unknown dimension uses one
  stable documented fallback until a complete newer catalog revision or compatible bounded
  geometry-evidence epoch supplies trustworthy evidence. Preview readiness never upgrades layout
  geometry on its own; first-time dimension recovery is coalesced and identity-checked separately.
- **Preview artifacts**: demand moves a compatible artifact through absent, pending, generating,
  ready, failed, stale, and evictable conditions without turning those conditions into layout state.
  Persistent representation may combine states where safe, but failure evidence, stale-publication
  guards, and recovery behavior remain explicit and testable.

R2b proves these contracts through initial scan, explicit rescan, restart, preview demand, cleanup,
and storage transitions. R2c reuses the same retain-or-invalidate semantics when it adds automatic
source-change detection; it does not create a second dimensions or preview lifecycle.

The preview-artifact lifecycle is complete only when all of the following hold:

1. viewer, visible, movement-direction-near, guard, and idle demand use the documented priority
   order with bounded generation and decode concurrency;
2. publication is atomic and generation-guarded against a newer query, catalog revision, source
   state, algorithm version, orientation contract, or requested size bucket;
3. compatible unchanged files and identity-proven renames or moves reuse artifacts, while content
   edits, same-path replacements, and incompatible algorithm or orientation contracts invalidate
   them without exposing stale pixels as current;
4. the preview index can account for artifact path, byte size, bounded size bucket, compatibility
   identity, and coarsened last-use evidence without writing persistent state on every scroll tick;
5. capacity uses a high watermark and a lower reclamation target so cleanup does not oscillate at
   the configured limit. Temporary and unreferenced files, obsolete algorithms, incompatible or
   superseded size variants, and then least-recently-used distant artifacts are reclaimed in that
   order;
6. the active viewer item, visible items, directional guard demand, and in-flight atomic publication
   are pinned for the current reclamation pass. Eviction never enters the pointer-to-scroll path;
7. startup reconciles reserved bytes, interrupted temporary files, missing ready files, and
   unreferenced artifacts in bounded work. A missing derived file returns to pending demand rather
   than becoming a permanent gallery failure;
8. manual cleanup and preview-location change expose progress, cancellation, completion, and
   failure honestly, preserve source media and durable dimensions, and leave one recoverable active
   storage configuration after restart;
9. size buckets and reclamation thresholds are selected from display-scale, quality, latency,
   storage, and churn measurements. They are bounded policy, not a per-pixel cache-key expansion;
10. fixed fixtures prove EXIF Orientation 1 through 8, unknown-dimension fallback and settled
    recovery, missing and failed previews, manual cleanup, automatic reclamation, restart recovery,
    and cache-boundary repetition without per-preview geometry churn or source-media mutation.

Within the currently accepted ADR 0005 lifecycle, resource-safety work comes first: artifact
accounting, bounded variants, high/low-watermark reclamation, stale-publication guards, and bounded
startup recovery. User-facing manual cleanup and preview-root transition follow only after that core
is stable. Moving either later workflow out of R2b requires an explicit amendment to ADR 0005; this
roadmap does not silently weaken an accepted architecture decision merely to shorten the stage.

R2b does not require every optional ADR 0014 scale adaptation to be enabled merely to complete a
migration checklist. Its acceptance policy is:

1. freeze the current wheel, time-rail, jump, and resize behavior as the comparison baseline;
2. run resource-bounded Profile and long-session observation against a retained catalog without a
   new real-root import;
3. record retained detail count, process working set, garbage collection, page-publication copy
   time, frame timing, programmatic scroll writers, and flat-manifest cost;
4. separately run a bounded, read-only, source-readable preview workload and record cold and warm
   preview latency, cache-byte growth, bucket demand and reuse, reclamation duration, regeneration,
   and boundary churn. A retained-gallery Profile that rejects source-media materialization and a
   catalog-parity run that leaves every preview pending do not satisfy this evidence;
5. implement and validate ADR 0005's preview lifecycle before enabling target cleanup, reclamation,
   or preview-root transition behavior; the accepted aspect-ratio contract remains fixed;
6. change any remaining performance structure only when it exceeds its recorded budget, one
   variable at a time;
7. compare every change with the frozen baseline and reject a nearby-return, reversal, distant-jump,
   resize, or native-input regression;
8. pass current-authorized real-library parity and Windows Release verification before closing R2b.

Profile, builds, tests, scans, and acceptance runs remain serial on the project workstation. They
reuse the retained catalog where the scenario permits, start with bounded durations, and stop at an
explicit memory or runtime limit. Resource exhaustion is neither product acceptance evidence nor a
reason to hide an unexecuted gate.

The timeline slice is accepted only when focused geometry and widget tests plus a real large-library
interaction run prove that dragging moves the gallery every frame, unloaded ranges materialize
without changing the global position, rapid reversals retain the latest target, no stale window
overwrites the current query, and source media remains untouched. Passing analysis or rendering the
rail without this interaction evidence is insufficient.

### R2c - Continuous directory synchronization and incremental indexing

Placement and dependency:

R2c begins only after the accepted R2b production gallery behavior is complete. It reuses the R1
catalog, scan, checkpoint, file-identity, metadata, preview, and atomic-publication foundations. It
must complete before R3 exact-duplicate analysis becomes the next active stage, because duplicate,
search, timeline, preview, and future classification evidence cannot be trustworthy when the
catalog silently lags behind the source directories.

R2c is a catalog-correctness stage, not a generic performance optimization and not a background
task-center product. It closes the distinction between:

- **incremental reconciliation already validated in R1**: an explicit complete rescan can reuse
  unchanged evidence and distinguish an edit, rename, replacement, and removal;
- **continuous synchronization introduced in R2c**: Ame detects source changes, durably schedules
  the minimum necessary reconciliation, publishes bounded deltas, catches up after downtime, and
  reports when it can no longer guarantee freshness.

R2c does not authorize a gallery hot-path, manifest, or navigation rewrite. It publishes stable
identity and catalog-revision changes through bounded application contracts; the accepted R2b
gallery decides how to preserve its logical anchor and visible interaction. Delivery slices R2c-A
through R2c-F established the first running-time synchronization and recovery workflow. Historical
R2c-G implemented direct-desktop USN catch-up, and R2c-H measured it honestly, but the target
workstation's normal desktop token could not open either journal. R2c-I through R2c-M then
implemented ADR 0023's non-privileged watcher-plus-startup-inventory replacement. That replacement
preserved source safety and eliminated automatic media scans, but live diagnostics proved that its
normal continuity path still enumerates every root after every process start and gives new changes
no reserved execution path. Its O(N) startup model is therefore historical implementation evidence,
not the accepted product direction.

ADR 0024 reopens R2c for R2c-N through R2c-R. These slices replace scan-driven continuity with one
Windows 11 x64 change-driven model: the live watcher owns reserved P0 low-latency delivery, an
installer-managed constrained USN journal broker owns P1 closed-process catch-up, and metadata
inventory is P2 only for one-time baseline establishment or a proven continuity gap. R3 remains
blocked until this replacement passes security, migration, source-safety, latency, installer, and
independent-audit gates.

User outcome:

After a folder has been added to Ame, images created, edited, deleted, renamed, or moved inside that
folder appear in the same unified gallery without waiting for old-library verification. If Ame was
closed, a supported local NTFS root replays only its missing persistent journal range after the live
watcher starts. A no-change normal startup enumerates zero source-root entries and opens no media.
If journal continuity is missing or no longer trustworthy, Ame retains the last trustworthy catalog
and runs one bounded metadata-only recovery while P0 live changes continue to publish. While an
automatic path can converge, the root reports `正在更新图库`; it reports `更新受阻` only when that
path has failed or exhausted. A permanently unsupported root is explicitly `LiveOnly` or manually
refreshed rather than being silently scanned on every startup.

The user continues to see one library rather than a separate synchronization application. Ordinary
wording is limited to concepts such as `已同步`, `正在更新图库`, `更新受阻`, `目录不可用`, and
`部分项目无法读取`. `需要核对` is not a product state. Terms such as watcher, queue, delta, adapter,
inventory epoch, and cursor belong only in diagnostic details.

#### R2c.1 Safety and authority rules

- Filesystem notifications, brokered journal records, and staged recovery inventory rows are hints
  that identify what must be checked. They are never accepted as the final file state.
- P0 live work has reserved execution capacity and may publish while P1 catch-up or P2 recovery is
  incomplete. Historical continuity is not a precondition for presenting a newly revalidated change.
- The filesystem plus Ame's already accepted identity and source-state revalidation remain the
  evidence used to reconcile the catalog.
- R2c observes and reconciles source state without changing it. It does not delete, move, copy,
  rename, rewrite, hydrate, or normalize any source file.
- Offline and recall-on-data-access placeholders are identified before content access. Continuous
  synchronization must not download a cloud-only file merely to classify an event.
- An unavailable root retains its last trustworthy catalog. Inaccessibility is not evidence that
  every location has been deleted.
- Only a completely reconciled path or subtree can authoritatively remove locations that are no
  longer present. A partial or failed pass cannot publish a complete-removal claim.
- A batch of related changes is visible at one catalog revision. The UI sees either the prior
  revision or the complete new revision, never a half-applied rename or replacement.
- Full-root scanning is permitted only for first import, the explicit `更新图库` action, or resumption
  of a foreground checkpoint created by either action. Every request carries one of these typed
  reasons.
- Normal create, modify, delete, rename, move, process start, watcher restart, evidence loss,
  overflow, retry, inventory size, availability recovery, and automatic reconciliation failure must
  not trigger a complete root scan.
- Ordinary startup with a continuous checkpoint must not enumerate source-root entries, create a
  metadata-inventory run, read media content, or hydrate a placeholder.
- The broker may read only journal metadata for caller-authorized roots. It must not read media,
  mutate source files, configure the journal, open the catalog, or disclose root-external records.

#### R2c.2 Ownership and boundaries

The Rust domain defines Ame-owned, platform-independent values for:

- library-root identity and configuration generation;
- normalized change intent, such as path reconciliation, rename candidate, subtree reconciliation,
  and root freshness unknown;
- change origin and priority lane, including P0 live notification, P1 brokered journal catch-up,
  P2 baseline or continuity-gap recovery, and user refresh; historical direct-desktop
  `StartupCatchUp`, `consistency_audit`, and ADR 0023 startup-inventory values remain readable only
  for forward migration and do not regain production authority;
- reconciliation outcomes: unchanged, added, modified, renamed or moved, replaced, removed,
  skipped, retryable failure, and terminal issue;
- watcher health and catalog-freshness states without exposing a Windows or third-party type.

The Rust application layer owns:

- starting and stopping change observation for configured, available roots;
- negotiating broker capability, validating per-root journal continuity, and scheduling shared
  per-volume reads without coupling root progress;
- converting raw signals into Ame change intents;
- durable enqueueing, enqueue-before-checkpoint, debounce, coalescing, priority isolation, retry,
  backoff, pause, cancellation, and recovery;
- deciding whether the minimum safe scope is one path, a subtree, root metadata reconciliation, or
  a complete scan;
- invoking the existing source-state, file-identity, metadata, and preview ports;
- atomically applying bounded catalog deltas and incrementing the catalog revision;
- precise retain-or-invalidate decisions for metadata, previews, fingerprints, similarity, and
  future classification evidence, expressed through stable asset identity rather than paths;
- publishing bounded status and revision events to Flutter.

Ports must remain narrow and should extend an existing natural boundary instead of creating a
second synonym for it. The implementation must at least evaluate these responsibilities:

- `LibraryChangeSource`: streams normalized hints and health transitions;
- `PersistentChangeJournal`: queries and streams bounded caller-authorized root changes without
  exposing Win32, service, volume, or raw USN types;
- `ChangeQueue`: durably records, leases, acknowledges, retries, and supersedes pending intents;
- `IncrementalReconciler`: checks a path or bounded subtree and returns Ame reconciliation results;
- `CatalogDeltaPublisher`: applies one batch at an atomic revision boundary;
- `MetadataInventory`: enumerates and pages current metadata evidence for an explicit P2 baseline or
  proven continuity gap without reading media content.

Names are illustrative, not mandatory APIs. Before adding a port, inspect whether an existing scan,
catalog, or filesystem contract already owns that responsibility.

Adapters own all platform and dependency details:

- evaluate the mature Rust `notify` ecosystem for recursive live observation on Windows and record
  its selected version, license, maintenance, cancellation behavior, overflow semantics, packaging,
  and replacement strategy before admission;
- keep `notify` event kinds, paths, errors, threads, and global state behind the adapter;
- keep service control, named-pipe, caller-token, volume-handle, journal-buffer, file-reference, and
  root-containment details inside the Windows broker adapter;
- continue using ADR 0007's Ame-owned Windows `FILE_ID_INFO` evidence for reconciliation instead of
  inventing another asset-identity rule;
- persist the durable queue, retry state, per-root journal checkpoint, covered range, cross-root
  lineage, inventory epoch and staging state, and delta publication through the Rust SQLite adapter;
- never reopen a volume from the desktop process or request UAC during ordinary startup. Service
  install and update belong to the signed installer and use explicit one-time administrative consent;
- keep Flutter presentation-only. Flutter does not watch directories, enumerate roots, write SQL,
  call the broker, interpret USN, or infer catalog policy from platform events.

#### R2c.3 Durable change intent

The logical persistent model must be able to express, without committing prematurely to one table
shape:

- a stable change ID and `root_id`;
- the root configuration generation so work for an unregistered or replaced root cannot publish;
- one affected relative path and an optional old path or rename-correlation identity;
- normalized intent kind and origin;
- P0, P1, or P2 lane and the source batch or journal range that owns the evidence;
- first-observed and most-recent-observed time;
- coalesced event count;
- pending, leased/in-progress, retry-wait, completed, and superseded states;
- attempt count, next retry time, and structured last failure;
- the catalog revision at enqueue and successful publication;
- the owning per-root journal checkpoint and covered boundary, or metadata-inventory epoch, scope,
  page cursor, and completion authority where applicable;
- deterministic terminal media evidence keyed by root, normalized path, complete source state, and
  inspection-engine identity so unchanged unsupported or malformed files are not decoded or retried
  again on every startup.

Per-root continuity state also binds root generation, volume identity, root file identity, journal
ID, next unread USN, covered catalog revision, broker protocol, and last structured failure. Roots
on one volume may share one bounded physical read, but a root advances only after its own work is
durably enrolled. No all-roots transaction may let one unavailable root block another root's P1
checkpoint.

This state is durable task data, not disposable thumbnail cache. Its schema changes require forward
migrations from every committed schema version and migration tests. Completed rows and obsolete
watermarks require a bounded retention strategy, but cleanup must never erase an unresolved gap or
user-owned decision.

#### R2c.4 Event normalization and coalescing

Raw filesystem events may be duplicated, reordered, incomplete, or delivered after the path changes
again. The inbound callback must remain lightweight: it normalizes and enqueues a hint without
running image decoding, a long SQLite transaction, a directory walk, or Flutter work on the callback
thread.

Live observations enter P0. P0 owns reserved admission, leasing, revalidation, and publication
capacity that P1 journal replay and P2 inventory cannot consume. P1 and P2 check for higher-priority
work at every bounded page and yield before beginning another page. Persisting `priority` without
that execution isolation is insufficient.

After a short, testable stabilization window, apply at least these rules:

- `create` followed by any number of `modify` events becomes one reconcile-or-add intent;
- repeated `modify` events for one path become one reconciliation;
- `create` followed quickly by `delete` is checked against final filesystem state rather than
  assumed to be a no-op;
- a reliably paired `rename(old, new)` is handled as one atomic intent;
- an unpaired rename degrades to an old-path reconciliation and a new-path reconciliation;
- a directory rename, move, or removal marks the minimum affected subtree instead of materializing
  an unbounded event for every known descendant;
- a stronger parent-subtree intent supersedes unleased child-path intents;
- a later event for the same path prevents an earlier leased result from overwriting newer state;
- application-owned catalog, preview, temporary, log, and model storage is excluded and must not be
  located inside a source root in the first place.

An initial debounce range may be measured around 350–1000 ms, but the final value must be justified
by controlled event-burst evidence rather than copied as a permanent constant. In-memory ingress,
database leases, batch size, retry count, and concurrency must all be bounded. Under a storm Ame may
be delayed; it must not grow memory without limit or silently drop events while claiming `已同步`.

#### R2c.5 Incremental reconciliation

For every stable path or subtree intent:

1. Verify that the root still exists in Ame, its configuration generation matches, and its current
   availability permits inspection.
2. Inspect root and path metadata before content. Distinguish missing, directory, regular file,
   offline placeholder, inaccessible, locked, and unsupported states.
3. Stop before content access for offline or recall placeholders and preserve explicit availability
   evidence.
4. For a locally readable candidate, reuse the existing format evidence, source state, optional
   Windows file identity, and metadata compatibility rules.
5. Compare against the current published location using ADR 0007's order of evidence.
6. Reuse derived evidence only when source state and engine identity remain compatible. Otherwise
   invalidate only what can no longer be trusted.
7. Revalidate required identity and state immediately before publication. If the file changed again,
   return the intent to the queue instead of publishing stale evidence.
8. Publish the complete bounded batch and one new catalog revision in a single transaction.
9. Admit only formats supported by the pinned decoder. Unsupported formats, malformed content, and
   decoder-limit violations complete once as terminal per-file evidence; only open, lock, read, and
   source-race failures consume retry attempts.

Required semantics:

- New local file: add a location; do not infer permanent logical identity from its path.
- Unchanged file: retain orientation-corrected dimensions and compatible preview artifacts; do not
  repeat metadata analysis or preview generation and do not create a meaningless visible refresh.
- In-place edit: preserve logical asset identity when accepted platform evidence supports it, while
  invalidating stale dimensions, preview, metadata, fingerprint, similarity, and classification
  evidence. Continue publishing the last trustworthy revision until replacement dimensions and the
  complete bounded delta can publish atomically.
- Same-volume rename or move: preserve the asset when identity matches and replace its location
  atomically; compatible dimensions and preview artifacts follow the stable identity instead of
  remaining attached to an obsolete path.
- Replacement at the same path: create a new asset and prevent it from inheriting the former
  file's dimensions, preview artifacts, other derived evidence, or user decisions.
- Removal: remove the published location only after an authoritative observation; do not let a
  delayed delete remove a new replacement now occupying that path. When the last active location is
  authoritatively removed, current derived projections must no longer surface the asset and its
  unreferenced previews become eligible for bounded reclamation.
- Cross-volume move: treat delete and create evidence conservatively unless a separately admitted
  stronger identity proves continuity; never transfer classification merely because names match.
- Directory change: enumerate only the minimum subtree in bounded windows. Absence is authoritative
  only for the scope that completed successfully.

Full scans continue to stage and atomically replace a complete root snapshot. Incremental work uses
atomic delta publication but must retain the same trust rule: failed, cancelled, stale, or partial
work does not replace trustworthy state.

#### R2c.6 Query, preview, and presentation consistency

- Every published delta increments the same catalog revision used by bounded keyset queries.
- Existing stale-cursor protection remains authoritative. Flutter handles a revision change through
  an Ame-owned refresh contract rather than querying SQL or rebuilding the whole application.
- Stable asset and location identity is used to merge a bounded update while preserving the active
  source, filters, sort, selection, preview, and visible scroll anchor when possible.
- A rename must not briefly appear as both a removed tile and an unrelated new tile.
- An edited visible image invalidates and recreates only the necessary preview; off-screen previews
  remain bounded and demand-driven.
- A dimensions change publishes with the same atomic catalog revision as its source-state change.
  Flutter assembles the replacement manifest and layout snapshot separately, keeps the last
  trustworthy geometry until the replacement is complete, and preserves a compatible logical
  viewport anchor. It never clears a tile to a transient square merely because reinspection or
  preview generation is pending.
- Preview demand and publication carry compatible location, source-state, revision, algorithm,
  orientation, and size-bucket identity. A late result may populate only the matching preview entry;
  it cannot restore an obsolete path, overwrite newer evidence, or mutate layout dimensions.
- Every bounded delta exposes enough stable identity and evidence disposition for later analysis
  consumers to retain compatible results after a rename, invalidate them after content change or
  replacement, and remove them from current projections after authoritative deletion. R2c defines
  this contract without implementing R5 classification.
- If the currently previewed file is removed, replaced, unavailable, or offline, the viewer presents
  a clear state and a safe return path instead of displaying stale bytes as current.
- Synchronization remains part of the existing library and source workflow. It does not create a
  sidebar Task entry or a second gallery.
- `更新图库` explicitly requests an application-owned full scan for the selected root. Automatic
  watcher and journal reconciliation remain the normal incremental path; exceptional metadata
  inventory remains application-owned P2 recovery. Flutter does not enumerate files.

#### R2c.7 Lifecycle and race handling

Startup order:

1. Load the last trustworthy catalog, root configuration, unresolved change queue, and any
   recoverable foreground explicit full-scan checkpoint. Retire prerelease
   `authoritative_recovery` full-scan checkpoints without resuming them.
2. Check each root's availability using metadata only.
3. Establish live observation before continuity work so new events do not open another avoidable
   gap.
4. For each broker-capable local NTFS root, query the current journal identity and exclusive end
   boundary. If its checkpoint is continuous, enqueue only the missing P1 range and perform no root
   enumeration when that range is empty.
5. Process P0 immediately while P1 catches up. Advance a root checkpoint only after its bounded
   range is durably enrolled; report `已同步` only after that checkpoint covers the declared boundary
   and required queue lineage is terminal.
6. Start P2 metadata inventory only for a one-time migration or first-authority baseline, or after a
   specific continuity failure. Keep P0 live during recovery, replay the closing journal interval,
   and require complete scope authority before publishing absence.
7. Project an unsupported persistent-change source as `LiveOnly` or an explicit blocked capability;
   never compensate with an automatic complete inventory on every process start.

Root changes:

- A newly added root completes its first trustworthy full scan before live deltas are applied to the
  published result; events arriving during the scan wait behind that publication boundary.
- Removing a root stops observation and invalidates its old generation. Unregistering a source from
  Ame never deletes or modifies its files.
- A changed root path or policy receives a new generation so old queued work cannot publish into the
  new configuration.
- An offline or disconnected root pauses processing. It does not publish mass removals.

Shutdown:

- stop accepting new live callbacks;
- immediately hide the desktop window so background teardown cannot present as an application hang;
- suspend a running foreground full scan at a durable checkpoint so the next process can resume it;
- cancel watcher, metadata inventory, path, subtree, and bounded root work; on the next start
  establish a new live watcher boundary and continue from the last durably advanced per-root journal
  checkpoint;
- safely return any currently leased non-scan batch instead of treating its old in-memory state as
  evidence for changes that may occur while Ame is closed;
- persist foreground full-scan checkpoints and durable catalog/queue state, but discard superseded
  inventory staging after replacement authority exists; preserve journal checkpoints only through
  their last durably enrolled boundaries;
- use bounded graceful shutdown so a watcher or queue cannot hang the window close path;
- leave foreground full scans and durable queue work recoverable on the next startup without keeping
  the window visible.

#### R2c.8 Failure and degradation matrix

- Unsupported, malformed, or decoder-limit media: record one structured terminal issue, persist its
  source-state and engine-version evidence atomically with completion, and continue the batch without
  consuming retry attempts.
- Temporarily unreadable or locked file: preserve the last trustworthy location and retry with
  bounded backoff; a later successful or terminal result closes the row.
- File changes again during processing: fail final revalidation, coalesce the newer event, and retry.
- Notification buffer overflow or known event loss: mark live observation degraded and replay the
  continuous P1 journal interval. Start P2 only if journal evidence cannot cover the gap.
- Watcher failure: restart with bounded exponential backoff while P1 preserves continuity; do not
  start inventory merely because the watcher restarted.
- Broker protocol, permission, or service failure: retain the checkpoint and cached catalog, keep
  P0 live when possible, and retry within a bounded policy. A root whose automatic persistent path
  is unavailable becomes `LiveOnly` or `更新受阻`; it is not silently scanned on every startup.
- Journal ID change, trimmed checkpoint, unsupported record, root/volume identity mismatch, or
  unprovable containment: start one explicit P2 baseline or recovery epoch while keeping P0 live.
- Root offline, disconnected, or inaccessible: retain its catalog and display availability status;
  do not reinterpret failure as deletion.
- Database transaction failure: roll back the entire delta, keep the intent retryable, and do not
  increment catalog revision.
- Huge directory rename or removal: process descendants through durable inventory pages; do not keep
  every row in memory or claim complete removals until the scope completes.
- Inventory failure or repeated source races: preserve the last trustworthy catalog and durable
  authority, then report `更新受阻` with one structured root error instead of starting a full scan.

Escalation order is:

```text
P0 live path or subtree reconciliation
-> P1 persistent journal catch-up
-> P2 pageable metadata-inventory recovery for a proven gap
-> blocked or LiveOnly state when complete automatic continuity cannot converge
```

The application must expose which level is in progress and why without leaking implementation
jargon into normal UI copy.

#### R2c.9 Persistent catch-up and exceptional metadata recovery

ADR 0024 replaces ADR 0023's routine startup inventory. Every supported local NTFS root establishes
the live watcher first, then uses the constrained journal broker to validate and replay its missing
per-root USN range. Roots on one volume share a bounded physical read, but each root enrolls work and
advances independently. A failed or unavailable root cannot hold another root's P1 progress.

Checkpoint advancement follows durable enqueue. The application captures an exclusive journal end
boundary, validates volume, journal, root generation, broker protocol, root-set containment, and
catalog authority, durably enrolls the bounded normalized plans, then advances only that root's
checkpoint. A crash before advancement rereads an idempotent range. A raw journal reason never
authorizes catalog removal; every candidate passes the existing final-state reconciler.

The broker is installed and updated with explicit administrative consent, but Ame remains an
ordinary-user process and shows no UAC during normal startup. The broker reads existing journal
metadata only. It never reads media, opens the catalog, changes the journal, modifies source files,
hydrates placeholders, accepts arbitrary volume access, or returns records outside a caller-
authorized configured root. Versioned bounded IPC, caller-token and root verification, pipe DACL,
service SID, minimum proven privileges, cancellation, and root-external disclosure tests are release
gates.

P2 metadata inventory is authorized only for a one-time migration or first-authority baseline, or a
proven gap such as journal recreation or trimming, root/volume identity mismatch, reconstruction or
containment ambiguity, incompatible protocol or record version, or simultaneous watcher and journal
coverage loss. The inventory:

- records normalized path, entry kind, size, modification evidence, Windows file identity when
  available, and placeholder attributes;
- never reads media bytes, decodes images, generates previews, follows an untrusted reparse directory
  outside the root, or hydrates a cloud placeholder;
- stages derived evidence in application storage and compares it with the published catalog in
  bounded pages;
- yields to P0 and P1 at every page boundary;
- may publish additions and modifications early only after path final-state revalidation;
- publishes an absence only after the complete owning scope and closing journal boundary succeed;
- accepts and immediately publishes independent P0 live work throughout the run; and
- fails closed without mass removal when partial, cancelled, raced, unavailable, or incomplete.

A continuous no-change checkpoint creates no inventory run and enumerates exactly zero source-root
entries. A permanently unsupported root becomes `LiveOnly` or is refreshed explicitly; routine O(N)
startup inventory is not a compatibility fallback.

#### R2c.10 Explicit refresh and recovery triggers

Ame does not schedule a periodic full-root consistency audit. A fixed seven-day interval adds
unbounded work without supplying evidence that anything changed. Continuous freshness instead uses
the P0 live watcher, P1 persistent journal, durable queue, and P2 inventory only when a specific
baseline or continuity-gap authority exists.

A complete root scan may start only when one of these authorities exists:

- a root is imported for the first time;
- the user explicitly selects `更新图库` for that root;
- a foreground full scan created by first import or explicit refresh is resumed from its durable
  checkpoint.

Normal file events, normal process restart, elapsed time, retry, watcher interruption, overflow,
an empty journal range, metadata-inventory size, and automatic recovery failure do not authorize a
complete scan. Work that exceeds the 4,096-entry or 128-path batch ceiling continues through bounded
P1 or P2 pages. Legacy direct-desktop `StartupCatchUp`, `consistency_audit`, prerelease
`authoritative_recovery` full-scan checkpoints, and ADR 0023 startup epochs are migration input only;
they cannot bypass the ADR 0024 broker, checkpoint, lane, or baseline contract. The last trustworthy
catalog remains visible while automatic continuity runs.

#### R2c.11 Delivery slices

R2c-A - contracts and deterministic fixtures:

- map existing scan, catalog, bridge, and Flutter ownership before editing;
- define normalized intent, reconciliation result, root generation, and freshness states;
- add domain/application tests for create, modify, rename, replacement, removal, directory changes,
  duplicate/late events, offline roots, Chinese paths, long paths, and event storms;
- record dependency and architecture decisions.

R2c-A is complete only when the behavior can be tested without a platform watcher and the UI is not
asked to infer business rules.

Status: **complete on 2026-08-13**. ADR 0016 owns the platform-independent contract and
deterministic fixtures; ADR 0017 records the R2c-B watcher decision without adding an unused
dependency in R2c-A.

R2c-B - live Windows observation:

- add the admitted recursive watcher adapter;
- connect one bounded lifecycle per available root;
- keep callbacks lightweight and cancellable;
- verify start, root removal, adapter failure, and window-close shutdown behavior.

R2c-B is complete only when controlled real filesystem changes produce Ame-owned intents without
blocking UI, decoding media in the callback, or growing memory without limit.

Status: **complete on 2026-08-13; audit-hardened on 2026-08-14**. ADR 0017 owns the admitted Windows adapter and
`docs/acceptance/r2c-b-windows-observation.md` records focused, controlled-filesystem, Daily, and
Windows Release evidence. No real-library root was accessed.

R2c-C - durable queue and coalescing:

- add forward migration and durable leasing/retry storage;
- implement debounce, path/subtree supersession, root-generation protection, crash recovery, and
  bounded cleanup;
- expose structured queue health and delay metrics.

R2c-C is complete only when an application terminated after enqueue resumes the same work and a
burst of repeated notifications produces the minimum necessary reconciliation.

Status: **complete and audit-hardened on 2026-08-17**. ADR 0018 owns the schema v17 leased SQLite
queue and `docs/acceptance/r2c-c-durable-change-queue.md` records migration, restart, coalescing,
stale-lease, retry, metrics, bounded-retention, Clippy, and Daily evidence. No real-library root was
accessed.

R2c-D - incremental delta publication:

- connect the existing file-identity and media-safety rules;
- implement unchanged, add, edit, rename/move, replacement, and removal transactions;
- invalidate only incompatible derived evidence;
- increment revision and refresh bounded UI state atomically.

R2c-D is complete only when every fundamental change is reflected without a normal root-wide scan,
failed transactions leave the old catalog unchanged, and source media remains untouched.

Status: **complete on 2026-08-18**. ADR 0019 owns identity-aware path reconciliation and the
generation-, revision-, lease-, and full-scan-guarded SQLite delta transaction.
`docs/acceptance/r2c-d-incremental-delta-publication.md` records unchanged, add, edit, paired
rename, recreated-old-path and case-only rename, identity backfill, rename-followed-by-removal,
same-path replacement, authoritative removal, preview ownership and cleanup races, filesystem-link
containment, bounded maintenance, failure isolation, rollback, source-byte, Clippy, and Daily
evidence. Subtree, root, and freshness-gap work remains durable and unleased by R2c-D for R2c-F
authoritative reconciliation rather than publishing a partial removal claim.

R2c-E - production UI and lifecycle:

- start and stop synchronization with the desktop application;
- connect simple Chinese freshness and degraded states;
- connect `更新图库` to the application use case;
- preserve active source, filters, selection, preview, and gallery scroll anchor through a bounded
  refresh.

R2c-E is complete. `docs/acceptance/r2c-e-production-ui-lifecycle.md` records production observer
start and stop, bounded root freshness snapshots, live path publication, stable-asset gallery refresh,
selection and viewer continuity, Chinese source status, idempotent bounded shutdown, bridge generation,
complete Daily, and Windows release evidence. The real user path no longer requires a permanent task
entry or manual re-import for ordinary supported path changes. The 2026-08-20 presentation hardening
keeps compact source status in the sidebar, routes synchronization and reconciliation detail through
one bounded notification queue and icon-only history control, and resets a newly selected Library,
source, or child-folder scope to its first result instead of reusing the prior scope's time anchor.

R2c-F - recovery and consistency:

- force overflow, watcher failure, offline roots, database rollback, and repeated source changes;
- implement the bounded escalation ladder and explicit full-scan trigger contract;
- prove that recovery does not publish mass false removals or claim health early.

Status: **complete and audit-hardened on 2026-08-18; corrective hardening verified locally on
2026-08-20**. ADR 0021 owns the bounded authoritative
subtree/root worker, schema v18 full-scan generation and queue-watermark coordination,
previous-snapshot preservation, background escalation, and bounded retry. Production isolates a
live authoritative lease from foreground path polling, rotates due
bounded work across roots, and preserves migrated v17 location identifiers during incremental
identity backfill. `docs/acceptance/r2c-f-recovery-consistency.md` records the original controlled
fixtures plus post-integration hardening. The 2026-08-20 correction separates structural
full-scan incompleteness from isolated media decode failures, atomically publishes exact path
retries, and projects an active authoritative scan as updating. The current local Rust suite is
405 tests total with 398 passing and seven explicit ignores. The refreshed complete Daily also
passed every Flutter test file, Windows Scan 2/2, Windows Accessibility 2/2, bridge compatibility,
release guardrails, formatting, analysis, and whitespace. Hosted closeout remains required before
the updated PR head is merged. The 2026-08-21 corrective working tree additionally separates a
recoverable event-evidence gap from native watcher failure, keeps observation live during automatic
authoritative recovery, narrows known-path metadata races to subtree work, and reserves `更新受阻`
for blocked recovery. It also treats a bounded set of exact file changes discovered during
authoritative full-scan finalization as path retries: the last trustworthy location is preserved or
an unverified new location is omitted, while the independent root snapshot publishes instead of
restarting a target-scale scan from zero. Structural path-set gaps and excessive exact-path races
remain fail-closed. The deterministic finalization-race fixture passes; the refreshed complete gates
for the full corrective working tree have not yet been run.

R2c-G - historical USN downtime catch-up:

- accept a focused ADR;
- implement per-volume watermarks, continuity validation, root filtering, candidate enqueueing, and
  explicit fallback;
- validate changes made while Ame is closed.

Historical status: **implemented and audit-hardened on 2026-08-19; superseded for production by ADR
0023 on 2026-08-21**. ADR 0022 records watcher-first bounded Windows USN catch-up, schema v19
checkpoints and durable cross-root handoff lineage, explicit authoritative fallback, exact-case
reconstruction, preview ownership, and fail-closed prerelease repair.
`docs/acceptance/r2c-g-usn-downtime-catch-up.md` records controlled fixtures, 391 Rust tests with
five existing explicit ignores, all Flutter tests, both Windows integration suites, the Windows
Release and 10,000-file synthetic performance gates, and final independent approval with no
remaining findings. Direct journal candidates remained unavailable to the standard workstation
token; permission fallback passed without elevation or source mutation. Production must stop using
this adapter, and schema v19 remains migration input only.

R2c-H - large-library reliability:

- run small and synthetic correctness fixtures first;
- then use the already authorized real roots in read-only mode, serially and with isolated derived
  storage;
- measure idle overhead, event-to-visible P50/P95 latency, event-storm coalescing, persistent queue
  growth, transaction time, startup catch-up, memory, database growth, cancellation, and controlled
  recovery; target-root authoritative convergence timing is deferred to extended R10 evidence;
- verify source bytes, source entries, and cloud-placeholder state remain unchanged.

Status: **complete and audit-hardened on 2026-08-19**. The final controlled Windows observer run
recorded 35 ms event-to-visible P95, bounded coalescing and restart recovery, and the authorized
two-root read-only rerun preserved 85,556 source entries plus 32 deterministic byte samples. Its
isolated catch-up honestly recorded two `usn_volume_open_failed` fallbacks without elevation,
leasing, publication, placeholder hydration, or source mutation. Physical path aliases are rejected
before any write, Cargo and descendants remain inside one kill-on-close Job Object, and hash reads
are bounded after opening. `docs/acceptance/r2c-h-large-library-reliability.md` records 402 Rust
tests with seven explicit ignores, all Flutter and Windows integration gates, Windows Release,
10,000-file synthetic performance, and final independent approval with no remaining findings.
The target-root phase intentionally did not execute authoritative leases or publication, so its
queue and storage measurements are not an end-to-end recovery-time claim.

R2c-I - non-USN contract and production cutover:

- accept ADR 0023 and mark ADR 0022 as superseded;
- remove USN initialization, scheduling, checkpoints, and permission fallback from production;
- keep schema v19 readable and preserve catalog, asset, preview, and user-decision authority;
- add typed full-scan reasons and reject every non-allowlisted automatic request.

R2c-I is complete only when production never opens a journal or requests elevation, existing v19
catalogs migrate safely, and automatic startup or recovery cannot start a full scan.

Status: **complete on 2026-08-21**. Production no longer constructs or schedules USN catch-up,
startup continuity starts after the watcher is healthy, evidence gaps cannot create a scan run,
oversized authoritative work returns `metadata_inventory_required`, and only user-requested or
checkpoint-resume paths can enter the full scanner. Schema v19 migration and handoff compatibility
remain intact. `docs/acceptance/r2c-i-non-usn-cutover.md` records the focused, Daily, and Windows
Release evidence.

R2c-J - metadata-inventory persistence and discovery:

- add the forward inventory-run and staging migration with exact-shape validation;
- enumerate path, kind, size, modification, identity, and placeholder attributes in bounded pages;
- compare against the active catalog without decoding media or reading source bytes;
- publish positive candidates through final-state path reconciliation and delay absence until the
  complete scope is authoritative.

R2c-J is complete only when controlled closed-process create, modify, delete, rename, directory move,
placeholder, Chinese-path, and long-path changes converge without a media scan or source mutation.

Implementation status: schema v20 now preserves the v19 queue and lineage while adding exact-shape
inventory run/staging authority and the `metadata_inventory` queue origin. The local adapter emits
fixed-bound metadata pages without content reads, media filtering, placeholder identity opens, or
reparse-directory traversal. Controlled closed-process fixtures cover additions, modifications,
deletions, file and directory moves, placeholders, Chinese and long paths, missing subtrees,
hard-link rename pairing, cancellation, and durable failure termination. Production epoch and
paging scheduling remain R2c-K. Focused tests, the 439-test Rust suite, repository lint, complete
Daily, and Windows Release gates pass; the final independent audit found no remaining findings, so
R2c-J is accepted.

R2c-K - pageable recovery and continuity epochs:

- make cold start, availability recovery, watcher restart, rescan, incomplete rename, and overflow
  create or extend metadata-inventory epochs;
- replace the 4,096-entry / 128-path automatic full-scan escalation with durable inventory paging;
- coalesce live events with staged evidence and supersede interrupted non-scan authority on restart;
- preserve exact removals, root generation, catalog revision, identity handoff, retry, and fairness.

R2c-K is complete only when oversized and racing scopes remain bounded, no partial absence publishes,
and every automatic failure either converges or remains durably `更新受阻` without a full scan.

Implementation status: production now allocates root-generation inventory epochs atomically,
routes startup and evidence-gap authority into metadata inventory, converts oversized subtree work
into durable inventory continuation, and yields after one comparison or absence page. Candidate
publication protects the exact authority lease, applies bounded backpressure without advancing a
cursor or consuming an attempt, and rejects output after a newer watcher gap supersedes the worker.
Focused epoch, overflow, cancellation, retry-exhaustion, capacity-one, fairness, and no-full-scan
fixtures pass together with the complete 452-test Rust suite. Cloud Files identity, exact-name
enumeration evidence, local availability, no-recall source opening, and matching unavailable
metadata now remain safe without hydration, removal work, or retry exhaustion. Repository lint,
complete Daily, and Windows Release gates pass. Final independent audit reported no Critical, High,
Medium, or Low findings. R2c-K is accepted.

R2c-L - lifecycle, presentation, and diagnostics:

- map product status to `已同步`, `正在更新图库`, `更新受阻`, or `目录不可用` and remove `需要核对`;
- keep normal update, retry, and success out of notifications while deduplicating blocked errors;
- expose phase, elapsed time, bounded counts, and issue code in development diagnostics;
- hide the window immediately, resume only foreground full scans, and restart every non-scan
  continuity epoch.

R2c-L is complete only when status cannot oscillate during automatic retries, blocked detail is
actionable, normal synchronization is silent, and shutdown never leaves a visible stalled window.

Implementation status: the typed synchronization snapshot now exposes watcher startup, inventory
enumeration, inventory comparison, queue publication, retry wait, bounded reconciliation, full
scan, blocked, synchronized, and unavailable phases. Flutter retains phase start time per root
generation and emits structured development diagnostics with elapsed time, bounded queue counts,
source status, and issue code. Durable exhausted work retains its diagnostic code across restart,
and active recovery follows a typed blocking flag instead of inferring state from code strings or
treating a protected live worker's nominal lease expiry as a product failure.
Normal update, retry, and convergence remain silent; blocked cause changes update one
root-and-generation-keyed notification in place with phase, current elapsed time, source path,
counts, and technical code. Existing shutdown ownership hides the window before waiting, cancels
non-scan work, and preserves only foreground full-scan checkpoints. Focused tests, repository lint, complete
Daily with 454 Rust tests and all Flutter/Windows integration partitions, and Windows Release pass
on the audit-fix head. Final independent audit reported no Critical, High, Medium, or Low findings.
R2c-L is accepted.

R2c-M - replacement reliability and closeout:

- repeat controlled Windows event-to-visible, storm, restart, overflow, cancellation, and source-
  safety tests;
- measure metadata-only startup continuity against the retained approximately 79,000-location
  workload without media reads or placeholder hydration;
- enforce event-to-visible P95 no greater than one second and an initial per-root metadata target no
  greater than 45 seconds on the recorded workstation;
- run migration, complete Daily, Windows Release, final independent audit, and final PR review.

Historical ADR 0023 status: **reopened on 2026-08-21 under ADR 0023**. R2c-A through R2c-F remain accepted
foundations. R2c-G and its USN-specific acceptance remain historical implementation and migration
evidence. R2c-H remains a valid source-safety and baseline measurement record but does not validate
the replacement continuity model. R2c-I through R2c-M retain their recorded implementation and
target-scale replacement evidence. The 2026-08-22 integration review identified two remaining
correctness gaps: ordinary watcher work could replace an active metadata-inventory control
authority, and a Windows source handle was not rechecked against the canonical root after an
ancestor-junction change. Both remediations are implemented and pass the complete local Daily and
Windows Release gates. Subsequent live diagnostics exposed a separate startup migration gap:
prerelease `authoritative_recovery` full-scan checkpoints could still be resumed before metadata
inventory. The production resume path is now removed, running and paused historical automatic
checkpoints are retired while preserving the active catalog, and only foreground checkpoints remain
resumable. Runtime diagnosis then identified two deterministic convergence defects: generic
ISO-BMFF signatures admitted MP4 and MOV files into image inspection, and an unrecognized reparse
directory aborted otherwise complete sibling inventory. Image admission now follows only the
pinned decoder's real formats and exact signatures; deterministic non-media, malformed-media, and
decoder-limit results complete once, persist atomically in schema v21, and are reused across restart
only while complete source and engine evidence still match. Transient open, lock, read, and race
failures remain retryable. Reparse directories are no-follow opaque leaves and no longer discard
sibling progress. Focused migration, restart-reuse, source-change invalidation, reparse, and mixed
96-video/32-malformed-image convergence fixtures pass. The complete Daily gate passes with 475 Rust
tests total, 464 passed and 11 explicitly ignored, all Flutter tests, Windows Scan 2/2, and Windows
Accessibility 2/2; Windows Release and packaged bridge smoke pass 2/2. A new independent audit has
not yet been run for this remediation. The changes remain local until an authorized commit and PR
update; the R2c branch remains unmerged and R3 remains paused.

ADR 0024 supersedes this closeout as the current production direction. The results above remain
valid implementation, migration, and source-safety evidence but do not prove a change-driven normal
startup, persistent closed-process catch-up, P0 isolation, broker security, or installer lifecycle.

R2c-N - change-driven decision and broker security feasibility:

- accept ADR 0024, mark ADR 0023 superseded, and reconcile ADR 0015, ADR 0017, ADR 0020, and ADR
  0021 with the new service, lifecycle, and recovery boundaries;
- map the retained direct-USN implementation into reusable parser, identity, checkpoint, handoff,
  and migration components without restoring direct desktop volume access or its global worker;
- build a minimal Windows 11 x64 broker PoC against disposable local NTFS roots;
- prove ordinary Ame can query an existing journal through bounded versioned IPC without per-start
  UAC, media reads, source writes, journal configuration, placeholder hydration, or root-external
  record disclosure;
- record the exact service account, required privileges, service SID, pipe DACL, caller-token/root
  validation, service binary ACL, cancellation, logging, and no-network security contract;
- stop before product integration if the required access cannot be achieved without an
  unacceptably broad service interface.

R2c-N is complete only when the disposable-root PoC and independent security review admit a
specific minimal boundary. An ADR, compilable stub, or broad `LocalSystem` proof is insufficient.

Status: **completed on 2026-08-23**. The bounded protocol, caller/root proof chain, fail-closed
service seam, cancellation and terminal delivery model, single-owner client dispatcher, lifecycle
linearization, bounded maintenance, and pending backoff passed focused verification and an
independent eleven-round security review. The admitted seam may now support R2c-O; it is not itself
a service, installer, product integration, or release acceptance result.

R2c-O - constrained broker and installer foundation:

- implement the x64 demand-start Windows service and Ame-owned `PersistentChangeJournal` adapter;
- implement versioned length-delimited named-pipe framing, bounded streaming, timeouts,
  cancellation, structured codes, protocol negotiation, and safe service stop;
- validate caller identity, configured-root access, volume/root identity, path containment, and
  root-scoped output inside the broker before returning any record;
- confine and test every token, service, pipe, handle, buffer, parser, and `unsafe` invariant from
  ADR 0024;
- add the signed installer lifecycle required to install, repair, atomically update, version-check,
  stop, and remove the broker; keep the user-writable portable ZIP explicitly `LiveOnly` even when
  an installed broker exists;
- prove protocol mismatch or broker absence leaves the cached catalog usable and cannot trigger a
  hidden startup inventory or source mutation.

R2c-O is complete only when service security, installer upgrade/uninstall, portable degradation,
package loading, Daily, Windows Release, and independent security audit gates pass.

Implementation checkpoint on 2026-08-23: the production boundary now retains an object-safe,
Ame-owned persistent-journal session with root registration, query, bounded read, cancellation, and
close, or an exact `LiveOnly` reason. Production startup no longer manufactures metadata-inventory
authority merely because a root entered the runtime; deterministic BrokerAbsent and
ProtocolMismatch product fixtures keep the watcher active while proving zero queued recovery and
zero metadata-inventory runs. At that checkpoint no R2c-P schema had been introduced. R2c-O remains
active until the complete non-elevated gates, disposable elevated acceptance, and the required
independent security re-review pass.

Security-remediation checkpoint on 2026-08-23: production admission now uses a protected fixed
Application/broker identity, bounded kill-on-close identity probe, caller-token/root-handle
capability, parent-specific path semantics, live-handle revalidation, two listeners, global worker
and root bounds, crash-recoverable installer ownership markers, and handle-anchored no-follow cleanup.
Acceptance accepts only an externally pre-signed protected V1/V2 bundle pair with a distinct broker
hash and has no repository path for certificate creation or trust-store mutation. Deterministic
non-elevated gates cover repair, real-version upgrade, rollback,
limited-client replacement, listener/worker bounds, stop, case semantics, root replacement,
reparse rejection, and protocol degradation. Real SCM/FSCTL acceptance and the independent security
re-review remain required; this checkpoint does not declare R2c-O complete.

Independent-review remediation checkpoint on 2026-08-23: the production client now decides broker
eligibility from its protected fixed `ProgramFilesX64` Application identity before constructing or
connecting the factory, and the transport repeats that check before any SCM demand-start or pipe
operation. Deterministic portable and installed-mode fixtures prove zero and one factory connection
respectively, including the boundary where a broker may already be installed. Uninstall now writes
its protected marker before the first tree or ACL mutation and retains it through service, binary,
Application, tree, transaction-created ancestor, and parent-prestate cleanup; the original eight
install/repair/upgrade fault cases remain. A transaction-level matrix now crashes the complete
uninstall and resumes it through dead-owner recovery before and after each of seven mutations and
after each completion write, from both present-tree and initially missing-tree states. It also
covers committed and final marker-removal boundaries, exact parent prestate, and fail-closed
contradictory markers. Persisted phase, pending-operation, and prefix flags permit only the
protected-tree postcondition to be superseded by a proven later install-tree deletion; the other six
operations retain their own postcondition verification. Volume-wide unrelated USN storms return bounded,
strictly advancing incomplete pages at whole-record boundaries without exposing root-external names
or paths; repeated calls cover the captured end. The focused Rust modules, exactly counted broker
integration matrix, installer and portable release guardrails, and non-accessing acceptance
guardrails pass locally. Full Daily and Windows Release gates, real elevated SCM/FSCTL acceptance
with externally pre-signed bundles, and independent security re-review remain outstanding; R2c-O
remains active.

R2c-P - persistent per-root journal continuity:

- add a forward migration for versioned broker capability, per-root volume/root/journal checkpoint,
  covered range, source batch, and bounded cross-root handoff authority;
- validate exact schema shape, generation authority, checkpoint bounds, journal ID, volume identity,
  protocol version, relational ownership, and lossless USN/file-reference storage on catalog open;
- share one physical read for roots on a volume while enrolling and advancing each root independently;
- enforce enqueue-before-checkpoint and idempotent replay across every injected crash boundary;
- reconstruct and filter root-scoped V2/V3 records conservatively, pair supported renames and
  cross-root moves, and send every candidate through final-state reconciliation;
- migrate historical v19 journal and v20/v21 inventory authority without losing catalog, preview,
  foreground-scan, terminal-media, queue, user-decision, or handoff evidence.

R2c-P is complete only when controlled closed-process create, modify, rename, move, replacement, and
delete converge without source-root enumeration, and one failed root cannot block another root on
the same volume.

Implementation checkpoint on 2026-08-23, subsequently superseded by the 2026-08-25 remediation:
schema v24 now owns the exact persistent-journal marker, per-root capability, V2/V3 checkpoint,
source-range lifecycle, canonical batch payload, queue-lineage, pending-rename carry, and exact
cross-root ownership tables. It supersedes v23 through an explicit forward migration.
Catalog open validates canonical unsigned journal and volume identifiers, signed nonnegative USNs,
generation authority, bounds, indexes, foreign keys, and relational ownership. Migration preserves
the catalog and durable queue, terminalizes partial v20/v21 inventory without absence authority,
and seeds active roots as `Unknown` plus `BaselineRequired`; it does not execute the R2c-Q baseline.
Root-generation replacement retires the prior journal authority without deleting its durable source
ranges or lineage, and only the active generation may be read or advanced by the repository.
The production repository publishes all successful pages from one shared response in a single
`IMMEDIATE` volume-batch transaction, rejects capacity degradation without advancement, and replays
only an exact normalized batch digest idempotently. Broker protocol v5 now admits at most eight
registered roots on one volume, performs one raw journal read from the minimum start to a shared
exclusive end, isolates per-root authorization and
continuity failures, and returns only root-relative candidates and cross-root handoffs. The
session-backed adapter registers and queries live capabilities, validates the shared boundary, and
translates each response into independently owned source ranges and lineage. Controlled fixtures
cover final-state candidates for create, modify, delete, rename, replacement, and cross-root move
without enumeration, media reads, or source mutation. Focused domain, migration, repository,
coordinator, broker, and session tests pass locally. Complete Daily and Windows Release gates,
controlled closed-process process-level acceptance, and independent audit remain outstanding, so
R2c-P is not complete and the active-slice declaration below remains unchanged.

Five independent-audit remediation passes reached an implementation checkpoint on 2026-08-25
without expanding into R2c-Q. Schema v23 adds durable range lifecycle and pending OLD carry;
schema v24 adds canonical 64-hex source-range identity, complete payload evidence, deterministic
provable v23 rekey, and exact carried-lineage/carry coordinates;
protocol v5 rejects v2/v3/v4 and binds carried and same-page handoffs through codec, client,
service, native backend, and session. Every successful query participates in the common boundary,
per-root starts filter before evidence budgeting, semantic OLD/NEW events remain whole across raw,
evidence, and native-buffer boundaries, and every page still performs one physical journal read.
All publishable pages from a shared response enter one atomic volume transaction with queue evidence,
both handoff owners, carry mutation, strict checkpoint compare-and-swap, and root state. The durable
batch digest binds complete normalized intents, timing, pending carry, lineage endpoints, owners,
and handoff state. Raw coverage leaves roots `CatchingUp`; only the production final-state terminal
transaction publishes `Current`. Bounded cleanup retains unresolved work, active authority,
cross-root owners, pending carries, recent active-generation history, and the newest necessary
retired-generation proof.

The third remediation adds a post-admission rename proof barrier shared by native endpoint
resolution and service revalidation. It uses the earliest rename at or after the failed root's own
start, permits only the proven non-rename prefix, retains a known OLD before an uncertain NEW, and
keeps no-rename sibling progress root-local. Complete empty pages can terminalize and publish
`Current` in their volume transaction; partial empty pages remain `CatchingUp`. Migration backfill
derives terminal state from durable queue, carry, and lineage evidence. Carry consumption requires
exactly two owners bound to the durable source range, both root generations, volume, journal,
reference, path, OLD/NEW USNs, and previous carry ID.

The fourth remediation makes target-side post-admission failure preserve a fully authorized source
OLD as durable carry before allowing the source checkpoint to advance to the NEW boundary. Source-
side failure or incomplete source evidence instead holds the proof boundary at OLD. Multiple
handoffs use the earliest safe barrier, while no-rename sibling work retains its proven progress.
Catalog reopen now requires every pending or completed cross-root lineage to have exactly two
payload-bound owners, one previous and one current, and the production terminalizer runs the same
validation before publishing `Current`. Schema-v24 source-range triggers explicitly reject null,
non-text, and noncanonical IDs on insert and update; the known prerelease-v24 trigger pair upgrades
transactionally, while mixed or weakened definitions still fail closed.

The fifth remediation makes every retained canonical source-range payload and its durable children
bidirectionally equivalent. Pending carry content must retain either its exact pending row or one
exact consumed-lineage proof through `previous_carry_id`; lineage content must retain its parent and
exactly two matching owners, and every durable child must appear in the owning payload. Missing,
duplicate, extra, or payload-only children fail catalog reopen and block `Current`. Bounded cleanup
deletes only complete terminal lineage/range proof clusters in one transaction. Schema-v24 DDL
comparison is quote-aware: it normalizes formatting only outside quoted tokens while preserving
literal, quoted-identifier, BLOB, GLOB, `RAISE`, and escaped-quote content exactly.

The sixth remediation binds every retained payload intent to one exact normalized persistent queue
row and source-range ownership proof in both directions. Production coalescing and validation share
one normalization path; terminal queue evidence remains until the complete closed range cluster is
cleaned atomically. Cross-root peer evidence requires exact lineage and enrollment provenance, and
consumed-carry recovery additionally binds the previous root, generation, path, NEW USN, volume,
journal, and covering range. Durable lineage is `Completed` only when both exact endpoint range
lifecycles are completed; `Superseded` requires proven root-generation retirement and cannot grant
`Current`. Missing, extra, or altered queue/ownership/payload evidence, premature or single-ended
completion, and forged supersession all fail catalog open and the production terminalizer closed.

Runtime tests at this checkpoint include v24 canonical-DDL and source-ID negative mutation,
v23 deterministic rekey/rollback/backfill, dynamic query growth,
single-FSCTL counting, unequal starts and root-order reversal, a pending OLD separated from NEW by
960 unrelated records and a native buffer, long-path/evidence overflow, SQLite reopen and atomic
crash/capacity rollback, full-content replay conflicts, historical-generation cleanup, and real
SQLite plus production final reconciliation in both source-first and target-first order. The final
intent/queue matrix also covers deletion, extra ownership, row and payload tampering, coalesced
retry-to-terminal cleanup, strict peer provenance, and both lineage endpoint completion orders. The
serial Rust, lint, broker integration, installer, and non-accessing guardrail counts are recorded in
the owning acceptance document. No real library, elevated SCM operation, real FSCTL acceptance,
Daily, Windows Release, or independent re-audit is claimed. R2c-P remains not complete, and R2c-O's
external elevated acceptance gap remains open.

R2c-Q - priority runtime, one-time baseline, and product state:

- persist and schedule P0 live, P1 journal, and P2 recovery lanes with actual reserved P0 admission,
  worker, filesystem-revalidation, and publication capacity;
- start the watcher first, then replay a continuous checkpoint without creating inventory; process
  P0 immediately while P1 or P2 remains active;
- make P1 and P2 yield at every bounded page, remove the single global recovery bottleneck, and
  preserve per-root fairness within each lane;
- perform exactly one baseline for an existing migrated root without current checkpoint, bracket it
  with journal boundaries, and keep live publication available throughout;
- trigger P2 only for the ADR 0024 baseline and proven-gap allowlist; terminalize partial ADR 0023
  startup epochs without granting absence authority;
- expose independent live and continuity state through the existing compact Chinese presentation,
  including truthful `LiveOnly`, catch-up, recovery, blocked, and unavailable behavior;
- keep preview generation asynchronous so a revalidated new location becomes visible before its
  derived preview is ready.

R2c-Q is complete only when a P0 change remains at event-to-visible P95 no greater than one second
during target-scale P1 backlog and P2 recovery, and a continuous no-change startup enumerates zero
source-root entries.

Implementation checkpoint on 2026-08-29: schema v25 adds durable P0 live, P1 journal, and P2
recovery lane identity, exact lane and recovery-authority shape validation, deterministic v24 queue
classification, and a persisted baseline lifecycle. Schema v26 adds persistent P2 candidate
ownership and the first metadata-inventory frontier. Schema v27 adds the application-storage durable
source spool with exact run/root/generation/authority binding, complete-directory type, no-follow/
reparse and identity evidence, and provisional incomplete-directory rows. Its forward migration
preserves all v26 durable state and validates the exact current marker, DDL, indexes, triggers,
foreign keys, and cross-table lifecycle. Schema v28 adds the immutable canonical-root identity to
each spool and requires that same proof at candidate drain, absence, completion, and finalization.
Its v27 migration does not trust or fabricate proof: active derived v27 candidates, owners,
inventory, and spools are retired and recaptured under the retained allowlisted authority, while
terminal v27 history is marked `RecoveryRequired` rather than accepted as `Current`.
Schema v29 promotes a complete root proof to a durable active-generation publication namespace.
Foreground scan and P2 completion establish or verify it atomically with freshness, generation
retirement removes it, and P0/P1 cannot publish without it. The v28 migration admits only consistent
full spool or V3 checkpoint proof, never V2 or a current path identity, and rolls back conflicts or a
partial schema.

The sixth R2c-Q implementation checkpoint removes the public proofless metadata-inventory API and
its production adapter surface. Metadata inventory can now start only from an unretired durable P2
authority (or the already defined first-baseline authority) and the guarded v29 path. P0/P1 path
workers lease real work before touching the source, then load the active proof and construct one
`PublicationGuardedFileDiscovery`; an empty queue and a missing proof open no source handle, while
an A-to-B configured-root replacement is rejected from the pinned descendant metadata identity
before the enumeration-root handle opens. Authoritative publication accepts only that non-forgeable,
non-cloneable capability, never an ordinary `FileDiscovery`.

The seventh R2c-Q remediation closes the remaining capability escape hatch: the guarded wrapper no
longer implements `Deref` or exposes raw handles, discovery references, or detachable iterators.
Authoritative traversal uses a lifetime-bound opaque cursor; the durable inventory cursor privately
owns a duplicated full namespace guard until its raw directory cursor has dropped. The application-
only authoritative entry requires that guard separately from ordinary context and constructs the
publication request inside a private coordinator, so enumeration, final revalidation, and the real
repository commit or rollback share one capability lifetime.

P0 has reserved admission, lane-specific leasing, a dedicated bounded worker, final-state
revalidation, and catalog publication ahead of lower lanes. A priority-aware SQLite admission
coordinator gives a waiting P0 transaction the next writer turn after the current transaction and
never holds its permit across filesystem, broker, hashing, signature, or pipe I/O. The runtime
registry uses short epoch and ownership transitions, so blocked I/O does not hold the global mutex,
stop retains cancel/join ownership, and late results cannot install into a replacement epoch.
Each production runtime epoch performs full schema, migration, and cross-table validation once;
later poll and worker connections use constant-size catalog path, Windows file-identity, WAL,
application-ID, user-version, and schema-cookie checks. Identity or schema-cookie drift fails closed
and revalidates outside the runtime mutex and every priority permit. Writer admission covers only
`BEGIN IMMEDIATE` through commit or rollback.

A successful public stop now means every P0, P1, and P2 worker was cancelled and joined, the core
observer stopped, the journal session closed exactly once, and the registry epoch was removed. The
bounded wait does not hold the runtime mutex. The first stop stores one absolute two-second deadline
with the registry epoch; concurrent stop and late Starting/Polling owners reuse it across all lanes,
roots, native watchers, core observer work, and journal close. A runtime returned after expiry first
becomes complete `Draining` ownership rather than an epoch-only stopping marker. Timeout retains the
same receiver, join-handle, watcher, and journal-close task ownership, including observers retired
during root changes, and cannot be projected to Flutter as stopped. Retry/reaper joins the one close
task rather than invoking close again or refreshing the deadline. Flutter invalidates the active
poll generation and invokes native stop immediately instead of awaiting that poll. A never-ending
poll cannot extend the public call beyond its absolute deadline: the registry immediately changes
the Polling entry to `Draining` with the same complete runtime `Arc` and first deadline, even while
the operation mutex is held. The late result is discarded and cleanup resumes on that exact owner
under the unchanged, possibly exhausted deadline. It cannot publish into or clear a restarted epoch.
Concurrent callers share only the in-flight stop future; settlement permits failure retry or
immediate successful restart, while dispose shares stop and permanently rejects a subsequent start.

The eighth R2c-Q remediation makes that registry ownership survive panics rather than depending on
the desktop bridge's outer panic containment. Constructor panic recovery clears and notifies only a
matching runtime-less Starting/Stopping epoch. Poll or operation panic retains the same complete
runtime in `Draining`, reuses a public deadline or creates one fault-stop deadline once, and returns
the stable `library_synchronization_owner_panicked` error without exposing its payload. Drain locks
are acquired outside `catch_unwind`; after acquiring the runtime mutex, each caller rechecks the
exact epoch, `Arc`, and deadline. Cleanup keeps P0/P1/P2, observer, journal-close receiver/outcome,
and every join handle in their owner slots until an idempotent step completes. One-shot panic can
retry the same owner, persistent panic remains fail closed, and neither path refreshes the deadline,
re-requests stop, closes or joins twice, detaches work, publishes `Empty`, or admits an ABA epoch.

The ninth R2c-Q remediation makes Flutter startup explicit and recoverable. One generation owns one
in-flight start; native failure retries the real start call only through a finite 100 ms, 500 ms,
and 2 s backoff, and no poll timer exists before success. Stop or dispose cancels retries and still
calls native stop when startup may have entered native ownership. Native not-started converges;
other stop failures retain ownership for retry. Old-generation completion cannot publish or start a
timer in a replacement epoch. Native panic reconciliation now accepts an epoch mismatch only when a
monotonic retirement watermark proves that old epoch already drained. A deterministic test barrier
forces stop, `Empty`, and replacement Polling between the old panic and reconciliation; the old
caller retains its sanitized stable error without touching the replacement. Same-epoch impossible
state and runtime-less `Stopping` remain fail closed.

The tenth R2c-Q remediation moves request admission ahead of asynchronous bridge scheduling. Short
synchronous bridge calls allocate checked monotonic owner tickets and cancellation fences. A stop
advances a cancel-through watermark even for `Empty`, stores the first absolute two-second deadline
at the public Dart stop boundary, and immediately changes a covered `Starting`, `Ready`, or `Polling`
owner to `Stopping` or `Draining` while retaining its cancellation flag and complete runtime `Arc`.
The later asynchronous stop task only drains that admitted owner under the stored deadline; it does
not perform a second admission or create a new timeout. A start ticket at or below the watermark is
rejected before epoch allocation and construction, an old poll ticket cannot attach to a replacement
epoch, a same-ticket bounded retry can recover, and ticket overflow or poisoned ownership remains
fail closed.

Flutter now reserves one owner ticket per shared start operation, reuses it only for the explicit
`catalog_database_busy` and `catalog_database_locked` 100 ms, 500 ms, and 2 s retry allowlist, and
attempts every non-transient failure once. Trusted cached catalog state reaches `runApp` before the
process-lifetime synchronization owner schedules background start with both synchronous and
asynchronous failure handling attached. Shutdown shares owner disposal and issues the synchronous
stop fence immediately without awaiting a hung start; late completion remains generation-stale and
cannot publish into the stopped or replacement UI.

The eleventh R2c-Q remediation closes the remaining Dart-side gap between process-global admission
and local controller state. Every successfully reserved cancellation fence now unconditionally
reaches the matching asynchronous native stop, so an unstarted second controller, hot-restart
replacement, or close-before-start cannot leave another owner in `Stopping` or `Draining`. Native
not-started remains idempotent convergence. The process-lifetime owner shares only an in-flight or
successful close; a failed dispose clears its cached future for same-owner retry while its permanent
closing state continues to reject restart. Deterministic Rust tests pin invalid tickets and fences,
epoch exhaustion without construction, and old-fence isolation from a newer epoch.

The twelfth R2c-Q remediation makes the native lifecycle representation kind-distinct. Lifecycle
values remain `u64`/Dart `BigInt` only at the bridge, and Rust decodes the low-bit kind into
`RuntimeStartTicket` or `RuntimeStopFence` before admission validation. A shared, checked, strictly
increasing `LifecycleOrdinal` owns ordering and cancel-through comparisons. The tag rejects an
unchanged start raw value at the stop boundary, but it does not prove issuance: flipping the low bit
produces a stop-shaped value with the same ordinal. The maximum process-local ordinal remains
`u64::MAX >> 1`.

The thirteenth R2c-Q remediation adds exact bounded provenance. Runtime state retains at most 64
exact admitted stop fences, each with its covered owner and pending or consumed state. Reservation
guarantees capacity, records the pending fence, advances the watermark, and transitions its owner at
one registry-lock linearization point. Pending entries are never evicted. If capacity contains no
reclaimable consumed history, reservation returns a stable structured capacity error without
allocating an ordinal, cancelling work, or changing owner state. Consumed history is reclaimed only
under later reservation pressure and never while it still covers the active owner; a call that has
already proved membership is independent of later reclamation. Same-fence concurrent/repeated stop,
timeout and panic retry, takeover by a later real fence without deadline refresh, Empty-delayed
no-op, and old-fence/new-owner isolation remain deterministic. The ledger is process-local, bounded,
not persisted, and requires no migration.

Production establishes the watcher before it opens the retained journal session. P1 requests at
most 64 broker records per page, checks cancellation between roots and broker calls, persists typed
per-root gap failures without checkpoint advancement, and does not let one failed root discard a
healthy sibling. P2 readiness and leasing require matching persisted unretired authority; each run
persistently owns its candidates, drains them in bounded batches after final filesystem
revalidation, and cannot retire through retry exhaustion. Its default inventory enumeration page
remains 4,095 entries while the real source iterator consumes at most 128 entries before yielding
and rotating the P2 root slot. Completed directories are retained in the durable spool and are not
re-read on output-page or process reopen; an incomplete directory is provisional and, after one
process loss, can be recaptured for at most twice that directory's entry consumption. Repeated
same-point crashes are not claimed as a persistent Windows cursor, but provisional rows cannot
publish false freshness or partial absence. Ordinary startup, elapsed time, an empty journal, queue
pressure, slow work, `LiveOnly`, and consistency audit do not authorize P2.

On Windows 11 x64 the source iterator walks descendant components relative to one pinned canonical-
root handle and never follows a junction. Each name uses its live parent's real case-sensitivity
semantics and containment matches root identity instead of lowercased strings. After rebinding the
configured root with a no-delete guard, a second root-relative terminal-file `NtCreateFile` alone
requests content access with native no-recall and no delete sharing. Its complete 128-bit identity,
volume, attributes, non-reparse state, and live root containment must match the attribute-only
handle before reading; both native calls keep a null EA buffer and zero EA length, and raw
volume/device roots never reach an OS open. Every P0/P1/P2 publication boundary is required to bind
the generation to its persisted v29 proof and hold a configured-namespace chain from the local DOS
volume root through every ancestor and root. The Win32 volume handle opens `C:\` with
`FILE_TRAVERSE | FILE_READ_ATTRIBUTES | SYNCHRONIZE`; descendant guards alone open relative to
their pinned parent with pure `FILE_TRAVERSE`. Both use backup/open-reparse semantics and
read/write sharing without delete sharing. Directory guards never use no-recall, and each already
pinned parent supplies a separate relative `FILE_READ_ATTRIBUTES | SYNCHRONIZE` metadata proof of
the next component. The long-DOS configured path supplies normalized descendant names, not the
volume-handle path or authority. The
complete chain remains held through the real SQLite commit or rollback. New production-poll
regressions exercise P0 root, P0 subtree, and bounded authoritative P2 with an existing proof: a
pre-commit root or ancestor rename returns Win32 32, the real delta transaction rolls back to a
structured retry, and catalog revision, completion/current state, proof, and authority retirement
remain unchanged. Renamed, replaced, missing, short-path-only, or inaccessible namespaces therefore
preserve the last trustworthy catalog. The durable output query uses one indexed
`LIMIT page_size + 1` sentinel instead of scanning all unstaged rows for every page.

The 4,096-row unresolved ceiling has a steady P2 cap of 3,072, 512 rows reserved for P1, and 512
reserved for P0. P1 yields before a next page when P0 is ready or active; P2 yields to both P0 and
P1. A migrated v26 catalog may carry the former 3,584-row P2 occupancy as capacity debt. In that
state exactly one bounded P2 debt page may release capacity while all new P2 enumeration, refill,
control, and finalizer pages remain blocked. Path-scoped rows that predate owners drain in bounded
pages; repeated non-path rows compact to one blocked root control with merged evidence, while each
superseded source retains its bounded lineage and cleanup protection until the survivor terminates.
No authority or freshness is fabricated. P0 still owns its reserve; the next admission is P1,
after which P2 resumes per-root rotation without lost work.

Legacy readiness selects zero-owner non-path work only while an unblocked survivor exists or more
than one row still needs compaction. The last survivor, including an input of exactly one row, runs
once and becomes an exhausted durable `legacy_recovery_authority_missing` condition, projected as
blocked `RecoveryRequired` with an explicit update-library action. It cannot enumerate or publish
authority, absence, checkpoint, or `Current`; reopening does not start another worker, and explicit
refresh or later legitimate authority can supersede it without losing lineage.

The existing-root baseline and watcher-gap path record complete opening root/volume/journal identity
and `NextUsn` before bounded metadata-only pages, keep P0 publication active, capture the closing
`NextUsn`, and replay exactly `[opening, closing)` through P1. Candidate terminal state, closing
replay, complete absence, replacement checkpoint, `Current`, and authority retirement share the
final completion barrier. Partial, failed, cancelled, crashed, retry-exhausted, or identity-drifted
work preserves the last trustworthy catalog and remains `RecoveryRequired` or `CatchingUp`. Rust
bridge and Flutter state project live health independently from `BaselineRequired`, `CatchingUp`,
`Current`, `RecoveryRequired`, `LiveOnly`, and `Unavailable`; new locations publish with preview
pending rather than waiting for derived preview work.

When bounded P0 reconciliation proves an uncovered watcher gap, one transaction creates an
independent P2 control and allowlisted authority, transfers lineage, and supersedes the P0 lease.
Injected failure rolls back every record, replay creates no duplicate, lower-lane reserve rejection
leaves P0 retryable, and ordinary P1 backlog creates no recovery authority.

The final controlled production run records 25 disposable-root P0 samples while 2,048 real PNG P1
candidates, a cold 10,000-entry P2 recovery, and a competing low-lane writer all use production
paths. The first P0 sample occurs after 128 real P2 source reads and before its first output page.
P1 and P2 are both active and make real progress in all 25 samples; P1 completes all 2,048 candidates,
P2 reads 128 to 3,200 to 10,000 source entries and publishes the production 4,095-entry page, and
the low-lane writer advances from 1 to 391,335 transactions. Queue P95 is 31 ms, worker-start P95 is
30 ms, visible-query P95 is 0 ms, and the current third-round end-to-end timing is P50 136 ms, P95
179 ms, maximum 251 ms, with zero samples above one second. The current focused rerun passes in
98.16 seconds. The earlier P50 61 ms, P95 70 ms, maximum 78 ms, 70.19-second run remains a clearly
historical pre-remediation snapshot.

A separate production coordinator run enumerates 4,096 real controlled files through
the same poll/open/worker chain, crosses the default 4,095-entry page and steady 3,072-row P2
backpressure boundary, terminalizes every owner, and reaches completed control/run, `Current`, and
retired authority only after closing replay, absence, and finalization. The separate 100-startup
focused rerun passes in 5.07 seconds with 100 journal queries, 100 session closes, zero physical
journal reads, zero inventory runs, and zero source-root entry enumerations. The current ordinary-
user 4,096-file rerun passes in 73.61 seconds; the earlier 33.63-second result remains historical.

The isolated release-profile large-v26 session fixture populates 1,024 queue, lineage, owner, and
frontier rows, performs the one migration and complete validation in 55.447 ms, and then completes
100 production-session reopens in 1.1784592 seconds with a 13.2528 ms maximum individual reopen and
exactly one recorded complete validation.

Focused remediation evidence covers proof-first P0 root/subtree and bounded authoritative P2
production scheduling with a guard held through real catalog-delta rollback; first-open P2 root and
ancestor replacement before enumeration; guarded spool-init commit/rollback; a legitimate no-proof
baseline; exact-one blocked legacy readiness and explicit-refresh supersession; atomic candidate
publication and rollback, ownership,
supersession, retry exhaustion, more than 4,095 candidates, multi-root rotation, watcher-gap
bracketing and crash boundaries, typed P1 failure isolation, blocked-I/O stop, cancellation,
v27-to-v28 phase-table migration and rollback, exact 3,584 zero-owner legacy debt, non-path blocked
compaction and explicit-refresh recovery, retained 64-owner debt, real per-directory case semantics,
handle-anchored junction/offline/move-out rejection, configured-root absence/finalizer rebinding,
shared-deadline retiring-observer joins, late Starting/Polling deadline retention, non-`Deref`
publication capability and guard-owned cursor lifetime, indexed sentinel paging, v28 exact-shape
reopen validation, bounded durable-spool resume, constructor and poll panic containment, same-owner
drain retry at every cleanup boundary, persistent-panic fail-closed behavior, and stale-drainer ABA
rejection. The P1
revision-rebase suite passes 3/3, the local registration isolation fixture passes, the 25-sample
production fixture passes, and development- and release-profile check and warnings-denied Clippy
pass. Seventh-remediation focused reruns pass 34/34 local-file, 41/41 incremental, 11/11
authoritative, 35/35 metadata-inventory, 47/47 production, 274/274 catalog, 62/62 migration,
23/23 legacy, and 36/36 scan-library tests with two intentional scan ignores; the compile-fail
doctest passes 1/1. The eighth-remediation production panic matrix adds eight tests, and its complete
serial production module passes 55/55 in 203.65 seconds. The exact ordinary-user 3,584-row legacy
and 4,096-file regressions pass in 0.89 and 73.61 seconds. Ninth-remediation red evidence records
Flutter 20 passed and 2 failed plus the forced Rust interleaving failing 1/1 before the fixes. The
green lifecycle file passes 26/26, the panic/stop filter 5/5, and the complete ordinary-user serial
production module 57/57 in 147.66 seconds. Tenth-remediation red evidence records Flutter 26 passed
and 1 failed because a non-transient start was attempted four times, plus the delayed-start Rust
interleaving failing 1/1 because stop at `Empty` did not prevent construction. The fixed controller
and lifecycle-owner files pass 28/28 and 2/2, and the complete ordinary-user serial production
module passes 62/62 in 241.94 seconds. Its non-sandbox Daily records 846 passed, zero failed, and 11
ignored library tests, followed by 3/3 broker integration tests and all 307 Flutter unit and widget
tests.

Eleventh-remediation red evidence records the controller at 28 passed and 1 failed because an
unstarted controller admitted a fence but did not drain, and the lifecycle owner at 2 passed and 1
failed because it permanently shared a failed close. The fixed files pass 29/29 and 3/3. Four new
Rust lifecycle boundary tests were direct green against the existing native implementation; the
focused filter passes 4/4 and the complete ordinary-user serial production module passes 66/66 in
281.70 seconds. Development/release check, warnings-denied Clippy, and `quality_lint.ps1` pass. After
one measured Windows OS 1455 commit-limit failure, the unchanged Daily gate passes under one Cargo
build job and one Rust test thread with 850 passed, zero failed, and 11 ignored Rust library tests,
3/3 broker integrations, all 309 Flutter unit and widget tests, controlled Windows scan 2/2, native
Windows accessibility 2/2, generated bridge compatibility, and tracked-diff whitespace validation.
The current internal Windows x64 Release application builds in 241.0 seconds; the earlier 60.3-,
77.5-, and 125.2-second results remain historical. Explicit absent external bundle input fails closed
at signed-bundle admission. R2c-Q remains an implementation checkpoint and is not accepted. R2c-O
remains the active acceptance slice; its elevated installed-service, real FSCTL, signed-bundle, and
separately authorized retained-library gates remain open. At this eleventh-remediation checkpoint,
R2c-R had not started.

Twelfth-remediation evidence presented the earlier start raw value unchanged after a real stop. It
proved tag decoding but did not test the same ordinal with its low bit changed, so its historical
5/5 lifecycle-boundary and 67/67 production results are not issuance-provenance evidence.

Thirteenth-remediation red evidence instead presents `start_ticket.raw() | 1` after the later real
fence has advanced the watermark and published `Draining`. The tagged implementation accepted the
forgery, returned success, and closed the owner; the focused run failed 1 test with 861 filtered.
The exact admitted-fence ledger returns
`library_synchronization_lifecycle_fence_invalid`, performs zero closes, preserves the real
`Draining` owner, and lets the real fence drain exactly once. The production lifecycle-boundary
filter passes 6/6, the stop-fence filter passes 8/8, and the active-owner capacity-pressure retry
fixture passes 1/1. The complete ordinary-user serial production module passes 72/72 in 74.32
seconds. Explicit formatting, development- and release-profile check and warnings-denied Clippy,
and `quality_lint.ps1` pass. The resource-bounded Daily passes 856 Rust library tests with zero
failures and 11 ignored, broker integration 3/3, every Flutter test file, controlled Windows scan
2/2, native Windows accessibility 2/2, generated bridge compatibility, and tracked-diff whitespace
validation. The internal Windows x64 Release application builds in 89.9 seconds; explicit absent
external bundle inputs fail closed before SCM or process startup. R2c-Q remains an implementation
checkpoint and is not accepted; R2c-O remains the active acceptance slice with its external gates
open. The fourteenth independent read-only R2c-Q re-audit reports zero Critical, High, Medium, or
Low findings after fresh 6/6 lifecycle, 8/8 stop-fence, 1/1 active-owner capacity/retry, 29/29
Flutter controller, 3/3 process-lifetime owner, and whitespace checks. The R2c-Q implementation/
audit checkpoint is therefore closed, but R2c-Q, R2c-P, R2c-O, and R2c remain not accepted. R2c-R
has begun only as the non-external controlled local reliability checkpoint below.

R2c-R - change-driven reliability and closeout:

- repeat controlled live and closed-process operations, event storm, journal backlog, journal reset
  and trimming, watcher overflow, broker crash/restart, application crash, cancellation, root
  replacement, OneDrive placeholder, Chinese path, long path, and multi-root isolation scenarios;
- prove running-time P95 no greater than one second, closed-process single-change visibility P95 no
  greater than two seconds after normal runtime readiness, and zero root enumeration across 100
  no-change restarts;
- record bounded queue, journal read, IPC, memory, SQLite transaction, retry, shutdown, service,
  installer, and storage evidence without turning a million-record unrelated interval into unbounded
  retained state;
- run migration, source immutability, no-hydration, journal-no-mutation, complete Daily, Windows
  Release, installer, upgrade, uninstall, and package compatibility gates;
- use the two real roots only through a separately current-authorized, serial, read-only workflow
  with isolated derived storage;
- finish with independent architecture, code, security, migration, performance, source-safety, and
  full-range audits, then close every finding before R2c can be accepted.

R2c-R is complete only when the change-driven production path, not an isolated parser or PoC,
passes the complete user scenarios. R3 remains paused until this closeout is accepted.

Non-external controlled local checkpoint on 2026-08-31: the repository-owned R2c-R gate now runs
19 fully qualified, exactly counted Windows x64 tests through the production coordinator and its
existing adapter contracts. Controlled live, closed-process durable P1, watcher-overflow/P2,
million-record parser stream, 100-startup no-change, P0-with-P1/P2, reset/trim, reconnect,
shutdown/cancellation, replacement, same-volume multi-root, placeholder, and Chinese/long-path
evidence pass. The fresh acceptance record owns the exact measurements and red/green history. This
is not R2c-R acceptance: externally signed release, elevated SCM/service lifecycle, real broker
FSCTL, and separately authorized retained-root source-immutability/no-hydration evidence remain
open, as does the final accumulated audit.

The checkpoint's final local gates also pass: the resource-bounded complete Daily run reports 859
Rust library tests passed, zero failed, and 16 expected ignored; broker binary integration passes
3/3; Flutter unit/widget tests pass 309/309, controlled Windows scan passes 2/2, and native Windows
accessibility passes 2/2. The internal unsigned Windows x64 Release application builds in 70.1
seconds, both the application and Rust DLL are PE machine `0x8664`, all 81 Cargokit Release
dependency files are current, and the packaged DLL hash matches. Dependency and binary scans find
zero R2c-R fixture, `test_support`, harness, environment, or counter-name matches. Formal Release
verification with explicitly absent signed-bundle and broker paths fails closed before SCM, FSCTL,
or packaged-process execution. These local results do not close any external acceptance gap.

The subsequent independent R2c-R audit reported zero Critical, four High, three Medium, and two Low
findings in the controlled checkpoint. The remediation keeps the change-driven architecture and
closes the local evidence gaps: an OS-known physically verified nonce root replaces public/temp
trust; native Windows 11 workstation/SKU/build admission replaces the generic Windows/x64 check;
Cargo and descendants are owned by a parent-deadline Job Object; worker reports bind nonce,
runner/parent/child PID, and phase; ValidationOnly proves the current 19/15/4 exact matrix; 100
no-change starts traverse and harvest the real production observer/coordinator path; the live storm
is bounded against 4,096 unrelated entries with scan rows unchanged; same-volume evidence now uses
the retained session-reader/production coordinator; and the one-million case exercises the 256 KiB
production buffer, real reference histories, and conservative capacity accounting. These repairs
remain a non-external local checkpoint. They do not accept R2c-R or replace the still-open signed,
SCM, named-pipe/FSCTL, retained-root, and final accumulated audit gates.

The second independent R2c-R review reported zero Critical, two High, one Medium, and one Low
finding. The follow-up remediation makes common-module loading definition-only and moves `Add-Type`
behind a verified repository-owned bootstrap, replaces `LocalAppData\Temp` and its predictable owner
with an exclusive native create relative to a held physical LocalApplicationData KnownFolder anchor,
and holds every verified OS filter-redirection component and the root until cleanup begins. Cleanup
then releases the runtime root blocker, reopens the unpredictable child relative to the still-held
physical parent, and verifies its exact identity and path before deletion. This is a bounded
cleanup-only namespace race, not an atomic transition: a same-user racer can force fail-closed residue but
cannot redirect deletion to a replacement identity. Internal junction attacks prove rejection with
zero sentinel writes or residue. The no-change gate
now reports the real constant work: 200 actual production polls produce exactly 200 metadata-only
availability probes while enumeration, inventory, full scans, media opens, discovery handles, and
publication guards remain zero. An executable metadata-only adapter contract protects that O(1)
boundary, and the worker-report binding tamper test is now the nineteenth exact public case. These
closures remain non-external implementation evidence; R2c-R and accumulated R2c are still not
accepted while the signed-bundle, SCM/service, named-pipe/FSCTL, retained-root, Cloud Files, and final
accumulated audit boundaries remain open.

The third independent R2c-R review reported zero Critical, two High, one Medium, and two Low
findings. Its local remediation replaces the compiler bootstrap's managed precheck/create gap with
an in-memory `Reflection.Emit` identity surface followed by a held-parent `NtCreateFile`; no CodeDOM
write is admitted before the repository tool parent and bootstrap child are held and physically
verified. Active replacement, pre-created junction, and forced compiler-failure guardrails are now
executable. Runtime cleanup has no string-recursive catch fallback: any reopen, identity, path,
reparse, or non-empty failure retains the owned nonce root, and internal ordinary/junction
replacement races prove replacements and sentinel targets remain untouched; the failure records the
owned leaf and expected file-identity token even when an attacker moved the original. Root
availability is split into a metadata-only filesystem probe and an opaque-evidence classifier, with
an executable source guard that rejects representative enumeration insertion. The 100-start
contract remains 200 polls, 200 metadata probes, and zero enumeration/full-scan/media work. The
owned-leaf contract is
limited to the fixed prefix and two 32-character lowercase-hex fields, plus one exact moved-fixture
shape used only by the internal race test; it rejects NT path and ADS syntax. The lint-reachable
guardrail now executes the exact report-tamper Rust behavior test rather than checking only its
matrix entry. These close only the third review's local guardrail findings; R2c-R and accumulated
R2c remain not accepted while the
signed-bundle, SCM/service, named-pipe/FSCTL, retained-root, Cloud Files, and final accumulated
audit
boundaries remain open.

The fourth independent R2c-R review reported zero Critical, two High, and one Medium finding. Its
remediation moves both writable anchors to a read-only, two-stage binding before any root or
bootstrap create: every existing component of the logical requested path is opened relative to a
held parent with reparse-point semantics, every component of the handle-resolved physical path is
bound the same way, and the two terminal volume/file identities must agree. This preserves supported
OS filter/container redirection while rejecting an intermediate junction before it can redirect a
write. Parent-relative opens use each live directory's Windows case-sensitivity flag, and identity
is never inferred from case-folded path text. A production KnownFolder intermediate-junction attack
now proves pre-write rejection, zero sentinel change, and no created disposable root.

Guardrail teardown is also entirely identity-bound. Sentinel and process-output files are held with
delete-on-close handles; every fixture directory is created or captured with a volume/file identity;
and cleanup reopens it relative to a held parent, rejects reparse or identity drift, and deletes only
that exact empty handle. Ordinary and junction fixtures are swapped a second time after validation;
stale tokens leave the unknown object in place, and cleanup proceeds only after the replacement's
new identity has been explicitly captured. A PowerShell AST audit rejects path-addressed deletion
primitives in the guardrail and runner. The Rust availability source guard now uses `syn` 2.0.119's
real parser and visitor instead of a handwritten brace scanner. The exact dev-only dependency is
MIT OR Apache-2.0, already existed transitively in the lockfile, and adds no production/release
dependency edge. Its red/green fixtures cover ordinary strings, raw strings, comments, and a real
post-brace `read_dir` call without false positives.

The fresh fourth-remediation ordinary-user controlled runner passes all 19 exact cases and safely
removes its final root. Live-change P50/P95/maximum is 390/435/435 ms against 4,096 unrelated
entries; closed-process recovery is 359/1,058/1,058 ms; overflow P0 P95 is 346 ms with 23.570-second
overall convergence; and the one-million-record stream completes 245 pages in 12.798 seconds. One
hundred no-change starts still perform exactly 200 polls and 200 metadata probes with every
enumeration, inventory, full-scan, media, discovery, and publication count at zero. Under 2,048 P1
and 10,000 P2 entries, 25 P0 samples measure 79/946/1,027 ms P50/P95/maximum, so the contractual P95
remains below one second.

The fourth-remediation `quality_lint.ps1` and complete resource-bounded Daily gate also pass.
Daily reports 861 Rust library tests passed, zero failed, and 16 ignored; broker integration is 3/3;
all 309 Flutter unit/widget tests pass; Windows scan and native accessibility are 2/2 each; and bridge
plus whitespace checks complete. Release-profile check and warnings-denied Clippy pass. The unsigned
Windows x64 application builds in 181.82 seconds; the application and both Rust DLL copies are PE
`0x8664`; all 81 Cargokit dependencies are current; hashes match; and dependency plus ASCII/UTF-16
scans find zero R2c-R test seams. Explicit absent signed inputs fail formal Release admission closed
in 0.51 seconds before packaged-process, SCM, or FSCTL entry. This is fresh non-external
implementation evidence only; it does not close the unchanged signed-bundle, SCM/service, named-
pipe/real-FSCTL, retained-root, Cloud Files, or final accumulated-audit boundaries.

The fifth independent R2c-R review reported zero Critical, two Medium, and one Low finding. Its
remediation replaces the remaining callable-name source blacklist with an owner-specific,
default-deny Rust AST structural allowlist, so aliases, macros, helpers, indirect calls, unknown
methods, and unknown expression shapes fail through one predicate rather than an enumeration-name
list. The common module now owns one recursive PowerShell AST audit applied to itself, the runner,
the guardrail, and every actual child command; module qualification, aliases, dynamic invocation,
reflection, unknown cleanup calls, and encoded deletion all fail closed. Native anchor binding and
failed-root cleanup take immediate local ownership of every newly opened handle before any fallible
identity, reparse, filter-path, or injected-fault query. Executable tool-only faults prove both
handles close before return and allow same-process rename plus identity-bound deletion. These are
guardrail and ownership corrections; they do not alter the accepted watcher, journal, queue, or
source-safety architecture.

Fresh fifth-remediation evidence passes the exact structural Rust test, all three PowerShell parses,
the recursive deletion-audit fixtures, native post-open fault fixtures, the 19-case lightweight
guardrail, exact 19/15/4 ValidationOnly matrix, 100-start zero-enumeration contract, exact report
tamper, formatting, development/release all-target checks, warnings-denied Clippy, `quality_lint`,
the complete 19-case ordinary-user runner, and the complete serial Daily gate. Daily reports 861
Rust tests passed, zero failed, and 16 ignored; broker integration 3/3; Flutter 309/309; and Windows
scan plus native accessibility 2/2 each. The unsigned Windows x64 application and broker build in
38.5 and 75 seconds; all five inspected PE images are `0x8664`; current 81-file DLL and 82-file
broker graphs have no newer dependency; the packaged DLL hash matches; and boundary-qualified
ASCII/UTF-16 scans contain zero fifth-remediation seam. Missing signing inputs and absent signed
artifacts fail formal Release admission closed without starting a process. This remains fresh non-
external implementation evidence only. R2c-R is not accepted, R2c-O remains active, and the signed-
bundle, SCM/service, real named-pipe/FSCTL, retained-root, Cloud Files, and final accumulated audit
boundaries remain open.

The sixth independent R2c-R review reported zero Critical, zero High, two Medium, and one Low
finding. The Rust correction replaces the still-general structural categories with an exact,
owner-specific call-closure contract: 17 named function items and 16 named support items each carry
a readable purpose, normalized-token digest, exact local-callee closure, and receiver/call shape in
failure evidence. The complete availability path, its cfg-specific helpers, imports, evidence
types, and reachable static state therefore change only through an explicit reviewed contract
update. Full-crate checks additionally reject `Drop` or overloaded-operator behavior for the six
availability-path types without rejecting unrelated module implementations. Local callee or
receiver shadowing, an unchanged owner whose helper gains `read_dir`, a reachable `Drop`, an
overloaded operator, and referenced lazy/static side effects all fail; the current production AST
remains green. This is an exact item-and-closure proof rather than another filesystem-name
blacklist, and the production availability operation remains O(1).

The PowerShell correction now audits the complete common, runner, guardrail, recursively resolved
helper, and exact dot-source closure under one default-deny policy. Every unresolved command, call
operator, variable dot-source, unknown helper, external executable, dynamic script construction,
reflection invocation, path-addressed deletion/move, or unapproved module qualification fails in
every scope. A single digest-locked process wrapper is the only execution boundary. It verifies its
own implementation and native Job Object suffix, resolves exact Cargo test arguments, and audits
the final actual PowerShell `Command`, canonical `EncodedCommand`, or held `File` source before the
child starts; payloads that cannot be recovered statically are rejected. Forty-five adversarial
fixtures cover the original seven bypasses plus aliases, module qualification, nested payloads,
`Start-Process`, `cmd`, `robocopy /MIR`, `System.IO`, reflection, unknown helpers, `Add-Type`, and
move/create boundaries while every real legal payload remains green. Native initialization now
keeps the volume handle in a nullable local `try/finally` until transfer and encloses bootstrap
creation, environment access, and all later initialization in a nullable-owner `try/finally`.
Tool-only post-open/pre-transfer and post-bootstrap/pre-initialization faults prove both owners are
closed in the same process before controlled native types are initialized.

Fresh sixth-remediation evidence passes the adversarial Rust exact test, all three PowerShell
parses and source closures, both native ownership faults, all 45 source/runtime audit fixtures, the
19-case lightweight guardrail, exact 19/15/4 `ValidationOnly` matrix, 100-start no-change contract,
exact tamper, root-replacement, and Cloud Files cases, formatting, all-target/all-feature check,
warnings-denied Clippy, `quality_lint.ps1`, the complete ordinary-user 19-case runner, and the
complete serial Daily gate. The no-change case records exactly 200 production polls and 200
metadata probes with every enumeration, inventory, full-scan, media, discovery, and publication
counter at zero. Daily records 861 Rust library tests passed, zero failed, and 16 ignored; broker
integration 3/3; Flutter 309/309; and Windows scan plus native accessibility 2/2 each. Because the
sixth correction is limited to acceptance tooling, `#[cfg(test)]` structural evidence, and exact
dev dependencies, no new Release build is claimed: the fifth-remediation fresh unsigned Release
remains the applicable product artifact. A new ASCII/UTF-16 scan finds both sixth-remediation fault
tokens absent from the application, packaged and Cargokit Rust DLLs, and independent broker. This
is still non-external implementation evidence. R2c-R is not accepted, R2c-O remains active, and
external signing/publisher admission, SCM/service lifecycle, real named-pipe/FSCTL, retained roots,
Cloud Files no-hydration, and the final accumulated independent audit remain open.

The seventh independent R2c-R review reported zero Critical, zero High, two Medium, and one Low
finding. The Rust correction removes the unqualified `vec!` from `windows_extended_path` and makes
environment-rebindable macro expressions illegal throughout the protected 17-function call closure.
Its exact contract now also validates the allowed attributes and built-in derives of all 16 support
items and scans the local, domain, metadata-domain, adapters-parent, and crate-parent macro
environments. Real synthetic same-module `macro_rules! vec`, parent-module `macro_rules! vec`, and
renamed `use ... as vec` fixtures fail with an exact item/source key while the production O(1)
availability closure remains green.

The PowerShell correction replaces the followed-path `FileStream` authority at every audited file
source with a separately digest-locked native snapshot. The preferred anonymous
`Command`/`EncodedCommand` form was rejected only after a real child proved that it clears
`$PSScriptRoot` and `$PSCommandPath`; preserving the runner's three-source semantics would otherwise
require source rewriting and a second general loader. The retained `-File` fallback now binds the
tool root, every parent, and the terminal with parent-relative, no-follow, same-volume, identity-held
handles; it rejects terminal reparse/directory state, denies terminal write/delete sharing, and
revalidates every wrapper/payload source by volume/file ID immediately before native Job transfer.
Ordinary identity, terminal junction, terminal swap, parent swap, and exceptional handle-close
fixtures are all internal and executable. No unaudited `-File` boundary remains.

The closure audit now fails closed under fixed limits of 8 sources, depth 8, 256 KiB per source,
512 KiB total, 32,768 AST nodes, 128 functions, 512 scopes, and queue high-water 512. Checked-add
errors report the exact budget key, limit, and actual or overflow; checks occur before their
corresponding read, parse publication, or enqueue, and deduplication is not a budget substitute.
The final measured maximum is 3 sources, depth 1, 158,839 bytes per source, 239,967 total bytes,
17,943 nodes, 60 functions, 54 scopes, and queue high-water 47. Every limit-minus-one/limit/limit-
plus-one arithmetic boundary, concrete byte/source/depth/node/function/scope/queue path, multi-source
closure, and overflow control is retained by the lightweight guardrail. These repairs remain non-
external implementation evidence: R2c-R is not accepted, R2c-O remains active, and the unchanged
signed-bundle, SCM/service, real named-pipe/FSCTL, retained-root, Cloud Files, and final accumulated-
audit gaps remain open.

Fresh seventh-remediation verification passes the three PowerShell parses, measured real closures,
four Rust availability-contract tests, exact no-change, report-tamper, root-replacement, and both
Cloud Files tests, the lightweight 19-case guardrail, exact 19/15/4 `ValidationOnly` matrix, format,
development and Release all-target/all-feature checks, warnings-denied Clippy, `quality_lint.ps1`,
and the complete ordinary-user 19-case runner. The serial Daily gate reports 862 Rust library tests
passed, zero failed, and 16 expected ignored; broker integration 3/3; Flutter 309/309; Windows scan
and native accessibility 2/2 each; bridge compatibility; and whitespace validation. The production
Rust edit is an O(1)-equivalent `Vec` construction with no ABI, bridge, packaging, or runtime-policy
change, so no fresh Windows Release is claimed. The retained fifth-remediation images remain prior
packaging evidence, not a current-tree build or new signature; seven new macro, identity, fault, and
budget tokens are absent from all four images under ASCII and UTF-16LE scans.

The ninth R2c-R reliability review reported zero Critical, zero High, one Medium, and zero Low
findings after the eighth remediation's first complete runner had already failed closed in watcher
overflow on SQLite `FileLockingProtocolFailed` (result code 15). Five additional independent exact
roots reproduced the exposure as four passes and one code-15 failure in the 10 ms raw SQLite
observer. This history remains failure evidence; later passing samples do not erase it. The bundled
`rusqlite` 0.40.1 / `libsqlite3-sys` 0.38.1 path uses SQLite 3.53.2 and WAL. The previous public
catalog path performed full session validation and a fresh query connection for each request, while
the acceptance observer repeatedly used raw, unconfigured connections. Code 15 could therefore
surface during open, fast schema validation, or the query and was flattened to the generic catalog
database error under a concurrent writer.

The adapter now owns one narrow `SqliteCatalogReadExecutor` for idempotent read operations. It
recognizes only rusqlite `FileLockingProtocolFailed`, drops the complete failed connection, opens a
fresh connection, reruns pure read-only schema/identity validation, and then reruns the query.
Application-owned preparation remains the only place that can create or migrate a catalog; writes,
migrations, transactions, preview-touch publication, and root removal never enter the retry owner.
All six public read categories--snapshot/window, timeline, layout manifest, folders, around-asset,
and asset-by-ID--use one application routing helper. Busy and Locked retain the existing five-second
SQLite busy-handler semantics and all other errors return immediately.

The production protocol policy is five total attempts, a 100 ms monotonic retry-admission window,
and 1/2/4/8 ms backoff capped at 8 ms. The window is evaluated after a completed SQLite call and
before another retry; it is not an interruptible hard wall-clock cap on an active call. Exhaustion
returns `catalog_read_protocol_retry_exhausted` with operation, attempts, actual elapsed time, and
the final structured cause without exposing a catalog root. Uncontended reads do not sleep. The
watcher-overflow case now retains one production-equivalent observer for gap, inventory, recovery-
authority, and location reads and propagates a final structured failure instead of reopening raw SQL
every 10 ms or panicking.

Deterministic red/green tests cover code 15 at open, read-only validation, and query; exact fresh
connection counts; no retry for non-protocol, Busy, or Locked errors; attempt/deadline exhaustion;
diagnostic preservation; no transaction or write-admission hold across backoff; and no implicit
migration. The final group passes 13/13, and a real concurrent WAL writer plus the public production
timeline path completes 256 child-process reads. The first full Daily kept its four parallel-load
failures: individual fresh opens took 101-212 ms and expired attempt-count fixtures that had copied
the 100 ms production window. Only those deterministic fixtures now use a 30-second attempt budget;
the dedicated 1 ms deadline test and every public/acceptance path keep the production policy.

Changing only the first raw gap-count helper produced four exact passes and one unpreserved nonzero
fifth diagnostic, so it was not accepted as complete. After the remaining raw inventory, authority,
and location reads were moved behind the same owner, twenty consecutive fresh-nonce, held-root exact
watcher-overflow runs pass. Their P95 range is 307-327 ms, convergence range 11.463-11.872 seconds,
maximum individual P0 is 420 ms, and every run reports attempts equal operations, zero protocol
retries, and maximum attempt one. The per-run `P95/convergence/attempts` sequence is
`307/11701/801`, `308/11610/798`, `308/11854/805`, `308/11772/800`,
`311/11690/797`, `310/11762/800`, `307/11463/787`, `308/11692/800`,
`325/11784/799`, `311/11600/794`, `308/11579/786`, `308/11776/802`,
`309/11872/808`, `325/11850/806`, `309/11825/803`, `327/11800/802`,
`308/11649/801`, `326/11805/800`, `307/11695/801`, and `308/11846/798`.
The following complete ordinary-user 19-case runner passes with watcher P95 308 ms, 11.631-second
convergence, 795 operations/attempts, no retry, and maximum attempt one; no-change, tamper, root
replacement, Cloud Files, backlog, and priority cases remain green.

Fresh verification passes the topology, macro, 17-function/16-support, case-sensitive, 19-case
guardrail, 19/15/4 ValidationOnly, three PowerShell parse, formatting, development/Release check,
warnings-denied Clippy, and quality-lint gates. Daily exits zero with 878 Rust tests passed, zero
failed, and 17 ignored; broker integration 3/3; Flutter, Windows scan 2/2, native accessibility 2/2,
bridge, and whitespace green. A fresh unsigned x64 app builds in 37.6 seconds and the independent
broker in 20.65 seconds. All four binaries are PE `0x8664`; 82 Cargokit dependencies are current,
packaged/built DLL hashes match, and ASCII/UTF-16 test-seam scans are empty. Formal Release and
portable gates fail closed on absent external signed inputs without process, SCM, pipe, or FSCTL
work. This remains non-external implementation evidence: R2c-R is not accepted, R2c-O remains
active, and the signed-bundle/publisher, service, real journal, retained-root, Cloud Files, and final
accumulated-audit boundaries remain open.

The tenth independent R2c-R boundary review reported zero Critical, zero High, two Medium, and one
Low finding. Executable red controls showed that the crate-visible generic read callback admitted
arbitrary repeated mutation or side effects, a forged public error code could request retry,
exhaustion could echo an absolute path, identity-check-then-default-open could recreate or replace a
catalog, and the production deadline had no deterministic clock proof. The corrected crate-visible
surface contains only named reads; its generic loop is private and a `syn` signature contract rejects
callbacks, mutable catalogs, connections, and transactions. The retry owner classifies only the
original rusqlite `FileLockingProtocolFailed` from an active attempt and returns fixed path-free
exhaustion text plus optional typed operation/attempt/elapsed/cause details through Rust, FRB, and
Dart. Ordinary `ScanError` construction remains compatible with absent details.

On Windows, every attempt holds a no-follow/no-recall regular-file handle with read-data,
read-attributes, and synchronize access, read/write sharing without delete sharing, and volume/file
identity taken from that handle. It matches the validated session before SQLite opens
`READ_ONLY | URI | NO_MUTEX`; the connection drops before the held identity guard and both release
before sleep. Delete, replacement, ABA, mismatch, reparse, success, terminal failure, and backoff
ownership controls pass without new `unsafe`. Production remains five attempts, 100 ms, and
1/2/4/8 ms, while a test-only manual clock proves recovery at attempts two through five for open,
validation, and query, refusal after the deadline, one over-deadline call with one attempt, and zero
sleep without contention. The production clock remains `Instant` plus thread sleep and release
builds contain no fault or manual-clock seam.

The dedicated observer boundary now includes completed P1 evidence as well as gap, inventory,
authority, and location, eliminating the last high-frequency raw reopen. Current focused evidence
passes 32/32 retry tests, 62/62 migration tests, all application catalog tests and the 256-read WAL
child, canonical bridge generation and structured SSE serialization, three PowerShell parses,
guardrail 19, ValidationOnly 19/15/4, topology/macro/source closure, no-change, tamper, root
replacement, and both disposable Cloud fixtures. This is non-external implementation evidence only:
R2c-R remains not accepted, R2c-O remains active, and repeated production watcher, full runner,
Daily, packaging, signed/service/real-journal, retained-root, and real Cloud evidence remain open.

The eleventh R2c-R remediation completes the second boundary review without widening the retry
policy. Locked SQLite 3.53.2 source ties the observed 10.941-second read to the 100-step
`walTryBeginRead` protocol loop. The bundled Windows build now enables
`SQLITE_ENABLE_SETLK_TIMEOUT`; every read connection uses a 100 ms busy timeout and `query_only`,
while writers retain five seconds. Compile-option and separate read/write timeout tests are green,
and the read-retry group is 33/33.

The watcher observer combines gap, inventory, authority, and bounded location evidence into one
named typed snapshot. Its pre-aggregation control failed at 789 operations. Twenty independent
fresh-nonce runs now pass with 474-527 operations/attempts, zero retries, maximum attempt one,
sample-P95 P95 374 ms, and convergence P95 12.128 seconds. A second red control found 78 availability
probes in the complete closed-process case, reproduced at 75/75/77: the fast observer exposed a
10 ms test wait loop below the then-current production 250 ms synchronization cadence. Five
corrected exact runs and the complete runner each use 24 probes. Every actual poll remains one
fresh O(1) availability probe, and the 200-poll no-change contract remains 200 probes with no
enumeration, inventory, or media access.

The final ordinary-user runner passes 19/19; formatting, development/Release checks, both Clippy
modes, quality lint, and complete Daily pass. Daily reports 898 Rust tests passed, zero failed, and
17 ignored, broker integration 3/3, all Flutter tests, and both Windows integrations 2/2. Current
unsigned Windows x64 builds pass PE `0x8664`, 82/83/83 dependency, packaged/Cargokit DLL hash,
`NotSigned`, and 18-token ASCII/UTF-16 seam checks. Missing formal and portable signed inputs fail
closed before process, SCM, named-pipe, or FSCTL work. The two failed disposable roots remain
retained. This is non-external evidence only: R2c-R is not accepted, R2c-O remains active, and all
external signed/service/real-journal/retained-root/real-Cloud/final-audit boundaries remain open.

The cadence-binding independent review then reported zero Critical, zero High, zero Medium, and one
Low finding: the closed-process Rust fixture still owned a separate 250 ms constant even though
`RustLibrarySynchronization` owns the production default, `Timer.periodic` consumes it, and
`main.dart` does not override it. The executable red mutation changed only the embedded Dart owner
from 250 ms to 875 ms; the old acceptance still returned 250 ms and failed with `left: 250ms` and
`right: 875ms`. A future Dart-only cadence change could therefore have left the old gate falsely
green.

The corrected test-only support compiles both authoritative Dart sources with `include_str!`, then
uses a closed tokenizer and strict local syntax contract to require one owner class, one constructor
default, one `final Duration pollInterval` field, one `Timer.periodic` call whose first argument is
that field, and one production construction in `main.dart` without a named cadence override. Missing,
duplicate, ambiguous, unconsumed, or overridden bindings fail closed. The parsed duration now drives
all closed-worker waits and the independent Rust constant is absent. No parser, Dart source payload,
or runtime dependency enters non-test Rust.

Fresh green evidence covers six source-contract tests plus the independent-literal control,
warnings-denied test Clippy, a locked Release library build, zero ASCII/UTF-16 matches for five
cadence-parser/source seams across the Release DLL, static library, and rlib, the ordinary-user
19-case guardrail, the exact 19/15/4 `ValidationOnly` matrix, and one complete ordinary-user 19-case
runner. The runner's bound
closed-process case reports six samples at 567/581/581 ms P50/P95/maximum, 24 availability probes,
zero enumeration or inventory reads, and three bounded content opens. The earlier independent
review's three fresh exact samples remain audit evidence; no temporary harness or public runner
selector was added merely to repeat them. This remains non-external implementation evidence:
R2c-R is not accepted, R2c-O remains active, and signed publisher/bundle, elevated service, real
journal, authorized retained-root/Cloud Files, and final accumulated acceptance remain open.

The phase-19 independent follow-up audit found two Low proof gaps in that cadence gate. First, the
Dart token contract searched whole files: moving the only `RustLibrarySynchronization()` call to an
unused helper, or moving the only `Timer.periodic(pollInterval, ...)` call from `_start` to an unused
method, still passed. Second, the Rust no-regression check used raw text containment: a comment or
string containing `Duration::from_millis(250)` failed falsely, while replacing any wait cadence with
`Duration::from_millis(250_u64)` or another independent expression escaped the check. Executable red
fixtures reproduced all four behaviors before correction without opening a real library.

The phase-20 remediation makes both contracts structural and default-deny. The Dart tokenizer now
locates one top-level `Future<void> main() async` body and its direct success `try`, then binds one
zero-argument synchronization construction through the same lifecycle owner, shutdown registration,
`ProviderScope` override, and ordered `startInBackground` call. It separately locates one real async
`_start(int generation, BigInt ownerTicket)` method and requires the class's sole periodic timer to
exist inside that body and consume `pollInterval`. Missing, duplicate, ambiguous, disconnected, or
dead bindings fail. The Rust side parses the controlled worker with the existing test-only `syn`
dependency, requires one direct immutable cadence local initialized by the zero-argument production
cadence function, exactly one owner call, and exactly three waits whose interval AST is that same
identifier. Comments and strings are not expressions, an alias/arithmetic/literal interval fails,
and a second cadence owner fails. The raw containment check is removed.

Fresh focused evidence is 14/14 cadence tests plus the production worker AST contract, warnings-
denied all-target/all-feature Clippy, and one ordinary-user controlled runner with all 19 internal-
disposable cases. Its closed-process case reports six samples at 577/592/592 ms
P50/P95/maximum, 24 availability probes, zero source or inventory reads, and three bounded content
opens; the no-change case remains 100 starts, 200 polls, 200 probes, and zero enumeration, inventory,
or media access. A read-only scan of the retained Release DLL, static library, rlib, and packaged DLL
finds zero ASCII or UTF-16LE match for six new parser/AST seam tokens; no new Release build is claimed.
All parser, embedded-source, and `syn` code remains below `cfg(test)`. This closes the two Low local
proof findings only: R2c-R remains non-external and not accepted, R2c-O remains active, and signed
publisher/bundle, elevated service, real journal, separately authorized retained-root and real Cloud
Files, and final accumulated acceptance remain open.

The phase-21 independent re-review reported zero Critical, zero High, zero Medium, and two Low
findings in that phase-20 proof. The Dart delimiter-depth model still treated an unbraced
`if (false) try` as direct, admitted a `holder.synchronizationLifecycle` receiver suffix, and counted
the timer inside a constant-false branch or uncalled local function. The recursive Rust visitor still
counted a wait under `if false`, a closure, or a nested item as if it occupied the production worker
slot, while its unqualified helper matcher missed `self::production_synchronization_poll_interval()`
and a `use ... as cadence_alias` owner. Eight independently executable mutation controls reproduced
those admissions against the previous implementation before correction.

The phase-21 remediation deliberately proves the current fixed source shape rather than claiming
general program reachability. A minimal Dart statement-owner cursor requires the unique top-level
`main` to own one direct `try`; its success block must directly own the exact construction,
lifecycle, shutdown, `runApp(ProviderScope(...))`, and full-receiver start statements in order, with
no intervening control owner, local executable, `return`, or `throw`. The unique `_start` must directly
own its retry loop; that loop must directly own the succeeded-status branch; and that branch must
directly assign `_timer = Timer.periodic(pollInterval, ...)` before its direct `started` return. The
Rust `syn` proof admits one direct owner, two waits in the worker's exact anchored statement slots,
and one first statement wait in the direct `crash-ready` branch. A whole-worker visitor separately
requires exactly three wait-call paths, exactly one path whose final segment is the production
cadence helper, and no `UseTree` reference or alias for that helper. Qualified, aliased, nested-item,
closure, and constant-false mutations therefore fail without a numeric-spelling blacklist.

Fresh phase-21 evidence is 25/25 focused cadence tests and warnings-denied all-target/all-feature
Clippy. The sandbox runner failed before fixture creation at the existing ancestor-pin Win32 5
boundary and is not counted; the identical ordinary-user runner passed all 19 internal-disposable
cases. Its closed-process case reports six samples at 560/580/580 ms P50/P95/maximum, 24 availability
probes, zero source or inventory reads, and three bounded content opens. Its no-change case remains
100 starts, 200 polls, 200 probes, and zero enumeration, inventory, full-scan, or media access. A
read-only scan of four retained Release artifacts finds zero ASCII or UTF-16LE matches across eight
new statement/visitor seam tokens; no Release artifact was rebuilt or relabelled. No real library,
retained catalog, external broker, elevation, SCM, named-pipe, or real FSCTL path was used. This
remediates the two phase-21 local proof findings only: R2c-R remains non-external and not accepted,
R2c-O remains active, and every external and final accumulated acceptance boundary remains open.

The phase-23 shared-policy correction supersedes the phase-19 through phase-21 cadence proof
implementation without rewriting their historical findings. Repeatedly extending a handwritten Dart
tokenizer/statement cursor and a Rust `syn` control-flow visitor attempted to infer cross-language
reachability from two source files; each audit found another syntactically valid dead-code, alias, or
ownership form. That proof surface was larger and less authoritative than the 250 ms policy it was
trying to protect.

The canonical cadence is now the exact UTF-8/LF bytes in
`tool/library_synchronization_poll_interval_ms.txt`. The quality generator rejects non-canonical,
zero, signed, unit-bearing, multiline, BOM, whitespace, and overflowing input. It also rejects values
above 9,223,372,036,854,775 milliseconds before output changes because Dart `Duration` stores signed
64-bit microseconds. The generator deterministically writes the Dart constant; its `-Check` mode never
writes and runs before format inside `quality_lint.ps1`. Dart production exposes only zero-argument
`RustLibrarySynchronization.production()`; every injected dependency and alternate cadence is
confined to the analyzer-enforced `@visibleForTesting .testing(...)` constructor. A successful-start
Zone timer test observes the actual `Timer.periodic` duration rather than source spelling.

The cfg(test) `production_synchronization_cadence` module includes the same text once, strictly parses
one positive non-zero millisecond value within that Dart-safe maximum, and owns it through
`ProductionSynchronizationCadence`. R2c-R composes that value with its counted readiness,
crash-ready, and recovery wrapper; the worker methods accept no interval argument and the parent
requires `1/1/0` for `crash-ready` or `1/0/1` for `recover`. R2c-M's still-runnable historical harness
stores the same value and exposes its interval to every foreground wait. The Dart/main embeds,
tokenizer, statement cursor, Rust cadence visitor, and their 25-plus mutation controls are deleted.
The existing `syn` dependency remains because separate filesystem source-topology tests still use it.

Executable red evidence first changed policy to 875 while generated Dart remained at 250, then
mutated the actual timer to 875, used `.testing()` from `main.dart`, and misrecorded a recovery wait.
The generation check, timer behavior assertions, fatal analyzer rule, and Rust count assertion each
failed at the intended boundary. Restored focused tests, warnings-denied Clippy, `quality_lint.ps1`,
and the ordinary-user 19-case lightweight R2c-R guardrail pass. The complete ordinary-user runner
also passes all 19 internal-disposable cases; its closed-process P50/P95/maximum is 566/703/703 ms
with 24 availability probes, zero source or inventory reads, and three bounded content opens. The
no-change case reports 100 starts, 200 polls, 200 probes, and zero enumeration, inventory, full-scan,
media, discovery, or publication work.

The serial Daily gate passes 900 Rust tests with zero failed and 17 expected ignored, broker binary
integration 3/3, all Flutter tests, controlled Windows scan 2/2, native Windows accessibility 2/2,
generated bridge compatibility, and whitespace validation. A fresh local unsigned Windows x64
Release build completes in 35.8 seconds; the application, packaged Rust DLL, and broker are PE x64,
the packaged and Cargokit DLL hashes match, the policy text and generated Dart source are absent from
assets and dependency manifests, and five Rust Release artifacts contain zero matches across eight
deleted parser/source-seam tokens. The sandbox variants fail only at the known held-parent reopen
boundary (`NTSTATUS=0xC0000022`, Win32 5); the identical commands pass as an ordinary user without
weakening the gates. This evidence cannot provide signed publisher/bundle, elevated service, real
journal, authorized retained-root, real Cloud Files, or final accumulated audit evidence. R2c-R
remains non-external and not accepted; R2c-O remains active.

The phase-24 independent review reported zero Critical, zero High, two Medium, and zero Low findings.
The first Medium finding showed that the phase-23 upper bound was expressed in milliseconds as an
i64/u64 limit even though Dart multiplies `Duration(milliseconds:)` by 1,000 into signed 64-bit
microseconds. Executable red accepted and generated `9223372036854776`, and the Rust parser reported
that same first unsafe value as accepted. The corrected common maximum is
`floor(9223372036854775807 / 1000) = 9223372036854775`; PowerShell generation and `-Check` reject any
larger value before output changes, while Rust uses the identical boundary. Exact maximum passes;
maximum plus one, u64 maximum, and u64 overflow fail. The current policy remains `250\n`.

The second Medium finding showed that a temporary tracked policy of 875 left R2c-M's independent
`PRODUCTION_POLL_INTERVAL` at 250 ms. Red failed with `left: 250ms` and `right: 875ms`. The single
shared Rust test-support module now owns the policy include, parser, safe bound, and value object.
`ProductionSynchronizationTestHarness` initializes from that value; R2c-M has no second production
cadence literal or parser; and R2c-R's counted wrapper composes the harness value. Synthetic and
actual tracked 875 mutations prove both consumers follow 875 before the policy is restored to 250.

The phase-24 follow-up review reported zero Critical, zero High, zero Medium, and one Low finding.
The first repair still returned a naked `Duration` from the harness and the R2c-M `wait_for` helper
still accepted that primitive. Executable red replaced one real startup-wait argument with
`Duration::from_millis(250_u64)`; both the raw source containment contract and the 875 accessor test
remained green, proving neither reached the production wait call. The containment contract and naked
getter are now deleted. `ProductionSynchronizationCadence` keeps its `Duration` field private and
owns the wait loop. Every R2c-M production wait passes the opaque cadence obtained from its harness,
and the R2c-R counted wrapper invokes the same method before recording its phase count. A compile-time
function-signature contract requires the opaque type; replaying the `250_u64` mutation now fails with
Rust `E0308`, expected `ProductionSynchronizationCadence`, found `Duration`. A thread-local sleeper
capture owned only by the cfg(test) shared module drives the actual R2c-M helper through an 875 ms
cadence without wall-clock sleep and records exactly one 875 ms request; Release contains no seam.

Focused policy and Rust contracts, the non-accessing R2c-M guardrail, warnings-denied Clippy, and the
ordinary-user complete lint gate pass. The complete ordinary-user R2c-R runner passes 19/19 with
initial closed-process P50/P95/maximum 565/578/578 ms and follow-up opaque-API rerun
565/585/585 ms, 24 availability probes, zero source or inventory reads, and three bounded content
opens; no-change remains exactly 100 starts, 200 polls, and 200 probes.
Sandbox execution still stops only at the known held-parent `C0000022`/Win32 5 boundary. Phase 24
does not change production Dart or a Release payload, so Daily and Release were not rebuilt; read-only
scans find no policy/test-support source in assets or dependency files, and the fresh eight-token
opaque-cadence scan finds zero matches in five retained Release Rust artifacts. No authorized R2c-M
retained-root phase ran. R2c-R remains non-external and not accepted;
R2c-O remains active, and external plus final accumulated audit evidence remains open.

The eighth independent R2c-R review reported zero Critical, zero High, two Medium, and one Low
finding. The Rust availability proof now closes the source-loading environment as well as the
17-function/16-support call closure. A `syn` AST contract locks the exact module declarations in the
crate, adapters, local source, domain, and metadata-domain chain, including visibility, cfg
attributes, inline/external shape, and the one digest-locked test-only `thread_local!` item. File
attributes, `cfg_attr(path)`, direct `path`, arbitrary or procedural attributes, `include!` item
macros, extern-crate aliases, and unexpected nested/generated modules fail by protected source key.
The unconditional private crate-to-adapters-to-local and crate-to-domain-to-metadata declarations
therefore load the same implementation under test and non-test production cfg. Executable fixtures
cover every reported loader mechanism while the same-module macro, parent macro, renamed import,
17-function, and 16-support controls remain green. No production availability code changed; the
operation remains one O(1) metadata probe with no enumeration, scan, or media open.

The PowerShell source audit no longer folds paths or treats path text as identity. Its path index is
ordinal and non-authoritative. Every requested file is first opened through the digest-locked
no-follow snapshot, whose held parent chain and terminal provide the volume/file ID; only that
identity may deduplicate a source. A duplicate open is closed before return, while a unique snapshot
has exactly one state owner and every pre-transfer exception closes the untransferred owner. Source-
count admission remains before the native open, and per-source bytes remain checked before text is
parsed. The guardrail opens an ordinary duplicate twice, proves one retained owner, exercises the
exceptional-close path, and verifies the case-disabled spelling control. It also successfully enables
a real case-sensitive NTFS directory below an identity-held high-entropy fixture: `Safe.ps1` and
`safe.ps1` bind as distinct identities, and the forbidden lower-case source makes the closure fail
closed. A platform that cannot enable the directory flag reports an explicit controlled `skipped`
result rather than a pass.

The AST-node budget now performs a bounded traversal whose predicate retains no node collection.
The root is counted once, each visit updates a separate high-water value, and the limit-plus-one
node throws immediately with `budget`, `limit`, and `actual`. Independent known shapes fix an empty
script at two nodes and a one-literal script at five; limit-minus-one, limit, and limit-plus-one
controls do not call the production counter. The other seven source/depth/byte/function/scope/queue
budgets retain their checked-add behavior. Current common, runner, and guardrail closure measurements
peak at 3 sources, depth 1, 162,853 bytes for one source, 251,983 total bytes, 18,901 AST nodes, 62
functions, 56 scopes, and queue high-water 49.

Fresh eighth-remediation evidence passes all three PowerShell parses, the real case-sensitive and
duplicate-ownership controls, the lightweight 19-case guardrail, exact 19/15/4 `ValidationOnly`
matrix, the Rust topology/macro/exact availability controls, exact no-change, report-tamper, root-
replacement and both Cloud Files tests, format, development and Release all-target/all-feature
checks, warnings-denied Clippy, and `quality_lint.ps1`. The first sandboxed Cloud fixture correctly
failed on ancestor pinning with Win32 5 and passed only after the identical ordinary-user command was
rerun outside that workspace sandbox. The first complete ordinary-user runner failed closed in the
watcher-overflow case on one SQLite `FileLockingProtocolFailed` result and cleaned its owned root and
processes; two immediately following complete runners each passed the exact 19-case report, including
that same case. The complete serial Daily then exited zero with 863 Rust library tests passed, zero
failed, and 16 ignored; broker binary integration 3/3; Flutter unit/widget tests 309/309; and Windows
scan plus native accessibility 2/2 each. Final read-only inspection still finds only the five empty
compiler bootstraps and one three-entry failed-run root created on 2026-08-31; no eighth-remediation
run added residue, and no historical object was deleted. This remediation changes acceptance tooling
and test-only Rust proof, not product ABI, bridge, package, or runtime code, so it does not require or
claim a new Release build. R2c-R remains a non-external checkpoint, R2c-O remains active, and the
signed-bundle, SCM/service, real named-pipe/FSCTL, retained-root, real Cloud Files, and final
accumulated-audit boundaries remain open.

Phase-26 closes the final phase-25 High in the production live-gap path. Native `need_rescan`,
observer-ingress loss, and offline-to-available evidence now remain durable `LiveNotification` P0
root gaps instead of becoming unowned `StartupCatchUp` P1 markers. A covering journal range first
acquires exact P1 source-range ownership before P0 supersession; an uncovered gap instead receives
an independent P2 control, allowlisted `watcher_uncovered_gap` authority, and lineage transfer in
one transaction. P2 capacity failure rolls back that transaction and leaves the P0 retryable. The
production five-case matrix reaches `Synchronized`, drains the queues, retains the exact lane,
origin, scope, intent, lineage, and consumer, and starts zero automatic full scans. Schema v30
records ambiguous naked v29 fallback rows under explicit recovery ownership without inventing event
provenance, journal coverage, or authority. This remains non-external implementation evidence:
R2c-R is not accepted, R2c-O remains active, and signed, SCM/service, real FSCTL, retained-root,
real Cloud Files, and final accumulated-audit evidence remain open.

Phase-27 closes the phase-26 follow-up High and Medium findings around manual recovery,
provenance, and presentation. Root/publication identity is no longer accepted as event provenance;
both historical v29 naked fallback shapes migrate conservatively to
`explicit_recovery_required`. An explicit foreground update transaction binds only that claim to a
scan ID/root/generation; generic scan high-watermark and terminalization exclude all claim-owned
gaps. Publish consumes the claim with the scan revision, while abandon or interrupted reopen
restores the exact typed block with no retry deadline. Rust metrics and snapshots, the existing
bridge field, Dart mapping, and the accessible notification now expose a clear manual `更新图库`
path; ordinary persistence failures do not acquire it. Focused migration, validator, scan lifecycle,
production snapshot, Dart, and widget tests pass. Complete Daily, Release, and the 19-case runner are
deferred to the next accumulated closeout; this phase does not access retained roots, Cloud Files,
SCM, real FSCTL, or signing. R2c-R remains non-external and not accepted; R2c-O remains active.

The earlier complete second-remediation local gate set was green. The three PowerShell scripts parse; the
fresh hostile-temporary/internal-junction guardrail and exact 19/15/4 ValidationOnly matrix pass;
format, development/release check and warnings-denied Clippy, `quality_lint.ps1`, and the serial Daily
gate pass. Daily reports 860 Rust library tests passed, zero failed, and 16 ignored; broker binary
integration is 3/3, all 309 Flutter unit/widget tests pass, Windows scan and native accessibility are
2/2 each, and bridge/whitespace checks pass. The unsigned x64 Release build completes in 67.3 seconds;
both packaged PE files are `0x8664`, its 81-file Cargokit dependency graph is current, hashes match,
and dependency plus ASCII/UTF-16 scans find zero R2c-R test seams. Formal Release admission with
explicitly absent signed inputs fails closed before packaged-process startup. These facts do not
change the non-external, not-accepted status.

#### R2c.12 Acceptance evidence

R2c is not complete until all applicable evidence exists. Its preview evidence is limited to
retain-or-invalidate behavior caused by automatic source changes; cache capacity, reclamation,
manual cleanup, storage relocation, and restart reconciliation remain R2b-owned contracts.

- create, modify, same-volume rename/move, same-path replacement, and removal update the gallery
  incrementally;
- the same controlled changes produce deterministic retain, invalidate, or remove semantics for
  derived evidence without keying any future smart-album result to an absolute path;
- normal single-file changes do not trigger a complete root scan;
- process start, watcher restart, overflow, retry, oversized inventory, and automatic recovery
  failure do not trigger a complete root scan;
- prerelease automatic full-scan checkpoints are retired into the one-time ADR 0024 migration
  baseline when required and cannot re-enter the production scanner;
- duplicate, reordered, incomplete, and late events converge on correct final filesystem state;
- related changes publish atomically at one catalog revision;
- a database failure or cancellation preserves the last trustworthy catalog;
- queued work survives a controlled process interruption without duplicate publication;
- a watcher overflow or failure marks live observation degraded and recovers from the continuous
  journal range; P2 starts only when that range cannot prove coverage;
- an offline or disconnected root retains its last catalog and does not publish mass removals;
- controlled content edits, same-path replacement, identity-proven rename or move, temporary
  unavailability, and authoritative removal produce the documented retain, atomic dimensions
  replacement, preview invalidation, or removal eligibility without a transient layout change;
- OneDrive and other recall placeholders are not hydrated by observation, metadata inventory, or
  authoritative recovery;
- the production Flutter gallery refreshes through bounded contracts and preserves stable identity
  and scroll position where the owning query remains valid;
- source removal, application shutdown, pause, retry, and cancellation do not hang the desktop app;
- every schema migration, adapter contract test, application test, Flutter state/accessibility test,
  and Windows integration scenario passes;
- Rust format, Clippy with warnings denied, Rust tests, generated bridge checks, Flutter analysis,
  Flutter tests, Windows Debug/Release build, and `git diff --check` pass serially;
- controlled fixtures and authorized real-root samples prove source bytes and entries are unchanged;
- closed-app create, modify, delete, rename, and move changes are covered by brokered journal replay
  without source-root enumeration on the continuous path;
- a no-change normal startup creates no inventory and enumerates exactly zero source-root entries;
- a P0 live change reaches the visible catalog at P95 no greater than one second even while P1 or P2
  is active, and a closed-process single change reaches it at P95 no greater than two seconds after
  normal runtime readiness;
- journal reset, trimming, broker failure, root replacement, and migration baseline use explicit P2
  recovery without gating independent P0 publication or publishing partial absence;
- broker caller identity, root containment, pipe access, protocol compatibility, installer lifecycle,
  portable `LiveOnly` degradation, root-external nondisclosure, and no-journal-mutation gates pass;
- cached catalog content remains immediately usable while startup continuity runs;
- development diagnostics expose active phase, elapsed time, bounded counts, and issue code;
- remaining filesystem limitations and measured performance are recorded honestly.

#### R2c.13 Explicit exclusions and anti-drift constraints

- Do not implement R3 fingerprinting, R5 classification, or R6 similarity to avoid finishing
  freshness.
- Do not build a second asset-identity or metadata pipeline for watcher events.
- Do not attach future classification or smart-album membership to a path or make Flutter infer
  retain-or-invalidate policy from a filesystem event.
- Do not accept platform notifications as authoritative state or assume they are ordered and unique.
- Do not full-scan the approximately 259 GB library in response to every change.
- Do not place the watcher, queue, inventory, or SQLite policy in Flutter.
- Do not open a volume or request elevation from the desktop process. UAC is limited to explicit
  broker installer, repair, update, or removal operations.
- Do not let the broker read media, mutate source files, configure the journal, open the catalog,
  accept arbitrary volumes or roots, or return root-external records.
- Do not create metadata inventory on an ordinary startup with a continuous journal checkpoint.
- Do not treat a persisted priority value as sufficient; P0 must retain real reserved execution
  capacity and must preempt P1/P2 at bounded boundaries.
- Do not let one root's P1 or P2 failure block another root on the same volume.
- Do not let an automatic synchronization or recovery path create a full-scan request.
- Do not add a synchronization, task, timeline, or duplicate sidebar destination.
- Do not mutate, normalize, hydrate, move, or delete source files.
- Do not expose a production control before its complete application use case, failure state, and
  tests are connected.
- Do not mark a slice complete because events print to logs, a fixture works, compilation passes, or
  a screenshot looks correct.
- After compaction or handoff, recover recent original conversation, inspect the live implementation
  and ADRs, and compare actual verification before continuing from this section.

### R3 - Exact duplicate understanding

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

### R4 - Virtual organization and review foundation

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

### R5 - Metadata, primary classification, and human review

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

### R6 - Perceptual similarity review

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

### R7 - Semantic discovery and advanced search

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

### R8 - Physical organization dry-run

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

### R9 - Explicitly authorized filesystem execution

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

### R10 - Large-library maturity and release readiness

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

### Product value checkpoints

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

## 7. Large-library test ladder

Large testing starts during R1 rather than waiting for R10:

1. deterministic fixtures for corrupt, locked, unavailable, Chinese, long-path, and wrong-extension
   media;
2. synthetic thousands and tens-of-thousands of paths and catalog rows;
3. virtual-gallery stress data large enough to exercise timeline jumps and lazy disposal;
4. controlled read-only scan of `local-primary`;
5. controlled read-only scan of `cloud-primary` after availability checks;
6. controlled read-only combined scan;
7. warm incremental scan after known additions, removals, and modifications;
8. live create, modify, rename, replacement, removal, and event-storm reconciliation during R2c;
9. closed-application journal catch-up, no-change zero-enumeration startup, live-change preemption
   during P2 recovery, and forced journal-gap recovery during R2c;
10. exact-fingerprint reuse, regrouping, cancellation, and target-library duplicate coverage during
    R3;
11. resumable review, override durability, and confidence-band sampling during R4 and R5;
12. perceptual-candidate quality and bounded index evidence during R6;
13. immutable dry-run determinism and unchanged source and target trees during R8;
14. separately authorized operation fixtures and recovery evidence during R9.

Every large run records file counts, duration, throughput, structured issue counts, cancellation
behavior, recovery behavior, peak resource observations where available, cache growth, and whether
source bytes changed.

## 8. Engine admission rule

An engine does not become default because another application uses it or its README lists a feature.
Each candidate requires license review, adoption and maintenance evidence, Windows integration cost,
fixed-corpus quality, cold and warm performance, failure isolation, cancellation behavior, cache
impact, Chinese and long-path behavior, and a replacement contract test.

Rejected and experimental engines remain documented with evidence. Ame-owned native implementations
may serve as benchmarks or fallbacks but are not automatically preferred over mature libraries.

## 9. Anti-drift rules for this roadmap

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

## 10. Current active stage

Active stage: **R2c - Windows 11 x64 change-driven continuity replacement**

Active delivery and acceptance slice: **R2c-O constrained broker and installer foundation**. R2c-N
has admitted the exact fail-closed protocol, authorization-proof, lifecycle, capacity, cancellation,
and terminal-delivery seam after focused verification and independent security review. R2c-O owns
the real Windows service, named-pipe transport, caller token and root-handle verification, bounded
USN FSCTL backend, installer lifecycle, package compatibility, and portable `LiveOnly` degradation.
Its external elevated installed-service evidence remains outstanding, so the active slice does not
advance. The live implementation foundation has separately reached schema v30 and broker protocol
v5 plus R2c-P and R2c-Q implementation checkpoints. Those checkpoints are not R2c-P or R2c-Q
acceptance.
R3 is paused; no R3 implementation belongs in this work.

Current work: preserve the open authorization-bound R2c-O, R2c-P, and R2c-Q external evidence while
continuing the non-external controlled R2c-R reliability checkpoint. No local result promotes those
slices to accepted. Each implementation slice receives focused tests, applicable complete gates,
and an independent read-only audit before the next slice can claim its foundation. The final R2c-R
audit covers the complete accumulated range.

Phase 33 is a retained-catalog startup remediation inside that active checkpoint. It must restore
schema-v23/v24 startup without weakening persistent-journal validation, preserve catalog and
completed inventory authority, prevent runtime recreation of terminal inventory authority, bind
recovery-authoritative P2 work to one exact per-root owner without blocking P0/P1 or other roots,
and keep both supported repository PowerShell hosts fail closed. It must pass the applicable
complete gates and receive an accumulated independent audit. It does not authorize a source-root
scan or advance R2c-O/R2c-R acceptance.

R2c-I through R2c-M and their audits remain historical evidence for the superseded ADR 0023 model.
Do not rewrite their recorded results as if they validated ADR 0024, and do not merge the integration
branch into `main` or start R3 until the new replacement reaches its own closeout.

R2b implementation, deterministic preview-lifecycle correctness, retained-catalog interaction
Profile, real-library catalog parity, Daily, Windows Release, and bounded source-readable preview
performance gates are complete. R2b was accepted on 2026-08-13. The former USN-based R2c reached its
recorded implementation and audit state on 2026-08-19, then was reopened on 2026-08-21 after the
target token could not use its primary continuity source and automatic fallback caused excessive
recovery work. The R2b interaction and source-safety contracts and accepted R2c incremental
publication contracts remain regression boundaries rather than migration work to repeat.

The frozen R2b interaction comparison revision is
`6d3f0686a91b85402251fe07fcc1690f268effd5`. It remains historical A/B evidence rather than a moving
current-status pointer. R2c must preserve the frozen native interaction contract, but new R2c
behavior establishes task-specific evidence against the current accepted implementation.

R2b Profile evidence reproduced retained-detail growth and triggered one guarded change. The
accepted controller now uses high and low watermarks with hysteresis rather than aggressive
page-by-page eviction; it did not replace the native scroll, time-rail, jump, or resize paths.

On 2026-08-10 the user reported that the current gallery interaction feels acceptable and directed
the project to avoid speculative or migration-driven performance changes that could create a
negative optimization. Remaining ADR 0014 slices are therefore implementation options behind
measured thresholds and behavior-parity gates, not authorization to rewrite the current scroll hot
path merely to complete the migration sequence. Bounded-memory requirements remain binding, but
their production implementation must preserve or improve the reported interaction baseline.

Every conditional interaction change is evaluated one variable at a time against the frozen
baseline in the same build mode. It is rejected when P95 build or raster time leaves the 60 Hz frame
budget, regresses by more than 10 percent, adds UI-thread stalls above 50 ms, increases nearby-return
or reversal placeholder exposure, adds avoidable catalog requests, delays the time-rail position
line beyond one display frame, or exceeds ADR 0014's two-logical-pixel settled resize drift. Profile
evidence compares Profile with Profile; final hand-feel acceptance uses Windows Release. A rejected
change is rolled back instead of being retained behind compensating debounce or synchronization.

The visible Flutter shell is the production surface and must not be discarded or treated as a
fixture-only prototype. R2c is now limited to freshness state, durable incremental change capture,
bounded catch-up, revision-safe delta publication, and explicit fallback reconciliation. It must not
use synchronization work as a reason to rewrite the accepted gallery, preview cache, classification,
or later analysis workflows.

### 10.1 Verified implementation snapshot

This snapshot was synchronized on 2026-09-02 against the live working tree and current planning
decision. Historical gate claims retain their recorded dates. R2c-I through R2c-M have recorded ADR
0023 implementation evidence. Under the accepted ADR 0024 replacement, R2c-O has deterministic
broker and installer evidence but still lacks its external elevated installed-service evidence;
schema v24 and protocol v5 provide an R2c-P sixth-remediation implementation checkpoint that
remains not accepted. Schema v30 provides the current R2c-Q priority runtime, candidate ownership,
durable bounded source spool, one-time baseline, and independent product-state implementation
checkpoint described above, including the ninth-remediation start lifecycle, stable retired-epoch
panic reconciliation, and the eleventh-remediation fenced-stop and retryable-close corrections.
Current Rust, quality, non-sandbox Daily, controlled Windows integration, bridge, and internal x64
Release build evidence pass. The fourteenth independent R2c-Q re-audit reports zero findings, so
its implementation/audit checkpoint is closed; R2c-Q remains not accepted pending external signed
release evidence. R2c-O remains the active acceptance slice. R2c-R has a remediated non-external
controlled local reliability checkpoint but remains unaccepted with all external evidence and the
final accumulated audit open. Its phase-23 cadence correction replaces the former fixed-shape
cross-language parser/visitor with one tracked policy, deterministic Dart generation, a real timer
behavior test, and a typed Rust worker cadence with counted phase invocations. Phase 24 closes the
Dart microsecond upper bound and removes R2c-M's second 250 ms production cadence through one shared
Rust cfg(test) policy value. Its follow-up replaces the remaining naked `Duration` wait boundary with
an opaque private-field cadence whose owned wait method is required by both R2c-M and R2c-R. Focused
tests, the ordinary-user 19-case lightweight guardrail, complete lint, the prior complete Daily and
fresh local unsigned x64 Release, the current complete ordinary-user 19-case runner, and the retained
Release seam scan pass. Phase 27 adds the explicit-claim/manual-update/provenance correction plus
typed capacity deferral and a non-empty real-P2 visibility matrix. Capacity waits refund leased
attempts, survive restart, yield to precise P0 work, wake when matching P1/P2 capacity is released,
and do not relax genuine failure exhaustion. The native `need_rescan` matrix proves exact add/delete
publication from a controlled PNG baseline without a new automatic full scan or source mutation.
Retained P2 source frontiers are now owned by immutable change ID, so overlapping containment and
watcher-gap authorities for one root cannot steal or reopen each other's source. Catalog validation
precedes transfer, worker-spawn failure restores the exact owner, and current-authority pruning keeps
the in-memory set bounded. The current schema-v30 fields are sufficient, so no v31 DDL is introduced.
The accumulated ordinary-user closeout now passes the complete 83-test production module,
`quality_lint.ps1`, the 19/19 internal-disposable runner, serial Daily with 927 Rust tests and both
Windows integrations, exact bridge hash, Release-profile Clippy, and a fresh unsigned x64 Release
with PE, `NotSigned`, dependency freshness, DLL hash, asset, and test-seam checks. The first Daily
red selected a newer valid P2 row instead of its retained P0 lineage owner; the exact-origin/lane
test query correction passes both focused and concurrent full-suite verification. Final independent
audit and all external signed, SCM/service, real named-pipe/FSCTL, retained-root, and real Cloud Files
acceptance evidence remain open; R2c-R remains not accepted and R2c-O remains active.
The live working tree, current schema, accepted ADRs, and fresh verification remain authoritative;
this roadmap does not preserve drifting commit hashes or duplicate complete test transcripts.

- R0 and R1 are accepted. The Rust-owned SQLite catalog, Flutter/Rust bridge, external preview
  storage, resumable multi-root scanning, atomic publication, per-file issue isolation, file
  identity, and revision-safe bounded queries are connected end to end.
- The live working tree advances the catalog schema to v30 and retains storage-settings schema v2.
  Schema v17 introduced the
  durable normalized change queue, root-generation tombstones, lease/retry state, catalog-revision
  evidence, bounded terminal-row retention, and permanent highest-generation authority. Schema v18
  adds authoritative scan ownership, generation and queue-watermark capture, previous-snapshot
  preservation, consistency-audit evidence, normalized historical relative paths, and single-scan
  root ownership. Schema v19 adds exact per-volume catch-up checkpoints, bounded queue and full-scan
  lineage, normalized cross-root identity handoff, exact-case path lookup, preview-repair authority,
  and fail-closed relational validation without losing the v16 preview ownership reconciliation or
  earlier root, scan, asset, location, frontier, capture-evidence, identity, and query evidence.
  Schema v20 adds durable metadata-inventory runs, fixed-bound staging and cleanup, completion and
  absence authority, and the `metadata_inventory` change origin while preserving the v19 queue and
  lineage contract. Schema v21 adds deterministic terminal-media evidence keyed by complete source
  state and inspection-engine identity. Schema v22 introduces ADR 0024 broker capability, per-root
  checkpoint, source range, and lineage authority; v23 adds range lifecycle and durable pending OLD
  carry; v24 adds the canonical complete batch payload, deterministic provable rekey, exact cross-
  root owner validation, and typed 64-hex source-range ID guards. Schema v25 adds exact durable
  lane rows and guards, recovery authority constrained to the ADR 0024 allowlist and matching P2
  work, and the persisted opening/inventory/replay/absence/completed baseline lifecycle. Existing
  queue rows are classified deterministically without fabricating recovery authority. Schema v26
  adds exact candidate ownership, the first inventory frontier, and cross-table recovery execution
  validation. Schema v27 adds the application-storage durable source spool, complete-directory
  identity and ordering evidence, provisional incomplete-directory recovery, exact run/root/
  generation/authority binding, and current-shape validation. It does not place sidecars in source
  trees or let provisional entries publish candidates or absence. Schema v28 adds immutable pinned-
  root identity to the spool, indexed sentinel paging, proof revalidation through candidate,
  absence, and finalization, and fail-closed v27 recapture migration semantics. Schema v29 promotes
  only mutually consistent full spool or V3 checkpoint evidence into the active root-generation
  publication namespace, binds foreground scans before work, establishes foreground/P2 proof with
  freshness in one transaction, and retires proof with the root generation. V2, conflicting,
  partial, or absent proof remains fail closed rather than inheriting authority from a current path.
  Schema v30 adds immutable live-gap recovery claims and validates an exact consumer for each
  superseded P0 gap. Root/publication identity is not event provenance, so its v29 forward migration
  conservatively retains every ambiguous naked fallback under explicit recovery ownership. A typed
  foreground-scan consumer may temporarily own only that claim after an explicit user update;
  publication consumes it atomically, while abandon or interrupted reopen restores it exactly.
- The authorized read-only target-library acceptance published 30,629 locations for
  `local-primary` and 48,384 for `cloud-primary`, for 79,013 active locations in one retained
  catalog. Sampled source bytes and source entries remained unchanged, and cloud-only placeholders
  were not intentionally hydrated.
- The authorized R2b retained-catalog parity gate reloaded those 79,013 locations through 155
  revision-safe 512-item pages using the effective capture, creation, then modification date keyset.
  Both roots remained available, every location appeared once, and all previews remained pending;
  the gate did not start a source scan or materialize media.
- R2a is accepted and its obsolete prototype entry point has been removed. ADR 0009 owns the
  production unified-gallery UI contract.
- The accepted R2b shell provides the source tree, search, complete-query sorting, equal-height and
  square layouts, density choices, selection, bounded complete-query selection, context menus,
  viewer, settings, temporary scan feedback, and right-side time navigation.
- The query-wide revision-bound manifest, deterministic equal-height layout snapshot, exact-extent
  lazy sliver, orientation-corrected dimensions, identity-keyed preview store, latest-wins target
  loading, logical-anchor resize correction, and first-publication migration from temporary window
  geometry to the query-wide wall are implemented. Late initial manifest publication cannot replace
  a completed timeline target with the new scroll position's default top offset.
- When compatible preview work first recovers dimensions that were previously unknown, the
  controller publishes query-, revision-, ordinal-, and identity-bound geometry evidence. Flutter
  freezes the actual viewport-center card and current visible logical range after native scrolling
  becomes idle, publishes sparse eligible evidence after a quiet boundary, bounds continuous
  evidence with a maximum-latency deadline, and defers directional, guard, and reflow-exposed
  evidence until a later user-scroll epoch. Each published batch overlays the compact manifest and
  atomically replaces the layout with a pre-paint center-card correction. Existing dimensions and
  ordinary preview readiness still cannot trigger geometry churn.
- An explicit time-rail interaction closes the previous dimension-recovery epoch before moving the
  viewport. The target's first usable metrics establish a new center-card epoch, so dimensions at a
  newly selected time publish without requiring a follow-up wheel or drag.
- Initial usable scroll metrics publish the first visible range and center-out preview demand, so
  unknown dimensions begin recovering without requiring the user to move the gallery. A compatible
  current preview with missing catalog dimensions uses bounded source header and orientation
  inspection instead of decoding the full source raster again.
- Native wheel, touchpad, keyboard, accessibility, and ballistic movement remain owned by Flutter's
  `Scrollable`. Programmatic navigation is generation-guarded and may be coordinated only where
  measured traces show competing writes. The newest explicit time target retains ownership across
  loading and layout alignment; stale visible ranges and late geometry callbacks cannot replace it,
  while real native movement cancels it explicitly.
- Retained-catalog Profile evidence reproduced unbounded detail growth, so the controller now keeps
  one contiguous detail window behind soft 5,000-item and 3,500-item watermarks. A 240-iteration
  reversal run reached 5,000 retained details and settled at 4,000; P95 build and raster times were
  2.611 ms and 1.047 ms with no UI-thread stall above 50 ms. Source-media materialization remained
  disabled during this evidence run.
- The manifest implementation contains flat and hierarchical storage paths behind a 64 MiB estimate.
  Existing evidence recorded about 5.9 MB for 79,013 items and 18.8 MB for 250,000 items, so the
  target workload remains flat. The million-item path is selected as hierarchical in current code,
  but its complete build, layout, resize, cancellation, and interaction performance evidence remains
  conditional scale validation rather than an R2b target-workload blocker.
- Preview storage has resolved-path ownership guards for import and cleanup, many-to-many active
  location ownership for shared artifacts, staged generation before post-decode source-state
  revalidation, atomic admission reservation and installation, display-driven 128/256/512
  physical-pixel buckets, on-demand legacy artifact adoption, protected high/low-watermark
  reclamation, structured failure, bounded startup recovery, verified foreground cleanup, and
  restart-safe preview-root activation with explicit retired-root cleanup. Cache-hit validation
  failure cannot delete an existing artifact, and reclamation protects or resets every compatible
  referencing location. Root unregistration and successful replacement publication detach retired
  location references and stale only zero-reference artifacts, while an abandoned staged scan
  preserves the authoritative active reference. These paths preserve durable geometry and exclude
  source media and unrelated files.
- The authorized R2b preview-performance gate used an isolated online catalog backup and 512
  bounded, locally readable `local-primary` locations. Across 24 cold and warm samples per display
  bucket, cold P95 was 193/201/211 ms at 128/256/512 px and warm P95 was 14/16/16 ms; all 72 warm
  requests reused their artifact. Natural pressure from 447 1024 px previews reached 57,068,214
  bytes under a 64 MiB budget. Reclamation took 234 ms, settled below the 80 percent low watermark,
  and an evicted preview regenerated in 19 ms with zero immediate boundary churn. Peak working set
  was 126,844,928 bytes, all 512 selected entries and 16 source-byte samples remained unchanged,
  and no preview request failed. The evidence accepts the current policy without triggering another
  optimization.
- R2c-A is complete. Rust now exposes a selective platform-independent synchronization facade with
  normalized observations and intents, root-generation isolation, explicit watcher-health and
  catalog-freshness states, deterministic path/subtree/root coalescing, bounded evidence-gap
  degradation, and ADR 0007-compatible final-state reconciliation outcomes. Forty-four focused tests,
  including 22 adversarial blue-team fixtures, cover create, modify, transient create/remove,
  paired and incomplete rename, path and nested-subtree supersession, stale generations, offline
  roots, malformed/Chinese/long paths, event storms, exact overflow counts, same-state replacement,
  identity-query degradation, path-bound authoritative removal, and failure preservation without a
  platform watcher or Flutter policy. The attack matrix is recorded in
  `docs/acceptance/r2c-a-blue-team.md`.
- R2c-B is complete and its 2026-08-14 audit hardening replaces the unpatched dependency with the
  ADR 0017 pinned `notify` 8.2.0 source plus narrow upstream-derived Windows backports. One bounded
  recursive observer lifecycle per root converts controlled
  create, modify, remove, paired rename, directory, rescan, callback-error, and overflow signals
  into R2c-A observations and intents. Thirteen application lifecycle tests and fourteen adapter tests
  cover non-blocking explicit-clock restart, crash-loop backoff, application-level dropped-evidence
  degradation, generation isolation, stop-failure isolation, metadata races,
  shutdown boundaries, queued native completions, server-exit acknowledgement, native notification
  overflow, watched-root loss, degraded recovery, Chinese and long paths, and a real temporary recursive root without touching
  a real library. The 2026-08-14 Daily gate passed all 207 Rust tests with 202 passing and five
  intentional ignores, all Flutter tests, Windows scan and accessibility integrations, bridge
  compatibility, and whitespace; the Windows Release gate built the packaged application and passed
  both bridge smoke tests.
- R2c-C is complete and audit-hardened. The application persists R2c-B plans through an Ame-owned
  queue port into schema v17. Stable change IDs, 250 ms default configurable stabilization,
  path/subtree/root supersession, paired rename paths, root generations, monotonic lease
  generations, crash recovery, bounded retry and retention, enqueue/success revisions, optional
  catch-up fields, structured health, and oldest-ready delay are durable. Source-local observation
  sequence, origin, or future-skewed timestamp cannot outrank later durable ingress, and compact
  highest-generation root tombstones survive terminal cleanup. Root registration seeds or advances
  that authority before queue ingress, including lifecycles with no queued work. A prerelease v17
  catalog without the complete-authority marker fails closed rather than guessing generation 1.
  Thirty-six focused tests prove process and watcher restart recovery, including equal-time and
  backward-clock source resets, minimum burst work, full old/new path and directory-subtree rename
  overlap, normalized capacity after a policy decrease, removed-root rejection across cleanup and re-registration,
  policy-adjusted retry exhaustion/reopening, migration, metrics, and cleanup. The 2026-08-17 Daily
  passed 239 Rust tests with five existing intentional ignores, all Flutter tests,
  both Windows integrations, bridge compatibility, formatting, and whitespace.
- R2c-D is complete. The application now leases path work only after a trustworthy published root
  is available and no full scan owns the publication boundary, checks final filesystem and media
  state through the existing adapters, applies ADR 0007 identity rules, and revalidates each
  present or absent path immediately before publication. Outcome and derived-evidence disposition
  travel together through an Ame-owned delta contract. SQLite rechecks root generation, catalog
  revision, active completed scan, and every lease generation under an immediate writer
  transaction, then atomically updates only affected locations, compatible preview ownership,
  orphan assets, active count, one catalog revision, and queue completion. Retained preview state
  is compare-and-swapped against cleanup/reclamation, filesystem access rejects intermediate
  links, paired rename reconciles both final paths, identity evidence is backfilled, and normal
  full-scan coordination restores rather than consumes retry attempts. Cloud-only placeholders
  preserve the last trustworthy location and remain durable retry work without content access or
  terminal completion. Thirty-eight focused tests
  prove these boundaries together with unchanged, add, edit, same-path replacement, authoritative
  removal, related-batch revision atomicity, malformed-file isolation, stale
  lease/revision/generation rejection, evidence validation, injected rollback, metadata-engine
  compatibility, and controlled source-byte preservation. The 2026-08-18 Daily passed 270 Rust
  tests with five
  existing intentional ignores, all Flutter tests, both Windows integrations, bridge
  compatibility, formatting, and whitespace. Authoritative subtree/root recovery, catch-up, and
  real-library event acceptance remain later R2c slices.
- R2c-E is complete and audit-hardened. The production desktop lifecycle starts and stops one Rust
  synchronization runtime, retains drained observer evidence until durable enqueue succeeds, and
  marks cold-start or recovered roots as needing authoritative reconciliation before claiming
  freshness. Flutter consumes a revision already published before screen construction, projects
  bounded Chinese root states, refreshes only published revisions, and
  preserves source scope, filters, selection, preview demand, stable asset identity, preferred
  location, await-safe viewer continuity, and the nearest surviving ordinal. A bridge failure before root
  metrics shows `更新受阻`. Background refresh distinguishes applied, busy, superseded, and failed
  outcomes; permanent catalog failures stop automatic retries, retain the target revision, and show
  one localized retry action. A newer revision arriving during failed work is maximum-coalesced into
  one follow-up attempt, while active or terminal scan feedback retains its progress, controls, retry,
  and acknowledgement before the pending synchronization error is shown. Acknowledging failed or
  cancelled scan feedback, including failure before scan allocation, clears only that transient task
  surface and does not restart the scanner. Desktop destruction remains bounded after coordinated
  shutdown.
  Eight runtime tests, preferred-location and ordinal SQLite fixtures, and production screen tests
  cover enqueue rollback, continuity gaps, rename, removal, and timeout behavior. The 2026-08-18
  Daily passed 283 Rust tests with five existing intentional ignores, all Flutter tests, both
  Windows integrations, bridge compatibility, formatting, and whitespace; the Windows Release gate
  passed both packaged bridge smoke tests. Authoritative recovery remains R2c-F.
- R2c-F is a retained accepted foundation. The historical implementation leases one bounded authoritative
  subtree, root, or freshness-gap row, enumerates no more than 4,096 entries and 128 affected paths,
  and publishes the complete retain/add/change/move/remove set at one catalog revision. Its historical
  oversized-work path escalates to the resumable full scanner; ADR 0023 and R2c-K replace that
  automatic escalation with metadata-inventory paging. Schema v18 captures the
  root generation and unresolved queue high watermark when a scan begins, freezes only pre-watermark
  work for that scan without consuming retry attempts, preserves later evidence, and releases only
  its own rows on abandonment. It normalizes historical relative paths, persists a fail-closed
  previous-snapshot requirement for unreadable rescans, and records authoritative audit completion.
  Production runs both bounded reconciliation and full-scan escalation outside the polling mutex,
  only after a healthy observer establishes continuity, with cancellable shutdown and bounded
  per-root retry whose failure history survives bounded re-escalation. Placeholders and other
  uninspectable entries remain unresolved through full-scan escalation without hydration or false
  freshness, and v17 path normalization preserves legacy location identity for both healthy and
  unavailable files. Durable lifecycle ownership separates foreground scans from bounded multi-root
  production recovery; foreground path polling cannot reclaim a slow live authoritative lease,
  and the Windows runner permits only one same-user production process to own that in-memory lease
  boundary before Flutter, Rust, or catalog initialization. Bounded authoritative root selection
  rotates fairly, timed-out workers block restart, future or exhausted retry rows do not create
  empty workers, and one root cannot own overlapping active scans. After process owner loss, a new
  SQLite connection can normalize and recover the expired work. Incremental identity backfill
  retains a normalized v17 location's legacy identifier. The 2026-08-21 policy correction removes
  periodic consistency-audit scheduling; prerelease audit rows are compatibility input only and are
  retired without source enumeration. SQLite queue and scan mutations acquire the writer before
  reading mutable state, empty path polls remain read-only, and ordinary foreground or background
  writer contention remains silent and retryable for 30 seconds. Deterministic two-connection and
  recovery-grace fixtures cover this boundary.
  Controlled fixtures did not access a real library. The complete Daily and Windows Release gates
  passed on 2026-08-19 with 402 Rust tests total, all Flutter files, both Windows integrations,
  packaged duplicate-process rejection, owner-loss replacement startup, bridge compatibility,
  formatting, and whitespace. The final independent integration audit found no remaining code or
  contract findings.
- Historical R2c-G was complete and audit-hardened for its recorded head, but ADR 0023 supersedes it
  as a production design. That implementation starts live observation before one bounded per-volume
  Windows USN catch-up, validates journal and catalog continuity, atomically enrolls all
  affected roots before advancing the exclusive checkpoint, and degrades every unsupported,
  permission-denied, discontinuous, malformed, unbounded, or unverifiable range to durable
  authoritative work. Schema v19 preserves exact-case candidates, deleted-parent reconstruction,
  bounded multi-watermark queue and full-scan lineage, stable cross-root asset identity, and
  compatible preview ownership without dependency cycles or Cartesian snapshot growth. Exact-shape
  migration validation rejects orphan or malformed authority; one-time prerelease preview repair is
  serialized across concurrent catalog opens. Controlled fixtures and the 10,000-file synthetic
  gate accessed no real library, requested no elevation, and mutated no source media. The complete
  Daily and Windows Release gates passed on 2026-08-19, and final independent audit returned no
  Critical, High, Medium, or Low findings. Authorized target-library evidence is recorded by the
  completed historical R2c-H acceptance. Schema v19 remains forward-migration input; production
  journal scheduling must be removed by R2c-I.
- Historical R2c-H remains valid baseline and source-safety evidence. The controlled production watcher records bounded idle,
  event-to-visible, storm-coalescing, restart, shutdown, queue, and storage evidence. The explicitly
  authorized two-root workload uses a read-only retained catalog and isolated SQLite backup,
  performs catch-up discovery without leasing or publishing authoritative work, and verifies all
  85,556 source entries plus 32 deterministic local byte samples before and after. Physical path
  aliases fail before writes, Cargo descendants are contained by one kill-on-close Job Object, hash
  reads remain bounded after open, and final memory/deadline samples cannot bypass their ceilings.
  This target-root phase does not claim authoritative recovery convergence or lease timing; those
  target-scale measurements remain extended R10 evidence.
  The complete Daily, Windows Release, and 10,000-file synthetic gates passed on 2026-08-19, and
  final independent audit returned no Critical, High, Medium, or Low findings. It does not validate
  the ADR 0023 metadata-inventory replacement.
- R2c-I implementation, local verification, and final independent audit are complete. Production
  no longer schedules USN or automatic full scans; scan resume is fail-closed and cannot recreate a
  removed root or checkpoint; schema v19 remains compatible; and focused, Daily, Windows Release,
  and zero-finding independent-audit evidence is recorded in
  `docs/acceptance/r2c-i-non-usn-cutover.md`. Historical R2c-G/H evidence remains separate from the
  target-scale replacement evidence recorded for R2c-M below.
- R2c-J implementation and local verification are complete. Schema v20 inventory persistence,
  bounded metadata-only discovery, safe positive-candidate routing, and complete-scope absence
  authority pass focused fixtures, repository lint, the complete Daily gate with 439 Rust tests,
  Windows Release verification, and a final independent audit with no remaining findings.
  Production continuity epochs and pageable scheduling are completed by R2c-K.
- R2c-K implementation and local verification are complete. Production continuity epochs,
  pageable metadata recovery, constant-work exact path checks, Cloud Files availability, live-event
  supersession, backpressure, retry, cancellation, and fairness pass focused fixtures, repository
  lint, the complete Daily gate with 452 Rust tests, Windows Release verification, and a final
  independent audit with no remaining findings.
- R2c-L implementation and local verification are complete. Typed lifecycle phases, durable
  exhausted-work diagnostics, protected active recovery, generation-bounded blocked presentation,
  current elapsed detail, silent normal synchronization, and immediate close semantics pass
  focused fixtures, repository lint, the complete Daily gate with 454 Rust tests, Windows Release
  verification, and a final independent audit with no remaining findings.
- R2c-M replacement tooling, target evidence, and stage audit are complete. The final Release disposable
  production coordinator records 25 mixed create, modify, rename, move, same-path replacement, and
  delete samples at 557 ms P50 and 828 ms P95 under the 250 ms foreground cadence, bounded storm
  coalescing, 1,188 ms cross-process restart convergence, immediate shutdown, and unchanged
  full-scan rows. The authorized isolated retained-catalog phase inventories 35,086
  `local-primary` entries in 7,674 ms and 50,472 `cloud-primary` entries in 14,409 ms, preserves
  source directory-entry metadata and placeholder state, retains 79,102 authorized manifest items,
  adds no queue rows, and stays below the 2 GiB Job Object limit without media reads or source
  mutation.
  The target-tested head passes the complete Daily gate with 461 Rust tests total, 450 passed and
  11 ignored, all Flutter and Windows integration partitions, and Windows Release verification.
  The final independent read-only stage audit reports no Critical, High, Medium, or Low findings.
- The 2026-08-22 R2c integration remediation protects leased metadata-inventory control authority
  from ordinary watcher work, retains bounded live-work prefixes under queue backpressure, and
  validates each opened Windows source handle against the canonical selected root. Startup also
  retires prerelease automatic full-scan checkpoints without replacing the active catalog and hands
  their unresolved evidence to metadata inventory. The complete local Daily gate passes with 468
  Rust tests total, 457 passed and 11 ignored, all Flutter tests, Windows scan 2/2, and Windows
  accessibility 2/2. Windows Release and packaged bridge smoke pass 2/2. The final incremental
  read-only audit reports no remaining findings. Real UNC/SMB platform coverage and
  positive-candidate early publication remain separate validation and contract work.
- ADR 0024 and R2c-N through R2c-R now own the active replacement. The accepted target is Windows 11
  x64 with complete automatic continuity limited to local NTFS roots whose constrained broker is
  installed and whose journal remains continuous. The current working tree now contains schema v30
  priority lanes, candidate ownership, the bounded durable source spool, watcher-first brokered
  catch-up, bounded recovery, the one-time baseline, and independent live and continuity projection.
  Controlled fixtures provide implementation evidence for reserved P0 publication, atomic
  P0-to-P2 gap promotion, and zero-enumeration no-change startup. The complete local Daily gate and
  internal Windows x64 Release build pass; service security, installed-service lifecycle, real
  brokered FSCTL, retained-library, externally signed Windows Release, and independent-audit
  acceptance remain active work rather than inferred completion from implementation existence.
- The phase-31 migration-integrity slice closes three phase-30 audit findings without changing the
  schema version or acceptance state. A v29 running or paused foreground scan now retains its exact
  queue ownership, scan identifier, and publication provenance through the v30 migration instead of
  being relabeled as explicit recovery; only a truly ownerless historical fallback is converted to
  `explicit_recovery_required`. Current-v30 validation and interrupted-scan repair require the linked
  scan to be owned by `foreground`, and crash cleanup uses the same handoff-aware orphan predicate as
  normal scan cleanup. Focused evidence passes 65 migration tests, 88 SQLite catalog tests, 36 scan
  tests with two expected authorization-bound ignores, 56 Flutter tests, warnings-denied Clippy,
  repository lint, bridge hash `941711727`, and absent-signed-bundle fail-closed admission. Daily,
  Release, the 19-case runner, retained roots, Cloud Files, SCM, real named-pipe/FSCTL journal, and
  signing remain the phase-32 or external closeout boundary; R2c-R remains not accepted and R2c-O
  remains active.
- The phase-32 queue-integrity slice closes the remaining phase-30 leased-capacity race and reserved
  failure-code findings without changing schema v30 or acceptance state. An exact capacity-deferred
  P0 live root gap is protected while either waiting or leased only when its lane, origin, intent,
  scope, path shape, failure code, status, and absent recovery claim all match. A concurrent precise
  live path therefore receives an independent P0 row and can publish while the gap remains deferred;
  repeated evidence for that path still coalesces normally and the gap retains its lease and owner.
  The `live_gap_p2_capacity_` namespace is now rejected by generic retry entrypoints and can be
  minted only by the typed capacity-deferral transaction. Lease-attempt refunds, terminal-budget
  exemption, metrics, and capacity wake-up use the same exact typed shape, so a forged code with the
  wrong lane, scope, status, or claim receives ordinary expiry and terminal handling.
  Focused evidence passes 82 queue tests, 84 production synchronization tests, retained-owner and
  native `need_rescan` regressions, 42 persistent-journal tests, 65 migrations, 36 runnable scan
  tests with two expected ignores, 56 Flutter tests, warnings-denied development and Release
  Clippy, repository lint, bridge hash `941711727`, and the ordinary-user 19/19 runner. The canonical
  Daily rerun passes 940 runnable Rust tests with 17 expected ignores, broker integration 3/3, all
  Flutter tests, and both Windows integrations 2/2. A fresh unsigned x64 Release is `0x8664` and
  `NotSigned`, has current 83/82/82 dependency graphs and matching packaged/Cargokit DLL hashes,
  and contains no executable test seam in six Release artifacts or ten Flutter assets. Retained
  roots, real Cloud Files, SCM, named-pipe/FSCTL, signing, and the final external audit remain open;
  R2c-R remains not accepted and R2c-O remains active.
- The phase-33 retained-catalog startup slice repairs a pre-recovery-authority lifecycle state
  without changing schema v30 or weakening fail-closed journal validation. A schema-v23 inventory
  that was already `superseded` could retain `absence_authority = 1`, so v23-to-v24 validation
  rejected the catalog and rolled back every startup. The migration now terminalizes interrupted
  legacy runs and clears authority from every non-completed run before v23 or direct-v24 contract
  validation, while preserving completed authority, issue evidence, staged entries, roots, assets,
  locations, and queued work. Runtime terminalization and newer-epoch supersession revoke authority
  atomically. The first independent audit found that newer-epoch supersession also had to delete its
  retired durable spool and that old runtime pollution could already exist in schema v25-v30. The
  remediation deletes that spool in the supersession transaction and adds one exact-DDL-gated,
  shrink-only v25-v30 repair before each owning validator. It touches only terminal
  `failed/cancelled/superseded` spools and unowned terminal authority; healthy catalogs stay on the
  read-only path, and malformed DDL, unowned active work, or any other contract failure rolls the
  repair back. Exact red regressions cover v23, v24, newer epoch, terminalization, superseded-spool
  reopen, and polluted current schema; 73 migration tests and 37 metadata-inventory application
  tests pass, including repair idempotence, malformed-DDL non-mutation, active-owner preservation,
  and rollback controls. A read-only online backup of the retained
  1.18 GB catalog migrated and reopened through production `SqliteCatalog::open` in disposable
  storage with zero foreign-key violations and unchanged root, asset, location, queue, run, entry,
  and completed-authority counts; only the one illegal superseded authority changed from one to
  zero. The original catalog and source libraries were untouched. Recovery-authoritative P2 work is
  now exact-owner-affine per root: an unfinished baseline selects its recorded run; otherwise an
  active run selects its own unretired recovery authority. If that owner is unavailable or not due,
  unrelated same-root P2 waits while P0/P1 and other roots continue. Selection is re-resolved inside
  the `IMMEDIATE` lease transaction, conflicting distinct-run begin fails before writes, and the
  validator rejects a mismatched or terminal baseline owner. Hosted PowerShell 7 separately exposed
  that the case-insensitive `$IsWindows` parameter collided with its automatic read-only platform
  variable. R2c-R and broker probes now use `IsWindowsPlatform`, assert the exact false-platform
  result, and retain the digest-locked destructive-fixture audit. A first hosted dual-shell attempt
  correctly failed closed when Windows PowerShell 5.1 left compiler output in the held R2c-R
  `Add-Type` bootstrap. CI therefore runs a compiler-free exact platform-binding probe under Windows
  PowerShell 5.1 and keeps the complete R2c-R and broker guardrails in the PowerShell 7 Daily job;
  no cleanup or source-audit rule was broadened. The next hosted run exposed a separate three-second
  test-fixture deadline that included fresh child `Add-Type` initialization before the child could
  report its blocked descendant. That fixture-only parent budget is now 15 seconds against the same
  30-second block and five-second termination requirement; production deadlines and cleanup are
  unchanged. Repository lint passes, including the 19-case R2c-R guardrail, broker installer
  guardrail, warnings-denied Clippy, and Dart analysis. Eighteen focused owner-affinity regressions
  pass. The complete serial evidence has 954 passing Rust tests with
  17 expected ignores, broker integration 3/3, all Flutter tests, Windows scan 2/2, Windows native
  accessibility 2/2, bridge/whitespace checks, and a current Debug application. Two post-guard
  disposable-directory rename/restore fixtures each pass 200 repeated cycles without relaxing the
  immediate active-guard assertions. The accumulated independent audit and hosted PR gate remain
  required before this phase closes; R2c-R remains not accepted and R2c-O remains active.
- The current R2b closeout working tree passed the complete local Daily gate and Windows Release
  gate on 2026-08-12, including packaged Rust-library loading and the release bridge smoke test.
  This is current-stage evidence, not a release candidate or completion of R10.
- Local Flutter and Dart commands use the repository-pinned SDK resolved from
  `$env:USERPROFILE\develop\flutter`; they are not assumed available through `PATH`.

### 10.2 Acceptance checkpoints

R0 acceptance:

```text
Windows Debug and Release launch
-> production native picker cancellation and controlled fixture import
-> Rust bridge scan, per-file issue isolation, preview rendering, and atomic catalog publication
-> catalog and previews outside source trees
-> unchanged source bytes and entries
-> accepted
```

R1 acceptance:

```text
resumable multi-root catalog and explicit-rescan reconciliation
-> authorized local-primary and cloud-primary read-only scans
-> two active roots and 79,013 active locations
-> bounded revision-safe catalog traversal without duplicate or gap
-> unchanged sampled source bytes and no forced full-library preview generation
-> accepted
```

R2b completed foundation:

- accepted production shell and unified-gallery interaction contract;
- complete-result timeline, keyset sort and search, stable selection, viewer, source actions, and
  scan feedback;
- query-wide final geometry with unloaded, failed, and ready states sharing the same rectangles;
- identity-keyed preview publication and bounded center-out demand priority;
- latest-wins time navigation, native relative scrolling, and actual-card anchor preservation;
- EXIF Orientation 1 through 8 reflected in durable dimensions and preview pixels;
- scan finalization, failure publication, and Explorer reveal corrections present in current
  history;
- deterministic fixture coverage and a successful hosted Daily gate for the frozen comparison
  revision.

R2b acceptance: **accepted on 2026-08-13**

1. **Complete** - frozen interaction Profile and long-session evidence on the retained catalog;
2. **Complete** - ADR 0005 preview-lifecycle implementation and deterministic recovery, cleanup,
   storage, geometry, and source-safety tests;
3. **Complete** - current local Daily and Windows Release gates;
4. **Complete** - currently authorized retained-catalog real-library parity without source mutation
   or cloud-placeholder hydration;
5. **Complete** - the separately authorized, bounded, source-readable preview workload required by
   ADR 0005 and acceptance-policy step 4, with cache pressure, reclamation, regeneration, memory,
   source-entry, and source-byte evidence recorded above.

R2b conditional-adaptation decisions:

1. **Triggered and complete** - retained-detail growth exceeded the stable-range requirement. The
   guarded high/low-watermark detail cache passed reversal, resize, viewer, native-input, and frozen
   Profile frame gates without changing the scroll, time-rail, or resize implementations.
2. **Not required for the target workload** - the 79,013-item flat manifest remains inside budget;
   the existing hierarchical fallback remains conditional scale validation.
3. **Not required** - traces did not reproduce competing programmatic position writers. Native
   Flutter `Scrollable` movement remains immediate and outside an asynchronous intent queue.

An untriggered conditional adaptation is a resolved **not required** decision, not unfinished R2b
work. Do not implement one merely to complete an ADR migration sequence or count it as a blocker
without the corresponding measurement or trace evidence.

### 10.3 Status maintenance rule

Historical commands, individual test transcripts, superseded prototype steps, and rewritten commit
identifiers belong in Git history, ADR evidence, acceptance reports, or release records rather than
this active plan. When implementation status changes, update this concise snapshot and its date only
after checking the live working tree and applicable verification. Never infer completion from the
existence of code, a workflow file, or an older passing gate. The frozen R2b comparison revision
in section 10 remains stable historical A/B evidence; it is not an active-stage status pointer.
