import "dart:async";

import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization_policy.g.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/src/rust/domain.dart" as rust_domain;
import "package:cedarflake_ame/src/rust/domain/library_change.dart"
    as rust_change;
import "package:cedarflake_ame/src/rust/domain/library_change_queue.dart"
    as rust_queue;
import "package:cedarflake_ame/src/rust/domain/library_synchronization.dart"
    as rust_sync;
import "package:cedarflake_ame/src/rust/domain/persistent_journal.dart"
    as rust_journal;
import "package:flutter/foundation.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "a successful start schedules the generated production poll cadence",
    () async {
      Duration? scheduledInterval;
      var periodicTimerCount = 0;

      await runZoned(
        () async {
          final synchronization = RustLibrarySynchronization.testing(
            startCall: () async => _snapshot(revision: 1),
            pollCall: () async => _snapshot(revision: 1),
            stopCall: () async {},
            enableDebugLogging: false,
          );
          try {
            expect(
              await synchronization.start(),
              LibrarySynchronizationStartResult.started,
            );
            expect(periodicTimerCount, 1);
            expect(
              scheduledInterval,
              productionLibrarySynchronizationPollInterval,
            );
          } finally {
            await synchronization.stop();
          }
        },
        zoneSpecification: ZoneSpecification(
          createPeriodicTimer: (self, parent, zone, duration, callback) {
            periodicTimerCount += 1;
            scheduledInterval = duration;
            return _ControlledPeriodicTimer();
          },
        ),
      );
    },
  );

  test(
    "testing cadence injection schedules the explicit fast interval",
    () async {
      const fastInterval = Duration(milliseconds: 3);
      Duration? scheduledInterval;

      await runZoned(
        () async {
          final synchronization = RustLibrarySynchronization.testing(
            startCall: () async => _snapshot(revision: 1),
            pollCall: () async => _snapshot(revision: 1),
            stopCall: () async {},
            pollInterval: fastInterval,
            enableDebugLogging: false,
          );
          try {
            expect(
              await synchronization.start(),
              LibrarySynchronizationStartResult.started,
            );
            expect(scheduledInterval, fastInterval);
          } finally {
            await synchronization.stop();
          }
        },
        zoneSpecification: ZoneSpecification(
          createPeriodicTimer: (self, parent, zone, duration, callback) {
            scheduledInterval = duration;
            return _ControlledPeriodicTimer();
          },
        ),
      );
    },
  );

  test("a failed native start retries through start before polling", () async {
    final secondStartEntered = Completer<void>();
    final releaseSecondStart = Completer<void>();
    var startCalls = 0;
    var pollCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async {
        startCalls += 1;
        if (startCalls == 1) {
          throw const rust_domain.ScanError(
            code: "catalog_database_busy",
            message: "the catalog is temporarily busy",
          );
        }
        secondStartEntered.complete();
        await releaseSecondStart.future;
        return _snapshot(revision: 2);
      },
      pollCall: () async {
        pollCalls += 1;
        return _snapshot(revision: 2);
      },
      stopCall: () async {},
      pollInterval: const Duration(milliseconds: 1),
      startRetryDelays: const [Duration.zero],
      enableDebugLogging: false,
    );
    addTearDown(() async {
      if (!releaseSecondStart.isCompleted) {
        releaseSecondStart.complete();
      }
      await synchronization.stop();
    });

    final starting = synchronization.start();
    await secondStartEntered.future.timeout(const Duration(seconds: 1));

    expect(startCalls, 2);
    expect(pollCalls, 0);
    releaseSecondStart.complete();
    expect(await starting, LibrarySynchronizationStartResult.started);
  });

  test("an exhausted native start never enables polling", () async {
    var startCalls = 0;
    var pollCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async {
        startCalls += 1;
        throw const rust_domain.ScanError(
          code: "catalog_database_locked",
          message: "the catalog is temporarily locked",
        );
      },
      pollCall: () async {
        pollCalls += 1;
        return _snapshot(revision: 3);
      },
      stopCall: () async {},
      pollInterval: const Duration(milliseconds: 1),
      startRetryDelays: const [Duration.zero, Duration.zero],
      enableDebugLogging: false,
    );
    addTearDown(synchronization.stop);

    final result = await synchronization.start();
    await Future<void>.delayed(const Duration(milliseconds: 20));

    expect(result, LibrarySynchronizationStartResult.failed);
    expect(startCalls, 3);
    expect(pollCalls, 0);
  });

  test("a non-transient native start failure is attempted once", () async {
    var startCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async {
        startCalls += 1;
        throw const rust_domain.ScanError(
          code: "library_synchronization_owner_panicked",
          message: "the native owner failed closed",
        );
      },
      pollCall: () async => _snapshot(revision: 3),
      stopCall: () async {},
      startRetryDelays: const [Duration.zero, Duration.zero, Duration.zero],
      enableDebugLogging: false,
    );
    addTearDown(synchronization.stop);

    expect(
      await synchronization.start(),
      LibrarySynchronizationStartResult.failed,
    );

    expect(startCalls, 1);
  });

  test("concurrent start callers share one native attempt", () async {
    final startEntered = Completer<void>();
    final releaseStart = Completer<void>();
    var startCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async {
        startCalls += 1;
        startEntered.complete();
        await releaseStart.future;
        return _snapshot(revision: 4);
      },
      pollCall: () async => _snapshot(revision: 4),
      stopCall: () async {},
      pollInterval: const Duration(days: 1),
      enableDebugLogging: false,
    );
    addTearDown(synchronization.stop);

    final first = synchronization.start();
    await startEntered.future;
    final second = synchronization.start();

    expect(startCalls, 1);
    releaseStart.complete();
    expect(
      await Future.wait([first, second]),
      everyElement(LibrarySynchronizationStartResult.started),
    );
    expect(startCalls, 1);
  });

  test(
    "start retries share one ticket and stop fences before async drain",
    () async {
      final stopRelease = Completer<void>();
      final stopOrder = <String>[];
      var startTicketReservations = 0;
      var stopFenceReservations = 0;
      var startCalls = 0;
      var stopCalls = 0;
      final synchronization = RustLibrarySynchronization.testing(
        reserveStartTicketCall: () {
          startTicketReservations += 1;
          return BigInt.from(11);
        },
        reserveStopFenceCall: () {
          stopFenceReservations += 1;
          stopOrder.add("fence");
          return BigInt.from(12);
        },
        startCall: () async {
          startCalls += 1;
          if (startCalls == 1) {
            throw const rust_domain.ScanError(
              code: "catalog_database_busy",
              message: "the catalog is temporarily busy",
            );
          }
          return _snapshot(revision: 4);
        },
        pollCall: () async => _snapshot(revision: 4),
        stopCall: () async {
          stopCalls += 1;
          stopOrder.add("drain");
          await stopRelease.future;
        },
        pollInterval: const Duration(days: 1),
        startRetryDelays: const [Duration.zero],
        enableDebugLogging: false,
      );
      addTearDown(() {
        if (!stopRelease.isCompleted) {
          stopRelease.complete();
        }
      });

      expect(
        await synchronization.start(),
        LibrarySynchronizationStartResult.started,
      );
      expect(startTicketReservations, 1);
      expect(startCalls, 2);

      final firstStop = synchronization.stop();
      final secondStop = synchronization.stop();
      expect(identical(firstStop, secondStop), isTrue);
      expect(stopFenceReservations, 1);
      expect(stopCalls, 1);
      expect(stopOrder, ["fence", "drain"]);

      stopRelease.complete();
      await firstStop;
    },
  );

  test(
    "stop and dispose cancel a failed-start retry and stop native once",
    () async {
      final firstStartFailed = Completer<void>();
      final releaseStop = Completer<void>();
      var startCalls = 0;
      var pollCalls = 0;
      var stopCalls = 0;
      final synchronization = RustLibrarySynchronization.testing(
        startCall: () async {
          startCalls += 1;
          firstStartFailed.complete();
          throw const rust_domain.ScanError(
            code: "catalog_database_busy",
            message: "the catalog is temporarily busy",
          );
        },
        pollCall: () async {
          pollCalls += 1;
          return _snapshot(revision: 5);
        },
        stopCall: () async {
          stopCalls += 1;
          await releaseStop.future;
        },
        pollInterval: const Duration(milliseconds: 1),
        startRetryDelays: const [Duration(days: 1)],
        enableDebugLogging: false,
      );

      final starting = synchronization.start();
      await firstStartFailed.future;
      await Future<void>.delayed(Duration.zero);
      final stopping = synchronization.stop();
      final disposing = synchronization.dispose();
      await Future<void>.delayed(Duration.zero);

      expect(startCalls, 1);
      expect(pollCalls, 0);
      expect(stopCalls, 1);
      releaseStop.complete();
      await Future.wait([stopping, disposing]);
      expect(await starting, LibrarySynchronizationStartResult.skipped);
      expect(startCalls, 1);
      expect(pollCalls, 0);
      expect(stopCalls, 1);
    },
  );

  test(
    "an unstarted controller drains the global owner admitted by its fence",
    () async {
      final nativeLifecycle = _FakeGlobalLifecycleRegistry();
      final owner = RustLibrarySynchronization.testing(
        reserveStartTicketCall: nativeLifecycle.reserveTicket,
        reserveStopFenceCall: nativeLifecycle.reserveStopFence,
        startCall: nativeLifecycle.start,
        pollCall: () async => _snapshot(revision: 5),
        stopCall: nativeLifecycle.drain,
        pollInterval: const Duration(days: 1),
        enableDebugLogging: false,
      );
      final unstartedController = RustLibrarySynchronization.testing(
        reserveStartTicketCall: nativeLifecycle.reserveTicket,
        reserveStopFenceCall: nativeLifecycle.reserveStopFence,
        startCall: nativeLifecycle.start,
        pollCall: () async => _snapshot(revision: 5),
        stopCall: nativeLifecycle.drain,
        pollInterval: const Duration(days: 1),
        enableDebugLogging: false,
      );

      expect(await owner.start(), LibrarySynchronizationStartResult.started);
      expect(nativeLifecycle.hasOwner, isTrue);

      try {
        await unstartedController.stop();

        expect(nativeLifecycle.drainCalls, 1);
        expect(nativeLifecycle.hasOwner, isFalse);
        expect(nativeLifecycle.events, ["start", "fence", "drain"]);
      } finally {
        await owner.stop();
      }
    },
  );

  test("native not-started stop converges and permits a later epoch", () async {
    var startCalls = 0;
    var stopCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async {
        startCalls += 1;
        if (startCalls == 1) {
          throw const rust_domain.ScanError(
            code: "library_synchronization_owner_panicked",
            message: "the constructor recovered to empty",
          );
        }
        return _snapshot(revision: 6);
      },
      pollCall: () async => _snapshot(revision: 6),
      stopCall: () async {
        stopCalls += 1;
        if (stopCalls == 1) {
          throw const rust_domain.ScanError(
            code: "library_synchronization_not_started",
            message: "the native registry is empty",
          );
        }
      },
      pollInterval: const Duration(days: 1),
      startRetryDelays: const [],
      enableDebugLogging: false,
    );

    expect(
      await synchronization.start(),
      LibrarySynchronizationStartResult.failed,
    );
    await synchronization.stop();
    await synchronization.stop();
    expect(stopCalls, 2);

    expect(
      await synchronization.start(),
      LibrarySynchronizationStartResult.started,
    );
    await synchronization.stop();
    expect(startCalls, 2);
    expect(stopCalls, 3);
    expect(synchronization.current.isRunning, isFalse);
  });

  test(
    "late old start success or failure cannot overwrite a replacement epoch",
    () async {
      for (final lateFailure in [false, true]) {
        final oldStartEntered = Completer<void>();
        final oldStart = Completer<rust_sync.LibrarySynchronizationSnapshot>();
        var startCalls = 0;
        var stopCalls = 0;
        final synchronization = RustLibrarySynchronization.testing(
          startCall: () {
            startCalls += 1;
            if (startCalls == 1) {
              oldStartEntered.complete();
              return oldStart.future;
            }
            return Future.value(_snapshot(revision: 20));
          },
          pollCall: () async => _snapshot(revision: 20),
          stopCall: () async => stopCalls += 1,
          pollInterval: const Duration(days: 1),
          startRetryDelays: const [],
          enableDebugLogging: false,
        );

        final first = synchronization.start();
        await oldStartEntered.future;
        await synchronization.stop();
        expect(
          await synchronization.start(),
          LibrarySynchronizationStartResult.started,
        );
        if (lateFailure) {
          oldStart.completeError(
            const rust_domain.ScanError(
              code: "library_synchronization_owner_panicked",
              message: "late old failure",
            ),
          );
        } else {
          oldStart.complete(_snapshot(revision: 999));
        }

        expect(await first, LibrarySynchronizationStartResult.skipped);
        await Future<void>.delayed(Duration.zero);
        expect(synchronization.current.catalogRevision, BigInt.from(20));
        expect(startCalls, 2);
        await synchronization.stop();
        expect(stopCalls, 2);
      }
    },
  );

  test(
    "maps root freshness and preserves the latest catalog revision",
    () async {
      final synchronization = RustLibrarySynchronization.testing(
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

  test("maps continuity independently from healthy live updates", () async {
    const expected = {
      rust_journal.PersistentJournalContinuityState.baselineRequired:
          LibraryContinuityState.baselineRequired,
      rust_journal.PersistentJournalContinuityState.catchingUp:
          LibraryContinuityState.catchingUp,
      rust_journal.PersistentJournalContinuityState.current:
          LibraryContinuityState.current,
      rust_journal.PersistentJournalContinuityState.recoveryRequired:
          LibraryContinuityState.recoveryRequired,
      rust_journal.PersistentJournalContinuityState.liveOnly:
          LibraryContinuityState.liveOnly,
      rust_journal.PersistentJournalContinuityState.unavailable:
          LibraryContinuityState.unavailable,
    };

    for (final entry in expected.entries) {
      final synchronization = RustLibrarySynchronization.testing(
        startCall: () async => _snapshot(revision: 7, continuity: entry.key),
        pollCall: () async => _snapshot(revision: 7, continuity: entry.key),
        stopCall: () async {},
        pollInterval: const Duration(days: 1),
      );

      await synchronization.start();
      final root = synchronization.current.statusFor("root-a");
      expect(root?.continuity, entry.value);
      expect(root?.freshness, LibraryCatalogFreshness.synchronized);
      expect(root?.sourceStatus, LibraryChangeSourceStatus.healthy);
      await synchronization.stop();
    }
  });

  test("preserves an explicit recovery block from the Rust snapshot", () async {
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async => _snapshot(
        revision: 7,
        freshness: rust_change.CatalogFreshnessState.needsReconciliation,
        continuity:
            rust_journal.PersistentJournalContinuityState.recoveryRequired,
        phase: rust_sync.LibrarySynchronizationPhase.blocked,
        recoveryBlocked: true,
        lastIssueCode: "live_gap_v30_explicit_recovery_required",
      ),
      pollCall: () async => _snapshot(revision: 7),
      stopCall: () async {},
      pollInterval: const Duration(days: 1),
    );

    await synchronization.start();

    final root = synchronization.current.statusFor("root-a");
    expect(root?.freshness, LibraryCatalogFreshness.needsReconciliation);
    expect(root?.phase, LibrarySynchronizationPhase.blocked);
    expect(root?.recoveryBlocked, isTrue);
    expect(root?.lastIssueCode, "live_gap_v30_explicit_recovery_required");
    await synchronization.stop();
  });

  test(
    "timer polling never overlaps and stop invalidates the active poll",
    () async {
      final pollStarted = Completer<void>();
      final releasePoll = Completer<void>();
      var activePolls = 0;
      var maximumActivePolls = 0;
      var pollCalls = 0;
      var stopCalls = 0;
      final synchronization = RustLibrarySynchronization.testing(
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
      expect(stopCalls, 1);
      await stopping;
      expect(synchronization.current.catalogRevision, BigInt.from(1));

      releasePoll.complete();
      await Future<void>.delayed(Duration.zero);

      expect(pollCalls, 1);
      expect(maximumActivePolls, 1);
      expect(stopCalls, 1);
      expect(synchronization.current.catalogRevision, BigInt.from(1));

      await synchronization.stop();
      expect(stopCalls, 2);
    },
  );

  test("a never-completing poll cannot delay native stop", () async {
    final pollStarted = Completer<void>();
    final neverCompletes =
        Completer<rust_sync.LibrarySynchronizationSnapshot>();
    var stopCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async => _snapshot(revision: 20),
      pollCall: () {
        pollStarted.complete();
        return neverCompletes.future;
      },
      stopCall: () async => stopCalls += 1,
      pollInterval: const Duration(milliseconds: 1),
    );

    await synchronization.start();
    await pollStarted.future;
    await synchronization.stop().timeout(const Duration(seconds: 1));

    expect(stopCalls, 1);
    expect(synchronization.current.isRunning, isFalse);
  });

  test("a late old poll cannot publish into a restarted epoch", () async {
    final oldPollStarted = Completer<void>();
    final oldPoll = Completer<rust_sync.LibrarySynchronizationSnapshot>();
    var starts = 0;
    var polls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async => _snapshot(revision: ++starts * 10),
      pollCall: () {
        polls += 1;
        if (polls == 1) {
          oldPollStarted.complete();
          return oldPoll.future;
        }
        return Future.value(_snapshot(revision: starts * 10));
      },
      stopCall: () async {},
      pollInterval: const Duration(milliseconds: 1),
    );

    await synchronization.start();
    await oldPollStarted.future;
    await synchronization.stop();
    await synchronization.start();
    expect(synchronization.current.catalogRevision, BigInt.from(20));

    oldPoll.complete(_snapshot(revision: 999));
    await Future<void>.delayed(Duration.zero);

    expect(synchronization.current.catalogRevision, BigInt.from(20));
    await synchronization.stop();
  });

  test("failed native stop does not publish a stopped snapshot", () async {
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async => _snapshot(revision: 11),
      pollCall: () async => _snapshot(revision: 11),
      stopCall: () async => throw const rust_domain.ScanError(
        code: "library_synchronization_stop_timeout",
        message: "worker is still draining",
      ),
      pollInterval: const Duration(days: 1),
    );

    await synchronization.start();
    await expectLater(
      synchronization.stop(),
      throwsA(
        isA<rust_domain.ScanError>().having(
          (error) => error.code,
          "code",
          "library_synchronization_stop_timeout",
        ),
      ),
    );

    expect(synchronization.current.isRunning, isTrue);
    expect(synchronization.current.catalogRevision, BigInt.from(11));
  });

  test(
    "successful stop clears lifecycle state and permits immediate restart",
    () async {
      var starts = 0;
      var stops = 0;
      final synchronization = RustLibrarySynchronization.testing(
        startCall: () async => _snapshot(revision: ++starts),
        pollCall: () async => _snapshot(revision: starts),
        stopCall: () async => stops += 1,
        pollInterval: const Duration(days: 1),
      );

      await synchronization.start();
      await synchronization.stop();
      await synchronization.start();
      await synchronization.stop();

      expect(starts, 2);
      expect(stops, 2);
      expect(synchronization.current.isRunning, isFalse);
    },
  );

  test("failed stop can retry the native draining epoch", () async {
    var stopCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async => _snapshot(revision: 12),
      pollCall: () async => _snapshot(revision: 12),
      stopCall: () async {
        stopCalls += 1;
        if (stopCalls == 1) {
          throw const rust_domain.ScanError(
            code: "library_synchronization_stop_timeout",
            message: "worker is still draining",
          );
        }
      },
      pollInterval: const Duration(days: 1),
    );

    await synchronization.start();
    await expectLater(
      synchronization.stop(),
      throwsA(isA<rust_domain.ScanError>()),
    );
    expect(synchronization.current.isRunning, isTrue);

    await synchronization.stop();

    expect(stopCalls, 2);
    expect(synchronization.current.isRunning, isFalse);
  });

  test("concurrent stop callers share one native operation", () async {
    final releaseStop = Completer<void>();
    var stopCalls = 0;
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async => _snapshot(revision: 13),
      pollCall: () async => _snapshot(revision: 13),
      stopCall: () async {
        stopCalls += 1;
        await releaseStop.future;
      },
      pollInterval: const Duration(days: 1),
    );
    await synchronization.start();

    final first = synchronization.stop();
    final second = synchronization.stop();
    await Future<void>.delayed(Duration.zero);
    expect(stopCalls, 1);
    releaseStop.complete();
    await Future.wait([first, second]);

    expect(stopCalls, 1);
    expect(synchronization.current.isRunning, isFalse);
  });

  test(
    "start during stop is ignored until the stop operation settles",
    () async {
      final releaseStop = Completer<void>();
      var starts = 0;
      final synchronization = RustLibrarySynchronization.testing(
        startCall: () async => _snapshot(revision: ++starts),
        pollCall: () async => _snapshot(revision: starts),
        stopCall: () => releaseStop.future,
        pollInterval: const Duration(days: 1),
      );
      await synchronization.start();

      final stopping = synchronization.stop();
      await synchronization.start();
      expect(starts, 1);
      releaseStop.complete();
      await stopping;

      await synchronization.start();
      expect(starts, 2);
      await synchronization.stop();
    },
  );

  test(
    "dispose shares an active stop and permanently rejects restart",
    () async {
      final releaseStop = Completer<void>();
      var starts = 0;
      var stopCalls = 0;
      final synchronization = RustLibrarySynchronization.testing(
        startCall: () async => _snapshot(revision: ++starts),
        pollCall: () async => _snapshot(revision: starts),
        stopCall: () async {
          stopCalls += 1;
          await releaseStop.future;
        },
        pollInterval: const Duration(days: 1),
      );
      await synchronization.start();
      final streamDone = synchronization.watch().drain<void>();

      final stopping = synchronization.stop();
      final disposing = synchronization.dispose();
      await synchronization.start();
      expect(starts, 1);
      expect(stopCalls, 1);
      expect(synchronization.current.isRunning, isTrue);

      releaseStop.complete();
      await Future.wait([stopping, disposing]);
      await streamDone;
      expect(stopCalls, 1);
      expect(synchronization.current.isRunning, isFalse);

      await synchronization.start();
      expect(starts, 1);
    },
  );

  test("poll failure degrades known roots without losing revision", () async {
    final synchronization = RustLibrarySynchronization.testing(
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
    final synchronization = RustLibrarySynchronization.testing(
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
      final synchronization = RustLibrarySynchronization.testing(
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
      final synchronization = RustLibrarySynchronization.testing(
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
      final synchronization = RustLibrarySynchronization.testing(
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

  test("blocked state does not cross a root generation boundary", () async {
    final nextGenerationReturned = Completer<void>();
    final synchronization = RustLibrarySynchronization.testing(
      startCall: () async => _snapshot(
        revision: 9,
        freshness: rust_change.CatalogFreshnessState.needsReconciliation,
        lastIssueCode: "catalog_database_error",
      ),
      pollCall: () async {
        if (!nextGenerationReturned.isCompleted) {
          nextGenerationReturned.complete();
        }
        return _snapshot(
          revision: 10,
          rootGeneration: BigInt.two,
          freshness: rust_change.CatalogFreshnessState.updating,
        );
      },
      stopCall: () async {},
      pollInterval: const Duration(milliseconds: 5),
      enableDebugLogging: false,
    );

    await synchronization.start();
    await nextGenerationReturned.future;
    await Future<void>.delayed(Duration.zero);

    final status = synchronization.current.statusFor("root-a");
    expect(status?.rootGeneration, BigInt.two);
    expect(status?.freshness, LibraryCatalogFreshness.updating);
    expect(status?.phase, LibrarySynchronizationPhase.queuePublication);
    expect(status?.lastIssueCode, isNull);
    await synchronization.stop();
  });

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
      final synchronization = RustLibrarySynchronization.testing(
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
    final synchronization = RustLibrarySynchronization.testing(
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

class _ControlledPeriodicTimer implements Timer {
  bool _isActive = true;

  @override
  bool get isActive => _isActive;

  @override
  int get tick => 0;

  @override
  void cancel() {
    _isActive = false;
  }
}

LibraryRootSynchronizationStatus _rootStatus(String rootId) {
  return LibraryRootSynchronizationStatus(
    rootId: rootId,
    rootGeneration: BigInt.one,
    availability: LibraryRootAvailability.available,
    freshness: LibraryCatalogFreshness.synchronized,
    freshnessCause: LibraryCatalogFreshnessCause.noPendingChanges,
    continuity: LibraryContinuityState.current,
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
  BigInt? rootGeneration,
  rust_change.CatalogFreshnessState freshness =
      rust_change.CatalogFreshnessState.synchronized,
  rust_journal.PersistentJournalContinuityState continuity =
      rust_journal.PersistentJournalContinuityState.current,
  rust_sync.LibrarySynchronizationPhase? phase,
  bool recoveryBlocked = false,
  String? lastIssueCode,
}) {
  return rust_sync.LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.from(revision),
    appliedMutationCount: 0,
    roots: [
      rust_sync.LibraryRootSynchronizationStatus(
        rootId: "root-a",
        rootGeneration: rootGeneration ?? BigInt.one,
        availability: rust_domain.LibraryRootAvailability.available,
        freshness: freshness,
        freshnessCause:
            freshness == rust_change.CatalogFreshnessState.synchronized
            ? rust_change.CatalogFreshnessCause.noPendingChanges
            : rust_change.CatalogFreshnessCause.pendingChanges,
        continuity: continuity,
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
        recoveryBlocked: recoveryBlocked,
        lastIssueCode: lastIssueCode,
      ),
    ],
  );
}

class _FakeGlobalLifecycleRegistry {
  BigInt _nextTicket = BigInt.zero;
  bool _hasOwner = false;
  bool _hasAdmittedFence = false;
  int drainCalls = 0;
  final events = <String>[];

  bool get hasOwner => _hasOwner;

  BigInt reserveTicket() {
    _nextTicket += BigInt.one;
    return _nextTicket;
  }

  BigInt reserveStopFence() {
    final fence = reserveTicket();
    _hasAdmittedFence = true;
    events.add("fence");
    return fence;
  }

  Future<rust_sync.LibrarySynchronizationSnapshot> start() async {
    _hasOwner = true;
    events.add("start");
    return _snapshot(revision: 5);
  }

  Future<void> drain() async {
    if (!_hasAdmittedFence) {
      throw StateError("native drain requires an admitted stop fence");
    }
    _hasAdmittedFence = false;
    if (!_hasOwner) {
      throw const rust_domain.ScanError(
        code: "library_synchronization_not_started",
        message: "the native registry is empty",
      );
    }
    _hasOwner = false;
    drainCalls += 1;
    events.add("drain");
  }
}
