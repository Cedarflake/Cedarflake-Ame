import "dart:async";

import "package:cedarflake_ame/app/bootstrap/ame_startup_orchestrator.dart";
import "package:cedarflake_ame/app/bootstrap/library_synchronization_lifecycle_owner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("mounts then starts background work before waiting to reveal", () async {
    final events = <String>[];
    final reveal = Completer<void>();
    var didComplete = false;

    final startup = mountAmeApplicationAndRevealWindow(
      mountApplication: () => events.add("mount"),
      startInBackground: () => events.add("start"),
      revealWindowAfterFirstFrame: () {
        events.add("reveal");
        return reveal.future;
      },
    ).whenComplete(() => didComplete = true);

    expect(events, ["mount", "start", "reveal"]);
    expect(didComplete, isFalse);

    reveal.complete();
    await startup;
    expect(didComplete, isTrue);
  });

  test(
    "does not await a background start operation before revealing",
    () async {
      final events = <String>[];
      final synchronization = _PendingLibrarySynchronization();
      final lifecycle = LibrarySynchronizationLifecycleOwner(synchronization);

      final startup = mountAmeApplicationAndRevealWindow(
        mountApplication: () => events.add("mount"),
        startInBackground: () {
          events.add("start");
          lifecycle.startInBackground();
        },
        revealWindowAfterFirstFrame: () async => events.add("reveal"),
      );

      expect(events, ["mount", "start", "reveal"]);
      await startup.timeout(const Duration(seconds: 1));
      expect(synchronization.startCalls, 1);
      expect(synchronization.startCompletion.isCompleted, isFalse);
      await lifecycle.close();
    },
  );
}

class _PendingLibrarySynchronization implements LibrarySynchronization {
  final startCompletion = Completer<LibrarySynchronizationStartResult>();
  int startCalls = 0;

  @override
  LibrarySynchronizationSnapshot get current =>
      LibrarySynchronizationSnapshot.stopped();

  @override
  Future<void> dispose() async {}

  @override
  Future<LibrarySynchronizationStartResult> start() {
    startCalls += 1;
    return startCompletion.future;
  }

  @override
  Future<void> stop() async {}

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => const Stream.empty();
}
