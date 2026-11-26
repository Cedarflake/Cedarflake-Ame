import "dart:async";

import "package:cedarflake_ame/app/window/ame_shutdown_coordinator.dart";
import "package:cedarflake_ame/app/window/ame_window_retirement.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "update joins work before activation and concurrent closure shares it",
    () async {
      final events = <String>[];
      final joined = Completer<void>();
      final started = Completer<void>();
      final shutdown = AmeShutdownCoordinator()
        ..register(() async {
          events.add("shutdown");
          started.complete();
          await joined.future;
        });
      final window = _window(shutdown, events);
      final update = window.update(_Update(events));
      await started.future;
      final close = window.close();
      expect(events, ["closing", "prepare", "shutdown"]);
      joined.complete();
      await Future.wait([update, close]);
      expect(events, [
        "closing",
        "prepare",
        "shutdown",
        "commit",
        "hide",
        "destroy",
      ]);
    },
  );

  test(
    "failed shutdown retires without authorizing package replacement",
    () async {
      final events = <String>[];
      final shutdown = AmeShutdownCoordinator()
        ..register(() => throw StateError("work not joined"));
      await expectLater(
        _window(shutdown, events).update(_Update(events)),
        throwsStateError,
      );
      expect(events, ["closing", "prepare", "hide", "destroy"]);
    },
  );

  test(
    "preparation failure preserves application and permits ordinary close",
    () async {
      final events = <String>[];
      final shutdown = AmeShutdownCoordinator()
        ..register(() => events.add("shutdown"));
      final window = _window(shutdown, events);
      await expectLater(
        window.update(_Update(events, fail: true)),
        throwsStateError,
      );
      expect(events, ["closing", "prepare", "retained"]);
      await window.close();
      expect(events.skip(3), ["closing", "hide", "shutdown", "destroy"]);
    },
  );
}

AmeWindowRetirement _window(
  AmeShutdownCoordinator shutdown,
  List<String> events,
) => AmeWindowRetirement(
  shutdown: shutdown,
  hide: () async => events.add("hide"),
  destroy: () async => events.add("destroy"),
  onClosing: () => events.add("closing"),
  onPreparationFailed: () => events.add("retained"),
);

class _Update implements AmeWindowUpdate {
  _Update(this.events, {this.fail = false});
  final List<String> events;
  final bool fail;
  @override
  Future<void> prepare() async {
    events.add("prepare");
    if (fail) throw StateError("helper unavailable");
  }

  @override
  Future<void> commit() async => events.add("commit");
}
