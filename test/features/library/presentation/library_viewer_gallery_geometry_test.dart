import "dart:async";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_image_viewer.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/library_query_snapshot_fixture.dart";
import "../support/retained_scan_fixture.dart";

void main() {
  for (final width in [1280.0, 1000.0]) {
    testWidgets("viewer preserves every gallery frame at width $width", (
      tester,
    ) async {
      tester.view.physicalSize = Size(width, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final scanner = RetainedScanScanner()..checkpoint = null;
      addTearDown(scanner.dispose);
      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(_galleryState()),
            libraryScannerProvider.overrideWithValue(scanner),
            libraryPreviewerProvider.overrideWithValue(_PendingPreviewer()),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();
      final wall = find.byKey(
        const Key("library-photo-wall"),
        skipOffstage: false,
      );
      final scrollable = find.descendant(
        of: wall,
        matching: find.byType(Scrollable, skipOffstage: false),
        skipOffstage: false,
      );
      final position = tester.state<ScrollableState>(scrollable).position;
      position.jumpTo(position.maxScrollExtent * 0.5);
      await tester.pump();
      await tester.pump();
      final wallRect = tester.getRect(wall);
      final tiles = find
          .byType(LibraryPhotoTile)
          .evaluate()
          .map((element) => element.widget as LibraryPhotoTile)
          .where((tile) {
            final rect = tester.getRect(
              find.byKey(ValueKey(tile.asset.locationId)),
            );
            return wallRect.contains(rect.center);
          })
          .toList();
      double distanceFromCenter(LibraryPhotoTile tile) =>
          (tester.getRect(find.byKey(ValueKey(tile.asset.locationId))).center -
                  wallRect.center)
              .distance;
      tiles.sort(
        (first, second) =>
            distanceFromCenter(first).compareTo(distanceFromCenter(second)),
      );
      final tile = tiles.first;
      final anchor = find.byKey(
        ValueKey(tile.asset.locationId),
        skipOffstage: false,
      );
      final before = tester.getRect(anchor);
      final pixels = position.pixels;
      for (var cycle = 0; cycle < 2; cycle++) {
        await tester.tapAt(before.center);
        for (var frame = 0; frame < 6; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
          expect(find.byType(LibraryImageViewer), findsOneWidget);
          expect(
            tester.getRect(wall),
            wallRect,
            reason: "hidden gallery frame $frame",
          );
          expect(tester.getRect(anchor), before);
          expect(position.pixels, pixels);
          expect(find.byKey(const Key("timeline-slider")), findsNothing);
        }
        await tester.tap(find.byKey(const Key("viewer-back-button")));
        for (var frame = 0; frame < 6; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
          expect(find.byType(LibraryImageViewer), findsNothing);
          expect(
            tester.getRect(wall),
            wallRect,
            reason: "returned gallery frame $frame",
          );
          expect(tester.getRect(anchor), before);
          expect(position.pixels, pixels);
          expect(find.byKey(const Key("timeline-slider")), findsOneWidget);
        }
      }
      await tester.tapAt(before.center);
      await tester.pump();
      tester.view.physicalSize = Size(width - 20, 800);
      for (var frame = 0; frame < 10; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
      }
      final resizedWall = tester.getRect(wall);
      final resizedAnchor = tester.getRect(anchor);
      final resizedPixels = position.pixels;
      expect(resizedWall.width, wallRect.width - 20);
      expect(resizedWall.overlaps(resizedAnchor), isTrue);
      await tester.tap(find.byKey(const Key("viewer-back-button")));
      for (var frame = 0; frame < 6; frame++) {
        await tester.pump(const Duration(milliseconds: 16));
        expect(tester.getRect(wall), resizedWall);
        expect(tester.getRect(anchor), resizedAnchor);
        expect(position.pixels, resizedPixels);
      }
      await tester.pumpWidget(const SizedBox.shrink());
    });
  }

  for (final closeBeforePublication in [false, true]) {
    testWidgets(
      "viewer return ${closeBeforePublication ? 'before' : 'after'} refresh keeps the wall anchor on screen",
      (tester) async {
        tester.view.physicalSize = const Size(1280, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final state = _galleryState();
        final catalog = _HeldViewerRefreshCatalog(state);
        final scanner = RetainedScanScanner()..checkpoint = null;
        addTearDown(scanner.dispose);
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              initialLibraryStateProvider.overrideWithValue(state),
              libraryCatalogProvider.overrideWithValue(catalog),
              libraryScannerProvider.overrideWithValue(scanner),
              libraryPreviewerProvider.overrideWithValue(_PendingPreviewer()),
            ],
            child: const AmeApp(),
          ),
        );
        await tester.pump();
        await tester.pump();
        final wall = find.byKey(
          const Key("library-photo-wall"),
          skipOffstage: false,
        );
        final scrollable = find.descendant(
          of: wall,
          matching: find.byType(Scrollable, skipOffstage: false),
          skipOffstage: false,
        );
        final position = tester.state<ScrollableState>(scrollable).position;
        position.jumpTo(position.maxScrollExtent * 0.5);
        await tester.pump();
        await tester.pump();
        final gallery = tester.widget<LibraryGalleryWall>(
          find.byType(LibraryGalleryWall),
        );
        final captured = gallery.positionResolver!.resolve(
          queryId: state.queryId,
          revision: state.catalogRevision,
          scrollOffset: position.pixels,
          viewportDimension: position.viewportDimension,
        )!;
        final anchor = find.byKey(
          ValueKey(captured.locationId),
          skipOffstage: false,
        );
        final originalRect = tester.getRect(anchor);
        final originalPixels = position.pixels;
        final wallRect = tester.getRect(wall);
        final openingTile = find.byType(LibraryPhotoTile).evaluate().firstWhere(
          (element) {
            final tile = element.widget as LibraryPhotoTile;
            return tile.asset.locationId != captured.locationId &&
                wallRect.contains(
                  tester
                      .getRect(find.byKey(ValueKey(tile.asset.locationId)))
                      .center,
                );
          },
        );
        final openingAsset = (openingTile.widget as LibraryPhotoTile).asset;
        await tester.tapAt(
          tester.getRect(find.byKey(ValueKey(openingAsset.locationId))).center,
        );
        await tester.pump();
        expect(find.byType(LibraryImageViewer), findsOneWidget);
        final container = ProviderScope.containerOf(
          tester.element(find.byType(AmeApp)),
        );
        final refresh = container
            .read(libraryControllerProvider.notifier)
            .refreshFromSynchronization(catalogRevision: BigInt.two);
        await tester.pump();
        expect(
          catalog.requestedAnchor?.requestedLocationId,
          openingAsset.locationId,
        );
        expect(
          catalog.requestedAnchor?.requestedLocationId,
          isNot(captured.locationId),
        );
        if (closeBeforePublication) {
          await tester.tap(find.byKey(const Key("viewer-back-button")));
          await tester.pump();
        }
        catalog.release.complete();
        await tester.pump();
        expect(await refresh, LibraryQueryUpdateOutcome.applied);
        for (var frame = 0; frame < 6; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
          expect(tester.getRect(anchor), originalRect);
          expect(position.pixels, originalPixels);
        }
        if (!closeBeforePublication) {
          await tester.tap(find.byKey(const Key("viewer-back-button")));
        }
        for (var frame = 0; frame < 6; frame++) {
          await tester.pump(const Duration(milliseconds: 16));
          expect(find.byType(LibraryImageViewer), findsNothing);
          expect(tester.getRect(anchor), originalRect);
          expect(position.pixels, originalPixels);
        }
        await tester.pumpWidget(const SizedBox.shrink());
      },
    );
  }
}

class _HeldViewerRefreshCatalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog {
  _HeldViewerRefreshCatalog(this.state);

  final LibraryState state;
  final release = Completer<void>();
  LibraryQueryAnchor? requestedAnchor;

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) async {
    requestedAnchor = anchor;
    await release.future;
    final index = state.assets.indexWhere(
      (asset) => asset.locationId == anchor?.requestedLocationId,
    );
    expectSync(index, greaterThanOrEqualTo(0));
    return LibraryQuerySnapshot(
      snapshot: LibrarySnapshot(
        catalogPath: state.catalogPath!,
        revision: BigInt.two,
        queryId: state.queryId,
        roots: state.roots,
        assets: state.assets,
        queryAnchorResolution: LibraryQueryAnchorResolution(
          requestedLocationId: anchor!.requestedLocationId,
          locationId: state.assets[index].locationId,
          ordinal: index,
          windowStartItemOffset: 0,
        ),
      ),
      timeline: await loadTimeline(query),
    );
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      LibraryTimeline(
        revision: BigInt.two,
        queryId: state.queryId,
        totalItems: state.assets.length,
        buckets: state.timeline!.buckets,
      );

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) => throw UnimplementedError();

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError();

  @override
  Future<bool> unregisterRoot(String rootId) => throw UnimplementedError();
}

LibraryState _galleryState() {
  final assets = List.generate(
    240,
    (index) => LibraryAsset(
      assetId: "asset-$index",
      locationId: "location-$index",
      rootId: "other",
      activeScanId: "published",
      sourcePath: "C:\\Fixture\\$index.png",
      displayPath: "C:\\Fixture\\$index.png",
      relativePath: "$index.png",
      previewPath: "",
      fileSize: BigInt.one,
      modifiedUnixMs: index < 120 ? 1788307200000 : 1330819200000,
      sourceRevision: null,
      sourceGeneration: BigInt.one,
      width: [640, 2160, 12000, 8000][index % 4],
      height: [480, 3840, 1500, 8000][index % 4],
      previewStatus: LibraryPreviewStatus.pending,
    ),
  );
  return LibraryState.fromSnapshot(
    LibrarySnapshot(
      roots: const [otherPublishedRoot],
      assets: assets,
      catalogPath: "fixture.sqlite3",
      revision: BigInt.one,
      queryId: "geometry",
    ),
  ).copyWith(
    timeline: LibraryTimeline(
      revision: BigInt.one,
      queryId: "geometry",
      totalItems: assets.length,
      buckets: const [
        LibraryTimeBucket(
          monthKey: "2026-09",
          itemCount: 120,
          aspectRatioSum: 326.875,
        ),
        LibraryTimeBucket(
          monthKey: "2012-03",
          itemCount: 120,
          aspectRatioSum: 326.875,
        ),
      ],
    ),
  );
}

class _PendingPreviewer implements LibraryPreviewer {
  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required String expectedRootId,
    required String expectedScanId,
    required LibrarySourceRevisionEvidence? expectedSourceRevision,
    required BigInt expectedSourceGeneration,
    required int previewEdge,
    bool force = false,
    Iterable<String> protectedLocationIds = const [],
  }) => Completer<LibraryAsset>().future;
}
