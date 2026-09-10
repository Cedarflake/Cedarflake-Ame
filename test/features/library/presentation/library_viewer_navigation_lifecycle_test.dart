import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/retained_scan_fixture.dart";

void main() {
  for (final (reopenSameAsset, clickAgain) in [
    (true, false),
    (false, false),
    (true, true),
  ]) {
    testWidgets(
      clickAgain
          ? "reopened boundary navigation joins the real pending page exactly once"
          : reopenSameAsset
          ? "closed viewer paging cannot navigate a reopened same-asset session"
          : "closed viewer paging cannot block navigation in a new session",
      (tester) async {
        tester.view.physicalSize = const Size(1280, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final scanner = RetainedScanScanner()..checkpoint = null;
        final catalog = _HeldViewerPageCatalog(scanner);
        final container = ProviderContainer(
          overrides: [
            libraryScannerProvider.overrideWithValue(scanner),
            libraryCatalogProvider.overrideWithValue(catalog),
            initialLibraryStateProvider.overrideWithValue(
              LibraryState.fromSnapshot(
                catalog.firstPage(),
                query: _viewerQuery,
              ),
            ),
          ],
        );
        addTearDown(scanner.dispose);
        addTearDown(container.dispose);
        addTearDown(catalog.releasePage);
        await tester.pumpWidget(
          UncontrolledProviderScope(
            container: container,
            child: const AmeApp(),
          ),
        );
        await tester.pumpAndSettle();

        await _openTile(tester, "location-1");
        expect(_viewedLocation(tester), "location-1");
        catalog.exposeNextPage = true;
        final refresh = container
            .read(libraryControllerProvider.notifier)
            .refreshCurrentQuery();
        await tester.pump();
        await tester.pump();
        final didRefresh = await refresh;
        final queryActivity = container
            .read(libraryControllerProvider)
            .queryActivity;
        expect(
          didRefresh,
          isTrue,
          reason: queryActivity is LibraryQueryFailed
              ? queryActivity.message
              : queryActivity.toString(),
        );
        await tester.pump();
        await tester.tap(find.byKey(const Key("viewer-next")));
        await tester.pump();
        expect(catalog.afterLoads, 1);
        expect(_viewedLocation(tester), "location-1");

        await tester.tap(find.byKey(const Key("viewer-back-button")));
        await tester.pump();
        expect(find.byType(LibraryImageViewer), findsNothing);
        await _openTile(tester, reopenSameAsset ? "location-1" : "location-0");
        if (clickAgain) {
          await tester.tap(find.byKey(const Key("viewer-next")));
          await tester.pump();
          expect(catalog.afterLoads, 1);
        }
        if (reopenSameAsset) {
          catalog.releasePage();
          await tester.pump();
          await tester.pump();
          expect(
            _viewedLocation(tester),
            clickAgain ? "location-2" : "location-1",
            reason:
                "Only a new-session command may consume the shared page result",
          );
        } else {
          await tester.tap(find.byKey(const Key("viewer-next")));
          await tester.pump();
          expect(
            _viewedLocation(tester),
            "location-1",
            reason:
                "New-session in-window navigation must not inherit the old pending flag",
          );
        }
      },
    );
  }
}

String _viewedLocation(WidgetTester tester) => tester
    .widget<LibraryImageViewer>(find.byType(LibraryImageViewer))
    .asset
    .locationId;

Future<void> _openTile(WidgetTester tester, String locationId) async {
  final tile = find.byWidgetPredicate(
    (widget) =>
        widget is LibraryPhotoTile && widget.asset.locationId == locationId,
  );
  expect(tile.hitTestable(), findsOneWidget);
  await tester.tapAt(tester.getRect(tile).topLeft + const Offset(16, 16));
  await tester.pump();
}

const _viewerQuery = LibraryGalleryQuery(
  sortKey: LibraryGallerySortKey.modifiedTime,
  sortDirection: LibraryGallerySortDirection.ascending,
);

class _HeldViewerPageCatalog extends RetainedScanCatalog {
  _HeldViewerPageCatalog(super.scanner);

  final Completer<LibrarySnapshot> _next = Completer();
  bool exposeNextPage = false;

  BigInt get _revision => exposeNextPage ? BigInt.two : BigInt.one;
  int get _itemCount => exposeNextPage ? 3 : 2;
  String get _queryId => "viewer-modified-time-ascending";

  List<LibraryRoot> get _roots => [
    LibraryRoot(
      id: otherPublishedRoot.id,
      path: otherPublishedRoot.path,
      displayPath: otherPublishedRoot.displayPath,
      activeScanId: otherPublishedRoot.activeScanId,
      createdUnixMs: otherPublishedRoot.createdUnixMs,
      assetCount: _itemCount,
      issueCount: 0,
      availability: otherPublishedRoot.availability,
    ),
  ];

  LibraryCatalogCursor _cursor(LibraryAsset last) => LibraryCatalogCursor(
    revision: _revision,
    queryId: _queryId,
    primaryMissing: false,
    primaryText: "",
    primaryNumber: last.modifiedUnixMs,
    rootId: last.rootId,
    locationId: last.locationId,
  );

  LibrarySnapshot firstPage() {
    final first = snapshot(_viewerQuery);
    final second = snapshot(_viewerQuery, offset: 1).assets.single;
    return LibrarySnapshot(
      roots: _roots,
      assets: [...first.assets, second],
      catalogPath: first.catalogPath,
      revision: _revision,
      queryId: _queryId,
      nextCursor: exposeNextPage ? _cursor(second) : null,
    );
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    expectSync(query, _viewerQuery);
    expectSync(maxItems, greaterThanOrEqualTo(2));
    expectSync(before, isNull);
    if (after != null) {
      final last = firstPage().assets.last;
      expectSync(exposeNextPage, isTrue);
      expectSync(after.revision, _revision);
      expectSync(after.queryId, _queryId);
      expectSync(after.primaryMissing, isFalse);
      expectSync(after.primaryText, "");
      expectSync(after.primaryNumber, last.modifiedUnixMs);
      expectSync(after.rootId, last.rootId);
      expectSync(after.locationId, last.locationId);
      afterLoads += 1;
      return _next.future;
    }
    return firstPage();
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    expectSync(query, _viewerQuery);
    final first = firstPage();
    return LibraryTimeline(
      revision: first.revision,
      queryId: first.queryId,
      totalItems: _itemCount,
      buckets: [
        LibraryTimeBucket(
          monthKey: "1970-01",
          itemCount: _itemCount,
          aspectRatioSum: _itemCount.toDouble(),
        ),
      ],
    );
  }

  void releasePage() {
    if (_next.isCompleted) {
      return;
    }
    final last = snapshot(_viewerQuery, offset: 2);
    _next.complete(
      LibrarySnapshot(
        roots: _roots,
        assets: last.assets,
        catalogPath: last.catalogPath,
        revision: _revision,
        queryId: _queryId,
        previousCursor: _cursor(last.assets.first),
      ),
    );
  }
}
