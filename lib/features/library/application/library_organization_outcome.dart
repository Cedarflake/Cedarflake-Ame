import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_organization.dart";

final libraryOrganizationOutcomeProvider =
    NotifierProvider<LibraryOrganizationOutcome, AmeNotificationEntry?>(
      LibraryOrganizationOutcome.new,
    );

class LibraryOrganizationOutcome extends Notifier<AmeNotificationEntry?> {
  @override
  AmeNotificationEntry? build() => null;

  void show(
    LibraryOrganizationReceipt receipt, {
    required String message,
    required bool canUndo,
  }) {
    state = AmeNotificationEntry(
      id: "organization-outcome-${receipt.operationId}",
      title: message,
      message: "",
      severity: AmeNotificationSeverity.success,
      occurredAt: DateTime.now(),
      isUnread: false,
      isActive: true,
      isPersistent: false,
      actionId: canUndo ? "organization.undo" : null,
      actionLabel: canUndo ? "撤销" : null,
      actionContext: canUndo ? receipt.operationId : null,
    );
  }

  AmeNotificationEntry? admitAction(AmeNotificationEntry observed) =>
      identical(state, observed) && observed.actionId != null ? state : null;

  void dismiss(String id) {
    if (state?.id == id) state = null;
  }
}
