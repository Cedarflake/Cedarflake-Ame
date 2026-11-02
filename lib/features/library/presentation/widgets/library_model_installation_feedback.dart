import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../../../app/presentation/ame_notifications.dart";
import "../../application/library_model_installation.dart";
import "library_model_setup.dart";

Widget? buildLibraryModelInstallationFeedback(
  BuildContext context,
  WidgetRef ref,
) {
  final installation = ref.watch(libraryModelInstallationProvider);
  if (installation == null ||
      installation.stage == LibraryModelInstallationStage.idle) {
    return null;
  }
  if (installation.busy) {
    final progress = installation.progress;
    final downloading =
        installation.stage == LibraryModelInstallationStage.downloading;
    final fraction = downloading && progress != null && progress.totalBytes > 0
        ? (progress.completedBytes / progress.totalBytes).clamp(0.0, 1.0)
        : null;
    return Semantics(
      liveRegion: true,
      child: AmeFeedbackCard(
        key: const Key("classification-installation-progress"),
        title: switch (installation.stage) {
          LibraryModelInstallationStage.downloading => "正在下载智能相册模型",
          LibraryModelInstallationStage.stopping => "正在停止",
          LibraryModelInstallationStage.enabling => "正在启用智能相册",
          _ => "正在准备智能相册模型",
        },
        detail: downloading && progress != null
            ? "${(progress.completedBytes / 1000000).toStringAsFixed(1)} / "
                  "${(progress.totalBytes / 1000000).toStringAsFixed(1)} MB"
            : null,
        severity: AmeNotificationSeverity.info,
        progress: LinearProgressIndicator(value: fraction),
        actions: [
          AmeFeedbackAction(
            label: "取消",
            onPressed: installation.canCancel ? installation.cancel : null,
          ),
        ],
      ),
    );
  }
  if (installation.stage == LibraryModelInstallationStage.failed) {
    return AmeFeedbackCard(
      key: const Key("classification-installation-failure"),
      title: "智能相册未能启用",
      message: "已下载的完整模型文件会保留，可以重试。",
      severity: AmeNotificationSeverity.error,
      actions: [
        AmeFeedbackAction(
          label: "重试",
          onPressed: () => showLibraryModelSetup(context, ref),
        ),
        AmeFeedbackAction(label: "关闭", onPressed: installation.dismiss),
      ],
    );
  }
  final enabled = installation.stage == LibraryModelInstallationStage.enabled;
  return AmeNotificationSurface(
    notification: AmeNotificationEntry(
      id: "model-installation-${installation.sequence}",
      title: enabled ? "已启用智能相册" : "已取消启用智能相册",
      message: "",
      severity: enabled
          ? AmeNotificationSeverity.success
          : AmeNotificationSeverity.info,
      occurredAt: installation.finishedAt!,
      isUnread: false,
      isActive: true,
      isPersistent: false,
    ),
    onAction: (_) {},
    onDismiss: (_) => installation.dismiss(),
  );
}
