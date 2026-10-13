import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_viewport_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_gallery_query_transition.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  for (final previous in [true, false]) {
    final direction = previous ? "previous" : "next";
    test("stale $direction page retains the middle gallery identity", () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final loading = fixture.loadPage(previous);
      fixture.catalog.page.completeError(_staleCursor);
      await loading;

      expect(fixture.catalog.anchors.single?.requestedLocationId, "middle");
      expect(fixture.catalog.anchors.single?.assetId, "asset-middle");
      expect(fixture.catalog.anchors.single?.fallbackGlobalItemIndex, 4101);
      expect(fixture.state.windowStartItemOffset, 4200);
      final position = fixture.publications.single.position!;
      expect(position.locationId, "middle");
      expect(position.globalItemIndex, 4201);
      expect(position.itemFraction, 0.3);
      expect(position.viewportFraction, 0.5);
      expect(fixture.state.isLoadingPage, isFalse);
      expect(fixture.state.isLoadingPreviousPage, isFalse);
    });
  }

  test(
    "page recovery waits for scrolling and captures its final position",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      fixture.transition.setUserScrolling(true);
      fixture.viewport.queryProjections.invalidatePosition();
      final loading = fixture.loadPage(true);
      fixture.catalog.page.completeError(_staleCursor);
      await Future<void>.delayed(Duration.zero);
      expect(fixture.catalog.anchors, isEmpty);
      fixture.moveToAfter();
      fixture.transition.setUserScrolling(false);
      await loading;
      expect(fixture.catalog.anchors.single?.requestedLocationId, "after");
      expect(fixture.publications.single.position?.globalItemIndex, 4202);
    },
  );

  test("later scrolling retires an in-flight recovery anchor", () async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    final pending = Completer<LibraryQuerySnapshot>();
    fixture.catalog.firstRefresh = pending;
    final loading = fixture.loadPage(true);
    fixture.catalog.page.completeError(_staleCursor);
    await fixture.catalog.refreshStarted.future;
    final oldAnchor = fixture.catalog.anchors.single;
    fixture.transition.setUserScrolling(true);
    fixture.viewport.queryProjections.invalidatePosition();
    fixture.moveToAfter();
    pending.complete(_querySnapshot(oldAnchor));
    await Future<void>.delayed(Duration.zero);
    expect(fixture.state.catalogRevision, BigInt.one);
    expect(fixture.publications, isEmpty);
    fixture.transition.setUserScrolling(false);
    await loading;
    expect(
      fixture.catalog.anchors.map((anchor) => anchor?.requestedLocationId),
      ["middle", "after"],
    );
    expect(fixture.publications.single.position?.globalItemIndex, 4202);
    expect(fixture.state.isLoadingPreviousPage, isFalse);
  });

  test(
    "disposal releases a pending recovery without accepting its late read",
    () async {
      final fixture = _Fixture();
      final pending = Completer<LibraryQuerySnapshot>();
      fixture.catalog.firstRefresh = pending;
      final loading = fixture.loadPage(true);
      fixture.catalog.page.completeError(_staleCursor);
      await fixture.catalog.refreshStarted.future;
      fixture.dispose();
      await loading;
      pending.complete(_querySnapshot(fixture.catalog.anchors.single));
      await Future<void>.delayed(Duration.zero);
      expect(fixture.state.catalogRevision, BigInt.one);
      expect(fixture.publications, isEmpty);
    },
  );

  test("retired page cannot replace a newer query", () async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    final loading = fixture.loadPage(true);
    fixture.viewport.supersedeExternalRequests();
    fixture.state = fixture.state.copyWith(queryId: "replacement");
    fixture.catalog.page.completeError(_staleCursor);
    await loading;
    expect(fixture.state.queryId, "replacement");
    expect(fixture.catalog.anchors, isEmpty);
    expect(fixture.publications, isEmpty);
    expect(fixture.state.isLoadingPreviousPage, isFalse);
  });

  test("page recovery cannot erase a newer query failure", () async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    final pending = Completer<LibraryQuerySnapshot>();
    fixture.catalog.firstRefresh = pending;
    final loading = fixture.loadPage(true);
    fixture.catalog.page.completeError(_staleCursor);
    await fixture.catalog.refreshStarted.future;
    final recoveryAnchor = fixture.catalog.anchors.single;

    final applied = await fixture.viewport.updateQuery(
      const LibraryGalleryQuery(searchText: "unavailable"),
    );
    expect(applied, isFalse);
    final queryFailure = fixture.state.queryActivity;
    expect(queryFailure, isA<LibraryQueryFailed>());

    pending.complete(_querySnapshot(recoveryAnchor));
    await loading;
    expect(fixture.state.queryActivity, same(queryFailure));
    expect(fixture.state.windowStartItemOffset, 4100);
    expect(fixture.catalog.anchors, hasLength(2));
    expect(fixture.publications, isEmpty);
    expect(fixture.state.isLoadingPreviousPage, isFalse);
  });

  test(
    "passive refresh captures the mounted gallery after admission",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final outcome = await fixture.viewport.refreshFromSynchronization(
        catalogRevision: BigInt.two,
      );
      expect(outcome, LibraryQueryUpdateOutcome.applied);
      expect(fixture.catalog.anchors.single?.requestedLocationId, "middle");
      expect(fixture.publications.single.position?.globalItemIndex, 4201);
    },
  );

  test("supersession retires a gesture-delayed passive refresh", () async {
    final fixture = _Fixture();
    addTearDown(fixture.dispose);
    fixture.transition.setUserScrolling(true);
    fixture.viewport.queryProjections.invalidatePosition();
    final passive = fixture.viewport.refreshFromSynchronization(
      catalogRevision: BigInt.two,
    );
    await Future<void>.delayed(Duration.zero);
    fixture.viewport.supersedeExternalRequests();
    fixture.transition.setUserScrolling(false);
    expect(await passive, LibraryQueryUpdateOutcome.superseded);
    expect(fixture.catalog.anchors, isEmpty);
    expect(fixture.publications, isEmpty);
    expect(fixture.state.catalogRevision, BigInt.one);
  });

  test(
    "busy passive refresh cannot retire the page recovery position",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final pending = Completer<LibraryQuerySnapshot>();
      fixture.catalog.firstRefresh = pending;
      final loading = fixture.loadPage(true);
      fixture.catalog.page.completeError(_staleCursor);
      await fixture.catalog.refreshStarted.future;

      final passive = await fixture.viewport.refreshFromSynchronization(
        catalogRevision: BigInt.two,
      );
      expect(passive, LibraryQueryUpdateOutcome.busy);
      expect(fixture.catalog.anchors, hasLength(1));
      pending.complete(_querySnapshot(fixture.catalog.anchors.single));
      await loading;
      final position = fixture.publications.single.position!;
      expect(position.locationId, "middle");
      expect(position.itemFraction, 0.3);
      expect(position.viewportFraction, 0.5);
    },
  );

  test(
    "ordinary page failure preserves the window and reports the error",
    () async {
      final fixture = _Fixture();
      addTearDown(fixture.dispose);
      final loading = fixture.loadPage(true);
      fixture.catalog.page.completeError(
        const LibraryCatalogFailure(
          code: "read_failed",
          message: "read failed",
        ),
      );
      await loading;
      expect(fixture.state.windowStartItemOffset, 4100);
      expect(fixture.state.previousPageErrorMessage, contains("read failed"));
      expect(fixture.catalog.anchors, isEmpty);
      expect(fixture.publications, isEmpty);
      expect(fixture.state.isLoadingPreviousPage, isFalse);
    },
  );
}

