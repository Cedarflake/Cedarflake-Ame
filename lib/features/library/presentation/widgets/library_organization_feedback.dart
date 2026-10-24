import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:material_symbols_icons/symbols.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_notification_lifetime.dart";
import "../../application/library_organization_actions.dart";
import "../../application/library_organization_outcome.dart";
import "library_viewer_notice.dart";

class LibraryOrganizationFeedback extends ConsumerWidget {
  const LibraryOrganizationFeedback({required this.notification, super.key});
  final AmeNotificationEntry notification;

  @override
  Widget build(BuildContext context, WidgetRef ref) {
    final saving = ref.watch(
      libraryOrganizationActionsProvider.select((state) => state.saving),
    );
    return AmeNotificationLifetime(
      notification: notification,
      onDismiss: (id) {
        ref.read(libraryOrganizationOutcomeProvider.notifier).dismiss(id);
        ref.read(ameNotificationControllerProvider.notifier).dismiss(id);
      },
      child: LibraryViewerNotice(
        icon: switch (notification.severity) {
          AmeNotificationSeverity.success => Symbols.check_circle_rounded,
          AmeNotificationSeverity.error => Symbols.error_rounded,
          AmeNotificationSeverity.warning => Symbols.warning_rounded,
          AmeNotificationSeverity.info => Symbols.info_rounded,
        },
        message: notification.message.isEmpty
            ? notification.title
            : "${notification.title}：${notification.message}",
        actionLabel: notification.actionLabel,
        onAction: saving
            ? null
            : () {
                ref
                    .read(libraryOrganizationActionsProvider.notifier)
                    .handleNotification(notification);
              },
        actionKey: const Key("viewer-organization-action"),
      ),
    );
  }
}
