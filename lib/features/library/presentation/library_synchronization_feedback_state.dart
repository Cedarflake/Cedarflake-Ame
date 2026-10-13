import "../domain/library_models.dart";
import "../domain/library_synchronization_models.dart";

enum LibrarySynchronizationFeedbackState {
  awaitingFirstImport,
  unavailable,
  blocked,
  interrupted,
  awaitingRetry,
  updating,
  unconfirmed,
  synchronized;

  static LibrarySynchronizationFeedbackState fromStatus(
    LibraryRootSynchronizationStatus status,
  ) {
    if (status.isAwaitingFirstImport) {
      return awaitingFirstImport;
    }
    if (status.availability != LibraryRootAvailability.available ||
        status.freshness == LibraryCatalogFreshness.unavailable ||
        status.continuity == LibraryContinuityState.unavailable) {
      return unavailable;
    }
    if (status.recoveryBlocked ||
        status.lastIssueCode == "legacy_recovery_authority_missing" ||
        status.lastIssueCode == "live_gap_v30_explicit_recovery_required") {
      return blocked;
    }
    if (status.sourceStatus == LibraryChangeSourceStatus.failed ||
        status.sourceStatus == LibraryChangeSourceStatus.degraded ||
        status.sourceStatus == LibraryChangeSourceStatus.stopped ||
        status.sourceStatus == LibraryChangeSourceStatus.unsupported) {
      return interrupted;
    }
    if (status.freshness == LibraryCatalogFreshness.updating) {
      if (status.phase == LibrarySynchronizationPhase.retryWait &&
          status.retryWaitCount > BigInt.zero &&
          status.sourceStatus == LibraryChangeSourceStatus.healthy) {
        return awaitingRetry;
      }
      if (status.lastIssueCode == null &&
          status.phase != LibrarySynchronizationPhase.retryWait &&
          status.phase != LibrarySynchronizationPhase.blocked &&
          status.phase != LibrarySynchronizationPhase.synchronized) {
        return updating;
      }
    }
    if (status.lastIssueCode != null) {
      return interrupted;
    }
    if (status.freshness == LibraryCatalogFreshness.synchronized &&
        (status.continuity == LibraryContinuityState.current ||
            status.continuity == LibraryContinuityState.liveOnly) &&
        status.phase == LibrarySynchronizationPhase.synchronized &&
        status.sourceStatus == LibraryChangeSourceStatus.healthy &&
        status.pendingChangeCount == BigInt.zero &&
        status.retryWaitCount == BigInt.zero &&
        status.freshnessUnknownCount == BigInt.zero) {
      return synchronized;
    }
    return unconfirmed;
  }

  bool get shouldNotify => switch (this) {
    awaitingFirstImport || blocked || interrupted || unconfirmed => true,
    unavailable || awaitingRetry || updating || synchronized => false,
  };
}
