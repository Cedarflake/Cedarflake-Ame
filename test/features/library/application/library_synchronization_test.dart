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
import "package:flutter/foundation.dart";
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
      expect(root?.phase, LibrarySynchronizationPhase.synchronized);
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

  test("short catalog writer contention retains the prior snapshot", () async {
    final firstPoll = Completer<void>();
    final synchronization = RustLibrarySynchronization(
      startCall: () async => _snapshot(
        revision: 9,
        freshness: rust_change.CatalogFreshnessState.updating,
      ),
      pollCall: () async {
        if (!firstPoll.isCompleted) {
          firstPoll.complete();
        }
        throw const rust_domain.ScanError(
          code: "catalog_database_busy",
          message: "writer is publishing",
        );
      },
      stopCall: () async {},
      pollInterval: const Duration(milliseconds: 1),
      transientFailureTolerance: const Duration(minutes: 1),
      now: () => DateTime.utc(2026, 8, 21),
      enableDebugLogging: false,
    );

    await synchronization.start();
    await firstPoll.future;
    await Future<void>.delayed(Duration.zero);

    expect(
      synchronization.current.statusFor("root-a")?.freshness,
      LibraryCatalogFreshness.updating,
    );
    expect(synchronization.current.lastErrorCode, isNull);
    await synchronization.stop();
  });

  test(
    "catalog writer contention becomes visible after the bounded wait",
    () async {
      final firstPoll = Completer<void>();
      final secondPoll = Completer<void>();
      var pollCalls = 0;
      var currentTime = DateTime.utc(2026, 8, 21);
      final synchronization = RustLibrarySynchronization(
        startCall: () async => _snapshot(
          revision: 9,
          freshness: rust_change.CatalogFreshnessState.updating,
        ),
        pollCall: () async {
          pollCalls += 1;
          if (pollCalls == 1) {
            firstPoll.complete();
          } else if (pollCalls == 2) {
            secondPoll.complete();
          }
          throw const rust_domain.ScanError(
            code: "catalog_database_locked",
            message: "writer remained locked",
          );
        },
        stopCall: () async {},
        pollInterval: const Duration(milliseconds: 10),
        transientFailureTolerance: const Duration(seconds: 30),
        now: () => currentTime,
        enableDebugLogging: false,
      );

      await synchronization.start();
      await firstPoll.future;
      await Future<void>.delayed(Duration.zero);

      expect(
        synchronization.current.statusFor("root-a")?.freshness,
        LibraryCatalogFreshness.updating,
      );
      expect(synchronization.current.lastErrorCode, isNull);

      currentTime = currentTime.add(const Duration(seconds: 31));
      await secondPoll.future;
      await Future<void>.delayed(Duration.zero);

      expect(
        synchronization.current.statusFor("root-a")?.freshness,
        LibraryCatalogFreshness.needsReconciliation,
      );
      expect(synchronization.current.lastErrorCode, "catalog_database_locked");
      await synchronization.stop();
    },
  );

  test(
    "poll failure remains stable through retry until synchronization succeeds",
    () async {
      final retryReturned = Completer<void>();
      final releaseSynchronized = Completer<void>();
      final synchronizedReturned = Completer<void>();
      var pollCalls = 0;
      final synchronization = RustLibrarySynchronization(
        startCall: () async => _snapshot(revision: 9),
        pollCall: () async {
          pollCalls += 1;
          if (pollCalls == 1) {
            throw const rust_domain.ScanError(
              code: "catalog_database_error",
              message: "write failed",
            );
          }
          if (pollCalls == 2) {
            retryReturned.complete();
            return _snapshot(
              revision: 9,
              freshness: rust_change.CatalogFreshnessState.updating,
            );
          }
          await releaseSynchronized.future;
          if (!synchronizedReturned.isCompleted) {
            synchronizedReturned.complete();
          }
          return _snapshot(revision: 10);
        },
        stopCall: () async {},
        pollInterval: const Duration(milliseconds: 1),
        enableDebugLogging: false,
      );

      await synchronization.start();
      await retryReturned.future;
      await Future<void>.delayed(Duration.zero);

      expect(
        synchronization.current.statusFor("root-a")?.freshness,
        LibraryCatalogFreshness.needsReconciliation,
      );
      expect(synchronization.current.lastErrorCode, "catalog_database_error");

      releaseSynchronized.complete();
      await synchronizedReturned.future;
      await Future<void>.delayed(Duration.zero);

      expect(
        synchronization.current.statusFor("root-a")?.freshness,
        LibraryCatalogFreshness.synchronized,
      );
      expect(synchronization.current.lastErrorCode, isNull);
      await synchronization.stop();
    },
  );

  test(
    "per-root failure remains stable while automatic recovery is running",
    () async {
      final recoveryReturned = Completer<void>();
      final releaseSynchronized = Completer<void>();
      var pollCalls = 0;
      final synchronization = RustLibrarySynchronization(
        startCall: () async => _snapshot(
          revision: 9,
          freshness: rust_change.CatalogFreshnessState.needsReconciliation,
          lastIssueCode: "catalog_database_error",
        ),
        pollCall: () async {
          pollCalls += 1;
          if (pollCalls == 1) {
            recoveryReturned.complete();
            return _snapshot(
              revision: 9,
              freshness: rust_change.CatalogFreshnessState.updating,
            );
          }
          await releaseSynchronized.future;
          return _snapshot(revision: 10);
        },
        stopCall: () async {},
        pollInterval: const Duration(milliseconds: 1),
        enableDebugLogging: false,
      );

      await synchronization.start();
      await recoveryReturned.future;
      await Future<void>.delayed(Duration.zero);

      expect(
        synchronization.current.statusFor("root-a")?.freshness,
        LibraryCatalogFreshness.needsReconciliation,
      );
      expect(
        synchronization.current.statusFor("root-a")?.lastIssueCode,
        "catalog_database_error",
      );
      expect(
        synchronization.current.statusFor("root-a")?.phase,
        LibrarySynchronizationPhase.blocked,
      );
      expect(synchronization.current.lastErrorCode, isNull);

      releaseSynchronized.complete();
      await Future<void>.delayed(const Duration(milliseconds: 5));

      expect(
        synchronization.current.statusFor("root-a")?.freshness,
        LibraryCatalogFreshness.synchronized,
      );
      expect(
        synchronization.current.statusFor("root-a")?.lastIssueCode,
        isNull,
      );
      await synchronization.stop();
    },
  );

  test(
    "development diagnostics include root phase elapsed counts and code",
    () async {
      final messages = <String>[];
      final priorDebugPrint = debugPrint;
      debugPrint = (message, {wrapWidth}) {
        if (message != null) {
          messages.add(message);
        }
      };
      addTearDown(() => debugPrint = priorDebugPrint);
      final synchronization = RustLibrarySynchronization(
        startCall: () async => _snapshot(
          revision: 9,
          freshness: rust_change.CatalogFreshnessState.updating,
          lastIssueCode: "metadata_inventory_pending",
        ),
        pollCall: () async => _snapshot(revision: 9),
        stopCall: () async {},
        pollInterval: const Duration(days: 1),
        now: () => DateTime.utc(2026, 8, 21),
        enableDebugLogging: true,
      );

      await synchronization.start();

      final diagnostics = messages.join("\n");
      expect(diagnostics, contains("root_phase=queuePublication"));
      expect(diagnostics, contains("root_phase_elapsed_ms=0"));
      expect(diagnostics, contains("pending=0 retry=0 gaps=0"));
      expect(diagnostics, contains("code=metadata_inventory_pending"));
      await synchronization.stop();
    },
  );

  test("phase start is retained until the root phase changes", () async {
    var currentTime = DateTime.utc(2026, 8, 21, 10);
    final samePhaseReturned = Completer<void>();
    final releaseTransition = Completer<void>();
    final transitionReturned = Completer<void>();
    var pollCalls = 0;
    final synchronization = RustLibrarySynchronization(
      startCall: () async => _snapshot(
        revision: 9,
        freshness: rust_change.CatalogFreshnessState.updating,
      ),
      pollCall: () async {
        pollCalls += 1;
        if (pollCalls == 1) {
          samePhaseReturned.complete();
          return _snapshot(
            revision: 9,
            freshness: rust_change.CatalogFreshnessState.updating,
          );
        }
        await releaseTransition.future;
        if (!transitionReturned.isCompleted) {
          transitionReturned.complete();
        }
        return _snapshot(
          revision: 9,
          freshness: rust_change.CatalogFreshnessState.updating,
          phase: rust_sync.LibrarySynchronizationPhase.inventoryComparison,
        );
      },
      stopCall: () async {},
      pollInterval: const Duration(milliseconds: 5),
      now: () => currentTime,
      enableDebugLogging: false,
    );

    await synchronization.start();
    final initialPhaseStart = synchronization.current
        .statusFor("root-a")!
        .phaseStartedAt;
    currentTime = currentTime.add(const Duration(seconds: 5));
    await samePhaseReturned.future;
    await Future<void>.delayed(Duration.zero);

    expect(
      synchronization.current.statusFor("root-a")?.phaseStartedAt,
      initialPhaseStart,
    );

    currentTime = currentTime.add(const Duration(seconds: 3));
    releaseTransition.complete();
    await transitionReturned.future;
    await Future<void>.delayed(Duration.zero);

    final transitioned = synchronization.current.statusFor("root-a");
    expect(
      transitioned?.phase,
      LibrarySynchronizationPhase.inventoryComparison,
    );
    expect(transitioned?.phaseStartedAt, currentTime);
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
    phase: LibrarySynchronizationPhase.synchronized,
    phaseStartedAt: DateTime.utc(2026, 8, 21),
    sourceStatus: LibraryChangeSourceStatus.healthy,
    pendingChangeCount: BigInt.zero,
    retryWaitCount: BigInt.zero,
    freshnessUnknownCount: BigInt.zero,
  );
}

