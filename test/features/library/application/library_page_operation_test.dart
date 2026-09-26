import "dart:async";

import "package:cedarflake_ame/features/library/application/library_page_operation.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "new query bypasses old read and retires its queued opposite direction",
    () async {
      final operation = LibraryPageOperation();
      final read = Completer<bool>();
      final old = operation.run(1, _cursor("old"), () => read.future);
      var queuedReads = 0;
      final queued = operation.run(1, _cursor("queued"), () async {
        queuedReads += 1;
        return true;
      }, direction: LibraryPageDirection.previous);
      expect(await operation.run(2, _cursor("new"), () async => true), isTrue);
      expect(read.isCompleted, isFalse);
      read.complete(false);
      expect(await old, isFalse);
      expect(await queued, isFalse);
      expect(queuedReads, 0);
    },
  );

  test(
    "failed predecessor releases opposite read without forwarding its error",
    () async {
      final operation = LibraryPageOperation();
      final read = Completer<bool>();
      final error = StateError("prefetch failed");
      final first = operation.run(
        1,
        _cursor("before"),
        () => read.future,
        direction: LibraryPageDirection.previous,
      );
      final failure = expectLater(first, throwsA(same(error)));
      var calls = 0;
      final next = operation.run(1, _cursor("after"), () async {
        calls += 1;
        return true;
      });
      expect(calls, 0);
      read.completeError(error);
      await failure;
      expect(await next, isTrue);
      expect(calls, 1);
    },
  );

  test(
    "synchronous state publication joins the already registered page flight",
    () async {
      final operation = LibraryPageOperation();
      final cursor = _cursor("first");
      final read = Completer<bool>();
      late Future<bool> reentered;
      final first = operation.run(1, cursor, () {
        reentered = operation.run(
          1,
          cursor,
          () => throw StateError("duplicate read"),
        );
        return read.future;
      });
      expect(reentered, same(first));
      read.complete(true);
      expect(await first, isTrue);
      expect(await reentered, isTrue);
    },
  );

  test(
    "synchronous read failure settles and releases the exact flight",
    () async {
      final operation = LibraryPageOperation();
      final cursor = _cursor("first");
      final error = StateError("synchronous read failure");
      await expectLater(
        operation.run(1, cursor, () => throw error),
        throwsA(same(error)),
      );
      expect(await operation.run(1, cursor, () async => true), isTrue);
    },
  );

  test(
    "same cursor joins the executing read and preserves its result",
    () async {
      final operation = LibraryPageOperation();
      final cursor = _cursor("first");
      final read = Completer<bool>();
      var calls = 0;
      Future<bool> load() {
        calls += 1;
        return read.future;
      }

      final first = operation.run(1, cursor, load);
      final second = operation.run(1, cursor, load);
      expect(second, same(first));
      expect(calls, 1);
      read.complete(true);
      expect(await first, isTrue);
      expect(await second, isTrue);
    },
  );

  for (final replaceCursor in [false, true]) {
    test(
      "retired ${replaceCursor ? 'cursor' : 'generation'} cannot clear the new flight",
      () async {
        final operation = LibraryPageOperation();
        final cursor = _cursor("first");
        final oldRead = Completer<bool>();
        final nextRead = Completer<bool>();
        final old = operation.run(1, cursor, () => oldRead.future);
        final generation = replaceCursor ? 1 : 2;
        final nextCursor = replaceCursor ? _cursor("second") : cursor;
        var calls = 0;
        Future<bool> load() {
          calls += 1;
          return nextRead.future;
        }

        final next = operation.run(generation, nextCursor, load);
        oldRead.complete(false);
        expect(await old, isFalse);
        await Future<void>.value();
        expect(operation.run(generation, nextCursor, load), same(next));
        expect(calls, 1);
        nextRead.complete(true);
        expect(await next, isTrue);
      },
    );
  }

  test(
    "shared failure reaches every caller and permits an explicit fresh retry",
    () async {
      final operation = LibraryPageOperation();
      final cursor = _cursor("first");
      final read = Completer<bool>();
      final error = StateError("page failed");
      final first = operation.run(1, cursor, () => read.future);
      final second = operation.run(
        1,
        cursor,
        () => throw StateError("duplicate read"),
      );
      final firstResult = expectLater(first, throwsA(same(error)));
      final secondResult = expectLater(second, throwsA(same(error)));
      read.completeError(error);
      await firstResult;
      await secondResult;
      expect(await operation.run(1, cursor, () async => true), isTrue);
    },
  );
}

LibraryCatalogCursor _cursor(String location) => LibraryCatalogCursor(
  revision: BigInt.one,
  queryId: "query",
  primaryMissing: false,
  primaryText: "",
  primaryNumber: 1,
  rootId: "root",
  locationId: location,
);
