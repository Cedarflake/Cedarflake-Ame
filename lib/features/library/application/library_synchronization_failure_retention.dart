import "../domain/library_synchronization_models.dart";

LibrarySynchronizationSnapshot retainUnresolvedSynchronizationFailures({
  required LibrarySynchronizationSnapshot previous,
  required LibrarySynchronizationSnapshot incoming,
}) {
  var retainedFailure = false;
  final roots = <String, LibraryRootSynchronizationStatus>{};
  for (final entry in incoming.roots.entries) {
    final current = entry.value;
    final prior = previous.roots[entry.key];
    if (prior == null || prior.rootGeneration != current.rootGeneration) {
      roots[entry.key] = current;
      continue;
    }
    if (prior.isAwaitingFirstImport ||
        prior.freshness != LibraryCatalogFreshness.needsReconciliation ||
        current.freshness != LibraryCatalogFreshness.updating) {
      roots[entry.key] = current;
      continue;
    }
    retainedFailure = true;
    roots[entry.key] = LibraryRootSynchronizationStatus(
      rootId: current.rootId,
      rootGeneration: current.rootGeneration,
      availability: current.availability,
      freshness: LibraryCatalogFreshness.needsReconciliation,
      freshnessCause: prior.freshnessCause,
      continuity: current.continuity,
      phase: prior.phase,
      phaseStartedAt: prior.phaseStartedAt,
      sourceStatus: current.sourceStatus,
      pendingChangeCount: current.pendingChangeCount,
      retryWaitCount: current.retryWaitCount,
      freshnessUnknownCount: current.freshnessUnknownCount,
      lastIssueCode: prior.lastIssueCode,
    );
  }
  if (!retainedFailure) {
    return incoming;
  }
  return LibrarySynchronizationSnapshot(
    isRunning: incoming.isRunning,
    catalogRevision: incoming.catalogRevision,
    appliedMutationCount: incoming.appliedMutationCount,
    roots: roots,
    lastErrorCode: previous.lastErrorCode ?? incoming.lastErrorCode,
  );
}
