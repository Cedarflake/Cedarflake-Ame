import "dart:async";
import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/app/presentation/ame_menu.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_view_preferences.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/library_strings.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_header.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_snapshot.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/library_screen_fixture.dart";
import "support/library_screen_menu_fixture.dart";

void main() {
  testWidgets(
    "batches recovered preview dimensions into dynamic gallery geometry",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final assets = [
        for (var index = 0; index < 80; index++)
          LibraryAsset(
            assetId: "asset-$index",
            locationId: "location-$index",
            rootId: "root-1",
            activeScanId: "scan-1",
            sourceRevision: null,
            sourceGeneration: BigInt.one,
            sourcePath: "C:\\Pictures\\$index.jpg",
            displayPath: "C:\\Pictures\\$index.jpg",
            relativePath: "$index.jpg",
            previewPath: "",
            fileSize: BigInt.one,
            modifiedUnixMs: 1,
            width: 0,
            height: 0,
            previewStatus: LibraryPreviewStatus.pending,
          ),
      ];
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            displayPath: "C:\\Pictures",
            activeScanId: "scan-1",
            createdUnixMs: 1,
            assetCount: 80,
            issueCount: 0,
          ),
        ],
        assets: assets,
      );
      final initialState = LibraryState.fromSnapshot(snapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: "query-1",
          totalItems: assets.length,
          buckets: [
            LibraryTimeBucket(
              itemCount: assets.length,
              aspectRatioSum: assets.length.toDouble(),
            ),
          ],
        ),
      );
      final previewer = _ControlledLibraryPreviewer(assets);
      final manifest = _unknownDimensionManifest(assets);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryPreviewerProvider.overrideWithValue(previewer),
            libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
              _FixedLayoutManifestLoader(manifest),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();

      final galleryWall = find.byKey(const Key("library-photo-wall"));
      final scrollPosition = galleryScrollPosition(tester);
      final initialLayoutSnapshot = LibraryGalleryLayoutSnapshot.build(
        manifest: manifest,
        availableWidth: tester.getSize(galleryWall).width - 40,
        thumbnailSize: GalleryThumbnailSize.medium,
        sortKey: LibraryGallerySortKey.captureTime,
      );
      final initialVisibleEnd = initialLayoutSnapshot.metrics
          .rowEndGlobalItemIndexExclusive(
            initialLayoutSnapshot.metrics.itemIndexForScrollOffset(
              scrollPosition.pixels + scrollPosition.viewportDimension,
            ),
          )!;
      expect(previewer.requests, hasLength(2));
      final firstLocation = previewer.requests[0];
      final secondLocation = previewer.requests[1];
      final firstTile = find.byKey(ValueKey(firstLocation));
      final secondTile = find.byKey(ValueKey(secondLocation));
      expect(firstTile, findsOneWidget);
      expect(secondTile, findsOneWidget);
      final firstWidthBefore = tester.getSize(firstTile).width;
      final secondWidthBefore = tester.getSize(secondTile).width;

      previewer.succeed(firstLocation, width: 800, height: 400);
      previewer.succeed(secondLocation, width: 200, height: 400);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 159));
      expect(tester.getSize(firstTile).width, firstWidthBefore);
      expect(tester.getSize(secondTile).width, secondWidthBefore);

      expect(previewer.requests, hasLength(4));
      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      await tester.pump();
      await tester.pump();
      expect(tester.getSize(firstTile).width, greaterThan(firstWidthBefore));
      expect(tester.getSize(secondTile).width, lessThan(secondWidthBefore));

      final gesture = await tester.startGesture(tester.getCenter(galleryWall));
      await gesture.moveBy(const Offset(0, -24));
      await tester.pump();
      final thirdLocation = previewer.requests[2];
      final fourthLocation = previewer.requests[3];
      final thirdTile = find.byKey(ValueKey(thirdLocation));
      final fourthTile = find.byKey(ValueKey(fourthLocation));
      final thirdWidthBefore = tester.getSize(thirdTile).width;
      final fourthWidthBefore = tester.getSize(fourthTile).width;
      previewer.succeed(thirdLocation, width: 800, height: 400);
      previewer.succeed(fourthLocation, width: 200, height: 400);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 1700));
      expect(tester.getSize(thirdTile).width, thirdWidthBefore);
      expect(tester.getSize(fourthTile).width, fourthWidthBefore);

      await gesture.up();
      await tester.pump();
      expect(tester.getSize(thirdTile).width, thirdWidthBefore);
      expect(tester.getSize(fourthTile).width, fourthWidthBefore);
      for (
        var attempt = 0;
        attempt < 20 && scrollPosition.isScrollingNotifier.value;
        attempt += 1
      ) {
        await tester.pump(const Duration(milliseconds: 50));
        expect(tester.getSize(thirdTile).width, thirdWidthBefore);
        expect(tester.getSize(fourthTile).width, fourthWidthBefore);
      }
      expect(scrollPosition.isScrollingNotifier.value, isFalse);
      await tester.pump(const Duration(milliseconds: 159));
      expect(tester.getSize(thirdTile).width, thirdWidthBefore);
      expect(tester.getSize(fourthTile).width, fourthWidthBefore);

      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      await tester.pump();
      await tester.pump();

      expect(tester.getSize(thirdTile).width, greaterThan(thirdWidthBefore));
      expect(tester.getSize(fourthTile).width, lessThan(fourthWidthBefore));

      final extentAfterVisibleRecovery = scrollPosition.maxScrollExtent;
      final completedLocations = {
        firstLocation,
        secondLocation,
        thirdLocation,
        fourthLocation,
      };
      String? deferredLocation;
      var requestCursor = 4;
      for (
        var attempt = 0;
        attempt < 80 && deferredLocation == null;
        attempt++
      ) {
        while (requestCursor < previewer.requests.length &&
            deferredLocation == null) {
          final locationId = previewer.requests[requestCursor];
          requestCursor += 1;
          if (!completedLocations.add(locationId)) {
            continue;
          }
          final itemIndex = int.parse(locationId.substring("location-".length));
          if (itemIndex < initialVisibleEnd) {
            previewer.succeed(locationId, width: 1000, height: 1000);
          } else {
            deferredLocation = locationId;
            previewer.succeed(locationId, width: 5000, height: 1000);
          }
        }
        await tester.pump();
        await tester.pump();
      }
      expect(deferredLocation, isNotNull);

      await tester.pump(const Duration(milliseconds: 1700));
      await tester.pump();
      await tester.pump();
      expect(
        scrollPosition.maxScrollExtent,
        closeTo(extentAfterVisibleRecovery, 0.01),
        reason:
            "reflow-exposed rows must remain outside the frozen recovery epoch",
      );
    },
  );

  testWidgets("later wheel position survives two recovered dimension batches", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final assets = [
      for (var index = 0; index < 80; index++)
        LibraryAsset(
          assetId: "asset-$index",
          locationId: "location-$index",
          rootId: "root-1",
          activeScanId: "scan-1",
          sourceRevision: null,
          sourceGeneration: BigInt.one,
          sourcePath: "C:\\Pictures\\$index.jpg",
          displayPath: "C:\\Pictures\\$index.jpg",
          relativePath: "$index.jpg",
          previewPath: "",
          fileSize: BigInt.one,
          modifiedUnixMs: 1,
          width: 0,
          height: 0,
          previewStatus: LibraryPreviewStatus.pending,
        ),
    ];
    final initialState = populatedScreenState(
      totalItems: assets.length,
    ).copyWith(assets: assets);
    final previewer = _ControlledLibraryPreviewer(assets);
    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryPreviewerProvider.overrideWithValue(previewer),
          libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
            _FixedLayoutManifestLoader(_unknownDimensionManifest(assets)),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();
    await tester.pump();
    expect(previewer.requests, hasLength(2));
    final completed = <String>{};
    for (final locationId in previewer.requests.toList()) {
      completed.add(locationId);
      previewer.succeed(locationId, width: 2000, height: 1000);
    }
    await tester.pump();
    await tester.pump(const Duration(milliseconds: 160));
    var wall = tester.widget<LibraryGalleryWall>(
      find.byType(LibraryGalleryWall),
    );
    expect(wall.layoutTransition, isNotNull);
    final oldAnchor = wall.layoutTransition!.position;
    final viewport = find.byKey(const Key("library-photo-wall"));
    final center = tester.getCenter(viewport);
    final scrollPosition = galleryScrollPosition(tester);
    await tester.sendEventToBinding(
      PointerScrollEvent(position: center, scrollDelta: const Offset(0, 480)),
    );
    expect(scrollPosition.pixels, 456);
    var expected = wall.positionResolver!.resolve(
      queryId: initialState.queryId,
      revision: initialState.catalogRevision,
      scrollOffset: scrollPosition.pixels,
      viewportDimension: scrollPosition.viewportDimension,
    )!;
    expect(expected.locationId, isNot(oldAnchor.locationId));
    await tester.pump();
    await tester.pump();

    void expectCurrentAnchor() {
      final card = find.byKey(ValueKey(expected.locationId));
      expect(card, findsOneWidget);
      final bounds = tester.getRect(card);
      expect(
        bounds.top + bounds.height * expected.itemFraction,
        closeTo(center.dy, 2),
      );
    }

    expectCurrentAnchor();
    wall = tester.widget<LibraryGalleryWall>(find.byType(LibraryGalleryWall));
    expect(wall.layoutTransition, isNull);
    await tester.pump(const Duration(milliseconds: 100));
    expected = wall.positionResolver!.resolve(
      queryId: initialState.queryId,
      revision: initialState.catalogRevision,
      scrollOffset: scrollPosition.pixels,
      viewportDimension: scrollPosition.viewportDimension,
    )!;
    await tester.pump();
    expectCurrentAnchor();
    Map<String, double> visibleCardWidths() => {
      for (final asset in assets)
        if (find.byKey(ValueKey(asset.locationId)).evaluate().length == 1)
          asset.locationId: tester
              .getSize(find.byKey(ValueKey(asset.locationId)))
              .width,
    };

    final firstWidths = visibleCardWidths();
    var completedVisible = false;
    for (var attempt = 0; attempt < 10 && !completedVisible; attempt++) {
      for (final locationId in previewer.requests.toList()) {
        if (!completed.add(locationId)) {
          continue;
        }
        final tile = find.byKey(ValueKey(locationId));
        if (tile.evaluate().isNotEmpty &&
            tester.getRect(tile).overlaps(tester.getRect(viewport))) {
          completedVisible = true;
        }
        previewer.succeed(locationId, width: 3000, height: 1000);
      }
      await tester.pump();
      await tester.pump();
    }
    expect(completedVisible, isTrue);
    await tester.pump(const Duration(milliseconds: 160));
    await tester.pump();
    await tester.pump();
    final secondWidths = visibleCardWidths();
    expect(
      firstWidths.entries.any(
        (entry) =>
            secondWidths.containsKey(entry.key) &&
            (secondWidths[entry.key]! - entry.value).abs() > 0.01,
      ),
      isTrue,
      reason: "recovered dimensions must change rendered card geometry",
    );
    expectCurrentAnchor();
    await tester.pumpWidget(const SizedBox());
  });

  testWidgets(
    "publishes recovered dimensions after a timeline jump without scrolling",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final assets = [
        for (var index = 0; index < 80; index++)
          LibraryAsset(
            assetId: "asset-$index",
            locationId: "location-$index",
            rootId: "root-1",
            activeScanId: "scan-1",
            sourceRevision: null,
            sourceGeneration: BigInt.one,
            sourcePath: "C:\\Pictures\\$index.jpg",
            displayPath: "C:\\Pictures\\$index.jpg",
            relativePath: "$index.jpg",
            previewPath: "",
            fileSize: BigInt.one,
            modifiedUnixMs: 1,
            width: 0,
            height: 0,
            previewStatus: LibraryPreviewStatus.pending,
          ),
      ];
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            displayPath: "C:\\Pictures",
            activeScanId: "scan-1",
            createdUnixMs: 1,
            assetCount: 80,
            issueCount: 0,
          ),
        ],
        assets: assets,
      );
      final initialState = LibraryState.fromSnapshot(snapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: "query-1",
          totalItems: assets.length,
          buckets: [
            LibraryTimeBucket(
              itemCount: assets.length,
              aspectRatioSum: assets.length.toDouble(),
            ),
          ],
        ),
      );
      final previewer = _ControlledLibraryPreviewer(assets);

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryPreviewerProvider.overrideWithValue(previewer),
            libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
              _FixedLayoutManifestLoader(_unknownDimensionManifest(assets)),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();
      expect(previewer.requests, hasLength(2));
      final scrollPosition = galleryScrollPosition(tester);
      final initialPixels = scrollPosition.pixels;

      final slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(0.8);
      slider.onChanged?.call(0.8);
      slider.onChangeEnd?.call(0.8);
      for (var attempt = 0; attempt < 12; attempt += 1) {
        await tester.pump(const Duration(milliseconds: 16));
        if (scrollPosition.pixels > initialPixels + 100) {
          break;
        }
      }

      expect(scrollPosition.pixels, greaterThan(initialPixels + 100));
      expect(
        previewer.requests,
        hasLength(2),
        reason: "obsolete active previews must not exceed the hard limit",
      );
      for (final locationId in previewer.requests.toList()) {
        previewer.succeed(locationId, width: 400, height: 400);
      }
      for (var attempt = 0; attempt < 12; attempt += 1) {
        await tester.pump(const Duration(milliseconds: 16));
        if (previewer.requests.length > 2) {
          break;
        }
      }
      expect(
        previewer.requests.length,
        greaterThan(2),
        reason:
            "a timeline jump must replace the visible preview demand when "
            "the bounded queue releases capacity",
      );

      final targetLocation = previewer.requests.firstWhere(
        (locationId) =>
            int.parse(locationId.substring("location-".length)) >= 20,
        orElse: () => throw StateError(
          "Timeline jump did not request a deep preview: "
          "${previewer.requests.join(', ')}",
        ),
      );
      final targetTile = find.byKey(ValueKey(targetLocation));
      expect(targetTile, findsOneWidget);
      final widthBefore = tester.getSize(targetTile).width;

      previewer.succeed(targetLocation, width: 800, height: 400);
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 159));
      expect(tester.getSize(targetTile).width, widthBefore);

      await tester.pump(const Duration(milliseconds: 1));
      await tester.pump();
      await tester.pump();
      await tester.pump();
      expect(tester.getSize(targetTile).width, greaterThan(widthBefore));
    },
  );

  testWidgets(
    "freezes the native offset when layout controls change geometry",
    (tester) async {
      final previousHitTestWarningPolicy =
          WidgetController.hitTestWarningShouldBeFatal;
      WidgetController.hitTestWarningShouldBeFatal = true;
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      addTearDown(() {
        WidgetController.hitTestWarningShouldBeFatal =
            previousHitTestWarningPolicy;
      });
      final assets = [
        for (var index = 0; index < 120; index++)
          galleryAsset(
            id: "geometry-anchor-$index",
            captureLocalTime: "2026-08-05T00:00:00.000000000",
          ),
      ];
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "geometry-anchor-initial",
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: "C:\\Pictures",
            displayPath: "C:\\Pictures",
            createdUnixMs: 1,
            assetCount: 120,
            issueCount: 0,
          ),
        ],
        assets: assets,
      );
      final initialState = LibraryState.fromSnapshot(snapshot).copyWith(
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: snapshot.queryId,
          totalItems: assets.length,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2026-08",
              itemCount: 120,
              aspectRatioSum: 160,
            ),
          ],
        ),
      );

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            initialLibraryViewPreferencesProvider.overrideWithValue(
              const LibraryViewPreferences(
                layoutShape: GalleryLayoutShape.equalHeight,
                thumbnailSize: GalleryThumbnailSize.medium,
              ),
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();

      final wall = find.byKey(const Key("library-photo-wall"));
      final scrollPosition = galleryScrollPosition(tester);
      scrollPosition.jumpTo(scrollPosition.maxScrollExtent * 0.55);
      final wallTop = tester.getTopLeft(wall).dy;

      final squareItem = await _openLayoutMenu(tester, LibraryStrings.square);
      final squareAnchor = _viewportCenterRowAnchor(tester, assets);
      await selectPopupMenuItem(tester, squareItem);
      await _pumpUntilGalleryGeometry(
        tester,
        previousShape: GalleryLayoutShape.equalHeight,
        expectedShape: GalleryLayoutShape.square,
        expectedSize: GalleryThumbnailSize.medium,
      );
      final squareFinder = find.byKey(ValueKey(squareAnchor.locationId));
      expect(squareFinder, findsOneWidget);
      final squareRect = tester.getRect(squareFinder);
      final squareTop = tester.getTopLeft(squareFinder).dy - wallTop;
      expect(
        squareRect.top,
        closeTo(
          squareAnchor.viewportCenterY -
              squareRect.height * squareAnchor.itemFraction,
          2,
        ),
      );
      final squarePixels = scrollPosition.pixels;
      await tester.pump();
      expect(scrollPosition.pixels, closeTo(squarePixels, 0.01));
      expect(
        tester.getTopLeft(squareFinder).dy - wallTop,
        closeTo(squareTop, 0.01),
      );

      final largeItem = await _openLayoutMenu(tester, LibraryStrings.large);
      final largeAnchor = _viewportCenterRowAnchor(tester, assets);
      await selectPopupMenuItem(tester, largeItem);
      await _pumpUntilGalleryGeometry(
        tester,
        previousShape: GalleryLayoutShape.square,
        expectedShape: GalleryLayoutShape.square,
        previousSize: GalleryThumbnailSize.medium,
        expectedSize: GalleryThumbnailSize.large,
      );
      final largeFinder = find.byKey(ValueKey(largeAnchor.locationId));
      expect(largeFinder, findsOneWidget);
      final largeRect = tester.getRect(largeFinder);
      final largeTop = tester.getTopLeft(largeFinder).dy - wallTop;
      expect(
        largeRect.top,
        closeTo(
          largeAnchor.viewportCenterY -
              largeRect.height * largeAnchor.itemFraction,
          2,
        ),
      );
      final largePixels = scrollPosition.pixels;
      await tester.pump();
      expect(scrollPosition.pixels, closeTo(largePixels, 0.01));
      expect(
        tester.getTopLeft(largeFinder).dy - wallTop,
        closeTo(largeTop, 0.01),
      );
    },
  );
}

