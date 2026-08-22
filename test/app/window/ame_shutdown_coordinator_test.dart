import "dart:async";

import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("shutdown actions run once in reverse registration order", () async {
    final coordinator = AmeShutdownCoordinator();
    final events = <String>[];
    coordinator.register(() async => events.add("first"));
    coordinator.register(() async => events.add("second"));

    await Future.wait([coordinator.shutdown(), coordinator.shutdown()]);

    expect(events, ["second", "first"]);
    expect(() => coordinator.register(() {}), throwsStateError);
  });

  test("shutdown waits for an asynchronous watcher stop", () async {
    final coordinator = AmeShutdownCoordinator();
    final completion = Completer<void>();
    var didFinish = false;
    coordinator.register(() async {
      await completion.future;
      didFinish = true;
    });

    final shutdown = coordinator.shutdown();
    await Future<void>.delayed(Duration.zero);
    expect(didFinish, isFalse);

    completion.complete();
    await shutdown;
    expect(didFinish, isTrue);
  });

  test("shutdown runs remaining actions after one action fails", () async {
    final coordinator = AmeShutdownCoordinator();
    final events = <String>[];
    coordinator.register(() async => events.add("synchronization"));
    coordinator.register(() {
      events.add("scan");
      throw StateError("checkpoint failed");
    });

    await expectLater(coordinator.shutdown(), throwsStateError);

    expect(events, ["scan", "synchronization"]);
  });
}
