import "../../../../src/rust/domain.dart" as rust_domain;
import "../../../../src/rust/domain/library_change.dart" as rust_change;
import "../../../../src/rust/domain/library_synchronization.dart" as rust_sync;
import "../../../../src/rust/domain/persistent_journal.dart" as rust_journal;
import "../../domain/library_models.dart";
import "../../domain/library_synchronization_models.dart";

LibrarySynchronizationSnapshot mapRustSynchronizationSnapshot(
  rust_sync.LibrarySynchronizationSnapshot snapshot, {
  required DateTime observedAt,
}) {
  return LibrarySynchronizationSnapshot(
    isRunning: snapshot.isRunning,
    catalogRevision: snapshot.catalogRevision,
    appliedMutationCount: snapshot.appliedMutationCount,
    roots: {
      for (final root in snapshot.roots)
        root.rootId: _mapRootStatus(root, observedAt),
    },
  );
}

LibraryRootSynchronizationStatus _mapRootStatus(
  rust_sync.LibraryRootSynchronizationStatus root,
  DateTime observedAt,
) {
  final phase = switch (root.phase) {
    rust_sync.LibrarySynchronizationPhase.watcherStartup =>
      LibrarySynchronizationPhase.watcherStartup,
    rust_sync.LibrarySynchronizationPhase.inventoryEnumeration =>
      LibrarySynchronizationPhase.inventoryEnumeration,
    rust_sync.LibrarySynchronizationPhase.inventoryComparison =>
      LibrarySynchronizationPhase.inventoryComparison,
    rust_sync.LibrarySynchronizationPhase.queuePublication =>
      LibrarySynchronizationPhase.queuePublication,
    rust_sync.LibrarySynchronizationPhase.retryWait =>
      LibrarySynchronizationPhase.retryWait,
    rust_sync.LibrarySynchronizationPhase.reconciliation =>
      LibrarySynchronizationPhase.reconciliation,
    rust_sync.LibrarySynchronizationPhase.fullScan =>
      LibrarySynchronizationPhase.fullScan,
    rust_sync.LibrarySynchronizationPhase.blocked =>
      LibrarySynchronizationPhase.blocked,
    rust_sync.LibrarySynchronizationPhase.synchronized =>
      LibrarySynchronizationPhase.synchronized,
    rust_sync.LibrarySynchronizationPhase.unavailable =>
      LibrarySynchronizationPhase.unavailable,
    rust_sync.LibrarySynchronizationPhase.backgroundVerification =>
      LibrarySynchronizationPhase.backgroundVerification,
  };
  return LibraryRootSynchronizationStatus(
    rootId: root.rootId,
    rootGeneration: root.rootGeneration,
    availability: _mapAvailability(root.availability),
    freshness: switch (root.freshness) {
      rust_change.CatalogFreshnessState.synchronized =>
        LibraryCatalogFreshness.synchronized,
      rust_change.CatalogFreshnessState.updating =>
        LibraryCatalogFreshness.updating,
      rust_change.CatalogFreshnessState.needsReconciliation =>
        LibraryCatalogFreshness.needsReconciliation,
      rust_change.CatalogFreshnessState.unavailable =>
        LibraryCatalogFreshness.unavailable,
    },
    freshnessCause: switch (root.freshnessCause) {
      rust_change.CatalogFreshnessCause.noPendingChanges =>
        LibraryCatalogFreshnessCause.noPendingChanges,
      rust_change.CatalogFreshnessCause.pendingChanges =>
        LibraryCatalogFreshnessCause.pendingChanges,
      rust_change.CatalogFreshnessCause.rootUnavailable =>
        LibraryCatalogFreshnessCause.rootUnavailable,
      rust_change.CatalogFreshnessCause.changeSourceUnhealthy =>
        LibraryCatalogFreshnessCause.changeSourceUnhealthy,
      rust_change.CatalogFreshnessCause.evidenceGap =>
        LibraryCatalogFreshnessCause.evidenceGap,
      rust_change.CatalogFreshnessCause.boundedCapacityExceeded =>
        LibraryCatalogFreshnessCause.boundedCapacityExceeded,
    },
    continuity: switch (root.continuity) {
      rust_journal.PersistentJournalContinuityState.baselineRequired =>
        LibraryContinuityState.baselineRequired,
      rust_journal.PersistentJournalContinuityState.catchingUp =>
        LibraryContinuityState.catchingUp,
      rust_journal.PersistentJournalContinuityState.current =>
        LibraryContinuityState.current,
      rust_journal.PersistentJournalContinuityState.recoveryRequired =>
        LibraryContinuityState.recoveryRequired,
      rust_journal.PersistentJournalContinuityState.liveOnly =>
        LibraryContinuityState.liveOnly,
      rust_journal.PersistentJournalContinuityState.unavailable =>
        LibraryContinuityState.unavailable,
    },
    phase: phase,
    phaseStartedAt: observedAt,
    sourceStatus: switch (root.sourceHealth) {
      rust_change.LibraryChangeSourceHealth.healthy =>
        LibraryChangeSourceStatus.healthy,
      rust_change.LibraryChangeSourceHealth.starting =>
        LibraryChangeSourceStatus.starting,
      rust_change.LibraryChangeSourceHealth.degraded =>
        LibraryChangeSourceStatus.degraded,
      rust_change.LibraryChangeSourceHealth.failed =>
        LibraryChangeSourceStatus.failed,
      rust_change.LibraryChangeSourceHealth.stopped =>
        LibraryChangeSourceStatus.stopped,
      rust_change.LibraryChangeSourceHealth.unsupported =>
        LibraryChangeSourceStatus.unsupported,
    },
    pendingChangeCount: root.pendingChangeCount,
    retryWaitCount: root.retryWaitCount,
    freshnessUnknownCount: root.freshnessUnknownCount,
    recoveryBlocked: root.recoveryBlocked,
    lastIssueCode: root.lastIssueCode,
  );
}

LibraryRootAvailability _mapAvailability(
  rust_domain.LibraryRootAvailability availability,
) {
  return switch (availability) {
    rust_domain.LibraryRootAvailability.unknown =>
      LibraryRootAvailability.unknown,
    rust_domain.LibraryRootAvailability.available =>
      LibraryRootAvailability.available,
    rust_domain.LibraryRootAvailability.missing =>
      LibraryRootAvailability.missing,
    rust_domain.LibraryRootAvailability.inaccessible =>
      LibraryRootAvailability.inaccessible,
    rust_domain.LibraryRootAvailability.offline =>
      LibraryRootAvailability.offline,
  };
}
