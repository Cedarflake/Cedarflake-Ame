import "dart:async";
import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/concurrent_library_scanner.dart";
import "../support/library_query_snapshot_fixture.dart";

const _query = LibraryGalleryQuery(rootId: "peer");

void main() {
  for (final addedCount in [48, 504]) {
    for (final scrollDuringRead in [false, true]) {
      testWidgets(
        "peer import keeps visible dates after $addedCount additions, later scroll: $scrollDuringRead",
        (tester) async {
          tester.view.physicalSize = const Size(1280, 800);
          tester.view.devicePixelRatio = 1;
          addTearDown(tester.view.resetPhysicalSize);
          addTearDown(tester.view.resetDevicePixelRatio);
          final directory = Directory.systemTemp.createTempSync(
            "ame-position-",
          );
          final preview = File("${directory.path}/preview.png")
            ..writeAsBytesSync(
              base64Decode(
                "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=",
              ),
            );
          addTearDown(() {
            preview.deleteSync();
            directory.deleteSync();
          });
          final catalog = _Catalog(preview.path);
          final scanner = ConcurrentLibraryScanner();
          addTearDown(scanner.dispose);
          final container = ProviderContainer(
            overrides: [
              libraryCatalogProvider.overrideWithValue(catalog),
              libraryScannerProvider.overrideWithValue(scanner),
              initialLibraryStateProvider.overrideWithValue(
                LibraryState.fromSnapshot(
                  catalog.snapshot(),
                  query: _query,
                ).copyWith(timeline: catalog.timeline()),
              ),
            ],
          );
          addTearDown(container.dispose);
          await tester.pumpWidget(
            UncontrolledProviderScope(
              container: container,
              child: const AmeApp(),
            ),
          );
          await _pumpFrames(tester);
          final controller = container.read(libraryControllerProvider.notifier);
          await controller.scanDirectory(r"C:\Generated\Background");
          final scanId = scanner.scanIdForRootPath(r"C:\Generated\Background");
          scanner.add(
            scanId,
            LibraryScanStarted(
              scanId: scanId,
              rootPath: r"C:\Generated\Background",
              itemLimit: null,
              entryLimit: null,
            ),
          );
          await _pumpFrames(tester);
          await tester.drag(
            find.byKey(const Key("library-photo-wall")),
            const Offset(0, -900),
          );
          await _pumpFrames(tester);
          var before = _visibleImages(tester);
          expect(before.length, greaterThan(2));

          catalog.prependRecentImages(addedCount);
          final pending = scrollDuringRead ? Completer<void>() : null;
          catalog.pendingRead = pending?.future;
          scanner.add(
            scanId,
            const LibraryScanCompleted(
              assetCount: 10000,
              issueCount: 0,
              catalogPath: "position-fixture.sqlite3",
              wasLimited: false,
            ),
          );
          await _pumpFrames(tester);

          if (pending != null) {
            await tester.drag(
              find.byKey(const Key("library-photo-wall")),
              const Offset(0, -420),
            );
            await _pumpFrames(tester);
            final afterScroll = _visibleImages(tester);
            expect(afterScroll, isNot(before));
            before = afterScroll;
            pending.complete();
            await _pumpFrames(tester);
          }

          expect(
            container.read(libraryControllerProvider).status,
            LibraryStatus.completed,
          );
          final state = container.read(libraryControllerProvider);
          expect(state.assets.length, lessThanOrEqualTo(libraryCatalogWindow));
          expect(state.timeline?.totalItems, 96 + addedCount);
          if (addedCount > libraryCatalogWindow) {
            expect(state.windowStartItemOffset, greaterThan(0));
          }
          expect(_visibleImages(tester), before);
          expect(tester.takeException(), isNull);
          await tester.pumpWidget(const SizedBox.shrink());
        },
        variant: TargetPlatformVariant.only(TargetPlatform.windows),
      );
    }
  }
}

