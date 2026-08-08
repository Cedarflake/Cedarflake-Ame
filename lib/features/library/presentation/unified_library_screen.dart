import "dart:async";

import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../../adapters/windows_library_platform_actions.dart";
import "../application/library_controller.dart";
import "../application/library_folder_controller.dart";
import "../domain/library_folder_models.dart";
import "../domain/library_models.dart";
import "../domain/library_state.dart";
import "gallery_selection.dart";
import "gallery_view_options.dart";
import "library_strings.dart";
import "widgets/library_gallery_header.dart";
import "widgets/library_gallery_layout.dart";
import "widgets/library_gallery_states.dart";
import "widgets/library_gallery_wall.dart";
import "widgets/library_global_bar.dart";
import "widgets/library_image_viewer.dart";
import "widgets/library_navigation.dart";
import "widgets/library_task_surface.dart";
import "widgets/library_time_navigation.dart";

class UnifiedLibraryScreen extends ConsumerStatefulWidget {
  const UnifiedLibraryScreen({super.key});

  @override
  ConsumerState<UnifiedLibraryScreen> createState() =>
      _UnifiedLibraryScreenState();
}

class _UnifiedLibraryScreenState extends ConsumerState<UnifiedLibraryScreen> {
  final ScrollController _galleryScrollController = ScrollController();
  late final TextEditingController _searchController;
  late GallerySelection _selection;
  GalleryLayoutShape _layoutShape = GalleryLayoutShape.equalHeight;
  GalleryThumbnailSize _thumbnailSize = GalleryThumbnailSize.medium;
  String? _viewerLocationId;
  bool _isSelecting = false;
  bool _isRestoringPreviousWindow = false;
  LibraryGalleryLayoutMetrics? _galleryLayoutMetrics;
  LibraryGalleryVisiblePosition? _visibleGalleryPosition;
  Timer? _searchDebounce;

  @override
  void initState() {
    super.initState();
    final state = ref.read(libraryControllerProvider);
    _searchController = TextEditingController(text: state.query.searchText);
    _selection = GallerySelection.empty(_queryId(state));
  }

  @override
  void dispose() {
    _searchDebounce?.cancel();
    _galleryScrollController.dispose();
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final state = ref.watch(libraryControllerProvider);
    final controller = ref.read(libraryControllerProvider.notifier);
    final queryId = _queryId(state);
    if (_selection.queryId != queryId) {
      _selection = GallerySelection.empty(queryId);
      _isSelecting = false;
    }
    final viewerAsset = _viewerLocationId == null
        ? null
        : _assetByLocation(state.assets, _viewerLocationId!);

    return CallbackShortcuts(
      bindings: {
        const SingleActivator(LogicalKeyboardKey.keyA, control: true): () =>
            _selectAll(state),
        const SingleActivator(LogicalKeyboardKey.escape): _cancelCurrentMode,
        const SingleActivator(LogicalKeyboardKey.keyD, control: true):
            _clearSelection,
      },
      child: Focus(
        autofocus: true,
        child: Scaffold(
          body: SafeArea(
            child: Stack(
              children: [
                Column(
                  children: [
                    LibraryGlobalBar(
                      isBusy: state.isBusy,
                      searchController: _searchController,
                      onSearchChanged: _onSearchChanged,
                      onImport: controller.chooseDirectoryAndScan,
                    ),
                    Divider(
                      height: 1,
                      color: Theme.of(context).colorScheme.outlineVariant,
                    ),
                    Expanded(
                      child: IndexedStack(
                        index: viewerAsset == null ? 0 : 1,
                        children: [
                          _buildLibrary(context, state, controller),
                          if (viewerAsset == null)
                            const SizedBox.shrink()
                          else
                            LibraryImageViewer(
                              asset: viewerAsset,
                              onBack: () =>
                                  setState(() => _viewerLocationId = null),
                              onInformation: () =>
                                  _showAssetInformation(viewerAsset),
                            ),
                        ],
                      ),
                    ),
                  ],
                ),
                if (_showsTaskSurface(state))
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
                      ),
                    ),
                  ),
              ],
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildLibrary(
    BuildContext context,
    LibraryState state,
    LibraryController controller,
  ) {
    final folderTree = ref.watch(libraryFolderControllerProvider);
    final folderController = ref.read(libraryFolderControllerProvider.notifier);
    return LayoutBuilder(
      builder: (context, constraints) {
        final isCompactNavigation = constraints.maxWidth < 940;
        return Row(
          children: [
            LibraryNavigation(
              isCompact: isCompactNavigation,
              roots: state.roots,
              selectedRootId: state.query.rootId,
              selectedFolderRelativePath: state.query.folderRelativePath,
              transientRootPath: _transientRootPath(state),
              folderTree: folderTree,
              isBusy: state.isBusy,
              onSelectLibrary: () => _applyQuery(
                state.query.copyWith(rootId: null, folderRelativePath: null),
              ),
              onSelectRoot: (root) => _applyQuery(
                state.query.copyWith(rootId: root.id, folderRelativePath: null),
              ),
              onSelectFolder: (root, folder) => _applyQuery(
                state.query.copyWith(
                  rootId: root.id,
                  folderRelativePath: folder.relativePath,
                  includeDescendants: true,
                ),
              ),
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
              onUpdateRoot: controller.scanDirectory,
              onOpenRoot: _openRoot,
              onOpenFolder: _openFolder,
              onRemoveRoot: _confirmRemoveRoot,
            ),
            VerticalDivider(
              width: 1,
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
            Expanded(
              child: Column(
                children: [
                  LibraryGalleryHeader(
                    galleryTitle: _galleryTitle(state),
                    totalItems: _totalItems(state),
                    selectedCount: _selection.selectedCount(_totalItems(state)),
                    isSelecting: _isSelecting,
                    layoutShape: _layoutShape,
                    thumbnailSize: _thumbnailSize,
                    sortKey: state.query.sortKey,
                    sortDirection: state.query.sortDirection,
                    onBeginSelection: () => setState(() => _isSelecting = true),
                    onCancelSelection: _clearSelection,
                    onViewSelected: () => _openFirstSelected(state.assets),
                    onSelectAll: () => _selectAll(state),
                    onDeselectAll: _clearSelection,
                    onLayoutShapeChanged: (value) =>
                        setState(() => _layoutShape = value),
                    onThumbnailSizeChanged: (value) =>
                        setState(() => _thumbnailSize = value),
                    onSortKeyChanged: (value) =>
                        _applyQuery(state.query.copyWith(sortKey: value)),
                    onSortDirectionChanged: (value) =>
                        _applyQuery(state.query.copyWith(sortDirection: value)),
                  ),
                  Divider(
                    height: 1,
                    color: Theme.of(context).colorScheme.outlineVariant,
                  ),
                  if (state.isLoadingTimeAnchor)
                    const LinearProgressIndicator(minHeight: 2),
                  Expanded(child: _buildGalleryBody(state, controller)),
                ],
              ),
            ),
          ],
        );
      },
    );
  }

  Widget _buildGalleryBody(LibraryState state, LibraryController controller) {
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
            onLoadPrevious: () =>
                _loadPreviousPagePreservingPosition(controller),
            onLayoutChanged: _handleGalleryLayoutChanged,
          ),
        ),
        LibraryTimeNavigation(
          isLoading: state.isLoadingTimeline,
          scrollController: _galleryScrollController,
          layoutMetrics: _galleryLayoutMetrics,
          timeline: state.timeline,
          layoutShape: _layoutShape,
          windowStartItemOffset: state.windowStartItemOffset,
          loadedItemCount: state.assets.length,
          onSeek: (bucket, itemOffset) =>
              _seekTimeline(controller, bucket, itemOffset),
        ),
      ],
    );
  }

