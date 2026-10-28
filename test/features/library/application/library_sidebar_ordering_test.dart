import "dart:async";

import "package:cedarflake_ame/features/library/application/library_sidebar_order.dart";
import "package:cedarflake_ame/features/library/application/library_sidebar_ordering.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "moves serialize against current identities and survive metadata refresh",
    () async {
      final writes = <LibrarySidebarMove>[];
      final save = Completer<void>();
      final ordering = LibrarySidebarOrdering(
        ids: ["a", "b", "c"],
        save: (move) {
          writes.add(move);
          return save.future;
        },
      );
      addTearDown(ordering.dispose);
      final first = ordering.move(0, 2);
      expect(ordering.ids, ["b", "c", "a"]);
      expect(ordering.saving, isTrue);
      expect(await ordering.move(0, 1), isFalse);
      ordering.updateItems(["a", "b", "c"]);
      expect(ordering.ids, ["b", "c", "a"]);
      save.complete();
      expect(await first, isTrue);
      expect(ordering.saving, isFalse);
      expect(writes.single.entryId, "a");
      expect(writes.single.anchorId, "c");
      expect(writes.single.placement, LibrarySidebarPlacement.after);
      expect(await ordering.move(2, 0), isTrue);
      expect(writes.last.placement, LibrarySidebarPlacement.before);
      expect(ordering.ids, ["a", "b", "c"]);
    },
  );

  test(
    "a failed save restores surviving rows and appends concurrent arrivals",
    () async {
      final save = Completer<void>();
      final ordering = LibrarySidebarOrdering(
        ids: ["a", "b", "c"],
        save: (_) => save.future,
      );
      addTearDown(ordering.dispose);
      final result = ordering.move(0, 2);
      final failure = expectLater(result, throwsStateError);
      ordering.updateItems(["a", "c", "d"]);
      expect(ordering.ids, ["c", "a", "d"]);
      save.completeError(StateError("write failed"));
      await failure;
      expect(ordering.ids, ["a", "c", "d"]);
      expect(ordering.saving, isFalse);
    },
  );

  test("disposal retires late completion notifications", () async {
    final save = Completer<void>();
    final ordering = LibrarySidebarOrdering(
      ids: ["a", "b"],
      save: (_) => save.future,
    );
    var notifications = 0;
    ordering.addListener(() => notifications++);
    final result = ordering.move(0, 1);
    expect(notifications, 1);
    ordering.dispose();
    save.complete();
    expect(await result, isTrue);
    expect(notifications, 1);
  });
}
