import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_feedback_style.dart";
import "../../../../app/presentation/ame_notifications.dart";
import "../../application/library_controller.dart";
import "../../application/library_operation_notice.dart";
import "../../application/library_organization_outcome.dart";
import "../../application/library_update_controller.dart";
import "../library_task_surface_selection.dart";
import "../library_workbench_destinations.dart";
import "library_model_installation_feedback.dart";
import "library_task_feedback.dart";

class LibraryBottomFeedback extends ConsumerWidget {
  const LibraryBottomFeedback({
    required this.onRetryPrimary,
    required this.onNotificationAction,
    this.onNotificationAcknowledged,
    super.key,
  });

  final Future<void> Function() onRetryPrimary;
  final ValueChanged<AmeNotificationEntry> onNotificationAction;
  final ValueChanged<AmeNotificationEntry>? onNotificationAcknowledged;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final state = ref.watch(libraryControllerProvider);
    final updates = ref.watch(libraryUpdateControllerProvider);
    final tasks = LibraryTaskSurfaceSelection(state, updates);
    final notifications = ref.watch(ameNotificationControllerProvider);
    final outcome = ref.watch(libraryOperationOutcomeProvider);
    final notification = notifications.current ?? outcome;
    Widget? feedback = ref.read(libraryRecordFeedbackProvider)(context, ref);
    if (feedback == null && (tasks.showPrimary || tasks.showUpdates)) {
      feedback = LibraryTaskFeedback(onRetryPrimary: onRetryPrimary);
    }
    feedback ??= buildLibraryModelInstallationFeedback(context, ref);
    if (feedback == null &&
        !tasks.hasPrimary &&
        !updates.hasFeedback &&
        notification != null) {
      feedback = AmeNotificationSurface(
        key: ValueKey(notification.id),
        notification: notification,
        onAction: onNotificationAction,
        onDismiss: (id) {
          onNotificationAcknowledged?.call(notification);
          ref.read(ameNotificationControllerProvider.notifier).dismiss(id);
          ref.read(libraryOrganizationOutcomeProvider.notifier).dismiss(id);
        },
      );
    }
    if (feedback == null) return const SizedBox.shrink();
    return Align(
      alignment: Alignment.bottomCenter,
      child: Padding(
        padding: const EdgeInsets.only(bottom: ameNotificationBottomInset),
        child: feedback,
      ),
    );
  }
}
