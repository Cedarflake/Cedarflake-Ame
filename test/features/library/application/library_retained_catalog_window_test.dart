import "package:cedarflake_ame/features/library/application/library_page_operation.dart";
import "package:cedarflake_ame/features/library/application/library_retained_catalog_window.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final direction in LibraryPageDirection.values) {
    test("overlap updates existing identities while extending $direction", () {
      final previous = direction == LibraryPageDirection.previous;
      final fixture = _WindowFixture([_asset(2), _asset(3)], start: 20);
      final replacement = _asset(2, preview: "updated.jpg");
      final result = fixture.merge(
        previous ? [_asset(1), replacement] : [replacement, _asset(4)],
        direction,
        before: _cursor(1),
        after: _cursor(4),
      );
      expect(
        result.assets.map((asset) => asset.locationId),
        previous ? ["1", "2", "3"] : ["2", "3", "4"],
      );
      expect(
        result.assets.firstWhere((asset) => asset.locationId == "2"),
        same(replacement),
      );
      expect(result.startItemOffset, previous ? 19 : 20);
      expect(result.addedItemCount, 1);
      expect(result.previousCursor?.locationId, previous ? "1" : "20");
      expect(result.nextCursor?.locationId, previous ? "22" : "4");
      expect(() => result.assets.clear(), throwsUnsupportedError);
    });

    test("overlap-only $direction keeps the other boundary cursor", () {
      final fixture = _WindowFixture([_asset(2)], start: 20);
      final first = _asset(2, preview: "first.jpg");
      final last = _asset(2, preview: "last.jpg");
      final result = fixture.merge(
        [first, last],
        direction,
        before: _cursor(10),
        after: _cursor(30),
      );
      expect(result.assets.single, same(last));
      expect(result.addedItemCount, 0);
      expect(result.startItemOffset, 20);
      expect(
        result.previousCursor?.locationId,
        direction == LibraryPageDirection.previous ? "10" : "20",
      );
      expect(
        result.nextCursor?.locationId,
        direction == LibraryPageDirection.next ? "30" : "21",
      );
    });

    test("empty terminal $direction advances only that boundary", () {
      final fixture = _WindowFixture([_asset(2)], start: 20);
      final result = fixture.merge([], direction);
      expect(result.assets.single.locationId, "2");
      expect(result.addedItemCount, 0);
      expect(result.startItemOffset, 20);
      expect(
        result.previousCursor?.locationId,
        direction == LibraryPageDirection.previous ? null : "20",
      );
      expect(
        result.nextCursor?.locationId,
        direction == LibraryPageDirection.next ? null : "21",
      );
    });
  }

  test("forward overflow evicts whole leading pages and retains reversal", () {
    final fixture = _WindowFixture(_assets(0, 500));
    for (var page = 1; page < 10; page += 1) {
      fixture.merge(
        _assets(page * 500, 500),
        LibraryPageDirection.next,
        before: _cursor(page - 1),
        after: _cursor(page + 1),
      );
    }
    expect(fixture.state.assets, hasLength(5000));
    expect(fixture.state.windowStartItemOffset, 0);
    final trimmed = fixture.merge(
      _assets(5000, 500),
      LibraryPageDirection.next,
      before: _cursor(9),
      after: _cursor(11),
    );
    expect(trimmed.assets, hasLength(3500));
    expect(trimmed.startItemOffset, 2000);
    expect(trimmed.assets.first.locationId, "2000");
    expect(trimmed.previousCursor?.locationId, "3");
    final reversed = fixture.merge(
      _assets(1500, 500),
      LibraryPageDirection.previous,
      before: _cursor(2),
      after: _cursor(4),
    );
    expect(reversed.assets, hasLength(4000));
    expect(reversed.startItemOffset, 1500);
    expect(reversed.nextCursor?.locationId, "11");
    expect(reversed.assets.last.locationId, "5499");
  });

  test("backward overflow evicts trailing pages and retains reversal", () {
    final fixture = _WindowFixture(_assets(5000, 500), start: 5000);
    for (var page = 9; page > 0; page -= 1) {
      fixture.merge(
        _assets(page * 500, 500),
        LibraryPageDirection.previous,
        before: _cursor(page - 1),
        after: _cursor(page + 1),
      );
    }
    expect(fixture.state.assets, hasLength(5000));
    expect(fixture.state.windowStartItemOffset, 500);
    final trimmed = fixture.merge(
      _assets(0, 500),
      LibraryPageDirection.previous,
      after: _cursor(1),
    );
    expect(trimmed.assets, hasLength(3500));
    expect(trimmed.startItemOffset, 0);
    expect(trimmed.previousCursor, isNull);
    expect(trimmed.nextCursor?.locationId, "7");
    expect(trimmed.assets.last.locationId, "3499");
    final reversed = fixture.merge(
      _assets(3500, 500),
      LibraryPageDirection.next,
      before: _cursor(6),
      after: _cursor(8),
    );
    expect(reversed.assets, hasLength(4000));
    expect(reversed.startItemOffset, 0);
    expect(reversed.previousCursor, isNull);
    expect(reversed.nextCursor?.locationId, "8");
  });

  for (final changesCount in [false, true]) {
    test("external window is adopted, count changed: $changesCount", () {
      final fixture = _WindowFixture([_asset(0)]);
      fixture.state = fixture.state.copyWith(
        assets: changesCount ? [_asset(10), _asset(11)] : [_asset(10)],
        windowStartItemOffset: changesCount ? 0 : 10,
        previousCursor: _cursor(9),
      );
      final result = fixture.merge([_asset(12)], LibraryPageDirection.next);
      expect(
        result.assets.map((asset) => asset.locationId),
        changesCount ? ["10", "11", "12"] : ["10", "12"],
      );
      expect(result.startItemOffset, changesCount ? 0 : 10);
      expect(result.previousCursor?.locationId, "9");
    });
  }

  test("reset discards a same-sized prior query window", () {
    final fixture = _WindowFixture([_asset(0)]);
    fixture.state = fixture.state.copyWith(assets: [_asset(10)]);
    fixture.reset();
    final result = fixture.merge([_asset(11)], LibraryPageDirection.next);
    expect(result.assets.map((asset) => asset.locationId), ["10", "11"]);
  });

  test("one oversized page is retained rather than split or discarded", () {
    final fixture = _WindowFixture(_assets(0, 5001));
    final result = fixture.merge([], LibraryPageDirection.next);
    expect(result.assets, hasLength(5001));
    expect(result.startItemOffset, 0);
    expect(result.nextCursor, isNull);
  });

  test("backward offset retains the missing-timeline fallback", () {
    final fixture = _WindowFixture([_asset(10)], start: 10);
    fixture.state = fixture.state.copyWith(timeline: null);
    final result = fixture.merge([_asset(9)], LibraryPageDirection.previous);
    expect(result.startItemOffset, 0);
  });
}