const _staleCursor = LibraryCatalogFailure(
  code: "catalog_cursor_stale",
  message: "Preview metadata changed the catalog revision",
);

class _Fixture {
  _Fixture() {
    viewport = LibraryViewportController(
      catalog,
      () => state,
      (value) => state = value,
      () => false,
      (_) {},
    )..seed(state);
    transition = LibraryGalleryQueryTransition(
      readState: () => state,
      capturePosition: (_) => position,
      readViewerAnchor: () => null,
      reconcileViewer: () async {},
      onPublished: publications.add,
      onFailed: () {},
    );
    viewport.queryProjections.attach(transition);
  }

  final catalog = _Catalog();
  LibraryState state = LibraryState.fromSnapshot(
    _snapshot(1),
  ).copyWith(windowStartItemOffset: 4100, timeline: _timeline(1));
  LibraryGalleryVisiblePosition position = LibraryGalleryVisiblePosition(
    queryId: "query",
    revision: BigInt.one,
    monthKey: "2012-05",
    locationId: "middle",
    assetId: "asset-middle",
    globalItemIndex: 4101,
    itemFraction: 0.3,
    viewportFraction: 0.5,
  );
  final publications = <LibraryGalleryQueryPublication>[];
  late final LibraryViewportController viewport;
  late final LibraryGalleryQueryTransition transition;

