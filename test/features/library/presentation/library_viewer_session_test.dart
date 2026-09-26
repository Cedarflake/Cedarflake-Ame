import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_viewer_session.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
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
      },
    );
  }

  test(
    "retired post-frame catalog selection cannot replace reopened source demand",
    () {
      final fixture = _ViewerFixture();
      fixture.viewer.open(fixture.assets[1]);
      final old = fixture.viewer.selection!;
      fixture.viewer.close();
      fixture.viewer.open(fixture.assets[1]);
      final count = fixture.demands.length;
      fixture.viewer.retainCatalogAsset(old, fixture.assets[1]);
      expect(fixture.demands, hasLength(count));
    },
  );
}

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
  final Completer<LibraryAsset?> lookup = Completer();

  @override
  Future<LibraryAsset?> loadAssetById({
    required String assetId,
    String? preferredLocationId,
  }) => lookup.future;
}
