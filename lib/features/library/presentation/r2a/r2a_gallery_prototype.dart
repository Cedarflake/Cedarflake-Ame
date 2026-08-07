import "dart:async";

import "package:flutter/material.dart";

import "../widgets/annotated_time_rail.dart";
import "../widgets/justified_gallery_layout.dart";
import "r2a_fixture_data.dart";
import "r2a_models.dart";
import "r2a_settings_page.dart";
import "r2a_strings.dart";

class R2aGalleryPrototype extends StatefulWidget {
  const R2aGalleryPrototype({super.key});

  @override
  State<R2aGalleryPrototype> createState() => _R2aGalleryPrototypeState();
}

class _R2aGalleryPrototypeState extends State<R2aGalleryPrototype> {
  final ScrollController _galleryScrollController = ScrollController();
  final TextEditingController _searchController = TextEditingController();
  final Set<String> _selectedAssetIds = {};
  final Set<String> _removedSourceIds = {};
  final Set<String> _ignoredDuplicateGroups = {};
  Timer? _importTimer;
  String? _selectedSourceId;
  R2aAsset? _viewerAsset;
  R2aLayoutShape _layoutShape = R2aLayoutShape.equalHeight;
  R2aThumbnailSize _thumbnailSize = R2aThumbnailSize.medium;
  R2aDuplicateMode _duplicateMode = R2aDuplicateMode.allFiles;
  R2aSortKey _sortKey = R2aSortKey.captureDate;
  R2aSortDirection _sortDirection = R2aSortDirection.descending;
  bool _isSelecting = false;
  bool _isShowingSettings = false;
  bool _isReviewingDuplicates = false;
  bool _isShowingSubfolders = true;
  bool _isImporting = false;
  double _importProgress = 0;
  double _timelineValue = 0;

  @override
  void initState() {
    super.initState();
    _galleryScrollController.addListener(_syncTimelineFromGallery);
  }

  @override
  void dispose() {
    _importTimer?.cancel();
    _galleryScrollController
      ..removeListener(_syncTimelineFromGallery)
      ..dispose();
    _searchController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return Scaffold(
      body: SafeArea(
        child: Stack(
          children: [
            Column(
              children: [
                _GlobalBar(
                  searchController: _searchController,
                  isSearchEnabled: !_isShowingSettings,
                  onSearchChanged: (_) => setState(() {}),
                  onImport: _startImport,
                  onSettings: () => setState(() {
                    _isShowingSettings = true;
                    _viewerAsset = null;
                  }),
                ),
                Divider(height: 1, color: colorScheme.outlineVariant),
                Expanded(child: _buildContent()),
              ],
            ),
            if (_isImporting)
              Align(
                alignment: Alignment.bottomCenter,
                child: Padding(
                  padding: const EdgeInsets.only(bottom: 24),
                  child: _ImportProgressCard(
                    progress: _importProgress,
                    onCancel: _cancelImport,
                  ),
                ),
              ),
          ],
        ),
      ),
    );
  }

  Widget _buildContent() {
    if (_isShowingSettings) {
      return R2aSettingsPage(
        onBack: () => setState(() => _isShowingSettings = false),
      );
    }
    if (_viewerAsset case final asset?) {
      return _ImageViewer(asset: asset, onBack: _closeViewer);
    }
    return LayoutBuilder(
      builder: (context, constraints) {
        final isCompactNavigation = constraints.maxWidth < 940;
        return Row(
          children: [
            _LibraryNavigation(
              isCompact: isCompactNavigation,
              sources: R2aFixtureData.sources
                  .where((source) => !_removedSourceIds.contains(source.id))
                  .toList(growable: false),
              selectedSourceId: _selectedSourceId,
              onSelectSource: (sourceId) => setState(() {
                _selectedSourceId = sourceId;
                _selectedAssetIds.clear();
                _isSelecting = false;
              }),
              onAddSource: _startImport,
              onSourceAction: _handleSourceAction,
            ),
            VerticalDivider(
              width: 1,
              color: Theme.of(context).colorScheme.outlineVariant,
            ),
            Expanded(child: _buildGalleryCanvas()),
          ],
        );
      },
    );
  }

  Widget _buildGalleryCanvas() {
    final assets = _visibleAssets();
    if (_isReviewingDuplicates) {
      return _DuplicateReviewCanvas(
        groups: _duplicateGroups(),
        onExit: () => setState(() => _isReviewingDuplicates = false),
        onIgnore: (groupId) => setState(() {
          _ignoredDuplicateGroups.add(groupId);
        }),
      );
    }
    return Column(
      children: [
        _GalleryHeader(
          resultCount: assets.length,
          selectedCount: _selectedAssetIds.length,
          isSelecting: _isSelecting,
          sortKey: _sortKey,
          sortDirection: _sortDirection,
          duplicateMode: _duplicateMode,
          isShowingSubfolders: _isShowingSubfolders,
          layoutShape: _layoutShape,
          thumbnailSize: _thumbnailSize,
          onBeginSelection: () => setState(() => _isSelecting = true),
          onCancelSelection: () => setState(() {
            _selectedAssetIds.clear();
            _isSelecting = false;
          }),
          onSortKeyChanged: (value) => setState(() => _sortKey = value),
          onSortDirectionChanged: (value) =>
              setState(() => _sortDirection = value),
          onSubfoldersChanged: (value) =>
              setState(() => _isShowingSubfolders = value),
          onDuplicateModeChanged: (value) =>
              setState(() => _duplicateMode = value),
          onReviewDuplicates: () => setState(() {
            _selectedAssetIds.clear();
            _isSelecting = false;
            _isReviewingDuplicates = true;
          }),
          onLayoutShapeChanged: (value) => setState(() => _layoutShape = value),
          onThumbnailSizeChanged: (value) =>
              setState(() => _thumbnailSize = value),
          onViewSelected: _openFirstSelectedAsset,
        ),
        Divider(height: 1, color: Theme.of(context).colorScheme.outlineVariant),
        Expanded(
          child: Row(
            children: [
              Expanded(
                child: assets.isEmpty
                    ? const _EmptySearchState()
                    : _PhotoWall(
                        controller: _galleryScrollController,
                        assets: assets,
                        layoutShape: _layoutShape,
                        thumbnailSize: _thumbnailSize,
                        selectedAssetIds: _selectedAssetIds,
                        isSelecting: _isSelecting,
                        copyCountFor: _copyCountFor,
                        onAssetPressed: _handleAssetPressed,
                      ),
              ),
              if (_sortKey != R2aSortKey.name)
                AnnotatedTimeRail(
                  key: const Key("r2a-time-rail"),
                  value: _timelineValue,
                  buckets: [
                    for (final bucket in R2aFixtureData.timelineBuckets)
                      TimelineRailBucket(
                        id: bucket.id,
                        label: bucket.label,
                        contentExtent: bucket.contentExtent,
                        year: bucket.year,
                        isUnknown: bucket.isUnknown,
                      ),
                  ],
                  onChanged: _setTimelineValue,
                ),
            ],
          ),
        ),
      ],
    );
  }

