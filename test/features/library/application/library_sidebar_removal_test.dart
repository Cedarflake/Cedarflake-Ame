import "dart:async";

import "package:cedarflake_ame/features/library/application/library_sidebar_removal.dart";
import "package:flutter_test/flutter_test.dart";

const entries = [
  LibrarySidebarRemovalEntry("one", "First"),
  LibrarySidebarRemovalEntry("two", "Second"),
  LibrarySidebarRemovalEntry("three", "Third"),
];
const saved = LibrarySidebarRemovalOutcome(committed: true);

void main() {
  test("partial failure retries only uncommitted entries", () async {
    final calls = <String>[];
    var fail = true;
    final removal = LibrarySidebarRemoval(
      entries: entries,
      remove: (id) async {
        calls.add(id);
        if (id == "two" && fail) {
          return const LibrarySidebarRemovalOutcome(
            committed: false,
            error: "retry",
          );
        }
        return saved;
      },
    );
    addTearDown(removal.dispose);
    await removal.run();
    expect(removal.completed, {"one"});
    expect(removal.error, "retry");
    fail = false;
    await removal.run();
    expect(calls, ["one", "two", "two", "three"]);
    expect(removal.isComplete, isTrue);
  });

  test(
    "stop waits for current commit and keeps the remainder untouched",
    () async {
      final commit = Completer<LibrarySidebarRemovalOutcome>();
      final calls = <String>[];
      final removal = LibrarySidebarRemoval(
        entries: entries,
        remove: (id) {
          calls.add(id);
          return commit.future;
        },
      );
      addTearDown(removal.dispose);
      final running = removal.run();
      await removal.run();
      removal.stop();
      expect(removal.isRunning, isTrue);
      commit.complete(saved);
      await running;
      expect(calls, ["one"]);
      expect(removal.completed, {"one"});
      expect(removal.isRunning, isFalse);
    },
  );

  test(
    "disposal cannot start another removal or notify a retired dialog",
    () async {
      final commit = Completer<LibrarySidebarRemovalOutcome>();
      final calls = <String>[];
      var notifications = 0;
      final removal = LibrarySidebarRemoval(
        entries: entries,
        remove: (id) {
          calls.add(id);
          return commit.future;
        },
      )..addListener(() => notifications++);
      final running = removal.run();
      removal.dispose();
      commit.complete(saved);
      await running;
      expect(calls, ["one"]);
      expect(notifications, 1);
    },
  );

  test("committed removal with a publication error is not replayed", () async {
    final calls = <String>[];
    final removal = LibrarySidebarRemoval(
      entries: entries,
      remove: (id) async {
        calls.add(id);
        return LibrarySidebarRemovalOutcome(
          committed: true,
          error: id == "one" ? "refresh" : null,
        );
      },
    );
    addTearDown(removal.dispose);
    await removal.run();
    expect(removal.completed, {"one"});
    expect(calls, ["one"]);
    await removal.run();
    expect(calls, ["one", "two", "three"]);
  });
}
