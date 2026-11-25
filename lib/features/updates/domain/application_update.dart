enum ApplicationUpdateChannel { stable, preview }

enum ApplicationUpdateOutcome { updated, cancelled, failed }

class ApplicationUpdateResult {
  const ApplicationUpdateResult(this.outcome, this.code);
  final ApplicationUpdateOutcome outcome;
  final int code;
}

enum ApplicationUpdatePolicy { manual, check, download }

class ApplicationUpdatePreferences {
  const ApplicationUpdatePreferences({
    required this.channel,
    this.policy = ApplicationUpdatePolicy.check,
  });

  final ApplicationUpdateChannel channel;
  final ApplicationUpdatePolicy policy;

  ApplicationUpdatePreferences copyWith({
    ApplicationUpdateChannel? channel,
    ApplicationUpdatePolicy? policy,
  }) => ApplicationUpdatePreferences(
    channel: channel ?? this.channel,
    policy: policy ?? this.policy,
  );
}

class ApplicationRelease {
  const ApplicationRelease({
    required this.version,
    required this.build,
    required this.downloadBytes,
    required this.notes,
  });

  final String version;
  final int build;
  final int downloadBytes;
  final String notes;
}

enum ApplicationUpdateFailureCode {
  unavailable,
  network,
  invalidRelease,
  damagedDownload,
  storage,
  installation,
  shutdown,
}

class ApplicationUpdateFailure implements Exception {
  const ApplicationUpdateFailure(this.code, this.diagnostic);

  final ApplicationUpdateFailureCode code;
  final String diagnostic;

  @override
  String toString() => diagnostic;
}

enum ApplicationDownloadPhase { downloading, verifying, ready, cancelled }

class ApplicationDownloadProgress {
  const ApplicationDownloadProgress({
    required this.phase,
    required this.receivedBytes,
    required this.totalBytes,
  });

  final ApplicationDownloadPhase phase;
  final int receivedBytes;
  final int totalBytes;
}
