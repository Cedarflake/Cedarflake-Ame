# Task, notification and source-status feedback

Status: binding scoped requirements under [gallery-ui](../gallery-ui.md).

Read this file when the task touches its listed workflow or owners. The parent document retains
shared principles; the roadmap owns current delivery order and acceptance status.

## Temporary task and notification feedback

An active import uses a bottom floating progress surface similar to the reference workflow:

```text
正在添加目录“Picture”…
已检查 12,340 个文件 · 已找到 10,826 张图片
进度条                                                       取消
```

Completion changes the same surface to its completed result, retains the final checked, imported,
and issue counts, and removes the cancel action. Eligible successful tasks dismiss after five
seconds; hover, keyboard focus and accessible navigation retain the task for interaction. Paused,
cancelled, failed, limited or unpublished results retain their existing recovery and dismissal rules.
Cancellation and failure use action-specific messages. The completed result is dismissible task feedback, not a
permanent validation card, status bar, task center, or generic task entry.

Bottom notifications and import feedback share one Material surface contract for color, width,
corner radius, elevation, and placement. The same event must not produce competing gray and white
notification surfaces.

### Loading visibility and geometry

`AmeProgressVisibility` owns the visual delay for Material loading indicators. A request completed
within 250 ms shows its result without displaying progress; longer work fades the indicator in.
The existing 500-ms selection-preparation threshold remains. Admission, cancellation, completion
and recovery actions never wait for this visual delay. Active phase/progress changes do not restart
the interval, and a short interruption reverses an existing fade instead of flashing a new indicator.

Keep the indicator's slot stable in galleries, search controls, settings rows and history lists.
Filmstrip paging retains its existing edge placeholder. Task-card progress slots collapse smoothly
when the terminal result replaces progress. Hidden indicators stop ticking and expose no stale
loading semantics; reduced motion skips the fade while retaining the quiet interval. Storage-usage
gauges describe measured capacity and are not delayed as loading feedback.

### Feedback templates

New feedback must use the template for its surface. Feature code supplies wording, severity,
progress and connected callbacks; it does not create another Material shell, set visual overrides,
add a separate status row, or introduce an independent dismissal timer.

| Surface | Implementation entry | Permitted variation |
| --- | --- | --- |
| Bottom notification and operation result | [`AmeNotificationSurface`](../../../lib/app/presentation/ame_notifications.dart) through the existing bottom host | Title, optional message/detail, severity, connected action and acknowledgement |
| Bottom import, recovery and cleanup progress | [`AmeFeedbackCard`](../../../lib/app/presentation/ame_feedback_card.dart) | The same text/action fields plus the existing progress indicator; `.tasks` admits the bounded multi-root list |
| Short message using ScaffoldMessenger, including startup failure recovery | [`showAmeTransientFeedback`](../../../lib/app/presentation/ame_transient_feedback.dart) | Message and severity; the helper owns the same card, acknowledgement and transient duration |
| Image viewport's top notice | [`LibraryViewerNotice`](../../../lib/features/library/presentation/widgets/library_viewer_notice.dart) | Icon, message and one optional action; no additional close button |

[`ame_feedback_style.dart`](../../../lib/app/presentation/ame_feedback_style.dart) owns the common
surface, 680-pixel width bound, 16-pixel radius, elevation 3, 20-pixel icon and five-second transient
duration. Both light and dark themes use `surfaceContainerHigh`. `AmeFeedbackAction` owns framework
text-button construction, including disabled callbacks and action keys. Callers cannot supply custom
button styles through the template.

Bottom cards retain padding `(20, 14, 12, 12)`, a 12-pixel icon gap and a 10-pixel progress gap.
The icon, text block and action labels share a vertical center. Single-line results align on one
line; additional message/detail lines expand the text block without top-aligning it against buttons.
Title, message and detail use `titleSmall`, `bodyMedium` and secondary `bodySmall`. A multi-root list
is capped at 360 pixels and scrolls through the existing list owner. Bottom feedback stays centered
24 pixels above the window edge in ordinary and organization modes; the upper organization toolbar
does not add a bottom offset.

File-operation completion places its status and nonzero outcome counts together in the title;
an available result report gets one separate message. Omit a generic partial-result heading and
zero completed, skipped, failed or stopped counts. If nothing ran, say so directly. Completed
deletion retains its single affected-file summary. Progress may still show zero before work completes.

Top notices retain the compact viewport layout: 16-pixel top/side inset, 48-pixel minimum height,
8-pixel icon gap, `bodyMedium`, and the existing optional retry/undo action. Text wraps within the
width bound without moving the image. Notices without an action use symmetric 14-pixel horizontal
content padding; notices with an action retain the 6-pixel trailing inset beside the text button.
Source and original-image recovery keep their current priority.
The shared style does not merge notification history, task cancellation, source recovery or receipt
ownership. Persistent failures do not acquire a transient timeout, and Favorite successes still
leave notification history and unread state unchanged.

Feature code must not construct `SnackBar` directly. The template regression checks this boundary
and verifies the two themes, shared geometry, text wrapping and connected actions. The
[verification map](../../acceptance/quality/ui-verification.md#ui-workflow-verification) owns the required checks.

### Source status and notification ownership

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
priority over progress. `已同步` requires a synchronized source snapshot and published catalog changes.
Completed automatic metadata fallback also displays `已同步`, while journal continuity remains
LiveOnly internally. Its source row stays visible and does not publish the manual-update capability
notice; that notice remains applicable to roots without an automatic offline recovery path.
Source rows describe import, explicit folder updates and directory synchronization. Once an import
has published its catalog and the source snapshot is synchronized, the row immediately displays
`已同步`, independently of notification dismissal and duplicate analysis. Automatic analysis waiting,
running, pausing, failure and result publication belong to the Duplicates workspace and do not
extend or restart a source's update label. Adding or updating one source leaves settled peers
unchanged. Background verification after lost offline history retains `正在核对` until its own
guarded changes are published; this recovery path is distinct from first import. Analysis results
still refresh the gallery through the existing anchored query lifecycle, including its failure and
reload workflow.
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
