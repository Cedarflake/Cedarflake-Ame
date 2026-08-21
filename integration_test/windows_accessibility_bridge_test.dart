import "dart:async";
import "dart:typed_data";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
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
import "package:integration_test/integration_test.dart";

void main() {
  IntegrationTestWidgetsFlutterBinding.ensureInitialized();

  testWidgets("keeps the native Windows accessibility tree synchronized", (
    tester,
  ) async {
    const itemCount = 1200;
    const wallSize = Size(1000, 700);
    final state = _galleryState(itemCount);
    final manifest = _manifest(itemCount);
    final snapshot = LibrarySnapshot(
      catalogPath: "C:\\AmeData\\ame.sqlite3",
      revision: BigInt.one,
      queryId: state.queryId,
      roots: state.roots,
      assets: state.assets,
    );
    final layoutSnapshot = LibraryGalleryLayoutSnapshot.build(
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
          overrides: [
            initialLibraryStateProvider.overrideWithValue(state),
            libraryCatalogProvider.overrideWithValue(
              _StaticCatalog(snapshot, state.timeline!),
            ),
            libraryScannerProvider.overrideWithValue(const _NoopScanner()),
            libraryPreviewerProvider.overrideWithValue(
              _StaticPreviewer(state.assets),
            ),
          ],
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
                            layoutMetrics: layoutSnapshot.metrics,
                            timeline: state.timeline,
                            layoutShape: GalleryLayoutShape.equalHeight,
                            virtualGeometry: LibraryVirtualGalleryGeometry(
                              totalContentExtent:
                                  layoutSnapshot.metrics.contentExtent,
                              viewportExtent: wallSize.height,
                              leadingExtent: 0,
                              loadedContentExtent:
                                  layoutSnapshot.metrics.contentExtent,
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
      await tester.pumpAndSettle();
      expect(find.byType(MenuAnchor), findsNothing);

      for (final ordinal in <int>[160, 480, 800, 1120, 640, 320, 960, 0]) {
        scrollController.jumpTo(layoutSnapshot.metrics.itemOffsets[ordinal]);
        await tester.pump();
        await tester.pump();

        await tester.tap(
          find.byType(LibraryPhotoTile).hitTestable().first,
          buttons: kSecondaryMouseButton,
        );
        await tester.pumpAndSettle();
        expect(find.byType(MenuAnchor), findsNothing);

        await tester.sendKeyEvent(LogicalKeyboardKey.escape);
        await tester.pumpAndSettle();
      }

      await tester.pumpWidget(const SizedBox.shrink());
      await tester.pumpAndSettle();
    } finally {
      semanticsHandle.dispose();
    }
  });

  testWidgets(
    "keeps the populated application accessibility tree synchronized",
    (tester) async {
      const itemCount = 1200;
      final state = _galleryState(itemCount);
      final manifest = _manifest(itemCount);
      final snapshot = LibrarySnapshot(
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        revision: BigInt.one,
        queryId: state.queryId,
        roots: state.roots,
        assets: state.assets,
      );
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final semanticsHandle = tester.ensureSemantics();
      try {
        await tester.pumpWidget(
          ProviderScope(
            overrides: [
              initialLibraryStateProvider.overrideWithValue(state),
              libraryCatalogProvider.overrideWithValue(
                _StaticCatalog(snapshot, state.timeline!),
              ),
              libraryScannerProvider.overrideWithValue(const _NoopScanner()),
              libraryPreviewerProvider.overrideWithValue(
                _StaticPreviewer(state.assets),
              ),
              libraryGalleryLayoutManifestLoaderProvider.overrideWithValue(
                _StaticManifestLoader(manifest),
              ),
            ],
            child: const AmeApp(),
          ),
        );
        await tester.pumpAndSettle();

        final mouse = await tester.createGesture(kind: PointerDeviceKind.mouse);
        await mouse.addPointer();
        for (final key in const <Key>[
          Key("library-sort-menu"),
          Key("library-layout-menu"),
          Key("library-more-menu"),
          ValueKey("source-more-root-1"),
        ]) {
          await mouse.moveTo(tester.getCenter(find.byKey(key)));
          await tester.pump(const Duration(milliseconds: 600));
          await tester.tap(find.byKey(key));
          await tester.pumpAndSettle();
          await tester.tap(find.byKey(key));
          await tester.pumpAndSettle();
        }

        final wall = tester.widget<LibraryGalleryWall>(
          find.byType(LibraryGalleryWall),
        );
        for (final fraction in <double>[
          0.15,
          0.45,
          0.75,
          1,
          0.6,
          0.3,
          0.9,
          0,
        ]) {
          wall.scrollController.jumpTo(
            wall.scrollController.position.maxScrollExtent * fraction,
          );
          await tester.pump();
          await tester.pump();

          await tester.tap(
            find.byType(LibraryPhotoTile).hitTestable().first,
            buttons: kSecondaryMouseButton,
          );
          await tester.pumpAndSettle();
          await tester.sendKeyEvent(LogicalKeyboardKey.escape);
          await tester.pumpAndSettle();
        }

        final timelineSlider = tester.widget<Slider>(
          find.byKey(const Key("timeline-slider")),
        );
        timelineSlider.onChangeStart?.call(timelineSlider.value);
        timelineSlider.onChanged?.call(0.5);
        timelineSlider.onChangeEnd?.call(0.5);
        await tester.pump();

        for (var cycle = 0; cycle < 2; cycle += 1) {
          final tile = find.byType(LibraryPhotoTile).hitTestable().first;
          final tileRect = tester.getRect(tile);
          await tester.tapAt(tileRect.topLeft + const Offset(16, 16));
          await tester.pump();
          await tester.pump();
          expect(find.byKey(const Key("viewer-back-button")), findsOneWidget);

          final viewerSlider = tester.widget<Slider>(
            find.descendant(
              of: find.byKey(const Key("viewer-zoom-controls")),
              matching: find.byType(Slider),
            ),
          );
          viewerSlider.onChangeStart?.call(viewerSlider.value);
          viewerSlider.onChanged?.call(1.25);
          viewerSlider.onChangeEnd?.call(1.25);
          await tester.pump();

          await mouse.moveTo(
            tester.getCenter(find.byKey(const Key("viewer-more-menu"))),
          );
          await tester.pump(const Duration(milliseconds: 600));
          await tester.tap(find.byKey(const Key("viewer-more-menu")));
          await tester.pumpAndSettle();
          await tester.tap(find.byKey(const Key("viewer-more-menu")));
          await tester.pumpAndSettle();

          await tester.tap(find.byKey(const Key("viewer-back-button")));
          await tester.pump();
          await tester.pump();
          expect(find.byType(LibraryPhotoTile), findsWidgets);
        }
        await mouse.removePointer();

        await tester.pumpWidget(const SizedBox.shrink());
        await tester.pumpAndSettle();
      } finally {
        semanticsHandle.dispose();
      }
    },
  );
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
    queryId: "windows-accessibility-query",
    assets: [for (var index = 0; index < itemCount; index++) _asset(index)],
    timeline: LibraryTimeline(
      revision: revision,
      queryId: "windows-accessibility-query",
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
    queryId: "windows-accessibility-query",
    totalItems: itemCount,
  );
  builder.append(
    LibraryGalleryLayoutManifestChunk(
      revision: revision,
      queryId: "windows-accessibility-query",
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

class _StaticCatalog implements LibraryCatalog {
  const _StaticCatalog(this.snapshot, this.timeline);

  final LibrarySnapshot snapshot;
  final LibraryTimeline timeline;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => snapshot;

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot;

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline;

  @override
  Future<bool> unregisterRoot(String rootId) async => false;
}

class _StaticPreviewer implements LibraryPreviewer {
  _StaticPreviewer(Iterable<LibraryAsset> assets)
    : _assets = {for (final asset in assets) asset.locationId: asset};

  final Map<String, LibraryAsset> _assets;

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
    bool retry = false,
    Iterable<String> protectedLocationIds = const [],
  }) async => _assets[locationId]!;
}

class _StaticManifestLoader implements LibraryGalleryLayoutManifestLoader {
  const _StaticManifestLoader(this.manifest);

  final LibraryGalleryLayoutManifest manifest;

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) async => manifest;
}

class _NoopScanner implements LibraryScanner {
  const _NoopScanner();

  @override
  bool cancel(String scanId) => false;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) => const Stream.empty();

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) => const Stream.empty();
}
