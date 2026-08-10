# Cedarflake Ame Roadmap

Status: active delivery plan

Last confirmed with the user: 2026-08-10

Last implementation-status synchronization: 2026-08-10

Repository: this repository root

Canonical discovery entry: the repository-root `AGENTS.md` points to `docs/roadmap.md` and
requires every new project session, post-compaction continuation, roadmap-status review, and
product-work delegation to read it completely. This file is the only active roadmap copy; do not
fork or mirror it into another editable roadmap.

This roadmap is stored in the repository so delivery intent survives workstation and session
changes. Durable engineering and architecture rules remain in `AGENTS.md`; accepted technical
decisions remain in `docs/architecture/`.

## 1. Product definition

Cedarflake Ame is an independently implemented, local-first Windows photo-library application for
very large personal collections. Its core distinction is progressive large-library indexing with
exact and perceptual duplicate understanding integrated into one unified gallery.

The real target library is approximately 259 GB across two machine-local roots:

- `local-primary`: the primary locally stored image library;
- `cloud-primary`: the primary cloud-backed image library.

Their exact filesystem paths and machine-specific identity are stored only in the ignored
`.agents/local-context.toml` mapping. Repository documents, tests, logs, and commits use only these
logical IDs. The mapping is discovery data and does not itself authorize source mutation or a new
real-library acceptance run.

Ame must not require a second complete copy of these sources. Cataloging, browsing, and analysis do
not modify original media. File-changing operations enter the product only in the separately
approved safety stage defined by R8.

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
- Automatic common-type classification is deliberately late because its quality must be evaluated.
- Classification result groups are model-managed smart albums, not user albums or ordinary filter
  choices. Users may browse them but cannot create, delete, rename, or edit their membership.
- Person recognition, face clustering, identity naming, pixel editing, RAW development, cloud sync,
  and a Lightroom-style editor are outside this roadmap.
- Delete, move, copy, rename, quarantine, and recycle-bin execution remain unavailable until a later
  explicitly approved safety stage.

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

On 2026-08-07 the user confirmed that this structure is sufficiently decided for implementation to
continue. R2a may refine spacing, responsive behavior, component details, and wording through visual
review, but it must not reopen or silently reverse these accepted information-architecture choices:

- one unified gallery rather than peer folder, timeline, category, search, or duplicate pages;
- sources and albums only in the sidebar;
- duplicate display and review inside the gallery filter menu rather than a standalone action;
- temporary action-specific import progress rather than permanent task navigation;
- a plain-language settings page opened from the global gear rather than sidebar or engineering
  dashboards;
- Simplified Chinese as the initial application language.

ADR 0003 predates these corrections and currently conflicts with this section by retaining a
standalone duplicate action, task activity, and sidebar settings. Before R2a implementation proceeds,
its status must be amended so it cannot override the latest user decision. After the interactive
prototype is accepted, a replacement UI ADR must record the verified layout and mark the obsolete
parts of ADR 0003 fully superseded.

Simplified Chinese (`zh-CN`) is the only initial application language. All user-visible titles,
actions, menus, tooltips, accessible labels, progress text, empty states, confirmations, and error
explanations use concise Simplified Chinese. File names, paths, metadata values, and quoted operating-
system details retain their original content. Internal code identifiers and stable error codes remain
English and must be translated into understandable Chinese at the presentation boundary rather than
leaking raw implementation messages.

R2 does not add a language selector or a complete localization runtime. User-facing copy is kept in
a presentation-owned string catalog instead of being scattered through widgets, so formal i18n can
be introduced later without rewriting the UI structure.

R2 begins with an interactive UI prototype before additional business integration. Prototype-only
fixtures may demonstrate confirmed states, but unavailable controls must not ship as dead production
actions and a screenshot is not feature-completion evidence.

### 4.2 Global shell

```text
Ame | [在图库中搜索] | 导入 | 设置 | 最小化 / 最大化 / 关闭
```

The global `导入` action and the sidebar add-source icon are two discoverability entry points into
the same Ame-owned `AddLibraryRoot` use case. In the current image-library scope, both open the same
folder picker and share validation, progress, cancellation, and error state. They must not create two
independent import implementations. A future device-import workflow may expand the global action
only after that scope is separately accepted.

There is no permanent Task button or task-center navigation entry. While an import or library update
is active, a temporary bottom progress surface uses the concrete action name, reports progress and
offers cancellation. After completion, the same surface changes to an explicit completed result,
retains the final counts, removes cancellation, and remains until the user acknowledges it.

### 4.3 Left sidebar

The sidebar contains only navigation and source scope:

- `图库` with a trailing folder-plus add-source action;
- an expandable `相册` section when R6 is functional, containing the system-provided `收藏夹` and
  user-created album groups;
- imported folders in one aligned source list, without separating OneDrive and local folders into
  different navigation hierarchies;
- expandable folder trees when functional.

`收藏夹` is not a separate data model or a peer outside the album system. It is the built-in album
group for the default collection workflow. User-created groups use the same durable membership
contract. Activating any album entry only scopes the unified gallery to that group and never moves,
copies, renames, or deletes source files.

When R7 is functional, the same `相册` navigation area also contains a distinct `智能相册` subsection
whose result groups are created and maintained only by the admitted classification model. Smart
albums are read-only navigation projections: users cannot create, delete, rename, manually add, or
manually remove their images. They do not use ordinary album membership and never appear as targets
in the `加入相册` dialog.

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

`加入相册` becomes visible only after R6 connects durable album membership. It is an action in this
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

Versions before R8 do not show disabled delete, move, or copy placeholders. Those actions are
introduced only when their implementation and safety milestone are accepted.

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

The menu contains two independent single-choice groups followed by one command. Initial defaults
are `显示子文件夹 + 显示所有文件`. The first group selects whether the current source includes
descendant folders. The second group selects one exact-duplicate display mode:

- `显示所有文件`: show every physical file instance;
- `合并完全相同图片`: merge byte-identical copies into one representative item;
- `仅显示重复图片`: show only exact duplicate groups.

`审查重复组` is a contextual command at the bottom of the same filter menu. It enters review in the
existing gallery canvas and does not create a new page or sidebar entry.

