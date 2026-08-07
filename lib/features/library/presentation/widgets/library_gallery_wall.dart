import "dart:async";
import "dart:io";

import "package:flutter/gestures.dart";
import "package:flutter/material.dart";
import "package:flutter/services.dart";
import "package:flutter_riverpod/flutter_riverpod.dart";

import "../../application/library_controller.dart";
import "../../domain/library_models.dart";
import "../../domain/library_state.dart";
import "../gallery_selection.dart";
import "../gallery_view_options.dart";
import "../library_strings.dart";
import "justified_gallery_layout.dart";

class LibraryGalleryVisiblePosition {
  const LibraryGalleryVisiblePosition({
    required this.monthKey,
    required this.locationId,
  });

  final String? monthKey;
  final String? locationId;
}

class LibraryGalleryWall extends StatelessWidget {
  const LibraryGalleryWall({
    required this.state,
    required this.controller,
    required this.scrollController,
    required this.layoutShape,
    required this.thumbnailSize,
    required this.selection,
    required this.isSelecting,
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    required this.onVisiblePositionChanged,
    required this.onLoadPrevious,
    required this.onContentExtentChanged,
    super.key,
  });

  final LibraryState state;
  final LibraryController controller;
  final ScrollController scrollController;
  final GalleryLayoutShape layoutShape;
  final GalleryThumbnailSize thumbnailSize;
  final GallerySelection selection;
  final bool isSelecting;
  final ValueChanged<LibraryAsset> onOpen;
  final ValueChanged<LibraryAsset> onToggleSelection;
  final ValueChanged<LibraryAsset> onViewInformation;
  final ValueChanged<LibraryAsset> onCopyPath;
  final ValueChanged<LibraryAsset> onRevealFile;
  final ValueChanged<LibraryGalleryVisiblePosition> onVisiblePositionChanged;
  final Future<void> Function() onLoadPrevious;
  final ValueChanged<double> onContentExtentChanged;

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraints) {
        const horizontalPadding = 24.0;
        final entries = _GalleryEntry.build(
          assets: state.assets,
          availableWidth: constraints.maxWidth - horizontalPadding - 16,
          layoutShape: layoutShape,
          thumbnailSize: thumbnailSize,
          sortKey: state.query.sortKey,
        );
        onContentExtentChanged(
          entries.fold<double>(90, (total, entry) => total + entry.extent),
        );
        return ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: Stack(
            children: [
              Listener(
                onPointerSignal: (event) {
                  if (event is PointerScrollEvent &&
                      event.scrollDelta.dy < 0 &&
                      scrollController.hasClients &&
                      scrollController.position.extentBefore < 900 &&
                      state.hasPreviousAssets &&
                      !state.isLoadingPage &&
                      !state.isLoadingPreviousPage &&
                      state.previousPageErrorMessage == null) {
                    unawaited(onLoadPrevious());
                  }
                },
                child: NotificationListener<ScrollNotification>(
                  onNotification: (notification) {
                    if (notification is ScrollUpdateNotification ||
                        notification is ScrollEndNotification) {
                      final position = _positionAtOffset(
                        entries,
                        notification.metrics.pixels,
                        notification.metrics.viewportDimension,
                      );
                      if (position != null) {
                        onVisiblePositionChanged(position);
                      }
                    }
                    if (notification.metrics.extentBefore < 900 &&
                        state.hasPreviousAssets &&
                        !state.isLoadingPage &&
                        !state.isLoadingPreviousPage &&
                        state.previousPageErrorMessage == null) {
                      unawaited(onLoadPrevious());
                    }
                    if (notification.metrics.extentAfter < 900 &&
                        state.hasMoreAssets &&
                        !state.isLoadingPage &&
                        !state.isLoadingPreviousPage &&
                        state.pageErrorMessage == null) {
                      unawaited(controller.loadNextPage());
                    }
                    return false;
                  },
                  child: CustomScrollView(
                    key: const Key("library-photo-wall"),
                    controller: scrollController,
                    slivers: [
                      SliverPadding(
                        padding: const EdgeInsets.fromLTRB(
                          horizontalPadding,
                          18,
                          16,
                          72,
                        ),
                        sliver: SliverList.builder(
                          itemCount: entries.length,
                          itemBuilder: (context, index) {
                            final entry = entries[index];
                            if (entry.headerLabel case final label?) {
                              return SizedBox(
                                height: entry.extent,
                                child: Align(
                                  alignment: Alignment.centerLeft,
                                  child: Semantics(
                                    header: true,
                                    child: Text(
                                      label,
                                      key: ValueKey(
                                        "gallery-date-${entry.dateKey ?? 'unknown'}",
                                      ),
                                      style: Theme.of(
                                        context,
                                      ).textTheme.titleSmall,
                                    ),
                                  ),
                                ),
                              );
                            }
                            if (entry.cells.isEmpty) {
                              return SizedBox(height: entry.extent);
                            }
                            return SizedBox(
                              height: entry.extent,
                              child: Align(
                                alignment: Alignment.topLeft,
                                child: SizedBox(
                                  height: entry.rowHeight,
                                  child: Row(
                                    children: [
                                      for (
                                        var cellIndex = 0;
                                        cellIndex < entry.cells.length;
                                        cellIndex++
                                      ) ...[
                                        if (cellIndex > 0)
                                          const SizedBox(
                                            width: _GalleryEntry.spacing,
                                          ),
                                        _LibraryPhotoTile(
                                          key: ValueKey(
                                            entry
                                                .cells[cellIndex]
                                                .asset
                                                .locationId,
                                          ),
                                          asset: entry.cells[cellIndex].asset,
                                          width: entry.cells[cellIndex].width,
                                          height: entry.rowHeight,
                                          isSelecting: isSelecting,
                                          isSelected: selection.contains(
                                            entry
                                                .cells[cellIndex]
                                                .asset
                                                .locationId,
                                          ),
                                          onOpen: onOpen,
                                          onToggleSelection: onToggleSelection,
                                          onViewInformation: onViewInformation,
                                          onCopyPath: onCopyPath,
                                          onRevealFile: onRevealFile,
                                        ),
                                      ],
                                    ],
                                  ),
                                ),
                              ),
                            );
                          },
                        ),
                      ),
                      if (state.isLoadingPage || state.pageErrorMessage != null)
                        SliverToBoxAdapter(
                          child: SizedBox(
                            height: 72,
                            child: state.isLoadingPage
                                ? const Center(
                                    child: SizedBox.square(
                                      dimension: 28,
                                      child: CircularProgressIndicator(
                                        strokeWidth: 3,
                                      ),
                                    ),
                                  )
                                : Center(
                                    child: OutlinedButton.icon(
                                      key: const Key(
                                        "library-load-more-button",
                                      ),
                                      onPressed: controller.loadNextPage,
                                      icon: const Icon(Icons.refresh),
                                      label: const Text(
                                        LibraryStrings.retryLoading,
                                      ),
                                    ),
                                  ),
                          ),
                        ),
                    ],
                  ),
                ),
              ),
              if (state.isLoadingPreviousPage ||
                  state.previousPageErrorMessage != null)
                Positioned(
                  top: 8,
                  left: 0,
                  right: 0,
                  child: Center(
                    child: Material(
                      elevation: 2,
                      color: Theme.of(context).colorScheme.surfaceContainer,
                      borderRadius: BorderRadius.circular(20),
                      child: Padding(
                        padding: const EdgeInsets.symmetric(
                          horizontal: 14,
                          vertical: 8,
                        ),
                        child: state.isLoadingPreviousPage
                            ? const SizedBox.square(
                                key: Key("library-load-previous-progress"),
                                dimension: 22,
                                child: CircularProgressIndicator(
                                  strokeWidth: 3,
                                ),
                              )
                            : TextButton.icon(
                                key: const Key("library-load-previous-button"),
                                onPressed: onLoadPrevious,
                                icon: const Icon(Icons.refresh),
                                label: const Text(LibraryStrings.retryLoading),
                              ),
                      ),
                    ),
                  ),
                ),
            ],
          ),
        );
      },
    );
  }

  static LibraryGalleryVisiblePosition? _positionAtOffset(
    List<_GalleryEntry> entries,
    double scrollOffset,
    double viewportDimension,
  ) {
    if (entries.isEmpty) {
      return null;
    }
    final target = (scrollOffset + (viewportDimension * 0.5) - 18).clamp(
      0.0,
      double.infinity,
    );
    var activeEntry = entries.first;
    var runningOffset = 0.0;
    for (final entry in entries) {
      if (runningOffset > target) {
        break;
      }
      activeEntry = entry;
      runningOffset += entry.extent;
    }
    return LibraryGalleryVisiblePosition(
      monthKey: activeEntry.monthKey,
      locationId: activeEntry.firstLocationId,
    );
  }
}

