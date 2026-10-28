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
const _removedRoot = LibraryRoot(
  id: "removed",
  path: r"C:\Generated\Removed",
  displayPath: r"C:\Generated\Removed",
  activeScanId: "removed-scan",
  createdUnixMs: 1,
  assetCount: 2,
  issueCount: 0,
);

void main() {
  for (final scenario in [
    (name: "normal reload", fails: false, scrolls: false, holdsCommit: false),
    (
      name: "retry after failed reload",
      fails: true,
      scrolls: false,
      holdsCommit: false,
    ),
    (name: "later scroll", fails: false, scrolls: true, holdsCommit: false),
    (
      name: "late failure after scroll",
      fails: true,
      scrolls: true,
      holdsCommit: false,
    ),
    (
      name: "later scroll before commit",
      fails: false,
      scrolls: true,
      holdsCommit: true,
    ),
  ]) {
    testWidgets("peer removal preserves images and position: ${scenario.name}", (
      tester,
    ) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final directory = Directory.systemTemp.createTempSync(
        "ame-removal-position-",
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
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await _pumpFrames(tester);
      final controller = container.read(libraryControllerProvider.notifier);
      expect(
        await controller.jumpToTime(
          catalog.timeline().buckets.single,
          itemOffset: 480,
        ),
        isTrue,
      );
      await _pumpFrames(tester);
      expect(
        container.read(libraryControllerProvider).windowStartItemOffset,
        480,
      );
      var before = _visibleImages(tester);
      expect(before.length, greaterThan(2));

      final pendingRead = Completer<void>();
      if (scenario.holdsCommit) {
        catalog.pendingRemoval = pendingRead.future;
      } else {
        catalog.pendingRead = pendingRead.future;
      }
      if (scenario.fails) {
        catalog.nextPageError = StateError("injected removal page failure");
      }
      final removal = container
          .read(libraryControllerProvider.notifier)
          .unregisterRoot(_removedRoot);
      await _pumpFrames(tester);
      try {
        expect(
          container.read(libraryControllerProvider).isRemovalCommitted,
          !scenario.holdsCommit,
        );
        expect(
          _visibleImages(tester),
          before,
          reason: "Pending removal refresh moved the peer gallery",
        );
        if (scenario.scrolls) {
          await tester.drag(
            find.byKey(const Key("library-photo-wall")),
            const Offset(0, -144),
          );
          await _pumpFrames(tester);
          final afterScroll = _visibleImages(tester);
          expect(afterScroll, isNot(before));
          before = afterScroll;
        }
      } finally {
        pendingRead.complete();
        await _pumpFrames(tester);
        await removal;
      }
      if (scenario.fails && !scenario.scrolls) {
        expect(
          container.read(libraryControllerProvider).status,
          LibraryStatus.failed,
        );
        expect(
          container.read(libraryControllerProvider).isRemovalCommitted,
          isTrue,
        );
        final retry = controller.retry();
        await _pumpFrames(tester);
        await retry;
      }
      expect(
        container.read(libraryControllerProvider).status,
        LibraryStatus.completed,
        reason: container.read(libraryControllerProvider).errorMessage,
      );
      expect(
        _visibleImages(tester),
        before,
        reason: "Completed removal refresh moved the peer gallery",
      );
      expect(catalog.removalCount, 1);
      expect(container.read(libraryControllerProvider).query, _query);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    }, variant: TargetPlatformVariant.only(TargetPlatform.windows));
  }
}

Map<String, Rect> _visibleImages(WidgetTester tester) {
  final wall = tester.getRect(find.byKey(const Key("library-photo-wall")));
  return {
    for (final element in find.byType(LibraryPhotoTile).evaluate())
      if (wall.contains(tester.getRect(find.byWidget(element.widget)).center))
        (element.widget as LibraryPhotoTile).asset.locationId: tester.getRect(
          find.byWidget(element.widget),
        ),
  };
}

Future<void> _pumpFrames(WidgetTester tester) async {
  for (var frame = 0; frame < 40; frame++) {
    await tester.pump(const Duration(milliseconds: 16));
  }
}

class _Catalog extends Fake
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog, LibraryStableQueryAnchorCatalog {
  _Catalog(String previewPath) {
    assets = List.generate(
      512,
      (index) => LibraryAsset(
        assetId: "asset-$index",
        locationId: "location-$index",
        rootId: "peer",
        activeScanId: "peer-scan",
        sourcePath: "generated-$index.png",
        displayPath: "generated-$index.png",
        relativePath: "generated-$index.png",
        previewPath: previewPath,
        fileSize: BigInt.one,
        modifiedUnixMs: DateTime.utc(2026, 10, 12).millisecondsSinceEpoch,
        sourceRevision: null,
        sourceGeneration: BigInt.one,
        width: 128,
        height: 96,
      ),
    );
  }

  late final List<LibraryAsset> assets;
  int removalCount = 0;
  Future<void>? pendingRead;
  Future<void>? pendingRemoval;
  Object? nextPageError;

  Future<void> _awaitPage() async {
    final error = nextPageError;
    nextPageError = null;
    await pendingRead;
    if (error != null) {
      throw error;
    }
  }

  LibrarySnapshot snapshot({String? anchorLocationId, int? startItemOffset}) {
    final index = assets.indexWhere(
      (asset) => asset.locationId == anchorLocationId,
    );
    final start = startItemOffset ?? ((index ~/ 4) * 4 - 100).clamp(0, 512);
    return LibrarySnapshot(
      catalogPath: "removal-position.sqlite3",
      revision: BigInt.from(removalCount + 1),
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
        ),
        const LibraryRoot(
          id: "large",
          path: r"C:\Generated\Large",
          displayPath: r"C:\Generated\Large",
          activeScanId: "large-scan",
          createdUnixMs: 1,
          assetCount: 10000,
          issueCount: 0,
        ),
        if (removalCount == 0) _removedRoot,
      ],
      assets: assets.sublist(
        start,
        (start + libraryCatalogWindow).clamp(0, assets.length),
      ),
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
    revision: BigInt.from(removalCount + 1),
    queryId: "peer-query",
    totalItems: assets.length,
    buckets: [
      LibraryTimeBucket(
        monthKey: "2026-10",
        itemCount: assets.length,
        aspectRatioSum: assets.length * 4 / 3,
      ),
    ],
  );

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    await _awaitPage();
    return snapshot();
  }

  @override
  Future<LibrarySnapshot> loadAroundAsset({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String requestedLocationId,
    required String anchorAssetId,
    required int fallbackGlobalItemIndex,
  }) async {
    await _awaitPage();
    return snapshot(anchorLocationId: requestedLocationId);
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline();

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot(startItemOffset: anchor.itemOffset);

  @override
  Future<bool> unregisterRoot(String rootId) async {
    expect(rootId, _removedRoot.id);
    await pendingRemoval;
    removalCount++;
    return true;
  }
}