R2a may exercise the confirmed duplicate choices with deterministic fixtures. They remain hidden in
the production shell until R3 connects trustworthy exact-duplicate evidence.

The current product indexes images, so the Microsoft Photos `所有媒体 / 照片 / 视频` group is not
copied into the early UI. Video filters appear only after video indexing becomes accepted product
scope. Classification and category choices do not become ordinary filter items. When R7 is
functional, the model publishes them as read-only smart-album result groups under `相册`.

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
- One deterministic layout snapshot derives final row membership, item rectangles, cumulative row
  offsets, date anchors, and total extent from that manifest. Placeholder, failed-preview, and
  decoded states use the same rectangle. Preview completion or eviction must never recompose rows,
  change total extent, or move the viewport.
- Full `LibraryAsset` details are queried in bounded revision-safe keyset pages. The current
  controller is known to merge those pages into a growing `state.assets` list; Profile evidence
  determines its target-library cost before a high/low-watermark cache replaces that retained-list
  baseline. No single page or 160-item replacement window is the gallery's global presentation
  model.
- Preview readiness lives in an identity-keyed store outside layout state. Visible and near-
  viewport previews receive priority, obsolete generations cannot publish, and expensive decoding
  may be deferred during high-velocity scrolling without deferring layout geometry.
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

### 4.10 Required UI prototype states

The R2 UI prototype must make these states reviewable with deterministic fixtures:

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

The global gear opens one shallow settings page. It is not a sidebar destination, engineering
dashboard, or hierarchy of abstract configuration pages. The visual pattern follows Microsoft
Photos: a clear `设置` title, a centered readable column, plain section headings, and full-width
rows containing an icon, a user-facing title, one short explanation, and a control on the right.

Initial settings are limited to behavior that is understandable and connected end to end:

