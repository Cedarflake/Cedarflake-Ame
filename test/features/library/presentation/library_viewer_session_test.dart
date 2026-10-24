import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_viewer_session.dart";
import "package:cedarflake_ame/features/library/presentation/library_viewer_source_state.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  for (final stillInCollection in [true, false]) {
    test(
      "revalidates retained position after grouping or collection removal: $stillInCollection",
      () async {
        final fixture = _ViewerFixture();
        final selected = fixture.assets[1];
        fixture.viewer.open(selected);
        fixture.state = fixture.state.copyWith(
          assets: [fixture.assets[0]],
          catalogRevision: BigInt.two,
        );
        expect(fixture.viewer.navigation(1).position, isNull);
        fixture.catalog.neighbor = Completer<LibraryQuerySnapshot>();
        final pending = fixture.viewer.reconcile();
        fixture.catalog.lookup.complete(selected);
        await Future<void>.delayed(Duration.zero);
        fixture.catalog.neighbor!.complete(
          _positionSnapshot(fixture, resolved: stillInCollection),
        );
        await pending;
        expect(fixture.viewer.retainedAsset, same(selected));
        expect(fixture.viewer.navigation(1), (
          position: stillInCollection ? 1 : null,
          canPrevious: false,
          canNext: false,
        ));
        expect(fixture.catalog.neighborLimit, 1);
        expect(fixture.viewer.sourceNotice, isNull);
      },
    );
  }

  test(
    "late position revalidation cannot replace a newer opened image",
    () async {
      final fixture = _ViewerFixture();
      final selected = fixture.assets[1];
      fixture.viewer.open(selected);
      fixture.state = fixture.state.copyWith(
        assets: [fixture.assets[0]],
        catalogRevision: BigInt.two,
      );
      fixture.catalog.neighbor = Completer<LibraryQuerySnapshot>();
      final pending = fixture.viewer.reconcile();
      fixture.catalog.lookup.complete(selected);
      await Future<void>.delayed(Duration.zero);
      fixture.viewer.open(fixture.assets[0]);
      fixture.catalog.neighbor!.complete(
        _positionSnapshot(fixture, resolved: false),
      );
      await pending;
      expect(fixture.viewer.locationId, fixture.assets[0].locationId);
      expect(fixture.viewer.navigation(1).position, 1);
    },
  );

  test(
    "retained duplicate member navigates from its current group representative",
    () async {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      fixture.state = fixture.state.copyWith(assets: const []);
      fixture.catalog.neighbor = Completer<LibraryQuerySnapshot>();
      final pending = fixture.viewer.navigate(LibraryViewerDirection.next);
      fixture.catalog.neighbor!.complete(
        LibraryQuerySnapshot(
          snapshot: LibrarySnapshot(
            catalogPath: "catalog.sqlite3",
            revision: BigInt.one,
            queryId: "viewer-test",
            roots: const [],
            assets: [fixture.assets[0], fixture.assets[2]],
            queryAnchorResolution: LibraryQueryAnchorResolution(
              requestedLocationId: fixture.assets[1].locationId,
              windowStartItemOffset: 0,
              locationId: fixture.assets[0].locationId,
              ordinal: 0,
            ),
          ),
          timeline: LibraryTimeline(
            revision: BigInt.one,
            queryId: "viewer-test",
            totalItems: 2,
            buckets: const [],
          ),
        ),
      );
      expect(await pending, isTrue);
      expect(fixture.viewer.locationId, fixture.assets[2].locationId);
      expect(fixture.viewer.ordinal, 1);
    },
  );

  for (final relocated in [false, true]) {
    for (final direction in LibraryViewerDirection.values) {
      test(
        "retired-page navigation uses the selected alias; relocated=$relocated, $direction",
        () async {
          final fixture = _ViewerFixture();
          final selected = fixture.assets[1];
          fixture.assets[0] = _alias(selected, "earlier-alias");
          fixture.viewer.open(selected);
          fixture.state = fixture.state.copyWith(
            assets: [fixture.assets[2]],
            windowStartItemOffset: 2,
          );
          if (relocated) {
            fixture.assets[1] = _alias(selected, "relocated-alias");
          }
          fixture.catalog.neighbor = Completer<LibraryQuerySnapshot>();
          final pending = fixture.viewer.navigate(direction);
          fixture.catalog.neighbor!.complete(
            LibraryQuerySnapshot(
              snapshot: LibrarySnapshot(
                catalogPath: "catalog.sqlite3",
                revision: BigInt.one,
                queryId: "viewer-test",
                roots: const [],
                assets: fixture.assets,
                queryAnchorResolution: LibraryQueryAnchorResolution(
                  requestedLocationId: selected.locationId,
                  windowStartItemOffset: 0,
                  locationId: fixture.assets[1].locationId,
                  ordinal: 1,
                ),
              ),
              timeline: LibraryTimeline(
                revision: BigInt.one,
                queryId: "viewer-test",
                totalItems: 3,
                buckets: const [],
              ),
            ),
          );
          final target = direction == LibraryViewerDirection.previous ? 0 : 2;
          expect(await pending, isTrue);
          expect(fixture.viewer.locationId, fixture.assets[target].locationId);
          expect(fixture.viewer.ordinal, target);
          expect(fixture.errors, isEmpty);
          expect(fixture.pages, isEmpty);
        },
      );
    }
  }

  for (final closeBeforeResult in [false, true]) {
    test(
      "retired gallery page uses a bounded viewer window; closed=$closeBeforeResult",
      () async {
        final fixture = _ViewerFixture();
        final viewer = fixture.viewer;
        viewer.open(fixture.assets[1]);
        fixture.state = fixture.state.copyWith(
          assets: [fixture.assets[2]],
          windowStartItemOffset: 2,
        );
        expect(viewer.navigation(3).canPrevious, isTrue);
        expect(viewer.navigation(3).position, 2);
        fixture.catalog.neighbor = Completer<LibraryQuerySnapshot>();
        final pending = viewer.navigate(LibraryViewerDirection.next);
        expect(fixture.catalog.neighborLimit, 64);
        expect(
          fixture.catalog.neighborAnchor?.requestedLocationId,
          fixture.assets[1].locationId,
        );
        if (closeBeforeResult) {
          viewer.close();
          viewer.open(fixture.assets[0], ordinal: 0);
        }
        fixture.catalog.neighbor!.complete(
          LibraryQuerySnapshot(
            snapshot: LibrarySnapshot(
              catalogPath: "catalog.sqlite3",
              revision: BigInt.one,
              queryId: "viewer-test",
              roots: const [],
              assets: fixture.assets,
              queryAnchorResolution: LibraryQueryAnchorResolution(
                requestedLocationId: fixture.assets[1].locationId,
                windowStartItemOffset: 0,
                locationId: fixture.assets[1].locationId,
                ordinal: 1,
              ),
            ),
            timeline: LibraryTimeline(
              revision: BigInt.one,
              queryId: "viewer-test",
              totalItems: 3,
              buckets: const [],
            ),
          ),
        );
        expect(await pending, !closeBeforeResult);
        expect(
          viewer.locationId,
          fixture.assets[closeBeforeResult ? 0 : 2].locationId,
        );
        expect(fixture.pages, isEmpty);
      },
    );
  }

  test(
    "current boundary request without an admitted page reports a visible failure",
    () async {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      final pending = fixture.viewer.navigate(LibraryViewerDirection.next);
      fixture.pages.single.complete();
      await pending;
      expect(fixture.viewer.locationId, fixture.assets[1].locationId);
      expect(fixture.viewer.isNavigating, isFalse);
      expect(
        fixture.errors.single.toString(),
        contains("viewer_page_load_failed"),
      );
    },
  );

  for (final fails in [false, true]) {
    test(
      "retired page ${fails ? 'failure' : 'completion'} preserves the new request owner",
      () async {
        final fixture = _ViewerFixture();
        final viewer = fixture.viewer;
        viewer.open(fixture.assets[1]);
        final old = viewer.navigate(LibraryViewerDirection.next);
        viewer.close();
        viewer.open(fixture.assets[1]);
        final current = viewer.navigate(LibraryViewerDirection.next);
        expect(fixture.pages, hasLength(2));
        if (fails) {
          fixture.pages.first.completeError(StateError("retired page"));
        } else {
          fixture.pages.first.complete();
        }
        await old;
        expect(viewer.isNavigating, isTrue);
        expect(viewer.locationId, fixture.assets[1].locationId);
        expect(fixture.errors, isEmpty);
        fixture.publishAll();
        fixture.pages.last.complete();
        await current;
        expect(viewer.isNavigating, isFalse);
        expect(viewer.locationId, fixture.assets[2].locationId);
      },
    );
  }

  test(
    "current page failure is reported and releases only its own busy state",
    () async {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      final pending = fixture.viewer.navigate(LibraryViewerDirection.next);
      final error = StateError("current page");
      fixture.pages.single.completeError(error);
      await pending;
      expect(fixture.errors, [same(error)]);
      expect(fixture.viewer.isNavigating, isFalse);
      expect(fixture.viewer.locationId, fixture.assets[1].locationId);
    },
  );

  test(
    "controller-owned page failure is reported without changing selection",
    () async {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      final pending = fixture.viewer.navigate(LibraryViewerDirection.next);
      fixture.state = fixture.state.copyWith(
        pageErrorMessage: "catalog unavailable",
      );
      fixture.pages.single.complete();
      await pending;
      expect(fixture.errors.single, isA<LibraryCatalogFailure>());
      expect(fixture.errors.single.toString(), contains("catalog unavailable"));
      expect(fixture.viewer.locationId, fixture.assets[1].locationId);
    },
  );

  test(
    "query revision retires pending navigation and permits a new in-window request",
    () async {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      final old = fixture.viewer.navigate(LibraryViewerDirection.next);
      fixture.state = fixture.state.copyWith(catalogRevision: BigInt.two);
      expect(fixture.viewer.isNavigating, isFalse);
      await fixture.viewer.navigate(LibraryViewerDirection.previous);
      expect(fixture.viewer.locationId, fixture.assets[0].locationId);
      fixture.pages.single.completeError(StateError("old revision"));
      await old;
      expect(fixture.viewer.locationId, fixture.assets[0].locationId);
      expect(fixture.errors, isEmpty);
    },
  );

  for (final fails in [false, true]) {
    test(
      "dispose fences delayed page ${fails ? 'failure' : 'completion'}",
      () async {
        final fixture = _ViewerFixture();
        fixture.viewer.open(fixture.assets[1]);
        final pending = fixture.viewer.navigate(LibraryViewerDirection.next);
        fixture.disposeViewer();
        final demands = fixture.demands.length;
        final notifications = fixture.notifications;
        if (fails) {
          fixture.pages.single.completeError(StateError("disposed"));
        } else {
          fixture.publishAll();
          fixture.pages.single.complete();
        }
        await pending;
        expect(fixture.errors, isEmpty);
        expect(fixture.demands, hasLength(demands));
        expect(fixture.demands.last, isNull);
        expect(fixture.notifications, notifications);
      },
    );
  }

  for (final changeQuery in [false, true]) {
    test(
      "stable-asset lookup cannot close a ${changeQuery ? 'changed query' : 'reopened same asset'} session",
      () async {
        final fixture = _ViewerFixture();
        fixture.viewer.open(fixture.assets[1]);
        final old = fixture.viewer.reconcile();
        if (changeQuery) {
          fixture.state = fixture.state.copyWith(catalogRevision: BigInt.two);
        } else {
          fixture.viewer.close();
          fixture.viewer.open(fixture.assets[1]);
        }
        fixture.catalog.lookup.complete(null);
        await old;
        expect(fixture.viewer.locationId, fixture.assets[1].locationId);
        expect(fixture.demands.last, same(fixture.assets[1]));
        expect(fixture.viewer.sourceNotice, isNull);
      },
    );
  }

  test(
    "a failed lookup cannot claim that the open image disappeared",
    () async {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      final pending = fixture.viewer.reconcile();
      fixture.catalog.lookup.completeError(StateError("catalog unavailable"));
      await pending;
      expect(fixture.viewer.retainedAsset, same(fixture.assets[1]));
      expect(fixture.viewer.sourceNotice, isNull);
    },
  );

  test(
    "confirmed removal preserves the viewer until the user leaves",
    () async {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      final navigation = fixture.viewer.navigate(LibraryViewerDirection.next);
      fixture.state = fixture.state.copyWith(assets: [fixture.assets[0]]);
      final pending = fixture.viewer.reconcile();
      fixture.catalog.lookup.complete(null);
      await pending;
      expect(fixture.viewer.retainedAsset, same(fixture.assets[1]));
      expect(fixture.viewer.sourceNotice, LibraryViewerSourceNotice.missing);
      expect(fixture.demands.last, isNull);
      expect(fixture.viewer.navigation(1), (
        position: null,
        canPrevious: false,
        canNext: false,
      ));
      expect(
        await fixture.viewer.navigate(LibraryViewerDirection.previous),
        isFalse,
      );
      fixture.pages.single.complete();
      expect(await navigation, isFalse);
      expect(fixture.errors, isEmpty);
      fixture.viewer.reloadSource();
      expect(fixture.viewer.retainedAsset, same(fixture.assets[1]));
      fixture.viewer.close();
      expect(fixture.viewer.retainedAsset, isNull);
      expect(fixture.viewer.sourceNotice, isNull);
    },
  );

  test("later catalog lookup owns the source change notice", () async {
    final fixture = _ViewerFixture();
    final selected = fixture.assets[1];
    fixture.viewer.open(selected);
    final oldLookup = fixture.catalog.lookup;
    final old = fixture.viewer.reconcile();
    fixture.catalog.lookup = Completer<LibraryAsset?>();
    final current = fixture.viewer.reconcile();
    fixture.catalog.lookup.complete(_alias(selected, "renamed"));
    await current;
    oldLookup.complete(null);
    await old;
    expect(fixture.viewer.locationId, "renamed");
    expect(fixture.viewer.sourceNotice, isNull);
  });

  for (final replacesIdentity in [false, true]) {
    test(
      "reload accepts the latest version; replaced=$replacesIdentity",
      () async {
        final fixture = _ViewerFixture();
        final selected = fixture.assets[1];
        fixture.viewer.open(selected);
        final changed = _alias(
          selected,
          selected.locationId,
          generation: BigInt.two,
          assetId: replacesIdentity ? "replacement" : selected.assetId,
        );
        fixture.state = fixture.state.copyWith(assets: [changed]);
        final pending = fixture.viewer.reconcile();
        fixture.catalog.lookup.complete(replacesIdentity ? null : changed);
        await pending;
        expect(fixture.viewer.retainedAsset, same(selected));
        expect(fixture.viewer.sourceNotice, LibraryViewerSourceNotice.changed);
        fixture.viewer.reloadSource();
        expect(fixture.viewer.retainedAsset, same(changed));
        expect(fixture.viewer.assetId, changed.assetId);
        expect(fixture.viewer.sourceNotice, isNull);
        expect(fixture.demands.last, same(changed));
      },
    );
  }
}

