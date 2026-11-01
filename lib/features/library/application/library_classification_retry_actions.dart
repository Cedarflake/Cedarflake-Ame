import "dart:async";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_organization.dart";
import "library_classification_retry.dart";
import "library_classification_settings.dart";
import "library_controller.dart";

class LibraryClassificationRetryState {
  const LibraryClassificationRetryState({this.busy = false, this.notice});
  final bool busy;
  final AmeNotificationEntry? notice;
}

final libraryClassificationRetryActionsProvider =
    NotifierProvider<
      LibraryClassificationRetryActions,
      LibraryClassificationRetryState
    >(LibraryClassificationRetryActions.new);

class LibraryClassificationRetryActions
    extends Notifier<LibraryClassificationRetryState> {
  bool _disposed = false;
  int _noticeSequence = 0;

  @override
  LibraryClassificationRetryState build() {
    ref.onDispose(() => _disposed = true);
    return const LibraryClassificationRetryState();
  }

  Future<bool> retry(
    LibraryOrganizationTarget target,
    BigInt dispatchRevision,
  ) async {
    if (_disposed || state.busy) return false;
    state = LibraryClassificationRetryState(busy: true, notice: state.notice);
    try {
      final outcome = await ref
          .read(libraryClassificationRetryProvider)
          .retry(target, dispatchRevision);
      if (_disposed) return true;
      _publish(switch (outcome) {
        LibraryClassificationRetryOutcome.unchanged => "图片已不需要重试",
        LibraryClassificationRetryOutcome.queued => "已安排重新分析",
        LibraryClassificationRetryOutcome.queuedWhilePaused => "已安排重试，继续分析后处理",
      }, AmeNotificationSeverity.info);
      await _refresh();
      return true;
    } on Object catch (error) {
      if (_disposed) return false;
      ref.invalidate(libraryClassificationSettingsProvider);
      final code = error is LibraryClassificationFailure
          ? error.code
          : "unexpected_error";
      _publish(switch (code) {
        "classification_dispatch_changed" ||
        "organization_selection_changed" => "图片或分析设置已变化，请重新打开菜单后重试",
        "classification_retry_unavailable" => "请先在智能相册设置中启用分析",
        _ => "未能安排重新分析，请重试",
      }, AmeNotificationSeverity.error);
      return false;
    } finally {
      if (!_disposed) {
        state = LibraryClassificationRetryState(notice: state.notice);
      }
    }
  }

  Future<void> _refresh() async {
    if (_disposed) return;
    var refreshed = false;
    try {
      refreshed = await ref
          .read(libraryControllerProvider.notifier)
          .refreshCurrentQuery();
    } on Object {
      refreshed = false;
    }
    if (_disposed) return;
    if (!refreshed) {
      _publish(
        "重试请求已处理，图库尚未刷新",
        AmeNotificationSeverity.warning,
        refresh: true,
      );
    } else if (state.notice?.actionId == "classification-analysis.refresh") {
      _publish("图库已刷新", AmeNotificationSeverity.success);
    }
  }

  bool handleNotification(AmeNotificationEntry notice) {
    if (notice.actionId != "classification-analysis.refresh") return false;
    if (_disposed || state.busy || !identical(state.notice, notice)) {
      return true;
    }
    state = LibraryClassificationRetryState(busy: true, notice: notice);
    unawaited(
      _refresh().whenComplete(() {
        if (!_disposed) {
          state = LibraryClassificationRetryState(notice: state.notice);
        }
      }),
    );
    return true;
  }

  void dismiss(String id) {
    if (state.notice?.id == id) {
      state = LibraryClassificationRetryState(busy: state.busy);
    }
  }

  void _publish(
    String message,
    AmeNotificationSeverity severity, {
    bool refresh = false,
  }) {
    state = LibraryClassificationRetryState(
      busy: state.busy,
      notice: AmeNotificationEntry(
        id: "classification-analysis-${DateTime.now().microsecondsSinceEpoch}-${++_noticeSequence}",
        title: message,
        message: "",
        severity: severity,
        occurredAt: DateTime.now(),
        isUnread: false,
        isActive: true,
        isPersistent: refresh || severity == AmeNotificationSeverity.error,
        actionId: refresh ? "classification-analysis.refresh" : null,
        actionLabel: refresh ? "重试" : null,
      ),
    );
  }
}
