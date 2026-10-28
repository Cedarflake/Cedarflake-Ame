import "dart:async";

import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../../library/application/library_archive_controller.dart";
import "../../../library/application/library_archive_state.dart";
import "../../../library/application/library_cleanup_controller.dart";
import "../../../library/application/library_cleanup_state.dart";
import "../organization_archive_messages.dart";
import "../organization_record_actions.dart";

Widget? buildOrganizationRecordFeedback(BuildContext context, WidgetRef ref) {
  final archive = ref.watch(libraryArchiveControllerProvider);
  final cleanup = ref.watch(libraryCleanupControllerProvider);
  final archiveController = ref.read(libraryArchiveControllerProvider.notifier);
  final cleanupController = ref.read(libraryCleanupControllerProvider.notifier);
  if (archive is LibraryArchiveWorking) {
    return _RecordFeedback(
      message: archiveProgressMessage(archive),
      detail: archive.cancelFailed ? "尚未停止，请再次尝试。" : null,
      working: true,
      actionLabel: "取消",
      actionKey: const Key("settings-archive-cancel"),
      onAction: archive.stopping ? null : archiveController.cancel,
    );
  }
  if (cleanup case LibraryCleanupWorking(request: LibraryCleanupApply())) {
    return _RecordFeedback(
      message: cleanup.stopping ? "正在停止" : "正在清理记录",
      detail: cleanup.cancelFailed ? "尚未停止，请再次尝试。" : null,
      working: true,
      actionLabel: "取消",
      actionKey: const Key("settings-cleanup-cancel"),
      onAction: cleanup.stopping ? null : cleanupController.cancel,
    );
  }
  if (archive is LibraryArchiveFailed) {
    return _RecordFeedback(
      message: archiveFailureMessage(archive.failure),
      actionLabel: archive.failure.requiresInspection ? "重新检查" : "重试",
      actionKey: const Key("settings-archive-retry"),
      onAction: () => unawaited(retryOrganizationArchive(context)),
      onDismiss: archiveController.dismiss,
    );
  }
  if (cleanup case LibraryCleanupFailed(request: LibraryCleanupApply())) {
    return _RecordFeedback(
      message: cleanup.failure.requiresInspection
          ? "记录已发生变化，请重新检查后确认。"
          : "未能确认清理结果，请重试。",
      actionLabel: cleanup.failure.requiresInspection ? "重新检查" : "重试",
      actionKey: const Key("settings-cleanup-retry"),
      onAction: () => unawaited(
        cleanup.failure.requiresInspection
            ? inspectOrganizationCleanup(context)
            : cleanupController.retry(),
      ),
      onDismiss: cleanupController.dismiss,
    );
  }
  return null;
}

class _RecordFeedback extends StatelessWidget {
  const _RecordFeedback({
    required this.message,
    required this.actionLabel,
    required this.actionKey,
    required this.onAction,
    this.detail,
    this.working = false,
    this.onDismiss,
  });

  final String message;
  final String? detail;
  final bool working;
  final String actionLabel;
  final Key actionKey;
  final VoidCallback? onAction;
  final VoidCallback? onDismiss;

  @override
  Widget build(BuildContext context) => Semantics(
    liveRegion: true,
    child: AmeFeedbackCard(
      key: const Key("organization-record-feedback"),
      title: message,
      message: detail,
      severity: working
          ? AmeNotificationSeverity.info
          : AmeNotificationSeverity.error,
      actions: [
        AmeFeedbackAction(
          key: actionKey,
          onPressed: onAction,
          label: actionLabel,
        ),
        if (onDismiss != null)
          AmeFeedbackAction(onPressed: onDismiss, label: "关闭"),
      ],
      progress: working ? const LinearProgressIndicator() : null,
    ),
  );
}