  Future<void> loadPage(bool previous) async {
    if (previous) {
      await viewport.loadPreviousPage();
    } else {
      await viewport.loadNextPage();
    }
  }

  void moveToAfter() {
    position = LibraryGalleryVisiblePosition(
      queryId: "query",
      revision: BigInt.one,
      monthKey: "2012-05",
      locationId: "after",
      assetId: "asset-after",
      globalItemIndex: 4102,
      itemFraction: 0.3,
      viewportFraction: 0.5,
    );
  }

  void dispose() {
    transition.dispose();
    viewport.dispose();
  }
}

class _Catalog implements LibraryCatalog {
  final page = Completer<LibrarySnapshot>();
  final anchors = <LibraryQueryAnchor?>[];
  final refreshStarted = Completer<void>();
  Completer<LibraryQuerySnapshot>? firstRefresh;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) => page.future;

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) async {
    anchors.add(anchor);
    if (query.searchText == "unavailable") {
      throw const LibraryCatalogFailure(
        code: "read_failed",
        message: "Search snapshot unavailable",
      );
    }
    if (!refreshStarted.isCompleted) {
      refreshStarted.complete();
    }
    if (anchors.length == 1 && firstRefresh != null) {
      return firstRefresh!.future;
    }
    return _querySnapshot(anchor);
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      _timeline(2);

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError("This fixture starts at a loaded middle date");

  @override
  Future<LibraryTimeSnapshot> resolveTimeIntent({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeIntent intent,
  }) => throw UnimplementedError("This fixture starts at a loaded middle date");

  @override
  Future<bool> unregisterRoot(String rootId) =>
      throw UnimplementedError("This fixture does not remove roots");
}

LibraryQuerySnapshot _querySnapshot(LibraryQueryAnchor? anchor) =>
    LibraryQuerySnapshot(
      snapshot: _snapshot(2, anchor: anchor),
      timeline: _timeline(2),
    );

LibrarySnapshot _snapshot(int revision, {LibraryQueryAnchor? anchor}) {
  final cursor = LibraryCatalogCursor(
    revision: BigInt.from(revision),
    queryId: "query",
    primaryMissing: false,
    primaryText: "2012-05",
    primaryNumber: 1,
    rootId: "fixture-root",
    locationId: "middle",
  );
  return LibrarySnapshot(
    catalogPath: "fixture.sqlite3",
    revision: BigInt.from(revision),
    queryId: "query",
    roots: const [
      LibraryRoot(
        id: "fixture-root",
        path: "fixture",
        displayPath: "fixture",
        activeScanId: "scan",
        createdUnixMs: 1,
        assetCount: 10000,
        issueCount: 0,
      ),
    ],
    assets: [_asset("before"), _asset("middle"), _asset("after")],
    previousCursor: cursor,
    nextCursor: cursor,
    queryAnchorResolution: anchor == null
        ? null
        : LibraryQueryAnchorResolution(
            requestedLocationId: anchor.requestedLocationId,
            locationId: anchor.requestedLocationId,
            ordinal: anchor.requestedLocationId == "after" ? 4202 : 4201,
            windowStartItemOffset: 4200,
          ),
  );
}

LibraryTimeline _timeline(int revision) => LibraryTimeline(
  revision: BigInt.from(revision),
  queryId: "query",
  totalItems: 10000,
  buckets: const [
    LibraryTimeBucket(
      monthKey: "2026-09",
      itemCount: 4000,
      aspectRatioSum: 4000,
    ),
    LibraryTimeBucket(
      monthKey: "2012-05",
      itemCount: 6000,
      aspectRatioSum: 6000,
    ),
  ],
);

LibraryAsset _asset(String id) => LibraryAsset(
  assetId: "asset-$id",
  locationId: id,
  rootId: "fixture-root",
  activeScanId: "scan",
  sourcePath: "$id.png",
  displayPath: "$id.png",
  relativePath: "$id.png",
  previewPath: "$id.preview.png",
  fileSize: BigInt.one,
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 800,
  height: 600,
);