  List<R2aAsset> _visibleAssets() {
    final query = _searchController.text.trim().toLowerCase();
    final selectedSource = _selectedSourceId;
    final activeSources = {
      for (final source in R2aFixtureData.sources)
        if (!_removedSourceIds.contains(source.id)) source.id: source,
    };
    final scopedSource = selectedSource != null && selectedSource != "favorites"
        ? activeSources[selectedSource]
        : null;
    final matching = R2aFixtureData.assets
        .where((asset) {
          if (selectedSource == "favorites" && !asset.isFavorite) {
            return false;
          }
          if (scopedSource != null &&
              !asset.path.startsWith(scopedSource.path)) {
            return false;
          }
          if (activeSources.values.every(
            (source) => !asset.path.startsWith(source.path),
          )) {
            return false;
          }
          if (query.isNotEmpty &&
              !asset.name.toLowerCase().contains(query) &&
              !asset.path.toLowerCase().contains(query)) {
            return false;
          }
          if (_duplicateMode == R2aDuplicateMode.duplicatesOnly &&
              asset.duplicateGroup == null) {
            return false;
          }
          return true;
        })
        .toList(growable: false);

    final displayed = <R2aAsset>[];
    final seenDuplicateGroups = <String>{};
    for (final asset in matching) {
      final group = asset.duplicateGroup;
      if (_duplicateMode == R2aDuplicateMode.mergedExact && group != null) {
        if (!seenDuplicateGroups.add(group)) {
          continue;
        }
      }
      displayed.add(asset);
    }

    if (_sortKey == R2aSortKey.name) {
      displayed.sort((left, right) => left.name.compareTo(right.name));
    }
    if (_sortDirection == R2aSortDirection.ascending) {
      return displayed.reversed.toList(growable: false);
    }
    return displayed;
  }

  List<R2aDuplicateGroup> _duplicateGroups() {
    final byGroup = <String, List<R2aAsset>>{};
    for (final asset in R2aFixtureData.assets) {
      final groupId = asset.duplicateGroup;
      if (groupId == null || _ignoredDuplicateGroups.contains(groupId)) {
        continue;
      }
      byGroup.putIfAbsent(groupId, () => []).add(asset);
    }
    return [
      for (final entry in byGroup.entries)
        R2aDuplicateGroup(
          id: entry.key,
          assets: List.unmodifiable(entry.value),
        ),
    ];
  }

  int _copyCountFor(R2aAsset asset) {
    final group = asset.duplicateGroup;
    if (group == null) {
      return 1;
    }
    return R2aFixtureData.assets
        .where((candidate) => candidate.duplicateGroup == group)
        .length;
  }

  void _handleAssetPressed(R2aAsset asset) {
    if (!_isSelecting) {
      setState(() => _viewerAsset = asset);
      return;
    }
    setState(() {
      if (!_selectedAssetIds.add(asset.id)) {
        _selectedAssetIds.remove(asset.id);
      }
    });
  }

  void _openFirstSelectedAsset() {
    if (_selectedAssetIds.isEmpty) {
      return;
    }
    final id = _selectedAssetIds.first;
    final asset = R2aFixtureData.assets.firstWhere((item) => item.id == id);
    setState(() => _viewerAsset = asset);
  }

  void _closeViewer() {
    setState(() => _viewerAsset = null);
  }

  Future<void> _handleSourceAction(String sourceId, String action) async {
    if (action != "remove") {
      return;
    }
    final shouldRemove = await showDialog<bool>(
      context: context,
      builder: (context) => AlertDialog(
        title: const Text(R2aStrings.removeSourceTitle),
        content: const Text(R2aStrings.removeSourceBody),
        actions: [
          TextButton(
            onPressed: () => Navigator.pop(context, false),
            child: const Text(R2aStrings.cancel),
          ),
          FilledButton(
            onPressed: () => Navigator.pop(context, true),
            child: const Text(R2aStrings.remove),
          ),
        ],
      ),
    );
    if (shouldRemove != true || !mounted) {
      return;
    }
    setState(() {
      _removedSourceIds.add(sourceId);
      if (_selectedSourceId == sourceId) {
        _selectedSourceId = null;
      }
    });
  }

  void _startImport() {
    _importTimer?.cancel();
    setState(() {
      _isImporting = true;
      _importProgress = 0.16;
    });
    _importTimer = Timer.periodic(const Duration(milliseconds: 320), (timer) {
      if (!mounted) {
        timer.cancel();
        return;
      }
      final next = (_importProgress + 0.11).clamp(0.0, 1.0);
      setState(() => _importProgress = next);
      if (next >= 1) {
        timer.cancel();
        Future<void>.delayed(const Duration(milliseconds: 700), () {
          if (mounted) {
            setState(() => _isImporting = false);
          }
        });
      }
    });
  }

  void _cancelImport() {
    _importTimer?.cancel();
    setState(() => _isImporting = false);
  }

  void _setTimelineValue(double value) {
    final normalized = value.clamp(0.0, 1.0).toDouble();
    if ((_timelineValue - normalized).abs() > 0.0001) {
      setState(() => _timelineValue = normalized);
    }
    _jumpGalleryToTimeline(normalized);
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted && (_timelineValue - normalized).abs() <= 0.0001) {
        _jumpGalleryToTimeline(normalized);
      }
    });
  }

  void _jumpGalleryToTimeline(double value) {
    if (!_galleryScrollController.hasClients) {
      return;
    }
    final position = _galleryScrollController.position;
    _galleryScrollController.jumpTo(value * position.maxScrollExtent);
  }

  void _syncTimelineFromGallery() {
    if (!_galleryScrollController.hasClients || !mounted) {
      return;
    }
    final position = _galleryScrollController.position;
    final nextValue = position.maxScrollExtent <= 0
        ? 0.0
        : (position.pixels / position.maxScrollExtent)
              .clamp(0.0, 1.0)
              .toDouble();
    if ((_timelineValue - nextValue).abs() <= 0.0001) {
      return;
    }
    setState(() => _timelineValue = nextValue);
  }
}

class _GlobalBar extends StatelessWidget {
  const _GlobalBar({
    required this.searchController,
    required this.isSearchEnabled,
    required this.onSearchChanged,
    required this.onImport,
    required this.onSettings,
  });