```text
个性化
  应用主题                 跟随系统 / 浅色 / 深色

浏览
  查看图片时的鼠标滚轮     放大或缩小 / 上一张或下一张
  打开图片时               适应窗口 / 实际大小

相册（R6 接通后）
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

Storage labels and descriptions must explain the consequence instead of exposing implementation
terms:

- `图库数据位置`: `保存 Ame 的图库索引和设置，不会移动或复制原图片`;
- `缩略图位置`: `保存可重新生成的预览，不会保存第二份原图`;
- `缩略图最大占用空间`: `达到上限后自动清理较少使用的缩略图`;
- `清理缩略图`: `缩略图会在需要时重新生成，不会删除原图片`.

Changing a storage location must show whether restart or migration is required before confirmation.
Clearing thumbnails must name the rebuild cost and confirm that source files are untouched. Theme,
viewer, album, and storage choices persist across restarts. `加入相册前询问` initially defaults to
on, and `默认加入的相册` initially defaults to `收藏夹`. When prompting is on, the configured group
is preselected in the dialog and may be changed for that operation. When prompting is off, the same
setting is the direct destination. These rows remain absent until R6 connects album membership end
to end.

Do not expose database schema, catalog revisions, adapter or engine names, worker counts, queue
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

Classification remains an R7 feature. It later appears as model-managed smart albums that scope the
same unified gallery without becoming filter choices, peer gallery applications, or editable user
albums.

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
- responsive, keyboard, focus, scaling, and accessibility behavior.

R2b is delivered as small end-to-end slices after UI acceptance. A fixture-backed control, bridge
type, database query, or rendered screenshot alone does not complete a use case.

R2b does not require every optional ADR 0014 scale adaptation to be enabled merely to complete a
migration checklist. Its remaining delivery order is:

1. complete the applicable gates for the latest scan-lifecycle and Explorer-reveal maintenance;
2. freeze the current wheel, time-rail, jump, and resize behavior as the comparison baseline;
3. run resource-bounded Profile and long-session observation against a retained catalog without a
   new real-root import;
4. record retained detail count, process working set, garbage collection, page-publication copy
   time, frame timing, preview latency, programmatic scroll writers, and flat-manifest cost;
5. change only a condition that exceeds its recorded budget, one variable at a time;
6. compare every change with the frozen baseline and reject a nearby-return, reversal, distant-jump,
   resize, or native-input regression;
7. pass real-library parity and Windows Release verification before closing R2b.

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
- Unchanged file: do not repeat metadata analysis or preview generation and do not create a
  meaningless visible refresh.
- In-place edit: preserve logical asset identity when accepted platform evidence supports it, while
  invalidating stale preview, metadata, fingerprint, similarity, and classification evidence.
- Same-volume rename or move: preserve the asset when identity matches and replace its location
  atomically; compatible derived evidence follows the stable asset instead of remaining attached to
  an obsolete path.
- Replacement at the same path: create a new asset and prevent it from inheriting the former
  file's derived evidence or user decisions.
- Removal: remove the published location only after an authoritative observation; do not let a
  delayed delete remove a new replacement now occupying that path. When the last active location is
  authoritatively removed, current derived projections must no longer surface the asset.
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
- Every bounded delta exposes enough stable identity and evidence disposition for later analysis
  consumers to retain compatible results after a rename, invalidate them after content change or
  replacement, and remove them from current projections after authoritative deletion. R2c defines
  this contract without implementing R7 classification.
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

R2c-B - live Windows observation:

- add the admitted recursive watcher adapter;
- connect one bounded lifecycle per available root;
- keep callbacks lightweight and cancellable;
- verify start, root removal, adapter failure, and window-close shutdown behavior.

R2c-B is complete only when controlled real filesystem changes produce Ame-owned intents without
blocking UI, decoding media in the callback, or growing memory without limit.

R2c-C - durable queue and coalescing:

- add forward migration and durable leasing/retry storage;
- implement debounce, path/subtree supersession, root-generation protection, crash recovery, and
  bounded cleanup;
- expose structured queue health and delay metrics.

R2c-C is complete only when an application terminated after enqueue resumes the same work and a
burst of repeated notifications produces the minimum necessary reconciliation.

R2c-D - incremental delta publication:

- connect the existing file-identity and media-safety rules;
- implement unchanged, add, edit, rename/move, replacement, and removal transactions;
- invalidate only incompatible derived evidence;
- increment revision and refresh bounded UI state atomically.

R2c-D is complete only when every fundamental change is reflected without a normal root-wide scan,
failed transactions leave the old catalog unchanged, and source media remains untouched.

R2c-E - production UI and lifecycle:

- start and stop synchronization with the desktop application;
- connect simple Chinese freshness and degraded states;
- connect `更新图库` to the application use case;
- preserve active source, filters, selection, preview, and gallery scroll anchor through a bounded
  refresh.

R2c-E is complete only after the real user path works end to end without a permanent task entry or
manual re-import.

R2c-F - recovery and consistency:

- force overflow, watcher failure, offline roots, database rollback, and repeated source changes;
- implement the escalation ladder and low-frequency audit;
- prove that recovery does not publish mass false removals or claim health early.

R2c-G - USN downtime catch-up:

- accept a focused ADR;
- implement per-volume watermarks, continuity validation, root filtering, candidate enqueueing, and
  explicit fallback;
- validate changes made while Ame is closed.

R2c-H - large-library reliability:

- run small and synthetic correctness fixtures first;
- then use the already authorized real roots in read-only mode, serially and with isolated derived
  storage;
- measure idle overhead, event-to-visible P50/P95 latency, event-storm coalescing, persistent queue
  growth, transaction time, startup catch-up, memory, database growth, cancellation, and recovery;
- verify source bytes, source entries, and cloud-placeholder state remain unchanged.

#### R2c.12 Acceptance evidence

R2c is not complete until all applicable evidence exists:

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

- Do not implement R3 hashing, R5 similarity, or R7 classification to avoid finishing freshness.
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

The gallery can show every physical instance, merge byte-identical copies, or display only exact
duplicate groups. The user can inspect every path and estimated redundant size without modifying a
file.

Scope:

- written engine candidate evaluation and contract tests;
- size grouping, candidate pruning, full identity evidence, and versioned analysis runs;
- duplicate group representatives and `AssetLocation` expansion;
- exact duplicate display modes and review command within the gallery filter menu;
- contextual duplicate-group review in the existing canvas;
- explicit distinction between logical group selection and physical-location selection.

### R4 - Metadata and local search

User outcome:

Search and filters compose with source, time, and duplicate state using filenames and trustworthy
local metadata.

Scope:

- EXIF and supported metadata extraction through an admitted adapter;
- capture-time source and fallback evidence;
- camera and basic metadata display;
- SQLite FTS5 search with bounded result windows;
- composed filters and recalculated time distribution;
- no embedded metadata writes.

Completion of R4 defines the first genuinely useful catalog-and-analysis release before controlled
file operations are introduced in R8.

### R5 - Perceptual similarity

User outcome:

The user reviews visually near-duplicate candidates such as recompressed, resized, or rotated images
without confusing them with byte-identical copies.

Scope:

- comparative candidate-engine evaluation;
- explainable threshold and evidence;
- candidate groups, side-by-side comparison, confirm, and ignore decisions;
- immutable analysis runs and durable user review state;
- no automatic deletion or exact-duplicate label for perceptual results.

### R6 - Personal organization

User outcome:

The user can place images in the built-in Favorites album and user-created album groups, then
maintain tags, ratings, and other durable manual decisions without changing original media.

Scope:

- durable user-owned catalog data separated from rebuildable caches;
- one album-membership model containing the system-provided `收藏夹` and user-created groups rather
  than a parallel favorite flag and album system;
- an expandable `相册` sidebar section whose entries scope the same unified gallery without changing
  membership;
- one `加入相册` command in the upper-right selection action area, with a multi-group Material
  chooser, a default checked group, existing-membership and mixed-selection states, group creation,
  confirmation, errors, and undo;
- persisted settings for whether the chooser appears and which group is selected or used by
  default; the initial defaults are prompt enabled and `收藏夹`;
- durable many-to-many membership that survives reindexing and does not move or copy source media;
- tags, ratings, review state, and data export or backup strategy;
- migration and restoration evidence.

### R7 - Local classification and semantic discovery

User outcome:

The user can browse model-generated smart albums for common primary image types and search images
with natural-language queries. Smart albums stay current when indexed files are renamed, moved,
edited, replaced, deleted, or become temporarily unavailable.

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

- written model and runtime admission evidence;
- local inference, model provenance, versioned parameters, and bounded model storage;
- immutable classification results associated with stable `Asset` identity and `AnalysisRun`, never
  an absolute path or an editable ordinary-album membership;
- a model-managed smart-album projection for each published result group, including confidence and
  model-generated `needs review` results;
- no user creation, deletion, rename, manual add, manual removal, or direct membership correction
  for smart albums; users only browse the model's published result groups;
- atomic replacement of the visible smart-album projection when a model run is published, while
  retaining older run evidence for traceability rather than mixing two active versions;
- consumption of R2c evidence disposition: compatible classification survives an identity-proven
  rename or move, content edits and same-path replacements invalidate old results, authoritative
  deletion removes the asset from current groups, and unavailable roots retain the last trustworthy
  result with explicit availability state;
- bounded reanalysis for newly indexed or invalidated assets without rebuilding every result group;
- CLIP-style semantic search and semantic similarity through a separate evidence type;
- reanalysis without erasing historical engine results;
- no face or identity recognition.

### R8 - Safety planning and explicitly authorized operations

R8a first provides only an immutable, non-executable dry-run `OperationPlan` containing intended
actions, reasons, expected file state, targets, cross-volume warnings, and estimated space impact.

R8b may be planned only after fresh user approval. It requires execution-time revalidation,
same-volume quarantine or system recycle-bin integration, operation logs, recovery evidence, and
clear partial-failure behavior. Permanent deletion is not a default operation.

### R9 - Large-library maturity and release readiness

Scope:

- cold and warm scan performance;
- cancellation latency and crash recovery;
- peak memory and cache-size enforcement;
- catalog migration and application upgrade tests;
- installer, signing strategy, diagnostics export, and recovery documentation;
- formal i18n infrastructure and additional locale catalogs only after product copy is stable and
  the supported locales are separately confirmed;
- controlled read-only combined scan of both real roots;
- regression comparison against recorded Lap behavior without importing Lap code.

## 7. Large-library test ladder

Large testing starts during R1 rather than waiting for R9:

1. deterministic fixtures for corrupt, locked, unavailable, Chinese, long-path, and wrong-extension
   media;
2. synthetic thousands and tens-of-thousands of paths and catalog rows;
3. virtual-gallery stress data large enough to exercise timeline jumps and lazy disposal;
4. controlled read-only scan of `local-primary`;
5. controlled read-only scan of `cloud-primary` after availability checks;
6. controlled read-only combined scan;
7. warm incremental scan after known additions, removals, and modifications.
8. live create, modify, rename, replacement, removal, and event-storm reconciliation during R2c;
9. closed-application change catch-up and forced notification/journal fallback during R2c.

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
- Do not expose classification, category filters, smart albums, or model placeholders before R7.
- Do not represent classification as an ordinary filter or editable album membership. R7 smart
  albums are read-only model projections over current trustworthy catalog and analysis evidence.
- Do not let users create, delete, rename, add to, or remove from a smart album result group.
- Do not key smart-album membership by path or allow rename, move, edit, replacement, deletion, or
  root unavailability to leave stale visible results.
- Do not turn internal scan, preview, hash, or analysis jobs into a permanent Task navigation entry.
- Do not display a chronological time rail or date headings while the active sort is by name.
- Do not sort only the currently loaded Flutter window; every sort and direction requires a bounded
  complete-result query contract.
- Do not allow a third-party engine to redefine Ame's domain or database.
- Do not confuse R1 explicit-rescan reconciliation with R2c automatic detection and catch-up.
- Do not start R3 until R2c can prove that the catalog does not silently remain stale.
- Do not treat filesystem notifications or USN records as authoritative file or asset state.
- Do not respond to ordinary source changes by repeatedly scanning every configured root.
- Do not mutate source media before the authorized portion of R8.
- Do not silently change the UI framework, bridge, database, taxonomy, or reference policy.
- After context compaction, query recent task history and verify live files before resuming.

## 10. Current active stage

Active stage: **R2b - production behavior integration**

Planned next stage: **R2c - continuous directory synchronization and incremental indexing**.
R2c does not become active until R2b is accepted; R3 remains blocked behind R2c catalog-freshness
acceptance.

Current priority:

1. preserve the current wheel, time-rail, jump, and resize interaction as the user-directed
   preservation baseline; first run resource-bounded Profile-mode and long-session observation on a
   retained catalog, recording frame time, memory, retained detail growth, preview latency, and
   programmatic scroll ownership without changing the interaction implementation;
2. design the ADR 0014 bounded detail-page cache against that baseline, but publish it only after
   focused parity evidence proves that returning to nearby content, rapid reversal, distant jumps,
   and native wheel or touchpad input are no worse. Consolidate only programmatic scroll writers
   proven by traces to conflict; native Flutter `Scrollable` movement must remain immediate and
   must not be routed through an asynchronous intent queue;
3. retain the flat exact manifest while it remains inside the recorded memory and frame budgets.
   Introduce block summaries plus bounded exact blocks only for result sizes that exceed those
   budgets, then validate the 79,000-, 250,000-, and 1,000,000-item cases before enabling the
   hierarchical representation or removing any rollback path.

The current controller's retained-detail growth is not an open hypothesis. Forward paging rebuilds
a map across all retained `state.assets`; backward paging rebuilds identity sets and a merged list.
Observation measures the resulting memory, garbage collection, copy cost, frame impact, and whether
repeated forward and reverse movement reaches a stable resource range. If the target workload stays
inside its explicit budget, the cache may remain guarded work; if it exceeds the budget, the cache
uses high and low watermarks with hysteresis rather than aggressive page-by-page eviction.

On 2026-08-10 the user reported that the current gallery interaction feels acceptable and directed
the project to avoid speculative or migration-driven performance changes that could create a
negative optimization. Remaining ADR 0014 slices are therefore implementation options behind
measured thresholds and behavior-parity gates, not authorization to rewrite the current scroll hot
path merely to complete the migration sequence. Bounded-memory requirements remain binding, but
their production implementation must preserve or improve the reported interaction baseline.

The visible Flutter shell is the active R2b production surface and must not be discarded or treated
as a fixture-only prototype. It is not yet a fully accepted product release. The current priority is
not classification and not additional analysis engines.

### 10.1 Historical implementation and verification evidence

<details>
<summary>Expanded repository evidence through the completed R1 acceptance</summary>

- Git history is now established on `main`; the verified 2026-08-09 baseline before the current
  maintenance work was `010c870 docs(structure): document repository ownership`. Live Git history
  remains authoritative if this historical identifier later changes.
- The 2026-08-10 R2b production-integration changes are committed through
  `58c2d97 style(dart): format bootstrap code`; live Git history remains authoritative for later
  commits and rollback boundaries.
- ADRs 0001 through 0008 record the Flutter/Rust stack, Rust-owned catalog boundaries, admitted
  dependencies, storage governance, capture-time extraction, Windows file-identity reconciliation,
  and capture-time gallery keyset. ADR 0003 contains superseded legacy UI choices and is not current
  authority for the conflicting R2a surfaces identified in section 4.1.

- Flutter 3.44.9, stable Rust 1.97.1, `flutter_rust_bridge` 2.12.0, and code-generation tooling are
  installed; Flutter and Cargo are present in the Windows user PATH;
- the Material 3 gallery, directory picker, Riverpod controller, typed bridge, bounded Rust scan,
  cancellation, per-file issues, external preview cache, and atomic SQLite publication are
  implemented;
- schema v2 persists `LibraryRoot`, `Asset`, and `AssetLocation` separately and migrates the R0 v1
  catalog forward without discarding the active published location;
- the Rust application API returns a bounded catalog snapshot; Flutter loads it at startup and
  after atomic publication instead of treating streamed scan candidates as trusted gallery data;
- multiple completed roots remain visible in a scrollable source list and a rescan reuses the prior
  asset identity for the same location;
- schema v3 adds an atomically incremented catalog revision and forward migration from both prior
  schemas;
- schema v4 persists scan parameters, traversal checkpoints, and progress counters; migrations from
  v1, v2, and v3 are tested, while an old uncheckpointed running task is never guessed as resumable;
- schema v5 persists the current directory and pending-directory frontier; migration from v4 marks
  old active tasks unrecoverable because their missing frontier cannot be reconstructed safely;
- schema v6 snapshots directory entries in SQLite in 256-entry batches and consumes them through
  256-entry keyset windows; active v5 tasks are marked unrecoverable because their absent entry
  snapshot cannot be reconstructed safely;
- schema v7 records pending, ready, and failed preview state with structured failure evidence;
  existing v6 preview paths migrate as ready;
- schema v8 records metadata engine identity and version plus normalized local capture time, optional
  offset, source tag, and bounded raw evidence; existing v7 locations migrate as explicitly
  unanalyzed;
- schema v9 records optional Ame-owned file-identity evidence; existing v8 locations migrate with
  identity explicitly unknown and are backfilled conservatively by later scans;
- schema v10 adds indexed active identity, location identity, and asset-reference lookup; v9
  migration preserves identity evidence, and a query-plan test proves terminal orphan cleanup uses
  the asset-reference index rather than a nested full-table scan;
- schema v11 adds the cross-root gallery-time index and replaces root/path pagination with a
  revision-protected keyset over capture availability, local capture time, modification time, root,
  and stable location identity;
- catalog rows load in bounded keyset windows; a publication invalidates old cursors explicitly,
  Flutter refreshes the first page, and page merges deduplicate by stable location identity;
- a 1,025-row SQLite fixture is read across multiple keyset windows without a duplicate or gap;
- Windows offline and recall attributes are checked before image content access; an actual file
  marked offline is skipped without decoding, hydration, or source-byte changes;
- an exclusively locked image is isolated as a structured per-file issue, while the scan completes;
- a source that disappears during final revalidation marks the scan stale instead of publishing;
- an image behind a path longer than 260 characters is discovered and previewed without changing
  the source, and the Windows runner manifest is explicitly long-path aware;
- deterministic traversal checkpoints are written every 128 entries, staged rows are replay-safe,
  and only unexpectedly interrupted running tasks are eligible for automatic startup recovery;
- a 130-image interruption fixture resumes from the checkpoint and publishes 130 distinct locations
  without a duplicate or gap; a missing checkpoint path becomes stale instead of being published;
- Flutter restores the persisted scan identity, parameters, and counts at startup and exposes an
  explicit resuming state; user cancellation remains terminal;
- the upper action area can pause a running scan; `paused` persists its private checkpoint across
  application restarts but never resumes until the user explicitly continues it;
- a Rust pause/resume fixture proves the staged scan is not published while paused and that explicit
  resume publishes every location once; Flutter tests cover paused-state restoration and controls;
- a deep-tree interruption fixture leaves completed directories behind, resumes only the persisted
  current directory, and publishes 130 unique locations; terminal tasks clear their frontier;
- a 1,025-entry single-directory fixture is enumerated and consumed through bounded windows without
  a duplicate or gap;
- a fixed configuration database stores catalog location, versioned preview location, and preview
  budget independently from the catalog it controls;
- active storage is frozen for one process lifetime; path or budget updates report restart-required
  state and never migrate or delete existing data automatically;
- storage paths overlapping an imported source root are rejected, and catalog relocation is locked
  after a source has been imported until a verified migration workflow exists;
- preview capacity includes existing artifacts and uses atomic reservation before publication; an
  exhausted budget becomes an isolated per-file issue without a partial cache file or source-media
  mutation;
- scan discovery probes image dimensions without full pixel decoding; lazily built Flutter tiles
  request previews through a queue limited to two active decodes, and queued off-screen requests are
  cancelled;
- failed preview requests retain structured evidence and require explicit retry; a missing derived
  preview is exposed as pending and rebuilt when the tile becomes visible;
- catalog loading reports configured roots as available, missing, inaccessible, or offline using
  root metadata only, without walking or hydrating their contents;
- capture-time inspection uses the admitted `kamadak-exif` 0.6.1 adapter behind Ame-owned ports;
  source values are calendar-validated, timezone absence remains unknown, raw EXIF parsing is capped
  at 4 MiB, and malformed metadata does not reject an otherwise readable image;
- unchanged sources reuse capture evidence only when metadata engine identity and version match;
  incompatible evidence is reanalyzed and all provenance survives the generated bridge;
- incremental rescans use Windows volume and 128-bit file ID as local reconciliation evidence:
  same-volume rename and in-place edit preserve logical asset identity, changed state invalidates
  derived evidence, a replacement at the same path receives a new asset, and removed locations are
  published only at the atomic snapshot boundary;
- a five-scan controlled fixture covers rename, edit, same-path replacement, and removal; inactive
  snapshot rows, terminal staged locations, and orphan derived asset rows are cleaned without
  touching source media outside the fixture's explicit setup actions;
- a repeatable 10,000-file synthetic benchmark covers cold scan, warm scan, pause, persisted resume,
  cancellation, catalog growth, bounded working set, final row counts, and unchanged source bytes;
  the latest debug run records 22.570-second cold, 21.030-second warm, 26-millisecond pause,
  20.033-second resume, 117-millisecond cancellation, a 27,262,976-byte catalog, and a
  15,659,008-byte peak test-process working set;
- a separately ignored real-library acceptance harness now requires an exact authorization token,
  one explicit root and scan ID, disjoint persistent storage, explicit reuse of nonempty storage,
  and a second acknowledgement for OneDrive-like paths before source traversal can begin;
- its controlled PowerShell 5.1 regression proves pre-access refusal, cloud and overlap guards,
  complete atomic publication, cancellation with zero staged rows, persisted pause and resume,
  process-memory reporting, retained evidence, and unchanged source bytes and entries; neither real
  root was used by this regression;
- on 2026-08-07 the user explicitly authorized read-only access to `local-primary` and
  `cloud-primary`, removed the waiting-for-authorization hold, and directed the
  project to continue; this authorization does not permit source mutation or cloud hydration;
- the controlled real-root sequence completed against isolated external storage: local-primary published
  30,629 locations in 106.905 seconds and cloud-primary published 48,384 locations in 154.698 seconds;
- cancellation trials left zero staged locations, responded in 66 milliseconds for local-primary and 231
  milliseconds for the authorized cloud-primary probe, and did not replace an already active root;
- the combined catalog contains two active roots and 79,013 active locations in 158,298,112 bytes;
  production `load_catalog` traversed all locations through 155 bounded windows at one stable
  revision without a duplicate or gap;
- real-scan reports retain structured issue-code evidence, representative messages, observed peak
  working set, and 35 plus 44 unchanged source-hash samples; full-library previews were not built;
- the legacy upper Material 3 engineering action area exposes storage status, cache usage, directory
  selection, budget selection, migration restrictions, and restart activation; this remains
  implementation evidence, not the accepted settings presentation defined in section 4.11;
- normal user imports are complete scans with no validation-only item or entry cap; explicitly
  bounded scans remain available to deterministic tests and controlled acceptance harnesses;
- Before the interrupted timeline integration, Rust format, Clippy with warnings denied, 52
  non-ignored Rust tests, Flutter analysis, and 19 Flutter tests passed;
- a Rust release dynamic library builds successfully;
- the Rust end-to-end fixture proves Chinese-path discovery, source-byte preservation, preview
  placement outside the source, and completed-catalog publication;
- Visual Studio Build Tools has the Desktop development with C++ workload, MSVC x64/x86 tools,
  CMake tools, and Windows SDK registered; `flutter doctor -v` recognizes the Windows toolchain;
- a Windows Debug runner build completes successfully at
  `build/windows/x64/runner/Debug/cedarflake_ame.exe`;
- Windows integration tests use an isolated catalog and cache, open and cancel the production
  directory picker, then import two controlled roots through the real picker;
- the Windows integration workflow reconstructs application state from SQLite twice and verifies
  multi-root persistence, corrupt-file isolation, real preview rendering, external catalog and
  preview placement, root availability, missing-preview regeneration, and unchanged source bytes
  and source entries;
- a Windows Release runner builds and launches successfully with
  `flutter run --release --no-resident`;
- both authorized real-library roots have completed Ame's read-only catalog acceptance;
- the local Lap reference remains outside the Ame repository.

</details>

### 10.2 Completed acceptance checkpoints

R0 acceptance result:

```text
Windows Debug and Release runner launch
-> production native picker cancellation
-> controlled fixture selection through the real native picker
-> Rust bridge scan, per-file issue isolation, preview rendering, and atomic catalog publication
-> catalog and previews outside the source tree
-> unchanged source bytes and entries
-> repository quality gates
-> accepted
```

Completed R1 slice:

```text
completed: persist LibraryRoot, Asset, and AssetLocation separately
completed: reload the last completed bounded catalog when application state is rebuilt
completed: add and retain more than one root without replacing prior roots
completed: replace the single bounded snapshot with revision-protected keyset windows
completed: isolate missing, locked, long-path, and Windows offline fixtures without hydration
completed: add deterministic traversal checkpoints and automatic interrupted-task recovery
completed: add explicit pause/resume while keeping cancel terminal and paused tasks non-automatic
completed: persist the current directory and pending frontier for bounded deep-tree recovery
completed: expose catalog and preview-cache locations and atomically enforced preview budgets
completed: window enumeration inside an extremely wide single directory
completed: report root availability without source enumeration or cloud hydration
completed: schedule bounded preview generation from lazily rendered gallery tiles
completed: persist trustworthy, versioned capture-time evidence without inventing timezone data
completed: reconcile unchanged, edited, renamed, replaced, and removed locations incrementally
completed: record synthetic large-library performance, memory, cancellation, and recovery evidence
completed: prepare and regression-test the explicitly authorized read-only real-root harness
completed: execute the authorized local-primary and cloud-primary read-only acceptance sequence
```

R1 acceptance result:

```text
authorized local-primary cancellation and cold scan
-> authorized cloud-primary cancellation and cold scan
-> two active roots and 79,013 active locations
-> production bounded catalog reload across 155 windows
-> unchanged sampled source bytes and no full-library preview generation
-> accepted
```

### 10.3 Production time-navigation foundation

The production gallery now uses Ame-owned Rust timeline-bucket and anchor types, SQLite complete-
result month distribution, revision-bound month or unknown anchor queries, generated Rust-Dart
bridge types, and the Flutter global virtual-scroll integration defined in section 4.8. The
right-side rail is no longer confined to the isolated R2a prototype.

The aggregate full-query extent, static placeholder slivers, settle-only wheel seek, and one
replacement asset window now remain only in the rollback path. The active equal-height manifest
path derives exact query-wide geometry and exposes it through an Ame-owned
`LibraryExactExtentSliver`: offset-to-index lookup is logarithmic, index-to-offset and item extent
are constant-time, and the render sliver publishes the exact content extent instead of estimating
unbuilt children. Flutter still owns lazy child creation, scrolling, focus, and semantics.

Real-library wheel, rapid time-rail, and resize evidence on 2026-08-09 rejected the interim model as
the production target. It showed layout recomposition during ordinary scrolling, slow pending-
preview fill, extended square placeholder walls during rapid navigation, and a second square-to-
justified transition when details became available. This confirms that the compact query-wide
layout index previously described as optional parity work is a required R2b foundation.

ADR 0014 is now the active accepted-for-validation correction. It introduces a chunked compact
query-wide manifest, deterministic final-geometry layout snapshots, a guarded bounded asset-detail
page-cache path, identity-keyed preview publication, programmatic navigation coordination,
latest-wins target loading, and logical-anchor resize preservation. Flutter's `Scrollable` retains
native relative movement on the single `ScrollPosition`; the programmatic path preserves Material
Slider behavior, complete-result date annotations, revision-safe keysets, and the read-only media
boundary accepted in ADRs 0009 through 0011.

ADR 0014's seven migration slices remain the architectural decomposition for any further work, but
the remaining slices are not an automatic delivery mandate. Existing aggregate placeholder and
replacement-window paths remain only as a rollback boundary until a measured need, focused parity
tests, and real-library evidence justify replacement; they must not receive further interaction-
specific fixes or be removed prematurely.

Migration slices 1 through 3 are connected. The Rust domain, application port, SQLite adapter, and
generated bridge expose revision-checked manifest chunks of at most 4,096 items. Flutter has an
Ame-owned chunk adapter, a sequential all-or-nothing loader, compact UTF-8 and typed-array flat
storage, an interim chunk-block over-budget representation, a revision/query-keyed provider that
publishes only a complete compatible manifest, deterministic final row snapshots, and the exact
render sliver. Provider disposal stops the loader before another chunk can publish. The interim
over-budget store still retains every exact block and the layout snapshot still allocates query-wide
exact offsets, so it does not yet satisfy ADR 0014's block-summary plus bounded-exact-block
requirement and must not be treated as the completed million-item fallback.

The equal-height production gallery now derives one query-wide deterministic layout snapshot from
that manifest. Loaded assets and unloaded placeholders occupy the same final rectangles in one
`LibraryExactExtentSliver`; preview or detail publication fills those rectangles without replacing
a square wall or changing row membership. While the complete manifest is unavailable, the rollback
equal-height path no longer paints generic square leading or trailing placeholder slivers. The
generic square painter remains only for the explicit square layout and as rollback code.

Preview readiness is now separated from the asset collection through an identity-keyed
`LibraryPreviewStore`. One tile subscribes to its own location identity, so preview completion does
not clone the complete asset list or republish query-wide geometry. Gallery demand is derived from
actual visible rows plus movement-direction and guard ranges. The bounded scheduler orders viewer,
visible, near-direction, guard, and idle work. Pending demand can be upgraded or replaced
atomically, and incompatible active generations cannot publish stale results.

Time navigation now uses a single active request plus one replaceable latest pending target.
Controller and presentation generations guard query, revision, target, state publication, loading
ownership, and post-frame alignment, so an obsolete completion cannot pull the viewport back.
Visible-range intents can invalidate a disjoint active seek even when the user reverses into an
already loaded range. The current controller still merges preceding and following detail pages into
one growing `state.assets` list and rebuilds full retained-detail collections during publication.
This is the known migration-slice-4 debt; Profile determines its target-library cost before a cache
is allowed to replace the current interaction baseline.

Aspect ratios are durable catalog data derived from stored width and height, so restarting Ame does
not decode source images again merely to recover tile proportions. The in-process layout snapshot is
keyed by query manifest identity, viewport width, thumbnail density, and sort key. A cross-startup
cache of final row rectangles remains an optional later optimization and must use that complete key;
it is not allowed to replace or contradict the catalog dimensions.

Resize now captures a query- and revision-bound logical anchor from the old snapshot before the
first size change, accepts only the newest width and viewport in a frame, and applies one
generation-guarded `scrollOffsetCorrection` during exact-sliver layout. The command is cleared after
the frame so rebuilding the render object cannot apply it twice. Programmatic resize and positioning
do not authorize pagination.

Resize-driven `ScrollStartNotification` events do not authorize catalog paging. Both the rollback
wall and the manifest-backed wall restrict proximity paging to real scroll updates or completion,
and defer provider-mutating page requests until the current frame has finished. This prevents a
window-size change from modifying Riverpod state during layout and leaving the gallery frame empty.

Focused 2026-08-09 evidence records 79,013 synthetic items as a 5,925,999-byte primitive flat
manifest built in 63 ms, 250,000 items as an 18,750,024-byte primitive flat manifest built in 104 ms,
and 1,000,000 items as a 73,006,860-byte interim chunk-block manifest built in 356 ms on the project
workstation. That million-item number is evidence of the present cost, not acceptance of the bounded
hierarchical design.

On 2026-08-10 the repository daily gate passed after the exact sliver, latest-wins lifecycle,
identity preview store, resize anchor, EXIF Orientation 1-through-8 correction, loading feedback,
and menu corrections were integrated: Dart formatting and analysis reported no issues; Rust
reported 74 passed and 3 explicitly ignored tests; every Flutter test file passed serially; the
Windows Debug build and native picker integration passed 2 of 2; bridge compatibility, Rust
follow-up checks, and whitespace validation passed. These are deterministic and controlled-fixture
results for that earlier baseline. They do not cover the later scan-lifecycle and Explorer-reveal
commits. ADR 0014 still requires resource-bounded Profile evidence and the authorized
79,013-location real-library parity run before the migration or R2b can be accepted.

### 10.4 R2a acceptance and R2b execution status

```text
completed foundation: capture-time keyset windows and explicit unknown capture-time ordering
completed foundation: authorized two-root read-only catalog acceptance with 79,013 locations
completed planning: exact current UI scope written into section 4
completed reconciliation: obsolete ADR 0003 surfaces marked superseded for R2a validation
completed stabilization: timeline controller race fixed and full Flutter/Rust gates passed
completed prototype: isolated deterministic Flutter entry implements the confirmed section 4 flow
completed verification: 17 focused gallery prototype and layout tests, 36 total Flutter tests,
                        zero Flutter analysis issues, Windows Debug build, and DPI-aware visual
                        inspection
