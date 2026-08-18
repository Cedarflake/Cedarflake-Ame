import "dart:async";

import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust_domain;
import "package:cedarflake_ame/src/rust/domain/library_change.dart"
    as rust_change;
import "package:cedarflake_ame/src/rust/domain/library_change_queue.dart"
    as rust_queue;
import "package:cedarflake_ame/src/rust/domain/library_synchronization.dart"
    as rust_sync;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "maps root freshness and preserves the latest catalog revision",
    () async {
      final synchronization = RustLibrarySynchronization(
        startCall: () async => _snapshot(revision: 7),
        pollCall: () async => _snapshot(revision: 7),
        stopCall: () async {},
        pollInterval: const Duration(days: 1),
      );

      await synchronization.start();

      expect(synchronization.current.catalogRevision, BigInt.from(7));
      final root = synchronization.current.statusFor("root-a");
      expect(root?.availability, LibraryRootAvailability.available);
      expect(root?.freshness, LibraryCatalogFreshness.synchronized);
      await synchronization.stop();
      expect(synchronization.current.isRunning, isFalse);
    },
  );

  test(
    "timer polling never overlaps and stop waits for the active poll",
    () async {
      final pollStarted = Completer<void>();
      final releasePoll = Completer<void>();
      var activePolls = 0;
      var maximumActivePolls = 0;
      var pollCalls = 0;
      var stopCalls = 0;
      final synchronization = RustLibrarySynchronization(
        startCall: () async => _snapshot(revision: 1),
        pollCall: () async {
          pollCalls += 1;
          activePolls += 1;
          maximumActivePolls = maximumActivePolls < activePolls
              ? activePolls
              : maximumActivePolls;
          if (!pollStarted.isCompleted) {
            pollStarted.complete();
          }
          await releasePoll.future;
          activePolls -= 1;
          return _snapshot(revision: 2);
        },
        stopCall: () async => stopCalls += 1,
        pollInterval: const Duration(milliseconds: 5),
      );

      await synchronization.start();
      await pollStarted.future;
      await Future<void>.delayed(const Duration(milliseconds: 20));
      final stopping = synchronization.stop();
      await Future<void>.delayed(Duration.zero);
      expect(stopCalls, 0);

      releasePoll.complete();
      await stopping;

      expect(pollCalls, 1);
      expect(maximumActivePolls, 1);
      expect(stopCalls, 1);
      expect(synchronization.current.catalogRevision, BigInt.from(2));

      await synchronization.stop();
      expect(stopCalls, 1);
    },
  );

  test("poll failure degrades known roots without losing revision", () async {
    final synchronization = RustLibrarySynchronization(
      startCall: () async => _snapshot(revision: 9),
      pollCall: () async => throw const rust_domain.ScanError(
        code: "watcher_failed",
        message: "failed",
      ),
      stopCall: () async {},
      pollInterval: const Duration(milliseconds: 5),
    );

    await synchronization.start();
    await Future<void>.delayed(const Duration(milliseconds: 20));

    expect(synchronization.current.catalogRevision, BigInt.from(9));
    expect(
      synchronization.current.statusFor("root-a")?.freshness,
      LibraryCatalogFreshness.needsReconciliation,
    );
    expect(synchronization.current.lastErrorCode, "watcher_failed");
    await synchronization.stop();
  });

  test("equal snapshots have an order-independent hash code", () {
    final rootA = _rootStatus("root-a");
    final rootB = _rootStatus("root-b");
    final first = LibrarySynchronizationSnapshot(
      isRunning: true,
      catalogRevision: BigInt.one,
      appliedMutationCount: 0,
      roots: {"root-a": rootA, "root-b": rootB},
    );
    final second = LibrarySynchronizationSnapshot(
      isRunning: true,
      catalogRevision: BigInt.one,
      appliedMutationCount: 0,
      roots: {"root-b": rootB, "root-a": rootA},
    );

    expect(first, second);
    expect(first.hashCode, second.hashCode);
  });
}

LibraryRootSynchronizationStatus _rootStatus(String rootId) {
  return LibraryRootSynchronizationStatus(
    rootId: rootId,
    rootGeneration: BigInt.one,
    availability: LibraryRootAvailability.available,
    freshness: LibraryCatalogFreshness.synchronized,
    freshnessCause: LibraryCatalogFreshnessCause.noPendingChanges,
    sourceStatus: LibraryChangeSourceStatus.healthy,
    pendingChangeCount: BigInt.zero,
    retryWaitCount: BigInt.zero,
    freshnessUnknownCount: BigInt.zero,
  );
}

rust_sync.LibrarySynchronizationSnapshot _snapshot({required int revision}) {
  return rust_sync.LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.from(revision),
    appliedMutationCount: 0,
    roots: [
      rust_sync.LibraryRootSynchronizationStatus(
        rootId: "root-a",
        rootGeneration: BigInt.one,
        availability: rust_domain.LibraryRootAvailability.available,
        freshness: rust_change.CatalogFreshnessState.synchronized,
        freshnessCause: rust_change.CatalogFreshnessCause.noPendingChanges,
        sourceHealth: rust_change.LibraryChangeSourceHealth.healthy,
        queueHealth: rust_queue.LibraryChangeQueueHealth.idle,
        pendingChangeCount: BigInt.zero,
        retryWaitCount: BigInt.zero,
        freshnessUnknownCount: BigInt.zero,
      ),
    ],
  );
}