  final TextEditingController searchController;
  final bool isSearchEnabled;
  final ValueChanged<String> onSearchChanged;
  final VoidCallback onImport;
  final VoidCallback onSettings;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        final isCompact = constraints.maxWidth < 1280;
        final isNarrow = constraints.maxWidth < 900;
        return SizedBox(
          height: 64,
          child: Padding(
            padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
            child: Row(
              children: [
                SizedBox(
                  width: isNarrow ? 64 : (isCompact ? 140 : 244),
                  child: Row(
                    children: [
                      const Icon(Icons.photo_library_outlined, size: 27),
                      if (!isNarrow) ...[
                        const SizedBox(width: 12),
                        const Text(
                          R2aStrings.appName,
                          style: TextStyle(
                            fontSize: 18,
                            fontWeight: FontWeight.w600,
                          ),
                        ),
                      ],
                    ],
                  ),
                ),
                Expanded(
                  child: Center(
                    child: ConstrainedBox(
                      constraints: BoxConstraints(
                        maxWidth: isCompact ? 560 : 720,
                      ),
                      child: SearchBar(
                        key: const Key("r2a-library-search"),
                        controller: searchController,
                        enabled: isSearchEnabled,
                        hintText: R2aStrings.searchHint,
                        leading: const Icon(Icons.search),
                        trailing: [
                          if (searchController.text.isNotEmpty)
                            IconButton(
                              tooltip: "清除搜索",
                              onPressed: () {
                                searchController.clear();
                                onSearchChanged("");
                              },
                              icon: const Icon(Icons.close),
                            ),
                        ],
                        onChanged: onSearchChanged,
                      ),
                    ),
                  ),
                ),
                const SizedBox(width: 12),
                if (isCompact)
                  IconButton.filledTonal(
                    key: const Key("r2a-global-import"),
                    tooltip: R2aStrings.import,
                    onPressed: onImport,
                    icon: const Icon(Icons.add_photo_alternate_outlined),
                  )
                else
                  FilledButton.tonalIcon(
                    key: const Key("r2a-global-import"),
                    onPressed: onImport,
                    icon: const Icon(Icons.add_photo_alternate_outlined),
                    label: const Text(R2aStrings.import),
                  ),
                const SizedBox(width: 4),
                IconButton(
                  key: const Key("r2a-settings-button"),
                  tooltip: R2aStrings.settings,
                  onPressed: onSettings,
                  icon: const Icon(Icons.settings_outlined),
                ),
              ],
            ),
          ),
        );
      },
    );
  }
}

class _LibraryNavigation extends StatelessWidget {
  const _LibraryNavigation({
    required this.isCompact,
    required this.sources,
    required this.selectedSourceId,
    required this.onSelectSource,
    required this.onAddSource,
    required this.onSourceAction,
  });

  final bool isCompact;
  final List<R2aSource> sources;
  final String? selectedSourceId;
  final ValueChanged<String?> onSelectSource;
  final VoidCallback onAddSource;
  final void Function(String sourceId, String action) onSourceAction;

  @override
  Widget build(BuildContext context) {
    final width = isCompact ? 76.0 : 260.0;
    return SizedBox(
      width: width,
      child: Material(
        color: Theme.of(context).colorScheme.surfaceContainerLowest,
        child: ListView(
          padding: const EdgeInsets.fromLTRB(8, 12, 8, 20),
          children: [
            _NavigationTile(
              key: const Key("r2a-library-navigation"),
              isCompact: isCompact,
              icon: Icons.photo_library_outlined,
              label: R2aStrings.library,
              isSelected: selectedSourceId == null,
              onTap: () => onSelectSource(null),
              trailing: IconButton(
                key: const Key("r2a-sidebar-import"),
                tooltip: R2aStrings.addFolder,
                onPressed: onAddSource,
                icon: const Icon(Icons.create_new_folder_outlined),
              ),
            ),
            _NavigationTile(
              isCompact: isCompact,
              icon: Icons.favorite_border,
              label: R2aStrings.favorites,
              isSelected: selectedSourceId == "favorites",
              onTap: () => onSelectSource("favorites"),
            ),
            const Padding(
              padding: EdgeInsets.symmetric(vertical: 10),
              child: Divider(height: 1),
            ),
            for (final source in sources)
              _SourceTile(
                key: ValueKey("r2a-source-${source.id}"),
                source: source,
                isCompact: isCompact,
                isSelected: selectedSourceId == source.id,
                onTap: () => onSelectSource(source.id),
                onAction: (action) => onSourceAction(source.id, action),
              ),
          ],
        ),
      ),
    );
  }
}

class _NavigationTile extends StatelessWidget {
  const _NavigationTile({
    required this.isCompact,
    required this.icon,
    required this.label,
    required this.isSelected,
    required this.onTap,
    this.trailing,
    super.key,
  });

  final bool isCompact;
  final IconData icon;
  final String label;
  final bool isSelected;
  final VoidCallback onTap;
  final Widget? trailing;

  @override
  Widget build(BuildContext context) {
    if (isCompact) {
      return Tooltip(
        message: label,
        child: Padding(
          padding: const EdgeInsets.only(bottom: 4),
          child: IconButton(
            isSelected: isSelected,
            onPressed: onTap,
            icon: Icon(icon),
            selectedIcon: Icon(icon),
          ),
        ),
      );
    }
    return ListTile(
      dense: true,
      contentPadding: const EdgeInsets.only(left: 16, right: 4),
      minLeadingWidth: 24,
      horizontalTitleGap: 12,
      selected: isSelected,
      selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
      shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
      leading: SizedBox(width: 24, child: Icon(icon)),
      title: Text(label),
      trailing: SizedBox(width: 48, child: trailing),
      onTap: onTap,
    );
  }
}

class _SourceTile extends StatelessWidget {
  const _SourceTile({
    required this.source,
    required this.isCompact,
    required this.isSelected,
    required this.onTap,
    required this.onAction,
    super.key,
  });

  final R2aSource source;
  final bool isCompact;
  final bool isSelected;
  final VoidCallback onTap;
  final ValueChanged<String> onAction;

  @override
  Widget build(BuildContext context) {
    final icon = source.isAvailable
        ? Icons.folder_outlined
        : Icons.folder_off_outlined;
    if (isCompact) {
      return Tooltip(
        message: source.isAvailable
            ? source.path
            : "${source.path} · ${R2aStrings.sourceUnavailable}",
        child: IconButton(
          isSelected: isSelected,
          onPressed: onTap,
          icon: Icon(icon),
        ),
      );
    }
    return Padding(
      padding: const EdgeInsets.only(bottom: 2),
      child: ListTile(
        dense: true,
        contentPadding: const EdgeInsets.only(left: 16, right: 4),
        minLeadingWidth: 24,
        horizontalTitleGap: 12,
        selected: isSelected,
        selectedTileColor: Theme.of(context).colorScheme.secondaryContainer,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        leading: SizedBox(width: 24, child: Icon(icon)),
        title: Text(source.label, maxLines: 1, overflow: TextOverflow.ellipsis),
        subtitle: source.isAvailable
            ? null
            : const Text(
                R2aStrings.sourceUnavailable,
                maxLines: 1,
                overflow: TextOverflow.ellipsis,
              ),
        trailing: SizedBox(
          width: 48,
          child: PopupMenuButton<String>(
            tooltip: R2aStrings.more,
            onSelected: onAction,
            itemBuilder: (context) => const [
              PopupMenuItem(
                value: "update",
                child: Text(R2aStrings.updateLibrary),
              ),
              PopupMenuItem(
                value: "explorer",
                child: Text(R2aStrings.openInExplorer),
              ),
              PopupMenuDivider(),
              PopupMenuItem(
                value: "remove",
                child: Text(R2aStrings.removeFromAme),
              ),
            ],
          ),
        ),
        onTap: onTap,
      ),
    );
  }
}