Map<String, Rect> _visibleImages(WidgetTester tester) {
  final wall = tester.getRect(find.byKey(const Key("library-photo-wall")));
  final result = <String, Rect>{};
  for (final element in find.byType(LibraryPhotoTile).evaluate()) {
    final rectangle = tester.getRect(
      find.byElementPredicate((candidate) => identical(candidate, element)),
    );
    if (wall.contains(rectangle.center)) {
      result[(element.widget as LibraryPhotoTile).asset.locationId] = rectangle;
    }
  }
  return result;
}

Future<void> _pumpFrames(WidgetTester tester) async {
  for (var frame = 0; frame < 40; frame += 1) {
    await tester.pump(const Duration(milliseconds: 16));
  }
}

class _Catalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog, LibraryStableQueryAnchorCatalog {
  _Catalog(this.previewPath) {
    assets = List.generate(96, (index) => _asset(index, 2013));
  }

  final String previewPath;
  late List<LibraryAsset> assets;
  int revision = 1;
  int addedCount = 0;
  Future<void>? pendingRead;

  void prependRecentImages(int count) {
    addedCount = count;
    assets = [
      ...List.generate(count, (index) => _asset(index + 96, 2022)),
      ...assets,
    ];
    revision += 1;
  }

  LibrarySnapshot snapshot({String? anchorLocationId}) {
    final index = assets.indexWhere(
      (asset) => asset.locationId == anchorLocationId,
    );
    final start = index >= libraryCatalogWindow ? (index ~/ 4) * 4 - 100 : 0;
    final end = (start + libraryCatalogWindow).clamp(0, assets.length);
    return LibrarySnapshot(
      catalogPath: "position-fixture.sqlite3",
      revision: BigInt.from(revision),
      queryId: "peer-query",
      roots: [
        LibraryRoot(
          id: "peer",
          path: r"C:\Generated\Peer",
          displayPath: r"C:\Generated\Peer",
          activeScanId: "peer-scan",
          createdUnixMs: 1,
          assetCount: assets.length,
          issueCount: 0,
          availability: LibraryRootAvailability.available,
        ),
      ],
      assets: assets.sublist(start, end),
      queryAnchorResolution: anchorLocationId == null
          ? null
          : LibraryQueryAnchorResolution(
              requestedLocationId: anchorLocationId,
              locationId: index < 0 ? null : anchorLocationId,
              ordinal: index < 0 ? null : index,
              windowStartItemOffset: start,
            ),
    );
  }

  LibraryTimeline timeline() => LibraryTimeline(
    revision: BigInt.from(revision),
    queryId: "peer-query",
    totalItems: assets.length,
    buckets: [
      if (revision > 1)
        LibraryTimeBucket(
          monthKey: "2022-12",
          itemCount: addedCount,
          aspectRatioSum: addedCount * 4 / 3,
        ),
      const LibraryTimeBucket(
        monthKey: "2013-08",
        itemCount: 96,
        aspectRatioSum: 128,
      ),
    ],
  );

  LibraryAsset _asset(int index, int year) => LibraryAsset(
    assetId: "asset-$index",
    locationId: "location-$index",
    rootId: "peer",
    activeScanId: "peer-scan",
    sourcePath: "generated-$index.png",
    displayPath: "generated-$index.png",
    relativePath: "generated-$index.png",
    previewPath: previewPath,
    fileSize: BigInt.one,
    modifiedUnixMs: DateTime.utc(
      year,
      year == 2013 ? 8 : 12,
      2,
    ).millisecondsSinceEpoch,
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    width: 400,
    height: 300,
  );

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    final result = snapshot();
    final pending = pendingRead;
    pendingRead = null;
    await pending;
    return result;
  }

  @override
  Future<LibrarySnapshot> loadAroundAsset({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String requestedLocationId,
    required String anchorAssetId,
    required int fallbackGlobalItemIndex,
  }) async {
    final result = snapshot(anchorLocationId: requestedLocationId);
    final pending = pendingRead;
    pendingRead = null;
    await pending;
    return result;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline();

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) => throw UnimplementedError("No rail navigation in this fixture");

  @override
  Future<bool> unregisterRoot(String rootId) =>
      throw UnimplementedError("No removal in this fixture");
}
