# Cedarflake Ame Roadmap

Status: active delivery plan

Last confirmed with the user: 2026-08-16

Last implementation-status synchronization: 2026-08-19

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
Ame | [在图库中搜索] | 最小化 / 最大化 / 关闭
```

The Library row's folder-plus action is the current entry point into the Ame-owned `AddLibraryRoot`
use case. It opens the folder picker and owns validation, progress, cancellation, and error state.
The global bar contains application identity, gallery search, and app-drawn window controls only;
library import and settings do not appear there.

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

### 4.9 Temporary import feedback

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
through R2c-F establish the first complete running-time synchronization and recovery workflow.
R2c-G adds supported-volume downtime catch-up only after that workflow is trustworthy, and R2c-H
provides large-library reliability evidence. USN catch-up therefore enhances R2c without blocking
its first running-time value.

R2c unlocks R3 only after R2c-A through R2c-F prove the running-time workflow, recovery ladder, and
truthful freshness state, while the target-library portion of R2c-H proves bounded catch-up ingress,
queue and storage behavior, and source safety at the retained-catalog scale.
R2c-G is conditional: it does not block R3 when startup can regain trustworthy freshness through a
bounded authoritative reconciliation, but it remains the preferred supported-volume optimization
when downtime catch-up cost or missed-change evidence exceeds the recorded budget.

User outcome:

After a folder has been added to Ame, images created, edited, deleted, renamed, or moved inside that
folder appear in the same unified gallery without requiring an ordinary full-root rescan. If Ame was
closed, a supported Windows volume can catch up from durable filesystem change evidence. If change
evidence is missing, overflowed, unsupported, or no longer trustworthy, Ame retains the last
trustworthy catalog, reports the degraded state, and performs the smallest authoritative
reconciliation needed to become current again.

The user continues to see one library rather than a separate synchronization application. Ordinary
wording is limited to concepts such as `已同步`, `正在更新图库`, `目录不可用`, `需要核对`, and
`部分项目无法读取`. Terms such as watcher, USN, queue, delta, adapter, and watermark belong only in
diagnostic details.

#### R2c.1 Safety and authority rules

- Filesystem notifications and journal records are hints that identify what must be checked. They
  are never accepted as the final file state.
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
- Full-root scanning remains the authority for first import, explicit rebuild, and final recovery.
  It is no longer the default reaction to every normal directory change.

#### R2c.2 Ownership and boundaries

The Rust domain defines Ame-owned, platform-independent values for:

- library-root identity and configuration generation;
- normalized change intent, such as path reconciliation, rename candidate, subtree reconciliation,
  and root freshness unknown;
- change origin, such as live notification, startup catch-up, user refresh, or consistency audit;
- reconciliation outcomes: unchanged, added, modified, renamed or moved, replaced, removed,
  skipped, retryable failure, and terminal issue;
- watcher health and catalog-freshness states without exposing a Windows or third-party type.

The Rust application layer owns:

- starting and stopping change observation for configured, available roots;
- converting raw signals into Ame change intents;
- durable enqueueing, debounce, coalescing, retry, backoff, pause, cancellation, and recovery;
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
- `ChangeQueue`: durably records, leases, acknowledges, retries, and supersedes pending intents;
- `IncrementalReconciler`: checks a path or bounded subtree and returns Ame reconciliation results;
- `CatalogDeltaPublisher`: applies one batch at an atomic revision boundary;
- `ChangeCatchUpSource`: supplies candidates observed while live notification was unavailable.

Names are illustrative, not mandatory APIs. Before adding a port, inspect whether an existing scan,
catalog, or filesystem contract already owns that responsibility.

Adapters own all platform and dependency details:

- evaluate the mature Rust `notify` ecosystem for recursive live observation on Windows and record
  its selected version, license, maintenance, cancellation behavior, overflow semantics, packaging,
  and replacement strategy before admission;
- keep `notify` event kinds, paths, errors, threads, and global state behind the adapter;
- continue using ADR 0007's Ame-owned Windows `FILE_ID_INFO` evidence for reconciliation instead of
  inventing another asset-identity rule;
- persist the durable queue, retry state, catch-up watermarks, and delta publication through the
  Rust SQLite adapter;
- add an NTFS/ReFS USN Journal adapter only in the later R2c catch-up slice and only after a focused
  ADR covers API behavior, journal invalidation, permissions, path reconstruction, `unsafe` safety
  invariants, tests, and fallback;
- keep Flutter presentation-only. Flutter does not watch directories, enumerate roots, write SQL,
  or infer catalog policy from platform events.

#### R2c.3 Durable change intent

The logical persistent model must be able to express, without committing prematurely to one table
shape:

- a stable change ID and `root_id`;
- the root configuration generation so work for an unregistered or replaced root cannot publish;
- one affected relative path and an optional old path or rename-correlation identity;
- normalized intent kind and origin;
- first-observed and most-recent-observed time;
- coalesced event count;
- pending, leased/in-progress, retry-wait, completed, and superseded states;
- attempt count, next retry time, and structured last failure;
- the catalog revision at enqueue and successful publication;
- the catch-up source and watermark where applicable.

This state is durable task data, not disposable thumbnail cache. Its schema changes require forward
migrations from every committed schema version and migration tests. Completed rows and obsolete
watermarks require a bounded retention strategy, but cleanup must never erase an unresolved gap or
user-owned decision.

#### R2c.4 Event normalization and coalescing

Raw filesystem events may be duplicated, reordered, incomplete, or delivered after the path changes
again. The inbound callback must remain lightweight: it normalizes and enqueues a hint without
running image decoding, a long SQLite transaction, a directory walk, or Flutter work on the callback
thread.

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
- `更新图库` requests application-owned reconciliation. It does not make Flutter enumerate files.

#### R2c.7 Lifecycle and race handling

Startup order:

1. Load the last trustworthy catalog, root configuration, unresolved change queue, and catch-up
   watermark.
2. Check each root's availability using metadata only.
3. Establish live observation before running startup catch-up so new events do not open another
   avoidable gap.
4. Resume durable pending work and process already-known changes.
5. Read a valid catch-up source from the last trustworthy watermark when available.
6. If no continuous evidence is available, mark the root `需要核对` and run the smallest safe
   authoritative metadata reconciliation.

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
- finish or safely return the currently leased bounded batch;
- persist health and the last acknowledged catch-up watermark;
- use bounded graceful shutdown so a watcher or queue cannot hang the window close path;
- leave incomplete work recoverable on the next startup.

#### R2c.8 Failure and degradation matrix

- Single unreadable or malformed file: record a structured issue and continue the remaining batch.
- File changes again during processing: fail final revalidation, coalesce the newer event, and retry.
- Notification buffer overflow or known event loss: mark the root dirty/degraded, stop presenting it
  as synchronized, and run an authoritative reconciliation of the narrowest known scope.
- Watcher failure: restart with bounded exponential backoff and cover the missing interval through
  catch-up or consistency reconciliation.
- Root offline, disconnected, or inaccessible: retain its catalog and display availability status;
  do not reinterpret failure as deletion.
- Database transaction failure: roll back the entire delta, keep the intent retryable, and do not
  increment catalog revision.
- Huge directory rename or removal: process descendants through bounded windows; do not keep every
  row in memory or claim complete removals until the scope completes.
- Catch-up log unsupported, truncated, recreated, or outside its retained range: invalidate the
  watermark and fall back explicitly; never guess continuity.

Escalation order is:

```text
single path reconciliation
-> dirty subtree reconciliation
-> root metadata reconciliation
-> complete root scan as the final recovery authority
```

The application must expose which level is in progress and why without leaking implementation
jargon into normal UI copy.

#### R2c.9 Startup catch-up with the Windows change journal

USN Journal support is an enhancement slice, not a prerequisite for the first live-update delivery.
The first R2c vertical slice should already provide reliable running-time observation, a durable
queue, bounded reconciliation, delta publication, overflow recovery, and manual consistency update.

When implemented:

- persist the journal identity, last trustworthy USN, volume identity, and associated catalog
  revision per volume;
- validate journal continuity before reading;
- share one bounded journal reader for multiple roots on the same volume while filtering candidates
  by root;
- translate records only into paths or subtrees that must be reconciled;
- treat file reference numbers and USN values as change-tracking evidence, never as `Asset`,
  `ContentFingerprint`, or cross-machine identity;
- handle journal recreation, truncation, unsupported filesystems, unavailable volumes, insufficient
  permissions, and failed path reconstruction with explicit fallback;
- introduce any new Windows `unsafe` only through an accepted ADR containing exact safety invariants
  and focused tests.

#### R2c.10 Low-frequency consistency audit

Live notification and journal catch-up do not eliminate the need for a low-frequency, cancellable,
observable consistency audit:

- prefer directory and file metadata without reading media content;
- schedule according to root health and last trustworthy audit rather than high-frequency fixed
  polling;
- allow `更新图库` for one selected root;
- reconcile a dirty subtree or root before escalating to expensive media reanalysis;
- publish removals only for the scope fully audited;
- preserve cloud-placeholder rules and R2c's non-mutating source-observation boundary.

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
entry or manual re-import for ordinary supported path changes.

R2c-F - recovery and consistency:

- force overflow, watcher failure, offline roots, database rollback, and repeated source changes;
- implement the escalation ladder and low-frequency audit;
- prove that recovery does not publish mass false removals or claim health early.

Status: **complete and audit-hardened on 2026-08-18**. ADR 0021 owns the bounded authoritative
subtree/root worker, schema v18 full-scan generation and queue-watermark coordination,
previous-snapshot preservation, background escalation, bounded retry, and low-frequency consistency
audit. Production isolates a live authoritative lease from foreground path polling, rotates due
bounded work across roots, and preserves migrated v17 location identifiers during incremental
identity backfill. `docs/acceptance/r2c-f-recovery-consistency.md` records the original controlled
fixtures plus post-integration hardening at 402 Rust tests total, all Flutter tests, both Windows
integration suites, and the Windows Release gate.

R2c-G - USN downtime catch-up:

- accept a focused ADR;
- implement per-volume watermarks, continuity validation, root filtering, candidate enqueueing, and
  explicit fallback;
- validate changes made while Ame is closed.

Status: **complete and audit-hardened on 2026-08-19**. ADR 0022 owns watcher-first bounded Windows
USN catch-up, schema v19 checkpoints and durable cross-root handoff lineage, explicit authoritative
fallback, exact-case reconstruction, preview ownership, and fail-closed prerelease repair.
`docs/acceptance/r2c-g-usn-downtime-catch-up.md` records controlled fixtures, 391 Rust tests with
five existing explicit ignores, all Flutter tests, both Windows integration suites, the Windows
Release and 10,000-file synthetic performance gates, and final independent approval with no
remaining findings. Direct journal candidates remained unavailable to the standard workstation
token; permission fallback passed without elevation or source mutation.

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

R2c status: **implementation complete; post-integration hardening awaits independent re-audit**.
R2c-A through R2c-H now provide the
normalized observation contract, live Windows watcher, durable queue, atomic incremental
publication, production UI lifecycle, authoritative recovery, downtime catch-up, and target-scale
catch-up, queue, storage, and source-safety evidence required by this stage. Target-scale
authoritative recovery timing remains extended R10 evidence. A later full-range review identified
live authoritative lease expiry, migrated identity backfill, target-evidence scope, and bounded-root
fairness gaps; the current head closes those paths and requires a fresh independent audit before the
PR returns to Ready.

#### R2c.12 Acceptance evidence

R2c is not complete until all applicable evidence exists. Its preview evidence is limited to
retain-or-invalidate behavior caused by automatic source changes; cache capacity, reclamation,
manual cleanup, storage relocation, and restart reconciliation remain R2b-owned contracts.

- create, modify, same-volume rename/move, same-path replacement, and removal update the gallery
  incrementally;
- the same controlled changes produce deterministic retain, invalidate, or remove semantics for
  derived evidence without keying any future smart-album result to an absolute path;
- normal single-file changes do not trigger a complete root scan;
- duplicate, reordered, incomplete, and late events converge on correct final filesystem state;
- related changes publish atomically at one catalog revision;
- a database failure or cancellation preserves the last trustworthy catalog;
- queued work survives a controlled process interruption without duplicate publication;
- a watcher overflow or failure marks the root degraded and recovers through the documented ladder;
- an offline or disconnected root retains its last catalog and does not publish mass removals;
- controlled content edits, same-path replacement, identity-proven rename or move, temporary
  unavailability, and authoritative removal produce the documented retain, atomic dimensions
  replacement, preview invalidation, or removal eligibility without a transient layout change;
- OneDrive and other recall placeholders are not hydrated by observation, catch-up, or audit;
- the production Flutter gallery refreshes through bounded contracts and preserves stable identity
  and scroll position where the owning query remains valid;
- source removal, application shutdown, pause, retry, and cancellation do not hang the desktop app;
- every schema migration, adapter contract test, application test, Flutter state/accessibility test,
  and Windows integration scenario passes;
- Rust format, Clippy with warnings denied, Rust tests, generated bridge checks, Flutter analysis,
  Flutter tests, Windows Debug/Release build, and `git diff --check` pass serially;
- controlled fixtures and authorized real-root samples prove source bytes and entries are unchanged;
- USN catch-up, when included, covers closed-app changes and safely falls back when continuity is
  invalid;
- remaining filesystem limitations and measured performance are recorded honestly.

#### R2c.13 Explicit exclusions and anti-drift constraints

- Do not implement R3 fingerprinting, R5 classification, or R6 similarity to avoid finishing
  freshness.
- Do not build a second asset-identity or metadata pipeline for watcher events.
- Do not attach future classification or smart-album membership to a path or make Flutter infer
  retain-or-invalidate policy from a filesystem event.
- Do not accept platform notifications as authoritative state or assume they are ordered and unique.
- Do not full-scan the approximately 259 GB library in response to every change.
- Do not place the watcher, queue, USN, or SQLite policy in Flutter.
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
- installer, signing strategy, diagnostics export, and recovery documentation;
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
9. closed-application change catch-up and forced notification/journal fallback during R2c;
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
- Do not treat filesystem notifications or USN records as authoritative file or asset state.
- Do not respond to ordinary source changes by repeatedly scanning every configured root.
- Do not mutate source or target media during R8, and do not execute an R9 action without fresh
  authorization bound to the exact immutable plan and current-state revalidation.
- Do not silently change the UI framework, bridge, database, taxonomy, or reference policy.
- After context compaction, query recent task history and verify live files before resuming.

## 10. Current active stage

Active stage: **R3 - exact duplicate understanding**

Active slice: **not yet selected**. R2c-A through R2c-H are complete and independently
audit-hardened; no R3 implementation is included in the R2c closeout.

Planned next work: evaluate the exact-fingerprint engine and define the first R3 vertical slice under
the R3 scope and acceptance boundaries above.

R2b implementation, deterministic preview-lifecycle correctness, retained-catalog interaction
Profile, real-library catalog parity, Daily, Windows Release, and bounded source-readable preview
performance gates are complete. R2b was accepted on 2026-08-13 and R2c was accepted on 2026-08-19.
R3 may now begin, while the R2b interaction and source-safety contracts and the R2c freshness
contracts remain regression boundaries rather than migration work to repeat.

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

This snapshot was synchronized on 2026-08-19 against the live working tree and fresh local gates.
The live working tree, current schema, accepted ADRs, and fresh verification remain authoritative;
this roadmap does not preserve drifting commit hashes or duplicate complete test transcripts.

- R0 and R1 are accepted. The Rust-owned SQLite catalog, Flutter/Rust bridge, external preview
  storage, resumable multi-root scanning, atomic publication, per-file issue isolation, file
  identity, and revision-safe bounded queries are connected end to end.
- The catalog schema is v19 and the storage-settings schema is v2. Schema v17 introduced the
  durable normalized change queue, root-generation tombstones, lease/retry state, catalog-revision
  evidence, bounded terminal-row retention, and permanent highest-generation authority. Schema v18
  adds authoritative scan ownership, generation and queue-watermark capture, previous-snapshot
  preservation, consistency-audit evidence, normalized historical relative paths, and single-scan
  root ownership. Schema v19 adds exact per-volume catch-up checkpoints, bounded queue and full-scan
  lineage, normalized cross-root identity handoff, exact-case path lookup, preview-repair authority,
  and fail-closed relational validation without losing the v16 preview ownership reconciliation or
  earlier root, scan, asset, location, frontier, capture-evidence, identity, and query evidence.
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
  queue port into schema v17. Stable change IDs, 500 ms configurable stabilization,
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
  metrics shows `需要核对`. Background refresh distinguishes applied, busy, superseded, and failed
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
- R2c-F is complete and audit-hardened. The application leases one bounded authoritative
  subtree, root, or freshness-gap row, enumerates no more than 4,096 entries and 128 affected paths,
  and publishes the complete retain/add/change/move/remove set at one catalog revision. Oversized
  work remains durable and escalates to the existing resumable full scanner. Schema v18 captures the
  root generation and unresolved queue high watermark when a scan begins, freezes only pre-watermark
  work for that scan without consuming retry attempts, preserves later evidence, and releases only
  its own rows on abandonment. It normalizes historical relative paths, persists a fail-closed
  previous-snapshot requirement for unreadable rescans, and records authoritative audit completion.
  Production runs both bounded reconciliation and full-scan escalation outside the polling mutex,
  only after a healthy observer establishes continuity, with cancellable shutdown and bounded
  per-root retry whose failure history survives bounded re-escalation. Placeholders and other
  uninspectable entries remain unresolved through full-scan escalation without hydration or false
  audit success, and v17 path normalization preserves legacy location identity for both healthy and
  unavailable files. Durable lifecycle ownership separates foreground scans from bounded multi-root
  production recovery; foreground path polling cannot reclaim a slow live authoritative lease,
  bounded authoritative root selection rotates fairly, timed-out workers block restart, future or
  exhausted retry rows do not create empty workers, and one root cannot own overlapping active
  scans. Incremental identity backfill retains a normalized v17 location's legacy identifier. The
  seven-day consistency audit
  cannot project freshness before publication.
  Controlled fixtures did not access a real library. The complete Daily and Windows Release gates
  passed on 2026-08-18, and final independent audit returned no Critical, High, Medium, or Low
  findings.
- R2c-G is complete and audit-hardened. The application starts live observation before one bounded
  per-volume Windows USN catch-up, validates journal and catalog continuity, atomically enrolls all
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
  completed R2c-H acceptance.
- R2c-H is complete and audit-hardened. The controlled production watcher records bounded idle,
  event-to-visible, storm-coalescing, restart, shutdown, queue, and storage evidence. The explicitly
  authorized two-root workload uses a read-only retained catalog and isolated SQLite backup,
  performs catch-up discovery without leasing or publishing authoritative work, and verifies all
  85,556 source entries plus 32 deterministic local byte samples before and after. Physical path
  aliases fail before writes, Cargo descendants are contained by one kill-on-close Job Object, hash
  reads remain bounded after open, and final memory/deadline samples cannot bypass their ceilings.
  This target-root phase does not claim authoritative recovery convergence or lease timing; those
  target-scale measurements remain extended R10 evidence.
  The complete Daily, Windows Release, and 10,000-file synthetic gates passed on 2026-08-19, and
  final independent audit returned no Critical, High, Medium, or Low findings.
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
