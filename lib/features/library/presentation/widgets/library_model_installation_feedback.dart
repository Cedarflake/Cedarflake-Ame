import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_card.dart";
import "../../../../app/presentation/ame_notifications.dart";
import "../../application/library_model_installation.dart";
import "../../domain/library_model_installation.dart";
import "../library_model_installation_notifications.dart";

Widget? buildLibraryModelInstallationFeedback(
  BuildContext context,
  WidgetRef ref,
) {
  final installation = ref.watch(libraryModelInstallationProvider);
  if (installation == null) return null;
  final notifications = ref.watch(ameNotificationControllerProvider);
  final key = libraryModelInstallationNotificationKey(installation.sequence);
  final entry = notifications.history
      .where((entry) => entry.dedupeKey == key)
      .firstOrNull;
  if (entry == null || !notifications.pendingIds.contains(entry.id)) {
    return null;
  }
  final controller = ref.read(ameNotificationControllerProvider.notifier);
  if (!installation.busy) {
    return AmeNotificationSurface(
      notification: entry,
      onAction: (observed) =>
          handleLibraryModelInstallationNotification(context, ref, observed),
      onDismiss: controller.dismiss,
    );
  }
  final progress = installation.progress;
  final downloading =
      installation.stage == LibraryModelInstallationStage.downloading;
  final fraction = downloading && progress != null && progress.totalBytes > 0
      ? (progress.completedBytes / progress.totalBytes).clamp(0.0, 1.0)
      : null;
  return AmeFeedbackCard(
    key: const Key("classification-installation-progress"),
    title: entry.title,
    detail: entry.message.isEmpty ? null : entry.message,
    severity: entry.severity,
    progress: LinearProgressIndicator(value: fraction),
    actions: [
      AmeFeedbackAction(
        label: "取消",
        onPressed: installation.canCancel ? installation.cancel : null,
      ),
      AmeFeedbackAction(
        label: "隐藏",
        onPressed: () => controller.dismiss(entry.id),
      ),
    ],
  );
}
