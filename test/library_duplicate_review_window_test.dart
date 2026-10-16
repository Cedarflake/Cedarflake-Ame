import "dart:async";
import "dart:math" as math;

import "package:cedarflake_ame/features/library/application/library_duplicate_review_window.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "continuous windows cross batches, revisit evictions and stay bounded",
    () async {
      final requests = <int>[];
      final window = LibraryDuplicateReviewWindow<int, int>((after) async {
        final start = after ?? 0;
        requests.add(start);
        final end = math.min(start + 64, 10000);
        return LibraryDuplicateReviewSlice(
          items: List.generate(end - start, (index) => start + index),
          total: 10000,
          next: end < 10000 ? end : null,
        );
      });
      addTearDown(window.dispose);
      await window.ensureRange(0, 10);
      expect(window.itemAt(0), 0);
      await window.ensureRange(60, 72);
      expect(window.itemAt(63), 63);
      expect(window.itemAt(64), 64);
      for (var start = 128; start < 9600; start += 128) {
        await window.ensureRange(start, start + 10);
        expect(window.itemAt(start), start);
        expect(window.cachedItemCount, lessThanOrEqualTo(384));
        expect(window.checkpointCount, lessThanOrEqualTo(64));
      }
      expect(window.itemAt(0), isNull);
      await window.ensureRange(0, 10);
      expect(window.itemAt(0), 0);
      expect(requests.where((start) => start == 0).length, 2);
      await window.ensureRange(9990, 10000);
      expect(window.itemAt(9999), 9999);
      expect(window.total, 10000);
    },
  );

  test(
    "superseded reads cannot replace results or clear the new loading owner",
    () async {
      final old = Completer<LibraryDuplicateReviewSlice<int, int>>();
      final current = Completer<LibraryDuplicateReviewSlice<int, int>>();
      var calls = 0;
      final window = LibraryDuplicateReviewWindow<int, int>((_) {
        return calls++ == 0 ? old.future : current.future;
      });
      addTearDown(window.dispose);
      final obsolete = window.ensureRange(0, 1);
      await Future<void>.value();
      final fresh = window.refresh();
      await Future<void>.value();
      old.complete(const LibraryDuplicateReviewSlice(items: [1], total: 1));
      await obsolete;
      expect(window.loading, isTrue);
      expect(window.itemAt(0), isNull);
      current.complete(const LibraryDuplicateReviewSlice(items: [2], total: 1));
      await fresh;
      expect(window.itemAt(0), 2);
      expect(window.loading, isFalse);
    },
  );

  test(
    "failed continuation retains earlier results and retries only on request",
    () async {
      var calls = 0;
      var fail = true;
      final window = LibraryDuplicateReviewWindow<int, int>((after) async {
        calls++;
        if (after != null && fail) throw StateError("controlled read failure");
        return LibraryDuplicateReviewSlice(
          items: after == null ? [0, 1] : [2, 3],
          total: 4,
          next: after == null ? 2 : null,
        );
      });
      addTearDown(window.dispose);
      await window.ensureRange(0, 2);
      await window.ensureRange(2, 4);
      expect(window.failed, isTrue);
      expect(window.itemAt(0), 0);
      await window.ensureRange(2, 4);
      expect(calls, 2);
      fail = false;
      await window.retryRange(2, 4);
      expect(window.failed, isFalse);
      expect(window.itemAt(3), 3);
    },
  );

  test("disposal rejects a late page without notifying", () async {
    final pending = Completer<LibraryDuplicateReviewSlice<int, int>>();
    final window = LibraryDuplicateReviewWindow<int, int>(
      (_) => pending.future,
    );
    final load = window.ensureRange(0, 1);
    await Future<void>.value();
    window.dispose();
    pending.complete(const LibraryDuplicateReviewSlice(items: [1], total: 1));
    await load;
    expect(window.itemAt(0), isNull);
  });

  test(
    "revalidation publishes a complete visible range without clearing it",
    () async {
      final first = Completer<LibraryDuplicateReviewSlice<int, int>>();
      final second = Completer<LibraryDuplicateReviewSlice<int, int>>();
      var replacing = false;
      final window = LibraryDuplicateReviewWindow<int, int>((after) async {
        if (replacing) return after == null ? first.future : second.future;
        return LibraryDuplicateReviewSlice(
          items: after == null ? [0, 1] : [2, 3],
          total: 4,
          next: after == null ? 2 : null,
        );
      });
      addTearDown(window.dispose);
      await window.ensureRange(0, 4);
      replacing = true;
      final refresh = window.revalidate();
      await Future<void>.value();
      expect(window.loading, isFalse);
      expect(window.loadedItems, [0, 1, 2, 3]);
      first.complete(
        const LibraryDuplicateReviewSlice(items: [10, 11], total: 4, next: 2),
      );
      await Future<void>.delayed(Duration.zero);
      expect(window.loadedItems, [0, 1, 2, 3]);
      second.complete(
        const LibraryDuplicateReviewSlice(items: [12, 13], total: 4),
      );
      await refresh;
      expect(window.loadedItems, [10, 11, 12, 13]);
      expect(window.loading, isFalse);
    },
  );

  test(
    "a superseded revalidation cannot publish over an explicit refresh",
    () async {
      final old = Completer<LibraryDuplicateReviewSlice<int, int>>();
      final fresh = Completer<LibraryDuplicateReviewSlice<int, int>>();
      var calls = 0;
      final window = LibraryDuplicateReviewWindow<int, int>((_) async {
        calls++;
        if (calls == 1) {
          return const LibraryDuplicateReviewSlice(items: [0], total: 1);
        }
        return calls == 2 ? old.future : fresh.future;
      });
      addTearDown(window.dispose);
      await window.ensureRange(0, 1);
      final obsolete = window.revalidate();
      await Future<void>.value();
      final current = window.refresh();
      await Future<void>.value();
      old.complete(const LibraryDuplicateReviewSlice(items: [1], total: 1));
      await obsolete;
      expect(window.loading, isTrue);
      expect(window.itemAt(0), isNull);
      fresh.complete(const LibraryDuplicateReviewSlice(items: [2], total: 1));
      await current;
      expect(window.itemAt(0), 2);
    },
  );

  test(
    "revalidation failure is reported while preserving the last payload",
    () async {
      var fail = false;
      final window = LibraryDuplicateReviewWindow<int, int>((_) async {
        if (fail) throw StateError("controlled revalidation failure");
        return const LibraryDuplicateReviewSlice(items: [0], total: 1);
      });
      addTearDown(window.dispose);
      await window.ensureRange(0, 1);
      fail = true;
      await window.revalidate();
      expect(window.failed, isTrue);
      expect(window.itemAt(0), 0);
      fail = false;
      await window.revalidate();
      expect(window.failed, isFalse);
    },
  );
}