rejected detail: provider-grouped source rows and the static evenly distributed time rail
completed correction: source rows flattened into one folder list with shared column constraints
completed investigation: Material 3 defines vertical Slider and stops, while Flutter 3.44.9 lacks
                         native orientation and only supports equidistant divisions;
                         low-adoption or stale replacement packages were rejected
completed implementation: Flutter Material Slider owns pointer, focus, keyboard, track, and handle;
                          a thin orientation adapter maps top-to-bottom time without divisions
completed implementation: nonuniform year/month annotations use complete-result content extent,
                          synchronize bidirectionally with gallery scroll, and retain unknown time
completed correction: year labels and the Slider use separate gutters; the continuous official
                      track, 28 px handle, month points, and arrow controls remain on one axis
completed correction: timeline-node centers use the Slider track coordinate instead of marker-box
                      origins; the current year/month node now coincides with the handle center
completed correction: the gallery no longer renders a second Flutter Scrollbar; the annotated
                      timeline Slider is the sole visible scroll-position control
completed correction: selection mode keeps Cancel outside the horizontally scrollable action area,
                      and the tested action exits to the normal browsing toolbar
completed correction: browsing actions align to the right edge of the gallery header
completed correction: justified rows balance aspect ratios, fill one gallery width, and bound sparse
                      row enlargement
