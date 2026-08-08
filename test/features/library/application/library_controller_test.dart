import "dart:async";

import "package:cedarflake_ame/features/library/adapters/directory_picker.dart";
import "package:cedarflake_ame/features/library/application/library_catalog.dart";
import "package:cedarflake_ame/features/library/application/library_controller.dart";
import "package:cedarflake_ame/features/library/application/library_previewer.dart";
import "package:cedarflake_ame/features/library/application/library_scanner.dart";
import "package:cedarflake_ame/features/library/domain/library_models.dart";
import "package:cedarflake_ame/features/library/domain/library_state.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";
import "package:flutter_test/flutter_test.dart";

void main() {
  test("publishes streamed assets only after a completed scan event", () async {
    final scanner = _FakeLibraryScanner();
    final catalog = _FakeLibraryCatalog(
      _snapshot(
        roots: const [
          LibraryRoot(
            id: "root-1",
            path: r"\\?\C:\Pictures",
            displayPath: "C:\\Pictures",
            activeScanId: "scan-test",
            createdUnixMs: 1,
            assetCount: 1,
            issueCount: 0,
          ),
        ],
        assets: [_asset()],
      ),
    );
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker(r"\\?\C:\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();

    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.scanning,
    );
    expect(scanner.startedItemLimit, isNull);
    expect(scanner.startedEntryLimit, isNull);

    scanner.add(
      const LibraryScanStarted(scanId: "scan-test", rootPath: "C:\\Pictures"),
    );
    scanner.add(
      LibraryAssetDiscovered(
        LibraryAsset(
          assetId: "asset-1",
          locationId: "location-1",
          rootId: "root-1",
          sourcePath: "C:\\Pictures\\one.png",
          displayPath: "C:\\Pictures\\one.png",
          relativePath: "one.png",
          previewPath: "C:\\AmeCache\\one.jpg",
          fileSize: BigInt.from(128),
          modifiedUnixMs: 42,
          width: 320,
          height: 240,
        ),
      ),
    );
    await Future<void>.delayed(Duration.zero);
    expect(
      container.read(libraryControllerProvider).rootPath,
      r"\\?\C:\Pictures",
    );
    expect(
      container.read(libraryControllerProvider).displayRootPath,
      r"C:\Pictures",
    );
    expect(container.read(libraryControllerProvider).assets, isEmpty);
    scanner.add(
      const LibraryScanCompleted(
        assetCount: 1,
        issueCount: 0,
        catalogPath: "C:\\AmeData\\ame.sqlite3",
        wasLimited: false,
      ),
    );
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    final state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.completed);
    expect(state.assets.single.relativePath, "1.png");
    expect(state.catalogPath, "C:\\AmeData\\ame.sqlite3");
  });

  test("forwards cancellation to the active Rust scan", () async {
    final scanner = _FakeLibraryScanner();
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker("C:\\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();
    controller.cancelScan();

    expect(scanner.cancelledScanId, isNotNull);
    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.cancelling,
    );
  });

  test("keeps source changes distinct from generic failures", () async {
    final scanner = _FakeLibraryScanner();
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker("C:\\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();
    scanner.add(const LibraryScanStale(acceptedItems: 1, issueCount: 1));
    await Future<void>.delayed(Duration.zero);

    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.stale,
    );
  });

  test("automatically resumes a persisted interrupted scan", () async {
    final scanner = _FakeLibraryScanner(
      recoverableScan: const RecoverableLibraryScan(
        scanId: "scan-recover",
        rootPath: r"\\?\C:\Pictures",
        displayRootPath: "C:\\Pictures",
        itemLimit: 500,
        entryLimit: 2000,
        previewEdge: 512,
        visitedEntries: 128,
        acceptedItems: 40,
        issueCount: 3,
      ),
    );
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    container.read(libraryControllerProvider);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    final state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.scanning);
    expect(state.isResumingScan, isTrue);
    expect(state.scanId, "scan-recover");
    expect(state.visitedEntries, 128);
    expect(state.stagedAssetCount, 40);
    expect(state.issueCount, 3);
    expect(scanner.startedScanId, "scan-recover");
  });

  test("restores a paused scan without starting it until resume", () async {
    final scanner = _FakeLibraryScanner(
      pausedScan: const RecoverableLibraryScan(
        scanId: "scan-paused",
        rootPath: r"\\?\C:\Pictures",
        displayRootPath: "C:\\Pictures",
        itemLimit: 500,
        entryLimit: 2000,
        previewEdge: 512,
        visitedEntries: 64,
        acceptedItems: 20,
        issueCount: 2,
      ),
    );
    final container = ProviderContainer(
      overrides: [
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    var state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.paused);
    expect(state.visitedEntries, 64);
    expect(scanner.startedScanId, isNull);

    await controller.resumePausedScan();
    state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.scanning);
    expect(state.isResumingScan, isTrue);
    expect(scanner.startedScanId, "scan-paused");
  });

  test("forwards pause and keeps the staged scan private", () async {
    final scanner = _FakeLibraryScanner();
    final container = ProviderContainer(
      overrides: [
        directoryPickerProvider.overrideWithValue(
          const _FakeDirectoryPicker("C:\\Pictures"),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(_snapshot()),
        ),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    await controller.chooseDirectoryAndScan();
    final scanId = container.read(libraryControllerProvider).scanId;
    controller.pauseScan();

    expect(scanner.pausedScanId, scanId);
    expect(
      container.read(libraryControllerProvider).status,
      LibraryStatus.pausing,
    );

    scanner.add(
      const LibraryScanPaused(
        visitedEntries: 10,
        acceptedItems: 4,
        issueCount: 1,
      ),
    );
    await Future<void>.delayed(Duration.zero);

    final state = container.read(libraryControllerProvider);
    expect(state.status, LibraryStatus.paused);
    expect(state.assets, isEmpty);
    expect(state.stagedAssetCount, 4);
  });

  test("merges keyset pages without duplicating a location", () async {
    final cursor = _cursor();
    const roots = [
      LibraryRoot(
        id: "root-1",
        path: "C:\\Pictures",
        displayPath: "C:\\Pictures",
        activeScanId: "scan-test",
        createdUnixMs: 1,
        assetCount: 2,
        issueCount: 0,
      ),
    ];
    final initialSnapshot = _snapshot(
      roots: roots,
      assets: [_asset()],
      nextCursor: cursor,
    );
    final catalog = _FakeLibraryCatalog.sequence([
      _snapshot(
        roots: roots,
        assets: [
          _asset(previewPath: "C:\\AmeCache\\one-updated.jpg"),
          _asset(suffix: "2"),
        ],
      ),
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    await container.read(libraryControllerProvider.notifier).loadNextPage();

    final state = container.read(libraryControllerProvider);
    expect(state.assets, hasLength(2));
    expect(state.assets.first.previewPath, endsWith("one-updated.jpg"));
    expect(state.assets.last.locationId, "location-2");
    expect(state.nextCursor, isNull);
    expect(catalog.afters.single, same(cursor));
  });

  test("loads both sides of a bounded window after a timeline jump", () async {
    const bucket = LibraryTimeBucket(
      monthKey: "2024-05",
      itemCount: 18,
      aspectRatioSum: 23.5,
    );
    final initialSnapshot = _snapshot(
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 19,
          issueCount: 0,
        ),
      ],
      assets: [_asset(suffix: "newest")],
    );
    final previousCursor = _cursor(suffix: "anchor-boundary");
    final nextCursor = _cursor(suffix: "next-boundary");
    final anchoredSnapshot = _snapshot(
      roots: initialSnapshot.roots,
      assets: [_asset(suffix: "anchor")],
      previousCursor: previousCursor,
      nextCursor: nextCursor,
    );
    final catalog = _FakeLibraryCatalog.sequence([
      anchoredSnapshot,
      _snapshot(
        roots: initialSnapshot.roots,
        assets: [_asset(suffix: "newer")],
      ),
    ], initialRevision: initialSnapshot.revision);
    final initialState = LibraryState.fromSnapshot(initialSnapshot).copyWith(
      timeline: LibraryTimeline(
        revision: initialSnapshot.revision,
        queryId: initialSnapshot.queryId,
        totalItems: 19,
        buckets: const [bucket],
      ),
    );
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(initialState),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    final didJump = await container
        .read(libraryControllerProvider.notifier)
        .jumpToTime(bucket, itemOffset: 9);

    final state = container.read(libraryControllerProvider);
    expect(didJump, isTrue);
    expect(state.assets.single.locationId, "location-anchor");
    expect(state.previousCursor, same(previousCursor));
    expect(state.nextCursor, isNotNull);
    expect(state.activeTimeAnchor?.monthKey, "2024-05");
    expect(state.activeTimeAnchor?.itemOffset, 9);
    expect(state.windowStartItemOffset, 9);
    expect(catalog.anchors, hasLength(1));
    expect(catalog.anchors.single.revision, initialSnapshot.revision);
    expect(catalog.anchors.single.itemOffset, 9);

    final loadedPrevious = await container
        .read(libraryControllerProvider.notifier)
        .loadPreviousPage();
    final expandedState = container.read(libraryControllerProvider);

    expect(loadedPrevious, isTrue);
    expect(expandedState.assets.map((asset) => asset.locationId), [
      "location-newer",
      "location-anchor",
    ]);
    expect(expandedState.previousCursor, isNull);
    expect(expandedState.nextCursor, same(nextCursor));
    expect(expandedState.windowStartItemOffset, 8);
    expect(catalog.befores.single, same(previousCursor));
  });

  test("unregisters one root and reloads the remaining catalog", () async {
    const removedRoot = LibraryRoot(
      id: "root-remove",
      path: "C:\\Remove",
      displayPath: "C:\\Remove",
      activeScanId: "scan-remove",
      createdUnixMs: 1,
      assetCount: 1,
      issueCount: 0,
    );
    const keptRoot = LibraryRoot(
      id: "root-keep",
      path: "C:\\Keep",
      displayPath: "C:\\Keep",
      activeScanId: "scan-keep",
      createdUnixMs: 2,
      assetCount: 1,
      issueCount: 0,
    );
    final initialSnapshot = _snapshot(
      roots: const [removedRoot, keptRoot],
      assets: [_asset()],
    );
    final refreshedSnapshot = _snapshot(
      revision: BigInt.two,
      roots: const [keptRoot],
      assets: [_asset(suffix: "kept")],
    );
    final catalog = _FakeLibraryCatalog.sequence([
      refreshedSnapshot,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    final didRemove = await container
        .read(libraryControllerProvider.notifier)
        .unregisterRoot(removedRoot);

    final state = container.read(libraryControllerProvider);
    expect(didRemove, isTrue);
    expect(catalog.unregisteredRootIds, ["root-remove"]);
    expect(state.roots, const [keptRoot]);
    expect(state.assets.single.locationId, "location-kept");
    expect(state.catalogRevision, BigInt.two);
  });

  test("refreshes the first page when a keyset cursor becomes stale", () async {
    final cursor = _cursor();
    final initialSnapshot = _snapshot(
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          activeScanId: "scan-1",
          createdUnixMs: 1,
          assetCount: 2,
          issueCount: 0,
        ),
      ],
      assets: [_asset()],
      nextCursor: cursor,
    );
    final refreshedSnapshot = _snapshot(
      revision: BigInt.two,
      roots: const [
        LibraryRoot(
          id: "root-1",
          path: "C:\\Pictures",
          displayPath: "C:\\Pictures",
          activeScanId: "scan-2",
          createdUnixMs: 1,
          assetCount: 1,
          issueCount: 0,
        ),
      ],
      assets: [_asset(suffix: "fresh")],
    );
    final catalog = _FakeLibraryCatalog.sequence([
      const LibraryCatalogFailure(
        code: "catalog_cursor_stale",
        message: "catalog changed",
      ),
      refreshedSnapshot,
    ], initialRevision: initialSnapshot.revision);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryCatalogProvider.overrideWithValue(catalog),
      ],
    );
    addTearDown(container.dispose);

    await container.read(libraryControllerProvider.notifier).loadNextPage();

    final state = container.read(libraryControllerProvider);
    expect(state.catalogRevision, BigInt.two);
    expect(state.assets.single.locationId, "location-fresh");
    expect(state.pageErrorMessage, isNull);
    expect(catalog.afters, [same(cursor), isNull]);
  });

  test("bounds preview work and starts the next visible request", () async {
    final scanner = _FakeLibraryScanner();
    final previewer = _FakeLibraryPreviewer();
    final initialSnapshot = _snapshot(
      assets: [_pendingAsset("1"), _pendingAsset("2"), _pendingAsset("3")],
    );
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(initialSnapshot),
        ),
        libraryPreviewerProvider.overrideWithValue(previewer),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    for (final asset in initialSnapshot.assets) {
      controller.requestPreview(asset);
    }

    expect(previewer.requests, ["location-1", "location-2"]);

    previewer.succeed("location-1", _asset(suffix: "1"));
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    expect(previewer.requests, ["location-1", "location-2", "location-3"]);
    expect(
      container.read(libraryControllerProvider).assets.first.previewStatus,
      LibraryPreviewStatus.ready,
    );
  });

  test("records a preview failure and requires an explicit retry", () async {
    final scanner = _FakeLibraryScanner();
    final previewer = _FakeLibraryPreviewer();
    final pending = _pendingAsset("1");
    final initialSnapshot = _snapshot(assets: [pending]);
    final container = ProviderContainer(
      overrides: [
        initialLibraryStateProvider.overrideWithValue(
          LibraryState.fromSnapshot(initialSnapshot),
        ),
        libraryScannerProvider.overrideWithValue(scanner),
        libraryCatalogProvider.overrideWithValue(
          _FakeLibraryCatalog(initialSnapshot),
        ),
        libraryPreviewerProvider.overrideWithValue(previewer),
      ],
    );
    addTearDown(container.dispose);
    addTearDown(scanner.dispose);

    final controller = container.read(libraryControllerProvider.notifier);
    controller.requestPreview(pending);
    previewer.fail("location-1", StateError("decoder stopped"));
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    final failed = container.read(libraryControllerProvider).assets.single;
    expect(failed.previewStatus, LibraryPreviewStatus.failed);
    expect(failed.previewIssueCode, "preview_request_failed");

    controller.requestPreview(failed);
    expect(previewer.requests, ["location-1"]);
    controller.requestPreview(failed, retry: true);
    expect(previewer.requests, ["location-1", "location-1"]);

    previewer.succeed("location-1", _asset(suffix: "1"), attempt: 1);
    await Future<void>.delayed(Duration.zero);
    await Future<void>.delayed(Duration.zero);

    expect(
      container.read(libraryControllerProvider).assets.single.previewStatus,
      LibraryPreviewStatus.ready,
    );
  });
}

