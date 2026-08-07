import "dart:async";
import "dart:collection";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../adapters/directory_picker.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_previewer.dart";
import "library_scanner.dart";

const _previewEdge = 512;
const _recentIssueLimit = 20;
const _maxActivePreviews = 2;

class LibraryController extends Notifier<LibraryState> {
  StreamSubscription<LibraryScanUpdate>? _subscription;
  int _scanSequence = 0;
  String? _activeScanId;
  RecoverableLibraryScan? _activeScan;
  RecoverableLibraryScan? _pausedScan;
  final Queue<LibraryAsset> _previewQueue = Queue();
  final Set<String> _queuedPreviewIds = {};
  final Set<String> _activePreviewIds = {};
  bool _isDisposed = false;

  @override
  LibraryState build() {
    final scanner = ref.read(libraryScannerProvider);
    ref.onDispose(() {
      _isDisposed = true;
      final scanId = _activeScanId;
      if (scanId != null) {
        scanner.cancel(scanId);
      }
      _previewQueue.clear();
      _queuedPreviewIds.clear();
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
    required int? itemLimit,
    required int? entryLimit,
    required int previewEdge,
    int visitedEntries = 0,
    int acceptedItems = 0,
    int issueCount = 0,
    bool isResuming = false,
  }) async {
    await _subscription?.cancel();
    _activeScanId = scanId;
    _pausedScan = null;
    _activeScan = RecoverableLibraryScan(
      scanId: scanId,
      rootPath: rootPath,
      itemLimit: itemLimit,
      entryLimit: entryLimit,
      previewEdge: previewEdge,
      visitedEntries: visitedEntries,
      acceptedItems: acceptedItems,
      issueCount: issueCount,
    );

    state = state.copyWith(
      status: LibraryStatus.scanning,
      scanId: scanId,
      rootPath: rootPath,
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
    final pausedScan = _pausedScan;
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
    _previewQueue.clear();
    _queuedPreviewIds.clear();
    state = state.copyWith(
      status: LibraryStatus.refreshing,
      query: normalized,
      queryId: "",
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
      _previewQueue.clear();
      _queuedPreviewIds.clear();
      state = state.copyWith(
        roots: snapshot.roots,
        assets: snapshot.assets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
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
        state = state.copyWith(rootPath: null);
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
    if (_isDisposed || asset.previewStatus == LibraryPreviewStatus.ready) {
      return;
    }
    if (asset.previewStatus == LibraryPreviewStatus.failed && !retry) {
      return;
    }
    if (_queuedPreviewIds.contains(asset.locationId) ||
        _activePreviewIds.contains(asset.locationId)) {
      return;
    }
    _previewQueue.addLast(asset);
    _queuedPreviewIds.add(asset.locationId);
    _drainPreviewQueue();
  }

  void cancelPreview(String locationId) {
    if (!_queuedPreviewIds.remove(locationId)) {
      return;
    }
    _previewQueue.removeWhere((asset) => asset.locationId == locationId);
  }

  void _drainPreviewQueue() {
    while (!_isDisposed &&
        _activePreviewIds.length < _maxActivePreviews &&
        _previewQueue.isNotEmpty) {
      final asset = _previewQueue.removeFirst();
      _queuedPreviewIds.remove(asset.locationId);
      _activePreviewIds.add(asset.locationId);
      unawaited(_loadPreview(asset));
    }
  }

  Future<void> _loadPreview(LibraryAsset asset) async {
    try {
      final previewed = await ref
          .read(libraryPreviewerProvider)
          .materialize(locationId: asset.locationId, previewEdge: _previewEdge);
      if (!_isDisposed) {
        _replaceAsset(previewed);
      }
    } on Object catch (error) {
      if (!_isDisposed) {
        _replaceAsset(
          asset.withPreview(
            previewPath: asset.previewPath,
            width: asset.width,
            height: asset.height,
            previewStatus: LibraryPreviewStatus.failed,
            previewIssueCode: "preview_request_failed",
            previewIssueMessage: error.toString(),
          ),
        );
      }
    } finally {
      _activePreviewIds.remove(asset.locationId);
      _drainPreviewQueue();
    }
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
    switch (update) {
      case LibraryScanStarted(
        :final scanId,
        :final rootPath,
        :final itemLimit,
        :final entryLimit,
      ):
        _activeScanId = scanId;
        state = state.copyWith(
          status: LibraryStatus.scanning,
          scanId: scanId,
          rootPath: rootPath,
          itemLimit: itemLimit,
          entryLimit: entryLimit,
        );
      case LibraryScanProgress(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ):
        _updateActiveProgress(
          visitedEntries: visitedEntries,
          acceptedItems: acceptedItems,
          issueCount: issueCount,
        );
        state = state.copyWith(
          visitedEntries: visitedEntries,
          stagedAssetCount: acceptedItems,
          issueCount: issueCount,
        );
      case LibraryAssetDiscovered():
        state = state.copyWith(
          stagedAssetCount: state.stagedAssetCount + 1,
          visitedEntries: state.visitedEntries + 1,
        );
      case LibraryIssueDiscovered(:final issue):
        final issues = [...state.recentIssues, issue];
        state = state.copyWith(
          issueCount: state.issueCount + 1,
          recentIssues: List.unmodifiable(
            issues.length > _recentIssueLimit
                ? issues.sublist(issues.length - _recentIssueLimit)
                : issues,
          ),
        );
      case LibraryScanCompleted(
        :final issueCount,
        :final catalogPath,
        :final wasLimited,
      ):
        _activeScanId = null;
        _activeScan = null;
        state = state.copyWith(
          status: LibraryStatus.refreshing,
          issueCount: issueCount,
          catalogPath: catalogPath,
          isScanLimited: wasLimited,
          isResumingScan: false,
        );
        unawaited(_reloadPublishedCatalog(_scanSequence));
      case LibraryScanCancelled(:final issueCount):
        _activeScanId = null;
        _activeScan = null;
        state = state.copyWith(
          status: LibraryStatus.cancelled,
          issueCount: issueCount,
          isResumingScan: false,
        );
      case LibraryScanPaused(
        :final visitedEntries,
        :final acceptedItems,
        :final issueCount,
      ):
        final activeScan = _activeScan;
        _activeScanId = null;
        _activeScan = null;
        if (activeScan != null) {
          _pausedScan = RecoverableLibraryScan(
            scanId: activeScan.scanId,
            rootPath: activeScan.rootPath,
            itemLimit: activeScan.itemLimit,
            entryLimit: activeScan.entryLimit,
            previewEdge: activeScan.previewEdge,
            visitedEntries: visitedEntries,
            acceptedItems: acceptedItems,
            issueCount: issueCount,
          );
        }
        state = state.copyWith(
          status: LibraryStatus.paused,
          visitedEntries: visitedEntries,
          stagedAssetCount: acceptedItems,
          issueCount: issueCount,
          isResumingScan: false,
        );
      case LibraryScanStale(:final issueCount):
        _activeScanId = null;
        _activeScan = null;
        state = state.copyWith(
          status: LibraryStatus.stale,
          issueCount: issueCount,
          isResumingScan: false,
        );
    }
  }

  void _handleError(Object error, StackTrace stackTrace) {
    _activeScanId = null;
    _activeScan = null;
    state = state.copyWith(
      status: LibraryStatus.failed,
      isResumingScan: false,
      errorMessage: error.toString(),
    );
  }

  void _handleDone() {
    if (state.status == LibraryStatus.scanning ||
        state.status == LibraryStatus.pausing ||
        state.status == LibraryStatus.cancelling) {
      _activeScanId = null;
      _activeScan = null;
      state = state.copyWith(
        status: LibraryStatus.failed,
        isResumingScan: false,
        errorMessage: "The scan ended without a completion event",
      );
    }
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
      _pausedScan = paused;
      state = state.copyWith(
        status: LibraryStatus.paused,
        scanId: paused.scanId,
        rootPath: paused.rootPath,
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
      itemLimit: scan.itemLimit,
      entryLimit: scan.entryLimit,
      previewEdge: scan.previewEdge,
      visitedEntries: scan.visitedEntries,
      acceptedItems: scan.acceptedItems,
      issueCount: scan.issueCount,
      isResuming: true,
    );
  }

  void _updateActiveProgress({
    required int visitedEntries,
    required int acceptedItems,
    required int issueCount,
  }) {
    final activeScan = _activeScan;
    if (activeScan == null) {
      return;
    }
    _activeScan = RecoverableLibraryScan(
      scanId: activeScan.scanId,
      rootPath: activeScan.rootPath,
      itemLimit: activeScan.itemLimit,
      entryLimit: activeScan.entryLimit,
      previewEdge: activeScan.previewEdge,
      visitedEntries: visitedEntries,
      acceptedItems: acceptedItems,
      issueCount: issueCount,
    );
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
    final recentIssues = state.recentIssues;
    final isScanLimited = state.isScanLimited;
    state = LibraryState.fromSnapshot(snapshot, query: query).copyWith(
      rootPath: rootPath,
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
