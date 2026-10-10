import "dart:async";

import "package:cedarflake_ame/app/bootstrap/library_synchronization_lifecycle_owner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/src/rust/domain/library_synchronization.dart"
    as rust_sync;
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  testWidgets(
    "the first frame pumps while start is pending and close stops immediately",
    (tester) async {
      final startEntered = Completer<void>();
      final startResult = Completer<rust_sync.LibrarySynchronizationSnapshot>();
      var stopCalls = 0;
      final synchronization = RustLibrarySynchronization.testing(
        startCall: () {
          startEntered.complete();
          return startResult.future;
        },
        pollCall: () => startResult.future,
        stopCall: () async => stopCalls += 1,
        pollInterval: const Duration(days: 1),
        enableDebugLogging: false,
      );
      final lifecycle = LibrarySynchronizationLifecycleOwner(synchronization);

      await tester.pumpWidget(const MaterialApp(home: Text("cached library")));
      lifecycle.startInBackground();
      await tester.pump();
      await startEntered.future;

      expect(find.text("cached library"), findsOneWidget);
      final closing = lifecycle.close();
      expect(stopCalls, 1);
      await closing;

      startResult.complete(_snapshot(revision: 41));
      expect(
        await lifecycle.startOperation,
        LibrarySynchronizationStartResult.skipped,
      );
      expect(synchronization.current.isRunning, isFalse);
    },
  );

  test("synchronous and asynchronous start errors are handled", () async {
    final reported = <Object>[];
    final synchronizations = <LibrarySynchronization>[
      _ThrowingLibrarySynchronization(
        startOperation: () => throw StateError("synchronous start failure"),
      ),
      _ThrowingLibrarySynchronization(
        startOperation: () =>
            Future.error(StateError("asynchronous start failure")),
      ),
    ];

    for (final synchronization in synchronizations) {
      final lifecycle = LibrarySynchronizationLifecycleOwner(
        synchronization,
        reportStartError: (error, _) => reported.add(error),
      );
      lifecycle.startInBackground();

      expect(
        await lifecycle.startOperation,
        LibrarySynchronizationStartResult.failed,
      );
      final firstClose = lifecycle.close();
      final secondClose = lifecycle.close();
      expect(identical(firstClose, secondClose), isTrue);
      await firstClose;
    }

    expect(reported, hasLength(2));
  });

  test(
    "a failed close can retry while a successful close remains shared",
    () async {
      final synchronization = _FailOnceDisposeLibrarySynchronization();
      final lifecycle = LibrarySynchronizationLifecycleOwner(synchronization);

      final firstClose = lifecycle.close();
      await expectLater(firstClose, throwsStateError);

      final secondClose = lifecycle.close();
      expect(identical(firstClose, secondClose), isFalse);
      await secondClose;
      final thirdClose = lifecycle.close();

      expect(identical(secondClose, thirdClose), isTrue);
      expect(synchronization.disposeCalls, 2);
      lifecycle.startInBackground();
      expect(lifecycle.startOperation, isNull);
    },
  );
}

class _ThrowingLibrarySynchronization implements LibrarySynchronization {
  _ThrowingLibrarySynchronization({required this.startOperation});

  final Future<LibrarySynchronizationStartResult> Function() startOperation;

  @override
  LibrarySynchronizationSnapshot get current =>
      LibrarySynchronizationSnapshot.stopped();

  @override
  Future<void> dispose() async {}

  @override
  Future<LibrarySynchronizationStartResult> start() => startOperation();

  @override
  Future<void> stop() async {}

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => const Stream.empty();
}

class _FailOnceDisposeLibrarySynchronization implements LibrarySynchronization {
  int disposeCalls = 0;

  @override
  LibrarySynchronizationSnapshot get current =>
      LibrarySynchronizationSnapshot.stopped();

  @override
  Future<void> dispose() async {
    disposeCalls += 1;
    if (disposeCalls == 1) {
      throw StateError("first dispose failed");
    }
  }

  @override
  Future<LibrarySynchronizationStartResult> start() async =>
      LibrarySynchronizationStartResult.started;

  @override
  Future<void> stop() async {}

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => const Stream.empty();
}

rust_sync.LibrarySynchronizationSnapshot _snapshot({required int revision}) {
  return rust_sync.LibrarySynchronizationSnapshot(
    isRunning: true,
    catalogRevision: BigInt.from(revision),
    appliedMutationCount: 0,
    roots: const [],
  );
}
