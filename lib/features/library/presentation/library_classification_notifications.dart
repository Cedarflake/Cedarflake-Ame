import "../../../app/notifications/ame_notification_controller.dart";

const libraryClassificationFailureTitle = "智能相册分析已中断";
const libraryClassificationFailureMessage = "已有分类结果仍可浏览。";

class LibraryClassificationNotifications {
  LibraryClassificationNotifications(this._notifications);

  final AmeNotificationController _notifications;
  static const _key = "classification-lifecycle";

  void accept(Object? error) {
    if (error == null) {
      _notifications.resolve(_key);
      return;
    }
    _notifications.publish(
      const AmeNotificationDraft(
        title: libraryClassificationFailureTitle,
        message: libraryClassificationFailureMessage,
        severity: AmeNotificationSeverity.error,
        dedupeKey: _key,
        isPersistent: true,
      ),
    );
  }
}