  void _openAsset(LibraryAsset asset) {
    setState(() => _viewerLocationId = asset.locationId);
  }

  void _toggleSelection(LibraryAsset asset) {
    setState(() {
      _isSelecting = true;
      _selection = _selection.toggle(asset.locationId);
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
    setState(() {
      _galleryLayoutMetrics = null;
      _visibleGalleryPosition = null;
    });
    if (_galleryScrollController.hasClients) {
      _galleryScrollController.jumpTo(0);
    }
  }

  void _cancelCurrentMode() {
    if (_viewerLocationId != null) {
      setState(() => _viewerLocationId = null);
      return;
    }
    if (_isSelecting) {
      _clearSelection();
    }
  }

  void _openFirstSelected(List<LibraryAsset> assets) {
    for (final asset in assets) {
      if (_selection.contains(asset.locationId)) {
        _openAsset(asset);
        return;
      }
    }
  }

  Future<void> _copyAssetPath(LibraryAsset asset) async {
    try {
      await ref.read(libraryPlatformActionsProvider).copyText(asset.sourcePath);
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
      await ref.read(libraryPlatformActionsProvider).openDirectory(root.path);
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
          .openLibraryFolder(root.path, folder.relativePath);
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
          "将停止在 Ame 中显示“${librarySourceName(root.path)}”。"
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

  Future<void> _loadPreviousPagePreservingPosition(
    LibraryController controller,
  ) async {
    if (_isRestoringPreviousWindow || !_galleryScrollController.hasClients) {
      return;
    }
    final position = _galleryScrollController.position;
    final previousPixels = position.pixels;
    final previousContentExtent = _galleryLayoutMetrics?.contentExtent ?? 0;
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
      final addedExtent =
          (_galleryLayoutMetrics?.contentExtent ?? previousContentExtent) -
          previousContentExtent;
      if (addedExtent > 0) {
        nextPosition.jumpTo(
          (previousPixels + addedExtent)
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

  void _handleGalleryLayoutChanged(LibraryGalleryLayoutMetrics nextMetrics) {
    final previousMetrics = _galleryLayoutMetrics;
    if (previousMetrics?.hasSameGeometry(nextMetrics) ?? false) {
      return;
    }
    final anchorLocationId = _visibleGalleryPosition?.locationId;
    final previousAnchorOffset = previousMetrics?.offsetForLocation(
      anchorLocationId,
    );
    final nextAnchorOffset = nextMetrics.offsetForLocation(anchorLocationId);
    final previousPixels = _galleryScrollController.hasClients
        ? _galleryScrollController.position.pixels
        : 0.0;
    if (mounted) {
      setState(() => _galleryLayoutMetrics = nextMetrics);
    }
    if (_isRestoringPreviousWindow ||
        previousAnchorOffset == null ||
        nextAnchorOffset == null ||
        !_galleryScrollController.hasClients) {
      return;
    }
    final position = _galleryScrollController.position;
    final target = previousPixels + nextAnchorOffset - previousAnchorOffset;
    position.jumpTo(
      target
          .clamp(position.minScrollExtent, position.maxScrollExtent)
          .toDouble(),
    );
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
        return librarySourceName(root.path);
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
    final rootPath = state.rootPath;
    if (rootPath == null || state.roots.any((root) => root.path == rootPath)) {
      return null;
    }
    return rootPath;
  }

  static bool _showsTaskSurface(LibraryState state) {
    return state.status != LibraryStatus.empty &&
        state.status != LibraryStatus.completed;
  }
}
