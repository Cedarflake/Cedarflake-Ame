# Organization workflow

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Review workflow

R4 uses the existing desktop gallery and image viewer. Ordinary browsing is the default. Only the
Library workspace's upper-right `更多` menu contains `整理模式`: Library covers every configured
source, while a source or folder selection restricts the queues and counts to that scope, including
descendants under the existing folder rule. Favorites, manual albums, and smart albums expose no
R4 organization mode.
Primary-sidebar visits preserve the organizing Library's mode, scope, selection and position for
its return; Albums itself remains ordinary browsing.
Entering organization mode reveals its controls and `整理模式` beside the gallery title; the same menu offers
`退出整理`. The title status disappears on exit. Do not add a separate review
workspace, mobile layout, or `返回全部图片` action duplicating mode exit.

The existing count row contains the total, `已整理`, `待整理`, and `已忽略` counts on one line. During
organization, the upper-right operation area has two right-aligned rows. The first retains browsing
or selection actions. The second contains one mode dropdown showing the current queue's icon, name
and dropdown chevron, followed by `退出整理` and `继续整理`. The trigger's visible label omits counts;
its tooltip and accessible name retain the queue name and count. Its menu offers `待整理`, `稍后整理`, and `已忽略`,
marks the current choice and omits queue counts. Exit and continuation use a close
icon and forward arrow with their text labels. Use compact shared button spacing and horizontal
padding in this row, with four pixels between the action rows. Both rows remain available during
selection and use the header's horizontal scrolling at constrained desktop widths or enlarged text scales. Counts
also retain a single horizontally scrollable line when enlarged text exceeds their available width.

Reuse the existing popup menu's small dot selection marker and icon/text-button styles. At desktop
widths with side-by-side information and actions, align the title with the first action row and
the count with the second. Reveal and collapse the second row over 260 ms, moving both information rows,
actions and the photo area's upper boundary together while fading the secondary actions. Interpolate
the complete collapsed and expanded geometry so minimum header space cannot delay the photo area's
movement. Retain the secondary content until exit finishes and retire its input immediately.
Reversing the transition preserves its current progress; reduced motion is immediate.
Keep these controls in the header without a bottom
dock or passive continuation messages. Bottom notifications retain their existing placement.
An operation failure uses the existing top notice with retry. Ordinary browsing and organization
share the same right-side rail, including its track, dots, position line, arrows and interaction.
The selected sort determines its annotations: date sorting retains the complete year/month rail
and time navigation in either mode; name sorting shows actual letter ranges, `0–9` for numeric names
and `#` for other prefixes without changing natural-name order. `新加入优先` marks the range of groups
first admitted after entering the current organization mode. Its boundary remains fixed across
queue, sort and primary-sidebar switches; exiting and entering again captures a new boundary.
New duplicate copies retain their group's earlier admission and do not create a new range.
These range annotations follow the filtered query's actual layout, retain bounded loading, and do
not add discrete Slider stops. Unannotated rails retain the existing decorative positional dots.
A new-arrival range includes the displayed rows containing its images, even when the last row
also contains earlier images. Keep a visible range stroke at small scales, and prefer its label
when it collides with the earlier-range label; annotation positions and scroll targets stay exact.

Mode entry, queue changes and exit retain published content while the replacement query loads.
An existing empty result does not flash a loading page or an hourglass, and the controls keep their
normal colors while blocking repeated commands. Query and mode waits show the shared top loading
line after 250 milliseconds; shorter waits stay quiet. Direct scrolling and positioning keep their
immediate feedback, and an initial catalog load without a published result retains its loading state.

Hide `继续整理` and any saved-position marker before organization has begun or when no resumable
position exists in the current source/folder scope. A saved position from another folder, all-source
view, album or Favorites cannot change the current scope. Pending images alone do not create that
entry. After a position has been recorded,
continuation returns to the logical anchor under the [queue rules](../r4-organization.md#queue-and-continuation).
There is no separate `新增` selector or button. Organization inherits the gallery sort and keeps
the sort menu, adding `新加入优先` for admission-descending order. New groups follow the chosen sort
without moving the currently viewed image or overwriting the saved anchor.

During organization-mode selection and viewing, `稍后整理` appears in the rightmost More menu
alongside ignore and operation history. Successful album assignment completes organization
automatically; omit separate `标为已整理`, `保存并完成`, and `完成并下一张` controls. The organization-mode
viewer can advance after the save commits, while the ordinary-mode viewer stays on the current image. Shared progress is
updated in either mode. Organization-mode actions provide `忽略` where applicable, plus `撤销操作` and
`重做操作` following committed operation order across groups, with unavailable directions disabled.
Ordinary viewing and selection omit these R4 organization actions. The history commands replace the
current-group `恢复待整理` shortcut. The contextual `取消稍后整理` or `取消忽略` action only clears the
corresponding current state; it remains distinct from chronological undo. Organization ignore
remains separate from Duplicates' review ignore.

Album conflicts are visible in `稍后整理` with the label `相册归属冲突`, the involved albums, and an
action to choose the destination. Use the shared notification surface to point to deferred conflicts
without interrupting current browsing. Coalesce background notifications; the conflict remains on
the group after a notice disappears and clears only when resolution is committed. Do not create
another conflict tab, generic task center, or transient-only error state.

The review surface supports keyboard-first decisions, safe multi-selection, undo, deferred work,
empty and failed states, and cross-restart continuation. Counts, assignment transitions, regrouping,
and durable decisions are owned by [R4 requirements](../r4-organization.md), not widget-local state.
Closing the application preserves committed progress; reopening does not infer completion from
scroll position or later model confidence. Later classification and similarity review reuse these
foundations through their own product contracts without adding model controls to R4.