LibraryQuerySnapshot _positionSnapshot(
  _ViewerFixture fixture, {
  required bool resolved,
}) => LibraryQuerySnapshot(
  snapshot: LibrarySnapshot(
    catalogPath: "catalog.sqlite3",
    revision: BigInt.two,
    queryId: "viewer-test",
    roots: const [],
    assets: [fixture.assets[0]],
    queryAnchorResolution: LibraryQueryAnchorResolution(
      requestedLocationId: fixture.assets[1].locationId,
      windowStartItemOffset: 0,
      locationId: resolved ? fixture.assets[0].locationId : null,
      ordinal: resolved ? 0 : null,
    ),
  ),
  timeline: LibraryTimeline(
    revision: BigInt.two,
    queryId: "viewer-test",
    totalItems: 1,
    buckets: const [],
  ),
);

LibraryAsset _alias(
  LibraryAsset asset,
  String locationId, {
  BigInt? generation,
  String? assetId,
}) => LibraryAsset(
  assetId: assetId ?? asset.assetId,
  locationId: locationId,
  rootId: asset.rootId,
  activeScanId: asset.activeScanId,
  sourcePath: "unused/$locationId.jpg",
  displayPath: "$locationId.jpg",
  relativePath: "$locationId.jpg",
  previewPath: asset.previewPath,
  fileSize: asset.fileSize,
  modifiedUnixMs: asset.modifiedUnixMs,
  sourceRevision: asset.sourceRevision,
  sourceGeneration: generation ?? asset.sourceGeneration,
  width: asset.width,
  height: asset.height,
);