class _WindowFixture {
  _WindowFixture(List<LibraryAsset> assets, {int start = 0}) {
    state = LibraryState(
      assets: assets,
      windowStartItemOffset: start,
      previousCursor: _cursor(start),
      nextCursor: _cursor(start + assets.length),
      timeline: LibraryTimeline(
        revision: BigInt.one,
        queryId: "query",
        totalItems: 10000,
        buckets: const [],
      ),
    );
    reset();
  }

  final window = LibraryRetainedCatalogWindow();
  late LibraryState state;

  void reset() => window.reset(
    assets: state.assets,
    startItemOffset: state.windowStartItemOffset,
    previousCursor: state.previousCursor,
    nextCursor: state.nextCursor,
  );

  LibraryRetainedWindowUpdate merge(
    List<LibraryAsset> assets,
    LibraryPageDirection direction, {
    LibraryCatalogCursor? before,
    LibraryCatalogCursor? after,
  }) {
    final result = window.merge(
      LibrarySnapshot(
        roots: const [],
        assets: assets,
        catalogPath: "catalog.sqlite3",
        revision: BigInt.one,
        queryId: "query",
        previousCursor: before,
        nextCursor: after,
      ),
      current: state,
      direction: direction,
    );
    state = state.copyWith(
      assets: result.assets,
      windowStartItemOffset: result.startItemOffset,
      previousCursor: result.previousCursor,
      nextCursor: result.nextCursor,
    );
    return result;
  }
}

List<LibraryAsset> _assets(int start, int count) =>
    List.generate(count, (index) => _asset(start + index));

LibraryAsset _asset(int index, {String preview = "preview.jpg"}) =>
    LibraryAsset(
      assetId: "asset-$index",
      locationId: "$index",
      rootId: "root",
      activeScanId: "scan",
      sourcePath: "source-$index.jpg",
      displayPath: "source-$index.jpg",
      relativePath: "source-$index.jpg",
      previewPath: preview,
      fileSize: BigInt.one,
      modifiedUnixMs: index,
      sourceRevision: null,
      sourceGeneration: BigInt.one,
      width: 100,
      height: 100,
    );

LibraryCatalogCursor _cursor(int index) => LibraryCatalogCursor(
  revision: BigInt.one,
  queryId: "query",
  primaryMissing: false,
  primaryText: "",
  primaryNumber: index,
  rootId: "root",
  locationId: "$index",
);
