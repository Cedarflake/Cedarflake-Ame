import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../../app/notifications/ame_notification_controller.dart";
import "../../../../app/presentation/ame_localizations.dart";
import "../../../../app/presentation/ame_message_text.dart";
import "../../../../app/presentation/ame_notification_lifetime.dart";
import "../../../../app/presentation/ame_feedback_style.dart";
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
        icon: ameNotificationIcon(notification.severity),
        message: notification.message.resolve(context.l10n).isEmpty
            ? notification.title.resolve(context.l10n)
            : context.l10n.labelAndValue(
                notification.title.resolve(context.l10n),
                notification.message.resolve(context.l10n),
              ),
        actionLabel: notification.actionLabel?.resolve(context.l10n),
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