class _ViewerFixture {
  _ViewerFixture() {
    catalog = _LookupCatalog(scanner);
    assets = List.generate(
      3,
      (index) => catalog
          .snapshot(const LibraryGalleryQuery(), offset: index)
          .assets
          .single,
    );
    state = LibraryState(
      assets: assets.take(2).toList(),
      catalogRevision: BigInt.one,
      queryId: "viewer-test",
      nextCursor: _cursor(assets[1]),
    );
    viewer = LibraryViewerSession(
      readLibrary: () => state,
      loadPrevious: _page,
      loadNext: _page,
      catalog: catalog,
      onPreviewDemand: demands.add,
      onNavigationError: errors.add,
    )..addListener(() => notifications += 1);
    addTearDown(disposeViewer);
    addTearDown(scanner.dispose);
  }

  final scanner = RetainedScanScanner();
  late final _LookupCatalog catalog;
  late final List<LibraryAsset> assets;
  late LibraryState state;
  late final LibraryViewerSession viewer;
  final List<Completer<void>> pages = [];
  final List<LibraryAsset?> demands = [];
  final List<Object> errors = [];
  int notifications = 0;
  bool _disposed = false;

  Future<void> _page() {
    final page = Completer<void>();
    pages.add(page);
    return page.future;
  }

