import "dart:typed_data";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_exact_extent_sliver.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_snapshot.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("visible range excludes surrounding prefetch evidence", () {
    final range = LibraryGalleryVisibleRange(
      queryId: "query-1",
      revision: BigInt.one,
      startGlobalItemIndex: 20,
      endGlobalItemIndexExclusive: 30,
    );

    expect(range.containsGlobalItemIndex(19), isFalse);
    expect(range.containsGlobalItemIndex(20), isTrue);
    expect(range.containsGlobalItemIndex(29), isTrue);
    expect(range.containsGlobalItemIndex(30), isFalse);
    expect(
      range.contains(
        queryId: "stale-query",
        revision: BigInt.one,
        globalItemIndex: 24,
      ),
      isFalse,
    );
  });

  testWidgets("preserves a deep logical anchor through a latest-only resize", (
    tester,
  ) async {
    const itemCount = 4000;
    const initialSize = Size(1000, 640);
    const finalSize = Size(760, 600);
    final manifest = _manifest(itemCount);
    final libraryState = ValueNotifier(_state(itemCount));
    final controller = _RecordingGalleryController();
    final scrollController = ScrollController();
    final wallSize = ValueNotifier(initialSize);
    LibraryGalleryVisiblePosition? visiblePosition;
    var previousRequests = 0;
    addTearDown(() async {
      scrollController.dispose();
      wallSize.dispose();
      libraryState.dispose();
      await tester.binding.setSurfaceSize(null);
    });
    await tester.binding.setSurfaceSize(const Size(1200, 800));

    Widget buildWall(Size size, LibraryState state) {
      return Align(
        alignment: Alignment.topLeft,
        child: SizedBox(
          width: size.width,
          height: size.height,
          child: LibraryGalleryWall(
            state: state,
            controller: controller,
            scrollController: scrollController,
            layoutShape: GalleryLayoutShape.equalHeight,
            thumbnailSize: GalleryThumbnailSize.medium,
            selection: GallerySelection.empty(state.queryId),
            isSelecting: false,
            layoutManifest: manifest,
            onOpen: (_) {},
            onToggleSelection: (_) {},
            onViewInformation: (_) {},
            onCopyPath: (_) {},
            onRevealFile: (_) {},
            onVisiblePositionChanged: (position) {
              visiblePosition = position;
            },
            onLoadPrevious: () async {
              previousRequests += 1;
            },
            onLayoutChanged: (_, _) {},
          ),
        ),
      );
    }

    await tester.pumpWidget(
      ProviderScope(
        child: MaterialApp(
          home: ListenableBuilder(
            listenable: Listenable.merge([wallSize, libraryState]),
            builder: (context, child) =>
                buildWall(wallSize.value, libraryState.value),
          ),
        ),
      ),
    );

    final initialSnapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: manifest,
      availableWidth: initialSize.width - 40,
      thumbnailSize: GalleryThumbnailSize.medium,
      sortKey: LibraryGallerySortKey.captureTime,
    );
    final deepOffset =
        initialSnapshot.metrics.itemOffsets[3000] -
        initialSize.height * 0.5 +
        initialSnapshot.metrics.photoRowHeight * 0.35;
    scrollController.jumpTo(deepOffset);
    await tester.pump();
    await tester.pump();

    final anchorBefore = visiblePosition;
    expect(anchorBefore, isNotNull);
    expect(controller.previewDemandRequests, isNotEmpty);
    final previewDemand = controller.previewDemandRequests.last;
    expect(previewDemand.visible, contains(anchorBefore!.locationId));
    expect(previewDemand.nearDirection, isNotEmpty);
    expect(previewDemand.guard, isNotEmpty);
    final anchorFinder = find.byKey(ValueKey(anchorBefore.locationId!));
    final initialRenderSliver = tester
        .renderObject<RenderLibraryExactExtentSliver>(
          find.byType(LibraryExactExtentSliver),
        );
    final expectedEntryIndex = initialSnapshot.entryIndexForScrollOffset(
      initialSnapshot.metrics.itemOffsets[3000] - 18,
    );
    expect(
      expectedEntryIndex,
      inInclusiveRange(
        initialRenderSliver.indexOf(initialRenderSliver.firstChild!),
        initialRenderSliver.indexOf(initialRenderSliver.lastChild!),
      ),
    );
    expect(anchorFinder, findsOneWidget);
    final wallTopBefore = tester.getTopLeft(
      find.byKey(const Key("library-photo-wall")),
    );
    final anchorTopBefore =
        tester.getTopLeft(anchorFinder).dy - wallTopBefore.dy;
    final oldAnchorOffset = initialSnapshot.metrics.offsetForGlobalItemIndex(
      anchorBefore.globalItemIndex,
    );
    controller.nextPageRequests = 0;
    controller.previousPageRequests = 0;
    controller.timeSeekRequests = 0;
    previousRequests = 0;

    wallSize.value = const Size(880, 620);
    wallSize.value = finalSize;
    await tester.pump();
    await tester.pump();

    final finalSnapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: manifest,
      availableWidth: finalSize.width - 40,
      thumbnailSize: GalleryThumbnailSize.medium,
      sortKey: LibraryGallerySortKey.captureTime,
    );
    final newAnchorOffset = finalSnapshot.metrics.offsetForGlobalItemIndex(
      anchorBefore.globalItemIndex,
    );
    expect((newAnchorOffset! - oldAnchorOffset!).abs(), greaterThan(2));
    expect(anchorFinder, findsOneWidget);
    final wallTopAfter = tester.getTopLeft(
      find.byKey(const Key("library-photo-wall")),
    );
    final anchorTopAfter = tester.getTopLeft(anchorFinder).dy - wallTopAfter.dy;
    expect(
      anchorTopAfter,
      closeTo(
        anchorTopBefore + (finalSize.height - initialSize.height) * 0.5,
        2,
      ),
    );
    expect(
      scrollController.position.maxScrollExtent,
      closeTo(finalSnapshot.metrics.contentExtent - finalSize.height, 0.01),
    );
    final sliver = tester.widget<LibraryExactExtentSliver>(
      find.byType(LibraryExactExtentSliver),
    );
    expect(
      identical(sliver.itemStartOffsets, finalSnapshot.entryStartOffsets),
      isFalse,
      reason: "the widget owns its separately built latest-width snapshot",
    );
    expect(sliver.itemStartOffsets, finalSnapshot.entryStartOffsets);
    expect(controller.nextPageRequests, 0);
    expect(controller.previousPageRequests, 0);
    expect(controller.timeSeekRequests, 0);
    expect(previousRequests, 0);

    final renderSliver = tester.renderObject<RenderLibraryExactExtentSliver>(
      find.byType(LibraryExactExtentSliver),
    );
    final offsetsBeforePreview = renderSliver.itemStartOffsets;
    final correctionsBeforePreview = renderSliver.appliedLayoutCorrectionCount;
    final pixelsBeforePreview = scrollController.position.pixels;
    final maximumBeforePreview = scrollController.position.maxScrollExtent;
    final previewTile = find.byKey(const ValueKey("location-3000"));
    final previewRectBefore = tester.getRect(previewTile);

    libraryState.value = libraryState.value.copyWith(
      assets: [
        for (final asset in libraryState.value.assets)
          if (asset.locationId == "location-3000")
            _asset(3000, "updated preview evidence")
          else
            asset,
      ],
    );
    await tester.pump();

    final updatedRenderSliver = tester
        .renderObject<RenderLibraryExactExtentSliver>(
          find.byType(LibraryExactExtentSliver),
        );
    expect(identical(updatedRenderSliver, renderSliver), isTrue);
    expect(
      identical(updatedRenderSliver.itemStartOffsets, offsetsBeforePreview),
      isTrue,
    );
    expect(
      updatedRenderSliver.appliedLayoutCorrectionCount,
      correctionsBeforePreview,
    );
    expect(
      tester
          .widget<LibraryExactExtentSliver>(
            find.byType(LibraryExactExtentSliver),
          )
          .layoutCorrection,
      isNull,
    );
    expect(scrollController.position.pixels, pixelsBeforePreview);
    expect(scrollController.position.maxScrollExtent, maximumBeforePreview);
    expect(tester.getRect(previewTile), previewRectBefore);
    expect(
      tester.widget<LibraryPhotoTile>(previewTile).asset.previewIssueMessage,
      "updated preview evidence",
    );
    expect(controller.nextPageRequests, 0);
    expect(controller.previousPageRequests, 0);
    expect(controller.timeSeekRequests, 0);
    expect(previousRequests, 0);

    final previewTopBeforeHeightResize = tester.getTopLeft(previewTile).dy;
    wallSize.value = const Size(760, 540);
    await tester.pump();
    await tester.pump();

    expect(
      tester.getTopLeft(previewTile).dy,
      closeTo(previewTopBeforeHeightResize - 30, 2),
    );
    expect(
      scrollController.position.maxScrollExtent,
      closeTo(finalSnapshot.metrics.contentExtent - 540, 0.01),
    );
    expect(controller.nextPageRequests, 0);
    expect(controller.previousPageRequests, 0);
    expect(controller.timeSeekRequests, 0);
    expect(previousRequests, 0);
  });

  testWidgets(
    "preserves the logical anchor when recovered dimensions replace geometry",
    (tester) async {
      const itemCount = 4000;
      const wallSize = Size(900, 620);
      final initialManifest = _manifest(itemCount, dimensionsKnown: false);
      final manifest = ValueNotifier(initialManifest);
      final state = _state(itemCount);
      final controller = _RecordingGalleryController();
      final scrollController = ScrollController();
      LibraryGalleryVisiblePosition? visiblePosition;
      addTearDown(() async {
        scrollController.dispose();
        manifest.dispose();
        await tester.binding.setSurfaceSize(null);
      });
      await tester.binding.setSurfaceSize(const Size(1100, 760));

      await tester.pumpWidget(
        ProviderScope(
          child: MaterialApp(
            home: ValueListenableBuilder(
              valueListenable: manifest,
              builder: (context, currentManifest, child) => Align(
                alignment: Alignment.topLeft,
                child: SizedBox(
                  width: wallSize.width,
                  height: wallSize.height,
                  child: LibraryGalleryWall(
                    state: state,
                    controller: controller,
                    scrollController: scrollController,
                    layoutShape: GalleryLayoutShape.equalHeight,
                    thumbnailSize: GalleryThumbnailSize.medium,
                    selection: GallerySelection.empty(state.queryId),
                    isSelecting: false,
                    layoutManifest: currentManifest,
                    onOpen: (_) {},
                    onToggleSelection: (_) {},
                    onViewInformation: (_) {},
                    onCopyPath: (_) {},
                    onRevealFile: (_) {},
                    onVisiblePositionChanged: (position) {
                      visiblePosition = position;
                    },
                    onLoadPrevious: controller.loadPreviousPage,
                    onLayoutChanged: (_, _) {},
                  ),
                ),
              ),
            ),
          ),
        ),
      );

      final initialSnapshot = LibraryGalleryLayoutSnapshot.build(
        manifest: initialManifest,
        availableWidth: wallSize.width - 40,
        thumbnailSize: GalleryThumbnailSize.medium,
        sortKey: LibraryGallerySortKey.captureTime,
      );
      scrollController.jumpTo(
        initialSnapshot.metrics.itemOffsets[3000] - wallSize.height * 0.5,
      );
      await tester.pump();
      await tester.pump();

      final anchorBefore = visiblePosition;
      expect(anchorBefore, isNotNull);
      final anchorFinder = find.byKey(ValueKey(anchorBefore!.locationId!));
      expect(anchorFinder, findsOneWidget);
      final anchorTopBefore = tester.getTopLeft(anchorFinder).dy;

      final recovered = initialManifest.withDimensionUpdates([
        for (var index = 0; index < itemCount; index++)
          LibraryGalleryLayoutDimensionUpdate(
            revision: initialManifest.revision,
            queryId: initialManifest.queryId,
            globalItemIndex: index,
            locationId: "location-$index",
            width: 650 + (index * 277) % 2350,
            height: 1000,
          ),
      ]);
      final recoveredSnapshot = LibraryGalleryLayoutSnapshot.build(
        manifest: recovered,
        availableWidth: wallSize.width - 40,
        thumbnailSize: GalleryThumbnailSize.medium,
        sortKey: LibraryGallerySortKey.captureTime,
      );
      expect(
        recoveredSnapshot.metrics.contentExtent,
        isNot(closeTo(initialSnapshot.metrics.contentExtent, 0.01)),
      );

      manifest.value = recovered;
      await tester.pump();
      await tester.pump();
      await tester.pump();

      final recoveredSliver = tester.widget<LibraryExactExtentSliver>(
        find.byType(LibraryExactExtentSliver),
      );
      expect(
        recoveredSliver.itemStartOffsets,
        recoveredSnapshot.entryStartOffsets,
      );
      expect(anchorFinder, findsOneWidget);
      expect(tester.getTopLeft(anchorFinder).dy, closeTo(anchorTopBefore, 2));
      expect(
        scrollController.position.maxScrollExtent,
        closeTo(
          recoveredSnapshot.metrics.contentExtent - wallSize.height,
          0.01,
        ),
      );
    },
  );

  testWidgets(
    "requests details for visible rows crossing either window boundary",
    (tester) async {
      const itemCount = 60;
      const wallSize = Size(1000, 360);
      final manifest = _manifest(itemCount);
      final snapshot = LibraryGalleryLayoutSnapshot.build(
        manifest: manifest,
        availableWidth: wallSize.width - 40,
        thumbnailSize: GalleryThumbnailSize.medium,
        sortKey: LibraryGallerySortKey.captureTime,
      );
      final targetRow = snapshot.entries.firstWhere(
        (entry) => entry.isPhotoRow && entry.itemCount >= 3,
      );
      final rowStart = targetRow.startItemIndex;
      final rowEnd = rowStart + targetRow.itemCount;

      Future<void> pumpBoundary({
        required LibraryState state,
        required _RecordingGalleryController controller,
        required ScrollController scrollController,
      }) async {
        await tester.pumpWidget(
          ProviderScope(
            child: MaterialApp(
              home: SizedBox(
                width: wallSize.width,
                height: wallSize.height,
                child: LibraryGalleryWall(
                  state: state,
                  controller: controller,
                  scrollController: scrollController,
                  layoutShape: GalleryLayoutShape.equalHeight,
                  thumbnailSize: GalleryThumbnailSize.medium,
                  selection: GallerySelection.empty(state.queryId),
                  isSelecting: false,
                  layoutManifest: manifest,
                  onOpen: (_) {},
                  onToggleSelection: (_) {},
                  onViewInformation: (_) {},
                  onCopyPath: (_) {},
                  onRevealFile: (_) {},
                  onVisiblePositionChanged: (_) {},
                  onLoadPrevious: controller.loadPreviousPage,
                  onLayoutChanged: (_, _) {},
                ),
              ),
            ),
          ),
        );
        await tester.pump();
        await tester.pump();
      }

      final previousController = _RecordingGalleryController();
      final previousScrollController = ScrollController(
        initialScrollOffset: snapshot.metrics.itemOffsets[rowStart],
      );
      addTearDown(previousScrollController.dispose);
      await pumpBoundary(
        state: _boundaryState(
          itemCount: itemCount,
          windowStart: rowStart + 1,
          windowEnd: rowEnd,
          hasPrevious: true,
          hasNext: false,
        ),
        controller: previousController,
        scrollController: previousScrollController,
      );

      expect(previousController.visibleRangeRequests, isNotEmpty);
      expect(previousController.previewDemandRequests, isNotEmpty);
      expect(
        previousController.previewDemandRequests.last.visible,
        containsAll([
          for (var index = rowStart + 1; index < rowEnd; index++)
            "location-$index",
        ]),
      );
      final previousVisibleRange = previousController.visibleRangeRequests.last;
      expect(previousVisibleRange.start, lessThanOrEqualTo(rowStart));
      expect(previousVisibleRange.end, greaterThanOrEqualTo(rowEnd));
      expect(previousController.previousPageRequests, 0);
      expect(previousController.nextPageRequests, 0);

      final nextController = _RecordingGalleryController();
      final nextScrollController = ScrollController(
        initialScrollOffset: snapshot.metrics.itemOffsets[rowStart],
      );
      addTearDown(nextScrollController.dispose);
      await pumpBoundary(
        state: _boundaryState(
          itemCount: itemCount,
          windowStart: rowStart,
          windowEnd: rowEnd - 1,
          hasPrevious: false,
          hasNext: true,
        ).copyWith(isLoadingTimeAnchor: true),
        controller: nextController,
        scrollController: nextScrollController,
      );

      expect(nextController.previousPageRequests, 0);
      expect(nextController.nextPageRequests, 0);
      expect(nextController.visibleRangeRequests, isNotEmpty);
      final nextVisibleRange = nextController.visibleRangeRequests.last;
      expect(nextVisibleRange.start, lessThanOrEqualTo(rowStart));
      expect(nextVisibleRange.end, greaterThanOrEqualTo(rowEnd));
    },
  );
}

