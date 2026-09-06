import "dart:async";

import "package:cedarflake_ame/features/library/adapters/directory_picker.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

Future<void> flushRetainedScanMicrotasks() async {
  await Future<void>.delayed(Duration.zero);
  await Future<void>.delayed(Duration.zero);
}

const retainedScanCheckpoint = RecoverableLibraryScan(
  scanId: "retained-first-import",
  rootPath: "C:\\Retained",
  displayRootPath: "C:\\Retained",
  previewEdge: 384,
  visitedEntries: 128,
  acceptedItems: 40,
  issueCount: 3,
);

const retainedScanRoot = LibraryRoot(
  id: "retained",
  path: "C:\\Retained",
  displayPath: "C:\\Retained",
  createdUnixMs: 1,
  assetCount: 0,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);
const otherPublishedRoot = LibraryRoot(
  id: "other",
  path: "C:\\Other",
  displayPath: "C:\\Other",
  activeScanId: "published",
  createdUnixMs: 1,
  assetCount: 6,
  issueCount: 0,
  availability: LibraryRootAvailability.available,
);

class RetainedScanFixture {
  RetainedScanFixture() {
    catalog = RetainedScanCatalog(scanner);
    container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(catalog),
        directoryPickerProvider.overrideWithValue(picker),
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(
            catalog.snapshot(const LibraryGalleryQuery()),
          ),
        ),
      ],
    );
    addTearDown(dispose);
  }

  final scanner = RetainedScanScanner();
  final picker = RetainedScanPicker();
  late final RetainedScanCatalog catalog;
  late final ProviderContainer container;
  bool _isDisposed = false;
  LibraryController get controller =>
      container.read(libraryControllerProvider.notifier);
  LibraryState get state => container.read(libraryControllerProvider);

  Future<void> restore() async {
    controller;
    await flushRetainedScanMicrotasks();
  }

  void dispose() {
    if (_isDisposed) {
      return;
    }
    _isDisposed = true;
    container.dispose();
    scanner.dispose();
  }
}

class RetainedScanPicker implements DirectoryPicker {
  int calls = 0;
  @override
  Future<String?> pickDirectory() async {
    calls += 1;
    return "C:\\New";
  }
}

class RetainedScanScanner implements LibraryScanner {
  RecoverableLibraryScan? checkpoint = retainedScanCheckpoint;
  Completer<RecoverableLibraryScan?>? pendingRecovery;
  Completer<void>? pendingCancellation;
  Object? recoveryFailure;
  Object? cancelFailure;
  final List<String> cancelRetainedIds = [];
  final List<String> cancelActiveIds = [];
  final List<String> startedRoots = [];
  final List<String> resumedIds = [];
  final Map<String, StreamController<LibraryScanUpdate>> _streams = {};

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async {
    if (recoveryFailure case final error?) {
      throw error;
    }
    return pendingRecovery == null ? checkpoint : await pendingRecovery!.future;
  }

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async => checkpoint;
  @override
  Future<void> cancelRetainedScan(String scanId) async {
    cancelRetainedIds.add(scanId);
    if (cancelFailure case final error?) {
      throw error;
    }
    await pendingCancellation?.future;
    checkpoint = null;
  }

  @override
  bool cancel(String scanId) {
    cancelActiveIds.add(scanId);
    return true;
  }