class _GalleryHeader extends StatelessWidget {
  const _GalleryHeader({
    required this.resultCount,
    required this.selectedCount,
    required this.isSelecting,
    required this.sortKey,
    required this.sortDirection,
    required this.duplicateMode,
    required this.isShowingSubfolders,
    required this.layoutShape,
    required this.thumbnailSize,
    required this.onBeginSelection,
    required this.onCancelSelection,
    required this.onSortKeyChanged,
    required this.onSortDirectionChanged,
    required this.onSubfoldersChanged,
    required this.onDuplicateModeChanged,
    required this.onReviewDuplicates,
    required this.onLayoutShapeChanged,
    required this.onThumbnailSizeChanged,
    required this.onViewSelected,
  });

  final int resultCount;
  final int selectedCount;
  final bool isSelecting;
  final R2aSortKey sortKey;
  final R2aSortDirection sortDirection;
  final R2aDuplicateMode duplicateMode;
  final bool isShowingSubfolders;
  final R2aLayoutShape layoutShape;
  final R2aThumbnailSize thumbnailSize;
  final VoidCallback onBeginSelection;
  final VoidCallback onCancelSelection;
  final ValueChanged<R2aSortKey> onSortKeyChanged;
  final ValueChanged<R2aSortDirection> onSortDirectionChanged;
  final ValueChanged<bool> onSubfoldersChanged;
  final ValueChanged<R2aDuplicateMode> onDuplicateModeChanged;
  final VoidCallback onReviewDuplicates;
  final ValueChanged<R2aLayoutShape> onLayoutShapeChanged;
  final ValueChanged<R2aThumbnailSize> onThumbnailSizeChanged;
  final VoidCallback onViewSelected;