Future<Finder> _openLayoutMenu(WidgetTester tester, String expectedItem) async {
  final button = find.byKey(const Key("library-layout-menu"));
  await tester.ensureVisible(button);
  await tester.tap(button);
  await tester.pump();
  await tester.pump(AmeMenuTransition.duration);
  await tester.pump();
  activeMenuItem(tester, expectedItem);
  return menuChoiceTarget(expectedItem);
}

({String locationId, double itemFraction, double viewportCenterY})
_viewportCenterRowAnchor(WidgetTester tester, List<LibraryAsset> assets) {
  final scrollable = find.descendant(
    of: find.byKey(const Key("library-photo-wall")),
    matching: find.byType(Scrollable),
  );
  final viewportRect = tester.getRect(scrollable);
  final viewportCenterY = viewportRect.top + viewportRect.height * 0.5;
  final candidates = <({String locationId, Rect rect})>[];
  for (final asset in assets) {
    final finder = find.byKey(ValueKey(asset.locationId));
    if (finder.evaluate().length == 1) {
      final rect = tester.getRect(finder);
      if (rect.top <= viewportCenterY &&
          viewportCenterY < rect.bottom + LibraryGalleryLayoutEntry.spacing) {
        candidates.add((locationId: asset.locationId, rect: rect));
      }
    }
  }
  expect(candidates, isNotEmpty);
  final rowTop = candidates
      .map((candidate) => candidate.rect.top)
      .reduce((first, second) => first > second ? first : second);
  final viewportCenterX = viewportRect.left + viewportRect.width * 0.5;
  final row =
      candidates
          .where((candidate) => (candidate.rect.top - rowTop).abs() < 0.01)
          .toList(growable: false)
        ..sort(
          (first, second) => (first.rect.center.dx - viewportCenterX)
              .abs()
              .compareTo((second.rect.center.dx - viewportCenterX).abs()),
        );
  final anchor = row.first;
  return (
    locationId: anchor.locationId,
    itemFraction: ((viewportCenterY - anchor.rect.top) / anchor.rect.height)
        .clamp(0.0, 1.0)
        .toDouble(),
    viewportCenterY: viewportCenterY,
  );
}

