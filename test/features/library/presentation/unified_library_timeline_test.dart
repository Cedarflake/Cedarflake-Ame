import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_extent_sliver.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "support/library_screen_fixture.dart";

void main() {
  testWidgets("groups server-ordered assets under capture and fallback dates", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final capture = LibraryCaptureTimeEvidence(
      localTime: "2025-08-07T12:34:56.000000000",
      source: LibraryCaptureTimeSource.exifDateTimeOriginal,
      rawValue: "2025:08:07 12:34:56",
    );
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
          assetCount: 3,
          issueCount: 0,
        ),
      ],
      assets: [
        LibraryAsset(
          assetId: "asset-1",
          locationId: "location-1",
          rootId: "root-1",
          activeScanId: "scan-1",
          sourceRevision: null,
          sourceGeneration: BigInt.one,
          sourcePath: "C:\\Pictures\\one.png",
          displayPath: "C:\\Pictures\\one.png",
          relativePath: "one.png",
          previewPath: "C:\\Missing\\one.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: 3,
          width: 1,
          height: 1,
          previewStatus: LibraryPreviewStatus.pending,
          captureTime: capture,
        ),
        LibraryAsset(
          assetId: "asset-2",
          locationId: "location-2",
          rootId: "root-1",
          activeScanId: "scan-1",
          sourceRevision: null,
          sourceGeneration: BigInt.one,
          sourcePath: "C:\\Pictures\\two.png",
          displayPath: "C:\\Pictures\\two.png",
          relativePath: "two.png",
          previewPath: "C:\\Missing\\two.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: 2,
          width: 1,
          height: 1,
          captureTime: capture,
        ),
        LibraryAsset(
          assetId: "asset-3",
          locationId: "location-3",
          rootId: "root-1",
          activeScanId: "scan-1",
          sourceRevision: null,
          sourceGeneration: BigInt.one,
          sourcePath: "C:\\Pictures\\fallback.png",
          displayPath: "C:\\Pictures\\fallback.png",
          relativePath: "fallback.png",
          previewPath: "C:\\Missing\\fallback.jpg",
          fileSize: BigInt.one,
          createdUnixMs: DateTime(2024, 3, 4).millisecondsSinceEpoch,
          modifiedUnixMs: DateTime(2026, 5, 6).millisecondsSinceEpoch,
          width: 1,
          height: 1,
        ),
      ],
    );

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(snapshot),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pump();

    final datedHeader = find.byKey(const Key("gallery-date-2025-08-07"));
    final fallbackHeader = find.byKey(const Key("gallery-date-2024-03-04"));
    expect(datedHeader, findsOneWidget);
    expect(fallbackHeader, findsOneWidget);
    expect(find.byKey(const Key("gallery-date-unknown")), findsNothing);
    expect(
      tester.getTopLeft(datedHeader).dy,
      lessThan(tester.getTopLeft(fallbackHeader).dy),
    );
    expect(find.byKey(const ValueKey("location-1")), findsOneWidget);
    expect(find.byKey(const ValueKey("location-2")), findsOneWidget);
    expect(find.byKey(const ValueKey("location-3")), findsOneWidget);
    final gallerySliver = tester.widget<LibraryExactExtentSliver>(
      find.byType(LibraryExactExtentSliver),
    );
    expect(
      (gallerySliver.delegate as SliverChildBuilderDelegate).addSemanticIndexes,
      isFalse,
    );
    expect(
      find.descendant(
        of: find.byKey(const ValueKey("location-1")),
        matching: find.byType(CircularProgressIndicator),
      ),
      findsNothing,
    );
  });

  testWidgets("renders the scroll-derived Material timeline", (tester) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(
            populatedScreenState(totalItems: 3).copyWith(
              timeline: LibraryTimeline(
                revision: BigInt.one,
                queryId: "query-1",
                totalItems: 3,
                buckets: const [
                  LibraryTimeBucket(
                    monthKey: "2026-08",
                    itemCount: 1,
                    aspectRatioSum: 1,
                  ),
                  LibraryTimeBucket(
                    monthKey: "2025-01",
                    itemCount: 2,
                    aspectRatioSum: 7,
                  ),
                ],
              ),
            ),
          ),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    expect(find.byKey(const Key("library-time-rail")), findsOneWidget);
    expect(find.byKey(const Key("timeline-slider")), findsOneWidget);
    expect(
      find.byKey(const Key("current-month-native-scrollbar")),
      findsNothing,
    );
    expect(find.byKey(const ValueKey("time-label-2026-08")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-marker-2026-08")), findsOneWidget);
    expect(find.byKey(const ValueKey("time-marker-unknown")), findsNothing);
    expect(
      find.descendant(
        of: find.byKey(const Key("library-time-rail")),
        matching: find.byType(MenuAnchor),
      ),
      findsNothing,
    );
  });

  testWidgets(
    "preserves the first timeline target when the manifest takes ownership",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final initialState = populatedScreenState(totalItems: 120);
      final snapshot = LibrarySnapshot(
        catalogPath: initialState.catalogPath ?? "",
        revision: initialState.catalogRevision ?? BigInt.zero,
        queryId: initialState.queryId,
        roots: initialState.roots,
        assets: initialState.assets,
      );
      final catalog = RecordingQueryCatalog(snapshot);
      final manifestLoader = ControlledLayoutManifestLoader();

      await tester.pumpWidget(
        ProviderScope(
          overrides: [
            initialLibraryStateProvider.overrideWithValue(initialState),
            libraryCatalogProvider.overrideWithValue(catalog),
            libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
              manifestLoader,
            ),
          ],
          child: const AmeApp(),
        ),
      );
      await tester.pump();
      await tester.pump();

      var slider = tester.widget<Slider>(
        find.byKey(const Key("timeline-slider")),
      );
      slider.onChangeStart?.call(slider.value);
      slider.onChanged?.call(0.25);
      slider.onChangeEnd?.call(0.25);
      await tester.pump();
      await tester.pumpAndSettle();

      expect(catalog.timeAnchors, hasLength(1));
      final selectedOffset = catalog.timeAnchors.single.itemOffset;
      expect(selectedOffset, greaterThan(0));

      manifestLoader.complete(
        _queryWideManifest(
          totalItems: 120,
          loadedItemIndex: selectedOffset,
          loadedLocationId: initialState.assets.single.locationId,
        ),
      );
      await tester.pump();
      await tester.pump(const Duration(milliseconds: 250));

      expect(galleryScrollPosition(tester).pixels, greaterThan(0));
      expect(find.byKey(const Key("library-top-loading")), findsNothing);
      expect(
        find.byKey(ValueKey(initialState.assets.single.locationId)),
        findsOneWidget,
      );
      expect(
        tester.widget<Slider>(find.byKey(const Key("timeline-slider"))).value,
        greaterThan(0),
      );
    },
  );

  testWidgets("keeps the single time rail synchronized with gallery scroll", (
    tester,
  ) async {
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final assets = [
      for (var index = 0; index < 120; index++)
        LibraryAsset(
          assetId: "asset-$index",
          locationId: "location-$index",
          rootId: "root-1",
          activeScanId: "scan-1",
          sourceRevision: null,
          sourceGeneration: BigInt.one,
          sourcePath: "C:\\Pictures\\$index.png",
          displayPath: "C:\\Pictures\\$index.png",
          relativePath: "$index.png",
          previewPath: "C:\\Missing\\$index.jpg",
          fileSize: BigInt.one,
          modifiedUnixMs: index,
          width: 4,
          height: 3,
          captureTime: const LibraryCaptureTimeEvidence(
            localTime: "2026-08-05T12:00:00.000000000",
            source: LibraryCaptureTimeSource.exifDateTimeOriginal,
            rawValue: "2026:08:05 12:00:00",
          ),
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
          assetCount: 120,
          issueCount: 0,
        ),
      ],
      assets: assets,
    );
    final state = LibraryState.fromSnapshot(snapshot).copyWith(
      timeline: LibraryTimeline(
        revision: BigInt.one,
        queryId: "query-1",
        totalItems: 120,
        buckets: const [
          LibraryTimeBucket(
            monthKey: "2026-08",
            itemCount: 120,
            aspectRatioSum: 160,
          ),
        ],
      ),
    );
    final catalog = RecordingQueryCatalog(snapshot);

    await tester.pumpWidget(
      ProviderScope(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(state),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
        child: const AmeApp(),
      ),
    );
    await tester.pumpAndSettle();

    final timelineSlider = find.byKey(const Key("timeline-slider"));
    expect(tester.widget<Slider>(timelineSlider).value, 1);
    final galleryScrollable = find.descendant(
      of: find.byKey(const Key("library-photo-wall")),
      matching: find.byType(Scrollable),
    );
    final galleryPosition = tester
        .state<ScrollableState>(galleryScrollable)
        .position;
    expect(galleryPosition.pixels, 0);

    expect(
      find.byKey(const Key("current-month-native-scrollbar")),
      findsNothing,
    );
    await tester.drag(
      find.byKey(const Key("library-photo-wall")),
      const Offset(0, -480),
    );
    await tester.pumpAndSettle();

    expect(galleryPosition.pixels, greaterThan(0));
    expect(tester.widget<Slider>(timelineSlider).value, lessThan(1));
    expect(catalog.timeAnchors, isEmpty);

    final beforeSlider = galleryPosition.pixels;
    tester.widget<Slider>(timelineSlider).onChanged?.call(0.25);
    await tester.pump();
    expect(galleryPosition.pixels, greaterThan(beforeSlider));
    expect(tester.widget<Slider>(timelineSlider).value, closeTo(0.25, 0.001));

    final beforeArrow = galleryPosition.pixels;
    await tester.tap(find.byKey(const Key("timeline-previous")));
    await tester.pumpAndSettle();
    expect(galleryPosition.pixels, lessThan(beforeArrow));
  });

  testWidgets(
    "loads the previous window from a timeline anchor without visual jumping",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      const roots = [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 90,
          issueCount: 0,
        ),
      ];
      final previousCursor = LibraryCatalogCursor(
        revision: BigInt.one,
        queryId: "query-1",
        primaryMissing: false,
        primaryText: "2025-01-01T00:00:00.000000000",
        primaryNumber: 0,
        rootId: "root-1",
        locationId: "anchor-0",
      );
      final anchoredAssets = [
        for (var index = 0; index < 60; index++)
          galleryAsset(
            id: "anchor-$index",
            captureLocalTime: "2025-01-01T00:00:00.000000000",
          ),
      ];
      final previousAssets = [
        for (var index = 0; index < 30; index++)
          galleryAsset(
            id: "previous-$index",
            captureLocalTime: "2026-08-01T00:00:00.000000000",
          ),
      ];
      final anchoredSnapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: roots,
        assets: anchoredAssets,
        previousCursor: previousCursor,
      );
      final previousSnapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: "query-1",
        roots: roots,
        assets: previousAssets,
      );
      final initialState = LibraryState.fromSnapshot(anchoredSnapshot).copyWith(
        windowStartItemOffset: 30,
        timeline: LibraryTimeline(
          revision: BigInt.one,
          queryId: "query-1",
          totalItems: 90,
          buckets: const [
            LibraryTimeBucket(
              monthKey: "2026-08",
              itemCount: 30,
              aspectRatioSum: 40,
            ),
            LibraryTimeBucket(
              monthKey: "2025-01",
              itemCount: 60,
              aspectRatioSum: 80,
            ),
          ],
        ),
      );
      final catalog = RecordingQueryCatalog(
        anchoredSnapshot,
        previousSnapshot: previousSnapshot,
      );
      final container = ProviderContainer(
        overrides: [
          initialLibraryStateProvider.overrideWithValue(initialState),
          libraryCatalogProvider.overrideWithValue(catalog),
        ],
      );
      addTearDown(container.dispose);

      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await tester.pumpAndSettle();

      final photoWall = find.byKey(const Key("library-photo-wall"));
      final scrollable = find.descendant(
        of: photoWall,
        matching: find.byType(Scrollable),
      );
      final position = tester.state<ScrollableState>(scrollable).position;
      final maximumBefore = position.maxScrollExtent;
      final anchorTopBefore = tester.getTopLeft(
        find.byKey(const ValueKey("anchor-0")),
      );
      expect(position.pixels, 0);

      tester.view.physicalSize = const Size(1180, 760);
      await tester.pump();
      await tester.pump();

      expect(tester.takeException(), isNull);
      expect(find.byKey(const ValueKey("anchor-0")), findsOneWidget);
      expect(catalog.beforeCursors, isEmpty);

      await tester.sendEventToBinding(
        PointerScrollEvent(
          position: tester.getCenter(photoWall),
          scrollDelta: const Offset(0, -240),
        ),
      );
      await tester.pumpAndSettle();

      final state = container.read(libraryControllerProvider);
      final anchorAfter = find.byKey(const ValueKey("anchor-0"));
      expect(
        anchorAfter,
        findsOneWidget,
        reason:
            "pixels=${position.pixels}, max=${position.maxScrollExtent}, "
            "oldMax=$maximumBefore, assets=${state.assets.length}, "
            "before=${catalog.beforeCursors.length}",
      );
      final anchorTopAfter = tester.getTopLeft(anchorAfter);
      expect(catalog.beforeCursors, [same(previousCursor)]);
      expect(state.assets, hasLength(90));
      expect(state.assets.first.locationId, "previous-0");
      expect(state.previousCursor, isNull);
      expect(position.pixels, greaterThan(0));
      expect(anchorTopAfter.dy, closeTo(anchorTopBefore.dy, 1));
    },
  );
}

LibraryGalleryLayoutManifest _queryWideManifest({
  required int totalItems,
  required int loadedItemIndex,
  required String loadedLocationId,
}) {
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: BigInt.one,
    queryId: "query-1",
    totalItems: totalItems,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: totalItems,
      startOrdinal: 0,
      locationIds: [
        for (var index = 0; index < totalItems; index++)
          index == loadedItemIndex
              ? loadedLocationId
              : "manifest-location-$index",
      ],
      aspectRatioMilli: Uint16List.fromList(List.filled(totalItems, 1333)),
      dateGroupIndices: Uint16List(totalItems),
      dateGroups: const ["2026-08-05"],
      flags: Uint8List.fromList(
        List.filled(totalItems, libraryGalleryLayoutDimensionsKnownFlag),
      ),
    ),
  );
  return builder.build();
}
