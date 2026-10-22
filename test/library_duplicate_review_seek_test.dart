import "dart:async";
import "dart:math" as math;

import "package:cedarflake_ame/features/library/application/library_duplicate_review_window.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test(
    "distant groups load directly and adjacent scrolling keeps keyset paging",
    () async {
      final fixture = _SeekFixture();
      final window = fixture.window;
      addTearDown(window.dispose);
      await window.ensureRange(0, 10);
      await window.ensureRange(12561, 12571);
      expect(fixture.loads, [0]);
      expect(fixture.seeks, [12561]);
      expect(window.itemAt(12561), 12561);
      expect(window.itemAt(12570), 12570);
      expect(window.total, 12571);

      await window.ensureRange(6300, 6310);
      await window.ensureRange(6334, 6350);
      expect(fixture.loads, [0, 6336]);
      expect(fixture.seeks, [12561, 6300]);
      expect(window.itemAt(6349), 6349);
      for (var start = 1000; start < 12000; start += 1000) {
        await window.ensureRange(start, start + 10);
        expect(window.itemAt(start), start);
        expect(window.cachedItemCount, lessThanOrEqualTo(384));
        expect(window.checkpointCount, lessThanOrEqualTo(64));
      }
      await window.ensureRange(12561, 12571);
      expect(window.itemAt(12570), 12570);
    },
  );

  test(
    "revalidation retains rows and follows a moved viewport without replay",
    () async {
      final fixture = _SeekFixture();
      final window = fixture.window;
      addTearDown(window.dispose);
      await window.ensureRange(0, 10);
      await window.ensureRange(12561, 12571);
      final held = Completer<LibraryDuplicateReviewSeek<int, int>>();
      fixture.held = held;
      fixture.version = 100000;
      final refresh = window.revalidate();
      await Future<void>.value();
      expect(window.loading, isFalse);
      expect(window.itemAt(12561), 12561);
      final moved = window.ensureRange(6300, 6310);
      expect(window.loading, isTrue);
      held.complete(fixture.range(12561));
      await refresh;
      await moved;
      expect(fixture.loads, [0]);
      expect(fixture.seeks, [12561, 12561, 6300]);
      expect(window.itemAt(6300), 106300);
      expect(window.loading, isFalse);
    },
  );

  test("seek clamps a stale viewport after the result set shrinks", () async {
    final fixture = _SeekFixture();
    final window = fixture.window;
    addTearDown(window.dispose);
    await window.ensureRange(0, 1);
    fixture.total = 70;
    await window.ensureRange(12561, 12571);
    expect(window.total, 70);
    expect(window.itemAt(69), 69);
    expect(window.failed, isFalse);
    fixture.total = 0;
    await window.revalidate();
    expect(window.total, 0);
    expect(window.loadedItems, isEmpty);
  });

  test("failed seeks retain cached rows and wait for explicit retry", () async {
    final fixture = _SeekFixture();
    final window = fixture.window;
    addTearDown(window.dispose);
    await window.ensureRange(0, 1);
    final held = Completer<LibraryDuplicateReviewSeek<int, int>>();
    fixture.held = held;
    final pending = window.ensureRange(12561, 12571);
    await Future<void>.value();
    held.completeError(StateError("controlled seek failure"));
    await pending;
    expect(window.failed, isTrue);
    expect(window.itemAt(0), 0);
    await window.ensureRange(12561, 12571);
    expect(fixture.seeks, [12561]);
    await window.retryRange(12561, 12571);
    expect(window.itemAt(12570), 12570);
    expect(window.failed, isFalse);
  });

  test("superseded and disposed seeks cannot publish late results", () async {
    final fixture = _SeekFixture();
    final window = fixture.window;
    await window.ensureRange(0, 1);
    final held = Completer<LibraryDuplicateReviewSeek<int, int>>();
    fixture.held = held;
    final obsolete = window.ensureRange(12561, 12571);
    await Future<void>.value();
    await window.refresh();
    fixture.version = 100000;
    held.complete(fixture.range(12561));
    await obsolete;
    expect(window.itemAt(12561), 12561);
    expect(window.loading, isFalse);

    final late = Completer<LibraryDuplicateReviewSeek<int, int>>();
    fixture.held = late;
    final pending = window.ensureRange(6300, 6310);
    await Future<void>.value();
    window.dispose();
    late.complete(fixture.range(6300));
    await pending;
    expect(window.itemAt(6300), isNull);
  });
}

class _SeekFixture {
  int total = 12571;
  int version = 0;
  final loads = <int>[];
  final seeks = <int>[];
  Completer<LibraryDuplicateReviewSeek<int, int>>? held;
  late final window = LibraryDuplicateReviewWindow<int, int>(
    (after) async {
      loads.add(after ?? 0);
      return slice(after ?? 0);
    },
    seek: (index) async {
      seeks.add(index);
      final pending = held;
      held = null;
      return pending == null ? range(index) : pending.future;
    },
  );

  LibraryDuplicateReviewSlice<int, int> slice(int start) {
    final end = math.min(start + 64, total);
    return LibraryDuplicateReviewSlice(
      items: List.generate(end - start, (index) => version + start + index),
      total: total,
      next: end < total ? end : null,
    );
  }

  LibraryDuplicateReviewSeek<int, int> range(int index) {
    final start = math.min(index, math.max(0, total - 1)) ~/ 64 * 64;
    return LibraryDuplicateReviewSeek(startIndex: start, slice: slice(start));
  }
}
