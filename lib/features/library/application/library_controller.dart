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
import "library_primary_scan_lifecycle.dart";
import "library_scan_shutdown.dart";
import "library_scanner.dart";
import "library_viewport_controller.dart";

export "library_viewport_controller.dart" show LibraryQueryUpdateOutcome;

const _previewEdge = 512;

class LibraryController extends Notifier<LibraryState>
    implements LibraryPrimaryScanHost {
  LibraryPrimaryScanLifecycle? _primaryScan;
  final StreamController<LibraryGalleryLayoutDimensionUpdate>
  _layoutDimensionUpdates = StreamController.broadcast(sync: true);
  LibraryPreviewCoordinator? _previewCoordinator;
  LibraryViewportController? _viewportController;
  LibraryRootRemovalController? _rootRemovalController;
  bool _isDisposed = false;
  LibraryPrimaryScanLifecycle get _primary =>
      _primaryScan ??= _createPrimary(state.primaryScanSnapshot);

  LibraryPrimaryScanLifecycle _createPrimary(
    LibraryPrimaryScanSnapshot initial,
  ) => LibraryPrimaryScanLifecycle(
    initial: initial.taskKind == LibraryTaskKind.remove
        ? const LibraryPrimaryScanSnapshot()
        : initial,
    host: this,
    scanner: ref.read(libraryScannerProvider),
    picker: ref.read(directoryPickerProvider),
    catalog: ref.read(libraryCatalogProvider),
    admission: ref.read(libraryScanExecutionCoordinatorProvider),
    shutdown: ref.read(libraryScanShutdownCoordinatorProvider),
    previewEdge: _previewEdge,
  );

  @override
  LibraryState get libraryState => state;

  @override
  void publishPrimaryScan(LibraryPrimaryScanSnapshot snapshot) {
    state = state.copyWith(primaryScan: snapshot);
  }

  void _publishCatalogState(LibraryState next) {
    state = next.copyWith(
      primaryScan: _primaryScan?.snapshot ?? state.primaryScanSnapshot,
    );
  }

  @override
  void preparePrimaryScanExecution() => _viewport.supersedeExternalRequests();

  @override
  Future<bool> reloadPrimaryScanCatalog() => _viewport.reloadFirstCatalogPage();

  LibraryViewportController get _viewport =>
      _viewportController ??= LibraryViewportController(
        ref.read(libraryCatalogProvider),
        () => state,
        _publishCatalogState,
        () => _isDisposed,
        (locationIds) => _previewCoordinator?.retainPending(locationIds),
      );

  LibraryRootRemovalController get _rootRemoval =>
      _rootRemovalController ??= LibraryRootRemovalController(
        ref.read(libraryCatalogProvider),
        _viewport,
        () => state,
        _publishCatalogState,
        () => _isDisposed,
        (rootId) => ref
            .read(libraryScanExecutionCoordinatorProvider)
            .hasUpdateForRoot(rootId),
        () => _primary.invalidateRestoration(),
        (root) => _primary.rootUnregistered(root),
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
    ref.onDispose(() {
      _isDisposed = true;
      _primaryScan?.dispose();
      _previewCoordinator?.dispose();
      _rootRemovalController?.dispose();
      _viewportController?.dispose();
      unawaited(_layoutDimensionUpdates.close());
    });
    final initialState = ref.watch(initialLibraryStateProvider);
    _viewport.seed(initialState);
    final primary = _primaryScan = _createPrimary(
      initialState.primaryScanSnapshot,
    );
    Future<void>.microtask(primary.restore);
    Future<void>.microtask(_viewport.loadInitialTimeline);
    return initialState.copyWith(primaryScan: primary.snapshot);
  }

  Future<void> chooseDirectoryAndScan() => _primary.chooseDirectoryAndScan();
  Future<void> scanDirectory(String rootPath) =>
      _primary.scanDirectory(rootPath);
  Future<void> cancelScan() => _primary.cancel();
  void pauseScan() => _primary.pause();
  Future<void> resumePausedScan() => _primary.resume();

  void dismissTaskFeedback() {
    if (state.taskKind == LibraryTaskKind.remove) {
      _rootRemoval.dismissTaskFeedback();
    } else {
      _primary.dismiss();
    }
  }

  Future<void> retry() async {
    if (state.taskKind != LibraryTaskKind.remove) {
      await _primary.retry();
      return;
    }
    if (state.isRemovalCommitted) {
      await _rootRemoval.retryCommittedReload();
      return;
    }
    for (final root in state.roots) {
      if (root.id == state.removingRootId) {
        await unregisterRoot(root);
        return;
      }
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