class _GalleryEntry {
  const _GalleryEntry({
    required this.extent,
    required this.monthKey,
    this.dateKey,
    this.headerLabel,
    this.rowHeight = 0,
    this.cells = const [],
    this.firstLocationId,
  });

  static const spacing = 6.0;
  static const headerExtent = 40.0;
  static const groupGap = 18.0;

  final double extent;
  final String? monthKey;
  final String? dateKey;
  final String? headerLabel;
  final double rowHeight;
  final List<_GalleryCell> cells;
  final String? firstLocationId;

  static List<_GalleryEntry> build({
    required List<LibraryAsset> assets,
    required double availableWidth,
    required GalleryLayoutShape layoutShape,
    required GalleryThumbnailSize thumbnailSize,
    required LibraryGallerySortKey sortKey,
  }) {
    if (assets.isEmpty || availableWidth <= 0) {
      return const [];
    }
    final groups = <_DateGroup>[];
    String? activeDateKey;
    var activeAssets = <LibraryAsset>[];
    var hasGroup = false;
    for (final asset in assets) {
      final dateKey = _dateKey(asset, sortKey);
      if (hasGroup && dateKey != activeDateKey) {
        groups.add(_DateGroup(activeDateKey, activeAssets));
        activeAssets = [];
      }
      activeDateKey = dateKey;
      activeAssets.add(asset);
      hasGroup = true;
    }
    if (hasGroup) {
      groups.add(_DateGroup(activeDateKey, activeAssets));
    }

    final entries = <_GalleryEntry>[];
    for (final group in groups) {
      final monthKey = group.dateKey?.substring(0, 7);
      if (sortKey != LibraryGallerySortKey.fileName) {
        entries.add(
          _GalleryEntry(
            extent: headerExtent,
            monthKey: monthKey,
            dateKey: group.dateKey,
            headerLabel: _dateLabel(group.dateKey),
            firstLocationId: group.assets.first.locationId,
          ),
        );
      }
      if (layoutShape == GalleryLayoutShape.equalHeight) {
        final rows =
            JustifiedGalleryLayout(
              targetRowHeight: thumbnailSize.targetExtent,
              spacing: spacing,
            ).compute(
              aspectRatios: [
                for (final asset in group.assets) _aspectRatio(asset),
              ],
              availableWidth: availableWidth,
            );
        for (final row in rows) {
          entries.add(
            _GalleryEntry(
              extent: row.height + spacing,
              monthKey: monthKey,
              rowHeight: row.height,
              firstLocationId:
                  group.assets[row.cells.first.itemIndex].locationId,
              cells: [
                for (final cell in row.cells)
                  _GalleryCell(
                    asset: group.assets[cell.itemIndex],
                    width: cell.width,
                  ),
              ],
            ),
          );
        }
      } else {
        final columnCount =
            ((availableWidth + spacing) /
                    (thumbnailSize.targetExtent + spacing))
                .floor()
                .clamp(1, group.assets.length)
                .toInt();
        final tileSize =
            (availableWidth - spacing * (columnCount - 1)) / columnCount;
        for (var start = 0; start < group.assets.length; start += columnCount) {
          final end = (start + columnCount).clamp(0, group.assets.length);
          entries.add(
            _GalleryEntry(
              extent: tileSize + spacing,
              monthKey: monthKey,
              rowHeight: tileSize,
              firstLocationId: group.assets[start].locationId,
              cells: [
                for (final asset in group.assets.sublist(start, end))
                  _GalleryCell(asset: asset, width: tileSize),
              ],
            ),
          );
        }
      }
      entries.add(
        _GalleryEntry(
          extent: groupGap,
          monthKey: monthKey,
          firstLocationId: group.assets.last.locationId,
        ),
      );
    }
    return entries;
  }