accepted review: user accepted the revised R2a visual and interaction contract on 2026-08-07
completed architecture: ADR 0009 records the accepted contract and fully supersedes ADR 0003
completed production integration: the real bounded gallery and complete-result timeline share one
                                  stable global virtual-scroll model
implemented performance correction: drag writes are frame-coalesced and unloaded regions move
                                    immediately without catalog queries or asset-window replacement
completed reference correction: pointer release commits only the final time target, matching the
                                direct-scroll separation observed in Lap and WinUI
completed presentation correction: the drag label follows the active line, gray hover preview is
                                   absent during drag, and nearby date markers remain visible
implemented scroll correction: wheel input moves the virtual canvas immediately; settled viewport
                               observation may request missing details but never queues the native
                               delta as a programmatic position intent
implemented resize correction: layout publication no longer rebuilds the entire unified screen;
                               redundant anchor writes and per-pixel thumbnail decode keys are gone
rejected production target: real-library wheel, rapid time-rail, and resize evidence confirmed that
                            aggregate placeholder geometry plus one replacement window causes
                            visible layout substitution, blank or slow fill, and a second reflow
accepted architecture: ADR 0014 replaces that interim target with a compact query-wide layout
                       manifest, deterministic row snapshot, guarded detail-page-cache path,
                       identity-keyed preview store, native Flutter scrolling, and coordination
                       for programmatic position changes that require arbitration
