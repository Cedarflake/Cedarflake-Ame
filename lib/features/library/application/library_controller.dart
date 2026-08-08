import "dart:async";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../adapters/directory_picker.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_preview_queue.dart";
import "library_previewer.dart";
import "library_scan_session.dart";
import "library_scanner.dart";

const _previewEdge = 512;
const _maxActivePreviews = 2;

class LibraryController extends Notifier<LibraryState> {
  StreamSubscription<LibraryScanUpdate>? _subscription;
  int _scanSequence = 0;
  final LibraryScanSession _scanSession = LibraryScanSession();
  LibraryPreviewQueue? _previewQueue;
  bool _isDisposed = false;

  LibraryPreviewQueue get _previews => _previewQueue ??= LibraryPreviewQueue(
    previewer: ref.read(libraryPreviewerProvider),
    previewEdge: _previewEdge,
    maxActive: _maxActivePreviews,
    onResult: _replaceAsset,
  );

  @override
  LibraryState build() {
    final scanner = ref.read(libraryScannerProvider);
    ref.onDispose(() {
      _isDisposed = true;
      final scanId = _scanSession.activeScanId;
      if (scanId != null) {
        scanner.cancel(scanId);
      }
      _previewQueue?.dispose();
      unawaited(_subscription?.cancel());
    });
    Future<void>.microtask(_resumeInterruptedScanIfAvailable);
    Future<void>.microtask(_loadInitialTimeline);
    return ref.watch(initialLibraryStateProvider);
  }

  Future<void> chooseDirectoryAndScan() async {
    if (state.isBusy) {
      return;
    }

    state = state.copyWith(
      status: LibraryStatus.choosingDirectory,
      errorMessage: null,
    );

    try {
      final directory = await ref.read(directoryPickerProvider).pickDirectory();
      if (directory == null) {
        state = state.copyWith(
          status: state.roots.isEmpty
              ? LibraryStatus.empty
              : LibraryStatus.completed,
        );
        return;
      }
      await scanDirectory(directory);
    } on Object catch (error) {
      state = state.copyWith(
        status: LibraryStatus.failed,
        errorMessage: error.toString(),
      );
    }
  }

  Future<void> scanDirectory(String rootPath) async {
    _scanSequence += 1;
    final scanId =
        "ame-${DateTime.now().microsecondsSinceEpoch}-$_scanSequence";
    await _startScan(
      scanId: scanId,
      rootPath: rootPath,
      itemLimit: null,
      entryLimit: null,
      previewEdge: _previewEdge,
    );
  }

  Future<void> _startScan({
    required String scanId,
    required String rootPath,
    String? displayRootPath,
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
    int visitedEntries = 0,
    int acceptedItems = 0,
    int issueCount = 0,
    bool isResuming = false,
  }) async {
    await _subscription?.cancel();
    _scanSession.begin(
      RecoverableLibraryScan(
        scanId: scanId,
        rootPath: rootPath,
        displayRootPath: displayRootPath ?? rootPath,
        itemLimit: itemLimit,
        entryLimit: entryLimit,
        previewEdge: previewEdge,
        visitedEntries: visitedEntries,
        acceptedItems: acceptedItems,
        issueCount: issueCount,
      ),
    );

    state = state.copyWith(
      status: LibraryStatus.scanning,
      scanId: scanId,
      rootPath: rootPath,
      displayRootPath: displayRootPath ?? rootPath,
      recentIssues: const [],
      visitedEntries: visitedEntries,
      stagedAssetCount: acceptedItems,
      issueCount: issueCount,
      itemLimit: itemLimit,
      entryLimit: entryLimit,
      isScanLimited: false,
      isResumingScan: isResuming,
      isLoadingPage: false,
      pageErrorMessage: null,
      errorMessage: null,
    );

    _subscription = ref
        .read(libraryScannerProvider)
        .scan(
          scanId: scanId,
          rootPath: rootPath,
          itemLimit: itemLimit,
          entryLimit: entryLimit,
          previewEdge: previewEdge,
        )
        .listen(
          _handleUpdate,
          onError: _handleError,
          onDone: _handleDone,
          cancelOnError: true,
        );
  }

  void cancelScan() {
    final scanId = state.scanId;
    if (scanId == null || !state.isScanning) {
      return;
    }
    if (ref.read(libraryScannerProvider).cancel(scanId)) {
      state = state.copyWith(status: LibraryStatus.cancelling);
    }
  }

