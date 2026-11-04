import "package:cedarflake_ame/features/library/adapters/rust/rust_library_synchronization_gateway.dart";
import "package:cedarflake_ame/features/library/application/ports/library_synchronization_gateway.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust;
import "package:cedarflake_ame/src/rust/domain/library_change.dart" as change;
import "package:cedarflake_ame/src/rust/domain/library_change_queue.dart"
    as queue;
import "package:cedarflake_ame/src/rust/domain/library_synchronization.dart"
    as sync;
import "package:cedarflake_ame/src/rust/domain/persistent_journal.dart"
    as journal;
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "native tickets and status evidence cross the typed gateway intact",
    () async {
      final calls = <(String, BigInt)>[];
      final observedAt = DateTime.utc(2026, 11, 3);
      final gateway = RustLibrarySynchronizationGateway.testing(
        reserveStartTicket: () => BigInt.from(11),
        reserveStopFence: () => BigInt.from(12),
        startCall: (ticket) async {
          calls.add(("start", ticket));
          return _snapshot();
        },
        pollCall: (ticket) async {
          calls.add(("poll", ticket));
          return _snapshot();
        },
        stopCall: (fence) async => calls.add(("stop", fence)),
        now: () => observedAt,
      );
      final ticket = gateway.reserveStartTicket();
      final started = await gateway.start(ticket);
      final polled = await gateway.poll(ticket);
      await gateway.stop(gateway.reserveStopFence());
      expect(calls, [
        ("start", BigInt.from(11)),
        ("poll", BigInt.from(11)),
        ("stop", BigInt.from(12)),
      ]);
      expect(started, polled);
      expect(started.isRunning, isTrue);
      expect(started.catalogRevision, BigInt.from(29));
      expect(started.appliedMutationCount, 3);
      final root = started.statusFor("root");
      expect(
        root,
        LibraryRootSynchronizationStatus(
          rootId: "root",
          rootGeneration: BigInt.two,
          availability: LibraryRootAvailability.available,
          freshness: LibraryCatalogFreshness.updating,
          freshnessCause: LibraryCatalogFreshnessCause.pendingChanges,
          continuity: LibraryContinuityState.catchingUp,
          phase: LibrarySynchronizationPhase.fullScan,
          phaseStartedAt: observedAt,
          sourceStatus: LibraryChangeSourceStatus.healthy,
          pendingChangeCount: BigInt.from(5),
          retryWaitCount: BigInt.two,
          freshnessUnknownCount: BigInt.one,
          recoveryBlocked: true,
          lastIssueCode: "retained_retry_failure",
        ),
      );
    },
  );

  test(
    "native and bridge failures retain Ame codes through every operation",
    () async {
      for (final error in <Object>[
        const rust.ScanError(
          code: "catalog_database_busy",
          message: "catalog busy",
        ),
        StateError("bridge unavailable"),
      ]) {
        final gateway = RustLibrarySynchronizationGateway.testing(
          reserveStartTicket: () => throw error,
          reserveStopFence: () => throw error,
          startCall: (_) async => throw error,
          pollCall: (_) => throw error,
          stopCall: (_) async => throw error,
        );
        final matcher = isA<LibrarySynchronizationFailure>()
            .having(
              (failure) => failure.code,
              "code",
              error is rust.ScanError
                  ? error.code
                  : "library_synchronization_poll_failed",
            )
            .having(
              (failure) => failure.message,
              "message",
              error is rust.ScanError ? error.message : error.toString(),
            );
        expect(gateway.reserveStartTicket, throwsA(matcher));
        expect(gateway.reserveStopFence, throwsA(matcher));
        await expectLater(gateway.start(BigInt.one), throwsA(matcher));
        await expectLater(gateway.poll(BigInt.one), throwsA(matcher));
        await expectLater(gateway.stop(BigInt.two), throwsA(matcher));
      }
    },
  );
}

sync.LibrarySynchronizationSnapshot _snapshot() =>
    sync.LibrarySynchronizationSnapshot(
      isRunning: true,
      catalogRevision: BigInt.from(29),
      appliedMutationCount: 3,
      roots: [
        sync.LibraryRootSynchronizationStatus(
          rootId: "root",
          rootGeneration: BigInt.two,
          availability: rust.LibraryRootAvailability.available,
          freshness: change.CatalogFreshnessState.updating,
          freshnessCause: change.CatalogFreshnessCause.pendingChanges,
          continuity: journal.PersistentJournalContinuityState.catchingUp,
          phase: sync.LibrarySynchronizationPhase.fullScan,
          sourceHealth: change.LibraryChangeSourceHealth.healthy,
          queueHealth: queue.LibraryChangeQueueHealth.degraded,
          pendingChangeCount: BigInt.from(5),
          retryWaitCount: BigInt.two,
          freshnessUnknownCount: BigInt.one,
          recoveryBlocked: true,
          lastIssueCode: "retained_retry_failure",
        ),
      ],
    );