LibraryState _state(int itemCount) {
  final revision = BigInt.one;
  final cursor = LibraryCatalogCursor(
    revision: revision,
    queryId: "resize-query",
    primaryMissing: false,
    primaryText: "2026-08-10T00:00:00.000000000",
    primaryNumber: 0,
    rootId: "root-1",
    locationId: "location-0",
  );
  const windowStart = 2800;
  final windowEnd = itemCount < 3201 ? itemCount : 3201;
  return LibraryState(
    status: LibraryStatus.completed,
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        createdUnixMs: 1,
        assetCount: itemCount,
        issueCount: 0,
      ),
    ],
    catalogRevision: revision,
    queryId: "resize-query",
    windowStartItemOffset: windowStart,
    assets: [
      for (var index = windowStart; index < windowEnd; index++)
        _asset(index, "initial preview evidence"),
    ],
    previousCursor: cursor,
    nextCursor: cursor,
    timeline: LibraryTimeline(
      revision: revision,
      queryId: "resize-query",
      totalItems: itemCount,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: itemCount,
          aspectRatioSum: itemCount.toDouble(),
        ),
      ],
    ),
  );
}

LibraryState _boundaryState({
  required int itemCount,
  required int windowStart,
  required int windowEnd,
  required bool hasPrevious,
  required bool hasNext,
}) {
  final revision = BigInt.one;
  final cursor = LibraryCatalogCursor(
    revision: revision,
    queryId: "resize-query",
    primaryMissing: false,
    primaryText: "2026-08-10T00:00:00.000000000",
    primaryNumber: 0,
    rootId: "root-1",
    locationId: "location-$windowStart",
  );
  return LibraryState(
    status: LibraryStatus.completed,
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        createdUnixMs: 1,
        assetCount: itemCount,
        issueCount: 0,
      ),
    ],
    catalogRevision: revision,
    queryId: "resize-query",
    windowStartItemOffset: windowStart,
    assets: [
      for (var index = windowStart; index < windowEnd; index++)
        _asset(index, "boundary preview evidence"),
    ],
    previousCursor: hasPrevious ? cursor : null,
    nextCursor: hasNext ? cursor : null,
    timeline: LibraryTimeline(
      revision: revision,
      queryId: "resize-query",
      totalItems: itemCount,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: itemCount,
          aspectRatioSum: itemCount.toDouble(),
        ),
      ],
    ),
  );
}