  void pauseScan() {
    final scanId = state.scanId;
    if (scanId == null || state.status != LibraryStatus.scanning) {
      return;
    }
    if (ref.read(libraryScannerProvider).pause(scanId)) {
      state = state.copyWith(status: LibraryStatus.pausing);
    }
  }

  Future<void> resumePausedScan() async {
    final pausedScan = _scanSession.pausedScan;
    if (pausedScan != null && state.status == LibraryStatus.paused) {
      await _resumeScan(pausedScan);
    }
  }

  Future<void> retry() async {
    final scanner = ref.read(libraryScannerProvider);
    try {
      final recoverable = await scanner.loadRecoverableScan();
      if (recoverable != null) {
        await _resumeScan(recoverable);
        return;
      }
      final paused = await scanner.loadPausedScan();
      if (paused != null) {
        await _resumeScan(paused);
        return;
      }
    } on Object catch (error) {
      state = state.copyWith(
        status: LibraryStatus.failed,
        errorMessage: error.toString(),
      );
      return;
    }
    final rootPath = state.rootPath;
    if (rootPath != null) {
      await scanDirectory(rootPath);
    } else {
      await chooseDirectoryAndScan();
    }
  }

  Future<bool> updateQuery(LibraryGalleryQuery query) async {
    final normalized = query.copyWith(
      folderRelativePath: query.folderRelativePath
          ?.replaceAll("\\", "/")
          .replaceAll(RegExp(r"^/+|/+$"), ""),
      searchText: query.searchText.trim(),
    );
    if (normalized == state.query) {
      return true;
    }
    if (state.status == LibraryStatus.choosingDirectory ||
        state.isScanning ||
        state.status == LibraryStatus.paused ||
        state.isLoadingTimeAnchor) {
      return false;
    }
    final requestSequence = ++_scanSequence;
    _previewQueue?.clearPending();
    state = state.copyWith(
      status: LibraryStatus.refreshing,
      query: normalized,
      queryId: "",
      windowStartItemOffset: 0,
      assets: const [],
      previousCursor: null,
      nextCursor: null,
      timeline: null,
      activeTimeAnchor: null,
      isLoadingTimeline: true,
      pageErrorMessage: null,
      previousPageErrorMessage: null,
      timeNavigationErrorMessage: null,
      errorMessage: null,
    );
    try {
      await _reloadFirstCatalogPage(requestSequence);
      return !_isDisposed && requestSequence == _scanSequence;
    } on Object catch (error) {
      if (!_isDisposed && requestSequence == _scanSequence) {
        state = state.copyWith(
          status: LibraryStatus.failed,
          isLoadingTimeline: false,
          errorMessage: error.toString(),
        );
      }
      return false;
    }
  }

