import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_viewport_controller.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_viewer_session.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  for (final prefetchPrevious in [true, false]) {
    for (final failRequested in [false, true]) {
      test(
        "viewer ${prefetchPrevious ? 'next' : 'previous'} waits for opposite prefetch then ${failRequested ? 'reports current failure' : 'navigates'}",
        () async {
          final fixture = _BoundaryFixture();
          final prefetch = prefetchPrevious
              ? fixture.viewport.loadPreviousPage().then<void>((_) {})
              : fixture.viewport.loadNextPage();
          fixture.viewer.open(fixture.catalog.assets[1]);
          final navigation = fixture.viewer.navigate(
            prefetchPrevious
                ? LibraryViewerDirection.next
                : LibraryViewerDirection.previous,
          );
          await flushRetainedScanMicrotasks();
          expect(fixture.catalog.beforeLoads, prefetchPrevious ? 1 : 0);
          expect(fixture.catalog.afterLoads, prefetchPrevious ? 0 : 1);
          expect(fixture.viewer.isNavigating, isTrue);
          final prefetchRead = prefetchPrevious
              ? fixture.catalog.previous
              : fixture.catalog.next;
          prefetchRead.complete(fixture.catalog.page(prefetchPrevious ? 0 : 2));
          await prefetch;
          await flushRetainedScanMicrotasks();
          expect(fixture.catalog.beforeLoads, 1);
          expect(fixture.catalog.afterLoads, 1);
          final requestedRead = prefetchPrevious
              ? fixture.catalog.next
              : fixture.catalog.previous;
          final target = prefetchPrevious ? 2 : 0;
          if (failRequested) {
            requestedRead.completeError(
              const LibraryCatalogFailure(
                code: "page_unavailable",
                message: "current requested page failed",
              ),
            );
          } else {
            requestedRead.complete(fixture.catalog.page(target));
          }
          await navigation;
          expect(fixture.viewer.isNavigating, isFalse);
          expect(
            fixture.viewer.locationId,
            "location-${failRequested ? 1 : target}",
          );
          expect(fixture.catalog.beforeLoads, 1);
          expect(fixture.catalog.afterLoads, 1);
          if (failRequested) {
            expect(
              fixture.errors.single.toString(),
              contains("current requested page failed"),
            );
          } else {
            expect(fixture.errors, isEmpty);
          }
        },
      );
    }
  }

  test(
    "closing while opposite prefetch runs retires the queued viewer command",
    () async {
      final fixture = _BoundaryFixture();
      final previous = fixture.viewport.loadPreviousPage();
      fixture.viewer.open(fixture.catalog.assets[1]);
      final next = fixture.viewer.navigate(LibraryViewerDirection.next);
      fixture.viewer.close();
      fixture.viewer.open(fixture.catalog.assets[1]);
      fixture.catalog.previous.complete(fixture.catalog.page(0));
      await previous;
      await flushRetainedScanMicrotasks();
      fixture.catalog.next.complete(fixture.catalog.page(2));
      await next;
      expect(fixture.viewer.locationId, "location-1");
      expect(fixture.errors, isEmpty);
    },
  );

  test(
    "query replacement bypasses old prefetch and queued navigation never reads its stale cursor",
    () async {
      final fixture = _BoundaryFixture();
      final previous = fixture.viewport.loadPreviousPage();
      fixture.viewer.open(fixture.catalog.assets[1]);
      final next = fixture.viewer.navigate(LibraryViewerDirection.next);
      final didRefresh = await fixture.viewport.updateQuery(
        _query.copyWith(sortDirection: LibraryGallerySortDirection.descending),
      );
      expect(didRefresh, isTrue);
      expect(fixture.catalog.previous.isCompleted, isFalse);
      expect(fixture.state.queryId, "boundary-descending");
      fixture.catalog.previous.complete(fixture.catalog.page(0));
      expect(await previous, isFalse);
      await next;
      expect(fixture.catalog.afterLoads, 0);
      expect(fixture.viewer.locationId, "location-1");
      expect(fixture.errors, isEmpty);
      expect(fixture.state.assets.map((asset) => asset.locationId), [
        "location-2",
        "location-1",
        "location-0",
      ]);
    },
  );
}

const _query = LibraryGalleryQuery(
  sortKey: LibraryGallerySortKey.modifiedTime,
  sortDirection: LibraryGallerySortDirection.ascending,
);