LibraryAsset _asset(int index, String previewIssueMessage) {
  return LibraryAsset(
    assetId: "asset-$index",
    locationId: "location-$index",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$index.jpg",
    displayPath: "C:\\Pictures\\$index.jpg",
    relativePath: "$index.jpg",
    previewPath: "",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 4,
    height: 3,
    previewStatus: LibraryPreviewStatus.failed,
    previewIssueCode: "preview_failed",
    previewIssueMessage: previewIssueMessage,
  );
}

LibraryGalleryLayoutManifest _manifest(
  int itemCount, {
  bool dimensionsKnown = true,
}) {
  final revision = BigInt.one;
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: "resize-query",
    totalItems: itemCount,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "resize-query",
      totalItems: itemCount,
      startOrdinal: 0,
      locationIds: [
        for (var index = 0; index < itemCount; index++) "location-$index",
      ],
      aspectRatioMilli: Uint16List.fromList([
        for (var index = 0; index < itemCount; index++)
          dimensionsKnown ? 650 + (index * 277) % 2350 : 1000,
      ]),
      dateGroupIndices: Uint16List.fromList([
        for (var index = 0; index < itemCount; index++) index ~/ 500,
      ]),
      dateGroups: const [
        "2026-08-10",
        "2026-08-09",
        "2026-08-08",
        "2026-08-07",
        "2026-08-06",
        "2026-08-05",
        "2026-08-04",
        "2026-08-03",
      ],
      flags: Uint8List.fromList(
        List.filled(
          itemCount,
          dimensionsKnown ? libraryGalleryLayoutDimensionsKnownFlag : 0,
        ),
      ),
    ),
  );
  return builder.build();
}

