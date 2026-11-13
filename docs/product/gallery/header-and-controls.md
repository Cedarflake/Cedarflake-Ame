# Gallery header, sorting, filters and layout

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Unified gallery header

Normal browsing state:

```text
图库 · 结果数量                          选择 | 排序 | 筛选 | 布局 | 更多
```

Selection state:

```text
已选择 N 个项目                         收藏 | 相册 | 取消 | 更多
```

The normal toolbar is replaced rather than nested when selection begins. Selection is keyed by
stable Ame asset identity and survives lazy item disposal and scrolling.
Desktop width changes retain the title/count column and the right action column. Do not move the
entire action area below the title at a fixed width breakpoint; use the existing horizontal action
scrolling when space is constrained. Organization mode retains its explicit animated second action row.

`加入相册` becomes visible only after R4 connects durable album membership. It is an action in this
upper-right selection-specific area, while entries under the sidebar's `相册` section are navigation
scopes. The action and entries share durable membership data but never share interaction
responsibility.

When the album prompt is enabled, the album action opens a Material bottom sheet with one
destination before group assignments are changed. The selection toolbar keeps Favorite first and
More last; ignore and defer appear inside More only in organization mode.
R4's organization actions in More, including chronological undo and redo, are limited to organization
mode. Ordinary selection and viewing retain their applicable browsing and later classification actions.
Preparation lasting over 500 milliseconds uses the existing bottom task feedback, outside the
toolbar. Short preparation shows no indicator; completion removes it immediately.
Tray opening and dismissal retain button bounds. State-dependent labels use their current content
width and animate size changes rather than reserving space for every possible label.

```text
选择相册

[ ] 用户相册 A
[ ] 用户相册 B

[忽略此图] [稍后整理] [新建相册]                         [加入相册]
```

- only user albums are destinations; Favorites have a separate action and never appear here;
- use checkboxes that allow the selected destination to be unchecked. At most one destination is
  selected; checking a different album replaces the previous choice for the complete image group;
- the checkbox conveys the selection; omit a separate `已选择` summary and routine whole-group
  explanation above the list. Retain actionable conflict feedback;
- `忽略此图` (`忽略所选` for a batch) and `稍后整理` appear only in organization mode.
  Ordinary Library/folder browsing, albums and Favorites omit both actions from the tray and menus;
- replace an already-applied defer or ignore action with `取消稍后整理` or `取消忽略`, returning that group
  to pending without changing Favorites or source files. Apply the same state-based choice to gallery
  selection, the detailed viewer and its album tray. Unresolved album conflicts cannot be removed from
  deferred work before an album choice resolves them;
- pair every tray action with a leading icon matching its operation, including create, remove,
  add and move; retain framework button labels, focus and disabled state;
- reopening reflects the current confirmed assignment. A mixed batch or unresolved conflict must
  not appear to have one shared assignment; show the existing album information and let the user
  choose one destination for the selected groups;
- the primary action is `移至其他相册` when all selected groups are assigned, `加入相册` when none
  are assigned, and `加入或移至相册` for a mixed batch. `移出相册` is available when there are confirmed
  assignments and no unresolved conflict. Selecting the shared current album cannot confirm a
  redundant move. Removal leaves already-unassigned groups unchanged;
- the sheet has no Cancel button. Escape or the backdrop dismisses without changing decisions;
  an admitted write retains the existing commit/cancellation boundary. The sheet offers ignore and
  defer through the same whole-group commands in organization mode, with plural wording for a batch.
  More offers ignore only in organization mode. These actions advance only after the committed receipt;
- a new unassigned selection may use a valid configured default. Initially no default is selected;
  an empty album list offers `新建相册`, and cancellation leaves assignments and progress unchanged;
- confirmation commits album and organization-state changes together and never changes source
  files. Favorites remain unchanged, and successful assignment counts as organized;
- when prompting is disabled and a valid default exists, the same operation targets that album and
  reports the result with undo. Missing/deleted defaults and unresolved assignment conflicts open
  the picker instead of silently guessing a destination;
- the production control remains absent until membership persistence, settings, errors, undo, and
  the complete user path are connected.