LibraryAsset _asset({String suffix = "1", String? previewPath}) {
  return LibraryAsset(
    assetId: "asset-$suffix",
    locationId: "location-$suffix",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$suffix.png",
    displayPath: "C:\\Pictures\\$suffix.png",
    relativePath: "$suffix.png",
    previewPath: previewPath ?? "C:\\AmeCache\\$suffix.jpg",
    fileSize: BigInt.from(128),
    modifiedUnixMs: 42,
    width: 320,
    height: 240,
  );
}

LibraryAsset _pendingAsset(String suffix) {
  return LibraryAsset(
    assetId: "asset-$suffix",
    locationId: "location-$suffix",
    rootId: "root-1",
    sourcePath: "C:\\Pictures\\$suffix.png",
    displayPath: "C:\\Pictures\\$suffix.png",
    relativePath: "$suffix.png",
    previewPath: "",
    fileSize: BigInt.from(128),
    modifiedUnixMs: 42,
    width: 320,
    height: 240,
    previewStatus: LibraryPreviewStatus.pending,
  );
}

LibrarySnapshot _snapshot({
  BigInt? revision,
  List<LibraryRoot> roots = const [],
  List<LibraryAsset> assets = const [],
  LibraryCatalogCursor? previousCursor,
  LibraryCatalogCursor? nextCursor,
}) {
  return LibrarySnapshot(
    catalogPath: "C:\\AmeData\\ame.sqlite3",
    revision: revision ?? BigInt.one,
    queryId: "query-1",
    roots: roots,
    assets: assets,
    previousCursor: previousCursor,
    nextCursor: nextCursor,
  );
}