  static double _aspectRatio(LibraryAsset asset) {
    if (asset.width <= 0 || asset.height <= 0) {
      return 1;
    }
    return asset.width / asset.height;
  }

  static String? _dateKey(LibraryAsset asset, LibraryGallerySortKey sortKey) {
    switch (sortKey) {
      case LibraryGallerySortKey.captureTime:
        final localTime = asset.captureTime?.localTime;
        if (localTime == null || localTime.length < 10) {
          return null;
        }
        final value = localTime.substring(0, 10);
        final match = RegExp(r"^\d{4}-\d{2}-\d{2}$").firstMatch(value);
        return match == null ? null : value;
      case LibraryGallerySortKey.createdTime:
        final createdUnixMs = asset.createdUnixMs;
        return createdUnixMs == null ? null : _unixDateKey(createdUnixMs);
      case LibraryGallerySortKey.modifiedTime:
        return _unixDateKey(asset.modifiedUnixMs);
      case LibraryGallerySortKey.fileName:
        return null;
    }
  }

  static String _unixDateKey(int unixMs) {
    final date = DateTime.fromMillisecondsSinceEpoch(unixMs);
    return "${date.year.toString().padLeft(4, '0')}-"
        "${date.month.toString().padLeft(2, '0')}-"
        "${date.day.toString().padLeft(2, '0')}";
  }

