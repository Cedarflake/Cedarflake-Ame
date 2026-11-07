import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_semantic_search.dart";
import "coordination/library_search.dart";
import "library_search_strings.dart";

class LibrarySearchNotifications {
  LibrarySearchNotifications(this.notifications);
  final AmeNotificationController notifications;
  static const retryAction = "library.retrySemanticSearch";
  static const _key = "library.semanticSearch";
  Object? _failure;
  int _generation = 0;

  void setFailure(Object? error) {
    _failure = error;
    if (error == null) {
      notifications.resolve(_key);
      return;
    }
    notifications.publish(
      AmeNotificationDraft(
        title: error is LibrarySemanticIndexFailure ? "图片分析已暂停" : "搜索暂时不可用",
        message: LibrarySearchStrings.failure(error),
        severity: AmeNotificationSeverity.error,
        detail: error.toString(),
        dedupeKey: _key,
        actionId: retryAction,
        actionContext: "${++_generation}",
        actionLabel: error is LibrarySemanticIndexFailure
            ? "重试分析"
            : LibrarySearchStrings.retry,
        isPersistent: true,
      ),
    );
  }

  Future<void> acknowledge(
    AmeNotificationEntry notification,
    LibrarySearch search,
  ) async {
    final failure = _failure;
    if (failure == null ||
        notification.dedupeKey != _key ||
        notifications.admitAction(notification) == null) {
      return;
    }
    await search.dismissFailure(failure);
  }
}