  @override
  Widget build(BuildContext context) {
    return ConstrainedBox(
      key: const Key("r2a-gallery-header"),
      constraints: const BoxConstraints(minHeight: 104),
      child: Padding(
        padding: const EdgeInsets.fromLTRB(28, 18, 20, 16),
        child: LayoutBuilder(
          builder: (context, constraints) {
            final title = isSelecting
                ? "已选择 $selectedCount 个项目"
                : R2aStrings.library;
            final subtitle = isSelecting
                ? R2aStrings.library
                : "$resultCount 张图片";
            final titleBlock = Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              mainAxisSize: MainAxisSize.min,
              children: [
                Text(title, style: Theme.of(context).textTheme.headlineSmall),
                const SizedBox(height: 4),
                Text(
                  subtitle,
                  key: const Key("r2a-library-summary"),
                  style: Theme.of(context).textTheme.bodyMedium?.copyWith(
                    color: Theme.of(context).colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            );
            final toolbar = isSelecting
                ? _SelectionToolbar(
                    hasSelection: selectedCount > 0,
                    onView: onViewSelected,
                    includeCancel: constraints.maxWidth < 760,
                    onCancel: onCancelSelection,
                  )
                : _BrowsingToolbar(
                    sortKey: sortKey,
                    sortDirection: sortDirection,
                    duplicateMode: duplicateMode,
                    isShowingSubfolders: isShowingSubfolders,
                    layoutShape: layoutShape,
                    thumbnailSize: thumbnailSize,
                    onBeginSelection: onBeginSelection,
                    onSortKeyChanged: onSortKeyChanged,
                    onSortDirectionChanged: onSortDirectionChanged,
                    onSubfoldersChanged: onSubfoldersChanged,
                    onDuplicateModeChanged: onDuplicateModeChanged,
                    onReviewDuplicates: onReviewDuplicates,
                    onLayoutShapeChanged: onLayoutShapeChanged,
                    onThumbnailSizeChanged: onThumbnailSizeChanged,
                  );
            if (constraints.maxWidth < 760) {
              return Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  titleBlock,
                  const SizedBox(height: 12),
                  SingleChildScrollView(
                    scrollDirection: Axis.horizontal,
                    child: toolbar,
                  ),
                ],
              );
            }
            if (isSelecting) {
              return Row(
                children: [
                  titleBlock,
                  const SizedBox(width: 24),
                  Expanded(
                    child: _RightAlignedToolbarViewport(
                      child: Row(
                        mainAxisSize: MainAxisSize.min,
                        children: [
                          _CancelSelectionButton(onPressed: onCancelSelection),
                          toolbar,
                        ],
                      ),
                    ),
                  ),
                ],
              );
            }
            return Row(
              children: [
                titleBlock,
                const SizedBox(width: 24),
                Expanded(child: _RightAlignedToolbarViewport(child: toolbar)),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _RightAlignedToolbarViewport extends StatelessWidget {
  const _RightAlignedToolbarViewport({required this.child});

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        return SingleChildScrollView(
          scrollDirection: Axis.horizontal,
          reverse: true,
          child: ConstrainedBox(
            constraints: BoxConstraints(minWidth: constraints.maxWidth),
            child: Align(alignment: Alignment.centerRight, child: child),
          ),
        );
      },
    );
  }
}

class _BrowsingToolbar extends StatelessWidget {
  const _BrowsingToolbar({
    required this.sortKey,
    required this.sortDirection,
    required this.duplicateMode,
    required this.isShowingSubfolders,
    required this.layoutShape,
    required this.thumbnailSize,
    required this.onBeginSelection,
    required this.onSortKeyChanged,
    required this.onSortDirectionChanged,
    required this.onSubfoldersChanged,
    required this.onDuplicateModeChanged,
    required this.onReviewDuplicates,
    required this.onLayoutShapeChanged,
    required this.onThumbnailSizeChanged,
  });

  final R2aSortKey sortKey;
  final R2aSortDirection sortDirection;
  final R2aDuplicateMode duplicateMode;
  final bool isShowingSubfolders;
  final R2aLayoutShape layoutShape;
  final R2aThumbnailSize thumbnailSize;
  final VoidCallback onBeginSelection;
  final ValueChanged<R2aSortKey> onSortKeyChanged;
  final ValueChanged<R2aSortDirection> onSortDirectionChanged;
  final ValueChanged<bool> onSubfoldersChanged;
  final ValueChanged<R2aDuplicateMode> onDuplicateModeChanged;
  final VoidCallback onReviewDuplicates;
  final ValueChanged<R2aLayoutShape> onLayoutShapeChanged;
  final ValueChanged<R2aThumbnailSize> onThumbnailSizeChanged;

  @override
  Widget build(BuildContext context) {
    return Row(
      key: const Key("r2a-browsing-toolbar"),
      mainAxisSize: MainAxisSize.min,
      children: [
        TextButton.icon(
          key: const Key("r2a-select-button"),
          onPressed: onBeginSelection,
          icon: const Icon(Icons.check_box_outlined),
          label: const Text(R2aStrings.select),
        ),
        _SortMenu(
          sortKey: sortKey,
          direction: sortDirection,
          onSortKeyChanged: onSortKeyChanged,
          onDirectionChanged: onSortDirectionChanged,
        ),
        _FilterMenu(
          duplicateMode: duplicateMode,
          isShowingSubfolders: isShowingSubfolders,
          onSubfoldersChanged: onSubfoldersChanged,
          onDuplicateModeChanged: onDuplicateModeChanged,
          onReviewDuplicates: onReviewDuplicates,
        ),
        _LayoutMenu(
          shape: layoutShape,
          size: thumbnailSize,
          onShapeChanged: onLayoutShapeChanged,
          onSizeChanged: onThumbnailSizeChanged,
        ),
        IconButton(
          tooltip: R2aStrings.more,
          onPressed: () {},
          icon: const Icon(Icons.more_horiz),
        ),
      ],
    );
  }
}

class _SelectionToolbar extends StatelessWidget {
  const _SelectionToolbar({
    required this.hasSelection,
    required this.onView,
    required this.includeCancel,
    required this.onCancel,
  });

  final bool hasSelection;
  final VoidCallback onView;
  final bool includeCancel;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    return Row(
      key: const Key("r2a-selection-toolbar"),
      mainAxisSize: MainAxisSize.min,
      children: [
        if (includeCancel) _CancelSelectionButton(onPressed: onCancel),
        TextButton.icon(
          onPressed: hasSelection ? onView : null,
          icon: const Icon(Icons.open_in_full),
          label: const Text(R2aStrings.view),
        ),
        TextButton.icon(
          onPressed: hasSelection ? () {} : null,
          icon: const Icon(Icons.favorite_border),
          label: const Text(R2aStrings.favorite),
        ),
        TextButton.icon(
          onPressed: hasSelection ? () {} : null,
          icon: const Icon(Icons.photo_album_outlined),
          label: const Text(R2aStrings.addToAlbum),
        ),
        TextButton.icon(
          onPressed: hasSelection ? () {} : null,
          icon: const Icon(Icons.compare_outlined),
          label: const Text(R2aStrings.compare),
        ),
        TextButton.icon(
          onPressed: hasSelection ? () {} : null,
          icon: const Icon(Icons.copy_all_outlined),
          label: const Text(R2aStrings.duplicateInfo),
        ),
        IconButton(
          tooltip: R2aStrings.more,
          onPressed: hasSelection ? () {} : null,
          icon: const Icon(Icons.more_horiz),
        ),
      ],
    );
  }
}

class _CancelSelectionButton extends StatelessWidget {
  const _CancelSelectionButton({required this.onPressed});

  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    return TextButton.icon(
      key: const Key("r2a-cancel-selection"),
      onPressed: onPressed,
      icon: const Icon(Icons.close),
      label: const Text(R2aStrings.cancel),
    );
  }
}

class _SortMenu extends StatelessWidget {
  const _SortMenu({
    required this.sortKey,
    required this.direction,
    required this.onSortKeyChanged,
    required this.onDirectionChanged,
  });

  final R2aSortKey sortKey;
  final R2aSortDirection direction;
  final ValueChanged<R2aSortKey> onSortKeyChanged;
  final ValueChanged<R2aSortDirection> onDirectionChanged;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<String>(
      key: const Key("r2a-sort-menu"),
      tooltip: R2aStrings.sort,
      onSelected: (value) {
        switch (value) {
          case "capture":
            onSortKeyChanged(R2aSortKey.captureDate);
          case "created":
            onSortKeyChanged(R2aSortKey.createdDate);
          case "modified":
            onSortKeyChanged(R2aSortKey.modifiedDate);
          case "name":
            onSortKeyChanged(R2aSortKey.name);
          case "ascending":
            onDirectionChanged(R2aSortDirection.ascending);
          case "descending":
            onDirectionChanged(R2aSortDirection.descending);
        }
      },
      itemBuilder: (context) => [
        _checkedMenuItem(
          value: "capture",
          label: R2aStrings.captureDate,
          checked: sortKey == R2aSortKey.captureDate,
          icon: Icons.calendar_month_outlined,
        ),
        _checkedMenuItem(
          value: "created",
          label: R2aStrings.createdDate,
          checked: sortKey == R2aSortKey.createdDate,
          icon: Icons.create_new_folder_outlined,
        ),
        _checkedMenuItem(
          value: "modified",
          label: R2aStrings.modifiedDate,
          checked: sortKey == R2aSortKey.modifiedDate,
          icon: Icons.edit_calendar_outlined,
        ),
        _checkedMenuItem(
          value: "name",
          label: R2aStrings.name,
          checked: sortKey == R2aSortKey.name,
          icon: Icons.text_fields,
        ),
        const PopupMenuDivider(),
        _checkedMenuItem(
          value: "ascending",
          label: R2aStrings.ascending,
          checked: direction == R2aSortDirection.ascending,
          icon: Icons.arrow_upward,
        ),
        _checkedMenuItem(
          value: "descending",
          label: R2aStrings.descending,
          checked: direction == R2aSortDirection.descending,
          icon: Icons.arrow_downward,
        ),
      ],
      child: const _MenuButtonContent(
        icon: Icons.swap_vert,
        label: R2aStrings.sort,
      ),
    );
  }
}

class _FilterMenu extends StatelessWidget {
  const _FilterMenu({
    required this.duplicateMode,
    required this.isShowingSubfolders,
    required this.onSubfoldersChanged,
    required this.onDuplicateModeChanged,
    required this.onReviewDuplicates,
  });

  final R2aDuplicateMode duplicateMode;
  final bool isShowingSubfolders;
  final ValueChanged<bool> onSubfoldersChanged;
  final ValueChanged<R2aDuplicateMode> onDuplicateModeChanged;
  final VoidCallback onReviewDuplicates;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<String>(
      key: const Key("r2a-filter-menu"),
      tooltip: R2aStrings.filter,
      onSelected: (value) {
        switch (value) {
          case "subfolders":
            onSubfoldersChanged(true);
          case "current-folder":
            onSubfoldersChanged(false);
          case "all-files":
            onDuplicateModeChanged(R2aDuplicateMode.allFiles);
          case "merged":
            onDuplicateModeChanged(R2aDuplicateMode.mergedExact);
          case "duplicates":
            onDuplicateModeChanged(R2aDuplicateMode.duplicatesOnly);
          case "review":
            onReviewDuplicates();
        }
      },
      itemBuilder: (context) => [
        _checkedMenuItem(
          value: "subfolders",
          label: R2aStrings.showSubfolders,
          checked: isShowingSubfolders,
          icon: Icons.folder_copy_outlined,
        ),
        _checkedMenuItem(
          value: "current-folder",
          label: R2aStrings.hideSubfolders,
          checked: !isShowingSubfolders,
          icon: Icons.folder_outlined,
        ),
        const PopupMenuDivider(),
        _checkedMenuItem(
          value: "all-files",
          label: R2aStrings.showAllFiles,
          checked: duplicateMode == R2aDuplicateMode.allFiles,
          icon: Icons.photo_library_outlined,
        ),
        _checkedMenuItem(
          value: "merged",
          label: R2aStrings.mergeExactCopies,
          checked: duplicateMode == R2aDuplicateMode.mergedExact,
          icon: Icons.copy_all_outlined,
        ),
        _checkedMenuItem(
          value: "duplicates",
          label: R2aStrings.showOnlyDuplicates,
          checked: duplicateMode == R2aDuplicateMode.duplicatesOnly,
          icon: Icons.filter_none_outlined,
        ),
        const PopupMenuDivider(),
        const PopupMenuItem(
          value: "review",
          child: ListTile(
            leading: Icon(Icons.fact_check_outlined),
            title: Text(R2aStrings.reviewDuplicateGroups),
            contentPadding: EdgeInsets.zero,
          ),
        ),
      ],
      child: const _MenuButtonContent(
        icon: Icons.filter_alt_outlined,
        label: R2aStrings.filter,
      ),
    );
  }
}

class _LayoutMenu extends StatelessWidget {
  const _LayoutMenu({
    required this.shape,
    required this.size,
    required this.onShapeChanged,
    required this.onSizeChanged,
  });

