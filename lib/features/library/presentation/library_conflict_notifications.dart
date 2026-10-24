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
        title: "相册归属冲突",
        message: "${status.groups} 组图片需要选择相册，已放入稍后整理。",
        severity: AmeNotificationSeverity.warning,
        dedupeKey: _key,
        actionId: actionId,
        actionLabel: "查看",
      ),
    );
  }

  bool admit(AmeNotificationEntry entry) =>
      entry.actionId == actionId && notifications.admitAction(entry) != null;
}
