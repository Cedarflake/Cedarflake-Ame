import "../../../app/messages/ame_message.dart";
import "../../../app/notifications/ame_notification_controller.dart";

const libraryClassificationFailureTitle = AmeLabel.smartAnalysisInterrupted;
const libraryClassificationFailureMessage = AmeLabel.smartResultsAvailable;

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