  static String _dateLabel(String? dateKey) {
    if (dateKey == null) {
      return LibraryStrings.unknownCaptureDate;
    }
    final parts = dateKey.split("-").map(int.parse).toList(growable: false);
    return "${parts[0]}年${parts[1]}月${parts[2]}日";
  }
}

class _DateGroup {
  const _DateGroup(this.dateKey, this.assets);

  final String? dateKey;
  final List<LibraryAsset> assets;
}

class _GalleryCell {
  const _GalleryCell({required this.asset, required this.width});

  final LibraryAsset asset;
  final double width;
}

class _LibraryPhotoTile extends ConsumerStatefulWidget {
  const _LibraryPhotoTile({
    required this.asset,
    required this.width,
    required this.height,
    required this.isSelecting,
    required this.isSelected,
    required this.onOpen,
    required this.onToggleSelection,
    required this.onViewInformation,
    required this.onCopyPath,
    required this.onRevealFile,
    super.key,
  });

  final LibraryAsset asset;
  final double width;
  final double height;
  final bool isSelecting;
  final bool isSelected;
  final ValueChanged<LibraryAsset> onOpen;
  final ValueChanged<LibraryAsset> onToggleSelection;
  final ValueChanged<LibraryAsset> onViewInformation;
  final ValueChanged<LibraryAsset> onCopyPath;
  final ValueChanged<LibraryAsset> onRevealFile;

  @override
  ConsumerState<_LibraryPhotoTile> createState() => _LibraryPhotoTileState();
}

class _LibraryPhotoTileState extends ConsumerState<_LibraryPhotoTile> {
  final MenuController _menuController = MenuController();
  final FocusNode _focusNode = FocusNode(debugLabel: "Library photo tile");
  late final LibraryController _controller;
  bool _isHovered = false;
  bool _isFocused = false;

  @override
  void initState() {
    super.initState();
    _controller = ref.read(libraryControllerProvider.notifier);
    _schedulePreview();
  }

  @override
  void didUpdateWidget(covariant _LibraryPhotoTile oldWidget) {
    super.didUpdateWidget(oldWidget);
    if (oldWidget.asset.locationId != widget.asset.locationId ||
        oldWidget.asset.previewStatus != widget.asset.previewStatus) {
      _controller.cancelPreview(oldWidget.asset.locationId);
      _schedulePreview();
    }
  }

  @override
  void dispose() {
    _controller.cancelPreview(widget.asset.locationId);
    _focusNode.dispose();
    super.dispose();
  }