  final R2aLayoutShape shape;
  final R2aThumbnailSize size;
  final ValueChanged<R2aLayoutShape> onShapeChanged;
  final ValueChanged<R2aThumbnailSize> onSizeChanged;

  @override
  Widget build(BuildContext context) {
    return PopupMenuButton<String>(
      key: const Key("r2a-layout-menu"),
      tooltip: R2aStrings.layout,
      onSelected: (value) {
        switch (value) {
          case "equal-height":
            onShapeChanged(R2aLayoutShape.equalHeight);
          case "square":
            onShapeChanged(R2aLayoutShape.square);
          case "small":
            onSizeChanged(R2aThumbnailSize.small);
          case "medium":
            onSizeChanged(R2aThumbnailSize.medium);
          case "large":
            onSizeChanged(R2aThumbnailSize.large);
        }
      },
      itemBuilder: (context) => [
        _checkedMenuItem(
          value: "equal-height",
          label: R2aStrings.equalHeight,
          checked: shape == R2aLayoutShape.equalHeight,
          icon: Icons.view_quilt_outlined,
        ),
        _checkedMenuItem(
          value: "square",
          label: R2aStrings.square,
          checked: shape == R2aLayoutShape.square,
          icon: Icons.grid_view_outlined,
        ),
        const PopupMenuDivider(),
        _checkedMenuItem(
          value: "small",
          label: R2aStrings.small,
          checked: size == R2aThumbnailSize.small,
          icon: Icons.grid_4x4_outlined,
        ),
        _checkedMenuItem(
          value: "medium",
          label: R2aStrings.medium,
          checked: size == R2aThumbnailSize.medium,
          icon: Icons.grid_view_outlined,
        ),
        _checkedMenuItem(
          value: "large",
          label: R2aStrings.large,
          checked: size == R2aThumbnailSize.large,
          icon: Icons.crop_square_outlined,
        ),
      ],
      child: const _MenuButtonContent(
        icon: Icons.grid_view_outlined,
        label: R2aStrings.layout,
      ),
    );
  }
}

CheckedPopupMenuItem<String> _checkedMenuItem({
  required String value,
  required String label,
  required bool checked,
  required IconData icon,
}) {
  return CheckedPopupMenuItem<String>(
    value: value,
    checked: checked,
    child: Row(
      children: [Icon(icon, size: 20), const SizedBox(width: 12), Text(label)],
    ),
  );
}

class _MenuButtonContent extends StatelessWidget {
  const _MenuButtonContent({required this.icon, required this.label});

  final IconData icon;
  final String label;

  @override
  Widget build(BuildContext context) {
    return Semantics(
      label: label,
      button: true,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 9, vertical: 10),
        child: Row(
          mainAxisSize: MainAxisSize.min,
          children: [
            Icon(icon, size: 22),
            const Icon(Icons.arrow_drop_down, size: 18),
          ],
        ),
      ),
    );
  }
}

class _PhotoWall extends StatelessWidget {
  const _PhotoWall({
    required this.controller,
    required this.assets,
    required this.layoutShape,
    required this.thumbnailSize,
    required this.selectedAssetIds,
    required this.isSelecting,
    required this.copyCountFor,
    required this.onAssetPressed,
  });

  final ScrollController controller;
  final List<R2aAsset> assets;
  final R2aLayoutShape layoutShape;
  final R2aThumbnailSize thumbnailSize;
  final Set<String> selectedAssetIds;
  final bool isSelecting;
  final int Function(R2aAsset asset) copyCountFor;
  final ValueChanged<R2aAsset> onAssetPressed;