LibraryCatalogCursor _cursor({String suffix = "1"}) {
  return LibraryCatalogCursor(
    revision: BigInt.one,
    queryId: "query-1",
    primaryMissing: true,
    primaryText: "",
    primaryNumber: 1,
    rootId: "root-1",
    locationId: "location-$suffix",
  );
}

class _FakeDirectoryPicker implements DirectoryPicker {
  const _FakeDirectoryPicker(this.path);

  final String? path;

  @override
  Future<String?> pickDirectory() async => path;
}

class _FakeLibraryScanner implements LibraryScanner {
  _FakeLibraryScanner({this.recoverableScan, this.pausedScan});

  final _controller = StreamController<LibraryScanUpdate>.broadcast();
  final RecoverableLibraryScan? recoverableScan;
  final RecoverableLibraryScan? pausedScan;
  String? cancelledScanId;
  String? pausedScanId;
  String? startedScanId;
  int? startedItemLimit;
  int? startedEntryLimit;

  void add(LibraryScanUpdate update) {
    _controller.add(update);
  }

  void dispose() {
    unawaited(_controller.close());
  }

  @override
  bool cancel(String scanId) {
    cancelledScanId = scanId;
    return true;
  }

  @override
  Future<RecoverableLibraryScan?> loadRecoverableScan() async {
    return recoverableScan;
  }

