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
import "library_root_removal_controller.dart";
import "library_scan_execution.dart";
import "library_scan_shutdown.dart";
import "library_scan_session.dart";
import "library_scanner.dart";
import "library_viewport_controller.dart";

export "library_viewport_controller.dart" show LibraryQueryUpdateOutcome;

const _previewEdge = 512;

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
  LibraryViewportController? _viewportController;
  LibraryRootRemovalController? _rootRemovalController;
  bool _isDisposed = false;
  bool _isShutdownSuspending = false;
  LibraryScanExecutionCoordinator? _scanExecutionCoordinator;
  LibraryScanShutdownCoordinator? _scanShutdownCoordinator;

  LibraryViewportController get _viewport =>
      _viewportController ??= LibraryViewportController(
        ref.read(libraryCatalogProvider),
        () => state,
        (nextState) => state = nextState,
        () => _isDisposed,
        (locationIds) => _previewCoordinator?.retainPending(locationIds),
      );

  LibraryRootRemovalController get _rootRemoval =>
      _rootRemovalController ??= LibraryRootRemovalController(
        ref.read(libraryCatalogProvider),
        _viewport,
        () => state,
        (nextState) => state = nextState,
        () => _isDisposed,
        (rootId) => ref
            .read(libraryScanExecutionCoordinatorProvider)
            .hasUpdateForRoot(rootId),
        () => _scanSequence += 1,
      );

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
    final scanShutdownCoordinator = ref.read(
      libraryScanShutdownCoordinatorProvider,
    );
    final scanExecutionCoordinator = ref.read(
      libraryScanExecutionCoordinatorProvider,
    );
    _scanExecutionCoordinator = scanExecutionCoordinator;
    _scanShutdownCoordinator = scanShutdownCoordinator;
    scanShutdownCoordinator.attach(this, _suspendActiveScanForShutdown);
    ref.onDispose(() {
      scanShutdownCoordinator.detach(this);
      _isDisposed = true;
      final activeScanRun = _activeScanRun;
      _activeScanRun = null;
      scanExecutionCoordinator.releasePrimary(this);
      if (activeScanRun != null && !activeScanRun.streamDone.isCompleted) {
        activeScanRun.streamDone.complete();
      }
      final scanId = _scanSession.activeScanId;
      if (scanId != null &&
          !_isShutdownSuspending &&
          !scanShutdownCoordinator.isShuttingDown) {
        scanner.cancel(scanId);
      }
      _previewCoordinator?.dispose();
      _rootRemovalController?.dispose();
      _viewportController?.dispose();
      unawaited(_layoutDimensionUpdates.close());
      unawaited(_subscription?.cancel());
    });
    final initialState = ref.watch(initialLibraryStateProvider);
    _viewport.seed(initialState);
    Future<void>.microtask(_resumeInterruptedScanIfAvailable);
    Future<void>.microtask(_viewport.loadInitialTimeline);
    return initialState;
  }

  Future<void> chooseDirectoryAndScan() async {
    final scanExecutionCoordinator = ref.read(
      libraryScanExecutionCoordinatorProvider,
    );
    if (state.isBusy || !scanExecutionCoordinator.tryAcquirePrimary(this)) {
      return;
    }

    state = state.copyWith(
      status: LibraryStatus.choosingDirectory,
      scanId: null,
      rootPath: null,
      displayRootPath: null,
      taskKind: LibraryTaskKind.import,
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
    } finally {
      if (_activeScanRun == null) {
        scanExecutionCoordinator.releasePrimary(this);
      }
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
            _isShutdownSuspending ||
            (_scanShutdownCoordinator?.isShuttingDown ?? false) ||
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
    final run = _ActiveScanRun(
      scanId: scanId,
      generation: ++_scanRunGeneration,
    );
    final scanExecutionCoordinator = _readScanExecutionCoordinator();
    if (!scanExecutionCoordinator.tryAcquirePrimary(this)) {
      throw const LibraryScanFailure(
        code: "foreground_scan_busy",
        message: "Another library update is already running",
      );
    }
    _activeScanRun = run;
    try {
      await _subscription?.cancel();
      final taskKind = state.scanId == scanId && state.taskKind != null
          ? state.taskKind!
          : _taskKindForRoot(rootPath, displayRootPath ?? rootPath);
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
        taskKind: taskKind,
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

      final scanner = ref.read(libraryScannerProvider);
      final stream = isResuming
          ? scanner.resume(
              scanId: scanId,
              rootPath: rootPath,
              itemLimit: itemLimit,
              entryLimit: entryLimit,
              previewEdge: previewEdge,
            )
          : scanner.scan(
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

  LibraryTaskKind _taskKindForRoot(String rootPath, String displayRootPath) {
    final isConfiguredRoot = state.roots.any(
      (root) =>
          root.path == rootPath ||
          root.path == displayRootPath ||
          root.displayPath == rootPath ||
          root.displayPath == displayRootPath,
    );
    return isConfiguredRoot ? LibraryTaskKind.update : LibraryTaskKind.import;
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

  Future<void> _suspendActiveScanForShutdown() async {
    if (_isDisposed || _isShutdownSuspending) {
      return;
    }
    _isShutdownSuspending = true;
    await _scanStartQueue;
    final scanner = ref.read(libraryScannerProvider);
    while (!_isDisposed) {
      final run = _activeScanRun;
      if (run == null || run.streamDone.isCompleted) {
        return;
      }
      if (scanner.suspend(run.scanId)) {
        await run.streamDone.future;
        return;
      }
      await Future.any([
        run.streamDone.future,
        Future<void>.delayed(const Duration(milliseconds: 10)),
      ]);
    }
  }

  void dismissTaskFeedback() {
    if (state.isCommittedRemovalReloadPending) {
      return;
    }
    final isDismissible =
        state.status == LibraryStatus.completed ||
        state.status == LibraryStatus.failed ||
        state.status == LibraryStatus.cancelled;
    if (!isDismissible) {
      return;
    }
    state = state.copyWith(
      status: state.roots.isEmpty
          ? LibraryStatus.empty
          : LibraryStatus.completed,
      scanId: null,
      rootPath: null,
      displayRootPath: null,
      taskKind: null,
      removingRootId: null,
      removingRootDisplayPath: null,
      isRemovalCommitted: false,
      visitedEntries: 0,
      stagedAssetCount: 0,
      scanPhase: LibraryScanPhase.discovering,
      validatedAssetCount: 0,
      validationAssetCount: 0,
      itemLimit: null,
      entryLimit: null,
      isScanLimited: false,
      errorMessage: null,
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
    if (state.taskKind == LibraryTaskKind.remove) {
      if (state.isRemovalCommitted) {
        await _rootRemoval.retryCommittedReload();
        return;
      }
      final rootId = state.removingRootId;
      LibraryRoot? root;
      if (rootId != null) {
        for (final candidate in state.roots) {
          if (candidate.id == rootId) {
            root = candidate;
            break;
          }
        }
      }
      if (root != null) {
        await unregisterRoot(root);
      }
      return;
    }
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
  }) {
    return _viewport.updateQuery(
      query,
      anchorLocationId: anchorLocationId,
      anchorAssetId: anchorAssetId,
      fallbackGlobalItemIndex: fallbackGlobalItemIndex,
      forceRefresh: forceRefresh,
      minimumCatalogRevision: minimumCatalogRevision,
      showRefreshingStatus: showRefreshingStatus,
    );
  }

  Future<LibraryQueryUpdateOutcome> refreshFromSynchronization({
    required BigInt catalogRevision,
    String? anchorLocationId,
    String? anchorAssetId,
    int? fallbackGlobalItemIndex,
  }) {
    return _viewport.refreshFromSynchronization(
      catalogRevision: catalogRevision,
      anchorLocationId: anchorLocationId,
      anchorAssetId: anchorAssetId,
      fallbackGlobalItemIndex: fallbackGlobalItemIndex,
    );
  }

  Future<bool> refreshCurrentQuery() => _viewport.refreshCurrentQuery();

  Future<void> loadNextPage() => _viewport.loadNextPage();

  Future<bool> loadPreviousPage() => _viewport.loadPreviousPage();

  Future<bool> jumpToTime(LibraryTimeBucket bucket, {int itemOffset = 0}) {
    return _viewport.jumpToTime(bucket, itemOffset: itemOffset);
  }

  Future<bool> prefetchTime(LibraryTimeBucket bucket, {int itemOffset = 0}) {
    return _viewport.prefetchTime(bucket, itemOffset: itemOffset);
  }

  Future<bool> unregisterRoot(LibraryRoot root) {
    return _rootRemoval.unregisterRoot(root);
  }

  PreparedLibraryRootRemoval? prepareRootRemoval(LibraryRoot root) {
    return _rootRemoval.prepareRemoval(root);
  }

  Future<bool> executeRootRemoval(PreparedLibraryRootRemoval prepared) {
    return _rootRemoval.executeRemoval(prepared);
  }

  void abandonRootRemoval(PreparedLibraryRootRemoval prepared) {
    _rootRemovalController?.abandonRemoval(prepared);
  }

  void restorePreviewAuthority(String rootId) {
    _previewCoordinator?.restoreRootAuthority(rootId);
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

  Future<LibraryPreviewRequestOutcome> retryPreview(
    LibraryAsset asset, {
    LibraryPreviewPriority priority = LibraryPreviewPriority.visible,
    int previewEdge = _previewEdge,
  }) {
    if (_isDisposed) {
      return Future.value(LibraryPreviewRequestOutcome.disposed);
    }
    return _previews.retry(asset, priority: priority, previewEdge: previewEdge);
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

  void cancelTimeNavigation() => _viewport.cancelTimeNavigation();

  void ensureVisibleRange({
    required int startItemOffset,
    required int endItemOffsetExclusive,
  }) {
    _viewport.ensureVisibleRange(
      startItemOffset: startItemOffset,
      endItemOffsetExclusive: endItemOffsetExclusive,
    );
  }

  bool _canPublishPreview(LibraryAsset replacement) {
    for (final asset in state.assets) {
      if (asset.locationId == replacement.locationId) {
        return libraryPreviewSourcesAreCompatible(asset, replacement);
      }
    }
    return false;
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
      final scanExecutionCoordinator = _scanExecutionCoordinator;
      if (scanExecutionCoordinator != null) {
        scanExecutionCoordinator.releasePrimary(this);
      }
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

  LibraryScanExecutionCoordinator _readScanExecutionCoordinator() {
    final scanExecutionCoordinator = _scanExecutionCoordinator;
    if (scanExecutionCoordinator != null) {
      return scanExecutionCoordinator;
    }
    final initialized = ref.read(libraryScanExecutionCoordinatorProvider);
    _scanExecutionCoordinator = initialized;
    return initialized;
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
        await _viewport.reloadFirstCatalogPage();
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
      await _viewport.reloadFirstCatalogPage();
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
    _viewport.supersedeExternalRequests();
    return ++_scanSequence;
  }
}

int _maxActivePreviewsFor(PreviewLoadingSpeed speed) {
  return switch (speed) {
    PreviewLoadingSpeed.small => 1,
    PreviewLoadingSpeed.medium => 2,
    PreviewLoadingSpeed.large => 3,
  };
}

final initialLibraryStateProvider = Provider<LibraryState>((ref) {
  return const LibraryState();
});

final libraryControllerProvider =
    NotifierProvider<LibraryController, LibraryState>(LibraryController.new);