  @override
  Widget build(BuildContext context) {
    final groups = <String, List<R2aAsset>>{};
    for (final asset in assets) {
      groups.putIfAbsent(asset.dateLabel, () => []).add(asset);
    }
    return ScrollConfiguration(
      behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
      child: ListView.builder(
        key: const Key("r2a-photo-wall"),
        controller: controller,
        padding: const EdgeInsets.fromLTRB(24, 20, 16, 80),
        itemCount: groups.length,
        itemBuilder: (context, index) {
          final entry = groups.entries.elementAt(index);
          return Padding(
            padding: const EdgeInsets.only(bottom: 24),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: [
                Padding(
                  padding: const EdgeInsets.only(left: 2, bottom: 10),
                  child: Text(
                    entry.key,
                    style: Theme.of(context).textTheme.titleSmall,
                  ),
                ),
                LayoutBuilder(
                  builder: (context, constraints) {
                    final height = switch (thumbnailSize) {
                      R2aThumbnailSize.small => 96.0,
                      R2aThumbnailSize.medium => 138.0,
                      R2aThumbnailSize.large => 190.0,
                    };
                    return layoutShape == R2aLayoutShape.equalHeight
                        ? _buildJustifiedRows(
                            groupIndex: index,
                            assets: entry.value,
                            availableWidth: constraints.maxWidth,
                            targetHeight: height,
                          )
                        : _buildSquareRows(
                            groupIndex: index,
                            assets: entry.value,
                            availableWidth: constraints.maxWidth,
                            targetSize: height,
                          );
                  },
                ),
              ],
            ),
          );
        },
      ),
    );
  }

  Widget _buildJustifiedRows({
    required int groupIndex,
    required List<R2aAsset> assets,
    required double availableWidth,
    required double targetHeight,
  }) {
    const spacing = 6.0;
    final rows =
        JustifiedGalleryLayout(
          targetRowHeight: targetHeight,
          spacing: spacing,
        ).compute(
          aspectRatios: [for (final asset in assets) asset.aspectRatio],
          availableWidth: availableWidth,
        );
    return Column(
      children: [
        for (var rowIndex = 0; rowIndex < rows.length; rowIndex++) ...[
          SizedBox(
            key: ValueKey("r2a-justified-row-$groupIndex-$rowIndex"),
            width: double.infinity,
            height: rows[rowIndex].height,
            child: Row(
              children: [
                for (
                  var cellIndex = 0;
                  cellIndex < rows[rowIndex].cells.length;
                  cellIndex++
                ) ...[
                  if (cellIndex > 0) const SizedBox(width: spacing),
                  _buildPhotoTile(
                    asset: assets[rows[rowIndex].cells[cellIndex].itemIndex],
                    width: rows[rowIndex].cells[cellIndex].width,
                    height: rows[rowIndex].height,
                  ),
                ],
              ],
            ),
          ),
          if (rowIndex < rows.length - 1) const SizedBox(height: spacing),
        ],
      ],
    );
  }

  Widget _buildSquareRows({
    required int groupIndex,
    required List<R2aAsset> assets,
    required double availableWidth,
    required double targetSize,
  }) {
    const spacing = 6.0;
    final columnCount = ((availableWidth + spacing) / (targetSize + spacing))
        .floor()
        .clamp(1, assets.length)
        .toInt();
    final tileSize =
        (availableWidth - spacing * (columnCount - 1)) / columnCount;
    final rowCount = (assets.length / columnCount).ceil();
    return Column(
      children: [
        for (var rowIndex = 0; rowIndex < rowCount; rowIndex++) ...[
          SizedBox(
            key: ValueKey("r2a-square-row-$groupIndex-$rowIndex"),
            width: double.infinity,
            height: tileSize,
            child: Row(
              children: [
                for (
                  var index = rowIndex * columnCount;
                  index <
                      ((rowIndex + 1) * columnCount)
                          .clamp(0, assets.length)
                          .toInt();
                  index++
                ) ...[
                  if (index > rowIndex * columnCount)
                    const SizedBox(width: spacing),
                  _buildPhotoTile(
                    asset: assets[index],
                    width: tileSize,
                    height: tileSize,
                  ),
                ],
              ],
            ),
          ),
          if (rowIndex < rowCount - 1) const SizedBox(height: spacing),
        ],
      ],
    );
  }

  Widget _buildPhotoTile({
    required R2aAsset asset,
    required double width,
    required double height,
  }) {
    return _PhotoTile(
      asset: asset,
      width: width,
      height: height,
      isSelecting: isSelecting,
      isSelected: selectedAssetIds.contains(asset.id),
      copyCount: copyCountFor(asset),
      onPressed: () => onAssetPressed(asset),
    );
  }
}

class _PhotoTile extends StatelessWidget {
  const _PhotoTile({
    required this.asset,
    required this.width,
    required this.height,
    required this.isSelecting,
    required this.isSelected,
    required this.copyCount,
    required this.onPressed,
  });

  final R2aAsset asset;
  final double width;
  final double height;
  final bool isSelecting;
  final bool isSelected;
  final int copyCount;
  final VoidCallback onPressed;

