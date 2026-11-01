import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "library_classification_retry_actions.dart";
import "library_organization_actions.dart";
import "library_organization_outcome.dart";

AmeNotificationEntry? latestLibraryOperationNotice(
  AmeNotificationEntry? organization,
  AmeNotificationEntry? classification,
) {
  if (organization == null) return classification;
  if (classification == null) return organization;
  return classification.occurredAt.isAfter(organization.occurredAt)
      ? classification
      : organization;
}

final libraryOperationOutcomeProvider = Provider<AmeNotificationEntry?>(
  (ref) => latestLibraryOperationNotice(
    ref.watch(libraryOrganizationOutcomeProvider),
    ref.watch(
      libraryClassificationRetryActionsProvider.select((state) => state.notice),
    ),
  ),
);

final libraryViewerOperationNoticeProvider = Provider<AmeNotificationEntry?>(
  (ref) => latestLibraryOperationNotice(
    ref.watch(libraryOrganizationNoticeProvider),
    ref.watch(libraryClassificationNoticeProvider),
  ),
);

final libraryClassificationNoticeProvider = Provider<AmeNotificationEntry?>((
  ref,
) {
  final notifications = ref.watch(ameNotificationControllerProvider);
  final pending = notifications.pendingIds.toSet();
  final failure = notifications.history
      .where(
        (entry) =>
            pending.contains(entry.id) &&
            entry.dedupeKey?.startsWith("classification-") == true,
      )
      .firstOrNull;
  return failure ??
      ref.watch(
        libraryClassificationRetryActionsProvider.select(
          (state) => state.notice,
        ),
      );
});
