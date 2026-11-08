import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/notifications/ame_notification_controller.dart";
import "../application/library_model_installation.dart";
import "../domain/library_model_installation.dart";
import "widgets/library_model_setup.dart";

String libraryModelInstallationNotificationKey(int sequence) =>
    "model-installation-$sequence";

final libraryModelInstallationNotificationsProvider = Provider((ref) {
  final notifications = LibraryModelInstallationNotifications(
    ref.read(ameNotificationControllerProvider.notifier),
  );
  ref.listen(libraryModelInstallationProvider, (_, installation) {
    scheduleMicrotask(() {
      if (ref.mounted) notifications.accept(installation);
    });
  }, fireImmediately: true);
  return notifications;
});

class LibraryModelInstallationNotifications {
  LibraryModelInstallationNotifications(this._notifications);

  final AmeNotificationController _notifications;
  String? _lastPublication;
  int? _sequence;
  DateTime? _startedAt;

  void accept(LibraryModelInstallation? installation) {
    if (installation == null ||
        installation.stage == LibraryModelInstallationStage.idle) {
      return;
    }
    final title = switch (installation.stage) {
      LibraryModelInstallationStage.downloading => "正在下载智能相册模型",
      LibraryModelInstallationStage.verifying => "正在校验模型",
      LibraryModelInstallationStage.stopping => "正在停止",
      LibraryModelInstallationStage.enabling => "正在启用智能相册",
      LibraryModelInstallationStage.enabled => "已启用智能相册",
      LibraryModelInstallationStage.cancelled => "已取消启用智能相册",
      LibraryModelInstallationStage.failed => "智能相册未能启用",
      _ => "正在准备智能相册模型",
    };
    final progress = installation.progress;
    final downloading =
        installation.stage == LibraryModelInstallationStage.downloading;
    final message = downloading && progress != null && progress.totalBytes > 0
        ? "${(100 * progress.completedBytes / progress.totalBytes).clamp(0, 100).toStringAsFixed(0)}% · "
              "${(progress.completedBytes / 1000000).toStringAsFixed(1)} / "
              "${(progress.totalBytes / 1000000).toStringAsFixed(1)} MB"
        : "";
    final failed = installation.stage == LibraryModelInstallationStage.failed;
    final (action, actionLabel) = switch (installation.stage) {
      LibraryModelInstallationStage.preparing ||
      LibraryModelInstallationStage.verifying ||
      LibraryModelInstallationStage.downloading => (
        "cancel-model-installation",
        "取消",
      ),
      LibraryModelInstallationStage.failed => (
        "retry-model-installation",
        "重试",
      ),
      _ => (null, null),
    };
    final severity = switch (installation.stage) {
      LibraryModelInstallationStage.failed => AmeNotificationSeverity.error,
      LibraryModelInstallationStage.enabled => AmeNotificationSeverity.success,
      _ => AmeNotificationSeverity.info,
    };
    final publication = "${installation.sequence}|$title|$message|$action";
    if (publication == _lastPublication) return;
    _lastPublication = publication;
    if (_sequence != installation.sequence) {
      if (_sequence case final previous?) {
        _notifications.resolve(
          libraryModelInstallationNotificationKey(previous),
        );
      }
      _sequence = installation.sequence;
      _startedAt = DateTime.now();
    }
    final key = libraryModelInstallationNotificationKey(installation.sequence);
    _notifications.publish(
      AmeNotificationDraft(
        dedupeKey: key,
        title: title,
        message: message,
        detail: failed ? "下载进度已保留，可以重试。" : null,
        severity: severity,
        isPersistent: installation.busy || failed,
        actionId: action,
        actionLabel: actionLabel,
        actionContext: installation.sequence.toString(),
      ),
      occurredAt: _startedAt,
    );
    if (!installation.busy && !failed) {
      _notifications.resolve(key, dismiss: false);
    }
  }
}

bool handleLibraryModelInstallationNotification(
  BuildContext context,
  WidgetRef ref,
  AmeNotificationEntry observed,
) {
  if (observed.actionId != "cancel-model-installation" &&
      observed.actionId != "retry-model-installation") {
    return false;
  }
  final current = ref
      .read(ameNotificationControllerProvider.notifier)
      .admitAction(observed);
  final installation = ref.read(libraryModelInstallationProvider);
  if (current == null ||
      installation == null ||
      current.actionContext != installation.sequence.toString()) {
    return true;
  }
  if (current.actionId == "cancel-model-installation") {
    installation.cancel();
  } else if (installation.canStart) {
    unawaited(showLibraryModelSetup(context, ref));
  }
  return true;
}
