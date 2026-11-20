import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_message_text.dart";
import "../../../../app/presentation/ame_progress.dart";
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
    title: entry.title.resolve(context.l10n),
    detail: entry.message.resolve(context.l10n).isEmpty
        ? null
        : entry.message.resolve(context.l10n),
    severity: entry.severity,
    progress: AmeLinearProgress(value: fraction),
    actions: [
      AmeFeedbackAction(
        label: context.l10n.cancel,
        onPressed: installation.canCancel ? installation.cancel : null,
      ),
      AmeFeedbackAction(
        label: context.l10n.hide,
        onPressed: () => controller.dismiss(entry.id),
      ),
    ],
  );
}
