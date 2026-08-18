import "dart:async";

import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../settings/application/ame_preferences.dart";
import "../adapters/directory_picker.dart";
import "../domain/gallery_layout_manifest.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "library_catalog.dart";
import "library_preview_coordinator.dart";
import "library_preview_queue.dart";
import "library_preview_store.dart";
import "library_previewer.dart";
import "library_scan_session.dart";
import "library_scanner.dart";

const _previewEdge = 512;
const _timeNavigationRetryDelay = Duration(milliseconds: 120);
const _maxVisibleRangePageLoads = 2;
const _retainedDetailHighWatermark = 5000;
const _retainedDetailLowWatermark = 3500;

class _RetainedCatalogPage {
  const _RetainedCatalogPage({
    required this.assets,
    required this.startItemOffset,
    required this.previousCursor,
    required this.nextCursor,
  });

  final List<LibraryAsset> assets;
  final int startItemOffset;
  final LibraryCatalogCursor? previousCursor;
  final LibraryCatalogCursor? nextCursor;
}

class _PendingTimeNavigation {
  _PendingTimeNavigation({
    required this.generation,
    required this.query,
    required this.timeline,
    required this.anchor,
    required this.globalItemOffset,
  });

  final int generation;
  final LibraryGalleryQuery query;
  final LibraryTimeline timeline;
  final LibraryTimeAnchor anchor;
  final int globalItemOffset;
  final Completer<bool> completion = Completer<bool>();
}

class _ActiveScanRun {
  _ActiveScanRun({required this.scanId, required this.generation});

  final String scanId;
  final int generation;
  final Completer<void> streamDone = Completer<void>();
  bool didStart = false;
  bool didReceiveTerminal = false;
}

class LibraryController extends Notifier<LibraryState> {
  StreamSubscription<LibraryScanUpdate>? _subscription;
  _ActiveScanRun? _activeScanRun;
  int _scanRunGeneration = 0;
  Future<void> _scanStartQueue = Future<void>.value();
  int _scanSequence = 0;
  final LibraryScanSession _scanSession = LibraryScanSession();
  final StreamController<LibraryGalleryLayoutDimensionUpdate>
  _layoutDimensionUpdates = StreamController.broadcast(sync: true);
  LibraryPreviewCoordinator? _previewCoordinator;
  _PendingTimeNavigation? _pendingTimeNavigation;
  _PendingTimeNavigation? _activeTimeNavigation;
  _PendingTimeNavigation? _timeNavigationOwner;
  Timer? _timeNavigationRetryTimer;
  bool _isRunningTimeNavigation = false;
  int _timeNavigationGeneration = 0;
  int? _loadingTimeNavigationGeneration;
  ({int start, int end})? _pendingVisibleRange;
  bool _isEnsuringVisibleRange = false;
  bool _isVisibleRangeDrainScheduled = false;
  final List<_RetainedCatalogPage> _retainedCatalogPages = [];
  LibraryState? _queryTransitionBaseState;
  int? _queryTransitionRequestSequence;
  bool _isDisposed = false;

  LibraryPreviewCoordinator get _previews =>
      _previewCoordinator ??= LibraryPreviewCoordinator(
        previewer: ref.read(libraryPreviewerProvider),
        defaultPreviewEdge: _previewEdge,
        maxActive: _maxActivePreviewsFor(
          ref.read(amePreferencesControllerProvider).previewLoadingSpeed,
        ),
        canPublish: _canPublishPreview,
        onPublished: _handlePreviewPublished,
      );

  @override
  LibraryState build() {
    ref.listen(
      amePreferencesControllerProvider.select(
        (preferences) => preferences.previewLoadingSpeed,
      ),
      (_, speed) =>
          _previewCoordinator?.updateMaxActive(_maxActivePreviewsFor(speed)),
    );
    final scanner = ref.read(libraryScannerProvider);
    ref.onDispose(() {
      _isDisposed = true;
      final activeScanRun = _activeScanRun;
      _activeScanRun = null;
      if (activeScanRun != null && !activeScanRun.streamDone.isCompleted) {
        activeScanRun.streamDone.complete();
      }
      final scanId = _scanSession.activeScanId;
      if (scanId != null) {
        scanner.cancel(scanId);
      }
      _previewCoordinator?.dispose();
      unawaited(_layoutDimensionUpdates.close());
      _timeNavigationRetryTimer?.cancel();
      _pendingVisibleRange = null;
      final pendingTimeNavigation = _pendingTimeNavigation;
      _pendingTimeNavigation = null;
      if (pendingTimeNavigation != null &&
          !pendingTimeNavigation.completion.isCompleted) {
        pendingTimeNavigation.completion.complete(false);
      }
      final activeTimeNavigation = _activeTimeNavigation;
      _activeTimeNavigation = null;
      _timeNavigationOwner = null;
      if (activeTimeNavigation != null &&
          !activeTimeNavigation.completion.isCompleted) {
        activeTimeNavigation.completion.complete(false);
      }
      unawaited(_subscription?.cancel());
    });
    final initialState = ref.watch(initialLibraryStateProvider);
    _resetRetainedCatalogPages(
      assets: initialState.assets,
      startItemOffset: initialState.windowStartItemOffset,
      previousCursor: initialState.previousCursor,
      nextCursor: initialState.nextCursor,
    );
    Future<void>.microtask(_resumeInterruptedScanIfAvailable);
    Future<void>.microtask(_loadInitialTimeline);
    return initialState;
  }