  void publishAll() => state = state.copyWith(assets: assets, nextCursor: null);

  void disposeViewer() {
    if (_disposed) {
      return;
    }
    _disposed = true;
    viewer.dispose();
  }

  LibraryCatalogCursor _cursor(LibraryAsset last) => LibraryCatalogCursor(
    revision: BigInt.one,
    queryId: "viewer-test",
    primaryMissing: false,
    primaryText: "",
    primaryNumber: last.modifiedUnixMs,
    rootId: last.rootId,
    locationId: last.locationId,
  );
}

class _LookupCatalog extends RetainedScanCatalog
    implements LibraryStableAssetCatalog {
  _LookupCatalog(super.scanner);
  Completer<LibraryAsset?> lookup = Completer();
  Completer<LibraryQuerySnapshot>? neighbor;
  int? neighborLimit;
  LibraryQueryAnchor? neighborAnchor;

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) {
    final pending = neighbor;
    if (pending == null) {
      return super.loadQuerySnapshot(
        maxItems: maxItems,
        query: query,
        anchor: anchor,
      );
    }
    neighborLimit = maxItems;
    neighborAnchor = anchor;
    return pending.future;
  }

  @override
  Future<LibraryAsset?> loadAssetById({
    required String assetId,
    String? preferredLocationId,
  }) => lookup.future;
}
