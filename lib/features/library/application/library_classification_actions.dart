import "dart:async";
import "dart:math";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../domain/library_classification_review.dart";
import "../domain/library_organization.dart";
import "library_classification_review.dart";
import "library_classification_selection.dart";
import "library_controller.dart";

class LibraryClassificationActivity {
  const LibraryClassificationActivity({
    this.saving = false,
    this.sequence = 0,
    this.notice,
  });
  final bool saving;
  final int sequence;
  final AmeNotificationEntry? notice;
}

final libraryClassificationActionsProvider =
    NotifierProvider<
      LibraryClassificationActions,
      LibraryClassificationActivity
    >(LibraryClassificationActions.new);

class LibraryClassificationActions
    extends Notifier<LibraryClassificationActivity> {
  bool _disposed = false;
  LibraryClassificationSelection? _active;

  @override
  LibraryClassificationActivity build() {
    ref.onDispose(() {
      _disposed = true;
      _active?.dispose();
    });
    return const LibraryClassificationActivity();
  }

  Future<bool> apply(
    LibraryClassificationSelection selection,
    LibraryClassificationCommand command,
  ) async {
    if (_disposed || state.saving) return false;
    _active = selection;
    return _run(
      () => selection.apply(_id(), command),
      message: switch (command) {
        LibraryConfirmClassification() => "已确认分类",
        LibraryCorrectClassification() => "已更改分类",
        LibraryDeferClassification() => "已留待确认",
      },
      canUndo: true,
    );
  }

  Future<bool> _run(
    Future<LibraryClassificationReceipt> Function() operation, {
    required String message,
    required bool canUndo,
  }) async {
    state = LibraryClassificationActivity(
      saving: true,
      sequence: state.sequence,
      notice: state.notice,
    );
    try {
      final receipt = await operation();
      if (_disposed) return true;
      state = LibraryClassificationActivity(
        sequence: state.sequence + 1,
        notice: _notice(
          receipt.changedGroups == 0 ? "分类无需更改" : message,
          AmeNotificationSeverity.success,
          actionId: canUndo && receipt.changedGroups > 0
              ? "classification.undo"
              : null,
          context: canUndo && receipt.changedGroups > 0
              ? receipt.operationId
              : null,
        ),
      );
      unawaited(_refresh());
      return true;
    } on Object catch (error) {
      if (!_disposed) showFailure(error);
      return false;
    } finally {
      _active = null;
      if (!_disposed && state.saving) {
        state = LibraryClassificationActivity(
          sequence: state.sequence,
          notice: state.notice,
        );
      }
    }
  }

  void showFailure(Object error) {
    if (_disposed) return;
    final code = error is LibraryClassificationFailure
        ? error.code
        : "unexpected_error";
    if (code == "classification_selection_cancelled") return;
    final message = switch (code) {
      "classification_changed" ||
      "classification_review_changed" ||
      "organization_selection_changed" => "图片或分类已变化，请重新选择后重试",
      "classification_not_configured" => "请先在设置中启用智能相册",
      "classification_selection_empty" => "所选图片已不在当前范围",
      _ => "未能保存分类，请重试",
    };
    state = LibraryClassificationActivity(
      sequence: state.sequence,
      notice: _notice(message, AmeNotificationSeverity.error),
    );
  }

  Future<void> _refresh() async {
    if (_disposed) return;
    final sequence = state.sequence;
    var refreshed = false;
    try {
      refreshed = await ref
          .read(libraryControllerProvider.notifier)
          .refreshCurrentQuery();
    } on Object {
      refreshed = false;
    }
    if (_disposed || sequence != state.sequence) return;
    final notifications = ref.read(ameNotificationControllerProvider.notifier);
    if (refreshed) {
      notifications.resolve("classification-gallery-refresh");
      return;
    }
    notifications.publish(
      const AmeNotificationDraft(
        title: "分类已保存",
        message: "图库尚未刷新，请重试。",
        severity: AmeNotificationSeverity.warning,
        dedupeKey: "classification-gallery-refresh",
        actionId: "classification.refresh",
        actionLabel: "重试",
        isPersistent: true,
      ),
    );
  }

  bool handleNotification(AmeNotificationEntry observed) {
    if (observed.actionId?.startsWith("classification.") != true) return false;
    final notifications = ref.read(ameNotificationControllerProvider.notifier);
    final admitted =
        identical(state.notice, observed) ||
        notifications.admitAction(observed) != null;
    if (!admitted || state.saving) return true;
    if (observed.actionId == "classification.refresh") {
      dismiss(observed.id);
      notifications.dismiss(observed.id);
      unawaited(_refresh());
    } else if (observed.actionContext case final original?) {
      unawaited(
        _run(
          () => ref
              .read(libraryClassificationReviewServiceProvider)
              .undo(_id(), original),
          message: "已撤销分类更改",
          canUndo: false,
        ),
      );
    }
    return true;
  }

  void dismiss(String id) {
    if (state.notice?.id != id) return;
    state = LibraryClassificationActivity(
      saving: state.saving,
      sequence: state.sequence,
    );
  }

  AmeNotificationEntry _notice(
    String message,
    AmeNotificationSeverity severity, {
    String? actionId,
    String? context,
  }) => AmeNotificationEntry(
    id: "classification-${_id()}",
    title: message,
    message: "",
    severity: severity,
    occurredAt: DateTime.now(),
    isUnread: false,
    isActive: true,
    isPersistent: severity == AmeNotificationSeverity.error,
    actionId: actionId,
    actionContext: context,
    actionLabel: switch (actionId) {
      "classification.undo" => "撤销",
      "classification.refresh" => "重试",
      _ => null,
    },
  );

  String _id() {
    final random = Random.secure();
    return List.generate(
      16,
      (_) => random.nextInt(256).toRadixString(16).padLeft(2, "0"),
    ).join();
  }
}

final libraryClassificationItemProvider = FutureProvider.autoDispose
    .family<LibraryClassificationItem?, LibraryOrganizationTarget>((
      ref,
      target,
    ) async {
      ref.watch(
        libraryControllerProvider.select((state) => state.catalogRevision),
      );
      ref.watch(
        libraryClassificationActionsProvider.select((state) => state.sequence),
      );
      final items = await ref
          .read(libraryClassificationReviewServiceProvider)
          .loadItems([target]);
      return items.firstOrNull;
    });
