import "dart:async";

import "package:material_ui/material_ui.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../app/messages/ame_message.dart";
import "../../../app/notifications/ame_notification_controller.dart";
import "../application/library_model_installation.dart";
import "../domain/library_classification_review.dart";
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
  (
    int,
    AmeLabel,
    ({int percent, int receivedTenths, int totalTenths})?,
    String?,
  )?
  _lastPublication;
  int? _sequence;
  DateTime? _startedAt;

  void accept(LibraryModelInstallation? installation) {
    if (installation == null ||
        installation.stage == LibraryModelInstallationStage.idle) {
      return;
    }
    final title = switch (installation.stage) {
      LibraryModelInstallationStage.downloading => AmeLabel.modelDownloading,
      LibraryModelInstallationStage.verifying => AmeLabel.modelVerifying,
      LibraryModelInstallationStage.stopping => AmeLabel.stopping,
      LibraryModelInstallationStage.enabling => AmeLabel.modelEnabling,
      LibraryModelInstallationStage.enabled => AmeLabel.modelEnabled,
      LibraryModelInstallationStage.cancelled => AmeLabel.modelEnableCancelled,
      LibraryModelInstallationStage.failed => AmeLabel.modelEnableFailed,
      _ => AmeLabel.modelPreparing,
    };
    final progress = installation.progress;
    final downloading =
        installation.stage == LibraryModelInstallationStage.downloading;
    final transfer = downloading && progress != null && progress.totalBytes > 0
        ? (
            percent: (100 * progress.completedBytes / progress.totalBytes)
                .clamp(0, 100)
                .round(),
            receivedTenths: (progress.completedBytes / 100000).round(),
            totalTenths: (progress.totalBytes / 100000).round(),
          )
        : null;
    final failed = installation.stage == LibraryModelInstallationStage.failed;
    final failure = switch (installation.error) {
      LibraryClassificationFailure error => error,
      _ => null,
    };
    final (action, actionLabel) = switch (installation.stage) {
      LibraryModelInstallationStage.preparing ||
      LibraryModelInstallationStage.verifying ||
      LibraryModelInstallationStage.downloading => (
        "cancel-model-installation",
        AmeLabel.cancel,
      ),
      LibraryModelInstallationStage.failed => (
        "retry-model-installation",
        AmeLabel.retry,
      ),
      _ => (null, null),
    };
    final severity = switch (installation.stage) {
      LibraryModelInstallationStage.failed => AmeNotificationSeverity.error,
      LibraryModelInstallationStage.enabled => AmeNotificationSeverity.success,
      _ => AmeNotificationSeverity.info,
    };
    final publication = (installation.sequence, title, transfer, action);
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
        message: transfer == null
            ? const AmeRawMessage("")
            : AmeDownloadProgressMessage(
                transfer.percent,
                transfer.receivedTenths / 10,
                transfer.totalTenths / 10,
              ),
        detail: failed ? _failureDetail(installation.error) : null,
        technicalCode: failed ? failure?.code : null,
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

AmeMessage _failureDetail(Object? error) => switch (error) {
  LibraryClassificationFailure(
    code: "classification_installation_unavailable",
  ) =>
    AmeLabel.modelConfigUnreadable,
  LibraryClassificationFailure(:final code)
      when code.startsWith("classification_download_") =>
    AmeLabel.modelDownloadRetained,
  _ => AmeLabel.modelEnableRetry,
};

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
