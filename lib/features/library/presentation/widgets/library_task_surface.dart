import "package:flutter/material.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../domain/library_state.dart";
import "../library_primary_task_feedback.dart";
import "library_task_live_region.dart";

/// 窗口底部主任务卡；标题、说明和恢复按钮文字来自 LibraryPrimaryTaskFeedback。
/// 暂停、继续、取消、关闭的固定按钮文字位于下方对应状态分支。
class LibraryTaskSurface extends StatelessWidget {
  const LibraryTaskSurface({
    required this.state,
    required this.onPause,
    required this.onCancel,
    required this.onResume,
    required this.onRetry,
    required this.onDismiss,
    super.key,
  });

  final LibraryState state;
  final VoidCallback onPause;
  final VoidCallback onCancel;
  final Future<void> Function() onResume;
  final Future<void> Function() onRetry;
  final VoidCallback onDismiss;

  @override
  Widget build(BuildContext context) {
    final feedback = LibraryPrimaryTaskFeedback(state);
    final title = feedback.title;
    final detail = feedback.detail;
    final surface = AmeFeedbackCard(
      key: const Key("library-task-surface"),
      title: title,
      detail: detail,
      severity: AmeNotificationSeverity.info,
      actions: [
        if (state.status == LibraryStatus.scanning ||
            state.status == LibraryStatus.pausing) ...[
          if (state.status == LibraryStatus.scanning &&
              !feedback.isLibraryUpdate)
            AmeFeedbackAction(
              key: const Key("library-pause-button"),
              onPressed: onPause,
              label: "暂停",
            ),
          AmeFeedbackAction(
            key: const Key("library-cancel-button"),
            onPressed: onCancel,
            label: "取消",
          ),
        ] else if (state.status == LibraryStatus.paused) ...[
          AmeFeedbackAction(
            key: const Key("library-resume-button"),
            onPressed: onResume,
            label: "继续",
          ),
          AmeFeedbackAction(
            key: const Key("library-cancel-button"),
            onPressed: onCancel,
            label: "取消",
          ),
        ] else if (state.status == LibraryStatus.discarding) ...[
          const AmeFeedbackAction(onPressed: null, label: "继续"),
          const AmeFeedbackAction(onPressed: null, label: "取消"),
        ] else if (state.status == LibraryStatus.failed ||
            state.status == LibraryStatus.cancelled) ...[
          AmeFeedbackAction(
            key: const Key("library-retry-button"),
            onPressed: onRetry,
            label: feedback.retryLabel,
          ),
          if (!state.isCommittedRemovalReloadPending &&
              !state.primaryScanSnapshot.hasUnresolvedPublication)
            AmeFeedbackAction(
              key: const Key("library-task-dismiss-button"),
              onPressed: onDismiss,
              label: "关闭",
            ),
        ] else if (state.status == LibraryStatus.stale)
          AmeFeedbackAction(
            key: const Key("library-retry-button"),
            onPressed: onRetry,
            label: feedback.retryLabel,
          )
        else if (state.status == LibraryStatus.completed)
          AmeFeedbackAction(
            key: const Key("library-task-dismiss-button"),
            onPressed: onDismiss,
            label: "关闭",
          ),
      ],
      progress: state.isTaskProcessing
          ? LinearProgressIndicator(
              value:
                  state.status == LibraryStatus.scanning &&
                      state.scanPhase == LibraryScanPhase.finalizing &&
                      state.validationAssetCount > 0
                  ? state.validatedAssetCount / state.validationAssetCount
                  : null,
            )
          : null,
    );
    if (!feedback.isRootRemoval) {
      return surface;
    }
    return LibraryTaskLiveRegion(
      message: detail == null ? title : "$title。$detail",
      announcementScope:
          "${state.status.name}:${state.isRemovalCommitted}:${state.removingRootId}",
      child: surface,
    );
  }
}