completed implementation: migration slices 1 through 3 now have the revision-safe Rust query,
                          generated async bridge, flat and interim chunk-block Flutter stores,
                          cancellable all-or-nothing publication, deterministic query-wide layout,
                          and an exact-geometry lazy render sliver
completed focused evidence: 79,013 and 250,000 flat manifests and a 1,000,000-item interim
                            chunk-block manifest build within the recorded evidence; full Dart
                            analysis passes; manifest, time-navigation, unified-gallery, and window
                            focused widget suites pass serially
completed stability correction: a loaded detail-window change no longer republishes query-wide
                                row geometry; the time rail retains one fixed footprint while detail
                                state is unavailable; invalid transient viewport constraints are
                                excluded from layout snapshots; manifest failures do not enter
                                Riverpod's periodic retry cycle; the final rail value resolves to
                                the last valid item instead of an out-of-range ordinal
completed navigation correction: post-jump wheel paging is a detail-prefetch intent and cannot
                                 realign the gallery scroll position after the detail window
                                 publishes; a focused widget regression preserves both pixels and
                                 the derived rail value across that publication
completed large-wall correction: pending gallery previews use their stable final rectangles
                                 without one indeterminate animation per tile; redundant automatic
                                 sliver row semantic indexes are disabled while photo and date
                                 semantics remain; a clean retained-catalog Debug launch records no
                                 AXTree, RangeError, or stderr output after startup