  @override
  Future<RecoverableLibraryScan?> loadPausedScan() async {
    return pausedScan;
  }

  @override
  bool pause(String scanId) {
    pausedScanId = scanId;
    return true;
  }

  @override
  Stream<LibraryScanUpdate> scan({
    required String scanId,
    required String rootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
  }) {
    startedScanId = scanId;
    startedItemLimit = itemLimit;
    startedEntryLimit = entryLimit;
    return _controller.stream;
  }
}

class _FakeLibraryCatalog implements LibraryCatalog {
  _FakeLibraryCatalog(LibrarySnapshot snapshot)
    : _responses = [snapshot],
      _lastRevision = snapshot.revision;

  _FakeLibraryCatalog.sequence(
    List<Object> responses, {
    required BigInt initialRevision,
  }) : _responses = List.of(responses),
       _lastRevision = initialRevision;

  final List<Object> _responses;
  final List<LibraryCatalogCursor?> afters = [];
  final List<LibraryCatalogCursor?> befores = [];
  final List<LibraryTimeAnchor> anchors = [];
  final List<String> unregisteredRootIds = [];
  BigInt _lastRevision;

  @override
  Future<LibrarySnapshot> load({
    required int maxItems,
    required LibraryGalleryQuery query,
    LibraryCatalogCursor? after,
    LibraryCatalogCursor? before,
  }) async {
    afters.add(after);
    befores.add(before);
    return _nextSnapshot();
  }

