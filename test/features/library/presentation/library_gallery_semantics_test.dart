import "dart:typed_data";

import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_selection.dart";
import "package:cedarflake_ame/features/library/presentation/gallery_view_options.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_layout_snapshot.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_gallery_wall.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_time_navigation.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_virtual_gallery_geometry.dart";
import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter/services.dart";
import "package:flutter_test/flutter_test.dart";

import "../../../support/semantics/retained_semantics_update_harness.dart";

void main() {
  RetainedSemanticsUpdateBinding();

  setUp(RetainedSemanticsUpdateValidator.instance.reset);

  testWidgets("keeps virtual gallery semantics valid across deep jumps", (
    tester,
  ) async {
    const itemCount = 1200;
    const wallSize = Size(1000, 700);
    final state = _galleryState(itemCount);
    final manifest = _manifest(itemCount);
    final snapshot = LibraryGalleryLayoutSnapshot.build(
      manifest: manifest,
      availableWidth: wallSize.width - 80 - 40,
      thumbnailSize: GalleryThumbnailSize.medium,
      sortKey: LibraryGallerySortKey.captureTime,
    );
    final scrollController = ScrollController();
    addTearDown(scrollController.dispose);
    tester.view.physicalSize = const Size(1280, 800);
    tester.view.devicePixelRatio = 1;
    addTearDown(tester.view.resetPhysicalSize);
    addTearDown(tester.view.resetDevicePixelRatio);
    final semanticsHandle = tester.ensureSemantics();
    try {
      await tester.pumpWidget(
        ProviderScope(
          overrides: [initialLibraryStateProvider.overrideWithValue(state)],
          child: MaterialApp(
            home: Scaffold(
              body: Align(
                alignment: Alignment.topLeft,
                child: SizedBox.fromSize(
                  size: wallSize,
                  child: Consumer(
                    builder: (context, ref, child) {
                      final controller = ref.read(
                        libraryControllerProvider.notifier,
                      );
                      return Row(
                        children: [
                          Expanded(
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
                              onLoadPrevious: () async {},
                              onLayoutChanged: (_, _) {},
                            ),
                          ),
                          LibraryTimeNavigation(
                            isLoading: false,
                            scrollController: scrollController,
                            layoutMetrics: snapshot.metrics,
                            timeline: state.timeline,
                            layoutShape: GalleryLayoutShape.equalHeight,
                            virtualGeometry: LibraryVirtualGalleryGeometry(
                              totalContentExtent:
                                  snapshot.metrics.contentExtent,
                              viewportExtent: wallSize.height,
                              leadingExtent: 0,
                              loadedContentExtent:
                                  snapshot.metrics.contentExtent,
                              trailingExtent: 0,
                              windowStartItemOffset: 0,
                              loadedItemCount: itemCount,
                              totalItemCount: itemCount,
                              queryId: state.queryId,
                            ),
                            windowStartItemOffset: 0,
                            loadedItemCount: itemCount,
                            onSeek: (_, _) async => true,
                          ),
                        ],
                      );
                    },
                  ),
                ),
              ),
            ),
          ),
        ),
      );
      await tester.pump();
      expect(find.byType(MenuAnchor), findsNothing);

      for (final ordinal in <int>[160, 480, 800, 1120, 640, 320, 960, 0]) {
        scrollController.jumpTo(snapshot.metrics.itemOffsets[ordinal]);
        await tester.pump();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "virtual-gallery-jump-$ordinal-frame-1",
        );
        await tester.pump();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "virtual-gallery-jump-$ordinal-frame-2",
        );

        await tester.tap(
          find.byType(LibraryPhotoTile).hitTestable().first,
          buttons: kSecondaryMouseButton,
        );
        await tester.pumpAndSettle();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "virtual-gallery-menu-$ordinal-open",
        );
        expect(find.byType(MenuAnchor), findsNothing);

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
        RetainedSemanticsUpdateValidator.instance.verifyLatestUpdate(
          trace: "virtual-gallery-menu-$ordinal-closed",
        );
      }
    } finally {
      semanticsHandle.dispose();
    }
  });
}

LibraryState _galleryState(int itemCount) {
  final revision = BigInt.one;
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
    queryId: "semantics-query",
    assets: [for (var index = 0; index < itemCount; index++) _asset(index)],
    timeline: LibraryTimeline(
      revision: revision,
      queryId: "semantics-query",
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

LibraryAsset _asset(int index) {
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
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibraryGalleryLayoutManifest _manifest(int itemCount) {
  final revision = BigInt.one;
  final builder = LibraryGalleryLayoutManifestBuilder(
    revision: revision,
    queryId: "semantics-query",
    totalItems: itemCount,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "semantics-query",
      totalItems: itemCount,
      startOrdinal: 0,
      locationIds: [
        for (var index = 0; index < itemCount; index++) "location-$index",
      ],
      aspectRatioMilli: Uint16List.fromList(List.filled(itemCount, 1000)),
      dateGroupIndices: Uint16List(itemCount),
      dateGroups: const ["2026-08-10"],
      flags: Uint8List.fromList(
        List.filled(itemCount, libraryGalleryLayoutDimensionsKnownFlag),
      ),
    ),
  );
  return builder.build();
}
