import "library_models.dart";

enum LibraryCatalogFreshness {
  synchronized,
  updating,
  needsReconciliation,
  unavailable,
}

enum LibraryCatalogFreshnessCause {
  noPendingChanges,
  pendingChanges,
  rootUnavailable,
  changeSourceUnhealthy,
  evidenceGap,
  boundedCapacityExceeded,
}

enum LibraryChangeSourceStatus {
  healthy,
  starting,
  degraded,
  failed,
  stopped,
  unsupported,
}

enum LibrarySynchronizationPhase {
  watcherStartup,
  inventoryEnumeration,
  inventoryComparison,
  queuePublication,
  retryWait,
  reconciliation,
  fullScan,
  blocked,
  synchronized,
  unavailable,
}

class LibraryRootSynchronizationStatus {
  const LibraryRootSynchronizationStatus({
    required this.rootId,
    required this.rootGeneration,
    required this.availability,
    required this.freshness,
    required this.freshnessCause,
    required this.phase,
    required this.phaseStartedAt,
    required this.sourceStatus,
    required this.pendingChangeCount,
    required this.retryWaitCount,
    required this.freshnessUnknownCount,
    this.lastIssueCode,
  });

  final String rootId;
  final BigInt rootGeneration;
  final LibraryRootAvailability availability;
  final LibraryCatalogFreshness freshness;
  final LibraryCatalogFreshnessCause freshnessCause;
  final LibrarySynchronizationPhase phase;
  final DateTime phaseStartedAt;
  final LibraryChangeSourceStatus sourceStatus;
  final BigInt pendingChangeCount;
  final BigInt retryWaitCount;
  final BigInt freshnessUnknownCount;
  final String? lastIssueCode;

  LibraryRootSynchronizationStatus degraded({
    String? issueCode,
    required DateTime occurredAt,
  }) {
    final targetPhase = availability == LibraryRootAvailability.available
        ? LibrarySynchronizationPhase.blocked
        : LibrarySynchronizationPhase.unavailable;
    return LibraryRootSynchronizationStatus(
      rootId: rootId,
      rootGeneration: rootGeneration,
      availability: availability,
      freshness: availability == LibraryRootAvailability.available
          ? LibraryCatalogFreshness.needsReconciliation
          : LibraryCatalogFreshness.unavailable,
      freshnessCause: availability == LibraryRootAvailability.available
          ? LibraryCatalogFreshnessCause.changeSourceUnhealthy
          : LibraryCatalogFreshnessCause.rootUnavailable,
      phase: targetPhase,
      phaseStartedAt: phase == targetPhase ? phaseStartedAt : occurredAt,
      sourceStatus: LibraryChangeSourceStatus.failed,
      pendingChangeCount: pendingChangeCount,
      retryWaitCount: retryWaitCount,
      freshnessUnknownCount: freshnessUnknownCount,
      lastIssueCode: issueCode ?? lastIssueCode,
    );
  }

  @override
  bool operator ==(Object other) {
    return other is LibraryRootSynchronizationStatus &&
        rootId == other.rootId &&
        rootGeneration == other.rootGeneration &&
        availability == other.availability &&
        freshness == other.freshness &&
        freshnessCause == other.freshnessCause &&
        phase == other.phase &&
        phaseStartedAt == other.phaseStartedAt &&
        sourceStatus == other.sourceStatus &&
        pendingChangeCount == other.pendingChangeCount &&
        retryWaitCount == other.retryWaitCount &&
        freshnessUnknownCount == other.freshnessUnknownCount &&
        lastIssueCode == other.lastIssueCode;
  }

  @override
  int get hashCode => Object.hash(
    rootId,
    rootGeneration,
    availability,
    freshness,
    freshnessCause,
    phase,
    phaseStartedAt,
    sourceStatus,
    pendingChangeCount,
    retryWaitCount,
    freshnessUnknownCount,
    lastIssueCode,
  );
}

class LibrarySynchronizationSnapshot {
  LibrarySynchronizationSnapshot({
    required this.isRunning,
    required this.catalogRevision,
    required this.appliedMutationCount,
    required Map<String, LibraryRootSynchronizationStatus> roots,
    this.lastErrorCode,
  }) : roots = Map.unmodifiable(roots);

  LibrarySynchronizationSnapshot.stopped()
    : isRunning = false,
      catalogRevision = BigInt.zero,
      appliedMutationCount = 0,
      roots = const {},
      lastErrorCode = null;

  final bool isRunning;
  final BigInt catalogRevision;
  final int appliedMutationCount;
  final Map<String, LibraryRootSynchronizationStatus> roots;
  final String? lastErrorCode;

  LibraryRootSynchronizationStatus? statusFor(String rootId) => roots[rootId];

  LibrarySynchronizationSnapshot degraded(
    String errorCode, {
    required DateTime occurredAt,
  }) {
    return LibrarySynchronizationSnapshot(
      isRunning: isRunning,
      catalogRevision: catalogRevision,
      appliedMutationCount: 0,
      roots: {
        for (final entry in roots.entries)
          entry.key: entry.value.degraded(
            issueCode: errorCode,
            occurredAt: occurredAt,
          ),
      },
      lastErrorCode: errorCode,
    );
  }

  LibrarySynchronizationSnapshot stopped() {
    return LibrarySynchronizationSnapshot(
      isRunning: false,
      catalogRevision: catalogRevision,
      appliedMutationCount: 0,
      roots: roots,
      lastErrorCode: lastErrorCode,
    );
  }

  @override
  bool operator ==(Object other) {
    if (other is! LibrarySynchronizationSnapshot ||
        isRunning != other.isRunning ||
        catalogRevision != other.catalogRevision ||
        appliedMutationCount != other.appliedMutationCount ||
        lastErrorCode != other.lastErrorCode ||
        roots.length != other.roots.length) {
      return false;
    }
    for (final entry in roots.entries) {
      if (other.roots[entry.key] != entry.value) {
        return false;
      }
    }
    return true;
  }

  @override
  int get hashCode => Object.hash(
    isRunning,
    catalogRevision,
    appliedMutationCount,
    lastErrorCode,
    Object.hashAllUnordered(
      roots.entries.map((entry) => Object.hash(entry.key, entry.value)),
    ),
  );
}