completed preview separation: an identity-keyed preview store and viewer-visible-near-guard-idle
                              scheduler update one tile without replacing the asset list; obsolete
                              source and query generations cannot publish
completed navigation lifecycle: one active and one replaceable latest pending seek, query/revision
                                publication guards, request-owned loading cleanup, and visible-range
                                invalidation prevent stale targets from publishing or realigning
completed resize lifecycle: latest-only snapshot publication preserves the query-bound logical
                            anchor through one exact-sliver layout correction without authorizing
                            page loading
completed orientation correction: EXIF Orientation 1 through 8 controls durable display dimensions
                                  and preview pixels; incompatible catalogs and previews recover on
                                  an explicit complete rescan without modifying source media
completed feedback correction: scroll-triggered paging uses one top linear indicator; import
                               completion retains final counts until acknowledgment; bottom notices
                               share one surface contract; edge menus retain a viewport margin and
                               constrain shortcut rows
pending observation: freeze the current interaction baseline and measure Profile-mode frame time,
                     long-session detail growth, preview latency, memory, and actual scroll-writer
                     conflicts before changing the gallery hot path
guarded implementation: validate a bounded asset-detail page cache against nearby-return, reversal,
                        distant-jump, and native-wheel parity; enable it only when it preserves or
                        improves the measured baseline