  void _schedulePreview() {
    if (widget.asset.previewStatus != LibraryPreviewStatus.pending) {
      return;
    }
    WidgetsBinding.instance.addPostFrameCallback((_) {
      if (mounted) {
        _controller.requestPreview(widget.asset);
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final isSelectionVisible =
        _isHovered || _isFocused || widget.isSelecting || widget.isSelected;
    return SizedBox(
      width: widget.width,
      height: widget.height,
      child: MenuAnchor(
        controller: _menuController,
        childFocusNode: _focusNode,
        menuChildren: [
          MenuItemButton(
            leadingIcon: const Icon(Icons.open_in_full),
            onPressed: () => widget.onOpen(widget.asset),
            child: const Text(LibraryStrings.open),
          ),
          MenuItemButton(
            leadingIcon: const Icon(Icons.info_outline),
            onPressed: () => widget.onViewInformation(widget.asset),
            child: const Text(LibraryStrings.viewInformation),
          ),
          const Divider(height: 1),
          MenuItemButton(
            leadingIcon: const Icon(Icons.content_copy_outlined),
            onPressed: () => widget.onCopyPath(widget.asset),
            child: const Text(LibraryStrings.copyPath),
          ),
          MenuItemButton(
            leadingIcon: const Icon(Icons.folder_open_outlined),
            onPressed: () => widget.onRevealFile(widget.asset),
            child: const Text(LibraryStrings.openInExplorer),
          ),
        ],
        child: CallbackShortcuts(
          bindings: {
            const SingleActivator(LogicalKeyboardKey.contextMenu):
                _openKeyboardMenu,
            const SingleActivator(LogicalKeyboardKey.f10, shift: true):
                _openKeyboardMenu,
          },
          child: Focus(
            focusNode: _focusNode,
            onFocusChange: (value) => setState(() => _isFocused = value),
            child: MouseRegion(
              onEnter: (_) => setState(() => _isHovered = true),
              onExit: (_) => setState(() => _isHovered = false),
              child: GestureDetector(
                behavior: HitTestBehavior.opaque,
                onSecondaryTapDown: (details) {
                  _focusNode.requestFocus();
                  _menuController.open(position: details.localPosition);
                },
                child: Semantics(
                  label: widget.asset.relativePath,
                  selected: widget.isSelected,
                  button: true,
                  child: Material(
                    color: colorScheme.surfaceContainerHighest,
                    shape: RoundedRectangleBorder(
                      borderRadius: BorderRadius.circular(10),
                      side: widget.isSelected
                          ? BorderSide(color: colorScheme.primary, width: 3)
                          : BorderSide.none,
                    ),
                    clipBehavior: Clip.antiAlias,
                    child: InkWell(
                      onTap: () => widget.onOpen(widget.asset),
                      child: Stack(
                        fit: StackFit.expand,
                        children: [
                          _buildPreview(context),
                          if (isSelectionVisible)
                            Positioned(
                              right: 4,
                              top: 4,
                              child: Material(
                                color: colorScheme.surface.withValues(
                                  alpha: 0.92,
                                ),
                                shape: RoundedRectangleBorder(
                                  borderRadius: BorderRadius.circular(8),
                                ),
                                child: Checkbox(
                                  key: ValueKey(
                                    "select-${widget.asset.locationId}",
                                  ),
                                  value: widget.isSelected,
                                  onChanged: (_) =>
                                      widget.onToggleSelection(widget.asset),
                                  materialTapTargetSize:
                                      MaterialTapTargetSize.shrinkWrap,
                                  visualDensity: VisualDensity.compact,
                                ),
                              ),
                            ),
                        ],
                      ),
                    ),
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }

  Widget _buildPreview(BuildContext context) {
    final asset = widget.asset;
    return switch (asset.previewStatus) {
      LibraryPreviewStatus.pending => const Center(
        key: Key("library-preview-pending"),
        child: CircularProgressIndicator(strokeWidth: 3),
      ),
      LibraryPreviewStatus.failed => Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.broken_image_outlined),
            const SizedBox(height: 4),
            TextButton(
              key: Key("preview-retry-${asset.locationId}"),
              onPressed: () => _controller.requestPreview(asset, retry: true),
              child: const Text(LibraryStrings.retryPreview),
            ),
          ],
        ),
      ),
      LibraryPreviewStatus.ready => Image.file(
        File(asset.previewPath),
        fit: BoxFit.cover,
        cacheWidth: (widget.width * MediaQuery.devicePixelRatioOf(context))
            .round()
            .clamp(64, 512),
        filterQuality: FilterQuality.low,
        errorBuilder: (context, error, stackTrace) {
          return const Center(child: Icon(Icons.broken_image_outlined));
        },
      ),
    };
  }

  void _openKeyboardMenu() {
    _menuController.open();
  }
}