  @override
  bool pause(String scanId) => true;
  @override
  bool suspend(String scanId) => true;
  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    startedRoots.add(rootPath);
    return _streams
        .putIfAbsent(scanId, StreamController<LibraryScanUpdate>.new)
        .stream;
  }

  @override
  Stream<LibraryScanUpdate> resume({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    resumedIds.add(scanId);
    return scan(
      scanId: scanId,
      rootPath: rootPath,
      itemLimit: itemLimit,
      entryLimit: entryLimit,
      previewEdge: previewEdge,
    );
  }

  void finishUpdates() {
    for (final stream in _streams.values) {
      if (!stream.isClosed) {
        stream.add(const LibraryScanCancelled(acceptedItems: 0, issueCount: 0));
        unawaited(stream.close());
      }
    }
  }

  void completeScan(String scanId) {
    checkpoint = null;
    final stream = _streams[scanId]!;
    stream.add(
      const LibraryScanCompleted(
        assetCount: 6,
        issueCount: 0,
        catalogPath: "catalog.sqlite3",
        wasLimited: false,
      ),
    );
    unawaited(stream.close());
  }

  void failUpdates() {
    for (final stream in _streams.values) {
      if (!stream.isClosed) {
        stream.add(
          const LibraryScanFailed(
            code: "fixture_update_failed",
            message: "更新夹具失败",
          ),
        );
        unawaited(stream.close());
      }
    }
  }

  void dispose() {
    for (final stream in _streams.values) {
      if (!stream.isClosed) {
        unawaited(stream.close());
      }
    }
  }
}

class RetainedScanCatalog implements LibraryCatalog {
  RetainedScanCatalog(this.scanner);
  final RetainedScanScanner scanner;
  final List<LibraryRoot> roots = [retainedScanRoot, otherPublishedRoot];
  final List<String> removedIds = [];
  Completer<LibrarySnapshot>? pendingLoad;
  bool failAfterRemoval = false;
  Object? loadFailure;
  Object? timelineFailure;
  int firstLoads = 0;
  int afterLoads = 0;
  int beforeLoads = 0;
  int timeLoads = 0;

  String _queryId(LibraryGalleryQuery query) => "query-${query.rootId}";
  LibraryCatalogCursor _cursor(LibraryGalleryQuery query, String id) =>
      LibraryCatalogCursor(
        revision: BigInt.one,
        queryId: _queryId(query),
        primaryMissing: false,
        primaryText: "",
        primaryNumber: 0,
        rootId: "other",
        locationId: id,
      );
  LibrarySnapshot snapshot(LibraryGalleryQuery query, {int offset = 0}) =>
      LibrarySnapshot(
        roots: List.of(roots),
        assets: [_asset(offset)],
        catalogPath: "catalog.sqlite3",
        revision: BigInt.one,
        queryId: _queryId(query),
        nextCursor: _cursor(query, "next"),
        previousCursor: _cursor(query, "previous"),
      );
  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    if (loadFailure case final failure?) {
      throw failure;
    }
    if (failAfterRemoval && removedIds.isNotEmpty) {
      throw StateError("reload failed after commit");
    }
    if (after != null) {
      afterLoads += 1;
      return snapshot(query, offset: 1);
    }
    if (before != null) {
      beforeLoads += 1;
      return snapshot(query, offset: 2);
    }
    final pending = pendingLoad;
    firstLoads += 1;
    pendingLoad = null;
    return pending == null ? snapshot(query) : await pending.future;
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    if (timelineFailure case final failure?) {
      throw failure;
    }
    return LibraryTimeline(
      revision: BigInt.one,
      queryId: _queryId(query),
      totalItems: 6,
      buckets: const [
        LibraryTimeBucket(monthKey: "2026-09", itemCount: 6, aspectRatioSum: 6),
      ],
    );
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    timeLoads += 1;
    return snapshot(query, offset: 3);
  }

  @override
  Future<bool> unregisterRoot(String rootId) async {
    removedIds.add(rootId);
    roots.removeWhere((root) => root.id == rootId);
    if (rootId == "retained") {
      scanner.checkpoint = null;
    }
    return true;
  }
}

LibraryAsset _asset(int index) => LibraryAsset(
  assetId: "asset-$index",
  locationId: "location-$index",
  rootId: "other",
  activeScanId: "published",
  sourcePath: "C:\\Other\\$index.png",
  displayPath: "C:\\Other\\$index.png",
  relativePath: "$index.png",
  previewPath: "",
  fileSize: BigInt.from(128),
  modifiedUnixMs: 1,
  sourceRevision: null,
  sourceGeneration: BigInt.one,
  width: 32,
  height: 32,
  previewStatus: LibraryPreviewStatus.pending,
);