rust_sync.LibrarySynchronizationSnapshot _snapshot({
  required int revision,
  rust_change.CatalogFreshnessState freshness =
      rust_change.CatalogFreshnessState.synchronized,
  rust_sync.LibrarySynchronizationPhase? phase,
  String? lastIssueCode,
}) {
  return rust_sync.LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.from(revision),
    appliedMutationCount: 0,
    roots: [
      rust_sync.LibraryRootSynchronizationStatus(
        rootId: "root-a",
        rootGeneration: BigInt.one,
        availability: rust_domain.LibraryRootAvailability.available,
        freshness: freshness,
        freshnessCause:
            freshness == rust_change.CatalogFreshnessState.synchronized
            ? rust_change.CatalogFreshnessCause.noPendingChanges
            : rust_change.CatalogFreshnessCause.pendingChanges,
        phase:
            phase ??
            switch (freshness) {
              rust_change.CatalogFreshnessState.synchronized =>
                rust_sync.LibrarySynchronizationPhase.synchronized,
              rust_change.CatalogFreshnessState.updating =>
                rust_sync.LibrarySynchronizationPhase.queuePublication,
              rust_change.CatalogFreshnessState.needsReconciliation =>
                rust_sync.LibrarySynchronizationPhase.blocked,
              rust_change.CatalogFreshnessState.unavailable =>
                rust_sync.LibrarySynchronizationPhase.unavailable,
            },
        sourceHealth: rust_change.LibraryChangeSourceHealth.healthy,
        queueHealth: rust_queue.LibraryChangeQueueHealth.idle,
        pendingChangeCount: BigInt.zero,
        retryWaitCount: BigInt.zero,
        freshnessUnknownCount: BigInt.zero,
        lastIssueCode: lastIssueCode,
      ),
    ],
  );
}
