import "dart:async";
import "dart:convert";
import "dart:io";

import "package:cedarflake_ame/app/ame_app.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/application/library_synchronization.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:cedarflake_ame/features/library/domain/library_synchronization_models.dart";
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/concurrent_library_scanner.dart";
import "../support/library_query_snapshot_fixture.dart";

void main() {
  testWidgets(
    "busy synchronization preserves a recovering middle window",
    (tester) async {
      tester.view.physicalSize = const Size(1280, 800);
      tester.view.devicePixelRatio = 1;
      addTearDown(tester.view.resetPhysicalSize);
      addTearDown(tester.view.resetDevicePixelRatio);
      final directory = Directory.systemTemp.createTempSync(
        "ame-page-position-",
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
      final synchronization = _Synchronization();
      addTearDown(scanner.dispose);
      addTearDown(synchronization.dispose);
      final container = ProviderContainer(
        overrides: [
          libraryCatalogProvider.overrideWithValue(catalog),
          libraryScannerProvider.overrideWithValue(scanner),
          librarySynchronizationProvider.overrideWithValue(synchronization),
          initialLibraryStateProvider.overrideWithValue(
            LibraryState.fromSnapshot(
              catalog.snapshot(1, start: 4000),
            ).copyWith(
              timeline: catalog.timeline(1),
              windowStartItemOffset: 4000,
            ),
          ),
        ],
      );
      addTearDown(container.dispose);
      await tester.pumpWidget(
        UncontrolledProviderScope(container: container, child: const AmeApp()),
      );
      await _pumpFrames(tester, 24);
      await tester.drag(
        find.byKey(const Key("library-photo-wall")),
        const Offset(0, -420),
      );
      await _pumpFrames(tester, 24);
      final before = _visibleImages(tester);
      expect(before.length, greaterThan(2));
      final controller = container.read(libraryControllerProvider.notifier);
      final loading = controller.loadPreviousPage();
      catalog.page.completeError(
        const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The loaded cursor predates preview metadata",
        ),
      );
      await _pumpFrames(tester, 4);
      expect(catalog.anchors, hasLength(1));
      expect(catalog.anchors.single, isNotNull);
      synchronization.publish(BigInt.two);
      await _pumpFrames(tester, 4);
      expect(catalog.anchors, hasLength(1));
      catalog.recovery.complete();
      await _pumpFrames(tester, 8);
      await loading;
      expect(
        container.read(libraryControllerProvider).catalogRevision,
        BigInt.two,
      );
      expect(_visibleImages(tester), before);
      expect(tester.takeException(), isNull);
      await tester.pumpWidget(const SizedBox.shrink());
    },
    variant: TargetPlatformVariant.only(TargetPlatform.windows),
  );
}

Future<void> _pumpFrames(WidgetTester tester, int count) async {
  for (var frame = 0; frame < count; frame += 1) {
    await tester.pump(const Duration(milliseconds: 16));
  }
}

Map<String, Rect> _visibleImages(WidgetTester tester) {
  final wall = tester.getRect(find.byKey(const Key("library-photo-wall")));
  final result = <String, Rect>{};
  for (final element in find.byType(LibraryPhotoTile).evaluate()) {
    final rectangle = tester.getRect(
      find.byElementPredicate((item) => identical(item, element)),
    );
    if (wall.contains(rectangle.center)) {
      result[(element.widget as LibraryPhotoTile).asset.locationId] = rectangle;
    }
  }
  return result;
}

class _Catalog with LibraryQuerySnapshotFixture implements LibraryCatalog {
  _Catalog(this.previewPath);

  final String previewPath;
  final page = Completer<LibrarySnapshot>();
  final recovery = Completer<void>();
  final anchors = <LibraryQueryAnchor?>[];

  LibrarySnapshot snapshot(
    int revision, {
    required int start,
    LibraryQueryAnchor? anchor,
  }) => LibrarySnapshot(
    catalogPath: "fixture.sqlite3",
    revision: BigInt.from(revision),
    queryId: "middle-query",
    roots: const [
      LibraryRoot(
        id: "root",
        path: "fixture",
        displayPath: "fixture",
        activeScanId: "scan",
        createdUnixMs: 1,
        assetCount: 10000,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
      ),
    ],
    assets: List.generate(120, (index) => asset(start + index)),
    previousCursor: LibraryCatalogCursor(
      revision: BigInt.from(revision),
      queryId: "middle-query",
      primaryMissing: false,
      primaryText: "2012-05",
      primaryNumber: 1,
      rootId: "root",
      locationId: "location-$start",
    ),
    queryAnchorResolution: anchor == null
        ? null
        : LibraryQueryAnchorResolution(
            requestedLocationId: anchor.requestedLocationId,
            locationId: anchor.requestedLocationId,
            ordinal: anchor.fallbackGlobalItemIndex,
            windowStartItemOffset: start,
          ),
  );

  LibraryTimeline timeline(int revision) => LibraryTimeline(
    revision: BigInt.from(revision),
    queryId: "middle-query",
    totalItems: 10000,
    buckets: const [
      LibraryTimeBucket(
        monthKey: "2026-09",
        itemCount: 4000,
        aspectRatioSum: 4000 * 4 / 3,
      ),
      LibraryTimeBucket(
        monthKey: "2012-05",
        itemCount: 6000,
        aspectRatioSum: 6000 * 4 / 3,
      ),
    ],
  );

  LibraryAsset asset(int index) => LibraryAsset(
    assetId: "asset-$index",
    locationId: "location-$index",
    rootId: "root",
    activeScanId: "scan",
    sourcePath: "fixture-$index.png",
    displayPath: "fixture-$index.png",
    relativePath: "fixture-$index.png",
    previewPath: previewPath,
    fileSize: BigInt.one,
    modifiedUnixMs: DateTime.utc(2012, 5, 2).millisecondsSinceEpoch,
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
  }) => page.future;

  @override
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) async {
    anchors.add(anchor);
    await recovery.future;
    return LibraryQuerySnapshot(
      snapshot: snapshot(2, start: anchor == null ? 0 : 4000, anchor: anchor),
      timeline: timeline(2),
    );
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async =>
      timeline(1);

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async => snapshot(1, start: 4000);

  @override
  Future<bool> unregisterRoot(String rootId) =>
      throw UnimplementedError("No root removal in this fixture");
}

class _Synchronization extends InertLibrarySynchronization {
  final _updates = StreamController<LibrarySynchronizationSnapshot>.broadcast();

  @override
  Stream<LibrarySynchronizationSnapshot> watch() => _updates.stream;

  void publish(BigInt revision) {
    _updates.add(
      LibrarySynchronizationSnapshot(
        isRunning: true,
        catalogRevision: revision,
        appliedMutationCount: 1,
        roots: const {},
      ),
    );
  }

  @override
  Future<void> dispose() => _updates.close();
}