  Future<void> loadNextPage() async {
    final cursor = state.nextCursor;
    if (cursor == null ||
        state.isBusy ||
        state.isLoadingPage ||
        state.isLoadingPreviousPage ||
        state.isLoadingTimeAnchor) {
      return;
    }

    final scanSequence = _scanSequence;
    state = state.copyWith(isLoadingPage: true, pageErrorMessage: null);
    try {
      final snapshot = await ref
          .read(libraryCatalogProvider)
          .load(
            maxItems: libraryCatalogWindow,
            query: state.query,
            after: cursor,
          );
      if (_isDisposed || scanSequence != _scanSequence) {
        return;
      }
      if (snapshot.queryId != state.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The gallery query changed while loading another window",
        );
      }
      final assetsByLocation = {
        for (final asset in state.assets) asset.locationId: asset,
      };
      for (final asset in snapshot.assets) {
        assetsByLocation[asset.locationId] = asset;
      }
      state = state.copyWith(
        roots: snapshot.roots,
        assets: List.unmodifiable(assetsByLocation.values),
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        nextCursor: snapshot.nextCursor,
        isLoadingPage: false,
      );
    } on LibraryCatalogFailure catch (error) {
      if (_isDisposed || scanSequence != _scanSequence) {
        return;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await _reloadFirstCatalogPage(scanSequence);
        } on Object catch (refreshError) {
          if (!_isDisposed && scanSequence == _scanSequence) {
            state = state.copyWith(
              isLoadingPage: false,
              pageErrorMessage: refreshError.toString(),
            );
          }
        }
        return;
      }
      state = state.copyWith(
        isLoadingPage: false,
        pageErrorMessage: error.toString(),
      );
    } on Object catch (error) {
      if (_isDisposed || scanSequence != _scanSequence) {
        return;
      }
      state = state.copyWith(
        isLoadingPage: false,
        pageErrorMessage: error.toString(),
      );
    }
  }

  Future<bool> loadPreviousPage() async {
    final cursor = state.previousCursor;
    if (cursor == null ||
        state.isBusy ||
        state.isLoadingPage ||
        state.isLoadingPreviousPage ||
        state.isLoadingTimeAnchor) {
      return false;
    }

    final scanSequence = _scanSequence;
    state = state.copyWith(
      isLoadingPreviousPage: true,
      previousPageErrorMessage: null,
    );
    try {
      final snapshot = await ref
          .read(libraryCatalogProvider)
          .load(
            maxItems: libraryCatalogWindow,
            query: state.query,
            before: cursor,
          );
      if (_isDisposed || scanSequence != _scanSequence) {
        return false;
      }
      if (snapshot.revision != state.catalogRevision ||
          snapshot.queryId != state.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The gallery changed while loading the previous window",
        );
      }
      final precedingIds = {
        for (final asset in snapshot.assets) asset.locationId,
      };
      final existingIds = {for (final asset in state.assets) asset.locationId};
      final addedItemCount = snapshot.assets
          .where((asset) => !existingIds.contains(asset.locationId))
          .length;
      final mergedAssets = [
        ...snapshot.assets,
        for (final asset in state.assets)
          if (!precedingIds.contains(asset.locationId)) asset,
      ];
      state = state.copyWith(
        roots: snapshot.roots,
        assets: List.unmodifiable(mergedAssets),
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: (state.windowStartItemOffset - addedItemCount)
            .clamp(0, state.timeline?.totalItems ?? 0)
            .toInt(),
        previousCursor: snapshot.previousCursor,
        isLoadingPreviousPage: false,
      );
      return snapshot.assets.isNotEmpty;
    } on LibraryCatalogFailure catch (error) {
      if (_isDisposed || scanSequence != _scanSequence) {
        return false;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await _reloadFirstCatalogPage(scanSequence);
        } on Object catch (refreshError) {
          if (!_isDisposed && scanSequence == _scanSequence) {
            state = state.copyWith(
              isLoadingPreviousPage: false,
              previousPageErrorMessage: refreshError.toString(),
            );
          }
        }
        return false;
      }
      state = state.copyWith(
        isLoadingPreviousPage: false,
        previousPageErrorMessage: error.toString(),
      );
      return false;
    } on Object catch (error) {
      if (!_isDisposed && scanSequence == _scanSequence) {
        state = state.copyWith(
          isLoadingPreviousPage: false,
          previousPageErrorMessage: error.toString(),
        );
      }
      return false;
    }
  }

  Future<bool> jumpToTime(
    LibraryTimeBucket bucket, {
    int itemOffset = 0,
  }) async {
    final timeline = state.timeline;
    if (timeline == null ||
        state.isBusy ||
        state.isLoadingPage ||
        state.isLoadingPreviousPage ||
        state.isLoadingTimeAnchor) {
      return false;
    }
    final requestSequence = ++_scanSequence;
    final anchor = LibraryTimeAnchor(
      revision: timeline.revision,
      queryId: timeline.queryId,
      monthKey: bucket.monthKey,
      itemOffset: itemOffset.clamp(
        0,
        bucket.itemCount > 0 ? bucket.itemCount - 1 : 0,
      ),
    );
    state = state.copyWith(
      isLoadingTimeAnchor: true,
      timeNavigationErrorMessage: null,
    );
    try {
      final snapshot = await ref
          .read(libraryCatalogProvider)
          .loadAtTime(
            maxItems: libraryCatalogWindow,
            query: state.query,
            anchor: anchor,
          );
      if (_isDisposed || requestSequence != _scanSequence) {
        return false;
      }
      if (snapshot.revision != timeline.revision ||
          snapshot.queryId != timeline.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The catalog changed while navigating the timeline",
        );
      }
      _previewQueue?.clearPending();
      state = state.copyWith(
        roots: snapshot.roots,
        assets: snapshot.assets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: _globalItemOffsetForAnchor(
          timeline,
          bucket,
          anchor.itemOffset,
        ),
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
        activeTimeAnchor: anchor,
        isLoadingTimeAnchor: false,
        pageErrorMessage: null,
      );
      return true;
    } on LibraryCatalogFailure catch (error) {
      if (_isDisposed || requestSequence != _scanSequence) {
        return false;
      }
      if (error.code == "catalog_cursor_stale") {
        try {
          await _reloadFirstCatalogPage(requestSequence);
        } on Object catch (refreshError) {
          if (!_isDisposed && requestSequence == _scanSequence) {
            state = state.copyWith(
              isLoadingTimeAnchor: false,
              timeNavigationErrorMessage: refreshError.toString(),
            );
          }
        }
        return false;
      }
      state = state.copyWith(
        isLoadingTimeAnchor: false,
        timeNavigationErrorMessage: error.toString(),
      );
      return false;
    } on Object catch (error) {
      if (!_isDisposed && requestSequence == _scanSequence) {
        state = state.copyWith(
          isLoadingTimeAnchor: false,
          timeNavigationErrorMessage: error.toString(),
        );
      }
      return false;
    }
  }

  Future<bool> unregisterRoot(LibraryRoot root) async {
    if (state.isBusy) {
      return false;
    }
    final requestSequence = ++_scanSequence;
    state = state.copyWith(
      status: LibraryStatus.refreshing,
      errorMessage: null,
    );
    try {
      final removed = await ref
          .read(libraryCatalogProvider)
          .unregisterRoot(root.id);
      if (_isDisposed || requestSequence != _scanSequence) {
        return false;
      }
      if (!removed) {
        state = state.copyWith(
          status: state.roots.isEmpty
              ? LibraryStatus.empty
              : LibraryStatus.completed,
        );
        return false;
      }
      if (state.query.rootId == root.id) {
        state = state.copyWith(query: const LibraryGalleryQuery());
      }
      await _reloadFirstCatalogPage(requestSequence);
      if (_isDisposed || requestSequence != _scanSequence) {
        return false;
      }
      if (state.rootPath == root.path) {
        state = state.copyWith(rootPath: null, displayRootPath: null);
      }
      return true;
    } on Object catch (error) {
      if (!_isDisposed && requestSequence == _scanSequence) {
        state = state.copyWith(
          status: LibraryStatus.failed,
          errorMessage: error.toString(),
        );
      }
      return false;
    }
  }

  void requestPreview(LibraryAsset asset, {bool retry = false}) {
    _previews.request(asset, retry: retry);
  }

  void cancelPreview(String locationId) {
    _previewQueue?.cancel(locationId);
  }

  void _replaceAsset(LibraryAsset replacement) {
    final index = state.assets.indexWhere(
      (asset) => asset.locationId == replacement.locationId,
    );
    if (index < 0) {
      return;
    }
    final assets = [...state.assets];
    assets[index] = replacement;
    state = state.copyWith(assets: List.unmodifiable(assets));
  }

  void _handleUpdate(LibraryScanUpdate update) {
    final transition = _scanSession.apply(state, update);
    state = transition.state;
    if (transition.shouldReloadCatalog) {
      unawaited(_reloadPublishedCatalog(_scanSequence));
    }
  }

  void _handleError(Object error, StackTrace stackTrace) {
    state = _scanSession.fail(state, error);
  }

  void _handleDone() {
    state = _scanSession.finish(state);
  }

  Future<void> _reloadPublishedCatalog(int scanSequence) async {
    try {
      await _reloadFirstCatalogPage(scanSequence);
    } on Object catch (error) {
      if (_isDisposed || scanSequence != _scanSequence) {
        return;
      }
      state = state.copyWith(
        status: LibraryStatus.failed,
        errorMessage: error.toString(),
      );
    }
  }

  Future<void> _resumeInterruptedScanIfAvailable() async {
    try {
      final recoverable = await ref
          .read(libraryScannerProvider)
          .loadRecoverableScan();
      if (_isDisposed || state.isBusy) {
        return;
      }
      if (recoverable != null) {
        await _resumeScan(recoverable);
        return;
      }
      final paused = await ref.read(libraryScannerProvider).loadPausedScan();
      if (_isDisposed || paused == null || state.isBusy) {
        return;
      }
      _scanSession.restorePaused(paused);
      state = state.copyWith(
        status: LibraryStatus.paused,
        scanId: paused.scanId,
        rootPath: paused.rootPath,
        displayRootPath: paused.displayRootPath,
        visitedEntries: paused.visitedEntries,
        stagedAssetCount: paused.acceptedItems,
        issueCount: paused.issueCount,
        itemLimit: paused.itemLimit,
        entryLimit: paused.entryLimit,
        isResumingScan: false,
      );
    } on Object catch (error) {
      if (_isDisposed || state.isBusy) {
        return;
      }
      state = state.copyWith(
        status: LibraryStatus.failed,
        errorMessage: error.toString(),
      );
    }
  }

  Future<void> _resumeScan(RecoverableLibraryScan scan) {
    _scanSequence += 1;
    return _startScan(
      scanId: scan.scanId,
      rootPath: scan.rootPath,
      displayRootPath: scan.displayRootPath,
      itemLimit: scan.itemLimit,
      entryLimit: scan.entryLimit,
      previewEdge: scan.previewEdge,
      visitedEntries: scan.visitedEntries,
      acceptedItems: scan.acceptedItems,
      issueCount: scan.issueCount,
      isResuming: true,
    );
  }

  static int _globalItemOffsetForAnchor(
    LibraryTimeline timeline,
    LibraryTimeBucket selectedBucket,
    int itemOffset,
  ) {
    var precedingItems = 0;
    for (final bucket in timeline.buckets) {
      if (identical(bucket, selectedBucket) ||
          bucket.monthKey == selectedBucket.monthKey) {
        return (precedingItems + itemOffset)
            .clamp(0, timeline.totalItems)
            .toInt();
      }
      precedingItems += bucket.itemCount;
    }
    return 0;
  }

  Future<void> _reloadFirstCatalogPage(int scanSequence) async {
    final catalog = ref.read(libraryCatalogProvider);
    final query = state.query;
    var snapshot = await catalog.load(
      maxItems: libraryCatalogWindow,
      query: query,
    );
    final timeline = await catalog.loadTimeline(query);
    if (snapshot.revision != timeline.revision ||
        snapshot.queryId != timeline.queryId) {
      snapshot = await catalog.load(
        maxItems: libraryCatalogWindow,
        query: query,
      );
      if (snapshot.revision != timeline.revision ||
          snapshot.queryId != timeline.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_timeline_stale",
          message: "The catalog changed while its timeline was loading",
        );
      }
    }
    if (_isDisposed || scanSequence != _scanSequence) {
      return;
    }
    final rootPath = state.rootPath;
    final displayRootPath = state.displayRootPath;
    final recentIssues = state.recentIssues;
    final isScanLimited = state.isScanLimited;
    state = LibraryState.fromSnapshot(snapshot, query: query).copyWith(
      rootPath: rootPath,
      displayRootPath: displayRootPath,
      recentIssues: recentIssues,
      isScanLimited: isScanLimited,
      timeline: timeline,
      activeTimeAnchor: null,
      isLoadingTimeline: false,
      isLoadingTimeAnchor: false,
      timeNavigationErrorMessage: null,
    );
  }

  Future<void> _loadInitialTimeline() async {
    if (state.roots.isEmpty || state.timeline != null) {
      return;
    }
    final requestSequence = _scanSequence;
    state = state.copyWith(
      isLoadingTimeline: true,
      timeNavigationErrorMessage: null,
    );
    try {
      final timeline = await ref
          .read(libraryCatalogProvider)
          .loadTimeline(state.query);
      if (_isDisposed || requestSequence != _scanSequence) {
        return;
      }
      if (timeline.revision != state.catalogRevision ||
          timeline.queryId != state.queryId) {
        await _reloadFirstCatalogPage(requestSequence);
        return;
      }
      state = state.copyWith(timeline: timeline, isLoadingTimeline: false);
    } on Object catch (error) {
      if (!_isDisposed && requestSequence == _scanSequence) {
        state = state.copyWith(
          isLoadingTimeline: false,
          timeNavigationErrorMessage: error.toString(),
        );
      }
    }
  }
}

final initialLibraryStateProvider = Provider<LibraryState>((ref) {
  return const LibraryState();
});

final libraryControllerProvider =
    NotifierProvider<LibraryController, LibraryState>(LibraryController.new);
