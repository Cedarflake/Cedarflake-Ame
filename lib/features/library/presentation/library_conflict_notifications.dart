import "../../../app/messages/ame_message.dart";
import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_organization_conflicts.dart";

class LibraryConflictNotifications {
  LibraryConflictNotifications(this.notifications);
  final AmeNotificationController notifications;
  int? _count;
  BigInt? _revision;
  static const _key = "library-album-conflicts";
  static const actionId = "library.open-conflicts";

  void accept(LibraryConflictStatus status) {
    if (_revision != null && status.revision < _revision!) return;
    _revision = status.revision;
    if (_count == status.groups) return;
    _count = status.groups;
    if (status.groups == 0) {
      notifications.resolve(_key);
      return;
    }
    notifications.publish(
      AmeNotificationDraft(
        title: AmeLabel.albumConflict,
        message: AmeConflictsDeferredMessage(status.groups),
        severity: AmeNotificationSeverity.warning,
        dedupeKey: _key,
        actionId: actionId,
        actionLabel: AmeLabel.viewAction,
      ),
    );
  }

  bool admit(AmeNotificationEntry entry) =>
      entry.actionId == actionId && notifications.admitAction(entry) != null;
}