  Future<void> chooseDirectoryAndScan() async {
    if (state.isBusy) {
      return;
    }

    state = state.copyWith(
      status: LibraryStatus.choosingDirectory,
      scanId: null,
      rootPath: null,
      displayRootPath: null,
      visitedEntries: 0,
      stagedAssetCount: 0,
      scanPhase: LibraryScanPhase.discovering,
      validatedAssetCount: 0,
      validationAssetCount: 0,
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
    await _enqueueScanStart(
      allowedBusyStatus: LibraryStatus.choosingDirectory,
      start: () async {
        _advanceSequenceOutsideQueryTransition();
        final scanId =
            "ame-${DateTime.now().microsecondsSinceEpoch}-$_scanSequence";
        await _startScan(
          scanId: scanId,
          rootPath: rootPath,
          itemLimit: null,
          entryLimit: null,
          previewEdge: _previewEdge,
        );
      },
    );
  }

  Future<void> _enqueueScanStart({
    required LibraryStatus? allowedBusyStatus,
    required Future<void> Function() start,
  }) {
    final previous = _scanStartQueue;
    final completion = Completer<void>();
    _scanStartQueue = completion.future;
    return () async {
      await previous;
      try {
        final terminalRun = _activeScanRun;
        if (terminalRun != null && terminalRun.didReceiveTerminal) {
          await terminalRun.streamDone.future;
        }
        final hasConflictingScan = _activeScanRun != null || state.isScanning;
        final hasProtectedPausedScan =
            state.status == LibraryStatus.paused &&
            allowedBusyStatus != LibraryStatus.paused;
        final hasDifferentPicker =
            state.status == LibraryStatus.choosingDirectory &&
            allowedBusyStatus != LibraryStatus.choosingDirectory;
        if (_isDisposed ||
            hasConflictingScan ||
            hasProtectedPausedScan ||
            hasDifferentPicker) {
          return;
        }
        await start();
      } finally {
        completion.complete();
      }
    }();
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
    if (_activeScanRun != null) {
      throw StateError("Cannot replace an active scan run");
    }
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
      scanPhase: LibraryScanPhase.discovering,
      validatedAssetCount: 0,
      validationAssetCount: 0,
      issueCount: issueCount,
      itemLimit: itemLimit,
      entryLimit: entryLimit,
      isScanLimited: false,
      isResumingScan: isResuming,
      isLoadingPage: false,
      pageErrorMessage: null,
      errorMessage: null,
    );

    final run = _ActiveScanRun(
      scanId: scanId,
      generation: ++_scanRunGeneration,
    );
    _activeScanRun = run;
    try {
      final stream = ref
          .read(libraryScannerProvider)
          .scan(
            scanId: scanId,
            rootPath: rootPath,
            itemLimit: itemLimit,
            entryLimit: entryLimit,
            previewEdge: previewEdge,
          );
      _subscription = stream.listen(
        (update) => _handleUpdate(run, update),
        onError: (Object error, StackTrace stackTrace) {
          _handleError(run, error, stackTrace);
        },
        onDone: () => _handleDone(run),
        cancelOnError: true,
      );
    } on Object catch (error) {
      _releaseScanRun(run);
      state = _scanSession.fail(state, error);
      rethrow;
    }
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

  void dismissCompletedImport() {
    if (state.status != LibraryStatus.completed || state.scanId == null) {
      return;
    }
    state = state.copyWith(
      scanId: null,
      rootPath: null,
      displayRootPath: null,
      visitedEntries: 0,
      stagedAssetCount: 0,
      scanPhase: LibraryScanPhase.discovering,
      validatedAssetCount: 0,
      validationAssetCount: 0,
      itemLimit: null,
      entryLimit: null,
      isScanLimited: false,
    );
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

  Future<bool> updateQuery(
    LibraryGalleryQuery query, {
    String? anchorLocationId,
    String? anchorAssetId,
    int? fallbackGlobalItemIndex,
    bool forceRefresh = false,
    BigInt? minimumCatalogRevision,
    bool showRefreshingStatus = true,
  }) async {
    final normalized = query.copyWith(
      folderRelativePath: query.folderRelativePath
          ?.replaceAll("\\", "/")
          .replaceAll(RegExp(r"^/+|/+$"), ""),
      searchText: query.searchText.trim(),
    );
    final doesQueryTransitionOwnSequence =
        _queryTransitionBaseState != null &&
        _queryTransitionRequestSequence == _scanSequence;
    if (_queryTransitionBaseState != null && !doesQueryTransitionOwnSequence) {
      _queryTransitionBaseState = null;
      _queryTransitionRequestSequence = null;
    }
    if (!forceRefresh &&
        normalized == state.query &&
        _queryTransitionBaseState == null) {
      return true;
    }
    if (!forceRefresh &&
        normalized == state.query &&
        _queryTransitionBaseState != null) {
      _scanSequence += 1;
      final baseState = _queryTransitionBaseState!;
      _queryTransitionBaseState = null;
      _queryTransitionRequestSequence = null;
      state = baseState;
      return true;
    }
    if (state.status == LibraryStatus.choosingDirectory ||
        state.isScanning ||
        state.status == LibraryStatus.paused ||
        state.isLoadingTimeAnchor) {
      return false;
    }
    final priorSequence = _scanSequence;
    final isContinuingQueryTransition =
        _queryTransitionBaseState != null &&
        _queryTransitionRequestSequence == priorSequence;
    final requestSequence = ++_scanSequence;
    if (!isContinuingQueryTransition) {
      _queryTransitionBaseState = state;
    }
    _queryTransitionRequestSequence = requestSequence;
    state = state.copyWith(
      status: showRefreshingStatus ? LibraryStatus.refreshing : state.status,
      isLoadingTimeline: true,
      pageErrorMessage: null,
      previousPageErrorMessage: null,
      timeNavigationErrorMessage: null,
      errorMessage: null,
    );
    try {
      final catalog = ref.read(libraryCatalogProvider);
      final stableAnchorCatalog = catalog is LibraryStableQueryAnchorCatalog
          ? catalog as LibraryStableQueryAnchorCatalog
          : null;
      final anchorCatalog = catalog is LibraryQueryAnchorCatalog
          ? catalog as LibraryQueryAnchorCatalog
          : null;
      Future<LibrarySnapshot> loadSnapshot() {
        if (anchorLocationId != null &&
            anchorAssetId != null &&
            stableAnchorCatalog != null) {
          return stableAnchorCatalog.loadAroundAsset(
            maxItems: libraryCatalogWindow,
            query: normalized,
            requestedLocationId: anchorLocationId,
            anchorAssetId: anchorAssetId,
            fallbackGlobalItemIndex: fallbackGlobalItemIndex ?? 0,
          );
        }
        if (anchorLocationId != null && anchorCatalog != null) {
          return anchorCatalog.loadAroundLocation(
            maxItems: libraryCatalogWindow,
            query: normalized,
            anchorLocationId: anchorLocationId,
          );
        }
        return catalog.load(maxItems: libraryCatalogWindow, query: normalized);
      }

      var snapshot = await loadSnapshot();
      final timeline = await catalog.loadTimeline(normalized);
      if (minimumCatalogRevision != null &&
          snapshot.revision < minimumCatalogRevision) {
        snapshot = await loadSnapshot();
      }
      if (snapshot.revision != timeline.revision ||
          snapshot.queryId != timeline.queryId) {
        snapshot = await loadSnapshot();
        if (snapshot.revision != timeline.revision ||
            snapshot.queryId != timeline.queryId) {
          throw const LibraryCatalogFailure(
            code: "catalog_timeline_stale",
            message: "The catalog changed while its timeline was loading",
          );
        }
      }
      if (minimumCatalogRevision != null &&
          snapshot.revision < minimumCatalogRevision) {
        throw const LibraryCatalogFailure(
          code: "catalog_revision_stale",
          message: "The catalog revision has not reached the requested refresh",
        );
      }
      if (_isDisposed || requestSequence != _scanSequence) {
        return false;
      }
      final baseState = _queryTransitionBaseState ?? state;
      final windowStart =
          snapshot.queryAnchorResolution?.windowStartItemOffset ?? 0;
      _invalidatePreviewContext();
      _resetRetainedCatalogPages(
        assets: snapshot.assets,
        startItemOffset: windowStart,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
      );
      state = LibraryState.fromSnapshot(snapshot, query: normalized).copyWith(
        rootPath: baseState.rootPath,
        displayRootPath: baseState.displayRootPath,
        scanId: baseState.scanId,
        visitedEntries: baseState.visitedEntries,
        stagedAssetCount: baseState.stagedAssetCount,
        scanPhase: baseState.scanPhase,
        validatedAssetCount: baseState.validatedAssetCount,
        validationAssetCount: baseState.validationAssetCount,
        itemLimit: baseState.itemLimit,
        entryLimit: baseState.entryLimit,
        recentIssues: baseState.recentIssues,
        isScanLimited: baseState.isScanLimited,
        windowStartItemOffset: windowStart,
        timeline: timeline,
        activeTimeAnchor: null,
        isLoadingTimeline: false,
        isLoadingTimeAnchor: false,
        pageErrorMessage: null,
        previousPageErrorMessage: null,
        timeNavigationErrorMessage: null,
        errorMessage: null,
      );
      _queryTransitionBaseState = null;
      _queryTransitionRequestSequence = null;
      return true;
    } on Object catch (error) {
      if (!_isDisposed && requestSequence == _scanSequence) {
        final baseState = _queryTransitionBaseState ?? state;
        _queryTransitionBaseState = null;
        _queryTransitionRequestSequence = null;
        state = baseState.copyWith(
          isLoadingTimeline: false,
          errorMessage: showRefreshingStatus
              ? error.toString()
              : baseState.errorMessage,
        );
      }
      return false;
    }
  }

  Future<bool> refreshFromSynchronization({
    required BigInt catalogRevision,
    String? anchorLocationId,
    String? anchorAssetId,
    int? fallbackGlobalItemIndex,
  }) {
    return updateQuery(
      state.query,
      anchorLocationId: anchorLocationId,
      anchorAssetId: anchorAssetId,
      fallbackGlobalItemIndex: fallbackGlobalItemIndex,
      forceRefresh: true,
      minimumCatalogRevision: catalogRevision,
      showRefreshingStatus: false,
    );
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
      _ensureRetainedCatalogPagesCurrent();
      final existingIds = {for (final asset in state.assets) asset.locationId};
      _mergeRetainedCatalogAssetUpdates(snapshot.assets);
      final addedAssets = [
        for (final asset in snapshot.assets)
          if (!existingIds.contains(asset.locationId)) asset,
      ];
      if (addedAssets.isNotEmpty) {
        _retainedCatalogPages.add(
          _RetainedCatalogPage(
            assets: List.unmodifiable(addedAssets),
            startItemOffset: state.windowStartItemOffset + state.assets.length,
            previousCursor: snapshot.previousCursor,
            nextCursor: snapshot.nextCursor,
          ),
        );
      } else if (_retainedCatalogPages.isNotEmpty) {
        final lastIndex = _retainedCatalogPages.length - 1;
        final lastPage = _retainedCatalogPages[lastIndex];
        _retainedCatalogPages[lastIndex] = _RetainedCatalogPage(
          assets: lastPage.assets,
          startItemOffset: lastPage.startItemOffset,
          previousCursor: lastPage.previousCursor,
          nextCursor: snapshot.nextCursor,
        );
      }
      _trimRetainedCatalogPages(trimLeading: true);
      final retainedAssets = _retainedCatalogAssets();
      final firstPage = _retainedCatalogPages.first;
      state = state.copyWith(
        roots: snapshot.roots,
        assets: retainedAssets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: firstPage.startItemOffset,
        previousCursor: firstPage.previousCursor,
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
      _ensureRetainedCatalogPagesCurrent();
      final existingIds = {for (final asset in state.assets) asset.locationId};
      _mergeRetainedCatalogAssetUpdates(snapshot.assets);
      final addedAssets = [
        for (final asset in snapshot.assets)
          if (!existingIds.contains(asset.locationId)) asset,
      ];
      final addedItemCount = addedAssets.length;
      if (addedAssets.isNotEmpty) {
        _retainedCatalogPages.insert(
          0,
          _RetainedCatalogPage(
            assets: List.unmodifiable(addedAssets),
            startItemOffset: (state.windowStartItemOffset - addedItemCount)
                .clamp(0, state.timeline?.totalItems ?? 0)
                .toInt(),
            previousCursor: snapshot.previousCursor,
            nextCursor: snapshot.nextCursor,
          ),
        );
      } else if (_retainedCatalogPages.isNotEmpty) {
        final firstPage = _retainedCatalogPages.first;
        _retainedCatalogPages[0] = _RetainedCatalogPage(
          assets: firstPage.assets,
          startItemOffset: firstPage.startItemOffset,
          previousCursor: snapshot.previousCursor,
          nextCursor: firstPage.nextCursor,
        );
      }
      _trimRetainedCatalogPages(trimLeading: false);
      final retainedAssets = _retainedCatalogAssets();
      final firstPage = _retainedCatalogPages.first;
      final lastPage = _retainedCatalogPages.last;
      state = state.copyWith(
        roots: snapshot.roots,
        assets: retainedAssets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: firstPage.startItemOffset,
        previousCursor: firstPage.previousCursor,
        nextCursor: lastPage.nextCursor,
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

  Future<bool> jumpToTime(LibraryTimeBucket bucket, {int itemOffset = 0}) {
    return _requestTimeNavigation(
      bucket,
      itemOffset: itemOffset,
      ownsVisibleRange: true,
    );
  }

  Future<bool> prefetchTime(LibraryTimeBucket bucket, {int itemOffset = 0}) {
    return _requestTimeNavigation(
      bucket,
      itemOffset: itemOffset,
      ownsVisibleRange: false,
    );
  }

  Future<bool> _requestTimeNavigation(
    LibraryTimeBucket bucket, {
    required int itemOffset,
    required bool ownsVisibleRange,
  }) {
    final timeline = state.timeline;
    if (timeline == null) {
      return Future.value(false);
    }
    if (!ownsVisibleRange && _hasCompatibleTimeNavigationOwner()) {
      return Future.value(false);
    }
    if (ownsVisibleRange) {
      _pendingVisibleRange = null;
    }
    final anchor = LibraryTimeAnchor(
      revision: timeline.revision,
      queryId: timeline.queryId,
      monthKey: bucket.monthKey,
      itemOffset: itemOffset.clamp(
        0,
        bucket.itemCount > 0 ? bucket.itemCount - 1 : 0,
      ),
    );
    final globalItemOffset = _globalItemOffsetForAnchor(
      timeline,
      bucket,
      anchor.itemOffset,
    );
    final pending = _pendingTimeNavigation;
    if (pending != null &&
        _matchesTimeNavigationTarget(
          pending,
          timeline,
          state.query,
          globalItemOffset,
        )) {
      if (ownsVisibleRange) {
        _timeNavigationOwner = pending;
      }
      return pending.completion.future;
    }
    final active = _activeTimeNavigation;
    if (active != null &&
        active.generation == _timeNavigationGeneration &&
        _matchesTimeNavigationTarget(
          active,
          timeline,
          state.query,
          globalItemOffset,
        )) {
      if (ownsVisibleRange) {
        _timeNavigationOwner = active;
      }
      return active.completion.future;
    }
    final generation = ++_timeNavigationGeneration;
    final request = _PendingTimeNavigation(
      generation: generation,
      query: state.query,
      timeline: timeline,
      anchor: anchor,
      globalItemOffset: globalItemOffset,
    );
    final previousPending = _pendingTimeNavigation;
    _pendingTimeNavigation = request;
    if (ownsVisibleRange) {
      _timeNavigationOwner = request;
    }
    if (previousPending != null && !previousPending.completion.isCompleted) {
      previousPending.completion.complete(false);
    }
    _scheduleTimeNavigationDrain();
    return request.completion.future;
  }

  bool _matchesTimeNavigationTarget(
    _PendingTimeNavigation request,
    LibraryTimeline timeline,
    LibraryGalleryQuery query,
    int globalItemOffset,
  ) {
    return request.query == query &&
        request.timeline.revision == timeline.revision &&
        request.timeline.queryId == timeline.queryId &&
        request.globalItemOffset == globalItemOffset;
  }

  void _scheduleTimeNavigationDrain() {
    if (_isDisposed || _isRunningTimeNavigation) {
      return;
    }
    _timeNavigationRetryTimer?.cancel();
    _timeNavigationRetryTimer = null;
    unawaited(Future<void>.microtask(_drainTimeNavigation));
  }

  Future<void> _drainTimeNavigation() async {
    if (_isDisposed || _isRunningTimeNavigation) {
      return;
    }
    final request = _pendingTimeNavigation;
    if (request == null) {
      return;
    }
    if (!_isCompatibleTimeNavigation(request)) {
      _pendingTimeNavigation = null;
      if (identical(_timeNavigationOwner, request)) {
        _timeNavigationOwner = null;
      }
      if (!request.completion.isCompleted) {
        request.completion.complete(false);
      }
      _scheduleTimeNavigationDrain();
      return;
    }
    if (_isTimeNavigationBlocked) {
      _timeNavigationRetryTimer ??= Timer(
        _timeNavigationRetryDelay,
        _scheduleTimeNavigationDrain,
      );
      return;
    }

    _pendingTimeNavigation = null;
    _isRunningTimeNavigation = true;
    _activeTimeNavigation = request;
    try {
      final didLoad = await _loadTimeNavigation(request);
      final isLatest = request.generation == _timeNavigationGeneration;
      if (!didLoad && identical(_timeNavigationOwner, request)) {
        _timeNavigationOwner = null;
      }
      if (!request.completion.isCompleted) {
        request.completion.complete(didLoad && isLatest);
      }
    } finally {
      if (identical(_activeTimeNavigation, request)) {
        _activeTimeNavigation = null;
      }
      _isRunningTimeNavigation = false;
      _scheduleTimeNavigationDrain();
    }
  }

  bool get _isTimeNavigationBlocked =>
      state.isProcessing ||
      state.status == LibraryStatus.paused ||
      state.isLoadingPage ||
      state.isLoadingPreviousPage ||
      state.isLoadingTimeAnchor;

  bool _isCompatibleTimeNavigation(_PendingTimeNavigation request) {
    final timeline = state.timeline;
    return timeline != null &&
        timeline.revision == request.timeline.revision &&
        timeline.queryId == request.timeline.queryId &&
        state.query == request.query;
  }

  Future<bool> _loadTimeNavigation(_PendingTimeNavigation request) async {
    final requestSequence = _advanceSequenceOutsideQueryTransition();
    _loadingTimeNavigationGeneration = request.generation;
    state = state.copyWith(
      isLoadingTimeAnchor: true,
      timeNavigationErrorMessage: null,
    );
    try {
      final snapshot = await ref
          .read(libraryCatalogProvider)
          .loadAtTime(
            maxItems: libraryTimelineWindow,
            query: request.query,
            anchor: request.anchor,
          );
      if (!_canPublishTimeNavigation(request, requestSequence)) {
        return false;
      }
      if (snapshot.revision != request.timeline.revision ||
          snapshot.queryId != request.timeline.queryId) {
        throw const LibraryCatalogFailure(
          code: "catalog_cursor_stale",
          message: "The catalog changed while navigating the timeline",
        );
      }
      _previewCoordinator?.retainPending(
        snapshot.assets.map((asset) => asset.locationId),
      );
      _resetRetainedCatalogPages(
        assets: snapshot.assets,
        startItemOffset: request.globalItemOffset,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
      );
      state = state.copyWith(
        roots: snapshot.roots,
        assets: snapshot.assets,
        catalogPath: snapshot.catalogPath,
        catalogRevision: snapshot.revision,
        queryId: snapshot.queryId,
        windowStartItemOffset: request.globalItemOffset,
        previousCursor: snapshot.previousCursor,
        nextCursor: snapshot.nextCursor,
        activeTimeAnchor: request.anchor,
        isLoadingTimeAnchor: false,
        pageErrorMessage: null,
      );
      return true;
    } on LibraryCatalogFailure catch (error) {
      if (!_canPublishTimeNavigation(request, requestSequence)) {
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
      if (_canPublishTimeNavigation(request, requestSequence)) {
        state = state.copyWith(
          isLoadingTimeAnchor: false,
          timeNavigationErrorMessage: error.toString(),
        );
      }
      return false;
    } finally {
      _releaseTimeNavigationLoading(request);
    }
  }

  bool _canPublishTimeNavigation(
    _PendingTimeNavigation request,
    int requestSequence,
  ) {
    return !_isDisposed &&
        requestSequence == _scanSequence &&
        request.generation == _timeNavigationGeneration &&
        _isCompatibleTimeNavigation(request);
  }

  void _releaseTimeNavigationLoading(_PendingTimeNavigation request) {
    if (_loadingTimeNavigationGeneration != request.generation) {
      return;
    }
    _loadingTimeNavigationGeneration = null;
    if (!_isDisposed && state.isLoadingTimeAnchor) {
      state = state.copyWith(isLoadingTimeAnchor: false);
    }
  }

  Future<bool> unregisterRoot(LibraryRoot root) async {
    if (state.isBusy) {
      return false;
    }
    final requestSequence = _advanceSequenceOutsideQueryTransition();
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

  void requestPreview(
    LibraryAsset asset, {
    bool retry = false,
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
    int previewEdge = _previewEdge,
  }) {
    _previews.request(
      asset,
      retry: retry,
      priority: priority,
      previewEdge: previewEdge,
    );
  }

  LibraryAsset resolvePreview(LibraryAsset asset) {
    return _previews.resolve(asset);
  }

  Stream<void> watchPreview(String locationId) {
    return _previews.watch(locationId);
  }

  void updateGalleryPreviewDemand({
    Iterable<LibraryAsset> visible = const <LibraryAsset>[],
    Iterable<LibraryAsset> nearDirection = const <LibraryAsset>[],
    Iterable<LibraryAsset> guard = const <LibraryAsset>[],
    Iterable<LibraryAsset> idle = const <LibraryAsset>[],
    Map<String, int> previewEdges = const <String, int>{},
  }) {
    _previews.updateGalleryDemand(
      visible: visible,
      nearDirection: nearDirection,
      guard: guard,
      idle: idle,
      previewEdges: previewEdges,
    );
  }

  void updateViewerPreviewDemand(LibraryAsset? viewer) {
    _previews.updateViewerDemand(viewer);
  }

  void _invalidatePreviewContext() {
    _previewCoordinator?.invalidateAll();
  }

  void cancelTimeNavigation() {
    if (_isDisposed) {
      return;
    }
    final pending = _pendingTimeNavigation;
    final active = _activeTimeNavigation;
    final owner = _timeNavigationOwner;
    final hasPendingRequest =
        pending != null && !pending.completion.isCompleted;
    final hasActiveRequest = active != null && !active.completion.isCompleted;
    if (!hasPendingRequest && !hasActiveRequest && owner == null) {
      if (state.activeTimeAnchor != null) {
        state = state.copyWith(activeTimeAnchor: null);
      }
      return;
    }
    _timeNavigationGeneration += 1;
    _pendingTimeNavigation = null;
    _timeNavigationOwner = null;
    _timeNavigationRetryTimer?.cancel();
    _timeNavigationRetryTimer = null;
    if (pending != null && !pending.completion.isCompleted) {
      pending.completion.complete(false);
    }
    if (active != null && !active.completion.isCompleted) {
      active.completion.complete(false);
    }
    _loadingTimeNavigationGeneration = null;
    if (state.isLoadingTimeAnchor || state.activeTimeAnchor != null) {
      state = state.copyWith(
        activeTimeAnchor: null,
        isLoadingTimeAnchor: false,
      );
    }
  }

  void ensureVisibleRange({
    required int startItemOffset,
    required int endItemOffsetExclusive,
  }) {
    if (_isDisposed || endItemOffsetExclusive <= startItemOffset) {
      return;
    }
    final totalItems = state.timeline?.totalItems ?? 0;
    if (totalItems <= 0) {
      return;
    }
    final start = startItemOffset.clamp(0, totalItems - 1).toInt();
    final end = endItemOffsetExclusive.clamp(start + 1, totalItems).toInt();
    final range = (start: start, end: end);
    if (!_visibleRangeRetainsNavigationOwner(range)) {
      return;
    }
    if (_timeNavigationOwner == null) {
      _retainPassiveTimeNavigationForVisibleRange(range);
    }
    final loadedStart = state.windowStartItemOffset;
    final loadedEnd = loadedStart + state.assets.length;
    if (range.start >= loadedStart && range.end <= loadedEnd) {
      _pendingVisibleRange = null;
      return;
    }
    if (_pendingVisibleRange == range) {
      return;
    }
    _pendingVisibleRange = range;
    _scheduleVisibleRangeDrain();
  }

  bool _visibleRangeRetainsNavigationOwner(({int start, int end}) range) {
    final owner = _timeNavigationOwner;
    if (owner == null) {
      return true;
    }
    if (!_isCompatibleTimeNavigation(owner)) {
      _timeNavigationOwner = null;
      return true;
    }
    return owner.globalItemOffset >= range.start &&
        owner.globalItemOffset < range.end;
  }

  bool _hasCompatibleTimeNavigationOwner() {
    final owner = _timeNavigationOwner;
    if (owner == null) {
      return false;
    }
    if (_isCompatibleTimeNavigation(owner)) {
      return true;
    }
    _timeNavigationOwner = null;
    return false;
  }

  void _retainPassiveTimeNavigationForVisibleRange(
    ({int start, int end}) range,
  ) {
    bool contains(_PendingTimeNavigation request) {
      return request.globalItemOffset >= range.start &&
          request.globalItemOffset < range.end;
    }

    final pending = _pendingTimeNavigation;
    if (pending != null && !contains(pending)) {
      if (pending.generation == _timeNavigationGeneration) {
        _timeNavigationGeneration += 1;
      }
      _pendingTimeNavigation = null;
      if (!pending.completion.isCompleted) {
        pending.completion.complete(false);
      }
    }
    final active = _activeTimeNavigation;
    if (active != null &&
        !contains(active) &&
        active.generation == _timeNavigationGeneration) {
      _timeNavigationGeneration += 1;
    }
    if (_pendingTimeNavigation == null) {
      _timeNavigationRetryTimer?.cancel();
      _timeNavigationRetryTimer = null;
    }
  }

  void _scheduleVisibleRangeDrain() {
    if (_isDisposed ||
        _isEnsuringVisibleRange ||
        _isVisibleRangeDrainScheduled ||
        _pendingVisibleRange == null) {
      return;
    }
    _isVisibleRangeDrainScheduled = true;
    unawaited(
      Future<void>.microtask(() {
        _isVisibleRangeDrainScheduled = false;
        return _drainVisibleRange();
      }),
    );
  }

  Future<void> _drainVisibleRange() async {
    _isVisibleRangeDrainScheduled = false;
    if (_isDisposed || _isEnsuringVisibleRange) {
      return;
    }
    _isEnsuringVisibleRange = true;
    try {
      while (!_isDisposed) {
        final range = _pendingVisibleRange;
        _pendingVisibleRange = null;
        if (range == null) {
          return;
        }
        await _loadVisibleRange(range);
      }
    } finally {
      _isEnsuringVisibleRange = false;
      if (_pendingVisibleRange != null && !_isDisposed) {
        _scheduleVisibleRangeDrain();
      }
    }
  }

  Future<void> _loadVisibleRange(({int start, int end}) range) async {
    for (var attempt = 0; attempt < _maxVisibleRangePageLoads; attempt++) {
      if (_isDisposed ||
          state.assets.isEmpty ||
          !_visibleRangeRetainsNavigationOwner(range)) {
        return;
      }
      final loadedStart = state.windowStartItemOffset;
      final loadedEnd = loadedStart + state.assets.length;
      if (range.end <= loadedStart || range.start >= loadedEnd) {
        await _loadDisjointVisibleRange(range.start);
        return;
      }
      if (range.start < loadedStart) {
        final previousStart = loadedStart;
        final didLoad = await loadPreviousPage();
        if (!didLoad || state.windowStartItemOffset >= previousStart) {
          return;
        }
        continue;
      }
      if (range.end > loadedEnd) {
        final previousEnd = loadedEnd;
        await loadNextPage();
        final nextEnd = state.windowStartItemOffset + state.assets.length;
        if (nextEnd <= previousEnd) {
          return;
        }
        continue;
      }
      return;
    }
  }

  Future<void> _loadDisjointVisibleRange(int globalItemOffset) async {
    final timeline = state.timeline;
    if (timeline == null || timeline.buckets.isEmpty) {
      return;
    }
    final target = globalItemOffset
        .clamp(0, timeline.totalItems > 0 ? timeline.totalItems - 1 : 0)
        .toInt();
    var precedingItems = 0;
    for (final bucket in timeline.buckets) {
      final bucketEnd = precedingItems + bucket.itemCount;
      if (target < bucketEnd) {
        await _requestTimeNavigation(
          bucket,
          itemOffset: target - precedingItems,
          ownsVisibleRange: false,
        );
        return;
      }
      precedingItems = bucketEnd;
    }
    final lastBucket = timeline.buckets.last;
    await _requestTimeNavigation(
      lastBucket,
      itemOffset: lastBucket.itemCount > 0 ? lastBucket.itemCount - 1 : 0,
      ownsVisibleRange: false,
    );
  }

  bool _canPublishPreview(LibraryAsset replacement) {
    for (final asset in state.assets) {
      if (asset.locationId == replacement.locationId) {
        return libraryPreviewSourcesAreCompatible(asset, replacement);
      }
    }
    return true;
  }

  void _handlePreviewPublished(LibraryAsset replacement) {
    for (var index = 0; index < state.assets.length; index++) {
      final asset = state.assets[index];
      if (asset.locationId != replacement.locationId) {
        continue;
      }
      final revision = state.catalogRevision;
      if ((asset.width <= 0 || asset.height <= 0) &&
          replacement.width > 0 &&
          replacement.height > 0 &&
          revision != null &&
          state.queryId.isNotEmpty) {
        _layoutDimensionUpdates.add(
          LibraryGalleryLayoutDimensionUpdate(
            revision: revision,
            queryId: state.queryId,
            globalItemIndex: state.windowStartItemOffset + index,
            locationId: replacement.locationId,
            width: replacement.width,
            height: replacement.height,
          ),
        );
      }
      break;
    }
  }

  Stream<LibraryGalleryLayoutDimensionUpdate> watchLayoutDimensionUpdates() {
    return _layoutDimensionUpdates.stream;
  }

  void _handleUpdate(_ActiveScanRun run, LibraryScanUpdate update) {
    if (!_ownsScanRun(run)) {
      return;
    }
    if (update case LibraryScanStarted(
      :final scanId,
    ) when scanId != run.scanId) {
      _handleError(
        run,
        const LibraryScanFailure(
          code: "bridge_scan_id_mismatch",
          message: "Received a start event for a different scan",
        ),
        StackTrace.current,
      );
      return;
    }
    if (update is LibraryScanStarted) {
      run.didStart = true;
    }
    final transition = _scanSession.apply(state, update);
    state = transition.state;
    if (_isTerminalScanUpdate(update)) {
      run.didReceiveTerminal = true;
    }
    if (transition.shouldReloadCatalog) {
      unawaited(_reloadPublishedCatalog(_scanSequence));
    }
  }

  void _handleError(_ActiveScanRun run, Object error, StackTrace stackTrace) {
    if (!_ownsScanRun(run)) {
      return;
    }
    _releaseScanRun(run);
    state = _scanSession.fail(state, error);
  }

  void _handleDone(_ActiveScanRun run) {
    if (!_ownsScanRun(run)) {
      if (!run.streamDone.isCompleted) {
        run.streamDone.complete();
      }
      return;
    }
    if (run.didReceiveTerminal) {
      _releaseScanRun(run);
      return;
    }
    if (!run.streamDone.isCompleted) {
      run.streamDone.complete();
    }
    unawaited(_reconcileEndedScan(run, _scanSequence));
  }

  void _releaseScanRun(_ActiveScanRun run) {
    if (identical(_activeScanRun, run)) {
      _activeScanRun = null;
    }
    if (!run.streamDone.isCompleted) {
      run.streamDone.complete();
    }
  }

  bool _ownsScanRun(_ActiveScanRun run) {
    final active = _activeScanRun;
    return !_isDisposed &&
        identical(active, run) &&
        active?.generation == run.generation &&
        active?.scanId == run.scanId;
  }

  bool _isTerminalScanUpdate(LibraryScanUpdate update) {
    return update is LibraryScanCompleted ||
        update is LibraryScanCancelled ||
        update is LibraryScanPaused ||
        update is LibraryScanStale ||
        update is LibraryScanFailed;
  }

  Future<void> _reconcileEndedScan(_ActiveScanRun run, int scanSequence) async {
    try {
      final snapshot = await ref
          .read(libraryCatalogProvider)
          .load(maxItems: libraryCatalogWindow, query: state.query);
      if (!_ownsScanRun(run) || scanSequence != _scanSequence) {
        return;
      }
      LibraryRoot? publishedRoot;
      for (final root in snapshot.roots) {
        if (root.activeScanId == run.scanId) {
          publishedRoot = root;
          break;
        }
      }
      if (run.didStart && publishedRoot != null) {
        final transition = _scanSession.apply(
          state,
          LibraryScanCompleted(
            assetCount: publishedRoot.assetCount,
            issueCount: publishedRoot.issueCount,
            catalogPath: snapshot.catalogPath,
            wasLimited: _didReachScanLimit(publishedRoot),
          ),
        );
        state = transition.state;
        await _reloadFirstCatalogPage(scanSequence);
        if (_ownsScanRun(run)) {
          _releaseScanRun(run);
        }
        return;
      }
      final paused = await ref.read(libraryScannerProvider).loadPausedScan();
      if (!_ownsScanRun(run) || scanSequence != _scanSequence) {
        return;
      }
      final pausedScan = paused;
      if (pausedScan != null && pausedScan.scanId == run.scanId) {
        _releaseScanRun(run);
        _scanSession.restorePaused(pausedScan);
        state = state.copyWith(
          status: LibraryStatus.paused,
          visitedEntries: pausedScan.visitedEntries,
          stagedAssetCount: pausedScan.acceptedItems,
          issueCount: pausedScan.issueCount,
          isResumingScan: false,
        );
        return;
      }
      _releaseScanRun(run);
      if (state.status == LibraryStatus.cancelling) {
        state = _scanSession
            .apply(
              state,
              LibraryScanCancelled(
                acceptedItems: state.stagedAssetCount,
                issueCount: state.issueCount,
              ),
            )
            .state;
        return;
      }
      state = _scanSession.finish(state);
    } on Object catch (error) {
      if (!_ownsScanRun(run) || scanSequence != _scanSequence) {
        return;
      }
      _releaseScanRun(run);
      state = _scanSession.fail(
        state,
        LibraryScanFailure(
          code: "scan_terminal_reconciliation_failed",
          message: error.toString(),
        ),
      );
    } finally {
      if (_ownsScanRun(run)) {
        _releaseScanRun(run);
      }
    }
  }

  bool _didReachScanLimit(LibraryRoot publishedRoot) {
    final itemLimit = state.itemLimit;
    if (itemLimit != null && publishedRoot.assetCount >= itemLimit) {
      return true;
    }
    final entryLimit = state.entryLimit;
    return entryLimit != null && state.visitedEntries >= entryLimit;
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
    return _enqueueScanStart(
      allowedBusyStatus: LibraryStatus.paused,
      start: () {
        _advanceSequenceOutsideQueryTransition();
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
      },
    );
  }

  int _advanceSequenceOutsideQueryTransition() {
    _queryTransitionBaseState = null;
    _queryTransitionRequestSequence = null;
    return ++_scanSequence;
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

  void _resetRetainedCatalogPages({
    required List<LibraryAsset> assets,
    required int startItemOffset,
    required LibraryCatalogCursor? previousCursor,
    required LibraryCatalogCursor? nextCursor,
  }) {
    _retainedCatalogPages
      ..clear()
      ..addAll(
        assets.isEmpty
            ? const []
            : [
                _RetainedCatalogPage(
                  assets: List.unmodifiable(assets),
                  startItemOffset: startItemOffset,
                  previousCursor: previousCursor,
                  nextCursor: nextCursor,
                ),
              ],
      );
  }

  void _ensureRetainedCatalogPagesCurrent() {
    final retainedCount = _retainedCatalogPages.fold(
      0,
      (total, page) => total + page.assets.length,
    );
    if (_retainedCatalogPages.isNotEmpty &&
        retainedCount == state.assets.length &&
        _retainedCatalogPages.first.startItemOffset ==
            state.windowStartItemOffset) {
      return;
    }
    _resetRetainedCatalogPages(
      assets: state.assets,
      startItemOffset: state.windowStartItemOffset,
      previousCursor: state.previousCursor,
      nextCursor: state.nextCursor,
    );
  }

  void _mergeRetainedCatalogAssetUpdates(List<LibraryAsset> updates) {
    final replacements = {for (final asset in updates) asset.locationId: asset};
    if (replacements.isEmpty) {
      return;
    }
    for (var index = 0; index < _retainedCatalogPages.length; index++) {
      final page = _retainedCatalogPages[index];
      var didChange = false;
      final assets = [
        for (final asset in page.assets)
          replacements[asset.locationId] ?? asset,
      ];
      for (var assetIndex = 0; assetIndex < page.assets.length; assetIndex++) {
        if (!identical(page.assets[assetIndex], assets[assetIndex])) {
          didChange = true;
          break;
        }
      }
      if (didChange) {
        _retainedCatalogPages[index] = _RetainedCatalogPage(
          assets: List.unmodifiable(assets),
          startItemOffset: page.startItemOffset,
          previousCursor: page.previousCursor,
          nextCursor: page.nextCursor,
        );
      }
    }
  }

  void _trimRetainedCatalogPages({required bool trimLeading}) {
    var retainedCount = _retainedCatalogPages.fold(
      0,
      (total, page) => total + page.assets.length,
    );
    if (retainedCount <= _retainedDetailHighWatermark) {
      return;
    }
    while (_retainedCatalogPages.length > 1 &&
        retainedCount > _retainedDetailLowWatermark) {
      final removed = trimLeading
          ? _retainedCatalogPages.removeAt(0)
          : _retainedCatalogPages.removeLast();
      retainedCount -= removed.assets.length;
    }
  }

  List<LibraryAsset> _retainedCatalogAssets() {
    return List.unmodifiable([
      for (final page in _retainedCatalogPages) ...page.assets,
    ]);
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
    final scanId = state.scanId;
    final visitedEntries = state.visitedEntries;
    final stagedAssetCount = state.stagedAssetCount;
    final scanPhase = state.scanPhase;
    final validatedAssetCount = state.validatedAssetCount;
    final validationAssetCount = state.validationAssetCount;
    final itemLimit = state.itemLimit;
    final entryLimit = state.entryLimit;
    final recentIssues = state.recentIssues;
    final isScanLimited = state.isScanLimited;
    if (state.catalogRevision != snapshot.revision ||
        state.queryId != snapshot.queryId) {
      _invalidatePreviewContext();
    }
    _resetRetainedCatalogPages(
      assets: snapshot.assets,
      startItemOffset: 0,
      previousCursor: snapshot.previousCursor,
      nextCursor: snapshot.nextCursor,
    );
    state = LibraryState.fromSnapshot(snapshot, query: query).copyWith(
      rootPath: rootPath,
      displayRootPath: displayRootPath,
      scanId: scanId,
      visitedEntries: visitedEntries,
      stagedAssetCount: stagedAssetCount,
      scanPhase: scanPhase,
      validatedAssetCount: validatedAssetCount,
      validationAssetCount: validationAssetCount,
      itemLimit: itemLimit,
      entryLimit: entryLimit,
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

int _maxActivePreviewsFor(PreviewLoadingSpeed speed) {
  return switch (speed) {
    PreviewLoadingSpeed.small => 1,
    PreviewLoadingSpeed.medium => 2,
    PreviewLoadingSpeed.large => 4,
  };
}

final initialLibraryStateProvider = Provider<LibraryState>((ref) {
  return const LibraryState();
});

final libraryControllerProvider =
    NotifierProvider<LibraryController, LibraryState>(LibraryController.new);
