import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../adapters/windows_library_platform_actions.dart";
import "../../settings/application/ame_preferences.dart";
import "../../settings/presentation/ame_settings_page.dart";
import "../application/library_controller.dart";
import "../application/library_folder_controller.dart";
import "../application/library_layout_manifest_catalog.dart";
import "../application/library_view_preferences.dart";
import "../domain/gallery_layout_manifest.dart";
import "../domain/library_folder_models.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "gallery_selection.dart";
import "library_strings.dart";
import "widgets/library_asset_information_sheet.dart";
import "widgets/library_gallery_header.dart";
import "widgets/library_gallery_layout.dart";
import "widgets/library_gallery_states.dart";
import "widgets/library_gallery_wall.dart";
import "widgets/library_global_bar.dart";
import "widgets/library_image_viewer.dart";
import "widgets/library_main_surface.dart";
import "widgets/library_navigation.dart";
import "widgets/library_navigation_resize_handle.dart";
import "widgets/library_task_surface.dart";
import "widgets/library_time_navigation.dart";
import "widgets/library_virtual_gallery_geometry.dart";

class UnifiedLibraryScreen extends ConsumerStatefulWidget {
  const UnifiedLibraryScreen({super.key});

  @override
  ConsumerState<UnifiedLibraryScreen> createState() =>
      _UnifiedLibraryScreenState();
}

class _UnifiedLibraryScreenState extends ConsumerState<UnifiedLibraryScreen> {
  static const _layoutDimensionQuietDelay = Duration(milliseconds: 320);
  static const _layoutDimensionMaximumDelay = Duration(milliseconds: 1600);
  static const _layoutDimensionMinimumBatchSize = 4;
  static const _galleryAnchorTopInset = 18.0;

  final ScrollController _galleryScrollController = ScrollController();
  late final LibraryController _libraryController;
  late final TextEditingController _searchController;
  late GallerySelection _selection;
  late GalleryLayoutShape _layoutShape;
  late GalleryThumbnailSize _thumbnailSize;
  late double _sidebarWidth;
  String? _viewerLocationId;
  bool _isSelecting = false;
  bool _isNavigatingViewer = false;
  bool _isRestoringPreviousWindow = false;
  _LibraryDestination _destination = _LibraryDestination.gallery;
  late final ValueNotifier<_LibraryGalleryLayoutSnapshot?>
  _galleryLayoutSnapshot;
  LibraryGalleryVisiblePosition? _visibleGalleryPosition;
  Timer? _searchDebounce;
  Timer? _layoutDimensionSettleTimer;
  Timer? _layoutDimensionDeadlineTimer;
  StreamSubscription<LibraryGalleryLayoutDimensionUpdate>?
  _layoutDimensionSubscription;
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _pendingLayoutDimensionUpdates = {};
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _deferredLayoutDimensionUpdates = {};
  final Map<int, LibraryGalleryLayoutDimensionUpdate>
  _publishedLayoutDimensionUpdates = {};
  LibraryGalleryLayoutManifest? _dimensionUpdateBaseManifest;
  LibraryGalleryLayoutManifest? _dimensionUpdatedManifest;
  BigInt? _layoutDimensionRevision;
  String _layoutDimensionQueryId = "";
  LibraryGalleryVisibleRange? _visibleGalleryRange;
  int _dimensionUpdateGeneration = 0;
  int _dimensionUpdatedManifestGeneration = -1;

  @override
  void initState() {
    super.initState();
    _libraryController = ref.read(libraryControllerProvider.notifier);
    final state = ref.read(libraryControllerProvider);
    final viewPreferences = ref.read(initialLibraryViewPreferencesProvider);
    final amePreferences = ref.read(initialAmePreferencesProvider);
    _searchController = TextEditingController(text: state.query.searchText);
    _galleryLayoutSnapshot = ValueNotifier(null);
    _selection = GallerySelection.empty(_queryId(state));
    _layoutShape = viewPreferences.layoutShape;
    _thumbnailSize = viewPreferences.thumbnailSize;
    _sidebarWidth = amePreferences.sidebarWidth
        .clamp(ameMinimumSidebarWidth, ameMaximumSidebarWidth)
        .toDouble();
    _layoutDimensionSubscription = _libraryController
        .watchLayoutDimensionUpdates()
        .listen(_handleLayoutDimensionUpdate);
  }

