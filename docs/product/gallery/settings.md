# Settings page

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

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
  默认加入的相册           未设置 / 用户创建的相册

智能相册
  GPU 加速                开（默认） / 关

整理记录（R4 接通后）
  导出整理记录             导出
  恢复整理记录             选择文件
  清理未关联记录           查看并清理

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
decodes. `加入相册前询问` initially defaults to on, and `默认加入的相册` initially displays `未设置`.
Favorites are not a possible default. A valid configured album can be preselected for unassigned
groups; existing assignments and conflicts remain visible. Turning the prompt off requires a valid
default. If that album is later deleted, reopen the picker on the next operation. These rows remain
absent until R4 connects the complete album workflow.

The R4 record controls use the [persistence and recovery contract](../r4-organization.md#persistence-removal-and-recovery).
Export and restore describe that they carry favorites, album decisions, and progress rather than
original images. Export, restore and cleanup progress, cancellation, retry and completion use the
shared bottom feedback area. Do not insert status rows, progress bars or additional actions beneath
the settings entries; these entries keep their position throughout the operation.
Inspection results belong in their confirmation dialog. A completed backup inspection remains
reachable through the existing restore row rather than a separate status/action row; cleanup
inspection progress and retry stay inside its dialog.
`清理未关联记录` shows the number and age of records whose images cannot be reliably
associated, explains which saved decisions will be removed, and requires confirmation of the
chosen cleanup. Matching again before confirmation preserves the recovered record. This is not
thumbnail cleanup or an automatic age-based purge. Keep the controls absent until their workflows
are functional; their inclusion here does not claim current availability.

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

`GPU 加速` applies to automatic smart-album image analysis. It defaults to on, prefers a supported
local GPU and persists across restarts; turning it off selects CPU. Device switching retires the
previous analysis process before replacing it, while the row retains its position and disables the
switch during the change. The description names the selected device when available and does not
claim that an idle worker is currently computing. Existing categories and legacy classification
decisions remain stored; no manual classification controls are restored.