conditional implementation: retain the flat exact manifest within budget; introduce ADR 0014 block
                            summaries plus bounded exact blocks only when measured result sizes
                            exceed the memory or frame budget
pending verification: prove resource-bounded Profile behavior, Windows Release packaging after the
                      current bridge and media changes, authorized real-library parity, and
                      read-only safety behavior before removing any rollback path
maintenance verification pending: the latest scan-lifecycle and Explorer-reveal fixes passed the
                                  focused Rust tests, release build, formatting, Clippy, Dart
                                  analysis, and whitespace checks, but the new complete daily and
                                  Flutter widget gates were interrupted by workstation resource
                                  exhaustion and must not inherit the earlier daily-gate result
accepted R2b addition: Material context menus for gallery items and source folders, exposing only
                       connected non-mutating or catalog-only actions
accepted R2b addition: toolbar More menu and bounded complete-query Select all / Deselect all
accepted R2b addition: hover/focus selection affordance with persistent selected state
completed maintenance: repository Flutter checks now share a named cross-process mutex and execute
                       widget tests one file at a time with concurrency fixed to one
completed correction: caption controls use their intended 40 px fallback hit target; sidebar drag
                      distance includes movement before the gesture threshold; readable-path,
                      import-state, viewer-position, and storage-setting fixtures match current
                      presentation contracts
completed verification: on 2026-08-10, 74 Rust tests passed with 3 explicit heavy acceptances
                        ignored; every Flutter test file passed serially; Windows Debug build and
                        native picker integration passed 2 of 2; formatting, analysis, Clippy,
                        bridge compatibility, Rust follow-up, and diff whitespace checks passed
```
