import "../../features/updates/application/application_update_controller.dart";
import "../../features/updates/application/application_update_state.dart";
import "../../features/updates/domain/application_update.dart";
import "../messages/ame_message.dart";
import "../notifications/ame_notification_controller.dart";

class ApplicationUpdateNotices {
  ApplicationUpdateNotices(this.controller, this.notifications) {
    controller.addListener(_changed);
  }
  final ApplicationUpdateController controller;
  final AmeNotificationController notifications;
  static const action = "application-update";
  (ApplicationUpdatePhase, String?, ApplicationUpdateFailureCode?)? _last;
  bool _reportedResult = false;

  void _changed() {
    final result = controller.lastResult;
    if (!_reportedResult && result != null) {
      _reportedResult = true;
      notifications.publish(
        AmeNotificationDraft(
          dedupeKey: "application-update-result",
          title: AmeLabel.updateTitle,
          message: switch (result.outcome) {
            ApplicationUpdateOutcome.updated => AmeLabel.updateInstalled,
            ApplicationUpdateOutcome.cancelled => AmeLabel.updateCancelled,
            ApplicationUpdateOutcome.failed =>
              AmeLabel.updateInstallationFailure,
          },
          severity: result.outcome == ApplicationUpdateOutcome.failed
              ? AmeNotificationSeverity.error
              : AmeNotificationSeverity.info,
          technicalCode: result.outcome == ApplicationUpdateOutcome.failed
              ? "WIN32: ${result.code}"
              : null,
        ),
      );
    }
    final state = controller.state;
    final key = (state.phase, state.release?.version, state.failure?.code);
    if (_last == key) return;
    _last = key;
    if (state.phase == ApplicationUpdatePhase.idle ||
        state.phase == ApplicationUpdatePhase.current) {
      notifications.resolve("application-update");
    }
    final title = state.failure != null
        ? AmeLabel.updateFailed
        : switch (state.phase) {
            ApplicationUpdatePhase.available => AmeLabel.updateAvailable,
            ApplicationUpdatePhase.downloading => AmeLabel.updateDownloading,
            ApplicationUpdatePhase.verifying => AmeLabel.updateVerifying,
            ApplicationUpdatePhase.stopping => AmeLabel.updateStopping,
            ApplicationUpdatePhase.ready => AmeLabel.updateReady,
            ApplicationUpdatePhase.failed => AmeLabel.updateFailed,
            _ => null,
          };
    if (title == null) return;
    final failure = state.failure;
    notifications.publish(
      AmeNotificationDraft(
        dedupeKey: "application-update",
        title: title,
        message: failure == null
            ? AmeRawMessage("Ame ${state.release?.version ?? ""}")
            : _failure(failure.code),
        detail: failure == null ? null : AmeRawMessage(failure.diagnostic),
        severity: failure == null
            ? AmeNotificationSeverity.info
            : AmeNotificationSeverity.error,
        actionId: action,
        actionLabel: AmeLabel.updateView,
        actionContext: state.release?.version,
        isPersistent: true,
      ),
    );
  }

  void close() => controller.removeListener(_changed);
}

AmeLabel _failure(ApplicationUpdateFailureCode code) => switch (code) {
  ApplicationUpdateFailureCode.unavailable => AmeLabel.updateUnavailable,
  ApplicationUpdateFailureCode.network => AmeLabel.updateNetworkFailure,
  ApplicationUpdateFailureCode.invalidRelease => AmeLabel.updateReleaseFailure,
  ApplicationUpdateFailureCode.damagedDownload => AmeLabel.updatePackageFailure,
  ApplicationUpdateFailureCode.storage => AmeLabel.updateStorageFailure,
  ApplicationUpdateFailureCode.installation =>
    AmeLabel.updateInstallationFailure,
  ApplicationUpdateFailureCode.shutdown => AmeLabel.updateShutdownFailure,
};