Future<void> _pumpUntilGalleryGeometry(
  WidgetTester tester, {
  required GalleryLayoutShape previousShape,
  required GalleryLayoutShape expectedShape,
  GalleryThumbnailSize? previousSize,
  required GalleryThumbnailSize expectedSize,
}) async {
  const maximumFrames = 3;
  for (var frame = 0; frame < maximumFrames; frame += 1) {
    await tester.pump();
    final header = tester.widget<LibraryGalleryHeader>(
      find.byType(LibraryGalleryHeader),
    );
    if (header.layoutShape == expectedShape &&
        header.thumbnailSize == expectedSize) {
      return;
    }
    expect(header.layoutShape, previousShape);
    if (previousSize != null) {
      expect(header.thumbnailSize, previousSize);
    }
  }
  fail("Gallery geometry was not published within $maximumFrames frames");
}

LibraryGalleryLayoutManifest _unknownDimensionManifest(
  List<LibraryAsset> assets,
) {
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: assets.length,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: assets.length,
      startOrdinal: 0,
      locationIds: [for (final asset in assets) asset.locationId],
      aspectRatioMilli: Uint16List.fromList(List.filled(assets.length, 1000)),
      dateGroupIndices: Uint16List(assets.length),
      dateGroups: const [null],
      flags: Uint8List(assets.length),
    ),
  );
  return builder.build();
}

class _FixedLayoutManifestLoader implements LibraryGalleryLayoutManifestLoader {
  const _FixedLayoutManifestLoader(this.manifest);

  final LibraryGalleryLayoutManifest manifest;

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) async {
    return manifest;
  }
}

class _ControlledLibraryPreviewer implements LibraryPreviewer {
  _ControlledLibraryPreviewer(Iterable<LibraryAsset> assets)
    : _assets = {for (final asset in assets) asset.locationId: asset};

  final Map<String, LibraryAsset> _assets;
  final Map<String, Completer<LibraryAsset>> _completers = {};
  final List<String> requests = [];

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
  }) {
    requests.add(locationId);
    return (_completers[locationId] ??= Completer<LibraryAsset>()).future;
  }

  void succeed(String locationId, {required int width, required int height}) {
    final asset = _assets[locationId];
    if (asset == null) {
      throw StateError("Unknown preview asset $locationId");
    }
    _completers[locationId]?.complete(
      asset.withPreview(
        previewPath: "C:\\AmeCache\\$locationId.jpg",
        width: width,
        height: height,
        previewStatus: LibraryPreviewStatus.ready,
      ),
    );
  }
}
