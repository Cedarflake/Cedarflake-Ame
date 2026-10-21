import "../domain/library_models.dart";
import "../domain/library_synchronization_models.dart";

enum LibrarySynchronizationFeedbackState {
  awaitingFirstImport,
  unavailable,
  blocked,
  interrupted,
  awaitingRetry,
  updating,
  backgroundVerification,
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
    if (status.recoveryBlocked) {
      return blocked;
    }
    if (status.sourceStatus == LibraryChangeSourceStatus.failed ||
        status.sourceStatus == LibraryChangeSourceStatus.degraded ||
        status.sourceStatus == LibraryChangeSourceStatus.stopped ||
        status.sourceStatus == LibraryChangeSourceStatus.unsupported) {
      return interrupted;
    }
    if (status.freshness == LibraryCatalogFreshness.updating) {
      if (status.phase == LibrarySynchronizationPhase.backgroundVerification) {
        return backgroundVerification;
      }
      if (status.phase == LibrarySynchronizationPhase.retryWait &&
          status.retryWaitCount > BigInt.zero &&
          status.sourceStatus == LibraryChangeSourceStatus.healthy) {
        return awaitingRetry;
      }
      switch (status.phase) {
        case LibrarySynchronizationPhase.watcherStartup:
        case LibrarySynchronizationPhase.inventoryEnumeration:
        case LibrarySynchronizationPhase.inventoryComparison:
        case LibrarySynchronizationPhase.queuePublication:
        case LibrarySynchronizationPhase.reconciliation:
        case LibrarySynchronizationPhase.fullScan:
          return updating;
        case LibrarySynchronizationPhase.retryWait:
        case LibrarySynchronizationPhase.blocked:
        case LibrarySynchronizationPhase.synchronized:
        case LibrarySynchronizationPhase.unavailable:
        case LibrarySynchronizationPhase.backgroundVerification:
          break;
      }
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
    if (status.lastIssueCode != null) {
      return interrupted;
    }
    return unconfirmed;
  }

  bool get shouldNotify => switch (this) {
    blocked || interrupted => true,
    awaitingFirstImport ||
    unavailable ||
    awaitingRetry ||
    updating ||
    backgroundVerification ||
    unconfirmed ||
    synchronized => false,
  };
}
