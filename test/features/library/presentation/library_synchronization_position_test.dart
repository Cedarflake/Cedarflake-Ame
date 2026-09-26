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
import "package:cedarflake_ame/features/library/presentation/widgets/library_photo_tile.dart";
import "package:flutter/material.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

import "../support/concurrent_library_scanner.dart";
import "../support/library_query_snapshot_fixture.dart";

void main() {
  for (final lateFailure in [false, true]) {
    testWidgets(
      "passive refresh retires after upward input, late failure: $lateFailure",
      (tester) async {
        tester.view.physicalSize = const Size(1280, 800);
        tester.view.devicePixelRatio = 1;
        addTearDown(tester.view.resetPhysicalSize);
        addTearDown(tester.view.resetDevicePixelRatio);
        final directory = Directory.systemTemp.createTempSync(
          "ame-sync-position-",
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
            librarySynchronizationProvider.overrideWithValue(
              InertLibrarySynchronization(),
            ),
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
          UncontrolledProviderScope(
            container: container,
            child: const AmeApp(),
          ),
        );
        await _pumpFrames(tester);
        final wall = find.byKey(const Key("library-photo-wall"));
        await tester.drag(wall, const Offset(0, -900));
        await _pumpFrames(tester);
        final controller = container.read(libraryControllerProvider.notifier);
        final old = controller.refreshFromSynchronization(
          catalogRevision: BigInt.two,
        );
        expect(catalog.reads, hasLength(1));
        final oldAnchor = catalog.reads.single.anchor!;

        await tester.drag(wall, const Offset(0, 420));
        await _pumpFrames(tester);
        final visibleAfterScroll = _visibleImages(tester);
        expect(visibleAfterScroll.length, greaterThan(2));
        if (lateFailure) {
          catalog.reads.first.result.completeError(
            const LibraryCatalogFailure(
              code: "catalog_database_busy",
              message: "The retired snapshot could not be read",
            ),
          );
        } else {
          catalog.complete(0, 2);
        }
        expect(await old, LibraryQueryUpdateOutcome.superseded);
        final retained = container.read(libraryControllerProvider);
        expect(retained.catalogRevision, BigInt.one);
        expect(retained.windowStartItemOffset, 4000);
        expect(retained.isLoadingTimeline, isFalse);
        expect(retained.errorMessage, isNull);

        final current = controller.refreshFromSynchronization(
          catalogRevision: BigInt.two,
        );
        final currentAnchor = catalog.reads.last.anchor!;
        expect(
          currentAnchor.requestedLocationId,
          isNot(oldAnchor.requestedLocationId),
        );
        catalog.complete(1, 2);
        expect(await current, LibraryQueryUpdateOutcome.applied);
        // The next revision may arrive before the just-published frame renders.
        final followOn = controller.refreshFromSynchronization(
          catalogRevision: BigInt.from(3),
        );
        expect(
          catalog.reads.last.anchor?.requestedLocationId,
          currentAnchor.requestedLocationId,
        );
        catalog.complete(2, 3);
        expect(await followOn, LibraryQueryUpdateOutcome.applied);
        await _pumpFrames(tester);
        expect(
          container.read(libraryControllerProvider).windowStartItemOffset,
          4000,
        );
        expect(_visibleImages(tester), visibleAfterScroll);
        expect(tester.takeException(), isNull);
        await tester.pumpWidget(const SizedBox.shrink());
      },
      variant: TargetPlatformVariant.only(TargetPlatform.windows),
    );
  }
}

Future<void> _pumpFrames(WidgetTester tester) async {
  for (var frame = 0; frame < 24; frame += 1) {
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

class _Read {
  _Read(this.anchor);
  final LibraryQueryAnchor? anchor;
  final result = Completer<LibraryQuerySnapshot>();
}

class _Catalog with LibraryQuerySnapshotFixture implements LibraryCatalog {
  _Catalog(this.previewPath);
  final String previewPath;
  final reads = <_Read>[];

  void complete(int readIndex, int revision) {
    final read = reads[readIndex];
    read.result.complete(
      LibraryQuerySnapshot(
        snapshot: snapshot(
          revision,
          start: read.anchor == null ? 0 : 4000,
          anchor: read.anchor,
        ),
        timeline: timeline(revision),
      ),
    );
  }

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
  Future<LibraryQuerySnapshot> loadQuerySnapshot({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryQueryAnchor? anchor,
  }) {
    final read = _Read(anchor);
    reads.add(read);
    return read.result.future;
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async => snapshot(1, start: 4000);

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
      throw UnimplementedError("No removal in this fixture");
}
