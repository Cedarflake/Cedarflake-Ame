import "package:cedarflake_ame/features/library/application/library_synchronization_failure_retention.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "resuming first import uses its new progress without an old failure",
    () {
      final paused = _snapshot(
        _status(
          continuity: LibraryContinuityState.baselineRequired,
          sourceStatus: LibraryChangeSourceStatus.stopped,
          issueCode: "library_first_import_required",
        ),
      );
      expect(paused.statusFor("root-a")!.isAwaitingFirstImport, isTrue);

      for (final continuity in [
        LibraryContinuityState.liveOnly,
        LibraryContinuityState.catchingUp,
      ]) {
        final resumed = _snapshot(
          _status(
            freshness: LibraryCatalogFreshness.updating,
            continuity: continuity,
            phase: LibrarySynchronizationPhase.queuePublication,
            pendingCount: 1,
          ),
        );
        final result = retainUnresolvedSynchronizationFailures(
          previous: paused,
          incoming: resumed,
        );
        expect(result, same(resumed));
        expect(result.statusFor("root-a")!.lastIssueCode, isNull);
        expect(
          retainUnresolvedSynchronizationFailures(
            previous: result,
            incoming: resumed,
          ),
          same(resumed),
        );
      }
    },
  );

  test("an actual failure remains until the same root finishes recovery", () {
    final failed = _snapshot(_status(issueCode: "catalog_database_error"));
    final updating = _snapshot(
      _status(
        freshness: LibraryCatalogFreshness.updating,
        phase: LibrarySynchronizationPhase.queuePublication,
        pendingCount: 2,
      ),
    );
    final retained = retainUnresolvedSynchronizationFailures(
      previous: failed,
      incoming: updating,
    ).statusFor("root-a")!;
    expect(retained.freshness, LibraryCatalogFreshness.needsReconciliation);
    expect(retained.lastIssueCode, "catalog_database_error");
    expect(retained.pendingChangeCount, BigInt.two);

    final completed = _snapshot(
      _status(
        freshness: LibraryCatalogFreshness.synchronized,
        phase: LibrarySynchronizationPhase.synchronized,
      ),
    );
    expect(
      retainUnresolvedSynchronizationFailures(
        previous: failed,
        incoming: completed,
      ),
      same(completed),
    );
  });

  test("failure retention does not cross root generations", () {
    final failed = _snapshot(_status(issueCode: "catalog_database_error"));
    final replacement = _snapshot(
      _status(
        freshness: LibraryCatalogFreshness.updating,
        phase: LibrarySynchronizationPhase.queuePublication,
        generation: 2,
      ),
    );
    expect(
      retainUnresolvedSynchronizationFailures(
        previous: failed,
        incoming: replacement,
      ),
      same(replacement),
    );
  });
}

LibrarySynchronizationSnapshot _snapshot(
  LibraryRootSynchronizationStatus root,
) => LibrarySynchronizationSnapshot(
  isRunning: true,
  catalogRevision: BigInt.one,
  appliedMutationCount: 0,
  roots: {root.rootId: root},
);

LibraryRootSynchronizationStatus _status({
  LibraryCatalogFreshness freshness =
      LibraryCatalogFreshness.needsReconciliation,
  LibraryContinuityState continuity = LibraryContinuityState.recoveryRequired,
  LibrarySynchronizationPhase phase = LibrarySynchronizationPhase.blocked,
  LibraryChangeSourceStatus sourceStatus = LibraryChangeSourceStatus.healthy,
  String? issueCode,
  int pendingCount = 0,
  int generation = 1,
}) => LibraryRootSynchronizationStatus(
  rootId: "root-a",
  rootGeneration: BigInt.from(generation),
  availability: LibraryRootAvailability.available,
  freshness: freshness,
  freshnessCause: LibraryCatalogFreshnessCause.pendingChanges,
  continuity: continuity,
  phase: phase,
  phaseStartedAt: DateTime.utc(2026, 10, 15),
  sourceStatus: sourceStatus,
  pendingChangeCount: BigInt.from(pendingCount),
  retryWaitCount: BigInt.zero,
  freshnessUnknownCount: BigInt.zero,
  lastIssueCode: issueCode,
);
