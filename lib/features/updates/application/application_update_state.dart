import "../domain/application_update.dart";

enum ApplicationUpdatePhase {
  initializing,
  idle,
  checking,
  current,
  available,
  downloading,
  verifying,
  stopping,
  ready,
  restarting,
  failed,
}

class ApplicationUpdateState {
  const ApplicationUpdateState({
    required this.preferences,
    this.phase = ApplicationUpdatePhase.initializing,
    this.supported = false,
    this.release,
    this.progress,
    this.failure,
  });

  final ApplicationUpdatePreferences preferences;
  final ApplicationUpdatePhase phase;
  final bool supported;
  final ApplicationRelease? release;
  final ApplicationDownloadProgress? progress;
  final ApplicationUpdateFailure? failure;

  bool get busy => switch (phase) {
    ApplicationUpdatePhase.initializing ||
    ApplicationUpdatePhase.checking ||
    ApplicationUpdatePhase.downloading ||
    ApplicationUpdatePhase.verifying ||
    ApplicationUpdatePhase.stopping ||
    ApplicationUpdatePhase.restarting => true,
    _ => false,
  };
}