  @override
  Future<LibrarySnapshot> loadAtTime({
    required int maxItems,
    required LibraryGalleryQuery query,
    required LibraryTimeAnchor anchor,
  }) async {
    anchors.add(anchor);
    return _nextSnapshot();
  }

  @override
  Future<LibraryTimeline> loadTimeline(LibraryGalleryQuery query) async {
    return LibraryTimeline(
      revision: _lastRevision,
      queryId: "query-1",
      totalItems: 0,
      buckets: const [],
    );
  }

  @override
  Future<bool> unregisterRoot(String rootId) async {
    unregisteredRootIds.add(rootId);
    return true;
  }

  LibrarySnapshot _nextSnapshot() {
    if (_responses.isEmpty) {
      throw StateError("No fake catalog response remains");
    }
    final response = _responses.removeAt(0);
    if (response is LibrarySnapshot) {
      _lastRevision = response.revision;
      return response;
    }
    throw response;
  }
}

class _FakeLibraryPreviewer implements LibraryPreviewer {
  final List<String> requests = [];
  final Map<String, List<Completer<LibraryAsset>>> _attempts = {};

  @override
  Future<LibraryAsset> materialize({
    required String locationId,
    required int previewEdge,
  }) {
    requests.add(locationId);
    final completer = Completer<LibraryAsset>();
    _attempts.putIfAbsent(locationId, () => []).add(completer);
    return completer.future;
  }

  void succeed(String locationId, LibraryAsset asset, {int attempt = 0}) {
    _attempts[locationId]![attempt].complete(asset);
  }

  void fail(String locationId, Object error, {int attempt = 0}) {
    _attempts[locationId]![attempt].completeError(error);
  }
}