class _RecordingGalleryController extends LibraryController {
  var nextPageRequests = 0;
  var previousPageRequests = 0;
  var timeSeekRequests = 0;
  final visibleRangeRequests = <({int start, int end})>[];
  final previewDemandRequests =
      <
        ({List<String> visible, List<String> nearDirection, List<String> guard})
      >[];

  @override
  void updateGalleryPreviewDemand({
    Iterable<LibraryAsset> visible = const <LibraryAsset>[],
    Iterable<LibraryAsset> nearDirection = const <LibraryAsset>[],
    Iterable<LibraryAsset> guard = const <LibraryAsset>[],
    Iterable<LibraryAsset> idle = const <LibraryAsset>[],
    Map<String, int> previewEdges = const <String, int>{},
  }) {
    previewDemandRequests.add((
      visible: [for (final asset in visible) asset.locationId],
      nearDirection: [for (final asset in nearDirection) asset.locationId],
      guard: [for (final asset in guard) asset.locationId],
    ));
  }

  @override
  void ensureVisibleRange({
    required int startItemOffset,
    required int endItemOffsetExclusive,
  }) {
    visibleRangeRequests.add((
      start: startItemOffset,
      end: endItemOffsetExclusive,
    ));
  }

  @override
  Future<void> loadNextPage() async {
    nextPageRequests += 1;
  }

  @override
  Future<bool> loadPreviousPage() async {
    previousPageRequests += 1;
    return false;
  }

  @override
  Future<bool> jumpToTime(
    LibraryTimeBucket bucket, {
    int itemOffset = 0,
  }) async {
    timeSeekRequests += 1;
    return false;
  }
}
