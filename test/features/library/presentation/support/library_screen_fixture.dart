import "dart:async";

import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_layout_manifest_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/gallery_layout_manifest.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter/material.dart";
import "package:flutter_test/flutter_test.dart";

import "../../support/library_query_snapshot_fixture.dart";

LibraryAsset galleryAsset({
  required String id,
  required String captureLocalTime,
}) {
  return LibraryAsset(
    assetId: "asset-$id",
    locationId: id,
    rootId: "root-1",
    activeScanId: "scan-1",
    sourceRevision: null,
    sourceGeneration: BigInt.one,
    sourcePath: "C:\\Pictures\\$id.png",
    displayPath: "C:\\Pictures\\$id.png",
    relativePath: "$id.png",
    previewPath: "C:\\Missing\\$id.jpg",
    fileSize: BigInt.one,
    modifiedUnixMs: 1,
    width: 4,
    height: 3,
    captureTime: LibraryCaptureTimeEvidence(
      localTime: captureLocalTime,
      source: LibraryCaptureTimeSource.exifDateTimeOriginal,
      rawValue: captureLocalTime,
    ),
  );
}

LibraryState populatedScreenState({
  required int totalItems,
  int assetWidth = 4,
  int assetHeight = 3,
}) {
  final snapshot = LibrarySnapshot(
    catalogPath: "C:\\AmeData\\ame.sqlite3",
    revision: BigInt.one,
    queryId: "query-1",
    roots: [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        activeScanId: "scan-1",
        createdUnixMs: 1,
        assetCount: totalItems,
        issueCount: 0,
        availability: LibraryRootAvailability.available,
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
        modifiedUnixMs: 1,
        width: assetWidth,
        height: assetHeight,
      ),
    ],
  );
  return LibraryState.fromSnapshot(snapshot).copyWith(
    timeline: LibraryTimeline(
      revision: BigInt.one,
      queryId: "query-1",
      totalItems: totalItems,
      buckets: [
        LibraryTimeBucket(
          itemCount: totalItems,
          aspectRatioSum: totalItems.toDouble(),
        ),
      ],
    ),
  );
}

ScrollPosition galleryScrollPosition(WidgetTester tester) {
  final scrollable = find.descendant(
    of: find.byKey(const Key("library-photo-wall")),
    matching: find.byType(Scrollable),
  );
  return tester.state<ScrollableState>(scrollable).position;
}

class ControlledLayoutManifestLoader
    implements LibraryGalleryLayoutManifestLoader {
  final Completer<LibraryGalleryLayoutManifest> _completer = Completer();

  @override
  Future<LibraryGalleryLayoutManifest> load(
    LibraryGalleryQuery query, {
    bool Function()? isCancelled,
  }) {
    return _completer.future;
  }

  void complete(LibraryGalleryLayoutManifest manifest) {
    _completer.complete(manifest);
  }
}

class RecordingQueryCatalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog {
  RecordingQueryCatalog(this.snapshot, {this.previousSnapshot});

  final LibrarySnapshot snapshot;
  final LibrarySnapshot? previousSnapshot;
  final List<LibraryGalleryQuery> loadQueries = [];
  final List<LibraryTimeAnchor> timeAnchors = [];
  final List<LibraryCatalogCursor?> beforeCursors = [];

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    loadQueries.add(query);
    beforeCursors.add(before);
    return before == null ? snapshot : previousSnapshot ?? snapshot;
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    loadQueries.add(query);
    timeAnchors.add(anchor);
    return snapshot;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      totalItems: snapshot.assets.length,
      buckets: [
        LibraryTimeBucket(
          itemCount: snapshot.assets.length,
          aspectRatioSum: snapshot.assets.length.toDouble(),
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => true;
}

class AnchorResolvingQueryCatalog
    with LibraryQuerySnapshotFixture
    implements LibraryCatalog, LibraryQueryAnchorCatalog {
  AnchorResolvingQueryCatalog(this.initialSnapshot);

  final LibrarySnapshot initialSnapshot;
  final List<String> requestedLocationIds = [];
  bool shouldResolveAnchor = true;
  bool holdNextAnchorRequest = false;
  Completer<void>? _heldAnchorRequest;
  int _generation = 0;
  LibrarySnapshot? lastSnapshot;

  @override
  Future<LibrarySnapshot> loadAroundLocation({
    required int maxItems,
    required LibraryGalleryQuery query,
    required String anchorLocationId,
  }) async {
    requestedLocationIds.add(anchorLocationId);
    if (holdNextAnchorRequest) {
      holdNextAnchorRequest = false;
      _heldAnchorRequest = Completer<void>();
      await _heldAnchorRequest!.future;
    }
    final reordered = List<LibraryAsset>.of(initialSnapshot.assets.reversed);
    final requestedIndex = reordered.indexWhere(
      (asset) => asset.locationId == anchorLocationId,
    );
    if (requestedIndex >= 0) {
      final targetOrdinal = (_generation.isEven ? 62 : 38).clamp(
        0,
        reordered.length - 1,
      );
      final requested = reordered.removeAt(requestedIndex);
      reordered.insert(targetOrdinal, requested);
    }
    _generation += 1;
    final queryId = "sort-anchor-result-$_generation";
    final resolution = shouldResolveAnchor
        ? LibraryQueryAnchorResolution(
            requestedLocationId: anchorLocationId,
            locationId: anchorLocationId,
            ordinal: reordered.indexWhere(
              (asset) => asset.locationId == anchorLocationId,
            ),
            windowStartItemOffset: 0,
          )
        : null;
    return lastSnapshot = LibrarySnapshot(
      catalogPath: initialSnapshot.catalogPath,
      revision: initialSnapshot.revision,
      queryId: queryId,
      roots: initialSnapshot.roots,
      assets: reordered,
      queryAnchorResolution: resolution,
    );
  }

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    return initialSnapshot;
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    return initialSnapshot;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    final snapshot = lastSnapshot ?? initialSnapshot;
    return LibraryTimeline(
      revision: snapshot.revision,
      queryId: snapshot.queryId,
      totalItems: snapshot.assets.length,
      buckets: [
        LibraryTimeBucket(
          monthKey: "2026-08",
          itemCount: snapshot.assets.length,
          aspectRatioSum: snapshot.assets.length * 4 / 3,
        ),
      ],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async => true;

  void releaseHeldAnchorRequest() {
    final request = _heldAnchorRequest;
    if (request != null && !request.isCompleted) {
      request.complete();
    }
    _heldAnchorRequest = null;
  }
}

class NoopLibraryScanner implements LibraryScanner {
  @override
  Future<void> cancelRetainedScan(String scanId) async {
    throw StateError("This fixture has no retained cancellation command");
  }

  const NoopLibraryScanner();

  @override
  bool cancel(String scanId) => false;

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => null;

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async => null;

  @override
  bool pause(String scanId) => false;

  @override
  bool suspend(String scanId) => false;

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return const Stream.empty();
  }

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    return const Stream.empty();
  }
}
