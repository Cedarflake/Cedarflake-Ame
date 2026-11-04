import "../domain/library_synchronization_models.dart";

LibrarySynchronizationSnapshot retainSynchronizationPhaseStarts({
  required LibrarySynchronizationSnapshot previous,
  required LibrarySynchronizationSnapshot incoming,
}) => LibrarySynchronizationSnapshot(
  isRunning: incoming.isRunning,
  catalogRevision: incoming.catalogRevision,
  appliedMutationCount: incoming.appliedMutationCount,
  lastErrorCode: incoming.lastErrorCode,
  roots: {
    for (final entry in incoming.roots.entries)
      entry.key: _retainPhaseStart(previous.roots[entry.key], entry.value),
  },
);

LibraryRootSynchronizationStatus _retainPhaseStart(
  LibraryRootSynchronizationStatus? previous,
  LibraryRootSynchronizationStatus incoming,
) {
  if (previous == null ||
      previous.rootGeneration != incoming.rootGeneration ||
      previous.phase != incoming.phase) {
    return incoming;
  }
  return LibraryRootSynchronizationStatus(
    rootId: incoming.rootId,
    rootGeneration: incoming.rootGeneration,
    availability: incoming.availability,
    freshness: incoming.freshness,
    freshnessCause: incoming.freshnessCause,
    continuity: incoming.continuity,
    phase: incoming.phase,
    phaseStartedAt: previous.phaseStartedAt,
    sourceStatus: incoming.sourceStatus,
    pendingChangeCount: incoming.pendingChangeCount,
    retryWaitCount: incoming.retryWaitCount,
    freshnessUnknownCount: incoming.freshnessUnknownCount,
    recoveryBlocked: incoming.recoveryBlocked,
    lastIssueCode: incoming.lastIssueCode,
  );
}