  @override
  Widget build(BuildContext context) {
    final color = Color(asset.colorValue);
    final colorScheme = Theme.of(context).colorScheme;
    return Semantics(
      label: "${asset.name}，${asset.dateLabel}",
      selected: isSelected,
      button: true,
      child: SizedBox(
        key: ValueKey("r2a-${asset.id}"),
        width: width,
        height: height,
        child: Material(
          color: color,
          shape: RoundedRectangleBorder(
            borderRadius: BorderRadius.circular(10),
            side: isSelected
                ? BorderSide(color: colorScheme.primary, width: 3)
                : BorderSide.none,
          ),
          clipBehavior: Clip.antiAlias,
          child: InkWell(
            onTap: onPressed,
            child: Stack(
              fit: StackFit.expand,
              children: [
                DecoratedBox(
                  decoration: BoxDecoration(
                    gradient: LinearGradient(
                      begin: Alignment.topLeft,
                      end: Alignment.bottomRight,
                      colors: [
                        Color.lerp(color, Colors.white, 0.28) ?? color,
                        color,
                        Color.lerp(color, Colors.black, 0.16) ?? color,
                      ],
                    ),
                  ),
                ),
                Center(
                  child: Icon(
                    asset.icon,
                    size: height * 0.34,
                    color: Colors.white.withValues(alpha: 0.76),
                  ),
                ),
                if (asset.path.startsWith("G:"))
                  const Positioned(
                    left: 8,
                    top: 8,
                    child: _TileBadge(icon: Icons.cloud_outlined),
                  ),
                if (asset.isFavorite)
                  const Positioned(
                    left: 8,
                    bottom: 8,
                    child: _TileBadge(icon: Icons.favorite),
                  ),
                if (isSelecting || isSelected)
                  Positioned(
                    right: 8,
                    top: 8,
                    child: AnimatedContainer(
                      duration: const Duration(milliseconds: 140),
                      width: 26,
                      height: 26,
                      decoration: BoxDecoration(
                        color: isSelected ? colorScheme.primary : Colors.white,
                        borderRadius: BorderRadius.circular(7),
                        border: Border.all(
                          color: isSelected
                              ? colorScheme.primary
                              : colorScheme.outline,
                        ),
                      ),
                      child: isSelected
                          ? Icon(
                              Icons.check,
                              size: 18,
                              color: colorScheme.onPrimary,
                            )
                          : null,
                    ),
                  ),
                if (copyCount > 1)
                  Positioned(
                    right: 8,
                    bottom: 8,
                    child: Container(
                      padding: const EdgeInsets.symmetric(
                        horizontal: 8,
                        vertical: 4,
                      ),
                      decoration: BoxDecoration(
                        color: colorScheme.inverseSurface.withValues(
                          alpha: 0.84,
                        ),
                        borderRadius: BorderRadius.circular(10),
                      ),
                      child: Text(
                        "$copyCount ${R2aStrings.copyCount}",
                        style: Theme.of(context).textTheme.labelSmall?.copyWith(
                          color: colorScheme.onInverseSurface,
                        ),
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
}

class _TileBadge extends StatelessWidget {
  const _TileBadge({required this.icon});

  final IconData icon;

  @override
  Widget build(BuildContext context) {
    return Container(
      width: 28,
      height: 28,
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface.withValues(alpha: 0.88),
        borderRadius: BorderRadius.circular(8),
      ),
      child: Icon(icon, size: 17),
    );
  }
}

class _DuplicateReviewCanvas extends StatelessWidget {
  const _DuplicateReviewCanvas({
    required this.groups,
    required this.onExit,
    required this.onIgnore,
  });

  final List<R2aDuplicateGroup> groups;
  final VoidCallback onExit;
  final ValueChanged<String> onIgnore;

  @override
  Widget build(BuildContext context) {
    return Column(
      key: const Key("r2a-duplicate-review"),
      children: [
        Padding(
          padding: const EdgeInsets.fromLTRB(28, 22, 24, 18),
          child: Row(
            children: [
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text(
                      "${R2aStrings.duplicateReview} · ${groups.length} 组",
                      style: Theme.of(context).textTheme.headlineSmall,
                    ),
                    const SizedBox(height: 4),
                    Text(
                      "比较文件位置并记录希望保留的副本，原图片不会被修改",
                      style: Theme.of(context).textTheme.bodyMedium,
                    ),
                  ],
                ),
              ),
              OutlinedButton.icon(
                onPressed: onExit,
                icon: const Icon(Icons.close),
                label: const Text(R2aStrings.exitReview),
              ),
            ],
          ),
        ),
        const Divider(height: 1),
        Expanded(
          child: groups.isEmpty
              ? const Center(child: Text("没有待审查的重复组"))
              : ListView.separated(
                  padding: const EdgeInsets.all(24),
                  itemCount: groups.length,
                  separatorBuilder: (_, _) => const SizedBox(height: 18),
                  itemBuilder: (context, index) {
                    final group = groups[index];
                    return Card(
                      elevation: 0,
                      color: Theme.of(context).colorScheme.surfaceContainerLow,
                      child: Padding(
                        padding: const EdgeInsets.all(18),
                        child: Column(
                          crossAxisAlignment: CrossAxisAlignment.stretch,
                          children: [
                            Text(
                              "重复组 ${index + 1} · ${group.assets.length} 个文件",
                              style: Theme.of(context).textTheme.titleMedium,
                            ),
                            const SizedBox(height: 14),
                            Wrap(
                              spacing: 12,
                              runSpacing: 12,
                              children: [
                                for (final asset in group.assets)
                                  _DuplicateCopyCard(asset: asset),
                              ],
                            ),
                            const SizedBox(height: 14),
                            Wrap(
                              alignment: WrapAlignment.end,
                              spacing: 8,
                              children: [
                                TextButton(
                                  onPressed: () => onIgnore(group.id),
                                  child: const Text(R2aStrings.ignoreGroup),
                                ),
                                OutlinedButton(
                                  onPressed: () {},
                                  child: const Text(
                                    R2aStrings.readOnlySuggestion,
                                  ),
                                ),
                              ],
                            ),
                          ],
                        ),
                      ),
                    );
                  },
                ),
        ),
      ],
    );
  }
}

class _DuplicateCopyCard extends StatelessWidget {
  const _DuplicateCopyCard({required this.asset});

  final R2aAsset asset;

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      width: 230,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          _FixtureImage(asset: asset, height: 126),
          const SizedBox(height: 8),
          Text(asset.path, maxLines: 2, overflow: TextOverflow.ellipsis),
          const SizedBox(height: 4),
          OutlinedButton.icon(
            onPressed: () {},
            icon: const Icon(Icons.bookmark_border, size: 18),
            label: const Text(R2aStrings.keepThisCopy),
          ),
        ],
      ),
    );
  }
}

class _ImageViewer extends StatelessWidget {
  const _ImageViewer({required this.asset, required this.onBack});

  final R2aAsset asset;
  final VoidCallback onBack;

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    return ColoredBox(
      key: const Key("r2a-image-viewer"),
      color: colorScheme.surfaceContainerHighest,
      child: Column(
        children: [
          SizedBox(
            height: 64,
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 12),
              child: Row(
                children: [
                  IconButton(
                    tooltip: R2aStrings.backToLibrary,
                    onPressed: onBack,
                    icon: const Icon(Icons.arrow_back),
                  ),
                  const SizedBox(width: 8),
                  Expanded(
                    child: Text(
                      asset.name,
                      maxLines: 1,
                      overflow: TextOverflow.ellipsis,
                      style: Theme.of(context).textTheme.titleMedium,
                    ),
                  ),
                  IconButton(
                    tooltip: R2aStrings.favorite,
                    onPressed: () {},
                    icon: const Icon(Icons.favorite_border),
                  ),
                  IconButton(
                    tooltip: "查看信息",
                    onPressed: () {},
                    icon: const Icon(Icons.info_outline),
                  ),
                  IconButton(
                    tooltip: R2aStrings.more,
                    onPressed: () {},
                    icon: const Icon(Icons.more_horiz),
                  ),
                ],
              ),
            ),
          ),
          const Divider(height: 1),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.all(32),
              child: Center(
                child: ConstrainedBox(
                  constraints: const BoxConstraints(
                    maxWidth: 980,
                    maxHeight: 700,
                  ),
                  child: AspectRatio(
                    aspectRatio: asset.aspectRatio,
                    child: _FixtureImage(asset: asset),
                  ),
                ),
              ),
            ),
          ),
          Padding(
            padding: const EdgeInsets.fromLTRB(24, 8, 24, 20),
            child: Text(
              asset.path,
              maxLines: 1,
              overflow: TextOverflow.ellipsis,
              style: Theme.of(context).textTheme.bodySmall,
            ),
          ),
        ],
      ),
    );
  }
}

class _FixtureImage extends StatelessWidget {
  const _FixtureImage({required this.asset, this.height});

  final R2aAsset asset;
  final double? height;

  @override
  Widget build(BuildContext context) {
    final color = Color(asset.colorValue);
    return Container(
      height: height,
      decoration: BoxDecoration(
        borderRadius: BorderRadius.circular(12),
        gradient: LinearGradient(
          begin: Alignment.topLeft,
          end: Alignment.bottomRight,
          colors: [
            Color.lerp(color, Colors.white, 0.32) ?? color,
            color,
            Color.lerp(color, Colors.black, 0.18) ?? color,
          ],
        ),
      ),
      child: Center(
        child: Icon(
          asset.icon,
          size: height == null ? 132 : 52,
          color: Colors.white.withValues(alpha: 0.78),
        ),
      ),
    );
  }
}

class _EmptySearchState extends StatelessWidget {
  const _EmptySearchState();

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          const Icon(Icons.image_search_outlined, size: 48),
          const SizedBox(height: 16),
          Text(
            R2aStrings.noSearchResults,
            style: Theme.of(context).textTheme.titleLarge,
          ),
          const SizedBox(height: 6),
          const Text(R2aStrings.noSearchResultsHint),
        ],
      ),
    );
  }
}

class _ImportProgressCard extends StatelessWidget {
  const _ImportProgressCard({required this.progress, required this.onCancel});

  final double progress;
  final VoidCallback onCancel;

  @override
  Widget build(BuildContext context) {
    return Card(
      key: const Key("r2a-import-progress"),
      elevation: 6,
      child: SizedBox(
        width: 520,
        child: Padding(
          padding: const EdgeInsets.fromLTRB(20, 16, 14, 14),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Row(
                children: [
                  const Icon(Icons.info_outline, size: 20),
                  const SizedBox(width: 10),
                  const Expanded(child: Text(R2aStrings.importingFolder)),
                  TextButton(
                    key: const Key("r2a-cancel-import"),
                    onPressed: onCancel,
                    child: const Text(R2aStrings.cancel),
                  ),
                ],
              ),
              const SizedBox(height: 4),
              const Text(R2aStrings.importProgress),
              const SizedBox(height: 10),
              LinearProgressIndicator(value: progress),
            ],
          ),
        ),
      ),
    );
  }
}