Selection quietly reads the current groups to show `收藏` when all are unfavorited and `取消收藏`
when all are favorited. These uniform selections act directly on activation. Only a mixed selection
opens a dropdown offering `加入收藏夹` and `取消收藏`; its trigger retains `收藏` with a dropdown arrow.
The summary read does not disable album, file or cancel controls. Activation reuses the prepared
guarded target; no decision changes before the user acts.
A cancelled or changed selection cannot apply an earlier action to new images. A chosen
mixed-selection command retains that intent through commit.
Unfavorite uses the heart-minus icon. Favorite changes leave album
assignments and organization progress intact. Album removal and movement retain distinct labels
and results.

In normal browsing, a gallery tile reveals its upper-right selection checkbox on pointer hover or
keyboard focus. Selection mode shows checkboxes on every visible tile, and selected tiles always
retain the checkbox, check mark, and Material primary-color outline. Activating the tile body opens
the viewer; activating the checkbox changes selection without opening. Touch and assistive
technology receive an always-available semantic selection action and do not depend on hover.

File-operation controls appear only when the
[R8 workflow](../organization-workflows.md#r8---file-organization-and-recovery) is connected end to
end and its safety gates are accepted. Do not show executable delete, move, or copy placeholders,
or introduce an organization-preview control.

Every gallery item supports a Material context menu opened by secondary click or the platform
keyboard context-menu gesture. It targets the item under the pointer without losing an existing
multi-selection. The connected actions are:

- `打开`;
- `加入收藏夹` or `取消收藏`, according to the current group's Favorite state;
- `加入相册` for an unassigned group, or `移至其他相册` and `移出相册` for an assigned group;
- `查看信息`;
- `复制路径`;
- `在文件资源管理器中打开`.

Album conflicts expose `选择相册` through the existing resolution tray. Assignment and movement use
the shared bottom album tray; direct removal and Favorite changes use existing commands and feedback.
The menu freezes its opening image identity and callbacks; a replaced or retired target cannot receive
a late action. Reading current organization state is bounded to that one group. Read failure leaves
basic file-inspection actions available and reports reopening the menu as the recovery action.
Album rows expose the same rename/delete menu through More, secondary click and keyboard gestures.
An album replaced while its menu is open cannot receive the old action.
Edit, print, share, move, copy-file, rename, and delete actions from Microsoft Photos
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
Popup widths fit the longest label and its icon, selection-marker and shortcut slots; short choices
do not inherit a wider fixed minimum.
Menu icons use the shared neutral icon color, independent of the anchor's selected color; disabled
entries retain the framework's reduced opacity.
Exclusive choices in Sort, Layout, Filter and the organization queue menu use the same small filled
dot. Disabled filter choices retain their selected marker and disabled styling.

## Sort behavior

The compact sort action opens two independent choice groups:

```text
拍摄日期 | 创建日期 | 修改日期 | 名字
升序 | 降序
```

The initial default is `拍摄日期 + 降序`. A capture-time sort keeps missing evidence in an explicit
unknown section rather than substituting another timestamp. Date headers use the selected date
source. Name sorting uses letter annotations on the shared scroll rail and omits date headers.

R2 UI fixtures show this confirmed menu. Production options become visible only with a bounded,
revision-safe backend query for the corresponding key and direction; sorting a partial Flutter
window locally is forbidden.

## Filter and exact-duplicate behavior

The gallery Filter menu contains source choices, image scope and a separate duplicate-folding toggle:

```text
显示子目录
隐藏子目录
────────────────
全部图片
仅显示重复
仅显示相似
────────────────
合并重复
────────────────
更多筛选
```

The source group controls descendants of the current folder. The duplicate toggle controls whether
all physical instances or one representative per confirmed exact group appears. Before trustworthy
exact analysis and incremental maintenance are connected, every image remains visible. Once folding
is connected, it becomes the initial default while preserving an explicit saved choice.

Library, Favorites, manual albums and smart albums share this menu. Outside a folder, the source
choices remain visible and disabled. Manual albums, smart albums and organization queues require
merged groups: `合并重复` remains visible, checked and disabled. Library and Favorites can toggle it.
The filter button retains the same neutral color as the adjacent toolbar buttons. The independent
folding toggle uses a checkmark when enabled and no marker when off; it is not a radio choice.

The additional group-filter choices and scoped member tray follow the
[group browsing contract](workbench-and-navigation.md). While either group filter is active,
`合并重复` is disabled and unchecked; only the active group-filter choice has a selection marker.
Choosing `全部图片` clears the group filter and restores the retained display choice for the collection.

The descendant and duplicate choices apply directly. `更多筛选` opens the standard Material bottom
sheet centered at the bottom of the window, using the same opening behavior and theme width as the
album picker. Content scrolls within the bounded sheet; its title and action row remain visible.
Formats use multi-select `FilterChip` controls; selecting JPG and PNG includes either format.
Each section title sits above its controls. Content-sized format chips occupy one horizontal row
with compact gaps. Arrange format, date, dimensions, file size and camera sections vertically.
Group the date-type dropdown with two calendar fields. Width, height and file
size expose minimum/maximum numeric fields directly, without presets; retain the size-unit dropdown.
Camera has its own section. Chips, outlined fields and buttons use the application theme and standard
component shapes, with no separate fills, rounding or fixed-width chip cells. Reuse shared scrolling,
with hover/pressed feedback following each control's shape. The custom form withdrawn on
2026-11-02 is superseded by this framework composition.
Draft edits leave the gallery unchanged until `应用`; invalid dates, bounds or text stay editable
with field errors. `取消`, Close and Escape preserve the applied conditions. `清除筛选` clears metadata
conditions while retaining the folder/collection, search and duplicate-display choices.

Place Clear at the tray's bottom left and Cancel/Apply at its bottom right. Applied conditions
give the toolbar filter button an active state; its tooltip summarizes them and its menu places
`清除筛选` immediately after `更多筛选`, followed by individually removable conditions. Keep the gallery
title area free of condition chips. Removing one
condition retains the others. An editor or removal menu opened for a retired query cannot publish
into a different query.
Organization continuation uses the current query's conditions, preserving compatibility with saved
payloads without erasing organization progress. [ADR 0033](../../architecture/0033-structured-gallery-filters.md)
owns field meaning and shared query behavior.

Consecutive metadata-filter and literal-search changes retain the original visible image and its fractional viewport
position. A missing image, empty result or automatic clamp to an edge does not replace that intent;
when conditions include the image again, restore it within the available scroll range. Actual
scrolling, timeline navigation, source/folder selection or a changed ordering establishes a new
browsing context. Passive catalog refresh preserves the currently displayed result and leaves the
latent filter intent available for the next explicit filter change.
Starting a new semantic description or reference establishes a new relevance order and opens its
first result. Refining filters or explicitly updating the same semantic search preserves position;
entering and returning from the shared viewer retains the current result.
Semantic search displays candidates admitted by its relevance rule, ordered by relevance. The result
limit never pads the list with unrelated candidates. An empty search explains that no matches were
found and retains the current scope/filter controls for refinement.

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
- `小`, `中等`, and `大` target 128, 192, and 256 logical pixels in both layouts. Equal-height rows
  keep that height; square tiles divide the available width into the corresponding column count.
  Lazy loading and decoding remain bounded.

Equal-height packing compares ending a row before or after the next image. It may reduce wider
cells to keep neighboring images together, while retaining the 128-pixel action width floor and
item order. Completed rows fill the wall even when they contain a single image; a sparse final row
keeps its natural width.

Preview demand includes the scale needed to fill the card before cropping and the display's pixel
density. Narrow and panoramic images use the existing size buckets up to 1024 pixels; generation
and decoded-image sizing must both retain the requested detail without changing the card's crop.

Both choices apply to the same unified gallery canvas and preserve stable item identity and scroll
position when possible. Consecutive density or shape choices retain the same image and fractional
viewport point until browsing moves on, so a size round trip does not accumulate a row of drift.
Scrolling or navigation establishes the next position; an earlier layout choice must not pull the
viewport back after that input. A layout choice made at the top or bottom retains that edge;
content that initially fits in one viewport stays at the top. Existing bottom intent survives
temporarily fitting in one viewport, so expanding the cards again returns to the bottom.
Other anchors clamp to the available scroll extent.