class _BoundaryFixture {
  _BoundaryFixture() {
    catalog = _BoundaryCatalog(scanner);
    final cursor = catalog.cursor(catalog.assets[1]);
    state = LibraryState(
      roots: catalog.roots,
      assets: [catalog.assets[1]],
      query: _query,
      queryId: "boundary-ascending",
      catalogRevision: BigInt.one,
      catalogPath: "catalog.sqlite3",
      windowStartItemOffset: 1,
      previousCursor: cursor,
      nextCursor: cursor,
      timeline: catalog.timeline(_query),
    );
    viewport = LibraryViewportController(
      catalog,
      () => state,
      (value) => state = value,
      () => false,
      (_) {},
    )..seed(state);
    viewer = LibraryViewerSession(
      readLibrary: () => state,
      loadPrevious: () async {
        await viewport.loadPreviousPage();
      },
      loadNext: viewport.loadNextPage,
      catalog: catalog,
      onPreviewDemand: (_) {},
      onNavigationError: errors.add,
    );
    addTearDown(viewer.dispose);
    addTearDown(viewport.dispose);
    addTearDown(scanner.dispose);
  }
  final scanner = RetainedScanScanner();
  late final _BoundaryCatalog catalog;
  late final LibraryViewportController viewport;
  late final LibraryViewerSession viewer;
  late LibraryState state;
  final List<Object> errors = [];
}

class _BoundaryCatalog extends RetainedScanCatalog {
  _BoundaryCatalog(super.scanner) {
    assets = List.generate(
      3,
      (index) => snapshot(_query, offset: index).assets.single,
    );
    roots
      ..clear()
      ..add(
        LibraryRoot(
          id: otherPublishedRoot.id,
          path: otherPublishedRoot.path,
          displayPath: otherPublishedRoot.displayPath,
          activeScanId: otherPublishedRoot.activeScanId,
          createdUnixMs: 1,
          assetCount: 3,
          issueCount: 0,
          availability: otherPublishedRoot.availability,
        ),
      );
  }

  late final List<LibraryAsset> assets;
  final Completer<LibrarySnapshot> previous = Completer();
  final Completer<LibrarySnapshot> next = Completer();

  LibraryCatalogCursor cursor(LibraryAsset asset) => LibraryCatalogCursor(
    revision: BigInt.one,
    queryId: "boundary-ascending",
    primaryMissing: false,
    primaryText: "",
    primaryNumber: asset.modifiedUnixMs,
    rootId: asset.rootId,
    locationId: asset.locationId,
  );

  LibrarySnapshot page(int index) => LibrarySnapshot(
    roots: roots,
    assets: [assets[index]],
    catalogPath: "catalog.sqlite3",
    revision: BigInt.one,
    queryId: "boundary-ascending",
    previousCursor: index == 0 ? null : cursor(assets[index]),
    nextCursor: index == 2 ? null : cursor(assets[index]),
  );

  LibraryTimeline timeline(LibraryGalleryQuery query) => LibraryTimeline(
    revision: BigInt.one,
    queryId: "boundary-${query.sortDirection.name}",
    totalItems: 3,
    buckets: const [
      LibraryTimeBucket(monthKey: "1970-01", itemCount: 3, aspectRatioSum: 3),
    ],
  );

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    expectSync(maxItems, libraryCatalogWindow);
    if (after == null && before == null) {
      expectSync(
        query,
        _query.copyWith(sortDirection: LibraryGallerySortDirection.descending),
      );
      return LibrarySnapshot(
        roots: roots,
        assets: assets.reversed.toList(),
        catalogPath: "catalog.sqlite3",
        revision: BigInt.one,
        queryId: "boundary-descending",
      );
    }
    expectSync(query, _query);
    final anchor = (before ?? after)!;
    expectSync(anchor.revision, BigInt.one);
    expectSync(anchor.queryId, "boundary-ascending");
    expectSync(anchor.primaryMissing, isFalse);
    expectSync(anchor.primaryText, "");
    expectSync(anchor.primaryNumber, assets[1].modifiedUnixMs);
    expectSync(anchor.rootId, assets[1].rootId);
    expectSync(anchor.locationId, assets[1].locationId);
    if (before != null) {
      expectSync(after, isNull);
      beforeLoads += 1;
      return previous.future;
    }
    afterLoads += 1;
    return next.future;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline(query);
}
