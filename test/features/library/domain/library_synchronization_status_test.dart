import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "projects dormant first import without asserting source availability",
    () {
      final status = _status();
      expect(status.isAwaitingFirstImport, isTrue);
      expect(status.availability, LibraryRootAvailability.unknown);
    },
  );

  test("requires the exact dormant first-import contract", () {
    final otherStates = [
      _status(code: "live_gap_v30_explicit_recovery_required"),
      _status(continuity: LibraryContinuityState.recoveryRequired),
      _status(freshness: LibraryCatalogFreshness.updating),
      _status(phase: LibrarySynchronizationPhase.fullScan),
      _status(sourceStatus: LibraryChangeSourceStatus.healthy),
      _status(recoveryBlocked: true),
    ];
    for (final status in otherStates) {
      expect(status.isAwaitingFirstImport, isFalse);
    }
  });
}

LibraryRootSynchronizationStatus _status({
  String code = "library_first_import_required",
  LibraryContinuityState continuity = LibraryContinuityState.baselineRequired,
  LibraryCatalogFreshness freshness =
      LibraryCatalogFreshness.needsReconciliation,
  LibrarySynchronizationPhase phase = LibrarySynchronizationPhase.blocked,
  LibraryChangeSourceStatus sourceStatus = LibraryChangeSourceStatus.stopped,
  bool recoveryBlocked = false,
}) => LibraryRootSynchronizationStatus(
  rootId: "first-import-root",
  rootGeneration: BigInt.one,
  availability: LibraryRootAvailability.unknown,
  freshness: freshness,
  freshnessCause: LibraryCatalogFreshnessCause.pendingChanges,
  continuity: continuity,
  phase: phase,
  phaseStartedAt: DateTime.utc(2026, 9, 6),
  sourceStatus: sourceStatus,
  pendingChangeCount: BigInt.zero,
  retryWaitCount: BigInt.zero,
  freshnessUnknownCount: BigInt.zero,
  recoveryBlocked: recoveryBlocked,
  lastIssueCode: code,
);