  @override
  void dispose() {
    if (_viewerLocationId != null) {
      _libraryController.updateViewerPreviewDemand(null);
    }
    _searchDebounce?.cancel();
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionDeadlineTimer?.cancel();
    unawaited(_layoutDimensionSubscription?.cancel());
    _galleryScrollController.dispose();
    _galleryLayoutSnapshot.dispose();
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(libraryControllerProvider);
    _synchronizeLayoutDimensionContext(state.catalogRevision, state.queryId);
    final amePreferences = ref.watch(amePreferencesControllerProvider);
    final controller = _libraryController;
    final queryId = _queryId(state);
    final catalogRevision = state.catalogRevision;
    final manifestRequest =
        _layoutShape == GalleryLayoutShape.equalHeight &&
            catalogRevision != null &&
            state.queryId.isNotEmpty
        ? LibraryGalleryLayoutManifestRequest(
            query: state.query,
            revision: catalogRevision,
            queryId: state.queryId,
          )
        : null;
    final baseLayoutManifest = manifestRequest == null
        ? null
        : ref
              .watch(libraryGalleryLayoutManifestProvider(manifestRequest))
              .value;
    final layoutManifest = baseLayoutManifest == null
        ? null
        : _manifestWithRecoveredDimensions(baseLayoutManifest);
    if (_selection.queryId != queryId) {
      _selection = GallerySelection.empty(queryId);
      _isSelecting = false;
    }
    final viewerCatalogAsset = _viewerLocationId == null
        ? null
        : _assetByLocation(state.assets, _viewerLocationId!);
    final viewerIndex = viewerCatalogAsset == null
        ? -1
        : state.assets.indexWhere(
            (asset) => asset.locationId == viewerCatalogAsset.locationId,
          );
    if (viewerCatalogAsset != null) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        if (!mounted || _viewerLocationId != viewerCatalogAsset.locationId) {
          return;
        }
        controller.updateViewerPreviewDemand(viewerCatalogAsset);
      });
    }

    return CallbackShortcuts(
      bindings: viewerCatalogAsset == null
          ? {
              const SingleActivator(
                LogicalKeyboardKey.keyA,
                control: true,
              ): () =>
                  _selectAll(state),
              const SingleActivator(LogicalKeyboardKey.escape):
                  _cancelCurrentMode,
              const SingleActivator(LogicalKeyboardKey.keyD, control: true):
                  _clearSelection,
            }
          : {
              const SingleActivator(LogicalKeyboardKey.escape):
                  _cancelCurrentMode,
            },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          backgroundColor: Theme.of(context).colorScheme.surfaceContainerLow,
          body: Stack(
            children: [
              IndexedStack(
                index: viewerCatalogAsset == null ? 0 : 1,
                children: [
                  Column(
                    children: [
                      LibraryGlobalBar(
                        isBusy: state.isBusy,
                        searchController: _searchController,
                        onSearchChanged: _onSearchChanged,
                      ),
                      Expanded(
                        child: _buildLibrary(
                          context,
                          state,
                          controller,
                          amePreferences,
                          layoutManifest,
                        ),
                      ),
                    ],
                  ),
                  if (viewerCatalogAsset == null)
                    const SizedBox.shrink()
                  else
                    StreamBuilder<void>(
                      stream: controller.watchPreview(
                        viewerCatalogAsset.locationId,
                      ),
                      builder: (context, snapshot) {
                        final viewerAsset = controller.resolvePreview(
                          viewerCatalogAsset,
                        );
                        return LibraryImageViewer(
                          asset: viewerAsset,
                          wheelBehavior: amePreferences.viewerWheelBehavior,
                          openBehavior: amePreferences.viewerOpenBehavior,
                          position: viewerIndex < 0
                              ? null
                              : state.windowStartItemOffset + viewerIndex + 1,
                          totalItems: _totalItems(state),
                          onPrevious:
                              viewerIndex > 0 ||
                                  (state.hasPreviousAssets &&
                                      !_isNavigatingViewer)
                              ? () => unawaited(_openAdjacentAsset(-1))
                              : null,
                          onNext:
                              viewerIndex >= 0 &&
                                  (viewerIndex < state.assets.length - 1 ||
                                      (state.hasMoreAssets &&
                                          !_isNavigatingViewer))
                              ? () => unawaited(_openAdjacentAsset(1))
                              : null,
                          onBack: _closeViewer,
                          onInformation: () =>
                              _showAssetInformation(viewerAsset),
                          onCopyPath: () => _copyAssetPath(viewerAsset),
                          onRevealFile: () => _revealAsset(viewerAsset),
                        );
                      },
                    ),
                ],
              ),
              if (viewerCatalogAsset == null && _showsTaskSurface(state))
                Align(
                  alignment: Alignment.bottomCenter,
                  child: Padding(
                    padding: const EdgeInsets.only(bottom: 24),
                    child: LibraryTaskSurface(
                      state: state,
                      onPause: controller.pauseScan,
                      onCancel: controller.cancelScan,
                      onResume: controller.resumePausedScan,
                      onRetry: controller.retry,
                      onDismiss: controller.dismissCompletedImport,
                    ),
                  ),
                ),
            ],
          ),
        ),
      ),
    );
  }

  void _handleLayoutDimensionUpdate(
    LibraryGalleryLayoutDimensionUpdate update,
  ) {
    if (!mounted) {
      return;
    }
    final current = ref.read(libraryControllerProvider);
    if (current.catalogRevision != update.revision ||
        current.queryId != update.queryId) {
      return;
    }
    _synchronizeLayoutDimensionContext(update.revision, update.queryId);
    if (_visibleGalleryRange?.contains(
          queryId: update.queryId,
          revision: update.revision,
          globalItemIndex: update.globalItemIndex,
        ) ??
        false) {
      _pendingLayoutDimensionUpdates[update.globalItemIndex] = update;
      _scheduleRecoveredDimensionPublication();
    } else {
      _deferredLayoutDimensionUpdates[update.globalItemIndex] = update;
    }
  }

  void _handleVisibleGalleryRangeChanged(LibraryGalleryVisibleRange range) {
    if (!mounted ||
        range.queryId != _layoutDimensionQueryId ||
        range.revision != _layoutDimensionRevision) {
      return;
    }
    final previous = _visibleGalleryRange;
    if (previous != null && previous.matches(range)) {
      return;
    }
    _visibleGalleryRange = range;
    for (final entry in _pendingLayoutDimensionUpdates.entries.toList()) {
      if (!range.containsGlobalItemIndex(entry.key)) {
        _pendingLayoutDimensionUpdates.remove(entry.key);
        _deferredLayoutDimensionUpdates[entry.key] = entry.value;
      }
    }
    for (final entry in _deferredLayoutDimensionUpdates.entries.toList()) {
      if (range.containsGlobalItemIndex(entry.key)) {
        _deferredLayoutDimensionUpdates.remove(entry.key);
        _pendingLayoutDimensionUpdates[entry.key] = entry.value;
      }
    }
    _scheduleRecoveredDimensionPublication();
  }

  void _scheduleRecoveredDimensionPublication() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    if (_pendingLayoutDimensionUpdates.isEmpty) {
      _layoutDimensionDeadlineTimer?.cancel();
      _layoutDimensionDeadlineTimer = null;
      return;
    }
    _layoutDimensionDeadlineTimer ??= Timer(
      _layoutDimensionMaximumDelay,
      _publishRecoveredDimensions,
    );
    if (_pendingLayoutDimensionUpdates.length >=
        _layoutDimensionMinimumBatchSize) {
      _layoutDimensionSettleTimer = Timer(
        _layoutDimensionQuietDelay,
        _publishRecoveredDimensions,
      );
    }
  }

  void _publishRecoveredDimensions() {
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    if (!mounted || _pendingLayoutDimensionUpdates.isEmpty) {
      return;
    }
    final current = ref.read(libraryControllerProvider);
    final compatible = <int, LibraryGalleryLayoutDimensionUpdate>{};
    for (final entry in _pendingLayoutDimensionUpdates.entries) {
      final update = entry.value;
      if (current.catalogRevision == update.revision &&
          current.queryId == update.queryId) {
        compatible[entry.key] = update;
      }
    }
    _pendingLayoutDimensionUpdates.clear();
    if (compatible.isEmpty) {
      return;
    }
    setState(() {
      _publishedLayoutDimensionUpdates.addAll(compatible);
      _dimensionUpdateGeneration += 1;
    });
  }

  LibraryGalleryLayoutManifest _manifestWithRecoveredDimensions(
    LibraryGalleryLayoutManifest manifest,
  ) {
    if (!identical(_dimensionUpdateBaseManifest, manifest) ||
        _dimensionUpdatedManifestGeneration != _dimensionUpdateGeneration) {
      _dimensionUpdateBaseManifest = manifest;
      _dimensionUpdatedManifest = manifest.withDimensionUpdates(
        _publishedLayoutDimensionUpdates.values,
      );
      _dimensionUpdatedManifestGeneration = _dimensionUpdateGeneration;
    }
    return _dimensionUpdatedManifest ?? manifest;
  }

  void _synchronizeLayoutDimensionContext(BigInt? revision, String queryId) {
    if (_layoutDimensionRevision == revision &&
        _layoutDimensionQueryId == queryId) {
      return;
    }
    _layoutDimensionRevision = revision;
    _layoutDimensionQueryId = queryId;
    _layoutDimensionSettleTimer?.cancel();
    _layoutDimensionSettleTimer = null;
    _layoutDimensionDeadlineTimer?.cancel();
    _layoutDimensionDeadlineTimer = null;
    _pendingLayoutDimensionUpdates.clear();
    _deferredLayoutDimensionUpdates.clear();
    _publishedLayoutDimensionUpdates.clear();
    _dimensionUpdateBaseManifest = null;
    _dimensionUpdatedManifest = null;
    _visibleGalleryRange = null;
    _dimensionUpdateGeneration = 0;
    _dimensionUpdatedManifestGeneration = -1;
  }

  Widget _buildLibrary(
    BuildContext context,
    LibraryState state,
    LibraryController controller,
    AmePreferences amePreferences,
    LibraryGalleryLayoutManifest? layoutManifest,
  ) {
    final folderTree = ref.watch(libraryFolderControllerProvider);
    final folderController = ref.read(libraryFolderControllerProvider.notifier);
    return LayoutBuilder(
      builder: (context, constraints) {
        final isCompactNavigation = constraints.maxWidth < 940;
        final navigationWidth = isCompactNavigation ? 76.0 : _sidebarWidth;
        final content = Row(
          children: [
            LibraryNavigation(
              isCompact: isCompactNavigation,
              width: navigationWidth,
              isSettingsSelected: _destination == _LibraryDestination.settings,
              roots: state.roots,
              selectedRootId: state.query.rootId,
              selectedFolderRelativePath: state.query.folderRelativePath,
              transientRootPath: _transientRootPath(state),
              folderTree: folderTree,
              isBusy: state.isBusy,
              onSelectLibrary: () => _selectLibrary(state),
              onSelectRoot: (root) => _selectRoot(state, root),
              onSelectFolder: (root, folder) =>
                  _selectFolder(state, root, folder),
              onExpandFolder: (rootId, parentRelativePath) => _loadFolderBranch(
                folderController,
                state,
                rootId,
                parentRelativePath,
              ),
              onLoadMoreFolders: (rootId, parentRelativePath) =>
                  _loadFolderBranch(
                    folderController,
                    state,
                    rootId,
                    parentRelativePath,
                    loadMore: true,
                  ),
              onAddSource: controller.chooseDirectoryAndScan,
              onOpenSettings: _openSettings,
              onUpdateRoot: controller.scanDirectory,
              onOpenRoot: _openRoot,
              onOpenFolder: _openFolder,
              onRemoveRoot: _confirmRemoveRoot,
            ),
            if (isCompactNavigation) const SizedBox(width: 1),
            Expanded(
              child: LibraryMainSurface(
                child: _destination == _LibraryDestination.settings
                    ? AmeSettingsPage(hasLibraryRoots: state.roots.isNotEmpty)
                    : Column(
                        children: [
                          LibraryGalleryHeader(
                            galleryTitle: _galleryTitle(state),
                            totalItems: _totalItems(state),
                            selectedCount: _selection.selectedCount(
                              _totalItems(state),
                            ),
                            isSelecting: _isSelecting,
                            layoutShape: _layoutShape,
                            thumbnailSize: _thumbnailSize,
                            sortKey: state.query.sortKey,
                            sortDirection: state.query.sortDirection,
                            onBeginSelection: () =>
                                setState(() => _isSelecting = true),
                            onCancelSelection: _clearSelection,
                            onSelectAll: () => _selectAll(state),
                            onLayoutShapeChanged: _changeLayoutShape,
                            onThumbnailSizeChanged: _changeThumbnailSize,
                            onSortKeyChanged: (value) =>
                                _changeSortKey(state, value),
                            onSortDirectionChanged: (value) =>
                                _changeSortDirection(state, value),
                          ),
                          if (state.isLoadingPage ||
                              state.isLoadingPreviousPage ||
                              state.isLoadingTimeAnchor)
                            const LinearProgressIndicator(
                              key: Key("library-top-loading"),
                              minHeight: 2,
                              semanticsLabel: "正在加载图片",
                            ),
                          Expanded(
                            child: _buildGalleryBody(
                              state,
                              controller,
                              layoutManifest,
                            ),
                          ),
                        ],
                      ),
              ),
            ),
          ],
        );
        if (isCompactNavigation) {
          return content;
        }
        return Stack(
          children: [
            content,
            Positioned(
              left:
                  navigationWidth -
                  LibraryNavigationResizeHandle.hitTargetWidth / 2,
              top: 0,
              bottom: 0,
              child: LibraryNavigationResizeHandle(
                width: _sidebarWidth,
                minimumWidth: ameMinimumSidebarWidth,
                maximumWidth: ameMaximumSidebarWidth,
                defaultWidth: ameDefaultSidebarWidth,
                onWidthChanged: _changeSidebarWidth,
                onWidthChangeEnd: (width) =>
                    unawaited(_persistSidebarWidth(amePreferences, width)),
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildGalleryBody(
    LibraryState state,
    LibraryController controller,
    LibraryGalleryLayoutManifest? layoutManifest,
  ) {
    if (state.assets.isEmpty) {
      if (state.status == LibraryStatus.refreshing || state.isLoadingTimeline) {
        return const GalleryLoadingState();
      }
      if (state.roots.isNotEmpty) {
        return NoGalleryResults(query: state.query);
      }
      return EmptyLibrary(
        state: state,
        onImport: controller.chooseDirectoryAndScan,
      );
    }
    return Row(
      children: [
        Expanded(
          child: LibraryGalleryWall(
            state: state,
            controller: controller,
            scrollController: _galleryScrollController,
            layoutShape: _layoutShape,
            thumbnailSize: _thumbnailSize,
            selection: _selection,
            isSelecting: _isSelecting,
            onOpen: _openAsset,
            onToggleSelection: _toggleSelection,
            onViewInformation: _showAssetInformation,
            onCopyPath: _copyAssetPath,
            onRevealFile: _revealAsset,
            onVisiblePositionChanged: (position) {
              _visibleGalleryPosition = position;
            },
            onVisibleRangeChanged: _handleVisibleGalleryRangeChanged,
            onLoadPrevious: () =>
                _loadPreviousPagePreservingPosition(controller),
            onLayoutChanged: _handleGalleryLayoutChanged,
            layoutManifest: layoutManifest,
          ),
        ),
        ValueListenableBuilder<_LibraryGalleryLayoutSnapshot?>(
          valueListenable: _galleryLayoutSnapshot,
          builder: (context, snapshot, child) {
            final activeSnapshot = snapshot?.matches(state) ?? false
                ? snapshot
                : null;
            return LibraryTimeNavigation(
              isLoading: state.isLoadingTimeline,
              scrollController: _galleryScrollController,
              layoutMetrics: activeSnapshot?.metrics,
              timeline: state.timeline,
              layoutShape: _layoutShape,
              virtualGeometry: activeSnapshot?.virtualGeometry,
              windowStartItemOffset: state.windowStartItemOffset,
              loadedItemCount: state.assets.length,
              onSeek: (bucket, itemOffset) =>
                  _seekTimeline(controller, bucket, itemOffset),
            );
          },
        ),
      ],
    );
  }

  void _openAsset(LibraryAsset asset) {
    ref
        .read(libraryControllerProvider.notifier)
        .updateViewerPreviewDemand(asset);
    setState(() => _viewerLocationId = asset.locationId);
  }

  void _closeViewer() {
    ref
        .read(libraryControllerProvider.notifier)
        .updateViewerPreviewDemand(null);
    setState(() => _viewerLocationId = null);
  }

  Future<void> _openAdjacentAsset(int direction) async {
    final currentLocationId = _viewerLocationId;
    if (currentLocationId == null || _isNavigatingViewer) {
      return;
    }
    setState(() => _isNavigatingViewer = true);
    try {
      var state = ref.read(libraryControllerProvider);
      var currentIndex = state.assets.indexWhere(
        (asset) => asset.locationId == currentLocationId,
      );
      var targetIndex = currentIndex + direction;
      if (targetIndex < 0 && state.hasPreviousAssets) {
        await _loadPreviousAssetsForViewer(
          ref.read(libraryControllerProvider.notifier),
        );
      } else if (targetIndex >= state.assets.length && state.hasMoreAssets) {
        await ref.read(libraryControllerProvider.notifier).loadNextPage();
      }
      if (!mounted || _viewerLocationId != currentLocationId) {
        return;
      }
      state = ref.read(libraryControllerProvider);
      currentIndex = state.assets.indexWhere(
        (asset) => asset.locationId == currentLocationId,
      );
      targetIndex = currentIndex + direction;
      if (currentIndex < 0 ||
          targetIndex < 0 ||
          targetIndex >= state.assets.length) {
        return;
      }
      final target = state.assets[targetIndex];
      ref
          .read(libraryControllerProvider.notifier)
          .updateViewerPreviewDemand(target);
      setState(() => _viewerLocationId = target.locationId);
    } finally {
      if (mounted) {
        setState(() => _isNavigatingViewer = false);
      }
    }
  }

  Future<void> _loadPreviousAssetsForViewer(
    LibraryController controller,
  ) async {
    if (_galleryScrollController.hasClients) {
      await _loadPreviousPagePreservingPosition(controller);
      return;
    }
    await controller.loadPreviousPage();
  }

  void _toggleSelection(LibraryAsset asset) {
    final totalItems = _totalItems(ref.read(libraryControllerProvider));
    setState(() {
      _selection = _selection.toggle(asset.locationId);
      _isSelecting = _selection.selectedCount(totalItems) > 0;
    });
  }

  void _selectAll(LibraryState state) {
    if (_totalItems(state) == 0) {
      return;
    }
    setState(() {
      _isSelecting = true;
      _selection = _selection.selectAll();
    });
  }

  void _clearSelection() {
    setState(() {
      _selection = _selection.clear();
      _isSelecting = false;
    });
  }

  void _onSearchChanged(String value) {
    if (_destination != _LibraryDestination.gallery) {
      setState(() => _destination = _LibraryDestination.gallery);
    }
    _searchDebounce?.cancel();
    _searchDebounce = Timer(const Duration(milliseconds: 250), () {
      if (!mounted) {
        return;
      }
      final state = ref.read(libraryControllerProvider);
      _applyQuery(state.query.copyWith(searchText: value));
    });
  }

  Future<void> _applyQuery(LibraryGalleryQuery query) async {
    final didUpdate = await ref
        .read(libraryControllerProvider.notifier)
        .updateQuery(query);
    if (!mounted || !didUpdate) {
      return;
    }
    _galleryLayoutSnapshot.value = null;
    _visibleGalleryPosition = null;
    if (_galleryScrollController.hasClients) {
      _galleryScrollController.jumpTo(0);
    }
  }

  void _changeLayoutShape(GalleryLayoutShape value) {
    setState(() => _layoutShape = value);
    unawaited(_persistViewPreferences());
  }

  void _changeThumbnailSize(GalleryThumbnailSize value) {
    setState(() => _thumbnailSize = value);
    unawaited(_persistViewPreferences());
  }

  void _changeSortKey(LibraryState state, LibraryGallerySortKey value) {
    final query = state.query.copyWith(sortKey: value);
    unawaited(_applyQuery(query));
    unawaited(_persistViewPreferences(query: query));
  }

  void _changeSortDirection(
    LibraryState state,
    LibraryGallerySortDirection value,
  ) {
    final query = state.query.copyWith(sortDirection: value);
    unawaited(_applyQuery(query));
    unawaited(_persistViewPreferences(query: query));
  }

  Future<void> _persistViewPreferences({LibraryGalleryQuery? query}) async {
    final currentQuery = query ?? ref.read(libraryControllerProvider).query;
    try {
      await ref
          .read(libraryViewPreferenceStoreProvider)
          .saveLibraryViewPreferences(
            LibraryViewPreferences(
              layoutShape: _layoutShape,
              thumbnailSize: _thumbnailSize,
              sortKey: currentQuery.sortKey,
              sortDirection: currentQuery.sortDirection,
            ),
          );
    } on Object catch (error) {
      if (mounted) {
        _showMessage("无法保存图库显示设置：$error");
      }
    }
  }

  void _changeSidebarWidth(double width) {
    final nextWidth = width
        .clamp(ameMinimumSidebarWidth, ameMaximumSidebarWidth)
        .toDouble();
    if ((nextWidth - _sidebarWidth).abs() < 0.1) {
      return;
    }
    setState(() => _sidebarWidth = nextWidth);
  }

  Future<void> _persistSidebarWidth(
    AmePreferences preferences,
    double width,
  ) async {
    final previousWidth = preferences.sidebarWidth;
    try {
      await ref
          .read(amePreferencesControllerProvider.notifier)
          .update(preferences.copyWith(sidebarWidth: width));
    } on Object catch (error) {
      if (mounted) {
        setState(() => _sidebarWidth = previousWidth);
        _showMessage("无法保存侧栏宽度：$error");
      }
    }
  }

  void _cancelCurrentMode() {
    if (_viewerLocationId != null) {
      _closeViewer();
      return;
    }
    if (_isSelecting) {
      _clearSelection();
      return;
    }
    if (_destination == _LibraryDestination.settings) {
      setState(() => _destination = _LibraryDestination.gallery);
    }
  }

  Future<void> _copyAssetPath(LibraryAsset asset) async {
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .copyText(asset.displayPath);
      if (mounted) {
        _showMessage("已复制文件路径");
      }
    } on Object catch (error) {
      if (mounted) {
        _showMessage("无法复制路径：$error");
      }
    }
  }

  Future<void> _revealAsset(LibraryAsset asset) async {
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .revealFile(asset.sourcePath);
    } on Object catch (error) {
      if (mounted) {
        _showMessage("无法在文件资源管理器中打开：$error");
      }
    }
  }

  Future<void> _openRoot(LibraryRoot root) async {
    try {
      await ref.read(libraryPlatformActionsProvider).revealDirectory(root.path);
    } on Object catch (error) {
      if (mounted) {
        _showMessage("无法在文件资源管理器中打开：$error");
      }
    }
  }

  Future<void> _openFolder(LibraryRoot root, LibraryFolder folder) async {
    try {
      await ref
          .read(libraryPlatformActionsProvider)
          .revealLibraryFolder(root.path, folder.relativePath);
    } on Object catch (error) {
      if (mounted) {
        _showMessage("无法在文件资源管理器中打开：$error");
      }
    }
  }

  Future<void> _loadFolderBranch(
    LibraryFolderController controller,
    LibraryState state,
    String rootId,
    String parentRelativePath, {
    bool loadMore = false,
  }) async {
    final revision = state.catalogRevision;
    if (revision == null) {
      return;
    }
    await controller.loadBranch(
      catalogRevision: revision,
      rootId: rootId,
      parentRelativePath: parentRelativePath,
      loadMore: loadMore,
    );
  }

  Future<void> _confirmRemoveRoot(LibraryRoot root) async {
    final shouldRemove = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text("从 Ame 中移除文件夹？"),
        content: Text(
          "将停止在 Ame 中显示“${librarySourceName(root.displayPath)}”。"
          "磁盘上的文件夹和图片不会被删除或修改。",
        ),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text(LibraryStrings.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text(LibraryStrings.removeFromAme),
          ),
        ],
      ),
    );
    if (shouldRemove != true || !mounted) {
      return;
    }
    final didRemove = await ref
        .read(libraryControllerProvider.notifier)
        .unregisterRoot(root);
    if (mounted && didRemove) {
      _showMessage("已从 Ame 中移除，原图片未作任何修改");
    }
  }

  void _openSettings() {
    setState(() {
      _destination = _LibraryDestination.settings;
      _selection = _selection.clear();
      _isSelecting = false;
    });
  }

  void _selectLibrary(LibraryState state) {
    setState(() => _destination = _LibraryDestination.gallery);
    unawaited(
      _applyQuery(state.query.copyWith(rootId: null, folderRelativePath: null)),
    );
  }

  void _selectRoot(LibraryState state, LibraryRoot root) {
    setState(() => _destination = _LibraryDestination.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(rootId: root.id, folderRelativePath: null),
      ),
    );
  }

  void _selectFolder(
    LibraryState state,
    LibraryRoot root,
    LibraryFolder folder,
  ) {
    setState(() => _destination = _LibraryDestination.gallery);
    unawaited(
      _applyQuery(
        state.query.copyWith(
          rootId: root.id,
          folderRelativePath: folder.relativePath,
          includeDescendants: true,
        ),
      ),
    );
  }

  Future<void> _loadPreviousPagePreservingPosition(
    LibraryController controller,
  ) async {
    if (_isRestoringPreviousWindow || !_galleryScrollController.hasClients) {
      return;
    }
    final position = _galleryScrollController.position;
    final previousPixels = position.pixels;
    final previousSnapshot = _galleryLayoutSnapshot.value;
    final previousContentExtent = previousSnapshot?.metrics.contentExtent ?? 0;
    final previousLeadingExtent =
        previousSnapshot?.virtualGeometry.leadingExtent ?? 0;
    _isRestoringPreviousWindow = true;
    try {
      final didLoad = await controller.loadPreviousPage();
      if (!didLoad || !mounted) {
        return;
      }
      await Future<void>.delayed(Duration.zero);
      await WidgetsBinding.instance.endOfFrame;
      if (!mounted || !_galleryScrollController.hasClients) {
        return;
      }
      final nextPosition = _galleryScrollController.position;
      final nextSnapshot = _galleryLayoutSnapshot.value;
      final addedExtent =
          (nextSnapshot?.metrics.contentExtent ?? previousContentExtent) -
          previousContentExtent;
      final leadingDelta =
          (nextSnapshot?.virtualGeometry.leadingExtent ??
              previousLeadingExtent) -
          previousLeadingExtent;
      final displacement = addedExtent + leadingDelta;
      if (displacement.abs() > 0.01) {
        nextPosition.jumpTo(
          (previousPixels + displacement)
              .clamp(nextPosition.minScrollExtent, nextPosition.maxScrollExtent)
              .toDouble(),
        );
      }
    } finally {
      _isRestoringPreviousWindow = false;
    }
  }

  Future<bool> _seekTimeline(
    LibraryController controller,
    LibraryTimeBucket bucket,
    int itemOffset,
  ) async {
    final didSeek = await controller.jumpToTime(bucket, itemOffset: itemOffset);
    if (!mounted) {
      return false;
    }
    if (!didSeek) {
      final error = ref
          .read(libraryControllerProvider)
          .timeNavigationErrorMessage;
      if (error != null) {
        _showMessage("无法跳转到所选日期：$error");
      }
    }
    return didSeek;
  }

  void _handleGalleryLayoutChanged(
    LibraryGalleryLayoutMetrics nextMetrics,
    LibraryVirtualGalleryGeometry nextVirtualGeometry,
  ) {
    if (!mounted) {
      return;
    }
    final currentState = ref.read(libraryControllerProvider);
    if (currentState.windowStartItemOffset !=
            nextVirtualGeometry.windowStartItemOffset ||
        currentState.assets.length != nextVirtualGeometry.loadedItemCount ||
        currentState.queryId != nextVirtualGeometry.queryId) {
      return;
    }
    final previousSnapshot = _galleryLayoutSnapshot.value;
    final previousMetrics = previousSnapshot?.metrics;
    final previousVirtualGeometry = previousSnapshot?.virtualGeometry;
    final hasSameMetrics =
        previousMetrics?.hasSameGeometry(nextMetrics) ?? false;
    if (hasSameMetrics && nextMetrics.isQueryWide) {
      return;
    }
    if (hasSameMetrics &&
        (previousVirtualGeometry?.hasSameGeometry(nextVirtualGeometry) ??
            false)) {
      return;
    }
    _galleryLayoutSnapshot.value = _LibraryGalleryLayoutSnapshot(
      metrics: nextMetrics,
      virtualGeometry: nextVirtualGeometry,
      catalogRevision: currentState.catalogRevision,
    );
    if (nextMetrics.isQueryWide) {
      if (!(previousMetrics?.isQueryWide ?? false)) {
        _restoreFirstQueryWidePosition(nextMetrics, currentState);
      }
      return;
    }
    final anchorItemIndex = _visibleGalleryPosition?.globalItemIndex;
    final previousAnchorOffset = anchorItemIndex == null
        ? null
        : previousMetrics?.offsetForGlobalItemIndex(anchorItemIndex);
    final nextAnchorOffset = anchorItemIndex == null
        ? null
        : nextMetrics.offsetForGlobalItemIndex(anchorItemIndex);
    final previousPixels = _galleryScrollController.hasClients
        ? _galleryScrollController.position.pixels
        : 0.0;
    final previousValue = previousVirtualGeometry?.valueForScrollOffset(
      previousPixels,
    );
    if (_isRestoringPreviousWindow || !_galleryScrollController.hasClients) {
      return;
    }
    final position = _galleryScrollController.position;
    final previousAnchorPixels =
        previousAnchorOffset == null || previousVirtualGeometry == null
        ? null
        : previousAnchorOffset +
              (previousMetrics?.isQueryWide ?? false
                  ? 0
                  : previousVirtualGeometry.leadingExtent);
    final nextAnchorPixels = nextAnchorOffset == null
        ? null
        : nextAnchorOffset +
              (nextMetrics.isQueryWide ? 0 : nextVirtualGeometry.leadingExtent);
    final target = previousAnchorPixels != null && nextAnchorPixels != null
        ? previousPixels + nextAnchorPixels - previousAnchorPixels
        : previousValue == null
        ? previousPixels
        : nextVirtualGeometry.scrollOffsetForValue(previousValue);
    final boundedTarget = target
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
    if ((position.pixels - boundedTarget).abs() >= 0.5) {
      position.jumpTo(boundedTarget);
    }
  }

  void _restoreFirstQueryWidePosition(
    LibraryGalleryLayoutMetrics metrics,
    LibraryState currentState,
  ) {
    if (!_galleryScrollController.hasClients || metrics.itemCount == 0) {
      return;
    }
    final visiblePosition = _visibleGalleryPosition;
    final visibleItemIndex = visiblePosition?.globalItemIndex;
    final loadedEnd =
        currentState.windowStartItemOffset + currentState.assets.length;
    final canRestoreVisibleItem =
        visibleItemIndex != null &&
        visibleItemIndex >= currentState.windowStartItemOffset &&
        visibleItemIndex < loadedEnd;
    var targetItemIndex = currentState.windowStartItemOffset;
    if (canRestoreVisibleItem) {
      targetItemIndex = visibleItemIndex;
    }
    final itemOffset = metrics.offsetForGlobalItemIndex(targetItemIndex);
    if (itemOffset == null) {
      return;
    }
    final position = _galleryScrollController.position;
    final target = canRestoreVisibleItem && position.hasViewportDimension
        ? itemOffset + _galleryAnchorTopInset - position.viewportDimension * 0.5
        : itemOffset;
    final boundedTarget = target
        .clamp(position.minScrollExtent, position.maxScrollExtent)
        .toDouble();
    if ((position.pixels - boundedTarget).abs() >= 0.5) {
      position.jumpTo(boundedTarget);
    }
  }

  Future<void> _showAssetInformation(LibraryAsset asset) =>
      showLibraryAssetInformation(context, asset);

  void _showMessage(String message) {
    ScaffoldMessenger.of(context)
      ..hideCurrentSnackBar()
      ..showSnackBar(SnackBar(content: Text(message)));
  }

  static String _queryId(LibraryState state) {
    if (state.queryId.isNotEmpty) {
      return "${state.catalogRevision ?? BigInt.zero}:${state.queryId}";
    }
    return "${state.catalogRevision ?? BigInt.zero}:${state.query.hashCode}";
  }

  static int _totalItems(LibraryState state) {
    return state.timeline?.totalItems ??
        state.roots.fold(0, (sum, root) => sum + root.assetCount);
  }

  static String _galleryTitle(LibraryState state) {
    final folder = state.query.folderRelativePath;
    if (folder != null) {
      return folder.split("/").last;
    }
    final rootId = state.query.rootId;
    if (rootId == null) {
      return LibraryStrings.library;
    }
    for (final root in state.roots) {
      if (root.id == rootId) {
        return librarySourceName(root.displayPath);
      }
    }
    return LibraryStrings.library;
  }

  static LibraryAsset? _assetByLocation(
    List<LibraryAsset> assets,
    String locationId,
  ) {
    for (final asset in assets) {
      if (asset.locationId == locationId) {
        return asset;
      }
    }
    return null;
  }

  static String? _transientRootPath(LibraryState state) {
    final rootPath = state.displayRootPath;
    if (rootPath == null ||
        state.roots.any((root) => root.displayPath == rootPath)) {
      return null;
    }
    return rootPath;
  }

  static bool _showsTaskSurface(LibraryState state) {
    return switch (state.status) {
      LibraryStatus.empty => false,
      LibraryStatus.completed => state.scanId != null,
      _ => true,
    };
  }
}

enum _LibraryDestination { gallery, settings }

class _LibraryGalleryLayoutSnapshot {
  const _LibraryGalleryLayoutSnapshot({
    required this.metrics,
    required this.virtualGeometry,
    required this.catalogRevision,
  });

  final LibraryGalleryLayoutMetrics metrics;
  final LibraryVirtualGalleryGeometry virtualGeometry;
  final BigInt? catalogRevision;

  bool matches(LibraryState state) {
    if (virtualGeometry.queryId != state.queryId ||
        catalogRevision != state.catalogRevision) {
      return false;
    }
    if (metrics.isQueryWide) {
      return virtualGeometry.totalItemCount ==
          (state.timeline?.totalItems ?? state.assets.length);
    }
    return virtualGeometry.windowStartItemOffset ==
            state.windowStartItemOffset &&
        virtualGeometry.loadedItemCount == state.assets.length;
  }
}
